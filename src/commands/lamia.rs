use anyhow::Result;
use serenity::{
    client::Context,
    model::application::{
        command::Command,
        interaction::{application_command::ApplicationCommandInteraction, InteractionResponseType},
    },
};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_application_command(ctx, |command| {
        command
            .name("days_since_lamia_horny")
            .description("Posts the amount of days since Lamia has last been horny")
            .dm_permission(false)
    })
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: ApplicationCommandInteraction) -> Result<()> {
    cmd.create_interaction_response(ctx, |response| {
        response.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(|data| {
            data.content("It has been **0** (zero) days since Lamia was last found to be horny.")
        })
    })
    .await?;
    Ok(())
}
