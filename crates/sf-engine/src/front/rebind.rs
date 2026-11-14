//! The rebind walk (spec #150, section 5.5, Controls tab): `REBIND P1`,
//! `REBIND P2` and `REBIND MENU` walk their logical inputs in order. A key
//! or a pad button pressed binds it, Esc keeps the old binding, and a
//! duplicate is refused with a one-line notice. The screen knows bindings
//! only by name; the host turns the names back into keys and pad buttons.
//!
//! A key is refused when anything else on the keyboard has it (one keyboard
//! serves both players, the menu and the cabinet keys). A pad button is
//! refused when another input of the same walk has it: each side's pad is
//! its own device, and the menu's pad buttons share the player's by design
//! (Cross is both confirm and LK).

use sf_video::compositor::{ink, RomFont};
use sf_video::render::Frame;

use super::draw::framed_wide_at;
use super::Transition;

/// A logical input's key and pad button, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bind {
    pub key: &'static str,
    pub pad: &'static str,
}

/// Every rebindable input's binding: each player's four directions and six
/// buttons (right, left, down, up, LP, MP, HP, LK, MK, HK), the five menu
/// inputs (confirm, back, start, tab left, tab right), and the cabinet's
/// keys (coin 1, coin 2, start 1, start 2, service 1), which are not
/// walked but which no walked key may take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binds {
    pub p1: [Bind; 10],
    pub p2: [Bind; 10],
    pub menu: [Bind; 5],
    pub cabinet: [&'static str; 5],
}

/// The names the walk shows for each list's inputs.
pub const PLAYER_INPUTS: [&str; 10] = ["RIGHT", "LEFT", "DOWN", "UP", "LP", "MP", "HP", "LK", "MK", "HK"];
pub const MENU_INPUTS: [&str; 5] = ["CONFIRM", "BACK", "START", "TAB LEFT", "TAB RIGHT"];
const CABINET_INPUTS: [&str; 5] = ["COIN 1", "COIN 2", "START 1", "START 2", "SERVICE 1"];

/// Which list a walk rebinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Walk {
    P1,
    P2,
    Menu,
}

impl Walk {
    pub const fn header(self) -> &'static str {
        match self {
            Walk::P1 => "REBIND P1",
            Walk::P2 => "REBIND P2",
            Walk::Menu => "REBIND MENU",
        }
    }

    fn names(self) -> &'static [&'static str] {
        match self {
            Walk::P1 | Walk::P2 => &PLAYER_INPUTS,
            Walk::Menu => &MENU_INPUTS,
        }
    }
}

/// A key or pad button pressed on one frame, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Key(&'static str),
    Pad(&'static str),
}

/// The key that keeps an input's old binding.
pub const KEEP_KEY: &str = "Escape";

const FIRST_ROW: usize = 6;
const LABEL_COL: usize = 4;
const KEY_COL: usize = 17;
const PAD_COL: usize = 33;
const PROMPT_ROW: usize = 21;
const NOTICE_ROW: usize = 23;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebindScreen {
    walk: Walk,
    binds: Binds,
    at: usize,
    notice: Option<String>,
}

impl RebindScreen {
    pub fn open(walk: Walk, binds: Binds) -> Self {
        RebindScreen {
            walk,
            binds,
            at: 0,
            notice: None,
        }
    }

    pub fn name(&self) -> &'static str {
        "rebind"
    }

    pub fn walk(&self) -> Walk {
        self.walk
    }

    pub fn binds(&self) -> &Binds {
        &self.binds
    }

    /// The input waiting for a press.
    pub fn at(&self) -> usize {
        self.at
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    fn list(&self) -> &[Bind] {
        match self.walk {
            Walk::P1 => &self.binds.p1,
            Walk::P2 => &self.binds.p2,
            Walk::Menu => &self.binds.menu,
        }
    }

    fn list_mut(&mut self) -> &mut [Bind] {
        match self.walk {
            Walk::P1 => &mut self.binds.p1,
            Walk::P2 => &mut self.binds.p2,
            Walk::Menu => &mut self.binds.menu,
        }
    }

    /// Who else on the keyboard has `key`, by name.
    fn key_owner(&self, key: &str) -> Option<String> {
        let lists = [
            (Walk::P1, &self.binds.p1[..], "P1 "),
            (Walk::P2, &self.binds.p2[..], "P2 "),
            (Walk::Menu, &self.binds.menu[..], "MENU "),
        ];
        for (walk, list, prefix) in lists {
            for (i, bind) in list.iter().enumerate() {
                if bind.key == key && !(walk == self.walk && i == self.at) {
                    return Some(format!("{prefix}{}", walk.names()[i]));
                }
            }
        }
        let i = self.binds.cabinet.iter().position(|&k| k == key)?;
        Some(CABINET_INPUTS[i].to_owned())
    }

    /// One press: Esc keeps the old binding and moves on; a free key or
    /// pad button binds and moves on; a taken one is refused with a notice
    /// and the walk waits. The last input's binding ends the walk (`Back`).
    pub fn press(&mut self, press: Press) -> Option<Transition> {
        let at = self.at;
        match press {
            Press::Key(KEEP_KEY) => {}
            Press::Key(key) => {
                if let Some(owner) = self.key_owner(key) {
                    self.notice = Some(format!("{} IS ALREADY {owner}", key.to_ascii_uppercase()));
                    return None;
                }
                self.list_mut()[at].key = key;
            }
            Press::Pad(pad) => {
                let names = self.walk.names();
                if let Some(i) = self.list().iter().enumerate().position(|(i, b)| i != at && b.pad == pad) {
                    self.notice = Some(format!("{pad} IS ALREADY {}", names[i]));
                    return None;
                }
                self.list_mut()[at].pad = pad;
            }
        }
        self.notice = None;
        self.at += 1;
        if self.at == self.list().len() {
            Some(Transition::Back)
        } else {
            Some(Transition::CursorMoved)
        }
    }

    pub fn render(&self, font: &RomFont<'_>) -> Frame {
        self.render_with_title(font, 0)
    }

    pub(super) fn render_with_title(&self, font: &RomFont<'_>, title_offset: usize) -> Frame {
        let mut frame = framed_wide_at(font, self.walk.header(), "ESC KEEPS THE OLD ONE", title_offset);
        let argb = &mut frame.argb;
        font.text(argb, KEY_COL, FIRST_ROW - 2, "KEY", ink::DIM);
        font.text(argb, PAD_COL, FIRST_ROW - 2, "PAD", ink::DIM);
        for (i, (name, bind)) in self.walk.names().iter().zip(self.list()).enumerate() {
            let row = FIRST_ROW + i;
            let color = if i == self.at { ink::YELLOW } else { ink::BLUE };
            if i == self.at {
                font.text(argb, LABEL_COL - 2, row, ">", ink::YELLOW);
            }
            font.text(argb, LABEL_COL, row, name, color);
            font.text(argb, KEY_COL, row, &bind.key.to_ascii_uppercase(), color);
            font.text(argb, PAD_COL, row, bind.pad, color);
        }
        if let Some(name) = self.walk.names().get(self.at) {
            font.text_centred(argb, PROMPT_ROW, &format!("PRESS A KEY OR PAD BUTTON FOR {name}"), ink::BLUE);
        }
        if let Some(notice) = &self.notice {
            font.text_centred(argb, NOTICE_ROW, notice, ink::YELLOW);
        }
        frame
    }
}
