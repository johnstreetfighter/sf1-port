//! Ticket FE5 (#155): the VS-screen bust decoder refuses damaged tile data
//! one fighter at a time.
//!
//! `decode_busts` reads the tile ROM, the gfx1 tiles and the palette tables
//! in the program ROM. Each test here damages one fighter's part of the tile
//! set (blank cells, out-of-range codes, zeroed tile graphics, a truncated
//! tile ROM) and checks that only that fighter's bust becomes `None` while
//! the other eleven decode exactly as before. The ROM tests consume
//! local-only data and skip on a clean checkout.

use std::path::Path;

use sf_video::busts::{decode_busts, decode_busts_placed, Bust, BustPlacement, BLOCK_TILES, FIGHTERS};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// The background tilemap's halves in `tilerom` (render.rs): colour and code
/// low byte in the first 0x10000 bytes, flip bits and code high byte in the
/// second.
const TILEROM_HALF: usize = 0x10000;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROMs not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).unwrap())
}

fn index(name: &str) -> usize {
    FIGHTERS.iter().position(|f| *f == name).unwrap()
}

/// The `tilerom` entry offsets (first half) of a bust's 6x6 block.
fn block_entries(p: &BustPlacement) -> Vec<usize> {
    let mut entries = Vec::new();
    for c in 0..BLOCK_TILES {
        for r in 0..BLOCK_TILES {
            entries.push(2 * ((p.block_col + c) * 16 + p.block_row + r));
        }
    }
    entries
}

fn code_at(assets: &Assets, entry: usize) -> usize {
    ((assets.tilerom[entry + TILEROM_HALF + 1] as usize) << 8) | assets.tilerom[entry + 1] as usize
}

fn same(a: &Bust, b: &Bust) -> bool {
    (a.width, a.height) == (b.width, b.height) && a.argb == b.argb
}

/// Decode, and check that `damaged` is the only bust that changed: it is
/// `None` and the other eleven equal the undamaged decode.
fn assert_only(assets: &Assets, baseline: &[Option<Bust>; 12], damaged: &str) {
    let busts = decode_busts(assets);
    for (i, bust) in busts.iter().enumerate() {
        if FIGHTERS[i] == damaged {
            assert!(bust.is_none(), "{damaged}'s damaged bust must be None");
        } else {
            let (got, want) = (bust.as_ref(), baseline[i].as_ref().unwrap());
            assert!(
                got.is_some_and(|b| same(b, want)),
                "{} must decode unchanged when only {damaged} is damaged",
                FIGHTERS[i]
            );
        }
    }
}

#[test]
fn every_bust_decodes_from_the_rom_regions() {
    let Some(assets) = assets() else { return };
    let placed = decode_busts_placed(&assets);
    for (i, b) in placed.iter().enumerate() {
        let (bust, p) = b.as_ref().unwrap_or_else(|| panic!("{} has no bust", FIGHTERS[i]));
        assert_eq!(bust.argb.len(), bust.width * bust.height);
        // Each bust lies inside its 6x6 block (screen x 64..=159 on the left,
        // 224..=319 on the right; y 32..=127) plus the one-pixel border.
        // Ryu is on the left of the one-player VS screens, everyone else on
        // the right.
        let left = if i == 0 { 63 } else { 223 };
        assert!(p.x >= left && p.x + bust.width <= left + 98, "{}: {p:?}", FIGHTERS[i]);
        assert!(p.y >= 31 && p.y + bust.height <= 31 + 98, "{}: {p:?}", FIGHTERS[i]);
        assert!(bust.width >= 64 && bust.height >= 64, "{}: a whole bust", FIGHTERS[i]);
        assert!(bust.argb.iter().all(|c| c >> 24 == 0xff), "opaque");
        // The border is the VS backdrop.
        assert_eq!(bust.argb[0], 0xff11_1111);
    }
}

#[test]
fn blank_cells_unset_only_that_fighters_bust() {
    let Some(mut assets) = assets() else { return };
    let baseline = decode_busts(&assets);
    let placed = decode_busts_placed(&assets);
    let geki = placed[index("Geki")].as_ref().unwrap().1;
    // Point every cell of Geki's block at the blank tile (code 0, colour 0).
    for e in block_entries(&geki) {
        assets.tilerom[e] = 0;
        assets.tilerom[e + 1] = 0;
        assets.tilerom[e + TILEROM_HALF] = 0;
        assets.tilerom[e + TILEROM_HALF + 1] = 0;
    }
    assert_only(&assets, &baseline, "Geki");
}

#[test]
fn an_out_of_range_tile_code_unsets_only_that_fighters_bust() {
    let Some(mut assets) = assets() else { return };
    let baseline = decode_busts(&assets);
    let placed = decode_busts_placed(&assets);
    let joe = placed[index("Joe")].as_ref().unwrap().1;
    // One cell of Joe's block names tile 0xffxx; gfx1 holds 0x1000 tiles.
    let e = block_entries(&joe)[14];
    assets.tilerom[e + TILEROM_HALF + 1] = 0xff;
    assert!(code_at(&assets, e) >= assets.gfx1.len() / 128);
    assert_only(&assets, &baseline, "Joe");
}

#[test]
fn zeroed_tile_graphics_unset_only_that_fighters_bust() {
    let Some(mut assets) = assets() else { return };
    let baseline = decode_busts(&assets);
    let placed = decode_busts_placed(&assets);
    // The tile codes each bust block uses.
    let codes: Vec<Vec<usize>> = placed
        .iter()
        .map(|b| {
            let p = b.as_ref().unwrap().1;
            block_entries(&p).into_iter().map(|e| code_at(&assets, e)).collect()
        })
        .collect();
    let mike = index("Mike");
    // Zero Mike's own tiles in both plane halves of gfx1 (64 bytes per tile
    // in each half); tiles another block also uses are left alone.
    let half = assets.gfx1.len() / 2;
    let own: Vec<usize> = codes[mike]
        .iter()
        .copied()
        .filter(|c| codes.iter().enumerate().all(|(i, cs)| i == mike || !cs.contains(c)))
        .collect();
    assert!(own.len() >= 30, "Mike's block is mostly his own tiles");
    for code in own {
        assets.gfx1[code * 64..code * 64 + 64].fill(0);
        assets.gfx1[half + code * 64..half + code * 64 + 64].fill(0);
    }
    assert_only(&assets, &baseline, "Mike");
}

#[test]
fn a_truncated_tile_rom_unsets_only_the_bust_past_its_end() {
    let Some(mut assets) = assets() else { return };
    let baseline = decode_busts(&assets);
    let placed = decode_busts_placed(&assets);
    // Sagat's block has the highest tilemap columns; cut the tile ROM's
    // second half (the code high bytes) just before it.
    let sagat = placed[index("Sagat")].as_ref().unwrap().1;
    let others_max = placed
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index("Sagat"))
        .map(|(_, b)| b.as_ref().unwrap().1.block_col)
        .max()
        .unwrap();
    assert!(others_max + BLOCK_TILES <= sagat.block_col);
    assets.tilerom.truncate(TILEROM_HALF + 2 * 16 * sagat.block_col);
    assert_only(&assets, &baseline, "Sagat");
}

#[test]
fn empty_regions_decode_to_no_busts() {
    let assets = Assets {
        maincpu: Vec::new(),
        gfx1: Vec::new(),
        gfx2: Vec::new(),
        gfx3: Vec::new(),
        gfx4: Vec::new(),
        tilerom: Vec::new(),
    };
    assert!(decode_busts(&assets).iter().all(Option::is_none));
}
