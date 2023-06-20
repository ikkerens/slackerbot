use anyhow::Result;
use chrono::FixedOffset;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serenity::{
    all::CommandInteraction,
    client::Context,
    model::{
        channel::Message,
        guild::Member,
        id::{ChannelId, GuildId, UserId},
        Timestamp,
    },
};

use entity::quote;

use crate::{
    quote::post_quote,
    util::{channel_name, download_file, DatabaseTypeMapKey},
};

pub mod reaction;
pub mod voice;

async fn ingest(
    ctx: Context,
    member: IngestMember,
    channel_id: ChannelId,
    content: String,
    message: Option<Message>,
    response: Option<CommandInteraction>,
) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();

    let avatar = Set(Some(download_file(&member.avatar_url).await?));
    let (attachment, attachment_name) = if let Some(attachment) = message.as_ref().and_then(|msg| {
        msg.attachments.iter().find(|attachment| {
            attachment.content_type.is_some() && attachment.content_type.as_ref().unwrap().starts_with("image/")
        })
    }) {
        (Set(Some(download_file(&attachment.url).await?)), Set(Some(attachment.filename.clone())))
    } else {
        if content.trim().is_empty() {
            return Ok(());
        }
        (Set(None), Set(None))
    };

    let inserted: quote::Model = quote::ActiveModel {
        id: Default::default(),
        server_id: Set(member.guild_id.0.get() as i64),
        channel_id: Set(channel_id.0.get() as i64),
        channel_name: Set(channel_name(&ctx, channel_id).await?),
        message_id: Set(message.as_ref().map(|msg| msg.id.0.get() as i64)),
        timestamp: Set(message
            .map(|m| m.timestamp)
            .unwrap_or_else(Timestamp::now)
            .with_timezone(&FixedOffset::east_opt(0).unwrap())),
        author_id: Set(member.user_id.0.get() as i64),
        author: Set(member.user_name),
        text: Set(content),
        author_image: avatar,
        attachment,
        attachment_name,
    }
    .insert(&db)
    .await?;

    post_quote(&ctx, inserted, channel_id, response).await
}

struct IngestMember {
    guild_id: GuildId,
    user_id: UserId,
    user_name: String,
    avatar_url: String,
}

impl From<Member> for IngestMember {
    fn from(member: Member) -> Self {
        Self {
            guild_id: member.guild_id,
            user_id: member.user.id,
            user_name: member.display_name().to_string(),
            avatar_url: member.face(),
        }
    }
}
