//! Keyboard session recording, scenario JSON conversion, and reference
//! verification (ticket #50, F10).
//!
//! A recorded session starts at one of the launcher's two fixtures:
//! - `cpu`: checkpoint `2389` of `sf-cpu-01` (`cpu-01-run-a`);
//! - `versus`: checkpoint `2450` of `sf-move-01` (`move-run-a`).
//!
//! For each stepped frame `f -> f + 1` (`f` from `fixture_frame` to
//! `last_frame - 1`), the recorder logs:
//! - `frame`: the scenario input frame `f` active before `machine.step()`;
//! - `focused`: whether the window was active (`1` or `0`);
//! - `focus_changed`: whether `focused` changed on this frame (`1` or `0`);
//! - `frame_ms`: wall-clock duration of the frame in milliseconds;
//! - `p1_mask` and `p2_mask`: 10-bit masks of [`Inputs`] (`directions[0..4]`
//!   at bits `0..4`, `buttons[0..6]` at bits `4..10`).
//!
//! The converter turns any recorded session into a scenario JSON whose boot
//! inputs reproduce the fixture state (`2389` or `2450`), whose gameplay
//! inputs encode every held span from `fixture_frame` on, and whose
//! `observations` set `window = [fixture_frame, last_frame]` and
//! `checkpoints = [fixture_frame..=last_frame]`.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use minifb::Key;
use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::cpu::CpuMachine;
use sf_engine::run::{
    cpu_owned_addresses, cpu_work_image, observe_cpu_fight, observe_versus_fight,
    run_cpu_fight_capture_with_inputs, run_versus_interval_capture, versus_owned_addresses,
    versus_work_image, ScratchDir,
};
use sf_video::render::render_frame;
use sf_video::romset::Assets;

use crate::{apply_cpu, Bindings, Inputs, P1_KEYS, P2_KEYS};

/// Which fixture a recorded session starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureKind {
    /// One-player fixture: `sf-cpu-01` checkpoint `2389`.
    Cpu,
    /// Versus fixture: `sf-move-01` checkpoint `2450`.
    Versus,
}

impl FixtureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FixtureKind::Cpu => "cpu",
            FixtureKind::Versus => "versus",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim() {
            "cpu" => Ok(FixtureKind::Cpu),
            "versus" => Ok(FixtureKind::Versus),
            other => Err(format!(
                "unknown fixture kind '{other}' (expected 'cpu' or 'versus')"
            )),
        }
    }

    /// The fixture's checkpoint frame (`2389` for `cpu`, `2450` for `versus`).
    pub fn fixture_frame(self) -> u32 {
        match self {
            FixtureKind::Cpu => 2389,
            FixtureKind::Versus => 2450,
        }
    }
}

/// Scenario input press names for a player's 10 logical inputs in [`Inputs`]
/// order: right, left, down, up, button1..=button6 (LP, MP, HP, LK, MK, HK).
pub const P1_PRESS_NAMES: [&str; 10] = [
    "p1_right",
    "p1_left",
    "p1_down",
    "p1_up",
    "p1_button1",
    "p1_button2",
    "p1_button3",
    "p1_button4",
    "p1_button5",
    "p1_button6",
];

pub const P2_PRESS_NAMES: [&str; 10] = [
    "p2_right",
    "p2_left",
    "p2_down",
    "p2_up",
    "p2_button1",
    "p2_button2",
    "p2_button3",
    "p2_button4",
    "p2_button5",
    "p2_button6",
];

/// Pack [`Inputs`] into a 10-bit mask (`directions[0..4]` in bits `0..4`,
/// `buttons[0..6]` in bits `4..10`).
pub fn pack_inputs(inputs: Inputs) -> u16 {
    let mut mask = 0u16;
    for (i, held) in inputs
        .directions
        .into_iter()
        .chain(inputs.buttons)
        .enumerate()
    {
        if held {
            mask |= 1 << i;
        }
    }
    mask
}

/// Unpack a 10-bit mask into [`Inputs`].
pub fn unpack_inputs(mask: u16) -> Inputs {
    let mut directions = [false; 4];
    let mut buttons = [false; 6];
    for (i, slot) in directions.iter_mut().enumerate() {
        *slot = (mask & (1 << i)) != 0;
    }
    for (i, slot) in buttons.iter_mut().enumerate() {
        *slot = (mask & (1 << (i + 4))) != 0;
    }
    Inputs {
        directions,
        buttons,
    }
}

/// Read a player's inputs through [`Bindings::inputs`] gated on window focus:
/// unfocused frames return [`Inputs::default`] (releasing all held keys).
pub fn read_with_focus(bindings: &Bindings, focused: bool, down: impl Fn(Key) -> bool) -> Inputs {
    let sampled = bindings.inputs(down);
    if focused {
        sampled
    } else {
        Inputs::default()
    }
}

/// One stepped frame's entry in a recorded session.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionFrame {
    /// Scenario input frame `f` (before stepping `f -> f + 1`).
    pub frame: u32,
    /// Whether the launcher window was focused during this frame.
    pub focused: bool,
    /// Whether `focused` changed from the previous frame.
    pub focus_changed: bool,
    /// Wall-clock frame duration in milliseconds.
    pub frame_ms: f64,
    /// P1's logical inputs active on this frame.
    pub p1: Inputs,
    /// P2's logical inputs active on this frame.
    pub p2: Inputs,
}

/// A recorded keyboard session from a fixture (`2389` or `2450`) to its end.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionLog {
    pub fixture: FixtureKind,
    pub rows: Vec<SessionFrame>,
}

/// Timing and focus summary of a [`SessionLog`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SessionStats {
    pub fixture: FixtureKind,
    pub fixture_frame: u32,
    pub last_frame: u32,
    pub frames: usize,
    pub focus_changes: usize,
    pub mean_frame_ms: f64,
    pub max_frame_ms: f64,
}

impl SessionLog {
    pub fn new(fixture: FixtureKind) -> Self {
        SessionLog {
            fixture,
            rows: Vec::new(),
        }
    }

    /// First frame of the session (the fixture checkpoint frame: `2389` or
    /// `2450`).
    pub fn fixture_frame(&self) -> u32 {
        self.fixture.fixture_frame()
    }

    /// Last end-of-frame produced by the session (`fixture_frame + rows.len()`).
    pub fn last_frame(&self) -> u32 {
        self.fixture_frame() + self.rows.len() as u32
    }

    /// Compute frame-time statistics (mean and maximum `frame_ms`) and focus
    /// change count.
    pub fn stats(&self) -> SessionStats {
        let frames = self.rows.len();
        let focus_changes = self.rows.iter().filter(|r| r.focus_changed).count();
        let sum: f64 = self.rows.iter().map(|r| r.frame_ms).sum();
        let mean_frame_ms = if frames > 0 { sum / frames as f64 } else { 0.0 };
        let max_frame_ms = self.rows.iter().map(|r| r.frame_ms).fold(0.0f64, f64::max);
        SessionStats {
            fixture: self.fixture,
            fixture_frame: self.fixture_frame(),
            last_frame: self.last_frame(),
            frames,
            focus_changes,
            mean_frame_ms,
            max_frame_ms,
        }
    }

    /// Write the session to a `.tsv` file.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut out = String::new();
        out.push_str(&format!("# fixture\t{}\n", self.fixture.as_str()));
        out.push_str(&format!("# fixture_frame\t{}\n", self.fixture_frame()));
        out.push_str("frame\tfocused\tfocus_changed\tframe_ms\tp1_mask\tp2_mask\n");
        for r in &self.rows {
            out.push_str(&format!(
                "{}\t{}\t{}\t{:.4}\t0x{:03x}\t0x{:03x}\n",
                r.frame,
                u8::from(r.focused),
                u8::from(r.focus_changed),
                r.frame_ms,
                pack_inputs(r.p1),
                pack_inputs(r.p2),
            ));
        }
        std::fs::write(path, out)
    }

    /// Load a session from a `.tsv` file written by [`SessionLog::write`].
    pub fn load(path: &Path) -> io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut fixture: Option<FixtureKind> = None;
        let mut rows = Vec::new();
        for (line_no, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix('#') {
                let rest = rest.trim();
                if let Some((k, v)) = rest.split_once('\t') {
                    if k.trim() == "fixture" {
                        fixture = Some(
                            FixtureKind::parse(v)
                                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                        );
                    }
                }
                continue;
            }
            if line.starts_with("frame\t") {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 6 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "line {}: expected 6 columns, got {}",
                        line_no + 1,
                        cols.len()
                    ),
                ));
            }
            let parse_u32 = |s: &str, name: &str| {
                s.parse::<u32>().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("line {}: invalid {name} '{s}'", line_no + 1),
                    )
                })
            };
            let parse_mask = |s: &str, name: &str| {
                let hex = s
                    .strip_prefix("0x")
                    .or_else(|| s.strip_prefix("0X"))
                    .unwrap_or(s);
                u16::from_str_radix(hex, 16).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("line {}: invalid {name} '{s}'", line_no + 1),
                    )
                })
            };
            let frame = parse_u32(cols[0], "frame")?;
            let focused = parse_u32(cols[1], "focused")? != 0;
            let focus_changed = parse_u32(cols[2], "focus_changed")? != 0;
            let frame_ms = cols[3].parse::<f64>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("line {}: invalid frame_ms '{}'", line_no + 1, cols[3]),
                )
            })?;
            let p1 = unpack_inputs(parse_mask(cols[4], "p1_mask")?);
            let p2 = unpack_inputs(parse_mask(cols[5], "p2_mask")?);
            rows.push(SessionFrame {
                frame,
                focused,
                focus_changed,
                frame_ms,
                p1,
                p2,
            });
        }
        let fixture = fixture.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "session file lacks '# fixture' header",
            )
        })?;
        let expected_first = fixture.fixture_frame();
        for (i, r) in rows.iter().enumerate() {
            let expected = expected_first + i as u32;
            if r.frame != expected {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("row {i}: expected frame {expected}, got {}", r.frame),
                ));
            }
        }
        Ok(SessionLog { fixture, rows })
    }
}

/// Convert a [`SessionLog`] into a scenario JSON value.
///
/// For every session it converts:
/// - Prepends the fixture's boot inputs (`sf-cpu-01` for `2389`, `sf-move-01`
///   for `2450`) so MAME arrives at the exact fixture state at `fixture_frame`.
/// - Encodes each contiguous held span of every P1 and P2 logical input from
///   `fixture_frame` through `last_frame - 1` into `{frame, press, hold_frames}`.
/// - Writes `duration_frames = last_frame + 1`.
/// - Writes `observations.window = [fixture_frame, last_frame]` and
///   `observations.checkpoints = [fixture_frame, fixture_frame + 1, ..., last_frame]`.
pub fn session_to_scenario_json(
    session: &SessionLog,
    scenario_id: &str,
    description: &str,
) -> serde_json::Value {
    let fixture_frame = session.fixture_frame();
    let last_frame = session.last_frame();
    let mut inputs: Vec<serde_json::Value> = match session.fixture {
        FixtureKind::Cpu => vec![
            serde_json::json!({
                "frame": 1450,
                "press": "coin1",
                "hold_frames": 12,
                "comment": "insert coin; title screen is up from ~1375"
            }),
            serde_json::json!({
                "frame": 1520,
                "press": "start1",
                "hold_frames": 12,
                "comment": "1P start; start sound observed ~1522"
            }),
            serde_json::json!({
                "frame": 1900,
                "press": "start1",
                "hold_frames": 12,
                "comment": "boot path input matching sf-cpu-01"
            }),
        ],
        FixtureKind::Versus => vec![
            serde_json::json!({
                "frame": 1450,
                "press": "coin1",
                "hold_frames": 12,
                "comment": "insert coin 1; title screen is up from ~1375"
            }),
            serde_json::json!({
                "frame": 1480,
                "press": "coin2",
                "hold_frames": 12,
                "comment": "insert coin 2 (versus needs both sides credited)"
            }),
            serde_json::json!({
                "frame": 1560,
                "press": "start2",
                "hold_frames": 12,
                "comment": "2P start: enters versus mode"
            }),
            serde_json::json!({
                "frame": 1900,
                "press": "start1",
                "hold_frames": 12,
                "comment": "P1 confirms default country Japan"
            }),
            serde_json::json!({
                "frame": 2000,
                "press": "start2",
                "hold_frames": 12,
                "comment": "P2 confirms default country Japan"
            }),
        ],
    };

    // Extract contiguous holds for each of P1's 10 and P2's 10 logical inputs.
    let mut gameplay_spans: Vec<(u32, &'static str, u32)> = Vec::new();
    for (p2, names) in [(false, P1_PRESS_NAMES), (true, P2_PRESS_NAMES)] {
        for (slot, press_name) in names.into_iter().enumerate() {
            let mut active_start: Option<u32> = None;
            let mut hold_len = 0u32;
            for r in &session.rows {
                let mask = if p2 {
                    pack_inputs(r.p2)
                } else {
                    pack_inputs(r.p1)
                };
                let is_held = (mask & (1 << slot)) != 0;
                if is_held {
                    if active_start.is_none() {
                        active_start = Some(r.frame);
                        hold_len = 1;
                    } else {
                        hold_len += 1;
                    }
                } else if let Some(start) = active_start.take() {
                    gameplay_spans.push((start, press_name, hold_len));
                    hold_len = 0;
                }
            }
            if let Some(start) = active_start.take() {
                gameplay_spans.push((start, press_name, hold_len));
            }
        }
    }
    gameplay_spans.sort_by_key(|(frame, name, _)| (*frame, *name));
    for (frame, press, hold_frames) in gameplay_spans {
        inputs.push(serde_json::json!({
            "frame": frame,
            "press": press,
            "hold_frames": hold_frames,
        }));
    }

    let checkpoints: Vec<u32> = (fixture_frame..=last_frame).collect();
    serde_json::json!({
        "scenario_id": scenario_id,
        "format_version": 1,
        "description": description,
        "reference": {
            "rom_set": "sf",
            "mame_version": "0.289",
            "notes": "US set 1, six-button controls; MAME set sf. ROM set verified by mame -verifyroms against the pinned build."
        },
        "machine": {
            "set": "sf",
            "mame_options": [
                "-video",
                "none",
                "-sound",
                "none",
                "-nothrottle",
                "-skip_gameinfo",
                "-noreadconfig",
                "-nowriteconfig"
            ],
            "dipswitches": "factory defaults; observed readback DSW1=0xdfff, DSW2=0xffff"
        },
        "controls": {
            "coin1": { "port": "IN0", "bit": 0 },
            "coin2": { "port": "IN0", "bit": 1 },
            "p1_button6": { "port": "IN0", "bit": 2 },
            "p2_button6": { "port": "IN0", "bit": 8 },
            "p1_button3": { "port": "IN0", "bit": 9 },
            "p2_button3": { "port": "IN0", "bit": 10 },
            "p1_right": { "port": "IN1", "bit": 0 },
            "p1_left": { "port": "IN1", "bit": 1 },
            "p1_down": { "port": "IN1", "bit": 2 },
            "p1_up": { "port": "IN1", "bit": 3 },
            "p1_button1": { "port": "IN1", "bit": 4 },
            "p1_button2": { "port": "IN1", "bit": 5 },
            "p1_button4": { "port": "IN1", "bit": 6 },
            "p1_button5": { "port": "IN1", "bit": 7 },
            "p2_right": { "port": "IN1", "bit": 8 },
            "p2_left": { "port": "IN1", "bit": 9 },
            "p2_down": { "port": "IN1", "bit": 10 },
            "p2_up": { "port": "IN1", "bit": 11 },
            "p2_button1": { "port": "IN1", "bit": 12 },
            "p2_button2": { "port": "IN1", "bit": 13 },
            "p2_button4": { "port": "IN1", "bit": 14 },
            "p2_button5": { "port": "IN1", "bit": 15 },
            "start1": { "port": "SYSTEM", "bit": 0 },
            "start2": { "port": "SYSTEM", "bit": 1 },
            "service1": { "port": "SYSTEM", "bit": 2 }
        },
        "duration_frames": last_frame + 1,
        "inputs": inputs,
        "observations": {
            "window": [fixture_frame, last_frame],
            "screen_downsample_stride": 4,
            "ram_crc_every_frames": 10,
            "checkpoints": checkpoints
        }
    })
}

/// Convert a session `.tsv` file on disk to a scenario `.json` file on disk.
pub fn convert_session_file(
    session_path: &Path,
    scenario_path: &Path,
    scenario_id: Option<&str>,
) -> io::Result<SessionStats> {
    let session = SessionLog::load(session_path)?;
    let stats = session.stats();
    let default_id = scenario_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sf-session-01")
        .replace('_', "-");
    let id = scenario_id.unwrap_or(&default_id);
    let desc = format!(
        "Recorded {} keyboard session from fixture {} through {} ({} frames).",
        session.fixture.as_str(),
        stats.fixture_frame,
        stats.last_frame,
        stats.frames
    );
    let json = session_to_scenario_json(&session, id, &desc);
    if let Some(parent) = scenario_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let formatted = serde_json::to_string_pretty(&json)
        .map_err(|e| io::Error::other(format!("serialize scenario JSON: {e}")))?;
    std::fs::write(scenario_path, formatted + "\n")?;
    Ok(stats)
}

/// Number of stepped frames in the synthetic session (`2389..3400` = `1,011`
/// frames, producing end-of-frame `2390..=3400`).
pub const SYNTHETIC_SESSION_FRAMES: u32 = 1011;

/// Synthetic key state and window focus at scenario input frame `f`
/// (`2389..3400`).
///
/// Exercises all 20 bound keys (`P1_KEYS` and `P2_KEYS`) in dedicated focused
/// windows, plus an unfocused window (`3050..3060`) while `Key::Right` is held
/// at the OS level (`3045..3065`) so focus loss and regain are recorded and
/// gated through [`read_with_focus`].
pub fn synthetic_key_and_focus(f: u32) -> (bool, impl Fn(Key) -> bool) {
    let focused = !(3050..3060).contains(&f);
    let down = move |k: Key| match k {
        // P1 directions (Right, Left, Down, Up) + focus-gated span + Hadoken + walk
        Key::Right => {
            (2400..2440).contains(&f)
                || (3045..3065).contains(&f)
                || (3102..3122).contains(&f)
                || (3200..3240).contains(&f)
        }
        Key::Left => (2445..2465).contains(&f),
        Key::Down => (2470..2485).contains(&f) || (3100..3103).contains(&f),
        Key::Up => (2490..2498).contains(&f),
        // P1 attack buttons (A=LP, S=MP, D=HP, Z=LK, X=MK, C=HK)
        Key::A => (2540..2546).contains(&f) || (3106..3110).contains(&f),
        Key::S => (2590..2596).contains(&f),
        Key::D => (2640..2646).contains(&f),
        Key::Z => (2690..2696).contains(&f) || (3245..3251).contains(&f),
        Key::X => (2740..2746).contains(&f),
        Key::C => (2790..2796).contains(&f),
        // P2 directions (H=Right, F=Left, G=Down, T=Up)
        Key::H => (2840..2855).contains(&f),
        Key::F => (2860..2875).contains(&f),
        Key::G => (2880..2895).contains(&f),
        Key::T => (2900..2908).contains(&f),
        // P2 attack buttons (U=LP, I=MP, O=HP, J=LK, K=MK, L=HK)
        Key::U => (2920..2926).contains(&f),
        Key::I => (2940..2946).contains(&f),
        Key::O => (2960..2966).contains(&f),
        Key::J => (2980..2986).contains(&f),
        Key::K => (3000..3006).contains(&f),
        Key::L => (3020..3026).contains(&f),
        _ => false,
    };
    (focused, down)
}

/// Apply both players' [`Inputs`] to a [`CpuMachine`] (including P2's
/// directions and buttons so `0xff868e` `IN1_COPY` matches MAME when P2 keys
/// are pressed during a 1P session).
pub fn apply_cpu_both(machine: &mut CpuMachine, p1: Inputs, p2: Inputs) {
    apply_cpu(machine, p1);
    let [r, l, d, u] = p2.directions;
    machine.set_p2_direction(r, l, d, u);
    machine.set_p2_buttons(p2.buttons);
}

/// Run the scripted synthetic session of [`SYNTHETIC_SESSION_FRAMES`] frames
/// (`2389..=3400`) from `sf-cpu-01`'s checkpoint `2389`, sending every bound
/// key through [`Bindings::inputs`] via [`read_with_focus`], stepping
/// [`CpuMachine`], rendering each frame, and recording per-frame inputs,
/// focus changes, and wall-clock frame times.
pub fn record_synthetic_session(
    cpu_fixture: &Path,
    assets: &Assets,
    session_out: &Path,
) -> io::Result<SessionLog> {
    let mut machine = CpuMachine::from_checkpoint_2389(cpu_fixture, assets.maincpu.clone())
        .map_err(|e| io::Error::other(format!("checkpoint-2389 fixture: {e}")))?;
    let mut log = SessionLog::new(FixtureKind::Cpu);
    let first = machine.state.frame;
    let last = first + SYNTHETIC_SESSION_FRAMES;
    let mut prev_focused = true;
    while machine.state.frame < last {
        let tick = Instant::now();
        let f = machine.state.frame;
        let (focused, down) = synthetic_key_and_focus(f);
        let focus_changed = focused != prev_focused;
        prev_focused = focused;
        let p1 = read_with_focus(&P1_KEYS, focused, &down);
        let p2 = read_with_focus(&P2_KEYS, focused, &down);
        apply_cpu_both(&mut machine, p1, p2);
        machine.step();
        let _rendered = render_frame(assets, &machine.state.scene());
        let frame_ms = tick.elapsed().as_secs_f64() * 1e3;
        log.rows.push(SessionFrame {
            frame: f,
            focused,
            focus_changed,
            frame_ms,
            p1,
            p2,
        });
        let unrecovered = machine.unrecovered();
        if !unrecovered.is_empty() {
            return Err(io::Error::other(format!(
                "synthetic session reached untranslated branches {unrecovered:05x?} at frame {}",
                machine.state.frame
            )));
        }
    }
    log.write(session_out)?;
    Ok(log)
}

/// Outcome of verifying a recorded session against two MAME reference captures.
#[derive(Debug, PartialEq)]
pub enum VerifyOutcome {
    /// Both MAME runs are identical and the native replay matches every frame
    /// (`screen_hash`, `latch`, `latch_changed`, checkpoint `pixels`, `video`,
    /// `palette`, and owned/classified work RAM) through `last_frame`.
    Match {
        stats: SessionStats,
        reference_files_compared: usize,
        frames_compared: u32,
        checkpoints_compared: usize,
    },
    /// The native replay matched for `frames_matched` frames and then reached
    /// the listed untranslated original branch(es) at `frame`.
    Stopped {
        stats: SessionStats,
        frames_matched: u32,
        frame: u32,
        unrecovered: Vec<u32>,
    },
    /// Either the two MAME runs differed or the native replay mismatched on an
    /// observable before reaching any untranslated branch.
    Mismatch {
        stats: SessionStats,
        frame: u32,
        reason: String,
    },
}

/// Classify a work-RAM address outside the owned regions (same partition as
/// `milestone_audit.rs`): only task slots' saved SR/PC/USP words and stack
/// memory may differ between native and MAME.
fn work_ram_area(addr: u32) -> &'static str {
    match addr {
        0xff8006..=0xff8105 if (addr - 0xff8006) % 16 >= 2 => "task slots' saved SR/PC/USP",
        0xff8106..=0xff8631 => "stack memory",
        _ => "other",
    }
}

/// Verify that two MAME capture directories have byte-identical observation
/// files (`.tsv`, `.bin`, `.rgba`). Returns the number of files compared, or a
/// description of the first differing file.
pub fn compare_mame_observation_files(
    run_a: &Path,
    run_b: &Path,
) -> io::Result<Result<usize, String>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(run_a)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    files.sort();
    let mut compared = 0usize;
    for a in &files {
        let Some(name) = a.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if !(name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba")) {
            continue;
        }
        let b = run_b.join(&name);
        let bytes_a = std::fs::read(a)?;
        let Ok(bytes_b) = std::fs::read(&b) else {
            return Ok(Err(format!("{name} missing in {}", run_b.display())));
        };
        if bytes_a != bytes_b {
            return Ok(Err(format!(
                "{name} differs between {} and {}",
                run_a.display(),
                run_b.display()
            )));
        }
        compared += 1;
    }
    Ok(Ok(compared))
}

/// Verify a recorded session (`.tsv`) and its scenario (`.json`) against two
/// MAME runs (`run_a` and `run_b`), replaying natively from `fixture_capture`
/// (`cpu-01-run-a` for `cpu` or `move-run-a` for `versus`).
pub fn verify_session(
    session_path: &Path,
    scenario_path: &Path,
    run_a: &Path,
    run_b: &Path,
    fixture_capture: &Path,
    assets: &Assets,
) -> io::Result<VerifyOutcome> {
    let session = SessionLog::load(session_path)?;
    let stats = session.stats();
    let fixture_frame = stats.fixture_frame;
    let last_frame = stats.last_frame;

    // 1. Check that the two MAME runs are byte-for-byte identical on all
    //    observation files (.tsv, .bin, .rgba) and via sf_compare.
    let reference_files_compared = match compare_mame_observation_files(run_a, run_b)? {
        Ok(n) => n,
        Err(reason) => {
            return Ok(VerifyOutcome::Mismatch {
                stats,
                frame: fixture_frame,
                reason: format!("reference self-check failed: {reason}"),
            });
        }
    };
    let scenario = load_scenario(scenario_path)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let cap_a = load_capture(run_a)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let cap_b = load_capture(run_b)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let ref_scope = CompareScope {
        from: Some(fixture_frame as u64),
        to: Some(last_frame as u64),
        fields: None,
    };
    match compare(
        &scenario,
        &cap_a,
        &cap_b,
        CandidateKind::Reference,
        &ref_scope,
    )
    .map_err(|e| io::Error::other(e.to_string()))?
    {
        Outcome::Match { .. } => {}
        Outcome::Mismatch(m) => {
            return Ok(VerifyOutcome::Mismatch {
                stats,
                frame: m.frame as u32,
                reason: format!(
                    "reference runs differ at frame {} on {}: {} vs {}",
                    m.frame, m.observable, m.expected, m.actual
                ),
            });
        }
    }

    // 2. Step the native machine first to check whether an untranslated branch
    //    is reached and, after every frame produced, compare work RAM (owned
    //    regions and non-CPU-state areas).
    let mut work_mismatch: Option<(u32, String)> = None;
    let mut check_work_frame = |frame: u32,
                                image: &[u8],
                                owned: &[(&'static str, u32)]|
     -> io::Result<()> {
        if work_mismatch.is_some() {
            return Ok(());
        }
        let dump_path = run_a.join(format!("ram_{frame:06}_work.bin"));
        let dump = std::fs::read(&dump_path)?;
        for &(name, addr) in owned {
            let i = (addr - 0xff8000) as usize;
            if dump[i] != image[i] {
                work_mismatch = Some((
                    frame,
                    format!(
                        "owned work RAM '{name}' at 0x{addr:06x}: reference {:02x}, native {:02x}",
                        dump[i], image[i]
                    ),
                ));
                return Ok(());
            }
        }
        for i in 0..dump.len().min(image.len()) {
            if dump[i] != image[i] {
                let addr = 0xff8000 + i as u32;
                if work_ram_area(addr) == "other" {
                    work_mismatch = Some((
                        frame,
                        format!(
                            "work RAM outside CPU-state areas at 0x{addr:06x}: reference {:02x}, native {:02x}",
                            dump[i], image[i]
                        ),
                    ));
                    return Ok(());
                }
            }
        }
        Ok(())
    };

    let (stopped_at, unrecovered) = match session.fixture {
        FixtureKind::Cpu => {
            let owned = cpu_owned_addresses();
            let mut io_err: Option<io::Error> = None;
            let res = observe_cpu_fight(
                fixture_capture,
                assets.maincpu.clone(),
                Some(scenario_path),
                fixture_frame,
                last_frame,
                |m| {
                    if io_err.is_none() {
                        let image = cpu_work_image(m);
                        if let Err(e) = check_work_frame(m.state.frame, &image, &owned) {
                            io_err = Some(e);
                        }
                    }
                },
            )?;
            if let Some(e) = io_err {
                return Err(e);
            }
            res
        }
        FixtureKind::Versus => {
            let owned = versus_owned_addresses();
            let mut io_err: Option<io::Error> = None;
            let res = observe_versus_fight(
                fixture_capture,
                assets.maincpu.clone(),
                scenario_path,
                last_frame,
                |m| {
                    if io_err.is_none() {
                        let image = versus_work_image(m);
                        if let Err(e) = check_work_frame(m.state.frame, &image, &owned) {
                            io_err = Some(e);
                        }
                    }
                },
            )?;
            if let Some(e) = io_err {
                return Err(e);
            }
            res
        }
    };

    // 3. Run the comparison boundary (`sf_compare`: screen_hash, latch,
    //    latch_changed, checkpoint pixels, video and palette RAM) up through
    //    the last translated frame (`stopped_at` or `stopped_at - 1` when
    //    stopped on an untranslated branch).
    let compare_last = if unrecovered.is_empty() {
        last_frame
    } else {
        stopped_at.saturating_sub(1)
    };
    let mut checkpoints_compared = 0usize;
    if compare_last > fixture_frame {
        let out = ScratchDir::new("sf-dev-verify");
        match session.fixture {
            FixtureKind::Cpu => {
                run_cpu_fight_capture_with_inputs(
                    fixture_capture,
                    assets,
                    &out,
                    &scenario.scenario_id,
                    Some(scenario_path),
                    fixture_frame,
                    compare_last,
                )?;
            }
            FixtureKind::Versus => {
                run_versus_interval_capture(
                    fixture_capture,
                    assets,
                    &out,
                    &scenario.scenario_id,
                    scenario_path,
                    compare_last,
                )?;
            }
        }
        let native_cap = load_capture(&*out)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let nat_scope = CompareScope {
            from: Some((fixture_frame + 1) as u64),
            to: Some(compare_last as u64),
            fields: Some(
                CompareScope::parse_fields("screen_hash,latch,latch_changed")
                    .expect("valid observable names"),
            ),
        };
        match compare(
            &scenario,
            &cap_a,
            &native_cap,
            CandidateKind::Native,
            &nat_scope,
        )
        .map_err(|e| io::Error::other(e.to_string()))?
        {
            Outcome::Match {
                checkpoints_compared: cp,
                ..
            } => {
                checkpoints_compared = cp;
            }
            Outcome::Mismatch(m) => {
                // Report whichever mismatch came at the earliest frame.
                if let Some((wf, wreason)) = work_mismatch {
                    if wf < m.frame as u32 {
                        return Ok(VerifyOutcome::Mismatch {
                            stats,
                            frame: wf,
                            reason: wreason,
                        });
                    }
                }
                let detail = m
                    .detail
                    .as_deref()
                    .map(|d| format!(" ({d})"))
                    .unwrap_or_default();
                return Ok(VerifyOutcome::Mismatch {
                    stats,
                    frame: m.frame as u32,
                    reason: format!(
                        "{}: expected {}, actual {}{detail}",
                        m.observable, m.expected, m.actual
                    ),
                });
            }
        }
    }

    if let Some((wf, wreason)) = work_mismatch {
        return Ok(VerifyOutcome::Mismatch {
            stats,
            frame: wf,
            reason: wreason,
        });
    }

    if !unrecovered.is_empty() {
        return Ok(VerifyOutcome::Stopped {
            stats,
            frames_matched: compare_last.saturating_sub(fixture_frame),
            frame: stopped_at,
            unrecovered,
        });
    }

    Ok(VerifyOutcome::Match {
        stats,
        reference_files_compared,
        frames_compared: last_frame - fixture_frame,
        checkpoints_compared,
    })
}
