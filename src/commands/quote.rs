use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serenity::{
    client::Context,
    model::{
        application::{
            command::{Command, CommandOptionType},
            interaction::application_command::ApplicationCommandInteraction,
        },
        prelude::interaction::application_command::CommandDataOptionValue,
    },
};

use entity::{prelude::Quote, quote};

use crate::{commands::send_ephemeral_message, quote::post_quote, util::DatabaseTypeMapKey};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_application_command(ctx, |command| {
        command.name("quote").description("Posts a specific quote").create_option(|option| {
            option
                .name("id")
                .description("A quote id (found in the bottom of the quote)")
                .kind(CommandOptionType::Integer)
                .min_int_value(0)
                .required(true)
        })
    })
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: &Context, cmd: ApplicationCommandInteraction) -> Result<()> {
    let id = match cmd.data.options.first().and_then(|id| id.resolved.clone()) {
        Some(CommandDataOptionValue::Integer(id)) => id,
        _ => return send_ephemeral_message(ctx, cmd, "No quote id received").await,
    };
    let guild_id = match cmd.guild_id {
        Some(guild_id) => guild_id.0 as i64,
        None => return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await,
    };
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();

    match Quote::find_by_id(id).filter(quote::Column::ServerId.eq(guild_id)).one(&db).await? {
        Some(quote) => post_quote(ctx, quote, cmd.channel_id, Some(cmd)).await,
        None => send_ephemeral_message(ctx, cmd, "Quote with that id does not exist!").await,
    }
}
