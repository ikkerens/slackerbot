use anyhow::{anyhow, Result};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serenity::{
    client::Context,
    model::{
        application::interaction::{message_component::MessageComponentInteraction, InteractionResponseType},
        id::RoleId,
    },
    prelude::Mentionable,
};

use entity::{prelude::RoleButtonServer, role_button_server};

use crate::util::DatabaseTypeMapKey;

pub(crate) async fn pressed(ctx: Context, mut interaction: MessageComponentInteraction) -> Result<()> {
    let member = match &mut interaction.member {
        Some(member) => member,
        None => return Err(anyhow!("Interaction that did not come from a server.")),
    };

    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let Some(server) =
        RoleButtonServer::find().filter(role_button_server::Column::ServerId.eq(member.guild_id.0 as i64)).one(&db).await? else { return Err(anyhow!("Button pressed for an unregistered server.")) };

    let role_id =
        interaction.data.custom_id.strip_prefix("role_").unwrap_or(&interaction.data.custom_id).parse::<RoleId>()?;
    if !server.roles.contains(&(role_id.0 as i64)) {
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
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|data| data.ephemeral(true).title("Done!").content(msg))
        })
        .await?;

    Ok(())
}
