//! Reactinator - Helper bot to react with any emoji.

pub mod commands;
pub mod context;

pub use context::BotContext;

use std::collections::HashMap;
use std::sync::Arc;

use commands::Command;
use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction, prelude::*,
    },
    prelude::*,
};

/// Event handler.
pub struct Handler {
    /// [`GuildCommands`].
    guild_commands: Arc<RwLock<HashMap<GuildId, GuildCommands>>>,

    /// [`BotContext`].
    bot_context: BotContext,
}

impl Handler {
    /// Create a new [`Handler`].
    pub fn new() -> Self {
        Self {
            guild_commands: Arc::new(RwLock::new(HashMap::new())),
            bot_context: BotContext::new(),
        }
    }
}

/// Commands of the guild.
pub struct GuildCommands(HashMap<String, Box<dyn Command>>);

impl GuildCommands {
    /// Create a new set of [`GuildCommands`].
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Insert a new command.
    pub fn insert(&mut self, command_creation: &CreateApplicationCommand, command: impl Command) {
        self.0.insert(
            command_creation
                .0
                .get("name")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            Box::new(command),
        );
    }

    /// Interaction with the commands.
    pub async fn interaction(
        &mut self,
        command_interaction: &ApplicationCommandInteraction,
        context: &Context,
        bot_context: &BotContext,
    ) {
        match self.0.get_mut(&command_interaction.data.name) {
            Some(command) => {
                command
                    .interaction(command_interaction, context, bot_context)
                    .await;
            }
            None => {
                tracing::error!("unknown command {}", command_interaction.data.name);
            }
        }
    }
}

impl Default for GuildCommands {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _context: Context, message: Message) {
        self.bot_context
            .last_message_ids
            .write()
            .await
            .insert(message.channel_id, message.id);
    }

    async fn interaction_create(&self, context: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command_interaction) = interaction {
            tracing::info!("command interaction: {:#?}", command_interaction);

            match &command_interaction.guild_id {
                Some(guild_id) => {
                    let mut guild_commands = self.guild_commands.write().await;
                    match guild_commands.get_mut(guild_id) {
                        Some(guild_commands) => {
                            guild_commands
                                .interaction(&command_interaction, &context, &self.bot_context)
                                .await
                        }
                        None => {
                            tracing::error!("commands not built for guild id {}", guild_id);
                        }
                    }
                }
                None => {
                    tracing::error!("expected guild id");
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("{} connected", ready.user.name);

        for guild_id in ctx.cache.guilds().into_iter() {
            let mut guild_commands = self.guild_commands.write().await;
            let commands = guild_id
                .set_application_commands(&ctx.http, |commands| {
                    let guild_commands = guild_commands
                        .entry(guild_id)
                        .or_insert_with(GuildCommands::default);

                    fn register_command<'a, C: Command>(
                        create_application_command: &'a mut CreateApplicationCommand,
                        guild_commands: &mut GuildCommands,
                        bot_context: &BotContext,
                    ) -> &'a mut CreateApplicationCommand {
                        let command = C::register(create_application_command, bot_context);
                        guild_commands.insert(&create_application_command, command);
                        create_application_command
                    }

                    commands
                        .create_application_command(|create_application_command| {
                            register_command::<commands::ping::Ping>(
                                create_application_command,
                                guild_commands,
                                &self.bot_context,
                            )
                        })
                        .create_application_command(|create_application_command| {
                            register_command::<commands::add_reaction::AddReaction>(
                                create_application_command,
                                guild_commands,
                                &self.bot_context,
                            )
                        })
                })
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
