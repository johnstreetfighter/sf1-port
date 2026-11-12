//! Authoritative era state for the translated fight loop (ADR-0015).

use sf_video::callouts::{CalloutKind, Side};
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

use crate::config::MatchConfig;
use crate::data_bank::{ALPHA_SUPER_SECOND_SLOT, ALPHA_SUPER_THIRD_SLOT};
use crate::era::{EraFamily, GaugeMode, Loadout, Revision, StylePick, STOCK_AT};
use crate::fight_reader::player_struct;
use crate::fighter::{Bus, CPU_STRUCT, GAME_MODE};
use crate::alpha::Stance;
use crate::moves::{ButtonClass, CHARGE_FRAMES, Dir, MoveKind};
use crate::roster::{PlayMode, TournamentFighter};

pub const SUB_HP_PER_POINT: u16 = 20;
pub const FULL_SUB_HP: u16 = 48 * SUB_HP_PER_POINT;
pub use crate::data_bank::{SF2_SPECIAL_SLOT as SF2_EXTRA_SPECIAL, SF2_SUPER_SLOT as SF2_SUPER};
pub const SUPER_FREEZE_FRAMES: u8 = 28;
const PROJECTILE_SLOTS: usize = 12;
const GUARD_POWER_STANDARD: u8 = 60;
const GUARD_POWER_X_ISM: u8 = 72;
const GUARD_CRUSH_FRAMES: u8 = 32;
const GUARD_CRUSH_CAPACITY_LOSS: u8 = 12;
const GUARD_RECOVERY_DELAY: u8 = 60;
const GUARD_RECOVERY_INTERVAL: u8 = 8;

pub(crate) const DOUBLE_QUARTER_CIRCLE: &[Dir] = &[
    Dir::Down, Dir::DownForward, Dir::Forward, Dir::Down, Dir::DownForward, Dir::Forward,
];
const DOUBLE_QUARTER_CIRCLE_BACK: &[Dir] = &[
    Dir::Down, Dir::DownBack, Dir::Back, Dir::Down, Dir::DownBack, Dir::Back,
];
const SHORT_DOUBLE_QUARTER_CIRCLE: &[Dir] = &[
    Dir::Down, Dir::DownForward, Dir::Forward, Dir::Down, Dir::DownForward,
];
const HALF_CIRCLE: &[Dir] = &[Dir::Back, Dir::DownBack, Dir::Down, Dir::DownForward, Dir::Forward];
const TIGER_KNEE: &[Dir] = &[Dir::Down, Dir::DownForward, Dir::Forward, Dir::UpForward];
const ALPHA_COUNTER_MOTION: &[Dir] = &[Dir::Back, Dir::DownBack, Dir::Down];

mod alpha_moves;
pub(crate) mod sf3;
pub(crate) mod sf4;
pub(crate) mod sf5;
pub(crate) mod sf6;
pub(crate) mod what_if;
pub(crate) mod what_if_others;

pub(crate) use alpha_moves::AlphaMove;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpecialContext {
    Ground { canceling: bool },
    Airborne,
    /// A rapid-press special may interrupt the normal started by its first tap.
    MashCancel,
}

const ALPHA_MOTION_SLOTS: usize = 10;
const PUNCH_MASK: u8 = 0b000111;
const KICK_MASK: u8 = 0b111000;

const ALPHA_REVISIONS: &[Revision] = &[Revision::Sfz1, Revision::Sfz2, Revision::Sfz2A, Revision::Sfz3];
const ALPHA_3: &[Revision] = &[Revision::Sfz3];

const RYU_ALPHA_SUPERS: &[AlphaMove] = &[
    AlphaMove::super_combo(SF2_SUPER, "SHINKUU HADOKEN", "SHINKUU HADOKEN!", DOUBLE_QUARTER_CIRCLE, ButtonClass::Punch),
    AlphaMove::super_combo(ALPHA_SUPER_SECOND_SLOT, "METSU SHORYUKEN", "METSU SHORYUKEN!", SHORT_DOUBLE_QUARTER_CIRCLE, ButtonClass::Kick).revision(ALPHA_3).level_three(),
    AlphaMove::super_combo(ALPHA_SUPER_THIRD_SLOT, "SHINKUU TATSUMAKI", "SHINKUU TATSUMAKI!", DOUBLE_QUARTER_CIRCLE_BACK, ButtonClass::Kick),
    AlphaMove::overhead_normal(0x1c, "COLLARBONE BREAKER", ButtonClass::MediumPunch),
];

const KEN_ALPHA_SUPERS: &[AlphaMove] = &[
    AlphaMove::super_combo(SF2_SUPER, "SHORYUREPPA", "SHORYUREPPA!", SHORT_DOUBLE_QUARTER_CIRCLE, ButtonClass::Punch),
    AlphaMove::super_combo(ALPHA_SUPER_SECOND_SLOT, "SHINRYUKEN", "SHINRYUKEN!", DOUBLE_QUARTER_CIRCLE, ButtonClass::Kick),
    AlphaMove::super_combo(ALPHA_SUPER_THIRD_SLOT, "SHIPPU JINRAIKYAKU", "SHIPPU JINRAIKYAKU!", DOUBLE_QUARTER_CIRCLE_BACK, ButtonClass::Kick).revision(ALPHA_3).level_three(),
    AlphaMove::overhead_normal(0x1c, "INAZUMA KAKATO WARI", ButtonClass::MediumKick),
];

fn alpha_super_hit_limit(level: u8) -> u8 {
    match level {
        1 => 2,
        2 => 3,
        3 => 5,
        _ => 1,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MotionTracker {
    position: u8,
    remaining: u8,
    previous: u8,
    charge_frames: u8,
    spin_direction: i8,
    spin_last: u8,
}

crate::canonical::canonical_struct!(MotionTracker, "EraMotionTracker.v2", {
    position, remaining, previous, charge_frames, spin_direction, spin_last,
});

impl MotionTracker {
    fn step(&mut self, direction: u8, pattern: &[Dir]) {
        let Some(first) = pattern.first().copied() else { return };
        if pattern == alpha_moves::SPIN {
            self.step_spin(direction);
            return;
        }
        if first.is_charge() {
            if direction & first.bits() != 0 && (self.position <= 1 || self.remaining <= 1) {
                self.charge_frames = self.charge_frames.saturating_add(1);
                if self.charge_frames as usize >= CHARGE_FRAMES {
                    self.position = 1;
                    self.remaining = 32;
                }
                self.previous = direction;
                return;
            }
            self.charge_frames = 0;
        }
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining == 0 {
            self.position = 0;
        }
        if self.previous == direction {
            return;
        }
        self.previous = direction;
        let position = usize::from(self.position);
        if pattern.get(position).map(|dir| dir.bits()) == Some(direction) {
            self.position += 1;
            if position == 0 {
                self.remaining = 32;
            } else if usize::from(self.position) == pattern.len() {
                self.remaining = 6;
            }
        } else if !first.is_charge() && direction == first.bits() {
            self.position = 1;
            self.remaining = 32;
        }
    }

    fn step_spin(&mut self, direction: u8) {
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining == 0 {
            self.position = 0;
            self.spin_direction = 0;
        }
        const CARDINALS: [u8; 4] = [1, 4, 2, 8];
        let Some(cardinal) = CARDINALS.iter().position(|&bits| bits == direction) else { return };
        let cardinal = cardinal as u8;
        if self.position == 0 {
            self.position = 1;
            self.spin_last = cardinal;
            self.remaining = 32;
        } else if self.position < 4 && self.spin_last != cardinal {
            let turn = (cardinal + 4 - self.spin_last) % 4;
            let spin = match turn {
                1 => 1,
                3 => -1,
                _ => 0,
            };
            if spin == 0 || (self.spin_direction != 0 && self.spin_direction != spin) {
                self.position = 1;
                self.remaining = 32;
            } else {
                self.position += 1;
                if self.position == 4 {
                    self.remaining = 6;
                }
            }
            self.spin_direction = spin;
            self.spin_last = cardinal;
        }
    }

    fn completed(self, pattern: &[Dir]) -> bool {
        usize::from(self.position) == pattern.len()
    }

    fn ready(self, pattern: &[Dir]) -> bool {
        self.completed(pattern)
            && (pattern.is_empty() || pattern == alpha_moves::SPIN
                || pattern.last().map(|dir| dir.bits()) == Some(self.previous))
    }
}

/// Frame and damage adjustments over a shared set of translated fighter streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RevisionDelta {
    pub startup: i8,
    pub recovery: i8,
    pub hitstun: i8,
    pub damage_percent: u16,
}

impl RevisionDelta {
    pub fn of(style: StylePick) -> Option<Self> {
        let (startup, recovery, hitstun, damage_percent) = match (style.revision, style.loadout) {
            (Revision::Ww, _) => (1, 1, 2, 100),
            (Revision::Ce, _) => (0, 1, 1, 106),
            (Revision::Hf, _) => (-1, -2, 0, 95),
            (Revision::Super, _) => (0, 0, 0, 103),
            (Revision::St, Loadout::Old) => (0, 1, 4, 120),
            (Revision::St, _) => (0, -1, 0, 105),
            _ => return None,
        };
        Some(Self {
            startup,
            recovery,
            hitstun,
            damage_percent,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct EraRules {
    pub(crate) enabled: bool,
    pub(crate) gauge_mode: GaugeMode,
    fill_meters: bool,
    pub(crate) untechable_sf2_throws: bool,
    pub(crate) allow_corner_throw_loops: bool,
    pub(crate) classic_1987_special_damage: bool,
    disable_freeze_input_lockout: bool,
    pub(crate) disable_comeback_scaling: bool,
    pub(crate) disable_raw_drive_rush: bool,
}

impl From<MatchConfig> for EraRules {
    fn from(config: MatchConfig) -> Self {
        Self {
            enabled: config.era_select_enabled() && config.dramatic.is_none(),
            gauge_mode: config.gauge_mode,
            fill_meters: config.fill_meters && config.roster.mode == PlayMode::Training,
            untechable_sf2_throws: config.sf1plus.untechable_sf2_throws,
            allow_corner_throw_loops: config.sf1plus.allow_corner_throw_loops,
            classic_1987_special_damage: config.sf1plus.classic_1987_special_damage,
            disable_freeze_input_lockout: config.sf1plus.disable_freeze_input_lockout,
            disable_comeback_scaling: config.disable_comeback_scaling,
            disable_raw_drive_rush: config.disable_raw_drive_rush,
        }
    }
}

/// SFZ3's guard reserve. The maximum contracts for the rest of the round
/// after a crush. The reserve is empty throughout the stagger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GuardPower {
    pub remaining: u8,
    pub maximum: u8,
    pub crush_left: u8,
    recovery_left: u8,
}

crate::canonical::canonical_struct!(GuardPower, "GuardPower.v1", {
    remaining, maximum, crush_left, recovery_left,
});

impl GuardPower {
    fn initial_maximum(loadout: Loadout) -> u8 {
        if loadout == Loadout::XIsm { GUARD_POWER_X_ISM } else { GUARD_POWER_STANDARD }
    }

    fn full(loadout: Loadout) -> Self {
        let maximum = Self::initial_maximum(loadout);
        Self { remaining: maximum, maximum, ..Self::default() }
    }

    fn block(&mut self, loadout: Loadout, pressure: u8) -> bool {
        if self.maximum == 0 || self.crush_left != 0 || pressure == 0 {
            return false;
        }
        self.recovery_left = GUARD_RECOVERY_DELAY;
        self.remaining = self.remaining.saturating_sub(pressure);
        if self.remaining != 0 {
            return false;
        }
        let minimum = Self::initial_maximum(loadout) / 2;
        self.maximum = self.maximum.saturating_sub(GUARD_CRUSH_CAPACITY_LOSS).max(minimum);
        self.crush_left = GUARD_CRUSH_FRAMES;
        true
    }

    fn tick(&mut self, guarding: bool) {
        if self.crush_left != 0 {
            self.crush_left -= 1;
            if self.crush_left == 0 {
                self.remaining = self.maximum;
            }
        } else if self.remaining < self.maximum {
            if guarding {
                self.recovery_left = GUARD_RECOVERY_DELAY;
            } else if self.recovery_left != 0 {
                self.recovery_left -= 1;
            } else {
                self.remaining += 1;
                self.recovery_left = GUARD_RECOVERY_INTERVAL - 1;
            }
        }
    }

    fn gauge_row(self) -> GaugeRow {
        let fill = Fill::of(self.remaining.into(), self.maximum.into());
        let length = Fill::of(self.maximum.into(), GUARD_POWER_X_ISM.into());
        let row = GaugeRow::bar(fill, length);
        if self.crush_left != 0 { row.with_state(GaugeState::Greyed) } else { row }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EraFighterState {
    pub style: StylePick,
    pub fighter: Option<TournamentFighter>,
    pub sub_hp: u16,
    pub max_sub_hp: u16,
    pub super_meter: u8,
    super_capacity: u8,
    unified_gauge: bool,
    what_if: bool,
    super_motion: MotionTracker,
    extra_motion: MotionTracker,
    alpha_counter_motion: MotionTracker,
    custom_frames: u8,
    pub(crate) sf3: sf3::Sf3State,
    pub(crate) sf4: sf4::Sf4State,
    pub(crate) sf5: sf5::VSystem,
    pub(crate) sf6: sf6::DriveState,
    pub(crate) what_if_kit: what_if::WhatIfState,
    alpha_motions: [MotionTracker; ALPHA_MOTION_SLOTS],
    alpha_buttons: u8,
    alpha_previous_buttons: u8,
    punch_taps: u8,
    punch_tap_window: u8,
    pair_held: u8,
    pair_released: bool,
    repeated_kick: bool,
    held_windup_frames: u8,
    gen_crane: bool,
    pub(crate) command_throw_damage: u8,
    sagat_charge: bool,
    pub(crate) charged_tiger_blow: bool,
    death_mark_frames: u8,
    dizzy_frames: u8,
    active_alpha_slot: u8,
    active_alpha_super_level: u8,
    guard_power: GuardPower,
    pub(crate) move_hits: u8,
    pub(crate) juggle_count: u8,
    attack_reached_active: bool,
}

crate::canonical::canonical_struct!(EraFighterState, "EraFighterState.v7", {
    style, fighter, sub_hp, max_sub_hp, super_meter, super_capacity, unified_gauge, what_if,
    super_motion, extra_motion, alpha_counter_motion, custom_frames, sf3, sf4, sf5, sf6, what_if_kit, alpha_motions,
    alpha_buttons, alpha_previous_buttons, punch_taps, punch_tap_window, pair_held, pair_released, repeated_kick, held_windup_frames,
    gen_crane, command_throw_damage, sagat_charge, charged_tiger_blow, death_mark_frames, dizzy_frames,
    active_alpha_slot, active_alpha_super_level, guard_power, move_hits, juggle_count, attack_reached_active,
});

impl EraFighterState {
    pub fn gauge(&self) -> Option<EraGauge> {
        if self.super_capacity == 0 {
            return None;
        }
        if let Some(gauge) = self.sf3_gauge() {
            return Some(gauge);
        }
        let main = GaugeRow::stock(
            self.super_capacity / STOCK_AT,
            self.super_meter / STOCK_AT,
            Fill::of(u16::from(self.super_meter % STOCK_AT), u16::from(STOCK_AT)),
        );
        if !self.unified_gauge {
            if self.is_sf6() {
                return Some(EraGauge::two(self.sf6.row(), main));
            }
            if self.is_sf4() {
                return Some(EraGauge::two(main, self.sf4.revenge_row()));
            }
            if let Some(v_gauge) = self.sf5.gauge_row(GaugeMode::EraDefault) {
                return Some(EraGauge::two(main, v_gauge));
            }
        }
        Some(if let Some(guard) = self.guard_power().filter(|_| !self.unified_gauge) {
            EraGauge::two(main, guard.gauge_row())
        } else {
            EraGauge::one(main)
        })
    }

    pub fn guard_power(&self) -> Option<GuardPower> {
        (self.guard_power.maximum != 0).then_some(self.guard_power)
    }

    pub(crate) fn is_sf2(&self) -> bool {
        self.max_sub_hp != 0
            && self.style.family == EraFamily::Sf2
            && matches!(
                self.fighter,
                Some(TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Sagat)
            )
    }

    pub(crate) fn is_alpha(&self) -> bool {
        self.max_sub_hp != 0
            && self.style.family == EraFamily::SfZ
            && self.fighter.is_some_and(|fighter| self.what_if || !crate::era::what_if_sfz(fighter))
    }

    pub fn custom_combo_active(&self) -> bool {
        self.custom_frames != 0
    }

    pub(crate) fn trail_color(&self) -> Option<u32> {
        if self.custom_combo_active() {
            Some(0xff426cfe)
        } else if let Some(color) = self.what_if_trail() {
            Some(color)
        } else if self.sf4_dash_trail() {
            Some(0xffded8c5)
        } else if self.sf5_afterimages() {
            Some(0xfff4f4f4)
        } else if self.sf6_trail() {
            Some(0xff59d97d)
        } else {
            None
        }
    }

    pub(crate) fn custom_combo_available(&self) -> bool {
        self.is_alpha() && self.style.revision == Revision::Sfz3
            && self.style.loadout == Loadout::VIsm && self.super_capacity != 0
    }

    pub(crate) fn family_move_progress(&self, slot: u8) -> (u8, u8) {
        let what_if = self.what_if_progress(slot);
        if what_if != (0, 0) {
            return what_if;
        }
        match self.style.family {
            EraFamily::Sf2 | EraFamily::Sf3 => self.sf3_progress(slot),
            EraFamily::Sf4 => self.sf4_progress(slot),
            EraFamily::Sf5 => self.sf5_progress(slot),
            EraFamily::Sf6 => self.sf6_progress(slot),
            _ => (0, 0),
        }
    }

    pub(crate) fn family_moves(&self) -> Vec<crate::moves::MoveEntry> {
        let mut moves = match self.style.family {
            EraFamily::Sf2 => self.sf2_air_tatsumaki_entry().into_iter().collect(),
            EraFamily::Sf3 => self.sf3_active_moves(),
            EraFamily::Sf4 => self.sf4_active_moves(),
            EraFamily::Sf5 => self.sf5_active_moves(),
            EraFamily::Sf6 => self.sf6_active_moves(),
            _ => Vec::new(),
        };
        if self.what_if_active() {
            moves.retain(|entry| entry.kind == MoveKind::System);
            moves.extend(self.what_if_moves());
        }
        moves
    }

    pub(crate) fn fighter_tint(&self) -> Option<sf_video::compositor::PaletteTint> {
        if let Some(tint) = self.what_if_tint() {
            return Some(tint);
        }
        match self.style.family {
            EraFamily::Sf3 => self.sf3_tint(),
            EraFamily::Sf4 => self.sf4_tint().map(|(color, strength)| sf_video::compositor::PaletteTint { color, strength }),
            EraFamily::Sf5 => self.sf5_tint(),
            EraFamily::Sf6 => self.sf6_tint(),
            _ => None,
        }
    }

    pub(crate) fn what_if_active(&self) -> bool {
        self.what_if
    }

    pub(crate) fn uses_family_bank(&self) -> bool {
        self.what_if_active()
            || self.sf2_air_tatsumaki_available()
            || matches!(self.style.family, EraFamily::Sf3 | EraFamily::Sf4 | EraFamily::Sf5 | EraFamily::Sf6)
    }

    pub(crate) fn is_alpha_x_ism(&self) -> bool {
        self.is_alpha() && self.style.revision == Revision::Sfz3 && self.style.loadout == Loadout::XIsm
    }

    pub(crate) fn gen_stance(&self) -> Stance {
        if self.gen_crane { Stance::Crane } else { Stance::Mantis }
    }

    fn alpha_roster(&self) -> &'static [AlphaMove] {
        if !self.is_alpha() {
            return &[];
        }
        match self.fighter {
            Some(TournamentFighter::Ryu) => RYU_ALPHA_SUPERS,
            Some(TournamentFighter::Ken) => KEN_ALPHA_SUPERS,
            Some(TournamentFighter::Sagat) => alpha_moves::SAGAT,
            Some(TournamentFighter::Adon) => alpha_moves::ADON,
            Some(TournamentFighter::Birdie) => alpha_moves::BIRDIE,
            Some(TournamentFighter::Eagle) => alpha_moves::EAGLE,
            Some(TournamentFighter::Gen) => alpha_moves::GEN,
            _ => &[],
        }
    }

    pub(crate) fn alpha_moves(&self) -> impl Iterator<Item = AlphaMove> + '_ {
        self.alpha_roster().iter().copied().filter(|entry| entry.available(self, true))
    }

    pub(crate) fn replaces_special(&self, slot: u8) -> bool {
        self.alpha_roster().iter().any(|entry| entry.kind == MoveKind::Special && entry.slot == slot)
            || (self.is_alpha() && match self.fighter {
                Some(TournamentFighter::Eagle) => slot == 0x1b,
                Some(TournamentFighter::Gen) => slot == 0x19,
                _ => false,
            })
    }

    pub(crate) fn alpha_move_active(&self, slot: u8) -> bool {
        self.active_alpha_slot == slot && slot != 0
    }

    pub(crate) fn alpha_move_airborne(&self, slot: u8) -> bool {
        self.alpha_move_active(slot) && self.alpha_roster().iter().any(|entry| entry.slot == slot && entry.airborne)
    }

    pub(crate) fn alpha_move_overhead(&self, slot: u8) -> bool {
        self.alpha_move_active(slot) && self.alpha_roster().iter().any(|entry| entry.slot == slot && entry.overhead)
    }

    fn track_alpha_buttons(&mut self) {
        self.punch_tap_window = self.punch_tap_window.saturating_sub(1);
        if self.punch_tap_window == 0 {
            self.punch_taps = 0;
        }
        if (self.alpha_buttons & !self.alpha_previous_buttons & PUNCH_MASK) != 0 {
            self.punch_taps = self.punch_taps.saturating_add(1);
            self.punch_tap_window = 18;
        }
        let held_pair = (self.alpha_buttons & PUNCH_MASK).count_ones() >= 2
            || (self.alpha_buttons & KICK_MASK).count_ones() >= 2;
        self.pair_released = !held_pair && self.pair_held as usize >= CHARGE_FRAMES;
        self.pair_held = if held_pair { self.pair_held.saturating_add(1) } else { 0 };
        if self.fighter == Some(TournamentFighter::Gen) && self.alpha_move_active(0x1b) {
            self.repeated_kick |= self.alpha_buttons & !self.alpha_previous_buttons & KICK_MASK != 0;
        }
    }

    fn alpha_button_fires(&self, button: ButtonClass, strength: [u8; 2]) -> bool {
        use ButtonClass::*;
        let new_buttons = self.alpha_buttons & !self.alpha_previous_buttons;
        match button {
            Punch | Kick | Either => strength[usize::from(button.kick())] != 0
                || (button == Either && strength[1] != 0),
            MashPunch => self.punch_taps >= 4 && new_buttons & PUNCH_MASK != 0,
            TriplePunch => self.alpha_buttons & PUNCH_MASK == PUNCH_MASK && new_buttons & PUNCH_MASK != 0,
            TripleKick => self.alpha_buttons & KICK_MASK == KICK_MASK && new_buttons & KICK_MASK != 0,
            ReleasePair => self.pair_released,
            MediumPunch | MediumKick => button.held(new_buttons),
            _ => false,
        }
    }

    fn alpha_super_cost(&self, level: u8) -> Option<u8> {
        if self.super_capacity == 0 || !(1..=3).contains(&level) {
            return None;
        }
        if self.is_alpha_x_ism() {
            (level == 3).then_some(self.super_capacity)
        } else {
            Some(level * STOCK_AT)
        }
    }

    pub(crate) fn alpha_super_level(&self, move_id: u8) -> u8 {
        if move_id == self.active_alpha_slot {
            self.active_alpha_super_level
        } else {
            0
        }
    }

    pub(crate) fn alpha_move_progress(&self, move_id: u8) -> (u8, u8) {
        self.alpha_roster().iter().enumerate()
            .find(|(_, entry)| entry.slot == move_id && entry.available(self, true))
            .map_or((0, 0), |(index, entry)| {
                let tracker = self.alpha_motions[index];
                let jump = u8::from(entry.input.motion.first() == Some(&Dir::Jump)
                    && tracker.position != 0);
                (tracker.position + jump, tracker.remaining)
            })
    }

    fn st_new(&self) -> bool {
        self.is_sf2()
            && self.style.revision == Revision::St
            && self.style.loadout == Loadout::Standard
    }

    fn set_gauge_mode(&mut self, mode: GaugeMode) {
        self.unified_gauge = mode == GaugeMode::Unified;
        self.sf3_set_gauge_mode(mode);
        self.sf5.configure(self.style, self.fighter, mode);
        self.sf6.configure(mode == GaugeMode::EraDefault && self.is_sf6());
        self.super_capacity = match mode {
            GaugeMode::EraDefault if self.st_new() || (self.what_if_active() && self.style.revision == Revision::St) => STOCK_AT,
            GaugeMode::EraDefault if self.is_alpha() => match self.style.loadout {
                // One 48-point stock is exactly half of the V-ISM gauge.
                Loadout::VIsm => 2 * STOCK_AT,
                Loadout::XIsm => STOCK_AT,
                _ => 3 * STOCK_AT,
            },
            GaugeMode::EraDefault if self.is_sf3() => self.sf3_capacity().unwrap_or_default(),
            GaugeMode::EraDefault if self.is_sf4() => 4 * STOCK_AT,
            GaugeMode::EraDefault if sf5::available(self.style, self.fighter) || self.is_sf6() => 3 * STOCK_AT,
            GaugeMode::Unified if self.max_sub_hp != 0 => 3 * STOCK_AT,
            _ => 0,
        };
        self.super_meter = self.super_meter.min(self.super_capacity);
        if mode == GaugeMode::NoGauges
            || !self.is_alpha()
            || self.style.revision != Revision::Sfz3
            || !matches!(self.style.loadout, Loadout::AIsm | Loadout::VIsm | Loadout::XIsm)
        {
            self.guard_power = GuardPower::default();
        } else if self.guard_power.maximum == 0 {
            self.guard_power = GuardPower::full(self.style.loadout);
        }
        if self.super_capacity == 0 {
            self.alpha_counter_motion = MotionTracker::default();
            self.custom_frames = 0;
        }
    }

    pub(crate) fn alpha_counter_enabled(&self) -> bool {
        self.is_alpha() && self.super_capacity != 0 && self.style.loadout != Loadout::XIsm
    }

    fn alpha_counter_ready(&self) -> bool {
        self.alpha_counter_enabled()
            && self.guard_power.crush_left == 0
            && self.super_meter >= STOCK_AT
            && self.alpha_counter_motion.ready(ALPHA_COUNTER_MOTION)
    }

    pub(crate) fn set_super_meter(&mut self, value: Option<u8>) {
        self.super_meter = value
            .unwrap_or(self.super_capacity)
            .min(self.super_capacity);
    }

    pub(crate) fn super_ready(&self) -> bool {
        self.st_new()
            && self.super_capacity > 0
            && self.super_meter == self.super_capacity
            && self.super_motion.completed(DOUBLE_QUARTER_CIRCLE)
    }

    pub(crate) fn command_progress(&self, super_move: bool) -> (u8, u8) {
        let tracker = if super_move {
            self.super_motion
        } else {
            self.extra_motion
        };
        (tracker.position, tracker.remaining)
    }

    pub(crate) fn alpha_counter_progress(&self) -> (u8, u8) {
        (self.alpha_counter_motion.position, self.alpha_counter_motion.remaining)
    }

    pub(crate) fn extra_ready(&self) -> bool {
        match self.fighter {
            Some(TournamentFighter::Ryu) => {
                matches!(self.style.revision, Revision::Super | Revision::St)
                    && self.extra_motion.completed(HALF_CIRCLE)
            }
            Some(TournamentFighter::Sagat) => self.extra_motion.completed(TIGER_KNEE),
            _ => false,
        }
    }

    pub(crate) fn move_hit_limit(&self, move_id: u8) -> u8 {
        if let Some(limit) = self.sf3_hit_limit(move_id) {
            return limit;
        }
        let alpha_level = self.alpha_super_level(move_id);
        if alpha_level != 0 {
            return alpha_super_hit_limit(alpha_level);
        }
        if self.fighter == Some(TournamentFighter::Gen) && move_id == 0x1b && self.alpha_move_active(move_id) {
            return 8;
        }
        match (self.fighter, move_id) {
            (Some(TournamentFighter::Ken | TournamentFighter::Sagat), SF2_SUPER) => 3,
            (Some(TournamentFighter::Ken), 0x19)
                if matches!(
                    self.style.revision,
                    Revision::Ce | Revision::Hf | Revision::St
                ) =>
            {
                2
            }
            (Some(TournamentFighter::Sagat), SF2_EXTRA_SPECIAL) => 2,
            _ => 1,
        }
    }

    fn health_points(&self) -> u8 {
        (u32::from(self.sub_hp) * 48).div_ceil(u32::from(self.max_sub_hp)) as u8
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EraCombatState {
    pub(crate) selections: [StylePick; 2],
    /// Authoritative fight-side state exposed in saved snapshots.
    pub fighters: [EraFighterState; 2],
    arcade_step: Option<u16>,
    pub(crate) freeze: u8,
    pub(crate) super_events: [Option<TournamentFighter>; 2],
    pub(crate) alpha_super_events: [Option<&'static str>; 2],
    pub(crate) alpha_counter_events: [bool; 2],
    pub(crate) guard_crush_events: [bool; 2],
    pub(crate) style_callouts: [Option<CalloutKind>; 2],
    projectiles: [EraProjectileState; PROJECTILE_SLOTS],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EraProjectileState {
    pub move_id: u8,
    pub hits_remaining: u8,
    pub crimson: bool,
    pub alpha_super_level: u8,
    pub family_hits: u8,
    pub unblockable: bool,
    pub sf4_scale: u8,
    hit_wait: u8,
    clash_wait: u8,
}

crate::canonical::canonical_struct!(EraProjectileState, "EraProjectileState.v2", {
    move_id, hits_remaining, crimson, alpha_super_level, family_hits, unblockable, sf4_scale, hit_wait, clash_wait,
});

impl EraProjectileState {
    pub(crate) fn hit_limit(self) -> u8 {
        if self.family_hits != 0 {
            self.family_hits
        } else if self.alpha_super_level != 0 {
            alpha_super_hit_limit(self.alpha_super_level)
        } else if self.crimson {
            3
        } else if self.move_id == SF2_SUPER {
            5
        } else {
            1
        }
    }
}

fn projectile_index(slot: u16) -> Option<usize> {
    let offset = usize::from(slot.checked_sub(0x100)?);
    let index = offset / 0x40;
    (offset % 0x40 == 0 && index < PROJECTILE_SLOTS).then_some(index)
}

impl crate::canonical::Canonical for EraCombatState {
    fn encode(&self, out: &mut crate::canonical::CanonicalHasher) {
        let Self {
            selections,
            fighters,
            arcade_step,
            freeze,
            super_events: _,
            alpha_super_events: _,
            alpha_counter_events: _,
            guard_crush_events: _,
            style_callouts: _,
            projectiles,
        } = self;
        out.tag("EraCombatState.v1");
        selections.encode(out);
        fighters.encode(out);
        arcade_step.encode(out);
        freeze.encode(out);
        projectiles.encode(out);
    }
}

impl Bus<'_> {
    pub(crate) fn era_begin_round(&mut self) {
        use TournamentFighter::{Adon, Birdie, Geki, Gen, Mike, Sagat};

        if !self.era_rules.enabled || self.rb(0xff8003) == 0 {
            return;
        }
        let arcade = self.roster.mode == PlayMode::Arcade;
        let mode = self.rb(GAME_MODE);
        let state = self.toggle_state.era;
        let step = arcade.then(|| u16::from(self.rb(0xffc003)) * 2 + u16::from(self.rb(0xffc004)));
        let new_match = step != state.arcade_step;
        self.toggle_state.era.arcade_step = step;
        self.toggle_state.era.freeze = 0;
        self.toggle_state.era.super_events = [None; 2];
        self.toggle_state.era.alpha_super_events = [None; 2];
        self.toggle_state.era.alpha_counter_events = [false; 2];
        self.toggle_state.era.guard_crush_events = [false; 2];
        self.toggle_state.era.style_callouts = [None; 2];
        self.toggle_state.era.projectiles = [EraProjectileState::default(); PROJECTILE_SLOTS];
        for side in Side::BOTH {
            let index = side.index();
            let ps = player_struct(side, mode);
            let fighter = TournamentFighter::from_move_table(self.rl(ps + 0x12));
            let style = if arcade {
                let pick = state.selections[0];
                if ps == CPU_STRUCT {
                    fighter.map_or_else(StylePick::default, |opponent| {
                        pick.arcade_opponent(
                            opponent,
                            self.roster.what_if_styles,
                            usize::from(step.unwrap_or(0)),
                        )
                    })
                } else {
                    pick
                }
            } else {
                state.selections[index]
            };
            let hp = match (style.family, fighter) {
                (EraFamily::Sf3 | EraFamily::Sf4 | EraFamily::Sf5 | EraFamily::Sf6, fighter) => match fighter {
                    Some(Gen) => 864,
                    Some(Adon | Geki) => 912,
                    Some(Sagat | Mike | Birdie) => 1008,
                    _ => FULL_SUB_HP,
                },
                _ => FULL_SUB_HP,
            };
            let mut next = EraFighterState {
                style,
                fighter,
                what_if: fighter.is_some_and(|fighter| {
                    !matches!(style.family, EraFamily::Sf1987 | EraFamily::Sf1Plus)
                        && !crate::era::revisions(fighter, style.family).contains(&style.revision)
                }),
                sub_hp: hp,
                max_sub_hp: hp,
                ..EraFighterState::default()
            };
            next.set_gauge_mode(self.era_rules.gauge_mode);
            let previous = state.fighters[index];
            next.super_meter = if self.era_rules.fill_meters {
                next.super_capacity
            } else if !new_match && previous.style == style && previous.fighter == fighter {
                previous.super_meter.min(next.super_capacity)
            } else {
                0
            };
            self.toggle_state.era.fighters[index] = next;
        }
    }

    pub(crate) fn era_reconfigure(&mut self, was_enabled: bool) {
        let mode = self.rb(GAME_MODE);
        for side in Side::BOTH {
            let ps = player_struct(side, mode);
            let native_points = u16::from(self.rb(ps + 7)) * 4 + u16::from(self.rb(ps + 6));
            let fighter = &mut self.toggle_state.era.fighters[side.index()];
            if !self.era_rules.enabled {
                fighter.set_gauge_mode(GaugeMode::NoGauges);
                fighter.super_motion = MotionTracker::default();
                fighter.extra_motion = MotionTracker::default();
            } else {
                if !was_enabled && fighter.max_sub_hp != 0 {
                    fighter.sub_hp = (native_points * SUB_HP_PER_POINT).min(fighter.max_sub_hp);
                }
                fighter.set_gauge_mode(self.era_rules.gauge_mode);
            }
        }
        if !self.era_rules.enabled {
            let era = &mut self.toggle_state.era;
            era.freeze = 0;
            era.super_events = [None; 2];
            era.alpha_super_events = [None; 2];
            era.alpha_counter_events = [false; 2];
            era.guard_crush_events = [false; 2];
            era.style_callouts = [None; 2];
            era.projectiles = [EraProjectileState::default(); PROJECTILE_SLOTS];
        }
    }

    pub(crate) fn era_side(&self, ps: u32) -> Option<usize> {
        if !self.era_rules.enabled {
            return None;
        }
        let mode = self.rb(GAME_MODE);
        Side::BOTH.into_iter().find_map(|side| {
            let i = side.index();
            (player_struct(side, mode) == ps && self.toggle_state.era.fighters[i].max_sub_hp != 0)
                .then_some(i)
        })
    }

    pub(crate) fn era_state(&self, ps: u32) -> Option<&EraFighterState> {
        self.toggle_state.era.fighters.get(self.era_side(ps)?)
    }

    pub(crate) fn era_buttons(&self, ps: u32) -> (u8, u8) {
        self.era_state(ps).map_or((0, 0), |state| {
            (state.alpha_buttons, state.alpha_buttons & !state.alpha_previous_buttons)
        })
    }

    pub(crate) fn sf2_state(&self, ps: u32) -> Option<&EraFighterState> {
        let state = &self.toggle_state.era.fighters[self.era_side(ps)?];
        state.is_sf2().then_some(state)
    }

    pub(crate) fn alpha_state(&self, ps: u32) -> Option<&EraFighterState> {
        let state = &self.toggle_state.era.fighters[self.era_side(ps)?];
        state.is_alpha().then_some(state)
    }

    pub(crate) fn chosen_era(&self, ps: u32) -> Option<EraFamily> {
        Some(
            self.toggle_state.era.fighters[self.era_side(ps)?]
                .style
                .family,
        )
    }

    pub(crate) fn era_offense_flag(&self, ps: u32, configured: bool) -> bool {
        match self.chosen_era(ps) {
            Some(EraFamily::Sf1987) => false,
            Some(EraFamily::Sf1Plus) | None => configured,
            Some(_) => true,
        }
    }

    pub(crate) fn era_projectile(&self, slot: u16) -> EraProjectileState {
        projectile_index(slot).map_or_else(EraProjectileState::default, |index| {
            self.toggle_state.era.projectiles[index]
        })
    }

    fn era_projectile_mut(&mut self, slot: u16) -> Option<&mut EraProjectileState> {
        Some(&mut self.toggle_state.era.projectiles[projectile_index(slot)?])
    }

    pub(crate) fn era_clear_projectile(&mut self, slot: u16) {
        if let Some(state) = self.era_projectile_mut(slot) {
            *state = EraProjectileState::default();
        }
    }

    pub(crate) fn era_mark_attack_active(&mut self, ps: u32) {
        if let Some(side) = self.era_side(ps) {
            self.toggle_state.era.fighters[side].attack_reached_active = true;
        }
    }

    pub(crate) fn era_spawn_projectile(&mut self, slot: u16, ps: u32, move_id: u8) {
        self.era_mark_attack_active(ps);
        self.era_clear_projectile(slot);
        let Some(era) = self.era_state(ps).filter(|state| state.is_sf2() || state.is_alpha() || state.uses_family_bank()) else {
            return;
        };
        let alpha_super_level = era.alpha_super_level(move_id);
        let sf3 = era.sf3_projectile(move_id);
        let family_hits = sf3.as_ref().map(|projectile| projectile.hits)
            .or_else(|| era.what_if_others_projectile_hits(move_id))
            .or_else(|| era.what_if_projectile_hits(move_id))
            .unwrap_or_else(|| {
                if !era.alpha_move_active(move_id) {
                    return 0;
                }
                match (era.style.family, era.fighter, move_id) {
                    (EraFamily::Sf6, _, 0x24) => 2,
                    (EraFamily::Sf6, _, 0x29) => 4,
                    (EraFamily::Sf5, Some(TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Sagat),
                        sf5::EX_FORWARD | sf5::EX_LOW_SHOT) => 2,
                    _ => 0,
                }
            });
        let sf4_scale = era.sf4_projectile_scale(move_id);
        let crimson = era.is_sf2() && era.fighter == Some(TournamentFighter::Ryu) && move_id == SF2_EXTRA_SPECIAL;
        if let Some(state) = self.era_projectile_mut(slot) {
            *state = EraProjectileState {
                move_id,
                crimson,
                alpha_super_level,
                family_hits,
                sf4_scale,
                unblockable: sf3.is_some_and(|projectile| projectile.unblockable),
                ..EraProjectileState::default()
            };
            state.hits_remaining = state.hit_limit();
        }
    }

    pub(crate) fn era_projectile_hit(&mut self, slot: u16) -> bool {
        let Some(state) = self.era_projectile_mut(slot) else {
            return true;
        };
        if state.hits_remaining == 0 {
            return true;
        }
        state.hits_remaining = state.hits_remaining.saturating_sub(1);
        state.hit_wait = 3;
        state.hits_remaining == 0
    }

    pub(crate) fn era_projectile_can_hit(&self, slot: u16) -> bool {
        self.era_projectile(slot).hit_wait == 0
    }

    pub(crate) fn era_projectile_can_clash(&self, slot: u16) -> bool {
        self.era_projectile(slot).clash_wait == 0
    }

    pub(crate) fn era_projectile_clash(&mut self, slot: u16) -> bool {
        let Some(state) = self.era_projectile_mut(slot) else {
            return true;
        };
        if state.hits_remaining == 0 {
            return true;
        }
        state.hits_remaining = state.hits_remaining.saturating_sub(1);
        state.clash_wait = 4;
        state.hits_remaining == 0
    }

    pub(crate) fn era_projectile_frame(&mut self, slot: u16) {
        if let Some(state) = self.era_projectile_mut(slot) {
            state.hit_wait = state.hit_wait.saturating_sub(1);
            state.clash_wait = state.clash_wait.saturating_sub(1);
        }
    }

    pub(crate) fn era_track_motion(&mut self, ps: u32, relative_direction: u8) {
        let Some(index) = self.era_side(ps) else {
            return;
        };
        let state = &mut self.toggle_state.era.fighters[index];
        if state.is_alpha() {
            state.track_alpha_buttons();
            for (index, entry) in state.alpha_roster().iter().enumerate() {
                if entry.available(state, false) {
                    state.alpha_motions[index].step(relative_direction, entry.motion());
                }
            }
            return;
        }
        if !state.is_sf2() {
            return;
        }
        state
            .super_motion
            .step(relative_direction, DOUBLE_QUARTER_CIRCLE);
        match state.fighter {
            Some(TournamentFighter::Ryu) => {
                state.extra_motion.step(relative_direction, HALF_CIRCLE)
            }
            Some(TournamentFighter::Sagat) => {
                state.extra_motion.step(relative_direction, TIGER_KNEE)
            }
            _ => {}
        }
    }

    pub(crate) fn era_spend_super(&mut self, ps: u32) {
        let Some(index) = self.era_side(ps) else {
            return;
        };
        let state = &mut self.toggle_state.era.fighters[index];
        state.super_meter = 0;
        state.super_motion = MotionTracker::default();
        self.toggle_state.era.freeze = SUPER_FREEZE_FRAMES;
        self.toggle_state.era.super_events[index] = state.fighter;
    }

    pub(crate) fn era_read_buttons(&mut self, ps: u32, buttons: u8) {
        if let Some(side) = self.era_side(ps).filter(|&i| {
            let state = &self.toggle_state.era.fighters[i];
            state.is_alpha() || state.uses_family_bank()
        }) {
            self.toggle_state.era.fighters[side].alpha_buttons = buttons;
        }
    }

    pub(crate) fn era_finish_input(&mut self, ps: u32) {
        if let Some(side) = self.era_side(ps) {
            let state = &mut self.toggle_state.era.fighters[side];
            state.alpha_previous_buttons = state.alpha_buttons;
        }
    }

    pub(crate) fn era_alpha_command(
        &self, ps: u32, strength: [u8; 2], airborne: bool, normal: bool,
    ) -> Option<(AlphaMove, u8, u8)> {
        let state = self.alpha_state(ps)?;
        state.alpha_roster().iter().enumerate().find_map(|(index, entry)| {
            let tracker = state.alpha_motions[index];
            let direction_ready = if entry.kind == MoveKind::Normal {
                !normal && tracker.previous == Dir::Forward.bits()
            } else {
                tracker.ready(entry.motion())
            };
            if !entry.available(state, false) || entry.airborne != airborne
                || (normal && entry.input.button != ButtonClass::MashPunch)
                || !state.alpha_button_fires(entry.input.button, strength) || !direction_ready
            {
                return None;
            }
            let requested = if entry.input.button == ButtonClass::Either {
                strength.into_iter().max()?
            } else {
                strength[usize::from(entry.input.button.kick())]
            }.max(1);
            Some((*entry, entry.level(state, requested)?, requested))
        })
    }

    pub(crate) fn era_begin_alpha_move(&mut self, ps: u32, entry: AlphaMove, level: u8) {
        let Some(index) = self.era_side(ps) else { return };
        let state = &mut self.toggle_state.era.fighters[index];
        state.alpha_motions = [MotionTracker::default(); ALPHA_MOTION_SLOTS];
        state.active_alpha_slot = entry.slot;
        state.active_alpha_super_level = level;
        state.punch_taps = 0;
        state.repeated_kick = false;
        state.held_windup_frames = 0;
        state.move_hits = 0;
        if state.fighter == Some(TournamentFighter::Sagat) {
            if entry.slot == 0x2f {
                state.sagat_charge = true;
            } else if entry.slot == SF2_EXTRA_SPECIAL {
                state.charged_tiger_blow = std::mem::take(&mut state.sagat_charge);
            }
        }
        if let Some(stance) = entry.switch_to {
            state.gen_crane = stance == Stance::Crane;
        }
        if level != 0 {
            let cost = state.alpha_super_cost(level).expect("recognized Alpha Super");
            state.super_meter -= cost;
            self.toggle_state.era.freeze = SUPER_FREEZE_FRAMES;
            self.toggle_state.era.alpha_super_events[index] = Some(entry.name);
        }
    }

    pub(crate) fn era_emit_callout(&mut self, ps: u32, kind: CalloutKind) {
        if let Some(side) = self.era_side(ps) {
            self.toggle_state.era.style_callouts[side] = Some(kind);
        }
    }

    pub(crate) fn era_begin_custom_combo(&mut self, ps: u32) -> bool {
        let Some(side) = self.era_side(ps) else { return false };
        let unified = self.era_rules.gauge_mode == GaugeMode::Unified;
        let state = &mut self.toggle_state.era.fighters[side];
        let minimum = if unified { 2 * STOCK_AT } else { STOCK_AT };
        if !state.custom_combo_available() || state.custom_combo_active() || state.super_meter < minimum {
            return false;
        }
        let points = if unified {
            state.super_meter -= minimum;
            minimum
        } else {
            state.super_meter
        };
        state.custom_frames = points * 2;
        state.alpha_motions = [MotionTracker::default(); ALPHA_MOTION_SLOTS];
        self.era_stage_banked_move(ps, crate::data_bank::ALPHA_COUNTER_SLOT, 0, None);
        self.toggle_state.era.freeze = 8;
        self.era_emit_callout(ps, CalloutKind::CustomCombo);
        true
    }

    pub(crate) fn era_stage_banked_move(&mut self, ps: u32, slot: u8, level: u8, super_name: Option<&'static str>) {
        let Some(side) = self.era_side(ps) else { return };
        let state = &mut self.toggle_state.era.fighters[side];
        state.active_alpha_slot = slot;
        state.active_alpha_super_level = level;
        state.move_hits = 0;
        state.held_windup_frames = 0;
        if let Some(name) = super_name {
            self.toggle_state.era.freeze = SUPER_FREEZE_FRAMES;
            self.toggle_state.era.alpha_super_events[side] = Some(name);
        }
    }

    pub(crate) fn era_clear_alpha_move(&mut self, ps: u32, next_move: u8) {
        if let Some(side) = self.era_side(ps) {
            let state = &mut self.toggle_state.era.fighters[side];
            if state.active_alpha_slot != next_move {
                state.active_alpha_slot = 0;
                state.active_alpha_super_level = 0;
                state.charged_tiger_blow = false;
            }
        }
    }

    pub(crate) fn era_hold_punch(&mut self, ps: u32, limit: u8) -> bool {
        let Some(side) = self.era_side(ps) else { return false };
        let state = &mut self.toggle_state.era.fighters[side];
        if state.held_windup_frames >= limit || state.alpha_buttons & PUNCH_MASK == 0 {
            return false;
        }
        state.held_windup_frames += 1;
        true
    }

    pub(crate) fn era_take_repeated_kick(&mut self, ps: u32) -> bool {
        self.era_side(ps).is_some_and(|side| {
            std::mem::take(&mut self.toggle_state.era.fighters[side].repeated_kick)
        })
    }

    pub(crate) fn era_dizzy(&self, ps: u32) -> bool {
        self.era_side(ps).is_some_and(|side| self.toggle_state.era.fighters[side].dizzy_frames != 0)
    }

    pub(crate) fn era_alpha_damage_effects(&mut self, owner: u32, target: u32, move_id: u8, level: u8) {
        let is_gen = |fighter: &EraFighterState| fighter.fighter == Some(TournamentFighter::Gen);
        if let Some(target) = self.era_side(target) {
            let states = &mut self.toggle_state.era.fighters;
            states[target].dizzy_frames = 0;
            if states[target].custom_combo_active() {
                states[target].custom_frames = 0;
                if self.era_rules.gauge_mode != GaugeMode::Unified {
                    states[target].super_meter = 0;
                }
            }
            if is_gen(&states[target]) {
                states[1 - target].death_mark_frames = 0;
            }
        }
        if move_id == 0x2a && level != 0 && self.alpha_state(owner).is_some_and(is_gen) {
            if let Some(target) = self.era_side(target) {
                self.toggle_state.era.fighters[target].death_mark_frames = 240 - 60 * level;
            }
        }
    }

    pub(crate) fn era_track_alpha_counter(&mut self, ps: u32, relative_direction: u8) {
        if let Some(index) = self.era_side(ps) {
            let state = &mut self.toggle_state.era.fighters[index];
            if state.alpha_counter_enabled() {
                state.alpha_counter_motion.step(relative_direction, ALPHA_COUNTER_MOTION);
            }
        }
    }

    pub(crate) fn era_reset_alpha_counter(&mut self, ps: u32) {
        if let Some(index) = self.era_side(ps) {
            self.toggle_state.era.fighters[index].alpha_counter_motion = MotionTracker::default();
        }
    }

    pub(crate) fn era_alpha_counter_ready(&self, ps: u32) -> bool {
        self.alpha_state(ps).is_some_and(EraFighterState::alpha_counter_ready)
    }

    pub(crate) fn era_guard_crushed(&self, ps: u32) -> bool {
        self.era_side(ps).is_some_and(|index| self.toggle_state.era.fighters[index].guard_power.crush_left != 0)
    }

    pub(crate) fn era_guard_contact(&mut self, ps: u32, pressure: u8) -> bool {
        let Some(index) = self.era_side(ps) else { return false };
        let era = &mut self.toggle_state.era;
        let fighter = &mut era.fighters[index];
        if fighter.sub_hp == 0 || !fighter.guard_power.block(fighter.style.loadout, pressure) {
            return false;
        }
        fighter.alpha_counter_motion = MotionTracker::default();
        era.guard_crush_events[index] = true;
        true
    }

    pub(crate) fn era_spend_alpha_counter(&mut self, ps: u32) -> bool {
        let Some(index) = self.era_side(ps) else { return false };
        let state = &mut self.toggle_state.era.fighters[index];
        if !state.alpha_counter_ready() {
            return false;
        }
        state.super_meter -= STOCK_AT;
        state.alpha_counter_motion = MotionTracker::default();
        self.toggle_state.era.alpha_counter_events[index] = true;
        true
    }

    pub(crate) fn era_use_extra_motion(&mut self, ps: u32) {
        if let Some(index) = self.era_side(ps) {
            self.toggle_state.era.fighters[index].extra_motion = MotionTracker::default();
        }
    }

    pub(crate) fn era_frozen(&self) -> bool {
        self.era_rules.enabled && self.toggle_state.era.freeze != 0
    }

    pub(crate) fn era_freeze_tick(&mut self) -> bool {
        if !self.era_frozen() {
            return self.sf5_slow_tick();
        }
        if !self.era_rules.disable_freeze_input_lockout {
            self.clear_frozen_attack_inputs();
        }
        self.toggle_state.era.freeze -= 1;
        true
    }

    pub(crate) fn era_meter_gain(&mut self, ps: u32, gain: u8) {
        if let Some(index) = self.era_side(ps) {
            let state = &mut self.toggle_state.era.fighters[index];
            if !state.custom_combo_active() {
                state.super_meter = state.super_meter.saturating_add(gain).min(state.super_capacity);
            }
        }
    }

    pub(crate) fn era_damage(&mut self, ps: u32, damage: u16, preserve_one: bool) -> bool {
        let Some(index) = self.era_side(ps) else {
            return false;
        };
        let fighter = &mut self.toggle_state.era.fighters[index];
        if fighter.sub_hp != 0 {
            fighter.sub_hp = fighter
                .sub_hp
                .saturating_sub(damage)
                .max(u16::from(preserve_one));
        }
        self.era_sync_health(ps, index);
        true
    }

    pub(crate) fn era_refill_health(&mut self, ps: u32) {
        if let Some(index) = self.era_side(ps) {
            let fighter = &mut self.toggle_state.era.fighters[index];
            fighter.sub_hp = fighter.max_sub_hp;
        }
    }

    pub(crate) fn era_health(&self, ps: u32) -> Option<(u32, u32)> {
        let fighter = self.toggle_state.era.fighters[self.era_side(ps)?];
        Some((u32::from(fighter.sub_hp), u32::from(fighter.max_sub_hp)))
    }

    pub(crate) fn era_sync_frame(&mut self) {
        let mode = self.rb(GAME_MODE);
        for side in Side::BOTH {
            let ps = player_struct(side, mode);
            if let Some(index) = self.era_side(ps) {
                self.era_sf4_tick(ps);
                self.era_sync_health(ps, index);
                let entity = self.rw(ps + 0x10);
                let guarding = self.eb(entity, 9) & 0x20 != 0 || matches!(self.eb(entity, 0xa), 4 | 8);
                let ready = self.eb(entity, 9) & 0xc1 == 0 && self.eb(entity, 8) == 2;
                let state = &mut self.toggle_state.era.fighters[index];
                state.sf3_tick();
                state.what_if_tick();
                state.sf6.tick(self.era_rules.fill_meters, ready, &mut state.sub_hp, state.max_sub_hp);
                state.sf5.tick(self.era_rules.fill_meters, ready, &mut state.sub_hp, state.max_sub_hp);
                if state.is_sf4() && self.era_rules.fill_meters && ready && state.super_capacity != 0 {
                    state.sf4.fill();
                }
                let was_custom = state.custom_combo_active();
                state.custom_frames = state.custom_frames.saturating_sub(1);
                if was_custom && state.custom_frames % 2 == 0 && self.era_rules.gauge_mode != GaugeMode::Unified {
                    state.super_meter = state.super_meter.saturating_sub(1);
                }
                state.guard_power.tick(guarding);
                state.dizzy_frames = state.dizzy_frames.saturating_sub(1);
                if state.death_mark_frames != 0 {
                    state.death_mark_frames -= 1;
                    if state.death_mark_frames == 0 && state.sub_hp != 0 {
                        state.dizzy_frames = 60;
                    }
                }
                if self.eb(entity, 9) & 3 != 1 {
                    self.toggle_state.era.fighters[index].juggle_count = 0;
                }
                if self.era_rules.fill_meters && !was_custom
                    && self.eb(entity, 9) & 0xc1 == 0
                    && self.eb(entity, 8) == 2
                {
                    let fighter = &mut self.toggle_state.era.fighters[index];
                    fighter.super_meter = fighter.super_capacity;
                }
            }
        }
    }

    fn era_sync_health(&mut self, ps: u32, index: usize) {
        let points = self.toggle_state.era.fighters[index].health_points();
        // Native life uses the remainder and cell count. The entity's +0x22
        // is a hitbox offset and the player's +0x22 holds score digits.
        let previous = u16::from(self.rb(ps + 7)) * 4 + u16::from(self.rb(ps + 6));
        if previous > u16::from(points) && previous <= 48 {
            self.drain_native_health(ps, previous as u8 - points);
        } else if previous != u16::from(points) {
            let cells = points.saturating_sub(1) / 4;
            let remainder = points - cells * 4;
            self.wb(ps + 6, remainder);
            self.wb(ps + 7, cells);
            self.draw_health(ps);
        }
    }
}

impl EraCombatState {
    pub fn projectiles(&self) -> &[EraProjectileState; PROJECTILE_SLOTS] {
        &self.projectiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_power_recovers_only_after_time_unguarded() {
        let mut guard = GuardPower::full(Loadout::AIsm);
        assert!(!guard.block(Loadout::AIsm, 8));
        let depleted = guard.remaining;
        for _ in 0..2 * GUARD_RECOVERY_DELAY {
            guard.tick(true);
        }
        for _ in 0..GUARD_RECOVERY_DELAY {
            guard.tick(false);
        }
        assert_eq!(guard.remaining, depleted);
        guard.tick(false);
        assert_eq!(guard.remaining, depleted + 1);
        for _ in 0..GUARD_RECOVERY_INTERVAL {
            guard.tick(false);
        }
        assert_eq!(guard.remaining, depleted + 2);
    }

    #[test]
    fn repeated_crushes_refill_at_no_less_than_half_capacity() {
        let loadout = Loadout::XIsm;
        let mut guard = GuardPower::full(loadout);
        for _ in 0..8 {
            assert!(guard.block(loadout, u8::MAX));
            let broken = guard;
            assert!(!guard.block(loadout, u8::MAX));
            assert_eq!(guard, broken);
            for _ in 1..GUARD_CRUSH_FRAMES {
                guard.tick(false);
                assert_eq!(guard.remaining, 0);
            }
            guard.tick(false);
            assert_eq!(guard.crush_left, 0);
            assert_eq!(guard.remaining, guard.maximum);
        }
        assert_eq!(guard.maximum, GUARD_POWER_X_ISM / 2);
    }

    #[test]
    fn alpha_guard_power_requires_a_canonical_or_enabled_what_if_style() {
        let mut fighter = EraFighterState {
            style: StylePick::for_revision(Revision::Sfz3, Loadout::AIsm),
            fighter: Some(TournamentFighter::Geki),
            sub_hp: FULL_SUB_HP,
            max_sub_hp: FULL_SUB_HP,
            ..EraFighterState::default()
        };
        fighter.set_gauge_mode(GaugeMode::Unified);
        assert!(fighter.guard_power().is_none());
        assert!(fighter.gauge().unwrap().second.is_none());

        fighter.what_if = true;
        fighter.set_gauge_mode(GaugeMode::EraDefault);
        assert!(fighter.guard_power().is_some());
        assert!(fighter.gauge().unwrap().second.is_some());
    }
}
