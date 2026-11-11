use std::time::{Duration, Instant};

use sf_engine::config::MatchConfig;
use sf_engine::front::{FrameInputs, GameSession, TypedKey};
use sf_engine::netplay::{
    Link, NetPacket, NetTransportCommand as Command, NetTransportEvent as Event,
};
use sf_engine::roster::MenuInput;
use sf_host::net::HostNetDriver;

fn session(config: MatchConfig) -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("real UDP session tests require the verified local ROMs");
    let mut bank = sf_video::background_bank::BackgroundBank::new();
    bank.add_rom_stages(&assets)
        .expect("verified ROM stages must load");
    bank.add_vs_screens(&assets)
        .expect("verified VS screens must load");
    GameSession::new(assets, config).with_background_bank(bank)
}

fn pulse(session: &mut GameSession, key: MenuInput) {
    let mut input = FrameInputs::default();
    match key {
        MenuInput::Down => input.p1.directions[2] = true,
        MenuInput::Left => input.p1.directions[1] = true,
        MenuInput::Right => input.p1.directions[0] = true,
        MenuInput::Confirm => input.menu.confirm = true,
        MenuInput::Start => input.menu.start = true,
        _ => panic!("unsupported test key"),
    }
    session.step(input);
    session.step(FrameInputs::default());
}

fn online(session: &mut GameSession) {
    session.step(FrameInputs::default());
    pulse(session, MenuInput::Start);
    pulse(session, MenuInput::Down);
    pulse(session, MenuInput::Confirm);
    pulse(session, MenuInput::Down);
    pulse(session, MenuInput::Confirm);
    assert_eq!(session.screen(), "online");
}

// The production menu advertises the configured port. This test binds an
// ephemeral port, then preserves it for subsequent same-room metadata updates.
fn transport(session: &mut GameSession, driver: &mut HostNetDriver) -> Vec<NetPacket> {
    let mut commands = session.drain_net_commands();
    let mut packets = Vec::new();
    for command in &mut commands {
        match command {
            Command::BindHost { port, .. } => {
                *port = driver.local_addr().map_or(0, |addr| addr.port())
            }
            Command::SendPacket { bytes, .. } => {
                packets.push(NetPacket::from_bytes(bytes).expect("engine emitted malformed packet"))
            }
            _ => {}
        }
    }
    let events = driver.poll(&commands);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::TransportError { .. })),
        "{events:?}"
    );
    session.push_net_events(&events);
    packets
}

#[test]
fn two_real_game_sessions_adopt_host_rules_and_run_a_matching_fight_over_udp() {
    let mut host_config = MatchConfig::definitive();
    host_config.difficulty = 4;
    host_config.round_time = 250;
    host_config.toggles.recovery_buffer_window = 7;
    let mut peers = [session(host_config), session(MatchConfig::definitive())];
    let mut drivers = [
        HostNetDriver::with_link_override(Link::Ethernet),
        HostNetDriver::with_link_override(Link::Ethernet),
    ];
    online(&mut peers[0]);
    peers[0].drain_net_commands();
    pulse(&mut peers[0], MenuInput::Right);
    for _ in 0..4 {
        pulse(&mut peers[0], MenuInput::Down);
    }
    pulse(&mut peers[0], MenuInput::Confirm);
    transport(&mut peers[0], &mut drivers[0]);
    let address = drivers[0].local_addr().unwrap().to_string();
    online(&mut peers[1]);
    peers[1].drain_net_commands();
    pulse(&mut peers[1], MenuInput::Left);
    pulse(&mut peers[1], MenuInput::Down);
    pulse(&mut peers[1], MenuInput::Confirm);
    peers[1].type_keys(address.chars().map(TypedKey::Char).chain([TypedKey::Enter]));
    peers[1].step(FrameInputs::default());
    transport(&mut peers[1], &mut drivers[1]);
    let until = Instant::now() + Duration::from_secs(3);
    let mut hello = false;
    let mut welcome = false;
    while Instant::now() < until && peers.iter().any(|peer| peer.screen() != "select") {
        for (peer, driver) in peers.iter_mut().zip(&mut drivers) {
            peer.set_net_time_ms(driver.now_ms());
            peer.step(FrameInputs::default());
            for packet in transport(peer, driver) {
                hello |= matches!(packet, NetPacket::Hello { .. });
                welcome |= matches!(packet, NetPacket::Welcome { accepted: true, .. });
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        hello && welcome,
        "the real engine queues must exchange Hello and accepted Welcome"
    );
    assert!(
        peers.iter().all(|peer| peer.screen() == "select"),
        "{:?}",
        peers.each_ref().map(|peer| (peer.screen(), peer.notice()))
    );
    for peer in &peers {
        assert_eq!(peer.match_config().difficulty, 4);
        assert_eq!(peer.match_config().round_time, 250);
        assert_eq!(peer.match_config().toggles.recovery_buffer_window, 7);
    }

    // Each player's physical P1 controller locks its assigned online side.
    pulse(&mut peers[1], MenuInput::Confirm);
    transport(&mut peers[1], &mut drivers[1]);
    for _ in 0..5 {
        step_pair(&mut peers, &mut drivers, [FrameInputs::default(); 2]);
    }
    pulse(&mut peers[0], MenuInput::Confirm);
    transport(&mut peers[0], &mut drivers[0]);
    let until = Instant::now() + Duration::from_secs(2);
    while peers.iter().any(|peer| peer.screen() != "stage select") {
        assert!(
            Instant::now() < until,
            "both remote fighter locks must reach stage select: {:?}",
            peers.each_ref().map(|peer| (peer.screen(), peer.notice()))
        );
        step_pair(&mut peers, &mut drivers, [FrameInputs::default(); 2]);
    }
    pulse(&mut peers[0], MenuInput::Confirm);
    transport(&mut peers[0], &mut drivers[0]);
    let until = Instant::now() + Duration::from_secs(2);
    while peers.iter().any(|peer| peer.fight().is_none()) {
        assert!(
            Instant::now() < until,
            "the shared stage lock must start both real Fights"
        );
        step_pair(&mut peers, &mut drivers, [FrameInputs::default(); 2]);
    }
    for _ in 0..3_000 {
        if peers
            .iter()
            .all(|peer| peer.fight().is_some_and(|fight| !fight.driving()))
        {
            break;
        }
        step_pair(&mut peers, &mut drivers, [FrameInputs::default(); 2]);
    }
    assert!(
        peers
            .iter()
            .all(|peer| peer.fight().is_some_and(|fight| !fight.driving())),
        "both real machines must finish their native intro before testing combat inputs"
    );
    let initial = peers
        .each_ref()
        .map(|peer| peer.fight_machine().unwrap().state.frame);
    let mut claims: [std::collections::BTreeMap<u32, u64>; 2] =
        std::array::from_fn(|_| Default::default());
    for frame in 0..120 {
        let inputs = std::array::from_fn(|side| {
            let mut input = FrameInputs::default();
            input.p1.directions[side] = frame < 90 && frame % 20 < 10;
            input.p1.buttons[0] = frame < 90 && frame % (17 + side) == 0;
            input.p2.buttons[5] = true;
            input
        });
        for (side, packet) in step_pair(&mut peers, &mut drivers, inputs) {
            if let NetPacket::Input {
                checksum_frame,
                checksum,
                ..
            } = packet
            {
                if checksum_frame != u32::MAX {
                    if let Some(old) = claims[side].insert(checksum_frame, checksum) {
                        assert_eq!(
                            old, checksum,
                            "a confirmed checksum cannot change on one peer"
                        );
                    }
                }
            }
        }
    }
    for side in 0..2 {
        assert!(peers[side].fight_machine().unwrap().state.frame > initial[side] + 20);
        assert!(!peers[side]
            .notice()
            .is_some_and(|notice| notice.contains("DESYNC")));
    }
    let mut compared = 0;
    for (frame, host) in &claims[0] {
        if let Some(client) = claims[1].get(frame) {
            assert_eq!(
                host, client,
                "the real UDP fight diverged at confirmed frame {frame}"
            );
            compared += 1;
        }
    }
    assert!(
        compared >= 30,
        "compare confirmed wire frames on both peers, not speculative tips: {compared}"
    );
}

fn step_pair(
    peers: &mut [GameSession; 2],
    drivers: &mut [HostNetDriver; 2],
    inputs: [FrameInputs; 2],
) -> Vec<(usize, NetPacket)> {
    let mut packets = Vec::new();
    for side in 0..2 {
        peers[side].set_net_time_ms(drivers[side].now_ms());
        peers[side].step(inputs[side]);
        packets.extend(
            transport(&mut peers[side], &mut drivers[side])
                .into_iter()
                .map(|packet| (side, packet)),
        );
    }
    std::thread::sleep(Duration::from_millis(1));
    packets
}
