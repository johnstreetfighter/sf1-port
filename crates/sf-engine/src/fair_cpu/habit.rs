//! Habit memory (SC9, `#201`; spec `#192`, ADR-0026): what the stronger
//! Fair CPU has noted during a match about what its opponent does in
//! recurring spots, and how it leans on that.
//!
//! **What it notes.** From the frames its reaction delay shows it (the same
//! view it decides from, [`super::FairCpu::shown`]), never an input: what the
//! opponent did
//! - after its projectile: jumped, guarded, answered with a projectile of
//!   his own, or none of these ([`ShotAnswer`]);
//! - rising from a knockdown: guarded, a reversal (a special or a super),
//!   a button, a throw, backing off, or none of these ([`RiseAnswer`]);
//! - after guarding its attack: pressed a button, jumped, threw, or kept
//!   guarding ([`GuardAnswer`]);
//! - when grabbed: teched or not ([`GrabAnswer`]);
//! - jumping in: how often, and from how far ([`JumpFrom`]).
//!
//! **Fading.** Each sighting in a spot adds [`SIGHTING`] to its answer after
//! every count of that spot has kept three quarters of itself, so the last
//! few sightings outweigh older ones and a change of habit is noticed within
//! a few. Jump-ins are not alternatives to each other: their counts fade
//! with time instead ([`JUMP_FADE`]), so their total says how often he jumps
//! in lately.
//!
//! **What it changes.** Game plan scores ([`Lean`], [`Habits::plan_bonus`]),
//! the throw's score in neutral ([`Habits::throw_bonus`]), the anti-air's
//! score against a jump it expects, and its focus: an opponent who jumps its
//! projectile gets a CPU that throws one and watches for the jump. How much
//! is a level value ([`uses`]): all of it at Very Difficult, less below, and
//! none at Easy, which keeps noting but never leans on it.
//!
//! **One match.** The memory lasts a match, all its rounds, and starts empty
//! at the next, in every mode ([`super::FairCpu::press`]). It lives in
//! [`super::FairCpu`], so snapshots and rollback carry it.

use super::game::knocked_down;
use super::{Focus, Level, Percept, Situation};
use crate::combo::throw::{THROWN_MOVE, THROW_MOVE};

/// What one sighting adds to its answer's count.
pub const SIGHTING: u16 = 64;
/// The weight a spot's counts must reach before anything is expected there:
/// two sightings at least (64, then 48 + 64).
const SURE: u16 = 100;
/// The share of a spot's weight, in percent, one answer needs to be
/// expected at Very Difficult; lower levels need more ([`expected`]).
const MOST: u16 = 60;
/// View frames after its projectile starts within which his answer to it
/// is looked for, and in which an expected jump over it is watched for.
pub const SHOT_WATCH: u32 = 60;
/// View frames after he is seen free of a knockdown within which his first
/// action counts as his wake-up.
pub const RISE_WATCH: u32 = 24;
/// View frames after its attack is seen guarded within which his first
/// action counts as his answer to its pressure.
pub const GUARD_WATCH: u32 = 40;
/// View frames after he is seen grabbed by which a tech has shown.
pub const GRAB_WATCH: u32 = 20;
/// Pixels a jump must carry him from where it began for its way to count,
/// and view frames past the watch it may take: his drawing's centre
/// shifts a few pixels as his pose changes.
const JUMP_WAY_PIXELS: i16 = 8;
const JUMP_WAY: u32 = 20;
/// View frames between fadings of the jump-in counts.
pub const JUMP_FADE: u32 = 300;
/// Jump-in weight, scaled by how much the level uses the memory, from
/// which he counts as jumping in often: about one jump every few seconds.
const OFTEN: u16 = 160;
/// Gaps (from the CPU's fighter to his hurtbox) dividing near, middle and
/// far jump-ins.
const JUMP_NEAR: i16 = 80;
const JUMP_FAR: i16 = 150;
/// The share a jump-in distance needs for jumps from there to be expected.
const JUMP_FROM: u16 = 34;

/// What he did after its projectile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotAnswer {
    Jumped,
    Guarded,
    Shot,
    Other,
}

/// What he did rising from a knockdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiseAnswer {
    Guarded,
    Reversal,
    Button,
    Throw,
    BackOff,
    Other,
}

/// What he did after guarding its attack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardAnswer {
    Button,
    Jumped,
    Throw,
    KeptGuarding,
}

/// Whether he teched its throw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrabAnswer {
    Teched,
    NotTeched,
}

/// How far off he jumped in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpFrom {
    Near,
    Mid,
    Far,
}

impl JumpFrom {
    fn of(gap: i16) -> JumpFrom {
        if gap < JUMP_NEAR {
            JumpFrom::Near
        } else if gap < JUMP_FAR {
            JumpFrom::Mid
        } else {
            JumpFrom::Far
        }
    }
}

/// What a game plan leans on, for the habits that make it better or worse
/// against this opponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lean {
    /// Nothing the memory weighs.
    None,
    /// A projectile, then walking in behind it.
    Shot,
    /// A projectile, then a dash in behind it.
    ShotRush,
    /// A projectile, then standing its ground to anti-air a jump over it.
    ShotWatch,
    /// A jump in from afar.
    JumpIn,
    /// An attack timed to his rise.
    Meaty,
    /// A guard held over his rise, to punish a reversal.
    Bait,
    /// A short wait after a guarded attack, then another attack.
    FrameTrap,
    /// A walk in and a throw after a guarded attack.
    WalkThrow,
}

/// The share of the memory a level leans on, in percent: Easy notes but
/// never uses it.
pub fn uses(level: Level) -> u16 {
    match level {
        Level::Easy => 0,
        Level::Normal => 50,
        Level::Hard => 75,
        Level::VeryHard => 100,
    }
}

/// The stronger CPU's notes on its opponent for this match.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Habits {
    after_shot: [u16; 4],
    rising: [u16; 6],
    after_guard: [u16; 4],
    grabbed: [u16; 2],
    jump_in: [u16; 3],
    /// Sightings noted this match, all spots together.
    sightings: u16,
    /// The view frame each open spot opened on (0 for none): its
    /// projectile, his rise, its attack guarded, his grab.
    shot_at: u32,
    rise_at: u32,
    guard_at: u32,
    grab_at: u32,
    /// The view frame its last projectile started on, and where he stood
    /// as a jump in its watch began, until its way shows.
    last_shot: u32,
    shot_jump_from: Option<i16>,
    /// The jump being drawn has been noted, and the view frame the jump-in
    /// counts last faded on.
    jump_noted: bool,
    jump_faded: u32,
}

crate::canonical::canonical_struct!(Habits, "FairCpuHabits.v1", {
    after_shot, rising, after_guard, grabbed, jump_in, sightings, shot_at, rise_at, guard_at, grab_at, last_shot,
    shot_jump_from, jump_noted, jump_faded,
});

/// One sighting of `answer` in a spot: every count keeps three quarters of
/// itself, then the answer's gains one sighting.
fn note(tally: &mut [u16], answer: usize) {
    for count in tally.iter_mut() {
        *count = *count * 3 / 4;
    }
    tally[answer] = tally[answer].saturating_add(SIGHTING);
}

fn total(tally: &[u16]) -> u16 {
    tally.iter().fold(0u16, |sum, &count| sum.saturating_add(count))
}

/// `answer`'s share of the spot's weight, in percent.
fn share(tally: &[u16], answer: usize) -> u16 {
    let all = u32::from(total(tally));
    if all == 0 { 0 } else { (u32::from(tally[answer]) * 100 / all) as u16 }
}

/// The level leans on `answer` being what he does in this spot: enough
/// sightings, and a share of them that the level finds sure enough (60% at
/// Very Difficult, 72% at Difficult, 85% at Normal; never at Easy).
fn expected(tally: &[u16], answer: usize, level: Level) -> bool {
    let using = uses(level);
    using > 0 && total(tally) >= SURE && share(tally, answer) >= MOST + (100 - using) / 2
}

/// A spot of the memory, for reading it from outside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    AfterShot,
    Rising,
    AfterGuard,
    Grabbed,
    JumpIn,
}

impl Habits {
    fn tally(&self, spot: Spot) -> &[u16] {
        match spot {
            Spot::AfterShot => &self.after_shot,
            Spot::Rising => &self.rising,
            Spot::AfterGuard => &self.after_guard,
            Spot::Grabbed => &self.grabbed,
            Spot::JumpIn => &self.jump_in,
        }
    }

    /// Sightings noted this match, all spots together.
    pub fn sightings(&self) -> u16 {
        self.sightings
    }

    /// The weight of a spot's counts, faded as they are.
    pub fn weight(&self, spot: Spot) -> u16 {
        total(self.tally(spot))
    }

    /// An answer's share of its spot's weight, in percent (answers numbered
    /// as their enum lists them).
    pub fn share(&self, spot: Spot, answer: usize) -> u16 {
        share(self.tally(spot), answer)
    }

    /// Nothing noted yet.
    pub fn is_empty(&self) -> bool {
        self.sightings == 0
    }

    pub(super) fn record(&mut self, spot: Spot, answer: usize) {
        match spot {
            Spot::AfterShot => note(&mut self.after_shot, answer),
            Spot::Rising => note(&mut self.rising, answer),
            Spot::AfterGuard => note(&mut self.after_guard, answer),
            Spot::Grabbed => note(&mut self.grabbed, answer),
            Spot::JumpIn => self.jump_in[answer] = self.jump_in[answer].saturating_add(SIGHTING),
        }
        self.sightings = self.sightings.saturating_add(1);
    }

    /// Note what `view`, the frame after `before`, shows of the spots.
    /// `shots` are the move ids its own fighter throws a projectile in;
    /// `blocked` is whether its attack has just been judged guarded.
    pub(super) fn watch(&mut self, view: &Percept, before: &Percept, shots: &[Option<u8>], blocked: bool) {
        let frame = view.frame;
        let (foe, was) = (view.foe, before.foe);
        if self.jump_faded == 0 || frame < self.jump_faded {
            self.jump_faded = frame;
        } else if frame >= self.jump_faded + JUMP_FADE {
            for count in &mut self.jump_in {
                *count = *count * 3 / 4;
            }
            self.jump_faded = frame;
        }
        let jumped = Focus::Jump.shows(view, before, 0);
        let attack_started =
            foe.attacking() && !(was.jumping() && foe.jumping()) && (!was.attacking() || foe.move_id != was.move_id);
        let toward_me = if view.me.x >= foe.x { 1 } else { -1 };
        let moved_away = (foe.x - was.x) * toward_me < 0;

        // After its projectile: the first of a jump at it, a projectile of
        // his, a guard; hit by it, or nothing of these in time, is none. A
        // jump's way shows once he has moved in the air: a jump back or
        // straight up is no jump at it.
        if self.shot_at != 0 {
            let answer = if let Some(from) = self.shot_jump_from {
                let moved = (foe.x - from) * toward_me;
                if !foe.jumping() || frame > self.shot_at + SHOT_WATCH + JUMP_WAY {
                    // Over before its way showed in full (its projectile hit
                    // him in the air): judged by what it did show.
                    Some(if moved > 0 { ShotAnswer::Jumped } else { ShotAnswer::Other })
                } else if moved.abs() >= JUMP_WAY_PIXELS {
                    Some(if moved > 0 { ShotAnswer::Jumped } else { ShotAnswer::Other })
                } else {
                    None
                }
            } else if jumped {
                self.shot_jump_from = Some(foe.x);
                None
            } else if view.shot && !before.shot {
                Some(ShotAnswer::Shot)
            } else if foe.guarding() {
                Some(ShotAnswer::Guarded)
            } else if foe.reacting() || frame > self.shot_at + SHOT_WATCH {
                Some(ShotAnswer::Other)
            } else {
                None
            };
            if let Some(answer) = answer {
                self.record(Spot::AfterShot, answer as usize);
                self.shot_at = 0;
                self.shot_jump_from = None;
            }
        }
        if shots.contains(&Some(view.me.move_id)) && before.me.move_id != view.me.move_id {
            self.shot_at = frame;
            self.shot_jump_from = None;
            self.last_shot = frame;
        }

        // Rising: his first action once free of the knockdown. Hit as he
        // rises shows nothing of what he chose.
        if knocked_down(&was) && !knocked_down(&foe) && foe.present {
            self.rise_at = frame;
        }
        if self.rise_at != 0 {
            let answer = if attack_started {
                Some(if foe.move_id == THROW_MOVE {
                    RiseAnswer::Throw
                } else if foe.normal() {
                    RiseAnswer::Button
                } else {
                    RiseAnswer::Reversal
                })
            } else if foe.guarding() {
                Some(RiseAnswer::Guarded)
            } else if (foe.jumping() || matches!(foe.move_id, 2 | 3)) && moved_away {
                Some(RiseAnswer::BackOff)
            } else if frame > self.rise_at + RISE_WATCH {
                Some(RiseAnswer::Other)
            } else {
                None
            };
            if let Some(answer) = answer {
                self.record(Spot::Rising, answer as usize);
                self.rise_at = 0;
            } else if foe.reacting() || knocked_down(&foe) {
                self.rise_at = 0;
            }
        }

        // After guarding its attack: his first action, or still guarding
        // when the watch ends (thrown out of his guard included). Hit
        // shows nothing of what he chose.
        if self.guard_at != 0 {
            let answer = if attack_started {
                Some(if foe.move_id == THROW_MOVE { GuardAnswer::Throw } else { GuardAnswer::Button })
            } else if jumped {
                Some(GuardAnswer::Jumped)
            } else if foe.move_id == THROWN_MOVE || frame > self.guard_at + GUARD_WATCH {
                Some(GuardAnswer::KeptGuarding)
            } else {
                None
            };
            if let Some(answer) = answer {
                self.record(Spot::AfterGuard, answer as usize);
                self.guard_at = 0;
            } else if foe.reacting() {
                self.guard_at = 0;
            }
        }
        if blocked && self.guard_at == 0 {
            self.guard_at = frame;
        }

        // Grabbed: free again at once is a tech; thrown on is none.
        if foe.move_id == THROWN_MOVE && was.move_id != THROWN_MOVE && view.me.move_id == THROW_MOVE {
            self.grab_at = frame;
        } else if self.grab_at != 0 {
            let answer = if foe.move_id != THROWN_MOVE {
                Some(if foe.free() && !knocked_down(&foe) { GrabAnswer::Teched } else { GrabAnswer::NotTeched })
            } else if frame > self.grab_at + GRAB_WATCH {
                Some(GrabAnswer::NotTeched)
            } else {
                None
            };
            if let Some(answer) = answer {
                self.record(Spot::Grabbed, answer as usize);
                self.grab_at = 0;
            }
        }

        // Jumping in: once a jump is seen to carry him toward it, from how
        // far it began. A special that rises (a Shoryuken) is airborne too,
        // but no jump.
        if !foe.jumping() {
            self.jump_noted = false;
        } else if !self.jump_noted && matches!(foe.move_id, 9..=0xb) && foe.x != was.x {
            self.jump_noted = true;
            if !moved_away {
                self.record(Spot::JumpIn, JumpFrom::of(before.gap()) as usize);
            }
        }
    }

    /// It expects a jump at it now, as far as the level uses the memory:
    /// over its projectile thrown lately, if that is what he does, or from
    /// this far off, if he jumps in often from here.
    pub(super) fn jump_expected(&self, frame: u32, gap: i16, level: Level) -> bool {
        let after_shot = self.last_shot != 0 && frame <= self.last_shot + SHOT_WATCH;
        (after_shot && expected(&self.after_shot, ShotAnswer::Jumped as usize, level)) || self.jumps_in_often(gap, level)
    }

    fn jumps_in_often(&self, gap: i16, level: Level) -> bool {
        let using = u32::from(uses(level));
        using > 0
            && u32::from(total(&self.jump_in)) * using / 100 >= u32::from(OFTEN)
            && share(&self.jump_in, JumpFrom::of(gap) as usize) >= JUMP_FROM
    }

    /// The focus it holds instead of its usual one, where the memory says a
    /// jump is coming and the fight is in neutral.
    pub(super) fn focus(&self, frame: u32, situation: Situation, gap: i16, level: Level) -> Option<Focus> {
        let neutral = matches!(situation, Situation::NeutralFar | Situation::NeutralClose);
        (neutral && self.jump_expected(frame, gap, level)).then_some(Focus::Jump)
    }

    /// What the memory adds to a game plan's score, as far as the level
    /// uses it.
    pub(super) fn plan_bonus(&self, lean: Lean, level: Level) -> i32 {
        let is = |tally: &[u16], answer: usize| expected(tally, answer, level);
        let jumps_shot = is(&self.after_shot, ShotAnswer::Jumped as usize);
        let bonus = match lean {
            Lean::None => 0,
            // He jumps it: throw fewer, and only from where his jump over
            // it can be met, then stand and watch for the jump to anti-air
            // it (the maintainer, 2026-11-20). From farther he lands short;
            // walking in behind it walks under him.
            Lean::ShotWatch if jumps_shot => 85,
            Lean::ShotWatch => 0,
            Lean::Shot if jumps_shot => -60,

            Lean::Shot if is(&self.after_shot, ShotAnswer::Shot as usize) => -10,
            Lean::Shot => 0,
            // A dash in under his jump is hit; against a guard it presses.
            Lean::ShotRush if jumps_shot => -60,
            Lean::ShotRush if is(&self.after_shot, ShotAnswer::Guarded as usize) => 10,
            Lean::ShotRush => 0,
            // Over his projectile, answering it.
            Lean::JumpIn if is(&self.after_shot, ShotAnswer::Shot as usize) => 20,
            Lean::JumpIn => 0,
            // An attack on his rise beats a button, a throw or a slow
            // back-off; it loses to a reversal, which a held guard baits.
            Lean::Meaty if is(&self.rising, RiseAnswer::Reversal as usize) => -15,
            Lean::Meaty
                if [RiseAnswer::Button, RiseAnswer::Throw, RiseAnswer::BackOff]
                    .iter()
                    .any(|&a| is(&self.rising, a as usize)) =>
            {
                15
            }
            Lean::Meaty => 0,
            Lean::Bait if is(&self.rising, RiseAnswer::Reversal as usize) => 25,
            Lean::Bait if is(&self.rising, RiseAnswer::Button as usize) || is(&self.rising, RiseAnswer::Throw as usize) => -10,
            Lean::Bait => 0,
            // After a guard: a frame trap catches a button, a throw or a
            // jump in its gap; a throw takes one who keeps guarding.
            Lean::FrameTrap
                if [GuardAnswer::Button, GuardAnswer::Throw, GuardAnswer::Jumped]
                    .iter()
                    .any(|&a| is(&self.after_guard, a as usize)) =>
            {
                15
            }
            Lean::FrameTrap if is(&self.after_guard, GuardAnswer::KeptGuarding as usize) => -10,
            Lean::FrameTrap => 0,
            Lean::WalkThrow => {
                let guards = if is(&self.after_guard, GuardAnswer::KeptGuarding as usize) {
                    15
                } else if is(&self.after_guard, GuardAnswer::Button as usize) {
                    -15
                } else {
                    0
                };
                guards + self.grab_lean(level)
            }
        };
        bonus * i32::from(uses(level)) / 100
    }

    /// What the memory adds to a throw's score: more against an opponent who
    /// does not tech, less against one who does.
    /// What the memory adds to a projectile chosen on its own (outside a
    /// game plan) from `gap`: against an opponent who jumps it, one thrown
    /// from outside `watched` (the gaps his jump over it can be met from)
    /// is worth much less.
    pub(super) fn shot_bonus(&self, gap: i16, watched: (i16, i16), level: Level) -> i32 {
        let jumps = expected(&self.after_shot, ShotAnswer::Jumped as usize, level);
        let bonus = if jumps && !(watched.0..=watched.1).contains(&gap) { -40 } else { 0 };
        bonus * i32::from(uses(level)) / 100
    }

    pub(super) fn throw_bonus(&self, level: Level) -> i32 {
        self.grab_lean(level) * i32::from(uses(level)) / 100
    }

    fn grab_lean(&self, level: Level) -> i32 {
        if expected(&self.grabbed, GrabAnswer::NotTeched as usize, level) {
            20
        } else if expected(&self.grabbed, GrabAnswer::Teched as usize, level) {
            -20
        } else {
            0
        }
    }

    /// What the memory adds to the anti-air's score against a jump it
    /// expects.
    pub(super) fn anti_air_bonus(&self, frame: u32, gap: i16, level: Level) -> i32 {
        if self.jump_expected(frame, gap, level) { 30 * i32::from(uses(level)) / 100 } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fair_cpu::Body;

    const LEVELS: [Level; 4] = [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard];

    fn noted(spot: Spot, answers: &[usize]) -> Habits {
        let mut habits = Habits::default();
        for &answer in answers {
            habits.record(spot, answer);
        }
        habits
    }

    #[test]
    fn counts_fade_so_a_changed_habit_is_noticed() {
        // Ten jumps over its projectile, then he starts guarding it.
        let jumped = ShotAnswer::Jumped as usize;
        let guarded = ShotAnswer::Guarded as usize;
        let mut answers = vec![jumped; 10];
        let mut habits = noted(Spot::AfterShot, &answers);
        assert!(expected(&habits.after_shot, jumped, Level::VeryHard));
        // Within four guards the jump is no longer expected, and the guard
        // is; it was never forgotten that he jumped.
        for n in 1..=4 {
            answers.push(guarded);
            habits = noted(Spot::AfterShot, &answers);
            eprintln!("after {n} guards: jumped {}%, guarded {}%", habits.share(Spot::AfterShot, jumped), habits.share(Spot::AfterShot, guarded));
        }
        assert!(!expected(&habits.after_shot, jumped, Level::VeryHard), "the old habit still expected");
        assert!(expected(&habits.after_shot, guarded, Level::VeryHard), "the new habit not noticed");
        assert!(habits.share(Spot::AfterShot, jumped) > 0);
        // Without fading, ten old sightings would outweigh four new ones.
        assert!(habits.share(Spot::AfterShot, guarded) > 50);
    }

    #[test]
    fn jump_ins_fade_with_time() {
        let mut habits = Habits::default();
        let mut frame = 100;
        let mut view = Percept { frame, fighting: true, ..Percept::default() };
        view.me = Body { present: true, x: 300, hurt: (290, 310), ..Body::default() };
        view.foe = Body { present: true, x: 200, hurt: (190, 210), ..Body::default() };
        let mut step = |habits: &mut Habits, foe_move: u8, dx: i16| {
            let before = view;
            frame += 1;
            view.frame = frame;
            view.foe.move_id = foe_move;
            view.foe.x += dx;
            view.foe.hurt = (view.foe.x - 10, view.foe.x + 10);
            habits.watch(&view, &before, &[], false);
        };
        // Three jumps toward it from about 80 pixels (a middle jump-in).
        for _ in 0..3 {
            step(&mut habits, 9, 0);
            for _ in 0..20 {
                step(&mut habits, 0xa, 1);
            }
            for _ in 0..20 {
                step(&mut habits, 1, -1);
            }
        }
        let after = habits.weight(Spot::JumpIn);
        assert_eq!(after, 3 * SIGHTING, "three jump-ins noted");
        assert_eq!(habits.share(Spot::JumpIn, JumpFrom::Mid as usize), 100);
        assert!(habits.jumps_in_often(90, Level::VeryHard));
        // A while without jumping: the counts fade, and he no longer
        // counts as jumping in often.
        for _ in 0..(3 * JUMP_FADE) {
            step(&mut habits, 1, 0);
        }
        assert!(habits.weight(Spot::JumpIn) < after / 2, "{} of {after}", habits.weight(Spot::JumpIn));
        assert!(!habits.jumps_in_often(90, Level::VeryHard));
    }

    #[test]
    fn easy_does_not_use_the_memory_and_lower_levels_use_it_less() {
        let habits = noted(Spot::Grabbed, &[GrabAnswer::NotTeched as usize; 6]);
        let jumps = noted(Spot::AfterShot, &[ShotAnswer::Jumped as usize; 6]);
        let bait = noted(Spot::Rising, &[RiseAnswer::Reversal as usize; 6]);
        let throws: Vec<i32> = LEVELS.iter().map(|&l| habits.throw_bonus(l)).collect();
        let shots: Vec<i32> = LEVELS.iter().map(|&l| jumps.plan_bonus(Lean::ShotWatch, l)).collect();
        let baits: Vec<i32> = LEVELS.iter().map(|&l| bait.plan_bonus(Lean::Bait, l)).collect();
        for bonuses in [&throws, &shots, &baits] {
            assert_eq!(bonuses[0], 0, "Easy leans on nothing: {bonuses:?}");
            assert!(bonuses.windows(2).all(|w| w[0] < w[1]), "each level leans on it more: {bonuses:?}");
        }
        assert_eq!(jumps.anti_air_bonus(jumps.last_shot, 100, Level::Easy), 0);
        assert!(!jumps.jump_expected(0, 100, Level::Easy));
        for lean in [Lean::Shot, Lean::ShotRush, Lean::ShotWatch, Lean::JumpIn, Lean::Meaty, Lean::Bait, Lean::FrameTrap, Lean::WalkThrow] {
            for full in [habits, jumps, bait] {
                assert_eq!(full.plan_bonus(lean, Level::Easy), 0, "{lean:?}");
            }
        }
        // One sighting is not a habit at any level.
        let once = noted(Spot::Grabbed, &[GrabAnswer::NotTeched as usize]);
        assert!(LEVELS.iter().all(|&l| once.throw_bonus(l) == 0));
    }

    #[test]
    fn each_habit_shifts_the_plans_that_answer_it() {
        let v = Level::VeryHard;
        let jumps = noted(Spot::AfterShot, &[ShotAnswer::Jumped as usize; 4]);
        assert!(jumps.plan_bonus(Lean::ShotWatch, v) > 0 && jumps.plan_bonus(Lean::Shot, v) < 0 && jumps.plan_bonus(Lean::ShotRush, v) < 0);
        let reversals = noted(Spot::Rising, &[RiseAnswer::Reversal as usize; 4]);
        assert!(reversals.plan_bonus(Lean::Bait, v) > 0 && reversals.plan_bonus(Lean::Meaty, v) < 0);
        let buttons = noted(Spot::Rising, &[RiseAnswer::Button as usize; 4]);
        assert!(buttons.plan_bonus(Lean::Meaty, v) > 0 && buttons.plan_bonus(Lean::Bait, v) < 0);
        let mashes = noted(Spot::AfterGuard, &[GuardAnswer::Button as usize; 4]);
        assert!(mashes.plan_bonus(Lean::FrameTrap, v) > 0 && mashes.plan_bonus(Lean::WalkThrow, v) < 0);
        let turtles = noted(Spot::AfterGuard, &[GuardAnswer::KeptGuarding as usize; 4]);
        assert!(turtles.plan_bonus(Lean::WalkThrow, v) > 0 && turtles.plan_bonus(Lean::FrameTrap, v) < 0);
        let teches = noted(Spot::Grabbed, &[GrabAnswer::Teched as usize; 4]);
        let never = noted(Spot::Grabbed, &[GrabAnswer::NotTeched as usize; 4]);
        assert!(teches.throw_bonus(v) < 0 && never.throw_bonus(v) > 0);
        assert!(jumps.shot_bonus(200, (140, 160), v) < 0 && jumps.shot_bonus(150, (140, 160), v) == 0);
        assert_eq!(jumps.shot_bonus(200, (140, 160), Level::Easy), 0);
        assert_eq!(Habits::default().shot_bonus(200, (140, 160), v), 0);
        assert!(teches.plan_bonus(Lean::WalkThrow, v) < 0 && never.plan_bonus(Lean::WalkThrow, v) > 0);
        assert!(Habits::default().plan_bonus(Lean::Shot, v) == 0 && Habits::default().throw_bonus(v) == 0);
    }

    /// Frames of a fight in which its projectile starts, then he jumps,
    /// moving `dx` pixels a frame toward its fighter (it stands on his right).
    fn after_its_projectile(dx: i16) -> Habits {
        let mut habits = Habits::default();
        let mut view = Percept { frame: 100, fighting: true, ..Percept::default() };
        view.me = Body { present: true, x: 300, hurt: (290, 310), move_id: 1, ..Body::default() };
        view.foe = Body { present: true, x: 150, hurt: (140, 160), move_id: 1, ..Body::default() };
        let mut frames = vec![(0x1a, 1, 0); 4];
        frames.extend([(0x1a, 9, 0); 3]);
        frames.extend([(0x1a, 0xa, dx); 30]);
        frames.extend([(0x1a, 1, 0); 3]);
        for (mine, his, step) in frames {
            let before = view;
            view.frame += 1;
            view.me.move_id = mine;
            view.foe.move_id = his;
            view.foe.x += step;
            habits.watch(&view, &before, &[Some(0x1a)], false);
        }
        habits
    }

    #[test]
    fn only_a_jump_at_it_counts_as_jumping_its_projectile() {
        let jumped = ShotAnswer::Jumped as usize;
        assert_eq!(after_its_projectile(3).share(Spot::AfterShot, jumped), 100, "a jump toward it");
        assert_eq!(after_its_projectile(-3).share(Spot::AfterShot, jumped), 0, "a jump back");
        assert_eq!(after_its_projectile(0).share(Spot::AfterShot, jumped), 0, "a jump straight up");
        assert_eq!(after_its_projectile(0).weight(Spot::AfterShot), SIGHTING, "noted as something else");
    }

    #[test]
    fn a_jump_over_its_projectile_is_expected_only_while_one_is_out() {
        let mut habits = noted(Spot::AfterShot, &[ShotAnswer::Jumped as usize; 4]);
        habits.last_shot = 1_000;
        assert!(habits.jump_expected(1_000 + SHOT_WATCH, 120, Level::VeryHard));
        assert_eq!(habits.focus(1_010, Situation::NeutralFar, 120, Level::VeryHard), Some(Focus::Jump));
        assert!(habits.anti_air_bonus(1_010, 120, Level::VeryHard) > 0);
        assert!(!habits.jump_expected(1_001 + SHOT_WATCH, 120, Level::VeryHard));
        // Over a knocked-down opponent it keeps watching his wake-up.
        assert_eq!(habits.focus(1_010, Situation::OpponentKnockedDown, 120, Level::VeryHard), None);
    }
}
