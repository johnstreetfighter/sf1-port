//! Internet room relay (#124): room codes, the wire format both ends share,
//! and (with `net`) the QUIC relay service and its client.

pub mod code;

pub use code::RoomCode;
