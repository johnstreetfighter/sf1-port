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
use crate::route_book::{Reach, Route, Stance};

/// The playfield's walls as a fighter's centre meets them, and how near one
/// counts as the corner. Measured by walking Ryu into each wall of
/// Training's stage in the SF6 era style; other stages are not checked.
const LEFT_WALL: i16 = 114;
const RIGHT_WALL: i16 = 400;
const CORNER: i16 = 40;

/// A reach to assume for the opponent's attacks until one is seen.
const DEFAULT_FOE_REACH: i16 = 66;
/// Frames from a knockdown showing to the opponent being free, until the
/// CPU has seen one: about a sweep's, as Training's Ryu shows it.
const DEFAULT_RISE: u16 = 43;
/// Frames one knockdown seen moves the CPU's idea of their length, at most.
const RISE_STEP: i64 = 3;
/// Frames before a rising opponent is free at which a timed attack is
/// pressed: a little under a quick normal's start-up, so it is out just
/// after he rises. (This engine lets a hit land on a knockdown's last
/// frame and knock him down again; six frames' lead met that frame.)
pub(super) const MEATY_LEAD: u32 = 4;
/// Frames a guard held over a rise lasts past it, for a reversal to show.
pub(super) const BAIT_FRAMES: u32 = 24;
/// Frames before a rising opponent is free at which the guard over his
/// rise goes up, ending the walk in.
pub(super) const GUARD_LEAD: u32 = 4;
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
    /// The opponent is the one near a wall.
    pub foe_cornered: bool,
}

/// Knocked down or rising: a hit reaction drawn off the ground (state bit 0,
/// as a sweep's and a launch's are), or the rise from a throw.
pub(super) fn knocked_down(body: &Body) -> bool {
    body.present && (body.move_id == crate::combo::throw::GET_UP_MOVE || (body.move_id >= 0x30 && body.state & 1 != 0))
}

fn cornered(body: &Body) -> bool {
    body.present && (body.x <= LEFT_WALL + CORNER || body.x >= RIGHT_WALL - CORNER)
}

/// What judging remembers from one seen frame to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Judge {
    /// The opponent's current attack has shown a live attack box, and that
    /// box has reached the CPU's fighter (or hit him).
    foe_attack_shown: bool,
    foe_attack_met: bool,
    /// The opponent's current attack began on the ground: one that rises
    /// (a Shoryuken) is still a whiff to punish as it falls.
    foe_attack_grounded: bool,
    /// The longest any of the opponent's attack boxes has been seen to reach.
    foe_reach: i16,
    /// Frames left of "it was blocked".
    blocked: u8,
    /// The opponent was knocked down on the last frame judged, the frame
    /// his knockdown was first seen, and how long a knockdown lasts until
    /// he is free, as the CPU has seen them last.
    foe_down: bool,
    foe_down_since: u32,
    rise_frames: u16,
}

impl Default for Judge {
    fn default() -> Self {
        Judge {
            foe_attack_shown: false,
            foe_attack_met: false,
            foe_attack_grounded: false,
            foe_reach: DEFAULT_FOE_REACH,
            blocked: 0,
            foe_down: false,
            foe_down_since: 0,
            rise_frames: DEFAULT_RISE,
        }
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
                self.foe_attack_grounded = foe_was.is_some_and(|was| !was.jumping());
            }
            if foe.active {
                self.foe_attack_shown = true;
                let toward_me = if me.x >= foe.x { 1 } else { -1 };
                self.foe_reach = self.foe_reach.max(((foe.front - foe.x) * toward_me).clamp(16, 120));
                // The box's forward edge at or past the near edge of the
                // CPU's hurtbox: it reached him, guarded or not.
                let near = if toward_me > 0 { me.hurt.0 } else { me.hurt.1 };
                if (foe.front - near) * toward_me >= 0 {
                    self.foe_attack_met = true;
                }
            }
            if me.reacting() {
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

        // A knockdown's length, from the first frame it shows to the first
        // frame he is free, learned as a person learns it by watching.
        let down = knocked_down(&foe);
        if down && !self.foe_down {
            self.foe_down_since = view.frame;
        } else if !down && self.foe_down && foe.free() {
            let seen = view.frame.saturating_sub(self.foe_down_since);
            if (20..=150).contains(&seen) {
                // The usual length: one sighting moves it a few frames at
                // most, so an odd long knockdown (about one in five lasts
                // twice the usual) does not throw the timing off.
                let shift = (i64::from(seen) - i64::from(self.rise_frames)).clamp(-RISE_STEP, RISE_STEP);
                self.rise_frames = (i64::from(self.rise_frames) + shift) as u16;
            }
        }
        self.foe_down = down;

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
            && (!foe.jumping() || self.foe_attack_grounded)
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
        Judged { situation, corner: cornered(&me) || cornered(&foe), foe_cornered: cornered(&foe) }
    }

    /// The machine frame a knocked-down opponent is expected to be free on,
    /// while he is down.
    pub(super) fn rise_at(&self) -> Option<u32> {
        self.foe_down.then(|| self.foe_down_since + u32::from(self.rise_frames))
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
    /// A route from the fighter's route book that starts this way, entered
    /// whole from its opener: its first cancel is committed before any hit
    /// is seen. With no route allowed, the plan ends.
    Route(Reach),
    /// Against a knocked-down opponent, walk in while he is down and press
    /// this normal so it is out as he rises, from the knockdown's length as
    /// the CPU has seen it. Out of its reach when the moment comes, nothing
    /// is pressed (the outcome is `Nothing`).
    Meaty { crouch: bool, button: usize },
    /// Walk in while he is down, then hold a guard over the opponent's
    /// rise, so a reversal meets it and can be punished.
    GuardRise,
}

impl Act {
    /// The step commits it to walking in or attacking.
    pub(super) fn commits(self) -> bool {
        matches!(
            self,
            Act::Reach { .. }
                | Act::Route(_)
                | Act::Meaty { .. }
                | Act::Do(Choice::Normal { .. } | Choice::Throw | Choice::Special(_) | Choice::Approach)
        )
    }

    /// The step waits for a knocked-down opponent's rise, or walks in to
    /// meet it: it is no waste against him while he is down.
    pub(super) fn meets_the_rise(self) -> bool {
        matches!(self, Act::Meaty { .. } | Act::GuardRise | Act::Reach { .. })
    }
}

/// Frames the CPU may walk in to start a route: from neutral, and to
/// punish a whiff, when the recovering opponent will not wait long.
pub(super) const NEUTRAL_WALK: u8 = 20;
pub(super) const PUNISH_WALK: u8 = 6;
/// Frames before a falling opponent lands at which a punish begins: about
/// the time a route's opener takes to reach him.
pub(super) const LANDING_LEAD: i16 = 6;

/// A walk covers about two pixels a frame.
const WALK_SPEED: i16 = 2;

/// What the moment allows when the CPU looks for a route.
pub(super) struct Moment<'a> {
    pub reaches: &'a [Reach],
    /// An era style is in force: it brings true combos and cancels with it.
    pub era: bool,
    pub true_combos: bool,
    pub cancels: bool,
    pub crouching: bool,
    pub meters: [u16; 2],
    pub corner: bool,
    pub gap: i16,
    pub walk: u8,
    /// In health bar points.
    pub foe_health: u16,
}

impl Moment<'_> {
    /// The route can be taken now.
    pub(super) fn allows(&self, route: &Route) -> bool {
        let stance = if self.crouching { Stance::Crouching } else { Stance::Standing };
        self.reaches.contains(&route.reach)
            && (!route.true_combos || self.true_combos || self.era)
            && (!route.cancels || self.cancels || self.era)
            && route.stances.contains(&stance)
            && (!route.corner || self.corner)
            && route.super_spent <= self.meters[0]
            && route.drive_spent <= self.meters[1]
            && self.gap <= route.gap + i16::from(self.walk) * WALK_SPEED
    }

    /// Frames to walk before the route reaches from here.
    pub(super) fn walk_frames(&self, route: &Route) -> u8 {
        ((self.gap - route.gap).max(0) + WALK_SPEED - 1).div_euclid(WALK_SPEED).min(i16::from(self.walk)) as u8
    }

    /// How good the route is now: one that wins the round above all;
    /// otherwise its damage, a knockdown at its end, and meter kept; a
    /// route taken lately scores lower.
    pub(super) fn score(&self, route: &Route, recent: bool) -> i32 {
        let points = i32::from(route.damage / 20);
        if points >= i32::from(self.foe_health) && self.foe_health > 0 {
            return 10_000 + points;
        }
        let meter = i32::from(route.super_spent) / 4 + i32::from(route.drive_spent) / 8;
        points * 4 + if route.knockdown { 12 } else { 0 } - meter - if recent { 30 } else { 0 }
    }
}

/// A route being entered: which one, where it is in its script, where it
/// stops (a slip cuts it short), and what the CPU has seen of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RouteRun {
    fighter: TournamentFighter,
    revision: crate::era::Revision,
    loadout: crate::era::Loadout,
    pub index: u8,
    walk: u8,
    pub at: u16,
    stop: u16,
    facing_right: bool,
    hit: bool,
    guarded: bool,
}

impl RouteRun {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        fighter: TournamentFighter,
        revision: crate::era::Revision,
        loadout: crate::era::Loadout,
        index: u8,
        walk: u8,
        stop: u16,
        facing_right: bool,
    ) -> Self {
        RouteRun { fighter, revision, loadout, index, walk, at: 0, stop, facing_right, hit: false, guarded: false }
    }

    fn route(&self) -> Option<&'static Route> {
        crate::route_book::book(self.fighter, self.revision, self.loadout)?.routes.get(usize::from(self.index))
    }

    /// The pad for this frame of the run, the walk first: `None` when the
    /// route is over or cut.
    pub(super) fn pad(&self) -> Option<u16> {
        let forward = if self.facing_right { super::RIGHT } else { super::LEFT };
        if self.at < u16::from(self.walk) {
            return Some(forward);
        }
        let frame = self.at - u16::from(self.walk);
        if frame >= self.stop {
            return None;
        }
        let (stick, buttons) = self.route()?.pad_at(usize::from(frame))?;
        // Routes are written for an attacker facing right.
        let stick = u16::from(stick);
        let stick = if self.facing_right { stick } else { (stick & !3) | ((stick & 1) << 1) | ((stick & 2) >> 1) };
        Some(stick | u16::from(buttons) << 4)
    }

    /// Note what the frame shows of the route.
    pub(super) fn watch(&mut self, view: Percept) {
        self.hit |= view.foe.reacting();
        self.guarded |= view.foe.guarding() && view.me.attacking();
    }

    pub(super) fn outcome(&self) -> Outcome {
        if self.hit {
            Outcome::Hit
        } else if self.guarded {
            Outcome::Guard
        } else {
            Outcome::Whiff
        }
    }
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
    /// Added to the score while the opponent is cornered.
    pub corner_bonus: i32,
    /// Taken up only while the opponent is cornered.
    pub cornered_only: bool,
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
/// range and press it. On a hit (it knocks down in this engine) follow him
/// in and time the same kick to his rise; on a guard walk in and throw; on
/// a whiff give ground and guard.
const RYU_SF6_POKE: [Step; 6] = [
    step(Act::Reach { crouch: true, button: 4 }, always(Some(1))),
    step(Act::Do(Choice::Normal { crouch: true, button: 4 }), [Some(2), Some(3), Some(4), None, None]),
    step(Act::Meaty { crouch: true, button: 4 }, always(None)),
    step(Act::Do(Choice::Throw), always(None)),
    step(Act::Do(Choice::Retreat), always(Some(5))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// Ryu in the SF6 era style, close neutral: walk in and enter a ground
/// route from the book. On a guard walk in and throw; on a whiff give
/// ground and guard.
const RYU_SF6_ROUTE: [Step; 4] = [
    step(Act::Route(Reach::Ground), [None, Some(1), Some(2), None, None]),
    step(Act::Do(Choice::Throw), always(None)),
    step(Act::Do(Choice::Retreat), always(Some(3))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// Ryu in the SF6 era style, after an attack was guarded: a short wait,
/// then a medium punch that hits him if he pressed a button in the gap. On
/// a guard again the pressure has run out: give ground and guard.
const RYU_SF6_FRAME_TRAP: [Step; 4] = [
    step(Act::Do(Choice::Wait), always(Some(1))),
    step(Act::Do(Choice::Normal { crouch: false, button: 1 }), [None, Some(2), Some(2), None, None]),
    step(Act::Do(Choice::Retreat), always(Some(3))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// After an attack was guarded: walk in and throw.
const RYU_SF6_WALK_THROW: [Step; 1] = [step(Act::Do(Choice::Throw), always(None))];

/// Against a knocked-down opponent: walk to crouching medium kick range
/// and time it to meet his rise. On a guard throw; on a whiff back off.
const RYU_SF6_MEATY: [Step; 4] = [
    step(Act::Meaty { crouch: true, button: 4 }, [None, Some(1), Some(2), None, None]),
    step(Act::Do(Choice::Throw), always(None)),
    step(Act::Do(Choice::Retreat), always(Some(3))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// Against a knocked-down opponent: walk in close and guard over his rise,
/// so a reversal is guarded and its whiff punished.
const RYU_SF6_BAIT: [Step; 1] = [step(Act::GuardRise, always(None))];

const RYU_SF6: [GamePlan; 7] = [
    GamePlan { situation: Situation::NeutralClose, score: 60, corner_bonus: 0, cornered_only: false, steps: &RYU_SF6_POKE },
    GamePlan { situation: Situation::NeutralClose, score: 62, corner_bonus: 6, cornered_only: false, steps: &RYU_SF6_ROUTE },
    GamePlan { situation: Situation::ItWasBlocked, score: 60, corner_bonus: 4, cornered_only: false, steps: &RYU_SF6_FRAME_TRAP },
    GamePlan { situation: Situation::ItWasBlocked, score: 56, corner_bonus: 12, cornered_only: false, steps: &RYU_SF6_WALK_THROW },
    GamePlan { situation: Situation::OpponentKnockedDown, score: 62, corner_bonus: 8, cornered_only: false, steps: &RYU_SF6_MEATY },
    GamePlan { situation: Situation::OpponentKnockedDown, score: 58, corner_bonus: -10, cornered_only: false, steps: &RYU_SF6_BAIT },
    // A cornered opponent out of reach (its own guarded attacks push it
    // back off him): walk him down rather than let him out.
    GamePlan { situation: Situation::NeutralFar, score: 60, corner_bonus: 0, cornered_only: true, steps: &RYU_SF6_POKE },
];

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
    /// The frame a timed step's walk in, before it acts, will last be seen
    /// on (0 for no walk yet).
    pub walked: u32,
    /// The machine step the plan began on.
    pub began: u32,
}

impl Running {
    /// The gap to the opponent now, as far as a timed step's walk in can
    /// tell: the CPU sees its own steps `reaction` frames late, so those it
    /// has walked since the frame seen are counted at a walk's pace.
    pub(super) fn gap_now(&self, view: Percept) -> i16 {
        let unseen = self.walked.saturating_sub(view.frame).min(60) as i16;
        view.gap() - unseen * WALK_SPEED
    }

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
crate::canonical::canonical_struct!(Judged, "FairCpuJudged.v2", { situation, corner, foe_cornered });
crate::canonical::canonical_struct!(Judge, "FairCpuJudge.v3", {
    foe_attack_shown, foe_attack_met, foe_attack_grounded, foe_reach, blocked, foe_down, foe_down_since, rise_frames,
});
crate::canonical::canonical_struct!(Running, "FairCpuGamePlan.v2", { plan, step, acted, pressed, walked, began });
crate::canonical::canonical_struct!(RouteRun, "FairCpuRouteRun.v1", {
    fighter, revision, loadout, index, walk, at, stop, facing_right, hit, guarded,
});
crate::canonical::canonical_enum!(Outcome, "FairCpuOutcome.v1"; Hit => 0, Guard => 1, Whiff => 2, Jumped => 3, Nothing => 4);

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
        changed[3].game = Some(Running { plan: 0, step: 1, acted: true, pressed: 9, walked: 0, began: 4 });
        for cpu in &changed {
            assert_ne!(hash(cpu), hash(&plain), "{cpu:?}");
        }
        assert_ne!(hash(&plain.force_stronger(true)), hash(&plain));
    }

    fn ryu_sf6() -> &'static crate::route_book::RouteBook {
        crate::route_book::book(TournamentFighter::Ryu, crate::era::Revision::Sf6, crate::era::Loadout::Standard).unwrap()
    }

    fn moment(reaches: &[Reach]) -> Moment<'_> {
        Moment {
            reaches,
            era: true,
            true_combos: false,
            cancels: false,
            crouching: false,
            meters: [300, 60_000],
            corner: false,
            gap: 20,
            walk: 0,
            foe_health: 48,
        }
    }

    #[test]
    fn only_routes_the_rules_the_stance_the_meter_and_the_distance_allow_are_taken() {
        let routes = &ryu_sf6().routes;
        let all = [Reach::Ground, Reach::Punish];
        let open = moment(&all);
        assert!(routes.iter().all(|r| open.allows(r)), "with everything in its favour every route is allowed");
        let only = |m: &Moment, keep: &dyn Fn(&Route) -> bool| routes.iter().all(|r| m.allows(r) == keep(r));
        assert!(only(&moment(&[Reach::Ground]), &|r| r.reach == Reach::Ground));
        assert!(only(&Moment { era: false, ..moment(&all) }, &|r| !r.true_combos && !r.cancels));
        assert!(only(&Moment { crouching: true, ..moment(&all) }, &|r| r.stances.contains(&Stance::Crouching)));
        assert!(only(&Moment { meters: [0, 60_000], ..moment(&all) }, &|r| r.super_spent == 0));
        assert!(only(&Moment { gap: 50, ..moment(&all) }, &|r| r.gap >= 50));
        assert!(only(&Moment { gap: 50, walk: 3, ..moment(&all) }, &|r| r.gap >= 44));
        assert!(routes.iter().any(|r| !r.stances.contains(&Stance::Crouching)) && routes.iter().any(|r| r.super_spent > 0));
    }

    #[test]
    fn a_route_that_wins_the_round_scores_above_any_that_does_not_and_a_recent_one_lower() {
        let routes = &ryu_sf6().routes;
        let weakest = routes.iter().min_by_key(|r| r.damage).unwrap();
        let strongest = routes.iter().max_by_key(|r| r.damage).unwrap();
        // The opponent has just less health than the weakest route does.
        let low = Moment { foe_health: weakest.damage / 20, ..moment(&[Reach::Ground]) };
        assert!(low.score(weakest, true) > moment(&[Reach::Ground]).score(strongest, false));
        let full = moment(&[Reach::Ground]);
        assert!(full.score(strongest, true) < full.score(strongest, false));
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
