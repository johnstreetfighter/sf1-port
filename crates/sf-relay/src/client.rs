//! The game's side of a relay connection.
//!
//! A worker thread owns the QUIC connection. The game thread hands it
//! datagrams and reads its events through bounded queues and never waits on
//! the network.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use quinn::crypto::rustls::QuicClientConfig;
use rustls::pki_types::CertificateDer;
use tokio::sync::mpsc as async_mpsc;

use crate::rooms::{Refusal, Seat};
use crate::wire::{self, close, Reply, Request};
use crate::RoomCode;

const QUEUE: usize = 256;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Which certificate authorities a client accepts for its relay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Roots {
    /// The public web authorities: the official clients' default.
    Web,
    /// Only these DER certificates, for tests and private relays.
    Only(Vec<Vec<u8>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientConfig {
    /// `host` or `host:port`; the port defaults to [`wire::DEFAULT_PORT`].
    pub relay: String,
    pub roots: Roots,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Closed {
    /// The host left, so the room is gone.
    RoomClosed,
    /// The room reached the relay's idle or age limit.
    RoomExpired,
    /// The relay is shutting down.
    Shutdown,
    /// The connection timed out or failed.
    Lost(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientEvent {
    /// The room is open (host) or joined (guest).
    Ready { code: RoomCode, seat: Seat, relay_ip: IpAddr, max_datagram: usize },
    Datagram { from: Seat, bytes: Vec<u8> },
    Refused(Refusal),
    Closed(Closed),
    /// The relay could not be reached or verified.
    Failed(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    TooLarge,
    QueueFull,
    Closed,
}

pub struct RelayClient {
    outgoing: async_mpsc::Sender<Vec<u8>>,
    events: mpsc::Receiver<ClientEvent>,
}

impl RelayClient {
    /// Start a worker that connects and makes `request`. Only starting the
    /// thread can fail here; connection failures arrive as events.
    pub fn open(config: ClientConfig, request: Request) -> io::Result<Self> {
        let (outgoing, datagrams) = async_mpsc::channel(QUEUE);
        let (events_tx, events) = mpsc::sync_channel(QUEUE);
        std::thread::Builder::new().name("sf-relay-client".into()).spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = events_tx.send(ClientEvent::Failed(format!("RELAY CLIENT: {error}")));
                    return;
                }
            };
            runtime.block_on(run(config, request, datagrams, events_tx));
        })?;
        Ok(Self { outgoing, events })
    }

    /// Queue a payload for `to` (guests always address seat 0).
    pub fn send(&self, to: Seat, payload: &[u8]) -> Result<(), SendError> {
        let sealed = wire::seal(to, payload).ok_or(SendError::TooLarge)?;
        self.outgoing.try_send(sealed).map_err(|error| match error {
            async_mpsc::error::TrySendError::Full(_) => SendError::QueueFull,
            async_mpsc::error::TrySendError::Closed(_) => SendError::Closed,
        })
    }

    pub fn poll(&self) -> Option<ClientEvent> {
        self.events.try_recv().ok()
    }
}

async fn run(
    config: ClientConfig,
    request: Request,
    mut datagrams: async_mpsc::Receiver<Vec<u8>>,
    events: mpsc::SyncSender<ClientEvent>,
) {
    let session = match tokio::time::timeout(CONNECT_TIMEOUT, connect(&config, request)).await {
        Ok(Ok(session)) => session,
        Ok(Err(Outcome::Refused(refusal))) => {
            let _ = events.send(ClientEvent::Refused(refusal));
            return;
        }
        Ok(Err(Outcome::Failed(message))) => {
            let _ = events.send(ClientEvent::Failed(message));
            return;
        }
        Err(_) => {
            let _ = events.send(ClientEvent::Failed(format!("RELAY {} DID NOT ANSWER", config.relay)));
            return;
        }
    };
    let Session { endpoint, connection, code, seat } = session;
    let ready = ClientEvent::Ready {
        code,
        seat,
        relay_ip: connection.remote_address().ip(),
        max_datagram: connection.max_datagram_size().unwrap_or(0),
    };
    if events.send(ready).is_err() {
        return;
    }
    loop {
        tokio::select! {
            outgoing = datagrams.recv() => match outgoing {
                Some(sealed) => {
                    // Gameplay is unreliable by design; rollback absorbs loss.
                    let _ = connection.send_datagram(sealed.into());
                }
                None => {
                    // The game dropped the client: leave the room.
                    connection.close(close::LEFT.into(), b"left");
                    let _ = tokio::time::timeout(Duration::from_secs(1), endpoint.wait_idle()).await;
                    return;
                }
            },
            incoming = connection.read_datagram() => match incoming {
                Ok(datagram) => {
                    if let Some((from, payload)) = wire::open(&datagram) {
                        // A full queue means the game is stalled; drop rather
                        // than block the connection.
                        let _ = events.try_send(ClientEvent::Datagram { from, bytes: payload.to_vec() });
                    }
                }
                Err(error) => {
                    let _ = events.send(ClientEvent::Closed(closed(&error)));
                    return;
                }
            },
        }
    }
}

struct Session {
    endpoint: quinn::Endpoint,
    connection: quinn::Connection,
    code: RoomCode,
    seat: Seat,
}

enum Outcome {
    Refused(Refusal),
    Failed(String),
}

/// A health check: reach `config.relay`, verify its certificate and the
/// relay protocol, and leave without asking for a room. Returns the address
/// that answered. It blocks for up to the connect timeout.
pub fn check(config: &ClientConfig) -> Result<SocketAddr, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("RELAY CLIENT: {error}"))?;
    runtime.block_on(async {
        match tokio::time::timeout(CONNECT_TIMEOUT, dial(config)).await {
            Ok(Ok((endpoint, connection))) => {
                let address = connection.remote_address();
                connection.close(close::LEFT.into(), b"check");
                let _ = tokio::time::timeout(Duration::from_secs(1), endpoint.wait_idle()).await;
                Ok(address)
            }
            Ok(Err(message)) => Err(message),
            Err(_) => Err(format!("RELAY {} DID NOT ANSWER", config.relay.trim())),
        }
    })
}

/// Connect and verify the relay, before any request.
async fn dial(config: &ClientConfig) -> Result<(quinn::Endpoint, quinn::Connection), String> {
    let failed = |what: &str, error: &dyn std::fmt::Display| failure(config, what, error);
    let (host, port) = crate::endpoint::split(&config.relay)
        .ok_or_else(|| format!("INVALID RELAY ADDRESS {}", config.relay))?;
    let address = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|error| failed("LOOKUP", &error))?
        .min_by_key(|address| !address.is_ipv4())
        .ok_or_else(|| format!("RELAY {host} HAS NO ADDRESS"))?;
    let bind: SocketAddr = if address.is_ipv4() { ([0, 0, 0, 0], 0).into() } else { ([0u16; 8], 0).into() };
    let endpoint = quinn::Endpoint::client(bind).map_err(|error| failed("SOCKET", &error))?;
    let connection = endpoint
        .connect_with(client_config(&config.roots)?, address, &host)
        .map_err(|error| failed("CONNECT", &error))?
        .await
        .map_err(|error| failed("CONNECT", &error))?;
    Ok((endpoint, connection))
}

/// Every failure names the relay it was looking at.
fn failure(config: &ClientConfig, what: &str, error: &dyn std::fmt::Display) -> String {
    format!("RELAY {} {what}: {error}", config.relay.trim())
}

async fn connect(config: &ClientConfig, request: Request) -> Result<Session, Outcome> {
    let failed = |what: &str, error: &dyn std::fmt::Display| Outcome::Failed(failure(config, what, error));
    let (endpoint, connection) = dial(config).await.map_err(Outcome::Failed)?;
    let (mut send, mut recv) = connection.open_bi().await.map_err(|error| failed("STREAM", &error))?;
    send.write_all(&request.encode()).await.map_err(|error| failed("REQUEST", &error))?;
    let mut reply = [0; wire::MESSAGE];
    recv.read_exact(&mut reply).await.map_err(|error| failed("REPLY", &error))?;
    let (code, seat) = match (Reply::decode(reply), request) {
        (Some(Reply::Created(code)), Request::Create) => (code, 0),
        (Some(Reply::Joined(seat)), Request::Join(code)) if seat != 0 => (code, seat),
        (Some(Reply::Refused(refusal)), _) => {
            connection.close(close::LEFT.into(), b"refused");
            return Err(Outcome::Refused(refusal));
        }
        _ => {
            connection.close(close::PROTOCOL.into(), b"protocol");
            return Err(Outcome::Failed("RELAY SPOKE AN UNKNOWN PROTOCOL".into()));
        }
    };
    Ok(Session { endpoint, connection, code, seat })
}

fn client_config(roots: &Roots) -> Result<quinn::ClientConfig, String> {
    let mut store = rustls::RootCertStore::empty();
    match roots {
        Roots::Web => store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned()),
        Roots::Only(certificates) => {
            for certificate in certificates {
                store
                    .add(CertificateDer::from(certificate.clone()))
                    .map_err(|error| format!("RELAY CERTIFICATE: {error}"))?;
            }
        }
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| format!("RELAY TLS: {error}"))?
        .with_root_certificates(store)
        .with_no_client_auth();
    tls.alpn_protocols = vec![wire::ALPN.to_vec()];
    let crypto = QuicClientConfig::try_from(tls).map_err(|error| format!("RELAY TLS: {error}"))?;
    let mut config = quinn::ClientConfig::new(Arc::new(crypto));
    config.transport_config(Arc::new(crate::transport()));
    Ok(config)
}

fn closed(error: &quinn::ConnectionError) -> Closed {
    match error {
        quinn::ConnectionError::ApplicationClosed(reason) => match reason.error_code.into_inner() {
            code if code == u64::from(close::ROOM_CLOSED) => Closed::RoomClosed,
            code if code == u64::from(close::ROOM_EXPIRED) => Closed::RoomExpired,
            code if code == u64::from(close::SHUTDOWN) => Closed::Shutdown,
            _ => Closed::Lost(error.to_string()),
        },
        other => Closed::Lost(other.to_string()),
    }
}
