//! Add a reaction to the given message or previous message.

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    json::Value,
    model::{
        application::interaction::InteractionResponseType,
        prelude::{
            application_command::ApplicationCommandInteraction, command::CommandOptionType,
            MessageId, ReactionConversionError, ReactionType,
        },
    },
};

use crate::{context::BotAddedEmoji, BotContext};

use super::Command;

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
            None => bot_context
                .last_message_ids
                .read()
                .await
                .get(&command_interaction.channel_id)
                .copied(),
        };

        if let Some(emojis) = emojis {
            match message_id {
                Some(message_id) => {
                    for emoji in emojis
                        .split_whitespace()
                        .map(|emoji| emoji.trim())
                        .filter(|emoji| !emoji.is_empty())
                    {
                        match ReactionType::try_from(emoji) {
                            Ok(reaction_type) => {
                                match context
                                    .http
                                    .create_reaction(
                                        command_interaction.channel_id.0,
                                        message_id.0,
                                        &reaction_type,
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        if let Some(guild_id) = command_interaction.guild_id {
                                            bot_context
                                                .bot_added_emojis
                                                .write()
                                                .await
                                                .entry(guild_id)
                                                .or_insert_with(Vec::new)
                                                .push(BotAddedEmoji {
                                                    channel_id: command_interaction.channel_id,
                                                    message_id,
                                                    user_id: command_interaction.user.id,
                                                    reaction_type,
                                                });
                                        }
                                    }
                                    Err(err) => {
                                        add_reaction_err = Some(Error::CouldNotReactToMessage(err));
                                    }
                                }
                            }
                            Err(err) => {
                                add_reaction_err = Some(Error::InvalidEmoji(err));
                            }
                        }
                    }
                }
                None => {
                    add_reaction_err = Some(Error::NoLastMessageAvailableAndNoMessageIdProvided);
                }
            }
        }

        if let Some(add_reaction_err) = &add_reaction_err {
            tracing::error!(
                target: "add_reaction",
                "{}", add_reaction_err
            );
        }

        if let Err(err) = command_interaction
            .create_interaction_response(&context.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message
                            .content(if let Some(err) = add_reaction_err {
                                format!("error: {}", err)
                            } else {
                                format!(
                                    "added `{}` reaction to `{}`",
                                    emojis.unwrap(),
                                    message_id.unwrap()
                                )
                            })
                            .ephemeral(true)
                    })
            })
            .await
        {
            tracing::error!(
                "couldn't respond error message to slash command due to `{}`",
                err
            );
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
        }
    }
}

impl std::error::Error for Error {}
