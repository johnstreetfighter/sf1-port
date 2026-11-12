use super::*;

const GEKIROU: u8 = 0x1b;
const CHANGE_STANCE: u8 = 0x1f;
const CRANE_IDLE: u8 = 0x28;
const ZANEI: u8 = 0x29;
const SHITENSHUU: u8 = 0x2a;
const JAKOUHA: u8 = 0x2b;
const KOUGA: u8 = 0x2c;
const JASEN: u8 = 0x2d;
const OUGA: u8 = 0x2f;

const ADVANCE: u32 = 0x10b6e;
const PALM: u32 = 0x10bf4;
const ARMS_CROSSED: u32 = 0x10b4e;
const ARMS_SPREAD: u32 = 0x10b8e;
const JAB_WINDUP: u32 = 0x103b0;
const PRESSURE_POINT: u32 = 0x103d6;
const CRANE_GUARD: u32 = 0x104f0;
const HIGH_KICK: u32 = 0x10970;
const LOW_KICK: u32 = 0x10930;
const FLYING_KICK: u32 = 0x1064c;
const AIR_TUCK: u32 = 0x10626;
const FALL: u32 = 0x10060;
const LAND: u32 = 0x10086;
const ROLL_PREP: u32 = 0x10a26;
const ROLL_TUCK: u32 = 0x10a4c;
const ROLL_STRIKE: u32 = 0x10a72;

const ACTIVE: u8 = 0x80;
const INVULNERABLE: u8 = 0x08;
const PUNCH_REACTION: u32 = 0x45c88;
const KNOCKDOWN_REACTION: u32 = 0x45f52;

pub(super) fn build(rom: &[u8], region: &mut Region) {
    if rom.len() < (KNOCKDOWN_REACTION + 8) as usize {
        return;
    }

    // His kick chamber reads as a one-legged guard without new tile art.
    // Preserve the logical idle ID so it follows the normal idle path.
    let guard = rom_pose(rom, CRANE_GUARD);
    let first = still(guard, 1, rom_boxes(rom, CRANE_GUARD));
    let idle = header(&first.bytes(), guard.top_on_ground(), 0x12, 1, [0; 3], 0);
    put_header(region, region.base, CRANE_IDLE, &idle);

    write_stance_change(rom, region);
    write_zanei(rom, region);
    write_shitenshuu(rom, region);
    write_gekirou(rom, region);
    write_jakouha(rom, region);
    write_jasen(rom, region);
    write_ouga(rom, region);
    write_kouga(rom, region);
}

fn hit_entry(rom: &[u8], region: &mut Region, source: u32, damage: u8) -> u32 {
    let mut entry: [u8; 8] = std::array::from_fn(|i| rom_b(rom, source + i as u32));
    entry[1] = damage;
    region.put(&entry)
}

fn install(region: &mut Region, id: u8, first: Rec, top: i32, state: u8, stream: u32) {
    let header = header(&first.bytes(), top, state, id, [0, 0, 5], stream);
    put_header(region, region.base, id, &header);
}

fn land(rom: &[u8], stream: &mut Stream<'_>) {
    let pose = rom_pose(rom, LAND);
    let boxes = rom_boxes(rom, LAND);
    stream.call(routine::TOUCHDOWN, 0);
    stream.hold(pose, pose.top_on_ground(), 3, boxes, 0, 0);
    stream.call(routine::LANDING, 0);
    stream.hold(pose, pose.top_on_ground(), 9, boxes, 0, 0);
    stream.end();
}

fn jump_to(stream: &mut Stream<'_>, target: u32) {
    stream.region.put(&[0xfe, 0]);
    stream.region.put_l(target);
}

fn write_stance_change(rom: &[u8], region: &mut Region) {
    let crossed = rom_pose(rom, ARMS_CROSSED);
    let mut boxes = rom_boxes(rom, ARMS_CROSSED);
    boxes[4..8].copy_from_slice(&rom_boxes(rom, JAB_WINDUP)[4..8]);
    let spread = rom_pose(rom, ARMS_SPREAD);
    let stream = region.here();
    let mut s = Stream::new(region, crossed, crossed.top_on_ground());
    s.hold(
        spread,
        spread.top_on_ground(),
        3,
        rom_boxes(rom, ARMS_SPREAD),
        0,
        0,
    );
    s.end();
    let first = still(crossed, 3, boxes);
    install(
        region,
        CHANGE_STANCE,
        first,
        crossed.top_on_ground(),
        0xc2,
        stream,
    );
}

fn write_zanei(rom: &[u8], region: &mut Region) {
    let windup = rom_pose(rom, ADVANCE);
    let windup_boxes = rom_boxes(rom, ADVANCE);
    let palm = rom_pose(rom, PALM);
    let mut boxes = rom_boxes(rom, PALM);
    // Keep the rapid palm in range through its close-range recoil.
    boxes[12..16].copy_from_slice(&[8, 22, 27, 25]);
    let hit = hit_entry(rom, region, PUNCH_REACTION, 4);
    let finish = hit_entry(rom, region, KNOCKDOWN_REACTION, 6);
    let stream = region.here();
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    s.travel(
        windup,
        windup.top_on_ground(),
        5,
        (0, 9),
        windup_boxes,
        INVULNERABLE,
        0,
    );
    s.call(routine::SOUND, 0xa1);
    for _ in 0..4 {
        s.strike(palm, 3, (0, 4), boxes, ACTIVE, hit);
    }
    s.strike(palm, 3, (0, 3), boxes, ACTIVE, finish);
    s.hold(windup, windup.top_on_ground(), 12, windup_boxes, 0, 0);
    s.end();
    let mut first = still(windup, 3, windup_boxes);
    first.flags = INVULNERABLE;
    install(region, ZANEI, first, windup.top_on_ground(), 0xc2, stream);
}

fn write_shitenshuu(rom: &[u8], region: &mut Region) {
    let windup = rom_pose(rom, JAB_WINDUP);
    let windup_boxes = rom_boxes(rom, JAB_WINDUP);
    let palm = rom_pose(rom, PRESSURE_POINT);
    let mut boxes = rom_boxes(rom, PRESSURE_POINT);
    boxes[12..16].copy_from_slice(&[16, 18, 21, 23]);
    let hit = hit_entry(rom, region, PUNCH_REACTION, 10);
    let stream = region.here();
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    s.travel(
        windup,
        windup.top_on_ground(),
        4,
        (0, 3),
        windup_boxes,
        0,
        0,
    );
    s.call(routine::SOUND, 0xa1);
    s.hold(palm, palm.top_on_ground(), 5, boxes, ACTIVE, hit);
    s.hold(windup, windup.top_on_ground(), 15, windup_boxes, 0, 0);
    s.end();
    let mut first = still(windup, 5, windup_boxes);
    first.flags = INVULNERABLE;
    install(
        region,
        SHITENSHUU,
        first,
        windup.top_on_ground(),
        0xc2,
        stream,
    );
}

fn write_jakouha(rom: &[u8], region: &mut Region) {
    let guard = rom_pose(rom, CRANE_GUARD);
    let guard_boxes = rom_boxes(rom, CRANE_GUARD);
    let kick = rom_pose(rom, HIGH_KICK);
    let mut boxes = rom_boxes(rom, HIGH_KICK);
    boxes[12..16].copy_from_slice(&[25, 20, 20, 24]);
    let hit = hit_entry(rom, region, PUNCH_REACTION, 6);
    let finish = hit_entry(rom, region, KNOCKDOWN_REACTION, 8);
    let sound = region.put(&[ROUTINE_POSE_ORIGIN, 0xb4, 0, 0]);
    let stream = region.here();
    let mut s = Stream::new(region, guard, guard.top_on_ground());
    s.call(sound, 0);
    s.call(routine::RISING, 0);
    s.strike(kick, 4, (-6, 3), boxes, ACTIVE | INVULNERABLE, hit);
    s.strike(kick, 4, (-5, 2), boxes, ACTIVE, hit);
    s.strike(kick, 4, (-4, 1), boxes, ACTIVE, finish);
    s.call(routine::FALLING, 0);
    let fall = rom_pose(rom, FALL);
    let fall_boxes = rom_boxes(rom, FALL);
    s.travel(fall, fall.top_on_ground() - 60, 6, (4, 0), fall_boxes, 0, 0);
    s.travel(fall, fall.top_on_ground() - 36, 6, (6, 0), fall_boxes, 0, 0);
    land(rom, &mut s);
    let mut first = still(guard, 4, guard_boxes);
    first.flags = INVULNERABLE;
    install(region, JAKOUHA, first, guard.top_on_ground(), 0xc2, stream);
}

fn write_gekirou(rom: &[u8], region: &mut Region) {
    let crouch = rom_pose(rom, LAND);
    let crouch_boxes = rom_boxes(rom, LAND);
    let high = rom_pose(rom, HIGH_KICK);
    let low = rom_pose(rom, LOW_KICK);
    let mut high_boxes = rom_boxes(rom, HIGH_KICK);
    let mut low_boxes = rom_boxes(rom, LOW_KICK);
    high_boxes[12..16].copy_from_slice(&[16, 25, 23, 21]);
    low_boxes[12..16].copy_from_slice(&[6, 24, 23, 21]);
    let launch = hit_entry(rom, region, KNOCKDOWN_REACTION, 5);
    let hit = hit_entry(rom, region, PUNCH_REACTION, 2);
    let finish = hit_entry(rom, region, KNOCKDOWN_REACTION, 4);
    let sound = region.put(&[ROUTINE_POSE_ORIGIN, 0xb4, 0, 0]);

    // Each window permits one additional kick. Its alternate stream falls
    // from that kick's exact height if the player stops tapping.
    let rises = [6, 5, 4, 3, 3, 2, 2, 0];
    let mut height = 0;
    let mut exits = [0; 8];
    for (index, rise) in rises.into_iter().enumerate() {
        let kick = if index % 2 == 0 { high } else { low };
        height += 4 * rise;
        let descent = write_gekirou_descent(rom, region, kick, height);
        if index < rises.len() - 1 {
            exits[index] = region.put(&[ROUTINE_ALPHA_REPEAT_K, 0, 0, 0]);
            region.put_l(descent);
        } else {
            exits[index] = descent;
        }
    }

    let stream = region.here();
    let mut s = Stream::new(region, crouch, crouch.top_on_ground());
    s.call(sound, 0);
    s.call(routine::RISING, 0);
    for (index, rise) in rises.into_iter().enumerate() {
        let (kick, boxes) = if index % 2 == 0 {
            (high, high_boxes)
        } else {
            (low, low_boxes)
        };
        let entry = match index {
            0 => launch,
            7 => finish,
            _ => hit,
        };
        let advance = if index < 3 { 2 } else { 1 };
        s.strike(kick, 4, (-rise, advance), boxes, ACTIVE, entry);
        if index < rises.len() - 1 {
            s.call(exits[index], 0);
        } else {
            // The eighth kick always finishes with its own descent.
            jump_to(&mut s, exits[index]);
        }
    }
    let mut first = still(crouch, 4, crouch_boxes);
    first.flags = 0x20;
    // Gekirou uses later-era command tracking. The base table only owns
    // Gen's legacy Gekiro and Hyakurenko.
    let move_header = header(
        &first.bytes(),
        crouch.top_on_ground(),
        0xc4,
        GEKIROU,
        [0, 0, 5],
        stream,
    );
    use EraFamily::{Sf2, Sf3, Sf4, Sf5, Sf6, SfZ};
    for family in [Sf2, SfZ, Sf3, Sf4, Sf5, Sf6] {
        put_era_header(region, family, GEKIROU, &move_header);
    }
}

fn write_gekirou_descent(rom: &[u8], region: &mut Region, kick: PoseGeom, height: i32) -> u32 {
    let tuck = rom_pose(rom, AIR_TUCK);
    let fall = rom_pose(rom, FALL);
    let stream = region.here();
    let mut s = Stream::new(region, kick, kick.top_on_ground() - height);
    s.call(routine::FALLING, 0);
    s.travel(
        tuck,
        tuck.top_on_ground() - height,
        4,
        (2, -2),
        rom_boxes(rom, AIR_TUCK),
        0,
        0,
    );
    let remaining = height - 8;
    s.travel(
        fall,
        fall.top_on_ground() - remaining,
        (remaining / 5) as u8,
        (5, -1),
        rom_boxes(rom, FALL),
        0,
        0,
    );
    land(rom, &mut s);
    stream
}

fn write_jasen(rom: &[u8], region: &mut Region) {
    let windup = rom_pose(rom, ROLL_PREP);
    let windup_boxes = rom_boxes(rom, ROLL_PREP);
    let tuck = rom_pose(rom, ROLL_TUCK);
    let tuck_boxes = rom_boxes(rom, ROLL_TUCK);
    let strike = rom_pose(rom, ROLL_STRIKE);
    let (strike_origin, strike_dy) = ground_origin_record(region, tuck, strike);
    let hit = hit_entry(rom, region, KNOCKDOWN_REACTION, 8);
    let stream = region.here();
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    s.travel(tuck, tuck.top_on_ground(), 8, (0, 6), tuck_boxes, 0, 0);
    s.change_origin(strike_origin, strike_dy);
    s.call(routine::SOUND, 0xa2);
    s.strike(strike, 6, (0, 5), rom_boxes(rom, ROLL_STRIKE), ACTIVE, hit);
    s.hold(tuck, tuck.top_on_ground(), 4, tuck_boxes, 0, 0);
    s.hold(windup, windup.top_on_ground(), 10, windup_boxes, 0, 0);
    s.end();
    let first = still(windup, 4, windup_boxes);
    install(region, JASEN, first, windup.top_on_ground(), 0xc4, stream);
}

fn dive_kick(rom: &[u8]) -> (PoseGeom, [u8; 16]) {
    let mut boxes = rom_boxes(rom, FLYING_KICK);
    boxes[12..16].copy_from_slice(&[0xe9, 14, 30, 22]);
    (rom_pose(rom, FLYING_KICK), boxes)
}

fn write_ouga(rom: &[u8], region: &mut Region) {
    let guard = rom_pose(rom, CRANE_GUARD);
    let guard_boxes = rom_boxes(rom, CRANE_GUARD);
    let rise = rom_pose(rom, FALL);
    let rise_boxes = rom_boxes(rom, FALL);
    let tuck = rom_pose(rom, AIR_TUCK);
    let tuck_boxes = rom_boxes(rom, AIR_TUCK);
    let (kick, kick_boxes) = dive_kick(rom);
    let hit = hit_entry(rom, region, KNOCKDOWN_REACTION, 9);
    let peak = tuck.top_on_ground() - 60;
    let rebound = region.here();
    let mut s = Stream::new(region, tuck, peak);
    s.hold(tuck, peak, 4, tuck_boxes, 0, 0);
    s.call(routine::FALLING, 0);
    s.travel(tuck, peak, 2, (5, 12), tuck_boxes, 0, 0);
    s.call(routine::SOUND, 0xb4);
    s.strike(kick, 10, (5, 13), kick_boxes, ACTIVE, hit);
    land(rom, &mut s);

    // Keep travelling back at the apex until the shared rear-wall check
    // sees the screen band. The finite seek still recovers on wide stages.
    let mut seek = rebound;
    for _ in 0..6 {
        let check = region.put(&[ROUTINE_ALPHA_WALL_REBOUND, 0, 0, 0]);
        region.put_l(seek);
        seek = region.here();
        let mut s = Stream::new(region, tuck, peak);
        s.travel(tuck, peak, 4, (0, -8), tuck_boxes, 0, 0);
        s.call(check, 0);
        jump_to(&mut s, rebound);
    }
    let check = region.put(&[ROUTINE_ALPHA_WALL_REBOUND, 0, 0, 0]);
    region.put_l(seek);
    let stream = region.here();
    let mut s = Stream::new(region, guard, guard.top_on_ground());
    s.call(routine::RISING, 0);
    s.travel(rise, rise.top_on_ground(), 6, (-6, -8), rise_boxes, 0, 0);
    s.travel(
        rise,
        rise.top_on_ground() - 36,
        6,
        (-4, -8),
        rise_boxes,
        0,
        0,
    );
    s.hold(tuck, peak, 1, tuck_boxes, 0, 0);
    s.call(check, 0);
    jump_to(&mut s, rebound);
    let first = still(guard, 4, guard_boxes);
    install(region, OUGA, first, guard.top_on_ground(), 0xc2, stream);
}

fn write_kouga(rom: &[u8], region: &mut Region) {
    let tuck = rom_pose(rom, AIR_TUCK);
    let tuck_boxes = rom_boxes(rom, AIR_TUCK);
    let (kick, kick_boxes) = dive_kick(rom);
    let hit = hit_entry(rom, region, PUNCH_REACTION, 5);
    let finish = hit_entry(rom, region, KNOCKDOWN_REACTION, 7);
    let sound = region.put(&[ROUTINE_POSE_ORIGIN, 0xb4, 0, 0]);
    let top = tuck.top_on_ground() - 56;
    let stream = region.here();
    let mut s = Stream::new(region, tuck, top);
    s.call(routine::RISING, 0);
    s.travel(tuck, top, 4, (-7, -6), tuck_boxes, INVULNERABLE, 0);
    s.hold(tuck, tuck.top_on_ground() - 84, 4, tuck_boxes, 0, 0);
    s.call(sound, 0);
    s.call(routine::FALLING, 0);
    s.strike(kick, 7, (4, 13), kick_boxes, ACTIVE, hit);
    s.strike(kick, 7, (4, 12), kick_boxes, ACTIVE, hit);
    s.strike(kick, 7, (4, 10), kick_boxes, ACTIVE, finish);
    land(rom, &mut s);
    let mut first = still(tuck, 4, tuck_boxes);
    first.flags = INVULNERABLE;
    install(region, KOUGA, first, top, 0xc1, stream);
}
