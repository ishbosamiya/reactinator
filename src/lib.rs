//! Reactinator - Helper bot to react with any emoji.

use serenity::{async_trait, model::prelude::*, prelude::*};

/// Event handler.
pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            tracing::info!("command interaction: {:#?}", command);

            let content = match command.data.name.as_str() {
                _ => Some("unimplemented_command".to_string()),
            };

            if let Some(content) = content {
                if let Err(err) = command
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| message.content(content))
                    })
                    .await
                {
                    tracing::error!("couldn't respond to slash command due to `{}`", err);
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("{} connected", ready.user.name);

        for guild_id in ctx.cache.guilds().iter() {
            let commands = guild_id
                .set_application_commands(&ctx.http, |commands| commands)
                .await;

            match commands {
                Ok(commands) => {
                    tracing::info!("guild `{}` has the commands {:#?}", guild_id, commands);
                }
                Err(err) => {
                    tracing::error!("couldn't create commands due to `{}`", err);
                }
            }
        }
    }
}
