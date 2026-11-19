//! Focus and the two reaction delays (SC7, `#199`; spec `#192`, ADR-0026):
//! the stronger Fair CPU holds one focus at a time, the event it expects
//! most where it is, and answers that event at its level's focused delay
//! ([`super::Level::reaction`]) and anything else at the slower one
//! ([`super::Level::unfocused_reaction`]).
//!
//! **The two-frame rule.** The CPU decides from the frame its focused
//! delay shows. The opponent's new actions are the events it answers: an
//! attack, a jump, a projectile, a whiff, a walk into throw range and a
//! reversal ([`Focus`]). One that matches its focus shows at once. Any other
//! is hidden until it is the slower delay old: until then the opponent's
//! pose, height, attack box and projectile are drawn as they were the frame
//! before it began, moved with him to where he is
//! ([`super::FairCpu::shown`]). Positions, health, its own fighter and what
//! came of its own attacks are seen at the focused delay (the maintainer's
//! choice, 2026-11-19). So a new action changes its pad no sooner than its
//! focused delay if it is the focus, and no sooner than its slower delay if
//! it is not.
//!
//! **Which focus.** A game plan step sets it from what the step does
//! ([`step_focus`]); outside a game plan the situation sets it
//! ([`situation_focus`]).

use super::game::{knocked_down, Act, Step};
use super::{Choice, Percept, Situation};

/// The one event the Fair CPU is watching for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Focus {
    /// The opponent's jump, from the pre-jump crouch.
    #[default]
    Jump,
    /// The opponent's projectile thrown at it.
    Projectile,
    /// An attack the opponent starts from the ground (or a new one after
    /// another): a button pressed.
    Attack,
    /// The opponent's attack box going without having met it: a whiff.
    Whiff,
    /// An attack begun straight from a knockdown or the rise from one.
    Reversal,
    /// The opponent walking into throw range.
    WalkIn,
}

impl Focus {
    /// Every new action of the opponent's the CPU answers.
    pub(super) const ALL: [Focus; 6] =
        [Focus::Jump, Focus::Projectile, Focus::Attack, Focus::Whiff, Focus::Reversal, Focus::WalkIn];

    /// `view`, the frame after `before`, shows this event starting.
    /// `engage` is how close a walk in must come to count.
    pub(super) fn shows(self, view: &Percept, before: &Percept, engage: i16) -> bool {
        let (foe, was) = (view.foe, before.foe);
        let attack_started = foe.attacking() && !(was.jumping() && foe.jumping()) && (!was.attacking() || foe.move_id != was.move_id);
        match self {
            Focus::Jump => matches!(foe.move_id, 9..=0xb) && !was.jumping(),
            Focus::Projectile => view.shot && !before.shot && view.shot_coming(),
            Focus::Attack => attack_started && !knocked_down(&was),
            Focus::Whiff => foe.attacking() && was.active && !foe.active && !view.me.reacting(),
            Focus::Reversal => attack_started && knocked_down(&was),
            Focus::WalkIn => {
                foe.move_id == 2 && !foe.attacking() && view.dist() <= engage && (was.move_id != 2 || before.dist() > engage)
            }
        }
    }
}

/// The focus a situation sets outside a game plan: far off, a jump in; in
/// reach, a whiff (it hovers at the edge of range); over a knocked-down
/// opponent, a reversal; otherwise a button pressed (while it guards, the
/// next attack of his string or a throw, which is an attack move too).
pub(super) fn situation_focus(situation: Situation) -> Focus {
    match situation {
        Situation::NeutralFar => Focus::Jump,
        Situation::NeutralClose => Focus::Whiff,
        Situation::ItHit
        | Situation::ItWasBlocked
        | Situation::OpponentWhiffed
        | Situation::Defending
        | Situation::KnockedDown => Focus::Attack,
        Situation::OpponentKnockedDown => Focus::Reversal,
    }
}

/// The focus a game plan step sets, from what it does: walking in to reach
/// him, a whiff; timing an attack or a guard to his rise, a reversal; a
/// projectile, the jump over it; a guard after giving ground, his walk in;
/// anything else, a button pressed against it.
pub(super) fn step_focus(step: &Step) -> Focus {
    match step.act {
        Act::Reach { .. } | Act::Do(Choice::Approach) => Focus::Whiff,
        Act::Meaty { .. } | Act::GuardRise => Focus::Reversal,
        Act::Do(Choice::Special(super::Role::Projectile)) => Focus::Jump,
        Act::Do(Choice::Guard) => Focus::WalkIn,
        Act::Do(_) | Act::Route(_) => Focus::Attack,
    }
}

crate::canonical::canonical_enum!(Focus, "FairCpuFocus.v1";
    Jump => 0,
    Projectile => 1,
    Attack => 2,
    Whiff => 3,
    Reversal => 4,
    WalkIn => 5,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_situation_and_every_game_plan_step_sets_a_focus() {
        use crate::era::EraFamily;
        use crate::roster::TournamentFighter;
        for situation in [
            Situation::NeutralFar,
            Situation::NeutralClose,
            Situation::ItHit,
            Situation::ItWasBlocked,
            Situation::OpponentKnockedDown,
            Situation::OpponentWhiffed,
            Situation::Defending,
            Situation::KnockedDown,
        ] {
            let _ = situation_focus(situation);
        }
        let plans = super::super::game::plans(Some(TournamentFighter::Ryu), Some(EraFamily::Sf6));
        let foci: Vec<Focus> = plans.iter().flat_map(|plan| plan.steps.iter().map(step_focus)).collect();
        assert!(!foci.is_empty());
        // Timed attacks on a rise watch for the reversal; walking in watches
        // for the whiff; a guard after giving ground for his walk in; the
        // rest for a button.
        for want in [Focus::Reversal, Focus::Whiff, Focus::WalkIn, Focus::Attack] {
            assert!(foci.contains(&want), "{want:?} among {foci:?}");
        }
    }
}
