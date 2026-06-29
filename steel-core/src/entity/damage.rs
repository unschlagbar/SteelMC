//! Damage source system.

use glam::DVec3;
use steel_registry::{
    REGISTRY, TaggedRegistryExt, damage_type::DamageScaling, damage_type::DamageType,
    entity_type::EntityTypeRef, loot_table::EntityRefFlags, vanilla_damage_type_tags,
};

/// Loot-context snapshot of the entity that caused this damage, captured at
/// damage time.
///
/// The death-loot path needs the causing entity's type and live flags to build
/// its loot `EntityRef`. Resolving that entity by id and locking it again would
/// deadlock when the causer is a player whose behavior mutex is already held by
/// the tick loop (the attacker, mid packet-handling). Capturing the snapshot
/// here — while the caller already holds that entity locked — lets the loot path
/// build the `EntityRef` without re-locking. See `entity_loot_ref`.
#[derive(Debug, Clone, Copy)]
pub struct CausingEntityLoot {
    /// The causing entity's type.
    pub entity_type: EntityTypeRef,
    /// The causing entity's live loot-predicate flags.
    pub flags: EntityRefFlags,
}

/// Describes how an entity was damaged.
#[derive(Debug, Clone)]
pub struct DamageSource {
    /// The damage type registry entry.
    pub damage_type: &'static DamageType,
    /// The entity ultimately responsible (e.g. the shooter for projectiles).
    pub causing_entity_id: Option<i32>,
    /// The entity that directly dealt the damage (e.g. the projectile itself).
    pub direct_entity_id: Option<i32>,
    /// Source position (for explosions, etc.).
    pub source_position: Option<DVec3>,
    /// Loot snapshot of the causing entity, threaded so the death-loot path can
    /// build its `EntityRef` without re-locking a possibly-already-locked causer.
    pub causing_entity_loot: Option<CausingEntityLoot>,
}

impl DamageSource {
    /// Environmental damage with no entity or position context (void, starvation, etc.).
    #[must_use]
    pub const fn environment(damage_type: &'static DamageType) -> Self {
        Self {
            damage_type,
            causing_entity_id: None,
            direct_entity_id: None,
            source_position: None,
            causing_entity_loot: None,
        }
    }

    /// Adds the entity ultimately responsible for the damage.
    #[must_use]
    pub const fn with_causing_entity(mut self, entity_id: i32) -> Self {
        self.causing_entity_id = Some(entity_id);
        self
    }

    /// Adds the direct entity that delivered the damage.
    #[must_use]
    pub const fn with_direct_entity(mut self, entity_id: i32) -> Self {
        self.direct_entity_id = Some(entity_id);
        self
    }

    /// Adds the vanilla source position used by damage events and knockback.
    #[must_use]
    pub const fn with_source_position(mut self, source_position: DVec3) -> Self {
        self.source_position = Some(source_position);
        self
    }

    /// Attaches the causing entity's loot snapshot (see [`CausingEntityLoot`]).
    ///
    /// Capture this at damage-creation time, where the caller already holds the
    /// causing entity locked, so the death-loot path never re-locks it.
    #[must_use]
    pub const fn with_causing_entity_loot(mut self, loot: CausingEntityLoot) -> Self {
        self.causing_entity_loot = Some(loot);
        self
    }

    /// Whether this damage bypasses creative/spectator invulnerability.
    #[must_use]
    pub fn bypasses_invulnerability(&self) -> bool {
        self.is(&vanilla_damage_type_tags::DamageTypeTag::BYPASSES_INVULNERABILITY)
    }

    /// Returns whether this damage type is in the given vanilla damage-type tag.
    #[must_use]
    pub fn is(&self, tag: &steel_utils::Identifier) -> bool {
        REGISTRY.damage_types.is_in_tag(self.damage_type, tag)
    }

    /// Returns vanilla `DamageSource.isDirect`.
    #[must_use]
    pub fn is_direct(&self) -> bool {
        self.causing_entity_id == self.direct_entity_id
    }

    /// Whether this damage bypasses the invulnerability cooldown timer.
    /// No vanilla damage types currently use this, but the logic exists in
    /// `LivingEntity.hurtServer()`.
    /// TODO: use damage type tag query once supported
    #[expect(clippy::unused_self, reason = "this is an api function")]
    #[must_use]
    pub const fn bypasses_cooldown(&self) -> bool {
        false
    }

    /// Whether this damage scales with world difficulty.
    /// Reads the `scaling` field from the damage type registry entry.
    #[must_use]
    pub const fn scales_with_difficulty(&self) -> bool {
        match self.damage_type.scaling {
            DamageScaling::Never => false,
            // TODO: WhenCausedByLivingNonPlayer needs entity type checking
            DamageScaling::Always | DamageScaling::WhenCausedByLivingNonPlayer => true,
        }
    }
}
