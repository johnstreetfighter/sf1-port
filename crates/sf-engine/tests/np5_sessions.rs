//! NP5 (#308): real online front-end flow through the public transport queues.

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::settings::NetplayHudMode;
use sf_engine::front::{
    FrameInputs, FrameOutput, GameSession, MenuItem, MenuKeys, PlayerInputs, Transition, TypedKey,
};
use sf_engine::netplay::bus::{PacketFault, VirtualNetBus, VirtualNetConfig};
use sf_engine::netplay::protocol::LobbyPhase;
use sf_engine::netplay::AfterMatchChoice;
use sf_engine::netplay::{
    encode_inputs, Link, NetPacket, NetTransportCommand, NetTransportEvent, Room, RoomKind,
};
use sf_engine::roster::{MenuInput, TournamentFighter};
use sf_video::background_bank::BackgroundBank;

fn session() -> GameSession {
    session_with(MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul))
}

fn session_with(config: MatchConfig) -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("NP5 requires the verified local ROMs");
    let mut bank = BackgroundBank::new();
    bank.add_rom_stages(&assets)
        .expect("the real 1987 stage bank must load");
    bank.add_vs_screens(&assets)
        .expect("the real VS screens must load");
    assert_eq!(bank.listed_stages().len(), 10);
    GameSession::new(assets, config).with_background_bank(bank)
}

fn key(key: MenuInput) -> FrameInputs {
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
    input
}

fn pulse(session: &mut GameSession, input: MenuInput) {
    session.step(key(input));
    session.step(FrameInputs::default());
}

fn online(session: &mut GameSession) {
    session.step(FrameInputs::default());
    for input in [
        MenuInput::Start,
        MenuInput::Down,
        MenuInput::Confirm,
        MenuInput::Down,
        MenuInput::Confirm,
    ] {
        pulse(session, input);
    }
    assert_eq!(session.screen(), "online");
    session.drain_net_commands();
}

#[derive(Clone)]
struct Pair {
    peers: [GameSession; 2],
    bus: VirtualNetBus,
    packets: Vec<(usize, NetPacket)>,
    drop_votes: usize,
    drop_welcome: Option<u32>,
    dropped: Vec<(usize, NetPacket)>,
    drop_inputs: bool,
}

impl Pair {
    fn new(peers: [GameSession; 2]) -> Self {
        Self {
            peers,
            bus: VirtualNetBus::new(VirtualNetConfig {
                latency_ms: 17,
                ..VirtualNetConfig::default()
            }),
            packets: Vec::new(),
            drop_votes: 0,
            drop_welcome: None,
            dropped: Vec::new(),
            drop_inputs: false,
        }
    }

    fn after_match() -> Self {
        static MATCH: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();
        MATCH.get_or_init(Self::play_to_after_match).clone()
    }

    fn play_to_after_match() -> Self {
        let mut pair = Self::fighting();
        for frame in 0..20_000 {
            let phase = frame % 120;
            let mut input = FrameInputs {
                pads: [Some(MenuKeys::default()), None],
                ..FrameInputs::default()
            };
            input.p1.directions[0] = phase < 40;
            if phase >= 40 && phase % 10 < 3 {
                input.p1.buttons[((frame / 10) % 6) as usize] = true;
            }
            pair.step([input, FrameInputs::default()]);
            if pair.peers.iter().all(|peer| peer.screen() == "after match") {
                assert!(pair
                    .peers
                    .iter()
                    .all(|peer| peer.fight().unwrap().is_over()));
                assert!(pair
                    .peers
                    .iter()
                    .all(|peer| peer.fight().unwrap().match_winner() == Some(0)));
                return pair;
            }
        }
        panic!(
            "the real attacking Ryu must finish both native rounds: {:?}",
            pair.peers.each_ref().map(|peer| peer.screen())
        );
    }

    fn fighting() -> Self {
        let mut pair = Self::stage_select();
        pair.start_fight();
        pair
    }

    fn start_fight(&mut self) {
        self.pulse(0, MenuInput::Confirm);
        for _ in 0..3_000 {
            self.idle();
            if self
                .peers
                .iter()
                .all(|peer| peer.fight().is_some_and(|fight| !fight.driving()))
            {
                return;
            }
        }
        panic!("both real machines must complete their native intro");
    }

    fn stage_select() -> Self {
        let mut pair = Self::connected();
        pair.lock_fighters();
        pair
    }

    fn lock_fighters(&mut self) {
        self.pulse(1, MenuInput::Confirm);
        for _ in 0..5 {
            self.idle();
        }
        self.pulse(0, MenuInput::Confirm);
        for _ in 0..60 {
            self.idle();
            if self
                .peers
                .iter()
                .all(|peer| peer.screen() == "stage select")
            {
                break;
            }
        }
        assert!(self
            .peers
            .iter()
            .all(|peer| peer.screen() == "stage select"));
    }

    fn connected() -> Self {
        let mut client = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
        client.difficulty = 4;
        let mut pair = Self::new([session(), session_with(client)]);
        online(&mut pair.peers[0]);
        pulse(&mut pair.peers[0], MenuInput::Right);
        for _ in 0..4 {
            pulse(&mut pair.peers[0], MenuInput::Down);
        }
        pulse(&mut pair.peers[0], MenuInput::Confirm);
        pair.bus.submit(0, &pair.peers[0].drain_net_commands());
        online(&mut pair.peers[1]);
        pulse(&mut pair.peers[1], MenuInput::Left);
        pulse(&mut pair.peers[1], MenuInput::Down);
        pulse(&mut pair.peers[1], MenuInput::Confirm);
        pair.peers[1].type_keys(
            "SF1-7F00-0001"
                .chars()
                .map(TypedKey::Char)
                .chain([TypedKey::Enter]),
        );
        pair.peers[1].step(FrameInputs::default());
        pair.bus.submit(1, &pair.peers[1].drain_net_commands());
        for _ in 0..180 {
            pair.idle();
            if pair.peers.iter().all(|peer| peer.screen() == "select") {
                return pair;
            }
        }
        panic!(
            "both peers must finish the public handshake: {:?}",
            pair.peers.each_ref().map(|peer| peer.screen())
        );
    }

    fn rehost(mut host: GameSession) -> Self {
        assert_eq!(host.screen(), "online");
        host.drain_net_commands();
        pulse(&mut host, MenuInput::Back);
        pulse(&mut host, MenuInput::Confirm);
        pulse(&mut host, MenuInput::Right);
        for _ in 0..4 {
            pulse(&mut host, MenuInput::Down);
        }
        pulse(&mut host, MenuInput::Confirm);
        let host_commands = host.drain_net_commands();
        assert!(host_commands
            .iter()
            .any(|command| matches!(command, NetTransportCommand::BindHost { .. })));
        let mut client = session();
        online(&mut client);
        pulse(&mut client, MenuInput::Left);
        pulse(&mut client, MenuInput::Down);
        pulse(&mut client, MenuInput::Confirm);
        client.type_keys(
            "SF1-7F00-0001"
                .chars()
                .map(TypedKey::Char)
                .chain([TypedKey::Enter]),
        );
        client.step(FrameInputs::default());
        let client_commands = client.drain_net_commands();
        let mut pair = Self::new([host, client]);
        pair.bus.submit(0, &host_commands);
        pair.bus.submit(1, &client_commands);
        for _ in 0..180 {
            pair.idle();
            if pair.peers.iter().all(|peer| peer.screen() == "select") {
                return pair;
            }
        }
        panic!("both peers must finish the new-room public handshake");
    }

    fn step(&mut self, inputs: [FrameInputs; 2]) -> [FrameOutput; 2] {
        let output = std::array::from_fn(|side| {
            let peer = &mut self.peers[side];
            peer.set_net_time_ms(self.bus.now_ms() as u32);
            peer.push_net_events(&self.bus.drain_events(side as u8));
            let output = peer.step(inputs[side]);
            let commands = peer.drain_net_commands();
            for command in &commands {
                if let NetTransportCommand::SendPacket { bytes, .. } = command {
                    if let Some(packet) = NetPacket::from_bytes(bytes) {
                        let drop = match &packet {
                            NetPacket::Input { .. } if self.drop_inputs => true,
                            NetPacket::AfterMatchVote { .. } if self.drop_votes > 0 => {
                                self.drop_votes -= 1;
                                true
                            }
                            NetPacket::Welcome {
                                accepted: true,
                                generation,
                                ..
                            } if self.drop_welcome == Some(*generation) => {
                                self.drop_welcome = None;
                                true
                            }
                            _ => false,
                        };
                        if drop {
                            self.dropped.push((side, packet.clone()));
                            self.bus.set_fault_script(vec![PacketFault {
                                delay_ms: 0,
                                drop: true,
                                copies: 1,
                            }]);
                        }
                        self.packets.push((side, packet));
                    }
                }
                self.bus.submit(side as u8, std::slice::from_ref(command));
            }
            output
        });
        self.bus.step_ms(17);
        output
    }

    fn idle(&mut self) -> [FrameOutput; 2] {
        self.step([FrameInputs::default(); 2])
    }

    fn pulse(&mut self, side: usize, input: MenuInput) -> [FrameOutput; 2] {
        let mut inputs = [FrameInputs::default(); 2];
        inputs[side] = key(input);
        let output = self.step(inputs);
        self.idle();
        output
    }
}

#[test]
fn each_local_p1_controls_its_assigned_fighter_and_both_locks_reach_a_real_rollback_fight() {
    let mut pair = Pair::connected();
    let picked = pair.pulse(1, MenuInput::Confirm);
    assert_eq!(
        picked[1].transition,
        Some(Transition::Picked(TournamentFighter::Ken)),
        "the client's physical P1 locks the assigned P2 fighter, never the host's P1"
    );
    assert!(
        pair.packets.iter().any(|(sender, packet)| matches!(
            (sender, packet),
            (
                1,
                NetPacket::LobbyPick {
                    side: 1,
                    fighter: TournamentFighter::Ken,
                    locked: true,
                    phase: LobbyPhase::Fighters,
                    ..
                }
            )
        )),
        "the local lock is sent through the public queue"
    );
    for _ in 0..5 {
        pair.idle();
    }
    pair.pulse(0, MenuInput::Confirm);
    for _ in 0..60 {
        pair.idle();
        if pair
            .peers
            .iter()
            .all(|peer| peer.screen() == "stage select")
        {
            break;
        }
    }
    assert!(
        pair.peers
            .iter()
            .all(|peer| peer.screen() == "stage select"),
        "both locked fighters must be replicated before stage select: {:?}",
        pair.peers.each_ref().map(|peer| peer.screen())
    );
    pair.pulse(0, MenuInput::Confirm);
    for _ in 0..120 {
        pair.idle();
        if pair.peers.iter().all(|peer| peer.fight().is_some()) {
            break;
        }
    }
    assert!(
        pair.peers.iter().all(|peer| peer.fight().is_some()),
        "the shared stage lock starts both real Fights"
    );
    let initial = pair
        .peers
        .each_ref()
        .map(|peer| peer.fight_machine().unwrap().state.frame);
    let local = [
        PlayerInputs {
            directions: [true, false, false, false],
            ..PlayerInputs::default()
        },
        PlayerInputs {
            directions: [false, true, false, false],
            ..PlayerInputs::default()
        },
    ];
    for _ in 0..60 {
        pair.step(std::array::from_fn(|side| FrameInputs {
            p1: local[side],
            p2: PlayerInputs {
                buttons: [false, false, false, false, false, true],
                ..PlayerInputs::default()
            },
            ..FrameInputs::default()
        }));
    }
    for (side, peer) in pair.peers.iter().enumerate() {
        let fight = peer.fight().unwrap();
        assert_eq!(
            (
                fight.setup().config.roster.p1,
                fight.setup().config.roster.p2
            ),
            (TournamentFighter::Ryu, TournamentFighter::Ken)
        );
        assert!(fight.setup().config.roster.stage.is_some());
        assert!(
            peer.fight_machine().unwrap().state.frame > initial[side] + 20,
            "the actual machine advances online"
        );
        assert!(
            pair.packets.iter().any(|(sender, packet)| match packet {
                NetPacket::Input { count, bits, .. } if *sender == side =>
                    bits[..usize::from(*count)].contains(&encode_inputs(local[side])),
                _ => false,
            }),
            "side {side} sends physical P1 inputs, excluding the physical P2 attack"
        );
    }
}

fn assert_live_neutral_step(
    paused: &mut Pair,
    control: &mut Pair,
    client: FrameInputs,
) -> [FrameOutput; 2] {
    let shown = paused.step([FrameInputs::default(), client]);
    control.idle();
    for side in 0..2 {
        let actual = paused.peers[side]
            .fight()
            .expect("the online fight remains open");
        let expected = control.peers[side].fight().unwrap();
        assert_eq!(
            actual.machine().state.frame,
            expected.machine().state.frame,
            "online menus never hold a native frame on side {side}"
        );
        let ram_differences: Vec<_> = actual
            .machine()
            .work_ram()
            .iter()
            .zip(expected.machine().work_ram())
            .enumerate()
            .filter(|(_, (actual, expected))| actual != expected)
            .take(8)
            .map(|(at, (actual, expected))| (at, *actual, *expected))
            .collect();
        assert!(actual.save_netplay_snapshot(0).machine == expected.save_netplay_snapshot(0).machine,
            "neutral simulation differs on side {side}: screen={}, transition={:?}, first RAM differences={ram_differences:?}",
            shown[side].screen, shown[side].transition);
        assert_eq!(actual.state_checksum(), expected.state_checksum());
    }
    shown
}

#[test]
fn online_pause_and_moves_keep_simulating_neutral_input_and_resume_without_a_held_frame() {
    let mut paused = Pair::fighting();
    let mut control = paused.clone();
    let opened = assert_live_neutral_step(&mut paused, &mut control, key(MenuInput::Start));
    assert_eq!(opened[1].screen, "pause");
    assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    for _ in 0..30 {
        assert_live_neutral_step(
            &mut paused,
            &mut control,
            FrameInputs {
                p1: PlayerInputs {
                    directions: [true, false, false, false],
                    buttons: [true; 6],
                },
                pads: [Some(MenuKeys::default()), None],
                ..FrameInputs::default()
            },
        );
    }
    for _ in 0..5 {
        assert_live_neutral_step(&mut paused, &mut control, key(MenuInput::Down));
        assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    }
    let moves = assert_live_neutral_step(&mut paused, &mut control, key(MenuInput::Confirm));
    assert!(
        moves[1].panel.is_some(),
        "the sixth online pause row opens MOVES"
    );
    assert!(paused.peers[1].move_list().is_some());
    assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    for _ in 0..20 {
        assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    }
    let resumed = assert_live_neutral_step(&mut paused, &mut control, key(MenuInput::Start));
    assert_ne!(resumed[1].screen, "pause");
    assert!(resumed[1].panel.is_none());
    assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    assert_live_neutral_step(&mut paused, &mut control, key(MenuInput::Start));
    assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    let closed = assert_live_neutral_step(
        &mut paused,
        &mut control,
        FrameInputs {
            p1: PlayerInputs {
                buttons: [true, false, false, false, false, false],
                ..PlayerInputs::default()
            },
            ..FrameInputs::default()
        },
    );
    assert_eq!(
        closed[1].transition,
        Some(Transition::Choose(MenuItem::Resume)),
        "keyboard LP acts as menu Confirm and closes Resume"
    );
    assert_ne!(closed[1].screen, "pause");
    let closing_window = paused
        .packets
        .iter()
        .rev()
        .find_map(|(side, packet)| match packet {
            NetPacket::Input { count, bits, .. } if *side == 1 => {
                Some(&bits[..usize::from(*count)])
            }
            _ => None,
        })
        .expect("the closing display tick still sends an input window");
    assert!(
        closing_window.iter().all(|bits| *bits == 0),
        "keyboard Confirm must not leak LP into the closing tick's wire window: {closing_window:?}"
    );
    for _ in 0..6 {
        assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    }
}

#[test]
fn telemetry_uses_measured_link_values_and_local_hud_preferences_do_not_change_simulation() {
    let original = Pair::fighting();
    let with_hud = |mode| {
        let mut pair = original.clone();
        let mut settings = pair.peers[1].settings().clone();
        settings.netplay_hud = mode;
        pair.peers[1] = pair.peers[1].clone().with_settings(settings);
        pair
    };
    let mut off = with_hud(NetplayHudMode::Off);
    let mut compact = with_hud(NetplayHudMode::Compact);
    let mut full = with_hud(NetplayHudMode::Full);
    let off_frame = off.idle();
    let compact_frame = compact.idle();
    let full_frame = full.idle();
    let delay = compact
        .packets
        .iter()
        .find_map(|(_, packet)| match packet {
            NetPacket::Welcome {
                accepted: true,
                assigned_delay,
                ..
            } => Some(*assigned_delay),
            _ => None,
        })
        .expect("the public Welcome negotiated a delay");
    let advantage = full
        .packets
        .iter()
        .rev()
        .find_map(|(side, packet)| match packet {
            NetPacket::Input {
                frame_advantage, ..
            } if *side == 1 => Some(*frame_advantage),
            _ => None,
        })
        .expect("the displayed advantage comes from the same outgoing simulation tick");
    let mut expected_compact = off_frame[1].frame.argb.clone();
    sf_video::compositor::fallback_text(
        &mut expected_compact,
        2,
        2,
        &format!("PING 34MS  RBF 0F  DLY {delay}F  [ETH]"),
        0xffff_d438,
        1,
    );
    assert!(
        compact_frame[1].frame.argb == expected_compact,
        "Compact draws literal measured 34ms RTT telemetry at y2 over the unchanged fight"
    );
    let mut expected_full = off_frame[1].frame.argb.clone();
    sf_video::compositor::fallback_text(
        &mut expected_full,
        2,
        2,
        &format!("PING 34MS  JIT 0MS  RBF 0/8F  DLY {delay}F  ADV {advantage:+}F  [ETH]"),
        0xffff_d438,
        1,
    );
    assert!(
        full_frame[1].frame.argb == expected_full,
        "Full adds literal jitter, rollback limit and signed frame advantage"
    );
    for side in 0..2 {
        let expected = off.peers[side].fight().unwrap();
        for peer in [&compact.peers[side], &full.peers[side]] {
            let actual = peer.fight().unwrap();
            assert!(
                actual.save_netplay_snapshot(0).machine
                    == expected.save_netplay_snapshot(0).machine
            );
            assert_eq!(
                actual.state_checksum(),
                expected.state_checksum(),
                "HUD is strictly local"
            );
        }
    }
}

#[test]
fn client_controller_loss_uses_physical_p1_but_opens_the_assigned_fighters_live_menu() {
    let mut paused = Pair::fighting();
    let mut control = paused.clone();
    let ignored = assert_live_neutral_step(
        &mut paused,
        &mut control,
        FrameInputs {
            pad_lost: [false, true],
            pads: [
                None,
                Some(MenuKeys {
                    start: true,
                    ..MenuKeys::default()
                }),
            ],
            ..FrameInputs::default()
        },
    );
    assert_ne!(
        ignored[1].screen, "pause",
        "the unused physical P2 cannot pause an online client"
    );
    let lost = assert_live_neutral_step(
        &mut paused,
        &mut control,
        FrameInputs {
            pad_lost: [true, false],
            ..FrameInputs::default()
        },
    );
    assert_eq!(lost[1].transition, Some(Transition::Paused));
    assert_eq!(lost[1].screen, "pause");
    {
        let assets = paused.peers[1].assets();
        let font = sf_video::compositor::RomFont::new(&assets.gfx4, &assets.maincpu);
        let mut expected = vec![0xff00_0000; 384 * 224];
        font.text(
            &mut expected,
            12,
            8,
            "CONTROLLER DISCONNECTED",
            sf_video::compositor::ink::YELLOW,
        );
        assert!(
            (64..72).all(|y| lost[1].frame.argb[y * 384 + 96..y * 384 + 280]
                == expected[y * 384 + 96..y * 384 + 280]),
            "the public live pause frame shows the exact controller notice"
        );
    }
    assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    for _ in 0..5 {
        assert_live_neutral_step(&mut paused, &mut control, key(MenuInput::Down));
        assert_live_neutral_step(&mut paused, &mut control, FrameInputs::default());
    }
    assert_live_neutral_step(&mut paused, &mut control, key(MenuInput::Confirm));
    assert_eq!(
        paused.peers[1].move_list().unwrap().fighter(),
        TournamentFighter::Ken,
        "physical P1 controls the client's logical P2 fighter and move list"
    );
}

#[test]
fn one_rematch_vote_waits_and_two_votes_start_a_new_shared_stage_epoch() {
    let mut pair = Pair::after_match();
    let (generation, old_seed, old_delay) = pair
        .packets
        .iter()
        .rev()
        .find_map(|(_, packet)| match packet {
            NetPacket::Welcome {
                accepted: true,
                generation,
                seed,
                assigned_delay,
                ..
            } => Some((*generation, *seed, *assigned_delay)),
            _ => None,
        })
        .expect("the real match has a public Welcome epoch");
    let old_tip = pair
        .packets
        .iter()
        .filter_map(|(_, packet)| match packet {
            NetPacket::Input { current_frame, .. } => Some(*current_frame),
            _ => None,
        })
        .max()
        .unwrap();
    let chosen = pair.pulse(1, MenuInput::Confirm);
    assert_eq!(
        chosen[1].screen, "after match",
        "one Rematch vote waits for the peer instead of restarting locally"
    );
    assert!(
        pair.packets.iter().any(|(side, packet)| matches!(packet,
        NetPacket::AfterMatchVote { generation: epoch, choice: AfterMatchChoice::Rematch, .. }
            if *side == 1 && *epoch == generation)),
        "the vote is sent with its current generation"
    );
    for _ in 0..35 {
        pair.idle();
    }
    assert!(pair.peers.iter().all(|peer| peer.screen() == "after match"));
    pair.pulse(0, MenuInput::Confirm);
    for _ in 0..120 {
        pair.idle();
        if pair
            .peers
            .iter()
            .all(|peer| peer.screen() == "stage select")
        {
            break;
        }
    }
    assert!(
        pair.peers
            .iter()
            .all(|peer| peer.screen() == "stage select"),
        "the host's shared decision opens stage select for both peers"
    );
    let (next_generation, next_base, next_seed, next_delay, phase, picker) = pair
        .packets
        .iter()
        .rev()
        .find_map(|(_, packet)| match packet {
            NetPacket::Welcome {
                accepted: true,
                generation: epoch,
                frame_base,
                seed,
                assigned_delay,
                phase,
                picker,
                ..
            } if *epoch > generation => {
                Some((*epoch, *frame_base, *seed, *assigned_delay, *phase, *picker))
            }
            _ => None,
        })
        .expect("the host publishes the next epoch through reliable Welcome");
    assert_eq!(next_generation, generation + 1);
    assert!(
        next_base > old_tip + 4,
        "new-match wire frames exclude all old delayed-input frames"
    );
    assert_ne!(next_seed, old_seed);
    assert_eq!(
        next_delay, old_delay,
        "the original measured delay remains negotiated"
    );
    assert_eq!(phase, LobbyPhase::Stage);
    assert_eq!(picker, 1, "the losing client controls the next stage");
    let ignored = pair.pulse(0, MenuInput::Confirm);
    assert_eq!(
        ignored[0].screen, "stage select",
        "the winner cannot choose the loser's stage"
    );
    pair.pulse(1, MenuInput::Confirm);
    for _ in 0..120 {
        pair.idle();
        if pair.peers.iter().all(|peer| peer.fight().is_some()) {
            break;
        }
    }
    assert!(
        pair.peers.iter().all(|peer| peer.fight().is_some()),
        "the losing client's physical P1 starts both new Fights"
    );
}

#[test]
fn dropped_vote_and_epoch_messages_recover_and_reselect_resets_both_locks() {
    let mut pair = Pair::after_match();
    let generation = pair
        .packets
        .iter()
        .rev()
        .find_map(|(_, packet)| match packet {
            NetPacket::Welcome {
                accepted: true,
                generation,
                ..
            } => Some(*generation),
            _ => None,
        })
        .unwrap();
    pair.drop_votes = 1;
    pair.drop_welcome = Some(generation + 1);
    pair.pulse(0, MenuInput::Confirm);
    for _ in 0..35 {
        pair.idle();
    }
    assert!(pair.peers.iter().all(|peer| peer.screen() == "after match"));
    assert!(
        pair.packets
            .iter()
            .filter(|(side, packet)| matches!(packet,
        NetPacket::AfterMatchVote { generation: epoch, choice: AfterMatchChoice::Rematch, .. }
            if *side == 0 && *epoch == generation))
            .count()
            >= 2,
        "an unacknowledged vote is retried"
    );
    assert!(pair
        .dropped
        .iter()
        .any(|(side, packet)| *side == 0 && matches!(packet, NetPacket::AfterMatchVote { .. })));
    pair.pulse(1, MenuInput::Down);
    pair.pulse(1, MenuInput::Confirm);
    for _ in 0..120 {
        pair.idle();
        if pair.peers.iter().all(|peer| peer.screen() == "select") {
            break;
        }
    }
    assert!(pair.dropped.iter().any(|(_, packet)| matches!(packet,
        NetPacket::Welcome { generation: epoch, phase: LobbyPhase::Fighters, .. } if *epoch == generation + 1)));
    assert!(
        pair.peers.iter().all(|peer| peer.screen() == "select"),
        "a dropped new Welcome cannot strand either peer in the old match"
    );
    let client = pair.pulse(1, MenuInput::Confirm);
    assert_eq!(
        client[1].transition,
        Some(Transition::Picked(TournamentFighter::Ken)),
        "Reselect clears the previous client lock"
    );
    assert!(pair.peers.iter().all(|peer| peer.fight().is_none()));
}

#[test]
fn leave_vetoes_reselect_returns_both_to_online_and_restores_each_local_configuration() {
    let mut pair = Pair::after_match();
    pair.pulse(1, MenuInput::Down);
    pair.pulse(1, MenuInput::Confirm);
    for _ in 0..5 {
        pair.idle();
    }
    assert!(
        pair.peers.iter().all(|peer| peer.screen() == "after match"),
        "one non-Leave vote waits for its peer"
    );
    pair.pulse(0, MenuInput::Down);
    pair.pulse(0, MenuInput::Down);
    let left = pair.pulse(0, MenuInput::Confirm);
    assert_eq!(left[0].screen, "online");
    for _ in 0..30 {
        pair.idle();
        if pair.peers.iter().all(|peer| peer.screen() == "online") {
            break;
        }
    }
    assert!(pair.peers.iter().all(|peer| peer.screen() == "online"));
    assert!(pair.peers.iter().all(|peer| peer.fight().is_none()));
    assert!(pair
        .peers
        .iter()
        .all(|peer| peer.notice() == Some("PEER LEFT ROOM")));
    let expected_host = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    let mut expected_client = expected_host;
    expected_client.difficulty = 4;
    assert_eq!(*pair.peers[0].match_config(), expected_host);
    assert_eq!(
        *pair.peers[1].match_config(),
        expected_client,
        "the client keeps its own pre-online rules after leaving"
    );
}

fn assert_shared_stage_back(mut pair: Pair, picker: usize) {
    let old_pick = pair
        .packets
        .iter()
        .rev()
        .find_map(|(side, packet)| match packet {
            NetPacket::LobbyPick {
                locked: true,
                phase: LobbyPhase::Fighters,
                ..
            } if *side == 1 => Some(packet.clone()),
            _ => None,
        });
    pair.pulse(picker, MenuInput::Back);
    for _ in 0..120 {
        pair.idle();
        if pair.peers.iter().all(|peer| peer.screen() == "select") {
            break;
        }
    }
    assert!(
        pair.peers.iter().all(|peer| peer.screen() == "select"),
        "the stage picker's Back returns both peers to unlocked character select: {:?}",
        pair.peers.each_ref().map(|peer| peer.screen())
    );
    if let Some(packet) = old_pick {
        pair.peers[0].push_net_events(&[NetTransportEvent::PacketReceived {
            from_peer: 1,
            addr: "127.0.0.2:7001".into(),
            bytes: packet.to_bytes(),
        }]);
        pair.idle();
    }
    let chosen = pair.pulse(0, MenuInput::Confirm);
    assert_eq!(
        chosen[0].transition,
        Some(Transition::Picked(TournamentFighter::Ryu)),
        "a delayed old character lock cannot relock P2 after Stage Back"
    );
}

#[test]
fn stage_back_is_shared_for_host_and_client_pickers_and_old_character_locks_stay_retired() {
    assert_shared_stage_back(Pair::stage_select(), 0);
    let mut rematch = Pair::after_match();
    rematch.pulse(0, MenuInput::Confirm);
    rematch.pulse(1, MenuInput::Confirm);
    for _ in 0..120 {
        rematch.idle();
        if rematch
            .peers
            .iter()
            .all(|peer| peer.screen() == "stage select")
        {
            break;
        }
    }
    assert!(rematch
        .peers
        .iter()
        .all(|peer| peer.screen() == "stage select"));
    assert_shared_stage_back(rematch, 1);
}

fn observer_step(
    pair: &mut Pair,
    observer: &mut GameSession,
    input: FrameInputs,
    packets: &mut Vec<NetPacket>,
) -> FrameOutput {
    pair.idle();
    observer.set_net_time_ms(pair.bus.now_ms() as u32);
    observer.push_net_events(&pair.bus.drain_events(2));
    let output = observer.step(input);
    let commands = observer.drain_net_commands();
    for command in &commands {
        if let NetTransportCommand::SendPacket { bytes, .. } = command {
            if let Some(packet) = NetPacket::from_bytes(bytes) {
                packets.push(packet);
            }
        }
    }
    pair.bus.submit(2, &commands);
    output
}

#[test]
fn an_observer_plays_the_confirmed_terminal_tail_and_waits_for_players_after_match() {
    let mut pair = Pair::after_match();
    let frame_base = pair
        .packets
        .iter()
        .rev()
        .find_map(|(_, packet)| match packet {
            NetPacket::Welcome {
                accepted: true,
                frame_base,
                ..
            } => Some(*frame_base),
            _ => None,
        })
        .unwrap();
    let terminal_next = pair
        .packets
        .iter()
        .filter_map(|(_, packet)| match packet {
            NetPacket::Input { current_frame, .. } => current_frame.checked_sub(frame_base),
            _ => None,
        })
        .max()
        .unwrap();
    let mut observer = session();
    online(&mut observer);
    observer.push_net_events(&[NetTransportEvent::RoomsDiscovered {
        rooms: vec![Room {
            code: "SF1-7F00-0001".into(),
            host: "PLAYER".into(),
            region: String::new(),
            ping_ms: 0,
            rollback_frames: 8,
            jitter_ms: 0,
            link: Link::Ethernet,
            kind: RoomKind::Versus,
            spectators: 0,
            what_if: false,
            bonus: false,
            preset: RulesetPreset::DefinitiveOverhaul,
            in_match: true,
        }],
    }]);
    observer.step(FrameInputs::default());
    for _ in 0..4 {
        pulse(&mut observer, MenuInput::Down);
    }
    pulse(&mut observer, MenuInput::Confirm);
    pair.bus.submit(2, &observer.drain_net_commands());
    let mut packets = Vec::new();
    for _ in 0..terminal_next / 4 + 600 {
        let output = observer_step(
            &mut pair,
            &mut observer,
            FrameInputs::default(),
            &mut packets,
        );
        if output.screen == "after match" {
            break;
        }
    }
    assert_eq!(
        observer.screen(),
        "after match",
        "the final MatchOver frame cannot remain trapped in the 15-frame spectator tail"
    );
    assert!(
        observer.fight().unwrap().save_netplay_snapshot(0).machine
            == pair.peers[0]
                .fight()
                .unwrap()
                .save_netplay_snapshot(0)
                .machine
    );
    assert_eq!(
        observer.fight().unwrap().state_checksum(),
        pair.peers[0].fight().unwrap().state_checksum()
    );
    let confirmed = observer_step(
        &mut pair,
        &mut observer,
        key(MenuInput::Confirm),
        &mut packets,
    );
    assert_eq!(
        confirmed.screen, "after match",
        "an observer cannot locally rematch or decide the players' next epoch"
    );
    observer_step(
        &mut pair,
        &mut observer,
        FrameInputs::default(),
        &mut packets,
    );
    let left = observer_step(&mut pair, &mut observer, key(MenuInput::Back), &mut packets);
    assert_eq!(left.screen, "online");
    assert!(
        packets.iter().all(|packet| !matches!(
            packet,
            NetPacket::Input { .. } | NetPacket::AfterMatchVote { .. }
        )),
        "observation never emits gameplay inputs or player votes"
    );
}

#[test]
fn rehosting_allocates_beyond_every_seen_epoch_and_retires_old_input_datagrams() {
    let mut old = Pair::fighting();
    let mut welcome = old
        .packets
        .iter()
        .rev()
        .find_map(|(_, packet)| match packet {
            packet @ NetPacket::Welcome { accepted: true, .. } => Some(packet.clone()),
            _ => None,
        })
        .unwrap();
    let retired_input = old
        .packets
        .iter()
        .find_map(|(_, packet)| match packet {
            packet @ NetPacket::Input { .. } => Some(packet.clone()),
            _ => None,
        })
        .unwrap();
    let (high_generation, high_base) = if let NetPacket::Welcome {
        generation,
        frame_base,
        phase,
        ..
    } = &mut welcome
    {
        *generation += 32;
        *frame_base += 1_000_000;
        *phase = LobbyPhase::Fighters;
        (*generation, *frame_base)
    } else {
        unreachable!()
    };
    old.peers[1].push_net_events(&[NetTransportEvent::PacketReceived {
        from_peer: 0,
        addr: "127.0.0.1:7000".into(),
        bytes: welcome.to_bytes(),
    }]);
    old.peers[1].step(FrameInputs::default());
    assert_eq!(
        old.peers[1].screen(),
        "select",
        "a lawful higher Welcome advances this client's epoch"
    );
    let mut rehost = old.peers[1].clone();
    pulse(&mut rehost, MenuInput::Back);
    assert_eq!(rehost.screen(), "online");
    let mut next = Pair::rehost(rehost);
    let (generation, frame_base) = next
        .packets
        .iter()
        .find_map(|(_, packet)| match packet {
            NetPacket::Welcome {
                accepted: true,
                generation,
                frame_base,
                ..
            } => Some((*generation, *frame_base)),
            _ => None,
        })
        .unwrap();
    assert!(frame_base > high_base + 4, "rehosting must reserve beyond prior wire frames, even when they exceed display-frame time: old={high_base}, new={frame_base}");
    assert!(
        generation > high_generation,
        "old lobby locks and votes also remain retired across rooms"
    );
    next.pulse(1, MenuInput::Confirm);
    for _ in 0..5 {
        next.idle();
    }
    next.pulse(0, MenuInput::Confirm);
    for _ in 0..60 {
        next.idle();
        if next
            .peers
            .iter()
            .all(|peer| peer.screen() == "stage select")
        {
            break;
        }
    }
    next.pulse(0, MenuInput::Confirm);
    for _ in 0..120 {
        next.idle();
        if next.peers.iter().all(|peer| peer.fight().is_some()) {
            break;
        }
    }
    let mut control = next.clone();
    next.peers[0].push_net_events(&[NetTransportEvent::PacketReceived {
        from_peer: 1,
        addr: "127.0.0.2:7001".into(),
        bytes: retired_input.to_bytes(),
    }]);
    for _ in 0..20 {
        next.idle();
        control.idle();
        for side in 0..2 {
            let actual = next.peers[side].fight().unwrap();
            let expected = control.peers[side].fight().unwrap();
            assert!(
                actual.save_netplay_snapshot(0).machine
                    == expected.save_netplay_snapshot(0).machine
            );
            assert_eq!(
                actual.state_checksum(),
                expected.state_checksum(),
                "a retired datagram cannot affect the new room"
            );
        }
    }
}

#[test]
fn quitting_a_live_match_sends_forfeit_returns_both_online_and_old_forfeits_stay_retired() {
    let mut pair = Pair::fighting();
    pair.pulse(1, MenuInput::Start);
    for _ in 0..4 {
        pair.pulse(1, MenuInput::Down);
    }
    let quit = pair.pulse(1, MenuInput::Confirm);
    assert_eq!(
        quit[1].screen, "online",
        "online Quit returns to the lobby rather than the ordinary Multi Player menu"
    );
    let retired_forfeit = pair
        .packets
        .iter()
        .rev()
        .find_map(|(side, packet)| match packet {
            packet @ NetPacket::Input { count, bits, .. }
                if *side == 1 && bits[..usize::from(*count)].contains(&0x8000) =>
            {
                Some(packet.clone())
            }
            _ => None,
        })
        .expect("the quitting player sends the public bit-15 forfeit before Close");
    for _ in 0..10 {
        pair.idle();
        if pair.peers.iter().all(|peer| peer.screen() == "online") {
            break;
        }
    }
    assert!(
        pair.peers.iter().all(|peer| peer.screen() == "online"),
        "the remote rollback forfeit event ends the match immediately"
    );
    assert!(pair.peers.iter().all(|peer| peer.fight().is_none()));
    let mut next = Pair::rehost(pair.peers[0].clone());
    next.lock_fighters();
    next.start_fight();
    let mut control = next.clone();
    next.peers[0].push_net_events(&[NetTransportEvent::PacketReceived {
        from_peer: 1,
        addr: "127.0.0.2:7001".into(),
        bytes: retired_forfeit.to_bytes(),
    }]);
    for _ in 0..20 {
        next.idle();
        control.idle();
        for side in 0..2 {
            let actual = next.peers[side]
                .fight()
                .expect("an old-room forfeit cannot end a new room");
            let expected = control.peers[side].fight().unwrap();
            assert!(
                actual.save_netplay_snapshot(0).machine
                    == expected.save_netplay_snapshot(0).machine
            );
            assert_eq!(actual.state_checksum(), expected.state_checksum());
        }
    }
}

#[test]
fn gameplay_loss_shows_waiting_and_rollback_timeout_closes_even_while_pings_continue() {
    let mut pair = Pair::fighting();
    let first_packet = pair.packets.len();
    pair.drop_inputs = true;
    for _ in 0..20 {
        pair.idle();
    }
    assert!(pair.peers.iter().all(|peer| peer.fight().is_some()));
    let mut off = pair.clone();
    let mut settings = off.peers[1].settings().clone();
    settings.netplay_hud = NetplayHudMode::Off;
    off.peers[1] = off.peers[1].clone().with_settings(settings);
    let waiting = pair.idle();
    let plain = off.idle();
    let mut expected = plain[1].frame.argb.clone();
    sf_video::compositor::fallback_text(&mut expected, 2, 2, "WAITING FOR PEER...", 0xffff_d438, 1);
    assert!(
        waiting[1].frame.argb == expected,
        "the stalled simulation shows its literal top-edge waiting status"
    );
    let mut closed_at = None;
    for tick in 22..=330 {
        pair.idle();
        if pair.peers.iter().all(|peer| peer.screen() == "online") {
            closed_at = Some(tick);
            break;
        }
    }
    assert!(
        closed_at.is_some_and(|tick| tick >= 300),
        "rollback's 300 waiting ticks terminate missing gameplay independently of connection pings"
    );
    assert!(pair.peers.iter().all(|peer| peer.fight().is_none()));
    assert!(pair
        .peers
        .iter()
        .all(|peer| peer.notice() == Some("PEER LEFT ROOM")));
    assert!(pair.packets[first_packet..]
        .iter()
        .any(|(_, packet)| matches!(packet, NetPacket::Ping { .. })));
    assert!(pair.packets[first_packet..]
        .iter()
        .any(|(_, packet)| matches!(packet, NetPacket::Pong { .. })));
}

#[test]
fn a_confirmed_checksum_mismatch_logs_diagnostics_and_returns_to_online() {
    let mut pair = Pair::fighting();
    for _ in 0..20 {
        pair.idle();
    }
    let base = pair
        .packets
        .iter()
        .find_map(|(_, packet)| match packet {
            NetPacket::Welcome {
                accepted: true,
                frame_base,
                ..
            } => Some(*frame_base),
            _ => None,
        })
        .unwrap();
    let mut corrupt = pair
        .packets
        .iter()
        .rev()
        .find_map(|(side, packet)| match packet {
            packet @ NetPacket::Input { checksum_frame, .. }
                if *side == 1 && *checksum_frame != u32::MAX =>
            {
                Some(packet.clone())
            }
            _ => None,
        })
        .expect("the live peer has a confirmed checksum");
    let (frame, local, remote) = if let NetPacket::Input {
        checksum_frame,
        checksum,
        ..
    } = &mut corrupt
    {
        let local = *checksum;
        *checksum ^= 1;
        (*checksum_frame - base, local, *checksum)
    } else {
        unreachable!()
    };
    pair.peers[0].push_net_events(&[NetTransportEvent::PacketReceived {
        from_peer: 1,
        addr: "127.0.0.2:7001".into(),
        bytes: corrupt.to_bytes(),
    }]);
    for _ in 0..20 {
        pair.idle();
        if pair.peers[0].screen() == "online" {
            break;
        }
    }
    assert_eq!(pair.peers[0].screen(), "online");
    assert_eq!(
        pair.peers[0].notice(),
        Some(format!("DESYNC DETECTED AT FRAME {frame}").as_str())
    );
    assert!(pair.peers[0].fight().is_none());
    let log = pair.peers[0].take_log();
    assert!(log
        .iter()
        .any(|line| line.contains(&format!("local {local:016x}, remote {remote:016x}"))));
    assert!(
        log.iter()
            .any(|line| line.contains("desync snapshot FightSnapshot")),
        "the full snapshot remains available in the diagnostic log after the fight closes"
    );
}

#[test]
fn changing_a_local_look_online_keeps_the_clients_own_configuration_for_persistence() {
    let mut pair = Pair::fighting();
    assert_eq!(
        pair.peers[1].match_config().difficulty,
        2,
        "active online rules belong to the host"
    );
    pair.pulse(1, MenuInput::Start);
    pair.pulse(1, MenuInput::Down);
    pair.pulse(1, MenuInput::Down);
    let changed = pair.pulse(1, MenuInput::Confirm);
    assert!(
        changed[1].settings_changed,
        "the local Look action reaches the host's persistence boundary"
    );
    assert_eq!(pair.peers[1].match_config().difficulty, 2);
    let mut preferred = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    preferred.difficulty = 4;
    assert_eq!(*pair.peers[1].local_match_config(), preferred);
    let path =
        std::env::temp_dir().join(format!("sf1-np5-local-config-{}.toml", std::process::id()));
    pair.peers[1]
        .local_match_config()
        .save(&path)
        .expect("local rules save");
    let restored = MatchConfig::load(&path).expect("saved local rules round-trip");
    std::fs::remove_file(path).expect("the temporary persistence fixture is removed");
    assert_eq!(
        restored, preferred,
        "a cosmetic change cannot persist the host's synced rules over offline preferences"
    );
}
