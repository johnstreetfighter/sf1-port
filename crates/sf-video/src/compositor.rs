//! Extended scene compositor for overhaul play (`#99`, `O8`).
//!
//! Faithful 1987 never enters this module. The hardware renderer in
//! [`crate::render`] stays the only path that comparison baselines use.
//! Overhaul frames start from that same `384×224` image, then add sprites
//! and a HUD the 1987 sprite list cannot hold: more than 64 sprites, and a
//! private 16-colour palette on each sprite (15 colours plus transparent).
//! The era gauges are drawn by [`crate::era_hud`], the call-outs and their
//! screen effects by [`crate::callouts`].

use crate::callouts::{self, CalloutFrame};
use crate::era_hud;
pub use crate::era_hud::{EraHud, GaugeArt};
use crate::render::{Frame, HEIGHT, WIDTH};

const HUD_INK: u32 = 0xff_e8_d8_b0;

/// Which presenter is allowed to add overhaul layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentMode {
    /// Return the hardware frame unchanged. Extra sprites and the HUD are ignored.
    Faithful,
    /// Hardware frame, then extended sprites, then the HUD.
    Overhaul,
}

/// One 16×16 overhaul sprite. Pen `0` is transparent. Pens `1..=15` index
/// `palette`. The palette is private to this sprite, so the frame is not
/// limited to the hardware's 16 sprite sub-palettes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSprite {
    pub x: i32,
    pub y: i32,
    pub pixels: [u8; 256],
    pub palette: [u32; 16],
}

impl IndexedSprite {
    pub fn solid(x: i32, y: i32, color: u32) -> Self {
        let mut pixels = [0u8; 256];
        pixels[0] = 1;
        let mut palette = [0u32; 16];
        palette[1] = color;
        Self {
            x,
            y,
            pixels,
            palette,
        }
    }
}

/// Blend each visible palette pen toward an RGB flash colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteTint {
    pub color: u32,
    pub strength: u8,
}

impl PaletteTint {
    /// Luminance greyscale for native fighter sprites during Burnout.
    pub const GREY: Self = Self { color: 0, strength: 0 };

    pub fn apply(self, pixel: u32) -> u32 {
        if self == Self::GREY { return callouts::grey(pixel); }
        let alpha = u32::from(self.strength);
        let channel = |shift: u32| {
            let original = (pixel >> shift) & 0xffu32;
            let tint = (self.color >> shift) & 0xffu32;
            ((original * (255 - alpha) + tint * alpha + 127) / 255) << shift
        };
        0xff00_0000 | channel(16) | channel(8) | channel(0)
    }
}

/// Three recent presented poses, newest first. The caller controls the
/// sampling interval and clears the trail when the action ends.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AfterimageTrail {
    poses: [Vec<IndexedSprite>; 3],
    color: Option<u32>,
}

impl AfterimageTrail {
    pub fn record(&mut self, sprites: &[IndexedSprite]) {
        self.poses.rotate_right(1);
        self.poses[0].clear();
        self.poses[0].extend_from_slice(sprites);
    }

    pub fn record_colored(&mut self, sprites: Vec<IndexedSprite>, color: u32) {
        self.poses.rotate_right(1);
        self.poses[0] = sprites;
        self.color = Some(color);
    }

    pub fn clear(&mut self) {
        self.color = None;
        self.poses.iter_mut().for_each(Vec::clear);
    }

    fn draw(&self, argb: &mut [u32], stage: Option<&[bool]>) {
        for (pose, strength) in self.poses.iter().zip([144, 96, 48]).rev() {
            for sprite in pose {
                blit_sprite_with(argb, sprite, stage, |color, behind| PaletteTint {
                    color: self.color.unwrap_or_else(|| callouts::grey(color)),
                    strength,
                }.apply(behind));
            }
        }
    }
}

/// Layers the overhaul presenter may add. Faithful play ignores the struct.
#[derive(Clone)]
pub struct OverhaulScene<'a> {
    pub sprites: &'a [IndexedSprite],
    /// The era gauges, drawn in the bottom corners in `gauge_art`.
    pub hud: EraHud,
    pub gauge_art: GaugeArt,
    /// Rows the gauge corners sit above the bottom, for the Training frame
    /// meter (see [`era_hud::lift_above_rows`]).
    pub hud_lift: usize,
    /// The ROM's text tiles, for HUD lettering. Without it the level
    /// numbers are left out.
    pub font: Option<&'a RomFont<'a>>,
    /// The call-outs, and the super dim, Burnout grey and gauge flash.
    pub callouts: CalloutFrame,
    /// Each side's fighter drawn as overhaul sprites (P1, P2), over the
    /// dim; a greyed side's palette is drawn grey.
    pub fighters: [&'a [IndexedSprite]; 2],
    pub fighter_tints: [Option<PaletteTint>; 2],
    pub afterimages: [Option<&'a AfterimageTrail>; 2],
    /// The hardware pixels that are stage (the bg and fg layers), which the
    /// super dim darkens. `None` dims the whole hardware frame.
    pub stage: Option<&'a [bool]>,
    /// Character and style select lines, drawn as pixel type from the top.
    pub select_lines: &'a [String],
    /// Training overlay. `0` is transparent. Any other ARGB value is copied.
    /// The buffer is one `384×224` frame.
    pub training: Option<&'a [u32]>,
}

impl OverhaulScene<'static> {
    pub const EMPTY: Self = Self {
        sprites: &[],
        hud: EraHud::NONE,
        gauge_art: GaugeArt::Ruled,
        hud_lift: 0,
        font: None,
        callouts: CalloutFrame::NONE,
        fighters: [&[], &[]],
        fighter_tints: [None; 2],
        afterimages: [None; 2],
        stage: None,
        select_lines: &[],
        training: None,
    };
}

/// Compose one presented frame.
///
/// `Faithful` clones `hardware` and does not read `scene`.
pub fn present(mode: PresentMode, hardware: &Frame, scene: &OverhaulScene<'_>) -> Frame {
    debug_assert_eq!(hardware.argb.len(), WIDTH * HEIGHT);
    if mode == PresentMode::Faithful {
        return Frame {
            argb: hardware.argb.clone(),
        };
    }
    let mut argb = hardware.argb.clone();
    if scene.callouts.dim {
        callouts::dim(&mut argb, scene.stage);
    }
    for sprite in scene.sprites {
        blit_sprite(&mut argb, sprite);
    }
    for trail in scene.afterimages.iter().flatten() {
        trail.draw(&mut argb, scene.stage);
    }
    for (side, fighter) in scene.fighters.iter().enumerate() {
        let greyed = scene.callouts.sides[side].greyed;
        for sprite in *fighter {
            let tint = scene.fighter_tints[side];
            if greyed || tint.is_some() {
                let mut colored = sprite.clone();
                colored.palette = colored.palette.map(|pen| {
                    let pen = tint.map_or(pen, |tint| tint.apply(pen));
                    if greyed { callouts::grey(pen) } else { pen }
                });
                blit_sprite(&mut argb, &colored);
            } else {
                blit_sprite(&mut argb, sprite);
            }
        }
    }
    era_hud::draw_lifted(&mut argb, &scene.callouts.gauges(scene.hud), scene.gauge_art, scene.font, scene.hud_lift);
    callouts::draw(&mut argb, &scene.callouts, scene.font);
    blit_select(&mut argb, scene.select_lines);
    if let Some(training) = scene.training {
        blit_training(&mut argb, training);
    }
    Frame { argb }
}

fn blit_sprite(argb: &mut [u32], sprite: &IndexedSprite) {
    blit_sprite_with(argb, sprite, None, |color, _| color);
}

fn blit_sprite_with(
    argb: &mut [u32],
    sprite: &IndexedSprite,
    mask: Option<&[bool]>,
    shade: impl Fn(u32, u32) -> u32,
) {
    for py in 0..16 {
        for px in 0..16 {
            let pen = sprite.pixels[py * 16 + px];
            if pen == 0 || pen > 15 {
                continue;
            }
            let x = sprite.x + px as i32;
            let y = sprite.y + py as i32;
            if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
                let index = y as usize * WIDTH + x as usize;
                if mask.is_none_or(|mask| mask[index]) {
                    argb[index] = shade(sprite.palette[pen as usize], argb[index]);
                }
            }
        }
    }
}

fn blit_select(argb: &mut [u32], lines: &[String]) {
    for (row, line) in lines.iter().enumerate() {
        let y = row * 6;
        if y + 5 > HEIGHT {
            break;
        }
        for (col, ch) in line.chars().enumerate() {
            let x = col * 4;
            if x + 3 > WIDTH {
                break;
            }
            blit_glyph_at(argb, x, y, &glyph(ch));
        }
    }
}

fn blit_glyph_at(argb: &mut [u32], x: usize, y: usize, rows_bits: &[u8; 5]) {
    for (dy, bits) in rows_bits.iter().enumerate() {
        for dx in 0..3 {
            if bits & (1 << (2 - dx)) == 0 {
                continue;
            }
            let px = x + dx;
            let py = y + dy;
            if px < WIDTH && py < HEIGHT {
                argb[py * WIDTH + px] = HUD_INK;
            }
        }
    }
}

fn blit_training(argb: &mut [u32], training: &[u32]) {
    if training.len() != argb.len() {
        return;
    }
    for (dst, src) in argb.iter_mut().zip(training.iter()) {
        if *src != 0 {
            *dst = *src;
        }
    }
}

fn glyph(ch: char) -> [u8; 5] {
    match ch.to_ascii_uppercase() {
        'A' => [0b111, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b111],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'Q' => [0b111, 0b101, 0b101, 0b111, 0b001],
        'R' => [0b111, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '<' => [0b001, 0b010, 0b100, 0b010, 0b001],
        '>' => [0b100, 0b010, 0b001, 0b010, 0b100],
        '\u{b7}' => [0b000, 0b000, 0b010, 0b000, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '\\' => [0b100, 0b100, 0b010, 0b001, 0b001],
        '_' => [0b000, 0b000, 0b000, 0b000, 0b111],
        '(' => [0b001, 0b010, 0b010, 0b010, 0b001],
        ')' => [0b100, 0b010, 0b010, 0b010, 0b100],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        '\'' => [0b010, 0b010, 0b000, 0b000, 0b000],
        '?' => [0b111, 0b001, 0b010, 0b000, 0b010],
        _ => [0; 5],
    }
}

/// Text in the ROM-independent 3×5 font at an integer `scale`, one cell
/// every 4 pixels (times the scale), from (`x`, `y`). For screens drawn
/// with no ROM set (the ROM-missing screen). Lower case is drawn as upper
/// case and a character the font lacks as `?`.
pub fn fallback_text(argb: &mut [u32], x: usize, y: usize, text: &str, color: u32, scale: usize) {
    for (i, ch) in text.chars().enumerate() {
        let known = ch == ' ' || glyph(ch) != [0; 5];
        let rows = glyph(if known { ch } else { '?' });
        let x0 = x + i * 4 * scale;
        for (dy, bits) in rows.iter().enumerate() {
            for dx in 0..3 {
                if bits & (1 << (2 - dx)) == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let (px, py) = (x0 + dx * scale + sx, y + dy * scale + sy);
                        if px < WIDTH && py < HEIGHT {
                            argb[py * WIDTH + px] = color;
                        }
                    }
                }
            }
        }
    }
}

pub const SELECT_GOLD: u32 = 0xff_c4_94_30;
pub const SELECT_YELLOW: u32 = 0xff_f0_d8_20;
pub const SELECT_BLUE: u32 = 0xff_48_90_dc;
const SELECT_BG: u32 = 0xff_08_08_0c;
const SELECT_EMPTY: u32 = 0xff_14_10_10;

/// One bust, already in `0xAARRGGBB`. The select screen scales it into the tile and the large frame.
pub struct SelectPortrait<'a> {
    pub argb: &'a [u32],
    pub width: usize,
    pub height: usize,
}

/// Top-left of a roster tile. Three columns, four rows, slot 0 at the top left.
pub fn select_cell_origin(slot: usize) -> (usize, usize) {
    let slot = slot % 12;
    (188 + (slot % 3) * 62, 36 + (slot / 3) * 44)
}

/// Portrait character select: gold frame, blue "PLAYER SELECT", large bust on the left, 12 tiles on the right.
/// The yellow border sits on `cursor`. Empty tiles get the fighter's name.
pub fn render_player_select(
    names: &[&str; 12],
    cursor: usize,
    caption: &str,
    portraits: &[Option<SelectPortrait<'_>>; 12],
) -> Frame {
    let mut argb = vec![SELECT_BG; WIDTH * HEIGHT];
    rect(&mut argb, 4, 4, 376, 216, SELECT_GOLD);
    chamfer(&mut argb, 4, 4, 380, 220);
    blit_text(&mut argb, 140, 12, "PLAYER SELECT", SELECT_BLUE, 2);
    rect(&mut argb, 16, 38, 152, 116, SELECT_GOLD);
    blit_portrait(&mut argb, 18, 40, 148, 112, portraits.get(cursor % 12).and_then(|p| p.as_ref()));
    blit_text(&mut argb, 22, 162, caption, SELECT_BLUE, 2);
    for slot in 0..12 {
        let (x, y) = select_cell_origin(slot);
        let color = if slot == cursor % 12 { SELECT_YELLOW } else { SELECT_GOLD };
        rect(&mut argb, x, y, 56, 38, color);
        blit_portrait(&mut argb, x + 2, y + 2, 52, 34, portraits[slot].as_ref());
        if portraits[slot].is_none() {
            blit_text(&mut argb, x + 4, y + 16, names[slot], SELECT_BLUE, 1);
        }
    }
    Frame { argb }
}

fn rect(argb: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    for dx in 0..w {
        put(argb, x + dx, y, color);
        put(argb, x + dx, y + h - 1, color);
    }
    for dy in 0..h {
        put(argb, x, y + dy, color);
        put(argb, x + w - 1, y + dy, color);
    }
}

fn chamfer(argb: &mut [u32], x0: usize, y0: usize, x1: usize, y1: usize) {
    for i in 0..4 {
        put(argb, x0 + i, y0, SELECT_BG);
        put(argb, x0, y0 + i, SELECT_BG);
        put(argb, x1 - 1 - i, y0, SELECT_BG);
        put(argb, x1 - 1, y0 + i, SELECT_BG);
        put(argb, x0 + i, y1 - 1, SELECT_BG);
        put(argb, x0, y1 - 1 - i, SELECT_BG);
        put(argb, x1 - 1 - i, y1 - 1, SELECT_BG);
        put(argb, x1 - 1, y1 - 1 - i, SELECT_BG);
    }
}

fn put(argb: &mut [u32], x: usize, y: usize, color: u32) {
    if x < WIDTH && y < HEIGHT {
        argb[y * WIDTH + x] = color;
    }
}

fn blit_text(argb: &mut [u32], x: usize, y: usize, text: &str, color: u32, scale: usize) {
    let mut cx = x;
    for ch in text.chars() {
        let bits = glyph(ch);
        for (dy, row) in bits.iter().enumerate() {
            for dx in 0..3 {
                if row & (1 << (2 - dx)) == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        put(argb, cx + dx * scale + sx, y + dy * scale + sy, color);
                    }
                }
            }
        }
        cx += 4 * scale;
    }
}

fn blit_portrait(argb: &mut [u32], x: usize, y: usize, w: usize, h: usize, portrait: Option<&SelectPortrait<'_>>) {
    let Some(portrait) = portrait else {
        for dy in 0..h {
            for dx in 0..w {
                put(argb, x + dx, y + dy, SELECT_EMPTY);
            }
        }
        return;
    };
    if portrait.width == 0 || portrait.height == 0 || portrait.argb.len() < portrait.width * portrait.height {
        return;
    }
    for dy in 0..h {
        for dx in 0..w {
            let sx = dx * portrait.width / w;
            let sy = dy * portrait.height / h;
            put(argb, x + dx, y + dy, portrait.argb[sy * portrait.width + sx]);
        }
    }
}

/// A one-pixel `color` outline of the `w` × `h` box at (`x`, `y`).
pub fn outline(argb: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    rect(argb, x, y, w, h, color);
}

/// Quarter brightness over the `w` × `h` box at (`x`, `y`).
pub fn darken(argb: &mut [u32], x: usize, y: usize, w: usize, h: usize) {
    for yy in y..(y + h).min(HEIGHT) {
        for xx in x..(x + w).min(WIDTH) {
            let p = &mut argb[yy * WIDTH + xx];
            *p = 0xff00_0000 | ((*p >> 2) & 0x003f_3f3f);
        }
    }
}

/// `portrait` at its own size (no scaling) through the `w` × `h` window at
/// (`x`, `y`): the portrait's pixel (`src_x`, `src_y`) lands on the
/// window's top-left; pixels outside the portrait leave the frame as it is.
#[allow(clippy::too_many_arguments)]
pub fn blit_window(
    argb: &mut [u32],
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    portrait: &SelectPortrait<'_>,
    src_x: isize,
    src_y: isize,
) {
    for dy in 0..h {
        for dx in 0..w {
            let (sx, sy) = (src_x + dx as isize, src_y + dy as isize);
            if sx < 0 || sy < 0 || sx as usize >= portrait.width || sy as usize >= portrait.height {
                continue;
            }
            if let Some(&p) = portrait.argb.get(sy as usize * portrait.width + sx as usize) {
                put(argb, x + dx, y + dy, p);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Front-end screens drawn from the ROM's own text tiles (spec #150, FE2).
// ---------------------------------------------------------------------------

/// ROM address of the text-layer palette that boot uploads to palette byte
/// offset `0x600` (`0x3c0d2..0x3c10c`): sixteen colours of four pens, in the
/// palette RAM's `xRGB_444` words. Front-end screens draw with it.
pub const TEXT_PALETTE_ROM: usize = 0x1bc68;

/// The screen in 8×8 text cells.
pub const TEXT_COLS: usize = WIDTH / 8;
pub const TEXT_ROWS: usize = HEIGHT / 8;

/// Text colours of the boot palette that the front end uses.
pub mod ink {
    /// The 2×2 `COUNTRY SELECT` header and the `JAPAN` / `U.S.A.` labels.
    pub const BLUE: u8 = 5;
    /// The cursor row.
    pub const YELLOW: u8 = 6;
    /// P2's cursor on the select (light blue, dark blue shadow).
    pub const P2_BLUE: u8 = 8;
    /// Greyed rows and the preset line: the one colour the front end adds,
    /// since no boot text colour has a dim body (see [`super::DIM_PENS`]).
    pub const DIM: u8 = 16;
    /// The gold double frame alternates these two colours, as the ROM draws it.
    pub const GOLD_A: u8 = 13;
    pub const GOLD_B: u8 = 14;
}

/// The ROM's 8×8 font: `0`–`9` are tiles `0x00`–`0x09`, `A`–`Z` are
/// `0x0a`–`0x23` (the `INSERT COIN` block at `0x1457a` and the copyright
/// line at `0x14e9c` spell with them), punctuation follows, and `0x3c` is
/// the blank cell the ROM erases with.
pub fn small_glyph(ch: char) -> Option<u16> {
    Some(match ch {
        '0'..='9' => ch as u16 - '0' as u16,
        'A'..='Z' => 0x0a + (ch as u16 - 'A' as u16),
        ' ' => 0x3c,
        '©' => 0x24,
        '>' => 0x25,
        '!' => 0x26,
        '.' => 0x27,
        ':' => 0x28,
        '\'' => 0x29,
        ',' => 0x2a,
        '@' => 0x2b,
        '?' => 0x31,
        '#' => 0x35,
        '*' => 0x36,
        '%' => 0x39,
        '[' => 0x40,
        ']' => 0x41,
        '(' => 0x48,
        ')' => 0x49,
        _ => return None,
    })
}

/// The ROM's 2×2 header font (`text_block_2x2`, `0x3bc3e`): a cell's code
/// `c` draws tiles `c`, `c + 1` over `c + 0x10`, `c + 0x11`. `COUNTRY
/// SELECT` (`0x15d1e`) spells with the letters; digits and `. ? !` follow
/// them in the same tile rows.
pub fn header_glyph(ch: char) -> Option<u16> {
    let grid = |i: u16| (i / 8) * 0x20 + (i % 8) * 2;
    Some(match ch {
        'A'..='Z' => 0x180 + grid(ch as u16 - 'A' as u16),
        '0'..='9' => 0x200 + grid(ch as u16 - '0' as u16),
        '.' => 0x224,
        '?' => 0x226,
        '!' => 0x228,
        _ => return None,
    })
}

/// Glyphs the ROM's font lacks, drawn in its style: body rows (bit 7 is the
/// left pixel) in pen 0 with pen 1 as the shadow one pixel down-right.
const MADE_GLYPHS: &[(char, [u8; 8])] = &[
    ('+', [0x00, 0x18, 0x18, 0x7e, 0x7e, 0x18, 0x18, 0x00]),
    ('-', [0x00, 0x00, 0x00, 0x7c, 0x7c, 0x00, 0x00, 0x00]),
    ('_', [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7c, 0x00]),
    ('/', [0x02, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x40, 0x00]),
    // The middle dot, which sets the move list's normals line apart.
    ('\u{b7}', [0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00]),
    // The eight arrows of the move list's drawn inputs (ticket #228, ML2):
    // six pixels square at the cell's top left so the shadow keeps a pixel
    // between neighbours, a two-pixel stem and a head, the four diagonals
    // a thick stroke ending in a corner.
    ('\u{2192}', [0x10, 0x18, 0xfc, 0xfc, 0x18, 0x10, 0x00, 0x00]),
    ('\u{2190}', [0x20, 0x60, 0xfc, 0xfc, 0x60, 0x20, 0x00, 0x00]),
    ('\u{2193}', [0x30, 0x30, 0x30, 0xfc, 0x78, 0x30, 0x00, 0x00]),
    ('\u{2191}', [0x30, 0x78, 0xfc, 0x30, 0x30, 0x30, 0x00, 0x00]),
    ('\u{2198}', [0xc0, 0xe0, 0x70, 0x3c, 0x1c, 0x3c, 0x00, 0x00]),
    ('\u{2199}', [0x0c, 0x1c, 0x38, 0xf0, 0xe0, 0xf0, 0x00, 0x00]),
    ('\u{2197}', [0x3c, 0x1c, 0x3c, 0x70, 0xe0, 0xc0, 0x00, 0x00]),
    ('\u{2196}', [0xf0, 0xe0, 0xf0, 0x38, 0x1c, 0x0c, 0x00, 0x00]),
];

/// The button chip's letters: three pixels wide, five tall, knocked out of
/// the chip. Only the letters a button is named with.
fn chip_letter(ch: char) -> Option<[u8; 5]> {
    Some(match ch {
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        _ => return None,
    })
}

/// Whether [`RomFont::button`] can set `label`: one chip cell a letter.
pub fn button_drawable(label: &str) -> bool {
    !label.is_empty() && label.chars().all(|c| chip_letter(c).is_some())
}

/// The rows of cell `cell` of the chip for `label`: a block seven pixels
/// tall and one pixel short of its cells' width (room for the shadow),
/// corners cut, the letters knocked out of it.
fn chip_rows(label: &str, cell: usize) -> [u8; 8] {
    let letters: Vec<[u8; 5]> = label.chars().filter_map(chip_letter).collect();
    let width = 8 * letters.len() - 1;
    let first_letter = 2 * letters.len();
    let mut rows = [0u8; 8];
    for (y, row) in rows.iter_mut().enumerate().take(7) {
        for dx in 0..8 {
            let x = cell * 8 + dx;
            let corner = (y == 0 || y == 6) && (x == 0 || x == width - 1);
            let mut body = x < width && !corner;
            let at = x.wrapping_sub(first_letter);
            if let (Some(letter), true) = (letters.get(at / 4), at % 4 < 3) {
                if (1..6).contains(&y) && letter[y - 1] & (0b100 >> (at % 4)) != 0 {
                    body = false;
                }
            }
            if body {
                *row |= 0x80 >> dx;
            }
        }
    }
    rows
}

/// Whether [`RomFont::text`] draws `ch`: the ROM's 8×8 font or one of the
/// glyphs made in its style.
pub fn drawable(ch: char) -> bool {
    small_glyph(ch).is_some() || MADE_GLYPHS.iter().any(|(c, _)| *c == ch)
}

/// [`ink::DIM`]'s pens. In the ROM's font pen 2 is a letter's body, pen 0
/// its highlight and pen 1 its shadow; the boot colours all have bright
/// bodies, so the dim blue is made here: a dark blue body under a muted
/// highlight.
pub const DIM_PENS: [u32; 4] = [0xff_66_66_99, 0xff_11_11_33, 0xff_44_44_77, 0xff_ff_ff_ff];

/// Draws front-end screens with the ROM's text tiles (`gfx4`) and the boot
/// text palette, the way the text layer would: pen 3 is transparent.
pub struct RomFont<'a> {
    gfx4: &'a [u8],
    palette: [u32; 68],
}

impl<'a> RomFont<'a> {
    /// The font over the text tiles and the palette at [`TEXT_PALETTE_ROM`]
    /// in the 68000 program region.
    pub fn new(gfx4: &'a [u8], program_rom: &[u8]) -> Self {
        let mut palette = [0u32; 68];
        palette[64..].copy_from_slice(&DIM_PENS);
        for (i, entry) in palette.iter_mut().take(64).enumerate() {
            let at = TEXT_PALETTE_ROM + 2 * i;
            let v = program_rom
                .get(at..at + 2)
                .map_or(0, |b| u16::from_be_bytes([b[0], b[1]]));
            let c = |x: u16| ((x & 0xf) as u32) * 0x11;
            *entry = 0xff00_0000 | (c(v >> 8) << 16) | (c(v >> 4) << 8) | c(v);
        }
        RomFont { gfx4, palette }
    }

    /// The ARGB value of `pen` in text colour `color` (0–15 from the ROM,
    /// or [`ink::DIM`]).
    pub fn pen(&self, color: u8, pen: u8) -> u32 {
        self.palette[(color as usize % 17) * 4 + (pen as usize & 3)]
    }

    /// One 8×8 text tile at cell (`col`, `row`).
    pub fn tile(&self, argb: &mut [u32], col: usize, row: usize, code: u16, color: u8) {
        self.tile_px(argb, col * 8, row * 8, code, color);
    }

    /// One 8×8 text tile with its top-left at pixel (`x`, `y`).
    pub fn tile_px(&self, argb: &mut [u32], x: usize, y: usize, code: u16, color: u8) {
        let tile = crate::decode::decode_char_tile(self.gfx4, code as usize);
        for (ty, line) in tile.iter().enumerate() {
            for (tx, &pen) in line.iter().enumerate() {
                if pen != 3 {
                    put(argb, x + tx, y + ty, self.pen(color, pen));
                }
            }
        }
    }

    /// One 8×8 text tile with its top-left at pixel (`x`, `y`), in `pens`
    /// (pen 0, 1 and 2: a letter's highlight, shadow and body) rather than a
    /// palette colour.
    pub fn tile_in(&self, argb: &mut [u32], x: usize, y: usize, code: u16, pens: [u32; 3]) {
        let tile = crate::decode::decode_char_tile(self.gfx4, code as usize);
        for (ty, line) in tile.iter().enumerate() {
            for (tx, &pen) in line.iter().enumerate() {
                if pen != 3 {
                    put(argb, x + tx, y + ty, pens[pen as usize]);
                }
            }
        }
    }

    /// One character, the ROM's or a glyph made in its style, with its
    /// top-left at pixel (`x`, `y`) in `pens` (a letter's highlight, shadow
    /// and body; a made glyph is drawn in the body pen over the shadow one,
    /// as the letters' bodies are). Returns whether the font has the
    /// character.
    pub fn glyph_in(&self, argb: &mut [u32], x: usize, y: usize, ch: char, pens: [u32; 3]) -> bool {
        if let Some(code) = small_glyph(ch) {
            self.tile_in(argb, x, y, code, pens);
            return true;
        }
        let Some((_, rows)) = MADE_GLYPHS.iter().find(|(c, _)| *c == ch) else { return false };
        let body = |dx: usize, dy: usize| rows[dy] & (0x80 >> dx) != 0;
        for dy in 0..8 {
            for dx in 0..8 {
                if body(dx, dy) {
                    put(argb, x + dx, y + dy, pens[2]);
                } else if dx > 0 && dy > 0 && body(dx - 1, dy - 1) {
                    put(argb, x + dx, y + dy, pens[1]);
                }
            }
        }
        true
    }

    /// `text` in the 8×8 font from pixel (`x`, `y`), eight pixels per
    /// character (characters the font lacks are left blank).
    pub fn text_px(&self, argb: &mut [u32], x: usize, y: usize, text: &str, color: u8) {
        for (i, ch) in text.chars().enumerate() {
            if let Some(code) = small_glyph(ch) {
                self.tile_px(argb, x + 8 * i, y, code, color);
            } else if let Some((_, rows)) = MADE_GLYPHS.iter().find(|(c, _)| *c == ch) {
                self.made_glyph(argb, x + 8 * i, y, rows, color);
            }
        }
    }

    /// `text` in the 8×8 font from cell (`col`, `row`), one cell per
    /// character. Characters neither the ROM nor [`MADE_GLYPHS`] has are
    /// left blank.
    pub fn text(&self, argb: &mut [u32], col: usize, row: usize, text: &str, color: u8) {
        for (i, ch) in text.chars().enumerate() {
            if let Some(code) = small_glyph(ch) {
                self.tile(argb, col + i, row, code, color);
            } else if let Some((_, rows)) = MADE_GLYPHS.iter().find(|(c, _)| *c == ch) {
                self.made_glyph(argb, (col + i) * 8, row * 8, rows, color);
            }
        }
    }

    /// `text` centred on the screen's cell grid at `row`.
    pub fn text_centred(&self, argb: &mut [u32], row: usize, text: &str, color: u8) {
        let n = text.chars().count();
        self.text(argb, TEXT_COLS.saturating_sub(n) / 2, row, text, color);
    }

    /// A button's name (`P`, `LP`) in a small chip from cell (`col`, `row`),
    /// a cell a letter: a block in the text colour's body pen with the
    /// letters knocked out of it, made in the font's style. A label with a
    /// letter the chip has no shape for is set as plain text.
    pub fn button(&self, argb: &mut [u32], col: usize, row: usize, label: &str, color: u8) {
        if !button_drawable(label) {
            self.text(argb, col, row, label, color);
            return;
        }
        for cell in 0..label.chars().count() {
            self.made_glyph(argb, (col + cell) * 8, row * 8, &chip_rows(label, cell), color);
        }
    }

    fn made_glyph(&self, argb: &mut [u32], x: usize, y: usize, rows: &[u8; 8], color: u8) {
        let body = |dx: usize, dy: usize| rows[dy] & (0x80 >> dx) != 0;
        for dy in 0..8 {
            for dx in 0..8 {
                if body(dx, dy) {
                    put(argb, x + dx, y + dy, self.pen(color, 0));
                } else if dx > 0 && dy > 0 && body(dx - 1, dy - 1) {
                    put(argb, x + dx, y + dy, self.pen(color, 1));
                }
            }
        }
    }

    /// `text` in the 2×2 header font from cell (`col`, `row`): two cells
    /// per character, two rows tall. Characters the font lacks are left blank.
    pub fn header(&self, argb: &mut [u32], col: usize, row: usize, text: &str, color: u8) {
        for (i, ch) in text.chars().enumerate() {
            let Some(code) = header_glyph(ch) else { continue };
            let c = col + 2 * i;
            self.tile(argb, c, row, code, color);
            self.tile(argb, c + 1, row, code + 1, color);
            self.tile(argb, c, row + 1, code + 0x10, color);
            self.tile(argb, c + 1, row + 1, code + 0x11, color);
        }
    }

    /// The gold double frame of `COUNTRY SELECT` (`0x15d42`): `cols` × `rows`
    /// cells from (`col`, `row`), corners `0xca` / `0xcd` / `0xdc` / `0xdd`,
    /// edges alternating `0xcc` / `0xcb` (top and bottom), `0xda` / `0xce`
    /// (left) and `0xcf` / `0xdb` (right) in colours 13 and 14. The top edge
    /// leaves `gap` (frame-relative columns) open for a header.
    pub fn gold_frame(
        &self,
        argb: &mut [u32],
        col: usize,
        row: usize,
        cols: usize,
        rows: usize,
        gap: std::ops::Range<usize>,
    ) {
        use ink::{GOLD_A as D, GOLD_B as E};
        let (last_c, last_r) = (cols - 1, rows - 1);
        for c in 0..cols {
            let (top, bottom) = match c {
                0 => ((0xca, E), (0xdc, E)),
                c if c == last_c => ((0xcd, D), (0xdd, D)),
                c if c % 2 == 1 => ((0xcc, D), (0xcc, D)),
                _ => ((0xcb, E), (0xcb, E)),
            };
            if !gap.contains(&c) {
                self.tile(argb, col + c, row, top.0, top.1);
            }
            self.tile(argb, col + c, row + last_r, bottom.0, bottom.1);
        }
        for r in 1..last_r {
            let (left, right) = if r % 2 == 1 {
                ((0xda, D), (0xcf, E))
            } else {
                ((0xce, E), (0xdb, D))
            };
            self.tile(argb, col, row + r, left.0, left.1);
            self.tile(argb, col + last_c, row + r, right.0, right.1);
        }
    }
}
