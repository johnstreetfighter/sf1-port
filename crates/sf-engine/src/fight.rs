//! The shared versus and one-player fight machine (ticket #66). A captured
//! fight-loop checkpoint supplies the video, palette, object RAM, task stacks
//! and translated program state; the same implementation accepts both
//! players' controls in every mode.
//!
//! Each frame runs the Freeze-aware vblank suffix, then the ready tasks in
//! slot order. The fight loop dispatches both fighters, CPU decisions when
//! present, effects, camera and TIME updates on the translated routines in
//! [`crate::fighter`]. No separate versus or CPU state machine remains.

use crate::fighter::{Program, CPU_STRUCT};
use crate::machine::MachineState;
use crate::tasks::{self, Io, Scheduler};

/// Deterministic full-session state snapshot of a [`FightMachine`] (`#93`, `O2`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FightSnapshot {
    pub state: MachineState,
    pub work: Vec<u8>,
    pub unrecovered: std::collections::BTreeSet<u32>,
    pub effect_stream_calls: std::collections::BTreeSet<u32>,
    pub match_config: crate::config::MatchConfig,
    pub toggle_state: crate::config::CoreToggleState,
    /// Training's rules' state (refill waits, the hit record), restored with
    /// the rest.
    pub training: crate::training::TrainingState,
    pub scheduler: Scheduler,
    pub p1_directions: [bool; 4],
    pub p1_buttons: [bool; 6],
    pub p2_directions: [bool; 4],
    pub p2_buttons: [bool; 6],
    pub coin1: bool,
    pub coin2: bool,
    pub system: u16,
    pub dsw1: u16,
    pub dsw2: u16,
    pub freeze_vblank_cycles: Option<u32>,
    pub latch_writes: Vec<tasks::LatchWrite>,
    pub overlay: crate::overlay::OverlayState,
}

/// Versus and one-player play on one machine, including a versus winner's
/// carry-over into the following CPU fight.
#[derive(Clone)]
pub struct FightMachine {
    pub state: MachineState,
    program: Program,
    scheduler: Scheduler,
    /// P1's logical directions (right, left, down, up) and attack buttons
    /// (LP, MP, HP, LK, MK, HK) for the frame about to be produced; all
    /// released unless a scenario sets them (ticket #27).
    p1_directions: [bool; 4],
    p1_buttons: [bool; 6],
    p2_directions: [bool; 4],
    p2_buttons: [bool; 6],
    coin1: bool,
    coin2: bool,
    system: u16,
    dsw1: u16,
    dsw2: u16,
    /// Handler cycles before the final Freeze-DIP read while the original
    /// vblank is suspended at 0x3b100; `None` outside the Freeze spin.
    freeze_vblank_cycles: Option<u32>,
    /// Every sound-latch write in the current frame; empty while frozen and
    /// two entries on release (interrupted suffix, then held vblank).
    latch_writes: Vec<tasks::LatchWrite>,
    /// Non-invasive collision and input diagnostic overlay state (`#92`, `O1`).
    overlay: crate::overlay::OverlayState,
}

impl FightMachine {
    /// Development-fixture init: the checkpoint-2389 dumps of a reference
    /// capture of sf-cpu-01 (video, palette, work RAM incl. object RAM) and
    /// the program region.
    pub fn from_checkpoint_2389(
        capture_dir: &std::path::Path,
        program_rom: Vec<u8>,
    ) -> Result<Self, String> {
        Self::load(capture_dir, 2389, program_rom)
    }

    /// Load any captured fight-loop checkpoint, in either versus or a
    /// one-player mode. Both players' inputs are accepted in every mode.
    pub fn from_fight_checkpoint(
        capture_dir: &std::path::Path,
        frame: u32,
        program_rom: Vec<u8>,
    ) -> Result<Self, String> {
        Self::load(capture_dir, frame, program_rom)
    }

    /// Compatibility constructor for the original checkpoint-2450 versus
    /// fixture; this is the same machine as [`Self::from_fight_checkpoint`].
    pub fn from_checkpoint_2450_with_program(
        capture_dir: &std::path::Path,
        program_rom: Vec<u8>,
    ) -> Result<Self, String> {
        Self::load(capture_dir, 2450, program_rom)
    }

    /// The checkpoint's dumps; the write-only scroll registers from their
    /// work-RAM copies (fg_scroll 0xffc680, bg_scroll 0xffc682); gfxctrl
    /// from its copy 0xff8682; coin_ctrl from its copy 0xff8684; the sound
    /// latch from the capture's frame record (`machine::fixture_latch`); the
    /// tasks' positions from their stacks.
    fn load(
        capture_dir: &std::path::Path,
        frame: u32,
        program_rom: Vec<u8>,
    ) -> Result<Self, String> {
        let read = |suffix: &str| {
            let path = capture_dir.join(format!("ram_{frame:06}_{suffix}.bin"));
            std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
        };
        let videoram = read("video")?;
        let paletteram = read("palette")?;
        let work = read("work")?;
        let objectram = work.get(0x6000..0x8000).ok_or("short work dump")?.to_vec();
        let reg = |a: usize| u16::from_be_bytes([work[a], work[a + 1]]);
        let latch = crate::machine::fixture_latch(capture_dir, frame)
            .ok_or("frames.tsv lacks the fixture frame")?;
        let gfxctrl = work[0x682];
        let mut state = MachineState::new(
            videoram,
            objectram,
            paletteram,
            reg(0x4682),
            reg(0x4680),
            gfxctrl,
            gfxctrl & 4 != 0,
            latch,
            frame,
        );
        state.coin_ctrl = reg(0x684);
        let (dsw1, dsw2) = crate::machine::fixture_dsw(capture_dir);
        Ok(FightMachine {
            state,
            scheduler: Scheduler::from_fixture(&work)?,
            program: Program::from_fixture(program_rom, &work).ok_or("short work dump")?,
            p1_directions: [false; 4],
            p1_buttons: [false; 6],
            p2_directions: [false; 4],
            p2_buttons: [false; 6],
            coin1: false,
            coin2: false,
            system: Io::idle().system,
            dsw1,
            dsw2,
            freeze_vblank_cycles: None,
            latch_writes: Vec::new(),
            overlay: crate::overlay::OverlayState::default(),
        })
    }

    /// Current DIP switch words (`DSW1`, `DSW2`).
    pub fn dsw(&self) -> (u16, u16) {
        (self.dsw1, self.dsw2)
    }

    /// Sound-latch writes produced by the current frame, in bus order. On a
    /// Freeze release the second write's cycles are cumulative from the
    /// interrupted suffix's cycle origin, so their difference is the full
    /// post-write / held-vblank interval.
    pub fn latch_writes(&self) -> &[tasks::LatchWrite] {
        &self.latch_writes
    }

    /// Active task slots and their current entry addresses.
    pub fn task_positions(&self) -> Vec<(u32, u32)> {
        self.scheduler.positions()
    }

    /// Original addresses of untranslated branches reached so far.
    pub fn unrecovered(&self) -> Vec<u32> {
        self.program.unrecovered.iter().copied().collect()
    }

    /// Target addresses of effect `0xff` stream calls (`0x42dd2`) reached so far.
    pub fn effect_stream_calls(&self) -> Vec<u32> {
        self.program.effect_stream_calls.iter().copied().collect()
    }

    /// Work RAM 0xff8000-0xffdfff as the translated routines keep it.
    pub fn work_ram(&self) -> &[u8] {
        &self.program.work
    }

    /// Write one byte in work RAM (`0xff8000..=0xffdfff`), for branch and
    /// fault-injection tests.
    pub fn set_work_byte(&mut self, addr: u32, val: u8) {
        self.program.work[(addr - 0xff8000) as usize] = val;
    }

    /// Set P1's logical direction inputs for the frame about to be
    /// produced (sampled at its vblank, as on the versus machine).
    pub fn set_p1_direction(&mut self, right: bool, left: bool, down: bool, up: bool) {
        self.p1_directions = [right, left, down, up];
    }

    /// Set P1's six attack-button levels (LP, MP, HP, LK, MK, HK).
    pub fn set_p1_buttons(&mut self, buttons: [bool; 6]) {
        self.p1_buttons = buttons;
    }

    /// Set P2's logical direction inputs (for IN1 readback in 1P sessions).
    pub fn set_p2_direction(&mut self, right: bool, left: bool, down: bool, up: bool) {
        self.p2_directions = [right, left, down, up];
    }

    /// Set P2's six attack-button levels (LP, MP, HP, LK, MK, HK).
    pub fn set_p2_buttons(&mut self, buttons: [bool; 6]) {
        self.p2_buttons = buttons;
    }

    /// Set the live DIP switch words (`DSW1`, `DSW2`) from a scenario.
    pub fn set_dsw(&mut self, dsw1: u16, dsw2: u16) {
        self.dsw1 = dsw1;
        self.dsw2 = dsw2;
    }

    /// Set the cabinet `SYSTEM` port word (`0xc0000c`, active low; bit 0 = P1
    /// start, bit 1 = P2 start, bit 2 = service1).
    pub fn set_system(&mut self, system: u16) {
        self.system = system;
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
        self.coin1 = coin1;
        self.coin2 = coin2;
        let mut sys = Io::idle().system;
        for (held, mask) in [(start1, 0x0001), (start2, 0x0002), (service1, 0x0004)] {
            if held {
                sys &= !mask;
            }
        }
        self.system = sys;
    }

    /// This frame's hardware inputs from the logical ones: the active-low
    /// P2 : P1 bytes of IN1 (directions bits 0-3 / 8-11, LP 0x10, MP 0x20,
    /// LK 0x40, MK 0x80) and coin1 / coin2 / HP / HK in IN0.
    fn io(&self) -> Io {
        let [r, l, d, u] = self.p1_directions;
        let [lp, mp, hp, lk, mk, hk] = self.p1_buttons;
        let [r2, l2, d2, u2] = self.p2_directions;
        let [lp2, mp2, hp2, lk2, mk2, hk2] = self.p2_buttons;
        let mut io = Io {
            system: self.system,
            dsw1: self.dsw1,
            dsw2: self.dsw2,
            ..Io::idle()
        };
        for (held, bit) in [
            (r, 0),
            (l, 1),
            (d, 2),
            (u, 3),
            (lp, 4),
            (mp, 5),
            (lk, 6),
            (mk, 7),
            (r2, 8),
            (l2, 9),
            (d2, 10),
            (u2, 11),
            (lp2, 12),
            (mp2, 13),
            (lk2, 14),
            (mk2, 15),
        ] {
            if held {
                io.in1 &= !(1 << bit);
            }
        }
        for (held, mask) in [
            (self.coin1, 0x01),
            (self.coin2, 0x02),
            (hp, 0x200),
            (hk, 0x04),
            (hp2, 0x400),
            (hk2, 0x100),
        ] {
            if held {
                io.in0 &= !mask;
            }
        }
        io
    }

    /// Advance one frame: produce the end-of-(frame+1) state from the
    /// current logical inputs (released unless set).
    pub fn step(&mut self) {
        self.overlay
            .capture_pre_step(&self.program.work, &self.state.objectram);
        self.state.frame += 1;
        self.latch_writes.clear();
        let io = self.io();
        let scheduler = &mut self.scheduler;
        let mut bus = self.program.bus_with_dsw1(&mut self.state, io.dsw1);
        let latch = if let Some(cycles) = self.freeze_vblank_cycles {
            if io.dsw1 & 0x4000 == 0 {
                // Still in 0x3b100..0x3b108: no new interrupt is accepted,
                // and neither game memory nor the scheduler advances.
                self.state.latch_cycles = 0;
                self.overlay
                    .record_post_step(&self.program.rom, self.program.data_bank.as_deref(), &self.program.work, &self.state);
                return;
            }
            // Finish the interrupted handler. Its RTE accepts the one held
            // vblank request, which runs completely before game tasks resume.
            self.freeze_vblank_cycles = None;
            let resumed = tasks::vblank_suffix(&mut bus, &io, cycles);
            let mut held = tasks::vblank(&mut bus, &io);
            held.cycles += resumed.latch.cycles + resumed.held_entry_cycles;
            self.latch_writes = vec![resumed.latch, held];
            held
        } else {
            let cycles = tasks::vblank_prefix(&mut bus, &io);
            if io.dsw1 & 0x4000 == 0 {
                self.freeze_vblank_cycles = Some(cycles);
                self.state.latch_cycles = 0;
                self.overlay
                    .record_post_step(&self.program.rom, self.program.data_bank.as_deref(), &self.program.work, &self.state);
                return;
            }
            let suffix = tasks::vblank_suffix(&mut bus, &io, cycles);
            self.latch_writes = vec![suffix.latch];
            suffix.latch
        };
        scheduler.run(&mut bus, &io);
        self.state.sound_latch = latch.value;
        self.state.latch_cycles = latch.cycles;
        self.overlay
            .record_post_step(&self.program.rom, self.program.data_bank.as_deref(), &self.program.work, &self.state);
    }

    /// The CPU opponent's entity offset (Retsu: CPU structure +0xe low word).
    pub fn cpu_entity(&self) -> u16 {
        let w = &self.program.work;
        let a = (CPU_STRUCT - 0xff8000) as usize + 0x10;
        u16::from_be_bytes([w[a], w[a + 1]])
    }

    /// Whether the diagnostic overlay is currently enabled (`#92`, `O1`).
    pub fn overlay_enabled(&self) -> bool {
        self.overlay.enabled
    }

    /// Sprites beyond the 1987 object list. The overhaul compositor draws them.
    pub fn set_overhaul_sprites(&mut self, sprites: Vec<sf_video::compositor::IndexedSprite>) {
        self.overlay.set_overhaul_sprites(sprites);
    }

    /// Character-select lines for the overhaul HUD.
    pub fn set_select_lines(&mut self, lines: Vec<String>) {
        self.overlay.set_select_lines(lines);
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
        self.overlay
            .diagnostic_telemetry(&self.program.rom, self.program.data_bank.as_deref(), &self.program.work, &self.state)
    }

    /// Render the current scene and composite the diagnostic overlay when enabled,
    /// leaving the base `384x224` game-frame rendering and machine state untouched.
    pub fn render_with_overlay(
        &self,
        assets: &sf_video::romset::Assets,
    ) -> sf_video::render::Frame {
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

    /// Active match configuration (ruleset preset and Tier 1 core toggles, `#94`, `O3`).
    pub fn match_config(&self) -> &crate::config::MatchConfig {
        &self.program.match_config
    }

    /// Mutable access to the active match configuration (`#94`, `O3`).
    pub fn match_config_mut(&mut self) -> &mut crate::config::MatchConfig {
        &mut self.program.match_config
    }

    /// Replace the active [`crate::config::MatchConfig`] (`#94` `O3`, `#95` `O4`, `#96` `O5`).
    pub fn set_match_config(&mut self, config: crate::config::MatchConfig) {
        self.program.match_config = config;
        if !config.toggles.positive_edge_attacks {
            self.program.toggle_state.neg_edge_latch.fill(false);
        }
        if !config.toggles.recovery_input_buffer {
            self.program.toggle_state.recovery_buffer.fill(None);
        }
        if !config.toggles.lenient_command_trackers {
            self.program.toggle_state.tracker_release_age.fill([0; 3]);
        }
        if !config.toggles.anytime_jump_attacks {
            self.program.toggle_state.air_adj_frames_1.fill(0);
            self.program.toggle_state.air_adj_rem_1.fill(0);
            self.program.toggle_state.air_adj_frames_2.fill(0);
            self.program.toggle_state.air_adj_rem_2.fill(0);
        }
        if !config.toggles.balanced_roster {
            self.program.toggle_state.dp_connected.fill(false);
        }
    }

    /// Run a closure with a mutable [`crate::fighter::Bus`] view over the machine's
    /// program ROM, work RAM, and hardware state (`#96`, `O5`).
    pub fn with_bus<R>(&mut self, f: impl FnOnce(&mut crate::fighter::Bus<'_>) -> R) -> R {
        let mut bus = self.program.bus_with_dsw1(&mut self.state, self.dsw1);
        f(&mut bus)
    }

    /// Builder: set the active [`crate::config::MatchConfig`] (`#94`, `O3`).
    pub fn with_match_config(mut self, config: crate::config::MatchConfig) -> Self {
        self.set_match_config(config);
        self
    }

    /// Attach the extended data bank (`0x10_0000..`, ADR-0014): 1987 player
    /// move tables built for the bosses from the art pack's poses.
    pub fn with_data_bank(mut self, bank: std::sync::Arc<crate::data_bank::DataBank>) -> Self {
        self.program.data_bank = Some(bank);
        self
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
    pub fn save_snapshot(&self) -> FightSnapshot {
        FightSnapshot {
            state: self.state.clone(),
            work: self.program.work.clone(),
            unrecovered: self.program.unrecovered.clone(),
            effect_stream_calls: self.program.effect_stream_calls.clone(),
            match_config: self.program.match_config,
            toggle_state: self.program.toggle_state,
            training: self.program.training,
            scheduler: self.scheduler,
            p1_directions: self.p1_directions,
            p1_buttons: self.p1_buttons,
            p2_directions: self.p2_directions,
            p2_buttons: self.p2_buttons,
            coin1: self.coin1,
            coin2: self.coin2,
            system: self.system,
            dsw1: self.dsw1,
            dsw2: self.dsw2,
            freeze_vblank_cycles: self.freeze_vblank_cycles,
            latch_writes: self.latch_writes.clone(),
            overlay: self.overlay.clone(),
        }
    }

    /// Restore a deterministic full-session state snapshot saved by [`Self::save_snapshot`] (`#93`, `O2`).
    pub fn restore_snapshot(&mut self, snapshot: &FightSnapshot) {
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
        self.p1_directions = snapshot.p1_directions;
        self.p1_buttons = snapshot.p1_buttons;
        self.p2_directions = snapshot.p2_directions;
        self.p2_buttons = snapshot.p2_buttons;
        self.coin1 = snapshot.coin1;
        self.coin2 = snapshot.coin2;
        self.system = snapshot.system;
        self.dsw1 = snapshot.dsw1;
        self.dsw2 = snapshot.dsw2;
        self.freeze_vblank_cycles = snapshot.freeze_vblank_cycles;
        self.latch_writes.clone_from(&snapshot.latch_writes);
        self.overlay = snapshot.overlay.clone();
    }
}
