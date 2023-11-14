//! Add a reaction to the given message or previous message.

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::{
        application::interaction::InteractionResponseType, prelude::command::CommandOptionType,
    },
};

use super::Command;

/// `add_reaction` command.
pub struct AddReaction;

#[async_trait]
impl Command for AddReaction {
    fn register(command: &mut CreateApplicationCommand) -> Self {
        command
            .name("add_reaction")
            .description("Add a reaction to the given message or previous message.")
            .create_option(|command_option| {
                command_option
                    .required(true)
                    .kind(CommandOptionType::String)
                    .name("name")
                    .description("Name of the reaction.")
            });
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
                    .interaction_response_data(|message| {
                        message.content(format!(
                            "add_reaction invoked with {:#?}",
                            command_interaction.data
                        ))
                    })
            })
            .await
        {
            tracing::error!("couldn't respond to slash command due to `{}`", err);
        }
    }
}
