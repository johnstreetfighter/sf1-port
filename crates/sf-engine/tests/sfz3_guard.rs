//! Guard Power through ROM-backed local fights and rollback.

#[path = "support/alpha.rs"]
mod alpha;

use alpha::{local_fight, local_select, pad, DOWN, LP, MANIFEST, RIGHT, ROMS, UP};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::era_fight::GuardPower;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::Fight;
use sf_engine::front::PlayerInputs;
use sf_engine::netplay::{encode_inputs, RollbackEngine};
use sf_engine::tasks::TIME_DIGITS;
use sf_video::callouts::{CalloutKind, Side};
use sf_video::era_hud::{GaugeRow, GaugeState};
use sf_video::romset::{load_assets, Assets};

const HK: usize = 5;
const IDLE: PlayerInputs = PlayerInputs {
    directions: [false; 4],
    buttons: [false; 6],
};
const FIGHT_LOOP: u32 = 0x3eafa;
const ROUND_COUNTER: usize = 0xffc00a - 0xff8000;

fn start_fight(mode: GaugeMode, revision: Revision, loadout: Loadout) -> Option<(Assets, Fight)> {
    let select = local_select(mode)?;
    let session = local_fight(&select, revision, loadout);
    let assets = load_assets(ROMS, MANIFEST).expect("local ROMs assemble");
    Some((assets, session.fight().expect("local fight").clone()))
}

fn guard(fight: &Fight) -> GuardPower {
    fight.machine().era_fighters()[1]
        .guard_power()
        .expect("Player 2's SFZ3 Guard Power")
}

fn approach(fight: &mut Fight, assets: &Assets) {
    for _ in 0..100 {
        let positions = fight.machine().diagnostic_telemetry();
        if positions.p2.center_x - positions.p1.center_x <= 38 {
            for _ in 0..8 {
                fight.step(assets, IDLE, IDLE, false);
            }
            return;
        }
        fight.step(assets, pad(RIGHT, None), IDLE, false);
    }
    panic!("Ryu could not approach Ken");
}

fn hard_kick(frame: u32) -> PlayerInputs {
    pad(0, (frame == 0).then_some(HK))
}

fn block_hard_kick(fight: &mut Fight, assets: &Assets) -> u32 {
    let back = pad(RIGHT, None);
    for frame in 0..32 {
        fight.step(assets, hard_kick(frame), back, false);
        let crushed = fight.machine().era_fighters()[1]
            .guard_power()
            .is_some_and(|power| power.crush_left > 0);
        if crushed || fight.fight_read().fighters[1].blockstun {
            return frame;
        }
    }
    panic!("Ryu's standing heavy kick did not make guarded contact");
}

/// Leave the fight just after its first crush and return the start of the
/// breaking kick so the same inputs can be exercised under rollback.
fn pressure_to_break(fight: &mut Fight, assets: &Assets) -> (Fight, u32) {
    for _ in 0..16 {
        approach(fight, assets);
        let before = fight.clone();
        let impact = block_hard_kick(fight, assets);
        if guard(fight).crush_left > 0 {
            return (before, impact);
        }
        assert!(guard(fight).remaining < guard(&before).remaining);
        for _ in impact..38 {
            fight.step(assets, IDLE, IDLE, false);
        }
    }
    panic!("repeated native blocks never crushed the guard");
}

fn guard_row(fight: &Fight) -> Option<GaugeRow> {
    fight.era_hud().p2.and_then(|gauge| gauge.second)
}

#[test]
fn guard_power_is_defender_side_and_only_present_for_sfz3() {
    for (mode, revision, loadout, enabled) in [
        (GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm, true),
        (GaugeMode::EraDefault, Revision::Sfz3, Loadout::VIsm, true),
        (GaugeMode::EraDefault, Revision::Sfz3, Loadout::XIsm, true),
        (GaugeMode::Unified, Revision::Sfz3, Loadout::AIsm, true),
        (
            GaugeMode::EraDefault,
            Revision::Sfz2A,
            Loadout::Standard,
            false,
        ),
        (GaugeMode::NoGauges, Revision::Sfz3, Loadout::AIsm, false),
    ] {
        let Some((assets, mut fight)) = start_fight(mode, revision, loadout) else {
            return;
        };
        assert!(fight.machine().era_fighters()[0].guard_power().is_none());
        let initial = fight.machine().era_fighters()[1].guard_power();
        let initial_row = guard_row(&fight);
        assert_eq!(
            initial.is_some(),
            enabled,
            "{mode:?} {revision:?} {loadout:?}"
        );
        let visible = enabled && mode != GaugeMode::Unified;
        assert_eq!(
            initial_row.is_some(),
            visible,
            "{mode:?} {revision:?} {loadout:?}"
        );
        if mode == GaugeMode::Unified {
            assert!(matches!(
                fight.era_hud().p2.unwrap().main,
                GaugeRow::Stock { cells: 3, .. }
            ));
        }

        approach(&mut fight, &assets);
        let health = fight.machine().era_fighters()[1].sub_hp;
        block_hard_kick(&mut fight, &assets);
        assert_eq!(fight.machine().era_fighters()[1].sub_hp, health);
        let after = fight.machine().era_fighters()[1].guard_power();
        assert!(fight.machine().era_fighters()[0].guard_power().is_none());
        if let Some(initial) = initial {
            let after = after.expect("ordinary block leaves the gauge enabled");
            assert_eq!(initial.remaining, initial.maximum);
            assert_eq!(after.maximum, initial.maximum);
            assert!(after.remaining < initial.remaining);
            assert_eq!(after.crush_left, 0);
            if visible {
                let Some(GaugeRow::Bar {
                    length: initial_length,
                    ..
                }) = initial_row
                else {
                    panic!("Guard Power uses a continuous second row");
                };
                assert!(matches!(
                    guard_row(&fight),
                    Some(GaugeRow::Bar { fill, length, .. })
                        if fill.px(100) < 100 && length == initial_length
                ));
            } else {
                assert!(guard_row(&fight).is_none());
            }
        } else {
            assert!(after.is_none());
        }
    }
}

#[test]
fn guard_crush_locks_the_defender_shrinks_the_bar_and_resets_next_round() {
    let Some((assets, mut fight)) =
        start_fight(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    let full = guard(&fight);
    let Some(GaugeRow::Bar {
        length: full_length,
        ..
    }) = guard_row(&fight)
    else {
        panic!("Guard Power starts with a continuous second row");
    };
    let initial_hp = fight.machine().era_fighters()[1].sub_hp;
    pressure_to_break(&mut fight, &assets);

    let broken = guard(&fight);
    assert_eq!(broken.remaining, 0);
    assert!(broken.maximum < full.maximum);
    assert!(broken.crush_left > 0);
    assert_eq!(fight.machine().era_fighters()[1].sub_hp, initial_hp);
    assert!(!fight.fight_read().fighters[1].guarding());
    assert!(!fight.machine().diagnostic_telemetry().p2.invulnerable);
    let callouts = fight.callouts().frame();
    let crush = Some(CalloutKind::Move {
        word: "GUARD CRUSH",
    });
    assert_eq!(
        callouts
            .side(Side::P2)
            .words
            .iter()
            .filter(|word| **word == crush)
            .count(),
        1
    );
    assert!(!callouts.side(Side::P1).words.contains(&crush));
    assert!(matches!(
        guard_row(&fight),
        Some(GaugeRow::Bar { fill, length, state: GaugeState::Greyed })
            if fill.px(100) == 0 && length.px(100) < full_length.px(100)
    ));

    fight.step(&assets, IDLE, pad(UP | RIGHT, Some(HK)), false);
    assert!(guard(&fight).crush_left > 0);
    assert!(!fight.fight_read().fighters[1].attacking());
    assert_ne!(
        fight.fight_read().fighters[1].state & 3,
        1,
        "cannot jump in stagger"
    );

    // The attacker still has to recover from the blocked move. A jab then
    // reaches the stagger while the defender keeps holding away.
    let back = pad(RIGHT, None);
    for _ in 0..20 {
        if !fight.fight_read().fighters[0].attacking() {
            break;
        }
        fight.step(&assets, IDLE, back, false);
    }
    for frame in 0..12 {
        assert!(guard(&fight).crush_left > 0, "jab must reach the stagger");
        fight.step(&assets, pad(0, (frame == 0).then_some(LP)), back, false);
        if fight.machine().era_fighters()[1].sub_hp < initial_hp {
            break;
        }
    }
    assert!(fight.machine().era_fighters()[1].sub_hp < initial_hp);

    for _ in 0..60 {
        fight.step(&assets, IDLE, IDLE, false);
        if guard(&fight).crush_left == 0 {
            break;
        }
    }
    let recovered = guard(&fight);
    assert_eq!(recovered.crush_left, 0);
    assert_eq!(
        (recovered.remaining, recovered.maximum),
        (broken.maximum, broken.maximum)
    );
    assert!(matches!(
        guard_row(&fight),
        Some(GaugeRow::Bar { fill, length, .. })
            if fill.px(100) == 100 && length.px(100) < full_length.px(100)
    ));

    // Force timeout, then let the native round transition run normally.
    let meter = fight.machine().era_fighters()[1].super_meter;
    let mut snapshot = fight.save_netplay_snapshot(0);
    let round = snapshot.machine.work[ROUND_COUNTER];
    let time = (TIME_DIGITS - 0xff8000) as usize;
    snapshot.machine.work[time..time + 3].fill(0);
    fight.restore_netplay_snapshot(&snapshot);
    for _ in 0..1500 {
        fight.step(&assets, IDLE, IDLE, false);
        if fight.machine().work_ram()[ROUND_COUNTER] != round
            && flow_position(fight.machine()) == FIGHT_LOOP
        {
            break;
        }
    }
    assert_ne!(fight.machine().work_ram()[ROUND_COUNTER], round);
    assert_eq!(flow_position(fight.machine()), FIGHT_LOOP);
    assert_eq!(guard(&fight), full);
    assert_eq!(fight.machine().era_fighters()[1].super_meter, meter);
}

#[test]
fn v_ism_carries_meter_and_resets_the_active_combo() {
    let Some((assets, mut fight)) =
        start_fight(GaugeMode::EraDefault, Revision::Sfz3, Loadout::VIsm)
    else {
        return;
    };
    let full_guard = guard(&fight);
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[1].super_meter = 96;
    fight.restore_netplay_snapshot(&snapshot);
    let mut custom = pad(0, Some(LP));
    custom.buttons[3] = true;
    fight.step(&assets, IDLE, custom, false);
    assert!(fight.machine().era_fighters()[1].custom_combo_active());

    let mut snapshot = fight.save_netplay_snapshot(0);
    let round = snapshot.machine.work[ROUND_COUNTER];
    let time = (TIME_DIGITS - 0xff8000) as usize;
    snapshot.machine.work[time..time + 3].fill(0);
    fight.restore_netplay_snapshot(&snapshot);
    let mut carried = None;
    for _ in 0..1500 {
        let before = fight.machine().era_fighters()[1];
        fight.step(&assets, IDLE, IDLE, false);
        let after = fight.machine().era_fighters()[1];
        if before.custom_combo_active() && !after.custom_combo_active() {
            assert_eq!(after.super_meter, before.super_meter);
            carried = Some(after.super_meter);
        }
        if fight.machine().work_ram()[ROUND_COUNTER] != round
            && flow_position(fight.machine()) == FIGHT_LOOP
        {
            break;
        }
    }
    assert_ne!(fight.machine().work_ram()[ROUND_COUNTER], round);
    assert_eq!(flow_position(fight.machine()), FIGHT_LOOP);
    let state = fight.machine().era_fighters()[1];
    assert!(!state.custom_combo_active());
    assert!(state.super_meter > 0);
    assert_eq!(Some(state.super_meter), carried);
    assert_eq!(guard(&fight), full_guard);
}

#[test]
fn blocked_hadoken_drains_guard_power_on_the_ground_and_in_the_air() {
    for airborne in [false, true] {
        let Some((assets, mut fight)) =
            start_fight(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
        else {
            return;
        };
        let initial = guard(&fight);
        let health = fight.machine().era_fighters()[1].sub_hp;
        for frame in 0..100 {
            let attacker = match frame {
                0..3 => pad(DOWN, None),
                3..6 => pad(DOWN | RIGHT, None),
                6..9 => pad(RIGHT, Some(LP)),
                _ => IDLE,
            };
            let defender = match (airborne, frame) {
                (true, 0..20) => IDLE,
                (true, 20) => pad(UP, None),
                _ => pad(RIGHT, None),
            };
            fight.step(&assets, attacker, defender, false);
            if fight.machine().era_fighters()[1].sub_hp < health {
                break;
            }
        }
        assert_eq!(
            fight.machine().era_fighters()[1].sub_hp,
            health - 20,
            "chip damage"
        );
        assert!(
            guard(&fight).remaining < initial.remaining,
            "airborne={airborne}"
        );
        assert_eq!(guard(&fight).maximum, initial.maximum);
    }
}

#[test]
fn delayed_block_input_rolls_back_to_the_same_guard_crush() {
    let Some((assets, mut staged)) =
        start_fight(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    let (mut reference, impact) = pressure_to_break(&mut staged, &assets);
    let mut predicted = reference.clone();
    let before = guard(&reference);
    let health = reference.machine().era_fighters()[1].sub_hp;
    let mut rollback = RollbackEngine::new(0, 0, 8);
    const DELAY: u32 = 4;
    const FRAMES: u32 = 64;
    let remote = |frame: u32| {
        if frame >= impact.saturating_sub(2) {
            pad(RIGHT, None)
        } else {
            IDLE
        }
    };
    let mut saw_prediction_miss = false;
    for tick in 0..FRAMES + DELAY {
        if let Some(frame) = tick.checked_sub(DELAY) {
            rollback.apply_remote_inputs(
                &mut predicted,
                &assets,
                frame,
                &[encode_inputs(remote(frame))],
            );
        }
        if tick < FRAMES {
            reference.step(&assets, hard_kick(tick), remote(tick), false);
            rollback
                .step(&mut predicted, &assets, hard_kick(tick))
                .expect("under the rollback limit");
            if !saw_prediction_miss
                && guard(&reference).crush_left != 0
                && guard(&predicted).crush_left == 0
            {
                assert_eq!(guard(&predicted).maximum, before.maximum);
                assert!(guard(&predicted).remaining >= before.remaining);
                assert!(predicted.machine().era_fighters()[1].sub_hp < health);
                saw_prediction_miss = true;
            }
        }
    }
    assert!(
        saw_prediction_miss,
        "the late block input changed the contact"
    );
    assert_eq!(rollback.confirmed_inputs().len(), FRAMES as usize);
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
    assert_eq!(guard(&predicted), guard(&reference));
    assert_eq!(guard(&reference).maximum, guard(&staged).maximum);
}
