//! Deterministic replay and live takeover on the legacy and native engines.
//!
//! Wraps a [`FightMachine`] and a recorded scenario input schedule so a player
//! can pause at any frame, take live control of either `P1` or `P2` from that
//! exact frame with branched inputs for a configurable window (default 10
//! seconds = 600 frames at 60 Hz) while the opponent continues replaying their
//! recorded inputs, and rewind back to the takeover bookmark cleanly with
//! bit-identical state restoration.

use std::io;
use std::path::Path;

use crate::fight::{FightMachine, FightSnapshot};
use crate::run::{apply_inputs, scenario_input_events, INPUTS};

mod native;
pub(crate) use native::MAX_REPLAY_COMMANDS;
pub use native::{MatchReplay, ReplayCommand, ReplayPlayback, MAX_REPLAY_BYTES, MAX_REPLAY_FRAMES, REPLAY_LIMIT};

/// Default Replay Takeover window duration: 10 seconds at 60 Hz (`600` frames).
pub const DEFAULT_TAKEOVER_WINDOW_FRAMES: u32 = 600;

/// Which fighter the live player controls during Replay Takeover (`#93`, `O2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeoverPlayer {
    P1,
    P2,
}

/// Live per-frame logical inputs for one player during Replay Takeover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerInputs {
    /// Logical directions: `[right, left, down, up]`.
    pub directions: [bool; 4],
    /// Logical attack buttons: `[lp, mp, hp, lk, mk, hk]`.
    pub buttons: [bool; 6],
}

/// Current operating mode of a [`ReplayTakeoverSession`] (`#93`, `O2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMode {
    /// Replaying recorded inputs for both P1 and P2.
    Playback,
    /// Paused at the current frame boundary.
    Paused,
    /// Live takeover of `player` from `start_frame` for up to `window_frames`
    /// while the other fighter continues replaying recorded inputs.
    Takeover {
        player: TakeoverPlayer,
        start_frame: u32,
        window_frames: u32,
        elapsed_frames: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TakeoverBookmark {
    snapshot: FightSnapshot,
    next_event: usize,
    held: [bool; INPUTS],
    dsw: Option<(u16, u16)>,
    next_dip: usize,
}

/// Deterministic Replay Takeover session over a [`FightMachine`] and recorded
/// input schedule (`#93`, `O2`).
#[derive(Clone)]
pub struct ReplayTakeoverSession {
    machine: FightMachine,
    events: Vec<(u32, usize, bool)>,
    next_event: usize,
    held: [bool; INPUTS],
    dsw: Option<(u16, u16)>,
    dip_changes: Vec<(u32, u16, u16)>,
    next_dip: usize,
    mode: ReplayMode,
    takeover_window_frames: u32,
    bookmark: Option<TakeoverBookmark>,
    in_branched_timeline: bool,
}

impl ReplayTakeoverSession {
    /// Construct a replay takeover session from a [`FightMachine`] and raw
    /// `(frame, input_index, held)` schedule events.
    pub fn new(machine: FightMachine, events: Vec<(u32, usize, bool)>) -> Self {
        let mut session = Self {
            machine,
            events,
            next_event: 0,
            held: [false; INPUTS],
            dsw: None,
            dip_changes: Vec::new(),
            next_dip: 0,
            mode: ReplayMode::Playback,
            takeover_window_frames: DEFAULT_TAKEOVER_WINDOW_FRAMES,
            bookmark: None,
            in_branched_timeline: false,
        };
        session.sync_schedule_to_current_frame();
        session
    }

    /// Construct a replay takeover session from a [`FightMachine`] and a
    /// scenario JSON path.
    pub fn from_scenario(mut machine: FightMachine, scenario_path: &Path) -> io::Result<Self> {
        let scenario = sf_compare::scenario::load_scenario(scenario_path)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let dip_changes: Vec<(u32, u16, u16)> = scenario
            .dip_changes
            .iter()
            .map(|c| (c.frame as u32, c.dsw1, c.dsw2))
            .collect();
        let dsw = Some((scenario.dsw1, scenario.dsw2));
        machine.set_dsw(scenario.dsw1, scenario.dsw2);
        let events = scenario_input_events(scenario_path)?;
        let mut session = Self {
            machine,
            events,
            next_event: 0,
            held: [false; INPUTS],
            dsw,
            dip_changes,
            next_dip: 0,
            mode: ReplayMode::Playback,
            takeover_window_frames: DEFAULT_TAKEOVER_WINDOW_FRAMES,
            bookmark: None,
            in_branched_timeline: false,
        };
        session.sync_schedule_to_current_frame();
        Ok(session)
    }

    fn sync_schedule_to_current_frame(&mut self) {
        let cur = self.machine.state.frame;
        self.advance_recorded_schedule_for_frame(cur);
    }

    /// Read-only reference to the underlying [`FightMachine`].
    pub fn machine(&self) -> &FightMachine {
        &self.machine
    }

    /// Mutable reference to the underlying [`FightMachine`].
    pub fn machine_mut(&mut self) -> &mut FightMachine {
        &mut self.machine
    }

    /// Current replay mode (`Playback`, `Paused`, or `Takeover`).
    pub fn mode(&self) -> ReplayMode {
        self.mode
    }

    /// Current simulation frame number.
    pub fn current_frame(&self) -> u32 {
        self.machine.state.frame
    }

    /// Configured takeover window length in frames (default `600` = 10s at 60 Hz).
    pub fn takeover_window_frames(&self) -> u32 {
        self.takeover_window_frames
    }

    /// Configure the takeover window length in frames (`#93`, `O2`).
    pub fn set_takeover_window_frames(&mut self, frames: u32) {
        self.takeover_window_frames = frames.max(1);
        if let ReplayMode::Takeover {
            player,
            start_frame,
            elapsed_frames,
            ..
        } = self.mode
        {
            self.mode = ReplayMode::Takeover {
                player,
                start_frame,
                window_frames: self.takeover_window_frames,
                elapsed_frames,
            };
        }
    }

    /// True if a takeover bookmark snapshot is currently saved.
    pub fn has_bookmark(&self) -> bool {
        self.bookmark.is_some()
    }

    /// True when in `Takeover` mode and `elapsed_frames >= window_frames`.
    pub fn is_takeover_window_expired(&self) -> bool {
        matches!(
            self.mode,
            ReplayMode::Takeover {
                window_frames,
                elapsed_frames,
                ..
            } if elapsed_frames >= window_frames
        )
    }

    /// Pause replay or takeover at the current frame boundary.
    pub fn pause(&mut self) {
        self.mode = ReplayMode::Paused;
    }

    /// Resume recorded replay playback from the current frame.
    pub fn resume_replay(&mut self) {
        self.mode = ReplayMode::Playback;
    }

    /// Toggle between `Paused` and `Playback` (if in `Takeover`, pauses at the current frame).
    pub fn toggle_pause(&mut self) -> ReplayMode {
        self.mode = match self.mode {
            ReplayMode::Paused => ReplayMode::Playback,
            ReplayMode::Playback | ReplayMode::Takeover { .. } => ReplayMode::Paused,
        };
        self.mode
    }

    fn save_bookmark_here(&mut self) {
        self.bookmark = Some(TakeoverBookmark {
            snapshot: self.machine.save_snapshot(),
            next_event: self.next_event,
            held: self.held,
            dsw: self.dsw,
            next_dip: self.next_dip,
        });
        self.in_branched_timeline = false;
    }

    /// Bookmark the current frame (or rewind to the active bookmark if already in
    /// `Takeover` mode or paused on a branched takeover timeline) and take live
    /// control of `player` (`P1` or `P2`) for `self.takeover_window_frames` frames
    /// while the opponent continues replaying their recorded inputs (`#93`, `O2`).
    pub fn take_over(&mut self, player: TakeoverPlayer) {
        if (matches!(self.mode, ReplayMode::Takeover { .. }) || self.in_branched_timeline)
            && self.bookmark.is_some()
        {
            self.restore_bookmark_internal();
        } else {
            self.save_bookmark_here();
        }
        self.mode = ReplayMode::Takeover {
            player,
            start_frame: self.machine.state.frame,
            window_frames: self.takeover_window_frames,
            elapsed_frames: 0,
        };
    }

    /// Always save a fresh takeover bookmark at the current frame and begin
    /// live takeover of `player`.
    pub fn take_over_here(&mut self, player: TakeoverPlayer) {
        self.save_bookmark_here();
        self.mode = ReplayMode::Takeover {
            player,
            start_frame: self.machine.state.frame,
            window_frames: self.takeover_window_frames,
            elapsed_frames: 0,
        };
    }

    fn restore_bookmark_internal(&mut self) -> bool {
        let Some(bm) = &self.bookmark else {
            return false;
        };
        self.machine.restore_snapshot(&bm.snapshot);
        self.next_event = bm.next_event;
        self.held = bm.held;
        self.dsw = bm.dsw;
        self.next_dip = bm.next_dip;
        self.in_branched_timeline = false;
        true
    }

    /// Rewind simulation and input-schedule state back to the saved takeover
    /// bookmark (`#93`, `O2`). If in `Takeover` mode, resets `elapsed_frames` to `0`
    /// at `start_frame`. Returns `true` if a bookmark existed and was restored.
    pub fn rewind_to_takeover(&mut self) -> bool {
        if !self.restore_bookmark_internal() {
            return false;
        }
        if let ReplayMode::Takeover {
            player,
            window_frames,
            ..
        } = self.mode
        {
            self.mode = ReplayMode::Takeover {
                player,
                start_frame: self.machine.state.frame,
                window_frames,
                elapsed_frames: 0,
            };
        }
        true
    }

    fn advance_recorded_schedule_for_frame(&mut self, frame: u32) {
        while self.next_dip < self.dip_changes.len() && self.dip_changes[self.next_dip].0 < frame {
            let (_, dsw1, dsw2) = self.dip_changes[self.next_dip];
            self.dsw = Some((dsw1, dsw2));
            self.next_dip += 1;
        }
        if let Some((dsw1, dsw2)) = self.dsw {
            self.machine.set_dsw(dsw1, dsw2);
        }
        while self.next_event < self.events.len() && self.events[self.next_event].0 < frame {
            let (_, idx, held) = self.events[self.next_event];
            self.held[idx] = held;
            self.next_event += 1;
        }
    }

    /// Advance one frame in `Playback` mode using recorded inputs for both
    /// fighters (returns `false` when `Paused`).
    pub fn step_replay(&mut self) -> bool {
        match self.mode {
            ReplayMode::Paused => false,
            ReplayMode::Playback => {
                let next_frame = self.machine.state.frame + 1;
                self.advance_recorded_schedule_for_frame(next_frame);
                let held = self.held;
                apply_inputs(&mut self.machine, &held);
                self.machine.step();
                true
            }
            ReplayMode::Takeover { .. } => self.step_with_live_input(PlayerInputs::default()),
        }
    }

    /// Advance one frame, injecting `live` inputs for the taken-over fighter
    /// while the opponent continues replaying their recorded scenario inputs (`#93`, `O2`).
    ///
    /// When the takeover window (`window_frames`) has already elapsed, this
    /// automatically rewinds to the takeover bookmark before stepping with `live`.
    pub fn step_with_live_input(&mut self, live: PlayerInputs) -> bool {
        match self.mode {
            ReplayMode::Paused => false,
            ReplayMode::Playback => self.step_replay(),
            ReplayMode::Takeover {
                player,
                mut start_frame,
                window_frames,
                mut elapsed_frames,
            } => {
                if elapsed_frames >= window_frames {
                    self.restore_bookmark_internal();
                    start_frame = self.machine.state.frame;
                    elapsed_frames = 0;
                }
                let next_frame = self.machine.state.frame + 1;
                self.advance_recorded_schedule_for_frame(next_frame);
                let mut effective = self.held;
                let base = match player {
                    TakeoverPlayer::P1 => 0,
                    TakeoverPlayer::P2 => 10,
                };
                for (i, &d) in live.directions.iter().enumerate() {
                    effective[base + i] = d;
                }
                for (i, &b) in live.buttons.iter().enumerate() {
                    effective[base + 4 + i] = b;
                }
                apply_inputs(&mut self.machine, &effective);
                self.machine.step();
                self.in_branched_timeline = true;
                self.mode = ReplayMode::Takeover {
                    player,
                    start_frame,
                    window_frames,
                    elapsed_frames: elapsed_frames + 1,
                };
                true
            }
        }
    }
}
