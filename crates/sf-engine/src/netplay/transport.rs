//! Pure commands and events at the engine/host boundary (ADR-0005).

use super::{Link, Room, RoomKind};
use crate::config::RulesetPreset;

pub const HOST_PEER: u8 = 0;
pub const MAX_SPECTATORS: u8 = 4;
pub const MAX_ROOM_MEMBERS: u8 = 2 + MAX_SPECTATORS;
/// Routes include pending/rejected handshakes. The session assigns accepted
/// player/spectator roles independently; route 1 is not necessarily Player 2.
pub const MAX_ROUTES: u8 = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomAnnouncement {
    pub host: [u8; 12],
    pub kind: RoomKind,
    pub link: Link,
    pub wired_only: bool,
    pub public: bool,
    pub preset: RulesetPreset,
    pub in_match: bool,
    /// Accepted spectators, owned by the session rather than UDP connections.
    pub spectators: u8,
    pub max_rollback: u8,
    pub config_hash: u64,
    pub bank_hash: u64,
    pub art_pack_hash: u64,
    pub protocol_hash: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetTransportCommand {
    BindHost { port: u16, room: RoomAnnouncement },
    ConnectPeer { addr: String },
    SendPacket { to_peer: u8, bytes: Vec<u8> },
    RefreshRooms,
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetTransportEvent {
    Bound { local_addr: String, room_code: String, link: Link },
    PacketReceived { from_peer: u8, addr: String, bytes: Vec<u8> },
    RoomsDiscovered { rooms: Vec<Room> },
    TransportError { message: String },
}
