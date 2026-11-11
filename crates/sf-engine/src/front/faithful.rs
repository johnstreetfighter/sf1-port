//! The 1987 item: the `1987 CABINET` DIP screen and the faithful cabinet it
//! powers on. The cabinet is the complete original machine from power-on
//! with the DIP words set here, the coin, start and service keys live, and
//! nothing from the overhaul settings but video, sound and bindings. A
//! one-second Esc hold, shown as a small bar drawn over the frame (the
//! machine untouched), is the only way out.

use sf_video::compositor::{ink, outline, RomFont};
use sf_video::render::{render_frame, Frame};
use sf_video::romset::Assets;

use super::draw::{centred_col, framed};
use super::session::FrameInputs;
use super::Transition;
use crate::config::MatchConfig;
use crate::roster::MenuInput;
use crate::run::PowerOnMachine;
use crate::tasks::Io;

/// One DIP setting the port verified (`docs/reference/dip-switches.md`):
/// its row name, its port (1 or 2), mask, and options as (bits, label).
pub struct Dip {
    pub name: &'static str,
    pub port: u8,
    pub mask: u16,
    pub options: &'static [(u16, &'static str)],
}

const STAGES: [(u16, &str); 6] = [
    (0x0007, "5TH STAGE"),
    (0x0006, "4TH STAGE"),
    (0x0005, "3RD STAGE"),
    (0x0004, "2ND STAGE"),
    (0x0003, "1ST STAGE"),
    (0x0002, "NONE"),
];

/// The rows, in the order of the spec: coinage A, coinage B, continuation,
/// buy-in, difficulty, round time, flip screen, demo sounds, attract music,
/// country count, test mode. Each option list starts at the factory
/// setting.
pub const DIPS: [Dip; 11] = [
    Dip {
        name: "COINAGE A",
        port: 1,
        mask: 0x0007,
        options: &[
            (0x0007, "1 COIN 1 CREDIT"),
            (0x0006, "1 COIN 2 CREDITS"),
            (0x0005, "1 COIN 3 CREDITS"),
            (0x0004, "1 COIN 4 CREDITS"),
            (0x0003, "1 COIN 6 CREDITS"),
            (0x0002, "2 COINS 1 CREDIT"),
            (0x0001, "3 COINS 1 CREDIT"),
            (0x0000, "4 COINS 1 CREDIT"),
        ],
    },
    Dip {
        name: "COINAGE B",
        port: 1,
        mask: 0x0038,
        options: &[
            (0x0038, "1 COIN 1 CREDIT"),
            (0x0030, "1 COIN 2 CREDITS"),
            (0x0028, "1 COIN 3 CREDITS"),
            (0x0020, "1 COIN 4 CREDITS"),
            (0x0018, "1 COIN 6 CREDITS"),
            (0x0010, "2 COINS 1 CREDIT"),
            (0x0008, "3 COINS 1 CREDIT"),
            (0x0000, "4 COINS 1 CREDIT"),
        ],
    },
    Dip { name: "CONTINUATION", port: 2, mask: 0x0007, options: &STAGES },
    Dip {
        name: "BUY-IN",
        port: 2,
        mask: 0x0380,
        options: &[
            (0x0380, "5TH STAGE"),
            (0x0300, "4TH STAGE"),
            (0x0280, "3RD STAGE"),
            (0x0200, "2ND STAGE"),
            (0x0180, "1ST STAGE"),
            (0x0080, "NONE"),
        ],
    },
    Dip {
        name: "DIFFICULTY",
        port: 2,
        mask: 0x0060,
        options: &[(0x0060, "NORMAL"), (0x0020, "DIFFICULT"), (0x0000, "VERY DIFFICULT"), (0x0040, "EASY")],
    },
    Dip {
        name: "ROUND TIME",
        port: 2,
        mask: 0x0018,
        options: &[(0x0018, "100"), (0x0010, "150"), (0x0008, "200"), (0x0000, "250")],
    },
    Dip { name: "FLIP SCREEN", port: 1, mask: 0x0100, options: &[(0x0100, "OFF"), (0x0000, "ON")] },
    Dip { name: "DEMO SOUNDS", port: 1, mask: 0x2000, options: &[(0x0000, "ON"), (0x2000, "OFF")] },
    Dip { name: "ATTRACT MUSIC", port: 1, mask: 0x0200, options: &[(0x0200, "ON"), (0x0000, "OFF")] },
    Dip { name: "COUNTRY COUNT", port: 2, mask: 0x0400, options: &[(0x0400, "2"), (0x0000, "4")] },
    Dip { name: "TEST MODE", port: 1, mask: 0x8000, options: &[(0x8000, "OFF"), (0x0000, "ON")] },
];

/// The hint at the frame's foot.
pub const HOLD_HINT: &str = "HOLD ESC ONE SECOND TO RETURN";
/// The rows' first text row, label column and value end column.
pub const FIRST_ROW: usize = 6;
const LABEL_COL: usize = 7;
const VALUE_END_COL: usize = 41;
/// The row of `POWER ON`, apart from the DIP rows.
pub const POWER_ON_ROW: usize = FIRST_ROW + DIPS.len() + 2;

/// The `1987 CABINET` screen: the DIP words being set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DipScreen {
    dsw1: u16,
    dsw2: u16,
    /// 0..=10 the DIP rows, 11 `POWER ON`.
    cursor: usize,
}

impl DipScreen {
    pub fn open(dsw1: u16, dsw2: u16) -> Self {
        DipScreen { dsw1, dsw2, cursor: 0 }
    }

    pub fn name(&self) -> &'static str {
        "1987 cabinet"
    }

    pub fn dsw(&self) -> (u16, u16) {
        (self.dsw1, self.dsw2)
    }

    fn word(&self, port: u8) -> u16 {
        if port == 1 {
            self.dsw1
        } else {
            self.dsw2
        }
    }

    /// Row `i`'s current option (its index in [`Dip::options`]; a setting
    /// outside the list reads as the factory one).
    pub fn option(&self, i: usize) -> usize {
        let dip = &DIPS[i];
        let bits = self.word(dip.port) & dip.mask;
        dip.options.iter().position(|&(v, _)| v == bits).unwrap_or(0)
    }

    fn set(&mut self, i: usize, option: usize) {
        let dip = &DIPS[i];
        let bits = dip.options[option].0;
        let word = if dip.port == 1 { &mut self.dsw1 } else { &mut self.dsw2 };
        *word = (*word & !dip.mask) | bits;
    }

    /// The text row of row `i` (`POWER ON` is row [`DIPS`]`.len()`).
    pub fn row_of(i: usize) -> usize {
        if i < DIPS.len() {
            FIRST_ROW + i
        } else {
            POWER_ON_ROW
        }
    }

    /// Up and down move (wrapping), left and right change a DIP, confirm on
    /// `POWER ON` powers the cabinet on, back leaves (the DIP words kept).
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        let n = DIPS.len() + 1;
        match input {
            MenuInput::Up => {
                self.cursor = (self.cursor + n - 1) % n;
                Some(Transition::CursorMoved)
            }
            MenuInput::Down => {
                self.cursor = (self.cursor + 1) % n;
                Some(Transition::CursorMoved)
            }
            MenuInput::Left | MenuInput::Right if self.cursor < DIPS.len() => {
                let len = DIPS[self.cursor].options.len();
                let d = if input == MenuInput::Right { 1 } else { len - 1 };
                self.set(self.cursor, (self.option(self.cursor) + d) % len);
                Some(Transition::ValueChanged)
            }
            MenuInput::Confirm | MenuInput::Start if self.cursor == DIPS.len() => {
                Some(Transition::Choose(super::MenuItem::PowerOn))
            }
            MenuInput::Back => Some(Transition::Back),
            _ => None,
        }
    }

    /// The framed screen: the DIP rows as `NAME .... OPTION`, `POWER ON`,
    /// and the hold hint at the foot.
    pub fn render(&self, font: &RomFont<'_>) -> Frame {
        let mut frame = framed(font, "1987 CABINET", HOLD_HINT);
        for (i, dip) in DIPS.iter().enumerate() {
            let row = Self::row_of(i);
            let color = if i == self.cursor {
                font.text(&mut frame.argb, LABEL_COL - 2, row, ">", ink::YELLOW);
                ink::YELLOW
            } else {
                ink::BLUE
            };
            let value = dip.options[self.option(i)].1;
            let vcol = VALUE_END_COL - value.len();
            font.text(&mut frame.argb, LABEL_COL, row, dip.name, color);
            let dots = LABEL_COL + dip.name.len() + 1;
            if vcol > dots + 1 {
                font.text(&mut frame.argb, dots, row, &".".repeat(vcol - 1 - dots), ink::DIM);
            }
            font.text(&mut frame.argb, vcol, row, value, color);
        }
        let on = "POWER ON";
        let color = if self.cursor == DIPS.len() { ink::YELLOW } else { ink::BLUE };
        if self.cursor == DIPS.len() {
            font.text(&mut frame.argb, centred_col(on) - 2, POWER_ON_ROW, ">", ink::YELLOW);
        }
        font.text_centred(&mut frame.argb, POWER_ON_ROW, on, color);
        frame
    }
}

/// Frames Esc must be held to leave the cabinet.
pub const HOLD_FRAMES: u32 = 60;

/// The hold bar: where it sits over the frame and its size.
pub const BAR: (usize, usize, usize, usize) = (8, 8, 64, 6);

/// The faithful cabinet: the original machine from power-on.
#[derive(Clone)]
pub struct Cabinet {
    machine: PowerOnMachine,
    held: u32,
}

/// What a cabinet frame ended in.
pub enum CabinetEvent {
    /// Esc was held for one second.
    Leave,
    /// The machine reached original branches the translation lacks.
    Untranslated { frame: u32, addresses: Vec<u32> },
}

impl Cabinet {
    /// Power on with `dsw1` and `dsw2` and every toggle off.
    pub fn power_on(program_rom: &[u8], dsw1: u16, dsw2: u16) -> Self {
        let machine = PowerOnMachine::with_io(program_rom.to_vec(), Io { dsw1, dsw2, ..Io::idle() })
            .with_match_config(MatchConfig::faithful());
        Cabinet { machine, held: 0 }
    }

    pub fn machine(&self) -> &PowerOnMachine {
        &self.machine
    }

    /// Frames Esc has been held.
    pub fn held(&self) -> u32 {
        self.held
    }

    /// One frame: both players' inputs and the cabinet switches straight to
    /// the machine. Esc (the menu back key) held counts towards leaving and
    /// reaches nothing else.
    pub fn step(&mut self, input: &FrameInputs) -> Option<CabinetEvent> {
        self.held = if input.menu.back { self.held + 1 } else { 0 };
        if self.held >= HOLD_FRAMES {
            return Some(CabinetEvent::Leave);
        }
        let m = &mut self.machine;
        let [r, l, d, u] = input.p1.directions;
        m.set_p1_direction(r, l, d, u);
        m.set_p1_buttons(input.p1.buttons);
        let [r, l, d, u] = input.p2.directions;
        m.set_p2_direction(r, l, d, u);
        m.set_p2_buttons(input.p2.buttons);
        let [coin1, coin2, start1, start2, service1] = input.cabinet;
        m.set_cabinet_inputs(coin1, coin2, start1, start2, service1);
        m.step();
        let addresses = m.unrecovered();
        (!addresses.is_empty()).then_some(CabinetEvent::Untranslated {
            frame: m.state.frame,
            addresses,
        })
    }

    /// The machine's own frame, with the hold bar drawn over it while Esc
    /// is down (a presentation overlay; the machine is not touched).
    pub fn render(&self, assets: &Assets, font: &RomFont<'_>) -> Frame {
        let mut frame = render_frame(assets, &self.machine.state.scene());
        if self.held > 0 {
            let (x, y, w, h) = BAR;
            let fill = (w - 2) * self.held as usize / HOLD_FRAMES as usize;
            outline(&mut frame.argb, x, y, w, h, font.pen(ink::GOLD_A, 0));
            let yellow = font.pen(ink::YELLOW, 0);
            for yy in y + 1..y + h - 1 {
                for xx in x + 1..x + 1 + fill {
                    frame.argb[yy * sf_video::render::WIDTH + xx] = yellow;
                }
            }
        }
        frame
    }
}
