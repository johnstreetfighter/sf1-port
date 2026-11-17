//! World Super Turbo standing normals for New and Old Sagat.

use super::*;

const CLOSE_LP: u8 = 0x04;
const CLOSE_NEW_MP: u8 = 0x05;
const CLOSE_OLD_MP: u8 = 0x06;
const CLOSE_NEW_HP: u8 = 0x07;
const CLOSE_OLD_HP: u8 = 0x08;
const FAR_OLD_MP: u8 = 0x09;
const FAR_OLD_HP: u8 = 0x0a;
const OLD_MK: u8 = 0x0b;
const OLD_HK: u8 = 0x0c;
const FAR_LP: u8 = 0x0e;
const FAR_MP: u8 = 0x0f;
const FAR_HP: u8 = 0x10;
const LK: u8 = 0x14;
const MK: u8 = 0x15;
const HK: u8 = 0x16;

pub(crate) fn slot(move_id: u8, old: bool, gap_after_target_width: u16) -> Option<u8> {
    match move_id {
        0x0d | FAR_LP => Some(if gap_after_target_width < 42 {
            CLOSE_LP
        } else {
            FAR_LP
        }),
        FAR_MP => Some(if gap_after_target_width < 29 {
            if old {
                CLOSE_OLD_MP
            } else {
                CLOSE_NEW_MP
            }
        } else if old {
            FAR_OLD_MP
        } else {
            FAR_MP
        }),
        FAR_HP => Some(if gap_after_target_width < 53 {
            if old {
                CLOSE_OLD_HP
            } else {
                CLOSE_NEW_HP
            }
        } else if old {
            FAR_OLD_HP
        } else {
            FAR_HP
        }),
        MK if old => Some(OLD_MK),
        HK if old => Some(OLD_HK),
        LK | MK | HK => Some(move_id),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct Strike {
    frame: u32,
    duration: u8,
    damage: u8,
    bounds: [u8; 4],
}

struct Normal<'a> {
    slot: u8,
    move_id: u8,
    startup: u8,
    exit: u8,
    windup: u32,
    sound: u8,
    strikes: &'a [Strike],
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    const JAB: u32 = 0x13ec6;
    const PUNCH: u32 = 0x13f36;
    const PUNCH_WINDUP: u32 = 0x13f10;
    const KICK_WINDUP: u32 = 0x13fa0;
    const LOW_KICK: u32 = 0x13fc6;
    const KNEE: u32 = 0x14056;
    const HIGH_KICK: u32 = 0x14076;
    const STANCE: u32 = 0x13ee6;
    // Representative clean damage from the phase-zero World contact fixtures.
    let close_lp = [Strike {
        frame: JAB,
        duration: 4,
        damage: 11,
        bounds: [60, 63, 37, 7],
    }];
    let far_lp = [Strike {
        frame: JAB,
        duration: 4,
        damage: 11,
        bounds: [69, 80, 35, 12],
    }];
    let close_mp = [Strike {
        frame: PUNCH,
        duration: 6,
        damage: 19,
        bounds: [61, 59, 36, 10],
    }];
    let far_mp = [Strike {
        frame: PUNCH,
        duration: 4,
        damage: 19,
        bounds: [76, 75, 43, 12],
    }];
    let old_far_mp = [Strike {
        damage: 22,
        ..far_mp[0]
    }];
    let close_hp = [Strike {
        frame: PUNCH,
        duration: 8,
        damage: 23,
        bounds: [68, 82, 43, 15],
    }];
    let old_close_hp = [Strike {
        damage: 24,
        ..close_hp[0]
    }];
    let far_hp = [Strike {
        frame: PUNCH,
        duration: 6,
        damage: 21,
        bounds: [86, 78, 53, 9],
    }];
    let old_far_hp = [Strike {
        damage: 27,
        ..far_hp[0]
    }];
    let lk = [
        Strike {
            frame: KNEE,
            duration: 2,
            damage: 11,
            bounds: [44, 23, 21, 21],
        },
        Strike {
            frame: LOW_KICK,
            duration: 2,
            damage: 11,
            bounds: [77, 44, 44, 13],
        },
    ];
    let mk = [
        Strike {
            frame: KNEE,
            duration: 2,
            damage: 12,
            bounds: [38, 58, 27, 15],
        },
        Strike {
            frame: HIGH_KICK,
            duration: 2,
            damage: 16,
            bounds: [80, 78, 49, 12],
        },
    ];
    let old_mk = [
        Strike {
            damage: 11,
            ..mk[0]
        },
        Strike {
            damage: 18,
            ..mk[1]
        },
    ];
    let hk = [
        Strike {
            frame: KNEE,
            duration: 3,
            damage: 14,
            bounds: [39, 58, 28, 17],
        },
        Strike {
            frame: HIGH_KICK,
            duration: 3,
            damage: 20,
            bounds: [96, 99, 31, 17],
        },
    ];
    let old_hk = [
        hk[0],
        Strike {
            damage: 19,
            ..hk[1]
        },
    ];
    for (slot, move_id, startup, exit, windup, sound, strikes) in [
        (CLOSE_LP, 0x0d, 4, 12, STANCE, 0xa1, &close_lp[..]),
        (FAR_LP, FAR_LP, 4, 13, STANCE, 0xa1, &far_lp[..]),
        (
            CLOSE_NEW_MP,
            FAR_MP,
            4,
            17,
            PUNCH_WINDUP,
            0xa1,
            &close_mp[..],
        ),
        (
            CLOSE_OLD_MP,
            FAR_MP,
            5,
            18,
            PUNCH_WINDUP,
            0xa1,
            &close_mp[..],
        ),
        (FAR_MP, FAR_MP, 6, 19, PUNCH_WINDUP, 0xa1, &far_mp[..]),
        (
            FAR_OLD_MP,
            FAR_MP,
            6,
            19,
            PUNCH_WINDUP,
            0xa1,
            &old_far_mp[..],
        ),
        (
            CLOSE_NEW_HP,
            FAR_HP,
            4,
            23,
            PUNCH_WINDUP,
            0xa2,
            &close_hp[..],
        ),
        (
            CLOSE_OLD_HP,
            FAR_HP,
            7,
            26,
            PUNCH_WINDUP,
            0xa2,
            &old_close_hp[..],
        ),
        (FAR_HP, FAR_HP, 8, 37, PUNCH_WINDUP, 0xa2, &far_hp[..]),
        (
            FAR_OLD_HP,
            FAR_HP,
            8,
            37,
            PUNCH_WINDUP,
            0xa2,
            &old_far_hp[..],
        ),
        (LK, LK, 4, 15, KICK_WINDUP, 0xa2, &lk[..]),
        (MK, MK, 4, 21, KICK_WINDUP, 0xa2, &mk[..]),
        (OLD_MK, MK, 4, 21, KICK_WINDUP, 0xa2, &old_mk[..]),
        (HK, HK, 4, 33, KICK_WINDUP, 0xa2, &hk[..]),
        (OLD_HK, HK, 4, 33, KICK_WINDUP, 0xa2, &old_hk[..]),
    ] {
        write_normal(
            rom,
            region,
            Normal {
                slot,
                move_id,
                startup,
                exit,
                windup,
                sound,
                strikes,
            },
        );
    }
}

fn write_normal(rom: &[u8], region: &mut Region, normal: Normal<'_>) {
    let hit_records: Vec<_> = normal
        .strikes
        .iter()
        .enumerate()
        .map(|(index, strike)| {
            let mut entry = [0; 13];
            for (offset, byte) in entry[..8].iter_mut().enumerate() {
                *byte = rom_b(rom, 0x45c88 + offset as u32);
            }
            entry[1] = strike.damage;
            entry[8..12].copy_from_slice(&strike.bounds);
            entry[12] = index as u8 + 1;
            region.put(&entry)
        })
        .collect();

    let windup = rom_pose(rom, normal.windup);
    let mut windup_boxes = rom_boxes(rom, normal.windup);
    windup_boxes[12..16].fill(0);
    let stream = region.here();
    let mut anim = Stream::new(region, windup, windup.top_on_ground());
    anim.call(routine::SOUND, normal.sound);
    for (strike, hit) in normal.strikes.iter().zip(hit_records) {
        let pose = rom_pose(rom, strike.frame);
        let mut boxes = rom_boxes(rom, strike.frame);
        let [x, y, rx, ry] = strike.bounds;
        boxes[12..16].copy_from_slice(&[
            (i32::from(y) - pose.half().1) as u8,
            ry,
            (i32::from(x) - i32::from(pose.x_offset())) as u8,
            rx,
        ]);
        anim.hold(
            pose,
            pose.top_on_ground(),
            strike.duration,
            boxes,
            0x80,
            hit,
        );
    }
    let active: u8 = normal.strikes.iter().map(|strike| strike.duration).sum();
    let recovery = normal.exit - normal.startup - active - 1;
    anim.hold(windup, windup.top_on_ground(), recovery, windup_boxes, 0, 0);
    anim.end();
    let first = still(windup, normal.startup, windup_boxes);
    let h = header(
        &first.bytes(),
        windup.top_on_ground(),
        0xc2,
        normal.move_id,
        [0; 3],
        stream,
    );
    put_era_header(region, EraFamily::Sf2, normal.slot, &h);
}
