//! Alpha moves built from Sagat and Adon's 1987 poses and hit reactions.

use super::*;

const JAGUAR_TOOTH: u8 = 0x2c;
const AIR_JAGUAR_KICK: u8 = 0x2d;
const ANGRY_CHARGE: u8 = 0x2f;

const SAGAT_SQUAT: u32 = 0x14242;
const SAGAT_FALL: u32 = 0x14268;
const SAGAT_LOW_KICK: u32 = 0x13fc6;
const SAGAT_KNEE: u32 = 0x1412c;
const SAGAT_UPPERCUT: u32 = 0x13f36;
const SAGAT_SHOT_CHAMBER: u32 = 0x144da;

const ADON_SQUAT: u32 = 0x12dc6;
const ADON_FALL: u32 = 0x12e0c;
const ADON_APEX: u32 = 0x12e4c;
const ADON_PUNCH: u32 = 0x13392;
const ADON_KNEE: u32 = 0x13896;
const ADON_FLYING_KICK: u32 = 0x135b8;
const ADON_FLYING_KICK_HIT: u32 = 0x47dce;

fn super_hit(rom: &[u8], region: &mut Region, reaction: u32, damage: u8) -> u32 {
    let mut entry = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, reaction + i as u32));
    entry[1] = damage;
    region.put(&entry)
}

fn write_header(region: &mut Region, id: u8, first: Rec, state: u8, sounds: [u8; 3], stream: u32) {
    let h = header(
        &first.bytes(),
        first.pose.top_on_ground(),
        state,
        id,
        sounds,
        stream,
    );
    put_header(region, region.base, id, &h);
}

pub(super) fn write_sagat(rom: &[u8], region: &mut Region) {
    // Tiger Genocide and Tiger Uppercut already use the SF2 streams.
    write_tiger_cannon(rom, region);
    write_tiger_raid(rom, region);
    write_angry_charge(rom, region);
}

fn write_tiger_cannon(rom: &[u8], region: &mut Region) {
    const SHOT_TEMPLATE: u32 = 0x1873c;
    const SHOT_FLIGHT: u32 = 0x190b6;
    const SHOT_CAST: u32 = 0x144b4 + 0x20;
    const SHOT_SPAWN: u32 = 0x45ace;
    const CANNON_HALF_HEIGHT: u8 = 26;
    const CANNON_HALF_WIDTH: u8 = 28;
    // Keep the Tiger Shot's speed and animation, extending the Super's
    // contact window so it can deliver all five hits as it travels.
    let flight = region.here();
    for frame in 0..3 {
        let mut record =
            std::array::from_fn::<_, 8, _>(|i| rom_b(rom, SHOT_FLIGHT + frame * 8 + i as u32));
        record[4..6].copy_from_slice(&[CANNON_HALF_HEIGHT, CANNON_HALF_WIDTH]);
        region.put(&record);
    }
    region.put(&[0xfe, 0]);
    region.put_l(flight);

    let hit = region.put(&[ROUTINE_EFFECT_ROM_HIT, 0, 0, 0]);
    region.put_l(0x43652);
    region.put_l(0x436a8);
    let mut template = std::array::from_fn::<_, 0x20, _>(|i| rom_b(rom, SHOT_TEMPLATE + i as u32));
    template[0x0c..0x0e].copy_from_slice(&[CANNON_HALF_HEIGHT, CANNON_HALF_WIDTH]);
    template[0x10..0x14].copy_from_slice(&flight.to_be_bytes());
    template[0x14..0x18].copy_from_slice(&hit.to_be_bytes());
    let template = region.put(&template);
    let spawn = region.put(&[ROUTINE_PROJECTILE, 7, 0, 0]);
    region.put_l(template);

    let stream = region.here();
    copy_rom_stream(
        region,
        rom,
        SHOT_CAST,
        CopyHow {
            replace: &[(SHOT_SPAWN, spawn)],
            ..CopyHow::default()
        },
    );
    region.patch(stream + 0x2d, &[sagat::high_tiger_shot::HOLD - 1]);
    let source = bank_header_offset(0x1a) as usize;
    let mut cannon = [0; MOVE_HEADER_SIZE as usize];
    cannon.copy_from_slice(&region.bytes[source..source + MOVE_HEADER_SIZE as usize]);
    cannon[0x0a] = ALPHA_SUPER_SECOND_SLOT;
    cannon[0x2c..0x30].copy_from_slice(&stream.to_be_bytes());
    put_header(region, region.base, ALPHA_SUPER_SECOND_SLOT, &cannon);
}

fn write_tiger_raid(rom: &[u8], region: &mut Region) {
    let ready = rom_pose(rom, SAGAT_SQUAT);
    let ready_boxes = rom_boxes(rom, SAGAT_SQUAT);
    let low_kick = rom_pose(rom, SAGAT_LOW_KICK);
    let low_boxes = rom_boxes(rom, SAGAT_LOW_KICK);
    let knee = rom_pose(rom, SAGAT_KNEE);
    let mut knee_boxes = rom_boxes(rom, SAGAT_KNEE);
    knee_boxes[12..16].copy_from_slice(&[4, 22, 20, 27]);
    let uppercut = rom_pose(rom, SAGAT_UPPERCUT);
    let mut uppercut_boxes = rom_boxes(rom, SAGAT_UPPERCUT);
    uppercut_boxes[12..16].copy_from_slice(&[12, 26, 14, 31]);
    let fall = rom_pose(rom, SAGAT_FALL);
    let fall_boxes = rom_boxes(rom, SAGAT_FALL);
    let hit = super_hit(rom, region, 0x45c88, 4);
    let finish = super_hit(rom, region, 0x45f52, 5);
    let (knee_origin, knee_dy) = ground_origin_record(region, ready, knee);
    let (uppercut_origin, uppercut_dy) = ground_origin_record(region, knee, uppercut);

    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    for _ in 0..2 {
        s.travel(ready, ready.top_on_ground(), 3, (0, 4), ready_boxes, 0, 0);
        s.call(routine::SOUND, 0xa2);
        s.hold(low_kick, low_kick.top_on_ground(), 3, low_boxes, 0x80, hit);
    }
    s.hold(ready, ready.top_on_ground(), 1, ready_boxes, 0, 0);
    s.change_origin(knee_origin, knee_dy);
    s.call(routine::RISING, 0);
    s.strike(knee, 3, (-3, 4), knee_boxes, 0x80, hit);
    s.strike(knee, 3, (-2, 3), knee_boxes, 0x80, hit);
    s.change_origin(uppercut_origin, uppercut_dy);
    s.strike(uppercut, 4, (-4, 2), uppercut_boxes, 0x80, finish);
    s.call(routine::FALLING, 0);
    s.travel(fall, fall.top_on_ground() - 31, 5, (3, 1), fall_boxes, 0, 0);
    s.travel(fall, fall.top_on_ground() - 16, 4, (4, 0), fall_boxes, 0, 0);
    s.call(routine::TOUCHDOWN, 0);
    s.hold(ready, ready.top_on_ground(), 3, ready_boxes, 0, 0);
    s.call(routine::LANDING, 0);
    s.hold(ready, ready.top_on_ground(), 12, ready_boxes, 0, 0);
    s.end();

    let first = still(ready, 4, ready_boxes);
    write_header(
        region,
        ALPHA_SUPER_THIRD_SLOT,
        first,
        0xc4,
        [0, 0, 0x19],
        stream,
    );
}

fn write_angry_charge(rom: &[u8], region: &mut Region) {
    let (stance, stance_boxes) = header_pose(
        rom,
        TournamentFighter::Sagat.move_table() + MOVE_HEADER_SIZE,
    );
    let chamber = rom_pose(rom, SAGAT_SHOT_CHAMBER);
    let chamber_boxes = rom_boxes(rom, SAGAT_SHOT_CHAMBER);
    let stream = region.here();
    let mut s = Stream::new(region, stance, stance.top_on_ground());
    s.call(routine::SOUND, 0xb8);
    s.hold(chamber, chamber.top_on_ground(), 24, chamber_boxes, 0, 0);
    s.hold(stance, stance.top_on_ground(), 8, stance_boxes, 0, 0);
    s.end();

    let first = still(stance, 4, stance_boxes);
    write_header(region, ANGRY_CHARGE, first, 0xc2, [0; 3], stream);
}

pub(super) fn write_adon(rom: &[u8], region: &mut Region) {
    let hit = super_hit(rom, region, 0x45c88, 4);
    let finish = super_hit(rom, region, 0x45f52, 5);
    write_jaguar_assault(rom, region, hit, finish);
    write_jaguar_revolver(rom, region, hit, finish);
    write_jaguar_tooth(rom, region);
    write_air_jaguar_kick(rom, region);
}

fn write_jaguar_assault(rom: &[u8], region: &mut Region, hit: u32, finish: u32) {
    let (stance, stance_boxes) =
        header_pose(rom, TournamentFighter::Adon.move_table() + MOVE_HEADER_SIZE);
    let punch = rom_pose(rom, ADON_PUNCH);
    let mut punch_boxes = rom_boxes(rom, ADON_PUNCH);
    punch_boxes[12..16].copy_from_slice(&[0, 17, 19, 32]);
    let knee = rom_pose(rom, ADON_KNEE);
    let mut knee_boxes = rom_boxes(rom, ADON_KNEE);
    knee_boxes[12..16].copy_from_slice(&[6, 24, 22, 29]);
    let (knee_origin, knee_dy) = ground_origin_record(region, stance, knee);
    let stream = region.here();
    let mut s = Stream::new(region, stance, stance.top_on_ground());
    for (pose, boxes) in [
        (punch, punch_boxes),
        (knee, knee_boxes),
        (punch, punch_boxes),
    ] {
        s.travel(
            stance,
            stance.top_on_ground(),
            3,
            (0, 5),
            stance_boxes,
            0,
            0,
        );
        s.hold(pose, pose.top_on_ground(), 3, boxes, 0x80, hit);
    }
    s.hold(stance, stance.top_on_ground(), 1, stance_boxes, 0, 0);
    s.change_origin(knee_origin, knee_dy);
    s.call(routine::RISING, 0);
    s.strike(knee, 4, (-4, 4), knee_boxes, 0x80, hit);
    s.strike(knee, 4, (-3, 3), knee_boxes, 0x80, finish);
    adon_land(rom, &mut s, 28, 0, 12);

    let first = still(stance, 4, stance_boxes);
    write_header(region, SF2_SUPER_SLOT, first, 0xc2, [0xa1, 0, 0x19], stream);
}

fn write_jaguar_revolver(rom: &[u8], region: &mut Region, hit: u32, finish: u32) {
    let ready = rom_pose(rom, ADON_SQUAT);
    let ready_boxes = rom_boxes(rom, ADON_SQUAT);
    let kick = rom_pose(rom, ADON_FLYING_KICK);
    let mut kick_boxes = rom_boxes(rom, ADON_FLYING_KICK);
    kick_boxes[12..16].copy_from_slice(&[0xf7, 23, 24, 36]);
    let (kick_origin, kick_dy) = ground_origin_record(region, ready, kick);
    let knee = rom_pose(rom, ADON_KNEE);
    let mut knee_boxes = rom_boxes(rom, ADON_KNEE);
    knee_boxes[12..16].copy_from_slice(&[6, 24, 22, 29]);
    let (knee_origin, knee_dy) = ground_origin_record(region, kick, knee);
    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    for _ in 0..2 {
        s.call(routine::RISING, 0);
        s.travel(ready, ready.top_on_ground(), 3, (-4, 4), ready_boxes, 0, 0);
        s.change_origin(kick_origin, kick_dy);
        s.call(routine::SOUND, 0xa2);
        s.strike(kick, 4, (0, 4), kick_boxes, 0x80, hit);
        adon_land(rom, &mut s, 12, 3, 0);
    }
    s.call(routine::RISING, 0);
    s.travel(ready, ready.top_on_ground(), 3, (-4, 4), ready_boxes, 0, 0);
    s.change_origin(kick_origin, kick_dy);
    s.strike(kick, 3, (-2, 4), kick_boxes, 0x80, hit);
    s.change_origin(knee_origin, knee_dy);
    s.strike(knee, 4, (-3, 3), knee_boxes, 0x80, hit);
    s.strike(knee, 4, (-3, 2), knee_boxes, 0x80, finish);
    adon_land(rom, &mut s, 42, 0, 12);

    let first = still(ready, 4, ready_boxes);
    write_header(
        region,
        ALPHA_SUPER_SECOND_SLOT,
        first,
        0xc2,
        [0xa2, 0, 0x19],
        stream,
    );
}

fn write_jaguar_tooth(rom: &[u8], region: &mut Region) {
    let ready = rom_pose(rom, ADON_SQUAT);
    let ready_boxes = rom_boxes(rom, ADON_SQUAT);
    let wall = rom_pose(rom, ADON_FALL);
    let wall_boxes = rom_boxes(rom, ADON_FALL);
    let kick = rom_pose(rom, ADON_FLYING_KICK);
    let kick_boxes = rom_boxes(rom, ADON_FLYING_KICK);
    let (kick_origin, kick_dy) = ground_origin_record(region, wall, kick);
    let wall_check = region.put(&[ROUTINE_ALPHA_WALL_REBOUND, 0, 0, 0]);
    region.put_l(0);
    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    s.call(routine::RISING, 0);
    s.travel(wall, wall.top_on_ground(), 6, (-7, -8), wall_boxes, 0, 0);
    let retreat = s.travel(
        wall,
        wall.top_on_ground() - 42,
        1,
        (0, -9),
        wall_boxes,
        0,
        0,
    );
    s.call(wall_check, 0);
    s.change_origin(kick_origin, kick_dy);
    s.call(routine::SOUND, 0xa2);
    s.strike(kick, 19, (2, 11), kick_boxes, 0x80, ADON_FLYING_KICK_HIT);
    adon_land(rom, &mut s, 4, 4, 12);
    region.patch(wall_check + 4, &retreat.to_be_bytes());

    let first = still(ready, 4, ready_boxes);
    write_header(region, JAGUAR_TOOTH, first, 0xc2, [0, 0, 0x19], stream);
}

fn write_air_jaguar_kick(rom: &[u8], region: &mut Region) {
    let apex = rom_pose(rom, ADON_APEX);
    let apex_boxes = rom_boxes(rom, ADON_APEX);
    let kick = rom_pose(rom, ADON_FLYING_KICK);
    let kick_boxes = rom_boxes(rom, ADON_FLYING_KICK);
    let kick_dy = (kick.top_on_ground() + kick.half().1) - (apex.top_on_ground() + apex.half().1);
    let kick_origin = pose_origin_record(region, kick_dy, 0xa2);
    let stream = region.here();
    // The air start preserves the current altitude and corrects this
    // neutral-jump trajectory so it still ends on the standing line.
    let apex_top = jump_top_at_apexes(
        rom,
        TournamentFighter::Adon.move_table(),
        &[(0x09, 0x12de6, ADON_APEX)],
        apex,
    )[0];
    let mut s = Stream::new(region, apex, apex_top);
    s.change_origin(kick_origin, kick_dy);
    s.call(routine::FALLING, 0);
    s.strike(kick, 10, (3, 6), kick_boxes, 0x80, ADON_FLYING_KICK_HIT);
    let altitude = kick.top_on_ground() - s.top;
    adon_land(rom, &mut s, altitude, 4, 8);

    let first = still(apex, 1, apex_boxes);
    let h = header(
        &first.bytes(),
        apex_top,
        0xc2,
        AIR_JAGUAR_KICK,
        [0, 0, 0x19],
        stream,
    );
    put_header(region, region.base, AIR_JAGUAR_KICK, &h);
}

fn adon_land(rom: &[u8], s: &mut Stream<'_>, altitude: i32, dx: i32, recovery: u8) {
    let fall = rom_pose(rom, ADON_FALL);
    let ready = rom_pose(rom, ADON_SQUAT);
    let fall_boxes = rom_boxes(rom, ADON_FALL);
    let ready_boxes = rom_boxes(rom, ADON_SQUAT);
    let fall_top = fall.top_on_ground() - altitude;
    s.call(routine::FALLING, 0);
    let frames = (altitude / 4) as u8;
    if frames != 0 {
        s.travel(fall, fall_top, frames, (4, dx), fall_boxes, 0, 0);
    }
    let remainder = altitude % 4;
    if remainder != 0 {
        s.travel(
            fall,
            fall.top_on_ground() - remainder,
            1,
            (remainder, dx),
            fall_boxes,
            0,
            0,
        );
    }
    s.call(routine::TOUCHDOWN, 0);
    s.hold(ready, ready.top_on_ground(), 1, ready_boxes, 0, 0);
    if recovery != 0 {
        s.call(routine::LANDING, 0);
        s.hold(ready, ready.top_on_ground(), recovery, ready_boxes, 0, 0);
        s.end();
    }
}
