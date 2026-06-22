//! Thread-safe player storage with dual indexing by UUID and entity ID.

use std::sync::Arc;

use scc::HashMap;
use uuid::Uuid;

use crate::{entity::Entity, player::ServerPlayer};

/// Thread-safe player storage with dual indexing.
///
/// Maintains two synchronized maps for O(1) lookup by either UUID or entity ID.
/// All operations keep both maps in sync automatically. Stores the outer
/// [`ServerPlayer`] session handle; the locked entity is reached via
/// [`ServerPlayer::entity`].
pub struct PlayerMap {
    /// Primary index by UUID (persistent identifier)
    by_uuid: HashMap<Uuid, Arc<ServerPlayer>>,
    /// Secondary index by entity ID (session-local identifier)
    by_entity_id: HashMap<i32, Arc<ServerPlayer>>,
}

impl Default for PlayerMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerMap {
    /// Creates a new empty player map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_uuid: HashMap::new(),
            by_entity_id: HashMap::new(),
        }
    }

    /// Inserts a player into both maps.
    ///
    /// Returns `true` if the player was inserted, `false` if a player with the same UUID already exists.
    ///
    /// # Panics
    ///
    /// Panics if another player already has the same entity ID. Entity IDs are
    /// session-unique; accepting a duplicate would break entity lookup and
    /// packet routing invariants.
    pub fn insert(&self, player: Arc<ServerPlayer>) -> bool {
        let (uuid, entity_id) = {
            let guard = player.entity().lock();
            (guard.gameprofile.id, guard.id())
        };

        if self.by_uuid.insert_sync(uuid, player.clone()).is_err() {
            return false;
        }

        if self.by_entity_id.insert_sync(entity_id, player).is_err() {
            let _ = self.by_uuid.remove_sync(&uuid);
            panic!("player entity id {entity_id} is already registered");
        }
        true
    }

    /// Removes a player by UUID from both maps.
    ///
    /// Returns the removed player if found.
    pub async fn remove(&self, uuid: &Uuid) -> Option<Arc<ServerPlayer>> {
        if let Some((_, player)) = self.by_uuid.remove_async(uuid).await {
            let entity_id = player.entity().lock().id();
            let _ = self.by_entity_id.remove_async(&entity_id).await;
            Some(player)
        } else {
            None
        }
    }

    /// Removes this exact player from both maps.
    ///
    /// Returns the removed player if the UUID still maps to this same player
    /// handle. A stale duplicate-login cleanup must not remove the accepted
    /// player that owns the UUID.
    pub async fn remove_player(&self, player: &Arc<ServerPlayer>) -> Option<Arc<ServerPlayer>> {
        let uuid = player.entity().lock().gameprofile.id;
        let (_, removed) = self
            .by_uuid
            .remove_if_async(&uuid, |current| Arc::ptr_eq(current, player))
            .await?;
        let removed_id = removed.entity().lock().id();
        let _ = self
            .by_entity_id
            .remove_if_async(&removed_id, |current| Arc::ptr_eq(current, &removed))
            .await;
        Some(removed)
    }

    /// Removes a player by UUID from both maps synchronously.
    ///
    /// Returns the removed player if found. Use this when async is not available
    /// (e.g., during world changes on the tick thread).
    pub fn remove_sync(&self, uuid: &Uuid) -> Option<Arc<ServerPlayer>> {
        if let Some((_, player)) = self.by_uuid.remove_sync(uuid) {
            let entity_id = player.entity().lock().id();
            let _ = self.by_entity_id.remove_sync(&entity_id);
            Some(player)
        } else {
            None
        }
    }

    /// Removes this exact player from both maps synchronously.
    pub fn remove_player_sync(&self, player: &Arc<ServerPlayer>) -> Option<Arc<ServerPlayer>> {
        let uuid = player.entity().lock().gameprofile.id;
        let (_, removed) = self
            .by_uuid
            .remove_if_sync(&uuid, |current| Arc::ptr_eq(current, player))?;
        let removed_id = removed.entity().lock().id();
        let _ = self
            .by_entity_id
            .remove_if_sync(&removed_id, |current| Arc::ptr_eq(current, &removed));
        Some(removed)
    }

    /// Gets a player by UUID.
    #[must_use]
    pub fn get_by_uuid(&self, uuid: &Uuid) -> Option<Arc<ServerPlayer>> {
        self.by_uuid.read_sync(uuid, |_, p| p.clone())
    }

    /// Gets a player by entity ID.
    #[must_use]
    pub fn get_by_entity_id(&self, entity_id: i32) -> Option<Arc<ServerPlayer>> {
        self.by_entity_id.read_sync(&entity_id, |_, p| p.clone())
    }

    /// Iterates over all players.
    ///
    /// The callback returns `true` to continue iteration, `false` to stop.
    pub fn iter_players<F>(&self, mut f: F)
    where
        F: FnMut(&Uuid, &Arc<ServerPlayer>) -> bool,
    {
        self.by_uuid.iter_sync(|uuid, player| f(uuid, player));
    }

    /// Iterates over the player session handles for the concurrent chunk-sending
    /// loop. Equivalent to [`Self::iter_players`]; the [`ServerPlayer`] is itself
    /// the lock-free handle (connection, view, chunk sender) the loop needs.
    pub fn iter_chunk_handles<F>(&self, mut f: F)
    where
        F: FnMut(&Uuid, &Arc<ServerPlayer>) -> bool,
    {
        self.by_uuid.iter_sync(|uuid, player| f(uuid, player));
    }

    /// Returns the number of players.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_uuid.len()
    }

    /// Returns true if there are no players.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_uuid.is_empty()
    }
}
