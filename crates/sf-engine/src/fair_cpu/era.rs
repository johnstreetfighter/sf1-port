//! Tactical use of the live Style Card kit through the same pad as a player.
use super::{alpha_motion, button, Level, Percept, Plan, PlanKind, Rng, DOWN, LEFT, RIGHT};
use crate::config::MatchConfig;
use crate::era::{EraFamily, GaugeMode, Loadout, STOCK_AT};
use crate::era_fight::{sf3, sf4, sf5, sf6, EraFighterState};
use crate::moves::{ButtonClass, Dir, MoveEntry, MoveKind, Recognizer};

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
fn sf4_route_directions(step: &crate::route_book::Step, at: usize) -> Option<([u8; 12], usize)> {
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
    pub waking: bool,
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
        let sf4_enhanced = (enabled && family == Sf4 && ex >= STOCK_AT)
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
            self.enhanced = self.sf4_enhanced;
        }
        // Nor does it switch Gen back from the Crane to the Mantis: back
        // and forth at range only stood him still (SC16, `#208`).
        self.stance = self.stance.filter(|_| self.can_change_stance());
        if self.family == EraFamily::Sf3 {
            self.enhanced = self.sf3_enhanced;
        }
        if self.family == EraFamily::Sf6 {
            self.rush = self.rush.filter(|_| self.keeps_drive(RUSH_COST));
            self.armor = self.armor.filter(|_| self.keeps_drive(STOCK_AT));
            self.enhanced = self.enhanced.filter(|_| self.keeps_drive(2 * STOCK_AT));
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
                let Some((directions, len)) = sf4_route_directions(step, at) else { return false };
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
            if let Some(entry) = self.guard_cancel.filter(|_| affordable && rng.chance(level.guard() / 2)) {
                return Some(self.pad(&entry, view, 2));
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
        let mut config = MatchConfig::definitive();
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
        config.gauge_mode = GaugeMode::Unified;
        let v = fighter(Revision::Sf5Ce, Loadout::Skill1Trigger1, 2 * STOCK_AT);
        let reversal = entry("V-REVERSAL", sf5::REVERSAL, MoveKind::System, &[Dir::Forward], ButtonClass::PairPunch);
        let sf5 = Tactics::of(&v, &[reversal], config, Level::VeryHard, blockstun);
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
        let shift = entry("V-SHIFT", sf5::SHIFT, MoveKind::System, &[], ButtonClass::HeavyPair);
        let sf5 = Tactics::of(&fighter(Revision::Sf5Ce, Loadout::Skill1Trigger1, 2 * STOCK_AT), &[shift], unified, Level::VeryHard, free);
        let heavy_pair = button(2) | button(5);
        assert!(stronger_answers(sf5, view, before).iter().any(|pads| pads.iter().any(|&p| p & heavy_pair == heavy_pair)), "SF5 shifts");
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
}
