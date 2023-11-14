//! Commands.

pub mod add_reaction;
pub mod ping;

use serenity::{
    async_trait, builder::CreateApplicationCommand,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    prelude::*,
};

/// Discord command.
#[async_trait]
pub trait Command: Send + Sync + 'static {
    /// Register the command.
    fn register(command: &mut CreateApplicationCommand) -> Self
    where
        Self: Sized;

    /// Interaction with the command.
    async fn interaction(
        &mut self,
        command_interaction: &ApplicationCommandInteraction,
        context: &Context,
    );
}
