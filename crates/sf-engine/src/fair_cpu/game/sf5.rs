//! SF5 game plans for all twelve tournament fighters (SC19, `#211`).
//!
//! The cast shares the situation judge, focus, habits, pressure and route
//! player. Its V-Skill belongs in close pressure or a safe space to charge,
//! as the fighter needs. Ryu's parry and counters answer delayed threats in
//! `Tactics::react` instead. V-Trigger activation, its offensive follow-ups
//! and Critical Art use a seen hit or whiff; readiness checks the live kit
//! and actual gauges. V-Reversal and V-Shift stay reactive choices.

use super::super::cast::Tool;
use super::super::habit::Lean;
use super::super::{Choice, Level, Role};
use super::{always, meaty, plan, poke, step, Act, EraAct, GamePlan, Situation, Step, ANY, CHARGE, COMMAND_THROW, JUMP_IN_GAP, SHOT_GAP, WATCHED_SHOT_GAPS};
use super::{RYU_SF6_BAIT, RYU_SF6_FRAME_TRAP, RYU_SF6_JUMP_IN, RYU_SF6_ROUTE, RYU_SF6_SHOT, RYU_SF6_SHOT_WATCH, RYU_SF6_WALK_THROW};
use crate::roster::TournamentFighter;

const SKILL: [Step; 1] = [step(Act::Era(EraAct::VSkill), always(None))];
const TRIGGER: [Step; 1] = [step(Act::Era(EraAct::VTrigger), always(None))];
const EX: [Step; 1] = [step(Act::Era(EraAct::Enhanced), always(None))];
const CRITICAL_ART: [Step; 1] = [step(Act::Era(EraAct::Super), always(None))];
const STANCE: [Step; 1] = [step(Act::Era(EraAct::Stance), always(None))];
const TELEPORT: [Step; 1] = [step(Act::Tool(Tool::Teleport), always(None))];
const ADVANCE: [Step; 1] = [step(Act::Do(Choice::Special(Role::Advance)), always(None))];

const POKE_HK: [Step; 6] = poke(false, 5);
const MEATY_HK: [Step; 4] = meaty(false, 5);
const POKE_MP: [Step; 6] = poke(false, 1);
const MEATY_MP: [Step; 4] = meaty(false, 1);
const POKE_LK: [Step; 6] = poke(false, 3);
const MEATY_LK: [Step; 4] = meaty(false, 3);

const TRIGGER_HIT: GamePlan = plan(Situation::ItHit, 84, 0, (i16::MIN, 90), &TRIGGER, Lean::None, Level::Normal);
const TRIGGER_PUNISH: GamePlan = plan(Situation::OpponentWhiffed, 84, 0, (i16::MIN, 90), &TRIGGER, Lean::None, Level::Normal);
const ART_HIT: GamePlan = plan(Situation::ItHit, 80, 0, (i16::MIN, 90), &CRITICAL_ART, Lean::None, Level::Normal);
const ART_PUNISH: GamePlan = plan(Situation::OpponentWhiffed, 80, 0, (i16::MIN, 90), &CRITICAL_ART, Lean::None, Level::Normal);
const SKILL_CLOSE: GamePlan = plan(Situation::ItWasBlocked, 64, 4, (i16::MIN, 56), &SKILL, Lean::None, Level::Normal);
const SKILL_FAR: GamePlan = plan(Situation::NeutralFar, 54, -6, (140, i16::MAX), &SKILL, Lean::None, Level::Normal);
// A shot must not always outrank Sagat's setup. The live skill gate stops
// offering this plan once his selected special has already been charged.
const SAGAT_CHARGE: GamePlan = GamePlan { score: 78, ..SKILL_FAR };

// Fourteen shared records leave room for two fighter tools without adding
// snapshot slots. The ordinary far approach remains available when every
// era command, route and special is absent or unaffordable.
macro_rules! shared {
    ($skill:expr, $far:expr; $poke:expr, $meaty:expr $(, $more:expr)* $(,)?) => {
        &[
            plan(Situation::NeutralClose, 60, 0, ANY, $poke, Lean::None, Level::Easy),
            plan(Situation::NeutralClose, 62, 6, ANY, &RYU_SF6_ROUTE, Lean::None, Level::Easy),
            plan(Situation::ItWasBlocked, 60, 4, ANY, &RYU_SF6_FRAME_TRAP, Lean::FrameTrap, Level::Normal),
            plan(Situation::ItWasBlocked, 56, 12, ANY, &RYU_SF6_WALK_THROW, Lean::WalkThrow, Level::Easy),
            $skill,
            plan(Situation::NeutralClose, 64, 0, (i16::MIN, 80), &EX, Lean::None, Level::Normal),
            plan(Situation::OpponentKnockedDown, 62, 8, ANY, $meaty, Lean::Meaty, Level::Normal),
            plan(Situation::OpponentKnockedDown, 52, -10, ANY, &RYU_SF6_BAIT, Lean::Bait, Level::VeryHard),
            plan(Situation::NeutralFar, 50, 10, ANY, $poke, Lean::None, Level::Easy),
            $far,
            TRIGGER_HIT,
            TRIGGER_PUNISH,
            ART_HIT,
            ART_PUNISH,
            $($more),*
        ]
    };
}

const SHOT: GamePlan = plan(Situation::NeutralFar, 70, 0, (SHOT_GAP, i16::MAX), &RYU_SF6_SHOT, Lean::Shot, Level::Easy);
const SHOT_WATCH: GamePlan = plan(Situation::NeutralFar, 0, 0, WATCHED_SHOT_GAPS, &RYU_SF6_SHOT_WATCH, Lean::ShotWatch, Level::Normal);
const JUMP_FAR: GamePlan = plan(Situation::NeutralFar, 46, 0, JUMP_IN_GAP, &RYU_SF6_JUMP_IN, Lean::JumpIn, Level::Easy);
const ADVANCE_FAR: GamePlan = plan(Situation::NeutralFar, 54, 0, (120, i16::MAX), &ADVANCE, Lean::None, Level::Easy);
const THROW_BLOCKED: GamePlan = plan(Situation::ItWasBlocked, 58, 8, ANY, &COMMAND_THROW, Lean::WalkThrow, Level::Normal);
const THROW_CLOSE: GamePlan = plan(Situation::NeutralClose, 62, 4, ANY, &COMMAND_THROW, Lean::None, Level::Hard);
const TELEPORT_FAR: GamePlan = plan(Situation::NeutralFar, 48, -6, (130, i16::MAX), &TELEPORT, Lean::None, Level::Normal);
const LEE_CHARGE: GamePlan = plan(Situation::NeutralFar, 50, 0, (150, i16::MAX), &CHARGE, Lean::None, Level::Easy);
const GEN_CHARGE: GamePlan = plan(Situation::NeutralFar, 48, 0, (60, 170), &CHARGE, Lean::None, Level::Easy);
const CRANE: GamePlan = plan(Situation::NeutralFar, 50, -10, ANY, &STANCE, Lean::None, Level::Easy);

// Ryu's V-Skills are defensive. His extra plan watches the projectile's
// answer instead of blindly pressing a parry/counter into a quiet scene.
const RYU: &[GamePlan] = shared!(SHOT_WATCH, SHOT; &POKE_HK, &MEATY_HK);
const KEN: &[GamePlan] = shared!(SKILL_CLOSE, SHOT; &POKE_HK, &MEATY_HK, SHOT_WATCH);
const SAGAT: &[GamePlan] = shared!(SAGAT_CHARGE, SHOT; &POKE_MP, &MEATY_MP, SHOT_WATCH);
const RETSU: &[GamePlan] = shared!(SKILL_CLOSE, SHOT; &POKE_MP, &MEATY_MP, THROW_CLOSE, THROW_BLOCKED);
const GEKI: &[GamePlan] = shared!(SKILL_CLOSE, JUMP_FAR; &POKE_MP, &MEATY_MP, THROW_BLOCKED, TELEPORT_FAR);
// Joe's single low kick avoids committing his whole MK straight sequence.
const JOE: &[GamePlan] = shared!(SKILL_CLOSE, SHOT; &POKE_LK, &MEATY_LK, SHOT_WATCH);
const MIKE: &[GamePlan] = shared!(SKILL_CLOSE, ADVANCE_FAR; &POKE_MP, &MEATY_MP);
const LEE: &[GamePlan] = shared!(SKILL_CLOSE, ADVANCE_FAR; &POKE_MP, &MEATY_MP, LEE_CHARGE);
const BIRDIE: &[GamePlan] = shared!(SKILL_FAR, ADVANCE_FAR; &POKE_MP, &MEATY_MP);
const GEN: &[GamePlan] = shared!(SKILL_CLOSE, GamePlan { score: 54, ..JUMP_FAR }; &POKE_HK, &MEATY_HK, CRANE, GEN_CHARGE);
const EAGLE: &[GamePlan] = shared!(SKILL_CLOSE, ADVANCE_FAR; &POKE_LK, &MEATY_LK);
const ADON: &[GamePlan] = shared!(SKILL_CLOSE, JUMP_FAR; &POKE_MP, &MEATY_MP);

pub(super) fn plans(fighter: TournamentFighter) -> &'static [GamePlan] {
    use TournamentFighter::*;
    match fighter {
        Ryu => RYU,
        Ken => KEN,
        Sagat => SAGAT,
        Retsu => RETSU,
        Geki => GEKI,
        Joe => JOE,
        Mike => MIKE,
        Lee => LEE,
        Birdie => BIRDIE,
        Gen => GEN,
        Eagle => EAGLE,
        Adon => ADON,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sf5_fighter_has_routes_situations_and_only_his_own_tools() {
        use TournamentFighter::*;
        for fighter in TournamentFighter::ALL {
            let plans = plans(fighter);
            assert!(plans.len() <= super::super::PLAN_SLOTS, "{fighter:?}: records fit");
            for situation in [Situation::NeutralClose, Situation::NeutralFar, Situation::ItWasBlocked, Situation::OpponentKnockedDown, Situation::ItHit, Situation::OpponentWhiffed] {
                assert!(plans.iter().any(|p| p.situation == situation), "{fighter:?}: {situation:?}");
            }
            assert!(plans.iter().any(|p| {
                p.situation == Situation::NeutralFar && p.gap == ANY && !p.cornered_only
                    && p.for_level(Level::Easy) && !p.needs_specials()
                    && matches!(p.steps.first().map(|s| s.act), Some(Act::Reach { .. }))
            }), "{fighter:?}: ordinary approach survives missing meter and tools");
            let uses = |act| plans.iter().any(|p| p.steps.iter().any(|s| s.act == act));
            assert!(uses(Act::Route(crate::route_book::Reach::Ground)), "{fighter:?}: own route book");
            assert!(uses(Act::Era(EraAct::VTrigger)) && uses(Act::Era(EraAct::Enhanced)) && uses(Act::Era(EraAct::Super)), "{fighter:?}: SF5 tools");
            assert_eq!(uses(Act::Era(EraAct::VSkill)), fighter != Ryu, "{fighter:?}: Ryu answers threats with his skill");
            assert_eq!(uses(Act::Tool(Tool::CommandThrow)), matches!(fighter, Retsu | Geki), "{fighter:?}: ordinary command throw");
            assert_eq!(uses(Act::Tool(Tool::Teleport)), fighter == Geki, "{fighter:?}: teleport");
            assert_eq!(uses(Act::Tool(Tool::Charge)), matches!(fighter, Lee | Gen), "{fighter:?}: charge");
            assert_eq!(uses(Act::Era(EraAct::Stance)), fighter == Gen, "{fighter:?}: stance");
            assert!(!uses(Act::Era(EraAct::Focus)) && !uses(Act::Era(EraAct::Ultra)) && !uses(Act::Era(EraAct::CustomCombo)), "{fighter:?}: other eras' commands stay out");
            for p in plans {
                for s in p.steps {
                    assert!(s.then.iter().flatten().all(|&next| usize::from(next) < p.steps.len()), "{fighter:?}: valid branch");
                }
            }
        }
    }

    #[test]
    fn sf5_spends_trigger_and_critical_art_only_on_seen_openings() {
        for fighter in TournamentFighter::ALL {
            for p in plans(fighter) {
                if p.steps.iter().any(|s| matches!(s.act, Act::Era(EraAct::VTrigger | EraAct::Super))) {
                    assert!(matches!(p.situation, Situation::ItHit | Situation::OpponentWhiffed), "{fighter:?}: seen opening");
                    assert!(p.gap.1 <= 90 && !p.for_level(Level::Easy), "{fighter:?}: deliberate meter spending");
                }
                if p.steps.iter().any(|s| s.act == Act::Era(EraAct::VSkill)) {
                    assert_eq!(p.situation, if matches!(fighter, TournamentFighter::Birdie | TournamentFighter::Sagat) { Situation::NeutralFar } else { Situation::ItWasBlocked });
                }
            }
        }
    }
}
