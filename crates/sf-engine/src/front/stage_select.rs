//! The stage screen of Local and Training under the overhaul presets (spec
//! #192, tickets #193, #196 and #197): the stages the background bank
//! lists and a `RANDOM` tile, shown after the fighters lock, whose pick
//! reaches the fight as the roster's stage. One player is at the controls
//! (the picker: Player 1 first, then the loser of the last match), and the
//! cursor starts on the opponent's home stage or, on a rematch, the stage
//! just played.
//!
//! The look is the approved layout A: the front end's wide gold frame with
//! `STAGE SELECT` in the header, who is picking under it in the picker's
//! colour, the cursor's stage large with its owner and place under it, a
//! grid of small preview tiles eight to a row (each with its owner's name,
//! the `RANDOM` tile last), and nothing at the foot. The grid grows
//! a row at a time and the large preview takes what it leaves, so the ten
//! 1987 stages and `RANDOM` (two rows) and all 38 stages and `RANDOM` (five
//! rows) fit one screen with no redesign.

use std::sync::Arc;

use sf_video::background_bank::StageListing;
use sf_video::compositor::{fallback_text, ink, outline, RomFont};
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::stage_preview::{StagePreview, StagePreviews};

use super::draw::{fit, framed_wide};
use super::fight::FightKind;
use super::Transition;
use crate::roster::{MenuInput, StagePick, TournamentFighter};

/// Tiles to a row.
pub const GRID_COLS: usize = 8;
/// A tile's outline box and the step between boxes, in pixels. A cursor's
/// second outline takes the gap around its box.
const TILE: (usize, usize) = (40, 22);
const PITCH: (usize, usize) = (44, 24);
/// The grid's first column of pixels, and the pixel row just under its last
/// row of tiles (the frame's foot row, 200..208, is left empty).
const GRID_X: usize = 18;
const GRID_BOTTOM: usize = 196;
/// A tile's preview window inside its outline, and the row of its name.
const TILE_IMAGE: (usize, usize) = (38, 14);
const TILE_NAME_Y: usize = 16;
/// Where the picker line and the large preview start, in pixels.
const PICKER_Y: usize = 25;
const BIG_Y: usize = 36;
/// The large preview's widest, the caption's height and its gaps.
const BIG_MAX_W: usize = 176;
const CAPTION_H: usize = 8;
const CAPTION_GAP: (usize, usize) = (3, 4);
/// The most characters of the caption: it fits inside the frame.
const CAPTION_CELLS: usize = 40;
/// Characters of an owner's name a tile's 3×5 font holds in its window.
const TILE_NAME_CELLS: usize = 9;

/// The `RANDOM` tile's label.
const RANDOM: &str = "RANDOM";

/// A pixel box: x, y, width, height.
type Rect = (usize, usize, usize, usize);
/// The `RANDOM` tile's two stripe colours.
const STRIPES: [u32; 2] = [0xff30_3040, 0xff50_5068];
/// A tile with no preview: a dark swatch.
const NO_PREVIEW: u32 = 0xff18_1820;

/// The stage screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageScreen {
    stages: Vec<StageListing>,
    /// The listed stage under the cursor, or one past the last: `RANDOM`.
    cursor: usize,
    kind: FightKind,
    p1: TournamentFighter,
    p2: TournamentFighter,
    /// The side (0 Player 1, 1 Player 2) at the controls.
    picker: usize,
    /// Each stage's backdrop, by bank index; without them the tiles are
    /// plain swatches.
    previews: Option<Arc<StagePreviews>>,
}

/// The stage the `RANDOM` tile gives for `seed` (the session's frame
/// count when the player confirms): any of `stages`, spread evenly.
pub fn random_pick(stages: &[StageListing], seed: u32) -> StagePick {
    let mut z = u64::from(seed).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    StagePick::Bank(stages[(z % stages.len() as u64) as usize].index)
}

/// The owner of the stage a fighter's opponent is sent to: a fighter's own
/// stage, and for Ryu and Ken, who have none, a stage of their home
/// country (Retsu's in Japan, Joe's in the USA).
pub fn home_owner(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ryu => "Retsu",
        TournamentFighter::Ken => "Joe",
        other => other.name(),
    }
}

impl StageScreen {
    /// The screen over `stages` for the fight of `kind` that `p1` and `p2`
    /// have locked, with side `picker` at the controls. The cursor starts
    /// on `played` (a rematch's stage) when that is listed, else on the
    /// home stage of `p2`, else on the first stage. `None` when there is
    /// no stage to pick.
    pub fn open(
        stages: &[StageListing],
        kind: FightKind,
        p1: TournamentFighter,
        p2: TournamentFighter,
        picker: usize,
        played: Option<StagePick>,
    ) -> Option<Self> {
        let home = || stages.iter().position(|s| s.owner.eq_ignore_ascii_case(home_owner(p2)));
        let cursor = match played {
            Some(StagePick::Bank(index)) => stages.iter().position(|s| s.index == index),
            None => None,
        };
        let cursor = cursor.or_else(home).unwrap_or(0);
        (!stages.is_empty()).then(|| StageScreen { stages: stages.to_vec(), cursor, kind, p1, p2, picker, previews: None })
    }

    /// The screen with each stage's backdrop (by bank index) for its tiles
    /// and the large preview.
    pub fn with_previews(mut self, previews: Arc<StagePreviews>) -> Self {
        self.previews = Some(previews);
        self
    }

    /// The side at the controls.
    pub fn picker(&self) -> usize {
        self.picker
    }

    /// With two human pads, only the assigned picker's inputs count.
    pub fn two_players(&self) -> bool {
        matches!(self.kind, FightKind::Local | FightKind::Dramatic { co_op: true })
    }

    /// One input from `side`: only the picker's moves the cursor.
    pub fn apply_side(&mut self, side: usize, input: MenuInput) -> Option<Transition> {
        (side == self.picker).then(|| self.apply(input)).flatten()
    }

    /// Whether the cursor is on the `RANDOM` tile.
    pub fn on_random(&self) -> bool {
        self.cursor == self.stages.len()
    }

    /// A replicated stage cursor must name a stage this screen lists. Random
    /// remains a cursor choice; the session resolves its shared seed.
    pub(crate) fn set_remote_pick(&mut self, pick: StagePick, random: bool, locked: bool) -> Option<Transition> {
        let StagePick::Bank(index) = pick;
        let cursor = self.stages.iter().position(|stage| stage.index == index)?;
        self.cursor = if random { self.stages.len() } else { cursor };
        Some(if locked {
            if random { Transition::RandomStage } else { Transition::Stage(pick) }
        } else { Transition::CursorMoved })
    }

    /// The listed stages the `RANDOM` tile picks among.
    pub fn stages(&self) -> &[StageListing] {
        &self.stages
    }

    /// The native fighter whose stage this listing uses, when it has one.
    pub(super) fn owner(&self, pick: StagePick) -> Option<TournamentFighter> {
        let StagePick::Bank(index) = pick;
        let stage = self.stages.iter().find(|stage| stage.index == index)?;
        TournamentFighter::from_name(&stage.owner).ok()
    }

    pub fn name(&self) -> &'static str {
        "stage select"
    }

    /// The fight the pick leads to: its kind and the fighters.
    pub fn fight(&self) -> (FightKind, TournamentFighter, TournamentFighter) {
        (self.kind, self.p1, self.p2)
    }

    /// The stage under the cursor; `None` on the `RANDOM` tile.
    pub fn cursor(&self) -> Option<&StageListing> {
        self.stages.get(self.cursor)
    }

    /// Tiles on the grid: the stages and the `RANDOM` tile.
    pub fn tiles(&self) -> usize {
        self.stages.len() + 1
    }

    /// Rows of tiles the grid takes.
    pub fn rows(&self) -> usize {
        self.tiles().div_ceil(GRID_COLS)
    }

    /// The pixel row of the grid's first tile: the grid sits on the screen's
    /// foot, so the large preview takes what its rows leave.
    pub fn grid_top(&self) -> usize {
        GRID_BOTTOM - (self.rows() * PITCH.1 - (PITCH.1 - TILE.1))
    }

    /// The outline box (x, y, w, h) of tile `index`; the `RANDOM` tile is
    /// the one past the last stage.
    pub fn tile_rect(&self, index: usize) -> Rect {
        (GRID_X + index % GRID_COLS * PITCH.0, self.grid_top() + index / GRID_COLS * PITCH.1, TILE.0, TILE.1)
    }

    /// The large preview's window (x, y, w, h), at the backdrop's shape.
    pub fn big_rect(&self) -> Rect {
        let room = self.grid_top() - BIG_Y - CAPTION_GAP.0 - CAPTION_H - CAPTION_GAP.1;
        let w = (room * 192 / 112).min(BIG_MAX_W);
        let h = w * 112 / 192;
        ((WIDTH - w) / 2, BIG_Y, w, h)
    }

    /// Up and left step back, down and right step forward (wrapping,
    /// through the `RANDOM` tile, in the order the tiles are drawn),
    /// confirm or Start picks the stage under the cursor or, on `RANDOM`,
    /// asks for any listed stage, back returns to the select.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        let n = self.stages.len() + 1;
        match input {
            MenuInput::Up | MenuInput::Left => {
                self.cursor = (self.cursor + n - 1) % n;
                Some(Transition::CursorMoved)
            }
            MenuInput::Down | MenuInput::Right => {
                self.cursor = (self.cursor + 1) % n;
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm | MenuInput::Start => Some(match self.cursor() {
                Some(stage) => Transition::Stage(StagePick::Bank(stage.index)),
                None => Transition::RandomStage,
            }),
            MenuInput::Back => Some(Transition::Back),
            _ => None,
        }
    }

    /// The framed screen: `STAGE SELECT`, who is picking, the cursor's
    /// stage large with its owner and place, the grid of tiles with the
    /// `RANDOM` tile last, and the cursor's tile framed in the picker's
    /// colour.
    pub fn render(&self, font: &RomFont<'_>) -> Frame {
        let mut frame = framed_wide(font, "STAGE SELECT", "");
        let argb = &mut frame.argb;
        let gold = font.pen(ink::GOLD_A, 2);
        // Training has one player at the controls, always drawn as Player 1.
        let (side_ink, side_color) = if self.picker == 1 && self.two_players() {
            (ink::P2_BLUE, font.pen(ink::P2_BLUE, 2))
        } else {
            (ink::YELLOW, font.pen(ink::YELLOW, 0))
        };
        // In Training either pad moves the cursor, so no side is named.
        if self.two_players() {
            let picking = format!("P{} PICKS", self.picker + 1);
            font.text_px(argb, (WIDTH - 8 * picking.len()) / 2, PICKER_Y, &picking, side_ink);
        }

        // The large preview, and under it the owner and place.
        let (bx, by, bw, bh) = self.big_rect();
        outline(argb, bx - 1, by - 1, bw + 2, bh + 2, gold);
        match self.preview(self.cursor) {
            Some(p) => blit_scaled(argb, (bx, by, bw, bh), p, (0, 0, p.width, p.height)),
            None => blank_or_random(argb, (bx, by, bw, bh), self.on_random(), side_color),
        }
        let caption = match self.cursor() {
            Some(stage) => format!("{} \u{b7} {}", stage.owner.to_ascii_uppercase(), stage.place.to_ascii_uppercase()),
            None => format!("{RANDOM} \u{b7} ANY STAGE"),
        };
        let caption = fit(&caption, CAPTION_CELLS);
        let caption_y = by + bh + CAPTION_GAP.0;
        font.text_px(argb, (WIDTH - 8 * caption.chars().count()) / 2, caption_y, &caption, side_ink);

        // The tiles.
        let blue = font.pen(ink::BLUE, 2);
        for i in 0..self.tiles() {
            let (x, y, w, h) = self.tile_rect(i);
            let here = i == self.cursor;
            if here {
                outline(argb, x - 1, y - 1, w + 2, h + 2, side_color);
            }
            outline(argb, x, y, w, h, if here { side_color } else { gold });
            let window = (x + 1, y + 1, TILE_IMAGE.0, TILE_IMAGE.1);
            match self.preview(i) {
                Some(p) => {
                    // A band across the backdrop, a little above its middle.
                    let band = p.width * TILE_IMAGE.1 / TILE_IMAGE.0;
                    blit_scaled(argb, window, p, (0, (p.height - band) * 2 / 5, p.width, band));
                }
                None => blank_or_random(argb, window, i == self.stages.len(), side_color),
            }
            let name = match self.stages.get(i) {
                Some(stage) => fit(&stage.owner.to_ascii_uppercase(), TILE_NAME_CELLS),
                None => RANDOM.to_owned(),
            };
            let name_x = x + (w - (4 * name.chars().count() - 1)) / 2;
            fallback_text(argb, name_x, y + TILE_NAME_Y, &name, if here { side_color } else { blue }, 1);
        }
        frame
    }

    /// The backdrop of tile `index`, when it is a stage with one.
    fn preview(&self, index: usize) -> Option<&StagePreview> {
        self.previews.as_ref()?.get(&self.stages.get(index)?.index)
    }
}

/// `src`'s (x, y, w, h) window scaled into `to` (x, y, w, h), each pixel
/// the mean of the source pixels it covers.
fn blit_scaled(argb: &mut [u32], to: Rect, src: &StagePreview, from: Rect) {
    let (tx, ty, tw, th) = to;
    let (fx, fy, fw, fh) = from;
    for dy in 0..th {
        let y0 = fy + dy * fh / th;
        let y1 = (fy + (dy + 1) * fh / th).max(y0 + 1);
        for dx in 0..tw {
            let x0 = fx + dx * fw / tw;
            let x1 = (fx + (dx + 1) * fw / tw).max(x0 + 1);
            let mut sum = [0u32; 3];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let p = src.argb[sy * src.width + sx];
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total += (p >> (16 - 8 * c)) & 0xff;
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            put(argb, tx + dx, ty + dy, 0xff00_0000 | ((sum[0] / n) << 16) | ((sum[1] / n) << 8) | (sum[2] / n));
        }
    }
}

/// A window with no backdrop: the `RANDOM` tile's stripes under a `?` in
/// `mark`, or a dark swatch for a stage whose preview is missing.
fn blank_or_random(argb: &mut [u32], to: Rect, random: bool, mark: u32) {
    let (x, y, w, h) = to;
    for dy in 0..h {
        for dx in 0..w {
            let color = if random { STRIPES[((dx + dy) / 4) % 2] } else { NO_PREVIEW };
            put(argb, x + dx, y + dy, color);
        }
    }
    if random {
        let scale = (h / 7).clamp(1, 8);
        fallback_text(argb, x + (w - 3 * scale) / 2, y + (h - 5 * scale) / 2, "?", mark, scale);
    }
}

fn put(argb: &mut [u32], x: usize, y: usize, color: u32) {
    if x < WIDTH && y < HEIGHT {
        argb[y * WIDTH + x] = color;
    }
}
