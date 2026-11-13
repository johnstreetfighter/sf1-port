#![allow(dead_code)]
#[path = "support/alpha.rs"]
mod alpha;

use sf_engine::era::{EraFamily, GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::front::{FrameInputs, PlayerInputs};
use sf_engine::moves::InputNotation;
use sf_engine::roster::TournamentFighter;
use sf_engine::utility::{ComboTrialSpec, Drill, RosterFighter};
use sf_video::callouts::{CalloutKind, Side, Strip};

#[test]
fn five_recipes_exist_for_each_fighter_in_each_era() {
    for fighter in TournamentFighter::ALL
        .into_iter()
        .map(RosterFighter::from)
        .chain([RosterFighter::EvilRyu, RosterFighter::MBison])
    {
        for era in [
            EraFamily::Sf1987,
            EraFamily::Sf1Plus,
            EraFamily::Sf2,
            EraFamily::SfZ,
            EraFamily::Sf3,
            EraFamily::Sf4,
            EraFamily::Sf5,
            EraFamily::Sf6,
        ] {
            for index in 1..=5 {
                let spec = ComboTrialSpec::for_fighter_era(fighter, era, index);
                assert_eq!(spec.index, index);
                assert!(spec.steps.len() >= 2);
                assert!(spec.steps.iter().all(|step| !step.label.is_empty()));
            }
        }
    }
}

fn training() -> Option<sf_engine::front::GameSession> {
    alpha::training_fight_with_gauges(
        TournamentFighter::Ryu,
        Revision::Sf1Plus,
        Loadout::Standard,
        GaugeMode::EraDefault,
        false,
    )
}

#[test]
fn selecting_an_exercise_replaces_the_other_and_resets_with_positions() {
    let Some(session) = training() else {
        return;
    };
    let mut fight = session.fight().unwrap().clone();
    for drill in Drill::ALL {
        fight.set_drill(Some(drill));
        for _ in 0..220 {
            fight.step(
                session.assets(),
                PlayerInputs::default(),
                PlayerInputs::default(),
                false,
            );
        }
        assert_eq!(fight.active_drill(), Some(drill));
        assert!(fight.reset_positions());
        assert_eq!(fight.drill_completed(), Some(0));
        assert_eq!(fight.active_drill(), Some(drill));
    }
    fight.set_combo_trial(Some(2));
    assert_eq!(fight.active_drill(), None);
    assert_eq!(fight.active_trial(), Some(2));
    assert!(fight.reset_positions());
    assert_eq!(fight.trial_progress(), Some(0));
    fight.set_drill(Some(Drill::AntiAir));
    assert_eq!(fight.active_trial(), None);
}

fn x(fight: &Fight, addr: u32) -> i32 {
    let ram = fight.machine().work_ram();
    let index = (addr + 0x10 - 0xff8000) as usize;
    let entity = u16::from_be_bytes([ram[index], ram[index + 1]]) as usize;
    let objects = &fight.machine().state.objectram;
    let coord = i32::from(u16::from_be_bytes([
        objects[entity + 6],
        objects[entity + 7],
    ]));
    let cell = if i16::from_be_bytes([objects[entity + 2], objects[entity + 3]]) >= 0x400 {
        32
    } else {
        16
    };
    coord + (i32::from(objects[entity + 0x0d]) + 1) * cell / 2
}

fn facing_left(fight: &Fight, addr: u32) -> bool {
    let ram = fight.machine().work_ram();
    let index = (addr + 0x10 - 0xff8000) as usize;
    let entity = u16::from_be_bytes([ram[index], ram[index + 1]]) as usize;
    fight.machine().state.objectram[entity + 0x37] as i8 >= 4
}

#[test]
fn real_ryu_trial_progresses_on_contact_and_resets_after_a_missed_cancel() {
    let Some(mut session) = training() else {
        return;
    };
    for _ in 0..180 {
        let fight = session.fight().unwrap();
        if x(fight, sf_engine::fighter::P2_STRUCT) - x(fight, sf_engine::fighter::P1_STRUCT) <= 37 {
            break;
        }
        session.step(FrameInputs {
            p1: alpha::pad(alpha::RIGHT, None),
            ..Default::default()
        });
    }
    for _ in 0..6 {
        session.step(FrameInputs::default());
    }
    let mut fight = session.fight().unwrap().clone();
    let assets = session.assets();
    fight.set_combo_trial(Some(1));
    let step = |fight: &mut Fight, pad: PlayerInputs| {
        fight.step(assets, pad, PlayerInputs::default(), false);
    };
    step(&mut fight, alpha::pad(0, Some(1)));
    for dir in [alpha::DOWN, alpha::DOWN | alpha::RIGHT] {
        step(&mut fight, alpha::pad(dir, None));
    }
    for _ in 0..22 {
        if fight.trial_progress() == Some(1) {
            break;
        }
        step(&mut fight, PlayerInputs::default());
    }
    assert_eq!(fight.trial_progress(), Some(1), "standing MP hits");
    let mut missed = fight.clone();
    for _ in 0..75 {
        if missed.trial_progress() == Some(0) {
            break;
        }
        step(&mut missed, PlayerInputs::default());
    }
    assert_eq!(missed.trial_progress(), Some(0), "the combo was dropped");
    assert!(missed
        .callouts()
        .frame()
        .side(Side::P1)
        .words
        .contains(&Some(CalloutKind::fix("CANCEL FASTER"))));
    step(&mut fight, alpha::pad(alpha::RIGHT, Some(2)));
    for _ in 0..30 {
        if fight.trial_progress() == Some(2) {
            break;
        }
        step(&mut fight, PlayerInputs::default());
    }
    assert_eq!(
        fight.trial_progress(),
        Some(2),
        "Hadoken completes the live cancel"
    );
}

fn drill_fight(
    assets: &sf_video::romset::Assets,
    kind: sf_engine::front::fight::FightKind,
) -> Fight {
    use sf_engine::config::MatchConfig;
    use sf_engine::data_bank::DataBank;
    use sf_engine::front::{
        cabinet::{flow_position, Players},
        fight::{FightDips, FightKind, FightSetup},
    };
    use sf_engine::roster::PlayMode;
    let mut config = MatchConfig::default();
    config.roster.mode = PlayMode::Training;
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = TournamentFighter::Ken;
    config.p2_fair_cpu = kind == FightKind::TrainingCpu;
    let setup = FightSetup {
        kind,
        config,
        players: Players::Two,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let mut fight = Fight::start_banked(
        &assets.maincpu,
        setup,
        Some(0),
        None,
        Some(std::sync::Arc::new(DataBank::build(&assets.maincpu, &[]))),
    )
    .expect("native Training");
    for _ in 0..1400 {
        if flow_position(fight.machine()) == sf_engine::fair_cpu::FIGHT_LOOP && !fight.driving() {
            return fight;
        }
        fight.step(
            assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
    }
    panic!("training did not reach the fight loop");
}

#[test]
fn real_throw_tech_drill_reaches_five_in_both_training_modes_and_coaches_a_miss() {
    use sf_engine::front::fight::FightKind;
    use sf_video::callouts::{CalloutKind, Side};
    let Some(session) = training() else { return };
    let assets = session.assets();
    for kind in [FightKind::TrainingDummy, FightKind::TrainingCpu] {
        let mut fight = drill_fight(assets, kind);
        fight.set_drill(Some(Drill::ThrowTech));
        let mut missed = fight.clone();
        let mut feedback = false;
        for _ in 0..350 {
            missed.step(
                assets,
                PlayerInputs::default(),
                PlayerInputs::default(),
                false,
            );
            if missed
                .callouts()
                .frame()
                .side(Side::P1)
                .words
                .contains(&Some(CalloutKind::fix("PRESS LP+LK")))
            {
                feedback = true;
                break;
            }
        }
        assert!(
            feedback,
            "{kind:?}: a missed throw tech offers the pad input"
        );
        for _ in 0..1200 {
            if fight.drill_completed() == Some(5) {
                break;
            }
            let mut pad = PlayerInputs::default();
            if fight.machine().toggle_state().throw.tech_window[0] > 0 {
                pad.buttons[0] = true;
                pad.buttons[3] = true;
            }
            fight.step(assets, pad, PlayerInputs::default(), false);
        }
        assert_eq!(
            fight.drill_completed(),
            Some(5),
            "{kind:?}: five genuine throw techs"
        );
    }
}

#[test]
fn native_guard_drill_pressure_reaches_the_player_in_both_training_modes() {
    use sf_engine::fighter::P1_STRUCT;
    use sf_engine::front::fight::FightKind;
    use sf_video::callouts::{CalloutKind, Side};
    let Some(session) = training() else { return };
    let assets = session.assets();
    for kind in [FightKind::TrainingDummy, FightKind::TrainingCpu] {
        let mut fight = drill_fight(assets, kind);
        fight.set_drill(Some(Drill::Survival));
        let mut missed = fight.clone();
        let mut feedback = false;
        for _ in 0..400 {
            missed.step(
                assets,
                PlayerInputs::default(),
                PlayerInputs::default(),
                false,
            );
            let callouts = missed.callouts().frame();
            let words = &callouts.side(Side::P1).words;
            if ["HOLD BACK", "HOLD DOWN-BACK"]
                .iter()
                .any(|msg| words.contains(&Some(CalloutKind::fix(msg))))
            {
                feedback = true;
                break;
            }
        }
        assert!(
            feedback && missed.machine().training().damage_taken[0] > 0,
            "{kind:?}: idle player gets hit by the scripted pressure"
        );
        let mut blocked_moves = Vec::new();
        for _ in 0..2500 {
            if fight.drill_completed() == Some(3) {
                break;
            }
            let mut pad = PlayerInputs::default();
            let opponent = fight.fight_read().fighters[1];
            // Use standing guard for the jumping HK, standing MP, and fireball.
            let airborne = matches!(opponent.move_id, 9..=11)
                || opponent.attacking() && opponent.state & 1 != 0;
            pad.directions[usize::from(!facing_left(&fight, P1_STRUCT))] = true;
            pad.directions[2] = !airborne && !matches!(opponent.move_id, 0x0f | 0x1a);
            fight.step(assets, pad, PlayerInputs::default(), false);
            if fight.fight_read().blocked[1]
                || fight.fight_read().landed[1].is_some_and(|hit| hit.chip)
            {
                blocked_moves.push(opponent.move_id);
            }
        }
        assert_eq!(
            fight.drill_completed(),
            Some(3),
            "{kind:?}: defended three pressure strings, blocked native moves {blocked_moves:x?}"
        );
        assert!(
            blocked_moves
                .chunks_exact(4)
                .any(|moves| moves == [0x24, 0x17, 0x0f, 0x1a]),
            "{kind:?}: jump HK, crouch LK, standing MP, and Hadoken each reached guard"
        );
    }
}

#[test]
fn native_anti_air_drill_checks_hits_in_the_air_in_both_training_modes() {
    use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
    use sf_engine::front::fight::FightKind;
    use sf_video::callouts::{CalloutKind, Side};
    let Some(session) = training() else { return };
    let assets = session.assets();
    for kind in [FightKind::TrainingDummy, FightKind::TrainingCpu] {
        let mut fight = drill_fight(assets, kind);
        fight.set_drill(Some(Drill::AntiAir));
        let mut missed = fight.clone();
        let mut feedback = false;
        for _ in 0..350 {
            missed.step(
                assets,
                PlayerInputs::default(),
                PlayerInputs::default(),
                false,
            );
            let callouts = missed.callouts().frame();
            let words = &callouts.side(Side::P1).words;
            if ["ANTI-AIR EARLIER", "HIT IN THE AIR"]
                .iter()
                .any(|msg| words.contains(&Some(CalloutKind::fix(msg))))
            {
                feedback = true;
                break;
            }
        }
        assert!(feedback, "{kind:?}: a missed anti-air gets feedback");
        let mut command = None;
        let mut fired = false;
        for _ in 0..1600 {
            if fight.drill_completed() == Some(5) {
                break;
            }
            let me = fight.fight_read().fighters[0];
            let foe = fight.fight_read().fighters[1];
            let dx = x(&fight, P2_STRUCT) - x(&fight, P1_STRUCT);
            let in_air = matches!(foe.move_id, 9..=11) || foe.attacking() && foe.state & 1 != 0;
            if !in_air {
                fired = false;
            }
            if in_air && !fired && dx.abs() <= 65 && me.move_id == 1 {
                let toward = if dx >= 0 { alpha::RIGHT } else { alpha::LEFT };
                command = Some(
                    [
                        (toward, None),
                        (alpha::DOWN, None),
                        (alpha::DOWN | toward, Some(2)),
                    ]
                    .into_iter(),
                );
                fired = true;
            }
            let pad = command
                .as_mut()
                .and_then(|command| command.next())
                .map_or_else(PlayerInputs::default, |(stick, button)| {
                    alpha::pad(stick, button)
                });
            fight.step(assets, pad, PlayerInputs::default(), false);
        }
        assert_eq!(
            fight.drill_completed(),
            Some(5),
            "{kind:?}: five native anti-airs"
        );
    }
}

#[test]
fn native_whiff_punish_drill_checks_real_recovery_in_both_training_modes() {
    use sf_engine::fighter::P1_STRUCT;
    use sf_engine::front::fight::FightKind;
    let Some(session) = training() else { return };
    let assets = session.assets();
    for kind in [FightKind::TrainingDummy, FightKind::TrainingCpu] {
        let mut fight = drill_fight(assets, kind);
        fight.set_drill(Some(Drill::WhiffPunish));
        let mut missed = fight.clone();
        let mut feedback = false;
        for _ in 0..350 {
            missed.step(
                assets,
                PlayerInputs::default(),
                PlayerInputs::default(),
                false,
            );
            if missed
                .callouts()
                .frame()
                .side(Side::P1)
                .words
                .contains(&Some(CalloutKind::fix("PUNISH SOONER")))
            {
                feedback = true;
                break;
            }
        }
        assert!(feedback, "{kind:?}: a missed whiff gets coaching");
        let step = |fight: &mut Fight, pad| fight.step(assets, pad, PlayerInputs::default(), false);
        for target in 1..=5 {
            for _ in 0..200 {
                if fight.fight_read().fighters[1].attacking() {
                    break;
                }
                step(&mut fight, PlayerInputs::default());
            }
            let (delay, punish) = match fight.fight_read().fighters[1].move_id {
                0x10 => (8, alpha::pad(0, Some(2))),
                0x16 => (21, alpha::pad(alpha::DOWN, Some(5))),
                move_id => panic!("{kind:?}: expected a heavy normal, got {move_id:#x}"),
            };
            let forward = if facing_left(&fight, P1_STRUCT) {
                alpha::LEFT
            } else {
                alpha::RIGHT
            };
            // Native sprites' move streams can leave the characters a few pixels
            // apart after each punish. Try the same attack with a short step and
            // allow adjacent input timings as the animations drift.
            let mut punished = None;
            'try_punish: for walk in [0, 2, 4] {
                for timing in [delay, delay - 2, delay + 2, delay - 1, delay + 1] {
                    let mut attempt = fight.clone();
                    for _ in 0..timing - walk {
                        step(&mut attempt, PlayerInputs::default());
                    }
                    for _ in 0..walk {
                        step(&mut attempt, alpha::pad(forward, None));
                    }
                    step(&mut attempt, punish);
                    for _ in 0..30 {
                        if attempt.drill_completed() == Some(target) {
                            punished = Some(attempt);
                            break 'try_punish;
                        }
                        step(&mut attempt, PlayerInputs::default());
                    }
                }
            }
            fight = punished
                .unwrap_or_else(|| panic!("{kind:?}: Ryu's heavy punishes recovery {target}"));
            for _ in 0..30 {
                step(&mut fight, PlayerInputs::default());
            }
        }
    }
}

fn walk_in(session: &mut sf_engine::front::GameSession, distance: i32) {
    for _ in 0..180 {
        let fight = session.fight().unwrap();
        if x(fight, sf_engine::fighter::P2_STRUCT) - x(fight, sf_engine::fighter::P1_STRUCT)
            <= distance
        {
            break;
        }
        session.step(FrameInputs {
            p1: alpha::pad(alpha::RIGHT, None),
            ..Default::default()
        });
    }
    for _ in 0..6 {
        session.step(FrameInputs::default());
    }
}

#[test]
fn real_ryu_world_warrior_trial_two_completes_with_only_the_player_pad() {
    use alpha::{DOWN, RIGHT, UP};
    use sf_engine::fighter::{P1_STRUCT, P2_STRUCT, WORK_BASE};

    let Some(session) = alpha::training_fight_with_gauges(
        TournamentFighter::Ryu,
        Revision::Ww,
        Loadout::Standard,
        GaugeMode::EraDefault,
        false,
    ) else {
        return;
    };
    let mut fight = session.fight().unwrap().clone();
    let idle_checksum = fight.state_checksum();
    fight.set_combo_trial(Some(2));
    assert_ne!(idle_checksum, fight.state_checksum(), "the dummy pad changes");
    let step = |fight: &mut Fight, dir, button| {
        fight.step(session.assets(), alpha::pad(dir, button), PlayerInputs::default(), false);
    };

    // Pin the stage camera to its right bound, place Ken in the corner,
    // and set Ryu 42 px away. The dummy's posture and all hits come from play.
    let mut snapshot = fight.save_netplay_snapshot(0);
    let camera = (0xffc00c - WORK_BASE) as usize;
    snapshot.machine.work[camera..camera + 2].copy_from_slice(&0x180u16.to_be_bytes());
    for (fighter, center_x) in [(P1_STRUCT, 348), (P2_STRUCT, 390)] {
        let delta = center_x - x(&fight, fighter);
        let index = (fighter + 0x10 - WORK_BASE) as usize;
        let work = &snapshot.machine.work;
        let entity = u16::from_be_bytes([work[index], work[index + 1]]) as usize;
        let objects = &mut snapshot.machine.state.objectram;
        let rows = usize::from(objects[entity + 0xc]) + 1;
        let cols = usize::from(objects[entity + 0xd]) + 1;
        for cell in 0..rows * cols {
            let offset = entity + cell * 0x40 + 6;
            let raw_x = &mut objects[offset..offset + 2];
            let left = u16::from_be_bytes(raw_x.try_into().unwrap());
            raw_x.copy_from_slice(&(left.wrapping_add_signed(delta as i16)).to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
    for _ in 0..8 {
        step(&mut fight, 0, None);
    }
    assert_eq!(x(&fight, P2_STRUCT) - x(&fight, P1_STRUCT), 42);
    fight.set_move_inputs(InputNotation::Numpad);
    assert_eq!(
        fight.trial_strip(),
        Some(Strip::Input {
            steps: vec!["J.HK".into(), "CR.MK".into(), "236 P".into()],
            lit: 0,
        })
    );

    for frame in 0..=60 {
        let direction = if frame == 0 { UP | RIGHT } else { 0 };
        step(&mut fight, direction, (frame == 29).then_some(5));
        if frame == 29 {
            assert_eq!(fight.trial_progress(), Some(0), "the button edge is not a hit");
        }
    }
    assert_eq!(fight.trial_progress(), Some(1), "jump HK links on landing");
    assert_eq!(fight.machine().toggle_state().combo.hits[1], 1);

    step(&mut fight, DOWN, Some(4));
    step(&mut fight, DOWN | RIGHT, None);
    for _ in 0..6 {
        step(&mut fight, RIGHT, None);
    }
    assert_eq!(fight.trial_progress(), Some(2), "crouching MK hits in combo");
    assert_eq!(fight.machine().toggle_state().combo.hits[1], 2);

    for dir in [DOWN, DOWN | RIGHT] {
        step(&mut fight, dir, None);
    }
    step(&mut fight, RIGHT, Some(2));
    for _ in 0..20 {
        if fight.trial_progress() == Some(3) {
            break;
        }
        step(&mut fight, 0, None);
    }
    assert_eq!(fight.trial_progress(), Some(3), "Hadoken completes the trial");
    assert_eq!(fight.machine().toggle_state().combo.hits[1], 3);
}

#[test]
fn real_ryu_sf2_trial_counts_a_native_super_cancel() {
    let Some(mut session) =
        alpha::training_fight_with_fighter(TournamentFighter::Ryu, Revision::St, Loadout::Standard)
    else {
        return;
    };
    walk_in(&mut session, 37);
    let mut fight = session.fight().unwrap().clone();
    let step = |fight: &mut Fight, dir, button| {
        fight.step(
            session.assets(),
            alpha::pad(dir, button),
            PlayerInputs::default(),
            false,
        );
    };
    fight.set_combo_trial(Some(4));
    // Buffer the first quarter circle so the rest of the motion fits the normal's cancel window.
    step(&mut fight, 0, Some(1));
    for dir in [alpha::DOWN, alpha::DOWN | alpha::RIGHT, alpha::RIGHT] {
        step(&mut fight, dir, None);
    }
    for _ in 0..22 {
        if fight.trial_progress() == Some(1) {
            break;
        }
        step(&mut fight, 0, None);
    }
    assert!(
        matches!(fight.trial_strip(), Some(Strip::Input { lit: 1, .. })),
        "standing medium punch hits"
    );
    for (dir, button) in [
        (alpha::DOWN, None),
        (alpha::DOWN | alpha::RIGHT, None),
        (alpha::RIGHT, Some(2)),
    ] {
        step(&mut fight, dir, button);
    }
    assert_eq!(
        fight.machine().era_fighters()[0].super_meter,
        0,
        "the native cancel spends Super"
    );
    for _ in 0..60 {
        if fight.trial_progress() == Some(2) {
            break;
        }
        step(&mut fight, 0, None);
    }
    assert!(
        matches!(fight.trial_strip(), Some(Strip::Input { lit: 2, .. })),
        "Shinku Hadoken completes the trial"
    );
}
