//! SF6 move streams assembled from native fighting poses.

use super::*;
use crate::era::EraFamily::Sf6;
use crate::era_fight::sf6::{IMPACT, PARRY, REVERSAL, RUSH};
use TournamentFighter::{Ken, Retsu, Ryu};

pub(super) fn build(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    if rom.len() < 0x45f5a {
        return;
    }
    let (windup, strike, retract) = match fighter {
        Ryu => (0x0af0, 0x0b30, 0x0b50),
        Ken => (0x2bce, 0x2c0e, 0x2c2e),
        Retsu => (0xc9b6, 0xc9dc, 0xca1c),
        _ => return,
    };
    if fighter == Ryu {
        alias_era_header(region, Sf6, 0x29, 0x29);
        alias_era_header(region, Sf6, 0x2b, 0x2a);
    } else if fighter == Ken {
        alias_era_header(region, Sf6, 0x2a, 0x2b);
        alias_era_header(region, Sf6, 0x2b, 0x29);
    } else {
        alias_era_header(region, Sf6, 0x29, 0x1a);
    }
    for (id, source) in [(0x24, 0x1a), (0x25, 0x19), (0x26, 0x1b)] {
        if fighter == Retsu {
            alias_era_header(region, Sf6, id, source);
        } else {
            let start = fighter.move_table() + u32::from(source) * MOVE_HEADER_SIZE;
            let mut header = std::array::from_fn(|i| rom_b(rom, start + i as u32));
            header[0x0a] = id;
            put_era_header(region, Sf6, id, &header);
        }
    }
    let (pose, boxes) = header_pose(rom, fighter.move_table() + MOVE_HEADER_SIZE);
    let poses = [windup, strike, retract];
    build_what_if_system(rom, region, fighter, poses);
    match fighter {
        Ryu => {
            write_strikes(rom, region, 0x2a, poses, 7, 3, 5, 1);
            write_strikes(rom, region, 0x1f, poses, 9, 1, 5, 0);
            write_strikes(rom, region, 0x22, poses, 7, 2, 4, 1);
            write_install(region, pose, boxes, 0x20, 20);
        }
        Ken => {
            let kick = [0x2f40, 0x2f80, 0x2f60];
            write_strikes(rom, region, 0x29, kick, 6, 4, 4, 4);
            write_strikes(rom, region, 0x1f, kick, 7, 1, 5, 4);
            write_strikes(rom, region, 0x22, kick, 6, 2, 4, 4);
            write_dash(region, pose, boxes, 0x1c, 9, 4);
            for (slot, travel) in [
                (0x21, 2),
                (0x23, 1),
                (crate::era_fight::sf6::JINRAI_MID, 2),
                (0x1d, 3),
            ] {
                write_strikes(rom, region, slot, kick, 5, 1, 4, travel);
            }
        }
        Retsu => {
            write_strikes(rom, region, 0x2a, poses, 6, 4, 4, 2);
            write_strikes(rom, region, 0x2b, poses, 5, 5, 5, 3);
            let kick = [0xc9b6, 0xcd42, 0xca1c];
            write_strikes(rom, region, 0x19, kick, 8, 1, 5, 4);
            write_strikes(rom, region, 0x25, kick, 6, 2, 4, 5);
            write_install(region, pose, boxes, 0x20, 20);
            write_install(region, pose, boxes, 0x26, 11);
        }
        _ => {}
    }
}

pub(super) fn build_what_if_system(
    rom: &[u8],
    region: &mut Region,
    fighter: TournamentFighter,
    poses: [u32; 3],
) {
    let (pose, boxes) = header_pose(rom, fighter.move_table() + MOVE_HEADER_SIZE);
    let parry = still(pose, 8, boxes);
    let stream = region.end();
    let h = header(
        &parry.bytes(),
        pose.top_on_ground(),
        0xc2,
        PARRY,
        [0; 3],
        stream,
    );
    put_era_header(region, Sf6, PARRY, &h);

    write_dash(region, pose, boxes, RUSH, 12, 5);
    write_strikes(rom, region, IMPACT, poses, 13, 1, 9, 4);
    write_strikes(rom, region, REVERSAL, poses, 6, 1, 4, 2);
}

fn write_dash(
    region: &mut Region,
    pose: PoseGeom,
    boxes: [u8; 16],
    slot: u8,
    frames: u8,
    speed: i32,
) {
    let first = still(pose, 2, boxes);
    let stream = region.here();
    let mut s = Stream::new(region, pose, pose.top_on_ground());
    s.travel(pose, pose.top_on_ground(), frames, (0, speed), boxes, 0, 0);
    s.end();
    let h = header(
        &first.bytes(),
        pose.top_on_ground(),
        0xc2,
        slot,
        [0; 3],
        stream,
    );
    put_era_header(region, Sf6, slot, &h);
}

fn write_install(region: &mut Region, pose: PoseGeom, boxes: [u8; 16], slot: u8, hold: u8) {
    let first = still(pose, hold, boxes);
    let stream = region.end();
    let h = header(
        &first.bytes(),
        pose.top_on_ground(),
        0xc2,
        slot,
        [0; 3],
        stream,
    );
    put_era_header(region, Sf6, slot, &h);
}

fn write_strikes(
    rom: &[u8],
    region: &mut Region,
    id: u8,
    frames: [u32; 3],
    startup: u8,
    hits: u8,
    damage: u8,
    travel: i32,
) {
    let [windup, strike, retract] = frames.map(|at| rom_pose(rom, at));
    let [windup_boxes, mut strike_boxes, retract_boxes] = frames.map(|at| rom_boxes(rom, at));
    strike_boxes[12..16].copy_from_slice(&[2, 26, 24, 32]);
    let mut entry = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, 0x45c88 + i as u32));
    entry[1] = damage;
    let hit = region.put(&entry);
    let mut finisher = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, 0x45f52 + i as u32));
    finisher[1] = damage;
    let finisher = region.put(&finisher);
    let (origin, shift) = ground_origin_record(region, windup, strike);
    let (origin_back, shift_back) = ground_origin_record(region, strike, windup);

    let stream = region.here();
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    s.travel(
        windup,
        windup.top_on_ground(),
        4,
        (0, travel),
        windup_boxes,
        0,
        0,
    );
    for i in 0..hits {
        s.change_origin(origin, shift);
        s.call(routine::SOUND, 0xa2);
        s.strike(
            strike,
            3,
            (0, travel),
            strike_boxes,
            0x80,
            if i + 1 == hits && matches!(id, IMPACT | REVERSAL | 0x2b) {
                finisher
            } else {
                hit
            },
        );
        s.change_origin(origin_back, shift_back);
        s.hold(windup, windup.top_on_ground(), 2, windup_boxes, 0, 0);
    }
    s.hold(retract, retract.top_on_ground(), 13, retract_boxes, 0, 0);
    s.end();
    let first = still(windup, startup, windup_boxes);
    let h = header(
        &first.bytes(),
        windup.top_on_ground(),
        0xc2,
        id,
        [0; 3],
        stream,
    );
    put_era_header(region, Sf6, id, &h);
}
