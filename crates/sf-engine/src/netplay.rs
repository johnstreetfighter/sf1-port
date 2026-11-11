//! Pure rollback on the real fight, versioned packets and the room browser.
//!
//! Late inputs restore and replay speculative native frames. The handshake
//! synchronizes fighters, rules, stage, seed and immutable bank/art identity.
//! Physical bindings, presentation settings and the jukebox remain local;
//! socket I/O belongs to sf-host through the command/event queues.

use crate::config::{MatchConfig, RulesetPreset};
use crate::front::session::PlayerInputs;

pub mod protocol;
pub(crate) mod connection;
pub mod rollback;
pub mod transport;
pub mod bus;

pub use protocol::{AfterMatchChoice, NetPacket, RejectReason, SyncedMatchRules};
pub use rollback::{RollbackEngine, RollbackEvent, MAX_ROLLBACK_FRAMES};
pub use transport::{NetTransportCommand, NetTransportEvent, RoomAnnouncement};

/// Right/left/down/up occupy bits 0..3; LP/MP/HP/LK/MK/HK occupy 4..9.
/// All other bits belong to the transport, including bit 15's sentinel.
pub fn encode_inputs(inputs: PlayerInputs) -> u16 {
    inputs.directions.into_iter().chain(inputs.buttons).enumerate()
        .fold(0, |bits, (i, pressed)| bits | (u16::from(pressed) << i))
}

/// Decode gameplay inputs only; reserved transport bits never press a button.
pub fn decode_inputs(bits: u16) -> PlayerInputs {
    PlayerInputs {
        directions: std::array::from_fn(|i| bits & (1 << i) != 0),
        buttons: std::array::from_fn(|i| bits & (1 << (i + 4)) != 0),
    }
}

/// A shareable IPv4 address, omitting the suffix for the default UDP port 7000.
///
/// # Panics
/// Panics when `port` is zero: room codes name a remote endpoint, not a local
/// request for the OS to choose a port.
pub fn encode_room_code(ipv4: [u8; 4], port: u16) -> String {
    assert_ne!(port, 0, "room code port must be nonzero");
    let [a, b, c, d] = ipv4;
    let address = format!("SF1-{a:02X}{b:02X}-{c:02X}{d:02X}");
    if port == 7000 { address } else { format!("{address}-{port:04X}") }
}

/// Read a room code (default/custom port) or a raw IPv4 `IP:PORT` endpoint.
/// Codes use the uppercase `SF1-` prefix and exactly four hexadecimal digits
/// per word (either digit case). Whitespace, malformed fields and port zero
/// are rejected. This parses addresses only; it performs no network I/O.
pub fn decode_room_code(input: &str) -> Option<([u8; 4], u16)> {
    let Some(code) = input.strip_prefix("SF1-") else {
        let address: std::net::SocketAddrV4 = input.parse().ok()?;
        return (address.port() != 0).then_some((address.ip().octets(), address.port()));
    };
    let mut parts = code.split('-');
    let high = parts.next()?;
    let low = parts.next()?;
    if high.len() != 4 || low.len() != 4
        || !high.bytes().all(|b| b.is_ascii_hexdigit())
        || !low.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let port = match parts.next() {
        None => 7000,
        Some(port) if port.len() == 4 && port.bytes().all(|b| b.is_ascii_hexdigit()) => {
            u16::from_str_radix(port, 16).ok()?
        }
        Some(_) => return None,
    };
    if parts.next().is_some() || port == 0 {
        return None;
    }
    let high = u16::from_str_radix(high, 16).ok()?;
    let low = u16::from_str_radix(low, 16).ok()?;
    Some((((u32::from(high) << 16) | u32::from(low)).to_be_bytes(), port))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Ethernet,
    Wifi,
}

pub fn detect_link(adapter: &str) -> Link {
    let name = adapter.to_ascii_lowercase();
    if name.contains("wi-fi") || name.contains("wifi") || name.contains("wireless") {
        Link::Wifi
    } else {
        Link::Ethernet
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind {
    Versus,
    Spectate,
    Dramatic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Room {
    pub code: String,
    pub host: String,
    pub region: String,
    pub ping_ms: u16,
    pub rollback_frames: u8,
    pub jitter_ms: u16,
    pub link: Link,
    pub kind: RoomKind,
    pub preset: RulesetPreset,
    pub in_match: bool,
    pub spectators: u8,
    pub what_if: bool,
    pub bonus: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Browser {
    pub wired_only: bool,
    pub what_if: Option<bool>,
    pub bonus: Option<bool>,
}

impl Browser {
    pub fn visible<'a>(&self, rooms: &'a [Room]) -> Vec<&'a Room> {
        rooms
            .iter()
            .filter(|room| !self.wired_only || room.link == Link::Ethernet)
            .filter(|room| self.what_if.map(|want| room.what_if == want).unwrap_or(true))
            .filter(|room| self.bonus.map(|want| room.bonus == want).unwrap_or(true))
            .collect()
    }

    pub fn badge(room: &Room) -> &'static str {
        match (room.what_if, room.bonus) {
            (false, false) => "1987 Tournament 12",
            (true, true) => "25-Fighter / What-If",
            (true, false) => "What-If",
            (false, true) => "25-Fighter",
        }
    }

    pub fn accept(&self, room: &Room) -> bool {
        !self.wired_only || room.link == Link::Ethernet
    }

    pub fn save(&self) -> String {
        if self.wired_only { "wired-only=1" } else { "wired-only=0" }.into()
    }

    pub fn load(text: &str) -> Self {
        Self { wired_only: text.contains("wired-only=1"), what_if: None, bonus: None }
    }
}

/// Canonical current configuration, seed and engine identity, without assets.
///
/// The shared configuration encoding excludes currently inert local scheme
/// selectors. Complete room compatibility additionally uses the synchronized
/// setup and actual bank/art identities in [`SyncedMatchRules::config_hash`].
pub fn config_hash(config: &MatchConfig, seed: u64) -> u64 {
    use crate::canonical::{Canonical, CanonicalHasher};
    let mut out = CanonicalHasher::new();
    out.tag("sf1-port/config/v1");
    crate::version::protocol_version_hash().encode(&mut out);
    config.encode(&mut out);
    seed.encode(&mut out);
    out.finish()
}
