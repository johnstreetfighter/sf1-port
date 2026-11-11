//! Spectator behavior through the real session queues, not transport route roles.

use sf_engine::config::MatchConfig;
use sf_engine::dramatic::LifebarMode;
use sf_engine::front::{FrameInputs, GameSession, MenuItem, MenuKeys, Transition, TypedKey};
use sf_engine::front::fight::FightSnapshot;
use sf_engine::netplay::bus::{PacketFault, VirtualNetBus, VirtualNetConfig};
use sf_engine::netplay::{Link, NetPacket, NetTransportCommand, NetTransportEvent, Room, RoomKind};
use sf_engine::roster::{MenuInput, TournamentFighter};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::background_bank::BackgroundBank;

fn session(kind: RoomKind) -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
    ).expect("NP6 requires the verified local ROMs");
    let mut backgrounds = BackgroundBank::new();
    backgrounds.add_rom_stages(&assets).unwrap();
    backgrounds.add_vs_screens(&assets).unwrap();
    let art = (kind == RoomKind::Dramatic).then(|| {
        let pack = ArtPack::open(std::path::Path::new("../../tools/reference/local/roms/sf1-port-art"))
            .expect("NP9 requires the approved art pack");
        load_art_from(&assets, &ArtSources {
            recipes: &[include_str!("../../../art/recipes/sagat.json")],
            reviews: Some(&[include_str!("../../../art/review/sagat.json")]),
            pack: Ok(&pack),
        }).expect("Sagat's approved poses load")
    });
    let session = GameSession::new(assets, MatchConfig::definitive()).with_background_bank(backgrounds);
    if let Some(art) = art {
        session.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues)
    } else {
        session
    }
}

fn key(key: MenuInput) -> FrameInputs {
    let mut input = FrameInputs::default();
    match key {
        MenuInput::Down => input.p1.directions[2] = true,
        MenuInput::Up => input.p1.directions[3] = true,
        MenuInput::Left => input.p1.directions[1] = true,
        MenuInput::Right => input.p1.directions[0] = true,
        MenuInput::Confirm => input.menu.confirm = true,
        MenuInput::Start => input.menu.start = true,
        _ => panic!("unsupported test input"),
    }
    input
}

fn pulse(session: &mut GameSession, key_value: MenuInput) {
    session.step(key(key_value));
    session.step(FrameInputs::default());
}

fn online(session: &mut GameSession) {
    session.step(FrameInputs::default());
    for input in [MenuInput::Start, MenuInput::Down, MenuInput::Confirm, MenuInput::Down, MenuInput::Confirm] {
        pulse(session, input);
    }
    assert_eq!(session.screen(), "online");
    session.drain_net_commands();
}

#[derive(Clone)]
struct RoomTest {
    peers: Vec<GameSession>,
    bus: VirtualNetBus,
    packets: Vec<(usize, u8, NetPacket)>,
    stream_cap: Option<u32>,
    base: Option<u32>,
    host_next: u32,
    host_snapshot_200: Option<(FightSnapshot, u64)>,
    host_confirmed_200: Option<u64>,
    spectator_faults: bool,
    drop_spectator_packets: bool,
    fault_counter: u32,
}

impl RoomTest {
    fn host(count: usize) -> Self {
        Self::host_with_visibility(count, true)
    }

    fn host_with_visibility(count: usize, public: bool) -> Self {
        Self::host_kind(count, public, RoomKind::Versus)
    }

    fn host_kind(count: usize, public: bool, kind: RoomKind) -> Self {
        let template = session(kind);
        let mut room = Self { peers: vec![template; count],
            bus: VirtualNetBus::new(VirtualNetConfig { latency_ms: 15, ..VirtualNetConfig::default() }),
            packets: Vec::new(), stream_cap: None, base: None, host_next: 0,
            host_snapshot_200: None, host_confirmed_200: None,
            spectator_faults: false, drop_spectator_packets: false, fault_counter: 0 };
        online(&mut room.peers[0]);
        pulse(&mut room.peers[0], MenuInput::Right);
        pulse(&mut room.peers[0], MenuInput::Down);
        if kind == RoomKind::Dramatic { pulse(&mut room.peers[0], MenuInput::Right); }
        pulse(&mut room.peers[0], MenuInput::Down);
        if !public { pulse(&mut room.peers[0], MenuInput::Right); }
        for _ in 0..2 { pulse(&mut room.peers[0], MenuInput::Down); }
        pulse(&mut room.peers[0], MenuInput::Confirm);
        room.dispatch(0);
        room
    }

    fn dispatch(&mut self, id: usize) {
        let mut commands = self.peers[id].drain_net_commands();
        if self.drop_spectator_packets {
            commands.retain(|command| !matches!(command,
                NetTransportCommand::SendPacket { to_peer, .. } if id >= 2 || *to_peer >= 2));
        }
        // Model the observer transport losing everything beyond this confirmed
        // boundary. Combatants continue receiving all of their own packets.
        if let Some(cap) = self.stream_cap {
            commands.retain_mut(|command| {
                let NetTransportCommand::SendPacket { bytes, .. } = command else { return true };
                let Some(mut packet) = NetPacket::from_bytes(bytes) else { return true };
                if let NetPacket::SpectatorChunk { start_frame, count, pairs, .. } = &mut packet {
                    if *start_frame >= cap { return false; }
                    *count = u32::from(*count).min(cap - *start_frame) as u8;
                    pairs[usize::from(*count)..].fill((0, 0));
                    *bytes = packet.to_bytes();
                }
                true
            });
        }
        for command in &commands {
            if let NetTransportCommand::SendPacket { to_peer, bytes } = command {
                if let Some(packet) = NetPacket::from_bytes(bytes) {
                    if id == 0 {
                        match &packet {
                            NetPacket::Welcome { accepted: true, frame_base, .. } if *to_peer == 1 => self.base = Some(*frame_base),
                            NetPacket::Input { current_frame, checksum_frame, checksum, .. } => {
                                if let Some(base) = self.base {
                                    self.host_next = current_frame.checked_sub(base).unwrap();
                                    if self.host_next == 200 {
                                        let fight = self.peers[0].fight().unwrap();
                                        self.host_snapshot_200 = Some((fight.save_netplay_snapshot(200), fight.state_checksum()));
                                    }
                                    if checksum_frame.checked_sub(base) == Some(199) { self.host_confirmed_200 = Some(*checksum); }
                                }
                            }
                            _ => {}
                        }
                    }
                    self.packets.push((id, *to_peer, packet));
                }
            }
        }
        if self.spectator_faults {
            let faults = commands.iter().filter_map(|command| {
                let NetTransportCommand::SendPacket { to_peer, .. } = command else { return None };
                if id >= 2 || *to_peer >= 2 {
                    self.fault_counter += 1;
                    Some(PacketFault { delay_ms: if self.fault_counter % 3 == 0 { 90 } else { 5 },
                        drop: self.fault_counter % 7 == 0, copies: if self.fault_counter % 5 == 0 { 2 } else { 1 } })
                } else { Some(PacketFault { delay_ms: 15, drop: false, copies: 1 }) }
            }).collect();
            self.bus.set_fault_script(faults);
        }
        self.bus.submit(id as u8, &commands);
    }

    fn tick(&mut self) {
        self.tick_input(None);
    }

    fn tick_input(&mut self, pressed: Option<(usize, MenuInput)>) {
        self.tick_frame(pressed.map(|(side, input)| (side, key(input))));
    }

    fn tick_frame(&mut self, pressed: Option<(usize, FrameInputs)>) {
        for id in 0..self.peers.len() {
            let peer = &mut self.peers[id];
            peer.set_net_time_ms(self.bus.now_ms() as u32);
            peer.push_net_events(&self.bus.drain_events(id as u8));
            peer.step(pressed.filter(|(side, _)| *side == id).map_or_else(FrameInputs::default, |(_, input)| input));
            self.dispatch(id);
        }
        self.bus.step_ms(17);
    }

    fn press(&mut self, id: usize, input: MenuInput) {
        self.tick_input(Some((id, input)));
        self.tick();
    }

    fn spectate(&mut self, id: usize) {
        let peer = &mut self.peers[id];
        online(peer);
        peer.push_net_events(&[NetTransportEvent::RoomsDiscovered { rooms: vec![Room {
            code: "SF1-7F00-0001".into(), host: "HOST".into(), region: String::new(),
            ping_ms: 0, rollback_frames: 8, jitter_ms: 0, link: Link::Ethernet,
            kind: RoomKind::Versus, spectators: 0, what_if: false, bonus: false,
            preset: MatchConfig::definitive().preset, in_match: false,
        }] }]);
        peer.step(FrameInputs::default());
        pulse(peer, MenuInput::Down);
        pulse(peer, MenuInput::Down);
        pulse(peer, MenuInput::Down);
        pulse(peer, MenuInput::Right); // JOIN AS: SPECTATOR before the match
        pulse(peer, MenuInput::Down);
        pulse(peer, MenuInput::Confirm);
        self.dispatch(id);
    }

    fn spectate_code(&mut self, id: usize) {
        let peer = &mut self.peers[id];
        online(peer);
        pulse(peer, MenuInput::Left);
        pulse(peer, MenuInput::Down);
        pulse(peer, MenuInput::Down);
        pulse(peer, MenuInput::Right); // direct-code JOIN AS: SPECTATOR
        pulse(peer, MenuInput::Up);
        pulse(peer, MenuInput::Confirm);
        peer.type_keys("SF1-7F00-0001".chars().map(TypedKey::Char).chain([TypedKey::Enter]));
        peer.step(FrameInputs::default());
        self.dispatch(id);
    }

    fn challenge(&mut self, id: usize) {
        self.challenge_kind(id, RoomKind::Versus);
    }

    fn challenge_kind(&mut self, id: usize, kind: RoomKind) {
        let peer = &mut self.peers[id];
        online(peer);
        pulse(peer, MenuInput::Left);
        if kind == RoomKind::Dramatic {
            pulse(peer, MenuInput::Down);
            pulse(peer, MenuInput::Down);
            pulse(peer, MenuInput::Left);
            pulse(peer, MenuInput::Up);
        } else {
            pulse(peer, MenuInput::Down);
        }
        pulse(peer, MenuInput::Confirm);
        peer.type_keys("SF1-7F00-0001".chars().map(TypedKey::Char).chain([TypedKey::Enter]));
        peer.step(FrameInputs::default());
        self.dispatch(id);
    }
}

#[test]
fn dramatic_host_setup_launches_the_same_fight_for_both_allies_and_late_spectators() {
    let mut room = RoomTest::host_kind(3, false, RoomKind::Dramatic);
    room.stream_cap = Some(215);
    room.challenge_kind(1, RoomKind::Dramatic);
    for _ in 0..60 { room.tick(); }
    assert_eq!(room.peers[0].screen(), "dramatic setup");
    assert!(room.peers[1].fight().is_none());

    for input in [MenuInput::Down, MenuInput::Down, MenuInput::Right,
        MenuInput::Down, MenuInput::Right, MenuInput::Down, MenuInput::Confirm] {
        room.press(0, input);
    }
    for _ in 0..30 { room.tick(); }
    assert_eq!(room.peers[1].screen(), "dramatic guest");
    assert!(room.peers[0].fight().is_none(), "the host waits for Ally 2 to lock in");
    room.press(1, MenuInput::Right); // Preset Ally 2 remains Ken.
    room.press(1, MenuInput::Confirm);
    for _ in 0..300 {
        if room.host_next >= 120 { break; }
        room.tick_input(Some((0, MenuInput::Right)));
    }
    assert_eq!(room.host_next, 120);
    for peer in &room.peers[..2] {
        let config = peer.fight().expect("the peer starts the committed co-op fight").setup().config;
        let rules = config.dramatic.expect("the native rules reach Fight");
        assert_eq!([config.roster.p1, rules.ally2, config.roster.p2],
            [TournamentFighter::Ryu, TournamentFighter::Ken, TournamentFighter::Sagat]);
        assert_eq!(rules.stage, TournamentFighter::Sagat);
        assert_eq!(rules.lifebar, LifebarMode::Independent);
        assert!(rules.friendly_fire);
        assert_eq!(rules.rivalry, Some(0));
    }

    room.spectate_code(2);
    for _ in 0..500 {
        if room.peers[2].spectator_frame() == Some(200) { break; }
        room.tick_input(Some((0, MenuInput::Right)));
    }
    assert_eq!(room.peers[2].spectator_frame(), Some(200));
    let observer = room.peers[2].fight().expect("the spectator starts the committed co-op fight");
    assert_eq!(observer.state_checksum(), room.host_confirmed_200.expect("host confirmed frame 199"));
    assert!(!room.packets.iter().any(|(from, _, packet)| *from == 2 && matches!(packet, NetPacket::Input { .. })));
}

#[test]
fn dramatic_custom_setup_leaves_the_second_ally_to_the_guest() {
    let mut room = RoomTest::host_kind(2, true, RoomKind::Dramatic);
    room.challenge_kind(1, RoomKind::Dramatic);
    for _ in 0..60 { room.tick(); }
    assert_eq!(room.peers[0].screen(), "dramatic setup");
    room.press(0, MenuInput::Left); // Custom 2v1.
    room.press(0, MenuInput::Up);
    room.press(0, MenuInput::Confirm);
    assert_eq!(room.peers[0].screen(), "dramatic select");
    let host_press = |room: &mut RoomTest, event| {
        let mut input = key(event);
        input.pads[0] = Some(MenuKeys::default());
        room.tick_frame(Some((0, input)));
        room.tick_frame(Some((0, FrameInputs { pads: input.pads, ..FrameInputs::default() })));
    };
    // The keyboard can confirm host picks with only one pad connected.
    host_press(&mut room, MenuInput::Confirm); // Ally 1.
    host_press(&mut room, MenuInput::Start); // Boss.
    assert_eq!(room.peers[0].screen(), "stage select");
    room.press(0, MenuInput::Right);
    host_press(&mut room, MenuInput::Confirm);
    for _ in 0..30 { room.tick(); }
    assert_eq!(room.peers[1].screen(), "dramatic guest");
    room.press(1, MenuInput::Left); // Move from the default Ken to Ryu.
    room.press(1, MenuInput::Confirm);
    for _ in 0..30 { room.tick(); }

    let config = |side: usize| room.peers[side].fight().expect("the committed custom fight starts").setup().config;
    assert_eq!(config(0), config(1));
    let rules = config(0).dramatic.unwrap();
    assert_eq!(rules.ally2, TournamentFighter::Ryu);
    assert_eq!(rules.rivalry, None);
    assert_eq!(rules.stage, TournamentFighter::Geki);
    assert_eq!(rules.lifebar, LifebarMode::Shared);
}

#[test]
fn a_spectator_mirrors_both_fighter_locks_and_the_stage_picker() {
    let mut room = RoomTest::host(3);
    room.spectate(2);
    room.challenge(1);
    for _ in 0..60 { room.tick(); }
    assert!(room.peers.iter().all(|peer| peer.screen() == "select"));
    room.press(1, MenuInput::Confirm);
    for _ in 0..4 { room.tick(); }
    room.press(0, MenuInput::Confirm);
    for _ in 0..60 { room.tick(); }
    assert!(room.peers.iter().all(|peer| peer.screen() == "stage select"),
        "spectators must mirror both assigned fighter locks: {:?}",
        room.peers.iter().map(GameSession::screen).collect::<Vec<_>>());
}

#[test]
fn spectators_acknowledge_a_confirmed_stream_from_frame_zero_without_sending_inputs() {
    let mut room = RoomTest::host(3);
    room.spectate(2);
    room.challenge(1);
    for _ in 0..60 { room.tick(); }
    room.press(1, MenuInput::Confirm);
    for _ in 0..4 { room.tick(); }
    room.press(0, MenuInput::Confirm);
    for _ in 0..60 { room.tick(); }
    assert!(room.peers.iter().all(|peer| peer.screen() == "stage select"));
    room.press(0, MenuInput::Confirm);
    for _ in 0..120 { room.tick_input(Some((0, MenuInput::Right))); }
    assert!(room.peers.iter().all(|peer| peer.fight().is_some()));
    assert!(room.packets.iter().any(|(from, to, packet)| *from == 0 && *to == 2
        && matches!(packet, NetPacket::SpectatorChunk { start_frame: 0, .. })),
        "the host streams confirmed pairs from frame zero to its accepted observer");
    assert!(room.packets.iter().any(|(from, _, packet)| *from == 2
        && matches!(packet, NetPacket::SpectatorAck { next_expected_frame, .. } if *next_expected_frame > 32)),
        "the observer acknowledges contiguous received pairs so streaming can progress");
    assert!(!room.packets.iter().any(|(from, _, packet)| *from == 2 && matches!(packet, NetPacket::Input { .. })),
        "observer inputs do not enter fighter rollback or pacing");
    let mut confirmed_end = 0;
    let base = room.packets.iter().find_map(|(from, to, packet)| match packet {
        NetPacket::Welcome { accepted: true, frame_base, .. } if *from == 0 && *to == 1 => Some(*frame_base),
        _ => None,
    }).unwrap();
    for (from, _, packet) in &room.packets {
        if *from != 0 { continue; }
        match packet {
            NetPacket::Input { checksum_frame, .. } if *checksum_frame != u32::MAX => {
                confirmed_end = confirmed_end.max(checksum_frame.checked_sub(base).unwrap() + 1);
            }
            NetPacket::SpectatorChunk { start_frame, count, .. } => {
                assert!((1..=16).contains(count));
                assert!(*start_frame + u32::from(*count) <= confirmed_end,
                    "a chunk contains only frames already confirmed at its send boundary");
            }
            _ => {}
        }
    }
}

#[test]
fn pre_match_and_mid_match_spectators_replay_the_same_confirmed_native_frame_200() {
    let mut room = RoomTest::host_with_visibility(4, false);
    // Receiving 215 confirmed pairs makes frame200 the last playable state
    // behind the observer's fifteen-frame buffer. Later spectator data is lost.
    room.stream_cap = Some(215);
    room.spectate_code(2);
    room.challenge(1);
    for _ in 0..60 { room.tick(); }
    room.press(1, MenuInput::Confirm);
    for _ in 0..4 { room.tick(); }
    room.press(0, MenuInput::Confirm);
    for _ in 0..60 { room.tick(); }
    assert!(room.peers[..3].iter().all(|peer| peer.screen() == "stage select"));
    room.press(0, MenuInput::Confirm);
    let mut saw_initial_buffer = false;
    for _ in 0..300 {
        if room.host_next >= 120 { break; }
        room.tick_input(Some((0, MenuInput::Right)));
        let received = room.packets.iter().rev().find_map(|(from, _, packet)| match packet {
            NetPacket::SpectatorAck { next_expected_frame, .. } if *from == 2 => Some(*next_expected_frame),
            _ => None,
        }).unwrap_or(0);
        if room.peers[2].fight().is_some() && received <= 15 {
            assert_eq!(room.peers[2].spectator_frame(), Some(0), "fifteen received pairs remain buffered");
            saw_initial_buffer = true;
        }
    }
    assert_eq!(room.host_next, 120);
    assert!(saw_initial_buffer);
    room.spectate_code(3);
    let mut previous = [room.peers[2].spectator_frame().unwrap(), 0];
    let mut caught_up_more_than_one = false;
    for _ in 0..600 {
        room.tick_input(Some((0, MenuInput::Right)));
        for (index, id) in [2, 3].into_iter().enumerate() {
            if let Some(frame) = room.peers[id].spectator_frame() {
                let advanced = frame - previous[index];
                assert!(advanced <= 8, "an observer replays at most eight frames per display tick");
                if id == 3 && advanced > 1 { caught_up_more_than_one = true; }
                previous[index] = frame;
            }
        }
        if previous == [200, 200] { break; }
    }
    assert_eq!(previous, [200, 200], "both observers reach the same buffered confirmed boundary");
    assert!(caught_up_more_than_one, "the frame120 join uses faster catch-up instead of staying behind");
    let (reference, captured_hash) = room.host_snapshot_200.as_ref().expect("host frame200 is captured at the public wire boundary");
    let confirmed_hash = room.host_confirmed_200.expect("host frame199 is subsequently confirmed");
    assert_eq!(*captured_hash, confirmed_hash, "the captured native reference was confirmed without correction");
    for id in [2, 3] {
        let fight = room.peers[id].fight().unwrap();
        let actual = fight.save_netplay_snapshot(200);
        assert!(actual.machine == reference.machine, "observer{id} matches the confirmed native registers, RAM, scheduler and inputs");
        assert_eq!(fight.state_checksum(), confirmed_hash, "observer{id} matches the host's full canonical gameplay identity");
    }
    assert!(!room.packets.iter().any(|(from, _, packet)| *from >= 2 && matches!(packet, NetPacket::Input { .. })));
}

#[test]
fn spectator_loss_duplicates_reordering_and_disconnect_do_not_change_combatant_pacing_or_state() {
    let mut room = RoomTest::host(3);
    room.spectate(2);
    room.challenge(1);
    for _ in 0..60 { room.tick(); }
    room.press(1, MenuInput::Confirm);
    for _ in 0..4 { room.tick(); }
    room.press(0, MenuInput::Confirm);
    for _ in 0..60 { room.tick(); }
    room.press(0, MenuInput::Confirm);
    for _ in 0..20 { room.tick(); }
    assert!(room.peers.iter().all(|peer| peer.fight().is_some()));
    let mut disconnected = room.clone();
    room.spectator_faults = true;
    disconnected.spectator_faults = true;
    disconnected.drop_spectator_packets = true;
    for tick in 0..600 {
        let input = if tick % 60 < 30 { Some((0, MenuInput::Right)) } else { Some((1, MenuInput::Left)) };
        room.tick_input(input);
        disconnected.tick_input(input);
        for side in 0..2 {
            let live = room.peers[side].fight().unwrap();
            let gone = disconnected.peers[side].fight().unwrap();
            assert!(live.machine().save_snapshot() == gone.machine().save_snapshot(),
                "combatant{side}, tick{tick}: spectator traffic never changes native advancement or state");
            assert_eq!(live.state_checksum(), gone.state_checksum(), "combatant{side}, tick{tick}");
        }
    }
    assert!(room.peers[2].spectator_frame().is_some_and(|frame| frame > 200),
        "the lossy/reordered observer stream still makes contiguous playback progress");
    assert_eq!(disconnected.peers[2].screen(), "online", "an observer with all traffic lost times out locally");
    assert!(room.peers[..2].iter().all(|peer| peer.fight().is_some()));
    assert!(disconnected.peers[..2].iter().all(|peer| peer.fight().is_some()), "an observer timeout cannot end either fighter's match");
    assert!(!room.packets.iter().any(|(from, _, packet)| *from == 2 && matches!(packet, NetPacket::Input { .. })));
}

#[test]
fn spectator_pause_keeps_playback_live_and_has_no_restart_or_fighter_forfeit() {
    let mut room = RoomTest::host(3);
    room.spectate(2);
    room.challenge(1);
    for _ in 0..60 { room.tick(); }
    room.press(1, MenuInput::Confirm);
    for _ in 0..4 { room.tick(); }
    room.press(0, MenuInput::Confirm);
    for _ in 0..60 { room.tick(); }
    room.press(0, MenuInput::Confirm);
    for _ in 0..200 {
        room.tick();
        if room.peers[2].fight().is_some_and(|fight| !fight.driving()) { break; }
    }
    room.press(2, MenuInput::Start);
    assert_eq!(room.peers[2].screen(), "pause");
    let before = room.peers[2].spectator_frame().unwrap();
    room.peers[2].step(key(MenuInput::Down));
    room.dispatch(2);
    room.peers[2].step(FrameInputs::default());
    room.dispatch(2);
    let output = room.peers[2].step(key(MenuInput::Confirm));
    assert_eq!(output.transition, Some(Transition::Choose(MenuItem::Overlays)),
        "an observer can change local overlays instead of restarting native match state");
    room.dispatch(2);
    room.peers[2].step(FrameInputs::default());
    room.dispatch(2);
    for _ in 0..30 { room.tick(); }
    assert!(room.peers[2].spectator_frame().unwrap() > before, "a local observer menu keeps confirmed playback moving");
    for _ in 0..3 { room.press(2, MenuInput::Down); }
    room.press(2, MenuInput::Confirm);
    assert_eq!(room.peers[2].screen(), "online");
    assert!(room.peers[..2].iter().all(|peer| peer.fight().is_some()), "leaving observation does not forfeit either fighter");
    assert!(!room.packets.iter().any(|(from, _, packet)| *from == 2 && matches!(packet, NetPacket::Input { .. })));
}

#[test]
fn four_spectators_can_join_before_the_challenger_without_occupying_player_two() {
    let mut room = RoomTest::host(7);
    for id in 1..=4 { room.spectate(id); }
    for _ in 0..60 { room.tick(); }
    for id in 1..=4 {
        assert!(room.packets.iter().any(|(from, _, packet)| *from == id && matches!(packet,
            NetPacket::Hello { kind: RoomKind::Spectate, .. })));
        assert!(room.packets.iter().any(|(from, to, packet)| *from == 0 && usize::from(*to) == id
            && matches!(packet, NetPacket::Welcome { accepted: true, .. })), "spectator {id} is admitted");
    }
    assert_eq!(room.peers[0].screen(), "online", "spectators do not start the host's fighter select");
    room.spectate(6);
    for _ in 0..40 { room.tick(); }
    assert_eq!(room.peers[6].screen(), "online");
    assert!(room.peers[6].notice().is_some_and(|notice| notice.contains("FULL")));
    assert!(room.packets.iter().any(|(from, to, packet)| *from == 0 && *to == 6 && matches!(packet,
        NetPacket::Welcome { accepted: false, reason: sf_engine::netplay::RejectReason::RoomFull, .. })));
    room.challenge(5);
    for _ in 0..60 { room.tick(); }
    assert_eq!(room.peers[0].screen(), "select");
    assert_eq!(room.peers[5].screen(), "select", "the challenger is accepted even though route1 is a spectator");
    assert!(!room.packets.iter().any(|(from, _, packet)| (1..=4).contains(from)
        && matches!(packet, NetPacket::Input { .. })), "spectators send no gameplay inputs");
}
