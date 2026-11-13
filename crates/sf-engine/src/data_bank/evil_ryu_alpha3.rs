//! ROM-derived animations for the private Alpha 3 air shoulder throw.

use super::{
    catalogue_pose, header, header_pose, put_era_header, rom_pose, still, PoseGeom, Region, Stream,
    EVIL_RYU_ALPHA3_AIR_THROWER_HEADER, EVIL_RYU_ALPHA3_AIR_VICTIM_HEADER, MOVE_HEADER_SIZE,
    ROUTINE_AIR_THROW_LAND, ROUTINE_THROW_ARC, ROUTINE_THROW_GET_UP, ROUTINE_THROW_SLAM,
};
use crate::combo::throw::{THROWN_MOVE, THROW_MOVE};
use crate::era::EraFamily;
use crate::roster::TournamentFighter;
use sf_video::art::Catalogue;

const GRAB_FRAMES: u8 = 8;
const AIR_PHASE_FRAMES: u8 = 4;
const PROTECTED: u8 = 0x08;

pub(super) fn write_air_shoulder_throw(rom: &[u8], region: &mut Region, sagat: Option<&Catalogue>) {
    let land = region.put(&[ROUTINE_AIR_THROW_LAND, 0]);
    write_air_thrower(rom, region, land);
    write_air_victim(rom, region, land, sagat);
}

fn throw_boxes(pose: PoseGeom) -> [u8; 16] {
    let below = pose.half().1 as u8;
    [0x30, below, 0x0c, 0x0c, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
}

fn write_air_thrower(rom: &[u8], region: &mut Region, land: u32) {
    let grab = rom_pose(rom, 0x01382); // two-hand reach
    let turn = rom_pose(rom, 0x00ec8); // turns his shoulder into the grab
    let release = rom_pose(rom, 0x015a4); // downward air punch
    let fall = rom_pose(rom, 0x0166c);
    let crouch = rom_pose(rom, 0x00d5e);
    let (stance, _) = header_pose(rom, TournamentFighter::Ryu.move_table() + MOVE_HEADER_SIZE);

    let stream = region.here();
    let mut s = Stream::new(region, grab, grab.top_on_ground());
    for pose in [turn, release, fall] {
        s.hold(
            pose,
            pose.top_on_ground(),
            AIR_PHASE_FRAMES,
            throw_boxes(pose),
            PROTECTED,
            0,
        );
    }
    s.call(land, 0);
    s.hold(
        crouch,
        crouch.top_on_ground(),
        8,
        throw_boxes(crouch),
        PROTECTED,
        0,
    );
    s.hold(
        stance,
        stance.top_on_ground(),
        4,
        throw_boxes(stance),
        PROTECTED,
        0,
    );
    s.end();

    let mut first = still(grab, GRAB_FRAMES, throw_boxes(grab));
    first.flags = PROTECTED;
    let h = header(
        &first.bytes(),
        grab.top_on_ground(),
        0xc1,
        THROW_MOVE,
        [1, 0, 0],
        stream,
    );
    put_era_header(
        region,
        EraFamily::SfZ,
        EVIL_RYU_ALPHA3_AIR_THROWER_HEADER,
        &h,
    );
}

fn write_air_victim(rom: &[u8], region: &mut Region, land: u32, sagat: Option<&Catalogue>) {
    let grabbed = rom_pose(rom, 0x0a4dc);
    let fall = rom_pose(rom, 0x0ab80);
    let lying = rom_pose(rom, 0x0abc6);
    let rising = rom_pose(rom, 0x14242);
    let stance = rom_pose(rom, 0x14218);
    let rotated = |name| {
        let mut pose = sagat
            .and_then(|catalogue| catalogue_pose(catalogue, name, grabbed.attr & 0x3ff).ok())
            .unwrap_or(fall);
        // The same foot line across both flips lets the move inherit the
        // victim's height at the grab instead of the ground throw's lift.
        pose.anchor[1] = -(pose.height() as i16);
        pose
    };
    let lift = rotated("thrown_1");
    let over = rotated("thrown_2");

    let arc = region.put(&[ROUTINE_THROW_ARC, 0]);
    let slam = region.put(&[ROUTINE_THROW_SLAM, 0]);
    let get_up = region.put(&[ROUTINE_THROW_GET_UP, 0]);
    let stream = region.here();
    let mut s = Stream::new(region, grabbed, grabbed.top_on_ground());
    s.call(arc, 0);
    s.travel(
        lift,
        lift.top_on_ground(),
        AIR_PHASE_FRAMES,
        (-2, -2),
        throw_boxes(lift),
        PROTECTED,
        0,
    );
    s.travel(
        over,
        over.top_on_ground() - 8,
        AIR_PHASE_FRAMES,
        (2, -2),
        throw_boxes(over),
        PROTECTED,
        0,
    );
    s.travel(
        fall,
        fall.top_on_ground(),
        AIR_PHASE_FRAMES,
        (0, -2),
        throw_boxes(fall),
        PROTECTED,
        0,
    );
    s.call(land, 0);
    s.call(slam, 0);
    s.hold(
        lying,
        lying.top_on_ground(),
        18,
        throw_boxes(lying),
        PROTECTED,
        0,
    );
    s.call(get_up, 0);
    s.hold(
        rising,
        rising.top_on_ground(),
        10,
        throw_boxes(rising),
        PROTECTED,
        0,
    );
    s.hold(
        stance,
        stance.top_on_ground(),
        6,
        throw_boxes(stance),
        PROTECTED,
        0,
    );
    s.end();

    let mut first = still(grabbed, GRAB_FRAMES, throw_boxes(grabbed));
    first.flags = PROTECTED;
    let h = header(
        &first.bytes(),
        grabbed.top_on_ground(),
        0x89,
        THROWN_MOVE,
        [0; 3],
        stream,
    );
    put_era_header(
        region,
        EraFamily::SfZ,
        EVIL_RYU_ALPHA3_AIR_VICTIM_HEADER,
        &h,
    );
}
