//! The Jukebox screen (ADR-0009, design section 6a, ticket #202 `ST10`):
//! the screen the Sound tab's `JUKEBOX` row opens. It lists the global
//! track, every stage the stage select shows and every fighter, released
//! challengers included, and sets each to `NATIVE` or to a file of the
//! `jukebox` folder beside the ROM set. There is no file browser and no loop
//! editing: left and right step through the choices, and the files come from
//! the host (the engine never reads a folder), like the look list.
//!
//! - **The choices.** A stage or fighter steps through `NATIVE` and the
//!   files; with a global track set, `GLOBAL` (no entry: it follows the
//!   global track) comes before them. `NATIVE` is an entry of its own, kept
//!   in the settings, so it beats the global track ([`Pick::Native`]).
//!   Without a global track an entry with none shows `NATIVE`, as that is
//!   what plays. The global row steps through `NATIVE` (no track) and the
//!   files.
//! - **Files.** Only names of the formats the host plays
//!   ([`TrackFormats`]: the audio formats, and tracker modules in a build
//!   that has their player), once each, in name order. A file
//!   an entry names that the folder lacks is not lost by looking: it shows
//!   dim, with a notice, and is the last choice on its row until the row is
//!   stepped off it.
//! - **Leaving** hands the tracks back to the Settings screen under it,
//!   which saves them with the other settings.

use sf_video::background_bank::{shown_stages, BackgroundBank};
use sf_video::compositor::{drawable, ink, RomFont, TEXT_COLS};
use sf_video::render::Frame;

use super::draw::{fit, framed_wide};
use super::Transition;
use crate::jukebox::{Jukebox, Pick, TrackFormats, NATIVE, TRACK_EXTENSIONS};
use crate::roster::{MenuInput, TournamentFighter};

/// One stage or fighter on the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// What the Jukebox keys it by: the stage record's name in the
    /// background bank, or the fighter's name.
    pub key: String,
    /// What the screen draws.
    pub label: String,
}

/// The stages the stage select shows (the 1987 ten, and a challenger's with
/// bonus challengers on once its wave has shipped: ST3's `shown_stages`) and
/// the fighters: the twelve of the tournament and, for each stage shown
/// that a challenger owns (wave 1 and up), that challenger. `released` is
/// how many bonus waves have shipped
/// ([`sf_video::background_bank::RELEASED_WAVES`]).
pub fn entries(bank: Option<&BackgroundBank>, bonus_challengers: bool, released: u8) -> (Vec<Entry>, Vec<Entry>) {
    let fighter = |name: &str| Entry { key: name.to_owned(), label: name.to_ascii_uppercase() };
    let mut fighters: Vec<Entry> = TournamentFighter::ALL.iter().map(|f| fighter(f.name())).collect();
    let mut stages = Vec::new();
    if let Some(bank) = bank {
        let names: Vec<&str> = bank.stage_names().collect();
        for listing in shown_stages(bank.listed_stages(), bonus_challengers, released) {
            let Some(key) = names.get(usize::from(listing.index)) else { continue };
            stages.push(Entry { key: (*key).to_owned(), label: listing.owner.to_ascii_uppercase() });
            if listing.wave > 0 && !fighters.iter().any(|f| f.key.eq_ignore_ascii_case(&listing.owner)) {
                fighters.push(fighter(&listing.owner));
            }
        }
    }
    (stages, fighters)
}

/// `files` as the screen lists them: the supported formats only, each name
/// once, in name order (no regard to case).
pub fn listed_tracks(files: impl IntoIterator<Item = String>, formats: &TrackFormats) -> Vec<String> {
    let mut out: Vec<String> = files.into_iter().filter(|f| formats.accepts(f)).collect();
    out.sort_by_cached_key(|f| (f.to_lowercase(), f.clone()));
    out.dedup();
    out
}

/// `file` as drawn: upper case in the ROM font's characters, `_` a space and
/// anything it lacks a `?`.
pub fn shown_name(file: &str) -> String {
    file.chars()
        .flat_map(char::to_uppercase)
        .map(|c| match c {
            '_' => ' ',
            c if drawable(c) => c,
            _ => '?',
        })
        .collect()
}

/// A row of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// The track for every fight with no stage or fighter entry.
    Global,
    Heading(&'static str),
    Stage(usize),
    Fighter(usize),
    /// Every entry back to nothing set.
    ClearAll,
}

impl Row {
    fn live(self) -> bool {
        !matches!(self, Row::Heading(_))
    }
}

/// One choice on a row's ring.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Slot {
    /// No entry: follows the global track.
    Follow,
    Native,
    File(String),
}

/// The rows the screen shows at once (it scrolls with the cursor).
pub const VISIBLE_ROWS: usize = 16;
/// The first row's text row, the label's column, the value's last column:
/// as Settings has them.
pub const FIRST_ROW: usize = 5;
const LABEL_COL: usize = 4;
/// A stage's or fighter's name sits two cells in, under its heading.
const ENTRY_COL: usize = LABEL_COL + 2;
const VALUE_END_COL: usize = 44;
/// The notice lines' first row, under the rows.
pub const NOTICE_ROW: usize = FIRST_ROW + VISIBLE_ROWS + 1;
/// How many choices a tab press jumps.
const JUMP: usize = 10;

/// The Jukebox screen: a copy of the tracks, handed back when it is left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JukeboxScreen {
    jukebox: Jukebox,
    stages: Vec<Entry>,
    fighters: Vec<Entry>,
    /// The files of the `jukebox` folder, as [`listed_tracks`] has them.
    tracks: Vec<String>,
    /// The formats the host plays: what `tracks` is filtered by, and what the
    /// empty-folder notice names.
    formats: TrackFormats,
    /// The cursor's place among [`Self::rows`].
    cursor: usize,
}

impl JukeboxScreen {
    /// The screen with the audio formats every build plays.
    pub fn open(jukebox: Jukebox, stages: Vec<Entry>, fighters: Vec<Entry>, tracks: Vec<String>) -> Self {
        JukeboxScreen::open_with(jukebox, stages, fighters, tracks, TrackFormats::default())
    }

    /// The screen offering the files of `tracks` that are of `formats`.
    pub fn open_with(jukebox: Jukebox, stages: Vec<Entry>, fighters: Vec<Entry>, tracks: Vec<String>, formats: TrackFormats) -> Self {
        JukeboxScreen { jukebox, stages, fighters, tracks: listed_tracks(tracks, &formats), formats, cursor: 0 }
    }

    pub fn name(&self) -> &'static str {
        "jukebox"
    }

    /// The tracks as set so far.
    pub fn jukebox(&self) -> &Jukebox {
        &self.jukebox
    }

    /// The host listed the folder again: these are the files now.
    pub fn set_tracks(&mut self, tracks: Vec<String>) {
        self.tracks = listed_tracks(tracks, &self.formats);
    }

    /// The files listed, in the order the choices step through them.
    pub fn tracks(&self) -> &[String] {
        &self.tracks
    }

    /// The rows, top to bottom: the global track, the stages, the fighters,
    /// and `CLEAR ALL`. A heading is left out with an empty list.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = vec![Row::Global];
        if !self.stages.is_empty() {
            rows.push(Row::Heading("STAGES"));
            rows.extend((0..self.stages.len()).map(Row::Stage));
        }
        if !self.fighters.is_empty() {
            rows.push(Row::Heading("FIGHTERS"));
            rows.extend((0..self.fighters.len()).map(Row::Fighter));
        }
        rows.push(Row::ClearAll);
        rows
    }

    /// The row under the cursor.
    pub fn row(&self) -> Row {
        self.rows()[self.cursor]
    }

    /// The row's name as drawn.
    pub fn label(&self, row: Row) -> String {
        match row {
            Row::Global => "GLOBAL TRACK".to_owned(),
            Row::Heading(name) => name.to_owned(),
            Row::Stage(i) => self.stages[i].label.clone(),
            Row::Fighter(i) => self.fighters[i].label.clone(),
            Row::ClearAll => "CLEAR ALL".to_owned(),
        }
    }

    /// The cell the row's name starts in.
    fn label_col(row: Row) -> usize {
        match row {
            Row::Stage(_) | Row::Fighter(_) => ENTRY_COL,
            _ => LABEL_COL,
        }
    }

    /// The most cells the row's value takes: it keeps a space and a dot of
    /// leader between it and the name.
    fn value_cells(&self, row: Row) -> usize {
        VALUE_END_COL.saturating_sub(Self::label_col(row) + self.label(row).chars().count() + 3)
    }

    /// The row's value as drawn: `NATIVE`, `GLOBAL` or a file's name, cut
    /// with `..` to fit.
    pub fn value(&self, row: Row) -> String {
        let full = match self.slot(row) {
            Some(Slot::Follow) => "GLOBAL".to_owned(),
            Some(Slot::Native) => NATIVE.to_owned(),
            Some(Slot::File(file)) => shown_name(&file),
            None => String::new(),
        };
        fit(&full, self.value_cells(row))
    }

    /// The file a row names that the folder does not hold.
    fn missing(&self, row: Row) -> Option<String> {
        match self.slot(row)? {
            Slot::File(file) if !self.tracks.contains(&file) => Some(file),
            _ => None,
        }
    }

    /// What the row is set to now.
    fn slot(&self, row: Row) -> Option<Slot> {
        let global = self.jukebox.global().is_some();
        let of = |pick: Option<&Pick>| match pick {
            Some(Pick::File(file)) => Slot::File(file.clone()),
            Some(Pick::Native) => Slot::Native,
            None if global => Slot::Follow,
            None => Slot::Native,
        };
        Some(match row {
            Row::Global => self.jukebox.global().map_or(Slot::Native, |file| Slot::File(file.to_owned())),
            Row::Stage(i) => of(self.jukebox.stage_pick(&self.stages[i].key)),
            Row::Fighter(i) => of(self.jukebox.fighter_pick(&self.fighters[i].key)),
            Row::Heading(_) | Row::ClearAll => return None,
        })
    }

    /// The choices a row steps through, in order, with the file it is on
    /// added if the folder does not hold it.
    fn ring(&self, row: Row) -> Vec<Slot> {
        let mut ring = Vec::new();
        if row != Row::Global && self.jukebox.global().is_some() {
            ring.push(Slot::Follow);
        }
        ring.push(Slot::Native);
        ring.extend(self.tracks.iter().cloned().map(Slot::File));
        if let Some(file) = self.missing(row) {
            ring.push(Slot::File(file));
        }
        ring
    }

    /// Move `row` by `delta` choices, wrapping.
    fn step(&mut self, row: Row, delta: isize) {
        let (Some(here), ring) = (self.slot(row), self.ring(row)) else { return };
        let at = ring.iter().position(|s| *s == here).unwrap_or(0) as isize;
        let next = ring[(at + delta).rem_euclid(ring.len() as isize) as usize].clone();
        let j = &mut self.jukebox;
        match (row, next) {
            (Row::Global, Slot::File(file)) => j.set_global(&file),
            (Row::Global, _) => j.clear_global(),
            (Row::Stage(i), slot) => {
                let key = &self.stages[i].key;
                match slot {
                    Slot::Follow => j.clear_stage(key),
                    Slot::Native => j.set_stage_native(key),
                    Slot::File(file) => j.set_stage(key, &file),
                }
            }
            (Row::Fighter(i), slot) => {
                let key = &self.fighters[i].key;
                match slot {
                    Slot::Follow => j.clear_fighter(key),
                    Slot::Native => j.set_fighter_native(key),
                    Slot::File(file) => j.set_fighter(key, &file),
                }
            }
            (Row::Heading(_) | Row::ClearAll, _) => {}
        }
    }

    fn move_cursor(&mut self, delta: isize) -> bool {
        let rows = self.rows();
        let n = rows.len() as isize;
        let mut at = self.cursor as isize;
        for _ in 0..n {
            at = (at + delta).rem_euclid(n);
            if rows[at as usize].live() {
                break;
            }
        }
        let moved = at as usize != self.cursor;
        self.cursor = at as usize;
        moved
    }

    /// Up and down move among the rows; left and right step the row's
    /// choice, and a tab press steps it ten at a time; confirm steps it on
    /// (or clears everything, on `CLEAR ALL`); back leaves.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        let row = self.row();
        let tracks_row = !matches!(row, Row::ClearAll | Row::Heading(_));
        match input {
            MenuInput::Up => self.move_cursor(-1).then_some(Transition::CursorMoved),
            MenuInput::Down => self.move_cursor(1).then_some(Transition::CursorMoved),
            MenuInput::Left | MenuInput::Right | MenuInput::TabLeft | MenuInput::TabRight if tracks_row => {
                let by = match input {
                    MenuInput::Left => -1,
                    MenuInput::Right => 1,
                    MenuInput::TabLeft => -(JUMP as isize),
                    _ => JUMP as isize,
                };
                self.step(row, by);
                Some(Transition::ValueChanged)
            }
            MenuInput::Confirm if tracks_row => {
                self.step(row, 1);
                Some(Transition::ValueChanged)
            }
            MenuInput::Confirm if row == Row::ClearAll => {
                self.jukebox = Jukebox::default();
                Some(Transition::ValueChanged)
            }
            MenuInput::Back => Some(Transition::Back),
            _ => None,
        }
    }

    /// The notice lines under the rows: the folder is empty (and what goes
    /// in it), or the row's file is not in it.
    pub fn notice(&self) -> Vec<String> {
        if self.tracks.is_empty() {
            // The audio formats by name; tracker modules, which are dozens, as one kind.
            let mut formats: Vec<String> = TRACK_EXTENSIONS.iter().map(|e| e.to_ascii_uppercase()).collect();
            if self.formats.has_trackers() {
                formats.push("MODULE".to_owned());
            }
            let (last, rest) = formats.split_last().expect("some format");
            let list = if rest.is_empty() { last.clone() } else { format!("{} OR {last}", rest.join(", ")) };
            return vec![format!("NO TRACKS: PUT {list} FILES"), "IN THE JUKEBOX FOLDER BESIDE THE ROMS".to_owned()];
        }
        if self.missing(self.row()).is_some() {
            return vec!["THIS FILE IS NOT IN THE JUKEBOX FOLDER".to_owned()];
        }
        Vec::new()
    }

    /// The framed screen, in Settings' style: the rows as `NAME .... VALUE`
    /// (scrolling with the cursor), a file the folder lacks dim, and the
    /// notice under them.
    pub fn render(&self, font: &RomFont<'_>) -> Frame {
        let mut frame = framed_wide(font, "JUKEBOX", "");
        let argb = &mut frame.argb;
        let rows = self.rows();
        let top = self.cursor.saturating_sub(VISIBLE_ROWS - 1).min(rows.len().saturating_sub(VISIBLE_ROWS));
        for (line, (i, row)) in rows.iter().enumerate().skip(top).take(VISIBLE_ROWS).enumerate() {
            let r = FIRST_ROW + line;
            let here = i == self.cursor;
            let dim = self.missing(*row).is_some();
            let color = if let Row::Heading(_) = row {
                ink::P2_BLUE
            } else if here {
                font.text(argb, LABEL_COL - 2, r, ">", ink::YELLOW);
                if dim { ink::DIM } else { ink::YELLOW }
            } else if dim {
                ink::DIM
            } else {
                ink::BLUE
            };
            let label = self.label(*row);
            let value = self.value(*row);
            let col = Self::label_col(*row);
            font.text(argb, col, r, &label, color);
            if !value.is_empty() {
                let vcol = VALUE_END_COL.saturating_sub(value.chars().count());
                let dots_from = col + label.chars().count() + 1;
                if vcol > dots_from + 1 {
                    font.text(argb, dots_from, r, &".".repeat(vcol - 1 - dots_from), ink::DIM);
                }
                font.text(argb, vcol, r, &value, color);
            }
        }
        // More rows above or below the window.
        if top > 0 {
            font.text(argb, TEXT_COLS - 3, FIRST_ROW - 1, "\u{2191}", ink::DIM);
        }
        if top + VISIBLE_ROWS < rows.len() {
            font.text(argb, TEXT_COLS - 3, FIRST_ROW + VISIBLE_ROWS, "\u{2193}", ink::DIM);
        }
        for (i, line) in self.notice().iter().enumerate() {
            font.text_centred(argb, NOTICE_ROW + i, line, ink::YELLOW);
        }
        frame
    }
}
