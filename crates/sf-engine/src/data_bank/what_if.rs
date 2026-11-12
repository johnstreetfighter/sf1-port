//! Native move streams for Retsu, Geki, Joe, Mike and Lee's What-If kits.
use super::*;
use crate::era::EraFamily;
use TournamentFighter::{Geki, Joe, Lee, Mike, Retsu};

const FAMILIES: [EraFamily; 6] = [
    EraFamily::Sf2,
    EraFamily::SfZ,
    EraFamily::Sf3,
    EraFamily::Sf4,
    EraFamily::Sf5,
    EraFamily::Sf6,
];

#[derive(Clone, Copy)]
struct Art {
    windup: u32,
    palm: u32,
    kick: u32,
    finish: u32,
}

fn kit_art(fighter: TournamentFighter) -> Option<Art> {
    Some(match fighter {
        Retsu => Art {
            windup: 0xc9b6,
            palm: 0xc9dc,
            kick: 0xcd42,
            finish: 0xccd6,
        },
        Geki => Art {
            windup: 0xb988,
            palm: 0xb9ae,
            kick: 0xbcca,
            finish: 0xba7e,
        },
        Joe => Art {
            windup: 0xe650,
            palm: 0xe676,
            kick: 0xea48,
            finish: 0xe912,
        },
        Mike => Art {
            windup: 0xd976,
            palm: 0xd99c,
            kick: 0xdb62,
            finish: 0xda8c,
        },
        Lee => Art {
            windup: 0xf5e8,
            palm: 0xf538,
            kick: 0xf60e,
            finish: 0xf6de,
        },
        _ => return None,
    })
}

pub(super) fn build(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    let Some(art) = kit_art(fighter) else { return };
    let canonical = crate::era::families(fighter);
    let families: Vec<_> = FAMILIES
        .into_iter()
        .filter(|family| !canonical.contains(family))
        .collect();
    for &family in &families {
        for slot in SPECIAL_SLOTS {
            alias_era_header(region, family, slot, slot);
        }
    }
    let hits = Hits {
        special: hit_entry(rom, region, 0x45c88, 6),
        hit: hit_entry(rom, region, 0x45c88, 4),
        finish: hit_entry(rom, region, 0x45f52, 5),
    };
    let strike = |region: &mut Region, slot, sequence: &[u32], travel| {
        write_series(
            rom, region, &families, slot, art.windup, sequence, travel, hits, false,
        );
    };
    match fighter {
        Retsu => {
            write_hop(rom, region, &families, 0x19, art, hits.finish, 3);
            write_hop(rom, region, &families, 0x1b, art, hits.special, 6);
            strike(region, 0x1f, &[art.kick, art.finish], 3);
            strike(region, 0x20, &[art.palm, art.palm, art.palm], 2);
            write_catch(rom, region, &families, 0x21, art, 43, 9);
        }
        Geki => {
            strike(region, 0x19, &[art.palm], 7);
            write_air_slash(rom, region, &families, art, hits.finish);
            write_catch(rom, region, &families, 0x20, art, 48, 8);
            write_utsu(rom, region, &families, art, true);
            write_utsu(rom, region, &families, art, false);
            write_shuriken_volley(rom, region, &families, art, 0x24, 3);
        }
        Joe => {
            write_shockwave(rom, region, &families, art);
            write_hop(rom, region, &families, 0x1f, art, hits.finish, 4);
            strike(region, 0x20, &[art.finish], 5);
            strike(region, 0x21, &[art.palm, art.palm, art.kick], 3);
            write_evade(rom, region, &families, 0x22, art.windup, art.palm, 7);
        }
        Mike => {
            strike(region, 0x1b, &[art.kick, art.palm], 4);
            write_evade(rom, region, &families, 0x1f, art.windup, art.kick, 5);
        }
        Lee => {
            write_hop(rom, region, &families, 0x19, art, hits.finish, 3);
            strike(region, 0x1f, &[art.kick, art.kick], 4);
            strike(region, 0x20, &[art.palm], 8);
            strike(region, 0x21, &[art.finish], 3);
        }
        _ => unreachable!(),
    }

    // Each fighter's three finishers share poses with his 1987 attacks.
    // The approach, order of the strikes and closing blow distinguish them.
    let supers = match fighter {
        Retsu => [
            [art.palm, art.palm, art.kick, art.palm, art.finish],
            [art.kick, art.palm, art.kick, art.palm, art.kick],
            [art.kick, art.kick, art.palm, art.palm, art.finish],
        ],
        Geki => [
            [art.palm, art.kick, art.palm, art.palm, art.finish],
            [art.finish, art.palm, art.kick, art.palm, art.kick],
            [art.kick, art.palm, art.palm, art.finish, art.kick],
        ],
        Joe => [
            [art.kick, art.finish, art.palm, art.palm, art.kick],
            [art.palm, art.palm, art.kick, art.palm, art.finish],
            [art.finish, art.kick, art.finish, art.palm, art.kick],
        ],
        Mike => [
            [art.palm, art.kick, art.palm, art.kick, art.finish],
            [art.kick, art.kick, art.palm, art.palm, art.finish],
            [art.palm, art.palm, art.palm, art.kick, art.finish],
        ],
        Lee => [
            [art.palm, art.kick, art.kick, art.palm, art.finish],
            [art.kick, art.kick, art.finish, art.palm, art.kick],
            [art.kick, art.palm, art.palm, art.kick, art.finish],
        ],
        _ => unreachable!(),
    };
    for (index, attacks) in supers.iter().enumerate() {
        let slot = 0x29 + index as u8;
        if fighter == Geki && index == 0 {
            write_shuriken_volley(rom, region, &families, art, slot, 5);
        } else {
            write_series(
                rom,
                region,
                &families,
                slot,
                art.windup,
                attacks,
                4 + index as i32,
                hits,
                true,
            );
        }
    }
}

#[derive(Clone, Copy)]
struct Hits {
    special: u32,
    hit: u32,
    finish: u32,
}

fn hit_entry(rom: &[u8], region: &mut Region, source: u32, damage: u8) -> u32 {
    let mut entry = std::array::from_fn::<_, 8, _>(|i| rom_b(rom, source + i as u32));
    entry[1] = damage;
    region.put(&entry)
}

fn boxes_with_reach(rom: &[u8], address: u32, pose: PoseGeom, reach: u8) -> [u8; 16] {
    let mut boxes = rom_boxes(rom, address);
    let x = pose.x_offset();
    boxes[12..16].copy_from_slice(&[
        (43 - pose.half().1) as i8 as u8,
        26,
        ((reach / 2) as i8 - x) as u8,
        reach / 2,
    ]);
    boxes
}

fn install(
    region: &mut Region,
    families: &[EraFamily],
    slot: u8,
    first: Rec,
    top: i32,
    stream: u32,
) {
    let h = header(&first.bytes(), top, 0xc2, slot, [0, 0, 5], stream);
    for &family in families {
        put_era_header(region, family, slot, &h);
    }
}

#[allow(clippy::too_many_arguments)]
fn write_series(
    rom: &[u8],
    region: &mut Region,
    families: &[EraFamily],
    slot: u8,
    ready: u32,
    attacks: &[u32],
    travel: i32,
    hits: Hits,
    super_move: bool,
) {
    let windup = rom_pose(rom, ready);
    let ready_boxes = rom_boxes(rom, ready);
    let start = region.here();
    let mut stream = Stream::new(region, windup, windup.top_on_ground());
    for (index, &frame) in attacks.iter().enumerate() {
        if travel != 0 {
            stream.travel(
                windup,
                windup.top_on_ground(),
                3,
                (0, travel),
                ready_boxes,
                0,
                0,
            );
        }
        let pose = rom_pose(rom, frame);
        let boxes = boxes_with_reach(rom, frame, pose, if super_move { 90 } else { 68 });
        let hit = if super_move && index + 1 == attacks.len() {
            hits.finish
        } else if super_move {
            hits.hit
        } else {
            hits.special
        };
        stream.call(routine::SOUND, 5);
        stream.hold(pose, pose.top_on_ground(), 4, boxes, 0x80, hit);
    }
    stream.hold(
        windup,
        windup.top_on_ground(),
        if super_move { 16 } else { 12 },
        ready_boxes,
        0,
        0,
    );
    stream.end();
    install(
        region,
        families,
        slot,
        still(windup, if super_move { 4 } else { 5 }, ready_boxes),
        windup.top_on_ground(),
        start,
    );
}

fn write_hop(
    rom: &[u8],
    region: &mut Region,
    families: &[EraFamily],
    slot: u8,
    art: Art,
    hit: u32,
    advance: i32,
) {
    let crouch = rom_pose(rom, art.windup);
    let crouch_boxes = rom_boxes(rom, art.windup);
    let kick = rom_pose(rom, art.kick);
    let strike = boxes_with_reach(rom, art.kick, kick, 73);
    let (origin, offset) = ground_origin_record(region, crouch, kick);
    let start = region.here();
    let mut stream = Stream::new(region, crouch, crouch.top_on_ground());
    stream.change_origin(origin, offset);
    stream.call(routine::RISING, 0);
    stream.strike(kick, 6, (-4, advance), strike, 0x80, hit);
    stream.call(routine::FALLING, 0);
    stream.travel(
        crouch,
        crouch.top_on_ground() - 24,
        6,
        (4, advance / 2),
        crouch_boxes,
        0,
        0,
    );
    stream.call(routine::TOUCHDOWN, 0);
    stream.hold(crouch, crouch.top_on_ground(), 2, crouch_boxes, 0, 0);
    stream.call(routine::LANDING, 0);
    stream.hold(crouch, crouch.top_on_ground(), 11, crouch_boxes, 0, 0);
    stream.end();
    install(
        region,
        families,
        slot,
        still(crouch, 4, crouch_boxes),
        crouch.top_on_ground(),
        start,
    );
}

fn write_air_slash(rom: &[u8], region: &mut Region, families: &[EraFamily], art: Art, hit: u32) {
    let tuck = rom_pose(rom, 0xbca4);
    let tuck_boxes = rom_boxes(rom, 0xbca4);
    let claw = rom_pose(rom, art.kick);
    let land = rom_pose(rom, 0xb214);
    let land_boxes = rom_boxes(rom, 0xb214);
    let (origin, offset) = ground_origin_record(region, tuck, claw);
    let start = region.here();
    let mut stream = Stream::new(region, tuck, tuck.top_on_ground() - 48);
    stream.change_origin(origin, offset);
    stream.call(routine::FALLING, 0);
    stream.strike(
        claw,
        12,
        (4, 5),
        boxes_with_reach(rom, art.kick, claw, 59),
        0x80,
        hit,
    );
    stream.call(routine::TOUCHDOWN, 0);
    stream.hold(land, land.top_on_ground(), 2, land_boxes, 0, 0);
    stream.call(routine::LANDING, 0);
    stream.hold(land, land.top_on_ground(), 10, land_boxes, 0, 0);
    stream.end();
    install(
        region,
        families,
        0x1f,
        still(tuck, 3, tuck_boxes),
        tuck.top_on_ground() - 48,
        start,
    );
}

fn write_utsu(rom: &[u8], region: &mut Region, families: &[EraFamily], art: Art, air: bool) {
    let ready = rom_pose(rom, art.windup);
    let ready_boxes = rom_boxes(rom, art.windup);
    let smoke = rom_pose(rom, 0xb8be);
    let land = rom_pose(rom, 0xb214);
    let land_boxes = rom_boxes(rom, 0xb214);
    let warp = region.put(&[ROUTINE_TELEPORT, 0, 0, 57]);
    let top = ready.top_on_ground() - if air { 36 } else { 0 };
    let smoke_top = smoke.top_on_ground() - if air { 36 } else { 0 };
    let start = region.here();
    let mut stream = Stream::new(region, ready, top);
    stream.call(routine::SOUND, 0x21);
    stream.hold(smoke, smoke_top, 8, [0; 16], 0, 0);
    if air {
        stream.call(warp, 0);
        stream.call(routine::FALLING, 0);
        stream.travel(ready, top, 9, (4, 0), ready_boxes, 0, 0);
        stream.call(routine::TOUCHDOWN, 0);
        stream.hold(land, land.top_on_ground(), 2, land_boxes, 0, 0);
        stream.call(routine::LANDING, 0);
    }
    stream.hold(ready, ready.top_on_ground(), 12, ready_boxes, 0, 0);
    stream.end();
    install(
        region,
        families,
        if air { 0x21 } else { 0x22 },
        still(ready, 4, ready_boxes),
        top,
        start,
    );
}

fn write_catch(
    rom: &[u8],
    region: &mut Region,
    families: &[EraFamily],
    slot: u8,
    art: Art,
    reach: u8,
    damage: u8,
) {
    let ready = rom_pose(rom, art.windup);
    let boxes = rom_boxes(rom, art.windup);
    let routine = region.put(&[ROUTINE_ALPHA_CATCH, 0, 0, 0, reach, damage, 0, 0]);
    let start = region.here();
    let mut stream = Stream::new(region, ready, ready.top_on_ground());
    stream.travel(ready, ready.top_on_ground(), 4, (0, 4), boxes, 0, 0);
    stream.call(routine, 0);
    stream.hold(ready, ready.top_on_ground(), 22, boxes, 0, 0);
    stream.end();
    install(
        region,
        families,
        slot,
        still(ready, 4, boxes),
        ready.top_on_ground(),
        start,
    );
}

fn write_evade(
    rom: &[u8],
    region: &mut Region,
    families: &[EraFamily],
    slot: u8,
    ready: u32,
    duck: u32,
    speed: i32,
) {
    let pose = rom_pose(rom, ready);
    let boxes = rom_boxes(rom, ready);
    let dip = rom_pose(rom, duck);
    let dip_boxes = rom_boxes(rom, duck);
    let start = region.here();
    let mut stream = Stream::new(region, pose, pose.top_on_ground());
    stream.travel(dip, dip.top_on_ground(), 7, (0, speed), dip_boxes, 0x08, 0);
    stream.hold(pose, pose.top_on_ground(), 8, boxes, 0, 0);
    stream.end();
    install(
        region,
        families,
        slot,
        still(pose, 1, boxes),
        pose.top_on_ground(),
        start,
    );
}

fn write_shockwave(rom: &[u8], region: &mut Region, families: &[EraFamily], art: Art) {
    let ready = rom_pose(rom, art.windup);
    let boxes = rom_boxes(rom, art.windup);
    let kick = rom_pose(rom, art.kick);
    let mut spawn_box = boxes_with_reach(rom, art.kick, kick, 44);
    spawn_box[12] = (12 - kick.half().1) as i8 as u8;
    let call = region.put(&[ROUTINE_PROJECTILE, 7, 0, 0]);
    region.put_l(0x1873c);
    let start = region.here();
    let mut stream = Stream::new(region, ready, ready.top_on_ground());
    stream.hold(kick, kick.top_on_ground(), 2, spawn_box, 0, 0);
    stream.call(call, 0);
    stream.hold(kick, kick.top_on_ground(), 8, spawn_box, 0, 0);
    stream.hold(ready, ready.top_on_ground(), 15, boxes, 0, 0);
    stream.end();
    install(
        region,
        families,
        0x1a,
        still(ready, 7, boxes),
        ready.top_on_ground(),
        start,
    );
}

fn write_shuriken_volley(
    rom: &[u8],
    region: &mut Region,
    families: &[EraFamily],
    art: Art,
    slot: u8,
    shots: usize,
) {
    let address = bank_header_offset(0x1a) + 0x2c;
    let mut frame = rom_l(&region.bytes, address).wrapping_sub(region.base);
    let template_record = (0..32).find_map(|_| {
        let kind = region.bytes.get(frame as usize).copied()?;
        if matches!(kind, 0xfd | 0xfe) {
            return None;
        }
        if kind == 0xff {
            let routine = rom_l(&region.bytes, frame + 2).wrapping_sub(region.base);
            frame += 6;
            if region.bytes.get(routine as usize).copied() == Some(ROUTINE_PROJECTILE_BY_STRENGTH) {
                return Some(routine);
            }
        } else {
            frame += 0x20;
        }
        None
    });
    let Some(template_record) = template_record else {
        return;
    };
    let calls = [(4, -1i8), (8, 0), (12, 1)].map(|(offset, dy)| {
        let source = rom_l(&region.bytes, template_record + offset) - region.base;
        let mut template = rom_record(&region.bytes, source);
        let original_stream = rom_l(&template, 0x10);
        let stream = region.here();
        for frame in [original_stream, original_stream + 8] {
            let mut record: [u8; 8] = std::array::from_fn(|i| rom_b(rom, frame + i as u32));
            record[2] = dy as u8;
            region.put(&record);
        }
        region.put(&[0xfe, 0]);
        region.put_l(stream);
        template[0x0a] = dy as u8;
        template[0x0e] |= 0x20;
        template[0x10..0x14].copy_from_slice(&stream.to_be_bytes());
        template[0x1c..0x20].copy_from_slice(&0x43476u32.to_be_bytes());
        let template = region.put(&template);
        let call = region.put(&[ROUTINE_PROJECTILE, 0, 0, 0]);
        region.put_l(template);
        call
    });
    let pose = rom_pose(rom, art.palm);
    let boxes = rom_boxes(rom, art.palm);
    let start = region.here();
    let mut stream = Stream::new(region, pose, pose.top_on_ground());
    for call in calls.into_iter().cycle().take(shots) {
        stream.call(call, 0);
        stream.hold(pose, pose.top_on_ground(), 3, boxes, 0, 0);
    }
    stream.hold(pose, pose.top_on_ground(), 15, boxes, 0, 0);
    stream.end();
    install(
        region,
        families,
        slot,
        still(pose, 5, boxes),
        pose.top_on_ground(),
        start,
    );
}

pub(super) fn alpha_counter_frames(fighter: TournamentFighter) -> Option<(u32, u32, u32)> {
    let art = kit_art(fighter)?;
    Some((art.windup, art.palm, art.windup))
}

pub(super) fn build_systems(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    let art = kit_art(fighter).or_else(|| {
        Some(match fighter {
            TournamentFighter::Gen => Art {
                windup: 0x103b0,
                palm: 0x103d6,
                kick: 0x10970,
                finish: 0x103f6,
            },
            TournamentFighter::Birdie => Art {
                windup: 0x1151c,
                palm: 0x11562,
                kick: 0x115f2,
                finish: 0x115a2,
            },
            TournamentFighter::Eagle => Art {
                windup: 0x12408,
                palm: 0x1242e,
                kick: 0x127c4,
                finish: 0x124ae,
            },
            TournamentFighter::Adon => Art {
                windup: 0x1336c,
                palm: 0x13392,
                kick: 0x135b8,
                finish: 0x133b2,
            },
            TournamentFighter::Sagat => Art {
                windup: 0x14030,
                palm: 0x14056,
                kick: 0x1414c,
                finish: 0x14096,
            },
            _ => return None,
        })
    });
    let Some(art) = art else { return };
    let canonical = crate::era::families(fighter);
    if !canonical.contains(&EraFamily::Sf3) {
        super::sf3::build_what_if_system(rom, region);
    }
    if !canonical.contains(&EraFamily::Sf4) {
        super::sf4::build_what_if_system(rom, region, fighter, [art.windup, art.palm, art.windup]);
    }
    if !canonical.contains(&EraFamily::Sf5) {
        super::sf5::build_what_if_system(rom, region, fighter, art.windup, art.palm, art.kick);
    }
    if !canonical.contains(&EraFamily::Sf6) {
        super::sf6::build_what_if_system(rom, region, fighter, [art.windup, art.palm, art.windup]);
    }
}
