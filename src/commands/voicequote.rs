use anyhow::{anyhow, Result};
use serenity::{
    client::Context,
    model::{
        application::{
            command::{Command, CommandOptionType},
            interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
        },
        channel::ChannelType,
    },
};

use crate::{commands::send_ephemeral_message, ingest::voice};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_application_command(ctx, |command| {
        command
            .name("voicequote")
            .description("Submits a quote said by someone in voice")
            .create_option(|option| {
                option
                    .name("user")
                    .description("The user that made the quote")
                    .kind(CommandOptionType::User)
                    .required(true)
            })
            .create_option(|option| {
                option
                    .name("channel")
                    .description("The channel the quote was said in")
                    .kind(CommandOptionType::Channel)
                    .required(true)
            })
            .create_option(|option| {
                option.name("message").description("The actual quote").kind(CommandOptionType::String).required(true)
            })
    })
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: &Context, cmd: ApplicationCommandInteraction) -> Result<()> {
    let Some(guild_id) = cmd.guild_id else {return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await};

    let mut args = cmd.data.options.iter().map(|v| &v.resolved).filter_map(|v| v.as_ref());
    let Some(CommandDataOptionValue::User(user, _)) = args.next() else { return Err(anyhow!("Could not parse user for first value")) };
    let Some(CommandDataOptionValue::Channel(channel)) = args.next() else { return Err(anyhow!("Could not parse channel for second value")) };
    let Some(CommandDataOptionValue::String(content)) = args.next() else { return Err(anyhow!("Could not parse string for third value")) };

    if channel.kind != ChannelType::Voice {
        return send_ephemeral_message(ctx, cmd, "That channel is not a voice channel!").await;
    }

    let member = guild_id.member(ctx, user.id).await?;
    voice::handle(ctx, member, channel.id, content.to_owned(), cmd).await
}
