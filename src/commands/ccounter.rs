use std::sync::OnceLock;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serenity::{
    all::{Command, CommandInteraction},
    builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage},
    client::Context,
    model::channel::Message,
};
use tokio::sync::Mutex;

use crate::util::{kvstore, DatabaseTypeMapKey};

static COUNTER_KEY: &str = "ccounter";
static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) async fn handle_ingress(ctx: &Context, msg: &Message) -> Result<()> {
    // Filter out bot messages
    if msg.author.bot {
        return Ok(());
    }

    // Filter out all non-alphabetic characters
    let phrase = msg.content.chars().filter(|c| matches!(*c, 'A'..='Z' | 'a'..='z' | ' ')).collect::<String>();
    // Lowercase, for easy filtering
    let phrase = phrase.to_lowercase();
    // Split into words
    let mut words = phrase.split(' ');
    // Detect the word
    let found = words.any(|w| w == "cum");
    if !found {
        return Ok(());
    }

    // We found the word, we increase the counter.
    let _guard = LOCK.get_or_init(|| Mutex::new(())).lock().await;

    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let mut counter = kvstore::get::<CCounter>(&db, COUNTER_KEY).await?.unwrap_or_else(CCounter::default);

    counter.count += 1;
    kvstore::set(&db, COUNTER_KEY, &counter).await?;

    Ok(())
}

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(ctx, CreateCommand::new("cum").description("Blame Qais").dm_permission(false))
        .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let count = match kvstore::get::<CCounter>(&db, "ccounter").await? {
        Some(counter) => counter.count,
        None => 0,
    };
    let plural = if count == 1 { "time" } else { "times" };

    cmd.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(format!("I have seen cum {count} {plural}.")),
        ),
    )
    .await?;
    Ok(())
}

#[derive(Serialize, Deserialize, Default)]
struct CCounter {
    count: u32,
}
