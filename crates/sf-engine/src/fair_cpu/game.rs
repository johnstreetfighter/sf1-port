//! Situations and game plans (SC2, `#194`; spec `#192`, ADR-0026): the
//! stronger Fair CPU's play, behind the developer-only switch
//! ([`crate::config::MatchConfig::stronger_cpu`]).
//!
//! **Situations.** Each frame the CPU is shown, through its reaction delay,
//! is judged to be one [`Situation`], with the corner as a modifier
//! ([`Judge::judge`]). Nothing but percepts goes in.
//!
//! **Game plans.** A [`GamePlan`] is a short script of [`Step`]s for one
//! situation. A step says what to do and which step follows each
//! [`Outcome`] the CPU then sees. The CPU's state holds only where it is in
//! a plan ([`Running`]); the plans themselves are tables here, looked up by
//! fighter and era style, so choosing and following one costs a few
//! comparisons a frame.

use super::{Body, Choice, Percept};
use crate::era::EraFamily;
use crate::roster::TournamentFighter;

/// The playfield's walls as a fighter's centre meets them, and how near one
/// counts as the corner. Measured by walking Ryu into each wall of
/// Training's stage in the SF6 era style; other stages are not checked.
const LEFT_WALL: i16 = 114;
const RIGHT_WALL: i16 = 400;
const CORNER: i16 = 40;

/// A reach to assume for the opponent's attacks until one is seen.
const DEFAULT_FOE_REACH: i16 = 66;
/// Frames "it was blocked" outlasts the last frame its attack was seen on a
/// guard: about the time a guarded normal takes to recover.
const BLOCKED_FRAMES: u8 = 14;
/// How far past its longest reach a recovering opponent still counts as
/// having whiffed in reach: a step forward closes it.
const WHIFF_MARGIN: i16 = 24;
/// Frames after its attack's input by which the CPU has seen what came of
/// it, if anything did.
const WATCH_FRAMES: u32 = 45;
/// The longest a game plan may take from its first step, in frames. A
/// plan still unfinished then is dropped.
pub(super) const PLAN_FRAMES: u32 = 300;

/// Which state the Fair CPU judges the fight to be in, from the frame it is
/// shown through its reaction delay. The corner is not one of these: it
/// modifies whichever holds ([`Judged::corner`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Situation {
    /// Outside both fighters' longest reach.
    #[default]
    NeutralFar,
    /// Inside someone's reach, nobody committed.
    NeutralClose,
    /// The opponent is in a hit reaction.
    ItHit,
    /// Its attack met a guard.
    ItWasBlocked,
    /// The opponent is knocked down or rising.
    OpponentKnockedDown,
    /// The opponent's attack missed and he is recovering in reach.
    OpponentWhiffed,
    /// The opponent's attack, jump or projectile is seen, or it is being hit.
    Defending,
    /// It is knocked down or rising.
    KnockedDown,
}

/// A judged frame: the situation, and whether either fighter is near a wall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Judged {
    pub situation: Situation,
    pub corner: bool,
}

/// Knocked down or rising: a hit reaction drawn off the ground (state bit 0,
/// as a sweep's and a launch's are), or the rise from a throw.
fn knocked_down(body: &Body) -> bool {
    body.present && (body.move_id == crate::combo::throw::GET_UP_MOVE || (body.move_id >= 0x30 && body.state & 1 != 0))
}

fn cornered(body: &Body) -> bool {
    body.present && (body.x <= LEFT_WALL + CORNER || body.x >= RIGHT_WALL - CORNER)
}

/// What judging remembers from one seen frame to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Judge {
    /// The opponent's current attack has shown a live attack box, and has
    /// met the CPU's fighter (a guard or a hit).
    foe_attack_shown: bool,
    foe_attack_met: bool,
    /// The longest any of the opponent's attack boxes has been seen to reach.
    foe_reach: i16,
    /// Frames left of "it was blocked".
    blocked: u8,
}

impl Default for Judge {
    fn default() -> Self {
        Judge { foe_attack_shown: false, foe_attack_met: false, foe_reach: DEFAULT_FOE_REACH, blocked: 0 }
    }
}

impl Judge {
    /// Judge `view`, the frame after `before`. `my_reach` is the CPU's
    /// longest normal as far as it knows.
    pub(super) fn judge(&mut self, view: Percept, before: Option<Percept>, my_reach: i16) -> Judged {
        let (me, foe) = (view.me, view.foe);
        let foe_was = before.map(|b| b.foe);
        if foe.attacking() {
            if foe_was.is_none_or(|was| !was.attacking() || was.move_id != foe.move_id) {
                self.foe_attack_shown = false;
                self.foe_attack_met = false;
            }
            if foe.active {
                self.foe_attack_shown = true;
                let toward_me = if me.x >= foe.x { 1 } else { -1 };
                self.foe_reach = self.foe_reach.max(((foe.front - foe.x) * toward_me).clamp(16, 120));
            }
            if me.reacting() || me.guarding() {
                self.foe_attack_met = true;
            }
        } else {
            self.foe_attack_shown = false;
            self.foe_attack_met = false;
        }
        // A guard pose alone is not a guarded attack: the pose is drawn as
        // soon as an attack is near. The attack must also be in reach.
        if me.attacking() && !me.jumping() && foe.guarding() && view.gap() <= my_reach + 8 {
            self.blocked = BLOCKED_FRAMES;
        } else if foe.attacking() || foe.reacting() || me.reacting() {
            self.blocked = 0;
        } else {
            self.blocked = self.blocked.saturating_sub(1);
        }

        let situation = if knocked_down(&me) {
            Situation::KnockedDown
        } else if me.reacting() {
            Situation::Defending
        } else if knocked_down(&foe) {
            Situation::OpponentKnockedDown
        } else if foe.reacting() {
            Situation::ItHit
        } else if self.blocked > 0 {
            Situation::ItWasBlocked
        } else if foe.attacking()
            && !foe.active
            && !foe.jumping()
            && self.foe_attack_shown
            && !self.foe_attack_met
            && view.gap() <= my_reach + WHIFF_MARGIN
        {
            Situation::OpponentWhiffed
        } else if view.threat().is_some() {
            Situation::Defending
        } else if view.gap() > my_reach.max(self.foe_reach) {
            Situation::NeutralFar
        } else {
            Situation::NeutralClose
        };
        Judged { situation, corner: cornered(&me) || cornered(&foe) }
    }
}

/// What the CPU sees come of a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Hit,
    Guard,
    Whiff,
    Jumped,
    Nothing,
}

/// What a step does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Act {
    /// Walk in, once, as far as it takes for this normal to reach where
    /// the opponent was last seen. The next step follows wherever he is.
    Reach { crouch: bool, button: usize },
    /// One of the CPU's actions, pressed as it presses it today.
    Do(Choice),
}

/// One step of a game plan: what to do, and the step that follows each
/// outcome, in [`Outcome`]'s order. `None` ends the plan.
#[derive(Clone, Copy, Debug)]
pub(super) struct Step {
    pub act: Act,
    pub then: [Option<u8>; 5],
}

impl Step {
    pub(super) fn next(&self, outcome: Outcome) -> Option<u8> {
        self.then[outcome as usize]
    }
}

/// A short script for one situation, and how it scores against the other
/// things the CPU could do there.
#[derive(Clone, Copy, Debug)]
pub(super) struct GamePlan {
    pub situation: Situation,
    pub score: i32,
    pub steps: &'static [Step],
}

const fn step(act: Act, then: [Option<u8>; 5]) -> Step {
    Step { act, then }
}

/// After any outcome, the same next step (or the plan's end).
const fn always(next: Option<u8>) -> [Option<u8>; 5] {
    [next; 5]
}

/// Ryu in the SF6 era style, close neutral: walk to crouching medium kick
/// range and press it. On a hit throw a projectile to keep him pinned; on a
/// guard walk in and throw; on a whiff give ground and guard.
const RYU_SF6_POKE: [Step; 6] = [
    step(Act::Reach { crouch: true, button: 4 }, always(Some(1))),
    step(Act::Do(Choice::Normal { crouch: true, button: 4 }), [Some(2), Some(3), Some(4), None, None]),
    step(Act::Do(Choice::Special(super::Role::Projectile)), always(None)),
    step(Act::Do(Choice::Throw), always(None)),
    step(Act::Do(Choice::Retreat), always(Some(5))),
    step(Act::Do(Choice::Guard), always(None)),
];

const RYU_SF6: [GamePlan; 1] = [GamePlan { situation: Situation::NeutralClose, score: 60, steps: &RYU_SF6_POKE }];

/// The game plans a fighter has in an era style. A pair with none plays
/// the CPU's single choices.
pub(super) fn plans(fighter: Option<TournamentFighter>, family: Option<EraFamily>) -> &'static [GamePlan] {
    match (fighter, family) {
        (Some(TournamentFighter::Ryu), Some(EraFamily::Sf6)) => &RYU_SF6,
        _ => &[],
    }
}

/// Where the CPU is in a game plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Running {
    /// The plan's place in its pair's table, and the step in it.
    pub plan: u8,
    pub step: u8,
    /// The step's action has been pressed, on this step of the machine.
    pub acted: bool,
    pub pressed: u32,
    /// The machine step the plan began on.
    pub began: u32,
}

impl Running {
    /// What has come of the pressed normal `(crouch, button)` as far as
    /// `view` shows, once it shows anything: `reach` is that normal's.
    pub(super) fn outcome(&self, view: Percept, reach: i16) -> Option<Outcome> {
        if view.frame <= self.pressed {
            return None;
        }
        if view.foe.reacting() {
            Some(Outcome::Hit)
        } else if view.me.attacking() && view.foe.guarding() && view.gap() <= reach + 8 {
            Some(Outcome::Guard)
        } else if view.foe.jumping() {
            Some(Outcome::Jumped)
        } else if !view.me.attacking() && view.frame > self.pressed + 6 {
            Some(Outcome::Whiff)
        } else if view.frame > self.pressed + WATCH_FRAMES {
            Some(Outcome::Nothing)
        } else {
            None
        }
    }
}

crate::canonical::canonical_enum!(Situation, "FairCpuSituation.v1";
    NeutralFar => 0, NeutralClose => 1, ItHit => 2, ItWasBlocked => 3, OpponentKnockedDown => 4,
    OpponentWhiffed => 5, Defending => 6, KnockedDown => 7,
);
crate::canonical::canonical_struct!(Judged, "FairCpuJudged.v1", { situation, corner });
crate::canonical::canonical_struct!(Judge, "FairCpuJudge.v1", { foe_attack_shown, foe_attack_met, foe_reach, blocked });
crate::canonical::canonical_struct!(Running, "FairCpuGamePlan.v1", { plan, step, acted, pressed, began });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canonical_hash_covers_the_situation_the_judging_memory_and_the_game_plan() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &crate::fair_cpu::FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        let plain = crate::fair_cpu::FairCpu::new();
        let mut changed = [plain; 4];
        changed[0].judged.situation = Situation::ItHit;
        changed[1].judged.corner = true;
        changed[2].judge.blocked = 3;
        changed[3].game = Some(Running { plan: 0, step: 1, acted: true, pressed: 9, began: 4 });
        for cpu in &changed {
            assert_ne!(hash(cpu), hash(&plain), "{cpu:?}");
        }
        assert_ne!(hash(&plain.force_stronger(true)), hash(&plain));
    }

    #[test]
    fn every_game_plan_step_points_inside_its_plan() {
        for plan in RYU_SF6 {
            for step in plan.steps {
                for next in step.then.into_iter().flatten() {
                    assert!(usize::from(next) < plan.steps.len(), "{step:?}");
                }
            }
        }
    }
}
