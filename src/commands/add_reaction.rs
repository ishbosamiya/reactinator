//! Add a reaction to the given message or previous message.

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    json::Value,
    model::{
        application::interaction::InteractionResponseType,
        prelude::{
            application_command::ApplicationCommandInteraction, command::CommandOptionType,
            MessageId, ReactionConversionError,
        },
    },
};

use crate::BotContext;

use super::{react_to_message_with, Command, ReactToMessageWithError};

/// `add_reaction` command.
pub struct AddReaction;

/// Option `emoji`.
const OPTION_EMOJI: &str = "emoji";

/// Option `message_id`.
const OPTION_MESSAGE_ID: &str = "message_id";

#[async_trait]
impl Command for AddReaction {
    fn register(command: &mut CreateApplicationCommand, _bot_context: &BotContext) -> Self {
        command
            .name("add_reaction")
            .description("Add reaction(s) to the given message or last message on the channel.")
            .create_option(|command_option| {
                command_option
                    .required(true)
                    .kind(CommandOptionType::String)
                    .name(OPTION_EMOJI)
                    .description("Emoji to react with. Can use multiple space separated emojis.")
            })
            .create_option(|command_option| {
                command_option
                    .kind(CommandOptionType::String)
                    .name(OPTION_MESSAGE_ID)
                    .description("Message ID to react to. Defaults to last message on channel.")
            });
        Self
    }

    async fn interaction(
        &mut self,
        command_interaction: &ApplicationCommandInteraction,
        context: &serenity::prelude::Context,
        bot_context: &BotContext,
    ) {
        let mut add_reaction_err = None;

        let emojis = match command_interaction
            .data
            .options
            .iter()
            .find_map(|option| (option.name == OPTION_EMOJI).then_some(option.value.as_ref()))
        {
            Some(Some(emojis)) => match emojis.as_str() {
                Some(emojis) => Some(emojis),
                None => {
                    add_reaction_err = Some(Error::EmojiMustBeProvidedInString(emojis.clone()));
                    None
                }
            },
            _ => {
                add_reaction_err = Some(Error::RequiresEmoji);
                None
            }
        };

        let message_id =
            match command_interaction.data.options.iter().find_map(|option| {
                (option.name == OPTION_MESSAGE_ID).then_some(option.value.as_ref())
            }) {
                Some(Some(message_id)) => match message_id.as_str() {
                    Some(message_id) => match message_id.parse::<u64>().ok() {
                        Some(message_id) => Some(MessageId(message_id)),
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

        let message_id = match message_id {
            Some(message_id) => Some(message_id),
            None => match bot_context
                .last_message_ids
                .read()
                .await
                .get(&command_interaction.channel_id)
                .copied()
            {
                Some(message_id) => Some(message_id),
                None => {
                    add_reaction_err = Some(Error::NoLastMessageAvailableAndNoMessageIdProvided);
                    None
                }
            },
        };

        if let Err(err) = command_interaction
            .create_interaction_response(&context.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message
                            .content(if let Some(err) = add_reaction_err.take() {
                                format!("error: {}", err)
                            } else {
                                format!(
                                    "Don't forget to react to message `{}` \
                                     yourself for the reactions {}.",
                                    message_id.unwrap(),
                                    emojis.unwrap(),
                                )
                            })
                            .ephemeral(true)
                    })
            })
            .await
        {
            tracing::error!(
                "couldn't respond error message to slash command for user `{}` due to `{}`",
                command_interaction.user.tag(),
                err,
            );
        }

        if let (Some(emojis), Some(message_id)) = (emojis, message_id) {
            if let Err(err) = react_to_message_with(
                message_id,
                &emojis,
                command_interaction,
                context,
                bot_context,
            )
            .await
            {
                add_reaction_err = Some(err.into());
            }
        }

        if let Some(add_reaction_err) = &add_reaction_err {
            tracing::error!(
                target: "add_reaction",
                "user `{}` - {}",
                command_interaction.user.tag(),
                add_reaction_err
            );

            if let Err(err) = command_interaction
                .edit_original_interaction_response(&context.http, |response| {
                    response.content(format!("error: {}", add_reaction_err))
                })
                .await
            {
                tracing::error!(
                    "couldn't edit interaction response message to \
                     slash command for user `{}` due to `{}`",
                    command_interaction.user.tag(),
                    err,
                );
            }
        }
    }
}

/// `add_reaction` related errors.
#[derive(Debug)]
pub enum Error {
    RequiresEmoji,
    EmojiMustBeProvidedInString(Value),
    InvalidEmoji(ReactionConversionError),
    MessageIdMustBeString(Value),
    InvalidMessageId(String),
    CouldNotReactToMessage(serenity::Error),
    NoLastMessageAvailableAndNoMessageIdProvided,
    ReactToMessageWith(ReactToMessageWithError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "add_reaction: ")?;
        match self {
            Error::RequiresEmoji => write!(f, "requires emoji"),
            Error::EmojiMustBeProvidedInString(value) => {
                write!(f, "emoji must be provided in a string, got `{}`", value)
            }
            Error::InvalidEmoji(err) => {
                write!(f, "invalid emoji `{}`", err)
            }
            Error::MessageIdMustBeString(value) => {
                write!(f, "message id must be a string, got `{}`", value)
            }
            Error::InvalidMessageId(value) => write!(f, "invalid message id, got `{}`", value),
            Error::CouldNotReactToMessage(err) => write!(f, "could not react to message: {}", err),
            Error::NoLastMessageAvailableAndNoMessageIdProvided => {
                write!(f, "no last message available and no message id provided")
            }
            Error::ReactToMessageWith(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<ReactToMessageWithError> for Error {
    fn from(err: ReactToMessageWithError) -> Self {
        Self::ReactToMessageWith(err)
    }
}
