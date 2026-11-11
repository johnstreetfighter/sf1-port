//! Authoritative era state for the translated fight loop (ADR-0015).

use sf_video::callouts::Side;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow};

use crate::config::MatchConfig;
use crate::era::{EraFamily, GaugeMode, Loadout, Revision, StylePick, STOCK_AT};
use crate::fight_reader::player_struct;
use crate::fighter::{Bus, CPU_STRUCT, GAME_MODE};
use crate::roster::{PlayMode, TournamentFighter};

pub const SUB_HP_PER_POINT: u16 = 20;
pub const FULL_SUB_HP: u16 = 48 * SUB_HP_PER_POINT;
pub use crate::data_bank::{SF2_SPECIAL_SLOT as SF2_EXTRA_SPECIAL, SF2_SUPER_SLOT as SF2_SUPER};
pub const SUPER_FREEZE_FRAMES: u8 = 28;
const PROJECTILE_SLOTS: usize = 12;

const DOUBLE_QUARTER_CIRCLE: &[u8] = &[4, 5, 1, 4, 5, 1];
const HALF_CIRCLE: &[u8] = &[2, 6, 4, 5, 1];
const TIGER_KNEE: &[u8] = &[4, 5, 1, 9];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MotionTracker {
    position: u8,
    remaining: u8,
    previous: u8,
}

crate::canonical::canonical_struct!(MotionTracker, "EraMotionTracker.v1", {
    position, remaining, previous,
});

impl MotionTracker {
    fn step(&mut self, direction: u8, pattern: &[u8]) {
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining == 0 {
            self.position = 0;
        }
        if self.previous == direction {
            return;
        }
        self.previous = direction;
        let position = usize::from(self.position);
        if pattern.get(position) == Some(&direction) {
            self.position += 1;
            if position == 0 {
                self.remaining = 32;
            } else if usize::from(self.position) == pattern.len() {
                self.remaining = 6;
            }
        } else if direction == pattern[0] {
            self.position = 1;
            self.remaining = 32;
        }
    }

    fn completed(self, pattern: &[u8]) -> bool {
        usize::from(self.position) == pattern.len()
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
}

impl From<MatchConfig> for EraRules {
    fn from(config: MatchConfig) -> Self {
        Self {
            enabled: config.era_select_enabled() && config.dramatic.is_none(),
            gauge_mode: config.gauge_mode,
            fill_meters: config.fill_meters && config.roster.mode == PlayMode::Training,
            untechable_sf2_throws: config.sf1plus.untechable_sf2_throws,
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
    super_motion: MotionTracker,
    extra_motion: MotionTracker,
    pub(crate) move_hits: u8,
    pub(crate) juggle_count: u8,
}

crate::canonical::canonical_struct!(EraFighterState, "EraFighterState.v1", {
    style, fighter, sub_hp, max_sub_hp, super_meter, super_capacity,
    super_motion, extra_motion, move_hits, juggle_count,
});

impl EraFighterState {
    pub fn gauge(&self) -> Option<EraGauge> {
        if self.super_capacity == 0 {
            return None;
        }
        Some(EraGauge::one(GaugeRow::stock(
            self.super_capacity / STOCK_AT,
            self.super_meter / STOCK_AT,
            Fill::of(u16::from(self.super_meter % STOCK_AT), u16::from(STOCK_AT)),
        )))
    }

    pub(crate) fn is_sf2(&self) -> bool {
        self.max_sub_hp != 0
            && self.style.family == EraFamily::Sf2
            && matches!(
                self.fighter,
                Some(TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Sagat)
            )
    }

    fn st_new(&self) -> bool {
        self.is_sf2()
            && self.style.revision == Revision::St
            && self.style.loadout == Loadout::Standard
    }

    fn set_gauge_mode(&mut self, mode: GaugeMode) {
        self.super_capacity = match mode {
            GaugeMode::EraDefault if self.st_new() => STOCK_AT,
            GaugeMode::Unified if self.max_sub_hp != 0 => 3 * STOCK_AT,
            _ => 0,
        };
        self.super_meter = self.super_meter.min(self.super_capacity);
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
    projectiles: [EraProjectileState; PROJECTILE_SLOTS],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EraProjectileState {
    pub move_id: u8,
    pub hits_remaining: u8,
    pub crimson: bool,
    hit_wait: u8,
    clash_wait: u8,
}

crate::canonical::canonical_struct!(EraProjectileState, "EraProjectileState.v1", {
    move_id, hits_remaining, crimson, hit_wait, clash_wait,
});

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
            let mut next = EraFighterState {
                style,
                fighter,
                sub_hp: FULL_SUB_HP,
                max_sub_hp: FULL_SUB_HP,
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

    pub(crate) fn sf2_state(&self, ps: u32) -> Option<&EraFighterState> {
        let state = &self.toggle_state.era.fighters[self.era_side(ps)?];
        state.is_sf2().then_some(state)
    }

    pub(crate) fn chosen_era(&self, ps: u32) -> Option<EraFamily> {
        Some(
            self.toggle_state.era.fighters[self.era_side(ps)?]
                .style
                .family,
        )
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

    pub(crate) fn era_spawn_projectile(&mut self, slot: u16, ps: u32, move_id: u8) {
        self.era_clear_projectile(slot);
        let Some(era) = self.sf2_state(ps) else {
            return;
        };
        let crimson = era.fighter == Some(TournamentFighter::Ryu) && move_id == SF2_EXTRA_SPECIAL;
        let hits_remaining = if crimson {
            3
        } else if move_id == SF2_SUPER {
            5
        } else {
            1
        };
        if let Some(state) = self.era_projectile_mut(slot) {
            *state = EraProjectileState {
                move_id,
                hits_remaining,
                crimson,
                ..EraProjectileState::default()
            };
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
            return false;
        }
        self.toggle_state.era.freeze -= 1;
        true
    }

    pub(crate) fn era_meter_gain(&mut self, ps: u32, gain: u8) {
        if let Some(index) = self.era_side(ps) {
            let state = &mut self.toggle_state.era.fighters[index];
            state.super_meter = state
                .super_meter
                .saturating_add(gain)
                .min(state.super_capacity);
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
                self.era_sync_health(ps, index);
                let entity = self.rw(ps + 0x10);
                if self.eb(entity, 9) & 3 != 1 {
                    self.toggle_state.era.fighters[index].juggle_count = 0;
                }
                if self.era_rules.fill_meters
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
