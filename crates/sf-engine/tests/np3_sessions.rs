use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::front::TypedKey;
use sf_engine::netplay::bus::{VirtualNetBus, VirtualNetConfig};
use sf_engine::front::settings::Settings;
use sf_engine::netplay::{Link, NetPacket};
use sf_engine::netplay::{NetTransportCommand, NetTransportEvent};
use sf_engine::roster::MenuInput;

fn session() -> GameSession {
    session_with_config(MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul))
}

fn session_with_config(config: MatchConfig) -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
    ).expect("netplay session tests require the verified local ROMs");
    GameSession::new(assets, config)
}

#[test]
fn two_sessions_measure_the_link_adopt_host_rules_and_reach_fighter_select() {
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.toggles.recovery_buffer_window = 7;
    config.difficulty = 4;
    config.round_time = 250;
    let mut peers = [session_with_config(config), session()];
    let mut bus = VirtualNetBus::new(VirtualNetConfig { latency_ms: 34, ..VirtualNetConfig::default() });
    online(&mut peers[0]);
    peers[0].drain_net_commands();
    pulse(&mut peers[0], MenuInput::Right);
    for _ in 0..4 { pulse(&mut peers[0], MenuInput::Down); }
    pulse(&mut peers[0], MenuInput::Confirm);
    bus.submit(0, &peers[0].drain_net_commands());
    online(&mut peers[1]);
    peers[1].drain_net_commands();
    pulse(&mut peers[1], MenuInput::Left);
    pulse(&mut peers[1], MenuInput::Down);
    pulse(&mut peers[1], MenuInput::Confirm);
    peers[1].type_keys("SF1-7F00-0001".chars().map(TypedKey::Char).chain([TypedKey::Enter]));
    peers[1].step(FrameInputs::default());
    bus.submit(1, &peers[1].drain_net_commands());
    let mut assigned_delay = None;
    for _ in 0..180 {
        for (id, peer) in peers.iter_mut().enumerate() {
            peer.set_net_time_ms(bus.now_ms() as u32);
            peer.push_net_events(&bus.drain_events(id as u8));
            peer.step(FrameInputs::default());
            let commands = peer.drain_net_commands();
            if id == 0 {
                for command in &commands {
                    if let NetTransportCommand::SendPacket { bytes, .. } = command {
                        if let Some(sf_engine::netplay::NetPacket::Welcome { accepted: true, assigned_delay: delay, .. })
                            = sf_engine::netplay::NetPacket::from_bytes(bytes) {
                            assigned_delay = Some(delay);
                        }
                    }
                }
            }
            bus.submit(id as u8, &commands);
        }
        bus.step_ms(17);
        if peers.iter().all(|peer| peer.screen() == "select") { break; }
    }
    assert!(peers.iter().all(|peer| peer.screen() == "select"),
        "both ends must finish the real queue handshake: {:?}", peers.each_ref().map(|peer| peer.screen()));
    for peer in &peers {
        assert_eq!(peer.match_config().toggles.recovery_buffer_window, 7);
        assert_eq!(peer.match_config().difficulty, 4);
        assert_eq!(peer.match_config().round_time, 250);
    }
    assert_eq!(assigned_delay, Some(2), "68 measured milliseconds chooses two frames of AUTO delay");
}

#[test]
fn a_lost_handshake_reply_does_not_turn_retry_wait_into_input_delay() {
    let mut peers = [session(), session()];
    let mut bus = VirtualNetBus::new(VirtualNetConfig { latency_ms: 17, ..VirtualNetConfig::default() });
    online(&mut peers[0]);
    peers[0].drain_net_commands();
    pulse(&mut peers[0], MenuInput::Right);
    for _ in 0..4 { pulse(&mut peers[0], MenuInput::Down); }
    pulse(&mut peers[0], MenuInput::Confirm);
    bus.submit(0, &peers[0].drain_net_commands());
    online(&mut peers[1]);
    peers[1].drain_net_commands();
    pulse(&mut peers[1], MenuInput::Left);
    pulse(&mut peers[1], MenuInput::Down);
    pulse(&mut peers[1], MenuInput::Confirm);
    peers[1].type_keys("SF1-7F00-0001".chars().map(TypedKey::Char).chain([TypedKey::Enter]));
    peers[1].step(FrameInputs::default());
    bus.submit(1, &peers[1].drain_net_commands());

    let mut first_probe = None;
    let mut old_reply = None;
    let mut reply_was_held = false;
    let mut retry_seen = false;
    let mut stale_reply_sent = false;
    let mut fresh_reply_sent = false;
    let mut assigned_delay = None;
    for _ in 0..180 {
        for (id, peer) in peers.iter_mut().enumerate() {
            peer.set_net_time_ms(bus.now_ms() as u32);
            peer.push_net_events(&bus.drain_events(id as u8));
            peer.step(FrameInputs::default());
            let mut commands = Vec::new();
            for command in peer.drain_net_commands() {
                if let NetTransportCommand::SendPacket { bytes, .. } = &command {
                    match NetPacket::from_bytes(bytes) {
                        Some(NetPacket::Ping { seq, sent_ms, .. }) if id == 0 => {
                            if first_probe.is_none() { first_probe = Some((seq, sent_ms)); }
                            else { retry_seen = true; }
                        }
                        Some(NetPacket::Pong { seq, echo_ms, .. }) if id == 1 => {
                            if first_probe == Some((seq, echo_ms)) && !reply_was_held {
                                old_reply = Some(command);
                                reply_was_held = true;
                                continue;
                            }
                            if retry_seen && first_probe != Some((seq, echo_ms)) { fresh_reply_sent = true; }
                        }
                        Some(NetPacket::Welcome { accepted: true, assigned_delay: delay, .. }) if id == 0 => {
                            assert_eq!(delay, 1, "34 ms RTT chooses 1F; the retry's 500 ms wait is not network latency");
                            assert!(fresh_reply_sent, "the delayed reply to the old probe must not admit the challenger");
                            assigned_delay = Some(delay);
                        }
                        _ => {}
                    }
                }
                commands.push(command);
            }
            bus.submit(id as u8, &commands);
            if retry_seen && !stale_reply_sent {
                if let Some(reply) = old_reply.take() {
                    // Deliver the old reply before the fresh probe's response.
                    bus.submit(1, &[reply]);
                    stale_reply_sent = true;
                }
            }
        }
        bus.step_ms(17);
        if assigned_delay.is_some() && peers.iter().all(|peer| peer.screen() == "select") { break; }
    }
    assert!(reply_was_held && retry_seen && stale_reply_sent);
    assert_eq!(assigned_delay, Some(1));
    assert!(peers.iter().all(|peer| peer.screen() == "select"));
}

fn pulse(session: &mut GameSession, key: MenuInput) {
    let mut input = FrameInputs::default();
    match key {
        MenuInput::Up => input.p1.directions[3] = true,
        MenuInput::Down => input.p1.directions[2] = true,
        MenuInput::Left => input.p1.directions[1] = true,
        MenuInput::Right => input.p1.directions[0] = true,
        MenuInput::Confirm => input.menu.confirm = true,
        MenuInput::Back => input.menu.back = true,
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

#[test]
fn the_online_menu_drives_socket_free_transport_queues_and_closes_them_on_back() {
    let mut session = session();
    online(&mut session);
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::RefreshRooms]);
    assert!(session.drain_net_commands().is_empty(), "commands drain exactly once");
    session.push_net_events(&[NetTransportEvent::TransportError { message: "TEST NETWORK ERROR".into() }]);
    session.step(FrameInputs::default());
    assert!(session.notice().is_some_and(|notice| notice.contains("TEST NETWORK ERROR")));
    pulse(&mut session, MenuInput::Back);
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::Close]);
    assert_ne!(session.screen(), "online");
}

#[test]
fn opening_a_room_advertises_real_rule_and_asset_compatibility() {
    fn announce(mut session: GameSession) -> sf_engine::netplay::RoomAnnouncement {
        online(&mut session);
        session.drain_net_commands();
        pulse(&mut session, MenuInput::Right);
        for _ in 0..4 { pulse(&mut session, MenuInput::Down); }
        pulse(&mut session, MenuInput::Confirm);
        let commands = session.drain_net_commands();
        let [NetTransportCommand::BindHost { port, room }] = commands.as_slice() else {
            panic!("OPEN ROOM must bind a host, got {commands:?}");
        };
        assert_eq!(*port, 7000);
        assert_eq!(room.preset, RulesetPreset::DefinitiveOverhaul);
        assert_eq!(room.protocol_hash, sf_engine::version::protocol_version_hash());
        assert!(room.public && !room.wired_only && !room.in_match);
        assert_eq!(room.max_rollback, 8);
        assert_eq!(room.spectators, 0);
        room.clone()
    }
    let original = announce(session());
    let changed = |graphics| {
        let mut assets = sf_video::romset::load_assets(
            "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
        ).unwrap();
        if graphics { assets.gfx3[0] ^= 1; } else { assets.maincpu[0] ^= 1; }
        GameSession::new(assets, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul))
    };
    let game_data = announce(changed(false));
    assert_ne!(original.bank_hash, game_data.bank_hash, "different machine data cannot share a room");
    let artwork = announce(changed(true));
    assert_eq!(original.bank_hash, artwork.bank_hash, "art does not masquerade as machine data");
    assert_ne!(original.art_pack_hash, artwork.art_pack_hash, "different art is detected before play");
}

#[test]
fn actual_session_handshakes_reject_wifi_changed_art_and_wrong_engine_identity() {
    for (wifi, changed_art, wrong_version, expected) in [
        (true, false, false, "WIRED ONLY"),
        (false, true, false, "PACK MISMATCH"),
        (false, false, true, "VERSION MISMATCH"),
    ] {
        let host = session().with_settings(Settings { wired_only: true, ..Settings::default() });
        let client = if changed_art {
            let mut assets = sf_video::romset::load_assets(
                "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
            ).unwrap();
            assets.gfx3[0] ^= 1;
            GameSession::new(assets, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul))
        } else { session() };
        let mut peers = [host, client];
        let mut bus = VirtualNetBus::new(VirtualNetConfig { latency_ms: 15, ..VirtualNetConfig::default() });
        bus.set_link(1, if wifi { Link::Wifi } else { Link::Ethernet });
        online(&mut peers[0]);
        peers[0].drain_net_commands();
        pulse(&mut peers[0], MenuInput::Right);
        for _ in 0..4 { pulse(&mut peers[0], MenuInput::Down); }
        pulse(&mut peers[0], MenuInput::Confirm);
        bus.submit(0, &peers[0].drain_net_commands());
        online(&mut peers[1]);
        peers[1].drain_net_commands();
        pulse(&mut peers[1], MenuInput::Left);
        pulse(&mut peers[1], MenuInput::Down);
        pulse(&mut peers[1], MenuInput::Confirm);
        peers[1].type_keys("SF1-7F00-0001".chars().map(TypedKey::Char).chain([TypedKey::Enter]));
        peers[1].step(FrameInputs::default());
        bus.submit(1, &peers[1].drain_net_commands());
        for _ in 0..120 {
            for (id, peer) in peers.iter_mut().enumerate() {
                peer.set_net_time_ms(bus.now_ms() as u32);
                peer.push_net_events(&bus.drain_events(id as u8));
                peer.step(FrameInputs::default());
                let mut commands = peer.drain_net_commands();
                if id == 1 && wrong_version {
                    for command in &mut commands {
                        if let NetTransportCommand::SendPacket { bytes, .. } = command {
                            if let Some(mut packet @ NetPacket::Hello { .. }) = NetPacket::from_bytes(bytes) {
                                if let NetPacket::Hello { protocol_hash, .. } = &mut packet { *protocol_hash ^= 1; }
                                *bytes = packet.to_bytes();
                            }
                        }
                    }
                }
                bus.submit(id as u8, &commands);
            }
            bus.step_ms(17);
            if peers[1].notice().is_some_and(|notice| notice.contains(expected)) { break; }
        }
        assert_eq!(peers[0].screen(), "online");
        assert_eq!(peers[1].screen(), "online");
        assert!(peers[1].notice().is_some_and(|notice| notice.contains(expected)),
            "rejected clients need a useful reason: {:?}", peers[1].notice());
    }
}
