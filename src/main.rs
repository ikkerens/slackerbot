#[macro_use]
extern crate tracing;

use std::{env, num::NonZeroU64, sync::Arc};

use anyhow::{anyhow, Result};
use chatgpt::client::ChatGPT;
use chatgpt::config::{ChatGPTEngine, ModelConfigurationBuilder};
use sea_orm::Database;
use serenity::{client::ClientBuilder, gateway::ShardManager, model::id::GuildId, prelude::GatewayIntents};
use tiktoken_rs::cl100k_base;
use tokio::{select, sync::Mutex};
use tracing_subscriber::filter::EnvFilter;

use migration::{Migrator, MigratorTrait};

use crate::util::ChatGPTTypeMapKey;
use crate::{
    handler::Handler,
    util::{wait_for_signal, DatabaseTypeMapKey},
    web::auth::Client,
};

mod commands;
mod db_integrity;
mod handler;
mod ingest;
mod quote;
mod util;
mod web;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().with_env_filter(EnvFilter::new("slackerbot=debug")).with_target(false).init();

    info!("Hello world, I am slackerbot");

    let database = {
        let database_url = env::var("DATABASE_URL").map_err(|_| anyhow!("No DATABASE_URL env var"))?;
        let connection = Database::connect(database_url).await?;
        Migrator::up(&connection, None).await?;
        connection
    };

    let mut discord_client = {
        let discord_token = env::var("DISCORD_TOKEN").map_err(|_| anyhow!("No DISCORD_TOKEN env var"))?;
        ClientBuilder::new(
            discord_token,
            GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::GUILD_MESSAGE_REACTIONS
                | GatewayIntents::MESSAGE_CONTENT
                | GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MEMBERS,
        )
        .event_handler(Handler::new())
        .cache_settings(|c| {
            c.max_messages = 300;
            c
        })
        .await?
    };

    let chatgpt = ChatGPT::new_with_config(
        env::var("CHATGPT_TOKEN").map_err(|_| anyhow!("No CHATGPT_TOKEN env var"))?,
        ModelConfigurationBuilder::default().engine(ChatGPTEngine::Gpt4).max_tokens(4096_u32).build()?,
    )?;

    {
        let client_id = env::var("OAUTH_CLIENT").map_err(|_| anyhow!("No OAUTH_CLIENT env var"))?;
        let client_secret = env::var("OAUTH_SECRET").map_err(|_| anyhow!("No OAUTH_SECRET env var"))?;
        let redirect_uri = env::var("OAUTH_REDIRECT").map_err(|_| anyhow!("No OAUTH_REDIRECT env var"))?;
        let jwt_secret = env::var("JWT_SECRET").map_err(|_| anyhow!("No JWT_SECRET env var"))?;
        let web_whitelist_guild_id = GuildId(
            env::var("WEB_WHITELIST_GUILD_ID")
                .map_err(|_| anyhow!("No WEB_WHITELIST_GUILD_ID"))?
                .parse::<NonZeroU64>()
                .map_err(|e| anyhow!("Could not parse guild id: {}", e))?,
        );
        let auth_client = Client::new(
            client_id,
            client_secret,
            redirect_uri,
            jwt_secret,
            discord_client.http.clone(),
            web_whitelist_guild_id,
        )?;
        web::start(database.clone(), auth_client)?;
    }

    {
        let mut data = discord_client.data.write().await;
        data.insert::<DatabaseTypeMapKey>(database);
        data.insert::<ChatGPTTypeMapKey>((Arc::new(chatgpt), Arc::new(cl100k_base()?)));
    }

    info!("Setup complete. Starting bot...");
    select! {
        _ = wait_for_shutdown(discord_client.shard_manager.clone()) => {
            Ok(())
        },
        result = discord_client.start_autosharded() => {
            Ok(result?)
        }
    }
}

async fn wait_for_shutdown(shard_manager: Arc<Mutex<ShardManager>>) {
    if let Err(e) = wait_for_signal().await {
        error!("Could not register interrupt signals: {}", e);
        return;
    }

    shard_manager.lock().await.shutdown_all().await
}
