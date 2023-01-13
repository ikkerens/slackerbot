use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serenity::{client::Context, model::channel::Reaction};

use entity::{prelude::Quote, quote};

use crate::{ingest::ingest, quote::post_quote, util::DatabaseTypeMapKey};

pub(crate) async fn handle(ctx: &Context, reaction: &Reaction) -> Result<()> {
	let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();

	let message = {
		let mut message = reaction.message(&ctx).await?;
		if message.guild_id.is_none() {
			message.guild_id = reaction.guild_id
		}
		message
	};

	// Check if the quote already exists in our database, and if so, just post it
	let existing = Quote::find()
		.filter(quote::Column::MessageId.eq(message.id.0))
		.one(&db)
		.await?;
	if let Some(quote) = existing {
		return post_quote(ctx, quote, reaction.channel_id, None).await;
	}

	// Nope, let's fetch the attachments, if any
	let content = message.content_safe(ctx);
	let member = message.member(ctx).await?;

	ingest(ctx, &member, reaction.channel_id, content, Some(message), None).await
}
