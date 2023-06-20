use anyhow::{anyhow, Result};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serenity::{
    all::{Command, CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    client::Context,
    model::permissions::Permissions,
};

use entity::{prelude::Quote, quote};

use crate::{commands::send_ephemeral_message, util::DatabaseTypeMapKey};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("purge")
            .description("Purges all quotes by the specified user.")
            .default_member_permissions(Permissions::MANAGE_MESSAGES)
            .dm_permission(false)
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "The user to purge all quotes from")
                    .required(true),
            ),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let Some(guild_id) = cmd.guild_id else {return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await};

    let permissions = match cmd.member.as_ref().and_then(|m| m.permissions) {
        Some(p) => p,
        None => return Err(anyhow!("Could not fetch member permissions")),
    };
    if !permissions.manage_messages() {
        return send_ephemeral_message(ctx, cmd, "You do not have permission to use this command.").await;
    }

    let user_id = match cmd.data.options.first().map(|id| &id.value) {
        Some(CommandDataOptionValue::User(user)) => user,
        _ => return send_ephemeral_message(ctx, cmd, "No user received, which is needed for deletion.").await,
    };
    let user_name =
        user_id.to_user(&ctx).await.ok().map(|u| u.name).unwrap_or_else(|| "<Can't fetch user>".to_string());

    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();

    Quote::delete_many()
        .filter(quote::Column::AuthorId.eq(user_id.0.get()))
        .filter(quote::Column::ServerId.eq(guild_id.0.get()))
        .exec(&db)
        .await?;
    send_ephemeral_message(ctx, cmd, &format!("Quotes by {user_name} deleted!")).await
}
