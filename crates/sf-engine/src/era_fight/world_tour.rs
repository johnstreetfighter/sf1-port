//! World Tour's native mission and P1-only combat modifiers.

use super::{EraFighterState, FULL_SUB_HP, STOCK_AT};
use crate::config::WorldTourRules;
use crate::fighter::{Bus, CPU_STRUCT, GAME_MODE, MODE_DEMO, P1_STRUCT, P2_STRUCT, SCENE_MODE};
use crate::world_tour::native::{MissionKind, Perk, RuleModifier};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TourFighter {
    pub(super) perk: Option<Perk>,
    neutral_frames: u8,
    walk_remainder: i16,
    guard_pushback: bool,
    guard_remainder: i16,
    shuriken_recovering: bool,
    super_gain_remainder: u8,
    pub(super) v_gain_remainder: u8,
    pub(super) drive_gain_remainder: u8,
    pub(super) revenge_gain_remainder: u8,
    pub(super) drive_cost_remainder: u8,
}

crate::canonical::canonical_struct!(TourFighter, "TourFighter.v1", {
    perk, neutral_frames, walk_remainder, guard_pushback, guard_remainder, shuriken_recovering,
    super_gain_remainder, v_gain_remainder, drive_gain_remainder, revenge_gain_remainder, drive_cost_remainder,
});

impl TourFighter {
    pub(super) fn boost_drive_gain(&mut self, gain: u16) -> u16 {
        let surge = self.perk == Some(Perk::MeterSurge);
        meter_gain(gain, surge, &mut self.drive_gain_remainder)
    }
}

impl EraFighterState {
    pub(super) fn world_tour_perk(&self, perk: Perk) -> bool {
        self.tour.perk == Some(perk)
    }

    pub(super) fn meter_cost(&self, cost: u8) -> u8 {
        if self.world_tour_perk(Perk::IsmPlus) {
            cost - cost / 4
        } else {
            cost
        }
    }

    pub(super) fn award_super_meter(&mut self, gain: u8) {
        let gain = meter_gain(
            u16::from(gain),
            self.world_tour_perk(Perk::MeterSurge),
            &mut self.tour.super_gain_remainder,
        )
        .min(u16::from(u8::MAX)) as u8;
        self.super_meter = self
            .super_meter
            .saturating_add(gain)
            .min(self.super_capacity);
    }

    pub(super) fn world_tour_start_meter(&mut self) {
        if self.world_tour_perk(Perk::MeterSurge) {
            let stock = self.sf3_stock_cost().unwrap_or(STOCK_AT);
            self.super_meter = self
                .super_meter
                .saturating_add(stock)
                .min(self.super_capacity);
        }
    }
}

/// Keep quarter-point increments so a stream of 1-point gains is boosted too.
pub(super) fn meter_gain(gain: u16, surge: bool, remainder: &mut u8) -> u16 {
    if !surge {
        return gain;
    }
    let quarters = u32::from(gain) + u32::from(*remainder);
    *remainder = (quarters % 4) as u8;
    gain.saturating_add((quarters / 4) as u16)
}

impl Bus<'_> {
    pub(crate) fn world_tour_rules(&self) -> Option<WorldTourRules> {
        self.world_tour.filter(|_| self.rb(MODE_DEMO) != 0)
    }

    pub(crate) fn world_tour_trial(&self, rule: RuleModifier) -> bool {
        self.world_tour_rules()
            .is_some_and(|rules| rules.mission == MissionKind::RuleModifierTrial(rule))
    }

    pub(crate) fn world_tour_pure_footsies(&self) -> bool {
        self.world_tour_trial(RuleModifier::PureFootsies)
    }

    pub(crate) fn era_true_combos(&self, player: u32) -> bool {
        (player == P1_STRUCT && self.world_tour_trial(RuleModifier::ComboOnly))
            || self.era_offense_flag(player, self.toggles.true_combos)
    }

    pub(crate) fn world_tour_can_block(&self, player: u32) -> bool {
        player != P1_STRUCT || !self.world_tour_trial(RuleModifier::NoBlocking)
    }

    pub(crate) fn world_tour_perk(&self, player: u32, perk: Perk) -> bool {
        player == P1_STRUCT
            && self
                .world_tour_rules()
                .and_then(|rules| rules.bonuses)
                .is_some_and(|bonuses| bonuses.perk == Some(perk))
    }

    pub(crate) fn world_tour_damage(
        &self,
        owner: u32,
        target: u32,
        damage: u16,
        chip: bool,
    ) -> u16 {
        let Some(rules) = self.world_tour_rules() else {
            return damage;
        };
        let attack = if owner == P1_STRUCT {
            rules.bonuses.map_or(0, |b| b.attack.min(5))
        } else {
            0
        };
        let defense = if target == P1_STRUCT {
            rules.bonuses.map_or(0, |b| b.defense.min(5))
        } else {
            0
        };
        if chip && self.world_tour_perk(target, Perk::GuardMaster) {
            return 0;
        }
        let combo = if rules.mission == MissionKind::RuleModifierTrial(RuleModifier::ComboOnly)
            && owner == P1_STRUCT
            && matches!(target, P2_STRUCT | CPU_STRUCT)
        {
            let slot = crate::combo::slot_in_fight(target, self.rb(GAME_MODE), false);
            if chip || self.toggle_state.combo.hits[slot] < 2 {
                0
            } else {
                125
            }
        } else {
            100
        };
        let damage =
            u64::from(damage) * u64::from(100 + 4 * attack) * u64::from(100 - 4 * defense) * combo;
        (damage / 1_000_000).min(u64::from(u16::MAX)) as u16
    }

    pub(crate) fn world_tour_guard_reaction(&mut self, player: u32, guarding: bool) {
        if self.world_tour_perk(player, Perk::GuardMaster) {
            self.toggle_state.era.fighters[0].tour.guard_pushback = guarding;
        }
    }

    pub(crate) fn world_tour_horizontal_step(&mut self, player: u32, mv: u8, dx: i16) -> i16 {
        let Some(bonuses) = self
            .world_tour_rules()
            .and_then(|rules| rules.bonuses)
            .filter(|_| player == P1_STRUCT)
        else {
            return dx;
        };
        let state = &mut self.toggle_state.era.fighters[0];
        let tour = &mut state.tour;
        if bonuses.perk == Some(Perk::GuardMaster)
            && (matches!(mv, 4 | 8) || (matches!(mv, 0x30..=0x3f) && tour.guard_pushback))
        {
            let scaled = dx + tour.guard_remainder;
            tour.guard_remainder = scaled % 2;
            return scaled / 2;
        }
        let dash = match state.style.family {
            crate::era::EraFamily::Sf4 => matches!(mv, super::sf4::DASH | super::sf4::BACKDASH),
            crate::era::EraFamily::Sf6 => matches!(mv, super::sf6::RUSH | super::sf6::QUICK_DASH),
            _ => false,
        };
        if !matches!(mv, 2 | 3) && !dash {
            return dx;
        }
        let scaled = i32::from(dx) * (100 + 3 * i32::from(bonuses.speed.min(5)))
            + i32::from(tour.walk_remainder);
        tour.walk_remainder = (scaled % 100) as i16;
        (scaled / 100) as i16
    }

    pub(crate) fn world_tour_shuriken_recovery(&mut self, player: u32, start: bool) {
        if player == P2_STRUCT && self.world_tour_trial(RuleModifier::ProjectileDefense) {
            self.toggle_state.era.fighters[1].tour.shuriken_recovering = start;
        }
    }

    pub(crate) fn world_tour_shuriken_recovering(&self, player: u32, mv: u8) -> bool {
        player == P2_STRUCT
            && mv == 0x1a
            && self.world_tour_trial(RuleModifier::ProjectileDefense)
            && self.era_state(player).is_some_and(|state| {
                state.fighter == Some(crate::roster::TournamentFighter::Geki)
                    && state.tour.shuriken_recovering
            })
    }

    pub(crate) fn world_tour_apply_starting_hp(&mut self) {
        if let Some(hp) = self.world_tour_rules().and_then(|rules| rules.starting_hp) {
            let state = &mut self.toggle_state.era.fighters[0];
            state.sub_hp = hp.clamp(1, FULL_SUB_HP);
            self.era_sync_health(P1_STRUCT, 0);
        }
    }

    pub(super) fn world_tour_heal_neutral(&mut self, player: u32, side: usize, neutral: bool) {
        if !self.world_tour_perk(player, Perk::VitalSpirit) || self.rb(SCENE_MODE) != 0x80 {
            return;
        }
        let state = &mut self.toggle_state.era.fighters[side];
        if !neutral || state.sub_hp == 0 {
            state.tour.neutral_frames = 0;
            return;
        }
        state.tour.neutral_frames += 1;
        if state.tour.neutral_frames == 30 {
            state.tour.neutral_frames = 0;
            let hp = state
                .sub_hp
                .saturating_add(2)
                .min(state.max_sub_hp)
                .min(FULL_SUB_HP);
            if hp > state.sub_hp {
                state.sub_hp = hp;
            }
        }
    }
}
