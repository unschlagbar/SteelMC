//! Handler for the "list" command.

use crate::command::{
    commands::{CommandHandlerBuilder, CommandHandlerDyn, literal},
    context::CommandContext,
    error::CommandError,
};
use steel_utils::translations::{COMMANDS_LIST_NAME_AND_ID, COMMANDS_LIST_PLAYERS};

/// Handler for the "list" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["list"],
        "Lists players on the server.",
        "minecraft:command.list",
    )
    .executes(
        |(), context: &mut CommandContext| -> Result<(), CommandError> {
            list_players(context, false);
            Ok(())
        },
    )
    .then(literal("uuids").executes(
        |(), context: &mut CommandContext| -> Result<(), CommandError> {
            list_players(context, true);
            Ok(())
        },
    ))
}

fn list_players(context: &mut CommandContext, show_uuids: bool) {
    let player_number = context.server.player_count();
    let max_player = context.server.config.max_players;
    let formatted_player_list = context
        .server
        .get_server_players()
        .iter()
        .map(|player| {
            // Name and UUID are lock-free on `ServerPlayer`.
            if show_uuids {
                COMMANDS_LIST_NAME_AND_ID
                    .message([player.name().to_string(), player.uuid().to_string()])
                    .component()
                    .to_string()
            } else {
                player.name().to_string()
            }
        })
        .collect::<Vec<String>>()
        .join(", ");

    context.sender.send_message(
        &COMMANDS_LIST_PLAYERS
            .message([
                player_number.to_string(),
                max_player.to_string(),
                formatted_player_list,
            ])
            .into(),
    );
}
