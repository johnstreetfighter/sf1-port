//! The ROM-missing screen (spec #150, section 5.5): no folder holds a ROM
//! set that matches the manifest. There is no ROM to draw with, so the
//! screen is drawn in the compositor's ROM-independent 3×5 font. It names
//! the set, lists each folder looked in with what was wrong there, and
//! offers `RETRY` (look again) and `EXIT`.

use sf_video::compositor::{fallback_text, outline, SELECT_BLUE, SELECT_GOLD, SELECT_YELLOW};
use sf_video::render::{Frame, HEIGHT, WIDTH};

use super::session::{menu_events, FrameInputs};
use super::{MenuItem, Transition};
use crate::roster::MenuInput;

/// The 3×5 font's advance and line height at 1×.
const ADVANCE: usize = 4;
const LINE: usize = 8;
/// The body's left edge and its width in characters.
const LEFT: usize = 16;
const COLUMNS: usize = (WIDTH - 2 * LEFT) / ADVANCE;
const ITEMS: [MenuItem; 2] = [MenuItem::Retry, MenuItem::Exit];
const DIM: u32 = 0xff_80_80_88;

/// One folder looked in and what was wrong there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Looked {
    /// The folder, or what names it when it is not set.
    pub folder: String,
    pub problem: String,
}

#[derive(Clone, Debug)]
pub struct RomMissing {
    set: String,
    description: String,
    looked: Vec<Looked>,
    cursor: usize,
    prev: FrameInputs,
}

impl RomMissing {
    /// `set` and `description` are the manifest's (`sf`, "US set 1 ...").
    pub fn open(set: &str, description: &str, looked: Vec<Looked>) -> Self {
        RomMissing {
            set: set.to_owned(),
            description: description.to_owned(),
            looked,
            cursor: 0,
            prev: FrameInputs::default(),
        }
    }

    pub fn name(&self) -> &'static str {
        "rom missing"
    }

    pub fn cursor(&self) -> MenuItem {
        ITEMS[self.cursor]
    }

    pub fn looked(&self) -> &[Looked] {
        &self.looked
    }

    /// A retry that found nothing either: the folders' new results.
    pub fn update(&mut self, looked: Vec<Looked>) {
        self.looked = looked;
    }

    /// Up and down move between the two items; confirm or start chooses.
    /// Back does nothing: there is nowhere to go back to.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        match input {
            MenuInput::Up | MenuInput::Down => {
                self.cursor = 1 - self.cursor;
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm | MenuInput::Start => Some(Transition::Choose(ITEMS[self.cursor])),
            _ => None,
        }
    }

    /// One frame of held inputs: the menu events since the last frame,
    /// applied in order; the first choice ends the frame.
    pub fn step(&mut self, input: FrameInputs) -> Option<Transition> {
        let prev = std::mem::replace(&mut self.prev, input);
        let mut last = None;
        for event in menu_events(&prev, &input) {
            match self.apply(event) {
                Some(t @ Transition::Choose(_)) => return Some(t),
                Some(t) => last = Some(t),
                None => {}
            }
        }
        last
    }

    /// The body text, top to bottom (each folder, then its problem).
    pub fn text(&self) -> Vec<String> {
        let mut lines = vec![
            format!("THE GAME NEEDS THE ROM SET '{}':", self.set),
            self.description.clone(),
            String::new(),
            "LOOKED IN:".to_owned(),
        ];
        for looked in &self.looked {
            lines.push(fit(&looked.folder, COLUMNS - 2));
            lines.push(format!("  {}", fit(&looked.problem, COLUMNS - 2)));
        }
        lines.push(String::new());
        lines.push("PUT THE FILES IN ONE OF THESE FOLDERS, THEN CHOOSE RETRY.".to_owned());
        lines
    }

    pub fn render(&self) -> Frame {
        let mut frame = Frame {
            argb: vec![0xff00_0000; WIDTH * HEIGHT],
        };
        let argb = &mut frame.argb;
        outline(argb, 4, 4, WIDTH - 8, HEIGHT - 8, SELECT_GOLD);
        outline(argb, 7, 7, WIDTH - 14, HEIGHT - 14, SELECT_GOLD);
        let title = "ROM SET NOT FOUND";
        fallback_text(argb, centred_x(title, 2), 16, title, SELECT_GOLD, 2);
        let mut y = 36;
        for (i, line) in self.text().iter().enumerate() {
            // The folders' problems are dimmer than the folders.
            let problem = i > 3 && (i - 4) % 2 == 1 && i < 4 + 2 * self.looked.len();
            fallback_text(argb, LEFT, y, line, if problem { DIM } else { SELECT_BLUE }, 1);
            y += LINE;
        }
        for (i, item) in ITEMS.iter().enumerate() {
            let label = item.label();
            let y = HEIGHT - 48 + i * 16;
            let x = centred_x(label, 2);
            let color = if i == self.cursor { SELECT_YELLOW } else { SELECT_BLUE };
            if i == self.cursor {
                fallback_text(argb, x - 2 * 2 * ADVANCE, y, ">", SELECT_YELLOW, 2);
            }
            fallback_text(argb, x, y, label, color, 2);
        }
        frame
    }
}

/// Left edge that centres `text` at `scale`.
fn centred_x(text: &str, scale: usize) -> usize {
    let width = text.chars().count() * ADVANCE * scale;
    WIDTH.saturating_sub(width) / 2
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
