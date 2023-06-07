use serenity::{
    client::{Context, EventHandler},
    model::{
        application::interaction::{
            message_component::MessageComponentInteraction,
            Interaction::{self, ApplicationCommand, MessageComponent},
        },
        channel::{Message, Reaction, ReactionType},
        gateway::{Activity, Ready},
        guild::Role,
        id::{GuildId, RoleId},
    },
};
use tokio::{join, sync::broadcast};

use crate::{
    commands::{handle_ccounter_ingress, handle_command, introduce_commands, rolebutton_press_loop},
    db_integrity,
    ingest::reaction,
};

const QUOTE_REACTION: &str = "💬";

pub(crate) struct Handler {
    component_interactions: broadcast::Sender<(Context, MessageComponentInteraction)>,
}

impl Handler {
    pub fn new() -> Self {
        let (sender, recv) = broadcast::channel(16);
        tokio::spawn(rolebutton_press_loop(recv));
        Self { component_interactions: sender }
    }

    pub fn subscribe_to_component_interactions(&self) -> broadcast::Receiver<(Context, MessageComponentInteraction)> {
        self.component_interactions.subscribe()
    }
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn guild_role_delete(&self, ctx: Context, guild_id: GuildId, role_id: RoleId, _old_role: Option<Role>) {
        if let Err(e) = db_integrity::guild_role_delete(ctx, guild_id, role_id).await {
            error!("Could not perform DB integrity on role deletion: {}", e);
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(e) = handle_ccounter_ingress(&ctx, &msg).await {
            error!("Could not handle ccounter ingress: {}", e);
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
                if let Err(e) = handle_command(self, ctx, cmd).await {
                    error!("Could not handle command: {}", e);
                }
            }
            MessageComponent(int) => {
                if let Err(e) = self.component_interactions.send((ctx, int)) {
                    error!("Could not handle component interaction: {e}");
                }
            }
            _ => {}
        }
    }
}
