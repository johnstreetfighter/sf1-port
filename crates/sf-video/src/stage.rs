//! Stages in the extended background bank (ADR-0009, ticket #198 `ST5`):
//! a stage's art as the packer takes it, the 1987 background rules the
//! packer checks, and the stage record file that rebuilds a stage's art
//! from the player's ROM.
//!
//! **The 1987 rules** (`docs/reference/background-format.md` sections 1-3):
//! - a fight shows 48 foreground columns (the camera's `0..=0x180` past the
//!   table word) and 56 background columns (the drift's `0..=0x200`), so a
//!   stage has exactly that many, starting 4 columns past its scroll word,
//!   which is a whole column (a multiple of 16);
//! - the drift jumps back `0x200` pixels unseen because a stage's first 24
//!   background columns equal the 24 that start 32 columns on, cell for
//!   cell;
//! - no window crosses its page's pixel `0x8000` (the 1987 game never
//!   wraps the map outside the service scroll test), and a stage never
//!   writes over cells already on its page;
//! - a cell names a tile the layer holds (the ROM's, or the stage's own new
//!   tiles, which go above the ROM's), one of the 16 colour rows, and only
//!   the two flip bits; every colour row a cell uses is one the stage's
//!   palette for that layer fills;
//! - pens are 4 bits and palette words xRGB_444.
//!
//! A rejected stage leaves the bank as it was.
//!
//! **A stage record** (`art/stages/*.json`) is committed; it holds no pixels.
//! It names the stage, its owner and theme, where its maps sit (page and
//! scroll words), and how its art is rebuilt from the ROM: runs of ROM map
//! columns per layer, whether their tiles are copied into the bank, colour
//! rows moved, and each layer's palette as rows of ROM palettes. Real
//! challengers' stage art will come from the art pack; only its records are
//! committed.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::background_bank::{BackgroundBank, Cell, LayerView, MapLayer, StageView, MAP_ROWS, PAGE_COLS, PALETTE_BYTES};
use crate::decode::decode_sprite_tile;
use crate::romset::Assets;

/// Foreground columns a fight shows.
pub const FG_COLUMNS: usize = 48;
/// Background columns a fight shows.
pub const BG_COLUMNS: usize = 56;
/// The drift's jump back, in columns (`0x200` pixels).
pub const DRIFT_JUMP: usize = 32;
/// Columns on screen at a whole-column scroll: the drift repeats them.
pub const SCREEN_COLUMNS: usize = 24;
/// The first map column a fight shows, past the scroll word: the visible
/// area starts 64 pixels into the screen bitmap.
const LEAD_COLUMNS: usize = 4;
/// One page's pixels across.
const PAGE_PIXELS: u32 = (PAGE_COLS * 16) as u32;

/// Where a cell's tile comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileRef {
    /// A ROM tile of the cell's layer.
    Rom(u16),
    /// The stage's own new tile, by its place in [`LayerArt::tiles`].
    New(u16),
}

/// One cell of a stage's map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ArtCell {
    pub tile: TileRef,
    pub colour: u8,
    pub attr: u8,
}

/// One layer of a stage: its columns left to right (rows 0..=15; the
/// screen shows rows 1..=14), the new tiles they draw, and its palette's
/// 16 colour rows (`None` for a row the stage leaves empty).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerArt {
    pub columns: Vec<[ArtCell; MAP_ROWS]>,
    pub tiles: Vec<[[u8; 16]; 16]>,
    pub palette: [Option<[u16; 16]>; 16],
}

/// A stage as the packer takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageArt {
    pub name: String,
    /// The fighter who owns it; `None` only for the test stage.
    pub owner: Option<String>,
    /// The theme's tune command.
    pub theme: u16,
    /// The page its maps sit on (1 and up; page 0 is the ROM's).
    pub page: u8,
    /// The words the 1987 country tables would hold for it: the
    /// foreground's before the camera's `0xc0`, the background's before the
    /// drift.
    pub fg_scroll: u16,
    pub bg_scroll: u16,
    pub fg: LayerArt,
    pub bg: LayerArt,
}

/// Why a stage was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageError {
    /// Page 0 is the ROM tilemap.
    RomPage,
    /// Not the 48 (fg) or 56 (bg) columns a fight shows.
    Width { layer: MapLayer, columns: usize, expected: usize },
    /// Background column `col` differs from column `col + 32`.
    DriftRepeat { col: usize },
    /// The scroll word is not a whole column.
    Unaligned { layer: MapLayer, scroll: u16 },
    /// The fight's window would reach past the page's pixel `0x7fff`.
    PastPage { layer: MapLayer, last_pixel: u32 },
    /// The page already has cells in the stage's columns.
    Overlap { layer: MapLayer, col: usize },
    /// A ROM tile code at or past the layer's ROM tiles.
    RomTile { layer: MapLayer, col: usize, row: usize, code: u16 },
    /// A new-tile index past the stage's new tiles.
    NewTile { layer: MapLayer, col: usize, row: usize, index: u16 },
    /// A colour row above 15, or one the layer's palette leaves empty.
    Colour { layer: MapLayer, col: usize, row: usize, colour: u8 },
    /// Attribute bits other than the two flips.
    Attr { layer: MapLayer, col: usize, row: usize, attr: u8 },
    /// A pen above 15 in a new tile.
    Pen { layer: MapLayer, tile: usize, pen: u8 },
    /// A palette word with bits above xRGB_444's.
    PaletteWord { layer: MapLayer, row: usize, word: u16 },
    /// The bank already holds a stage of this name.
    Name(String),
    /// A record that cannot be rebuilt from the ROM.
    Record(String),
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageError::RomPage => write!(f, "page 0 is the ROM tilemap"),
            StageError::Width { layer, columns, expected } => {
                write!(f, "{layer:?} has {columns} columns, not the {expected} a fight shows")
            }
            StageError::DriftRepeat { col } => write!(
                f,
                "background column {col} differs from column {}: the drift's jump back would show",
                col + DRIFT_JUMP
            ),
            StageError::Unaligned { layer, scroll } => write!(f, "{layer:?} scroll {scroll:#06x} is not a whole column"),
            StageError::PastPage { layer, last_pixel } => {
                write!(f, "{layer:?} window reaches pixel {last_pixel:#x}, past the page")
            }
            StageError::Overlap { layer, col } => write!(f, "{layer:?} column {col} is already on the page"),
            StageError::RomTile { layer, col, row, code } => {
                write!(f, "{layer:?} ({col}, {row}): tile {code:#06x} is past the ROM's")
            }
            StageError::NewTile { layer, col, row, index } => {
                write!(f, "{layer:?} ({col}, {row}): new tile {index} does not exist")
            }
            StageError::Colour { layer, col, row, colour } => {
                write!(f, "{layer:?} ({col}, {row}): colour row {colour} is not in the stage's palette")
            }
            StageError::Attr { layer, col, row, attr } => {
                write!(f, "{layer:?} ({col}, {row}): attribute {attr:#04x} has bits past the flips")
            }
            StageError::Pen { layer, tile, pen } => write!(f, "{layer:?} new tile {tile}: pen {pen}"),
            StageError::PaletteWord { layer, row, word } => {
                write!(f, "{layer:?} palette row {row}: word {word:#06x} is not xRGB_444")
            }
            StageError::Name(name) => write!(f, "the bank already holds a stage named {name}"),
            StageError::Record(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for StageError {}

/// The first map column a fight shows at scroll `word`, and the last pixel
/// its window reaches when the scroll runs `range` past it.
fn footprint(word: u16, range: u16) -> (usize, u32) {
    (word as usize / 16 + LEAD_COLUMNS, word as u32 + range as u32 + 447)
}

/// Check one layer against the rules; returns its first map column.
fn check_layer(
    bank: &BackgroundBank,
    page: u8,
    layer: MapLayer,
    art: &LayerArt,
    scroll: u16,
) -> Result<usize, StageError> {
    let (expected, range) = match layer {
        MapLayer::Fg => (FG_COLUMNS, 0x180),
        MapLayer::Bg => (BG_COLUMNS, 0x200),
    };
    if art.columns.len() != expected {
        return Err(StageError::Width { layer, columns: art.columns.len(), expected });
    }
    if !scroll.is_multiple_of(16) {
        return Err(StageError::Unaligned { layer, scroll });
    }
    let (first, last_pixel) = footprint(scroll, range);
    if last_pixel >= PAGE_PIXELS {
        return Err(StageError::PastPage { layer, last_pixel });
    }
    if (page as usize) <= bank.pages() {
        for col in first..first + expected {
            if (0..MAP_ROWS).any(|row| bank.cell(page, layer, col, row) != UNPROGRAMMED) {
                return Err(StageError::Overlap { layer, col });
            }
        }
    }
    for (row, words) in art.palette.iter().enumerate() {
        if let Some(&word) = words.iter().flatten().find(|&&w| w > 0x0fff) {
            return Err(StageError::PaletteWord { layer, row, word });
        }
    }
    for (tile, pens) in art.tiles.iter().enumerate() {
        if let Some(&pen) = pens.iter().flatten().find(|&&p| p > 15) {
            return Err(StageError::Pen { layer, tile, pen });
        }
    }
    for (col, cells) in art.columns.iter().enumerate() {
        for (row, cell) in cells.iter().enumerate() {
            match cell.tile {
                TileRef::Rom(code) if code as usize >= layer.rom_tiles() => {
                    return Err(StageError::RomTile { layer, col, row, code })
                }
                TileRef::New(index) if index as usize >= art.tiles.len() => {
                    return Err(StageError::NewTile { layer, col, row, index })
                }
                _ => {}
            }
            if cell.colour > 15 || art.palette[cell.colour as usize].is_none() {
                return Err(StageError::Colour { layer, col, row, colour: cell.colour });
            }
            if cell.attr & !3 != 0 {
                return Err(StageError::Attr { layer, col, row, attr: cell.attr });
            }
        }
    }
    Ok(first)
}

/// An unprogrammed cell, all four bytes `0xff`.
const UNPROGRAMMED: Cell = Cell { code: 0xffff, colour: 0xff, attr: 0xff };

fn palette_bytes(rows: &[Option<[u16; 16]>; 16]) -> Vec<u8> {
    let mut bytes = vec![0u8; PALETTE_BYTES];
    for (r, words) in rows.iter().enumerate() {
        for (i, w) in words.iter().flatten().enumerate() {
            bytes[r * 32 + 2 * i..r * 32 + 2 * i + 2].copy_from_slice(&w.to_be_bytes());
        }
    }
    bytes
}

/// Write one checked layer onto the page: its new tiles above the bank's,
/// its cells, and its palette. Returns the palette's pointer.
fn place_layer(bank: &mut BackgroundBank, page: u8, layer: MapLayer, art: &LayerArt, first: usize) -> u32 {
    let codes: Vec<u16> = art.tiles.iter().map(|pens| bank.add_tile(layer, pens)).collect();
    for (c, cells) in art.columns.iter().enumerate() {
        for (row, cell) in cells.iter().enumerate() {
            let code = match cell.tile {
                TileRef::Rom(code) => code,
                TileRef::New(index) => codes[index as usize],
            };
            bank.set_cell(page, layer, first + c, row, Cell { code, colour: cell.colour, attr: cell.attr });
        }
    }
    bank.add_palette(&palette_bytes(&art.palette))
}

impl BackgroundBank {
    /// Check `art` against the 1987 background rules and, if it keeps
    /// them, place it: its pages opened as needed, its tiles, maps and
    /// palettes added, and the stage named. Returns its place in the bank's
    /// stage list. A rejected stage changes nothing.
    pub fn pack_stage(&mut self, art: &StageArt) -> Result<u8, StageError> {
        if art.page == 0 {
            return Err(StageError::RomPage);
        }
        if self.stage_index(&art.name).is_some() {
            return Err(StageError::Name(art.name.clone()));
        }
        let fg_first = check_layer(self, art.page, MapLayer::Fg, &art.fg, art.fg_scroll)?;
        let bg_first = check_layer(self, art.page, MapLayer::Bg, &art.bg, art.bg_scroll)?;
        if let Some(col) = (0..SCREEN_COLUMNS).find(|&c| art.bg.columns[c] != art.bg.columns[c + DRIFT_JUMP]) {
            return Err(StageError::DriftRepeat { col });
        }
        while self.pages() < art.page as usize {
            self.add_page();
        }
        let fg_palette = place_layer(self, art.page, MapLayer::Fg, &art.fg, fg_first);
        let bg_palette = place_layer(self, art.page, MapLayer::Bg, &art.bg, bg_first);
        let view = StageView {
            fg: LayerView { page: art.page, scroll: art.fg_scroll, palette: fg_palette },
            bg: LayerView { page: art.page, scroll: art.bg_scroll, palette: bg_palette },
            theme: art.theme,
        };
        Ok(self.add_stage(&art.name, view))
    }
}

// ---- the record file ---------------------------------------------------

fn hex_u16<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u16, D::Error> {
    let s = String::deserialize(d)?;
    u16::from_str_radix(s.trim_start_matches("0x"), 16).map_err(serde::de::Error::custom)
}

fn hex(s: &str) -> Result<u32, StageError> {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| StageError::Record(format!("{s}: {e}")))
}

/// A stage record, as committed in `art/stages/`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRecord {
    pub name: String,
    /// What the stage is, for the reader.
    #[serde(default)]
    pub about: String,
    /// `null` only for the test stage, which no fighter owns.
    pub owner: Option<String>,
    #[serde(deserialize_with = "hex_u16")]
    pub theme: u16,
    pub page: u8,
    #[serde(deserialize_with = "hex_u16")]
    pub fg_scroll: u16,
    #[serde(deserialize_with = "hex_u16")]
    pub bg_scroll: u16,
    pub fg: LayerRecipe,
    pub bg: LayerRecipe,
}

/// How one layer is rebuilt from the ROM.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerRecipe {
    /// Runs of the ROM map's columns of this layer, left to right.
    pub columns: Vec<ColumnRun>,
    /// Copy each distinct ROM tile the columns draw into the bank, once,
    /// and draw the copies.
    #[serde(default)]
    pub bank_tiles: bool,
    /// Colour rows moved: the ROM cell's row (the key) to the stage's.
    #[serde(default)]
    pub colours: BTreeMap<String, u8>,
    pub palette: PaletteRecipe,
}

/// `count` ROM map columns from column `from`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnRun {
    pub from: usize,
    pub count: usize,
}

/// A layer palette made of ROM palette rows: every row of `base`, if given,
/// then each listed row (the key) taken from row `.1` of the ROM palette
/// at `.0`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteRecipe {
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub rows: BTreeMap<String, (String, u8)>,
}

fn row_key(key: &str) -> Result<usize, StageError> {
    key.parse::<usize>()
        .ok()
        .filter(|&r| r < 16)
        .ok_or_else(|| StageError::Record(format!("colour row {key:?} is not 0..=15")))
}

/// Colour row `row` of the 0x200-byte ROM palette at `pointer`.
fn rom_row(rom: &Assets, pointer: u32, row: usize) -> Result<[u16; 16], StageError> {
    let at = pointer as usize + row * 32;
    let bytes = rom
        .maincpu
        .get(at..at + 32)
        .filter(|_| pointer < 0x50000 && row < 16)
        .ok_or_else(|| StageError::Record(format!("palette {pointer:#x} row {row} is outside the program ROM")))?;
    Ok(std::array::from_fn(|i| u16::from_be_bytes([bytes[2 * i], bytes[2 * i + 1]])))
}

impl PaletteRecipe {
    fn build(&self, rom: &Assets) -> Result<[Option<[u16; 16]>; 16], StageError> {
        let mut rows = [None; 16];
        if let Some(base) = &self.base {
            let pointer = hex(base)?;
            for (r, row) in rows.iter_mut().enumerate() {
                *row = Some(rom_row(rom, pointer, r)?);
            }
        }
        for (key, (pointer, from)) in &self.rows {
            rows[row_key(key)?] = Some(rom_row(rom, hex(pointer)?, *from as usize)?);
        }
        Ok(rows)
    }
}

impl LayerRecipe {
    fn build(&self, rom: &Assets, layer: MapLayer) -> Result<LayerArt, StageError> {
        let mut moved = [None; 16];
        for (key, &to) in &self.colours {
            moved[row_key(key)?] = Some(to);
        }
        let region = match layer {
            MapLayer::Bg => &rom.gfx1,
            MapLayer::Fg => &rom.gfx2,
        };
        let mut new_tiles: BTreeMap<u16, u16> = BTreeMap::new();
        let mut tiles = Vec::new();
        let mut columns = Vec::new();
        for run in &self.columns {
            if run.from + run.count > PAGE_COLS {
                return Err(StageError::Record(format!(
                    "{layer:?} columns {}..{} run past the ROM map",
                    run.from,
                    run.from + run.count
                )));
            }
            for col in run.from..run.from + run.count {
                let column: [ArtCell; MAP_ROWS] = std::array::from_fn(|row| {
                    let cell = Cell::read(&rom.tilerom, layer.base(), col * MAP_ROWS + row);
                    let tile = if self.bank_tiles && (cell.code as usize) < layer.rom_tiles() {
                        let index = *new_tiles.entry(cell.code).or_insert_with(|| {
                            tiles.push(decode_sprite_tile(region, cell.code as usize));
                            (tiles.len() - 1) as u16
                        });
                        TileRef::New(index)
                    } else {
                        TileRef::Rom(cell.code)
                    };
                    let colour = moved.get(cell.colour as usize).copied().flatten().unwrap_or(cell.colour);
                    ArtCell { tile, colour, attr: cell.attr }
                });
                columns.push(column);
            }
        }
        Ok(LayerArt { columns, tiles, palette: self.palette.build(rom)? })
    }
}

impl StageRecord {
    pub fn from_json(text: &str) -> Result<StageRecord, StageError> {
        serde_json::from_str(text).map_err(|e| StageError::Record(format!("bad stage record: {e}")))
    }

    /// The stage's art, rebuilt from the player's ROM.
    pub fn build(&self, rom: &Assets) -> Result<StageArt, StageError> {
        Ok(StageArt {
            name: self.name.clone(),
            owner: self.owner.clone(),
            theme: self.theme,
            page: self.page,
            fg_scroll: self.fg_scroll,
            bg_scroll: self.bg_scroll,
            fg: self.fg.build(rom, MapLayer::Fg)?,
            bg: self.bg.build(rom, MapLayer::Bg)?,
        })
    }
}

/// A bank holding every stage in `records` (JSON texts), each rebuilt from
/// `rom` and packed in order.
pub fn pack_records(rom: &Assets, records: &[&str]) -> Result<BackgroundBank, StageError> {
    let mut bank = BackgroundBank::new();
    for text in records {
        bank.pack_stage(&StageRecord::from_json(text)?.build(rom)?)?;
    }
    Ok(bank)
}
