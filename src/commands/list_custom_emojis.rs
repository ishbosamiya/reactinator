//! Command to list custom emojis of the server.

use serenity::{
    async_trait,
    builder::{CreateCommand, CreateInteractionResponse},
    model::application::CommandInteraction,
};

use crate::BotContext;

use super::Command;

/// `list_custom_emojis` command.
pub struct ListCustomEmojis;

#[async_trait]
impl Command for ListCustomEmojis {
    fn register(command: &mut CreateCommand, _bot_context: &BotContext) -> Self {
        command
            .name("list_custom_emojis")
            .description("List the custom emojis of the server.");
        Self
    }

    async fn interaction(
        &mut self,
        command_interaction: &CommandInteraction,
        context: &serenity::prelude::Context,
        bot_context: &BotContext,
    ) {
        let guild_emojis = bot_context.guild_emojis.read().await;
        let emojis = command_interaction
            .guild_id
            .as_ref()
            .and_then(|guild_id| guild_emojis.get(guild_id))
            .map(|guild_emojis| {
                let lines = guild_emojis
                    .iter()
                    .map(|(emoji_name, emoji)| format!("{} - `:{}:`", emoji, emoji_name))
                    .collect::<Vec<_>>();

                let mut joined_lines = vec![String::new()];

                lines.into_iter().for_each(|line| {
                    let last_joined_line = joined_lines.last_mut().unwrap();
                    if last_joined_line.len() + line.len() < 2000 {
                        last_joined_line.push('\n');
                        last_joined_line.push_str(&line);
                    } else {
                        joined_lines.push(line);
                    }
                });

                joined_lines
            });

        match emojis {
            Some(emojis) => {
                for emojis in emojis {
                    tracing::info!(
                        target: "list_custom_emojis",
                        "creating interaction response for `{}` with `{}`",
                        command_interaction.user.tag(),
                        emojis
                    );

                    if let Err(err) = command_interaction
                        .create_response(&context.http, |response| {
                            response
                                .kind(CreateInteractionResponse::Message)
                                .interaction_response_data(|message| {
                                    message.content(emojis).ephemeral(true)
                                })
                        })
                        .await
                    {
                        tracing::error!(
                            target: "list_custom_emojis",
                            "couldn't respond to `list_custom_emojis` for user `{}` due to `{}`",
                            command_interaction.user.tag(),
                            err
                        );
                    }
                }
            }
            None => {
                let response_content = "No custom emojis";

                tracing::info!(
                    target: "list_custom_emojis",
                    "creating interaction response for `{}` with `{}`",
                    command_interaction.user.tag(),
                    response_content,
                );

                if let Err(err) = command_interaction
                    .create_response(&context.http, |response| {
                        response
                            .kind(CreateInteractionResponse::Message)
                            .interaction_response_data(|message| {
                                message.content(response_content).ephemeral(true)
                            })
                    })
                    .await
                {
                    tracing::error!(
                        target: "list_custom_emojis",
                        "couldn't respond to `list_custom_emojis` for user `{}` due to `{}`",
                        command_interaction.user.tag(),
                        err
                    );
                }
            }
        }
    }
}
