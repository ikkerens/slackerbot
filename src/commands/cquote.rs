use anyhow::{anyhow, Result};
use rand::{seq::SliceRandom, thread_rng};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serenity::{
    client::Context,
    model::application::{
        command::{Command, CommandOptionType},
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
    },
};

use entity::{prelude::Quote, quote};

use crate::{commands::send_ephemeral_message, quote::post_quote, util::DatabaseTypeMapKey};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_application_command(ctx, |command| {
        command
            .name("cquote")
            .description("Posts a random command posted in this channel (or a specific one if provided)")
            .dm_permission(false)
            .create_option(|option| {
                option
                    .name("channel")
                    .description("The channel to get a random quote from (will default to current channel)")
                    .kind(CommandOptionType::Channel)
                    .required(false)
            })
    })
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: ApplicationCommandInteraction) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let Some(guild_id) = cmd.guild_id else {return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await};
    let channel = match cmd.data.options.first().and_then(|id| id.resolved.clone()) {
        Some(CommandDataOptionValue::Channel(channel)) => channel.id,
        None => cmd.channel_id,
        _ => return send_ephemeral_message(ctx, cmd, "No channel received").await,
    };

    let ids: Vec<i64> = Quote::find()
        .select_only()
        .column(quote::Column::Id)
        .filter(quote::Column::ServerId.eq(guild_id.0))
        .filter(quote::Column::ChannelId.eq(channel.0))
        .into_tuple()
        .all(&db)
        .await?;
    let Some(chosen_random) = ids.choose(&mut thread_rng()) else {
        return send_ephemeral_message(ctx, cmd, "Could not find any random quotes for that channel, do none exist?").await
    };

    let quote = Quote::find_by_id(*chosen_random).one(&db).await?;

    match quote {
        Some(quote) => post_quote(&ctx, quote, cmd.channel_id, Some(cmd)).await,
        None => Err(anyhow!("Selected random channel quote that ended up not existing")),
    }
}
