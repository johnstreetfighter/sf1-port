//! The after-match box: `REMATCH`, `RESELECT` and `MENU` over the dimmed
//! final frame. Online matches also show both peers' live votes. Training
//! fights never end (ADR-0013), so they never reach it.

use sf_video::compositor::{ink, RomFont};
use sf_video::render::Frame;

use super::draw::{boxed, dimmed};
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
}

impl AfterMatchBox {
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

    /// `last` dimmed, the box over it.
    pub fn render(&self, font: &RomFont<'_>, last: &Frame) -> Frame {
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
            let row = Self::item_row(i);
            let color = if i == self.cursor {
                font.text(&mut frame.argb, Self::item_col() - 2, row, ">", ink::YELLOW);
                ink::YELLOW
            } else {
                ink::BLUE
            };
            font.text(&mut frame.argb, Self::item_col(), row, item.label(), color);
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
