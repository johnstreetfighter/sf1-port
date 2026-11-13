//! The attract as the front end presents it: the faithful power-on machine,
//! every toggle off and the factory DIP words, untouched, with `FREE PLAY`
//! composited where the cabinet blinks `INSERT COIN`; and the title logo the
//! Main Menu borrows from it.

use sf_video::compositor::{small_glyph, RomFont};
use sf_video::render::{render_frame, Frame};
use sf_video::romset::Assets;

use crate::machine::MachineState;
use crate::run::PowerOnMachine;

/// The attract's completed title logo, from its appearance over the wall
/// through both parts of the following wait before the rankings.
pub(super) fn showing_title(machine: &PowerOnMachine) -> bool {
    matches!(super::cabinet::flow_position(machine), 0x3cda8 | 0x3b588 | 0x3cdb8)
}

/// The ROM's `INSERT COIN` text block, which slot 3 blinks 20 frames on and
/// 20 off in attract mode (`0x3f3a4..0x3f40e`).
pub const INSERT_COIN_BLOCK: usize = 0x1457a;

/// What replaces it, cell for cell (eleven cells, centred).
const FREE_PLAY: &str = " FREE PLAY ";

/// The cell the ROM blanks with (`0x3bd00`, and a text block's fill).
const BLANK_CELL: u16 = 0x103c;

/// The `INSERT COIN` cells as the ROM writes them, and their `FREE PLAY`
/// replacement, both read from the ROM's text block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreePlay {
    /// Video-RAM byte offset of the block's first cell.
    offset: usize,
    insert_coin: Vec<u16>,
    free_play: Vec<u16>,
}

impl FreePlay {
    /// Parse the `INSERT COIN` block (`0x1457a`: a video-RAM offset, one row
    /// of eleven cells, an attribute, then a word per cell or `0xffff` and a
    /// blank count, as the text writer `0x3bba0` reads it). None if the
    /// block is not one row of [`FREE_PLAY`]'s width.
    pub fn from_rom(program_rom: &[u8]) -> Option<Self> {
        let b = INSERT_COIN_BLOCK;
        let rw = |a: usize| program_rom.get(a..a + 2).map(|w| u16::from_be_bytes([w[0], w[1]]));
        let offset = rw(b)? as usize;
        let (rows, cols) = (*program_rom.get(b + 2)?, *program_rom.get(b + 3)? as usize);
        let attr = rw(b + 4)?;
        if rows != 1 || cols != FREE_PLAY.len() {
            return None;
        }
        let mut insert_coin = Vec::with_capacity(cols);
        let mut at = b + 6;
        while insert_coin.len() < cols {
            let w = rw(at)?;
            at += 2;
            if w == 0xffff {
                let count = rw(at)? as usize;
                at += 2;
                insert_coin.extend(std::iter::repeat(BLANK_CELL).take(count));
            } else {
                insert_coin.push(w | attr);
            }
        }
        insert_coin.truncate(cols);
        let free_play = FREE_PLAY
            .chars()
            .map(|ch| match ch {
                ' ' => Some(BLANK_CELL),
                _ => small_glyph(ch).map(|code| code | attr),
            })
            .collect::<Option<Vec<u16>>>()?;
        Some(FreePlay {
            offset,
            insert_coin,
            free_play,
        })
    }

    /// The text cell (column, row) of the block's first cell on screen.
    pub fn screen_cell(&self) -> (usize, usize) {
        let (col, row) = ((self.offset % 0x80) / 2, self.offset / 0x80);
        (col - sf_video::render::VISAREA_X / 8, row - sf_video::render::VISAREA_Y / 8)
    }

    /// Whether `INSERT COIN` is on screen in `videoram` right now.
    pub fn showing(&self, videoram: &[u8]) -> bool {
        self.insert_coin.iter().enumerate().all(|(i, &w)| {
            let a = self.offset + 2 * i;
            videoram.get(a..a + 2) == Some(&w.to_be_bytes()[..])
        })
    }

    /// The machine's frame with `FREE PLAY` in place of `INSERT COIN` while
    /// it shows. The machine's own state is not touched: the cells are
    /// replaced in a copy of the scene before rendering.
    pub fn present(&self, assets: &Assets, state: &MachineState) -> Frame {
        let mut scene = state.scene();
        if self.showing(&scene.videoram) {
            for (i, w) in self.free_play.iter().enumerate() {
                let a = self.offset + 2 * i;
                scene.videoram[a..a + 2].copy_from_slice(&w.to_be_bytes());
            }
        }
        render_frame(assets, &scene)
    }
}

/// The title screen's `STREET FIGHTER` logo: the ROM's text block that the
/// attract draws after the title blocks and holds 60 frames (`0x3cd96`).
pub const TITLE_LOGO_BLOCK: usize = 0x14afc;

/// The title logo's cells as the ROM writes them, for the Main Menu to draw
/// where the title screen does (ADR-0006: ROM art only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleLogo {
    /// The screen cell (column, row) of the block's first cell.
    col: usize,
    row: usize,
    cols: usize,
    /// One video-RAM word per cell, row by row.
    cells: Vec<u16>,
}

impl TitleLogo {
    /// Parse the logo block (`0x14afc`) as the text writer `0x3bba0` reads
    /// it: a video-RAM offset, rows and columns, an attribute, then a word
    /// per cell or `0xffff` and a blank count. None if the block is not on
    /// screen or has a flipped cell (the logo has none, and the front end's
    /// font draws tiles upright).
    pub fn from_rom(program_rom: &[u8]) -> Option<Self> {
        let b = TITLE_LOGO_BLOCK;
        let rw = |a: usize| program_rom.get(a..a + 2).map(|w| u16::from_be_bytes([w[0], w[1]]));
        let offset = rw(b)? as usize;
        let (rows, cols) = (*program_rom.get(b + 2)? as usize, *program_rom.get(b + 3)? as usize);
        let attr = rw(b + 4)?;
        let mut cells = Vec::with_capacity(rows * cols);
        let mut at = b + 6;
        while cells.len() < rows * cols {
            let w = rw(at)?;
            at += 2;
            if w == 0xffff {
                let count = rw(at)? as usize;
                at += 2;
                cells.extend(std::iter::repeat(BLANK_CELL).take(count));
            } else {
                cells.push(w | attr);
            }
        }
        cells.truncate(rows * cols);
        if cells.iter().any(|w| w & 0x0c00 != 0) {
            return None;
        }
        let col = ((offset % 0x80) / 2).checked_sub(sf_video::render::VISAREA_X / 8)?;
        let row = (offset / 0x80).checked_sub(sf_video::render::VISAREA_Y / 8)?;
        Some(TitleLogo { col, row, cols, cells })
    }

    /// The text cell (column, row) of the logo's top-left on screen.
    pub fn screen_cell(&self) -> (usize, usize) {
        (self.col, self.row)
    }

    /// The logo's size in text cells (columns, rows).
    pub fn size(&self) -> (usize, usize) {
        (self.cols, self.cells.len() / self.cols)
    }

    /// Draw the logo where the title screen has it, as the text layer does.
    pub fn draw(&self, font: &RomFont<'_>, argb: &mut [u32]) {
        self.draw_at(font, argb, self.row);
    }

    pub(super) fn draw_at(&self, font: &RomFont<'_>, argb: &mut [u32], row: usize) {
        for (i, &w) in self.cells.iter().enumerate() {
            let (c, r) = (self.col + i % self.cols, row + i / self.cols);
            font.tile(argb, c, r, w & 0x3ff, (w >> 12) as u8);
        }
    }
}
