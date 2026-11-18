//! The ROM browser (#129): the ROM-missing screen's way to choose the ROM
//! set's folder or archive with a pad alone, for hosts that have no file
//! picker to show (a Steam Deck in Gaming Mode). The engine reads no
//! folders: the host lists one, the screen shows it, and each choice goes
//! back to the host as a [`Browse`] request. Like the ROM-missing screen
//! it is drawn in the compositor's ROM-independent 3×5 font.

use std::ops::Range;

use sf_video::compositor::{fallback_text, outline, SELECT_BLUE, SELECT_GOLD, SELECT_YELLOW};
use sf_video::render::{Frame, HEIGHT, WIDTH};

use super::session::{held_repeat, menu_events, FrameInputs};
use crate::roster::MenuInput;

const USE_ROW: usize = 0;
const UP_ROW: usize = 1;
const FIRST_SOURCE_ROW: usize = 2;
/// Rows of the list on one screen.
const SHOWN: usize = 14;
/// The 3×5 font's advance at 1×, the list's left edge and its row height.
const ADVANCE: usize = 4;
const LEFT: usize = 24;
const ROW: usize = 10;
const TOP: usize = 52;
const COLUMNS: usize = (WIDTH - 2 * LEFT) / ADVANCE;
const DIM: u32 = 0xff_80_80_88;

/// One thing in a folder that could hold the ROM set: a folder to enter,
/// or a ZIP or 7z archive to choose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub name: String,
    pub archive: bool,
}

/// What the player asked for; the host answers with [`RomBrowser::show`],
/// [`RomBrowser::problem`], or by leaving the browser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Browse {
    /// Load the ROM set from the folder shown.
    Use,
    /// Show the folder above.
    Up,
    /// Show the named folder inside this one.
    Enter(String),
    /// Load the ROM set from the named archive in this folder.
    Choose(String),
    /// Go back to the ROM-missing screen.
    Cancel,
}

#[derive(Clone, Debug)]
pub struct RomBrowser {
    folder: String,
    sources: Vec<Source>,
    cursor: usize,
    notice: Option<String>,
    prev: FrameInputs,
    held: Option<(MenuInput, u32)>,
}

impl RomBrowser {
    pub fn open(folder: &str, sources: Vec<Source>) -> Self {
        RomBrowser {
            folder: folder.to_owned(),
            sources,
            cursor: 0,
            notice: None,
            prev: FrameInputs::default(),
            held: None,
        }
    }

    /// Opened with `held` still down: the button that chose
    /// `BROWSE FOLDER` chooses nothing here.
    pub fn resume(&mut self, held: FrameInputs) {
        self.prev = held;
    }

    pub fn name(&self) -> &'static str {
        "rom browser"
    }

    /// Another folder, listed by the host: the cursor starts at the top.
    pub fn show(&mut self, folder: &str, sources: Vec<Source>) {
        self.folder = folder.to_owned();
        self.sources = sources;
        self.cursor = 0;
        self.notice = None;
    }

    /// Why the last request could not be met (no ROM set there, a folder
    /// that cannot be read). The folder shown stays as it is.
    pub fn problem(&mut self, problem: &str) {
        self.notice = Some(problem.to_owned());
    }

    pub fn folder(&self) -> &str {
        &self.folder
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Every row, top to bottom; a folder's name ends in `/`.
    pub fn rows(&self) -> Vec<String> {
        let sources = self.sources.iter().map(|source| {
            if source.archive { source.name.clone() } else { format!("{}/", source.name) }
        });
        ["USE THIS FOLDER".to_owned(), "UP ONE FOLDER".to_owned()].into_iter().chain(sources).collect()
    }

    /// The rows on screen now: a window that keeps the cursor in view.
    pub fn visible(&self) -> Range<usize> {
        let count = FIRST_SOURCE_ROW + self.sources.len();
        let start = (self.cursor + 1).saturating_sub(SHOWN);
        start..(start + SHOWN).min(count)
    }

    /// Up and down move and wrap, Confirm or Start asks for the row under
    /// the cursor, Back cancels.
    pub fn apply(&mut self, input: MenuInput) -> Option<Browse> {
        let count = FIRST_SOURCE_ROW + self.sources.len();
        match input {
            MenuInput::Up => self.cursor = (self.cursor + count - 1) % count,
            MenuInput::Down => self.cursor = (self.cursor + 1) % count,
            MenuInput::Back => return Some(Browse::Cancel),
            MenuInput::Confirm | MenuInput::Start => {
                return Some(match self.cursor {
                    USE_ROW => Browse::Use,
                    UP_ROW => Browse::Up,
                    row => {
                        let source = &self.sources[row - FIRST_SOURCE_ROW];
                        if source.archive { Browse::Choose(source.name.clone()) } else { Browse::Enter(source.name.clone()) }
                    }
                });
            }
            _ => {}
        }
        None
    }

    /// One frame of held inputs: the menu events since the last frame and
    /// a held Up or Down's repeats, applied in order. The first request
    /// ends the frame.
    pub fn step(&mut self, input: FrameInputs) -> Option<Browse> {
        let prev = std::mem::replace(&mut self.prev, input);
        let held = |i: usize| input.p1.directions[i] || input.p2.directions[i];
        let repeat = held_repeat(&mut self.held, held(3), held(2));
        menu_events(&prev, &input).into_iter().chain(repeat).find_map(|event| self.apply(event))
    }

    pub fn render(&self) -> Frame {
        let mut frame = Frame {
            argb: vec![0xff00_0000; WIDTH * HEIGHT],
        };
        let argb = &mut frame.argb;
        outline(argb, 4, 4, WIDTH - 8, HEIGHT - 8, SELECT_GOLD);
        outline(argb, 7, 7, WIDTH - 14, HEIGHT - 14, SELECT_GOLD);
        let title = "FIND THE ROM SET";
        fallback_text(argb, WIDTH.saturating_sub(title.len() * ADVANCE * 2) / 2, 16, title, SELECT_GOLD, 2);
        fallback_text(argb, LEFT, 36, &fit(&self.folder, COLUMNS), SELECT_BLUE, 1);
        let rows = self.rows();
        for (line, row) in self.visible().enumerate() {
            let y = TOP + line * ROW;
            let color = if row == self.cursor { SELECT_YELLOW } else { SELECT_BLUE };
            if row == self.cursor {
                fallback_text(argb, LEFT - 2 * ADVANCE, y, ">", SELECT_YELLOW, 1);
            }
            fallback_text(argb, LEFT, y, &fit(&rows[row], COLUMNS), color, 1);
        }
        if let Some(notice) = &self.notice {
            // A problem's start says most, unlike a path's.
            let shown: String = notice.chars().take(COLUMNS).collect();
            fallback_text(argb, LEFT, HEIGHT - 28, &shown, SELECT_YELLOW, 1);
        }
        fallback_text(argb, LEFT, HEIGHT - 18, "UP/DN MOVE  CONFIRM CHOOSE  BACK CANCEL", DIM, 1);
        frame
    }
}

/// `text` cut to `columns` characters, keeping its end (a path's last
/// folders say most).
fn fit(text: &str, columns: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= columns {
        return text.to_owned();
    }
    let tail: String = chars[chars.len() - (columns - 3)..].iter().collect();
    format!("...{tail}")
}
