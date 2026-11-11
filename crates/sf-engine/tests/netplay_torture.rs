//! NP8: native rollback under delayed, missing, repeated and reordered packets.

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::Arc;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::DataBank;
use sf_engine::dramatic::{DramaticRules, LifebarMode};
use sf_engine::era::Revision;
use sf_engine::fight_reader::FightReader;
use sf_engine::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup, FightSnapshot};
use sf_engine::front::session::PlayerInputs;
use sf_engine::netplay::bus::{PacketFault, VirtualNetBus, VirtualNetConfig};
use sf_engine::netplay::protocol::tag_from_str;
use sf_engine::netplay::{
    encode_inputs, Link, NetPacket, NetTransportCommand as Command, NetTransportEvent as Event,
    RollbackEngine, RollbackEvent, RoomAnnouncement, RoomKind,
};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::{load_assets, Assets};

fn banked_fight(dramatic: Option<(LifebarMode, bool)>) -> (Assets, Fight) {
    let assets = load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("NP8 requires the verified local ROM set");
    let pack = ArtPack::open(Path::new("../../tools/reference/local/roms/sf1-port-art"))
        .expect("NP8 requires the approved current art pack");
    let mut recipes = vec![
        include_str!("../../../art/recipes/ryu.json"),
        include_str!("../../../art/recipes/ken.json"),
    ];
    let mut reviews = vec![
        include_str!("../../../art/review/ryu.json"),
        include_str!("../../../art/review/ken.json"),
    ];
    if dramatic.is_some() {
        recipes.push(include_str!("../../../art/recipes/sagat.json"));
        reviews.push(include_str!("../../../art/review/sagat.json"));
    }
    let art = load_art_from(
        &assets,
        &ArtSources {
            recipes: &recipes,
            reviews: Some(&reviews),
            pack: Ok(&pack),
        },
    )
    .expect("the fighters' current approved poses load");
    let bank = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
    let assets = assets.with_sprite_bank(&art.bank);
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = TournamentFighter::Ken;
    if let Some((lifebar, friendly_fire)) = dramatic {
        config.roster.p2 = TournamentFighter::Sagat;
        config.dramatic = Some(DramaticRules {
            co_op: true,
            ally2: TournamentFighter::Ken,
            stage: TournamentFighter::Sagat,
            lifebar,
            friendly_fire,
            styles: [Revision::Sf1Plus; 3],
            rivalry: Some(0),
        });
    }
    let setup = FightSetup {
        kind: if dramatic.is_some() {
            FightKind::Dramatic { co_op: true }
        } else {
            FightKind::Local
        },
        players: if dramatic.is_some() {
            Players::One
        } else {
            Players::Two
        },
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let mut fight = Fight::start_banked(&assets.maincpu, setup, Some(0), None, Some(bank))
        .expect("the native cabinet driver starts the banked Local fight");
    for _ in 0..1_500 {
        if flow_position(fight.machine()) == 0x3eafa && !fight.driving() {
            if dramatic.is_some() {
                return (assets, fight);
            }
            for _ in 0..240 {
                fight.step(
                    &assets,
                    PlayerInputs::default(),
                    PlayerInputs::default(),
                    false,
                );
            }
            return (assets, fight);
        }
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
    }
    panic!("the real fight never handed control to both players");
}

fn pads(frame: u32) -> [PlayerInputs; 2] {
    let mut p1 = PlayerInputs::default();
    let mut p2 = PlayerInputs::default();
    p1.directions[0] = frame % 120 < 25;
    p1.directions[2] = frame % 120 >= 85;
    p1.buttons[0] = frame % 16 == 0;
    p1.buttons[3] = frame % 70 == 40;
    p2.directions[1] = frame % 100 < 30;
    p2.directions[2] = frame % 100 >= 80;
    p2.buttons[0] = frame % 18 == 4;
    p2.buttons[5] = frame % 80 == 50;
    if (80..140).contains(&frame) {
        // Separate through actual walking, then let existing reactions end.
        p1 = PlayerInputs::default();
        p2 = PlayerInputs::default();
        p1.directions[1] = frame < 120;
        p2.directions[0] = frame < 120;
    } else if (140..240).contains(&frame) {
        p1 = PlayerInputs::default();
        p2 = PlayerInputs::default();
        let motion = frame - 140;
        p1.directions[2] = motion <= 4;
        p1.directions[0] = (3..=8).contains(&motion);
        p1.buttons[0] = motion == 8;
        p2.directions[0] = (180..=220).contains(&frame);
    }
    [p1, p2]
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    Fighting,
    RoundTransition,
}

impl Scenario {
    fn pads(self, frame: u32) -> [PlayerInputs; 2] {
        if self == Self::Fighting {
            return pads(frame);
        }
        let mut inputs = [PlayerInputs::default(); 2];
        if frame == 0 {
            inputs[0].directions[0] = true;
            inputs[1].directions[1] = true;
        }
        // The native next-round wipe, ROUND2 and FIGHT! have completed by
        // this point. Throw a real projectile in the newly initialized round.
        if frame >= 420 {
            let motion = frame - 420;
            inputs[0].directions[2] = motion <= 4;
            inputs[0].directions[0] = (3..=8).contains(&motion);
            inputs[0].buttons[0] = motion == 8;
            inputs[1].directions[0] = (40..=80).contains(&motion);
        }
        inputs
    }
}

fn center_x(fight: &Fight, player: u32) -> i32 {
    let work = fight.machine().work_ram();
    let at = (player + 0x10 - 0xff8000) as usize;
    let entity = u16::from_be_bytes([work[at], work[at + 1]]) as usize;
    let objects = &fight.machine().state.objectram;
    let x = u16::from_be_bytes([objects[entity + 6], objects[entity + 7]]) as i32;
    let cell = if i16::from_be_bytes([objects[entity + 2], objects[entity + 3]]) >= 0x400 {
        32
    } else {
        16
    };
    x + (i32::from(objects[entity + 0xd]) + 1) * cell / 2
}

fn round_counter(fight: &Fight) -> u8 {
    // Native game-flow round index, documented in reference/round-transition.
    fight.machine().work_ram()[0x400a]
}

fn next_round_boundary(initial: &Fight, assets: &Assets) -> Fight {
    let mut fight = initial.clone();
    for _ in 0..200 {
        if (center_x(&fight, P1_STRUCT) - center_x(&fight, P2_STRUCT)).abs() <= 40 {
            break;
        }
        let mut walk = PlayerInputs::default();
        walk.directions[1] = true;
        fight.step(assets, PlayerInputs::default(), walk, false);
    }
    for _ in 0..16 {
        fight.step(
            assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
    }
    assert!((center_x(&fight, P1_STRUCT) - center_x(&fight, P2_STRUCT)).abs() <= 40);
    assert_eq!(round_counter(&fight), 0);

    // Public snapshot setup seeds only the defender's health. The contact,
    // KO, victory, result and next-round transition are the real native code.
    let mut saved = fight.save_netplay_snapshot(0);
    let health = (P2_STRUCT + 6 - 0xff8000) as usize;
    saved.machine.work[health] = 1;
    saved.machine.work[health + 1] = 0;
    saved.machine.toggle_state.era.fighters[1].sub_hp = sf_engine::era_fight::SUB_HP_PER_POINT;
    saved.read.fighters[1].health = 1;
    saved.reader = FightReader::new();
    fight.restore_netplay_snapshot(&saved);
    fight.step(
        assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
    assert_eq!(fight.fight_read().fighters[1].health, 1);

    let mut history = VecDeque::new();
    let mut saw_ko = false;
    for frame in 0..2_000 {
        history.push_back(fight.save_netplay_snapshot(0));
        if history.len() > 8 {
            history.pop_front();
        }
        let mut jab = PlayerInputs::default();
        jab.buttons[0] = frame == 0;
        fight.step(assets, jab, PlayerInputs::default(), false);
        saw_ko |= fight.fight_read().fighters[1].knocked_out();
        if round_counter(&fight) == 1 {
            assert!(saw_ko, "the next round follows an actual native knockout");
            assert_eq!(history.len(), 8);
            fight.restore_netplay_snapshot(history.front().unwrap());
            assert_eq!(round_counter(&fight), 0);
            assert!(!fight.is_over(), "one round win does not end this match");
            return fight;
        }
    }
    panic!("the native first KO never reached the next-round flow");
}

fn announcement() -> RoomAnnouncement {
    RoomAnnouncement {
        host: tag_from_str("TORTURE").unwrap(),
        kind: RoomKind::Versus,
        link: Link::Ethernet,
        wired_only: false,
        public: false,
        preset: RulesetPreset::DefinitiveOverhaul,
        in_match: true,
        max_rollback: 8,
        spectators: 0,
        config_hash: 0,
        bank_hash: 1,
        art_pack_hash: 2,
        protocol_hash: sf_engine::version::protocol_version_hash(),
    }
}

#[derive(Default)]
struct Traffic {
    sent: BTreeMap<(u8, Vec<u8>), usize>,
    received: BTreeMap<(u8, Vec<u8>), usize>,
    highest: [u32; 2],
    reordered: bool,
    corrected_flying_projectile: bool,
}

fn flying_projectile(fight: &Fight) -> bool {
    let objects = &fight.machine().state.objectram;
    // The native projectile slot, with its collision box active (+0x0e).
    // The later explosion effect alone does not satisfy this observation.
    objects[0x108] != 0 && objects[0x10e] & 0x80 != 0
}

fn send_window(net: &RollbackEngine, from: u8, bus: &mut VirtualNetBus, traffic: &mut Traffic) {
    let Some(last) = net.latest_local_frame() else {
        return;
    };
    let start = last.saturating_sub(8);
    let count = (last - start + 1) as u8;
    let mut bits = [0; 9];
    for (offset, bits) in bits[..usize::from(count)].iter_mut().enumerate() {
        *bits = net.local_input(start + offset as u32).unwrap();
    }
    let (checksum_frame, checksum) = net.latest_confirmed_checksum().unwrap_or((u32::MAX, 0));
    let packet = NetPacket::Input {
        // The wire tip is the first unsimulated frame. A stalled step can
        // already have sampled its input four frames ahead of this boundary.
        current_frame: net.next_frame(),
        start_frame: start,
        count,
        bits,
        ack_frame: u32::try_from(net.confirmed_frame()).unwrap_or(u32::MAX),
        frame_advantage: net.frame_advantage(),
        checksum_frame,
        checksum,
    };
    let bytes = packet.to_bytes();
    assert_eq!(
        bytes.len(),
        50,
        "the torture test uses the actual Input wire format"
    );
    assert_eq!(NetPacket::from_bytes(&bytes), Some(packet));
    *traffic.sent.entry((from, bytes.clone())).or_default() += 1;
    bus.submit(
        from,
        &[Command::SendPacket {
            to_peer: 1 - from,
            bytes,
        }],
    );
}

fn receive(
    net: &mut RollbackEngine,
    fight: &mut Fight,
    assets: &Assets,
    peer: u8,
    bus: &mut VirtualNetBus,
    traffic: &mut Traffic,
) {
    for event in bus.drain_events(peer) {
        match event {
            Event::PacketReceived {
                from_peer, bytes, ..
            } => {
                assert_eq!(from_peer, 1 - peer);
                *traffic
                    .received
                    .entry((from_peer, bytes.clone()))
                    .or_default() += 1;
                let NetPacket::Input {
                    current_frame,
                    start_frame,
                    count,
                    bits,
                    frame_advantage,
                    checksum_frame,
                    checksum,
                    ..
                } = NetPacket::from_bytes(&bytes).unwrap()
                else {
                    panic!("only real Input packets are sent")
                };
                traffic.reordered |= current_frame < traffic.highest[usize::from(from_peer)];
                traffic.highest[usize::from(from_peer)] =
                    current_frame.max(traffic.highest[usize::from(from_peer)]);
                let before = flying_projectile(fight).then(|| fight.state_checksum());
                net.apply_remote_inputs(fight, assets, start_frame, &bits[..usize::from(count)]);
                if let Some(before) = before {
                    traffic.corrected_flying_projectile |=
                        net.last_rollback_frames() > 0 && fight.state_checksum() != before;
                }
                net.set_remote_advantage(frame_advantage);
                if checksum_frame != u32::MAX {
                    net.receive_checksum(checksum_frame, checksum);
                }
            }
            Event::TransportError { message } => {
                panic!("virtual transport rejected traffic: {message}")
            }
            _ => {}
        }
    }
}

fn check_confirmation(
    net: &RollbackEngine,
    fight: &Fight,
    expected: &[(FightSnapshot, u64)],
    side: usize,
) {
    if let Some((frame, checksum)) = net.latest_confirmed_checksum() {
        let snapshot = net
            .confirmed_snapshot(fight)
            .expect("the confirmed native state is retained");
        let (want, hash) = &expected[frame as usize];
        assert!(
            snapshot.machine == want.machine,
            "peer{side}, confirmed frame{frame}: native RAM, registers, scheduler and inputs agree"
        );
        assert_eq!(
            snapshot.read, want.read,
            "peer{side}, frame{frame}: actual hit/guard reads agree"
        );
        assert_eq!(
            snapshot.over, want.over,
            "peer{side}, frame{frame}: native match flow agrees"
        );
        assert!(
            snapshot.round_start == want.round_start,
            "peer{side}, frame{frame}: the native round-start snapshot agrees"
        );
        assert_eq!(
            checksum, *hash,
            "peer{side}, frame{frame}: canonical authoritative state agrees"
        );
    }
}

fn delayed_pads(frame: u32, delay: u8, scenario: Scenario) -> [PlayerInputs; 2] {
    frame
        .checked_sub(u32::from(delay))
        .map_or([PlayerInputs::default(); 2], |sample| scenario.pads(sample))
}

fn adversarial_match(delay: u8, scenario: Scenario) {
    const FRAMES: u32 = 600;
    let (assets, mut initial) = banked_fight(None);
    if scenario == Scenario::RoundTransition {
        initial = next_round_boundary(&initial, &assets);
    }
    let mut reference = initial.clone();
    let mut expected = Vec::new();
    let mut expected_events = Vec::new();
    let mut saw_flying_projectile = false;
    for frame in 0..FRAMES {
        let [p1, p2] = delayed_pads(frame, delay, scenario);
        if let Some(event) = reference.step(&assets, p1, p2, false).event {
            expected_events.push((frame, event));
        }
        saw_flying_projectile |= flying_projectile(&reference);
        expected.push((
            reference.save_netplay_snapshot(frame + 1),
            reference.state_checksum(),
        ));
    }
    assert!(
        saw_flying_projectile,
        "the native quarter-circle really spawns a projectile"
    );
    if scenario == Scenario::RoundTransition {
        assert!(
            expected
                .iter()
                .take(8)
                .any(|(saved, _)| saved.machine.work[0x400a] == 1),
            "the actual next-round transition falls within the eight-frame misprediction window"
        );
        assert_eq!(round_counter(&reference), 1);
        assert!(
            reference.fight_read().fighters[1].health > 1,
            "the original game actually reinitialized the knocked-out fighter for round2"
        );
        assert!(!reference.is_over());
    }

    let mut fights = [initial.clone(), initial];
    let mut nets = [
        RollbackEngine::new(0, delay, 8),
        RollbackEngine::new(1, delay, 8),
    ];
    let mut bus = VirtualNetBus::new(VirtualNetConfig {
        latency_ms: 60,
        jitter_ms: 30,
        drop_percent: 15,
        duplicate_percent: 12,
        reorder_ms: 35,
        seed: 0x1987_2026,
    });
    bus.submit(
        0,
        &[Command::BindHost {
            port: 7000,
            room: announcement(),
        }],
    );
    bus.submit(
        1,
        &[Command::ConnectPeer {
            addr: "127.0.0.1:7000".into(),
        }],
    );
    // The first frame arrives after exactly eight simulations. The following
    // seven datagrams in each direction are lost; a redundant window must heal
    // them. Remaining traffic uses the percentage-based adversarial network.
    let mut faults = vec![
        PacketFault {
            delay_ms: 133,
            drop: false,
            copies: 2
        };
        2
    ];
    faults.extend(
        [PacketFault {
            delay_ms: 0,
            drop: true,
            copies: 0,
        }; 14],
    );
    bus.set_fault_script(faults);
    let mut traffic = Traffic::default();
    let mut deepest = [0; 2];
    let mut waiting_ticks = 0;
    for tick in 0..3_000 {
        for side in 0..2 {
            receive(
                &mut nets[side],
                &mut fights[side],
                &assets,
                side as u8,
                &mut bus,
                &mut traffic,
            );
            deepest[side] = deepest[side].max(nets[side].last_rollback_frames());
            if nets[side].next_frame() < FRAMES {
                let local = scenario.pads(nets[side].next_frame())[side];
                nets[side].step(&mut fights[side], &assets, local);
                waiting_ticks += usize::from(nets[side].is_waiting());
            }
            check_confirmation(&nets[side], &fights[side], &expected, side);
            for event in nets[side].drain_events() {
                assert!(
                    matches!(event, RollbackEvent::WaitingForPeer),
                    "unexpected rollback event: {event:?}"
                );
            }
            send_window(&nets[side], side as u8, &mut bus, &mut traffic);
        }
        bus.step_ms([16, 17, 17][tick % 3]);
        if nets
            .iter()
            .all(|net| net.confirmed_frame() == i64::from(FRAMES - 1))
        {
            break;
        }
    }
    // Deliver the finite remaining datagrams before measuring actual loss and
    // duplication, so packets still in flight cannot masquerade as lost ones.
    bus.step_ms(1_000);
    for side in 0..2 {
        receive(
            &mut nets[side],
            &mut fights[side],
            &assets,
            side as u8,
            &mut bus,
            &mut traffic,
        );
        check_confirmation(&nets[side], &fights[side], &expected, side);
        assert_eq!(nets[side].next_frame(), FRAMES);
        assert_eq!(nets[side].confirmed_frame(), i64::from(FRAMES - 1));
        assert_eq!(nets[side].confirmed_events(), expected_events);
        let inputs: Vec<_> = (0..FRAMES)
            .map(|frame| {
                let [p1, p2] = delayed_pads(frame, delay, scenario);
                (encode_inputs(p1), encode_inputs(p2))
            })
            .collect();
        assert_eq!(nets[side].confirmed_inputs(), inputs);
        assert!(fights[side].machine().save_snapshot() == reference.machine().save_snapshot());
        assert!(fights[side].machine().unrecovered().is_empty());
        assert!(!nets[side].is_disconnected());
        for event in nets[side].drain_events() {
            assert!(
                matches!(event, RollbackEvent::WaitingForPeer),
                "final datagrams reported {event:?}"
            );
        }
    }
    if delay == 0 {
        assert_eq!(
            deepest,
            [8, 8],
            "both player routes actually restore the full eight-frame window"
        );
    } else {
        assert!(
            deepest.iter().all(|depth| (4..=8).contains(depth)),
            "delayed inputs still cause real native corrections: {deepest:?}"
        );
    }
    if delay == 0 {
        assert!(
            waiting_ticks > 0,
            "the zero-delay bounded window visibly waits and then recovers"
        );
        if scenario == Scenario::Fighting {
            assert!(
                traffic.corrected_flying_projectile,
                "a late datagram actually corrects native state while a projectile is in flight"
            );
        }
    }
    assert!(
        traffic.reordered,
        "the deterministic network actually reorders Input packets"
    );
    assert!(
        traffic.sent.iter().any(|(packet, sent)| traffic
            .received
            .get(packet)
            .copied()
            .unwrap_or(0)
            < *sent),
        "real datagrams were lost rather than merely configured for loss"
    );
    assert!(
        traffic
            .received
            .iter()
            .any(|(packet, received)| *received > traffic.sent[packet]),
        "real datagrams were duplicated rather than merely configured for duplication"
    );
}

#[test]
fn six_hundred_real_frames_converge_under_120ms_rtt_jitter_loss_duplicates_and_reordering() {
    adversarial_match(0, Scenario::Fighting);
}

#[test]
fn four_frame_input_delay_preserves_neutral_bootstrap_and_recovers_adversarial_windows() {
    adversarial_match(4, Scenario::Fighting);
}

#[test]
fn eight_frame_network_correction_crosses_the_real_next_round_transition_and_reset() {
    adversarial_match(0, Scenario::RoundTransition);
}

fn dramatic_pads(frame: u32, fight: &Fight) -> [PlayerInputs; 2] {
    let boss = center_x(fight, CPU_STRUCT);
    [P1_STRUCT, P2_STRUCT].map(|player| {
        let mut pad = PlayerInputs::default();
        let distance = boss - center_x(fight, player);
        let phase = (frame + if player == P1_STRUCT { 0 } else { 7 }) % 36;
        pad.directions[usize::from(distance < 0)] = distance.abs() > 30;
        pad.directions[2] = distance.abs() <= 55 && (28..=30).contains(&phase);
        pad.buttons[2] = distance.abs() <= 65 && phase == 4;
        pad.buttons[3] = distance.abs() <= 38 && phase == 26;
        pad
    })
}

/// Rewind across a real damaging frame using its actual two pad inputs.
fn replay_dramatic_contact(
    mut before: Fight,
    after: &Fight,
    assets: &Assets,
    pads: [PlayerInputs; 2],
) -> bool {
    let Some(remote_side) = pads.iter().position(|pad| *pad != PlayerInputs::default()) else {
        return false;
    };
    let local_side = 1 - remote_side;
    let mut rollback = RollbackEngine::new(local_side, 0, 8);
    rollback.step(&mut before, assets, pads[local_side]);
    rollback.apply_remote_inputs(&mut before, assets, 0, &[encode_inputs(pads[remote_side])]);
    assert_eq!(rollback.last_rollback_frames(), 1);
    assert_eq!(rollback.confirmed_checksum(0), Some(after.state_checksum()));
    assert!(before.save_netplay_snapshot(1).machine == after.save_netplay_snapshot(1).machine);
    true
}

#[test]
fn native_dramatic_battle_confirms_all_300_frames_through_loss_in_both_lifebar_modes() {
    const FRAMES: u32 = 300;
    for (lifebar, friendly_fire) in [
        (LifebarMode::Shared, false),
        (LifebarMode::Independent, true),
    ] {
        let (assets, initial) = banked_fight(Some((lifebar, friendly_fire)));
        let mut reference = initial.clone();
        let mut previous_health = initial.machine().toggle_state().dramatic.health;
        let mut expected = Vec::new();
        let mut inputs = Vec::new();
        let mut replayed_damage = [false; 2];
        for frame in 0..FRAMES {
            let pads = dramatic_pads(frame, &reference);
            let [p1, p2] = pads;
            let before =
                (!replayed_damage.iter().all(|&replayed| replayed)).then(|| reference.clone());
            inputs.push(pads);
            let result = reference.step(&assets, p1, p2, false);
            assert!(
                result.event.is_none(),
                "native reference frame {frame}: {:?}",
                result.event
            );
            let health = reference.machine().toggle_state().dramatic.health;
            let damaged = [
                health[0] < previous_health[0] || health[2] < previous_health[2],
                health[1] < previous_health[1],
            ];
            if let Some(before) = before.filter(|_| {
                damaged
                    .iter()
                    .zip(replayed_damage)
                    .any(|(&hit, replayed)| hit && !replayed)
            }) {
                if replay_dramatic_contact(before, &reference, &assets, pads) {
                    for (replayed, hit) in replayed_damage.iter_mut().zip(damaged) {
                        *replayed |= hit;
                    }
                }
            }
            previous_health = health;
            if lifebar == LifebarMode::Shared {
                assert_eq!(health[0], health[2]);
            }
            expected.push(reference.state_checksum());
        }
        assert_eq!(
            replayed_damage,
            [true; 2],
            "rollbacks cover damage to both sides in {lifebar:?}; final health {:?}",
            reference.machine().toggle_state().dramatic.health
        );

        let mut bus = VirtualNetBus::new(VirtualNetConfig {
            latency_ms: 25,
            drop_percent: 10,
            seed: 0x287_352,
            ..VirtualNetConfig::default()
        });
        bus.submit(
            0,
            &[Command::BindHost {
                port: 7000,
                room: RoomAnnouncement {
                    kind: RoomKind::Dramatic,
                    ..announcement()
                },
            }],
        );
        bus.submit(
            1,
            &[Command::ConnectPeer {
                addr: "127.0.0.1:7000".into(),
            }],
        );
        let mut fights = [initial.clone(), initial];
        let mut nets = [RollbackEngine::new(0, 0, 8), RollbackEngine::new(1, 0, 8)];
        let mut traffic = Traffic::default();
        let mut corrected = [false; 2];
        for tick in 0..1_500 {
            for side in 0..2 {
                let before = fights[side].state_checksum();
                receive(
                    &mut nets[side],
                    &mut fights[side],
                    &assets,
                    side as u8,
                    &mut bus,
                    &mut traffic,
                );
                corrected[side] |= before != fights[side].state_checksum();
                if nets[side].next_frame() < FRAMES {
                    let pad = inputs[nets[side].next_frame() as usize][side];
                    nets[side].step(&mut fights[side], &assets, pad);
                }
                for event in nets[side].drain_events() {
                    assert!(
                        matches!(event, RollbackEvent::WaitingForPeer),
                        "{lifebar:?}: {event:?}"
                    );
                }
                send_window(&nets[side], side as u8, &mut bus, &mut traffic);
            }
            if nets
                .iter()
                .all(|net| net.confirmed_frame() == i64::from(FRAMES - 1))
            {
                break;
            }
            bus.step_ms([16, 17, 17][tick % 3]);
        }
        assert_eq!(corrected, [true; 2], "both peers roll back {lifebar:?}");
        bus.step_ms(200);
        for side in 0..2 {
            receive(
                &mut nets[side],
                &mut fights[side],
                &assets,
                side as u8,
                &mut bus,
                &mut traffic,
            );
        }
        assert!(traffic.sent.iter().any(|(packet, sent)| traffic
            .received
            .get(packet)
            .copied()
            .unwrap_or(0)
            < *sent));
        let reference_snapshot = reference.save_netplay_snapshot(FRAMES);
        for (side, net) in nets.iter().enumerate() {
            assert_eq!(net.confirmed_frame(), i64::from(FRAMES - 1));
            assert!(net.confirmed_events().is_empty());
            let confirmed = net.confirmed_snapshot(&fights[side]).unwrap();
            assert!(
                confirmed.machine == reference_snapshot.machine,
                "{lifebar:?} peer {side} native state"
            );
            for (frame, &checksum) in expected.iter().enumerate() {
                assert_eq!(
                    net.confirmed_checksum(frame as u32),
                    Some(checksum),
                    "{lifebar:?}, peer {side}, confirmed frame {frame}"
                );
            }
        }
    }
}
