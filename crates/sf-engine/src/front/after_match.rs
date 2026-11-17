//! The after-match box: `REMATCH`, `RESELECT` and `MENU` over the dimmed
//! final frame. Online matches also show both peers' live votes. Training
//! fights never end (ADR-0013), so they never reach it.

use sf_video::compositor::{ink, RomFont};
use sf_video::render::Frame;

use super::draw::{boxed, dimmed, selected_row, stamp_offset};
use super::menu::MenuItem;
use super::Transition;
use crate::netplay::protocol::AfterMatchChoice;
use crate::roster::MenuInput;

const ITEMS: [MenuItem; 3] = [MenuItem::Rematch, MenuItem::Reselect, MenuItem::Menu];

/// The box's cells: 16 columns by 9 rows, centred.
pub const BOX_COL: usize = 16;
pub const BOX_ROW: usize = 9;
const BOX_COLS: usize = 16;
const BOX_ROWS: usize = 9;
const ONLINE_BOX_COL: usize = 6;
const ONLINE_BOX_COLS: usize = 36;
const ONLINE_BOX_ROWS: usize = 12;
const VOTE_ROW: usize = BOX_ROW + 9;

/// The after-match box.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AfterMatchBox {
    cursor: usize,
    peer_votes: Option<[Option<AfterMatchChoice>; 2]>,
    motion: Option<LocalMotion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalMotion {
    opened_at: u32,
    cursor_moved_at: Option<u32>,
}

impl AfterMatchBox {
    pub(super) fn open_local(frame: u32) -> Self {
        Self {
            motion: Some(LocalMotion { opened_at: frame, cursor_moved_at: None }),
            ..Self::default()
        }
    }

    /// An online result starts with both peers waiting to choose.
    pub fn open_online() -> Self {
        Self {
            peer_votes: Some([None; 2]),
            ..Self::default()
        }
    }

    /// Refresh the online vote display without changing the menu selection.
    pub fn set_peer_votes(&mut self, votes: [Option<AfterMatchChoice>; 2]) {
        self.peer_votes = Some(votes);
        self.motion = None;
    }

    pub fn name(&self) -> &'static str {
        "after match"
    }

    /// The text row of item `index`.
    pub fn item_row(index: usize) -> usize {
        BOX_ROW + 2 + 2 * index
    }

    /// The first text column of the items.
    pub fn item_col() -> usize {
        BOX_COL + 4
    }

    /// Up and down move (wrapping), confirm or Start chooses. Back does
    /// nothing: the match is over and one of the three must be chosen.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        match input {
            MenuInput::Up => {
                self.cursor = (self.cursor + ITEMS.len() - 1) % ITEMS.len();
                Some(Transition::CursorMoved)
            }
            MenuInput::Down => {
                self.cursor = (self.cursor + 1) % ITEMS.len();
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm | MenuInput::Start => Some(Transition::Choose(ITEMS[self.cursor])),
            _ => None,
        }
    }

    pub(super) fn apply_at(&mut self, input: MenuInput, frame: u32) -> Option<Transition> {
        let transition = self.apply(input)?;
        if let Some(motion) = self.motion.as_mut().filter(|_| transition == Transition::CursorMoved) {
            motion.cursor_moved_at = Some(frame);
        }
        Some(transition)
    }

    /// `last` dimmed, the box over it.
    pub fn render(&self, font: &RomFont<'_>, last: &Frame) -> Frame {
        self.render_box(font, last, 0, None)
    }

    pub(super) fn render_at(&self, font: &RomFont<'_>, last: &Frame, frame: u32) -> Frame {
        let offset = stamp_offset(self.motion.as_ref().map(|motion| motion.opened_at), frame);
        let cursor_age = self.motion.as_ref().and_then(|motion| motion.cursor_moved_at)
            .map(|start| frame.wrapping_sub(start));
        self.render_box(font, last, offset, cursor_age)
    }

    fn render_box(&self, font: &RomFont<'_>, last: &Frame, offset: usize, cursor_age: Option<u32>) -> Frame {
        let mut frame = dimmed(last);
        if let Some(votes) = self.peer_votes {
            boxed(
                &mut frame,
                font,
                ONLINE_BOX_COL,
                BOX_ROW,
                ONLINE_BOX_COLS,
                ONLINE_BOX_ROWS,
            );
            let text = format!(
                "P1: {}   P2: {}",
                vote_label(votes[0]),
                vote_label(votes[1])
            );
            let col = ONLINE_BOX_COL + (ONLINE_BOX_COLS - text.len()) / 2;
            font.text(&mut frame.argb, col, VOTE_ROW, &text, ink::YELLOW);
        } else {
            boxed(&mut frame, font, BOX_COL, BOX_ROW, BOX_COLS, BOX_ROWS);
        }
        for (i, item) in ITEMS.iter().enumerate() {
            let (x, y) = (Self::item_col() * 8, Self::item_row(i) * 8 + offset);
            if i == self.cursor {
                selected_row(font, &mut frame.argb, x, y, item.label(), cursor_age);
            } else {
                font.text_px(&mut frame.argb, x, y, item.label(), ink::BLUE);
            }
        }
        frame
    }
}

fn vote_label(choice: Option<AfterMatchChoice>) -> &'static str {
    match choice {
        Some(AfterMatchChoice::Rematch) => "REMATCH",
        Some(AfterMatchChoice::Reselect) => "RESELECT",
        Some(AfterMatchChoice::Leave) => "LEAVE",
        None => "WAITING...",
    }
}
