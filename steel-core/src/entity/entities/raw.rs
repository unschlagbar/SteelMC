//! NBT-preserving fallback entity.

use std::sync::{Arc, Weak};

use glam::DVec3;
use simdnbt::borrow::NbtCompound as BorrowedNbtCompoundView;
use simdnbt::owned::NbtCompound;
use steel_registry::entity_type::EntityTypeRef;
use steel_utils::{Identifier, locks::SyncMutex};

use crate::entity::{
    Entity, EntityBase, EntityBaseLoad, EntityIdentifier, SharedEntity, next_entity_id,
};

/// Steel-specific fallback for entity types whose runtime behavior is not implemented yet.
///
/// Vanilla has concrete classes for every entity type. Steel uses this only to preserve
/// worldgen and disk NBT until the corresponding typed implementation is added.
pub struct RawEntity {
    base: Weak<EntityBase>,
    entity_type: EntityTypeRef,
    data: SyncMutex<NbtCompound>,
}

impl RawEntity {
    /// Creates a fresh raw entity for an entity type Steel cannot behaviorally model yet.
    #[must_use]
    pub fn new(entity_type: EntityTypeRef) -> SharedEntity {
        Self::new_raw(next_entity_id(), entity_type)
    }

    /// Todo
    #[must_use]
    pub fn new_raw(id: i32, entity_type: EntityTypeRef) -> SharedEntity {
        EntityBase::pack_with(
            id,
            DVec3::ZERO,
            entity_type.dimensions,
            Weak::new(),
            |base| Self {
                base,
                entity_type,
                data: SyncMutex::new(NbtCompound::new()),
            },
        )
    }

    /// Creates a raw entity from base entity data.
    #[must_use]
    pub fn from_saved(load: EntityBaseLoad, entity_type: EntityTypeRef) -> SharedEntity {
        EntityBase::pack_loaded_with(load, entity_type.dimensions, |base| Self {
            base,
            entity_type,
            data: SyncMutex::new(NbtCompound::new()),
        })
    }

    /// Sets position and rotation, matching vanilla `Entity.snapTo`.
    ///
    /// # Panics
    ///
    /// Panics if the active world entity manager rejects the snap position. This is an invariant
    /// failure for loaded raw entities.
    pub fn snap_to(&self, position: DVec3, yaw: f32, pitch: f32) {
        let base = self.base().expect("RawEntity snap_to called after removal");
        if let Err(error) = base.try_set_position(position) {
            panic!(
                "failed to commit raw entity {} snap position: {error}",
                base.id()
            );
        }
        base.set_rotation((yaw, pitch));
        self.set_old_position_to_current();
    }

    fn base(&self) -> Option<Arc<EntityBase>> {
        self.base.upgrade()
    }

    /// Marks a raw mob as persistent when vanilla structure generation would do so.
    pub fn set_persistence_required(&self) {
        self.data.lock().insert("PersistenceRequired", 1_i8);
    }
}

impl Entity for RawEntity {
    fn base_weak(&self) -> &Weak<EntityBase> {
        &self.base
    }

    fn entity_type(&self) -> EntityTypeRef {
        self.entity_type
    }

    fn tick(&mut self) {
        // TODO: Replace raw entity ticking with full vanilla behavior for this entity type.
    }

    fn attackable(&self) -> bool {
        false
    }

    fn load_additional(&mut self, nbt: BorrowedNbtCompoundView<'_, '_>) {
        *self.data.lock() = nbt.to_owned();
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        *nbt = self.data.lock().clone();
    }
}

impl EntityIdentifier for RawEntity {
    const KEY: Identifier = Identifier::new_static("steel", "unimplemented");
}
