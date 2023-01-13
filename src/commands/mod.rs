use anyhow::{anyhow, Result};
use serenity::{
	client::Context,
	model::application::interaction::{application_command::ApplicationCommandInteraction, InteractionResponseType},
};

mod delete;
mod purge;
mod quote;
mod rquote;
mod uquote;
mod voicequote;

pub(crate) async fn introduce_commands(ctx: &Context) -> Result<()> {
	delete::register(ctx).await?;
	purge::register(ctx).await?;
	quote::register(ctx).await?;
	rquote::register(ctx).await?;
	uquote::register(ctx).await?;
	voicequote::register(ctx).await?;
	Ok(())
}

pub(crate) async fn handle_command(ctx: &Context, cmd: ApplicationCommandInteraction) -> Result<()> {
	info!(
		"Received command from {}#{}: /{} {}",
		cmd.user.name,
		cmd.user.discriminator,
		cmd.data.name,
		cmd.data
			.options
			.iter()
			.map(|v| format!(
				"{}={}",
				v.name,
				v.value
					.as_ref()
					.map(|v| v.to_string())
					.unwrap_or_else(|| "None".to_string())
			))
			.collect::<Vec<String>>()
			.join(", ")
	);

	match cmd.data.name.as_str() {
		"delete" => delete::handle_command(ctx, cmd).await,
		"purge" => purge::handle_command(ctx, cmd).await,
		"quote" => quote::handle_command(ctx, cmd).await,
		"rquote" => rquote::handle_command(ctx, cmd).await,
		"uquote" => uquote::handle_command(ctx, cmd).await,
		"voicequote" => voicequote::handle_command(ctx, cmd).await,
		_ => return Err(anyhow!("Unknown command received: {}", cmd.data.name)),
	}?;
	Ok(())
}

async fn send_ephemeral_message(ctx: &Context, cmd: ApplicationCommandInteraction, error: &str) -> Result<()> {
	Ok(cmd
		.create_interaction_response(ctx, |response| {
			response
				.kind(InteractionResponseType::ChannelMessageWithSource)
				.interaction_response_data(|data| data.ephemeral(true).title("Error!").content(error))
		})
		.await?)
}
