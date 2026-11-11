//! The game's per-frame logical inputs: each side from the keyboard or its
//! pad (the Controls tab's `P1 DEVICE` and `P2 DEVICE`), the cabinet keys,
//! the menu keys, the pads' menu buttons, the pads lost on the frame, and
//! the key or pad button first pressed (for the rebind walk). Esc is
//! always back; it is never bound.

use minifb::Key;
use sf_engine::front::rebind::Press;
use sf_engine::front::settings::Device;
use sf_engine::front::{FrameInputs, MenuKeys, PlayerInputs};

pub use crate::bindings::{MenuBindings, DEFAULT_MENU_KEYS};
use crate::bindings::{key_to_name, BindingsTable, PadBindings};
use crate::keys::Keyboard;
use crate::pads::{PadButton, PadFrame, PadState};

/// What drives a side this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Keyboard,
    /// A numbered pad (0 is `PAD 1`).
    Pad(usize),
    /// The side's pad is lost, or has not been seen: the side idles. The
    /// keyboard never takes over on its own.
    Idle,
}

/// The pad number a side's device names, if any: `AUTO` names `PAD 1` for
/// P1 and `PAD 2` for P2 once that number has held a pad.
fn pad_number(device: Device, side: usize, pads: &PadFrame) -> Option<usize> {
    match device {
        Device::Keyboard => None,
        Device::Pad1 => Some(0),
        Device::Pad2 => Some(1),
        Device::Auto => pads.seen[side].then_some(side),
    }
}

/// What drives `side` this frame.
pub fn source(device: Device, side: usize, pads: &PadFrame) -> Source {
    match pad_number(device, side, pads) {
        None => Source::Keyboard,
        Some(n) if pads.pads[n].is_some() => Source::Pad(n),
        Some(_) => Source::Idle,
    }
}

fn pad_player(pad: &PadState, bindings: &PadBindings) -> PlayerInputs {
    PlayerInputs {
        directions: std::array::from_fn(|i| pad.down(bindings.directions[i]) || pad.stick[i]),
        buttons: bindings.buttons.map(|b| pad.down(b)),
    }
}

fn pad_menu(pad: &PadState, buttons: &[PadButton; 5]) -> MenuKeys {
    let [confirm, back, start, tab_left, tab_right] = buttons.map(|b| pad.down(b));
    MenuKeys {
        confirm,
        back,
        start,
        tab_left,
        tab_right,
    }
}

/// One frame's inputs when `down` tells which keys are held, `pressed` is
/// the key first pressed this frame, and `pads` is the pads' frame. A pad
/// side's Options is also its cabinet Start and its Share its coin, so the
/// 1987 cabinet plays from the pads.
pub fn frame_inputs_from(
    table: &BindingsTable,
    devices: [Device; 2],
    pads: &PadFrame,
    down: impl Fn(Key) -> bool,
    pressed: Option<Key>,
) -> FrameInputs {
    let keys = [&table.p1, &table.p2];
    let pad_bindings = [&table.p1_pad, &table.p2_pad];
    let mut inputs = FrameInputs {
        menu: MenuKeys {
            confirm: down(table.menu.confirm),
            back: down(table.menu.back) || down(Key::Escape),
            start: down(table.menu.start),
            tab_left: down(table.menu.tab_left),
            tab_right: down(table.menu.tab_right),
        },
        cabinet: table.cabinet_inputs(&down),
        ..FrameInputs::default()
    };
    for side in 0..2 {
        let player = match source(devices[side], side, pads) {
            Source::Keyboard => {
                let i = keys[side].inputs(&down);
                PlayerInputs {
                    directions: i.directions,
                    buttons: i.buttons,
                }
            }
            Source::Pad(n) => {
                let pad = pads.pads[n].expect("a connected pad");
                inputs.pads[side] = Some(pad_menu(&pad, &table.menu_pad));
                inputs.cabinet[2 + side] |= pad.down(PadButton::Options);
                inputs.cabinet[side] |= pad.down(PadButton::Share);
                pad_player(&pad, pad_bindings[side])
            }
            Source::Idle => PlayerInputs::default(),
        };
        if side == 0 {
            inputs.p1 = player;
        } else {
            inputs.p2 = player;
        }
        inputs.pad_lost[side] = pad_number(devices[side], side, pads).is_some_and(|n| pads.lost[n]);
    }
    inputs.press = match (pressed.and_then(|k| key_to_name(k).ok()), pads.pressed) {
        (Some(key), _) => Some(Press::Key(key)),
        (None, Some(b)) => Some(Press::Pad(b.name())),
        (None, None) => None,
    };
    inputs
}

/// One frame's inputs from the game window's keyboard and the pads.
/// Nothing is held while the window is not focused, but a pad lost still
/// counts.
pub fn frame_inputs(keyboard: &mut Keyboard, table: &BindingsTable, devices: [Device; 2], pads: &PadFrame) -> FrameInputs {
    if !keyboard.has_focus() {
        return FrameInputs {
            pad_lost: std::array::from_fn(|side| pad_number(devices[side], side, pads).is_some_and(|n| pads.lost[n])),
            ..FrameInputs::default()
        };
    }
    let pressed = keyboard.take_pressed();
    frame_inputs_from(table, devices, pads, |k| keyboard.is_down(k), pressed)
}

/// The development launcher's pads. Once a frame, `PAD 1`'s inputs are
/// added to P1's keyboard inputs and `PAD 2`'s to P2's, through each
/// side's pad bindings. A pad's Options is its side's start and its Share
/// its coin, so every subcommand plays from the pads unchanged. Pads seen
/// or lost are printed.
pub struct LauncherPads {
    reader: crate::pads::PadReader,
    frame: PadFrame,
}

impl LauncherPads {
    pub fn open() -> Self {
        let mut reader = crate::pads::PadReader::open();
        for note in reader.take_notes() {
            println!("pads: {note}");
        }
        LauncherPads {
            reader,
            frame: PadFrame::default(),
        }
    }

    /// Read the pads for this frame; nothing is held while the window is
    /// not focused.
    pub fn poll(&mut self, focused: bool) {
        self.frame = self.reader.poll();
        for note in self.reader.take_notes() {
            println!("pads: {note}");
        }
        if !focused {
            self.frame.pads = [None; 2];
        }
    }

    /// `keyboard` with side `side`'s pad added.
    pub fn player(&self, side: usize, keyboard: crate::bindings::Inputs, bindings: &PadBindings) -> crate::bindings::Inputs {
        let Some(pad) = self.frame.pads[side] else {
            return keyboard;
        };
        let p = pad_player(&pad, bindings);
        crate::bindings::Inputs {
            directions: std::array::from_fn(|i| keyboard.directions[i] || p.directions[i]),
            buttons: std::array::from_fn(|i| keyboard.buttons[i] || p.buttons[i]),
        }
    }

    /// The cabinet switches with each pad's Share (coin) and Options
    /// (start) added.
    pub fn cabinet(&self, mut keyboard: [bool; 5]) -> [bool; 5] {
        for (side, pad) in self.frame.pads.iter().enumerate() {
            if let Some(pad) = pad {
                keyboard[side] |= pad.down(PadButton::Share);
                keyboard[2 + side] |= pad.down(PadButton::Options);
            }
        }
        keyboard
    }
}
