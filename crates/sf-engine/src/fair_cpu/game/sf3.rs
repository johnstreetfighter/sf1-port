//! SF3 game plans for all twelve tournament fighters (SC17, `#209`).
//!
//! The shared judge, focus, habit memory, pressure and route player stay
//! the ones proven on Ryu SF6. Here the cast chooses its own poke, routes,
//! knockdown play and tools. Parries answer the delayed event under focus
//! in `Tactics::react`; Super Arts are spent inside the fighter's route
//! book or on a visible confirm. EX specials and the universal overhead
//! have explicit plan steps, gated by the live revision and meter.

use super::super::cast::Tool;
use super::super::habit::Lean;
use super::super::Level;
use super::{always, meaty, plan, poke, step, Act, EraAct, GamePlan, Situation, Step, ANY, CHARGE, COMMAND_THROW, JUMP_IN_GAP, SHOT_GAP, WATCHED_SHOT_GAPS};
use super::{RYU_SF6_BAIT, RYU_SF6_FRAME_TRAP, RYU_SF6_JUMP_IN, RYU_SF6_ROUTE, RYU_SF6_SHOT, RYU_SF6_SHOT_WATCH, RYU_SF6_WALK_THROW};
use crate::roster::TournamentFighter;

const OVERHEAD: [Step; 1] = [step(Act::Era(EraAct::Overhead), always(None))];
const EX: [Step; 1] = [step(Act::Era(EraAct::Enhanced), always(None))];
const STANCE: [Step; 1] = [step(Act::Era(EraAct::Stance), always(None))];
const TELEPORT: [Step; 1] = [step(Act::Tool(Tool::Teleport), always(None))];

const POKE_HK: [Step; 6] = poke(false, 5);
const MEATY_HK: [Step; 4] = meaty(false, 5);
const POKE_MK: [Step; 6] = poke(false, 4);
const MEATY_MK: [Step; 4] = meaty(false, 4);
const POKE_MP: [Step; 6] = poke(false, 1);
const MEATY_MP: [Step; 4] = meaty(false, 1);
const POKE_LK: [Step; 6] = poke(false, 3);
const MEATY_LK: [Step; 4] = meaty(false, 3);

// These situations call for a continuing game plan. ItHit, whiffed
// attacks, defence and our own wake-up use the shared route and reaction
// paths, which can answer as soon as the applicable reaction delay ends.
macro_rules! shared {
    ($poke:expr, $meaty:expr $(, $more:expr)* $(,)?) => {
        &[
            plan(Situation::NeutralClose, 60, 0, ANY, $poke, Lean::None, Level::Easy),
            plan(Situation::NeutralClose, 62, 6, ANY, &RYU_SF6_ROUTE, Lean::None, Level::Easy),
            plan(Situation::ItWasBlocked, 60, 4, ANY, &RYU_SF6_FRAME_TRAP, Lean::FrameTrap, Level::Normal),
            plan(Situation::ItWasBlocked, 56, 12, ANY, &RYU_SF6_WALK_THROW, Lean::WalkThrow, Level::Easy),
            plan(Situation::ItWasBlocked, 64, 4, (i16::MIN, 56), &OVERHEAD, Lean::None, Level::Normal),
            plan(Situation::NeutralClose, 64, 0, (i16::MIN, 80), &EX, Lean::None, Level::Normal),
            plan(Situation::OpponentKnockedDown, 62, 8, ANY, $meaty, Lean::Meaty, Level::Normal),
            plan(Situation::OpponentKnockedDown, 52, -10, ANY, &RYU_SF6_BAIT, Lean::Bait, Level::VeryHard),
            plan(Situation::NeutralFar, 40, 0, JUMP_IN_GAP, &RYU_SF6_JUMP_IN, Lean::JumpIn, Level::Easy),
            $($more),*
        ]
    };
}

const fn walk_down(poke: &'static [Step]) -> GamePlan {
    GamePlan { cornered_only: true, ..plan(Situation::NeutralFar, 60, 0, ANY, poke, Lean::None, Level::Normal) }
}

const SHOT: GamePlan = plan(Situation::NeutralFar, 70, 0, (SHOT_GAP, i16::MAX), &RYU_SF6_SHOT, Lean::Shot, Level::Easy);
const SHOT_WATCH: GamePlan = plan(Situation::NeutralFar, 0, 0, WATCHED_SHOT_GAPS, &RYU_SF6_SHOT_WATCH, Lean::ShotWatch, Level::Normal);
const THROW_BLOCKED: GamePlan = plan(Situation::ItWasBlocked, 58, 8, ANY, &COMMAND_THROW, Lean::WalkThrow, Level::Normal);
const THROW_CLOSE: GamePlan = plan(Situation::NeutralClose, 62, 4, ANY, &COMMAND_THROW, Lean::None, Level::Hard);
const TELEPORT_FAR: GamePlan = plan(Situation::NeutralFar, 56, -6, (130, i16::MAX), &TELEPORT, Lean::None, Level::Normal);
const CHARGE_FAR: GamePlan = plan(Situation::NeutralFar, 58, 0, (60, 170), &CHARGE, Lean::None, Level::Easy);
const LEE_CHARGE: GamePlan = plan(Situation::NeutralFar, 58, 0, (150, i16::MAX), &CHARGE, Lean::None, Level::Easy);
const CRANE: GamePlan = plan(Situation::NeutralFar, 50, -10, ANY, &STANCE, Lean::None, Level::Easy);

const RYU: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK, walk_down(&POKE_HK), SHOT, SHOT_WATCH);
const KEN: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK, walk_down(&POKE_HK), SHOT, SHOT_WATCH);
const SAGAT: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, walk_down(&POKE_MP), SHOT, SHOT_WATCH);
const RETSU: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, THROW_CLOSE, THROW_BLOCKED);
const GEKI: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, walk_down(&POKE_MP), THROW_CLOSE, THROW_BLOCKED, TELEPORT_FAR);
const JOE: &[GamePlan] = shared!(&POKE_MK, &MEATY_MK);
const MIKE: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, walk_down(&POKE_MP));
const LEE: &[GamePlan] = shared!(&POKE_MK, &MEATY_MK, LEE_CHARGE);
const BIRDIE: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, walk_down(&POKE_MP), THROW_CLOSE, THROW_BLOCKED, CHARGE_FAR);
const GEN: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK, walk_down(&POKE_HK), CRANE, CHARGE_FAR);
const EAGLE: &[GamePlan] = shared!(&POKE_LK, &MEATY_LK, walk_down(&POKE_LK));
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
    fn every_sf3_fighter_has_plans_routes_and_only_his_own_tools() {
        use TournamentFighter::*;
        for fighter in TournamentFighter::ALL {
            let plans = plans(fighter);
            assert!(plans.len() <= super::super::PLAN_SLOTS, "{fighter:?}: records fit");
            for situation in [Situation::NeutralClose, Situation::NeutralFar, Situation::ItWasBlocked, Situation::OpponentKnockedDown] {
                assert!(plans.iter().any(|p| p.situation == situation), "{fighter:?}: {situation:?}");
            }
            let uses = |act| plans.iter().any(|p| p.steps.iter().any(|s| s.act == act));
            assert!(uses(Act::Route(crate::route_book::Reach::Ground)), "{fighter:?}: own route book");
            assert!(uses(Act::Era(EraAct::Overhead)) && uses(Act::Era(EraAct::Enhanced)), "{fighter:?}: SF3 tools");
            assert_eq!(uses(Act::Tool(Tool::CommandThrow)), matches!(fighter, Retsu | Geki | Birdie), "{fighter:?}: grab");
            assert_eq!(uses(Act::Tool(Tool::Teleport)), fighter == Geki, "{fighter:?}: teleport");
            assert_eq!(uses(Act::Tool(Tool::Charge)), matches!(fighter, Birdie | Lee | Gen), "{fighter:?}: charge");
            assert_eq!(uses(Act::Era(EraAct::Stance)), fighter == Gen, "{fighter:?}: stance");
            assert!(!uses(Act::Era(EraAct::CustomCombo)), "SF3 does not start Alpha's Custom Combo");
            for p in plans {
                for s in p.steps {
                    assert!(s.then.iter().flatten().all(|&next| usize::from(next) < p.steps.len()), "{fighter:?}: valid branch");
                }
            }
        }
    }
}
