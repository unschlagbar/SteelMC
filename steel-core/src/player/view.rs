//! Lock-free published view of a player's cross-loop state.
//!
//! The game tick is the sole writer of these fields; the chunk-sending,
//! chunk-scheduling, tracker, and network paths read them concurrently without
//! taking a lock. Values are stored in atomics so reads can never tear.
//!
//! This is the "shared, read-mostly slice" of the player partition: the bulk of
//! player state lives on [`Player`](crate::player::Player) (owned, mutated under
//! `&mut self` by the game tick), while only the handful of values other loops
//! need are published here behind an `Arc<PlayerView>`.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use steel_utils::types::ChunkPos;

/// Packs a [`ChunkPos`] (two `i32`) into a single `u64` for atomic storage.
#[inline]
const fn pack_chunk_pos(pos: ChunkPos) -> u64 {
    ((pos.0.x as u32 as u64) << 32) | (pos.0.y as u32 as u64)
}

/// Unpacks a `u64` produced by [`pack_chunk_pos`] back into a [`ChunkPos`].
#[inline]
const fn unpack_chunk_pos(raw: u64) -> ChunkPos {
    ChunkPos::new((raw >> 32) as u32 as i32, raw as u32 as i32)
}

/// Published, lock-free snapshot of a player's cross-loop state.
///
/// Written only by the game tick; read by the chunk and network loops.
#[derive(Debug)]
pub struct PlayerView {
    /// The player's last known chunk position (packed `ChunkPos`).
    last_chunk_pos: AtomicU64,
    /// Monotonic epoch bumped on world teleport/reset, used by the chunk
    /// sending tick to detect and discard stale batches.
    chunk_send_epoch: AtomicU32,
}

impl PlayerView {
    /// Creates a new view with the given initial chunk position.
    #[must_use]
    pub fn new(last_chunk_pos: ChunkPos) -> Self {
        Self {
            last_chunk_pos: AtomicU64::new(pack_chunk_pos(last_chunk_pos)),
            chunk_send_epoch: AtomicU32::new(0),
        }
    }

    /// Returns the player's last published chunk position.
    #[must_use]
    pub fn last_chunk_pos(&self) -> ChunkPos {
        unpack_chunk_pos(self.last_chunk_pos.load(Ordering::Relaxed))
    }

    /// Publishes a new chunk position (game tick only).
    pub fn set_last_chunk_pos(&self, pos: ChunkPos) {
        self.last_chunk_pos
            .store(pack_chunk_pos(pos), Ordering::Relaxed);
    }

    /// Returns the current chunk-send epoch.
    #[must_use]
    pub fn chunk_send_epoch(&self) -> u32 {
        self.chunk_send_epoch.load(Ordering::Relaxed)
    }

    /// Bumps the chunk-send epoch, returning the new value (game tick only).
    pub fn bump_chunk_send_epoch(&self) -> u32 {
        // `fetch_add` wraps on overflow, matching the previous `wrapping_add`.
        self.chunk_send_epoch.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_pos_round_trips_through_atomic_packing() {
        for pos in [
            ChunkPos::new(0, 0),
            ChunkPos::new(1, -1),
            ChunkPos::new(i32::MAX, i32::MIN),
            ChunkPos::new(-7, 12345),
        ] {
            assert_eq!(unpack_chunk_pos(pack_chunk_pos(pos)), pos);
        }
    }

    #[test]
    fn view_publishes_and_reads_chunk_pos() {
        let view = PlayerView::new(ChunkPos::new(3, 4));
        assert_eq!(view.last_chunk_pos(), ChunkPos::new(3, 4));
        view.set_last_chunk_pos(ChunkPos::new(-9, 8));
        assert_eq!(view.last_chunk_pos(), ChunkPos::new(-9, 8));
    }

    #[test]
    fn epoch_bumps_monotonically_and_wraps() {
        let view = PlayerView::new(ChunkPos::new(0, 0));
        assert_eq!(view.chunk_send_epoch(), 0);
        assert_eq!(view.bump_chunk_send_epoch(), 1);
        assert_eq!(view.chunk_send_epoch(), 1);
    }
}
