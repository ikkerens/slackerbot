use anyhow::{anyhow, Result};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serenity::{
    all::{ComponentInteraction, Mentionable},
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    client::Context,
    model::id::RoleId,
};
use tokio::sync::broadcast::{self, error::RecvError};

use entity::{prelude::RoleButtonServer, role_button_server};

use crate::util::DatabaseTypeMapKey;

pub(crate) async fn press_loop(mut recv: broadcast::Receiver<(Context, ComponentInteraction)>) {
    loop {
        let (ctx, interaction) = match recv.recv().await {
            Ok(interaction) => interaction,
            Err(e) => {
                if matches!(e, RecvError::Closed) {
                    return;
                }

                error!("Error receiving interaction in role button loop: {e}");
                continue;
            }
        };

        if !interaction.data.custom_id.starts_with("role_") {
            continue;
        }

        if let Err(e) = pressed(ctx, interaction).await {
            error!("Could not handle role button press: {e}");
        }
    }
}

async fn pressed(ctx: Context, mut interaction: ComponentInteraction) -> Result<()> {
    let member = match &mut interaction.member {
        Some(member) => member,
        None => return Err(anyhow!("Interaction that did not come from a server.")),
    };

    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let Some(server) = RoleButtonServer::find()
        .filter(role_button_server::Column::ServerId.eq(member.guild_id.get() as i64))
        .one(&db)
        .await?
    else {
        return Err(anyhow!("Button pressed for an unregistered server."));
    };

    let role_id =
        interaction.data.custom_id.strip_prefix("role_").unwrap_or(&interaction.data.custom_id).parse::<RoleId>()?;
    if !server.roles.contains(&(role_id.get() as i64)) {
        return Err(anyhow!("Role was requested that is not in the rolebuttons."));
    }

    let msg = if member.roles.contains(&role_id) {
        member.remove_role(&ctx, role_id).await?;
        format!("I've removed the {} role from you.", role_id.mention())
    } else {
        member.add_role(&ctx, role_id).await?;
        format!("I've given you the {} role.", role_id.mention())
    };

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().ephemeral(true).content(msg)),
        )
        .await?;

    Ok(())
}
