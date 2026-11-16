//! Internet room relay (#124): room codes, the wire format both ends share,
//! and (with `net`) the QUIC relay service and its client.

pub mod code;
pub mod rooms;

pub use code::RoomCode;
