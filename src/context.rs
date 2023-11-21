//! Bot's context.

use std::collections::HashMap;
use std::sync::Arc;

use serenity::model::prelude::*;
use tokio::sync::RwLock;

/// Bot's context.
#[derive(Debug, Clone)]
pub struct BotContext {
    /// Last sent message's [`MessageId`] for the [`ChannelId`].
    pub last_message_ids: Arc<RwLock<HashMap<ChannelId, MessageId>>>,

    /// [`GuildId`] to emoji name to [`Emoji`] mapping.
    pub guild_emojis: Arc<RwLock<HashMap<GuildId, HashMap<String, Emoji>>>>,

    /// Bot added emojis. Mapping from [`GuildId`] to the
    /// [`BotAddedEmoji`]s for that guild.
    pub bot_added_emojis: Arc<RwLock<HashMap<GuildId, Vec<BotAddedEmoji>>>>,
}

impl BotContext {
    /// Create a new [`BotContext`].
    pub fn new() -> Self {
        Self {
            last_message_ids: Arc::new(RwLock::new(HashMap::new())),
            guild_emojis: Arc::new(RwLock::new(HashMap::new())),
            bot_added_emojis: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for BotContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Bot added emoji.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BotAddedEmoji {
    /// [`ChannelId`].
    pub channel_id: ChannelId,
    /// [`MessageId`].
    pub message_id: MessageId,
    /// [`UserId`].
    pub user_id: UserId,
    /// [`ReactionType`].
    pub reaction_type: ReactionType,

    /// Creation time of the emoji.
    pub creation_time: std::time::Instant,
}
