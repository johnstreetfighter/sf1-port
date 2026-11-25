//! SF4 game plans for all twelve tournament fighters (SC18, `#210`).
//!
//! The cast keeps the shared situation judge, focus, habit memory,
//! pressure and route player. Focus is a close pressure choice after a
//! guarded attack. Super and Ultra are separate choices on a seen hit or
//! whiff, never blind neutral attacks; their live move and resource gates
//! are checked before choosing a plan. FADC answers a delayed hit or
//! guard during the fighter's own special in `Tactics::react`.

use super::super::cast::Tool;
use super::super::habit::Lean;
use super::super::{Choice, Level, Role};
use super::{always, meaty, plan, poke, step, Act, EraAct, GamePlan, Situation, Step, ANY, CHARGE, COMMAND_THROW, JUMP_IN_GAP, SHOT_GAP, WATCHED_SHOT_GAPS};
use super::{RYU_SF6_BAIT, RYU_SF6_FRAME_TRAP, RYU_SF6_JUMP_IN, RYU_SF6_ROUTE, RYU_SF6_SHOT, RYU_SF6_SHOT_WATCH, RYU_SF6_WALK_THROW};
use crate::roster::TournamentFighter;

const FOCUS: [Step; 1] = [step(Act::Era(EraAct::Focus), always(None))];
const EX: [Step; 1] = [step(Act::Era(EraAct::Enhanced), always(None))];
const SUPER: [Step; 1] = [step(Act::Era(EraAct::Super), always(None))];
const ULTRA: [Step; 1] = [step(Act::Era(EraAct::Ultra), always(None))];
const STANCE: [Step; 1] = [step(Act::Era(EraAct::Stance), always(None))];
const TELEPORT: [Step; 1] = [step(Act::Tool(Tool::Teleport), always(None))];
const ADVANCE: [Step; 1] = [step(Act::Do(Choice::Special(Role::Advance)), always(None))];

const POKE_HK: [Step; 6] = poke(false, 5);
const MEATY_HK: [Step; 4] = meaty(false, 5);
const POKE_MP: [Step; 6] = poke(false, 1);
const MEATY_MP: [Step; 4] = meaty(false, 1);
const POKE_LK: [Step; 6] = poke(false, 3);
const MEATY_LK: [Step; 4] = meaty(false, 3);

// A full Revenge gauge can be spent without consuming the native Super
// meter, so an affordable Ultra wins over a Super in the same opening.
const SUPER_HIT: GamePlan = plan(Situation::ItHit, 80, 0, (i16::MIN, 90), &SUPER, Lean::None, Level::Normal);
const ULTRA_HIT: GamePlan = plan(Situation::ItHit, 88, 0, (i16::MIN, 90), &ULTRA, Lean::None, Level::Normal);
const SUPER_PUNISH: GamePlan = plan(Situation::OpponentWhiffed, 80, 0, (i16::MIN, 90), &SUPER, Lean::None, Level::Normal);
const ULTRA_PUNISH: GamePlan = plan(Situation::OpponentWhiffed, 88, 0, (i16::MIN, 90), &ULTRA, Lean::None, Level::Normal);

// Defence and our own wake-up use the shared reaction paths. The four
// continuing situations below have ordinary pressure/spacing choices as
// well as the SF4 tools, so a missing or unaffordable era tool cannot
// leave a fighter with nothing to do.
macro_rules! shared {
    ($poke:expr, $meaty:expr $(, $more:expr)* $(,)?) => {
        shared!(62, JUMP_FAR; $poke, $meaty $(, $more)*)
    };
    ($route_score:expr, $far:expr; $poke:expr, $meaty:expr $(, $more:expr)* $(,)?) => {
        &[
            plan(Situation::NeutralClose, 60, 0, ANY, $poke, Lean::None, Level::Easy),
            plan(Situation::NeutralClose, $route_score, 6, ANY, &RYU_SF6_ROUTE, Lean::None, Level::Easy),
            plan(Situation::ItWasBlocked, 60, 4, ANY, &RYU_SF6_FRAME_TRAP, Lean::FrameTrap, Level::Normal),
            plan(Situation::ItWasBlocked, 56, 12, ANY, &RYU_SF6_WALK_THROW, Lean::WalkThrow, Level::Easy),
            plan(Situation::ItWasBlocked, 64, 4, (i16::MIN, 48), &FOCUS, Lean::None, Level::Normal),
            plan(Situation::NeutralClose, 64, 0, (i16::MIN, 80), &EX, Lean::None, Level::Normal),
            plan(Situation::OpponentKnockedDown, 62, 8, ANY, $meaty, Lean::Meaty, Level::Normal),
            plan(Situation::OpponentKnockedDown, 52, -10, ANY, &RYU_SF6_BAIT, Lean::Bait, Level::VeryHard),
            plan(Situation::NeutralFar, 50, 10, ANY, $poke, Lean::None, Level::Easy),
            $far,
            SUPER_HIT,
            ULTRA_HIT,
            SUPER_PUNISH,
            ULTRA_PUNISH,
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
const CHARGE_FAR: GamePlan = plan(Situation::NeutralFar, 50, 0, (60, 170), &CHARGE, Lean::None, Level::Easy);
const LEE_CHARGE: GamePlan = plan(Situation::NeutralFar, 50, 0, (150, i16::MAX), &CHARGE, Lean::None, Level::Easy);
const CRANE: GamePlan = plan(Situation::NeutralFar, 50, -10, ANY, &STANCE, Lean::None, Level::Easy);
// Let Gen's slow backward charge compete with walking in rather than
// taking every uncornered far-neutral decision after changing stance.
const GEN_CHARGE: GamePlan = GamePlan { score: 48, ..CHARGE_FAR };

const RYU: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK, SHOT, SHOT_WATCH);
const KEN: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK, SHOT, SHOT_WATCH);
const SAGAT: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, SHOT, SHOT_WATCH);
// Their live projectiles offer another way in while retaining both
// fighter tools within the sixteen remembered plan slots.
const RETSU: &[GamePlan] = shared!(62, SHOT; &POKE_MP, &MEATY_MP, THROW_CLOSE, THROW_BLOCKED);
const GEKI: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, THROW_BLOCKED, TELEPORT_FAR);
// Joe's MK continues into a straight. His single low kick keeps the
// pressure shorter, and Shockwave gives him a purpose at long range.
const JOE: &[GamePlan] = shared!(&POKE_LK, &MEATY_LK, SHOT, SHOT_WATCH);
const MIKE: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, ADVANCE_FAR);
const LEE: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, LEE_CHARGE, ADVANCE_FAR);
const BIRDIE: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, THROW_BLOCKED, CHARGE_FAR);
const GEN: &[GamePlan] = shared!(62, GamePlan { score: 54, ..JUMP_FAR }; &POKE_HK, &MEATY_HK, CRANE, GEN_CHARGE);
const EAGLE: &[GamePlan] = shared!(&POKE_LK, &MEATY_LK, ADVANCE_FAR);
const ADON: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP);

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
    fn every_sf4_fighter_has_routes_situations_and_only_his_own_tools() {
        use TournamentFighter::*;
        for fighter in TournamentFighter::ALL {
            let plans = plans(fighter);
            assert!(plans.len() <= super::super::PLAN_SLOTS, "{fighter:?}: records fit");
            for situation in [Situation::NeutralClose, Situation::NeutralFar, Situation::ItWasBlocked, Situation::OpponentKnockedDown] {
                assert!(plans.iter().any(|p| p.situation == situation), "{fighter:?}: {situation:?}");
            }
            assert!(plans.iter().any(|p| {
                p.situation == Situation::NeutralFar && p.gap == ANY && !p.cornered_only
                    && p.for_level(Level::Easy) && !p.needs_specials()
                    && matches!(p.steps.first().map(|s| s.act), Some(Act::Reach { .. }))
            }), "{fighter:?}: an approach remains playable without a route or meter");
            let uses = |act| plans.iter().any(|p| p.steps.iter().any(|s| s.act == act));
            assert!(uses(Act::Route(crate::route_book::Reach::Ground)), "{fighter:?}: own route book");
            assert!(uses(Act::Era(EraAct::Focus)) && uses(Act::Era(EraAct::Enhanced)), "{fighter:?}: SF4 tools");
            assert_eq!(uses(Act::Tool(Tool::CommandThrow)), matches!(fighter, Retsu | Geki | Birdie), "{fighter:?}: command throw");
            assert_eq!(uses(Act::Tool(Tool::Teleport)), fighter == Geki, "{fighter:?}: teleport");
            assert_eq!(uses(Act::Tool(Tool::Charge)), matches!(fighter, Birdie | Lee | Gen), "{fighter:?}: charge");
            assert_eq!(uses(Act::Era(EraAct::Stance)), fighter == Gen, "{fighter:?}: stance");
            assert!(!uses(Act::Era(EraAct::CustomCombo)) && !uses(Act::Era(EraAct::Overhead)), "{fighter:?}: other eras' commands stay out");
            for p in plans {
                for s in p.steps {
                    assert!(s.then.iter().flatten().all(|&next| usize::from(next) < p.steps.len()), "{fighter:?}: valid branch");
                }
            }
        }
    }

    #[test]
    fn sf4_finishers_require_an_opening_and_focus_stays_in_close_pressure() {
        for fighter in TournamentFighter::ALL {
            let plans = plans(fighter);
            for situation in [Situation::ItHit, Situation::OpponentWhiffed] {
                for action in [EraAct::Super, EraAct::Ultra] {
                    assert!(plans.iter().any(|p| p.situation == situation && p.steps.iter().any(|s| s.act == Act::Era(action))), "{fighter:?}: {action:?} on {situation:?}");
                }
            }
            for p in plans {
                if p.steps.iter().any(|s| matches!(s.act, Act::Era(EraAct::Super | EraAct::Ultra))) {
                    assert!(matches!(p.situation, Situation::ItHit | Situation::OpponentWhiffed));
                    assert!(p.gap.1 <= 90 && !p.for_level(Level::Easy), "{fighter:?}: bounded deliberate finisher");
                }
                if p.steps.iter().any(|s| s.act == Act::Era(EraAct::Focus)) {
                    assert_eq!(p.situation, Situation::ItWasBlocked);
                    assert!(p.gap.1 <= 48 && !p.for_level(Level::Easy), "{fighter:?}: close pressure Focus");
                }
            }
        }
    }
}
