//! The QUIC relay service with real clients on localhost.
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use sf_relay::client::{ClientConfig, ClientEvent, Closed, RelayClient, Roots};
use sf_relay::rooms::{Limits, Refusal};
use sf_relay::service::{RelayService, ServiceConfig};
use sf_relay::wire::{Request, MAX_DATAGRAM, MAX_PAYLOAD};
use sf_relay::RoomCode;

struct TestRelay {
    service: RelayService,
    certificate: Vec<u8>,
}

impl TestRelay {
    fn start() -> Self {
        Self::start_valid_for(None)
    }

    /// A relay whose certificate expires after `days`, or rcgen's far-off
    /// default.
    fn start_valid_for(days: Option<u64>) -> Self {
        Self::start_with(days, Limits::default())
    }

    fn start_limited(limits: Limits) -> Self {
        Self::start_with(None, limits)
    }

    fn start_with(days: Option<u64>, limits: Limits) -> Self {
        let mut params = rcgen::CertificateParams::new(vec!["127.0.0.1".to_owned()]).unwrap();
        if let Some(days) = days {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
            params.not_after = rcgen::date_time_ymd(1970, 1, 1) + now + Duration::from_secs(days * 86_400);
        }
        let signing_key = rcgen::KeyPair::generate().unwrap();
        let certificate = params.self_signed(&signing_key).unwrap().der().to_vec();
        let service = RelayService::start(ServiceConfig {
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            certificates: vec![certificate.clone()],
            private_key: signing_key.serialize_der(),
            limits,
        })
        .expect("the test relay starts");
        Self { service, certificate }
    }

    fn address(&self) -> String {
        self.service.local_addr().to_string()
    }

    fn client(&self, request: Request) -> RelayClient {
        RelayClient::open(
            ClientConfig { relay: self.address(), roots: Roots::Only(vec![self.certificate.clone()]) },
            request,
        )
        .expect("the client worker starts")
    }
}

/// The next event, waiting at most six seconds.
fn next(client: &RelayClient) -> ClientEvent {
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        if let Some(event) = client.poll() {
            return event;
        }
        assert!(Instant::now() < deadline, "no relay event within six seconds");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn ready(client: &RelayClient) -> (RoomCode, u8, usize) {
    match next(client) {
        ClientEvent::Ready { code, seat, max_datagram, .. } => (code, seat, max_datagram),
        other => panic!("expected Ready, got {other:?}"),
    }
}

fn datagram(client: &RelayClient) -> (u8, Vec<u8>) {
    match next(client) {
        ClientEvent::Datagram { from, bytes } => (from, bytes),
        other => panic!("expected a datagram, got {other:?}"),
    }
}

fn quiet(client: &RelayClient) {
    std::thread::sleep(Duration::from_millis(200));
    assert!(client.poll().is_none(), "this client should have heard nothing");
}

#[test]
fn a_room_carries_the_largest_packet_both_ways_through_the_relay() {
    let relay = TestRelay::start();
    let host = relay.client(Request::Create);
    let (code, seat, max_datagram) = ready(&host);
    assert_eq!(seat, 0);
    assert!(
        max_datagram >= MAX_DATAGRAM,
        "QUIC's smallest path allows {max_datagram} bytes; the relay needs {MAX_DATAGRAM}"
    );
    let guest = relay.client(Request::Join(code));
    let (joined, seat, _) = ready(&guest);
    assert_eq!((joined, seat), (code, 1));

    let largest: Vec<u8> = (0..MAX_PAYLOAD).map(|i| i as u8).collect();
    guest.send(0, &largest).unwrap();
    assert_eq!(datagram(&host), (1, largest.clone()));
    host.send(1, &largest).unwrap();
    assert_eq!(datagram(&guest), (0, largest));
    assert!(host.send(1, &[0; MAX_PAYLOAD + 1]).is_err());
}

#[test]
fn rooms_are_isolated_and_a_closed_host_takes_its_code_with_it() {
    let relay = TestRelay::start();
    let first_host = relay.client(Request::Create);
    let (first, ..) = ready(&first_host);
    let second_host = relay.client(Request::Create);
    let (second, ..) = ready(&second_host);
    assert_ne!(first, second);
    let first_guest = relay.client(Request::Join(first));
    ready(&first_guest);
    let second_guest = relay.client(Request::Join(second));
    ready(&second_guest);

    first_host.send(1, b"first").unwrap();
    assert_eq!(datagram(&first_guest), (0, b"first".to_vec()));
    second_guest.send(0, b"second").unwrap();
    assert_eq!(datagram(&second_host), (1, b"second".to_vec()));
    quiet(&first_host);
    quiet(&second_guest);

    drop(first_host);
    assert_eq!(next(&first_guest), ClientEvent::Closed(Closed::RoomClosed));
    let late = relay.client(Request::Join(first));
    assert_eq!(next(&late), ClientEvent::Refused(Refusal::NotFound));
    // The other room plays on.
    second_host.send(1, b"still here").unwrap();
    assert_eq!(datagram(&second_guest), (0, b"still here".to_vec()));
    assert_eq!(relay.service.open_rooms(), 1);
}

#[test]
fn an_unknown_code_is_refused() {
    let relay = TestRelay::start();
    let guest = relay.client(Request::Join(RoomCode::parse("K7Q-MPX").unwrap()));
    assert_eq!(next(&guest), ClientEvent::Refused(Refusal::NotFound));
}

#[test]
fn a_client_only_trusts_its_configured_certificates() {
    let relay = TestRelay::start();
    let stranger = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
    let client = RelayClient::open(
        ClientConfig { relay: relay.address(), roots: Roots::Only(vec![stranger.cert.der().to_vec()]) },
        Request::Create,
    )
    .unwrap();
    match next(&client) {
        ClientEvent::Failed(message) => assert!(message.to_ascii_lowercase().contains("certificate"), "{message}"),
        other => panic!("an untrusted relay must not be used, got {other:?}"),
    }
    assert_eq!(relay.service.open_rooms(), 0);
}

#[test]
fn an_unreachable_relay_fails_without_blocking() {
    let started = Instant::now();
    let client = RelayClient::open(
        ClientConfig { relay: "relay.invalid:7443".into(), roots: Roots::Only(Vec::new()) },
        Request::Create,
    )
    .unwrap();
    assert!(started.elapsed() < Duration::from_millis(100), "opening never waits for the network");
    assert!(matches!(next(&client), ClientEvent::Failed(_)));
}

#[test]
fn a_health_check_passes_only_for_a_reachable_relay_with_a_trusted_certificate() {
    let relay = TestRelay::start();
    let trusted = ClientConfig { relay: relay.address(), roots: Roots::Only(vec![relay.certificate.clone()]) };
    assert_eq!(sf_relay::client::check(&trusted), Ok(relay.service.local_addr()));
    assert_eq!((relay.service.open_rooms(), relay.service.connections()), (0, 0), "a check takes no room or seat");

    let stranger = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
    let untrusted = ClientConfig { relay: relay.address(), roots: Roots::Only(vec![stranger.cert.der().to_vec()]) };
    let message = sf_relay::client::check(&untrusted).unwrap_err();
    assert!(message.to_ascii_lowercase().contains("certificate"), "{message}");

    drop(relay);
    let message = sf_relay::client::check(&trusted).unwrap_err();
    assert!(message.starts_with("RELAY "), "{message}");
}

#[test]
fn a_health_check_can_require_days_of_certificate_lifetime() {
    for (days, lasts) in [(None, true), (Some(40), true), (Some(5), false)] {
        let relay = TestRelay::start_valid_for(days);
        let config = ClientConfig { relay: relay.address(), roots: Roots::Only(vec![relay.certificate.clone()]) };
        assert_eq!(sf_relay::client::check(&config), Ok(relay.service.local_addr()), "{days:?}: a plain check");
        let checked = sf_relay::client::check_valid_for(&config, 14);
        if lasts {
            assert_eq!(checked, Ok(relay.service.local_addr()), "{days:?}");
        } else {
            let message = checked.unwrap_err();
            assert!(message.contains("EXPIRES WITHIN 14 DAYS"), "{message}");
            assert!(message.starts_with(&format!("RELAY {} ", relay.address())), "{message}");
        }
        assert_eq!((relay.service.open_rooms(), relay.service.connections()), (0, 0), "{days:?}: no room or seat");
    }
}

fn refused(client: &RelayClient) -> Refusal {
    match next(client) {
        ClientEvent::Refused(refusal) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn closed(client: &RelayClient) -> Closed {
    match next(client) {
        ClientEvent::Closed(closed) => closed,
        other => panic!("expected the connection to close, got {other:?}"),
    }
}

/// #127: a room nobody uses expires; everyone in it is told, and the code
/// then names no room.
#[test]
fn an_idle_room_expires_for_everyone_in_it_and_its_code_dies() {
    let relay = TestRelay::start_limited(Limits { max_idle: Duration::from_millis(300), ..Limits::default() });
    let host = relay.client(Request::Create);
    let (code, ..) = ready(&host);
    let guest = relay.client(Request::Join(code));
    ready(&guest);

    assert!(matches!(closed(&host), Closed::RoomExpired));
    assert!(matches!(closed(&guest), Closed::RoomExpired));
    assert_eq!(relay.service.open_rooms(), 0);
    assert_eq!(refused(&relay.client(Request::Join(code))), Refusal::NotFound);
}

/// #127: sixteen seats to a room; the seventeenth participant is refused and
/// a freed seat can be taken again.
#[test]
fn a_full_room_refuses_the_next_guest_until_a_seat_frees() {
    let relay = TestRelay::start_limited(Limits { joins_per_minute: 100, ..Limits::default() });
    let host = relay.client(Request::Create);
    let (code, ..) = ready(&host);
    let mut guests: Vec<RelayClient> = (0..15)
        .map(|_| {
            let guest = relay.client(Request::Join(code));
            ready(&guest);
            guest
        })
        .collect();
    assert_eq!(refused(&relay.client(Request::Join(code))), Refusal::Full);

    drop(guests.pop());
    let deadline = Instant::now() + Duration::from_secs(6);
    while relay.service.connections() > 15 {
        assert!(Instant::now() < deadline, "the relay should notice the guest leaving");
        std::thread::sleep(Duration::from_millis(2));
    }
    let late = relay.client(Request::Join(code));
    assert_eq!(ready(&late).1, 15, "the freed seat is reused");
    quiet(&host);
}

/// #127: one address cannot guess codes, or open rooms, without limit.
#[test]
fn an_address_is_throttled_after_its_minute_of_joins_and_creates() {
    let relay = TestRelay::start_limited(Limits { joins_per_minute: 2, creates_per_minute: 1, ..Limits::default() });
    let host = relay.client(Request::Create);
    let (code, ..) = ready(&host);
    assert_eq!(refused(&relay.client(Request::Create)), Refusal::Throttled);

    let wrong = RoomCode::parse("AAA-AAA").unwrap();
    for _ in 0..2 {
        assert_eq!(refused(&relay.client(Request::Join(wrong))), Refusal::NotFound);
    }
    assert_eq!(refused(&relay.client(Request::Join(code))), Refusal::Throttled, "even the right code waits");
}

/// #127: a relay holding its limit of rooms says so.
#[test]
fn a_relay_at_its_room_limit_refuses_a_new_room() {
    let relay = TestRelay::start_limited(Limits { max_rooms: 1, ..Limits::default() });
    let host = relay.client(Request::Create);
    ready(&host);
    assert_eq!(refused(&relay.client(Request::Create)), Refusal::Capacity);
}

/// #127: stopping the relay tells every participant it shut down.
#[test]
fn a_relay_that_stops_tells_everyone_it_shut_down() {
    let relay = TestRelay::start();
    let host = relay.client(Request::Create);
    let (code, ..) = ready(&host);
    let guest = relay.client(Request::Join(code));
    ready(&guest);

    drop(relay);
    assert!(matches!(closed(&host), Closed::Shutdown));
    assert!(matches!(closed(&guest), Closed::Shutdown));
}
