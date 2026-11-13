//! The extended background bank (ADR-0009, ticket #198 `ST5`): background
//! and foreground tiles above the ROM's own, maps on pages past the ROM
//! tilemap, and palettes beside the ROM's, which only overhaul fights load.
//! The renderer draws them like the ROM's, and the translated background
//! code scrolls, flips and drifts them.
//!
//! - **Tiles** go after the ROM's in both halves of `gfx1` (bg, `0x1000`
//!   ROM tiles) and `gfx2` (fg, `0x2000`), as the extended sprite bank does
//!   (ADR-0007). The background layers do not swizzle codes, so tile `code`
//!   is slot `code`.
//! - **Pages.** The ROM tilemap fills its 2048 columns
//!   (`docs/reference/background-format.md` section 5), so page p is
//!   another 0x40000-byte block laid out like `tilerom`: bg cells at
//!   `0x00000`, fg cells at `0x20000`, each split into a colour/code-low
//!   half and an attribute/code-high half `0x10000` above it. Page 0 is the
//!   ROM. The renderer reads the page the scene names, and only an overhaul
//!   gate ever names one (ADR-0009, second amendment); the scroll registers
//!   keep their 1987 meaning within it.
//! - **Palettes** are 0x200-byte layer palettes named by a pointer in a
//!   window the 1987 program never requests from ([`PALETTE_WINDOW`]); the
//!   vblank's palette load reads them from the bank.
//!
//! A page's maps are written cell by cell, so a page can hold packed stage
//! art ([`crate::stage`]) or maps built in code when the game loads (the
//! composite VS screens of #215). What a gate shows is a [`LayerView`]: a
//! page, a scroll value and a palette pointer. Stages are named views of
//! both layers ([`StageView`]); other views are kept by key.

use std::collections::{BTreeMap, BTreeSet};

use crate::romset::Assets;
use crate::sprite_bank::encode_sprite_tile;

/// The background (`gfx1`) tiles the ROM holds.
pub const ROM_BG_TILES: usize = 0x1000;
/// The foreground (`gfx2`) tiles the ROM holds.
pub const ROM_FG_TILES: usize = 0x2000;
/// Bytes of one page: both layers' maps, as `tilerom` holds them.
pub const PAGE_BYTES: usize = 0x40000;
/// Columns and rows of one layer's map on a page.
pub const PAGE_COLS: usize = 2048;
pub const MAP_ROWS: usize = 16;
/// The first pointer of the bank's palettes, past everything the main
/// CPU maps (`sfus_map`: the program ROM ends at `0x04ffff`, video RAM
/// starts at `0x800000`). The 1987 game only requests `0x1a268..0x1c068`.
pub const PALETTE_WINDOW: u32 = 0x40_0000;
/// Bytes of one layer palette: 16 colour rows of 16 xRGB_444 words.
pub const PALETTE_BYTES: usize = 0x200;
/// Bytes of one tile in each half of a tile region.
const HALF_TILE: usize = 64;

/// One of the two scroll layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapLayer {
    /// `gfx1`, opaque, drawn first; `tilerom` `0x00000`.
    Bg,
    /// `gfx2`, pen 15 transparent; `tilerom` `0x20000`.
    Fg,
}

impl MapLayer {
    /// The layer's cells within a page.
    pub const fn base(self) -> usize {
        match self {
            MapLayer::Bg => 0,
            MapLayer::Fg => 0x20000,
        }
    }

    /// The tiles the ROM holds for this layer.
    pub const fn rom_tiles(self) -> usize {
        match self {
            MapLayer::Bg => ROM_BG_TILES,
            MapLayer::Fg => ROM_FG_TILES,
        }
    }

    const fn index(self) -> usize {
        match self {
            MapLayer::Bg => 0,
            MapLayer::Fg => 1,
        }
    }
}

/// One map cell in the 1987 format (`get_bg_tile_info` / `get_fg_tile_info`,
/// `sf.cpp:242-258`): a 16-bit tile code, one of the layer's 16 colour rows,
/// and the attribute bits (bit 0 flips the tile in x, bit 1 in y).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cell {
    pub code: u16,
    pub colour: u8,
    pub attr: u8,
}

impl Cell {
    /// The cell at entry `index` (`col * 16 + row`) of the layer at `base`
    /// in a `tilerom`-shaped block.
    pub fn read(block: &[u8], base: usize, index: usize) -> Cell {
        let at = base + 2 * index;
        Cell {
            code: (block[at + 0x10001] as u16) << 8 | block[at + 1] as u16,
            colour: block[at],
            attr: block[at + 0x10000],
        }
    }

    fn write(self, block: &mut [u8], base: usize, index: usize) {
        let at = base + 2 * index;
        block[at] = self.colour;
        block[at + 1] = self.code as u8;
        block[at + 0x10000] = self.attr;
        block[at + 0x10001] = (self.code >> 8) as u8;
    }
}

/// What an overhaul gate shows on one layer: the page the renderer reads,
/// the scroll value the 1987 code writes (its meaning unchanged within the
/// page), and the palette the request loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerView {
    pub page: u8,
    pub scroll: u16,
    pub palette: u32,
}

/// A stage in the bank, as the stage gates read it: the foreground's and
/// the background's views (each scroll is the word the 1987 country tables
/// would give, so `stage_reset` adds the camera's `0xc0` to the
/// foreground's as it does to a table word) and the theme's tune command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageView {
    pub fg: LayerView,
    pub bg: LayerView,
    pub theme: u16,
}

/// The first tune command the extended tune table answers (`sf_audio::tunes`).
/// A theme below it is one of the 1987 tunes and always plays.
pub const EXTENDED_THEME_FIRST: u16 = 0x40;
/// The extended tune command for Ryu's theme (`0x41`), played in Arcade
/// whenever Ryu is the CPU opponent (ticket #267 `SP15`).
pub const RYU_THEME: u8 = 0x41;
/// Evil Ryu has a separate command from the approved Ryu score.
pub const EVIL_RYU_THEME: u8 = 0x51;
/// The 1987 stage theme a stage plays when its own theme is not in the
/// extended tune table (Geki's, a stage tune of priority 8 like the
/// extended themes' default).
pub const FALLBACK_THEME: u16 = 0x29;

/// A stage whose theme the extended tune table does not hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingTheme {
    pub stage: String,
    pub theme: u16,
}

/// A stage the stage select lists (spec #192): who owns it and where it
/// is, for the stage screen. A stage with no listing, like the test stage,
/// is never shown to players.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageListing {
    /// The stage's place in the bank's list (what the roster's pick holds).
    pub index: u8,
    /// The owner's name, as the stage screen spells it.
    pub owner: String,
    /// The country or place the stage is set in.
    pub place: String,
    /// The bonus-roster wave the stage ships with: 0 for the ten 1987
    /// stages, 1 to 3 for a challenger's (spec #192, section 4).
    pub wave: u8,
}

/// How many bonus-roster waves have shipped. None has yet, so no wave's
/// stage is shown; a wave's release raises this with its stages.
pub const RELEASED_WAVES: u8 = 0;

/// The stages the stage select shows from `listed`, in their order: the
/// 1987 stages (wave 0) always, and a stage of wave `w` (1 and up) only
/// with bonus challengers on and `w` among the first `released` waves. An
/// unreleased wave's stage is never shown, with the switch on or off.
pub fn shown_stages(listed: &[StageListing], bonus_challengers: bool, released: u8) -> Vec<StageListing> {
    listed
        .iter()
        .filter(|s| s.wave == 0 || (bonus_challengers && s.wave <= released))
        .cloned()
        .collect()
}

/// Tiles above one layer's ROM tiles, both halves.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Tiles {
    first: Vec<u8>,
    second: Vec<u8>,
}

/// The bank: tiles, pages, palettes and the views gates read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackgroundBank {
    /// Bg then fg.
    tiles: [Tiles; 2],
    /// Pages 1 and up.
    pages: Vec<Vec<u8>>,
    palettes: Vec<Vec<u8>>,
    stages: Vec<(String, StageView)>,
    listed: Vec<StageListing>,
    views: BTreeMap<String, LayerView>,
    /// The extended tune commands the music CPU can play (ticket #199).
    playable: BTreeSet<u8>,
}

impl BackgroundBank {
    /// Stable immutable bank metadata/bytes for netplay compatibility.
    /// Local availability of theme audio is excluded. Current online stages
    /// use native themes; future extended-stage integration must normalize
    /// any gameplay-side sound fallback before enabling it online.
    pub fn compatibility_bytes(&self) -> Vec<u8> {
        fn bytes(out: &mut Vec<u8>, value: &[u8]) {
            out.extend((value.len() as u64).to_le_bytes());
            out.extend_from_slice(value);
        }
        fn count(out: &mut Vec<u8>, value: usize) { out.extend((value as u64).to_le_bytes()); }
        fn layer(out: &mut Vec<u8>, value: &LayerView) {
            let LayerView { page, scroll, palette } = value;
            out.push(*page);
            out.extend(scroll.to_le_bytes());
            out.extend(palette.to_le_bytes());
        }
        let Self { tiles, pages, palettes, stages, listed, views, playable: _ } = self;
        let mut out = b"background-bank/v1".to_vec();
        count(&mut out, tiles.len());
        for Tiles { first, second } in tiles { bytes(&mut out, first); bytes(&mut out, second); }
        count(&mut out, pages.len());
        for page in pages { bytes(&mut out, page); }
        count(&mut out, palettes.len());
        for palette in palettes { bytes(&mut out, palette); }
        count(&mut out, stages.len());
        for (name, StageView { fg, bg, theme }) in stages {
            bytes(&mut out, name.as_bytes());
            layer(&mut out, fg);
            layer(&mut out, bg);
            out.extend(theme.to_le_bytes());
        }
        count(&mut out, listed.len());
        for StageListing { index, owner, place, wave } in listed {
            out.push(*index);
            bytes(&mut out, owner.as_bytes());
            bytes(&mut out, place.as_bytes());
            out.push(*wave);
        }
        count(&mut out, views.len());
        for (name, view) in views { bytes(&mut out, name.as_bytes()); layer(&mut out, view); }
        out
    }

    pub fn new() -> Self {
        BackgroundBank::default()
    }

    /// Store `pens` (0..=15; 15 is transparent on the foreground) as the
    /// layer's next tile, and return its code.
    pub fn add_tile(&mut self, layer: MapLayer, pens: &[[u8; 16]; 16]) -> u16 {
        let code = layer.rom_tiles() + self.tiles(layer);
        assert!(code <= 0xffff, "the {layer:?} tile codes are full");
        let [first, second] = encode_sprite_tile(pens);
        let tiles = &mut self.tiles[layer.index()];
        tiles.first.extend_from_slice(&first);
        tiles.second.extend_from_slice(&second);
        code as u16
    }

    /// The tiles the bank holds above the layer's ROM tiles.
    pub fn tiles(&self, layer: MapLayer) -> usize {
        self.tiles[layer.index()].first.len() / HALF_TILE
    }

    /// Open a new page, every cell unprogrammed (all four bytes `0xff`, as
    /// the ROM map's tail), and return its number.
    pub fn add_page(&mut self) -> u8 {
        self.pages.push(vec![0xff; PAGE_BYTES]);
        u8::try_from(self.pages.len()).expect("at most 255 pages")
    }

    /// The pages the bank holds past the ROM's.
    pub fn pages(&self) -> usize {
        self.pages.len()
    }

    fn page(&self, page: u8) -> &[u8] {
        assert!(page >= 1, "page 0 is the ROM's");
        &self.pages[page as usize - 1]
    }

    /// Write the cell at (`col`, `row`) of the layer's map on `page`.
    /// Panics on page 0, which is the ROM's, or on a page not opened.
    pub fn set_cell(&mut self, page: u8, layer: MapLayer, col: usize, row: usize, cell: Cell) {
        assert!(page >= 1, "page 0 is the ROM's");
        assert!(col < PAGE_COLS && row < MAP_ROWS, "cell ({col}, {row}) is off the map");
        cell.write(&mut self.pages[page as usize - 1], layer.base(), col * MAP_ROWS + row);
    }

    /// The cell at (`col`, `row`) of the layer's map on `page` (1 and up).
    pub fn cell(&self, page: u8, layer: MapLayer, col: usize, row: usize) -> Cell {
        Cell::read(self.page(page), layer.base(), col * MAP_ROWS + row)
    }

    /// Store a layer palette (0x200 bytes, big-endian xRGB_444 words) and
    /// return the pointer a palette request names it by.
    pub fn add_palette(&mut self, bytes: &[u8]) -> u32 {
        assert_eq!(bytes.len(), PALETTE_BYTES, "a layer palette");
        self.palettes.push(bytes.to_vec());
        PALETTE_WINDOW + ((self.palettes.len() - 1) * PALETTE_BYTES) as u32
    }

    /// The palette a request for `pointer` loads, when the bank holds it.
    pub fn palette(&self, pointer: u32) -> Option<&[u8]> {
        let offset = pointer.checked_sub(PALETTE_WINDOW)? as usize;
        if !offset.is_multiple_of(PALETTE_BYTES) {
            return None;
        }
        self.palettes.get(offset / PALETTE_BYTES).map(Vec::as_slice)
    }

    /// Name a stage's views and return its place in the bank's list (what
    /// the roster's stage pick holds).
    pub fn add_stage(&mut self, name: &str, view: StageView) -> u8 {
        assert!(self.stage_index(name).is_none(), "a second stage named {name}");
        self.stages.push((name.to_owned(), view));
        u8::try_from(self.stages.len() - 1).expect("at most 256 stages")
    }

    /// The stage at `index` in the bank's list, its theme as the gates play
    /// it: its own when it is a 1987 tune or one the extended table holds
    /// ([`BackgroundBank::set_playable_themes`]), else [`FALLBACK_THEME`].
    pub fn stage(&self, index: u8) -> Option<StageView> {
        self.stages.get(index as usize).map(|(_, v)| StageView { theme: self.played(v.theme), ..*v })
    }

    /// The authored stage theme, independent of locally available music.
    pub fn stage_record(&self, index: u8) -> Option<StageView> {
        self.stages.get(usize::from(index)).map(|(_, view)| *view)
    }

    fn played(&self, theme: u16) -> u16 {
        let playable = theme < EXTENDED_THEME_FIRST || u8::try_from(theme).is_ok_and(|c| self.playable.contains(&c));
        if playable {
            theme
        } else {
            FALLBACK_THEME
        }
    }

    /// Whether the stage at `index` plays a theme from the extended tune
    /// table (its own is there, not a fallback).
    pub fn plays_extended_theme(&self, index: u8) -> bool {
        self.stage(index).is_some_and(|v| v.theme >= EXTENDED_THEME_FIRST)
    }

    /// Say which extended tune commands the music CPU can play. A bank is
    /// born saying none, so a stage naming an extended theme plays the
    /// fallback until the table's commands are given.
    pub fn set_playable_themes(&mut self, commands: &[u8]) {
        self.playable = commands.iter().copied().collect();
    }

    /// Whether the extended tune table holds `command`
    /// ([`BackgroundBank::set_playable_themes`]).
    pub fn can_play(&self, command: u8) -> bool {
        self.playable.contains(&command)
    }

    /// The extended tune commands the music CPU can play.
    pub fn playable_themes(&self) -> Vec<u8> {
        self.playable.iter().copied().collect()
    }

    /// The stages whose own theme is missing, so they play
    /// [`FALLBACK_THEME`].
    pub fn missing_themes(&self) -> Vec<MissingTheme> {
        self.stages
            .iter()
            .filter(|(_, v)| self.played(v.theme) != v.theme)
            .map(|(name, v)| MissingTheme { stage: name.clone(), theme: v.theme })
            .collect()
    }

    /// Where the stage named `name` is in the bank's list.
    pub fn stage_index(&self, name: &str) -> Option<u8> {
        self.stages.iter().position(|(n, _)| n == name).map(|i| i as u8)
    }

    /// The stages' names, in the bank's order.
    pub fn stage_names(&self) -> impl Iterator<Item = &str> {
        self.stages.iter().map(|(n, _)| n.as_str())
    }

    /// List the stage at `index` on the stage select under its owner's name
    /// and place. Panics when the bank holds no such stage.
    pub fn list_stage(&mut self, index: u8, owner: &str, place: &str) {
        assert!(self.stage(index).is_some(), "no stage at {index} to list");
        self.listed.push(StageListing { index, owner: owner.to_owned(), place: place.to_owned(), wave: 0 });
    }

    /// [`Self::list_stage`] for a challenger's stage, which ships with
    /// bonus-roster `wave` (1 to 3).
    pub fn list_wave_stage(&mut self, index: u8, owner: &str, place: &str, wave: u8) {
        assert!((1..=3).contains(&wave), "bonus waves are 1 to 3");
        self.list_stage(index, owner, place);
        if let Some(listing) = self.listed.last_mut() {
            listing.wave = wave;
        }
    }

    /// The stages the stage select lists, in the order they were listed.
    pub fn listed_stages(&self) -> &[StageListing] {
        &self.listed
    }

    /// Keep a view under `key`, for a gate that is not a stage's (#215's VS
    /// screens, one per pair).
    pub fn set_view(&mut self, key: &str, view: LayerView) {
        self.views.insert(key.to_owned(), view);
    }

    pub fn view(&self, key: &str) -> Option<LayerView> {
        self.views.get(key).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.stages.is_empty() && self.pages.is_empty() && self.palettes.is_empty() && self.tiles(MapLayer::Bg) == 0 && self.tiles(MapLayer::Fg) == 0
    }
}

/// `rom`'s halves with `tiles` after each.
fn extend_region(rom: &[u8], rom_tiles: usize, tiles: &Tiles) -> Vec<u8> {
    let half = rom.len() / 2;
    assert_eq!(half, rom_tiles * HALF_TILE, "the ROM's tile region");
    let mut out = Vec::with_capacity(rom.len() + 2 * tiles.first.len());
    out.extend_from_slice(&rom[..half]);
    out.extend_from_slice(&tiles.first);
    out.extend_from_slice(&rom[half..]);
    out.extend_from_slice(&tiles.second);
    out
}

impl Assets {
    /// These graphics with `bank`'s tiles above the ROM's background and
    /// foreground tiles and its pages after the ROM tilemap, for an
    /// overhaul fight. Every other region is a copy.
    pub fn with_background_bank(&self, bank: &BackgroundBank) -> Assets {
        let mut tilerom = Vec::with_capacity(PAGE_BYTES * (1 + bank.pages.len()));
        tilerom.extend_from_slice(&self.tilerom);
        assert_eq!(tilerom.len(), PAGE_BYTES, "the ROM tilemap");
        for page in &bank.pages {
            tilerom.extend_from_slice(page);
        }
        Assets {
            maincpu: self.maincpu.clone(),
            gfx1: extend_region(&self.gfx1, ROM_BG_TILES, &bank.tiles[0]),
            gfx2: extend_region(&self.gfx2, ROM_FG_TILES, &bank.tiles[1]),
            gfx3: self.gfx3.clone(),
            gfx4: self.gfx4.clone(),
            tilerom,
        }
    }

    /// Map pages these graphics hold: the ROM's, and any bank's.
    pub fn background_pages(&self) -> usize {
        self.tilerom.len() / PAGE_BYTES
    }

    /// The tiles these graphics hold for `layer`, the ROM's and any bank's.
    pub fn layer_tiles(&self, layer: MapLayer) -> usize {
        match layer {
            MapLayer::Bg => self.gfx1.len() / (2 * HALF_TILE),
            MapLayer::Fg => self.gfx2.len() / (2 * HALF_TILE),
        }
    }
}
