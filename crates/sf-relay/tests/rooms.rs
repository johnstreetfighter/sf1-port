//! The relay's room rules on their own: codes, seats, routing, limits and
//! expiry, with no sockets.
use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};

use sf_relay::rooms::{Departure, Limits, Refusal, Registry};
use sf_relay::RoomCode;

const HOME: IpAddr = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 7));
const AWAY: IpAddr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9));

fn registry() -> Registry {
    let mut next = 0;
    Registry::new(Limits::default(), move || {
        next += 1;
        next * 7_919
    })
}

#[test]
fn a_guest_joins_by_code_and_packets_reach_only_the_other_end() {
    let now = Instant::now();
    let mut rooms = registry();
    let code = rooms.create(1, HOME, now).expect("room opens");
    assert_eq!(rooms.join(2, AWAY, code, now), Ok(1));
    // Guests address the host as member 0; the host sees the guest's seat.
    assert_eq!(rooms.route(2, 0, 512, now), Some((1, 1)));
    assert_eq!(rooms.route(1, 1, 512, now), Some((2, 0)));
    // Nobody reaches an empty seat, and guests never reach each other.
    assert_eq!(rooms.route(1, 2, 512, now), None);
    assert_eq!(rooms.join(3, AWAY, code, now), Ok(2));
    assert_eq!(rooms.route(2, 2, 512, now), None);
    assert_eq!(rooms.route(3, 0, 512, now), Some((1, 2)));
    // A connection outside every room routes nowhere.
    assert_eq!(rooms.route(99, 0, 512, now), None);
}

#[test]
fn two_rooms_never_see_each_others_packets() {
    let now = Instant::now();
    let mut rooms = registry();
    let first = rooms.create(1, HOME, now).unwrap();
    let second = rooms.create(10, AWAY, now).unwrap();
    assert_ne!(first, second);
    assert_eq!(rooms.join(2, AWAY, first, now), Ok(1));
    assert_eq!(rooms.join(11, HOME, second, now), Ok(1));
    assert_eq!(rooms.route(2, 0, 64, now), Some((1, 1)));
    assert_eq!(rooms.route(11, 0, 64, now), Some((10, 1)));
    assert_eq!(rooms.route(1, 1, 64, now), Some((2, 0)));
    assert_eq!(rooms.route(10, 1, 64, now), Some((11, 0)));
}

#[test]
fn codes_name_open_rooms_only_and_closing_the_host_ends_the_room() {
    let now = Instant::now();
    let mut rooms = registry();
    assert_eq!(rooms.join(2, AWAY, RoomCode::parse("K7Q-MPX").unwrap(), now), Err(Refusal::NotFound));
    let code = rooms.create(1, HOME, now).unwrap();
    rooms.join(2, AWAY, code, now).unwrap();
    rooms.join(3, AWAY, code, now).unwrap();
    assert_eq!(rooms.leave(3), Departure::Guest { host: 1, member: 2 });
    assert_eq!(rooms.leave(1), Departure::Host { code, guests: vec![2] });
    assert_eq!(rooms.join(4, AWAY, code, now), Err(Refusal::NotFound));
    assert_eq!(rooms.route(2, 0, 64, now), None);
    assert_eq!(rooms.leave(2), Departure::None);
    assert_eq!(rooms.open_rooms(), 0);
}

#[test]
fn a_room_holds_the_host_and_fifteen_guests() {
    let now = Instant::now();
    let mut rooms = registry();
    let code = rooms.create(100, HOME, now).unwrap();
    for guest in 1..=15 {
        assert_eq!(rooms.join(guest, IpAddr::V4(Ipv4Addr::new(10, 0, 0, guest as u8)), code, now), Ok(guest as u8));
    }
    assert_eq!(rooms.join(16, AWAY, code, now), Err(Refusal::Full));
    // A freed seat is reused.
    rooms.leave(4);
    assert_eq!(rooms.join(16, AWAY, code, now), Ok(4));
}

#[test]
fn one_connection_holds_one_seat() {
    let now = Instant::now();
    let mut rooms = registry();
    let code = rooms.create(1, HOME, now).unwrap();
    assert_eq!(rooms.create(1, HOME, now), Err(Refusal::AlreadyInRoom));
    assert_eq!(rooms.join(1, HOME, code, now), Err(Refusal::AlreadyInRoom));
    rooms.join(2, AWAY, code, now).unwrap();
    assert_eq!(rooms.create(2, AWAY, now), Err(Refusal::AlreadyInRoom));
}

#[test]
fn codes_expire_when_a_room_goes_idle_or_reaches_its_age_limit() {
    let start = Instant::now();
    let limits = Limits::default();
    let mut rooms = registry();
    let idle = rooms.create(1, HOME, start).unwrap();
    let busy = rooms.create(2, AWAY, start).unwrap();
    rooms.join(3, AWAY, busy, start).unwrap();
    // Traffic keeps the busy room alive past the idle limit.
    let mut now = start;
    while now < start + limits.max_idle + Duration::from_secs(1) {
        now += Duration::from_secs(30);
        assert!(rooms.route(3, 0, 64, now).is_some());
    }
    assert_eq!(rooms.expire(now), vec![(idle, vec![1])]);
    assert_eq!(rooms.join(4, HOME, idle, now), Err(Refusal::NotFound));
    // No room outlives its age limit, busy or not.
    let old = start + limits.max_age + Duration::from_secs(1);
    assert!(rooms.route(3, 0, 64, old).is_some());
    assert_eq!(rooms.expire(old), vec![(busy, vec![2, 3])]);
    assert_eq!(rooms.open_rooms(), 0);
}

#[test]
fn creating_and_joining_are_throttled_per_address() {
    let start = Instant::now();
    let limits = Limits::default();
    let mut rooms = registry();
    for conn in 0..u64::from(limits.creates_per_minute) {
        rooms.create(conn, HOME, start).unwrap();
        rooms.leave(conn);
    }
    assert_eq!(rooms.create(500, HOME, start), Err(Refusal::Throttled));
    assert!(rooms.create(501, AWAY, start).is_ok(), "other addresses are unaffected");
    assert!(rooms.create(500, HOME, start + Duration::from_secs(61)).is_ok());

    // Failed guesses spend the join allowance too.
    let wrong = RoomCode::parse("K7Q-MPX").unwrap();
    for conn in 0..u64::from(limits.joins_per_minute) {
        assert_eq!(rooms.join(1000 + conn, AWAY, wrong, start), Err(Refusal::NotFound));
    }
    assert_eq!(rooms.join(2000, AWAY, wrong, start), Err(Refusal::Throttled));
    assert_eq!(
        rooms.join(2000, AWAY, wrong, start + Duration::from_secs(61)),
        Err(Refusal::NotFound)
    );
}

#[test]
fn each_connection_has_a_traffic_budget_that_refills() {
    let start = Instant::now();
    let limits = Limits::default();
    let mut rooms = registry();
    let code = rooms.create(1, HOME, start).unwrap();
    rooms.join(2, AWAY, code, start).unwrap();
    let sent = (0..limits.datagrams_per_second * 2)
        .filter(|_| rooms.route(2, 0, 16, start).is_some())
        .count();
    assert_eq!(sent, limits.datagrams_per_second as usize, "one second's burst, then drops");
    assert!(rooms.route(1, 1, 16, start).is_some(), "the host's budget is its own");
    assert!(rooms.route(2, 0, 16, start + Duration::from_secs(1)).is_some());

    // Bytes are budgeted as well as datagrams.
    let mut rooms = registry();
    let code = rooms.create(1, HOME, start).unwrap();
    rooms.join(2, AWAY, code, start).unwrap();
    let large = (0..limits.datagrams_per_second)
        .filter(|_| rooms.route(2, 0, 513, start).is_some())
        .count();
    assert_eq!(large, limits.bytes_per_second as usize / 513);
}

#[test]
fn a_full_relay_refuses_new_rooms_and_codes_never_collide() {
    let now = Instant::now();
    let limits = Limits { max_rooms: 3, ..Limits::default() };
    // The code source repeats itself; the registry must still hand out
    // distinct codes.
    let mut calls = 0;
    let mut rooms = Registry::new(limits, move || {
        calls += 1;
        calls / 2
    });
    let codes: Vec<_> = (0..3)
        .map(|conn| rooms.create(conn, IpAddr::V4(Ipv4Addr::new(10, 0, 1, conn as u8)), now).unwrap())
        .collect();
    assert_ne!(codes[0], codes[1]);
    assert_ne!(codes[1], codes[2]);
    assert_ne!(codes[0], codes[2]);
    assert_eq!(rooms.create(9, AWAY, now), Err(Refusal::Capacity));
    assert_eq!(rooms.open_rooms(), 3);
}

#[test]
fn ipv6_clients_share_an_allowance_per_64_prefix() {
    let now = Instant::now();
    let limits = Limits::default();
    let mut rooms = registry();
    let address = |host: u16| IpAddr::V6(std::net::Ipv6Addr::new(0x2001, 0xdb8, 1, 2, 0, 0, 0, host));
    for conn in 0..u64::from(limits.creates_per_minute) {
        rooms.create(conn, address(conn as u16 + 1), now).unwrap();
        rooms.leave(conn);
    }
    assert_eq!(rooms.create(900, address(999), now), Err(Refusal::Throttled));
    let elsewhere = IpAddr::V6(std::net::Ipv6Addr::new(0x2001, 0xdb8, 1, 3, 0, 0, 0, 1));
    assert!(rooms.create(901, elsewhere, now).is_ok());
}

#[test]
fn a_full_address_table_makes_room_by_forgetting_recovered_addresses() {
    let start = Instant::now();
    let limits = Limits { max_tracked_addresses: 4, ..Limits::default() };
    let mut rooms = Registry::new(limits, {
        let mut next = 0;
        move || {
            next += 1;
            next * 7_919
        }
    });
    let wrong = RoomCode::parse("K7Q-MPX").unwrap();
    for host in 1..=4 {
        rooms.join(host, IpAddr::V4(Ipv4Addr::new(10, 9, 0, host as u8)), wrong, start).unwrap_err();
    }
    // While all four are still recovering, a fifth address waits its turn.
    assert_eq!(rooms.join(5, AWAY, wrong, start), Err(Refusal::Throttled));
    // A minute later they have recovered and are forgotten on demand.
    let later = start + Duration::from_secs(61);
    assert_eq!(rooms.join(5, AWAY, wrong, later), Err(Refusal::NotFound));
}
