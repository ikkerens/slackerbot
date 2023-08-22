use std::cmp::min;

use anyhow::Result;
use chatgpt::types::{ChatMessage, Role};
use chrono::{Duration, Utc};
use serenity::{
    all::{Command, CommandInteraction, CreateInteractionResponseMessage, Member, Message, User},
    builder::{CreateCommand, CreateInteractionResponse, EditInteractionResponse},
    client::Context,
    futures::StreamExt,
};
use tiktoken_rs::CoreBPE;

use crate::{
    commands::{edit_interaction, send_ephemeral_message},
    util::ChatGPTTypeMapKey,
};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("tldr")
            .description("Posts a tl;dr of the last specified amount of time")
            .dm_permission(false),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let (gpt, bpe) = ctx.data.read().await.get::<ChatGPTTypeMapKey>().unwrap().clone();

    // Tell the user the bot is thinking, as ChatGPT API is not super fast.
    cmd.create_response(&ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;

    let Some(channel) = cmd.channel_id.to_channel(&ctx).await?.guild() else { return send_ephemeral_message(ctx, cmd, "That is not a valid channel.").await; };

    // Start a conversation and direct ChatGPT with an initial prompt
    let mut conversation =
        gpt.new_conversation_directed(format!("You are Slackerbot, a multi-purpose Discord bot that has been tasked with summarizing the recent topics of a text chat channel. The channels name is \"{}\"{}. The history that follows is the chat history of this channel.",
                                              channel.name,
                                              channel.topic.map(|t| format!(" with the assigned topic \"{}\"", t)).unwrap_or_else(|| "".to_string())
        ));

    let mut messages = Vec::new(); // A place to store all the history to send to ChatGPT
    let earliest = Utc::now() - Duration::hours(8); // We don't want messages older than this

    // Get a history of messages
    let mut msg_iter = cmd.channel_id.messages_iter(&ctx).boxed();
    while let Some(message) = msg_iter.next().await {
        let message = message?;

        if message.timestamp < earliest.into() {
            break;
        }

        // Ignore messages from the bot
        if message.author.bot {
            continue;
        }
        // Ignore messages we can't process
        if message.content.is_empty() {
            continue;
        }

        messages.push(message);

        if messages.len() >= 300 {
            break;
        }
    }
    drop(msg_iter);

    if messages.len() < 50 {
        return edit_interaction(
            ctx,
            cmd,
            "Look, We're talking about at most 50 messages in the last 8 hours, surely you can just scroll up.",
        )
        .await;
    }

    // Then decide how much context we want
    let context = min(5 + (messages.len() / 100), 10);
    let prompt = format!(
        "Please summarize the discussed subjects using at most {context} bullet points, use usernames where reasonable."
    );

    // Sort it by timestamp, so it all makes sense
    messages.sort_by_key(|m| m.timestamp);

    // Prepare a list of token checks
    let mut history = Vec::with_capacity(messages.len());

    // Add our directive and prompt
    history.push(conversation.history.first().unwrap().clone());
    history.push(ChatMessage { role: Role::System, content: prompt.clone() });

    // Calculate their cost, and determine a remaining amount
    let mut remaining = 8192 - num_tokens_from_messages(&bpe, &history)?;

    for message in messages.into_iter().rev() {
        // Convert it into a GPT message
        let gpt_message = message_to_gpt_message(&ctx, message).await?;
        let cost = num_tokens_from_messages(&bpe, &[gpt_message.clone()])?;

        // Count the tokens, if we exceed 4096 total we stop accepting more messages
        if cost >= remaining {
            break;
        }

        remaining -= cost;
        conversation.history.push(gpt_message);
    }
    conversation.history.reverse();

    // Send it all off, prompting ChatGPT to write a summary.
    let response = conversation.send_message(prompt).await?;
    cmd.edit_response(&ctx, EditInteractionResponse::new().content(response.message().content.to_owned())).await?;
    Ok(())
}

async fn message_to_gpt_message(ctx: &Context, msg: Message) -> Result<ChatMessage> {
    let context = if let Some(reference) = msg.referenced_message.as_ref() {
        format!(", in reply to {}", resolve_name(&reference.author, reference.member(ctx).await.ok().as_ref()))
    } else {
        "".to_string()
    };

    Ok(ChatMessage {
        role: Role::System,
        content: format!(
            "{author} says{context}: \"{message}\"",
            author = resolve_name(&msg.author, msg.member(&ctx).await.ok().as_ref()),
            message = msg.content_safe(ctx)
        ),
    })
}

fn resolve_name<'a>(user: &'a User, member: Option<&'a Member>) -> &'a str {
    member.map_or_else(
        || user.global_name.as_deref().map_or_else(|| user.name.as_str(), |nick| nick),
        |m| m.display_name(),
    )
}

fn num_tokens_from_messages(bpe: &CoreBPE, messages: &[ChatMessage]) -> Result<usize> {
    let mut num_tokens: i32 = 0;
    for message in messages {
        num_tokens += 4; // every message follows <im_start>{role/name}\n{content}<im_end>\n;
        num_tokens += bpe.encode_with_special_tokens("system").len() as i32;
        num_tokens += bpe.encode_with_special_tokens(&message.content).len() as i32;
    }
    num_tokens += 3; // every reply is primed with <|start|>assistant<|message|>
    Ok(num_tokens as usize)
}
