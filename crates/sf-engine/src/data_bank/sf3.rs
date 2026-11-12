//! 1987 pose streams for the Street Fighter III system moves.

use super::*;
use crate::era_fight::sf3::{
    AIR_TATSU, ART_1, ART_2, ART_3, EX_AIR_TATSU, EX_HADOKEN, EX_JOUDAN, EX_SHORYUKEN,
    EX_TATSUMAKI, JOUDAN, OVERHEAD, PARRY,
};

const ACTIVE: u8 = 0x80;
const INVULNERABLE: u8 = 0x08;
const KEN_POSE_DELTA: u32 = 0x20de;
const RYU_STRAIGHT: u32 = 0x0b30;
const RYU_UPPERCUT: u32 = 0x1200;
const RYU_UPPERCUT_RISE: u32 = 0x1220;
const RYU_UPPERCUT_FALL: u32 = 0x12c0;
const RYU_HADOKEN_GATHER: u32 = 0x1362;
const RYU_HADOKEN_THRUST: u32 = 0x1382;
const RYU_HADOKEN_RELEASE: u32 = 0x13a8;
const RYU_HADOKEN_RECOVER: u32 = 0x13c8;
const RYU_TATSU_CHAMBER: u32 = 0x13ea;
const RYU_TATSU_STRIKE: u32 = 0x14b0;
const RYU_TATSU_TURN: u32 = 0x14d6;
const RYU_TATSU_FALL: u32 = 0x14f6;
const HIT_MEDIUM: u32 = 0x45c88;
const HIT_LAUNCH: u32 = 0x45f52;

pub(super) fn build(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    let delta = match fighter {
        TournamentFighter::Ryu => 0,
        TournamentFighter::Ken => KEN_POSE_DELTA,
        _ => return,
    };
    if rom.len() < HIT_LAUNCH as usize + 8 {
        return;
    }

    alias_era_header(region, EraFamily::Sf3, ART_1, SF2_SUPER_SLOT);
    if fighter == TournamentFighter::Ken {
        alias_era_header(region, EraFamily::Sf3, ART_2, ALPHA_SUPER_SECOND_SLOT);
        alias_era_header(region, EraFamily::Sf3, ART_3, ALPHA_SUPER_THIRD_SLOT);
    } else {
        write_uppercut(rom, region, fighter, delta, ART_2);
        write_projectile(rom, region, fighter, delta, ART_3);
        write_blade_kick(rom, region, fighter, JOUDAN);
        write_blade_kick(rom, region, fighter, EX_JOUDAN);
    }

    write_projectile(rom, region, fighter, delta, EX_HADOKEN);
    write_uppercut(rom, region, fighter, delta, EX_SHORYUKEN);
    write_ex_tatsumaki(rom, region, fighter, delta);
    write_air_tatsumaki(rom, region, fighter, delta, AIR_TATSU);
    write_air_tatsumaki(rom, region, fighter, delta, EX_AIR_TATSU);
    write_parry(rom, region, fighter);
    write_overhead(rom, region, fighter, delta);
}

pub(super) fn build_what_if_system(rom: &[u8], region: &mut Region) {
    let ready = header_pose(&region.bytes, bank_header_offset(1));
    let (guard, guard_boxes) = header_pose(&region.bytes, bank_header_offset(4));
    let guard_stream = region.end();
    bind(
        region,
        PARRY,
        still(guard, 1, guard_boxes),
        0xc2,
        [0; 3],
        guard_stream,
    );

    let counter_stream = rom_l(&region.bytes, bank_header_offset(ALPHA_COUNTER_SLOT) + 0x2c);
    let strike = match counter_stream.checked_sub(region.base) {
        Some(relative) if (relative as usize + 0x20) <= region.bytes.len() => (
            rom_pose(&region.bytes, relative),
            rom_boxes(&region.bytes, relative),
        ),
        _ => header_pose(&region.bytes, bank_header_offset(0x10)),
    };
    author_overhead(rom, region, ready, strike);
}

fn hit_entry(rom: &[u8], region: &mut Region, source: u32, damage: u8) -> u32 {
    let mut entry: [u8; 8] = std::array::from_fn(|offset| rom_b(rom, source + offset as u32));
    entry[1] = damage;
    region.put(&entry)
}

fn bind(region: &mut Region, id: u8, first: Rec, state: u8, sounds: [u8; 3], stream: u32) {
    let h = header(
        &first.bytes(),
        first.pose.top_on_ground(),
        state,
        id,
        sounds,
        stream,
    );
    put_era_header(region, EraFamily::Sf3, id, &h);
}

fn stand(rom: &[u8], fighter: TournamentFighter) -> (PoseGeom, [u8; 16]) {
    header_pose(rom, fighter.move_table() + MOVE_HEADER_SIZE)
}

fn guard(rom: &[u8], fighter: TournamentFighter) -> (PoseGeom, [u8; 16]) {
    header_pose(rom, fighter.move_table() + 4 * MOVE_HEADER_SIZE)
}

fn write_parry(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    let (pose, boxes) = guard(rom, fighter);
    let stream = region.end();
    let first = still(pose, 1, boxes);
    bind(region, PARRY, first, 0xc2, [0; 3], stream);
}

fn write_overhead(rom: &[u8], region: &mut Region, fighter: TournamentFighter, delta: u32) {
    let frame = RYU_STRAIGHT + delta;
    author_overhead(
        rom,
        region,
        stand(rom, fighter),
        (rom_pose(rom, frame), rom_boxes(rom, frame)),
    );
}

fn author_overhead(
    rom: &[u8],
    region: &mut Region,
    (ready, ready_boxes): (PoseGeom, [u8; 16]),
    (strike, mut boxes): (PoseGeom, [u8; 16]),
) {
    // The low hop brings this punch down to the crouching body box.
    boxes[12..16].copy_from_slice(&[0xf6, 16, 24, 20]);
    let hit = hit_entry(rom, region, HIT_MEDIUM, 4);
    let (strike_origin, strike_offset) = ground_origin_record(region, ready, strike);
    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    s.call(routine::RISING, 0);
    s.travel(ready, ready.top_on_ground(), 4, (-4, 3), ready_boxes, 0, 0);
    s.change_origin(strike_origin, strike_offset);
    s.call(routine::FALLING, 0);
    s.strike(strike, 4, (2, 2), boxes, ACTIVE, hit);
    s.travel(
        ready,
        ready.top_on_ground() - 8,
        2,
        (4, 1),
        ready_boxes,
        0,
        0,
    );
    s.call(routine::TOUCHDOWN, 0);
    s.hold(ready, ready.top_on_ground(), 10, ready_boxes, 0, 0);
    s.end();
    bind(
        region,
        OVERHEAD,
        still(ready, 5, ready_boxes),
        0xc2,
        [5, 0, 0],
        stream,
    );
}

fn write_projectile(
    rom: &[u8],
    region: &mut Region,
    fighter: TournamentFighter,
    delta: u32,
    id: u8,
) {
    let charge = if id == ART_3 {
        let record = region.put(&[ROUTINE_ALPHA_HOLD_P, 36, 0, 0]);
        region.put_l(0);
        Some(record)
    } else {
        None
    };
    let mut gather = rom_record(rom, RYU_HADOKEN_GATHER + delta);
    gather[1] = 0;
    let stream = region.put(&gather);
    if let Some(charge) = charge {
        region.patch(charge + 4, &stream.to_be_bytes());
        region.call(charge, 0);
    }
    region.put(&rom_record(rom, RYU_HADOKEN_THRUST + delta));
    region.call(0x4548c, 0);
    let mut thrust = rom_record(rom, RYU_HADOKEN_RELEASE + delta);
    thrust[1] = 17;
    region.put(&thrust);
    region.put(&rom_record(rom, RYU_HADOKEN_RECOVER + delta));
    region.end();

    let source = fighter.move_table() + 0x1a * MOVE_HEADER_SIZE;
    let mut h = std::array::from_fn(|index| rom_b(rom, source + index as u32));
    h[0x0a] = id;
    h[0x0e] = 1;
    h[0x2c..0x30].copy_from_slice(&stream.to_be_bytes());
    put_era_header(region, EraFamily::Sf3, id, &h);
}

fn write_uppercut(rom: &[u8], region: &mut Region, fighter: TournamentFighter, delta: u32, id: u8) {
    let super_art = id == ART_2;
    let source = fighter.move_table() + 0x19 * MOVE_HEADER_SIZE;
    let (ready, ready_boxes) = header_pose(rom, source);
    let opener = rom_pose(rom, RYU_STRAIGHT + delta);
    let launch = rom_pose(rom, RYU_UPPERCUT + delta);
    let rise = rom_pose(rom, RYU_UPPERCUT_RISE + delta);
    let fall = rom_pose(rom, RYU_UPPERCUT_FALL + delta);
    let fall_boxes = rom_boxes(rom, RYU_UPPERCUT_FALL + delta);
    let mut opener_boxes = rom_boxes(rom, RYU_STRAIGHT + delta);
    opener_boxes[12..16].copy_from_slice(&[7, 17, 24, 18]);
    let mut launch_boxes = rom_boxes(rom, RYU_UPPERCUT + delta);
    launch_boxes[12..16].copy_from_slice(&[9, 23, 21, 26]);
    let mut rise_boxes = rom_boxes(rom, RYU_UPPERCUT_RISE + delta);
    rise_boxes[12..16].copy_from_slice(&[0, 30, 21, 24]);

    let opening_hit = hit_entry(rom, region, HIT_MEDIUM, if super_art { 5 } else { 3 });
    let finish = hit_entry(rom, region, HIT_LAUNCH, if super_art { 7 } else { 4 });
    let before_launch = if super_art { opener } else { ready };
    let (launch_origin, launch_offset) = ground_origin_record(region, before_launch, launch);
    let (rise_origin, rise_offset) = ground_origin_record(region, launch, rise);
    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    if super_art {
        s.travel(ready, ready.top_on_ground(), 4, (0, 4), ready_boxes, 0, 0);
        s.hold(
            opener,
            opener.top_on_ground(),
            5,
            opener_boxes,
            ACTIVE,
            opening_hit,
        );
        s.hold(
            opener,
            opener.top_on_ground(),
            4,
            opener_boxes,
            ACTIVE,
            opening_hit,
        );
    }
    s.change_origin(launch_origin, launch_offset);
    s.call(routine::RISING, 0);
    s.strike(
        launch,
        4,
        (-4, 3),
        launch_boxes,
        ACTIVE | INVULNERABLE,
        opening_hit,
    );
    s.change_origin(rise_origin, rise_offset);
    s.strike(
        rise,
        4,
        (-4, 1),
        rise_boxes,
        ACTIVE,
        if super_art { finish } else { opening_hit },
    );
    if !super_art {
        s.strike(rise, 4, (-4, 1), rise_boxes, ACTIVE, finish);
    }
    let lift = rise.top_on_ground() - s.top;
    s.call(routine::FALLING, 0);
    s.travel(
        fall,
        fall.top_on_ground() - lift,
        (lift / 4) as u8,
        (4, 0),
        fall_boxes,
        0,
        0,
    );
    s.call(routine::TOUCHDOWN, 0);
    s.hold(ready, ready.top_on_ground(), 3, ready_boxes, 0, 0);
    s.call(routine::LANDING, 0);
    s.hold(ready, ready.top_on_ground(), 13, ready_boxes, 0, 0);
    s.end();
    let mut first = still(ready, 2, ready_boxes);
    first.flags = INVULNERABLE;
    bind(region, id, first, 0xc4, [0xa2, 0, 0x19], stream);
}

fn write_ex_tatsumaki(rom: &[u8], region: &mut Region, fighter: TournamentFighter, delta: u32) {
    let (ready, ready_boxes) = stand(rom, fighter);
    let chamber = rom_pose(rom, RYU_TATSU_CHAMBER + delta);
    let spin = rom_pose(rom, RYU_TATSU_STRIKE + delta);
    let turn = rom_pose(rom, RYU_TATSU_TURN + delta);
    let fall = rom_pose(rom, RYU_TATSU_FALL + delta);
    let mut spin_boxes = rom_boxes(rom, RYU_TATSU_STRIKE + delta);
    spin_boxes[12..16].copy_from_slice(&[0xee, 18, 18, 28]);
    let hit = hit_entry(rom, region, HIT_MEDIUM, 4);
    let finish = hit_entry(rom, region, HIT_LAUNCH, 4);
    let (spin_origin, spin_offset) = ground_origin_record(region, chamber, spin);
    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    s.call(routine::RISING, 0);
    s.travel(
        chamber,
        chamber.top_on_ground(),
        4,
        (-4, 3),
        rom_boxes(rom, RYU_TATSU_CHAMBER + delta),
        0,
        0,
    );
    s.change_origin(spin_origin, spin_offset);
    for index in 0..3 {
        let entry = if index == 2 { finish } else { hit };
        s.strike(spin, 4, (0, 2), spin_boxes, ACTIVE, entry);
        if index != 2 {
            s.hold(
                turn,
                turn.top_on_ground() - 16,
                2,
                rom_boxes(rom, RYU_TATSU_TURN + delta),
                0,
                0,
            );
        }
    }
    s.call(routine::FALLING, 0);
    s.travel(
        fall,
        fall.top_on_ground() - 16,
        4,
        (4, 1),
        rom_boxes(rom, RYU_TATSU_FALL + delta),
        0,
        0,
    );
    s.call(routine::TOUCHDOWN, 0);
    s.hold(ready, ready.top_on_ground(), 3, ready_boxes, 0, 0);
    s.call(routine::LANDING, 0);
    s.hold(ready, ready.top_on_ground(), 10, ready_boxes, 0, 0);
    s.end();
    bind(
        region,
        EX_TATSUMAKI,
        still(ready, 2, ready_boxes),
        0xc2,
        [0x8b, 0, 0x19],
        stream,
    );
}

fn write_air_tatsumaki(
    rom: &[u8],
    region: &mut Region,
    fighter: TournamentFighter,
    delta: u32,
    id: u8,
) {
    let ex = id == EX_AIR_TATSU;
    let height = 44;
    let (ready, ready_boxes) = stand(rom, fighter);
    let chamber = rom_pose(rom, RYU_TATSU_CHAMBER + delta);
    let spin = rom_pose(rom, RYU_TATSU_STRIKE + delta);
    let turn = rom_pose(rom, RYU_TATSU_TURN + delta);
    let fall = rom_pose(rom, RYU_TATSU_FALL + delta);
    let mut spin_boxes = rom_boxes(rom, RYU_TATSU_STRIKE + delta);
    spin_boxes[12..16].copy_from_slice(&[0xec, 17, 20, 29]);
    let hit = hit_entry(rom, region, HIT_MEDIUM, if ex { 4 } else { 5 });
    let (spin_origin, spin_offset) = ground_origin_record(region, chamber, spin);
    let stream = region.here();
    let mut s = Stream::new(region, chamber, chamber.top_on_ground() - height);
    s.change_origin(spin_origin, spin_offset);
    for _ in 0..if ex { 3 } else { 1 } {
        s.strike(spin, 3, (0, 3), spin_boxes, ACTIVE, hit);
        s.hold(
            turn,
            turn.top_on_ground() - height,
            2,
            rom_boxes(rom, RYU_TATSU_TURN + delta),
            0,
            0,
        );
    }
    s.call(routine::FALLING, 0);
    s.travel(
        fall,
        fall.top_on_ground() - height,
        11,
        (4, 2),
        rom_boxes(rom, RYU_TATSU_FALL + delta),
        0,
        0,
    );
    s.call(routine::TOUCHDOWN, 0);
    s.hold(ready, ready.top_on_ground(), 3, ready_boxes, 0, 0);
    s.call(routine::LANDING, 0);
    s.hold(ready, ready.top_on_ground(), 8, ready_boxes, 0, 0);
    s.end();

    let first = still(
        chamber,
        if ex { 2 } else { 4 },
        rom_boxes(rom, RYU_TATSU_CHAMBER + delta),
    );
    let h = header(
        &first.bytes(),
        chamber.top_on_ground() - height,
        0xc1,
        id,
        [0x8b, 0, 0x19],
        stream,
    );
    put_era_header(region, EraFamily::Sf3, id, &h);
    if !ex {
        put_era_header(region, EraFamily::Sf2, id, &h);
    }
}

fn write_blade_kick(rom: &[u8], region: &mut Region, fighter: TournamentFighter, id: u8) {
    let ex = id == EX_JOUDAN;
    let (ready, ready_boxes) = stand(rom, fighter);
    let chamber = rom_pose(rom, 0x0fcc);
    let kick = rom_pose(rom, 0x1012);
    let mut kick_boxes = rom_boxes(rom, 0x1012);
    kick_boxes[12..16].copy_from_slice(&[10, 14, 35, 18]);
    let hit = hit_entry(rom, region, HIT_LAUNCH, if ex { 10 } else { 7 });
    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    s.travel(
        chamber,
        chamber.top_on_ground(),
        if ex { 3 } else { 5 },
        (0, 4),
        rom_boxes(rom, 0x0fcc),
        0,
        0,
    );
    s.hold(kick, kick.top_on_ground(), 4, kick_boxes, ACTIVE, hit);
    s.hold(
        chamber,
        chamber.top_on_ground(),
        6,
        rom_boxes(rom, 0x0fcc),
        0,
        0,
    );
    s.hold(ready, ready.top_on_ground(), 14, ready_boxes, 0, 0);
    s.end();
    bind(
        region,
        id,
        still(ready, if ex { 3 } else { 6 }, ready_boxes),
        0xc2,
        [5, 0, 0],
        stream,
    );
}
