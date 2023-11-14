//! Reactinator binary.

use reactinator::Handler;
use serenity::{model::prelude::*, Client};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let token = std::env::var("DISCORD_TOKEN")
        .or_else(|_| std::fs::read_to_string("discord.token"))
        .expect(
            "Expected `DISCORD_TOKEN` environment variable \
             or `./discord.token` file containing the token",
        );

    let mut client = Client::builder(
        token,
        GatewayIntents::non_privileged() | GatewayIntents::GUILD_MESSAGE_REACTIONS,
    )
    .event_handler(Handler::new())
    .await
    .expect("Couldn't create client");

    if let Err(err) = client.start().await {
        tracing::error!("client error: {}", err);
    }
}
