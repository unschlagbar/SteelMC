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
use steel_protocol::packet_traits::{ClientPacket, EncodedPacket};
use steel_protocol::utils::{ConnectionProtocol, RawPacket};
use steel_utils::ChunkPos;
use steel_utils::locks::SyncMutex;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::chunk::player_chunk_view::PlayerChunkView;
use crate::config::RuntimeConfig;
use crate::player::chat_state::ChatState;
use crate::player::chunk_sender::ChunkSender;
use crate::player::connection::NetworkConnection as _;
use crate::player::lifecycle_state::PlayerLifecycleState;
use crate::player::networking::BundleBuilder;
use crate::player::teleport_state::TeleportState;
use crate::player::tick_state::PlayerTickState;
use crate::player::view::PlayerView;
use crate::player::{ClientInformation, GameProfile, Player, PlayerConnection};
use crate::server::Server;
use crate::world::World;

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
    pub(crate) inbound_rx: SyncMutex<UnboundedReceiver<RawPacket>>,

    /// Lock-free published view of cross-loop state (chunk position, send epoch).
    pub view: Arc<PlayerView>,

    /// Chat state: message counters, signature cache, validator, session, chain.
    pub chat: SyncMutex<ChatState>,

    /// The chunk sender for the player (shared with the chunk-sending loop).
    pub chunk_sender: Arc<SyncMutex<ChunkSender>>,

    /// The last chunk tracking view of the player.
    pub last_tracking_view: SyncMutex<Option<PlayerChunkView>>,

    /// The client's settings/information (language, view distance, chat, etc.).
    pub(crate) client_information: SyncMutex<ClientInformation>,

    /// Client lifecycle flags.
    pub(crate) lifecycle: SyncMutex<PlayerLifecycleState>,

    /// Pending server-initiated teleport state (ID, position, timeout).
    pub(crate) teleport_state: SyncMutex<TeleportState>,

    /// Local tick and once-per-tick packet state.
    pub(crate) tick_state: SyncMutex<PlayerTickState>,

    /// World the player's view is being kept for (`ArcSwap` for lock-free reads).
    pub world: ArcSwap<World>,
}

impl ServerPlayer {
    /// Creates the server-side session and its locked [`Player`] entity.
    ///
    /// Must be called inside `Arc::new_cyclic` so the entity and connection can
    /// hold a [`Weak<ServerPlayer>`] back reference. The `connection` is built by
    /// the caller (it needs login-specific transport state) already wired to
    /// `sp_weak`.
    #[expect(clippy::too_many_arguments, reason = "session construction is wide")]
    #[must_use]
    pub fn new(
        sp_weak: &Weak<ServerPlayer>,
        gameprofile: GameProfile,
        connection: Arc<PlayerConnection>,
        world: Arc<World>,
        server: Weak<Server>,
        config: Arc<RuntimeConfig>,
        entity_id: i32,
        client_information: ClientInformation,
        inbound_rx: UnboundedReceiver<RawPacket>,
    ) -> Self {
        let entity = Arc::new_cyclic(|entity_weak| {
            SyncMutex::new(Player::new(
                gameprofile,
                &world,
                entity_id,
                entity_weak,
                sp_weak.clone(),
            ))
        });

        let chat_spam_threshold_seconds = config.chat_spam_threshold_seconds;
        let command_spam_threshold_seconds = config.command_spam_threshold_seconds;

        Self {
            connection,
            server,
            config,
            entity,
            inbound_rx: SyncMutex::new(inbound_rx),
            view: Arc::new(PlayerView::new(ChunkPos::new(0, 0))),
            chat: SyncMutex::new(ChatState::new(
                chat_spam_threshold_seconds,
                command_spam_threshold_seconds,
            )),
            chunk_sender: Arc::new(SyncMutex::new(ChunkSender::default())),
            last_tracking_view: SyncMutex::new(None),
            client_information: SyncMutex::new(client_information),
            lifecycle: SyncMutex::new(PlayerLifecycleState::default()),
            teleport_state: SyncMutex::new(TeleportState::new()),
            tick_state: SyncMutex::new(PlayerTickState::new()),
            world: ArcSwap::new(world),
        }
    }

    /// Returns the locked game entity for this player.
    #[must_use]
    pub fn entity(&self) -> &Arc<SyncMutex<Player>> {
        &self.entity
    }

    /// Returns the effective view distance (client request clamped to the server
    /// maximum). Read lock-free from session state, without the entity lock.
    #[must_use]
    pub fn view_distance(&self) -> u8 {
        let client_view_distance = self.client_information.lock().view_distance;
        client_view_distance.min(self.world.load().view_distance)
    }

    /// Sends a packet to the player's connection (lock-free; no entity lock).
    ///
    /// # Panics
    /// Panics if the packet fails to encode.
    pub fn send_packet<P: ClientPacket>(&self, packet: P) {
        let encoded = EncodedPacket::from_bare(
            packet,
            self.connection.compression(),
            ConnectionProtocol::Play,
        )
        .expect("Failed to encode packet");
        self.connection.send_encoded(encoded);
    }

    /// Sends multiple packets as an atomic bundle.
    pub fn send_bundle<F>(&self, f: F)
    where
        F: FnOnce(&mut BundleBuilder),
    {
        let mut builder = BundleBuilder::new(self.connection.compression());
        f(&mut builder);
        let packets = builder.into_packets();
        if !packets.is_empty() {
            self.connection.send_encoded_bundle(packets);
        }
    }
}
