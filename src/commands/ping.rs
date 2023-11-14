//! `ping` command.

use serenity::{
    async_trait, builder::CreateApplicationCommand,
    model::application::interaction::InteractionResponseType,
};

use super::Command;

/// `ping` command.
pub struct Ping;

#[async_trait]
impl Command for Ping {
    fn register(command: &mut CreateApplicationCommand) -> Self {
        command.name("ping").description("Ping command");
        Self
    }

    async fn interaction(
        &mut self,
        command_interaction: &serenity::model::prelude::application_command::ApplicationCommandInteraction,
        context: &serenity::prelude::Context,
    ) {
        if let Err(err) = command_interaction
            .create_interaction_response(&context.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content("pong"))
            })
            .await
        {
            tracing::error!("couldn't respond to slash command due to `{}`", err);
        }
    }
}
