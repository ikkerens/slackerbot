use std::collections::HashMap;

use anyhow::{anyhow, Result};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serenity::{
    all::{CommandInteraction, CreateInteractionResponse},
    builder::{
        CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponseMessage, CreateMessage, EditMessage,
    },
    client::Context,
    model::{
        channel::{Message, ReactionType},
        guild::Role,
        id::{ChannelId, GuildId, RoleId},
        Colour,
    },
};

use entity::{prelude::RoleButtonServer, role_button_server};

use crate::{commands::send_ephemeral_message, util::DatabaseTypeMapKey};

pub(super) async fn handle(
    ctx: Context,
    cmd: CommandInteraction,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let Some(server) =
        RoleButtonServer::find().filter(role_button_server::Column::ServerId.eq(guild_id.get())).one(&db).await?
    else {
        return send_ephemeral_message(ctx, cmd, "Nothing configured in this server.").await;
    };

    let existing_message = match server.post_channel_id.zip(server.post_message_id) {
        Some((channel, message)) => ChannelId::from(channel as u64).message(&ctx, message as u64).await.ok(),
        None => None,
    };

    if let Some(old_message) = &existing_message {
        old_message.delete(&ctx).await?;
    }

    let components = create_components(&ctx, &server, guild_id).await?;

    let message = channel_id
        .send_message(
            &ctx,
            CreateMessage::new()
                .add_embed(
                    CreateEmbed::new()
                        .title("Roles self-service")
                        .description("Click one of the buttons below to give yourself a role")
                        .colour(Colour::FABLED_PINK),
                )
                .components(components),
        )
        .await?;
    let mut db_server = server.into_active_model();
    db_server.post_channel_id = Set(Some(message.channel_id.get() as i64));
    db_server.post_message_id = Set(Some(message.id.get() as i64));
    db_server.save(&db).await?;

    cmd.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("I created the post (but Discord forces me to confirm this to you)"),
        ),
    )
    .await?;

    Ok(())
}

pub(crate) async fn check_for_update(ctx: Context, server: role_button_server::Model) {
    let existing_message = match server.post_channel_id.zip(server.post_message_id) {
        Some((channel, message)) => ChannelId::from(channel as u64).message(&ctx, message as u64).await.ok(),
        None => None,
    };

    if let Some(msg) = existing_message {
        if let Err(e) = update_post(ctx, server, msg).await {
            error!("Could not update existing post: {}", e);
        }
    }
}

async fn update_post(ctx: Context, server: role_button_server::Model, mut msg: Message) -> Result<()> {
    let components = create_components(&ctx, &server, GuildId::from(server.server_id as u64)).await?;
    if components.is_empty() {
        msg.delete(ctx).await?;
    } else {
        msg.edit(ctx, EditMessage::new().components(components)).await?;
    }
    Ok(())
}

async fn create_components(
    ctx: &Context,
    server: &role_button_server::Model,
    guild_id: GuildId,
) -> Result<Vec<CreateActionRow>> {
    let mut roles: Option<HashMap<RoleId, Role>> = None;
    let mut buttons = Vec::new();

    for (index, role) in server.roles.iter().enumerate() {
        let role_id = RoleId::from(*role as u64);
        let Some(emoji_str) = server.role_emojis.get(index) else {
            return Err(anyhow!("Role without emoji on that index"));
        };
        let emoji = emoji_str.parse::<ReactionType>()?;

        let role = match role_id.to_role_cached(ctx) {
            Some(role) => role,
            None => {
                let roles_cache = match roles.as_ref() {
                    Some(roles_cache) => roles_cache,
                    None => {
                        roles = Some(guild_id.roles(&ctx).await?);
                        roles.as_ref().unwrap()
                    }
                };
                let Some(cached_role) = roles_cache.get(&role_id) else { continue };
                cached_role.clone()
            }
        };
        buttons.push(CreateButton::new(format!("role_{role_id}")).emoji(emoji).label(role.name));
    }
    Ok(vec![CreateActionRow::Buttons(buttons)])
}
