//! What-If move streams for the fighters with existing Alpha animations.

use super::*;
use crate::era_fight::what_if_others::{EAGLE_COUNTER, EAGLE_RIPOSTE, NEXUS, OD_NEXUS};
use TournamentFighter::{Adon, Birdie, Eagle, Gen, Sagat};

const PUNCH_REACTION: u32 = 0x45c88;
const LAUNCH_REACTION: u32 = 0x45f52;
const ACTIVE: u8 = 0x80;

pub(super) fn build(
    rom: &[u8],
    region: &mut Region,
    fighter: TournamentFighter,
    catalogue: &Catalogue,
) -> Result<(), String> {
    match fighter {
        Gen => build_gen(rom, region),
        Birdie => build_birdie(rom, region),
        Eagle => build_eagle(rom, region),
        Adon => build_adon(rom, region),
        Sagat => build_sagat(rom, region, catalogue)?,
        _ => {}
    }
    Ok(())
}

fn aliases(region: &mut Region, family: EraFamily, slots: &[(u8, u8)]) {
    for &(slot, source) in slots {
        alias_era_header(region, family, slot, source);
    }
}

fn build_gen(rom: &[u8], region: &mut Region) {
    use EraFamily::*;

    for family in [Sf2, Sf3, Sf5, Sf6] {
        aliases(
            region,
            family,
            &[
                (0x1a, 0x1a),
                (0x1f, 0x1f),
                (0x20, 0x2d),
                (0x21, 0x2f),
                (0x29, 0x29),
            ],
        );
    }
    for family in [Sf3, Sf6] {
        aliases(region, family, &[(0x2a, 0x2a)]);
    }
    aliases(region, Sf5, &[(0x2b, 0x1a), (0x2c, 0x2d)]);

    // Genei-Jin begins with a brief pose. The timed install and cancels are
    // handled by the fighter state after this stream returns to idle.
    let crossed_frame = 0x10b4e;
    let crossed = rom_pose(rom, crossed_frame);
    let first = still(crossed, 6, rom_boxes(rom, crossed_frame));
    let stream = region.end();
    let data = header(
        &first.bytes(),
        crossed.top_on_ground(),
        0xc2,
        0x2b,
        [0; 3],
        stream,
    );
    put_era_header(region, Sf3, 0x2b, &data);

    let ryukoha = rush(rom, region, 0x2b, &[0x103d6, 0x103d6, 0x103d6, 0x103d6], 5);
    put_era_header(region, Sf6, 0x2b, &ryukoha);
}

fn build_birdie(rom: &[u8], region: &mut Region) {
    use EraFamily::*;

    let hammer = rush(rom, region, 0x20, &[0x11562, 0x11562], 4);
    let spray = rush(rom, region, 0x2b, &[0x114b2, 0x11562, 0x11b8e], 5);
    let skip = rush(rom, region, 0x2b, &[0x11562, 0x114b2, 0x11b8e, 0x11562], 6);
    for family in [Sf2, Sf3, Sf4, Sf6] {
        aliases(
            region,
            family,
            &[(0x19, 0x1a), (0x1a, 0x1b), (0x1b, 0x2c), (0x21, 0x2d)],
        );
        put_era_header(region, family, 0x20, &hammer);
    }
    for family in [Sf2, Sf3, Sf6] {
        aliases(region, family, &[(0x29, 0x29), (0x2a, 0x2a)]);
    }
    put_era_header(region, Sf3, 0x2b, &spray);
    put_era_header(region, Sf6, 0x2b, &skip);
    aliases(region, Sf4, &[(0x2a, 0x29), (0x2b, 0x2a)]);
    let mut sf4_spray = spray;
    sf4_spray[0xa] = 0x29;
    put_era_header(region, Sf4, 0x29, &sf4_spray);
}

fn build_eagle(rom: &[u8], region: &mut Region) {
    use EraFamily::*;

    let albion = rush(rom, region, 0x2b, &[0x1242e, 0x124ae, 0x1242e, 0x1242e], 5);
    let clobbering = rush(rom, region, 0x19, &[0x1242e], 5);
    let riposte = rush(rom, region, EAGLE_RIPOSTE, &[0x1242e], 6);
    let guard_frame = 0x12408;
    let guard = rom_pose(rom, guard_frame);
    let boxes = rom_boxes(rom, guard_frame);
    let recovery = region.here();
    let mut s = Stream::new(region, guard, guard.top_on_ground());
    s.hold(guard, guard.top_on_ground(), 13, boxes, 0, 0);
    s.end();
    let counter = header(
        &still(guard, 12, boxes).bytes(),
        guard.top_on_ground(),
        0xc2,
        EAGLE_COUNTER,
        [0; 3],
        recovery,
    );
    for family in [Sf2, Sf3, Sf4, Sf5, Sf6] {
        aliases(
            region,
            family,
            &[
                (0x1a, 0x1a),
                (0x20, 0x2c),
                (0x21, 0x2d),
                (0x22, 0x2f),
                (0x29, 0x29),
            ],
        );
        put_era_header(region, family, EAGLE_COUNTER, &counter);
        put_era_header(region, family, EAGLE_RIPOSTE, &riposte);
        put_era_header(region, family, 0x19, &clobbering);
    }
    for family in [Sf3, Sf4, Sf6] {
        aliases(region, family, &[(0x2a, 0x2a)]);
        put_era_header(region, family, 0x2b, &albion);
    }
    aliases(region, Sf5, &[(0x2b, 0x2c), (0x2c, 0x1f)]);
}

fn build_adon(rom: &[u8], region: &mut Region) {
    use EraFamily::*;

    let avalanche = rush(rom, region, 0x2b, &[0x13392, 0x13896, 0x13392, 0x135b8], 6);
    for family in [Sf2, Sf3, Sf5, Sf6] {
        aliases(
            region,
            family,
            &[
                (0x19, 0x19),
                (0x1b, 0x1b),
                (0x20, 0x2c),
                (0x21, 0x2d),
                (0x29, 0x29),
            ],
        );
    }
    for family in [Sf3, Sf6] {
        aliases(region, family, &[(0x2a, 0x2a)]);
        put_era_header(region, family, 0x2b, &avalanche);
    }
    aliases(region, Sf5, &[(0x2b, 0x19), (0x2c, 0x2c)]);
}

fn build_sagat(rom: &[u8], region: &mut Region, catalogue: &Catalogue) -> Result<(), String> {
    use EraFamily::*;

    for family in [Sf3, Sf6] {
        aliases(
            region,
            family,
            &[
                (0x19, 0x19),
                (0x1a, 0x1a),
                (0x1b, 0x1b),
                (0x1f, 0x1f),
                (0x2b, 0x2b),
            ],
        );
    }
    aliases(region, Sf3, &[(0x29, 0x29), (0x2a, 0x2a)]);
    aliases(region, Sf6, &[(0x29, 0x2a), (0x2a, 0x29)]);

    build_nexus(rom, region, catalogue)
}

fn build_nexus(rom: &[u8], region: &mut Region, catalogue: &Catalogue) -> Result<(), String> {
    let (ready, ready_boxes) = header_pose(&region.bytes, bank_header_offset(1));
    let mut kick = |name, box_frame, damage, launch, windup| {
        let mut strike = catalogue_pose(catalogue, name, ready.attr & 0xff)?;
        let previous = rom_pose(rom, box_frame);
        // Place the new drawing around the previous frame's collision centre.
        let (half_w, half_h) = strike.half();
        strike.anchor = [
            i16::from(previous.x_offset()) - half_w as i16,
            (previous.top_on_ground() + previous.half().1 - half_h - GROUND) as i16,
        ];
        let mut boxes = rom_boxes(rom, box_frame);
        boxes[12..16].copy_from_slice(&[6, 26, 28, 25]);
        let source = if launch {
            LAUNCH_REACTION
        } else {
            PUNCH_REACTION
        };
        let hit = hit_entry(rom, region, source, damage);
        let stream = region.here();
        let mut s = Stream::new(region, ready, ready.top_on_ground());
        s.travel(
            ready,
            ready.top_on_ground(),
            windup,
            (0, 4),
            ready_boxes,
            0,
            0,
        );
        s.hold(strike, strike.top_on_ground(), 4, boxes, ACTIVE, hit);
        s.hold(ready, ready.top_on_ground(), 14, ready_boxes, 0, 0);
        s.end();
        Ok::<_, String>(stream)
    };
    let mighty = kick("jump_kick_down", 0x1414c, 5, false, 3)?;
    let greedy = kick("tiger_knee_high_kick", 0x14056, 6, true, 4)?;
    let nova = kick("jump_kick", 0x1412c, 6, true, 6)?;
    let select = region.put(&[ROUTINE_SF6_TIGER_NEXUS, 0, 0, 0]);
    for stream in [mighty, greedy, nova] {
        region.put_l(stream);
    }

    let knee = rom_pose(rom, 0x1412c);
    let mut boxes = rom_boxes(rom, 0x1412c);
    boxes[12..16].copy_from_slice(&[0xf8, 24, 25, 25]);
    let hit = hit_entry(rom, region, PUNCH_REACTION, 4);
    for (slot, hits) in [(NEXUS, 1), (OD_NEXUS, 2)] {
        let stream = region.here();
        let mut s = Stream::new(region, ready, ready.top_on_ground());
        s.travel(ready, ready.top_on_ground(), 7, (0, 4), ready_boxes, 0, 0);
        for _ in 0..hits {
            s.hold(knee, knee.top_on_ground(), 3, boxes, ACTIVE, hit);
        }
        s.hold(ready, ready.top_on_ground(), 1, ready_boxes, 0, 0);
        for _ in 0..12 {
            s.call(select, 0);
            s.hold(ready, ready.top_on_ground(), 1, ready_boxes, 0, 0);
        }
        s.hold(ready, ready.top_on_ground(), 10, ready_boxes, 0, 0);
        s.end();
        let first = still(ready, 5, ready_boxes);
        let data = header(
            &first.bytes(),
            ready.top_on_ground(),
            0xc2,
            slot,
            [0, 0, 5],
            stream,
        );
        put_era_header(region, EraFamily::Sf6, slot, &data);
    }
    Ok(())
}

fn hit_entry(rom: &[u8], region: &mut Region, source: u32, damage: u8) -> u32 {
    let mut data: [u8; 8] = std::array::from_fn(|i| rom_b(rom, source + i as u32));
    data[1] = damage;
    region.put(&data)
}

/// A grounded succession of the fighter's original strike poses. Keep the
/// active records separate so each strike can connect once.
fn rush(rom: &[u8], region: &mut Region, slot: u8, strikes: &[u32], damage: u8) -> [u8; 0x30] {
    let (ready, ready_boxes) = header_pose(&region.bytes, bank_header_offset(1));
    let hit = hit_entry(rom, region, PUNCH_REACTION, damage);
    let finish = hit_entry(rom, region, LAUNCH_REACTION, damage + 2);
    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    for (i, &address) in strikes.iter().enumerate() {
        let strike = rom_pose(rom, address);
        let mut boxes = rom_boxes(rom, address);
        boxes[12..16].copy_from_slice(&[8, 24, 25, 25]);
        s.travel(ready, ready.top_on_ground(), 3, (0, 4), ready_boxes, 0, 0);
        let reaction = if i + 1 == strikes.len() { finish } else { hit };
        s.hold(strike, strike.top_on_ground(), 4, boxes, ACTIVE, reaction);
    }
    s.hold(ready, ready.top_on_ground(), 12, ready_boxes, 0, 0);
    s.end();
    let first = still(ready, 5, ready_boxes);
    header(
        &first.bytes(),
        ready.top_on_ground(),
        0xc2,
        slot,
        [0, 0, 5],
        stream,
    )
}
