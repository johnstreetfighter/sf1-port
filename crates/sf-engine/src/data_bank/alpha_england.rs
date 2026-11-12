//! Birdie and Eagle's Alpha move streams, using their 1987 sprites and
//! already-approved player poses. The Alpha commands are bound separately.

use super::*;

const ACTIVE: u8 = 0x80;
const PUNCH_REACTION: u32 = 0x45c88;
const KNOCKDOWN_REACTION: u32 = 0x45f52;

fn hit_entry(rom: &[u8], region: &mut Region, reaction: u32, damage: u8) -> u32 {
    let mut entry: [u8; 8] = std::array::from_fn(|i| rom_b(rom, reaction + i as u32));
    entry[1] = damage;
    region.put(&entry)
}

fn bind_move(region: &mut Region, id: u8, first: Rec, stream: u32) {
    let h = header(
        &first.bytes(),
        first.pose.top_on_ground(),
        0xc2,
        id,
        [0; 3],
        stream,
    );
    put_header(region, region.base, id, &h);
}

fn catch_record(region: &mut Region, range: u8, damage: u8) -> u32 {
    region.put(&[ROUTINE_ALPHA_CATCH, 0, 0, 0, range, damage, 0, 0])
}

pub(super) fn build_birdie(rom: &[u8], region: &mut Region) {
    write_the_birdie(rom, region);
    write_bull_revenger(rom, region);
    write_chain(rom, region, 0x2c, 0x11562);
    write_chain(rom, region, 0x2d, 0x114b2);
}

fn write_the_birdie(rom: &[u8], region: &mut Region) {
    let (stance, stance_boxes) = header_pose(
        rom,
        TournamentFighter::Birdie.move_table() + MOVE_HEADER_SIZE,
    );
    let windup = rom_pose(rom, 0x11b42);
    let windup_boxes = rom_boxes(rom, 0x11b42);
    let lunge = rom_pose(rom, 0x11b68);
    let lunge_boxes = rom_boxes(rom, 0x11b68);
    let headbutt = rom_pose(rom, 0x11b8e);
    let mut headbutt_boxes = rom_boxes(rom, 0x11b8e);
    headbutt_boxes[4..8].copy_from_slice(&windup_boxes[4..8]);
    // The 1987 Bull Head is airborne. Bring the attack down to meet a
    // crouching defender during this grounded succession of headbutts.
    headbutt_boxes[12..16].copy_from_slice(&[0xf8, 25, 30, 27]);
    let recover = rom_pose(rom, 0x11c14);
    let rush_hit = hit_entry(rom, region, PUNCH_REACTION, 6);
    let finish = hit_entry(rom, region, KNOCKDOWN_REACTION, 8);
    let (headbutt_origin, headbutt_dy) = ground_origin_record(region, lunge, headbutt);
    let stream = region.here();
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    for hit in [rush_hit, rush_hit, finish] {
        s.travel(lunge, lunge.top_on_ground(), 3, (0, 4), lunge_boxes, 0, 0);
        s.change_origin(headbutt_origin, headbutt_dy);
        s.call(routine::SOUND, 0xa1);
        s.strike(headbutt, 5, (0, 6), headbutt_boxes, ACTIVE, hit);
    }
    s.hold(
        recover,
        recover.top_on_ground(),
        7,
        rom_boxes(rom, 0x11c14),
        0,
        0,
    );
    s.hold(stance, stance.top_on_ground(), 13, stance_boxes, 0, 0);
    s.end();
    bind_move(
        region,
        SF2_SUPER_SLOT,
        still(windup, 5, windup_boxes),
        stream,
    );
}

fn write_bull_revenger(rom: &[u8], region: &mut Region) {
    let (stance, stance_boxes) = header_pose(
        rom,
        TournamentFighter::Birdie.move_table() + MOVE_HEADER_SIZE,
    );
    let squat = rom_pose(rom, 0x11072);
    let squat_boxes = rom_boxes(rom, 0x11072);
    let rising = rom_pose(rom, 0x1104c);
    let rising_boxes = rom_boxes(rom, 0x1104c);
    let apex = rom_pose(rom, 0x10fcc);
    let catch = catch_record(region, 60, 22);
    let stream = region.here();
    let mut s = Stream::new(region, squat, squat.top_on_ground());
    s.call(routine::RISING, 0);
    s.travel(
        rising,
        rising.top_on_ground(),
        7,
        (-4, 6),
        rising_boxes,
        0,
        0,
    );
    s.travel(
        apex,
        apex.top_on_ground() - 28,
        6,
        (0, 6),
        rom_boxes(rom, 0x10fcc),
        0,
        0,
    );
    s.call(routine::FALLING, 0);
    s.travel(
        rising,
        rising.top_on_ground() - 28,
        7,
        (4, 6),
        rising_boxes,
        0,
        0,
    );
    // The descent ends on the ground. Its standing state permits a catch;
    // the jump's touchdown routine would switch to a crouching state.
    s.hold(squat, squat.top_on_ground(), 1, squat_boxes, 0, 0);
    s.call(catch, 0);
    s.hold(squat, squat.top_on_ground(), 8, squat_boxes, 0, 0);
    s.hold(stance, stance.top_on_ground(), 16, stance_boxes, 0, 0);
    s.end();
    bind_move(
        region,
        ALPHA_SUPER_SECOND_SLOT,
        still(squat, 5, squat_boxes),
        stream,
    );
}

fn write_chain(rom: &[u8], region: &mut Region, id: u8, reach_frame: u32) {
    let (stance, stance_boxes) = header_pose(
        rom,
        TournamentFighter::Birdie.move_table() + MOVE_HEADER_SIZE,
    );
    let windup = rom_pose(rom, 0x1151c);
    let reach = rom_pose(rom, reach_frame);
    let catch = catch_record(region, 54, 12);
    let stream = region.here();
    // Birdie's ROM hammer and hook stand in for his reaching arm until
    // the chain artwork is available. A successful catch starts his head slam.
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    s.hold(
        reach,
        reach.top_on_ground(),
        3,
        rom_boxes(rom, reach_frame),
        0,
        0,
    );
    s.call(catch, 0);
    s.hold(
        reach,
        reach.top_on_ground(),
        6,
        rom_boxes(rom, reach_frame),
        0,
        0,
    );
    s.hold(stance, stance.top_on_ground(), 16, stance_boxes, 0, 0);
    s.end();
    bind_move(
        region,
        id,
        still(windup, 4, rom_boxes(rom, 0x1151c)),
        stream,
    );
}

pub(super) fn build_eagle(
    rom: &[u8],
    region: &mut Region,
    catalogue: &Catalogue,
) -> Result<(), String> {
    let source = TournamentFighter::Eagle.move_table() + MOVE_HEADER_SIZE;
    let palette = rom_w(rom, source + 2) & 0x3ff;
    let (stance, stance_boxes) = header_pose(rom, source);
    let swing_windup = rom_pose(rom, 0x12408);
    let swing_boxes = rom_boxes(rom, 0x12408);
    let thrust_windup = rom_pose(rom, 0x12648);
    let thrust_windup_boxes = rom_boxes(rom, 0x12648);
    let thrust = rom_pose(rom, 0x123be);
    let thrust_boxes = rom_boxes(rom, 0x123be);
    let rising = catalogue_pose(catalogue, "rising_strike", palette)?;
    let rising_boxes = boxes_named(eagle::BOXES, "rising_strike").bytes(&rising);
    let smash = catalogue_pose(catalogue, "smash", palette)?;
    let smash_boxes = boxes_named(eagle::BOXES, "smash").bytes(&smash);
    let (thrust_origin, thrust_dy) = ground_origin_record(region, thrust_windup, thrust);
    let (smash_origin, smash_dy) = ground_origin_record(region, thrust_windup, smash);
    let poke = hit_entry(rom, region, PUNCH_REACTION, 4);
    let special_finish = hit_entry(rom, region, PUNCH_REACTION, 8);
    let launch = hit_entry(rom, region, KNOCKDOWN_REACTION, 10);

    // St Andrews Green: a rising stick swing from a planted stance.
    let stream = region.here();
    let mut s = Stream::new(region, thrust_windup, thrust_windup.top_on_ground());
    s.call(routine::SOUND, 0xa1);
    s.hold(
        rising,
        rising.top_on_ground(),
        8,
        rising_boxes,
        ACTIVE,
        launch,
    );
    s.hold(stance, stance.top_on_ground(), 17, stance_boxes, 0, 0);
    s.end();
    bind_move(
        region,
        0x1f,
        still(thrust_windup, 6, thrust_windup_boxes),
        stream,
    );

    // Canterbury Blue: a stick twirl into one long-reaching strike.
    let spin = rom_pose(rom, 0x1242e);
    let hold = region.put(&[ROUTINE_ALPHA_HOLD_P, 60, 0, 0, 0, 0, 0, 0]);
    let stream = region.here();
    let mut s = Stream::new(region, swing_windup, swing_windup.top_on_ground());
    s.hold(spin, spin.top_on_ground(), 4, rom_boxes(rom, 0x1242e), 0, 0);
    s.hold(
        thrust_windup,
        thrust_windup.top_on_ground(),
        3,
        thrust_windup_boxes,
        0,
        0,
    );
    let replay = s.hold(
        thrust_windup,
        thrust_windup.top_on_ground(),
        1,
        thrust_windup_boxes,
        0,
        0,
    );
    s.region.patch(hold + 4, &replay.to_be_bytes());
    s.call(hold, 0);
    s.change_origin(thrust_origin, thrust_dy);
    s.call(routine::SOUND, 0xa1);
    s.strike(thrust, 6, (0, 2), thrust_boxes, ACTIVE, special_finish);
    s.hold(stance, stance.top_on_ground(), 17, stance_boxes, 0, 0);
    s.end();
    bind_move(region, 0x2c, still(swing_windup, 5, swing_boxes), stream);

    // Oxford Red and Union Jack Platinum run in with a stick-thrust flurry.
    // Platinum adds two thrusts and the two-stick finisher.
    for (id, thrusts, startup) in [(0x2d, 3, 10), (ALPHA_SUPER_SECOND_SLOT, 5, 5)] {
        let stream = region.here();
        let mut s = Stream::new(region, thrust_windup, thrust_windup.top_on_ground());
        for _ in 0..thrusts {
            s.travel(
                thrust_windup,
                thrust_windup.top_on_ground(),
                3,
                (0, 3),
                thrust_windup_boxes,
                0,
                0,
            );
            s.change_origin(thrust_origin, thrust_dy);
            s.call(routine::SOUND, 0xa1);
            s.strike(thrust, 4, (0, 4), thrust_boxes, ACTIVE, poke);
        }
        if id == ALPHA_SUPER_SECOND_SLOT {
            s.travel(
                thrust_windup,
                thrust_windup.top_on_ground(),
                3,
                (0, 2),
                thrust_windup_boxes,
                0,
                0,
            );
            s.change_origin(smash_origin, smash_dy);
            s.call(routine::SOUND, 0xb0);
            s.strike(smash, 6, (0, 1), smash_boxes, ACTIVE, launch);
        }
        s.hold(stance, stance.top_on_ground(), 18, stance_boxes, 0, 0);
        s.end();
        bind_move(
            region,
            id,
            still(thrust_windup, startup, thrust_windup_boxes),
            stream,
        );
    }

    // Manchester Gold: both sticks spin around Eagle into a rising finisher.
    let stream = region.here();
    let mut s = Stream::new(region, swing_windup, swing_windup.top_on_ground());
    s.call(routine::SOUND, 0xb0);
    for frame in [0x124fe, 0x1253e, 0x1257e, 0x125be] {
        let spin = rom_pose(rom, frame);
        s.travel(
            spin,
            spin.top_on_ground(),
            3,
            (0, 2),
            rom_boxes(rom, frame),
            ACTIVE,
            poke,
        );
    }
    s.hold(
        rising,
        rising.top_on_ground(),
        6,
        rising_boxes,
        ACTIVE,
        launch,
    );
    s.hold(stance, stance.top_on_ground(), 18, stance_boxes, 0, 0);
    s.end();
    bind_move(
        region,
        SF2_SUPER_SLOT,
        still(swing_windup, 5, swing_boxes),
        stream,
    );

    write_liverpool_white(rom, region, stance, stance_boxes, launch);
    Ok(())
}

fn write_liverpool_white(
    rom: &[u8],
    region: &mut Region,
    stance: PoseGeom,
    stance_boxes: [u8; 16],
    hit: u32,
) {
    let squat = rom_pose(rom, 0x11fa0);
    let squat_boxes = rom_boxes(rom, 0x11fa0);
    let rising = rom_pose(rom, 0x11f7a);
    let rising_boxes = rom_boxes(rom, 0x11f7a);
    let swing = rom_pose(rom, 0x12a88);
    let (swing_origin, swing_dy) = ground_origin_record(region, rising, swing);
    let stream = region.here();
    let mut s = Stream::new(region, squat, squat.top_on_ground());
    s.call(routine::RISING, 0);
    s.travel(
        rising,
        rising.top_on_ground(),
        8,
        (-3, 3),
        rising_boxes,
        0,
        0,
    );
    s.change_origin(swing_origin, swing_dy);
    s.call(routine::SOUND, 0xb0);
    s.strike(swing, 8, (2, 3), rom_boxes(rom, 0x12a88), ACTIVE, hit);
    s.call(routine::FALLING, 0);
    s.travel(
        rising,
        rising.top_on_ground() - 8,
        4,
        (2, 2),
        rising_boxes,
        0,
        0,
    );
    s.call(routine::TOUCHDOWN, 0);
    s.hold(squat, squat.top_on_ground(), 5, squat_boxes, 0, 0);
    s.hold(stance, stance.top_on_ground(), 16, stance_boxes, 0, 0);
    s.end();
    bind_move(region, 0x2f, still(squat, 6, squat_boxes), stream);
}
