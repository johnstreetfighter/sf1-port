//! Ken's free jumps from World Super Turbo. Native Ken air attacks branch
//! out of these streams through the ordinary jump input path.

use super::*;
use crate::era::Loadout;

pub(crate) fn slot(move_id: u8, loadout: Loadout) -> Option<u8> {
    let variant = match loadout {
        Loadout::Standard => 0,
        Loadout::Old => 3,
        _ => return None,
    };
    (9..=11)
        .contains(&move_id)
        .then(|| 0x37 + variant + (move_id - 9))
}

const NEUTRAL: &[u8] = &[
    6, 13, 19, 25, 30, 36, 41, 45, 50, 54, 57, 61, 64, 66, 69, 71, 73, 74, 76, 76, 77, 77, 77, 77,
    76, 75, 74, 72, 70, 68, 65, 63, 59, 56, 52, 48, 43, 39, 34, 28, 23, 17, 10, 4, 0,
];
const FORWARD: &[u8] = &[
    6, 13, 19, 24, 30, 35, 40, 44, 48, 52, 56, 59, 62, 65, 67, 69, 71, 72, 73, 74, 74, 74, 74, 74,
    73, 72, 70, 69, 67, 64, 62, 59, 55, 52, 48, 43, 39, 34, 29, 23, 17, 11, 5, 0,
];
const BACK: &[u8] = &[
    6, 13, 19, 25, 31, 36, 42, 46, 51, 55, 59, 62, 65, 68, 71, 73, 75, 77, 78, 79, 80, 80, 80, 80,
    79, 78, 77, 76, 74, 72, 69, 67, 63, 60, 56, 52, 48, 43, 39, 33, 28, 22, 16, 9, 2, 0,
];

fn horizontal_position(ticks: i32, speed: i32) -> i32 {
    if speed == 0 {
        return 0;
    }
    let distance = ticks * speed - ticks * (ticks - 1) * 0x280;
    (distance + 0x8000) >> 16
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    let airborne = region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, 0x81]);
    let grounded = region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, 0x82]);
    let crouch = rom_pose(rom, 0x2828);
    let settle = rom_pose(rom, 0x2848);
    for (move_id, heights, speed, sign, poses, boundaries) in [
        (
            9,
            NEUTRAL,
            0,
            0,
            [0x273c, 0x277c, 0x277c, 0x2802],
            [9, 24, 35],
        ),
        (
            10,
            FORWARD,
            0x37b00,
            1,
            [0x286a, 0x28aa, 0x28ea, 0x2910],
            [7, 19, 34],
        ),
        (
            11,
            BACK,
            0x3fb00,
            -1,
            [0x2978, 0x29b8, 0x29f8, 0x2a1e],
            [7, 19, 35],
        ),
    ] {
        let start = region.here();
        let mut stream = Stream::new(region, crouch, crouch.top_on_ground());
        stream.call(airborne, 0);
        for (index, &height) in heights.iter().enumerate() {
            let next = index as i32 + 1;
            let dx =
                sign * (horizontal_position(next, speed) - horizontal_position(next - 1, speed));
            if height == 0 {
                stream.call(grounded, 0);
                stream.call(routine::LANDING, 0);
                stream.travel(
                    crouch,
                    crouch.top_on_ground(),
                    1,
                    (0, dx),
                    rom_boxes(rom, 0x2828),
                    0,
                    0,
                );
            } else {
                let frame = poses[boundaries.partition_point(|&tick| tick <= index)];
                let pose = rom_pose(rom, frame);
                stream.travel(
                    pose,
                    pose.top_on_ground() - i32::from(height),
                    1,
                    (0, dx),
                    rom_boxes(rom, frame),
                    0,
                    0,
                );
            }
        }
        stream.hold(
            crouch,
            crouch.top_on_ground(),
            3,
            rom_boxes(rom, 0x2828),
            0,
            0,
        );
        stream.hold(
            settle,
            settle.top_on_ground(),
            2,
            rom_boxes(rom, 0x2848),
            0,
            0,
        );
        stream.end();
        for (loadout, startup) in [(Loadout::Standard, 5), (Loadout::Old, 4)] {
            let pose = still(crouch, startup, rom_boxes(rom, 0x2828));
            let h = header(
                &pose.bytes(),
                crouch.top_on_ground(),
                0x82,
                move_id,
                [0x82, 5, 0x19],
                start,
            );
            put_era_header(region, EraFamily::Sf2, slot(move_id, loadout).unwrap(), &h);
        }
    }
}
