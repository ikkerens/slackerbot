use std::str::FromStr;

use anyhow::{anyhow, Result};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, Mentionable},
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    client::Context,
    model::{channel::ReactionType, id::GuildId},
};

use entity::{prelude::RoleButtonServer, role_button_server};

use crate::{
    commands::{rolebuttons::post, send_ephemeral_message},
    util::DatabaseTypeMapKey,
};

pub(super) async fn handle(ctx: Context, cmd: CommandInteraction, guild_id: GuildId) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let mut server = match RoleButtonServer::find()
        .filter(role_button_server::Column::ServerId.eq(guild_id.get()))
        .one(&db)
        .await?
    {
        Some(server) => server.into_active_model(),
        None => role_button_server::ActiveModel { server_id: Set(guild_id.get() as i64), ..Default::default() },
    };
    let Some(CommandDataOptionValue::SubCommand(args)) = cmd.data.options.first().map(|o| &o.value) else {
        return Err(anyhow!("Could not parse subcommand options"));
    };
    let Some(CommandDataOptionValue::Role(role_id)) = args.first().map(|r| &r.value) else {
        return send_ephemeral_message(ctx, cmd, "Could not parse role.").await;
    };
    let Some(CommandDataOptionValue::String(emoji)) = args.get(1).map(|r| &r.value) else {
        return send_ephemeral_message(ctx, cmd, "Could not parse emoji.").await;
    };
    if ReactionType::from_str(emoji.as_str()).is_err() {
        return send_ephemeral_message(ctx, cmd, "Could not parse emoji").await;
    }

    let mut roles = match server.roles.take() {
        Some(roles) => roles,
        None => vec![],
    };

    if roles.contains(&(role_id.get() as i64)) {
        return send_ephemeral_message(ctx, cmd, "That role is already registered.").await;
    }

    let mut emojis = match server.role_emojis.take() {
        Some(emojis) => emojis,
        None => vec![],
    };

    roles.push(role_id.get() as i64);
    emojis.push(emoji.clone());
    server.roles = Set(roles);
    server.role_emojis = Set(emojis);
    let model = if server.id.is_unchanged() { server.update(&db).await? } else { server.insert(&db).await? };

    tokio::spawn(post::check_for_update(ctx.clone(), model));

    cmd.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("I've added {} to the rolebuttons.", role_id.mention())),
        ),
    )
    .await?;

    Ok(())
}
