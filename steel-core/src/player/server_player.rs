//! The server-side player session wrapper.
//!
//! Mirrors vanilla's `ServerGamePacketListenerImpl` plus the per-connection
//! server session data. A [`ServerPlayer`] is **directly accessible** (held as
//! `Arc<ServerPlayer>`, never behind the entity lock) and owns the network and
//! session state — connection, inbound queue, chat session, chunk sending, and
//! the published view — alongside a reference to the locked [`Player`] entity.
//!
//! This split exists to keep cross-player fan-out (chat broadcast, tracking)
//! deadlock-free: those flows touch session data on the outer struct and never
//! re-lock the entity. The [`Player`] entity holds a [`Weak<ServerPlayer>`] back
//! reference so entity code can send packets and reach session data.

use std::sync::{Arc, Weak};

use arc_swap::ArcSwap;
use steel_protocol::utils::RawPacket;
use steel_utils::locks::SyncMutex;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::chunk::player_chunk_view::PlayerChunkView;
use crate::config::RuntimeConfig;
use crate::player::chat_state::ChatState;
use crate::player::chunk_sender::ChunkSender;
use crate::player::lifecycle_state::PlayerLifecycleState;
use crate::player::teleport_state::TeleportState;
use crate::player::tick_state::PlayerTickState;
use crate::player::view::PlayerView;
use crate::player::{ClientInformation, Player, PlayerConnection};
use crate::server::Server;

/// The server-side player session: network + session state plus the entity ref.
pub struct ServerPlayer {
    /// The player's connection (abstracted for testing).
    pub connection: Arc<PlayerConnection>,

    /// Reference to the server.
    pub(crate) server: Weak<Server>,
    /// Runtime configuration shared with the server.
    pub(crate) config: Arc<RuntimeConfig>,

    /// The locked game entity for this player.
    entity: Arc<SyncMutex<Player>>,

    /// Inbound game-state packets queued by the connection listener, drained and
    /// applied on the game tick so game state is only mutated by one thread.
    inbound_rx: SyncMutex<UnboundedReceiver<RawPacket>>,

    /// Lock-free published view of cross-loop state (chunk position, send epoch).
    pub view: Arc<PlayerView>,

    /// Chat state: message counters, signature cache, validator, session, chain.
    pub chat: SyncMutex<ChatState>,

    /// The chunk sender for the player (shared with the chunk-sending loop).
    pub chunk_sender: Arc<SyncMutex<ChunkSender>>,

    /// The last chunk tracking view of the player.
    pub last_tracking_view: SyncMutex<Option<PlayerChunkView>>,

    /// The client's settings/information (language, view distance, chat, etc.).
    client_information: SyncMutex<ClientInformation>,

    /// Client lifecycle flags.
    lifecycle: SyncMutex<PlayerLifecycleState>,

    /// Pending server-initiated teleport state (ID, position, timeout).
    teleport_state: SyncMutex<TeleportState>,

    /// Local tick and once-per-tick packet state.
    tick_state: SyncMutex<PlayerTickState>,

    /// World the player's view is being kept for (ArcSwap for lock-free reads).
    pub world: ArcSwap<crate::world::World>,
}

impl ServerPlayer {
    /// Returns the locked game entity for this player.
    #[must_use]
    pub fn entity(&self) -> &Arc<SyncMutex<Player>> {
        &self.entity
    }
}
