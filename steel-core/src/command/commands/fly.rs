//! Handler for the "flyspeed" command.
use std::slice;
use std::sync::Arc;

use crate::command::arguments::bool::BoolArgument;
use crate::command::arguments::float::FloatArgument;
use crate::command::arguments::player::PlayerArgument;
use crate::command::commands::{CommandHandlerBuilder, CommandHandlerDyn, argument, literal};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::command::sender::CommandSender;
use crate::player::ServerPlayer;
use text_components::TextComponent;

const MAX_FLY_SPEED: f32 = 30f32;

/// Handler for the "flyspeed" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["fly"],
        "Sets the target's flying parameters (may_fly, speed).",
        "minecraft:command.fly",
    )
    .executes(|(), ctx: &mut CommandContext| {
        let player = ctx
            .sender
            .get_player()
            .ok_or(CommandError::InvalidRequirement)?;

        toggle_fly(slice::from_ref(player));

        Ok(())
    })
    .then(
        argument("target", PlayerArgument::multiple())
            .executes(
                |((), targets): ((), Vec<Arc<ServerPlayer>>), _ctx: &mut CommandContext| {
                    toggle_fly(&targets);
                    Ok(())
                },
            )
            .then(argument("value", BoolArgument).executes(
                |(((), targets), value): (((), Vec<Arc<ServerPlayer>>), bool),
                 _ctx: &mut CommandContext| {
                    set_fly(&targets, value);
                    Ok(())
                },
            ))
            .then(
                literal("speed")
                    .executes(
                        |((), targets): ((), Vec<Arc<ServerPlayer>>), ctx: &mut CommandContext| {
                            query_flying_speed(&targets, &ctx.sender);
                            Ok(())
                        },
                    )
                    .then(
                        argument(
                            "speed",
                            FloatArgument::bounded(Some(0.0), Some(MAX_FLY_SPEED)),
                        )
                        .executes(
                            |(((), targets), speed): (((), Vec<Arc<ServerPlayer>>), f32),
                             ctx: &mut CommandContext| {
                                set_flying_speed(&targets, speed, &ctx.sender);

                                Ok(())
                            },
                        ),
                    ),
            ),
    )
    .then(
        literal("speed")
            .executes(|(), ctx: &mut CommandContext| {
                let player = ctx
                    .sender
                    .get_player()
                    .ok_or(CommandError::InvalidRequirement)?;

                query_flying_speed(slice::from_ref(player), &ctx.sender);

                Ok(())
            })
            .then(
                argument(
                    "speed",
                    FloatArgument::bounded(Some(0.0), Some(MAX_FLY_SPEED)),
                )
                .executes(|((), speed): ((), f32), ctx: &mut CommandContext| {
                    let player = ctx
                        .sender
                        .get_player()
                        .ok_or(CommandError::InvalidRequirement)?;
                    set_flying_speed(slice::from_ref(player), speed, &ctx.sender);
                    Ok(())
                }),
            ),
    )
}

fn toggle_fly(targets: &[Arc<ServerPlayer>]) {
    for target in targets {
        {
            let mut target_guard = target.entity.lock();
            let abilities = &mut target_guard.abilities;
            abilities.may_fly = !abilities.may_fly;
            if !abilities.may_fly {
                abilities.flying = false;
            }
        }
        target.entity.lock().send_abilities();
    }
}

fn set_fly(targets: &[Arc<ServerPlayer>], value: bool) {
    for target in targets {
        {
            let mut target_guard = target.entity.lock();
            let abilities = &mut target_guard.abilities;

            abilities.may_fly = value;
            if !value {
                abilities.flying = false;
            }
        }
        target.entity.lock().send_abilities();
    }
}

fn set_flying_speed(targets: &[Arc<ServerPlayer>], multiplier: f32, sender: &CommandSender) {
    let speed = multiplier * 0.05;
    for target in targets {
        let target_name = {
            let mut guard = target.entity.lock();
            guard.set_flying_speed(speed);
            guard.send_abilities();
            guard.gameprofile.name.clone()
        };
        // Lock released above: `send_message` locks the sender, who may be this target.
        sender.send_message(&TextComponent::from(format!(
            "Set flying speed for player '{target_name}' to {multiplier:.1}x ({speed:.3})"
        )));
    }
}

fn query_flying_speed(targets: &[Arc<ServerPlayer>], sender: &CommandSender) {
    for target in targets {
        let (speed, target_name) = {
            let guard = target.entity.lock();
            (guard.get_flying_speed(), guard.gameprofile.name.clone())
        };
        let multiplier = speed / 0.05; // Show as multiplier of default speed

        // Lock released above: `send_message` locks the sender, who may be this target.
        sender.send_message(&TextComponent::from(format!(
            "Current flying speed for player '{target_name}': {multiplier:.1}x ({speed:.3})"
        )));
    }
}
