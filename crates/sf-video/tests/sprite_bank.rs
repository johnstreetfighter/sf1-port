//! Ticket #165 (`AP1`): the extended sprite bank. Tiles above the ROM's
//! 14,336 go into both halves of the sprite region, so every ROM tile keeps
//! its place, and each sits in the slot the renderer reads for its code, so
//! the renderer's tile-code swizzle draws the intended tile. The ROM checks
//! need the local ROMs and skip without them; the drawing check builds its
//! own graphics.

use std::path::Path;

use sf_video::decode::decode_sprite_tile;
use sf_video::render::{render_frame, VISAREA_X, VISAREA_Y, WIDTH};
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;
use sf_video::sprite_bank::{encode_sprite_tile, sprite_slot, SpriteBank, ROM_SPRITE_TILES};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn rom() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).unwrap())
}

/// A tile no other code shares: pens 0..=14 (15 is transparent) in a
/// pattern keyed by the code.
fn pattern(code: u16) -> [[u8; 16]; 16] {
    std::array::from_fn(|y| std::array::from_fn(|x| ((code as usize * 7 + x + 3 * y) % 15) as u8))
}

#[test]
fn encoding_is_the_exact_inverse_of_decoding_for_every_rom_tile() {
    let Some(a) = rom() else { return };
    assert_eq!(a.sprite_tiles(), ROM_SPRITE_TILES);
    let half = a.gfx3.len() / 2;
    for i in 0..ROM_SPRITE_TILES {
        let [first, second] = encode_sprite_tile(&decode_sprite_tile(&a.gfx3, i));
        assert_eq!(first[..], a.gfx3[i * 64..i * 64 + 64], "tile {i:#06x}, first half");
        assert_eq!(second[..], a.gfx3[half + i * 64..half + i * 64 + 64], "tile {i:#06x}, second half");
    }
}

#[test]
fn a_bank_keeps_every_rom_tile_and_holds_its_own_where_the_renderer_reads() {
    let Some(a) = rom() else { return };
    let mut bank = SpriteBank::new();
    let codes: Vec<u16> = (0x3800..0x3840).chain([0x3a5b, 0x3fff]).collect();
    for &c in &codes {
        bank.put(c, &pattern(c));
    }
    let with = a.with_sprite_bank(&bank);
    assert_eq!(with.sprite_tiles(), ROM_SPRITE_TILES + bank.tiles());
    for i in 0..ROM_SPRITE_TILES {
        assert_eq!(decode_sprite_tile(&with.gfx3, i), decode_sprite_tile(&a.gfx3, i), "ROM tile {i:#06x}");
    }
    for &c in &codes {
        assert_eq!(decode_sprite_tile(&with.gfx3, sprite_slot(c)), pattern(c), "bank tile {c:#06x}");
    }
    // An empty bank changes nothing.
    assert_eq!(a.with_sprite_bank(&SpriteBank::new()).gfx3, a.gfx3);
}

#[test]
#[should_panic(expected = "belongs to the ROM")]
fn the_bank_never_replaces_a_rom_tile() {
    SpriteBank::new().put(0x37ff, &pattern(0));
}

/// Graphics with a blank ROM sprite region of the real size, so the
/// drawing check runs without the ROMs.
fn blank_graphics() -> Assets {
    Assets {
        maincpu: Vec::new(),
        gfx1: vec![0; 128],
        gfx2: vec![0; 128],
        gfx3: vec![0xff; ROM_SPRITE_TILES * 128],
        gfx4: vec![0; 16],
        tilerom: vec![0; 0x20000],
    }
}

#[test]
fn bank_tiles_draw_through_the_renderer_and_its_code_swizzle() {
    // Small entries for 32 codes (all four swizzle groups) and one large
    // 2×2 entry; each must draw its own tile.
    let small: Vec<u16> = (0x3800..0x3820).collect();
    let large = 0x3842u16;
    let mut bank = SpriteBank::new();
    for &c in small.iter().chain(&[large, large + 1, large + 16, large + 17]) {
        bank.put(c, &pattern(c));
    }
    let gfx = blank_graphics().with_sprite_bank(&bank);
    let mut objectram = vec![0u8; 0x2000];
    // Park every entry off screen, then place the ones under test.
    for e in 0..128 {
        objectram[e * 0x40 + 4..e * 0x40 + 6].copy_from_slice(&0x0300u16.to_be_bytes());
    }
    let mut place = |e: usize, code: u16, attr: u16, x: usize, y: usize| {
        let words = [code, attr, (y + VISAREA_Y) as u16, (x + VISAREA_X) as u16];
        for (i, w) in words.iter().enumerate() {
            objectram[e * 0x40 + 2 * i..e * 0x40 + 2 * i + 2].copy_from_slice(&w.to_be_bytes());
        }
    };
    for (k, &c) in small.iter().enumerate() {
        place(k, c, 0, 16 * (k % 16), 16 * (k / 16));
    }
    place(40, large, 0x400, 0, 64);
    // Sprite palette 0: pen p is a grey of level p.
    let mut paletteram = vec![0u8; 2048];
    for p in 0..16u16 {
        let v = (p << 8) | (p << 4) | p;
        paletteram[2 * (512 + p as usize)..2 * (512 + p as usize) + 2].copy_from_slice(&v.to_be_bytes());
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
    let pal = sf_video::palette::Palette::from_ram(&paletteram);
    let check = |code: u16, x0: usize, y0: usize| {
        for (y, row) in pattern(code).iter().enumerate() {
            for (x, &pen) in row.iter().enumerate() {
                let want = pal.argb(512 + pen as usize);
                assert_eq!(frame.argb[(y0 + y) * WIDTH + x0 + x], want, "code {code:#06x} at ({x}, {y})");
            }
        }
    };
    for (k, &c) in small.iter().enumerate() {
        check(c, 16 * (k % 16), 16 * (k / 16));
    }
    for (c, dx, dy) in [(large, 0, 0), (large + 1, 16, 0), (large + 16, 0, 16), (large + 17, 16, 16)] {
        check(c, dx, 64 + dy);
    }
}
