//! Tactical use of the live Style Card kit through the same pad as a player.
use super::{alpha_motion, button, Level, Percept, Plan, PlanKind, Rng, DOWN, LEFT, RIGHT};
use crate::config::MatchConfig;
use crate::era::{EraFamily, GaugeMode, Loadout, STOCK_AT};
use crate::era_fight::{sf3, sf4, sf5, sf6, EraFighterState};
use crate::moves::{ButtonClass, Dir, MoveEntry, MoveKind, Recognizer};

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
    guard_cancel: Option<MoveEntry>,
    evade: Option<MoveEntry>,
    armor: Option<MoveEntry>,
    rush: Option<MoveEntry>,
    install: Option<MoveEntry>,
    skill: Option<MoveEntry>,
    mix_up: Option<MoveEntry>,
    stance: Option<MoveEntry>,
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
        let enhanced = (enabled && enhanced)
            .then(|| {
                moves.iter().copied().find(|m| {
                    m.recognizer == Recognizer::FamilyMove
                        && m.kind == MoveKind::Special
                        && matches!(
                            m.input.button,
                            ButtonClass::PairPunch | ButtonClass::PairKick
                        )
                        && m.input.motion.first() != Some(&Dir::Jump)
                })
            })
            .flatten();
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
            guard_cancel,
            evade,
            armor,
            rush: (family == Sf6 && !cfg.disable_raw_drive_rush && can_drive(STOCK_AT + 2))
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
        }
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
    pub(super) fn react(
        self,
        rng: &mut Rng,
        level: Level,
        view: Percept,
        before: Percept,
    ) -> Option<Plan> {
        let toward = if view.toward() > 0 { RIGHT } else { LEFT };
        let away = if toward == RIGHT { LEFT } else { RIGHT };
        let threatened = view.foe.attacking() && view.gap() < 80
            || (view.shot && (view.shot_x - view.me.x).abs() < 80);
        if self.can_air_block && self.control.airborne && threatened && rng.chance(level.guard()) {
            return Some(Plan::of(PlanKind::Act, &[away; 12]));
        }
        if self.control.blockstun && (view.me.guarding() || threatened) {
            if let Some(entry) = self.guard_cancel.filter(|_| rng.chance(level.guard() / 2)) {
                return Some(self.pad(&entry, view, 2));
            }
            return None;
        }
        let hit_confirm = view.foe.health < before.foe.health && view.gap() < 90;
        let wakeup = (self.control.waking || (before.me.reacting() && !view.me.reacting()))
            && view.gap() < 68;
        if (hit_confirm || wakeup)
            && (self.control.free || self.control.cancel)
            && rng.chance(level.quality())
        {
            if let Some((entry, strength)) = self.super_move {
                return Some(self.pad(&entry, view, strength));
            }
        }
        if self.fadc
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
        if !(self.control.free || (self.can_parry && self.control.airborne))
            || !threatened
            || !rng.chance(level.guard() / 2)
        {
            return None;
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
            .react(&mut Rng(0), Level::Hard, before, before)
            .is_none());
        let plan = hard
            .react(&mut Rng(0), Level::Hard, confirmed, before)
            .expect("hit confirm plans CA");
        assert_eq!(plan.pads[..3], [DOWN; 3]);
        assert!(plan.pads[..usize::from(plan.len)].contains(&(RIGHT | button(2))));
        let easy = Tactics::of(&state, &moves, config, Level::Easy, control);
        assert!(easy
            .react(&mut Rng(0), Level::Easy, confirmed, before)
            .is_none());
    }
}
