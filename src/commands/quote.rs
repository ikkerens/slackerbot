use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
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
        CreateCommand::new("quote").description("Posts a specific quote").dm_permission(false).add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "id", "A quote id (found in the bottom of a quote)")
                .required(true)
                .min_int_value(0),
        ),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let Some(guild_id) = cmd.guild_id else {
        return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await;
    };
    let id = match cmd.data.options.first().map(|id| &id.value) {
        Some(CommandDataOptionValue::Integer(id)) => *id,
        _ => return send_ephemeral_message(ctx, cmd, "No quote id received").await,
    };

    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();

    match Quote::find_by_id(id).filter(quote::Column::ServerId.eq(guild_id.get())).one(&db).await? {
        Some(quote) => post_quote(&ctx, quote, cmd.channel_id, Some(cmd)).await,
        None => send_ephemeral_message(ctx, cmd, "Quote with that id does not exist!").await,
    }
}
