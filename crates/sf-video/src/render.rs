//! Frame rendering: compose the background, foreground, sprite, and text
//! layers into a 384x224 frame, exactly as screen_update in the pinned
//! MAME source orders them.
//!
//! - bg tilemap (gfx1): 2048x16 column-major 16px tiles, opaque.
//! - fg tilemap (gfx2): same geometry from tilerom base 0x20000, pen 15
//!   transparent.
//! - Each scroll layer reads the map page its scene names: 0 is the ROM
//!   tilemap, the extended background bank's pages follow (ADR-0009).
//! - sprites (gfx3): object-RAM list, pen 15 transparent, code inverted
//!   per the driver's `invert()`.
//! - tx tilemap (gfx4): 64x32 row-major 8px tiles from video RAM, pen 3
//!   transparent.

use crate::background_bank::PAGE_BYTES;
use crate::decode::{decode_char_tile, decode_sprite_tile};
use crate::compositor::PaletteTint;
use crate::palette::Palette;
use crate::romset::Assets;
use crate::scene::SceneState;

pub const WIDTH: usize = 384;
pub const HEIGHT: usize = 224;

/// Visible-area origin within the 512x256 screen bitmap
/// (`screen.set_visarea(8*8, (64-8)*8-1, 2*8, 30*8-1)` @ mame0289):
/// the bitmap pixel at (64, 16) is screen pixel (0, 0).
pub const VISAREA_X: usize = 64;
pub const VISAREA_Y: usize = 16;

/// A rendered frame at game resolution, before display scaling.
#[derive(Clone)]
pub struct Frame {
    pub argb: Vec<u32>,
}

/// Palette index base of each gfx region's color codes (GFXDECODE_START).
const GFX1_PALETTE_BASE: usize = 0;
const GFX2_PALETTE_BASE: usize = 256;
const GFX3_PALETTE_BASE: usize = 512;
const GFX4_PALETTE_BASE: usize = 768;

const BG_FG_COLS: usize = 2048;
const BG_FG_ROWS: usize = 16;
const BG_FG_TILE: usize = 16;
const TX_COLS: usize = 64;
const TX_ROWS: usize = 32;

/// Changes to the presented text layer. The scene's video RAM is never
/// written: a hidden cell is skipped when the layer is composed, so what
/// the cell covered shows through, and an added tile is drawn as the layer's
/// own tile at a screen position.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextEdits {
    /// Cells to leave out: a text row and an inclusive column range.
    pub hidden: Vec<TextSpan>,
    /// Text tiles to draw after the layer: the screen pixel of the tile's
    /// top-left corner and the video-RAM word (code, flips, colour).
    pub added: Vec<(i32, i32, u16)>,
}

/// A run of text-layer cells on one row (`col_first..=col_last`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSpan {
    pub row: usize,
    pub col_first: usize,
    pub col_last: usize,
}

impl TextEdits {
    fn hides(&self, col: usize, row: usize) -> bool {
        self.hidden
            .iter()
            .any(|s| s.row == row && (s.col_first..=s.col_last).contains(&col))
    }
}

pub fn render_frame(assets: &Assets, scene: &SceneState) -> Frame {
    render_frame_edited(assets, scene, &TextEdits::default())
}

/// `render_frame` with the text layer's `edits` applied to the presented
/// frame only.
pub fn render_frame_edited(assets: &Assets, scene: &SceneState, edits: &TextEdits) -> Frame {
    let pens = compose_pens(assets, scene, edits, &[]);
    frame_from_pens(scene, &pens)
}

/// `render_frame_edited`, and which pixels are stage: drawn last by the bg
/// or fg layer, not a sprite or the text layer. The overhaul presenter's
/// super dim darkens only those.
pub fn render_frame_staged(assets: &Assets, scene: &SceneState, edits: &TextEdits) -> (Frame, Vec<bool>) {
    render_frame_staged_with_tints(assets, scene, edits, &[])
}

/// A tint applied to the sprite entries in an object RAM byte range.
pub struct SpriteTint {
    pub entries: std::ops::Range<usize>,
    pub tint: PaletteTint,
}

// Native pens occupy the low bits. Reserve the high nibble for an optional
// sprite tint index while retaining the original sprite and text draw order.
const TINT_SHIFT: usize = 12;
const PEN_MASK: u16 = (1 << TINT_SHIFT) - 1;

pub fn render_frame_staged_with_tints(
    assets: &Assets,
    scene: &SceneState,
    edits: &TextEdits,
    tints: &[SpriteTint],
) -> (Frame, Vec<bool>) {
    let tints = &tints[..tints.len().min(15)];
    let pens = compose_pens(assets, scene, edits, tints);
    let stage = pens.iter().map(|&pen| (pen as usize) < GFX3_PALETTE_BASE).collect();
    let frame = if tints.is_empty() {
        frame_from_pens(scene, &pens)
    } else {
        let palette = Palette::from_ram(&scene.paletteram);
        let argb = pens.iter().map(|&pen| {
            let pixel = palette.argb(usize::from(pen & PEN_MASK));
            match usize::from(pen >> TINT_SHIFT) {
                0 => pixel,
                tag => tints[tag - 1].tint.apply(pixel),
            }
        }).collect();
        Frame { argb }
    };
    (frame, stage)
}

fn compose_pens(assets: &Assets, scene: &SceneState, edits: &TextEdits, tints: &[SpriteTint]) -> Vec<u16> {
    let mut pens = vec![0u16; WIDTH * HEIGHT];

    if scene.active & 0x20 != 0 {
        draw_tile_layer(&mut pens, assets, scene, Layer::Bg);
    }
    if scene.active & 0x40 != 0 {
        draw_tile_layer(&mut pens, assets, scene, Layer::Fg);
    }
    if scene.active & 0x80 != 0 {
        draw_sprites(&mut pens, assets, scene, tints);
    }
    if scene.active & 0x08 != 0 {
        draw_text_layer(&mut pens, assets, scene, edits);
    }
    pens
}

fn frame_from_pens(scene: &SceneState, pens: &[u16]) -> Frame {
    let palette = Palette::from_ram(&scene.paletteram);
    let mut argb = vec![0u32; WIDTH * HEIGHT];
    for (px, pen) in argb.iter_mut().zip(pens.iter()) {
        *px = palette.argb(*pen as usize);
    }
    Frame { argb }
}

enum Layer {
    Bg,
    Fg,
}

fn draw_tile_layer(pens: &mut [u16], assets: &Assets, scene: &SceneState, layer: Layer) {
    let (tiles, palette_base, transparent_pen, scroll, page, layer_base) = match layer {
        Layer::Bg => (
            &assets.gfx1,
            GFX1_PALETTE_BASE,
            None,
            scene.bg_scroll,
            scene.bg_page,
            0usize,
        ),
        Layer::Fg => (
            &assets.gfx2,
            GFX2_PALETTE_BASE,
            Some(15u8),
            scene.fg_scroll,
            scene.fg_page,
            0x20000usize,
        ),
    };
    // The page's block of the tilemap (ADR-0009): page 0 is the ROM's, the
    // extended background bank's follow it. Without the bank every page
    // reads the ROM's.
    let tilerom_base = PAGE_BYTES * (page as usize % assets.background_pages().max(1)) + layer_base;
    // Tile codes beyond the region wrap, as MAME's gfx element data access
    // reduces the code modulo the element count. A sprite_layout tile
    // spans 64 bytes in each of the two plane halves: 128 bytes per tile.
    let tile_count = tiles.len() / 128;
    // Decode each tile code once per frame; tilemaps reuse codes heavily.
    let mut cache: std::collections::HashMap<usize, [[u8; 16]; 16]> =
        std::collections::HashMap::new();
    let map_width_px = BG_FG_COLS * BG_FG_TILE;
    // tilemap.cpp:1010-1012: xextent = visarea.right() + visarea.left() + 1 = 512,
    // yextent = visarea.bottom() + visarea.top() + 1 = 256 (equal to BG/FG m_height).
    // tilemap.cpp:27-46 (effective_rowscroll) & 55-74 (effective_colscroll):
    // With m_dx = m_dx_flipped = m_dy = m_dy_flipped = 0 and m_colscroll[0] = 0:
    // - Unflipped: value = -(scroll as i32); clamped to 0..map_width_px.
    // - Flipped (TILEMAP_FLIPX | TILEMAP_FLIPY): value = xextent - map_width_px + scroll;
    //   effective_colscroll = (yextent - m_height) mod m_height = 0.
    let xextent = (2 * VISAREA_X + WIDTH) as i32;
    let scroll_value = if scene.flip_screen {
        xextent - map_width_px as i32 + scroll as i32
    } else {
        -(scroll as i32)
    };
    let scrollx = scroll_value.rem_euclid(map_width_px as i32) as usize;
    for sy in 0..HEIGHT {
        for sx in 0..WIDTH {
            // draw_instance (tilemap.cpp:1289-1292): convert bitmap coordinates
            // (bx, by) = (sx + VISAREA_X, sy + VISAREA_Y) to pixmap coordinates.
            let px_x = (sx + VISAREA_X + map_width_px - scrollx) % map_width_px;
            let px_y = sy + VISAREA_Y;
            let (pcol, prow) = (px_x / BG_FG_TILE, px_y / BG_FG_TILE);
            // mappings_update (tilemap.cpp:728-737): flip logical tile (col, row)
            // under TILEMAP_FLIPX | TILEMAP_FLIPY.
            let (col, row) = if scene.flip_screen {
                (BG_FG_COLS - 1 - pcol, BG_FG_ROWS - 1 - prow)
            } else {
                (pcol, prow)
            };
            // TILEMAP_SCAN_COLS: column-major tile indices.
            let tile_index = col * BG_FG_ROWS + row;
            let base = tilerom_base + 2 * tile_index;
            let color = assets.tilerom[base] as usize & 0x0f;
            let code = ((assets.tilerom[base + 0x10000 + 1] as usize) << 8)
                | assets.tilerom[base + 1] as usize;
            let attr = assets.tilerom[base + 0x10000];
            // tile_update (tilemap.cpp:806): flags = m_tileinfo.flags ^ (m_attributes & 0x03).
            let (flipx, flipy) = (
                (attr & 1 != 0) ^ scene.flip_screen,
                (attr & 2 != 0) ^ scene.flip_screen,
            );
            let tile = cache
                .entry(code)
                .or_insert_with(|| decode_sprite_tile(tiles, code % tile_count));
            let (tx, ty) = (px_x % BG_FG_TILE, px_y % BG_FG_TILE);
            let (tx, ty) = (
                if flipx { BG_FG_TILE - 1 - tx } else { tx },
                if flipy { BG_FG_TILE - 1 - ty } else { ty },
            );
            let pen = tile[ty][tx];
            if Some(pen) == transparent_pen {
                continue;
            }
            pens[sy * WIDTH + sx] = (palette_base + color * 16 + pen as usize) as u16;
        }
    }
}

fn draw_text_layer(pens: &mut [u16], assets: &Assets, scene: &SceneState, edits: &TextEdits) {
    // For the 64x32 8x8 text tilemap, m_width = 512 = xextent, m_height = 256 = yextent,
    // and scroll is 0, so effective_rowscroll and effective_colscroll (tilemap.cpp:27-74)
    // are 0 in both orientations. mappings_update (tilemap.cpp:728-737) maps logical
    // (tx, ty) to pixmap (TX_COLS - 1 - tx, TX_ROWS - 1 - ty) when flipped, and
    // tile_update (tilemap.cpp:806) XORs TILE_FLIPX | TILE_FLIPY into each tile's flip flags.
    for ty in 0..TX_ROWS {
        for tx in 0..TX_COLS {
            if edits.hides(tx, ty) {
                continue;
            }
            // TILEMAP_SCAN_ROWS: row-major indices over big-endian u16 words.
            let idx = ty * TX_COLS + tx;
            let word = u16::from_be_bytes([scene.videoram[2 * idx], scene.videoram[2 * idx + 1]]);
            let code = (word & 0x3ff) as usize;
            let color = (word >> 12) as usize;
            let (flipx, flipy) = (
                (word & 0x400 != 0) ^ scene.flip_screen,
                (word & 0x800 != 0) ^ scene.flip_screen,
            );
            let (ptx, pty) = if scene.flip_screen {
                (TX_COLS - 1 - tx, TX_ROWS - 1 - ty)
            } else {
                (tx, ty)
            };
            let tile = decode_char_tile(&assets.gfx4, code);
            for py in 0..8 {
                for px in 0..8 {
                    let (cx, cy) = (
                        if flipx { 7 - px } else { px },
                        if flipy { 7 - py } else { py },
                    );
                    let pen = tile[cy][cx];
                    if pen == 3 {
                        continue; // text layer transparent pen
                    }
                    // Tilemap pixel to screen pixel via the visible area.
                    let sx = (ptx * 8 + px) as i32 - VISAREA_X as i32;
                    let sy = (pty * 8 + py) as i32 - VISAREA_Y as i32;
                    if (0..WIDTH as i32).contains(&sx) && (0..HEIGHT as i32).contains(&sy) {
                        // GFXDECODE color granularity follows the bit depth:
                        // the 2bpp text layer strides the palette by 4 pens
                        // per color, the 4bpp layers by 16.
                        pens[sy as usize * WIDTH + sx as usize] =
                            (GFX4_PALETTE_BASE + color * 4 + pen as usize) as u16;
                    }
                }
            }
        }
    }

    // Added tiles, upright at their screen pixel, over the layer.
    for &(x, y, word) in &edits.added {
        let code = (word & 0x3ff) as usize;
        let color = (word >> 12) as usize;
        let tile = decode_char_tile(&assets.gfx4, code);
        for py in 0..8 {
            for px in 0..8 {
                let (cx, cy) = (
                    if word & 0x400 != 0 { 7 - px } else { px },
                    if word & 0x800 != 0 { 7 - py } else { py },
                );
                let pen = tile[cy][cx];
                if pen == 3 {
                    continue;
                }
                let (sx, sy) = (x + px as i32, y + py as i32);
                if (0..WIDTH as i32).contains(&sx) && (0..HEIGHT as i32).contains(&sy) {
                    pens[sy as usize * WIDTH + sx as usize] =
                        (GFX4_PALETTE_BASE + color * 4 + pen as usize) as u16;
                }
            }
        }
    }
}

/// The driver's `invert()`: bit-twiddle applied to every sprite tile code.
pub(crate) fn invert(code: u16) -> u16 {
    const DELTA: [u16; 4] = [0x00, 0x18, 0x18, 0x00];
    code ^ DELTA[((code >> 3) & 3) as usize]
}

fn draw_sprites(pens: &mut [u16], assets: &Assets, scene: &SceneState, tints: &[SpriteTint]) {
    // Object RAM as big-endian u16 words; entries of 0x20 words each,
    // iterated from the top of the list downward (draw order matters).
    let word = |i: usize| -> u16 {
        u16::from_be_bytes([scene.objectram[2 * i], scene.objectram[2 * i + 1]])
    };
    let mut offs = 0x1000usize.wrapping_sub(0x20);
    loop {
        let code = word(offs);
        let attr = word(offs + 1);
        let sy = word(offs + 2) as i32;
        let sx = word(offs + 3) as i32;
        let tint_tag = tints.iter().position(|tint| tint.entries.contains(&(2 * offs)))
            .map_or(0, |index| (index + 1) << TINT_SHIFT);
        let palette_base = (GFX3_PALETTE_BASE + usize::from(attr & 0x000f) * 16) | tint_tag;
        let mut flipx = attr & 0x0100 != 0;
        let mut flipy = attr & 0x0200 != 0;

        if attr & 0x400 != 0 {
            // Large sprite: a 2x2 block of 16px tiles.
            let (mut sx, mut sy) = (sx, sy);
            if scene.flip_screen {
                sx = 480 - sx;
                sy = 224 - sy;
                flipx = !flipx;
                flipy = !flipy;
            }
            let mut quad = [code, code + 1, code + 16, code + 17];
            if flipx {
                quad.swap(0, 1);
                quad.swap(2, 3);
            }
            if flipy {
                quad.swap(0, 2);
                quad.swap(1, 3);
            }
            for (i, c) in quad.iter().enumerate() {
                let dx = (i % 2) as i32 * 16;
                let dy = (i / 2) as i32 * 16;
                draw_sprite_tile(
                    pens,
                    assets,
                    invert(*c),
                    palette_base,
                    (flipx, flipy),
                    (sx + dx, sy + dy),
                );
            }
        } else {
            let (mut sx, mut sy) = (sx, sy);
            if scene.flip_screen {
                sx = 496 - sx;
                sy = 240 - sy;
                flipx = !flipx;
                flipy = !flipy;
            }
            draw_sprite_tile(pens, assets, invert(code), palette_base, (flipx, flipy), (sx, sy));
        }
        if offs == 0 {
            break;
        }
        offs -= 0x20;
    }
}

fn draw_sprite_tile(
    pens: &mut [u16],
    assets: &Assets,
    code: u16,
    palette_base: usize,
    flip: (bool, bool),
    pos: (i32, i32),
) {
    let (flipx, flipy) = flip;
    let (sx, sy) = pos;
    let tile_count = assets.gfx3.len() / 128;
    let tile = decode_sprite_tile(&assets.gfx3, (code as usize) % tile_count);
    for py in 0..16 {
        for px in 0..16 {
            let (cx, cy) = (
                if flipx { 15 - px } else { px },
                if flipy { 15 - py } else { py },
            );
            let pen = tile[cy][cx];
            if pen == 15 {
                continue; // sprite transparent pen
            }
            // Sprite coordinates are bitmap coordinates; the screen's
            // visible area starts at (VISAREA_X, VISAREA_Y).
            let (x, y) = (
                sx + px as i32 - VISAREA_X as i32,
                sy + py as i32 - VISAREA_Y as i32,
            );
            if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
                pens[y as usize * WIDTH + x as usize] =
                    (palette_base + usize::from(pen)) as u16;
            }
        }
    }
}
