//! Pure room membership and measured handshakes at the session boundary.
//!
//! Transport routes name addresses, never player roles. The caller supplies
//! display-frame time and consumes commands/events; no socket or wall clock
//! is used here. Fight state and phase transitions remain with GameSession.

use std::collections::{BTreeMap, VecDeque};

use super::protocol::{HandshakePolicy, LobbyPhase, NetPacket, RejectReason, RttEstimator, SyncedMatchRules};
use super::transport::{
    NetTransportCommand, NetTransportEvent, RoomAnnouncement, MAX_ROUTES, MAX_SPECTATORS,
};
use super::{Link, RoomKind};

const RETRY_FRAMES: u32 = 30;
const TIMEOUT_FRAMES: u32 = 300;
const MAX_QUEUED: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ConnectionEvent {
    Ready {
        rules: SyncedMatchRules,
        local_side: Option<usize>,
        delay: u8,
        max_rollback: u8,
        generation: u32,
        frame_base: u32,
        phase: LobbyPhase,
        picker: u8,
    },
    Rejected {
        reason: RejectReason,
    },
    PeerLeft {
        peer: u8,
        kind: RoomKind,
    },
    Packet {
        from: u8,
        packet: NetPacket,
    },
    SpectatorJoined {
        peer: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PeerInfo {
    pub tag: [u8; 12],
    pub link: Link,
    pub rtt_ms: u32,
    pub jitter_ms: u32,
}

#[derive(Clone, Copy, Debug)]
struct Probe {
    seq: u32,
    sent_ms: u32,
    last_sent: u32,
}

#[derive(Clone, Debug)]
struct Member {
    hello: Option<NetPacket>,
    kind: RoomKind,
    tag: [u8; 12],
    link: Link,
    admitted: bool,
    delay: Option<u8>,
    joined_at: u32,
    last_seen: u32,
    last_probe: u32,
    probe: Option<Probe>,
    timing: RttEstimator,
}

impl Member {
    fn new(kind: RoomKind, tag: [u8; 12], link: Link, frame: u32) -> Self {
        Self {
            hello: None,
            kind,
            tag,
            link,
            admitted: false,
            delay: None,
            joined_at: frame,
            last_seen: frame,
            last_probe: frame,
            probe: None,
            timing: RttEstimator::default(),
        }
    }
}

/// A room connection, separate from the Fight and its rollback bookkeeping.
#[derive(Clone, Debug)]
pub(crate) struct Connection {
    hosting: bool,
    kind: RoomKind,
    rules: SyncedMatchRules,
    tag: [u8; 12],
    announcement: Option<(u16, RoomAnnouncement)>,
    input_delay: Option<u8>,
    max_rollback: u8,
    generation: u32,
    frame_base: u32,
    phase: LobbyPhase,
    picker: u8,
    last_welcome: u32,
    time_ms: Option<u32>,
    request_id: u64,
    local_side: Option<usize>,
    local_link: Link,
    bound: bool,
    ready: bool,
    closed: bool,
    started: Option<u32>,
    last_hello: u32,
    next_probe: u32,
    members: BTreeMap<u8, Member>,
    commands: VecDeque<NetTransportCommand>,
    events: VecDeque<ConnectionEvent>,
}

impl Connection {
    pub(crate) fn host(
        rules: SyncedMatchRules,
        announcement: RoomAnnouncement,
        port: u16,
        input_delay: Option<u8>,
    ) -> Self {
        Self {
            hosting: true,
            kind: rules.room_kind(),
            rules,
            tag: announcement.host,
            max_rollback: announcement.max_rollback.clamp(1, 8),
            announcement: Some((port, announcement)),
            input_delay: input_delay.map(|f| f.min(4)),
            generation: 0,
            frame_base: 0,
            phase: LobbyPhase::Fighters,
            picker: 0,
            last_welcome: 0,
            time_ms: None,
            request_id: 0,
            local_side: None,
            local_link: Link::Wifi,
            bound: false,
            ready: false,
            closed: false,
            started: None,
            last_hello: 0,
            next_probe: 0,
            members: BTreeMap::new(),
            commands: VecDeque::new(),
            events: VecDeque::new(),
        }
    }

    pub(crate) fn join(
        local_rules: SyncedMatchRules,
        tag: [u8; 12],
        kind: RoomKind,
        request_id: u64,
    ) -> Self {
        Self {
            hosting: false,
            kind,
            rules: local_rules,
            tag,
            announcement: None,
            input_delay: None,
            max_rollback: 8,
            generation: 0,
            frame_base: 0,
            phase: LobbyPhase::Fighters,
            picker: 0,
            last_welcome: 0,
            time_ms: None,
            request_id,
            local_side: None,
            local_link: Link::Wifi,
            bound: false,
            ready: false,
            closed: false,
            started: None,
            last_hello: 0,
            next_probe: 0,
            members: BTreeMap::new(),
            commands: VecDeque::new(),
            events: VecDeque::new(),
        }
    }

    pub(crate) fn is_host(&self) -> bool {
        self.hosting
    }

    pub(crate) fn set_time_ms(&mut self, milliseconds: u32) { self.time_ms = Some(milliseconds); }
    pub(crate) fn connected(&self) -> bool {
        self.ready && !self.closed
    }
    pub(crate) fn rules(&self) -> &SyncedMatchRules {
        &self.rules
    }
    pub(crate) fn local_link(&self) -> Link {
        self.local_link
    }
    pub(crate) fn generation(&self) -> u32 {
        self.generation
    }
    /// Initial room identity comes from the session's lifetime counter, so
    /// old datagrams cannot become current again when a room is reopened.
    pub(crate) fn set_epoch(&mut self, generation: u32, frame_base: u32) -> bool {
        if !self.hosting || self.closed || self.bound
            || frame_base == u32::MAX { return false; }
        self.generation = generation;
        self.frame_base = frame_base;
        true
    }

    /// Phase metadata is also used when a spectator first joins mid-match.
    pub(crate) fn set_lobby_phase(&mut self, phase: LobbyPhase, picker: u8) -> bool {
        if !self.hosting || self.closed || picker > 1 { return false; }
        self.phase = phase;
        self.picker = picker;
        true
    }

    /// The host starts a new match generation. Retrying the same Welcome is
    /// harmless: clients accept each newer generation once. The session must
    /// reserve a base beyond every input it has sent for the previous match.
    pub(crate) fn advance_epoch(&mut self, rules: SyncedMatchRules, frame_base: u32,
        phase: LobbyPhase, picker: u8) -> Option<u32>
    {
        if !self.hosting || !self.connected() || picker > 1
            || rules.room_kind() != self.kind || !rules.supports_online()
            || frame_base <= self.frame_base || frame_base == u32::MAX { return None; }
        let generation = self.generation.checked_add(1)?;
        let delay = self.player_peer().and_then(|peer| self.members.get(&peer)?.delay)?;
        self.generation = generation;
        self.frame_base = frame_base;
        self.phase = phase;
        self.picker = picker;
        self.rules = rules;
        self.resend_welcomes();
        self.emit(ConnectionEvent::Ready { rules, local_side: self.local_side, delay,
            max_rollback: self.max_rollback, generation, frame_base, phase, picker });
        self.advertise();
        Some(generation)
    }
    pub(crate) fn kind(&self) -> RoomKind {
        self.kind
    }

    pub(crate) fn player_peer(&self) -> Option<u8> {
        if self.closed {
            return None;
        }
        if !self.hosting {
            return self.ready.then_some(0);
        }
        self.members.iter().find_map(|(&route, member)| {
            (member.admitted && member.kind.is_combatant()).then_some(route)
        })
    }

    pub(crate) fn spectator_peers(&self) -> Vec<u8> {
        self.members
            .iter()
            .filter_map(|(&route, member)| {
                (member.admitted && member.kind == RoomKind::Spectate).then_some(route)
            })
            .collect()
    }

    pub(crate) fn peer_info(&self, route: u8) -> Option<PeerInfo> {
        let member = self.members.get(&route)?;
        Some(PeerInfo {
            tag: member.tag,
            link: member.link,
            rtt_ms: member.timing.rtt_ms(),
            jitter_ms: member.timing.jitter_ms(),
        })
    }

    pub(crate) fn player_info(&self) -> Option<PeerInfo> {
        self.peer_info(self.player_peer()?)
    }

    pub(crate) fn drain_commands(&mut self) -> Vec<NetTransportCommand> {
        self.commands.drain(..).collect()
    }

    pub(crate) fn drain_events(&mut self) -> Vec<ConnectionEvent> {
        self.events.drain(..).collect()
    }

    pub(crate) fn send_to(&mut self, peer: u8, packet: NetPacket) {
        if self.closed
            || !self.bound
            || peer >= MAX_ROUTES
            || (self.hosting && peer == 0)
            || (!self.hosting && peer != 0)
        {
            return;
        }
        self.command(NetTransportCommand::SendPacket {
            to_peer: peer,
            bytes: packet.to_bytes(),
        });
    }

    pub(crate) fn send_to_player(&mut self, packet: NetPacket) {
        if let Some(peer) = self.player_peer() {
            self.send_to(peer, packet);
        }
    }

    /// Called when synchronized picks change; both policy and discovery then
    /// identify the same rules. Physical/presentation settings live elsewhere.
    pub(crate) fn set_rules(&mut self, rules: SyncedMatchRules) {
        self.rules = rules;
        self.advertise();
    }

    pub(crate) fn set_in_match(&mut self, in_match: bool) {
        if let Some((_, room)) = self.announcement.as_mut() {
            if room.in_match == in_match {
                return;
            }
            room.in_match = in_match;
            self.advertise();
        }
    }

    pub(crate) fn receive(&mut self, event: &NetTransportEvent, frame: u32) {
        if self.closed {
            return;
        }
        self.started.get_or_insert(frame);
        match event {
            NetTransportEvent::Bound { link, .. } => {
                self.local_link = *link;
                let first = !self.bound;
                self.bound = true;
                if self.hosting {
                    self.advertise();
                } else if first {
                    self.members
                        .insert(0, Member::new(RoomKind::Versus, [0; 12], Link::Wifi, frame));
                    self.hello(frame);
                    self.probe(0, frame);
                }
            }
            NetTransportEvent::PacketReceived {
                from_peer, bytes, ..
            } => {
                if !self.bound
                    || *from_peer >= MAX_ROUTES
                    || (self.hosting && *from_peer == 0)
                    || (!self.hosting && *from_peer != 0)
                {
                    return;
                }
                if let Some(packet) = NetPacket::from_bytes(bytes) {
                    self.packet(*from_peer, packet, frame);
                }
            }
            NetTransportEvent::TransportError { .. }
            | NetTransportEvent::RoomsDiscovered { .. } => {}
        }
    }

    pub(crate) fn tick(&mut self, frame: u32) {
        if self.closed {
            return;
        }
        let started = *self.started.get_or_insert(frame);
        if !self.hosting && !self.ready && frame.wrapping_sub(started) >= TIMEOUT_FRAMES {
            self.client_left();
            return;
        }
        if !self.bound {
            return;
        }
        if !self.hosting && !self.ready && frame.wrapping_sub(self.last_hello) >= RETRY_FRAMES {
            self.hello(frame);
        }
        if self.hosting && frame.wrapping_sub(self.last_welcome) >= RETRY_FRAMES {
            self.last_welcome = frame;
            self.resend_welcomes();
        }
        let routes: Vec<_> = self.members.keys().copied().collect();
        for route in routes {
            let member = &self.members[&route];
            if frame.wrapping_sub(member.last_seen) >= TIMEOUT_FRAMES
                || (!member.admitted && frame.wrapping_sub(member.joined_at) >= TIMEOUT_FRAMES)
            {
                if self.hosting {
                    let left = self.members.remove(&route).expect("listed member");
                    if left.admitted {
                        if left.kind.is_combatant() {
                            self.ready = false;
                            self.local_side = None;
                        }
                        self.emit(ConnectionEvent::PeerLeft { peer: route, kind: left.kind });
                        self.advertise();
                    }
                } else {
                    self.client_left();
                    return;
                }
            } else if member.probe.map_or_else(
                || frame.wrapping_sub(member.last_probe) >= RETRY_FRAMES,
                |probe| frame.wrapping_sub(probe.last_sent) >= RETRY_FRAMES,
            ) {
                self.probe(route, frame);
            }
        }
    }

    fn packet(&mut self, route: u8, packet: NetPacket, frame: u32) {
        if let Some(member) = self.members.get_mut(&route) {
            member.last_seen = frame;
        }
        match packet {
            NetPacket::Ping { seq, sent_ms, link } => {
                if let Some(member) = self.members.get_mut(&route) {
                    member.link = link;
                }
                self.send_to(
                    route,
                    NetPacket::Pong {
                        seq,
                        echo_ms: sent_ms,
                        link: self.local_link,
                    },
                );
            }
            NetPacket::Pong { seq, echo_ms, link } => {
                let Some(member) = self.members.get_mut(&route) else {
                    return;
                };
                let Some(probe) = member
                    .probe
                    .filter(|probe| probe.seq == seq && probe.sent_ms == echo_ms)
                else {
                    return;
                };
                if member.timing.observe(self.time_ms.unwrap_or_else(|| milliseconds(frame)), probe.sent_ms) {
                    member.probe = None;
                    member.link = link;
                    if self.hosting && !member.admitted {
                        self.admit(route);
                    }
                } else {
                    // A previously lost probe must not prevent fresh RTT
                    // measurements forever after other traffic recovers.
                    member.probe = None;
                }
            }
            hello @ NetPacket::Hello { .. } if self.hosting => {
                self.receive_hello(route, hello, frame)
            }
            welcome @ NetPacket::Welcome { .. } if !self.hosting => self.receive_welcome(welcome),
            packet if self.authorized(route, &packet) => self.emit(ConnectionEvent::Packet {
                from: route,
                packet,
            }),
            _ => {}
        }
    }

    fn receive_hello(&mut self, route: u8, hello: NetPacket, frame: u32) {
        let NetPacket::Hello {
            request_id,
            kind,
            tag,
            link,
            ..
        } = &hello
        else {
            return;
        };
        if let Some(existing) = self.members.get(&route) {
            if existing.admitted {
                let same = matches!(existing.hello, Some(NetPacket::Hello { request_id: old, .. }) if old == *request_id)
                    && existing.kind == *kind;
                let reply = self
                    .policy()
                    .welcome(&hello, existing.timing.rtt_ms(), !same);
                if let Some(mut reply) = reply {
                    if same {
                        if let (NetPacket::Welcome { assigned_delay, .. }, Some(delay)) = (&mut reply, existing.delay) {
                            *assigned_delay = delay;
                        }
                    }
                    self.send_to(route, reply);
                }
                return;
            }
        }
        let Some(reply) = self.policy().welcome(&hello, 0, self.full(*kind, route)) else {
            return;
        };
        if matches!(
            reply,
            NetPacket::Welcome {
                accepted: false,
                ..
            }
        ) {
            self.send_to(route, reply);
            self.members.remove(&route);
            return;
        }
        let same_pending = self.members.get(&route).is_some_and(|member| {
            matches!(member.hello, Some(NetPacket::Hello { request_id: old, .. }) if old == *request_id)
                && member.kind == *kind
        });
        if !same_pending {
            let mut member = Member::new(*kind, *tag, *link, frame);
            member.hello = Some(hello);
            self.members.insert(route, member);
            self.probe(route, frame);
        }
    }

    fn admit(&mut self, route: u8) {
        let Some(member) = self.members.get(&route) else {
            return;
        };
        let Some(mut hello) = member.hello.clone() else {
            return;
        };
        if let NetPacket::Hello { link, .. } = &mut hello {
            *link = member.link;
        }
        let kind = member.kind;
        let Some(reply) =
            self.policy()
                .welcome(&hello, member.timing.rtt_ms(), self.full(kind, route))
        else {
            return;
        };
        let NetPacket::Welcome {
            accepted,
            assigned_delay,
            ..
        } = reply
        else {
            return;
        };
        self.send_to(route, reply);
        if !accepted {
            self.members.remove(&route);
            return;
        }
        let member = self.members.get_mut(&route).expect("measured member");
        member.admitted = true;
        member.delay = Some(assigned_delay);
        if kind.is_combatant() {
            self.ready = true;
            self.local_side = Some(0);
            self.emit(ConnectionEvent::Ready {
                rules: self.rules,
                local_side: self.local_side,
                delay: assigned_delay,
                max_rollback: self.max_rollback,
                generation: self.generation,
                frame_base: self.frame_base,
                phase: self.phase,
                picker: self.picker,
            });
        } else {
            self.emit(ConnectionEvent::SpectatorJoined { peer: route });
        }
        self.advertise();
    }

    fn receive_welcome(&mut self, welcome: NetPacket) {
        let NetPacket::Welcome {
            request_id,
            generation,
            frame_base,
            phase,
            picker,
            accepted,
            reason,
            config,
            assigned_delay,
            max_rollback,
            host_tag,
            ..
        } = welcome
        else {
            return;
        };
        if request_id != self.request_id {
            return;
        }
        if self.ready && !newer(generation, self.generation) {
            return;
        }
        let reason = if config.protocol_hash != self.rules.protocol_hash {
            RejectReason::VersionMismatch
        } else if config.bank_hash != self.rules.bank_hash
            || config.art_pack_hash != self.rules.art_pack_hash
        {
            RejectReason::PackHashMismatch
        } else if accepted && (!config.supports_online()
            || (self.kind.is_combatant() && self.kind != config.room_kind())
            || (self.ready && self.rules.room_kind() != config.room_kind()))
        {
            RejectReason::UnsupportedMode
        } else {
            reason
        };
        if !accepted || reason != RejectReason::None {
            self.emit(ConnectionEvent::Rejected { reason });
            self.close_client();
            return;
        }
        self.rules = config;
        self.generation = generation;
        self.frame_base = frame_base;
        self.phase = phase;
        self.picker = picker;
        self.max_rollback = max_rollback;
        self.input_delay = Some(assigned_delay);
        self.local_side = self.kind.is_combatant().then_some(1);
        self.ready = true;
        if let Some(host) = self.members.get_mut(&0) {
            host.tag = host_tag;
            host.kind = config.room_kind();
            host.admitted = true;
        }
        self.emit(ConnectionEvent::Ready {
            rules: config,
            local_side: self.local_side,
            delay: assigned_delay,
            max_rollback,
            generation,
            frame_base,
            phase,
            picker,
        });
    }

    fn authorized(&self, route: u8, packet: &NetPacket) -> bool {
        let Some(member) = self.members.get(&route).filter(|member| member.admitted) else {
            return false;
        };
        if self.hosting {
            match packet {
                NetPacket::Input { .. } => member.kind.is_combatant(),
                NetPacket::LobbyPick {
                    generation, side, ..
                } => {
                    member.kind == RoomKind::Versus && *generation == self.generation && *side == 1
                }
                NetPacket::AfterMatchVote { generation, .. } => {
                    member.kind.is_combatant() && *generation == self.generation
                }
                NetPacket::DramaticAllyPick { generation, revision, .. } => {
                    member.kind == RoomKind::Dramatic && *generation == self.generation
                        && self.phase == LobbyPhase::Fighters
                        && crate::dramatic::DramaticRules::supports_style(*revision)
                }
                NetPacket::SpectatorAck { generation, .. } => {
                    member.kind == RoomKind::Spectate && *generation == self.generation
                }
                _ => false,
            }
        } else {
            match packet {
                NetPacket::Input { .. } => self.kind.is_combatant(),
                NetPacket::LobbyPick { generation, .. } => {
                    self.rules.room_kind() == RoomKind::Versus && *generation == self.generation
                }
                NetPacket::AfterMatchVote { generation, .. } => {
                    self.kind.is_combatant() && *generation == self.generation
                }
                NetPacket::DramaticSetup { generation, rules, .. } => {
                    self.rules.room_kind() == RoomKind::Dramatic && *generation == self.generation
                        && self.phase == LobbyPhase::Fighters && rules.supports_online()
                        && rules.protocol_hash == self.rules.protocol_hash
                        && rules.bank_hash == self.rules.bank_hash
                        && rules.art_pack_hash == self.rules.art_pack_hash
                }
                NetPacket::SpectatorChunk { generation, .. } => {
                    self.kind == RoomKind::Spectate && *generation == self.generation
                }
                _ => false,
            }
        }
    }

    fn full(&self, kind: RoomKind, except: u8) -> bool {
        match kind {
            RoomKind::Versus | RoomKind::Dramatic => self
                .members
                .iter()
                .any(|(&route, member)| route != except && member.admitted && member.kind.is_combatant()),
            RoomKind::Spectate => {
                self.members
                    .iter()
                    .filter(|(&route, member)| {
                        route != except && member.admitted && member.kind == kind
                    })
                    .count()
                    >= usize::from(MAX_SPECTATORS)
            }
        }
    }

    fn policy(&self) -> HandshakePolicy {
        HandshakePolicy {
            rules: self.rules,
            wired_only: self
                .announcement
                .as_ref()
                .is_some_and(|(_, room)| room.wired_only),
            host_tag: self.tag,
            max_rollback: self.max_rollback,
            generation: self.generation,
            frame_base: self.frame_base,
            phase: self.phase,
            picker: self.picker,
            input_delay: self.input_delay,
        }
    }

    fn hello(&mut self, frame: u32) {
        self.last_hello = frame;
        self.send_to(
            0,
            NetPacket::Hello {
                request_id: self.request_id,
                tag: self.tag,
                kind: self.kind,
                link: self.local_link,
                protocol_hash: self.rules.protocol_hash,
                config_hash: self.rules.config_hash(),
                bank_hash: self.rules.bank_hash,
                art_pack_hash: self.rules.art_pack_hash,
            },
        );
    }

    fn resend_welcomes(&mut self) {
        let replies: Vec<_> = self.members.iter().filter_map(|(&peer, member)| {
            if !member.admitted { return None; }
            let mut hello = member.hello.clone()?;
            if let NetPacket::Hello { link, .. } = &mut hello { *link = member.link; }
            let mut reply = self.policy().welcome(&hello, member.timing.rtt_ms(), false)?;
            if let (NetPacket::Welcome { assigned_delay, .. }, Some(delay)) = (&mut reply, member.delay) {
                *assigned_delay = delay;
            }
            Some((peer, reply))
        }).collect();
        for (peer, reply) in replies { self.send_to(peer, reply); }
    }

    fn probe(&mut self, route: u8, frame: u32) {
        let Some(member) = self.members.get_mut(&route) else {
            return;
        };
        // A retry is a new measurement. Reusing an unanswered timestamp
        // would count the retry interval as RTT and inflate AUTO input delay.
        // Correlation also makes delayed replies to earlier probes harmless.
        let seq = self.next_probe;
        self.next_probe = self.next_probe.wrapping_add(1);
        let probe = Probe {
            seq,
            sent_ms: self.time_ms.unwrap_or_else(|| milliseconds(frame)),
            last_sent: frame,
        };
        member.last_probe = frame;
        member.probe = Some(probe);
        self.send_to(
            route,
            NetPacket::Ping {
                seq: probe.seq,
                sent_ms: probe.sent_ms,
                link: self.local_link,
            },
        );
    }

    fn advertise(&mut self) {
        if !self.bound {
            return;
        }
        let spectators = self.spectator_peers().len() as u8;
        let Some((port, room)) = self.announcement.as_mut() else {
            return;
        };
        room.link = self.local_link;
        room.spectators = spectators;
        room.preset = self.rules.config.preset;
        room.config_hash = self.rules.config_hash();
        room.bank_hash = self.rules.bank_hash;
        room.art_pack_hash = self.rules.art_pack_hash;
        room.protocol_hash = self.rules.protocol_hash;
        let command = NetTransportCommand::BindHost {
            port: *port,
            room: room.clone(),
        };
        self.command(command);
    }

    fn client_left(&mut self) {
        self.emit(ConnectionEvent::PeerLeft { peer: 0, kind: self.rules.room_kind() });
        self.close_client();
    }

    fn close_client(&mut self) {
        self.command(NetTransportCommand::Close);
        self.closed = true;
        self.ready = false;
        self.members.clear();
    }

    fn command(&mut self, command: NetTransportCommand) {
        if self.commands.len() < MAX_QUEUED {
            self.commands.push_back(command);
        }
    }

    fn emit(&mut self, event: ConnectionEvent) {
        if self.events.len() < MAX_QUEUED {
            self.events.push_back(event);
        }
    }
}

fn milliseconds(frame: u32) -> u32 {
    (u64::from(frame) * 1000 / 60) as u32
}
fn newer(value: u32, previous: u32) -> bool {
    value != previous && value.wrapping_sub(previous) < (1 << 31)
}
