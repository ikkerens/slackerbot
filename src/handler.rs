use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;

pub(crate) struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
	async fn ready(&self, _ctx: Context, _data_about_bot: Ready) {
		info!("Bot connected!")
	}
}
