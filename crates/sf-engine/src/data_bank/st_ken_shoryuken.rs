//! Ordinary Shoryuken for both Ken loadouts in World 940223 at Turbo 2.
//!
//! Evidence: docs/research/ken-super-turbo-shoryuken.md.

use super::*;
use crate::era::Loadout;

pub(crate) const MOVE: u8 = 0x19;
const FIRST_SLOT: u8 = 0x33;

/// The heavy punch's close opener, which only it has.
pub(crate) const OPENER: u8 = 1;
/// The lower rising strike.
pub(crate) const LOWER: u8 = 2;
/// The upper rising strike, which lasts until the punch retracts.
pub(crate) const UPPER: u8 = 3;

const STRIKE: u32 = 0x32de;
const EXTENDED: u32 = 0x331e;
const RETRACT: u32 = 0x339e;

const GROUNDED: u8 = 0xc4;
const RISING: u8 = 0xc1;
const FALLING: u8 = 0xc2;

/// The bank slot for a punch strength. Only the heavy opener differs by loadout.
pub(crate) fn slot(strength: u8, loadout: Loadout) -> Option<u8> {
    match (strength, loadout) {
        (1 | 2, _) => Some(FIRST_SLOT + strength - 1),
        (3, Loadout::Old) => Some(FIRST_SLOT + 3),
        (3, _) => Some(FIRST_SLOT + 2),
        _ => None,
    }
}

/// Facing-relative X, height above Ken's floor axis, and the two box radii.
pub(crate) fn attack_box(phase: u8, loadout: Loadout) -> Option<(i16, i16, u16, u16)> {
    match phase {
        OPENER if loadout == Loadout::Old => Some((9, 49, 21, 13)),
        OPENER => Some((11, 49, 21, 13)),
        LOWER => Some((30, 45, 28, 22)),
        UPPER => Some((27, 79, 26, 44)),
        _ => None,
    }
}

/// Clean and guarded damage on the 144-point arcade bar. The arcade rolls
/// damage. These are the values most often seen against New Ryu.
pub(crate) fn arcade_hp(strength: u8, phase: u8, blocked: bool) -> Option<u16> {
    let (clean, chip) = match (strength, phase) {
        (1, LOWER) => (30, 8),
        (1, UPPER) => (20, 4),
        (2, LOWER) => (29, 7),
        (2, UPPER) => (12, 3),
        (3, OPENER) => (4, 1),
        (3, LOWER) => (27, 6),
        (3, UPPER) => (11, 2),
        _ => return None,
    };
    Some(if blocked { chip } else { clean })
}

/// Whether a clean hit on a grounded New Ryu leaves him standing.
pub(crate) fn grounds(strength: u8, phase: u8) -> bool {
    matches!((strength, phase), (2, LOWER) | (3, OPENER | LOWER))
}

/// Pointers to New Ryu's two knockdowns, after Ken's throw and air-normal pointers.
pub(super) const POINTER_BYTES: usize = 2 * 4;
const KNOCKDOWNS: u32 = ERA_HEADER_END + (st_ken_throws::POINTER_BYTES + st_ken_air_normals::POINTER_BYTES) as u32;
/// A strike that lands during the previous reaction skips the still tick
/// between the contact freeze and the first rising tick.
pub(crate) const KNOCKDOWN_REHIT: u32 = 0x20;
/// The knockdown's height above the floor through the contact freeze.
pub(crate) const KNOCKDOWN_START: u8 = 16;

pub(super) fn knockdown_offset(heavy: bool) -> u32 {
    KNOCKDOWNS + 4 * u32::from(heavy)
}

// New Ryu's height above the floor on each advancing tick of the knockdown,
// from the first rising tick through the bounce to the final landing. The
// heavy punch launches him lower than the light and medium punches do.
// From docs/research/ken-super-turbo-shoryuken-reactions.tsv.
#[rustfmt::skip]
const KNOCKDOWN: [u8; 66] = [
    21, 27, 32, 37, 41, 46, 50, 53, 57, 60, 63, 66, 68, 70, 72, 73, 74, 75, 76, 76, 77, 76,
    76, 75, 74, 73, 71, 69, 67, 65, 62, 59, 56, 52, 48, 44, 40, 35, 30, 25, 19, 14, 7, 1,
    0, 0, 0, 0, 0, 3, 5, 6, 8, 9, 10, 10, 10, 10, 10, 9, 8, 7, 6, 4, 2, 0,
];

#[rustfmt::skip]
const HEAVY_KNOCKDOWN: [u8; 59] = [
    20, 25, 29, 33, 36, 40, 43, 45, 48, 50, 52, 54, 55, 56, 57, 57, 57, 57, 57, 56, 56, 54,
    53, 51, 49, 47, 44, 41, 38, 35, 31, 27, 23, 18, 13, 8, 3,
    0, 0, 0, 0, 0, 2, 4, 6, 7, 8, 9, 10, 10, 10, 10, 9, 8, 7, 6, 4, 2, 0,
];

/// New Ryu's knockdown from the ground. He drifts back 2 px per tick to the
/// first landing, rests four ticks, hops back at 2.5 px per tick, slides to a
/// stop and is back in neutral 53 ticks after the final landing.
fn build_knockdowns(rom: &[u8], region: &mut Region) {
    const AIR: u32 = 0x1ce8;
    const DOWN: u32 = 0x1d2e;
    const RISE: u32 = 0x1d4e;
    const SLIDE: [i8; 9] = [2, 1, 1, 1, 1, 0, 1, 0, 1];
    const DOWN_TICKS: u8 = 34;
    const RISE_TICKS: u8 = 8;

    let grounded = region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, 0x8a]);
    let step = |frame: u32, hold: u8, dy: i32, dx: i8| {
        let mut record = rom_record(rom, frame);
        record[0] = 0;
        record[1] = hold;
        record[6] = dy as i8 as u8;
        record[7] = (-dx) as u8;
        record
    };
    for (heavy, heights) in [(false, &KNOCKDOWN[..]), (true, &HEAVY_KNOCKDOWN[..])] {
        if region.here() & 1 != 0 {
            region.put(&[0]);
        }
        let origin = region.put(&step(AIR, 0, 0, 0));
        region.put(&step(AIR, 0, 0, 0));
        let first_floor = heights.iter().position(|&height| height == 0).unwrap();
        let (&landing, flight) = heights.split_last().unwrap();
        let drift = |tick: usize| match tick.checked_sub(first_floor + 1) {
            None => 2,
            Some(0..=3) => 0,
            Some(rested) => 3 - (rested % 2) as i8,
        };
        let mut previous = i32::from(KNOCKDOWN_START);
        for (tick, &height) in flight.iter().enumerate() {
            if tick == first_floor {
                region.call(routine::LANDING, 0);
            }
            region.put(&step(AIR, 0, previous - i32::from(height), drift(tick)));
            previous = i32::from(height);
        }
        // The lying pose keeps the standing grid, whose bottom is the floor.
        let settle = previous - i32::from(landing) + rom_pose(rom, AIR).half().1 - rom_pose(rom, DOWN).half().1;
        region.call(grounded, 0);
        region.call(routine::LANDING, 0);
        region.put(&step(DOWN, 0, settle, drift(flight.len())));
        for dx in SLIDE {
            region.put(&step(DOWN, 0, 0, dx));
        }
        region.put(&step(DOWN, DOWN_TICKS - 1, 0, 0));
        region.put(&step(RISE, RISE_TICKS - 1, 0, 0));
        region.end();
        region.patch(region.base + knockdown_offset(heavy), &origin.to_be_bytes());
    }
}

// Cumulative forward travel and height above the floor for each advancing
// tick, from docs/research/ken-super-turbo-shoryuken-flight.tsv. Travel is
// rounded once in cumulative form to the nearest port pixel.
#[rustfmt::skip]
const LP_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0),
    (4, 5), (8, 9), (11, 13), (13, 17), (15, 21), (17, 24), (18, 27), (18, 29),
    (18, 31), (18, 31), (18, 33), (18, 34), (18, 35), (18, 35), (18, 35), (18, 35),
    (18, 35), (18, 34), (18, 32), (18, 30), (18, 28), (18, 26), (18, 23), (18, 20),
    (18, 16), (18, 12), (18, 8), (18, 3), (18, 0), (18, 0), (18, 0), (18, 0),
];

#[rustfmt::skip]
const MP_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0),
    (8, 8), (16, 15), (23, 22), (29, 29), (35, 36), (41, 42), (46, 48), (50, 53),
    (54, 58), (58, 63), (61, 67), (63, 71), (65, 74), (67, 77), (68, 80), (68, 83),
    (68, 85), (68, 85), (68, 86), (68, 87), (68, 88), (68, 89), (68, 89), (68, 89),
    (68, 88), (68, 87), (68, 86), (68, 84), (68, 82), (68, 79), (68, 76), (68, 73),
    (68, 70), (68, 66), (68, 61), (68, 56), (68, 51), (68, 46), (68, 40), (68, 34),
    (68, 27), (68, 20), (68, 13), (68, 5), (68, 0), (68, 0), (68, 0), (68, 0),
];

#[rustfmt::skip]
const HP_TRAJECTORY: &[(u8, u8)] = &[
    (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (11, 10), (22, 19),
    (32, 28), (41, 37), (50, 46), (59, 54), (67, 62), (74, 69), (81, 76), (88, 83),
    (94, 89), (99, 95), (104, 100), (109, 105), (113, 110), (116, 115), (119, 119), (122, 122),
    (124, 125), (125, 128), (126, 131), (127, 133), (127, 135), (127, 135), (127, 136), (127, 137),
    (127, 138), (127, 138), (127, 138), (127, 137), (127, 136), (127, 135), (127, 134), (127, 132),
    (127, 129), (127, 126), (127, 123), (127, 120), (127, 116), (127, 112), (127, 107), (127, 102),
    (127, 97), (127, 91), (127, 85), (127, 78), (127, 71), (127, 64), (127, 57), (127, 49),
    (127, 40), (127, 31), (127, 22), (127, 13), (127, 3), (127, 0), (127, 0), (127, 0),
    (127, 0),
];

struct Profile {
    strength: u8,
    loadout: Loadout,
    trajectory: &'static [(u8, u8)],
    /// The first tick of each strike phase and the tick after the last strike.
    opener: Option<u8>,
    lower: u8,
    upper: u8,
    retract: u8,
}

const PROFILES: [Profile; 4] = [
    Profile { strength: 1, loadout: Loadout::Standard, trajectory: LP_TRAJECTORY, opener: None, lower: 2, upper: 6, retract: 20 },
    Profile { strength: 2, loadout: Loadout::Standard, trajectory: MP_TRAJECTORY, opener: None, lower: 2, upper: 6, retract: 28 },
    Profile { strength: 3, loadout: Loadout::Standard, trajectory: HP_TRAJECTORY, opener: Some(0), lower: 2, upper: 4, retract: 30 },
    Profile { strength: 3, loadout: Loadout::Old, trajectory: HP_TRAJECTORY, opener: Some(1), lower: 2, upper: 4, retract: 30 },
];

impl Profile {
    fn phase(&self, tick: u8) -> u8 {
        if tick >= self.retract {
            0
        } else if tick >= self.upper {
            UPPER
        } else if tick >= self.lower {
            LOWER
        } else if self.opener.is_some_and(|start| tick >= start) {
            OPENER
        } else {
            0
        }
    }
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    build_knockdowns(rom, region);
    let source = TournamentFighter::Ken.move_table() + u32::from(MOVE) * MOVE_HEADER_SIZE;
    let (crouch, crouch_boxes) = header_pose(rom, source);
    let sounds = std::array::from_fn(|i| rom_b(rom, source + 0x10 + i as u32));
    let hits: [u32; 3] = std::array::from_fn(|index| {
        let mut entry: [u8; 9] = std::array::from_fn(|offset| rom_b(rom, 0x45f52 + offset as u32));
        entry[8] = index as u8 + 1;
        region.put(&entry)
    });
    let rising = region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, RISING]);
    let falling = region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, FALLING]);
    let grounded = region.put(&[ROUTINE_ST_SHORYUREPPA_POSTURE, GROUNDED]);

    for profile in &PROFILES {
        let floor = profile.trajectory.iter().rposition(|&(_, height)| height != 0).unwrap() as u8 + 1;
        let apex = profile.trajectory.iter().map(|&(_, height)| height).max().unwrap();
        let first_fall = profile.trajectory.iter().rposition(|&(_, height)| height == apex).unwrap() as u8 + 1;
        // The 1987 record's guard and invulnerability bits stay with its pose.
        let record = |tick: u8| {
            let phase = profile.phase(tick);
            let (pose, boxes, kept_flags) = if tick >= floor || (tick < profile.lower && profile.opener.is_none()) {
                (crouch, crouch_boxes, 0)
            } else {
                let frame = if tick >= profile.retract {
                    RETRACT
                } else if phase == UPPER {
                    EXTENDED
                } else {
                    STRIKE
                };
                (rom_pose(rom, frame), rom_boxes(rom, frame), rom_b(rom, frame + 0x1a) & 0x7f)
            };
            (phase, pose, boxes, kept_flags)
        };
        let strike = |phase: u8, pose: PoseGeom, mut boxes: [u8; 16], kept_flags: u8| {
            if let Some((x, y, rx, ry)) = attack_box(phase, profile.loadout) {
                boxes[12..16].copy_from_slice(&[
                    (i32::from(y) - pose.half().1) as u8,
                    ry as u8,
                    (i32::from(x) - i32::from(pose.x_offset())) as u8,
                    rx as u8,
                ]);
                (boxes, kept_flags | 0x80, hits[usize::from(phase - 1)])
            } else {
                boxes[12..16].fill(0);
                (boxes, kept_flags, 0)
            }
        };

        let stream = region.here();
        let (first_phase, first_pose, first_boxes, first_flags) = record(0);
        let mut pose = first_pose;
        let mut top = pose.top_on_ground();
        // The header is entry tick 0. The last two stationary samples belong
        // to the end record and the return to neutral on the next tick.
        for tick in 1..profile.trajectory.len() - 2 {
            let (x, height) = profile.trajectory[tick];
            let previous_x = profile.trajectory[tick - 1].0;
            let tick = tick as u8;
            if height != 0 && profile.trajectory[usize::from(tick) - 1].1 == 0 {
                region.call(rising, 0);
            } else if tick == first_fall {
                region.call(falling, 0);
            } else if tick == floor {
                region.call(grounded, 0);
                region.call(routine::LANDING, 0);
            }
            let (phase, next_pose, boxes, kept_flags) = record(tick);
            let next_top = next_pose.top_on_ground() - i32::from(height);
            let dy = next_top - (top + pose.half().1 - next_pose.half().1);
            let (boxes, flags, hit) = strike(phase, next_pose, boxes, kept_flags);
            region.put(&Rec {
                hold: 0, dur: 0, pose: next_pose, dy: dy as i8,
                dx: (i16::from(x) - i16::from(previous_x)) as i8,
                boxes, flags, x_offset: next_pose.x_offset(), hit,
            }.bytes());
            pose = next_pose;
            top = next_top;
        }
        region.end();

        let (boxes, flags, hit) = strike(first_phase, first_pose, first_boxes, first_flags);
        let first = Rec {
            hold: 0, dur: 0, pose: first_pose, dy: 0, dx: 0,
            boxes, flags, x_offset: first_pose.x_offset(), hit,
        };
        let h = header(&first.bytes(), first_pose.top_on_ground(), GROUNDED, MOVE, sounds, stream);
        put_era_header(region, EraFamily::Sf2, slot(profile.strength, profile.loadout).unwrap(), &h);
    }
}
