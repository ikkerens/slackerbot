use anyhow::{anyhow, Result};
use serenity::{
    client::Context,
    model::application::interaction::{
        application_command::{ApplicationCommandInteraction, CommandDataOption},
        InteractionResponseType,
    },
};

mod delete;
mod purge;
mod quote;
mod readycheck;
mod rolebuttons;
mod rquote;
mod uquote;
mod voicequote;

pub(crate) use rolebuttons::button::pressed as rolebutton_pressed;
pub(crate) use rolebuttons::post::check_for_update as rolebutton_post_check_for_update;

pub(crate) async fn introduce_commands(ctx: &Context) -> Result<()> {
    delete::register(ctx).await?;
    purge::register(ctx).await?;
    quote::register(ctx).await?;
    readycheck::register(ctx).await?;
    rolebuttons::register(ctx).await?;
    rquote::register(ctx).await?;
    uquote::register(ctx).await?;
    voicequote::register(ctx).await?;
    Ok(())
}

pub(crate) async fn handle_command(ctx: Context, cmd: ApplicationCommandInteraction) -> Result<()> {
    info!(
        "Received command from {}#{}: /{} {}",
        cmd.user.name,
        cmd.user.discriminator,
        cmd.data.name,
        unwrap_options(&cmd.data.options, true)
    );

    match cmd.data.name.as_str() {
        "delete" => delete::handle_command(ctx, cmd).await,
        "purge" => purge::handle_command(ctx, cmd).await,
        "quote" => quote::handle_command(ctx, cmd).await,
        "readycheck" => readycheck::handle_command(ctx, cmd).await,
        "rolebuttons" => rolebuttons::handle_command(ctx, cmd).await,
        "rquote" => rquote::handle_command(ctx, cmd).await,
        "uquote" => uquote::handle_command(ctx, cmd).await,
        "voicequote" => voicequote::handle_command(ctx, cmd).await,
        _ => return Err(anyhow!("Unknown command received: {}", cmd.data.name)),
    }?;
    Ok(())
}

async fn send_ephemeral_message(ctx: Context, cmd: ApplicationCommandInteraction, error: &str) -> Result<()> {
    Ok(cmd
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|data| data.ephemeral(true).title("Error!").content(error))
        })
        .await?)
}

fn unwrap_options(options: &[CommandDataOption], first: bool) -> String {
    if options.is_empty() {
        return if first { "" } else { "None" }.to_string();
    }

    options
        .iter()
        .map(|v| {
            let str = match &v.value {
                Some(value) => value.to_string(),
                None => format!("({})", unwrap_options(&v.options, false)),
            };
            format!("{}={}", v.name, str)
        })
        .collect::<Vec<String>>()
        .join(", ")
}
