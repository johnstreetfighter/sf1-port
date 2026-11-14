//! Per-fighter translation for the six logical pad buttons used by Fight.

use crate::era::EraFamily;
use crate::front::PlayerInputs;
use crate::moves::{ButtonClass, Dir};
use crate::session_io::ControlScheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shortcut {
    Projectile,
    AntiAir,
    Advance,
    Extra,
    Finisher,
}

crate::canonical::canonical_enum!(Shortcut, "ModernShortcut.v1";
    Projectile => 0, AntiAir => 1, Advance => 2, Extra => 3, Finisher => 4,
);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PressurePad {
    frames: u8,
    fired: bool,
}

crate::canonical::canonical_struct!(PressurePad, "PressurePad.v1", { frames, fired });

impl PressurePad {
    fn step(&mut self, [light, medium, heavy]: [bool; 3]) -> Option<usize> {
        if !(light || medium || heavy) {
            let strength =
                (!self.fired && self.frames != 0).then_some(usize::from(self.frames >= 3));
            *self = Self::default();
            return strength;
        }
        self.frames = self.frames.saturating_add(1).min(5);
        if self.fired {
            return None;
        }
        // An explicit heavy binding fires now. An explicit medium binding
        // caps the gesture at medium so it never becomes a heavy press.
        let strength = if heavy && !light && !medium {
            Some(2)
        } else if medium && !light && !heavy {
            (self.frames >= 3).then_some(1)
        } else {
            (self.frames == 5).then_some(2)
        };
        self.fired = strength.is_some();
        strength
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SchemeState {
    pub mode: ControlScheme,
    punch: PressurePad,
    kick: PressurePad,
    previous: [bool; 6],
    assist_strength: Option<u8>,
    assist_step: u8,
    assist_left: u8,
    assist_pending: bool,
    pub assist_normal: Option<(u8, bool, bool)>,
    pub shortcut: Option<Shortcut>,
    pub shortcut_strength: u8,
    pub shortcut_scaled: bool,
    pub executing: Option<u8>,
    pub scaled_move: bool,
    pub throw_macro: bool,
}

crate::canonical::canonical_struct!(SchemeState, "SchemeState.v1", {
    mode, punch, kick, previous, assist_strength, assist_step, assist_left, assist_pending, assist_normal, shortcut,
    shortcut_strength, shortcut_scaled, executing, scaled_move, throw_macro,
});

impl SchemeState {
    pub(crate) fn translate(
        &mut self,
        scheme: ControlScheme,
        mut input: PlayerInputs,
        facing_right: bool,
        family: Option<EraFamily>,
        close: bool,
        penalty: bool,
    ) -> PlayerInputs {
        self.shortcut = None;
        self.assist_normal = None;
        self.shortcut_scaled = false;
        self.executing = None;
        self.throw_macro = false;
        self.mode = scheme;
        match scheme {
            ControlScheme::Classic => {
                // Show, drills and CPU pads already have cabinet semantics.
                // The damage flag remains attached to the preceding move.
                let scaled_move = self.scaled_move;
                *self = Self {
                    scaled_move,
                    ..Self::default()
                };
            }
            ControlScheme::Pneumatic => {
                let b = input.buttons;
                input.buttons = [false; 6];
                if let Some(strength) = self.punch.step([b[0], b[1], b[2]]) {
                    input.buttons[strength] = true;
                }
                if let Some(strength) = self.kick.step([b[3], b[4], b[5]]) {
                    input.buttons[3 + strength] = true;
                }
            }
            ControlScheme::Modern => self.modern(&mut input, facing_right, family, close, penalty),
        }
        input
    }

    fn modern(
        &mut self,
        input: &mut PlayerInputs,
        facing_right: bool,
        family: Option<EraFamily>,
        close: bool,
        penalty: bool,
    ) {
        let raw = input.buttons;
        let [light, medium, heavy, special, assist, macro_button] = raw;
        let dir = Dir::from_stick(input.directions, facing_right);
        let fresh = std::array::from_fn::<_, 6, _>(|i| raw[i] && !self.previous[i]);
        self.previous = raw;
        input.buttons = [light, medium, heavy, false, false, false];

        if macro_button {
            self.assist_pending = false;
            let forward = dir == Some(Dir::Forward);
            let neutral = dir.is_none();
            let command = if forward && assist {
                ButtonClass::HeavyPair
            } else if dir == Some(Dir::DownBack)
                || (neutral && !close && matches!(family, Some(EraFamily::Sf4 | EraFamily::Sf6)))
            {
                ButtonClass::MediumPair
            } else if close && matches!(dir, None | Some(Dir::Forward | Dir::Back)) {
                self.throw_macro = fresh[5];
                ButtonClass::LightPair
            } else {
                ButtonClass::DirectionOnly
            };
            input.buttons = mask_buttons(command.fixed_mask());
        } else if special {
            self.assist_pending = false;
            input.buttons = [false; 6];
            if fresh[3] {
                self.shortcut = match dir {
                    _ if assist => Some(Shortcut::Finisher),
                    None => Some(Shortcut::Projectile),
                    Some(Dir::Forward) => Some(Shortcut::AntiAir),
                    Some(Dir::Back) => Some(Shortcut::Advance),
                    Some(Dir::Down) => Some(Shortcut::Extra),
                    Some(Dir::DownForward) => Some(Shortcut::Finisher),
                    _ => None,
                };
                self.shortcut_strength = 3;
                self.shortcut_scaled = penalty;
            }
        } else if assist {
            self.assist_left = self.assist_left.saturating_sub(1);
            if self.assist_left == 0 {
                self.assist_pending = false;
                self.assist_step = 0;
            }
            if let Some(strength) = fresh[..3]
                .iter()
                .rposition(|&pressed| pressed)
                .map(|i| i as u8)
            {
                if self.assist_strength != Some(strength) {
                    self.assist_step = 0;
                }
                self.assist_strength = Some(strength);
                self.assist_left = 36;
                self.assist_pending = true;
            }
            input.buttons = [false; 6];
            if self.assist_pending {
                let strength = self
                    .assist_strength
                    .expect("a queued assist press has a strength");
                match (strength, self.assist_step) {
                    (0, 0) => self.assist_normal = Some((1, false, false)),
                    (0, 1) => self.assist_normal = Some((1, true, false)),
                    (1, 0) => self.assist_normal = Some((2, false, true)),
                    (1, 1) => self.assist_normal = Some((2, false, false)),
                    (2, 0) => self.assist_normal = Some((3, false, false)),
                    _ => {
                        self.shortcut = Some(if strength == 2 && self.assist_step == 2 {
                            Shortcut::Finisher
                        } else {
                            Shortcut::Projectile
                        });
                        self.shortcut_strength = strength + 1;
                        self.shortcut_scaled = penalty;
                    }
                }
            }
        } else {
            self.assist_strength = None;
            self.assist_step = 0;
            self.assist_left = 0;
            self.assist_pending = false;
        }
    }

    pub(crate) fn accept_assist(&mut self) {
        if self.assist_pending {
            self.assist_pending = false;
            self.assist_step = (self.assist_step + 1) % 3;
        }
    }

    pub(crate) fn assist_can_chain_normal(&self) -> bool {
        self.assist_step == 1 && self.assist_normal.is_some()
    }
}

pub(crate) fn mask_buttons(mask: u8) -> [bool; 6] {
    std::array::from_fn(|i| mask & (1 << i) != 0)
}

pub(crate) fn shortcut_buttons(button: ButtonClass, strength: u8) -> u8 {
    let punch = 1 << strength.clamp(1, 3).saturating_sub(1);
    match button {
        ButtonClass::Punch | ButtonClass::Either | ButtonClass::MashPunch => punch,
        ButtonClass::Kick | ButtonClass::KickOnLastDirection => punch << 3,
        ButtonClass::TriplePunch => 0b000111,
        ButtonClass::TripleKick => 0b111000,
        ButtonClass::PairPunch => 0b000011,
        ButtonClass::PairKick => 0b011000,
        ButtonClass::SameStrength => punch | punch << 3,
        _ => button.fixed_mask(),
    }
}

pub(crate) fn shortcut_damage(damage: u16) -> u16 {
    if damage == 0 {
        0
    } else {
        (u32::from(damage) * 4 / 5).max(1) as u16
    }
}
