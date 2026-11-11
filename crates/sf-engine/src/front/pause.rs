//! The pause box: a small gold frame over the dimmed fight with `RESUME`,
//! `OVERLAYS` (off, hitboxes, frame meter, both), `LOOK` (the player's
//! `CRT LOOK`, saved as a Settings change), `GAUGE ART` (ruled, blocks,
//! plate), `RESTART ROUND` and `QUIT TO MENU`. Training has `RESET
//! POSITIONS` in `RESTART ROUND`'s place (it keeps the fight's present
//! settings, spec #238) and a `TRAINING` row, which opens the Training
//! settings box over the pause box: a `PRESET` row cycling the overhaul
//! presets, `HOLD COMBO`, which keeps the last combo count up, and `FILL
//! METERS`, which keeps every era gauge's resource full (both saved in
//! `settings.toml`). `MOVES` is the last row under every preset: it opens
//! the fighter's move list (spec #226). Local and Training fights pause
//! while the box is up; an online caller keeps stepping and passes its
//! current frame to the same renderer.

use sf_video::compositor::{ink, RomFont};
use sf_video::era_hud::GaugeArt;
use sf_video::render::Frame;

use super::draw::{boxed, dimmed};
use super::menu::MenuItem;
use super::Transition;
use crate::config::RulesetPreset;
use crate::roster::MenuInput;

/// The box's cells.
const BOX_COL: usize = 10;
const BOX_ROW: usize = 6;
const BOX_COLS: usize = 28;
const BOX_ROWS: usize = 17;
/// The screen's text rows.
const SCREEN_ROWS: usize = 28;
/// The notice line (a pad disconnect, say) and the first item's row.
const NOTICE_ROW: usize = 8;
const FIRST_ROW: usize = 10;
/// Where labels start and values end.
const LABEL_COL: usize = 13;
const VALUE_END_COL: usize = 36;

/// The Training settings box's cells: a smaller box inside the pause box's,
/// its title in the 2x2 header font under the top edge.
const SETTINGS_COL: usize = 12;
const SETTINGS_ROW: usize = 10;
const SETTINGS_COLS: usize = 24;
/// The rows the box has room for: `PRESET`, `HOLD COMBO` and `FILL METERS`.
const SETTINGS_SLOTS: usize = 3;
const SETTINGS_TITLE: &str = "TRAINING";
/// Where its labels start (the arrow is two cells before) and its values end.
const SETTINGS_LABEL_COL: usize = SETTINGS_COL + 3;
const SETTINGS_VALUE_END_COL: usize = SETTINGS_COL + SETTINGS_COLS - 2;

/// The overhaul presets the live `PRESET` row cycles: never Faithful 1987,
/// which would unbind a boss on the human side mid-fight.
pub const LIVE_PRESETS: [RulesetPreset; 3] = [
    RulesetPreset::DefinitiveOverhaul,
    RulesetPreset::ClassicCompetitive,
    RulesetPreset::PureFootsies1987Plus,
];

/// The live preset after `preset`.
pub fn next_live_preset(preset: RulesetPreset) -> RulesetPreset {
    let at = LIVE_PRESETS.iter().position(|&p| p == preset);
    LIVE_PRESETS[at.map_or(0, |i| (i + 1) % LIVE_PRESETS.len())]
}

/// The most cells a `LOOK` value takes: the label's four and a gap are
/// left of it in the box's 28.
pub const LOOK_CELLS: usize = VALUE_END_COL - LABEL_COL - 5;

/// A look's name short enough for the box: upper-case, and past
/// [`LOOK_CELLS`] cut with `..`.
pub fn short_look_name(name: &str) -> String {
    super::draw::fit(&name.to_ascii_uppercase(), LOOK_CELLS)
}

/// A preset's name short enough for the box.
pub fn short_preset_name(preset: RulesetPreset) -> &'static str {
    match preset {
        RulesetPreset::DefinitiveOverhaul => "DEFINITIVE",
        RulesetPreset::Faithful1987 => "FAITHFUL",
        RulesetPreset::ClassicCompetitive => "COMPETITIVE",
        RulesetPreset::PureFootsies1987Plus => "FOOTSIES",
        RulesetPreset::Custom => "CUSTOM",
    }
}

/// The Training settings box: the settings only Training has, in a small
/// gold box over the pause box. Up and down move; left, right and confirm
/// choose a row (each cycles its value); back returns to the pause box and
/// Start resumes the fight.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrainingBox {
    cursor: usize,
}

impl TrainingBox {
    /// The rows, top to bottom.
    pub fn items() -> &'static [MenuItem] {
        const ITEMS: &[MenuItem] = &[MenuItem::LivePreset, MenuItem::HoldCombo, MenuItem::FillMeters];
        ITEMS
    }

    /// The text row of item `index`.
    pub fn item_row(index: usize) -> usize {
        SETTINGS_ROW + 4 + 2 * index
    }

    /// The label column.
    pub fn label_col() -> usize {
        SETTINGS_LABEL_COL
    }

    /// The cell after a value's last character.
    pub fn value_end_col() -> usize {
        SETTINGS_VALUE_END_COL
    }

    /// The text row of the box's bottom edge.
    pub fn bottom_row() -> usize {
        Self::item_row(SETTINGS_SLOTS)
    }

    /// The first cell of the title's 2x2 letters.
    pub fn title_cell() -> (usize, usize) {
        (SETTINGS_COL + (SETTINGS_COLS - 2 * SETTINGS_TITLE.len()) / 2, SETTINGS_ROW + 1)
    }

    fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        let items = Self::items();
        match input {
            MenuInput::Up => {
                self.cursor = (self.cursor + items.len() - 1) % items.len();
                Some(Transition::CursorMoved)
            }
            MenuInput::Down => {
                self.cursor = (self.cursor + 1) % items.len();
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm | MenuInput::Left | MenuInput::Right => Some(Transition::Choose(items[self.cursor])),
            MenuInput::Back => Some(Transition::Back),
            MenuInput::Start => Some(Transition::Choose(MenuItem::Resume)),
            _ => None,
        }
    }

    /// The box drawn over `frame`: the title, the rows (the cursor's yellow
    /// with the arrow) and the preset, hold and fill values.
    fn render(&self, font: &RomFont<'_>, frame: &mut Frame, preset: RulesetPreset, hold_combo: bool, fill_meters: bool) {
        let rows = Self::bottom_row() - SETTINGS_ROW + 1;
        boxed(frame, font, SETTINGS_COL, SETTINGS_ROW, SETTINGS_COLS, rows);
        let (col, row) = Self::title_cell();
        font.header(&mut frame.argb, col, row, SETTINGS_TITLE, ink::BLUE);
        for (i, item) in Self::items().iter().enumerate() {
            let row = Self::item_row(i);
            let color = if i == self.cursor {
                font.text(&mut frame.argb, SETTINGS_LABEL_COL - 2, row, ">", ink::YELLOW);
                ink::YELLOW
            } else {
                ink::BLUE
            };
            font.text(&mut frame.argb, SETTINGS_LABEL_COL, row, item.label(), color);
            let value = match item {
                MenuItem::LivePreset => short_preset_name(preset),
                MenuItem::HoldCombo if hold_combo => "ON",
                MenuItem::HoldCombo => "OFF",
                MenuItem::FillMeters if fill_meters => "ON",
                MenuItem::FillMeters => "OFF",
                _ => continue,
            };
            font.text(&mut frame.argb, SETTINGS_VALUE_END_COL - value.len(), row, value, color);
        }
    }
}

/// The pause box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PauseBox {
    training: bool,
    online: bool,
    cursor: usize,
    notice: Option<String>,
    /// The Training settings box over it, while it shows.
    settings: Option<TrainingBox>,
}

impl PauseBox {
    /// The box for a fight; `training` gives it Training's rows.
    pub fn open(training: bool) -> Self {
        PauseBox {
            training,
            online: false,
            cursor: 0,
            notice: None,
            settings: None,
        }
    }

    /// The local overlay for an online fight: no restart or Training actions.
    pub fn open_online() -> Self {
        Self {
            online: true,
            ..Self::open(false)
        }
    }

    /// An online overlay opened for a local notice, such as a lost controller.
    pub fn with_online_notice(notice: impl Into<String>) -> Self {
        Self {
            notice: Some(notice.into()),
            ..Self::open_online()
        }
    }

    /// Put a one-line notice above the rows of a box already up.
    pub fn set_notice(&mut self, notice: String) {
        self.notice = Some(notice);
    }

    /// The box with a one-line notice above its rows.
    pub fn with_notice(training: bool, notice: String) -> Self {
        PauseBox {
            notice: Some(notice),
            ..Self::open(training)
        }
    }

    pub fn name(&self) -> &'static str {
        "pause"
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// The rows, top to bottom.
    pub fn items(&self) -> &'static [MenuItem] {
        const ONLINE: &[MenuItem] = &[
            MenuItem::Resume,
            MenuItem::Overlays,
            MenuItem::Look,
            MenuItem::GaugeArt,
            MenuItem::QuitToMenu,
            MenuItem::Moves,
        ];
        const FIGHT: &[MenuItem] = &[
            MenuItem::Resume,
            MenuItem::Overlays,
            MenuItem::Look,
            MenuItem::GaugeArt,
            MenuItem::RestartRound,
            MenuItem::QuitToMenu,
            MenuItem::Moves,
        ];
        const TRAINING: &[MenuItem] = &[
            MenuItem::Resume,
            MenuItem::Overlays,
            MenuItem::Look,
            MenuItem::GaugeArt,
            MenuItem::ResetPositions,
            MenuItem::QuitToMenu,
            MenuItem::TrainingSettings,
            MenuItem::Moves,
        ];
        if self.online {
            ONLINE
        } else if self.training {
            TRAINING
        } else {
            FIGHT
        }
    }

    /// The text row of item `index`.
    pub fn item_row(index: usize) -> usize {
        FIRST_ROW + 2 * index
    }

    /// The label column.
    pub fn label_col() -> usize {
        LABEL_COL
    }

    /// Up and down move (wrapping); confirm chooses (`TRAINING` opens the
    /// settings box, which then takes the inputs); left and right on
    /// `OVERLAYS`, `LOOK` and `GAUGE ART` choose too (they cycle); back or
    /// Start resumes.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        if let Some(settings) = self.settings.as_mut() {
            let t = settings.apply(input);
            // Back leaves the settings box; the cursor is still on `TRAINING`.
            if t == Some(Transition::Back) {
                self.settings = None;
            }
            return t;
        }
        let items = self.items();
        let item = items[self.cursor];
        match input {
            MenuInput::Up => {
                self.cursor = (self.cursor + items.len() - 1) % items.len();
                Some(Transition::CursorMoved)
            }
            MenuInput::Down => {
                self.cursor = (self.cursor + 1) % items.len();
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm => {
                if item == MenuItem::TrainingSettings {
                    self.settings = Some(TrainingBox::default());
                }
                Some(Transition::Choose(item))
            }
            MenuInput::Left | MenuInput::Right if matches!(item, MenuItem::Overlays | MenuItem::Look | MenuItem::GaugeArt) => {
                Some(Transition::Choose(item))
            }
            MenuInput::Back | MenuInput::Start => Some(Transition::Choose(MenuItem::Resume)),
            _ => None,
        }
    }

    /// `last` dimmed, the box over it: the notice, the rows (the cursor's
    /// yellow with the arrow), and the overlay, look and gauge art values;
    /// then the Training settings box, with the preset, hold and fill values,
    /// when it is open. `look` is already short ([`short_look_name`]).
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        font: &RomFont<'_>,
        last: &Frame,
        overlay: &str,
        look: &str,
        gauge_art: GaugeArt,
        preset: RulesetPreset,
        hold_combo: bool,
        fill_meters: bool,
    ) -> Frame {
        let mut frame = dimmed(last);
        // Training's extra rows take the box deeper, to the screen's last row.
        let rows = BOX_ROWS.max(Self::item_row(self.items().len() - 1) + 2 - BOX_ROW + 1).min(SCREEN_ROWS - BOX_ROW);
        boxed(&mut frame, font, BOX_COL, BOX_ROW, BOX_COLS, rows);
        if let Some(notice) = &self.notice {
            let col = BOX_COL + (BOX_COLS - notice.chars().count().min(BOX_COLS)) / 2;
            font.text(&mut frame.argb, col, NOTICE_ROW, notice, ink::YELLOW);
        }
        for (i, item) in self.items().iter().enumerate() {
            let row = Self::item_row(i);
            let color = if i == self.cursor {
                font.text(&mut frame.argb, LABEL_COL - 2, row, ">", ink::YELLOW);
                ink::YELLOW
            } else {
                ink::BLUE
            };
            font.text(&mut frame.argb, LABEL_COL, row, item.label(), color);
            let value = match item {
                MenuItem::Overlays => overlay,
                MenuItem::Look => look,
                MenuItem::GaugeArt => gauge_art.label(),
                _ => continue,
            };
            font.text(&mut frame.argb, VALUE_END_COL - value.len(), row, value, color);
        }
        if let Some(settings) = &self.settings {
            settings.render(font, &mut frame, preset, hold_combo, fill_meters);
        }
        frame
    }
}
