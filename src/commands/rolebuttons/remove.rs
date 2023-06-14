use anyhow::{anyhow, Result};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serenity::{
    client::Context,
    model::{
        application::interaction::{
            application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
            InteractionResponseType,
        },
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
    let mut server =
        match RoleButtonServer::find().filter(role_button_server::Column::ServerId.eq(guild_id.0)).one(&db).await? {
            Some(server) => server.into_active_model(),
            None => return send_ephemeral_message(ctx, cmd, "Nothing configured in this server.").await,
        };
    let Some(CommandDataOptionValue::Role(role)) = cmd.data.options.get(0).and_then(|o| o.options.get(0)).and_then(|r| r.resolved.as_ref()) else { return send_ephemeral_message(ctx, cmd, "Could not parse role.").await };

    let mut roles = match server.roles.take() {
        Some(roles) => roles,
        None => return Err(anyhow!("Guild without roles")),
    };
    let mut emojis = match server.role_emojis.take() {
        Some(emojis) => emojis,
        None => return Err(anyhow!("Guild with roles but no emojis")),
    };

    let index = roles.iter().position(|x| *x == role.id.0 as i64);
    match index {
        Some(index) => {
            roles.remove(index);
            emojis.remove(index)
        }
        None => return send_ephemeral_message(ctx, cmd, "That role is not in the list.").await,
    };
    server.roles = Set(roles);
    server.role_emojis = Set(emojis);
    let model = server.update(&db).await?;
    tokio::spawn(post::check_for_update(ctx.clone(), model));

    cmd.create_interaction_response(ctx, |response| {
        response.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(|data| {
            data.ephemeral(true)
                .title("Done!")
                .content(format!("I've removed {} from the rolebuttons.", role.mention()))
        })
    })
    .await?;

    Ok(())
}
