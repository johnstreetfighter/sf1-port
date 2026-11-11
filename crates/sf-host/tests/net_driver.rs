use sf_engine::config::MatchConfig;
use sf_engine::front::fight::FightDips;
use sf_engine::netplay::protocol::{tag_from_str, RejectReason};
use sf_engine::netplay::{
    Link, NetPacket, NetTransportCommand as Command, NetTransportEvent as Event, RoomAnnouncement,
    RoomKind, SyncedMatchRules,
};
use sf_engine::roster::{StagePick, TournamentFighter};
use sf_host::net::{AdapterInfo, AdapterQuery, HostNetDriver, NetDriverConfig};
use std::time::{Duration, Instant};

fn rules() -> SyncedMatchRules {
    SyncedMatchRules::new(
        MatchConfig::definitive(),
        [TournamentFighter::Ryu, TournamentFighter::Ken],
        FightDips::default(),
        StagePick::Bank(0),
        42,
        123,
        456,
    )
}
fn announcement() -> RoomAnnouncement {
    let rules = rules();
    RoomAnnouncement {
        host: tag_from_str("HOST").unwrap(),
        kind: RoomKind::Versus,
        link: Link::Ethernet,
        wired_only: true,
        public: false,
        preset: rules.config.preset,
        in_match: false,
        spectators: 0,
        max_rollback: 8,
        config_hash: rules.config_hash(),
        bank_hash: 123,
        art_pack_hash: 456,
        protocol_hash: rules.protocol_hash,
    }
}
fn receive(driver: &mut HostNetDriver) -> (u8, NetPacket) {
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        for event in driver.poll(&[]) {
            match event {
                Event::PacketReceived {
                    from_peer, bytes, ..
                } => return (from_peer, NetPacket::from_bytes(&bytes).unwrap()),
                Event::TransportError { message } => panic!("{message}"),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("loopback packet did not arrive");
}

#[test]
fn two_real_udp_drivers_exchange_hello_welcome_and_exact_fifty_byte_inputs() {
    let mut host = HostNetDriver::with_link_override(Link::Ethernet);
    let events = host.poll(&[Command::BindHost {
        port: 0,
        room: announcement(),
    }]);
    assert!(matches!(events.as_slice(), [Event::Bound { .. }]));
    let address = host.local_addr().unwrap();
    assert!(address.port() > 0);
    let mut client = HostNetDriver::with_link_override(Link::Ethernet);
    assert!(matches!(
        client
            .poll(&[Command::ConnectPeer {
                addr: address.to_string()
            }])
            .as_slice(),
        [Event::Bound { .. }]
    ));
    let rules = rules();
    let hello = NetPacket::Hello {
        request_id: 1,
        tag: tag_from_str("CLIENT").unwrap(),
        kind: RoomKind::Versus,
        link: Link::Ethernet,
        protocol_hash: rules.protocol_hash,
        config_hash: 0,
        bank_hash: 123,
        art_pack_hash: 456,
    };
    client.poll(&[Command::SendPacket {
        to_peer: 0,
        bytes: hello.to_bytes(),
    }]);
    let (route, received) = receive(&mut host);
    assert_eq!(received, hello);
    let welcome = NetPacket::Welcome {
        request_id: 1,
        generation: 1,
        accepted: true,
        reason: RejectReason::None,
        phase: sf_engine::netplay::protocol::LobbyPhase::Fighters,
        picker: 0,
        config: rules,
        seed: 42,
        assigned_delay: 0,
        max_rollback: 8,
        host_tag: tag_from_str("HOST").unwrap(),
        frame_base: 0,
    };
    host.poll(&[Command::SendPacket {
        to_peer: route,
        bytes: welcome.to_bytes(),
    }]);
    assert_eq!(receive(&mut client), (0, welcome));
    let input = NetPacket::Input {
        current_frame: 0,
        start_frame: 0,
        count: 1,
        bits: [1, 0, 0, 0, 0, 0, 0, 0, 0],
        ack_frame: u32::MAX,
        frame_advantage: 0,
        checksum_frame: u32::MAX,
        checksum: 0,
    };
    assert_eq!(input.to_bytes().len(), 50);
    client.poll(&[Command::SendPacket {
        to_peer: 0,
        bytes: input.to_bytes(),
    }]);
    assert_eq!(receive(&mut host), (route, input));
    let started = Instant::now();
    for _ in 0..100 {
        host.poll(&[]);
    }
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "poll waited for network I/O"
    );
}

#[test]
fn real_lan_probes_discover_public_rooms_and_expire_closed_or_private_advertisements() {
    for kind in [RoomKind::Versus, RoomKind::Dramatic] {
        discover_and_expire_room(kind);
    }
}

fn discover_and_expire_room(kind: RoomKind) {
    let mut public = announcement();
    public.public = true;
    public.kind = kind;
    let local_config = NetDriverConfig {
        bind_ip: std::net::Ipv4Addr::LOCALHOST,
        link_override: Some(Link::Ethernet),
        discovery_port: 0,
        discovery_targets: Vec::new(),
        room_ttl: Duration::from_millis(30),
        ..NetDriverConfig::default()
    };
    let mut host = HostNetDriver::with_config(local_config.clone());
    host.poll(&[Command::BindHost {
        port: 0,
        room: public.clone(),
    }]);
    let discovery = host
        .discovery_addr()
        .expect("a public room listens for LAN probes");
    let mut browser = HostNetDriver::with_config(NetDriverConfig {
        discovery_targets: vec![discovery],
        ..local_config
    });
    browser.poll(&[Command::RefreshRooms]);
    let until = Instant::now() + Duration::from_secs(2);
    let mut found = false;
    while Instant::now() < until && !found {
        host.poll(&[]);
        for event in browser.poll(&[]) {
            if let Event::RoomsDiscovered { rooms } = event {
                if let Some(room) = rooms.first() {
                    assert_eq!(
                        room.code,
                        sf_engine::netplay::encode_room_code(
                            [127, 0, 0, 1],
                            host.local_addr().unwrap().port()
                        )
                    );
                    assert_eq!(room.host, "HOST");
                    assert_eq!(room.kind, kind);
                    found = true;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        found,
        "localhost UDP probe/beacon exchange must discover the public room"
    );
    public.public = false;
    host.poll(&[Command::BindHost {
        port: host.local_addr().unwrap().port(),
        room: public,
    }]);
    std::thread::sleep(Duration::from_millis(50));
    assert!(browser
        .poll(&[])
        .iter()
        .any(|event| matches!(event, Event::RoomsDiscovered { rooms } if rooms.is_empty())));
    browser.poll(&[Command::RefreshRooms]);
    host.poll(&[]);
    assert!(!browser
        .poll(&[])
        .iter()
        .any(|event| matches!(event, Event::RoomsDiscovered { rooms } if !rooms.is_empty())));
}

fn slow_adapter_probe(_: AdapterQuery) -> std::io::Result<AdapterInfo> {
    assert_eq!(std::thread::current().name(), Some("sf-net-adapter"));
    std::thread::sleep(Duration::from_millis(200));
    Ok(AdapterInfo {
        ip: std::net::Ipv4Addr::LOCALHOST,
        link: Link::Wifi,
    })
}

fn isolated_lan_probe(query: AdapterQuery) -> std::io::Result<AdapterInfo> {
    if matches!(query, AdapterQuery::Peer { target, .. } if target == std::net::Ipv4Addr::new(1, 1, 1, 1))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NetworkUnreachable,
            "NO DEFAULT ROUTE",
        ));
    }
    Ok(AdapterInfo {
        ip: std::net::Ipv4Addr::new(192, 0, 2, 20),
        link: Link::Wifi,
    })
}

fn bound_adapter_probe(query: AdapterQuery) -> std::io::Result<AdapterInfo> {
    use std::net::Ipv4Addr;
    let link = match query {
        AdapterQuery::Host { bind_ip } if bind_ip == Ipv4Addr::LOCALHOST => Link::Wifi,
        AdapterQuery::Peer { target, bind_ip }
            if target == Ipv4Addr::LOCALHOST && bind_ip == Ipv4Addr::LOCALHOST =>
        {
            Link::Ethernet
        }
        _ => {
            return Err(std::io::Error::other(
                "WRONG HOST BIND OR CLIENT DESTINATION",
            ))
        }
    };
    Ok(AdapterInfo {
        ip: Ipv4Addr::LOCALHOST,
        link,
    })
}

#[test]
fn explicit_host_bind_and_client_destination_keep_their_own_adapter_context() {
    let config = NetDriverConfig {
        bind_ip: std::net::Ipv4Addr::LOCALHOST,
        discovery_port: 0,
        discovery_targets: Vec::new(),
        adapter_probe: bound_adapter_probe,
        ..NetDriverConfig::default()
    };
    let bound = |driver: &mut HostNetDriver, commands: &[Command]| {
        let mut events = driver.poll(commands);
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            for event in events {
                match event {
                    Event::Bound { link, .. } => return link,
                    Event::TransportError { message } => panic!("{message}"),
                    _ => {}
                }
            }
            assert!(Instant::now() < until, "the requested adapter never bound");
            std::thread::sleep(Duration::from_millis(1));
            events = driver.poll(&[]);
        }
    };
    let mut host = HostNetDriver::with_config(config.clone());
    assert_eq!(
        bound(
            &mut host,
            &[Command::BindHost {
                port: 0,
                room: announcement()
            }]
        ),
        Link::Wifi
    );
    let mut client = HostNetDriver::with_config(config);
    assert_eq!(
        bound(
            &mut client,
            &[Command::ConnectPeer {
                addr: host.local_addr().unwrap().to_string()
            }]
        ),
        Link::Ethernet
    );
}

#[test]
fn an_isolated_lan_can_host_without_a_route_to_an_external_address() {
    let mut driver = HostNetDriver::with_config(NetDriverConfig {
        discovery_port: 0,
        discovery_targets: Vec::new(),
        adapter_probe: isolated_lan_probe,
        ..NetDriverConfig::default()
    });
    let mut events = driver.poll(&[Command::BindHost {
        port: 0,
        room: announcement(),
    }]);
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        for event in events {
            match event {
                Event::Bound {
                    local_addr, link, ..
                } => {
                    assert!(local_addr.starts_with("192.0.2.20:"));
                    assert_eq!(
                        link,
                        Link::Wifi,
                        "the local adapter's actual media is retained"
                    );
                    return;
                }
                Event::TransportError { message } => {
                    panic!("a reachable local LAN must not need an external route: {message}")
                }
                _ => {}
            }
        }
        assert!(
            Instant::now() < until,
            "the isolated LAN never received Bound"
        );
        std::thread::sleep(Duration::from_millis(1));
        events = driver.poll(&[]);
    }
}

#[test]
fn adapter_detection_runs_off_the_frame_thread_and_never_guesses_a_wired_link() {
    let start = Instant::now();
    let mut driver = HostNetDriver::with_config(NetDriverConfig {
        bind_ip: std::net::Ipv4Addr::LOCALHOST,
        discovery_port: 0,
        discovery_targets: Vec::new(),
        adapter_probe: slow_adapter_probe,
        ..NetDriverConfig::default()
    });
    let events = driver.poll(&[Command::BindHost {
        port: 0,
        room: announcement(),
    }]);
    assert!(start.elapsed() < Duration::from_millis(100));
    assert!(
        events.is_empty(),
        "no Bound event may guess Ethernet while the OS query is pending"
    );
    let until = Instant::now() + Duration::from_secs(2);
    let mut bound = false;
    while Instant::now() < until && !bound {
        for event in driver.poll(&[]) {
            match event {
                Event::Bound { link, .. } => {
                    assert_eq!(link, Link::Wifi);
                    bound = true;
                }
                Event::TransportError { message } => panic!("{message}"),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(bound);
}

#[test]
fn a_final_packet_is_dispatched_before_close_clears_the_local_transport() {
    let mut host = HostNetDriver::with_link_override(Link::Ethernet);
    host.poll(&[Command::BindHost {
        port: 0,
        room: announcement(),
    }]);
    let mut client = HostNetDriver::with_link_override(Link::Ethernet);
    client.poll(&[Command::ConnectPeer {
        addr: host.local_addr().unwrap().to_string(),
    }]);
    let final_packet = NetPacket::Ping {
        seq: 99,
        sent_ms: 0,
        link: Link::Ethernet,
    };
    client.poll(&[
        Command::SendPacket {
            to_peer: 0,
            bytes: final_packet.to_bytes(),
        },
        Command::Close,
    ]);
    assert!(client.local_addr().is_none());
    assert_eq!(receive(&mut host).1, final_packet);
    assert!(
        client.poll(&[]).is_empty(),
        "closed transport retains no incoming events"
    );
}

#[test]
fn a_configured_directory_fetch_populates_the_driver_browser_without_waiting_for_http() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            if let Ok((stream, _)) = listener.accept() {
                break stream;
            }
            assert!(
                Instant::now() < until,
                "no configured directory request arrived"
            );
            std::thread::sleep(Duration::from_millis(1));
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
            assert!(header.len() < 8192);
        }
        assert!(header.starts_with(b"GET /rooms HTTP/1.1\r\n"));
        std::thread::sleep(Duration::from_millis(200));
        let body = concat!(
            r#"{"version":1,"rooms":[{"address":"127.0.0.2:7000","beacon":""#,
            "534631440102581b484f53540000000000000000",
            "0000010100000008",
            "08070605040302011817161514131211f8f7f6f5f4f3f2f1a4a3a2a1",
            r#""}]}"#
        );
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body.as_bytes()).unwrap();
    });
    let mut browser = HostNetDriver::with_link_override(Link::Ethernet);
    browser.set_lobby_url(Some(format!("http://{address}/rooms")));
    let start = Instant::now();
    browser.poll(&[Command::RefreshRooms]);
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "HTTP blocked the frame thread"
    );
    let until = Instant::now() + Duration::from_secs(2);
    let mut found = false;
    while Instant::now() < until && !found {
        for event in browser.poll(&[]) {
            match event {
                Event::RoomsDiscovered { rooms } => {
                    found |= rooms
                        .iter()
                        .any(|room| room.code == "SF1-7F00-0002" && room.host == "HOST")
                }
                Event::TransportError { message } => panic!("{message}"),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        found,
        "the configured directory listing never reached RoomsDiscovered"
    );
    server.join().unwrap();
}

#[test]
fn a_shared_game_and_discovery_socket_can_send_an_actual_broadcast() {
    use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
    let reservation = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let game_port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let receiver = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    receiver.set_nonblocking(true).unwrap();
    let mut room = announcement();
    room.public = true;
    let mut host = HostNetDriver::with_config(NetDriverConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        link_override: Some(Link::Ethernet),
        discovery_port: game_port,
        discovery_targets: vec![SocketAddrV4::new(
            Ipv4Addr::BROADCAST,
            receiver.local_addr().unwrap().port(),
        )],
        ..NetDriverConfig::default()
    });
    assert!(host
        .poll(&[Command::BindHost {
            port: game_port,
            room
        }])
        .iter()
        .any(|event| matches!(event, Event::Bound { .. })));
    assert!(
        host.drain_notes().is_empty(),
        "the shared socket must permit broadcasts"
    );
    let until = Instant::now() + Duration::from_secs(1);
    let mut bytes = [0; 512];
    loop {
        if let Ok((count, from)) = receiver.recv_from(&mut bytes) {
            assert_eq!(from.port(), game_port);
            assert_eq!(count, 56);
            assert_eq!(&bytes[..6], b"SF1D\x01\x02");
            break;
        }
        assert!(Instant::now() < until, "actual broadcast did not arrive");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn directory_request(listener: &std::net::TcpListener) -> (std::net::TcpStream, serde_json::Value) {
    use std::io::Read;
    let until = Instant::now() + Duration::from_secs(3);
    let mut stream = loop {
        if let Ok((stream, _)) = listener.accept() {
            break stream;
        }
        assert!(Instant::now() < until, "no public-room directory request");
        std::thread::sleep(Duration::from_millis(1));
    };
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 8192);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header.starts_with("POST /rooms HTTP/1.1\r\n"));
    let length = header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap();
    assert!(length < 8192);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    (stream, serde_json::from_slice(&body).unwrap())
}

#[test]
fn public_driver_rooms_publish_the_actual_link_then_withdraw_on_private_update_or_close() {
    use std::io::Write;
    for close in [false, true] {
        let mut host = HostNetDriver::with_link_override(Link::Wifi);
        let mut room = announcement();
        room.public = true;
        host.poll(&[Command::BindHost {
            port: 0,
            room: room.clone(),
        }]);
        let port = host.local_addr().unwrap().port();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (phases, progress) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            for phase in 0..2 {
                let (mut stream, body) = directory_request(&listener);
                assert_eq!(body["version"], 1);
                assert_eq!(body["port"], port);
                if phase == 0 {
                    let hex = body["beacon"].as_str().unwrap();
                    assert_eq!(hex.len(), 112);
                    let bytes: Vec<_> = (0..56)
                        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
                        .collect();
                    assert_eq!(&bytes[..6], b"SF1D\x01\x02");
                    assert_eq!(
                        bytes[21], 1,
                        "the adapter's Wi-Fi link overrides the claimed Ethernet advertisement"
                    );
                    assert_eq!(bytes[23], 1);
                } else {
                    assert!(body["beacon"].is_null());
                }
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                    )
                    .unwrap();
                phases.send(()).unwrap();
            }
        });
        host.set_lobby_url(Some(format!("http://{address}/rooms")));
        for phase in 0..2 {
            let until = Instant::now() + Duration::from_secs(2);
            while progress.try_recv().is_err() {
                assert!(
                    Instant::now() < until,
                    "directory phase {phase} never arrived"
                );
                assert!(!host
                    .poll(&[])
                    .iter()
                    .any(|event| matches!(event, Event::TransportError { .. })));
                std::thread::sleep(Duration::from_millis(1));
            }
            if phase == 0 {
                room.public = false;
                host.poll(&[if close {
                    Command::Close
                } else {
                    Command::BindHost {
                        port,
                        room: room.clone(),
                    }
                }]);
            }
        }
        if close {
            assert!(host.local_addr().is_none());
        } else {
            assert_eq!(host.local_addr().unwrap().port(), port);
        }
        server.join().unwrap();
    }
}

#[test]
fn discovery_metrics_require_a_correlated_echo_and_ignore_unsolicited_or_stale_replies() {
    use std::net::{Ipv4Addr, UdpSocket};
    let remote = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    remote
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let std::net::SocketAddr::V4(target) = remote.local_addr().unwrap() else {
        unreachable!()
    };
    let mut browser = HostNetDriver::with_config(NetDriverConfig {
        bind_ip: Ipv4Addr::LOCALHOST,
        link_override: Some(Link::Ethernet),
        discovery_port: 0,
        discovery_targets: vec![target],
        beacon_interval: Duration::from_secs(10),
        ..NetDriverConfig::default()
    });
    let hex = concat!(
        "534631440102581b484f53540000000000000000",
        "0000010100000008",
        "08070605040302011817161514131211f8f7f6f5f4f3f2f1a4a3a2a1"
    );
    let beacon: Vec<_> = (0..56)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    let query = |browser: &mut HostNetDriver| {
        browser.poll(&[Command::RefreshRooms]);
        let mut bytes = [0; 512];
        let (count, scanner) = remote.recv_from(&mut bytes).unwrap();
        assert_eq!(count, 14);
        assert_eq!(&bytes[..6], b"SF1D\x01\x03");
        (bytes[..count].to_vec(), scanner)
    };
    let (first, scanner) = query(&mut browser);
    remote.send_to(&beacon, scanner).unwrap();
    let rooms = wait_room_update(&mut browser);
    assert_eq!(
        (rooms[0].ping_ms, rooms[0].jitter_ms),
        (0, 0),
        "an unsolicited beacon has no RTT sample"
    );
    let response = |query: &[u8]| {
        let mut bytes = b"SF1D\x01\x04".to_vec();
        bytes.extend(&query[6..14]);
        bytes.extend(&beacon);
        bytes
    };
    let mut mismatched = response(&first);
    mismatched[6] ^= 1;
    remote.send_to(&mismatched, scanner).unwrap();
    assert!(!browser.poll(&[]).iter().any(|event| matches!(event, Event::RoomsDiscovered { rooms } if rooms.iter().any(|room| room.ping_ms > 0))));
    let mut trailing = response(&first);
    trailing.push(0);
    remote.send_to(&trailing, scanner).unwrap();
    browser.poll(&[]);
    std::thread::sleep(Duration::from_millis(20));
    remote.send_to(&response(&first), scanner).unwrap();
    let rooms = wait_room_update(&mut browser);
    assert!((15..500).contains(&rooms[0].ping_ms));
    let (second, scanner) = query(&mut browser);
    assert_ne!(&first[6..10], &second[6..10]);
    std::thread::sleep(Duration::from_millis(60));
    remote.send_to(&response(&second), scanner).unwrap();
    let rooms = wait_room_update(&mut browser);
    assert!(
        rooms[0].jitter_ms > 0,
        "different measured round trips produce jitter"
    );
    let (old, _) = query(&mut browser);
    browser.poll(&[Command::Close]);
    let (new, scanner) = query(&mut browser);
    assert_ne!(
        &old[6..10],
        &new[6..10],
        "Close must not reuse an old nonce"
    );
    remote.send_to(&response(&old), scanner).unwrap();
    for _ in 0..5 {
        assert!(!browser
            .poll(&[])
            .iter()
            .any(|event| matches!(event, Event::RoomsDiscovered { rooms } if !rooms.is_empty())));
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn wait_room_update(driver: &mut HostNetDriver) -> Vec<sf_engine::netplay::Room> {
    let until = Instant::now() + Duration::from_secs(1);
    loop {
        for event in driver.poll(&[]) {
            if let Event::RoomsDiscovered { rooms } = event {
                if !rooms.is_empty() {
                    return rooms;
                }
            }
        }
        assert!(Instant::now() < until, "no measured room update arrived");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn directory_rooms_are_measured_through_the_game_port_without_allocating_a_player_route() {
    use std::io::{Read, Write};
    let mut host = HostNetDriver::with_link_override(Link::Ethernet);
    let mut room = announcement();
    room.public = true;
    host.poll(&[Command::BindHost {
        port: 0,
        room: room.clone(),
    }]);
    let game = host.local_addr().unwrap();
    let beacon = sf_host::net::encode_room_announcement(&room, game.port()).unwrap();
    let hex: String = beacon.iter().map(|byte| format!("{byte:02x}")).collect();
    let listing =
        serde_json::json!({"version":1,"rooms":[{"address":game.to_string(),"beacon":hex}]})
            .to_string();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            if let Ok((stream, _)) = listener.accept() {
                break stream;
            }
            assert!(Instant::now() < until, "no directory GET");
            std::thread::sleep(Duration::from_millis(1));
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
            assert!(header.len() < 8192);
        }
        assert!(header.starts_with(b"GET /rooms HTTP/1.1\r\n"));
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{listing}",
            listing.len()
        )
        .unwrap();
    });
    let mut browser = HostNetDriver::with_link_override(Link::Ethernet);
    browser.set_lobby_url(Some(format!("http://{address}/rooms")));
    browser.poll(&[Command::RefreshRooms]);
    let until = Instant::now() + Duration::from_secs(2);
    let mut measured = false;
    while Instant::now() < until && !measured {
        let host_events = host.poll(&[]);
        assert!(
            !host_events.iter().any(|event| matches!(
                event,
                Event::PacketReceived { .. } | Event::TransportError { .. }
            )),
            "discovery must not allocate or surface a combatant route: {host_events:?}"
        );
        for event in browser.poll(&[]) {
            if let Event::RoomsDiscovered { rooms } = event {
                measured |= rooms.iter().any(|room| room.ping_ms > 0);
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        measured,
        "a directory-only row must be probed at its game address"
    );
    server.join().unwrap();
}
