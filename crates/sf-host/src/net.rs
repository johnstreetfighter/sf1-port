//! Nonblocking host transport for the pure engine's netplay queues (NP7).

pub mod directory;

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::{Duration, Instant};

use sf_engine::config::RulesetPreset;
use sf_engine::netplay::protocol::{tag_from_str, tag_to_string, RttEstimator, MAX_PACKET_BYTES};
use sf_engine::netplay::transport::MAX_ROUTES;
use sf_engine::netplay::{
    decode_room_code, encode_room_code, Link, NetPacket, NetTransportCommand as Command,
    NetTransportEvent as Event, Room, RoomAnnouncement, RoomKind,
};

const IO_BUDGET: usize = 64;
const QUEUE_LIMIT: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdapterInfo {
    pub ip: Ipv4Addr,
    pub link: Link,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterQuery {
    Host { bind_ip: Ipv4Addr },
    Peer { target: Ipv4Addr, bind_ip: Ipv4Addr },
}
pub type AdapterProbe = fn(AdapterQuery) -> io::Result<AdapterInfo>;

struct AdapterWorker {
    tx: SyncSender<(u64, AdapterQuery)>,
    rx: Receiver<(u64, io::Result<AdapterInfo>)>,
}

impl AdapterWorker {
    fn new(probe: AdapterProbe) -> io::Result<Self> {
        let (tx, requests) = mpsc::sync_channel::<(u64, AdapterQuery)>(1);
        let (results, rx) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("sf-net-adapter".into())
            .spawn(move || {
                while let Ok((serial, query)) = requests.recv() {
                    let result = std::panic::catch_unwind(|| probe(query))
                        .unwrap_or_else(|_| Err(io::Error::other("ADAPTER QUERY FAILED")));
                    if results.send((serial, result)).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self { tx, rx })
    }
}

#[derive(Clone, Debug)]
pub struct NetDriverConfig {
    pub bind_ip: Ipv4Addr,
    pub link_override: Option<Link>,
    pub discovery_port: u16,
    pub discovery_targets: Vec<SocketAddrV4>,
    pub room_ttl: Duration,
    pub beacon_interval: Duration,
    pub route_ttl: Duration,
    pub directory_timeout: Duration,
    /// OS boundary injection for deterministic worker tests. The normal probe
    /// queries the actual route/media; link_override bypasses it completely.
    pub adapter_probe: AdapterProbe,
}

impl Default for NetDriverConfig {
    fn default() -> Self {
        Self {
            bind_ip: Ipv4Addr::UNSPECIFIED,
            link_override: None,
            discovery_port: 7001,
            discovery_targets: vec![SocketAddrV4::new(Ipv4Addr::BROADCAST, 7001)],
            room_ttl: Duration::from_secs(4),
            beacon_interval: Duration::from_secs(1),
            route_ttl: Duration::from_secs(10),
            directory_timeout: Duration::from_secs(2),
            adapter_probe: detect_adapter,
        }
    }
}

struct Route {
    addr: SocketAddrV4,
    seen: Instant,
}
struct Pending {
    addr: SocketAddrV4,
    bytes: Vec<u8>,
}
struct CachedRoom {
    room: Room,
    expires: Instant,
}
struct DiscoveryProbe {
    seq: u32,
    sent_ms: u32,
    epoch: u64,
    created: Instant,
    expected: Option<SocketAddrV4>,
    seen: Vec<SocketAddrV4>,
}
struct DiscoverySend {
    scanner: bool,
    addr: SocketAddrV4,
    bytes: Vec<u8>,
    created: Instant,
}
struct RoomTiming {
    rtt: RttEstimator,
    last_probe: Instant,
}

pub struct HostNetDriver {
    config: NetDriverConfig,
    socket: Option<UdpSocket>,
    hosting: bool,
    room: Option<RoomAnnouncement>,
    routes: [Option<Route>; MAX_ROUTES as usize],
    outgoing: VecDeque<Pending>,
    events: Vec<Event>,
    link: Option<Link>,
    advertised_ip: Ipv4Addr,
    pending_bound: bool,
    lobby_url: Option<String>,
    discovery: Option<UdpSocket>,
    scanner: Option<UdpSocket>,
    scanning: bool,
    last_beacon: Option<Instant>,
    last_probe: Option<Instant>,
    rooms: BTreeMap<SocketAddrV4, CachedRoom>,
    visible_rooms: Vec<Room>,
    notes: VecDeque<String>,
    adapter: Option<AdapterWorker>,
    adapter_serial: u64,
    adapter_pending: Option<(u64, AdapterQuery)>,
    adapter_busy: bool,
    clock_started: Instant,
    directory: Option<directory::DirectoryWorker>,
    directory_epoch: u64,
    directory_busy: bool,
    directory_fetch: bool,
    directory_last_fetch: Option<Instant>,
    directory_rooms: BTreeMap<SocketAddrV4, CachedRoom>,
    directory_published: Option<(String, u16)>,
    directory_withdrawals: VecDeque<(String, u16)>,
    directory_publish: bool,
    directory_last_publish: Option<Instant>,
    directory_job_fetch: bool,
    discovery_seq: u32,
    discovery_probes: VecDeque<DiscoveryProbe>,
    discovery_sends: VecDeque<DiscoverySend>,
    room_timing: BTreeMap<SocketAddrV4, RoomTiming>,
}

impl Default for HostNetDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl HostNetDriver {
    pub fn new() -> Self {
        Self::with_config(NetDriverConfig::default())
    }

    /// Deterministic localhost transport; production queries the OS instead.
    pub fn with_link_override(link: Link) -> Self {
        Self::with_config(NetDriverConfig {
            bind_ip: Ipv4Addr::LOCALHOST,
            link_override: Some(link),
            discovery_port: 0,
            discovery_targets: Vec::new(),
            ..NetDriverConfig::default()
        })
    }

    pub fn with_config(mut config: NetDriverConfig) -> Self {
        config.discovery_targets.truncate(32);
        let link = config.link_override;
        let advertised_ip = config.bind_ip;
        let adapter = if link.is_none() {
            AdapterWorker::new(config.adapter_probe).ok()
        } else {
            None
        };
        Self {
            config,
            socket: None,
            hosting: false,
            room: None,
            routes: std::array::from_fn(|_| None),
            outgoing: VecDeque::new(),
            events: Vec::new(),
            link,
            advertised_ip,
            pending_bound: false,
            lobby_url: None,
            discovery: None,
            scanner: None,
            scanning: false,
            last_beacon: None,
            last_probe: None,
            rooms: BTreeMap::new(),
            visible_rooms: Vec::new(),
            notes: VecDeque::new(),
            adapter,
            adapter_serial: 0,
            adapter_pending: None,
            adapter_busy: false,
            clock_started: Instant::now(),
            directory: None,
            directory_epoch: 0,
            directory_busy: false,
            directory_fetch: false,
            directory_last_fetch: None,
            directory_rooms: BTreeMap::new(),
            directory_published: None,
            directory_withdrawals: VecDeque::new(),
            directory_publish: false,
            directory_last_publish: None,
            directory_job_fetch: false,
            discovery_seq: 0,
            discovery_probes: VecDeque::new(),
            discovery_sends: VecDeque::new(),
            room_timing: BTreeMap::new(),
        }
    }

    pub fn local_addr(&self) -> Option<SocketAddrV4> {
        match self.socket.as_ref()?.local_addr().ok()? {
            SocketAddr::V4(addr) => Some(addr),
            _ => None,
        }
    }

    /// Caller-supplied engine timing, independent of simulation frames. Like
    /// the wire's millisecond timestamps, this wraps after u32::MAX ms.
    pub fn now_ms(&self) -> u32 {
        self.clock_started.elapsed().as_millis() as u32
    }

    pub fn discovery_addr(&self) -> Option<SocketAddrV4> {
        let socket = self
            .discovery
            .as_ref()
            .or(self.scanner.as_ref())
            .or_else(|| {
                self.shared_discovery()
                    .then_some(self.socket.as_ref())
                    .flatten()
            })?;
        match socket.local_addr().ok()? {
            SocketAddr::V4(addr) => Some(addr),
            _ => None,
        }
    }

    /// Optional LAN/directory failures are diagnostics, never fight failures.
    pub fn drain_notes(&mut self) -> Vec<String> {
        self.notes.drain(..).collect()
    }

    fn note(&mut self, message: impl Into<String>) {
        if self.notes.len() == 32 {
            self.notes.pop_front();
        }
        self.notes
            .push_back(message.into().chars().take(256).collect());
    }

    pub fn set_lobby_url(&mut self, url: Option<String>) {
        let url = url.filter(|value| !value.trim().is_empty());
        if self.lobby_url == url {
            return;
        }
        self.withdraw_directory();
        self.lobby_url = url;
        self.directory_epoch = self.directory_epoch.wrapping_add(1);
        self.directory_rooms.clear();
        self.directory_fetch = self.scanning;
        self.directory_last_fetch = None;
        self.directory_publish = true;
        self.directory_last_publish = None;
    }

    /// Never waits for network I/O. Reads/writes and queued commands are bounded
    /// each tick so a busy peer cannot monopolize the game's frame thread.
    pub fn poll(&mut self, commands: &[Command]) -> Vec<Event> {
        self.poll_adapter();
        for command in commands.iter().take(QUEUE_LIMIT) {
            if let Err(error) = self.command(command) {
                self.error(error.to_string());
            }
        }
        if commands.len() > QUEUE_LIMIT {
            self.error("TOO MANY NETWORK COMMANDS");
        }
        self.emit_bound();
        if !self.pending_bound {
            self.flush();
            self.receive();
        }
        self.lan_tick();
        self.directory_tick();
        let now = Instant::now();
        for route in &mut self.routes {
            if route
                .as_ref()
                .is_some_and(|route| now.duration_since(route.seen) > self.config.route_ttl)
            {
                *route = None;
            }
        }
        self.rooms.retain(|_, cached| cached.expires > now);
        self.directory_rooms
            .retain(|_, cached| cached.expires > now);
        self.room_timing.retain(|addr, _| {
            self.rooms.contains_key(addr) || self.directory_rooms.contains_key(addr)
        });
        self.publish_rooms(false);
        std::mem::take(&mut self.events)
    }

    fn error(&mut self, message: impl Into<String>) {
        if self.events.len() < QUEUE_LIMIT {
            self.events.push(Event::TransportError {
                message: message.into().chars().take(256).collect(),
            });
        }
    }

    fn close(&mut self) {
        self.withdraw_directory();
        self.socket = None;
        self.hosting = false;
        self.room = None;
        self.routes = std::array::from_fn(|_| None);
        self.outgoing.clear();
        self.events.clear();
        self.pending_bound = false;
        self.discovery = None;
        self.scanner = None;
        self.scanning = false;
        self.last_beacon = None;
        self.last_probe = None;
        self.rooms.clear();
        self.visible_rooms.clear();
        self.directory_epoch = self.directory_epoch.wrapping_add(1);
        self.directory_fetch = false;
        self.directory_last_fetch = None;
        self.directory_rooms.clear();
        self.directory_publish = false;
        self.directory_last_publish = None;
        self.discovery_probes.clear();
        self.discovery_sends.clear();
        self.room_timing.clear();
        self.adapter_serial = self.adapter_serial.wrapping_add(1);
        self.adapter_pending = None;
        self.link = self.config.link_override;
    }

    fn command(&mut self, command: &Command) -> io::Result<()> {
        match command {
            Command::BindHost { port, room } => {
                if !valid_announcement(room) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "INVALID ROOM ANNOUNCEMENT",
                    ));
                }
                if self.hosting && self.local_addr().is_some_and(|addr| addr.port() == *port) {
                    if !room.public {
                        self.withdraw_directory();
                        self.discovery_sends.retain(|send| send.scanner);
                    }
                    self.directory_publish = room.public;
                    self.directory_last_publish = None;
                    self.room = Some(room.clone());
                    self.last_beacon = None;
                    self.prepare_lan();
                    return Ok(());
                }
                self.close();
                let socket = UdpSocket::bind(SocketAddrV4::new(self.config.bind_ip, *port))?;
                socket.set_nonblocking(true)?;
                socket.set_broadcast(true)?; // also carries LAN beacons when the game uses port 7001
                self.socket = Some(socket);
                self.hosting = true;
                self.room = Some(room.clone());
                self.pending_bound = true;
                self.request_adapter(AdapterQuery::Host {
                    bind_ip: self.config.bind_ip,
                })?;
                self.prepare_lan();
            }
            Command::ConnectPeer { addr } => {
                let (ip, port) = decode_room_code(addr).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "INVALID ROOM ADDRESS")
                })?;
                self.close();
                let addr = SocketAddrV4::new(Ipv4Addr::from(ip), port);
                let socket = UdpSocket::bind(SocketAddrV4::new(self.config.bind_ip, 0))?;
                socket.set_nonblocking(true)?;
                socket.connect(addr)?;
                self.socket = Some(socket);
                self.routes[0] = Some(Route {
                    addr,
                    seen: Instant::now(),
                });
                self.pending_bound = true;
                self.request_adapter(AdapterQuery::Peer {
                    target: *addr.ip(),
                    bind_ip: self.config.bind_ip,
                })?;
            }
            Command::SendPacket { to_peer, bytes } => {
                let addr = self
                    .routes
                    .get(usize::from(*to_peer))
                    .and_then(Option::as_ref)
                    .map(|route| route.addr)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::NotConnected, "UNKNOWN NETWORK PEER")
                    })?;
                if bytes.len() > MAX_PACKET_BYTES || NetPacket::from_bytes(bytes).is_none() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "INVALID OUTGOING PACKET",
                    ));
                }
                if self.outgoing.len() >= QUEUE_LIMIT {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "NETWORK SEND QUEUE FULL",
                    ));
                }
                self.outgoing.push_back(Pending {
                    addr,
                    bytes: bytes.clone(),
                });
            }
            Command::RefreshRooms => {
                if let Err(error) = self.ensure_scanner() {
                    self.note(format!("LAN discovery: {error}"));
                }
                self.scanning = true;
                self.last_probe = None;
                self.publish_rooms(true);
                self.directory_fetch = true;
            }
            Command::Close => {
                // Dispatch a final forfeit/leave before releasing the socket.
                // This remains best effort and bounded; WouldBlock never waits.
                self.flush();
                self.close();
            }
        }
        Ok(())
    }

    fn emit_bound(&mut self) {
        if !self.pending_bound {
            return;
        }
        let (Some(link), Some(mut addr)) = (self.link, self.local_addr()) else {
            return;
        };
        if addr.ip().is_unspecified() {
            addr.set_ip(self.advertised_ip);
        }
        if addr.ip().is_unspecified() {
            return;
        }
        let code_addr = if self.hosting {
            addr
        } else {
            self.routes[0]
                .as_ref()
                .map(|route| route.addr)
                .unwrap_or(addr)
        };
        self.events.push(Event::Bound {
            local_addr: addr.to_string(),
            room_code: encode_room_code(code_addr.ip().octets(), code_addr.port()),
            link,
        });
        self.pending_bound = false;
    }

    fn request_adapter(&mut self, query: AdapterQuery) -> io::Result<()> {
        if self.config.link_override.is_some() {
            return Ok(());
        }
        self.link = None;
        self.adapter_serial = self.adapter_serial.wrapping_add(1);
        self.adapter_pending = Some((self.adapter_serial, query));
        self.start_adapter()
    }

    fn start_adapter(&mut self) -> io::Result<()> {
        if self.adapter_busy {
            return Ok(());
        }
        let Some(request) = self.adapter_pending.take() else {
            return Ok(());
        };
        let worker = self
            .adapter
            .as_ref()
            .ok_or_else(|| io::Error::other("NETWORK ADAPTER WORKER UNAVAILABLE"))?;
        worker
            .tx
            .try_send(request)
            .map_err(|_| io::Error::other("NETWORK ADAPTER WORKER UNAVAILABLE"))?;
        self.adapter_busy = true;
        Ok(())
    }

    fn poll_adapter(&mut self) {
        let result = self
            .adapter
            .as_ref()
            .and_then(|worker| worker.rx.try_recv().ok());
        if let Some((serial, result)) = result {
            self.adapter_busy = false;
            if serial == self.adapter_serial && self.pending_bound {
                match result {
                    Ok(info) if !info.ip.is_unspecified() => {
                        self.link = Some(info.link);
                        self.advertised_ip = info.ip;
                    }
                    Ok(_) => {
                        self.close();
                        self.error("INVALID NETWORK ADAPTER ADDRESS");
                    }
                    Err(error) => {
                        self.close();
                        self.error(format!("NETWORK ADAPTER: {error}"));
                    }
                }
            }
        }
        if let Err(error) = self.start_adapter() {
            self.close();
            self.error(error.to_string());
        }
    }

    fn flush(&mut self) {
        for _ in 0..IO_BUDGET {
            let (Some(socket), Some(packet)) = (&self.socket, self.outgoing.front()) else {
                break;
            };
            match socket.send_to(&packet.bytes, packet.addr) {
                Ok(_) => {
                    self.outgoing.pop_front();
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    self.outgoing.pop_front();
                    self.error(error.to_string());
                }
            }
        }
    }

    fn receive(&mut self) {
        let mut bytes = [0; MAX_PACKET_BYTES + 1];
        for _ in 0..IO_BUDGET {
            let Some(socket) = &self.socket else {
                break;
            };
            let (count, addr) = match socket.recv_from(&mut bytes) {
                Ok((count, SocketAddr::V4(addr))) => (count, addr),
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error)
                    if error.kind() == io::ErrorKind::ConnectionReset
                        || error.raw_os_error() == Some(10040) =>
                {
                    continue
                }
                Err(error) => {
                    self.error(error.to_string());
                    break;
                }
            };
            if count > MAX_PACKET_BYTES {
                continue;
            }
            if bytes[..count].starts_with(b"SF1D") {
                self.lan_packet(&bytes[..count], addr);
                continue;
            }
            let Some(packet) = NetPacket::from_bytes(&bytes[..count]) else {
                continue;
            };
            let existing = self
                .routes
                .iter()
                .position(|route| route.as_ref().is_some_and(|route| route.addr == addr));
            let route = if let Some(route) = existing {
                route
            } else {
                if !self.hosting
                    || !matches!(packet, NetPacket::Hello { .. } | NetPacket::Ping { .. })
                {
                    continue;
                }
                let Some(free) = self.routes[1..].iter().position(Option::is_none) else {
                    continue;
                };
                let route = free + 1;
                self.routes[route] = Some(Route {
                    addr,
                    seen: Instant::now(),
                });
                route
            };
            self.routes[route].as_mut().unwrap().seen = Instant::now();
            if self.events.len() < QUEUE_LIMIT {
                self.events.push(Event::PacketReceived {
                    from_peer: route as u8,
                    addr: addr.to_string(),
                    bytes: bytes[..count].to_vec(),
                });
            }
        }
    }

    fn shared_discovery(&self) -> bool {
        self.hosting
            && self.config.discovery_port != 0
            && self
                .local_addr()
                .is_some_and(|addr| addr.port() == self.config.discovery_port)
    }

    fn prepare_lan(&mut self) {
        if !self.room.as_ref().is_some_and(|room| room.public) {
            self.discovery = None;
            return;
        }
        if self.shared_discovery() {
            self.discovery = None;
            return;
        }
        if self.discovery.is_some() {
            return;
        }
        let result = (|| {
            let socket = socket2::Socket::new(
                socket2::Domain::IPV4,
                socket2::Type::DGRAM,
                Some(socket2::Protocol::UDP),
            )?;
            socket.set_reuse_address(true)?;
            socket.set_broadcast(true)?;
            socket.set_nonblocking(true)?;
            socket
                .bind(&SocketAddrV4::new(self.config.bind_ip, self.config.discovery_port).into())?;
            Ok::<UdpSocket, io::Error>(socket.into())
        })();
        match result {
            Ok(socket) => self.discovery = Some(socket),
            Err(error) => self.note(format!("LAN discovery: {error}")),
        }
    }

    fn ensure_scanner(&mut self) -> io::Result<()> {
        if self.scanner.is_none() {
            let socket = UdpSocket::bind(SocketAddrV4::new(self.config.bind_ip, 0))?;
            socket.set_nonblocking(true)?;
            socket.set_broadcast(true)?;
            self.scanner = Some(socket);
        }
        Ok(())
    }

    fn lan_tick(&mut self) {
        let interval = self.config.beacon_interval.max(Duration::from_millis(50));
        let now = Instant::now();
        self.discovery_probes.retain(|probe| {
            probe.epoch == self.directory_epoch
                && now.duration_since(probe.created) <= Duration::from_secs(5)
        });
        if self.scanning
            && self
                .last_probe
                .is_none_or(|at| now.duration_since(at) >= interval)
        {
            self.last_probe = Some(now);
            for target in self.config.discovery_targets.clone() {
                self.probe_room(target, None);
            }
        }
        if self.scanning {
            let targets: Vec<_> =
                self.directory_rooms
                    .keys()
                    .copied()
                    .filter(|addr| {
                        self.room_timing.get(addr).is_none_or(|timing| {
                            timing.last_probe.elapsed() >= Duration::from_secs(3)
                        }) && !self
                            .discovery_probes
                            .iter()
                            .any(|probe| probe.expected == Some(*addr))
                    })
                    .take(16)
                    .collect();
            for target in targets {
                self.probe_room(target, Some(target));
            }
        }
        if self.hosting
            && self.room.as_ref().is_some_and(|room| room.public)
            && self.link.is_some()
            && self
                .last_beacon
                .is_none_or(|at| now.duration_since(at) >= interval)
        {
            self.last_beacon = Some(now);
            for target in self.config.discovery_targets.clone() {
                self.send_beacon(target);
            }
        }
        self.receive_lan(false);
        self.receive_lan(true);
        self.flush_discovery();
    }

    fn receive_lan(&mut self, scanner: bool) {
        let mut bytes = [0; MAX_PACKET_BYTES + 1];
        for _ in 0..IO_BUDGET {
            let socket = if scanner {
                &self.scanner
            } else {
                &self.discovery
            };
            let Some(socket) = socket else {
                break;
            };
            let (count, addr) = match socket.recv_from(&mut bytes) {
                Ok((count, SocketAddr::V4(addr))) => (count, addr),
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error)
                    if error.kind() == io::ErrorKind::ConnectionReset
                        || error.raw_os_error() == Some(10040) =>
                {
                    continue
                }
                Err(error) => {
                    self.note(format!("LAN receive: {error}"));
                    break;
                }
            };
            if count <= MAX_PACKET_BYTES {
                self.lan_packet(&bytes[..count], addr);
            }
        }
    }

    fn lan_packet(&mut self, bytes: &[u8], from: SocketAddrV4) {
        if bytes == b"SF1D\x01\x01" {
            self.send_beacon(from);
            return;
        }
        if bytes.len() == 14 && bytes.starts_with(b"SF1D\x01\x03") {
            if let Some(beacon) = self.public_beacon() {
                let mut response = b"SF1D\x01\x04".to_vec();
                response.extend(&bytes[6..14]);
                response.extend(beacon);
                self.queue_discovery(false, from, response);
            }
            return;
        }
        if bytes.starts_with(b"SF1D\x01\x04") {
            self.probe_reply(bytes, from);
            return;
        }
        if let Some(room) = decode_room_announcement(bytes, *from.ip()) {
            let Some((ip, port)) = decode_room_code(&room.code) else {
                return;
            };
            let address = SocketAddrV4::new(Ipv4Addr::from(ip), port);
            if self.rooms.len() >= 128 && !self.rooms.contains_key(&address) {
                return;
            }
            self.rooms.insert(
                address,
                CachedRoom {
                    room,
                    expires: Instant::now() + self.config.room_ttl,
                },
            );
        }
    }

    fn send_beacon(&mut self, target: SocketAddrV4) {
        if let Some(bytes) = self.public_beacon() {
            self.queue_discovery(false, target, bytes);
        }
    }

    fn public_beacon(&self) -> Option<Vec<u8>> {
        let (Some(room), Some(addr), Some(link)) =
            (self.room.as_ref(), self.local_addr(), self.link)
        else {
            return None;
        };
        if !self.hosting || !room.public {
            return None;
        }
        let mut room = room.clone();
        room.link = link;
        encode_room_announcement(&room, addr.port())
    }

    fn queue_discovery(&mut self, scanner: bool, addr: SocketAddrV4, bytes: Vec<u8>) -> bool {
        if self.discovery_sends.len() >= 128 {
            self.note("LAN discovery queue full");
            return false;
        }
        self.discovery_sends.push_back(DiscoverySend {
            scanner,
            addr,
            bytes,
            created: Instant::now(),
        });
        true
    }

    fn probe_room(&mut self, target: SocketAddrV4, expected: Option<SocketAddrV4>) {
        if self.scanner.is_none() {
            return;
        }
        let seq = self.discovery_seq;
        self.discovery_seq = self.discovery_seq.wrapping_add(1);
        let sent_ms = self.now_ms();
        let mut bytes = b"SF1D\x01\x03".to_vec();
        bytes.extend(seq.to_le_bytes());
        bytes.extend(sent_ms.to_le_bytes());
        if !self.queue_discovery(true, target, bytes) {
            return;
        }
        if self.discovery_probes.len() == 128 {
            self.discovery_probes.pop_front();
        }
        self.discovery_probes.push_back(DiscoveryProbe {
            seq,
            sent_ms,
            epoch: self.directory_epoch,
            created: Instant::now(),
            expected,
            seen: Vec::new(),
        });
    }

    fn probe_reply(&mut self, bytes: &[u8], from: SocketAddrV4) {
        if bytes.len() != 70 {
            return;
        }
        let Some(room) = decode_room_announcement(&bytes[14..], *from.ip()) else {
            return;
        };
        let Some((ip, port)) = decode_room_code(&room.code) else {
            return;
        };
        let address = SocketAddrV4::new(Ipv4Addr::from(ip), port);
        let seq = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
        let echo = u32::from_le_bytes(bytes[10..14].try_into().unwrap());
        let now_ms = self.now_ms();
        let Some(probe) = self.discovery_probes.iter_mut().find(|probe| {
            probe.seq == seq
                && probe.sent_ms == echo
                && probe.epoch == self.directory_epoch
                && probe.created.elapsed() <= Duration::from_secs(5)
                && probe.expected.is_none_or(|expected| expected == address)
                && !probe.seen.contains(&address)
        }) else {
            return;
        };
        if probe.seen.len() >= 128
            || (self.rooms.len() >= 128 && !self.rooms.contains_key(&address))
            || (self.room_timing.len() >= 128 && !self.room_timing.contains_key(&address))
        {
            return;
        }
        probe.seen.push(address);
        let timing = self
            .room_timing
            .entry(address)
            .or_insert_with(|| RoomTiming {
                rtt: RttEstimator::default(),
                last_probe: Instant::now(),
            });
        if !timing.rtt.observe(now_ms, echo) {
            return;
        }
        timing.last_probe = Instant::now();
        self.rooms.insert(
            address,
            CachedRoom {
                room,
                expires: Instant::now() + self.config.room_ttl,
            },
        );
    }

    fn flush_discovery(&mut self) {
        for _ in 0..IO_BUDGET {
            let Some(packet) = self.discovery_sends.front() else {
                break;
            };
            if packet.created.elapsed() > Duration::from_secs(5) {
                self.discovery_sends.pop_front();
                continue;
            }
            let socket = if packet.scanner {
                self.scanner.as_ref()
            } else {
                self.discovery.as_ref().or(self.socket.as_ref())
            };
            let Some(socket) = socket else {
                self.discovery_sends.pop_front();
                continue;
            };
            match socket.send_to(&packet.bytes, packet.addr) {
                Ok(_) => {
                    self.discovery_sends.pop_front();
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    self.discovery_sends.pop_front();
                    self.note(format!("LAN discovery: {error}"));
                }
            }
        }
    }

    fn publish_rooms(&mut self, force: bool) {
        let mut merged: BTreeMap<_, _> = self
            .directory_rooms
            .iter()
            .map(|(addr, cached)| (*addr, cached.room.clone()))
            .collect();
        for (addr, cached) in &self.rooms {
            merged.insert(*addr, cached.room.clone());
        }
        for (addr, room) in &mut merged {
            if let Some(timing) = self.room_timing.get(addr) {
                room.ping_ms = timing.rtt.rtt_ms().clamp(1, u16::MAX as u32) as u16;
                room.jitter_ms = timing.rtt.jitter_ms().min(u16::MAX as u32) as u16;
            }
        }
        let rooms: Vec<_> = merged.into_values().take(128).collect();
        if force || rooms != self.visible_rooms {
            self.visible_rooms = rooms.clone();
            if self.events.len() < QUEUE_LIMIT {
                self.events.push(Event::RoomsDiscovered { rooms });
            }
        }
    }

    fn directory_tick(&mut self) {
        if let Some(result) = self
            .directory
            .as_ref()
            .and_then(directory::DirectoryWorker::try_result)
        {
            self.directory_busy = false;
            if result.epoch == self.directory_epoch {
                match result.result {
                    Ok(rooms) if self.directory_job_fetch => {
                        self.directory_rooms.clear();
                        let expires =
                            Instant::now() + self.config.room_ttl.max(Duration::from_secs(15));
                        for room in rooms.into_iter().take(128) {
                            if let Some((ip, port)) = decode_room_code(&room.code) {
                                self.directory_rooms.insert(
                                    SocketAddrV4::new(Ipv4Addr::from(ip), port),
                                    CachedRoom { room, expires },
                                );
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(message) => self.note(format!("Room directory: {message}")),
                }
            }
        }
        if self.scanning
            && self
                .directory_last_fetch
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(3))
        {
            self.directory_fetch = true;
        }
        let publication = if self.hosting
            && !self.pending_bound
            && (self.directory_publish
                || self
                    .directory_last_publish
                    .is_none_or(|at| at.elapsed() >= Duration::from_secs(5)))
        {
            match (self.room.as_ref(), self.local_addr(), self.link) {
                (Some(room), Some(addr), Some(link)) if room.public => {
                    let mut room = room.clone();
                    room.link = link;
                    Some((addr.port(), room))
                }
                _ => None,
            }
        } else {
            None
        };
        if self.directory_busy {
            return;
        }
        let withdrawal = self.directory_withdrawals.front().cloned();
        let job = if let Some((url, port)) = withdrawal.as_ref() {
            directory::DirectoryJob {
                epoch: self.directory_epoch,
                url: url.clone(),
                publish: None,
                withdraw_port: Some(*port),
                fetch: false,
            }
        } else {
            let Some(url) = self.lobby_url.clone() else {
                return;
            };
            if !self.directory_fetch && publication.is_none() {
                return;
            }
            directory::DirectoryJob {
                epoch: self.directory_epoch,
                url,
                publish: publication,
                withdraw_port: None,
                fetch: self.directory_fetch,
            }
        };
        if self.directory.is_none() {
            match directory::DirectoryWorker::new(self.config.directory_timeout) {
                Ok(worker) => self.directory = Some(worker),
                Err(_) => {
                    self.directory_fetch = false;
                    self.directory_last_fetch = Some(Instant::now());
                    self.note("Room directory worker unavailable");
                    return;
                }
            }
        }
        let published = job
            .publish
            .as_ref()
            .map(|(port, _)| (job.url.clone(), *port));
        let fetch = job.fetch;
        if self.directory.as_ref().unwrap().try_submit(job).is_ok() {
            self.directory_busy = true;
            self.directory_job_fetch = fetch;
            if fetch {
                self.directory_fetch = false;
                self.directory_last_fetch = Some(Instant::now());
            }
            if let Some(published) = published {
                self.directory_published = Some(published);
                self.directory_publish = false;
                self.directory_last_publish = Some(Instant::now());
            }
            if withdrawal.is_some() {
                self.directory_withdrawals.pop_front();
            }
        }
    }

    fn withdraw_directory(&mut self) {
        if let Some(published) = self.directory_published.take() {
            if self.directory_withdrawals.len() == 4 {
                self.directory_withdrawals.pop_front();
            }
            if !self.directory_withdrawals.contains(&published) {
                self.directory_withdrawals.push_back(published);
            }
        }
    }
}

fn valid_announcement(room: &RoomAnnouncement) -> bool {
    tag_from_str(&tag_to_string(room.host)) == Some(room.host)
        && room.spectators <= 4
        && (1..=8).contains(&room.max_rollback)
}

/// LAN discovery v1. Hashes stay binary, avoiding JSON/JavaScript u64 precision
/// loss when this same beacon is hex-encoded by the optional room directory.
pub fn encode_room_announcement(room: &RoomAnnouncement, port: u16) -> Option<Vec<u8>> {
    if port == 0 || !valid_announcement(room) {
        return None;
    }
    let mut bytes = b"SF1D\x01\x02".to_vec();
    bytes.extend(port.to_le_bytes());
    bytes.extend(room.host);
    bytes.push(match room.kind {
        RoomKind::Versus => 0,
        RoomKind::Spectate => 1,
        RoomKind::Dramatic => 2,
    });
    bytes.push(match room.link {
        Link::Ethernet => 0,
        Link::Wifi => 1,
    });
    bytes.extend([
        u8::from(room.wired_only),
        u8::from(room.public),
        u8::from(room.in_match),
    ]);
    bytes.push(match room.preset {
        RulesetPreset::DefinitiveOverhaul => 0,
        RulesetPreset::Faithful1987 => 1,
        RulesetPreset::ClassicCompetitive => 2,
        RulesetPreset::PureFootsies1987Plus => 3,
        RulesetPreset::Custom => 4,
    });
    bytes.extend([room.spectators, room.max_rollback]);
    bytes.extend(room.config_hash.to_le_bytes());
    bytes.extend(room.bank_hash.to_le_bytes());
    bytes.extend(room.art_pack_hash.to_le_bytes());
    bytes.extend(room.protocol_hash.to_le_bytes());
    Some(bytes)
}

pub fn decode_room_announcement(bytes: &[u8], source: Ipv4Addr) -> Option<Room> {
    if bytes.len() != 56
        || bytes.get(..6)? != b"SF1D\x01\x02"
        || bytes[22..25].iter().any(|b| *b > 1)
        || bytes[23] == 0
        || bytes[26] > 4
        || !(1..=8).contains(&bytes[27])
    {
        return None;
    }
    let port = u16::from_le_bytes(bytes[6..8].try_into().ok()?);
    if port == 0 {
        return None;
    }
    let tag: [u8; 12] = bytes[8..20].try_into().ok()?;
    if tag_from_str(&tag_to_string(tag)) != Some(tag) {
        return None;
    }
    Some(Room {
        code: encode_room_code(source.octets(), port),
        host: tag_to_string(tag),
        region: String::new(),
        ping_ms: 0,
        rollback_frames: bytes[27],
        jitter_ms: 0,
        kind: match bytes[20] {
            0 => RoomKind::Versus,
            1 => RoomKind::Spectate,
            2 => RoomKind::Dramatic,
            _ => return None,
        },
        link: match bytes[21] {
            0 => Link::Ethernet,
            1 => Link::Wifi,
            _ => return None,
        },
        preset: match bytes[25] {
            0 => RulesetPreset::DefinitiveOverhaul,
            1 => RulesetPreset::Faithful1987,
            2 => RulesetPreset::ClassicCompetitive,
            3 => RulesetPreset::PureFootsies1987Plus,
            4 => RulesetPreset::Custom,
            _ => return None,
        },
        spectators: bytes[26],
        in_match: bytes[24] != 0,
        what_if: false,
        bonus: false,
    })
}

/// Query the selected IPv4 route and its media. Called only by the adapter
/// worker. Loopback carries no radio traffic. Unknown/ambiguous interfaces fail
/// explicitly instead of claiming to be wired.
pub fn detect_active_adapter(target: Ipv4Addr) -> io::Result<AdapterInfo> {
    detect_adapter(AdapterQuery::Peer {
        target,
        bind_ip: Ipv4Addr::UNSPECIFIED,
    })
}

/// Hosts select an available local interface, including isolated LANs with no
/// default route. Clients select the actual destination route. Explicit binds
/// constrain either query to the selected local address.
pub fn detect_adapter(query: AdapterQuery) -> io::Result<AdapterInfo> {
    let loopback = match query {
        AdapterQuery::Host { bind_ip } if bind_ip.is_loopback() => Some(bind_ip),
        AdapterQuery::Peer { target, bind_ip } if target.is_loopback() => {
            Some(if bind_ip.is_unspecified() {
                Ipv4Addr::LOCALHOST
            } else {
                bind_ip
            })
        }
        _ => None,
    };
    if let Some(ip) = loopback {
        return Ok(AdapterInfo {
            ip,
            link: Link::Ethernet,
        });
    }
    detect_platform_adapter(query)
}

#[cfg(windows)]
fn detect_platform_adapter(query: AdapterQuery) -> io::Result<AdapterInfo> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command as Process, Stdio};
    // Host addresses come from the bound/available local interfaces, with a
    // preferred default route when present. No external destination is needed.
    // InterfaceType 6 is Ethernet and 71 is IEEE802.11.
    // https://learn.microsoft.com/powershell/module/nettcpip/find-netroute
    // https://learn.microsoft.com/powershell/module/nettcpip/get-netipaddress
    // https://learn.microsoft.com/powershell/module/nettcpip/get-netipinterface
    // https://learn.microsoft.com/powershell/module/nettcpip/get-netroute
    // https://learn.microsoft.com/powershell/module/netadapter/get-netadapter
    const SCRIPT: &str = r#"$ErrorActionPreference='Stop';
[Console]::OutputEncoding=New-Object System.Text.UTF8Encoding($false);
$bind='__BIND__'; $target='__TARGET__'; $hosting=__HOST__;
$ip=$null; $a=$null;
if ($hosting -and $bind -ne '0.0.0.0') {
  $ip=@(Get-NetIPAddress -AddressFamily IPv4 -IPAddress $bind -AddressState Preferred)[0];
} elseif (-not $hosting) {
  if ($bind -eq '0.0.0.0') {
    $ip=@(Find-NetRoute -RemoteIPAddress $target | Where-Object {$_.IPAddress})[0];
  } else {
    $ip=@(Find-NetRoute -RemoteIPAddress $target -LocalIPAddress $bind | Where-Object {$_.IPAddress})[0];
  }
} else {
  $addresses=@(Get-NetIPAddress -AddressFamily IPv4 -AddressState Preferred | Where-Object {-not $_.SkipAsSource});
  $interfaces=@(Get-NetIPInterface -AddressFamily IPv4 | Where-Object {$_.ConnectionState -eq 'Connected'} | Sort-Object InterfaceMetric,InterfaceIndex);
  $metrics=@{}; foreach ($i in $interfaces) { $metrics[[uint32]$i.InterfaceIndex]=[uint32]$i.InterfaceMetric };
  $routes=@(Get-NetRoute -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Sort-Object @{Expression={[uint64]$_.RouteMetric+[uint64]$metrics[[uint32]$_.InterfaceIndex]}},InterfaceIndex);
  foreach ($r in $routes) {
    if (-not $metrics.ContainsKey([uint32]$r.InterfaceIndex)) { continue };
    $ip=@($addresses | Where-Object {$_.InterfaceIndex -eq $r.InterfaceIndex} | Sort-Object IPAddress)[0];
    if ($ip) { break };
  };
  if (!$ip) {
    $physical=@(Get-NetAdapter -Physical | Where-Object {$_.Status -eq 'Up' -and [int]$_.InterfaceType -in @(6,71)});
    foreach ($i in $interfaces) {
      $a=@($physical | Where-Object {$_.InterfaceIndex -eq $i.InterfaceIndex})[0];
      if (!$a) { continue };
      $ip=@($addresses | Where-Object {$_.InterfaceIndex -eq $i.InterfaceIndex} | Sort-Object IPAddress)[0];
      if ($ip) { break };
    };
  };
  if (!$ip -and @($addresses | Where-Object {$_.IPAddress -eq '127.0.0.1'}).Count -gt 0) {
    Write-Output '6|127.0.0.1'; exit;
  };
};
if (!$ip) { throw 'No available local IPv4 address' };
if (!$a) { $a=Get-NetAdapter -InterfaceIndex $ip.InterfaceIndex -IncludeHidden -ErrorAction SilentlyContinue };
if ((-not $a.HardwareInterface) -or ([int]$a.InterfaceType -notin @(6,71))) {
  $physical=@(Get-NetAdapter -Physical | Where-Object {$_.Status -eq 'Up'});
  if ($physical.Count -ne 1) { throw 'Cannot identify physical media for this route' };
  $a=$physical[0];
};
if ($a.Status -ne 'Up') { throw 'Selected network adapter is not up' };
Write-Output ([string][int]$a.InterfaceType+'|'+[string]$ip.IPAddress);"#;
    let (hosting, target, bind_ip) = match query {
        AdapterQuery::Host { bind_ip } => (true, Ipv4Addr::UNSPECIFIED, bind_ip),
        AdapterQuery::Peer { target, bind_ip } => (false, target, bind_ip),
    };
    let script = SCRIPT
        .replace("__TARGET__", &target.to_string())
        .replace("__BIND__", &bind_ip.to_string())
        .replace("__HOST__", if hosting { "$true" } else { "$false" });
    let mut command = Process::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &script,
    ]);
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let until = Instant::now() + Duration::from_secs(4);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "ADAPTER QUERY TIMED OUT",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if !status.success() {
        return Err(io::Error::other(
            "CANNOT IDENTIFY THE ACTIVE NETWORK ADAPTER",
        ));
    }
    let mut text = String::new();
    child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("NO ADAPTER QUERY OUTPUT"))?
        .take(4097)
        .read_to_string(&mut text)?;
    if text.len() > 4096 {
        return Err(io::Error::other("INVALID ADAPTER QUERY OUTPUT"));
    }
    let (kind, ip) = text
        .trim()
        .split_once('|')
        .ok_or_else(|| io::Error::other("INVALID ADAPTER QUERY OUTPUT"))?;
    let ip = ip
        .parse()
        .map_err(|_| io::Error::other("INVALID ADAPTER IPV4 ADDRESS"))?;
    let hint = match kind {
        "6" => "Ethernet",
        "71" => "Wi-Fi",
        _ => return Err(io::Error::other("UNSUPPORTED NETWORK MEDIA")),
    };
    Ok(AdapterInfo {
        ip,
        link: sf_engine::netplay::detect_link(hint),
    })
}

#[cfg(target_os = "linux")]
fn detect_platform_adapter(query: AdapterQuery) -> io::Result<AdapterInfo> {
    // iproute2's JSON preserves source/interface identity without parsing a
    // human-facing table or guessing an IPv4 address from a route prefix.
    // https://man7.org/linux/man-pages/man8/ip.8.html
    // https://man7.org/linux/man-pages/man8/ip-address.8.html
    // https://man7.org/linux/man-pages/man8/ip-route.8.html
    match query {
        AdapterQuery::Peer { target, bind_ip } => {
            let mut args = vec![
                "-j".into(),
                "-4".into(),
                "route".into(),
                "get".into(),
                target.to_string(),
            ];
            if !bind_ip.is_unspecified() {
                args.extend(["from".into(), bind_ip.to_string()]);
            }
            let value = linux_ip(&args)?;
            let route = value
                .as_array()
                .and_then(|rows| rows.first())
                .ok_or_else(|| io::Error::other("NO IPV4 ROUTE"))?;
            let name = route
                .get("dev")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| io::Error::other("NO ROUTED INTERFACE"))?;
            let ip = if bind_ip.is_unspecified() {
                route
                    .get("prefsrc")
                    .or_else(|| route.get("src"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(|ip| ip.parse().ok())
                    .ok_or_else(|| io::Error::other("NO ROUTED IPV4 ADDRESS"))?
            } else {
                bind_ip
            };
            Ok(AdapterInfo {
                ip,
                link: linux_link(name)?,
            })
        }
        AdapterQuery::Host { bind_ip } => {
            let value = linux_ip(&[
                "-j".into(),
                "-4".into(),
                "address".into(),
                "show".into(),
                "up".into(),
            ])?;
            let rows = value
                .as_array()
                .ok_or_else(|| io::Error::other("INVALID LOCAL ADDRESS LIST"))?;
            let mut candidates = Vec::new();
            for row in rows {
                // Administrative UP alone does not mean a cable/carrier is
                // usable. UNKNOWN remains usable for drivers without operstate.
                // https://docs.kernel.org/networking/operstates.html
                if matches!(
                    row.get("operstate").and_then(serde_json::Value::as_str),
                    Some("DOWN" | "LOWERLAYERDOWN" | "DORMANT" | "TESTING" | "NOTPRESENT")
                ) {
                    continue;
                }
                let Some(name) = row.get("ifname").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let Some(addresses) = row.get("addr_info").and_then(serde_json::Value::as_array)
                else {
                    continue;
                };
                for address in addresses {
                    if address
                        .get("tentative")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                        || address
                            .get("dadfailed")
                            .and_then(serde_json::Value::as_bool)
                            == Some(true)
                        || address
                            .get("preferred_life_time")
                            .and_then(serde_json::Value::as_u64)
                            == Some(0)
                    {
                        continue;
                    }
                    let Some(ip) = address
                        .get("local")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|ip| ip.parse::<Ipv4Addr>().ok())
                    else {
                        continue;
                    };
                    if ip.is_unspecified() || (!bind_ip.is_unspecified() && ip != bind_ip) {
                        continue;
                    }
                    if let Ok(link) = linux_link(name) {
                        candidates.push((
                            ip.is_loopback(),
                            row.get("ifindex")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(u64::MAX),
                            name,
                            ip,
                            link,
                        ));
                    }
                }
            }
            candidates
                .sort_by_key(|(loopback, index, name, ip, _)| (*loopback, *index, *name, *ip));
            if bind_ip.is_unspecified() {
                // Prefer the default interface when one exists; an isolated LAN
                // still has its real local candidates when this list is empty.
                if let Ok(defaults) = linux_ip(&[
                    "-j".into(),
                    "-4".into(),
                    "route".into(),
                    "show".into(),
                    "default".into(),
                ]) {
                    let mut routes: Vec<_> = defaults.as_array().into_iter().flatten().collect();
                    routes.sort_by_key(|route| {
                        route
                            .get("metric")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0)
                    });
                    for route in routes {
                        if let Some((_, _, _, ip, link)) =
                            candidates.iter().find(|(_, _, name, _, _)| {
                                Some(*name) == route.get("dev").and_then(serde_json::Value::as_str)
                            })
                        {
                            return Ok(AdapterInfo {
                                ip: *ip,
                                link: *link,
                            });
                        }
                    }
                }
            }
            let (_, _, _, ip, link) = candidates
                .first()
                .ok_or_else(|| io::Error::other("NO AVAILABLE LOCAL IPV4 ADAPTER"))?;
            Ok(AdapterInfo {
                ip: *ip,
                link: *link,
            })
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_link(name: &str) -> io::Result<Link> {
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
    {
        return Err(io::Error::other("INVALID INTERFACE NAME"));
    }
    let interface = std::path::Path::new("/sys/class/net").join(name);
    if std::fs::read_to_string(interface.join("type"))?.trim() == "772" {
        return Ok(Link::Ethernet); // loopback carries no radio traffic
    }
    let hint = if interface.join("wireless").exists() || interface.join("phy80211").exists() {
        "Wi-Fi"
    } else if std::fs::read_to_string(interface.join("type"))?.trim() == "1" {
        "Ethernet"
    } else {
        return Err(io::Error::other("UNSUPPORTED NETWORK MEDIA"));
    };
    Ok(sf_engine::netplay::detect_link(hint))
}

#[cfg(target_os = "linux")]
fn linux_ip(args: &[String]) -> io::Result<serde_json::Value> {
    use std::io::Read;
    use std::process::{Command as Process, Stdio};
    let mut child = Process::new("ip")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| io::Error::other("IPROUTE2 ADDRESS QUERY IS UNAVAILABLE"))?;
    let until = Instant::now() + Duration::from_secs(4);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "ADAPTER QUERY TIMED OUT",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if !status.success() {
        return Err(io::Error::other("IPROUTE2 ADAPTER QUERY FAILED"));
    }
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("NO ADAPTER QUERY OUTPUT"))?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(io::Error::other("ADAPTER QUERY OUTPUT TOO LARGE"));
    }
    serde_json::from_slice(&bytes).map_err(|_| io::Error::other("INVALID ADAPTER QUERY OUTPUT"))
}

#[cfg(not(any(windows, target_os = "linux")))]
fn detect_platform_adapter(_: AdapterQuery) -> io::Result<AdapterInfo> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "ACTIVE NETWORK MEDIA QUERY IS UNAVAILABLE ON THIS OS",
    ))
}
