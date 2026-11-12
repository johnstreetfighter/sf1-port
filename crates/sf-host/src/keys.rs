//! The GPU window's keyboard (ticket #205, VO2; ADR-0010). winit's
//! physical key codes are translated into the host's key vocabulary,
//! minifb's `Key`, which `bindings.toml` names, so every binding name keeps
//! meaning the same key. minifb reads keys by scan code on Windows, and
//! winit's key codes are scan-code positions too, so `A` is the key in the
//! A position on any layout, as before. The keyboard also carries the
//! window's focus: unfocused, nothing is held and key events are ignored.
//! It also keeps the text typed, for the front end's text entry.

use minifb::Key;
use sf_engine::front::TypedKey;
use winit::keyboard::{Key as LogicalKey, KeyCode, NamedKey, PhysicalKey};

/// The most typed keys kept between game frames.
const TYPED_MAX: usize = 32;

/// The typed keys a key press gives: an editing key by its logical key
/// (Enter and Esc not on a repeat), else each character of its text.
pub fn typed_keys(logical: &LogicalKey, text: Option<&str>, repeat: bool) -> Vec<TypedKey> {
    let named = match logical {
        LogicalKey::Named(NamedKey::Backspace) => Some(TypedKey::Erase),
        LogicalKey::Named(NamedKey::Delete) => Some(TypedKey::EraseAhead),
        LogicalKey::Named(NamedKey::ArrowLeft) => Some(TypedKey::Left),
        LogicalKey::Named(NamedKey::ArrowRight) => Some(TypedKey::Right),
        LogicalKey::Named(NamedKey::ArrowUp) => Some(TypedKey::Up),
        LogicalKey::Named(NamedKey::ArrowDown) => Some(TypedKey::Down),
        LogicalKey::Named(NamedKey::Home) => Some(TypedKey::Home),
        LogicalKey::Named(NamedKey::End) => Some(TypedKey::End),
        LogicalKey::Named(NamedKey::Enter) => return if repeat { Vec::new() } else { vec![TypedKey::Enter] },
        LogicalKey::Named(NamedKey::Escape) => return if repeat { Vec::new() } else { vec![TypedKey::Escape] },
        _ => None,
    };
    match named {
        Some(key) => vec![key],
        None => text.unwrap_or("").chars().filter(|c| !c.is_control()).map(TypedKey::Char).collect(),
    }
}

/// Every winit key code with a binding name, and the key it is.
pub const CODES: &[(KeyCode, Key)] = &[
    (KeyCode::Digit0, Key::Key0),
    (KeyCode::Digit1, Key::Key1),
    (KeyCode::Digit2, Key::Key2),
    (KeyCode::Digit3, Key::Key3),
    (KeyCode::Digit4, Key::Key4),
    (KeyCode::Digit5, Key::Key5),
    (KeyCode::Digit6, Key::Key6),
    (KeyCode::Digit7, Key::Key7),
    (KeyCode::Digit8, Key::Key8),
    (KeyCode::Digit9, Key::Key9),
    (KeyCode::KeyA, Key::A),
    (KeyCode::KeyB, Key::B),
    (KeyCode::KeyC, Key::C),
    (KeyCode::KeyD, Key::D),
    (KeyCode::KeyE, Key::E),
    (KeyCode::KeyF, Key::F),
    (KeyCode::KeyG, Key::G),
    (KeyCode::KeyH, Key::H),
    (KeyCode::KeyI, Key::I),
    (KeyCode::KeyJ, Key::J),
    (KeyCode::KeyK, Key::K),
    (KeyCode::KeyL, Key::L),
    (KeyCode::KeyM, Key::M),
    (KeyCode::KeyN, Key::N),
    (KeyCode::KeyO, Key::O),
    (KeyCode::KeyP, Key::P),
    (KeyCode::KeyQ, Key::Q),
    (KeyCode::KeyR, Key::R),
    (KeyCode::KeyS, Key::S),
    (KeyCode::KeyT, Key::T),
    (KeyCode::KeyU, Key::U),
    (KeyCode::KeyV, Key::V),
    (KeyCode::KeyW, Key::W),
    (KeyCode::KeyX, Key::X),
    (KeyCode::KeyY, Key::Y),
    (KeyCode::KeyZ, Key::Z),
    (KeyCode::F1, Key::F1),
    (KeyCode::F2, Key::F2),
    (KeyCode::F3, Key::F3),
    (KeyCode::F4, Key::F4),
    (KeyCode::F5, Key::F5),
    (KeyCode::F6, Key::F6),
    (KeyCode::F7, Key::F7),
    (KeyCode::F8, Key::F8),
    (KeyCode::F9, Key::F9),
    (KeyCode::F10, Key::F10),
    (KeyCode::F11, Key::F11),
    (KeyCode::F12, Key::F12),
    (KeyCode::F13, Key::F13),
    (KeyCode::F14, Key::F14),
    (KeyCode::F15, Key::F15),
    (KeyCode::ArrowDown, Key::Down),
    (KeyCode::ArrowLeft, Key::Left),
    (KeyCode::ArrowRight, Key::Right),
    (KeyCode::ArrowUp, Key::Up),
    (KeyCode::Quote, Key::Apostrophe),
    (KeyCode::Backquote, Key::Backquote),
    (KeyCode::Backslash, Key::Backslash),
    (KeyCode::Comma, Key::Comma),
    (KeyCode::Equal, Key::Equal),
    (KeyCode::BracketLeft, Key::LeftBracket),
    (KeyCode::Minus, Key::Minus),
    (KeyCode::Period, Key::Period),
    (KeyCode::BracketRight, Key::RightBracket),
    (KeyCode::Semicolon, Key::Semicolon),
    (KeyCode::Slash, Key::Slash),
    (KeyCode::Backspace, Key::Backspace),
    (KeyCode::Delete, Key::Delete),
    (KeyCode::End, Key::End),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::Escape, Key::Escape),
    (KeyCode::Home, Key::Home),
    (KeyCode::Insert, Key::Insert),
    (KeyCode::ContextMenu, Key::Menu),
    (KeyCode::PageDown, Key::PageDown),
    (KeyCode::PageUp, Key::PageUp),
    (KeyCode::Pause, Key::Pause),
    (KeyCode::Space, Key::Space),
    (KeyCode::Tab, Key::Tab),
    (KeyCode::NumLock, Key::NumLock),
    (KeyCode::CapsLock, Key::CapsLock),
    (KeyCode::ScrollLock, Key::ScrollLock),
    (KeyCode::ShiftLeft, Key::LeftShift),
    (KeyCode::ShiftRight, Key::RightShift),
    (KeyCode::ControlLeft, Key::LeftCtrl),
    (KeyCode::ControlRight, Key::RightCtrl),
    (KeyCode::Numpad0, Key::NumPad0),
    (KeyCode::Numpad1, Key::NumPad1),
    (KeyCode::Numpad2, Key::NumPad2),
    (KeyCode::Numpad3, Key::NumPad3),
    (KeyCode::Numpad4, Key::NumPad4),
    (KeyCode::Numpad5, Key::NumPad5),
    (KeyCode::Numpad6, Key::NumPad6),
    (KeyCode::Numpad7, Key::NumPad7),
    (KeyCode::Numpad8, Key::NumPad8),
    (KeyCode::Numpad9, Key::NumPad9),
    (KeyCode::NumpadDecimal, Key::NumPadDot),
    (KeyCode::NumpadDivide, Key::NumPadSlash),
    (KeyCode::NumpadMultiply, Key::NumPadAsterisk),
    (KeyCode::NumpadSubtract, Key::NumPadMinus),
    (KeyCode::NumpadAdd, Key::NumPadPlus),
    (KeyCode::NumpadEnter, Key::NumPadEnter),
    (KeyCode::AltLeft, Key::LeftAlt),
    (KeyCode::AltRight, Key::RightAlt),
    (KeyCode::SuperLeft, Key::LeftSuper),
    (KeyCode::SuperRight, Key::RightSuper),
];

/// The key a winit physical key is, if it has a binding name.
pub fn key_from_physical(key: PhysicalKey) -> Option<Key> {
    let PhysicalKey::Code(code) = key else {
        return None;
    };
    CODES.iter().find(|&&(c, _)| c == code).map(|&(_, k)| k)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomCodeShortcut {
    Copy,
    Paste,
}

/// The keys held, the first key pressed since the last game frame, and
/// the window's focus, from the window's events.
#[derive(Clone, Debug, Default)]
pub struct Keyboard {
    held: Vec<Key>,
    pressed: Option<Key>,
    typed: Vec<TypedKey>,
    focused: bool,
    copy_room_code: bool,
    paste_room_code: bool,
    room_code_shortcut: Option<RoomCodeShortcut>,
}

impl Keyboard {
    /// A keyboard whose window has focus (winit reports focus only as it
    /// changes; a window opens focused).
    pub fn focused() -> Self {
        Keyboard {
            focused: true,
            ..Keyboard::default()
        }
    }

    /// A key going down (`down`) or up. A key repeat is not a press.
    pub fn key(&mut self, key: PhysicalKey, down: bool, repeat: bool) {
        let Some(key) = key_from_physical(key) else {
            return;
        };
        if !self.focused {
            return;
        }
        if down {
            if !self.held.contains(&key) {
                self.held.push(key);
            }
            if !repeat && self.pressed.is_none() {
                self.pressed = Some(key);
            }
        } else {
            self.held.retain(|&k| k != key);
        }
    }

    /// A key press's text ([`typed_keys`]), key repeats included.
    pub fn text(&mut self, logical: &LogicalKey, text: Option<&str>, repeat: bool) {
        if !self.focused {
            return;
        }
        let room = TYPED_MAX.saturating_sub(self.typed.len());
        self.typed.extend(typed_keys(logical, text, repeat).into_iter().take(room));
    }

    pub fn set_room_code_shortcuts(&mut self, copy: bool, paste: bool) {
        self.copy_room_code = copy;
        self.paste_room_code = paste;
    }

    /// Consume the shortcut only where the current lobby accepts it. Use the
    /// logical letter so this follows the player's keyboard layout.
    pub(crate) fn room_code_key(&mut self, logical: &LogicalKey, repeat: bool) -> bool {
        if !self.focused
            || !(self.is_down(Key::LeftCtrl) || self.is_down(Key::RightCtrl))
            || [Key::LeftAlt, Key::RightAlt, Key::LeftSuper, Key::RightSuper]
                .into_iter().any(|key| self.is_down(key))
        {
            return false;
        }
        let LogicalKey::Character(letter) = logical else { return false };
        let shortcut = if self.copy_room_code && letter.eq_ignore_ascii_case("c") {
            RoomCodeShortcut::Copy
        } else if self.paste_room_code && letter.eq_ignore_ascii_case("v") {
            RoomCodeShortcut::Paste
        } else {
            return false;
        };
        if !repeat {
            self.room_code_shortcut = Some(shortcut);
        }
        true
    }

    pub fn take_room_code_shortcut(&mut self) -> Option<RoomCodeShortcut> {
        self.room_code_shortcut.take()
    }

    /// The window gaining or losing focus; losing it lets every key go.
    pub fn focus(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.held.clear();
            self.pressed = None;
            self.typed.clear();
            self.room_code_shortcut = None;
        }
    }

    /// The keys typed since the last call.
    pub fn take_typed(&mut self) -> Vec<TypedKey> {
        std::mem::take(&mut self.typed)
    }

    pub fn has_focus(&self) -> bool {
        self.focused
    }

    pub fn is_down(&self, key: Key) -> bool {
        self.held.contains(&key)
    }

    /// The first key pressed since the last call.
    pub fn take_pressed(&mut self) -> Option<Key> {
        self.pressed.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::{key_names, key_to_name, name_to_key, BindingsTable};

    #[test]
    fn every_binding_name_has_its_winit_key_and_no_two_codes_share_one() {
        for name in key_names() {
            let key = name_to_key(name).unwrap();
            let codes: Vec<_> = CODES.iter().filter(|&&(_, k)| k == key).collect();
            assert_eq!(codes.len(), 1, "{name}: {codes:?}");
        }
        assert_eq!(CODES.len(), key_names().count());
    }

    #[test]
    fn keys_mean_the_same_positions_as_before() {
        for (code, name) in [
            (KeyCode::KeyA, "A"),
            (KeyCode::KeyZ, "Z"),
            (KeyCode::Digit1, "Key1"),
            (KeyCode::Digit5, "Key5"),
            (KeyCode::Enter, "Enter"),
            (KeyCode::NumpadEnter, "NumPadEnter"),
            (KeyCode::Backspace, "Backspace"),
            (KeyCode::Space, "Space"),
            (KeyCode::Escape, "Escape"),
            (KeyCode::ArrowRight, "Right"),
            (KeyCode::Quote, "Apostrophe"),
            (KeyCode::BracketLeft, "LeftBracket"),
            (KeyCode::ContextMenu, "Menu"),
            (KeyCode::ControlRight, "RightCtrl"),
            (KeyCode::NumpadDecimal, "NumPadDot"),
        ] {
            let key = key_from_physical(PhysicalKey::Code(code)).unwrap();
            assert_eq!(key_to_name(key).unwrap(), name);
        }
        assert_eq!(key_from_physical(PhysicalKey::Code(KeyCode::PrintScreen)), None);
    }

    #[test]
    fn a_bindings_file_naming_any_key_loads_and_the_window_reports_that_key() {
        // Each name bound in turn to P1's first button, written out and
        // read back as a bindings file: the file loads, the button is that
        // key, and the winit code in that position is the key.
        let defaults = BindingsTable::default();
        for name in key_names() {
            let key = name_to_key(name).unwrap();
            let mut table = defaults.clone();
            let taken = defaults.entries().iter().any(|&(_, k)| k == key) || defaults.menu.all().contains(&key);
            if key == Key::Escape || taken {
                continue;
            }
            table.p1.buttons[0] = key;
            let back = BindingsTable::from_toml(&table.to_toml()).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(back.p1.buttons[0], key, "{name}");
            let &(code, _) = CODES.iter().find(|&&(_, k)| k == key).unwrap();
            assert_eq!(key_from_physical(PhysicalKey::Code(code)), Some(key), "{name}");
        }
        // And every key the default table binds is one the window reports.
        for (_, key) in defaults.entries() {
            assert!(CODES.iter().any(|&(_, k)| k == key), "{key:?}");
        }
    }

    #[test]
    fn focus_gates_the_keyboard() {
        let a = PhysicalKey::Code(KeyCode::KeyA);
        let mut keys = Keyboard::focused();
        keys.key(a, true, false);
        assert!(keys.is_down(Key::A));
        assert_eq!(keys.take_pressed(), Some(Key::A));
        assert_eq!(keys.take_pressed(), None);
        // A repeat holds but is not a press.
        keys.key(a, true, true);
        assert_eq!(keys.take_pressed(), None);
        // Losing focus lets go, and keys go unheard until focus returns.
        keys.focus(false);
        assert!(!keys.is_down(Key::A));
        keys.key(PhysicalKey::Code(KeyCode::KeyS), true, false);
        assert!(!keys.is_down(Key::S));
        assert_eq!(keys.take_pressed(), None);
        keys.focus(true);
        keys.key(PhysicalKey::Code(KeyCode::KeyS), true, false);
        assert!(keys.is_down(Key::S));
        keys.key(PhysicalKey::Code(KeyCode::KeyS), false, false);
        assert!(!keys.is_down(Key::S));
    }

    #[test]
    fn room_code_chords_require_focus_control_and_the_enabled_lobby_action() {
        let copy = LogicalKey::Character("c".into());
        let paste = LogicalKey::Character("V".into());
        let mut keys = Keyboard::focused();
        keys.key(PhysicalKey::Code(KeyCode::ControlLeft), true, false);
        assert!(!keys.room_code_key(&copy, false), "other screens keep their keys");

        keys.set_room_code_shortcuts(true, false);
        assert!(!keys.room_code_key(&paste, false));
        assert!(keys.room_code_key(&copy, false));
        assert_eq!(keys.take_room_code_shortcut(), Some(RoomCodeShortcut::Copy));
        assert!(keys.room_code_key(&copy, true), "consume the repeat as a shortcut");
        assert_eq!(keys.take_room_code_shortcut(), None);

        keys.set_room_code_shortcuts(false, true);
        keys.key(PhysicalKey::Code(KeyCode::AltRight), true, false);
        assert!(!keys.room_code_key(&paste, false), "preserve AltGr text");
        keys.key(PhysicalKey::Code(KeyCode::AltRight), false, false);
        assert!(keys.room_code_key(&paste, false));
        assert_eq!(keys.take_room_code_shortcut(), Some(RoomCodeShortcut::Paste));
        assert!(keys.room_code_key(&paste, false));
        keys.focus(false);
        assert_eq!(keys.take_room_code_shortcut(), None);
        assert!(!keys.room_code_key(&paste, false));
    }

    #[test]
    fn the_game_reads_the_window_keyboard_through_the_bindings_and_only_with_focus() {
        use crate::input::frame_inputs;
        use crate::pads::PadFrame;
        use sf_engine::front::settings::Device;

        let table = BindingsTable::default();
        let devices = [Device::Keyboard; 2];
        let pads = PadFrame::default();
        let mut keys = Keyboard::focused();
        keys.key(PhysicalKey::Code(KeyCode::KeyA), true, false);
        keys.key(PhysicalKey::Code(KeyCode::Enter), true, false);
        keys.key(PhysicalKey::Code(KeyCode::Digit5), true, false);
        let inputs = frame_inputs(&mut keys, &table, devices, &pads);
        assert!(inputs.p1.buttons[0], "A is P1's light punch");
        assert!(inputs.menu.confirm, "Enter confirms");
        assert!(inputs.cabinet[0], "5 is coin 1");
        assert_eq!(inputs.press, Some(sf_engine::front::rebind::Press::Key("A")));
        // The press is the frame's alone.
        assert_eq!(frame_inputs(&mut keys, &table, devices, &pads).press, None);
        keys.focus(false);
        assert_eq!(frame_inputs(&mut keys, &table, devices, &pads), Default::default());
    }

    #[test]
    fn a_press_types_its_text_or_its_editing_key() {
        use winit::keyboard::SmolStr;
        let ch = |c: &str| LogicalKey::Character(SmolStr::new(c));
        let named = |n| LogicalKey::Named(n);
        assert_eq!(typed_keys(&ch("a"), Some("a"), false), [TypedKey::Char('a')]);
        assert_eq!(typed_keys(&ch("a"), Some("a"), true), [TypedKey::Char('a')], "a repeat types again");
        assert_eq!(typed_keys(&named(NamedKey::Space), Some(" "), false), [TypedKey::Char(' ')]);
        assert_eq!(typed_keys(&named(NamedKey::Backspace), Some("\u{8}"), true), [TypedKey::Erase]);
        assert_eq!(typed_keys(&named(NamedKey::Delete), None, false), [TypedKey::EraseAhead]);
        assert_eq!(typed_keys(&named(NamedKey::ArrowLeft), None, false), [TypedKey::Left]);
        assert_eq!(typed_keys(&named(NamedKey::Home), None, false), [TypedKey::Home]);
        assert_eq!(typed_keys(&named(NamedKey::Enter), Some("\r"), false), [TypedKey::Enter]);
        assert!(typed_keys(&named(NamedKey::Enter), Some("\r"), true).is_empty(), "Enter held saves once");
        assert!(typed_keys(&named(NamedKey::Escape), None, true).is_empty());
        assert!(typed_keys(&named(NamedKey::Shift), None, false).is_empty());
        assert!(typed_keys(&named(NamedKey::Tab), Some("\t"), false).is_empty(), "no control characters");
        // Unfocused, nothing is typed; losing focus drops what was.
        let mut keys = Keyboard::focused();
        keys.text(&ch("q"), Some("q"), false);
        keys.focus(false);
        keys.text(&ch("w"), Some("w"), false);
        assert!(keys.take_typed().is_empty());
        keys.focus(true);
        for _ in 0..100 {
            keys.text(&ch("q"), Some("q"), false);
        }
        assert_eq!(keys.take_typed().len(), TYPED_MAX, "kept to a frame's worth");
        assert!(keys.take_typed().is_empty(), "taken once");
    }
}
