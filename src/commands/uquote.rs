use anyhow::{anyhow, Result};
use rand::{seq::SliceRandom, thread_rng};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
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
        CreateCommand::new("uquote")
            .description("Posts a random quote by the specified user")
            .dm_permission(false)
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "The user to get a random quote from.")
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
    let user = match cmd.data.options.first().map(|id| &id.value) {
        Some(CommandDataOptionValue::User(user)) => user,
        _ => return send_ephemeral_message(ctx, cmd, "No user received").await,
    };

    let ids: Vec<i64> = Quote::find()
        .select_only()
        .column(quote::Column::Id)
        .filter(quote::Column::ServerId.eq(guild_id.get()))
        .filter(quote::Column::AuthorId.eq(user.get()))
        .into_tuple()
        .all(&db)
        .await?;
    let Some(chosen_random) = ids.choose(&mut thread_rng()) else {
        return send_ephemeral_message(ctx, cmd, "Could not find any random quotes for that user, do none exist?")
            .await;
    };

    let quote = Quote::find_by_id(*chosen_random).one(&db).await?;

    match quote {
        Some(quote) => post_quote(&ctx, quote, cmd.channel_id, Some(cmd)).await,
        None => Err(anyhow!("Selected random user quote that ended up not existing")),
    }
}
