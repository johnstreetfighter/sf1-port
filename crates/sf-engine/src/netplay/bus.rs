//! Deterministic, bounded in-memory transport for real session tests.

use std::collections::VecDeque;

use super::{decode_room_code, encode_room_code, Link, Room};
use super::protocol::{tag_from_str, tag_to_string, MAX_PACKET_BYTES};
use super::transport::{NetTransportCommand, NetTransportEvent, RoomAnnouncement, MAX_ROUTES, MAX_SPECTATORS};

const MAX_PENDING: usize = 4096;
const MAX_EVENTS: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualNetConfig {
    pub latency_ms: u32,
    pub jitter_ms: u32,
    pub drop_percent: u8,
    pub duplicate_percent: u8,
    pub reorder_ms: u32,
    pub seed: u64,
}

impl Default for VirtualNetConfig {
    fn default() -> Self {
        Self { latency_ms: 0, jitter_ms: 0, drop_percent: 0, duplicate_percent: 0, reorder_ms: 0, seed: 1 }
    }
}

/// An exact scripted datagram fate, overriding percentage/jitter settings for
/// that SendPacket. Copies are clamped to two; zero copies also drops it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketFault { pub delay_ms: u32, pub drop: bool, pub copies: u8 }

#[derive(Clone, Debug)]
struct Endpoint {
    open: bool,
    link: Link,
    port: u16,
    room: Option<RoomAnnouncement>,
    events: VecDeque<NetTransportEvent>,
}

#[derive(Clone, Debug)]
struct Datagram { from: u8, to: u8, bytes: Vec<u8>, due: u64, order: u64 }

/// Sixteen bounded transport routes, with the host at 0. The session maps
/// routes to player/spectator roles and admits at most six room members.
/// No sockets, clocks or GameSession internals are used. Queues are bounded so adversarial
/// fault scripts cannot grow memory indefinitely. Sessions drain events after
/// calling step_ms and feed their emitted commands back through submit.
#[derive(Clone, Debug)]
pub struct VirtualNetBus {
    config: VirtualNetConfig,
    endpoints: [Endpoint; MAX_ROUTES as usize],
    now: u64,
    rng: u64,
    serial: u64,
    pending: Vec<Datagram>,
    faults: VecDeque<PacketFault>,
}

impl VirtualNetBus {
    pub fn new(config: VirtualNetConfig) -> Self {
        Self { config, endpoints: std::array::from_fn(|id| Endpoint {
            open: false, link: Link::Ethernet, port: 7000 + id as u16, room: None, events: VecDeque::new(),
        }), now: 0, rng: config.seed.max(1), serial: 0, pending: Vec::new(), faults: VecDeque::new() }
    }

    pub fn now_ms(&self) -> u64 { self.now }

    pub fn set_link(&mut self, peer: u8, link: Link) {
        if let Some(endpoint) = self.endpoints.get_mut(usize::from(peer)) { endpoint.link = link; }
    }

    pub fn set_fault_script(&mut self, faults: Vec<PacketFault>) {
        self.faults = faults.into_iter().take(MAX_PENDING).collect();
    }

    pub fn submit(&mut self, peer: u8, commands: &[NetTransportCommand]) {
        if peer >= MAX_ROUTES { return; }
        for command in commands {
            match command {
                NetTransportCommand::BindHost { port, room } => {
                    if peer != 0 || *port == 0 || room.spectators > MAX_SPECTATORS
                        || !(1..=8).contains(&room.max_rollback)
                        || tag_from_str(&tag_to_string(room.host)) != Some(room.host)
                    { self.error(peer, "INVALID HOST ROOM"); continue; }
                    let reused = self.endpoints[0].open && self.endpoints[0].port == *port;
                    if self.endpoints[0].open && !reused { self.close(0); }
                    self.endpoints[0].open = true;
                    self.endpoints[0].port = *port;
                    let mut advertised = room.clone();
                    advertised.link = self.endpoints[0].link;
                    self.endpoints[0].room = Some(advertised);
                    if !reused {
                        self.emit(peer, NetTransportEvent::Bound {
                            local_addr: self.address(peer), room_code: encode_room_code(self.ip(peer), *port), link: self.endpoints[0].link,
                        });
                    }
                }
                NetTransportCommand::ConnectPeer { addr } => {
                    let host = &self.endpoints[0];
                    if peer == 0 || !host.open || host.room.is_none()
                        || decode_room_code(addr) != Some((self.ip(0), host.port))
                    { self.error(peer, "ROOM NOT FOUND"); continue; }
                    let room_code = encode_room_code(self.ip(0), host.port);
                    self.endpoints[usize::from(peer)].open = true;
                    self.emit(peer, NetTransportEvent::Bound {
                        local_addr: self.address(peer), room_code, link: self.endpoints[usize::from(peer)].link,
                    });
                }
                NetTransportCommand::SendPacket { to_peer, bytes } => self.send(peer, *to_peer, bytes),
                NetTransportCommand::RefreshRooms => {
                    let host = &self.endpoints[0];
                    let rooms = if let Some(room) = host.room.as_ref().filter(|r| host.open && r.public) {
                        vec![Room {
                            code: encode_room_code(self.ip(0), host.port), host: tag_to_string(room.host), region: String::new(),
                            ping_ms: 0, rollback_frames: room.max_rollback, jitter_ms: 0, link: room.link, kind: room.kind,
                            spectators: room.spectators,
                            what_if: false, bonus: false, preset: room.preset, in_match: room.in_match,
                        }]
                    } else { Vec::new() };
                    self.emit(peer, NetTransportEvent::RoomsDiscovered { rooms });
                }
                NetTransportCommand::Close => self.close(peer),
            }
        }
    }

    pub fn drain_events(&mut self, peer: u8) -> Vec<NetTransportEvent> {
        self.endpoints.get_mut(usize::from(peer)).map(|p| p.events.drain(..).collect()).unwrap_or_default()
    }

    pub fn step_ms(&mut self, delta: u32) {
        self.now = self.now.saturating_add(u64::from(delta));
        self.pending.sort_by_key(|packet| (packet.due, packet.order));
        let count = self.pending.partition_point(|packet| packet.due <= self.now);
        let delivered: Vec<_> = self.pending.drain(..count).collect();
        for packet in delivered {
            if self.endpoints[usize::from(packet.to)].open {
                self.emit(packet.to, NetTransportEvent::PacketReceived {
                    from_peer: packet.from, addr: self.address(packet.from), bytes: packet.bytes,
                });
            }
        }
    }

    fn ip(&self, peer: u8) -> [u8; 4] { [127, 0, 0, peer + 1] }
    fn address(&self, peer: u8) -> String { format!("127.0.0.{}:{}", peer + 1, self.endpoints[usize::from(peer)].port) }

    fn emit(&mut self, peer: u8, event: NetTransportEvent) {
        let events = &mut self.endpoints[usize::from(peer)].events;
        if events.len() == MAX_EVENTS { events.pop_front(); }
        events.push_back(event);
    }

    fn error(&mut self, peer: u8, message: &str) {
        self.emit(peer, NetTransportEvent::TransportError { message: message.into() });
    }

    fn close(&mut self, peer: u8) {
        let endpoint = &mut self.endpoints[usize::from(peer)];
        endpoint.open = false;
        endpoint.room = None;
        endpoint.events.clear();
        // SendPacket already dispatched these datagrams. Closing the source
        // cannot recall UDP packets in flight to another open destination.
        self.pending.retain(|packet| packet.to != peer);
    }

    fn random(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn send(&mut self, from: u8, to: u8, bytes: &[u8]) {
        if to >= MAX_ROUTES || !self.endpoints[usize::from(from)].open || !self.endpoints[usize::from(to)].open
            || bytes.is_empty() || bytes.len() > MAX_PACKET_BYTES
        { self.error(from, "INVALID PACKET DESTINATION OR SIZE"); return; }
        let scripted = self.faults.pop_front();
        let (drop, copies) = if let Some(fault) = scripted {
            (fault.drop, fault.copies.min(2))
        } else {
            let drop = self.random() % 100 < u64::from(self.config.drop_percent.min(100));
            let duplicate = self.random() % 100 < u64::from(self.config.duplicate_percent.min(100));
            (drop, if duplicate { 2 } else { 1 })
        };
        if drop { return; }
        for _ in 0..copies {
            if self.pending.len() >= MAX_PENDING { self.error(from, "VIRTUAL QUEUE FULL"); break; }
            let delay = if let Some(fault) = scripted { u64::from(fault.delay_ms) } else {
                let spread = u64::from(self.config.jitter_ms);
                let jitter = (self.random() % (2 * spread + 1)) as i64 - spread as i64;
                let delay = (i64::from(self.config.latency_ms) + jitter).max(0) as u64;
                delay + self.random() % (u64::from(self.config.reorder_ms) + 1)
            };
            self.pending.push(Datagram { from, to, bytes: bytes.to_vec(), due: self.now.saturating_add(delay), order: self.serial });
            self.serial = self.serial.wrapping_add(1);
        }
    }
}
