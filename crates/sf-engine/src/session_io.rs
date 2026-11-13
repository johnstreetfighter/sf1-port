//! Sound test and per-player control schemes (`#100`, `O9`).
//!
//! Faithful play stays on Classic 6-button input and the native YM2151 +
//! MSM5205 latch. Overhaul play can pick a scheme per player. The Jukebox
//! (external tracks per stage or fighter) is [`crate::jukebox`]; the sound
//! test's music entries follow it.

use crate::jukebox::Jukebox;

/// Cabinet scheme for one player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlScheme {
    #[default]
    Classic,
    Pneumatic,
    Modern,
}

impl ControlScheme {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Classic => "CLASSIC",
            Self::Pneumatic => "PNEUMATIC",
            Self::Modern => "MODERN",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Classic => Self::Pneumatic,
            Self::Pneumatic => Self::Modern,
            Self::Modern => Self::Classic,
        }
    }

    pub const fn prev(self) -> Self {
        self.next().next()
    }

    pub const fn slug(self) -> &'static str {
        match self {
            ControlScheme::Classic => "classic",
            ControlScheme::Pneumatic => "pneumatic",
            ControlScheme::Modern => "modern",
        }
    }

    pub fn from_name(name: &str) -> Result<Self, String> {
        match name.trim() {
            "classic" => Ok(ControlScheme::Classic),
            "pneumatic" => Ok(ControlScheme::Pneumatic),
            "modern" => Ok(ControlScheme::Modern),
            other => Err(format!("unknown control scheme '{other}'")),
        }
    }
}

crate::canonical::canonical_enum!(ControlScheme, "ControlScheme.v1";
    Classic => 0, Pneumatic => 1, Modern => 2,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialKind {
    Hadoken,
    Shoryuken,
    Tatsumaki,
    Super,
}

/// What one player presented this frame, after the scheme translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameOut {
    /// `LP, MP, HP, LK, MK, HK`.
    pub buttons: [bool; 6],
    pub special: Option<SpecialCommit>,
}

/// A shortcut special the modern scheme asked the match to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecialCommit {
    pub kind: SpecialKind,
    pub penalized: bool,
    /// Frames the shortcut waited before this commit. Manual motions stay at 0.
    pub startup_extra: u8,
}

/// Raw pads for one player. Classic reads `classic`. Pneumatic reads the two
/// pads and, when set, the analog pressures. Modern reads the five buttons
/// and the direction flags.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FramePads {
    pub classic: [bool; 6],
    pub punch: bool,
    pub kick: bool,
    pub punch_pressure: Option<f32>,
    pub kick_pressure: Option<f32>,
    pub light: bool,
    pub medium: bool,
    pub heavy: bool,
    pub special: bool,
    pub assist: bool,
    pub down: bool,
    pub forward: bool,
    pub back: bool,
    /// A completed manual motion this frame. Modern leaves that special alone.
    pub manual_motion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SideState {
    punch_held: u16,
    kick_held: u16,
    delay: u8,
    pending: Option<SpecialKind>,
}

impl Default for SideState {
    fn default() -> Self {
        Self {
            punch_held: 0,
            kick_held: 0,
            delay: 0,
            pending: None,
        }
    }
}

/// Per-player schemes and the competitive damage/startup rule.
#[derive(Debug, Clone)]
pub struct ControlSession {
    pub p1: ControlScheme,
    pub p2: ControlScheme,
    pub penalty: bool,
    p1_state: SideState,
    p2_state: SideState,
}

impl ControlSession {
    pub fn new(p1: ControlScheme, p2: ControlScheme, penalty: bool) -> Self {
        Self {
            p1,
            p2,
            penalty,
            p1_state: SideState::default(),
            p2_state: SideState::default(),
        }
    }

    pub fn step(&mut self, p1: &FramePads, p2: &FramePads) -> (FrameOut, FrameOut) {
        let penalty = self.penalty;
        let a = translate(self.p1, &mut self.p1_state, p1, penalty);
        let b = translate(self.p2, &mut self.p2_state, p2, penalty);
        (a, b)
    }
}

fn translate(
    scheme: ControlScheme,
    state: &mut SideState,
    pads: &FramePads,
    penalty: bool,
) -> FrameOut {
    match scheme {
        ControlScheme::Classic => FrameOut {
            buttons: pads.classic,
            special: None,
        },
        ControlScheme::Pneumatic => FrameOut {
            buttons: pneumatic_buttons(state, pads),
            special: None,
        },
        ControlScheme::Modern => modern_buttons(state, pads, penalty),
    }
}

fn pneumatic_buttons(state: &mut SideState, pads: &FramePads) -> [bool; 6] {
    let mut buttons = [false; 6];
    if let Some(strength) = pad_strength(&mut state.punch_held, pads.punch, pads.punch_pressure) {
        buttons[strength_index(strength)] = true;
    }
    if let Some(strength) = pad_strength(&mut state.kick_held, pads.kick, pads.kick_pressure) {
        buttons[3 + strength_index(strength)] = true;
    }
    buttons
}

fn pad_strength(held: &mut u16, digital: bool, pressure: Option<f32>) -> Option<Strength> {
    if let Some(pressure) = pressure {
        *held = 0;
        return strength_from_pressure(pressure);
    }
    if digital {
        *held = held.saturating_add(1);
        strength_from_frames(*held)
    } else {
        *held = 0;
        None
    }
}

fn strength_from_frames(frames: u16) -> Option<Strength> {
    match frames {
        0 => None,
        1..=2 => Some(Strength::Light),
        3..=4 => Some(Strength::Medium),
        _ => Some(Strength::Heavy),
    }
}

fn strength_from_pressure(pressure: f32) -> Option<Strength> {
    if pressure <= 0.0 {
        None
    } else if pressure <= 1.0 / 3.0 {
        Some(Strength::Light)
    } else if pressure <= 2.0 / 3.0 {
        Some(Strength::Medium)
    } else {
        Some(Strength::Heavy)
    }
}

fn strength_index(strength: Strength) -> usize {
    match strength {
        Strength::Light => 0,
        Strength::Medium => 1,
        Strength::Heavy => 2,
    }
}

fn modern_buttons(state: &mut SideState, pads: &FramePads, penalty: bool) -> FrameOut {
    let mut buttons = [false; 6];
    let kick = pads.assist;
    if pads.light {
        buttons[if kick { 3 } else { 0 }] = true;
    }
    if pads.medium {
        buttons[if kick { 4 } else { 1 }] = true;
    }
    if pads.heavy && !pads.special {
        buttons[if kick { 5 } else { 2 }] = true;
    }
    let fresh = if pads.manual_motion {
        None
    } else if pads.special && pads.heavy {
        Some(SpecialKind::Super)
    } else if pads.special && pads.down {
        Some(SpecialKind::Shoryuken)
    } else if pads.special && (pads.forward || pads.back) {
        Some(SpecialKind::Tatsumaki)
    } else if pads.special {
        Some(SpecialKind::Hadoken)
    } else {
        None
    };
    let penalized = penalty && matches!(fresh, Some(SpecialKind::Hadoken | SpecialKind::Super));
    FrameOut {
        buttons,
        special: take_special(state, fresh, penalized),
    }
}

fn take_special(
    state: &mut SideState,
    fresh: Option<SpecialKind>,
    penalized: bool,
) -> Option<SpecialCommit> {
    if let Some(kind) = fresh {
        if !penalized {
            state.delay = 0;
            state.pending = None;
            return Some(SpecialCommit {
                kind,
                penalized: false,
                startup_extra: 0,
            });
        }
        if state.pending != Some(kind) {
            state.delay = 2;
            state.pending = Some(kind);
            return None;
        }
    }
    if state.delay > 0 {
        state.delay -= 1;
        if state.delay == 0 {
            return state.pending.take().map(|kind| SpecialCommit {
                kind,
                penalized: true,
                startup_extra: 2,
            });
        }
    }
    None
}

/// 20% off when the modern penalty applies. Manual motions stay at `base`.
pub fn scaled_damage(base: u16, penalized: bool) -> u16 {
    if penalized {
        (u32::from(base) * 4 / 5) as u16
    } else {
        base
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEntry {
    Fm { cmd: u8 },
    Music { cmd: u8 },
    Adpcm { cmd: u8 },
}

impl SoundEntry {
    pub const fn latch(self) -> u8 {
        match self {
            SoundEntry::Fm { cmd } | SoundEntry::Music { cmd } | SoundEntry::Adpcm { cmd } => cmd,
        }
    }
}

/// Service-test commands: every FM effect `0x00..=0x27`, the music specials
/// the sound-test run writes, and every ADPCM command `0x80..=0xbf`.
pub fn sound_test_catalog() -> Vec<SoundEntry> {
    let mut entries = Vec::with_capacity(40 + 5 + 64);
    for cmd in 0x00u8..=0x27 {
        entries.push(SoundEntry::Fm { cmd });
    }
    for cmd in [0x28, 0x33, 0x35, 0x3d, 0x3f] {
        entries.push(SoundEntry::Music { cmd });
    }
    for cmd in 0x80u8..=0xbf {
        entries.push(SoundEntry::Adpcm { cmd });
    }
    entries
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Playback {
    Native {
        latch: u8,
    },
    /// The Jukebox's track, a file name as the settings give it.
    Track {
        file: String,
    },
}

#[derive(Debug, Clone)]
pub struct SoundTest {
    index: usize,
    entries: Vec<SoundEntry>,
}

impl SoundTest {
    pub fn open() -> Self {
        Self {
            index: 0,
            entries: sound_test_catalog(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn selected(&self) -> SoundEntry {
        self.entries[self.index]
    }

    pub fn move_by(&mut self, delta: isize) {
        let n = self.entries.len() as isize;
        let next = (self.index as isize + delta).rem_euclid(n);
        self.index = next as usize;
    }

    /// Music follows the jukebox. Effects and voices always use the latch.
    pub fn audition(&self, jukebox: &Jukebox, stage: Option<&str>, fighter: Option<&str>) -> Playback {
        let entry = self.selected();
        if let SoundEntry::Music { .. } = entry {
            if let Some(track) = jukebox.resolve(stage, fighter, None) {
                return Playback::Track { file: track.file };
            }
        }
        Playback::Native {
            latch: entry.latch(),
        }
    }
}

/// The host smoke check's controller and jukebox line. Switches all three
/// schemes, confirms no track set means native music, and confirms the
/// Jukebox resolves a fighter's track, then a stage's, then the global one.
pub fn host_smoke_line() -> (bool, String) {
    let mut session = ControlSession::new(ControlScheme::Classic, ControlScheme::Pneumatic, false);
    let mut classic = FramePads::default();
    classic.classic[0] = true;
    let mut pneumatic = FramePads::default();
    pneumatic.punch = true;
    let (p1, p2) = session.step(&classic, &pneumatic);
    let classic_ok = p1.buttons == [true, false, false, false, false, false] && p2.buttons[0];
    session.p1 = ControlScheme::Modern;
    let mut modern = FramePads::default();
    modern.special = true;
    modern.down = true;
    let (p1, _) = session.step(&modern, &FramePads::default());
    let modern_ok = p1.special.map(|s| s.kind) == Some(SpecialKind::Shoryuken);

    let mut jukebox = Jukebox::default();
    let native = jukebox.resolve(Some("retsu"), Some("Ryu"), Some("Ken")).is_none();
    jukebox.set_global("any.ogg");
    jukebox.set_stage("retsu", "retsu.ogg");
    jukebox.set_fighter("Ken", "ken.flac");
    let present = jukebox.resolve(Some("retsu"), Some("Ryu"), Some("Ken")).is_some_and(|t| t.file == "ken.flac")
        && jukebox.resolve(Some("retsu"), Some("Ryu"), None).is_some_and(|t| t.file == "retsu.ogg")
        && jukebox.resolve(None, None, None).is_some_and(|t| t.file == "any.ogg");
    let ok = classic_ok && modern_ok && native && present;
    let line = format!(
        "jukebox and controls: classic/pneumatic/modern dispatch, native when no track is set, character then stage then global ({})",
        if ok { "ok" } else { "failed" }
    );
    (ok, line)
}
