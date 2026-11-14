//! World Super Turbo crouching normals for New and Old Sagat.

use super::*;

const CROUCH_LP: u8 = 0x11;
const CROUCH_MP: u8 = 0x1c;
const CROUCH_KICK: u8 = 0x17;
const SWEEP: u8 = 0x18;
const LP_SLOT: u8 = 0x37;
const LK_SLOT: u8 = 0x38;
const MP_SLOT: u8 = 0x39;
const MK_SLOT: u8 = 0x3a;
const HP_SLOT: u8 = 0x3b;
const HK_SLOT: u8 = 0x3c;

pub(crate) fn slot(move_id: u8, strength: u8) -> Option<u8> {
    match (move_id, strength) {
        (0x11..=0x13, 1) => Some(LP_SLOT),
        (0x1c..=0x1e, 2) => Some(MP_SLOT),
        (0x1c..=0x1e, 3) => Some(HP_SLOT),
        (CROUCH_KICK, 1) => Some(LK_SLOT),
        (CROUCH_KICK, 2) => Some(MK_SLOT),
        (SWEEP, 3) => Some(HK_SLOT),
        _ => None,
    }
}

/// Facing-relative center X, height above the floor and horizontal/vertical radii.
pub(crate) fn attack_box(move_id: u8, strength: u8) -> Option<(i32, i32, u8, u8)> {
    match slot(move_id, strength)? {
        LP_SLOT => Some((57, 44, 29, 11)),
        LK_SLOT => Some((52, 10, 33, 10)),
        MP_SLOT => Some((62, 44, 34, 11)),
        MK_SLOT => Some((62, 10, 41, 10)),
        HP_SLOT => Some((65, 44, 40, 11)),
        HK_SLOT => Some((67, 10, 53, 10)),
        _ => None,
    }
}

/// Fixed clean damage uses the measured phase-zero standing Ken sample.
pub(crate) fn clean_hp(move_id: u8, strength: u8, old: bool) -> Option<u16> {
    match (slot(move_id, strength)?, old) {
        (LP_SLOT, _) | (LK_SLOT, true) => Some(11),
        (LK_SLOT, false) => Some(4),
        (MP_SLOT | MK_SLOT, _) => Some(19),
        (HP_SLOT | HK_SLOT, false) => Some(18),
        (HP_SLOT, true) => Some(26),
        (HK_SLOT, true) => Some(27),
        _ => None,
    }
}

pub(super) fn build(rom: &[u8], region: &mut Region, catalogue: &Catalogue) -> Result<(), String> {
    let palette = rom_w(rom, TournamentFighter::Sagat.move_table() + 0x32) & 0xff;
    let pose = |name| {
        let pose = catalogue_pose(catalogue, name, palette)?;
        let mut boxes = boxes_named(&sagat::BOXES, name).bytes(&pose);
        boxes[12..16].fill(0);
        Ok::<_, String>((pose, boxes))
    };
    let (crouch, crouch_boxes) = pose("crouch")?;
    let (punch, punch_boxes) = pose("crouching_punch")?;
    let (chamber, chamber_boxes) = pose("crouching_kick_chamber")?;
    let (kick, kick_boxes) = pose("crouching_kick_2")?;
    let (sweep, sweep_boxes) = pose("sweep_1")?;
    let (sweep_recover, sweep_recover_boxes) = pose("sweep_2")?;
    let (sweep_finish, sweep_finish_boxes) = pose("sweep_3")?;

    // The last tick is the end record that returns to crouch.
    for (move_id, strength, startup, active, exit, sound, strike, mut strike_boxes) in [
        (CROUCH_LP, 1, 5, 3, 12, 0xa1, punch, punch_boxes),
        (CROUCH_KICK, 1, 5, 4, 14, 0xa2, kick, kick_boxes),
        (CROUCH_MP, 2, 5, 4, 15, 0xa1, punch, punch_boxes),
        (CROUCH_KICK, 2, 5, 4, 16, 0xa2, kick, kick_boxes),
        (CROUCH_MP, 3, 7, 6, 26, 0xa1, punch, punch_boxes),
        (SWEEP, 3, 7, 6, 30, 0xa2, sweep, sweep_boxes),
    ] {
        let hit_entry = if move_id == SWEEP {
            hit::SWEEP
        } else {
            0x45c88
        };
        let mut entry: [u8; 8] =
            std::array::from_fn(|offset| rom_b(rom, hit_entry + offset as u32));
        entry[1] = clean_hp(move_id, strength, false).expect("Sagat crouching normal damage") as u8;
        let hit = region.put(&entry);
        let (x, y, rx, ry) = attack_box(move_id, strength).expect("Sagat crouching attack box");
        strike_boxes[12..16].copy_from_slice(&[
            (y - strike.half().1) as u8,
            ry,
            (x - i32::from(strike.x_offset())) as u8,
            rx,
        ]);
        let (windup, windup_boxes) = if move_id == CROUCH_KICK {
            (chamber, chamber_boxes)
        } else {
            (crouch, crouch_boxes)
        };
        let stream = region.here();
        let mut anim = Stream::new(region, crouch, crouch.top_on_ground());
        if move_id == SWEEP {
            anim.hold(crouch, crouch.top_on_ground(), 4, crouch_boxes, 0, 0);
            anim.hold(chamber, chamber.top_on_ground(), 2, chamber_boxes, 0, 0);
        } else {
            anim.hold(
                windup,
                windup.top_on_ground(),
                startup - 1,
                windup_boxes,
                0,
                0,
            );
        }
        anim.call(routine::SOUND, sound);
        anim.hold(
            strike,
            strike.top_on_ground(),
            active,
            strike_boxes,
            0x80,
            hit,
        );
        if move_id == SWEEP {
            anim.hold(
                sweep_recover,
                sweep_recover.top_on_ground(),
                8,
                sweep_recover_boxes,
                0,
                0,
            );
            anim.hold(
                sweep_finish,
                sweep_finish.top_on_ground(),
                8,
                sweep_finish_boxes,
                0,
                0,
            );
        } else {
            anim.hold(
                windup,
                windup.top_on_ground(),
                exit - startup - active - 1,
                windup_boxes,
                0,
                0,
            );
        }
        anim.end();
        let first = still(crouch, 1, crouch_boxes);
        let header = header(
            &first.bytes(),
            crouch.top_on_ground(),
            0xc4,
            move_id,
            [0; 3],
            stream,
        );
        let bank_slot = slot(move_id, strength).expect("Sagat crouching normal slot");
        put_era_header(region, EraFamily::Sf2, bank_slot, &header);
    }
    Ok(())
}
