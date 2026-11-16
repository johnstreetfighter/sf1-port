//! World Super Turbo 2 specials using Sagat's native poses.

use super::*;

const KNEE: u8 = 0x19;
const UPPERCUT: u8 = 0x1f;
const GENOCIDE: u8 = SF2_SUPER_SLOT;
const FIRST_KNEE_SLOT: u8 = 0x30;
const FIRST_UPPERCUT_SLOT: u8 = 0x33;
const NEW_HP_UPPERCUT_SLOT: u8 = 0x36;

const READY: u32 = 0x13f10;
const SQUAT: u32 = 0x14242;
const LIFT: u32 = 0x14268;
const KNEE_STRIKE: u32 = 0x1412c;
const KNEE_EXTEND: u32 = 0x14076;
const UPPERCUT_STRIKE: u32 = 0x13f36;

const GROUNDED: u8 = 0xc4;
const RISING: u8 = 0xc1;
const FALLING: u8 = 0xc2;

/// The move IDs in the headers remain the public Knee and Uppercut IDs.
pub(crate) fn slot(move_id: u8, strength: u8, old: bool) -> Option<u8> {
    match (move_id, strength, old) {
        (KNEE, 1..=3, _) => Some(FIRST_KNEE_SLOT + strength - 1),
        (UPPERCUT, 3, false) => Some(NEW_HP_UPPERCUT_SLOT),
        (UPPERCUT, 1..=3, _) => Some(FIRST_UPPERCUT_SLOT + strength - 1),
        (GENOCIDE, _, false) => Some(GENOCIDE),
        _ => None,
    }
}

/// Facing-relative X, height above Sagat's floor axis, and the two box radii.
pub(crate) fn attack_box(move_id: u8, phase: u8) -> Option<(i32, i32, u8, u8)> {
    match (move_id, phase) {
        (KNEE, 1) => Some((30, 18, 18, 19)),
        (KNEE, 2) => Some((27, 67, 29, 15)),
        (KNEE, 3) => Some((36, 61, 32, 15)),
        (UPPERCUT, 1) => Some((42, 31, 28, 27)),
        (UPPERCUT, 2) => Some((57, 68, 42, 18)),
        (UPPERCUT, 3..=5) => Some((61, 118, 33, 39)),
        (GENOCIDE, 1..=3) => attack_box(KNEE, phase),
        (GENOCIDE, 4..=6) => attack_box(UPPERCUT, phase - 3),
        _ => None,
    }
}

// Cumulative forward displacement and floor-relative height from
// docs/research/sagat-super-turbo-melee-whiff-trajectory.tsv. X is rounded
// once in cumulative form before conversion to the integer pose steps.
#[rustfmt::skip]
const LP_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0),
    (0, 0), (0, 0), (0, 0), (0, 0), (4, 5), (7, 10), (9, 15), (11, 20),
    (13, 24), (14, 28), (14, 31), (14, 34), (14, 37), (14, 37), (14, 39), (14, 41),
    (14, 42), (14, 43), (14, 44), (14, 45), (14, 45), (14, 44), (14, 43), (14, 42),
    (14, 41), (14, 39), (14, 37), (14, 34), (14, 31), (14, 28), (14, 24), (14, 20),
    (14, 15), (14, 10), (14, 5), (14, 0), (14, 0), (14, 0), (14, 0), (14, 0),
    (14, 0), (14, 0), (14, 0), (14, 0),
];

#[rustfmt::skip]
const MP_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0),
    (0, 0), (0, 0), (0, 0), (0, 0), (6, 7), (11, 14), (15, 21), (19, 28),
    (23, 34), (26, 40), (28, 45), (30, 50), (32, 55), (33, 59), (33, 63), (33, 66),
    (33, 69), (33, 69), (33, 72), (33, 75), (33, 77), (33, 78), (33, 79), (33, 80),
    (33, 81), (33, 81), (33, 81), (33, 80), (33, 79), (33, 78), (33, 76), (33, 74),
    (33, 71), (33, 68), (33, 65), (33, 62), (33, 58), (33, 53), (33, 48), (33, 43),
    (33, 38), (33, 32), (33, 26), (33, 19), (33, 12), (33, 5), (33, 0), (33, 0),
    (33, 0), (33, 0), (33, 0), (33, 0), (33, 0), (33, 0),
];

#[rustfmt::skip]
const HP_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0),
    (0, 0), (0, 0), (0, 0), (0, 0), (8, 8), (15, 16), (21, 24), (27, 32),
    (33, 39), (38, 46), (42, 52), (46, 58), (50, 64), (53, 69), (55, 74), (57, 78),
    (59, 82), (60, 86), (60, 90), (60, 93), (60, 95), (60, 95), (60, 97), (60, 99),
    (60, 101), (60, 102), (60, 103), (60, 103), (60, 103), (60, 103), (60, 102), (60, 101),
    (60, 99), (60, 97), (60, 95), (60, 93), (60, 90), (60, 86), (60, 82), (60, 78),
    (60, 74), (60, 69), (60, 64), (60, 58), (60, 52), (60, 46), (60, 39), (60, 32),
    (60, 24), (60, 16), (60, 8), (60, 0), (60, 0), (60, 0), (60, 0), (60, 0),
    (60, 0), (60, 0), (60, 0), (60, 0), (60, 0), (60, 0), (60, 0), (60, 0),
];

#[rustfmt::skip]
const LK_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (10, 0), (10, 0), (10, 0), (17, 0), (17, 0), (17, 0), (32, 0),
    (32, 0), (41, 9), (50, 17), (59, 23), (68, 27), (77, 30), (86, 31), (95, 31),
    (96, 31), (97, 30), (98, 29), (99, 27), (100, 25), (101, 23), (102, 21), (103, 18),
    (104, 14), (105, 10), (106, 6), (107, 2), (108, 0), (108, 0), (108, 0), (108, 0),
    (108, 0), (108, 0), (108, 0), (108, 0),
];

#[rustfmt::skip]
const MK_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (18, 0), (18, 0), (18, 0), (25, 0), (25, 0), (25, 0), (45, 0),
    (45, 0), (54, 10), (62, 19), (71, 26), (79, 31), (88, 35), (96, 37), (105, 38),
    (113, 37), (114, 37), (115, 36), (116, 35), (117, 33), (118, 31), (119, 29), (120, 27),
    (121, 24), (122, 20), (123, 16), (124, 12), (125, 8), (126, 3), (127, 0), (127, 0),
    (127, 0), (127, 0), (127, 0), (127, 0), (127, 0), (127, 0),
];

#[rustfmt::skip]
const HK_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (26, 0), (26, 0), (26, 0), (26, 0), (33, 0), (33, 0), (33, 0),
    (58, 0), (58, 0), (66, 11), (74, 21), (82, 29), (90, 35), (98, 40), (106, 43),
    (114, 45), (122, 45), (130, 43), (131, 43), (132, 42), (133, 41), (134, 40), (135, 38),
    (136, 36), (137, 33), (138, 30), (139, 27), (140, 23), (141, 19), (142, 14), (143, 9),
    (144, 4), (145, 0), (145, 0), (145, 0), (145, 0), (145, 0), (145, 0), (145, 0),
    (145, 0),
];

// World Turbo 2 whiff path after the 18-tick flash. The trace's initial
// horizontal fraction is zero, so use its integer world X samples.
#[rustfmt::skip]
const GENOCIDE_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (0, 0), (0, 0), (26, 0), (26, 0), (26, 0), (26, 0), (33, 0),
    (33, 0), (33, 0), (48, 0), (48, 0), (57, 9), (66, 17), (75, 23), (84, 27),
    (93, 30), (102, 31), (111, 31), (112, 31), (113, 30), (114, 29), (115, 27), (116, 25),
    (117, 23), (118, 21), (119, 18), (120, 14), (121, 10), (122, 6), (123, 2), (124, 0),
    (124, 0), (124, 0), (124, 0), (124, 0), (124, 0), (124, 0), (124, 0), (124, 0),
    (124, 0), (124, 0), (124, 0), (131, 9), (131, 17), (131, 25), (131, 32), (131, 39),
    (131, 46), (131, 52), (131, 58), (131, 64), (131, 69), (131, 74), (131, 79), (131, 83),
    (131, 87), (131, 90), (131, 93), (131, 96), (131, 98), (131, 100), (131, 101), (131, 102),
    (131, 103), (131, 103), (131, 103), (131, 103), (131, 102), (131, 101), (131, 100), (131, 98),
    (131, 96), (131, 93), (131, 90), (131, 87), (131, 83), (131, 79), (131, 74), (131, 69),
    (131, 64), (131, 58), (131, 52), (131, 46), (131, 39), (131, 32), (131, 25), (131, 17),
    (131, 9), (131, 0), (131, 0), (131, 0), (131, 0), (131, 0), (131, 0), (131, 0),
    (131, 0), (131, 0), (131, 0), (131, 0), (131, 0),
];

const GENOCIDE_PROFILE: Profile = Profile {
    trajectory: GENOCIDE_TRAJECTORY,
    strikes: &[(6, 8), (9, 11), (12, 17), (34, 37), (38, 40), (41, 67)],
    poses: &[
        (0, SQUAT),
        (3, LIFT),
        (6, KNEE_STRIKE),
        (12, KNEE_EXTEND),
        (18, KNEE_STRIKE),
        (23, LIFT),
        (31, SQUAT),
        (34, UPPERCUT_STRIKE),
        (68, LIFT),
        (89, SQUAT),
        (97, READY),
    ],
    posture: &[
        (11, RISING),
        (19, FALLING),
        (31, GROUNDED),
        (42, RISING),
        (68, FALLING),
        (89, GROUNDED),
    ],
};

struct Profile {
    trajectory: &'static [(u8, u8)],
    strikes: &'static [(u8, u8)],
    poses: &'static [(u8, u32)],
    // Air, falling, and ground state writes can lead or lag physical Y.
    posture: &'static [(u8, u8)],
}

const KNEES: [Profile; 3] = [
    Profile {
        trajectory: LK_TRAJECTORY,
        strikes: &[(4, 6), (7, 9), (10, 14)],
        posture: &[(8, RISING), (15, FALLING), (28, GROUNDED)],
        poses: &[
            (0, SQUAT),
            (1, LIFT),
            (4, KNEE_STRIKE),
            (10, KNEE_EXTEND),
            (15, KNEE_STRIKE),
            (21, LIFT),
            (28, SQUAT),
            (33, READY),
        ],
    },
    Profile {
        trajectory: MK_TRAJECTORY,
        strikes: &[(4, 6), (7, 9), (10, 15)],
        posture: &[(8, RISING), (16, FALLING), (30, GROUNDED)],
        poses: &[
            (0, SQUAT),
            (1, LIFT),
            (4, KNEE_STRIKE),
            (10, KNEE_EXTEND),
            (16, KNEE_STRIKE),
            (22, LIFT),
            (30, SQUAT),
            (35, READY),
        ],
    },
    Profile {
        trajectory: HK_TRAJECTORY,
        strikes: &[(5, 7), (8, 10), (11, 17)],
        posture: &[(9, RISING), (18, FALLING), (33, GROUNDED)],
        poses: &[
            (0, SQUAT),
            (1, LIFT),
            (5, KNEE_STRIKE),
            (11, KNEE_EXTEND),
            (18, KNEE_STRIKE),
            (24, LIFT),
            (33, SQUAT),
            (38, READY),
        ],
    },
];

const HP_UPPERCUT_POSES: &[(u8, u32)] = &[
    (0, READY),
    (1, SQUAT),
    (3, UPPERCUT_STRIKE),
    (36, LIFT),
    (61, SQUAT),
    (69, READY),
];

const UPPERCUTS: [Profile; 4] = [
    Profile {
        trajectory: LP_TRAJECTORY,
        strikes: &[(3, 6), (7, 9), (10, 23)],
        posture: &[(11, RISING), (21, FALLING), (44, GROUNDED)],
        poses: &[
            (0, READY),
            (1, SQUAT),
            (3, UPPERCUT_STRIKE),
            (24, LIFT),
            (45, SQUAT),
            (49, READY),
        ],
    },
    Profile {
        trajectory: MP_TRAJECTORY,
        strikes: &[(3, 6), (7, 9), (10, 31)],
        posture: &[(11, RISING), (25, FALLING), (54, GROUNDED)],
        poses: &[
            (0, READY),
            (1, SQUAT),
            (3, UPPERCUT_STRIKE),
            (32, LIFT),
            (55, SQUAT),
            (59, READY),
        ],
    },
    Profile {
        trajectory: HP_TRAJECTORY,
        strikes: &[(3, 6), (7, 9), (10, 35)],
        posture: &[(11, RISING), (29, FALLING), (60, GROUNDED)],
        poses: HP_UPPERCUT_POSES,
    },
    Profile {
        trajectory: HP_TRAJECTORY,
        strikes: &[(3, 6), (7, 9), (10, 13), (14, 17), (18, 35)],
        posture: &[(11, RISING), (29, FALLING), (60, GROUNDED)],
        poses: HP_UPPERCUT_POSES,
    },
];

struct StreamEntries {
    hits: [u32; 6],
    rising: u32,
    falling: u32,
    grounded: u32,
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    let hits = std::array::from_fn(|index| {
        let mut entry: [u8; 9] = std::array::from_fn(|offset| rom_b(rom, 0x45f52 + offset as u32));
        entry[8] = index as u8 + 1;
        region.put(&entry)
    });
    let entries = StreamEntries {
        hits,
        rising: region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, RISING]),
        falling: region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, FALLING]),
        grounded: region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, GROUNDED]),
    };

    for (index, profile) in KNEES.iter().enumerate() {
        write_stream(
            rom,
            region,
            KNEE,
            FIRST_KNEE_SLOT + index as u8,
            profile,
            &entries,
        );
    }
    for (index, profile) in UPPERCUTS.iter().enumerate() {
        write_stream(
            rom,
            region,
            UPPERCUT,
            FIRST_UPPERCUT_SLOT + index as u8,
            profile,
            &entries,
        );
    }
    write_stream(rom, region, GENOCIDE, GENOCIDE, &GENOCIDE_PROFILE, &entries);
}

fn write_stream(
    rom: &[u8],
    region: &mut Region,
    move_id: u8,
    bank_slot: u8,
    profile: &Profile,
    entries: &StreamEntries,
) {
    let stream = region.here();
    let first_frame = profile.poses[0].1;
    let first_pose = rom_pose(rom, first_frame);
    let first = still(first_pose, 1, rom_boxes(rom, first_frame));
    let mut pose = first_pose;
    let mut top = pose.top_on_ground();

    // The header represents entry tick 0. The last two stationary samples
    // belong to the end record and the return to neutral on the next tick.
    for tick in 1..profile.trajectory.len() - 2 {
        let (x, height) = profile.trajectory[tick];
        let previous_x = profile.trajectory[tick - 1].0;
        let tick = tick as u8;
        if let Some(&(_, posture)) = profile.posture.iter().find(|&&(at, _)| at == tick) {
            let entry = match posture {
                RISING => entries.rising,
                FALLING => entries.falling,
                _ => entries.grounded,
            };
            region.call(entry, 0);
        }
        if move_id == KNEE && tick == profile.strikes[0].0 {
            region.call(routine::SOUND, 0xa2);
        }
        let frame = profile
            .poses
            .iter()
            .rev()
            .find(|&&(start, _)| tick >= start)
            .expect("Sagat's first pose starts on tick zero")
            .1;
        let phase = profile
            .strikes
            .iter()
            .position(|&(start, end)| (start..=end).contains(&tick))
            .map_or(0, |index| index as u8 + 1);
        let next_pose = rom_pose(rom, frame);
        let next_top = next_pose.top_on_ground() - i32::from(height);
        let dy = next_top - (top + pose.half().1 - next_pose.half().1);
        let mut boxes = rom_boxes(rom, frame);
        let (flags, hit) = if let Some((x, y, rx, ry)) = attack_box(move_id, phase) {
            boxes[12..16].copy_from_slice(&[
                (y - next_pose.half().1) as u8,
                ry,
                (x - i32::from(next_pose.x_offset())) as u8,
                rx,
            ]);
            // Consecutive records share the same strike ID. Collision consumes
            // the phase once per activation, including when the next record loads.
            (0x80, entries.hits[usize::from(phase - 1)])
        } else {
            boxes[12..16].fill(0);
            (0, 0)
        };
        region.put(
            &Rec {
                hold: 0,
                dur: 0,
                pose: next_pose,
                dy: dy as i8,
                dx: (i16::from(x) - i16::from(previous_x)) as i8,
                boxes,
                flags,
                x_offset: next_pose.x_offset(),
                hit,
            }
            .bytes(),
        );
        pose = next_pose;
        top = next_top;
    }
    region.end();

    let sounds = if move_id == KNEE {
        [0, 0x05, 0x19]
    } else {
        [0xa2, 0, 0]
    };
    let h = header(
        &first.bytes(),
        first_pose.top_on_ground(),
        GROUNDED,
        move_id,
        sounds,
        stream,
    );
    put_era_header(region, EraFamily::Sf2, bank_slot, &h);
}
