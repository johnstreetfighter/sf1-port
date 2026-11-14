//! World 940223 Ken jump attack phases, using the resident Ken art.

use super::{rom_b, rom_boxes, rom_pose, still, Region, ERA_HEADER_END};

pub(super) const POINTER_BYTES: usize = 24 * 4;
const HEADERS: u32 = ERA_HEADER_END + super::st_ken_throws::POINTER_BYTES as u32;

pub(super) fn header_offset(old: bool, directional: bool, button: u8) -> u32 {
    HEADERS + (12 * u32::from(old) + 6 * u32::from(directional) + u32::from(button)) * 4
}

// Every row is a pose change on the attack clock. Includes the last recovery
// pose, which the jump's own floor transition can replace at any age.
type Phase = (u8, u32, u8);
const NEUTRAL: [&[Phase]; 6] = [
    &[(0, 0x3662, 0), (2, 0x3682, 45)],
    &[
        (0, 0x3662, 0),
        (2, 0x3662, 0),
        (4, 0x3682, 12),
        (24, 0x36a2, 0),
        (28, 0x36c2, 0),
        (32, 0x36c2, 0),
        (36, 0x36e2, 0),
    ],
    &[
        (0, 0x3662, 0),
        (2, 0x3662, 0),
        (4, 0x3682, 46),
        (24, 0x36a2, 0),
        (28, 0x36c2, 0),
        (32, 0x36c2, 0),
        (36, 0x36e2, 0),
    ],
    &[(0, 0x38da, 0), (3, 0x38fa, 27)],
    &[
        (0, 0x38da, 0),
        (5, 0x38fa, 28),
        (18, 0x393a, 0),
        (24, 0x395a, 0),
        (30, 0x397a, 0),
    ],
    &[
        (0, 0x39e2, 0),
        (2, 0x3a02, 25),
        (6, 0x3a22, 26),
        (10, 0x3a48, 0),
        (14, 0x3a68, 0),
        (17, 0x36c2, 0),
        (20, 0x36e2, 0),
    ],
];

const DIRECTIONAL: [&[Phase]; 6] = [
    &[(0, 0x374a, 0), (2, 0x376a, 11)],
    &[
        (0, 0x374a, 0),
        (2, 0x374a, 0),
        (4, 0x376a, 13),
        (12, 0x378a, 0),
        (16, 0x37aa, 0),
        (20, 0x37aa, 0),
        (24, 0x37aa, 0),
    ],
    &[
        (0, 0x374a, 0),
        (2, 0x374a, 0),
        (4, 0x376a, 47),
        (12, 0x378a, 0),
        (16, 0x37aa, 0),
        (20, 0x37aa, 0),
        (24, 0x37aa, 0),
    ],
    &[
        (0, 0x3ad0, 0),
        (2, 0x3ad0, 0),
        (5, 0x3af0, 35),
        (15, 0x3af0, 35),
    ],
    &[
        (0, 0x3ad0, 0),
        (1, 0x3ad0, 0),
        (3, 0x3ad0, 0),
        (5, 0x3af0, 29),
        (10, 0x38fa, 73),
        (18, 0x3b10, 0),
        (21, 0x3b30, 0),
        (24, 0x3b30, 0),
        (27, 0x3b30, 0),
    ],
    &[
        (0, 0x3ad0, 0),
        (2, 0x3ad0, 0),
        (5, 0x3af0, 30),
        (12, 0x3b10, 0),
        (15, 0x3b30, 0),
        (18, 0x3b30, 0),
        (21, 0x3b30, 0),
    ],
];

const OLD_DIRECTIONAL_LK: &[Phase] = &[(0, 0x3ad0, 0), (2, 0x3ad0, 0), (5, 0x3af0, 35)];
const OLD_DIRECTIONAL_MK: &[Phase] = &[
    (0, 0x3ad0, 0),
    (2, 0x3ad0, 0),
    (5, 0x3af0, 29),
    (18, 0x3b10, 0),
    (21, 0x3b30, 0),
    (24, 0x3b30, 0),
    (27, 0x3b30, 0),
];

fn strike(id: u8, old: bool) -> [i8; 4] {
    match id {
        11 => [29, 74, 21, 16],
        12 | 13 => [30, 81, 31, 11],
        25 => [18, 70, 25, 20],
        26 if old => [34, 56, 46, 25],
        26 => [30, 45, 42, 14],
        27 => [10, 75, 34, 19],
        28 => [16, 75, 39, 19],
        29 if old => [24, 61, 49, 12],
        29 => [-2, 51, 25, 18],
        30 if old => [30, 55, 56, 18],
        30 => [43, 55, 43, 18],
        35 if old => [13, 56, 26, 15],
        35 => [15, 50, 19, 15],
        45 => [29, 77, 21, 19],
        46 | 47 => [33, 59, 31, 9],
        73 => [45, 61, 35, 9],
        _ => [0; 4],
    }
}

/// The three source hurtboxes are represented by a torso and a union in the
/// port's two-box format. This keeps the exposed limbs for mixed-era contacts.
fn body(old: bool, directional: bool, button: u8, age: u8, id: u8) -> ([i8; 4], [i8; 4]) {
    let torso = if directional && button == 2 && age < 12 {
        [-6, 75, 20, 14]
    } else {
        [-6, 68, 20, 21]
    };
    let head = if directional && button == 0 && age == 0 {
        torso
    } else {
        [1, 96, 14, 9]
    };
    let legs = match (old, directional, button, id) {
        (false, false, _, 26) => [27, 65, 20, 13],
        (false, true, _, 29 | 30 | 73) => [31, 69, 25, 13],
        (_, true, 2, _) if age < 12 => torso,
        (false, true, 3, _) if age < 15 => torso,
        _ => [-6, 41, 20, 13],
    };
    let [left, right, bottom, top] = [head, torso, legs].into_iter().fold(
        [i8::MAX, i8::MIN, i8::MAX, i8::MIN],
        |[l, r, b, t], [x, y, rx, ry]| [l.min(x - rx), r.max(x + rx), b.min(y - ry), t.max(y + ry)],
    );
    let loose = [
        (left + right) / 2,
        bottom + (top - bottom) / 2,
        (right - left + 1) / 2,
        (top - bottom + 1) / 2,
    ];
    (torso, loose)
}

pub(crate) fn clean_hp(old: bool, direction: u8, button: u8, crouching: bool) -> u16 {
    const STANDING: [[[u16; 6]; 3]; 2] = [
        [
            [11, 19, 22, 12, 19, 25],
            [13, 20, 23, 11, 22, 22],
            [11, 22, 21, 11, 22, 22],
        ],
        [
            [11, 19, 22, 12, 19, 27],
            [11, 17, 26, 9, 17, 24],
            [10, 22, 26, 11, 19, 26],
        ],
    ];
    // A zero represents no crouching contact in the source fixture.
    const CROUCHING: [[[u16; 6]; 3]; 2] = [
        [
            [0, 0, 19, 12, 19, 21],
            [0, 0, 19, 11, 22, 22],
            [0, 0, 21, 0, 19, 22],
        ],
        [
            [0, 0, 24, 11, 18, 26],
            [0, 0, 26, 9, 19, 24],
            [0, 0, 24, 0, 21, 26],
        ],
    ];
    let index = (
        usize::from(old),
        usize::from(direction - 9),
        usize::from(button),
    );
    let crouch_hp = CROUCHING[index.0][index.1][index.2];
    if crouching && crouch_hp != 0 {
        crouch_hp
    } else {
        STANDING[index.0][index.1][index.2]
    }
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    // Use the ordinary jump-hit reaction table for air-to-air and mixed-era
    // contacts. The verified grounded New Ryu match supplies its own recoil.
    let hit = region.put(&std::array::from_fn::<_, 8, _>(|i| {
        rom_b(rom, 0x45c88 + i as u32)
    }));
    for old in [false, true] {
        for directional in [false, true] {
            for button in 0..6 {
                let phases = match (old, directional, button) {
                    (true, true, 3) => OLD_DIRECTIONAL_LK,
                    (true, true, 4) => OLD_DIRECTIONAL_MK,
                    (_, true, _) => DIRECTIONAL[usize::from(button)],
                    _ => NEUTRAL[usize::from(button)],
                };
                let mut entries = Vec::with_capacity(1 + phases.len() * 5);
                entries.push(phases.len() as u8);
                for &(age, frame, attack) in phases {
                    let pose = rom_pose(rom, frame);
                    let mut boxes = rom_boxes(rom, frame);
                    let for_pose = |[x, y, rx, ry]: [i8; 4]| {
                        [
                            (i32::from(y) - pose.half().1) as u8,
                            ry as u8,
                            (i32::from(x) - i32::from(pose.x_offset())) as u8,
                            rx as u8,
                        ]
                    };
                    let (tight, loose) = body(old, directional, button, age, attack);
                    boxes[4..8].copy_from_slice(&for_pose(tight));
                    boxes[8..12].copy_from_slice(&for_pose(loose));
                    if attack == 0 {
                        boxes[12..16].fill(0);
                    } else {
                        boxes[12..16].copy_from_slice(&for_pose(strike(attack, old)));
                    }
                    let mut rec = still(pose, 1, boxes);
                    rec.flags = if attack == 0 { 0 } else { 0x80 };
                    rec.hit = if attack == 0 { 0 } else { hit };
                    let address = region.put(&rec.bytes());
                    entries.push(age);
                    entries.extend(address.to_be_bytes());
                }
                let address = region.put(&entries);
                region.patch(
                    region.base + header_offset(old, directional, button),
                    &address.to_be_bytes(),
                );
            }
        }
    }
}
