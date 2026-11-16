//! Internet room relay (#124): room codes, the wire format both ends share,
//! and (with `net`) the QUIC relay service and its client.

pub mod code;
pub mod rooms;
pub mod wire;
#[cfg(feature = "net")]
pub mod client;
#[cfg(feature = "net")]
pub mod service;

pub use code::RoomCode;

/// QUIC settings both ends share.
#[cfg(feature = "net")]
pub(crate) fn transport() -> quinn::TransportConfig {
    use std::time::Duration;
    let mut transport = quinn::TransportConfig::default();
    transport
        .max_idle_timeout(Some(quinn::IdleTimeout::from(quinn::VarInt::from_u32(10_000))))
        .keep_alive_interval(Some(Duration::from_secs(2)))
        // Stay at QUIC's smallest packet size so every relayed datagram fits
        // any path that carries QUIC at all.
        .initial_mtu(1200)
        .mtu_discovery_config(None)
        .datagram_receive_buffer_size(Some(256 * 1024))
        .datagram_send_buffer_size(256 * 1024)
        .max_concurrent_bidi_streams(quinn::VarInt::from_u32(1))
        .max_concurrent_uni_streams(quinn::VarInt::from_u32(0));
    transport
}
