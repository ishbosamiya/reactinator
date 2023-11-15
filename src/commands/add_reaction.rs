//! Add a reaction to the given message or previous message.

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    json::Value,
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
        bot_context: &BotContext,
    ) {
        let mut add_reaction_err = None;

        let emoji_name =
            match command_interaction.data.options.iter().find_map(|option| {
                (option.name == OPTION_EMOJI_NAME).then_some(option.value.as_ref())
            }) {
                Some(Some(emoji_name)) => match emoji_name.as_str() {
                    Some(emoji_name) => Some(emoji_name),
                    None => {
                        add_reaction_err = Some(Error::EmojiNameMustBeString(emoji_name.clone()));
                        None
                    }
                },
                _ => {
                    add_reaction_err = Some(Error::RequiresEmojiName);
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
                            add_reaction_err =
                                Some(Error::InvalidMessageId(message_id.to_string()));
                            None
                        }
                    },
                    None => {
                        add_reaction_err = Some(Error::MessageIdMustBeString(message_id.clone()));
                        None
                    }
                },
                _ => None,
            };

        if let Some(emoji_name) = emoji_name {
            let message_id = match message_id {
                Some(message_id) => Some(message_id),
                None => bot_context
                    .last_message_ids
                    .read()
                    .await
                    .get(&command_interaction.channel_id)
                    .map(|message_id| message_id.0),
            };
            match message_id {
                Some(message_id) => {
                    if let Err(err) = context
                        .http
                        .create_reaction(
                            command_interaction.channel_id.0,
                            message_id,
                            &ReactionType::Unicode(emoji_name.to_string()),
                        )
                        .await
                    {
                        add_reaction_err = Some(Error::CouldNotReactToMessage(err));
                    }
                }
                None => {
                    add_reaction_err = Some(Error::NoLastMessageAvailableAndNoMessageIdProvided);
                }
            }
        }

        if let Some(err) = add_reaction_err {
            tracing::error!(
                target: "add_reaction",
                "{}", err
            );

            if let Err(err) = command_interaction
                .create_interaction_response(&context.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .content(format!("error in `add_reaction`: {}", err))
                                .ephemeral(true)
                        })
                })
                .await
            {
                tracing::error!("couldn't respond to slash command due to `{}`", err);
            }
        }
    }
}

/// `add_reaction` related errors.
#[derive(Debug)]
pub enum Error {
    RequiresEmojiName,
    EmojiNameMustBeString(Value),
    MessageIdMustBeString(Value),
    InvalidMessageId(String),
    CouldNotReactToMessage(serenity::Error),
    NoLastMessageAvailableAndNoMessageIdProvided,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "add_reaction: ")?;
        match self {
            Error::RequiresEmojiName => write!(f, "requires emoji name"),
            Error::EmojiNameMustBeString(value) => {
                write!(f, "emoji name must be a string, got `{}`", value)
            }
            Error::MessageIdMustBeString(value) => {
                write!(f, "message id must be a string, got `{}`", value)
            }
            Error::InvalidMessageId(value) => write!(f, "invalid message id, got `{}`", value),
            Error::CouldNotReactToMessage(err) => write!(f, "could not react to message: {}", err),
            Error::NoLastMessageAvailableAndNoMessageIdProvided => {
                write!(f, "no last message available and no message id provided")
            }
        }
    }
}

impl std::error::Error for Error {}
