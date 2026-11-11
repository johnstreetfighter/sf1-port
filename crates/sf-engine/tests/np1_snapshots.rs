//! NP1 (#304): complete Fight snapshots replay the real engine and its output.

use std::path::Path;
use std::sync::Arc;

use sf_engine::combo::throw::{DAMAGE, TECH_WINDOW, THROWN_MOVE, THROW_MOVE};
use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::DataBank;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{
    Fight, FightDips, FightFrame, FightKind, FightSetup, FightSnapshot, OverlayMode,
};
use sf_engine::front::session::PlayerInputs;
use sf_engine::moves::{InputNotation, REGISTRY};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::tasks::Scheduler;
use sf_video::announcer::AnnouncerLine;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::{CalloutEvent, CalloutKind, Side};
use sf_video::era_hud::{EraGauge, EraHud, Fill, GaugeArt, GaugeRow};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
const FIGHT_LOOP: u32 = 0x3eafa;
const ROUND_START: u32 = 0x3ea3e;

fn assets() -> Assets {
    load_assets(ROMS, MANIFEST)
        .expect("NP1 snapshot integration tests require the verified local ROM set")
}

fn setup(kind: FightKind) -> FightSetup {
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = match kind {
        FightKind::TrainingCpu | FightKind::TrainingDummy => PlayMode::Training,
        _ => PlayMode::Versus,
    };
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = TournamentFighter::Ken;
    FightSetup {
        kind,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    }
}

fn start(assets: &Assets, kind: FightKind) -> Fight {
    Fight::start(&assets.maincpu, setup(kind), Some(0), None)
        .expect("the cabinet driver reaches a real Local fight")
}

fn local(assets: &Assets) -> Fight {
    start(assets, FightKind::Local)
}

fn step(fight: &mut Fight, assets: &Assets, p1: PlayerInputs, p2: PlayerInputs) -> FightFrame {
    let out = fight.step(assets, p1, p2, false);
    assert!(
        fight.machine().unrecovered().is_empty(),
        "the real fight stays on translated paths"
    );
    out
}

fn idle(fight: &mut Fight, assets: &Assets) -> FightFrame {
    step(
        fight,
        assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
    )
}

fn to_control(fight: &mut Fight, assets: &Assets) {
    for _ in 0..1_500 {
        if flow_position(fight.machine()) == FIGHT_LOOP && !fight.driving() {
            for _ in 0..240 {
                idle(fight, assets);
            }
            return;
        }
        idle(fight, assets);
    }
    panic!("the cabinet driver never handed control to the players");
}

fn same_output(actual: &FightFrame, expected: &FightFrame, frame: usize) {
    assert!(
        actual.frame.argb == expected.frame.argb,
        "frame {frame}: presented pixels replay"
    );
    assert_eq!(
        actual.sound_latch, expected.sound_latch,
        "frame {frame}: sound command replays"
    );
    assert_eq!(
        actual.latch_cycles, expected.latch_cycles,
        "frame {frame}: sound timing replays"
    );
    assert_eq!(
        actual.announcements, expected.announcements,
        "frame {frame}: announcer output replays"
    );
    assert_eq!(
        actual.event, expected.event,
        "frame {frame}: fight event replays"
    );
}

fn replay(fight: &mut Fight, assets: &Assets, inputs: &[(PlayerInputs, PlayerInputs)]) {
    let saved = fight.save_netplay_snapshot(17);
    let checksum = fight.state_checksum();
    let native = fight.machine().save_snapshot();
    let read = *fight.fight_read();
    let mut expected: Vec<(FightFrame, FightSnapshot, u64)> = Vec::new();
    for (frame, &(p1, p2)) in inputs.iter().enumerate() {
        let out = step(fight, assets, p1, p2);
        expected.push((
            out,
            fight.save_netplay_snapshot(18 + frame as u32),
            fight.state_checksum(),
        ));
    }
    assert!(
        fight.save_netplay_snapshot(17) != saved,
        "the original run advanced beyond the saved state"
    );
    fight.restore_netplay_snapshot(&saved);
    assert!(
        fight.save_netplay_snapshot(17) == saved,
        "every saved wrapper field is restored"
    );
    assert!(
        fight.machine().save_snapshot() == native,
        "the independent native-machine snapshot is restored"
    );
    assert_eq!(
        *fight.fight_read(),
        read,
        "the public reader rolls back with the machine"
    );
    assert_eq!(
        fight.state_checksum(),
        checksum,
        "the restored gameplay checksum matches"
    );
    for (frame, (&(p1, p2), (out, snapshot, checksum))) in inputs.iter().zip(&expected).enumerate()
    {
        let actual = step(fight, assets, p1, p2);
        same_output(&actual, out, frame);
        assert!(
            fight.save_netplay_snapshot(18 + frame as u32) == *snapshot,
            "frame {frame}: full fight state replays"
        );
        assert!(
            fight.machine().save_snapshot() == snapshot.machine,
            "frame {frame}: the independent machine state replays"
        );
        assert_eq!(
            fight.state_checksum(),
            *checksum,
            "frame {frame}: the gameplay checksum replays"
        );
    }
}

#[test]
fn hidden_drive_and_midfight_snapshots_restore_state_pixels_and_sound() {
    let assets = assets();
    let mut fight = local(&assets);
    assert!(fight.driving());
    replay(
        &mut fight,
        &assets,
        &[(PlayerInputs::default(), PlayerInputs::default()); 8],
    );

    to_control(&mut fight, &assets);
    let inputs: Vec<_> = (0..24)
        .map(|frame| {
            let mut p1 = PlayerInputs::default();
            p1.directions[0] = frame < 10;
            p1.buttons[0] = frame == 12;
            (p1, PlayerInputs::default())
        })
        .collect();
    replay(&mut fight, &assets, &inputs);
}

#[test]
fn local_presentation_and_output_queues_restore_without_changing_simulation_checksum() {
    let assets = assets();
    let mut fight = local(&assets);
    to_control(&mut fight, &assets);
    let original = fight.save_netplay_snapshot(0);
    let checksum = fight.state_checksum();
    let mut plain = fight.clone();

    fight.set_overlay(OverlayMode::Both);
    fight.set_gauge_art(GaugeArt::Plate);
    fight.set_move_inputs(InputNotation::Words);
    assert!(
        fight.save_netplay_snapshot(0) == original,
        "local choices are not in a rollback snapshot"
    );
    assert_eq!(
        fight.state_checksum(),
        checksum,
        "local choices do not split peers"
    );

    fight.push_callouts(&[CalloutEvent::new(Side::P1, CalloutKind::Counter)]);
    fight.finish_combo(5);
    fight.set_era_hud(EraHud {
        p1: Some(EraGauge::one(GaugeRow::stock(3, 2, Fill::EMPTY))),
        p2: None,
    });
    let mut queued = fight.save_netplay_snapshot(0);
    queued.sounds.push_back(0x90);
    fight.restore_netplay_snapshot(&queued);
    assert!(
        queued != original,
        "the full snapshot retains pending delivery state"
    );
    assert_eq!(
        fight.state_checksum(),
        checksum,
        "HUD and output queues are not gameplay state"
    );

    let delivered = idle(&mut fight, &assets);
    let after_delivery = fight.save_netplay_snapshot(1);
    let plain_frame = idle(&mut plain, &assets);
    assert_eq!(
        delivered.sound_latch, 0x90,
        "the saved sound queue is delivered"
    );
    assert_eq!(
        delivered.announcements,
        [AnnouncerLine::Counter, AnnouncerLine::Combo { hits: 5 }]
    );
    assert!(
        delivered.frame.argb != plain_frame.frame.argb,
        "the local presentation really differs"
    );
    assert!(
        fight.machine().save_snapshot() == plain.machine().save_snapshot(),
        "delivery and rendering leave identical native state"
    );
    assert_eq!(
        fight.state_checksum(),
        plain.state_checksum(),
        "delivering different local output does not split peers"
    );

    fight.restore_netplay_snapshot(&queued);
    same_output(&idle(&mut fight, &assets), &delivered, 0);
    assert!(
        fight.save_netplay_snapshot(1) == after_delivery,
        "pending sounds, call-outs and gauge state replay together"
    );

    fight.restore_netplay_snapshot(&original);
    assert!(
        fight.save_netplay_snapshot(0) == original,
        "earlier HUD and delivery state is restored"
    );
    assert_eq!(
        fight.overlay(),
        OverlayMode::Both,
        "the current overlay choice survives rollback"
    );
    assert_eq!(
        fight.gauge_art(),
        GaugeArt::Plate,
        "the current gauge art survives rollback"
    );
    assert_eq!(
        fight.move_inputs(),
        InputNotation::Words,
        "the current notation survives rollback"
    );
    assert_eq!(fight.state_checksum(), checksum);
}

#[test]
fn training_show_and_the_reads_that_drive_it_are_gameplay_state() {
    let assets = assets();
    let mut fight = start(&assets, FightKind::TrainingDummy);
    to_control(&mut fight, &assets);
    let idle_checksum = fight.state_checksum();
    let mut idle_fight = fight.clone();

    fight.show(REGISTRY[0]);
    assert!(
        fight.showing().is_some(),
        "Training started the actual move input driver"
    );
    assert_ne!(
        fight.state_checksum(),
        idle_checksum,
        "a Show changes future player inputs"
    );

    let mut busy_read = *fight.fight_read();
    busy_read.fighters[0].state |= 0x80;
    let mut queued = fight.clone();
    queued.feed_reads(&[busy_read]);
    assert_ne!(
        queued.state_checksum(),
        fight.state_checksum(),
        "queued reads affect a Show's future readiness decisions"
    );

    replay(
        &mut fight,
        &assets,
        &[(PlayerInputs::default(), PlayerInputs::default()); 120],
    );
    for _ in 0..120 {
        idle(&mut idle_fight, &assets);
    }
    assert_eq!(
        fight.machine().state.frame,
        idle_fight.machine().state.frame
    );
    assert!(
        fight.machine().save_snapshot() != idle_fight.machine().save_snapshot(),
        "the Show really fed different inputs to the native engine"
    );
}

/// An independently observed step, retained while the live fight advances.
/// The comparison clone has never been restored through the API under test.
struct Boundary {
    before: FightSnapshot,
    input: (PlayerInputs, PlayerInputs),
    output: FightFrame,
    after: Fight,
}

fn replay_boundary(
    fight: &mut Fight,
    assets: &Assets,
    mut boundary: Boundary,
    tail: &[(PlayerInputs, PlayerInputs)],
) {
    fight.restore_netplay_snapshot(&boundary.before);
    assert!(!fight.is_over(), "a speculative match end is undone");
    assert!(
        fight.save_netplay_snapshot(boundary.before.frame) == boundary.before,
        "the complete pre-step boundary is restored"
    );
    let out = step(fight, assets, boundary.input.0, boundary.input.1);
    same_output(&out, &boundary.output, 0);
    assert!(
        fight.save_netplay_snapshot(0) == boundary.after.save_netplay_snapshot(0),
        "the independently observed boundary step replays"
    );
    for (frame, &(p1, p2)) in tail.iter().enumerate() {
        let expected = step(&mut boundary.after, assets, p1, p2);
        let actual = step(fight, assets, p1, p2);
        same_output(&actual, &expected, frame + 1);
        assert!(
            fight.save_netplay_snapshot(0) == boundary.after.save_netplay_snapshot(0),
            "frame {frame}: wrapper state matches the never-restored fight"
        );
        assert!(
            fight.machine().save_snapshot() == boundary.after.machine().save_snapshot(),
            "frame {frame}: native state matches the never-restored fight"
        );
        assert_eq!(
            fight.state_checksum(),
            boundary.after.state_checksum(),
            "frame {frame}: gameplay checksum replays"
        );
    }
}

/// The existing Local acceptance script: walk in, stop, then cycle normals.
fn p1_attacks(frame: u32) -> (PlayerInputs, PlayerInputs) {
    let mut p1 = PlayerInputs::default();
    let phase = frame % 120;
    p1.directions[0] = phase < 40;
    if phase >= 40 && phase % 10 < 3 {
        p1.buttons[((frame / 10) % 6) as usize] = true;
    }
    (p1, PlayerInputs::default())
}

#[test]
fn round_transition_and_final_ko_replay_after_a_speculative_match_end() {
    let assets = assets();
    let mut fight = local(&assets);
    to_control(&mut fight, &assets);
    let mut frame = 0;
    let mut checked_round_transition = false;
    while frame < 20_000 {
        let before = fight.save_netplay_snapshot(frame);
        let input = p1_attacks(frame);
        let output = step(&mut fight, &assets, input.0, input.1);
        frame += 1;

        if !checked_round_transition && before.round_start.as_ref() != fight.round_start() {
            assert_eq!(
                flow_position(fight.machine()),
                ROUND_START,
                "the next round really starts"
            );
            let boundary = Boundary {
                before,
                input,
                output,
                after: fight.clone(),
            };
            let tail: Vec<_> = (frame..frame + 8).map(p1_attacks).collect();
            for &(p1, p2) in &tail {
                step(&mut fight, &assets, p1, p2);
            }
            replay_boundary(&mut fight, &assets, boundary, &tail);
            frame += 8;
            checked_round_transition = true;
            continue;
        }

        let p1_wins = before.machine.work[(P1_STRUCT + 8 - 0xff8000) as usize];
        let lethal_final_hit = p1_wins == 1
            && before.read.fighters[1].health > 0
            && fight.fight_read().fighters[1].knocked_out();
        if lethal_final_hit {
            assert!(
                !before.over,
                "the saved frame is immediately before the real final-round KO"
            );
            let boundary = Boundary {
                before,
                input,
                output,
                after: fight.clone(),
            };
            let mut tail = Vec::new();
            let mut match_events = 0;
            while !fight.is_over() && tail.len() < 1_500 {
                let input = p1_attacks(frame);
                let out = step(&mut fight, &assets, input.0, input.1);
                match_events +=
                    usize::from(out.event == Some(sf_engine::front::fight::FightEvent::MatchOver));
                tail.push(input);
                frame += 1;
            }
            assert!(
                fight.is_over(),
                "the lethal hit reaches the real match-end flow"
            );
            assert_eq!(match_events, 1);
            assert_eq!(fight.match_winner(), Some(0));
            let stopped_frame = fight.machine().state.frame;
            for _ in 0..8 {
                let input = (PlayerInputs::default(), PlayerInputs::default());
                let out = step(&mut fight, &assets, input.0, input.1);
                assert!(out.event.is_none(), "the match-end event is emitted once");
                tail.push(input);
            }
            assert_eq!(
                fight.machine().state.frame,
                stopped_frame,
                "eight post-match frames do not step the machine"
            );
            replay_boundary(&mut fight, &assets, boundary, &tail);
            assert!(
                fight.is_over(),
                "replaying the actual final KO ends the match again"
            );
            assert!(
                checked_round_transition,
                "the first round checkpoint was tested too"
            );
            return;
        }
        assert!(
            !fight.is_over(),
            "the input script must win by a real observed KO"
        );
    }
    panic!("the Local input script did not finish the match");
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

fn close_in(fight: &mut Fight, assets: &Assets, distance: i32) {
    for _ in 0..300 {
        if (center_x(fight, P2_STRUCT) - center_x(fight, P1_STRUCT)).abs() <= distance {
            for _ in 0..4 {
                idle(fight, assets);
            }
            return;
        }
        let mut walk = PlayerInputs::default();
        walk.directions[0] = true;
        step(fight, assets, walk, PlayerInputs::default());
    }
    panic!("the players never walked into attack range");
}

#[test]
fn a_real_modern_combo_replays_from_its_first_hits_hitstun() {
    let assets = assets();
    let mut fight = local(&assets);
    to_control(&mut fight, &assets);
    close_in(&mut fight, &assets, 48);
    let inputs: Vec<_> = (0..30)
        .map(|frame| {
            let mut p1 = PlayerInputs::default();
            p1.buttons[0] = frame % 2 == 0;
            (p1, PlayerInputs::default())
        })
        .collect();
    let mut frame = 0;
    while !fight.fight_read().fighters[1].hitstun() && frame < inputs.len() {
        step(&mut fight, &assets, inputs[frame].0, inputs[frame].1);
        frame += 1;
    }
    assert!(
        frame < inputs.len(),
        "the real jab lands before the string ends"
    );
    assert_ne!(fight.machine().toggle_state().combo.hitstun[..2], [0, 0]);
    replay(&mut fight, &assets, &inputs[frame..]);
    assert!(
        fight
            .callouts()
            .frame()
            .side(Side::P1)
            .combo
            .is_some_and(|combo| combo.hits >= 2),
        "the continued inputs produce a real multi-hit combo after restoring hitstun",
    );
}

#[test]
fn a_real_normal_to_special_cancel_replays_from_its_open_window() {
    let assets = assets();
    let mut fight = local(&assets);
    to_control(&mut fight, &assets);
    close_in(&mut fight, &assets, 48);
    // Crouching LK into down, down-forward, forward + LP on frame 8.
    let inputs: Vec<_> = (0..60)
        .map(|frame| {
            let mut p1 = PlayerInputs::default();
            match frame {
                0 => {
                    p1.directions[2] = true;
                    p1.buttons[3] = true;
                }
                1..=2 => p1.directions[2] = true,
                3..=4 => {
                    p1.directions[0] = true;
                    p1.directions[2] = true;
                }
                5..=8 => {
                    p1.directions[0] = true;
                    p1.buttons[0] = frame == 8;
                }
                _ => {}
            }
            (p1, PlayerInputs::default())
        })
        .collect();
    let mut frame = 0;
    while fight.machine().toggle_state().cancel.window[0] == 0 && frame < 8 {
        step(&mut fight, &assets, inputs[frame].0, inputs[frame].1);
        frame += 1;
    }
    assert!(
        frame < 8,
        "the normal connects before the special button press"
    );
    assert!(fight.machine().toggle_state().cancel.window[0] > 0);
    replay(&mut fight, &assets, &inputs[frame..]);
    assert_eq!(
        fight.machine().toggle_state().cancel.count[0],
        1,
        "the actual normal cancels into the special"
    );
}

fn local_with_throw_art() -> (Assets, Fight) {
    let assets = assets();
    let pack = ArtPack::open(Path::new(PACK))
        .expect("NP1 throw replay requires the verified local art pack");
    let recipes = [
        include_str!("../../../art/recipes/ryu.json"),
        include_str!("../../../art/recipes/ken.json"),
    ];
    let reviews = [
        include_str!("../../../art/review/ryu.json"),
        include_str!("../../../art/review/ken.json"),
    ];
    let art = load_art_from(
        &assets,
        &ArtSources {
            recipes: &recipes,
            reviews: Some(&reviews),
            pack: Ok(&pack),
        },
    )
    .expect("Ryu and Ken's approved poses load");
    for fighter in ["ryu", "ken"] {
        assert!(
            art.catalogue(fighter).is_some(),
            "{fighter}'s actual catalogue is loaded"
        );
    }
    let bank = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
    for fighter in [TournamentFighter::Ryu, TournamentFighter::Ken] {
        assert!(
            bank.throw_entry(fighter).is_some(),
            "{fighter:?}'s real throw data is built"
        );
    }
    let assets = assets.with_sprite_bank(&art.bank);
    let fight = Fight::start_banked(
        &assets.maincpu,
        setup(FightKind::Local),
        Some(0),
        None,
        Some(bank),
    )
    .expect("the actual banked Local fight starts");
    (assets, fight)
}

#[test]
fn real_throw_and_last_window_tech_replay_the_catalogue_moves_and_timers() {
    let (assets, mut close) = local_with_throw_art();
    to_control(&mut close, &assets);
    // Keep the thrower in stance while P2 walks into the proven throw setup.
    for _ in 0..200 {
        if (center_x(&close, P1_STRUCT) - center_x(&close, P2_STRUCT)).abs() <= 40 {
            break;
        }
        let mut p2 = PlayerInputs::default();
        p2.directions[1] = true;
        step(&mut close, &assets, PlayerInputs::default(), p2);
    }
    for _ in 0..16 {
        idle(&mut close, &assets);
    }
    assert!((center_x(&close, P1_STRUCT) - center_x(&close, P2_STRUCT)).abs() <= 48);
    let before_health = close.fight_read().fighters[1].health;
    let mut grab = PlayerInputs::default();
    grab.directions[0] = true;
    grab.buttons[2] = true;

    let mut thrown = close.clone();
    step(&mut thrown, &assets, grab, PlayerInputs::default());
    assert_eq!(thrown.fight_read().fighters[0].move_id, THROW_MOVE);
    assert_eq!(thrown.fight_read().fighters[1].move_id, THROWN_MOVE);
    assert!(thrown.machine().toggle_state().throw.tech_window[1] > 0);
    replay(
        &mut thrown,
        &assets,
        &[(PlayerInputs::default(), PlayerInputs::default()); 90],
    );
    assert_eq!(
        thrown.fight_read().fighters[1].health,
        before_health - u16::from(DAMAGE),
        "the restored throw actually slams for damage"
    );

    let mut teched = close;
    step(&mut teched, &assets, grab, PlayerInputs::default());
    for _ in 1..TECH_WINDOW {
        idle(&mut teched, &assets);
    }
    assert!(
        teched.machine().toggle_state().throw.tech_window[1] > 0,
        "the saved grab is still techable on its last window frame"
    );
    let mut tech = PlayerInputs::default();
    tech.directions[0] = true;
    tech.buttons[5] = true;
    replay(
        &mut teched,
        &assets,
        &[
            (PlayerInputs::default(), tech),
            (PlayerInputs::default(), PlayerInputs::default()),
            (PlayerInputs::default(), PlayerInputs::default()),
        ],
    );
    assert_eq!(
        teched.fight_read().fighters[1].health,
        before_health,
        "the replayed last-window tech prevents damage"
    );
    assert_eq!(teched.fight_read().fighters[0].move_id, 1);
    assert_eq!(teched.fight_read().fighters[1].move_id, 1);
    assert_eq!(teched.machine().toggle_state().throw.tech_window[1], 0);
    assert!(
        (center_x(&teched, P1_STRUCT) - center_x(&teched, P2_STRUCT)).abs() > 48,
        "the real tech pushes the players apart"
    );
}

#[test]
fn checksum_detects_hidden_scheduler_and_hitstun_that_change_future_gameplay() {
    let assets = assets();
    let mut fight = local(&assets);
    to_control(&mut fight, &assets);
    let original = fight.save_netplay_snapshot(0);
    let checksum = fight.state_checksum();

    // A missing hidden continuation is a desync even with identical RAM
    // and registers. Inject it through the public machine snapshot seam.
    let mut rescheduled = original.clone();
    rescheduled.machine.scheduler = Scheduler::new();
    assert_ne!(rescheduled.machine.scheduler, original.machine.scheduler);
    assert!(rescheduled.machine.work == original.machine.work);
    assert!(rescheduled.machine.state == original.machine.state);
    let mut different_flow = fight.clone();
    different_flow.restore_netplay_snapshot(&rescheduled);
    assert!(
        different_flow.machine().save_snapshot() == rescheduled.machine,
        "the native snapshot independently confirms the hidden change"
    );
    assert_ne!(
        different_flow.state_checksum(),
        checksum,
        "scheduler continuations are not represented by work RAM alone"
    );
    let mut straight = fight.clone();
    let mut observed_untranslated = false;
    for _ in 0..30 {
        let expected = idle(&mut straight, &assets);
        assert!(expected.event.is_none());
        let actual = different_flow.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
        if matches!(
            actual.event,
            Some(sf_engine::front::fight::FightEvent::Untranslated { .. })
        ) {
            observed_untranslated = true;
            assert!(
                straight.machine().save_snapshot() != different_flow.machine().save_snapshot(),
                "the independent native snapshots confirm the divergence"
            );
            break;
        }
    }
    assert!(
        observed_untranslated,
        "the missing scheduler continuation changes an actual fight event"
    );

    close_in(&mut fight, &assets, 48);
    for frame in 0..30 {
        if fight.fight_read().fighters[1].hitstun() {
            break;
        }
        let mut jab = PlayerInputs::default();
        jab.buttons[0] = frame % 2 == 0;
        step(&mut fight, &assets, jab, PlayerInputs::default());
    }
    assert!(
        fight.fight_read().fighters[1].hitstun(),
        "a real jab supplies the lock being altered"
    );
    let in_hitstun = fight.save_netplay_snapshot(0);
    let checksum = fight.state_checksum();
    let mut longer_stun = in_hitstun.clone();
    longer_stun.machine.toggle_state.combo.hitstun[1] =
        longer_stun.machine.toggle_state.combo.hitstun[1].saturating_add(8);
    assert!(longer_stun.machine.work == in_hitstun.machine.work);
    assert!(longer_stun.machine.state == in_hitstun.machine.state);
    let mut delayed = fight.clone();
    delayed.restore_netplay_snapshot(&longer_stun);
    assert!(
        delayed.machine().save_snapshot() == longer_stun.machine,
        "the native snapshot independently confirms the hidden timer change"
    );
    assert_ne!(
        delayed.state_checksum(),
        checksum,
        "modern runtime timers affect the gameplay checksum"
    );
    let mut observed_different_move = false;
    for frame in 0..60 {
        let mut counterattack = PlayerInputs::default();
        counterattack.buttons[0] = frame % 2 == 0;
        step(&mut fight, &assets, PlayerInputs::default(), counterattack);
        step(
            &mut delayed,
            &assets,
            PlayerInputs::default(),
            counterattack,
        );
        observed_different_move |=
            fight.fight_read().fighters[1].move_id != delayed.fight_read().fighters[1].move_id;
    }
    assert!(
        observed_different_move,
        "the hidden hitstun change really delays the player's next move"
    );
}
