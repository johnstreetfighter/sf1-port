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
        let signed = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
        let certificate = signed.cert.der().to_vec();
        let service = RelayService::start(ServiceConfig {
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            certificates: vec![certificate.clone()],
            private_key: signed.signing_key.serialize_der(),
            limits: Limits::default(),
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
