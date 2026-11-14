//! World 940223 New Ken Air Throw against an airborne New Ryu.

pub(super) const RESPONSE_LAST: u8 = 13;
pub(super) const RELEASE: u8 = 46;
pub(super) const FULL_LAND: u8 = 79;
pub(super) const SOFT_LAND: u8 = 95;
pub(super) const KEN_READY: u8 = 84;
pub(super) const FULL_RYU_READY: u8 = 153;
pub(super) const SOFT_RYU_READY: u8 = 101;

/// Gap is positive toward the direction Ken faces. Heights are above the
/// floor. The remaining target parameters are the arcade pushbox operands.
pub(super) fn eligible(
    falling: bool,
    ken_height: i16,
    facing_gap: i16,
    victim_height: i16,
    victim_push_radius: i16,
    victim_vertical_offset: i16,
) -> bool {
    let minimum = if falling { 48 } else { 56 };
    if ken_height < minimum {
        return false;
    }
    let horizontal_gap = (i32::from(facing_gap) - 24).abs();
    let vertical_gap =
        (i32::from(victim_height) + i32::from(victim_vertical_offset) - i32::from(ken_height) - 53)
            .abs();
    horizontal_gap <= 24 + i32::from(victim_push_radius) && vertical_gap <= 16
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Frame {
    /// Signed position offsets from Ken's pre-capture world X, in pixels.
    pub(super) ken_x: i16,
    pub(super) victim_x: i16,
    pub(super) ken_height: i16,
    pub(super) victim_height: i16,
    /// Zero-based index in the respective arcade pose sequence.
    pub(super) ken_pose: u8,
    pub(super) victim_pose: u8,
    /// Whether the victim is oriented opposite to his initial facing.
    pub(super) victim_turn: bool,
}

pub(super) fn frame(tick: u8, start_height: i16, softened: bool, facing_left: bool) -> Frame {
    let weight = i32::from(start_height.clamp(59, 69) - 59);
    let held_tick = usize::from(tick.min(RELEASE));
    let [ken_x, ken_height, victim_x, held_victim_height] = std::array::from_fn(|i| {
        interpolate(
            LOW_CAPTURE[held_tick][i],
            HIGH_CAPTURE[held_tick][i],
            weight,
        )
    });
    let victim_height = if tick <= RELEASE {
        held_victim_height
    } else {
        let age = usize::from(tick - RELEASE);
        let (low, high) = if softened {
            (&SOFT_LOW_HEIGHTS[..], &SOFT_HIGH_HEIGHTS[..])
        } else {
            (&FULL_LOW_HEIGHTS[..], &FULL_HIGH_HEIGHTS[..])
        };
        interpolate(
            i32::from(low.get(age - 1).copied().unwrap_or(0)),
            i32::from(high.get(age - 1).copied().unwrap_or(0)),
            weight,
        )
    };
    let dx = release_travel(tick.saturating_sub(RELEASE), softened);
    let sign = if facing_left { -1 } else { 1 };
    let ken_pose = pose_index(
        tick,
        &[14, 20, 26, 31, 36, 41, 43, 45, 66, 67, 75, 83, KEN_READY],
    );
    let victim_pose = if tick < RELEASE {
        pose_index(tick, &[14, 20, 26, 31, 36, 41, 43, 45])
    } else if softened {
        // Air recover, tumble, landing, and stance.
        9 + pose_index(tick, &[65, 67, 69, 71, 73, SOFT_LAND, SOFT_RYU_READY])
    } else {
        // Released, landing, bounce, lying, and get-up.
        9 + pose_index(
            tick,
            &[FULL_LAND, 83, 101, 124, 130, 135, 141, 148, FULL_RYU_READY],
        )
    };
    Frame {
        ken_x: sign * pixels(ken_x),
        victim_x: sign * pixels(victim_x + dx),
        ken_height: ken_height as i16,
        victim_height: victim_height as i16,
        ken_pose,
        victim_pose,
        victim_turn: matches!(tick, 0..=13 | 26..=35 | 45)
            || (!softened && (RELEASE..FULL_RYU_READY).contains(&tick)),
    }
}

fn pixels(position: i32) -> i16 {
    (position.signum() * ((position.abs() + 0x8000) >> 16)) as i16
}

fn interpolate(low: i32, high: i32, weight: i32) -> i32 {
    (low * (10 - weight) + high * weight + 5).div_euclid(10)
}

fn pose_index(tick: u8, transitions: &[u8]) -> u8 {
    transitions.partition_point(|&start| start <= tick) as u8
}

fn release_travel(age: u8, softened: bool) -> i32 {
    let steps = i32::from(age.saturating_sub(1));
    if softened {
        return steps.min(48) * 0x20000;
    }
    // 32 advancing ticks at 3½ px. After the ground pause the bounce
    // moves 18 ticks at 2½ px, then a friction slide decays over 12 ticks.
    let fall = steps.min(32) * 0x38000;
    let bounce = i32::from(age.saturating_sub(37).min(18)) * 0x28000;
    let slide_steps = i32::from(age.saturating_sub(55).min(12));
    let slide = slide_steps * 0xec00 - slide_steps * (slide_steps - 1) * 0x0a00;
    fall + bounce + slide
}

// Attached/capture positions, in signed forward 16.16 from Ken's origin,
// with absolute height in pixels. The final row is the common release
// origin, after which the defender follows the selected fall.
const LOW_CAPTURE: [[i32; 4]; 47] = [
    [212736, 62, -752128, 75],
    [212736, 62, -752128, 75],
    [212736, 62, -752128, 75],
    [212736, 62, -752128, 75],
    [212736, 62, -752128, 75],
    [212736, 62, -752128, 75],
    [212736, 62, -752128, 75],
    [212736, 62, -752128, 75],
    [212736, 62, -752128, 75],
    [424192, 65, -555520, 78],
    [634368, 67, -293376, 80],
    [843264, 69, -96768, 82],
    [1050880, 71, 99840, 84],
    [1257216, 72, 296448, 85],
    [1462272, 73, 1345024, 100],
    [1666048, 74, 1541632, 101],
    [1868544, 74, 1738240, 101],
    [2069760, 74, 2000384, 101],
    [2269696, 74, 2196992, 101],
    [2468352, 74, 2393600, 101],
    [2665728, 73, 2328064, 58],
    [2861824, 72, 2524672, 57],
    [3056640, 70, 2721280, 55],
    [3250176, 69, 2917888, 54],
    [3442432, 67, 3114496, 52],
    [3633408, 64, 3245568, 49],
    [3823104, 62, 3900928, 40],
    [4011520, 59, 4097536, 37],
    [4198656, 55, 4294144, 33],
    [4384512, 52, 4490752, 30],
    [4569088, 48, 4687360, 26],
    [4752384, 43, 5146112, 38],
    [4934400, 39, 5342720, 34],
    [5115136, 34, 5539328, 29],
    [5294592, 29, 5735936, 24],
    [5472768, 23, 5867008, 18],
    [5649664, 17, 8029696, 63],
    [5825280, 11, 8226304, 57],
    [5999616, 5, 8422912, 51],
    [6172672, 0, 8553984, 46],
    [6434816, 0, 8816128, 46],
    [6696960, 0, 11699712, 25],
    [6959104, 0, 11961856, 25],
    [7155712, 0, 14648832, 28],
    [7417856, 0, 14910976, 28],
    [7090176, 0, 15435264, 32],
    [7352320, 0, 15435264, 32],
];

const HIGH_CAPTURE: [[i32; 4]; 47] = [
    [207616, 71, -743424, 84],
    [207616, 71, -743424, 84],
    [207616, 71, -743424, 84],
    [207616, 71, -743424, 84],
    [207616, 71, -743424, 84],
    [207616, 71, -743424, 84],
    [207616, 71, -743424, 84],
    [207616, 71, -743424, 84],
    [207616, 71, -743424, 84],
    [413952, 72, -546816, 85],
    [619008, 73, -350208, 86],
    [822784, 74, -153600, 87],
    [1025280, 74, 43008, 87],
    [1226496, 74, 305152, 87],
    [1426432, 74, 1353728, 101],
    [1625088, 74, 1550336, 101],
    [1822464, 73, 1746944, 100],
    [2018560, 72, 1943552, 99],
    [2213376, 70, 2140160, 97],
    [2406912, 69, 2336768, 96],
    [2599168, 67, 2271232, 52],
    [2790144, 64, 2402304, 49],
    [2979840, 62, 2598912, 47],
    [3168256, 59, 2795520, 44],
    [3355392, 55, 2992128, 40],
    [3541248, 52, 3188736, 37],
    [3725824, 48, 3844096, 26],
    [3909120, 43, 3975168, 21],
    [4091136, 39, 4171776, 17],
    [4271872, 34, 4368384, 12],
    [4451328, 29, 4564992, 7],
    [4629504, 23, 5023744, 18],
    [4806400, 17, 5220352, 12],
    [4982016, 11, 5416960, 6],
    [5156352, 5, 5613568, 0],
    [5329408, 0, 5744640, -5],
    [5591552, 0, 7972864, 46],
    [5853696, 0, 8235008, 46],
    [6115840, 0, 8497152, 46],
    [6377984, 0, 8759296, 46],
    [6640128, 0, 9021440, 46],
    [6902272, 0, 11905024, 25],
    [7164416, 0, 12167168, 25],
    [6771200, 0, 14264320, 28],
    [7033344, 0, 14526464, 28],
    [6705664, 0, 15050752, 32],
    [6967808, 0, 15050752, 32],
];

const FULL_LOW_HEIGHTS: [i8; 55] = [
    32, 35, 38, 41, 43, 45, 47, 48, 50, 51, 51, 52, 52, 52, 51, 50, 49, 48, 47, 45, 43, 40, 38, 35,
    31, 28, 24, 20, 16, 11, 6, 1, 0, 0, 0, 0, 0, 2, 4, 6, 7, 8, 9, 10, 10, 10, 10, 9, 8, 7, 6, 4,
    2, 0, 0,
];

const FULL_HIGH_HEIGHTS: [i8; 55] = [
    32, 35, 38, 41, 43, 45, 47, 48, 50, 51, 51, 52, 52, 52, 51, 51, 50, 48, 47, 45, 43, 40, 38, 35,
    31, 28, 24, 20, 16, 11, 6, 1, 0, 0, 0, 0, 0, 3, 4, 6, 8, 9, 9, 10, 10, 10, 10, 9, 8, 7, 6, 4,
    2, 0, 0,
];

const SOFT_LOW_HEIGHTS: [i8; 49] = [
    48, 53, 58, 63, 68, 72, 76, 80, 84, 87, 90, 92, 95, 97, 99, 100, 101, 102, 103, 104, 104, 103,
    103, 102, 101, 100, 98, 97, 95, 92, 89, 86, 83, 80, 76, 72, 67, 63, 58, 53, 47, 41, 35, 29, 22,
    15, 8, 1, 0,
];

const SOFT_HIGH_HEIGHTS: [i8; 49] = [
    48, 53, 58, 63, 68, 72, 76, 80, 84, 87, 90, 93, 95, 97, 99, 100, 102, 103, 103, 104, 104, 104,
    103, 102, 101, 100, 99, 97, 95, 92, 90, 87, 83, 80, 76, 72, 68, 63, 58, 53, 47, 41, 35, 29, 22,
    15, 8, 1, 0,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcade_height_and_rectangle_boundaries_are_inclusive() {
        for (falling, minimum) in [(false, 56), (true, 48)] {
            assert!(!eligible(falling, minimum - 1, 24, minimum, 29, 53));
            for gap in [-29, 77] {
                assert!(eligible(falling, minimum, gap, minimum - 16, 29, 53));
                assert!(eligible(falling, minimum, gap, minimum + 16, 29, 53));
            }
            for gap in [-30, 78] {
                assert!(!eligible(falling, minimum, gap, minimum, 29, 53));
            }
            for difference in [-17, 17] {
                assert!(!eligible(
                    falling,
                    minimum,
                    24,
                    minimum + difference,
                    29,
                    53
                ));
            }
        }
    }

    #[test]
    fn both_capture_heights_match_arcade_position_and_life_clocks() {
        let tsv =
            include_str!("../../../../docs/research/ken-super-turbo-air-throw-trajectory.tsv");
        let mut count = 0;
        for row in tsv.lines().skip(1) {
            let cols: Vec<_> = row.split('\t').collect();
            let parsed = |index: usize| cols[index].parse::<i32>().unwrap();
            let (height, gap) = match cols[0] {
                "4" => (59, 2_065_920),
                "7" => (69, 2_140_160),
                _ => panic!("unrecorded replay phase"),
            };
            let softened = cols[2] == "soft";
            let tick = parsed(3) as u8;
            let measured = frame(tick, height, softened, true);
            let expected_life = match (softened, tick) {
                (true, RELEASE..) => 128,
                (false, FULL_LAND..) => 112,
                _ => 144,
            };
            assert_eq!(parsed(14), expected_life, "{row}");
            assert_eq!(parsed(15), if tick < RELEASE { 255 } else { 0 }, "{row}");
            assert_eq!(i32::from(measured.victim_height), parsed(5), "{row}");
            assert_eq!(i32::from(measured.ken_height), parsed(10), "{row}");
            let expected_victim_x = pixels(parsed(4) - gap);
            assert_eq!(measured.victim_x, expected_victim_x, "{row}");
            // The port hands Ken back to normal state at +84. Ignore later
            // sprite offsets when comparing his owned throw position.
            if tick < KEN_READY {
                assert_eq!(measured.ken_x, pixels(parsed(9)), "{row}");
            }
            assert_eq!(measured.victim_turn, parsed(17) == 0, "{row}");
            let mirrored = frame(tick, height, softened, false);
            assert_eq!(
                (mirrored.ken_x, mirrored.victim_x),
                (-measured.ken_x, -measured.victim_x)
            );
            count += 1;
        }
        assert_eq!(count, 1024);
    }
}
