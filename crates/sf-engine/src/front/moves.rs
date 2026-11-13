//! The move list panel (ticket #227, ML1; spec #226): the pause box's
//! `MOVES` row opens the player's fighter's moves on a gold-framed panel at
//! the left of the screen, with the fight beside it.
//!
//! It is drawn like the tuning panel ([`super::tune`]): a [`Panel`] layer
//! the host composites sharp after the look, on the left edge, with the
//! picture moved to the right. The fight's own frame is untouched. The
//! header is the fighter's name; below it come the headed sections
//! SPECIALS, SUPER, SYSTEM and THROWS (empty ones left out) with each move's
//! name on the left and its input on the right, then the normals as one
//! compact line. The cursor row is yellow, the list scrolls when it is
//! longer than the panel, and back returns to the pause box. The moves are
//! the registry ([`crate::moves`]) filtered by the fight; the inputs are
//! written in the chosen notation (ML2, the Settings row `MOVE INPUTS`):
//! arrows drawn for the fighter's facing when the list opened (the fight
//! is frozen behind it), numpad numbers facing right, or motion words.
//!
//! Whose list opens is the session's (ML3): this panel takes the fighter it is given.
//! In Training ([`MovesPanel::with_show`]) confirm on a move's row offers
//! `SHOW` and `TRY` in the foot of the panel: left and right pick, confirm
//! chooses, back drops the choice. Show (ML4) closes the panel and the
//! session has the fighter perform the move ([`MovesPanel::take_show`]);
//! Try (ML5) closes it and the coach watches the player attempt the move
//! ([`MovesPanel::take_try`]). Outside Training confirm on a row does
//! nothing.

use sf_video::compositor::{ink, RomFont};
use sf_video::render::WIDTH;

use super::draw::{fit, BLACK};
use super::tune::{Panel, PanelSide};
use super::menu::MenuItem;
use super::Transition;
use crate::config::{EvilRyuPlaytest, RulesetPreset};
use crate::moves::{input_cells, input_text, move_list, ButtonClass, Dir, InputNotation, InputPiece, MoveEntry, MoveInput, MoveKind, Recognizer, NORMALS_LINE};
use crate::roster::{MenuInput, TournamentFighter};

/// The panel in cells: 40 across (the normals line is 37), as tall as the
/// game frame.
pub const MOVES_COLS: usize = 40;
pub const MOVES_ROWS: usize = 28;
/// The list's first row and how many rows it shows at once.
pub const LIST_ROW: usize = 3;
pub const LIST_ROWS: usize = 21;
const HINT_ROW: usize = 26;
const LABEL_COL: usize = 2;
/// One past the input's last column (the frame's right edge is the last).
const VALUE_END_COL: usize = MOVES_COLS - 1;

/// The foot's choice on a move (Training).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Show,
    Try,
}

impl Choice {
    pub const ALL: [Choice; 2] = [Choice::Show, Choice::Try];

    pub const fn label(self) -> &'static str {
        match self {
            Choice::Show => "SHOW",
            Choice::Try => "TRY",
        }
    }

    /// The other choice.
    pub const fn other(self) -> Choice {
        match self {
            Choice::Show => Choice::Try,
            Choice::Try => Choice::Show,
        }
    }
}

/// The choice's row, above the hint.
const CHOICE_ROW: usize = HINT_ROW - 2;

/// One line of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    /// A section's heading.
    Heading(&'static str),
    /// A move: index into the entries.
    Move(usize),
    /// A command documented in the private Arcade playtest.
    Command(&'static str, MoveInput, Option<&'static str>),
    /// The private super uses a sequence of individual button presses.
    RagingDemon(&'static str),
    CloseThrow(&'static str, &'static str),
    /// The normals line.
    Normals,
}

/// The move list panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovesPanel {
    fighter: TournamentFighter,
    display_name: &'static str,
    playtest: Option<EvilRyuPlaytest>,
    entries: Vec<MoveEntry>,
    lines: Vec<Line>,
    /// The cursor's line, when there is a move to stand on.
    cursor: Option<usize>,
    /// The first line shown.
    top: usize,
    /// Training offers Show and Try on a move's row.
    training: bool,
    /// The choice up on the cursor's move, with its cursor.
    choosing: Option<Choice>,
    /// The move Show was chosen for, until the session takes it.
    shown: Option<MoveEntry>,
    /// The move Try was chosen for, until the session takes it.
    tried: Option<MoveEntry>,
    /// How the inputs are written, and the way the fighter faces for the
    /// arrows.
    notation: InputNotation,
    facing_right: bool,
}

impl MovesPanel {
    /// The list for `fighter` in a fight under `preset`.
    pub fn open(fighter: TournamentFighter, preset: RulesetPreset) -> Self {
        Self::from_entries(fighter, move_list(fighter, preset).into_iter().copied().collect())
    }

    /// The list for `fighter` in a real fight, which has his 1987 player
    /// table from the extended data bank (`player_table`) or not: a move
    /// only that table brings is left out of a fight without it.
    pub fn open_in_fight(fighter: TournamentFighter, preset: RulesetPreset, player_table: bool) -> Self {
        let entries = move_list(fighter, preset).into_iter().filter(|e| player_table || !e.available.player_table).copied().collect();
        Self::from_entries(fighter, entries)
    }

    /// The list of `entries`, sorted into sections.
    pub fn from_entries(fighter: TournamentFighter, entries: Vec<MoveEntry>) -> Self {
        let mut lines = Vec::new();
        for kind in MoveKind::SECTIONS {
            let mut section = entries.iter().enumerate().filter(|(_, e)| e.kind == kind).peekable();
            if section.peek().is_some() {
                lines.push(Line::Heading(kind.heading()));
                lines.extend(section.map(|(i, _)| Line::Move(i)));
            }
        }
        lines.push(Line::Heading("NORMALS"));
        lines.push(Line::Normals);
        let cursor = lines.iter().position(|l| matches!(l, Line::Move(_)));
        let mut panel = MovesPanel {
            fighter,
            display_name: fighter.name(),
            playtest: None,
            entries,
            lines,
            cursor,
            top: 0,
            training: false,
            choosing: None,
            shown: None,
            tried: None,
            notation: InputNotation::default(),
            facing_right: true,
        };
        panel.scroll_to_cursor();
        panel
    }

    /// The panel of a Training fight: confirm on a move offers Show and Try.
    pub fn with_show(mut self, training: bool) -> Self {
        self.training = training;
        self
    }

    pub(super) fn with_evil_ryu_playtest_moves(mut self, playtest: EvilRyuPlaytest) -> Self {
        if playtest.is_kage() || playtest == EvilRyuPlaytest::Sf6 {
            self.display_name = playtest.display_name();
            self.playtest = Some(playtest);
            let demon = match playtest {
                EvilRyuPlaytest::KageII => Some("SHUN GOKU SATSU (VT II)"),
                EvilRyuPlaytest::Sf6 => Some("SHUN GOKU SATSU (CA)"),
                _ => None,
            };
            if let Some(label) = demon {
                if let Some(at) = self.lines.iter().rposition(|line| {
                    matches!(*line, Line::Move(i) if self.entries[i].kind == MoveKind::Super)
                }) {
                    if playtest == EvilRyuPlaytest::Sf6 {
                        self.lines.insert(at + 1, Line::Heading("CRITICAL ART (25% LIFE, 3 STOCKS)"));
                        self.lines.insert(at + 2, Line::RagingDemon(label));
                    } else {
                        self.lines.insert(at + 1, Line::RagingDemon(label));
                    }
                }
            }
            return self;
        }
        let usf4 = playtest == EvilRyuPlaytest::Usf4;
        let sf4 = playtest.is_ae2012() || usf4;
        if playtest == EvilRyuPlaytest::UltraSf2 {
            self.entries.retain(|entry| !matches!(entry.name, "SHINKU HADOKEN" | "SHAKUNETSU HADOKEN"));
            if let Some(uppercut) = self.entries.iter_mut().find(|entry| entry.name == "SHORYUKEN") {
                uppercut.name = "METSU SHORYUKEN";
                uppercut.call_out = "METSU SHORYUKEN!";
            }
            if let Some(i) = self.entries.iter().position(|entry| entry.name == "HADOKEN") {
                self.entries.insert(i + 1, MoveEntry {
                    name: "SHAKUNETSU HADOKEN",
                    call_out: "SHAKUNETSU HADOKEN!",
                    ..self.entries[i]
                });
            }
        }
        if matches!(playtest, EvilRyuPlaytest::Alpha3A | EvilRyuPlaytest::Alpha3V) {
            self.entries.retain(|entry| entry.name != "ALPHA COUNTER (K)");
            if let Some(counter) = self.entries.iter_mut().find(|entry| entry.recognizer == Recognizer::AlphaCounter) {
                counter.name = if playtest == EvilRyuPlaytest::Alpha3V {
                    "ALPHA COUNTER SWEEP"
                } else {
                    "ALPHA COUNTER (BLOCK)"
                };
                counter.input = MoveInput { motion: &[Dir::Forward], button: ButtonClass::SameStrength };
            }
        }
        if matches!(playtest, EvilRyuPlaytest::Alpha3V | EvilRyuPlaytest::Alpha3X) {
            self.entries.retain(|entry| entry.kind != MoveKind::Super);
        }
        if playtest == EvilRyuPlaytest::Alpha3V {
            if let Some(combo) = self.entries.iter_mut().find(|entry| entry.name == "CUSTOM COMBO") {
                combo.name = "VARIABLE COMBO (50%+)";
            }
        }
        if sf4 {
            self.entries.retain(|entry| entry.kind != MoveKind::Throw);
        }
        if usf4 {
            if let Some(red_focus) = self.entries.iter_mut().find(|entry| entry.name == "RED FOCUS ATTACK") {
                red_focus.name = "RED FOCUS (2 STOCKS)";
            }
        }
        let mut panel = Self::from_entries(self.fighter, self.entries);
        panel.display_name = "Evil Ryu";
        panel.playtest = Some(playtest);
        let (ashura_button, ashura_label) = match playtest {
            EvilRyuPlaytest::UltraSf2 => (ButtonClass::Kick, None),
            EvilRyuPlaytest::Alpha2
            | EvilRyuPlaytest::Alpha3A
            | EvilRyuPlaytest::Alpha3V
            | EvilRyuPlaytest::Alpha3X
            | EvilRyuPlaytest::Ae2012
            | EvilRyuPlaytest::Ae2012Ultra2
            | EvilRyuPlaytest::Usf4 => (ButtonClass::TriplePunch, Some("PPP/KKK")),
            EvilRyuPlaytest::Kage | EvilRyuPlaytest::KageII | EvilRyuPlaytest::Sf6 => {
                unreachable!("This profile uses the native move list")
            }
        };
        if let Some(last_special) = panel.lines.iter().rposition(|line| {
            matches!(*line, Line::Move(i) if panel.entries[i].kind == MoveKind::Special)
        }) {
            let at = last_special + 1;
            panel.lines.splice(at..at, [
                Line::Command("ASHURA SENKU FORWARD", MoveInput {
                    motion: &[Dir::Forward, Dir::Down, Dir::DownForward], button: ashura_button,
                }, ashura_label),
                Line::Command("ASHURA SENKU BACKWARD", MoveInput {
                    motion: &[Dir::Back, Dir::Down, Dir::DownBack], button: ashura_button,
                }, ashura_label),
            ]);
            if matches!(playtest, EvilRyuPlaytest::UltraSf2 | EvilRyuPlaytest::Alpha3X) {
                let at = at + 2;
                panel.lines.splice(at..at, [
                    Line::Heading("SUPER (FULL GAUGE)"),
                    Line::RagingDemon("RAGING DEMON"),
                ]);
            }
        }
        if sf4 {
            if let Some(at) = panel.lines.iter().position(|line| *line == Line::Heading(MoveKind::Super.heading())) {
                panel.lines[at] = Line::Heading(if usf4 { "ULTRA DOUBLE (75%)" } else { "ULTRA (REVENGE)" });
                panel.lines.splice(at..at, [
                    Line::Heading("SUPER (FULL GAUGE)"),
                    Line::RagingDemon("RAGING DEMON"),
                ]);
            }
            if let Some(at) = panel.lines.iter().position(|line| {
                matches!(*line, Line::Move(i) if panel.entries[i].name == "RED FOCUS (2 STOCKS)")
            }) {
                panel.lines.insert(at + 1, Line::Command("EX RED FOCUS (3 STOCKS)", MoveInput {
                    motion: &[], button: ButtonClass::RedFocus,
                }, Some("CANCEL LP+MP+MK")));
            }
            if let Some(at) = panel.lines.iter().position(|line| {
                matches!(*line, Line::Move(i) if panel.entries[i].name == "FOCUS ATTACK")
            }) {
                panel.lines.insert(at + 1, Line::Command("FADC (2 STOCKS)", MoveInput {
                    motion: &[], button: ButtonClass::MediumPair,
                }, Some("MP+MK THEN DASH")));
            }
            let at = panel.lines.len() - 2;
            panel.lines.splice(at..at, [
                Line::Heading(MoveKind::Throw.heading()),
                Line::Command("SHOULDER THROW (CLOSE)", MoveInput {
                    motion: &[Dir::Forward], button: ButtonClass::LightPair,
                }, None),
                Line::Command("TOMOE NAGE (CLOSE)", MoveInput {
                    motion: &[Dir::Back], button: ButtonClass::LightPair,
                }, None),
            ]);
        }
        if playtest == EvilRyuPlaytest::Alpha2 {
            if let Some(last_special) = panel.lines.iter().rposition(|line| {
                matches!(*line, Line::Command("ASHURA SENKU BACKWARD", ..))
            }) {
                panel.lines.insert(last_special + 1, Line::Command("HADO NO KAMAE (FAKE)", MoveInput {
                    motion: &[Dir::Down, Dir::DownForward, Dir::Forward], button: ButtonClass::Punch,
                }, Some("START")));
            }
        }
        if playtest == EvilRyuPlaytest::Alpha2 || playtest.is_alpha3() {
            let at = panel.lines.len() - 2;
            if playtest.is_alpha3() {
                panel.lines.splice(at..at, [
                    Line::Heading(MoveKind::Throw.heading()),
                    Line::CloseThrow("SHOULDER THROW", "PP"),
                    Line::CloseThrow("AIR SHOULDER THROW", "PP"),
                    Line::CloseThrow("TOMOE NAGE", "KK"),
                ]);
            } else {
                panel.lines.insert(at, Line::CloseThrow("TOMOE NAGE", "MK/HK"));
            }
            let at = panel.lines.len() - 2;
            let command_normals = panel.entries.iter().enumerate()
                .filter(|(_, entry)| entry.kind == MoveKind::Normal)
                .map(|(index, _)| Line::Move(index));
            panel.lines.splice(at..at, std::iter::once(Line::Heading(MoveKind::Normal.heading())).chain(command_normals));
            if let Some(last_super) = panel.lines.iter().rposition(|line| {
                matches!(*line, Line::Move(i) if panel.entries[i].kind == MoveKind::Super)
            }) {
                panel.lines.insert(last_super + 1, Line::RagingDemon("RAGING DEMON (LV 3)"));
            }
        }
        panel
    }

    /// The choice up on the cursor's move, if there is one.
    pub fn choosing(&self) -> Option<Choice> {
        self.choosing
    }

    /// The move Show was chosen for: the panel hands it over once, and the
    /// session has the fighter perform it.
    pub fn take_show(&mut self) -> Option<MoveEntry> {
        self.shown.take()
    }

    /// The move Try was chosen for: the panel hands it over once, and the
    /// session has the coach watch the player attempt it.
    pub fn take_try(&mut self) -> Option<MoveEntry> {
        self.tried.take()
    }

    /// The list with its inputs written in `notation`, the arrows for a
    /// fighter facing right or left.
    pub fn with_inputs(mut self, notation: InputNotation, facing_right: bool) -> Self {
        self.notation = notation;
        self.facing_right = facing_right;
        self
    }

    /// How the inputs are written.
    pub fn notation(&self) -> InputNotation {
        self.notation
    }

    /// Whether the arrows are drawn for a fighter facing right.
    pub fn facing_right(&self) -> bool {
        self.facing_right
    }

    /// A move's input as this list writes it.
    fn written(&self, entry: &MoveEntry) -> Vec<InputPiece> {
        let button_label = match (self.playtest, entry.name) {
            (Some(EvilRyuPlaytest::UltraSf2), "HADOKEN") => Some("LP/MP"),
            (Some(EvilRyuPlaytest::UltraSf2), "SHAKUNETSU HADOKEN") => Some("HP"),
            (Some(EvilRyuPlaytest::Alpha2), "CUSTOM COMBO") => Some("PPK/KKP"),
            (Some(EvilRyuPlaytest::Alpha3A), "ALPHA COUNTER (BLOCK)") => Some("P+K SAME LV"),
            (Some(EvilRyuPlaytest::Alpha3V), "VARIABLE COMBO (50%+)") => Some("P+K SAME LV"),
            (Some(EvilRyuPlaytest::Alpha3V), "ALPHA COUNTER SWEEP") => {
                let mut input = self.write_input(entry.input, Some("P+K SAME"));
                input.insert(0, InputPiece::Text("BLOCK ".to_owned()));
                return input;
            }
            _ => None,
        };
        let mut input = self.write_input(entry.input, button_label);
        if matches!(self.playtest, Some(EvilRyuPlaytest::Ae2012 | EvilRyuPlaytest::Usf4))
            && entry.name == "METSU HADOKEN"
        {
            input.push(InputPiece::Text(" HOLD/RELEASE".to_owned()));
        }
        if self.playtest == Some(EvilRyuPlaytest::Kage) && entry.name == "SENHA KASSATSU" {
            input.push(InputPiece::Text(" HOLD TO CHARGE".to_owned()));
        }
        input
    }

    fn write_input(&self, input: MoveInput, button_label: Option<&str>) -> Vec<InputPiece> {
        let mut written = input.write(self.notation, self.facing_right);
        if let Some(button_label) = button_label {
            match written.last_mut() {
                Some(InputPiece::Button(label)) => *label = button_label.to_owned(),
                Some(InputPiece::Text(text)) => {
                    if let Some(motion) = text.strip_suffix(input.button.label()) {
                        *text = format!("{motion}{button_label}");
                    }
                }
                None => {}
            }
        }
        written
    }

    fn close_throw_input(&self, buttons: &str) -> Vec<InputPiece> {
        let forward = Dir::Forward.written(self.notation, self.facing_right);
        let back = Dir::Back.written(self.notation, self.facing_right);
        vec![
            InputPiece::Text(format!("CLOSE {forward}/{back} ")),
            InputPiece::Button(buttons.to_owned()),
        ]
    }

    fn raging_demon_input(&self) -> Vec<InputPiece> {
        let forward = crate::moves::Dir::Forward.written(self.notation, self.facing_right);
        vec![
            InputPiece::Button("LP".to_owned()),
            InputPiece::Text(" ".to_owned()),
            InputPiece::Button("LP".to_owned()),
            InputPiece::Text(format!(" {forward} ")),
            InputPiece::Button("LK".to_owned()),
            InputPiece::Text(" ".to_owned()),
            InputPiece::Button("HP".to_owned()),
        ]
    }

    pub fn name(&self) -> &'static str {
        "moves"
    }

    /// The fighter whose list this is.
    pub fn fighter(&self) -> TournamentFighter {
        self.fighter
    }

    /// The list's text, line by line: the headings, the rows (`NAME` and
    /// `INPUT` in the chosen notation, a button in square brackets) and the
    /// normals line, for tests.
    pub fn text(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|line| match *line {
                Line::Heading(h) => h.to_owned(),
                Line::Move(i) => format!("{} {}", self.entries[i].name, input_text(&self.written(&self.entries[i]))),
                Line::Command(name, input, button) => format!("{} {}", name, input_text(&self.write_input(input, button))),
                Line::RagingDemon(name) => format!("{} {}", name, input_text(&self.raging_demon_input())),
                Line::CloseThrow(name, buttons) => format!("{} {}", name, input_text(&self.close_throw_input(buttons))),
                Line::Normals => NORMALS_LINE.to_owned(),
            })
            .collect()
    }

    /// The entry the cursor stands on.
    pub fn selected(&self) -> Option<&MoveEntry> {
        match self.lines.get(self.cursor?)? {
            Line::Move(i) => self.entries.get(*i),
            _ => None,
        }
    }

    /// The first line shown.
    pub fn top(&self) -> usize {
        self.top
    }

    /// Whether the list is longer than the panel.
    pub fn scrolls(&self) -> bool {
        self.lines.len() > LIST_ROWS
    }

    /// Up and down move the cursor between moves (wrapping), or scroll a
    /// list with no move; back returns to the pause box and Start resumes
    /// the fight. In Training confirm on a move offers Show and Try: left
    /// and right pick, confirm resumes the fight with the show or the try
    /// taken by [`Self::take_show`] or [`Self::take_try`], and back drops
    /// the choice first. The rest does nothing.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        if let Some(choice) = self.choosing {
            return match input {
                MenuInput::Left | MenuInput::Right => {
                    self.choosing = Some(choice.other());
                    Some(Transition::CursorMoved)
                }
                MenuInput::Confirm => {
                    let picked = self.selected().copied();
                    match choice {
                        Choice::Show => self.shown = picked,
                        Choice::Try => self.tried = picked,
                    }
                    self.choosing = None;
                    Some(Transition::Choose(MenuItem::Resume))
                }
                MenuInput::Back => {
                    self.choosing = None;
                    Some(Transition::CursorMoved)
                }
                MenuInput::Start => Some(Transition::Choose(MenuItem::Resume)),
                _ => None,
            };
        }
        match input {
            MenuInput::Confirm if self.training && self.selected().is_some() => {
                self.choosing = Some(Choice::Show);
                Some(Transition::CursorMoved)
            }
            MenuInput::Up | MenuInput::Down => {
                let down = input == MenuInput::Down;
                match self.cursor {
                    Some(at) => {
                        let moves: Vec<usize> = (0..self.lines.len()).filter(|&l| matches!(self.lines[l], Line::Move(_))).collect();
                        let i = moves.iter().position(|&l| l == at)?;
                        let next = if down { (i + 1) % moves.len() } else { (i + moves.len() - 1) % moves.len() };
                        if moves.len() < 2 {
                            return None;
                        }
                        self.cursor = Some(moves[next]);
                        self.scroll_to_cursor();
                    }
                    None => {
                        let last = self.lines.len().saturating_sub(LIST_ROWS);
                        let top = if down { (self.top + 1).min(last) } else { self.top.saturating_sub(1) };
                        if top == self.top {
                            return None;
                        }
                        self.top = top;
                    }
                }
                Some(Transition::CursorMoved)
            }
            MenuInput::Back => Some(Transition::Back),
            // Start resumes the fight, as it does from the pause box.
            MenuInput::Start => Some(Transition::Choose(MenuItem::Resume)),
            _ => None,
        }
    }

    /// Scroll so the cursor's line shows, with its heading when it is the
    /// first of its section.
    fn scroll_to_cursor(&mut self) {
        let Some(at) = self.cursor else { return };
        let first = if at > 0 && matches!(self.lines[at - 1], Line::Heading(_)) { at - 1 } else { at };
        if first < self.top {
            self.top = first;
        } else if at >= self.top + LIST_ROWS {
            self.top = at + 1 - LIST_ROWS;
        }
        // On the last move the view runs on over the lines after it (the
        // normals), so they can be scrolled into view.
        if self.lines[at + 1..].iter().all(|l| !matches!(l, Line::Move(_))) {
            self.top = self.top.max(self.lines.len().saturating_sub(LIST_ROWS));
        }
    }

    /// The panel layer: the gold frame with the fighter's name in its top
    /// edge, the list (headings in their brackets, `NAME ..... INPUT`, the
    /// cursor row yellow, the normals line) and the hint at the foot.
    pub fn panel(&self, font: &RomFont<'_>) -> Panel {
        let (width, height) = (MOVES_COLS * 8, MOVES_ROWS * 8);
        // The font draws into frames a game frame wide: draw there, then
        // take the panel's columns.
        let mut argb = vec![BLACK; WIDTH * height];
        let header = fit(&self.display_name.to_ascii_uppercase(), (MOVES_COLS - 4) / 2);
        let gap = (MOVES_COLS - 2 * header.chars().count()) / 2;
        font.gold_frame(&mut argb, 0, 1, MOVES_COLS, MOVES_ROWS - 1, gap..gap + 2 * header.chars().count());
        font.header(&mut argb, gap, 0, &header, ink::BLUE);
        let centred = |argb: &mut [u32], row: usize, text: &str, color: u8| {
            font.text(argb, MOVES_COLS.saturating_sub(text.chars().count()) / 2, row, text, color);
        };
        for (n, &line) in self.lines.iter().enumerate().skip(self.top).take(LIST_ROWS) {
            let row = LIST_ROW + n - self.top;
            let (name, input) = match line {
                Line::Heading(h) => {
                    centred(&mut argb, row, &format!("[ {h} ]"), ink::BLUE);
                    continue;
                }
                Line::Normals => {
                    centred(&mut argb, row, NORMALS_LINE, ink::BLUE);
                    continue;
                }
                Line::Move(i) => (self.entries[i].name, self.written(&self.entries[i])),
                Line::Command(name, input, button) => (name, self.write_input(input, button)),
                Line::RagingDemon(name) => (name, self.raging_demon_input()),
                Line::CloseThrow(name, buttons) => (name, self.close_throw_input(buttons)),
            };
            let color = if self.cursor == Some(n) {
                font.text(&mut argb, LABEL_COL - 1, row, ">", ink::YELLOW);
                ink::YELLOW
            } else {
                ink::BLUE
            };
            font.text(&mut argb, LABEL_COL, row, name, color);
            let vcol = VALUE_END_COL - input_cells(&input);
            let dots_from = LABEL_COL + name.chars().count() + 1;
            if vcol > dots_from + 1 {
                font.text(&mut argb, dots_from, row, &".".repeat(vcol - 1 - dots_from), ink::DIM);
            }
            let mut col = vcol;
            for piece in &input {
                match piece {
                    InputPiece::Text(t) => font.text(&mut argb, col, row, t, color),
                    InputPiece::Button(b) => font.button(&mut argb, col, row, b, color),
                }
                col += piece.cells();
            }
        }
        if self.scrolls() {
            centred(&mut argb, HINT_ROW - 1, "UP DOWN SCROLLS", ink::DIM);
        }
        if let Some(choice) = self.choosing {
            // SHOW and TRY side by side, the picked one yellow.
            let (show, try_) = (Choice::Show.label(), Choice::Try.label());
            let col = (MOVES_COLS - (show.len() + 4 + try_.len())) / 2;
            let color = |c: Choice| if c == choice { ink::YELLOW } else { ink::BLUE };
            font.text(&mut argb, col, CHOICE_ROW, show, color(Choice::Show));
            font.text(&mut argb, col + show.len() + 4, CHOICE_ROW, try_, color(Choice::Try));
        }
        let hint = if self.choosing.is_some() { "CONFIRM CHOOSES  BACK DROPS IT" } else { "BACK RETURNS TO THE PAUSE BOX" };
        centred(&mut argb, HINT_ROW, hint, ink::DIM);
        let argb = argb.chunks_exact(WIDTH).flat_map(|line| line[..width].iter().copied()).collect();
        Panel {
            side: PanelSide::Left,
            width,
            height,
            argb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::REGISTRY;

    fn ryu() -> MovesPanel {
        MovesPanel::open(TournamentFighter::Ryu, RulesetPreset::Faithful1987).with_inputs(InputNotation::Words, true)
    }

    #[test]
    fn ryus_list_has_the_specials_section_then_the_normals() {
        assert_eq!(
            ryu().text(),
            ["SPECIALS", "HADOKEN QCF+P", "SHORYUKEN DP+P", "TATSUMAKI QCB+K", "NORMALS", NORMALS_LINE]
        );
    }

    #[test]
    fn the_inputs_are_written_in_the_chosen_notation_and_for_the_facing() {
        let rows = |n, right| {
            MovesPanel::open(TournamentFighter::Ryu, RulesetPreset::Faithful1987).with_inputs(n, right).text()[1..4].to_vec()
        };
        assert_eq!(MovesPanel::open(TournamentFighter::Ryu, RulesetPreset::Faithful1987).notation(), InputNotation::Arrows, "arrows by default");
        assert_eq!(rows(InputNotation::Arrows, true)[0], "HADOKEN \u{2193}\u{2198}\u{2192} [P]");
        assert_eq!(rows(InputNotation::Arrows, false)[0], "HADOKEN \u{2193}\u{2199}\u{2190} [P]");
        assert_eq!(rows(InputNotation::Numpad, false), ["HADOKEN 236 P", "SHORYUKEN 623 P", "TATSUMAKI 214 K"]);
        assert_eq!(rows(InputNotation::Words, false), ["HADOKEN QCF+P", "SHORYUKEN DP+P", "TATSUMAKI QCB+K"]);
    }

    #[test]
    fn a_boss_has_the_normals_only() {
        let boss = MovesPanel::open(TournamentFighter::Sagat, RulesetPreset::Faithful1987);
        assert_eq!(boss.text(), ["NORMALS", NORMALS_LINE]);
        assert!(boss.selected().is_none());
    }

    #[test]
    fn the_cursor_stands_on_the_first_move_and_wraps_over_moves_only() {
        let mut p = ryu();
        assert_eq!(p.selected().map(|e| e.name), Some("HADOKEN"));
        assert_eq!(p.apply(MenuInput::Down), Some(Transition::CursorMoved));
        assert_eq!(p.selected().map(|e| e.name), Some("SHORYUKEN"));
        p.apply(MenuInput::Down);
        p.apply(MenuInput::Down);
        assert_eq!(p.selected().map(|e| e.name), Some("HADOKEN"), "wraps past the last move");
        p.apply(MenuInput::Up);
        assert_eq!(p.selected().map(|e| e.name), Some("TATSUMAKI"));
        assert_eq!(p.apply(MenuInput::Confirm), None, "outside Training confirm does nothing");
    }

    #[test]
    fn back_returns_to_the_pause_box_and_start_resumes() {
        assert_eq!(ryu().apply(MenuInput::Back), Some(Transition::Back));
        assert_eq!(ryu().apply(MenuInput::Start), Some(Transition::Choose(MenuItem::Resume)));
    }

    #[test]
    fn try_is_chosen_like_show_and_handed_over_once() {
        let mut p = ryu().with_show(true);
        p.apply(MenuInput::Down);
        assert_eq!(p.apply(MenuInput::Confirm), Some(Transition::CursorMoved));
        assert_eq!(p.choosing(), Some(Choice::Show));
        assert_eq!(p.apply(MenuInput::Right), Some(Transition::CursorMoved));
        assert_eq!(p.choosing(), Some(Choice::Try));
        assert_eq!(p.apply(MenuInput::Confirm), Some(Transition::Choose(MenuItem::Resume)));
        assert_eq!(p.take_try().map(|e| e.name), Some("SHORYUKEN"));
        assert!(p.take_try().is_none(), "once");
        assert!(p.take_show().is_none(), "Show was not chosen");
        // Left comes back to SHOW, and back drops the choice.
        p.apply(MenuInput::Confirm);
        p.apply(MenuInput::Right);
        assert_eq!(p.apply(MenuInput::Left), Some(Transition::CursorMoved));
        assert_eq!(p.choosing(), Some(Choice::Show));
        assert_eq!(p.apply(MenuInput::Back), Some(Transition::CursorMoved));
        assert!(p.choosing().is_none() && p.take_try().is_none());
    }

    #[test]
    fn a_list_longer_than_the_panel_scrolls_with_the_cursor() {
        let entries = vec![REGISTRY[0]; 30];
        let mut p = MovesPanel::from_entries(TournamentFighter::Ryu, entries);
        assert!(p.scrolls());
        assert_eq!(p.top(), 0);
        for _ in 0..25 {
            p.apply(MenuInput::Down);
        }
        // Heading, 30 moves, normals heading and line: the cursor is on
        // the 26th move (line 26), and shows at the foot of the window.
        assert_eq!(p.top(), 26 + 1 - LIST_ROWS);
        // On the last move the normals scroll into view too: 33 lines.
        for _ in 0..4 {
            p.apply(MenuInput::Down);
        }
        assert_eq!(p.top(), 33 - LIST_ROWS, "the normals heading and line show");
        for _ in 0..29 {
            p.apply(MenuInput::Up);
        }
        assert_eq!(p.top(), 0, "back at the first move its heading shows too");
        // Up from the first move wraps to the last, scrolling to it.
        p.apply(MenuInput::Up);
        assert_eq!(p.top(), 33 - LIST_ROWS);
    }

    #[test]
    fn a_list_with_no_move_scrolls_nothing_when_it_fits() {
        let mut boss = MovesPanel::open(TournamentFighter::Sagat, RulesetPreset::Faithful1987);
        assert_eq!(boss.apply(MenuInput::Down), None);
        assert_eq!(boss.top(), 0);
    }
}
