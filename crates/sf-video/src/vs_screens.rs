//! The two-player front end's VS screens, built in the extended background
//! bank from the ROM when the game loads (ADR-0009, first amendment;
//! ticket #215).
//!
//! The versus branch of event 4 (`0x3fa00`) always scrolls to the RYU VS
//! KEN screen (bg `0x780`, palette `0x1ae68`), and the ROM tilemap holds no
//! screen for most pairs: only Ryu against each boss (game mode 2,
//! `0x3fa66`), each boss against Ken (mode 4, `0x3fad6`) and Ryu against
//! Ken. Every VS screen has one layout (`crate::busts`): a 6x6 bust block on
//! rows 3..=8 at screen columns 4..=9 (left) and 14..=19 (right), and the
//! nameplate on rows 10..=11: a bracket at column 5, the left name at
//! 6..=9, `VS` at 10..=13, the right name at 14..=17 and a bracket at 18,
//! all in colour row 2. Each fighter's name tiles are the same on either
//! side. The blank cell (code 0, colour 0) fills the rest.
//!
//! A pair's screen is the left fighter's bust and name from a ROM screen
//! where he stands on the left, beside the right fighter's from one where
//! he stands on the right:
//!
//! - a boss's left bust is his mode-4 screen's, his right bust his mode-2
//!   screen's, each in the stage palette that screen loads (`0x43748`);
//! - Ryu stands only on the left (Retsu's mode-2 screen) and Ken only on
//!   the right (Retsu's mode-4 screen). On his other side the bust block is
//!   mirrored with the per-cell x-flip bit, its columns reversed, as the ROM
//!   itself mirrors a boss's bust between his two screens; the name is
//!   placed unflipped.
//!
//! The map starts as the left part's ROM screen, so the brackets, `VS` and
//! blanks are the ROM's. The palette starts as the left part's ROM palette;
//! each colour row the right part draws with is copied from its own
//! palette, and a row the left side already draws with in other colours
//! (two bosses on row 4, say) is moved to the lowest row the screen leaves
//! free by rewriting the colour nibble of the right part's cells. Every
//! cell is a ROM tile in its ROM colours.
//!
//! Ryu against Ken has no built screen: the gate stays closed and the
//! 1987 branch shows the ROM's own. The other 143 pairs sit
//! [`SCREENS_PER_PAGE`] to a page on pages opened after any stage's, all
//! built at load (about 55,000 cell writes, a millisecond), so a fight needs
//! nothing built later and the bank stays immutable once shared.

use crate::background_bank::{BackgroundBank, Cell, LayerView, MapLayer, MAP_ROWS, PAGE_COLS, PALETTE_BYTES};
use crate::busts::{BLOCK_TILES, FIGHTERS};
use crate::romset::Assets;

/// Columns of one VS screen: the 24 a whole-column scroll shows.
pub const SCREEN_COLUMNS: usize = 24;
/// The first page column a built screen starts at. The 85 screens a page
/// holds start at 8, 32, .., 2024, so the last ends at column 2047 and no
/// screen's scroll is 0 (an add to a cleared copy is how the 1987 code
/// leaves a page, `tasks.rs` `add_bg_scroll`).
const FIRST_COLUMN: usize = 8;
/// Built screens on one page.
pub const SCREENS_PER_PAGE: usize = (PAGE_COLS - FIRST_COLUMN) / SCREEN_COLUMNS;
/// The visible area starts 64 pixels into the screen bitmap.
const LEAD_PIXELS: usize = 64;
const TILE: usize = 16;

/// `0x3fa66` / `0x3fad6`: one longword per starting country, each pointing
/// to ten words of bg scroll, one per opponent (modes 2 and 4).
const MODE_2_SCROLLS: usize = 0x3fa66;
const MODE_4_SCROLLS: usize = 0x3fad6;
/// `0x43778`: one longword per country, each pointing to five longwords of
/// stage palettes, one per stage (`0x43748`).
const STAGE_PALETTES: usize = 0x43778;
/// The Japan flag: its tables name the bosses in [`FIGHTERS`] order.
const JAPAN: usize = 0;

/// The bust blocks' top row, and their first screen columns by side.
const BLOCK_ROW: usize = 3;
const BLOCK_COL: [usize; 2] = [4, 14];
/// The nameplate's rows, and each side's four name columns.
const NAME_ROWS: [usize; 2] = [10, 11];
const NAME_COL: [usize; 2] = [6, 14];
const NAME_TILES: usize = 4;

const LEFT: usize = 0;
const RIGHT: usize = 1;

/// The key a gate finds a pair's view by: the left (Player 1) fighter's
/// name, then the right's, as [`FIGHTERS`] spells them.
pub fn vs_key(left: &str, right: &str) -> String {
    format!("vs:{left}:{right}")
}

/// One fighter's half of a VS screen as a ROM screen holds it.
#[derive(Clone, Copy)]
struct Part {
    /// The ROM screen's first column (bg scroll + 64, over 16).
    col: usize,
    /// Its palette's address in `maincpu`.
    palette: usize,
    /// The side he stands on there.
    side: usize,
}

fn word(m: &[u8], addr: usize) -> Result<u16, String> {
    match (m.get(addr), m.get(addr + 1)) {
        (Some(&a), Some(&b)) => Ok(u16::from_be_bytes([a, b])),
        _ => Err(format!("{addr:#x} is past the program ROM")),
    }
}

fn long(m: &[u8], addr: usize) -> Result<usize, String> {
    Ok(((word(m, addr)? as usize) << 16) | word(m, addr + 2)? as usize)
}

/// The ROM screen of `opponent` (0..=9) in game mode 2 or 4 from Japan.
fn rom_screen(m: &[u8], mode_table: usize, opponent: usize, side: usize) -> Result<Part, String> {
    let scroll = word(m, long(m, mode_table + 4 * JAPAN)? + 2 * opponent)? as usize;
    if !(scroll + LEAD_PIXELS).is_multiple_of(TILE) {
        return Err(format!("the VS scroll {scroll:#06x} is not a whole column"));
    }
    let palette = long(m, long(m, STAGE_PALETTES + 4 * JAPAN)? + 4 * (opponent / 2))?;
    Ok(Part { col: (scroll + LEAD_PIXELS) / TILE, palette, side })
}

/// Fighter `i`'s ([`FIGHTERS`] order) part for `side`: the ROM screen it
/// comes from, standing on the side it stands there.
fn part(m: &[u8], i: usize, side: usize) -> Result<Part, String> {
    match i {
        0 => rom_screen(m, MODE_2_SCROLLS, 0, LEFT),
        1 => rom_screen(m, MODE_4_SCROLLS, 0, RIGHT),
        _ if side == LEFT => rom_screen(m, MODE_4_SCROLLS, i - 2, LEFT),
        _ => rom_screen(m, MODE_2_SCROLLS, i - 2, RIGHT),
    }
}

/// The cells one part draws, by (screen column, row) on `side`: its bust
/// block, mirrored when it changes sides, and its name, unflipped.
fn part_cells(tilerom: &[u8], p: Part, side: usize) -> Vec<(usize, usize, Cell)> {
    let rom = |c: usize, r: usize| Cell::read(tilerom, MapLayer::Bg.base(), (p.col + c) * MAP_ROWS + r);
    let mut out = Vec::with_capacity(BLOCK_TILES * BLOCK_TILES + 2 * NAME_TILES);
    for r in BLOCK_ROW..BLOCK_ROW + BLOCK_TILES {
        for k in 0..BLOCK_TILES {
            let cell = if p.side == side {
                rom(BLOCK_COL[p.side] + k, r)
            } else {
                let c = rom(BLOCK_COL[p.side] + BLOCK_TILES - 1 - k, r);
                Cell { attr: c.attr ^ 1, ..c }
            };
            out.push((BLOCK_COL[side] + k, r, cell));
        }
    }
    for r in NAME_ROWS {
        for k in 0..NAME_TILES {
            out.push((NAME_COL[side] + k, r, rom(NAME_COL[p.side] + k, r)));
        }
    }
    out
}

/// One built screen: its 24 columns of cells and its palette.
struct Screen {
    cells: Vec<[Cell; MAP_ROWS]>,
    palette: Vec<u8>,
}

fn row_bytes(palette: &[u8], row: usize) -> &[u8] {
    &palette[row * 32..row * 32 + 32]
}

/// The screen of `left` against `right` ([`FIGHTERS`] indices).
fn build(rom: &Assets, left: usize, right: usize) -> Result<Screen, String> {
    let m = &rom.maincpu;
    let (lp, rp) = (part(m, left, LEFT)?, part(m, right, RIGHT)?);
    let rom_palette = |p: Part| {
        m.get(p.palette..p.palette + PALETTE_BYTES)
            .ok_or_else(|| format!("the palette {:#x} is past the program ROM", p.palette))
    };
    let mut palette = rom_palette(lp)?.to_vec();
    let theirs = rom_palette(rp)?;
    let tilerom = &rom.tilerom;
    if (lp.col.max(rp.col) + SCREEN_COLUMNS) * MAP_ROWS * 2 > 0x10000 {
        return Err("a VS screen is past the ROM tilemap".into());
    }
    let mut cells: Vec<[Cell; MAP_ROWS]> = (0..SCREEN_COLUMNS)
        .map(|c| std::array::from_fn(|r| Cell::read(tilerom, MapLayer::Bg.base(), (lp.col + c) * MAP_ROWS + r)))
        .collect();
    for (c, r, cell) in part_cells(tilerom, lp, LEFT) {
        cells[c][r] = cell;
    }
    let right_cells = part_cells(tilerom, rp, RIGHT);
    // The rows everything but the right part draws with.
    let is_right = |c: usize, r: usize| right_cells.iter().any(|&(rc, rr, _)| (rc, rr) == (c, r));
    let mut used = [false; 16];
    for (c, column) in cells.iter().enumerate() {
        for (r, cell) in column.iter().enumerate() {
            if !is_right(c, r) {
                used[(cell.colour & 0x0f) as usize] = true;
            }
        }
    }
    // Each row the right part draws with: kept when the screen has it in
    // the same colours, copied in when the screen leaves it free, moved to
    // a free row otherwise.
    let mut moved = [None::<u8>; 16];
    for &(_, _, cell) in &right_cells {
        let row = (cell.colour & 0x0f) as usize;
        if moved[row].is_some() {
            continue;
        }
        let to = if !used[row] || row_bytes(&palette, row) == row_bytes(theirs, row) {
            row
        } else {
            (1..16).find(|&r| !used[r]).ok_or("a VS screen with no free colour row")?
        };
        used[to] = true;
        palette[to * 32..to * 32 + 32].copy_from_slice(row_bytes(theirs, row));
        moved[row] = Some(to as u8);
    }
    for (c, r, cell) in right_cells {
        let to = moved[(cell.colour & 0x0f) as usize].unwrap();
        cells[c][r] = Cell { colour: (cell.colour & 0xf0) | to, ..cell };
    }
    Ok(Screen { cells, palette })
}

impl BackgroundBank {
    /// Build every pair's VS screen but Ryu against Ken's from `rom`, on
    /// pages opened after the bank's others, and keep each as the view
    /// [`vs_key`] names. Returns the screens built; on an error the bank
    /// is as it was.
    pub fn add_vs_screens(&mut self, rom: &Assets) -> Result<usize, String> {
        let mut screens = Vec::new();
        for (left, l) in FIGHTERS.iter().enumerate() {
            for (right, r) in FIGHTERS.iter().enumerate() {
                if (*l, *r) != ("Ryu", "Ken") {
                    screens.push((vs_key(l, r), build(rom, left, right)?));
                }
            }
        }
        let mut page = 0;
        for (k, (key, screen)) in screens.iter().enumerate() {
            let slot = k % SCREENS_PER_PAGE;
            if slot == 0 {
                page = self.add_page();
            }
            let first = FIRST_COLUMN + slot * SCREEN_COLUMNS;
            for (c, column) in screen.cells.iter().enumerate() {
                for (r, &cell) in column.iter().enumerate() {
                    self.set_cell(page, MapLayer::Bg, first + c, r, cell);
                }
            }
            let palette = self.add_palette(&screen.palette);
            let scroll = (first * TILE - LEAD_PIXELS) as u16;
            self.set_view(key, LayerView { page, scroll, palette });
        }
        Ok(screens.len())
    }
}
