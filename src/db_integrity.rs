use anyhow::{anyhow, Result};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serenity::{
    client::Context,
    model::id::{GuildId, RoleId},
};

use entity::{prelude::RoleButtonServer, role_button_server};

use crate::{commands::rolebutton_post_check_for_update, util::DatabaseTypeMapKey};

pub(crate) async fn guild_role_delete(ctx: Context, guild_id: GuildId, role_id: RoleId) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let mut server = match RoleButtonServer::find()
        .filter(role_button_server::Column::ServerId.eq(guild_id.0 as i64))
        .one(&db)
        .await?
    {
        Some(server) => server.into_active_model(),
        None => return Ok(()),
    };

    let mut roles = match server.roles.take() {
        Some(roles) => roles,
        None => return Err(anyhow!("Guild without roles")),
    };
    let mut emojis = match server.role_emojis.take() {
        Some(emojis) => emojis,
        None => return Err(anyhow!("Guild with roles but no emojis")),
    };

    let index = roles.iter().position(|x| *x == role_id.0 as i64);
    match index {
        Some(index) => {
            roles.remove(index);
            emojis.remove(index)
        }
        None => return Ok(()),
    };
    server.roles = Set(roles);
    server.role_emojis = Set(emojis);
    let model = server.update(&db).await?;

    tokio::spawn(rolebutton_post_check_for_update(ctx, model));

    Ok(())
}
