//! Baseline SF1/SF1+ game plans (SC21, `#213`).
//!
//! Plain normals, spacing and delayed reactions remain useful without
//! combos or cancels. Throw and book steps are separate plans so the
//! baseline rules can exclude them before selection. The old Fair CPU
//! never receives this style identity.

use super::super::habit::Lean;
use super::super::{Choice, Level, Role};
use super::{
    always, plan, step, Act, GamePlan, Situation, Step, ANY, JUMP_IN_GAP, SHOT_GAP,
    WATCHED_SHOT_GAPS,
};
use super::{
    RYU_SF6_BAIT, RYU_SF6_JUMP_IN, RYU_SF6_SHOT, RYU_SF6_SHOT_WATCH, RYU_SF6_WALK_THROW,
    SF2_FRAME_TRAP,
};
use crate::roster::TournamentFighter;
use crate::route_book::Reach;

// A blocked poke keeps to another normal rather than taking an implicit
// throw branch. Throws get their own rule-gated plan below.
const fn poke(crouch: bool, button: usize) -> [Step; 5] {
    [
        step(Act::Reach { crouch, button }, always(Some(1))),
        step(
            Act::Do(Choice::Normal { crouch, button }),
            [Some(2), Some(3), Some(3), None, None],
        ),
        step(
            Act::Meaty { crouch, button },
            [None, None, None, None, Some(0)],
        ),
        step(Act::Do(Choice::Retreat), always(Some(4))),
        step(Act::Do(Choice::Guard), always(None)),
    ]
}

const fn meaty(crouch: bool, button: usize) -> [Step; 3] {
    [
        step(
            Act::Meaty { crouch, button },
            [None, Some(1), Some(1), None, None],
        ),
        step(Act::Do(Choice::Retreat), always(Some(2))),
        step(Act::Do(Choice::Guard), always(None)),
    ]
}

// The route player closes to each route's recorded spacing; no common
// fixed gap is assumed across the twelve baseline books.
const ROUTE: [Step; 1] = [step(Act::Route(Reach::Ground), always(None))];
const POKE_MP: [Step; 5] = poke(false, 1);
// Mike's close opener uses the ducking jab's quicker native startup.
// Keep the straight's on-hit meaty and every branch of the existing poke.
const MIKE_CLOSE_POKE: [Step; 5] = {
    let mut steps = poke(false, 1);
    steps[0].act = Act::Reach {
        crouch: true,
        button: 0,
    };
    steps[1].act = Act::Do(Choice::Normal {
        crouch: true,
        button: 0,
    });
    steps
};
const MEATY_MP: [Step; 3] = meaty(false, 1);
const POKE_HP: [Step; 5] = poke(false, 2);
// Lee opens close neutral with standing MP; retain HP on-hit timing
// and every branch of the original HP poke.
const LEE_CLOSE_MP_POKE: [Step; 5] = {
    let mut steps = poke(false, 2);
    steps[0].act = Act::Reach {
        crouch: false,
        button: 1,
    };
    steps[1].act = Act::Do(Choice::Normal {
        crouch: false,
        button: 1,
    });
    steps
};
const MEATY_HP: [Step; 3] = meaty(false, 2);
const POKE_MK: [Step; 5] = poke(false, 4);
const MEATY_MK: [Step; 3] = meaty(false, 4);
const POKE_HK: [Step; 5] = poke(false, 5);
const MEATY_HK: [Step; 3] = meaty(false, 5);
// Geki opens close neutral from his crouching claw posture; keep the
// on-hit meaty HK and every branch of the existing poke.
const GEKI_CLOSE_POKE: [Step; 5] = {
    let mut steps = poke(false, 5);
    steps[0].act = Act::Reach {
        crouch: true,
        button: 1,
    };
    steps[1].act = Act::Do(Choice::Normal {
        crouch: true,
        button: 1,
    });
    steps
};
const ADVANCE: [Step; 3] = [
    step(Act::Do(Choice::Special(Role::Advance)), always(Some(1))),
    step(
        Act::Reach {
            crouch: true,
            button: 3,
        },
        always(Some(2)),
    ),
    step(
        Act::Do(Choice::Normal {
            crouch: true,
            button: 3,
        }),
        always(None),
    ),
];
const ADVANCE_CLOSE: GamePlan = plan(
    Situation::NeutralClose,
    78,
    0,
    (40, 110),
    &ADVANCE,
    Lean::None,
    Level::Easy,
);
// Birdie's existing standing MP follows the Horn at its own reach.
const BIRDIE_ADVANCE: [Step; 3] = [
    step(Act::Do(Choice::Special(Role::Advance)), always(Some(1))),
    step(
        Act::Reach {
            crouch: false,
            button: 1,
        },
        always(Some(2)),
    ),
    step(
        Act::Do(Choice::Normal {
            crouch: false,
            button: 1,
        }),
        always(None),
    ),
];
const BIRDIE_ADVANCE_CLOSE: GamePlan = plan(
    Situation::NeutralClose,
    78,
    0,
    (40, 110),
    &BIRDIE_ADVANCE,
    Lean::None,
    Level::Easy,
);
const BIRDIE_ADVANCE_FAR: GamePlan = plan(
    Situation::NeutralFar,
    78,
    0,
    (100, 260),
    &BIRDIE_ADVANCE,
    Lean::None,
    Level::Easy,
);
const ADVANCE_GAP: i16 = 84;
const SPACED_ADVANCE: [Step; 4] = [
    step(Act::Space(ADVANCE_GAP), always(Some(1))),
    step(Act::Do(Choice::Special(Role::Advance)), always(Some(2))),
    step(
        Act::Reach {
            crouch: true,
            button: 3,
        },
        always(Some(3)),
    ),
    step(
        Act::Do(Choice::Normal {
            crouch: true,
            button: 3,
        }),
        always(None),
    ),
];
const SPACED_ADVANCE_FAR: GamePlan = plan(
    Situation::NeutralFar,
    78,
    0,
    (ADVANCE_GAP, 260),
    &SPACED_ADVANCE,
    Lean::None,
    Level::Easy,
);

macro_rules! shared {
    ($poke:expr, $meaty:expr $(, $extra:expr)*) => {
        shared!(@close $poke, $poke, $meaty $(, $extra)*)
    };
    (@close $close:expr, $poke:expr, $meaty:expr $(, $extra:expr)*) => {
        &[
            plan(Situation::NeutralClose, 60, 0, ANY, $close, Lean::None, Level::Easy),
            plan(Situation::NeutralClose, 62, 6, ANY, &ROUTE, Lean::None, Level::Easy),
            plan(Situation::ItWasBlocked, 60, 4, ANY, &SF2_FRAME_TRAP, Lean::FrameTrap, Level::Normal),
            plan(Situation::ItWasBlocked, 56, 12, ANY, &RYU_SF6_WALK_THROW, Lean::WalkThrow, Level::Easy),
            plan(Situation::OpponentKnockedDown, 62, 8, ANY, $meaty, Lean::Meaty, Level::Normal),
            plan(Situation::OpponentKnockedDown, 52, -10, ANY, &RYU_SF6_BAIT, Lean::Bait, Level::VeryHard),
            plan(Situation::NeutralFar, 50, 10, ANY, $poke, Lean::None, Level::Easy),
            plan(Situation::NeutralFar, 70, 0, (SHOT_GAP, i16::MAX), &RYU_SF6_SHOT, Lean::Shot, Level::Easy),
            plan(Situation::NeutralFar, 0, 0, WATCHED_SHOT_GAPS, &RYU_SF6_SHOT_WATCH, Lean::ShotWatch, Level::Normal),
            plan(Situation::NeutralFar, 42, 0, JUMP_IN_GAP, &RYU_SF6_JUMP_IN, Lean::JumpIn, Level::Easy),
            plan(Situation::ItHit, 60, 0, ANY, $poke, Lean::None, Level::Easy),
            $($extra,)*
        ]
    };
}

const MEDIUM: &[GamePlan] = shared!(&POKE_MP, &MEATY_MP);
const MIKE: &[GamePlan] = shared!(@close &MIKE_CLOSE_POKE, &POKE_MP, &MEATY_MP);
const HEAVY_PUNCH: &[GamePlan] = shared!(&POKE_HP, &MEATY_HP);
const MEDIUM_KICK: &[GamePlan] = shared!(&POKE_MK, &MEATY_MK);
const GEN: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK, ADVANCE_CLOSE, SPACED_ADVANCE_FAR);
const BIRDIE: &[GamePlan] = shared!(
    &POKE_MP,
    &MEATY_MP,
    BIRDIE_ADVANCE_CLOSE,
    BIRDIE_ADVANCE_FAR
);
const LEE: &[GamePlan] =
    shared!(@close &LEE_CLOSE_MP_POKE, &POKE_HP, &MEATY_HP, ADVANCE_CLOSE, SPACED_ADVANCE_FAR);
const HEAVY: &[GamePlan] = shared!(&POKE_HK, &MEATY_HK);
const GEKI: &[GamePlan] = shared!(@close &GEKI_CLOSE_POKE, &POKE_HK, &MEATY_HK);

pub(super) fn plans(fighter: TournamentFighter) -> &'static [GamePlan] {
    use TournamentFighter::*;
    match fighter {
        Ryu | Ken | Sagat => HEAVY,
        Geki => GEKI,
        Gen => GEN,
        Birdie => BIRDIE,
        Lee => LEE,
        Joe => MEDIUM_KICK,
        Eagle => HEAVY_PUNCH,
        Mike => MIKE,
        Retsu | Adon => MEDIUM,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_plans_keep_plain_pressure_separate_from_optional_rules() {
        for fighter in TournamentFighter::ALL {
            let table = plans(fighter);
            assert!(
                table.len() <= super::super::PLAN_SLOTS,
                "{fighter:?}: bounded snapshot records"
            );
            for situation in [
                Situation::NeutralClose,
                Situation::NeutralFar,
                Situation::ItHit,
                Situation::ItWasBlocked,
                Situation::OpponentKnockedDown,
            ] {
                assert!(
                    table.iter().any(|p| p.situation == situation
                        && !p.needs_specials()
                        && p.steps
                            .iter()
                            .all(|s| !matches!(s.act, Act::Route(_) | Act::Do(Choice::Throw)))),
                    "{fighter:?}: plain pressure in {situation:?}"
                );
            }
            assert!(
                table.iter().any(|p| p.situation == Situation::NeutralFar
                    && p.gap == ANY
                    && p.for_level(Level::Easy)
                    && matches!(p.steps.first().map(|s| s.act), Some(Act::Reach { .. }))),
                "{fighter:?}: an ordinary approach without a projectile or book"
            );
            for p in table {
                for s in p.steps {
                    assert!(
                        s.then
                            .iter()
                            .flatten()
                            .all(|&next| usize::from(next) < p.steps.len()),
                        "{fighter:?}: valid branch"
                    );
                    assert!(
                        !matches!(s.act, Act::Era(_) | Act::Rush | Act::Tool(_)),
                        "{fighter:?}: no later-era command"
                    );
                }
            }
        }
    }
}
