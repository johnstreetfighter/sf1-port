use super::{ground_origin_record, header, put_era_header, rom_b, rom_boxes, rom_pose, rom_record, routine, still, Region, Stream};
use crate::era::EraFamily;

pub(crate) const COLLARBONE_BREAKER: u8 = 0x2a;
pub(crate) const SOLAR_PLEXUS_SMASHER: u8 = 0x2b;
pub(crate) const CLOSE_LK: u8 = 0x2c;
pub(crate) const FAR_LK: u8 = 0x14;
pub(crate) const OLD_FAR_LK_HEADER: u8 = 0x0e;
pub(crate) const CLOSE_LP_HEADER: u8 = 0x0c;
pub(crate) const CLOSE_MK: u8 = 0x2d;
pub(crate) const FAR_MK: u8 = 0x15;
pub(crate) const OLD_FAR_MK_HEADER: u8 = 0x0b;
pub(crate) const FAR_MP: u8 = 0x0f;
pub(crate) const FAR_HP: u8 = 0x10;
pub(crate) const FAR_HK: u8 = 0x16;
pub(crate) const CLOSE_MP_HEADER: u8 = 0x09;
pub(crate) const CLOSE_HP_HEADER: u8 = 0x0a;
pub(crate) const CLOSE_HK: u8 = 0x2f;
pub(crate) const CROUCH_LP_HEADER: u8 = 0x11;
pub(crate) const CROUCH_MP_HEADER: u8 = 0x1c;
pub(crate) const CROUCH_HP_HEADER: u8 = 0x1d;
pub(crate) const SWEEP_HEADER: u8 = 0x18;

pub(super) fn build(rom: &[u8], region: &mut Region) {
    let windup = rom_pose(rom, 0x0af0);
    let finish = rom_pose(rom, 0x0b50);
    let windup_boxes = rom_boxes(rom, 0x0af0);
    let finish_boxes = rom_boxes(rom, 0x0b50);
    let hit = |region: &mut Region, damage| {
        let mut entry = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, 0x45c88 + i as u32));
        entry[1] = damage;
        region.put(&entry)
    };
    let main_hit = hit(region, 7);
    let followup = hit(region, 2);

    // World 940223's audit confirms the moves and travel. The separately captured
    // ST hitbox diagrams provide the phase lengths for these port-authored poses.
    // https://doctorguile.github.io/ssf2st/st/ryu/#Overhead
    let chop = rom_pose(rom, 0x1200);
    let punch = rom_pose(rom, 0x0b30);
    let mut chop_boxes = rom_boxes(rom, 0x1200);
    chop_boxes[12..16].copy_from_slice(&[8, 24, 34, 30]);
    let mut punch_boxes = rom_boxes(rom, 0x0b30);
    punch_boxes[12..16].copy_from_slice(&[8, 26, 34, 30]);
    let (to_chop, chop_shift) = ground_origin_record(region, windup, chop);
    let (to_punch, punch_shift) = ground_origin_record(region, chop, punch);
    let stream = region.here();
    let mut anim = Stream::new(region, windup, windup.top_on_ground());
    anim.travel(windup, windup.top_on_ground(), 7, (0, 1), windup_boxes, 0, 0);
    anim.change_origin(to_chop, chop_shift);
    anim.call(routine::SOUND, 0xa2);
    anim.strike(chop, 2, (0, 3), chop_boxes, 0x80, main_hit);
    anim.change_origin(to_punch, punch_shift);
    anim.strike(punch, 9, (0, 1), punch_boxes, 0x80, followup);
    anim.hold(finish, finish.top_on_ground(), 4, finish_boxes, 0, 0);
    anim.end();
    let first = still(windup, 10, windup_boxes);
    let h = header(&first.bytes(), windup.top_on_ground(), 0xc2, COLLARBONE_BREAKER, [0; 3], stream);
    put_era_header(region, EraFamily::Sf2, COLLARBONE_BREAKER, &h);

    // https://doctorguile.github.io/ssf2st/st/ryu/#Rush%20Punch
    let lead = rom_pose(rom, 0x0a8e);
    let thrust = rom_pose(rom, 0x13a8);
    let mut lead_boxes = rom_boxes(rom, 0x0a8e);
    lead_boxes[12..16].copy_from_slice(&[4, 20, 48, 32]);
    let mut thrust_boxes = rom_boxes(rom, 0x13a8);
    thrust_boxes[12..16].copy_from_slice(&[4, 20, 38, 30]);
    let (to_lead, lead_shift) = ground_origin_record(region, windup, lead);
    let (to_thrust, thrust_shift) = ground_origin_record(region, lead, thrust);
    let stream = region.here();
    let mut anim = Stream::new(region, windup, windup.top_on_ground());
    anim.change_origin(to_lead, lead_shift);
    anim.call(routine::SOUND, 0xa2);
    anim.strike(lead, 1, (0, 0), lead_boxes, 0x80, main_hit);
    anim.change_origin(to_thrust, thrust_shift);
    anim.strike(thrust, 8, (0, 6), thrust_boxes, 0x80, followup);
    anim.travel(finish, finish.top_on_ground(), 6, (0, 2), finish_boxes, 0, 0);
    anim.end();
    let first = still(windup, 11, windup_boxes);
    let h = header(&first.bytes(), windup.top_on_ground(), 0xc2, SOLAR_PLEXUS_SMASHER, [0; 3], stream);
    put_era_header(region, EraFamily::Sf2, SOLAR_PLEXUS_SMASHER, &h);

    build_close_lp(rom, region);
    build_standing_kicks(rom, region);
    build_standing_mp_hp_hk(rom, region);
    build_crouching_normals(rom, region);
}

fn build_close_lp(rom: &[u8], region: &mut Region) {
    const STANDING_LP: u8 = 0x0d;
    let source = crate::roster::TournamentFighter::Ryu.move_table() + u32::from(STANDING_LP) * super::MOVE_HEADER_SIZE;
    let mut jab = std::array::from_fn(|i| rom_b(rom, source + i as u32));
    let mut active = rom_record(rom, 0x09ca);
    let mut recovery = rom_record(rom, 0x09ea);
    jab[0x0e] = 2;
    active[1] = 3;
    recovery[1] = 4;
    let stream = region.put(&active);
    region.put(&recovery);
    region.end();
    jab[0x2c..0x30].copy_from_slice(&stream.to_be_bytes());
    put_era_header(region, EraFamily::Sf2, CLOSE_LP_HEADER, &jab);
}

fn build_standing_kicks(rom: &[u8], region: &mut Region) {
    let ready = rom_pose(rom, 0x0e82);
    let ready_boxes = rom_boxes(rom, 0x0e82);
    let close = rom_pose(rom, 0x0e00);
    let far = rom_pose(rom, 0x0ea2);
    let mut close_boxes = rom_boxes(rom, 0x0e00);
    close_boxes[12..16].copy_from_slice(&[0xec, 13, 33, 28]);
    let mut far_boxes = rom_boxes(rom, 0x0ea2);
    // The far kick passes above crouching Ken in the World 940223 guard probe.
    far_boxes[12..16].copy_from_slice(&[29, 8, 32, 26]);
    // World 940223: both loadouts land the close kick against standing guard.
    // The far kick uses the original standing-kick hit handler and damage.
    let mut close_entry = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, 0x460a8 + i as u32));
    close_entry[1] = 4;
    let close_hit = region.put(&close_entry);
    let mut close_mk_boxes = rom_boxes(rom, 0x0e00);
    close_mk_boxes[12..16].copy_from_slice(&[0xf8, 21, 32, 30]);
    let mut far_mk_boxes = rom_boxes(rom, 0x0ea2);
    far_mk_boxes[12..16].copy_from_slice(&[0xf8, 20, 38, 30]);
    let mut mk_entry = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, 0x45c88 + i as u32));
    mk_entry[1] = 7;
    let mk_hit = region.put(&mk_entry);
    // World Turbo 2 whiff pose writes and contacts are in ryu-st-standing-proximity.md.
    // For the MK rows, the final hold reproduces the first neutral game tick.
    for (slot, id, state, pose, boxes, hit, [startup, active, recovery]) in [
        (CLOSE_LK, CLOSE_LK, 0xc4, close, close_boxes, close_hit, [6, 6, 4]),
        (FAR_LK, FAR_LK, 0xc2, far, far_boxes, 0x4602a, [5, 8, 5]),
        (OLD_FAR_LK_HEADER, FAR_LK, 0xc2, far, far_boxes, 0x4602a, [7, 8, 5]),
        (CLOSE_MK, CLOSE_MK, 0xc2, close, close_mk_boxes, mk_hit, [4, 6, 9]),
        (FAR_MK, FAR_MK, 0xc2, far, far_mk_boxes, mk_hit, [8, 8, 6]),
        (OLD_FAR_MK_HEADER, FAR_MK, 0xc2, far, far_mk_boxes, mk_hit, [12, 12, 6]),
    ] {
        let stream = region.here();
        let mut anim = Stream::new(region, ready, ready.top_on_ground());
        anim.call(routine::SOUND, 0xa2);
        anim.hold(pose, pose.top_on_ground(), active, boxes, 0x80, hit);
        anim.hold(ready, ready.top_on_ground(), recovery, ready_boxes, 0, 0);
        anim.end();
        let first = still(ready, startup, ready_boxes);
        let h = header(&first.bytes(), ready.top_on_ground(), state, id, [0; 3], stream);
        put_era_header(region, EraFamily::Sf2, slot, &h);
    }
}

type StandingPhase = (u32, u8, Option<([u8; 4], u32)>);

fn build_crouching_normals(rom: &[u8], region: &mut Region) {
    // Source rectangles are relative to the fighter's axis and the floor.
    // The existing SF1 crouch drawings supply the visuals for each phase.
    const PUNCH: u32 = 0x45d26;
    const SWEEP: u32 = 0x46300;
    let jab: &[StandingPhase] = &[
        (0x0bf2, 1, None), (0x0b92, 2, None),
        (0x0bb2, 4, Some(([39, 46, 31, 7], PUNCH))),
        (0x0bd2, 4, None), (0x0bf2, 1, None),
    ];
    let medium: &[StandingPhase] = &[
        (0x0d3e, 2, None), (0x0d5e, 2, None),
        (0x0d9e, 4, Some(([44, 42, 36, 11], PUNCH))),
        (0x0dbe, 3, None), (0x0d5e, 3, None), (0x0dde, 1, None),
    ];
    let heavy: &[StandingPhase] = &[
        (0x0d3e, 4, None), (0x0d9e, 3, Some(([37, 68, 28, 20], PUNCH))),
        (0x1200, 8, Some(([34, 85, 21, 30], PUNCH))),
        (0x0d9e, 10, None), (0x0dbe, 12, None), (0x0dde, 1, None),
    ];
    let sweep: &[StandingPhase] = &[
        (0x1158, 4, None), (0x1178, 6, Some(([58, 9, 43, 12], SWEEP))),
        (0x119e, 6, None), (0x11be, 8, None), (0x1158, 10, None), (0x11de, 1, None),
    ];
    for (slot, move_id, phases) in [
        (CROUCH_LP_HEADER, 0x11, jab), (CROUCH_MP_HEADER, 0x1c, medium),
        (CROUCH_HP_HEADER, 0x1c, heavy), (SWEEP_HEADER, 0x18, sweep),
    ] {
        build_normal(rom, region, slot, move_id, 0xc4, phases);
    }
}

fn build_normal(rom: &[u8], region: &mut Region, slot: u8, move_id: u8, state: u8, phases: &[StandingPhase]) {
    let (first_frame, startup, _) = phases[0];
    let ready = rom_pose(rom, first_frame);
    let ready_boxes = rom_boxes(rom, first_frame);
    let stream = region.here();
    let mut anim = Stream::new(region, ready, ready.top_on_ground());
    for &(frame, duration, attack) in &phases[1..] {
        let pose = rom_pose(rom, frame);
        let mut boxes = rom_boxes(rom, frame);
        let (flags, hit) = if let Some(([x, y, width, height], hit)) = attack {
            let x = (i32::from(x) - i32::from(pose.x_offset())) as u8;
            let y = (i32::from(y) - pose.half().1) as u8;
            boxes[12..16].copy_from_slice(&[y, height, x, width]);
            (0x80, hit)
        } else {
            (0, 0)
        };
        anim.hold(pose, pose.top_on_ground(), duration, boxes, flags, hit);
    }
    anim.end();
    let first = still(ready, startup, ready_boxes);
    let h = header(&first.bytes(), ready.top_on_ground(), state, move_id, [0x82, 0, 0], stream);
    put_era_header(region, EraFamily::Sf2, slot, &h);
}

fn build_standing_mp_hp_hk(rom: &[u8], region: &mut Region) {
    // World 940223 keeps standing Ken grounded on these contacts. Reuse
    // Ryu's ground recoil entry; the heavy strikes carry heavy damage.
    let mut entry = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, 0x45c88 + i as u32));
    let medium = region.put(&entry);
    entry[1] = 8;
    let heavy = region.put(&entry);
    let mut followup = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, 0x460a8 + i as u32));
    followup[1] = 2;
    let second_hk = region.put(&followup);

    // Each phase uses an SF1 drawing. Active rectangles are measured from
    // the World 940223 fighter axis and floor, then placed on that drawing.
    // Array order is forward offset, height above floor, X radius, Y radius.
    let close_mp: &[StandingPhase] = &[
        (0x0a6e, 2, None), (0x0af0, 2, None), (0x0a8e, 2, Some(([40, 71, 26, 13], medium))),
        (0x0aae, 6, None), (0x0b50, 4, None), (0x0a6e, 3, None), (0x0ace, 4, None),
    ];
    let far_mp: &[StandingPhase] = &[
        (0x0a6e, 2, None), (0x0af0, 2, None), (0x0a8e, 4, Some(([50, 72, 34, 7], medium))),
        (0x0aae, 3, None), (0x0a6e, 3, None), (0x0ace, 1, None),
    ];
    let close_hp: &[StandingPhase] = &[
        (0x0af0, 4, None), (0x0b30, 2, Some(([43, 71, 31, 13], heavy))),
        (0x0a8e, 6, Some(([44, 88, 36, 20], heavy))),
        (0x0b50, 10, None), (0x0b10, 12, None), (0x0ace, 1, None),
    ];
    let far_hp: &[StandingPhase] = &[
        (0x0af0, 4, None), (0x0b10, 2, None), (0x0b30, 6, Some(([54, 72, 38, 7], heavy))),
        (0x0b50, 10, None), (0x0b10, 12, None), (0x0ace, 1, None),
    ];
    let close_hk: &[StandingPhase] = &[
        (0x0fac, 4, None), (0x0fcc, 4, None), (0x0fec, 8, Some(([43, 87, 36, 33], heavy))),
        (0x0ea2, 4, Some(([62, 67, 43, 18], second_hk))), (0x0e82, 10, None), (0x0ace, 1, None),
    ];
    let far_hk: &[StandingPhase] = &[
        (0x0fac, 3, None), (0x0fec, 4, Some(([34, 82, 20, 16], heavy))),
        (0x0ea2, 8, Some(([51, 84, 55, 12], heavy))),
        (0x0e82, 10, None), (0x0fcc, 6, None), (0x0ace, 1, None),
    ];
    for (slot, move_id, phases) in [
        (CLOSE_MP_HEADER, FAR_MP, close_mp), (FAR_MP, FAR_MP, far_mp),
        (CLOSE_HP_HEADER, FAR_HP, close_hp), (FAR_HP, FAR_HP, far_hp),
        (CLOSE_HK, CLOSE_HK, close_hk), (FAR_HK, FAR_HK, far_hk),
    ] {
        build_normal(rom, region, slot, move_id, 0xc2, phases);
    }
}
