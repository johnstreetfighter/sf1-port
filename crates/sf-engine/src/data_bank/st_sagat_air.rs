//! World 940223 Sagat's jump-normal geometry and measured flight.

use super::*;

const FIRST_NEUTRAL: u8 = 0x20;
const OLD_DIRECTIONAL_MK: u8 = 0x2d;
pub(crate) const LIFT: u32 = 0x14268;
pub(crate) const LAND: u32 = 0x14242;

pub(crate) fn slot(button: u8, directional: bool, old: bool) -> u8 {
    if directional && old && button == 4 {
        OLD_DIRECTIONAL_MK
    } else {
        let slot = FIRST_NEUTRAL + u8::from(directional) * 6 + button;
        // The family table also holds the public Tiger Genocide slot.
        slot + u8::from(slot >= SF2_SUPER_SLOT)
    }
}

pub(crate) fn active(button: u8, directional: bool, age: u8) -> bool {
    let (first, end) = match button {
        0 => (3, if directional { 27 } else { u8::MAX }),
        1 => (3, 17),
        2 => (5, if directional { 13 } else { 15 }),
        3 => (2, u8::MAX),
        4 => (3, 23),
        5 => (5, 13),
        _ => return false,
    };
    (first..end).contains(&age)
}

/// The flight clock starts one tick before the first positive height.
pub(crate) fn jump_position(direction: u8, flight: u8) -> (i32, u16) {
    const NEUTRAL: &[u8] = &[
        0, 7, 13, 20, 26, 31, 37, 42, 46, 50, 54, 58, 61, 64, 67, 69, 71, 72, 73, 74, 75, 75, 75,
        74, 73, 72, 71, 69, 66, 64, 61, 58, 54, 50, 46, 41, 36, 31, 25, 19, 13, 6, 0,
    ];
    const FORWARD: &[u8] = &[
        0, 6, 13, 19, 25, 30, 35, 40, 44, 48, 52, 55, 58, 61, 63, 65, 67, 68, 69, 70, 70, 70, 69,
        69, 67, 66, 64, 62, 59, 57, 53, 50, 46, 42, 37, 32, 27, 21, 16, 9, 3, 0,
    ];
    const BACK: &[u8] = &[
        0, 7, 14, 20, 27, 32, 38, 43, 48, 53, 57, 61, 64, 67, 70, 73, 75, 77, 78, 79, 80, 80, 80,
        80, 79, 78, 77, 75, 73, 71, 68, 65, 62, 58, 54, 50, 45, 40, 35, 29, 23, 16, 9, 2, 0,
    ];
    let path = match direction {
        0x0a => FORWARD,
        0x0b => BACK,
        _ => NEUTRAL,
    };
    let ticks = i32::from(flight);
    // Horizontal 16.16 speed decreases by 0x500 per tick. The last two
    // backward steps travel one pixel less each.
    let drift = |ticks, speed| ticks * speed - ticks * (ticks - 1) * 640;
    let x = match direction {
        0x0a => drift(ticks.min(41), 170_752),
        0x0b => -drift(ticks.min(44), 178_944) + (ticks - 42).clamp(0, 2) * 65_536,
        _ => 0,
    };
    (
        x,
        u16::from(path.get(usize::from(flight)).copied().unwrap_or(0)),
    )
}

pub(crate) fn landing(direction: u8) -> u8 {
    match direction {
        0x0a => 41,
        0x0b => 44,
        _ => 42,
    }
}

pub(super) fn build(rom: &[u8], region: &mut Region, catalogue: &Catalogue) -> Result<(), String> {
    const NEUTRAL: [[u8; 4]; 6] = [
        [56, 96, 27, 11],
        [61, 96, 35, 11],
        [55, 75, 23, 20],
        [61, 87, 28, 10],
        [65, 65, 35, 8],
        [70, 58, 41, 15],
    ];
    const DIRECTIONAL: [[u8; 4]; 6] = [
        [56, 89, 27, 18],
        [62, 95, 35, 12],
        [65, 70, 25, 9],
        [42, 64, 25, 18],
        [46, 65, 54, 8],
        [67, 60, 38, 13],
    ];
    let palette = rom_w(rom, TournamentFighter::Sagat.move_table() + 0x32) & 0xff;
    for (directional, old) in [(false, false), (true, false), (true, true)] {
        let bounds = if directional { DIRECTIONAL } else { NEUTRAL };
        for (button, mut strike) in bounds.into_iter().enumerate() {
            if old && button != 4 {
                continue;
            }
            if old {
                strike = [65, 65, 35, 8];
            }
            let name = match (directional, button) {
                (false, 0..=2) => "jump_punch",
                (true, 0..=2) => "jump_punch_down",
                (false, 3..=4) => "jump_kick",
                (false, _) => "jump_kick_high",
                (true, _) => "jump_kick_down",
            };
            let pose = catalogue_pose(catalogue, name, palette)?;
            let mut boxes = boxes_named(&sagat::BOXES, name).bytes(&pose);
            let [x, y, rx, ry] = strike;
            boxes[12..16].copy_from_slice(&[
                (i32::from(y) - pose.half().1) as u8,
                ry,
                (i32::from(x) - i32::from(pose.x_offset())) as u8,
                rx,
            ]);
            let mut reaction = [0; 12];
            for (offset, byte) in reaction[..8].iter_mut().enumerate() {
                *byte = rom_b(rom, 0x45c88 + offset as u32);
            }
            reaction[1] = [12, 21, 27, 14, 21, 21][button];
            reaction[8..12].copy_from_slice(&strike);
            let reaction = region.put(&reaction);
            let mut frame = still(pose, 1, boxes);
            frame.flags = 0x80;
            frame.hit = reaction;
            let address = region.put(&frame.bytes());
            bind_era_header(
                region,
                EraFamily::Sf2,
                slot(button as u8, directional, old),
                address,
            );
        }
    }
    Ok(())
}
