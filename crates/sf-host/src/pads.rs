//! Pads (spec #150, section 5.6): DualShock 4s and other pads through
//! `gilrs` and its default Windows Gaming Input backend. Pads are numbered
//! by connection order, `PAD 1` and `PAD 2`; a pad that connects after a
//! loss takes the first empty number, so a pulled cable plugged back in
//! returns its side. A third pad is ignored. Each frame gives every
//! numbered pad's held buttons and left stick, the numbers lost on that
//! frame, and the first button pressed (for the rebind walk).

use std::time::{Duration, Instant};

use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};

/// How long the backend gets at start to report the pads already plugged
/// in. Windows Gaming Input reports them as connection events a moment
/// after it starts (a DualShock 4 over USB took 41 ms), not at once.
const SETTLE: Duration = Duration::from_millis(250);

/// A pad's buttons, named as on a DualShock 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PadButton {
    Cross,
    Circle,
    Square,
    Triangle,
    L1,
    R1,
    L2,
    R2,
    Share,
    Options,
    L3,
    R3,
    Ps,
    DpadRight,
    DpadLeft,
    DpadDown,
    DpadUp,
}

impl PadButton {
    pub const ALL: [PadButton; 17] = [
        PadButton::Cross,
        PadButton::Circle,
        PadButton::Square,
        PadButton::Triangle,
        PadButton::L1,
        PadButton::R1,
        PadButton::L2,
        PadButton::R2,
        PadButton::Share,
        PadButton::Options,
        PadButton::L3,
        PadButton::R3,
        PadButton::Ps,
        PadButton::DpadRight,
        PadButton::DpadLeft,
        PadButton::DpadDown,
        PadButton::DpadUp,
    ];

    /// The name the bindings file and the screens use.
    pub const fn name(self) -> &'static str {
        match self {
            PadButton::Cross => "CROSS",
            PadButton::Circle => "CIRCLE",
            PadButton::Square => "SQUARE",
            PadButton::Triangle => "TRIANGLE",
            PadButton::L1 => "L1",
            PadButton::R1 => "R1",
            PadButton::L2 => "L2",
            PadButton::R2 => "R2",
            PadButton::Share => "SHARE",
            PadButton::Options => "OPTIONS",
            PadButton::L3 => "L3",
            PadButton::R3 => "R3",
            PadButton::Ps => "PS",
            PadButton::DpadRight => "DPAD RIGHT",
            PadButton::DpadLeft => "DPAD LEFT",
            PadButton::DpadDown => "DPAD DOWN",
            PadButton::DpadUp => "DPAD UP",
        }
    }

    pub fn from_name(name: &str) -> Option<PadButton> {
        let name = name.trim();
        PadButton::ALL.into_iter().find(|b| b.name().eq_ignore_ascii_case(name))
    }

    const fn bit(self) -> u32 {
        1 << self as u32
    }

    /// The `gilrs` button (SDL's names: South is Cross, East is Circle).
    const fn gilrs(self) -> Button {
        match self {
            PadButton::Cross => Button::South,
            PadButton::Circle => Button::East,
            PadButton::Square => Button::West,
            PadButton::Triangle => Button::North,
            PadButton::L1 => Button::LeftTrigger,
            PadButton::R1 => Button::RightTrigger,
            PadButton::L2 => Button::LeftTrigger2,
            PadButton::R2 => Button::RightTrigger2,
            PadButton::Share => Button::Select,
            PadButton::Options => Button::Start,
            PadButton::L3 => Button::LeftThumb,
            PadButton::R3 => Button::RightThumb,
            PadButton::Ps => Button::Mode,
            PadButton::DpadRight => Button::DPadRight,
            PadButton::DpadLeft => Button::DPadLeft,
            PadButton::DpadDown => Button::DPadDown,
            PadButton::DpadUp => Button::DPadUp,
        }
    }

    fn from_gilrs(button: Button) -> Option<PadButton> {
        PadButton::ALL.into_iter().find(|b| b.gilrs() == button)
    }
}

/// One pad on one frame: the buttons held, and the left stick past half
/// way (right, left, down, up).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    held: u32,
    pub stick: [bool; 4],
}

impl PadState {
    pub fn down(&self, button: PadButton) -> bool {
        self.held & button.bit() != 0
    }

    /// The state with `buttons` held and the stick at rest (for tests).
    pub fn holding(buttons: &[PadButton]) -> PadState {
        PadState {
            held: buttons.iter().fold(0, |held, b| held | b.bit()),
            stick: [false; 4],
        }
    }
}

/// The numbered pads on one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadFrame {
    /// `PAD 1` and `PAD 2`: `None` until one is seen, and while it is lost.
    pub pads: [Option<PadState>; 2],
    /// Whether each number has held a pad since launch.
    pub seen: [bool; 2],
    /// The numbers whose pad was lost on this frame.
    pub lost: [bool; 2],
    /// The first button pressed on this frame, on a numbered pad.
    pub pressed: Option<PadButton>,
}

/// The KADE SDL mapping's `righty:a5` aliases the first hat's vertical
/// axis on GP2040-CE DirectInput devices that expose four analog axes to WGI.
/// Compare the underlying code so a real right stick is left alone.
#[cfg(windows)]
fn is_kade_vertical_hat(vendor: Option<u16>, product: Option<u16>, code: u32) -> bool {
    vendor == Some(0x10c4)
        && product == Some(0x82c0)
        && code == gilrs_core::native_ev_codes::AXIS_DPADY.into_u32()
}

#[cfg(any(windows, test))]
fn vertical_hat_button(value: f32) -> Option<PadButton> {
    if value > 0.5 {
        Some(PadButton::DpadUp)
    } else if value < -0.5 {
        Some(PadButton::DpadDown)
    } else {
        None
    }
}

#[cfg(any(windows, test))]
fn record_hat_edge(pressed: &mut Option<PadButton>, previous: &mut Option<PadButton>, value: f32) {
    let next = vertical_hat_button(value);
    if std::mem::replace(previous, next) != next && pressed.is_none() {
        *pressed = next;
    }
}

/// Which pad holds each number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slots {
    ids: [Option<usize>; 2],
    seen: [bool; 2],
}

impl Slots {
    /// A pad connected: the number it takes, the first empty one (`None`
    /// when it already has one, or both are taken).
    pub fn connect(&mut self, id: usize) -> Option<usize> {
        if self.number(id).is_some() {
            return None;
        }
        let n = self.ids.iter().position(Option::is_none)?;
        self.ids[n] = Some(id);
        self.seen[n] = true;
        Some(n)
    }

    /// A pad disconnected: the number it gives up.
    pub fn disconnect(&mut self, id: usize) -> Option<usize> {
        let n = self.number(id)?;
        self.ids[n] = None;
        Some(n)
    }

    pub fn number(&self, id: usize) -> Option<usize> {
        self.ids.iter().position(|&s| s == Some(id))
    }

    pub fn id(&self, n: usize) -> Option<usize> {
        self.ids[n]
    }

    pub fn seen(&self) -> [bool; 2] {
        self.seen
    }
}

/// The pads, read through `gilrs` once a frame.
pub struct PadReader {
    gilrs: Option<Gilrs>,
    /// Every pad the backend has named, by its number in [`Slots`].
    handles: Vec<GamepadId>,
    slots: Slots,
    notes: Vec<String>,
    #[cfg(windows)]
    hat: [Option<PadButton>; 2],
}

impl PadReader {
    /// Start the pad backend, give it a moment to report the pads already
    /// plugged in, and number them.
    pub fn open() -> PadReader {
        let mut reader = PadReader {
            gilrs: None,
            handles: Vec::new(),
            slots: Slots::default(),
            notes: Vec::new(),
            #[cfg(windows)]
            hat: [None; 2],
        };
        match Gilrs::new() {
            Ok(mut gilrs) => {
                let start = Instant::now();
                while start.elapsed() < SETTLE {
                    while gilrs.next_event().is_some() {}
                    std::thread::sleep(Duration::from_millis(5));
                }
                let connected: Vec<(GamepadId, String)> = gilrs
                    .gamepads()
                    .filter(|(_, g)| g.is_connected())
                    .map(|(id, g)| (id, g.name().to_owned()))
                    .collect();
                reader.gilrs = Some(gilrs);
                for (id, name) in connected {
                    reader.connected(id, &name);
                }
                if reader.slots.seen() == [false; 2] {
                    reader.notes.push("no pads connected".to_owned());
                }
            }
            Err(e) => reader.notes.push(format!("the pad backend did not start: {e}")),
        }
        reader
    }

    fn connected(&mut self, handle: GamepadId, name: &str) {
        if !self.handles.contains(&handle) {
            self.handles.push(handle);
        }
        let id = usize::from(handle);
        if self.slots.number(id).is_some() {
            return;
        }
        match self.slots.connect(id) {
            Some(n) => self.notes.push(format!("PAD {} seen: {name}", n + 1)),
            None => self.notes.push(format!("a third pad is ignored: {name}")),
        }
    }

    /// The lines for the log since the last call: pads seen or lost.
    pub fn take_notes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notes)
    }

    /// The names of the connected pads, numbered or not.
    pub fn names(&self) -> Vec<String> {
        self.gilrs
            .iter()
            .flat_map(|g| g.gamepads().filter(|(_, p)| p.is_connected()).map(|(_, p)| p.name().to_owned()))
            .collect()
    }

    /// One frame: the backend's events, then each numbered pad's state.
    pub fn poll(&mut self) -> PadFrame {
        let mut frame = PadFrame::default();
        let Some(gilrs) = self.gilrs.as_mut() else {
            return frame;
        };
        let mut events = Vec::new();
        while let Some(event) = gilrs.next_event() {
            events.push((event.id, event.event));
        }
        for (handle, event) in events {
            let id = usize::from(handle);
            match event {
                EventType::Connected => {
                    let name = self
                        .gilrs
                        .as_ref()
                        .and_then(|g| g.connected_gamepad(handle).map(|p| p.name().to_owned()))
                        .unwrap_or_default();
                    self.connected(handle, &name);
                }
                EventType::Disconnected => {
                    if let Some(n) = self.slots.disconnect(id) {
                        frame.lost[n] = true;
                        self.notes.push(format!("PAD {} lost", n + 1));
                        #[cfg(windows)]
                        {
                            self.hat[n] = None;
                        }
                    }
                }
                EventType::ButtonPressed(button, _) if self.slots.number(id).is_some() && frame.pressed.is_none() => {
                    frame.pressed = PadButton::from_gilrs(button);
                }
                #[cfg(windows)]
                EventType::AxisChanged(Axis::RightStickY, value, code) => {
                    let pad = self.gilrs.as_ref().and_then(|g| g.connected_gamepad(handle));
                    if let (Some(n), Some(pad)) = (self.slots.number(id), pad) {
                        if is_kade_vertical_hat(pad.vendor_id(), pad.product_id(), code.into_u32()) {
                            record_hat_edge(&mut frame.pressed, &mut self.hat[n], value);
                        }
                    }
                }
                _ => {}
            }
        }
        let gilrs = self.gilrs.as_ref().expect("opened");
        for n in 0..2 {
            let handle = self.slots.id(n).and_then(|id| self.handles.iter().find(|&&h| usize::from(h) == id));
            let Some(pad) = handle.and_then(|&h| gilrs.connected_gamepad(h)) else {
                continue;
            };
            let held = PadButton::ALL
                .into_iter()
                .filter(|b| pad.is_pressed(b.gilrs()))
                .fold(0, |held, b| held | b.bit());
            #[cfg(windows)]
            let held = {
                self.hat[n] = pad
                    .axis_code(Axis::RightStickY)
                    .filter(|code| is_kade_vertical_hat(pad.vendor_id(), pad.product_id(), code.into_u32()))
                    .and_then(|_| vertical_hat_button(pad.value(Axis::RightStickY)));
                held | self.hat[n].map_or(0, PadButton::bit)
            };
            let (x, y) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            frame.pads[n] = Some(PadState {
                held,
                stick: [x > 0.5, x < -0.5, y < -0.5, y > 0.5],
            });
        }
        frame.seen = self.slots.seen();
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::{PadButton, record_hat_edge, vertical_hat_button};

    #[test]
    fn directinput_hat_supplies_vertical_hold_and_rebind_edge() {
        let mut previous = None;
        for (value, expected, edge) in [
            (1.0, Some(PadButton::DpadUp), Some(PadButton::DpadUp)),
            (1.0, Some(PadButton::DpadUp), None),
            (-1.0, Some(PadButton::DpadDown), Some(PadButton::DpadDown)),
            (0.0, None, None),
            (1.0, Some(PadButton::DpadUp), Some(PadButton::DpadUp)),
        ] {
            let mut pressed = None;
            record_hat_edge(&mut pressed, &mut previous, value);
            assert_eq!(vertical_hat_button(value), expected);
            assert_eq!(pressed, edge);
        }
    }

    #[test]
    fn directinput_hat_preserves_first_press_during_a_tap() {
        let mut previous = None;
        let mut pressed = None;
        record_hat_edge(&mut pressed, &mut previous, -1.0);
        record_hat_edge(&mut pressed, &mut previous, 0.0);
        assert_eq!(pressed, Some(PadButton::DpadDown));
        assert_eq!(previous, None);

        pressed = Some(PadButton::Square);
        record_hat_edge(&mut pressed, &mut previous, 1.0);
        assert_eq!(pressed, Some(PadButton::Square));
    }

    #[cfg(windows)]
    #[test]
    fn directinput_correction_uses_kade_hat_code_only() {
        use gilrs_core::native_ev_codes::{AXIS_DPADY, AXIS_RSTICKY};

        assert!(super::is_kade_vertical_hat(Some(0x10c4), Some(0x82c0), AXIS_DPADY.into_u32()));
        assert!(!super::is_kade_vertical_hat(Some(0x10c4), Some(0x82c0), AXIS_RSTICKY.into_u32()));
        assert!(!super::is_kade_vertical_hat(Some(0x045e), Some(0x028e), AXIS_DPADY.into_u32()));
    }
}
