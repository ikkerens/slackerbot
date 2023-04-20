use std::str::FromStr;

use anyhow::Result;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serenity::{
    client::Context,
    model::{
        application::interaction::{
            application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
            InteractionResponseType,
        },
        channel::ReactionType,
        id::GuildId,
    },
    prelude::Mentionable,
};

use entity::{prelude::RoleButtonServer, role_button_server};

use crate::{
    commands::{rolebuttons::post, send_ephemeral_message},
    util::DatabaseTypeMapKey,
};

pub(super) async fn handle(ctx: Context, cmd: ApplicationCommandInteraction, guild_id: GuildId) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let mut server = match RoleButtonServer::find()
        .filter(role_button_server::Column::ServerId.eq(guild_id.0 as i64))
        .one(&db)
        .await?
    {
        Some(server) => server.into_active_model(),
        None => role_button_server::ActiveModel { server_id: Set(guild_id.0 as i64), ..Default::default() },
    };
    let args = cmd.data.options.get(0).map(|o| &o.options);
    let Some(CommandDataOptionValue::Role(role)) = args.and_then(|o| o.get(0)).and_then(|r| r.resolved.as_ref()) else { return send_ephemeral_message(ctx, cmd, "Could not parse role.").await };
    let Some(CommandDataOptionValue::String(emoji)) = args.and_then(|o| o.get(1)).and_then(|r| r.resolved.as_ref()) else { return send_ephemeral_message(ctx, cmd, "Could not parse emoji.").await };
    if ReactionType::from_str(emoji.as_str()).is_err() {
        return send_ephemeral_message(ctx, cmd, "Could not parse emoji").await;
    }

    let mut roles = match server.roles.take() {
        Some(roles) => roles,
        None => vec![],
    };

    if roles.contains(&(role.id.0 as i64)) {
        return send_ephemeral_message(ctx, cmd, "That role is already registered.").await;
    }

    let mut emojis = match server.role_emojis.take() {
        Some(emojis) => emojis,
        None => vec![],
    };

    roles.push(role.id.0 as i64);
    emojis.push(emoji.clone());
    server.roles = Set(roles);
    server.role_emojis = Set(emojis);
    let model = if server.id.is_unchanged() { server.update(&db).await? } else { server.insert(&db).await? };

    tokio::spawn(post::check_for_update(ctx.clone(), model));

    cmd.create_interaction_response(ctx, |response| {
        response.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(|data| {
            data.ephemeral(true).title("Done!").content(format!("I've added {} to the rolebuttons.", role.mention()))
        })
    })
    .await?;

    Ok(())
}
