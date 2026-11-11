//! Remappable keyboard and pad bindings with TOML persistence (ticket #76,
//! F36; pads and menu inputs, ticket #159, FE9).
//!
//! Default path on Windows: `%APPDATA%\sf1-port\bindings.toml`.
//! Covers all 25 logical inputs of the cabinet (`Io`):
//! - P1 directions (`p1_right`, `p1_left`, `p1_down`, `p1_up`) and six buttons
//!   (`p1_button1`..=`p1_button6`);
//! - P2 directions (`p2_right`, `p2_left`, `p2_down`, `p2_up`) and six buttons
//!   (`p2_button1`..=`p2_button6`);
//! - Cabinet switches (`coin1`, `coin2`, `start1`, `start2`, `service1`).
//!
//! The front end adds the menu inputs' keys (`[menu]`: confirm, back,
//! start, tab left, tab right), each side's pad buttons (`[p1_pad]`,
//! `[p2_pad]`: the four directions and six buttons) and the pads' menu
//! buttons (`[menu_pad]`). A file without those sections takes their
//! defaults: Enter, Backspace, Space, Q and E; the Classic pad layout;
//! Cross, Circle, Options, L1 and R1.
//!
//! `Key::Escape` stays fixed to quit and is refused as a binding. Duplicate or
//! missing bindings are refused. When the file is missing, corrupt, partial, or
//! invalid, [`BindingsTable::load_or_default`] falls back to
//! [`DEFAULT_BINDINGS`] and returns a human-readable diagnostic message.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use minifb::{Key, Window};
use sf_engine::front::rebind::{Bind, Binds};
use sf_engine::tasks::Io;

use crate::pads::PadButton;

/// One player's logical inputs for a frame: directions (right, left, down,
/// up) and the six attack buttons (LP, MP, HP, LK, MK, HK).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Inputs {
    pub directions: [bool; 4],
    pub buttons: [bool; 6],
}

/// A player's key bindings, in the order of [`Inputs`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bindings {
    pub directions: [Key; 4],
    pub buttons: [Key; 6],
}

pub const P1_KEYS: Bindings = Bindings {
    directions: [Key::Right, Key::Left, Key::Down, Key::Up],
    buttons: [Key::A, Key::S, Key::D, Key::Z, Key::X, Key::C],
};

pub const P2_KEYS: Bindings = Bindings {
    directions: [Key::H, Key::F, Key::G, Key::T],
    buttons: [Key::U, Key::I, Key::O, Key::J, Key::K, Key::L],
};

impl Bindings {
    /// The player's inputs from the keys held: a key counts only while the
    /// window is focused.
    pub fn read(&self, window: &mut Window) -> Inputs {
        if !window.is_active() {
            return Inputs::default();
        }
        self.inputs(|k| window.is_key_down(k))
    }

    /// The player's inputs when `down` tells which keys are held.
    pub fn inputs(&self, down: impl Fn(Key) -> bool) -> Inputs {
        Inputs {
            directions: self.directions.map(&down),
            buttons: self.buttons.map(&down),
        }
    }
}

/// Cabinet-level switch bindings (`coin1`, `coin2`, `start1`, `start2`, `service1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CabinetBindings {
    pub coin1: Key,
    pub coin2: Key,
    pub start1: Key,
    pub start2: Key,
    pub service1: Key,
}

pub const DEFAULT_CABINET_KEYS: CabinetBindings = CabinetBindings {
    coin1: Key::Key5,
    coin2: Key::Key6,
    start1: Key::Key1,
    start2: Key::Key2,
    service1: Key::Key9,
};

/// The front end's menu keys (spec #150: Enter, Backspace, Q and E; Space
/// is the keyboard's Start). Esc is always back as well and is never bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuBindings {
    pub confirm: Key,
    pub back: Key,
    pub start: Key,
    pub tab_left: Key,
    pub tab_right: Key,
}

pub const DEFAULT_MENU_KEYS: MenuBindings = MenuBindings {
    confirm: Key::Enter,
    back: Key::Backspace,
    start: Key::Space,
    tab_left: Key::Q,
    tab_right: Key::E,
};

impl MenuBindings {
    /// Confirm, back, start, tab left, tab right.
    pub fn all(&self) -> [Key; 5] {
        [self.confirm, self.back, self.start, self.tab_left, self.tab_right]
    }

    fn from_all(keys: [Key; 5]) -> Self {
        let [confirm, back, start, tab_left, tab_right] = keys;
        MenuBindings { confirm, back, start, tab_left, tab_right }
    }
}

/// A side's pad buttons, in the order of [`Inputs`]. The left stick moves
/// as well, whatever the directions are bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadBindings {
    pub directions: [PadButton; 4],
    pub buttons: [PadButton; 6],
}

/// The Classic scheme on a DualShock 4: the d-pad; Square, Triangle and R1
/// for LP, MP and HP; Cross, Circle and R2 for LK, MK and HK.
pub const CLASSIC_PAD: PadBindings = PadBindings {
    directions: [PadButton::DpadRight, PadButton::DpadLeft, PadButton::DpadDown, PadButton::DpadUp],
    buttons: [
        PadButton::Square,
        PadButton::Triangle,
        PadButton::R1,
        PadButton::Cross,
        PadButton::Circle,
        PadButton::R2,
    ],
};

impl PadBindings {
    fn all(&self) -> [PadButton; 10] {
        let mut all = [PadButton::Cross; 10];
        all[..4].copy_from_slice(&self.directions);
        all[4..].copy_from_slice(&self.buttons);
        all
    }

    fn from_all(all: [PadButton; 10]) -> Self {
        let mut p = CLASSIC_PAD;
        p.directions.copy_from_slice(&all[..4]);
        p.buttons.copy_from_slice(&all[4..]);
        p
    }
}

/// The pads' menu buttons: confirm, back, start, tab left, tab right.
pub const DEFAULT_MENU_PAD: [PadButton; 5] =
    [PadButton::Cross, PadButton::Circle, PadButton::Options, PadButton::L1, PadButton::R1];

/// The names of the menu sections' and pad sections' entries.
const MENU_NAMES: [&str; 5] = ["confirm", "back", "start", "tab_left", "tab_right"];
const PAD_NAMES: [&str; 10] = [
    "right", "left", "down", "up", "button1", "button2", "button3", "button4", "button5", "button6",
];

/// The complete table: the 25 cabinet inputs' keys (P1, P2, the cabinet
/// switches), the menu keys, each side's pad buttons and the pads' menu
/// buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindingsTable {
    pub p1: Bindings,
    pub p2: Bindings,
    pub cabinet: CabinetBindings,
    pub menu: MenuBindings,
    pub p1_pad: PadBindings,
    pub p2_pad: PadBindings,
    pub menu_pad: [PadButton; 5],
}

pub const DEFAULT_BINDINGS: BindingsTable = BindingsTable {
    p1: P1_KEYS,
    p2: P2_KEYS,
    cabinet: DEFAULT_CABINET_KEYS,
    menu: DEFAULT_MENU_KEYS,
    p1_pad: CLASSIC_PAD,
    p2_pad: CLASSIC_PAD,
    menu_pad: DEFAULT_MENU_PAD,
};

impl Default for BindingsTable {
    fn default() -> Self {
        DEFAULT_BINDINGS
    }
}

/// Every logical input in the 25-input table, in canonical order matching
/// `sf_engine::run::INPUT_NAMES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogicalInput {
    P1Right,
    P1Left,
    P1Down,
    P1Up,
    P1Button1,
    P1Button2,
    P1Button3,
    P1Button4,
    P1Button5,
    P1Button6,
    P2Right,
    P2Left,
    P2Down,
    P2Up,
    P2Button1,
    P2Button2,
    P2Button3,
    P2Button4,
    P2Button5,
    P2Button6,
    Coin1,
    Coin2,
    Start1,
    Start2,
    Service1,
}

impl LogicalInput {
    pub const ALL: [LogicalInput; 25] = [
        LogicalInput::P1Right,
        LogicalInput::P1Left,
        LogicalInput::P1Down,
        LogicalInput::P1Up,
        LogicalInput::P1Button1,
        LogicalInput::P1Button2,
        LogicalInput::P1Button3,
        LogicalInput::P1Button4,
        LogicalInput::P1Button5,
        LogicalInput::P1Button6,
        LogicalInput::P2Right,
        LogicalInput::P2Left,
        LogicalInput::P2Down,
        LogicalInput::P2Up,
        LogicalInput::P2Button1,
        LogicalInput::P2Button2,
        LogicalInput::P2Button3,
        LogicalInput::P2Button4,
        LogicalInput::P2Button5,
        LogicalInput::P2Button6,
        LogicalInput::Coin1,
        LogicalInput::Coin2,
        LogicalInput::Start1,
        LogicalInput::Start2,
        LogicalInput::Service1,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            LogicalInput::P1Right => "p1_right",
            LogicalInput::P1Left => "p1_left",
            LogicalInput::P1Down => "p1_down",
            LogicalInput::P1Up => "p1_up",
            LogicalInput::P1Button1 => "p1_button1",
            LogicalInput::P1Button2 => "p1_button2",
            LogicalInput::P1Button3 => "p1_button3",
            LogicalInput::P1Button4 => "p1_button4",
            LogicalInput::P1Button5 => "p1_button5",
            LogicalInput::P1Button6 => "p1_button6",
            LogicalInput::P2Right => "p2_right",
            LogicalInput::P2Left => "p2_left",
            LogicalInput::P2Down => "p2_down",
            LogicalInput::P2Up => "p2_up",
            LogicalInput::P2Button1 => "p2_button1",
            LogicalInput::P2Button2 => "p2_button2",
            LogicalInput::P2Button3 => "p2_button3",
            LogicalInput::P2Button4 => "p2_button4",
            LogicalInput::P2Button5 => "p2_button5",
            LogicalInput::P2Button6 => "p2_button6",
            LogicalInput::Coin1 => "coin1",
            LogicalInput::Coin2 => "coin2",
            LogicalInput::Start1 => "start1",
            LogicalInput::Start2 => "start2",
            LogicalInput::Service1 => "service1",
        }
    }

    pub fn from_name(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        for input in Self::ALL {
            if input.name() == trimmed {
                return Ok(input);
            }
        }
        Err(format!("unknown logical input '{trimmed}'"))
    }
}

const KEY_NAMES: &[(Key, &str)] = &[
    (Key::Key0, "Key0"),
    (Key::Key1, "Key1"),
    (Key::Key2, "Key2"),
    (Key::Key3, "Key3"),
    (Key::Key4, "Key4"),
    (Key::Key5, "Key5"),
    (Key::Key6, "Key6"),
    (Key::Key7, "Key7"),
    (Key::Key8, "Key8"),
    (Key::Key9, "Key9"),
    (Key::A, "A"),
    (Key::B, "B"),
    (Key::C, "C"),
    (Key::D, "D"),
    (Key::E, "E"),
    (Key::F, "F"),
    (Key::G, "G"),
    (Key::H, "H"),
    (Key::I, "I"),
    (Key::J, "J"),
    (Key::K, "K"),
    (Key::L, "L"),
    (Key::M, "M"),
    (Key::N, "N"),
    (Key::O, "O"),
    (Key::P, "P"),
    (Key::Q, "Q"),
    (Key::R, "R"),
    (Key::S, "S"),
    (Key::T, "T"),
    (Key::U, "U"),
    (Key::V, "V"),
    (Key::W, "W"),
    (Key::X, "X"),
    (Key::Y, "Y"),
    (Key::Z, "Z"),
    (Key::F1, "F1"),
    (Key::F2, "F2"),
    (Key::F3, "F3"),
    (Key::F4, "F4"),
    (Key::F5, "F5"),
    (Key::F6, "F6"),
    (Key::F7, "F7"),
    (Key::F8, "F8"),
    (Key::F9, "F9"),
    (Key::F10, "F10"),
    (Key::F11, "F11"),
    (Key::F12, "F12"),
    (Key::F13, "F13"),
    (Key::F14, "F14"),
    (Key::F15, "F15"),
    (Key::Down, "Down"),
    (Key::Left, "Left"),
    (Key::Right, "Right"),
    (Key::Up, "Up"),
    (Key::Apostrophe, "Apostrophe"),
    (Key::Backquote, "Backquote"),
    (Key::Backslash, "Backslash"),
    (Key::Comma, "Comma"),
    (Key::Equal, "Equal"),
    (Key::LeftBracket, "LeftBracket"),
    (Key::Minus, "Minus"),
    (Key::Period, "Period"),
    (Key::RightBracket, "RightBracket"),
    (Key::Semicolon, "Semicolon"),
    (Key::Slash, "Slash"),
    (Key::Backspace, "Backspace"),
    (Key::Delete, "Delete"),
    (Key::End, "End"),
    (Key::Enter, "Enter"),
    (Key::Escape, "Escape"),
    (Key::Home, "Home"),
    (Key::Insert, "Insert"),
    (Key::Menu, "Menu"),
    (Key::PageDown, "PageDown"),
    (Key::PageUp, "PageUp"),
    (Key::Pause, "Pause"),
    (Key::Space, "Space"),
    (Key::Tab, "Tab"),
    (Key::NumLock, "NumLock"),
    (Key::CapsLock, "CapsLock"),
    (Key::ScrollLock, "ScrollLock"),
    (Key::LeftShift, "LeftShift"),
    (Key::RightShift, "RightShift"),
    (Key::LeftCtrl, "LeftCtrl"),
    (Key::RightCtrl, "RightCtrl"),
    (Key::NumPad0, "NumPad0"),
    (Key::NumPad1, "NumPad1"),
    (Key::NumPad2, "NumPad2"),
    (Key::NumPad3, "NumPad3"),
    (Key::NumPad4, "NumPad4"),
    (Key::NumPad5, "NumPad5"),
    (Key::NumPad6, "NumPad6"),
    (Key::NumPad7, "NumPad7"),
    (Key::NumPad8, "NumPad8"),
    (Key::NumPad9, "NumPad9"),
    (Key::NumPadDot, "NumPadDot"),
    (Key::NumPadSlash, "NumPadSlash"),
    (Key::NumPadAsterisk, "NumPadAsterisk"),
    (Key::NumPadMinus, "NumPadMinus"),
    (Key::NumPadPlus, "NumPadPlus"),
    (Key::NumPadEnter, "NumPadEnter"),
    (Key::LeftAlt, "LeftAlt"),
    (Key::RightAlt, "RightAlt"),
    (Key::LeftSuper, "LeftSuper"),
    (Key::RightSuper, "RightSuper"),
];

/// Every key name a bindings file may use.
pub fn key_names() -> impl Iterator<Item = &'static str> {
    KEY_NAMES.iter().map(|&(_, name)| name)
}

pub fn key_to_name(key: Key) -> Result<&'static str, String> {
    for &(k, name) in KEY_NAMES {
        if k == key {
            return Ok(name);
        }
    }
    Err(format!("unsupported key {key:?}"))
}

pub fn name_to_key(name: &str) -> Result<Key, String> {
    let trimmed = name.trim();
    for &(k, n) in KEY_NAMES {
        if n == trimmed {
            return Ok(k);
        }
    }
    Err(format!("unknown key name '{trimmed}'"))
}

/// Default Windows path `%APPDATA%\sf1-port\bindings.toml` (Decision 9).
pub fn default_bindings_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("sf1-port").join("bindings.toml")
}

impl BindingsTable {
    /// Get the key bound to `input`.
    pub fn key(&self, input: LogicalInput) -> Key {
        match input {
            LogicalInput::P1Right => self.p1.directions[0],
            LogicalInput::P1Left => self.p1.directions[1],
            LogicalInput::P1Down => self.p1.directions[2],
            LogicalInput::P1Up => self.p1.directions[3],
            LogicalInput::P1Button1 => self.p1.buttons[0],
            LogicalInput::P1Button2 => self.p1.buttons[1],
            LogicalInput::P1Button3 => self.p1.buttons[2],
            LogicalInput::P1Button4 => self.p1.buttons[3],
            LogicalInput::P1Button5 => self.p1.buttons[4],
            LogicalInput::P1Button6 => self.p1.buttons[5],
            LogicalInput::P2Right => self.p2.directions[0],
            LogicalInput::P2Left => self.p2.directions[1],
            LogicalInput::P2Down => self.p2.directions[2],
            LogicalInput::P2Up => self.p2.directions[3],
            LogicalInput::P2Button1 => self.p2.buttons[0],
            LogicalInput::P2Button2 => self.p2.buttons[1],
            LogicalInput::P2Button3 => self.p2.buttons[2],
            LogicalInput::P2Button4 => self.p2.buttons[3],
            LogicalInput::P2Button5 => self.p2.buttons[4],
            LogicalInput::P2Button6 => self.p2.buttons[5],
            LogicalInput::Coin1 => self.cabinet.coin1,
            LogicalInput::Coin2 => self.cabinet.coin2,
            LogicalInput::Start1 => self.cabinet.start1,
            LogicalInput::Start2 => self.cabinet.start2,
            LogicalInput::Service1 => self.cabinet.service1,
        }
    }

    fn set_key_unchecked(&mut self, input: LogicalInput, key: Key) {
        match input {
            LogicalInput::P1Right => self.p1.directions[0] = key,
            LogicalInput::P1Left => self.p1.directions[1] = key,
            LogicalInput::P1Down => self.p1.directions[2] = key,
            LogicalInput::P1Up => self.p1.directions[3] = key,
            LogicalInput::P1Button1 => self.p1.buttons[0] = key,
            LogicalInput::P1Button2 => self.p1.buttons[1] = key,
            LogicalInput::P1Button3 => self.p1.buttons[2] = key,
            LogicalInput::P1Button4 => self.p1.buttons[3] = key,
            LogicalInput::P1Button5 => self.p1.buttons[4] = key,
            LogicalInput::P1Button6 => self.p1.buttons[5] = key,
            LogicalInput::P2Right => self.p2.directions[0] = key,
            LogicalInput::P2Left => self.p2.directions[1] = key,
            LogicalInput::P2Down => self.p2.directions[2] = key,
            LogicalInput::P2Up => self.p2.directions[3] = key,
            LogicalInput::P2Button1 => self.p2.buttons[0] = key,
            LogicalInput::P2Button2 => self.p2.buttons[1] = key,
            LogicalInput::P2Button3 => self.p2.buttons[2] = key,
            LogicalInput::P2Button4 => self.p2.buttons[3] = key,
            LogicalInput::P2Button5 => self.p2.buttons[4] = key,
            LogicalInput::P2Button6 => self.p2.buttons[5] = key,
            LogicalInput::Coin1 => self.cabinet.coin1 = key,
            LogicalInput::Coin2 => self.cabinet.coin2 = key,
            LogicalInput::Start1 => self.cabinet.start1 = key,
            LogicalInput::Start2 => self.cabinet.start2 = key,
            LogicalInput::Service1 => self.cabinet.service1 = key,
        }
    }

    /// All 25 `(LogicalInput, Key)` pairs in canonical order.
    pub fn entries(&self) -> [(LogicalInput, Key); 25] {
        LogicalInput::ALL.map(|input| (input, self.key(input)))
    }

    /// Validate that no input is bound to `Key::Escape` (`Esc` stays fixed to
    /// quit) or `Key::Unknown`, that all 25 logical inputs have distinct keys,
    /// that the five menu keys are distinct, and that no pad list (P1's, P2's,
    /// the menu's) binds one button twice.
    pub fn validate(&self) -> Result<(), String> {
        let mut menu: BTreeMap<u32, &str> = BTreeMap::new();
        for (name, key) in MENU_NAMES.iter().zip(self.menu.all()) {
            if key == Key::Escape {
                return Err(format!("menu '{name}' cannot bind Escape (Esc is always back)"));
            }
            let key_name = key_to_name(key)?;
            if let Some(prev) = menu.insert(key as u32, name) {
                return Err(format!("duplicate menu key '{key_name}' bound to both '{prev}' and '{name}'"));
            }
        }
        for (section, list) in [
            ("p1_pad", &self.p1_pad.all()[..]),
            ("p2_pad", &self.p2_pad.all()[..]),
            ("menu_pad", &self.menu_pad[..]),
        ] {
            for (i, b) in list.iter().enumerate() {
                if list[..i].contains(b) {
                    return Err(format!("duplicate pad button '{}' in [{section}]", b.name()));
                }
            }
        }
        let mut seen: BTreeMap<u32, LogicalInput> = BTreeMap::new();
        for (input, key) in self.entries() {
            if key == Key::Escape {
                return Err(format!(
                    "'{}' cannot bind Escape (Esc is reserved to quit)",
                    input.name()
                ));
            }
            let key_name = key_to_name(key)?;
            if let Some(prev) = seen.insert(key as u32, input) {
                return Err(format!(
                    "duplicate key '{key_name}' bound to both '{}' and '{}'",
                    prev.name(),
                    input.name()
                ));
            }
        }
        Ok(())
    }

    /// Rebind one logical input to `key`, refusing `Key::Escape` or any key
    /// already bound to another logical input. If validation fails, `self` is
    /// left unchanged.
    pub fn rebind(&mut self, input: LogicalInput, key: Key) -> Result<(), String> {
        let prev = *self;
        self.set_key_unchecked(input, key);
        if let Err(e) = self.validate() {
            *self = prev;
            return Err(e);
        }
        Ok(())
    }

    /// Rebind one logical input and save the updated table to `path`.
    pub fn rebind_and_save(
        &mut self,
        path: &Path,
        input: LogicalInput,
        key: Key,
    ) -> Result<(), String> {
        self.rebind(input, key)?;
        self.save(path)
    }

    /// Serialize to canonical `bindings.toml` text.
    pub fn to_toml(&self) -> String {
        let mut out = String::from("# sf1-port keyboard and pad bindings (Esc is fixed to quit)\n");
        for (input, key) in self.entries() {
            let name = key_to_name(key).expect("valid key in BindingsTable");
            out.push_str(&format!("{} = \"{name}\"\n", input.name()));
        }
        out.push_str("\n[menu]\n");
        for (name, key) in MENU_NAMES.iter().zip(self.menu.all()) {
            out.push_str(&format!("{name} = \"{}\"\n", key_to_name(key).expect("valid key")));
        }
        for (section, pad) in [("p1_pad", &self.p1_pad), ("p2_pad", &self.p2_pad)] {
            out.push_str(&format!("\n[{section}]\n"));
            for (name, b) in PAD_NAMES.iter().zip(pad.all()) {
                out.push_str(&format!("{name} = \"{}\"\n", b.name()));
            }
        }
        out.push_str("\n[menu_pad]\n");
        for (name, b) in MENU_NAMES.iter().zip(self.menu_pad) {
            out.push_str(&format!("{name} = \"{}\"\n", b.name()));
        }
        out
    }

    /// Every rebindable input by name, for the rebind walk.
    pub fn binds(&self) -> Binds {
        let name = |k: Key| key_to_name(k).unwrap_or("?");
        let player = |keys: &Bindings, pad: &PadBindings| -> [Bind; 10] {
            let keys: Vec<Key> = keys.directions.iter().chain(&keys.buttons).copied().collect();
            let pads = pad.all();
            std::array::from_fn(|i| Bind { key: name(keys[i]), pad: pads[i].name() })
        };
        let menu = self.menu.all();
        let c = &self.cabinet;
        Binds {
            p1: player(&self.p1, &self.p1_pad),
            p2: player(&self.p2, &self.p2_pad),
            menu: std::array::from_fn(|i| Bind { key: name(menu[i]), pad: self.menu_pad[i].name() }),
            cabinet: [c.coin1, c.coin2, c.start1, c.start2, c.service1].map(name),
        }
    }

    /// This table with the walk's `binds`, validated.
    pub fn with_binds(&self, binds: &Binds) -> Result<Self, String> {
        let key = |n: &str| name_to_key(n);
        let pad = |n: &str| PadButton::from_name(n).ok_or_else(|| format!("unknown pad button '{n}'"));
        let player = |list: &[Bind; 10]| -> Result<(Bindings, PadBindings), String> {
            let mut keys = [Key::Unknown; 10];
            let mut pads = [PadButton::Cross; 10];
            for (i, b) in list.iter().enumerate() {
                keys[i] = key(b.key)?;
                pads[i] = pad(b.pad)?;
            }
            let mut k = P1_KEYS;
            k.directions.copy_from_slice(&keys[..4]);
            k.buttons.copy_from_slice(&keys[4..]);
            Ok((k, PadBindings::from_all(pads)))
        };
        let mut t = *self;
        (t.p1, t.p1_pad) = player(&binds.p1)?;
        (t.p2, t.p2_pad) = player(&binds.p2)?;
        let mut menu = [Key::Unknown; 5];
        for (i, b) in binds.menu.iter().enumerate() {
            menu[i] = key(b.key)?;
            t.menu_pad[i] = pad(b.pad)?;
        }
        t.menu = MenuBindings::from_all(menu);
        t.validate()?;
        Ok(t)
    }

    /// Parse and validate a `bindings.toml` string. Refuses corrupt syntax,
    /// unknown keys/inputs, missing inputs, duplicate inputs, duplicate keys,
    /// and `Escape`.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let mut assigned: BTreeMap<LogicalInput, Key> = BTreeMap::new();
        let mut section: Option<String> = None;
        let mut table = Self::default();
        let mut menu = table.menu.all();
        let mut pads = [table.p1_pad.all(), table.p2_pad.all()];

        for (line_no, raw_line) in text.lines().enumerate() {
            let line_num = line_no + 1;
            let without_comment = match raw_line.find('#') {
                Some(idx) => &raw_line[..idx],
                None => raw_line,
            };
            let line = without_comment.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                let sec = line[1..line.len() - 1].trim();
                if !matches!(sec, "p1" | "p2" | "cabinet" | "menu" | "p1_pad" | "p2_pad" | "menu_pad") {
                    return Err(format!("line {line_num}: unknown section '[{sec}]'"));
                }
                section = Some(sec.to_string());
                continue;
            }
            let Some((raw_lhs, raw_rhs)) = line.split_once('=') else {
                return Err(format!(
                    "line {line_num}: expected 'input = \"Key\"', got '{line}'"
                ));
            };
            let lhs = raw_lhs.trim();
            let rhs = raw_rhs.trim();
            let key_str = if (rhs.starts_with('"') && rhs.ends_with('"') && rhs.len() >= 2)
                || (rhs.starts_with('\'') && rhs.ends_with('\'') && rhs.len() >= 2)
            {
                &rhs[1..rhs.len() - 1]
            } else {
                return Err(format!(
                    "line {line_num}: expected quoted key string for '{lhs}', got '{rhs}'"
                ));
            };
            let entry = |names: &[&str]| {
                names
                    .iter()
                    .position(|&n| n == lhs)
                    .ok_or_else(|| format!("line {line_num}: unknown entry '{lhs}'"))
            };
            let pad = |s: &str| {
                PadButton::from_name(s).ok_or_else(|| format!("line {line_num}: unknown pad button '{s}'"))
            };
            match section.as_deref() {
                Some("menu") => {
                    menu[entry(&MENU_NAMES)?] = name_to_key(key_str).map_err(|e| format!("line {line_num}: {e}"))?;
                    continue;
                }
                Some(s @ ("p1_pad" | "p2_pad")) => {
                    pads[usize::from(s == "p2_pad")][entry(&PAD_NAMES)?] = pad(key_str)?;
                    continue;
                }
                Some("menu_pad") => {
                    table.menu_pad[entry(&MENU_NAMES)?] = pad(key_str)?;
                    continue;
                }
                _ => {}
            }
            let canonical_name = match section.as_deref() {
                Some("p1") if !lhs.starts_with("p1_") => format!("p1_{lhs}"),
                Some("p2") if !lhs.starts_with("p2_") => format!("p2_{lhs}"),
                _ => lhs.to_string(),
            };
            let input = LogicalInput::from_name(&canonical_name)
                .map_err(|e| format!("line {line_num}: {e}"))?;
            let key = name_to_key(key_str).map_err(|e| format!("line {line_num}: {e}"))?;
            if assigned.insert(input, key).is_some() {
                return Err(format!(
                    "line {line_num}: duplicate assignment for '{}'",
                    input.name()
                ));
            }
        }

        let missing: Vec<&str> = LogicalInput::ALL
            .into_iter()
            .filter(|i| !assigned.contains_key(i))
            .map(LogicalInput::name)
            .collect();
        if !missing.is_empty() {
            return Err(format!("missing bindings for: {}", missing.join(", ")));
        }

        for (input, key) in assigned {
            table.set_key_unchecked(input, key);
        }
        table.menu = MenuBindings::from_all(menu);
        table.p1_pad = PadBindings::from_all(pads[0]);
        table.p2_pad = PadBindings::from_all(pads[1]);
        table.validate()?;
        Ok(table)
    }

    /// Load and validate `path`.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::from_toml(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Load `path`, or fall back to [`DEFAULT_BINDINGS`] with a diagnostic
    /// message when `path` is missing, corrupt, partial, or invalid.
    pub fn load_or_default(path: &Path) -> (Self, Option<String>) {
        match Self::load(path) {
            Ok(table) => (table, None),
            Err(reason) => (
                Self::default(),
                Some(format!(
                    "{reason}; falling back to default keyboard bindings"
                )),
            ),
        }
    }

    /// Save this table to `path`, creating parent directories if needed.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
            }
        }
        std::fs::write(path, self.to_toml())
            .map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    /// Sample the five cabinet switches (`coin1`, `coin2`, `start1`, `start2`,
    /// `service1`) from `down`.
    pub fn cabinet_inputs(&self, down: impl Fn(Key) -> bool) -> [bool; 5] {
        [
            down(self.cabinet.coin1),
            down(self.cabinet.coin2),
            down(self.cabinet.start1),
            down(self.cabinet.start2),
            down(self.cabinet.service1),
        ]
    }

    /// Sample the five cabinet switches from `window` (only while focused).
    pub fn read_cabinet(&self, window: &mut Window) -> [bool; 5] {
        if !window.is_active() {
            return [false; 5];
        }
        self.cabinet_inputs(|k| window.is_key_down(k))
    }

    /// Build the full hardware [`Io`] (`in0`, `in1`, `system`, `dsw1`, `dsw2`)
    /// from the keys reported held by `down`, using the exact active-low bit
    /// positions of `sf-mame0289.cpp`.
    pub fn io(&self, dsw1: u16, dsw2: u16, down: impl Fn(Key) -> bool) -> Io {
        let p1 = self.p1.inputs(&down);
        let p2 = self.p2.inputs(&down);
        let [coin1, coin2, start1, start2, service1] = self.cabinet_inputs(&down);

        let mut in1: u16 = 0xffff;
        for (bit, held) in p1.directions.into_iter().enumerate() {
            if held {
                in1 &= !(1 << bit);
            }
        }
        for (bit, b) in [(4, 0), (5, 1), (6, 3), (7, 4)] {
            if p1.buttons[b] {
                in1 &= !(1 << bit);
            }
        }
        for (bit, held) in p2.directions.into_iter().enumerate() {
            if held {
                in1 &= !(1 << (8 + bit));
            }
        }
        for (bit, b) in [(12, 0), (13, 1), (14, 3), (15, 4)] {
            if p2.buttons[b] {
                in1 &= !(1 << bit);
            }
        }

        let mut in0: u16 = 0xffff;
        if coin1 {
            in0 &= !(1 << 0);
        }
        if coin2 {
            in0 &= !(1 << 1);
        }
        if p1.buttons[5] {
            in0 &= !(1 << 2);
        }
        if p2.buttons[5] {
            in0 &= !(1 << 8);
        }
        if p1.buttons[2] {
            in0 &= !(1 << 9);
        }
        if p2.buttons[2] {
            in0 &= !(1 << 10);
        }

        let mut system: u16 = 0xffff;
        if start1 {
            system &= !(1 << 0);
        }
        if start2 {
            system &= !(1 << 1);
        }
        if service1 {
            system &= !(1 << 2);
        }

        Io {
            in0,
            in1,
            system,
            dsw1,
            dsw2,
            ..Io::idle()
        }
    }

    /// Compact single-line summary of the active binding table for the smoke
    /// check and CLI output.
    pub fn summary(&self) -> String {
        let fmt_keys = |keys: &[Key]| -> String {
            keys.iter()
                .map(|&k| key_to_name(k).unwrap_or("?"))
                .collect::<Vec<_>>()
                .join("/")
        };
        format!(
            "P1 dir={} btn={}, P2 dir={} btn={}, cabinet coin1={} coin2={} start1={} start2={} service1={}",
            fmt_keys(&self.p1.directions),
            fmt_keys(&self.p1.buttons),
            fmt_keys(&self.p2.directions),
            fmt_keys(&self.p2.buttons),
            key_to_name(self.cabinet.coin1).unwrap_or("?"),
            key_to_name(self.cabinet.coin2).unwrap_or("?"),
            key_to_name(self.cabinet.start1).unwrap_or("?"),
            key_to_name(self.cabinet.start2).unwrap_or("?"),
            key_to_name(self.cabinet.service1).unwrap_or("?"),
        )
    }

    /// Convert a frame's 25-element logical input state (ordered as
    /// `sf_engine::run::INPUT_NAMES`) into the set of bound physical [`Key`]s
    /// held on that frame, then decode `(p1, p2, cabinet)` back through
    /// [`Bindings::inputs`] and [`Self::cabinet_inputs`].
    pub fn round_trip_logical_frame(&self, held: &[bool; 25]) -> (Inputs, Inputs, [bool; 5]) {
        let mut down_keys = BTreeSet::new();
        for (idx, input) in LogicalInput::ALL.into_iter().enumerate() {
            if held[idx] {
                down_keys.insert(self.key(input) as u32);
            }
        }
        let down = |k: Key| down_keys.contains(&(k as u32));
        (
            self.p1.inputs(down),
            self.p2.inputs(down),
            self.cabinet_inputs(down),
        )
    }
}
