//! The relay's room rules, free of sockets and clocks: callers pass the time.
//!
//! A host connection opens a room and gets its code. Guests join by code and
//! take seats 1 to 15; the host is seat 0. A guest's datagrams go only to the
//! host, and the host addresses one seat at a time, so rooms never see each
//! other's traffic. Every limit here bounds service state or traffic.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use crate::RoomCode;

/// The relay's own name for one client connection.
pub type ConnId = u64;
/// A seat in a room: 0 is the host, 1 to 15 are guests.
pub type Seat = u8;

pub const SEATS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_rooms: usize,
    /// A room with no traffic for this long closes and its code expires.
    pub max_idle: Duration,
    /// No room outlives this, busy or not.
    pub max_age: Duration,
    /// Per client address.
    pub creates_per_minute: u32,
    /// Per client address, counting failed attempts.
    pub joins_per_minute: u32,
    /// Per connection; one second's worth may arrive at once.
    pub datagrams_per_second: u32,
    /// Per connection; one second's worth may arrive at once.
    pub bytes_per_second: u32,
    /// Addresses tracked for throttling; new addresses beyond this are refused.
    pub max_tracked_addresses: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_rooms: 4096,
            max_idle: Duration::from_secs(15 * 60),
            max_age: Duration::from_secs(12 * 60 * 60),
            creates_per_minute: 10,
            joins_per_minute: 30,
            // A host sends about 60 packets a second to each of up to five
            // peers (a challenger and four spectators).
            datagrams_per_second: 600,
            bytes_per_second: 256 * 1024,
            max_tracked_addresses: 16 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotFound,
    Full,
    Throttled,
    /// The relay holds as many rooms as it allows.
    Capacity,
    AlreadyInRoom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Departure {
    /// The host left: the room is gone and these guests must be closed.
    Host { code: RoomCode, guests: Vec<ConnId> },
    /// A guest left the host's room, freeing its seat.
    Guest { host: ConnId, member: Seat },
    None,
}

struct Bucket {
    capacity: f64,
    tokens: f64,
    last: Instant,
}

impl Bucket {
    fn full(capacity: u32, now: Instant) -> Self {
        Self { capacity: f64::from(capacity), tokens: f64::from(capacity), last: now }
    }

    /// Refill at `capacity` per `period`.
    fn refill(&mut self, now: Instant, period: Duration) {
        let elapsed = now.saturating_duration_since(self.last).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.capacity / period.as_secs_f64()).min(self.capacity);
        self.last = now;
    }

    fn is_full(&self) -> bool {
        self.tokens >= self.capacity
    }
}

struct Room {
    seats: [Option<ConnId>; SEATS],
    created: Instant,
    active: Instant,
}

struct Member {
    code: RoomCode,
    seat: Seat,
    datagrams: Bucket,
    bytes: Bucket,
}

struct Throttle {
    creates: Bucket,
    joins: Bucket,
}

pub struct Registry {
    limits: Limits,
    codes: Box<dyn FnMut() -> u64 + Send>,
    rooms: HashMap<RoomCode, Room>,
    members: HashMap<ConnId, Member>,
    throttles: HashMap<IpAddr, Throttle>,
}

const MINUTE: Duration = Duration::from_secs(60);
const SECOND: Duration = Duration::from_secs(1);
/// Fresh codes tried before a crowded code space counts as full.
const CODE_TRIES: usize = 64;

impl Registry {
    /// `codes` supplies random code indexes; the service uses the system
    /// random source, tests a fixed sequence.
    pub fn new(limits: Limits, codes: impl FnMut() -> u64 + Send + 'static) -> Self {
        Self {
            limits,
            codes: Box::new(codes),
            rooms: HashMap::new(),
            members: HashMap::new(),
            throttles: HashMap::new(),
        }
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    pub fn open_rooms(&self) -> usize {
        self.rooms.len()
    }

    pub fn create(&mut self, conn: ConnId, from: IpAddr, now: Instant) -> Result<RoomCode, Refusal> {
        if self.members.contains_key(&conn) {
            return Err(Refusal::AlreadyInRoom);
        }
        self.spend_throttle(from, now, |throttle| &mut throttle.creates, MINUTE)?;
        if self.rooms.len() >= self.limits.max_rooms {
            return Err(Refusal::Capacity);
        }
        let code = (0..CODE_TRIES)
            .map(|_| RoomCode::from_index((self.codes)()))
            .find(|code| !self.rooms.contains_key(code))
            .ok_or(Refusal::Capacity)?;
        let mut seats = [None; SEATS];
        seats[0] = Some(conn);
        self.rooms.insert(code, Room { seats, created: now, active: now });
        self.seat(conn, code, 0, now);
        Ok(code)
    }

    pub fn join(&mut self, conn: ConnId, from: IpAddr, code: RoomCode, now: Instant) -> Result<Seat, Refusal> {
        if self.members.contains_key(&conn) {
            return Err(Refusal::AlreadyInRoom);
        }
        self.spend_throttle(from, now, |throttle| &mut throttle.joins, MINUTE)?;
        let room = self.rooms.get_mut(&code).ok_or(Refusal::NotFound)?;
        let seat = room.seats.iter().position(Option::is_none).ok_or(Refusal::Full)?;
        room.seats[seat] = Some(conn);
        room.active = now;
        self.seat(conn, code, seat as Seat, now);
        Ok(seat as Seat)
    }

    /// Where a datagram from `conn` addressed to seat `to` goes, and the seat
    /// to stamp on it as its sender. `None` drops it: an unknown sender, an
    /// empty or forbidden seat, or a spent traffic budget.
    pub fn route(&mut self, conn: ConnId, to: Seat, len: usize, now: Instant) -> Option<(ConnId, Seat)> {
        let member = self.members.get_mut(&conn)?;
        member.datagrams.refill(now, SECOND);
        member.bytes.refill(now, SECOND);
        let bytes = len as f64;
        if member.datagrams.tokens < 1.0 || member.bytes.tokens < bytes {
            return None;
        }
        member.datagrams.tokens -= 1.0;
        member.bytes.tokens -= bytes;
        let (code, from) = (member.code, member.seat);
        let room = self.rooms.get_mut(&code)?;
        let target = match (from, to) {
            (0, 0) => None,
            (0, guest) => room.seats.get(usize::from(guest)).copied().flatten(),
            (_, 0) => room.seats[0],
            _ => None,
        }?;
        room.active = now;
        Some((target, from))
    }

    pub fn leave(&mut self, conn: ConnId) -> Departure {
        let Some(member) = self.members.remove(&conn) else {
            return Departure::None;
        };
        if member.seat == 0 {
            let Some(room) = self.rooms.remove(&member.code) else {
                return Departure::None;
            };
            let guests: Vec<_> = room.seats[1..].iter().flatten().copied().collect();
            for guest in &guests {
                self.members.remove(guest);
            }
            return Departure::Host { code: member.code, guests };
        }
        let Some(room) = self.rooms.get_mut(&member.code) else {
            return Departure::None;
        };
        room.seats[usize::from(member.seat)] = None;
        match room.seats[0] {
            Some(host) => Departure::Guest { host, member: member.seat },
            None => Departure::None,
        }
    }

    /// Close rooms past their idle or age limit. Each closed room comes back
    /// with its connections, host first, for the service to close.
    pub fn expire(&mut self, now: Instant) -> Vec<(RoomCode, Vec<ConnId>)> {
        let limits = self.limits;
        let mut expired: Vec<_> = self
            .rooms
            .iter()
            .filter(|(_, room)| {
                now.saturating_duration_since(room.created) > limits.max_age
                    || now.saturating_duration_since(room.active) > limits.max_idle
            })
            .map(|(code, _)| *code)
            .collect();
        expired.sort();
        let closed = expired
            .into_iter()
            .filter_map(|code| {
                let room = self.rooms.remove(&code)?;
                let conns: Vec<_> = room.seats.iter().flatten().copied().collect();
                for conn in &conns {
                    self.members.remove(conn);
                }
                Some((code, conns))
            })
            .collect();
        self.throttles.retain(|_, throttle| {
            throttle.creates.refill(now, MINUTE);
            throttle.joins.refill(now, MINUTE);
            !(throttle.creates.is_full() && throttle.joins.is_full())
        });
        closed
    }

    fn seat(&mut self, conn: ConnId, code: RoomCode, seat: Seat, now: Instant) {
        self.members.insert(conn, Member {
            code,
            seat,
            datagrams: Bucket::full(self.limits.datagrams_per_second, now),
            bytes: Bucket::full(self.limits.bytes_per_second, now),
        });
    }

    fn spend_throttle(
        &mut self,
        from: IpAddr,
        now: Instant,
        bucket: fn(&mut Throttle) -> &mut Bucket,
        period: Duration,
    ) -> Result<(), Refusal> {
        if !self.throttles.contains_key(&from) && self.throttles.len() >= self.limits.max_tracked_addresses {
            return Err(Refusal::Throttled);
        }
        let limits = self.limits;
        let throttle = self.throttles.entry(from).or_insert_with(|| Throttle {
            creates: Bucket::full(limits.creates_per_minute, now),
            joins: Bucket::full(limits.joins_per_minute, now),
        });
        let bucket = bucket(throttle);
        bucket.refill(now, period);
        if bucket.tokens < 1.0 {
            return Err(Refusal::Throttled);
        }
        bucket.tokens -= 1.0;
        Ok(())
    }
}
