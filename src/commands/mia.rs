use serenity::all::{
    ButtonStyle, ChannelId, Colour, Command, CommandInteraction, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateCommand, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditChannel, Permissions,
};
use std::{env::var, str::FromStr};
use tokio::sync::broadcast::{self, error::RecvError};

pub(super) async fn register(ctx: &Context) -> anyhow::Result<()> {
    if var("MIA_VARS").is_err() {
        info!("Not running in MIA mode, skipping module enabling");
        return Ok(());
    }

    Command::create_global_command(
        ctx,
        CreateCommand::new("mia")
            .description("Don't ask")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .dm_permission(false),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> anyhow::Result<()> {
    cmd.channel_id
        .send_message(
            &ctx,
            CreateMessage::new()
                .add_embed(
                    CreateEmbed::new()
                        .title("Idiot-proof buttons")
                        .description("Just read the buttons, press them if you feel like it.")
                        .colour(Colour::LIGHTER_GREY),
                )
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new("mia_y").label("Going MIA!").style(ButtonStyle::Danger),
                    CreateButton::new("mia_n").label("I'm back!").style(ButtonStyle::Success),
                ])]),
        )
        .await?;

    Ok(())
}

pub(crate) async fn press_loop(mut recv: broadcast::Receiver<(Context, ComponentInteraction)>) {
    let Ok(env) = var("MIA_VARS") else { return };
    let Some((name, channel)) = env.split_once(':') else {
        error!("Invalid MIA_VARS format, expected 'name:channel_id'");
        return;
    };
    let Ok(channel) = ChannelId::from_str(channel) else {
        error!("Invalid channel ID format in MIA_VARS");
        return;
    };

    loop {
        let (ctx, interaction) = match recv.recv().await {
            Ok(interaction) => interaction,
            Err(e) => {
                if matches!(e, RecvError::Closed) {
                    return;
                }

                error!("Error receiving interaction in mia button loop: {e}");
                continue;
            }
        };

        if !interaction.data.custom_id.starts_with("mia_") {
            continue;
        }

        let new_name = match interaction.data.custom_id.as_str() {
            "mia_y" => format!("Is {} MIA: Yes", name),
            "mia_n" => format!("Is {} MIA: No", name),
            _ => {
                error!("Unknown custom id in mia button loop: {}", interaction.data.custom_id);
                continue;
            }
        };

        if let Err(e) = channel.edit(&ctx, EditChannel::new().name(&new_name)).await {
            error!("Failed to rename channel: {}", e);
            if let Err(e) = interaction
                .create_response(
                    &ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Failed to rename channel: {}", e))
                            .ephemeral(true),
                    ),
                )
                .await
            {
                error!("Failed to send error response: {}", e);
            }
            continue;
        }

        if let Err(e) = interaction
            .create_response(
                &ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Channel renamed to: {}", new_name))
                        .ephemeral(true),
                ),
            )
            .await
        {
            error!("Failed to send success response: {}", e);
        }
    }
}
