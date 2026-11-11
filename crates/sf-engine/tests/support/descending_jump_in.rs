//! A measured native jump-in fixture, shared by the boss guard regressions.

use sf_engine::front::{FrameInputs, GameSession};

#[derive(Clone, Copy, Debug)]
pub enum Guard {
    None,
    Standing,
    Crouching,
}

/// Only the spacing and guard timing that differ between measured fixtures.
#[derive(Clone, Copy, Debug)]
pub struct Setup {
    pub approach_frames: u32,
    pub guard_from_takeoff: bool,
}

pub struct Observed {
    pub moves: Vec<u8>,
    pub health_before: u16,
    pub health_after: u16,
    pub airborne_attack_frames: u32,
}

fn pose(session: &GameSession) -> (u16, u8, u8) {
    let machine = session.fight_machine().expect("a ready Local fight");
    let entity = usize::from(machine.diagnostic_telemetry().p2.entity_offset);
    let objects = &machine.state.objectram;
    (
        u16::from_be_bytes([objects[entity + 2], objects[entity + 3]]),
        objects[entity + 0xc],
        objects[entity + 0xd],
    )
}

/// Walk to the measured spacing, then observe one descending Geki kick.
/// Every guard variant must actually contact in the air; a miss cannot make
/// the caller's health-preservation assertion pass vacuously.
pub fn observe(start: &GameSession, guard: Guard, setup: Setup) -> Observed {
    let mut session = start.clone();
    let mut retreat = FrameInputs::default();
    retreat.p1.directions[1] = true;
    for _ in 0..60 {
        session.step(retreat);
    }
    let mut approach = FrameInputs::default();
    approach.p1.directions[0] = true;
    for _ in 0..setup.approach_frames {
        session.step(approach);
    }

    let health_before = session.fight().unwrap().fight_read().fighters[0].health;
    let mut moves = Vec::new();
    let mut airborne_attack_frames = 0;
    let mut connected_in_air = false;
    let grounded_y = session
        .fight_machine()
        .unwrap()
        .diagnostic_telemetry()
        .p2
        .center_y;
    let mut previous_y = grounded_y;
    let mut previous_pose = pose(&session);
    for frame in 0..60 {
        let before = session.fight_machine().unwrap().diagnostic_telemetry().p2;
        let current_pose = pose(&session);
        if frame == 36 {
            assert!(
                (9..=0x0b).contains(&before.move_id),
                "Geki is still jumping"
            );
            assert_eq!(
                current_pose, previous_pose,
                "descent is not a pose-origin change"
            );
            assert!(
                before.center_y > previous_y && before.center_y < grounded_y,
                "Geki is descending above the floor before the kick"
            );
        }
        previous_y = before.center_y;
        previous_pose = current_pose;

        let mut input = FrameInputs::default();
        if setup.guard_from_takeoff || frame >= 36 {
            match guard {
                Guard::None => {}
                Guard::Standing => input.p1.directions[1] = true,
                Guard::Crouching => {
                    input.p1.directions[1] = true;
                    input.p1.directions[2] = true;
                }
            }
        }
        if frame < 2 {
            input.p2.directions[3] = true;
            input.p2.directions[1] = true;
        }
        input.p2.buttons[3] = (36..38).contains(&frame);
        session.step(input);

        let attacker = session.fight_machine().unwrap().diagnostic_telemetry().p2;
        if (0x20..=0x26).contains(&attacker.move_id) {
            airborne_attack_frames += 1;
            if attacker.attack_connected_this_frame {
                assert_eq!(attacker.state_byte, 0xc1, "the connecting kick is airborne");
                assert!(
                    attacker.center_y < grounded_y,
                    "the kick connects above the floor"
                );
                connected_in_air = true;
            }
        }
        let defender = session.fight().unwrap().fight_read().fighters[0];
        if !moves.contains(&defender.move_id) {
            moves.push(defender.move_id);
        }
    }
    assert!(
        connected_in_air,
        "the jump kick actually contacts {guard:?} with {setup:?}"
    );
    Observed {
        moves,
        health_before,
        health_after: session.fight().unwrap().fight_read().fighters[0].health,
        airborne_attack_frames,
    }
}
