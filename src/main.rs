#[macro_use]
extern crate tracing;

use std::env;

use anyhow::{anyhow, Result};
use sea_orm::{Database, DatabaseConnection};
use serenity::{client::ClientBuilder, prelude::GatewayIntents, prelude::TypeMapKey};
use tracing_subscriber::filter::EnvFilter;

use crate::handler::Handler;
use migration::{Migrator, MigratorTrait};

mod handler;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::fmt()
		.with_env_filter(EnvFilter::new("slackerbot=debug"))
		.with_target(false)
		.init();

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
			GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_MESSAGE_REACTIONS | GatewayIntents::MESSAGE_CONTENT,
		)
		.event_handler(Handler)
		.await?
	};

	{
		let mut data = discord_client.data.write().await;
		data.insert::<DatabaseTypeMapKey>(database);
	}

	info!("Setup complete. Starting bot...");
	Ok(discord_client.start_autosharded().await?)
}

pub(crate) struct DatabaseTypeMapKey;

impl TypeMapKey for DatabaseTypeMapKey {
	type Value = DatabaseConnection;
}
