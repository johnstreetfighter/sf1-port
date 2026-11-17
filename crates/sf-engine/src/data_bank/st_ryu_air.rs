use super::{
    ground_origin_record, header, header_pose, rom_b, rom_boxes, rom_pose, rom_record, routine,
    still, Region, Stream, MOVE_HEADER_SIZE, ROUTINE_AIRBORNE_REACTION_LAND, ST_RYU_AIR_HEADERS,
    ST_RYU_JUMP_MP_REACTION, ST_RYU_JUMP_MP_STREAMS,
};
use crate::era_fight::sf3::AIR_TATSU;
use crate::roster::TournamentFighter;

const CHAMBER: u32 = 0x13ea;
const STRIKE: u32 = 0x14b0;
const TURN: u32 = 0x14d6;
const FALL: u32 = 0x14f6;
const ACTIVE: u8 = 0x80;
const REFERENCE_HEIGHT: i32 = 56;

#[derive(Clone, Copy)]
enum SpinPose {
    Front,
    Rear,
    Turn,
}

use SpinPose::{Front, Rear, Turn};

// World 940223 Turbo 2. Phase lengths count advancing game ticks.
// See docs/research/ryu-st-air-combat.md for the source traces.
const STRENGTHS: [(&[(u8, SpinPose)], u8, u8); 3] = [
    (&[(3, Front), (3, Turn), (3, Rear), (2, Turn)], 11, 42),
    (
        &[
            (3, Front),
            (3, Turn),
            (3, Rear),
            (3, Turn),
            (3, Front),
            (3, Turn),
            (3, Rear),
            (2, Turn),
        ],
        23,
        51,
    ),
    (
        &[
            (3, Front),
            (2, Turn),
            (3, Rear),
            (2, Turn),
            (3, Front),
            (2, Turn),
            (3, Rear),
            (2, Turn),
            (3, Front),
            (2, Turn),
            (3, Rear),
            (1, Turn),
        ],
        29,
        54,
    ),
];

#[derive(Clone, Copy)]
enum Jump {
    Neutral,
    Forward,
    Back,
}

const JUMPS: [Jump; 3] = [Jump::Neutral, Jump::Forward, Jump::Back];
pub(super) const HEADERS: usize = STRENGTHS.len() * JUMPS.len() * 2;
pub(super) const JUMP_MP_STREAMS: usize = 4;

pub(super) fn jump_mp_pointer(old: bool, backward: bool) -> u32 {
    ST_RYU_JUMP_MP_STREAMS + (u32::from(old) * 2 + u32::from(backward)) * 4
}

pub(super) fn header_offset(old: bool, jump_move: u8, strength: u32) -> u32 {
    // The existing connected and whiff cancel gate uses the forward jump punch.
    let direction = match jump_move {
        9 => Jump::Neutral,
        0x0b => Jump::Back,
        _ => Jump::Forward,
    };
    pointer_offset(old, direction, strength)
}

fn pointer_offset(old: bool, direction: Jump, strength: u32) -> u32 {
    let index = (u32::from(old) * JUMPS.len() as u32 + direction as u32) * STRENGTHS.len() as u32
        + strength;
    ST_RYU_AIR_HEADERS + index * 4
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    build_jump_mp(rom, region);
    build_jump_mp_reaction(rom, region);
    let chamber = rom_pose(rom, CHAMBER);
    let strike = rom_pose(rom, STRIKE);
    let turn = rom_pose(rom, TURN);
    let fall = rom_pose(rom, FALL);
    let (stand, stand_boxes) =
        header_pose(rom, TournamentFighter::Ryu.move_table() + MOVE_HEADER_SIZE);
    let mut hit: [u8; 8] = std::array::from_fn(|offset| rom_b(rom, 0x45c88 + offset as u32));
    hit[1] = 5;
    let hit = region.put(&hit);
    let launch = ground_origin_record(region, chamber, strike);
    let start_turn = ground_origin_record(region, strike, turn);
    let end_turn = ground_origin_record(region, turn, strike);

    for (old, direction) in [false, true]
        .into_iter()
        .flat_map(|old| JUMPS.map(|jump| (old, jump)))
    {
        for (strength, &(phases, recovery, forward_landing)) in STRENGTHS.iter().enumerate() {
            let landing = landing_tick(old, direction, strength, forward_landing);
            let landing_recovery = if !old && matches!(direction, Jump::Back) && strength == 1 {
                6
            } else {
                5
            };
            let top = chamber.top_on_ground() - REFERENCE_HEIGHT;
            let stream = region.here();
            let mut animation = Stream::new(region, chamber, top);
            let mut tick = 0;
            for &(frames, phase) in phases {
                let (pose, boxes, flags, entry, (origin, offset)) = match phase {
                    Front | Rear => {
                        let mut boxes = rom_boxes(rom, STRIKE);
                        let (x, radius) = if old { (35, 34) } else { (54, 15) };
                        let x = if matches!(phase, Front) { x } else { -x };
                        boxes[12..16].copy_from_slice(&[
                            (67 - strike.half().1) as u8,
                            16,
                            (x - strike.x_offset()) as u8,
                            radius,
                        ]);
                        (
                            strike,
                            boxes,
                            ACTIVE,
                            hit,
                            if tick == 0 { launch } else { end_turn },
                        )
                    }
                    Turn => (turn, rom_boxes(rom, TURN), 0, 0, start_turn),
                };
                animation.change_origin(origin, offset);
                let step = flight_step(direction, strength, recovery, tick, frames);
                animation.strike(pose, frames, step, boxes, flags, entry);
                tick += frames;
            }

            animation.call(routine::FALLING, 0);
            let fall_boxes = rom_boxes(rom, FALL);
            // Give the arcade's accelerating descent an integer-pixel path.
            let mut next = tick + 1;
            let fall_top = animation.top + fall.top_on_ground() - turn.top_on_ground();
            animation.travel(
                fall,
                fall_top,
                1,
                flight_step(direction, strength, recovery, tick, 1),
                fall_boxes,
                0,
                0,
            );
            while next < landing {
                let frames = (landing - next).min(3);
                let step = flight_step(direction, strength, recovery, next, frames);
                animation.strike(fall, frames, step, fall_boxes, 0, 0);
                next += frames;
            }

            animation.call(routine::LANDING, 0);
            animation.hold(
                fall,
                fall.top_on_ground(),
                landing_recovery - 1,
                fall_boxes,
                0,
                0,
            );
            animation.hold(stand, stand.top_on_ground(), 1, stand_boxes, 0, 0);
            animation.end();
            let first = still(chamber, 1, rom_boxes(rom, CHAMBER));
            let header = header(
                &first.bytes(),
                top,
                0xc1,
                AIR_TATSU,
                [0x8b, 0, 0x19],
                stream,
            );
            let address = region.put(&header);
            region.patch(
                region.base + pointer_offset(old, direction, strength as u32),
                &address.to_be_bytes(),
            );
        }
    }
}

fn build_jump_mp(rom: &[u8], region: &mut Region) {
    let jump_hit = |region: &mut Region, damage| {
        let mut entry =
            std::array::from_fn::<_, 8, _>(|offset| rom_b(rom, 0x45eda + offset as u32));
        entry[1] = damage;
        region.put(&entry)
    };
    let new_first = jump_hit(region, 7);
    let new_second = jump_hit(region, 1);
    let old_hit = jump_hit(region, 5);
    let windup = rom_pose(rom, 0x166c);
    let strike = rom_pose(rom, 0x168c);
    let retract = rom_pose(rom, 0x16ac);
    let fall = rom_pose(rom, 0x16cc);

    for old in [false, true] {
        for backward in [false, true] {
            let direction = if backward { -1 } else { 1 };
            let stream = region.here();
            let mut s = Stream::new(region, windup, windup.top_on_ground());
            s.strike(windup, 4, (0, 2 * direction), rom_boxes(rom, 0x166c), 0, 0);
            let mut first = rom_boxes(rom, 0x168c);
            // CPS2 measures Y from its fighter axis. Keep the first strike at
            // the SF1 donor's height and use the measured 16 px rise for hit two.
            let reach = if old { 30 } else { 43 };
            first[12..16].copy_from_slice(&[10, 11, (reach - strike.x_offset()) as u8, 31]);
            s.strike(
                strike,
                if old { 20 } else { 5 },
                (3, 2 * direction),
                first,
                ACTIVE,
                if old { old_hit } else { new_first },
            );
            if !old {
                let mut second = rom_boxes(rom, 0x168c);
                second[12..16].copy_from_slice(&[26, 17, (29 - strike.x_offset()) as u8, 15]);
                s.strike(strike, 4, (-3, 2 * direction), second, ACTIVE, new_second);
            }
            s.strike(
                retract,
                if old { 5 } else { 8 },
                (4, 2 * direction),
                rom_boxes(rom, 0x16ac),
                0,
                0,
            );
            s.strike(
                fall,
                if old { 3 } else { 6 },
                (4, 2 * direction),
                rom_boxes(rom, 0x16cc),
                0,
                0,
            );
            s.call(routine::LANDING, 0);
            region.put(&rom_record(rom, 0x16f2));
            region.put(&rom_record(rom, 0x1712));
            region.end();
            region.patch(
                region.base + jump_mp_pointer(old, backward),
                &stream.to_be_bytes(),
            );
        }
    }
}

fn build_jump_mp_reaction(rom: &[u8], region: &mut Region) {
    let mut pose = rom_record(rom, 0x3fe6);
    pose[0] = 0;
    pose[1] = 0;
    pose[0x0c..0x0e].copy_from_slice(&[16, 16]);
    let geom = rom_pose(rom, 0x3fe6);
    let landing = region.call(routine::LANDING, 0);
    for frame in [0x400c, 0x402c, 0x404c] {
        region.put(&rom_record(rom, frame));
    }
    region.end();
    let floor = geom.top_on_ground() + geom.half().1;
    let [floor_hi, floor_lo] = (floor as u16).to_be_bytes();
    let floor_check = region.put(&[ROUTINE_AIRBORNE_REACTION_LAND, 0, floor_hi, floor_lo]);
    region.put(&landing.to_be_bytes());

    // Apply the observed integer trajectory from the second contact's
    // uninterrupted knockback. The airborne launch poses use rectangles
    // relative to Ken's facing; world X signs reverse when Ken faces right.
    const Y_STEPS: [i8; 48] = [
        6, 6, 5, 5, 4, 4, 4, 4, 4, 3, 3, 2, 3, 2, 2, 1, 1, 1, 1, 0, 0, 0, 0, -1, -1, -1, -2, -2,
        -2, -2, -3, -3, -3, -4, -4, -4, -4, -5, -5, -5, -6, -6, -6, -6, -7, -7, -7, -7,
    ];
    let stream = region.here();
    for (tick, up) in std::iter::once(20).chain(Y_STEPS).enumerate() {
        let (x, y, w, h) = if tick < 8 {
            (-19, 39, 23, 26)
        } else {
            (8, 16, 50, 15)
        };
        pose[0x12..0x16].copy_from_slice(&[
            (y - geom.half().1) as u8,
            h,
            (x - geom.x_offset()) as u8,
            w,
        ]);
        pose[6] = (-up) as u8;
        pose[7] = match tick {
            0 => 0,
            1 | 2 => (-3i8) as u8,
            _ => (-2i8) as u8,
        };
        if up < 0 {
            region.call(floor_check, 0);
        }
        region.put(&pose);
    }
    // Higher contacts can leave the defender in the air after this trace.
    // Continue falling, checking the landing threshold each tick.
    let fall = region.call(floor_check, 0);
    pose[6] = 8;
    region.put(&pose);
    region.put(&[0xfe, 0]);
    region.put(&fall.to_be_bytes());
    region.patch(region.base + ST_RYU_JUMP_MP_REACTION, &stream.to_be_bytes());
}

fn landing_tick(old: bool, direction: Jump, strength: usize, forward: u8) -> u8 {
    // First floor write in each World Turbo 2 directional fixture.
    match (old, direction) {
        (_, Jump::Forward) => forward,
        (false, Jump::Neutral) => [43, 52, 55][strength],
        (false, Jump::Back) => [44, 52, 56][strength],
        (true, Jump::Neutral) => [42, 50, 54][strength],
        (true, Jump::Back) => [43, 51, 55][strength],
    }
}

// The port's jump streams do not expose an arcade launch velocity. Use the
// 190-pixel forward fixture's initial velocity and the measured deceleration.
// The engine rebases this integer-step stream to the current jump height.
fn flight_step(direction: Jump, strength: usize, recovery: u8, tick: u8, frames: u8) -> (i32, i32) {
    // Velocity is in 1/512 px. Averaging over each animation record yields
    // one whole-pixel step without breaking up an active strike record.
    let midpoint_twice = 2 * i32::from(tick) + i32::from(frames) - 1;
    let spin = midpoint_twice.min(2 * i32::from(recovery));
    let strength_step = strength as i32 * 256;
    let dx = match direction {
        Jump::Neutral => 512 + strength_step,
        Jump::Forward => 2184 + strength_step - 5 * midpoint_twice,
        Jump::Back => -2440 - strength_step + 5 * midpoint_twice,
    };
    let dy = 1760 - 32 * spin - 80 * (midpoint_twice - spin);
    let nearest_pixel = |step: i32| (step + 256).div_euclid(512);
    (-nearest_pixel(dy), nearest_pixel(dx))
}
