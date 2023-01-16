use serenity::{
    client::{Context, EventHandler},
    model::{
        application::{interaction::Interaction, interaction::Interaction::ApplicationCommand},
        channel::{Reaction, ReactionType},
        gateway::{Activity, Ready},
    },
};
use tokio::join;

use crate::{
    commands::{handle_command, introduce_commands},
    ingest::reaction,
};

pub(crate) struct Handler;

const QUOTE_REACTION: &str = "💬";

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if reaction.emoji != ReactionType::Unicode(QUOTE_REACTION.to_string()) {
            return;
        }

        let deletion = reaction.delete(&ctx);
        let handle_quote = reaction::handle(&ctx, &reaction);

        let (deletion_result, handling_result) = join!(deletion, handle_quote);
        if let Err(e) = deletion_result {
            error!("Could not delete speech bubble reaction: {}", e);
        }
        if let Err(e) = handling_result {
            error!("Could not handle adding reaction: {}", e);
        }
    }

    async fn ready(&self, ctx: Context, _ready_data: Ready) {
        info!("Bot connected!");
        if let Err(e) = introduce_commands(&ctx).await {
            error!("Could not register global commands: {}", e);
        }

        ctx.shard.set_activity(Some(Activity::playing("in therapy")))
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let ApplicationCommand(cmd) = interaction {
            if let Err(e) = handle_command(&ctx, cmd).await {
                error!("Could not handle command: {}", e);
            }
        }
    }
}
