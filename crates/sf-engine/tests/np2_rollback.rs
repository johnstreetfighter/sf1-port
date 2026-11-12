//! NP2: delayed inputs restore real native state, outputs and round flow.

use std::path::Path;
use std::sync::Arc;

use sf_engine::combo::throw::{DAMAGE, TECH_WINDOW, THROWN_MOVE, THROW_MOVE};
use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::DataBank;
use sf_engine::fight_reader::FightReader;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightEvent, FightKind, FightSetup};
use sf_engine::front::session::PlayerInputs;
use sf_engine::netplay::encode_inputs;
use sf_engine::netplay::rollback::{RollbackEngine, RollbackEvent};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_video::announcer::AnnouncerLine;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::{CalloutEvent, CalloutKind, Side};
use sf_video::romset::{load_assets, Assets};

fn assets() -> Assets {
    load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("NP2 requires the verified local ROM set; it must not silently skip")
}

fn setup() -> FightSetup {
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = TournamentFighter::Ken;
    FightSetup {
        kind: FightKind::Local,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    }
}

fn controlled_fight(assets: &Assets) -> Fight {
    let mut fight = Fight::start(&assets.maincpu, setup(), Some(0), None)
        .expect("the faithful driver reaches a Local fight");
    to_control(&mut fight, assets);
    fight
}

fn to_control(fight: &mut Fight, assets: &Assets) {
    for _ in 0..1_500 {
        if flow_position(fight.machine()) == 0x3eafa && !fight.driving() {
            for _ in 0..240 {
                fight.step(
                    assets,
                    PlayerInputs::default(),
                    PlayerInputs::default(),
                    false,
                );
            }
            return;
        }
        fight.step(
            assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
    }
    panic!("the real fight never handed control to the players");
}

fn center_x(fight: &Fight, player: u32) -> i32 {
    let work = fight.machine().work_ram();
    let at = (player + 0x10 - 0xff8000) as usize;
    let entity = u16::from_be_bytes([work[at], work[at + 1]]) as usize;
    let obj = &fight.machine().state.objectram;
    let x = u16::from_be_bytes([obj[entity + 6], obj[entity + 7]]) as i32;
    let cell = if i16::from_be_bytes([obj[entity + 2], obj[entity + 3]]) >= 0x400 {
        32
    } else {
        16
    };
    x + (i32::from(obj[entity + 0xd]) + 1) * cell / 2
}

fn close_with_actual_throw_art() -> (Assets, Fight) {
    let assets = assets();
    let pack = ArtPack::open(Path::new("../../assets/sf1-port-art"))
        .expect("NP2 real throws require the approved local art pack");
    let art = load_art_from(
        &assets,
        &ArtSources {
            recipes: &[
                include_str!("../../../art/recipes/ryu.json"),
                include_str!("../../../art/recipes/ken.json"),
            ],
            reviews: Some(&[
                include_str!("../../../art/review/ryu.json"),
                include_str!("../../../art/review/ken.json"),
            ]),
            pack: Ok(&pack),
        },
    )
    .expect("Ryu and Ken's approved poses load");
    assert!(art.catalogue("ryu").is_some() && art.catalogue("ken").is_some());
    let bank = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
    assert!(
        bank.throw_entry(TournamentFighter::Ryu).is_some()
            && bank.throw_entry(TournamentFighter::Ken).is_some()
    );
    let assets = assets.with_sprite_bank(&art.bank);
    let mut fight = Fight::start_banked(&assets.maincpu, setup(), Some(0), None, Some(bank))
        .expect("the real banked Local fight starts");
    to_control(&mut fight, &assets);
    for _ in 0..200 {
        if (center_x(&fight, P1_STRUCT) - center_x(&fight, P2_STRUCT)).abs() <= 40 {
            break;
        }
        let mut walk = PlayerInputs::default();
        walk.directions[1] = true;
        fight.step(&assets, PlayerInputs::default(), walk, false);
    }
    for _ in 0..16 {
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
    }
    assert!((center_x(&fight, P1_STRUCT) - center_x(&fight, P2_STRUCT)).abs() <= 48);
    (assets, fight)
}

#[test]
fn a_late_frame_zero_input_restores_the_real_fight() {
    let assets = assets();
    let mut fight = controlled_fight(&assets);
    let mut reference = fight.clone();
    let mut remote = PlayerInputs::default();
    remote.directions[1] = true;
    remote.buttons[0] = true;
    let expected = reference.step(&assets, PlayerInputs::default(), remote, false);
    let mut net = RollbackEngine::new(0, 0, 8);
    net.step(&mut fight, &assets, PlayerInputs::default())
        .expect("frame zero advances speculatively");
    net.apply_remote_inputs(&mut fight, &assets, 0, &[encode_inputs(remote)]);
    assert!(
        fight.machine().save_snapshot() == reference.machine().save_snapshot(),
        "correcting frame zero restores native machine state"
    );
    assert_eq!(fight.state_checksum(), reference.state_checksum());
    assert_eq!(fight.fight_read(), reference.fight_read());
    assert_eq!(
        net.next_frame(),
        1,
        "resimulation does not advance the live tip"
    );
    assert_eq!(expected.event, None);
}

fn stream(frame: u32) -> (PlayerInputs, PlayerInputs) {
    let mut p1 = PlayerInputs::default();
    let mut p2 = PlayerInputs::default();
    p1.directions[0] = frame % 100 < 20;
    p1.directions[2] = frame % 100 >= 75;
    p1.buttons[0] = frame % 30 == 15;
    p1.buttons[3] = frame % 90 == 80;
    p2.directions[1] = frame % 80 < 14;
    p2.directions[2] = frame % 80 >= 60;
    p2.buttons[0] = frame % 24 == 3;
    (p1, p2)
}

#[test]
fn three_hundred_real_frames_match_at_every_confirmation_with_one_to_eight_frames_late() {
    let assets = assets();
    let initial = controlled_fight(&assets);
    let mut reference = initial.clone();
    let mut expected = Vec::new();
    let mut events = Vec::new();
    for frame in 0..300 {
        let (p1, p2) = stream(frame);
        let output = reference.step(&assets, p1, p2, false);
        if let Some(event) = output.event {
            events.push((frame, event));
        }
        expected.push((
            reference.save_netplay_snapshot(frame + 1),
            reference.state_checksum(),
        ));
    }
    for late in 1..=8u32 {
        // Both player-side routes are exercised without doubling the loop.
        let side = late as usize % 2;
        let mut fight = initial.clone();
        let mut net = RollbackEngine::new(side, 0, 8);
        let mut deepest = 0;
        for tick in 0..300 + late {
            if tick >= late {
                let remote_frame = tick - late;
                let (p1, p2) = stream(remote_frame);
                let remote = if side == 0 { p2 } else { p1 };
                net.apply_remote_inputs(
                    &mut fight,
                    &assets,
                    remote_frame,
                    &[encode_inputs(remote)],
                );
                deepest = deepest.max(net.last_rollback_frames());
            }
            if net.next_frame() < 300 {
                let (p1, p2) = stream(net.next_frame());
                net.step(&mut fight, &assets, if side == 0 { p1 } else { p2 });
            }
            if let Some((frame, checksum)) = net.latest_confirmed_checksum() {
                let snapshot = net
                    .confirmed_snapshot(&fight)
                    .expect("confirmed native state remains available");
                let (want, hash) = &expected[frame as usize];
                assert!(
                    snapshot.machine == want.machine,
                    "{late} frames late, confirmed {frame}: native state agrees"
                );
                assert_eq!(
                    snapshot.read, want.read,
                    "{late} frames late, confirmed {frame}: public fight events agree"
                );
                assert_eq!(
                    checksum, *hash,
                    "{late} frames late, confirmed {frame}: canonical checksum agrees"
                );
            }
        }
        assert_eq!(net.next_frame(), 300);
        assert_eq!(net.confirmed_frame(), 299);
        assert_eq!(
            deepest, late as u8,
            "the late frame-zero correction actually exercises this depth"
        );
        let inputs: Vec<_> = (0..300)
            .map(|f| {
                let (p1, p2) = stream(f);
                (encode_inputs(p1), encode_inputs(p2))
            })
            .collect();
        assert_eq!(net.confirmed_inputs(), inputs);
        assert_eq!(net.confirmed_events(), events);
        assert!(fight.machine().save_snapshot() == reference.machine().save_snapshot());
        assert!(fight.machine().unrecovered().is_empty());
    }
}

#[test]
fn a_delayed_confirmed_checksum_detects_native_divergence_after_snapshot_ring_wraps() {
    let assets = assets();
    let mut fight = controlled_fight(&assets);
    let mut net = RollbackEngine::new(0, 0, 8);
    let mut first = None;
    for frame in 0..24 {
        net.apply_remote_inputs(&mut fight, &assets, frame, &[0]);
        net.step(&mut fight, &assets, PlayerInputs::default())
            .unwrap();
        if frame == 0 {
            let local = fight.state_checksum();
            let mut other_peer = fight.clone();
            let mut changed = other_peer.save_netplay_snapshot(1);
            changed.machine.work[0x200] ^= 1;
            other_peer.restore_netplay_snapshot(&changed);
            let remote = other_peer.state_checksum();
            assert_ne!(local, remote, "the other peer's native RAM really diverged");
            first = Some((local, remote));
        }
    }
    let (local, remote) = first.unwrap();
    net.receive_checksum(0, remote);
    assert_eq!(
        net.drain_events(),
        vec![RollbackEvent::Desync {
            frame: 0,
            local,
            remote
        }],
        "a late checksum still compares after the nine snapshots were reused"
    );
    net.receive_checksum(0, remote);
    assert!(
        net.drain_events().is_empty(),
        "the same mismatch is reported only once"
    );
}

#[test]
fn repeated_corrections_preserve_original_audio_multiplicity_without_replaying_it() {
    let assets = assets();
    let mut fight = controlled_fight(&assets);
    let mut seed = fight.save_netplay_snapshot(0);
    seed.sounds = [0x0c, 8, 9].into();
    fight.restore_netplay_snapshot(&seed);
    let mut net = RollbackEngine::new(0, 0, 8);
    let first = net
        .step(&mut fight, &assets, PlayerInputs::default())
        .unwrap();
    assert_eq!(first.sound_latch, 0x0c);
    fight.push_callouts(&[
        CalloutEvent::new(Side::P1, CalloutKind::Counter),
        CalloutEvent::new(Side::P2, CalloutKind::Counter),
    ]);
    let second = net
        .step(&mut fight, &assets, PlayerInputs::default())
        .unwrap();
    assert_eq!(second.sound_latch, 8);
    assert_eq!(
        second.announcements,
        [AnnouncerLine::Counter, AnnouncerLine::Counter],
        "two original simultaneous calls are not rollback duplicates"
    );
    let third = net
        .step(&mut fight, &assets, PlayerInputs::default())
        .unwrap();
    assert_eq!(third.sound_latch, 9);
    for _ in 0..2 {
        net.step(&mut fight, &assets, PlayerInputs::default())
            .unwrap();
    }
    let mut changed = PlayerInputs::default();
    changed.directions[1] = true;
    // Frame zero stays unknown, so frame one remains correctable A/B/A.
    for bits in [encode_inputs(changed), 0, encode_inputs(changed)] {
        net.apply_remote_inputs(&mut fight, &assets, 1, &[bits]);
    }
    assert_eq!(net.confirmed_frame(), -1);
    let after = net
        .step(&mut fight, &assets, PlayerInputs::default())
        .unwrap();
    assert_eq!(
        after.sound_latch, 0xff,
        "already heard latches are not replayed"
    );
    assert!(
        after.announcements.is_empty(),
        "already heard call occurrences are not replayed"
    );
}

#[test]
fn delay_pacing_window_stalls_and_input_validation_preserve_frame_boundaries() {
    let assets = assets();
    let initial = controlled_fight(&assets);
    for delay in 0..=4u32 {
        let mut fight = initial.clone();
        let mut reference = initial.clone();
        let mut net = RollbackEngine::new(0, delay as u8, 8);
        for frame in 0..40 {
            let (local, remote) = stream(frame);
            net.apply_remote_inputs(&mut fight, &assets, frame + delay, &[encode_inputs(remote)]);
            net.step(&mut fight, &assets, local)
                .expect("known delayed streams advance");
            let (p1, p2) = frame.checked_sub(delay).map(stream).unwrap_or_default();
            reference.step(&assets, p1, p2, false);
            assert!(
                fight.machine().save_snapshot() == reference.machine().save_snapshot(),
                "{delay}F delay, frame {frame}: leading neutral frames and scheduling agree"
            );
            assert_eq!(net.confirmed_frame(), i64::from(frame));
        }
        for frame in 0..delay {
            assert_eq!(net.local_input(frame), Some(0));
        }
        assert_eq!(net.local_input(delay), Some(encode_inputs(stream(0).0)));
    }

    let mut fight = initial.clone();
    let mut paced = RollbackEngine::new(0, 0, 8);
    paced.apply_remote_inputs(&mut fight, &assets, 0, &[0]);
    paced.set_remote_advantage(-4);
    let native = fight.machine().save_snapshot();
    let first = stream(0).0;
    assert!(
        paced.step(&mut fight, &assets, first).is_none(),
        "an advantage rift holds one display tick"
    );
    assert_eq!(paced.next_frame(), 0);
    assert!(fight.machine().save_snapshot() == native);
    assert!(
        paced
            .step(&mut fight, &assets, PlayerInputs::default())
            .is_some(),
        "the next tick advances rather than holding forever"
    );
    assert_eq!(
        paced.local_input(0),
        Some(encode_inputs(first)),
        "a hold cannot rewrite already sampled input"
    );
    assert!(!paced.is_waiting());

    let mut fight = initial.clone();
    let mut waiting = RollbackEngine::new(0, 0, 2);
    for _ in 0..2 {
        waiting
            .step(&mut fight, &assets, PlayerInputs::default())
            .unwrap();
    }
    let native = fight.machine().save_snapshot();
    assert!(waiting
        .step(&mut fight, &assets, PlayerInputs::default())
        .is_none());
    assert_eq!(waiting.drain_events(), [RollbackEvent::WaitingForPeer]);
    assert!(fight.machine().save_snapshot() == native);
    waiting.apply_remote_inputs(&mut fight, &assets, 0, &[0, 0]);
    assert!(waiting
        .step(&mut fight, &assets, PlayerInputs::default())
        .is_some());
    assert!(!waiting.is_waiting());
    assert_eq!(waiting.next_frame(), 3);

    let mut fight = initial.clone();
    let mut timeout = RollbackEngine::new(0, 0, 1);
    timeout
        .step(&mut fight, &assets, PlayerInputs::default())
        .unwrap();
    for _ in 0..299 {
        assert!(timeout
            .step(&mut fight, &assets, PlayerInputs::default())
            .is_none());
    }
    assert!(
        !timeout.is_disconnected(),
        "299 stalled ticks are not the timeout"
    );
    assert!(timeout
        .step(&mut fight, &assets, PlayerInputs::default())
        .is_none());
    assert!(timeout.is_disconnected());
    assert_eq!(timeout.next_frame(), 1);
    assert_eq!(
        timeout.drain_events(),
        [
            RollbackEvent::WaitingForPeer,
            RollbackEvent::PeerDisconnected
        ]
    );

    let mut fight = initial.clone();
    let mut validated = RollbackEngine::new(0, 0, 8);
    for (start, bits) in [
        (u32::MAX, vec![0]),
        (16, vec![0]),
        (0, vec![0x0400]),
        (0, vec![0; 10]),
    ] {
        validated.apply_remote_inputs(&mut fight, &assets, start, &bits);
    }
    validated
        .step(&mut fight, &assets, PlayerInputs::default())
        .unwrap();
    assert_eq!(
        validated.confirmed_frame(),
        -1,
        "malformed, oversized and far-future windows have no effect"
    );
    validated.apply_remote_inputs(&mut fight, &assets, 0, &[0]);
    let native = fight.machine().save_snapshot();
    validated.apply_remote_inputs(&mut fight, &assets, 0, &[1]);
    assert!(
        fight.machine().save_snapshot() == native,
        "confirmed input cannot be rewritten by stale packets"
    );
    assert_eq!(validated.confirmed_inputs(), [(0, 0)]);
    validated.apply_remote_inputs(&mut fight, &assets, 0, &[0x8000]);
    assert!(
        validated.is_disconnected(),
        "a valid active-match quit also works after its source frame was confirmed"
    );
    assert_eq!(
        validated.drain_events(),
        [RollbackEvent::PeerForfeit { frame: 0 }]
    );

    let mut fight = initial;
    let mut delayed_quit = RollbackEngine::new(0, 4, 8);
    delayed_quit.apply_remote_inputs(&mut fight, &assets, 0, &[0x8000]);
    assert!(
        delayed_quit.is_disconnected(),
        "forfeit is transport control even during neutral delay bootstrap"
    );
    assert_eq!(
        delayed_quit.drain_events(),
        [RollbackEvent::PeerForfeit { frame: 0 }]
    );
}

struct Replayed {
    fight: Fight,
    max_combo: u8,
    saw_throw: bool,
    cues: Vec<u8>,
    reference_cues: Vec<u8>,
    announcements: Vec<AnnouncerLine>,
    reference_announcements: Vec<AnnouncerLine>,
}

fn replay_late_attacker(
    initial: &Fight,
    assets: &Assets,
    inputs: &[(PlayerInputs, PlayerInputs)],
) -> Replayed {
    let mut reference = initial.clone();
    let mut expected = Vec::new();
    let mut reference_cues = Vec::new();
    let mut reference_announcements = Vec::new();
    let mut max_combo = 0;
    let mut saw_throw = false;
    for (frame, &(p1, p2)) in inputs.iter().enumerate() {
        let output = reference.step(assets, p1, p2, false);
        if output.sound_latch != 0xff {
            reference_cues.push(output.sound_latch);
        }
        reference_announcements.extend(output.announcements);
        max_combo = max_combo.max(reference.machine().toggle_state().combo.hits[1]);
        saw_throw |= reference.fight_read().fighters[0].move_id == THROW_MOVE
            && reference.fight_read().fighters[1].move_id == THROWN_MOVE;
        expected.push((
            reference.save_netplay_snapshot(frame as u32 + 1),
            reference.state_checksum(),
        ));
    }
    let mut fight = initial.clone();
    // Attacker P1 is remote; the confirmed reference must really execute its
    // mechanics after six-frame mispredictions, not merely receive idle pads.
    let mut net = RollbackEngine::new(1, 0, 8);
    let mut cues = Vec::new();
    let mut announcements = Vec::new();
    for tick in 0..inputs.len() + 6 {
        if tick >= 6 {
            let frame = tick - 6;
            net.apply_remote_inputs(
                &mut fight,
                assets,
                frame as u32,
                &[encode_inputs(inputs[frame].0)],
            );
        }
        if (net.next_frame() as usize) < inputs.len() {
            let local = inputs[net.next_frame() as usize].1;
            let output = net
                .step(&mut fight, assets, local)
                .expect("six late frames fit the eight-frame window");
            if output.sound_latch != 0xff {
                cues.push(output.sound_latch);
            }
            announcements.extend(output.announcements);
        }
        if let Some((frame, hash)) = net.latest_confirmed_checksum() {
            let got = net.confirmed_snapshot(&fight).unwrap();
            let (want, checksum) = &expected[frame as usize];
            assert!(
                got.machine == want.machine,
                "mechanic frame {frame}: independently observed native state agrees"
            );
            assert_eq!(
                got.read, want.read,
                "mechanic frame {frame}: public hits/blocks agree"
            );
            assert_eq!(hash, *checksum);
        }
    }
    assert_eq!(net.confirmed_inputs().len(), inputs.len());
    assert!(fight.machine().save_snapshot() == reference.machine().save_snapshot());
    assert!(fight.machine().unrecovered().is_empty());
    Replayed {
        fight,
        max_combo,
        saw_throw,
        cues,
        reference_cues,
        announcements,
        reference_announcements,
    }
}

#[test]
fn late_attacker_inputs_replay_real_combos_cancels_throws_and_last_window_techs() {
    let (assets, initial) = close_with_actual_throw_art();

    let mut jab = vec![(PlayerInputs::default(), PlayerInputs::default()); 60];
    jab[0].0.buttons[0] = true;
    let single = replay_late_attacker(&initial, &assets, &jab);
    assert!(
        !single.reference_cues.is_empty(),
        "the actual jab produces sound"
    );
    assert_eq!(
        single.cues, single.reference_cues,
        "newly discovered jab/hit cues are delivered once, in order"
    );
    assert_eq!(
        single
            .reference_announcements
            .iter()
            .filter(|&&line| line == AnnouncerLine::FirstAttack)
            .count(),
        1
    );
    assert_eq!(
        single
            .announcements
            .iter()
            .filter(|&&line| line == AnnouncerLine::FirstAttack)
            .count(),
        1,
        "the newly discovered first hit announces once"
    );

    let mut combo = vec![(PlayerInputs::default(), PlayerInputs::default()); 60];
    for frame in (0..30).step_by(2) {
        combo[frame].0.buttons[0] = true;
    }
    let combo = replay_late_attacker(&initial, &assets, &combo);
    assert!(
        combo.max_combo >= 2,
        "the real LP sequence produces a multi-hit combo"
    );

    let mut cancel = vec![(PlayerInputs::default(), PlayerInputs::default()); 60];
    cancel[0].0.directions[2] = true;
    cancel[0].0.buttons[3] = true;
    for frame in 1..=2 {
        cancel[frame].0.directions[2] = true;
    }
    for frame in 3..=4 {
        cancel[frame].0.directions[0] = true;
        cancel[frame].0.directions[2] = true;
    }
    for frame in 5..=8 {
        cancel[frame].0.directions[0] = true;
    }
    cancel[8].0.buttons[0] = true;
    let cancel = replay_late_attacker(&initial, &assets, &cancel);
    assert_eq!(
        cancel.fight.machine().toggle_state().cancel.count[0],
        1,
        "the crouching LK actually cancels into Hadoken"
    );

    let health = initial.fight_read().fighters[1].health;
    let mut grab = vec![(PlayerInputs::default(), PlayerInputs::default()); 110];
    grab[0].0.directions[0] = true;
    grab[0].0.buttons[2] = true;
    let thrown = replay_late_attacker(&initial, &assets, &grab);
    assert!(
        thrown.saw_throw,
        "actual catalogue throw/thrown moves execute"
    );
    assert_eq!(
        thrown.fight.fight_read().fighters[1].health,
        health - u16::from(DAMAGE)
    );

    grab[usize::from(TECH_WINDOW) - 1].1.directions[0] = true;
    grab[usize::from(TECH_WINDOW) - 1].1.buttons[5] = true;
    let teched = replay_late_attacker(&initial, &assets, &grab);
    assert!(teched.saw_throw);
    assert_eq!(
        teched.fight.fight_read().fighters[1].health,
        health,
        "last-window tech prevents real throw damage"
    );
    assert_eq!(
        teched.fight.machine().toggle_state().throw.tech_window[1],
        0
    );
    assert!((center_x(&teched.fight, P1_STRUCT) - center_x(&teched.fight, P2_STRUCT)).abs() > 48);
}

fn final_round_with_one_health(initial: &Fight, assets: &Assets) -> Fight {
    let mut fight = initial.clone();
    let mut saved = fight.save_netplay_snapshot(0);
    let p1 = (P1_STRUCT - 0xff8000) as usize;
    let p2 = (P2_STRUCT - 0xff8000) as usize;
    saved.machine.work[p1 + 8] = 1; // A real next KO wins the match.
    saved.machine.work[p2 + 6] = 1;
    saved.machine.work[p2 + 7] = 0;
    saved.machine.toggle_state.era.fighters[1].sub_hp = sf_engine::era_fight::SUB_HP_PER_POINT;
    saved.reader = FightReader::new();
    saved.read.fighters[1].health = 1;
    fight.restore_netplay_snapshot(&saved);
    fight.step(
        assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
    assert_eq!(fight.fight_read().fighters[1].health, 1);
    assert!(!fight.fight_read().fighters[1].reacting());
    fight
}

#[test]
fn late_block_undoes_an_actual_ko_and_only_confirmed_match_end_leaves_the_fight() {
    let (assets, close) = close_with_actual_throw_art();
    let initial = final_round_with_one_health(&close, &assets);
    let mut jab = PlayerInputs::default();
    jab.buttons[0] = true;
    let mut probe = initial.clone();
    let mut checkpoints = Vec::new();
    let mut before_hit = None;
    for frame in 0..32 {
        let input = if frame == 0 {
            jab
        } else {
            PlayerInputs::default()
        };
        checkpoints.push((probe.save_netplay_snapshot(0), input));
        probe.step(&assets, input, PlayerInputs::default(), false);
        if probe.fight_read().fighters[1].knocked_out() {
            assert!(
                frame > 0,
                "the real jab has startup before its lethal contact"
            );
            let (saved, input) = &checkpoints[checkpoints.len() - 2];
            probe.restore_netplay_snapshot(saved);
            before_hit = Some((probe, *input));
            break;
        }
    }
    let (mut fight, first_input) =
        before_hit.expect("the real light punch actually knocks out the wounded fighter");
    let mut reference = fight.clone();
    let mut guard = PlayerInputs::default();
    guard.directions[0] = true; // P2 faces left: right is back.
    let mut observed_block = false;
    let mut net = RollbackEngine::new(0, 0, 8);
    for frame in 0..8 {
        let local = if frame == 0 {
            first_input
        } else {
            PlayerInputs::default()
        };
        let reference_output = reference.step(&assets, local, guard, false);
        // Native normals leave the block sound and guard state rather
        // than the styles-only FightRead::blocked edge.
        observed_block |= reference_output.sound_latch == 8
            && reference.fight_read().fighters[1].guarding()
            && reference.fight_read().fighters[1].health == 1;
        let output = net.step(&mut fight, &assets, local).unwrap();
        assert!(
            output.event.is_none(),
            "a speculative result does not leave the fight"
        );
        if frame >= 1 {
            assert!(
                fight.fight_read().fighters[1].knocked_out(),
                "the predicted lethal contact really occurred"
            );
        }
    }
    assert!(
        observed_block,
        "the correction is an actual block, not merely an attack that missed"
    );
    assert!(!net.confirmed_match_over());
    net.apply_remote_inputs(&mut fight, &assets, 0, &[encode_inputs(guard); 8]);
    assert_eq!(net.last_rollback_frames(), 8);
    assert_eq!(fight.fight_read().fighters[1].health, 1);
    assert!(!fight.fight_read().fighters[1].knocked_out());
    assert!(!fight.is_over());
    assert!(fight.machine().save_snapshot() == reference.machine().save_snapshot());
    assert_eq!(fight.state_checksum(), reference.state_checksum());

    // The original engine reports MatchOver later, after its victory/result
    // flow. Observe that real boundary separately from the earlier KO.
    let mut terminal = initial;
    let mut boundary = None;
    for frame in 0..1_500 {
        let before = terminal.save_netplay_snapshot(0);
        let output = terminal.step(
            &assets,
            if frame == 0 {
                jab
            } else {
                PlayerInputs::default()
            },
            PlayerInputs::default(),
            false,
        );
        if output.event == Some(FightEvent::MatchOver) {
            assert_eq!(terminal.match_winner(), Some(0));
            terminal.restore_netplay_snapshot(&before);
            assert!(!terminal.is_over());
            boundary = Some(terminal);
            break;
        }
    }
    let mut terminal = boundary.expect("the actual final KO reaches the original match-end flow");
    let mut net = RollbackEngine::new(0, 0, 8);
    for _ in 0..7 {
        let output = net
            .step(&mut terminal, &assets, PlayerInputs::default())
            .unwrap();
        assert!(output.event.is_none());
        assert!(
            terminal.is_over(),
            "speculation can continue for six ticks after the real match-end event"
        );
        assert!(!net.confirmed_match_over());
    }
    // Correcting frame zero restores its pre-step over=false state before
    // replaying the real terminal flow; only confirmation releases the event.
    net.apply_remote_inputs(&mut terminal, &assets, 0, &[1]);
    assert!(net.confirmed_match_over());
    assert_eq!(net.confirmed_events(), [(0, FightEvent::MatchOver)]);
    let output = net
        .step(&mut terminal, &assets, PlayerInputs::default())
        .unwrap();
    assert_eq!(output.event, Some(FightEvent::MatchOver));
    let output = net
        .step(&mut terminal, &assets, PlayerInputs::default())
        .unwrap();
    assert!(
        output.event.is_none(),
        "the confirmed terminal event is delivered once"
    );
}
