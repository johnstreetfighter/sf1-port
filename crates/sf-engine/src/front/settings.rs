//! Settings: the new settings file (video, sound, the ROM folder, the 1987
//! cabinet's DIP words) and the framed Settings screen with its tabs.
//!
//! Gameplay and roster live in the match-config file (`MatchConfig`,
//! difficulty and round time included); bindings stay in the bindings
//! file. The screen edits copies of both and hands them back when it is
//! left; the game binary writes the files.

use std::path::{Path, PathBuf};

use sf_video::compositor::{fallback_text, ink, RomFont};
use sf_video::era_hud::GaugeArt;
use sf_video::render::{Frame, HEIGHT, WIDTH};

use super::tune::TypedKey;
use super::{MenuItem, Transition};
use crate::config::{MatchConfig, RulesetPreset};
use crate::jukebox::{Jukebox, Section as JukeboxSection};
use crate::moves::InputNotation;
use crate::netplay::protocol::{tag_from_str, PLAYER_TAG_CHARS};
use crate::roster::{BanFormat, MenuInput, RosterSettings};

/// The settings file's contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The windowed vertical scale, 1 to [`MAX_SCALE`]; the Video tab offers
    /// up to the largest that fits the monitor ([`DisplayInfo::max_scale`]).
    pub scale: u8,
    /// A borderless window at the monitor's size, the picture centred on
    /// black (the Video tab's `MODE`).
    pub fullscreen: bool,
    /// Present on the display's vsync (the Video tab's `VSYNC`); off shows
    /// frames as soon as they are ready, the game on a 60 Hz clock.
    pub vsync: bool,
    /// Milliseconds to wait after each vblank before stepping the game
    /// (the Video tab's `FRAME DELAY`), 0 to [`FRAME_DELAY_CAP_MS`]: the
    /// delay saved. The delay used is clamped to the cap for the look
    /// ([`Settings::frame_delay`]), so a delay set with `CRT LOOK` off is
    /// kept while a look is on.
    pub frame_delay_ms: u8,
    /// How the era gauges are drawn (the Video tab's `GAUGE ART`). A
    /// player's own look: never in the match config or the netplay handshake.
    pub gauge_art: GaugeArt,
    /// The picture's shape on the window (the Video tab's `ASPECT`).
    pub aspect: Aspect,
    /// Volumes, 0 to 10.
    pub master: u8,
    pub fm: u8,
    pub samples: u8,
    /// The overhaul announcer's chip (ADR 0011), under `master`; 0 is off.
    /// Local presentation: never in the synced `MatchConfig`.
    pub announcer: u8,
    /// A ROM folder or archive to try after the standard locations.
    pub rom_folder: Option<String>,
    /// The 1987 cabinet's DIP words.
    pub cabinet_dsw1: u16,
    pub cabinet_dsw2: u16,
    /// What drives each side (the Controls tab's `P1 DEVICE`, `P2 DEVICE`).
    pub devices: [Device; 2],
    /// Each menu's last cursor row: Main Menu, Single Player, Multi Player,
    /// Training. A menu opens where it was left.
    pub menu_positions: [u8; 4],
    /// Training's `HOLD COMBO` (the pause box's row): the last combo count
    /// stays on screen until the next combo. Off by default.
    pub hold_combo: bool,
    /// Training's `FILL METERS` (the Training settings box's row, spec #238,
    /// TR7): every era gauge's resource is held full in a Training fight.
    /// Off by default; a file from before it loads off.
    pub fill_meters: bool,
    /// The Controls tab's `MOVE INPUTS` (spec #226, ML2): how the move list
    /// writes an input. A player's own choice: never in the match config or
    /// the netplay handshake.
    pub move_inputs: InputNotation,
    /// The Video tab's `CRT LOOK` (spec #203): [`LOOK_OFF`] or a look's id
    /// from the host's catalogue. The engine never checks it against the
    /// catalogue; the host does, and an unknown id falls back to
    /// [`DEFAULT_LOOK`] there.
    pub crt_look: String,
    /// The Jukebox's tracks (ticket #200): the audio files that play in real
    /// fights in place of the native theme, set for the whole game, a stage
    /// or a fighter. Local presentation: never in the synced `MatchConfig`.
    /// The file keeps them in the `[jukebox]`, `[jukebox.stages]` and
    /// `[jukebox.fighters]` tables, which a default file leaves out.
    pub jukebox: Jukebox,
    /// The local player's lobby tag, up to twelve uppercase ASCII characters.
    pub player_tag: String,
    /// AUTO estimates a delay; fixed choices are zero through four frames.
    pub netplay_input_delay: InputDelaySetting,
    /// The rollback window offered to peers, one through eight frames.
    pub netplay_max_rollback: u8,
    /// Hide Wi-Fi rooms and refuse Wi-Fi challengers when hosting.
    pub wired_only: bool,
    /// Local telemetry presentation, never part of the synced match rules.
    pub netplay_hud: NetplayHudMode,
    /// The local UDP listening port, 1024 through 65535.
    pub netplay_port: u16,
    /// An optional community room directory; absent for LAN/direct play.
    pub lobby_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputDelaySetting {
    #[default]
    Auto,
    Frames(u8),
}

impl InputDelaySetting {
    pub const ALL: [Self; 6] = [Self::Auto, Self::Frames(0), Self::Frames(1), Self::Frames(2), Self::Frames(3), Self::Frames(4)];

    pub const fn frames(self) -> Option<u8> {
        match self {
            Self::Auto => None,
            Self::Frames(frames) => Some(frames),
        }
    }

    fn toml_value(self) -> String {
        match self {
            Self::Auto => "\"auto\"".to_owned(),
            Self::Frames(frames) => frames.to_string(),
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Auto => "AUTO".to_owned(),
            Self::Frames(frames) => format!("{frames}F"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NetplayHudMode {
    #[default]
    Compact,
    Full,
    Off,
}

impl NetplayHudMode {
    pub const ALL: [Self; 3] = [Self::Compact, Self::Full, Self::Off];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "COMPACT",
            Self::Full => "FULL",
            Self::Off => "OFF",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Full => "full",
            Self::Off => "off",
        }
    }
}

/// The largest `scale` the settings file takes. The Video tab offers only
/// what fits the monitor; this is the ceiling for any screen.
pub const MAX_SCALE: u8 = 20;

/// The largest `FRAME DELAY`, in milliseconds, with `CRT LOOK` off: the
/// largest delay at which the sharp picture kept the ship bar in both sweeps
/// (`sf-game smoke --frame-delay-sweep --sharp`) on the user's Radeon 780M
/// at 3200×2000, 120 Hz, on a quiet machine (2026-11-05, ticket #214;
/// design section 9 records the runs). Also the most `settings.toml` keeps.
pub const FRAME_DELAY_CAP_MS: u8 = 4;

/// The largest `FRAME DELAY` with any look on: no delay above 0 kept the
/// ship bar through `ARCADE CABINET` (1 ms held 58.08-58.50 game fps over
/// four runs), so the row shows `0MS`, drawn dim (user, 2026-11-05).
pub const FRAME_DELAY_CAP_LOOK_MS: u8 = 0;

/// The `FRAME DELAY` cap for the look `look` ([`LOOK_OFF`] or a look's id).
pub fn frame_delay_cap(look: &str) -> u8 {
    if look == LOOK_OFF { FRAME_DELAY_CAP_MS } else { FRAME_DELAY_CAP_LOOK_MS }
}

/// What the host tells the screens about the display: its refresh rate (the
/// Video tab's notice line) and the largest window scale that fits it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayInfo {
    /// The monitor's refresh rate in millihertz, when it says.
    pub refresh_mhz: Option<u32>,
    pub max_scale: u8,
}

impl Default for DisplayInfo {
    /// Nothing known: no refresh rate, and the old range of 1 to 4.
    fn default() -> Self {
        DisplayInfo { refresh_mhz: None, max_scale: 4 }
    }
}

/// The Video tab's notice line for a display at `refresh_mhz`, in lines of
/// at most 44 cells: what the refresh rate means for the game's motion.
/// Empty when the monitor does not say.
pub fn refresh_notice(refresh_mhz: Option<u32>, vsync: bool) -> Vec<String> {
    let Some(mhz) = refresh_mhz.filter(|&m| m > 0) else {
        return Vec::new();
    };
    let hz = f64::from(mhz) / 1000.0;
    let label = format!("{}HZ", hz.round() as u32);
    if !vsync {
        return vec![format!("{label}, VSYNC OFF: FRAMES SHOWN AS SOON"), "AS READY, TEARING POSSIBLE".to_owned()];
    }
    // The pacing rule's: within 1% of a whole multiple of 60.
    let n = (hz / 60.0).round();
    if n >= 1.0 && (hz - n * 60.0).abs() <= 0.01 * n * 60.0 {
        let times = match n as u32 {
            1 => "ONCE".to_owned(),
            2 => "TWICE".to_owned(),
            n => format!("{n} TIMES"),
        };
        vec![format!("{label}: EVERY FRAME SHOWN {times}")]
    } else {
        vec![format!("{label}: SET YOUR DISPLAY TO 120 OR 60"), "FOR SMOOTH MOTION".to_owned()]
    }
}

/// `CRT LOOK`'s sharp picture, first in every look list.
pub const LOOK_OFF: &str = "off";
/// The look a first launch and an unknown `crt_look` get: `ARCADE CABINET`.
pub const DEFAULT_LOOK: &str = "arcade_cabinet";

/// One entry of the look list as the screens show it: the id stored in
/// `crt_look` and the name drawn. The host's catalogue supplies the list
/// (ADR-0005: screens never read the shaders folder).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookName {
    pub id: String,
    pub name: String,
}

/// The look list when the host gives none: `OFF` and `ARCADE CABINET`, the
/// built-in looks' names as the host's catalogue has them.
pub fn default_looks() -> Vec<LookName> {
    [(LOOK_OFF, "OFF"), (DEFAULT_LOOK, "ARCADE CABINET")]
        .into_iter()
        .map(|(id, name)| LookName { id: id.to_owned(), name: name.to_owned() })
        .collect()
}

/// The shape of the picture: the 1987 cabinet's 4:3 tube showed each game
/// pixel 7/9 as wide as it was tall (ADR 0010). The vertical scale is a
/// whole number in every mode; the host's `picture_rect` places it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Aspect {
    /// Width is height times 4/3, the one non-integer scale.
    #[default]
    True4x3,
    /// Each pixel a whole number wide, the nearest to 7/9 of its height.
    Integer4x3,
    /// Each pixel as wide as it is tall.
    Square,
}

impl Aspect {
    pub const ALL: [Aspect; 3] = [Aspect::True4x3, Aspect::Integer4x3, Aspect::Square];

    /// The Video tab value, in the ROM font's character set.
    pub const fn label(self) -> &'static str {
        match self {
            Aspect::True4x3 => "TRUE 4:3",
            Aspect::Integer4x3 => "4:3 INTEGER",
            Aspect::Square => "SQUARE PIXELS",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Aspect::True4x3 => "true_4_3",
            Aspect::Integer4x3 => "integer_4_3",
            Aspect::Square => "square",
        }
    }

    pub fn from_slug(s: &str) -> Option<Aspect> {
        Aspect::ALL.into_iter().find(|a| a.slug() == s)
    }

    /// The aspect after this one, wrapping.
    pub fn next(self) -> Aspect {
        Aspect::ALL[(self as usize + 1) % Aspect::ALL.len()]
    }

    /// The aspect before this one, wrapping.
    pub fn prev(self) -> Aspect {
        Aspect::ALL[(self as usize + Aspect::ALL.len() - 1) % Aspect::ALL.len()]
    }
}

/// The settings file's names for [`Settings::menu_positions`].
const MENU_KEYS: [&str; 4] = ["main_menu", "single_player_menu", "multi_player_menu", "training_menu"];

/// What drives a side. `AUTO`, the default, is connection order: P1 takes
/// the first pad seen and P2 the second, and each side is on the keyboard
/// until its pad is seen. A side whose pad is lost idles until a pad is
/// seen again; the keyboard never takes over on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    Auto,
    Keyboard,
    Pad1,
    Pad2,
}

impl Device {
    pub const ALL: [Device; 4] = [Device::Auto, Device::Keyboard, Device::Pad1, Device::Pad2];

    pub const fn label(self) -> &'static str {
        match self {
            Device::Auto => "AUTO",
            Device::Keyboard => "KEYBOARD",
            Device::Pad1 => "PAD 1",
            Device::Pad2 => "PAD 2",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Device::Auto => "auto",
            Device::Keyboard => "keyboard",
            Device::Pad1 => "pad1",
            Device::Pad2 => "pad2",
        }
    }

    pub fn from_slug(s: &str) -> Option<Device> {
        Device::ALL.into_iter().find(|d| d.slug() == s)
    }
}

impl Default for Settings {
    /// First launch (spec #203, amending front-end story 5): borderless
    /// fullscreen with `ARCADE CABINET` at true 4:3, vsync on, no frame
    /// delay, every volume 10, factory DIP words. 3× is the scale
    /// `WINDOWED` opens at.
    fn default() -> Self {
        Settings {
            scale: 3,
            fullscreen: true,
            vsync: true,
            frame_delay_ms: 0,
            gauge_art: GaugeArt::Ruled,
            aspect: Aspect::default(),
            master: 10,
            fm: 10,
            samples: 10,
            announcer: 10,
            rom_folder: None,
            cabinet_dsw1: 0xdfff,
            cabinet_dsw2: 0xffff,
            devices: [Device::Auto; 2],
            menu_positions: [0; 4],
            hold_combo: false,
            fill_meters: false,
            move_inputs: InputNotation::default(),
            crt_look: DEFAULT_LOOK.to_owned(),
            jukebox: Jukebox::default(),
            player_tag: "PLAYER".to_owned(),
            netplay_input_delay: InputDelaySetting::Auto,
            netplay_max_rollback: 8,
            wired_only: false,
            netplay_hud: NetplayHudMode::Compact,
            netplay_port: 7000,
            lobby_url: None,
        }
    }
}

impl Settings {
    /// The frame delay in use: the saved delay, clamped to the cap for the
    /// look ([`frame_delay_cap`]).
    pub fn frame_delay(&self) -> u8 {
        self.frame_delay_ms.min(frame_delay_cap(&self.crt_look))
    }
}

/// `text` as a quoted string in the file, `\` and `"` escaped (a drop-in's
/// path or the ROM folder may hold either, or a `#`).
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if c == '\\' || c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// A quoted value's text ([`escaped`] reversed), or `None` when `v` is not
/// one quoted string. `\\` and `\"` are the escapes; any other `\`, and a
/// `\` before the closing quote, is kept as it is, so a file from before
/// the escapes (`rom_folder = "D:\roms\"`) still reads.
fn unquoted(v: &str) -> Option<String> {
    let mut chars = v.strip_prefix('"')?.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => return chars.as_str().trim().is_empty().then_some(out),
            '\\' => match chars.clone().next() {
                Some('"') if chars.as_str()[1..].trim().is_empty() => out.push('\\'),
                Some(next @ ('\\' | '"')) => {
                    chars.next();
                    out.push(next);
                }
                _ => out.push('\\'),
            },
            c => out.push(c),
        }
    }
    None
}

/// `line` up to its comment: the first `#` outside a quoted string.
fn without_comment(line: &str) -> &str {
    let (mut quoted, mut escape) = (false, false);
    for (i, c) in line.char_indices() {
        match c {
            _ if escape => escape = false,
            '\\' if quoted => escape = true,
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..i],
            _ => {}
        }
    }
    line
}

/// `%APPDATA%\sf1-port\settings.toml`.
pub fn default_settings_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("sf1-port").join("settings.toml")
}

impl Settings {
    pub fn to_toml(&self) -> String {
        format!(
            "# sf1-port game settings\n\
             scale = {}\n\
             fullscreen = {}\n\
             vsync = {}\n\
             frame_delay_ms = {}\n\
             gauge_art = \"{}\"\n\
             aspect = \"{}\"\n\
             master = {}\n\
             fm = {}\n\
             samples = {}\n\
             announcer = {}\n\
             rom_folder = {}\n\
             cabinet_dsw1 = 0x{:04x}\n\
             cabinet_dsw2 = 0x{:04x}\n\
             p1_device = \"{}\"\n\
             p2_device = \"{}\"\n\
             hold_combo = {}\n\
             fill_meters = {}\n\
             move_inputs = \"{}\"\n\
             crt_look = {}\n\
             player_tag = {}\n\
             netplay_input_delay = {}\n\
             netplay_max_rollback = {}\n\
             wired_only = {}\n\
             netplay_hud = \"{}\"\n\
             netplay_port = {}\n\
             lobby_url = {}\n",
            self.scale,
            self.fullscreen,
            self.vsync,
            self.frame_delay_ms,
            self.gauge_art.slug(),
            self.aspect.slug(),
            self.master,
            self.fm,
            self.samples,
            self.announcer,
            escaped(self.rom_folder.as_deref().unwrap_or("")),
            self.cabinet_dsw1,
            self.cabinet_dsw2,
            self.devices[0].slug(),
            self.devices[1].slug(),
            self.hold_combo,
            self.fill_meters,
            self.move_inputs.slug(),
            escaped(&self.crt_look),
            escaped(&self.player_tag),
            self.netplay_input_delay.toml_value(),
            self.netplay_max_rollback,
            self.wired_only,
            self.netplay_hud.slug(),
            self.netplay_port,
            escaped(self.lobby_url.as_deref().unwrap_or("")),
        ) + &MENU_KEYS
            .iter()
            .zip(self.menu_positions)
            .map(|(k, v)| format!("{k} = {v}
"))
            .collect::<String>()
            + &self.jukebox.to_toml()
    }

    /// Parse the file; unknown keys and out-of-range values are refused.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        Self::parse(text, false).map(|(s, _)| s)
    }

    /// Parse the file as the game loads it: a problem with a line of the
    /// Jukebox's tables (a bad table name, a key without a quoted file, a
    /// key set twice) costs only that line, and comes back as a message
    /// beside the settings; every other problem is refused as in
    /// [`Settings::from_toml`].
    pub fn from_toml_lenient(text: &str) -> Result<(Self, Vec<String>), String> {
        Self::parse(text, true)
    }

    fn parse(text: &str, lenient: bool) -> Result<(Self, Vec<String>), String> {
        let mut s = Settings::default();
        let mut warnings = Vec::new();
        // The Jukebox's table the lines belong to, once a header names one;
        // `skipping` after a header that names none (leniently).
        let mut table: Option<JukeboxSection> = None;
        let mut skipping = false;
        for (i, raw) in text.lines().enumerate() {
            let line = without_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            let n = i + 1;
            if let Some(header) = line.strip_prefix('[') {
                let found = header
                    .strip_suffix(']')
                    .ok_or_else(|| format!("line {n}: expected ']' to end the table name"))
                    .and_then(|h| JukeboxSection::from_header(h).ok_or_else(|| format!("line {n}: unknown table '[{}]'", h.trim())));
                match found {
                    Ok(section) => (table, skipping) = (Some(section), false),
                    Err(e) if lenient => {
                        warnings.push(format!("{e}; its lines are skipped"));
                        (table, skipping) = (None, true);
                    }
                    Err(e) => return Err(e),
                }
                continue;
            }
            if skipping {
                continue;
            }
            if let Some(section) = table {
                let read = (|| {
                    let (name, file) = line.split_once('=').map(|(k, v)| (k.trim(), v.trim())).ok_or_else(|| format!("line {n}: expected 'key = value'"))?;
                    let name = if name.starts_with('"') { unquoted(name) } else { Some(name.to_owned()) };
                    let (name, file) = name.zip(unquoted(file)).ok_or_else(|| format!("line {n}: expected a name and a quoted file"))?;
                    s.jukebox.read_line(section, &name, &file).map_err(|e| format!("line {n}: {e}"))
                })();
                match read {
                    Ok(warning) => warnings.extend(warning.map(|w| format!("line {n}: {w}"))),
                    Err(e) if lenient => warnings.push(format!("{e}; the line is skipped")),
                    Err(e) => return Err(e),
                }
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .ok_or_else(|| format!("line {n}: expected 'key = value'"))?;
            let num = |max: u32| -> Result<u32, String> {
                let x = match v.strip_prefix("0x") {
                    Some(hex) => u32::from_str_radix(hex, 16),
                    None => v.parse(),
                }
                .map_err(|_| format!("line {n}: bad number for '{k}'"))?;
                (x <= max).then_some(x).ok_or_else(|| format!("line {n}: '{k}' out of range"))
            };
            match k {
                "scale" => s.scale = num(u32::from(MAX_SCALE))?.max(1) as u8,
                // A delay past the cap (the cap was higher once) is
                // clamped, never refused: the rest of the file still loads.
                "frame_delay_ms" => s.frame_delay_ms = num(u32::MAX)?.min(u32::from(FRAME_DELAY_CAP_MS)) as u8,
                "fullscreen" | "vsync" | "hold_combo" | "fill_meters" | "wired_only" => {
                    let on = match v {
                        "true" => true,
                        "false" => false,
                        _ => return Err(format!("line {n}: expected true or false for '{k}'")),
                    };
                    match k {
                        "fullscreen" => s.fullscreen = on,
                        "vsync" => s.vsync = on,
                        "hold_combo" => s.hold_combo = on,
                        "wired_only" => s.wired_only = on,
                        _ => s.fill_meters = on,
                    }
                }
                "gauge_art" => {
                    s.gauge_art = unquoted(v)
                        .as_deref()
                        .and_then(GaugeArt::from_slug)
                        .ok_or_else(|| format!("line {n}: expected \"ruled\", \"blocks\" or \"plate\" for '{k}'"))?;
                }
                "aspect" => {
                    s.aspect = unquoted(v)
                        .as_deref()
                        .and_then(Aspect::from_slug)
                        .ok_or_else(|| format!("line {n}: expected \"true_4_3\", \"integer_4_3\" or \"square\" for '{k}'"))?;
                }
                "move_inputs" => {
                    s.move_inputs = unquoted(v)
                        .as_deref()
                        .and_then(InputNotation::from_slug)
                        .ok_or_else(|| format!("line {n}: expected \"arrows\", \"numpad\" or \"words\" for '{k}'"))?;
                }
                "master" => s.master = num(10)? as u8,
                "fm" => s.fm = num(10)? as u8,
                "samples" => s.samples = num(10)? as u8,
                "announcer" => s.announcer = num(10)? as u8,
                "player_tag" => {
                    s.player_tag = unquoted(v)
                        .filter(|tag| tag_from_str(tag).is_some())
                        .ok_or_else(|| format!("line {n}: expected a tag of 1 to 12 uppercase letters, digits, spaces, '-' or '_'"))?;
                }
                "netplay_input_delay" => {
                    s.netplay_input_delay = if unquoted(v).as_deref() == Some("auto") {
                        InputDelaySetting::Auto
                    } else {
                        InputDelaySetting::Frames(num(4)? as u8)
                    };
                }
                "netplay_max_rollback" => {
                    let frames = num(8)?;
                    if frames == 0 {
                        return Err(format!("line {n}: '{k}' out of range"));
                    }
                    s.netplay_max_rollback = frames as u8;
                }
                "netplay_hud" => {
                    s.netplay_hud = match unquoted(v).as_deref() {
                        Some("compact") => NetplayHudMode::Compact,
                        Some("full") => NetplayHudMode::Full,
                        Some("off") => NetplayHudMode::Off,
                        _ => return Err(format!("line {n}: expected \"compact\", \"full\" or \"off\" for '{k}'")),
                    };
                }
                "netplay_port" => {
                    let port = num(65535)?;
                    if port < 1024 {
                        return Err(format!("line {n}: '{k}' out of range"));
                    }
                    s.netplay_port = port as u16;
                }
                "lobby_url" => {
                    let url = unquoted(v)
                        .ok_or_else(|| format!("line {n}: expected a quoted lobby URL"))?;
                    s.lobby_url = (!url.is_empty()).then_some(url);
                }
                "rom_folder" => {
                    let inner = unquoted(v).ok_or_else(|| format!("line {n}: expected a quoted folder"))?;
                    s.rom_folder = (!inner.is_empty()).then_some(inner);
                }
                "crt_look" => {
                    s.crt_look = unquoted(v)
                        .filter(|v| !v.is_empty())
                        .ok_or_else(|| format!("line {n}: expected a quoted look id for '{k}'"))?;
                }
                "cabinet_dsw1" => s.cabinet_dsw1 = num(0xffff)? as u16,
                "cabinet_dsw2" => s.cabinet_dsw2 = num(0xffff)? as u16,
                k if MENU_KEYS.contains(&k) => {
                    let slot = MENU_KEYS.iter().position(|m| *m == k).expect("a menu key");
                    s.menu_positions[slot] = num(7)? as u8;
                }
                "p1_device" | "p2_device" => {
                    let device = unquoted(v)
                        .as_deref()
                        .and_then(Device::from_slug)
                        .ok_or_else(|| format!("line {n}: expected \"auto\", \"keyboard\", \"pad1\" or \"pad2\" for '{k}'"))?;
                    s.devices[usize::from(k == "p2_device")] = device;
                }
                _ => return Err(format!("line {n}: unknown key '{k}'")),
            }
        }
        Ok((s, warnings))
    }

    /// Load `path`, or the defaults and a message saying why not.
    pub fn load_or_default(path: &Path) -> (Self, Option<String>) {
        match std::fs::read_to_string(path) {
            Ok(text) => match Self::from_toml_lenient(&text) {
                Ok((s, warnings)) if warnings.is_empty() => (s, None),
                Ok((s, warnings)) => (s, Some(format!("{}: {}", path.display(), warnings.join("; ")))),
                Err(e) => (
                    Self::default(),
                    Some(format!("{}: {e}; using the default settings", path.display())),
                ),
            },
            Err(e) => (
                Self::default(),
                Some(format!("cannot read {}: {e}; using the default settings", path.display())),
            ),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        std::fs::write(path, self.to_toml()).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
}

// ---------------------------------------------------------------------------
// The Settings screen.
// ---------------------------------------------------------------------------

/// The tabs, left to right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Gameplay,
    Roster,
    Video,
    Sound,
    Controls,
    Netplay,
}

const TABS: [Tab; 6] = [Tab::Gameplay, Tab::Roster, Tab::Video, Tab::Sound, Tab::Controls, Tab::Netplay];
const TAB_NAMES: [&str; 6] = ["GAMEPLAY", "ROSTER", "VIDEO", "SOUND", "CONTROLS", "NETPLAY"];

/// One core toggle row: its display name, its `MatchConfig` key, and for a
/// frame count its range.
struct ToggleRow {
    label: &'static str,
    key: &'static str,
    frames: Option<(u8, u8)>,
}

const TOGGLES: [ToggleRow; 20] = [
    ToggleRow { label: "POSITIVE EDGE", key: "positive_edge_attacks", frames: None },
    ToggleRow { label: "NEGATIVE EDGE SPECIALS", key: "negative_edge_specials", frames: None },
    ToggleRow { label: "RECOVERY BUFFER", key: "recovery_input_buffer", frames: None },
    ToggleRow { label: "RECOVERY WINDOW", key: "recovery_buffer_window", frames: Some((0, 30)) },
    ToggleRow { label: "LENIENT COMMANDS", key: "lenient_command_trackers", frames: None },
    ToggleRow { label: "COMMAND WINDOW", key: "command_tracker_window", frames: Some((1, 60)) },
    ToggleRow { label: "RELEASE GRACE", key: "command_release_grace", frames: Some((0, 30)) },
    ToggleRow { label: "WALK CANCEL", key: "interruptible_walk_cycle", frames: None },
    ToggleRow { label: "ANYTIME JUMP ATTACKS", key: "anytime_jump_attacks", frames: None },
    ToggleRow { label: "FAST PRE-JUMP", key: "fast_prejump_crouch", frames: None },
    ToggleRow { label: "HIGH LOW BLOCK", key: "standard_high_low_block", frames: None },
    ToggleRow { label: "BALANCED ROSTER", key: "balanced_roster", frames: None },
    ToggleRow { label: "TRUE COMBOS", key: "true_combos", frames: None },
    ToggleRow { label: "CANCELS", key: "cancels", frames: None },
    ToggleRow { label: "THROWS", key: "throws", frames: None },
    ToggleRow { label: "CHIP DEATH", key: "chip_death", frames: None },
    ToggleRow { label: "NO FIREBALL SLOWDOWN", key: "disable_fireball_slowdown", frames: None },
    ToggleRow { label: "REAL SECOND TIMER", key: "real_second_timer_and_draws", frames: None },
    ToggleRow { label: "FAIR CPU", key: "fair_cpu_and_sagat_scaling", frames: None },
    ToggleRow { label: "CPU REACTION", key: "cpu_reaction_delay", frames: Some((0, 30)) },
];

/// A toggle's current value, read from the configuration's toggles.
fn toggle_value(config: &MatchConfig, key: &str) -> u8 {
    let t = &config.toggles;
    match key {
        "positive_edge_attacks" => t.positive_edge_attacks as u8,
        "negative_edge_specials" => t.negative_edge_specials as u8,
        "recovery_input_buffer" => t.recovery_input_buffer as u8,
        "recovery_buffer_window" => t.recovery_buffer_window,
        "lenient_command_trackers" => t.lenient_command_trackers as u8,
        "command_tracker_window" => t.command_tracker_window,
        "command_release_grace" => t.command_release_grace,
        "interruptible_walk_cycle" => t.interruptible_walk_cycle as u8,
        "anytime_jump_attacks" => t.anytime_jump_attacks as u8,
        "fast_prejump_crouch" => t.fast_prejump_crouch as u8,
        "standard_high_low_block" => t.standard_high_low_block as u8,
        "balanced_roster" => t.balanced_roster as u8,
        "true_combos" => t.true_combos as u8,
        "cancels" => t.cancels as u8,
        "throws" => t.throws as u8,
        "chip_death" => t.chip_death as u8,
        "disable_fireball_slowdown" => t.disable_fireball_slowdown as u8,
        "real_second_timer_and_draws" => t.real_second_timer_and_draws as u8,
        "fair_cpu_and_sagat_scaling" => t.fair_cpu_and_sagat_scaling as u8,
        "cpu_reaction_delay" => t.cpu_reaction_delay,
        _ => 0,
    }
}

/// One row of a tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Preset,
    /// Index into the core toggles.
    Toggle(usize),
    Difficulty,
    RoundTime,
    FullCast,
    FourCountries,
    BanList,
    WhatIfStyles,
    /// `WINDOW SCALE`.
    Scale,
    /// `MODE`: fullscreen or windowed.
    Fullscreen,
    GaugeArt,
    Aspect,
    CrtLook,
    /// Opens the tuning screen; disabled while `CRT LOOK` is `OFF`.
    TuneLook,
    Vsync,
    FrameDelay,
    Master,
    Announcer,
    Fm,
    Samples,
    Device(usize),
    Scheme(usize),
    /// `MOVE INPUTS`: how the move list writes an input.
    MoveInputs,
    /// Opens the rebind walk.
    Rebind(MenuItem),
    /// `JUKEBOX`: opens the Jukebox screen.
    Jukebox,
    PlayerTag,
    InputDelay,
    MaxRollback,
    WiredOnly,
    NetplayHud,
    UdpPort,
    /// A row whose project has not landed: drawn dim and skipped.
    Greyed(&'static str),
    /// A group's name over its rows (the Video tab's): skipped.
    Heading(&'static str),
    ResetTab,
}

impl Row {
    fn live(self) -> bool {
        !matches!(self, Row::Greyed(_) | Row::Heading(_))
    }
}

/// The rows of `tab`, top to bottom.
pub fn rows(tab: Tab) -> Vec<Row> {
    let mut rows = match tab {
        Tab::Gameplay => {
            let mut r = vec![Row::Preset];
            r.extend((0..TOGGLES.len()).map(Row::Toggle));
            r.extend([Row::Difficulty, Row::RoundTime]);
            r
        }
        Tab::Roster => vec![
            Row::FullCast,
            Row::FourCountries,
            Row::BanList,
            Row::WhatIfStyles,
            Row::Greyed("BONUS CHALLENGERS"),
            Row::Greyed("SECRET RIVALS"),
        ],
        Tab::Video => vec![
            Row::Heading("DISPLAY"),
            Row::Fullscreen,
            Row::Scale,
            Row::GaugeArt,
            Row::Heading("MONITOR"),
            Row::Aspect,
            Row::CrtLook,
            Row::TuneLook,
            Row::Heading("MOTION"),
            Row::Vsync,
            Row::FrameDelay,
        ],
        Tab::Sound => vec![Row::Master, Row::Announcer, Row::Fm, Row::Samples, Row::Jukebox],
        Tab::Controls => vec![
            Row::Device(0),
            Row::Device(1),
            Row::Scheme(0),
            Row::Scheme(1),
            Row::MoveInputs,
            Row::Rebind(MenuItem::RebindP1),
            Row::Rebind(MenuItem::RebindP2),
            Row::Rebind(MenuItem::RebindMenu),
        ],
        Tab::Netplay => vec![Row::PlayerTag, Row::InputDelay, Row::MaxRollback, Row::WiredOnly, Row::NetplayHud, Row::UdpPort],
    };
    rows.push(Row::ResetTab);
    rows
}

/// The rows the screen shows at once (it scrolls with the cursor).
pub const VISIBLE_ROWS: usize = 16;
/// The first row's text row, the label's column, the value's last column.
pub const FIRST_ROW: usize = 7;
const LABEL_COL: usize = 4;
const VALUE_END_COL: usize = 44;
/// The tab names' row.
pub const TAB_ROW: usize = 4;
/// The notice line's row, under the rows.
pub const NOTICE_ROW: usize = FIRST_ROW + VISIBLE_ROWS + 1;

#[derive(Clone, Debug, PartialEq, Eq)]
struct NetplayEntry {
    row: Row,
    text: Vec<char>,
    at: usize,
    problem: Option<String>,
}

impl NetplayEntry {
    fn limit(&self) -> usize {
        if self.row == Row::UdpPort { 5 } else { 12 }
    }

    fn alphabet(&self) -> &'static str {
        if self.row == Row::UdpPort { "0123456789" } else { PLAYER_TAG_CHARS }
    }
}

/// The Settings screen: copies of the configuration and the settings,
/// handed back to the session when the screen is left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsScreen {
    config: MatchConfig,
    settings: Settings,
    tab: usize,
    cursor: [usize; 6],
    reset_bindings: bool,
    /// `CRT LOOK`'s list, in order.
    looks: Vec<LookName>,
    /// The notice line (a look that failed to load), until the next change.
    notice: Option<String>,
    /// The display, for the refresh-rate notice and the scale's range.
    display: DisplayInfo,
    /// The chosen look, when it failed to load this run: the screen shows
    /// and uses `OFF`, while the settings keep the look ([`look_in_use`]).
    failed_look: Option<String>,
    entry: Option<NetplayEntry>,
}

/// The look drawn this run: `chosen` (the settings' `crt_look`), or `OFF`
/// when it is `failed`, the look that failed to load. The fallback is never
/// saved: the file keeps the player's choice, and the next launch tries it
/// again.
pub fn look_in_use<'a>(chosen: &'a str, failed: Option<&str>) -> &'a str {
    if failed == Some(chosen) { LOOK_OFF } else { chosen }
}

impl SettingsScreen {
    pub fn open(config: MatchConfig, settings: Settings) -> Self {
        SettingsScreen {
            config,
            settings,
            tab: 0,
            cursor: TABS.map(|tab| rows(tab).iter().position(|r| r.live()).unwrap_or(0)),
            reset_bindings: false,
            looks: default_looks(),
            notice: None,
            display: DisplayInfo::default(),
            failed_look: None,
            entry: None,
        }
    }

    /// Builder: the look that failed to load this run, if any (the
    /// session's, as the screen opens).
    pub fn with_failed_look(mut self, failed: Option<String>) -> Self {
        self.failed_look = failed;
        self
    }

    /// The look that failed to load this run, while it is still the chosen
    /// one.
    pub fn failed_look(&self) -> Option<&str> {
        self.failed_look.as_deref().filter(|f| *f == self.settings.crt_look)
    }

    /// The look drawn: `CRT LOOK`'s choice, or `OFF` while it failed.
    pub fn look(&self) -> &str {
        look_in_use(&self.settings.crt_look, self.failed_look.as_deref())
    }

    /// The frame delay in use, clamped to the cap for the look drawn.
    fn frame_delay(&self) -> u8 {
        self.settings.frame_delay_ms.min(frame_delay_cap(self.look()))
    }

    /// The display as the host reports it (each frame, through the session).
    pub fn set_display(&mut self, display: DisplayInfo) {
        self.display = display;
    }

    /// Builder: `CRT LOOK`'s list (the host's catalogue, through the
    /// session). An empty list keeps the default one.
    pub fn with_looks(mut self, looks: Vec<LookName>) -> Self {
        if !looks.is_empty() {
            self.looks = looks;
        }
        self
    }

    /// The look `id` failed to load: `CRT LOOK` shows and uses `OFF` for
    /// this run (the settings keep `id`) and the notice line shows `notice`.
    pub fn look_failed(&mut self, id: &str, notice: &str) {
        if self.settings.crt_look == id {
            self.failed_look = Some(id.to_owned());
        }
        self.notice = Some(notice.to_owned());
    }

    /// The Jukebox screen was left: these are the tracks now. They are saved
    /// with the other settings when this screen is left.
    pub fn set_jukebox(&mut self, jukebox: Jukebox) {
        self.settings.jukebox = jukebox;
    }

    /// `CRT LOOK` is now the look `id` (a look just saved).
    pub fn set_look(&mut self, id: &str) {
        self.settings.crt_look = id.to_owned();
        self.failed_look = None;
        self.notice = None;
    }

    /// Whether `TUNE CRT LOOK` opens: not while the look drawn is `OFF`.
    pub fn tune_enabled(&self) -> bool {
        self.look() != LOOK_OFF
    }

    /// Whether `row` is drawn dim and takes no left or right while live:
    /// `TUNE CRT LOOK` with the look `OFF`, `FRAME DELAY` while its cap for
    /// the look is 0.
    pub fn dim(&self, row: Row) -> bool {
        match row {
            Row::TuneLook => !self.tune_enabled(),
            Row::FrameDelay => frame_delay_cap(self.look()) == 0,
            Row::WhatIfStyles => !self.config.era_select_enabled(),
            _ => false,
        }
    }

    /// The notice line, when one is showing.
    pub fn notice(&self) -> Option<&str> {
        match &self.entry {
            Some(entry) => entry.problem.as_deref(),
            None => self.notice.as_deref(),
        }
    }

    pub fn name(&self) -> &'static str {
        "settings"
    }

    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn tab(&self) -> Tab {
        TABS[self.tab]
    }

    /// The row under the cursor.
    pub fn row(&self) -> Row {
        rows(self.tab())[self.cursor[self.tab]]
    }

    /// The row's name as drawn.
    pub fn label(&self, row: Row) -> &'static str {
        match row {
            Row::Preset => "PRESET",
            Row::Toggle(i) => TOGGLES[i].label,
            Row::Difficulty => "DIFFICULTY",
            Row::RoundTime => "ROUND TIME",
            Row::FullCast => "FULL CAST",
            Row::FourCountries => "FOUR COUNTRIES",
            Row::BanList => "BAN LIST",
            Row::WhatIfStyles => "WHAT-IF STYLES",
            Row::Scale => "WINDOW SCALE",
            Row::Fullscreen => "MODE",
            Row::Aspect => "ASPECT",
            Row::GaugeArt => "GAUGE ART",
            Row::CrtLook => "CRT LOOK",
            Row::TuneLook => "TUNE CRT LOOK >",
            Row::Vsync => "VSYNC",
            Row::FrameDelay => "FRAME DELAY",
            Row::Master => "MASTER",
            Row::Announcer => "ANNOUNCER",
            Row::Fm => "FM",
            Row::Samples => "SAMPLES",
            Row::Device(0) => "P1 DEVICE",
            Row::Device(_) => "P2 DEVICE",
            Row::MoveInputs => "MOVE INPUTS",
            Row::Scheme(0) => "P1 SCHEME",
            Row::Scheme(_) => "P2 SCHEME",
            Row::Rebind(item) => item.label(),
            Row::Jukebox => MenuItem::Jukebox.label(),
            Row::PlayerTag => "PLAYER TAG",
            Row::InputDelay => "INPUT DELAY",
            Row::MaxRollback => "MAX ROLLBACK",
            Row::WiredOnly => "WIRED ONLY",
            Row::NetplayHud => "NETPLAY HUD",
            Row::UdpPort => "UDP PORT",
            Row::Greyed(name) | Row::Heading(name) => name,
            Row::ResetTab => "RESET TAB",
        }
    }

    /// The most cells `row`'s value takes: from a cell past its name to the
    /// value column's end.
    pub fn value_cells(&self, row: Row) -> usize {
        VALUE_END_COL.saturating_sub(LABEL_COL + self.label(row).chars().count() + 1)
    }

    /// The row's value as drawn: cut with `..` past [`Self::value_cells`]
    /// (a long drop-in's name, or an id the look list does not hold).
    pub fn value(&self, row: Row) -> String {
        super::draw::fit(&self.full_value(row), self.value_cells(row))
    }

    /// The row's value in full.
    fn full_value(&self, row: Row) -> String {
        if let Some(entry) = self.entry.as_ref().filter(|entry| entry.row == row) {
            return entry.text.iter().collect();
        }
        let on = |b: bool| if b { "ON" } else { "OFF" }.to_string();
        match row {
            Row::Preset => self.config.preset.display_name().to_ascii_uppercase(),
            Row::Toggle(i) => {
                let v = toggle_value(&self.config, TOGGLES[i].key);
                match TOGGLES[i].frames {
                    Some(_) => format!("{v}F"),
                    None => on(v != 0),
                }
            }
            Row::Difficulty => self.config.difficulty.to_string(),
            Row::RoundTime => self.config.round_time.to_string(),
            Row::FullCast => on(self.config.roster.full_cast),
            Row::FourCountries => on(self.config.roster.four_country),
            Row::WhatIfStyles => on(self.config.roster.what_if_styles),
            Row::BanList => match self.config.roster.ban {
                BanFormat::Unrestricted => "UNRESTRICTED",
                BanFormat::StandardTournament => "STANDARD TOURNAMENT",
                BanFormat::LowTierNoShoto => "LOW TIER",
            }
            .to_string(),
            Row::Scale => format!("{}X", self.settings.scale),
            Row::Fullscreen => if self.settings.fullscreen { "FULLSCREEN" } else { "WINDOWED" }.to_string(),
            Row::Vsync => on(self.settings.vsync),
            Row::FrameDelay => format!("{}MS", self.frame_delay()),
            Row::Aspect => self.settings.aspect.label().to_string(),
            Row::GaugeArt => self.settings.gauge_art.label().to_string(),
            Row::CrtLook => match self.looks.iter().find(|l| l.id == self.look()) {
                Some(look) => look.name.clone(),
                None => self.look().to_ascii_uppercase(),
            },
            Row::Master => self.settings.master.to_string(),
            Row::Announcer => self.settings.announcer.to_string(),
            Row::Fm => self.settings.fm.to_string(),
            Row::Samples => self.settings.samples.to_string(),
            Row::Device(side) => self.settings.devices[side].label().to_string(),
            Row::MoveInputs => self.settings.move_inputs.label().to_string(),
            Row::Scheme(_) => "CLASSIC".to_string(),
            Row::PlayerTag => self.settings.player_tag.clone(),
            Row::InputDelay => self.settings.netplay_input_delay.label(),
            Row::MaxRollback => format!("{}F", self.settings.netplay_max_rollback),
            Row::WiredOnly => on(self.settings.wired_only),
            Row::NetplayHud => self.settings.netplay_hud.label().to_owned(),
            Row::UdpPort => self.settings.netplay_port.to_string(),
            Row::Rebind(_) | Row::TuneLook | Row::Jukebox | Row::Greyed(_) | Row::Heading(_) | Row::ResetTab => String::new(),
        }
    }

    /// Change `row` by one step (`delta` is +1 or -1, wrapping where the
    /// values cycle).
    fn change(&mut self, row: Row, delta: i32) {
        let step = |v: i32, lo: i32, hi: i32| (v + delta).clamp(lo, hi);
        match row {
            Row::Preset => {
                let all = RulesetPreset::ALL;
                let at = all.iter().position(|&p| p == self.config.preset).unwrap_or(0) as i32;
                let next = all[(at + delta).rem_euclid(all.len() as i32) as usize];
                // A preset brings its toggles and roster defaults; the
                // difficulty, round time and the select's fighters are not
                // the preset's to change.
                let (p1, p2) = (self.config.roster.p1, self.config.roster.p2);
                self.config.apply_preset(next);
                self.config.roster.p1 = p1;
                self.config.roster.p2 = p2;
            }
            Row::Toggle(i) => {
                let t = &TOGGLES[i];
                let v = toggle_value(&self.config, t.key) as i32;
                let next = match t.frames {
                    Some((lo, hi)) => step(v, lo as i32, hi as i32).to_string(),
                    None => if v == 0 { "on" } else { "off" }.to_string(),
                };
                let _ = self.config.set_field_by_name(t.key, &next);
            }
            Row::Difficulty => self.config.difficulty = step(self.config.difficulty as i32, 1, 4) as u8,
            Row::RoundTime => {
                let times = [100u16, 150, 200, 250];
                let at = times.iter().position(|&t| t == self.config.round_time).unwrap_or(0) as i32;
                self.config.round_time = times[step(at, 0, 3) as usize];
            }
            Row::FullCast => self.config.roster.full_cast = !self.config.roster.full_cast,
            Row::FourCountries => self.config.roster.four_country = !self.config.roster.four_country,
            Row::WhatIfStyles => self.config.roster.what_if_styles = !self.config.roster.what_if_styles,
            Row::BanList => {
                let all = [BanFormat::Unrestricted, BanFormat::StandardTournament, BanFormat::LowTierNoShoto];
                let at = all.iter().position(|&b| b == self.config.roster.ban).unwrap_or(0) as i32;
                self.config.roster.ban = all[(at + delta).rem_euclid(3) as usize];
            }
            Row::Scale => {
                // Up to the largest that fits; a scale kept from a bigger
                // monitor steps down from there.
                let max = i32::from(self.display.max_scale.clamp(1, MAX_SCALE));
                self.settings.scale = step(i32::from(self.settings.scale).min(max), 1, max) as u8;
            }
            Row::Fullscreen => self.settings.fullscreen = !self.settings.fullscreen,
            Row::Vsync => self.settings.vsync = !self.settings.vsync,
            Row::FrameDelay => {
                // From the delay in use, within the look's cap.
                let cap = i32::from(frame_delay_cap(self.look()));
                self.settings.frame_delay_ms = step(i32::from(self.frame_delay()), 0, cap) as u8;
            }
            Row::Aspect => {
                let aspect = self.settings.aspect;
                self.settings.aspect = if delta < 0 { aspect.prev() } else { aspect.next() };
            }
            Row::GaugeArt => {
                let art = self.settings.gauge_art;
                self.settings.gauge_art = if delta < 0 { art.prev() } else { art.next() };
            }
            Row::CrtLook => {
                // An id not in the list (it stepped from nowhere) starts
                // from OFF, as does a look that failed (it shows OFF); the
                // look stepped to is tried afresh.
                let n = self.looks.len() as i32;
                let at = self.looks.iter().position(|l| l.id == self.look()).map_or(0, |i| i as i32);
                self.settings.crt_look = self.looks[(at + delta).rem_euclid(n) as usize].id.clone();
                self.failed_look = None;
                self.notice = None;
            }
            Row::Master => self.settings.master = step(self.settings.master as i32, 0, 10) as u8,
            Row::Announcer => self.settings.announcer = step(self.settings.announcer as i32, 0, 10) as u8,
            Row::Fm => self.settings.fm = step(self.settings.fm as i32, 0, 10) as u8,
            Row::Samples => self.settings.samples = step(self.settings.samples as i32, 0, 10) as u8,
            Row::Device(side) => {
                let all = Device::ALL;
                let at = all.iter().position(|&d| d == self.settings.devices[side]).unwrap_or(0) as i32;
                self.settings.devices[side] = all[(at + delta).rem_euclid(all.len() as i32) as usize];
            }
            Row::MoveInputs => {
                let n = self.settings.move_inputs;
                self.settings.move_inputs = if delta < 0 { n.prev() } else { n.next() };
            }
            Row::InputDelay => {
                let all = InputDelaySetting::ALL;
                let at = all.iter().position(|&value| value == self.settings.netplay_input_delay).unwrap_or(0) as i32;
                self.settings.netplay_input_delay = all[(at + delta).rem_euclid(all.len() as i32) as usize];
            }
            Row::MaxRollback => self.settings.netplay_max_rollback = step(i32::from(self.settings.netplay_max_rollback), 1, 8) as u8,
            Row::WiredOnly => self.settings.wired_only = !self.settings.wired_only,
            Row::NetplayHud => {
                let all = NetplayHudMode::ALL;
                let at = all.iter().position(|&value| value == self.settings.netplay_hud).unwrap_or(0) as i32;
                self.settings.netplay_hud = all[(at + delta).rem_euclid(all.len() as i32) as usize];
            }
            Row::UdpPort => self.settings.netplay_port = step(i32::from(self.settings.netplay_port), 1024, 65535) as u16,
            Row::Scheme(_) | Row::Rebind(_) | Row::TuneLook | Row::Jukebox | Row::PlayerTag | Row::Greyed(_) | Row::Heading(_) | Row::ResetTab => {}
        }
    }

    /// Put this tab's rows back to their defaults.
    fn reset_tab(&mut self) {
        let defaults = MatchConfig::default();
        let first = Settings::default();
        match self.tab() {
            Tab::Gameplay => {
                let roster = self.config.roster;
                self.config = MatchConfig {
                    roster,
                    ..defaults
                };
            }
            Tab::Roster => {
                let (p1, p2) = (self.config.roster.p1, self.config.roster.p2);
                self.config.roster.full_cast = defaults.roster.full_cast;
                self.config.roster.four_country = defaults.roster.four_country;
                self.config.roster.what_if_styles =
                    RosterSettings::for_preset(self.config.preset).what_if_styles;
                self.config.roster.ban = defaults.roster.ban;
                self.config.roster.p1 = p1;
                self.config.roster.p2 = p2;
            }
            Tab::Video => {
                self.settings.scale = first.scale;
                self.settings.fullscreen = first.fullscreen;
                self.settings.vsync = first.vsync;
                self.settings.frame_delay_ms = first.frame_delay_ms;
                self.settings.aspect = first.aspect;
                self.settings.gauge_art = first.gauge_art;
                self.settings.crt_look = first.crt_look;
                self.failed_look = None;
                self.notice = None;
            }
            Tab::Sound => {
                self.settings.master = first.master;
                self.settings.announcer = first.announcer;
                self.settings.fm = first.fm;
                self.settings.samples = first.samples;
            }
            Tab::Controls => {
                self.settings.devices = first.devices;
                self.settings.move_inputs = first.move_inputs;
                self.reset_bindings = true;
            }
            Tab::Netplay => {
                self.settings.player_tag = first.player_tag;
                self.settings.netplay_input_delay = first.netplay_input_delay;
                self.settings.netplay_max_rollback = first.netplay_max_rollback;
                self.settings.wired_only = first.wired_only;
                self.settings.netplay_hud = first.netplay_hud;
                self.settings.netplay_port = first.netplay_port;
                self.settings.lobby_url = first.lobby_url;
            }
        }
    }

    /// Whether `RESET TAB` on the Controls tab asked for the default
    /// bindings since the last call (the session holds the bindings).
    pub fn take_reset_bindings(&mut self) -> bool {
        std::mem::take(&mut self.reset_bindings)
    }

    fn move_cursor(&mut self, delta: isize) -> bool {
        let rows = rows(self.tab());
        let n = rows.len() as isize;
        let mut at = self.cursor[self.tab] as isize;
        for _ in 0..n {
            at = (at + delta).rem_euclid(n);
            if rows[at as usize].live() {
                break;
            }
        }
        let moved = at as usize != self.cursor[self.tab];
        self.cursor[self.tab] = at as usize;
        moved
    }

    fn begin_entry(&mut self, row: Row) {
        let text: Vec<char> = self.full_value(row).chars().collect();
        self.entry = Some(NetplayEntry { row, at: text.len(), text, problem: None });
    }

    /// Whether the keyboard is editing text instead of driving menu shortcuts.
    pub fn typing(&self) -> bool {
        self.entry.is_some()
    }

    /// Typed keys share the pad's inline editor; characters insert at the caret.
    pub fn type_key(&mut self, key: TypedKey) -> Option<Transition> {
        if !self.typing() {
            return None;
        }
        let input = match key {
            TypedKey::Left => MenuInput::Left,
            TypedKey::Right => MenuInput::Right,
            TypedKey::Up => MenuInput::Up,
            TypedKey::Down => MenuInput::Down,
            TypedKey::Enter => MenuInput::Confirm,
            TypedKey::Escape => MenuInput::Back,
            _ => {
                let entry = self.entry.as_mut()?;
                match key {
                    TypedKey::Char(ch) => {
                        let ch = ch.to_ascii_uppercase();
                        if !entry.alphabet().contains(ch) || entry.text.len() >= entry.limit() {
                            return None;
                        }
                        entry.text.insert(entry.at, ch);
                        entry.at += 1;
                    }
                    TypedKey::Erase if entry.at > 0 => {
                        entry.at -= 1;
                        entry.text.remove(entry.at);
                    }
                    TypedKey::EraseAhead if entry.at < entry.text.len() => {
                        entry.text.remove(entry.at);
                    }
                    TypedKey::Home if entry.at > 0 => entry.at = 0,
                    TypedKey::End if entry.at < entry.text.len() => entry.at = entry.text.len(),
                    _ => return None,
                }
                entry.problem = None;
                return Some(Transition::CursorMoved);
            }
        };
        self.apply_entry(input)
    }

    fn apply_entry(&mut self, input: MenuInput) -> Option<Transition> {
        if input == MenuInput::Back {
            self.entry = None;
            return Some(Transition::CursorMoved);
        }
        if input == MenuInput::Confirm {
            let entry = self.entry.as_mut()?;
            let text: String = entry.text.iter().collect();
            if entry.row == Row::UdpPort {
                let Some(port) = text.parse::<u16>().ok().filter(|&port| port >= 1024) else {
                    entry.problem = Some("PORT MUST BE 1024 TO 65535".to_owned());
                    return None;
                };
                self.settings.netplay_port = port;
            } else {
                let tag = text.trim();
                if tag_from_str(tag).is_none() {
                    entry.problem = Some("USE 1 TO 12 LETTERS, DIGITS, SPACE, - OR _".to_owned());
                    return None;
                }
                self.settings.player_tag = tag.to_owned();
            }
            self.entry = None;
            return Some(Transition::ValueChanged);
        }
        let entry = self.entry.as_mut()?;
        match input {
            MenuInput::Up | MenuInput::Down => {
                if entry.at == entry.text.len() {
                    if entry.text.len() >= entry.limit() {
                        return None;
                    }
                    entry.text.push(if entry.row == Row::UdpPort { '0' } else { 'A' });
                } else {
                    let alphabet: Vec<char> = entry.alphabet().chars().collect();
                    let at = alphabet.iter().position(|&ch| ch == entry.text[entry.at]).unwrap_or(0) as isize;
                    let delta = if input == MenuInput::Up { 1 } else { -1 };
                    entry.text[entry.at] = alphabet[(at + delta).rem_euclid(alphabet.len() as isize) as usize];
                }
            }
            MenuInput::Left if entry.at > 0 => entry.at -= 1,
            MenuInput::Right if entry.at < entry.text.len() => entry.at += 1,
            MenuInput::Start if entry.at < entry.text.len() => {
                entry.text.remove(entry.at);
            }
            MenuInput::Start if entry.at > 0 => {
                entry.at -= 1;
                entry.text.remove(entry.at);
            }
            _ => return None,
        }
        entry.problem = None;
        Some(Transition::CursorMoved)
    }

    /// Up and down move among the live rows; left and right change the
    /// value; confirm toggles, edits Netplay text, or resets on `RESET TAB`; the tab pair
    /// switches tabs; back leaves.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        if self.typing() {
            return self.apply_entry(input);
        }
        match input {
            MenuInput::Up => self.move_cursor(-1).then_some(Transition::CursorMoved),
            MenuInput::Down => self.move_cursor(1).then_some(Transition::CursorMoved),
            MenuInput::TabLeft | MenuInput::TabRight => {
                let d = if input == MenuInput::TabRight { 1 } else { TABS.len() - 1 };
                self.tab = (self.tab + d) % TABS.len();
                Some(Transition::CursorMoved)
            }
            MenuInput::Left | MenuInput::Right => {
                let row = self.row();
                if matches!(row, Row::ResetTab | Row::Greyed(_) | Row::Heading(_) | Row::Scheme(_) | Row::Rebind(_) | Row::TuneLook | Row::Jukebox | Row::PlayerTag)
                    || self.dim(row)
                {
                    return None;
                }
                self.change(row, if input == MenuInput::Right { 1 } else { -1 });
                Some(Transition::ValueChanged)
            }
            MenuInput::Confirm => match self.row() {
                row @ (Row::PlayerTag | Row::UdpPort) => {
                    self.begin_entry(row);
                    Some(Transition::CursorMoved)
                }
                Row::Rebind(item) => Some(Transition::Choose(item)),
                Row::TuneLook => self.tune_enabled().then_some(Transition::Choose(MenuItem::TuneLook)),
                Row::Jukebox => Some(Transition::Choose(MenuItem::Jukebox)),
                Row::ResetTab => {
                    self.reset_tab();
                    Some(Transition::ValueChanged)
                }
                row @ (Row::Toggle(_) | Row::FullCast | Row::FourCountries | Row::WhatIfStyles | Row::Fullscreen | Row::Vsync | Row::Aspect | Row::GaugeArt | Row::MoveInputs | Row::CrtLook | Row::WiredOnly | Row::NetplayHud | Row::InputDelay) if !self.dim(row) => {
                    if let Row::Toggle(i) = row {
                        if TOGGLES[i].frames.is_some() {
                            return None;
                        }
                    }
                    self.change(row, 1);
                    Some(Transition::ValueChanged)
                }
                _ => None,
            },
            MenuInput::Back => Some(Transition::Back),
            _ => None,
        }
    }

    /// The framed screen: the tabs across the top (the current one yellow
    /// and underlined), the current tab's rows as `NAME ....
    /// VALUE` (scrolling with the cursor), and per-tab notices or inline
    /// Netplay text entry.
    pub fn render(&self, font: &RomFont<'_>) -> Frame {
        let mut frame = super::draw::framed_wide(font, "SETTINGS", "");
        let argb = &mut frame.argb;
        let stamp = crate::version::short_stamp();
        fallback_text(
            argb,
            (VALUE_END_COL * 8).saturating_sub(stamp.chars().count() * 4),
            24,
            stamp,
            font.pen(ink::DIM, 0),
            1,
        );
        let mut col = 2;
        for (i, name) in TAB_NAMES.iter().enumerate() {
            let color = if i == self.tab {
                ink::YELLOW
            } else {
                ink::BLUE
            };
            font.text(argb, col, TAB_ROW, name, color);
            if i == self.tab {
                let y = TAB_ROW * 8 + 9;
                let yellow = font.pen(ink::YELLOW, 0);
                for x in col * 8..(col + name.len()) * 8 {
                    if y < HEIGHT && x < WIDTH {
                        argb[y * WIDTH + x] = yellow;
                    }
                }
            }
            col += name.len() + 1;
        }
        let rows = rows(self.tab());
        let cursor = self.cursor[self.tab];
        let top = cursor.saturating_sub(VISIBLE_ROWS - 1).min(rows.len().saturating_sub(VISIBLE_ROWS));
        for (line, (i, row)) in rows.iter().enumerate().skip(top).take(VISIBLE_ROWS).enumerate() {
            let r = FIRST_ROW + line;
            let color = if let Row::Heading(_) = row {
                ink::P2_BLUE
            } else if !row.live() {
                ink::DIM
            } else if i == cursor {
                font.text(argb, LABEL_COL - 2, r, ">", ink::YELLOW);
                if self.dim(*row) { ink::DIM } else { ink::YELLOW }
            } else if self.dim(*row) {
                ink::DIM
            } else {
                ink::BLUE
            };
            let label = self.label(*row);
            let value = self.value(*row);
            font.text(argb, LABEL_COL, r, label, color);
            if !value.is_empty() {
                // `value` fits its cells; saturating all the same, so no
                // value can wrap the column.
                let vcol = VALUE_END_COL.saturating_sub(value.chars().count());
                let dots_from = LABEL_COL + label.chars().count() + 1;
                if vcol > dots_from + 1 {
                    font.text(argb, dots_from, r, &".".repeat(vcol - 1 - dots_from), ink::DIM);
                }
                font.text(argb, vcol, r, &value, color);
            }
            if let Some(entry) = self.entry.as_ref().filter(|entry| entry.row == *row) {
                let x = (VALUE_END_COL.saturating_sub(value.chars().count()) + entry.at) * 8;
                let y = r * 8 + 7;
                for px in x..x + 7 {
                    if px < WIDTH && y < HEIGHT {
                        argb[y * WIDTH + px] = font.pen(ink::YELLOW, 0);
                    }
                }
            }
        }
        if let Some(entry) = &self.entry {
            let hint = entry.problem.as_deref().unwrap_or("TYPE OR UP/DOWN CHANGE  LEFT/RIGHT MOVE");
            font.text_centred(argb, NOTICE_ROW - 1, hint, ink::YELLOW);
            font.text_centred(argb, NOTICE_ROW, "START DELETE  CONFIRM SAVE  BACK CANCEL", ink::YELLOW);
        } else if self.tab() == Tab::Netplay && matches!(self.row(), Row::PlayerTag | Row::UdpPort) {
            let hint = if self.row() == Row::PlayerTag { "CONFIRM TO EDIT PLAYER TAG" } else { "CONFIRM TO EDIT UDP PORT" };
            font.text_centred(argb, NOTICE_ROW, hint, ink::YELLOW);
        } else if self.tab() == Tab::Video {
            match &self.notice {
                Some(notice) => font.text_centred(argb, NOTICE_ROW, notice, ink::YELLOW),
                None => {
                    // The refresh rate and what it means, ending on the notice row.
                    let lines = refresh_notice(self.display.refresh_mhz, self.settings.vsync);
                    for (i, line) in lines.iter().enumerate() {
                        font.text_centred(argb, NOTICE_ROW + 1 + i - lines.len(), line, ink::YELLOW);
                    }
                }
            }
        }
        frame
    }
}
