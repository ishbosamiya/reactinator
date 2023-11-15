//! Commands.

pub mod add_reaction;
pub mod list_custom_emojis;
pub mod ping;

use std::borrow::Cow;

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::application_command::CommandDataOption,
    },
    prelude::*,
};

use crate::BotContext;

/// Discord command.
#[async_trait]
pub trait Command: Send + Sync + 'static {
    /// Register the command.
    fn register(command: &mut CreateApplicationCommand, bot_context: &BotContext) -> Self
    where
        Self: Sized;

    /// Interaction with the command.
    async fn interaction(
        &mut self,
        command_interaction: &ApplicationCommandInteraction,
        context: &Context,
        bot_context: &BotContext,
    );
}

/// Convert the [`ApplicationCommandInteraction`] to a string.
pub fn command_interaction_to_string(
    command_interaction: &ApplicationCommandInteraction,
) -> String {
    format!(
        "/{}{}{}",
        command_interaction.data.name,
        if command_interaction.data.options.is_empty() {
            ""
        } else {
            " "
        },
        command_data_options_to_string(&command_interaction.data.options)
    )
}

/// Convert the [`CommandDataOption`]s to a string.
pub fn command_data_options_to_string(command_data_options: &[CommandDataOption]) -> String {
    command_data_options
        .iter()
        .map(|command_data_option| command_data_option_to_string(command_data_option))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Convert the [`CommandDataOption`] to a string.
pub fn command_data_option_to_string(command_data_option: &CommandDataOption) -> String {
    format!(
        "{}{}{}{}{}",
        command_data_option.name,
        command_data_option.value.as_ref().map_or("", |_| " "),
        command_data_option
            .value
            .as_ref()
            .map_or(Cow::Borrowed(""), |value| Cow::Owned(value.to_string())),
        if command_data_option.options.is_empty() {
            ""
        } else {
            " "
        },
        command_data_option
            .options
            .iter()
            .map(|command_data_option| command_data_option_to_string(command_data_option))
            .collect::<Vec<_>>()
            .join(" ")
    )
}
