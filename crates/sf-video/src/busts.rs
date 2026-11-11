//! The twelve tournament fighters' VS-screen busts, decoded from the ROM
//! regions for the front end's portrait select (ticket FE5, #155).
//!
//! Before each fight the screen task's event 4 (`0x3f9ec`) shows a VS
//! screen: the foreground and text layers off (`0x48`), the background
//! layer (gfx1, tilerom base 0) on (`0x20`) and scrolled to a 384-pixel
//! window of its tilemap, and a 0x200-byte palette table copied from the
//! program ROM to palette RAM offset 0 (the background's 256 entries) by the
//! vblank's palette request (`0x3b362`). Every VS screen has the same
//! layout: two 6x6-tile busts on tilemap rows 3..=8 (screen y 32..=127), the
//! left one at screen tile columns 4..=9 (x 64..=159), the right one at
//! 14..=19 (x 224..=319), and the nameplate on rows 10..=11. The cells around
//! them hold the blank tile (code 0, colour 0), whose pens are all 15; the
//! bust tiles draw their background with pen 15 too, and every VS palette
//! sets pen 15 of each colour to the same backdrop (`0x111`).
//!
//! - One player (game mode 2, `0x3fa56`): the background scroll is the word
//!   at `opponent * 2` of the table that `0x3fa66 + country * 4` points to
//!   (the opponent index is stage `0xffc003` * 2 + fight `0xffc004`), and the
//!   palette is the stage palette that `0x43748` requests: the longword at
//!   `stage * 4` of the table that `0x43778 + country * 4` points to. The
//!   player's bust (Ryu) is on the left, the opponent's on the right. (Mode
//!   4, Player 2 alone, uses `0x3fad6` instead: the opponent on the left and
//!   Ken on the right; the decoder does not need those screens.)
//! - Versus (`0x3fa00`): the background scroll `0x780` and the palette
//!   `0x1ae68`: RYU VS KEN, Ken on the right.
//!
//! From the Japan flag (`0xffc006 = 0`) the ten opponents come in the order
//! Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat: the names
//! on the ten VS screens' nameplates and the stages fought after them
//! (`sf-win-04` shows RYU VS LEE and then the fight on the Great Wall).
//! Those are entries 2..=11 of the select order.
//!
//! The decoder reads only `tilerom`, `gfx1` and `maincpu`. A bust whose
//! tables, tile codes or palette fall outside the regions, or whose block
//! draws nothing but backdrop or a single colour, is `None`.

use crate::decode::decode_sprite_tile;
use crate::palette::Palette;
use crate::render::{HEIGHT, VISAREA_X, VISAREA_Y, WIDTH};
use crate::romset::Assets;

/// One fighter's bust: the VS screen's own pixels, trimmed to every pixel
/// the bust tiles draw plus a one-pixel backdrop border (like the crops
/// `tools/player_select.py` cuts from the captures, which keep only the
/// largest connected blob and so can lose a detached pixel's column).
pub struct Bust {
    /// `0xAARRGGBB`, opaque, row-major.
    pub argb: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

impl Bust {
    /// The bust flipped left to right.
    pub fn mirrored(&self) -> Bust {
        let mut argb = Vec::with_capacity(self.argb.len());
        for row in self.argb.chunks(self.width) {
            argb.extend(row.iter().rev());
        }
        Bust {
            argb,
            width: self.width,
            height: self.height,
        }
    }
}

/// The busts for the portrait select, all facing the same way. The VS
/// screen draws the player's bust on the left facing right and the
/// opponent's on the right facing left, so the left-side busts (Ryu's)
/// are mirrored to face left like the other eleven.
pub fn decode_select_busts(assets: &Assets) -> [Option<Bust>; 12] {
    decode_busts_placed(assets).map(|placed| {
        placed.map(|(bust, at)| if at.x < WIDTH / 2 { bust.mirrored() } else { bust })
    })
}

/// Where a decoded bust comes from and where the machine draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BustPlacement {
    /// The VS screen's background scroll register (`0xc00018`) value.
    pub bg_scroll: u16,
    /// Address in `maincpu` of the 0x200-byte palette table the VS screen
    /// uploads to palette RAM offset 0.
    pub palette: usize,
    /// Tilemap cell (column, row) of the bust block's top-left tile; the
    /// block is [`BLOCK_TILES`] x [`BLOCK_TILES`] cells of the background
    /// tilemap (`tilerom` entry `2 * (col * 16 + row)`).
    pub block_col: usize,
    pub block_row: usize,
    /// Screen position (in the 384x224 frame) of the bust image's top-left
    /// pixel while the VS screen is up.
    pub x: usize,
    pub y: usize,
}

/// A bust block is 6x6 tiles of 16x16 pixels.
pub const BLOCK_TILES: usize = 6;

/// Select order (`sf_engine::roster::TournamentFighter::ALL`).
pub const FIGHTERS: [&str; 12] = [
    "Ryu", "Ken", "Retsu", "Geki", "Joe", "Mike", "Lee", "Gen", "Birdie", "Eagle", "Adon", "Sagat",
];

/// `0x3fa66`: one longword per starting country, each pointing to ten words
/// of background scroll, one per opponent index (read by event 4's mode-2
/// branch at `0x3fa56`; Japan's table is `0x3fa76`).
const ONE_PLAYER_VS_SCROLLS: usize = 0x3fa66;
/// `0x43778`: one longword per starting country, each pointing to five
/// longwords (one per stage) of palette table addresses (`0x43748`).
const STAGE_PALETTES: usize = 0x43778;
/// The Japan flag, `0xffc006 = 0`.
const JAPAN: usize = 0;
/// Versus event 4 (`0x3fa00`): background scroll `0x780`, palette `0x1ae68`.
const VERSUS_VS_SCROLL: u16 = 0x0780;
const VERSUS_VS_PALETTE: usize = 0x1ae68;

/// Bytes in one VS palette table: 256 xRGB_444 words, colours 0..=15 of the
/// background layer.
const PALETTE_BYTES: usize = 0x200;
/// Tilemap row of the bust blocks' top edge.
const BLOCK_ROW: usize = 3;
/// Screen tile columns of the left and right bust blocks.
const LEFT_BLOCK: usize = 4;
const RIGHT_BLOCK: usize = 14;
/// The bust tiles' background pen (and the blank tile's only pen).
const BACKDROP_PEN: u8 = 15;

/// The background tilemap: 2048 columns of 16 rows of 16x16 tiles,
/// column-major, its entries split across the two 0x10000-byte halves of
/// `tilerom` (render.rs).
const MAP_COLS: usize = 2048;
const MAP_ROWS: usize = 16;
const TILE: usize = 16;
const MAP_WIDTH_PX: usize = MAP_COLS * TILE;
const TILEROM_HALF: usize = 0x10000;
/// Bytes per tile of a sprite_layout region: 64 in each plane half.
const TILE_BYTES: usize = 128;

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

/// Decode the twelve busts in [`FIGHTERS`] order.
pub fn decode_busts(assets: &Assets) -> [Option<Bust>; 12] {
    decode_busts_placed(assets).map(|b| b.map(|(bust, _)| bust))
}

/// [`decode_busts`] with each bust's VS screen and screen position.
pub fn decode_busts_placed(assets: &Assets) -> [Option<(Bust, BustPlacement)>; 12] {
    std::array::from_fn(|i| {
        let (scroll, palette, side) = source(assets, i)?;
        decode_bust(assets, scroll, palette, side)
    })
}

/// The VS screen (background scroll, palette table) and side of fighter `i`.
fn source(assets: &Assets, i: usize) -> Option<(u16, usize, Side)> {
    let m = &assets.maincpu;
    match i {
        // Ken: the versus screen, right side.
        1 => Some((VERSUS_VS_SCROLL, VERSUS_VS_PALETTE, Side::Right)),
        // Ryu: the first one-player screen (Retsu's), left side; every
        // one-player VS screen draws the same Ryu block in colour 2, which
        // all five stage palettes give the same values.
        0 | 2..=11 => {
            let opponent = i.saturating_sub(2);
            let scrolls = long(m, ONE_PLAYER_VS_SCROLLS + 4 * JAPAN)? as usize;
            let scroll = word(m, scrolls + 2 * opponent)?;
            let palettes = long(m, STAGE_PALETTES + 4 * JAPAN)? as usize;
            let palette = long(m, palettes + 4 * (opponent / 2))? as usize;
            let side = if i == 0 { Side::Left } else { Side::Right };
            Some((scroll, palette, side))
        }
        _ => None,
    }
}

fn word(m: &[u8], addr: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*m.get(addr)?, *m.get(addr + 1)?]))
}

fn long(m: &[u8], addr: usize) -> Option<u32> {
    Some(((word(m, addr)? as u32) << 16) | word(m, addr + 2)? as u32)
}

/// One background-layer pixel: its pen and its colour.
#[derive(Clone, Copy)]
struct Pixel {
    pen: u8,
    argb: u32,
}

/// Decode the bust on `side` of the VS screen at `scroll` with `palette`.
fn decode_bust(
    assets: &Assets,
    scroll: u16,
    palette: usize,
    side: Side,
) -> Option<(Bust, BustPlacement)> {
    let table = assets.maincpu.get(palette..palette + PALETTE_BYTES)?;
    let mut ram = vec![0u8; 0x800];
    ram[..PALETTE_BYTES].copy_from_slice(table);
    let palette_ram = Palette::from_ram(&ram);

    let block_x = TILE
        * match side {
            Side::Left => LEFT_BLOCK,
            Side::Right => RIGHT_BLOCK,
        };
    let block_y = BLOCK_ROW * TILE - VISAREA_Y;
    let size = BLOCK_TILES * TILE;
    // The block and a one-pixel ring around it, in screen pixels.
    let (wx, wy, ww) = (block_x - 1, block_y - 1, size + 2);
    let mut window = Vec::with_capacity(ww * ww);
    let mut tiles = std::collections::HashMap::new();
    for sy in wy..wy + ww {
        for sx in wx..wx + ww {
            window.push(background_pixel(assets, &palette_ram, &mut tiles, scroll, sx, sy)?);
        }
    }

    // What the bust tiles draw: pens other than the backdrop pen, inside
    // the block.
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    let mut colours = std::collections::HashSet::new();
    for y in 1..=size {
        for x in 1..=size {
            let p = window[y * ww + x];
            if p.pen != BACKDROP_PEN {
                colours.insert(p.argb);
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    // Nothing drawn, or one flat colour: no picture to show.
    if colours.len() < 2 {
        return None;
    }
    let (x0, y0, x1, y1) = (x0 - 1, y0 - 1, x1 + 1, y1 + 1);
    let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
    let mut argb = Vec::with_capacity(width * height);
    for y in y0..=y1 {
        argb.extend(window[y * ww + x0..=y * ww + x1].iter().map(|p| p.argb));
    }
    let first_col = (scroll as usize + VISAREA_X + block_x) % MAP_WIDTH_PX / TILE;
    Some((
        Bust {
            argb,
            width,
            height,
        },
        BustPlacement {
            bg_scroll: scroll,
            palette,
            block_col: first_col,
            block_row: BLOCK_ROW,
            x: wx + x0,
            y: wy + y0,
        },
    ))
}

/// The background layer's pixel at screen (`sx`, `sy`) with the scroll
/// register at `scroll`, unflipped (render.rs `draw_tile_layer`, Layer::Bg).
/// `None` when the tilemap entry or the tile lies outside the regions.
fn background_pixel(
    assets: &Assets,
    palette: &Palette,
    tiles: &mut std::collections::HashMap<usize, [[u8; 16]; 16]>,
    scroll: u16,
    sx: usize,
    sy: usize,
) -> Option<Pixel> {
    debug_assert!(sx < WIDTH && sy < HEIGHT);
    let px_x = (sx + VISAREA_X + scroll as usize) % MAP_WIDTH_PX;
    let px_y = sy + VISAREA_Y;
    let (col, row) = (px_x / TILE, px_y / TILE);
    let base = 2 * (col * MAP_ROWS + row);
    let colour = (*assets.tilerom.get(base)? & 0x0f) as usize;
    let code = ((*assets.tilerom.get(base + TILEROM_HALF + 1)? as usize) << 8)
        | *assets.tilerom.get(base + 1)? as usize;
    let attr = *assets.tilerom.get(base + TILEROM_HALF)?;
    // The renderer wraps codes past the region (MAME's modulo); a bust
    // pointing there is damaged data.
    if code >= assets.gfx1.len() / TILE_BYTES {
        return None;
    }
    let tile = tiles
        .entry(code)
        .or_insert_with(|| decode_sprite_tile(&assets.gfx1, code));
    let (tx, ty) = (px_x % TILE, px_y % TILE);
    let tx = if attr & 1 != 0 { TILE - 1 - tx } else { tx };
    let ty = if attr & 2 != 0 { TILE - 1 - ty } else { ty };
    let pen = tile[ty][tx];
    Some(Pixel {
        pen,
        argb: palette.argb(colour * 16 + pen as usize),
    })
}
