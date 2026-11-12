//! Drive and Super Arts on native 1987 moves.

use super::*;
use crate::moves::{Availability, MoveEntry, MoveInput, Recognizer};
use ButtonClass::*;
use Dir::*;
use TournamentFighter::{Ken, Retsu, Ryu};

pub(crate) const IMPACT: u8 = 0x2f;
pub(crate) const REVERSAL: u8 = 0x2e;
pub(crate) const PARRY: u8 = 0x2d;
pub(crate) const RUSH: u8 = 0x2c;
const SA1: u8 = 0x29;
const SA2: u8 = 0x2a;
const SA3: u8 = 0x2b;
const OD_PROJECTILE: u8 = 0x24;
const OD_UPPERCUT: u8 = 0x25;
const OD_KICK: u8 = 0x26;
const OD_EXTRA: u8 = 0x22;
const EXTRA: u8 = 0x1f;
const INSTALL: u8 = 0x20;
const JINRAI: u8 = 0x21;
pub(crate) const QUICK_DASH: u8 = 0x1c;
const JINRAI_LOW: u8 = 0x23;
pub(crate) const JINRAI_MID: u8 = 0x1e;
const JINRAI_OVERHEAD: u8 = 0x1d;
const TOBIGERI: u8 = 0x19;
const INSTALL_INPUT: &[Dir] = &[Down, Neutral, Down];
const FULL_DRIVE: u16 = 6 * STOCK_AT as u16;
const QCF: &[Dir] = &[Down, DownForward, Forward];
const QCB: &[Dir] = &[Down, DownBack, Back];
const DP: &[Dir] = &[Forward, Down, DownForward];
const DASH: &[Dir] = &[Forward, Neutral, Forward];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DriveState {
    points: u16,
    enabled: bool,
    burnout: bool,
    regen_delay: u8,
    parry_age: u8,
    parry_flash: u8,
    perfect_scale: u8,
    armor: u8,
    armored_hit: bool,
    recoverable_hp: u16,
    recovery_delay: u8,
    reversal_left: u8,
    rush_left: u8,
    rush_bonus: bool,
    od_flash: u8,
    wall_left: u8,
    installed: bool,
    charged_move: u8,
    jinrai_left: u8,
    forward_window: u8,
    previous_direction: u8,
    motions: [MotionTracker; 6],
}

crate::canonical::canonical_struct!(DriveState, "Sf6Drive.v1", {
    points, enabled, burnout, regen_delay, parry_age, parry_flash, perfect_scale, armor, armored_hit, recoverable_hp, recovery_delay,
    reversal_left, rush_left, rush_bonus, od_flash, wall_left, installed, charged_move, jinrai_left, forward_window, previous_direction, motions,
});

impl DriveState {
    pub(super) fn configure(&mut self, enabled: bool) {
        if !enabled {
            *self = Self::default();
        } else if !self.enabled {
            self.enabled = true;
            self.points = FULL_DRIVE;
        }
    }

    pub(super) fn row(&self) -> GaugeRow {
        let stock = u16::from(STOCK_AT);
        let row = GaugeRow::stock(
            6,
            (self.points / stock) as u8,
            Fill::of(self.points % stock, stock),
        );
        if self.burnout {
            row.with_state(GaugeState::Greyed)
        } else {
            row
        }
    }

    fn recover(&mut self, gain: u16) {
        self.points = self.points.saturating_add(gain).min(FULL_DRIVE);
        if self.points == FULL_DRIVE {
            self.burnout = false;
        }
    }

    pub(super) fn parrying(self) -> bool {
        self.parry_age != 0
    }

    pub(super) fn share_meter(&mut self, source: Self) {
        self.points = source.points;
        self.burnout = source.burnout;
        self.regen_delay = source.regen_delay;
        if self.burnout {
            self.parry_age = 0;
        }
    }

    pub(super) fn tick(&mut self, fill: bool, ready: bool, hp: &mut u16, max_hp: u16, meter_tick: bool, tour: &mut super::world_tour::TourFighter) {
        self.parry_flash = self.parry_flash.saturating_sub(1);
        self.perfect_scale = self.perfect_scale.saturating_sub(1);
        self.reversal_left = self.reversal_left.saturating_sub(1);
        self.rush_left = self.rush_left.saturating_sub(1);
        self.od_flash = self.od_flash.saturating_sub(1);
        self.wall_left = self.wall_left.saturating_sub(1);
        self.jinrai_left = self.jinrai_left.saturating_sub(1);
        self.recovery_delay = self.recovery_delay.saturating_sub(1);
        if self.recovery_delay == 0 && self.recoverable_hp != 0 {
            let gain = self.recoverable_hp.min(3);
            *hp = hp.saturating_add(gain).min(max_hp);
            self.recoverable_hp -= gain;
        }
        if !self.enabled || !meter_tick {
            return;
        }
        if fill && ready {
            self.recover(FULL_DRIVE);
        } else if self.parry_age == 0 {
            self.regen_delay = self.regen_delay.saturating_sub(1);
            if self.regen_delay == 0 {
                self.recover(tour.boost_drive_gain(1));
            }
        }
    }
}

fn available(fighter: Option<TournamentFighter>) -> bool {
    matches!(fighter, Some(Ryu | Ken | Retsu))
}

fn arts(fighter: TournamentFighter) -> [(&'static str, ButtonClass, usize); 3] {
    match fighter {
        Ken => [
            ("DRAGONLASH FLAME", Kick, 0),
            ("SHIPPU JINRAIKYAKU", Kick, 1),
            ("SHINRYU REPPA", Punch, 0),
        ],
        Retsu => [
            ("SHIN KIAIHOU", Punch, 0),
            ("KIAI RENBU", Punch, 1),
            ("TENCHI SHORINJI DROP", Kick, 0),
        ],
        _ => [
            ("SHINKU HADOKEN", Punch, 0),
            ("SHIN HASHOGEKI", Punch, 1),
            ("SHIN SHORYUKEN", Kick, 0),
        ],
    }
}

fn extra(fighter: TournamentFighter) -> (&'static str, ButtonClass, usize) {
    match fighter {
        Ken => ("DRAGONLASH KICK", Kick, 4),
        _ => ("HASHOGEKI", Punch, 3),
    }
}

fn motion(index: usize) -> &'static [Dir] {
    [
        DOUBLE_QUARTER_CIRCLE,
        DOUBLE_QUARTER_CIRCLE_BACK,
        QCF,
        QCB,
        DP,
        INSTALL_INPUT,
    ][index]
}

impl EraFighterState {
    pub(crate) fn is_sf6(&self) -> bool {
        self.style.family == EraFamily::Sf6
            && self.max_sub_hp != 0
            && (available(self.fighter) || self.what_if_active())
    }

    pub(crate) fn sf6_burnout(&self) -> bool {
        self.is_sf6() && self.sf6.burnout
    }

    pub fn sf6_drive_points(&self) -> Option<u16> {
        self.is_sf6().then_some(self.sf6.points)
    }

    pub(crate) fn set_sf6_practice_drive(&mut self, points: Option<u16>) {
        if self.sf6.enabled {
            self.sf6.points = points.unwrap_or(FULL_DRIVE).min(FULL_DRIVE);
            self.sf6.burnout = self.sf6.points == 0;
        }
    }

    pub(crate) fn restore_sf6_practice_drive(&mut self, points: u16, burnout: bool) {
        if self.sf6.enabled {
            self.sf6.points = points.min(FULL_DRIVE);
            self.sf6.burnout = burnout;
        }
    }

    pub(super) fn sf6_trail(&self) -> bool {
        self.is_sf6() && self.sf6.rush_left != 0
    }

    pub(super) fn sf6_tint(&self) -> Option<sf_video::compositor::PaletteTint> {
        if self.sf6_burnout() {
            return Some(sf_video::compositor::PaletteTint::GREY);
        }
        let color = if self.sf6.parry_flash != 0 || self.sf6.rush_left != 0 {
            0xff64e591
        } else if self.sf6.armor != 0 {
            0xfff052aa
        } else if self.sf6.od_flash != 0 || self.sf6.installed {
            0xffffce75
        } else {
            return None;
        };
        Some(sf_video::compositor::PaletteTint {
            color,
            strength: 122,
        })
    }

    pub(crate) fn sf6_jinrai_overhead(&self, slot: u8) -> bool {
        self.is_sf6()
            && self.fighter == Some(Ken)
            && self.alpha_move_active(slot)
            && slot == JINRAI_OVERHEAD
    }

    pub(super) fn sf6_progress(&self, slot: u8) -> (u8, u8) {
        let index = match slot {
            SA1 | SA2 | SA3 => arts(self.fighter.unwrap_or(Ryu))[(slot - SA1) as usize].2,
            OD_PROJECTILE => 2,
            OD_KICK => 3,
            OD_UPPERCUT => 4,
            INSTALL => 5,
            JINRAI | TOBIGERI => 2,
            EXTRA | OD_EXTRA => extra(self.fighter.unwrap_or(Ryu)).2,
            _ => return (0, 0),
        };
        let tracker = self.sf6.motions[index];
        (tracker.position, tracker.remaining)
    }

    pub(super) fn sf6_active_moves(&self) -> Vec<MoveEntry> {
        if !self.is_sf6() {
            return Vec::new();
        }
        let fighter = self.fighter.unwrap_or(Ryu);
        let mut moves = Vec::new();
        let mut add = |name, kind, slot, motion, button| {
            moves.push(MoveEntry {
                name,
                kind,
                produces: slot,
                input: MoveInput { motion, button },
                recognizer: Recognizer::FamilyMove,
                call_out: name,
                available: Availability {
                    fighters: &[Ryu, Ken, Retsu],
                    presets: &[
                        crate::config::RulesetPreset::DefinitiveOverhaul,
                        crate::config::RulesetPreset::Custom,
                    ],
                    player_table: fighter == Retsu,
                },
            });
        };
        if fighter != Retsu {
            let (name, button, index) = extra(fighter);
            add(name, MoveKind::Special, EXTRA, motion(index), button);
        }
        match fighter {
            Ryu => add(
                "DENJIN CHARGE",
                MoveKind::Special,
                INSTALL,
                INSTALL_INPUT,
                Punch,
            ),
            Ken => {
                add("JINRAI KICK", MoveKind::Special, JINRAI, QCF, Kick);
                add("QUICK DASH", MoveKind::Special, QUICK_DASH, &[], PairKick);
            }
            Retsu => {
                add("TOBIGERI", MoveKind::Special, TOBIGERI, QCF, Kick);
                add("RENKIKO", MoveKind::Special, INSTALL, INSTALL_INPUT, Punch);
            }
            _ => {}
        }
        if self.super_capacity == 0 {
            return moves;
        }
        for (i, (name, button, index)) in arts(fighter).into_iter().enumerate() {
            add(name, MoveKind::Super, SA1 + i as u8, motion(index), button);
        }
        for (name, slot, index, button) in [
            (
                if fighter == Retsu {
                    "OD KIAI SHOT"
                } else {
                    "OD HADOKEN"
                },
                OD_PROJECTILE,
                2,
                PairPunch,
            ),
            (
                if fighter == Retsu {
                    "OD TOBIGERI"
                } else {
                    "OD SHORYUKEN"
                },
                OD_UPPERCUT,
                if fighter == Retsu { 2 } else { 4 },
                if fighter == Retsu {
                    PairKick
                } else {
                    PairPunch
                },
            ),
            (
                if fighter == Retsu {
                    "OD RENKIKO"
                } else {
                    "OD TATSUMAKI"
                },
                OD_KICK,
                if fighter == Retsu { 5 } else { 3 },
                if fighter == Retsu {
                    PairPunch
                } else {
                    PairKick
                },
            ),
        ] {
            add(name, MoveKind::Special, slot, motion(index), button);
        }
        if fighter != Retsu {
            let (_, button, index) = extra(fighter);
            let name = if fighter == Ken {
                "OD DRAGONLASH KICK"
            } else {
                "OD HASHOGEKI"
            };
            add(
                name,
                MoveKind::Special,
                OD_EXTRA,
                motion(index),
                if button.kick() { PairKick } else { PairPunch },
            );
        }
        add("DRIVE IMPACT", MoveKind::System, IMPACT, &[], HeavyPair);
        add("DRIVE PARRY", MoveKind::System, PARRY, &[], MediumPair);
        add("DRIVE RUSH", MoveKind::System, RUSH, DASH, MediumPair);
        add(
            "DRIVE REVERSAL",
            MoveKind::System,
            REVERSAL,
            &[Forward],
            HeavyPair,
        );
        moves
    }
}

impl Bus<'_> {
    pub(crate) fn sf6_spend(&mut self, ps: u32, cost: u8) -> bool {
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let mode = self.era_gauge_mode(side);
        let state = &mut self.toggle_state.era.fighters[side];
        if mode == GaugeMode::Unified {
            if let Some(meter) = state.super_meter.checked_sub(state.meter_cost(STOCK_AT)) {
                state.super_meter = meter;
                self.era_share_gauge(side);
                return true;
            }
            return false;
        }
        let cost = if cost == 1 && state.world_tour_perk(crate::world_tour::native::Perk::IsmPlus) {
            let remainder = &mut state.tour.drive_cost_remainder;
            *remainder = (*remainder + 1) % 4;
            u8::from(*remainder != 0)
        } else { state.meter_cost(cost) };
        let drive = &mut state.sf6;
        if !drive.enabled || drive.burnout || drive.points < u16::from(cost) {
            return false;
        }
        drive.points -= u16::from(cost);
        drive.regen_delay = 90;
        if drive.points == 0 {
            drive.burnout = true;
            drive.parry_age = 0;
            drive.regen_delay = 150;
            self.era_emit_callout(ps, CalloutKind::Burnout);
        }
        self.era_share_gauge(side);
        true
    }

    pub(crate) fn sf6_can_spend(&self, ps: u32, cost: u8) -> bool {
        self.era_state(ps).is_some_and(|state| {
            if state.unified_gauge {
                state.super_meter >= state.meter_cost(STOCK_AT)
            } else {
                state.sf6.enabled && !state.sf6.burnout && state.sf6.points >= u16::from(state.meter_cost(cost))
            }
        })
    }

    pub(crate) fn sf6_input(
        &mut self,
        ps: u32,
        entity: u16,
        relative: u8,
        free: bool,
        cancel: bool,
        blocked: bool,
    ) -> bool {
        let Some(side) = self
            .era_side(ps)
            .filter(|&side| self.toggle_state.era.fighters[side].is_sf6())
        else {
            return false;
        };
        let (held, fresh) = self.era_buttons(ps);
        let mode = self.era_gauge_mode(side);
        let state = &mut self.toggle_state.era.fighters[side];
        for (i, tracker) in state.sf6.motions.iter_mut().enumerate() {
            tracker.step(relative, motion(i));
        }
        let forward_edge = relative == 1 && state.sf6.previous_direction != 1;
        let dash = forward_edge && state.sf6.forward_window != 0;
        state.sf6.forward_window = if forward_edge {
            14
        } else {
            state.sf6.forward_window.saturating_sub(1)
        };
        state.sf6.previous_direction = relative;
        let parry_chord = MediumPair.held(held);
        let new_parry = parry_chord && !MediumPair.held(held & !fresh);
        let impact = HeavyPair.held(held) && !HeavyPair.held(held & !fresh);
        let was_parrying = state.sf6.parry_age != 0;
        let jinrai = state.sf6.jinrai_left != 0 && fresh & KICK_MASK != 0;
        if jinrai {
            let slot = if relative & Down.bits() != 0 {
                JINRAI_LOW
            } else if relative & Forward.bits() != 0 {
                JINRAI_OVERHEAD
            } else {
                JINRAI_MID
            };
            if self.era_start_banked_move(ps, entity, slot, 0, None) {
                self.toggle_state.era.fighters[side].sf6.jinrai_left = 0;
                return true;
            }
        }
        let state = &mut self.toggle_state.era.fighters[side];
        if !parry_chord {
            state.sf6.parry_age = 0;
        }
        if state.super_capacity == 0 {
            return self.sf6_attacks(ps, entity, free, cancel, held, fresh);
        }
        let rush = (cancel && (dash || new_parry))
            || (!self.era_rules.disable_raw_drive_rush
                && dash
                && parry_chord
                && (free || was_parrying));
        let stock_cost = if cancel { 3 * STOCK_AT } else { STOCK_AT };
        if rush
            && self.sf6_can_spend(ps, stock_cost)
            && self.era_start_banked_move(ps, entity, RUSH, 0, None)
        {
            self.sf6_spend(ps, stock_cost);
            let drive = &mut self.toggle_state.era.fighters[side].sf6;
            drive.rush_left = 25;
            drive.rush_bonus = true;
            drive.parry_age = 0;
            return true;
        }
        let reversal = impact && relative == 1 && (blocked || was_parrying);
        if reversal
            && self.sf6_can_spend(ps, 2 * STOCK_AT)
            && self.era_start_banked_move(ps, entity, REVERSAL, 0, None)
        {
            self.sf6_spend(ps, 2 * STOCK_AT);
            let drive = &mut self.toggle_state.era.fighters[side].sf6;
            drive.reversal_left = 13;
            drive.parry_age = 0;
            return true;
        }
        if impact
            && (free || cancel)
            && self.sf6_can_spend(ps, STOCK_AT)
            && self.era_start_banked_move(ps, entity, IMPACT, 0, None)
        {
            self.sf6_spend(ps, STOCK_AT);
            self.toggle_state.era.fighters[side].sf6.armor = 2;
            return true;
        }
        if new_parry
            && free
            && self.sf6_can_spend(ps, STOCK_AT / 2)
            && self.era_start_banked_move(ps, entity, PARRY, 0, None)
        {
            self.sf6_spend(ps, STOCK_AT / 2);
            let drive = &mut self.toggle_state.era.fighters[side].sf6;
            if !drive.burnout {
                drive.parry_age = 1;
                drive.parry_flash = 6;
            }
            return true;
        }
        if was_parrying && parry_chord {
            if mode != GaugeMode::Unified {
                self.sf6_spend(ps, 1);
            }
            let drive = &mut self.toggle_state.era.fighters[side].sf6;
            if !drive.burnout {
                drive.parry_age = drive.parry_age.saturating_add(1);
                return true;
            }
        }
        if free
            && forward_edge
            && state_forward(self.era_state(ps))
            && mode == GaugeMode::EraDefault
        {
            let state = &mut self.toggle_state.era.fighters[side];
            state.sf6.recover(state.tour.boost_drive_gain(1));
            self.era_share_gauge(side);
        }
        self.sf6_attacks(ps, entity, free, cancel, held, fresh)
    }

    fn sf6_attacks(
        &mut self,
        ps: u32,
        entity: u16,
        free: bool,
        cancel: bool,
        held: u8,
        fresh: u8,
    ) -> bool {
        if !(free || cancel) {
            return false;
        }
        let Some(state) = self.era_state(ps).filter(|state| !state.what_if_active()) else {
            return false;
        };
        let Some(fighter) = state.fighter else {
            return false;
        };
        let mut command = None;
        for (i, (name, button, index)) in arts(fighter).into_iter().enumerate().rev() {
            let level = i as u8 + 1;
            let cost = state.meter_cost(level * STOCK_AT);
            if state.super_meter >= cost
                && button.held(fresh)
                && state.sf6.motions[index].ready(motion(index))
            {
                let critical = level == 3
                    && !self.era_rules.disable_comeback_scaling
                    && state.sub_hp <= state.max_sub_hp / 4;
                command = Some((
                    SA1 + i as u8,
                    level,
                    cost,
                    0,
                    Some(if critical { "CRITICAL ART" } else { name }),
                ));
                break;
            }
        }
        let cost = 2 * STOCK_AT;
        if command.is_none() && self.sf6_can_spend(ps, cost) {
            let (_, extra_button, index) = extra(fighter);
            let uppercut = if fighter == Retsu {
                (OD_UPPERCUT, 2, PairKick)
            } else {
                (OD_UPPERCUT, 4, PairPunch)
            };
            let kick = if fighter == Retsu {
                (OD_KICK, 5, PairPunch)
            } else {
                (OD_KICK, 3, PairKick)
            };
            for (slot, index, button) in [
                uppercut,
                (OD_PROJECTILE, 2, PairPunch),
                kick,
                (
                    OD_EXTRA,
                    index,
                    if extra_button.kick() {
                        PairKick
                    } else {
                        PairPunch
                    },
                ),
            ] {
                let mask = if button.kick() { KICK_MASK } else { PUNCH_MASK };
                if (slot != OD_EXTRA || fighter != Retsu)
                    && button.held(held)
                    && fresh & mask != 0
                    && state.sf6.motions[index].ready(motion(index))
                {
                    command = Some((slot, 0, 0, cost, None));
                    break;
                }
            }
        }
        if command.is_none() && fighter != Retsu {
            let (_, button, index) = extra(fighter);
            if button.held(fresh) && state.sf6.motions[index].ready(motion(index)) {
                command = Some((EXTRA, 0, 0, 0, None));
            }
        }
        if command.is_none()
            && Punch.held(fresh)
            && fighter != Ken
            && state.sf6.motions[5].ready(INSTALL_INPUT)
        {
            command = Some((INSTALL, 0, 0, 0, None));
        }
        if command.is_none()
            && Kick.held(fresh)
            && fighter != Ryu
            && state.sf6.motions[2].ready(QCF)
        {
            command = Some((
                if fighter == Ken { JINRAI } else { TOBIGERI },
                0,
                0,
                0,
                None,
            ));
        }
        if command.is_none() && fighter == Ken && PairKick.held(held) && fresh & KICK_MASK != 0 {
            command = Some((QUICK_DASH, 0, 0, 0, None));
        }
        let Some((slot, level, stock_cost, drive_cost, callout)) = command else {
            return false;
        };
        if !self.era_start_banked_move(ps, entity, slot, level, callout) {
            return false;
        }
        if drive_cost != 0 {
            self.sf6_spend(ps, drive_cost);
        }
        let side = self.era_side(ps).expect("active SF6 side");
        let state = &mut self.toggle_state.era.fighters[side];
        state.super_meter -= stock_cost;
        state.sf6.motions = [MotionTracker::default(); 6];
        state.sf6.od_flash = if drive_cost != 0 { 10 } else { 0 };
        if slot == INSTALL || (fighter == Retsu && slot == OD_KICK) {
            state.sf6.installed = true;
        }
        if slot == JINRAI {
            state.sf6.jinrai_left = 35;
        }
        self.era_share_gauge(side);
        true
    }

    pub(crate) fn sf6_defend(&mut self, ps: u32) -> bool {
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let state = &mut self.toggle_state.era.fighters[side];
        if state.sf6.reversal_left != 0 {
            return true;
        }
        if state.sf6.parry_age == 0 {
            return false;
        }
        let perfect = state.sf6.parry_age <= 2;
        state.sf6.parry_flash = 6;
        if state.unified_gauge {
            state.award_super_meter(STOCK_AT / 2);
        } else {
            state.sf6.recover(state.tour.boost_drive_gain(u16::from(STOCK_AT)));
        }
        if perfect {
            state.sf6.perfect_scale = 60;
            self.toggle_state.era.freeze = self.toggle_state.era.freeze.max(8);
            self.era_emit_callout(ps, CalloutKind::PerfectParry);
        }
        self.era_share_gauge(side);
        self.queue_sound(8);
        true
    }

    pub(crate) fn sf6_damage(
        &mut self,
        owner: u32,
        target: u32,
        slot: u8,
        mut damage: u16,
        chip: bool,
        armor_break: bool,
    ) -> Option<u16> {
        let attack = self
            .era_state(owner)
            .filter(|state| state.is_sf6())
            .map(|state| {
                (
                    state.alpha_super_level(slot),
                    state.sf6.perfect_scale != 0,
                    state.sf6.charged_move == slot,
                )
            });
        let punish = !chip && self.sf6_is_punish(owner, target);
        if let Some((level, perfect, charged)) = attack {
            if charged && !chip {
                damage += damage / 3;
            }
            if perfect && !chip {
                damage /= 2;
            }
            if level == 3
                && !self.era_rules.disable_comeback_scaling
                && self
                    .era_state(owner)
                    .is_some_and(|state| state.sub_hp <= state.max_sub_hp / 4)
            {
                damage += damage / 5;
            }
            if level == 0 && !chip {
                self.era_meter_gain(owner, 4);
            }
        }
        if let Some(side) = self.era_side(target) {
            let received = self.world_tour_damage(owner, target, damage, chip);
            let absorbed_damage = self.dramatic_scale_damage(owner, received);
            let state = &mut self.toggle_state.era.fighters[side];
            if !chip && state.sf6.armor != 0 && !armor_break {
                state.sf6.armor -= 1;
                state.sf6.armored_hit = true;
                let lost = absorbed_damage.min(state.sub_hp.saturating_sub(1));
                state.sub_hp -= lost;
                state.sf6.recoverable_hp = state.sf6.recoverable_hp.saturating_add(lost);
                state.sf6.recovery_delay = 75;
                self.era_reconcile_health(target, side);
                return None;
            }
            if !chip {
                state.sf6.recoverable_hp = 0;
            }
        }
        if chip {
            self.sf6_drain_drive(target, 16);
        } else if punish || (attack.is_some() && slot == IMPACT) {
            self.sf6_drain_drive(target, STOCK_AT);
        }
        if punish {
            self.era_emit_callout(owner, CalloutKind::PunishCounter);
        }
        Some(damage)
    }

    pub(crate) fn sf6_take_absorbed(&mut self, ps: u32) -> bool {
        self.era_side(ps).is_some_and(|side| {
            std::mem::take(&mut self.toggle_state.era.fighters[side].sf6.armored_hit)
        })
    }

    pub(crate) fn sf6_started(&mut self, ps: u32, slot: u8) {
        if let Some(side) = self.era_side(ps) {
            let fighter = &mut self.toggle_state.era.fighters[side];
            fighter.attack_reached_active = false;
            let drive = &mut fighter.sf6;
            if drive.installed
                && matches!(slot, 0x1a | EXTRA | OD_PROJECTILE | OD_EXTRA | SA1 | SA2)
            {
                drive.charged_move = slot;
                drive.installed = false;
            } else {
                drive.charged_move = 0;
            }
            if slot != JINRAI {
                drive.jinrai_left = 0;
            }
            if slot != IMPACT {
                drive.armor = 0;
            }
            if slot != PARRY {
                drive.parry_age = 0;
            }
            if drive.rush_left == 0 {
                drive.rush_bonus = false;
            }
        }
    }

    pub(crate) fn sf6_take_rush_bonus(&mut self, ps: u32) -> u8 {
        let Some(side) = self.era_side(ps) else {
            return 0;
        };
        let state = &mut self.toggle_state.era.fighters[side].sf6;
        if std::mem::take(&mut state.rush_bonus) {
            4
        } else {
            0
        }
    }

    fn sf6_is_punish(&self, owner: u32, target: u32) -> bool {
        if !self.era_state(owner).is_some_and(|state| state.is_sf6() && state.super_capacity != 0)
            || !self.era_state(target).is_some_and(|state| state.attack_reached_active)
        {
            return false;
        }
        let entity = self.rw(target + 0x10);
        self.eb(entity, 9) & 0xc0 == 0xc0 && self.eb(entity, 0xf) & 0x80 == 0
    }

    pub(crate) fn sf6_impact_punish(&mut self, owner: u32, target: u32, slot: u8) -> bool {
        if slot != IMPACT
            || !self.era_state(owner).is_some_and(|state| state.alpha_move_active(slot))
            || !self.sf6_is_punish(owner, target)
        {
            return false;
        }
        let Some(side) = self.era_side(target) else {
            return false;
        };
        self.toggle_state.era.fighters[side].sf6.wall_left = 28;
        true
    }

    pub(crate) fn sf6_wall_stun(&self, ps: u32) -> bool {
        self.era_state(ps)
            .is_some_and(|state| state.sf6.wall_left != 0)
    }

    pub(crate) fn sf6_drain_drive(&mut self, ps: u32, amount: u8) {
        let Some(side) = self.era_side(ps) else {
            return;
        };
        let state = &mut self.toggle_state.era.fighters[side].sf6;
        if !state.enabled || state.burnout {
            return;
        }
        state.points = state.points.saturating_sub(u16::from(amount));
        state.regen_delay = 90;
        if state.points == 0 {
            state.burnout = true;
            self.era_emit_callout(ps, CalloutKind::Burnout);
        }
        self.era_share_gauge(side);
    }

    pub(crate) fn sf6_impact_block(&mut self, owner: u32, target: u32, corner: bool) -> bool {
        let impact = self
            .era_state(owner)
            .is_some_and(|state| state.is_sf6() && state.alpha_move_active(IMPACT));
        let Some(side) = self.era_side(target) else {
            return false;
        };
        let state = &mut self.toggle_state.era.fighters[side];
        if impact && corner {
            state.sf6.wall_left = if state.sf6.burnout { 56 } else { 29 };
            return true;
        }
        false
    }
}

fn state_forward(state: Option<&EraFighterState>) -> bool {
    state.is_some_and(|state| state.sf6.enabled && !state.sf6.burnout)
}
