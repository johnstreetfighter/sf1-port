//! The era HUD's call-outs and screen regions (spec #216, `EH4`).
//!
//! A call-out is a word on the side of the fighter who earned a moment.
//! Moments arrive as one-frame [`CalloutEvent`]s; a [`CalloutBoard`] takes
//! each frame's events and keeps what is still showing, and its
//! [`CalloutFrame`] is what the compositor draws in the overhaul present
//! path. Faithful frames never come here.
//!
//! **Nothing overlaps.** Every element has a fixed screen region of its own
//! ([`regions`]): the health block with TIME, each side's gauge corner, each
//! side's call-out column and each side's super move-name line. Each column
//! has a top slot reserved for the combo count and [`WORD_SLOTS`] word slots
//! under it, filled in arrival order; a word that finds the column full
//! drops the oldest word, never the count.
//!
//! **Lettering.** The words are the ROM's 8×8 font (the type the 1987 round
//! calls, `FIGHT!` at `0x14f44`, are lettered in) in PLAYER1 gold. The ROM's
//! larger 2×2 header type cannot set the long words inside a half-screen
//! column; see `docs/planning/art-upgrades.md`. The combo count's number
//! is that header type's digits, in white, over `HITS` in the 8×8 gold.
//!
//! **Screen effects.** A super dims the stage one level, however many supers
//! are running; a burnt-out fighter is drawn grey, under the dim; an Alpha
//! Counter flashes its side's gauge. They change only the presented frame:
//! no input is ever held or dropped for them.

use std::collections::VecDeque;

use crate::compositor::{header_glyph, RomFont};
use crate::era_hud::{self, EraHud, GaugeRow, GaugeState};
use crate::render::{HEIGHT, WIDTH};

/// A side of the screen: the earner of a call-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    P1,
    P2,
}

impl Side {
    pub const BOTH: [Side; 2] = [Side::P1, Side::P2];

    pub const fn index(self) -> usize {
        match self {
            Side::P1 => 0,
            Side::P2 => 1,
        }
    }
}

/// The moment a call-out marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalloutKind {
    /// A super starts: SUPER, the move's `name` on the move-name line, and
    /// the stage dimmed.
    Super { name: &'static str },
    Counter,
    PunishCounter,
    /// SF6, a parry in the perfect window.
    PerfectParry,
    /// SF6. The grey itself follows the fighter's state
    /// ([`CalloutBoard::set_greyed`]), not this one-frame event.
    Burnout,
    /// SF5.
    CrushCounter,
    /// Alpha, with a gauge flash.
    AlphaCounter,
    /// Alpha. The gauge drains with the combo's timer (the era model's own
    /// [`GaugeState::Draining`]).
    CustomCombo,
    /// The round's first clean hit.
    FirstAttack,
    /// An attack on the first possible frame after blockstun or a get-up.
    Reversal,
    /// A round won without taking damage.
    Perfect,
    /// Training's Try (spec #226, ML5): the move the player was trying came
    /// out, named in `word` (`HADOKEN!`). It is the player's own, so the
    /// announcer says nothing for it, and [`CalloutKind::all`] leaves it out.
    Move { word: &'static str },
    /// Training's coach after a missed attempt (spec #226, ML6): the short
    /// fix (`TOO SLOW`, `MISSED ↘`), worded in the chosen notation. It is
    /// the player's own, so the announcer says nothing for it, and
    /// [`CalloutKind::all`] leaves it out.
    Fix { text: FixText },
}

/// The most bytes a [`FixText`] holds.
pub const FIX_BYTES: usize = 48;

/// A line of words held inline, so a [`CalloutKind`] stays `Copy`: the
/// coach's fix, whose last word depends on the chosen notation (an arrow,
/// a numpad digit or a direction's initials). Cut at a line's letters and
/// at a character boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixText {
    bytes: [u8; FIX_BYTES],
    len: u8,
}

impl FixText {
    pub fn new(text: &str) -> Self {
        let mut bytes = [0u8; FIX_BYTES];
        let mut len = 0;
        for ch in text.chars().take(regions::LINE_LETTERS) {
            let n = ch.len_utf8();
            if len + n > FIX_BYTES {
                break;
            }
            ch.encode_utf8(&mut bytes[len..len + n]);
            len += n;
        }
        FixText { bytes, len: len as u8 }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

impl CalloutKind {
    /// A fix worded `text`.
    pub fn fix(text: &str) -> Self {
        CalloutKind::Fix { text: FixText::new(text) }
    }

    /// One of every kind but [`CalloutKind::Move`], a super named `name`.
    pub fn all(name: &'static str) -> [CalloutKind; 11] {
        [
            CalloutKind::Super { name },
            CalloutKind::Counter,
            CalloutKind::PunishCounter,
            CalloutKind::PerfectParry,
            CalloutKind::Burnout,
            CalloutKind::CrushCounter,
            CalloutKind::AlphaCounter,
            CalloutKind::CustomCombo,
            CalloutKind::FirstAttack,
            CalloutKind::Reversal,
            CalloutKind::Perfect,
        ]
    }

    /// The word shown in the column.
    pub fn word(&self) -> &str {
        match self {
            CalloutKind::Super { .. } => "SUPER",
            CalloutKind::Counter => "COUNTER",
            CalloutKind::PunishCounter => "PUNISH COUNTER",
            CalloutKind::PerfectParry => "PERFECT PARRY",
            CalloutKind::Burnout => "BURNOUT",
            CalloutKind::CrushCounter => "CRUSH COUNTER",
            CalloutKind::AlphaCounter => "ALPHA COUNTER",
            CalloutKind::CustomCombo => "CUSTOM COMBO",
            CalloutKind::FirstAttack => "FIRST ATTACK",
            CalloutKind::Reversal => "REVERSAL",
            CalloutKind::Perfect => "PERFECT",
            CalloutKind::Move { word } => word,
            CalloutKind::Fix { text } => text.as_str(),
        }
    }
}

/// One frame's moment: who earned it and what it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalloutEvent {
    pub side: Side,
    pub kind: CalloutKind,
}

impl CalloutEvent {
    pub const fn new(side: Side, kind: CalloutKind) -> Self {
        CalloutEvent { side, kind }
    }
}

/// The combo count in a column's top slot: `hits`, and the combo's damage
/// where it is shown (Training). Real fights fill it from the fight reader
/// (`sf_engine::fight_reader`, `EH5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboCount {
    pub hits: u16,
    pub damage: Option<u16>,
}

/// Word slots under each column's combo slot.
pub const WORD_SLOTS: usize = 4;
/// Frames a word stays up.
pub const WORD_FRAMES: u16 = 90;
/// Frames a super dims the stage; the move's name stays up as long as its word.
pub const SUPER_DIM_FRAMES: u16 = 45;
/// Frames an Alpha Counter flashes its side's gauge, lit and unlit in turn
/// every [`FLASH_PHASE`] frames.
pub const FLASH_FRAMES: u16 = 32;
pub const FLASH_PHASE: u16 = 4;

/// What one side shows this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SideCallouts {
    /// The top slot.
    pub combo: Option<ComboCount>,
    /// The word slots top down, oldest first; the rest `None`.
    pub words: [Option<CalloutKind>; WORD_SLOTS],
    /// The running super's name, on the move-name line.
    pub move_name: Option<&'static str>,
    /// The fighter and their gauge are grey (SF6 Burnout).
    pub greyed: bool,
    /// The gauge's flash and this frame's phase (an Alpha Counter).
    pub flash: Option<bool>,
}

impl SideCallouts {
    pub const NONE: SideCallouts =
        SideCallouts { combo: None, words: [None; WORD_SLOTS], move_name: None, greyed: false, flash: None };
}

/// What the call-out layer shows this frame, as the compositor draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CalloutFrame {
    pub sides: [SideCallouts; 2],
    /// The super dim: one level, on while any super runs.
    pub dim: bool,
    /// Pixels the call-out columns sit lower than their usual place: 0 in
    /// every fight but a Training Try, which needs the room under the health
    /// block for the strip's second line ([`regions::TRY_DROP`], ML6).
    pub drop: usize,
}

impl CalloutFrame {
    pub const NONE: CalloutFrame = CalloutFrame { sides: [SideCallouts::NONE; 2], dim: false, drop: 0 };

    pub fn side(&self, side: Side) -> &SideCallouts {
        &self.sides[side.index()]
    }

    /// Nothing to draw and no effect (where the columns would sit does not matter).
    pub fn is_empty(&self) -> bool {
        CalloutFrame { drop: 0, ..*self } == CalloutFrame::NONE
    }

    /// `hud` with this frame's gauge effects: a flashing side's main row
    /// flashes (a greyed row stays grey).
    pub fn gauges(&self, hud: EraHud) -> EraHud {
        let flash = |gauge: Option<era_hud::EraGauge>, side: &SideCallouts| {
            gauge.map(|mut gauge| {
                if let Some(lit) = side.flash {
                    if !matches!(gauge.main, GaugeRow::Stock { state: GaugeState::Greyed, .. }
                        | GaugeRow::Level { state: GaugeState::Greyed, .. })
                    {
                        gauge.main = gauge.main.with_state(GaugeState::Flashing { lit });
                    }
                }
                gauge
            })
        };
        EraHud { p1: flash(hud.p1, &self.sides[0]), p2: flash(hud.p2, &self.sides[1]) }
    }

    /// Every element this frame draws, with its box.
    pub fn elements(&self) -> Vec<Element> {
        let mut out = Vec::new();
        for side in Side::BOTH {
            let shown = self.side(side);
            if shown.combo.is_some() {
                out.push(Element { side, part: Part::Combo, rect: regions::combo_slot_at(side, self.drop) });
            }
            for (slot, word) in shown.words.iter().enumerate() {
                if let Some(word) = word {
                    let rect = regions::text_box(side, regions::word_slot_at(side, slot, self.drop).y, word.word());
                    out.push(Element { side, part: Part::Word(slot), rect });
                }
            }
            if let Some(name) = shown.move_name {
                let rect = regions::text_box(side, regions::move_name_line_at(side, self.drop).y, name);
                out.push(Element { side, part: Part::MoveName, rect });
            }
        }
        out
    }
}

/// A drawn call-out element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    pub side: Side,
    pub part: Part,
    pub rect: regions::Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The combo count's reserved slot.
    Combo,
    /// A word in word slot `n` (0 is the top one).
    Word(usize),
    MoveName,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Column {
    combo: Option<ComboCount>,
    /// Oldest first, with the frames each has left.
    words: VecDeque<(CalloutKind, u16)>,
    name: Option<(&'static str, u16)>,
    dim_left: u16,
    flash_left: u16,
    greyed: bool,
}

/// The call-outs showing on both sides, stepped once a frame. Deterministic:
/// the same events on the same frames show the same call-outs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalloutBoard {
    columns: [Column; 2],
    drop: usize,
}

impl CalloutBoard {
    pub fn new() -> Self {
        Self::default()
    }

    /// One frame: what was showing ages a frame and what has run its time
    /// goes, then this frame's `events` arrive in order.
    pub fn step(&mut self, events: &[CalloutEvent]) {
        for column in &mut self.columns {
            for (_, left) in column.words.iter_mut() {
                *left -= 1;
            }
            column.words.retain(|(_, left)| *left > 0);
            column.name = column.name.and_then(|(name, left)| (left > 1).then_some((name, left - 1)));
            column.dim_left = column.dim_left.saturating_sub(1);
            column.flash_left = column.flash_left.saturating_sub(1);
        }
        for event in events {
            let column = &mut self.columns[event.side.index()];
            if column.words.len() == WORD_SLOTS {
                column.words.pop_front();
            }
            column.words.push_back((event.kind, WORD_FRAMES));
            match event.kind {
                CalloutKind::Super { name } => {
                    column.name = Some((name, WORD_FRAMES));
                    column.dim_left = SUPER_DIM_FRAMES;
                }
                CalloutKind::AlphaCounter => column.flash_left = FLASH_FRAMES,
                _ => {}
            }
        }
    }

    /// The combo count in `side`'s top slot, or none (`EH5` sets it each frame).
    pub fn set_combo(&mut self, side: Side, combo: Option<ComboCount>) {
        self.columns[side.index()].combo = combo;
    }

    /// Whether `side`'s fighter is drawn grey (SF6 Burnout, for as long as it lasts).
    pub fn set_greyed(&mut self, side: Side, greyed: bool) {
        self.columns[side.index()].greyed = greyed;
    }

    /// Sets the columns lower by `drop` pixels ([`regions::TRY_DROP`] in a
    /// Training Try, 0 otherwise).
    pub fn set_drop(&mut self, drop: usize) {
        self.drop = drop;
    }

    /// What to draw this frame.
    pub fn frame(&self) -> CalloutFrame {
        let mut out = CalloutFrame { drop: self.drop, ..CalloutFrame::NONE };
        for side in Side::BOTH {
            let column = &self.columns[side.index()];
            let shown = &mut out.sides[side.index()];
            shown.combo = column.combo;
            for (slot, (kind, _)) in column.words.iter().enumerate() {
                shown.words[slot] = Some(*kind);
            }
            shown.move_name = column.name.map(|(name, _)| name);
            shown.greyed = column.greyed;
            shown.flash =
                (column.flash_left > 0).then(|| ((FLASH_FRAMES - column.flash_left) / FLASH_PHASE).is_multiple_of(2));
            out.dim = out.dim || column.dim_left > 0;
        }
        out
    }
}

/// The fixed screen regions of the era HUD. None intersects another.
pub mod regions {
    use super::{Side, WORD_SLOTS};
    use crate::era_hud::{CORNER_BOTTOM, CORNER_EDGE, CORNER_HEIGHT, CORNER_WIDTH};
    use crate::render::{HEIGHT, WIDTH};

    /// A screen box: `w` × `h` pixels from (`x`, `y`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Rect {
        pub x: usize,
        pub y: usize,
        pub w: usize,
        pub h: usize,
    }

    impl Rect {
        pub const fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
            Rect { x, y, w, h }
        }

        pub fn intersects(&self, other: &Rect) -> bool {
            self.x < other.x + other.w
                && other.x < self.x + self.w
                && self.y < other.y + other.h
                && other.y < self.y + self.h
        }

        pub fn contains(&self, x: usize, y: usize) -> bool {
            (self.x..self.x + self.w).contains(&x) && (self.y..self.y + self.h).contains(&y)
        }

        pub fn holds(&self, other: &Rect) -> bool {
            other.x >= self.x
                && other.y >= self.y
                && other.x + other.w <= self.x + self.w
                && other.y + other.h <= self.y + self.h
        }
    }

    /// The health block: the scores, the PLAYER1 / PLAYER2 bars and TIME
    /// under them (`x = 164..228, y = 52..60`), as the whole band down to TIME.
    pub const HEALTH_BLOCK: Rect = Rect::new(0, 0, WIDTH, 60);
    /// TIME as the overhaul layout draws it, under the health bars: the
    /// part of the health block the era HUD adds.
    pub const TIME: Rect = Rect::new(164, 52, 64, 8);
    /// The call-out columns' top, under the health block.
    pub const COLUMN_TOP: usize = 72;
    /// Pixels the columns sit lower during a Training Try (ML6), so the strip
    /// can have a second line, the player's own inputs after a miss, without
    /// meeting them. Every other fight keeps [`COLUMN_TOP`].
    pub const TRY_DROP: usize = 10;
    /// A column's width: a side's half less the screen edge and an 8-pixel
    /// gutter at the centre; 22 letters of the 8×8 font.
    pub const COLUMN_WIDTH: usize = WIDTH / 2 - CORNER_EDGE - 8;
    /// The combo count's slot: room for a 16-pixel number, HITS and a damage line.
    pub const COMBO_SLOT_HEIGHT: usize = 32;
    /// A word slot is one 8×8 line; slots are this far apart.
    pub const WORD_PITCH: usize = 10;
    const SLOT_GAP: usize = 2;
    /// The column: the combo slot and the word slots.
    pub const COLUMN_HEIGHT: usize = COMBO_SLOT_HEIGHT + SLOT_GAP + WORD_SLOTS * WORD_PITCH - SLOT_GAP;
    const NAME_GAP: usize = 4;

    /// The region kinds, for the no-overlap checks.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Region {
        HealthBlock,
        GaugeCorner(Side),
        CalloutColumn(Side),
        MoveName(Side),
        /// The Show strip under the health block (spec #226, ML4).
        Strip,
        /// The line under the strip that holds the player's last inputs after
        /// a miss (ML6).
        StripInputs,
    }

    /// A box `w` wide, `CORNER_EDGE` from `side`'s edge of the screen.
    fn from_edge(side: Side, y: usize, w: usize, h: usize) -> Rect {
        match side {
            Side::P1 => Rect::new(CORNER_EDGE, y, w, h),
            Side::P2 => Rect::new(WIDTH - CORNER_EDGE - w, y, w, h),
        }
    }

    /// `side`'s gauge corner: the most an era gauge draws.
    pub fn gauge_corner(side: Side) -> Rect {
        gauge_corner_lifted(side, 0)
    }

    /// [`gauge_corner`] with the corner `lift` rows higher (see
    /// [`crate::era_hud::draw_lifted`]).
    pub fn gauge_corner_lifted(side: Side, lift: usize) -> Rect {
        from_edge(side, HEIGHT - CORNER_BOTTOM - lift - CORNER_HEIGHT, CORNER_WIDTH, CORNER_HEIGHT)
    }

    /// `side`'s call-out column, at mid-height in its own half.
    pub fn callout_column(side: Side) -> Rect {
        callout_column_at(side, 0)
    }

    /// [`callout_column`] `drop` pixels lower ([`TRY_DROP`] in a Try).
    pub fn callout_column_at(side: Side, drop: usize) -> Rect {
        from_edge(side, COLUMN_TOP + drop, COLUMN_WIDTH, COLUMN_HEIGHT)
    }

    /// The column's top slot, reserved for the combo count.
    pub fn combo_slot(side: Side) -> Rect {
        combo_slot_at(side, 0)
    }

    pub fn combo_slot_at(side: Side, drop: usize) -> Rect {
        from_edge(side, COLUMN_TOP + drop, COLUMN_WIDTH, COMBO_SLOT_HEIGHT)
    }

    /// Word slot `slot` (0 is the top one, under the combo slot).
    pub fn word_slot(side: Side, slot: usize) -> Rect {
        word_slot_at(side, slot, 0)
    }

    pub fn word_slot_at(side: Side, slot: usize, drop: usize) -> Rect {
        from_edge(side, COLUMN_TOP + drop + COMBO_SLOT_HEIGHT + SLOT_GAP + slot * WORD_PITCH, COLUMN_WIDTH, 8)
    }

    /// The line under `side`'s column that holds its super's name.
    pub fn move_name_line(side: Side) -> Rect {
        move_name_line_at(side, 0)
    }

    pub fn move_name_line_at(side: Side, drop: usize) -> Rect {
        from_edge(side, COLUMN_TOP + drop + COLUMN_HEIGHT + NAME_GAP, COLUMN_WIDTH, 8)
    }

    /// The letters that fit on the strip.
    pub const STRIP_LETTERS: usize = 24;
    /// The Show strip (spec #226, ML4): one 8×8 line centred under the
    /// health block, in the gap above the call-out columns, so it meets
    /// neither the block with TIME nor a column.
    pub const STRIP: Rect = Rect::new((WIDTH - 8 * STRIP_LETTERS) / 2, 62, 8 * STRIP_LETTERS, 8);
    /// The player's last inputs after a miss (spec #226, ML6): the line under
    /// the strip, a pixel below it, above the call-out columns of a Try
    /// ([`TRY_DROP`] lower than in other fights, where this line is not drawn).
    pub const STRIP_INPUTS: Rect = Rect::new(STRIP.x, STRIP.y + 9, 8 * STRIP_LETTERS, 8);

    /// The letters that fit on a column line.
    pub const LINE_LETTERS: usize = COLUMN_WIDTH / 8;

    /// The box `text` takes on a line at `y`, set from `side`'s edge (P1's
    /// left-aligned, P2's right-aligned) and cut at [`LINE_LETTERS`].
    pub fn text_box(side: Side, y: usize, text: &str) -> Rect {
        from_edge(side, y, 8 * text.chars().count().min(LINE_LETTERS), 8)
    }

    /// Every fixed region of an ordinary fight.
    pub fn all() -> Vec<(Region, Rect)> {
        all_lifted(0)
    }

    /// Every fixed region of an ordinary fight with the gauge corners `lift`
    /// rows higher.
    pub fn all_lifted(lift: usize) -> Vec<(Region, Rect)> {
        all_with(lift, 0)
    }

    /// Every fixed region of a Training Try: the columns [`TRY_DROP`] lower,
    /// and the strip's second line.
    pub fn all_try() -> Vec<(Region, Rect)> {
        all_with(0, TRY_DROP)
    }

    /// Every fixed region with the gauge corners `lift` rows higher and the
    /// call-out columns `drop` pixels lower; the strip's second line is one
    /// of them only when the columns have made room for it (`drop` over 0).
    pub fn all_with(lift: usize, drop: usize) -> Vec<(Region, Rect)> {
        let mut out = vec![(Region::HealthBlock, HEALTH_BLOCK), (Region::Strip, STRIP)];
        if drop > 0 {
            out.push((Region::StripInputs, STRIP_INPUTS));
        }
        for side in Side::BOTH {
            out.push((Region::GaugeCorner(side), gauge_corner_lifted(side, lift)));
            out.push((Region::CalloutColumn(side), callout_column_at(side, drop)));
            out.push((Region::MoveName(side), move_name_line_at(side, drop)));
        }
        out
    }
}

/// The call-out lettering's pens: PLAYER1 gold, text colour 4's highlight,
/// shadow and body, as [`era_hud::pens::DIGIT_GOLD`].
pub const LETTER_PENS: [u32; 3] = era_hud::pens::DIGIT_GOLD;
/// The combo count's number pens (highlight, shadow, body): the ROM's
/// whitest text pen (colour 10's highlight, `#eeeeee`) for the body and the
/// highlight, over the health bar's `#444444` shadow. No boot text colour
/// is white with a dark shadow.
pub const NUMBER_PENS: [u32; 3] = [era_hud::pens::GREY_LIGHT, era_hud::pens::SHADOW, era_hud::pens::GREY_LIGHT];
/// The combo slot's lines, as offsets from its top: the count in the 2×2
/// header digits (16 pixels), `HITS` under it and the damage line under that.
pub const COMBO_HITS_LINE: usize = 16;
pub const COMBO_DAMAGE_LINE: usize = 24;

/// Draws `frame`'s combo counts, words and move names in the ROM's type.
/// Without the font nothing is lettered. The effects (dim, grey, flash) are
/// the compositor's.
pub fn draw(argb: &mut [u32], frame: &CalloutFrame, font: Option<&RomFont<'_>>) {
    let Some(font) = font else { return };
    for element in frame.elements() {
        let shown = frame.side(element.side);
        let text = match element.part {
            Part::Combo => {
                if let Some(combo) = shown.combo {
                    draw_combo(argb, element.side, combo, frame.drop, font);
                }
                continue;
            }
            Part::Word(slot) => shown.words[slot].as_ref().map(CalloutKind::word).unwrap_or_default(),
            Part::MoveName => shown.move_name.unwrap_or_default(),
        };
        letter(argb, element.rect, text, font);
    }
}

/// `text` in the 8×8 font in PLAYER1 gold from `rect`'s top left, cut at a line.
fn letter(argb: &mut [u32], rect: regions::Rect, text: &str, font: &RomFont<'_>) {
    for (i, ch) in text.chars().take(regions::LINE_LETTERS).enumerate() {
        if ch != ' ' {
            font.glyph_in(argb, rect.x + 8 * i, rect.y, ch.to_ascii_uppercase(), LETTER_PENS);
        }
    }
}

/// What the Show strip says (spec #226, ML4): `WAIT` while the fighter is
/// not free, then the input being fed, one label to a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Strip {
    Wait,
    /// The steps' labels in order, and how many are lit (fed so far).
    Input { steps: Vec<String>, lit: usize },
}

/// The strip's unlit steps: grey, under the lit steps' gold.
pub const STRIP_UNLIT_PENS: [u32; 3] = [era_hud::pens::GREY, era_hud::pens::SHADOW, era_hud::pens::GREY];

/// Draws `strip` in the ROM's type inside [`regions::STRIP`], centred, a
/// space between steps; a lit step in gold, an unlit one in grey, and a
/// line longer than the strip cut at its end. Without the font nothing is
/// lettered.
pub fn draw_strip(argb: &mut [u32], strip: &Strip, font: Option<&RomFont<'_>>) {
    let Some(font) = font else { return };
    let mut cells: Vec<(char, [u32; 3])> = Vec::new();
    match strip {
        Strip::Wait => cells.extend("WAIT".chars().map(|c| (c, LETTER_PENS))),
        Strip::Input { steps, lit } => {
            for (i, step) in steps.iter().enumerate() {
                if i > 0 {
                    cells.push((' ', LETTER_PENS));
                }
                let pens = if i < *lit { LETTER_PENS } else { STRIP_UNLIT_PENS };
                cells.extend(step.chars().map(|c| (c, pens)));
            }
        }
    }
    cells.truncate(regions::STRIP_LETTERS);
    let x = regions::STRIP.x + 8 * (regions::STRIP_LETTERS - cells.len()) / 2;
    for (i, (ch, pens)) in cells.into_iter().enumerate() {
        if ch != ' ' {
            font.glyph_in(argb, x + 8 * i, regions::STRIP.y, ch.to_ascii_uppercase(), pens);
        }
    }
}

/// The pens of the player's own inputs under the strip: the whitest pen the
/// ROM has, as the combo count's number, so they read apart from the gold
/// of the input to do and the grey of its unlit steps.
pub const STRIP_INPUTS_PENS: [u32; 3] = NUMBER_PENS;

/// Draws the player's last inputs (one label to a step, as the strip's) on
/// the line under the strip, centred, a space between labels; the labels
/// that do not fit are the oldest and are left out. Without the font
/// nothing is lettered.
pub fn draw_strip_inputs(argb: &mut [u32], labels: &[String], font: Option<&RomFont<'_>>) {
    let Some(font) = font else { return };
    let mut first = labels.len();
    let mut width = 0;
    while first > 0 {
        let next = width + labels[first - 1].chars().count() + usize::from(width > 0);
        if next > regions::STRIP_LETTERS {
            break;
        }
        (width, first) = (next, first - 1);
    }
    let mut x = regions::STRIP_INPUTS.x + 8 * (regions::STRIP_LETTERS - width) / 2;
    for label in &labels[first..] {
        for ch in label.chars() {
            font.glyph_in(argb, x, regions::STRIP_INPUTS.y, ch.to_ascii_uppercase(), STRIP_INPUTS_PENS);
            x += 8;
        }
        x += 8;
    }
}

/// The text of the combo's damage line.
pub fn damage_line(damage: u16) -> String {
    format!("DAMAGE {damage}")
}

/// The count in `side`'s combo slot: a large white number in the ROM's 2×2
/// header digits (`0x3bc3e`, each four 8×8 tiles), `HITS` in gold under it
/// and, where it is shown, the damage line under that, each set from the
/// side's screen edge. Everything stays inside the slot.
fn draw_combo(argb: &mut [u32], side: Side, combo: ComboCount, drop: usize, font: &RomFont<'_>) {
    let slot = regions::combo_slot_at(side, drop);
    let digits = combo.hits.to_string();
    let width = 16 * digits.len();
    let x = match side {
        Side::P1 => slot.x,
        Side::P2 => slot.x + slot.w - width,
    };
    for (i, ch) in digits.chars().enumerate() {
        let Some(code) = header_glyph(ch) else { continue };
        let (cx, y) = (x + 16 * i, slot.y);
        font.tile_in(argb, cx, y, code, NUMBER_PENS);
        font.tile_in(argb, cx + 8, y, code + 1, NUMBER_PENS);
        font.tile_in(argb, cx, y + 8, code + 0x10, NUMBER_PENS);
        font.tile_in(argb, cx + 8, y + 8, code + 0x11, NUMBER_PENS);
    }
    letter(argb, regions::text_box(side, slot.y + COMBO_HITS_LINE, "HITS"), "HITS", font);
    if let Some(damage) = combo.damage {
        let line = damage_line(damage);
        letter(argb, regions::text_box(side, slot.y + COMBO_DAMAGE_LINE, &line), &line, font);
    }
}

/// The super dim: one level (half brightness) over the pixels `stage` marks,
/// or over the whole frame when there is no mask.
pub fn dim(argb: &mut [u32], stage: Option<&[bool]>) {
    for (i, p) in argb.iter_mut().enumerate().take(WIDTH * HEIGHT) {
        if stage.is_none_or(|stage| stage.get(i).copied().unwrap_or(false)) {
            *p = 0xff00_0000 | ((*p >> 1) & 0x007f_7f7f);
        }
    }
}

/// `color` as a grey of its own brightness (a burnt-out fighter's palette).
pub fn grey(color: u32) -> u32 {
    let (r, g, b) = ((color >> 16) & 0xff, (color >> 8) & 0xff, color & 0xff);
    let y = (r * 30 + g * 59 + b * 11) / 100;
    (color & 0xff00_0000) | (y << 16) | (y << 8) | y
}
