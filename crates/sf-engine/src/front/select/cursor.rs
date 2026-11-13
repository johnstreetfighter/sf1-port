//! Pixel cursors for the portrait grid.

use sf_video::render::WIDTH;

/// Bright borders and pale label outlines, in 1P and 2P order.
pub(super) const COLORS: [u32; 2] = [0xff5a_81ff, 0xffff_634f];
pub(super) const LABEL_LIGHT: [u32; 2] = [0xffb7_cbff, 0xffff_c4af];

const DIM: [u32; 2] = [0xff34_4d94, 0xff94_3b3e];
const RAIL: [u32; 2] = [0xff13_1d46, 0xff45_1c21];
const LABEL_INK: [u32; 2] = [0xff24_48db, 0xffd9_2d25];

/// Inclusive horizontal spans of the upper-left bracket, mirrored at the
/// other corners. The tapered ends leave the middle of the portrait clear.
const CORNER: [(usize, usize); 12] = [
    (2, 11),
    (1, 10),
    (0, 10),
    (0, 9),
    (0, 3),
    (0, 3),
    (0, 3),
    (0, 3),
    (0, 3),
    (0, 2),
    (0, 1),
    (0, 0),
];

const DIGITS: [[u8; 5]; 2] = [
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b110, 0b001, 0b010, 0b100, 0b111],
];
const P: [u8; 5] = [0b110, 0b101, 0b110, 0b100, 0b100];

/// Draw one player's cursor. On a shared tile, 1P owns the left half and
/// 2P the right. Only the bracket accents flash.
pub(super) fn draw(
    argb: &mut [u32],
    tile: (usize, usize, usize, usize),
    side: usize,
    shared: bool,
    bright: bool,
) {
    let (x, y, w, h) = tile;
    let middle = x + w / 2;
    let right_edge = x + w - 1;
    let bottom = y + h - 1;
    let left = !shared || side == 0;
    let right = !shared || side == 1;
    let rail_left = if left { x } else { middle };
    let rail_right = if right { right_edge } else { middle - 1 };
    let color = if bright { COLORS[side] } else { DIM[side] };
    let highlight = if bright { LABEL_LIGHT[side] } else { color };

    for row in [y, bottom] {
        argb[row * WIDTH + rail_left..=row * WIDTH + rail_right].fill(RAIL[side]);
    }
    for (visible, edge, mirrored) in [(left, x, false), (right, right_edge, true)] {
        if visible {
            for row in y..=bottom {
                argb[row * WIDTH + edge] = RAIL[side];
            }
            draw_corner(argb, edge, y, mirrored, false, color, highlight);
            draw_corner(argb, edge, bottom, mirrored, true, color, highlight);
        }
    }

    // Labels sit one pixel above the top edge. On a shared tile each is
    // centered on its own half, with the extra pixel going to the right.
    let center = if shared && side == 0 {
        x + (w / 2) / 2
    } else if shared {
        middle + (w - w / 2 + 1) / 2
    } else {
        middle
    };
    draw_label(argb, center - 7, y - 1, side);
}

fn draw_corner(
    argb: &mut [u32],
    x: usize,
    y: usize,
    mirrored: bool,
    bottom: bool,
    color: u32,
    highlight: u32,
) {
    for (dy, (first, last)) in CORNER.into_iter().enumerate() {
        let row = if bottom { y - dy } else { y + dy };
        for dx in first..=last {
            let col = if mirrored { x - dx } else { x + dx };
            argb[row * WIDTH + col] = if dy == 0 && dx < 11 { highlight } else { color };
        }
    }
}

fn draw_label(argb: &mut [u32], x: usize, y: usize, side: usize) {
    let glyphs = [DIGITS[side], P];
    // Draw the outline for both glyphs before filling either one.
    for outline in [true, false] {
        for (i, glyph) in glyphs.into_iter().enumerate() {
            for (dy, bits) in glyph.into_iter().enumerate() {
                for dx in 0..3 {
                    if bits & (0b100 >> dx) == 0 {
                        continue;
                    }
                    let px = x + i * 8 + dx * 2;
                    let py = y + dy * 2;
                    let top = py * WIDTH + px;
                    let bottom = top + WIDTH;
                    if outline {
                        let pale = LABEL_LIGHT[side];
                        argb[top - WIDTH..top - WIDTH + 2].fill(pale);
                        argb[top - 1..top + 3].fill(pale);
                        argb[bottom - 1..bottom + 3].fill(pale);
                        argb[bottom + WIDTH..bottom + WIDTH + 2].fill(pale);
                    } else {
                        let ink = LABEL_INK[side];
                        argb[top..top + 2].fill(ink);
                        argb[bottom..bottom + 2].fill(ink);
                    }
                }
            }
        }
    }
}
