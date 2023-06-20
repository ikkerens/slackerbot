use anyhow::{anyhow, Result};
use serenity::{
    all::{Channel, Command, CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    client::Context,
    model::channel::ChannelType,
};

use crate::{commands::send_ephemeral_message, ingest::voice};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("voicequote")
            .description("Submits a quote said by someone in voice")
            .dm_permission(false)
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "The user that made the quote")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Channel, "channel", "The channel the quote was said in")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "message", "The actual quote").required(true),
            ),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let Some(guild_id) = cmd.guild_id else {return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await};

    let mut args = cmd.data.options.iter().map(|v| &v.value);
    let Some(CommandDataOptionValue::User(user)) = args.next() else { return Err(anyhow!("Could not parse user for first value")) };
    let Some(CommandDataOptionValue::Channel(channel_id)) = args.next() else { return Err(anyhow!("Could not parse channel for second value")) };
    let Some(CommandDataOptionValue::String(content)) = args.next() else { return Err(anyhow!("Could not parse string for third value")) };

    let Channel::Guild(channel) = channel_id.to_channel(&ctx).await? else { return Err(anyhow!("The channel is not a guild channel.")) };

    if channel.kind != ChannelType::Voice {
        return send_ephemeral_message(ctx, cmd, "That channel is not a voice channel!").await;
    }

    let member = guild_id.member(&ctx, user).await?;
    voice::handle(ctx, member, *channel_id, content.to_owned(), cmd).await
}
