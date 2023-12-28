//! Commands.

pub mod add_reaction;
pub mod list_custom_emojis;
pub mod ping;
pub mod text_to_reactions;

use std::{borrow::Cow, collections::HashSet, sync::Arc};

use serenity::{async_trait, builder::CreateCommand, model::prelude::*, prelude::*};

use crate::{context::BotAddedReactions, BotContext};

/// React to the given message ID with the given emoji text.
///
/// The emoji text must be separated by whitespace.
pub async fn react_to_message_with(
    message_id: MessageId,
    emoji_text: &str,
    command_interaction: &CommandInteraction,
    context: &Context,
    bot_context: &BotContext,
) -> Result<(), ReactToMessageWithError> {
    const REACTION_TIMEOUT_TIME_IN_SECONDS: u64 = 10;

    let mut react_to_message_with_err = None;

    let mut reaction_types = HashSet::new();
    for emoji in emoji_text
        .split_whitespace()
        .map(|emoji| emoji.trim())//! Commands.

pub mod add_reaction;
pub mod list_custom_emojis;
pub mod ping;
pub mod text_to_reactions;

use std::{borrow::Cow, collections::HashSet, sync::Arc};

use serenity::{async_trait, builder::CreateCommand, model::prelude::*, prelude::*};

use crate::{context::BotAddedReactions, BotContext};

/// React to the given message ID with the given emoji text.
///
/// The emoji text must be separated by whitespace.
pub async fn react_to_message_with(
    message_id: MessageId,
    emoji_text: &str,
    command_interaction: &CommandInteraction,
    context: &Context,
    bot_context: &BotContext,
) -> Result<(), ReactToMessageWithError> {
    const REACTION_TIMEOUT_TIME_IN_SECONDS: u64 = 10;

    let mut react_to_message_with_err = None;

    let mut reaction_types = HashSet::new();
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

                        reaction_types.insert(reaction_type);
                    }
                    Err(err) => {
                        react_to_message_with_err =
                            Some(ReactToMessageWithError::CouldNotReactToMessage(err));
                    }
                }
            }
            Err(err) => {
                react_to_message_with_err = Some(ReactToMessageWithError::InvalidEmoji(err));
            }
        }
    }

    if !reaction_types.is_empty() {
        if let Some(guild_id) = command_interaction.guild_id {
            let bot_added_reactions = Arc::new(std::sync::RwLock::new(BotAddedReactions {
                channel_id: command_interaction.channel_id,
                message_id,
                user_id: command_interaction.user.id,
                reaction_types,
                creation_time: std::time::Instant::now(),
            }));
            bot_context
                .bot_added_reactions
                .write()
                .await
                .entry(guild_id)
                .or_insert_with(Vec::new)
                .push(bot_added_reactions.clone());

            let context_http = context.http.clone();
            let bot_id = context.cache.current_user_id();
            let user_tag = command_interaction.user.tag();
            let command_interaction_user = command_interaction.user.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(
                    REACTION_TIMEOUT_TIME_IN_SECONDS,
                ))
                .await;

                let bot_added_reactions = { bot_added_reactions.write().unwrap().clone() };
                if bot_added_reactions.reaction_types.is_empty() {
                    tracing::info!(
                        "user `{}` has reacted to all \
                         reactions for message `{}` in channel `{}`",
                        user_tag,
                        bot_added_reactions.message_id,
                        bot_added_reactions.channel_id
                    );
                } else {
                    tracing::info!(
                        "attempting to remove reactions because \
                         user `{}` didn't interact with them",
                        user_tag,
                    );
                    for reaction_type in &bot_added_reactions.reaction_types {
                        match context_http
                            .delete_reaction(
                                bot_added_reactions.channel_id.0,
                                bot_added_reactions.message_id.0,
                                Some(bot_id.0),
                                reaction_type,
                            )
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "deleted `{}` reaction from message\
                                     `{}` in channel `{}` succesfully \
                                     because user `{}` didn't react",
                                    reaction_type,
                                    bot_added_reactions.message_id,
                                    bot_added_reactions.channel_id,
                                    user_tag,
                                );
                            }
                            Err(err) => {
                                tracing::error!(
                                    "couldn't delete `{}` reaction from \
                                     message `{}` in channel `{}` because \
                                     user `{}` didn't react due to `{}",
                                    reaction_type,
                                    bot_added_reactions.message_id,
                                    bot_added_reactions.channel_id,
                                    user_tag,
                                    err,
                                );
                            }
                        }
                    }
                    match command_interaction_user
                        .direct_message(&context_http, |create_message| {
                            create_message.content(format!(
                                "Removed reactions \"{}\" for message `{}` since \
                                 you did **not** react within {} seconds.",
                                bot_added_reactions
                                    .reaction_types
                                    .iter()
                                    .map(|reaction| reaction.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                bot_added_reactions.message_id,
                                REACTION_TIMEOUT_TIME_IN_SECONDS,
                            ))
                        })
                        .await
                    {
                        Ok(_) => {
                            tracing::info!(
                                "informed `{}` about deleting the \
                                 reactions from message `{}` in channel `{}`",
                                user_tag,
                                bot_added_reactions.message_id,
                                bot_added_reactions.channel_id
                            );
                        }
                        Err(err) => {
                            tracing::error!(
                                "couldn't inform `{}` about deleting \
                                 the reactions from message `{}` \
                                 in channel `{}` due to `{}`",
                                user_tag,
                                bot_added_reactions.message_id,
                                bot_added_reactions.channel_id,
                                err
                            );
                        }
                    }
                }
            })
            .await
            .unwrap();
        }
    }

    if let Some(err) = react_to_message_with_err {
        Err(err)
    } else {
        Ok(())
    }
}

/// [`react_to_message_with()`] errors.
#[derive(Debug)]
pub enum ReactToMessageWithError {
    CouldNotReactToMessage(serenity::Error),
    InvalidEmoji(ReactionConversionError),
}

impl std::fmt::Display for ReactToMessageWithError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "react_to_message_with: ")?;
        match self {
            Self::InvalidEmoji(err) => write!(f, "invalid emoji: `{}`", err),
            Self::CouldNotReactToMessage(err) => write!(f, "could not react to message: {}", err),
        }
    }
}

impl std::error::Error for ReactToMessageWithError {}

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
        command_interaction: &CommandInteraction,
        context: &Context,
        bot_context: &BotContext,
    );
}

/// Convert the [`CommandInteraction`] to a string.
pub fn command_interaction_to_string(command_interaction: &CommandInteraction) -> String {
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

                        reaction_types.insert(reaction_type);
                    }
                    Err(err) => {
                        react_to_message_with_err =
                            Some(ReactToMessageWithError::CouldNotReactToMessage(err));
                    }
                }
            }
            Err(err) => {
                react_to_message_with_err = Some(ReactToMessageWithError::InvalidEmoji(err));
            }
        }
    }

    if !reaction_types.is_empty() {
        if let Some(guild_id) = command_interaction.guild_id {
            let bot_added_reactions = Arc::new(std::sync::RwLock::new(BotAddedReactions {
                channel_id: command_interaction.channel_id,
                message_id,
                user_id: command_interaction.user.id,
                reaction_types,
                creation_time: std::time::Instant::now(),
            }));
            bot_context
                .bot_added_reactions
                .write()
                .await
                .entry(guild_id)
                .or_insert_with(Vec::new)
                .push(bot_added_reactions.clone());

            let context_http = context.http.clone();
            let bot_id = context.cache.current_user_id();
            let user_tag = command_interaction.user.tag();
            let command_interaction_user = command_interaction.user.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(
                    REACTION_TIMEOUT_TIME_IN_SECONDS,
                ))
                .await;

                let bot_added_reactions = { bot_added_reactions.write().unwrap().clone() };
                if bot_added_reactions.reaction_types.is_empty() {
                    tracing::info!(
                        "user `{}` has reacted to all \
                         reactions for message `{}` in channel `{}`",
                        user_tag,
                        bot_added_reactions.message_id,
                        bot_added_reactions.channel_id
                    );
                } else {
                    tracing::info!(
                        "attempting to remove reactions because \
                         user `{}` didn't interact with them",
                        user_tag,
                    );
                    for reaction_type in &bot_added_reactions.reaction_types {
                        match context_http
                            .delete_reaction(
                                bot_added_reactions.channel_id.0,
                                bot_added_reactions.message_id.0,
                                Some(bot_id.0),
                                reaction_type,
                            )
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "deleted `{}` reaction from message\
                                     `{}` in channel `{}` succesfully \
                                     because user `{}` didn't react",
                                    reaction_type,
                                    bot_added_reactions.message_id,
                                    bot_added_reactions.channel_id,
                                    user_tag,
                                );
                            }
                            Err(err) => {
                                tracing::error!(
                                    "couldn't delete `{}` reaction from \
                                     message `{}` in channel `{}` because \
                                     user `{}` didn't react due to `{}",
                                    reaction_type,
                                    bot_added_reactions.message_id,
                                    bot_added_reactions.channel_id,
                                    user_tag,
                                    err,
                                );
                            }
                        }
                    }
                    match command_interaction_user
                        .direct_message(&context_http, |create_message| {
                            create_message.content(format!(
                                "Removed reactions \"{}\" for message `{}` since \
                                 you did **not** react within {} seconds.",
                                bot_added_reactions
                                    .reaction_types
                                    .iter()
                                    .map(|reaction| reaction.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                bot_added_reactions.message_id,
                                REACTION_TIMEOUT_TIME_IN_SECONDS,
                            ))
                        })
                        .await
                    {
                        Ok(_) => {
                            tracing::info!(
                                "informed `{}` about deleting the \
                                 reactions from message `{}` in channel `{}`",
                                user_tag,
                                bot_added_reactions.message_id,
                                bot_added_reactions.channel_id
                            );
                        }
                        Err(err) => {
                            tracing::error!(
                                "couldn't inform `{}` about deleting \
                                 the reactions from message `{}` \
                                 in channel `{}` due to `{}`",
                                user_tag,
                                bot_added_reactions.message_id,
                                bot_added_reactions.channel_id,
                                err
                            );
                        }
                    }
                }
            })
            .await
            .unwrap();
        }
    }

    if let Some(err) = react_to_message_with_err {
        Err(err)
    } else {
        Ok(())
    }
}

/// [`react_to_message_with()`] errors.
#[derive(Debug)]
pub enum ReactToMessageWithError {
    CouldNotReactToMessage(serenity::Error),
    InvalidEmoji(ReactionConversionError),
}

impl std::fmt::Display for ReactToMessageWithError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "react_to_message_with: ")?;
        match self {
            Self::InvalidEmoji(err) => write!(f, "invalid emoji: `{}`", err),
            Self::CouldNotReactToMessage(err) => write!(f, "could not react to message: {}", err),
        }
    }
}

impl std::error::Error for ReactToMessageWithError {}

/// Discord command.
#[async_trait]
pub trait Command: Send + Sync + 'static {
    /// Register the command.
    fn register(command: &mut CreateCommand, bot_context: &BotContext) -> Self
    where
        Self: Sized;

    /// Interaction with the command.
    async fn interaction(
        &mut self,
        command_interaction: &CommandInteraction,
        context: &Context,
        bot_context: &BotContext,
    );
}

/// Convert the [`CommandInteraction`] to a string.
pub fn command_interaction_to_string(command_interaction: &CommandInteraction) -> String {
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
