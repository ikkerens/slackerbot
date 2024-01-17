use std::{string::ToString, time::Duration};

use anyhow::Result;
use serenity::builder::EditInteractionResponse;
use serenity::{
    all::{
        ButtonStyle, Command, CommandInteraction, ComponentInteraction, ComponentInteractionDataKind, RoleId, UserId,
    },
    builder::{
        CreateActionRow, CreateButton, CreateCommand, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind,
        CreateSelectMenuOption, EditMessage,
    },
    client::Context,
    model::{channel::ReactionType, guild::Member, id::EmojiId, prelude::ChannelId, Colour, Permissions},
    prelude::Mentionable,
};
use tokio::{
    select,
    time::{sleep, sleep_until, Instant},
};

use crate::{
    commands::{
        edit_interaction,
        readycheck::ReadyState::{NotReady, Ready, Unknown},
        send_ephemeral_message,
    },
    handler::Handler,
};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("readycheck")
            .description("Starts a readycheck for a group of people")
            .default_member_permissions(Permissions::MENTION_EVERYONE)
            .dm_permission(false),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(handler: &Handler, ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let Some(guild_id) = cmd.guild_id else {
        return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await;
    };

    // Start the setup wizard, allowing the selection of roles, users and the duration
    let (duration, roles, users) = match setup(handler, &ctx, &cmd).await? {
        SetupResult::Invalid(e) => {
            cmd.edit_response(&ctx, EditInteractionResponse::new().content(e).components(vec![])).await?;
            return Ok(());
        }
        SetupResult::Valid { duration, roles, users } => (duration, roles, users),
    };

    let mentions = roles
        .iter()// Create an iterator
        .map(|r| r.mention()) // Map all roles to Mentions
        .chain(users.iter().map(|u| u.mention())) // Chain all users mapped to mentions at the end 
        .map(|m| m.to_string()) // Convert to strings
        .collect::<Vec<_>>()
        .join(" "); // And join them into one comma-separated string

    // Get a list of all members, and filter them to see if they have the role.
    // Then, while doing so, give them the "Unknown" status so they can fill it in themselves.
    let members = guild_id.members(&ctx, None, None).await?;
    let mut members_that_match: Vec<(Member, ReadyState)> = members
        .into_iter()
        .filter(|m| {
            if users.contains(&m.user.id) {
                return true;
            }

            for role in &roles {
                if m.roles.contains(role) {
                    return true;
                }
            }

            false
        })
        .map(|m| (m, Unknown))
        .collect();

    // If more than 25 people match this criteria, we abort. Discord doesn't allow more fields than that.
    if members_that_match.is_empty() {
        return edit_interaction(ctx, cmd, "Can't send a readycheck to zero people.").await;
    }
    if members_that_match.len() > 25 {
        return edit_interaction(ctx, cmd, "The readycheck only works for up to 25 people.").await;
    }

    members_that_match.sort_by_key(|(member, _)| member.user.name.to_owned());

    // Subscribe to component interaction events
    let mut recv = handler.subscribe_to_component_interactions();
    let interaction_prefix = format!("rc_{}_", cmd.id);

    // Send the initial message with the buttons attached
    let mut readycheck_msg = cmd
        .channel_id
        .send_message(
            &ctx,
            CreateMessage::new().add_embed(create_embed(&members_that_match, false)).components(vec![
                CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("{interaction_prefix}1"))
                        .emoji(Ready.to_emoji())
                        .label("Ready")
                        .style(ButtonStyle::Success),
                    CreateButton::new(format!("{interaction_prefix}0"))
                        .emoji(NotReady.to_emoji())
                        .label("Not ready")
                        .style(ButtonStyle::Danger),
                ]),
            ]),
        )
        .await?;
    cmd.delete_response(&ctx).await?;

    tokio::spawn(shadow_ping(ctx.clone(), mentions.to_string(), cmd.channel_id));

    let end_time = Instant::now() + duration;
    let mut needs_update = false;
    loop {
        // Now we wait for either someone to press a button, or for the readycheck to expire
        let (interaction_ctx, interaction): (Context, ComponentInteraction) = select! {
            interaction = recv.recv() => {
                match interaction {
                    Ok(interaction) => interaction,
                    Err(e) => {
                        error!("Error receiving interaction in readycheck loop: {e}");
                        continue;
                    }
                }
            },
            _ = sleep_until(end_time) => {
                break
            }
        };

        // Check if that interaction belongs to this readycheck
        let status_str = match interaction.data.custom_id.strip_prefix(&interaction_prefix) {
            Some(str) => str,
            None => continue,
        };

        let status = members_that_match.iter_mut().find(|(member, _)| member.user.id == interaction.user.id);
        if let Some((_, status)) = status {
            // Someone pressed a button, and they're part of the readycheck, mark them.
            let new_state = match status_str {
                "1" => Ready,
                "0" => NotReady,
                _ => continue,
            };
            *status = new_state;
            needs_update = true;
            if let Err(e) = interaction.create_response(&ctx, CreateInteractionResponse::Acknowledge).await {
                error!("Could not send button confirmation to user for readycheck: {e}");
            }
        } else {
            // Someone who isn't part of the readycheck pressed the button, send them an error.
            tokio::spawn(async move {
                let result = interaction
                    .create_response(
                        interaction_ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content("This readycheck is not for you!"),
                        ),
                    )
                    .await;
                if let Err(e) = result {
                    error!("Could not reply to non-readycheck user replying to readycheck: {e}");
                }
            });
        }

        if needs_update {
            // Check if everyone is ready, if so, send a message, otherwise just update the embed.
            // We'll also want to update the embed if we send a message, but the loop-break will already take care of that.
            if members_that_match.iter().any(|(_, status)| *status != Ready) {
                readycheck_msg.edit(&ctx, EditMessage::new().embed(create_embed(&members_that_match, false))).await?;
            } else {
                cmd.channel_id
                    .send_message(
                        &ctx,
                        CreateMessage::new().content(format!("{}, everyone is ready!", cmd.user.mention())),
                    )
                    .await?;
                break;
            }
        }
    }

    // Mark everyone that has not responded as not ready.
    for (_, status) in members_that_match.iter_mut() {
        if *status == Unknown {
            *status = NotReady;
        }
    }
    readycheck_msg
        .edit(&ctx, EditMessage::new().components(vec![]).embed(create_embed(&members_that_match, true)))
        .await?;

    // And then wait 10 minutes before we clean up the readycheck
    sleep(Duration::from_secs(10 * 60)).await;
    if let Err(e) = readycheck_msg.delete(&ctx).await {
        error!("Could not delete readycheck after 10 minutes: {e}");
    }

    Ok(())
}

enum SetupResult {
    Valid { duration: Duration, roles: Vec<RoleId>, users: Vec<UserId> },
    Invalid(&'static str),
}

async fn setup(handler: &Handler, ctx: &Context, cmd: &CommandInteraction) -> Result<SetupResult> {
    let setup_end_time = Instant::now() + Duration::from_secs(60);
    let mention_id = format!("rou_{}_mention", cmd.id);
    let timeout_id = format!("rou_{}_timeout", cmd.id);
    let submit_id = format!("rou_{}_submit", cmd.id);

    let mut recv = handler.subscribe_to_component_interactions();

    // First we send a prompt to the user, asking them to choose who to ping.
    cmd.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("Select the roles/users you wish to include in this readycheck.")
                .components(vec![
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(
                            mention_id.as_str(),
                            CreateSelectMenuKind::Mentionable { default_roles: None, default_users: None },
                        )
                        .min_values(1)
                        .max_values(25),
                    ),
                    CreateActionRow::SelectMenu(CreateSelectMenu::new(
                        timeout_id.as_str(),
                        CreateSelectMenuKind::String {
                            options: vec![
                                CreateSelectMenuOption::new("1 minute", "60").default_selection(true),
                                CreateSelectMenuOption::new("5 minutes", "300"),
                                CreateSelectMenuOption::new("1 hour", "3600"),
                            ],
                        },
                    )),
                    CreateActionRow::Buttons(vec![CreateButton::new(submit_id.as_str()).label("Start readycheck!")]),
                ]),
        ),
    )
    .await?;

    let mut duration = Duration::from_secs(60);
    let mut roles = Vec::new();
    let mut users = Vec::new();

    loop {
        let (interaction_ctx, interaction): (Context, ComponentInteraction) = select! {
            interaction = recv.recv() => {
                match interaction {
                    Ok(interaction) => interaction,
                    Err(e) => {
                        error!("Error receiving interaction in readycheck setup loop: {e}");
                        continue;
                    }
                }
            },
            _ = sleep_until(setup_end_time) => {
                return Ok(SetupResult::Invalid("Readycheck setup time expired"));
            }
        };

        // Figure out which information we received
        match interaction.data.custom_id.as_str() {
            custom_id if custom_id == mention_id.as_str() => {
                let ComponentInteractionDataKind::MentionableSelect { values } = &interaction.data.kind else {
                    return Ok(SetupResult::Invalid("Could not parse users/roles."));
                };

                roles.clear();
                users.clear();

                for generic_id in values {
                    // First we try the role ID, as we can do this from the cache
                    let role_attempt = RoleId::new(generic_id.get());

                    if role_attempt.to_role_cached(&interaction_ctx).is_some() {
                        // ID is a known role
                        roles.push(role_attempt);
                    } else {
                        users.push(UserId::new(generic_id.get()));
                    }
                }

                interaction.create_response(interaction_ctx, CreateInteractionResponse::Acknowledge).await?;
            }
            custom_id if custom_id == timeout_id.as_str() => {
                let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind else {
                    return Ok(SetupResult::Invalid("Could not parse duration."));
                };
                let Some(selected_time): Option<Duration> =
                    values.first().and_then(|s| s.parse().ok()).map(Duration::from_secs)
                else {
                    return Ok(SetupResult::Invalid("Could not parse duration."));
                };

                duration = selected_time;

                interaction.create_response(interaction_ctx, CreateInteractionResponse::Acknowledge).await?;
            }
            custom_id if custom_id == submit_id.as_str() => break,
            _ => continue,
        }
    }

    if roles.is_empty() && users.is_empty() {
        return Ok(SetupResult::Invalid("No users and/or roles selected."));
    }

    Ok(SetupResult::Valid { duration, users, roles })
}

fn create_embed(members_with_role: &[(Member, ReadyState)], expired: bool) -> CreateEmbed {
    let mut e = CreateEmbed::default();
    if expired {
        e = e.colour(Colour::DARK_GREY);
    } else {
        e = e.colour(Colour::FABLED_PINK);
    }
    for (member, ready) in members_with_role.iter() {
        e = e.field(format!("{} {}", ready.to_emoji(), member.nick.as_ref().unwrap_or(&member.user.name)), "", true);
    }
    e.title("Readycheck!").description("Ready the feck up <a:catreeee:1110171057853300766>")
}

async fn shadow_ping(ctx: Context, mentions: String, channel: ChannelId) -> Result<()> {
    let msg = channel.send_message(&ctx, CreateMessage::new().content(mentions)).await?;
    msg.delete(ctx).await?;
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
                id: EmojiId::from(1108310596660772944),
            },
        }
    }
}
