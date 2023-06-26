use anyhow::Result;
use chatgpt::types::{ChatMessage, Role};
use chrono::{Duration, Utc};
use serenity::{
    all::{
        Command, CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateInteractionResponseMessage,
        Message, MessageId,
    },
    builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, EditInteractionResponse},
    client::Context,
    futures::StreamExt,
};
use std::cmp::min;

use crate::{commands::send_ephemeral_message, util::ChatGPTTypeMapKey};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("tldr")
            .description("Posts a tl;dr of the last specified amount of time")
            .dm_permission(false)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "hours",
                    "How many hours to look back into the channels history. Defaults to 2.",
                )
                .required(false),
            ),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let gpt = ctx.data.read().await.get::<ChatGPTTypeMapKey>().unwrap().clone();

    let duration = Duration::hours(match cmd.data.options.first().map(|id| &id.value) {
        Some(CommandDataOptionValue::Integer(hours)) => *hours,
        _ => 2,
    });

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
    let earliest = Utc::now() - duration; // We don't want messages older than this

    // First try cache
    for message in
        ctx.cache.channel_messages(cmd.channel_id).as_ref().map(|c| c.values().collect::<Vec<_>>()).unwrap_or_default()
    {
        if oldest.created_at() > message.timestamp {
            oldest = message.id;
        }

        if message.timestamp < earliest.into() {
            continue;
        }

        messages.push(message.clone());
    }

    // Then, if we haven't found the oldest yet, ask Discord
    if oldest.created_at() > earliest.into() {
        let mut msg_iter = cmd.channel_id.messages_iter(&ctx).boxed();
        while let Some(message) = msg_iter.next().await {
            let message = message?;

            if message.timestamp < earliest.into() {
                break;
            }

            if !messages.iter().any(|m| m.id == message.id) {
                messages.push(message);
            }
        }
    }

    // Then decide how much context we want
    let context = min(5 + (messages.len() / 100), 10);

    // Sort it by timestamp, so it all makes sense
    messages.sort_by_key(|m| m.timestamp);
    for message in messages.into_iter() {
        // Ignore messages from the bot
        if message.author.bot {
            continue;
        }
        // Ignore messages we can't process
        if message.content.is_empty() {
            continue;
        }

        conversation.history.push(message_to_gpt_message(&ctx, message).await?);
    }

    // Send it all off, prompting ChatGPT to write a summary.
    let response = conversation
        .send_message(format!("Please summarize the discussed subjects in at most {context} bullet points."))
        .await?;

    // Edit in the response from the bot
    cmd.edit_response(ctx, EditInteractionResponse::new().content(response.message().content.clone())).await?;
    Ok(())
}

async fn message_to_gpt_message(ctx: &Context, msg: Message) -> Result<ChatMessage> {
    let member = msg.member(&ctx).await.ok();

    Ok(ChatMessage {
        role: Role::System,
        content: format!(
            "{author} says: \"{message}\"",
            author = member.as_ref().map_or_else(
                || msg.author.global_name.as_ref().map_or_else(|| msg.author.name.as_str(), |nick| nick.as_str()),
                |m| m.display_name()
            ),
            message = msg.content_safe(ctx)
        ),
    })
}
