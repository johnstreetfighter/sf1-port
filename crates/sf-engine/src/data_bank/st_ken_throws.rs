//! Resident Ken poses for the Super Turbo throw profiles.

use super::{header, rom_pose, still, Region, Stream, ERA_HEADER_END};
use crate::combo::throw::{StRyuThrowFamily, THROW_MOVE};

pub(super) const POINTER_BYTES: usize = 12;

pub(super) const fn header_offset(family: StRyuThrowFamily) -> Option<u32> {
    match family {
        StRyuThrowFamily::HellWheel => Some(ERA_HEADER_END),
        StRyuThrowFamily::KneeBash => Some(ERA_HEADER_END + 4),
        StRyuThrowFamily::KenAir => Some(ERA_HEADER_END + 8),
        _ => None,
    }
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    use StRyuThrowFamily::{HellWheel, KenAir, KneeBash};

    for (family, poses) in [
        // The resident tumble, landing and stand-in recovery drawings.
        (
            HellWheel,
            [
                (0x2f40, 15),
                (0x2f60, 12),
                (0x3fe6, 24),
                (0x2f80, 12),
                (0x2f40, 46),
            ],
        ),
        // Hold, knee lift and retraction. Contact is governed by the throw profile.
        (
            KneeBash,
            [
                (0x2f40, 13),
                (0x2f80, 16),
                (0x2f60, 2),
                (0x2f40, 4),
                (0x2f40, 1),
            ],
        ),
        (
            KenAir,
            [
                (0x2f40, 14),
                (0x2f60, 12),
                (0x3fe6, 10),
                (0x2f80, 9),
                (0x2f40, 39),
            ],
        ),
    ] {
        let first = rom_pose(rom, poses[0].0);
        let boxes = |pose: super::PoseGeom| {
            let below = super::GROUND - (pose.top_on_ground() + pose.half().1);
            [
                0x30,
                below as u8,
                12,
                12,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ]
        };
        let mut entry = still(first, poses[0].1, boxes(first));
        entry.flags = 8;
        let stream = region.here();
        let mut writer = Stream::new(region, first, first.top_on_ground());
        for &(frame, ticks) in &poses[1..] {
            let pose = rom_pose(rom, frame);
            writer.hold(pose, pose.top_on_ground(), ticks, boxes(pose), 8, 0);
        }
        writer.end();
        let address = region.put(&header(
            &entry.bytes(),
            first.top_on_ground(),
            0xc2,
            THROW_MOVE,
            [1, 0, 0],
            stream,
        ));
        region.patch(
            region.base + header_offset(family).unwrap(),
            &address.to_be_bytes(),
        );
    }
}
