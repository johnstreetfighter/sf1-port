//! World Super Turbo 940223, Turbo 1, Ryu pacing witnesses for spec #138.

#[path = "support/alpha.rs"]
mod support;

use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::fighter::P1_STRUCT;
use sf_engine::front::fight::Fight;
use sf_engine::overlay::FighterTelemetry;
use sf_video::callouts::Side;
use sf_video::romset::Assets;
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, LP, RIGHT, UP};

const FAR_STANDING_LP: u8 = 0x0d;
const HADOKEN: u8 = 0x1a;
const WALK_BACK: u8 = 3;

fn world_x(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let machine = fight.machine();
    let work = machine.work_ram();
    let camera = u16::from_be_bytes([work[0x400c], work[0x400d]]);
    let pose_offset = machine.state.objectram[usize::from(fighter.entity_offset) + 0x26] as i8;
    i32::from(camera) + fighter.center_x - i32::from(pose_offset)
}

fn back_up(fight: &mut Fight, assets: &Assets) {
    for _ in 0..42 {
        fight.step(assets, pad(LEFT, None), pad(RIGHT, None), false);
    }
    for _ in 0..18 {
        fight.step(assets, pad(0, None), pad(0, None), false);
    }
}

fn walk_distance(fight: &mut Fight, assets: &Assets) -> i32 {
    let positions: Vec<_> = (0..=20)
        .map(|_| {
            fight.step(assets, pad(RIGHT, None), pad(0, None), false);
            let read = fight.machine().diagnostic_telemetry();
            assert!(!read.p1.facing_left);
            assert!(read.p1.pushbox.right < read.p2.pushbox.left);
            world_x(fight, read.p1)
        })
        .collect();
    for _ in 0..4 {
        fight.step(assets, pad(0, None), pad(0, None), false);
        let ryu = fight.machine().diagnostic_telemetry().p1;
        assert_eq!(world_x(fight, ryu), positions[20], "walk stops on release");
    }
    positions[20] - positions[4]
}

fn jab(fight: &mut Fight, assets: &Assets) -> (usize, usize) {
    let mut active = None;
    let mut active_ticks = 0;
    for tick in 0..24 {
        let input = if tick == 0 {
            pad(0, Some(LP))
        } else {
            pad(LEFT, None)
        };
        fight.step(assets, input, pad(0, None), false);
        let read = fight.fight_read();
        assert!(!read.connected(Side::P1));
        let ryu = read.fighters[0];
        if ryu.move_id == FAR_STANDING_LP && ryu.attack_active {
            active.get_or_insert(tick);
            active_ticks += 1;
        }
        if ryu.move_id == WALK_BACK {
            assert_eq!(active_ticks, 4);
            return (active.expect("whiffed standing LP became active"), tick);
        }
    }
    panic!("held retreat never took over from standing LP");
}

fn fireball(fight: &mut Fight, assets: &Assets) -> usize {
    let mut spawned = None;
    let mut recovered = None;
    let health = fight.fight_read().fighters[1].health;
    for tick in 0..160 {
        let ryu = match tick {
            0 => pad(DOWN, None),
            1 => pad(DOWN | RIGHT, None),
            2 => pad(RIGHT, Some(LP)),
            _ => pad(LEFT, None),
        };
        let ken = pad(
            if (15..=26).contains(&tick) {
                UP | LEFT
            } else {
                0
            },
            None,
        );
        fight.step(assets, ryu, ken, false);
        let read = fight.fight_read();
        assert_eq!(read.fighters[1].health, health, "Hadoken must whiff");
        assert!(!read.connected(Side::P1));
        if read.fighters[0].move_id == HADOKEN && spawned.is_none() {
            let telemetry = fight.machine().diagnostic_telemetry();
            if telemetry
                .projectiles
                .iter()
                .any(|p| p.owner_struct == P1_STRUCT)
            {
                spawned = Some(tick);
            }
        }
        if let Some(spawn) = spawned {
            if read.fighters[0].move_id == WALK_BACK {
                recovered.get_or_insert(tick - spawn);
            }
        }
    }
    recovered.expect("held retreat took over after a live light Hadoken")
}

#[test]
fn super_turbo_ryu_matches_world_turbo_one_pacing() {
    for (loadout, walk, recovery) in [
        (Loadout::Standard, 60..=60, 33),
        (Loadout::Old, 60..=63, 32),
    ] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        for opponent in [Revision::St, Revision::Sf1Plus] {
            let session = local_fight(&select, opponent, Loadout::Standard);
            let assets = session.assets();
            let mut fight = session.fight().expect("local fight").clone();
            assert_eq!(fight.fighter_style(0).unwrap().loadout, loadout);
            back_up(&mut fight, assets);

            let distance = walk_distance(&mut fight.clone(), assets);
            let (active, neutral) = jab(&mut fight.clone(), assets);
            let hadoken = fireball(&mut fight, assets);
            let context = format!("{loadout:?} Ryu vs {opponent:?} Ken");
            eprintln!(
                "{context}: walk {distance} px/16 ticks, jab {active}/{neutral}, Hadoken {hadoken}"
            );
            assert!(
                walk.contains(&distance),
                "{context}: {distance} px/16 ticks"
            );
            assert_eq!((active, neutral), (4, 11), "{context}: jab");
            assert_eq!(hadoken, recovery, "{context}: Hadoken recovery");
            assert!(fight.machine().unrecovered().is_empty());
        }
    }
}
