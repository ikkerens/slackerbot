use anyhow::Result;
use serenity::{
	client::Context,
	model::{
		application::interaction::application_command::ApplicationCommandInteraction, guild::Member, id::ChannelId,
	},
};

use crate::ingest::ingest;

pub(crate) async fn handle(
	ctx: &Context, member: Member, channel: ChannelId, content: String, cmd: ApplicationCommandInteraction,
) -> Result<()> {
	ingest(ctx, member.into(), channel, content, None, Some(cmd)).await
}
