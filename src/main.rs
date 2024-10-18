#[macro_use]
extern crate tracing;

use std::{env, sync::Arc, time::Duration};

use chatgpt::{
    client::ChatGPT,
    config::{ChatGPTEngine, ModelConfigurationBuilder},
};
use rs_utils::{exit_on_anyhow_error, exit_on_error, get_env_exit, wait_for_signal};
use sea_orm::Database;
use serenity::{cache, client::ClientBuilder, model::id::GuildId, prelude::GatewayIntents};
use tiktoken_rs::o200k_base;
use tokio::{select, sync::Mutex};

use migration::{Migrator, MigratorTrait};

use crate::{
    commands::tldr,
    handler::Handler,
    util::{DatabaseTypeMapKey, TLDRTypeMapKey, TLDRUsageStatus::Unused},
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
async fn main() {
    rs_utils::setup_logs(env!("CARGO_PKG_NAME"), vec![]);

    info!("Hello world, I am slackerbot");

    let database = {
        let database_url = get_env_exit("DATABASE_URL");
        let connection = exit_on_error(
            Database::connect(database_url).await,
            "Could not connect to database",
        );
        exit_on_error(
            Migrator::up(&connection, None).await,
            "Could not run migrations",
        );
        connection
    };

    let mut discord_client = {
        let mut settings = cache::Settings::default();
        settings.max_messages = 100;
        let discord_token = get_env_exit("DISCORD_TOKEN");
        let discord_client = ClientBuilder::new(
            discord_token,
            GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::GUILD_MESSAGE_REACTIONS
                | GatewayIntents::MESSAGE_CONTENT
                | GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MEMBERS,
        )
            .event_handler(Handler::new())
            .cache_settings(settings)
            .await;
        exit_on_error(discord_client, "Could not create discord client")
    };

    let chatgpt = exit_on_error(ChatGPT::new_with_config(
        get_env_exit("CHATGPT_TOKEN"),
        exit_on_error(ModelConfigurationBuilder::default()
                          .engine(ChatGPTEngine::Custom("gpt-4o"))
                          .timeout(Duration::from_secs(60))
                          .max_tokens(tldr::GPT_MAX_RESPONSE)
                          .build(), "Could not build ChatGPT model configuration"),
    ), "Could not initialize ChatGPT client");

    {
        let client_id = get_env_exit("OAUTH_CLIENT");
        let client_secret = get_env_exit("OAUTH_SECRET");
        let redirect_uri = get_env_exit("OAUTH_REDIRECT");
        let jwt_secret = get_env_exit("JWT_SECRET");
        let web_whitelist_guild_id = GuildId::new(
            exit_on_error(
                get_env_exit("WEB_WHITELIST_GUILD_ID")
                    .parse::<u64>(), "Could not parse guild id"),
        );
        let auth_client = exit_on_anyhow_error(Client::new(
            client_id,
            client_secret,
            redirect_uri,
            jwt_secret,
            discord_client.http.clone(),
            web_whitelist_guild_id,
        ), "Could not initialise oAuth client");
        exit_on_anyhow_error(web::start(database.clone(), auth_client), "Could not start web server")
    }

    {
        let mut data = discord_client.data.write().await;
        data.insert::<DatabaseTypeMapKey>(database);
        data.insert::<TLDRTypeMapKey>((Arc::new(chatgpt), Arc::new(exit_on_anyhow_error(o200k_base(), "Could not initialise tokenizer")), Arc::new(Mutex::new(Unused))));
    }

    info!("Setup complete. Starting bot...");
    select! {
        _ = wait_for_signal() => {},
        result = discord_client.start_autosharded() => {
            exit_on_error(result, "Discord client returned error");
        }
    }

    discord_client.shard_manager.shutdown_all().await
}
