use anyhow::Result;
use serenity::{
    all::{Command, CommandInteraction, CreateInteractionResponseMessage},
    builder::{CreateCommand, CreateInteractionResponse},
    client::Context,
};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("days_since_lamia_horny")
            .description("Posts the amount of days since Lamia has last been horny")
            .dm_permission(false),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    cmd.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("It has been **0** (zero) days since Lamia was last found to be horny."),
        ),
    )
    .await?;
    Ok(())
}
