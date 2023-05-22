use std::{string::ToString, time::Duration};

use anyhow::{anyhow, Result};
use duration_str::parse;
use serenity::model::mention::Mention;
use serenity::model::prelude::ChannelId;
use serenity::prelude::Mentionable;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        application::{
            command::{Command, CommandOptionType},
            component::ButtonStyle,
            interaction::{
                application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
                InteractionResponseType,
            },
        },
        channel::{Message, ReactionType},
        guild::Member,
        id::EmojiId,
        Permissions,
    },
    utils::Colour,
};
use tokio::time::sleep;
use tokio::{
    select,
    time::{sleep_until, Instant},
};

use crate::commands::{
    readycheck::ReadyState::{NotReady, Ready, Unknown},
    send_ephemeral_message,
};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_application_command(ctx, |command| {
        command
            .name("readycheck")
            .description("Starts a readycheck for all the person who have a certain role")
            .default_member_permissions(Permissions::MENTION_EVERYONE)
            .dm_permission(false)
            .create_option(|option| {
                option.name("role").description("The role to ready check").kind(CommandOptionType::Role).required(true)
            })
            .create_option(|option| {
                option
                    .name("timeout")
                    .description("The duration how long the readycheck should last. Default: 60s")
                    .kind(CommandOptionType::String)
            })
    })
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: ApplicationCommandInteraction) -> Result<()> {
    let Some(guild_id) = cmd.guild_id else {return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await};

    // Parse the arguments
    let mut args = cmd.data.options.iter().map(|v| &v.resolved).filter_map(|v| v.as_ref());
    let Some(CommandDataOptionValue::Role(role)) = args.next() else { return Err(anyhow!("Could not parse role for first value")) };
    let duration: Duration = if let Some(CommandDataOptionValue::String(duration_str)) = args.next() {
        match parse(duration_str) {
            Ok(duration) => duration,
            Err(_) => {
                return send_ephemeral_message(ctx, cmd, "I could not parse that duration, please try again.").await
            }
        }
    } else {
        Duration::from_secs(60)
    };

    // Get a list of all members, and filter them to see if they have the role.
    // Then, while doing so, give them the "Unknown" status so they can fill it in themselves.
    let members = guild_id.members(&ctx, None, None).await?;
    let mut members_with_role: Vec<(Member, ReadyState)> =
        members.into_iter().filter(|m| m.roles.contains(&role.id)).map(|m| (m, Unknown)).collect();

    // If more than 25 people match this criteria, we abort. Discord doesn't allow more fields than that.
    if members_with_role.len() > 25 {
        return send_ephemeral_message(ctx, cmd, "The readycheck only works for up to 25 people.").await;
    }

    members_with_role.sort_by_key(|(member, _)| member.user.name.to_owned());

    // Send the initial message with the buttons attached
    cmd.create_interaction_response(&ctx, |response| {
        response.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(|data| {
            data.add_embed(create_embed(&role.name, &members_with_role, false)).components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.emoji(Ready.to_emoji()).label("Ready").custom_id("rc_1").style(ButtonStyle::Success)
                    })
                    .create_button(|b| {
                        b.emoji(NotReady.to_emoji()).label("Not ready").custom_id("rc_0").style(ButtonStyle::Danger)
                    })
                })
            })
        })
    })
    .await?;
    let msg: Message = cmd.get_interaction_response(&ctx).await?;

    tokio::spawn(shadow_ping(ctx.clone(), role.mention(), cmd.channel_id));

    let end_time = Instant::now() + duration;
    let mut needs_update = false;
    loop {
        // Now we wait for either someone to press a button, or for the readycheck to expire
        let interaction = select! {
            interaction = msg.await_component_interaction(&ctx.shard) => {
                interaction
            },
            _ = sleep_until(end_time) => {
                break
            }
        };

        if let Some(interaction) = interaction {
            let status = members_with_role.iter_mut().find(|(member, _)| member.user.id == interaction.user.id);
            if let Some((_, status)) = status {
                // Someone pressed a button, and they're part of the readycheck, mark them.
                let new_state = match interaction.data.custom_id.as_str() {
                    "rc_1" => Ready,
                    "rc_0" => NotReady,
                    _ => continue,
                };
                *status = new_state;
                needs_update = true;
                if let Err(e) = interaction
                    .create_interaction_response(&ctx, |r| r.kind(InteractionResponseType::UpdateMessage))
                    .await
                {
                    error!("Could not send button confirmation to user for readycheck: {e}");
                }
            } else {
                // Someone who isn't part of the readycheck pressed the button, send them an error.
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    let result = interaction
                        .create_interaction_response(ctx, |f| {
                            f.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(
                                |data| data.ephemeral(true).title("Error!").content("This readycheck is not for you!"),
                            )
                        })
                        .await;
                    if let Err(e) = result {
                        error!("Could not reply to non-readycheck user replying to readycheck: {e}");
                    }
                });
            }
        }

        if needs_update {
            // Check if everyone is ready, if so, send a message, otherwise just update the embed.
            // We'll also want to update the embed if we send a message, but the loop-break will already take care of that.
            if members_with_role.iter().any(|(_, status)| *status != Ready) {
                cmd.edit_original_interaction_response(&ctx, |m| {
                    m.add_embed(create_embed(&role.name, &members_with_role, false))
                })
                .await?;
            } else {
                cmd.channel_id
                    .send_message(&ctx, |m| m.content(format!("{}, everyone is ready!", cmd.user.mention())))
                    .await?;
                break;
            }
        }
    }

    // Mark everyone that has not responded as not ready.
    for (_, status) in members_with_role.iter_mut() {
        if *status == Unknown {
            *status = NotReady;
        }
    }
    cmd.edit_original_interaction_response(&ctx, |m| {
        m.components(|c| c).add_embed(create_embed(&role.name, &members_with_role, true))
    })
    .await?;

    // And then wait 10 minutes before we clean up the readycheck
    sleep(Duration::from_secs(10 * 60)).await;
    if let Err(e) = cmd.delete_original_interaction_response(&ctx).await {
        error!("Could not delete readycheck after 10 minutes: {e}");
    }

    Ok(())
}

fn create_embed(role: &str, members_with_role: &[(Member, ReadyState)], expired: bool) -> CreateEmbed {
    let mut e = CreateEmbed::default();
    e.title(format!("Readycheck for *{}*", role)).description("Ready the feck up <a:catreeee:1110171057853300766>");
    if expired {
        e.colour(Colour::DARK_GREY);
    } else {
        e.colour(Colour::FABLED_PINK);
    }
    for (member, ready) in members_with_role.iter() {
        e.field(format!("{} {}", ready.to_emoji(), member.nick.as_ref().unwrap_or(&member.user.name)), "", true);
    }
    e
}

async fn shadow_ping(ctx: Context, mention: Mention, channel: ChannelId) -> Result<()> {
    let msg = channel.send_message(&ctx, |m| m.content(mention)).await?;
    msg.delete(&ctx).await?;
    Ok(())
}

#[derive(PartialEq)]
enum ReadyState {
    Unknown,
    Ready,
    NotReady,
}

impl ReadyState {
    fn to_emoji(&self) -> ReactionType {
        match self {
            Unknown => ReactionType::Unicode("❔".to_string()),
            Ready => ReactionType::Unicode("✅".to_string()),
            NotReady => ReactionType::Custom {
                animated: false,
                name: Some("redcross".to_string()),
                id: EmojiId(1108310596660772944),
            },
        }
    }
}
