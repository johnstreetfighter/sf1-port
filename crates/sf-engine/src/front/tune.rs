//! The tuning screen (ticket #212, VO9; spec #203, design section 8): the
//! current look's knobs on a panel at the right, over a looping scene of
//! real gameplay drawn through the look.
//!
//! The panel is the one exception to "one frame per screen" (ADR-0005):
//! the screen's frame is the scene, which the host draws through the look,
//! and [`TuneScreen::panel`] is a separate layer the host composites sharp
//! on top, after the filter chain. The parameters come from the host
//! (librashader reads them from the preset, a drop-in's included) through
//! the session as [`LookParam`]s; what the screen hands back is the values
//! that differ from the look's own ([`TuneScreen::overrides`]), which the
//! host sets on the running chain.
//!
//! The scene behind is the attract's demo fight ([`DemoScene`]): a separate
//! faithful machine from power-on, every toggle off and no input, looped
//! over a stretch with stage pans.

use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::render::WIDTH;

use super::settings::LookName;
use super::Transition;
use crate::roster::MenuInput;
use crate::run::PowerOnMachine;

/// One shader parameter as the preset has it: librashader's metadata, its
/// `initial` value the look's own (the preset's override where it sets
/// one). A parameter whose description is in brackets (`[ BLOOM
/// SETTINGS ]:`) is the shader's section heading, not a knob.
#[derive(Clone, Debug, PartialEq)]
pub struct LookParam {
    pub id: String,
    pub description: String,
    pub initial: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

impl LookParam {
    /// Whether this is a section heading.
    pub fn heading(&self) -> bool {
        self.description.trim_start().starts_with('[')
    }
}

/// A look's tuning for the session: the parameters set away from the
/// look's own values. The host sets them on the look's chain whenever it
/// is the current look.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LookTuning {
    pub look: String,
    pub values: Vec<(String, f32)>,
}

/// One row of the panel's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuneRow {
    /// A section heading: index into the parameters.
    Heading(usize),
    /// A knob: index into the parameters.
    Param(usize),
    /// Saving the tuning as a look of its own (VO10).
    SaveAs,
    /// Every parameter back to the look's own value.
    ResetLook,
}

impl TuneRow {
    fn live(self) -> bool {
        matches!(self, TuneRow::Param(_) | TuneRow::SaveAs | TuneRow::ResetLook)
    }
}

/// A saved look's ids begin with this: the file's path under the drop-in
/// folder (`saved/NAME.slangp`), as every other look's id is.
pub const SAVED_PREFIX: &str = "saved/";

/// Whether the look id `id` is one the player saved.
pub fn is_saved(id: &str) -> bool {
    id.starts_with(SAVED_PREFIX)
}

/// The longest name a saved look can have.
pub const NAME_MAX: usize = 20;
/// The characters name entry steps through: the ROM font's, less those a
/// Windows file name can't hold.
pub const ENTRY_CHARS: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-()'!+,";
/// Names Windows keeps for devices, which no file can have.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3",
    "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Why `name` can't name a saved look, if it can't (it is the file's name
/// too, so the host asks as well).
pub fn name_problem(name: &str) -> Option<&'static str> {
    if name.trim().is_empty() {
        Some("GIVE IT A NAME")
    } else if name.chars().count() > NAME_MAX {
        Some("THAT NAME IS TOO LONG")
    } else if name != tidy(&name.chars().collect::<Vec<_>>()) || !name.chars().all(|c| ENTRY_CHARS.contains(c)) {
        Some("THAT NAME CAN'T BE USED")
    } else if RESERVED.contains(&name) {
        Some("THAT NAME CAN'T BE USED")
    } else {
        None
    }
}

/// `text` with its spaces collapsed and none at either end.
fn tidy(text: &[char]) -> String {
    text.iter().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A request to save the tuned look: the look tuned (its id), the name
/// chosen and the parameters away from the look's own values. The host
/// writes the file ([`TuneScreen::take_save`] through the session, then the
/// session's `look_saved` with the answer).
#[derive(Clone, Debug, PartialEq)]
pub struct SaveLook {
    pub look: String,
    pub name: String,
    pub overrides: Vec<(String, f32)>,
}

/// A key typed on the keyboard while name entry is open: a character, or
/// one of the text-editing keys. The window sends these as typed, key
/// repeats included, and the keyboard's menu keys and fighter keys stand
/// down meanwhile (a pad still steps the characters).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedKey {
    Char(char),
    /// Backspace: the character before the place.
    Erase,
    /// Delete: the character at the place.
    EraseAhead,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    /// Enter saves.
    Enter,
    /// Esc cancels.
    Escape,
}

/// What the screen is doing: showing the list, or one of the questions
/// SAVE AS asks.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    List,
    /// Leaving with changes: save them as a look? `true` is YES.
    Ask(bool),
    /// Name entry: the characters so far and the place being edited (the
    /// end is a blank place to add one).
    Name { text: Vec<char>, at: usize },
    /// The name is a saved look's: replace it? `true` is YES.
    Replace { name: String, yes: bool },
}

/// Which edge of the surface a panel sits at: the picture moves to the
/// other edge while it shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelSide {
    Left,
    #[default]
    Right,
}

/// The panel layer: `width` × `height` opaque ARGB pixels, drawn in the
/// ROM font at 1×. The host scales it by a whole number and composites it
/// at its side of the surface (the tuning screen's is at the right, the
/// move list's at the left), after the look.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel {
    pub side: PanelSide,
    pub width: usize,
    pub height: usize,
    pub argb: Vec<u32>,
}

/// The panel in cells: 32 across (names of up to 22 characters beside a
/// value), as tall as the game frame.
pub const PANEL_COLS: usize = 32;
pub const PANEL_ROWS: usize = 28;
/// The list's first row and how many rows it shows at once.
pub const LIST_ROW: usize = 5;
pub const LIST_ROWS: usize = 20;
const LOOK_ROW: usize = 3;
const HINT_ROW: usize = 26;
const LABEL_COL: usize = 2;
/// One past the value's last column (the frame's right edge is column 31).
const VALUE_END_COL: usize = PANEL_COLS - 1;
/// The longest name drawn: the inner width less a value and the dots.
pub const NAME_CHARS: usize = 22;

/// The tuning screen.
#[derive(Clone, Debug, PartialEq)]
pub struct TuneScreen {
    look: String,
    name: String,
    params: Vec<LookParam>,
    /// The knobs' names as drawn ([short_names]).
    names: Vec<String>,
    values: Vec<f32>,
    rows: Vec<TuneRow>,
    cursor: usize,
    top: usize,
    shown: bool,
    mode: Mode,
    /// A knob changed since the screen opened or the last answer to the
    /// leaving question: leaving asks about saving.
    touched: bool,
    /// Names no saved look can take (`OFF`, the built-in looks, drop-ins),
    /// and the saved looks' (which a save can replace), upper-cased.
    unavailable: Vec<String>,
    saved: Vec<String>,
    /// A line under the list: why a save didn't go, or that it did.
    message: Option<String>,
    save: Option<SaveLook>,
}

impl TuneScreen {
    /// The look `look` (shown as `name`) with its parameters, the session's
    /// earlier tuning of it applied.
    pub fn open(look: &str, name: &str, params: Vec<LookParam>, tuning: &[(String, f32)]) -> Self {
        let mut screen = TuneScreen {
            look: look.to_owned(),
            name: name.to_owned(),
            params: Vec::new(),
            names: Vec::new(),
            values: Vec::new(),
            rows: Vec::new(),
            cursor: 0,
            top: 0,
            shown: true,
            mode: Mode::List,
            touched: false,
            unavailable: Vec::new(),
            saved: Vec::new(),
            message: None,
            save: None,
        };
        screen.set_params(params, tuning);
        screen
    }

    /// Builder: the session's look list, which decides what names `SAVE AS`
    /// takes and what its first suggestion is.
    pub fn with_looks(mut self, looks: &[LookName]) -> Self {
        let names = |saved: bool| looks.iter().filter(|l| is_saved(&l.id) == saved).map(|l| l.name.to_uppercase()).collect();
        self.unavailable = names(false);
        self.saved = names(true);
        self
    }

    /// Builder: the line under the list.
    pub fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_owned());
        self
    }

    /// The save asked for, once.
    pub fn take_save(&mut self) -> Option<SaveLook> {
        self.save.take()
    }

    /// The host couldn't save: back to the list, `message` under it.
    pub fn save_failed(&mut self, message: &str) {
        self.mode = Mode::List;
        self.message = Some(message.to_owned());
    }

    /// The line under the list, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The name `SAVE AS` starts from: the look's own name with the first
    /// number from 2 up that nothing has taken (`ARCADE CABINET 2`).
    pub fn suggested_name(&self) -> String {
        let plain: Vec<char> = self.name.to_uppercase().chars().map(|c| if ENTRY_CHARS.contains(c) { c } else { ' ' }).collect();
        let tidied = tidy(&plain);
        // `ARCADE CABINET 2` suggests 3, not `ARCADE CABINET 2 2`.
        let base = match tidied.rsplit_once(' ') {
            Some((head, tail)) if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) => head,
            _ => &tidied,
        };
        let base = if base.is_empty() { "MY LOOK" } else { base };
        (2..)
            .map(|n| {
                let suffix = format!(" {n}");
                let head: String = base.chars().take(NAME_MAX - suffix.len()).collect();
                format!("{}{suffix}", head.trim_end())
            })
            .find(|c| !self.unavailable.contains(c) && !self.saved.contains(c))
            .expect("a free number")
    }

    /// The name entry's text, when it is open.
    pub fn entry(&self) -> Option<String> {
        match &self.mode {
            Mode::Name { text, .. } => Some(text.iter().collect()),
            _ => None,
        }
    }

    /// Whether the leaving question is showing.
    pub fn asking(&self) -> bool {
        matches!(self.mode, Mode::Ask(_))
    }

    /// Whether the replace question is showing.
    pub fn replacing(&self) -> bool {
        matches!(self.mode, Mode::Replace { .. })
    }

    /// The look's parameters arrived (or changed): the list is rebuilt,
    /// `tuning` applied, the cursor on the first knob.
    pub fn set_params(&mut self, params: Vec<LookParam>, tuning: &[(String, f32)]) {
        self.values = params
            .iter()
            .map(|p| tuning.iter().find(|(id, _)| *id == p.id).map_or(p.initial, |&(_, v)| v))
            .collect();
        self.rows = params
            .iter()
            .enumerate()
            .map(|(i, p)| if p.heading() { TuneRow::Heading(i) } else { TuneRow::Param(i) })
            .chain([TuneRow::SaveAs, TuneRow::ResetLook])
            .collect();
        self.names = short_names(&params, NAME_CHARS);
        self.params = params;
        self.cursor = self.rows.iter().position(|r| r.live()).unwrap_or(0);
        self.top = 0;
        self.scroll();
    }

    pub fn name(&self) -> &'static str {
        "tune"
    }

    /// The look being tuned: its id and name.
    pub fn look(&self) -> (&str, &str) {
        (&self.look, &self.name)
    }

    pub fn params(&self) -> &[LookParam] {
        &self.params
    }

    pub fn rows(&self) -> &[TuneRow] {
        &self.rows
    }

    /// The row under the cursor.
    pub fn row(&self) -> TuneRow {
        self.rows[self.cursor]
    }

    /// The first row the list shows.
    pub fn top(&self) -> usize {
        self.top
    }

    /// Whether the panel is showing (one key hides it).
    pub fn panel_shown(&self) -> bool {
        self.shown
    }

    /// Parameter `id`'s value now.
    pub fn value(&self, id: &str) -> Option<f32> {
        self.params.iter().position(|p| p.id == id).map(|i| self.values[i])
    }

    /// The parameters away from the look's own values, in list order.
    pub fn overrides(&self) -> Vec<(String, f32)> {
        self.params
            .iter()
            .zip(&self.values)
            .filter(|(p, &v)| !p.heading() && v != p.initial)
            .map(|(p, &v)| (p.id.clone(), v))
            .collect()
    }

    /// The tuning as the session keeps it.
    pub fn tuning(&self) -> LookTuning {
        LookTuning {
            look: self.look.clone(),
            values: self.overrides(),
        }
    }

    /// A row's name as drawn: a knob's description upper-cased and
    /// shortened, a heading's in its brackets.
    pub fn label(&self, row: TuneRow) -> String {
        match row {
            TuneRow::Heading(i) => heading_name(&self.params[i].description, PANEL_COLS - 4),
            TuneRow::Param(i) => self.names[i].clone(),
            TuneRow::SaveAs => "SAVE AS >".to_owned(),
            TuneRow::ResetLook => "RESET LOOK".to_owned(),
        }
    }

    /// A knob's value as drawn, to the places its step needs.
    pub fn value_text(&self, row: TuneRow) -> String {
        match row {
            TuneRow::Param(i) => format_value(self.values[i], self.params[i].step),
            _ => String::new(),
        }
    }

    fn move_cursor(&mut self, delta: isize) -> bool {
        let n = self.rows.len() as isize;
        let mut at = self.cursor as isize;
        for _ in 0..n {
            at = (at + delta).rem_euclid(n);
            if self.rows[at as usize].live() {
                break;
            }
        }
        let moved = at as usize != self.cursor;
        self.cursor = at as usize;
        self.scroll();
        moved
    }

    /// The first knob of the next (`delta` 1) or previous (-1) section; the
    /// foot rows count as a section.
    fn jump_section(&mut self, delta: isize) -> bool {
        let starts: Vec<usize> = (0..self.rows.len())
            .filter(|&i| self.rows[i].live() && (i == 0 || !self.rows[i - 1].live() || self.rows[i] == TuneRow::SaveAs))
            .collect();
        let Some(here) = starts.iter().rposition(|&s| s <= self.cursor) else {
            return false;
        };
        let next = (here as isize + delta).rem_euclid(starts.len() as isize) as usize;
        let moved = starts[next] != self.cursor;
        self.cursor = starts[next];
        self.scroll();
        moved
    }

    /// Keep the cursor in view, with the heading above it when it is the
    /// first knob of its section.
    fn scroll(&mut self) {
        let mut want = self.cursor;
        while want > 0 && !self.rows[want - 1].live() && self.cursor - (want - 1) < LIST_ROWS {
            want -= 1;
        }
        if want < self.top {
            self.top = want;
        }
        if self.cursor >= self.top + LIST_ROWS {
            self.top = self.cursor + 1 - LIST_ROWS;
        }
    }

    /// Up and down move among the knobs and `RESET LOOK`, the tab pair
    /// jumps a section, left and right step a knob by its own step within
    /// its own limits, confirm on `RESET LOOK` puts back the look's own
    /// values, Start hides the panel (Start, confirm or back shows it
    /// again), and back leaves.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        match self.mode.clone() {
            Mode::List => {}
            Mode::Ask(yes) => return self.apply_ask(yes, input),
            Mode::Name { .. } => return self.apply_name(input),
            Mode::Replace { name, yes } => return self.apply_replace(&name, yes, input),
        }
        if !self.shown {
            return matches!(input, MenuInput::Start | MenuInput::Confirm | MenuInput::Back).then(|| {
                self.shown = true;
                Transition::CursorMoved
            });
        }
        match input {
            MenuInput::Up => self.move_cursor(-1).then_some(Transition::CursorMoved),
            MenuInput::Down => self.move_cursor(1).then_some(Transition::CursorMoved),
            MenuInput::TabLeft => self.jump_section(-1).then_some(Transition::CursorMoved),
            MenuInput::TabRight => self.jump_section(1).then_some(Transition::CursorMoved),
            MenuInput::Left | MenuInput::Right => {
                let TuneRow::Param(i) = self.row() else { return None };
                let delta = if input == MenuInput::Right { 1.0 } else { -1.0 };
                let next = step_value(&self.params[i], self.values[i], delta);
                (next != self.values[i]).then(|| {
                    self.values[i] = next;
                    self.touched = true;
                    self.message = None;
                    Transition::ValueChanged
                })
            }
            MenuInput::Confirm => match self.row() {
                TuneRow::ResetLook => {
                    self.values = self.params.iter().map(|p| p.initial).collect();
                    self.touched = true;
                    self.message = None;
                    Some(Transition::ValueChanged)
                }
                TuneRow::SaveAs => {
                    self.open_name();
                    Some(Transition::CursorMoved)
                }
                _ => None,
            },
            MenuInput::Start => {
                self.shown = false;
                Some(Transition::CursorMoved)
            }
            MenuInput::Back => {
                if self.touched && !self.overrides().is_empty() {
                    self.mode = Mode::Ask(true);
                    Some(Transition::CursorMoved)
                } else {
                    Some(Transition::Back)
                }
            }
            MenuInput::Filter => None,
        }
    }

    /// Open name entry on the suggested name.
    fn open_name(&mut self) {
        let text: Vec<char> = self.suggested_name().chars().collect();
        self.mode = Mode::Name { at: text.len().min(NAME_MAX - 1), text };
        self.message = None;
    }

    /// Leaving with changes: any direction chooses, confirm answers (YES
    /// goes to the name, NO leaves keeping the changes for the session), and
    /// back is NO.
    fn apply_ask(&mut self, yes: bool, input: MenuInput) -> Option<Transition> {
        match input {
            MenuInput::Left | MenuInput::Right | MenuInput::Up | MenuInput::Down => {
                self.mode = Mode::Ask(!yes);
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm if yes => {
                self.open_name();
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm | MenuInput::Back => {
                self.mode = Mode::List;
                self.touched = false;
                Some(Transition::Back)
            }
            _ => None,
        }
    }

    /// Name entry, as the arcade's initials: up and down step the character
    /// under the cursor (the blank place after the last adds one), left and
    /// right move it, Start deletes, confirm saves and back cancels.
    fn apply_name(&mut self, input: MenuInput) -> Option<Transition> {
        let Mode::Name { text, at } = &mut self.mode else { return None };
        match input {
            MenuInput::Up | MenuInput::Down => {
                let chars: Vec<char> = ENTRY_CHARS.chars().collect();
                if *at == text.len() {
                    if text.len() >= NAME_MAX {
                        return None;
                    }
                    text.push('A');
                } else {
                    let now = chars.iter().position(|&c| c == text[*at]).unwrap_or(0) as isize;
                    let step = if input == MenuInput::Up { 1 } else { -1 };
                    text[*at] = chars[(now + step).rem_euclid(chars.len() as isize) as usize];
                }
                self.message = None;
                Some(Transition::CursorMoved)
            }
            MenuInput::Left => {
                if *at == 0 {
                    return None;
                }
                *at -= 1;
                Some(Transition::CursorMoved)
            }
            MenuInput::Right => {
                if *at >= text.len().min(NAME_MAX - 1) {
                    return None;
                }
                *at += 1;
                Some(Transition::CursorMoved)
            }
            MenuInput::Start => {
                if *at < text.len() {
                    text.remove(*at);
                } else if *at > 0 {
                    *at -= 1;
                    text.remove(*at);
                } else {
                    return None;
                }
                self.message = None;
                Some(Transition::CursorMoved)
            }
            MenuInput::Back => {
                self.mode = Mode::List;
                self.message = None;
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm => {
                let name = tidy(text);
                if let Some(problem) = name_problem(&name) {
                    self.message = Some(problem.to_owned());
                    None
                } else if self.unavailable.contains(&name) {
                    self.message = Some("THAT NAME IS TAKEN".to_owned());
                    None
                } else if self.saved.contains(&name) {
                    self.mode = Mode::Replace { name, yes: false };
                    Some(Transition::CursorMoved)
                } else {
                    Some(self.request_save(name))
                }
            }
            _ => None,
        }
    }

    /// Whether name entry is open, so typed keys reach it.
    pub fn typing(&self) -> bool {
        matches!(self.mode, Mode::Name { .. })
    }

    /// A key typed while name entry is open: a character the entry can
    /// hold goes in at the place (letters upper-cased) and the place moves
    /// past it, Backspace and Delete take out the character before and at
    /// the place, Home and End jump, and the arrows, Enter and Esc work as
    /// the stick, confirm and back do. A full name takes no more.
    pub fn type_key(&mut self, key: TypedKey) -> Option<Transition> {
        let input = match key {
            TypedKey::Left => MenuInput::Left,
            TypedKey::Right => MenuInput::Right,
            TypedKey::Up => MenuInput::Up,
            TypedKey::Down => MenuInput::Down,
            TypedKey::Enter => MenuInput::Confirm,
            TypedKey::Escape => MenuInput::Back,
            TypedKey::Char(_) | TypedKey::Erase | TypedKey::EraseAhead | TypedKey::Home | TypedKey::End => {
                let Mode::Name { text, at } = &mut self.mode else { return None };
                let last = NAME_MAX - 1;
                match key {
                    TypedKey::Char(c) => {
                        let c = c.to_ascii_uppercase();
                        if !ENTRY_CHARS.contains(c) || text.len() >= NAME_MAX {
                            return None;
                        }
                        text.insert(*at, c);
                        *at = (*at + 1).min(last);
                    }
                    // A full name's place stops on its last character,
                    // which then stands for the end.
                    TypedKey::Erase if text.len() == NAME_MAX && *at == last => {
                        text.remove(*at);
                    }
                    TypedKey::Erase if *at > 0 => {
                        *at -= 1;
                        text.remove(*at);
                    }
                    TypedKey::EraseAhead if *at < text.len() => {
                        text.remove(*at);
                    }
                    TypedKey::Home if *at > 0 => *at = 0,
                    TypedKey::End if *at < text.len().min(last) => *at = text.len().min(last),
                    _ => return None,
                }
                self.message = None;
                return Some(Transition::CursorMoved);
            }
        };
        self.typing().then(|| self.apply_name(input)).flatten()
    }

    /// The name is a saved look's: YES replaces it, NO (the default, and
    /// back) goes back to the name.
    fn apply_replace(&mut self, name: &str, yes: bool, input: MenuInput) -> Option<Transition> {
        match input {
            MenuInput::Left | MenuInput::Right | MenuInput::Up | MenuInput::Down => {
                self.mode = Mode::Replace { name: name.to_owned(), yes: !yes };
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm if yes => Some(self.request_save(name.to_owned())),
            MenuInput::Confirm | MenuInput::Back => {
                let text: Vec<char> = name.chars().collect();
                self.mode = Mode::Name { at: text.len().min(NAME_MAX - 1), text };
                Some(Transition::CursorMoved)
            }
            _ => None,
        }
    }

    fn request_save(&mut self, name: String) -> Transition {
        self.save = Some(SaveLook {
            look: self.look.clone(),
            name,
            overrides: self.overrides(),
        });
        self.mode = Mode::List;
        self.touched = false;
        Transition::SaveLook
    }

    /// The panel layer, or `None` while it is hidden: the gold frame with
    /// `TUNE LOOK` in its top edge, the look's name, the list (`NAME .....
    /// VALUE`, headings in their brackets, the cursor row yellow), and the
    /// hint at the foot; or, in place of the list, the question or the name
    /// entry `SAVE AS` is on.
    pub fn panel(&self, font: &RomFont<'_>) -> Option<Panel> {
        if !self.shown {
            return None;
        }
        let (width, height) = (PANEL_COLS * 8, PANEL_ROWS * 8);
        // The font draws into frames a game frame wide: draw there, then
        // take the panel's columns.
        let mut argb = vec![super::draw::BLACK; WIDTH * height];
        let header = "TUNE LOOK";
        let gap = (PANEL_COLS - 2 * header.len()) / 2;
        font.gold_frame(&mut argb, 0, 1, PANEL_COLS, PANEL_ROWS - 1, gap..gap + 2 * header.len());
        font.header(&mut argb, gap, 0, header, ink::BLUE);
        let centred = |argb: &mut [u32], row: usize, text: &str, color: u8| {
            font.text(argb, PANEL_COLS.saturating_sub(text.chars().count()) / 2, row, text, color);
        };
        centred(&mut argb, LOOK_ROW, &sanitise(&self.name, PANEL_COLS - 4), ink::BLUE);
        // YES and NO, the chosen one yellow.
        let choice = |argb: &mut [u32], row: usize, yes: bool| {
            for (col, text, on) in [(8, "YES", yes), (19, "NO", !yes)] {
                if on {
                    font.text(argb, col - 1, row, ">", ink::YELLOW);
                }
                font.text(argb, col, row, text, if on { ink::YELLOW } else { ink::BLUE });
            }
        };
        match &self.mode {
            Mode::List => {
                if self.params.is_empty() {
                    centred(&mut argb, LOOK_ROW + 1, "NO PARAMETERS", ink::DIM);
                }
                for (line, (i, &row)) in self.rows.iter().enumerate().skip(self.top).take(LIST_ROWS).enumerate() {
                    let r = LIST_ROW + line;
                    let label = self.label(row);
                    if let TuneRow::Heading(_) = row {
                        centred(&mut argb, r, &label, ink::BLUE);
                        continue;
                    }
                    let color = if !row.live() {
                        ink::DIM
                    } else if i == self.cursor {
                        font.text(&mut argb, LABEL_COL - 1, r, ">", ink::YELLOW);
                        ink::YELLOW
                    } else {
                        ink::BLUE
                    };
                    font.text(&mut argb, LABEL_COL, r, &label, color);
                    let value = self.value_text(row);
                    if !value.is_empty() {
                        let vcol = VALUE_END_COL - value.len();
                        let dots_from = LABEL_COL + label.len() + 1;
                        if vcol > dots_from + 1 {
                            font.text(&mut argb, dots_from, r, &".".repeat(vcol - 1 - dots_from), ink::DIM);
                        }
                        font.text(&mut argb, vcol, r, &value, color);
                    }
                }
                if let Some(message) = &self.message {
                    centred(&mut argb, HINT_ROW - 1, &sanitise(message, PANEL_COLS - 4), ink::YELLOW);
                }
                centred(&mut argb, HINT_ROW, "START HIDES THE PANEL", ink::DIM);
            }
            Mode::Ask(yes) => {
                centred(&mut argb, LIST_ROW + 3, "SAVE THESE CHANGES", ink::BLUE);
                centred(&mut argb, LIST_ROW + 4, "AS A NEW LOOK?", ink::BLUE);
                choice(&mut argb, LIST_ROW + 7, *yes);
                centred(&mut argb, HINT_ROW, "NO KEEPS THEM THIS SESSION", ink::DIM);
            }
            Mode::Name { text, at } => {
                centred(&mut argb, LIST_ROW + 2, "NAME THIS LOOK", ink::BLUE);
                let col = (PANEL_COLS - NAME_MAX) / 2;
                let shown: String = text.iter().collect();
                font.text(&mut argb, col, LIST_ROW + 5, &shown, ink::BLUE);
                if let Some(&under) = text.get(*at) {
                    font.text(&mut argb, col + *at, LIST_ROW + 5, &under.to_string(), ink::YELLOW);
                }
                font.text(&mut argb, col + *at, LIST_ROW + 6, "*", ink::YELLOW);
                if let Some(message) = &self.message {
                    centred(&mut argb, LIST_ROW + 9, &sanitise(message, PANEL_COLS - 4), ink::YELLOW);
                }
                for (line, hint) in ["TYPE, OR UP DOWN: CHARACTER", "LEFT RIGHT: PLACE", "BACKSPACE OR START: DELETE", "CONFIRM SAVES  BACK CANCELS"].iter().enumerate() {
                    centred(&mut argb, LIST_ROW + 12 + line, hint, ink::DIM);
                }
            }
            Mode::Replace { name, yes } => {
                centred(&mut argb, LIST_ROW + 3, "REPLACE THE SAVED LOOK", ink::BLUE);
                centred(&mut argb, LIST_ROW + 4, &format!("{name}?"), ink::YELLOW);
                choice(&mut argb, LIST_ROW + 7, *yes);
            }
        }
        let argb = argb.chunks_exact(WIDTH).flat_map(|line| line[..width].iter().copied()).collect();
        Some(Panel { side: PanelSide::Right, width, height, argb })
    }
}

/// `value` stepped once by `param`'s own step (`delta` is 1 or -1), within
/// its own limits, rounded to the step's places so steps never drift.
pub fn step_value(param: &LookParam, value: f32, delta: f32) -> f32 {
    if param.step <= 0.0 || param.max <= param.min {
        return value;
    }
    let places = places(param.step) as i32;
    let scale = 10f64.powi(places);
    let next = (f64::from(value) + f64::from(delta) * f64::from(param.step)).clamp(f64::from(param.min), f64::from(param.max));
    ((next * scale).round() / scale) as f32
}

/// The decimal places `step` needs (at most 3 drawn).
fn places(step: f32) -> usize {
    let step = f64::from(step);
    (0..6)
        .find(|&d| {
            let s = step * 10f64.powi(d as i32);
            (s - s.round()).abs() < 1e-4
        })
        .unwrap_or(6)
}

/// `value` to the places `step` needs, at most three.
pub fn format_value(value: f32, step: f32) -> String {
    let places = places(step).min(3);
    let text = format!("{value:.places$}");
    // No "-0" for a value that rounds to zero.
    match text.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_owned(),
        _ => text,
    }
}

/// `text` upper-cased in the characters the ROM font draws (others become
/// spaces), spaces collapsed, cut to `max` characters.
fn sanitise(text: &str, max: usize) -> String {
    let drawn = |c: char| small_glyph(c).is_some() || matches!(c, '+' | '-' | '/');
    let upper: String = text.to_uppercase().chars().map(|c| if drawn(c) { c } else { ' ' }).collect();
    let words: Vec<&str> = upper.split_whitespace().collect();
    let mut out = String::new();
    for word in words {
        let len = out.chars().count();
        let space = usize::from(len > 0);
        if len + space + word.chars().count() > max {
            if len == 0 {
                out = word.chars().take(max).collect();
            }
            break;
        }
        if space == 1 {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// A knob's description as drawable words, uncut, shortened where it is
/// longer than `max`.
fn knob_words(param: &LookParam, max: usize) -> String {
    let text = param.description.trim();
    let text = match text.split_once(':') {
        Some((before, _)) if !before.trim().is_empty() => before,
        _ => text,
    };
    let mut plain = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            c if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    // Too long whole: the long words Guest and Royale use most, shortened.
    let mut full = sanitise(&plain, usize::MAX);
    if full.chars().count() > max {
        full = full.split(' ').map(|w| ABBREVIATIONS.iter().find(|(long, _)| *long == w).map_or(w, |&(_, short)| short)).collect::<Vec<_>>().join(" ");
    }
    if full.is_empty() {
        full = sanitise(&param.id.replace('_', " "), usize::MAX);
    }
    full
}

/// A knob's name for the panel: its description without what follows a
/// colon (Guest lists a knob's choices there) or anything in parentheses,
/// upper-cased in the ROM font's characters, long words shortened if it
/// does not fit, then cut at a whole word to `max`. A knob with no
/// description is named by its id.
pub fn short_name(param: &LookParam, max: usize) -> String {
    trim_joins(sanitise(&knob_words(param, max), max))
}

/// A cut name does not end on a joining mark ("ORDINARY GLOW /").
fn trim_joins(name: String) -> String {
    name.trim_end_matches(|c: char| matches!(c, '/' | '-' | '+' | ',' | ':' | '.' | ' ')).to_owned()
}

/// Every knob's name ([`short_name`]); knobs whose cut names would match
/// keep their first word and as many of their last as fit instead, which
/// is where Guest's similar names differ (`HOR BLOOM/HALO RADIUS`, `..
/// SIGMA`).
pub fn short_names(params: &[LookParam], max: usize) -> Vec<String> {
    let first: Vec<String> = params.iter().map(|p| short_name(p, max)).collect();
    let mut names = first.clone();
    let knob = |i: usize| !params[i].heading();
    for i in 0..params.len() {
        let twins = (0..params.len()).any(|j| j != i && knob(i) && knob(j) && first[j] == first[i]);
        if !twins {
            continue;
        }
        let full = knob_words(&params[i], max);
        let words: Vec<&str> = full.split(' ').collect();
        let mut tail: Vec<&str> = Vec::new();
        let mut len = words[0].chars().count();
        for w in words[1..].iter().rev() {
            if len + 1 + w.chars().count() > max {
                break;
            }
            len += 1 + w.chars().count();
            tail.insert(0, w);
        }
        let mut name = words[0].chars().take(max).collect::<String>();
        for w in tail {
            name.push(' ');
            name.push_str(w);
        }
        names[i] = trim_joins(name);
    }
    names
}

/// Shorter forms for long words in knob names, used only when a name does
/// not fit whole.
const ABBREVIATIONS: &[(&str, &str)] = &[
    ("HORIZONTAL", "HOR"),
    ("PIXELS", "PX"),
    ("HALATION", "HALO"),
    ("BLOOM/HALATION", "BLOOM/HALO"),
    ("DECONVERGENCE", "DECONV"),
    ("VERTICAL", "VERT"),
    ("RESOLUTION", "RES"),
    ("SATURATION", "SAT"),
    ("STRENGTH", "STR"),
    ("BRIGHTNESS", "BRIGHT"),
    ("TEMPERATURE", "TEMP"),
    ("ADJUSTMENT", "ADJ"),
    ("CORRECTION", "CORR"),
    ("THRESHOLD", "THRESH"),
    ("EFFECT", "FX"),
    ("INTERLACING", "INTERLACE"),
    ("PERSISTENCE", "PERSIST"),
    ("DISTRIBUTION", "DISTRIB"),
    ("SHARPNESS", "SHARP"),
    ("SCANLINES", "SCANS"),
    ("SCANLINE", "SCAN"),
    ("CURVATURE", "CURVE"),
    ("SUBSTRACTIVE", "SUBTR"),
    ("SUBTRACTIVE", "SUBTR"),
    ("INTENSITY", "INTENS"),
];

/// A heading's name: what its brackets hold, in brackets.
fn heading_name(description: &str, max: usize) -> String {
    let inner = description.trim().trim_start_matches('[');
    let inner = inner.split(']').next().unwrap_or(inner);
    format!("[ {} ]", sanitise(inner, max.saturating_sub(4)))
}

/// The attract's demo fight from frame [`DEMO_FROM`] to [`DEMO_TO`], looped:
/// Ryu against Retsu on the temple stage, the playfield panning left about
/// a second in (frame 2078) and several times more, right at the end
/// (3097..=3128), just before the demo cuts to its next scene at 3129.
#[derive(Clone)]
pub struct DemoScene {
    start: PowerOnMachine,
    now: PowerOnMachine,
}

/// The loop's first and last frames, in the attract's power-on frames.
pub const DEMO_FROM: u32 = 2020;
pub const DEMO_TO: u32 = 3128;

impl DemoScene {
    /// A faithful machine powered on and run to the loop's start: a few
    /// milliseconds, the machine stepping with no input as the attract does.
    pub fn new(program_rom: &[u8]) -> Self {
        let mut machine = PowerOnMachine::new(program_rom.to_vec());
        while machine.state.frame < DEMO_FROM {
            machine.step();
        }
        DemoScene {
            start: machine.clone(),
            now: machine,
        }
    }

    /// One frame on, back to the loop's start after its last.
    pub fn step(&mut self) {
        if self.now.state.frame >= DEMO_TO {
            self.now = self.start.clone();
        } else {
            self.now.step();
        }
    }

    pub fn machine(&self) -> &PowerOnMachine {
        &self.now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(id: &str, description: &str, initial: f32, min: f32, max: f32, step: f32) -> LookParam {
        LookParam {
            id: id.into(),
            description: description.into(),
            initial,
            min,
            max,
            step,
        }
    }

    #[test]
    fn steps_stay_on_the_steps_grid_and_inside_the_limits() {
        let p = param("PR", "Persistence Red", 0.32, 0.0, 0.5, 0.01);
        let mut v = p.initial;
        for _ in 0..30 {
            v = step_value(&p, v, 1.0);
        }
        assert_eq!(v, 0.5, "held at the maximum");
        assert_eq!(step_value(&p, 0.32, -1.0), 0.31);
        assert_eq!(step_value(&p, 0.0, -1.0), 0.0, "held at the minimum");
        let flat = param("x", "x", 1.0, 1.0, 1.0, 0.1);
        assert_eq!(step_value(&flat, 1.0, 1.0), 1.0);
    }

    #[test]
    fn values_show_the_places_their_step_needs() {
        assert_eq!(format_value(4.0, 1.0), "4");
        assert_eq!(format_value(0.32, 0.01), "0.32");
        assert_eq!(format_value(0.75, 0.025), "0.750");
        assert_eq!(format_value(-0.25, 0.05), "-0.25");
        assert_eq!(format_value(-0.0001, 0.01), "0.00");
    }

    #[test]
    fn names_are_upper_cased_shortened_and_drawable() {
        let p = param("esrc", "          Afterglow Effect Source: OriginalHistory | Source", 1.0, 1.0, 2.0, 1.0);
        assert_eq!(short_name(&p, NAME_CHARS), "AFTERGLOW FX SOURCE", "long words shortened to fit");
        let p = param("glow", "          (Magic) Glow Strength", 0.08, -2.0, 2.0, 0.01);
        assert_eq!(short_name(&p, NAME_CHARS), "GLOW STRENGTH");
        let p = param("BLOOM", "          Raster bloom %", 0.0, 0.0, 20.0, 1.0);
        assert_eq!(short_name(&p, NAME_CHARS), "RASTER BLOOM %");
        let p = param("odd_id", "  ", 0.0, 0.0, 1.0, 1.0);
        assert_eq!(short_name(&p, NAME_CHARS), "ODD ID");
        assert_eq!(heading_name("[ AFTERGLOW SETTINGS ]:", 28), "[ AFTERGLOW SETTINGS ]");
        assert!(param("bogus", "[ X ]:", 0.0, 0.0, 1.0, 1.0).heading());
        // Names that would cut the same keep their last words instead.
        let twins = [
            param("deconrr", "          Horizontal Deconvergence Red Range", 0.0, -15.0, 15.0, 0.25),
            param("SIZEHB", "          Horizontal Bloom/Halation Radius", 3.0, 1.0, 50.0, 1.0),
            param("SIGMA_HB", "          Horizontal Bloom/Halation Sigma", 0.75, 0.25, 15.0, 0.025),
        ];
        assert_eq!(
            short_names(&twins, 16),
            ["HOR DECONV RED", "HOR RADIUS", "HOR SIGMA"],
            "only the twins change"
        );
        assert_eq!(short_names(&twins, NAME_CHARS)[1..], ["HOR BLOOM/HALO RADIUS", "HOR BLOOM/HALO SIGMA"]);
    }
}
