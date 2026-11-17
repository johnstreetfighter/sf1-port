//! Ground Tatsumaki for both Ken loadouts in World 940223.

use super::{header, put_era_header, rom_b, rom_boxes, rom_pose, routine, EraFamily, Rec, Region};

pub(crate) const MOVE: u8 = 0x1b;
pub(crate) const CROUCH_GUARDED_OPENER: u8 = 1 << 6;

pub(crate) fn slot(strength: u8) -> Option<u8> {
    (1..=3).contains(&strength).then(|| 0x30 + strength - 1)
}

// Horizontal movement skips the first descent tick and resumes through landing.
const RISE: [i32; 10] = [4, 7, 10, 13, 16, 18, 20, 21, 22, 23];
const FALL: [i32; 13] = [23, 23, 22, 21, 20, 19, 17, 14, 11, 8, 5, 1, 0];

pub(crate) fn attack_box(strike: u8) -> (i16, i16, u16, u16) {
    match strike {
        1 => (18, 52, 21, 15),
        2 => (35, 67, 34, 16),
        _ => (-35, 67, 34, 16),
    }
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    if region.here() & 1 != 0 {
        region.put(&[0]);
    }
    let rising = region.put(&[super::ROUTINE_ST_SHORYUREPPA_POSTURE, 0xc1]);
    let falling = region.put(&[super::ROUTINE_ST_SHORYUREPPA_POSTURE, 0xc2]);
    let grounded = region.put(&[super::ROUTINE_ST_SHORYUREPPA_POSTURE, 0xc4]);
    let sound = region.put(&[super::ROUTINE_POSE_ORIGIN, 0xa2, 0, 0]);
    // Three fixed-point steps, in eighths of a pixel.
    for (strength, vx) in [(1, 20), (2, 25), (3, 30)] {
        let cycles = strength + 2;
        let fall = 11 + 7 * cycles;
        let ready = fall + 18;
        let hits: [[u32; 3]; 6] = std::array::from_fn(|phase| std::array::from_fn(|index| {
            let mut entry: [u8; 12] = std::array::from_fn(|i| rom_b(rom, 0x46012 + i as u32));
            entry[8] = phase as u8 + 1;
            entry[9] = if phase == 0 { 2 } else { 3 };
            entry[10] = index as u8 + 1;
            region.put(&entry)
        }));
        let start = region.here();
        let first = rom_pose(rom, 0x34e8);
        let mut pose = first;
        let mut top = first.top_on_ground();
        let mut previous_x = 0;
        for tick in 1..ready - 1 {
            if tick == 1 {
                region.call(rising, 0);
                region.call(sound, 0);
            } else if tick == fall {
                region.call(falling, 0);
            } else if tick == fall + 12 {
                region.call(grounded, 0);
                region.call(routine::LANDING, 0);
            }
            let height = if tick <= 10 {
                RISE[usize::from(tick - 1)]
            } else if tick < fall {
                23
            } else {
                FALL.get(usize::from(tick - fall)).copied().unwrap_or(0)
            };
            let (frame, contact) = if tick <= 3 {
                (0x3508, Some((0, 1)))
            } else if tick <= 6 {
                (0x3528, None)
            } else if tick <= 10 {
                (0x3548, None)
            } else if tick < fall {
                let cycles_elapsed = (tick - 11) / 7;
                let opportunity = usize::from(cycles_elapsed + 1);
                match (tick - 11) % 7 {
                    0 | 1 => (0x358e, Some((opportunity, 2))),
                    2 => (0x35b4, None),
                    3 | 4 => (0x35d4, Some((opportunity, 3))),
                    _ => (0x35b4, None),
                }
            } else if height > 0 {
                (0x35fa, None)
            } else {
                (0x3620, None)
            };
            let next = rom_pose(rom, frame);
            let next_top = next.top_on_ground() - height;
            let dy = next_top - (top + pose.half().1 - next.half().1);
            let x = (i32::from(tick.min(fall + 12) - u8::from(tick >= fall)) * vx + 4) / 8;
            let mut boxes = rom_boxes(rom, frame);
            let (flags, hit) = if let Some((phase, strike)) = contact {
                let (x, y, rx, ry) = attack_box(strike);
                boxes[12..16].copy_from_slice(&[
                    (i32::from(y) - next.half().1) as u8,
                    ry as u8,
                    (i32::from(x) - i32::from(next.x_offset())) as u8,
                    rx as u8,
                ]);
                // Front and back kicks share the spin's contact allowance.
                // The strike ID belongs in the frame independently of it.
                (0x80, hits[phase][usize::from(strike - 1)])
            } else {
                boxes[12..16].fill(0);
                (0, 0)
            };
            region.put(&Rec {
                hold: 0, dur: 0, pose: next, dy: dy as i8, dx: (x - previous_x) as i8,
                boxes, flags, x_offset: next.x_offset(), hit,
            }.bytes());
            pose = next;
            top = next_top;
            previous_x = x;
        }
        region.put(&[0xfd, 0]);
        let first_record = super::still(first, 1, rom_boxes(rom, 0x34e8));
        let h = header(&first_record.bytes(), first.top_on_ground(), 0xc4, MOVE, [0, 5, 0x19], start);
        put_era_header(region, EraFamily::Sf2, slot(strength).unwrap(), &h);
    }
}
