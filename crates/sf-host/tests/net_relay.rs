//! The host transport against a real relay on localhost (#124).
use std::time::{Duration, Instant};

use sf_engine::config::MatchConfig;
use sf_engine::front::fight::FightDips;
use sf_engine::netplay::protocol::tag_from_str;
use sf_engine::netplay::{
    Link, NetPacket, NetTransportCommand as Command, NetTransportEvent as Event, RoomAnnouncement, RoomCode,
    RoomKind, SyncedMatchRules,
};
use sf_engine::roster::{StagePick, TournamentFighter};
use sf_host::net::HostNetDriver;

#[path = "support/relay.rs"]
mod relay;
use relay::TestRelay;

fn announcement(in_match: bool) -> RoomAnnouncement {
    let rules = SyncedMatchRules::new(
        MatchConfig::definitive(),
        [TournamentFighter::Ryu, TournamentFighter::Ken],
        FightDips::default(),
        StagePick::Bank(0),
        42,
        123,
        456,
    );
    RoomAnnouncement {
        host: tag_from_str("HOST").unwrap(),
        kind: RoomKind::Versus,
        link: Link::Ethernet,
        wired_only: false,
        public: false,
        preset: rules.config.preset,
        in_match,
        spectators: 0,
        max_rollback: 8,
        config_hash: rules.config_hash(),
        bank_hash: 123,
        art_pack_hash: 456,
        protocol_hash: rules.protocol_hash,
    }
}

/// Poll until an event matches, failing on any transport error first.
fn until(driver: &mut HostNetDriver, commands: &[Command], want: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs(6);
    let mut commands = commands.to_vec();
    loop {
        for event in driver.poll(&std::mem::take(&mut commands)) {
            if want(&event) {
                return event;
            }
            assert!(!matches!(event, Event::TransportError { .. }), "{event:?}");
        }
        assert!(Instant::now() < deadline, "the relay did not answer in time");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn error(driver: &mut HostNetDriver, commands: &[Command]) -> String {
    let deadline = Instant::now() + Duration::from_secs(6);
    let mut commands = commands.to_vec();
    loop {
        for event in driver.poll(&std::mem::take(&mut commands)) {
            if let Event::TransportError { message } = event {
                return message;
            }
        }
        assert!(Instant::now() < deadline, "expected a transport error");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn bound(event: Event) -> (String, Link) {
    match event {
        Event::Bound { room_code, link, .. } => (room_code, link),
        other => panic!("expected Bound, got {other:?}"),
    }
}

fn host_room(driver: &mut HostNetDriver) -> String {
    let (code, link) = bound(until(
        driver,
        &[Command::HostInternetRoom { room: announcement(false) }],
        |event| matches!(event, Event::Bound { .. }),
    ));
    assert_eq!(link, Link::Ethernet);
    code
}

fn join(driver: &mut HostNetDriver, code: &str) -> String {
    bound(until(driver, &[Command::JoinInternetRoom { code: code.into() }], |event| {
        matches!(event, Event::Bound { .. })
    }))
    .0
}

fn packet(driver: &mut HostNetDriver) -> (u8, NetPacket) {
    match until(driver, &[], |event| matches!(event, Event::PacketReceived { .. })) {
        Event::PacketReceived { from_peer, bytes, .. } => (from_peer, NetPacket::from_bytes(&bytes).unwrap()),
        _ => unreachable!(),
    }
}

fn send(driver: &mut HostNetDriver, to_peer: u8, packet: &NetPacket) {
    let events = driver.poll(&[Command::SendPacket { to_peer, bytes: packet.to_bytes() }]);
    assert!(events.iter().all(|event| !matches!(event, Event::TransportError { .. })), "{events:?}");
}

#[test]
fn an_internet_room_shows_its_code_only_once_the_relay_has_the_room() {
    let relay = TestRelay::start();
    let mut host = relay.driver();
    let first = host.poll(&[Command::HostInternetRoom { room: announcement(false) }]);
    assert!(
        !first.iter().any(|event| matches!(event, Event::Bound { .. })),
        "no code before the relay answers: {first:?}"
    );
    let code = host_room(&mut host);
    let parsed = RoomCode::parse(&code).expect("an Internet room code");
    assert_eq!(parsed.to_string(), code);
    assert_eq!(relay.service.open_rooms(), 1);
    // The code is ready to use the moment it is shown, typed in any case.
    let mut guest = relay.driver();
    assert_eq!(join(&mut guest, &code.to_ascii_lowercase()), code);
}

#[test]
fn rollback_packets_cross_the_relay_by_route_both_ways() {
    let relay = TestRelay::start();
    let mut host = relay.driver();
    let code = host_room(&mut host);
    let mut guest = relay.driver();
    join(&mut guest, &code);

    let ping = NetPacket::Ping { seq: 7, sent_ms: 1234, link: Link::Wifi };
    send(&mut guest, 0, &ping);
    assert_eq!(packet(&mut host), (1, ping));
    let pong = NetPacket::Pong { seq: 7, echo_ms: 1234, link: Link::Ethernet };
    send(&mut host, 1, &pong);
    assert_eq!(packet(&mut guest), (0, pong));

    // A second guest gets its own route; the first never hears it.
    let mut spectator = relay.driver();
    join(&mut spectator, &code);
    let hello = NetPacket::Ping { seq: 9, sent_ms: 99, link: Link::Ethernet };
    send(&mut spectator, 0, &hello);
    assert_eq!(packet(&mut host), (2, hello));
    std::thread::sleep(Duration::from_millis(150));
    assert!(!guest.poll(&[]).iter().any(|event| matches!(event, Event::PacketReceived { .. })));
}

#[test]
fn announcement_updates_keep_the_same_room_and_code() {
    let relay = TestRelay::start();
    let mut host = relay.driver();
    let code = host_room(&mut host);
    for in_match in [true, false, true] {
        let events = host.poll(&[Command::HostInternetRoom { room: announcement(in_match) }]);
        assert!(
            events.iter().all(|event| !matches!(event, Event::Bound { .. } | Event::TransportError { .. })),
            "{events:?}"
        );
    }
    let mut guest = relay.driver();
    assert_eq!(join(&mut guest, &code), code);
    assert_eq!(relay.service.open_rooms(), 1);
}

#[test]
fn closing_the_host_ends_the_room_and_its_code() {
    let relay = TestRelay::start();
    let mut host = relay.driver();
    let code = host_room(&mut host);
    let mut guest = relay.driver();
    join(&mut guest, &code);
    host.poll(&[Command::Close]);
    assert_eq!(error(&mut guest, &[]), "ROOM CLOSED");
    let mut late = relay.driver();
    let message = error(&mut late, &[Command::JoinInternetRoom { code: code.clone() }]);
    assert!(message.contains("NOT FOUND") && message.contains(&relay.address()), "{message}");
    // A new room on the same driver gets a new code.
    let next = host_room(&mut host);
    assert_ne!(next, code);
}

#[test]
fn internet_rooms_need_a_relay_and_a_valid_code() {
    let relay = TestRelay::start();
    let mut unset = relay.driver();
    unset.set_relay(None);
    assert_eq!(
        error(&mut unset, &[Command::HostInternetRoom { room: announcement(false) }]),
        "NO INTERNET RELAY SET"
    );
    let mut guest = relay.driver();
    assert_eq!(
        error(&mut guest, &[Command::JoinInternetRoom { code: "SF1-7F00-0001".into() }]),
        "INVALID ROOM CODE"
    );
}

#[test]
fn an_unreachable_relay_reports_an_error_without_blocking_the_frame() {
    let relay = TestRelay::start();
    let mut host = relay.driver();
    host.set_relay(Some("relay.invalid".into()));
    let started = Instant::now();
    host.poll(&[Command::HostInternetRoom { room: announcement(false) }]);
    assert!(started.elapsed() < Duration::from_millis(100));
    let message = error(&mut host, &[]);
    assert!(message.starts_with("RELAY"), "{message}");
}

#[test]
fn lan_hosting_still_works_after_an_internet_room() {
    let relay = TestRelay::start();
    let mut host = relay.driver();
    host_room(&mut host);
    host.poll(&[Command::Close]);
    let (code, _) = bound(until(
        &mut host,
        &[Command::BindHost { port: 0, room: announcement(false) }],
        |event| matches!(event, Event::Bound { .. }),
    ));
    assert!(code.starts_with("SF1-"), "{code}");
    let deadline = Instant::now() + Duration::from_secs(6);
    while relay.service.open_rooms() != 0 {
        assert!(Instant::now() < deadline, "leaving for LAN play closes the relay room");
        std::thread::sleep(Duration::from_millis(5));
    }
}

// #128: a community relay beside the default one.

#[test]
fn a_join_is_looked_up_only_on_the_selected_relay_and_names_it_on_failure() {
    let (default, community) = (TestRelay::start(), TestRelay::start());
    let mut host = default.driver();
    let code = host_room(&mut host);

    // The guest has the community relay selected; the code lives on the default.
    let mut guest = community.driver_trusting(&[&default, &community]);
    let message = error(&mut guest, &[Command::JoinInternetRoom { code: code.clone() }]);
    assert_eq!(message, format!("ROOM CODE NOT FOUND ON {}", community.address()));
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(default.service.connections(), 1, "the default relay seated the host and no one else");
    assert!(!host.poll(&[]).iter().any(|event| matches!(event, Event::PacketReceived { .. })));

    // Switching back to the default relay finds the room.
    guest.set_relay(Some(default.address()));
    assert_eq!(join(&mut guest, &code), code);
    assert_eq!(default.service.connections(), 2);
    assert_eq!(community.service.connections(), 0);
}

#[test]
fn a_community_relay_carries_a_room_while_the_default_relay_is_down() {
    let (default, community) = (TestRelay::start(), TestRelay::start());
    let default_address = default.address();
    drop(default);

    let mut host = community.driver();
    let code = host_room(&mut host);
    let mut guest = community.driver();
    join(&mut guest, &code);
    let ping = NetPacket::Ping { seq: 1, sent_ms: 5, link: Link::Ethernet };
    send(&mut guest, 0, &ping);
    assert_eq!(packet(&mut host), (1, ping));

    // The relay that is down fails by name, and the next room can go elsewhere.
    let mut other = community.driver();
    other.set_relay(Some(default_address.clone()));
    let message = error(&mut other, &[Command::HostInternetRoom { room: announcement(false) }]);
    assert!(message.starts_with(&format!("RELAY {default_address} ")), "{message}");
    other.set_relay(Some(community.address()));
    assert_ne!(host_room(&mut other), code);
}

#[test]
fn a_relay_whose_certificate_is_not_trusted_is_refused_by_name() {
    let (trusted, imposter) = (TestRelay::start(), TestRelay::start());
    let mut host = imposter.driver_trusting(&[&trusted]);
    let message = error(&mut host, &[Command::HostInternetRoom { room: announcement(false) }]);
    assert!(message.starts_with(&format!("RELAY {} CONNECT", imposter.address())), "{message}");
    assert_eq!(imposter.service.open_rooms(), 0);
    assert_eq!(imposter.service.connections(), 0);
}
