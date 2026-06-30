//! A player argument.
use crate::command::arguments::CommandArgument;
use crate::command::arguments::SuggestionContext;
use crate::command::context::CommandContext;
use crate::player::ServerPlayer;
use rand::seq::IteratorRandom;
use std::sync::Arc;
use steel_protocol::packets::game::{ArgumentType, SuggestionEntry, SuggestionType};
use steel_utils::translations::{
    ARGUMENT_ENTITY_SELECTOR_ALL_PLAYERS, ARGUMENT_ENTITY_SELECTOR_NEAREST_PLAYER,
    ARGUMENT_ENTITY_SELECTOR_RANDOM_PLAYER, ARGUMENT_ENTITY_SELECTOR_SELF,
};
use uuid::Uuid;

/// A player argument.
#[derive(Default)]
pub struct PlayerArgument {
    /// If only accepts one player
    one: bool,
}
impl PlayerArgument {
    /// Creates a selector for multiple players
    #[must_use]
    pub const fn multiple() -> Self {
        PlayerArgument { one: false }
    }
    /// Creates a selector for one player
    #[must_use]
    pub const fn one() -> Self {
        PlayerArgument { one: true }
    }
}

impl CommandArgument for PlayerArgument {
    type Output = Vec<Arc<ServerPlayer>>;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let players = context.server.get_server_players();
        if arg.is_empty() {
            return None;
        }
        if players.is_empty() {
            return Some((&arg[1..], vec![]));
        }
        let entities = match arg[0] {
            "@a" => players,
            "@p" => {
                let position = context.position;
                let mut near_dist = (f64::MAX, players[0].clone());
                for player in players {
                    let dist = player.entity_base.position().distance_squared(position);
                    if dist < near_dist.0 {
                        near_dist = (dist, player);
                    }
                }
                vec![near_dist.1]
            }
            "@r" => {
                vec![players.into_iter().choose(&mut rand::rng())?]
            }
            "@s" => {
                if let Some(player) = &context.player {
                    vec![player.clone()]
                } else {
                    vec![]
                }
            }
            name => {
                let uuid = if let Ok(uuid) = Uuid::parse_str(name) {
                    uuid
                } else {
                    Uuid::nil()
                };
                // Name and UUID are lock-free on `ServerPlayer`.
                let player = players
                    .into_iter()
                    .find(|p| &p.name == name || p.uuid == uuid)?;
                vec![player]
            }
        };
        // TODO: Add entity argiments. (e.g. @e[limit=1])
        Some((&arg[1..], entities))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (
            ArgumentType::Entity {
                flags: 2 | u8::from(self.one),
            },
            Some(SuggestionType::AskServer),
        )
    }

    fn suggest(&self, prefix: &str, suggestion_ctx: &SuggestionContext) -> Vec<SuggestionEntry> {
        let mut suggestions = vec![
            SuggestionEntry::with_tooltip("@a", &ARGUMENT_ENTITY_SELECTOR_ALL_PLAYERS),
            SuggestionEntry::with_tooltip("@p", &ARGUMENT_ENTITY_SELECTOR_NEAREST_PLAYER),
            SuggestionEntry::with_tooltip("@r", &ARGUMENT_ENTITY_SELECTOR_RANDOM_PLAYER),
            SuggestionEntry::with_tooltip("@s", &ARGUMENT_ENTITY_SELECTOR_SELF),
        ];
        suggestions.append(
            &mut suggestion_ctx
                .server
                .get_server_players()
                .iter()
                .map(|p| SuggestionEntry::new(p.name.clone()))
                .collect(),
        );
        suggestions.retain(|s| s.text.starts_with(prefix));
        suggestions
    }
}
