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
                    tracing::info!(
                        "converted `{}` to `{}` for user `{}`",
                        text,
                        emoji_text,
                        command_interaction.user.tag()
                    );
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
            None => match bot_context
                .last_message_ids
                .read()
                .await
                .get(&command_interaction.channel_id)
                .copied()
            {
                Some(message_id) => Some(message_id),
                None => {
                    text_to_reactions_err =
                        Some(Error::NoLastMessageAvailableAndNoMessageIdProvided);
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
                            .content(if let Some(err) = text_to_reactions_err.take() {
                                format!("error: {}", err)
                            } else {
                                format!(
                                    "Don't forget to react to message `{}` \
                                     yourself for the reactions {}.",
                                    message_id.unwrap(),
                                    emoji_text.as_ref().unwrap(),
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
                err
            );
        }

        if let (Some(emoji_text), Some(message_id)) = (emoji_text, message_id) {
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
                                    "added reaction `{}` to `{}` for user `{}`",
                                    reaction_type,
                                    message_id,
                                    command_interaction.user.tag(),
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
                                            creation_time: std::time::Instant::now(),
                                        });
                                }
                            }
                            Err(err) => {
                                text_to_reactions_err = Some(Error::CouldNotReactToMessage(err));
                            }
                        }
                    }
                    Err(err) => {
                        text_to_reactions_err = Some(Error::InvalidEmoji(err));
                    }
                }
            }
        }

        if let Some(text_to_reactions_err) = &text_to_reactions_err {
            tracing::error!(
                target: "text_to_reactions",
                "user `{}` - {}",
                command_interaction.user.tag(),
                text_to_reactions_err,
            );

            if let Err(err) = command_interaction
                .edit_original_interaction_response(&context.http, |response| {
                    response.content(format!("error: {}", text_to_reactions_err))
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

lazy_static! {
    /// [`text_to_emojis()`]: alternatives for the [`char`]s.
    pub static ref TEXT_TO_EMOJIS_ALTERNATIVES: HashMap<char, &'static [char]> = [
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

    /// [`text_to_emojis()`]: [`char`] to `emoji_name`.
    pub static ref TEXT_TO_EMOJIS_CHAR_TO_EMOJI_NAME: HashMap<char, Arc<[String]>> = [
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
                'a' | 'b' | 'm' => (ch, Arc::from_iter([regional_indicator, format!(":{}:", ch)])),
                'o' => (ch, Arc::from_iter([regional_indicator, ":o:".to_string(), ":o2:".to_string()])),
                'p' => (ch, Arc::from_iter([regional_indicator, ":parking:".to_string()])),
                'i' => (ch, Arc::from_iter([regional_indicator, ":information_source:".to_string()])),
                _ => (ch, Arc::from_iter([regional_indicator]))
            }
        }))
        .collect();

    /// [`text_to_emojis()`]: `emoji_name` to [`char`] emoji.
    pub static ref TEXT_TO_EMOJIS_EMOJI_NAME_TO_EMOJI: HashMap<&'static str, &'static str> = {
        [
            (":regional_indicator_a:", "🇦"),
            (":regional_indicator_b:", "🇧"),
            (":regional_indicator_c:", "🇨"),
            (":regional_indicator_d:", "🇩"),
            (":regional_indicator_e:", "🇪"),
            (":regional_indicator_f:", "🇫"),
            (":regional_indicator_g:", "🇬"),
            (":regional_indicator_h:", "🇭"),
            (":regional_indicator_i:", "🇮"),
            (":regional_indicator_j:", "🇯"),
            (":regional_indicator_k:", "🇰"),
            (":regional_indicator_l:", "🇱"),
            (":regional_indicator_m:", "🇲"),
            (":regional_indicator_n:", "🇳"),
            (":regional_indicator_o:", "🇴"),
            (":regional_indicator_p:", "🇵"),
            (":regional_indicator_q:", "🇶"),
            (":regional_indicator_r:", "🇷"),
            (":regional_indicator_s:", "🇸"),
            (":regional_indicator_t:", "🇹"),
            (":regional_indicator_u:", "🇺"),
            (":regional_indicator_v:", "🇻"),
            (":regional_indicator_w:", "🇼"),
            (":regional_indicator_x:", "🇽"),
            (":regional_indicator_y:", "🇾"),
            (":regional_indicator_z:", "🇿"),
            (":zero:", "0️⃣"),
            (":one:", "1️⃣"),
            (":two:", "2️⃣"),
            (":three:", "3️⃣"),
            (":four:", "4️⃣"),
            (":five:", "5️⃣"),
            (":six:", "6️⃣"),
            (":seven:", "7️⃣"),
            (":eight:", "8️⃣"),
            (":nine:", "9️⃣"),
            (":keycap_ten:", "🔟"),
            (":information_source:", "ℹ️"),
            (":a:", "🅰️"),
            (":b:", "🅱️"),
            (":o2:", "🅾️"),
            (":o:", "⭕"),
            (":m:", "Ⓜ️"),
            (":parking:", "🅿️"),
        ].into_iter().collect()
    };
}

/// Text to emoji compatible text.
pub fn text_to_emojis(text: &str) -> Option<String> {
    let mut used_characters: HashMap<char, usize> = HashMap::new();
    Some(
        text.to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| {
                if used_characters.contains_key(&c) {
                    if *used_characters.get(&c).unwrap()
                        < TEXT_TO_EMOJIS_CHAR_TO_EMOJI_NAME.get(&c)?.len()
                    {
                        let emoji = TEXT_TO_EMOJIS_CHAR_TO_EMOJI_NAME.get(&c).unwrap()
                            [*used_characters.get(&c).unwrap()]
                        .as_str();

                        *used_characters.get_mut(&c).unwrap() += 1;

                        Some(emoji)
                    } else {
                        let alternative =
                            *TEXT_TO_EMOJIS_ALTERNATIVES.get(&c)?.iter().find(|c| {
                                match used_characters.get(*c) {
                                    Some(num_allowed) => match TEXT_TO_EMOJIS_CHAR_TO_EMOJI_NAME
                                        .get(&c)
                                    {
                                        Some(char_to_emoji) => *num_allowed < char_to_emoji.len(),
                                        None => false,
                                    },
                                    None => true,
                                }
                            })?;

                        let alternative_emoji = TEXT_TO_EMOJIS_CHAR_TO_EMOJI_NAME
                            .get(&alternative)
                            .unwrap()[*used_characters.entry(alternative).or_insert(0)]
                        .as_str();

                        *used_characters.get_mut(&alternative).unwrap() += 1;

                        Some(alternative_emoji)
                    }
                } else {
                    let alternative_emoji = TEXT_TO_EMOJIS_CHAR_TO_EMOJI_NAME.get(&c)?[0].as_str();
                    used_characters.insert(c, 1);
                    Some(alternative_emoji)
                }
            })
            .map(|emoji_name| {
                emoji_name
                    .map(|emoji_name| *TEXT_TO_EMOJIS_EMOJI_NAME_TO_EMOJI.get(emoji_name).unwrap())
            })
            .collect::<Option<Vec<_>>>()?
            .join(" "),
    )
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

#[cfg(test)]
mod tests {
    use crate::commands::text_to_reactions::TEXT_TO_EMOJIS_EMOJI_NAME_TO_EMOJI;

    use super::{text_to_emojis, TEXT_TO_EMOJIS_CHAR_TO_EMOJI_NAME};

    /// Basic test of alternatives.
    #[test]
    fn text_to_emojis_01() {
        assert_eq!(text_to_emojis("a").unwrap(), "🇦");
        assert_eq!(text_to_emojis("aa").unwrap(), "🇦 🅰️");
        assert_eq!(text_to_emojis("aaa").unwrap(), "🇦 🅰️ 4️⃣");
        assert_eq!(text_to_emojis("aaaa"), None);
    }

    /// Test all the characters, does not test the alternatives.
    #[test]
    fn text_to_emojis_02() {
        let mut char_to_emoji = TEXT_TO_EMOJIS_CHAR_TO_EMOJI_NAME
            .iter()
            .map(|(key, value)| (*key, value))
            .collect::<Vec<_>>();

        char_to_emoji.sort_by_key(|(key, _)| *key);

        let char_string = char_to_emoji
            .iter()
            .flat_map(|(ch, emojis)| vec![*ch; emojis.len()])
            .collect::<String>();
        let emoji_string = char_to_emoji
            .iter()
            .flat_map(|(_, emoji_names)| &***emoji_names)
            .map(|emoji_name| {
                *TEXT_TO_EMOJIS_EMOJI_NAME_TO_EMOJI
                    .get(emoji_name.as_str())
                    .unwrap()
            })
            .collect::<Vec<_>>()
            .join(" ");

        assert_eq!(text_to_emojis(&char_string).unwrap(), emoji_string);
    }
}
