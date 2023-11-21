//! Reactinator binary.

use std::path::PathBuf;

use clap::Parser;
use reactinator::Handler;
use serenity::{model::prelude::*, Client};

/// Reactinator
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct CommandLineArguments {
    /// Discord token to use. Chosen over the path if present.
    #[arg(env)]
    pub discord_token: Option<String>,

    /// Discord token provided through a path to file containing the
    /// token.
    #[arg(long, default_value("discord.token"))]
    pub discord_token_path: PathBuf,
}

#[tokio::main]
async fn main() {
    let command_line_arguments = CommandLineArguments::parse();
    tracing_subscriber::fmt::init();

    tracing::info!(
        target: "CommandLineArguments",
        "{:?}",
        command_line_arguments,
    );

    let token = command_line_arguments.discord_token.unwrap_or_else(|| {
        std::fs::read_to_string(&command_line_arguments.discord_token_path).unwrap_or_else(|err| {
            panic!(
                "unable to read discord token at `{}` due to `{}`",
                command_line_arguments.discord_token_path.display(),
                err,
            )
        })
    });

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
