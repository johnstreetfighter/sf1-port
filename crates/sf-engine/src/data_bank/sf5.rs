//! Native SF5 action streams composed from the existing 1987 artwork.
use super::*;
use crate::era::EraFamily;
use crate::era_fight::sf5::{
    COUNTER_STRIKE, CRITICAL_ART, EX_FORWARD, EX_KICK, EX_LOW_SHOT, EX_RISE, REVERSAL, SHIFT,
    SHIFT_BREAK, SKILL_ONE, SKILL_TWO, TRIGGER, TRIGGER_ATTACK, TRIGGER_FORWARD, UPPERCUT,
};
use TournamentFighter::{Birdie, Ken, Ryu, Sagat};

type Pose = (PoseGeom, [u8; 16]);

const KAGE_SENHA: u8 = SKILL_ONE;
pub(crate) const KAGE_ASHURA_FORWARD: u8 = TRIGGER_FORWARD;
pub(crate) const KAGE_ASHURA_BACK: u8 = TRIGGER_ATTACK;
pub(crate) const KAGE_ASHURA_AIR_FORWARD: u8 = COUNTER_STRIKE;
pub(crate) const KAGE_ASHURA_AIR_BACK: u8 = EX_LOW_SHOT;
pub(crate) const KAGE_HADOKEN: u8 = 0x1a;
pub(crate) const KAGE_KUREKIJIN: u8 = 0x1b;
pub(crate) const KAGE_MISOGI: u8 = 0x1d;
pub(crate) const KAGE_SEKIEIKEN: u8 = 0x1e;
pub(crate) const KAGE_SHAKUNETSU: u8 = SF2_SPECIAL_SLOT;
pub(crate) const KAGE_RYUSOKYAKU: u8 = SKILL_TWO;

// Keep the private streams off Ryu's normal SF5 headers. The move IDs
// written into those streams still use the native attack slots.
pub(super) fn kage_header_slot(slot: u8, airborne: bool) -> Option<u8> {
    Some(match slot {
        KAGE_SENHA => 0x30,
        KAGE_ASHURA_FORWARD => 0x31,
        KAGE_ASHURA_BACK => 0x32,
        KAGE_ASHURA_AIR_FORWARD => 0x33,
        KAGE_ASHURA_AIR_BACK => 0x34,
        KAGE_HADOKEN => 0x35,
        EX_FORWARD => 0x36,
        KAGE_SHAKUNETSU => 0x37,
        KAGE_RYUSOKYAKU => 0x38,
        CRITICAL_ART => 0x39,
        KAGE_KUREKIJIN => 0x3a,
        EX_KICK => 0x3b,
        0x19 => 0x3c,
        KAGE_SEKIEIKEN => 0x3d,
        KAGE_MISOGI if airborne => 0x3f,
        KAGE_MISOGI => 0x3e,
        _ => return None,
    })
}

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
    if fighter == Ryu {
        write_kage(rom, region, stance, punch);
    }
}

fn put_kage_header(region: &mut Region, slot: u8, mut move_header: [u8; 0x30]) {
    move_header[0xa] = slot;
    put_era_header(
        region,
        EraFamily::Sf5,
        kage_header_slot(slot, false).expect("Kage move has a private header slot"),
        &move_header,
    );
}

fn bind_kage(region: &mut Region, slot: u8, first: Rec, stream: u32, airborne: bool) {
    put_era_header(
        region,
        EraFamily::Sf5,
        kage_header_slot(slot, airborne).expect("Kage move has a private header slot"),
        &header(
            &first.bytes(),
            first.pose.top_on_ground(),
            if airborne { 0xc1 } else { 0xc2 },
            slot,
            [0; 3],
            stream,
        ),
    );
}

fn write_kage(rom: &[u8], region: &mut Region, stance: Pose, punch: Pose) {
    let gather = frame(rom, 0x1362);
    let glide = frame(rom, 0x0e20);
    write_senha(rom, region, gather, punch, stance);
    write_ashura(region, gather, glide, stance);
    write_sekieiken(rom, region, stance);
    write_misogi(rom, region, gather, stance);
    write_short_hadoken(rom, region, gather, stance);
    write_kurekijin(rom, region, glide, stance);
    put_kage_header(region, 0x19, rom_header(rom, Ryu.move_table() + 0x19 * MOVE_HEADER_SIZE));

    let source = bank_header_offset(SF2_SPECIAL_SLOT) as usize;
    let shaku = region.bytes[source..source + 0x30].try_into().unwrap();
    put_kage_header(region, KAGE_SHAKUNETSU, shaku);
    for (slot, source) in [
        (KAGE_RYUSOKYAKU, crate::era_fight::sf4::RYUSOKYAKU),
        (CRITICAL_ART, 0x2b),
    ] {
        let pointer = era_header_offset(EraFamily::Sf4, source).unwrap() as usize;
        let address = u32::from_be_bytes(region.bytes[pointer..pointer + 4].try_into().unwrap());
        if let Some(source) = address.checked_sub(region.base) {
            let source = source as usize;
            let copied = region.bytes[source..source + 0x30].try_into().unwrap();
            put_kage_header(region, slot, copied);
        }
    }
}

fn write_senha(rom: &[u8], region: &mut Region, gather: Pose, mut punch: Pose, stance: Pose) {
    punch.1[12..16].copy_from_slice(&[4, 27, 31, 28]);
    let (origin, dy) = ground_origin_record(region, gather.0, punch.0);
    let branches = [(6, 3), (8, 5), (7, 6), (9, 9)].map(|(damage, step)| {
        let entry = hit(rom, region, 0x45c88, damage);
        let stream = region.here();
        let mut animation = Stream::new(region, gather.0, gather.0.top_on_ground());
        animation.change_origin(origin, dy);
        animation.call(routine::SOUND, 0xa2);
        animation.strike(punch.0, 3, (0, step), punch.1, 0x80, entry);
        animation.hold(stance.0, stance.0.top_on_ground(), 20, stance.1, 0, 0);
        animation.end();
        stream
    });
    let select = region.put(&[ROUTINE_KAGE_SENHA, 0, 0, 0]);
    for stream in branches {
        region.put_l(stream);
    }
    let stream = region.here();
    let mut windup = Stream::new(region, gather.0, gather.0.top_on_ground());
    windup.travel(gather.0, gather.0.top_on_ground(), 8, (0, 2), gather.1, 0, 0);
    windup.call(select, 0);
    bind_kage(region, KAGE_SENHA, still(gather.0, 8, gather.1), stream, false);
}

fn write_ashura(region: &mut Region, gather: Pose, glide: Pose, stance: Pose) {
    let land = region.put(&[ROUTINE_AIR_THROW_LAND, 0]);
    for (slot, airborne, step) in [
        (KAGE_ASHURA_FORWARD, false, 14),
        (KAGE_ASHURA_BACK, false, -10),
        (KAGE_ASHURA_AIR_FORWARD, true, 14),
        (KAGE_ASHURA_AIR_BACK, true, -10),
    ] {
        let stream = region.here();
        let mut animation = Stream::new(region, gather.0, gather.0.top_on_ground());
        animation.call(routine::SOUND, 0xa2);
        animation.travel(glide.0, glide.0.top_on_ground(), 8, (0, step), glide.1, 0x08, 0);
        if airborne {
            animation.hold(glide.0, glide.0.top_on_ground(), 4, glide.1, 0, 0);
            animation.call(land, 0);
        }
        animation.hold(stance.0, stance.0.top_on_ground(), 13, stance.1, 0, 0);
        animation.end();
        bind_kage(region, slot, still(gather.0, 3, gather.1), stream, airborne);
    }
}

fn write_sekieiken(rom: &[u8], region: &mut Region, stance: Pose) {
    // The forward-jump stream already has Ryu's complete somersault.
    let [rise, tuck, flip, uncurl, windup, mut strike, land] =
        [0x078c, 0x07cc, 0x07ec, 0x080c, 0x166c, 0x168c, 0x0858].map(|at| frame(rom, at));
    strike.1[12..16].copy_from_slice(&[0xf4, 18, 32, 22]);
    let entry = hit(rom, region, 0x45c88, 7);
    let stream = region.here();
    let mut animation = Stream::new(region, land.0, land.0.top_on_ground());
    animation.call(routine::RISING, 0);
    animation.travel(rise.0, rise.0.top_on_ground(), 4, (-7, 2), rise.1, 0, 0);
    animation.travel(tuck.0, tuck.0.top_on_ground() - 28, 4, (-5, 3), tuck.1, 0, 0);
    animation.hold(flip.0, flip.0.top_on_ground() - 48, 5, flip.1, 0, 0);
    animation.travel(uncurl.0, uncurl.0.top_on_ground() - 48, 4, (5, 4), uncurl.1, 0, 0);
    animation.travel(windup.0, windup.0.top_on_ground() - 28, 4, (3, 4), windup.1, 0, 0);
    animation.call(routine::SOUND, 0xa2);
    animation.strike(strike.0, 4, (4, 3), strike.1, 0x80, entry);
    animation.call(routine::TOUCHDOWN, 0);
    animation.hold(land.0, land.0.top_on_ground(), 8, land.1, 0, 0);
    animation.hold(stance.0, stance.0.top_on_ground(), 14, stance.1, 0, 0);
    animation.end();
    bind_kage(region, KAGE_SEKIEIKEN, still(land.0, 6, land.1), stream, false);
}

fn write_misogi(rom: &[u8], region: &mut Region, gather: Pose, stance: Pose) {
    let [rise, chamber, mut strike, mut land] =
        [0x1220, 0x166c, 0x168c, 0x0858].map(|at| frame(rom, at));
    strike.1[12..16].copy_from_slice(&[0xe9, 18, 39, 27]);
    land.1[12..16].copy_from_slice(&[0xf2, 22, 22, 29]);
    let entry = hit(rom, region, 0x45f52, 11);
    let align = region.put(&[ROUTINE_KAGE_MISOGI_ALIGN, 71, 0, 106]);
    // Use the existing impact burst at the strike point even on guard or whiff.
    let mut effect: [u8; 0x20] = std::array::from_fn(|byte| rom_b(rom, 0x186fc + byte as u32));
    effect[3] = 14;
    let effect = region.put(&effect);
    let burst = region.put(&[ROUTINE_PROJECTILE, 0, 0, 0]);
    region.put_l(effect);

    let apex = chamber.0.top_on_ground() - 71;
    let dive = region.here();
    let mut animation = Stream::new(region, chamber.0, apex);
    animation.call(align, 0);
    animation.travel(chamber.0, apex, 6, (5, 0), chamber.1, 0, 0);
    animation.call(routine::SOUND, 0xa2);
    animation.strike(strike.0, 5, (7, 1), strike.1, 0x80, entry);
    animation.call(routine::TOUCHDOWN, 0);
    animation.travel(land.0, land.0.top_on_ground(), 2, (0, 0), land.1, 0x80, entry);
    animation.call(burst, 0);
    animation.hold(land.0, land.0.top_on_ground(), 10, land.1, 0, 0);
    animation.hold(stance.0, stance.0.top_on_ground(), 19, stance.1, 0, 0);
    animation.end();

    let stream = region.here();
    let mut animation = Stream::new(region, gather.0, gather.0.top_on_ground());
    animation.call(routine::RISING, 0);
    animation.travel(rise.0, rise.0.top_on_ground(), 8, (-8, 0), rise.1, 0, 0);
    animation.travel(chamber.0, chamber.0.top_on_ground() - 64, 7, (-1, 0), chamber.1, 0, 0);
    region.put(&[0xfe, 0]);
    region.put_l(dive);
    bind_kage(region, KAGE_MISOGI, still(gather.0, 8, gather.1), stream, false);

    // A confirmed airborne Shoryuken has already lifted both fighters.
    // Keep Kage's starting height and reach the shared dive during the juggle.
    let air_stream = region.here();
    let mut animation = Stream::new(region, chamber.0, chamber.0.top_on_ground());
    animation.travel(chamber.0, chamber.0.top_on_ground(), 9, (-6, 0), chamber.1, 0, 0);
    region.put(&[0xfe, 0]);
    region.put_l(dive);
    bind_kage(region, KAGE_MISOGI, still(chamber.0, 3, chamber.1), air_stream, true);
}

fn write_short_hadoken(rom: &[u8], region: &mut Region, gather: Pose, stance: Pose) {
    const HADOKEN: u32 = 0x186dc;
    let source = rom_l(rom, HADOKEN + 0x10);
    let flight = region.here();
    for at in [source, source + 8] {
        let mut frame: [u8; 8] = std::array::from_fn(|byte| rom_b(rom, at + byte as u32));
        frame[0] = 2;
        frame[3] = 1;
        region.put(&frame);
    }
    region.end();
    let mut effect: [u8; 0x20] = std::array::from_fn(|byte| rom_b(rom, HADOKEN + byte as u32));
    effect[9] = 3;
    effect[0xb] = 1;
    effect[0x10..0x14].copy_from_slice(&flight.to_be_bytes());
    let effect = region.put(&effect);
    let spawn = region.put(&[ROUTINE_PROJECTILE, 7, 0, 0]);
    region.put_l(effect);

    let wind = frame(rom, 0x1382);
    let palm = frame(rom, 0x13a8);
    let stream = region.here();
    let mut animation = Stream::new(region, gather.0, gather.0.top_on_ground());
    animation.hold(wind.0, wind.0.top_on_ground(), 6, wind.1, 0, 0);
    animation.call(spawn, 0);
    animation.hold(palm.0, palm.0.top_on_ground(), 18, palm.1, 0, 0);
    animation.hold(stance.0, stance.0.top_on_ground(), 10, stance.1, 0, 0);
    animation.end();
    let first = still(gather.0, 7, gather.1);
    bind_kage(region, KAGE_HADOKEN, first, stream, false);
    bind_kage(region, EX_FORWARD, first, stream, false);
}

fn write_kurekijin(rom: &[u8], region: &mut Region, chamber: Pose, stance: Pose) {
    let mut kick = strike_pose(rom, Ryu.move_table() + 0x16 * MOVE_HEADER_SIZE);
    kick.1[12..16].copy_from_slice(&[4, 31, 24, 30]);
    let (origin, dy) = ground_origin_record(region, chamber.0, kick.0);
    let entry = hit(rom, region, 0x45c88, 7);
    let knockdown = hit(rom, region, 0x45f52, 8);
    for (slot, startup, step, reaction) in [
        (KAGE_KUREKIJIN, 10, 7, entry),
        (EX_KICK, 6, 9, knockdown),
    ] {
        let stream = region.here();
        let mut animation = Stream::new(region, chamber.0, chamber.0.top_on_ground());
        animation.travel(chamber.0, chamber.0.top_on_ground(), 4, (0, 3), chamber.1, 0, 0);
        animation.change_origin(origin, dy);
        animation.call(routine::SOUND, 0xa2);
        animation.strike(kick.0, 4, (0, step), kick.1, 0x80, reaction);
        animation.hold(chamber.0, chamber.0.top_on_ground(), 6, chamber.1, 0, 0);
        animation.hold(stance.0, stance.0.top_on_ground(), 14, stance.1, 0, 0);
        animation.end();
        bind_kage(region, slot, still(chamber.0, startup, chamber.1), stream, false);
    }
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
