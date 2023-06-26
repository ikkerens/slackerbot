use std::sync::Arc;

use anyhow::{anyhow, Result};
use chatgpt::client::ChatGPT;
use sea_orm::DatabaseConnection;
use serenity::{
    builder::CreateAttachment,
    client::Context,
    model::{channel::Channel, id::ChannelId},
    prelude::TypeMapKey,
};
use tokio::select;

pub mod kvstore;

pub(crate) async fn channel_name(ctx: &Context, id: ChannelId) -> Result<String> {
    if let Channel::Guild(channel) = id.to_channel(&ctx).await? {
        Ok(channel.name)
    } else {
        Err(anyhow!("Provided channel_id is not a GuildChannel"))
    }
}

pub(crate) async fn download_file(url: &str) -> Result<Vec<u8>> {
    Ok(reqwest::get(url).await?.bytes().await?.into())
}

pub(crate) fn convert_bytes_to_attachment(name: impl ToString, bytes: Vec<u8>) -> CreateAttachment {
    CreateAttachment::bytes(bytes, name.to_string())
}

#[cfg(windows)]
pub(super) async fn wait_for_signal() -> Result<()> {
    tokio::signal::ctrl_c().await?;
    info!("Received Ctrl+C, shutting down...");
    Ok(())
}

#[cfg(unix)]
pub(super) async fn wait_for_signal() -> Result<()> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;

    select! {
        // Wait for SIGINT (which is sent on the first Ctrl+C)
        _ = interrupt.recv() => {
            info!("Received interrupt signal, shutting down...");
        }
        // Wait for SIGTERM
        _ = terminate.recv() => {
            info!("Received terminate signal, shutting down...");
        }
    };

    Ok(())
}

pub(crate) struct DatabaseTypeMapKey;

impl TypeMapKey for DatabaseTypeMapKey {
    type Value = DatabaseConnection;
}

pub(crate) struct ChatGPTTypeMapKey;

impl TypeMapKey for ChatGPTTypeMapKey {
    type Value = Arc<ChatGPT>;
}
