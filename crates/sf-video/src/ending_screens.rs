//! The ten bosses' ending views (ADR-0016), built from their ROM VS
//! portraits and home stages. The right side is reserved for native text.

use crate::background_bank::{BackgroundBank, Cell, LayerView, MapLayer, MAP_ROWS, PAGE_COLS};
use crate::busts::FIGHTERS;
use crate::render::VISAREA_X;
use crate::rom_stages::rom_stage;
use crate::romset::Assets;
use crate::vs_screens::{left_boss_card, FIRST_COLUMN, SCREEN_COLUMNS};

const TILE: usize = 16;
const TOP_BORDER_ROWS: [usize; 2] = [1, 2];
const FIRST_TEXT_COLUMN: usize = 10;

pub fn ending_key(fighter: &str) -> String {
    format!("ending:{fighter}")
}

struct EndingScreen {
    cells: [[Cell; MAP_ROWS]; SCREEN_COLUMNS],
    palette: u32,
}

fn build(rom: &Assets, opponent: usize) -> Result<EndingScreen, String> {
    let (blank, card) = left_boss_card(rom, opponent)?;
    let stage = rom_stage(&rom.maincpu, opponent)?.bg;
    let stage_x = usize::from(stage.scroll) + VISAREA_X;
    let stage_col = stage_x / TILE;
    if !stage_x.is_multiple_of(TILE) || stage_col + SCREEN_COLUMNS > PAGE_COLS {
        return Err(format!(
            "the ending stage scroll {:#06x} has no complete tile view",
            stage.scroll
        ));
    }
    let mut cells = [[blank; MAP_ROWS]; SCREEN_COLUMNS];
    for (col, column) in cells[..FIRST_TEXT_COLUMN].iter_mut().enumerate() {
        for row in TOP_BORDER_ROWS {
            let cell = Cell::read(
                &rom.tilerom,
                MapLayer::Bg.base(),
                (stage_col + col) * MAP_ROWS + row,
            );
            column[row] = cell;
            // The home stages draw their floors on the foreground layer.
            // Mirror the background strip to frame the portrait at the bottom.
            column[MAP_ROWS - 1 - row] = Cell {
                attr: cell.attr ^ 2,
                ..cell
            };
        }
    }
    for (col, row, cell) in card {
        cells[col][row] = cell;
    }
    Ok(EndingScreen {
        cells,
        palette: stage.palette,
    })
}

impl BackgroundBank {
    /// Build the ten composite views. A ROM or duplicate-key error leaves
    /// the bank as it was.
    pub fn add_ending_screens(&mut self, rom: &Assets) -> Result<usize, String> {
        let screens = FIGHTERS[2..]
            .iter()
            .enumerate()
            .map(|(opponent, fighter)| {
                let key = ending_key(fighter);
                if self.view(&key).is_some() {
                    return Err(format!("the bank already holds {key}"));
                }
                Ok((key, build(rom, opponent)?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let page = self.add_page();
        for (slot, (key, screen)) in screens.iter().enumerate() {
            let first = FIRST_COLUMN + slot * SCREEN_COLUMNS;
            for (col, column) in screen.cells.iter().enumerate() {
                for (row, cell) in column.iter().copied().enumerate() {
                    self.set_cell(page, MapLayer::Bg, first + col, row, cell);
                }
            }
            let scroll = (first * TILE - VISAREA_X) as u16;
            self.set_view(
                key,
                LayerView {
                    page,
                    scroll,
                    palette: screen.palette,
                },
            );
        }
        Ok(screens.len())
    }
}
