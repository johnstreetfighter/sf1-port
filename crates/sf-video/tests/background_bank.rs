//! Ticket #198 (`ST5`): the extended background bank and its packer.
//!
//! - The bank's tiles go above the ROM's in both halves of `gfx1` and
//!   `gfx2`, its pages after the ROM tilemap, and the renderer draws a
//!   layer from the page its scene names, its tiles through the same
//!   decode as the ROM's.
//! - The packer places a stage that keeps the 1987 background rules and
//!   rejects one that breaks any of them, leaving the bank as it was.
//! - The committed test stage rebuilds from the ROM and packs.
//!
//! The ROM checks need the local ROMs and skip without them; the rest
//! build their own graphics.

use std::path::Path;

use sf_video::background_bank::{BackgroundBank, Cell, MapLayer, PAGE_BYTES, PALETTE_WINDOW, ROM_BG_TILES, ROM_FG_TILES};
use sf_video::decode::decode_sprite_tile;
use sf_video::render::{render_frame, WIDTH};
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;
use sf_video::stage::{pack_records, ArtCell, LayerArt, StageArt, StageError, StageRecord, TileRef};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const TEST_STAGE: &str = include_str!("../../../art/stages/test-stage.json");

fn rom() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).unwrap())
}

/// A tile no other shares: pens 0..=14 in a pattern keyed by `k`.
fn pattern(k: usize) -> [[u8; 16]; 16] {
    std::array::from_fn(|y| std::array::from_fn(|x| ((k * 7 + x + 3 * y) % 15) as u8))
}

/// Blank ROM graphics of the real sizes, so the checks run without the ROMs.
fn blank_graphics() -> Assets {
    Assets {
        maincpu: vec![0; 0x60000],
        gfx1: vec![0; ROM_BG_TILES * 128],
        gfx2: vec![0; ROM_FG_TILES * 128],
        gfx3: vec![0; 128],
        gfx4: vec![0; 16],
        tilerom: vec![0; PAGE_BYTES],
    }
}

fn cell(tile: TileRef, colour: u8) -> ArtCell {
    ArtCell { tile, colour, attr: 0 }
}

/// A palette filling `rows` with greys.
fn palette(rows: &[usize]) -> [Option<[u16; 16]>; 16] {
    std::array::from_fn(|r| rows.contains(&r).then(|| std::array::from_fn(|p| (p as u16) * 0x111)))
}

/// A stage that keeps every rule: ROM tiles and two new tiles, its
/// background's first 24 columns repeated 32 columns on.
fn good() -> StageArt {
    let fg = LayerArt {
        columns: (0..48).map(|c| std::array::from_fn(|r| cell(TileRef::Rom((c * 16 + r) as u16), 1))).collect(),
        tiles: vec![],
        palette: palette(&[1]),
    };
    let bg_column = |c: usize| -> [ArtCell; 16] {
        let c = if (32..56).contains(&c) { c - 32 } else { c };
        std::array::from_fn(|r| {
            if r == 7 {
                cell(TileRef::New((c % 2) as u16), 3)
            } else {
                cell(TileRef::Rom((c * 16 + r) as u16), 2)
            }
        })
    };
    let bg = LayerArt { columns: (0..56).map(bg_column).collect(), tiles: vec![pattern(1), pattern(2)], palette: palette(&[2, 3]) };
    StageArt {
        name: "good".into(),
        owner: Some("Nobody".into()),
        theme: 0x29,
        page: 1,
        fg_scroll: 0x1c80,
        bg_scroll: 0x1800,
        fg,
        bg,
    }
}

#[test]
fn a_stage_that_keeps_the_rules_is_placed_where_its_scroll_words_show_it() {
    let mut bank = BackgroundBank::new();
    let art = good();
    let index = bank.pack_stage(&art).expect("the good stage packs");
    assert_eq!(index, 0);
    assert_eq!(bank.stage_index("good"), Some(0));
    assert_eq!((bank.pages(), bank.tiles(MapLayer::Bg), bank.tiles(MapLayer::Fg)), (1, 2, 0));
    let view = bank.stage(0).unwrap();
    assert_eq!((view.fg.page, view.fg.scroll, view.bg.page, view.bg.scroll, view.theme), (1, 0x1c80, 1, 0x1800, 0x29));
    // The fight's first column is 4 past the word: the visible area starts
    // 64 pixels into the screen bitmap.
    let (fg0, bg0) = (0x1c80 / 16 + 4, 0x1800 / 16 + 4);
    for c in 0..48 {
        for r in 0..16 {
            assert_eq!(bank.cell(1, MapLayer::Fg, fg0 + c, r), Cell { code: (c * 16 + r) as u16, colour: 1, attr: 0 });
        }
    }
    assert_eq!(bank.cell(1, MapLayer::Bg, bg0 + 3, 7), Cell { code: ROM_BG_TILES as u16 + 1, colour: 3, attr: 0 });
    assert_eq!(bank.cell(1, MapLayer::Bg, bg0 + 3, 6).code, 3 * 16 + 6);
    // Outside the footprint the page is unprogrammed.
    assert_eq!(bank.cell(1, MapLayer::Fg, fg0 - 1, 0), Cell { code: 0xffff, colour: 0xff, attr: 0xff });
    assert_eq!(bank.cell(1, MapLayer::Bg, bg0 + 56, 0), Cell { code: 0xffff, colour: 0xff, attr: 0xff });
    // Its palettes are named by pointers in the bank's window.
    for (pointer, row) in [(view.fg.palette, 1), (view.bg.palette, 3)] {
        assert!(pointer >= PALETTE_WINDOW);
        let bytes = bank.palette(pointer).unwrap();
        assert_eq!(u16::from_be_bytes([bytes[row * 32 + 30], bytes[row * 32 + 31]]), 15 * 0x111);
        assert_eq!(&bytes[..32], &[0; 32][..], "an empty row is black");
    }
    assert_eq!(bank.palette(0x1b068), None, "the ROM's palettes are not the bank's");
}

/// `good()` changed by `f` is rejected with `want`, and the bank is left
/// as it was.
fn rejects(f: impl FnOnce(&mut StageArt), want: StageError) {
    let mut bank = BackgroundBank::new();
    let mut other = good();
    other.name = "first".into();
    other.page = 2;
    bank.pack_stage(&other).unwrap();
    let before = bank.clone();
    let mut art = good();
    f(&mut art);
    assert_eq!(bank.pack_stage(&art), Err(want));
    assert_eq!(bank, before, "a rejected stage changes nothing");
}

#[test]
fn the_packer_rejects_a_stage_that_breaks_the_1987_background_rules() {
    use MapLayer::{Bg, Fg};
    rejects(|a| a.page = 0, StageError::RomPage);
    rejects(|a| a.name = "first".into(), StageError::Name("first".into()));
    // The footprints: 48 foreground and 56 background columns.
    rejects(|a| { a.fg.columns.pop(); }, StageError::Width { layer: Fg, columns: 47, expected: 48 });
    rejects(|a| a.bg.columns.push(a.bg.columns[0]), StageError::Width { layer: Bg, columns: 57, expected: 56 });
    // The drift's jump back: column 5 no longer equals column 37.
    rejects(|a| a.bg.columns[37][9].colour = 3, StageError::DriftRepeat { col: 5 });
    rejects(|a| a.bg.columns[23][0].attr = 1, StageError::DriftRepeat { col: 23 });
    // Whole-column scroll words, and windows inside the page.
    rejects(|a| a.fg_scroll = 0x1c88, StageError::Unaligned { layer: Fg, scroll: 0x1c88 });
    rejects(|a| a.bg_scroll = 0x7e00, StageError::PastPage { layer: Bg, last_pixel: 0x7e00 + 0x200 + 447 });
    rejects(|a| a.fg_scroll = 0x7d00, StageError::PastPage { layer: Fg, last_pixel: 0x7d00 + 0x180 + 447 });
    // Never over cells already on the page.
    rejects(|a| a.page = 2, StageError::Overlap { layer: Fg, col: 0x1c80 / 16 + 4 });
    // Tile and colour ranges.
    rejects(|a| a.bg.columns[40][2].tile = TileRef::Rom(0x1000), StageError::RomTile { layer: Bg, col: 40, row: 2, code: 0x1000 });
    rejects(|a| a.fg.columns[3][4].tile = TileRef::Rom(0x2000), StageError::RomTile { layer: Fg, col: 3, row: 4, code: 0x2000 });
    rejects(|a| a.bg.columns[50][1].tile = TileRef::New(2), StageError::NewTile { layer: Bg, col: 50, row: 1, index: 2 });
    rejects(|a| a.fg.columns[0][0].colour = 16, StageError::Colour { layer: Fg, col: 0, row: 0, colour: 16 });
    rejects(|a| a.fg.columns[1][2].colour = 2, StageError::Colour { layer: Fg, col: 1, row: 2, colour: 2 });
    rejects(|a| a.bg.palette[2] = None, StageError::Colour { layer: Bg, col: 0, row: 0, colour: 2 });
    rejects(|a| a.fg.columns[9][9].attr = 4, StageError::Attr { layer: Fg, col: 9, row: 9, attr: 4 });
    rejects(|a| a.bg.tiles[1][3][4] = 16, StageError::Pen { layer: Bg, tile: 1, pen: 16 });
    rejects(|a| a.fg.palette[1].as_mut().unwrap()[0] = 0x1000, StageError::PaletteWord { layer: Fg, row: 1, word: 0x1000 });
}

#[test]
fn the_renderer_draws_a_layer_from_its_page_with_the_banks_tiles() {
    let mut bank = BackgroundBank::new();
    bank.pack_stage(&good()).unwrap();
    // A built map, as #215 builds its VS screens: a page of its own.
    let page = bank.add_page();
    assert_eq!(page, 2);
    let code = bank.add_tile(MapLayer::Fg, &pattern(9));
    assert_eq!(code as usize, ROM_FG_TILES);
    for c in 0..2048 {
        for r in 0..16 {
            bank.set_cell(page, MapLayer::Fg, c, r, Cell { code, colour: 5, attr: 0 });
        }
    }
    let gfx = blank_graphics().with_background_bank(&bank);
    assert_eq!(gfx.background_pages(), 3);
    assert_eq!(gfx.tilerom[..PAGE_BYTES], blank_graphics().tilerom[..], "the ROM tilemap is page 0");
    assert_eq!(decode_sprite_tile(&gfx.gfx1, ROM_BG_TILES + 1), pattern(2));
    assert_eq!(decode_sprite_tile(&gfx.gfx2, ROM_FG_TILES), pattern(9));
    // Fg palette row 5: pen p is a grey of level p.
    let mut paletteram = vec![0u8; 2048];
    for p in 0..16u16 {
        let at = 2 * (256 + 5 * 16 + p as usize);
        paletteram[at..at + 2].copy_from_slice(&(p * 0x111).to_be_bytes());
    }
    let scene = |fg_page: u8, flip: bool| SceneState {
        videoram: vec![0; 0x1000],
        objectram: vec![0; 0x2000],
        paletteram: paletteram.clone(),
        bg_scroll: 0,
        fg_scroll: 0x30,
        active: 0x40,
        flip_screen: flip,
        bg_page: 0,
        fg_page,
    };
    let pal = sf_video::palette::Palette::from_ram(&paletteram);
    for flip in [false, true] {
        let frame = render_frame(&gfx, &scene(2, flip));
        // Every cell shows the bank tile in row 5's greys, either way up.
        let pen_at = |x: usize, y: usize| frame.argb[y * WIDTH + x];
        let tile = pattern(9);
        let shown: std::collections::BTreeSet<u32> = frame.argb.iter().copied().collect();
        let want: std::collections::BTreeSet<u32> =
            tile.iter().flatten().filter(|&&p| p != 15).map(|&p| pal.argb(256 + 80 + p as usize)).collect();
        assert!(shown.is_subset(&want.union(&[pal.argb(0)].into()).copied().collect()), "flip {flip}");
        if !flip {
            // Unflipped, screen pixel (x, y) is tile pixel ((x + 0x70) % 16, y % 16).
            for y in 0..224 {
                for x in 0..384 {
                    let pen = tile[(y + 16) % 16][(x + 0x70) % 16];
                    if pen != 15 {
                        assert_eq!(pen_at(x, y), pal.argb(256 + 80 + pen as usize), "({x}, {y})");
                    }
                }
            }
        }
    }
    // Page 0 still draws the ROM's map (blank: tile 0, colour 0, pen 0).
    let frame = render_frame(&gfx, &scene(0, false));
    assert!(frame.argb.iter().all(|&p| p == pal.argb(0) || p == pal.argb(256)));
    // Without the bank every page reads the ROM's.
    assert_eq!(render_frame(&blank_graphics(), &scene(2, false)).argb, render_frame(&blank_graphics(), &scene(0, false)).argb);
}

#[test]
fn an_empty_bank_changes_no_region() {
    let Some(a) = rom() else { return };
    let with = a.with_background_bank(&BackgroundBank::new());
    assert_eq!((with.gfx1 == a.gfx1, with.gfx2 == a.gfx2, with.tilerom == a.tilerom), (true, true, true));
    assert_eq!(a.background_pages(), 1);
    assert_eq!((a.layer_tiles(MapLayer::Bg), a.layer_tiles(MapLayer::Fg)), (ROM_BG_TILES, ROM_FG_TILES));
}

#[test]
fn the_test_stage_rebuilds_from_the_rom_and_keeps_every_rom_tile() {
    let Some(a) = rom() else { return };
    let record = StageRecord::from_json(TEST_STAGE).unwrap();
    assert_eq!((record.name.as_str(), record.owner.as_deref(), record.page), ("test-stage", None, 1));
    let bank = pack_records(&a, &[TEST_STAGE]).expect("the test stage keeps the 1987 rules");
    let with = a.with_background_bank(&bank);
    // Geki's background's distinct tiles are copied above the ROM's.
    assert_eq!(bank.tiles(MapLayer::Bg), 353);
    assert_eq!(bank.tiles(MapLayer::Fg), 0);
    let half = |r: &[u8]| r.len() / 2;
    assert_eq!(with.gfx1[..half(&a.gfx1)], a.gfx1[..half(&a.gfx1)]);
    for i in 0..ROM_BG_TILES {
        assert_eq!(decode_sprite_tile(&with.gfx1, i), decode_sprite_tile(&a.gfx1, i), "ROM bg tile {i:#06x}");
    }
    // Every background cell is the ROM's cell of Geki's background, drawn
    // from a bank copy of its tile, in its moved colour row.
    let view = bank.stage(bank.stage_index("test-stage").unwrap()).unwrap();
    let (geki, joe) = (0x2180 / 16 + 4, 0x2400 / 16 + 4);
    let (bg0, fg0) = (view.bg.scroll as usize / 16 + 4, view.fg.scroll as usize / 16 + 4);
    for c in 0..56 {
        for r in 0..16 {
            let rom = Cell::read(&a.tilerom, 0, (geki + c) * 16 + r);
            let ours = bank.cell(1, MapLayer::Bg, bg0 + c, r);
            assert!(ours.code as usize >= ROM_BG_TILES);
            assert_eq!(decode_sprite_tile(&with.gfx1, ours.code as usize), decode_sprite_tile(&a.gfx1, rom.code as usize));
            assert_eq!((ours.colour, ours.attr), ([(0, 12), (8, 13)].iter().find(|m| m.0 == rom.colour).unwrap().1, rom.attr));
        }
    }
    // Every foreground cell is Joe's, ROM tile and colour row as they are.
    for c in 0..48 {
        for r in 0..16 {
            assert_eq!(bank.cell(1, MapLayer::Fg, fg0 + c, r), Cell::read(&a.tilerom, 0x20000, (joe + c) * 16 + r));
        }
    }
    // Its palettes are rows of the ROM's: Japan's stage rows 0 and 8 at 12
    // and 13, USA's foreground rows 0-4 in place.
    let (bg, fg) = (bank.palette(view.bg.palette).unwrap(), bank.palette(view.fg.palette).unwrap());
    assert_eq!(bg[12 * 32..13 * 32], a.maincpu[0x1b068..0x1b068 + 32]);
    assert_eq!(bg[13 * 32..14 * 32], a.maincpu[0x1b068 + 8 * 32..0x1b068 + 9 * 32]);
    assert_eq!(fg[..5 * 32], a.maincpu[0x1a668..0x1a668 + 5 * 32]);
}
