//! The live Utility AI uses the banked player's command rules and only
//! supplies pad input. A native round owns its controller in CoreToggleState.

use super::{decide, Action, Difficulty, Spot};
use crate::config::RulesetPreset;
use crate::era::{EraFamily, STOCK_AT};
use crate::fair_cpu::{pad_parts, perceive_fighters, Context, Level, Percept, DOWN, LEFT, RIGHT};
use crate::fighter::{CPU_STRUCT, GAME_MODE, P1_STRUCT, WORK_BASE};
use crate::front::PlayerInputs;
use crate::roster::TournamentFighter;
use crate::run::PowerOnMachine;

const MAX_MACRO_FRAMES: usize = 12;
const THREAT_HISTORY_FRAMES: usize = 27;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UtilityState {
    seed: Option<u64>,
    threat_history: [Threat; THREAT_HISTORY_FRAMES],
    queued: [u16; MAX_MACRO_FRAMES],
    cursor: u8,
    len: u8,
    cooldown: u8,
    last_pad: u16,
    fighter: Option<TournamentFighter>,
}

crate::canonical::canonical_struct!(UtilityState, "UtilityState.v3", {
    seed, threat_history, queued, cursor, len, cooldown, last_pad, fighter,
});

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Threat {
    frame: u32,
    distance: i32,
    airborne: bool,
    incoming: bool,
    reacting: bool,
}

crate::canonical::canonical_struct!(Threat, "UtilityThreat.v2", { frame, distance, airborne, incoming, reacting });

impl UtilityState {
    pub fn seeded(seed: u64) -> Self {
        Self {
            seed: Some(seed),
            ..Self::default()
        }
    }

    pub(crate) fn reset_round(&mut self) {
        // Keep the match seed while clearing the previous round's commands.
        *self = self.seed.map_or_else(Self::default, Self::seeded);
    }

    pub(crate) fn pad(&self) -> u16 {
        self.last_pad
    }

    /// Frames still queued for the native input handler.
    pub fn macro_remaining(&self) -> u8 {
        self.len - self.cursor
    }

    fn replace_macro(&mut self, frames: &[u16]) {
        assert!(!frames.is_empty() && frames.len() <= MAX_MACRO_FRAMES);
        self.queued = [0; MAX_MACRO_FRAMES];
        self.queued[..frames.len()].copy_from_slice(frames);
        self.cursor = 0;
        self.len = frames.len() as u8;
    }

    fn take_frame(&mut self) -> Option<u16> {
        if self.cursor == self.len {
            return None;
        }
        let pad = self.queued[usize::from(self.cursor)];
        self.cursor += 1;
        if self.cursor == self.len {
            self.queued = [0; MAX_MACRO_FRAMES];
            self.cursor = 0;
            self.len = 0;
        }
        Some(pad)
    }
}

pub struct UtilityAi;

impl UtilityAi {
    /// The live opponent and resources, without reading their current input.
    pub fn observe(machine: &PowerOnMachine, cpu_struct: u32, opp_struct: u32) -> Spot {
        let view = Self::view(machine, cpu_struct, opp_struct);
        Self::spot(machine, cpu_struct, view)
    }

    fn view(machine: &PowerOnMachine, me: u32, foe: u32) -> Percept {
        perceive_fighters(
            machine.state.frame,
            machine.in_fight(),
            machine.work_ram(),
            &machine.state.objectram,
            me,
            foe,
        )
    }

    fn side(machine: &PowerOnMachine, ps: u32) -> usize {
        if machine.match_config().dramatic.is_some() {
            return match ps {
                P1_STRUCT => 0,
                CPU_STRUCT => 1,
                _ => 2,
            };
        }
        let mode = machine.work_ram()[(GAME_MODE - WORK_BASE) as usize];
        let p1 = crate::fight_reader::player_struct(sf_video::callouts::Side::P1, mode);
        usize::from(ps != p1)
    }

    fn spot(machine: &PowerOnMachine, ps: u32, view: Percept) -> Spot {
        let side = Self::side(machine, ps);
        let toggle = machine.toggle_state();
        let era = &machine.dramatic_era_fighters()[side];
        let mode = machine.work_ram()[(GAME_MODE - WORK_BASE) as usize];
        let slot = crate::combo::slot_in_fight(ps, mode, machine.match_config().dramatic.is_some());
        let distance = i32::from(view.me.x.abs_diff(view.foe.x));
        let can_cancel =
            toggle.cancel.window[slot] != 0 && toggle.cancel.move_id[slot] == view.me.move_id;
        let native_stock = era.sf3_stock_cost().unwrap_or(STOCK_AT);
        let native_meter = match era.style.family {
            EraFamily::Sf5 if !era.unified_gauge => era
                .sf5_practice_points()
                .map_or(0, |(meter, _, _)| u16::from(meter)),
            EraFamily::Sf6 if !era.unified_gauge => era.sf6_drive_points().unwrap_or_default(),
            _ => u16::from(era.super_meter),
        };
        Spot {
            distance,
            airborne: view.foe.jumping(),
            throw_attempt: toggle.throw.tech_window[slot] != 0,
            can_cancel,
            attack_incoming: (view.foe.attacking() && distance <= 100)
                || (view.shot && view.shot_x.abs_diff(view.me.x) <= 92),
            blocking: toggle.combo.blockstun[slot] != 0,
            stocks: (native_meter / u16::from(native_stock)).min(255) as u8,
            cancel_ok: can_cancel
                || (view.me.state & 0xc0 == 0xc0
                    && (matches!(view.me.move_id, 0x19..=0x1b)
                        || era.sf4_special_active(view.me.move_id))),
            command_throw: false,
            teleport: false,
        }
    }

    /// One frame of synthetic Classic pad input from a snapshottable
    /// controller. The same interface also supplies a replay opponent.
    pub(crate) fn step(
        machine: &PowerOnMachine,
        state: &mut UtilityState,
        me: u32,
        foe: u32,
        fighter: TournamentFighter,
        family: EraFamily,
        context: &Context,
        positional_pad: Option<(u16, bool)>,
    ) -> PlayerInputs {
        let view = Self::view(machine, me, foe);
        if !view.fighting {
            state.reset_round();
            return PlayerInputs::default();
        }
        if state.fighter != Some(fighter) {
            state.reset_round();
            state.fighter = Some(fighter);
        }

        let spot = Self::spot(machine, me, view);
        let step = machine.state.frame.wrapping_add(1);
        let config = machine.match_config();
        let difficulty = match config.difficulty {
            1 => Difficulty::Easy,
            3 => Difficulty::Hard,
            4 => Difficulty::VeryHard,
            _ => Difficulty::Normal,
        };
        let history_slot = |frame: u32| frame as usize % THREAT_HISTORY_FRAMES;
        state.threat_history[history_slot(view.frame)] = Threat {
            frame: view.frame,
            distance: spot.distance,
            airborne: spot.airborne,
            incoming: spot.attack_incoming,
            reacting: view.foe.reacting(),
        };
        let legacy_kit = family == EraFamily::Sf1Plus
            || machine.dramatic_era_fighters()[Self::side(machine, me)].what_if_active();
        let delay = if legacy_kit && positional_pad.is_some() {
            Level::from_difficulty(config.difficulty).reaction()
        } else {
            u32::from(difficulty.delay())
        };
        let delayed = step
            .checked_sub(delay)
            .map(|frame| (frame, state.threat_history[history_slot(frame)]))
            .filter(|(frame, threat)| *frame != 0 && *frame == threat.frame)
            .map(|(_, threat)| threat);
        state.cooldown = state.cooldown.saturating_sub(1);
        if let Some(delayed) = delayed.filter(|_| {
            state.len == 0
                && state.cooldown == 0
                && config.preset != RulesetPreset::Faithful1987
                && (positional_pad.is_none()
                    || positional_pad.is_some_and(|(pad, idle)| pad == 0 && (!legacy_kit || idle)))
        }) {
            let delayed_spot = Spot {
                distance: if legacy_kit { delayed.distance } else { spot.distance },
                airborne: delayed.airborne,
                attack_incoming: delayed.incoming,
                ..spot
            };
            let seed = state.seed.unwrap_or(0x9e37_79b9_7f4a_7c15)
                ^ u64::from(step).wrapping_mul(0x5851_f42d_4c95_7f2d)
                ^ u64::from(me);
            let mut decision = decide(
                config.roster.mode,
                family,
                difficulty,
                config.toggles.fair_cpu_and_sagat_scaling,
                seed,
                delayed_spot,
            );
            if decision.action != Action::Hold && decision.action != Action::ThrowTech {
                if family == EraFamily::Sf5 && spot.can_cancel && spot.stocks > 0 {
                    decision.action = Action::VTrigger;
                } else if family == EraFamily::Sf4
                    && spot.cancel_ok
                    && spot.stocks >= 2
                    && delayed.incoming
                {
                    decision.action = Action::Fadc;
                }
            }
            // The original kits keep their delayed anti-air, spacing, stance
            // and throw plans. Offer a short cancel from a confirmed hit if
            // that plan has already finished and the cancel remains legal.
            let legal = !legacy_kit
                || positional_pad.is_none()
                || match decision.action {
                    Action::SpecialCancel => spot.can_cancel && delayed.reacting,
                    _ => false,
                };
            // Active CPU fights already have anti-air timing in the shared
            // positional controller. Takeover uses the utility macro here.
            let legal = legal && (decision.action != Action::AntiAir || positional_pad.is_none());
            if let Some(frames) = legal
                .then(|| {
                    Self::macro_for(
                        machine,
                        context,
                        family,
                        view,
                        delayed_spot,
                        decision.action,
                        positional_pad.is_none(),
                    )
                })
                .flatten()
            {
                state.replace_macro(&frames);
                state.cooldown = decision.delay.saturating_add(frames.len() as u8);
            }
        }
        state.last_pad = state
            .take_frame()
            .or(positional_pad.map(|(pad, _)| pad))
            .unwrap_or_else(|| {
                let toward = if view.me.facing_left { LEFT } else { RIGHT };
                let away = if toward == RIGHT { LEFT } else { RIGHT };
                if spot.attack_incoming {
                    away | if view.foe.low_attack() { DOWN } else { 0 }
                } else if view.me.free() && spot.distance > 65 {
                    toward
                } else {
                    0
                }
            });
        let (directions, buttons) = pad_parts(state.last_pad);
        PlayerInputs {
            directions,
            buttons,
        }
    }

    fn macro_for(
        machine: &PowerOnMachine,
        context: &Context,
        family: EraFamily,
        view: Percept,
        spot: Spot,
        action: Action,
        simple_spacing: bool,
    ) -> Option<Vec<u16>> {
        // A trial that disallows specials must still be played through the
        // same native action gates as a human.
        if machine.match_config().world_tour.is_some_and(|rules| {
            rules.mission
                == crate::world_tour::native::MissionKind::RuleModifierTrial(
                    crate::world_tour::native::RuleModifier::PureFootsies,
                )
        }) {
            return None;
        }
        context.utility_macro(family, view, spot, action, simple_spacing, MAX_MACRO_FRAMES)
    }
}
