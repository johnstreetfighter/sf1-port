//! The move registry (spec #226, ML1; design `docs/design/2026-11-06-move-list-design.md`):
//! the moves a fighter has in real fights, each with the input that brings
//! it out, and the move list the pause box's MOVES row shows as the
//! registry filtered by the fight.
//!
//! An entry holds the move's display name, its [`MoveKind`], its input as
//! notation-free data ([`MoveInput`]: facing-relative directions and a
//! button class), the [`Recognizer`] that reports its progress, the engine
//! move it produces, and when it is available ([`Availability`]: fighters
//! and presets). Real fights run the 1987 engine, so the first entries are
//! Ryu's and Ken's Hadoken, Shoryuken and Tatsumaki (moves `0x1a`, `0x19`,
//! `0x1b`; `docs/reference/specials.md`, `hadoken.md`). A feature that
//! brings another move into real fights adds its entry here, its expected
//! move to `tests/move_list.rs` and a read-only view of its recognizer
//! (ADR-0012).
//!
//! Nothing here writes the machine (ADR-0004): [`Recognizer::read`] and
//! [`Recognizer::view`] take the machine's memory as shared slices, and
//! [`MoveInput::frames`] only builds the player inputs a pad would give.
//! The view is what Training's coach reads (ticket #231, ML5).

use crate::config::{CoreToggles, RulesetPreset};
use crate::fighter::{busy_banked_jump, CPU_STRUCT, P1_STRUCT, P2_STRUCT, WORK_BASE};
use crate::front::PlayerInputs;
use crate::roster::TournamentFighter;

/// What sort of move an entry is: the move list's sections, in order, with
/// the normals last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveKind {
    Special,
    Super,
    System,
    Throw,
    /// A command normal, which needs a direction with its button. The six
    /// plain normals are no entries: they are the list's one compact line.
    Normal,
}

impl MoveKind {
    /// The sections a list shows, in order.
    pub const SECTIONS: [MoveKind; 4] = [MoveKind::Special, MoveKind::Super, MoveKind::System, MoveKind::Throw];

    /// The section's heading in the panel.
    pub const fn heading(self) -> &'static str {
        match self {
            MoveKind::Special => "SPECIALS",
            MoveKind::Super => "SUPER",
            MoveKind::System => "SYSTEM",
            MoveKind::Throw => "THROWS",
            MoveKind::Normal => "COMMAND NORMALS",
        }
    }
}

/// A direction relative to the way the fighter faces: forward is toward
/// the opponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Neutral,
    Forward,
    Back,
    Up,
    Down,
    UpForward,
    UpBack,
    DownForward,
    DownBack,
    ChargeBack,
    ChargeDown,
    Jump,
}

pub(crate) const CHARGE_FRAMES: usize = 40;

impl Dir {
    /// The facing-right input nibble used by the native fight loop.
    pub(crate) const fn bits(self) -> u8 {
        match self {
            Dir::Neutral => 0,
            Dir::Forward => 1,
            Dir::Back => 2,
            Dir::Down => 4,
            Dir::Up | Dir::Jump => 8,
            Dir::DownForward => 5,
            Dir::DownBack => 6,
            Dir::UpForward => 9,
            Dir::UpBack => 10,
            Dir::ChargeBack => 2,
            Dir::ChargeDown => 4,
        }
    }

    pub(crate) const fn is_charge(self) -> bool {
        matches!(self, Self::ChargeBack | Self::ChargeDown)
    }

    const fn frames(self) -> usize {
        match self {
            Self::ChargeBack | Self::ChargeDown => CHARGE_FRAMES,
            Self::Jump => 12,
            _ => STEP_FRAMES,
        }
    }

    /// The stick's right, left, down and up for this direction when the
    /// fighter faces right (`facing_right`) or left.
    fn pressed(self, facing_right: bool) -> [bool; 4] {
        let (forward, back) = if facing_right { (0, 1) } else { (1, 0) };
        let mut d = [false; 4];
        match self {
            Dir::Neutral => {},
            Dir::Forward => d[forward] = true,
            Dir::Back | Dir::ChargeBack => d[back] = true,
            Dir::Up | Dir::Jump => d[3] = true,
            Dir::Down | Dir::ChargeDown => d[2] = true,
            Dir::UpForward => (d[3], d[forward]) = (true, true),
            Dir::UpBack => (d[3], d[back]) = (true, true),
            Dir::DownForward => (d[2], d[forward]) = (true, true),
            Dir::DownBack => (d[2], d[back]) = (true, true),
        }
        d
    }
}

/// The button or chord that completes a move command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonClass {
    Punch,
    Kick,
    Either,
    MashPunch,
    TriplePunch,
    TripleKick,
    ReleasePair,
    PairPunch,
    PairKick,
    MediumPair,
    HeavyPair,
    ShiftPair,
    RedFocus,
    SameStrength,
    LightPair,
    MediumPunch,
    MediumKick,
}

impl ButtonClass {
    /// The letter the move list writes.
    pub const fn letter(self) -> char {
        match self {
            Self::Kick | Self::TripleKick | Self::PairKick | Self::MediumKick => 'K',
            _ => 'P',
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Punch => "P",
            Self::Kick => "K",
            Self::Either => "P/K",
            Self::MashPunch => "TAP P",
            Self::TriplePunch => "PPP",
            Self::TripleKick => "KKK",
            Self::ReleasePair => "HOLD PP/KK, RELEASE",
            Self::PairPunch => "PP",
            Self::PairKick => "KK",
            Self::MediumPair => "MP+MK",
            Self::HeavyPair => "HP+HK",
            Self::ShiftPair => "HP+MK",
            Self::RedFocus => "LP+MP+MK",
            Self::SameStrength => "P+K",
            Self::LightPair => "LP+LK",
            Self::MediumPunch => "MP",
            Self::MediumKick => "MK",
        }
    }

    pub const fn kick(self) -> bool {
        matches!(self, Self::Kick | Self::TripleKick | Self::PairKick | Self::MediumKick)
    }

    /// The other class.
    pub const fn other(self) -> ButtonClass {
        if self.kick() { Self::Punch } else { Self::Kick }
    }

    /// Whether the required buttons are down (LP, MP, HP, LK, MK, HK).
    pub fn down(self, buttons: [bool; 6]) -> bool {
        self.held(buttons.into_iter().enumerate().fold(0, |mask, (index, down)| {
            mask | (u8::from(down) << index)
        }))
    }

    /// Whether the required buttons are held in the native six-bit pad mask.
    pub(crate) fn held(self, buttons: u8) -> bool {
        let punches = buttons & 0b000111;
        let kicks = buttons >> 3 & 0b000111;
        match self {
            Self::Punch | Self::MashPunch => punches != 0,
            Self::Kick => kicks != 0,
            Self::Either => punches | kicks != 0,
            Self::TriplePunch => punches == 7,
            Self::TripleKick => kicks == 7,
            Self::PairPunch => punches.count_ones() >= 2,
            Self::PairKick => kicks.count_ones() >= 2,
            Self::ReleasePair => punches.count_ones() >= 2 || kicks.count_ones() >= 2,
            Self::SameStrength => punches & kicks != 0,
            chord => {
                let required = chord.fixed_mask();
                buttons & required == required
            }
        }
    }

    pub(crate) const fn fixed_mask(self) -> u8 {
        match self {
            Self::MediumPair => 0b010010,
            Self::HeavyPair => 0b100100,
            Self::ShiftPair => 0b010100,
            Self::RedFocus => 0b010011,
            Self::LightPair => 0b001001,
            Self::MediumPunch => 0b000010,
            Self::MediumKick => 0b010000,
            _ => 0,
        }
    }

    fn press_mask(self, level: usize) -> u8 {
        let strength = 1 << level;
        let adjacent = 1 << ((level + 1) % 3);
        match self {
            Self::PairPunch => strength | adjacent,
            Self::PairKick => (strength | adjacent) << 3,
            Self::SameStrength => strength | (strength << 3),
            Self::Kick => strength << 3,
            Self::Punch | Self::Either => strength,
            chord => chord.fixed_mask(),
        }
    }
}

/// How hard the button is pressed: light, medium or heavy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strength {
    Light,
    Medium,
    Heavy,
}

/// How the move list writes an input (ticket #228, ML2; the Settings
/// row `MOVE INPUTS`): drawn arrows, numpad numbers or motion words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputNotation {
    /// Arrow tiles for the fighter's facing, the button in a small chip.
    #[default]
    Arrows,
    /// `236 P`, always written facing right.
    Numpad,
    /// `QCF+P`.
    Words,
}

impl InputNotation {
    pub const ALL: [InputNotation; 3] = [InputNotation::Arrows, InputNotation::Numpad, InputNotation::Words];

    /// The Settings value, in the ROM font's character set.
    pub const fn label(self) -> &'static str {
        match self {
            InputNotation::Arrows => "ARROWS",
            InputNotation::Numpad => "NUMPAD",
            InputNotation::Words => "WORDS",
        }
    }

    /// The `settings.toml` value.
    pub const fn slug(self) -> &'static str {
        match self {
            InputNotation::Arrows => "arrows",
            InputNotation::Numpad => "numpad",
            InputNotation::Words => "words",
        }
    }

    pub fn from_slug(s: &str) -> Option<InputNotation> {
        InputNotation::ALL.into_iter().find(|n| n.slug() == s)
    }

    /// The notation after this one, wrapping.
    pub fn next(self) -> InputNotation {
        InputNotation::ALL[(self as usize + 1) % InputNotation::ALL.len()]
    }

    /// The notation before this one, wrapping.
    pub fn prev(self) -> InputNotation {
        InputNotation::ALL[(self as usize + InputNotation::ALL.len() - 1) % InputNotation::ALL.len()]
    }
}

/// One piece of a written input: text in the ROM font (arrows included),
/// or a button's name, drawn in a small chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputPiece {
    Text(String),
    Button(String),
}

impl InputPiece {
    /// The cells the piece takes: a character a cell, a button a cell a letter.
    pub fn cells(&self) -> usize {
        match self {
            InputPiece::Text(t) | InputPiece::Button(t) => t.chars().count(),
        }
    }
}

/// The cells a written input takes.
pub fn input_cells(pieces: &[InputPiece]) -> usize {
    pieces.iter().map(InputPiece::cells).sum()
}

/// A written input as plain text, a button in square brackets (`↓↘→ [P]`),
/// for tests and logs.
pub fn input_text(pieces: &[InputPiece]) -> String {
    pieces
        .iter()
        .map(|p| match p {
            InputPiece::Text(t) => t.clone(),
            InputPiece::Button(b) => format!("[{b}]"),
        })
        .collect()
}

/// Whether the fighter whose player structure is at `player_struct` faces
/// right, read from the work RAM (`0xff8000..`) and the object RAM
/// (`0xffe000..`): the structure's `+0x10` holds the fighter's entity, and
/// the entity's facing byte at `+0x37` is 4 and over facing left (`0x43dde`).
/// Read only.
pub fn facing_right(work: &[u8], obj: &[u8], player_struct: u32) -> bool {
    let entity = entity_of(work, player_struct);
    obj.get(entity + 0x37).is_none_or(|&b| (b as i8) < 4)
}

/// The fighter entity's offset in object RAM, from the player structure's
/// `+0x10`.
fn entity_of(work: &[u8], player_struct: u32) -> usize {
    let at = (player_struct + 0x10 - WORK_BASE) as usize;
    work.get(at..at + 2).map_or(0, |b| usize::from(u16::from_be_bytes([b[0], b[1]])))
}

/// A move's input as data, independent of how the list writes it: the
/// directions to go through, facing-relative, then a button of a class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveInput {
    pub motion: &'static [Dir],
    pub button: ButtonClass,
}

/// Frames each step of a motion but the last is held when fed.
const STEP_FRAMES: usize = 3;
/// Frames the last step is held when fed: through the button and the walk
/// cycle's next interruptible frame (`hadoken.md`, "A note on the command's
/// timing"), and long enough for the back-held motions: the Tatsumaki's
/// scenarios hold back for 22 frames to get through the walk-back's gated
/// phase (`specials.md`, "Holding back through the gate").
const HOLD_FRAMES: usize = 22;
/// Frames the button is held, pressed on the second frame of the last step.
const BUTTON_FRAMES: usize = 4;

impl MoveInput {
    /// The motion's word, as the 90s guides write it (`QCF`, `DP`, `QCB`),
    /// or its directions' initials (`D,DF,F`) for a motion with no word.
    pub fn motion_word(&self) -> String {
        use Dir::*;
        match self.motion {
            [Down, DownForward, Forward] => "QCF".to_owned(),
            [Forward, Down, DownForward] => "DP".to_owned(),
            [Down, DownBack, Back] => "QCB".to_owned(),
            [Back, Down, DownBack] => "RDP".to_owned(),
            motion => motion.iter().map(|d| d.initials()).collect::<Vec<_>>().join(","),
        }
    }

    /// The input as the list writes it until arrows arrive: motion word,
    /// plus, button class (`QCF+P`).
    pub fn words(&self) -> String {
        if self.motion.is_empty() {
            self.button.label().to_owned()
        } else {
            format!("{}+{}", self.motion_word(), self.button.label())
        }
    }

    /// The input's steps as the Show strip labels them: each direction's
    /// initials, then the button's letter (`D`, `DF`, `F`, `P`).
    pub fn strip_steps(&self) -> Vec<String> {
        let mut steps: Vec<String> = self.motion.iter().map(|d| d.initials().to_owned()).collect();
        steps.push(self.button.label().to_owned());
        steps
    }

    /// How many of [`Self::strip_steps`] are fed once frame `frame` of
    /// [`Self::frames`] is: a motion step from its first frame, the button
    /// from the frame it is pressed.
    pub fn lit_at(&self, frame: usize) -> usize {
        let mut until = 0;
        for (index, direction) in self.motion.iter().take(self.motion.len().saturating_sub(1)).enumerate() {
            until += direction.frames();
            if frame < until {
                return index + 1;
            }
        }
        if self.motion.last() == Some(&Dir::Jump) { until += Dir::Jump.frames(); }
        let pressed = match self.button {
            ButtonClass::ReleasePair => frame >= CHARGE_FRAMES,
            ButtonClass::MashPunch => frame >= 12,
            _ => frame >= until + self.button_offset(),
        };
        self.motion.len() + usize::from(pressed)
    }

    fn button_offset(&self) -> usize {
        // A raw up input can start a jump immediately. Complete the
        // grounded command on that same frame.
        let dash = self.motion.ends_with(&[Dir::Neutral, Dir::Forward]);
        usize::from(!dash && !matches!(self.motion.last(), Some(Dir::Up | Dir::UpForward)))
    }

    /// The input written in `notation`. Arrows are drawn for a fighter
    /// facing right or left (`facing_right`), so they match the stick as
    /// held; numpad numbers are always written facing right; words are the
    /// guides' motion words.
    pub fn write(&self, notation: InputNotation, facing_right: bool) -> Vec<InputPiece> {
        match notation {
            InputNotation::Arrows => vec![
                InputPiece::Text(self.motion.iter().map(|d| d.written(notation, facing_right)).collect()),
                InputPiece::Text(" ".to_owned()),
                InputPiece::Button(self.button.label().to_owned()),
            ],
            InputNotation::Numpad => {
                vec![InputPiece::Text(format!("{} {}", self.motion.iter().map(|d| d.written(notation, facing_right)).collect::<String>(), self.button.label()))]
            }
            InputNotation::Words => vec![InputPiece::Text(self.words())],
        }
    }

    /// The player inputs that perform the input, one entry a frame, for a
    /// fighter facing right or left: each step but the last held for
    /// [`STEP_FRAMES`] frames, with additional time for charge or jump setup.
    /// The button is pressed on the last direction. This is what a pad gives
    /// the machine, and what Show feeds ([`crate::front::show`]).
    pub fn frames(&self, facing_right: bool, strength: Strength) -> Vec<PlayerInputs> {
        let level = match strength {
            Strength::Light => 0,
            Strength::Medium => 1,
            Strength::Heavy => 2,
        };
        let mut frames = Vec::new();
        for direction in self.motion.iter().take(self.motion.len().saturating_sub(1)) {
            frames.extend(std::iter::repeat_n(PlayerInputs {
                directions: direction.pressed(facing_right),
                ..PlayerInputs::default()
            }, direction.frames()));
        }
        let directions = self.motion.last().map_or([false; 4], |d| d.pressed(facing_right));
        let button = level + if self.button.kick() { 3 } else { 0 };
        let hold = if self.button == ButtonClass::ReleasePair {
            CHARGE_FRAMES
        } else if self.motion.last() == Some(&Dir::Jump) {
            Dir::Jump.frames()
        } else {
            0
        };
        let press = hold + self.button_offset();
        frames.extend((0..hold + HOLD_FRAMES).map(|frame| {
            let mut inputs = PlayerInputs { directions, ..PlayerInputs::default() };
            match self.button {
                ButtonClass::ReleasePair => inputs.buttons[..2].fill(frame < hold),
                ButtonClass::MashPunch => inputs.buttons[button] = frame <= 12 && frame % 4 == 0,
                ButtonClass::TriplePunch => inputs.buttons[..3].fill(frame == 1),
                ButtonClass::TripleKick => inputs.buttons[3..].fill(frame == 1),
                _ if (press..press + BUTTON_FRAMES).contains(&frame) => {
                    let mask = self.button.press_mask(level);
                    inputs.buttons = std::array::from_fn(|index| mask & (1 << index) != 0);
                }
                _ => {}
            }
            inputs
        }));
        frames
    }
}

impl Dir {
    /// The direction a stick held (right, left, down, up) gives a fighter
    /// facing right or left, facing-relative; none for neutral. Opposites
    /// held together cancel, as they do on a pad that cannot hold them.
    pub fn from_stick(directions: [bool; 4], facing_right: bool) -> Option<Dir> {
        let [right, left, down, up] = directions;
        let (forward, back) = if facing_right { (right, left) } else { (left, right) };
        let across = i8::from(forward) - i8::from(back);
        let vertical = i8::from(up) - i8::from(down);
        Some(match (across, vertical) {
            (0, 0) => return None,
            (1, 0) => Dir::Forward,
            (-1, 0) => Dir::Back,
            (0, 1) => Dir::Up,
            (0, -1) => Dir::Down,
            (1, 1) => Dir::UpForward,
            (-1, 1) => Dir::UpBack,
            (1, -1) => Dir::DownForward,
            _ => Dir::DownBack,
        })
    }

    /// The direction written in `notation`, for a fighter facing right or
    /// left: an arrow, a numpad digit (always facing right) or the
    /// direction's initials (`DF`).
    pub fn written(self, notation: InputNotation, facing_right: bool) -> String {
        if self == Self::Jump { return self.initials().to_owned(); }
        let value = match notation {
            InputNotation::Arrows => self.arrow(facing_right).to_string(),
            InputNotation::Numpad => self.numpad().to_string(),
            InputNotation::Words => return self.initials().to_owned(),
        };
        if self.is_charge() { format!("[{value}]") } else { value }
    }

    /// The arrow that draws the direction for a fighter facing right or
    /// left: forward is toward the opponent, so it points right when the
    /// fighter faces right.
    pub const fn arrow(self, facing_right: bool) -> char {
        match (self, facing_right) {
            (Dir::Neutral, _) => '.',
            (Dir::Forward, true) | (Dir::Back | Dir::ChargeBack, false) => '\u{2192}',
            (Dir::Forward, false) | (Dir::Back | Dir::ChargeBack, true) => '\u{2190}',
            (Dir::Up | Dir::Jump, _) => '\u{2191}',
            (Dir::Down | Dir::ChargeDown, _) => '\u{2193}',
            (Dir::UpForward, true) | (Dir::UpBack, false) => '\u{2197}',
            (Dir::UpForward, false) | (Dir::UpBack, true) => '\u{2196}',
            (Dir::DownForward, true) | (Dir::DownBack, false) => '\u{2198}',
            (Dir::DownForward, false) | (Dir::DownBack, true) => '\u{2199}',
        }
    }

    /// The direction's numpad digit, written facing right: 6 is forward,
    /// 4 back, 2 down, 8 up, the corners 1 3 7 9.
    pub const fn numpad(self) -> char {
        match self {
            Dir::Neutral => '5',
            Dir::Forward => '6',
            Dir::Back | Dir::ChargeBack => '4',
            Dir::Up | Dir::Jump => '8',
            Dir::Down | Dir::ChargeDown => '2',
            Dir::UpForward => '9',
            Dir::UpBack => '7',
            Dir::DownForward => '3',
            Dir::DownBack => '1',
        }
    }

    /// The direction a ROM pattern byte names (the facing-relative codes of
    /// `0x403be`: 2 forward, 3 back, 5 down, 6 down-forward, 7 down-back,
    /// 9 up, 0xa up-forward, 0xb up-back), or none for neutral and the
    /// impossible combinations.
    pub const fn from_pattern(code: u8) -> Option<Dir> {
        Some(match code {
            2 => Dir::Forward,
            3 => Dir::Back,
            5 => Dir::Down,
            6 => Dir::DownForward,
            7 => Dir::DownBack,
            9 => Dir::Up,
            0xa => Dir::UpForward,
            0xb => Dir::UpBack,
            _ => return None,
        })
    }

    /// The direction's initials (`DF`).
    pub const fn initials(self) -> &'static str {
        match self {
            Dir::Neutral => "N",
            Dir::Forward => "F",
            Dir::Back => "B",
            Dir::Up => "U",
            Dir::Jump => "JUMP",
            Dir::Down => "D",
            Dir::UpForward => "UF",
            Dir::UpBack => "UB",
            Dir::DownForward => "DF",
            Dir::DownBack => "DB",
            Dir::ChargeBack => "[B]",
            Dir::ChargeDown => "[D]",
        }
    }
}

/// One of the 1987 input handler's three command trackers
/// (`docs/reference/posture-dispatch.md`, "The input handler"): three
/// bytes in the player structure (stage, frame count, done), with the
/// pattern of directions it goes through in the ROM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tracker1987 {
    /// `+0x16`, pattern `0x4038e`: D, DB, B.
    Tatsumaki,
    /// `+0x19`, pattern `0x403a2`: F, D, DF.
    Shoryuken,
    /// `+0x1c`, pattern `0x403ba`: D, DF, F.
    Hadoken,
}

impl Tracker1987 {
    /// The tracker's offset in the player structure.
    pub const fn offset(self) -> u32 {
        match self {
            Tracker1987::Tatsumaki => 0x16,
            Tracker1987::Shoryuken => 0x19,
            Tracker1987::Hadoken => 0x1c,
        }
    }

    /// The ROM address of the tracker's pattern (three direction codes).
    pub const fn pattern_address(self) -> usize {
        match self {
            Tracker1987::Tatsumaki => 0x4038e,
            Tracker1987::Shoryuken => 0x403a2,
            Tracker1987::Hadoken => 0x403ba,
        }
    }
}

/// What a recognizer reports of an attempt, read and never written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecognizerState {
    /// How many steps of the motion are through (0 waiting for the first,
    /// 3 complete).
    pub stage: u8,
    /// Frames since the first direction; the tracker resets at `0x12`.
    pub frames: u8,
    /// The command is complete and still held.
    pub done: bool,
}

impl RecognizerState {
    /// The steps of the motion the engine has counted now. A tracker at
    /// stage 3 with its done byte clear has had its command consumed (the
    /// button branch clears the tracker when the move starts), so it has
    /// counted none: the attempt is over.
    pub const fn counted(&self) -> u8 {
        self.counted_for(3)
    }

    pub(crate) const fn counted_for(&self, length: u8) -> u8 {
        if self.stage == length && !self.done {
            0
        } else {
            self.stage
        }
    }
}

/// The frames a 1987 tracker's window lasts (`0x40b20`).
pub const TRACKER_WINDOW: u8 = 0x12;

/// The window a fight's trackers run: the ROM's `0x12` frames, or the
/// configured one under lenient command trackers.
pub fn tracker_window(toggles: &CoreToggles) -> u8 {
    if toggles.lenient_command_trackers {
        toggles.command_tracker_window
    } else {
        TRACKER_WINDOW
    }
}

/// The busy gate's first test (`0x403d4`): a fighter's state byte (entity
/// `+9`) & `0xc9` is 0, 1, 8 or 9, or its move (`+0xa`) is 0 or `0xc`. This
/// is the stricter reading Show uses with the stance ([`crate::front::show`]);
/// the gate the input handler really applies is [`input_gate_free`].
pub const fn busy_gate_free(state: u8, move_id: u8) -> bool {
    matches!(state & 0xc9, 0 | 1 | 8 | 9) || matches!(move_id, 0 | 0xc)
}

/// The machine as a recognizer view reads it: the program ROM, the work and
/// object RAM, and the extended data bank when the machine has one (a
/// boss's 1987 player table, ADR-0014), all shared, so a view cannot write
/// them (ADR-0004).
#[derive(Clone, Copy, Debug)]
pub struct MachineRead<'a> {
    pub rom: &'a [u8],
    pub work: &'a [u8],
    pub obj: &'a [u8],
    pub bank: Option<&'a crate::data_bank::DataBank>,
}

/// A read-only view of one recognizer for the fighter at a player structure
/// (ADR-0012): the attempt's progress, and the engine's own word on whether
/// the fighter could act and what move it is in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecognizerView {
    /// The recognizer's raw bytes.
    pub state: RecognizerState,
    /// Frames left in the window (0 when no attempt is open).
    pub time_left: u8,
    /// The direction the recognizer awaits next, facing-relative, from the
    /// ROM's pattern table; none once the motion is complete.
    pub awaiting: Option<Dir>,
    /// Every direction the engine takes for the next step under the fight's
    /// toggles: the ROM's one, or under lenient command trackers what
    /// `tracker_matches_stage` accepts (the Shoryuken's shortcuts).
    pub accepts: [Option<Dir>; 2],
    /// The input handler's busy gate frees the fighter to act on this frame
    /// ([`input_gate_free`]): it mirrors the handler, walks and toggles
    /// included.
    pub gate_free: bool,
    /// The fighter's current move (entity `+0xa`).
    pub current_move: u8,
    /// The frames the window lasts under the fight's toggles
    /// ([`tracker_window`]).
    pub window: u8,
    /// The input handler is holding a punch (`P1_STRUCT + 0`) or a kick
    /// (`+ 1`) it took from the pad and has not yet used: the engine's own
    /// word that it took the button (the overhaul on the press, 1987 on the
    /// release), seen while the busy gate holds the fighter. A button taken on
    /// a frame the gate is open starts its attack at once and leaves no latch.
    pub latched: [bool; 2],
}

/// What reports an entry's progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recognizer {
    /// One of the 1987 trackers, in the player structure.
    Tracker(Tracker1987),
    /// A separate native era command tracked outside 1987 work RAM.
    EraMotion { super_move: bool },
    /// The Alpha motion tracked during ground blockstun.
    AlphaCounter,
    /// A native Alpha command tracked for this move's slot.
    AlphaMove,
    /// An era-family command tracked outside native work RAM.
    FamilyMove,
}

impl Recognizer {
    /// The recognizer's state for the player structure at `player_struct`,
    /// from the machine's work RAM (`0xff8000..`).
    pub fn read(&self, work: &[u8], player_struct: u32) -> RecognizerState {
        match self {
            Recognizer::Tracker(t) => {
                let at = (player_struct + t.offset() - WORK_BASE) as usize;
                RecognizerState {
                    stage: work[at],
                    frames: work[at + 1],
                    done: work[at + 2] != 0,
                }
            }
            Recognizer::EraMotion { .. } | Recognizer::AlphaCounter | Recognizer::AlphaMove | Recognizer::FamilyMove => RecognizerState::default(),
        }
    }

    /// The recognizer's state for the first (P1) player structure.
    pub fn read_p1(&self, work: &[u8]) -> RecognizerState {
        self.read(work, P1_STRUCT)
    }

    /// The recognizer's view for the fighter at `player_struct` in a fight
    /// under `toggles` (they set the window, the directions taken and the
    /// busy gate). Nothing is written.
    pub fn view(&self, machine: MachineRead<'_>, player_struct: u32, toggles: &CoreToggles) -> RecognizerView {
        let state = self.read(machine.work, player_struct);
        let counted = state.counted();
        let Recognizer::Tracker(tracker) = self else {
            return recognizer_view(machine, player_struct, toggles, state, 0, [None; 2], 32);
        };
        let pattern = |stage: u8| {
            machine.rom.get(tracker.pattern_address() + usize::from(stage)).and_then(|&code| Dir::from_pattern(code))
        };
        // Stage 0 awaits the pattern's first direction, 1 and 2 the next.
        let awaiting = if counted < 3 { pattern(counted) } else { None };
        let accepts = if counted >= 3 {
            [None, None]
        } else if toggles.lenient_command_trackers && *tracker == Tracker1987::Shoryuken {
            // `tracker_matches_stage`: the Shoryuken's shortcuts.
            use Dir::*;
            match counted {
                0 => [Some(Forward), Some(DownForward)],
                1 => [Some(Down), Some(DownBack)],
                _ => {
                    // Forward only while the Hadoken tracker is not at stage 2 or more.
                    let hadoken = Tracker1987::Hadoken.offset();
                    let at = (player_struct + hadoken - WORK_BASE) as usize;
                    let free = machine.work.get(at).is_some_and(|&stage| stage < 2);
                    [Some(DownForward), free.then_some(Forward)]
                }
            }
        } else {
            [awaiting, None]
        };
        // The window runs from the first direction until the command is let go.
        let time_left = if counted > 0 { tracker_window(toggles).saturating_sub(state.frames) } else { 0 };
        let mut view = recognizer_view(machine, player_struct, toggles, state, time_left, accepts, tracker_window(toggles));
        view.awaiting = awaiting;
        view
    }
}

fn recognizer_view(
    machine: MachineRead<'_>,
    player_struct: u32,
    toggles: &CoreToggles,
    state: RecognizerState,
    time_left: u8,
    accepts: [Option<Dir>; 2],
    window: u8,
) -> RecognizerView {
    let entity = entity_of(machine.work, player_struct);
    RecognizerView {
        state,
        time_left,
        awaiting: accepts[0],
        accepts,
        gate_free: input_gate_free(machine, player_struct, toggles),
        current_move: machine.obj.get(entity + 0xa).copied().unwrap_or(0),
        window,
        latched: [machine.byte(player_struct) != 0, machine.byte(player_struct + 1) != 0],
    }
}

impl MachineRead<'_> {
    /// A byte at a 68000 address in the program ROM, the extended data
    /// bank, the work RAM or the object RAM (0 elsewhere).
    fn byte(&self, addr: u32) -> u8 {
        let addr = addr & 0xff_ffff;
        let (slice, at) = match addr {
            0..=0x05_ffff => (self.rom, addr),
            crate::data_bank::DATA_BANK_BASE..=crate::data_bank::DATA_BANK_END => return self.bank.map_or(0, |b| b.rb(addr)),
            0xff_8000..=0xff_dfff => (self.work, addr - WORK_BASE),
            0xff_e000..=0xff_ffff => (self.obj, addr - 0xff_e000),
            _ => return 0,
        };
        slice.get(at as usize).copied().unwrap_or(0)
    }

    fn long(&self, addr: u32) -> u32 {
        (0..4).fold(0, |acc, i| acc << 8 | u32::from(self.byte(addr + i)))
    }

    fn entity_byte(&self, entity: u16, at: u32) -> u8 {
        self.byte(0xff_e000 + u32::from(entity) + at)
    }
}

/// Whether the input handler (`0x402f8`) lets the fighter at `player_struct`
/// act on this frame, as it reads the gate (`0x403d4`): the state byte
/// (entity `+9`) & `0xc9` is 0, 1, 8 or 9, or the move is 0 or `0xc`; or the
/// state & `0xc9` is `0x80` (a walk's interruptible phase, `0x92`); or the
/// walk is interruptible under `interruptible_walk_cycle` (and under
/// `standard_high_low_block`, while holding back against an attack). A state
/// of exactly `0x80` is held unless the walk is interruptible
/// (`0x404b4`), and a fighter who is down (`+8` is 8) never gets there.
/// A bank player's busy jump stays held through its CPU falling phase.
/// Nothing is written.
pub fn input_gate_free(machine: MachineRead<'_>, player_struct: u32, toggles: &CoreToggles) -> bool {
    let both = machine.long(player_struct + 0xe);
    let (me, other) = (both as u16, (both >> 16) as u16);
    if machine.entity_byte(me, 8) == 8 {
        return false;
    }
    let (state, mv) = (machine.entity_byte(me, 9), machine.entity_byte(me, 0xa));
    if matches!(player_struct, P1_STRUCT | P2_STRUCT | CPU_STRUCT)
        && busy_banked_jump(state, mv, machine.long(player_struct + 0x12))
    {
        return false;
    }
    let d2 = state & 0xc9;
    let walking = state & 0x80 != 0 && matches!(mv, 2 | 3);
    let interruptible_walk = walking
        && (toggles.interruptible_walk_cycle || (toggles.standard_high_low_block && guard_interrupt(machine, player_struct, me, other)));
    let first = matches!(d2, 0 | 1 | 8 | 9) || matches!(mv, 0 | 0xc);
    if !first && d2 != 0x80 && !interruptible_walk {
        return false;
    }
    !(state == 0x80 && !interruptible_walk)
}

/// `is_guard_interrupt`: holding back (or down-back) while the opponent
/// attacks high (or low), under `standard_high_low_block`.
fn guard_interrupt(machine: MachineRead<'_>, player_struct: u32, me: u16, other: u16) -> bool {
    let a1 = machine.long(player_struct + 2);
    let d0 = !machine.byte(a1) & 0xf;
    let facing_right = (machine.entity_byte(me, 0x37) as i8) < 4;
    let (back, down_back) = if facing_right { (2, 6) } else { (1, 5) };
    let (st, mv) = (machine.entity_byte(other, 9), machine.entity_byte(other, 0xa));
    // A projectile of the opponent's in flight (`has_incoming_projectile`).
    let incoming = || {
        let owner = (0..4).fold(0u32, |acc, i| acc << 8 | u32::from(machine.entity_byte(other, 0x38 + i)));
        (0..12u16).any(|k| {
            let slot = 0x100 + 0x40 * k;
            let long = (0..4).fold(0u32, |acc, i| acc << 8 | u32::from(machine.entity_byte(slot, 0x20 + i)));
            machine.entity_byte(slot, 8) != 0 && machine.entity_byte(slot, 0xe) & 0x80 != 0 && long == owner
        })
    };
    let high = matches!(st, 0xc2 | 0xc1 | 0xc4) || incoming();
    // An airborne special guarded crouching: by its slot in the opponent's
    // table (the Shoryuken and Tatsumaki on Ryu's and Ken's).
    let low_in_air = || {
        let table = machine.long(machine.long(0xff_e000 + u32::from(other) + 0x38) + 0x12);
        crate::data_bank::SPECIAL_SLOTS.contains(&mv) && crate::data_bank::special_slot(|a| machine.byte(a), table, mv).low_in_air
    };
    let low = matches!(st, 0xc4 | 0xc2) || (st == 0xc1 && low_in_air()) || incoming();
    (d0 == back && high) || (d0 == down_back && low)
}

/// The presets and fighters an entry is available to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Availability {
    pub fighters: &'static [TournamentFighter],
    pub presets: &'static [RulesetPreset],
    /// The move is in a boss's 1987 player table in the extended data bank
    /// (ADR-0014), so a fight without his table (no art pack, or a pose
    /// missing) does not have it.
    pub player_table: bool,
}

impl Availability {
    pub fn includes(&self, fighter: TournamentFighter, preset: RulesetPreset) -> bool {
        self.fighters.contains(&fighter) && self.presets.contains(&preset)
    }
}

/// One move of the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveEntry {
    pub name: &'static str,
    pub kind: MoveKind,
    pub input: MoveInput,
    pub recognizer: Recognizer,
    /// The engine move the player's fighter is in once it comes out
    /// (entity `+0xa`).
    pub produces: u8,
    /// The word a success call-out shows (Try, ML5): the name and a bang.
    pub call_out: &'static str,
    pub available: Availability,
}

impl MoveEntry {
    pub(crate) fn live_recognizer(
        &self,
        machine: MachineRead<'_>,
        player_struct: u32,
        toggles: &CoreToggles,
        era: &crate::era_fight::EraFighterState,
    ) -> RecognizerView {
        let (stage, remaining) = match self.recognizer {
            Recognizer::EraMotion { super_move } => era.command_progress(super_move),
            Recognizer::AlphaCounter => era.alpha_counter_progress(),
            Recognizer::AlphaMove => era.alpha_move_progress(self.produces),
            Recognizer::FamilyMove => era.family_move_progress(self.produces),
            Recognizer::Tracker(_) => return self.recognizer.view(machine, player_struct, toggles),
        };
        let state = RecognizerState {
            stage,
            frames: if stage == 0 { 0 } else { 32u8.saturating_sub(remaining) },
            done: usize::from(stage) == self.input.motion.len(),
        };
        let next = self.input.motion.get(usize::from(stage)).copied();
        let mut view = recognizer_view(machine, player_struct, toggles, state, remaining, [next, None], 32);
        if self.recognizer == Recognizer::AlphaCounter {
            view.gate_free = stage != 0
                && matches!(view.current_move, 4 | 8)
                && era.super_meter >= crate::era::STOCK_AT;
        }
        view
    }
}

const SHOTOS: &[TournamentFighter] = &[TournamentFighter::Ryu, TournamentFighter::Ken];
const RETSU: &[TournamentFighter] = &[TournamentFighter::Retsu];
const GEKI: &[TournamentFighter] = &[TournamentFighter::Geki];
const JOE: &[TournamentFighter] = &[TournamentFighter::Joe];
const MIKE: &[TournamentFighter] = &[TournamentFighter::Mike];
const LEE: &[TournamentFighter] = &[TournamentFighter::Lee];
const GEN: &[TournamentFighter] = &[TournamentFighter::Gen];
const BIRDIE: &[TournamentFighter] = &[TournamentFighter::Birdie];
const EAGLE: &[TournamentFighter] = &[TournamentFighter::Eagle];
const ADON: &[TournamentFighter] = &[TournamentFighter::Adon];
const SAGAT: &[TournamentFighter] = &[TournamentFighter::Sagat];
/// The presets a boss plays in: every one but Faithful 1987, where only Ryu
/// and Ken can be picked.
const OVERHAUL_PRESETS: &[RulesetPreset] = &[
    RulesetPreset::DefinitiveOverhaul,
    RulesetPreset::ClassicCompetitive,
    RulesetPreset::PureFootsies1987Plus,
    RulesetPreset::Custom,
];
/// Every preset: the 1987 moves are in all of them (the toggles loosen
/// their trackers, never take a move away).
const ALL_PRESETS: &[RulesetPreset] = &RulesetPreset::ALL;

/// Every move the registry knows, in the order a list shows them.
pub const REGISTRY: &[MoveEntry] = &[
    MoveEntry {
        name: "HADOKEN",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "HADOKEN!",
        available: Availability { fighters: SHOTOS, presets: ALL_PRESETS, player_table: false },
    },
    MoveEntry {
        name: "SHORYUKEN",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Forward, Dir::Down, Dir::DownForward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Shoryuken),
        produces: 0x19,
        call_out: "SHORYUKEN!",
        available: Availability { fighters: SHOTOS, presets: ALL_PRESETS, player_table: false },
    },
    MoveEntry {
        name: "TATSUMAKI",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownBack, Dir::Back],
            button: ButtonClass::Kick,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Tatsumaki),
        produces: 0x1b,
        call_out: "TATSUMAKI!",
        available: Availability { fighters: SHOTOS, presets: ALL_PRESETS, player_table: false },
    },
    // Retsu's Kiai Shot (ticket #248): his 1987 player table puts it in the
    // Hadoken's move, so the Hadoken's tracker brings it out.
    MoveEntry {
        name: "KIAI SHOT",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "KIAI SHOT!",
        available: Availability { fighters: RETSU, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Geki's Shuriken and Teleport Strike (ticket #251): his 1987 player
    // table puts the Shuriken in the Hadoken's move on its command, and the
    // Teleport Strike in the Tatsumaki's move on the Tatsumaki's command
    // with a punch.
    MoveEntry {
        name: "SHURIKEN",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "SHURIKEN!",
        available: Availability { fighters: GEKI, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "TELEPORT STRIKE",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownBack, Dir::Back],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Tatsumaki),
        produces: 0x1b,
        call_out: "TELEPORT STRIKE!",
        available: Availability { fighters: GEKI, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Joe's Rolling Sobat and Dynamite Straight (ticket #252): his 1987
    // player table puts the Rolling Sobat in the Tatsumaki's move on its
    // command, and the Dynamite Straight in the Shoryuken's on its command.
    MoveEntry {
        name: "ROLLING SOBAT",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownBack, Dir::Back],
            button: ButtonClass::Kick,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Tatsumaki),
        produces: 0x1b,
        call_out: "ROLLING SOBAT!",
        available: Availability { fighters: JOE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "DYNAMITE STRAIGHT",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Forward, Dir::Down, Dir::DownForward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Shoryuken),
        produces: 0x19,
        call_out: "DYNAMITE STRAIGHT!",
        available: Availability { fighters: JOE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Mike's Bison Rush and Iron Upper (ticket #253): his 1987 player table
    // puts the Bison Rush in the Hadoken's move on its command, and the Iron
    // Upper in the Shoryuken's on its command.
    MoveEntry {
        name: "BISON RUSH",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "BISON RUSH!",
        available: Availability { fighters: MIKE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "IRON UPPER",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Forward, Dir::Down, Dir::DownForward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Shoryuken),
        produces: 0x19,
        call_out: "IRON UPPER!",
        available: Availability { fighters: MIKE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Lee's Zesshouohken and Double Kick (ticket #254): his 1987 player table
    // puts the Zesshouohken in the Hadoken's move on its command, and the
    // Double Kick in the Tatsumaki's on its command.
    MoveEntry {
        name: "ZESSHOUOHKEN",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "ZESSHOUOHKEN!",
        available: Availability { fighters: LEE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "DOUBLE KICK",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownBack, Dir::Back],
            button: ButtonClass::Kick,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Tatsumaki),
        produces: 0x1b,
        call_out: "DOUBLE KICK!",
        available: Availability { fighters: LEE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Gen's Hyakurenko and Gekiro (ticket #255): his 1987 player table puts
    // the Hyakurenko in the Hadoken's move on its command, and the Gekiro in
    // the Shoryuken's on its command.
    MoveEntry {
        name: "HYAKURENKO",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "HYAKURENKO!",
        available: Availability { fighters: GEN, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "GEKIRO",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Forward, Dir::Down, Dir::DownForward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Shoryuken),
        produces: 0x19,
        call_out: "GEKIRO!",
        available: Availability { fighters: GEN, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Birdie's Bull Head and Bull Horn (ticket #256): his 1987 player table
    // puts the Bull Head in the Hadoken's move on its command, and the Bull
    // Horn in the Tatsumaki's on its command.
    MoveEntry {
        name: "BULL HEAD",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "BULL HEAD!",
        available: Availability { fighters: BIRDIE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "BULL HORN",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownBack, Dir::Back],
            button: ButtonClass::Kick,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Tatsumaki),
        produces: 0x1b,
        call_out: "BULL HORN!",
        available: Availability { fighters: BIRDIE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Eagle's Manchester Black and St. Andrew's Green (ticket #257): his 1987
    // player table puts the Manchester Black in the Hadoken's move on its
    // command, and the St. Andrew's Green in the Tatsumaki's on its command.
    MoveEntry {
        name: "MANCHESTER BLACK",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "MANCHESTER BLACK!",
        available: Availability { fighters: EAGLE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "ST. ANDREW'S GREEN",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownBack, Dir::Back],
            button: ButtonClass::Kick,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Tatsumaki),
        produces: 0x1b,
        call_out: "ST. ANDREW'S GREEN!",
        available: Availability { fighters: EAGLE, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Adon's Rising Jaguar and Jaguar Kick (ticket #258): his 1987 player table
    // puts the Rising Jaguar in the Shoryuken's move on its command, and the
    // Jaguar Kick in the Tatsumaki's on its command.
    MoveEntry {
        name: "RISING JAGUAR",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Forward, Dir::Down, Dir::DownForward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Shoryuken),
        produces: 0x19,
        call_out: "RISING JAGUAR!",
        available: Availability { fighters: ADON, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "JAGUAR KICK",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownBack, Dir::Back],
            button: ButtonClass::Kick,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Tatsumaki),
        produces: 0x1b,
        call_out: "JAGUAR KICK!",
        available: Availability { fighters: ADON, presets: OVERHAUL_PRESETS, player_table: true },
    },
    // Sagat's High Tiger Shot, Low Tiger Shot and Tiger Knee (ticket #259):
    // his 1987 player table puts the High Tiger Shot in the Hadoken's move on
    // its command, the Low Tiger Shot in the Tatsumaki's move on quarter-circle
    // forward and kick, and the Tiger Knee in the Shoryuken's move on dragon
    // punch and kick.
    MoveEntry {
        name: "HIGH TIGER SHOT",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Punch,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1a,
        call_out: "HIGH TIGER SHOT!",
        available: Availability { fighters: SAGAT, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "LOW TIGER SHOT",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
            button: ButtonClass::Kick,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Hadoken),
        produces: 0x1b,
        call_out: "LOW TIGER SHOT!",
        available: Availability { fighters: SAGAT, presets: OVERHAUL_PRESETS, player_table: true },
    },
    MoveEntry {
        name: "TIGER KNEE",
        kind: MoveKind::Special,
        input: MoveInput {
            motion: &[Dir::Forward, Dir::Down, Dir::DownForward],
            button: ButtonClass::Kick,
        },
        recognizer: Recognizer::Tracker(Tracker1987::Shoryuken),
        produces: 0x19,
        call_out: "TIGER KNEE!",
        available: Availability { fighters: SAGAT, presets: OVERHAUL_PRESETS, player_table: true },
    },
];

/// The moves `fighter` has in a fight under `preset`: the registry's
/// entries available to both, in registry order.
pub fn move_list(fighter: TournamentFighter, preset: RulesetPreset) -> Vec<&'static MoveEntry> {
    REGISTRY.iter().filter(|e| e.available.includes(fighter, preset)).collect()
}

const HALF_CIRCLE_FORWARD: &[Dir] = &[
    Dir::Back, Dir::DownBack, Dir::Down, Dir::DownForward, Dir::Forward,
];
const TIGER_KNEE: &[Dir] = &[Dir::Down, Dir::DownForward, Dir::Forward, Dir::UpForward];
const UPPERCUT: &[Dir] = &[Dir::Forward, Dir::Down, Dir::DownForward];
const ALPHA_COUNTER_INPUT: &[Dir] = &[Dir::Back, Dir::DownBack, Dir::Down];
const ALPHA_FIGHTERS: &[TournamentFighter] = &[
    TournamentFighter::Ryu,
    TournamentFighter::Ken,
    TournamentFighter::Sagat,
    TournamentFighter::Birdie,
    TournamentFighter::Adon,
    TournamentFighter::Gen,
    TournamentFighter::Eagle,
];

pub(crate) fn live_move_list(
    fighter: TournamentFighter,
    preset: RulesetPreset,
    player_table: bool,
    era: &crate::era_fight::EraFighterState,
) -> Vec<MoveEntry> {
    use crate::data_bank::{ALPHA_COUNTER_SLOT, SF2_SPECIAL_SLOT, SF2_SUPER_SLOT};
    use crate::era::Revision;
    use crate::era_fight::DOUBLE_QUARTER_CIRCLE;
    use TournamentFighter::{Ken, Ryu, Sagat};

    let mut entries: Vec<_> = move_list(fighter, preset).into_iter()
        .filter(|entry| player_table || !entry.available.player_table).copied().collect();
    if era.fighter == Some(fighter) {
        entries.retain(|entry| !era.replaces_special(entry.produces));
        let family_moves = era.family_moves();
        entries.retain(|entry| !family_moves.iter().any(|other| {
            other.kind == MoveKind::Special && (entry.produces == other.produces || entry.name == other.name)
        }));
        entries.extend(family_moves.into_iter().filter(|entry| player_table || !entry.available.player_table));
        for entry in era.alpha_moves().filter(|_| fighter.is_shoto() || player_table) {
            entries.push(MoveEntry {
                name: entry.name,
                kind: entry.kind,
                input: entry.input,
                recognizer: Recognizer::AlphaMove,
                produces: entry.slot,
                call_out: entry.call_out,
                available: Availability {
                    fighters: ALPHA_FIGHTERS,
                    presets: OVERHAUL_PRESETS,
                    player_table: !fighter.is_shoto(),
                },
            });
        }
    }
    if era.fighter == Some(fighter) && era.custom_combo_available() && (fighter.is_shoto() || player_table) {
        entries.push(MoveEntry {
            name: "CUSTOM COMBO",
            kind: MoveKind::System,
            input: MoveInput { motion: &[], button: ButtonClass::SameStrength },
            recognizer: Recognizer::AlphaMove,
            produces: ALPHA_COUNTER_SLOT,
            call_out: "CUSTOM COMBO!",
            available: Availability {
                fighters: ALPHA_FIGHTERS,
                presets: OVERHAUL_PRESETS,
                player_table: !fighter.is_shoto(),
            },
        });
    }
    if era.fighter == Some(fighter) && era.alpha_counter_enabled() && (fighter.is_shoto() || player_table) {
        let available = Availability {
            fighters: ALPHA_FIGHTERS,
            presets: OVERHAUL_PRESETS,
            player_table: !fighter.is_shoto(),
        };
        for (name, button) in [
            ("ALPHA COUNTER (P)", ButtonClass::Punch),
            ("ALPHA COUNTER (K)", ButtonClass::Kick),
        ] {
            entries.push(MoveEntry {
                name,
                kind: MoveKind::System,
                input: MoveInput { motion: ALPHA_COUNTER_INPUT, button },
                recognizer: Recognizer::AlphaCounter,
                produces: ALPHA_COUNTER_SLOT,
                call_out: "ALPHA COUNTER!",
                available,
            });
        }
    }
    if !era.is_sf2() || era.fighter != Some(fighter) {
        return entries;
    }
    let fighters = match fighter {
        Ryu => &[Ryu][..],
        Ken => &[Ken][..],
        Sagat => SAGAT,
        _ => return entries,
    };
    let available = Availability { fighters, presets: OVERHAUL_PRESETS, player_table: fighter == Sagat };
    if fighter == Ryu && matches!(era.style.revision, Revision::Super | Revision::St) {
        entries.push(MoveEntry {
            name: "SHAKUNETSU HADOKEN",
            kind: MoveKind::Special,
            input: MoveInput { motion: HALF_CIRCLE_FORWARD, button: ButtonClass::Punch },
            recognizer: Recognizer::EraMotion { super_move: false },
            produces: SF2_SPECIAL_SLOT,
            call_out: "SHAKUNETSU!",
            available,
        });
    }
    if fighter == Sagat && player_table {
        if let Some(knee) = entries.iter_mut().find(|entry| entry.name == "TIGER KNEE") {
            knee.input.motion = TIGER_KNEE;
            knee.recognizer = Recognizer::EraMotion { super_move: false };
        }
        entries.push(MoveEntry {
            name: "TIGER UPPERCUT",
            kind: MoveKind::Special,
            input: MoveInput { motion: UPPERCUT, button: ButtonClass::Punch },
            recognizer: Recognizer::Tracker(Tracker1987::Shoryuken),
            produces: SF2_SPECIAL_SLOT,
            call_out: "TIGER UPPERCUT!",
            available,
        });
    }
    if era.style.revision == Revision::St && era.style.loadout == crate::era::Loadout::Standard
        && era.gauge().is_some() && (fighter != Sagat || player_table)
    {
        let (name, call_out, button) = match fighter {
            Ryu => ("SHINKU HADOKEN", "SHINKU HADOKEN!", ButtonClass::Punch),
            Ken => ("SHORYUREPPA", "SHORYUREPPA!", ButtonClass::Punch),
            Sagat => ("TIGER GENOCIDE", "TIGER GENOCIDE!", ButtonClass::Kick),
            _ => unreachable!(),
        };
        entries.push(MoveEntry {
            name,
            kind: MoveKind::Super,
            input: MoveInput { motion: DOUBLE_QUARTER_CIRCLE, button },
            recognizer: Recognizer::EraMotion { super_move: true },
            produces: SF2_SUPER_SLOT,
            call_out,
            available,
        });
    }
    entries
}

/// The six normals and the three postures, as the list's one compact line:
/// `LP MP HP LK MK HK · STAND CROUCH JUMP`. Every fighter has them.
pub const NORMALS_LINE: &str = "LP MP HP LK MK HK \u{b7} STAND CROUCH JUMP";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stick_held_gives_the_facing_relative_direction_it_was_built_from() {
        for dir in [Dir::Forward, Dir::Back, Dir::Up, Dir::Down, Dir::UpForward, Dir::UpBack, Dir::DownForward, Dir::DownBack] {
            for facing_right in [true, false] {
                assert_eq!(Dir::from_stick(dir.pressed(facing_right), facing_right), Some(dir), "{dir:?} facing right {facing_right}");
            }
        }
        assert_eq!(Dir::from_stick([false; 4], true), None);
        assert_eq!(Dir::from_stick([true, true, false, false], true), None, "left and right together cancel");
        assert_eq!(Dir::from_stick([true, true, true, false], true), Some(Dir::Down));
        // The same stick is forward facing right and back facing left.
        assert_eq!(Dir::from_stick([true, false, false, false], true), Some(Dir::Forward));
        assert_eq!(Dir::from_stick([true, false, false, false], false), Some(Dir::Back));
    }

    #[test]
    fn a_direction_is_written_in_each_notation() {
        let (d, f) = (Dir::DownForward, Dir::Forward);
        assert_eq!(d.written(InputNotation::Arrows, true), "\u{2198}");
        assert_eq!(d.written(InputNotation::Arrows, false), "\u{2199}");
        assert_eq!(d.written(InputNotation::Numpad, false), "3", "numpad is always written facing right");
        assert_eq!(d.written(InputNotation::Words, true), "DF");
        assert_eq!(f.written(InputNotation::Arrows, false), "\u{2190}");
        // The same as the input's own writing, a step at a time.
        let input = REGISTRY[0].input;
        let arrows: String = input.motion.iter().map(|d| d.written(InputNotation::Arrows, true)).collect();
        assert_eq!(input.write(InputNotation::Arrows, true)[0], InputPiece::Text(arrows));
    }

    #[test]
    fn a_button_class_is_down_when_one_of_its_buttons_is() {
        let mut buttons = [false; 6];
        assert!(!ButtonClass::Punch.down(buttons) && !ButtonClass::Kick.down(buttons));
        buttons[2] = true;
        assert!(ButtonClass::Punch.down(buttons) && !ButtonClass::Kick.down(buttons));
        buttons[5] = true;
        assert!(ButtonClass::Kick.down(buttons));
    }

    #[test]
    fn words_write_the_motion_and_the_button() {
        let words: Vec<_> = REGISTRY.iter().map(|e| e.input.words()).collect();
        assert_eq!(
            words,
            [
                "QCF+P", "DP+P", "QCB+K", "QCF+P", "QCF+P", "QCB+P", "QCB+K", "DP+P", "QCF+P", "DP+P", "QCF+P", "QCB+K", "QCF+P", "DP+P", "QCF+P",
                "QCB+K", "QCF+P", "QCB+K", "DP+P", "QCB+K", "QCF+P", "QCF+K", "DP+K"
            ]
        );
    }

    #[test]
    fn arrows_are_drawn_for_the_facing_and_numpad_is_always_right() {
        let text = |n, right| REGISTRY.iter().map(|e| input_text(&e.input.write(n, right))).collect::<Vec<_>>();
        assert_eq!(
            text(InputNotation::Arrows, true),
            [
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2192}\u{2193}\u{2198} [P]",
                "\u{2193}\u{2199}\u{2190} [K]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2193}\u{2199}\u{2190} [K]",
                "\u{2192}\u{2193}\u{2198} [P]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2192}\u{2193}\u{2198} [P]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2193}\u{2199}\u{2190} [K]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2192}\u{2193}\u{2198} [P]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2193}\u{2199}\u{2190} [K]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2193}\u{2199}\u{2190} [K]",
                "\u{2192}\u{2193}\u{2198} [P]",
                "\u{2193}\u{2199}\u{2190} [K]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2193}\u{2198}\u{2192} [K]",
                "\u{2192}\u{2193}\u{2198} [K]"
            ]
        );
        assert_eq!(
            text(InputNotation::Arrows, false),
            [
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2190}\u{2193}\u{2199} [P]",
                "\u{2193}\u{2198}\u{2192} [K]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2193}\u{2198}\u{2192} [P]",
                "\u{2193}\u{2198}\u{2192} [K]",
                "\u{2190}\u{2193}\u{2199} [P]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2190}\u{2193}\u{2199} [P]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2193}\u{2198}\u{2192} [K]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2190}\u{2193}\u{2199} [P]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2193}\u{2198}\u{2192} [K]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2193}\u{2198}\u{2192} [K]",
                "\u{2190}\u{2193}\u{2199} [P]",
                "\u{2193}\u{2198}\u{2192} [K]",
                "\u{2193}\u{2199}\u{2190} [P]",
                "\u{2193}\u{2199}\u{2190} [K]",
                "\u{2190}\u{2193}\u{2199} [K]"
            ]
        );
        for right in [true, false] {
            assert_eq!(
                text(InputNotation::Numpad, right),
                [
                    "236 P", "623 P", "214 K", "236 P", "236 P", "214 P", "214 K", "623 P", "236 P", "623 P", "236 P", "214 K", "236 P", "623 P",
                    "236 P", "214 K", "236 P", "214 K", "623 P", "214 K", "236 P", "236 K", "623 K"
                ]
            );
            assert_eq!(
                text(InputNotation::Words, right),
                [
                    "QCF+P", "DP+P", "QCB+K", "QCF+P", "QCF+P", "QCB+P", "QCB+K", "DP+P", "QCF+P", "DP+P", "QCF+P", "QCB+K", "QCF+P", "DP+P", "QCF+P",
                    "QCB+K", "QCF+P", "QCB+K", "DP+P", "QCB+K", "QCF+P", "QCF+K", "DP+K"
                ]
            );
        }
    }

    #[test]
    fn every_direction_has_an_arrow_each_way_and_a_numpad_digit() {
        use Dir::*;
        let all = [Forward, Back, Up, Down, UpForward, UpBack, DownForward, DownBack];
        for right in [true, false] {
            let arrows: std::collections::HashSet<_> = all.iter().map(|d| d.arrow(right)).collect();
            assert_eq!(arrows.len(), 8, "eight different arrows facing {right}");
            assert!(arrows.iter().all(|&a| sf_video::compositor::drawable(a)));
        }
        let digits: String = all.iter().map(|d| d.numpad()).collect();
        assert_eq!(digits, "64829731");
    }

    #[test]
    fn the_notation_cycles_and_round_trips_by_slug() {
        assert_eq!(InputNotation::default(), InputNotation::Arrows);
        assert_eq!(InputNotation::Words.next(), InputNotation::Arrows);
        assert_eq!(InputNotation::Arrows.prev(), InputNotation::Words);
        for n in InputNotation::ALL {
            assert_eq!(InputNotation::from_slug(n.slug()), Some(n));
        }
        assert_eq!(InputNotation::from_slug("kanji"), None);
    }

    #[test]
    fn the_facing_is_read_from_the_fighters_entity() {
        let mut work = vec![0u8; 0x6000];
        let mut obj = vec![0u8; 0x2000];
        // P1's entity is at 0x400 (`+0x10` of the structure).
        let at = (P1_STRUCT + 0x10 - WORK_BASE) as usize;
        work[at..at + 2].copy_from_slice(&0x400u16.to_be_bytes());
        obj[0x400 + 0x37] = 4;
        assert!(!facing_right(&work, &obj, P1_STRUCT));
        obj[0x400 + 0x37] = 2;
        assert!(facing_right(&work, &obj, P1_STRUCT));
    }

    #[test]
    fn an_unnamed_motion_is_written_by_its_directions() {
        let input = MoveInput { motion: &[Dir::Back, Dir::Forward], button: ButtonClass::Kick };
        assert_eq!(input.words(), "B,F+K");
    }

    #[test]
    fn the_fed_frames_hold_each_step_then_press_the_button_on_the_last() {
        let hadoken = &REGISTRY[0].input;
        let right = hadoken.frames(true, Strength::Light);
        assert_eq!(right.len(), 2 * STEP_FRAMES + HOLD_FRAMES);
        // D, then DF, then F (right, left, down, up).
        assert_eq!(right[0].directions, [false, false, true, false]);
        assert_eq!(right[STEP_FRAMES].directions, [true, false, true, false]);
        assert_eq!(right[2 * STEP_FRAMES].directions, [true, false, false, false]);
        let pressed: Vec<_> = right.iter().enumerate().filter(|(_, i)| i.buttons[0]).map(|(f, _)| f).collect();
        assert_eq!(pressed, (2 * STEP_FRAMES + 1..2 * STEP_FRAMES + 1 + BUTTON_FRAMES).collect::<Vec<_>>());
        // Facing left mirrors the horizontals; strength picks the button.
        let left = hadoken.frames(false, Strength::Heavy);
        assert_eq!(left[2 * STEP_FRAMES].directions, [false, true, false, false]);
        assert!(left[2 * STEP_FRAMES + 1].buttons[2]);
        // A kick move presses a kick.
        assert!(REGISTRY[2].input.frames(true, Strength::Medium).iter().any(|i| i.buttons[4]));
    }

    #[test]
    fn the_strip_lights_each_step_as_its_frames_are_fed() {
        let hadoken = &REGISTRY[0].input;
        assert_eq!(hadoken.strip_steps(), ["D", "DF", "F", "P"]);
        let lit: Vec<_> = (0..hadoken.frames(true, Strength::Light).len()).map(|f| hadoken.lit_at(f)).collect();
        // D from the first frame, DF and F a step later each, P with the press.
        assert_eq!(lit[0], 1);
        assert_eq!(lit[STEP_FRAMES], 2);
        assert_eq!(lit[2 * STEP_FRAMES], 3);
        assert_eq!(lit[2 * STEP_FRAMES + 1], 4);
        assert_eq!(*lit.last().unwrap(), 4);
        assert!(lit.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn the_list_is_the_registry_filtered_by_fighter_and_preset() {
        for preset in RulesetPreset::ALL {
            let names = |f| move_list(f, preset).iter().map(|e| e.name).collect::<Vec<_>>();
            assert_eq!(names(TournamentFighter::Ryu), ["HADOKEN", "SHORYUKEN", "TATSUMAKI"]);
            assert_eq!(names(TournamentFighter::Ken), ["HADOKEN", "SHORYUKEN", "TATSUMAKI"]);
            let retsu: &[&str] = if preset == RulesetPreset::Faithful1987 { &[] } else { &["KIAI SHOT"] };
            assert_eq!(names(TournamentFighter::Retsu), retsu);
            let geki: &[&str] = if preset == RulesetPreset::Faithful1987 { &[] } else { &["SHURIKEN", "TELEPORT STRIKE"] };
            assert_eq!(names(TournamentFighter::Geki), geki);
            let joe: &[&str] = if preset == RulesetPreset::Faithful1987 { &[] } else { &["ROLLING SOBAT", "DYNAMITE STRAIGHT"] };
            assert_eq!(names(TournamentFighter::Joe), joe);
            let mike: &[&str] = if preset == RulesetPreset::Faithful1987 { &[] } else { &["BISON RUSH", "IRON UPPER"] };
            assert_eq!(names(TournamentFighter::Mike), mike);
            let lee: &[&str] = if preset == RulesetPreset::Faithful1987 { &[] } else { &["ZESSHOUOHKEN", "DOUBLE KICK"] };
            assert_eq!(names(TournamentFighter::Lee), lee);
            let gen: &[&str] = if preset == RulesetPreset::Faithful1987 { &[] } else { &["HYAKURENKO", "GEKIRO"] };
            assert_eq!(names(TournamentFighter::Gen), gen);
            let birdie: &[&str] = if preset == RulesetPreset::Faithful1987 { &[] } else { &["BULL HEAD", "BULL HORN"] };
            assert_eq!(names(TournamentFighter::Birdie), birdie);
            let eagle: &[&str] = if preset == RulesetPreset::Faithful1987 {
                &[]
            } else {
                &["MANCHESTER BLACK", "ST. ANDREW'S GREEN"]
            };
            assert_eq!(names(TournamentFighter::Eagle), eagle);
            let adon: &[&str] = if preset == RulesetPreset::Faithful1987 { &[] } else { &["RISING JAGUAR", "JAGUAR KICK"] };
            assert_eq!(names(TournamentFighter::Adon), adon);
            let sagat: &[&str] = if preset == RulesetPreset::Faithful1987 {
                &[]
            } else {
                &["HIGH TIGER SHOT", "LOW TIGER SHOT", "TIGER KNEE"]
            };
            assert_eq!(names(TournamentFighter::Sagat), sagat);
        }
    }

    #[test]
    fn the_normals_line_is_drawable_in_the_rom_font() {
        assert!(NORMALS_LINE.chars().all(sf_video::compositor::drawable), "{NORMALS_LINE}");
    }

    /// A machine in miniature: the ROM's three pattern tables, a player
    /// structure with its entity pointer and the entity's bytes.
    fn mini(tracker: Tracker1987, bytes: [u8; 3], state: u8, mv: u8) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut rom = vec![0u8; 0x40500];
        for (t, pattern) in [
            (Tracker1987::Tatsumaki, [5, 7, 3]),
            (Tracker1987::Shoryuken, [2, 5, 6]),
            (Tracker1987::Hadoken, [5, 6, 2]),
        ] {
            rom[t.pattern_address()..t.pattern_address() + 3].copy_from_slice(&pattern);
        }
        let mut work = vec![0u8; 0x6000];
        let at = (P1_STRUCT + tracker.offset() - WORK_BASE) as usize;
        work[at..at + 3].copy_from_slice(&bytes);
        let ptr = (P1_STRUCT + 0x10 - WORK_BASE) as usize;
        work[ptr..ptr + 2].copy_from_slice(&0x400u16.to_be_bytes());
        let mut obj = vec![0u8; 0x2000];
        obj[0x400 + 9] = state;
        obj[0x400 + 0xa] = mv;
        (rom, work, obj)
    }

    fn view_of(tracker: Tracker1987, bytes: [u8; 3], state: u8, mv: u8) -> RecognizerView {
        let (rom, work, obj) = mini(tracker, bytes, state, mv);
        Recognizer::Tracker(tracker).view(MachineRead { rom: &rom, work: &work, obj: &obj, bank: None }, P1_STRUCT, &faithful())
    }

    fn faithful() -> CoreToggles {
        crate::config::MatchConfig::from_preset(RulesetPreset::Faithful1987).toggles
    }

    #[test]
    fn a_view_reports_the_stage_the_time_left_and_the_direction_awaited_from_the_roms_table() {
        use Tracker1987::*;
        // Idle: the first direction of each pattern is awaited, no window open.
        for (t, first) in [(Tatsumaki, Dir::Down), (Shoryuken, Dir::Forward), (Hadoken, Dir::Down)] {
            let v = view_of(t, [0, 0, 0], 0x14, 5);
            assert_eq!((v.state.stage, v.time_left, v.awaiting), (0, 0, Some(first)), "{t:?}");
        }
        // Hadoken (D, DF, F): stage 1 awaits DF and stage 2 awaits F, the window running down.
        let v = view_of(Hadoken, [1, 4, 0], 0x14, 5);
        assert_eq!((v.state.stage, v.time_left, v.awaiting), (1, 14, Some(Dir::DownForward)));
        let v = view_of(Hadoken, [2, 10, 0], 0x14, 6);
        assert_eq!((v.time_left, v.awaiting), (8, Some(Dir::Forward)));
        // Complete: the button is awaited, not a direction; the window still runs.
        let v = view_of(Hadoken, [3, 12, 0xff], 0x14, 6);
        assert_eq!((v.state.done, v.time_left, v.awaiting), (true, 6, None));
        // Shoryuken's and Tatsumaki's tables (F, D, DF / D, DB, B).
        assert_eq!(view_of(Shoryuken, [2, 0, 0], 0, 1).awaiting, Some(Dir::DownForward));
        assert_eq!(view_of(Tatsumaki, [1, 0, 0], 0, 1).awaiting, Some(Dir::DownBack));
        assert_eq!(view_of(Tatsumaki, [2, 0, 0], 0, 1).awaiting, Some(Dir::Back));
        // A consumed tracker (stage 3, done clear) has counted nothing.
        let v = view_of(Hadoken, [3, 0, 0], 0x91, 0x1a);
        assert_eq!((v.state.counted(), v.time_left, v.awaiting), (0, 0, Some(Dir::Down)));
        // A longer window (lenient trackers) leaves more time.
        let (rom, work, obj) = mini(Hadoken, [1, 4, 0], 0, 1);
        let toggles = crate::config::MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul).toggles;
        let lenient = Recognizer::Tracker(Hadoken).view(MachineRead { rom: &rom, work: &work, obj: &obj, bank: None }, P1_STRUCT, &toggles);
        assert_eq!(lenient.time_left, toggles.command_tracker_window - 4);
    }

    #[test]
    fn a_view_reports_the_busy_gate_and_the_current_move() {
        use Tracker1987::Hadoken;
        let v = view_of(Hadoken, [0, 0, 0], 0x14, 5);
        assert_eq!((v.gate_free, v.current_move), (true, 5));
        let v = view_of(Hadoken, [0, 0, 0], 0xc1, 0x15);
        assert_eq!((v.gate_free, v.current_move), (false, 0x15));
        // The gate as the input handler applies it (`0x403d4`, `0x404b4`):
        // the state bits or move 0xc open it; so does a walk's 0x92 phase
        // (& 0xc9 is 0x80) but not exactly 0x80; 0x91 holds it shut.
        for (state, mv, free) in [
            (0x00, 7, true),
            (0x01, 7, true),
            (0x08, 7, true),
            (0x09, 7, true),
            (0x92, 2, true),
            (0x92, 7, true),
            (0x80, 7, false),
            (0x91, 2, false),
            (0xc1, 0x1a, false),
            (0x91, 0xc, true),
            (0xc0, 0, true),
            (0x80, 0, false),
        ] {
            assert_eq!(view_of(Hadoken, [0, 0, 0], state, mv).gate_free, free, "{state:#x}/{mv:#x}");
        }
        // Show's stricter rule is only the first test of the gate.
        assert!(!busy_gate_free(0x92, 2));
    }

    #[test]
    fn the_interruptible_walk_opens_the_gate_on_a_gated_walk_frame() {
        let (rom, work, obj) = mini(Tracker1987::Hadoken, [0, 0, 0], 0x91, 2);
        let machine = MachineRead { rom: &rom, work: &work, obj: &obj, bank: None };
        let mut toggles = faithful();
        assert!(!input_gate_free(machine, P1_STRUCT, &toggles));
        toggles.interruptible_walk_cycle = true;
        assert!(input_gate_free(machine, P1_STRUCT, &toggles));
        // Only a walk (move 2 or 3) with the busy bit; a fighter who is down never acts.
        let (rom, work, mut obj) = mini(Tracker1987::Hadoken, [0, 0, 0], 0x91, 7);
        assert!(!input_gate_free(MachineRead { rom: &rom, work: &work, obj: &obj, bank: None }, P1_STRUCT, &toggles));
        obj[0x400 + 0xa] = 0;
        obj[0x400 + 8] = 8;
        assert!(!input_gate_free(MachineRead { rom: &rom, work: &work, obj: &obj, bank: None }, P1_STRUCT, &toggles));
    }

    #[test]
    fn lenient_trackers_take_the_directions_the_engine_takes() {
        use Tracker1987::*;
        let lenient = crate::config::MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul).toggles;
        let accepts = |t, bytes, toggles: &CoreToggles| {
            let (rom, work, obj) = mini(t, bytes, 0x14, 5);
            Recognizer::Tracker(t).view(MachineRead { rom: &rom, work: &work, obj: &obj, bank: None }, P1_STRUCT, toggles).accepts
        };
        assert_eq!(accepts(Shoryuken, [0, 0, 0], &faithful()), [Some(Dir::Forward), None]);
        assert_eq!(accepts(Shoryuken, [0, 0, 0], &lenient), [Some(Dir::Forward), Some(Dir::DownForward)]);
        assert_eq!(accepts(Shoryuken, [1, 2, 0], &lenient), [Some(Dir::Down), Some(Dir::DownBack)]);
        assert_eq!(accepts(Shoryuken, [2, 2, 0], &lenient), [Some(Dir::DownForward), Some(Dir::Forward)]);
        assert_eq!(accepts(Shoryuken, [3, 2, 0xff], &lenient), [None, None]);
        // The other two keep the ROM's table.
        assert_eq!(accepts(Hadoken, [1, 2, 0], &lenient), [Some(Dir::DownForward), None]);
        assert_eq!(accepts(Tatsumaki, [2, 2, 0], &lenient), [Some(Dir::Back), None]);
    }

    #[test]
    fn a_view_reads_the_machine_and_writes_nothing() {
        use Tracker1987::Shoryuken;
        let (rom, work, obj) = mini(Shoryuken, [2, 3, 0], 0x14, 5);
        let copy = (rom.clone(), work.clone(), obj.clone());
        for entry in REGISTRY {
            entry.recognizer.view(MachineRead { rom: &rom, work: &work, obj: &obj, bank: None }, P1_STRUCT, &faithful());
        }
        assert_eq!((rom, work, obj), copy);
        // Memory with nothing in it reads as a fighter in no move, not a panic.
        let v = Recognizer::Tracker(Shoryuken).view(MachineRead { rom: &[], work: &vec![0; 0x6000], obj: &[], bank: None }, P1_STRUCT, &faithful());
        assert_eq!((v.awaiting, v.current_move), (None, 0));
    }

    #[test]
    fn the_tracker_window_is_the_roms_unless_the_trackers_are_lenient() {
        use crate::config::MatchConfig;
        let faithful = MatchConfig::from_preset(RulesetPreset::Faithful1987);
        assert_eq!(tracker_window(&faithful.toggles), TRACKER_WINDOW);
        let lenient = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
        assert!(lenient.toggles.lenient_command_trackers);
        assert_eq!(tracker_window(&lenient.toggles), lenient.toggles.command_tracker_window);
    }

    #[test]
    fn the_pattern_codes_name_their_directions_and_match_every_entrys_motion() {
        let all = [
            (2, Dir::Forward),
            (3, Dir::Back),
            (5, Dir::Down),
            (6, Dir::DownForward),
            (7, Dir::DownBack),
            (9, Dir::Up),
            (0xa, Dir::UpForward),
            (0xb, Dir::UpBack),
        ];
        for (code, dir) in all {
            assert_eq!(Dir::from_pattern(code), Some(dir));
        }
        assert_eq!((Dir::from_pattern(1), Dir::from_pattern(0), Dir::from_pattern(0xc)), (None, None, None));
        let rom_patterns = [(Tracker1987::Hadoken, [5u8, 6, 2]), (Tracker1987::Shoryuken, [2, 5, 6]), (Tracker1987::Tatsumaki, [5, 7, 3])];
        for entry in REGISTRY {
            let Recognizer::Tracker(t) = entry.recognizer else { panic!("expected a 1987 tracker") };
            let (_, pattern) = rom_patterns.iter().find(|(r, _)| *r == t).unwrap();
            let motion: Vec<_> = pattern.iter().map(|&c| Dir::from_pattern(c).unwrap()).collect();
            assert_eq!(motion, entry.input.motion, "{}", entry.name);
        }
    }

    #[test]
    fn every_entry_calls_out_its_name_in_the_rom_font() {
        for entry in REGISTRY {
            assert_eq!(entry.call_out, format!("{}!", entry.name));
            assert!(entry.call_out.chars().all(|c| sf_video::compositor::small_glyph(c).is_some()), "{}", entry.call_out);
        }
    }

    #[test]
    fn a_tracker_reads_its_three_bytes() {
        let mut work = vec![0u8; 0x6000];
        let at = (P1_STRUCT + 0x19 - WORK_BASE) as usize;
        work[at..at + 3].copy_from_slice(&[2, 7, 0xff]);
        let state = Recognizer::Tracker(Tracker1987::Shoryuken).read_p1(&work);
        assert_eq!(state, RecognizerState { stage: 2, frames: 7, done: true });
        assert_eq!(Recognizer::Tracker(Tracker1987::Hadoken).read_p1(&work).stage, 0);
    }
}
