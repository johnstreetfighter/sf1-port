//! SF6 game plans for the remaining tournament cast (SC20, `#212`).
//!
//! Ryu keeps his original table in `game.rs`. These fighters share its
//! situation judge, focus, habits, pressure and route player, with their
//! own normals and tools. Drive Parry and Reversal stay delayed reactions;
//! Impact, OD and Super Arts have deliberate steps whose readiness reads
//! the live kit and gauges. Initial scores start from the preceding cast
//! rollouts; the measuring report records where they pass or miss.

use super::super::cast::Tool;
use super::super::habit::Lean;
use super::super::{Choice, Level, Role};
use super::{always, meaty, plan, poke, step, Act, EraAct, GamePlan, Situation, Step, ANY, CHARGE, COMMAND_THROW, JUMP_IN_GAP, SHOT_GAP, WATCHED_SHOT_GAPS};
use super::{RYU_SF6_BAIT, RYU_SF6_FRAME_TRAP, RYU_SF6_JUMP_IN, RYU_SF6_ROUTE, RYU_SF6_SHOT, RYU_SF6_SHOT_RUSH, RYU_SF6_SHOT_WATCH, RYU_SF6_WALK_THROW};
use crate::roster::TournamentFighter;

const IMPACT: [Step; 1] = [step(Act::Era(EraAct::DriveImpact), always(None))];
const OD: [Step; 1] = [step(Act::Era(EraAct::Enhanced), always(None))];
const SUPER: [Step; 1] = [step(Act::Era(EraAct::Super), always(None))];
const STANCE: [Step; 1] = [step(Act::Era(EraAct::Stance), always(None))];
const TELEPORT: [Step; 1] = [step(Act::Tool(Tool::Teleport), always(None))];
const ADVANCE: [Step; 1] = [step(Act::Do(Choice::Special(Role::Advance)), always(None))];
// A raw Rush begins from spacing that leaves room for its travelling move,
// then its fighter's own medium punch. The new cast's selection gate checks
// Drive before taking a plan containing Rush; execution checks it again.
const RUSH: [Step; 4] = [
    step(Act::Space(112), always(Some(1))),
    step(Act::Rush, always(Some(2))),
    step(Act::Reach { crouch: false, button: 1 }, always(Some(3))),
    step(Act::Do(Choice::Normal { crouch: false, button: 1 }), always(None)),
];

const POKE_HK: [Step; 6] = poke(false, 5);
const MEATY_HK: [Step; 4] = meaty(false, 5);
const POKE_MP: [Step; 6] = poke(false, 1);
const MEATY_MP: [Step; 4] = meaty(false, 1);
const POKE_LK: [Step; 6] = poke(false, 3);
const MEATY_LK: [Step; 4] = meaty(false, 3);

// Twelve shared records leave up to four places for the fighter's own
// projectile and tools without changing the snapshot's sixteen slots.
macro_rules! shared {
    ($poke:expr, $meaty:expr $(, $more:expr)* $(,)?) => {
        &[
            plan(Situation::NeutralClose, 60, 0, ANY, $poke, Lean::None, Level::Easy),
            plan(Situation::NeutralClose, 62, 6, ANY, &RYU_SF6_ROUTE, Lean::None, Level::Easy),
            plan(Situation::ItWasBlocked, 60, 4, ANY, &RYU_SF6_FRAME_TRAP, Lean::FrameTrap, Level::Normal),
            plan(Situation::ItWasBlocked, 56, 12, ANY, &RYU_SF6_WALK_THROW, Lean::WalkThrow, Level::Easy),
            plan(Situation::NeutralClose, 64, 0, (i16::MIN, 80), &OD, Lean::None, Level::Normal),
            plan(Situation::OpponentKnockedDown, 62, 8, ANY, $meaty, Lean::Meaty, Level::Normal),
            plan(Situation::OpponentKnockedDown, 52, -10, ANY, &RYU_SF6_BAIT, Lean::Bait, Level::VeryHard),
            plan(Situation::NeutralFar, 50, 10, ANY, $poke, Lean::None, Level::Easy),
            plan(Situation::NeutralFar, 64, 4, (90, 160), &RUSH, Lean::None, Level::Hard),
            // A fresh close decision can pressure a guard with Impact.
            // A blocked attack's short remembered outcome usually belongs
            // to the plan still finishing that attack, before we are free.
            plan(Situation::NeutralClose, 64, 8, (i16::MIN, 90), &IMPACT, Lean::FrameTrap, Level::Normal),
            plan(Situation::ItHit, 80, 0, (i16::MIN, 90), &SUPER, Lean::None, Level::Normal),
            plan(Situation::OpponentWhiffed, 80, 0, (i16::MIN, 90), &SUPER, Lean::None, Level::Normal),
            $($more),*
        ]
    };
}

const SHOT: GamePlan = plan(Situation::NeutralFar, 70, 0, (SHOT_GAP, i16::MAX), &RYU_SF6_SHOT, Lean::Shot, Level::Easy);
const SHOT_RUSH: GamePlan = plan(Situation::NeutralFar, 66, 6, (SHOT_GAP, i16::MAX), &RYU_SF6_SHOT_RUSH, Lean::ShotRush, Level::Hard);
const SHOT_WATCH: GamePlan = plan(Situation::NeutralFar, 0, 0, WATCHED_SHOT_GAPS, &RYU_SF6_SHOT_WATCH, Lean::ShotWatch, Level::Normal);
const JUMP_FAR: GamePlan = plan(Situation::NeutralFar, 46, 0, JUMP_IN_GAP, &RYU_SF6_JUMP_IN, Lean::JumpIn, Level::Easy);
const ADVANCE_FAR: GamePlan = plan(Situation::NeutralFar, 54, 0, (120, i16::MAX), &ADVANCE, Lean::None, Level::Easy);
const THROW_BLOCKED: GamePlan = plan(Situation::ItWasBlocked, 58, 8, ANY, &COMMAND_THROW, Lean::WalkThrow, Level::Normal);
// A free projectile must not always outrank Geki's other way in. Failed
// decision quality does not choose the runner-up, so its base score needs
// a real opportunity before the habit/repetition bonuses are learned.
const TELEPORT_FAR: GamePlan = plan(Situation::NeutralFar, 78, -6, (130, i16::MAX), &TELEPORT, Lean::None, Level::Normal);
const CHARGE_FAR: GamePlan = plan(Situation::NeutralFar, 50, 0, (150, i16::MAX), &CHARGE, Lean::None, Level::Easy);
// Once in Crane, Jasen must compete with the raw Rush in this same gap;
// a lower score leaves the charge unavailable in practice while Drive lasts.
const GEN_CHARGE: GamePlan = plan(Situation::NeutralFar, 68, 0, (60, 170), &CHARGE, Lean::None, Level::Easy);
const CRANE: GamePlan = plan(Situation::NeutralFar, 50, -10, ANY, &STANCE, Lean::None, Level::Easy);

const KEN: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK, SHOT, SHOT_RUSH, SHOT_WATCH);
// Native SF6 Retsu has Tobigeri and Renkiko, not his what-if command catch.
const RETSU: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, SHOT, SHOT_RUSH, SHOT_WATCH);
const GEKI: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, SHOT, SHOT_RUSH, THROW_BLOCKED, TELEPORT_FAR);
const JOE: &[GamePlan] = shared!(&POKE_LK, &MEATY_LK, SHOT, SHOT_RUSH, SHOT_WATCH);
const MIKE: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, ADVANCE_FAR, JUMP_FAR);
const LEE: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, ADVANCE_FAR, CHARGE_FAR, JUMP_FAR);
const GEN: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK, CRANE, GEN_CHARGE, JUMP_FAR);
const BIRDIE: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, ADVANCE_FAR, THROW_BLOCKED, CHARGE_FAR, JUMP_FAR);
const EAGLE: &[GamePlan] = shared!(&POKE_LK, &MEATY_LK, ADVANCE_FAR, JUMP_FAR);
const ADON: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, ADVANCE_FAR, JUMP_FAR);
const SAGAT: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP, SHOT, SHOT_RUSH, SHOT_WATCH);

pub(super) fn plans(fighter: TournamentFighter) -> &'static [GamePlan] {
    use TournamentFighter::*;
    match fighter {
        Ryu => &[], // The original table is dispatched first in game.rs.
        Ken => KEN,
        Retsu => RETSU,
        Geki => GEKI,
        Joe => JOE,
        Mike => MIKE,
        Lee => LEE,
        Gen => GEN,
        Birdie => BIRDIE,
        Eagle => EAGLE,
        Adon => ADON,
        Sagat => SAGAT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sf6_cast_has_its_own_routes_situations_tools_and_bounded_records() {
        use TournamentFighter::*;
        for fighter in TournamentFighter::ALL.into_iter().filter(|&fighter| fighter != Ryu) {
            let plans = plans(fighter);
            assert!(plans.len() <= super::super::PLAN_SLOTS, "{fighter:?}: snapshot records fit");
            for situation in [Situation::NeutralClose, Situation::NeutralFar, Situation::ItWasBlocked, Situation::OpponentKnockedDown, Situation::ItHit, Situation::OpponentWhiffed] {
                assert!(plans.iter().any(|p| p.situation == situation), "{fighter:?}: {situation:?}");
            }
            assert!(plans.iter().any(|p| {
                p.situation == Situation::NeutralFar && p.gap == ANY && !p.cornered_only
                    && p.for_level(Level::Easy) && !p.needs_specials()
                    && matches!(p.steps.first().map(|s| s.act), Some(Act::Reach { .. }))
            }), "{fighter:?}: ordinary approach survives disabled gauges or tools");
            let uses = |act| plans.iter().any(|p| p.steps.iter().any(|s| s.act == act));
            assert!(uses(Act::Route(crate::route_book::Reach::Ground)), "{fighter:?}: own book");
            assert!(uses(Act::Rush) && uses(Act::Era(EraAct::DriveImpact)) && uses(Act::Era(EraAct::Enhanced)) && uses(Act::Era(EraAct::Super)), "{fighter:?}: deliberate SF6 tools");
            assert_eq!(uses(Act::Tool(Tool::CommandThrow)), matches!(fighter, Geki | Birdie), "{fighter:?}: ordinary command catch");
            assert_eq!(uses(Act::Tool(Tool::Teleport)), fighter == Geki, "{fighter:?}: teleport");
            assert_eq!(uses(Act::Tool(Tool::Charge)), matches!(fighter, Lee | Gen | Birdie), "{fighter:?}: charge");
            assert_eq!(uses(Act::Era(EraAct::Stance)), fighter == Gen, "{fighter:?}: Crane stance");
            for p in plans {
                for s in p.steps {
                    assert!(s.then.iter().flatten().all(|&next| usize::from(next) < p.steps.len()), "{fighter:?}: valid branch");
                    assert!(!matches!(s.act, Act::Era(EraAct::CustomCombo | EraAct::CustomChain | EraAct::Overhead | EraAct::Focus | EraAct::Ultra | EraAct::VSkill | EraAct::VTrigger)), "{fighter:?}: no other era command");
                }
            }
        }
        assert!(plans(Ryu).is_empty(), "Ryu has no new cast override");
        assert_eq!(super::super::plans(Some(Ryu), Some(crate::era::EraFamily::Sf6)).len(), 13, "Ryu keeps his original table");
    }

    #[test]
    fn sf6_paid_plans_keep_easy_free_and_super_arts_use_seen_openings() {
        for fighter in TournamentFighter::ALL {
            for p in plans(fighter) {
                let uses = |act| p.steps.iter().any(|s| s.act == act);
                if uses(Act::Rush) || uses(Act::Era(EraAct::DriveImpact)) || uses(Act::Era(EraAct::Enhanced)) || uses(Act::Era(EraAct::Super)) {
                    assert!(!p.for_level(Level::Easy), "{fighter:?}: no self-started paid plan on Easy");
                }
                if uses(Act::Era(EraAct::Super)) {
                    assert!(matches!(p.situation, Situation::ItHit | Situation::OpponentWhiffed), "{fighter:?}: seen hit or whiff");
                    assert!(p.gap.1 <= 90, "{fighter:?}: close enough for a finisher");
                }
            }
        }
    }
}
