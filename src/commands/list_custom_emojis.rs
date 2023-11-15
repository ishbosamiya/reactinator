//! Command to list custom emojis of the server.

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::InteractionResponseType,
    },
};

use crate::BotContext;

use super::Command;

/// `list_custom_emojis` command.
pub struct ListCustomEmojis;

#[async_trait]
impl Command for ListCustomEmojis {
    fn register(command: &mut CreateApplicationCommand, _bot_context: &BotContext) -> Self {
        command
            .name("list_custom_emojis")
            .description("List the custom emojis of the server.");
        Self
    }

    async fn interaction(
        &mut self,
        command_interaction: &ApplicationCommandInteraction,
        context: &serenity::prelude::Context,
        bot_context: &BotContext,
    ) {
        let guild_emojis = bot_context.guild_emojis.read().await;
        if let Err(err) = command_interaction
            .create_interaction_response(&context.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        let emojis = command_interaction
                            .guild_id
                            .as_ref()
                            .and_then(|guild_id| guild_emojis.get(guild_id))
                            .map(|guild_emojis| {
                                guild_emojis
                                    .iter()
                                    .map(|(emoji_name, emoji)| {
                                        format!("{} : {}", emoji_name, emoji)
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            });
                        message
                            .content(emojis.unwrap_or_else(|| "No custom emojis".to_string()))
                            .ephemeral(true)
                    })
            })
            .await
        {
            tracing::error!("couldn't respond to `list_custom_emojis` due to `{}`", err);
        }
    }
}
