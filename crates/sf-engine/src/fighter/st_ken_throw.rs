//! World 940223 Ken ground-throw trajectories on the advancing game clock.

use super::{ballistic_height, signed_q8_step, ThrowPosition, BOUNCE_HEIGHT};
use crate::combo::throw::{StRyuThrow, StRyuThrowFamily};

// The roll begins at +27. Ken's first step rounds towards negative infinity
// when left-facing. The final five samples include the pose offsets recorded
// as he flips and plants his feet.
const WHEEL_HEIGHTS: [u8; 32] = [
    2, 3, 5, 7, 8, 10, 11, 12, 13, 14, 15, 15, 16, 16, 16, 17, 17, 16, 16, 16, 15, 15, 14, 13, 12,
    11, 10, 8, 7, 5, 3, 2,
];
const WHEEL_PLANT_LEFT: [i16; 5] = [-136, -140, -135, -139, -144];
const WHEEL_PLANT_RIGHT: [i16; 5] = [136, 140, 135, 139, 144];
// Signed integer source axes above the floor, facing left and throwing
// forward. The low heel contacts briefly put the receiver below the floor.
// The engine places the art at floor level for these poses.
const WHEEL_CAPTURE: &[(i16, i8)] = &[
    (-3, 29),
    (-7, 30),
    (-12, 32),
    (-16, 34),
    (-21, 35),
    (-25, 37),
    (-26, -4),
    (-30, -3),
    (-35, -2),
    (-39, -1),
    (-44, 0),
    (-48, 0),
    (-53, 1),
    (-64, -6),
    (-69, -6),
    (-73, -5),
    (-78, -5),
    (-82, -6),
    (-87, -6),
    (-96, 11),
    (-101, 10),
    (-105, 10),
    (-110, 9),
    (-114, 8),
    (-149, 58),
    (-153, 57),
    (-158, 56),
    (-162, 54),
    (-207, 32),
    (-211, 30),
    (-216, 28),
    (-220, 27),
    (-250, 28),
    (-254, 28),
    (-262, 32),
];

pub(super) fn attacker_position(motion: StRyuThrow) -> ThrowPosition {
    if motion.family != StRyuThrowFamily::HellWheel || motion.elapsed < 27 {
        return ThrowPosition::new(0, 0);
    }
    if motion.elapsed < 59 {
        let step = (motion.elapsed - 26) as i16;
        let x = signed_q8_step(0, step, 1152, direction(motion.leftward));
        return ThrowPosition::new(x, WHEEL_HEIGHTS[usize::from(motion.elapsed - 27)]);
    }
    let steps = if motion.leftward {
        &WHEEL_PLANT_LEFT
    } else {
        &WHEEL_PLANT_RIGHT
    };
    ThrowPosition::new(steps[usize::from((motion.elapsed - 59).min(4))], 0)
}

pub(super) fn victim_position(motion: StRyuThrow) -> Option<ThrowPosition> {
    match motion.family {
        StRyuThrowFamily::HellWheel => Some(wheel_position(motion)),
        StRyuThrowFamily::KneeBash => Some(knee_position(motion)),
        _ => None,
    }
}

pub(super) const fn victim_travel_leftward(motion: StRyuThrow) -> bool {
    // Knee Bash is an escape away from the attacker's original facing.
    motion.leftward ^ (matches!(motion.family, StRyuThrowFamily::KneeBash) && motion.back)
}

const fn direction(leftward: bool) -> i16 {
    if leftward {
        -1
    } else {
        1
    }
}

fn wheel_position(motion: StRyuThrow) -> ThrowPosition {
    let tick = motion.elapsed;
    let sign = direction(motion.leftward);
    if tick < 27 {
        let (x, height) = if tick < 15 {
            (-25, 0)
        } else if tick < 25 {
            (-15, 13)
        } else {
            (-2, 27)
        };
        return ThrowPosition::new(sign * x, height);
    }
    if tick <= 63 {
        let (x, y) = WHEEL_CAPTURE[usize::from((tick - 27).min(34))];
        let height = if motion.softened && tick == 63 {
            48
        } else {
            y.max(0) as u8
        };
        return ThrowPosition::new(-sign * x, height);
    }
    let landing = motion.landing_tick();
    let first_flight = tick.min(landing) - 63;
    if motion.softened {
        let x = sign * (262 + first_flight as i16 * 2);
        let height = if tick >= landing {
            0
        } else {
            ballistic_height(48, first_flight as i16, 1400)
        };
        return ThrowPosition::new(x, height);
    }
    if tick <= landing {
        let x = signed_q8_step(262, first_flight as i16, 896, sign);
        return ThrowPosition::new(
            x,
            if tick == landing {
                0
            } else {
                ballistic_height(32, first_flight as i16, 824)
            },
        );
    }
    let bounce = tick.saturating_sub(landing + 4);
    let height = bounce
        .checked_sub(1)
        .and_then(|step| BOUNCE_HEIGHT.get(usize::from(step)))
        .copied()
        .unwrap_or(0);
    let landing_x = 262 + (landing - 63) as i16 * 7 / 2;
    let x = signed_q8_step(landing_x, bounce.min(17) as i16, 640, sign);
    let slide = [0, 1, 2, 3, 3, 4, 4, 5, 5, 5, 6];
    let slide_index = usize::from(bounce.saturating_sub(17)).min(slide.len() - 1);
    ThrowPosition::new(x + sign * slide[slide_index], height)
}

fn knee_position(motion: StRyuThrow) -> ThrowPosition {
    let tick = motion.elapsed;
    let sign = direction(victim_travel_leftward(motion));
    let [x, height] = knee_pose(motion.knee.phase);
    let Some(release) = motion.knee.released_at else {
        return ThrowPosition::new(sign * i16::from(x), height);
    };
    let [x, height] = knee_pose(motion.knee.release_pose);
    if tick == release {
        return ThrowPosition::new(sign * i16::from(x), height);
    }
    let landing = motion.landing_tick();
    let step = (tick.min(landing) - release - 1) as i16;
    let x = sign * (i16::from(x) + step * 2);
    let height = if tick >= landing {
        0
    } else {
        ballistic_height(height + 16, step, 1312)
    };
    ThrowPosition::new(x, height)
}

fn knee_pose(phase: u8) -> [u8; 2] {
    match phase {
        1 => [33, 16],
        2 => [36, 2],
        _ => [39, 0],
    }
}

pub(super) fn advance_knee_bash(motion: &mut StRyuThrow, inputs: [(u8, u8); 2]) {
    let knee = &mut motion.knee;
    knee.damage_hp = 0;
    let [defender, attacker] = std::array::from_fn(|index| {
        let (direction, buttons) = inputs[index];
        let edge = u8::from(knee.directions[index] == 0 && direction != 0);
        knee.directions[index] = direction;
        3 * edge as i8 + i8::from(buttons != 0)
    });
    if knee.released_at.is_some() {
        return;
    }
    if knee.freeze > 0 {
        knee.freeze -= 1;
        return;
    }
    knee.hold_ticks = knee.hold_ticks.saturating_sub(1);
    knee.grip -= defender;
    if knee.hold_ticks == 0 || knee.grip < 0 {
        knee.released_at = Some(motion.elapsed);
        knee.release_pose = knee.phase;
        return;
    }
    knee.attack_budget -= attacker;
    if knee.attack_budget < 0 {
        knee.attack_budget = 12;
        knee.pose_tick = knee.pose_tick.min(1);
    }
    if knee.knee_pending {
        knee.knee_pending = false;
        knee.damage_hp = if knee.hits == 0 { 26 } else { 4 };
        knee.hits += 1;
        knee.freeze = 12;
        return;
    }
    knee.pose_tick = knee.pose_tick.saturating_sub(1);
    if knee.pose_tick == 0 {
        (knee.phase, knee.pose_tick) = match knee.phase {
            1 => (2, 2),
            2 => (0, 4),
            _ => (1, 15),
        };
        knee.knee_pending = knee.phase == 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knee_bash(input: impl Fn(u16) -> [(u8, u8); 2]) -> (u16, Vec<(u16, u16)>) {
        let mut motion = StRyuThrow::new(StRyuThrowFamily::KneeBash, 0, false);
        let mut hits = Vec::new();
        for tick in 1..=300 {
            motion.elapsed = tick;
            advance_knee_bash(&mut motion, input(tick));
            if let Some(hp) = motion.damage_hp() {
                hits.push((tick, hp));
            }
            if !motion.capture_active() {
                return (motion.release_tick(), hits);
            }
        }
        panic!("Knee Bash must end when its hold budget runs out");
    }

    #[test]
    fn tap_and_mash_use_different_arcade_hold_budgets() {
        let neutral = (0, 0);
        let baseline = knee_bash(|_| [neutral; 2]);
        assert_eq!(
            baseline,
            (179, vec![(14, 26), (48, 4), (82, 4), (116, 4), (150, 4)])
        );

        let direction_pulses = |tick| if tick % 4 == 0 { (1, 0) } else { neutral };
        let escaped = knee_bash(|tick| [direction_pulses(tick), neutral]);
        assert!(escaped.0 < baseline.0 && escaped.1.len() < baseline.1.len());

        let mashed = knee_bash(|tick| [neutral, direction_pulses(tick)]);
        assert!(mashed.0 > baseline.0 && mashed.1.len() > baseline.1.len());
        assert!(mashed.1[0].0 < baseline.1[0].0);
    }
}
