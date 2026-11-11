//! Ticket #159 (`FE9`): pads and the extended bindings, with no pad
//! attached. The pads are numbered by connection order and a pad that
//! returns takes the first empty number; the bindings file carries the
//! menu keys, each side's pad buttons and the pads' menu buttons, and an
//! older file without them takes their defaults; each side is driven by
//! its device (the keyboard until its pad is seen under `AUTO`, then the
//! pad, idle while the pad is lost); a pad's Options is its side's Start
//! and its Share its coin; the frame's first press is reported by name.

use minifb::Key;
use sf_engine::front::rebind::Press;
use sf_engine::front::settings::Device;
use sf_host::bindings::{BindingsTable, CLASSIC_PAD, DEFAULT_BINDINGS, DEFAULT_MENU_PAD};
use sf_host::input::{frame_inputs_from, source, Source};
use sf_host::pads::{PadButton, PadFrame, PadState, Slots};

#[test]
fn pads_are_numbered_by_connection_order_and_a_returning_pad_takes_the_first_empty_number() {
    let mut slots = Slots::default();
    assert_eq!(slots.connect(7), Some(0), "the first pad is PAD 1");
    assert_eq!(slots.connect(7), None, "a pad already numbered keeps its number");
    assert_eq!(slots.connect(3), Some(1), "the second is PAD 2");
    assert_eq!(slots.connect(9), None, "a third pad is ignored");
    assert_eq!(slots.disconnect(7), Some(0));
    assert_eq!(slots.disconnect(7), None);
    assert_eq!(slots.seen(), [true, true], "PAD 1 was seen, and is lost");
    assert_eq!(slots.connect(9), Some(0), "the next pad seen takes PAD 1");
    assert_eq!((slots.id(0), slots.id(1)), (Some(9), Some(3)));
}

#[test]
fn every_pad_button_has_a_name_that_reads_back() {
    for b in PadButton::ALL {
        assert_eq!(PadButton::from_name(b.name()), Some(b));
    }
    assert_eq!(PadButton::from_name("cross"), Some(PadButton::Cross));
    assert_eq!(PadButton::from_name("START"), None);
}

#[test]
fn the_bindings_file_carries_the_menu_keys_and_the_pads() {
    let mut t = DEFAULT_BINDINGS;
    assert_eq!(t.menu.all(), [Key::Enter, Key::Backspace, Key::Space, Key::Q, Key::E]);
    assert_eq!(t.p1_pad, CLASSIC_PAD);
    assert_eq!(
        CLASSIC_PAD.buttons,
        [PadButton::Square, PadButton::Triangle, PadButton::R1, PadButton::Cross, PadButton::Circle, PadButton::R2]
    );
    assert_eq!(t.menu_pad, [PadButton::Cross, PadButton::Circle, PadButton::Options, PadButton::L1, PadButton::R1]);
    t.menu.confirm = Key::NumPadEnter;
    t.p2_pad.buttons[0] = PadButton::L2;
    t.menu_pad[1] = PadButton::Share;
    let text = t.to_toml();
    assert!(text.contains("[menu]\nconfirm = \"NumPadEnter\"\n"), "{text}");
    assert!(text.contains("[p2_pad]\n"), "{text}");
    assert!(text.contains("button1 = \"L2\""), "{text}");
    assert!(text.contains("[menu_pad]\nconfirm = \"CROSS\"\nback = \"SHARE\""), "{text}");
    assert_eq!(BindingsTable::from_toml(&text), Ok(t));

    // A file from before the pads: the 25 keys only. The rest are defaults.
    let old: String = DEFAULT_BINDINGS.to_toml().split("\n[menu]").next().unwrap().to_owned();
    assert_eq!(BindingsTable::from_toml(&old), Ok(DEFAULT_BINDINGS));

    // Refused: Escape on a menu input, a menu key twice, a pad button twice
    // in one side's list.
    let menu_escape = text.replace("confirm = \"NumPadEnter\"", "confirm = \"Escape\"");
    assert!(BindingsTable::from_toml(&menu_escape).is_err());
    let menu_twice = text.replace("back = \"Backspace\"", "back = \"NumPadEnter\"");
    assert!(BindingsTable::from_toml(&menu_twice).unwrap_err().contains("duplicate menu key"));
    let pad_twice = text.replacen("button2 = \"TRIANGLE\"", "button2 = \"SQUARE\"", 1);
    assert!(BindingsTable::from_toml(&pad_twice).unwrap_err().contains("duplicate pad button 'SQUARE' in [p1_pad]"));
    let unknown = text.replacen("button2 = \"TRIANGLE\"", "button2 = \"START\"", 1);
    assert!(BindingsTable::from_toml(&unknown).unwrap_err().contains("unknown pad button 'START'"));
}

#[test]
fn the_walks_names_turn_back_into_the_table() {
    let binds = DEFAULT_BINDINGS.binds();
    assert_eq!(binds.p1[0].key, "Right");
    assert_eq!(binds.p1[4].pad, "SQUARE");
    assert_eq!(binds.p2[7].key, "J");
    assert_eq!(binds.menu[2].key, "Space");
    assert_eq!(binds.menu[2].pad, "OPTIONS");
    assert_eq!(binds.cabinet, ["Key5", "Key6", "Key1", "Key2", "Key9"]);
    assert_eq!(DEFAULT_BINDINGS.with_binds(&binds), Ok(DEFAULT_BINDINGS));
    let mut changed = binds;
    changed.p1[4].key = "W";
    changed.p2[9].pad = "L2";
    changed.menu[0].pad = "PS";
    let t = DEFAULT_BINDINGS.with_binds(&changed).unwrap();
    assert_eq!(t.p1.buttons[0], Key::W);
    assert_eq!(t.p2_pad.buttons[5], PadButton::L2);
    assert_eq!(t.menu_pad[0], PadButton::Ps);
    let mut clash = binds;
    clash.p1[4].key = "H"; // P2's right
    assert!(DEFAULT_BINDINGS.with_binds(&clash).is_err());
}

fn frame(pads: [Option<PadState>; 2], seen: [bool; 2], lost: [bool; 2]) -> PadFrame {
    PadFrame { pads, seen, lost, pressed: None }
}

#[test]
fn each_side_follows_its_device() {
    let none = PadFrame::default();
    let pad = Some(PadState::default());
    let one = frame([pad, None], [true, false], [false; 2]);
    let lost = frame([None, None], [true, false], [true, false]);
    assert_eq!(source(Device::Auto, 0, &none), Source::Keyboard, "AUTO is the keyboard until a pad is seen");
    assert_eq!(source(Device::Auto, 0, &one), Source::Pad(0), "then P1 takes PAD 1");
    assert_eq!(source(Device::Auto, 1, &one), Source::Keyboard, "and P2 stays on the keyboard");
    assert_eq!(source(Device::Auto, 0, &lost), Source::Idle, "a lost pad idles its side");
    assert_eq!(source(Device::Keyboard, 0, &one), Source::Keyboard);
    assert_eq!(source(Device::Pad1, 1, &one), Source::Pad(0), "the Controls tab swaps the pads");
    assert_eq!(source(Device::Pad2, 0, &one), Source::Idle, "a pad never seen idles its side");
}

#[test]
fn a_pad_side_reads_its_pad_and_the_keyboard_never_takes_over() {
    let t = DEFAULT_BINDINGS;
    let held = PadState::holding(&[PadButton::Square, PadButton::DpadDown, PadButton::Cross, PadButton::Options]);
    let pads = frame([Some(held), None], [true, false], [false; 2]);
    // The keyboard holds P1's Right and LP and P2's LP.
    let keys = [Key::Right, Key::A, Key::U];
    let i = frame_inputs_from(&t, [Device::Auto; 2], &pads, |k| keys.contains(&k), None);
    assert_eq!(i.p1.directions, [false, false, true, false], "P1's stick is the pad's d-pad, not the arrows");
    assert_eq!(i.p1.buttons, [true, false, false, true, false, false], "Square is LP, Cross is LK");
    assert!(i.p2.buttons[0], "P2 is on the keyboard");
    let menu = i.pads[0].expect("P1's pad menu buttons");
    assert!(menu.confirm && menu.start && !menu.back, "Cross confirms, Options is Start");
    assert_eq!(i.pads[1], None);
    assert!(i.cabinet[2], "Options is P1's start on the cabinet");
    assert!(!i.cabinet[0]);

    // PAD 1 lost: the frame reports it, P1 idles though the keys are held.
    let lost = frame([None, None], [true, false], [true, false]);
    let i = frame_inputs_from(&t, [Device::Auto; 2], &lost, |k| keys.contains(&k), None);
    assert_eq!(i.pad_lost, [true, false]);
    assert_eq!((i.p1.directions, i.p1.buttons), ([false; 4], [false; 6]));
    let i = frame_inputs_from(&t, [Device::Auto; 2], &frame([None, None], [true, false], [false; 2]), |k| keys.contains(&k), None);
    assert_eq!(i.pad_lost, [false, false], "lost on one frame only");
    assert_eq!(i.p1.buttons, [false; 6], "still idle");

    // The left stick moves whatever the d-pad is bound to; Share is a coin.
    let mut stick = PadState::holding(&[PadButton::Share]);
    stick.stick = [true, false, false, true];
    let i = frame_inputs_from(&t, [Device::Keyboard, Device::Pad1], &frame([Some(stick), None], [true, false], [false; 2]), |_| false, None);
    assert_eq!(i.p2.directions, [true, false, false, true]);
    assert!(i.cabinet[1], "Share is P2's coin");
}

#[test]
fn the_first_press_of_a_frame_is_named() {
    let t = DEFAULT_BINDINGS;
    let mut pads = PadFrame::default();
    assert_eq!(frame_inputs_from(&t, [Device::Auto; 2], &pads, |_| false, Some(Key::W)).press, Some(Press::Key("W")));
    pads.pressed = Some(PadButton::Triangle);
    assert_eq!(frame_inputs_from(&t, [Device::Auto; 2], &pads, |_| false, None).press, Some(Press::Pad("TRIANGLE")));
    assert_eq!(
        frame_inputs_from(&t, [Device::Auto; 2], &pads, |_| false, Some(Key::Escape)).press,
        Some(Press::Key("Escape")),
        "a key wins a tie, and Esc is reported for the walk"
    );
    let _ = DEFAULT_MENU_PAD;
}
