//! Street Fighter III inputs and contact on the real 1987 fight loop.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::{Fight, FightFrame};
use sf_engine::front::PlayerInputs;
use sf_video::callouts::{CalloutKind, Side};
use sf_video::era_hud::{Fill, GaugeRow};
use sf_video::romset::Assets;
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, LP, RIGHT, UP};

const MP: usize = 1;
const LK: usize = 3;
const MK: usize = 4;
const QCF: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT];
const QCF_TWICE: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];

fn directions_for(direction: u8, side: usize) -> u8 {
    if side == 0 {
        direction
    } else {
        (direction & !3) | ((direction & RIGHT) << 1) | ((direction & LEFT) >> 1)
    }
}

fn inputs(direction: u8, buttons: &[usize]) -> PlayerInputs {
    let mut input = pad(direction, None);
    for &button in buttons {
        input.buttons[button] = true;
    }
    input
}

fn command(fight: &mut Fight, assets: &Assets, side: usize, motion: &[u8], buttons: &[usize]) {
    for (index, &direction) in motion.iter().enumerate() {
        let mut pads = [PlayerInputs::default(); 2];
        let pressed = if index + 1 == motion.len() {
            buttons
        } else {
            &[]
        };
        pads[side] = inputs(directions_for(direction, side), pressed);
        step(fight, assets, pads[0], pads[1]);
    }
}

fn step(fight: &mut Fight, assets: &Assets, p1: PlayerInputs, p2: PlayerInputs) -> FightFrame {
    fight.step(assets, p1, p2, false)
}

fn idle(fight: &mut Fight, assets: &Assets, frames: usize) {
    for _ in 0..frames {
        step(
            fight,
            assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
        );
    }
}

fn meter(fight: &Fight, side: usize) -> u8 {
    fight.machine().era_fighters()[side].super_meter
}

fn hp(fight: &Fight, side: usize) -> u16 {
    fight.machine().era_fighters()[side].sub_hp
}

fn set_meter(fight: &mut Fight, side: usize, amount: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[side].super_meter = amount;
    fight.restore_netplay_snapshot(&snapshot);
}

fn approach(fight: &mut Fight, assets: &Assets) {
    for _ in 0..100 {
        let telemetry = fight.machine().diagnostic_telemetry();
        if telemetry.p2.center_x - telemetry.p1.center_x <= 38 {
            idle(fight, assets, 8);
            return;
        }
        step(fight, assets, pad(RIGHT, None), PlayerInputs::default());
    }
    panic!("fighters never reached close range");
}

#[test]
fn selectable_super_arts_spend_one_stock_and_keep_their_own_bars() {
    for revision in [Revision::Sf3Ng, Revision::Sf3TwoI, Revision::Sf3ThreeS] {
        for (art, ryu, ken, slot, ken_button) in [
            (
                Loadout::Art1,
                (112, 2, "SHINKUU HADOKEN"),
                (96, 2, "SHORYUREPPA"),
                0x29,
                LP,
            ),
            (
                Loadout::Art2,
                (128, 1, "SHIN SHORYUKEN"),
                (112, 1, "SHINRYUKEN"),
                0x2a,
                LK,
            ),
            (
                Loadout::Art3,
                (96, 1, "DENJIN HADOKEN"),
                (80, 3, "SHIPPU JINRAIKYAKU"),
                0x2b,
                LK,
            ),
        ] {
            let Some(select) = local_select_with_p1(GaugeMode::EraDefault, revision, art) else {
                return;
            };
            let session = local_fight(&select, revision, art);
            let base = session.fight().unwrap();
            for (side, (length, mut stocks, name), button) in [(0, ryu, LP), (1, ken, ken_button)] {
                if revision == Revision::Sf3Ng {
                    stocks = if side == 1 && art != Loadout::Art1 {
                        2
                    } else {
                        1
                    };
                }
                let mut fight = base.clone();
                set_meter(&mut fight, side, length * stocks);
                let gauge = fight.machine().era_fighters()[side].gauge().unwrap();
                let GaugeRow::Level {
                    level,
                    length: art_length,
                    ..
                } = gauge.main
                else {
                    panic!("{name} should use a numbered Super Art bar");
                };
                assert_eq!(level, stocks);
                assert_eq!(art_length.px(128), usize::from(length));
                command(&mut fight, session.assets(), side, QCF_TWICE, &[button]);
                assert_eq!(
                    fight.fight_read().fighters[side].move_id,
                    slot,
                    "{revision:?}: {name}"
                );
                assert_eq!(meter(&fight, side), length * (stocks - 1));
                assert!(fight.machine().era_super_freeze() > 0);
                assert!(fight
                    .callouts()
                    .frame()
                    .side(Side::BOTH[side])
                    .words
                    .contains(&Some(CalloutKind::Super { name }),));
            }
        }
    }
}

#[test]
fn each_canonical_super_art_lands_multiple_native_hits() {
    for (art, cost, slot, ken_button) in [
        (Loadout::Art1, [112, 96], 0x29, LP),
        (Loadout::Art2, [128, 112], 0x2a, LK),
        (Loadout::Art3, [96, 80], 0x2b, LK),
    ] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, art)
        else {
            return;
        };
        let session = local_fight(&select, Revision::Sf3ThreeS, art);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        approach(&mut base, assets);
        for (side, button) in [(0, LP), (1, ken_button)] {
            let mut fight = base.clone();
            set_meter(&mut fight, side, cost[side]);
            command(&mut fight, assets, side, QCF_TWICE, &[button]);
            assert_eq!(fight.fight_read().fighters[side].move_id, slot);
            let defender = 1 - side;
            let mut previous_health = hp(&fight, defender);
            let mut hits = 0;
            for _ in 0..150 {
                idle(&mut fight, assets, 1);
                let health = hp(&fight, defender);
                hits += usize::from(health < previous_health);
                previous_health = health;
            }
            assert!(hits > 1, "{art:?} for side {side} landed {hits} hits");
            if side == 0 && art == Loadout::Art1 {
                assert_eq!(hits, 5, "a point-blank Shinkuu Hadoken lands all five hits");
            }
        }
    }
}

#[test]
fn super_art_stocks_and_partial_meter_carry_into_the_next_round() {
    use sf_engine::fighter::{P2_STRUCT, WORK_BASE};
    use sf_engine::tasks::TIME_DIGITS;

    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let mut fight = session.fight().unwrap().clone();
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = 140;
    snapshot.machine.toggle_state.era.fighters[1].sub_hp = 4;
    let p2_health = (P2_STRUCT + 6 - WORK_BASE) as usize;
    snapshot.machine.work[p2_health..p2_health + 2].copy_from_slice(&[1, 0]);
    let timer = (TIME_DIGITS - WORK_BASE) as usize;
    snapshot.machine.work[timer..timer + 3].fill(0);
    fight.restore_netplay_snapshot(&snapshot);

    assert!(
        (0..900).any(|_| {
            idle(&mut fight, session.assets(), 1);
            fight.machine().work_ram()[0x400a] == 1 && hp(&fight, 1) > 4
        }),
        "round two starts with the opponent's health restored"
    );
    assert_eq!(meter(&fight, 0), 140);
    assert_eq!(
        fight.machine().era_fighters()[0].gauge().unwrap().main,
        GaugeRow::level(1, 2, Fill::of(28, 112)).with_length(Fill::of(112, 128))
    );
}

#[test]
fn unified_sf3_ex_uses_one_standard_stock() {
    let Some(select) = local_select_with_p1(GaugeMode::Unified, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let mut fight = session.fight().unwrap().clone();
    set_meter(&mut fight, 0, 96);
    command(&mut fight, session.assets(), 0, QCF, &[LP, MP]);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x24);
    assert_eq!(meter(&fight, 0), 48);
}

#[test]
fn unified_sf3_super_arts_use_one_two_or_three_stocks() {
    for (art, slot, cost, ken_button) in [
        (Loadout::Art1, 0x29, 48, LP),
        (Loadout::Art2, 0x2a, 96, LK),
        (Loadout::Art3, 0x2b, 144, LK),
    ] {
        let Some(select) = local_select_with_p1(GaugeMode::Unified, Revision::Sf3ThreeS, art)
        else {
            return;
        };
        let session = local_fight(&select, Revision::Sf3ThreeS, art);
        let base = session.fight().unwrap();
        for (side, button) in [(0, LP), (1, ken_button)] {
            let mut short = base.clone();
            set_meter(&mut short, side, cost - 1);
            command(&mut short, session.assets(), side, QCF_TWICE, &[button]);
            assert_ne!(short.fight_read().fighters[side].move_id, slot, "{art:?}");
            assert_eq!(meter(&short, side), cost - 1);

            let mut full = base.clone();
            set_meter(&mut full, side, 144);
            command(&mut full, session.assets(), side, QCF_TWICE, &[button]);
            assert_eq!(full.fight_read().fighters[side].move_id, slot);
            assert_eq!(meter(&full, side), 144 - cost, "{art:?}");
        }
    }
}

#[test]
fn high_and_low_parry_use_the_correct_tap_and_cancel_damage() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    approach(&mut base, assets);
    for (attack, direction, wrong) in [
        (pad(0, Some(LP)), RIGHT, DOWN),
        (pad(DOWN, Some(LK)), DOWN, RIGHT),
    ] {
        let starting_hp = hp(&base, 0);
        let mut probe = base.clone();
        let impact = (0usize..30)
            .find(|&frame| {
                let p2 = if frame == 0 {
                    attack
                } else {
                    PlayerInputs::default()
                };
                step(&mut probe, assets, PlayerInputs::default(), p2);
                hp(&probe, 0) < starting_hp
            })
            .expect("the attack would hit");

        for (tap, should_parry) in [(direction, true), (wrong, false)] {
            let mut fight = base.clone();
            for frame in 0..=impact {
                let p2 = if frame == 0 {
                    attack
                } else {
                    PlayerInputs::default()
                };
                let p1 = if frame == impact.saturating_sub(1) {
                    pad(tap, None)
                } else {
                    PlayerInputs::default()
                };
                step(&mut fight, assets, p1, p2);
            }
            if should_parry {
                assert_eq!(
                    hp(&fight, 0),
                    starting_hp,
                    "tap {tap:#04x} for {attack:?} at impact {impact}"
                );
                assert_eq!(fight.fight_read().fighters[0].move_id, 0x2e);
                assert!(!fight.fight_read().fighters[0].blockstun);
                assert!(meter(&fight, 0) > 0);
                assert_eq!(fight.machine().era_super_freeze(), 6);
                idle(&mut fight, assets, 6);
                let gate_free = sf_engine::front::show::gate_free;
                assert!(
                    (0..12).any(|_| {
                        idle(&mut fight, assets, 1);
                        let [defender, attacker] = fight.fight_read().fighters;
                        gate_free(&defender) && !gate_free(&attacker)
                    }),
                    "the parry recovers ahead of the attacking move"
                );
            } else {
                assert!(hp(&fight, 0) < starting_hp);
            }
        }
    }
}

#[test]
fn projectile_parry_has_a_three_frame_window() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    command(&mut base, assets, 1, QCF, &[LP]);
    let before = hp(&base, 0);
    let mut probe = base.clone();
    let impact = (0..80)
        .find(|_| {
            idle(&mut probe, assets, 1);
            hp(&probe, 0) < before
        })
        .expect("normal Hadoken hits");
    assert!(impact >= 7, "space to compare early and late taps");

    for (age, expected) in [(1, true), (7, false)] {
        let mut fight = base.clone();
        for frame in 0..=impact + 7 {
            let p1 = if frame == impact - age {
                pad(RIGHT, None)
            } else {
                PlayerInputs::default()
            };
            step(&mut fight, assets, p1, PlayerInputs::default());
        }
        assert_eq!(hp(&fight, 0) == before, expected, "tap age {age}");
        assert_eq!(meter(&fight, 0) > 0, expected);
    }
}

#[test]
fn air_parry_deflects_a_jump_kick_without_ending_the_jump() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    approach(&mut base, assets);
    let floor_center = base.machine().diagnostic_telemetry().p1.center_y;
    let before = hp(&base, 0);
    let p2 = |frame| match frame {
        0 => pad(UP, None),
        10 => pad(0, Some(MK)),
        _ => PlayerInputs::default(),
    };
    let mut probe = base.clone();
    let impact = (0..50)
        .find(|&frame| {
            let p1 = if frame == 0 {
                pad(UP, None)
            } else {
                PlayerInputs::default()
            };
            step(&mut probe, assets, p1, p2(frame));
            hp(&probe, 0) < before
        })
        .expect("the opponent's jump kick hits the airborne defender");

    for frame in 0..=impact {
        let p1 = if frame == 0 {
            pad(UP, None)
        } else if frame == impact - 1 {
            pad(RIGHT, None)
        } else {
            PlayerInputs::default()
        };
        step(&mut base, assets, p1, p2(frame));
    }
    assert_eq!(hp(&base, 0), before);
    assert!(meter(&base, 0) > 0);
    assert!(base.machine().diagnostic_telemetry().p1.center_y < floor_center);
    idle(&mut base, assets, 80);
    assert_eq!(
        base.machine().diagnostic_telemetry().p1.center_y,
        floor_center
    );
}

#[test]
fn overhead_crosses_crouch_guard_and_ex_spends_partial_super_meter() {
    for revision in [Revision::Sf3Ng, Revision::Sf3TwoI, Revision::Sf3ThreeS] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, revision, Loadout::Art1)
        else {
            return;
        };
        let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
        let assets = session.assets();
        let mut fight = session.fight().unwrap().clone();
        set_meter(&mut fight, 0, 40);
        command(&mut fight, assets, 0, QCF, &[LP, MP]);
        let can_ex = revision != Revision::Sf3Ng;
        assert_eq!(
            fight.fight_read().fighters[0].move_id == 0x24,
            can_ex,
            "{revision:?}"
        );
        assert_eq!(meter(&fight, 0), if can_ex { 0 } else { 40 });

        let mut fight = session.fight().unwrap().clone();
        approach(&mut fight, assets);
        let before = hp(&fight, 1);
        for _ in 0..3 {
            step(
                &mut fight,
                assets,
                PlayerInputs::default(),
                pad(DOWN | RIGHT, None),
            );
        }
        step(
            &mut fight,
            assets,
            inputs(0, &[MP, MK]),
            pad(DOWN | RIGHT, None),
        );
        assert_eq!(fight.fight_read().fighters[0].move_id, 0x2f);
        for _ in 0..26 {
            step(
                &mut fight,
                assets,
                PlayerInputs::default(),
                pad(DOWN | RIGHT, None),
            );
        }
        assert!(
            hp(&fight, 1) < before,
            "crouching guard must lose to the hop"
        );
    }
}

#[test]
fn denjin_grows_while_held_and_bypasses_standing_guard() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art3)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    set_meter(&mut base, 0, 96);
    command(&mut base, assets, 0, QCF_TWICE, &[LP]);
    assert_eq!(base.fight_read().fighters[0].move_id, 0x2b);
    for (hold, hits) in [(0, 2), (80, 5)] {
        let mut fight = base.clone();
        let before = hp(&fight, 1);
        let mut largest = 0;
        for frame in 0..145 {
            let p1 = if frame < hold {
                pad(RIGHT, Some(LP))
            } else {
                PlayerInputs::default()
            };
            step(&mut fight, assets, p1, pad(RIGHT, None));
            for projectile in fight.machine().toggle_state().era.projectiles() {
                if projectile.move_id == 0x2b {
                    assert!(projectile.unblockable);
                    largest = largest.max(projectile.hits_remaining);
                }
            }
        }
        assert_eq!(largest, hits, "charge held {hold} frames");
        assert!(before.saturating_sub(hp(&fight, 1)) >= 120);
    }
}

#[test]
fn third_strike_red_parries_a_second_projectile_hit_from_blockstun() {
    for revision in [Revision::Sf3TwoI, Revision::Sf3ThreeS] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, revision, Loadout::Art1)
        else {
            return;
        };
        let session = local_fight(&select, Revision::Sf3ThreeS, Loadout::Art1);
        let assets = session.assets();
        let mut fight = session.fight().unwrap().clone();
        set_meter(&mut fight, 1, 40);
        for (frame, &direction) in QCF.iter().enumerate() {
            let p2 = inputs(
                directions_for(direction, 1),
                if frame == 2 { &[LP, MP] } else { &[] },
            );
            step(&mut fight, assets, pad(LEFT, None), p2);
        }
        assert_eq!(fight.fight_read().fighters[1].move_id, 0x24);
        let before = hp(&fight, 0);
        assert!((0..80).any(|_| {
            step(&mut fight, assets, pad(LEFT, None), PlayerInputs::default());
            hp(&fight, 0) < before
        }));
        assert!(
            fight.fight_read().fighters[0].blockstun,
            "first hit was guarded"
        );
        let after_first = hp(&fight, 0);
        let mut probe = fight.clone();
        let next_hit = (0..12)
            .find(|_| {
                step(&mut probe, assets, pad(LEFT, None), PlayerInputs::default());
                hp(&probe, 0) < after_first
            })
            .expect("the next fireball hit would chip the guard");
        assert!(next_hit >= 2);

        for lead in [1, 2] {
            let mut attempt = fight.clone();
            for frame in 0..=next_hit {
                let direction = if frame + lead == next_hit { RIGHT } else { LEFT };
                step(
                    &mut attempt,
                    assets,
                    pad(direction, None),
                    PlayerInputs::default(),
                );
            }
            let red_enabled = revision == Revision::Sf3ThreeS && lead == 1;
            assert_eq!(
                hp(&attempt, 0) == after_first,
                red_enabled,
                "{revision:?} lead {lead}"
            );
            assert_eq!(attempt.fight_read().fighters[0].blockstun, !red_enabled);
            if red_enabled {
                assert_eq!(attempt.fight_read().fighters[0].move_id, 0x2e);
                assert!(meter(&attempt, 0) >= 8);
            }
        }
    }
}

#[test]
fn airborne_ex_tatsumaki_keeps_jump_height_and_lands() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3TwoI, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut fight = session.fight().unwrap().clone();
    set_meter(&mut fight, 0, 40);
    let floor_center = fight.machine().diagnostic_telemetry().p1.center_y;
    step(&mut fight, assets, pad(UP, None), PlayerInputs::default());
    idle(&mut fight, assets, 11);
    let before = fight.machine().diagnostic_telemetry().p1.center_y;
    assert!(before < floor_center);
    command(&mut fight, assets, 0, &[DOWN, DOWN | LEFT, LEFT], &[LK, MK]);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x21);
    assert_eq!(meter(&fight, 0), 0);
    assert!(fight.machine().diagnostic_telemetry().p1.center_y < floor_center);
    idle(&mut fight, assets, 90);
    assert_eq!(fight.fight_read().fighters[0].move_id, 1);
    assert_eq!(
        fight.machine().diagnostic_telemetry().p1.center_y,
        floor_center
    );
}

#[test]
fn ex_blade_kick_rebounds_a_launched_target_at_the_wall() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut fight = session.fight().unwrap().clone();
    for _ in 0..110 {
        step(&mut fight, assets, pad(RIGHT, None), pad(RIGHT, None));
    }
    approach(&mut fight, assets);
    set_meter(&mut fight, 0, 40);
    let before = hp(&fight, 1);
    command(
        &mut fight,
        assets,
        0,
        &[LEFT, LEFT | DOWN, DOWN, DOWN | RIGHT, RIGHT],
        &[LK, MK],
    );
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x2c);
    assert_eq!(meter(&fight, 0), 0);
    let mut edge = 0;
    let mut rebound = false;
    for _ in 0..42 {
        idle(&mut fight, assets, 1);
        let x = fight.machine().diagnostic_telemetry().p2.center_x;
        if hp(&fight, 1) < before {
            edge = edge.max(x);
            rebound |= edge - x > 10;
        }
    }
    assert!(hp(&fight, 1) < before, "EX blade kick made contact");
    assert!(
        rebound,
        "the target returned from the wall during the launch"
    );
}

#[test]
fn a_late_parry_input_replays_the_same_freeze_health_and_meter() {
    use sf_engine::netplay::{encode_inputs, RollbackEngine};

    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut reference = session.fight().unwrap().clone();
    approach(&mut reference, assets);
    let before = hp(&reference, 0);
    let local = |frame| {
        if frame == 0 {
            pad(0, Some(LP))
        } else {
            PlayerInputs::default()
        }
    };
    let mut probe = reference.clone();
    let impact = (0..30u32)
        .find(|&frame| {
            step(&mut probe, assets, PlayerInputs::default(), local(frame));
            hp(&probe, 0) < before
        })
        .expect("the jab reaches the opponent");
    let remote = |frame| {
        if frame == impact.saturating_sub(1) {
            pad(RIGHT, None)
        } else {
            PlayerInputs::default()
        }
    };
    let mut predicted = reference.clone();
    let mut net = RollbackEngine::new(1, 0, 8);
    const DELAY: u32 = 4;
    const FRAMES: u32 = 32;
    for frame in 0..FRAMES + DELAY {
        if let Some(confirmed) = frame.checked_sub(DELAY) {
            net.apply_remote_inputs(
                &mut predicted,
                assets,
                confirmed,
                &[encode_inputs(remote(confirmed))],
            );
        }
        if frame < FRAMES {
            step(&mut reference, assets, remote(frame), local(frame));
            net.step(&mut predicted, assets, local(frame))
                .expect("prediction stays within eight frames");
        }
    }
    assert_eq!(hp(&reference, 0), before);
    assert_eq!(meter(&reference, 0), 8);
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
}

#[test]
fn late_sf3_ex_and_super_art_inputs_replay_the_same_meter_and_projectiles() {
    use sf_engine::netplay::{encode_inputs, RollbackEngine};

    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut reference = session.fight().unwrap().clone();
    set_meter(&mut reference, 0, 152);
    let mut predicted = reference.clone();
    let mut net = RollbackEngine::new(1, 0, 8);
    const DELAY: u32 = 4;
    const SUPER_START: u32 = 90;
    const FRAMES: u32 = 180;
    let remote = |frame: u32| {
        if let Some(&direction) = QCF.get(frame as usize) {
            return inputs(direction, if frame == 2 { &[LP, MP] } else { &[] });
        }
        if let Some(index) = frame.checked_sub(SUPER_START).map(|index| index as usize) {
            if let Some(&direction) = QCF_TWICE.get(index) {
                return inputs(direction, if index + 1 == QCF_TWICE.len() { &[LP] } else { &[] });
            }
        }
        PlayerInputs::default()
    };
    let mut checksums = Vec::new();
    let mut ex_projectile = false;
    let mut art_projectile = false;
    for tick in 0..FRAMES + DELAY {
        if let Some(frame) = tick.checked_sub(DELAY) {
            net.apply_remote_inputs(&mut predicted, assets, frame, &[encode_inputs(remote(frame))]);
            assert_eq!(net.confirmed_checksum(frame), Some(checksums[frame as usize]));
        }
        if tick >= FRAMES {
            continue;
        }
        let before = meter(&reference, 0);
        step(&mut reference, assets, remote(tick), PlayerInputs::default());
        net.step(&mut predicted, assets, PlayerInputs::default())
            .expect("prediction stays within eight frames");
        if tick == 2 {
            assert_eq!(reference.fight_read().fighters[0].move_id, 0x24);
            assert_eq!(before - meter(&reference, 0), 40);
        }
        if tick == SUPER_START + QCF_TWICE.len() as u32 - 1 {
            assert_eq!(reference.fight_read().fighters[0].move_id, 0x29);
            assert_eq!(before - meter(&reference, 0), 112);
        }
        for projectile in reference.machine().toggle_state().era.projectiles() {
            ex_projectile |= projectile.move_id == 0x24 && projectile.hits_remaining == 2;
            art_projectile |= projectile.move_id == 0x29 && projectile.hits_remaining == 5;
        }
        checksums.push(reference.state_checksum());
    }
    assert!(ex_projectile && art_projectile, "both multi-hit projectiles were live");
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
}

#[test]
fn training_parry_uses_a_forward_tap_and_real_contact() {
    use sf_engine::front::FrameInputs;
    use sf_engine::moves::Strength;
    use sf_engine::roster::TournamentFighter::{Ken, Ryu};

    for fighter in [Ryu, Ken] {
        let Some(mut session) =
            support::training_fight_with_fighter(fighter, Revision::Sf3ThreeS, Loadout::Art1)
        else {
            return;
        };
        support::assert_training_show(&mut session, "PARRY", 0x2e);
        let entry = *session.coach().unwrap().entry();
        assert_eq!(entry.input.words(), "F");
        assert_eq!(entry.input.strip_steps(), ["F"]);
        assert_eq!(entry.input.frames(true, Strength::Medium), [pad(RIGHT, None)]);
        assert_eq!(entry.input.frames(false, Strength::Medium), [pad(LEFT, None)]);

        for attacking in [false, true] {
            assert!((0..120).any(|_| {
                if session.fight().unwrap().fight_read().fighters[1].attacking() == attacking {
                    return true;
                }
                session.step(FrameInputs::default());
                false
            }), "the dummy reaches attack state {attacking}");
        }
        let positions = session.fight_machine().unwrap().diagnostic_telemetry();
        let direction = if positions.p1.center_x < positions.p2.center_x { RIGHT } else { LEFT };
        let health = hp(session.fight().unwrap(), 0);
        session.step(FrameInputs { p1: pad(direction, None), ..FrameInputs::default() });
        assert!((0..12).any(|_| {
            if session.coach().unwrap().successes() == 1 {
                return true;
            }
            session.step(FrameInputs::default());
            false
        }), "Try records {fighter:?}'s timed parry");
        assert_eq!(hp(session.fight().unwrap(), 0), health);
    }
}

#[test]
fn training_moves_show_and_try_the_selected_super_art_with_fill_meters() {
    use sf_engine::config::MatchConfig;
    use sf_engine::front::cabinet::flow_position;
    use sf_engine::front::pause::PauseBox;
    use sf_engine::front::settings::Settings;
    use sf_engine::front::{FrameInputs, GameSession, MenuItem, Transition};
    use sf_engine::moves::{MoveKind, Strength};
    use sf_video::romset::load_assets;

    if !std::path::Path::new(support::ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let tap = |session: &mut GameSession, input: FrameInputs| {
        let transition = session.step(input).transition;
        session.step(FrameInputs::default());
        transition
    };
    let key = |direction, button| FrameInputs {
        p1: pad(direction, button),
        ..FrameInputs::default()
    };
    let start = || {
        let mut input = FrameInputs::default();
        input.menu.start = true;
        input
    };
    let choose = |session: &mut GameSession, item: MenuItem| {
        for _ in 0..8 {
            if session.menu_item() == Some(item) {
                tap(session, key(0, Some(LP)));
                return;
            }
            tap(session, key(DOWN, None));
        }
        panic!("{item:?} was absent on {}", session.screen());
    };

    for (ken, art, name, full) in [
        (false, 0, "SHINKUU HADOKEN", 224),
        (false, 1, "SHIN SHORYUKEN", 128),
        (false, 2, "DENJIN HADOKEN", 96),
        (true, 0, "SHORYUREPPA", 192),
        (true, 1, "SHINRYUKEN", 112),
        (true, 2, "SHIPPU JINRAIKYAKU", 240),
    ] {
        let assets = load_assets(support::ROMS, support::MANIFEST).unwrap();
        let mut session =
            GameSession::new(assets, MatchConfig::definitive()).with_settings(Settings {
                fill_meters: true,
                ..Settings::default()
            });
        for _ in 0..200 {
            session.step(FrameInputs::default());
        }
        assert_eq!(tap(&mut session, start()), Some(Transition::OpenMainMenu));
        choose(&mut session, MenuItem::SinglePlayer);
        choose(&mut session, MenuItem::Training);
        choose(&mut session, MenuItem::TrainingDummy);
        if ken {
            tap(&mut session, key(RIGHT, None));
        }
        assert_eq!(
            tap(&mut session, key(0, Some(LP))),
            Some(Transition::StyleOpened)
        );
        for _ in 0..3 {
            tap(&mut session, key(RIGHT, None));
        }
        if art > 0 {
            tap(&mut session, key(UP, None));
            for _ in 0..art {
                tap(&mut session, key(RIGHT, None));
            }
        }
        tap(&mut session, key(0, Some(LP)));
        if tap(&mut session, key(0, Some(LP))) == Some(Transition::StyleOpened) {
            tap(&mut session, key(0, Some(LP)));
        }
        for _ in 0..60 {
            if session.screen() == "country select" {
                break;
            }
            session.step(FrameInputs::default());
        }
        assert_eq!(session.screen(), "country select");
        tap(&mut session, key(0, Some(LP)));
        assert!((0..1500).any(|_| {
            session.step(FrameInputs::default());
            session.fight_machine().map(flow_position) == Some(0x3eafa)
        }));
        for _ in 0..10 {
            session.step(FrameInputs::default());
        }
        assert_eq!(
            session.fight_machine().unwrap().era_fighters()[0].super_meter,
            full
        );

        assert_eq!(tap(&mut session, start()), Some(Transition::Paused));
        let moves_row = PauseBox::open(true)
            .items()
            .iter()
            .position(|&item| item == MenuItem::Moves)
            .unwrap();
        for _ in 0..moves_row {
            tap(&mut session, key(DOWN, None));
        }
        tap(&mut session, key(0, Some(LP)));
        assert!(
            (0..16).any(|_| {
                let selected = session.move_list().unwrap().selected().unwrap();
                if selected.kind == MoveKind::Super {
                    assert_eq!(selected.name, name);
                    true
                } else {
                    tap(&mut session, key(DOWN, None));
                    false
                }
            }),
            "the selected Super Art is in MOVES"
        );
        tap(&mut session, key(0, Some(LP)));
        tap(&mut session, key(0, Some(LP)));

        assert!(
            (0..220).any(|_| {
                session.step(FrameInputs::default());
                session.coach().is_some()
            }),
            "{name} Show completes and starts Try"
        );
        let entry = *session.coach().unwrap().entry();
        assert_eq!(entry.name, name);
        for p1 in entry.input.frames(true, Strength::Light) {
            session.step(FrameInputs {
                p1,
                ..FrameInputs::default()
            });
        }
        assert_eq!(
            session.coach().unwrap().successes(),
            1,
            "{name} Try records a real activation"
        );
    }
}
