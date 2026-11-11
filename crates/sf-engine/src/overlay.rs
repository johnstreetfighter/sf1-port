//! Real-time collision, input, and 60-cell frame meter diagnostic overlay (`#92` `O1`, `#93` `O2`).
//!
//! Extracts non-invasive per-frame diagnostic telemetry from the 1987 engine's
//! work RAM (`0xff8000..0xffe000`), object RAM (`0xffe000..0xffffff`), and
//! read-only program ROM (`0x000000..0x05ffff`), and composites color-coded
//! collision geometry, HUD telemetry, and a 60-cell rolling frame meter onto a
//! post-frame overlay buffer without mutating any simulation state or base
//! `384x224` game-frame pixels.
//!
//! Original 68000 references:
//! - Fighter center (`0x43be8` / `0x43bda`)
//! - Pushbox separation & clamps (`0x43d7c`, `0x43c18`, `0x43c9a`)
//! - Two-stage hurtbox (`depth 2` loose, `depth 1` tight) & melee hitbox judge (`0x43e82`)
//! - Projectile / effect center & collisions (`0x43234`, `0x430b6`, `0x43144`, `0x431c6`)
//! - Button hold-duration & negative-edge release latches (`0x3b3ae` family)
//! - Walk-cycle busy gate (`0x403d4`, `0x91` gated vs `0x92` actionable)
//! - Special-move 18-frame command trackers (`0x40360..0x403b2`, `0x40ae4`)
//! - Move-stream lifecycle (`0x441b0` `start_move`, `0x42c4a` `end_move`, `0x4451a` `react`)

use sf_video::callouts::Side;
use sf_video::render::{render_frame, Frame, HEIGHT, VISAREA_X, VISAREA_Y, WIDTH};
use sf_video::romset::Assets;

use crate::fight_reader::FIGHTER_SLOTS;
use crate::fighter::{CPU_STRUCT, DRAMATIC_ALLY2, OBJ_BASE, P1_STRUCT, P2_STRUCT, WORK_BASE};
use crate::machine::MachineState;

/// Number of rolling cells in the per-fighter visual frame meter (`#93`, `O2`).
pub const FRAME_METER_CELLS: usize = 60;

/// Rows the frame meter's panel fills at the bottom of the screen.
pub const FRAME_METER_ROWS: usize = 20;
/// The panel's drawn height, within [`FRAME_METER_ROWS`].
const FRAME_METER_PANEL_HEIGHT: usize = 18;
/// How strongly the frame meter's panel tints the frame under it, out of 256.
///
/// the maintainer picked option B from the mockups (spec #238, design 5.3): a faint
/// panel, with the cells and text blended over the frame instead of drawn
/// solid, so the floor and the fighters' feet show through. The mockup used
/// 40 here (it was 200). The value is tuned by eye, not derived.
pub const FRAME_METER_PANEL_ALPHA: u32 = 40;
/// How strongly the frame meter's cells and text show over the frame, out of
/// 256. Option B's mockup used 170; tuned by eye like
/// [`FRAME_METER_PANEL_ALPHA`].
pub const FRAME_METER_CELL_ALPHA: u32 = 170;
/// A telemetry box: the text lines and a 2-pixel margin.
const TELEMETRY_WIDTH: usize = 152;
const TELEMETRY_HEIGHT: usize = 22;
/// Rows between the super's move-name line and the telemetry boxes.
/// The telemetry boxes' top edge: the panels' text starts two rows lower.
const TELEMETRY_TOP: usize = 24;

/// The Overlay's fixed-position boxes: each fighter's telemetry box
/// (`Side::P1` at the left), where the Training overlay has always drawn it,
/// over the 1987 health block and above TIME, the call-out columns and the
/// gauge corners, so it clears every element the era HUD adds and never
/// sits over the fighters. The hitbox viewer has no fixed part: its drawings
/// follow the fighters.
pub fn telemetry_box(side: sf_video::callouts::Side) -> sf_video::callouts::regions::Rect {
    use sf_video::callouts::regions;
    match side {
        Side::P1 => regions::Rect::new(2, TELEMETRY_TOP, TELEMETRY_WIDTH, TELEMETRY_HEIGHT),
        Side::P2 => regions::Rect::new(WIDTH - 6 - TELEMETRY_WIDTH, TELEMETRY_TOP, TELEMETRY_WIDTH, TELEMETRY_HEIGHT),
    }
}

/// The frame meter's panel.
pub fn frame_meter_panel() -> sf_video::callouts::regions::Rect {
    sf_video::callouts::regions::Rect::new(
        4,
        HEIGHT - FRAME_METER_ROWS,
        WIDTH - 8,
        FRAME_METER_PANEL_HEIGHT,
    )
}

/// Rows the era HUD's gauge corners are lifted while the meter is `on`:
/// their bottom edge sits at the panel's top.
pub fn gauge_lift(meter: bool) -> usize {
    if meter {
        sf_video::era_hud::lift_above_rows(FRAME_METER_ROWS)
    } else {
        0
    }
}

/// Every fixed box the overlay draws with its meter part `meter` on.
pub fn fixed_boxes(meter: bool) -> Vec<sf_video::callouts::regions::Rect> {
    if !meter {
        return Vec::new();
    }
    let mut out: Vec<_> = Side::BOTH.map(telemetry_box).into();
    out.push(frame_meter_panel());
    out
}

/// Per-frame classification in the 60-cell rolling visual frame meter (`#93`, `O2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameCellState {
    #[default]
    Neutral,
    Startup,
    Active,
    Recovery,
    Hitstun,
    Blockstun,
    Invulnerable,
    CancelWindow,
}

impl FrameCellState {
    /// Distinct ARGB color (`0xffRRGGBB`) for this frame-meter cell state.
    pub fn color(self) -> u32 {
        match self {
            Self::Neutral => OverlayColors::FRAME_NEUTRAL,
            Self::Startup => OverlayColors::FRAME_STARTUP,
            Self::Active => OverlayColors::FRAME_ACTIVE,
            Self::Recovery => OverlayColors::FRAME_RECOVERY,
            Self::Hitstun => OverlayColors::FRAME_HITSTUN,
            Self::Blockstun => OverlayColors::FRAME_BLOCKSTUN,
            Self::Invulnerable => OverlayColors::FRAME_INVULNERABLE,
            Self::CancelWindow => OverlayColors::FRAME_CANCEL_WINDOW,
        }
    }

    /// Three-letter HUD label for this frame-meter cell state.
    pub fn label(self) -> &'static str {
        match self {
            Self::Neutral => "NEU",
            Self::Startup => "STR",
            Self::Active => "ACT",
            Self::Recovery => "REC",
            Self::Hitstun => "HIT",
            Self::Blockstun => "BLK",
            Self::Invulnerable => "INV",
            Self::CancelWindow => "CAN",
        }
    }
}

/// Whether the most recent hit/block exchange was an unblocked hit or a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionKind {
    Hit,
    Block,
}

/// Rolling 60-cell visual frame meter and calculated frame advantage for one fighter (`#93`, `O2`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerFrameMeter {
    /// Rolling 60-cell history buffer (`cells[FRAME_METER_CELLS - 1]` is the latest frame).
    pub cells: [FrameCellState; FRAME_METER_CELLS],
    /// Total frames recorded into this meter.
    pub recorded_frames: u32,
    /// Calculated signed frame advantage (`+` or `-`) from the most recent hit or block
    /// interaction (`opponent_locked_frames - self_locked_frames`).
    pub frame_advantage: Option<i16>,
    /// Whether the most recent interaction was a [`InteractionKind::Hit`] or [`InteractionKind::Block`].
    pub last_interaction: Option<InteractionKind>,
    prev_move_id: u8,
    prev_state_byte: u8,
    seen_active_in_move: bool,
    in_chip_blockstun: bool,
}

impl Default for PlayerFrameMeter {
    fn default() -> Self {
        Self {
            cells: [FrameCellState::Neutral; FRAME_METER_CELLS],
            recorded_frames: 0,
            frame_advantage: None,
            last_interaction: None,
            prev_move_id: 0,
            prev_state_byte: 0,
            seen_active_in_move: false,
            in_chip_blockstun: false,
        }
    }
}

impl PlayerFrameMeter {
    /// Most recently recorded cell state (or [`FrameCellState::Neutral`] if none recorded yet).
    pub fn current_cell(&self) -> FrameCellState {
        self.cells[FRAME_METER_CELLS - 1]
    }

    fn push_cell(&mut self, cell: FrameCellState) {
        self.cells.copy_within(1..FRAME_METER_CELLS, 0);
        self.cells[FRAME_METER_CELLS - 1] = cell;
        self.recorded_frames = self.recorded_frames.saturating_add(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockKind {
    None,
    Attack { move_id: u8 },
    Hitstun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActiveInteraction {
    kind: InteractionKind,
    p1_lock: LockKind,
    p2_lock: LockKind,
    p1_frames: i16,
    p2_frames: i16,
}

/// Combined 60-cell frame meter state for both fighters (`#93`, `O2`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FrameMeterState {
    pub p1: PlayerFrameMeter,
    pub p2: PlayerFrameMeter,
    active_interaction: Option<ActiveInteraction>,
}

impl FrameMeterState {
    fn update_chip_blockstun(
        meter: &mut PlayerFrameMeter,
        me: &FighterTelemetry,
        other: &FighterTelemetry,
    ) {
        let prev_blocking = (meter.prev_state_byte & 0x20) != 0
            || meter.prev_move_id == 4
            || meter.prev_move_id == 8
            || meter.in_chip_blockstun;
        if me.active && (0x30..=0x3f).contains(&me.move_id) && (me.state_byte & 0x80) != 0 {
            if me.move_id == 0x3b
                || (meter.prev_move_id < 0x30
                    && prev_blocking
                    && ((other.attack_connected_this_frame && other.move_id == 0x1b)
                        || me.projectile_impacted_this_frame))
            {
                meter.in_chip_blockstun = true;
            } else if other.attack_connected_this_frame && other.move_id != 0x1b {
                meter.in_chip_blockstun = false;
            }
        } else {
            meter.in_chip_blockstun = false;
        }
    }

    /// Record one completed simulation step into both fighters' 60-cell rolling
    /// frame meters and update the hit/block frame-advantage tracker.
    pub fn record_frame(&mut self, telemetry: &DiagnosticTelemetry) {
        Self::update_chip_blockstun(&mut self.p1, &telemetry.p1, &telemetry.p2);
        Self::update_chip_blockstun(&mut self.p2, &telemetry.p2, &telemetry.p1);

        let p1_cell = Self::classify_fighter(&mut self.p1, &telemetry.p1);
        let p2_cell = Self::classify_fighter(&mut self.p2, &telemetry.p2);
        self.p1.push_cell(p1_cell);
        self.p2.push_cell(p2_cell);

        // Check if a new hit or block interaction started on this frame.
        let new_interaction = Self::detect_interaction(
            &telemetry.p1,
            &telemetry.p2,
            self.p1.in_chip_blockstun,
            self.p2.in_chip_blockstun,
            self.p1.prev_move_id,
            self.p2.prev_move_id,
        );

        if let Some(kind) = new_interaction {
            let p1_lock = Self::initial_lock_kind(&telemetry.p1, self.p1.in_chip_blockstun);
            let p2_lock = Self::initial_lock_kind(&telemetry.p2, self.p2.in_chip_blockstun);
            self.active_interaction = Some(ActiveInteraction {
                kind,
                p1_lock,
                p2_lock,
                p1_frames: 0,
                p2_frames: 0,
            });
            self.p1.last_interaction = Some(kind);
            self.p2.last_interaction = Some(kind);
            self.p1.frame_advantage = Some(0);
            self.p2.frame_advantage = Some(0);
        } else if let Some(mut inter) = self.active_interaction {
            if !Self::remains_locked(&telemetry.p1, inter.p1_lock, self.p1.in_chip_blockstun) {
                inter.p1_lock = LockKind::None;
            } else {
                inter.p1_frames = inter.p1_frames.saturating_add(1);
            }
            if !Self::remains_locked(&telemetry.p2, inter.p2_lock, self.p2.in_chip_blockstun) {
                inter.p2_lock = LockKind::None;
            } else {
                inter.p2_frames = inter.p2_frames.saturating_add(1);
            }

            let p1_adv = inter.p2_frames - inter.p1_frames;
            self.p1.frame_advantage = Some(p1_adv);
            self.p2.frame_advantage = Some(-p1_adv);
            self.p1.last_interaction = Some(inter.kind);
            self.p2.last_interaction = Some(inter.kind);

            if inter.p1_lock == LockKind::None && inter.p2_lock == LockKind::None {
                self.active_interaction = None;
            } else {
                self.active_interaction = Some(inter);
            }
        }

        self.p1.prev_move_id = telemetry.p1.move_id;
        self.p1.prev_state_byte = telemetry.p1.state_byte;
        self.p2.prev_move_id = telemetry.p2.move_id;
        self.p2.prev_state_byte = telemetry.p2.state_byte;
    }

    fn classify_fighter(meter: &mut PlayerFrameMeter, f: &FighterTelemetry) -> FrameCellState {
        if !f.active {
            meter.seen_active_in_move = false;
            return FrameCellState::Neutral;
        }

        let in_attack_move = (0x0d..=0x2f).contains(&f.move_id) && (f.state_byte & 0x80) != 0;
        if !in_attack_move || f.move_started_this_frame || f.move_id != meter.prev_move_id {
            meter.seen_active_in_move = false;
        }

        let is_active_attack_frame =
            f.attack_active || f.attack_connected_this_frame || f.spawned_projectile_this_frame;
        if in_attack_move && is_active_attack_frame {
            meter.seen_active_in_move = true;
        }

        // 1. Blockstun (standing guard move 4, crouching guard move 8, state bit 0x20, or chip blockstun).
        if (f.state_byte & 0x20) != 0 || f.move_id == 4 || f.move_id == 8 || meter.in_chip_blockstun
        {
            return FrameCellState::Blockstun;
        }

        // 2. Hitstun (alive reaction 0x30..=0x3f while busy bit 0x80 is set, or KO 0x40..=0x4f).
        if f.move_id >= 0x40 || (f.move_id >= 0x30 && (f.state_byte & 0x80) != 0) {
            return FrameCellState::Hitstun;
        }

        // 3. Attack move lifecycle: Invulnerable / Startup / Active / Recovery.
        if in_attack_move {
            if is_active_attack_frame {
                return FrameCellState::Active;
            }
            if f.invulnerable {
                return FrameCellState::Invulnerable;
            }
            if !meter.seen_active_in_move {
                return FrameCellState::Startup;
            }
            return FrameCellState::Recovery;
        }

        // 4. Invulnerability outside an attack move.
        if f.invulnerable {
            return FrameCellState::Invulnerable;
        }

        // 5. Cancel windows:
        //    - Walk-cycle actionable window (the 2 `0x92` frames per 10-frame walk cycle at 0x403d4)
        //    - Open special-move command-completion window (`done == 0xff`) while actionable
        //    - Buffered air-attack pre-apex jump window (`move_id` 9..=0x0b with `state_byte == 0xc1`)
        let special_window_ready = f.walk_gate.actionable
            && (f.hadoken.done == 0xff || f.shoryuken.done == 0xff || f.tatsumaki.done == 0xff);
        let walk_cancel_window = f.walk_gate.in_walk_cycle && f.walk_gate.actionable;
        let air_buffer_window = matches!(f.move_id, 9..=0x0b) && f.state_byte == 0xc1;
        if walk_cancel_window || special_window_ready || air_buffer_window {
            return FrameCellState::CancelWindow;
        }

        FrameCellState::Neutral
    }

    fn detect_interaction(
        p1: &FighterTelemetry,
        p2: &FighterTelemetry,
        p1_chip_block: bool,
        p2_chip_block: bool,
        p1_prev_mv: u8,
        p2_prev_mv: u8,
    ) -> Option<InteractionKind> {
        if p1.attack_connected_this_frame {
            if p2.move_id < 0x30 || p2_chip_block {
                return Some(InteractionKind::Block);
            }
            return Some(InteractionKind::Hit);
        }
        if p2.attack_connected_this_frame {
            if p1.move_id < 0x30 || p1_chip_block {
                return Some(InteractionKind::Block);
            }
            return Some(InteractionKind::Hit);
        }
        if p1.projectile_impacted_this_frame {
            if p1.move_id < 0x30 || p1_chip_block {
                return Some(InteractionKind::Block);
            }
            return Some(InteractionKind::Hit);
        }
        if p2.projectile_impacted_this_frame {
            if p2.move_id < 0x30 || p2_chip_block {
                return Some(InteractionKind::Block);
            }
            return Some(InteractionKind::Hit);
        }
        // Throw start (`0x1c` / `0x1d` putting defender into `0x35`).
        if (p1_prev_mv < 0x30 && p1.move_id >= 0x30) || (p2_prev_mv < 0x30 && p2.move_id >= 0x30) {
            return Some(InteractionKind::Hit);
        }
        None
    }

    fn initial_lock_kind(f: &FighterTelemetry, in_chip_block: bool) -> LockKind {
        if !f.active || (f.state_byte & 0x80) == 0 {
            return LockKind::None;
        }
        if in_chip_block || (0x30..=0x4f).contains(&f.move_id) {
            return LockKind::Hitstun;
        }
        if (0x0d..=0x2f).contains(&f.move_id) {
            return LockKind::Attack { move_id: f.move_id };
        }
        LockKind::None
    }

    fn remains_locked(f: &FighterTelemetry, lock: LockKind, in_chip_block: bool) -> bool {
        if !f.active || (f.state_byte & 0x80) == 0 {
            return false;
        }
        match lock {
            LockKind::None => false,
            LockKind::Hitstun => in_chip_block || (0x30..=0x4f).contains(&f.move_id),
            LockKind::Attack { move_id } => f.move_id == move_id && !f.move_started_this_frame,
        }
    }
}

/// Non-invasive overlay state shared by [`crate::fight::FightMachine`] and
/// [`crate::run::PowerOnMachine`]. Keeps the enable flag, a read-only pre-step
/// RAM snapshot, and the 60-cell rolling frame meter (`#92` `O1`, `#93` `O2`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverlayState {
    pub enabled: bool,
    prev_work: Option<Vec<u8>>,
    prev_obj: Option<Vec<u8>>,
    pub frame_meter: FrameMeterState,
    /// Sprites the 1987 object list cannot hold. Drawn only in overhaul play.
    extra_sprites: Vec<sf_video::compositor::IndexedSprite>,
    /// Character and style select lines. Drawn only in overhaul play.
    select_lines: Vec<String>,
}

impl OverlayState {
    /// Extra sprites for the overhaul compositor. Faithful play ignores them.
    pub fn set_overhaul_sprites(&mut self, sprites: Vec<sf_video::compositor::IndexedSprite>) {
        self.extra_sprites = sprites;
    }

    /// Select-screen lines for the overhaul HUD. Faithful play ignores them.
    pub fn set_select_lines(&mut self, lines: Vec<String>) {
        self.select_lines = lines;
    }

    /// Save the pre-step work and object RAM slices before advancing a frame.
    pub fn capture_pre_step(&mut self, work: &[u8], obj: &[u8]) {
        self.prev_work = Some(work.to_vec());
        self.prev_obj = Some(obj.to_vec());
    }

    /// Record the completed simulation step into the 60-cell rolling frame meter.
    pub fn record_post_step(&mut self, rom: &[u8], bank: Option<&crate::data_bank::DataBank>, work: &[u8], state: &MachineState) {
        let prev = match (&self.prev_work, &self.prev_obj) {
            (Some(pw), Some(po)) => Some((pw.as_slice(), po.as_slice())),
            _ => None,
        };
        let telemetry = extract_diagnostic_telemetry_banked(rom, bank, work, &state.objectram, prev, state.flip_screen);
        self.frame_meter.record_frame(&telemetry);
    }

    /// Extract diagnostic telemetry using the current state, the pre-step
    /// snapshot (when a step has occurred), and the 60-cell rolling frame meter.
    pub fn diagnostic_telemetry(
        &self,
        rom: &[u8],
        bank: Option<&crate::data_bank::DataBank>,
        work: &[u8],
        state: &MachineState,
    ) -> DiagnosticTelemetry {
        let prev = match (&self.prev_work, &self.prev_obj) {
            (Some(pw), Some(po)) => Some((pw.as_slice(), po.as_slice())),
            _ => None,
        };
        let mut telemetry = extract_diagnostic_telemetry_banked(rom, bank, work, &state.objectram, prev, state.flip_screen);
        telemetry.frame_meter = self.frame_meter.clone();
        telemetry
    }

    /// Render the current scene and composite the diagnostic overlay when
    /// `self.enabled` is `true`, leaving the base `384x224` game-frame pixels
    /// and machine state untouched.
    pub fn render_with_overlay(
        &self,
        assets: &Assets,
        rom: &[u8],
        work: &[u8],
        state: &MachineState,
    ) -> Frame {
        let base = render_frame(assets, &state.scene());
        if !self.enabled {
            return base;
        }
        let telemetry = self.diagnostic_telemetry(rom, None, work, state);
        render_diagnostic_overlay(&base, &telemetry, true)
    }

    /// Faithful play uses the hardware frame (and the diagnostic overlay when
    /// it is enabled). Overhaul play goes through the extended compositor:
    /// the same hardware frame, then the extra sprites, select type and the
    /// training overlay. The 1987 fighters carry no era state, so no era
    /// gauges are drawn here.
    pub fn render_presented(
        &self,
        assets: &Assets,
        rom: &[u8],
        work: &[u8],
        state: &MachineState,
        overhaul: bool,
    ) -> Frame {
        if !overhaul {
            return self.render_with_overlay(assets, rom, work, state);
        }
        let hardware = render_frame(assets, &state.scene());
        let training = if self.enabled {
            let diagnosed = self.render_with_overlay(assets, rom, work, state);
            let mut layer = vec![0u32; hardware.argb.len()];
            for (slot, (over, base)) in layer
                .iter_mut()
                .zip(diagnosed.argb.iter().zip(hardware.argb.iter()))
            {
                if over != base {
                    *slot = *over;
                }
            }
            Some(layer)
        } else {
            None
        };
        sf_video::compositor::present(
            sf_video::compositor::PresentMode::Overhaul,
            &hardware,
            &sf_video::compositor::OverhaulScene {
                sprites: &self.extra_sprites,
                select_lines: &self.select_lines,
                training: training.as_deref(),
                ..sf_video::compositor::OverhaulScene::EMPTY
            },
        )
    }
}

/// Distinct ARGB colors (`0xffRRGGBB`) used by the diagnostic overlay renderer.
pub struct OverlayColors;

impl OverlayColors {
    /// Pushbox border (`+0x16..+0x19`): emerald green.
    pub const PUSHBOX_BORDER: u32 = 0xff00_ff66;
    /// Pushbox translucent interior tint.
    pub const PUSHBOX_FILL: u32 = 0xff00_c048;

    /// Outer loose hurtbox (`depth 2`, `+0x1e..+0x21`): cyan-blue.
    pub const LOOSE_HURTBOX_BORDER: u32 = 0xff38_a8ff;
    /// Outer loose hurtbox interior tint.
    pub const LOOSE_HURTBOX_FILL: u32 = 0xff18_68c0;

    /// Inner tight / critical hurtbox (`depth 1`, `+0x1a..+0x1d`): gold-yellow.
    pub const TIGHT_HURTBOX_BORDER: u32 = 0xffff_dc20;
    /// Inner tight hurtbox interior tint.
    pub const TIGHT_HURTBOX_FILL: u32 = 0xffc8_9800;

    /// Active melee attack hitbox (`+0xf` bit 7, `+0x22..+0x25`): crimson red.
    pub const MELEE_HITBOX_BORDER: u32 = 0xffff_2838;
    /// Active melee attack hitbox interior tint.
    pub const MELEE_HITBOX_FILL: u32 = 0xffd0_1020;

    /// Active projectile hitbox (`0x430b6` / `0x431c6`): magenta.
    pub const PROJECTILE_HITBOX_BORDER: u32 = 0xffff_50e8;
    /// Active projectile hitbox interior tint.
    pub const PROJECTILE_HITBOX_FILL: u32 = 0xffb8_20a8;

    /// Invulnerability state (`+0xf` bit 3): pure white highlight.
    pub const INVULNERABLE_BORDER: u32 = 0xffff_ffff;
    /// Invulnerability interior tint.
    pub const INVULNERABLE_FILL: u32 = 0xffc0_d8ff;

    /// HUD panel background tint.
    pub const HUD_BG: u32 = 0xff0c_1018;
    /// HUD primary text color.
    pub const HUD_TEXT: u32 = 0xffe8_f0ff;
    /// HUD active / ready highlight color.
    pub const HUD_ACTIVE: u32 = 0xff40_ff80;
    /// HUD warning / gated highlight color.
    pub const HUD_WARN: u32 = 0xffff_9820;

    /// Frame meter Neutral cell (`#93`, `O2`): slate gray.
    pub const FRAME_NEUTRAL: u32 = 0xff28_303c;
    /// Frame meter Startup cell (`#93`, `O2`): teal-green.
    pub const FRAME_STARTUP: u32 = 0xff18_c890;
    /// Frame meter Active cell (`#93`, `O2`): crimson red.
    pub const FRAME_ACTIVE: u32 = 0xffff_2838;
    /// Frame meter Recovery cell (`#93`, `O2`): cobalt blue.
    pub const FRAME_RECOVERY: u32 = 0xff28_78ff;
    /// Frame meter Hitstun cell (`#93`, `O2`): amber-gold.
    pub const FRAME_HITSTUN: u32 = 0xffff_b818;
    /// Frame meter Blockstun cell (`#93`, `O2`): violet-purple.
    pub const FRAME_BLOCKSTUN: u32 = 0xffa8_48ff;
    /// Frame meter Invulnerable cell (`#93`, `O2`): pure white.
    pub const FRAME_INVULNERABLE: u32 = 0xffff_ffff;
    /// Frame meter CancelWindow cell (`#93`, `O2`): lime-cyan.
    pub const FRAME_CANCEL_WINDOW: u32 = 0xff60_ff40;
}

/// An axis-aligned bounding box in 1987 bitmap coordinates (`sx`/`sy` space,
/// before subtracting `VISAREA_X = 64` and `VISAREA_Y = 16`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoundingBox {
    pub center_x: i32,
    pub center_y: i32,
    pub half_w: i32,
    pub half_h: i32,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl BoundingBox {
    /// Construct a symmetric bounding box from `(center_x, center_y)` and
    /// `(half_w, half_h)`, matching `0x43e82` and `0x430b6`.
    pub fn from_center_half(center_x: i32, center_y: i32, half_w: i32, half_h: i32) -> Self {
        Self {
            center_x,
            center_y,
            half_w,
            half_h,
            left: center_x - half_w,
            top: center_y - half_h,
            right: center_x + half_w,
            bottom: center_y + half_h,
        }
    }

    /// Construct a bounding box from explicit bitmap edges around `(center_x, center_y)`
    /// (used for the facing-dependent pushbox at `0x43d7c` / `0x43c9a`).
    pub fn from_edges(
        center_x: i32,
        center_y: i32,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Self {
        Self {
            center_x,
            center_y,
            half_w: (right - left) / 2,
            half_h: (bottom - top) / 2,
            left,
            top,
            right,
            bottom,
        }
    }

    /// Inclusive overlap test (`|dx| <= w_sum && |dy| <= h_sum`), matching the
    /// melee collision judge at `0x43e82`.
    pub fn overlaps_inclusive(&self, other: &BoundingBox) -> bool {
        self.left <= other.right
            && self.right >= other.left
            && self.top <= other.bottom
            && self.bottom >= other.top
    }

    /// Strict overlap test (`|dx| < w_sum && |dy| < h_sum`), matching the
    /// projectile collision checks at `0x430b6`, `0x43144`, and `0x431c6`.
    pub fn overlaps_strict(&self, other: &BoundingBox) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }

    /// Map this bitmap-space box onto the `384x224` visible screen rectangle
    /// `(x0, y0, x1, y1)` (inclusive pixel bounds before screen clamping).
    pub fn screen_rect(&self, flip_screen: bool) -> (i32, i32, i32, i32) {
        let x0 = self.left - VISAREA_X as i32;
        let y0 = self.top - VISAREA_Y as i32;
        let x1 = self.right - VISAREA_X as i32;
        let y1 = self.bottom - VISAREA_Y as i32;
        if flip_screen {
            (
                (WIDTH as i32 - 1) - x1,
                (HEIGHT as i32 - 1) - y1,
                (WIDTH as i32 - 1) - x0,
                (HEIGHT as i32 - 1) - y0,
            )
        } else {
            (x0, y0, x1, y1)
        }
    }
}

/// Evaluate the 1987 two-stage melee collision judge (`0x43e82`):
/// returns `0` on a miss or when the defender is invulnerable (`+0xf` bit 3),
/// `2` when the attack overlaps only the outer loose hurtbox (`depth 2`), and
/// `1` when the attack overlaps both the outer loose and inner tight hurtboxes
/// (`depth 1`).
pub fn judge_collision_depth(
    attacker_hitbox: &BoundingBox,
    defender_loose: &BoundingBox,
    defender_tight: &BoundingBox,
    defender_invulnerable: bool,
) -> u8 {
    if defender_invulnerable {
        return 0;
    }
    if !attacker_hitbox.overlaps_inclusive(defender_loose) {
        return 0;
    }
    if !attacker_hitbox.overlaps_inclusive(defender_tight) {
        2
    } else {
        1
    }
}

/// Telemetry for one three-button attack group (`0x3b3ae` family: P1 punch
/// `0xff8690`, P1 kick `0xff8698`, P2 punch `0xff86a0`, P2 kick `0xff86a8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ButtonGroupTelemetry {
    /// True while any button in the group is held (`[state] == 0xff`).
    pub held: bool,
    /// Raw hold counter byte `[state + 3]` (`0..=0xff`).
    pub hold_counter: u8,
    /// Total frames the group has been held (`hold_counter + 1` while held, `0` when idle).
    pub hold_frames: u16,
    /// Strongest button held during the current press (`[state + 4]`: `0` none, `1` L, `2` M, `3` H).
    pub strongest_button: u8,
    /// Pad-equivalent strength currently latched at `[out]` (`0x40` L, `0xe0` M, `0xfe` H).
    pub latched_strength: u8,
    /// Buffered or mapped strength on the fighter entity (`+0x34` punch / level, `+0x35` air kick).
    pub buffered_strength: u8,
    /// True when a negative-edge release trigger fired on this frame or remains
    /// latched in `[out]`.
    pub release_triggered: bool,
}

/// Telemetry for one special-move command tracker (`0x40360..0x403b2`, `0x40ae4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandTrackerTelemetry {
    /// Tracker progress stage (`0` idle, `1` first input, `2` second input, `3` completed).
    pub stage: u8,
    /// Elapsed frames since stage 1 opened (`0..=0x12`).
    pub timer: u8,
    /// Completion flag (`0xff` when stage 3 is active, `0` otherwise).
    pub done: u8,
    /// Remaining frames in the 18-frame (`0x12`) command window (`0` when inactive).
    pub remaining_window: u8,
    /// True on the frame this special move actually starts (`move_id` `0x19` / `0x1a` / `0x1b`).
    pub triggered_this_frame: bool,
}

/// Telemetry for the walk-cycle and busy gate at `0x403d4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WalkGateState {
    /// True when the fighter is in forward walk (`move 2`) or back walk (`move 3`).
    pub in_walk_cycle: bool,
    /// True on the 8 of 10 walk frames (`state 0x91`, `st & 0xc9 != 0x80`) where
    /// `0x403d4` returns early and locks out crouch, jump, and special-move dispatch.
    pub walk_gated: bool,
    /// True when `0x403d4` allows full directional/attack dispatch on this frame.
    pub actionable: bool,
}

/// Per-fighter collision and input telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FighterTelemetry {
    pub active: bool,
    pub player_struct: u32,
    pub entity_offset: u16,
    pub center_x: i32,
    pub center_y: i32,
    pub facing_left: bool,
    pub state_byte: u8,
    pub move_id: u8,
    pub move_started_this_frame: bool,
    pub invulnerable: bool,
    pub pushbox: BoundingBox,
    pub loose_hurtbox: BoundingBox,
    pub tight_hurtbox: BoundingBox,
    pub attack_active: bool,
    pub attack_connected_this_frame: bool,
    pub spawned_projectile_this_frame: bool,
    pub projectile_impacted_this_frame: bool,
    pub attack_hitbox: Option<BoundingBox>,
    pub walk_gate: WalkGateState,
    pub punch_latch: ButtonGroupTelemetry,
    pub kick_latch: ButtonGroupTelemetry,
    pub hadoken: CommandTrackerTelemetry,
    pub shoryuken: CommandTrackerTelemetry,
    pub tatsumaki: CommandTrackerTelemetry,
}

/// Telemetry for an active colliding projectile / effect slot (`0x100 + 0x40 * k`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProjectileTelemetry {
    pub slot_offset: u16,
    pub state: u8,
    pub collision_flags: u8,
    pub owner_struct: u32,
    pub target_struct: u32,
    pub hitbox: BoundingBox,
}

/// Complete per-frame diagnostic telemetry snapshot across the fighters,
/// all active projectiles, and the 60-cell rolling frame meter (`#92` `O1`, `#93` `O2`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagnosticTelemetry {
    pub flip_screen: bool,
    pub p1: FighterTelemetry,
    pub p2: FighterTelemetry,
    pub ally2: Option<FighterTelemetry>,
    /// Show the second ally's input panel when a human controls them.
    pub show_ally2_inputs: bool,
    pub projectiles: Vec<ProjectileTelemetry>,
    pub frame_meter: FrameMeterState,
}

struct MemView<'a> {
    rom: &'a [u8],
    /// The extended data bank (ADR-0014), as the machine's bus reads it:
    /// a boss's 1987 player table, streams and frame records.
    bank: Option<&'a crate::data_bank::DataBank>,
    work: &'a [u8],
    obj: &'a [u8],
}

/// Whether `a` holds move data the overlay can read: the program ROM's, or
/// the extended data bank's (a boss's bank table, ADR-0014).
fn readable(a: u32) -> bool {
    a < 0x06_0000 || crate::data_bank::in_bank(a)
}

impl MemView<'_> {
    fn rb(&self, a: u32) -> u8 {
        let a = a & 0xff_ffff;
        match a {
            0..=0x05_ffff => self.rom.get(a as usize).copied().unwrap_or(0xff),
            crate::data_bank::DATA_BANK_BASE..=crate::data_bank::DATA_BANK_END => self.bank.map_or(0, |b| b.rb(a)),
            0xff_8000..=0xff_dfff => self
                .work
                .get((a - WORK_BASE) as usize)
                .copied()
                .unwrap_or(0),
            0xff_e000..=0xff_ffff => self.obj.get((a - OBJ_BASE) as usize).copied().unwrap_or(0),
            _ => 0,
        }
    }

    fn rw(&self, a: u32) -> u16 {
        u16::from_be_bytes([self.rb(a), self.rb(a + 1)])
    }

    fn rl(&self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | (self.rw(a + 2) as u32)
    }

    fn eb(&self, e: u16, off: u32) -> u8 {
        self.rb(OBJ_BASE + e as u32 + off)
    }

    fn ew(&self, e: u16, off: u32) -> u16 {
        self.rw(OBJ_BASE + e as u32 + off)
    }

    fn el(&self, e: u16, off: u32) -> u32 {
        self.rl(OBJ_BASE + e as u32 + off)
    }

    fn is_big(&self, e: u16) -> bool {
        (self.ew(e, 2) as i16) >= 0x400
    }

    fn half_extent(n: u8, big: bool) -> u16 {
        let cell = if big { 0x20 } else { 0x10 };
        ((n as u16).wrapping_mul(cell).wrapping_add(cell)) >> 1
    }

    fn center_x(&self, e: u16) -> u16 {
        self.ew(e, 6)
            .wrapping_add(Self::half_extent(self.eb(e, 0xd), self.is_big(e)))
    }

    fn center_y(&self, e: u16) -> u16 {
        self.ew(e, 4)
            .wrapping_add(Self::half_extent(self.eb(e, 0xc), self.is_big(e)))
    }
}

fn sext8(b: u8) -> u16 {
    b as i8 as i16 as u16
}

fn is_punch_attack_move(mv: u8) -> bool {
    matches!(mv, 0x0d..=0x13 | 0x19 | 0x1a | 0x1c..=0x1e)
}

fn is_kick_attack_move(mv: u8) -> bool {
    matches!(mv, 0x14..=0x18 | 0x1b)
}

/// Whether move `mv` of the fighter on `ps` is an attack of the kick group
/// (`kick`) or the punch group: the 1987 ids, and the special slots by the
/// button their command takes, which a boss's bank table may change
/// (ADR-0014, [`crate::data_bank::special_command`]; Geki's Teleport Strike
/// is a punch in the Tatsumaki's slot).
fn is_button_attack_move(mem: &MemView<'_>, ps: u32, mv: u8, kick: bool) -> bool {
    if crate::data_bank::SPECIAL_SLOTS.contains(&mv) {
        let table = mem.rl(ps + 0x12);
        return crate::data_bank::special_command(|a| mem.rb(a), table, mv).1 == kick;
    }
    if kick {
        is_kick_attack_move(mv)
    } else {
        is_punch_attack_move(mv)
    }
}

fn move_started_this_frame(
    mem: &MemView<'_>,
    prev_mem: Option<&MemView<'_>>,
    ps: u32,
    e: u16,
    active: bool,
) -> bool {
    if !active {
        return false;
    }
    let mv = mem.eb(e, 0x0a);
    let table = mem.rl(ps + 0x12);
    if !readable(table) {
        return false;
    }
    let hdr = table.wrapping_add(mv as u32 * 0x30);
    let at_header = mem.el(e, 0x2c) == mem.rl(hdr + 0x2c)
        && mem.eb(e, 0x33) == mem.rb(hdr + 0x0e)
        && mem.eb(e, 0x0b) == mem.rb(hdr + 0x0b);
    if !at_header {
        return false;
    }
    if let Some(pm) = prev_mem {
        let e_prev = pm.rl(ps + 0x0e) as u16;
        if FIGHTER_SLOTS.contains(&e_prev)
            && pm.eb(e_prev, 8) != 0
            && pm.eb(e_prev, 0x0a) == mv
            && pm.el(e_prev, 0x2c) == mem.rl(hdr + 0x2c)
            && pm.eb(e_prev, 0x33) == mem.rb(hdr + 0x0e)
            && pm.eb(e_prev, 0x0b) == mem.rb(hdr + 0x0b)
            && (pm.eb(e_prev, 9) & 0x80 != 0)
        {
            return false;
        }
    }
    true
}

fn extract_button_group(
    mem: &MemView<'_>,
    prev_mem: Option<&MemView<'_>>,
    group_addr: u32,
    out_addr: u32,
    active: bool,
    ps: u32,
    input_ps: u32,
    e: u16,
    is_punch: bool,
    started_this_frame: bool,
) -> ButtonGroupTelemetry {
    let held = mem.rb(group_addr) != 0;
    let hold_counter = mem.rb(group_addr + 3);
    let hold_frames = if held { hold_counter as u16 + 1 } else { 0 };
    let strongest_button = mem.rb(group_addr + 4);
    let latched_strength = mem.rb(out_addr);
    let buffered_strength = if active {
        mem.eb(e, if is_punch { 0x34 } else { 0x35 })
    } else {
        0
    };
    let release_triggered = if let Some(pm) = prev_mem {
        // Exact negative-edge transition in 0x3b3ae (`LatchPath::Release { saturated: false }`):
        // group was held (`[group_addr] == 0xff`) and not saturated (`[group_addr + 3] != 0xff`)
        // on the previous frame and is now released (`[group_addr] == 0`), or its output
        // latch byte is still pending in `[out_addr]`.
        let released_now = pm.rb(group_addr) != 0 && pm.rb(group_addr + 3) != 0xff && !held;
        latched_strength != 0 || released_now
    } else {
        let st = if active { mem.eb(e, 9) } else { 0 };
        let mv = if active { mem.eb(e, 0xa) } else { 0 };
        let started_group_attack = active
            && ps == input_ps
            && (st & 0xc0 == 0xc0)
            && started_this_frame
            && is_button_attack_move(mem, ps, mv, !is_punch);
        latched_strength != 0 || started_group_attack
    };
    ButtonGroupTelemetry {
        held,
        hold_counter,
        hold_frames,
        strongest_button,
        latched_strength,
        buffered_strength,
        release_triggered,
    }
}

fn extract_tracker(
    mem: &MemView<'_>,
    prev_mem: Option<&MemView<'_>>,
    tracker_addr: u32,
    active: bool,
    ps: u32,
    input_ps: u32,
    st: u8,
    mv: u8,
    started_this_frame: bool,
    special_mv: u8,
) -> CommandTrackerTelemetry {
    let mut stage = mem.rb(tracker_addr);
    let mut timer = mem.rb(tracker_addr + 1);
    let mut done = mem.rb(tracker_addr + 2);
    let triggered_this_frame =
        active && ps == input_ps && (st & 0xc0 == 0xc0) && started_this_frame && mv == special_mv;

    // On the exact frame a special move triggers, 0x406ac / 0x407d0 (Hadoken &
    // Tatsumaki) or 0x40918 (Shoryuken) clears the tracker's timer and done
    // bytes after `track_commands` ran. Preserve the completed stage/done/window
    // from the pre-consumption state on that trigger frame.
    if triggered_this_frame && done == 0 {
        stage = 3;
        done = 0xff;
        if let Some(pm) = prev_mem {
            timer = pm.rb(tracker_addr + 1).saturating_add(1).min(0x12);
        }
    }

    // Note: when Shoryuken fires (0x40918), only timer (+1) and done (+2) are
    // cleared while stage (+0) stays 3 with done == 0. An open command window
    // exists only while stage is 1..=2 or stage == 3 with done != 0.
    let window_open = matches!(stage, 1 | 2) || (stage == 3 && done != 0);
    let remaining_window = if window_open {
        0x12u8.saturating_sub(timer)
    } else {
        0
    };
    CommandTrackerTelemetry {
        stage,
        timer,
        done,
        remaining_window,
        triggered_this_frame,
    }
}

fn did_attack_connect_this_frame(
    mem: &MemView<'_>,
    prev_mem: Option<&MemView<'_>>,
    ps: u32,
    e: u16,
    started_this_frame: bool,
) -> bool {
    let flags = mem.eb(e, 0x0f);
    if flags & 0x80 != 0 {
        return false;
    }
    let st = mem.eb(e, 9);
    let aw = mem.eb(e, 0x25);
    let ah = mem.eb(e, 0x23);
    if (st & 0xc0 != 0xc0) || aw == 0 || ah == 0 {
        return false;
    }
    // Case 1: first frame of a move (`0x42ad0` branch `eb(me, 8) == 1` -> `judge`).
    if started_this_frame {
        let mv = mem.eb(e, 0x0a);
        let table = mem.rl(ps + 0x12);
        if readable(table) {
            let hdr = table.wrapping_add(mv as u32 * 0x30);
            return mem.rb(hdr + 0x0f) & 0x80 != 0;
        }
        return false;
    }

    let cur_stream = mem.el(e, 0x2c);
    // Blocked medium/heavy kick recoil targets set by `hit_redirect` (`0x460b8` / `0x46164` / `0x46184`).
    let is_recoil_target = matches!(
        cur_stream,
        0x0f4a | 0x10b4 | 0x1094 | 0x3028 | 0x3192 | 0x3172
    );

    if let Some(pm) = prev_mem {
        let e_prev = pm.rl(ps + 0x0e) as u16;
        if FIGHTER_SLOTS.contains(&e_prev) {
            let prev_stream = pm.el(e_prev, 0x2c);
            if cur_stream == prev_stream {
                // Case 2: held (`+0x33`) or repeated (`+0x0b`) frame — `judge` cleared bit 7
                // on this frame iff bit 7 was set at the end of the previous frame.
                return pm.eb(e_prev, 0x0f) & 0x80 != 0;
            }
            // Case 3: `load_frame` advanced `+0x2c` (or `hit_redirect` jumped to recoil)
            // on this frame, and `judge` immediately cleared bit 7.
            if is_recoil_target {
                return true;
            }
            let a1 = cur_stream.wrapping_sub(0x20);
            return readable(a1)
                && (mem.rb(a1 + 0x1a) & 0x80 != 0)
                && mem.eb(e, 0x33) == mem.eb(e, 0x0e);
        }
    }

    // Fallback when no previous frame snapshot is available:
    let a1 = cur_stream.wrapping_sub(0x20);
    readable(a1) && (mem.rb(a1 + 0x1a) & 0x80 != 0) && mem.eb(e, 0x33) == mem.eb(e, 0x0e)
}

fn extract_fighter(
    mem: &MemView<'_>,
    prev_mem: Option<&MemView<'_>>,
    ps: u32,
    input_ps: u32,
    punch_group: u32,
    kick_group: u32,
) -> FighterTelemetry {
    let d1 = mem.rl(ps + 0x0e);
    let e = d1 as u16;
    let valid_slot = FIGHTER_SLOTS.contains(&e);
    let active = valid_slot && mem.eb(e, 8) != 0;

    if !active {
        return FighterTelemetry {
            active: false,
            player_struct: ps,
            entity_offset: e,
            punch_latch: extract_button_group(
                mem,
                prev_mem,
                punch_group,
                input_ps,
                false,
                ps,
                input_ps,
                e,
                true,
                false,
            ),
            kick_latch: extract_button_group(
                mem,
                prev_mem,
                kick_group,
                input_ps + 1,
                false,
                ps,
                input_ps,
                e,
                false,
                false,
            ),
            hadoken: extract_tracker(
                mem,
                prev_mem,
                input_ps + 0x1c,
                false,
                ps,
                input_ps,
                0,
                0,
                false,
                0x1a,
            ),
            shoryuken: extract_tracker(
                mem,
                prev_mem,
                input_ps + 0x19,
                false,
                ps,
                input_ps,
                0,
                0,
                false,
                0x19,
            ),
            tatsumaki: extract_tracker(
                mem,
                prev_mem,
                input_ps + 0x16,
                false,
                ps,
                input_ps,
                0,
                0,
                false,
                0x1b,
            ),
            ..FighterTelemetry::default()
        };
    }

    let cx = mem.center_x(e);
    let cy = mem.center_y(e);
    let facing_left = (mem.eb(e, 0x37) as i8) >= 4;
    let st = mem.eb(e, 9);
    let mv = mem.eb(e, 0xa);
    let started_this_frame = move_started_this_frame(mem, prev_mem, ps, e, true);
    let flags = mem.eb(e, 0xf);
    let invulnerable = flags & 0x08 != 0;

    // Pushbox (0x43d7c / 0x43c9a / 0x43c18)
    let (left_half, right_half) = if facing_left {
        (mem.eb(e, 0x18) as i32, mem.eb(e, 0x19) as i32)
    } else {
        (mem.eb(e, 0x19) as i32, mem.eb(e, 0x18) as i32)
    };
    let top_half = mem.eb(e, 0x16) as i32;
    let bottom_half = mem.eb(e, 0x17) as i32;
    let cx_i = cx as i32;
    let cy_i = cy as i32;
    let pushbox = BoundingBox::from_edges(
        cx_i,
        cy_i,
        cx_i - left_half,
        cy_i - top_half,
        cx_i + right_half,
        cy_i + bottom_half,
    );

    // Outer loose hurtbox (depth 2, +0x1e..+0x21)
    let dx_loose = cx.wrapping_add(sext8(mem.eb(e, 0x20))) as i32;
    let dy_loose = cy.wrapping_sub(sext8(mem.eb(e, 0x1e))) as i32;
    let loose_hurtbox = BoundingBox::from_center_half(
        dx_loose,
        dy_loose,
        mem.eb(e, 0x21) as i32,
        mem.eb(e, 0x1f) as i32,
    );

    // Inner tight hurtbox (depth 1, +0x1a..+0x1d)
    let dx_tight = cx.wrapping_add(sext8(mem.eb(e, 0x1c))) as i32;
    let dy_tight = cy.wrapping_sub(sext8(mem.eb(e, 0x1a))) as i32;
    let tight_hurtbox = BoundingBox::from_center_half(
        dx_tight,
        dy_tight,
        mem.eb(e, 0x1d) as i32,
        mem.eb(e, 0x1b) as i32,
    );

    // Active melee attack hitbox (+0xf bit 7, or consumed on this exact frame by judge)
    let attack_active = flags & 0x80 != 0;
    let aw = mem.eb(e, 0x25) as i32;
    let ah = mem.eb(e, 0x23) as i32;
    let attack_connected_this_frame =
        did_attack_connect_this_frame(mem, prev_mem, ps, e, started_this_frame);

    let attack_hitbox = if (attack_active || attack_connected_this_frame) && aw > 0 && ah > 0 {
        let ax = cx.wrapping_add(sext8(mem.eb(e, 0x24))) as i32;
        let ay = cy.wrapping_sub(sext8(mem.eb(e, 0x22))) as i32;
        Some(BoundingBox::from_center_half(ax, ay, aw, ah))
    } else {
        None
    };

    // Walk-cycle gate (0x403d4)
    let d2 = st & 0xc9;
    let in_walk_cycle = mv == 2 || mv == 3;
    let actionable =
        (d2 == 0 || mv == 0 || mv == 0x0c || d2 == 8 || d2 == 9 || d2 == 1 || d2 == 0x80)
            && st != 0x80;
    let walk_gated = in_walk_cycle && !actionable;

    FighterTelemetry {
        active: true,
        player_struct: ps,
        entity_offset: e,
        center_x: cx_i,
        center_y: cy_i,
        facing_left,
        state_byte: st,
        move_id: mv,
        move_started_this_frame: started_this_frame,
        invulnerable,
        pushbox,
        loose_hurtbox,
        tight_hurtbox,
        attack_active,
        attack_connected_this_frame,
        spawned_projectile_this_frame: false,
        projectile_impacted_this_frame: false,
        attack_hitbox,
        walk_gate: WalkGateState {
            in_walk_cycle,
            walk_gated,
            actionable,
        },
        punch_latch: extract_button_group(
            mem,
            prev_mem,
            punch_group,
            input_ps,
            true,
            ps,
            input_ps,
            e,
            true,
            started_this_frame,
        ),
        kick_latch: extract_button_group(
            mem,
            prev_mem,
            kick_group,
            input_ps + 1,
            true,
            ps,
            input_ps,
            e,
            false,
            started_this_frame,
        ),
        hadoken: extract_tracker(
            mem,
            prev_mem,
            input_ps + 0x1c,
            true,
            ps,
            input_ps,
            st,
            mv,
            started_this_frame,
            0x1a,
        ),
        shoryuken: extract_tracker(
            mem,
            prev_mem,
            input_ps + 0x19,
            true,
            ps,
            input_ps,
            st,
            mv,
            started_this_frame,
            0x19,
        ),
        tatsumaki: extract_tracker(
            mem,
            prev_mem,
            input_ps + 0x16,
            true,
            ps,
            input_ps,
            st,
            mv,
            started_this_frame,
            0x1b,
        ),
    }
}

/// Extract the complete diagnostic telemetry snapshot from read-only references
/// to `rom`, `work_ram`, `objectram`, and `flip_screen`.
pub fn extract_diagnostic_telemetry(
    rom: &[u8],
    work: &[u8],
    obj: &[u8],
    flip_screen: bool,
) -> DiagnosticTelemetry {
    extract_diagnostic_telemetry_with_prev(rom, work, obj, None, flip_screen)
}

/// Extract the complete diagnostic telemetry snapshot using both the current
/// frame's `(work, obj)` and an optional pre-step `(prev_work, prev_obj)` snapshot.
pub fn extract_diagnostic_telemetry_with_prev(
    rom: &[u8],
    work: &[u8],
    obj: &[u8],
    prev: Option<(&[u8], &[u8])>,
    flip_screen: bool,
) -> DiagnosticTelemetry {
    extract_diagnostic_telemetry_banked(rom, None, work, obj, prev, flip_screen)
}

/// [`extract_diagnostic_telemetry_with_prev`] for a machine holding the
/// extended data bank (ADR-0014), whose bosses' tables and streams it reads
/// as the bus does.
pub fn extract_diagnostic_telemetry_banked(
    rom: &[u8],
    bank: Option<&crate::data_bank::DataBank>,
    work: &[u8],
    obj: &[u8],
    prev: Option<(&[u8], &[u8])>,
    flip_screen: bool,
) -> DiagnosticTelemetry {
    let mem = MemView { rom, bank, work, obj };
    let prev_view = prev.map(|(pw, po)| MemView {
        rom,
        bank,
        work: pw,
        obj: po,
    });
    let prev_mem = prev_view.as_ref();
    let game_mode = mem.rb(0xffc002);

    let p1_struct = if game_mode == 4 {
        CPU_STRUCT
    } else {
        P1_STRUCT
    };
    let p2_struct = if game_mode == 2 {
        CPU_STRUCT
    } else {
        P2_STRUCT
    };

    let mut p1 = extract_fighter(&mem, prev_mem, p1_struct, P1_STRUCT, 0xff8690, 0xff8698);
    let mut p2 = extract_fighter(&mem, prev_mem, p2_struct, P2_STRUCT, 0xff86a0, 0xff86a8);
    let ally2 = (game_mode == 2 && mem.rw(P2_STRUCT + 0x10) == DRAMATIC_ALLY2)
        .then(|| extract_fighter(&mem, prev_mem, P2_STRUCT, P2_STRUCT, 0xff86a0, 0xff86a8));

    // Check whether any projectile slot transitioned into the explosion template
    // `0x186fc` or shuriken-hit template `0x18944` (`state == 1`) on this frame
    // (`0x433ba` / `0x43476` / `0x43490` / `0x43652` / `0x43348`).
    let explosion_stream = mem.rl(0x186fc + 0x10);
    let shuriken_hit_stream = mem.rl(0x18944 + 0x10);
    let is_projectile_explosion = |s: u16| {
        mem.eb(s, 8) == 1
            && (mem.el(s, 0x10) == explosion_stream || mem.el(s, 0x10) == shuriken_hit_stream)
    };
    let explosion_started_this_frame =
        (0..12u16).any(|k| is_projectile_explosion(0x100 + 0x40 * k));
    let any_cancelled_by_other = prev_mem.is_some_and(|pm| {
        explosion_started_this_frame
            && (0..12u16).any(|k| {
                let s = 0x100 + 0x40 * k;
                mem.eb(s, 8) == 0 && pm.eb(s, 8) != 0 && (pm.eb(s, 0x0e) & 0x20) != 0
            })
    });

    let mut projectiles = Vec::new();
    for k in 0..12u16 {
        let s = 0x100 + 0x40 * k;
        let state = mem.eb(s, 8);
        let collision_flags = mem.eb(s, 0x0e);
        if state != 0 && (collision_flags & 0xe0) != 0 {
            let owner_struct = mem.el(s, 0x20);
            let target_struct = mem.el(s, 0x24);
            let spawned_now =
                prev_mem.is_some_and(|pm| pm.eb(s, 8) == 0 || (pm.eb(s, 0x0e) & 0xe0) == 0);
            if spawned_now {
                if owner_struct == p1.player_struct {
                    p1.spawned_projectile_this_frame = true;
                } else if owner_struct == p2.player_struct {
                    p2.spawned_projectile_this_frame = true;
                }
            }
            let half = if mem.ew(s, 2) & 0x400 != 0 { 0x10 } else { 8 };
            let cx = mem.ew(s, 6).wrapping_add(half) as i32;
            let cy = mem.ew(s, 4).wrapping_add(half) as i32;
            let half_h = mem.eb(s, 0x0c) as i32;
            let half_w = mem.eb(s, 0x0d) as i32;
            projectiles.push(ProjectileTelemetry {
                slot_offset: s,
                state,
                collision_flags,
                owner_struct,
                target_struct,
                hitbox: BoundingBox::from_center_half(cx, cy, half_w, half_h),
            });
        } else if let Some(pm) = prev_mem {
            // Preserve the projectile's hitbox on the exact frame it impacts a
            // fighter (`0x433ba`), an attack box (`0x43490`), or cancels against
            // another projectile (`0x43476`), including spawn-frame impacts.
            let prev_state = pm.eb(s, 8);
            let prev_flags = pm.eb(s, 0x0e);
            let became_explosion = is_projectile_explosion(s);
            let cancelled_by_other = state == 0
                && prev_state != 0
                && (prev_flags & 0x20) != 0
                && explosion_started_this_frame;
            if became_explosion || cancelled_by_other {
                let src = if became_explosion { &mem } else { pm };
                let owner_struct = src.el(s, 0x20);
                let target_struct = src.el(s, 0x24);
                let spawned_and_impacted = prev_state == 0 || (prev_flags & 0xe0) == 0;
                if spawned_and_impacted {
                    if owner_struct == p1.player_struct {
                        p1.spawned_projectile_this_frame = true;
                    } else if owner_struct == p2.player_struct {
                        p2.spawned_projectile_this_frame = true;
                    }
                }
                if became_explosion && !any_cancelled_by_other {
                    if target_struct == p1.player_struct
                        && (p1.move_id >= 0x30 || (p1.state_byte & 0x20) != 0)
                    {
                        p1.projectile_impacted_this_frame = true;
                    } else if target_struct == p2.player_struct
                        && (p2.move_id >= 0x30 || (p2.state_byte & 0x20) != 0)
                    {
                        p2.projectile_impacted_this_frame = true;
                    }
                }
                let half = if src.ew(s, 2) & 0x400 != 0 { 0x10 } else { 8 };
                let cx = src.ew(s, 6).wrapping_add(half) as i32;
                let cy = src.ew(s, 4).wrapping_add(half) as i32;
                let (half_h, half_w, flags, eff_state) =
                    if prev_state != 0 && (prev_flags & 0xe0) != 0 {
                        let (hh, hw) = if became_explosion && prev_state == 2 && pm.eb(s, 9) == 0 {
                            let mut a0 = pm.el(s, 0x10);
                            if pm.rb(a0) == 0xfe {
                                a0 = pm.rl(a0 + 2);
                            }
                            (pm.rb(a0 + 4) as i32, pm.rb(a0 + 5) as i32)
                        } else {
                            (pm.eb(s, 0x0c) as i32, pm.eb(s, 0x0d) as i32)
                        };
                        (hh, hw, prev_flags, prev_state)
                    } else {
                        // Spawned and impacted on the same frame (template 0x186dc / 0x1873c).
                        (0x0c, 0x0c, 0xe0, 2)
                    };
                projectiles.push(ProjectileTelemetry {
                    slot_offset: s,
                    state: eff_state,
                    collision_flags: flags,
                    owner_struct,
                    target_struct,
                    hitbox: BoundingBox::from_center_half(cx, cy, half_w, half_h),
                });
            }
        }
    }

    DiagnosticTelemetry {
        flip_screen,
        p1,
        p2,
        ally2,
        show_ally2_inputs: false,
        projectiles,
        frame_meter: FrameMeterState::default(),
    }
}

/// Composite the collision, input, and 60-cell frame meter diagnostic overlay
/// over `base_frame` into a fresh [`Frame`] when `enabled` is `true`, or return
/// a copy of `base_frame` untouched when `enabled` is `false`.
pub fn render_diagnostic_overlay(
    base_frame: &Frame,
    telemetry: &DiagnosticTelemetry,
    enabled: bool,
) -> Frame {
    render_diagnostic_overlay_parts(base_frame, telemetry, enabled, enabled)
}

/// The diagnostic overlay in parts, for the front end's `OVERLAYS` row:
/// `boxes` draws the fighters' and projectiles' boxes, `meter` the input
/// panels and the frame meter. `base_frame` itself is left untouched.
pub fn render_diagnostic_overlay_parts(
    base_frame: &Frame,
    telemetry: &DiagnosticTelemetry,
    boxes: bool,
    meter: bool,
) -> Frame {
    let mut out = Frame {
        argb: base_frame.argb.clone(),
    };
    if boxes {
        draw_boxes(&mut out, telemetry);
    }
    if meter {
        // 3. Draw live per-player input & command-tracker telemetry HUD panels.
        let (p1, p2) = (telemetry_box(Side::P1), telemetry_box(Side::P2));
        draw_player_hud(&mut out.argb, p1.x as i32 + 2, p1.y as i32 + 2, "P1", &telemetry.p1);
        let right = telemetry.ally2.as_ref().filter(|_| telemetry.show_ally2_inputs)
            .unwrap_or(&telemetry.p2);
        draw_player_hud(&mut out.argb, p2.x as i32 + 2, p2.y as i32 + 2, "P2", right);

        // 4. Draw 60-cell rolling frame meter and signed frame advantage for P1 and P2 (`#93`, `O2`).
        draw_frame_meter(&mut out.argb, &telemetry.frame_meter);
    }
    out
}

/// Steps 1 and 2 of the overlay: fighter and projectile boxes.
fn draw_boxes(out: &mut Frame, telemetry: &DiagnosticTelemetry) {
    let flip = telemetry.flip_screen;

    // 1. Draw fighter bounding boxes (pushbox -> loose hurtbox -> tight hurtbox -> invuln -> melee hitbox).
    for fighter in [&telemetry.p1, &telemetry.p2].into_iter().chain(telemetry.ally2.as_ref()) {
        if !fighter.active {
            continue;
        }
        draw_box(
            &mut out.argb,
            fighter.pushbox.screen_rect(flip),
            OverlayColors::PUSHBOX_FILL,
            OverlayColors::PUSHBOX_BORDER,
            48,
        );
        draw_box(
            &mut out.argb,
            fighter.loose_hurtbox.screen_rect(flip),
            OverlayColors::LOOSE_HURTBOX_FILL,
            OverlayColors::LOOSE_HURTBOX_BORDER,
            56,
        );
        draw_box(
            &mut out.argb,
            fighter.tight_hurtbox.screen_rect(flip),
            OverlayColors::TIGHT_HURTBOX_FILL,
            OverlayColors::TIGHT_HURTBOX_BORDER,
            64,
        );
        if fighter.invulnerable {
            draw_box(
                &mut out.argb,
                fighter.loose_hurtbox.screen_rect(flip),
                OverlayColors::INVULNERABLE_FILL,
                OverlayColors::INVULNERABLE_BORDER,
                80,
            );
        }
        if let Some(atk) = &fighter.attack_hitbox {
            draw_box(
                &mut out.argb,
                atk.screen_rect(flip),
                OverlayColors::MELEE_HITBOX_FILL,
                OverlayColors::MELEE_HITBOX_BORDER,
                88,
            );
        }
    }

    // 2. Draw active projectile hitboxes.
    for proj in &telemetry.projectiles {
        draw_box(
            &mut out.argb,
            proj.hitbox.screen_rect(flip),
            OverlayColors::PROJECTILE_HITBOX_FILL,
            OverlayColors::PROJECTILE_HITBOX_BORDER,
            88,
        );
    }
}

fn blend_argb(dst: u32, src: u32, alpha: u32) -> u32 {
    let inv = 256 - alpha;
    let r = (((dst >> 16) & 0xff) * inv + ((src >> 16) & 0xff) * alpha) >> 8;
    let g = (((dst >> 8) & 0xff) * inv + ((src >> 8) & 0xff) * alpha) >> 8;
    let b = ((dst & 0xff) * inv + (src & 0xff) * alpha) >> 8;
    0xff00_0000 | (r << 16) | (g << 8) | b
}

fn draw_box(argb: &mut [u32], rect: (i32, i32, i32, i32), fill: u32, border: u32, alpha: u32) {
    let (x0, y0, x1, y1) = (
        rect.0.min(rect.2),
        rect.1.min(rect.3),
        rect.0.max(rect.2),
        rect.1.max(rect.3),
    );
    if x1 < 0 || y1 < 0 || x0 >= WIDTH as i32 || y0 >= HEIGHT as i32 {
        return;
    }
    let cx0 = x0.clamp(0, WIDTH as i32 - 1);
    let cy0 = y0.clamp(0, HEIGHT as i32 - 1);
    let cx1 = x1.clamp(0, WIDTH as i32 - 1);
    let cy1 = y1.clamp(0, HEIGHT as i32 - 1);

    for y in cy0..=cy1 {
        let on_y_edge = y == y0 || y == y1;
        let row = y as usize * WIDTH;
        for x in cx0..=cx1 {
            let on_x_edge = x == x0 || x == x1;
            let idx = row + x as usize;
            if on_x_edge || on_y_edge {
                argb[idx] = border;
            } else {
                argb[idx] = blend_argb(argb[idx], fill, alpha);
            }
        }
    }
}

fn fill_rect_alpha(argb: &mut [u32], x0: i32, y0: i32, w: i32, h: i32, color: u32, alpha: u32) {
    let cx0 = x0.clamp(0, WIDTH as i32);
    let cy0 = y0.clamp(0, HEIGHT as i32);
    let cx1 = (x0 + w).clamp(0, WIDTH as i32);
    let cy1 = (y0 + h).clamp(0, HEIGHT as i32);
    for y in cy0..cy1 {
        let row = y as usize * WIDTH;
        for x in cx0..cx1 {
            let idx = row + x as usize;
            argb[idx] = blend_argb(argb[idx], color, alpha);
        }
    }
}

fn format_advantage(adv: Option<i16>) -> (String, u32) {
    match adv {
        Some(v) if v > 0 => (format!("+{:02}", v.min(99)), OverlayColors::HUD_ACTIVE),
        Some(v) if v < 0 => (
            format!("-{:02}", v.unsigned_abs().min(99)),
            OverlayColors::HUD_WARN,
        ),
        Some(_) => ("+00".to_string(), OverlayColors::HUD_TEXT),
        None => (" --".to_string(), OverlayColors::HUD_TEXT),
    }
}

fn draw_frame_meter_row(argb: &mut [u32], y: i32, label: &str, meter: &PlayerFrameMeter) {
    draw_text_alpha(argb, 6, y + 1, label, OverlayColors::HUD_TEXT, FRAME_METER_CELL_ALPHA);
    let (adv_str, adv_color) = format_advantage(meter.frame_advantage);
    draw_text_alpha(argb, 18, y + 1, &adv_str, adv_color, FRAME_METER_CELL_ALPHA);

    let cells_x0 = 34;
    for (idx, cell) in meter.cells.iter().copied().enumerate() {
        let cx = cells_x0 + (idx as i32) * 5;
        fill_rect_alpha(argb, cx, y, 4, 6, cell.color(), FRAME_METER_CELL_ALPHA);
    }

    let cur = meter.current_cell();
    draw_text_alpha(argb, 338, y + 1, cur.label(), cur.color(), FRAME_METER_CELL_ALPHA);
}

fn draw_frame_meter(argb: &mut [u32], meter: &FrameMeterState) {
    let panel = frame_meter_panel();
    let panel_y = panel.y as i32;
    fill_rect_alpha(
        argb,
        panel.x as i32,
        panel_y,
        panel.w as i32,
        panel.h as i32,
        OverlayColors::HUD_BG,
        FRAME_METER_PANEL_ALPHA,
    );
    draw_frame_meter_row(argb, panel_y + 2, "P1", &meter.p1);
    draw_frame_meter_row(argb, panel_y + 10, "P2", &meter.p2);
}

fn tracker_sep(t: &CommandTrackerTelemetry) -> char {
    if t.triggered_this_frame {
        '!'
    } else if t.done == 0xff {
        'D'
    } else {
        '/'
    }
}

fn draw_player_hud(argb: &mut [u32], x: i32, y: i32, label: &str, f: &FighterTelemetry) {
    fill_rect_alpha(argb, x - 2, y - 2, 152, 22, OverlayColors::HUD_BG, 180);

    let gate_str = if f.walk_gate.walk_gated {
        "WLK:GATE"
    } else if f.walk_gate.in_walk_cycle {
        "WLK:OPEN"
    } else if f.walk_gate.actionable {
        "ACT:OPEN"
    } else {
        "ACT:BUSY"
    };
    let gate_color = if f.walk_gate.walk_gated {
        OverlayColors::HUD_WARN
    } else if f.walk_gate.actionable {
        OverlayColors::HUD_ACTIVE
    } else {
        OverlayColors::HUD_TEXT
    };

    let line1 = format!(
        "{label} {gate_str}{}",
        if f.invulnerable { " INV" } else { "" }
    );
    draw_text(argb, x, y, &line1, gate_color);

    let p_rel = if f.punch_latch.release_triggered {
        "R"
    } else {
        "-"
    };
    let k_rel = if f.kick_latch.release_triggered {
        "R"
    } else {
        "-"
    };
    let line2 = format!(
        "P:{:02}/{}{p_rel} K:{:02}/{}{k_rel}",
        f.punch_latch.hold_frames.min(99),
        f.punch_latch.strongest_button,
        f.kick_latch.hold_frames.min(99),
        f.kick_latch.strongest_button,
    );
    let latch_color = if f.punch_latch.release_triggered || f.kick_latch.release_triggered {
        OverlayColors::HUD_ACTIVE
    } else {
        OverlayColors::HUD_TEXT
    };
    draw_text(argb, x, y + 6, &line2, latch_color);

    let line3 = format!(
        "H:{}{}{:02} S:{}{}{:02} T:{}{}{:02}",
        f.hadoken.stage,
        tracker_sep(&f.hadoken),
        f.hadoken.remaining_window,
        f.shoryuken.stage,
        tracker_sep(&f.shoryuken),
        f.shoryuken.remaining_window,
        f.tatsumaki.stage,
        tracker_sep(&f.tatsumaki),
        f.tatsumaki.remaining_window,
    );
    let tracker_ready = f.hadoken.done == 0xff
        || f.shoryuken.done == 0xff
        || f.tatsumaki.done == 0xff
        || f.hadoken.triggered_this_frame
        || f.shoryuken.triggered_this_frame
        || f.tatsumaki.triggered_this_frame;
    let tracker_color = if tracker_ready {
        OverlayColors::HUD_ACTIVE
    } else {
        OverlayColors::HUD_TEXT
    };
    draw_text(argb, x, y + 12, &line3, tracker_color);
}

fn glyph_rows(c: char) -> [u8; 5] {
    match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b111, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        '!' => [0b010, 0b010, 0b010, 0b000, 0b010],
        _ => [0; 5],
    }
}

fn draw_text(argb: &mut [u32], mut x: i32, y: i32, text: &str, color: u32) {
    for ch in text.chars() {
        let rows = glyph_rows(ch);
        for (ry, bits) in rows.into_iter().enumerate() {
            let py = y + ry as i32;
            if !(0..HEIGHT as i32).contains(&py) {
                continue;
            }
            for rx in 0..3 {
                if (bits >> (2 - rx)) & 1 != 0 {
                    let px = x + rx;
                    if (0..WIDTH as i32).contains(&px) {
                        argb[py as usize * WIDTH + px as usize] = color;
                    }
                }
            }
        }
        x += 4;
    }
}

/// `text` blended over the frame at `alpha` out of 256, not drawn solid.
fn draw_text_alpha(argb: &mut [u32], mut x: i32, y: i32, text: &str, color: u32, alpha: u32) {
    for ch in text.chars() {
        let rows = glyph_rows(ch);
        for (ry, bits) in rows.into_iter().enumerate() {
            let py = y + ry as i32;
            if !(0..HEIGHT as i32).contains(&py) {
                continue;
            }
            for rx in 0..3 {
                if (bits >> (2 - rx)) & 1 != 0 {
                    let px = x + rx;
                    if (0..WIDTH as i32).contains(&px) {
                        let idx = py as usize * WIDTH + px as usize;
                        argb[idx] = blend_argb(argb[idx], color, alpha);
                    }
                }
            }
        }
        x += 4;
    }
}
