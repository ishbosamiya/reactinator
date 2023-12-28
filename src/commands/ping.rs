//! `ping` command.

use serenity::{
    async_trait,
    builder::{CreateCommand, CreateInteractionResponse},
};

use crate::BotContext;

use super::Command;

/// `ping` command.
pub struct Ping;

#[async_trait]
impl Command for Ping {
    fn register(command: &mut CreateCommand, _bot_context: &BotContext) -> Self {
        command.name("ping").description("Ping command");
        Self
    }

    async fn interaction(
        &mut self,
        command_interaction: &serenity::model::application::CommandInteraction,
        context: &serenity::prelude::Context,
        _bot_context: &BotContext,
    ) {
        if let Err(err) = command_interaction
            .create_response(&context.http, CreateInteractionResponse::Pong)
            .await
        {
            tracing::error!("couldn't respond to slash command due to `{}`", err);
        }
    }
}
