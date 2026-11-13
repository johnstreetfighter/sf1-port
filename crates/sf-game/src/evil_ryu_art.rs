//! ROM-derived appearances for Evil Ryu and the private Kage fights.

use sf_engine::config::EvilRyuPlaytest;
use sf_video::background_bank::{BackgroundBank, LayerView, MapLayer, MAP_ROWS};
use sf_video::busts::BLOCK_TILES;
use sf_video::decode::decode_sprite_tile;
use sf_video::render::{SpriteArt, VISAREA_X};
use sf_video::romset::Assets;
use sf_video::sprite_bank::{encode_sprite_tile, sprite_slot};
use sf_video::vs_screens::SCREEN_COLUMNS;

const RYU_PALETTE: usize = 0x1ba68;
const PALETTE_BYTES: usize = 32;
const HADOKEN_PALETTE: usize = RYU_PALETTE + 4 * PALETTE_BYTES;
// The spike's Shakunetsu effect uses sprite colour row 14.
const SHAKUNETSU_PALETTE: usize = RYU_PALETTE + 14 * PALETTE_BYTES;
const EYE_PEN: u8 = 6;

// xRGB444. Deeper skin tones (1–5), a red eye (6), crimson accents (8–10),
// and near-black violet cloth (7, 11–14). Pen 15 is the ROM's transparent pen.
const PENS: [u16; 15] = [
    0x000, 0x430, 0x643, 0x865, 0xa76, 0xc98, 0xf43, 0x657, 0x300, 0x601, 0xa12, 0x112, 0x213,
    0x324, 0x546,
];

// Kage's story costume uses darker hair and warmer red cloth accents.
const KAGE_PENS: [u16; 15] = [
    0x000, 0x431, 0x654, 0x876, 0xa86, 0xdb9, 0xf63, 0x756, 0x201, 0x402, 0x713, 0x112, 0x224,
    0x436, 0x657,
];

// Hadoken flight shades (pens 1–6) and the impact highlight (pen 7).
const SHAKUNETSU_PENS: [u16; 7] = [0x500, 0x900, 0xd11, 0xf21, 0xf53, 0xfcc, 0xfa8];

// The VS portrait has two more skin pens than the body sprite.
const VS_PENS: [u16; 15] = [
    PENS[0], PENS[1], PENS[2], 0x754, PENS[3], PENS[4], PENS[5], 0xcb9, PENS[8], PENS[9], PENS[10],
    PENS[11], PENS[12], PENS[13], PENS[14],
];

// Visible eyes in the ROM's movement, guard, attack, and hit poses. A pair
// of points crossing a tile boundary is listed on its two respective tiles.
const EYE_PIXELS: &[(u16, &[(usize, usize)])] = &[
    (0x0001, &[(8, 11), (9, 11)]),
    (0x0004, &[(8, 12), (9, 12)]),
    (0x0007, &[(8, 13), (9, 13)]),
    (0x000a, &[(8, 11), (9, 11)]),
    (0x000d, &[(8, 12), (9, 12)]),
    (0x0061, &[(8, 13), (9, 13)]),
    (0x0064, &[(15, 14)]),
    (0x0065, &[(0, 14)]),
    (0x00c6, &[(7, 15), (8, 15)]),
    (0x00cc, &[(5, 11), (6, 11)]),
    (0x00cf, &[(6, 10), (7, 10)]),
    (0x00d1, &[(14, 0), (15, 0)]),
    (0x00da, &[(1, 3), (2, 3)]),
    (0x010f, &[(4, 10), (5, 10)]),
    (0x0121, &[(11, 11), (12, 11)]),
    (0x0124, &[(5, 11), (6, 11)]),
    (0x0128, &[(1, 14), (2, 14)]),
    (0x0161, &[(14, 10), (15, 10)]),
    (0x0168, &[(1, 14), (2, 14)]),
    (0x01a2, &[(2, 10), (3, 10)]),
    (0x01a4, &[(13, 10), (14, 10)]),
    (0x01a7, &[(11, 12), (12, 12)]),
    (0x0222, &[(0, 14), (1, 14)]),
    (0x0226, &[(1, 14), (2, 14)]),
    (0x0282, &[(4, 14), (5, 14)]),
    (0x0286, &[(10, 14), (11, 14)]),
    (0x028a, &[(0, 14), (1, 14)]),
    (0x028d, &[(7, 13), (8, 13)]),
    (0x02ef, &[(3, 11), (4, 11)]),
    (0x02f1, &[(8, 6), (9, 6)]),
    (0x0359, &[(3, 0), (4, 0)]),
    (0x03a1, &[(11, 13), (12, 13)]),
    (0x03a7, &[(13, 13), (14, 13)]),
    (0x03e1, &[(11, 10), (12, 10)]),
    (0x0427, &[(15, 14)]),
    (0x0428, &[(0, 14)]),
    (0x043d, &[(2, 1), (3, 1)]),
    (0x04b5, &[(15, 1)]),
    (0x04b6, &[(0, 1)]),
    (0x04ba, &[(8, 1), (9, 1)]),
    (0x04e1, &[(6, 9), (7, 9)]),
    (0x0516, &[(1, 10), (2, 10)]),
    (0x05a8, &[(4, 10), (5, 10)]),
    (0x0606, &[(2, 11), (3, 11)]),
    (0x060a, &[(0, 11), (1, 11)]),
    (0x060e, &[(2, 12), (3, 12)]),
    (0x0661, &[(15, 14)]),
    (0x0662, &[(0, 14)]),
];

/// Prepare sparse art for the Arcade awakening without changing the ROM.
pub(crate) fn satsui_art(assets: &Assets) -> (SpriteArt, SpriteArt) {
    let mut body = [0; 16];
    body[..PENS.len()].copy_from_slice(&PENS);
    let mut fire = std::array::from_fn(|pen| {
        let at = HADOKEN_PALETTE + pen * 2;
        u16::from_be_bytes([assets.maincpu[at], assets.maincpu[at + 1]])
    });
    fire[1..1 + SHAKUNETSU_PENS.len()].copy_from_slice(&SHAKUNETSU_PENS);
    let eyes = EYE_PIXELS.iter().map(|&(code, pixels)| eye_tile(assets, code, pixels));
    (SpriteArt::new(body, eyes), SpriteArt::new(fire, []))
}

fn eye_tile(assets: &Assets, code: u16, pixels: &[(usize, usize)]) -> (usize, [[u8; 16]; 16]) {
    let slot = sprite_slot(code);
    let mut pens = decode_sprite_tile(&assets.gfx3, slot);
    for &(x, y) in pixels {
        pens[y][x] = EYE_PEN;
    }
    (slot, pens)
}

/// Change only the spike's loaded assets. Ryu is the only fighter in the
/// demo who uses this body palette and these tiles.
pub(crate) fn apply(assets: &mut Assets, profile: EvilRyuPlaytest) {
    let pens = if profile.is_kage() { KAGE_PENS } else { PENS };
    for (pen, word) in pens.iter().enumerate() {
        let at = RYU_PALETTE + pen * 2;
        assets.maincpu[at..at + 2].copy_from_slice(&word.to_be_bytes());
    }
    assets.maincpu.copy_within(
        HADOKEN_PALETTE..HADOKEN_PALETTE + PALETTE_BYTES,
        SHAKUNETSU_PALETTE,
    );
    for (pen, word) in SHAKUNETSU_PENS.iter().enumerate() {
        let at = SHAKUNETSU_PALETTE + (pen + 1) * 2;
        assets.maincpu[at..at + 2].copy_from_slice(&word.to_be_bytes());
    }

    let plane_size = assets.gfx3.len() / 2;
    for &(code, pixels) in EYE_PIXELS {
        let (slot, pens) = eye_tile(assets, code, pixels);
        let [first, second] = encode_sprite_tile(&pens);
        let at = slot * first.len();
        assets.gfx3[at..at + first.len()].copy_from_slice(&first);
        let at = at + plane_size;
        assets.gfx3[at..at + second.len()].copy_from_slice(&second);
    }
}

/// Recolour the original Ryu portrait in the spike's own copy of the VS
/// page. The portrait and the nameplates can share the ROM's colour row.
pub(crate) fn apply_vs_bust(
    rom: &Assets,
    bank: &mut BackgroundBank,
    view: LayerView,
    profile: EvilRyuPlaytest,
) -> LayerView {
    let first = (usize::from(view.scroll) + VISAREA_X) / 16;
    let left = first + 4;
    let top = 3;
    let mut used = [false; 16];
    for col in first..first + SCREEN_COLUMNS {
        for row in 0..MAP_ROWS {
            let cell = bank.cell(view.page, MapLayer::Bg, col, row);
            used[usize::from(cell.colour & 0x0f)] = true;
        }
    }
    let mut free = (1u8..16).filter(|&row| !used[usize::from(row)]);
    let bust_row = free.next().expect("a free VS portrait colour row");
    let eyes_row = free.next().expect("a free VS eye colour row");

    let mut palette = bank.palette(view.palette).expect("the VS palette").to_vec();
    let source = usize::from(bank.cell(view.page, MapLayer::Bg, left, top).colour & 0x0f) * 32;
    let kage = profile.is_kage();
    let mut bust_pens = VS_PENS;
    if kage {
        bust_pens[1..3].copy_from_slice(&KAGE_PENS[1..3]);
        bust_pens[4..7].copy_from_slice(&KAGE_PENS[3..6]);
        bust_pens[7] = 0xfdb;
        bust_pens[8..].copy_from_slice(&KAGE_PENS[8..]);
    }
    for row in [bust_row, eyes_row] {
        let start = usize::from(row) * 32;
        // Keep the original backdrop pen on the recoloured tiles.
        palette.copy_within(source..source + 32, start);
        for (pen, word) in bust_pens.iter().enumerate() {
            let at = start + pen * 2;
            palette[at..at + 2].copy_from_slice(&word.to_be_bytes());
        }
    }
    // The eye tiles leave pen 11 free for the same red highlight used in a fight.
    let highlight = usize::from(eyes_row) * 32 + 11 * 2;
    let eye_pen = if kage { KAGE_PENS[EYE_PEN as usize] } else { PENS[EYE_PEN as usize] };
    palette[highlight..highlight + 2].copy_from_slice(&eye_pen.to_be_bytes());

    for col in left..left + BLOCK_TILES {
        for row in top..top + BLOCK_TILES {
            let mut cell = bank.cell(view.page, MapLayer::Bg, col, row);
            // Bounds are in each unflipped ROM tile. Keep the brow, outline,
            // and dark pupil, tinting only the eye whites and iris highlights.
            let eye = match cell.code {
                0x0019 => Some((10..=15, 3..=5)),
                0x001a => Some((11..=15, 2..=5)),
                0x001b => Some((0..=3, 2..=5)),
                _ => None,
            };
            if kage || eye.is_some() {
                let mut pens = decode_sprite_tile(&rom.gfx1, usize::from(cell.code));
                if let Some((xs, ys)) = &eye {
                    for y in ys.clone() {
                        for x in xs.clone() {
                            pens[y][x] = match pens[y][x] {
                                7 => 11,
                                14 => 10,
                                pen => pen,
                            };
                        }
                    }
                }
                let detail = kage && add_kage_bust_detail(&mut pens, col - left, row - top, cell.attr);
                if detail || eye.is_some() {
                    cell.code = bank.add_tile(MapLayer::Bg, &pens);
                }
            }
            let palette_row = if eye.is_some() { eyes_row } else { bust_row };
            cell.colour = cell.colour & 0xf0 | palette_row;
            bank.set_cell(view.page, MapLayer::Bg, col, row, cell);
        }
    }

    LayerView {
        palette: bank.add_palette(&palette),
        ..view
    }
}

/// Horn profiles and fangs drawn into the existing portrait tiles.
fn add_kage_bust_detail(pens: &mut [[u8; 16]; 16], col: usize, row: usize, attr: u8) -> bool {
    const HORNS: [[(usize, usize); 15]; 2] = [
        [(39, 1), (39, 2), (40, 2), (40, 3), (41, 3), (41, 4), (42, 4), (42, 5),
         (43, 5), (43, 6), (44, 6), (44, 7), (45, 6), (46, 5), (47, 3)],
        [(71, 1), (70, 2), (70, 3), (70, 3), (70, 4), (70, 4), (69, 5), (69, 5),
         (69, 5), (69, 6), (69, 6), (69, 6), (69, 6), (70, 4), (71, 3)],
    ];
    let mut changed = false;
    let mut pixel = |x: usize, y: usize, pen: u8| {
        if x / 16 == col && y / 16 == row {
            let sx = if attr & 1 == 0 { x % 16 } else { 15 - x % 16 };
            let sy = if attr & 2 == 0 { y % 16 } else { 15 - y % 16 };
            pens[sy][sx] = pen;
            changed = true;
        }
    };
    for horn in HORNS {
        for (y, (start, width)) in horn.into_iter().enumerate() {
            for x in 0..width {
                let pen = if x + 1 == width && width > 1 { 4 } else if x < 2 { 7 } else { 6 };
                pixel(start + x, y + 9, pen);
            }
        }
    }
    for (x, y) in [(48, 53), (48, 54), (49, 54), (57, 53), (56, 54), (57, 54)] {
        pixel(x, y, 7);
    }
    changed
}
