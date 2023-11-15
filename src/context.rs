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
}

impl BotContext {
    /// Create a new [`BotContext`].
    pub fn new() -> Self {
        Self {
            last_message_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for BotContext {
    fn default() -> Self {
        Self::new()
    }
}
