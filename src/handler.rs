use serenity::{
    client::{Context, EventHandler},
    model::{
        application::interaction::Interaction::{self, ApplicationCommand, MessageComponent},
        channel::{Reaction, ReactionType},
        gateway::{Activity, Ready},
        guild::Role,
        id::{GuildId, RoleId},
    },
};
use tokio::join;

use crate::{
    commands::{handle_command, introduce_commands, rolebutton_pressed},
    db_integrity,
    ingest::reaction,
};

pub(crate) struct Handler;

const QUOTE_REACTION: &str = "💬";

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn guild_role_delete(&self, ctx: Context, guild_id: GuildId, role_id: RoleId, _old_role: Option<Role>) {
        if let Err(e) = db_integrity::guild_role_delete(ctx, guild_id, role_id).await {
            error!("Could not perform DB integrity on role deletion: {}", e);
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if reaction.emoji != ReactionType::Unicode(QUOTE_REACTION.to_string()) {
            return;
        }

        let deletion = reaction.delete(ctx.clone());
        let handle_quote = reaction::handle(ctx, &reaction);

        let (deletion_result, handling_result) = join!(deletion, handle_quote);
        if let Err(e) = deletion_result {
            error!("Could not delete speech bubble reaction: {}", e);
        }
        if let Err(e) = handling_result {
            error!("Could not handle adding reaction: {}", e);
        }
    }

    async fn ready(&self, ctx: Context, _ready: Ready) {
        info!("Bot connected!");
        if let Err(e) = introduce_commands(&ctx).await {
            error!("Could not register global commands: {}", e);
        }

        ctx.shard.set_activity(Some(Activity::playing("in therapy")))
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            ApplicationCommand(cmd) => {
                if let Err(e) = handle_command(ctx, cmd).await {
                    error!("Could not handle command: {}", e);
                }
            }
            MessageComponent(int) => {
                if int.data.custom_id.starts_with("rc_") {
                    // Ignore
                } else {
                    // role_ or nothing
                    if let Err(e) = rolebutton_pressed(ctx, int).await {
                        error!("Could not handle button press: {}", e);
                    }
                }
            }
            _ => {}
        }
    }
}
