use anyhow::Result;
use serenity::{
    all::{ChannelId, CommandInteraction, Member},
    client::Context,
};

use crate::ingest::ingest;

pub(crate) async fn handle(
    ctx: Context,
    member: Member,
    channel: ChannelId,
    content: String,
    cmd: CommandInteraction,
) -> Result<()> {
    ingest(ctx, member.into(), channel, content, None, Some(cmd)).await
}
