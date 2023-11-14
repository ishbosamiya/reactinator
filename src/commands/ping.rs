//! `ping` command.

use serenity::{
    builder::CreateApplicationCommand, model::prelude::application_command::CommandDataOption,
};

/// Register the command.
pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("ping").description("Ping command")
}

/// Run the command.
pub fn run(_options: &[CommandDataOption]) -> Option<String> {
    Some("pong".to_string())
}
