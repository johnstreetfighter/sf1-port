//! NP5: online room paths must agree about stage flow and live-match metadata.

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, Transition, TypedKey};
use sf_engine::netplay::bus::{VirtualNetBus, VirtualNetConfig};
use sf_engine::netplay::protocol::LobbyPhase;
use sf_engine::netplay::{Link, NetPacket, NetTransportCommand, NetTransportEvent, Room, RoomKind};
use sf_engine::roster::{MenuInput, StagePick};
use sf_video::background_bank::BackgroundBank;

fn session(preset: RulesetPreset) -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    ).expect("NP5 requires the verified local ROMs");
    let mut bank = BackgroundBank::new();
    bank.add_rom_stages(&assets).expect("the real stage bank loads");
    bank.add_vs_screens(&assets).expect("the real VS screens load");
    assert_eq!(bank.listed_stages().len(), 10);
    GameSession::new(assets, MatchConfig::from_preset(preset))
        .with_background_bank(bank)
}

fn key(key: MenuInput) -> FrameInputs {
    let mut input = FrameInputs::default();
    match key {
        MenuInput::Down => input.p1.directions[2] = true,
        MenuInput::Left => input.p1.directions[1] = true,
        MenuInput::Right => input.p1.directions[0] = true,
        MenuInput::Confirm => input.menu.confirm = true,
        MenuInput::Start => input.menu.start = true,
        _ => panic!("unsupported test key"),
    }
    input
}

fn pulse(peer: &mut GameSession, input: MenuInput) {
    peer.step(key(input));
    peer.step(FrameInputs::default());
}

fn online(peer: &mut GameSession) {
    peer.step(FrameInputs::default());
    for input in [MenuInput::Start, MenuInput::Down, MenuInput::Confirm,
        MenuInput::Down, MenuInput::Confirm] {
        pulse(peer, input);
    }
    assert_eq!(peer.screen(), "online");
    peer.drain_net_commands();
}

struct Pair {
    peers: [GameSession; 2],
    bus: VirtualNetBus,
    host_commands: Vec<NetTransportCommand>,
}

impl Pair {
    fn step(&mut self, inputs: [FrameInputs; 2]) -> [FrameOutput; 2] {
        let output = std::array::from_fn(|side| {
            let peer = &mut self.peers[side];
            peer.set_net_time_ms(self.bus.now_ms() as u32);
            peer.push_net_events(&self.bus.drain_events(side as u8));
            let output = peer.step(inputs[side]);
            let commands = peer.drain_net_commands();
            if side == 0 { self.host_commands.extend(commands.iter().cloned()); }
            self.bus.submit(side as u8, &commands);
            output
        });
        self.bus.step_ms(17);
        output
    }

    fn idle(&mut self) { self.step([FrameInputs::default(); 2]); }

    fn pulse(&mut self, side: usize, input: MenuInput) -> FrameOutput {
        let mut inputs = [FrameInputs::default(); 2];
        inputs[side] = key(input);
        let output = self.step(inputs);
        self.idle();
        output.into_iter().nth(side).unwrap()
    }

    fn confirm_fighter(&mut self, side: usize) {
        if self.pulse(side, MenuInput::Confirm).transition == Some(Transition::StyleOpened) {
            self.pulse(side, MenuInput::Confirm);
        }
    }

    fn connected(preset: RulesetPreset, override_side: Option<usize>) -> Self {
        let mut pair = Self {
            peers: [session(preset), session(preset)],
            bus: VirtualNetBus::new(VirtualNetConfig {
                latency_ms: 17, ..VirtualNetConfig::default()
            }),
            host_commands: Vec::new(),
        };
        if let Some(side) = override_side { pair.peers[side].set_stage(Some(StagePick::Bank(9))); }
        online(&mut pair.peers[0]);
        pulse(&mut pair.peers[0], MenuInput::Right);
        for _ in 0..4 { pulse(&mut pair.peers[0], MenuInput::Down); }
        pulse(&mut pair.peers[0], MenuInput::Confirm);
        let commands = pair.peers[0].drain_net_commands();
        pair.host_commands.extend(commands.iter().cloned());
        pair.bus.submit(0, &commands);
        online(&mut pair.peers[1]);
        pulse(&mut pair.peers[1], MenuInput::Left);
        pulse(&mut pair.peers[1], MenuInput::Down);
        pulse(&mut pair.peers[1], MenuInput::Confirm);
        pair.peers[1].type_keys("SF1-7F00-0001".chars()
            .map(TypedKey::Char).chain([TypedKey::Enter]));
        pair.peers[1].step(FrameInputs::default());
        pair.bus.submit(1, &pair.peers[1].drain_net_commands());
        for _ in 0..180 {
            pair.idle();
            if pair.peers.iter().all(|peer| peer.screen() == "select") { return pair; }
        }
        panic!("both peers must complete the public handshake");
    }
}

#[test]
fn a_local_stage_override_on_either_peer_keeps_the_shared_online_stage_flow() {
    // GameSession::set_stage documents this as a Local/Training development hook.
    for override_side in 0..2 {
        let mut pair = Pair::connected(RulesetPreset::DefinitiveOverhaul, Some(override_side));
        pair.confirm_fighter(1);
        for _ in 0..5 { pair.idle(); }
        pair.confirm_fighter(0);
        for _ in 0..60 { pair.idle(); }
        let screens = pair.peers.each_ref().map(|peer| peer.screen());
        assert_eq!(screens, ["stage select"; 2],
            "override on side {override_side} must not bypass the synchronized stage screen");
        // Move away from Bank(9), including when it was the room's initial cursor.
        for _ in 0..2 { pair.pulse(0, MenuInput::Right); }
        let Some(Transition::Stage(chosen)) = pair.pulse(0, MenuInput::Confirm).transition else {
            panic!("the host's shared stage cursor must lock a concrete stage");
        };
        assert_ne!(chosen, StagePick::Bank(9));
        for _ in 0..120 {
            pair.idle();
            if pair.peers.iter().all(|peer| peer.fight().is_some()) { break; }
        }
        for peer in &pair.peers {
            let fight = peer.fight().expect("both online peers start the shared fight");
            assert_eq!(fight.setup().config.roster.stage, Some(chosen),
                "the local override must not replace the shared stage lock");
        }
    }
}

#[test]
fn faithful_rooms_publish_the_real_match_and_late_spectators_receive_its_fight_phase() {
    let mut pair = Pair::connected(RulesetPreset::Faithful1987, None);
    pair.pulse(1, MenuInput::Confirm);
    for _ in 0..5 { pair.idle(); }
    pair.pulse(0, MenuInput::Confirm);
    for _ in 0..60 { pair.idle(); }
    assert!(pair.peers.iter().all(|peer| peer.fight().is_some()),
        "Faithful's native flag flow must start both real fights");
    let announcement = pair.host_commands.iter().rev().find_map(|command| match command {
        NetTransportCommand::BindHost { room, .. } => Some(room),
        _ => None,
    }).expect("the real room advertises through public transport commands");
    assert!(announcement.in_match, "Faithful fights must be advertised as in progress");

    let mut observer = session(RulesetPreset::Faithful1987);
    online(&mut observer);
    observer.push_net_events(&[NetTransportEvent::RoomsDiscovered { rooms: vec![Room {
        code: "SF1-7F00-0001".into(), host: "PLAYER".into(), region: String::new(),
        ping_ms: 0, rollback_frames: 8, jitter_ms: 0, link: Link::Ethernet,
        kind: RoomKind::Versus, spectators: 0, what_if: false, bonus: false,
        preset: RulesetPreset::Faithful1987, in_match: true,
    }] }]);
    observer.step(FrameInputs::default());
    for _ in 0..4 { pulse(&mut observer, MenuInput::Down); }
    pulse(&mut observer, MenuInput::Confirm);
    pair.bus.submit(2, &observer.drain_net_commands());
    for _ in 0..180 {
        pair.idle();
        observer.set_net_time_ms(pair.bus.now_ms() as u32);
        observer.push_net_events(&pair.bus.drain_events(2));
        observer.step(FrameInputs::default());
        pair.bus.submit(2, &observer.drain_net_commands());
    }
    let phase = pair.host_commands.iter().rev().find_map(|command| match command {
        NetTransportCommand::SendPacket { to_peer: 2, bytes } => match NetPacket::from_bytes(bytes) {
            Some(NetPacket::Welcome { accepted: true, phase, .. }) => Some(phase),
            _ => None,
        },
        _ => None,
    }).expect("the public spectator join must be accepted");
    assert_eq!(phase, LobbyPhase::Stage,
        "a late observer must join the fight rather than remain on fighter select");
}
