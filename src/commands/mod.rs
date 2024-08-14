use anyhow::{anyhow, Result};
use serenity::{
    all::{CommandDataOption, CommandDataOptionValue, CommandInteraction},
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse},
    client::Context,
};

pub(crate) use ccounter::handle_ingress as handle_ccounter_ingress;
pub(crate) use rolebuttons::button::press_loop as rolebutton_press_loop;
pub(crate) use rolebuttons::post::check_for_update as rolebutton_post_check_for_update;

use crate::handler::Handler;

mod ccounter;
mod cquote;
mod delete;
mod kwquote;
mod lamia;
mod purge;
mod quote;
mod readycheck;
mod rolebuttons;
mod rquote;
pub(crate) mod tldr;
mod uquote;
mod voicequote;

pub(crate) async fn introduce_commands(ctx: &Context) -> Result<()> {
    ccounter::register(ctx).await?;
    cquote::register(ctx).await?;
    delete::register(ctx).await?;
    kwquote::register(ctx).await?;
    purge::register(ctx).await?;
    quote::register(ctx).await?;
    lamia::register(ctx).await?;
    readycheck::register(ctx).await?;
    rolebuttons::register(ctx).await?;
    rquote::register(ctx).await?;
    tldr::register(ctx).await?;
    uquote::register(ctx).await?;
    voicequote::register(ctx).await?;
    Ok(())
}

pub(crate) async fn handle_command(handler: &Handler, ctx: Context, cmd: CommandInteraction) -> Result<()> {
    info!("Received command from {}: /{} {}", cmd.user.name, cmd.data.name, unwrap_options(&cmd.data.options, true));

    match cmd.data.name.as_str() {
        "cum" => ccounter::handle_command(ctx, cmd).await,
        "cquote" => cquote::handle_command(ctx, cmd).await,
        "delete" => delete::handle_command(ctx, cmd).await,
        "kwquote" => kwquote::handle_command(ctx, cmd).await,
        "purge" => purge::handle_command(ctx, cmd).await,
        "quote" => quote::handle_command(ctx, cmd).await,
        "days_since_lamia_horny" => lamia::handle_command(ctx, cmd).await,
        "readycheck" => readycheck::handle_command(handler, ctx, cmd).await,
        "rolebuttons" => rolebuttons::handle_command(ctx, cmd).await,
        "rquote" => rquote::handle_command(ctx, cmd).await,
        "tldr" => tldr::handle_command(ctx, cmd).await,
        "uquote" => uquote::handle_command(ctx, cmd).await,
        "voicequote" => voicequote::handle_command(ctx, cmd).await,
        _ => return Err(anyhow!("Unknown command received: {}", cmd.data.name)),
    }?;
    Ok(())
}

async fn send_ephemeral_message(ctx: Context, cmd: CommandInteraction, error: &str) -> Result<()> {
    Ok(cmd
        .create_response(
            ctx,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().ephemeral(true).content(error)),
        )
        .await?)
}

async fn edit_interaction(ctx: Context, cmd: CommandInteraction, error: &str) -> Result<()> {
    Ok(cmd.edit_response(ctx, EditInteractionResponse::new().content(error)).await.map(|_| ())?)
}
fn unwrap_options(options: &[CommandDataOption], first: bool) -> String {
    if options.is_empty() {
        return if first { "" } else { "None" }.to_string();
    }

    options
        .iter()
        .map(|v| {
            let str = match &v.value {
                CommandDataOptionValue::SubCommand(options) => format!("({})", unwrap_options(options, false)),
                CommandDataOptionValue::String(str) => str.to_owned(),
                CommandDataOptionValue::Role(role) => role.to_string(),
                CommandDataOptionValue::Channel(channel) => channel.to_string(),
                CommandDataOptionValue::User(user) => user.to_string(),
                CommandDataOptionValue::Integer(int) => int.to_string(),
                CommandDataOptionValue::Boolean(bool) => bool.to_string(),
                _ => "<unsupported type>".to_string(),
            };
            format!("{}={}", v.name, str)
        })
        .collect::<Vec<String>>()
        .join(", ")
}
