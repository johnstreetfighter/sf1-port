//! Ticket #167 (`AP3`): every pose in Ryu's catalogue draws exactly as it
//! should when the engine's own pose-grid code places it and the real
//! renderer draws it, facing right and mirrored facing left. ROM poses must
//! draw as their ROM grid; the rebuilt pose, from the bank, as its recipe
//! built it. Local ROMs only; each test skips without them.

use std::path::Path;

use sf_engine::fighter::pose_grid_entries;
use sf_video::art::{load_art, rom_only, Pens, Source, TRANSPARENT};
use sf_video::palette::Palette;
use sf_video::render::{render_frame, VISAREA_X, VISAREA_Y, WIDTH};
use sf_video::romset::load_assets;
use sf_video::scene::SceneState;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const RYU: &str = include_str!("../../../art/recipes/ryu.json");

#[test]
fn every_catalogue_pose_draws_through_the_engines_grid_in_both_facings() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let art = load_art(&rom, &[&rom_only(RYU).unwrap()]).unwrap();
    let gfx = rom.with_sprite_bank(&art.bank);
    let ryu = art.catalogue("ryu").unwrap();
    // Sprite palette 0: pen p a distinct grey; the background another.
    let mut paletteram = vec![0u8; 2048];
    for p in 0..16u16 {
        let v = (p << 8) | (p << 4) | p;
        paletteram[2 * (512 + p as usize)..2 * (512 + p as usize) + 2].copy_from_slice(&v.to_be_bytes());
    }
    paletteram[0..2].copy_from_slice(&0x0f00u16.to_be_bytes());
    let pal = Palette::from_ram(&paletteram);
    let (left, top) = (40usize, 40usize);
    let mut drawn = 0;
    for pose in &ryu.poses {
        let pens = match pose.source {
            Source::Rom => Pens::from_grid(&rom.gfx3, &pose.grid),
            Source::Recipe | Source::Pack => art.built.iter().find(|(_, n, _)| *n == pose.name).unwrap().2.clone(),
        };
        let attr = if pose.grid.large { 0x400 } else { 0 };
        for flipped in [false, true] {
            let entries = pose_grid_entries(
                pose.grid.code,
                attr,
                pose.grid.rows,
                pose.grid.cols,
                (left + VISAREA_X) as u16,
                (top + VISAREA_Y) as u16,
                flipped,
            );
            assert_eq!(entries.len(), pose.grid.entries());
            let mut objectram = vec![0u8; 0x2000];
            for e in 0..128 {
                objectram[e * 0x40 + 4..e * 0x40 + 6].copy_from_slice(&0x0300u16.to_be_bytes());
            }
            for (e, words) in entries.iter().enumerate() {
                for (i, w) in words.iter().enumerate() {
                    objectram[e * 0x40 + 2 * i..e * 0x40 + 2 * i + 2].copy_from_slice(&w.to_be_bytes());
                }
            }
            let scene = SceneState {
                videoram: vec![0; 0x1000],
                objectram,
                paletteram: paletteram.clone(),
                bg_scroll: 0,
                fg_scroll: 0,
                active: 0x80,
                flip_screen: false,
                bg_page: 0,
                fg_page: 0,
            };
            let frame = render_frame(&gfx, &scene);
            let want = if flipped { pens.mirrored() } else { pens.clone() };
            for y in 0..want.height {
                for x in 0..want.width {
                    let pen = want.at(x, y);
                    let expect = if pen == TRANSPARENT { pal.argb(0) } else { pal.argb(512 + pen as usize) };
                    let got = frame.argb[(top + y) * WIDTH + left + x];
                    assert_eq!(got, expect, "{} {} at ({x}, {y})", pose.name, if flipped { "facing left" } else { "facing right" });
                }
            }
            drawn += 1;
        }
    }
    assert_eq!(drawn, 2 * ryu.poses.len());
}
