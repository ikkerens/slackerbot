use anyhow::{anyhow, Result};
use rand::{rng, seq::IndexedRandom};
use sea_orm::{
    sea_query::{Expr, ExprTrait, Func},
    ColumnTrait, EntityTrait, QueryFilter, QuerySelect,
};
use serenity::{
    all::{Command, CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    client::Context,
};

use entity::{prelude::Quote, quote};

use crate::{commands::send_ephemeral_message, quote::post_quote, util::DatabaseTypeMapKey};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("kwquote")
            .description("Posts a random quote containing a specific keyword")
            .dm_permission(false)
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "keyword", "The keyword to search for.")
                    .required(true),
            ),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let Some(guild_id) = cmd.guild_id else {
        return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await;
    };
    let keyword = match cmd.data.options.first().map(|id| &id.value) {
        Some(CommandDataOptionValue::String(keyword)) => keyword,
        _ => return send_ephemeral_message(ctx, cmd, "No keyword received").await,
    };

    let ids: Vec<i64> = Quote::find()
        .select_only()
        .column(quote::Column::Id)
        .filter(quote::Column::ServerId.eq(guild_id.get()))
        .filter(
            Func::lower(Expr::col((quote::Entity, quote::Column::Text))).like(format!("%{}%", keyword.to_lowercase())),
        )
        .into_tuple()
        .all(&db)
        .await?;
    let Some(chosen_random) = ids.choose(&mut rng()) else {
        return send_ephemeral_message(ctx, cmd, "Could not find any random quotes for that keyword, do none exist?")
            .await;
    };

    let quote = Quote::find_by_id(*chosen_random).one(&db).await?;

    match quote {
        Some(quote) => post_quote(&ctx, quote, cmd.channel_id, Some(cmd)).await,
        None => Err(anyhow!("Selected random keyword quote that ended up not existing")),
    }
}
