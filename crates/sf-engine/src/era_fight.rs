//! Authoritative era state for the translated fight loop (ADR-0015).

use sf_video::callouts::{CalloutKind, Side};
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

use crate::config::{EvilRyuPlaytest, MatchConfig};
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
const ALPHA_COUNTER_GUARD_COST: u8 = GUARD_CRUSH_CAPACITY_LOSS;
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
mod world_tour;
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

const SHINKUU_HADOKEN: AlphaMove =
    AlphaMove::super_combo(SF2_SUPER, "SHINKUU HADOKEN", "SHINKUU HADOKEN!", DOUBLE_QUARTER_CIRCLE, ButtonClass::Punch);
const SHINKUU_TATSUMAKI: AlphaMove =
    AlphaMove::super_combo(ALPHA_SUPER_THIRD_SLOT, "SHINKUU TATSUMAKI", "SHINKUU TATSUMAKI!", DOUBLE_QUARTER_CIRCLE_BACK, ButtonClass::Kick);
const COLLARBONE_BREAKER: AlphaMove =
    AlphaMove::overhead_normal(0x1c, "COLLARBONE BREAKER", ButtonClass::MediumPunch);

const RYU_ALPHA_SUPERS: &[AlphaMove] = &[
    SHINKUU_HADOKEN,
    AlphaMove::super_combo(ALPHA_SUPER_SECOND_SLOT, "METSU SHORYUKEN", "METSU SHORYUKEN!", SHORT_DOUBLE_QUARTER_CIRCLE, ButtonClass::Kick).revision(ALPHA_3).level_three(),
    SHINKUU_TATSUMAKI,
    COLLARBONE_BREAKER,
];

const EVIL_RYU_ALPHA_MOVES: &[AlphaMove] = &[
    SHINKUU_HADOKEN.no_x(),
    AlphaMove::super_combo(
        ALPHA_SUPER_SECOND_SLOT, "MESSATSU GOSHORYU", "MESSATSU GOSHORYU!",
        SHORT_DOUBLE_QUARTER_CIRCLE, ButtonClass::Kick,
    ),
    SHINKUU_TATSUMAKI,
    COLLARBONE_BREAKER,
    {
        let mut entry = AlphaMove::overhead_normal(
            crate::data_bank::EVIL_RYU_ALPHA2_SENPUKYAKU, "SENPUKYAKU", ButtonClass::MediumKick,
        ).no_x();
        entry.overhead = false;
        entry
    },
    {
        let mut move_entry = AlphaMove::special(
            sf3::AIR_TATSU, "AIR TATSUMAKI",
            &[Dir::Jump, Dir::Down, Dir::DownBack, Dir::Back], ButtonClass::Kick,
        );
        move_entry.airborne = true;
        move_entry
    },
    AlphaMove::special(SF2_EXTRA_SPECIAL, "SHAKUNETSU HADOKEN", HALF_CIRCLE, ButtonClass::Punch)
        .revision(ALPHA_3),
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
            enabled: config.era_select_enabled(),
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
        let minimum = (Self::initial_maximum(loadout) / 2).min(self.maximum);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DeathMark {
    // The native EraCombatState fighter slot that applied the mark.
    source: u8,
    frames: u8,
}

crate::canonical::canonical_struct!(DeathMark, "DeathMark.v1", { source, frames });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum EvilRyuKit {
    #[default]
    Standard,
    EvilAlpha2 { custom_left: u16 },
    EvilAlpha3A,
    EvilAlpha3V,
    EvilAlpha3X,
    EvilSf4 { metsu_charge: Option<u8> },
    Kage { v: sf5::KageVState },
    EvilSf6,
}

impl crate::canonical::Canonical for EvilRyuKit {
    fn encode(&self, out: &mut crate::canonical::CanonicalHasher) {
        // Keep the existing checksum layout for the standard kit.
        match self {
            Self::Standard => {}
            Self::EvilAlpha2 { custom_left } => {
                out.tag("EvilRyuAlpha2Kit");
                custom_left.encode(out);
            }
            Self::EvilAlpha3A => out.tag("EvilRyuAlpha3AKit"),
            Self::EvilAlpha3V => out.tag("EvilRyuAlpha3VKit"),
            Self::EvilAlpha3X => out.tag("EvilRyuAlpha3XKit"),
            Self::EvilSf4 { metsu_charge } => {
                // Keep the AE2012 tag. The style separately encodes the SFIV revision.
                out.tag("EvilRyuAe2012Kit");
                metsu_charge.encode(out);
            }
            Self::Kage { v } => {
                out.tag("KageSf5CeKit");
                v.encode(out);
            }
            Self::EvilSf6 => out.tag("EvilRyuSf6Kit"),
        }
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
    pub(crate) unified_gauge: bool,
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
    death_mark: Option<DeathMark>,
    dizzy_frames: u8,
    active_alpha_slot: u8,
    active_alpha_super_level: u8,
    guard_power: GuardPower,
    pub(crate) move_hits: u8,
    pub(crate) juggle_count: u8,
    attack_reached_active: bool,
    tour: world_tour::TourFighter,
    evil_ryu_kit: EvilRyuKit,
}

crate::canonical::canonical_struct!(EraFighterState, "EraFighterState.v10", {
    style, fighter, sub_hp, max_sub_hp, super_meter, super_capacity, unified_gauge, what_if,
    super_motion, extra_motion, alpha_counter_motion, custom_frames, sf3, sf4, sf5, sf6, what_if_kit, alpha_motions,
    alpha_buttons, alpha_previous_buttons, punch_taps, punch_tap_window, pair_held, pair_released, repeated_kick, held_windup_frames,
    gen_crane, command_throw_damage, sagat_charge, charged_tiger_blow, death_mark, dizzy_frames,
    active_alpha_slot, active_alpha_super_level, guard_power, move_hits, juggle_count, attack_reached_active, tour, evil_ryu_kit,
});

impl EraFighterState {
    pub fn gauge(&self) -> Option<EraGauge> {
        self.gauge_with_trigger_timer(true)
    }

    pub(crate) fn team_gauge(&self) -> Option<EraGauge> {
        self.gauge_with_trigger_timer(false)
    }

    fn gauge_with_trigger_timer(&self, show_trigger_timer: bool) -> Option<EraGauge> {
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
        if let EvilRyuKit::EvilAlpha2 { custom_left: left @ 1.. } = self.evil_ryu_kit {
            let timer = GaugeRow::bar(Fill::of(left, u16::from(self.super_capacity) * 2), Fill::FULL);
            return Some(EraGauge::two(timer, main));
        }
        if !self.unified_gauge {
            if self.is_sf6() {
                return Some(EraGauge::two(self.sf6.row(), main));
            }
            if self.is_sf4() {
                return Some(EraGauge::two(main, self.sf4.revenge_row()));
            }
            if let Some(v_gauge) = self.sf5.gauge_row(show_trigger_timer) {
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

    pub(crate) fn evil_ryu_alpha2(&self) -> bool {
        matches!(self.evil_ryu_kit, EvilRyuKit::EvilAlpha2 { .. })
    }

    fn evil_ryu_alpha3(&self) -> bool {
        matches!(self.evil_ryu_kit, EvilRyuKit::EvilAlpha3A | EvilRyuKit::EvilAlpha3V | EvilRyuKit::EvilAlpha3X)
    }

    pub(crate) fn is_evil_ryu_sf4(&self) -> bool {
        matches!(self.evil_ryu_kit, EvilRyuKit::EvilSf4 { .. })
    }

    pub(crate) fn is_kage(&self) -> bool {
        matches!(self.evil_ryu_kit, EvilRyuKit::Kage { .. })
    }

    pub(crate) fn is_evil_ryu_sf6(&self) -> bool {
        self.evil_ryu_kit == EvilRyuKit::EvilSf6
    }

    pub fn custom_combo_active(&self) -> bool {
        self.custom_frames != 0
            || matches!(self.evil_ryu_kit, EvilRyuKit::EvilAlpha2 { custom_left: 1.. })
    }

    fn clear_custom_combo(&mut self) {
        self.custom_frames = 0;
        if let EvilRyuKit::EvilAlpha2 { custom_left } = &mut self.evil_ryu_kit {
            *custom_left = 0;
        }
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
        } else if self.sf6_evil_ryu_ashura(self.active_alpha_slot) {
            Some(0xffb48edf)
        } else if self.sf6_trail() {
            Some(0xff59d97d)
        } else {
            None
        }
    }

    pub(crate) fn custom_combo_available(&self) -> bool {
        self.is_alpha() && self.super_capacity != 0
            && (self.evil_ryu_alpha2()
                || (self.style.revision == Revision::Sfz3 && self.style.loadout == Loadout::VIsm))
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
            Some(TournamentFighter::Ryu) if self.evil_ryu_kit != EvilRyuKit::Standard => EVIL_RYU_ALPHA_MOVES,
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
        (self.is_kage() && matches!(slot, 0x19..=0x1b))
            || self.alpha_roster().iter().any(|entry| entry.kind == MoveKind::Special && entry.slot == slot)
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
            self.clear_custom_combo();
        }
    }

    pub(crate) fn alpha_counter_enabled(&self) -> bool {
        self.is_alpha() && self.super_capacity != 0 && self.style.loadout != Loadout::XIsm
    }

    fn alpha_counter_ready(&self) -> bool {
        if !self.alpha_counter_enabled()
            || (self.evil_ryu_alpha2() && self.custom_combo_active())
            || self.guard_power.crush_left != 0
            || self.super_meter < self.meter_cost(STOCK_AT)
        {
            return false;
        }
        if self.evil_ryu_alpha3() {
            let held = self.alpha_buttons;
            let fresh = held & !self.alpha_previous_buttons;
            self.guard_power.maximum >= 2 * ALPHA_COUNTER_GUARD_COST
                && self.guard_power.remaining > ALPHA_COUNTER_GUARD_COST
                && [0x09, 0x12, 0x24].into_iter().any(|pair| held & pair == pair && fresh & pair != 0)
        } else {
            self.alpha_counter_motion.ready(ALPHA_COUNTER_MOTION)
        }
    }

    pub(crate) fn set_super_meter(&mut self, value: Option<u8>) {
        self.super_meter = value
            .unwrap_or(self.super_capacity)
            .min(self.super_capacity);
    }

    fn apply_satsui_surge(&mut self, previous: Self) {
        if self.super_capacity == 0 {
            return;
        }
        if self.unified_gauge {
            self.set_super_meter(Some(self.super_meter.saturating_add(STOCK_AT)));
        } else if self.is_sf4() {
            let half_ultra = sf4::REVENGE_MAX / 2;
            self.sf4.revenge = (previous.sf4.revenge + half_ultra).min(sf4::REVENGE_MAX);
        } else if self.style.family == EraFamily::Sf5 {
            self.sf5.apply_satsui_surge(previous.sf5);
        } else if !self.is_sf6() {
            let stock = self.sf3_stock_cost().unwrap_or(STOCK_AT);
            self.set_super_meter(Some(self.super_meter.saturating_add(stock)));
        }
        // SF6's native round setup already fills all six Drive stocks.
    }

    pub(crate) fn super_ready(&self) -> bool {
        self.super_stock_full() && self.super_motion.completed(DOUBLE_QUARTER_CIRCLE)
    }

    pub(crate) fn super_stock_full(&self) -> bool {
        self.st_new() && self.super_capacity > 0 && self.super_meter >= self.meter_cost(self.super_capacity)
    }

    fn spend_super_stock(&mut self) {
        self.super_meter -= self.meter_cost(self.super_capacity);
        self.super_motion = MotionTracker::default();
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
    pub fighters: [EraFighterState; 3],
    arcade_step: Option<u16>,
    pub(crate) freeze: u8,
    pub(crate) super_events: [Option<TournamentFighter>; 3],
    pub(crate) alpha_super_events: [Option<&'static str>; 3],
    pub(crate) alpha_counter_events: [bool; 3],
    pub(crate) guard_crush_events: [bool; 3],
    pub(crate) style_callouts: [Option<CalloutKind>; 3],
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
        out.tag("EraCombatState.v2");
        selections.encode(out);
        fighters.encode(out);
        arcade_step.encode(out);
        freeze.encode(out);
        projectiles.encode(out);
    }
}

impl Bus<'_> {
    fn era_player_slots(&self) -> [Option<u32>; 3] {
        if self.dramatic_rules().is_some() {
            return crate::fighter::DRAMATIC_PLAYERS.map(Some);
        }
        let mode = self.rb(GAME_MODE);
        [Some(player_struct(Side::P1, mode)), Some(player_struct(Side::P2, mode)), None]
    }

    fn era_shared_gauge(&self, index: usize) -> Option<usize> {
        use crate::dramatic::LifebarMode;

        (matches!(index, 0 | 2)
            && self.dramatic_rules().is_some_and(|rules| rules.lifebar == LifebarMode::Shared))
            .then(|| 2 - index)
    }

    pub(crate) fn era_gauge_mode(&self, index: usize) -> GaugeMode {
        if self.era_shared_gauge(index).is_some()
            && self.dramatic_rules().is_some_and(|rules| {
                rules.styles[0].family() != rules.styles[2].family()
            })
        {
            GaugeMode::Unified
        } else {
            self.era_rules.gauge_mode
        }
    }

    /// Keep both views of the shared gauge current before the next fighter
    /// acts. Temporary move effects and motion history stay fighter-local.
    fn era_share_gauge(&mut self, index: usize) {
        if self.era_shared_gauge(index).is_none() {
            return;
        }
        let [ally1, _, ally2] = &mut self.toggle_state.era.fighters;
        let (source, target) = if index == 0 { (ally1, ally2) } else { (ally2, ally1) };
        target.super_meter = source.super_meter;
        if !source.unified_gauge {
            target.sf4.revenge = source.sf4.revenge;
            target.sf5.share_meter(source.sf5);
            target.sf6.share_meter(source.sf6);
        }
    }

    pub(crate) fn era_begin_round(&mut self) {
        use TournamentFighter::{Adon, Birdie, Geki, Gen, Mike, Ryu, Sagat};

        if !self.era_rules.enabled || self.rb(0xff8003) == 0 {
            return;
        }
        let arcade = self.roster.mode == PlayMode::Arcade;
        let dramatic = self.dramatic_rules();
        let tour = self.world_tour_rules();
        let state = self.toggle_state.era;
        let awakening = self.toggle_state.satsui_awakening;
        let step = arcade.then(|| u16::from(self.rb(0xffc003)) * 2 + u16::from(self.rb(0xffc004)));
        let new_match = step != state.arcade_step;
        self.toggle_state.era.arcade_step = step;
        self.toggle_state.era.freeze = 0;
        self.toggle_state.era.super_events = [None; 3];
        self.toggle_state.era.alpha_super_events = [None; 3];
        self.toggle_state.era.alpha_counter_events = [false; 3];
        self.toggle_state.era.guard_crush_events = [false; 3];
        self.toggle_state.era.style_callouts = [None; 3];
        self.toggle_state.era.projectiles = [EraProjectileState::default(); PROJECTILE_SLOTS];
        for (index, ps) in self.era_player_slots().into_iter().enumerate() {
            let Some(ps) = ps else { continue };
            let fighter = TournamentFighter::from_move_table(self.rl(ps + 0x12));
            let style = if index == 0 && tour.is_some() {
                state.selections[0]
            } else if let Some(rules) = dramatic {
                crate::dramatic::DramaticRules::style_pick(rules.styles[index])
            } else if arcade {
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
            let awakened = awakening.filter(|form| form.side.fighter_struct() == ps);
            let profile = awakened.map(|form| form.profile)
                .or_else(|| self.evil_ryu_playtest.filter(|_| ps == crate::fighter::P1_STRUCT))
                .filter(|_| dramatic.is_none() && fighter == Some(Ryu));
            let kage = style.revision == Revision::Sf5Ce && matches!(
                (profile, style.loadout),
                (Some(EvilRyuPlaytest::Kage), Loadout::Skill1Trigger1)
                    | (Some(EvilRyuPlaytest::KageII), Loadout::Skill2Trigger2)
            );
            let hp = if dramatic.is_some() {
                self.toggle_state.dramatic.health[index]
            } else if index == 0 && tour.is_some_and(|rules| matches!(rules.mission, crate::world_tour::native::MissionKind::SurvivalGauntlet { .. })) {
                FULL_SUB_HP
            } else {
                match (style.family, fighter) {
                    // Champion Edition Kage has 925 of the standard 1000 vitality.
                    _ if kage => FULL_SUB_HP * 37 / 40,
                    _ if profile.is_some() => 864,
                    (EraFamily::Sf3 | EraFamily::Sf4 | EraFamily::Sf5 | EraFamily::Sf6, fighter) => match fighter {
                        Some(Gen) => 864,
                        Some(Adon | Geki) => 912,
                        Some(Sagat | Mike | Birdie) => 1008,
                        _ => FULL_SUB_HP,
                    },
                    _ => FULL_SUB_HP,
                }
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
                evil_ryu_kit: match (profile, style.revision, style.loadout) {
                    _ if kage => EvilRyuKit::Kage { v: sf5::KageVState::default() },
                    (Some(EvilRyuPlaytest::Sf6), Revision::Sf6, Loadout::Standard) => EvilRyuKit::EvilSf6,
                    (Some(EvilRyuPlaytest::Alpha2), Revision::Sfz2, _) => EvilRyuKit::EvilAlpha2 { custom_left: 0 },
                    (Some(EvilRyuPlaytest::Alpha3A), Revision::Sfz3, Loadout::AIsm) => EvilRyuKit::EvilAlpha3A,
                    (Some(EvilRyuPlaytest::Alpha3V), Revision::Sfz3, Loadout::VIsm) => EvilRyuKit::EvilAlpha3V,
                    (Some(EvilRyuPlaytest::Alpha3X), Revision::Sfz3, Loadout::XIsm) => EvilRyuKit::EvilAlpha3X,
                    (Some(EvilRyuPlaytest::Ae2012 | EvilRyuPlaytest::Ae2012Ultra2), Revision::Ae2012, Loadout::Ultra1 | Loadout::Ultra2)
                    | (Some(EvilRyuPlaytest::Usf4), Revision::Usf4, Loadout::Ultra1 | Loadout::Ultra2 | Loadout::UltraDouble) => {
                        EvilRyuKit::EvilSf4 { metsu_charge: None }
                    }
                    _ => EvilRyuKit::Standard,
                },
                ..EraFighterState::default()
            };
            next.set_gauge_mode(self.era_gauge_mode(index));
            if index == 0 {
                next.tour.perk = tour.and_then(|rules| rules.bonuses).and_then(|bonuses| bonuses.perk);
            }
            if index == 2 && self.era_shared_gauge(index).is_some() {
                next.super_capacity = self.toggle_state.era.fighters[0].super_capacity;
            }
            let previous = state.fighters[index];
            next.super_meter = if self.era_rules.fill_meters {
                next.super_capacity
            } else if !new_match && previous.style == style && previous.fighter == fighter {
                previous.super_meter.min(next.super_capacity)
            } else {
                0
            };
            next.world_tour_start_meter();
            if let Some(form) = awakened.filter(|form| form.surge_pending && profile.is_some()) {
                next.apply_satsui_surge(previous);
                self.toggle_state.satsui_awakening = Some(crate::config::SatsuiAwakening {
                    surge_pending: false,
                    ..form
                });
            }
            self.toggle_state.era.fighters[index] = next;
        }
        self.era_share_gauge(0);
    }

    pub(crate) fn era_reconfigure(&mut self, was_enabled: bool) {
        for (index, ps) in self.era_player_slots().into_iter().enumerate() {
            let Some(ps) = ps else { continue };
            let mode = self.era_gauge_mode(index);
            let native_points = u16::from(self.rb(ps + 7)) * 4 + u16::from(self.rb(ps + 6));
            let fighter = &mut self.toggle_state.era.fighters[index];
            if !self.era_rules.enabled {
                fighter.set_gauge_mode(GaugeMode::NoGauges);
                fighter.super_motion = MotionTracker::default();
                fighter.extra_motion = MotionTracker::default();
            } else {
                if !was_enabled && fighter.max_sub_hp != 0 {
                    fighter.sub_hp = (native_points * SUB_HP_PER_POINT).min(fighter.max_sub_hp);
                }
                fighter.set_gauge_mode(mode);
            }
        }
        if self.era_shared_gauge(0).is_some() {
            let [ally1, _, ally2] = &mut self.toggle_state.era.fighters;
            ally2.super_capacity = ally1.super_capacity;
            self.era_share_gauge(0);
        }
        if !self.era_rules.enabled {
            let era = &mut self.toggle_state.era;
            era.freeze = 0;
            era.super_events = [None; 3];
            era.alpha_super_events = [None; 3];
            era.alpha_counter_events = [false; 3];
            era.guard_crush_events = [false; 3];
            era.style_callouts = [None; 3];
            era.projectiles = [EraProjectileState::default(); PROJECTILE_SLOTS];
        }
    }

    pub(crate) fn era_side(&self, ps: u32) -> Option<usize> {
        if !self.era_rules.enabled {
            return None;
        }
        self.era_player_slots().into_iter().position(|slot| slot == Some(ps))
            .filter(|&index| self.toggle_state.era.fighters[index].max_sub_hp != 0)
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
            .or_else(|| era.sf6_evil_ryu_projectile_hits(move_id))
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
        state.spend_super_stock();
        self.toggle_state.era.freeze = SUPER_FREEZE_FRAMES;
        self.toggle_state.era.super_events[index] = state.fighter;
        self.era_share_gauge(index);
    }

    pub(crate) fn try_spend_demon_super(&mut self, ps: u32) -> bool {
        let Some(index) = self.era_side(ps) else { return false };
        let profile = self.evil_ryu_profile(ps);
        let state = &mut self.toggle_state.era.fighters[index];
        match (profile, state.evil_ryu_kit) {
            (Some(EvilRyuPlaytest::UltraSf2), _) if state.super_stock_full() => state.spend_super_stock(),
            (Some(EvilRyuPlaytest::KageII), EvilRyuKit::Kage { .. }) => {
                if !state.sf5_spend_kage_demon() {
                    return false;
                }
                self.era_share_gauge(index);
                self.era_emit_callout(ps, CalloutKind::Super { name: "SHUN GOKU SATSU" });
            }
            (Some(EvilRyuPlaytest::Sf6), EvilRyuKit::EvilSf6) => {
                if !state.sf6_spend_evil_ryu_demon(self.era_rules.disable_comeback_scaling) {
                    return false;
                }
                self.era_share_gauge(index);
                self.toggle_state.era.freeze = SUPER_FREEZE_FRAMES;
                self.era_emit_callout(ps, CalloutKind::Super { name: "SHUN GOKU SATSU" });
            }
            (Some(playtest), EvilRyuKit::EvilSf4 { .. }) if playtest.is_sf4() => {
                let cost = state.sf4_super_cost();
                if state.super_meter < cost {
                    return false;
                }
                state.super_meter -= cost;
                state.sf4.reset_command_motions();
            }
            (Some(EvilRyuPlaytest::Alpha2), EvilRyuKit::EvilAlpha2 { custom_left: 0 })
            | (Some(EvilRyuPlaytest::Alpha3A), EvilRyuKit::EvilAlpha3A)
            | (Some(EvilRyuPlaytest::Alpha3X), EvilRyuKit::EvilAlpha3X) => {
                let Some(cost) = state.alpha_super_cost(3).filter(|&cost| state.super_meter >= cost) else {
                    return false;
                };
                state.super_meter -= cost;
                state.alpha_motions = [MotionTracker::default(); ALPHA_MOTION_SLOTS];
            }
            _ => return false,
        }
        true
    }

    pub(crate) fn era_read_buttons(&mut self, ps: u32, buttons: u8) {
        if let Some(side) = self.era_side(ps).filter(|&i| {
            let state = &self.toggle_state.era.fighters[i];
            state.is_alpha() || state.uses_family_bank()
        }) {
            let state = &mut self.toggle_state.era.fighters[side];
            state.alpha_buttons = buttons;
            state.sf4_keep_metsu_hold(buttons);
            state.sf5_keep_senha_hold(buttons);
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
        let alpha2_custom = state.evil_ryu_alpha2() && state.custom_combo_active();
        state.alpha_roster().iter().enumerate().find_map(|(index, entry)| {
            let tracker = state.alpha_motions[index];
            let direction_ready = if entry.kind == MoveKind::Normal {
                !normal && tracker.previous == Dir::Forward.bits()
            } else {
                tracker.ready(entry.motion())
            };
            if (alpha2_custom && entry.kind == MoveKind::Super)
                || !entry.available(state, false) || entry.airborne != airborne
                || (self.world_tour_trial(crate::world_tour::native::RuleModifier::PureFootsies) && entry.kind != MoveKind::Normal)
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

    pub(crate) fn era_begin_alpha2_feint(&mut self, ps: u32) {
        let feint = AlphaMove::special(
            crate::data_bank::EVIL_RYU_ALPHA2_FAKE_HADOKEN,
            "HADO NO KAMAE",
            &[Dir::Down, Dir::DownForward, Dir::Forward],
            ButtonClass::DirectionOnly,
        );
        self.era_begin_alpha_move(ps, feint, 0);
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
            let cost = state.meter_cost(state.alpha_super_cost(level).expect("recognized Alpha Super"));
            state.super_meter -= cost;
            self.toggle_state.era.freeze = SUPER_FREEZE_FRAMES;
            self.toggle_state.era.alpha_super_events[index] = Some(entry.name);
            self.era_share_gauge(index);
        }
    }

    pub(crate) fn era_emit_callout(&mut self, ps: u32, kind: CalloutKind) {
        if let Some(side) = self.era_side(ps) {
            self.toggle_state.era.style_callouts[side] = Some(kind);
        }
    }

    pub(crate) fn era_begin_custom_combo(&mut self, ps: u32) -> bool {
        let Some(side) = self.era_side(ps) else { return false };
        let unified = self.era_gauge_mode(side) == GaugeMode::Unified;
        let state = &mut self.toggle_state.era.fighters[side];
        let minimum = state.meter_cost(if unified && !state.evil_ryu_alpha2() { 2 * STOCK_AT } else { STOCK_AT });
        if !state.custom_combo_available() || state.custom_combo_active() || state.super_meter < minimum {
            return false;
        }
        if let EvilRyuKit::EvilAlpha2 { custom_left } = &mut state.evil_ryu_kit {
            // Convert the whole charge using the engine's two frames per point.
            *custom_left = u16::from(std::mem::take(&mut state.super_meter)) * 2;
        } else {
            let points = if unified {
                state.super_meter -= minimum;
                minimum
            } else {
                state.super_meter
            };
            state.custom_frames = points * 2;
        }
        state.alpha_motions = [MotionTracker::default(); ALPHA_MOTION_SLOTS];
        self.era_stage_banked_move(ps, crate::data_bank::ALPHA_COUNTER_SLOT, 0, None);
        self.toggle_state.era.freeze = 8;
        self.era_emit_callout(ps, CalloutKind::CustomCombo);
        self.era_share_gauge(side);
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
        let Some(target) = self.era_side(target) else { return };
        let mode = self.era_gauge_mode(target);
        let source = self.era_side(owner).filter(|_| {
            move_id == 0x2a && matches!(level, 1..=3)
                && self.alpha_state(owner).is_some_and(|state| state.fighter == Some(TournamentFighter::Gen))
        });
        let states = &mut self.toggle_state.era.fighters;
        states[target].dizzy_frames = 0;
        if states[target].custom_combo_active() {
            states[target].clear_custom_combo();
            if mode != GaugeMode::Unified {
                states[target].super_meter = 0;
            }
        }
        // A clean hit on a marking fighter breaks every mark they own.
        // Other Gen fighters may have live marks in a Dramatic match.
        for state in states.iter_mut() {
            if state.death_mark.is_some_and(|mark| usize::from(mark.source) == target) {
                state.death_mark = None;
            }
        }
        if let Some(source) = source {
            states[target].death_mark = Some(DeathMark { source: source as u8, frames: 240 - 60 * level });
        }
        self.era_share_gauge(target);
    }

    pub(crate) fn era_track_alpha_counter(&mut self, ps: u32, relative_direction: u8) {
        if let Some(index) = self.era_side(ps) {
            let state = &mut self.toggle_state.era.fighters[index];
            if state.alpha_counter_enabled() && !state.evil_ryu_alpha3() {
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
        let pressure = if self.world_tour_perk(ps, crate::world_tour::native::Perk::GuardMaster) {
            pressure.div_ceil(2)
        } else {
            pressure
        };
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
        state.super_meter -= state.meter_cost(STOCK_AT);
        if state.evil_ryu_alpha3() {
            state.guard_power.maximum -= ALPHA_COUNTER_GUARD_COST;
            state.guard_power.remaining -= ALPHA_COUNTER_GUARD_COST;
        }
        state.alpha_counter_motion = MotionTracker::default();
        self.toggle_state.era.alpha_counter_events[index] = true;
        self.era_share_gauge(index);
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
                state.award_super_meter(gain);
                self.era_share_gauge(index);
            }
        }
    }

    pub(crate) fn era_damage(&mut self, ps: u32, damage: u16, preserve_one: bool) -> bool {
        let Some(index) = self.era_side(ps) else {
            return false;
        };
        if self.dramatic_rules().is_some() {
            let hp = self.toggle_state.dramatic.health[index];
            if hp != 0 {
                self.dramatic_set_health(ps, hp.saturating_sub(damage).max(u16::from(preserve_one)));
            }
            return true;
        }
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

    fn era_reconcile_health(&mut self, ps: u32, index: usize) {
        if self.dramatic_rules().is_some() {
            let hp = self.toggle_state.era.fighters[index].sub_hp;
            if hp != self.toggle_state.dramatic.health[index] {
                self.dramatic_set_health(ps, hp);
            }
        }
    }

    pub(crate) fn era_sync_frame(&mut self) {
        let dramatic = self.dramatic_rules().is_some();
        for ps in self.era_player_slots().into_iter().flatten() {
            if let Some(index) = self.era_side(ps) {
                let gauge_mode = self.era_gauge_mode(index);
                if dramatic {
                    let hp = self.toggle_state.dramatic.health[index];
                    self.toggle_state.era.fighters[index].sub_hp = hp;
                    if hp == 0 { continue; }
                }
                self.era_sf4_tick(ps);
                let tour_player = ps == crate::fighter::P1_STRUCT && self.world_tour_rules().is_some();
                if !dramatic && !tour_player {
                    self.era_sync_health(ps, index);
                }
                let entity = self.rw(ps + 0x10);
                let guarding = self.eb(entity, 9) & 0x20 != 0 || matches!(self.eb(entity, 0xa), 4 | 8);
                let ready = self.eb(entity, 9) & 0xc1 == 0 && self.eb(entity, 8) == 2;
                let neutral = ready && self.eb(entity, 0xa) == 1 && self.pad_directions(ps) == 0;
                self.world_tour_heal_neutral(ps, index, neutral);
                // Shared Drive recovers once per frame and a held parry
                // on either ally pauses that recovery.
                let drive_tick = self.era_shared_gauge(index).map_or(true, |partner| {
                    index == 0 && !self.toggle_state.era.fighters[partner].sf6.parrying()
                });
                let state = &mut self.toggle_state.era.fighters[index];
                state.sf3_tick();
                state.what_if_tick();
                state.sf6.tick(self.era_rules.fill_meters, ready, &mut state.sub_hp, state.max_sub_hp, drive_tick, &mut state.tour);
                state.sf5.tick(self.era_rules.fill_meters, ready, &mut state.sub_hp, state.max_sub_hp);
                if state.is_sf4() && self.era_rules.fill_meters && ready && state.super_capacity != 0 {
                    state.sf4.fill();
                }
                let was_custom = state.custom_combo_active();
                if let EvilRyuKit::EvilAlpha2 { custom_left } = &mut state.evil_ryu_kit {
                    *custom_left = custom_left.saturating_sub(1);
                } else {
                    state.custom_frames = state.custom_frames.saturating_sub(1);
                    if was_custom && state.custom_frames % 2 == 0 && gauge_mode != GaugeMode::Unified {
                        state.super_meter = state.super_meter.saturating_sub(1);
                    }
                }
                state.guard_power.tick(guarding);
                state.dizzy_frames = state.dizzy_frames.saturating_sub(1);
                if let Some(mark) = &mut state.death_mark {
                    mark.frames -= 1;
                    if mark.frames == 0 {
                        state.death_mark = None;
                        if state.sub_hp != 0 {
                            state.dizzy_frames = 60;
                        }
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
                self.era_share_gauge(index);
                self.era_reconcile_health(ps, index);
                if !dramatic && tour_player {
                    self.era_sync_health(ps, index);
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
    fn shared_sf5_gauge_shows_spendable_meter_during_one_allys_trigger() {
        let mut fighter = EraFighterState {
            style: StylePick::for_revision(Revision::Sf5Ce, Loadout::Skill1Trigger1),
            fighter: Some(TournamentFighter::Ryu),
            sub_hp: FULL_SUB_HP,
            max_sub_hp: FULL_SUB_HP,
            ..EraFighterState::default()
        };
        fighter.set_gauge_mode(GaugeMode::EraDefault);
        fighter.set_sf5_practice_points((STOCK_AT, 30, 40));

        let stock = GaugeRow::stock(2, 1, Fill::of(0, STOCK_AT.into()));
        assert_eq!(fighter.team_gauge().unwrap().second, Some(stock));
        assert_ne!(fighter.gauge().unwrap().second, Some(stock));
    }

    #[test]
    fn dramatic_death_marks_clear_by_source_slot() {
        use crate::config::RulesetPreset;
        use crate::dramatic::{DramaticRules, LifebarMode};
        use crate::fighter::{Program, P1_STRUCT, P2_STRUCT};
        use crate::machine::MachineState;
        use TournamentFighter::{Gen, Retsu, Ryu};

        let mut program = Program::power_on(vec![]);
        program.match_config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
        program.match_config.dramatic = Some(DramaticRules {
            co_op: true,
            ally2: Gen,
            stage: Retsu,
            lifebar: LifebarMode::Independent,
            friendly_fire: true,
            styles: [Revision::Sf1Plus, Revision::Sfz3, Revision::Sfz3],
            rivalry: None,
        });
        let mut machine = MachineState::new(vec![], vec![], vec![], 0, 0, 0, false, 0xff, 0);
        let mut bus = program.bus(&mut machine);
        bus.wb(GAME_MODE, 2);
        bus.wb(0xff8003, 1);
        for (index, fighter) in [Ryu, Gen, Gen].into_iter().enumerate() {
            bus.toggle_state.era.fighters[index] = EraFighterState {
                fighter: Some(fighter),
                style: DramaticRules::style_pick(if fighter == Gen { Revision::Sfz3 } else { Revision::Sf1Plus }),
                sub_hp: FULL_SUB_HP,
                max_sub_hp: FULL_SUB_HP,
                ..EraFighterState::default()
            };
        }

        let marks = |bus: &Bus<'_>| bus.toggle_state.era.fighters.map(|state| state.death_mark.map(|mark| mark.source));
        bus.era_alpha_damage_effects(CPU_STRUCT, P1_STRUCT, 0x2a, 3);
        assert_eq!(marks(&bus), [Some(1), None, None]);
        bus.era_alpha_damage_effects(P2_STRUCT, CPU_STRUCT, 0x2a, 3);
        assert_eq!(marks(&bus), [None, Some(2), None]);
        bus.era_alpha_damage_effects(CPU_STRUCT, P1_STRUCT, 0x2a, 3);
        bus.era_alpha_damage_effects(P1_STRUCT, CPU_STRUCT, 0x0e, 0);
        assert_eq!(marks(&bus), [None, Some(2), None]);
        bus.era_alpha_damage_effects(P1_STRUCT, P2_STRUCT, 0x0e, 0);
        assert_eq!(marks(&bus), [None, None, None]);
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
