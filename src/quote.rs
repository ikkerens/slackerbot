use anyhow::Result;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        channel::Channel,
        id::{ChannelId, UserId},
    },
    prelude::Mentionable,
    utils::Colour,
};

use entity::quote;

use crate::util::convert_bytes_to_attachment;

pub(crate) async fn post_quote<'a>(
    ctx: &Context,
    quote: quote::Model,
    channel: ChannelId,
    response: Option<ApplicationCommandInteraction>,
) -> Result<()> {
    let (avatar_url, avatar_data) = if let Some(author_image) = quote.author_image {
        (Some("attachment://avatar.png".to_string()), Some(convert_bytes_to_attachment("avatar.png", author_image)))
    } else {
        (UserId(quote.author_id as u64).to_user(&ctx).await?.avatar_url(), None)
    };

    let image_name = quote.attachment_name.unwrap_or_else(|| "unknown.png".to_string());
    let image = quote.attachment.map(|d| convert_bytes_to_attachment(&image_name, d));

    let channel_name =
        if let Ok(Channel::Guild(guild_channel)) = ChannelId(quote.channel_id as u64).to_channel(&ctx).await {
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
            e.image("attachment://".to_string() + &image_name);
        }
        if quote.text.trim().is_empty() {
            e.description(channel_name);
        } else {
            e.description(format!("{} - {channel_name}", quote.text));
        }
        e.author(|a| {
            if let Some(url) = avatar_url {
                a.icon_url(url);
            }
            a.name(quote.author)
        })
        .footer(|footer| footer.text(format!("Id: {}", quote.id)))
        .colour(Colour::FABLED_PINK)
        .timestamp(quote.timestamp);
        e
    };

    if let Some(interaction) = response {
        interaction
            .create_interaction_response(ctx, |rep| {
                rep.interaction_response_data(|data| {
                    if let Some(avatar) = avatar_data {
                        data.add_file(avatar);
                    }
                    if let Some(image) = image {
                        data.add_file(image);
                    }
                    data.add_embed(embed)
                })
            })
            .await?;
    } else {
        channel
            .send_message(ctx, |msg| {
                if let Some(avatar) = avatar_data {
                    msg.add_file(avatar);
                }
                if let Some(image) = image {
                    msg.add_file(image);
                }
                msg.add_embeds(vec![embed])
            })
            .await?;
    }

    Ok(())
}
