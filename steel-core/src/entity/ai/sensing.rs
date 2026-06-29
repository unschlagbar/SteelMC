//! Per-mob sensing caches.

use rustc_hash::FxHashSet;

/// Vanilla `Sensing` line-of-sight cache.
#[derive(Debug, Default)]
pub(crate) struct Sensing {
    seen: FxHashSet<i32>,
    unseen: FxHashSet<i32>,
}

impl Sensing {
    /// Creates an empty sensing cache.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Clears per-tick sensing results.
    pub(crate) fn tick(&mut self) {
        self.seen.clear();
        self.unseen.clear();
    }

    /// Returns cached line-of-sight for `target_id`, computing it on first use.
    pub(crate) fn has_line_of_sight(
        &mut self,
        target_id: i32,
        test: impl FnOnce() -> bool,
    ) -> bool {
        if let Some(cached) = self.cached_line_of_sight(target_id) {
            return cached;
        }

        let has_line_of_sight = test();
        self.record_line_of_sight(target_id, has_line_of_sight);
        has_line_of_sight
    }

    /// Returns the line-of-sight result cached for `target_id` this tick, if any.
    ///
    /// Split out from [`Self::has_line_of_sight`] so callers can run the
    /// (immutable-`self`) line-of-sight test without holding a `&mut` borrow of
    /// the sensing cache across it.
    #[must_use]
    pub(crate) fn cached_line_of_sight(&self, target_id: i32) -> Option<bool> {
        if self.seen.contains(&target_id) {
            Some(true)
        } else if self.unseen.contains(&target_id) {
            Some(false)
        } else {
            None
        }
    }

    /// Records a freshly computed line-of-sight result for `target_id`.
    pub(crate) fn record_line_of_sight(&mut self, target_id: i32, has_line_of_sight: bool) {
        if has_line_of_sight {
            self.seen.insert(target_id);
        } else {
            self.unseen.insert(target_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn sensing_caches_seen_targets_until_tick() {
        let calls = AtomicUsize::new(0);
        let mut sensing = Sensing::new();

        assert!(sensing.has_line_of_sight(1, || {
            calls.fetch_add(1, Ordering::Relaxed);
            true
        }));
        assert!(sensing.has_line_of_sight(1, || {
            calls.fetch_add(1, Ordering::Relaxed);
            false
        }));
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        sensing.tick();
        assert!(!sensing.has_line_of_sight(1, || {
            calls.fetch_add(1, Ordering::Relaxed);
            false
        }));
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn sensing_caches_unseen_targets_until_tick() {
        let calls = AtomicUsize::new(0);
        let mut sensing = Sensing::new();

        assert!(!sensing.has_line_of_sight(1, || {
            calls.fetch_add(1, Ordering::Relaxed);
            false
        }));
        assert!(!sensing.has_line_of_sight(1, || {
            calls.fetch_add(1, Ordering::Relaxed);
            true
        }));

        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
