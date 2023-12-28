//! Reactinator - Helper bot to react with any emoji.

pub mod commands;
pub mod context;

pub use context::BotContext;

use std::collections::HashMap;
use std::sync::Arc;

use commands::Command;
use serenity::{
    async_trait, builder::CreateCommand, model::application::CommandInteraction, model::prelude::*,
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
    pub fn insert(&mut self, command_creation: &CreateCommand, command: impl Command) {
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
        command_interaction: &CommandInteraction,
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
    async fn message(&self, context: Context, message: Message) {
        if message.author.id != context.cache.current_user_id() {
            self.bot_context
                .last_message_ids
                .write()
                .await
                .insert(message.channel_id, message.id);
        }
    }

    async fn interaction_create(&self, context: Context, interaction: Interaction) {
        if let Interaction::Command(command_interaction) = interaction {
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
                .set_commands(&ctx.http, |commands| {
                    let guild_commands = guild_commands
                        .entry(guild_id)
                        .or_insert_with(GuildCommands::default);

                    fn register_command<'a, C: Command>(
                        create_application_command: &'a mut CreateCommand,
                        guild_commands: &mut GuildCommands,
                        bot_context: &BotContext,
                    ) -> &'a mut CreateCommand {
                        let command = C::register(create_application_command, bot_context);
                        guild_commands.insert(&create_application_command, command);
                        create_application_command
                    }

                    commands
                        // .create_application_command(|create_application_command| {
                        //     register_command::<commands::ping::Ping>(
                        //         create_application_command,
                        //         guild_commands,
                        //         &self.bot_context,
                        //     )
                        // })
                        .create_application_command(|create_application_command| {
                            register_command::<commands::add_reaction::AddReaction>(
                                create_application_command,
                                guild_commands,
                                &self.bot_context,
                            )
                        })
                        .create_application_command(|create_application_command| {
                            register_command::<commands::list_custom_emojis::ListCustomEmojis>(
                                create_application_command,
                                guild_commands,
                                &self.bot_context,
                            )
                        })
                        .create_application_command(|create_application_command| {
                            register_command::<commands::text_to_reactions::TextToReactions>(
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

            match guild_id.emojis(&ctx.http).await {
                Ok(emojis) => {
                    self.bot_context
                        .guild_emojis
                        .write()
                        .await
                        .entry(guild_id)
                        .or_insert_with(HashMap::new)
                        .extend(emojis.into_iter().map(|emoji| (emoji.name.clone(), emoji)));
                }
                Err(err) => {
                    tracing::error!(
                        "couldn't fetch emojis of the guild `{}` due to `{}`",
                        guild_id,
                        err
                    );
                }
            }
        }
    }

    async fn guild_emojis_update(
        &self,
        _context: Context,
        guild_id: GuildId,
        emojis: HashMap<EmojiId, Emoji>,
    ) {
        self.bot_context
            .guild_emojis
            .write()
            .await
            .entry(guild_id)
            .or_insert_with(HashMap::new)
            .extend(
                emojis
                    .into_iter()
                    .map(|(_, emoji)| (emoji.name.clone(), emoji)),
            );
    }

    async fn reaction_add(&self, context: Context, reaction: Reaction) {
        if let (Some(guild_id), Some(user_id)) = (&reaction.guild_id, reaction.user_id) {
            let guilds_to_bot_added_reactions = self.bot_context.bot_added_reactions.read().await;
            if let Some(bot_added_reactions) = guilds_to_bot_added_reactions.get(guild_id) {
                if let Some(bot_added_reactions_index) = bot_added_reactions
                    .iter()
                    .enumerate()
                    .find_map(|(bot_added_reactions_index, bot_added_reactions)| {
                        let bot_added_reactions = bot_added_reactions.read().unwrap();
                        (bot_added_reactions.channel_id == reaction.channel_id
                            && bot_added_reactions.message_id == reaction.message_id
                            && bot_added_reactions.user_id == user_id
                            && bot_added_reactions.reaction_types.contains(&reaction.emoji))
                        .then_some(bot_added_reactions_index)
                    })
                {
                    drop(guilds_to_bot_added_reactions);
                    let mut guilds_to_bot_added_reactions =
                        self.bot_context.bot_added_reactions.write().await;
                    let bot_added_reactions = guilds_to_bot_added_reactions
                        .get_mut(guild_id)
                        .unwrap()
                        .get_mut(bot_added_reactions_index)
                        .unwrap();
                    match reaction.message(&context.http).await {
                        Ok(message) => {
                            bot_added_reactions
                                .write()
                                .unwrap()
                                .reaction_types
                                .remove(&reaction.emoji);
                            match message
                                .delete_reaction(
                                    &context.http,
                                    Some(context.cache.current_user_id()),
                                    reaction.emoji.clone(),
                                )
                                .await
                            {
                                Ok(_) => {
                                    tracing::info!(
                                        "deleted reaction `{}` from `{}` since user reacted",
                                        reaction.emoji,
                                        reaction.message_id,
                                    );
                                }
                                Err(err) => {
                                    tracing::error!(
                                        "unable to delete reaction `{:?}` due to `{}`",
                                        reaction,
                                        err
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            tracing::error!(
                                "couldn't get message for reaction `{:?}` due to `{}`",
                                reaction,
                                err
                            );
                        }
                    }

                    if bot_added_reactions
                        .read()
                        .unwrap()
                        .reaction_types
                        .is_empty()
                    {
                        tracing::info!(
                            "removing bot added reactions at index `{}` since it is now empty",
                            bot_added_reactions_index
                        );
                        guilds_to_bot_added_reactions
                            .get_mut(guild_id)
                            .unwrap()
                            .swap_remove(bot_added_reactions_index);
                    }
                }
            }
        }
    }
}
