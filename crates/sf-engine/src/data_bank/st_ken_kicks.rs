//! World 940223 Super Turbo New Ken ground kick streams.

use super::*;
use crate::era_fight::st_ken_kicks::{KAMA_BARAI, NATA_OTOSHI, OSOTO_MAWASHI};

#[derive(Clone, Copy)]
struct Strike {
    bounds: [u8; 4],
    damage: u8,
}

struct Kick {
    id: u8,
    first: u8,
    branch: u8,
    second: u8,
    last: u8,
    step: (u8, i32),
    exit: u8,
    heel_first: u8,
    heel_exit: u8,
    strikes: [Strike; 4],
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    for kick in [
        Kick {
            id: KAMA_BARAI,
            first: 7,
            branch: 9,
            second: 10,
            last: 15,
            step: (29, 42),
            exit: 36,
            heel_first: 27,
            heel_exit: 40,
            strikes: [
                Strike {
                    bounds: [66, 79, 15, 14],
                    damage: 7,
                },
                Strike {
                    bounds: [96, 82, 22, 13],
                    damage: 27,
                },
                Strike {
                    bounds: [105, 78, 14, 16],
                    damage: 21,
                },
                Strike {
                    bounds: [89, 47, 30, 16],
                    damage: 21,
                },
            ],
        },
        Kick {
            id: NATA_OTOSHI,
            first: 16,
            branch: 18,
            second: 19,
            last: 22,
            step: (30, 32),
            exit: 35,
            heel_first: 36,
            heel_exit: 49,
            strikes: [
                Strike {
                    bounds: [62, 70, 14, 16],
                    damage: 2,
                },
                Strike {
                    bounds: [80, 79, 15, 24],
                    damage: 26,
                },
                Strike {
                    bounds: [105, 78, 14, 16],
                    damage: 23,
                },
                Strike {
                    bounds: [89, 47, 30, 16],
                    damage: 23,
                },
            ],
        },
        Kick {
            id: OSOTO_MAWASHI,
            first: 12,
            branch: 14,
            second: 15,
            last: 19,
            step: (27, 32),
            exit: 34,
            heel_first: 34,
            heel_exit: 47,
            strikes: [
                Strike {
                    bounds: [58, 74, 15, 11],
                    damage: 27,
                },
                Strike {
                    bounds: [93, 78, 19, 14],
                    damage: 27,
                },
                Strike {
                    bounds: [118, 78, 14, 16],
                    damage: 21,
                },
                Strike {
                    bounds: [102, 47, 30, 16],
                    damage: 21,
                },
            ],
        },
    ] {
        write_kick(rom, region, kick);
    }
}

fn write_kick(rom: &[u8], region: &mut Region, kick: Kick) {
    // These are Ken's resident SF1 kick poses. The boxes and clocks below
    // come from ST. The original ST sprites are absent from this art bank.
    const CHAMBER: u32 = 0x2f60;
    const HIGH_KICK: u32 = 0x2f80;
    const EXTENDED_KICK: u32 = 0x30ca;
    let chamber = rom_pose(rom, CHAMBER);
    let high = rom_pose(rom, HIGH_KICK);
    let extended = rom_pose(rom, EXTENDED_KICK);
    let mut idle_boxes = rom_boxes(rom, CHAMBER);
    idle_boxes[12..16].fill(0);

    let damage_entry = |region: &mut Region, damage, phase, chip, overhead| {
        let mut entry = [0; 11];
        for (i, byte) in entry[..8].iter_mut().enumerate() {
            *byte = rom_b(rom, 0x45c88 + i as u32);
        }
        entry[1] = damage;
        entry[8] = phase;
        entry[9] = chip;
        entry[10] = u8::from(overhead);
        region.put(&entry)
    };
    let strike_boxes = |pose: PoseGeom, frame, strike: Strike| {
        let mut boxes = rom_boxes(rom, frame);
        let [x, y, rx, ry] = strike.bounds;
        boxes[12..16].copy_from_slice(&[
            (i32::from(y) - pose.half().1) as u8,
            ry,
            (i32::from(x) - i32::from(pose.x_offset())) as u8,
            rx,
        ]);
        boxes
    };
    let osoto = kick.id == OSOTO_MAWASHI;
    let opening = damage_entry(
        region,
        kick.strikes[0].damage,
        1,
        if osoto { 7 } else { 1 },
        false,
    );
    let extended_hit = damage_entry(region, kick.strikes[1].damage, 2, 6, false);
    let heel_high = damage_entry(region, kick.strikes[2].damage, 2, 5, false);
    let heel_low = damage_entry(region, kick.strikes[3].damage, 2, 5, true);
    let opening_boxes = strike_boxes(high, HIGH_KICK, kick.strikes[0]);
    let heel_step = region.put(&[ROUTINE_ST_KEN_KICK_STEP, if osoto { 26 } else { 22 }]);
    let sample = region.put(&[ROUTINE_ST_KEN_KICK_BRANCH, 0]);

    let heel = region.here();
    let mut stream = Stream::new(region, high, high.top_on_ground());
    stream.hold(
        chamber,
        chamber.top_on_ground(),
        kick.heel_first - kick.branch,
        idle_boxes,
        0,
        0,
    );
    stream.hold(
        high,
        high.top_on_ground(),
        3,
        strike_boxes(high, HIGH_KICK, kick.strikes[2]),
        0x80,
        heel_high,
    );
    stream.hold(
        extended,
        extended.top_on_ground(),
        3,
        strike_boxes(extended, EXTENDED_KICK, kick.strikes[3]),
        0x80,
        heel_low,
    );
    stream.hold(
        chamber,
        chamber.top_on_ground(),
        kick.heel_exit - kick.heel_first - 7,
        idle_boxes,
        0,
        0,
    );
    // The step and terminal record share the source return-to-neutral tick.
    region.call(heel_step, 0);
    region.end();

    let switch = region.put(&[ROUTINE_ST_KEN_KICK_BRANCH, 1, 0, 0]);
    region.put_l(heel);
    let start = region.here();
    let mut stream = Stream::new(region, chamber, chamber.top_on_ground());
    stream.call(sample, 0);
    stream.call(routine::SOUND, 0x82);
    stream.hold(
        high,
        high.top_on_ground(),
        kick.branch - kick.first,
        opening_boxes,
        0x80,
        opening,
    );
    stream.call(switch, 0);
    stream.hold(
        high,
        high.top_on_ground(),
        kick.second - kick.branch,
        opening_boxes,
        0x80,
        opening,
    );
    stream.hold(
        extended,
        extended.top_on_ground(),
        kick.last - kick.second,
        strike_boxes(extended, EXTENDED_KICK, kick.strikes[1]),
        0x80,
        extended_hit,
    );
    stream.hold(
        chamber,
        chamber.top_on_ground(),
        kick.step.0 - kick.last,
        idle_boxes,
        0,
        0,
    );
    stream.travel(
        chamber,
        chamber.top_on_ground(),
        1,
        (0, kick.step.1),
        idle_boxes,
        0,
        0,
    );
    stream.hold(
        chamber,
        chamber.top_on_ground(),
        kick.exit - kick.step.0 - 2,
        idle_boxes,
        0,
        0,
    );
    stream.end();
    let initial = still(chamber, kick.first, idle_boxes);
    put_era_header(
        region,
        EraFamily::Sf2,
        kick.id,
        &header(
            &initial.bytes(),
            chamber.top_on_ground(),
            0xc4,
            kick.id,
            [0; 3],
            start,
        ),
    );
}
