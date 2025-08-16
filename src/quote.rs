use anyhow::Result;
use serenity::{
    all::{CommandInteraction, CreateInteractionResponse},
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateInteractionResponseMessage, CreateMessage},
    client::Context,
    model::Colour,
    model::{
        channel::Channel,
        id::{ChannelId, UserId},
    },
    prelude::Mentionable,
};

use entity::quote;

use crate::util::convert_bytes_to_attachment;

pub(crate) async fn post_quote(
    ctx: &Context,
    quote: quote::Model,
    channel: ChannelId,
    response: Option<CommandInteraction>,
) -> Result<()> {
    let (avatar_url, avatar_data) = if let Some(author_image) = quote.author_image {
        (Some("attachment://avatar.png".to_string()), Some(convert_bytes_to_attachment("avatar.png", author_image)))
    } else {
        (UserId::from(quote.author_id as u64).to_user(&ctx).await?.avatar_url(), None)
    };

    let image_name = quote.attachment_name.unwrap_or_else(|| "unknown.png".to_string());
    let image = quote.attachment.map(|d| convert_bytes_to_attachment(&image_name, d));

    let channel_name =
        if let Ok(Channel::Guild(guild_channel)) = ChannelId::from(quote.channel_id as u64).to_channel(&ctx).await {
            if let Some(message_id) = quote.message_id {
                format!("https://discord.com/channels/{}/{}/{}", quote.server_id, quote.channel_id, message_id)
            } else {
                guild_channel.mention().to_string()
            }
        } else {
            format!("#{}", quote.channel_name)
        };

    let embed = {
        let mut e = CreateEmbed::default();
        if image.is_some() {
            e = e.image("attachment://".to_string() + &image_name);
        }
        if quote.text.trim().is_empty() {
            e = e.description(channel_name);
        } else {
            e = e.description(format!("{} - {channel_name}", quote.text));
        }

        let mut author = CreateEmbedAuthor::new(quote.author);
        if let Some(url) = avatar_url {
            author = author.icon_url(url);
        }
        e.author(author)
            .footer(CreateEmbedFooter::new(format!("Id: {}", quote.id)))
            .colour(Colour::FABLED_PINK)
            .timestamp(quote.timestamp)
    };

    if let Some(interaction) = response {
        let mut response = CreateInteractionResponseMessage::new();
        if let Some(avatar) = avatar_data {
            response = response.add_file(avatar);
        }
        if let Some(image) = image {
            response = response.add_file(image);
        }
        interaction.create_response(ctx, CreateInteractionResponse::Message(response.add_embed(embed))).await?;
    } else {
        let mut message = CreateMessage::new();
        if let Some(avatar) = avatar_data {
            message = message.add_file(avatar);
        }
        if let Some(image) = image {
            message = message.add_file(image);
        }
        channel.send_message(ctx, message.add_embed(embed)).await?;
    }

    Ok(())
}
