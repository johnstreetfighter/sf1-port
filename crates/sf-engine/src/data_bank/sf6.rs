//! SF6 move streams assembled from native fighting poses.

use super::*;
use crate::era::EraFamily::Sf6;
use crate::era_fight::sf6::{
    ASHURA_BACK, ASHURA_FORWARD, IMPACT, OD_RYUSOKYAKU, PARRY, REVERSAL, RUSH, RYUSOKYAKU,
};
use TournamentFighter::{Ken, Retsu, Ryu};

const SHAKUNETSU_HEADER: u8 = 0x30;
const OD_SHAKUNETSU_HEADER: u8 = 0x31;
const ASHURA_FORWARD_HEADER: u8 = 0x32;
const ASHURA_BACK_HEADER: u8 = 0x33;
const RYUSOKYAKU_HEADER: u8 = 0x34;
const OD_RYUSOKYAKU_HEADER: u8 = 0x35;

pub(super) fn evil_ryu_header_slot(slot: u8) -> Option<u8> {
    match slot {
        SF2_SPECIAL_SLOT => Some(SHAKUNETSU_HEADER),
        0x22 => Some(OD_SHAKUNETSU_HEADER),
        ASHURA_FORWARD => Some(ASHURA_FORWARD_HEADER),
        ASHURA_BACK => Some(ASHURA_BACK_HEADER),
        RYUSOKYAKU => Some(RYUSOKYAKU_HEADER),
        OD_RYUSOKYAKU => Some(OD_RYUSOKYAKU_HEADER),
        _ => None,
    }
}

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
            write_evil_ryu_shakunetsu(region);
            write_evil_ryu_ashura(rom, region, pose, boxes);
            write_evil_ryu_ryusokyaku(region);
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

fn write_evil_ryu_shakunetsu(region: &mut Region) {
    // Use the existing red fireball stream in two private header slots.
    let source = bank_header_offset(SF2_SPECIAL_SLOT) as usize;
    let normal: [u8; 0x30] = region.bytes[source..source + 0x30].try_into().unwrap();
    let mut overdrive = normal;
    overdrive[0xa] = 0x22;
    overdrive[0xe] = 1;
    put_era_header(region, Sf6, SHAKUNETSU_HEADER, &normal);
    put_era_header(region, Sf6, OD_SHAKUNETSU_HEADER, &overdrive);
}

fn write_evil_ryu_ashura(rom: &[u8], region: &mut Region, stance: PoseGeom, stance_boxes: [u8; 16]) {
    // Use the same gather and raised-knee poses as Kage's grounded Ashura.
    let gather = rom_pose(rom, 0x1362);
    let gather_boxes = rom_boxes(rom, 0x1362);
    let glide = rom_pose(rom, 0x0e20);
    let glide_boxes = rom_boxes(rom, 0x0e20);
    for (slot, startup, travel, step, recovery) in [
        (ASHURA_FORWARD, 17, 14, 10, 17),
        (ASHURA_BACK, 13, 12, -8, 22),
    ] {
        let stream = region.here();
        let mut animation = Stream::new(region, gather, gather.top_on_ground());
        animation.call(routine::SOUND, 0xa2);
        animation.travel(glide, glide.top_on_ground(), travel, (0, step), glide_boxes, SF6_ASHURA_GLIDE, 0);
        animation.hold(stance, stance.top_on_ground(), recovery, stance_boxes, 0, 0);
        animation.end();

        let first = still(gather, startup, gather_boxes);
        let header = header(&first.bytes(), gather.top_on_ground(), 0xc2, slot, [0; 3], stream);
        put_era_header(region, Sf6, evil_ryu_header_slot(slot).unwrap(), &header);
    }
}

fn write_evil_ryu_ryusokyaku(region: &mut Region) {
    // The SF4 bank already supplies the strength-specific axe kick streams.
    let pointer = era_header_offset(EraFamily::Sf4, crate::era_fight::sf4::RYUSOKYAKU).unwrap() as usize;
    let address = u32::from_be_bytes(region.bytes[pointer..pointer + 4].try_into().unwrap());
    let Some(source) = address.checked_sub(region.base) else { return };
    let source = source as usize;
    let mut header: [u8; 0x30] = region.bytes[source..source + 0x30].try_into().unwrap();
    for (slot, key) in [(RYUSOKYAKU, RYUSOKYAKU_HEADER), (OD_RYUSOKYAKU, OD_RYUSOKYAKU_HEADER)] {
        header[0xa] = slot;
        put_era_header(region, Sf6, key, &header);
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
