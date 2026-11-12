//! The era HUD's gauges (spec #216, `EH1`): a per-side description of what
//! to show, and its drawing in the bottom corners of an overhaul frame.
//!
//! Era models fill an [`EraGauge`] for their side from state they already
//! hold; the compositor draws the pair in a [`GaugeArt`]. Faithful frames
//! never come here.
//!
//! **Ruled** gauge art is drawn after the 1987 PLAYER1 health bar (text
//! tiles `0x120`–`0x125` in text colour 1): a lavender rule above and below,
//! a lavender cap at each end, a dark shadow row under the bar and a dark
//! interior where the bar is empty. The health bar's red fill becomes
//! PLAYER1 gold. Every pen is a boot text-palette entry (see [`pens`]).
//!
//! **Blocks** draws stocks as bevelled boxes after the round-win markers
//! (steel bevel pens from the ROM's marker palette, lit gold when full) and
//! a level gauge as a thin ruled line with its number in a small bevelled
//! box. **Plate** sets the bar in a gold frame with a light top edge and a
//! dark-gold shadow, stock cells split by dark gold, and `LV` with the
//! number in PLAYER1 gold. Both fit the same corner box as Ruled.

use crate::compositor::{small_glyph, RomFont};
use crate::render::{HEIGHT, WIDTH};

/// A portion of a whole: `num` out of `den`, clamped to `0..=1` when drawn.
/// A zero `den` is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub num: u16,
    pub den: u16,
}

impl Fill {
    pub const EMPTY: Fill = Fill { num: 0, den: 1 };
    pub const FULL: Fill = Fill { num: 1, den: 1 };

    pub const fn of(num: u16, den: u16) -> Self {
        Fill { num, den }
    }

    /// This portion of `width` pixels, rounded down.
    pub fn px(self, width: usize) -> usize {
        if self.den == 0 {
            return 0;
        }
        (width * self.num.min(self.den) as usize) / self.den as usize
    }
}

/// How a row is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GaugeState {
    /// Filling or spent: the plain fill.
    #[default]
    Normal,
    /// At its maximum.
    Full,
    /// Running down on a timer (an Alpha Custom Combo, an SF5 V-Trigger).
    Draining,
    /// Out of use (SF6 Burnout).
    Greyed,
    /// Flashing; `lit` is this frame's phase.
    Flashing { lit: bool },
}

/// One row of a corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeRow {
    /// A segmented bar of `cells` stocks: `full` of them filled, and `part`
    /// of the next one.
    Stock { cells: u8, full: u8, part: Fill, state: GaugeState },
    /// One long bar with `level` beside it: `fill` of the bar lit, the bar
    /// `length` of a full-length bar long.
    Level { level: u8, fill: Fill, length: Fill, state: GaugeState },
    /// One continuous bar without a level number.
    Bar { fill: Fill, length: Fill, state: GaugeState },
}

impl GaugeRow {
    /// `full` of `cells` stocks and `part` of the next. Full when every cell is.
    pub fn stock(cells: u8, full: u8, part: Fill) -> Self {
        let full = full.min(cells);
        let state = if full == cells { GaugeState::Full } else { GaugeState::Normal };
        let part = if full == cells { Fill::EMPTY } else { part };
        GaugeRow::Stock { cells, full, part, state }
    }

    /// A full-length level bar at `level` of `max`, `fill` lit. Full at `max`.
    pub fn level(level: u8, max: u8, fill: Fill) -> Self {
        let state = if level >= max { GaugeState::Full } else { GaugeState::Normal };
        GaugeRow::Level { level, fill, length: Fill::FULL, state }
    }

    /// An unnumbered bar with the given fill and width.
    pub fn bar(fill: Fill, length: Fill) -> Self {
        let state = if fill.den != 0 && fill.num >= fill.den { GaugeState::Full } else { GaugeState::Normal };
        GaugeRow::Bar { fill, length, state }
    }

    /// The same continuous row `length` of a full-length bar long.
    pub fn with_length(self, length: Fill) -> Self {
        match self {
            GaugeRow::Level { level, fill, state, .. } => GaugeRow::Level { level, fill, length, state },
            GaugeRow::Bar { fill, state, .. } => GaugeRow::Bar { fill, length, state },
            row => row,
        }
    }

    /// The same row in `state`.
    pub fn with_state(self, state: GaugeState) -> Self {
        match self {
            GaugeRow::Stock { cells, full, part, .. } => GaugeRow::Stock { cells, full, part, state },
            GaugeRow::Level { level, fill, length, .. } => GaugeRow::Level { level, fill, length, state },
            GaugeRow::Bar { fill, length, .. } => GaugeRow::Bar { fill, length, state },
        }
    }

    pub(crate) fn state(self) -> GaugeState {
        match self {
            GaugeRow::Stock { state, .. } | GaugeRow::Level { state, .. } | GaugeRow::Bar { state, .. } => state,
        }
    }
}

/// One side's era gauge: a main row and an optional thinner second row
/// under it. A corner never holds more than these two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EraGauge {
    pub main: GaugeRow,
    pub second: Option<GaugeRow>,
}

impl EraGauge {
    pub fn one(main: GaugeRow) -> Self {
        EraGauge { main, second: None }
    }

    pub fn two(main: GaugeRow, second: GaugeRow) -> Self {
        EraGauge { main, second: Some(second) }
    }
}

/// What the era HUD shows: each side's gauge, or nothing for a side with
/// none (an SF1 or SF1+ fighter, or gauges off).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EraHud {
    pub p1: Option<EraGauge>,
    pub p2: Option<EraGauge>,
}

impl EraHud {
    pub const NONE: EraHud = EraHud { p1: None, p2: None };
}

/// The drawing of the era gauges. Every kind draws the same [`EraHud`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GaugeArt {
    /// After the PLAYER1 health bar: lavender rules, gold fill, dark shadow.
    #[default]
    Ruled,
    /// After the round-win markers: bevelled boxes, lit gold when full.
    Blocks,
    /// The bar in a gold frame with a dark-gold shadow.
    Plate,
}

impl GaugeArt {
    pub const ALL: [GaugeArt; 3] = [GaugeArt::Ruled, GaugeArt::Blocks, GaugeArt::Plate];

    /// The Settings and pause box value, in the ROM font's character set.
    pub const fn label(self) -> &'static str {
        match self {
            GaugeArt::Ruled => "RULED",
            GaugeArt::Blocks => "BLOCKS",
            GaugeArt::Plate => "PLATE",
        }
    }

    /// The `settings.toml` value.
    pub const fn slug(self) -> &'static str {
        match self {
            GaugeArt::Ruled => "ruled",
            GaugeArt::Blocks => "blocks",
            GaugeArt::Plate => "plate",
        }
    }

    pub fn from_slug(s: &str) -> Option<GaugeArt> {
        GaugeArt::ALL.into_iter().find(|a| a.slug() == s)
    }

    /// The art after this one, wrapping.
    pub fn next(self) -> GaugeArt {
        GaugeArt::ALL[(self as usize + 1) % GaugeArt::ALL.len()]
    }

    /// The art before this one, wrapping.
    pub fn prev(self) -> GaugeArt {
        GaugeArt::ALL[(self as usize + GaugeArt::ALL.len() - 1) % GaugeArt::ALL.len()]
    }
}

/// The HUD pens, boot text-palette entries (`TEXT_PALETTE_ROM`) written as
/// text colour / pen. The ROM's own health bar and PLAYER1 label draw with
/// colours 1 and 4; `era_hud_pens_are_the_rom_palette` checks every value
/// against the ROM.
pub mod pens {
    /// Health bar rule (1/2).
    pub const RULE: u32 = 0xff_99_99_ff;
    /// Health bar shadow and empty interior (1/0).
    pub const SHADOW: u32 = 0xff_44_44_44;
    /// PLAYER1 gold body (4/2), highlight (4/0) and shadow (4/1).
    pub const GOLD: u32 = 0xff_cc_99_00;
    pub const GOLD_LIGHT: u32 = 0xff_ff_cc_77;
    pub const GOLD_DARK: u32 = 0xff_88_55_00;
    /// The `COUNTRY SELECT` blue's highlight (5/0) and shadow (5/1).
    pub const LAVENDER_LIGHT: u32 = 0xff_cc_cc_ff;
    pub const LAVENDER_DARK: u32 = 0xff_00_00_99;
    /// The round-win marker's bevel: light, face and dark. They sit in the
    /// ROM's palette table at [`BEVEL_ROM`], not in the boot text palette.
    pub const BEVEL_LIGHT: u32 = 0xff_cc_dd_dd;
    pub const BEVEL: u32 = 0xff_99_aa_aa;
    pub const BEVEL_DARK: u32 = 0xff_88_99_99;
    /// The pale highlight of text colour 13 (13/0), for a full plate's top row.
    pub const CREAM: u32 = 0xff_ff_ff_aa;
    /// Where [`BEVEL_LIGHT`], [`BEVEL`] and [`BEVEL_DARK`] are in the 68000
    /// program region, as palette words (`0x0RGB`).
    pub const BEVEL_ROM: [usize; 3] = [0x1b08c, 0x1b092, 0x1b094];
    /// Grey text: body (10/2) and highlight (10/0).
    pub const GREY: u32 = 0xff_bb_bb_bb;
    pub const GREY_LIGHT: u32 = 0xff_ee_ee_ee;
    /// The level number's pens (highlight, shadow, body): PLAYER1 gold, as
    /// text colour 4 letters it. A greyed row's number is the same glyph in
    /// greys over the rule's shadow, since every grey text colour of the
    /// ROM has a red shadow.
    pub const DIGIT_GOLD: [u32; 3] = [GOLD_LIGHT, GOLD_DARK, GOLD];
    pub const DIGIT_GREY: [u32; 3] = [GREY_LIGHT, SHADOW, GREY];
}

/// Pixels from the screen's side to the corner.
pub const CORNER_EDGE: usize = 8;
/// Rows left clear under the corner.
pub const CORNER_BOTTOM: usize = 8;
/// Interior width of a full-length bar: the health bar's twelve 8-pixel cells.
pub const BAR_WIDTH: usize = 96;
/// The main row: rule, five fill rows, rule, shadow.
pub const MAIN_ROW: usize = 8;
/// The second row: rule, three fill rows, rule, shadow.
pub const SECOND_ROW: usize = 6;
const ROW_GAP: usize = 2;
/// The level number's column: an 8×8 glyph and a 2-pixel gap. Blocks
/// bevels the glyph in a box of the same width.
const DIGIT_COLUMN: usize = 10;
/// Plate's level lettering: `LV` and the number, and a 2-pixel gap.
const PLATE_COLUMN: usize = 26;
/// The level number's box in Blocks: the glyph and a bevel pixel around it.
const LEVEL_BOX: usize = 10;

/// The tallest a corner draws: both rows and the gap between them.
pub const CORNER_HEIGHT: usize = MAIN_ROW + ROW_GAP + SECOND_ROW;
/// The widest a corner draws: the level number's column and a full bar with its caps.
pub const CORNER_WIDTH: usize = DIGIT_COLUMN + BAR_WIDTH + 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    P1,
    P2,
}

/// The `lift` that sets the corners' bottom edge `rows` up from the screen's
/// bottom, so the corners sit directly above something that fills those rows.
pub const fn lift_above_rows(rows: usize) -> usize {
    rows.saturating_sub(CORNER_BOTTOM)
}

/// Draws `hud` into the bottom corners of `argb` in `art`. P1 sits in the
/// bottom-left and P2 mirrored in the bottom-right; fills grow from the
/// screen edge inward. Level numbers need `font` (the ROM's 8×8 text tiles)
/// and are left out without it. A side with no gauge draws nothing.
pub fn draw(argb: &mut [u32], hud: &EraHud, art: GaugeArt, font: Option<&RomFont<'_>>) {
    draw_lifted(argb, hud, art, font, 0);
}

/// [`draw`] with the corners `lift` rows higher: the Training frame meter
/// takes the bottom rows and the corners sit directly above it.
pub fn draw_lifted(argb: &mut [u32], hud: &EraHud, art: GaugeArt, font: Option<&RomFont<'_>>, lift: usize) {
    for (side, gauge) in [(Side::P1, hud.p1), (Side::P2, hud.p2)] {
        if let Some(gauge) = gauge {
            draw_corner(&mut Corner { argb, side }, &gauge, art, font, lift, MAIN_ROW);
        }
    }
}

/// The independent ally's compact gauge, above the primary left corner.
pub(crate) fn draw_ally2(
    argb: &mut [u32],
    gauge: &EraGauge,
    art: GaugeArt,
    font: Option<&RomFont<'_>>,
    lift: usize,
) {
    draw_corner(
        &mut Corner { argb, side: Side::P1 },
        gauge,
        art,
        font,
        lift.saturating_add(CORNER_HEIGHT + ROW_GAP),
        SECOND_ROW,
    );
}

/// One corner, addressed from its screen edge: local `x` 0 is the column
/// nearest the side of the screen, so P2's drawing is P1's mirrored.
struct Corner<'a> {
    argb: &'a mut [u32],
    side: Side,
}

impl Corner<'_> {
    fn screen_x(&self, lx: usize) -> usize {
        match self.side {
            Side::P1 => CORNER_EDGE + lx,
            Side::P2 => WIDTH - 1 - CORNER_EDGE - lx,
        }
    }

    /// The screen column where lettering `width` pixels wide, starting at
    /// local `lx`, begins: lettering reads left to right on both sides.
    fn text_x(&self, lx: usize, width: usize) -> usize {
        match self.side {
            Side::P1 => self.screen_x(lx),
            Side::P2 => self.screen_x(lx + width - 1),
        }
    }

    fn put(&mut self, lx: usize, y: usize, color: u32) {
        let x = self.screen_x(lx);
        if x < WIDTH && y < HEIGHT {
            self.argb[y * WIDTH + x] = color;
        }
    }
}

fn draw_corner(
    corner: &mut Corner<'_>,
    gauge: &EraGauge,
    art: GaugeArt,
    font: Option<&RomFont<'_>>,
    lift: usize,
    main_height: usize,
) {
    let height = main_height + gauge.second.map_or(0, |_| ROW_GAP + SECOND_ROW);
    let bottom = HEIGHT.saturating_sub(CORNER_BOTTOM.saturating_add(lift));
    let Some(top) = bottom.checked_sub(height) else { return };
    let floor = bottom.saturating_sub(CORNER_HEIGHT);
    let levelled = std::iter::once(gauge.main)
        .chain(gauge.second)
        .any(|row| matches!(row, GaugeRow::Level { .. }));
    let lead = match art {
        GaugeArt::Ruled | GaugeArt::Blocks => DIGIT_COLUMN,
        GaugeArt::Plate => PLATE_COLUMN,
    };
    let bar_x = if levelled { lead } else { 0 };
    let main = (top, main_height, gauge.main, Ink::GOLD);
    let second = gauge.second.map(|row| {
        (top + main_height + ROW_GAP, SECOND_ROW, row, Ink::LAVENDER)
    });
    for (i, (y0, height, row, ink)) in std::iter::once(main).chain(second).enumerate() {
        let avail = (CORNER_WIDTH - bar_x).min(BAR_WIDTH + 2);
        let at = Slot { x0: bar_x, avail, floor, y0, height };
        match art {
            GaugeArt::Ruled => draw_row(corner, bar_x, y0, height, row, ink, font),
            GaugeArt::Blocks => draw_blocks_row(corner, at, row, ink, font),
            GaugeArt::Plate => draw_plate_row(corner, at, row, i == 1, font),
        }
    }
}

/// A row's fill pens: the body, its highlight and its shadow.
#[derive(Clone, Copy)]
struct Ink {
    body: u32,
    light: u32,
    dark: u32,
}

impl Ink {
    const GOLD: Ink = Ink { body: pens::GOLD, light: pens::GOLD_LIGHT, dark: pens::GOLD_DARK };
    const LAVENDER: Ink = Ink { body: pens::RULE, light: pens::LAVENDER_LIGHT, dark: pens::LAVENDER_DARK };
}

/// A lit fill row's pen: `fy` counts from the top of the fill.
fn lit_pen(state: GaugeState, ink: Ink, fy: usize) -> u32 {
    match state {
        GaugeState::Normal | GaugeState::Flashing { lit: false } => ink.body,
        GaugeState::Full if fy == 0 => ink.light,
        GaugeState::Full => ink.body,
        GaugeState::Flashing { lit: true } => ink.light,
        GaugeState::Draining => ink.dark,
        GaugeState::Greyed => pens::GREY_LIGHT,
    }
}

fn draw_row(
    corner: &mut Corner<'_>,
    x0: usize,
    y0: usize,
    height: usize,
    row: GaugeRow,
    ink: Ink,
    font: Option<&RomFont<'_>>,
) {
    let state = row.state();
    let rule = if state == GaugeState::Greyed { pens::GREY } else { pens::RULE };
    // Fill rows `y0 + 1 ..= y0 + height - 3`, the top one lit when full.
    let fill_pen = |fy: usize| lit_pen(state, ink, fy);
    let fill_rows = height - 3;
    // Interior columns, each lit or not; dividers and caps are `None`.
    let mut columns: Vec<Option<bool>> = Vec::with_capacity(BAR_WIDTH + 2);
    columns.push(None);
    match row {
        GaugeRow::Stock { cells, full, part, .. } => {
            let cells = cells.max(1) as usize;
            let cell_w = (BAR_WIDTH - (cells - 1)) / cells;
            for cell in 0..cells {
                if cell > 0 {
                    columns.push(None);
                }
                let lit = if cell < full as usize {
                    cell_w
                } else if cell == full as usize {
                    part.px(cell_w)
                } else {
                    0
                };
                columns.extend((0..cell_w).map(|i| Some(i < lit)));
            }
        }
        GaugeRow::Level { fill, length, .. } | GaugeRow::Bar { fill, length, .. } => {
            let width = length.px(BAR_WIDTH).max(1);
            let lit = fill.px(width);
            columns.extend((0..width).map(|i| Some(i < lit)));
            if let (GaugeRow::Level { level, .. }, Some(font)) = (row, font) {
                // The ROM's 8×8 digit (tiles `0x00`–`0x09`), bottom-aligned
                // with the row: a second row's number reaches up into the gap.
                let digit_pens = if state == GaugeState::Greyed { pens::DIGIT_GREY } else { pens::DIGIT_GOLD };
                let x = match corner.side {
                    Side::P1 => corner.screen_x(0),
                    Side::P2 => corner.screen_x(7),
                };
                font.tile_in(corner.argb, x, (y0 + height).saturating_sub(8), u16::from(level.min(9)), digit_pens);
            }
        }
    }
    columns.push(None);
    for (i, column) in columns.iter().enumerate() {
        let lx = x0 + i;
        let edge_cap = i == 0 || i == columns.len() - 1;
        match column {
            None => {
                for dy in 0..height - 1 {
                    corner.put(lx, y0 + dy, rule);
                }
                if !edge_cap {
                    corner.put(lx, y0 + height - 1, pens::SHADOW);
                }
            }
            Some(lit) => {
                corner.put(lx, y0, rule);
                for fy in 0..fill_rows {
                    let pen = if *lit { fill_pen(fy) } else { pens::SHADOW };
                    corner.put(lx, y0 + 1 + fy, pen);
                }
                corner.put(lx, y0 + height - 2, rule);
                corner.put(lx, y0 + height - 1, pens::SHADOW);
            }
        }
    }
}

/// Where a row sits in its corner: the first local column, the columns it
/// may fill, the corner's top row (a taller box than the row stops there)
/// and the row's own top row and height.
#[derive(Clone, Copy)]
struct Slot {
    x0: usize,
    avail: usize,
    floor: usize,
    y0: usize,
    height: usize,
}

/// A bevelled box `w` by `h` from local (`x`, `y`): `light` along the top
/// and left, `dark` inside the bottom and right, a shadow row and column
/// outside them, `inside(column, fill row)` between. The shadow starts one
/// pixel in from the top and left, so the box seems to stand off the screen.
/// The bevel is lit from the screen's top left on both sides, as the 1987
/// round-win markers are; only `inside`'s columns count from the screen edge.
fn bevel_box(
    corner: &mut Corner<'_>,
    (x, y): (usize, usize),
    (w, h): (usize, usize),
    (light, dark): (u32, u32),
    inside: impl Fn(usize, usize) -> u32,
) {
    // A column counted from the box's screen-left edge, as a local column.
    let mirrored = corner.side == Side::P2;
    let col = |sx: usize| if mirrored { w - 1 - sx } else { sx };
    // P2's inside starts past the dark edge and its shadow.
    let inner = if mirrored { 2 } else { 1 };
    for sx in 0..w {
        corner.put(x + col(sx), y, if sx + 1 < w { light } else { pens::SHADOW });
        corner.put(x + col(sx), y + h - 2, if sx + 2 < w { dark } else { pens::SHADOW });
        corner.put(x + col(sx), y + h - 1, pens::SHADOW);
    }
    for dy in 1..h - 2 {
        corner.put(x + col(0), y + dy, light);
        for sx in 1..w - 2 {
            let lx = col(sx);
            corner.put(x + lx, y + dy, inside(lx - inner, dy - 1));
        }
        corner.put(x + col(w - 2), y + dy, dark);
        corner.put(x + col(w - 1), y + dy, pens::SHADOW);
    }
}

/// Blocks: a stock row is a run of bevelled boxes, a full box lit in the
/// row's ink and the next one lit up to its `part`; a level row is a thin
/// ruled line with the level in a bevelled box.
fn draw_blocks_row(corner: &mut Corner<'_>, at: Slot, row: GaugeRow, ink: Ink, font: Option<&RomFont<'_>>) {
    let state = row.state();
    let greyed = state == GaugeState::Greyed;
    let lit_bevel = match state {
        GaugeState::Draining => (ink.body, pens::SHADOW),
        GaugeState::Greyed => (pens::GREY_LIGHT, pens::SHADOW),
        _ => (ink.light, ink.dark),
    };
    let dark_bevel = if greyed { (pens::GREY_LIGHT, pens::GREY) } else { (pens::BEVEL_LIGHT, pens::BEVEL_DARK) };
    let empty = if greyed { pens::SHADOW } else { pens::BEVEL };
    match row {
        GaugeRow::Stock { cells, full, part, .. } => {
            let cells = cells.max(1) as usize;
            // The boxes share the width; the first ones take the odd pixels.
            let (base, odd) = (at.avail / cells, at.avail % cells);
            let mut x = at.x0;
            for cell in 0..cells {
                let cell_w = base + usize::from(cell < odd);
                let size = (cell_w, at.height);
                let origin = x;
                x += cell_w;
                if cell < full as usize {
                    bevel_box(corner, (origin, at.y0), size, lit_bevel, |_, fy| lit_pen(state, ink, fy));
                } else {
                    let lit = if cell == full as usize { part.px(cell_w - 3) } else { 0 };
                    bevel_box(corner, (origin, at.y0), size, dark_bevel, |dx, fy| {
                        if dx < lit {
                            lit_pen(state, ink, fy)
                        } else {
                            empty
                        }
                    });
                }
            }
        }
        GaugeRow::Bar { fill, length, .. } => {
            let width = length.px(at.avail - 3).max(1);
            let lit = fill.px(width);
            let border = if state == GaugeState::Full { lit_bevel } else { dark_bevel };
            bevel_box(corner, (at.x0, at.y0), (width + 3, at.height), border, |dx, fy| {
                if dx < lit { lit_pen(state, ink, fy) } else { empty }
            });
        }
        GaugeRow::Level { level, fill, length, .. } => {
            let bottom = at.y0 + at.height;
            let top = bottom.saturating_sub(LEVEL_BOX).max(at.floor);
            level_box(corner, top, dark_bevel);
            if let Some(font) = font {
                let digit_pens = if greyed { pens::DIGIT_GREY } else { pens::DIGIT_GOLD };
                let x = corner.text_x(1, 8);
                font.tile_in(corner.argb, x, top + 1, u16::from(level.min(9)), digit_pens);
            }
            // The line leaves the box's right side: rule, one fill row,
            // rule and shadow, in the box's middle.
            let rule = if greyed { pens::GREY } else { pens::RULE };
            let width = length.px(at.avail - 1).max(1);
            let lit = fill.px(width);
            let line = top + 3;
            for i in 0..=width {
                let lx = at.x0 + i;
                let cap = i == width;
                corner.put(lx, line, rule);
                let pen = if cap {
                    rule
                } else if i < lit {
                    lit_pen(state, ink, 0)
                } else {
                    pens::SHADOW
                };
                corner.put(lx, line + 1, pen);
                corner.put(lx, line + 2, rule);
                if !cap {
                    corner.put(lx, line + 3, pens::SHADOW);
                }
            }
        }
    }
}

/// The level number's box: a bevel around the glyph, which carries its own
/// shadow. Lit from the screen's top left on both sides, as [`bevel_box`].
fn level_box(corner: &mut Corner<'_>, top: usize, (light, dark): (u32, u32)) {
    let mirrored = corner.side == Side::P2;
    let col = |sx: usize| if mirrored { LEVEL_BOX - 1 - sx } else { sx };
    for d in 0..LEVEL_BOX {
        corner.put(col(d), top, light);
        corner.put(col(0), top + d, light);
        corner.put(col(d), top + LEVEL_BOX - 1, dark);
        corner.put(col(LEVEL_BOX - 1), top + d, dark);
    }
    for dy in 1..LEVEL_BOX - 1 {
        for dx in 1..LEVEL_BOX - 1 {
            corner.put(dx, top + dy, pens::BEVEL);
        }
    }
}

/// Plate: the bar inside a one-pixel gold frame (a light top edge, a
/// dark-gold shadow row), stock cells split by dark gold. A level row has
/// `LV` and its number in PLAYER1 gold to its left. A second row's plate is
/// lavender, as Ruled's second bar.
fn draw_plate_row(corner: &mut Corner<'_>, at: Slot, row: GaugeRow, second: bool, font: Option<&RomFont<'_>>) {
    let state = row.state();
    let greyed = state == GaugeState::Greyed;
    let (light, body, dark) = match (greyed, second) {
        (true, _) => (pens::GREY_LIGHT, pens::GREY, pens::SHADOW),
        (false, false) => (pens::GOLD_LIGHT, pens::GOLD, pens::GOLD_DARK),
        (false, true) => (pens::LAVENDER_LIGHT, pens::RULE, pens::LAVENDER_DARK),
    };
    let ink = if second {
        Ink { body: pens::LAVENDER_LIGHT, light: pens::CREAM, dark: pens::RULE }
    } else {
        Ink { body: pens::GOLD_LIGHT, light: pens::CREAM, dark: pens::GOLD }
    };
    let inner = at.avail - 2;
    let fill_rows = at.height - 3;
    // Interior columns, each lit or not; `None` is a divider.
    let mut columns: Vec<Option<bool>> = Vec::with_capacity(inner);
    match row {
        GaugeRow::Stock { cells, full, part, .. } => {
            let cells = cells.max(1) as usize;
            // The cells share the width between the dividers; the first
            // ones take the odd pixels, so the plate is as wide as a level plate.
            let (base, odd) = ((inner - (cells - 1)) / cells, (inner - (cells - 1)) % cells);
            for cell in 0..cells {
                if cell > 0 {
                    columns.push(None);
                }
                let cell_w = base + usize::from(cell < odd);
                let lit = if cell < full as usize {
                    cell_w
                } else if cell == full as usize {
                    part.px(cell_w)
                } else {
                    0
                };
                columns.extend((0..cell_w).map(|i| Some(i < lit)));
            }
        }
        GaugeRow::Level { fill, length, .. } | GaugeRow::Bar { fill, length, .. } => {
            let width = length.px(inner).max(1);
            let lit = fill.px(width);
            columns.extend((0..width).map(|i| Some(i < lit)));
            if let (GaugeRow::Level { level, .. }, Some(font)) = (row, font) {
                let digit_pens = if greyed { pens::DIGIT_GREY } else { pens::DIGIT_GOLD };
                let text = format!("LV{}", level.min(9));
                let x = corner.text_x(0, 8 * text.len());
                for (i, ch) in text.chars().enumerate() {
                    if let Some(code) = small_glyph(ch) {
                        font.tile_in(corner.argb, x + 8 * i, (at.y0 + at.height).saturating_sub(8), code, digit_pens);
                    }
                }
            }
        }
    }
    let right = columns.len() + 1;
    for dx in 0..=right {
        corner.put(at.x0 + dx, at.y0, light);
        corner.put(at.x0 + dx, at.y0 + at.height - 2, body);
        corner.put(at.x0 + dx, at.y0 + at.height - 1, dark);
    }
    for fy in 0..fill_rows {
        let y = at.y0 + 1 + fy;
        corner.put(at.x0, y, body);
        corner.put(at.x0 + right, y, body);
        for (i, column) in columns.iter().enumerate() {
            let pen = match column {
                None => pens::GOLD_DARK,
                Some(true) => lit_pen(state, ink, fy),
                Some(false) => pens::SHADOW,
            };
            corner.put(at.x0 + 1 + i, y, pen);
        }
    }
}
