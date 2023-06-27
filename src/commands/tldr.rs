use std::cmp::min;

use anyhow::Result;
use chatgpt::types::{ChatMessage, Role};
use chrono::{Duration, Utc};
use serenity::{
    all::{Command, CommandInteraction, CreateInteractionResponseMessage, Member, Message, MessageId, User},
    builder::{CreateCommand, CreateInteractionResponse, EditInteractionResponse},
    client::Context,
    futures::StreamExt,
};
use tiktoken_rs::cl100k_base;

use crate::{commands::send_ephemeral_message, util::ChatGPTTypeMapKey};

const TLDR_MESSAGE_HISTORY: usize = 300;

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
    let gpt = ctx.data.read().await.get::<ChatGPTTypeMapKey>().unwrap().clone();

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
    let mut oldest = MessageId::from(cmd.id.0); // Grab the interaction ID, so we have a very new ID to compare against
    let earliest = Utc::now() - Duration::hours(8); // We don't want messages older than this

    // First try cache
    {
        let cache_ref = ctx.cache.channel_messages(cmd.channel_id);
        let cache_messages = cache_ref.as_ref().map(|c| c.values().collect::<Vec<_>>()).unwrap_or_default();

        for message in cache_messages.into_iter().rev().take(TLDR_MESSAGE_HISTORY) {
            if oldest.created_at() > message.timestamp {
                oldest = message.id;
            }

            if message.timestamp < earliest.into() {
                continue;
            }

            // Ignore messages from the bot
            if message.author.bot {
                continue;
            }
            // Ignore messages we can't process
            if message.content.is_empty() {
                continue;
            }

            messages.push(message.clone());
        }
    }

    // Then, if we haven't found the oldest yet, ask Discord
    if oldest.created_at() > earliest.into() && messages.len() < TLDR_MESSAGE_HISTORY {
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

            if !messages.iter().any(|m| m.id == message.id) {
                messages.push(message);
            }

            if messages.len() == 300 {
                break;
            }
        }
    }

    if messages.len() < 50 {
        cmd.edit_response(
            ctx,
            EditInteractionResponse::new().content(
                "Look, We're talking about at most 50 messages in the last 8 hours, surely you can just scroll up.",
            ),
        )
        .await?;
        return Ok(());
    }

    // Then decide how much context we want
    let context = min(5 + (messages.len() / 100), 10);

    // Sort it by timestamp, so it all makes sense
    messages.sort_by_key(|m| m.timestamp);

    // Check the token length, so we don't exceed ChatGPTs cap
    let tokenizer = cl100k_base()?;
    let mut history = "".to_string();
    for message in messages.into_iter().rev() {
        // Convert it into a GPT message
        let gpt_message = message_to_gpt_message(&ctx, message).await?;
        history += &gpt_message.content;

        // Count the tokens, if we exceed 4000 we stop accepting more messages
        if tokenizer.encode_with_special_tokens(&history).len() > 4000 {
            break;
        }

        conversation.history.push(gpt_message);
    }
    conversation.history.reverse();

    // Send it all off, prompting ChatGPT to write a summary.
    let response = conversation
        .send_message(format!("Please summarize the discussed subjects in at most {context} bullet points."))
        .await?;

    // Edit in the response from the bot
    cmd.edit_response(ctx, EditInteractionResponse::new().content(response.message().content.clone())).await?;
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
