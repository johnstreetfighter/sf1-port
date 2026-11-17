use std::time::{Duration, Instant};

use sf_engine::config::MatchConfig;
use sf_engine::era::{Costume, Loadout, Revision, StylePick};
use sf_engine::front::{FrameInputs, GameSession, TypedKey};
use sf_engine::netplay::{
    Link, NetPacket, NetTransportCommand as Command, NetTransportEvent as Event,
};
use sf_engine::roster::MenuInput;
use sf_engine::front::settings::Settings;
use sf_engine::netplay::AfterMatchChoice;
use sf_host::net::HostNetDriver;

#[path = "support/relay.rs"]
mod relay;

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
            .any(|event| matches!(event, Event::TransportError { .. } | Event::TransportClosed { .. })),
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
    // VISIBILITY one step right of INTERNET CODE is PUBLIC, a local room.
    for key in [MenuInput::Down, MenuInput::Down, MenuInput::Right, MenuInput::Down, MenuInput::Down] {
        pulse(&mut peers[0], key);
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

    // Each player's physical P1 controller opens and locks its assigned Style card.
    pulse(&mut peers[1], MenuInput::Confirm);
    pulse(&mut peers[1], MenuInput::Confirm);
    transport(&mut peers[1], &mut drivers[1]);
    for _ in 0..5 {
        step_pair(&mut peers, &mut drivers, [FrameInputs::default(); 2]);
    }
    pulse(&mut peers[0], MenuInput::Confirm);
    pulse(&mut peers[0], MenuInput::Right); // SF2 ST New
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
    let st = StylePick::sf2(Revision::St, Loadout::Standard, Costume::Classic1987, 0);
    for peer in &peers {
        let fight = peer.fight().unwrap();
        assert_eq!(fight.fighter_style(0), Some(st));
        assert_eq!(fight.fighter_style(1), Some(StylePick::default()));
    }
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

/// Step both peers until every one shows `screen`, failing after `limit` steps.
fn wait_for(
    peers: &mut [GameSession; 2],
    drivers: &mut [HostNetDriver; 2],
    screen: &str,
    limit: usize,
    seen: &mut Vec<(usize, NetPacket)>,
) {
    for _ in 0..limit {
        if peers.iter().all(|peer| peer.screen() == screen) {
            return;
        }
        seen.extend(step_pair(peers, drivers, [FrameInputs::default(); 2]));
    }
    panic!(
        "both peers should reach {screen}: {:?}",
        peers.each_ref().map(|peer| (peer.screen(), peer.notice()))
    );
}

/// Player 1's fighter walks in and attacks until both peers see the result.
fn fight_to_the_end(
    peers: &mut [GameSession; 2],
    drivers: &mut [HostNetDriver; 2],
    seen: &mut Vec<(usize, NetPacket)>,
) {
    for _ in 0..3_000 {
        if peers.iter().all(|peer| peer.fight().is_some_and(|fight| !fight.driving())) {
            break;
        }
        seen.extend(step_pair(peers, drivers, [FrameInputs::default(); 2]));
    }
    for frame in 0..20_000_u32 {
        if peers.iter().all(|peer| peer.screen() == "after match") {
            return;
        }
        let phase = frame % 120;
        let mut attack = FrameInputs::default();
        attack.p1.directions[0] = phase < 40;
        if phase >= 40 && phase % 10 < 3 {
            attack.p1.buttons[((frame / 10) % 6) as usize] = true;
        }
        seen.extend(step_pair(peers, drivers, [attack, FrameInputs::default()]));
    }
    panic!(
        "the relayed match should finish: {:?}",
        peers.each_ref().map(|peer| (peer.screen(), peer.notice()))
    );
}

#[test]
fn two_real_game_sessions_fight_rematch_and_reselect_through_an_internet_room() {
    let relay = relay::TestRelay::start();
    let settings = Settings { relay: Some(relay.address()), ..Settings::default() };
    let mut peers = [
        session(MatchConfig::definitive()).with_settings(settings.clone()),
        session(MatchConfig::definitive()).with_settings(settings),
    ];
    let mut drivers = [relay.driver(), relay.driver()];
    let mut seen = Vec::new();

    // Host Match opens an Internet room by default once a relay is set.
    online(&mut peers[0]);
    peers[0].drain_net_commands();
    pulse(&mut peers[0], MenuInput::Right);
    for _ in 0..4 {
        pulse(&mut peers[0], MenuInput::Down);
    }
    pulse(&mut peers[0], MenuInput::Confirm);
    let deadline = Instant::now() + Duration::from_secs(6);
    let code = loop {
        transport(&mut peers[0], &mut drivers[0]);
        peers[0].step(FrameInputs::default());
        if let Some(code) = peers[0].room_code_for_copy() {
            break code.to_owned();
        }
        assert!(Instant::now() < deadline, "the host should show its Internet room code");
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(sf_engine::netplay::RoomCode::parse(&code).is_some(), "{code}");

    // The guest pastes it in lower case and joins.
    online(&mut peers[1]);
    peers[1].drain_net_commands();
    pulse(&mut peers[1], MenuInput::Left);
    pulse(&mut peers[1], MenuInput::Down);
    peers[1].paste_room_code(Some(&code.to_ascii_lowercase()));
    pulse(&mut peers[1], MenuInput::Confirm);
    transport(&mut peers[1], &mut drivers[1]);
    wait_for(&mut peers, &mut drivers, "select", 3_000, &mut seen);
    assert!(
        seen.iter().any(|(side, packet)| *side == 1 && matches!(packet, NetPacket::Hello { .. })),
        "the guest's Hello crossed the relay"
    );
    assert!(
        seen.iter().any(|(side, packet)| *side == 0 && matches!(packet, NetPacket::Welcome { accepted: true, .. })),
        "the host's compatibility check accepted the guest through the relay"
    );

    // Fighters, stage and the first fight.
    let lock_fighters = |peers: &mut [GameSession; 2], drivers: &mut [HostNetDriver; 2]| {
        pulse(&mut peers[1], MenuInput::Confirm);
        pulse(&mut peers[1], MenuInput::Confirm);
        transport(&mut peers[1], &mut drivers[1]);
        pulse(&mut peers[0], MenuInput::Confirm);
        pulse(&mut peers[0], MenuInput::Confirm);
        transport(&mut peers[0], &mut drivers[0]);
    };
    lock_fighters(&mut peers, &mut drivers);
    wait_for(&mut peers, &mut drivers, "stage select", 3_000, &mut seen);
    pulse(&mut peers[0], MenuInput::Confirm);
    transport(&mut peers[0], &mut drivers[0]);
    fight_to_the_end(&mut peers, &mut drivers, &mut seen);

    // Both vote Rematch; the room and its code carry on.
    pulse(&mut peers[1], MenuInput::Confirm);
    pulse(&mut peers[0], MenuInput::Confirm);
    for side in 0..2 {
        seen.extend(transport(&mut peers[side], &mut drivers[side]).into_iter().map(|packet| (side, packet)));
    }
    wait_for(&mut peers, &mut drivers, "stage select", 3_000, &mut seen);
    assert!(seen.iter().any(|(_, packet)| matches!(packet,
        NetPacket::AfterMatchVote { choice: AfterMatchChoice::Rematch, .. })));
    assert!(relay.can_join(&code), "the code still opens the room after a rematch");
    // The rematch's stage pick belongs to the side the host's Welcome names.
    let picker = seen
        .iter()
        .rev()
        .find_map(|(_, packet)| match packet {
            NetPacket::Welcome { accepted: true, picker, .. } => Some(usize::from(*picker)),
            _ => None,
        })
        .expect("the rematch epoch arrives in a Welcome");
    pulse(&mut peers[picker], MenuInput::Confirm);
    transport(&mut peers[picker], &mut drivers[picker]);
    fight_to_the_end(&mut peers, &mut drivers, &mut seen);

    // Reselect returns both to fighter select in the same room.
    for peer in 0..2 {
        pulse(&mut peers[peer], MenuInput::Down);
        pulse(&mut peers[peer], MenuInput::Confirm);
        transport(&mut peers[peer], &mut drivers[peer]);
    }
    wait_for(&mut peers, &mut drivers, "select", 3_000, &mut seen);
    assert!(relay.can_join(&code), "the code still opens the room after a reselect");

    // Peer timing ran end to end over the relay the whole time.
    for side in 0..2 {
        assert!(seen.iter().any(|(from, packet)| *from == side && matches!(packet, NetPacket::Ping { .. })));
        assert!(seen.iter().any(|(from, packet)| *from == side && matches!(packet, NetPacket::Pong { .. })));
    }
    assert!(peers.iter().all(|peer| !peer.notice().is_some_and(|notice| notice.contains("DESYNC"))));
    assert_eq!(relay.service.open_rooms(), 1, "one room, one code, for the whole session");

    // Closing the host ends the room: the guest hears it and the code is dead.
    drivers[0].poll(&[Command::Close]);
    let deadline = Instant::now() + Duration::from_secs(6);
    let closed = 'heard: loop {
        for event in drivers[1].poll(&[]) {
            if let Event::TransportClosed { message } = event {
                break 'heard message;
            }
        }
        assert!(Instant::now() < deadline, "the guest should hear the room close");
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(closed, "ROOM CLOSED");
    assert!(!relay.can_join(&code));
}
