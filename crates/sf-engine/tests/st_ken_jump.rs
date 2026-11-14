//! World Super Turbo Ken free jumps and native aerial entry points.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::data_bank::GROUND;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::overlay::FighterTelemetry;
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, RIGHT, UP};

const LP: usize = 0;
const HK: usize = 5;

fn axis(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let offset = fight.machine().state.objectram[usize::from(fighter.entity_offset) + 0x26] as i8;
    fighter.center_x - i32::from(offset)
}

fn world_x(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let camera = fight.machine().work_ram()[0x400c..0x400e]
        .try_into()
        .unwrap();
    axis(fight, fighter) + i32::from(u16::from_be_bytes(camera))
}

fn height(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let objects = &fight.machine().state.objectram[usize::from(fighter.entity_offset)..];
    let tile = if u16::from_be_bytes([objects[2], objects[3]]) & 0x400 == 0 {
        16
    } else {
        32
    };
    GROUND
        - i32::from(u16::from_be_bytes([objects[4], objects[5]]))
        - tile * (i32::from(objects[0xc]) + 1)
}

fn place_axes(fight: &mut Fight, x: [i32; 2]) {
    let read = fight.machine().diagnostic_telemetry();
    let mut snapshot = fight.save_netplay_snapshot(0);
    for (fighter, x) in [read.p1, read.p2].into_iter().zip(x) {
        let e = usize::from(fighter.entity_offset);
        let dt = (x - axis(fight, fighter)) as u16;
        let ram = &mut snapshot.machine.state.objectram;
        let cells = (usize::from(ram[e + 0xc]) + 1) * (usize::from(ram[e + 0xd]) + 1);
        for cell in (e..).step_by(0x40).take(cells) {
            let bytes = &mut ram[cell + 6..cell + 8];
            let sx = u16::from_be_bytes(bytes.try_into().unwrap()).wrapping_add(dt);
            bytes.copy_from_slice(&sx.to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
}

#[test]
fn ken_free_jumps_replay_world_arcade_trajectories_and_rollback() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let source = include_str!("../../../docs/research/ken-super-turbo-jump-trajectory.tsv");
    for (loadout, name) in [(Loadout::Standard, "new"), (Loadout::Old, "old")] {
        let session = local_fight(&select, Revision::St, loadout);
        let mut base = session.fight().unwrap().clone();
        let idle = pad(0, None);
        for _ in 0..94 {
            base.step(session.assets(), idle, idle, false);
        }
        for facing_right in [false, true] {
            for (direction, move_id, x, input) in [
                ("up", 9, [157, 285], UP),
                (
                    "forward",
                    10,
                    [132, 335],
                    UP | if facing_right { RIGHT } else { LEFT },
                ),
                (
                    "back",
                    11,
                    [135, 200],
                    UP | if facing_right { LEFT } else { RIGHT },
                ),
            ] {
                let mut fight = base.clone();
                let positions = if facing_right {
                    [490 - x[0], 490 - x[1]]
                } else {
                    x
                };
                place_axes(&mut fight, positions);
                fight.step(session.assets(), idle, idle, false);
                let origin = world_x(&fight, fight.machine().diagnostic_telemetry().p2);
                for line in source.lines().skip(1) {
                    let row: Vec<_> = line.split('\t').collect();
                    if (row[0], row[1], row[2]) != (name, direction, "0") {
                        continue;
                    }
                    let tick: u32 = row[3].parse().unwrap();
                    let raw: i32 = row[4].parse().unwrap();
                    let dx = raw.signum() * ((raw.abs() + 0x8000) >> 16);
                    let target_height: i32 = row[5].parse().unwrap();
                    let next = if tick == 0 { pad(input, None) } else { idle };
                    let rollback = (tick == 22).then(|| fight.save_netplay_snapshot(tick));
                    fight.step(session.assets(), idle, next, false);
                    if let Some(rollback) = rollback {
                        let digest = fight.state_checksum();
                        fight.restore_netplay_snapshot(&rollback);
                        fight.step(session.assets(), idle, next, false);
                        assert_eq!(fight.state_checksum(), digest);
                    }
                    let ken = fight.machine().diagnostic_telemetry().p2;
                    let case = format!("{name} {direction} facing_right={facing_right} +{tick}");
                    assert_eq!(height(&fight, ken), target_height, "{case}");
                    assert_eq!(
                        world_x(&fight, ken) - origin,
                        if facing_right { -dx } else { dx },
                        "{case}"
                    );
                    assert_eq!(ken.move_id == move_id, row[8] == "0", "{case}");
                }
            }
        }
    }
}

#[test]
fn ken_free_jumps_can_still_branch_to_jump_normals_and_air_tatsumaki() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    for loadout in [Loadout::Standard, Loadout::Old] {
        let session = local_fight(&select, Revision::St, loadout);
        let mut base = session.fight().unwrap().clone();
        let idle = pad(0, None);
        for _ in 0..94 {
            base.step(session.assets(), idle, idle, false);
        }
        place_axes(&mut base, [132, 335]);
        for direction in [UP, UP | LEFT, UP | RIGHT] {
            let mut jump = base.clone();
            jump.step(session.assets(), idle, pad(direction, None), false);
            for _ in 0..10 {
                jump.step(session.assets(), idle, idle, false);
            }
            let mut normal = jump.clone();
            normal.step(session.assets(), idle, pad(0, Some(LP)), false);
            assert!(
                (0x20..=0x22).contains(&normal.fight_read().fighters[1].move_id),
                "{loadout:?}, {direction}"
            );

            let mut tatsu = jump;
            for dir in [DOWN, DOWN | RIGHT] {
                tatsu.step(session.assets(), idle, pad(dir, None), false);
            }
            tatsu.step(session.assets(), idle, pad(RIGHT, Some(HK)), false);
            let ken = tatsu.machine().diagnostic_telemetry().p2;
            let objects = &tatsu.machine().state.objectram[usize::from(ken.entity_offset)..];
            let stream = u32::from_be_bytes(objects[0x2c..0x30].try_into().unwrap());
            assert_eq!(ken.move_id, 0x20, "{loadout:?}, {direction}");
            assert!(
                sf_engine::data_bank::in_bank(stream),
                "banked Air Tatsumaki"
            );
        }
    }
}

#[test]
fn ken_air_normals_replay_world_turbo_two_boxes_and_keep_the_jump_arc() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let streams = include_str!("../../../docs/research/ken-super-turbo-jump-normal-streams.tsv");
    let trajectories = include_str!("../../../docs/research/ken-super-turbo-jump-trajectory.tsv");
    let idle = pad(0, None);
    for (loadout, name) in [(Loadout::Standard, "new"), (Loadout::Old, "old")] {
        let session = local_fight(&select, Revision::St, loadout);
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            base.step(session.assets(), idle, idle, false);
        }
        for facing_right in [false, true] {
            for (direction, trajectory, coordinates, jump) in [
                ("neutral", "up", [157, 285], UP),
                ("forward", "forward", [132, 347], UP | LEFT),
                ("back", "back", [135, 200], UP | RIGHT),
            ] {
                let arc: Vec<_> = trajectories
                    .lines()
                    .skip(1)
                    .filter_map(|line| {
                        let row: Vec<_> = line.split('\t').collect();
                        if (row[0], row[1], row[2]) != (name, trajectory, "0") {
                            return None;
                        }
                        let raw: i32 = row[4].parse().unwrap();
                        let dx = raw.signum() * ((raw.abs() + 0x8000) >> 16);
                        Some((
                            dx,
                            row[5].parse::<i32>().unwrap(),
                            row[7] == "1",
                            row[8] == "1",
                        ))
                    })
                    .collect();
                let floor = arc.iter().position(|pose| pose.2).unwrap();
                let mut settled = base.clone();
                let (coordinates, jump) = if facing_right {
                    (
                        coordinates.map(|x| 490 - x),
                        match jump {
                            d if d == UP | LEFT => UP | RIGHT,
                            d if d == UP | RIGHT => UP | LEFT,
                            _ => UP,
                        },
                    )
                } else {
                    (coordinates, jump)
                };
                place_axes(&mut settled, coordinates);
                settled.step(session.assets(), idle, idle, false);
                let origin = world_x(&settled, settled.machine().diagnostic_telemetry().p2);
                for (button, command) in
                    ["lp", "mp", "hp", "lk", "mk", "hk"].into_iter().enumerate()
                {
                    let mut poses = Vec::new();
                    let mut start = None;
                    for line in streams.lines().skip(1) {
                        let row: Vec<_> = line.split('\t').collect();
                        if (row[0], row[1], row[2], row[3]) != (name, direction, command, "0") {
                            continue;
                        }
                        start = Some(row[4].parse::<usize>().unwrap());
                        let active = (row[9] != "-").then(|| {
                            let fields: Vec<_> = row[9].split(',').collect();
                            let field = |index: usize| fields[index].parse::<i32>().unwrap();
                            (-field(1), field(2), field(3), field(4))
                        });
                        poses.push((row[7].parse::<usize>().unwrap(), active));
                    }
                    let start = start.unwrap();
                    assert!(!poses.is_empty());
                    let mut fight = settled.clone();
                    for (tick, &(dx, h, _, ready)) in arc.iter().enumerate() {
                        let next = if tick == 0 {
                            pad(jump, None)
                        } else if tick == start {
                            pad(0, Some(button))
                        } else {
                            idle
                        };
                        let checkpoint = (tick == start + 5 && command == "mk")
                            .then(|| fight.save_netplay_snapshot(tick as u32));
                        fight.step(session.assets(), idle, next, false);
                        if let Some(checkpoint) = checkpoint {
                            let digest = fight.state_checksum();
                            fight.restore_netplay_snapshot(&checkpoint);
                            fight.step(session.assets(), idle, next, false);
                            assert_eq!(fight.state_checksum(), digest);
                        }
                        let ken = fight.machine().diagnostic_telemetry().p2;
                        let tag = format!("{name} {direction} {command}, facing_right={facing_right}, jump +{tick}");
                        assert_eq!(
                            world_x(&fight, ken) - origin,
                            if facing_right { -dx } else { dx },
                            "{tag}"
                        );
                        assert_eq!(height(&fight, ken), h, "{tag}");
                        assert_eq!(ken.move_id == 1, ready, "{tag}");
                        let expected = if (start..floor).contains(&tick) {
                            poses
                                .iter()
                                .rev()
                                .find(|(age, _)| *age <= tick - start)
                                .and_then(|p| p.1)
                        } else {
                            None
                        };
                        let hitbox = ken.attack_hitbox.map(|hit| {
                            (
                                (hit.center_x - axis(&fight, ken))
                                    * if facing_right { 1 } else { -1 },
                                GROUND - h - hit.center_y,
                                hit.half_w,
                                hit.half_h,
                            )
                        });
                        assert_eq!(hitbox, expected, "{tag}");
                    }
                }
                if direction == "forward" {
                    let mut late = settled;
                    for tick in 0..arc.len() {
                        let next = if tick == 0 {
                            pad(jump, None)
                        } else if tick == floor - 3 {
                            pad(0, Some(LP))
                        } else {
                            idle
                        };
                        late.step(session.assets(), idle, next, false);
                        let ken = late.machine().diagnostic_telemetry().p2;
                        assert_eq!(
                            ken.attack_active,
                            tick == floor - 1,
                            "{name} late LP +{tick}"
                        );
                        assert_eq!(height(&late, ken), arc[tick].1);
                    }
                }
            }
        }
    }
}

#[test]
fn ken_late_neutral_medium_kick_uses_world_standing_and_crouching_reactions() {
    use sf_engine::era_fight::FULL_SUB_HP;
    use sf_video::callouts::Side;

    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let idle = pad(0, None);
    for loadout in [Loadout::Standard, Loadout::Old] {
        let session = local_fight(&select, Revision::St, loadout);
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            base.step(session.assets(), idle, idle, false);
        }
        let old = loadout == Loadout::Old;
        for (directions, blocked, damage, contact, recoil, ready) in [
            (0, false, 19, if old { 45 } else { 46 }, 30, 27),
            (LEFT, true, 0, if old { 45 } else { 46 }, 54, 32),
            (
                DOWN | LEFT,
                false,
                if old { 18 } else { 19 },
                if old { 47 } else { 48 },
                54,
                32,
            ),
        ] {
            let mut fight = base.clone();
            place_axes(&mut fight, [270, 308]);
            fight.step(session.assets(), idle, idle, false);
            let mut at_contact = None;
            let mut hit_ticks = Vec::new();
            let case = format!("{loadout:?} neutral MK, defender directions {directions}");
            for tick in 0..=contact + ready {
                let ken = if tick == 0 {
                    pad(UP, None)
                } else if tick == if old { 40 } else { 41 } {
                    pad(0, Some(4))
                } else {
                    idle
                };
                let guard_start = if old { 41 } else { 42 };
                let ryu = if directions != 0 && tick >= guard_start {
                    pad(directions, None)
                } else {
                    idle
                };
                fight.step(session.assets(), ryu, ken, false);
                let read = fight.fight_read();
                let telemetry = fight.machine().diagnostic_telemetry();
                if read.connected(Side::P2) {
                    hit_ticks.push(tick);
                    assert_eq!(read.blocked[1], blocked, "{case}");
                    assert_eq!(
                        FULL_SUB_HP - fight.machine().era_fighters()[0].sub_hp,
                        (damage * FULL_SUB_HP + 72) / 144,
                        "{case}"
                    );
                    assert_eq!(
                        fight.machine().toggle_state().combo.contact_freeze[..2],
                        [14, 14],
                        "{case}"
                    );
                    at_contact.get_or_insert_with(|| world_x(&fight, telemetry.p1));
                }
                if (contact + 1..contact + ready).contains(&tick) {
                    let fighter = read.fighters[0];
                    assert!(
                        if blocked {
                            fighter.blockstun
                        } else {
                            fighter.hitstun()
                        },
                        "{case}, tick {tick}"
                    );
                }
                if tick == contact + ready - 1 {
                    assert_eq!(
                        at_contact.unwrap() - world_x(&fight, telemetry.p1),
                        recoil,
                        "{case}"
                    );
                }
                if tick == contact + ready {
                    assert!(
                        !read.fighters[0].hitstun() && !read.fighters[0].blockstun,
                        "{case}"
                    );
                }
            }
            assert_eq!(hit_ticks, [contact], "{case}");
        }
    }
}
