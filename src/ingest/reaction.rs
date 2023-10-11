use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serenity::{client::Context, model::channel::Reaction};

use entity::{prelude::Quote, quote};

use crate::{
    ingest::{ingest, IngestMember},
    quote::post_quote,
    util::DatabaseTypeMapKey,
};

pub(crate) async fn handle(ctx: Context, reaction: &Reaction) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();

    let message = {
        let mut message = reaction.message(&ctx).await?;
        if message.guild_id.is_none() {
            message.guild_id = reaction.guild_id
        }
        message
    };

    // If the message is from a bot ignore it
    if message.author.bot {
        return Ok(());
    }

    // Check if the quote already exists in our database, and if so, just post it
    let existing = Quote::find().filter(quote::Column::MessageId.eq(message.id.get())).one(&db).await?;
    if let Some(quote) = existing {
        return post_quote(&ctx, quote, reaction.channel_id, None).await;
    }

    // Nope, fetch the content and member, and move on.
    let content = message.content_safe(&ctx);
    let member = message.member(&ctx).await.ok();

    let ingest_member = if let Some(member) = member {
        member.into()
    } else {
        // If the person is no longer in the guild
        IngestMember {
            guild_id: message.guild_id.unwrap(),
            user_id: message.author.id,
            user_name: message.author.name.clone(),
            avatar_url: message.author.face(),
        }
    };

    ingest(ctx, ingest_member, reaction.channel_id, content, Some(message), None).await
}
