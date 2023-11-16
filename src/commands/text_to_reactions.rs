//! Add the given text as list of reactions to the given message or
//! previous message.

use std::{collections::HashMap, sync::Arc};

use lazy_static::lazy_static;
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

/// `text_to_reactions` command.
pub struct TextToReactions;

/// Option `text`.
const OPTION_TEXT: &str = "text";

/// Option `message_id`.
const OPTION_MESSAGE_ID: &str = "message_id";

#[async_trait]
impl Command for TextToReactions {
    fn register(command: &mut CreateApplicationCommand, _bot_context: &BotContext) -> Self {
        command
            .name("text_to_reactions")
            .description(
                "Text as list of reactions to the given \
                 message or last message on the channel.",
            )
            .create_option(|command_option| {
                command_option
                    .required(true)
                    .kind(CommandOptionType::String)
                    .name(OPTION_TEXT)
                    .description("Text to convert to reactions.")
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
        let mut text_to_reactions_err = None;

        let text = match command_interaction
            .data
            .options
            .iter()
            .find_map(|option| (option.name == OPTION_TEXT).then_some(option.value.as_ref()))
        {
            Some(Some(text)) => match text.as_str() {
                Some(text) => Some(text),
                None => {
                    text_to_reactions_err = Some(Error::TextMustBeProvidedInString(text.clone()));
                    None
                }
            },
            _ => {
                text_to_reactions_err = Some(Error::RequiresText);
                None
            }
        };

        let emoji_text = match text {
            Some(text) => match text_to_emojis(&text) {
                Some(emoji_text) => {
                    tracing::info!("converted `{}` to `{}`", text, emoji_text);
                    Some(emoji_text)
                }
                None => {
                    text_to_reactions_err = Some(Error::CouldNotConvertTextToEmojis);
                    None
                }
            },
            None => None,
        };

        let message_id =
            match command_interaction.data.options.iter().find_map(|option| {
                (option.name == OPTION_MESSAGE_ID).then_some(option.value.as_ref())
            }) {
                Some(Some(message_id)) => match message_id.as_str() {
                    Some(message_id) => match message_id.parse::<u64>().ok() {
                        Some(message_id) => Some(MessageId(message_id)),
                        None => {
                            text_to_reactions_err =
                                Some(Error::InvalidMessageId(message_id.to_string()));
                            None
                        }
                    },
                    None => {
                        text_to_reactions_err =
                            Some(Error::MessageIdMustBeString(message_id.clone()));
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

        if let Some(emoji_text) = &emoji_text {
            match message_id {
                Some(message_id) => {
                    for emoji in emoji_text
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
                                        tracing::info!(
                                            "added reaction `{}` to `{}`",
                                            reaction_type,
                                            message_id
                                        );

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
                                        text_to_reactions_err =
                                            Some(Error::CouldNotReactToMessage(err));
                                    }
                                }
                            }
                            Err(err) => {
                                text_to_reactions_err = Some(Error::InvalidEmoji(err));
                            }
                        }
                    }
                }
                None => {
                    text_to_reactions_err =
                        Some(Error::NoLastMessageAvailableAndNoMessageIdProvided);
                }
            }
        }

        if let Some(text_to_reactions_err) = &text_to_reactions_err {
            tracing::error!(
                target: "text_to_reactions",
                "{}", text_to_reactions_err
            );
        }

        if let Err(err) = command_interaction
            .create_interaction_response(&context.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message
                            .content(if let Some(err) = text_to_reactions_err {
                                format!("error: {}", err)
                            } else {
                                format!(
                                    "Don't forget to react to message `{}` \
                                     yourself for the reactions {}.",
                                    message_id.unwrap(),
                                    emoji_text.unwrap(),
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

/// Text to emoji compatible text.
pub fn text_to_emojis(text: &str) -> Option<String> {
    lazy_static! {
        /// Alternatives for the [`char`]s.
        pub static ref ALTERNATIVES: HashMap<char, &'static [char]> = [
            ('a', ['4'].as_slice()),
            ('b', &['8']),
            ('e', &['3']),
            ('g', &['9']),
            ('i', &['1', '!']),
            ('l', &['1']),
            ('o', &['0']),
            ('s', &['5', '$', 'z']),
            ('t', &['7']),
            ('u', &['v']),
            ('z', &['s']),
        ].into_iter().collect();

        /// Allowed frequency for the [`char`]s.
        pub static ref ALLOWED_FREQUENCY: HashMap<char, usize> = [
            ('a', 2),
            ('b', 2),
            ('i', 2)
        ].into_iter().collect();

        /// [`char`] to `emoji`.
        pub static ref CHAR_TO_EMOJI: HashMap<char, Arc<[String]>> = [
            ('0', "zero"),
            ('1', "one"),
            ('2', "two"),
            ('3', "three"),
            ('4', "four"),
            ('5', "five"),
            ('6', "six"),
            ('7', "seven"),
            ('8', "eight"),
            ('9', "nine"),
        ].into_iter()
            .map(|(key, emoji)| (key, [format!(":{}:", emoji)]))
            .map(|(key, emoji)| (key, Arc::from_iter(emoji)))
            .chain(('a'..='z').map(|ch| {
                let regional_indicator = format!(":regional_indicator_{}:", ch);
                match ch {
                    'a' | 'b' => (ch, Arc::from_iter([regional_indicator, format!(":{}:", ch)])),
                    'i' => (ch, Arc::from_iter([regional_indicator, ":information_source:".to_string()])),
                    _ => (ch, Arc::from_iter([regional_indicator]))
                }
            }))
            .collect();
    }

    let mut used_characters = HashMap::new();
    text.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            if used_characters.contains_key(&c) {
                if used_characters.get(&c).unwrap() < ALLOWED_FREQUENCY.get(&c).unwrap_or(&1) {
                    *used_characters.get_mut(&c).unwrap() += 1;
                    Some(c)
                } else {
                    let alternative =
                        *ALTERNATIVES
                            .get(&c)?
                            .iter()
                            .find(|c| match used_characters.get(*c) {
                                Some(num_allowed) => {
                                    num_allowed < ALLOWED_FREQUENCY.get(c).unwrap_or(&1)
                                }
                                None => true,
                            })?;

                    *used_characters.entry(alternative).or_insert(0) += 1;

                    Some(alternative)
                }
            } else {
                used_characters.insert(c, 1);
                Some(c)
            }
        })
        .collect()
}

/// `text_to_reactions` related errors.
#[derive(Debug)]
pub enum Error {
    RequiresText,
    TextMustBeProvidedInString(Value),
    InvalidEmoji(ReactionConversionError),
    MessageIdMustBeString(Value),
    InvalidMessageId(String),
    CouldNotReactToMessage(serenity::Error),
    NoLastMessageAvailableAndNoMessageIdProvided,
    CouldNotConvertTextToEmojis,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "text_to_reactions: ")?;
        match self {
            Error::RequiresText => write!(f, "requires text"),
            Error::TextMustBeProvidedInString(value) => {
                write!(f, "text must be provided in a string, got `{}`", value)
            }
            Error::InvalidEmoji(err) => write!(f, "invalid emoji: `{}`", err),
            Error::MessageIdMustBeString(value) => {
                write!(f, "message id must be a string, got `{}`", value)
            }
            Error::InvalidMessageId(value) => write!(f, "invalid message id, got `{}`", value),
            Error::CouldNotReactToMessage(err) => write!(f, "could not react to message: {}", err),
            Error::NoLastMessageAvailableAndNoMessageIdProvided => {
                write!(f, "no last message available and no message id provided")
            }
            Error::CouldNotConvertTextToEmojis => write!(f, "could not convert text to emojis"),
        }
    }
}

impl std::error::Error for Error {}
