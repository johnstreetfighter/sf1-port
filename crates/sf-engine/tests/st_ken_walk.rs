//! World Super Turbo Ken walk timing from normal Local Versus inputs.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::{fight::Fight, GameSession, PlayerInputs};
use sf_engine::roster::TournamentFighter::Ken;
use support::{local_fight, local_select_with_fighter, local_select_with_p1, pad, LEFT, LP, RIGHT};

fn axes(fight: &Fight) -> [(i32, usize); 2] {
    let machine = fight.machine();
    let telemetry = machine.diagnostic_telemetry();
    [telemetry.p1, telemetry.p2].map(|fighter| {
        let entity = usize::from(fighter.entity_offset);
        let offset = machine.state.objectram[entity + 0x26] as i8;
        (fighter.center_x - i32::from(offset), entity)
    })
}

fn world_x(fight: &Fight, side: usize) -> i32 {
    let camera = fight.machine().work_ram()[0x400c..0x400e]
        .try_into()
        .unwrap();
    i32::from(u16::from_be_bytes(camera)) + axes(fight)[side].0
}

fn set_axes(fight: &mut Fight, positions: [i32; 2]) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    for ((axis, entity), target) in axes(fight).into_iter().zip(positions) {
        let objects = &mut snapshot.machine.state.objectram;
        let rows = usize::from(objects[entity + 0xc]) + 1;
        let cols = usize::from(objects[entity + 0xd]) + 1;
        for cell in (entity..).step_by(0x40).take(rows * cols) {
            let x = u16::from_be_bytes(objects[cell + 6..cell + 8].try_into().unwrap());
            objects[cell + 6..cell + 8]
                .copy_from_slice(&x.wrapping_add((target - axis) as u16).to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
}

fn step(fight: &mut Fight, session: &GameSession, side: usize, input: PlayerInputs) {
    let mut pads = [PlayerInputs::default(); 2];
    pads[side] = input;
    fight.step(session.assets(), pads[0], pads[1], false);
}

#[test]
fn world_ken_walk_uses_both_directional_cadences_and_stops_on_release() {
    let Some(ryu) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    for loadout in [Loadout::Standard, Loadout::Old] {
        let ken =
            local_select_with_fighter(GaugeMode::EraDefault, Ken, Revision::St, loadout).unwrap();
        for (side, session) in [
            (0, local_fight(&ken, Revision::St, Loadout::Standard)),
            (1, local_fight(&ryu, Revision::St, loadout)),
        ] {
            let mut base = session.fight().unwrap().clone();
            for _ in 0..94 {
                step(&mut base, &session, side, pad(0, None));
            }
            let forward = if side == 0 { RIGHT } else { LEFT };
            let back = if side == 0 { LEFT } else { RIGHT };
            let facing = if side == 0 { 1 } else { -1 };
            let retreat = if loadout == Loadout::Old { -89 } else { -87 };
            for (direction, move_id, positions, first, total) in [
                (forward, 2, [145, 365], 3, 137),
                (back, 3, [225, 270], -2, retreat),
            ] {
                let mut fight = base.clone();
                set_axes(&mut fight, positions);
                let origin = world_x(&fight, side);
                let input = pad(direction, None);
                let case = format!("{loadout:?}, side {side}, move {move_id}");
                let mut previous = 0;
                for tick in 1..=32 {
                    let rollback = (tick == 13).then(|| fight.save_netplay_snapshot(tick));
                    step(&mut fight, &session, side, input);
                    if let Some(snapshot) = rollback {
                        let checksum = fight.state_checksum();
                        fight.restore_netplay_snapshot(&snapshot);
                        step(&mut fight, &session, side, input);
                        assert_eq!(fight.state_checksum(), checksum, "{case}, rollback");
                    }
                    let distance = (world_x(&fight, side) - origin) * facing;
                    let delta = distance - previous;
                    assert!(
                        if move_id == 2 {
                            matches!(delta, 3 | 4 | 6 | 7)
                        } else {
                            matches!(delta, -2 | -3 | -4 | -5)
                        },
                        "{case}, tick {tick}, delta {delta}"
                    );
                    if tick == 1 {
                        assert_eq!(fight.fight_read().fighters[side].move_id, move_id, "{case}");
                        assert_eq!(distance, first, "{case}, first input");
                    }
                    previous = distance;
                }
                assert_eq!(previous, total, "{case}, 32 ticks");
                step(&mut fight, &session, side, pad(0, None));
                assert_eq!(
                    (world_x(&fight, side) - origin) * facing,
                    total,
                    "{case}, release"
                );
                step(&mut fight, &session, side, input);
                assert_eq!(
                    (world_x(&fight, side) - origin) * facing,
                    total + first,
                    "{case}, fresh start"
                );
                step(&mut fight, &session, side, pad(0, Some(LP)));
                assert!(
                    !matches!(fight.fight_read().fighters[side].move_id, 2 | 3),
                    "{case}, interrupt"
                );
            }
        }
    }
}
