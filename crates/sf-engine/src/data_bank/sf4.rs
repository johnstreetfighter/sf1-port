//! Native Focus, dash, and fighter move headers for Street Fighter IV.
use super::*;
use crate::era::EraFamily::Sf4;
use crate::era_fight::sf4::{BACKDASH, DASH, FOCUS, FOCUS_CHARGE, RYUSOKYAKU};
use TournamentFighter::{Adon, Gen, Ken, Ryu, Sagat};

pub(super) fn build(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    let (windup, strike, retract, aliases): (_, _, _, &[(u8, u8)]) = match fighter {
        Ryu => (
            0x0af0,
            0x0b30,
            0x0b50,
            &[
                (SF2_SPECIAL_SLOT, SF2_SPECIAL_SLOT),
                (0x29, 0x29),
                (0x2a, 0x29),
                (0x2b, 0x2a),
            ],
        ),
        Ken => (
            0x2bce,
            0x2c0e,
            0x2c2e,
            &[(0x29, 0x29), (0x2a, 0x2a), (0x2b, 0x2b)],
        ),
        Sagat => (
            0x14030,
            0x14056,
            0x14096,
            &[
                (0x1f, 0x1f),
                (0x29, 0x29),
                (0x2a, 0x2b),
                (0x2b, 0x2a),
                (0x2c, 0x2f),
            ],
        ),
        Adon => (
            0x1336c,
            0x13392,
            0x133b2,
            &[(0x20, 0x2c), (0x21, 0x2d), (0x29, 0x29), (0x2a, 0x2a)],
        ),
        Gen => (
            0x103b0,
            0x103d6,
            0x103f6,
            &[
                (0x1c, 0x2c),
                (0x1f, 0x1f),
                (0x20, 0x2d),
                (0x21, 0x2f),
                (0x29, 0x29),
                (0x2a, 0x29),
                (0x2b, 0x2b),
                (0x2c, 0x2a),
                (0x2d, 0x2b),
            ],
        ),
        _ => return,
    };
    if rom.len() < 0x45f5a {
        return;
    }
    for &(slot, source) in aliases {
        alias_era_header(region, Sf4, slot, source);
    }
    for slot in SPECIAL_SLOTS {
        alias_era_header(region, Sf4, slot, slot);
    }
    write_focus(rom, region, windup, strike, retract);
    write_dash(rom, region, fighter, DASH, 4);
    write_dash(rom, region, fighter, BACKDASH, -4);
    match fighter {
        Ryu => write_evil_ryu_moves(rom, region),
        Adon => write_jaguar_avalanche(rom, region),
        _ => {}
    }
}

fn install(region: &mut Region, id: u8, first: Rec, stream: u32) {
    let h = header(
        &first.bytes(),
        first.pose.top_on_ground(),
        0xc2,
        id,
        [0; 3],
        stream,
    );
    put_era_header(region, Sf4, id, &h);
}

fn hit_entry(rom: &[u8], region: &mut Region, source: u32, damage: u8) -> u32 {
    let mut entry = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, source + i as u32));
    entry[1] = damage;
    region.put(&entry)
}

fn write_evil_ryu_moves(rom: &[u8], region: &mut Region) {
    // SFIII's builder has already installed the normal ROM air hurricane
    // in the SFII family for the shoto profiles that share it.
    let air_tatsu = crate::era_fight::sf3::AIR_TATSU;
    let pointer = era_header_offset(EraFamily::Sf2, air_tatsu).unwrap() as usize;
    let address = u32::from_be_bytes(region.bytes[pointer..pointer + 4].try_into().unwrap());
    if address != 0 {
        bind_era_header(region, Sf4, air_tatsu, address);
    }

    const KNEE_LIFT: u32 = 0x00e82;
    const LEG_RAISE: u32 = 0x00ea2;
    const LEG_DROP: u32 = 0x01a12;
    const LAND: u32 = 0x00d5e;
    const LIFT_FRAMES: u8 = 8;
    const STANCE_FRAMES: u8 = 5;
    // Startup (including the first active frame), active and recovery.
    // Ultra shortened only HK startup, from 26 to 25 frames.
    const TIMINGS: [(u8, u8, u8); 5] = [
        (17, 2, 22),
        (20, 2, 20),
        (26, 2, 18),
        (21, 2, 22),
        (25, 2, 18),
    ];
    let [lift, raise, drop, land] =
        [KNEE_LIFT, LEG_RAISE, LEG_DROP, LAND].map(|at| rom_pose(rom, at));
    let lift_boxes = rom_boxes(rom, KNEE_LIFT);
    let raise_boxes = rom_boxes(rom, LEG_RAISE);
    let mut drop_boxes = rom_boxes(rom, LEG_DROP);
    let land_boxes = rom_boxes(rom, LAND);
    let (stance, stance_boxes) = header_pose(rom, Ryu.move_table() + MOVE_HEADER_SIZE);
    // The diagonal jump kick supplies the descending leg. Put its feet at
    // the standing line and cover only the downward extension with the hit.
    drop_boxes[12..16].copy_from_slice(&[0xee, 10, 27, 18]);
    let hit = hit_entry(rom, region, 0x45c88, 6);
    let variants = TIMINGS.map(|(startup, active, recovery)| {
        let stream = region.here();
        let mut s = Stream::new(region, lift, lift.top_on_ground());
        s.hold(
            raise,
            raise.top_on_ground(),
            startup - LIFT_FRAMES - 1,
            raise_boxes,
            0,
            0,
        );
        s.call(routine::SOUND, 0xa2);
        s.hold(drop, drop.top_on_ground(), active, drop_boxes, 0x80, hit);
        s.hold(
            land,
            land.top_on_ground(),
            recovery - STANCE_FRAMES,
            land_boxes,
            0,
            0,
        );
        s.hold(
            stance,
            stance.top_on_ground(),
            STANCE_FRAMES,
            stance_boxes,
            0,
            0,
        );
        s.end();
        stream
    });
    let select = region.put(&[ROUTINE_RYUSOKYAKU_VARIANT, 0, 0, 0]);
    for stream in variants {
        region.put_l(stream);
    }
    let stream = region.call(select, 0);
    install(region, RYUSOKYAKU, still(lift, LIFT_FRAMES, lift_boxes), stream);
}

fn write_focus(rom: &[u8], region: &mut Region, ready: u32, attack: u32, recover: u32) {
    let ready_boxes = rom_boxes(rom, ready);
    let mut attack_boxes = rom_boxes(rom, attack);
    let recover_boxes = rom_boxes(rom, recover);
    let ready = rom_pose(rom, ready);
    let attack = rom_pose(rom, attack);
    let recover = rom_pose(rom, recover);

    let hold = region.here();
    let mut stream = Stream::new(region, ready, ready.top_on_ground());
    stream.hold(ready, ready.top_on_ground(), 53, ready_boxes, 0, 0);
    stream.end();
    install(region, FOCUS_CHARGE, still(ready, 1, ready_boxes), hold);

    // Keep the fighter planted. The strike reaches from the fighter's
    // shoulder to the pose's original tip, including a crouching target.
    let origin_x = i32::from(attack.x_offset());
    let center_height = attack.half().1;
    let end_x =
        (origin_x + i32::from(attack_boxes[14] as i8) + i32::from(attack_boxes[15])).max(56);
    let start_x = 5;
    let half_width = (end_x - start_x + 1) / 2;
    attack_boxes[12..16].copy_from_slice(&[
        (39 - center_height) as i8 as u8,
        24,
        (start_x + half_width - origin_x) as i8 as u8,
        half_width as u8,
    ]);
    let hit = hit_entry(rom, region, 0x45c88, 4);
    let strike = region.here();
    let mut stream = Stream::new(region, ready, ready.top_on_ground());
    stream.call(routine::SOUND, 5);
    stream.hold(attack, attack.top_on_ground(), 4, attack_boxes, 0x80, hit);
    stream.hold(recover, recover.top_on_ground(), 16, recover_boxes, 0, 0);
    stream.end();
    install(region, FOCUS, still(ready, 2, ready_boxes), strike);
}

fn write_dash(rom: &[u8], region: &mut Region, fighter: TournamentFighter, id: u8, dx: i32) {
    let table = fighter.move_table();
    let (stance, stance_boxes) = header_pose(rom, table + MOVE_HEADER_SIZE);
    let (step, step_boxes) = header_pose(rom, table + 2 * MOVE_HEADER_SIZE);
    let start = region.here();
    let mut stream = Stream::new(region, stance, stance.top_on_ground());
    if id == BACKDASH {
        stream.travel(step, step.top_on_ground(), 6, (0, dx), step_boxes, 0x08, 0);
        stream.travel(step, step.top_on_ground(), 2, (0, dx), step_boxes, 0, 0);
    } else {
        stream.travel(step, step.top_on_ground(), 8, (0, dx), step_boxes, 0, 0);
    }
    stream.hold(stance, stance.top_on_ground(), 3, stance_boxes, 0, 0);
    stream.end();
    let mut first = still(stance, 1, stance_boxes);
    if id == BACKDASH {
        first.flags = 0x08;
    }
    install(region, id, first, start);
}

fn write_jaguar_avalanche(rom: &[u8], region: &mut Region) {
    const CROUCH: u32 = 0x12dc6;
    const PUNCH: u32 = 0x13392;
    const KNEE: u32 = 0x13896;
    const FLYING_KICK: u32 = 0x135b8;
    let ready = rom_pose(rom, CROUCH);
    let ready_boxes = rom_boxes(rom, CROUCH);
    let knee = rom_pose(rom, KNEE);
    let kick = rom_pose(rom, FLYING_KICK);
    let punch = rom_pose(rom, PUNCH);
    let mut knee_boxes = rom_boxes(rom, KNEE);
    let mut kick_boxes = rom_boxes(rom, FLYING_KICK);
    let mut punch_boxes = rom_boxes(rom, PUNCH);
    knee_boxes[12..16].copy_from_slice(&[6, 24, 22, 29]);
    kick_boxes[12..16].copy_from_slice(&[0xf7, 23, 24, 36]);
    punch_boxes[12..16].copy_from_slice(&[0, 17, 19, 32]);
    let hit = hit_entry(rom, region, 0x45c88, 4);
    let finish = hit_entry(rom, region, 0x45f52, 6);
    let start = region.here();
    let mut stream = Stream::new(region, ready, ready.top_on_ground());
    for (pose, boxes) in [
        (knee, knee_boxes),
        (punch, punch_boxes),
        (knee, knee_boxes),
        (kick, kick_boxes),
    ] {
        stream.travel(ready, ready.top_on_ground(), 3, (0, 5), ready_boxes, 0, 0);
        stream.call(routine::SOUND, 0xa2);
        stream.hold(pose, pose.top_on_ground(), 3, boxes, 0x80, hit);
    }
    stream.travel(ready, ready.top_on_ground(), 2, (0, 4), ready_boxes, 0, 0);
    stream.hold(kick, kick.top_on_ground(), 5, kick_boxes, 0x80, finish);
    stream.hold(ready, ready.top_on_ground(), 12, ready_boxes, 0, 0);
    stream.end();
    install(region, 0x2b, still(ready, 4, ready_boxes), start);
}

pub(super) fn build_what_if_system(
    rom: &[u8],
    region: &mut Region,
    fighter: TournamentFighter,
    [windup, strike, retract]: [u32; 3],
) {
    write_focus(rom, region, windup, strike, retract);
    write_dash(rom, region, fighter, DASH, 4);
    write_dash(rom, region, fighter, BACKDASH, -4);
}
