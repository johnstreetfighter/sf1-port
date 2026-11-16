//! What the relay and its clients say to each other.
//!
//! Each client opens one QUIC connection and one bidirectional stream. It
//! sends one [`Request`] and reads one [`Reply`]. Gameplay then travels as
//! QUIC datagrams of one envelope byte (the seat) and an opaque payload:
//! a client names the destination seat, the relay rewrites it to the
//! sender's seat. Guests may only address seat 0, the host.

use crate::rooms::{Refusal, Seat};
use crate::RoomCode;

/// The QUIC application protocol; a changed wire format gets a new name.
pub const ALPN: &[u8] = b"sf1-relay/1";
pub const DEFAULT_PORT: u16 = 7443;
/// The largest gameplay payload the relay forwards.
pub const MAX_PAYLOAD: usize = 512;
pub const ENVELOPE: usize = 1;
pub const MAX_DATAGRAM: usize = MAX_PAYLOAD + ENVELOPE;
/// Requests and replies are one kind byte and six code bytes.
pub const MESSAGE: usize = 7;

/// QUIC application close codes.
pub mod close {
    pub const LEFT: u32 = 0;
    pub const ROOM_CLOSED: u32 = 1;
    pub const ROOM_EXPIRED: u32 = 2;
    pub const REFUSED: u32 = 3;
    pub const PROTOCOL: u32 = 4;
    pub const SHUTDOWN: u32 = 5;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Create,
    Join(RoomCode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    Created(RoomCode),
    Joined(Seat),
    Refused(Refusal),
}

impl Request {
    pub fn encode(self) -> [u8; MESSAGE] {
        let mut bytes = [0; MESSAGE];
        match self {
            Request::Create => bytes[0] = 1,
            Request::Join(code) => {
                bytes[0] = 2;
                bytes[1..].copy_from_slice(&code.bytes());
            }
        }
        bytes
    }

    pub fn decode(bytes: [u8; MESSAGE]) -> Option<Self> {
        match bytes[0] {
            1 if bytes[1..] == [0; 6] => Some(Request::Create),
            2 => Some(Request::Join(RoomCode::from_bytes(bytes[1..].try_into().ok()?)?)),
            _ => None,
        }
    }
}

impl Reply {
    pub fn encode(self) -> [u8; MESSAGE] {
        let mut bytes = [0; MESSAGE];
        match self {
            Reply::Created(code) => {
                bytes[0] = 1;
                bytes[1..].copy_from_slice(&code.bytes());
            }
            Reply::Joined(seat) => [bytes[0], bytes[1]] = [2, seat],
            Reply::Refused(refusal) => {
                let reason = match refusal {
                    Refusal::NotFound => 1,
                    Refusal::Full => 2,
                    Refusal::Throttled => 3,
                    Refusal::Capacity => 4,
                    Refusal::AlreadyInRoom => 5,
                };
                [bytes[0], bytes[1]] = [3, reason];
            }
        }
        bytes
    }

    pub fn decode(bytes: [u8; MESSAGE]) -> Option<Self> {
        match bytes[0] {
            1 => Some(Reply::Created(RoomCode::from_bytes(bytes[1..].try_into().ok()?)?)),
            2 if usize::from(bytes[1]) < crate::rooms::SEATS => Some(Reply::Joined(bytes[1])),
            3 => Some(Reply::Refused(match bytes[1] {
                1 => Refusal::NotFound,
                2 => Refusal::Full,
                3 => Refusal::Throttled,
                4 => Refusal::Capacity,
                5 => Refusal::AlreadyInRoom,
                _ => return None,
            })),
            _ => None,
        }
    }
}

/// Wrap a payload for `seat`. `None` when the payload is too large.
pub fn seal(seat: Seat, payload: &[u8]) -> Option<Vec<u8>> {
    (payload.len() <= MAX_PAYLOAD).then(|| [&[seat][..], payload].concat())
}

/// Split a datagram into its seat and payload.
pub fn open(datagram: &[u8]) -> Option<(Seat, &[u8])> {
    let (&seat, payload) = datagram.split_first()?;
    (usize::from(seat) < crate::rooms::SEATS && payload.len() <= MAX_PAYLOAD).then_some((seat, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_round_trip() {
        let code = RoomCode::parse("K7Q-MPX").unwrap();
        for request in [Request::Create, Request::Join(code)] {
            assert_eq!(Request::decode(request.encode()), Some(request));
        }
        for reply in [
            Reply::Created(code),
            Reply::Joined(15),
            Reply::Refused(Refusal::NotFound),
            Reply::Refused(Refusal::Full),
            Reply::Refused(Refusal::Throttled),
            Reply::Refused(Refusal::Capacity),
            Reply::Refused(Refusal::AlreadyInRoom),
        ] {
            assert_eq!(Reply::decode(reply.encode()), Some(reply));
        }
        assert_eq!(Request::decode([9; MESSAGE]), None);
        assert_eq!(Reply::decode([2, 16, 0, 0, 0, 0, 0]), None);
    }

    #[test]
    fn the_largest_payload_fits_one_envelope() {
        let payload = [0xa5; MAX_PAYLOAD];
        let sealed = seal(3, &payload).unwrap();
        assert_eq!(sealed.len(), MAX_DATAGRAM);
        assert_eq!(open(&sealed), Some((3, &payload[..])));
        assert_eq!(seal(0, &[0; MAX_PAYLOAD + 1]), None);
        assert_eq!(open(&[16, 1, 2]), None);
        assert_eq!(open(&[]), None);
    }
}
