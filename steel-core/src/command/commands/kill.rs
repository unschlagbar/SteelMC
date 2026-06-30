//! Handler for the "kill" command.
//! Mirrors `net.minecraft.server.commands.KillCommand`.

use std::sync::Arc;

use text_components::TextComponent;

use crate::command::arguments::entity::EntityArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::entity::damage::DamageSource;
use crate::player::{Player, ServerPlayer};
use steel_registry::vanilla_damage_types;
use steel_utils::translations;

/// Creates the `/kill` command handler.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(&["kill"], "Kills entities.", "minecraft:command.kill")
        .executes(KillSelfExecutor)
        .then(argument("targets", EntityArgument::multiple()).executes(KillTargetsExecutor))
}

/// `LivingEntity.kill()` — hurt with `genericKill` at `Float.MAX_VALUE`.
fn kill_player(player: &mut Player) {
    player.hurt(
        &DamageSource::environment(&vanilla_damage_types::GENERIC_KILL),
        f32::MAX,
    );
}

struct KillSelfExecutor;

impl CommandExecutor<()> for KillSelfExecutor {
    fn execute(&self, _args: (), context: &mut CommandContext) -> Result<(), CommandError> {
        let player = context
            .sender
            .get_player()
            .ok_or(CommandError::InvalidRequirement)?;

        kill_player(&mut player.entity.lock());

        // TODO: use getDisplayName() (team formatting, hover event, UUID insertion)
        // Release the player lock before `send_message` re-locks the same sender.
        let player_name = player.name.clone();
        context.sender.send_message(
            &translations::COMMANDS_KILL_SUCCESS_SINGLE
                .message([TextComponent::plain(player_name)])
                .into(),
        );

        Ok(())
    }
}

struct KillTargetsExecutor;

impl CommandExecutor<((), Vec<Arc<ServerPlayer>>)> for KillTargetsExecutor {
    fn execute(
        &self,
        args: ((), Vec<Arc<ServerPlayer>>),
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let ((), targets) = args;

        if targets.is_empty() {
            return Err(CommandError::CommandFailed(Box::new(
                TextComponent::const_plain("No entity was found"),
            )));
        }

        let mut last_name = String::new();
        let mut victim_count = 0;
        for target in &targets {
            kill_player(&mut target.entity.lock());
            victim_count += 1;
            last_name.clone_from(&target.name);
            // TODO: non-player entities via Entity::kill() (remove with RemovalReason::KILLED)
        }

        if victim_count == 0 {
            return Err(CommandError::CommandFailed(Box::new(
                TextComponent::const_plain("No entity was found"),
            )));
        }

        // TODO: use getDisplayName() (team formatting, hover event, UUID insertion)
        if victim_count == 1 {
            context.sender.send_message(
                &translations::COMMANDS_KILL_SUCCESS_SINGLE
                    .message([TextComponent::plain(last_name)])
                    .into(),
            );
        } else {
            context.sender.send_message(
                &translations::COMMANDS_KILL_SUCCESS_MULTIPLE
                    .message([TextComponent::plain(victim_count.to_string())])
                    .into(),
            );
        }

        Ok(())
    }
}
