//! Add a reaction to the given message or previous message.

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::{
        application::interaction::InteractionResponseType,
        prelude::{command::CommandOptionType, ReactionType},
    },
};

use crate::BotContext;

use super::Command;

/// `add_reaction` command.
pub struct AddReaction;

/// Option `emoji_name`.
const OPTION_EMOJI_NAME: &str = "emoji_name";

/// Option `message_id`.
const OPTION_MESSAGE_ID: &str = "message_id";

#[async_trait]
impl Command for AddReaction {
    fn register(command: &mut CreateApplicationCommand, _bot_context: &BotContext) -> Self {
        command
            .name("add_reaction")
            .description("Add a reaction to the given message or previous message.")
            .create_option(|command_option| {
                command_option
                    .required(true)
                    .kind(CommandOptionType::String)
                    .name(OPTION_EMOJI_NAME)
                    .description("Emoji's name.")
            })
            .create_option(|command_option| {
                command_option
                    .required(true)
                    .kind(CommandOptionType::String)
                    .name(OPTION_MESSAGE_ID)
                    .description("Message ID to react to.")
            });
        Self
    }

    async fn interaction(
        &mut self,
        command_interaction: &serenity::model::prelude::application_command::ApplicationCommandInteraction,
        context: &serenity::prelude::Context,
        _bot_context: &BotContext,
    ) {
        let emoji_name =
            match command_interaction.data.options.iter().find_map(|option| {
                (option.name == OPTION_EMOJI_NAME).then_some(option.value.as_ref())
            }) {
                Some(Some(emoji_name)) => match emoji_name.as_str() {
                    Some(emoji_name) => Some(emoji_name),
                    None => {
                        tracing::error!(
                            "expected string for {} of `add_reaction`, got `{}`",
                            OPTION_EMOJI_NAME,
                            emoji_name
                        );
                        None
                    }
                },
                _ => {
                    tracing::error!("required `{}` for `add_reaction`", OPTION_EMOJI_NAME);
                    None
                }
            };

        let message_id =
            match command_interaction.data.options.iter().find_map(|option| {
                (option.name == OPTION_MESSAGE_ID).then_some(option.value.as_ref())
            }) {
                Some(Some(message_id)) => match message_id.as_str() {
                    Some(message_id) => match message_id.parse::<u64>().ok() {
                        Some(message_id) => Some(message_id),
                        None => {
                            tracing::error!(
                                "not a valid message id for `{}` of `add_reaction`, got `{}`",
                                OPTION_MESSAGE_ID,
                                message_id
                            );
                            None
                        }
                    },
                    None => {
                        tracing::error!(
                            "expected string for {} of `add_reaction`, got `{}`",
                            OPTION_MESSAGE_ID,
                            message_id
                        );
                        None
                    }
                },
                _ => {
                    tracing::error!("required `{}` for `add_reaction`", OPTION_MESSAGE_ID);
                    None
                }
            };

        if let (Some(emoji_name), Some(message_id)) = (emoji_name, message_id) {
            if let Err(err) = context
                .http
                .create_reaction(
                    command_interaction.channel_id.0,
                    message_id,
                    &ReactionType::Unicode(emoji_name.to_string()),
                )
                .await
            {
                tracing::error!(
                    "couldn't react to message `{}` with `{}` due to `{}`",
                    message_id,
                    emoji_name,
                    err,
                );
            }
        }

        if let Err(err) = command_interaction
            .create_interaction_response(&context.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content(format!(
                            "add_reaction invoked with {}: {:?}, {}: {:?}",
                            OPTION_EMOJI_NAME, emoji_name, OPTION_MESSAGE_ID, message_id,
                        ))
                    })
            })
            .await
        {
            tracing::error!("couldn't respond to slash command due to `{}`", err);
        }
    }
}
