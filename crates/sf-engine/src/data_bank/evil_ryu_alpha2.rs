//! ROM streams used by the private Alpha 2 Evil Ryu matchup.

use super::{
    bind_era_header, header, header_pose, put_era_header, rom_boxes, rom_pose, routine, still,
    write_throw_pair, Region, Stream, EVIL_RYU_ALPHA2_FAKE_HADOKEN, EVIL_RYU_ALPHA2_SENPUKYAKU,
    MOVE_HEADER_SIZE,
};
use crate::combo::throw::{THROWN_MOVE, THROW_MOVE};
use crate::era::EraFamily;
use crate::roster::TournamentFighter;

pub(super) fn build(rom: &[u8], region: &mut Region) {
    const STANDING_HIT: usize = 0x45c88;
    let Some(source) = rom.get(STANDING_HIT..STANDING_HIT + 8) else {
        return;
    };
    let mut hit: [u8; 8] = source.try_into().unwrap();
    hit[1] = 4;
    let hit = region.put(&hit);
    write_senpukyaku(rom, region, hit);
    write_fake_hadoken(rom, region);
    write_kick_throw(rom, region);
}

fn write_senpukyaku(rom: &[u8], region: &mut Region, hit: u32) {
    const CHAMBER: u32 = 0x00e82;
    const TURN: u32 = 0x014d6;
    const KICK: u32 = 0x014b0;
    const RETRACT: u32 = 0x014f6;
    let [chamber, turn, kick, retract] = [CHAMBER, TURN, KICK, RETRACT].map(|at| rom_pose(rom, at));
    let [chamber_boxes, turn_boxes, mut kick_boxes, retract_boxes] =
        [CHAMBER, TURN, KICK, RETRACT].map(|at| rom_boxes(rom, at));
    let (stance, stance_boxes) =
        header_pose(rom, TournamentFighter::Ryu.move_table() + MOVE_HEADER_SIZE);
    // The leg extends to the front of this short hop. Use one active record
    // and a mid-level hit box, so crouching guard still works.
    kick_boxes[12..16].copy_from_slice(&[6, 14, 34, 18]);
    let stream = region.here();
    let mut s = Stream::new(region, chamber, chamber.top_on_ground());
    s.call(routine::RISING, 0);
    s.travel(turn, turn.top_on_ground(), 4, (-3, 3), turn_boxes, 0, 0);
    s.call(routine::SOUND, 0xa2);
    s.strike(kick, 5, (0, 2), kick_boxes, 0x80, hit);
    s.call(routine::FALLING, 0);
    s.travel(
        retract,
        retract.top_on_ground() - 12,
        4,
        (3, 1),
        retract_boxes,
        0,
        0,
    );
    s.call(routine::TOUCHDOWN, 0);
    s.hold(stance, stance.top_on_ground(), 7, stance_boxes, 0, 0);
    s.end();

    let first = still(chamber, 6, chamber_boxes);
    let slot = EVIL_RYU_ALPHA2_SENPUKYAKU;
    let h = header(
        &first.bytes(),
        chamber.top_on_ground(),
        0xc2,
        slot,
        [0; 3],
        stream,
    );
    put_era_header(region, EraFamily::SfZ, slot, &h);
}

fn write_fake_hadoken(rom: &[u8], region: &mut Region) {
    const GATHER: u32 = 0x01362;
    const THRUST: u32 = 0x01382;
    let table = TournamentFighter::Ryu.move_table();
    let (windup, windup_boxes) = header_pose(rom, table + 0x1a * MOVE_HEADER_SIZE);
    let (stance, stance_boxes) = header_pose(rom, table + MOVE_HEADER_SIZE);
    let gather = rom_pose(rom, GATHER);
    let gather_boxes = rom_boxes(rom, GATHER);
    let thrust = rom_pose(rom, THRUST);
    let stream = region.here();
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    s.hold(gather, gather.top_on_ground(), 4, gather_boxes, 0, 0);
    s.hold(
        thrust,
        thrust.top_on_ground(),
        5,
        rom_boxes(rom, THRUST),
        0,
        0,
    );
    s.hold(gather, gather.top_on_ground(), 4, gather_boxes, 0, 0);
    s.hold(stance, stance.top_on_ground(), 3, stance_boxes, 0, 0);
    s.end();

    let first = still(windup, 6, windup_boxes);
    let slot = EVIL_RYU_ALPHA2_FAKE_HADOKEN;
    let h = header(
        &first.bytes(),
        windup.top_on_ground(),
        0xc2,
        slot,
        [0; 3],
        stream,
    );
    put_era_header(region, EraFamily::SfZ, slot, &h);
}

fn write_kick_throw(rom: &[u8], region: &mut Region) {
    let table = TournamentFighter::Ryu.move_table();
    let (stance, _) = header_pose(rom, table + MOVE_HEADER_SIZE);
    let grab = rom_pose(rom, 0x01382);
    let recoil = rom_pose(rom, 0x01ce8);
    let mut roll = rom_pose(rom, 0x0218c);
    // Centre the lying pose. Its ROM offset throws a KO'd fighter backward.
    roll.anchor[0] = -(roll.half().0 as i16);
    let press = rom_pose(rom, 0x01a12);
    let sagat_grabbed = rom_pose(rom, 0x0a4dc);
    let mut sagat_lift = rom_pose(rom, 0x0ab80);
    sagat_lift.anchor[1] -= 16;
    let mut sagat_over = sagat_lift;
    sagat_over.anchor[1] -= 20;
    let sagat_lying = rom_pose(rom, 0x0abc6);
    let sagat_rising = rom_pose(rom, 0x14242);
    let sagat_stance = rom_pose(rom, 0x14218);
    // The grab folds into a backward roll and low leg press. The native throw
    // runner then carries Sagat over Ryu and handles the landing and tech.
    let pair = write_throw_pair(
        region,
        &[
            (grab, 7),
            (recoil, 4),
            (roll, 3),
            (press, 7),
            (recoil, 6),
            (stance, 7),
        ],
        [
            (sagat_grabbed, 11),
            (sagat_lift, 8),
            (sagat_over, 8),
            (sagat_lying, 18),
        ],
        &[(sagat_rising, 10), (sagat_stance, 6)],
        0,
    );
    bind_era_header(region, EraFamily::SfZ, THROW_MOVE, pair.throw_header);
    bind_era_header(region, EraFamily::SfZ, THROWN_MOVE, pair.thrown_header);
}
