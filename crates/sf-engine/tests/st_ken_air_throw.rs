//! New Ken Air Throw through Local Versus controller input.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::combo::throw::{THROWN_MOVE, THROW_MOVE};
use sf_engine::data_bank::GROUND;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::{fight::Fight, GameSession, PlayerInputs};
use sf_engine::overlay::FighterTelemetry;
use support::{local_fight, local_select_with_p1, pad, LEFT, RIGHT, UP};

const LP: usize = 0;
const MP: usize = 1;
const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;

fn axis(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let pose_x = fight.machine().state.objectram[usize::from(fighter.entity_offset) + 0x26] as i8;
    fighter.center_x - i32::from(pose_x)
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

fn place_axes(fight: &mut Fight, positions: [i32; 2]) {
    let read = fight.machine().diagnostic_telemetry();
    let mut snapshot = fight.save_netplay_snapshot(0);
    for (fighter, x) in [read.p1, read.p2].into_iter().zip(positions) {
        let ram = &mut snapshot.machine.state.objectram;
        let entity = usize::from(fighter.entity_offset);
        let dx = (x - axis(fight, fighter)) as u16;
        let cells = (usize::from(ram[entity + 0xc]) + 1) * (usize::from(ram[entity + 0xd]) + 1);
        for cell in (entity..).step_by(0x40).take(cells) {
            let field = &mut ram[cell + 6..cell + 8];
            let x = u16::from_be_bytes(field.try_into().unwrap()).wrapping_add(dx);
            field.copy_from_slice(&x.to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
}

fn ready(session: &GameSession) -> Fight {
    let mut fight = session.fight().unwrap().clone();
    for _ in 0..94 {
        fight.step(session.assets(), pad(0, None), pad(0, None), false);
    }
    fight
}

fn jump(session: &GameSession, base: &Fight, victim_air: bool, ken_right: bool) -> Fight {
    let mut fight = base.clone();
    // The native neutral jump rises later and lower than Ken's directional
    // jump. Give Ryu a head start so their heights overlap around 59 px.
    place_axes(&mut fight, if ken_right { [244, 196] } else { [196, 244] });
    fight.step(
        session.assets(),
        pad(if victim_air { UP } else { 0 }, None),
        pad(0, None),
        false,
    );
    for _ in 0..4 {
        fight.step(session.assets(), pad(0, None), pad(0, None), false);
    }
    fight.step(
        session.assets(),
        pad(0, None),
        pad(UP | if ken_right { RIGHT } else { LEFT }, None),
        false,
    );
    for _ in 0..40 {
        let read = fight.machine().diagnostic_telemetry();
        if height(&fight, read.p2) >= 59
            && (!victim_air || (height(&fight, read.p1) - height(&fight, read.p2)).abs() <= 16)
        {
            return fight;
        }
        fight.step(session.assets(), pad(0, None), pad(0, None), false);
    }
    panic!("Ken did not reach his rising Air Throw window");
}

fn captured(fight: &Fight, tick: u32) -> bool {
    fight
        .save_netplay_snapshot(tick)
        .machine
        .toggle_state
        .throw
        .st_ryu_capture_active(0)
}

fn jump_state(fight: &Fight) -> [(u8, i32, i32, i32, u8, u8); 2] {
    let read = fight.machine().diagnostic_telemetry();
    [read.p1, read.p2].map(|fighter| {
        let ram = &fight.machine().state.objectram[usize::from(fighter.entity_offset)..];
        let push_floor = GROUND - fighter.center_y - i32::from(ram[0x17]);
        (
            ram[0xa],
            axis(fight, fighter),
            height(fight, fighter),
            push_floor,
            ram[8],
            ram[0xf],
        )
    })
}

#[test]
fn new_ken_uses_air_throw_with_either_kick_direction_and_old_uses_a_normal() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    for loadout in [Loadout::Standard, Loadout::Old] {
        let session = local_fight(&select, Revision::St, loadout);
        let base = ready(&session);
        for ken_right in [false, true] {
            let jump = jump(&session, &base, true, ken_right);
            for (direction, button, expected) in [
                (LEFT, MK, true),
                (RIGHT, MK, true),
                (LEFT, HK, true),
                (RIGHT, HK, true),
                (0, HK, false),
                (LEFT, MP, false),
                (RIGHT, LK, false),
            ] {
                let mut fight = jump.clone();
                fight.step(
                    session.assets(),
                    pad(0, None),
                    pad(direction, Some(button)),
                    false,
                );
                let accepted = expected && loadout == Loadout::Standard;
                assert_eq!(
                    captured(&fight, 0),
                    accepted,
                    "{loadout:?}, facing right {ken_right}, direction {direction}, button {button}: {:?}",
                    jump_state(&jump)
                );
                if accepted {
                    let fighters = fight.fight_read().fighters;
                    assert_eq!(
                        (fighters[0].move_id, fighters[1].move_id),
                        (THROWN_MOVE, THROW_MOVE)
                    );
                }
            }
        }
    }
}

#[test]
fn air_throw_releases_with_arcade_damage_and_softening_boundary() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let base = ready(&session);
    let grounded = jump(&session, &base, false, false);
    let mut ground_control = grounded;
    ground_control.step(session.assets(), pad(0, None), pad(LEFT, Some(HK)), false);
    assert!(!captured(&ground_control, 0));
    let mut in_air = jump(&session, &base, true, false);
    let before = jump_state(&in_air);
    in_air.step(session.assets(), pad(0, None), pad(LEFT, Some(HK)), false);
    assert!(
        captured(&in_air, 0),
        "before {before:?}, after {:?}",
        jump_state(&in_air)
    );
    let initial_hp = in_air.machine().era_fighters()[0].sub_hp;
    for (response_tick, button, softened) in [(13, MP, true), (13, LP, false), (14, MP, false)] {
        let mut fight = in_air.clone();
        let (damage_tick, damage, victim_ready) = if softened {
            (46, 107, 101)
        } else {
            (79, 213, 153)
        };
        for tick in 1..=victim_ready {
            let response = if tick == response_tick {
                pad(RIGHT, Some(button))
            } else {
                PlayerInputs::default()
            };
            fight.step(session.assets(), response, pad(0, None), false);
            assert_eq!(captured(&fight, tick), tick < 46, "release at tick {tick}");
            let health = fight.machine().era_fighters()[0].sub_hp;
            assert_eq!(
                initial_hp - health,
                if tick >= damage_tick { damage } else { 0 },
                "response +{response_tick}, button {button}, tick {tick}"
            );
            let fighters = fight.fight_read().fighters;
            assert_eq!(
                fighters[1].move_id == THROW_MOVE,
                tick < 84,
                "Ken ready tick"
            );
            assert_eq!(
                fighters[0].state & 0x80 != 0,
                tick < victim_ready,
                "Ryu ready at tick {tick}, response +{response_tick}, button {button}"
            );
        }
    }
}
