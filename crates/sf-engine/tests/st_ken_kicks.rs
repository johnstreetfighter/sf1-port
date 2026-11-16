//! World Super Turbo commands replayed through the live controller path.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::front::PlayerInputs;
use sf_engine::roster::TournamentFighter::Ken;
use support::{
    local_fight, local_select_with_fighter, local_select_with_p1, pad, DOWN, LEFT, RIGHT, UP,
};

const LK: usize = 3;
const HK: usize = 5;
const MOVES: [(u8, &[u8], u8, u8, u8, u8); 3] = [
    (0x2a, &[2, 3, 6], 7, 36, 27, 40),
    (0x2b, &[6, 3, 2], 16, 35, 36, 49),
    (0x2c, &[4, 1, 2, 3, 6], 12, 34, 34, 47),
];

fn directional_pad(side: usize, direction: u8, button: Option<usize>) -> PlayerInputs {
    let (forward, back) = if side == 0 {
        (RIGHT, LEFT)
    } else {
        (LEFT, RIGHT)
    };
    let mut bits = 0;
    if matches!(direction, 1 | 4 | 7) {
        bits |= back;
    }
    if matches!(direction, 3 | 6 | 9) {
        bits |= forward;
    }
    if matches!(direction, 1..=3) {
        bits |= DOWN;
    }
    if matches!(direction, 7..=9) {
        bits |= UP;
    }
    pad(bits, button)
}

fn step(
    fight: &mut Fight,
    session: &sf_engine::front::GameSession,
    side: usize,
    direction: u8,
    button: Option<usize>,
) {
    let mut pads = [PlayerInputs::default(); 2];
    pads[side] = directional_pad(side, direction, button);
    fight.step(session.assets(), pads[0], pads[1], false);
}

fn set_gap(fight: &mut Fight, gap: i32) {
    let telemetry = fight.machine().diagnostic_telemetry();
    let mut snapshot = fight.save_netplay_snapshot(0);
    for (fighter, center) in [(telemetry.p1, 192 - gap / 2), (telemetry.p2, 192 + gap / 2)] {
        let ram = &mut snapshot.machine.state.objectram;
        let index = usize::from(fighter.entity_offset);
        let rows = usize::from(ram[index + 0xc]) + 1;
        let cols = usize::from(ram[index + 0xd]) + 1;
        for cell in 0..rows * cols {
            let offset = index + cell * 0x40 + 6;
            let x = u16::from_be_bytes(ram[offset..offset + 2].try_into().unwrap());
            ram[offset..offset + 2].copy_from_slice(
                &x.wrapping_add((center - fighter.center_x) as u16)
                    .to_be_bytes(),
            );
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
}

#[test]
fn world_ken_tap_and_held_kicks_preserve_action_clocks_on_both_sides() {
    let Some(ryu) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let ken =
        local_select_with_fighter(GaugeMode::EraDefault, Ken, Revision::St, Loadout::Standard)
            .unwrap();
    for (side, select) in [(0, &ken), (1, &ryu)] {
        let session = local_fight(select, Revision::St, Loadout::Standard);
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            step(&mut base, &session, side, 5, None);
        }
        set_gap(&mut base, 190);
        for (slot, motion, first, tap_exit, heel_first, heel_exit) in MOVES {
            for button in LK..=HK {
                for held in [false, true] {
                    let mut fight = base.clone();
                    let (&last, prefix) = motion.split_last().unwrap();
                    for &direction in prefix {
                        step(&mut fight, &session, side, direction, None);
                    }
                    step(&mut fight, &session, side, last, Some(button));
                    assert_eq!(
                        fight.fight_read().fighters[side].move_id,
                        slot,
                        "side {side}, {motion:?}, button {button}"
                    );
                    let expected_exit = if held { heel_exit } else { tap_exit };
                    let mut active = Vec::new();
                    let mut exit = 0;
                    for tick in 1..=expected_exit + 3 {
                        step(&mut fight, &session, side, 5, held.then_some(button));
                        let fighter = fight.fight_read().fighters[side];
                        if fighter.move_id != slot {
                            exit = tick;
                            break;
                        }
                        if fighter.attack_active {
                            active.push(tick);
                        }
                    }
                    assert_eq!(
                        active.first(),
                        Some(&first),
                        "side {side}, {motion:?}, button {button}, held {held}"
                    );
                    if held {
                        let rearmed = active.iter().copied().find(|&tick| tick >= 22);
                        assert_eq!(rearmed, Some(heel_first), "{motion:?}");
                    }
                    let telemetry = fight.machine().diagnostic_telemetry();
                    let entity = if side == 0 {
                        telemetry.p1
                    } else {
                        telemetry.p2
                    };
                    let offset = usize::from(entity.entity_offset);
                    let ram = &fight.machine().state.objectram;
                    assert_eq!(exit, expected_exit, "side {side}, {motion:?}, button {button}, held {held}. active {active:?}; state {:02x?} stream {:02x?}",
                        &ram[offset + 8..offset + 0x10], &ram[offset + 0x28..offset + 0x34]);
                }
            }
        }
    }
}

#[test]
fn world_ken_accepts_neutral_and_up_finish_and_old_keeps_normal_fallback() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    for loadout in [Loadout::Standard, Loadout::Old] {
        let session = local_fight(&select, Revision::St, loadout);
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            step(&mut base, &session, 1, 5, None);
        }
        for (slot, motion, ..) in MOVES {
            for finish in [5, 8] {
                let mut fight = base.clone();
                for &direction in motion {
                    step(&mut fight, &session, 1, direction, None);
                }
                step(&mut fight, &session, 1, finish, Some(LK));
                assert_eq!(
                    fight.fight_read().fighters[1].move_id == slot,
                    loadout == Loadout::Standard,
                    "{motion:?}, finish {finish}"
                );
            }
            let mut delayed = base.clone();
            for &direction in motion {
                step(&mut delayed, &session, 1, direction, None);
            }
            for _ in 1..10 {
                step(&mut delayed, &session, 1, 5, None);
            }
            let mut last_accepted = delayed.clone();
            step(&mut last_accepted, &session, 1, 5, Some(LK));
            assert_eq!(
                last_accepted.fight_read().fighters[1].move_id == slot,
                loadout == Loadout::Standard,
                "{motion:?}, final direction +10"
            );
            step(&mut delayed, &session, 1, 5, None);
            step(&mut delayed, &session, 1, 5, Some(LK));
            assert_ne!(
                delayed.fight_read().fighters[1].move_id,
                slot,
                "{motion:?}, final direction +11"
            );
        }
        let mut qcb = base.clone();
        for direction in [6, 3, 2, 1] {
            step(&mut qcb, &session, 1, direction, None);
        }
        step(&mut qcb, &session, 1, 4, Some(LK));
        assert_eq!(qcb.fight_read().fighters[1].move_id, 0x1b);
    }
}

#[test]
fn world_ken_held_kick_replays_from_its_branch_input() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let mut fight = session.fight().unwrap().clone();
    for _ in 0..94 {
        step(&mut fight, &session, 1, 5, None);
    }
    set_gap(&mut fight, 190);
    step(&mut fight, &session, 1, 2, None);
    step(&mut fight, &session, 1, 3, None);
    step(&mut fight, &session, 1, 6, Some(LK));
    for _ in 1..=7 {
        step(&mut fight, &session, 1, 5, Some(LK));
    }
    let snapshot = fight.save_netplay_snapshot(7);
    for _ in 8..=40 {
        step(&mut fight, &session, 1, 5, None);
    }
    assert_ne!(fight.fight_read().fighters[1].move_id, 0x2a);
    let expected = fight.state_checksum();
    fight.restore_netplay_snapshot(&snapshot);
    for tick in 8..=40 {
        step(&mut fight, &session, 1, 5, None);
        if tick == 27 {
            assert_eq!(fight.fight_read().fighters[1].move_id, 0x2a);
            assert!(fight.fight_read().fighters[1].attack_active);
        }
    }
    assert_eq!(fight.state_checksum(), expected);
}

#[test]
fn world_ken_opening_kicks_use_arcade_health_units() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let mut base = session.fight().unwrap().clone();
    for _ in 0..94 {
        step(&mut base, &session, 1, 5, None);
    }
    set_gap(&mut base, 66);
    for ((slot, motion, first, ..), damage) in MOVES.into_iter().zip([7, 2, 27]) {
        let mut fight = base.clone();
        let (&last, prefix) = motion.split_last().unwrap();
        for &direction in prefix {
            step(&mut fight, &session, 1, direction, None);
        }
        let hp = fight.machine().era_fighters()[0].sub_hp;
        step(&mut fight, &session, 1, last, Some(LK));
        assert_eq!(fight.fight_read().fighters[1].move_id, slot);
        let mut hit = None;
        for tick in 1..=first + 4 {
            step(&mut fight, &session, 1, 5, None);
            let remaining = fight.machine().era_fighters()[0].sub_hp;
            if hp != remaining {
                hit = Some((tick, hp - remaining));
                break;
            }
        }
        assert_eq!(hit, Some((first, (damage * 960 + 72) / 144)), "{motion:?}");
    }
}
