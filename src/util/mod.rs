use std::sync::Arc;

use anyhow::{anyhow, Result};
use chatgpt::client::ChatGPT;
use chrono::{DateTime, Utc};
use sea_orm::DatabaseConnection;
use serenity::{
    builder::CreateAttachment,
    client::Context,
    model::{channel::Channel, id::ChannelId},
    prelude::TypeMapKey,
};
use tiktoken_rs::CoreBPE;
use tokio::{sync::Mutex};

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

pub(crate) struct DatabaseTypeMapKey;

impl TypeMapKey for DatabaseTypeMapKey {
    type Value = DatabaseConnection;
}

pub(crate) struct TLDRTypeMapKey;

impl TypeMapKey for TLDRTypeMapKey {
    // ChatGPT is the connection library for GPT
    // CoreBPE is our token counter
    // The usage status is to throttle usage of the command
    type Value = (Arc<ChatGPT>, Arc<CoreBPE>, Arc<Mutex<TLDRUsageStatus>>);
}

pub(crate) enum TLDRUsageStatus {
    Unused,
    Running(DateTime<Utc>),
    // These two instants are a timeout for when the command becomes available again
    Done(DateTime<Utc>),
}
