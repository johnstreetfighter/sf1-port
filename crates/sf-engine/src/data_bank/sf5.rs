//! Native SF5 action streams composed from the existing 1987 artwork.
use super::*;
use crate::era::EraFamily;
use crate::era_fight::sf5::{
    COUNTER_STRIKE, CRITICAL_ART, EX_FORWARD, EX_KICK, EX_LOW_SHOT, EX_RISE, REVERSAL, SHIFT,
    SHIFT_BREAK, SKILL_ONE, SKILL_TWO, TRIGGER, TRIGGER_ATTACK, TRIGGER_FORWARD, UPPERCUT,
};
use TournamentFighter::{Birdie, Ken, Ryu, Sagat};

type Pose = (PoseGeom, [u8; 16]);

pub(super) fn build(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    if !matches!(fighter, Ryu | Ken | Birdie | Sagat) || rom.len() < 0x48000 {
        return;
    }
    let table = fighter.move_table();
    let stance = header_pose(rom, table + MOVE_HEADER_SIZE);
    let (chamber, punch, kick) = match fighter {
        Birdie => (
            frame(rom, 0x1151c),
            frame(rom, 0x11562),
            frame(rom, 0x114b2),
        ),
        Sagat => (
            frame(rom, 0x144da),
            frame(rom, 0x13f36),
            frame(rom, 0x13fc6),
        ),
        _ => (
            header_pose(rom, table + 5 * MOVE_HEADER_SIZE),
            strike_pose(rom, table + 0x10 * MOVE_HEADER_SIZE),
            strike_pose(rom, table + 0x16 * MOVE_HEADER_SIZE),
        ),
    };
    let standard_hit = hit(rom, region, 0x45c88, 5);
    let knockdown = hit(rom, region, 0x45f52, 6);

    write_wait(region, TRIGGER, chamber, stance, 5, 0);
    let backstep = if fighter.is_shoto() {
        header_pose(rom, table + 3 * MOVE_HEADER_SIZE)
    } else {
        chamber
    };
    write_wait(region, SHIFT, backstep, stance, 8, -5);
    write_strike(region, REVERSAL, chamber, punch, stance, 2, knockdown);
    write_strike(region, SHIFT_BREAK, chamber, kick, stance, 5, knockdown);

    match fighter {
        Ryu => {
            write_wait(region, SKILL_ONE, chamber, stance, 12, 0);
            write_strike(region, SKILL_TWO, chamber, punch, stance, 5, standard_hit);
            write_wait(region, TRIGGER_ATTACK, chamber, stance, 12, 0);
            write_strike(region, COUNTER_STRIKE, chamber, punch, stance, 3, knockdown);
        }
        Ken => {
            write_strike(region, SKILL_ONE, stance, kick, stance, 6, standard_hit);
            write_strike(region, SKILL_TWO, chamber, kick, stance, 3, knockdown);
            alias(region, TRIGGER_ATTACK, ALPHA_SUPER_SECOND_SLOT);
        }
        Birdie => {
            write_wait(region, SKILL_ONE, chamber, stance, 23, 0);
            write_strike(region, SKILL_TWO, chamber, punch, stance, 1, standard_hit);
            write_strike(region, TRIGGER_ATTACK, chamber, punch, stance, 4, knockdown);
            alias(region, TRIGGER_FORWARD, 0x2c);
        }
        Sagat => {
            write_wait(region, SKILL_ONE, chamber, stance, 20, 0);
            write_wait(region, SKILL_TWO, chamber, stance, 20, 0);
            write_strike(region, TRIGGER_ATTACK, chamber, kick, stance, 3, knockdown);
            alias(region, TRIGGER_FORWARD, ALPHA_SUPER_THIRD_SLOT);
            alias(region, COUNTER_STRIKE, ALPHA_SUPER_SECOND_SLOT);
        }
        _ => unreachable!(),
    }

    if fighter.is_shoto() {
        for (slot, base) in [(EX_FORWARD, 0x1a), (EX_RISE, 0x19), (EX_KICK, 0x1b)] {
            let mut move_header = rom_header(rom, table + u32::from(base as u8) * MOVE_HEADER_SIZE);
            move_header[0xa] = slot;
            put_era_header(region, EraFamily::Sf5, slot, &move_header);
        }
    } else {
        alias(region, EX_FORWARD, 0x1a);
        if fighter == Birdie {
            write_strike(region, EX_KICK, chamber, punch, stance, 2, knockdown);
        } else {
            alias(region, UPPERCUT, SF2_SPECIAL_SLOT);
            alias(region, EX_RISE, SF2_SPECIAL_SLOT);
            alias(region, EX_KICK, 0x19);
            alias(region, EX_LOW_SHOT, 0x1b);
        }
    }

    let critical = if fighter == Ken {
        ALPHA_SUPER_THIRD_SLOT
    } else {
        SF2_SUPER_SLOT
    };
    alias(region, CRITICAL_ART, critical);
}

fn alias(region: &mut Region, slot: u8, source: u8) {
    alias_era_header(region, EraFamily::Sf5, slot, source);
}

fn frame(rom: &[u8], address: u32) -> Pose {
    (rom_pose(rom, address), rom_boxes(rom, address))
}

fn rom_header(rom: &[u8], address: u32) -> [u8; 0x30] {
    std::array::from_fn(|byte| rom_b(rom, address + byte as u32))
}

fn strike_pose(rom: &[u8], address: u32) -> Pose {
    let mut stream = rom_l(rom, address + 0x2c);
    for _ in 0..12 {
        match rom_b(rom, stream) {
            0xff => stream += 6,
            0xfd | 0xfe => break,
            _ => {
                if rom_b(rom, stream + 0x1a) & 0x80 != 0 {
                    return frame(rom, stream);
                }
                stream += 0x20;
            }
        }
    }
    header_pose(rom, address)
}

fn hit(rom: &[u8], region: &mut Region, reaction: u32, damage: u8) -> u32 {
    let mut entry: [u8; 8] = std::array::from_fn(|byte| rom_b(rom, reaction + byte as u32));
    entry[1] = damage;
    region.put(&entry)
}

fn bind(region: &mut Region, slot: u8, first: Rec, stream: u32) {
    put_era_header(
        region,
        EraFamily::Sf5,
        slot,
        &header(
            &first.bytes(),
            first.pose.top_on_ground(),
            0xc2,
            slot,
            [0; 3],
            stream,
        ),
    );
}

fn write_wait(region: &mut Region, slot: u8, chamber: Pose, stance: Pose, frames: u8, step: i32) {
    let stream = region.here();
    let mut animation = Stream::new(region, chamber.0, chamber.0.top_on_ground());
    animation.travel(
        chamber.0,
        chamber.0.top_on_ground(),
        frames,
        (0, step),
        chamber.1,
        0,
        0,
    );
    animation.hold(stance.0, stance.0.top_on_ground(), 7, stance.1, 0, 0);
    animation.end();
    bind(region, slot, still(chamber.0, 2, chamber.1), stream);
}

fn write_strike(
    region: &mut Region,
    slot: u8,
    chamber: Pose,
    mut attack: Pose,
    stance: Pose,
    step: i32,
    entry: u32,
) {
    attack.1[12..16].copy_from_slice(&[3, 25, 31, 28]);
    let (origin, dy) = ground_origin_record(region, chamber.0, attack.0);
    let stream = region.here();
    let mut animation = Stream::new(region, chamber.0, chamber.0.top_on_ground());
    animation.travel(
        chamber.0,
        chamber.0.top_on_ground(),
        4,
        (0, step),
        chamber.1,
        0,
        0,
    );
    animation.change_origin(origin, dy);
    animation.call(routine::SOUND, 0xa2);
    animation.strike(attack.0, 5, (0, step), attack.1, 0x80, entry);
    animation.hold(stance.0, stance.0.top_on_ground(), 15, stance.1, 0, 0);
    animation.end();
    bind(region, slot, still(chamber.0, 4, chamber.1), stream);
}

pub(super) fn build_what_if_system(
    rom: &[u8],
    region: &mut Region,
    fighter: TournamentFighter,
    windup: u32,
    strike: u32,
    kick: u32,
) {
    let stance = header_pose(rom, fighter.move_table() + MOVE_HEADER_SIZE);
    let chamber = frame(rom, windup);
    let punch = frame(rom, strike);
    let kick = frame(rom, kick);
    let standard_hit = hit(rom, region, 0x45c88, 5);
    let knockdown = hit(rom, region, 0x45f52, 6);

    write_wait(region, TRIGGER, chamber, stance, 5, 0);
    write_wait(region, SHIFT, chamber, stance, 8, -5);
    write_strike(region, REVERSAL, chamber, punch, stance, 2, knockdown);
    write_strike(region, SHIFT_BREAK, chamber, kick, stance, 5, knockdown);
    write_strike(region, SKILL_ONE, chamber, punch, stance, 3, standard_hit);
    write_strike(region, SKILL_TWO, chamber, kick, stance, 2, knockdown);
}
