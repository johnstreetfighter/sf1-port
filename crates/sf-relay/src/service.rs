//! The relay service: QUIC in, room rules in the middle, datagrams out.
//!
//! Each connection makes one request and then only sends and receives
//! datagrams. The relay never reads gameplay payloads; it forwards them
//! between the members of one room by seat.

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use quinn::crypto::rustls::QuicServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::sync::oneshot;

use crate::rooms::{ConnId, Departure, Limits, Registry};
use crate::wire::{self, close, Reply, Request};
use crate::RoomCode;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const EXPIRY_TICK: Duration = Duration::from_secs(5);

pub struct ServiceConfig {
    pub listen: SocketAddr,
    /// DER certificate chain, leaf first.
    pub certificates: Vec<Vec<u8>>,
    /// DER PKCS #8 private key.
    pub private_key: Vec<u8>,
    pub limits: Limits,
}

/// A running relay. Dropping it shuts the relay down.
pub struct RelayService {
    local_addr: SocketAddr,
    state: Arc<State>,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

struct State {
    rooms: Mutex<Registry>,
    connections: Mutex<HashMap<ConnId, quinn::Connection>>,
    next: AtomicU64,
    /// Connections still handshaking or waiting on their request.
    admitting: AtomicUsize,
    max_connections: usize,
}

/// Counts one connection as admitting until it is seated or gone.
struct Admitting(Arc<State>);

impl Drop for Admitting {
    fn drop(&mut self) {
        self.0.admitting.fetch_sub(1, Ordering::Relaxed);
    }
}

impl RelayService {
    pub fn start(config: ServiceConfig) -> io::Result<Self> {
        let server = server_config(&config)?;
        let limits = config.limits;
        let state = Arc::new(State {
            rooms: Mutex::new(Registry::new(limits, random_index()?)),
            connections: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            admitting: AtomicUsize::new(0),
            // Every seat of every room, plus headroom for joins in flight.
            max_connections: limits.max_rooms.saturating_mul(crate::rooms::SEATS).min(64 * 1024) + 256,
        });
        let (stop, stopped) = oneshot::channel();
        let (bound_tx, bound) = mpsc::sync_channel(1);
        let shared = state.clone();
        let listen = config.listen;
        let thread = std::thread::Builder::new().name("sf-relay".into()).spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = bound_tx.send(Err(error));
                    return;
                }
            };
            runtime.block_on(async move {
                let endpoint = match quinn::Endpoint::server(server, listen) {
                    Ok(endpoint) => endpoint,
                    Err(error) => {
                        let _ = bound_tx.send(Err(error));
                        return;
                    }
                };
                let _ = bound_tx.send(endpoint.local_addr());
                serve(endpoint, shared, stopped).await;
            });
        })?;
        let local_addr = bound.recv().map_err(|_| io::Error::other("RELAY THREAD ENDED"))??;
        Ok(Self { local_addr, state, shutdown: Some(stop), thread: Some(thread) })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Capacity metrics; never codes or payloads.
    pub fn open_rooms(&self) -> usize {
        self.state.rooms.lock().map_or(0, |rooms| rooms.open_rooms())
    }

    pub fn connections(&self) -> usize {
        self.state.connections.lock().map_or(0, |connections| connections.len())
    }
}

impl Drop for RelayService {
    fn drop(&mut self) {
        if let Some(stop) = self.shutdown.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn serve(endpoint: quinn::Endpoint, state: Arc<State>, mut stopped: oneshot::Receiver<()>) {
    let mut expiry = tokio::time::interval(EXPIRY_TICK);
    loop {
        tokio::select! {
            incoming = endpoint.accept() => {
                let Some(incoming) = incoming else { break };
                let seated = state.connections.lock().map_or(usize::MAX, |c| c.len());
                if seated.saturating_add(state.admitting.load(Ordering::Relaxed)) >= state.max_connections {
                    incoming.refuse();
                    continue;
                }
                state.admitting.fetch_add(1, Ordering::Relaxed);
                tokio::spawn(handle(incoming, Admitting(state.clone())));
            }
            _ = expiry.tick() => expire(&state),
            _ = &mut stopped => break,
        }
    }
    endpoint.close(close::SHUTDOWN.into(), b"shutdown");
    let _ = tokio::time::timeout(Duration::from_secs(1), endpoint.wait_idle()).await;
}

fn expire(state: &State) {
    let closed = match state.rooms.lock() {
        Ok(mut rooms) => rooms.expire(Instant::now()),
        Err(_) => return,
    };
    let Ok(connections) = state.connections.lock() else { return };
    for (_, members) in closed {
        for member in members {
            if let Some(connection) = connections.get(&member) {
                connection.close(close::ROOM_EXPIRED.into(), b"room expired");
            }
        }
    }
}

async fn handle(incoming: quinn::Incoming, admitting: Admitting) {
    let state = admitting.0.clone();
    let Ok(Ok(connection)) = tokio::time::timeout(REQUEST_TIMEOUT, incoming).await else { return };
    let id = state.next.fetch_add(1, Ordering::Relaxed);
    let from = canonical(connection.remote_address().ip());
    let Some(request) = read_request(&connection).await else {
        connection.close(close::PROTOCOL.into(), b"protocol");
        return;
    };
    let now = Instant::now();
    let reply = {
        let Ok(mut rooms) = state.rooms.lock() else { return };
        match request.request {
            Request::Create => rooms.create(id, from, now).map(Reply::Created),
            Request::Join(code) => rooms.join(id, from, code, now).map(Reply::Joined),
        }
        .unwrap_or_else(Reply::Refused)
    };
    let mut send = request.send;
    let refused = matches!(reply, Reply::Refused(_));
    if !refused {
        if let Ok(mut connections) = state.connections.lock() {
            connections.insert(id, connection.clone());
        }
    }
    drop(admitting);
    let delivered = send.write_all(&reply.encode()).await.is_ok() && send.finish().is_ok();
    if refused {
        // Let the reply arrive; the client closes once it has read it.
        let _ = tokio::time::timeout(Duration::from_secs(1), connection.closed()).await;
        connection.close(close::REFUSED.into(), b"refused");
        return;
    }
    if delivered {
        forward(&connection, id, &state).await;
    }
    depart(id, &state);
}

struct Opened {
    request: Request,
    send: quinn::SendStream,
}

async fn read_request(connection: &quinn::Connection) -> Option<Opened> {
    let (send, mut recv) = tokio::time::timeout(REQUEST_TIMEOUT, connection.accept_bi()).await.ok()?.ok()?;
    let mut bytes = [0; wire::MESSAGE];
    tokio::time::timeout(REQUEST_TIMEOUT, recv.read_exact(&mut bytes)).await.ok()?.ok()?;
    Some(Opened { request: Request::decode(bytes)?, send })
}

async fn forward(connection: &quinn::Connection, id: ConnId, state: &State) {
    while let Ok(datagram) = connection.read_datagram().await {
        let Some((to, payload)) = wire::open(&datagram) else { continue };
        let route = match state.rooms.lock() {
            Ok(mut rooms) => rooms.route(id, to, payload.len(), Instant::now()),
            Err(_) => return,
        };
        let Some((target, from)) = route else { continue };
        let target = state.connections.lock().ok().and_then(|c| c.get(&target).cloned());
        if let (Some(target), Some(sealed)) = (target, wire::seal(from, payload)) {
            let _ = target.send_datagram(sealed.into());
        }
    }
}

fn depart(id: ConnId, state: &State) {
    let departure = match state.rooms.lock() {
        Ok(mut rooms) => rooms.leave(id),
        Err(_) => Departure::None,
    };
    let Ok(mut connections) = state.connections.lock() else { return };
    connections.remove(&id);
    if let Departure::Host { guests, .. } = departure {
        for guest in guests {
            if let Some(connection) = connections.get(&guest) {
                connection.close(close::ROOM_CLOSED.into(), b"room closed");
            }
        }
    }
}

/// IPv4 clients on a dual-stack socket throttle as their IPv4 address.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        v4 => v4,
    }
}

fn server_config(config: &ServiceConfig) -> io::Result<quinn::ServerConfig> {
    let invalid = |error: &dyn std::fmt::Display| io::Error::new(io::ErrorKind::InvalidInput, error.to_string());
    let chain = config.certificates.iter().cloned().map(CertificateDer::from).collect();
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(config.private_key.clone()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut tls = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| invalid(&error))?
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(|error| invalid(&error))?;
    tls.alpn_protocols = vec![wire::ALPN.to_vec()];
    tls.max_early_data_size = 0;
    let crypto = QuicServerConfig::try_from(tls).map_err(|error| invalid(&error))?;
    let mut server = quinn::ServerConfig::with_crypto(Arc::new(crypto));
    server.transport_config(Arc::new(crate::transport()));
    Ok(server)
}

/// Random code indexes from the system's secure random source.
fn random_index() -> io::Result<impl FnMut() -> u64 + Send + 'static> {
    use ring::rand::{SecureRandom, SystemRandom};
    let random = SystemRandom::new();
    let mut probe = [0; 8];
    random.fill(&mut probe).map_err(|_| io::Error::other("NO SECURE RANDOM SOURCE"))?;
    Ok(move || {
        let mut bytes = [0; 8];
        // A failure after the probe leaves zeros; the registry then retries
        // and reports capacity rather than reusing an open code.
        let _ = random.fill(&mut bytes);
        // Rejection-free: 2^64 is far larger than the code space, so the
        // modulo bias is below one part in 2^34.
        u64::from_le_bytes(bytes) % RoomCode::COUNT
    })
}
