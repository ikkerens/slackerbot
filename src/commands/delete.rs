use anyhow::{anyhow, Result};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serenity::{
    client::Context,
    model::{
        application::{
            command::{Command, CommandOptionType},
            interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
        },
        permissions::Permissions,
    },
};

use entity::{prelude::Quote, quote};

use crate::{commands::send_ephemeral_message, util::DatabaseTypeMapKey};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_application_command(&ctx, |command| {
        command
            .name("delete")
            .description("Deletes a specific quote")
            .default_member_permissions(Permissions::MANAGE_MESSAGES)
            .dm_permission(false)
            .create_option(|option| {
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
    let Some(guild_id) = cmd.guild_id else {return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await};

    let permissions = match cmd.member.as_ref().and_then(|m| m.permissions) {
        Some(p) => p,
        None => return Err(anyhow!("Could not fetch member permissions")),
    };
    if !permissions.manage_messages() {
        return send_ephemeral_message(ctx, cmd, "You do not have permission to use this command.").await;
    }

    let id = match cmd.data.options.first().and_then(|id| id.resolved.clone()) {
        Some(CommandDataOptionValue::Integer(id)) => id,
        _ => return send_ephemeral_message(ctx, cmd, "No quote id received, which is needed for deletion.").await,
    };

    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();

    Quote::delete_by_id(id).filter(quote::Column::ServerId.eq(guild_id.0 as i64)).exec(&db).await?;
    send_ephemeral_message(ctx, cmd, &format!("Quote with id {} deleted!", id)).await
}
