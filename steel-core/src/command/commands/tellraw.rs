//! Handler for the "tellraw" command.
use crate::command::arguments::player::PlayerArgument;
use steel_utils::locks::SyncMutex;
use crate::command::arguments::text_component::TextComponentArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::command::sender::CommandSender;
use crate::player::Player;
use std::sync::Arc;
use text_components::TextComponent;

/// Handler for the "tellraw" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["tellraw"],
        "Sends a JSON message to players.",
        "minecraft:command.tellraw",
    )
    .then(
        argument("targets", PlayerArgument::multiple())
            .then(argument("message", TextComponentArgument).executes(TellrawCommandExecutor)),
    )
}

struct TellrawCommandExecutor;

impl CommandExecutor<(((), Vec<Arc<SyncMutex<Player>>>), TextComponent)> for TellrawCommandExecutor {
    fn execute(
        &self,
        args: (((), Vec<Arc<SyncMutex<Player>>>), TextComponent),
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let sender = match &context.sender {
            CommandSender::Player(player) => player.lock().gameprofile.name.clone(),
            CommandSender::Console => "Console".to_string(),
            CommandSender::Rcon => "Rcon".to_string(),
        };
        log::info!("{}'s tellraw: {:p}", sender, args.1);
        for player in args.0.1 {
            player.lock().send_message(&args.1);
        }
        Ok(())
    }
}
