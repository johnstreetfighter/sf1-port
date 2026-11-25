//! Tactical use of the live Style Card kit through the same pad as a player.
use super::{alpha_motion, button, Level, Percept, Plan, PlanKind, Rng, DOWN, LEFT, RIGHT};
use crate::config::MatchConfig;
use crate::era::{EraFamily, GaugeMode, Loadout, STOCK_AT};
use crate::era_fight::{sf3, sf4, sf5, sf6, EraFighterState};
use crate::moves::{ButtonClass, Dir, MoveEntry, MoveInput, MoveKind, Recognizer};

/// What a raw Drive Rush costs, as the kit's gate has it (a stock and a
/// little more).
const RUSH_COST: u8 = STOCK_AT + 2;
/// How near a projectile the stronger play parries is, as it starts, and
/// how far off a Drive Rush it meets with an armoured move begins: about
/// as far as one carries him in (SC8, `#200`).
const SHOT_ANSWERED: i16 = 120;
const RUSH_ANSWERED: i16 = 160;
/// One Super and the two selectable Ultras in a live SF4 kit. An expanded
/// kit that exceeds this bound leaves finisher routes out conservatively.
const SF4_ROUTE_FINISHERS: usize = 3;

/// A route step's actual directions up to a button edge, with held frames
/// collapsed. Route pads are already relative to a fighter facing right.
fn route_directions(step: &crate::route_book::Step, at: usize) -> Option<([u8; 12], usize)> {
    let mut directions = [0; 12];
    let mut len = 0;
    for &(direction, _) in step.pads.get(..=at)? {
        if len != 0 && directions[len - 1] == direction {
            continue;
        }
        if len == directions.len() {
            return None;
        }
        directions[len] = direction;
        len += 1;
    }
    Some((directions, len))
}

/// Match the written pad motion, not its descriptive step name. Charged
/// or airborne finishers need context this bounded check cannot prove and
/// are left out. A previous button in the same step is ambiguous too.
fn sf4_route_input(step: &crate::route_book::Step, at: usize, entry: MoveEntry, directions: &[u8]) -> bool {
    let buttons = step.pads[at].1;
    if !entry.input.button.held(buttons) {
        return false;
    }
    if entry.input.motion.is_empty() {
        return at == 0;
    }
    if step.pads[..at].iter().any(|&(_, held)| held != 0)
        || entry.input.motion.iter().any(|direction| direction.is_charge() || matches!(*direction, Dir::Jump | Dir::Up | Dir::UpForward | Dir::UpBack))
        || directions.len() != entry.input.motion.len()
    {
        return false;
    }
    let ultra = matches!(entry.input.button, ButtonClass::TriplePunch | ButtonClass::TripleKick);
    // SF4's ordinary Super does not read a triple chord as a punch/kick:
    // that chord belongs to its Ultra, even though Punch::held accepts it.
    if !ultra && (if entry.input.button.kick() { ButtonClass::TripleKick } else { ButtonClass::TriplePunch }).held(buttons) {
        return false;
    }
    entry.input.motion.iter().zip(directions).all(|(expected, actual)| expected.bits() == *actual)
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EraControl {
    pub free: bool,
    pub blockstun: bool,
    pub airborne: bool,
    pub cancel: bool,
    pub special: bool,
    /// SF5's own native grounded-special cancel gate, including EX slots.
    pub sf5_special_cancel: bool,
    pub waking: bool,
}

/// The SF6 cast's deliberate paid commands. Ryu keeps the kit and choices
/// already proven for him; reading these entries changes no legacy fields
/// or random choices. PairKick alone is not OD: Ken's Quick Dash is free.
#[derive(Clone, Copy, Debug)]
struct Sf6Tactics {
    impact: Option<MoveEntry>,
    enhanced: Option<MoveEntry>,
    super_move: Option<(MoveEntry, usize)>,
    keep_cancel_rush: bool,
    dash_pending: bool,
}

impl Sf6Tactics {
    fn of(era: &EraFighterState, moves: &[MoveEntry], mode: GaugeMode, level: Level) -> Self {
        let enabled = mode != GaugeMode::NoGauges && level.spends_meter();
        let unified = mode == GaugeMode::Unified;
        let drive = if unified { u16::from(era.super_meter) } else { era.sf6_drive_points().unwrap_or(0) };
        let can_drive = |cost| {
            let cost = u16::from(era.meter_cost(if unified { STOCK_AT } else { cost }));
            enabled && !era.sf6_burnout() && drive > cost && drive >= cost.saturating_add(level.drive_kept())
        };
        let family = |entry: &&MoveEntry| entry.recognizer == Recognizer::FamilyMove;
        let impact = can_drive(STOCK_AT).then(|| moves.iter().filter(family).copied().find(|entry| {
            entry.kind == MoveKind::System && entry.produces == sf6::IMPACT
        })).flatten();
        let enhanced = can_drive(2 * STOCK_AT).then(|| moves.iter().filter(family).copied().find(|entry| {
            entry.kind == MoveKind::Special && entry.name.starts_with("OD ")
                && matches!(entry.input.button, ButtonClass::PairPunch | ButtonClass::PairKick)
                && entry.input.motion.first() != Some(&Dir::Jump)
        })).flatten();
        let super_move = enabled.then(|| moves.iter().filter(family).copied().filter(|entry| {
            entry.kind == MoveKind::Super && entry.input.motion.first() != Some(&Dir::Jump)
                && entry.produces.checked_sub(0x29).filter(|index| *index < 3).is_some_and(|index| {
                    era.super_meter >= era.meter_cost((index + 1) * STOCK_AT)
                })
        }).max_by_key(|entry| entry.produces).map(|entry| (entry, 2))).flatten();
        let cancel_cost = u16::from(era.meter_cost(if unified { STOCK_AT } else { 3 * STOCK_AT }));
        Self { impact, enhanced, super_move,
            keep_cancel_rush: level == Level::Easy && mode != GaugeMode::NoGauges && !era.sf6_burnout() && drive >= cancel_cost,
            dash_pending: era.sf6_dash_pending(),
        }
    }
}

/// The stronger SF5 play's reading of its live kit and its own resources.
/// Keeping it separate leaves the old choices and their RNG calls unchanged.
#[derive(Clone, Copy, Debug)]
struct Sf5Tactics {
    gauges: bool,
    skill: Option<MoveEntry>,
    counter: Option<MoveEntry>,
    trigger: Option<MoveEntry>,
    followups: [Option<MoveEntry>; 2],
    enhanced: Option<MoveEntry>,
    critical: Option<MoveEntry>,
    reversal: Option<MoveEntry>,
    shift: Option<MoveEntry>,
    skill_live: bool,
    enhanced_live: bool,
    critical_input: Option<MoveInput>,
    keep_meter_directions: u16,
    previous_buttons: u8,
    stance_inputs: [Option<MoveInput>; 2],
    warm: bool,
    trigger_cost: u16,
    ex_cost: u16,
    critical_cost: u16,
}

impl Sf5Tactics {
    fn of(era: &EraFighterState, moves: &[MoveEntry], mode: GaugeMode, level: Level) -> Self {
        use crate::roster::TournamentFighter::Ryu;

        let v = era.sf5_practice_points();
        let gauges = mode != GaugeMode::NoGauges && v.is_some();
        let warm = v.is_some_and(|(_, timer, _)| timer != 0);
        let points = if mode == GaugeMode::Unified { era.super_meter } else { v.map_or(0, |(points, _, _)| points) };
        let trigger_cost = era.meter_cost(era.v_trigger_cost(mode));
        let spends = gauges && level.spends_meter();
        let entry = |slot, forward| moves.iter().copied().find(|entry| {
            entry.recognizer == Recognizer::FamilyMove && entry.kind == MoveKind::System
                && entry.produces == slot
                && if forward { entry.input.motion == [Dir::Forward] } else { entry.input.motion.is_empty() }
        });
        let skill = moves.iter().copied().find(|entry| {
            entry.recognizer == Recognizer::FamilyMove && entry.kind == MoveKind::System
                && matches!(entry.produces, sf5::SKILL_ONE | sf5::SKILL_TWO)
                && entry.input.button == ButtonClass::MediumPair
        });
        let enhanced = moves.iter().copied().find(|entry| {
            entry.recognizer == Recognizer::FamilyMove && entry.kind == MoveKind::Special
                && matches!(entry.input.button, ButtonClass::PairPunch | ButtonClass::PairKick)
                && entry.input.motion.first() != Some(&Dir::Jump)
        });
        let critical = moves.iter().copied().find(|entry| {
            entry.recognizer == Recognizer::FamilyMove && entry.kind == MoveKind::Super
                && entry.input.motion.first() != Some(&Dir::Jump)
        });
        let mut stance_inputs = [None; 2];
        for (slot, input) in stance_inputs.iter_mut().zip(moves.iter().filter(|entry| {
            entry.recognizer == Recognizer::FamilyMove && entry.kind == MoveKind::System
                && entry.input.motion.is_empty()
                && matches!(entry.input.button, ButtonClass::TriplePunch | ButtonClass::TripleKick)
        }).map(|entry| entry.input)) {
            *slot = Some(input);
        }
        let trigger = era.fighter.and_then(|fighter| entry(sf5::trigger_slot(fighter, era.style.loadout), false));
        let followup = |forward| era.fighter
            .and_then(|fighter| sf5::trigger_follow_up(fighter, era.style.loadout, forward))
            .and_then(|(slot, _)| entry(slot, slot == sf5::TRIGGER_FORWARD));
        let ryu = era.fighter == Some(Ryu) && !era.is_kage();
        let mut followups = [None; 2];
        if spends && warm && !ryu {
            followups[0] = followup(false);
            followups[1] = followup(true).filter(|entry| Some(entry.produces) != followups[0].map(|entry| entry.produces));
        }
        let counter = gauges.then(|| {
            // Isshin is a counter, never the offensive follow-up of a hit.
            (spends && warm && ryu).then(|| followup(false)).flatten().or_else(|| ryu.then_some(skill).flatten())
        }).flatten();
        let ex_cost = era.meter_cost(STOCK_AT);
        let critical_cost = era.meter_cost(3 * STOCK_AT);
        let can_v = spends && !warm && points >= era.meter_cost(STOCK_AT);
        Self {
            gauges,
            skill: skill.filter(|_| gauges && !ryu && !era.sf5_skill_charged()),
            counter,
            trigger: trigger.filter(|_| spends && !warm && points >= trigger_cost),
            followups,
            enhanced: enhanced.filter(|_| spends && era.super_meter >= ex_cost),
            critical: critical.filter(|_| spends && era.super_meter >= critical_cost),
            reversal: entry(sf5::REVERSAL, true).filter(|_| can_v),
            shift: entry(sf5::SHIFT, false).filter(|_| can_v),
            skill_live: gauges && skill.is_some(),
            enhanced_live: enhanced.is_some(),
            critical_input: critical.map(|entry| entry.input),
            keep_meter_directions: if gauges && !level.spends_meter() && era.super_meter >= critical_cost {
                era.sf5_critical_directions()
            } else {
                0
            },
            previous_buttons: era.held_buttons(),
            stance_inputs,
            warm,
            trigger_cost: u16::from(trigger_cost),
            ex_cost: u16::from(ex_cost),
            critical_cost: u16::from(critical_cost),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Tactics {
    family: EraFamily,
    control: EraControl,
    can_air_block: bool,
    can_parry: bool,
    fadc: bool,
    super_move: Option<(MoveEntry, usize)>,
    enhanced: Option<MoveEntry>,
    /// SF3's EX move at its real cost. The old play keeps its two-stock
    /// threshold; only the stronger play substitutes this candidate.
    sf3_enhanced: Option<MoveEntry>,
    /// The stronger SF4 play spends one EX stock, and keeps Super and
    /// Ultra as separate choices rather than whichever appears last.
    sf4_enhanced: Option<MoveEntry>,
    sf4_super: Option<MoveEntry>,
    sf4_ultra: Option<MoveEntry>,
    /// Live inputs are retained even when their gauges are empty, so a
    /// route cannot disguise an unavailable Ultra as a zero-cost recipe.
    sf4_route_finishers: [Option<MoveEntry>; SF4_ROUTE_FINISHERS],
    sf4_route_finishers_complete: bool,
    sf4_route_stance: Option<MoveEntry>,
    sf4_route_gauges: bool,
    sf4_route_super_cost: u16,
    /// Native SF4 Gen's stance name differs from his what-if kits. The
    /// old play's `stance` stays unchanged until `spending_safely`.
    sf4_stance: Option<MoveEntry>,
    sf5: Option<Sf5Tactics>,
    sf6: Option<Sf6Tactics>,
    guard_cancel: Option<MoveEntry>,
    evade: Option<MoveEntry>,
    armor: Option<MoveEntry>,
    rush: Option<MoveEntry>,
    install: Option<MoveEntry>,
    skill: Option<MoveEntry>,
    mix_up: Option<MoveEntry>,
    stance: Option<MoveEntry>,
    /// SF6's Drive Parry can be held through a guard: the gauge holds two
    /// bars over the level's reserve (it drains a point a frame held, and
    /// each hit it catches gives a bar back).
    parry_hold: bool,
    /// Its Drive gauge (its Super meter under a unified gauge) and the
    /// level's reserve, for the stronger play's spending (SC8, `#200`).
    drive: u16,
    reserve: u16,
    unified: bool,
    /// The opponent's era style, where known.
    foe_family: Option<EraFamily>,
    /// Custom Combo is running (SC16, `#208`).
    custom: bool,
}

impl Tactics {
    pub(super) fn of(
        era: &EraFighterState,
        moves: &[MoveEntry],
        cfg: MatchConfig,
        level: Level,
        control: EraControl,
    ) -> Self {
        use EraFamily::{Sf2, Sf3, Sf4, Sf5, Sf6, SfZ};
        let family = era.style.family;
        let mode = cfg.gauge_mode;
        let enabled = mode != GaugeMode::NoGauges;
        let ex = era.super_meter;
        let unified = mode == GaugeMode::Unified;
        let v_state = era.sf5_practice_points();
        let v = if unified {
            ex
        } else {
            v_state.map_or(0, |(points, _, _)| points)
        };
        let v_trigger_active = v_state.is_some_and(|(_, timer, _)| timer != 0);
        let v_cost = era.v_trigger_cost(mode);
        let drive = if unified {
            u16::from(ex)
        } else {
            era.sf6_drive_points().unwrap_or(0)
        };
        let reserve = super::drive_reserve(level);
        let can_drive = |native_cost| {
            let cost = if unified { STOCK_AT } else { native_cost };
            enabled && !era.sf6_burnout() && drive >= reserve + u16::from(cost)
        };
        let entry = |slot| {
            moves
                .iter()
                .copied()
                .find(|m| m.produces == slot && m.recognizer == Recognizer::FamilyMove)
        };
        let named = |name| moves.iter().copied().find(|m| m.name == name);
        let can_super = |m: &MoveEntry| {
            if m.kind != MoveKind::Super || m.input.motion.first() == Some(&Dir::Jump) {
                return false;
            }
            match family {
                Sf2 => ex >= if unified { 3 * STOCK_AT } else { STOCK_AT },
                SfZ if era.style.loadout != Loadout::XIsm
                    && matches!(m.name, "METSU SHORYUKEN" | "SHIPPU JINRAIKYAKU") =>
                {
                    ex >= 3 * STOCK_AT
                }
                SfZ if era.style.loadout == Loadout::XIsm && unified => ex >= 3 * STOCK_AT,
                SfZ => ex >= STOCK_AT,
                Sf3 => era.sf3_super_art_cost().is_some_and(|cost| ex >= cost),
                Sf4 if matches!(
                    m.input.button,
                    ButtonClass::TriplePunch | ButtonClass::TripleKick
                ) =>
                {
                    if unified {
                        ex >= 3 * STOCK_AT
                    } else {
                        era.sf4_revenge_points()
                            .is_some_and(|r| r >= sf4::REVENGE_MAX / 2)
                    }
                }
                Sf4 => ex >= if unified { 3 * STOCK_AT } else { 4 * STOCK_AT },
                Sf5 => ex >= 3 * STOCK_AT,
                Sf6 => m
                    .produces
                    .checked_sub(0x29)
                    .filter(|index| *index < 3)
                    .is_some_and(|index| ex >= STOCK_AT * (index + 1)),
                _ => false,
            }
        };
        let super_move = (enabled && level != Level::Easy)
            .then(|| {
                moves.iter().filter(|m| can_super(m)).last().map(|m| {
                    let strength =
                        if family == SfZ && ex < 3 * STOCK_AT && era.style.loadout != Loadout::XIsm
                        {
                            if ex >= 2 * STOCK_AT {
                                1
                            } else {
                                0
                            }
                        } else {
                            2
                        };
                    (*m, strength)
                })
            })
            .flatten();
        let enhanced = if family == Sf6 {
            can_drive(2 * STOCK_AT) && drive >= 3 * u16::from(STOCK_AT)
        } else {
            let stock = if family == Sf3 {
                era.sf3_stock_cost().unwrap_or(STOCK_AT)
            } else {
                STOCK_AT
            };
            u16::from(ex) >= 2 * u16::from(stock)
        };
        let enhanced_move = || {
            moves.iter().copied().find(|m| {
                m.recognizer == Recognizer::FamilyMove
                    && m.kind == MoveKind::Special
                    && matches!(m.input.button, ButtonClass::PairPunch | ButtonClass::PairKick)
                    && m.input.motion.first() != Some(&Dir::Jump)
            })
        };
        let enhanced = (enabled && enhanced).then(enhanced_move).flatten();
        let sf3_enhanced = (enabled && family == Sf3 && ex >= era.sf3_ex_cost())
            .then(enhanced_move)
            .flatten();
        let sf4_enhanced = (enabled && family == Sf4 && level.spends_meter() && ex >= STOCK_AT)
            .then(enhanced_move)
            .flatten();
        let sf4_finish = |ultra: bool| {
            (enabled && family == Sf4 && level.spends_meter())
                .then(|| {
                    moves.iter().copied().find(|m| {
                        let chord = matches!(m.input.button, ButtonClass::TriplePunch | ButtonClass::TripleKick);
                        let affordable = if ultra {
                            if unified { ex >= 3 * STOCK_AT } else { era.sf4_ultra_ready() }
                        } else {
                            ex >= era.sf4_super_cost()
                        };
                        m.kind == MoveKind::Super
                            && m.recognizer == Recognizer::FamilyMove
                            && m.input.motion.first() != Some(&Dir::Jump)
                            && chord == ultra
                            && affordable
                    })
                })
                .flatten()
        };
        let guard_cancel = if enabled {
            match family {
                SfZ if era.alpha_counter_enabled() && ex >= STOCK_AT => moves
                    .iter()
                    .copied()
                    .find(|m| m.recognizer == Recognizer::AlphaCounter),
                Sf5 if !v_trigger_active && v >= STOCK_AT => entry(sf5::REVERSAL),
                Sf6 if can_drive(2 * STOCK_AT) => entry(sf6::REVERSAL),
                _ => None,
            }
        } else {
            None
        };
        let (mut evade, armor) = if enabled {
            match family {
                Sf4 => (
                    None,
                    moves.iter().copied().find(|m| {
                        m.produces == sf4::FOCUS && m.input.button == ButtonClass::MediumPair
                    }),
                ),
                Sf5 if !v_trigger_active && v >= STOCK_AT => (entry(sf5::SHIFT), None),
                Sf6 => (
                    can_drive(STOCK_AT / 2 + 4)
                        .then(|| entry(sf6::PARRY))
                        .flatten(),
                    can_drive(STOCK_AT).then(|| entry(sf6::IMPACT)).flatten(),
                ),
                _ => (None, None),
            }
        } else {
            (None, None)
        };
        if enabled && family == Sf5 && evade.is_none() {
            evade = named("MIND'S EYE");
        }
        let install = if enabled {
            match family {
                SfZ if ex >= if unified { 2 * STOCK_AT } else { STOCK_AT } => named("CUSTOM COMBO"),
                Sf5 if v_trigger_active => moves.iter().copied().find(|m| {
                    m.input.button == ButtonClass::HeavyPair
                        && matches!(
                            m.produces,
                            sf5::TRIGGER_ATTACK | sf5::TRIGGER_FORWARD | sf5::COUNTER_STRIKE
                        )
                }),
                Sf5 if v_state.is_some() && v >= v_cost => moves.iter().copied().find(|m| {
                    m.input.button == ButtonClass::HeavyPair
                        && matches!(m.produces, sf5::TRIGGER | sf5::TRIGGER_ATTACK)
                }),
                _ => None,
            }
        } else {
            None
        };
        let mut sf4_route_finishers = [None; SF4_ROUTE_FINISHERS];
        let mut sf4_route_finishers_complete = true;
        if family == Sf4 {
            for (index, entry) in moves.iter().copied().filter(|entry| entry.kind == MoveKind::Super && entry.recognizer == Recognizer::FamilyMove).enumerate() {
                if let Some(slot) = sf4_route_finishers.get_mut(index) {
                    *slot = Some(entry);
                } else {
                    sf4_route_finishers_complete = false;
                }
            }
        }
        Self {
            family,
            control,
            can_air_block: enabled
                && family == SfZ
                && era.is_alpha()
                && era.style.loadout != Loadout::XIsm,
            can_parry: enabled && family == Sf3 && era.sf3_stock_cost().is_some(),
            fadc: enabled && family == Sf4 && ex >= if unified { STOCK_AT } else { 2 * STOCK_AT },
            super_move,
            enhanced,
            sf3_enhanced,
            sf4_enhanced,
            sf4_super: sf4_finish(false),
            sf4_ultra: sf4_finish(true),
            sf4_route_finishers,
            sf4_route_finishers_complete,
            sf4_route_stance: (family == Sf4).then(|| moves.iter().copied().find(|entry| {
                entry.kind == MoveKind::System && entry.recognizer == Recognizer::FamilyMove
                    && entry.input.motion.is_empty()
                    && matches!(entry.input.button, ButtonClass::TriplePunch | ButtonClass::TripleKick)
            })).flatten(),
            sf4_route_gauges: enabled,
            sf4_route_super_cost: if family == Sf4 { u16::from(era.sf4_super_cost()) } else { 0 },
            sf4_stance: (family == Sf4).then(|| named("CRANE STANCE")).flatten(),
            sf5: (family == Sf5 && !era.is_kage()).then(|| Sf5Tactics::of(era, moves, mode, level)),
            sf6: (family == Sf6 && era.fighter.is_some_and(|fighter| fighter != crate::roster::TournamentFighter::Ryu))
                .then(|| Sf6Tactics::of(era, moves, mode, level)),
            guard_cancel,
            evade,
            armor,
            rush: (family == Sf6 && !cfg.disable_raw_drive_rush && can_drive(RUSH_COST))
                .then(|| entry(sf6::RUSH))
                .flatten(),
            install,
            skill: (enabled && family == Sf5)
                .then(|| entry(sf5::SKILL_ONE).or_else(|| entry(sf5::SKILL_TWO)))
                .flatten()
                .filter(|entry| entry.name != "MIND'S EYE"),
            mix_up: if family == Sf3 {
                entry(sf3::OVERHEAD)
            } else {
                named("MURDERER CHAIN")
            },
            stance: named("SWITCH TO CRANE").or_else(|| named("SWITCH TO MANTIS")),
            parry_hold: enabled && family == Sf6 && can_drive(2 * STOCK_AT) && entry(sf6::PARRY).is_some(),
            drive,
            reserve,
            unified,
            foe_family: None,
            custom: era.custom_combo_active(),
        }
    }

    /// The stronger play spends SF6's Drive only down to the level's
    /// reserve and never to burnout (SC8, `#200`): what is left after
    /// paying `native_cost` (a stock under a unified gauge) is at least the
    /// reserve and more than nothing. The other era styles pay from their
    /// own meters, which the kit's own gates already check.
    fn keeps_drive(self, native_cost: u8) -> bool {
        if self.family != EraFamily::Sf6 {
            return true;
        }
        let cost = u16::from(if self.unified { STOCK_AT } else { native_cost });
        self.drive >= self.reserve.saturating_add(cost) && self.drive > cost
    }

    /// The opponent's era style, which tells his moves apart (another
    /// style can use SF6's Drive Rush move id for something else).
    pub(super) fn against(mut self, family: Option<EraFamily>) -> Self {
        self.foe_family = family;
        self
    }

    /// The kit as the stronger play may spend it (SC8, `#200`): SF6's Drive
    /// Rush, Drive Impact and OD moves only where they leave Drive above the
    /// level's reserve, never down to burnout.
    pub(super) fn spending_safely(mut self, level: Level) -> Self {
        // The stronger play keeps the level's own share of the gauge
        // (SC11, `#203`).
        self.reserve = level.drive_kept();
        if self.family == EraFamily::Sf4 {
            self.stance = self.sf4_stance;
            self.enhanced = self.sf4_enhanced.filter(|_| level.spends_meter());
        }
        // Nor does it switch Gen back from the Crane to the Mantis: back
        // and forth at range only stood him still (SC16, `#208`).
        self.stance = self.stance.filter(|_| self.can_change_stance());
        if self.family == EraFamily::Sf3 {
            self.enhanced = self.sf3_enhanced;
        }
        if let Some(sf5) = self.sf5 {
            self.enhanced = sf5.enhanced.filter(|_| level.spends_meter());
            self.super_move = sf5.critical.filter(|_| level.spends_meter()).map(|entry| (entry, 2));
            self.guard_cancel = sf5.reversal.filter(|_| level.spends_meter());
            self.evade = sf5.counter.or(sf5.shift.filter(|_| level.spends_meter()));
            // Trigger activations/follow-ups belong to explicit seen-hit
            // and whiff plans, never the quiet neutral fallback.
            self.install = None;
            self.skill = sf5.skill;
        }
        if self.family == EraFamily::Sf6 {
            self.rush = self.rush.filter(|_| self.keeps_drive(RUSH_COST));
            if let Some(sf6) = self.sf6 {
                self.armor = sf6.impact;
                self.enhanced = sf6.enhanced;
                self.super_move = sf6.super_move;
            } else {
                self.armor = self.armor.filter(|_| self.keeps_drive(STOCK_AT));
                self.enhanced = self.enhanced.filter(|_| self.keeps_drive(2 * STOCK_AT));
            }
        }
        self
    }

    /// The pads of a dash that spends meter to close in, when the gauge,
    /// the level's reserve and the rules allow one now (SC8, `#200`).
    pub(super) fn rush_in(self, view: Percept) -> Option<Plan> {
        let entry = self.rush.filter(|_| self.control.free && self.keeps_drive(RUSH_COST))?;
        let mut plan = self.pad(&entry, view, 1);
        plan.end_at_last_press();
        Some(plan)
    }

    /// New cast plans must not repeatedly select an unaffordable rush.
    /// The caller applies the proactive reserve; Ryu's selection is intact.
    pub(super) fn sf6_rush_ready(self) -> bool {
        self.family == EraFamily::Sf6 && self.control.free && !self.control.airborne
            && self.rush.is_some() && self.keeps_drive(RUSH_COST)
    }

    pub(super) fn sf6_impact_ready(self) -> bool {
        self.sf6.is_some_and(|kit| kit.impact.is_some()) && self.control.free && !self.control.airborne
    }

    pub(super) fn sf6_impact(self, view: Percept) -> Option<Plan> {
        let entry = self.sf6?.impact.filter(|_| self.sf6_impact_ready())?;
        Some(Plan::of(PlanKind::Act, &alpha_motion(&entry, !view.me.facing_left, 2, true)))
    }

    pub(super) fn sf6_ex_ready(self) -> bool {
        self.sf6.is_some_and(|kit| kit.enhanced.is_some()) && self.control.free && !self.control.airborne
    }

    pub(super) fn sf6_ex(self, view: Percept) -> Option<Plan> {
        let entry = self.sf6?.enhanced.filter(|_| self.sf6_ex_ready())?;
        Some(Plan::of(PlanKind::Act, &alpha_motion(&entry, !view.me.facing_left, 1, true)))
    }

    pub(super) fn sf6_super_ready(self) -> bool {
        self.sf6.is_some_and(|kit| kit.super_move.is_some())
            && (self.control.free || self.control.cancel) && !self.control.airborne
    }

    pub(super) fn sf6_super(self, view: Percept) -> Option<Plan> {
        let (entry, strength) = self.sf6?.super_move.filter(|_| self.sf6_super_ready())?;
        Some(Plan::of(PlanKind::Act, &alpha_motion(&entry, !view.me.facing_left, strength, true)))
    }

    /// Easy's ordinary motions must not become a paid cancel Rush. Native
    /// SF6 accepts a double Forward or fresh MP/MK during a normal cancel;
    /// release that input only while it could spend our own Drive. Other
    /// levels, Ryu and human pads keep their native commands unchanged.
    pub(super) fn sf6_keep_meter(self, mut pad: u16, facing_left: bool) -> u16 {
        let Some(sf6) = self.sf6.filter(|kit| kit.keep_cancel_rush) else { return pad };
        if !self.control.cancel || self.control.airborne || self.control.blockstun { return pad }
        let forward = if facing_left { LEFT } else { RIGHT };
        if sf6.dash_pending && pad & 15 == forward { pad &= !forward; }
        if ButtonClass::MediumPair.held((pad >> 4) as u8) { pad &= !button(4); }
        pad
    }

    /// The stronger play may hold a parry in place of a guard (SC8, `#200`).
    /// Not while its own move can still be cancelled: the parry's buttons
    /// would cancel it into a Drive Rush.
    pub(super) fn can_hold_parry(self) -> bool {
        self.parry_hold && !self.control.cancel
    }

    /// Custom Combo can be started now (SC16, `#208`): the SFZ3 V-ism's
    /// meter holds enough, and its fighter is free.
    pub(super) fn custom_combo_ready(self) -> bool {
        self.family == EraFamily::SfZ && self.install.is_some() && self.control.free && !self.custom
    }

    /// Custom Combo is running.
    pub(super) fn custom_active(self) -> bool {
        self.custom
    }

    /// SF3's universal overhead is present in this fight and can be
    /// entered from the ground. A game plan chooses its moment; this gate
    /// checks only the fighter's own controls and available moves.
    pub(super) fn sf3_overhead_ready(self) -> bool {
        self.family == EraFamily::Sf3 && self.mix_up.is_some() && self.control.free && !self.control.airborne
    }

    /// The overhead's real input (medium punch and medium kick).
    pub(super) fn sf3_overhead(self, view: Percept) -> Option<Plan> {
        let entry = self.mix_up.filter(|_| self.sf3_overhead_ready())?;
        Some(self.pad(&entry, view, 1))
    }

    /// The SF3 fighter has an EX move in its live move list and enough
    /// meter for that move's actual cost, rather than two Super Art bars.
    /// Native New Generation lists no EX move.
    pub(super) fn sf3_ex_ready(self) -> bool {
        self.family == EraFamily::Sf3 && self.sf3_enhanced.is_some() && self.control.free && !self.control.airborne
    }

    /// The real motion and pair of buttons for that EX move.
    pub(super) fn sf3_ex(self, view: Percept) -> Option<Plan> {
        let entry = self.sf3_enhanced.filter(|_| self.sf3_ex_ready())?;
        // A plan feels its own facing now; the opponent's position is
        // still delayed and can be on the other side after a cross-up.
        Some(Plan::of(PlanKind::Act, &alpha_motion(&entry, !view.me.facing_left, 1, true)))
    }

    /// A free grounded SF4 fighter can start its ordinary Focus Attack.
    pub(super) fn sf4_focus_ready(self) -> bool {
        self.family == EraFamily::Sf4 && self.armor.is_some() && self.control.free && !self.control.airborne
    }

    /// Hold the real medium-pair chord long enough for level two, then
    /// release. The game plan chooses the spacing and delayed event.
    pub(super) fn sf4_focus(self, _view: Percept) -> Option<Plan> {
        if !self.sf4_focus_ready() { return None }
        let pair = button(1) | button(4);
        let mut pads = vec![0];
        pads.extend([pair; 22]);
        pads.push(0);
        Some(Plan::of(PlanKind::Act, &pads))
    }

    /// FADC spends from the fighter's own meter during its own special.
    /// The caller decides from a delayed hit or guard whether to go in
    /// for a follow-up or out to make the special safer.
    pub(super) fn sf4_fadc_ready(self) -> bool {
        self.family == EraFamily::Sf4 && self.fadc && self.armor.is_some() && self.control.special && !self.control.airborne
    }

    pub(super) fn sf4_fadc(self, view: Percept, forward: bool) -> Option<Plan> {
        if !self.sf4_fadc_ready() { return None }
        let toward = if view.me.facing_left { LEFT } else { RIGHT };
        let direction = if forward { toward } else { LEFT + RIGHT - toward };
        let focus = button(1) | button(4);
        Some(Plan::of(PlanKind::Act, &[0, focus, focus | direction, focus, focus | direction, direction, 0]))
    }

    pub(super) fn sf4_ex_ready(self) -> bool {
        self.family == EraFamily::Sf4 && self.sf4_enhanced.is_some() && self.control.free && !self.control.airborne
    }

    pub(super) fn sf4_ex(self, view: Percept) -> Option<Plan> {
        let entry = self.sf4_enhanced.filter(|_| self.sf4_ex_ready())?;
        Some(Plan::of(PlanKind::Act, &alpha_motion(&entry, !view.me.facing_left, 1, true)))
    }

    pub(super) fn sf4_super_ready(self) -> bool {
        self.family == EraFamily::Sf4 && self.sf4_super.is_some() && (self.control.free || self.control.cancel) && !self.control.airborne
    }

    pub(super) fn sf4_super(self, view: Percept) -> Option<Plan> {
        let entry = self.sf4_super.filter(|_| self.sf4_super_ready())?;
        Some(Plan::of(PlanKind::Act, &alpha_motion(&entry, !view.me.facing_left, 2, true)))
    }

    pub(super) fn sf4_ultra_ready(self) -> bool {
        self.family == EraFamily::Sf4 && self.sf4_ultra.is_some() && (self.control.free || self.control.cancel) && !self.control.airborne
    }

    pub(super) fn sf4_ultra(self, view: Percept) -> Option<Plan> {
        let entry = self.sf4_ultra.filter(|_| self.sf4_ultra_ready())?;
        Some(Plan::of(PlanKind::Act, &alpha_motion(&entry, !view.me.facing_left, 2, true)))
    }

    /// Eligibility of SF4 finisher recipes for the stronger play only.
    /// Route books measure Super/Drive, not Revenge: an Ultra can report
    /// zero in both columns. Match real pad commands against the live kit
    /// before trusting that metadata. No other era is changed here.
    pub(super) fn sf4_route_ready(self, route: &crate::route_book::Route, level: Level, super_meter: u16) -> bool {
        if self.family != EraFamily::Sf4 {
            return true;
        }
        if (route.super_spent != 0 || route.drive_spent != 0) && (!level.spends_meter() || !self.sf4_route_gauges) {
            return false;
        }
        let mut supers = 0u16;
        let mut ultras = 0u16;
        let mut changed_stance = false;
        for step in &route.steps {
            let mut previous = 0;
            for (at, &(_, buttons)) in step.pads.iter().enumerate() {
                let fresh = buttons & !previous;
                previous = buttons;
                if fresh == 0 {
                    continue;
                }
                let Some((directions, len)) = route_directions(step, at) else { return false };
                let directions = &directions[..len];
                if self.sf4_route_stance.is_some_and(|entry| sf4_route_input(step, at, entry, directions)) {
                    changed_stance = true;
                }
                let mut matching = self.sf4_route_finishers.iter().flatten().copied().filter(|&entry| sf4_route_input(step, at, entry, directions));
                let matched = matching.next();
                // Double quarter/half circles, or the back-forward-back-
                // forward charge recipe, must not turn into an unpriced
                // finisher merely because that input is absent in this
                // stance/loadout. Other ambiguous long motions stay out.
                let charge_super = step.hold & crate::route_book::LEFT != 0
                    && directions == [crate::route_book::RIGHT, crate::route_book::LEFT, crate::route_book::RIGHT];
                let finisher = matched.is_some() || len >= 6 || charge_super;
                if !finisher {
                    continue;
                }
                if !level.spends_meter() || !self.sf4_route_gauges || !self.sf4_route_finishers_complete || matching.next().is_some() {
                    return false;
                }
                let Some(entry) = matched else { return false };
                if matches!(entry.input.button, ButtonClass::TriplePunch | ButtonClass::TripleKick) {
                    if self.sf4_ultra.is_none() { return false }
                    let Some(next) = ultras.checked_add(1) else { return false };
                    ultras = next;
                    if !self.unified && ultras > 1 { return false }
                } else {
                    if self.sf4_super.is_none() { return false }
                    let Some(next) = supers.checked_add(1) else { return false };
                    supers = next;
                }
            }
        }
        // The current kit describes one stance. Do not guess what a
        // later finisher means after a stance command inside the recipe.
        if changed_stance && (supers != 0 || ultras != 0) {
            return false;
        }
        let Some(super_cost) = supers.checked_mul(self.sf4_route_super_cost) else { return false };
        if route.super_spent < super_cost {
            return false;
        }
        // Native Revenge is separate; Unified pays each Ultra from the
        // same gauge as any EX/Super consumption recorded in the book.
        let extra = if self.unified { ultras.checked_mul(u16::from(3 * STOCK_AT)) } else { Some(0) };
        extra.and_then(|cost| route.super_spent.checked_add(cost)).is_some_and(|cost| cost <= super_meter)
    }

    fn sf5_pad(entry: MoveEntry, view: Percept, strength: usize) -> Plan {
        Plan::of(PlanKind::Act, &alpha_motion(&entry, !view.me.facing_left, strength, true))
    }

    pub(super) fn sf5_skill_ready(self) -> bool {
        self.sf5.is_some_and(|sf5| sf5.skill.is_some()) && self.control.free && !self.control.airborne
    }

    pub(super) fn sf5_skill(self, view: Percept) -> Option<Plan> {
        let entry = self.sf5?.skill.filter(|_| self.sf5_skill_ready())?;
        Some(Self::sf5_pad(entry, view, 1))
    }

    /// An inactive Trigger may be installed; an active one may offer an
    /// offensive follow-up. Ryu's Isshin instead answers a delayed threat.
    pub(super) fn sf5_trigger_ready(self) -> bool {
        self.sf5.is_some_and(|sf5| sf5.trigger.is_some() || sf5.followups.iter().any(Option::is_some))
            && (self.control.free || self.control.cancel) && !self.control.airborne
    }

    pub(super) fn sf5_trigger(self, view: Percept, rng: &mut Rng) -> Option<Plan> {
        if !self.sf5_trigger_ready() { return None }
        let sf5 = self.sf5?;
        let entry = sf5.trigger.or_else(|| match sf5.followups {
            [Some(first), Some(second)] => Some(if rng.below(2) == 0 { first } else { second }),
            [first, second] => first.or(second),
        })?;
        Some(Self::sf5_pad(entry, view, 2))
    }

    pub(super) fn sf5_ex_ready(self) -> bool {
        self.sf5.is_some_and(|sf5| sf5.enhanced.is_some()) && self.control.free && !self.control.airborne
    }

    pub(super) fn sf5_ex(self, view: Percept) -> Option<Plan> {
        let entry = self.sf5?.enhanced.filter(|_| self.sf5_ex_ready())?;
        Some(Self::sf5_pad(entry, view, 1))
    }

    pub(super) fn sf5_super_ready(self) -> bool {
        self.sf5.is_some_and(|sf5| sf5.critical.is_some())
            && (self.control.free || self.control.cancel) && !self.control.airborne
    }

    pub(super) fn sf5_super(self, view: Percept) -> Option<Plan> {
        let entry = self.sf5?.critical.filter(|_| self.sf5_super_ready())?;
        Some(Self::sf5_pad(entry, view, 2))
    }

    /// Easy may repeat an ordinary quarter circle while the first one is
    /// still in its own CA recognizer. Keep its meter by withholding only
    /// the CA's button family on a direction that would fire that command.
    /// This uses its own input bookkeeping, never the opponent's input.
    pub(super) fn sf5_keep_meter(self, pad: u16, facing_left: bool) -> u16 {
        if self.control.airborne || self.control.blockstun
            || !(self.control.free || self.control.cancel || self.control.special || self.control.sf5_special_cancel)
        {
            return pad;
        }
        let Some(sf5) = self.sf5 else { return pad };
        let Some(input) = sf5.critical_input else { return pad };
        let mut relative = pad & 0xf;
        if facing_left {
            relative = (relative & !3) | ((relative & 1) << 1) | ((relative & 2) >> 1);
        }
        if sf5.keep_meter_directions & (1 << relative) == 0 { return pad }
        let buttons = (pad >> 4) as u8;
        // Native free/cancel-ground V-Skill recognition takes priority
        // over CA only on a fresh MP/MK edge. A pair held from an earlier
        // input must not hide a newly pressed CA-family attack button.
        if sf5.skill_live && (self.control.free || self.control.cancel)
            && ButtonClass::MediumPair.held(buttons)
            && buttons & !sf5.previous_buttons & 0b010_010 != 0
        {
            return pad;
        }
        let family = if input.button.kick() {
            button(3) | button(4) | button(5)
        } else {
            button(0) | button(1) | button(2)
        };
        pad & !family
    }

    /// SF5 route metadata omits V meter. Inspect actual button edges so a
    /// zero-Super recipe cannot borrow a Trigger, even if its name changes.
    /// A route's later commands are checked for availability, not whether
    /// the fighter could execute that later step on this very frame.
    pub(super) fn sf5_route_ready(self, route: &crate::route_book::Route, level: Level, super_meter: u16) -> bool {
        let Some(sf5) = self.sf5 else { return true };
        if (route.super_spent != 0 || route.drive_spent != 0) && (!sf5.gauges || !level.spends_meter()) {
            return false;
        }
        let mut triggers = 0u16;
        let mut enhanced = 0u16;
        let mut critical = 0u16;
        for step in &route.steps {
            let mut previous = 0;
            for (at, &(direction, buttons)) in step.pads.iter().enumerate() {
                let fresh = buttons & !previous;
                previous = buttons;
                if fresh == 0 { continue }
                if ButtonClass::HeavyPair.held(buttons) {
                    // Native recognition gives this chord priority over
                    // Skill or CA, and only exact Forward selects the
                    // forward follow-up. Do not guess a second activation
                    // after the first has spent its V/timer budget.
                    triggers += 1;
                    let followup = if direction == crate::route_book::RIGHT {
                        sf5.followups[1].or(sf5.followups[0].filter(|entry| entry.input.motion.is_empty()))
                    } else {
                        sf5.followups[0]
                    };
                    if !sf5.gauges || !level.spends_meter() || triggers > 1
                        || if sf5.warm { followup.is_none() } else { sf5.trigger.is_none() }
                    {
                        return false;
                    }
                    continue;
                }
                if ButtonClass::ShiftPair.held(buttons) && direction & !3 == 0 {
                    // Shift/Break are defensive answers with their own V
                    // cost or evade window, never an unpriced attack route.
                    return false;
                }
                if ButtonClass::MediumPair.held(buttons) {
                    if !sf5.skill_live { return false }
                    continue;
                }
                let Some((directions, len)) = route_directions(step, at) else { return false };
                let directions = &directions[..len];
                let ca = sf5.critical_input.is_some_and(|input| {
                    input.button.held(buttons)
                        && input.motion.len() == len
                        && input.motion.iter().zip(directions).all(|(expected, actual)| expected.bits() == *actual)
                });
                if ca {
                    critical += 1;
                } else if len >= 6 {
                    return false;
                } else if at == 0 && sf5.stance_inputs.iter().flatten().any(|input| input.button.held(buttons)) {
                    // A naked live Gen stance chord spends no gauge. It
                    // must not be mistaken for an EX pair inside a route.
                    continue;
                } else if ButtonClass::PairPunch.held(buttons) || ButtonClass::PairKick.held(buttons) {
                    if !sf5.enhanced_live { return false }
                    enhanced += 1;
                }
            }
        }
        if (enhanced != 0 || critical != 0) && (!sf5.gauges || !level.spends_meter()) { return false }
        let Some(actual_cost) = enhanced.checked_mul(sf5.ex_cost)
            .and_then(|ex| critical.checked_mul(sf5.critical_cost).and_then(|ca| ex.checked_add(ca))) else { return false };
        if route.super_spent < actual_cost { return false }
        let trigger_cost = if self.unified && triggers != 0 && !sf5.warm { sf5.trigger_cost } else { 0 };
        route.super_spent.checked_add(trigger_cost).is_some_and(|cost| cost <= super_meter)
    }

    /// Its fighter can change stance from the Mantis to the Crane (Gen):
    /// the stronger play's game plans never switch him back (SC16, `#208`).
    pub(super) fn can_change_stance(self) -> bool {
        self.stance.is_some_and(|entry| matches!(entry.name, "SWITCH TO CRANE" | "CRANE STANCE"))
            || self.sf4_stance.is_some()
    }

    /// The pads of a change of stance, when its fighter is free to make one.
    pub(super) fn stance_change(self, view: Percept) -> Option<Plan> {
        let entry = self.stance.or(self.sf4_stance).filter(|_| self.control.free && self.can_change_stance())?;
        Some(self.pad(&entry, view, 1))
    }

    pub(super) fn can_interrupt(self) -> bool {
        self.control.airborne || self.control.cancel || self.control.special
    }

    pub(super) fn can_tap_parry(self) -> bool {
        self.can_parry && (self.control.free || self.control.airborne)
    }

    pub(super) fn can_fadc(self) -> bool {
        self.fadc && self.control.special
    }

    pub(super) fn utility_command(
        self,
        spot: crate::utility::Spot,
        action: crate::utility::Action,
    ) -> Option<(MoveEntry, usize)> {
        use crate::utility::Action::*;
        match action {
            Super if self.control.cancel || (self.control.free && spot.distance < 105) => self.super_move,
            AlphaCounter if self.control.blockstun => self.guard_cancel.map(|entry| (entry, 2)),
            VTrigger if (self.control.cancel || self.control.free) && spot.distance < 108 => {
                self.install.map(|entry| (entry, 2))
            }
            DriveImpact if self.control.cancel || (self.control.free && spot.attack_incoming) => {
                self.armor.map(|entry| (entry, 2))
            }
            DriveRush if self.control.free && (80..=148).contains(&spot.distance) => {
                self.rush.map(|entry| (entry, 1))
            }
            _ => None,
        }
    }

    fn pad(&self, entry: &MoveEntry, view: Percept, strength: usize) -> Plan {
        let mut frames = alpha_motion(entry, view.toward() > 0, strength, true);
        if self.family == EraFamily::Sf6
            && entry.kind == MoveKind::System
            && entry.produces == sf6::RUSH
        {
            // The preceding plan may have left Forward held.
            frames.insert(0, 0);
        }
        Plan::of(PlanKind::Act, &frames)
    }

    /// Reactions use the delayed opponent view. The current local state is
    /// used only for whether our own controls can execute the chosen input.
    /// The stronger play (SC8, `#200`) parries or meets with an armoured
    /// move only an attack or a projectile it has just seen start, once,
    /// not every frame one is in view.
    pub(super) fn react(
        self,
        rng: &mut Rng,
        level: Level,
        view: Percept,
        before: Percept,
        felt: Percept,
        stronger: bool,
        spend_super: bool,
    ) -> Option<Plan> {
        let toward = if view.toward() > 0 { RIGHT } else { LEFT };
        let away = if toward == RIGHT { LEFT } else { RIGHT };
        let threatened = view.foe.attacking() && view.gap() < 80
            || (view.shot && (view.shot_x - view.me.x).abs() < 80);
        // A normal seen start in reach. A special is guarded instead: it
        // may be one to punish as it whiffs.
        let started = view.foe.normal()
            && !view.foe.jumping()
            && (!before.foe.attacking() || before.foe.move_id != view.foe.move_id)
            && view.gap() < 80;
        let shot_started = view.shot && !before.shot && (view.shot_x - view.me.x).abs() < SHOT_ANSWERED;
        // A Drive Rush seen start, from as far as one carries him in: met
        // by an armoured move, which takes the normal he rushes into.
        let rushed = self.family == EraFamily::Sf6
            && self.foe_family == Some(EraFamily::Sf6)
            && view.foe.move_id == sf6::RUSH
            && before.foe.move_id != sf6::RUSH
            && view.gap() < RUSH_ANSWERED;
        if self.can_air_block && self.control.airborne && threatened && rng.chance(level.guard()) {
            return Some(Plan::of(PlanKind::Act, &[away; 12]));
        }
        if self.control.blockstun && (view.me.guarding() || threatened) {
            let affordable = !stronger || self.keeps_drive(2 * STOCK_AT);
            let guard_cancel = if stronger { self.sf5.map_or(self.guard_cancel, |sf5| sf5.reversal) } else { self.guard_cancel };
            if let Some(entry) = guard_cancel.filter(|_| affordable && rng.chance(level.guard() / 2)) {
                return Some(if stronger && self.sf5.is_some() { Self::sf5_pad(entry, felt, 2) } else { self.pad(&entry, view, 2) });
            }
            return None;
        }
        let hit_confirm = view.foe.health < before.foe.health && view.gap() < 90;
        let wakeup = (self.control.waking || (before.me.reacting() && !view.me.reacting()))
            && view.gap() < 68;
        if (hit_confirm || wakeup)
            && spend_super
            && (self.control.free || self.control.cancel)
            && rng.chance(level.quality())
        {
            if stronger && self.family == EraFamily::Sf4 {
                // Ultra spends Revenge independently of Super. A seen
                // hit can earn either finisher; a wake-up keeps the Ultra
                // for a confirm instead of throwing it into a guard.
                if let Some(plan) = hit_confirm.then(|| self.sf4_ultra(felt)).flatten().or_else(|| self.sf4_super(felt)) {
                    return Some(plan);
                }
            } else if stronger && self.sf5.is_some() {
                if let Some(plan) = self.sf5_super(felt) { return Some(plan) }
            } else if stronger && self.sf6.is_some() {
                if let Some(plan) = self.sf6_super(felt) { return Some(plan) }
            } else if let Some((entry, strength)) = self.super_move {
                return Some(self.pad(&entry, view, strength));
            }
        }
        let guarded = view.foe.guarding() && !before.foe.guarding();
        if stronger && self.sf4_fadc_ready() && level.spends_meter() && (hit_confirm || guarded || threatened) && rng.chance(level.guard() / 2) {
            if let Some(plan) = self.sf4_fadc(felt, hit_confirm) {
                return Some(plan);
            }
        }
        if !stronger && self.fadc
            && self.control.special
            && (hit_confirm || threatened)
            && rng.chance(level.guard() / 2)
        {
            let focus = button(1) | button(4);
            return Some(Plan::of(
                PlanKind::Act,
                &[focus, focus | toward, focus, focus | toward, toward, 0],
            ));
        }
        if stronger && rushed && self.control.free && self.keeps_drive(STOCK_AT) && rng.chance(level.guard() / 2) {
            if let Some(armor) = self.armor {
                return Some(self.pad(&armor, view, 1));
            }
        }
        let answered = if stronger { started || shot_started } else { threatened };
        if !(self.control.free || (self.can_parry && self.control.airborne))
            || !answered
            || !rng.chance(level.guard() / 2)
        {
            return None;
        }
        if stronger && started && self.family == EraFamily::Sf6 {
            // A normal seen start: Drive Parried, or met with a Drive
            // Impact, which takes its hit on its armour. (The other era
            // styles answer as below.)
            let evade = self.evade.filter(|_| self.keeps_drive(sf6::DRIVE_PARRY_START_COST));
            let armor = self.armor.filter(|_| self.keeps_drive(STOCK_AT));
            let choice = match (evade, armor) {
                (Some(evade), Some(armor)) => if rng.chance(50) { armor } else { evade },
                (evade, armor) => evade.or(armor)?,
            };
            return Some(self.pad(&choice, view, 1));
        }
        if stronger {
            if let Some(sf5) = self.sf5 {
                let entry = sf5.counter.or(sf5.shift)?;
                return (!self.control.airborne).then(|| Self::sf5_pad(entry, felt, 1));
            }
        }
        if self.can_parry {
            let direction = if view.foe.low_attack() && !self.control.airborne {
                DOWN
            } else {
                toward
            };
            return Some(Plan::of(PlanKind::Act, &[0, direction, 0, 0]));
        }
        self.evade
            .or(self.armor)
            .map(|entry| self.pad(&entry, view, 1))
    }

    pub(super) fn neutral(self, rng: &mut Rng, level: Level, view: Percept) -> Option<Plan> {
        if !self.control.free
            || view.foe.attacking()
            || view.foe.reacting()
            || !rng.chance(level.quality() / 4)
        {
            return None;
        }
        let gap = view.gap();
        if let Some(entry) = self.stance.filter(|_| gap >= 56 && rng.chance(level.quality() / 4)) {
            return Some(self.pad(&entry, view, 1));
        }
        let choice = if view.foe.guarding() && gap < 56 {
            self.mix_up.or(self.armor)
        } else if gap < 85 {
            self.install.or(self.enhanced).or(self.armor)
        } else if gap > 105 {
            self.stance.or(self.skill).or(self.rush).or(self
                .enhanced
                .filter(|m| m.input.motion == [Dir::Down, Dir::DownForward, Dir::Forward]))
        } else if self.family == EraFamily::Sf6 {
            self.rush
        } else {
            self.enhanced
        };
        choice.map(|entry| self.pad(&entry, view, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RulesetPreset;
    use crate::era_fight::sf5;
    use crate::era::StylePick;
    use crate::moves::{Availability, MoveInput};
    use crate::roster::TournamentFighter::Ryu;

    const QCF: &[Dir] = &[Dir::Down, Dir::DownForward, Dir::Forward];

    fn entry(
        name: &'static str,
        slot: u8,
        kind: MoveKind,
        motion: &'static [Dir],
        button: ButtonClass,
    ) -> MoveEntry {
        MoveEntry {
            name,
            kind,
            produces: slot,
            input: MoveInput { motion, button },
            recognizer: Recognizer::FamilyMove,
            call_out: name,
            available: Availability {
                fighters: &[Ryu],
                presets: &RulesetPreset::ALL,
                player_table: false,
            },
        }
    }

    fn fighter(revision: crate::era::Revision, loadout: Loadout, meter: u8) -> EraFighterState {
        let mut state = EraFighterState::default();
        state.fighter = Some(Ryu);
        state.style = StylePick::for_revision(revision, loadout);
        state.max_sub_hp = 960;
        state.super_meter = meter;
        state
    }

    /// Real search pad scripts, deliberately carrying misleading labels:
    /// route eligibility must inspect their commands rather than names.
    fn sf4_recipe(actions: &[&str], super_spent: u16) -> crate::route_book::Route {
        use crate::era::Revision;
        let mut route = crate::route_book::book(Ryu, Revision::Usf4, Loadout::Ultra1).unwrap().routes[0].clone();
        let available = crate::route_book::ground_actions();
        route.steps = actions.iter().map(|name| {
            let action = available.iter().find(|action| action.name == *name).unwrap();
            crate::route_book::Step { name: "meterless normal".into(), wait: 0, window: (0, 0), hold: action.hold, pads: action.pads.clone() }
        }).collect();
        route.super_spent = super_spent;
        route.drive_spent = 0;
        route
    }

    #[test]
    fn low_difficulty_keeps_a_drive_reserve_and_obeys_raw_rush_setting() {
        use crate::era::Revision;
        let mut config = MatchConfig::definitive();
        config.gauge_mode = GaugeMode::Unified;
        let state = fighter(Revision::Sf6, Loadout::Standard, 3 * STOCK_AT);
        let moves = [
            entry(
                "DRIVE RUSH",
                sf6::RUSH,
                MoveKind::System,
                &[Dir::Forward, Dir::Neutral, Dir::Forward],
                ButtonClass::MediumPair,
            ),
            entry(
                "DRIVE REVERSAL",
                sf6::REVERSAL,
                MoveKind::System,
                &[Dir::Forward],
                ButtonClass::HeavyPair,
            ),
            entry(
                "OD HADOKEN",
                0x24,
                MoveKind::Special,
                QCF,
                ButtonClass::PairPunch,
            ),
        ];
        let control = EraControl {
            free: true,
            ..EraControl::default()
        };
        let easy = Tactics::of(&state, &moves, config, Level::Easy, control);
        assert!(easy.rush.is_some());
        assert!(easy.enhanced.is_some());
        assert!(easy.guard_cancel.is_some());
        let below_reserve = fighter(Revision::Sf6, Loadout::Standard, 2 * STOCK_AT);
        let easy = Tactics::of(&below_reserve, &moves, config, Level::Easy, control);
        assert!(easy.rush.is_none());
        assert!(easy.enhanced.is_none());
        assert!(easy.guard_cancel.is_none());
        let hard = Tactics::of(&state, &moves, config, Level::Hard, control);
        assert!(hard.enhanced.is_some());
        assert!(hard.guard_cancel.is_some());
        config.disable_raw_drive_rush = true;
        assert!(
            Tactics::of(&state, &moves, config, Level::VeryHard, control)
                .rush
                .is_none()
        );
    }

    /// The CPU at x 200 and the opponent at `foe_x`, on the frame before
    /// he is seen to begin move `move_id`, and on that frame.
    fn seen_begin(foe_x: i16, move_id: u8) -> (Percept, Percept) {
        let me = super::super::Body { present: true, x: 200, hurt: (190, 210), move_id: 1, health: 48, ..Default::default() };
        let foe = super::super::Body { present: true, x: foe_x, hurt: (foe_x - 10, foe_x + 10), move_id: 1, health: 48, ..Default::default() };
        let before = Percept { fighting: true, me, foe, ..Percept::default() };
        let mut view = before;
        view.foe.move_id = move_id;
        view.foe.state = 0xc2;
        (view, before)
    }

    /// Every answer the stronger play gives over a spread of random seeds.
    fn stronger_answers(tactics: Tactics, view: Percept, before: Percept) -> Vec<Vec<u16>> {
        (0..64u64)
            .filter_map(|seed| tactics.react(&mut Rng(seed * 2 + 1), Level::VeryHard, view, before, view, true, true))
            .map(|plan| plan.pads[..usize::from(plan.len)].to_vec())
            .collect()
    }

    #[test]
    fn with_the_stronger_play_the_other_era_styles_keep_their_guard_cancels() {
        // SC8 (`#200`): the Drive rule is SF6's; an Alpha Counter or a
        // V-Reversal pays from its own meter.
        use crate::era::Revision;
        let config = MatchConfig::definitive();
        let guarding = |view: &mut Percept| view.me.move_id = 4;
        let (mut view, mut before) = seen_begin(240, 0x10);
        guarding(&mut view);
        guarding(&mut before);
        let blockstun = EraControl { blockstun: true, ..EraControl::default() };
        // SFZ: an Alpha Counter.
        let mut alpha = fighter(Revision::Sfz3, Loadout::AIsm, 0);
        alpha.set_gauge_mode(GaugeMode::EraDefault);
        alpha.super_meter = STOCK_AT;
        let mut counter = entry("ALPHA COUNTER (P)", 0x2c, MoveKind::System, &[Dir::Back, Dir::DownBack, Dir::Down], ButtonClass::Punch);
        counter.recognizer = Recognizer::AlphaCounter;
        let sfz = Tactics::of(&alpha, &[counter], config, Level::VeryHard, blockstun);
        assert!(sfz.keeps_drive(2 * STOCK_AT), "no Drive gauge to keep outside SF6");
        assert!(!stronger_answers(sfz, view, before).is_empty(), "an Alpha Counter out of a guard");
        // SF5: a V-Reversal, its V-Gauge here the unified meter.
        let (mut v, config) = sf5_fighter(Ryu, Revision::Sf5Ce, Loadout::Skill1Trigger1, GaugeMode::Unified);
        v.super_meter = 2 * STOCK_AT;
        let sf5 = Tactics::of(&v, &sf5_moves(&v, config), config, Level::VeryHard, blockstun);
        assert!(!stronger_answers(sf5, view, before).is_empty(), "a V-Reversal out of a guard");
    }

    #[test]
    fn with_the_stronger_play_sf3_parries_and_sf4_and_sf5_use_their_own_answers_to_a_normal() {
        // SC8 (`#200`): the Drive Parry and Drive Impact choice is SF6's
        // alone; the other styles answer a normal seen to start in reach
        // with their own tools.
        use crate::era::{loadouts, Revision};
        let free = EraControl { free: true, ..EraControl::default() };
        let (view, before) = seen_begin(240, 0x10);
        let config = MatchConfig::definitive();
        // SF3: the parry's tap toward him.
        let sf3 = Tactics::of(&fighter(Revision::Sf3ThreeS, Loadout::Art2, 128), &[], config, Level::VeryHard, free);
        assert!(stronger_answers(sf3, view, before).contains(&vec![0, RIGHT, 0, 0]), "SF3 parries");
        // SF4: the Focus Attack.
        let focus = entry("FOCUS ATTACK", sf4::FOCUS, MoveKind::System, &[], ButtonClass::MediumPair);
        let sf4 = Tactics::of(&fighter(Revision::Sf4, loadouts(Revision::Sf4)[0], 4 * STOCK_AT), &[focus], config, Level::VeryHard, free);
        let medium_pair = button(1) | button(4);
        assert!(stronger_answers(sf4, view, before).iter().any(|pads| pads.iter().any(|&p| p & medium_pair == medium_pair)), "SF4 focuses");
        // SF5: the V-Shift.
        let mut unified = config;
        unified.gauge_mode = GaugeMode::Unified;
        let (mut v, _) = sf5_fighter(crate::roster::TournamentFighter::Ken, Revision::Sf5Ce, Loadout::Skill1Trigger1, GaugeMode::Unified);
        v.super_meter = 2 * STOCK_AT;
        let sf5 = Tactics::of(&v, &sf5_moves(&v, unified), unified, Level::VeryHard, free);
        let shift_pair = button(2) | button(4);
        assert!(stronger_answers(sf5, view, before).iter().any(|pads| pads.iter().any(|&p| p & shift_pair == shift_pair)), "SF5 shifts with its native HP+MK chord");
    }

    #[test]
    fn a_move_another_style_numbers_as_drive_rush_draws_no_drive_impact() {
        // SC8 (`#200`): an Alpha-style fighter's move 0x2c (a Murderer Chain,
        // a Jaguar Tooth) is no Drive Rush.
        use crate::era::Revision;
        let mut config = MatchConfig::definitive();
        config.gauge_mode = GaugeMode::Unified;
        let impact = entry("DRIVE IMPACT", sf6::IMPACT, MoveKind::System, &[], ButtonClass::HeavyPair);
        let free = EraControl { free: true, ..EraControl::default() };
        let kit = Tactics::of(&fighter(Revision::Sf6, Loadout::Standard, 5 * STOCK_AT), &[impact], config, Level::VeryHard, free);
        let (view, before) = seen_begin(330, sf6::RUSH);
        let heavy_pair = button(2) | button(5);
        let impacts = |answers: Vec<Vec<u16>>| answers.iter().filter(|pads| pads.iter().any(|&p| p & heavy_pair == heavy_pair)).count();
        assert!(impacts(stronger_answers(kit.against(Some(EraFamily::Sf6)), view, before)) > 0, "an SF6 Drive Rush is met");
        assert_eq!(impacts(stronger_answers(kit.against(Some(EraFamily::SfZ)), view, before)), 0, "an Alpha move is not");
    }

    #[test]
    fn the_stronger_play_spends_no_drive_in_neutral_down_to_burnout() {
        use crate::era::Revision;
        let mut config = MatchConfig::definitive();
        config.gauge_mode = GaugeMode::Unified;
        let moves = [
            entry("DRIVE RUSH", sf6::RUSH, MoveKind::System, &[Dir::Forward, Dir::Neutral, Dir::Forward], ButtonClass::MediumPair),
            entry("DRIVE IMPACT", sf6::IMPACT, MoveKind::System, &[], ButtonClass::HeavyPair),
        ];
        let control = EraControl { free: true, ..EraControl::default() };
        let at_cost = Tactics::of(&fighter(Revision::Sf6, Loadout::Standard, STOCK_AT), &moves, config, Level::VeryHard, control);
        assert!(at_cost.rush.is_some() && at_cost.armor.is_some(), "the old play would spend the last stock");
        let safe = at_cost.spending_safely(Level::VeryHard);
        assert!(safe.rush.is_none() && safe.armor.is_none(), "the stronger play keeps it");
        let above = Tactics::of(&fighter(Revision::Sf6, Loadout::Standard, STOCK_AT + 3), &moves, config, Level::VeryHard, control).spending_safely(Level::VeryHard);
        assert!(above.rush.is_some() && above.armor.is_some());
    }

    #[test]
    fn how_much_drive_the_stronger_play_keeps_back_is_a_level_value_and_easy_spends_none() {
        // SC11 (`#203`): with a full gauge (three stocks of the unified
        // gauge, which stands in for Drive here) Easy spends nothing
        // on its own; the others spend down to their level's share, the
        // top all of it but the last point.
        use crate::era::Revision;
        let mut config = MatchConfig::definitive();
        config.gauge_mode = GaugeMode::Unified;
        let moves = [
            entry("DRIVE RUSH", sf6::RUSH, MoveKind::System, &[Dir::Forward, Dir::Neutral, Dir::Forward], ButtonClass::MediumPair),
            entry("DRIVE IMPACT", sf6::IMPACT, MoveKind::System, &[], ButtonClass::HeavyPair),
        ];
        let control = EraControl { free: true, ..EraControl::default() };
        let full = 3 * u16::from(STOCK_AT);
        let tactics = |drive: u16, level: Level| {
            Tactics::of(&fighter(Revision::Sf6, Loadout::Standard, drive as u8), &moves, config, level, control).spending_safely(level)
        };
        let easy = tactics(full, Level::Easy);
        assert!(easy.rush.is_none() && easy.armor.is_none() && easy.rush_in(Percept::default()).is_none(), "Easy spent Drive");
        for level in [Level::Normal, Level::Hard, Level::VeryHard] {
            assert!(tactics(full, level).armor.is_some(), "{level:?} spends from a full gauge");
            // Impact costs a stock: just enough over the level's share, and
            // one point short of it.
            let kept = level.drive_kept();
            assert!(tactics(kept + u16::from(STOCK_AT) + 1, level).armor.is_some(), "{level:?} above its share");
            assert!(tactics(kept + u16::from(STOCK_AT) - 1, level).armor.is_none() || kept == 0, "{level:?} below its share");
        }
    }

    #[test]
    fn the_stronger_play_never_rushes_into_burnout() {
        // SC8 (`#200`): at Very Difficult there is no reserve, but a Drive
        // Rush that would empty the gauge is not entered.
        use crate::era::Revision;
        let mut config = MatchConfig::definitive();
        config.gauge_mode = GaugeMode::Unified;
        let moves = [entry("DRIVE RUSH", sf6::RUSH, MoveKind::System, &[Dir::Forward, Dir::Neutral, Dir::Forward], ButtonClass::MediumPair)];
        let control = EraControl { free: true, ..EraControl::default() };
        let view = Percept::default();
        let at_cost = Tactics::of(&fighter(Revision::Sf6, Loadout::Standard, STOCK_AT), &moves, config, Level::VeryHard, control);
        assert!(at_cost.rush.is_some(), "the kit would rush");
        assert!(at_cost.rush_in(view).is_none(), "not down to an empty gauge");
        let above = Tactics::of(&fighter(Revision::Sf6, Loadout::Standard, STOCK_AT + 1), &moves, config, Level::VeryHard, control);
        assert!(above.rush_in(view).is_some());
    }

    #[test]
    fn unified_cpu_requires_three_stocks_for_st_supers_and_x_ism() {
        use crate::era::Revision;

        let mut config = MatchConfig::definitive();
        config.gauge_mode = GaugeMode::Unified;
        let moves = [entry(
            "SHINKUU HADOKEN",
            0x29,
            MoveKind::Super,
            crate::era_fight::DOUBLE_QUARTER_CIRCLE,
            ButtonClass::Punch,
        )];
        for (revision, loadout) in [(Revision::St, Loadout::Standard), (Revision::Sfz3, Loadout::XIsm)] {
            for (points, can_super) in [(3 * STOCK_AT - 1, false), (3 * STOCK_AT, true)] {
                let state = fighter(revision, loadout, points);
                let tactics = Tactics::of(&state, &moves, config, Level::Normal, EraControl::default());
                assert_eq!(tactics.super_move.is_some(), can_super, "{revision:?} {points}");
            }
        }
    }

    #[test]
    fn native_art_with_one_long_bar_does_not_meet_the_two_stock_ex_threshold() {
        use crate::era::Revision;

        let state = fighter(Revision::Sf3ThreeS, Loadout::Art2, 128);
        let moves = [entry(
            "EX HADOKEN",
            0x24,
            MoveKind::Special,
            QCF,
            ButtonClass::PairPunch,
        )];
        let tactics = Tactics::of(
            &state,
            &moves,
            MatchConfig::definitive(),
            Level::Hard,
            EraControl::default(),
        );
        assert_eq!(state.sf3_stock_cost(), Some(128));
        assert!(tactics.enhanced.is_none());
    }

    #[test]
    fn the_stronger_sf3_play_spends_an_ex_moves_real_cost_in_every_super_art() {
        use crate::era::Revision;
        let free = EraControl { free: true, ..EraControl::default() };
        for revision in [Revision::Sf3TwoI, Revision::Sf3ThreeS] {
            for loadout in [Loadout::Art1, Loadout::Art2, Loadout::Art3] {
                for mode in [GaugeMode::EraDefault, GaugeMode::Unified] {
                    let mut cfg = MatchConfig::definitive();
                    cfg.gauge_mode = mode;
                    let mut state = fighter(revision, loadout, 0);
                    state.set_gauge_mode(mode);
                    let cost = state.sf3_ex_cost();
                    let moves = state.sf3_active_moves();
                    for meter in [cost - 1, cost] {
                        state.super_meter = meter;
                        let old = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
                        assert!(old.enhanced.is_none(), "the old play keeps its two-stock gate");
                        let new = old.spending_safely(Level::VeryHard);
                        assert_eq!(new.sf3_ex_ready(), meter == cost, "{revision:?} {loadout:?} {mode:?} at {meter}");
                        assert_eq!(new.enhanced.is_some(), meter == cost);
                        assert_eq!(new.sf3_ex(Percept::default()).is_some(), meter == cost);
                    }
                }
            }
        }
    }

    #[test]
    fn sf3_plan_tools_obey_the_fighters_controls_gauge_and_live_move_list() {
        use crate::era::Revision;
        let cfg = MatchConfig::definitive();
        let free = EraControl { free: true, ..EraControl::default() };
        let mut state = fighter(Revision::Sf3ThreeS, Loadout::Art2, 128);
        state.set_gauge_mode(GaugeMode::EraDefault);
        let moves = state.sf3_active_moves();
        let ready = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
        let view = Percept::default();
        let overhead = ready.sf3_overhead(view).expect("the real overhead is available");
        assert!(overhead.pads[..usize::from(overhead.len)].contains(&(button(1) | button(4))));
        let ex = ready.sf3_ex(view).expect("the real EX move is available");
        assert!(ex.pads[..usize::from(ex.len)].iter().any(|pad| (pad & (button(0) | button(1) | button(2))).count_ones() >= 2));

        for control in [EraControl::default(), EraControl { airborne: true, ..free }] {
            let busy = Tactics::of(&state, &moves, cfg, Level::VeryHard, control);
            assert!(!busy.sf3_overhead_ready() && busy.sf3_overhead(view).is_none());
            assert!(!busy.sf3_ex_ready() && busy.sf3_ex(view).is_none());
        }
        let absent = Tactics::of(&state, &[], cfg, Level::VeryHard, free);
        assert!(!absent.sf3_overhead_ready() && !absent.sf3_ex_ready());
        state.set_gauge_mode(GaugeMode::NoGauges);
        let mut no_gauges = cfg;
        no_gauges.gauge_mode = GaugeMode::NoGauges;
        let disabled = Tactics::of(&state, &state.sf3_active_moves(), no_gauges, Level::VeryHard, free);
        assert!(!disabled.sf3_overhead_ready() && !disabled.sf3_ex_ready());

        let mut ng = fighter(Revision::Sf3Ng, Loadout::Art2, 128);
        ng.set_gauge_mode(GaugeMode::EraDefault);
        let original = Tactics::of(&ng, &ng.sf3_active_moves(), cfg, Level::VeryHard, free);
        assert!(original.sf3_overhead_ready(), "New Generation has the universal overhead");
        assert!(!original.sf3_ex_ready(), "native New Generation has no EX move");
    }

    #[test]
    fn an_sf3_ex_motion_uses_its_felt_facing_even_when_the_delayed_opponent_is_behind_it() {
        use crate::era::Revision;
        let mut state = fighter(Revision::Sf3ThreeS, Loadout::Art2, 40);
        state.set_gauge_mode(GaugeMode::EraDefault);
        let tactics = Tactics::of(&state, &state.sf3_active_moves(), MatchConfig::definitive(), Level::VeryHard, EraControl { free: true, ..EraControl::default() });
        for (facing_left, opponent_x, forward, back) in [(false, 100, RIGHT, LEFT), (true, 300, LEFT, RIGHT)] {
            let mut view = Percept::default();
            view.me.x = 200;
            view.me.facing_left = facing_left;
            view.foe.x = opponent_x;
            let plan = tactics.sf3_ex(view).expect("an affordable EX");
            let pads = &plan.pads[..usize::from(plan.len)];
            assert!(pads.iter().any(|&pad| pad & forward != 0 && pad & 0x3f0 != 0), "motion ends facing {forward:x}: {pads:x?}");
            assert!(pads.iter().all(|&pad| pad & back == 0), "delayed opponent must not reverse the motion: {pads:x?}");
        }
    }

    #[test]
    fn sf4_plans_keep_super_and_ultra_separate_and_use_their_own_resources() {
        use crate::era::{loadouts, Revision};
        let free = EraControl { free: true, ..EraControl::default() };
        for revision in [Revision::Sf4, Revision::Ssf4, Revision::Ae2012, Revision::Usf4] {
            for loadout in loadouts(revision) {
                for mode in [GaugeMode::EraDefault, GaugeMode::Unified] {
                    let mut cfg = MatchConfig::definitive();
                    cfg.gauge_mode = mode;
                    let mut state = fighter(revision, loadout, 0);
                    state.set_gauge_mode(mode);
                    let moves = state.sf4_active_moves();
                    let super_cost = state.sf4_super_cost();
                    for meter in [super_cost - 1, super_cost] {
                        state.super_meter = meter;
                        let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
                        assert_eq!(tactics.sf4_super_ready(), meter == super_cost, "{revision:?} {loadout:?} {mode:?}");
                        assert_eq!(tactics.sf4_super(Percept::default()).is_some(), meter == super_cost);
                        assert_eq!(tactics.sf4_ultra_ready(), mode == GaugeMode::Unified && meter == super_cost, "native Super meter cannot buy Revenge");
                    }
                    if mode == GaugeMode::EraDefault {
                        state.super_meter = 0;
                        for revenge in [sf4::REVENGE_MAX / 2 - 1, sf4::REVENGE_MAX / 2] {
                            state.set_sf4_practice_revenge(Some(revenge));
                            let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
                            assert!(!tactics.sf4_super_ready(), "Revenge cannot buy a Super");
                            assert_eq!(tactics.sf4_ultra_ready(), revenge == sf4::REVENGE_MAX / 2);
                            assert_eq!(tactics.sf4_ultra(Percept::default()).is_some(), revenge == sf4::REVENGE_MAX / 2);
                        }
                    }
                    let easy = Tactics::of(&state, &moves, cfg, Level::Easy, free);
                    assert!(!easy.sf4_super_ready() && !easy.sf4_ultra_ready(), "Easy keeps its meter");
                }
            }
        }
    }

    #[test]
    fn sf4_focus_and_fadc_obey_the_controls_cost_and_felt_facing() {
        use crate::era::Revision;
        let free = EraControl { free: true, ..EraControl::default() };
        let special = EraControl { special: true, ..EraControl::default() };
        for mode in [GaugeMode::EraDefault, GaugeMode::Unified] {
            let mut cfg = MatchConfig::definitive();
            cfg.gauge_mode = mode;
            let mut state = fighter(Revision::Usf4, Loadout::Ultra1, 0);
            state.set_gauge_mode(mode);
            let moves = state.sf4_active_moves();
            let cost = state.sf4_focus_cancel_cost();
            for meter in [cost - 1, cost] {
                state.super_meter = meter;
                let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, special);
                assert_eq!(tactics.sf4_fadc_ready(), meter == cost);
                for facing_left in [false, true] {
                    let mut view = Percept::default();
                    view.me.facing_left = facing_left;
                    view.me.x = 200;
                    view.foe.x = if facing_left { 250 } else { 150 };
                    let toward = if facing_left { LEFT } else { RIGHT };
                    for forward in [false, true] {
                        let plan = tactics.sf4_fadc(view, forward);
                        assert_eq!(plan.is_some(), meter == cost);
                        if let Some(plan) = plan {
                            let pads = &plan.pads[..usize::from(plan.len)];
                            let direction = if forward { toward } else { RIGHT + LEFT - toward };
                            assert_eq!(pads[2] & 0xf, direction);
                            assert_eq!(pads[3] & 0xf, 0);
                            assert_eq!(pads[4] & 0xf, direction);
                            assert!(pads[1..=4].iter().all(|pad| pad & (button(1) | button(4)) == button(1) | button(4)));
                        }
                    }
                }
            }
            let ready = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
            assert!(!ready.sf4_fadc_ready(), "a free fighter cannot FADC nothing");
            let plan = ready.sf4_focus(Percept::default()).expect("a free fighter can Focus without a meter cost");
            let pads = &plan.pads[..usize::from(plan.len)];
            assert_eq!(pads[0], 0);
            assert_eq!(pads.last(), Some(&0));
            assert_eq!(pads.iter().filter(|&&pad| pad == button(1) | button(4)).count(), 22, "charge past level two before releasing");
            for control in [EraControl::default(), EraControl { airborne: true, ..free }] {
                let busy = Tactics::of(&state, &moves, cfg, Level::VeryHard, control);
                assert!(!busy.sf4_focus_ready() && busy.sf4_focus(Percept::default()).is_none());
            }
            let airborne = Tactics::of(&state, &moves, cfg, Level::VeryHard, EraControl { airborne: true, ..special });
            assert!(!airborne.sf4_fadc_ready(), "no airborne FADC");
        }
    }

    #[test]
    fn sf4_stronger_ex_cost_stance_and_no_gauge_gates_preserve_the_old_play() {
        use crate::era::Revision;
        let mut cfg = MatchConfig::definitive();
        let free = EraControl { free: true, ..EraControl::default() };
        let mut state = fighter(Revision::Usf4, Loadout::Ultra1, STOCK_AT);
        state.set_gauge_mode(GaugeMode::EraDefault);
        let moves = state.sf4_active_moves();
        let old = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
        assert!(old.enhanced.is_none(), "old EX choices keep their two-stock gate");
        assert!(old.spending_safely(Level::VeryHard).enhanced.is_some(), "the stronger play can spend one EX stock");
        assert!(old.sf4_ex_ready() && old.sf4_ex(Percept::default()).is_some());
        assert!(old.spending_safely(Level::Easy).enhanced.is_none(), "Easy's neutral fallback keeps its meter too");
        let easy = Tactics::of(&state, &moves, cfg, Level::Easy, free);
        assert!(!easy.sf4_ex_ready() && easy.spending_safely(Level::Easy).enhanced.is_none());
        state.super_meter = 2 * STOCK_AT;
        let legacy_easy = Tactics::of(&state, &moves, cfg, Level::Easy, free);
        assert!(legacy_easy.enhanced.is_some(), "the old play retains its existing EX option");
        assert!(legacy_easy.spending_safely(Level::Easy).enhanced.is_none());
        state.super_meter = STOCK_AT;
        state.super_meter -= 1;
        let short = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
        assert!(short.spending_safely(Level::VeryHard).enhanced.is_none());
        assert!(!short.sf4_ex_ready() && short.sf4_ex(Percept::default()).is_none());

        state.fighter = Some(crate::roster::TournamentFighter::Gen);
        let gen = Tactics::of(&state, &state.sf4_active_moves(), cfg, Level::VeryHard, free);
        assert!(gen.stance.is_none(), "native Gen's old reading is unchanged");
        assert!(gen.can_change_stance());
        assert_eq!(gen.spending_safely(Level::VeryHard).stance.unwrap().name, "CRANE STANCE");
        let change = gen.stance_change(Percept::default()).expect("a plan can enter the Crane");
        assert!(change.pads[..usize::from(change.len)].contains(&(button(3) | button(4) | button(5))));

        state.set_gauge_mode(GaugeMode::NoGauges);
        cfg.gauge_mode = GaugeMode::NoGauges;
        let disabled = Tactics::of(&state, &state.sf4_active_moves(), cfg, Level::VeryHard, free);
        assert!(!disabled.sf4_focus_ready() && !disabled.sf4_fadc_ready());
        assert!(!disabled.sf4_super_ready() && !disabled.sf4_ultra_ready());
        assert!(!disabled.sf4_ex_ready());
        assert!(disabled.spending_safely(Level::VeryHard).enhanced.is_none());
    }

    #[test]
    fn sf4_zero_meter_route_metadata_cannot_bypass_revenge_or_easys_meter_rule() {
        use crate::era::Revision;
        let cfg = MatchConfig::definitive();
        let mut state = fighter(Revision::Usf4, Loadout::Ultra1, 0);
        state.set_gauge_mode(GaugeMode::EraDefault);
        let recipe = sf4_recipe(&["MP", "236236+PPP"], 0);
        let moves = state.sf4_active_moves();
        let empty = Tactics::of(&state, &moves, cfg, Level::VeryHard, EraControl::default());
        assert!(!empty.sf4_route_ready(&recipe, Level::VeryHard, 0), "zero recorded Super/Drive does not make an Ultra free");
        state.set_sf4_practice_revenge(Some(sf4::REVENGE_MAX / 2));
        let ready = Tactics::of(&state, &moves, cfg, Level::VeryHard, EraControl::default());
        assert!(ready.sf4_route_ready(&recipe, Level::VeryHard, 0), "a later route step need not be executable on this frame");
        assert!(!ready.sf4_route_ready(&recipe, Level::Easy, 0), "Easy must not spend unrecorded Revenge");
        assert!(!ready.sf4_route_ready(&sf4_recipe(&["236236+PPP", "236236+PPP"], 0), Level::VeryHard, 0), "native Revenge buys at most one Ultra");

        let mut no_gauges = cfg;
        no_gauges.gauge_mode = GaugeMode::NoGauges;
        state.set_gauge_mode(GaugeMode::NoGauges);
        let disabled = Tactics::of(&state, &state.sf4_active_moves(), no_gauges, Level::VeryHard, EraControl::default());
        assert!(!disabled.sf4_route_ready(&recipe, Level::VeryHard, 0));
        assert!(disabled.sf4_route_ready(&sf4_recipe(&["MP", "LK"], 0), Level::Easy, 0), "plain routes remain available");

        let other = Tactics::of(&fighter(Revision::Sf6, Loadout::Standard, 0), &[], cfg, Level::Easy, EraControl::default());
        assert!(other.sf4_route_ready(&recipe, Level::Easy, 0), "other families are outside this gate");
    }

    #[test]
    fn sf4_route_gate_adds_unified_ultra_cost_to_the_recorded_ex_spending() {
        use crate::era::Revision;
        let mut cfg = MatchConfig::definitive();
        cfg.gauge_mode = GaugeMode::Unified;
        let recipe = sf4_recipe(&["236+PP", "236236+PPP"], u16::from(STOCK_AT));
        let mut state = fighter(Revision::Usf4, Loadout::Ultra1, 3 * STOCK_AT);
        state.set_gauge_mode(GaugeMode::Unified);
        state.super_meter = 3 * STOCK_AT;
        let moves = state.sf4_active_moves();
        let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, EraControl::default());
        assert!(tactics.sf4_ultra.is_some(), "an Ultra alone is affordable");
        assert!(!tactics.sf4_route_ready(&recipe, Level::VeryHard, u16::from(3 * STOCK_AT)), "EX first would leave too little for the Ultra");
        assert!(!tactics.sf4_route_ready(&recipe, Level::VeryHard, u16::from(4 * STOCK_AT) - 1));
        assert!(tactics.sf4_route_ready(&recipe, Level::VeryHard, u16::from(4 * STOCK_AT)), "the budget includes both costs");
        assert!(!tactics.sf4_route_ready(&recipe, Level::Easy, u16::from(4 * STOCK_AT)));
    }

    #[test]
    fn sf4_route_finishers_match_the_live_gen_stance_and_leave_naked_stance_inputs_free() {
        use crate::era::Revision;
        let cfg = MatchConfig::definitive();
        let mut state = fighter(Revision::Usf4, Loadout::Ultra1, 0);
        state.fighter = Some(crate::roster::TournamentFighter::Gen);
        state.set_gauge_mode(GaugeMode::EraDefault);
        state.set_sf4_practice_revenge(Some(sf4::REVENGE_MAX / 2));
        let mantis_moves = state.sf4_active_moves();
        let mantis = Tactics::of(&state, &mantis_moves, cfg, Level::VeryHard, EraControl::default());
        let mantis_ultra = sf4_recipe(&["236236+PPP"], 0);
        let crane_ultra = sf4_recipe(&["236236+KKK"], 0);
        assert!(mantis.sf4_route_ready(&mantis_ultra, Level::VeryHard, 0));
        assert!(!mantis.sf4_route_ready(&crane_ultra, Level::VeryHard, 0), "the Mantis cannot borrow the Crane's command");

        // The helper consumes the current move-list boundary; use the
        // Crane's actual inputs to prove that boundary independently.
        let crane_moves = [
            entry("RYUKOHA", 0x2d, MoveKind::Super, crate::era_fight::DOUBLE_QUARTER_CIRCLE, ButtonClass::TripleKick),
            entry("MANTIS STANCE", 0x1f, MoveKind::System, &[], ButtonClass::TriplePunch),
        ];
        let crane = Tactics::of(&state, &crane_moves, cfg, Level::VeryHard, EraControl::default());
        assert!(crane.sf4_route_ready(&crane_ultra, Level::VeryHard, 0));
        assert!(!crane.sf4_route_ready(&mantis_ultra, Level::VeryHard, 0), "the Crane cannot borrow the Mantis's command");

        let mut stance = sf4_recipe(&["MP"], 0);
        stance.steps[0].pads = vec![(0, 0b111_000), (0, 0b111_000), (0, 0)];
        assert!(mantis.sf4_route_ready(&stance, Level::Easy, 0), "Gen's naked stance chord spends no gauge");
        stance.steps.extend(mantis_ultra.steps.clone());
        assert!(!mantis.sf4_route_ready(&stance, Level::VeryHard, 0), "a recipe that changes stance before its finisher is ambiguous");
    }

    #[test]
    fn sf4_route_gate_requires_the_live_super_and_cannot_underprice_it() {
        use crate::era::Revision;
        let cfg = MatchConfig::definitive();
        let mut state = fighter(Revision::Usf4, Loadout::Ultra1, 4 * STOCK_AT);
        state.set_gauge_mode(GaugeMode::EraDefault);
        state.super_meter = 4 * STOCK_AT;
        let moves = state.sf4_active_moves();
        let ready = Tactics::of(&state, &moves, cfg, Level::VeryHard, EraControl::default());
        assert!(!ready.sf4_route_ready(&sf4_recipe(&["236236+HP"], 0), Level::VeryHard, u16::from(4 * STOCK_AT)), "the real Super cannot masquerade as meterless");
        let paid = sf4_recipe(&["236236+HP"], u16::from(4 * STOCK_AT));
        assert!(ready.sf4_route_ready(&paid, Level::VeryHard, u16::from(4 * STOCK_AT)));
        let absent = Tactics::of(&state, &[], cfg, Level::VeryHard, EraControl::default());
        assert!(!absent.sf4_route_ready(&paid, Level::VeryHard, u16::from(4 * STOCK_AT)), "a long motion without a live command is not a finisher route");
    }

    #[test]
    fn cpu_sends_a_real_super_motion_after_a_visible_hit_confirm() {
        use crate::era::Revision;
        let state = fighter(Revision::Sf5Ce, Loadout::Skill1Trigger1, 3 * STOCK_AT);
        let moves = [entry(
            "SHINKU HADOKEN",
            0x29,
            MoveKind::Super,
            crate::era_fight::DOUBLE_QUARTER_CIRCLE,
            ButtonClass::Punch,
        )];
        let control = EraControl {
            free: true,
            ..EraControl::default()
        };
        let before = Percept {
            me: super::super::Body {
                present: true,
                x: 200,
                ..Default::default()
            },
            foe: super::super::Body {
                present: true,
                x: 254,
                hurt: (236, 272),
                health: 40,
                ..Default::default()
            },
            ..Percept::default()
        };
        let mut confirmed = before;
        confirmed.foe.health = 36;
        let config = MatchConfig::definitive();
        let hard = Tactics::of(&state, &moves, config, Level::Hard, control);
        assert!(hard
            .react(&mut Rng(0), Level::Hard, before, before, before, false, true)
            .is_none());
        let plan = hard
            .react(&mut Rng(0), Level::Hard, confirmed, before, confirmed, false, true)
            .expect("hit confirm plans CA");
        assert_eq!(plan.pads[..3], [DOWN; 3]);
        assert!(plan.pads[..usize::from(plan.len)].contains(&(RIGHT | button(2))));
        let easy = Tactics::of(&state, &moves, config, Level::Easy, control);
        assert!(easy
            .react(&mut Rng(0), Level::Easy, confirmed, before, confirmed, false, true)
            .is_none());
    }

    #[test]
    fn retsus_real_meterless_sf4_route_passes_the_live_move_gate_without_resources() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::Retsu;

        let book = crate::route_book::book(Retsu, Revision::Usf4, Loadout::Ultra1).unwrap();
        let route = book.routes.iter().find(|route| route.name() == "LK, 236+LP").unwrap();
        assert_eq!((route.super_spent, route.drive_spent), (0, 0));

        for mode in [GaugeMode::EraDefault, GaugeMode::NoGauges] {
            let mut config = MatchConfig::definitive();
            config.gauge_mode = mode;
            let mut state = fighter(Revision::Usf4, Loadout::Ultra1, 0);
            state.fighter = Some(Retsu);
            state.set_what_if(true);
            state.set_gauge_mode(mode);
            state.set_sf4_practice_revenge(Some(0));
            assert_eq!(state.super_meter, 0);
            assert!(!state.sf4_ultra_ready());
            let moves = crate::moves::live_move_list(Retsu, config.preset, true, &state);
            assert!(moves.iter().any(|entry| entry.name == "KIAI SHOT" && entry.recognizer == Recognizer::FamilyMove), "Retsu's actual SF4 what-if kit is in force");
            for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                let tactics = Tactics::of(&state, &moves, config, level, EraControl::default());
                assert!(tactics.sf4_route_ready(route, level, 0), "{mode:?} {level:?}: the single quarter circle is not a Super or Ultra");
            }
        }
    }

    fn sf5_fighter(fighter_id: crate::roster::TournamentFighter, revision: crate::era::Revision, loadout: Loadout, mode: GaugeMode) -> (EraFighterState, MatchConfig) {
        let mut state = fighter(revision, loadout, 0);
        state.fighter = Some(fighter_id);
        state.set_what_if(!matches!(fighter_id, crate::roster::TournamentFighter::Ryu | crate::roster::TournamentFighter::Ken | crate::roster::TournamentFighter::Birdie | crate::roster::TournamentFighter::Sagat));
        state.set_gauge_mode(mode);
        let mut cfg = MatchConfig::definitive();
        cfg.gauge_mode = mode;
        (state, cfg)
    }

    fn sf5_moves(state: &EraFighterState, cfg: MatchConfig) -> Vec<MoveEntry> {
        crate::moves::live_move_list(state.fighter.unwrap(), cfg.preset, true, state)
    }

    #[test]
    fn sf5_live_costs_controls_and_easy_policy_keep_legacy_choices_separate() {
        use crate::era::Revision;
        let free = EraControl { free: true, ..EraControl::default() };
        for fighter_id in crate::roster::TournamentFighter::ALL {
            let (mut state, mut cfg) = sf5_fighter(fighter_id, Revision::Sf5Ce, Loadout::Skill1Trigger1, GaugeMode::EraDefault);
            let moves = sf5_moves(&state, cfg);
            for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                for meter in [STOCK_AT - 1, STOCK_AT, 3 * STOCK_AT - 1, 3 * STOCK_AT] {
                    state.super_meter = meter;
                    let tactics = Tactics::of(&state, &moves, cfg, level, free);
                    assert_eq!(tactics.sf5_ex_ready(), level.spends_meter() && meter >= STOCK_AT, "{fighter_id:?} {level:?} EX {meter}");
                    assert_eq!(tactics.sf5_super_ready(), level.spends_meter() && meter >= 3 * STOCK_AT, "{fighter_id:?} {level:?} CA {meter}");
                }
            }
            state.super_meter = 2 * STOCK_AT;
            let old = Tactics::of(&state, &moves, cfg, Level::Easy, free);
            assert!(old.enhanced.is_some(), "{fighter_id:?}: old Easy retains its existing two-stock EX choice");
            assert!(old.spending_safely(Level::Easy).enhanced.is_none());
            state.super_meter = 3 * STOCK_AT;
            for control in [EraControl::default(), EraControl { airborne: true, ..free }] {
                let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, control);
                assert!(!tactics.sf5_ex_ready() && !tactics.sf5_super_ready() && !tactics.sf5_skill_ready());
            }
            state.set_gauge_mode(GaugeMode::NoGauges);
            cfg.gauge_mode = GaugeMode::NoGauges;
            let disabled = Tactics::of(&state, &sf5_moves(&state, cfg), cfg, Level::VeryHard, free);
            assert!(!disabled.sf5_ex_ready() && !disabled.sf5_super_ready() && !disabled.sf5_skill_ready() && !disabled.sf5_trigger_ready());
        }
    }

    #[test]
    fn every_sf5_pair_uses_its_native_trigger_gate_and_followup_table() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter;
        let free = EraControl { free: true, ..EraControl::default() };
        let mut checked = 0;
        for fighter_id in TournamentFighter::ALL {
            for revision in [Revision::Sf5, Revision::Sf5Ae, Revision::Sf5Ce] {
                for loadout in crate::era::loadouts(revision) {
                    if !sf5::available(StylePick::for_revision(revision, loadout), Some(fighter_id)) { continue }
                    checked += 1;
                    for mode in [GaugeMode::EraDefault, GaugeMode::Unified] {
                        let (mut state, cfg) = sf5_fighter(fighter_id, revision, loadout, mode);
                        let cost = state.meter_cost(state.v_trigger_cost(mode));
                        let moves = sf5_moves(&state, cfg);
                        for points in [cost - 1, cost] {
                            state.super_meter = if mode == GaugeMode::Unified { points } else { 0 };
                            state.set_sf5_practice_points((points, 0, 0));
                            let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
                            assert_eq!(tactics.sf5_trigger_ready(), points == cost, "{fighter_id:?}/{revision:?}/{loadout:?} {mode:?}");
                        }
                        state.super_meter = 0;
                        state.set_sf5_practice_points((0, 1, 1500));
                        let active = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
                        let offensive = fighter_id != Ryu && sf5::trigger_follow_up(fighter_id, loadout, false).is_some();
                        assert_eq!(active.sf5_trigger_ready(), offensive, "{fighter_id:?}/{revision:?}/{loadout:?}: positive native timer is enough for one follow-up");
                        let easy = Tactics::of(&state, &moves, cfg, Level::Easy, free);
                        assert!(!easy.sf5_trigger_ready());
                        if fighter_id == Ryu {
                            assert!(easy.sf5.unwrap().counter.is_some_and(|entry| matches!(entry.produces, sf5::SKILL_ONE | sf5::SKILL_TWO)), "Easy may use a free Skill counter, not spend Isshin's timer");
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 59);
    }

    #[test]
    fn sf5_offensive_followups_include_forward_tools_and_use_felt_facing() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::{Birdie, Sagat};
        let free = EraControl { free: true, ..EraControl::default() };
        for fighter_id in [Birdie, Sagat] {
            let (mut state, cfg) = sf5_fighter(fighter_id, Revision::Sf5Ce, Loadout::Skill1Trigger2, GaugeMode::EraDefault);
            state.set_sf5_practice_points((0, 1, 2400));
            let tactics = Tactics::of(&state, &sf5_moves(&state, cfg), cfg, Level::VeryHard, free);
            for facing_left in [false, true] {
                let mut view = Percept::default();
                view.me.facing_left = facing_left;
                view.me.x = 200;
                view.foe.x = if facing_left { 250 } else { 150 };
                let mut neutral = false;
                let mut forward = false;
                for seed in 0..64 {
                    let plan = tactics.sf5_trigger(view, &mut Rng(seed)).unwrap();
                    let pads = &plan.pads[..usize::from(plan.len)];
                    let chord = button(2) | button(5);
                    neutral |= pads.contains(&chord);
                    forward |= pads.contains(&(chord | if facing_left { LEFT } else { RIGHT }));
                }
                assert!(neutral && forward, "{fighter_id:?}: both native trigger follow-ups remain choices in either facing");
            }
        }
    }

    #[test]
    fn sf5_zero_metadata_trigger_route_cannot_borrow_v_or_bypass_easy() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::Ken;
        let (mut state, cfg) = sf5_fighter(Ken, Revision::Sf5Ae, Loadout::Skill1Trigger2, GaugeMode::EraDefault);
        let book = crate::route_book::book(Ken, Revision::Sf5Ae, Loadout::Skill1Trigger2).unwrap();
        let mut route = book.routes.iter().find(|route| route.name() == "cr.MP, 236+LP, HP+HK").unwrap().clone();
        assert_eq!((route.super_spent, route.drive_spent), (0, 0));
        for step in &mut route.steps { step.name = "meterless normal".into() }
        let moves = sf5_moves(&state, cfg);
        let at = |state: &EraFighterState| Tactics::of(state, &moves, cfg, Level::VeryHard, EraControl::default());
        assert!(!at(&state).sf5_route_ready(&route, Level::VeryHard, 0));
        let cost = state.v_trigger_cost(GaugeMode::EraDefault);
        state.set_sf5_practice_points((cost - 1, 0, 0));
        assert!(!at(&state).sf5_route_ready(&route, Level::VeryHard, 0));
        state.set_sf5_practice_points((cost, 0, 0));
        assert!(at(&state).sf5_route_ready(&route, Level::VeryHard, 0), "a later step need not be executable in the current local controls");
        assert!(!at(&state).sf5_route_ready(&route, Level::Easy, 0));
        state.set_sf5_practice_points((0, 1, 3000));
        assert!(at(&state).sf5_route_ready(&route, Level::VeryHard, 0), "a native active follow-up can spend the remaining timer");
        route.steps.push(route.steps.last().unwrap().clone());
        assert!(!at(&state).sf5_route_ready(&route, Level::VeryHard, 0), "do not assume a second trigger use is available");
    }

    #[test]
    fn sf5_route_gate_prices_actual_chords_and_motions_and_keeps_plain_routes() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::Ken;
        let (mut state, mut cfg) = sf5_fighter(Ken, Revision::Sf5Ce, Loadout::Skill1Trigger2, GaugeMode::Unified);
        state.super_meter = 3 * STOCK_AT;
        let moves = sf5_moves(&state, cfg);
        let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, EraControl::default());
        let combined = sf4_recipe(&["236+PP", "HP+HK"], u16::from(STOCK_AT));
        assert!(!tactics.sf5_route_ready(&combined, Level::VeryHard, u16::from(3 * STOCK_AT) - 1));
        assert!(tactics.sf5_route_ready(&combined, Level::VeryHard, u16::from(3 * STOCK_AT)), "Unified must pay EX and then V activation from the same gauge");
        assert!(!tactics.sf5_route_ready(&sf4_recipe(&["236+PP"], 0), Level::VeryHard, u16::from(3 * STOCK_AT)), "real EX is not meterless");
        assert!(!tactics.sf5_route_ready(&sf4_recipe(&["236236+HK"], 0), Level::VeryHard, u16::from(3 * STOCK_AT)), "real CA is not meterless");
        assert!(tactics.sf5_route_ready(&sf4_recipe(&["236236+HK"], u16::from(3 * STOCK_AT)), Level::VeryHard, u16::from(3 * STOCK_AT)));
        let mut skill = sf4_recipe(&["MP"], 0);
        skill.steps[0].pads = vec![(0, 0b010_010), (0, 0b010_010), (0, 0)];
        assert!(tactics.sf5_route_ready(&skill, Level::Easy, u16::from(3 * STOCK_AT)), "V-Skill spends no meter");
        state.set_gauge_mode(GaugeMode::NoGauges);
        cfg.gauge_mode = GaugeMode::NoGauges;
        let disabled = Tactics::of(&state, &sf5_moves(&state, cfg), cfg, Level::VeryHard, EraControl::default());
        assert!(!disabled.sf5_route_ready(&skill, Level::VeryHard, 0));
        assert!(!disabled.sf5_route_ready(&combined, Level::VeryHard, u16::from(3 * STOCK_AT)));
        assert!(disabled.sf5_route_ready(&sf4_recipe(&["MP", "LK"], 0), Level::Easy, 0));
    }

    #[test]
    fn sf5_quiet_neutral_cannot_start_a_trigger_or_ryus_counter() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::{Birdie, Ken, Sagat};
        let free = EraControl { free: true, ..EraControl::default() };
        let (view, _) = seen_begin(254, 1);
        for fighter_id in [Ryu, Ken, Birdie, Sagat] {
            for loadout in [Loadout::Skill1Trigger1, Loadout::Skill1Trigger2] {
                let (mut state, cfg) = sf5_fighter(fighter_id, Revision::Sf5Ce, loadout, GaugeMode::EraDefault);
                let cost = state.v_trigger_cost(GaugeMode::EraDefault);
                for timer in [0, 1500] {
                    state.set_sf5_practice_points((cost, timer, timer));
                    let old = Tactics::of(&state, &sf5_moves(&state, cfg), cfg, Level::VeryHard, free);
                    if timer == 0 { assert!(old.install.is_some(), "the legacy neutral activation remains unchanged") }
                    let stronger = old.spending_safely(Level::VeryHard);
                    assert!(stronger.install.is_none());
                    if fighter_id == Ryu { assert!(stronger.skill.is_none()) }
                    for seed in 0..64 {
                        if let Some(plan) = stronger.neutral(&mut Rng(seed), Level::VeryHard, view) {
                            assert!(!plan.pads[..usize::from(plan.len)].iter().any(|pad| pad & (button(2) | button(5)) == button(2) | button(5)), "{fighter_id:?}: quiet neutral does not borrow a Trigger chord");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sf5_route_gate_keeps_native_stance_chords_free_and_rejects_defensive_chords() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::{Gen, Ken};
        let (state, cfg) = sf5_fighter(Gen, Revision::Sf5Ce, Loadout::Skill1Trigger1, GaugeMode::EraDefault);
        let moves = sf5_moves(&state, cfg);
        assert!(moves.iter().any(|entry| entry.name == "SWITCH TO CRANE" && entry.input.button == ButtonClass::TripleKick));
        let tactics = Tactics::of(&state, &moves, cfg, Level::Easy, EraControl::default());
        let mut route = sf4_recipe(&["MP"], 0);
        route.steps[0].name = "ordinary normal".into();
        route.steps[0].pads = vec![(0, 0b111_000), (0, 0b111_000), (0, 0)];
        assert!(tactics.sf5_route_ready(&route, Level::Easy, 0), "the actual free Crane input is not EX");
        route.steps[0].pads = vec![(0, 0b000_111), (0, 0)];
        assert!(!tactics.sf5_route_ready(&route, Level::Easy, 0), "the current Mantis cannot borrow an absent stance command");
        route.steps[0].pads = vec![(0, 0b010_100), (0, 0b010_100), (0, 0)];
        for mode in [GaugeMode::EraDefault, GaugeMode::NoGauges] {
            let (mut state, cfg) = sf5_fighter(Ken, Revision::Sf5Ce, Loadout::Skill1Trigger1, mode);
            state.set_sf5_practice_points((STOCK_AT, 0, 0));
            let moves = sf5_moves(&state, cfg);
            for level in [Level::Easy, Level::VeryHard] {
                let tactics = Tactics::of(&state, &moves, cfg, level, EraControl::default());
                assert!(!tactics.sf5_route_ready(&route, level, 0), "{mode:?} {level:?}: Shift is not an unpriced attack recipe");
            }
        }
    }

    #[test]
    fn sf5_reactive_counters_and_v_defence_require_a_new_delayed_threat() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::Ken;
        let free = EraControl { free: true, ..EraControl::default() };
        let (view, before) = seen_begin(254, 0x11);
        let (mut ryu, cfg) = sf5_fighter(Ryu, Revision::Sf5Ce, Loadout::Skill2Trigger2, GaugeMode::EraDefault);
        ryu.set_sf5_practice_points((0, 1, 1500));
        let tactics = Tactics::of(&ryu, &sf5_moves(&ryu, cfg), cfg, Level::VeryHard, free);
        assert!(!tactics.sf5_skill_ready() && !tactics.sf5_trigger_ready(), "Ryu's counter is not a blind offensive plan");
        assert_eq!(tactics.sf5.unwrap().counter.unwrap().produces, sf5::TRIGGER_ATTACK);
        let answers = stronger_answers(tactics, view, before);
        assert!(!answers.is_empty());
        assert!(answers.iter().all(|pads| pads.contains(&(button(2) | button(5)))));
        assert!(stronger_answers(tactics, before, before).is_empty());
        assert!(stronger_answers(tactics, view, view).is_empty(), "a continuing attack is not a new event");
        for (revision, shift) in [(Revision::Sf5Ae, false), (Revision::Sf5Ce, true)] {
            let (mut state, cfg) = sf5_fighter(Ken, revision, Loadout::Skill1Trigger1, GaugeMode::EraDefault);
            let moves = sf5_moves(&state, cfg);
            for points in [STOCK_AT - 1, STOCK_AT] {
                state.set_sf5_practice_points((points, 0, 0));
                let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
                assert_eq!(tactics.sf5.unwrap().shift.is_some(), shift && points == STOCK_AT);
                assert_eq!(tactics.sf5.unwrap().reversal.is_some(), points == STOCK_AT);
            }
            state.set_sf5_practice_points((STOCK_AT, 1, 1200));
            let active = Tactics::of(&state, &moves, cfg, Level::VeryHard, free).sf5.unwrap();
            assert!(active.shift.is_none() && active.reversal.is_none());
        }
    }

    #[test]
    fn sf5_easy_keeps_meter_when_separate_commands_complete_its_own_ca_motion() {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::Ken;
        let free = EraControl { free: true, ..EraControl::default() };
        for fighter_id in [Ryu, Ken] {
            let (mut state, cfg) = sf5_fighter(fighter_id, Revision::Sf5Ce, Loadout::Skill1Trigger1, GaugeMode::EraDefault);
            state.super_meter = 3 * STOCK_AT;
            let moves = sf5_moves(&state, cfg);
            let easy = Tactics::of(&state, &moves, cfg, Level::Easy, free);
            assert_eq!(easy.sf5.unwrap().keep_meter_directions, 0, "a fresh native recognizer is unprimed");
            let kick = easy.sf5.unwrap().critical_input.unwrap().button.kick();
            let attack = button(if kick { 3 } else { 0 });
            let other = button(if kick { 2 } else { 5 });
            let motion = entry("ordinary quarter circle", 0x19, MoveKind::Special, QCF, if kick { ButtonClass::Kick } else { ButtonClass::Punch });
            for facing_left in [false, true] {
                let forward = if facing_left { LEFT } else { RIGHT };
                let first = alpha_motion(&motion, !facing_left, 0, true);
                assert!(first.iter().any(|pad| pad & attack != 0));
                assert!(first.iter().all(|&pad| easy.sf5_keep_meter(pad, facing_left) == pad), "the first standalone quarter circle stays valid");

                // The native helper tests replay the tracker itself. Supply
                // its boundary result after an earlier quarter circle and
                // this command's Down/DownForward: Forward would finish CA.
                let mut primed = easy;
                primed.sf5.as_mut().unwrap().keep_meter_directions = 1 << Dir::Forward.bits();
                assert_eq!(primed.sf5_keep_meter(forward | attack, facing_left), forward, "a second independently planned motion cannot spend Easy's meter");
                assert_eq!(primed.sf5_keep_meter(forward | attack | other, facing_left), forward | other, "only the CA button family is withheld");
                assert_eq!(primed.sf5_keep_meter(DOWN | attack, facing_left), DOWN | attack, "directions outside the native risk mask stay unchanged");
                assert_eq!(primed.sf5_keep_meter(forward | other, facing_left), forward | other, "the other family's normal stays playable");
                let skill = forward | button(1) | button(4);
                assert_eq!(primed.sf5_keep_meter(skill, facing_left), skill, "the free native V-Skill chord has priority over CA");
                let mut held_skill = primed;
                held_skill.sf5.as_mut().unwrap().previous_buttons = 0b010_010;
                assert_eq!(held_skill.sf5_keep_meter(skill | attack, facing_left), forward | button(if kick { 1 } else { 4 }), "an old held MediumPair does not exempt a fresh CA-family button");
                held_skill.sf5.as_mut().unwrap().previous_buttons = 0b000_010;
                assert_eq!(held_skill.sf5_keep_meter(skill | attack, facing_left), skill | attack, "one fresh MK edge is enough for native Skill priority");
                for control in [
                    EraControl { airborne: true, ..free },
                    EraControl { blockstun: true, ..free },
                    EraControl::default(),
                    EraControl { waking: true, ..EraControl::default() },
                ] {
                    let locked = Tactics { control, ..primed };
                    assert_eq!(locked.sf5_keep_meter(forward | attack, facing_left), forward | attack, "an unspendable state keeps its jump/recovery buttons");
                }
                for control in [
                    EraControl { cancel: true, ..EraControl::default() },
                    EraControl { special: true, ..EraControl::default() },
                    EraControl { sf5_special_cancel: true, ..EraControl::default() },
                ] {
                    let cancelling = Tactics { control, ..primed };
                    assert_eq!(cancelling.sf5_keep_meter(forward | attack, facing_left), forward, "a grounded cancel can still spend CA and needs protection");
                }
            }
            let normal = Tactics::of(&state, &moves, cfg, Level::Normal, free);
            assert_eq!(normal.sf5.unwrap().keep_meter_directions, 0, "other levels keep their deliberate spending policy");
            for pad in [0, RIGHT | attack, LEFT | attack, DOWN | attack, attack | other] {
                assert_eq!(normal.sf5_keep_meter(pad, false), pad);
            }
        }
        let other = Tactics::of(&fighter(Revision::Usf4, Loadout::Ultra1, 192), &[], MatchConfig::definitive(), Level::Easy, free);
        for pad in 0..1024 {
            assert_eq!(other.sf5_keep_meter(pad, false), pad, "other eras do not use this guard");
        }
    }

    fn sc20_fighter(fighter: crate::roster::TournamentFighter, mode: GaugeMode, drive: u16, meter: u8) -> (EraFighterState, MatchConfig, Vec<MoveEntry>) {
        let mut state = EraFighterState::default();
        state.fighter = Some(fighter);
        state.style = StylePick::for_revision(crate::era::Revision::Sf6, Loadout::Standard);
        state.max_sub_hp = 960;
        state.set_what_if(!crate::era::families(fighter).contains(&EraFamily::Sf6));
        state.set_gauge_mode(mode);
        state.set_sf6_practice_drive(Some(drive));
        state.super_meter = meter;
        let cfg = MatchConfig { gauge_mode: mode, ..MatchConfig::definitive() };
        let moves = crate::moves::live_move_list(fighter, cfg.preset, true, &state);
        (state, cfg, moves)
    }

    #[test]
    fn sf6_cast_paid_commands_use_live_entries_and_keep_each_levels_drive() {
        let free = EraControl { free: true, ..EraControl::default() };
        for fighter in crate::roster::TournamentFighter::ALL.into_iter().filter(|fighter| *fighter != Ryu) {
            for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                for (cost, impact) in [(STOCK_AT, true), (2 * STOCK_AT, false)] {
                    let enough = u16::from(cost).saturating_add(level.drive_kept()).max(u16::from(cost) + 1);
                    for drive in [enough.saturating_sub(1).min(288), enough.min(288)] {
                        let (state, cfg, moves) = sc20_fighter(fighter, GaugeMode::EraDefault, drive, 144);
                        let tactics = Tactics::of(&state, &moves, cfg, level, free);
                        let ready = if impact { tactics.sf6_impact_ready() } else { tactics.sf6_ex_ready() };
                        assert_eq!(ready, level.spends_meter() && drive >= enough, "{fighter:?} {level:?}: cost{cost} at{drive}");
                        if ready {
                            let entry = if impact { tactics.sf6.unwrap().impact.unwrap() } else { tactics.sf6.unwrap().enhanced.unwrap() };
                            assert!(moves.contains(&entry));
                            assert_eq!(entry.recognizer, Recognizer::FamilyMove);
                            if !impact { assert!(entry.name.starts_with("OD ")); }
                        }
                    }
                }
            }
        }
        let (state, cfg, moves) = sc20_fighter(crate::roster::TournamentFighter::Ken, GaugeMode::EraDefault, 288, 144);
        assert!(moves.iter().any(|entry| entry.name == "QUICK DASH" && entry.input.button == ButtonClass::PairKick));
        let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, free).spending_safely(Level::VeryHard);
        assert!(tactics.enhanced.unwrap().name.starts_with("OD "), "a free pair-kick dash is not OD");
    }

    #[test]
    fn sf6_cast_uses_the_affordable_super_level_and_actual_own_facing() {
        let free = EraControl { free: true, ..EraControl::default() };
        for fighter in crate::roster::TournamentFighter::ALL.into_iter().filter(|fighter| *fighter != Ryu) {
            for meter in [47, 48, 95, 96, 143, 144] {
                let (state, cfg, moves) = sc20_fighter(fighter, GaugeMode::EraDefault, 288, meter);
                let tactics = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
                assert_eq!(tactics.sf6.unwrap().super_move.map(|(entry, _)| entry.produces),
                    (meter >= STOCK_AT).then(|| 0x28 + meter / STOCK_AT), "{fighter:?} at{meter}");
                for facing_left in [false, true] {
                    let view = Percept { me: super::super::Body { x: 200, facing_left, ..Default::default() },
                        foe: super::super::Body { x: if facing_left { 300 } else { 100 }, ..Default::default() }, ..Default::default() };
                    let Some(plan) = tactics.sf6_super(view) else { assert!(meter < STOCK_AT); continue };
                    let (entry, strength) = tactics.sf6.unwrap().super_move.unwrap();
                    let expected = alpha_motion(&entry, !facing_left, strength, true);
                    assert_eq!(&plan.pads[..usize::from(plan.len)], &expected[..plan.len as usize]);
                }
            }
        }
    }

    #[test]
    fn sf6_cast_obeys_unified_disabled_burnout_and_control_gates_without_changing_ryu() {
        use crate::roster::TournamentFighter::Ken;
        let free = EraControl { free: true, ..EraControl::default() };
        for mode in [GaugeMode::Unified, GaugeMode::NoGauges] {
            for meter in [48, 49, 119, 120] {
                let (state, cfg, moves) = sc20_fighter(Ken, mode, 288, meter);
                let tactics = Tactics::of(&state, &moves, cfg, Level::Normal, free);
                assert_eq!(tactics.sf6_impact_ready(), mode == GaugeMode::Unified && meter >= 120);
                assert_eq!(tactics.sf6_ex_ready(), mode == GaugeMode::Unified && meter >= 120);
                assert_eq!(tactics.sf6_super_ready(), mode == GaugeMode::Unified);
            }
        }
        let (mut state, cfg, moves) = sc20_fighter(Ken, GaugeMode::EraDefault, 288, 144);
        for control in [EraControl::default(), EraControl { airborne: true, ..free }] {
            let locked = Tactics::of(&state, &moves, cfg, Level::VeryHard, control);
            assert!(!locked.sf6_impact_ready() && !locked.sf6_ex_ready() && !locked.sf6_super_ready());
        }
        let cancel = Tactics::of(&state, &moves, cfg, Level::VeryHard, EraControl { cancel: true, ..Default::default() });
        assert!(cancel.sf6_super_ready() && !cancel.sf6_impact_ready() && !cancel.sf6_ex_ready());
        state.restore_sf6_practice_drive(120, true);
        let burnout = Tactics::of(&state, &moves, cfg, Level::VeryHard, free);
        assert!(!burnout.sf6_impact_ready() && !burnout.sf6_ex_ready());
        assert!(burnout.sf6_super_ready(), "burnout does not remove Super meter");
        let (ryu, cfg, moves) = sc20_fighter(Ryu, GaugeMode::EraDefault, 288, 144);
        let tactics = Tactics::of(&ryu, &moves, cfg, Level::VeryHard, free);
        assert!(tactics.sf6.is_none() && !tactics.sf6_super_ready(), "Ryu keeps his existing choices");
    }

    #[test]
    fn easy_sf6_cast_keeps_ordinary_inputs_from_becoming_cancel_rushes() {
        use crate::roster::TournamentFighter::Lee;
        let (state, cfg, moves) = sc20_fighter(Lee, GaugeMode::EraDefault, 288, 144);
        let cancel = EraControl { cancel: true, ..Default::default() };
        let mut tactics = Tactics::of(&state, &moves, cfg, Level::Easy, cancel);
        tactics.sf6.as_mut().unwrap().dash_pending = true;
        for facing_left in [false, true] {
            let forward = if facing_left { LEFT } else { RIGHT };
            assert_eq!(tactics.sf6_keep_meter(forward, facing_left), 0);
            assert_eq!(tactics.sf6_keep_meter(forward | button(0), facing_left), button(0));
            assert_eq!(tactics.sf6_keep_meter(DOWN | forward | button(0), facing_left), DOWN | forward | button(0));
            assert_eq!(tactics.sf6_keep_meter(button(1) | button(4), facing_left), button(1));
            let free = Tactics { control: EraControl { free: true, ..Default::default() }, ..tactics };
            assert_eq!(free.sf6_keep_meter(forward, facing_left), forward);
            let mut normal = Tactics::of(&state, &moves, cfg, Level::Normal, cancel);
            normal.sf6.as_mut().unwrap().dash_pending = true;
            assert_eq!(normal.sf6_keep_meter(forward | button(1) | button(4), facing_left), forward | button(1) | button(4));
        }
    }
}
