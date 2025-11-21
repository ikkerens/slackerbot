use std::{collections::VecDeque, sync::OnceLock};

use anyhow::{anyhow, Result};
use rand::{rng, seq::IteratorRandom};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serenity::{
    all::{Command, CommandInteraction},
    builder::CreateCommand,
    client::Context,
};
use tokio::sync::Mutex;

use entity::{prelude::Quote, quote};

use crate::{commands::send_ephemeral_message, quote::post_quote, util::DatabaseTypeMapKey};

static RANDOM_BLACKLIST: OnceLock<Mutex<VecDeque<i64>>> = OnceLock::new();

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("rquote").description("Posts a random quote").dm_permission(false),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let Some(guild_id) = cmd.guild_id else {
        return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await;
    };

    // First we find a collection of IDs we can choose from
    let ids: Vec<i64> = Quote::find()
        .select_only()
        .column(quote::Column::Id)
        .filter(quote::Column::ServerId.eq(guild_id.get()))
        .into_tuple()
        .all(&db)
        .await?;

    // Then we get the blacklist, to avoid quote repeats
    let mut blacklist = RANDOM_BLACKLIST.get_or_init(Default::default).lock().await;

    // Then we filter our id list and choose a random quote
    let Some(chosen_random) = ids.iter().filter(|v| !blacklist.contains(*v)).choose(&mut rng()) else {
        drop(blacklist); // Drop our blacklist reference early
        return send_ephemeral_message(ctx, cmd, "Could not find any random quotes, do none exist?").await;
    };

    // Update our blacklist
    blacklist.push_back(*chosen_random);
    if blacklist.len() as f32 > (ids.len() as f32 / 10f32).floor() {
        blacklist.pop_front();
    }

    // Drop our lock on the blacklist
    drop(blacklist);

    // And fetch the quote that belongs to that
    let quote = Quote::find_by_id(*chosen_random).one(&db).await?;

    match quote {
        Some(quote) => post_quote(&ctx, quote, cmd.channel_id, Some(cmd)).await,
        None => Err(anyhow!("Selected random quote that ended up not existing")),
    }
}
