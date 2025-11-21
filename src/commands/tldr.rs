use anyhow::Result;
use chatgpt::types::{ChatMessage, Role};
use chrono::{Duration, Utc};
use serenity::{
    all::{Command, CommandInteraction, CreateInteractionResponseMessage, Member, Message, User},
    builder::{CreateCommand, CreateInteractionResponse, EditInteractionResponse},
    client::Context,
    futures::StreamExt,
};
use std::{cmp::min, slice::from_ref};
use tiktoken_rs::CoreBPE;

use crate::{
    commands::{edit_interaction, send_ephemeral_message},
    util::{TLDRTypeMapKey, TLDRUsageStatus},
};

const GPT_MAX_TOKENS: u32 = 9500;
const GPT_API_TPM: u32 = 10000;
// For some reason this needs to be set at API initialization
pub(crate) const GPT_MAX_RESPONSE: u32 = 2048;

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
    let (gpt, bpe, throttle) = ctx.data.read().await.get::<TLDRTypeMapKey>().unwrap().clone();

    // If we have a blacklist active, block the command. If we don't, set it to running.
    {
        let mut throttle_guard = throttle.lock().await;
        match *throttle_guard {
            TLDRUsageStatus::Running(instant) => {
                if instant >= Utc::now() {
                    return send_ephemeral_message(
                        ctx,
                        cmd,
                        "Please wait a little, I'm already thinking about a TLDR somewhere else.",
                    )
                    .await;
                }
            }
            TLDRUsageStatus::Done(instant) => {
                if instant >= Utc::now() {
                    return send_ephemeral_message(
                        ctx,
                        cmd,
                        "Please wait a little, this command is being used too fast.",
                    )
                    .await;
                }
            }
            _ => {}
        }

        *throttle_guard = TLDRUsageStatus::Running(Utc::now() + Duration::minutes(2));
        drop(throttle_guard);
    }

    // Tell the user the bot is thinking, as ChatGPT API is not super fast.
    cmd.create_response(&ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;

    let Some(channel) = cmd.channel_id.to_channel(&ctx).await?.guild() else {
        return send_ephemeral_message(ctx, cmd, "That is not a valid channel.").await;
    };

    // Start a conversation and direct ChatGPT with an initial prompt
    let mut conversation =
        gpt.new_conversation_directed(format!("You are Slackerbot, a multi-purpose Discord bot that has been tasked with summarizing the recent topics of a text chat channel. The channels name is \"{}\"{}. The history that follows is the chat history of this channel. The current time is {}. Feel free to use markdown formatting in your response.",
                                              channel.name,
                                              channel.topic.map(|t| format!(" with the assigned topic \"{}\"", t)).unwrap_or_else(|| "".to_string()),
                                              cmd.data.id.created_at()
        ));

    let mut messages = Vec::new(); // A place to store all the history to send to ChatGPT
    let earliest = Utc::now() - Duration::hours(16); // We don't want messages older than this

    // Get a history of messages
    let mut msg_iter = cmd.channel_id.messages_iter(&ctx).boxed();
    while let Some(message) = msg_iter.next().await {
        let mut message = message?;

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
        // Make sure the guild is set
        if message.guild_id.is_none() {
            message.guild_id = Some(channel.guild_id);
        }

        messages.push(message);

        if messages.len() >= 500 {
            break;
        }
    }
    drop(msg_iter);

    if messages.len() < 50 {
        *throttle.lock().await = TLDRUsageStatus::Done(Utc::now());
        return edit_interaction(
            ctx,
            cmd,
            "Look, We're talking about at most 50 messages in the last 24 hours, surely you can just scroll up.",
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
    // Current model supports 128k tokens, and we reserve 4096 for the output prompt.
    // However, we reduce this to 100k just to add a good amount of margin of error in case our token calculation differs from GPTs
    let mut remaining = GPT_MAX_TOKENS - GPT_MAX_RESPONSE - num_tokens_from_messages(&bpe, &history)?;

    for message in messages.into_iter().rev() {
        // Convert it into a GPT message
        let gpt_message = message_to_gpt_message(&ctx, message).await?;
        let cost = num_tokens_from_messages(&bpe, from_ref(&gpt_message))?;

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

    // Calculate how long we need to block usage from the API, concerning GPT
    let tokens_used = GPT_MAX_TOKENS - remaining;
    let blacklist_in_minutes = (tokens_used / GPT_API_TPM) + 1;
    *throttle.lock().await = TLDRUsageStatus::Done(Utc::now() + Duration::minutes(blacklist_in_minutes as i64));

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
            "At {time}, {author} says{context}: \"{message}\"",
            time = msg.timestamp,
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

fn num_tokens_from_messages(bpe: &CoreBPE, messages: &[ChatMessage]) -> Result<u32> {
    let mut num_tokens: u32 = 0;
    for message in messages {
        num_tokens += 4; // every message follows <im_start>{role/name}\n{content}<im_end>\n;
        num_tokens += bpe.encode_with_special_tokens("system").len() as u32;
        num_tokens += bpe.encode_with_special_tokens(&message.content).len() as u32;
    }
    num_tokens += 3; // every reply is primed with <|start|>assistant<|message|>
    Ok(num_tokens)
}
