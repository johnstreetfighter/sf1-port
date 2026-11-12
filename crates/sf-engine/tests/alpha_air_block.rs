//! Alpha air block through the Style card and the translated fight loop.

#[path = "support/alpha.rs"]
mod alpha;

use alpha::{local_fight, local_select, pad, DOWN, LEFT, LP, MANIFEST, RIGHT, ROMS, UP};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::PlayerInputs;
use sf_engine::run::PowerOnMachine;
use sf_video::romset::load_assets;

const HK: usize = 5;

fn step(machine: &mut PowerOnMachine, p1: PlayerInputs, p2: PlayerInputs) {
    let [r, l, d, u] = p1.directions;
    machine.set_p1_direction(r, l, d, u);
    machine.set_p1_buttons(p1.buttons);
    let [r, l, d, u] = p2.directions;
    machine.set_p2_direction(r, l, d, u);
    machine.set_p2_buttons(p2.buttons);
    machine.step();
}

fn projectile_inputs(
    frame: usize,
    jump_frame: usize,
    direction: u8,
) -> (PlayerInputs, PlayerInputs) {
    let p1 = match frame {
        0..3 => pad(DOWN, None),
        3..6 => pad(DOWN | RIGHT, None),
        6..9 => pad(RIGHT, Some(LP)),
        _ => PlayerInputs::default(),
    };
    let p2 = if frame < jump_frame {
        PlayerInputs::default()
    } else if frame == jump_frame {
        pad(UP, None)
    } else {
        pad(direction, None)
    };
    (p1, p2)
}

fn projectile_contact(
    start: &PowerOnMachine,
    jump_frame: usize,
    direction: u8,
) -> Option<(usize, u16, u8, u8)> {
    let mut machine = start.clone();
    let health = machine.era_fighters()[1].sub_hp;
    for frame in 0..110 {
        let (p1, p2) = projectile_inputs(frame, jump_frame, direction);
        let before = machine.diagnostic_telemetry().p2.move_id;
        step(&mut machine, p1, p2);
        let after = machine.diagnostic_telemetry().p2.move_id;
        let damage = health - machine.era_fighters()[1].sub_hp;
        if damage != 0 {
            return Some((frame, damage, before, after));
        }
    }
    None
}

fn approach(machine: &mut PowerOnMachine, gap: i32) {
    for _ in 0..100 {
        let read = machine.diagnostic_telemetry();
        if read.p2.center_x - read.p1.center_x <= gap {
            for _ in 0..8 {
                step(machine, PlayerInputs::default(), PlayerInputs::default());
            }
            return;
        }
        step(machine, pad(RIGHT, None), PlayerInputs::default());
    }
    panic!("failed to approach to gap {gap}");
}

fn melee_contact(start: &PowerOnMachine, direction: u8) -> Option<(usize, u16, u8, u8, u8)> {
    let mut machine = start.clone();
    let health = machine.era_fighters()[1].sub_hp;
    for frame in 0..42 {
        let p1 = if frame == 0 {
            pad(UP | RIGHT, None)
        } else {
            pad(0, (frame == 12).then_some(HK))
        };
        let p2 = if frame == 0 {
            pad(UP, None)
        } else {
            pad(direction, None)
        };
        let before = machine.diagnostic_telemetry().p2.move_id;
        step(&mut machine, p1, p2);
        let after = machine.diagnostic_telemetry().p2.move_id;
        let damage = health - machine.era_fighters()[1].sub_hp;
        let blockstun = machine.toggle_state().combo.blockstun[1];
        if damage != 0 || blockstun != 0 {
            return Some((frame, damage, before, after, blockstun));
        }
    }
    None
}

#[test]
fn late_air_guard_rolls_back_a_projectile_hit_to_chip() {
    use sf_engine::netplay::{encode_inputs, RollbackEngine};

    let Some(select) = local_select(GaugeMode::EraDefault) else {
        return;
    };
    let session = local_fight(&select, Revision::Sfz3, Loadout::AIsm);
    let assets = load_assets(ROMS, MANIFEST).expect("local ROMs assemble");
    let mut reference = session.fight().unwrap().clone();
    let mut predicted = reference.clone();
    let initial_health = reference.machine().era_fighters()[1].sub_hp;
    let mut rollback = RollbackEngine::new(0, 0, 8);
    const DELAY: u32 = 4;
    const IMPACT: u32 = 31;
    const FRAMES: u32 = 70;
    let inputs = |frame: u32| {
        let away = if frame >= IMPACT { RIGHT } else { 0 };
        projectile_inputs(frame as usize, 20, away)
    };

    for tick in 0..FRAMES + DELAY {
        if let Some(frame) = tick.checked_sub(DELAY) {
            let (_, defender) = inputs(frame);
            rollback.apply_remote_inputs(
                &mut predicted,
                &assets,
                frame,
                &[encode_inputs(defender)],
            );
        }
        if tick < FRAMES {
            let (attacker, defender) = inputs(tick);
            reference.step(&assets, attacker, defender, false);
            rollback
                .step(&mut predicted, &assets, attacker)
                .expect("under the rollback limit");
            if tick == IMPACT {
                assert_eq!(
                    reference.machine().era_fighters()[1].sub_hp,
                    initial_health - 20
                );
                assert!(predicted.machine().era_fighters()[1].sub_hp < initial_health - 20);
            }
        }
    }
    assert_eq!(rollback.confirmed_inputs().len(), FRAMES as usize);
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
    assert_eq!(
        predicted.machine().era_fighters()[1].sub_hp,
        initial_health - 20
    );
}

#[test]
fn alpha_air_block_projectile() {
    let Some(select) = local_select(GaugeMode::EraDefault) else {
        return;
    };
    let no_gauges = local_select(GaugeMode::NoGauges).expect("same local ROM set");
    for (select, revision, loadout, direction, blocked) in [
        (&select, Revision::Sfz1, Loadout::Standard, RIGHT, true),
        (&select, Revision::Sfz2, Loadout::Standard, RIGHT, true),
        (&select, Revision::Sfz2A, Loadout::Standard, RIGHT, true),
        (&select, Revision::Sfz3, Loadout::AIsm, RIGHT, true),
        (&select, Revision::Sfz3, Loadout::VIsm, RIGHT, true),
        (&select, Revision::Sfz3, Loadout::XIsm, RIGHT, false),
        (&select, Revision::Sfz3, Loadout::AIsm, LEFT, false),
        (&no_gauges, Revision::Sfz3, Loadout::AIsm, RIGHT, false),
        (&select, Revision::Sf1Plus, Loadout::Standard, RIGHT, false),
    ] {
        let session = local_fight(select, revision, loadout);
        let machine = session.fight_machine().unwrap();
        let (frame, damage, before, after) = projectile_contact(machine, 20, direction)
            .expect("Hadoken contacts the defender's jump");
        assert_eq!((frame, before), (31, 9), "{revision:?} {loadout:?}");
        if blocked {
            assert_eq!((damage, after), (20, 9), "{revision:?} {loadout:?}");
        } else {
            let expected = if loadout == Loadout::XIsm { 207 } else { 180 };
            assert_eq!((damage, after), (expected, 0x39), "{revision:?} {loadout:?}");
        }
    }
}

#[test]
fn alpha_air_block_melee() {
    let Some(select) = local_select(GaugeMode::EraDefault) else {
        return;
    };
    let a_ism = local_fight(&select, Revision::Sfz3, Loadout::AIsm);
    let x_ism = local_fight(&select, Revision::Sfz3, Loadout::XIsm);
    for (session, direction, blocked) in [
        (&a_ism, RIGHT, true),
        (&a_ism, LEFT, false),
        (&x_ism, RIGHT, false),
    ] {
        let mut machine = session.fight_machine().unwrap().clone();
        approach(&mut machine, 65);
        let (frame, damage, before, after, stun) = melee_contact(&machine, direction)
            .expect("the jumping hard kick contacts the defender's jump");
        assert_eq!((frame, before), (18, 9));
        if blocked {
            assert_eq!((damage, after, stun), (0, 9, 20));
        } else {
            assert!(damage > 0 && after >= 0x30 && stun == 0);
        }
    }
}

#[test]
fn air_block_keeps_the_jump_height_and_landing() {
    let Some(select) = local_select(GaugeMode::EraDefault) else {
        return;
    };
    let session = local_fight(&select, Revision::Sfz3, Loadout::AIsm);
    let mut blocked = session.fight_machine().unwrap().clone();
    let mut uninterrupted = blocked.clone();
    let health = blocked.era_fighters()[1].sub_hp;
    for frame in 0..75 {
        let direction = if frame <= 31 { RIGHT } else { 0 };
        let (p1, p2) = projectile_inputs(frame, 20, direction);
        step(&mut blocked, p1, p2);
        step(&mut uninterrupted, PlayerInputs::default(), p2);
        let impact = blocked.diagnostic_telemetry().p2;
        let normal = uninterrupted.diagnostic_telemetry().p2;
        assert_eq!(
            (impact.center_y, impact.move_id),
            (normal.center_y, normal.move_id),
            "frame {frame}"
        );
    }
    assert_eq!(health - blocked.era_fighters()[1].sub_hp, 20);
    assert_eq!(
        blocked.diagnostic_telemetry().p2.move_id,
        1,
        "returned to standing"
    );
}
