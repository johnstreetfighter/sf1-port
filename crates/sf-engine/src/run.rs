//! Capture runs: drive a native scene machine over its bounded interval and
//! write an observation capture.
//!
//! Row/timing convention (evidence in docs/reference/idle-scene.md): the
//! two screen observables sit at different points of the emulated frame
//! pipeline. The driver's frames.tsv hash reads `screen:pixel` — the bitmap
//! scanned out during frame N, rendered from the end-of-(N-1) state — while
//! its checkpoint pixel snapshot (`video:snapshot_pixels`) is the freshly
//! rendered bitmap from the end-of-N state. RAM dumps and the sound-latch
//! read are likewise end-of-N. Verified: the bg_scroll write of frame 2283
//! appears in checkpoint pixels 2283 but in the hash at 2284; the FIGHT!
//! text written in S_2360 appears in pixels 2360 but in the hash at 2361.
//!
//! The loop therefore renders each state once and uses it for both
//! observables at their own lag: row N's hash comes from the render of
//! S_(N-1); row N's checkpoint pixels, RAM dumps, and latch come from S_N.
//! One exception (evidence in docs/reference/round-outcomes.md): a palette
//! the vblank loads in frame N (0x3b362, from the request a task made in
//! N-1) is already in the scanout the hash at N reads — the 68000's
//! interrupt handler copies it before the screen is drawn — so when the
//! palette changed, S_(N-1) is rendered again with S_N's palette for the
//! hash (`examples/hash_probe.rs` checks this against a capture's dumps).
//! Row INTERVAL_FIRST is the fixture state: render(end-of-2280) hashes
//! identically to render(end-of-2279) (reference rows 2280/2281 both
//! a5cc9aa9).

use std::io;
use std::path::Path;

use sf_video::render::render_frame;
use sf_video::romset::Assets;

use crate::capture::CaptureWriter;
use crate::cpu::{CpuMachine, INTERVAL_FIRST as CPU_FIRST, INTERVAL_LAST as CPU_LAST};
use crate::fight::FightMachine;
use crate::fighter::Program;
use crate::machine::MachineState;
use crate::versus::{
    VersusMachine, CROUCH_INTERVAL_LAST as CROUCH_LAST, INTERVAL_FIRST as VERSUS_FIRST,
    INTERVAL_LAST as VERSUS_LAST, JUMP_INTERVAL_LAST as JUMP_LAST,
    PUNCH_INTERVAL_LAST as PUNCH_LAST,
};

/// A scene machine as the capture loop needs it: frame-addressable state
/// plus a one-frame advance.
trait SceneMachine {
    fn state(&self) -> &MachineState;
    fn dsw(&self) -> (u16, u16);
    fn step(&mut self);
}

impl SceneMachine for FightMachine {
    fn state(&self) -> &MachineState {
        &self.state
    }
    fn dsw(&self) -> (u16, u16) {
        FightMachine::dsw(self)
    }
    fn step(&mut self) {
        FightMachine::step(self);
    }
}

/// The MAME 0.289 `sf` driver's frame-0 screen state before `0x3b77e` writes
/// `gfxctrl_w` in frame 1 (evidence in docs/reference/power-on.md):
/// `sf_state::video_start()` (`sf-mame0289.cpp:266-274`) creates `m_fg_tilemap`
/// and `m_tx_tilemap` enabled (`tilemap_t::m_enable = true`) and
/// `sf_state::machine_reset()` (`sf-mame0289.cpp:745-751`) zeroes `m_active`
/// without calling `enable()`, while `palette_device::device_start()`
/// (`emupal.cpp`) initializes decoded pens to the 8-color RGB cube before any
/// write to `m_paletteram` (`paletteram16_xGGGGGRRRRRBBBBB_word_w`).
fn power_on_frame0_scene(state: &MachineState) -> sf_video::scene::SceneState {
    let mut scene = state.scene();
    scene.active = 0x48;
    for i in 0..1024usize {
        let word: u16 = (((i as u16) & 1) * 0x0f00)
            | ((((i as u16) >> 1) & 1) * 0x00f0)
            | ((((i as u16) >> 2) & 1) * 0x000f);
        let [hi, lo] = word.to_be_bytes();
        scene.paletteram[i * 2] = hi;
        scene.paletteram[i * 2 + 1] = lo;
    }
    scene
}

/// The shared capture loop. `before_step(frame)` runs before producing the
/// end-of-`frame` state (the versus runner uses it to apply the input event
/// scheduled at frame-1, matching the reference driver's frame_done timing).
fn drive<M: SceneMachine>(
    machine: &mut M,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    first: u32,
    last: u32,
    before_step: impl FnMut(&mut M, u32),
) -> io::Result<u64> {
    drive_filtered(
        machine,
        assets,
        out_dir,
        scenario_id,
        first,
        last,
        |_| true,
        before_step,
    )
}

#[allow(clippy::too_many_arguments)]
fn drive_filtered<M: SceneMachine>(
    machine: &mut M,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    first: u32,
    last: u32,
    mut write_checkpoint: impl FnMut(u32) -> bool,
    mut before_step: impl FnMut(&mut M, u32),
) -> io::Result<u64> {
    let mut writer = CaptureWriter::create(out_dir)?;

    // Row `first`: the fixture state (see module doc for the hash lag). At
    // power-on (first == 0), row 0's screen hash is the pre-screen_update
    // blank bitmap while its checkpoint pixel snapshot renders the frame-0
    // tilemap/palette state before 0x3b77e writes gfxctrl_w in frame 1.
    let mut current = if first == 0 {
        let blank = sf_video::render::Frame {
            argb: vec![0xff00_0000; sf_video::render::WIDTH * sf_video::render::HEIGHT],
        };
        writer.push_frame(
            0,
            &blank,
            machine.state().sound_latch,
            machine.state().latch_cycles,
        );
        let frame0 = render_frame(assets, &power_on_frame0_scene(machine.state()));
        if write_checkpoint(0) {
            writer.push_checkpoint(0, &frame0, machine.state())?;
        }
        frame0
    } else {
        let current = render_frame(assets, &machine.state().scene());
        writer.push_frame(
            first,
            &current,
            machine.state().sound_latch,
            machine.state().latch_cycles,
        );
        if write_checkpoint(first) {
            writer.push_checkpoint(first, &current, machine.state())?;
        }
        current
    };

    let mut frames_executed = 1u64;
    for frame in (first + 1)..=last {
        let previous = machine.state().scene(); // S_(frame-1)
        before_step(machine, frame);
        machine.step(); // S_frame
                        // The frame's hash pixels: render(S_(frame-1)), except that a
                        // palette the vblank loaded this frame (0x3b362) is already in the
                        // scanout the hash reads, so the previous state is rendered with
                        // this frame's palette when it changed.
        let lagged = if machine.state().paletteram != previous.paletteram {
            let mut with_palette = previous;
            with_palette.paletteram = machine.state().paletteram.clone();
            render_frame(assets, &with_palette)
        } else {
            current
        };
        current = render_frame(assets, &machine.state().scene()); // checkpoint pixels
        writer.push_frame(
            frame,
            &lagged,
            machine.state().sound_latch,
            machine.state().latch_cycles,
        );
        if write_checkpoint(frame) {
            writer.push_checkpoint(frame, &current, machine.state())?;
        }
        frames_executed += 1;
    }
    let (dsw1, dsw2) = machine.dsw();
    writer.finish_with_dsw(frames_executed, scenario_id, dsw1, dsw2)?;
    Ok(frames_executed)
}

/// Run the idle interval (ticket #5) and write the native capture.
pub fn run_idle_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
) -> io::Result<u64> {
    // Since ticket #29 the round-1 intro runs on the one-player machine
    // from its checkpoint (the game-flow task resumes in the translated
    // intro loop); the observed tables of ticket #5 are retired.
    run_cpu_fight_capture(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        IDLE_FIRST,
        IDLE_LAST,
    )
}

/// A uniquely named scratch directory under the system temp directory,
/// removed with its contents when dropped, also when a test panics. The
/// acceptance tests write their native captures (hundreds of MB each) into
/// one, so repeated runs do not fill the disk.
pub struct ScratchDir(std::path::PathBuf);

impl ScratchDir {
    /// A fresh directory named `label-<pid>-<nanos>` (not yet created; the
    /// capture writers create it).
    pub fn new(label: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        ScratchDir(std::env::temp_dir().join(format!("{label}-{}-{nanos}", std::process::id())))
    }
}

impl std::ops::Deref for ScratchDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for ScratchDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The idle scene's interval (ticket #5): the round-1 intro of
/// sf-idle-scene-01, from the checkpoint-2280 fixture to one frame before
/// the CPU's first decision.
pub const IDLE_FIRST: u32 = 2280;
pub const IDLE_LAST: u32 = 2388;

/// A frame-indexed logical input change for the native machines, derived
/// from the scenario's `inputs` (press frame and release frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectionEvent {
    frame: u32,
    control: Direction,
    /// P2's control (`false` for P1 and cabinet controls).
    p2: bool,
    held: bool,
}

impl DirectionEvent {
    fn input_index(self) -> usize {
        let idx = self.control.index();
        if idx < PER_PLAYER {
            idx + if self.p2 { PER_PLAYER } else { 0 }
        } else {
            PER_PLAYER + idx
        }
    }
}

/// The logical inputs the native machines understand: the four directions,
/// the six attack buttons (light / medium / heavy punch and kick), and the
/// cabinet controls (`coin1`, `coin2`, `start1`, `start2`, `service1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Right,
    Left,
    Down,
    Up,
    LightPunch,
    MediumPunch,
    HeavyPunch,
    LightKick,
    MediumKick,
    HeavyKick,
    Coin1,
    Coin2,
    Start1,
    Start2,
    Service1,
}

impl Direction {
    /// Index into the logical input enum: 0..=3 directions (right, left,
    /// down, up), 4..=9 buttons (LP, MP, HP, LK, MK, HK), 10..=14 cabinet
    /// controls (`coin1`, `coin2`, `start1`, `start2`, `service1`).
    fn index(self) -> usize {
        self as usize
    }
}

/// Number of logical inputs per player.
const PER_PLAYER: usize = 10;
/// Number of cabinet inputs (`coin1`, `coin2`, `start1`, `start2`, `service1`).
const CABINET_INPUTS: usize = 5;
/// Number of logical inputs ([`scenario_input_events`] indices): P1's ten
/// (`0..10`), P2's ten (`10..20`), then the five cabinet controls (`20..25`).
pub const INPUTS: usize = 2 * PER_PLAYER + CABINET_INPUTS;

fn match_player_control(control: &str) -> Option<Direction> {
    Some(match control {
        "right" => Direction::Right,
        "left" => Direction::Left,
        "down" => Direction::Down,
        "up" => Direction::Up,
        // The cabinet's buttons 1-3 are the punches, 4-6 the kicks
        // (sf_fixed_fight_01 names them so).
        "lp" | "button1" => Direction::LightPunch,
        "mp" | "button2" => Direction::MediumPunch,
        "hp" | "button3" => Direction::HeavyPunch,
        "lk" | "button4" => Direction::LightKick,
        "mk" | "button5" => Direction::MediumKick,
        "hk" | "button6" => Direction::HeavyKick,
        _ => return None,
    })
}

/// Pull the input schedule out of a scenario JSON: each `p1_*`, `p2_*`, or
/// cabinet (`coin1`, `coin2`, `start1`, `start2`, `service1`) press expands
/// into a press event at its frame and a release event after `hold_frames` —
/// the same translation the reference driver's generator
/// (`tools/reference/scenario.py`) performs.
fn versus_direction_events(scenario_path: &Path) -> io::Result<Vec<DirectionEvent>> {
    let text = std::fs::read_to_string(scenario_path)?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let inputs = json
        .get("inputs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "scenario lacks inputs"))?;
    let mut events = Vec::new();
    for input in inputs {
        let name = input.get("press").and_then(|v| v.as_str()).unwrap_or("");
        let (p2, control) = match name {
            "coin1" => (false, Direction::Coin1),
            "coin2" => (false, Direction::Coin2),
            "start1" => (false, Direction::Start1),
            "start2" => (false, Direction::Start2),
            "service1" => (false, Direction::Service1),
            _ => match name.split_once('_') {
                Some(("p1", c)) => match match_player_control(c) {
                    Some(ctrl) => (false, ctrl),
                    None => continue,
                },
                Some(("p2", c)) => match match_player_control(c) {
                    Some(ctrl) => (true, ctrl),
                    None => continue,
                },
                _ => continue,
            },
        };
        let frame =
            input.get("frame").and_then(|v| v.as_u64()).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "input event lacks frame")
            })? as u32;
        let hold = input
            .get("hold_frames")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "input event lacks hold_frames")
            })? as u32;
        events.push(DirectionEvent {
            frame,
            control,
            p2,
            held: true,
        });
        events.push(DirectionEvent {
            frame: frame + hold,
            control,
            p2,
            held: false,
        });
    }
    events.sort_by_key(|ev| ev.frame);
    Ok(events)
}

/// Resolve the scenario's DIP switch words `(dsw1, dsw2)` (defaulting to
/// `(0xdfff, 0xffff)` when `machine.dipswitches` is omitted or set to a
/// default preset).
pub fn scenario_dsw(scenario_path: &Path) -> io::Result<(u16, u16)> {
    let text = std::fs::read_to_string(scenario_path)?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let raw = json.get("machine").and_then(|m| m.get("dipswitches"));
    sf_compare::scenario::parse_dipswitches(raw)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

/// The scenario's input schedule as (frame, input index, held) triples.
/// Index 0..=3 = P1 right, left, down, up; 4..=9 = P1 LP, MP, HP, LK, MK,
/// HK; 10..=19 the same for P2; 20..=24 = `coin1`, `coin2`, `start1`,
/// `start2`, `service1`.
pub fn scenario_input_events(scenario_path: &Path) -> io::Result<Vec<(u32, usize, bool)>> {
    Ok(versus_direction_events(scenario_path)?
        .into_iter()
        .map(|ev| (ev.frame, ev.input_index(), ev.held))
        .collect())
}

/// Apply one frame's logical inputs of both players and the cabinet (see
/// [`scenario_input_events`]).
pub fn apply_inputs(machine: &mut FightMachine, held: &[bool; INPUTS]) {
    machine.set_p1_direction(held[0], held[1], held[2], held[3]);
    machine.set_p1_buttons([held[4], held[5], held[6], held[7], held[8], held[9]]);
    machine.set_p2_direction(held[10], held[11], held[12], held[13]);
    machine.set_p2_buttons([held[14], held[15], held[16], held[17], held[18], held[19]]);
    machine.set_cabinet_inputs(held[20], held[21], held[22], held[23], held[24]);
}

/// Versus compatibility wrapper around the shared fight capture path.
fn run_versus_scene(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario_path: &Path,
    last: u32,
) -> io::Result<u64> {
    run_fight_capture(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        Some(scenario_path),
        VERSUS_FIRST,
        last,
    )
}

/// Run any versus-fixture scenario from the checkpoint-2450 fixture through
/// `last` and write the native capture (the per-ticket wrappers below fix
/// their intervals).
pub fn run_versus_interval_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario_path: &Path,
    last: u32,
) -> io::Result<u64> {
    run_versus_scene(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        scenario_path,
        last,
    )
}

/// The versus fixture's round timer (TIME 99 at 2450, a tick every 19
/// frames from 2449) reaches zero at this frame, where the time-over path
/// (0x3ed20, ticket #30) judges the round by health.
pub const VERSUS_TIME_OVER: u32 = 2449 + 99 * 19;

/// The memory the translated routines own in the versus fixture, compared
/// byte for byte by [`compare_versus_ram`]: (name, first address, end).
pub const VERSUS_RAM_REGIONS: &[(&str, u32, u32)] = &[
    ("effect slots", 0xffe100, 0xffe400),
    ("Ryu entity", 0xffe400, 0xffea00),
    ("Ken entity", 0xffea00, 0xfff000),
    ("P1 struct (latches, pointers, health, trackers, score)", 0xff86b0, 0xff86da),
    ("P2 struct", 0xff86da, 0xff8704),
    ("Tatsumaki loop counts and starts", 0xff8704, 0xff8710),
    ("boot counter", 0xff8622, 0xff8624),
    ("sound queue", 0xff865e, 0xff8682),
    ("gfxctrl copy", 0xff8682, 0xff8683),
    ("button latches", 0xff8690, 0xff86b0),
    ("IN1 copy", 0xff868e, 0xff8690),
    ("random state", 0xff8e4a, 0xff8e52),
    ("ranking table", 0xff8e6e, 0xff8f0e),
    ("camera", 0xffc00c, 0xffc00e),
    ("top score", 0xffc012, 0xffc01a),
    ("start-screen blink counters", 0xffc01a, 0xffc01c),
    ("opponent AI table pointers", 0xffc01c, 0xffc050),
    ("scroll copies", 0xffc680, 0xffc684),
    ("scheduler: vblank flag, current slot, frame counters", 0xff8000, 0xff8006),
    ("frame-done and palette flags", 0xff8632, 0xff8634),
    ("coin slots, credits", 0xff863a, 0xff865e),
    ("coin-control copy", 0xff8684, 0xff8686),
    ("text sound", 0xff868c, 0xff868e),
    ("round-timer task (flags, TIME, frame counter, tallies)", 0xff8e16, 0xff8e36),
    ("watch continuation, difficulty copy", 0xff8f20, 0xff8f29),
    ("screen-event request", 0xff8e36, 0xff8e37),
    ("round result: winner, TIME copy, vitality and bonus digits", 0xff8e38, 0xff8e49),
    ("ranking check and name entry state", 0xff8e52, 0xff8e6e),
    ("game state: free play, mode, stage, fight, country, difficulty, scene, winner, round, slow motion", 0xffc000, 0xffc00c),
    ("palette request: flag, source, offset", 0xff8633, 0xff863a),
];

/// Each task slot's state and sleep-count bytes (0xff8006 + 0x10k, +0 and
/// +1), compared with the regions; the saved SR / PC / USP words and the
/// tasks' stacks are not reproduced (see `tasks`).
fn slot_bytes() -> impl Iterator<Item = u32> {
    (0..16u32).flat_map(|k| [0xff8006 + 0x10 * k, 0xff8007 + 0x10 * k])
}

/// Replay a versus-fixture scenario natively and compare, after every frame
/// through `last`, the memory the translated routines own
/// ([`VERSUS_RAM_REGIONS`]) with the reference capture's work-RAM dump of
/// that frame. Returns the number of frames compared, or a description of
/// the first differing byte (or of an untranslated branch reached).
pub fn compare_versus_ram(
    reference: &Path,
    program_rom: Vec<u8>,
    scenario_path: &Path,
    last: u32,
) -> io::Result<Result<u32, String>> {
    let mut m = VersusMachine::from_checkpoint_2450_with_program(reference, program_rom)
        .map_err(|e| io::Error::other(format!("checkpoint-2450 fixture: {e}")))?;
    let (dsw1, dsw2) = scenario_dsw(scenario_path)?;
    m.set_dsw(dsw1, dsw2);
    let events = scenario_input_events(scenario_path)?;
    let (mut next, mut held, mut compared) = (0, [false; INPUTS], 0);
    while m.state.frame < last {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        let unrecovered = m.unrecovered();
        if !unrecovered.is_empty() {
            return Ok(Err(format!(
                "frame {frame}: untranslated branches {unrecovered:05x?} reached after {compared} matching frames"
            )));
        }
        let work_path = reference.join(format!("ram_{frame:06}_work.bin"));
        if !work_path.exists() {
            continue;
        }
        let work = std::fs::read(work_path)?;
        let native_work = m.work_ram();
        let slots = slot_bytes().map(|a| ("task slot state / sleep", a, a + 1));
        for (name, a, b) in VERSUS_RAM_REGIONS.iter().copied().chain(slots) {
            for addr in a..b {
                let i = (addr - 0xff8000) as usize;
                let native = if addr >= 0xffe000 {
                    m.state.objectram[i - 0x6000]
                } else {
                    native_work[i]
                };
                if work[i] != native {
                    return Ok(Err(format!(
                        "frame {frame}: {name} differs first at 0x{addr:06x}: reference {:02x}, native {native:02x}",
                        work[i]
                    )));
                }
            }
        }
        compared += 1;
    }
    let unrecovered = m.unrecovered();
    if !unrecovered.is_empty() {
        return Ok(Err(format!(
            "run reached untranslated or unverified branches {unrecovered:05x?}"
        )));
    }
    Ok(Ok(compared))
}

/// The memory the translated routines own in one-player play, compared
/// byte for byte by [`compare_cpu_ram`]: (name, first address, end).
pub const CPU_RAM_REGIONS: &[(&str, u32, u32)] = &[
    ("object RAM (both fighters, effects)", 0xffe000, 0x1000000),
    ("P1 struct", 0xff86b0, 0xff86da),
    ("P2 struct", 0xff86da, 0xff8704),
    ("CPU struct", 0xffc664, 0xffc68e),
    ("Tatsumaki loop counts and starts", 0xff8704, 0xff8710),
    ("AI tables", 0xffc058, 0xffc664),
    ("CPU reaction table pointers", 0xffc050, 0xffc058),
    ("boot counter", 0xff8622, 0xff8624),
    ("sound queue", 0xff865e, 0xff8682),
    ("gfxctrl copy", 0xff8682, 0xff8683),
    ("IN1 copy and button latches", 0xff868e, 0xff86b0),
    ("random state", 0xff8e4a, 0xff8e52),
    ("ranking table", 0xff8e6e, 0xff8f0e),
    ("bonus stage state", 0xff8f0e, 0xff8f20),
    ("camera, drift", 0xffc00c, 0xffc012),
    ("top score", 0xffc012, 0xffc01a),
    ("start-screen blink counters", 0xffc01a, 0xffc01c),
    ("opponent AI table pointers", 0xffc01c, 0xffc050),
    ("scroll copies", 0xffc680, 0xffc684),
    ("scheduler: vblank flag, current slot, frame counters", 0xff8000, 0xff8006),
    ("frame-done flag, palette request (flag, source, offset)", 0xff8632, 0xff863a),
    ("coin slots, credits", 0xff863a, 0xff865e),
    ("coin-control copy", 0xff8684, 0xff8686),
    ("aux-control copy", 0xff8686, 0xff8688),
    ("text sound", 0xff868c, 0xff868e),
    ("service-mode state", 0xff872c, 0xff886c),
    ("watch continuation, difficulty copy", 0xff8f20, 0xff8f29),
    ("round-timer task (flags, TIME, frame counter, tallies)", 0xff8e16, 0xff8e36),
    ("screen-event request", 0xff8e36, 0xff8e37),
    ("round result: winner, TIME copy, vitality and bonus digits", 0xff8e38, 0xff8e49),
    ("ranking check and name entry state", 0xff8e52, 0xff8e6e),
    ("game state: free play, mode, stage, fight, country, difficulty, scene, winner, round, slow motion", 0xffc000, 0xffc00c),
];

/// How far a native one-player run from a fight checkpoint agreed with the
/// reference ([`compare_cpu_ram`]).
#[derive(Debug, PartialEq, Eq)]
pub enum CpuRamOutcome {
    /// Every frame through the last matched.
    Match { frames: u32 },
    /// The run matched for `frames` frames, then reached the listed
    /// untranslated (or unverified) branches at `frame`; that frame and
    /// later ones are not compared.
    Stopped {
        frames: u32,
        frame: u32,
        unrecovered: Vec<u32>,
    },
    /// A byte differed before any untranslated branch was reached.
    Mismatch(String),
}

/// Replay one-player play (P1 idle) natively from the fight checkpoint
/// `first` of a reference capture and compare [`CPU_RAM_REGIONS`] with the
/// reference work-RAM dump after every frame through `last`, stopping at
/// the first untranslated branch.
pub fn compare_cpu_ram(
    reference: &Path,
    program_rom: Vec<u8>,
    first: u32,
    last: u32,
) -> io::Result<CpuRamOutcome> {
    compare_cpu_ram_with_inputs(reference, program_rom, None, first, last)
}

/// Player logical inputs of a scenario's events, applied to the one-player
/// machine before each step (P2's inputs reach `IN1` / `IN0` so `0xff868e`
/// `IN1_COPY` matches MAME when P2 keys are pressed during a 1P session).
struct P1Inputs {
    events: Vec<(u32, usize, bool)>,
    next: usize,
    held: [bool; INPUTS],
    dsw: Option<(u16, u16)>,
    dip_changes: Vec<(u32, u16, u16)>,
    next_dip: usize,
}

impl P1Inputs {
    fn load(scenario: Option<&Path>) -> io::Result<Self> {
        let (events, dsw, dip_changes) = match scenario {
            Some(p) => {
                let scenario = sf_compare::scenario::load_scenario(p)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                let changes = scenario
                    .dip_changes
                    .iter()
                    .map(|change| (change.frame as u32, change.dsw1, change.dsw2))
                    .collect();
                (
                    scenario_input_events(p)?,
                    Some((scenario.dsw1, scenario.dsw2)),
                    changes,
                )
            }
            None => (Vec::new(), None, Vec::new()),
        };
        Ok(P1Inputs {
            events,
            next: 0,
            held: [false; INPUTS],
            dsw,
            dip_changes,
            next_dip: 0,
        })
    }

    /// Apply the events of frames before `frame` (the input a frame sees
    /// is the one set before its step), then set the machine's inputs. A DIP
    /// write at reference frame-done N is likewise visible to native N+1.
    fn apply(&mut self, m: &mut CpuMachine, frame: u32) {
        while self.next_dip < self.dip_changes.len() && self.dip_changes[self.next_dip].0 < frame {
            let (_, dsw1, dsw2) = self.dip_changes[self.next_dip];
            self.dsw = Some((dsw1, dsw2));
            self.next_dip += 1;
        }
        if let Some((dsw1, dsw2)) = self.dsw {
            m.set_dsw(dsw1, dsw2);
        }
        while self.next < self.events.len() && self.events[self.next].0 < frame {
            let (_, i, held) = self.events[self.next];
            self.held[i] = held;
            self.next += 1;
        }
        let h = &self.held;
        m.set_p1_direction(h[0], h[1], h[2], h[3]);
        m.set_p1_buttons([h[4], h[5], h[6], h[7], h[8], h[9]]);
        m.set_p2_direction(h[10], h[11], h[12], h[13]);
        m.set_p2_buttons([h[14], h[15], h[16], h[17], h[18], h[19]]);
        m.set_cabinet_inputs(h[20], h[21], h[22], h[23], h[24]);
    }

    fn apply_power_on(&mut self, m: &mut PowerOnMachine, frame: u32) {
        while self.next_dip < self.dip_changes.len() && self.dip_changes[self.next_dip].0 < frame {
            let (_, dsw1, dsw2) = self.dip_changes[self.next_dip];
            self.dsw = Some((dsw1, dsw2));
            self.next_dip += 1;
        }
        if let Some((dsw1, dsw2)) = self.dsw {
            m.set_dsw(dsw1, dsw2);
        }
        while self.next < self.events.len() && self.events[self.next].0 < frame {
            let (_, i, held) = self.events[self.next];
            self.held[i] = held;
            self.next += 1;
        }
        let h = &self.held;
        m.set_p1_direction(h[0], h[1], h[2], h[3]);
        m.set_p1_buttons([h[4], h[5], h[6], h[7], h[8], h[9]]);
        m.set_p2_direction(h[10], h[11], h[12], h[13]);
        m.set_p2_buttons([h[14], h[15], h[16], h[17], h[18], h[19]]);
        m.set_cabinet_inputs(h[20], h[21], h[22], h[23], h[24]);
    }
}

/// Run a versus fight natively from checkpoint 2450 of `fixture_capture`,
/// both players on `scenario`'s input schedule, calling `observe` after
/// every frame produced through `last`. Stops at the first untranslated
/// original branch, before observing that frame. Returns the last frame
/// produced and the branches reached (empty when the run reached `last`).
pub fn observe_versus_fight(
    fixture_capture: &Path,
    program_rom: Vec<u8>,
    scenario: &Path,
    last: u32,
    mut observe: impl FnMut(&VersusMachine),
) -> io::Result<(u32, Vec<u32>)> {
    let mut m = VersusMachine::from_checkpoint_2450_with_program(fixture_capture, program_rom)
        .map_err(|e| io::Error::other(format!("checkpoint-2450 fixture: {e}")))?;
    let (dsw1, dsw2) = scenario_dsw(scenario)?;
    m.set_dsw(dsw1, dsw2);
    let events = scenario_input_events(scenario)?;
    let (mut next, mut held) = (0usize, [false; INPUTS]);
    while m.state.frame < last {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        let unrecovered = m.unrecovered();
        if !unrecovered.is_empty() {
            return Ok((m.state.frame, unrecovered));
        }
        observe(&m);
    }
    Ok((m.state.frame, Vec::new()))
}

/// The versus machine's `0xff8000..0xffffff` image (`work_ram` followed by
/// `objectram`): the range of the reference's `work` dumps and `ram_crc`.
pub fn versus_work_image(m: &VersusMachine) -> Vec<u8> {
    work_image_bytes(m.work_ram(), &m.state.objectram)
}

/// The addresses of that image the translated routines own in versus play
/// ([`VERSUS_RAM_REGIONS`] and the task slots' state bytes), each with its
/// region's name.
pub fn versus_owned_addresses() -> Vec<(&'static str, u32)> {
    let slots = slot_bytes().map(|a| ("task slot state / sleep", a, a + 1));
    VERSUS_RAM_REGIONS
        .iter()
        .copied()
        .chain(slots)
        .flat_map(|(name, a, b)| (a..b).map(move |addr| (name, addr)))
        .collect()
}

/// Run a one-player fight natively from checkpoint `first` of
/// `fixture_capture`, P1 on `scenario`'s input schedule (or idle), calling
/// `observe` after every frame produced through `last` (the milestone
/// audit, ticket #39). Stops at the first untranslated original branch,
/// before observing that frame. Returns the last frame produced and the
/// branches reached (empty when the run reached `last`).
pub fn observe_cpu_fight(
    fixture_capture: &Path,
    program_rom: Vec<u8>,
    scenario: Option<&Path>,
    first: u32,
    last: u32,
    mut observe: impl FnMut(&CpuMachine),
) -> io::Result<(u32, Vec<u32>)> {
    let mut m = CpuMachine::from_fight_checkpoint(fixture_capture, first, program_rom)
        .map_err(|e| io::Error::other(format!("fight checkpoint {first} fixture: {e}")))?;
    let mut inputs = P1Inputs::load(scenario)?;
    while m.state.frame < last {
        let frame = m.state.frame + 1;
        inputs.apply(&mut m, frame);
        m.step();
        let unrecovered = m.unrecovered();
        if !unrecovered.is_empty() {
            return Ok((m.state.frame, unrecovered));
        }
        observe(&m);
    }
    Ok((m.state.frame, Vec::new()))
}

fn work_image_bytes(work: &[u8], objectram: &[u8]) -> Vec<u8> {
    let mut image = work.to_vec();
    image.extend_from_slice(objectram);
    image
}

/// The one-player machine's 0xff8000-0xffffff image (work RAM, then
/// object RAM): the range of the reference's `work` dumps and `ram_crc`.
pub fn cpu_work_image(m: &CpuMachine) -> Vec<u8> {
    work_image_bytes(m.work_ram(), &m.state.objectram)
}

/// The addresses of that image the translated routines own in one-player
/// play ([`CPU_RAM_REGIONS`] and the task slots' state bytes), each with
/// its region's name.
pub fn cpu_owned_addresses() -> Vec<(&'static str, u32)> {
    let slots = slot_bytes().map(|a| ("task slot state / sleep", a, a + 1));
    CPU_RAM_REGIONS
        .iter()
        .copied()
        .chain(slots)
        .flat_map(|(name, a, b)| (a..b).map(move |addr| (name, addr)))
        .collect()
}

/// The shared fight comparison owns the union of the versus and CPU regions.
/// `CPU_RAM_REGIONS` is already that superset, including 0xff8e52..0xff8f20.
pub const FIGHT_RAM_REGIONS: &[(&str, u32, u32)] = CPU_RAM_REGIONS;

/// The shared machine's complete work/object RAM image.
pub fn fight_work_image(m: &FightMachine) -> Vec<u8> {
    work_image_bytes(m.work_ram(), &m.state.objectram)
}

/// The shared comparison's addresses, including task state and sleep bytes.
pub fn fight_owned_addresses() -> Vec<(&'static str, u32)> {
    cpu_owned_addresses()
}

/// Replay either fight mode from any captured checkpoint and compare the
/// union of owned RAM after every frame. Both players use the scenario's
/// input schedule when one is supplied.
pub fn compare_fight_ram_with_inputs(
    reference: &Path,
    program_rom: Vec<u8>,
    scenario: Option<&Path>,
    first: u32,
    last: u32,
) -> io::Result<CpuRamOutcome> {
    let mut m = FightMachine::from_fight_checkpoint(reference, first, program_rom)
        .map_err(|e| io::Error::other(format!("fight checkpoint {first} fixture: {e}")))?;
    let mut inputs = P1Inputs::load(scenario)?;
    let mut frames = 0;
    while m.state.frame < last {
        let frame = m.state.frame + 1;
        inputs.apply(&mut m, frame);
        m.step();
        let frame = m.state.frame;
        let unrecovered = m.unrecovered();
        if !unrecovered.is_empty() {
            return Ok(CpuRamOutcome::Stopped {
                frames,
                frame,
                unrecovered,
            });
        }
        let work_path = reference.join(format!("ram_{frame:06}_work.bin"));
        if !work_path.exists() {
            continue;
        }
        let work = std::fs::read(work_path)?;
        let slots = slot_bytes().map(|a| ("task slot state / sleep", a, a + 1));
        for (name, a, b) in FIGHT_RAM_REGIONS.iter().copied().chain(slots) {
            for addr in a..b {
                let i = (addr - 0xff8000) as usize;
                let native = if addr >= 0xffe000 {
                    m.state.objectram[i - 0x6000]
                } else {
                    m.work_ram()[i]
                };
                if work[i] != native {
                    return Ok(CpuRamOutcome::Mismatch(format!(
                        "frame {frame}: {name} differs first at 0x{addr:06x}: reference {:02x}, native {native:02x}",
                        work[i]
                    )));
                }
            }
        }
        frames += 1;
    }
    Ok(CpuRamOutcome::Match { frames })
}

/// One-player compatibility wrapper for the shared comparison.
pub fn compare_cpu_ram_with_inputs(
    reference: &Path,
    program_rom: Vec<u8>,
    scenario: Option<&Path>,
    first: u32,
    last: u32,
) -> io::Result<CpuRamOutcome> {
    compare_fight_ram_with_inputs(reference, program_rom, scenario, first, last)
}

/// One copy of the AI-table setup: (ROM source, work-RAM destination,
/// words).
pub type AiTableCopy = (u32, u32, u32);

/// Run the translated CPU-opponent setup (0x44880 / 0x44860,
/// `Bus::setup_cpu_opponent`) over a copy of a reference work-RAM dump for
/// opponent `stage`, `fight`, `difficulty` and starting-country byte
/// `country` (0xffc006, whose bit 1 is the table variant). Returns the work
/// RAM after the setup and the copies it made.
pub fn cpu_opponent_setup(
    program_rom: Vec<u8>,
    work_dump: &[u8],
    stage: u8,
    fight: u8,
    difficulty: u8,
    country: u8,
) -> Option<(Vec<u8>, Vec<AiTableCopy>)> {
    let mut program = Program::from_fixture(program_rom, work_dump)?;
    let gfx = work_dump[0x682];
    let mut state = MachineState::new(
        vec![0; 0x1000],
        work_dump.get(0x6000..0x8000)?.to_vec(),
        vec![0; 0x800],
        0,
        0,
        gfx,
        gfx & 4 != 0,
        0xff,
        0,
    );
    let mut bus = program.bus(&mut state);
    for (a, v) in [
        (0xffc003, stage),
        (0xffc004, fight),
        (0xffc007, difficulty),
        (0xffc006, country),
    ] {
        bus.wb(a, v);
    }
    let copies = bus.ai_table_copies();
    bus.setup_cpu_opponent();
    Some((program.work, copies))
}

/// The round-timer task's memory (0xff8e16..0xff8e36: flags, TIME digits,
/// frame counter, round-time setting, the four score-tally counters).
pub const TIMER_RAM: (u32, u32) = (0xff8e16, 0xff8e36);

/// One pass of the translated round-timer task (`tasks::round_timer`) over
/// a copy of a reference work-RAM dump; returns the dump with the pass
/// applied. With the dump of frame f-1 this predicts the task's memory at
/// frame f: the task runs first in slot order, so what the other tasks
/// wrote in frame f-1 is what it sees.
pub fn timer_pass(program_rom: Vec<u8>, work_dump: &[u8]) -> Option<Vec<u8>> {
    let mut program = Program::from_fixture(program_rom, work_dump)?;
    let gfx = work_dump[0x682];
    let mut state = MachineState::new(
        vec![0; 0x1000],
        work_dump.get(0x6000..0x8000)?.to_vec(),
        vec![0; 0x800],
        0,
        0,
        gfx,
        gfx & 4 != 0,
        0xff,
        0,
    );
    crate::tasks::round_timer(&mut program.bus(&mut state));
    Some(program.work)
}

/// Run the versus movement interval (ticket #6) and write the native
/// capture.
pub fn run_versus_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario_path: &Path,
) -> io::Result<u64> {
    run_versus_scene(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        scenario_path,
        VERSUS_LAST,
    )
}

/// Run the versus crouch interval (ticket #7) and write the native capture.
pub fn run_crouch_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario_path: &Path,
) -> io::Result<u64> {
    run_versus_scene(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        scenario_path,
        CROUCH_LAST,
    )
}

/// Run the versus jump interval (ticket #8) and write the native capture.
pub fn run_jump_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario_path: &Path,
) -> io::Result<u64> {
    run_versus_scene(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        scenario_path,
        JUMP_LAST,
    )
}

/// Run the versus light-punch interval (ticket #9) and write the native
/// capture.
pub fn run_punch_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario_path: &Path,
) -> io::Result<u64> {
    run_versus_scene(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        scenario_path,
        PUNCH_LAST,
    )
}

/// Run one-player play (P1 idle) from a fight checkpoint `first` of a
/// reference capture through `last` and write the native capture (ticket
/// #26). The row of `first` is the fixture state; its screen hash (which
/// the reference takes from the previous frame's render) is not
/// comparable, so comparisons start at `first + 1`.
pub fn run_cpu_fight_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    first: u32,
    last: u32,
) -> io::Result<u64> {
    run_cpu_fight_capture_with_inputs(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        None,
        first,
        last,
    )
}

/// Capture either fight mode on the shared machine from any checkpoint.
pub fn run_fight_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario: Option<&Path>,
    first: u32,
    last: u32,
) -> io::Result<u64> {
    let mut machine =
        FightMachine::from_fight_checkpoint(fixture_capture, first, assets.maincpu.clone())
            .map_err(|e| io::Error::other(format!("fight checkpoint {first} fixture: {e}")))?;
    let mut inputs = P1Inputs::load(scenario)?;
    let scenario_checkpoints: std::collections::BTreeSet<u32> = match scenario {
        Some(p) => sf_compare::scenario::load_scenario(p)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?
            .checkpoints
            .into_iter()
            .map(|c| c as u32)
            .collect(),
        None => std::collections::BTreeSet::new(),
    };
    let frames = drive_filtered(
        &mut machine,
        assets,
        out_dir,
        scenario_id,
        first,
        last,
        |frame| {
            scenario.is_none()
                || scenario_checkpoints.contains(&frame)
                || fixture_capture
                    .join(format!("ram_{frame:06}_work.bin"))
                    .exists()
        },
        |m, frame| inputs.apply(m, frame),
    )?;
    let unrecovered = machine.unrecovered();
    if !unrecovered.is_empty() {
        return Err(io::Error::other(format!(
            "run reached untranslated original branches: {unrecovered:05x?}"
        )));
    }
    Ok(frames)
}

/// One-player compatibility wrapper for the shared capture path.
pub fn run_cpu_fight_capture_with_inputs(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario: Option<&Path>,
    first: u32,
    last: u32,
) -> io::Result<u64> {
    run_fight_capture(
        fixture_capture,
        assets,
        out_dir,
        scenario_id,
        scenario,
        first,
        last,
    )
}

/// Run the CPU-decision interval (ticket #12) and write the native capture.
/// P1 is idle in this scenario, so no input schedule is read.
pub fn run_cpu_capture(
    fixture_capture: &Path,
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
) -> io::Result<u64> {
    let mut machine = CpuMachine::from_checkpoint_2389(fixture_capture, assets.maincpu.clone())
        .map_err(|e| io::Error::other(format!("checkpoint-2389 fixture: {e}")))?;
    let frames = drive(
        &mut machine,
        assets,
        out_dir,
        scenario_id,
        CPU_FIRST,
        CPU_LAST,
        |_, _| {},
    )?;
    let unrecovered = machine.unrecovered();
    if !unrecovered.is_empty() {
        let list: Vec<String> = unrecovered.iter().map(|a| format!("0x{a:05x}")).collect();
        return Err(io::Error::other(format!(
            "run reached untranslated original branches: {}",
            list.join(", ")
        )));
    }
    Ok(frames)
}

/// Last frame of the power-on boot set-up interval (ticket #41, F1):
/// `0x3c112..0x3c204` overruns frame 6 into frame 7 and sleeps 1 frame at
/// `0x3c20a`, so frame 7 is the last verified frame before `0x3c210` in
/// frame 8.
pub const BOOT_SETUP_LAST: u32 = 7;

/// Last frame of the power-on warning screen and attract title interval
/// (ticket #55, F15): `0..=1647` from power-on with no fixture through the
/// warning typewriter, 240-frame hold, join at frame 794, and title loop up
/// to the attract ranking entry `0x4268c` at frame 1648.
pub const WARNING_TITLE_LAST: u32 = 1647;

/// End of the work-RAM prefix cleared by the reset loop (`0x3b018..0x3b01a`)
/// before frame 0's end (`133,333 1/3` cycles at 8 MHz / 60 Hz): 88 cycles
/// of reset entry + 4,441 completed 30-cycle `clr.l (A0)+; dbra` passes =
/// 133,318 cycles (`0xff8000 + 4,441 * 4 = 0xffc564`).
pub const RESET_CLEAR_FRAME0_END: u32 = 0xff8000 + 4_441 * 4;

/// Deterministic full-session state snapshot of a [`PowerOnMachine`] (`#93`, `O2`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowerOnSnapshot {
    pub state: MachineState,
    pub work: Vec<u8>,
    pub unrecovered: std::collections::BTreeSet<u32>,
    pub effect_stream_calls: std::collections::BTreeSet<u32>,
    pub match_config: crate::config::MatchConfig,
    pub toggle_state: crate::config::CoreToggleState,
    /// Training's rules' state (refill waits, the hit record), restored with
    /// the rest.
    pub training: crate::training::TrainingState,
    pub scheduler: crate::tasks::Scheduler,
    pub io: crate::tasks::Io,
    pub freeze_vblank_cycles: Option<u32>,
    pub latch_writes: Vec<crate::tasks::LatchWrite>,
    pub overlay: crate::overlay::OverlayState,
    /// The fair CPU on Player 2's pad (SP21, `#264`), while one plays.
    pub fair_cpu: Option<crate::fair_cpu::FairCpu>,
    /// The Dramatic boss controller and the controller for solo Ally 2.
    pub dramatic_cpu: Option<Box<[crate::fair_cpu::FairCpu; 2]>>,
}

/// The common authoritative state of a live machine and a saved snapshot.
/// Borrowing this view keeps one schema order without cloning live state.
struct PowerOnGameplay<'a> {
    state: &'a MachineState,
    work: &'a [u8],
    match_config: &'a crate::config::MatchConfig,
    toggle_state: &'a crate::config::CoreToggleState,
    training: &'a crate::training::TrainingState,
    scheduler: &'a crate::tasks::Scheduler,
    io: &'a crate::tasks::Io,
    freeze_vblank_cycles: &'a Option<u32>,
    fair_cpu: &'a Option<crate::fair_cpu::FairCpu>,
    dramatic_cpu: Option<&'a [crate::fair_cpu::FairCpu; 2]>,
}

impl crate::canonical::Canonical for PowerOnGameplay<'_> {
    fn encode(&self, out: &mut crate::canonical::CanonicalHasher) {
        let Self {
            state, work, match_config, toggle_state, training, scheduler, io,
            freeze_vblank_cycles, fair_cpu, dramatic_cpu,
        } = self;
        out.tag("PowerOnGameplay.v2");
        state.encode(out);
        work.encode(out);
        match_config.encode(out);
        toggle_state.encode(out);
        training.encode(out);
        scheduler.encode(out);
        io.encode(out);
        freeze_vblank_cycles.encode(out);
        fair_cpu.encode(out);
        dramatic_cpu.encode(out);
    }
}

impl PowerOnSnapshot {
    pub(crate) fn encode_gameplay(&self, out: &mut crate::canonical::CanonicalHasher) {
        use crate::canonical::Canonical;
        let Self {
            state, work, match_config, toggle_state, training, scheduler, io,
            freeze_vblank_cycles, fair_cpu, dramatic_cpu,
            unrecovered: _, effect_stream_calls: _, latch_writes: _, overlay: _,
        } = self;
        PowerOnGameplay {
            state, work, match_config, toggle_state, training, scheduler, io,
            freeze_vblank_cycles, fair_cpu, dramatic_cpu: dramatic_cpu.as_deref(),
        }.encode(out);
        // Untranslated-address sets and overlay history are diagnostics.
        // latch_writes is emitted audio history; MachineState's actual latch
        // register/timing remain authoritative and are encoded above.
    }
}

#[cfg(test)]
mod canonical_gameplay_tests {
    use super::{PowerOnMachine, PowerOnSnapshot};
    use crate::canonical::CanonicalHasher;

    fn checksum(machine: &PowerOnMachine) -> u64 {
        let mut out = CanonicalHasher::new();
        machine.encode_gameplay(&mut out);
        out.finish()
    }

    #[test]
    fn canonical_gameplay_tracks_machine_frame_and_matches_a_saved_snapshot() {
        let mut machine = PowerOnMachine::new(Vec::new());
        let original = checksum(&machine);
        machine.state.frame = 1;
        assert_ne!(checksum(&machine), original, "the simulation frame changes gameplay state");
        let mut saved = CanonicalHasher::new();
        machine.save_snapshot().encode_gameplay(&mut saved);
        assert_eq!(checksum(&machine), saved.finish(), "live and saved state use the same encoding");
    }

    #[test]
    fn canonical_gameplay_tracks_state_outside_machine_ram() {
        let mut machine = PowerOnMachine::new(Vec::new());
        let original = machine.save_snapshot();
        let expected = checksum(&machine);
        let changes: [(&str, fn(&mut PowerOnSnapshot)); 8] = [
            ("work RAM", |s| s.work[0] ^= 1),
            ("numeric rules", |s| s.match_config.toggles.recovery_buffer_window += 1),
            ("modern-rule state", |s| s.toggle_state.combo.hitstun[0] = 1),
            ("input I/O", |s| s.io.in1 ^= 1),
            ("freeze timing", |s| s.freeze_vblank_cycles = Some(1)),
            ("training wait", |s| s.training.since_contact[0] = 1),
            ("fair CPU", |s| s.fair_cpu = Some(crate::fair_cpu::FairCpu::new())),
            ("scheduler continuation", |s| {
                let mut work = vec![0; 0x6000];
                let slot = (crate::tasks::slot_address(1) - 0xff8000) as usize;
                work[slot] = 8;
                work[slot + 8..slot + 12].copy_from_slice(&0xff9000u32.to_be_bytes());
                work[0x1000 + 60..0x1000 + 64].copy_from_slice(&0x3b5a8u32.to_be_bytes());
                work[0x1000 + 64..0x1000 + 68].copy_from_slice(&0x3eafa_u32.to_be_bytes());
                s.scheduler = crate::tasks::Scheduler::from_fixture(&work).expect("a sleeping fight-loop task");
            }),
        ];
        for (name, change) in changes {
            let mut changed = original.clone();
            change(&mut changed);
            machine.restore_snapshot(&changed);
            assert_ne!(checksum(&machine), expected, "{name} affects future gameplay");
            let mut saved = CanonicalHasher::new();
            changed.encode_gameplay(&mut saved);
            assert_eq!(checksum(&machine), saved.finish(), "{name}: live and saved encoding agree");
        }
    }

    #[test]
    fn canonical_gameplay_ignores_local_diagnostics_and_delivered_audio() {
        let mut machine = PowerOnMachine::new(Vec::new());
        let original = checksum(&machine);
        let mut saved = machine.save_snapshot();
        saved.unrecovered.insert(0x1234);
        saved.effect_stream_calls.insert(0x5678);
        saved.overlay.enabled = true;
        saved.latch_writes.push(crate::tasks::LatchWrite { value: 8, cycles: 10 });
        saved.toggle_state.missing_ryu_theme = Some(0x41);
        saved.toggle_state.cancel.count[0] = 1;
        machine.restore_snapshot(&saved);
        assert_eq!(checksum(&machine), original);
        let mut out = CanonicalHasher::new();
        saved.encode_gameplay(&mut out);
        assert_eq!(out.finish(), original, "saved state excludes the same peer-local fields");
        machine.state.sound_latch = 8;
        assert_ne!(checksum(&machine), original, "the authoritative latch register is not delivery history");
    }

    #[test]
    fn canonical_gameplay_tracks_fair_cpu_percept_history() {
        let mut machine = PowerOnMachine::new(Vec::new());
        let mut saved = machine.save_snapshot();
        saved.fair_cpu = Some(crate::fair_cpu::FairCpu::new());
        machine.restore_snapshot(&saved);
        let original = checksum(&machine);
        saved.fair_cpu.as_mut().unwrap().observe(crate::fair_cpu::Percept { frame: 1, ..Default::default() });
        machine.restore_snapshot(&saved);
        assert_ne!(checksum(&machine), original, "what the CPU saw changes its future choices");
    }
}

/// A power-on Street Fighter machine built from the program ROM alone, with
/// zeroed RAM in every region and no checkpoint fixture (ticket #41, F1).
#[derive(Clone)]
pub struct PowerOnMachine {
    pub state: MachineState,
    program: Program,
    scheduler: crate::tasks::Scheduler,
    io: crate::tasks::Io,
    freeze_vblank_cycles: Option<u32>,
    latch_writes: Vec<crate::tasks::LatchWrite>,
    overlay: crate::overlay::OverlayState,
    /// The fair CPU (SP21, `#264`): under an overhaul preset, when the match
    /// configuration gives it Player 2's pad, it sets P2's stick and
    /// buttons before each frame, as a pad would, from what earlier frames
    /// showed; nothing else in the machine is touched.
    fair_cpu: Option<crate::fair_cpu::FairCpu>,
    /// Boss and, in solo Dramatic Battle, Ally 2. Allocate their perception
    /// histories only in Dramatic so other fights keep small snapshots.
    dramatic_cpu: Option<Box<[crate::fair_cpu::FairCpu; 2]>>,
}

impl PowerOnMachine {
    pub(crate) fn encode_gameplay(&self, out: &mut crate::canonical::CanonicalHasher) {
        use crate::canonical::Canonical;
        let Self {
            state, program, scheduler, io, freeze_vblank_cycles, fair_cpu, dramatic_cpu,
            latch_writes: _, overlay: _,
        } = self;
        let crate::fighter::Program {
            work, match_config, toggle_state, training,
            rom: _, backgrounds: _, data_bank: _,
            unrecovered: _, effect_stream_calls: _,
        } = program;
        PowerOnGameplay {
            state, work, match_config, toggle_state, training, scheduler, io,
            freeze_vblank_cycles, fair_cpu, dramatic_cpu: dramatic_cpu.as_deref(),
        }.encode(out);
        // ROM/immutable bank compatibility is checked by NP3, not rehashed
        // here on every frame. Local diagnostics/audio history are excluded.
    }
    /// Construct the power-on machine at frame 0 with default idle cabinet
    /// inputs and DIP switches (`dsw1 = 0xdfff`, `dsw2 = 0xffff`).
    pub fn new(program_rom: Vec<u8>) -> Self {
        Self::with_io(program_rom, crate::tasks::Io::idle())
    }

    /// Construct the power-on machine at frame 0 with DIP switches loaded
    /// from `scenario_path` and idle player inputs.
    pub fn from_scenario(program_rom: Vec<u8>, scenario_path: &Path) -> io::Result<Self> {
        let (dsw1, dsw2) = scenario_dsw(scenario_path)?;
        Ok(Self::with_io(
            program_rom,
            crate::tasks::Io {
                dsw1,
                dsw2,
                ..crate::tasks::Io::idle()
            },
        ))
    }

    /// Construct the power-on machine at frame 0 with the given cabinet I/O
    /// words.
    pub fn with_io(program_rom: Vec<u8>, io: crate::tasks::Io) -> Self {
        let mut state = MachineState::new(
            vec![0; 0x1000],
            vec![0; 0x2000],
            vec![0; 0x800],
            0,
            0,
            0,
            false,
            0x00,
            0,
        );
        state.frame = 0;
        PowerOnMachine {
            state,
            program: Program::power_on(program_rom),
            scheduler: crate::tasks::Scheduler::new(),
            io,
            freeze_vblank_cycles: None,
            latch_writes: Vec::new(),
            overlay: crate::overlay::OverlayState::default(),
            fair_cpu: None,
            dramatic_cpu: None,
        }
    }

    /// Set P1's logical direction inputs (`IN1` bits 0–3, active low).
    pub fn set_p1_direction(&mut self, right: bool, left: bool, down: bool, up: bool) {
        self.io.in1 |= 0x000f;
        for (held, bit) in [(right, 0), (left, 1), (down, 2), (up, 3)] {
            if held {
                self.io.in1 &= !(1 << bit);
            }
        }
    }

    /// Set P1's six attack-button levels (`LP`, `MP`, `HP`, `LK`, `MK`, `HK`).
    pub fn set_p1_buttons(&mut self, [lp, mp, hp, lk, mk, hk]: [bool; 6]) {
        self.io.in1 |= 0x00f0;
        for (held, bit) in [(lp, 4), (mp, 5), (lk, 6), (mk, 7)] {
            if held {
                self.io.in1 &= !(1 << bit);
            }
        }
        self.io.in0 |= 0x0204;
        for (held, mask) in [(hp, 0x0200), (hk, 0x0004)] {
            if held {
                self.io.in0 &= !mask;
            }
        }
    }

    /// Translate a human pad before the cabinet latches it. The physical
    /// input stays in the replay and on the network; only this frame's
    /// translated pad and shortcut reach the fight.
    pub(crate) fn translate_player_controls(
        &mut self, side: usize, input: crate::front::PlayerInputs, bypass: bool,
    ) -> crate::front::PlayerInputs {
        use crate::fighter::{CPU_STRUCT, GAME_MODE, P1_STRUCT, P2_STRUCT};
        use crate::session_io::ControlScheme;

        let config = self.program.match_config;
        let scheme = if bypass { ControlScheme::Classic } else { config.effective_control_schemes()[side] };
        let index = if side == 1 && config.dramatic.is_some() { 2 } else { side };
        let (facing_right, family, close) = if scheme == ControlScheme::Modern {
            let bus = self.program.bus(&mut self.state);
            // The second cabinet pad drives the opponent's CPU structure
            // when replay takeover overrides a one-player Arcade match.
            let ps = match (side, config.dramatic.is_some(), bus.rb(GAME_MODE)) {
                (0, _, _) => P1_STRUCT,
                (1, false, 2) => CPU_STRUCT,
                _ => P2_STRUCT,
            };
            let (me, other) = bus.player_entities(ps);
            (
                bus.eb(me, 0x37) & 4 == 0,
                bus.chosen_era(ps),
                bus.center_x(me).abs_diff(bus.center_x(other)) <= crate::combo::throw::RANGE,
            )
        } else {
            (true, None, false)
        };
        self.program.toggle_state.schemes[index]
            .translate(scheme, input, facing_right, family, close, config.modern_control_damage_penalty)
    }

    /// Set P2's logical direction inputs (`IN1` bits 8–11, active low).
    pub fn set_p2_direction(&mut self, right: bool, left: bool, down: bool, up: bool) {
        self.io.in1 |= 0x0f00;
        for (held, bit) in [(right, 8), (left, 9), (down, 10), (up, 11)] {
            if held {
                self.io.in1 &= !(1 << bit);
            }
        }
    }

    /// Set P2's six attack-button levels (`LP`, `MP`, `HP`, `LK`, `MK`, `HK`).
    pub fn set_p2_buttons(&mut self, [lp, mp, hp, lk, mk, hk]: [bool; 6]) {
        self.io.in1 |= 0xf000;
        for (held, bit) in [(lp, 12), (mp, 13), (lk, 14), (mk, 15)] {
            if held {
                self.io.in1 &= !(1 << bit);
            }
        }
        self.io.in0 |= 0x0500;
        for (held, mask) in [(hp, 0x0400), (hk, 0x0100)] {
            if held {
                self.io.in0 &= !mask;
            }
        }
    }

    /// Set the live DIP switch words (`DSW1`, `DSW2`).
    pub fn set_dsw(&mut self, dsw1: u16, dsw2: u16) {
        self.io.dsw1 = dsw1;
        self.io.dsw2 = dsw2;
    }

    /// Set the logical cabinet inputs (`IN0` bits 0–1 = `coin1`/`coin2`,
    /// `SYSTEM` bits 0–2 = `start1`/`start2`/`service1`, active low).
    pub fn set_cabinet_inputs(
        &mut self,
        coin1: bool,
        coin2: bool,
        start1: bool,
        start2: bool,
        service1: bool,
    ) {
        self.io.in0 |= 0x0003;
        for (held, mask) in [(coin1, 0x0001), (coin2, 0x0002)] {
            if held {
                self.io.in0 &= !mask;
            }
        }
        self.io.system |= 0x0007;
        for (held, mask) in [(start1, 0x0001), (start2, 0x0002), (service1, 0x0004)] {
            if held {
                self.io.system &= !mask;
            }
        }
    }

    /// Sound-latch writes produced by the current frame, in bus order.
    pub fn latch_writes(&self) -> &[crate::tasks::LatchWrite] {
        &self.latch_writes
    }

    pub(crate) fn move_effect_cues(&self) -> crate::fight_reader::MoveEffectCues {
        self.program.toggle_state.era.move_effects
    }

    /// Advance the machine by one 60 Hz frame: the fair CPU's pad first,
    /// when one plays, then the frame, then the CPU's look at it.
    pub fn step(&mut self) {
        self.step_with_training_pad(false);
    }

    /// A Training drill can supply P2's pad for this frame.
    pub(crate) fn step_with_training_pad(&mut self, scripted_p2: bool) {
        self.program.toggle_state.era.gauge_drive_spends = [false; 3];
        self.program.toggle_state.era.gauge_v_trigger_spends = [false; 3];
        self.program.toggle_state.era.guard_contacts = [false; 2];
        self.program.toggle_state.era.player_parry_contacts = 0;
        self.program.toggle_state.scripted_cpu_pad = scripted_p2;
        if let Some(rules) = self.program.match_config.dramatic {
            self.fair_cpu = None;
            self.dramatic_cpu_press(rules, scripted_p2);
            self.step_frame();
            self.dramatic_cpu_observe(rules);
            self.program.toggle_state.scripted_cpu_pad = false;
            self.program.toggle_state.cpu_drive_reserve = [0; 3];
            return;
        }
        self.dramatic_cpu = None;
        let cpu = !scripted_p2 && self.fair_cpu_press();
        self.step_frame();
        if cpu {
            self.fair_cpu_observe();
        }
        self.program.toggle_state.scripted_cpu_pad = false;
        self.program.toggle_state.cpu_drive_reserve = [0; 3];
    }

    /// Whether a round is being fought: the game flow's task is at the
    /// fight loop.
    pub(crate) fn in_fight(&self) -> bool {
        self.scheduler.in_fight_loop()
    }

    pub(crate) fn settling_knockout(&self) -> bool {
        self.scheduler.settling_knockout()
    }

    /// The fair CPU's pad for the coming frame (SP21, `#264`), set as P2's
    /// stick and buttons in place of the pad's. Only under an overhaul
    /// preset whose match configuration gives the CPU Player 2's pad; any
    /// other machine drops the CPU and its inputs stay as they were set.
    fn fair_cpu_press(&mut self) -> bool {
        use crate::fair_cpu::{pad_parts, FairCpu};
        use crate::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
        let cfg = self.program.match_config;
        if !cfg.p2_fair_cpu || cfg.preset == crate::config::RulesetPreset::Faithful1987 {
            self.fair_cpu = None;
            return false;
        }
        // His specials' slot records: a boss's from his bank table, Ryu's and
        // Ken's the 1987 ones; none for a boss the fight holds no table for.
        let mode = self.program.work[0x4002];
        let slot = usize::from(mode != 4);
        let family = self.program.toggle_state.era.fighters[slot].style.family;
        let fighter = if matches!(mode, 2 | 4) {
            if self.program.work[0x0003] == 0 {
                return false;
            }
            if family == crate::era::EraFamily::Sf1987 {
                self.fair_cpu = None;
                return false;
            }
            let table = u32::from_be_bytes(self.program.work[0x4676..0x467a].try_into().unwrap());
            if !matches!(table, 0x1605c | 0x1662c) && !crate::data_bank::in_bank(table) {
                return false;
            }
            crate::roster::TournamentFighter::from_move_table(table).unwrap_or(cfg.roster.p2)
        } else {
            cfg.roster.p2
        };
        let (me, foe) = match mode {
            2 => (CPU_STRUCT, P1_STRUCT),
            4 => (CPU_STRUCT, P2_STRUCT),
            _ => (P2_STRUCT, P1_STRUCT),
        };
        let ctx = self.fair_cpu_context(fighter, slot, me);
        let in_fight = self.in_fight();
        let step = self.state.frame + 1;
        let cpu = self.fair_cpu.get_or_insert_with(FairCpu::new);
        let pad = cpu.press(step, in_fight, &ctx);
        let idle = cpu.utility_idle();
        let input = if family == crate::era::EraFamily::Sf1987 {
            let (directions, buttons) = pad_parts(pad);
            crate::front::PlayerInputs { directions, buttons }
        } else {
            self.step_utility_cpu(slot, me, foe, fighter, family, &ctx, Some((pad, idle)))
        };
        if family != crate::era::EraFamily::Sf1987 {
            self.fair_cpu.as_mut().unwrap().set_utility_pad(self.program.toggle_state.utility[slot].pad());
        }
        self.apply_cpu_pad(input);
        true
    }

    /// P2's logical cabinet pad after any CPU or Training script supplied it.
    pub(crate) fn applied_p2_pad(&self) -> crate::front::PlayerInputs {
        let held = |port: u16, bit: u8| port & (1 << bit) == 0;
        crate::front::PlayerInputs {
            directions: [8, 9, 10, 11].map(|bit| held(self.io.in1, bit)),
            buttons: [
                held(self.io.in1, 12),
                held(self.io.in1, 13),
                held(self.io.in0, 10),
                held(self.io.in1, 14),
                held(self.io.in1, 15),
                held(self.io.in0, 8),
            ],
        }
    }

    fn apply_cpu_pad(&mut self, input: crate::front::PlayerInputs) {
        let [right, left, down, up] = input.directions;
        self.set_p2_direction(right, left, down, up);
        self.set_p2_buttons(input.buttons);
    }

    fn fair_cpu_context(
        &self,
        fighter: crate::roster::TournamentFighter,
        era_side: usize,
        ps: u32,
    ) -> crate::fair_cpu::Context {
        use crate::fair_cpu::{Context, EraControl, Kit, Level};
        let mut cfg = self.program.match_config;
        let slots = match self.program.data_bank.as_deref().and_then(|b| b.table(fighter).map(|t| (b, t))) {
            Some((bank, table)) => Some([0x19, 0x1a, 0x1b].map(|slot| crate::data_bank::special_slot(|a| bank.rb(a), table, slot))),
            None if fighter.is_shoto() => Some([0x19, 0x1a, 0x1b].map(crate::data_bank::SpecialSlot::ryu)),
            None => None,
        };
        let era = &self.dramatic_era_fighters()[era_side];
        if era.unified_gauge {
            cfg.gauge_mode = crate::era::GaugeMode::Unified;
        }
        let level = Level::from_difficulty(cfg.difficulty);
        if crate::era_fight::EraRules::from(cfg).enabled && era.fighter == Some(fighter)
            && !matches!(era.style.family, crate::era::EraFamily::Sf1987 | crate::era::EraFamily::Sf1Plus)
        {
            use crate::fighter::WORK_BASE;
            let game_mode = self.program.work[0x4002];
            let entity_word = (ps + 0x10 - WORK_BASE) as usize;
            let entity = u16::from_be_bytes(self.program.work[entity_word..entity_word + 2].try_into().unwrap()) as usize;
            let control = self.state.objectram.get(entity..entity + 0x40).map(|obj| {
                let slot = crate::combo::slot_in_fight(ps, game_mode, cfg.dramatic.is_some());
                let combo = &self.program.toggle_state.combo;
                let cancel = &self.program.toggle_state.cancel;
                let move_id = obj[0xa];
                let posture = obj[9];
                let airborne = posture & 1 != 0 || matches!(move_id, 9..=0x0b);
                EraControl {
                    free: obj[8] == 2 && !airborne && combo.blockstun[slot] == 0
                        && (posture & 0x80 == 0 || cfg.toggles.interruptible_walk_cycle && matches!(move_id, 2 | 3)),
                    blockstun: combo.blockstun[slot] != 0,
                    airborne,
                    cancel: !airborne && cancel.window[slot] != 0 && cancel.move_id[slot] == move_id,
                    special: posture & 0xc0 == 0xc0
                        && (matches!(move_id, 0x19..=0x1b) || era.sf4_special_active(move_id)),
                    waking: matches!(move_id, 0x28 | 0x35..=0x3f),
                }
            }).unwrap_or_default();
            Context::with_era(level, fighter, slots, cfg, era, control)
        } else {
            Context::new(level, Kit::of(fighter, cfg.preset, slots), &cfg.toggles)
        }
    }

    pub(crate) fn seed_dramatic_cpu(&mut self, seed: u64) {
        use crate::fair_cpu::FairCpu;
        self.seed_utility_cpu(seed);
        if self.program.match_config.dramatic.is_some() {
            self.dramatic_cpu = Some(Box::new([
                FairCpu::seeded(0x9e37_79b9_7f4a_7c15 ^ seed.rotate_left(17)),
                FairCpu::seeded(0xbb67_ae85_84ca_a73b ^ seed.rotate_left(33)),
            ]));
        }
    }

    /// Seed each deterministic utility controller independently.
    pub fn seed_utility_cpu(&mut self, seed: u64) {
        if self.program.match_config.preset != crate::config::RulesetPreset::Faithful1987 {
            for (state, salt) in self.program.toggle_state.utility.iter_mut().zip([
                0x6a09_e667_f3bc_c909, 0x9e37_79b9_7f4a_7c15, 0xbb67_ae85_84ca_a73b,
            ]) {
                *state = crate::utility::UtilityState::seeded(seed ^ salt);
            }
        }
    }

    fn step_utility_cpu(
        &mut self,
        slot: usize,
        me: u32,
        foe: u32,
        fighter: crate::roster::TournamentFighter,
        family: crate::era::EraFamily,
        context: &crate::fair_cpu::Context,
        positional_pad: Option<(u16, bool)>,
    ) -> crate::front::PlayerInputs {
        self.program.toggle_state.cpu_drive_reserve[slot] = crate::fair_cpu::drive_reserve(
            crate::fair_cpu::Level::from_difficulty(self.program.match_config.difficulty),
        );
        let mut state = std::mem::take(&mut self.program.toggle_state.utility[slot]);
        let input = crate::utility::UtilityAi::step(self, &mut state, me, foe, fighter, family, context, positional_pad);
        self.program.toggle_state.utility[slot] = state;
        input
    }

    /// A replay branch's opponent uses the live fighter's command bank.
    /// The playback owns this controller separately from the recorded CPU.
    pub fn utility_takeover_pad(&self, side: usize, state: &mut crate::utility::UtilityState) -> crate::front::PlayerInputs {
        use crate::fight_reader::player_struct;
        use crate::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
        use sf_video::callouts::Side;
        let mode = self.program.work[0x4002];
        let (slot, me, foe) = if self.program.match_config.dramatic.is_some() {
            match side {
                0 => (0, P1_STRUCT, CPU_STRUCT),
                1 => (2, P2_STRUCT, CPU_STRUCT),
                _ => return crate::front::PlayerInputs::default(),
            }
        } else {
            let Some(&player) = Side::BOTH.get(side) else { return crate::front::PlayerInputs::default(); };
            (side, player_struct(player, mode), player_struct(if player == Side::P1 { Side::P2 } else { Side::P1 }, mode))
        };
        let era = &self.dramatic_era_fighters()[slot];
        let fighter = era.fighter.unwrap_or_else(|| {
            let config = self.program.match_config;
            match slot {
                0 => config.roster.p1,
                2 => config.dramatic.map_or(config.roster.p2, |rules| rules.ally2),
                _ => config.roster.p2,
            }
        });
        let context = self.fair_cpu_context(fighter, slot, me);
        crate::utility::UtilityAi::step(self, state, me, foe, fighter, era.style.family, &context, None)
    }

    /// Human-controlled sides can always take over. A one-player cabinet
    /// CPU also needs its player command table to consume the synthetic pad.
    pub fn utility_takeover_side_available(&self, side: usize) -> bool {
        use crate::fight_reader::player_struct;
        use crate::fighter::CPU_STRUCT;
        use sf_video::callouts::Side;
        let Some(&player) = Side::BOTH.get(side) else { return false };
        if self.program.match_config.dramatic.is_some()
            || player_struct(player, self.program.work[0x4002]) != CPU_STRUCT
        {
            return true;
        }
        self.arcade_opponent().is_some_and(|fighter| fighter.is_shoto()
            || self.program.data_bank.as_ref().is_some_and(|bank| bank.table(fighter).is_some()))
    }

    fn dramatic_cpu_press(&mut self, rules: crate::dramatic::DramaticRules, scripted_p2: bool) {
        use crate::fair_cpu::pad_parts;
        use crate::fighter::{CPU_STRUCT, P2_STRUCT};
        let [_, boss_hp, ally_hp] = self.program.toggle_state.dramatic.health;
        let fighter = self.program.match_config.roster.p2;
        let boss = self.fair_cpu_context(fighter, 1, CPU_STRUCT);
        let opponent = self.dramatic_boss_target();
        let families = self.dramatic_era_fighters().map(|era| era.style.family);
        let in_fight = self.in_fight();
        let step = self.state.frame + 1;
        let cpu = &mut self.dramatic_cpu.get_or_insert_with(Default::default)[0];
        let pad = cpu.press(step, in_fight && boss_hp > 0, &boss);
        let idle = cpu.utility_idle();
        self.program.toggle_state.dramatic.boss_pad = if boss_hp > 0 {
            self.step_utility_cpu(1, CPU_STRUCT, opponent, fighter, families[1], &boss, Some((pad, idle)))
        } else {
            crate::front::PlayerInputs::default()
        };
        if boss_hp > 0 {
            self.dramatic_cpu.as_mut().unwrap()[0].set_utility_pad(self.program.toggle_state.utility[1].pad());
        }
        if !rules.co_op && !scripted_p2 {
            let ally = self.fair_cpu_context(rules.ally2, 2, P2_STRUCT);
            let cpu = &mut self.dramatic_cpu.as_mut().unwrap()[1];
            let pad = cpu.press(step, in_fight && ally_hp > 0, &ally);
            let idle = cpu.utility_idle();
            let input = if ally_hp > 0 && boss_hp > 0 {
                self.step_utility_cpu(2, P2_STRUCT, CPU_STRUCT, rules.ally2, families[2], &ally, Some((pad, idle)))
            } else {
                let (directions, buttons) = pad_parts(pad);
                crate::front::PlayerInputs { directions, buttons }
            };
            if ally_hp > 0 && boss_hp > 0 {
                self.dramatic_cpu.as_mut().unwrap()[1].set_utility_pad(self.program.toggle_state.utility[2].pad());
            }
            self.apply_cpu_pad(input);
        }
    }

    fn dramatic_cpu_observe(&mut self, rules: crate::dramatic::DramaticRules) {
        use crate::fair_cpu::perceive_fighters;
        use crate::fighter::{CPU_STRUCT, P2_STRUCT};
        let target = self.dramatic_boss_target();
        let fighting = self.in_fight();
        let [p1_hp, boss_hp, ally_hp] = self.program.toggle_state.dramatic.health;
        let boss = perceive_fighters(self.state.frame, fighting && boss_hp > 0 && (p1_hp > 0 || ally_hp > 0),
            &self.program.work, &self.state.objectram, CPU_STRUCT, target);
        let ally = (!rules.co_op).then(|| perceive_fighters(self.state.frame, fighting && boss_hp > 0 && ally_hp > 0,
            &self.program.work, &self.state.objectram, P2_STRUCT, CPU_STRUCT));
        let cpu = self.dramatic_cpu.as_mut().unwrap();
        cpu[0].observe(boss);
        if let Some(ally) = ally {
            cpu[1].observe(ally);
        }
    }

    fn dramatic_boss_target(&self) -> u32 {
        use crate::fair_cpu::perceive_fighters;
        use crate::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};

        let in_fight = self.in_fight();
        let [p1_hp, _, p2_hp] = self.program.toggle_state.dramatic.health;
        let observe = |me, foe| {
            perceive_fighters(self.state.frame, in_fight, &self.program.work, &self.state.objectram, me, foe)
        };
        let p1 = observe(CPU_STRUCT, P1_STRUCT);
        let p2 = observe(CPU_STRUCT, P2_STRUCT);
        if p1_hp == 0
            || (p2_hp > 0 && p2.me.x.abs_diff(p2.foe.x) < p1.me.x.abs_diff(p1.foe.x))
        {
            P2_STRUCT
        } else {
            P1_STRUCT
        }
    }

    /// The fair CPU's look at the frame just stepped.
    fn fair_cpu_observe(&mut self) {
        let in_fight = self.in_fight();
        use sf_video::callouts::Side;
        let side = if self.program.work[0x4002] == 4 { Side::P1 } else { Side::P2 };
        let p = crate::fair_cpu::perceive(self.state.frame, in_fight, &self.program.work, &self.state.objectram, side);
        if let Some(cpu) = self.fair_cpu.as_mut() {
            cpu.observe(p);
        }
    }

    /// The fair CPU's pad on the last frame, while one plays: (right, left,
    /// down, up), then LP to HK, as Player 2's stick and buttons took them.
    pub fn fair_cpu_pad(&self) -> Option<([bool; 4], [bool; 6])> {
        self.fair_cpu.as_ref().map(|cpu| crate::fair_cpu::pad_parts(cpu.pad()))
    }

    /// One 60 Hz frame of the machine.
    ///
    /// Frame 0's vblank occurs while the reset's 6,144-longword RAM clear
    /// (`0x3b018..0x3b01a`, 184,412 cycles > 133,333 1/3 cycles/frame) is
    /// still running with interrupts masked (`SR = $2700`), so MAME's
    /// `irq1_line_hold` holds vblank 0 until `0x3b04a: move #$2000, SR` in
    /// frame 1.
    fn step_frame(&mut self) {
        self.overlay
            .capture_pre_step(&self.program.work, &self.state.objectram);
        self.state.frame += 1;
        let frame = self.state.frame;
        self.latch_writes.clear();
        self.program.toggle_state.latched_theme = None;
        let io = self.io;
        let effects_preview = self.program.match_config.preset == crate::config::RulesetPreset::DefinitiveOverhaul;
        let in_fight = effects_preview && self.in_fight();
        let scheduler = &mut self.scheduler;
        let mut bus = self.program.bus_with_dsw1(&mut self.state, io.dsw1);
        let before = effects_preview.then(|| bus.begin_move_effects(in_fight));
        let latch = if frame == 1 {
            let l = crate::tasks::reset(scheduler, &mut bus, &io);
            if let Some(lw) = l {
                self.latch_writes = vec![lw];
            }
            l
        } else if io.dsw1 & 0x8000 == 0 && frame < crate::tasks::SERVICE_RAM_TEST_DONE_FRAME {
            self.state.latch_cycles = 0;
            None
        } else if io.dsw1 & 0x8000 == 0 && frame == crate::tasks::SERVICE_RAM_TEST_DONE_FRAME {
            let lw = crate::tasks::ram_test_finish(scheduler, &mut bus, &io);
            self.latch_writes = vec![lw];
            Some(lw)
        } else if let Some(cycles) = self.freeze_vblank_cycles {
            if io.dsw1 & 0x4000 == 0 {
                self.state.latch_cycles = 0;
                self.overlay
                    .record_post_step(&self.program.rom, self.program.data_bank.as_deref(), &self.program.work, &self.state);
                return;
            }
            self.freeze_vblank_cycles = None;
            let resumed = crate::tasks::vblank_suffix(&mut bus, &io, cycles);
            let mut held = crate::tasks::vblank(&mut bus, &io);
            held.cycles += resumed.latch.cycles + resumed.held_entry_cycles;
            self.latch_writes = vec![resumed.latch, held];
            scheduler.run(&mut bus, &io);
            // Mirror palettes (SP16): the end of the frame, after the tasks
            // and the vblank; nothing outside a Local mirror match under
            // the full cast.
            bus.mirror_palette_pass();
            Some(held)
        } else {
            let cycles = crate::tasks::vblank_prefix(&mut bus, &io);
            if io.dsw1 & 0x4000 == 0 {
                self.freeze_vblank_cycles = Some(cycles);
                self.state.latch_cycles = 0;
                self.overlay
                    .record_post_step(&self.program.rom, self.program.data_bank.as_deref(), &self.program.work, &self.state);
                return;
            }
            let suffix = crate::tasks::vblank_suffix(&mut bus, &io, cycles);
            self.latch_writes = vec![suffix.latch];
            scheduler.run(&mut bus, &io);
            // Mirror palettes (SP16), as above.
            bus.mirror_palette_pass();
            Some(suffix.latch)
        };
        if let Some(latch) = latch {
            self.state.sound_latch = latch.value;
            self.state.latch_cycles = latch.cycles;
        }
        if let Some(before) = before {
            let in_fight = self.in_fight();
            let settling_knockout = self.settling_knockout();
            self.program.bus_with_dsw1(&mut self.state, io.dsw1).finish_move_effects(in_fight, settling_knockout, before);
        }
        self.overlay
            .record_post_step(&self.program.rom, self.program.data_bank.as_deref(), &self.program.work, &self.state);
    }

    /// The work RAM (`0xff8000..0xffe000`).
    pub fn work_ram(&self) -> &[u8] {
        &self.program.work
    }

    /// Training's rules' state (the refill waits and the hit record), for
    /// the fight reader.
    pub fn training(&self) -> &crate::training::TrainingState {
        &self.program.training
    }

    /// The program ROM, for the readers that look up its tables.
    pub fn program_rom(&self) -> &[u8] {
        &self.program.rom
    }

    /// The full `0xff8000..0xffffff` image (`work_ram` followed by
    /// `objectram`), matching the range of the reference's `work` dumps.
    pub fn work_image(&self) -> Vec<u8> {
        work_image_bytes(&self.program.work, &self.state.objectram)
    }

    /// Active task slots and their current entry addresses.
    pub fn task_positions(&self) -> Vec<(u32, u32)> {
        self.scheduler.positions()
    }

    /// Untranslated or unverified original 68000 addresses reached so far.
    pub fn unrecovered(&self) -> Vec<u32> {
        self.program.unrecovered.iter().copied().collect()
    }

    /// Target addresses of effect `0xff` stream calls (`0x42dd2`) reached so far.
    pub fn effect_stream_calls(&self) -> Vec<u32> {
        self.program.effect_stream_calls.iter().copied().collect()
    }

    /// Whether the diagnostic overlay is currently enabled (`#92`, `O1`).
    pub fn overlay_enabled(&self) -> bool {
        self.overlay.enabled
    }

    /// Enable or disable the diagnostic overlay without mutating simulation state.
    pub fn set_overlay_enabled(&mut self, enabled: bool) {
        self.overlay.enabled = enabled;
    }

    /// Toggle the diagnostic overlay on or off without mutating simulation state.
    pub fn toggle_overlay(&mut self) -> bool {
        self.overlay.enabled = !self.overlay.enabled;
        self.overlay.enabled
    }

    /// Read-only reference to the 60-cell rolling visual frame meter (`#93`, `O2`).
    pub fn frame_meter(&self) -> &crate::overlay::FrameMeterState {
        &self.overlay.frame_meter
    }

    /// Extract read-only per-frame collision and input diagnostic telemetry (`#92`, `O1`).
    pub fn diagnostic_telemetry(&self) -> crate::overlay::DiagnosticTelemetry {
        let mut telemetry = self.overlay
            .diagnostic_telemetry(&self.program.rom, self.program.data_bank.as_deref(), &self.program.work, &self.state);
        telemetry.show_ally2_inputs = self.program.match_config.dramatic.is_some_and(|rules| rules.co_op);
        telemetry
    }

    /// Render the current scene and composite the diagnostic overlay when enabled,
    /// leaving the base `384x224` game-frame rendering and machine state untouched.
    pub fn render_with_overlay(&self, assets: &Assets) -> sf_video::render::Frame {
        let overhaul =
            self.program.match_config.preset != crate::config::RulesetPreset::Faithful1987;
        self.overlay.render_presented(
            assets,
            &self.program.rom,
            &self.program.work,
            &self.state,
            overhaul,
        )
    }

    /// Render through the faithful hardware path, plus the diagnostic overlay
    /// when it is enabled, whatever the preset: the front end's fights never
    /// draw the era-gauge strip [`Self::render_with_overlay`] adds to
    /// overhaul presets (spec #150).
    pub fn render_faithful_with_overlay(&self, assets: &Assets) -> sf_video::render::Frame {
        self.overlay
            .render_with_overlay(assets, &self.program.rom, &self.program.work, &self.state)
    }

    /// Whether the game flow has just ended a match and waits on its wipe:
    /// in versus before the winner carries on against the CPU (or, with no
    /// winner, before the game over); in a one-player game before the next
    /// opponent or the continue. Read-only: the front end stops Local and
    /// Training fights here.
    pub fn match_over(&self) -> bool {
        self.scheduler.match_over()
    }

    pub(crate) fn start_world_tour_ending(&mut self) {
        let mut bus = self.program.bus(&mut self.state);
        self.scheduler.start_world_tour_ending(&mut bus);
    }

    /// Active match configuration (ruleset preset and Tier 1 core toggles, `#94`, `O3`).
    pub fn match_config(&self) -> &crate::config::MatchConfig {
        &self.program.match_config
    }

    /// The runtime state of the core toggles (read only): the recovery
    /// buffer, the lenient trackers, and the true-combo state (SP17).
    pub fn toggle_state(&self) -> &crate::config::CoreToggleState {
        &self.program.toggle_state
    }

    /// The active Arcade Thailand transformation, if either fighter awakened.
    pub fn satsui_awakening(&self) -> Option<crate::config::SatsuiAwakening> {
        self.program.toggle_state.satsui_awakening
    }

    /// Pass the locked Style cards to the native round setup.
    pub fn set_era_styles(&mut self, picks: [crate::era::StylePick; 2]) {
        self.program.toggle_state.era.selections = picks;
    }

    /// Era state by displayed fight side, including the current Arcade CPU.
    pub fn era_fighters(&self) -> &[crate::era_fight::EraFighterState; 2] {
        self.program.toggle_state.era.fighters.first_chunk().expect("two native fight sides")
    }

    /// Native Dramatic slot order: Ally 1, boss, Ally 2.
    pub fn dramatic_era_fighters(&self) -> &[crate::era_fight::EraFighterState; 3] {
        &self.program.toggle_state.era.fighters
    }

    /// Fill Player 1's Training gauge, or restore the supplied meter amount.
    pub(crate) fn training_era_fighter_mut(&mut self) -> Option<&mut crate::era_fight::EraFighterState> {
        (self.program.match_config.roster.mode == crate::roster::PlayMode::Training)
            .then_some(&mut self.program.toggle_state.era.fighters[0])
    }

    pub fn training_super_meter(&mut self, value: Option<u8>) {
        if self.program.match_config.roster.mode == crate::roster::PlayMode::Training {
            self.program.toggle_state.era.fighters[0].set_super_meter(value);
        }
    }

    pub(crate) fn try_alpha2_feint(&mut self, directions: [bool; 4]) -> bool {
        let direction = directions.into_iter().enumerate()
            .fold(0, |bits, (index, held)| bits | (u8::from(held) << index));
        self.program.bus_with_dsw1(&mut self.state, self.io.dsw1).try_alpha2_feint(direction)
    }

    /// Fight frames still frozen after a Super Combo starts.
    pub fn era_super_freeze(&self) -> u8 {
        self.program.toggle_state.era.freeze
    }

    /// Consume presentation events earned by the native fight's pad inputs.
    pub fn take_era_super_events(&mut self) -> [Option<crate::roster::TournamentFighter>; 2] {
        std::array::from_fn(|i| std::mem::take(&mut self.program.toggle_state.era.super_events[i]))
    }

    pub(crate) fn take_alpha_super_events(&mut self) -> [Option<&'static str>; 2] {
        std::array::from_fn(|i| std::mem::take(&mut self.program.toggle_state.era.alpha_super_events[i]))
    }

    pub(crate) fn take_alpha_counter_events(&mut self) -> [bool; 2] {
        std::array::from_fn(|i| std::mem::take(&mut self.program.toggle_state.era.alpha_counter_events[i]))
    }

    pub(crate) fn take_era_style_callouts(&mut self) -> [Option<sf_video::callouts::CalloutKind>; 2] {
        std::array::from_fn(|i| std::mem::take(&mut self.program.toggle_state.era.style_callouts[i]))
    }

    pub(crate) fn take_gauge_drive_spends(&mut self) -> [bool; 3] {
        std::mem::take(&mut self.program.toggle_state.era.gauge_drive_spends)
    }

    pub(crate) fn take_gauge_v_trigger_spends(&mut self) -> [bool; 3] {
        std::mem::take(&mut self.program.toggle_state.era.gauge_v_trigger_spends)
    }

    pub(crate) fn take_guard_crush_events(&mut self) -> [bool; 2] {
        std::array::from_fn(|i| std::mem::take(&mut self.program.toggle_state.era.guard_crush_events[i]))
    }

    pub(crate) fn take_dramatic_ally_era_events(&mut self) -> [Option<sf_video::callouts::CalloutKind>; 5] {
        use sf_video::callouts::CalloutKind;

        let era = &mut self.program.toggle_state.era;
        [
            era.style_callouts[2].take(),
            era.super_events[2].take().map(|fighter| CalloutKind::Super {
                name: crate::era::super_label(fighter),
            }),
            era.alpha_super_events[2].take().map(|name| CalloutKind::Super { name }),
            std::mem::take(&mut era.alpha_counter_events[2]).then_some(CalloutKind::AlphaCounter),
            std::mem::take(&mut era.guard_crush_events[2]).then_some(CalloutKind::Move { word: "GUARD CRUSH" }),
        ]
    }

    pub(crate) fn select_theme(&mut self, theme: Option<crate::front::fight::MatchTheme>) {
        self.program.toggle_state.selected_theme = theme;
    }

    /// Take the stage tune command a stand-in fight fell back to when Ryu's
    /// extended theme was missing (SP15, `#267`).
    pub fn take_missing_ryu_theme(&mut self) -> Option<u16> {
        self.program.toggle_state.missing_ryu_theme.take()
    }

    /// The zero-based Arcade ladder step from the stage and fight counters.
    pub(crate) fn arcade_ladder_step(&self) -> usize {
        let work = &self.program.work;
        usize::from(work[0x4003]) * 2 + usize::from(work[0x4004])
    }

    /// The opponent the Arcade ladder brings for the current stage and
    /// fight (`0xffc003`, `0xffc004` through `0xffc01c`): under the full
    /// cast outside demo play, Ryu stands in when the ladder slot is the
    /// human player's own fighter (SP14 `#266`, SP15 `#267`).
    pub fn arcade_opponent(&self) -> Option<crate::roster::TournamentFighter> {
        let base = u32::from_be_bytes(self.program.work[0x401c..0x4020].try_into().unwrap());
        if base == 0 {
            return None;
        }
        let ptr_addr = (base as usize).wrapping_add(self.arcade_ladder_step() * 4);
        let table = u32::from_be_bytes(self.program.rom.get(ptr_addr..ptr_addr + 4)?.try_into().ok()?);
        let slot = crate::roster::TournamentFighter::from_move_table(table)?;
        let roster = self.program.match_config.roster;
        if roster.full_cast && self.program.work[0x0003] != 0 && slot == roster.p1 {
            Some(crate::roster::TournamentFighter::Ryu)
        } else {
            Some(slot)
        }
    }

    /// Mutable access to the active match configuration (`#94`, `O3`).
    pub fn match_config_mut(&mut self) -> &mut crate::config::MatchConfig {
        &mut self.program.match_config
    }

    /// Replace the active [`crate::config::MatchConfig`] (`#94`, `O3`).
    pub fn set_match_config(&mut self, config: crate::config::MatchConfig) {
        let previous_era = crate::era_fight::EraRules::from(self.program.match_config);
        let next_era = crate::era_fight::EraRules::from(config);
        let previous_schemes = self.program.match_config.effective_control_schemes();
        for (side, scheme) in config.effective_control_schemes().into_iter().enumerate() {
            if previous_schemes[side] != scheme {
                self.program.toggle_state.schemes[side] = crate::control::SchemeState::default();
                if side == 1 {
                    self.program.toggle_state.schemes[2] = crate::control::SchemeState::default();
                }
            }
        }
        self.program.match_config = config;
        if config.roster.mode != crate::roster::PlayMode::Arcade
            || !config.roster.full_cast
            || !next_era.enabled
            || config.evil_ryu_playtest.is_some()
            || config.dramatic.is_some()
            || config.world_tour.is_some()
        {
            self.program.toggle_state.satsui_awakening = None;
        }
        if !config.toggles.positive_edge_attacks {
            self.program.toggle_state.neg_edge_latch.fill(false);
        }
        if !config.toggles.recovery_input_buffer {
            self.program.toggle_state.recovery_buffer.fill(None);
        }
        if previous_era.enabled != next_era.enabled || previous_era.gauge_mode != next_era.gauge_mode {
            self.program.bus(&mut self.state).era_reconfigure(previous_era.enabled);
        }
    }

    /// Builder: set the active [`crate::config::MatchConfig`] (`#94`, `O3`).
    pub fn with_match_config(mut self, config: crate::config::MatchConfig) -> Self {
        self.set_match_config(config);
        self
    }

    /// Builder: the extended background bank (ADR-0009), which the stage
    /// gates and the palette load read. Only an overhaul fight's machine is
    /// given one.
    pub fn with_background_bank(mut self, bank: std::sync::Arc<sf_video::background_bank::BackgroundBank>) -> Self {
        self.program.backgrounds = Some(bank);
        self
    }

    /// Builder: the extended data bank (ADR-0014), the bosses' 1987 player
    /// tables the player handler reads. Only an overhaul fight's machine is
    /// given one.
    pub fn with_data_bank(mut self, bank: std::sync::Arc<crate::data_bank::DataBank>) -> Self {
        self.program.data_bank = Some(bank);
        self
    }

    /// The extended data bank this machine holds, if any.
    pub fn data_bank(&self) -> Option<&crate::data_bank::DataBank> {
        self.program.data_bank.as_deref()
    }

    /// The extended background bank this machine holds, if any.
    pub fn background_bank(&self) -> Option<&sf_video::background_bank::BackgroundBank> {
        self.program.backgrounds.as_deref()
    }

    /// Switch to the given [`crate::config::RulesetPreset`] (`#94`, `O3`).
    pub fn set_preset(&mut self, preset: crate::config::RulesetPreset) {
        let mut cfg = self.program.match_config;
        cfg.apply_preset(preset);
        self.set_match_config(cfg);
    }

    /// Builder: switch to the given [`crate::config::RulesetPreset`] (`#94`, `O3`).
    pub fn with_preset(mut self, preset: crate::config::RulesetPreset) -> Self {
        self.set_preset(preset);
        self
    }

    /// Cycle to the next standard [`crate::config::RulesetPreset`] (`#94`, `O3`).
    pub fn cycle_preset(&mut self) -> crate::config::RulesetPreset {
        let next = self.program.match_config.cycle_preset();
        let cfg = self.program.match_config;
        self.set_match_config(cfg);
        next
    }

    /// Save a deterministic full-session state snapshot at the current frame boundary (`#93`, `O2`).
    pub fn save_snapshot(&self) -> PowerOnSnapshot {
        PowerOnSnapshot {
            state: self.state.clone(),
            work: self.program.work.clone(),
            unrecovered: self.program.unrecovered.clone(),
            effect_stream_calls: self.program.effect_stream_calls.clone(),
            match_config: self.program.match_config,
            toggle_state: self.program.toggle_state,
            training: self.program.training,
            scheduler: self.scheduler,
            io: self.io,
            freeze_vblank_cycles: self.freeze_vblank_cycles,
            latch_writes: self.latch_writes.clone(),
            overlay: self.overlay.clone(),
            fair_cpu: self.fair_cpu,
            dramatic_cpu: self.dramatic_cpu.clone(),
        }
    }

    /// Restore a deterministic full-session state snapshot saved by [`Self::save_snapshot`] (`#93`, `O2`).
    pub fn restore_snapshot(&mut self, snapshot: &PowerOnSnapshot) {
        self.state = snapshot.state.clone();
        self.program.work.clone_from(&snapshot.work);
        self.program.unrecovered.clone_from(&snapshot.unrecovered);
        self.program
            .effect_stream_calls
            .clone_from(&snapshot.effect_stream_calls);
        self.program.match_config = snapshot.match_config;
        self.program.toggle_state = snapshot.toggle_state;
        self.program.training = snapshot.training;
        self.scheduler = snapshot.scheduler;
        self.io = snapshot.io;
        self.freeze_vblank_cycles = snapshot.freeze_vblank_cycles;
        self.latch_writes.clone_from(&snapshot.latch_writes);
        self.overlay = snapshot.overlay.clone();
        self.fair_cpu = snapshot.fair_cpu;
        self.dramatic_cpu.clone_from(&snapshot.dramatic_cpu);
    }
}

impl SceneMachine for PowerOnMachine {
    fn state(&self) -> &MachineState {
        &self.state
    }
    fn dsw(&self) -> (u16, u16) {
        (self.io.dsw1, self.io.dsw2)
    }
    fn step(&mut self) {
        PowerOnMachine::step(self);
    }
}

/// Run the power-on machine from frame 0 through `last` and write the native
/// capture (ticket #41, F1).
pub fn run_power_on_capture(
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    last: u32,
) -> io::Result<u64> {
    let mut machine = PowerOnMachine::new(assets.maincpu.clone());
    let frames = drive(
        &mut machine,
        assets,
        out_dir,
        scenario_id,
        0,
        last,
        |_, _| {},
    )?;
    let unrecovered = machine.unrecovered();
    if !unrecovered.is_empty() {
        return Err(io::Error::other(format!(
            "run reached untranslated original branches: {unrecovered:05x?}"
        )));
    }
    Ok(frames)
}

/// Run the power-on machine with DIP switches and inputs loaded from
/// `scenario_path` from frame 0 through `last` and write the native capture
/// (tickets #42, #89; F2, F49).
pub fn run_power_on_scenario_capture(
    assets: &Assets,
    out_dir: &Path,
    scenario_id: &str,
    scenario_path: &Path,
    last: u32,
) -> io::Result<u64> {
    let mut machine = PowerOnMachine::from_scenario(assets.maincpu.clone(), scenario_path)?;
    let mut inputs = P1Inputs::load(Some(scenario_path))?;
    let frames = drive(
        &mut machine,
        assets,
        out_dir,
        scenario_id,
        0,
        last,
        |m, frame| inputs.apply_power_on(m, frame),
    )?;
    let unrecovered = machine.unrecovered();
    if !unrecovered.is_empty() {
        return Err(io::Error::other(format!(
            "run reached untranslated original branches: {unrecovered:05x?}"
        )));
    }
    Ok(frames)
}

/// Like [`run_power_on_scenario_capture`], writing checkpoint artifacts only on
/// frames where `reference_capture` has a checkpoint dump (`ram_NNNNNN_work.bin`).
pub fn run_power_on_scenario_capture_for_reference(
    assets: &Assets,
    reference_capture: &Path,
    out_dir: &Path,
    scenario_id: &str,
    scenario_path: &Path,
    last: u32,
) -> io::Result<u64> {
    let mut machine = PowerOnMachine::from_scenario(assets.maincpu.clone(), scenario_path)?;
    let mut inputs = P1Inputs::load(Some(scenario_path))?;
    let frames = drive_filtered(
        &mut machine,
        assets,
        out_dir,
        scenario_id,
        0,
        last,
        |frame| {
            reference_capture
                .join(format!("ram_{frame:06}_work.bin"))
                .exists()
        },
        |m, frame| inputs.apply_power_on(m, frame),
    )?;
    let unrecovered = machine.unrecovered();
    if !unrecovered.is_empty() {
        return Err(io::Error::other(format!(
            "run reached untranslated original branches: {unrecovered:05x?}"
        )));
    }
    Ok(frames)
}

/// Run the power-on machine with DIP switches and inputs loaded from
/// `scenario_path` from frame 0 through `last`, calling `observe` at frame 0
/// and after every frame produced through `last` (ticket #89, F49). Stops at
/// the first untranslated original branch before observing that frame,
/// returning the frame where it stopped and the branches reached.
pub fn observe_power_on_scenario(
    program_rom: Vec<u8>,
    scenario_path: &Path,
    last: u32,
    observe: impl FnMut(&PowerOnMachine),
) -> io::Result<(u32, Vec<u32>)> {
    observe_power_on_scenario_with(PowerOnMachine::from_scenario(program_rom, scenario_path)?, scenario_path, last, observe)
}

/// [`observe_power_on_scenario`] on `m`, a machine at power-on with the
/// scenario's DIP words (one given the extended background bank, say).
pub fn observe_power_on_scenario_with(
    mut m: PowerOnMachine,
    scenario_path: &Path,
    last: u32,
    mut observe: impl FnMut(&PowerOnMachine),
) -> io::Result<(u32, Vec<u32>)> {
    let mut inputs = P1Inputs::load(Some(scenario_path))?;
    observe(&m);
    while m.state.frame < last {
        let frame = m.state.frame + 1;
        inputs.apply_power_on(&mut m, frame);
        m.step();
        let unrecovered = m.unrecovered();
        if !unrecovered.is_empty() {
            return Ok((m.state.frame, unrecovered));
        }
        observe(&m);
    }
    Ok((m.state.frame, Vec::new()))
}

/// Like [`observe_power_on_scenario`], continuing through `last` when a child
/// task slot hits an untranslated entry so a multi-item scenario can record the
/// exact frame and address of each untranslated entry reached (ticket #146,
/// F74).
pub fn observe_power_on_scenario_through(
    program_rom: Vec<u8>,
    scenario_path: &Path,
    last: u32,
    mut observe: impl FnMut(&PowerOnMachine),
) -> io::Result<(u32, Vec<u32>)> {
    let mut m = PowerOnMachine::from_scenario(program_rom, scenario_path)?;
    let mut inputs = P1Inputs::load(Some(scenario_path))?;
    observe(&m);
    while m.state.frame < last {
        let frame = m.state.frame + 1;
        inputs.apply_power_on(&mut m, frame);
        m.step();
        observe(&m);
    }
    Ok((m.state.frame, m.unrecovered()))
}

/// Run the power-on machine from frame 0 through `last`, calling `observe`
/// at frame 0 and after every frame produced through `last`. Stops at the
/// first untranslated original branch before observing that frame, returning
/// the frame where it stopped and the branches reached.
pub fn observe_power_on(
    program_rom: Vec<u8>,
    last: u32,
    observe: impl FnMut(&PowerOnMachine),
) -> (u32, Vec<u32>) {
    observe_power_on_with_io(program_rom, crate::tasks::Io::idle(), last, observe)
}

/// [`observe_power_on`] with the given cabinet I/O and DIP switch words.
pub fn observe_power_on_with_io(
    program_rom: Vec<u8>,
    io: crate::tasks::Io,
    last: u32,
    mut observe: impl FnMut(&PowerOnMachine),
) -> (u32, Vec<u32>) {
    let mut m = PowerOnMachine::with_io(program_rom, io);
    observe(&m);
    while m.state.frame < last {
        m.step();
        let unrecovered = m.unrecovered();
        if !unrecovered.is_empty() {
            return (m.state.frame, unrecovered);
        }
        observe(&m);
    }
    (m.state.frame, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_second_pad_uses_the_arcade_opponents_facing() {
        use crate::control::Shortcut;
        use crate::fighter::{CPU_STRUCT, GAME_MODE, P2_STRUCT, WORK_BASE};
        use crate::front::PlayerInputs;
        use crate::session_io::ControlScheme;

        let mut machine = PowerOnMachine::new(Vec::new());
        let mut config = crate::config::MatchConfig::definitive();
        config.p2_scheme = ControlScheme::Modern;
        machine.set_match_config(config);
        machine.program.work[(GAME_MODE - WORK_BASE) as usize] = 2;

        for (player, entity, facing) in [(CPU_STRUCT, 0x100u16, 4), (P2_STRUCT, 0x200, 0)] {
            let at = (player + 0x10 - WORK_BASE) as usize;
            machine.program.work[at..at + 2].copy_from_slice(&entity.to_be_bytes());
            machine.state.objectram[usize::from(entity) + 0x37] = facing;
        }
        let mut input = PlayerInputs::default();
        input.directions[1] = true;
        input.buttons[3] = true;
        machine.translate_player_controls(1, input, false);
        assert_eq!(machine.program.toggle_state.schemes[1].shortcut, Some(Shortcut::AntiAir));
    }

    /// Regression: the scenario mapping must not swap the directions —
    /// `p1_right` schedules the machine's right input, `p1_left` the left,
    /// `p1_down` the down input, `p1_up` the up input. (A right/left swap
    /// here shifted the native walk 24px from the reference: the right
    /// frame's +8 became the left frame's -16.)
    #[test]
    fn direction_events_map_press_names_to_their_own_input() {
        let scenario = r#"{
            "inputs": [
                {"press": "p1_right", "frame": 2460, "hold_frames": 70},
                {"press": "p1_left", "frame": 2600, "hold_frames": 450},
                {"press": "p1_down", "frame": 2700, "hold_frames": 120},
                {"press": "p1_up", "frame": 2900, "hold_frames": 12},
                {"press": "coin1", "frame": 1450, "hold_frames": 2}
            ]
        }"#;
        let dir = ScratchDir::new("sf-engine-test-scenario");
        std::fs::create_dir_all(&*dir).unwrap();
        let path = dir.join("scenario.json");
        std::fs::write(&path, scenario).unwrap();
        let events = versus_direction_events(&path).unwrap();
        assert_eq!(events.len(), 10, "press+release per direction and coin1");
        assert_eq!(
            events[0],
            DirectionEvent {
                frame: 1450,
                control: Direction::Coin1,
                p2: false,
                held: true
            },
            "coin1 presses coin1"
        );
        assert_eq!(
            events[1],
            DirectionEvent {
                frame: 1452,
                control: Direction::Coin1,
                p2: false,
                held: false
            },
            "coin1 releases coin1 at frame+hold"
        );
        assert_eq!(
            events[2],
            DirectionEvent {
                frame: 2460,
                control: Direction::Right,
                p2: false,
                held: true
            },
            "p1_right presses right"
        );
        assert_eq!(
            events[3],
            DirectionEvent {
                frame: 2530,
                control: Direction::Right,
                p2: false,
                held: false
            },
            "p1_right releases right at frame+hold"
        );
        assert_eq!(
            events[4],
            DirectionEvent {
                frame: 2600,
                control: Direction::Left,
                p2: false,
                held: true
            },
            "p1_left presses left"
        );
        assert_eq!(
            events[5],
            DirectionEvent {
                frame: 2700,
                control: Direction::Down,
                p2: false,
                held: true
            },
            "p1_down presses down"
        );
        assert_eq!(
            events[6],
            DirectionEvent {
                frame: 2820,
                control: Direction::Down,
                p2: false,
                held: false
            },
            "p1_down releases at frame+hold"
        );
        assert_eq!(
            events[7],
            DirectionEvent {
                frame: 2900,
                control: Direction::Up,
                p2: false,
                held: true
            },
            "p1_up presses up"
        );
        assert_eq!(
            events[8],
            DirectionEvent {
                frame: 2912,
                control: Direction::Up,
                p2: false,
                held: false
            },
            "p1_up releases at frame+hold"
        );
        assert_eq!(
            events[9],
            DirectionEvent {
                frame: 3050,
                control: Direction::Left,
                p2: false,
                held: false
            },
            "p1_left releases left at frame+hold"
        );
    }

    #[test]
    fn scenario_dsw_loads_dip_words_and_queue_sound_filters_read_live_dsw1() {
        let dir = ScratchDir::new("sf-engine-test-dsw");
        std::fs::create_dir_all(&*dir).unwrap();
        let path = dir.join("scenario.json");
        std::fs::write(
            &path,
            r#"{
                "machine": {
                    "set": "sf",
                    "dipswitches": {
                        "Coin A": "2C_1C",
                        "Coin B": "1C_6C",
                        "Number of Countries Selected": "4"
                    }
                }
            }"#,
        )
        .unwrap();
        assert_eq!(scenario_dsw(&path).unwrap(), (0xdfda, 0xfbff));

        let mut program = Program::power_on(vec![0; 0x60000]);
        let mut state = MachineState::new(
            vec![0; 0x1000],
            vec![0; 0x2000],
            vec![0; 0x800],
            0,
            0,
            0,
            false,
            0xff,
            0,
        );
        // Attract Music Off (bit 9 = 0 in DSW1): drops 0x28..=0x3e, keeps 0x27 and 0x3f.
        {
            let mut bus = program.bus_with_dsw1(&mut state, 0xdfff & !0x0200);
            bus.wb(0xff8003, 1);
            bus.queue_sound(0x28);
            assert_eq!(
                bus.rw(0xff8660),
                0,
                "0x28 dropped when Attract Music is Off"
            );
            bus.queue_sound(0x27);
            assert_eq!(bus.rw(0xff8660), 2, "0x27 queued when Attract Music is Off");
            bus.queue_sound(0x3f);
            assert_eq!(bus.rw(0xff8660), 4, "0x3f queued when Attract Music is Off");
        }
        // Demo Sounds Off (bit 13 = 1 in DSW1): drops commands in demo mode (0xff8003 == 0).
        {
            let mut bus = program.bus_with_dsw1(&mut state, 0xdfff | 0x2000);
            bus.wb(0xff8003, 0);
            bus.queue_sound(0x01);
            assert_eq!(
                bus.rw(0xff8660),
                4,
                "0x01 dropped in demo mode when Demo Sounds is Off"
            );
            bus.wb(0xff8003, 1);
            bus.queue_sound(0x01);
            assert_eq!(bus.rw(0xff8660), 6, "0x01 queued outside demo mode");
        }
    }
}
