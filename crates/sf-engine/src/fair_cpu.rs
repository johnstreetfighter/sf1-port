//! The fair CPU (SP21, #264; SP22, #265; design
//! `docs/design/2026-11-07-sf1plus-on-the-real-engine-design.md`
//! 5.4): the overhaul's CPU opponent, which plays through Player 2's pad
//! like a person.
//!
//! **What it sees.** Once a frame, after the machine has stepped, the CPU
//! takes a [`Percept`] of what the frame shows: each fighter's place, the way
//! he faces, the move he is drawn in (entity `+0xa`; an attack is an attack
//! move drawn, never the state byte's attack bits alone, which a button held
//! for a jump attack sets early, [`Body::attacking`]; the state byte then
//! says whether it is a standing, crouching or jumping attack),
//! whether his attack box is live and how far forward it reaches, his loose
//! hurtbox, his health bar, and the opponent's projectile nearest it. That
//! is object RAM and the health cells, read and never written. It never
//! reads an input byte, a button latch or a command tracker, the
//! opponent's or its own: the player structures' `+2` input pointers and the
//! latch groups are not touched here.
//!
//! **When it sees it.** The CPU decides each frame from the percept its
//! reaction delay ago ([`Level::reaction`]): the input it gives on step `n`
//! depends only on percepts of frames up to `n - reaction` and on its own
//! memory (what it pressed, its plan and its random state). So something
//! shown on frame `k` can change its inputs on frame `k + reaction` at the
//! soonest, as a person's hand answers what the eye saw. Its own fighter is
//! seen through the same delay. Motion specials given in answer cost a few
//! more frames ([`Level::special_cost`], Infil's ordering: blocks and jumps
//! are the fastest answers, motions the slowest).
//!
//! **How it decides.** Utility scoring, as SF6's card decks and Killer
//! Instinct's ranked choice do (research section 4.3): in neutral it scores
//! walking in or out, waiting, guarding in range, each normal that reaches,
//! its specials and a jump-in, and, when the modern switches are on
//! ([`Context`]), link strings ([`Choice::Combo`], under `true_combos`),
//! normal-to-special cancels ([`Choice::Cancel`], under `cancels`), and
//! close-range throws ([`Choice::Throw`], under `throws`), taking the best
//! with the level's [`Level::quality`] chance, otherwise one of the others.
//! Things it sees start (an attack, a jump, a projectile, a whiffed move to
//! punish, or a close walk-in when `throws` is on) are answered with the
//! level's [`Level::guard`] chance: a guard of the right height, an anti-air,
//! a jump over the projectile, a punish, or a guard with a delayed throw tech.
//!
//! **How it presses.** A plan is a short run of pad frames, given to the
//! machine as Player 2's stick and buttons ([`crate::run::PowerOnMachine`]
//! sets them before the frame, where a pad's would be). The 1987 player
//! code reads them as it reads a person's, so the CPU can only do what a
//! player can, at whichever rules are on. Specials come from the move
//! registry ([`crate::moves::move_list`]), what each is for from its slot
//! record in the fighter's move table ([`SpecialSlot`]), so a fighter whose
//! table and registry entries land gets them here with no change; their
//! motions, link timings and cancels fail with the level's
//! [`Level::execution`] chance, worse under pressure (SF6's "pressure"), and
//! a failed motion comes out as whatever the bad input makes.
//!
//! **What it learns.** Each normal's reach and its projectile's range,
//! from its own attack boxes and shots as it sees them, longer or shorter
//! than its starting guess ([`FairCpu::learn`]).
//!
//! **Determinism.** Its random numbers come from its own generator, seeded
//! from the fight (the frame count, both fighters' places and health, the
//! level) the first time it sees the fight, and it keeps everything in
//! [`FairCpu`], which the machine's snapshot holds. The same inputs give the
//! same fight, RESET POSITIONS included.

mod cast;
mod era;
mod focus;
mod game;
mod habit;
pub(crate) use era::EraControl;
pub use cast::Tool;
pub use focus::Focus;
pub use game::{EraAct, Judged, ResetKind, Situation};
pub use habit::{GrabAnswer, GuardAnswer, Habits, JumpFrom, RiseAnswer, ShotAnswer, Spot};
use era::Tactics;

use crate::config::{CoreToggles, RulesetPreset};
use crate::data_bank::SpecialSlot;
use crate::fight_reader::{player_struct, FIGHTER_SLOTS};
use crate::fighter::{GAME_MODE, WORK_BASE};
use crate::era_fight::EraFighterState;
use crate::moves::{live_move_list, move_list, ButtonClass, Dir, MoveEntry, MoveKind, Recognizer, Strength};
use crate::roster::TournamentFighter;
use sf_video::callouts::Side;

/// The game flow's fight loop (`0x3eafa`): the CPU presses only while a round
/// is being fought.
pub const FIGHT_LOOP: u32 = 0x3eafa;

/// Percepts kept: more than the slowest level's slower delay and the frame
/// before it.
const RING: usize = 40;
/// The longest plan, in pad frames.
const PLAN_CAP: usize = 48;
/// How far inside its catch range, centre to centre, a command throw is
/// entered from (SC15, `#207`): he may drift a little as it starts.
const CATCH_MARGIN: i16 = 4;
/// The SF2 era style's shimmy over a knocked-down opponent (SC15, `#207`):
/// how far past a timed attack's reach a walk in that has stopped against
/// his body still counts as there, the frames it must have stood still to
/// count as stopped, and the frames of each walk back out.
const SHIMMY_BODY: i16 = 12;
const SHIMMY_STUCK: u32 = 4;
const SHIMMY_OUT: usize = 6;
/// A route of this many hits or more is a long one (the combo search's
/// first bar): Easy takes none ([`Level::route_hits`]).
pub const LONG_ROUTE: u8 = 5;
/// Combo resets (SC10, `#202`): the weights of what a cut goes into before
/// the habit memory shifts them; the gap a throw reset walks in to (from
/// its fighter's centre to his hurtbox, inside a throw's reach); and the
/// frames before he is free that a low reset presses its kick, about its
/// start-up, so it is out as he comes out of his hit reaction.
const RESET_THROW: i32 = 40;
const RESET_LOW: i32 = 30;
const RESET_BAIT: i32 = 30;
const RESET_THROW_GAP: i16 = 36;
const RESET_LOW_LEAD: usize = 3;
/// The farthest a bait is held from (the gap as he comes free): farther,
/// his reversal would not reach the guard, nor its punish him.
const BAIT_NEAR: i16 = 80;

/// Pad bits: the stick (right, left, down, up) and the six buttons.
pub const RIGHT: u16 = 1;
pub const LEFT: u16 = 2;
pub const DOWN: u16 = 4;
pub const UP: u16 = 8;
const BUTTON0: u16 = 0x10;
/// All six buttons' pad bits.
const BUTTONS: u16 = 0x3f0;

/// The pad bit of button `b` (0 LP, 1 MP, 2 HP, 3 LK, 4 MK, 5 HK).
const fn button(b: usize) -> u16 {
    BUTTON0 << b
}

/// The stick and buttons of a pad word: (right, left, down, up), then LP to HK.
pub fn pad_parts(pad: u16) -> ([bool; 4], [bool; 6]) {
    (
        [pad & RIGHT != 0, pad & LEFT != 0, pad & DOWN != 0, pad & UP != 0],
        std::array::from_fn(|b| pad & button(b) != 0),
    )
}

/// The Gameplay tab's difficulty as the CPU plays it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Easy,
    Normal,
    Hard,
    VeryHard,
}

pub(crate) fn drive_reserve(level: Level) -> u16 {
    match level {
        Level::Easy | Level::Normal => u16::from(crate::era::STOCK_AT) * 3 / 2,
        Level::Hard | Level::VeryHard => 0,
    }
}

impl Level {
    /// The match settings' `DIFFICULTY` (1 Easy, 2 Normal, 3 Difficult, 4
    /// Very Difficult).
    pub const fn from_difficulty(difficulty: u8) -> Level {
        match difficulty {
            1 => Level::Easy,
            3 => Level::Hard,
            4 => Level::VeryHard,
            _ => Level::Normal,
        }
    }

    /// Frames from a frame being shown to the CPU's inputs answering it:
    /// inside the research's proposed bands (section 5.4: Easy 24 to 30,
    /// Normal 18 to 20, Hard 15 to 16, Very Hard 12 to 13), never below the
    /// corrected human simple reaction time of about 12 to 13 frames.
    pub const fn reaction(self) -> u32 {
        match self {
            Level::Easy => 27,
            Level::Normal => 19,
            Level::Hard => 15,
            Level::VeryHard => 12,
        }
    }

    /// The stronger play's slower delay (SC7, `#199`): frames from a frame
    /// being shown to its inputs answering anything but its focus, which it
    /// answers at [`Level::reaction`].
    pub const fn unfocused_reaction(self) -> u32 {
        match self {
            Level::Easy => 32,
            Level::Normal => 24,
            Level::Hard => 20,
            Level::VeryHard => 17,
        }
    }

    /// How close an opponent walking in must come for it to answer him as
    /// a throw threat: a throw's range, the ground a walk covers while the
    /// CPU's focused delay passes, and a little more.
    const fn throw_engage(self) -> i16 {
        46 + self.reaction() as i16 + 8
    }

    /// Extra frames before a motion special given in answer to something
    /// seen (Infil: motions are the slowest answers).
    pub const fn special_cost(self) -> usize {
        match self {
            Level::Easy => 6,
            Level::Normal => 4,
            Level::Hard => 3,
            Level::VeryHard => 2,
        }
    }

    /// Percent chance that a threat it sees start (an attack, a jump, a
    /// projectile) is answered at all.
    pub const fn guard(self) -> u32 {
        match self {
            Level::Easy => 35,
            Level::Normal => 60,
            Level::Hard => 80,
            Level::VeryHard => 92,
        }
    }

    /// Percent chance it takes the best-scored option (decision quality);
    /// otherwise one of the next best.
    pub const fn quality(self) -> u32 {
        match self {
            Level::Easy => 45,
            Level::Normal => 65,
            Level::Hard => 80,
            Level::VeryHard => 90,
        }
    }

    /// Percent chance that a route with a marked cut point, which could
    /// not win the round on the health his bar shows, is cut there on purpose into a combo reset (SC10,
    /// `#202`), before the habit memory shifts it
    /// ([`habit::Habits::reset_lean`]). Easy never resets.
    pub const fn reset_chance(self) -> u32 {
        match self {
            Level::Easy => 0,
            Level::Normal => 8,
            Level::Hard => 14,
            Level::VeryHard => 20,
        }
    }

    /// Percent chance a special's motion is entered cleanly (execution).
    pub const fn execution(self) -> u32 {
        match self {
            Level::Easy => 70,
            Level::Normal => 85,
            Level::Hard => 93,
            Level::VeryHard => 97,
        }
    }

    /// Percent chance a motion, a link or a route's next step goes in
    /// cleanly: [`Level::execution`], and under pressure (lower on health,
    /// or the opponent close) half as far again from sure.
    pub const fn clean(self, pressure: bool) -> u32 {
        let execution = self.execution();
        if pressure { execution.saturating_sub((100 - execution) / 2) } else { execution }
    }

    /// The stronger play spends meter on its own routes and plans (SC11,
    /// `#203`): every level but Easy.
    pub const fn spends_meter(self) -> bool {
        !matches!(self, Level::Easy)
    }

    /// Easy 0 to Very Difficult 3: the order of the levels.
    pub const fn rank(self) -> u8 {
        match self {
            Level::Easy => 0,
            Level::Normal => 1,
            Level::Hard => 2,
            Level::VeryHard => 3,
        }
    }

    /// The most hits a route the stronger play takes may have (SC11,
    /// `#203`). Easy takes only short ones, never a long route
    /// ([`LONG_ROUTE`] hits or more); Very Difficult any.
    pub const fn route_hits(self) -> u8 {
        match self {
            Level::Easy => 3,
            Level::Normal => 4,
            Level::Hard => 7,
            Level::VeryHard => u8::MAX,
        }
    }

    /// How many pressure plans in a row (after a guarded attack, on a
    /// knocked-down or cornered opponent) the stronger play takes before it
    /// backs off and gives him room (SC11, `#203`). Very Difficult's
    /// `u8::MAX` is no limit: it stays on him for as long as the situation
    /// lasts.
    pub const fn pressure_plans(self) -> u8 {
        match self {
            Level::Easy => 1,
            Level::Normal => 2,
            Level::Hard => 3,
            Level::VeryHard => u8::MAX,
        }
    }

    /// The SF6 Drive gauge the stronger play keeps when it spends from it
    /// on its own initiative (a Drive Rush, a Drive Impact or an OD move in
    /// neutral or a game plan, a route): Easy spends none of it so, and
    /// spends no meter on a route at all (SC11, `#203`). Its answers to
    /// what it sees (a parry, a Drive Reversal, an Impact in answer) keep
    /// [`drive_reserve`], as the old play's do.
    pub const fn drive_kept(self) -> u16 {
        match self {
            Level::Easy => u16::MAX,
            Level::Normal => crate::era::STOCK_AT as u16 * 3 / 2,
            Level::Hard => crate::era::STOCK_AT as u16 / 2,
            Level::VeryHard => 0,
        }
    }

    /// Score for guarding in range before anything is seen (anticipation).
    const fn anticipate(self) -> i32 {
        match self {
            Level::Easy => 6,
            Level::Normal => 24,
            Level::Hard => 38,
            Level::VeryHard => 50,
        }
    }

    /// Score for standing still a moment.
    const fn wait(self) -> i32 {
        match self {
            Level::Easy => 34,
            Level::Normal => 20,
            Level::Hard => 12,
            Level::VeryHard => 8,
        }
    }

    /// Percent of a special's score kept (Easy rarely goes for them, as SF6's
    /// lowest levels do without them).
    const fn specials(self) -> i32 {
        match self {
            Level::Easy => 35,
            _ => 100,
        }
    }
}

/// One fighter as a frame shows him.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Body {
    pub present: bool,
    /// The centre of his drawing, in the playfield's pixels.
    pub x: i16,
    pub y: i16,
    pub facing_left: bool,
    /// The move he is drawn in (entity `+0xa`) and its state byte (`+9`).
    pub move_id: u8,
    pub state: u8,
    /// His attack box is live (`+0xf` bit 7), and its forward edge.
    pub active: bool,
    pub front: i16,
    /// His loose hurtbox's left and right edges.
    pub hurt: (i16, i16),
    /// His health bar: 4 a cell.
    pub health: u16,
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(Body, "Body.v1", {
    present, x, y, facing_left, move_id, state, active, front, hurt, health,
});

crate::canonical::canonical_struct!(Percept, "Percept.v1", {
    frame, fighting, me, foe, shot, shot_x, shot_half, my_shot, my_shot_dist,
});

crate::canonical::canonical_struct!(Plan, "Plan.v2", {
    kind, pads, len, at, answer,
});

crate::canonical::canonical_struct!(Normal, "Normal.v1", {
    crouch, button, pressed, from, best,
});

crate::canonical::canonical_struct!(FairCpu, "FairCpu.v9", {
    seen, rng, seeded, plan, settle, reach, reach_seen, last_normal, shot_range, shot_far, fighter, pad,
    forced_stronger, judge, judged, game, route, recent_routes, route_outcome, route_drops, route_check,
    focus, action_frame, last_view, habits, plan_worked, last_plan, route_landed, out_of_fight, jump_began, jump_met, answered_plan,
    bait_until, reset_wait, pressure_run, forced_level,
});

crate::canonical::canonical_enum!(Role, "Role.v1";
    Projectile => 0,
    AntiAir => 1,
    Advance => 2,
);

crate::canonical::canonical_enum!(PlanKind, "PlanKind.v1";
    Idle => 0,
    Walk => 1,
    Guard { low, min, clear, held } => 2,
    Await { choice, wait } => 3,
    Act => 4,
);

crate::canonical::canonical_enum!(Choice, "Choice.v1";
    Approach => 0,
    Retreat => 1,
    Wait => 2,
    Guard => 3,
    Normal { crouch, button } => 4,
    Combo => 5,
    Cancel { crouch, button, role } => 6,
    Throw => 7,
    Special(value0) => 8,
    JumpIn => 9,
    JumpBack => 10,
);


impl crate::canonical::Canonical for Rng {
    fn encode(&self, out: &mut crate::canonical::CanonicalHasher) {
        let Self(seed) = self;
        out.tag("FairCpuRng.v1");
        seed.encode(out);
    }
}

impl Body {
    /// In an attack, as drawn: one of the attack moves (`0x0d..=0x2f`, the
    /// normals, the specials, the throw and the jump attacks, excluding
    /// [`crate::combo::throw::GET_UP_MOVE`]). Not the state byte's
    /// attack bits alone: the 1987 jump code sets them (`+9 = 0xc1`) as soon
    /// as a button is pressed for a jump attack still to come
    /// (`buffer_air_strength`), frames before anything is drawn, so they
    /// would show the CPU a button the player has only pressed.
    pub fn attacking(&self) -> bool {
        self.present
            && (0x0d..=0x2f).contains(&self.move_id)
            && self.move_id != crate::combo::throw::GET_UP_MOVE
    }

    /// In a normal attack (moves `0x0d..=0x18`), not a special, a super or
    /// a system move.
    pub fn normal(&self) -> bool {
        self.attacking() && self.move_id <= 0x18
    }

    /// In a jump (moves 9, 0xa and 0xb, the pre-jump crouch included) or a
    /// jump attack (an attack move whose state says airborne, `+9` bit 0).
    pub fn jumping(&self) -> bool {
        self.present && (matches!(self.move_id, 9..=0xb) || (self.attacking() && self.state & 1 != 0))
    }

    /// An attack that must be guarded low: a crouching attack (an attack
    /// move whose state says crouching, `+9` bit 2).
    pub fn low_attack(&self) -> bool {
        self.attacking() && self.state & 4 != 0
    }

    /// In a hit reaction, rising from a throw knockdown, or knocked out
    /// (moves `0x30..` or [`crate::combo::throw::GET_UP_MOVE`]).
    pub fn reacting(&self) -> bool {
        self.present && (self.move_id >= 0x30 || self.move_id == crate::combo::throw::GET_UP_MOVE)
    }

    /// Guarding: the standing or crouching guard (moves 4 and 8).
    pub fn guarding(&self) -> bool {
        self.present && matches!(self.move_id, 4 | 8)
    }

    /// Crouching, as his pose's state byte says (`+9` bit 2).
    pub fn crouching(&self) -> bool {
        self.present && self.state & 4 != 0
    }

    /// Free to start something: not attacking, hit, or in the air.
    pub fn free(&self) -> bool {
        self.present && !self.attacking() && !self.reacting() && !self.jumping()
    }
}

/// One frame as the CPU saw it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Percept {
    /// The frame (the machine's frame count after the step).
    pub frame: u32,
    /// A round is being fought and both fighters are drawn.
    pub fighting: bool,
    /// The CPU's own fighter and the opponent.
    pub me: Body,
    pub foe: Body,
    /// The opponent's projectile nearest the CPU's fighter: present, its
    /// centre and its half width.
    pub shot: bool,
    pub shot_x: i16,
    pub shot_half: i16,
    /// One of the CPU's own projectiles is on screen, and how far the
    /// farthest is from the CPU's fighter.
    pub my_shot: bool,
    pub my_shot_dist: i16,
}

impl Percept {
    /// Which way is toward the opponent: +1 right, -1 left.
    fn toward(&self) -> i16 {
        if self.foe.x >= self.me.x {
            1
        } else {
            -1
        }
    }

    /// Distance between the two centres.
    fn dist(&self) -> i16 {
        (self.foe.x - self.me.x).abs()
    }

    /// From the CPU fighter's centre to the near edge of the opponent's
    /// hurtbox: what an attack must reach.
    fn gap(&self) -> i16 {
        if self.toward() > 0 {
            self.foe.hurt.0 - self.me.x
        } else {
            self.me.x - self.foe.hurt.1
        }
    }

    /// From the opponent's centre to the near edge of the CPU fighter's
    /// hurtbox: what his attacks must reach.
    fn foe_gap(&self) -> i16 {
        if self.toward() > 0 {
            self.foe.x - self.me.hurt.1
        } else {
            self.me.hurt.0 - self.foe.x
        }
    }

    /// The opponent's projectile is coming: on screen and in front of the CPU.
    fn shot_coming(&self) -> bool {
        self.shot && (self.shot_x - self.me.x) * self.toward() > -16
    }

    /// What the CPU should guard against now, if anything: `Some(true)` for
    /// low (a crouching attack), `Some(false)` for high (a jump, a jump
    /// attack, a projectile), and for a standing attack whichever it chose.
    fn threat(&self) -> Option<Option<bool>> {
        if self.foe.jumping() || self.shot_coming() {
            Some(Some(false))
        } else if self.foe.low_attack() {
            Some(Some(true))
        } else if self.foe.attacking() {
            Some(None)
        } else {
            None
        }
    }
}

/// Reads one frame from the machine's work RAM (`0xff8000..`) and object RAM
/// (`0xffe000..`), for the CPU on `side`. Read only; see the module docs for
/// what it does not read.
pub fn perceive(frame: u32, in_fight: bool, work: &[u8], obj: &[u8], side: Side) -> Percept {
    let mode = work.get((GAME_MODE - WORK_BASE) as usize).copied().unwrap_or(0);
    let other = if side == Side::P1 { Side::P2 } else { Side::P1 };
    perceive_fighters(frame, in_fight, work, obj, player_struct(side, mode), player_struct(other, mode))
}

/// Dramatic controllers observe their own fighter and one designated opponent.
pub fn perceive_fighters(
    frame: u32,
    in_fight: bool,
    work: &[u8],
    obj: &[u8],
    my_ps: u32,
    foe_ps: u32,
) -> Percept {
    let wb = |a: u32| work.get((a - WORK_BASE) as usize).copied().unwrap_or(0);
    let ob = |at: u32| obj.get(at as usize).copied().unwrap_or(0);
    let ow = |at: u32| u16::from_be_bytes([ob(at), ob(at + 1)]);
    let ol = |at: u32| u32::from(ow(at)) << 16 | u32::from(ow(at + 2));
    let sext = |b: u8| i16::from(b as i8);
    let body = |ps: u32| -> Body {
        let health = 4 * u16::from(wb(ps + 7)) + u16::from(wb(ps + 6));
        let e = u16::from_be_bytes([wb(ps + 0x10), wb(ps + 0x11)]);
        if !FIGHTER_SLOTS.contains(&e) || ob(u32::from(e) + 8) == 0 {
            return Body { health, ..Body::default() };
        }
        let e = u32::from(e);
        // The drawing's centre (`center_x` / `center_y`, as the collision
        // judge measures from it): the grid's corner and half its cells.
        let big = (ow(e + 2) as i16) >= 0x400;
        let cell: u16 = if big { 0x20 } else { 0x10 };
        let half = |n: u8| (u16::from(n).wrapping_mul(cell).wrapping_add(cell)) >> 1;
        let x = ow(e + 6).wrapping_add(half(ob(e + 0xd))) as i16;
        let y = ow(e + 4).wrapping_add(half(ob(e + 0xc))) as i16;
        let facing_left = (ob(e + 0x37) as i8) >= 4;
        let flags = ob(e + 0xf);
        // The attack box (`+0x22..+0x25`) and the loose hurtbox
        // (`+0x1e..+0x21`): x offsets already signed for the facing.
        let (ax, aw) = (x + sext(ob(e + 0x24)), i16::from(ob(e + 0x25)));
        let (hx, hw) = (x + sext(ob(e + 0x20)), i16::from(ob(e + 0x21)));
        let active = flags & 0x80 != 0 && aw > 0;
        Body {
            present: true,
            x,
            y,
            facing_left,
            move_id: ob(e + 0xa),
            state: ob(e + 9),
            active,
            front: if !active {
                x
            } else if facing_left {
                ax - aw
            } else {
                ax + aw
            },
            hurt: (hx - hw, hx + hw),
            health,
        }
    };
    let me = body(my_ps);
    let foe = body(foe_ps);
    let mut p = Percept { frame, fighting: in_fight && me.present && foe.present, me, foe, ..Percept::default() };
    // The projectile and effect slots (`0x100 + 0x40 * k`) whose collision
    // flags are live, by their owner's player structure.
    for k in 0..12u32 {
        let s = 0x100 + 0x40 * k;
        if ob(s + 8) == 0 || ob(s + 0xe) & 0xe0 == 0 {
            continue;
        }
        let owner = ol(s + 0x20);
        let half = if ow(s + 2) & 0x400 != 0 { 0x10 } else { 8 };
        let sx = ow(s + 6).wrapping_add(half) as i16;
        if owner == my_ps {
            p.my_shot = true;
            p.my_shot_dist = p.my_shot_dist.max((sx - me.x).abs());
        } else if owner == foe_ps && (!p.shot || (sx - me.x).abs() < (p.shot_x - me.x).abs()) {
            p.shot = true;
            p.shot_x = sx;
            p.shot_half = i16::from(ob(s + 0xd));
        }
    }
    p
}

/// What a special is for. It comes from the special's slot record in the
/// fighter's move table (ticket #251, [`SpecialSlot`]; Ryu's and Ken's ROM
/// tables have the 1987 records, [`SpecialSlot::ryu`]), so a boss whose
/// table lands needs no change here:
/// - a melee special on the Shoryuken's rising tracker (`0x19`, or guarded
///   crouching even in the air: the Shoryuken, Joe's Dynamite Straight) is
///   the **anti-air**;
/// - one that is not melee nor guarded crouching in the air (the Hadoken's
///   kind) is a **projectile**; how far it goes the CPU learns by watching
///   its own ([`FairCpu::learn_shot`]), so a short blast such as Retsu's
///   Kiai Shot is soon kept for close range;
/// - any other (the Tatsumaki's kind, Geki's Teleport Strike, Joe's Rolling
///   Sobat) **carries the fighter in** from the middle distance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Projectile,
    AntiAir,
    Advance,
}

fn role(slot: &SpecialSlot) -> Role {
    if !slot.melee && !slot.low_in_air {
        Role::Projectile
    } else if slot.melee && (slot.low_in_air || slot.tracker == 0x19) {
        Role::AntiAir
    } else {
        Role::Advance
    }
}

/// What the CPU's fighter has: the specials the registry gives him in this
/// fight, by role (up to two when a fighter such as Lee, Birdie, Eagle or
/// Sagat has two specials of one role).
#[derive(Clone, Copy, Debug, Default)]
pub struct Kit {
    pub fighter: Option<TournamentFighter>,
    projectile: Option<MoveEntry>,
    projectile_alt: Option<MoveEntry>,
    anti_air: Option<MoveEntry>,
    anti_air_alt: Option<MoveEntry>,
    advance: Option<MoveEntry>,
    advance_alt: Option<MoveEntry>,
}

impl Kit {
    /// `fighter`'s specials under `preset`. `slots` are the special slot
    /// records of the move table he plays on (`0x19`, `0x1a`, `0x1b`), or
    /// `None` when the fight holds no 1987 player table for him (a boss
    /// without the art pack), whose specials he then does not have.
    pub fn of(fighter: TournamentFighter, preset: RulesetPreset, slots: Option<[SpecialSlot; 3]>) -> Kit {
        Self::from_moves(fighter, slots, move_list(fighter, preset).into_iter().copied())
    }

    #[cfg(test)]
    fn of_alpha(fighter: TournamentFighter, preset: RulesetPreset, slots: Option<[SpecialSlot; 3]>, era: &EraFighterState) -> Kit {
        let entries = live_move_list(fighter, preset, slots.is_some(), era);
        Self::from_moves(fighter, slots, entries)
    }

    fn from_moves(fighter: TournamentFighter, slots: Option<[SpecialSlot; 3]>, entries: impl IntoIterator<Item = MoveEntry>) -> Kit {
        let mut kit = Kit {
            fighter: Some(fighter),
            ..Kit::default()
        };
        let ryu = [0x19, 0x1a, 0x1b].map(SpecialSlot::ryu);
        for entry in entries {
            if entry.kind != MoveKind::Special || (entry.available.player_table && slots.is_none()) {
                continue;
            }
            let move_role = if entry.recognizer == Recognizer::AlphaMove {
                use TournamentFighter::{Adon, Birdie, Eagle, Gen, Sagat};
                match (fighter, entry.produces) {
                    (Adon, 0x19) | (Gen, 0x1b) | (Eagle | Sagat, 0x1f) => Some(Role::AntiAir),
                    (Sagat, 0x19) | (Adon, 0x1b | 0x2c) | (Birdie, 0x1a | 0x1b)
                    | (Eagle, 0x1a | 0x2d | 0x2f) | (Gen, 0x1a | 0x2d | 0x2f) => Some(Role::Advance),
                    // The ground attack plan needs a setup for airborne moves,
                    // command throws and counters before it can pick them.
                    _ => None,
                }
            } else if entry.recognizer == Recognizer::FamilyMove {
                if entry.input.motion.first() == Some(&Dir::Jump)
                    || matches!(entry.input.button, ButtonClass::PairPunch | ButtonClass::PairKick)
                {
                    None
                } else if ["SHORYUKEN", "UPPERCUT", "GEKIROU"].iter().any(|name| entry.name.contains(name)) {
                    Some(Role::AntiAir)
                } else if ["HADOKEN", "TIGER SHOT", "KIAI SHOT"].iter().any(|name| entry.name.contains(name)) {
                    Some(Role::Projectile)
                } else {
                    Some(Role::Advance)
                }
            } else {
                (0x19..=0x1b).contains(&entry.produces).then(|| role(&slots.unwrap_or(ryu)[usize::from(entry.produces - 0x19)]))
            };
            let Some(move_role) = move_role else {
                continue;
            };
            let (first, second) = match move_role {
                Role::Projectile => (&mut kit.projectile, &mut kit.projectile_alt),
                Role::AntiAir => (&mut kit.anti_air, &mut kit.anti_air_alt),
                Role::Advance => (&mut kit.advance, &mut kit.advance_alt),
            };
            if first.is_none() {
                *first = Some(entry);
            } else if second.is_none() {
                *second = Some(entry);
            }
        }
        kit
    }

    /// The two places for specials of `role`, first choice first.
    fn slots(&mut self, role: Role) -> (&mut Option<MoveEntry>, &mut Option<MoveEntry>) {
        match role {
            Role::Projectile => (&mut self.projectile, &mut self.projectile_alt),
            Role::AntiAir => (&mut self.anti_air, &mut self.anti_air_alt),
            Role::Advance => (&mut self.advance, &mut self.advance_alt),
        }
    }

    fn by_role(&self, role: Role) -> [Option<MoveEntry>; 2] {
        match role {
            Role::Projectile => [self.projectile, self.projectile_alt],
            Role::AntiAir => [self.anti_air, self.anti_air_alt],
            Role::Advance => [self.advance, self.advance_alt],
        }
    }

    fn cancels_with(&self, role: Role) -> bool {
        self.by_role(role).into_iter().flatten().any(can_cancel_into)
    }
}

fn can_cancel_into(entry: MoveEntry) -> bool {
    !matches!(entry.recognizer, Recognizer::AlphaMove | Recognizer::FamilyMove) || entry.input.button == ButtonClass::MashPunch
}

/// What the CPU is told about the fight besides what it sees: its level,
/// its fighter's specials, and the rules (a player knows which rules are on).
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub level: Level,
    pub kit: Kit,
    era: Option<Tactics>,
    /// Under `standard_high_low_block` the crouching guard stops standing
    /// attacks too; without it only the standing guard does.
    pub standard_block: bool,
    /// True combos (`true_combos`, SP17): link strings can hit an opponent
    /// still in hitstun.
    pub true_combos: bool,
    /// Normal-to-special cancels (`cancels`, SP18): cancelable normals can
    /// cancel into a special on hit or block.
    pub cancels: bool,
    /// Throws and throw techs (`throws`, SP19): close `Forward/Back + HP/HK`
    /// throws, and techs inside the 7-frame window.
    pub throws: bool,
    /// The developer-only switch for the stronger play (spec `#192`):
    /// situations and game plans ([`game`]).
    pub stronger: bool,
    /// The era style its fighter plays in, where one is in force: game
    /// plans and route books are looked up by fighter and era style.
    family: Option<crate::era::EraFamily>,
    style: Option<(crate::era::Revision, crate::era::Loadout)>,
    /// Its own Super meter and Drive gauge, as its gauges show them, and
    /// whether its Super meter is full.
    meters: [u16; 2],
    meter_full: bool,
    /// The round could end the match: either fighter has a round won, or
    /// it is the third round (the win marks and the round call show it).
    /// Super meter carries from round to round in this engine, so a round
    /// that cannot end the match is no reason to spend it.
    pub(crate) round_decides_match: bool,
    /// The opponent's whole health in an era style's finer units (a route
    /// book's damage is in them): how much a health bar point of his is.
    foe_max_sub_hp: u16,
    /// The opponent's fighter and style, where an era style is in force: a
    /// route the book records failing on him is not taken (SC13, `#205`).
    pub(crate) foe: Option<(TournamentFighter, (crate::era::Revision, crate::era::Loadout))>,
    /// The round of the match being fought, from 0 (`ROUND_NO`): the first
    /// round's start is a new match, where the habit memory starts empty.
    pub(crate) round: u8,
    /// A World Tour mission allows only normals and throws (Pure Footsies):
    /// the engine refuses specials, supers, era commands, command normals
    /// and air throws to both fighters, though it still lets through the
    /// SF3 era style's overhead and the Alpha styles' moves of the normal
    /// kind. The stronger play keeps to plain normals and throws (SC23,
    /// `#215`), stricter than the rule: see [`Context::normals_and_throws`].
    /// The old play ignores it, as it always has.
    pub(crate) pure_footsies: bool,
    /// What the stronger play reads its fighter's specials as, where it
    /// reads them otherwise than the old play (SC15, `#207`;
    /// [`cast::stronger_kit`]), and his tools: command throws, teleports
    /// and charge moves. Only the stronger play uses either.
    strong_kit: Option<Kit>,
    tools: cast::Tools,
    /// Its own fighter is in blockstun, as it feels its own state (SC15,
    /// `#207`). Kept under a mission that sets the era kit aside.
    own_blockstun: bool,
    /// Baseline identity and felt blockstun, applied only to effective
    /// stronger play (including the measuring/replay controller override).
    /// Keeping it deferred leaves the old play's context unchanged.
    baseline: Option<(crate::era::StylePick, bool)>,
}

impl Context {
    pub fn new(level: Level, kit: Kit, toggles: &CoreToggles) -> Context {
        Context {
            level,
            kit,
            era: None,
            standard_block: toggles.standard_high_low_block,
            true_combos: toggles.true_combos,
            cancels: toggles.cancels,
            throws: toggles.throws,
            stronger: false,
            family: None,
            style: None,
            meters: [0; 2],
            meter_full: false,
            round_decides_match: false,
            foe_max_sub_hp: crate::era_fight::FULL_SUB_HP,
            foe: None,
            round: 0,
            pure_footsies: false,
            strong_kit: None,
            tools: cast::Tools::default(),
            own_blockstun: false,
            baseline: None,
        }
    }

    /// The context the stronger play decides from: its fighter's specials
    /// as it reads them and his tools ([`cast`]), and, under a mission
    /// that allows only normals and throws, without them
    /// ([`Self::normals_and_throws`]).
    pub(crate) fn for_stronger(&self) -> Context {
        let mut context = *self;
        if let Some((style, blockstun)) = self.baseline {
            context.family = Some(style.family);
            context.style = Some((style.revision, style.loadout));
            context.own_blockstun = blockstun;
            // The 1987 style never brings overhaul offense with it.
            // SF1+ keeps the configured, independent offense toggles.
            if style.family == crate::era::EraFamily::Sf1987 {
                context.true_combos = false;
                context.cancels = false;
                context.throws = false;
            }
        }
        if let Some(kit) = self.strong_kit {
            context.kit = kit;
        }
        context.normals_and_throws()
    }

    /// What the stronger play may use under a mission that allows only
    /// normals and throws (SC23, `#215`): its fighter's specials and the
    /// era style's commands (Drive moves, parries, supers and the rest)
    /// are gone, and with them every choice, game plan and route that
    /// needs one ([`game::GamePlan::needs_specials`],
    /// [`crate::route_book::Route::normals_only`]). Its normals, link
    /// strings, jump-ins and throws stay. Any other context is returned as
    /// it is.
    pub(crate) fn normals_and_throws(&self) -> Context {
        let mut context = *self;
        if context.pure_footsies {
            context.kit = Kit { fighter: self.kit.fighter, ..Kit::default() };
            context.era = None;
            context.tools = cast::Tools::default();
        }
        context
    }

    /// Attach the identity of a baseline style without changing a single
    /// old-play choice. Baseline styles use their original kit, without
    /// later-era tactics or gauges (SC21, `#213`).
    pub(crate) fn with_baseline(mut self, style: crate::era::StylePick, blockstun: bool) -> Self {
        if matches!(style.family, crate::era::EraFamily::Sf1987 | crate::era::EraFamily::Sf1Plus) {
            self.baseline = Some((style, blockstun));
        }
        self
    }

    fn baseline_rules(&self) -> bool {
        matches!(self.family, Some(crate::era::EraFamily::Sf1987 | crate::era::EraFamily::Sf1Plus))
    }

    /// A later era brings its offense rules; baseline styles do not.
    fn era_offense(&self) -> bool {
        self.family.is_some() && !self.baseline_rules()
    }

    /// SC20 extends the SF6 cast after Ryu's separately proven first pair.
    /// New plan gates must not change his choices or random-number stream.
    fn sf6_cast(&self) -> bool {
        self.family == Some(crate::era::EraFamily::Sf6)
            && self.kit.fighter.is_some_and(|fighter| fighter != TournamentFighter::Ryu)
    }

    pub(crate) fn with_era(
        level: Level,
        fighter: TournamentFighter,
        slots: Option<[SpecialSlot; 3]>,
        config: crate::config::MatchConfig,
        state: &EraFighterState,
        control: EraControl,
    ) -> Self {
        let entries = live_move_list(fighter, config.preset, slots.is_some(), state);
        let kit = Kit::from_moves(fighter, slots, entries.iter().copied());
        let mut context = Self::new(level, kit, &config.toggles);
        context.era = Some(Tactics::of(state, &entries, config, level, control));
        context.family = Some(state.style.family);
        context.style = Some((state.style.revision, state.style.loadout));
        context.meters = [u16::from(state.super_meter), state.sf6_drive_points().unwrap_or(0)];
        context.meter_full = state.super_capacity() != 0 && state.super_meter >= state.super_capacity();
        context.own_blockstun = control.blockstun;
        if let Some((kit, tools)) = cast::stronger_kit(fighter, state.style.family, &entries) {
            context.strong_kit = Some(kit);
            context.tools = tools;
        }
        context
    }

    /// Short commands supplement the established spacing, guards, and
    /// longer charge and jump sequences. A command still has to pass the
    /// fighter's own motion recognizer and action gates.
    /// Tell it the opponent's era style (its kit tells his moves apart by
    /// it) and his whole health in finer units (0 for none: a health bar
    /// point is then Ryu's).
    pub(crate) fn set_opponent(&mut self, family: crate::era::EraFamily, max_sub_hp: u16) {
        self.era = self.era.map(|tactics| tactics.against(Some(family)));
        if max_sub_hp != 0 {
            self.foe_max_sub_hp = max_sub_hp;
        }
    }

    pub(crate) fn utility_macro(
        &self,
        family: crate::era::EraFamily,
        view: Percept,
        spot: crate::utility::Spot,
        action: crate::utility::Action,
        simple_spacing: bool,
        max_frames: usize,
    ) -> Option<Vec<u16>> {
        use crate::utility::Action::*;

        let forward = if view.me.facing_left { LEFT } else { RIGHT };
        let back = if forward == LEFT { RIGHT } else { LEFT };
        if action == Parry && self.era.is_some_and(Tactics::can_tap_parry) {
            let direction = if view.foe.low_attack() { DOWN } else { forward };
            return Some(vec![0, direction, 0]);
        }
        if action == Fadc && self.era.is_some_and(Tactics::can_fadc) {
            let focus = button(1) | button(4);
            return Some(vec![focus, focus | forward, focus, focus | forward, forward, 0]);
        }
        let entry = match action {
            AntiAir if view.me.free() && spot.airborne => {
                if let Some(entry) = self.kit.anti_air {
                    Some((entry, 2))
                } else {
                    return Some(vec![DOWN | button(2), DOWN, 0]);
                }
            }
            Normal if simple_spacing && view.me.free() && !spot.attack_incoming => {
                return Some(if spot.distance > 65 {
                    vec![forward; 6]
                } else {
                    vec![button(1), button(1), 0]
                });
            }
            ThrowTech if self.throws => return Some(vec![back | button(2), 0]),
            SpecialCancel if self.cancels && spot.can_cancel => {
                // The motion must finish inside the live cancel window.
                // Longer charge and mash plans remain with the base CPU.
                [Role::AntiAir, Role::Projectile, Role::Advance].into_iter()
                    .flat_map(|role| self.kit.by_role(role))
                    .flatten()
                    .find(|entry| entry.input.motion.len() <= 3 && !entry.input.motion.iter().any(|d| d.is_charge())
                        && matches!(entry.input.button, ButtonClass::Punch | ButtonClass::Kick))
                    .map(|entry| (entry, 2))
            }
            _ => self.era.and_then(|tactics| tactics.utility_command(spot, action)),
        }?;
        let mut pads = alpha_motion(&entry.0, !view.me.facing_left, entry.1, true);
        if family == crate::era::EraFamily::Sf6 && entry.0.produces == crate::era_fight::sf6::RUSH {
            pads.insert(0, 0); // A held Forward must release before the dash.
        }
        if !pads.iter().take(max_frames).any(|&pad| pad & !0xf != 0) && entry.0.input.button != ButtonClass::DirectionOnly {
            // Supers can have six or more motion steps. Preserve the same
            // directions and edge while leaving longer charges to the base CPU.
            if entry.0.input.motion.iter().any(|d| d.is_charge() || *d == Dir::Jump)
                || matches!(entry.0.input.button, ButtonClass::ReleasePair | ButtonClass::MashPunch)
            {
                return None;
            }
            pads.dedup();
        }
        pads.truncate(max_frames);
        Some(pads)
    }
}

/// The fighter's route book in its era style holds a route of plain
/// normals ([`crate::route_book::Route::normals_only`]) that starts in
/// `reach`: what a mission allowing only normals and throws leaves it.
fn normals_route(ctx: &Context, reach: crate::route_book::Reach) -> bool {
    let (Some(fighter), Some((revision, loadout))) = (ctx.kit.fighter, ctx.style) else { return false };
    crate::route_book::book(fighter, revision, loadout)
        .is_some_and(|book| book.routes.iter().any(|route| route.reach == reach && route.normals_only()))
}

/// How near, centre to centre, its fighter's command throw is entered from:
/// inside its catch range by [`CATCH_MARGIN`] (SC15, `#207`).
fn grab_range(ctx: &Context) -> i16 {
    ctx.kit.fighter.and_then(cast::catch_range).unwrap_or(40) - CATCH_MARGIN
}

/// The era style's tool `act` starts is there now (SC16, `#208`): Custom
/// Combo with the meter to start it, a change of stance from the Mantis,
/// SF3's overhead, or affordable SF4/SF5 tools and finishers.
fn era_ready(act: game::EraAct, ctx: &Context) -> bool {
    ctx.era.is_some_and(|tactics| match act {
        game::EraAct::CustomCombo => tactics.custom_combo_ready(),
        game::EraAct::Stance => tactics.can_change_stance(),
        game::EraAct::CustomChain => true,
        game::EraAct::Overhead => tactics.sf3_overhead_ready(),
        game::EraAct::Enhanced => tactics.sf3_ex_ready() || tactics.sf4_ex_ready() || tactics.sf5_ex_ready() || tactics.sf6_ex_ready(),
        game::EraAct::Focus => tactics.sf4_focus_ready(),
        game::EraAct::Super => (tactics.sf4_super_ready() || tactics.sf5_super_ready() || tactics.sf6_super_ready()) && ctx.level.spends_meter(),
        game::EraAct::Ultra => tactics.sf4_ultra_ready() && ctx.level.spends_meter(),
        game::EraAct::VSkill => tactics.sf5_skill_ready(),
        game::EraAct::VTrigger => tactics.sf5_trigger_ready(),
        game::EraAct::DriveImpact => tactics.sf6_impact_ready(),
    })
}

/// The fighter's route book in its era style holds a route that starts in
/// `reach`.
fn book_route(ctx: &Context, reach: crate::route_book::Reach) -> bool {
    let (Some(fighter), Some((revision, loadout))) = (ctx.kit.fighter, ctx.style) else { return false };
    crate::route_book::book(fighter, revision, loadout).is_some_and(|book| book.routes.iter().any(|route| route.reach == reach))
}

/// A baseline book contains material the actual offense rules and level
/// permit. Spacing, stance and observed opponent restrictions are checked
/// again by the route player at execution time.
fn baseline_route(ctx: &Context, reach: crate::route_book::Reach) -> bool {
    let (Some(fighter), Some((revision, loadout))) = (ctx.kit.fighter, ctx.style) else { return false };
    crate::route_book::book(fighter, revision, loadout).is_some_and(|book| book.routes.iter().any(|route| {
        route.reach == reach && (!route.true_combos || ctx.true_combos) && (!route.cancels || ctx.cancels)
            && route.hits <= ctx.level.route_hits()
            && (ctx.level.spends_meter() || route.super_spent + route.drive_spent == 0)
            && (!ctx.pure_footsies || route.normals_only())
    }))
}

/// What a plan is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlanKind {
    /// Nothing, or standing still.
    Idle,
    /// Walking in or out.
    Walk,
    /// Holding a guard while there is something to guard against: `low`
    /// for a standing attack is the choice made when it started; `min`
    /// frames at least (a guard held in anticipation); `clear` frames seen
    /// with nothing to guard against.
    Guard { low: bool, min: u16, clear: u8, held: u16 },
    /// An anti-air waiting for its moment: `choice` once `wait` more frames
    /// have passed, unless the jump is seen to end first.
    Await { choice: Choice, wait: u16 },
    /// An attack, a special or a jump: never interrupted.
    Act,
}

/// The CPU's current plan: pad frames to give, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Plan {
    kind: PlanKind,
    pads: [u16; PLAN_CAP],
    len: u8,
    at: u8,
    /// The era kit's answer to something seen (SC8, `#200`): its stick
    /// forward is a motion's (a Drive Reversal's), not a walk in.
    answer: bool,
}

impl Default for Plan {
    fn default() -> Self {
        Plan { kind: PlanKind::Idle, pads: [0; PLAN_CAP], len: 0, at: 0, answer: false }
    }
}

impl Plan {
    fn of(kind: PlanKind, frames: &[u16]) -> Plan {
        let mut plan = Plan { kind, ..Plan::default() };
        for &pad in frames.iter().take(PLAN_CAP) {
            plan.pads[usize::from(plan.len)] = pad;
            plan.len += 1;
        }
        plan
    }

    fn guard(low: bool, min: u16) -> Plan {
        Plan { kind: PlanKind::Guard { low, min, clear: 0, held: 0 }, ..Plan::default() }
    }

    fn done(&self) -> bool {
        match self.kind {
            PlanKind::Guard { .. } | PlanKind::Await { .. } => false,
            _ => self.at >= self.len,
        }
    }

    /// A plan something seen may cut short: standing still, walking, and
    /// guarding (a new threat starts a new guard).
    /// End the plan with its last press: what follows it (empty frames, a
    /// stick still held) is dropped, so the stronger play chooses again as
    /// soon as its last input is in rather than standing or walking on
    /// through them (SC8, `#200`).
    pub(super) fn end_at_last_press(&mut self) {
        if let Some(last) = self.pads[..usize::from(self.len)].iter().rposition(|&pad| pad & BUTTONS != 0) {
            self.len = last as u8 + 1;
        }
    }

    fn interruptible(&self) -> bool {
        self.done() || matches!(self.kind, PlanKind::Idle | PlanKind::Walk | PlanKind::Guard { .. })
    }

    /// An action whose first button is still to come: nothing it does has
    /// begun yet (a crouching normal holds the stick down a frame first).
    fn press_to_come(&self) -> bool {
        self.kind == PlanKind::Act
            && self.pads[..usize::from(self.len)].iter().any(|&pad| pad & BUTTONS != 0)
            && self.pads[..usize::from(self.at.min(self.len))].iter().all(|&pad| pad & BUTTONS == 0)
    }
}

/// The options the CPU weighs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Choice {
    Approach,
    Retreat,
    Wait,
    Guard,
    Normal { crouch: bool, button: usize },
    Combo,
    Cancel { crouch: bool, button: usize, role: Role },
    Throw,
    Special(Role),
    JumpIn,
    JumpBack,
}

/// A 64-bit xorshift generator: the CPU's only randomness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }

    /// True `percent` times in a hundred.
    fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }
}

/// Each normal's reach before the CPU has seen it: pixels from the stance
/// centre to the attack box's forward edge, standing then crouching, LP to
/// HK (Ryu's, as the fight shows them; the ROM box audit's median attack
/// reaches 64). The CPU replaces each with what it sees its own attack box
/// reach ([`FairCpu::learn`]), longer or shorter, so a fighter is soon
/// played with his own reach.
const DEFAULT_REACH: [[i16; 6]; 2] = [[50, 62, 66, 52, 58, 66], [58, 58, 58, 52, 52, 56]];

/// Steps an answer that cut a game plan short has to land for the plan to
/// count as having worked: an anti-air timed to his jump meets him within
/// about this.
const ANSWER_WATCH: u32 = 60;

/// Steps with no round fought that part two fights: the shortest break
/// between rounds (the knockout, the call, the walk-in) is longer, and a
/// moment a fighter is not drawn, or a frame missing from its view, in a
/// first round is far shorter, so it does not wipe the memory mid-match.
const BETWEEN_FIGHTS: u16 = 90;

/// Frames, and pixels over them, the opponent's drift is judged by: his
/// drawing's centre shifts a few pixels as his pose changes.
const DRIFT_FRAMES: u32 = 4;
const DRIFT: i16 = 6;

/// The most view frames into a jump it was too busy to answer as it began
/// that an expected jump is still answered: later, an anti-air would come
/// out after he lands.
const MISSED_JUMP: u32 = 36;

/// A projectile seen to go at least this far (pixels from the thrower) is
/// thrown from across the screen; a shorter one is kept for close range.
const LONG_SHOT: i16 = 160;

/// The normal the CPU pressed last: its posture and button, the step it was
/// pressed on, where its fighter stood when it was seen to start, and the
/// farthest its attack box has reached from there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Normal {
    crouch: bool,
    button: usize,
    pressed: u32,
    from: Option<i16>,
    best: i16,
}

/// The fair CPU's whole state, kept in the machine's snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FairCpu {
    seen: [Percept; RING],
    rng: Rng,
    seeded: bool,
    plan: Plan,
    /// The view frame the CPU waits for before choosing again, so it sees
    /// what its last action did before it acts again.
    settle: u32,
    reach: [[i16; 6]; 2],
    /// Which normals' reaches it has seen for itself.
    reach_seen: [[bool; 6]; 2],
    /// The normal it pressed last, for [`FairCpu::learn`].
    last_normal: Option<Normal>,
    /// How far its projectile goes, once seen, and the farthest the shot
    /// on screen has gone so far.
    shot_range: Option<i16>,
    shot_far: i16,
    /// Which fighter the learned reaches and projectile range belong to, so
    /// an Arcade stage transition to the next opponent resets them.
    fighter: Option<TournamentFighter>,
    /// The pad given on the last step.
    pad: u16,
    /// The measuring run plays the stronger game on this side whatever the
    /// match's switch says (SC1, `#193`: new against old).
    forced_stronger: bool,
    /// The stronger play (spec `#192`): what judging remembers, the
    /// situation it last judged, and the game plan being followed.
    judge: game::Judge,
    judged: Judged,
    game: Option<game::Running>,
    /// The route it is entering, the routes it took last (the book's index
    /// plus one; 0 for none; kept across rounds, a cut route counting as
    /// taken), and what came of the last route it entered.
    route: Option<game::RouteRun>,
    recent_routes: [u8; 4],
    route_outcome: Option<game::Outcome>,
    /// What it has seen of its routes in this fight: for each of the
    /// book's routes (by index), how far drops it saw outnumber routes it
    /// saw land. A route that keeps dropping in play is taken less, then
    /// not at all (the book proves routes against one reference position;
    /// in a fight some drop).
    route_drops: [u8; ROUTE_SLOTS],
    /// A route entered in full whose ending it is waiting to see.
    route_check: Option<RouteCheck>,
    /// The stronger play's focus (SC7, `#199`), the frame its last view of
    /// the opponent's actions came from, and that view as it saw it: the
    /// frame before the next one, so an action it has not been shown yet
    /// is seen to start when it shows.
    focus: Focus,
    action_frame: u32,
    last_view: Percept,
    /// The habit memory (SC9, `#201`): what it has noted of the opponent
    /// this match.
    habits: Habits,
    /// How many times in a row each game plan of its table has worked (seen
    /// to hit) this match, the plan taken last (its index plus one; 0 for
    /// none), and which of its book's routes landed whole the last time
    /// they were taken (a bit each): Very Difficult and Difficult repeat
    /// what keeps working, Normal and Easy put variety first.
    plan_worked: [u8; game::PLAN_SLOTS],
    last_plan: u8,
    route_landed: u32,
    /// Steps in a row it has been told no round is being fought (to
    /// `u16::MAX`): a break as long as the one between two fights, before
    /// a first round, begins a new match.
    out_of_fight: u16,
    /// Pressure plans taken in a row since it last took a plan from
    /// neutral, backed off, or a round ended (SC11, `#203`; to
    /// `u8::MAX`); its single choices in neutral do not count.
    pressure_run: u8,
    /// The measuring run plays this side at a level of its own (SC11,
    /// `#203`: one level against the next): [`Level::rank`] plus one, or 0
    /// for the match's own level.
    forced_level: u8,
    /// A game plan an anti-air answer cut short, and the step by which the
    /// anti-air has had time to land: a plan that wins through it (the
    /// stand-and-watch plan's) has worked.
    answered_plan: Option<(u8, u32)>,
    /// The view frame the opponent's jump in the air was seen to begin on,
    /// and whether it has been answered (or let go by the level's chance).
    jump_began: u32,
    jump_met: bool,
    /// The step a combo reset's bait holds its guard until (SC10, `#202`):
    /// nothing seen cuts that guard short.
    bait_until: u32,
    /// A combo reset whose route is cut, until its fighter is free.
    reset_wait: Option<game::ResetWait>,
}

/// What the Fair CPU watches for, and the frame its last view of the
/// opponent's actions came from: the frame its reaction delay shows, or an
/// earlier one while a new action that is not its focus is not yet the
/// slower delay old (SC7, `#199`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attention {
    pub focus: Focus,
    pub action_frame: u32,
}

/// A route entered in full whose ending the CPU is waiting to see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RouteCheck {
    /// The route's index in the fighter's book.
    index: u8,
    /// The view frame that shows how it ended.
    at: u32,
    /// The route has been seen to hit him.
    hit: bool,
}

crate::canonical::canonical_struct!(RouteCheck, "FairCpuRouteCheck.v1", { index, at, hit });

/// Routes of one book whose drops the CPU remembers.
const ROUTE_SLOTS: usize = 32;

impl Default for FairCpu {
    fn default() -> Self {
        FairCpu {
            seen: [Percept::default(); RING],
            rng: Rng(0),
            seeded: false,
            plan: Plan::default(),
            settle: 0,
            reach: DEFAULT_REACH,
            reach_seen: [[false; 6]; 2],
            last_normal: None,
            shot_range: None,
            shot_far: 0,
            fighter: None,
            pad: 0,
            forced_stronger: false,
            judge: game::Judge::default(),
            judged: Judged::default(),
            game: None,
            route: None,
            recent_routes: [0; 4],
            route_outcome: None,
            route_drops: [0; ROUTE_SLOTS],
            route_check: None,
            focus: Focus::default(),
            action_frame: 0,
            last_view: Percept::default(),
            habits: Habits::default(),
            plan_worked: [0; game::PLAN_SLOTS],
            last_plan: 0,
            route_landed: 0,
            out_of_fight: u16::MAX,
            pressure_run: 0,
            forced_level: 0,
            jump_began: 0,
            jump_met: false,
            answered_plan: None,
            bait_until: 0,
            reset_wait: None,
        }
    }
}

impl FairCpu {
    pub fn new() -> FairCpu {
        FairCpu::default()
    }

    pub(crate) fn seeded(seed: u64) -> Self {
        Self { rng: Rng(seed | 1), seeded: true, ..Self::default() }
    }

    /// The pad given on the last step.
    pub fn pad(&self) -> u16 {
        self.pad
    }

    /// The stronger play is on for this CPU: by the match's switch, or
    /// forced for the measuring run.
    fn stronger(&self, ctx: &Context) -> bool {
        ctx.stronger || self.forced_stronger
    }

    /// Play the stronger game whatever the match's switch says. `false`
    /// leaves it to the switch: it does not force the old play where the
    /// switch is on (the measuring run's old sides play with it off).
    pub(crate) fn force_stronger(mut self, on: bool) -> Self {
        self.forced_stronger = on;
        self
    }

    /// Play at `level` whatever the match's difficulty says (the measuring
    /// run's one level against another), or at the match's with `None`.
    pub(crate) fn force_level(mut self, level: Option<Level>) -> Self {
        self.forced_level = level.map_or(0, |level| level.rank() + 1);
        self
    }

    /// The level the measuring run set this CPU to play at, if any.
    pub(crate) fn forced_level(&self) -> Option<Level> {
        [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard].get(usize::from(self.forced_level).checked_sub(1)?).copied()
    }

    /// The situation the stronger play last judged the fight to be in, from
    /// the frame its reaction delay showed it. A CPU playing the old game
    /// judges none and this stays at its default.
    pub fn situation(&self) -> Judged {
        self.judged
    }

    /// What it watches for, and the frame its last view of the opponent's
    /// actions came from.
    pub fn attention(&self) -> Attention {
        Attention { focus: self.focus, action_frame: self.action_frame }
    }

    /// The frame it last decided from, as it saw it (the default between
    /// rounds): whose fighter it plays and whom it faces.
    pub fn last_view(&self) -> Percept {
        self.last_view
    }

    /// What its habit memory holds (SC9, `#201`).
    pub fn habits(&self) -> Habits {
        self.habits
    }

    /// Forget what the habit memory holds, as at a match's start.
    pub fn forget_habits(&mut self) {
        self.habits = Habits::default();
    }

    /// Forget this match: the habit memory, and what has worked.
    fn new_match(&mut self) {
        self.habits = Habits::default();
        self.pressure_run = 0;
        self.answered_plan = None;
        self.plan_worked = [0; game::PLAN_SLOTS];
        self.last_plan = 0;
        self.route_landed = 0;
    }

    /// End the game plan being followed, keeping what came of it: whether
    /// it worked, and that it was the last taken.
    fn end_game_plan(&mut self) {
        if let Some(run) = self.game.take() {
            if let Some(worked) = self.plan_worked.get_mut(usize::from(run.plan)) {
                *worked = if run.hit { worked.saturating_add(1).min(game::WORKED_KEPT) } else { 0 };
            }
            self.last_plan = run.plan + 1;
        }
    }

    /// End the game plan being followed because an anti-air waiting on his
    /// jump takes over: whether the plan worked is judged by what the
    /// anti-air meets.
    fn answer_ends_game_plan(&mut self, step: u32) {
        match self.game {
            Some(run) if !run.hit => {
                self.game = None;
                self.last_plan = run.plan + 1;
                self.answered_plan = Some((run.plan, step + ANSWER_WATCH));
            }
            _ => self.end_game_plan(),
        }
    }

    /// An answer that cut a game plan short is seen to hit him: the plan
    /// worked. Hit itself, or with nothing landed in time, it did not.
    fn credit_answer(&mut self, step: u32, view: Percept) {
        let Some((plan, until)) = self.answered_plan else { return };
        let landed = view.foe.reacting() && view.me.attacking();
        if landed || step > until || view.me.reacting() {
            if let Some(worked) = self.plan_worked.get_mut(usize::from(plan)) {
                *worked = if landed { worked.saturating_add(1).min(game::WORKED_KEPT) } else { 0 };
            }
            self.answered_plan = None;
        }
    }

    /// Whether it is in the middle of a game plan.
    pub fn following_game_plan(&self) -> bool {
        self.game.is_some()
    }

    /// Whether the current input sequence answers an event it has seen.
    pub fn answering(&self) -> bool {
        self.plan.answer
    }

    /// The game plan it is following, as its place in its fighter's table,
    /// and the step it is on (SC15, `#207`).
    pub fn game_plan(&self) -> Option<(usize, usize)> {
        self.game.map(|run| (usize::from(run.plan), usize::from(run.step)))
    }

    /// The situation the game plan it is following is for (SC15, `#207`),
    /// for its fighter and era style.
    pub fn game_plan_situation(&self, fighter: TournamentFighter, family: crate::era::EraFamily) -> Option<Situation> {
        let run = self.game?;
        Some(game::plans(Some(fighter), Some(family)).get(usize::from(run.plan))?.situation)
    }

    /// The tool the step of its game plan it is on uses, if any (SC15,
    /// `#207`), for its fighter and era style.
    pub fn game_plan_tool(&self, fighter: TournamentFighter, family: crate::era::EraFamily) -> Option<Tool> {
        let run = self.game?;
        match game::plans(Some(fighter), Some(family)).get(usize::from(run.plan))?.steps.get(usize::from(run.step))?.act {
            game::Act::Tool(tool) => Some(tool),
            _ => None,
        }
    }

    /// The era style's tool the step of its game plan it is on starts, if
    /// any (SC16, `#208`), for its fighter and era style.
    pub fn game_plan_era(&self, fighter: TournamentFighter, family: crate::era::EraFamily) -> Option<EraAct> {
        let run = self.game?;
        match game::plans(Some(fighter), Some(family)).get(usize::from(run.plan))?.steps.get(usize::from(run.step))?.act {
            game::Act::Era(act) => Some(act),
            _ => None,
        }
    }

    /// The machine step the game plan it is following began on: two plans
    /// followed back to back are told apart by it.
    pub fn game_plan_began(&self) -> Option<u32> {
        self.game.map(|run| run.began)
    }

    /// Whether it is entering a route from its route book.
    pub fn entering_route(&self) -> bool {
        self.route.is_some()
    }

    /// The route it is entering: its place in its fighter's book.
    pub fn route_index(&self) -> Option<usize> {
        self.route.map(|run| usize::from(run.index))
    }

    /// The combo reset the route it is entering is cut into, when it is
    /// cut on purpose at a marked point (SC10, `#202`).
    pub fn route_reset(&self) -> Option<ResetKind> {
        self.route.and_then(|run| run.reset).map(|reset| reset.kind)
    }

    /// The step of that route its next pad belongs to, once its walk in is
    /// over.
    pub fn route_step(&self) -> Option<usize> {
        self.route.and_then(|run| run.step_index())
    }

    /// Whether its pad is its own alone: with the stronger play (by the
    /// switch, or forced for the measuring run) it enters every command
    /// (SC8, `#200`), and the Utility AI adds none. The old play shares the
    /// pad with the Utility AI. Training's drills never reach the Fair CPU.
    pub(crate) fn owns_pad(&self, ctx: &Context) -> bool {
        self.stronger(ctx)
    }

    /// Record what frame `p.frame` showed.
    pub fn observe(&mut self, p: Percept) {
        self.seen[p.frame as usize % RING] = p;
    }

    pub(crate) fn set_utility_pad(&mut self, pad: u16) {
        if pad != self.pad {
            self.plan = Plan::default();
            self.last_normal = None;
            self.pad = pad;
        }
    }

    /// A neutral pause can accept a new tactic. A zero pad inside a motion,
    /// timed anti-air or stance plan must stay with that plan.
    pub(crate) fn utility_idle(&self) -> bool {
        self.pad == 0 && self.game.is_none() && self.route.is_none() && (self.plan.done() || self.plan.kind == PlanKind::Idle)
    }

    /// What frame `frame` showed, if the CPU still holds it.
    fn seen(&self, frame: u32) -> Option<Percept> {
        let p = self.seen[frame as usize % RING];
        (p.frame == frame && frame != 0).then_some(p)
    }

    /// The pad for step `step` (the machine's frame count once it has
    /// stepped), decided from the percept `ctx.level.reaction()` frames old.
    /// `in_fight` is whether a round is being fought now; outside one the
    /// CPU lets go of everything.
    pub fn press(&mut self, step: u32, in_fight: bool, ctx: &Context) -> u16 {
        if self.fighter != ctx.kit.fighter {
            self.fighter = ctx.kit.fighter;
            self.reach = DEFAULT_REACH;
            self.reach_seen = [[false; 6]; 2];
            self.last_normal = None;
            self.shot_range = None;
            self.shot_far = 0;
            self.plan = Plan::default();
            self.settle = 0;
            self.judge = game::Judge::default();
            self.judged = Judged::default();
            self.game = None;
            self.route = None;
            self.recent_routes = [0; 4];
            self.route_drops = [0; ROUTE_SLOTS];
            self.route_check = None;
            self.route_outcome = None;
            self.focus = Focus::default();
            self.action_frame = 0;
            self.last_view = Percept::default();
            self.new_match();
        }
        let stronger = self.stronger(ctx);
        // A mission that allows only normals and throws: the stronger play
        // keeps to it (SC23, `#215`).
        // Its fighter's specials as the stronger play reads them (SC15,
        // `#207`).
        let limited;
        let ctx = if stronger && (ctx.pure_footsies || ctx.strong_kit.is_some() || ctx.baseline.is_some()) {
            limited = ctx.for_stronger();
            &limited
        } else {
            ctx
        };
        let delay = ctx.level.reaction();
        let view = step.checked_sub(delay).and_then(|f| self.shown(f, step, ctx.level, stronger));
        let pad = match view {
            Some((view, action_frame)) if in_fight && view.fighting => {
                // The first round's start is a new match, in every mode:
                // nothing of the last is remembered (SC9, `#201`).
                if self.out_of_fight >= BETWEEN_FIGHTS && ctx.round == 0 {
                    self.new_match();
                }
                self.out_of_fight = 0;
                let before = if stronger && self.last_view.frame == view.frame - 1 && self.last_view.frame != 0 {
                    Some(self.last_view)
                } else {
                    self.shown(view.frame - 1, step - 1, ctx.level, stronger).map(|(b, _)| b)
                }
                .filter(|b| b.fighting);
                self.action_frame = action_frame;
                self.last_view = view;
                let pad = self.decide(step, view, before, ctx);
                if stronger {
                    let watched = self.focus;
                    // Over a knocked-down opponent it watches his wake-up,
                    // whatever step it is on (walking in or timing an attack).
                    self.focus = if self.judged.situation == Situation::OpponentKnockedDown {
                        focus::situation_focus(Situation::OpponentKnockedDown)
                    } else {
                        self.game
                            .and_then(|run| {
                                game::plans(ctx.kit.fighter, ctx.family)
                                    .get(usize::from(run.plan))?
                                    .steps
                                    .get(usize::from(run.step))
                            })
                            .map_or_else(|| focus::situation_focus(self.judged.situation), focus::step_focus)
                    };
                    // A jump its memory expects is what it watches for.
                    if let Some(focus) = self.habits.focus(view.frame, self.judged.situation, view.gap(), ctx.level) {
                        self.focus = focus;
                    }
                    // A jump it was watching for and has seen stays watched
                    // until he lands: switching away would hide it again
                    // until it is the slower delay old, and an anti-air
                    // waiting for it would let it go (SC9, `#201`).
                    if watched == Focus::Jump && view.foe.jumping() {
                        self.focus = Focus::Jump;
                    }
                }
                pad
            }
            _ => {
                self.plan = Plan::default();
                // What came of the round's last plan is kept.
                self.end_game_plan();
                self.route = None;
                self.route_outcome = None;
                self.reset_wait = None;
                self.judge = game::Judge::default();
                self.judged = Judged::default();
                self.focus = Focus::default();
                self.action_frame = 0;
                self.last_view = Percept::default();
                // A new round starts with its pressure count spent.
                self.pressure_run = 0;
                self.out_of_fight = self.out_of_fight.saturating_add(1);
                0
            }
        };
        // Easy keeps its meter even when two separate special commands
        // accidentally complete a Critical Art motion across plans. Only
        // its own recognizer and facing are read; the native command and
        // meter are left intact and the offending button is released.
        let pad = if stronger && !ctx.level.spends_meter() {
            let facing_left = self.seen(step.saturating_sub(1)).is_some_and(|view| view.me.facing_left);
            ctx.era.map_or(pad, |tactics| tactics.sf6_keep_meter(tactics.sf5_keep_meter(pad, facing_left), facing_left))
        } else {
            pad
        };
        self.pad = pad;
        pad
    }

    /// What frame `frame` showed, as the CPU sees it on step `step`, and the
    /// frame its view of the opponent's actions comes from. The old play
    /// sees the frame as it was. The stronger play (SC7, `#199`) sees a new
    /// action of the opponent's that is not its focus only once it is the
    /// slower delay old: until then the opponent's pose, height, attack box
    /// and projectile are as they were the frame before that action began,
    /// moved with him to where the frame shows him. Positions, health and
    /// its own fighter are as the frame shows them. A new action that is its
    /// focus shows at once, and with it everything up to that frame. Its
    /// focus is the one it holds as it decides: an action it comes to watch
    /// for, or one before its focus showing, can show before the slower
    /// delay.
    fn shown(&self, frame: u32, step: u32, level: Level, stronger: bool) -> Option<(Percept, u32)> {
        let mut view = self.seen(frame)?;
        if !stronger {
            return Some((view, frame));
        }
        let engage = level.throw_engage();
        let from = step.saturating_sub(level.unfocused_reaction()) + 1;
        let mut hidden_from = None;
        for f in from.max(1)..=frame {
            let (Some(v), Some(b)) = (self.seen(f), self.seen(f - 1)) else { continue };
            if !v.fighting || !b.fighting {
                continue;
            }
            if self.event_shows(self.focus, &v, &b, engage) {
                hidden_from = None;
            } else if hidden_from.is_none() && Focus::ALL.iter().any(|&event| self.event_shows(event, &v, &b, engage)) {
                hidden_from = Some(f);
            }
        }
        // Guarding and hit reactions are what came of its own attacks:
        // those it sees at the focused delay, whatever else is hidden.
        let outcome = view.foe.guarding() || view.foe.reacting();
        let Some(f) = hidden_from.filter(|_| !outcome) else { return Some((view, frame)) };
        let was = self.seen(f - 1)?;
        let dx = view.foe.x - was.foe.x;
        view.foe = Body {
            x: view.foe.x,
            health: view.foe.health,
            present: view.foe.present,
            front: was.foe.front + dx,
            hurt: (was.foe.hurt.0 + dx, was.foe.hurt.1 + dx),
            ..was.foe
        };
        (view.shot, view.shot_x, view.shot_half) = (was.shot, was.shot_x, was.shot_half);
        Some((view, f - 1))
    }

    /// `view`, the frame after `before`, shows `event` starting. A whiff is
    /// an attack whose box goes without having reached the CPU's fighter
    /// on any frame of it the CPU still holds, as judging counts one.
    fn event_shows(&self, event: Focus, view: &Percept, before: &Percept, engage: i16) -> bool {
        event.shows(view, before, engage) && (event != Focus::Whiff || !self.attack_met(view.frame))
    }

    /// The opponent's attack drawn on frame `frame` reached the CPU's
    /// fighter (or hit him) on some earlier frame of it.
    fn attack_met(&self, frame: u32) -> bool {
        let Some(now) = self.seen(frame) else { return false };
        let mut f = frame;
        while f > 1 && frame - f < RING as u32 - 1 {
            f -= 1;
            let Some(v) = self.seen(f) else { return false };
            if !v.foe.attacking() || v.foe.move_id != now.foe.move_id {
                return false;
            }
            if v.me.reacting() {
                return true;
            }
            if v.foe.active {
                let toward_me = if v.me.x >= v.foe.x { 1 } else { -1 };
                let near = if toward_me > 0 { v.me.hurt.0 } else { v.me.hurt.1 };
                if (v.foe.front - near) * toward_me >= 0 {
                    return true;
                }
            }
        }
        false
    }

    fn decide(&mut self, step: u32, view: Percept, before: Option<Percept>, ctx: &Context) -> u16 {
        if !self.seeded {
            // Seeded from the fight itself: the same fight, the same seed.
            let mut seed = 0x9e37_79b9_7f4a_7c15u64 ^ u64::from(view.frame);
            for v in [view.me.x, view.foe.x, view.me.health as i16, view.foe.health as i16, ctx.level.reaction() as i16] {
                seed = seed.rotate_left(13) ^ (v as u16 as u64).wrapping_mul(0x100_0000_01b3);
            }
            self.rng = Rng(seed | 1);
            self.seeded = true;
        }
        let stronger = self.stronger(ctx);
        // Its own hit reaction is felt now; opponent/contact evidence
        // remains delayed. Other families keep their original feedback.
        let own_reacting = if stronger && ctx.baseline_rules() {
            self.felt(step, view).me.reacting()
        } else {
            view.me.reacting()
        };
        self.learn(view, before, stronger && ctx.baseline_rules());
        // Whether its pads come from a game plan, before being hit ends it.
        let in_plan = self.game.is_some();
        if stronger {
            let was_blocked = self.judged.situation == Situation::ItWasBlocked;
            self.judged = self.judge.judge(view, before, self.longest_reach());
            if ctx.own_blockstun {
                self.judge.felt_blockstun(step);
            }
            if let Some(before) = before {
                let blocked = !was_blocked && self.judged.situation == Situation::ItWasBlocked;
                let shots = [ctx.kit.projectile, ctx.kit.projectile_alt].map(|entry| entry.map(|entry| entry.produces));
                self.habits.watch(&view, &before, &shots, blocked);
                if !view.foe.jumping() {
                    self.jump_met = false;
                } else if !before.foe.jumping() {
                    // The jump's first frame among those it has now been
                    // shown: a jump outside its focus shows late, with its
                    // first frames behind it.
                    let mut began = view.frame;
                    while view.frame - began < RING as u32 - 2 && self.seen(began - 1).is_some_and(|was| was.foe.jumping()) {
                        began -= 1;
                    }
                    self.jump_began = began;
                }
            }
            if own_reacting {
                // Being hit ends whatever it was following.
                self.end_game_plan();
            }
            self.credit_answer(step, view);
        }
        if own_reacting {
            self.route = None;
            self.reset_wait = None;
            self.bait_until = 0;
        }
        if let Some(mut wait) = self.reset_wait.filter(|_| stronger) {
            // A throw or a low goes on only once it has seen the route hit,
            // from its fighter's first free frame; seen nothing by his first
            // free frame (cut after a guard or a whiff), it ends as a slip
            // does. A bait's guard is up already. The game plan that took
            // the route goes on from what it saw.
            wait.hit |= view.foe.reacting();
            let since = step.saturating_sub(wait.cut);
            let free_from = u32::from(wait.reset.free.saturating_sub(wait.reset.ahead));
            // Knocked down, he is not where the cut was proven (on his
            // feet): a throw or a low is let go at once.
            let down = game::knocked_down(&view.foe);
            let go = wait.hit && !down && since >= free_from;
            if down && wait.reset.kind != ResetKind::Bait {
                self.reset_wait = None;
                self.route_outcome = Some(game::Outcome::Hit);
                self.plan = Plan::default();
            } else if go || since >= u32::from(wait.reset.free) {
                self.reset_wait = None;
                self.route_outcome = Some(if wait.hit { game::Outcome::Hit } else { game::Outcome::Nothing });
                if go && wait.reset.kind != ResetKind::Bait {
                    let pads = self.reset_pads(wait.reset, if wait.facing_right { RIGHT } else { LEFT }, since as usize);
                    self.settle = step + pads.len() as u32;
                    self.plan = Plan::of(PlanKind::Act, &pads);
                }
            } else {
                self.reset_wait = Some(wait);
            }
        }
        if let Some(RouteCheck { index, at, hit }) = self.route_check.filter(|_| stronger) {
            let hit = hit || view.foe.reacting();
            if view.frame >= at {
                // Once it hit: landed if he is still in a hit reaction or
                // knocked down, dropped if he is free again. A route that
                // never hit dropped nothing.
                if let Some(drops) = self.route_drops.get_mut(usize::from(index)).filter(|_| hit) {
                    *drops = if view.foe.reacting() { drops.saturating_sub(1) } else { drops.saturating_add(1).min(game::DROPS_KEPT) };
                }
                if hit && usize::from(index) < 32 {
                    let bit = 1u32 << index;
                    self.route_landed = if view.foe.reacting() { self.route_landed | bit } else { self.route_landed & !bit };
                }
                self.route_check = None;
            } else {
                self.route_check = Some(RouteCheck { index, at, hit });
            }
        }
        if self.route.is_some() {
            // A route is entered to its end, or to where it is cut.
            return self.next_route_pad(step, view, ctx);
        }
        if let Some(before) = before {
            // A combo reset's bait (SC10, `#202`) holds its guard for its
            // time: nothing seen cuts it short. The guard itself meets his
            // attack (its Drive Parry included, after which his reversal is
            // a whiff to punish).
            let holding_bait = stronger && matches!(self.plan.kind, PlanKind::Guard { .. }) && step < self.bait_until;
            let interruptible = self.plan.interruptible() && !holding_bait;
            // The stronger play leaves a knocked-down opponent to its
            // knockdown plans: the era kit's answer to a hit (a super on
            // the hit confirm) would only whiff over him.
            let foe_down = stronger && self.judged.situation == Situation::OpponentKnockedDown;
            // A ground attack he starts out of its reach is left to whiff,
            // not guarded or parried: the CPU stays free to move and to
            // punish it.
            let baiting = stronger && self.out_of_reach(step, view);
            let spend_super = self.spends_super(ctx, view);
            // Nor does the era kit answer anything while a bait's guard is
            // held (its hit confirm would give up the bait it chose).
            let felt = self.felt(step, view);
            let era_response = ctx.era.filter(|tactics| !foe_down && !baiting && !holding_bait && (interruptible || tactics.can_interrupt()))
                .and_then(|tactics| tactics.react(&mut self.rng, ctx.level, view, before, felt, stronger, spend_super));
            let mut answered = false;
            if let Some(mut plan) = era_response {
                if stronger {
                    plan.end_at_last_press();
                    plan.answer = true;
                }
                self.plan = plan;
                self.end_game_plan();
                answered = true;
            } else if interruptible {
                let answer = self.react(step, view, before, ctx).filter(|plan| !(baiting && matches!(plan.kind, PlanKind::Guard { .. })));
                if let Some(plan) = answer {
                    // A threat it answers ends the game plan it was following;
                    // an anti-air that lands is the plan's win.
                    let anti_air = matches!(plan.kind, PlanKind::Await { .. });
                    self.plan = plan;
                    if anti_air {
                        self.answer_ends_game_plan(step);
                    } else {
                        self.end_game_plan();
                    }
                    answered = true;
                }
            }
            // A game plan's attack not yet begun is not pressed into an
            // attack it has just seen start: it guards instead. (Not an
            // answer it has just chosen to that attack.)
            let foe_started = view.foe.attacking() && (!before.foe.attacking() || before.foe.move_id != view.foe.move_id);
            if stronger && !answered && (in_plan || self.game.is_some()) && self.plan.press_to_come() && foe_started {
                self.plan = Plan::guard(view.foe.low_attack(), 0);
                self.end_game_plan();
            }
        }
        if self.route.is_some() {
            // An answer that is a route: a punish.
            return self.next_route_pad(step, view, ctx);
        }
        if self.plan.done() {
            if let Some(plan) = self.follow_game_plan(step, view, ctx) {
                self.plan = plan;
                if self.route.is_some() {
                    // A step that is a route: its first pad is this frame's.
                    return self.next_route_pad(step, view, ctx);
                }
                return self.next_pad(step, view, ctx);
            }
            if !self.ready(step, view, stronger) {
                return 0;
            }
            // The stronger play takes up a game plan first; the era kit's
            // own neutral choices come where none is taken (SC8, `#200`).
            if stronger && self.start_game_plan(step, view, ctx) {
                self.plan = self.follow_game_plan(step, view, ctx).unwrap_or_else(|| self.choose(step, view, ctx));
            } else if stronger && self.backs_off(ctx) {
                self.plan = self.plan_for(Choice::Retreat, step, view, ctx, false);
            } else {
                let tactics = ctx.era.map(|tactics| if stronger { tactics.spending_safely(ctx.level) } else { tactics });
                let era = tactics.and_then(|tactics| tactics.neutral(&mut self.rng, ctx.level, view)).map(|mut plan| {
                    if stronger {
                        plan.end_at_last_press();
                    }
                    plan
                });
                self.plan = match era {
                    Some(plan) => plan,
                    None => self.choose(step, view, ctx),
                };
            }
        }
        if self.route.is_some() {
            // A game plan's step that is a route.
            return self.next_route_pad(step, view, ctx);
        }
        self.next_pad(step, view, ctx)
    }

    /// The next pad of the route being entered, turned to face the
    /// opponent as it faced him when the route began. What came of it is
    /// kept for the game plan that chose it.
    fn next_route_pad(&mut self, step: u32, view: Percept, ctx: &Context) -> u16 {
        let stronger = self.stronger(ctx);
        let Some(mut run) = self.route else { return 0 };
        if stronger && run.walking() && self.judge.coming(view) {
            // Its walk in, before the route's first press, stops for an
            // attack or a projectile it sees coming: the route is dropped.
            self.route = None;
            self.route_outcome = Some(game::Outcome::Nothing);
            return 0;
        }
        run.watch(view);
        let pad = run.pad();
        match pad {
            Some(pad) => {
                run.at += 1;
                self.route = Some(run);
                pad
            }
            None => {
                self.route = None;
                self.route_outcome = Some(run.outcome());
                // A combo entered in full is watched to its end: whether he
                // is still reeling once its last hit has had time to land.
                if run.combo_in_full() {
                    self.route_check = Some(RouteCheck {
                        index: run.index,
                        at: step + game::ROUTE_WATCH,
                        // A hit seen earlier in an SF4/SF5 route still counts
                        // when its late continuation whiffs or is guarded.
                        hit: stronger && run.outcome() == game::Outcome::Hit
                            && matches!(ctx.family, Some(crate::era::EraFamily::Sf4 | crate::era::EraFamily::Sf5)),
                    });
                }
                // Cut on purpose: its fighter waits out its own recovery
                // (a bait guarding through it) and then goes on with the
                // reset, if it has seen the route hit by then. What came of
                // the route is known only then.
                if let Some(reset) = run.reset {
                    // Still while it waits, to his first free frame at most.
                    let wait = usize::from(reset.free);
                    self.reset_wait = Some(game::ResetWait { reset, cut: step, facing_right: run.facing_right(), hit: false });
                    self.route_outcome = None;
                    self.last_normal = None;
                    self.plan = if reset.kind == ResetKind::Bait { self.bait_plan(reset, step) } else { Plan::of(PlanKind::Act, &vec![0; wait][..wait - 1]) };
                }
                0
            }
        }
    }

    /// Take a route from the fighter's route book that starts in one of
    /// `reaches` and that the moment allows: the rules in force, the
    /// opponent's stance, its meters, the corner, and the distance, closed
    /// by a walk of at most `walk` frames. The level's decision quality
    /// takes the best scored, as for its other choices; a slip of its
    /// execution cuts the route short. False when none is allowed.
    fn take_route(&mut self, ctx: &Context, view: Percept, reaches: &[crate::route_book::Reach], walk: u8) -> bool {
        let (Some(fighter), Some((revision, loadout))) = (ctx.kit.fighter, ctx.style) else { return false };
        let Some(book) = crate::route_book::book(fighter, revision, loadout) else { return false };
        let foe = ctx.foe.map(|(who, style)| crate::route_book::opponent_key(who, style));
        let moment = game::Moment {
            reaches,
            // Later eras bring offense rules even under a mission that
            // sets the kit aside. Baseline styles keep their own toggles.
            era: ctx.era_offense(),
            normals_only: ctx.pure_footsies,
            true_combos: ctx.true_combos,
            cancels: ctx.cancels,
            crouching: view.foe.crouching(),
            meters: ctx.meters,
            corner: self.judged.corner,
            gap: view.gap(),
            walk,
            foe_health: view.foe.health,
            foe_max_sub_hp: ctx.foe_max_sub_hp,
            spend_super: self.spends_super(ctx, view),
            drive_reserve: if self.stronger(ctx) { ctx.level.drive_kept() } else { 0 },
            avoid_burnout: self.stronger(ctx),
            foe: foe.as_deref(),
        };
        let mut options: Vec<(usize, i32)> = Vec::new();
        // The level's longest route, and at Easy none that spends meter
        // (SC11, `#203`).
        let stronger = self.stronger(ctx);
        let within_level = |route: &crate::route_book::Route| {
            !stronger || (route.hits <= ctx.level.route_hits()
                && (ctx.level.spends_meter() || route.super_spent + route.drive_spent == 0)
                // Native books do not record SF4 Revenge or SF5 V meter;
                // check their real commands against the current kit too.
                && ctx.era.is_none_or(|tactics| tactics.sf4_route_ready(route, ctx.level, ctx.meters[0])
                    && tactics.sf5_route_ready(route, ctx.level, ctx.meters[0])))
        };
        for (index, route) in book.routes.iter().enumerate() {
            let drops = self.route_drops.get(index).copied().unwrap_or(0);
            if moment.allows(route) && within_level(route) && drops < game::DROPS_KEPT {
                // A route taken lately is passed over for variety, unless
                // it landed whole last time at a level that repeats what
                // keeps working.
                let landed = index < 32 && self.route_landed & (1 << index) != 0;
                let recent = self.recent_routes.contains(&(index as u8 + 1)) && !(landed && game::leans_on_what_works(ctx.level));
                // A route it has seen drop in this fight scores lower.
                let dropped = i32::from(drops) * game::DROP_COST;
                options.push((index, moment.score(route, recent) - dropped + self.rng.below(12) as i32));
            }
        }
        if options.is_empty() {
            return false;
        }
        options.sort_by_key(|o| std::cmp::Reverse(o.1));
        let pick = if options.len() == 1 || self.rng.chance(ctx.level.quality()) {
            options[0].0
        } else {
            options[1 + self.rng.below((options.len() - 1).min(3) as u32) as usize].0
        };
        let route = &book.routes[pick];
        // Execution: a slip drops the route after one of its steps.
        let pressure = view.me.health < view.foe.health || view.dist() < 60;
        let clean = ctx.level.clean(pressure);
        let mut stop = if route.steps.len() >= 2 && !self.rng.chance(clean) {
            route.step_start(1 + self.rng.below(route.steps.len() as u32 - 1) as usize)
        } else {
            route.len()
        };
        // A combo reset (SC10, `#202`): cut on purpose at a marked point.
        let reset = if self.stronger(ctx) { self.choose_reset(ctx, route, &moment, stop) } else { None };
        if let Some((at, _)) = reset {
            stop = at;
        }
        let walk = moment.walk_frames(route);
        self.recent_routes.rotate_right(1);
        self.recent_routes[0] = pick as u8 + 1;
        let mut run = game::RouteRun::new(fighter, revision, loadout, pick as u8, walk, stop as u16, !view.me.facing_left);
        run.reset = reset.map(|(_, reset)| reset);
        self.route = Some(run);
        self.route_outcome = None;
        self.last_normal = None;
        self.plan = Plan::default();
        true
    }

    /// Whether to cut `route` on purpose at one of its marked points (SC10,
    /// `#202`), and into what: the cut's script frame and the reset. Never a
    /// route that might win the round (on the health its bar shows), and
    /// never past where a slip of
    /// execution already cuts it (`stop`). How often is the level's
    /// [`Level::reset_chance`], shifted by the habit memory, which also
    /// weighs what it cuts into. A throw or a low is taken only where its
    /// walk in and press fit one plan, a bait only near him.
    fn choose_reset(&mut self, ctx: &Context, route: &crate::route_book::Route, moment: &game::Moment, stop: usize) -> Option<(usize, game::Reset)> {
        if route.cuts.is_empty() || moment.may_win_round(route) {
            return None;
        }
        let lean = self.habits.reset_lean(ctx.level);
        let chance = ctx.level.reset_chance() as i32 + lean.chance;
        let cuts: Vec<&crate::route_book::Cut> = route.cuts.iter().filter(|cut| route.step_start(usize::from(cut.step)) <= stop).collect();
        if chance <= 0 || cuts.is_empty() || !self.rng.chance(chance as u32) {
            return None;
        }
        let cut = *cuts[self.rng.below(cuts.len() as u32) as usize];
        let reset = |kind| game::Reset { kind, free: cut.free, ahead: cut.ahead, gap: cut.gap };
        let fits = |kind| self.reset_pads(reset(kind), 0, 0).len() <= PLAN_CAP;
        let weights = [
            (ResetKind::Throw, if (!ctx.baseline_rules() || ctx.throws) && fits(ResetKind::Throw) { (RESET_THROW + lean.throw).max(0) } else { 0 }),
            (ResetKind::Low, if fits(ResetKind::Low) { (RESET_LOW + lean.low).max(0) } else { 0 }),
            (ResetKind::Bait, if cut.gap <= BAIT_NEAR { (RESET_BAIT + lean.bait).max(0) } else { 0 }),
        ];
        let total: i32 = weights.iter().map(|w| w.1).sum();
        if total <= 0 {
            return None;
        }
        let mut roll = self.rng.below(total as u32) as i32;
        let kind = weights.iter().find(|w| {
            roll -= w.1;
            roll < 0
        })?.0;
        Some((route.step_start(usize::from(cut.step)), reset(kind)))
    }

    /// The pads of a throw or low reset from the frame after its route is cut,
    /// toward the opponent's side `fwd` (empty for a bait, which is a
    /// guard): it waits for its fighter to be free, walks in as far as the
    /// book says he will be, and presses so the throw comes once he is out
    /// of his hit reaction (no throw lands in one) and the low meets him as
    /// he comes out of it. Throw buttons as the stronger play's throw.
    ///
    /// `from` is the frame after the cut the pads begin on: a reset that
    /// goes on later than its fighter is free walks in from then, its press
    /// kept no earlier than the book's timing.
    fn reset_pads(&self, reset: game::Reset, fwd: u16, from: usize) -> Vec<u16> {
        let free = usize::from(reset.free).saturating_sub(from);
        let mut pads = vec![0u16; usize::from(reset.free.saturating_sub(reset.ahead)).saturating_sub(from)];
        match reset.kind {
            ResetKind::Throw => {
                let walk = (reset.gap - RESET_THROW_GAP).max(0) as usize;
                pads.extend(std::iter::repeat_n(fwd, walk.div_ceil(2)));
                // The engine throws only from a standing fighter.
                pads.extend([0, 0]);
                pads.resize(pads.len().max(free), 0);
                pads.extend([fwd | button(2), fwd | button(2), 0]);
            }
            ResetKind::Low => {
                let walk = (reset.gap - (self.reach[1][3] - 4)).max(0) as usize;
                pads.extend(std::iter::repeat_n(fwd, walk.div_ceil(2)));
                pads.resize(pads.len().max(free.saturating_sub(RESET_LOW_LEAD)), 0);
                pads.extend([DOWN, DOWN | button(3), DOWN | button(3), DOWN | button(3), DOWN, 0]);
            }
            ResetKind::Bait => pads.clear(),
        }
        pads
    }

    /// A bait's guard, from the frame its route is cut: held past his
    /// recovery for a reversal to show; crouching, as over his rise (a
    /// standing guard with nothing coming is a walk back).
    fn bait_plan(&mut self, reset: game::Reset, step: u32) -> Plan {
        let held = u32::from(reset.free) + game::BAIT_FRAMES;
        self.bait_until = step + held;
        Plan::guard(true, held as u16)
    }

    /// Its longest normal, as far as it knows.
    /// Whether a super may be spent now other than to win the round
    /// (which always may): the old play spends whenever it can; the
    /// stronger play (SC8, `#200`) while it is behind on health as its view
    /// shows the bars, with its Super meter full, where keeping it gains
    /// nothing, or in a round that could end the match, after which meter
    /// kept is wasted.
    fn spends_super(&self, ctx: &Context, view: Percept) -> bool {
        !self.stronger(ctx) || view.me.health < view.foe.health || ctx.meter_full || ctx.round_decides_match
    }

    fn longest_reach(&self) -> i16 {
        self.reach[0].iter().chain(self.reach[1].iter()).copied().max().unwrap_or(60)
    }

    /// Take up a game plan for the situation, if its fighter and era style
    /// have one the level takes and the level's decision quality chooses it
    /// over the single choices. Among several the best score wins, after a
    /// little noise. A pressure plan past the level's run of them is not
    /// taken ([`Level::pressure_plans`]): it backs off instead
    /// ([`Self::backs_off`]).
    fn start_game_plan(&mut self, step: u32, view: Percept, ctx: &Context) -> bool {
        let table = game::plans(ctx.kit.fighter, ctx.family);
        let baseline = self.stronger(ctx) && ctx.baseline_rules();
        let pressure = |plan: &game::GamePlan| plan.pressure() || (baseline && plan.situation == Situation::ItHit);
        let mut best: Option<(u8, i32)> = None;
        let gap = view.gap();
        let felt = self.felt(step, view);
        let pressed_enough = self.pressed_enough(ctx.level);
        for (index, plan) in table.iter().enumerate() {
            if plan.situation == self.judged.situation
                && (!plan.cornered_only || self.judged.foe_cornered)
                && (plan.gap.0..=plan.gap.1).contains(&gap)
                && plan.for_level(ctx.level)
                && !(pressure(plan) && pressed_enough)
                // A cast plan cannot loop on a rush it has no Drive to
                // execute. Ryu keeps his established selection unchanged.
                && !(ctx.sf6_cast() && plan.steps.iter().any(|step| step.act == game::Act::Rush)
                    && !ctx.era.is_some_and(|tactics| tactics.spending_safely(ctx.level).sf6_rush_ready()))
                && !(ctx.sf6_cast() && plan.steps.iter().any(|step| step.act == game::Act::Era(game::EraAct::Super))
                    && !self.spends_super(ctx, view))
                // Under a mission that allows only normals and throws, no
                // plan that needs a special or an era command, or a route
                // its book has none of plain normals for (SC23, `#215`).
                && !(ctx.pure_footsies && !plan.keeps_to_normals(|reach| normals_route(ctx, reach)))
                // Nor one needing a projectile, a tool or a route its
                // fighter has not got (SC15, `#207`), a tool it cannot use
                // from here, or an era tool not there now (SC16, `#208`).
                // Baseline styles keep combos, cancels and throws
                // independent. Do not take a plan whose ground route is
                // forbidden or absent, including bookless SF1+ Geki.
                && (!ctx.baseline_rules() || plan.steps.iter().all(|step| match step.act {
                    game::Act::Do(Choice::Throw) => ctx.throws,
                    game::Act::Do(Choice::Combo) => ctx.true_combos,
                    game::Act::Do(Choice::Cancel { .. }) => ctx.cancels,
                    game::Act::Do(Choice::Special(role)) => ctx.kit.by_role(role).into_iter().any(|entry| entry.is_some()),
                    game::Act::Route(reach) => baseline_route(ctx, reach),
                    _ => true,
                }))
                && plan.playable(ctx.kit.projectile.is_some(), |tool| self.tool_ready(tool, &felt, ctx), |reach| book_route(ctx, reach), |act| era_ready(act, ctx))
            {
                // A cornered opponent is pressed harder.
                let corner = if self.judged.foe_cornered { plan.corner_bonus } else { 0 };
                // What its memory of him says, and what has worked.
                let habit = self.habits.plan_bonus(plan.lean, ctx.level);
                if plan.score + habit <= 0 {
                    // A plan taken only for a habit he has not shown is not
                    // weighed at all (nor draws on its random numbers).
                    continue;
                }
                let worked = self.plan_worked.get(index).copied().unwrap_or(0);
                let repeat = game::repeat_score(ctx.level, worked, usize::from(self.last_plan) == index + 1);
                let score = plan.score + corner + habit + repeat + self.rng.below(12) as i32;
                if best.is_none_or(|(_, top)| score > top) {
                    best = Some((index as u8, score));
                }
            }
        }
        let Some((plan, _)) = best else { return false };
        if !self.rng.chance(ctx.level.quality()) {
            return false;
        }
        // How often its fighter takes a plan at all in this style (SC15,
        // `#207`): a fighter whose game is its specials leaves more to its
        // single choices.
        let rate = game::plan_rate(ctx.kit.fighter, ctx.family);
        if rate < 100 && !self.rng.chance(rate) {
            return false;
        }
        self.pressure_run = if pressure(&table[usize::from(plan)]) { self.pressure_run.saturating_add(1) } else { 0 };
        self.game = Some(game::Running { plan, step: 0, acted: false, pressed: 0, began: step, hit: false });
        true
    }

    /// The stronger play has stayed on the opponent for as many pressure
    /// plans as its level takes, and the situation still asks for another
    /// (after a guarded attack, on a knocked-down or cornered opponent): it
    /// backs off and gives him room (SC11, `#203`), and the count starts
    /// again. Only for a pair with game plans.
    fn backs_off(&mut self, ctx: &Context) -> bool {
        let pressing = match self.judged.situation {
            Situation::ItWasBlocked | Situation::OpponentKnockedDown => true,
            Situation::ItHit => ctx.baseline_rules(),
            Situation::NeutralFar => self.judged.foe_cornered,
            _ => false,
        };
        if !pressing || !self.pressed_enough(ctx.level) || game::plans(ctx.kit.fighter, ctx.family).is_empty() {
            return false;
        }
        self.pressure_run = 0;
        true
    }

    /// It has taken its level's run of pressure plans; never at a level
    /// with no limit.
    fn pressed_enough(&self, level: Level) -> bool {
        let cap = level.pressure_plans();
        cap != u8::MAX && self.pressure_run >= cap
    }

    /// A lingering reaction to the preceding hit is not a new attack's hit.
    /// Baseline pressure uses only history already in its delayed view;
    /// contact can consume the active box before that view is sampled.
    fn attack_outcome(&self, run: game::Running, view: Percept, reach: i16, ctx: &Context) -> Option<game::Outcome> {
        if !self.stronger(ctx) || !ctx.baseline_rules() {
            return run.outcome(view, reach);
        }
        let hit_seen = self.seen(run.pressed).is_some_and(|at_press| at_press.fighting && at_press.foe.present
            && (!at_press.foe.reacting() || view.foe.health < at_press.foe.health));
        run.outcome_with_hit(view, reach, hit_seen)
    }

    /// The next pads of the game plan being followed, when its last ones
    /// are over: the step's action, a frame of waiting to see what came of
    /// it, or the action of the step that outcome leads to. `None` when no
    /// plan is being followed, or it has just ended.
    fn follow_game_plan(&mut self, step: u32, view: Percept, ctx: &Context) -> Option<Plan> {
        use game::{Act, Outcome};
        let stronger = self.stronger(ctx);
        let mut run = self.game?;
        if step.saturating_sub(run.began) > game::PLAN_FRAMES {
            // Something kept it from finishing: let go of the plan.
            self.end_game_plan();
            return None;
        }
        let table = game::plans(ctx.kit.fighter, ctx.family);
        let wait = Plan::of(PlanKind::Idle, &[0]);
        // A plan has few steps, and each pass either returns or moves on one.
        for _ in 0..8 {
            let Some(at) = table.get(usize::from(run.plan)).and_then(|plan| plan.steps.get(usize::from(run.step))) else { break };
            // It does not walk or attack into an attack or a projectile it
            // has seen start: the plan ends, and its answer takes over.
            // (A reach is walked a few frames at a time: each walk is one.)
            let commits = (!run.acted || matches!(at.act, Act::Reach { .. })) && at.act.commits();
            // A whiffed attack still has its attack pose during recovery.
            // A dedicated SF4/SF5 finisher punish may meet that seen recovery;
            // an active attack or a projectile still ends the plan.
            let finisher_punish = stronger && (matches!(ctx.family, Some(crate::era::EraFamily::Sf4 | crate::era::EraFamily::Sf5)) || ctx.sf6_cast())
                && self.judged.situation == Situation::OpponentWhiffed
                && table[usize::from(run.plan)].situation == Situation::OpponentWhiffed
                && matches!(at.act, Act::Era(game::EraAct::Super | game::EraAct::Ultra | game::EraAct::VTrigger))
                && !view.foe.active;
            let approaching = matches!(at.act, Act::Reach { .. } | Act::Do(Choice::Approach))
                && self.may_approach_attack(step, view, ctx);
            if commits && ((view.foe.attacking() && !finisher_punish && !approaching) || view.shot_coming()) {
                break;
            }
            // An attack thrown at a knocked-down opponent is wasted: a plan
            // for another situation ends there, and the knockdown's plans,
            // which time their attack to his rise, take over.
            if commits && !at.act.meets_the_rise() && self.judged.situation == Situation::OpponentKnockedDown {
                break;
            }
            // Against a cornered opponent it gives no ground: where a plan
            // would back off and guard, it ends, and the pressure goes on
            // with the next plan for the situation.
            if !run.acted && at.act == Act::Do(Choice::Retreat) && self.judged.foe_cornered {
                break;
            }
            let outcome = match (at.act, run.acted) {
                (Act::Edge | Act::Space(_), acted) => {
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    // How far off its mark it stands, positive too far out:
                    // its own place felt as it is now, his seen late.
                    let felt = self.felt(step, view);
                    let off = match at.act {
                        Act::Space(target) => felt.gap() - target,
                        _ => felt.foe_gap() - (self.judge.foe_edge() + game::EDGE_OUT),
                    };
                    if off.abs() <= game::EDGE_NEAR || (off < 0 && self.judged.foe_cornered) || (acted && step > run.pressed + game::EDGE_FRAMES) {
                        Outcome::Nothing
                    } else {
                        // A few frames at a time, looking again after each.
                        let frames = ((off.abs() + 1) / 2).clamp(1, game::EDGE_WALK) as u32;
                        if !acted {
                            run.pressed = step;
                        }
                        run.acted = true;
                        self.game = Some(run);
                        return Some(walk(off > 0, felt, frames));
                    }
                }
                (Act::Step { forward }, false) => {
                    if !forward && self.judged.foe_cornered {
                        Outcome::Nothing
                    } else {
                        self.game = Some(run);
                        if !self.ready(step, view, stronger) {
                            return Some(wait);
                        }
                        let frames = game::STEP_FRAMES + self.rng.below(game::STEP_SPREAD);
                        run.acted = true;
                        run.pressed = step;
                        self.game = Some(run);
                        return Some(walk(forward, view, frames));
                    }
                }
                (Act::Shot, false) => {
                    if view.my_shot || ctx.kit.projectile.is_none() {
                        Outcome::Nothing
                    } else {
                        self.game = Some(run);
                        if !self.ready(step, view, stronger) {
                            return Some(wait);
                        }
                        run.acted = true;
                        run.pressed = step;
                        self.game = Some(run);
                        return Some(self.plan_for(Choice::Special(Role::Projectile), step, view, ctx, false));
                    }
                }
                (Act::Rush, false) => {
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    // Without the gauge for it (or its rules off) the plan ends.
                    let Some(plan) = ctx.era.and_then(|tactics| tactics.spending_safely(ctx.level).rush_in(view)) else { break };
                    run.acted = true;
                    run.pressed = step;
                    self.game = Some(run);
                    self.settle = step + plan.len as u32;
                    return Some(plan);
                }
                (Act::Reach { crouch, button }, acted) => {
                    let reach = self.reach[usize::from(crouch)][button] - 4;
                    let felt = self.felt(step, view);
                    // Baseline gap and direction use the same felt own pose;
                    // the opponent still comes from the delayed view.
                    let gap = felt.gap();
                    if gap > reach && !(acted && (step > run.pressed + game::EDGE_FRAMES || self.walk_stalled(step, run.pressed, ctx))) {
                        // A few frames at a time, looking again after each:
                        // the walk is reckoned at two pixels a frame, and a
                        // fighter's can cover more (Ryu's SF6 one about 2.7),
                        // so one walk the whole way in overshot the normal's
                        // reach into the opponent's (SC13, `#205`).
                        let frames = ((gap - reach + 1) / 2).clamp(1, game::EDGE_WALK) as usize;
                        let toward = if stronger && ctx.baseline_rules() { felt.toward() } else { view.toward() };
                        let forward = if toward > 0 { RIGHT } else { LEFT };
                        if !acted {
                            run.pressed = step;
                        }
                        run.acted = true;
                        self.game = Some(run);
                        return Some(Plan::of(PlanKind::Walk, &[forward; PLAN_CAP][..frames]));
                    }
                    Outcome::Nothing
                }
                (Act::JumpIn { gap }, false) => {
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    let felt = self.felt(step, view);
                    if !(gap.0..=gap.1).contains(&felt.gap()) {
                        break;
                    }
                    run.acted = true;
                    run.pressed = step;
                    let plan = self.plan_for(Choice::JumpIn, step, felt, ctx, false);
                    self.game = Some(run);
                    return Some(plan);
                }
                (Act::Do(choice), false) => {
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    run.acted = true;
                    run.pressed = step;
                    let plan = self.plan_for(choice, step, view, ctx, false);
                    if stronger && ctx.baseline_rules() && matches!(choice, Choice::Special(_)) {
                        // The direction prefix is not the special's press.
                        // Its delayed view must reach the generated button
                        // before an absent attack can count as a whiff.
                        run.pressed += plan.pads[..usize::from(plan.len)].iter()
                            .position(|pad| pad & BUTTONS != 0).unwrap_or(0) as u32;
                    }
                    self.game = Some(run);
                    return Some(plan);
                }
                (Act::Tool(tool), false) => {
                    let Some(entry) = ctx.tools.get(tool) else { break };
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    // With a wall behind a charge, as it feels its own place
                    // now: the plan ends.
                    let felt = self.felt(step, view);
                    if !self.tool_ready(tool, &felt, ctx) {
                        break;
                    }
                    // A command throw walks in to its catch range, a few
                    // frames at a time and looking again (each walk is
                    // dropped for an attack it sees coming, as any walk in
                    // is, the way it reads that walk: toward him as it sees
                    // him), and is let go if it cannot get there (SC15,
                    // `#207`). Its motion begins only inside the range, as
                    // it feels its distance then.
                    if tool == cast::Tool::CommandThrow {
                        let range = grab_range(ctx);
                        if felt.dist() > range {
                            if step > run.began + game::EDGE_FRAMES {
                                break;
                            }
                            let frames = ((felt.dist() - range + 1) / 2).clamp(1, game::EDGE_WALK) as usize;
                            let forward = if view.toward() > 0 { RIGHT } else { LEFT };
                            return Some(Plan::of(PlanKind::Walk, &[forward; PLAN_CAP][..frames]));
                        }
                    }
                    run.acted = true;
                    run.pressed = step;
                    self.game = Some(run);
                    let plan = self.tool_plan(tool, &entry, felt, ctx);
                    self.settle = step + u32::from(plan.len);
                    return Some(plan);
                }
                (Act::Era(act), false) => {
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    let Some(tactics) = ctx.era else { break };
                    if ctx.sf6_cast() && act == game::EraAct::Super && !self.spends_super(ctx, view) { break }
                    // Its own place and facing as it feels them now.
                    let felt = self.felt(step, view);
                    let forward = if felt.me.facing_left { LEFT } else { RIGHT };
                    let plan = match act {
                        game::EraAct::CustomCombo => tactics.custom_combo_ready().then(|| Plan::of(PlanKind::Act, &game::custom_combo_pads(forward))),
                        game::EraAct::CustomChain => tactics.custom_active().then(|| Plan::of(PlanKind::Act, &game::chain_pads(forward, PLAN_CAP))),
                        game::EraAct::Stance => tactics.stance_change(felt),
                        game::EraAct::Overhead => tactics.sf3_overhead(felt),
                        game::EraAct::Enhanced => tactics.sf3_ex(felt).or_else(|| tactics.sf4_ex(felt)).or_else(|| tactics.sf5_ex(felt)).or_else(|| tactics.sf6_ex(felt)),
                        game::EraAct::Focus => tactics.sf4_focus(felt),
                        game::EraAct::Super => tactics.sf4_super(felt).or_else(|| tactics.sf5_super(felt)).or_else(|| tactics.sf6_super(felt)),
                        game::EraAct::Ultra => tactics.sf4_ultra(felt),
                        game::EraAct::VSkill => tactics.sf5_skill(felt),
                        game::EraAct::VTrigger => tactics.sf5_trigger(felt, &mut self.rng),
                        game::EraAct::DriveImpact => tactics.sf6_impact(felt),
                    };
                    let Some(plan) = plan else { break };
                    run.acted = true;
                    run.pressed = step;
                    self.game = Some(run);
                    self.last_normal = None;
                    self.settle = step + u32::from(plan.len);
                    return Some(plan);
                }
                (Act::Route(reach), false) => {
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    // A route is not begun on an opponent it has just seen
                    // moving (where he is now is not where it saw him), nor
                    // before it has seen its own last attack through to its
                    // end (what that attack met is what it goes on from).
                    if !self.foe_settled(view) || view.me.attacking() {
                        return Some(wait);
                    }
                    // The distance a route needs is judged from where it
                    // feels its own fighter now.
                    let felt = self.felt(step, view);
                    if !self.take_route(ctx, felt, &[reach], game::NEUTRAL_WALK) {
                        break;
                    }
                    run.acted = true;
                    run.pressed = step;
                    self.game = Some(run);
                    return Some(Plan::default());
                }
                (Act::Meaty { crouch, button }, false) => {
                    // While he is down it walks in; pressed `MEATY_LEAD`
                    // frames before he is expected to be free, or at once
                    // if he is already up.
                    let reach = self.reach[usize::from(crouch)][button] - 4;
                    if let Some(rise) = self.judge.rise_at() {
                        if step + game::MEATY_LEAD < rise {
                            // In the SF2 era style it shimmies at the edge
                            // of the attack's reach while it waits (SC15,
                            // `#207`), as a player does over a long
                            // knockdown, rather than standing over him.
                            let shimmy = (ctx.family == Some(crate::era::EraFamily::Sf2)).then(|| rise - step - game::MEATY_LEAD);
                            return Some(self.close_in(run, step, view, reach, shimmy, ctx));
                        }
                    }
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    if self.felt(step, view).gap() > reach + 4 {
                        // Out of reach as he rises: the moment is lost.
                        Outcome::Nothing
                    } else {
                        run.acted = true;
                        run.pressed = step;
                        self.game = Some(run);
                        return Some(self.plan_for(Choice::Normal { crouch, button }, step, view, ctx, false));
                    }
                }
                // Pressed into his rise: what it met shows once he is up.
                // Down again after that (the attack knocked him down) is
                // its hit, read below.
                (Act::Meaty { .. }, true)
                    if game::knocked_down(&view.foe) && view.frame > run.pressed && view.frame <= run.pressed + game::MEATY_LEAD =>
                {
                    self.game = Some(run);
                    return Some(wait);
                }
                (Act::Meaty { crouch, button }, true) => match self.attack_outcome(run, view, self.reach[usize::from(crouch)][button], ctx) {
                    Some(outcome) => outcome,
                    None => return Some(wait),
                },
                (Act::GuardRise, false) => {
                    let close = self.reach[0][1] - 4;
                    let rise = self.judge.rise_at();
                    if let Some(rise) = rise {
                        if step + game::GUARD_LEAD < rise {
                            return Some(self.close_in(run, step, view, close, None, ctx));
                        }
                    }
                    let until = rise.unwrap_or(step) + game::BAIT_FRAMES;
                    run.acted = true;
                    run.pressed = step;
                    self.game = Some(run);
                    let frames = until.saturating_sub(step).clamp(8, 120) as u16;
                    // Crouching: a standing guard with nothing coming is a
                    // walk back. It rises to meet an attack that needs it.
                    return Some(Plan::guard(true, frames));
                }
                (Act::Route(_), true) => match self.route_outcome {
                    // Its last attack can still be out as its pads end: what
                    // it meets is seen before the plan goes on.
                    Some(Outcome::Whiff) if view.foe.reacting() => {
                        self.route_outcome = None;
                        Outcome::Hit
                    }
                    Some(Outcome::Whiff) if view.me.attacking() && view.foe.guarding() => {
                        self.route_outcome = None;
                        Outcome::Guard
                    }
                    Some(Outcome::Whiff) if view.me.attacking() => {
                        self.game = Some(run);
                        return Some(wait);
                    }
                    Some(outcome) => {
                        self.route_outcome = None;
                        outcome
                    }
                    None if self.route.is_some() => return Some(wait),
                    None => Outcome::Nothing,
                },
                (Act::Do(Choice::Normal { crouch, button }), true) => {
                    match self.attack_outcome(run, view, self.reach[usize::from(crouch)][button], ctx) {
                        Some(outcome) => outcome,
                        None => return Some(wait),
                    }
                }
                (Act::Do(Choice::Special(_)), true) if stronger && ctx.baseline_rules() => {
                    match self.attack_outcome(run, view, self.longest_reach(), ctx) {
                        Some(outcome) => outcome,
                        None => return Some(wait),
                    }
                }
                (_, true) => Outcome::Nothing,
            };
            run.hit |= outcome == Outcome::Hit;
            match at.next(outcome) {
                Some(next) => {
                    run.step = next;
                    run.acted = false;
                }
                None => break,
            }
        }
        self.game = Some(run);
        self.end_game_plan();
        None
    }

    /// Its fighter can use `tool` from where it feels itself in `felt` (SC16,
    /// `#208`): it has the tool, and for a charge, held walking back, room
    /// behind it. (A command throw walks in to its range first.)
    fn tool_ready(&self, tool: cast::Tool, felt: &Percept, ctx: &Context) -> bool {
        ctx.tools.get(tool).is_some() && (tool != cast::Tool::Charge || game::room_behind(felt))
    }

    /// The pads of one of its fighter's tools (SC15, `#207`), through the
    /// level's execution chance as a special's are ([`Self::motion`]), from
    /// `felt`, the frame with its own fighter as it feels it now: its facing
    /// then, as after a cross-up the delayed view's would mirror the motion
    /// (Retsu's Izuna as his Senpuu Kyaku) and hold a charge the wrong way.
    /// A charge move's input holds back (a walk back while it builds) and
    /// then releases. A command throw's motion that slips (its button
    /// dropped) leaves out its last Up, which would otherwise be a jump
    /// (SC16, `#208`; Birdie's chain ends on Up).
    fn tool_plan(&mut self, tool: cast::Tool, entry: &MoveEntry, felt: Percept, ctx: &Context) -> Plan {
        self.last_normal = None;
        let facing_right = !felt.me.facing_left;
        let motion = self.motion(entry, facing_right, felt, ctx);
        let slipped = motion.iter().all(|&pad| pad & BUTTONS == 0);
        let pads: Vec<u16> = motion.into_iter().map(|pad| if slipped && tool == cast::Tool::CommandThrow { pad & !UP } else { pad }).collect();
        Plan::of(PlanKind::Act, &pads)
    }

    /// A frame of a timed step's walk in toward a knocked-down opponent, or
    /// of waiting once within `reach` of him; the plan is kept with it.
    ///
    /// With `shimmy`, the frames left before the attack is due: once well
    /// inside the attack's reach ([`game::SHIMMY`]) or stopped against his
    /// body, it walks back out a few frames and in again, rather than
    /// standing, while there is time to come back.
    fn close_in(&mut self, run: game::Running, step: u32, view: Percept, reach: i16, shimmy: Option<u32>, ctx: &Context) -> Plan {
        // Its own place is felt as it is now; his is seen late.
        let ready = self.ready(step, view, true);
        let felt = self.felt(step, view);
        let gap = felt.gap();
        let toward = if self.stronger(ctx) && ctx.baseline_rules() { felt.toward() } else { view.toward() };
        let forward = if toward > 0 { RIGHT } else { LEFT };
        let back = RIGHT + LEFT - forward;
        self.game = Some(run);
        let Some(room) = shimmy.filter(|_| ready) else {
            return if ready && gap > reach { Plan::of(PlanKind::Walk, &[forward]) } else { Plan::of(PlanKind::Idle, &[0]) };
        };
        // A walk in that has not moved it over the last few frames has
        // stopped against his body: as near as it gets.
        let walking_in = self.pad & (RIGHT | LEFT) == forward;
        let blocked = walking_in
            && gap <= reach + SHIMMY_BODY
            && self.seen(step.saturating_sub(1 + SHIMMY_STUCK)).is_some_and(|was| was.me.x == felt.me.x);
        // Well inside the attack's reach, or stopped against him: a few
        // frames' walk back out, if it leaves time to come in again before
        // the attack is due; otherwise in, a frame at a time.
        let inside = gap <= reach - game::SHIMMY || blocked;
        if inside && room > SHIMMY_OUT as u32 * 2 {
            Plan::of(PlanKind::Walk, &[back; SHIMMY_OUT])
        } else if !inside {
            Plan::of(PlanKind::Walk, &[forward])
        } else {
            Plan::of(PlanKind::Idle, &[0])
        }
    }

    /// The plan's next pad walks toward the opponent: the stick forward
    /// alone, in a walk or before an attack's first button.
    fn walking_in_next(&self, view: Percept) -> bool {
        let forward = if view.toward() > 0 { RIGHT } else { LEFT };
        let walk = match self.plan.kind {
            PlanKind::Walk => true,
            PlanKind::Act => self.plan.press_to_come() && !self.plan.answer,
            _ => false,
        };
        walk && !self.plan.done() && self.plan.pads[usize::from(self.plan.at)] == forward
    }

    /// `view` shows the opponent in a ground attack, with no projectile
    /// coming, while the CPU is out of his reach (as far as it has seen his
    /// attacks go, from where it feels its own place): one that will whiff.
    fn out_of_reach(&self, step: u32, view: Percept) -> bool {
        view.foe.attacking()
            && !view.foe.jumping()
            && !view.shot_coming()
            && self.felt(step, view).foe_gap() > self.judge.foe_edge()
    }

    /// Baseline play can approach a seen ground attack while it remains
    /// outside the opponent's learned reach, with room for a short step.
    /// The foe is still seen late; only its own position is felt live.
    /// This permits the next movement pad; it does not open an attack
    /// choice against the committed move.
    fn may_approach_attack(&self, step: u32, view: Percept, ctx: &Context) -> bool {
        self.stronger(ctx) && ctx.baseline_rules() && self.out_of_reach(step, view)
            && self.felt(step, view).foe_gap() > self.judge.foe_edge() + game::EDGE_OUT
    }

    /// How far the opponent has moved away from the CPU's fighter in the
    /// air, over at most the last [`DRIFT_FRAMES`] frames up to `view`
    /// (toward it, negative): his steps before leaving the ground are not
    /// his jump's way.
    fn foe_drift(&self, view: Percept) -> i16 {
        let airborne = |p: &Percept| matches!(p.foe.move_id, 0xa | 0xb);
        let mut from = view;
        for back in 1..=DRIFT_FRAMES {
            match self.seen(view.frame.saturating_sub(back)).filter(airborne) {
                Some(was) => from = was,
                None => break,
            }
        }
        (view.foe.x - from.foe.x) * view.toward()
    }

    /// The opponent has stood in one place over the last few frames `view`
    /// and those before it show.
    fn foe_settled(&self, view: Percept) -> bool {
        (1..=game::SETTLED_FRAMES).all(|back| self.seen(view.frame.saturating_sub(back)).is_some_and(|was| was.foe.x == view.foe.x))
    }

    /// `view` with its own fighter as the last frame shows him: a player
    /// feels where his own fighter stands, and sees the opponent through
    /// the reaction delay.
    fn felt(&self, step: u32, view: Percept) -> Percept {
        match self.seen(step.saturating_sub(1)).filter(|now| now.fighting) {
            Some(now) => Percept { me: now.me, ..view },
            None => view,
        }
    }

    /// The CPU's own fighter can start something, and it has seen its last
    /// action begin. The old play goes by its fighter as `view` shows him;
    /// the stronger play feels its own fighter as he is now (a player feels
    /// when his fighter can move again) and sees the opponent through the
    /// delay.
    fn ready(&self, step: u32, view: Percept, stronger: bool) -> bool {
        if stronger {
            let me = self.felt(step, view).me;
            (me.free() || me.guarding()) && self.settled(step, view, true)
        } else {
            (view.me.free() || view.me.guarding()) && self.settled(step, view, false)
        }
    }

    /// An SF4 or SF5 fighter feels a walk cycle fail to advance him.
    /// End the reach step as its normal timeout would, instead of
    /// pushing against a body or wall for ninety frames. Only its own
    /// observed position is used; a locked guard is left to recover.
    fn walk_stalled(&self, step: u32, since: u32, ctx: &Context) -> bool {
        const FRAMES: u32 = 16;
        if !self.stronger(ctx) || !matches!(ctx.family, Some(crate::era::EraFamily::Sf4 | crate::era::EraFamily::Sf5))
            || ctx.own_blockstun || self.plan.kind != PlanKind::Walk
            || step <= FRAMES || step.saturating_sub(since) < FRAMES {
            return false;
        }
        let (Some(now), Some(from)) = (self.seen(step - 1), self.seen(step - FRAMES)) else { return false };
        let toward = if now.me.facing_left { -1 } else { 1 };
        // Native walking can move in small steps or rock back and forth
        // over its animation. Keep genuine forward progress, including
        // Retsu's slow steps; a complete cycle with none ends the reach.
        (now.me.x - from.me.x) * toward <= 0
            && (1..=FRAMES).all(|age| self.seen(step - age).is_some_and(|was| {
                was.me.present && was.me.free() && !was.me.guarding() && !was.me.jumping()
                    && was.me.facing_left == now.me.facing_left
            }))
    }

    /// It has seen its last action begin: in `view`, or for the stronger
    /// play, which feels its own fighter, on the last frame.
    fn settled(&self, step: u32, view: Percept, stronger: bool) -> bool {
        if stronger {
            self.seen(step.saturating_sub(1)).map_or(view.frame, |now| now.frame) >= self.settle
        } else {
            view.frame >= self.settle
        }
    }

    /// Learn a normal's reach from its attack box, seen live in front of the
    /// CPU's fighter after it pressed it (and its projectile's range,
    /// [`Self::learn_shot`]).
    fn learn(&mut self, view: Percept, before: Option<Percept>, from_stance: bool) {
        self.learn_shot(view);
        let Some(Normal { crouch, button: b, pressed, from, best }) = self.last_normal else { return };
        if view.frame < pressed {
            return;
        }
        if !view.me.attacking() {
            // Over (or never seen to start): fold what this attack reached
            // into what the CPU knows, and the next attack is another's.
            if from.is_some() && best > 0 {
                let (c, known) = (usize::from(crouch), &mut self.reach[usize::from(crouch)][b]);
                // The first sighting replaces the starting guess, longer or
                // shorter; later ones move the estimate halfway, so one odd
                // reading neither sticks nor is ignored.
                *known = if self.reach_seen[c][b] { (*known + best) / 2 } else { best };
                self.reach_seen[c][b] = true;
            }
            if from.is_some() || view.frame > pressed + 8 {
                self.last_normal = None;
            }
            return;
        }
        // Baseline attack poses can shift the drawn centre as they start.
        // Measure their reach from the adjacent pre-attack pose, the same
        // coordinate used to choose an attack. Older play keeps its first
        // attack-pose origin. A jumped view supplies no baseline origin.
        let from = match (from, before) {
            (Some(x), _) => x,
            (None, Some(b)) if !b.me.attacking() && (!from_stance || b.frame.checked_add(1) == Some(view.frame)) => {
                if from_stance { b.me.x } else { view.me.x }
            }
            _ => {
                // Started unseen (the view jumped): measure nothing.
                self.last_normal = None;
                return;
            }
        };
        // Its own visible facing defines its attack-box direction even
        // when the delayed opponent crosses during that normal.
        let forward = if from_stance { if view.me.facing_left { -1 } else { 1 } } else { view.toward() };
        let best = if view.me.active { best.max(((view.me.front - from) * forward).clamp(16, 120)) } else { best };
        self.last_normal = Some(Normal { crouch, button: b, pressed, from: Some(from), best });
    }

    /// Learn how far the CPU's projectile goes: the farthest each of its
    /// shots is seen from its fighter, folded in as a normal's reach is
    /// when the shot is gone. A shot that leaves the screen reads as far.
    fn learn_shot(&mut self, view: Percept) {
        if view.my_shot {
            self.shot_far = self.shot_far.max(view.my_shot_dist);
        } else if self.shot_far > 0 {
            let far = self.shot_far;
            self.shot_range = Some(self.shot_range.map_or(far, |known| (known + far) / 2));
            self.shot_far = 0;
        }
    }

    /// The CPU's projectile carries across the screen, as far as it knows:
    /// not yet seen, or seen to go far.
    fn shot_goes_far(&self) -> bool {
        self.shot_range.is_none_or(|r| r >= LONG_SHOT)
    }

    /// The next pad of the plan.
    fn next_pad(&mut self, step: u32, view: Percept, ctx: &Context) -> u16 {
        let stronger = self.stronger(ctx);
        // Judge queued baseline walks from that same felt own position.
        let walk_view = if stronger && ctx.baseline_rules() { self.felt(step, view) } else { view };
        if stronger && self.judge.coming(view) && self.walking_in_next(walk_view)
            && !self.may_approach_attack(step, view, ctx)
        {
            // It does not walk into an attack or a projectile it has seen:
            // the walk (or the walk before an attack) is dropped, and what
            // it does instead is chosen on the next frame.
            self.plan = Plan::default();
            self.settle = 0;
            return 0;
        }
        let back = if stronger && ctx.baseline_rules() {
            if self.felt(step, view).me.facing_left { RIGHT } else { LEFT }
        } else if view.toward() > 0 {
            LEFT
        } else {
            RIGHT
        };
        if let PlanKind::Await { choice, wait } = self.plan.kind {
            // The stronger play lets a jump it sees carry him away go by
            // (SC8, `#200`): there is nothing there to meet.
            // Judged once he is off the ground and over a few frames (SC9,
            // `#201`): in the pre-jump crouch his walk's last steps can
            // still show, and his drawing's centre shifts as his pose does.
            let away = self.stronger(ctx) && matches!(view.foe.move_id, 0xa | 0xb) && self.foe_drift(view) > DRIFT;
            if !view.foe.jumping() || away {
                self.plan = Plan::default();
                return 0;
            }
            if wait > 0 {
                self.plan.kind = PlanKind::Await { choice, wait: wait - 1 };
                return 0;
            }
            // The stronger play enters it facing the way it feels its own
            // fighter face now: a jump that carries him over it has turned
            // it since the frame it decides from (SC9, `#201`).
            let facing = if self.stronger(ctx) { self.felt(step, view) } else { view };
            self.plan = self.plan_for(choice, step, facing, ctx, true);
        }
        if let PlanKind::Guard { low, min, clear, held } = self.plan.kind {
            // The stronger play keeps guarding while an attack is still
            // coming, not through its recovery: then it is free to act.
            let recovering = self.stronger(ctx)
                && view.foe.attacking()
                && !view.foe.jumping()
                && !view.shot_coming()
                && !self.judge.coming(view);
            let threat = if recovering { None } else { view.threat() };
            // Keep the initial throw-tech window, but do not spend two
            // seconds guarding only because a quiet opponent stays close.
            // The opponent is still read through the reaction delay; a
            // moving foe, live attack, locked guard or held bait keeps it.
            let short_guard = self.stronger(ctx) && (ctx.baseline_rules()
                || matches!(ctx.family, Some(crate::era::EraFamily::Sf4 | crate::era::EraFamily::Sf5)));
            let quiet = short_guard
                && !ctx.own_blockstun && step >= self.bait_until && held >= 8
                && view.foe.free() && !view.foe.jumping() && self.foe_settled(view);
            let throw_threat = ctx.throws && !quiet
                && !view.foe.reacting()
                && (view.dist() <= 48
                    || (view.foe.move_id == 2
                        && view.dist() <= 48 + (ctx.level.reaction() as i16) + 8));
            let clear = if threat.is_some() || throw_threat {
                0
            } else {
                clear.saturating_add(1)
            };
            let held = held + 1;
            // Once this guard has seen three clear delayed frames, its
            // initial eight-frame hold is enough. Actual blockstun and a
            // chosen reset bait still keep their recovery/hold time.
            let clear_early = short_guard
                && !ctx.own_blockstun && step >= self.bait_until && held >= 8;
            if (clear >= 5 && held >= min) || (clear_early && clear >= 3) || held > 120 {
                self.plan = Plan::default();
                return 0;
            }
            // Delayed throw tech (research 4.3, SP22): while guarding against
            // an opponent who has walked into throw range, pulse `Back + HP`
            // on a short delay so a grab is teched inside its 7-frame window
            // (and an opponent standing in throw range is thrown).
            // Not in a combo reset's bait (SC10, `#202`): its guard waits
            // for his reversal, and the pulse would throw him or come out
            // as a button into it.
            let baiting = self.stronger(ctx) && step < self.bait_until;
            if ctx.throws
                && !baiting
                && threat.is_none()
                && !view.foe.jumping()
                && !view.foe.reacting()
                && view.dist() <= 46
                && held >= 3
                && held % 3 == 0
                && self.rng.chance(ctx.level.guard())
            {
                self.plan.kind = PlanKind::Guard { low, min, clear, held };
                return back | button(2);
            }
            // Low against a crouching attack, high against a jump or a
            // projectile, and against a standing attack the guard chosen
            // (the crouching one stops it only under standard blocking).
            let low_now = match threat {
                Some(Some(low)) => low,
                Some(None) => low && ctx.standard_block,
                None => low,
            };
            self.plan.kind = PlanKind::Guard { low, min, clear, held };
            let guard = if low_now { back | DOWN } else { back };
            // The stronger play holds SF6's Drive Parry through a guard
            // while an attack or a projectile it sees is still coming (an
            // attack's box not yet come and gone) and the gauge allows; the
            // stick stays back, so a parry that cannot start is still a
            // guard. Not against an opponent up close with nothing coming
            // (a parry loses to his throw), nor through his recovery, which
            // it may punish from its guard.
            let parry = self.stronger(ctx)
                && threat.is_some()
                && self.judge.coming(view)
                && ctx.era.is_some_and(|tactics| tactics.can_hold_parry());
            return if parry { guard | button(1) | button(4) } else { guard };
        }
        if self.plan.done() {
            return 0;
        }
        let pad = self.plan.pads[usize::from(self.plan.at)];
        self.plan.at += 1;
        pad
    }

    /// Answer something that started in the last frame seen, if it is worth
    /// answering and the level answers it.
    fn react(&mut self, step: u32, view: Percept, before: Percept, ctx: &Context) -> Option<Plan> {
        let (foe, was) = (view.foe, before.foe);
        let level = ctx.level;
        let dist = view.dist();
        // A jump its habit memory expects is met more surely, and one it
        // was too busy to answer as it began (its plan could not be cut
        // short) is answered as soon as it can be, while there is still
        // time to meet it: entered while its fighter recovers, as a player
        // enters an anti-air to come out on the first free frame (SC9,
        // `#201`).
        let stronger = self.stronger(ctx);
        let expected = if stronger { self.habits.anti_air_bonus(view.frame, view.gap(), level) } else { 0 };
        let began = matches!(foe.move_id, 9..=0xb) && !was.jumping();
        let late = view.frame.saturating_sub(self.jump_began);
        let coming = self.foe_drift(view) < -DRIFT;
        let missed = expected > 0
            && !began
            && !self.jump_met
            && foe.jumping()
            && coming
            && late <= MISSED_JUMP;
        // A jump toward the CPU, from its start (the pre-jump crouch shows).
        if (began || missed) && dist < 200 {
            self.jump_met = true;
            if !self.rng.chance(level.guard()) {
                return None;
            }
            let mut options = vec![(Choice::Guard, 55), (Choice::Retreat, 12)];
            if ctx.kit.anti_air.is_some() {
                options.push((Choice::Special(Role::AntiAir), 62 * level.specials() / 100 + expected));
            }
            if dist < 130 {
                // The crouching heavy punch rises sooner than the standing one.
                options.push((Choice::Normal { crouch: true, button: 2 }, 38 + expected / 2));
            }
            let choice = self.pick(&mut options, level);
            // An anti-air is timed to meet the jump low, about 32 frames in
            // (a forward jump lasts about 47): the CPU has seen it start
            // `reaction` frames late, and the input and the move's start
            // take some more.
            let lead = match choice {
                Choice::Special(_) => level.special_cost() as u32 + 10,
                Choice::Normal { .. } => 7,
                _ => return Some(self.plan_for(choice, step, view, ctx, true)),
            };
            // A jump answered late is met that much sooner after it is
            // chosen.
            let late = if missed { late } else { 0 };
            if missed && lead + level.reaction() + late > 32 {
                // Too late for that answer to meet him before he lands
                // (a Shoryuken begun then comes out after he is down): it
                // guards instead.
                return Some(self.plan_for(Choice::Guard, step, view, ctx, true));
            }
            let wait = 32u32.saturating_sub(lead + level.reaction() + late) as u16;
            return Some(Plan { kind: PlanKind::Await { choice, wait }, ..Plan::default() });
        }
        // A projectile thrown.
        if view.shot && !before.shot && view.shot_coming() {
            if !self.rng.chance(level.guard()) {
                return None;
            }
            let shot_dist = (view.shot_x - view.me.x).abs();
            let mut options = vec![(Choice::Guard, 45)];
            if (48..170).contains(&shot_dist) {
                options.push((Choice::JumpIn, 40));
            }
            if ctx.kit.projectile.is_some() && !view.my_shot && shot_dist > 80 && self.shot_goes_far() {
                options.push((Choice::Special(Role::Projectile), 36 * level.specials() / 100));
            }
            let choice = self.pick(&mut options, level);
            return Some(self.plan_for(choice, step, view, ctx, true));
        }
        // An attack started from the ground (or a new one after another):
        // not a jump's attack, which its jump was answered for.
        let from_jump = was.jumping() && foe.jumping();
        let started = foe.attacking() && !from_jump && (!was.attacking() || foe.move_id != was.move_id);
        if started && view.gap() < 150 {
            if !self.rng.chance(level.guard()) {
                return None;
            }
            // Against a standing attack either guard does under standard
            // blocking; the CPU picks one, so both are seen.
            let low = foe.low_attack() || (ctx.standard_block && self.rng.chance(50));
            return Some(Plan::guard(low, 0));
        }
        // Baseline stronger play feels when its own fighter can move;
        // the opponent and its attack history still come through the delay.
        // A current guard pose alone does not release native blockstun.
        let (punish_me, punish_unlocked) = if stronger && ctx.baseline_rules() {
            (self.felt(step, view).me, !ctx.own_blockstun)
        } else {
            (view.me, true)
        };
        // A move that missed: its attack box has gone and it is recovering,
        // within reach of a heavy normal.
        // Not a move the CPU guarded: one hit a move, so a guarded move's box
        // goes too, and under true combos (SP17) the guard is then locked in
        // blockstun, which the CPU cannot see the end of.
        let whiffed = foe.attacking() && !foe.active && was.active && !foe.jumping() && !view.me.guarding() && !before.me.guarding();
        // The stronger play knows a whiff by its attack box never reaching
        // the CPU, so a move it guarded in case can be a whiff too.
        let whiff_seen = stronger
            && self.judged.situation == Situation::OpponentWhiffed
            && punish_unlocked && (punish_me.free() || punish_me.guarding())
            && self.rng.chance(level.quality());
        // A whiffed attack still in the air is punished as it lands: as the
        // CPU sees it fall, it starts the route so its opener meets him on
        // the ground. Rising, or with the ground unknown, it waits.
        let landing_soon = !foe.jumping() || {
            let fall = foe.y - was.y;
            let ground = if view.me.jumping() { None } else { Some(view.me.y) };
            ground.is_some_and(|ground| fall > 0 && (ground - foe.y) <= fall * game::LANDING_LEAD)
        };
        if whiff_seen && landing_soon && self.settled(step, view, stronger) {
            use crate::route_book::Reach;
            let felt = self.felt(step, view);
            if self.take_route(ctx, felt, &[Reach::Punish, Reach::Ground], game::PUNISH_WALK) {
                return Some(Plan::default());
            }
        }
        // In SF2 (SC15, `#207`), SFZ (SC16, `#208`), SF3 (SC17, `#209`)
        // SF4 (SC18, `#210`), SF5 (SC19, `#211`) and the new SF6 cast
        // (SC20, `#212`; Ryu keeps its established decisions), and the
        // baseline styles (SC21, `#213`):
        // his ground attack it has guarded, now recovering in reach, is
        // its turn. Out of its own
        // blockstun it takes a punish route, or a crouching light punch,
        // rather than guarding on while he presses again.
        if stronger
            && (ctx.baseline_rules() || matches!(ctx.family, Some(crate::era::EraFamily::Sf2 | crate::era::EraFamily::SfZ | crate::era::EraFamily::Sf3 | crate::era::EraFamily::Sf4 | crate::era::EraFamily::Sf5)) || ctx.sf6_cast())
            && self.judge.recovering_after_meeting(view)
            && !ctx.own_blockstun
            && punish_unlocked && (punish_me.free() || punish_me.guarding())
            && view.gap() <= self.longest_reach() + 2 * i16::from(game::PUNISH_WALK)
            && self.settled(step, view, stronger)
            && self.rng.chance(level.quality())
        {
            use crate::route_book::Reach;
            let felt = self.felt(step, view);
            if self.take_route(ctx, felt, &[Reach::Punish, Reach::Ground], game::PUNISH_WALK) {
                return Some(Plan::default());
            }
            if view.gap() <= self.reach[1][0] + 6 {
                return Some(self.plan_for(Choice::Normal { crouch: true, button: 0 }, step, view, ctx, true));
            }
        }
        if whiffed && punish_unlocked && punish_me.free() && self.settled(step, view, stronger) {
            let reach = self.reach[0][2].max(self.reach[0][5]) + 6;
            if view.gap() <= reach && self.rng.chance(level.quality()) {
                let cancel_role = if ctx.kit.cancels_with(Role::AntiAir) {
                    Some(Role::AntiAir)
                } else if ctx.kit.cancels_with(Role::Advance) {
                    Some(Role::Advance)
                } else if ctx.kit.cancels_with(Role::Projectile) && !view.my_shot {
                    Some(Role::Projectile)
                } else {
                    None
                };
                let choice = if ctx.cancels
                    && view.gap() <= self.reach[1][3]
                    && cancel_role.is_some()
                    && self.rng.chance(level.specials() as u32)
                {
                    Choice::Cancel { crouch: true, button: 3, role: cancel_role.unwrap() }
                } else if ctx.true_combos && view.gap() <= 32 && self.rng.chance(60) {
                    Choice::Combo
                } else if ctx.kit.anti_air.is_some() && view.gap() < 36 && self.rng.chance(level.specials() as u32) {
                    Choice::Special(Role::AntiAir)
                } else {
                    let button = if self.reach[0][5] >= view.gap() && self.rng.chance(50) { 5 } else { 2 };
                    Choice::Normal { crouch: false, button }
                };
                return Some(self.plan_for(choice, step, view, ctx, true));
            }
        }
        // Under throws (SP19, SP22): an opponent walking into throw range is
        // met with a crouching guard that delays a throw tech.
        if ctx.throws
            && !foe.attacking()
            && !foe.jumping()
            && !foe.reacting()
            && foe.move_id == 2
        {
            let engage = level.throw_engage();
            if dist <= engage
                && (was.move_id != 2 || before.dist() > engage)
                && self.rng.chance(level.guard())
            {
                return Some(Plan::guard(true, 16));
            }
        }
        None
    }

    /// Pick from `options` (choice, score): the best with the level's
    /// quality, else one of the rest. Scores get a little noise first, so
    /// equal options take turns.
    fn pick(&mut self, options: &mut [(Choice, i32)], level: Level) -> Choice {
        for o in options.iter_mut() {
            o.1 += self.rng.below(12) as i32;
        }
        options.sort_by_key(|o| std::cmp::Reverse(o.1));
        let live = options.iter().filter(|o| o.1 > 0).count().max(1);
        if live == 1 || self.rng.chance(level.quality()) {
            return options[0].0;
        }
        let rest = (live - 1).min(4) as u32;
        options[1 + self.rng.below(rest) as usize].0
    }

    /// Choose what to do next in neutral.
    fn choose(&mut self, step: u32, view: Percept, ctx: &Context) -> Plan {
        let level = ctx.level;
        let gap = view.gap();
        let longest = self.reach[0].iter().chain(self.reach[1].iter()).copied().max().unwrap_or(60);
        // The opponent hit (nothing to guard against), or in an attack
        // (startup, active or recovery: a whiff is punished in `react`).
        let foe_busy = view.foe.reacting();
        let foe_attacking = view.foe.attacking();
        let approach_clear = self.may_approach_attack(step, view, ctx);
        let threatened = foe_attacking && !approach_clear;
        // The CPU attacks only into an opening it can use. An attack still
        // coming is guarded, not answered with one of its own (under true
        // combos, SP17, a guard locked in blockstun holds through the string
        // this way). A hit reaction is no opening: it is seen `reaction`
        // frames late, so whatever hitstun it shows (true combos make it
        // hittable) has mostly gone by the time an attack would land, and
        // under the classic rules the reaction cannot be hit at all. Combos
        // as tactics are SP22's.
        let open = !foe_attacking && !foe_busy;
        let mut options: Vec<(Choice, i32)> = vec![(Choice::Wait, level.wait())];
        let stronger = self.stronger(ctx);
        if stronger && self.judge.coming(view) && !approach_clear {
            // The stronger play does not walk into what it sees coming.
        } else if threatened {
            options.push((Choice::Approach, 4));
        } else if gap > longest - 8 {
            options.push((Choice::Approach, 40 + i32::from((gap - longest).clamp(0, 80)) / 2));
        } else {
            options.push((Choice::Approach, 6));
        }
        // The stronger play gives a cornered opponent no ground.
        let cornering = stronger && self.judged.foe_cornered;
        if !cornering {
            options.push((Choice::Retreat, if gap < 24 { 26 } else { 8 }));
        }
        if gap < 100 && !foe_busy {
            options.push((Choice::Guard, level.anticipate() + if threatened { 30 } else { 0 }));
        }
        // A normal that reaches, unless the opponent's attack is already
        // coming. The 1987 heavies start slowly (up to about 20 frames), so
        // the light and medium buttons, which start sooner, score higher;
        // the sweep knocks down.
        for crouch in [false, true] {
            for button in 0..6 {
                if gap <= self.reach[usize::from(crouch)][button] && open {
                    let weight = [8, 4, 0, 6, 2, 0][button] + if crouch && button == 5 { 6 } else { 0 };
                    options.push((Choice::Normal { crouch, button }, 44 + weight));
                }
            }
        }
        let specials = level.specials();
        if ctx.true_combos && open && gap <= 46 {
            options.push((Choice::Combo, if gap <= 28 { 58 } else { 52 }));
        }
        if ctx.cancels && open {
            let role = if ctx.kit.cancels_with(Role::Projectile) && !view.my_shot {
                Some(Role::Projectile)
            } else if ctx.kit.cancels_with(Role::Advance) {
                Some(Role::Advance)
            } else if ctx.kit.cancels_with(Role::AntiAir) {
                Some(Role::AntiAir)
            } else {
                None
            };
            if let Some(role) = role {
                for (crouch, button, bonus) in [(true, 3, 14), (false, 0, 12), (false, 1, 10)] {
                    if gap <= self.reach[usize::from(crouch)][button] + 6 {
                        options.push((
                            Choice::Cancel { crouch, button, role },
                            (44 + bonus) * specials / 100,
                        ));
                    }
                }
            }
        }
        if ctx.throws && open && view.dist() <= 72 {
            let close = if view.dist() <= 48 { 16 } else { 0 };
            let vs_guard = if view.foe.guarding() { 14 } else { 0 };
            // More against an opponent its memory says does not tech.
            let habit = if stronger { self.habits.throw_bonus(level) } else { 0 };
            options.push((Choice::Throw, 44 + close + vs_guard + habit));
        }
        if ctx.kit.projectile.is_some() && !view.my_shot && open {
            if ctx.kit.projectile_alt.is_some() {
                if gap > 56 {
                    options.push((Choice::Special(Role::Projectile), (48 + if gap > 120 { 14 } else { 0 }) * specials / 100));
                }
            } else {
                match self.shot_range {
                    // A short one, like a heavy normal where it reaches.
                    Some(range) if range < LONG_SHOT => {
                        if gap <= range - 8 {
                            options.push((Choice::Special(Role::Projectile), 48 * specials / 100));
                        }
                    }
                    // Across the screen (or not yet seen): from far off.
                    _ if gap > 90 => {
                        // Not from where a jump over it lands short, against
                        // one who jumps it (SC9, `#201`).
                        let habit = if stronger { self.habits.shot_bonus(gap, game::WATCHED_SHOT_GAPS, level) } else { 0 };
                        options.push((Choice::Special(Role::Projectile), (40 + if gap > 150 { 18 } else { 0 }) * specials / 100 + habit));
                    }
                    _ => {}
                }
            }
        }
        if ctx.kit.advance.is_some() && (40..110).contains(&gap) && open {
            options.push((Choice::Special(Role::Advance), (if ctx.kit.advance_alt.is_some() { 44 } else { 24 }) * specials / 100));
        }
        if ctx.kit.projectile.is_none() && ctx.kit.anti_air.is_some() && gap < 40 && open {
            options.push((Choice::Special(Role::AntiAir), 18 * specials / 100));
        }
        if (70..150).contains(&gap) && open {
            options.push((Choice::JumpIn, 30));
        }
        if gap < 60 && open && !cornering {
            options.push((Choice::JumpBack, 6));
        }
        let choice = self.pick(&mut options, level);
        self.plan_for(choice, step, view, ctx, false)
    }

    /// The pad frames for `choice`, facing as last seen. `answer` is a
    /// response to something seen, which costs a motion special its extra
    /// frames.
    fn plan_for(&mut self, choice: Choice, step: u32, view: Percept, ctx: &Context, answer: bool) -> Plan {
        let level = ctx.level;
        let (fwd, back) = if view.toward() > 0 { (RIGHT, LEFT) } else { (LEFT, RIGHT) };
        // The CPU waits to see its action begin before choosing again.
        let settle_after = |frames: u32| step + frames;
        match choice {
            Choice::Wait => {
                let n = 6 + self.rng.below(3) as usize;
                Plan::of(PlanKind::Idle, &[0; PLAN_CAP][..n])
            }
            Choice::Approach => {
                let n = 8 + self.rng.below(12) as usize;
                Plan::of(PlanKind::Walk, &[fwd; PLAN_CAP][..n])
            }
            Choice::Retreat => {
                let n = 8 + self.rng.below(10) as usize;
                Plan::of(PlanKind::Walk, &[back; PLAN_CAP][..n])
            }
            Choice::Guard => Plan::guard(true, 10 + self.rng.below(14) as u16),
            Choice::Normal { crouch, button: b } => {
                let stick = if crouch { DOWN } else { 0 };
                let press = stick | button(b);
                self.last_normal = Some(Normal { crouch, button: b, pressed: step + 1, from: None, best: 0 });
                self.settle = settle_after(3);
                // Native baseline normals begin on release. A one-frame
                // tap produces the same move two frames before a held tap.
                let pads: &[u16] = if self.stronger(ctx) && ctx.baseline_rules() {
                    &[stick, press, stick, 0]
                } else {
                    &[stick, press, press, press, stick, 0]
                };
                Plan::of(PlanKind::Act, pads)
            }
            Choice::Combo => {
                self.last_normal = None;
                let mut pads = Vec::with_capacity(PLAN_CAP);
                if view.dist() > 40 {
                    let walk = (view.dist() - 38).clamp(2, 22) as usize;
                    pads.extend(std::iter::repeat_n(fwd, walk));
                }
                pads.push(0);
                if self.rng.chance(level.execution()) {
                    for i in 0..24 {
                        pads.push(if i % 2 == 0 { button(0) } else { 0 });
                    }
                } else {
                    pads.extend([button(0), button(0), button(0), 0]);
                }
                self.settle = settle_after(pads.len().min(PLAN_CAP) as u32);
                Plan::of(PlanKind::Act, &pads)
            }
            Choice::Cancel { crouch, button: norm_btn, role } => {
                let entries = ctx.kit.by_role(role).map(|entry| entry.filter(|entry| can_cancel_into(*entry)));
                let Some(entry) = self.pick_special(entries) else { return Plan::default() };
                self.last_normal = None;
                let mut pads = Vec::with_capacity(PLAN_CAP);
                let reach = self.reach[usize::from(crouch)][norm_btn];
                if view.gap() > reach - 8 {
                    let walk = (view.gap() - (reach - 10)).clamp(2, 16) as usize;
                    pads.extend(std::iter::repeat_n(fwd, walk));
                }
                let norm_stick = if crouch { DOWN } else { 0 };
                pads.push(norm_stick);
                pads.push(norm_stick | button(norm_btn));
                let facing_right = !view.me.facing_left;
                let pressure = view.me.health < view.foe.health || view.dist() < 60;
                let clean = level.clean(pressure);
                let ok = self.rng.chance(clean);
                let motion = entry.input.motion;
                let strength = self.rng.below(3) as usize;
                if matches!(entry.recognizer, Recognizer::AlphaMove | Recognizer::FamilyMove) {
                    // Give the first tap its own edge even if the preceding
                    // normal and the mash use the same punch button.
                    pads.push(norm_stick);
                    pads.extend(alpha_motion(&entry, facing_right, strength, ok));
                    self.settle = settle_after(pads.len().min(PLAN_CAP) as u32);
                    return Plan::of(PlanKind::Act, &pads);
                }
                let sp_btn = strength + if entry.input.button.kick() { 3 } else { 0 };
                for &dir in &motion[..motion.len().saturating_sub(1)] {
                    let s = stick(dir, facing_right);
                    pads.push(s);
                    pads.push(s);
                }
                let last_pad = motion.last().map_or(0, |&d| stick(d, facing_right));
                for f in 0..16 {
                    let mut pad = last_pad;
                    if ok && f % 2 == 1 {
                        pad |= button(sp_btn);
                    }
                    pads.push(pad);
                }
                self.settle = settle_after(pads.len().min(PLAN_CAP) as u32);
                Plan::of(PlanKind::Act, &pads)
            }
            Choice::Throw => {
                self.last_normal = None;
                let mut pads = Vec::with_capacity(PLAN_CAP);
                if view.dist() > 40 {
                    let walk = (view.dist() - 38).clamp(2, 34) as usize;
                    pads.extend(std::iter::repeat_n(fwd, walk));
                }
                let dir = if self.rng.chance(70) { fwd } else { back };
                let b = if self.rng.chance(50) { 2 } else { 5 };
                if self.stronger(ctx) {
                    // The engine throws only from a standing fighter, not
                    // one walking: stop, then the direction and the button
                    // on the same frame.
                    pads.extend([0, 0, dir | button(b), dir | button(b), 0]);
                } else {
                    for i in 0..10 {
                        pads.push(if i % 3 == 1 { dir | button(b) } else { dir });
                    }
                    pads.push(0);
                }
                self.settle = settle_after(pads.len().min(PLAN_CAP) as u32);
                Plan::of(PlanKind::Act, &pads)
            }
            Choice::JumpIn | Choice::JumpBack => {
                self.last_normal = None;
                let up = UP | if choice == Choice::JumpIn { fwd } else { back };
                let mut frames = [0u16; 44];
                frames[..3].fill(up);
                if choice == Choice::JumpIn {
                    // A jump kick or punch on the way down (a forward jump
                    // lasts about 47 frames, its peak near the 21st).
                    let at = 19 + self.rng.below(7) as usize;
                    let b = [5, 4, 2][self.rng.below(3) as usize];
                    frames[at..at + 3].fill(button(b));
                }
                self.settle = settle_after(6);
                Plan::of(PlanKind::Act, &frames)
            }
            Choice::Special(role) => {
                let Some(entry) = self.pick_special(ctx.kit.by_role(role)) else { return Plan::default() };
                self.last_normal = None;
                let lead = if answer { level.special_cost() } else { 0 };
                // Own facing is felt now; the motion's pressure and the
                // opponent still come from the delayed view.
                let facing_left = if self.stronger(ctx) && ctx.baseline_rules() {
                    self.felt(step, view).me.facing_left
                } else {
                    view.me.facing_left
                };
                let frames = self.motion(&entry, !facing_left, view, ctx);
                let mut pads = vec![0u16; lead];
                pads.extend(frames);
                self.settle = settle_after(lead as u32 + 9);
                Plan::of(PlanKind::Act, &pads)
            }
        }
    }

    fn pick_special(&mut self, [first, second]: [Option<MoveEntry>; 2]) -> Option<MoveEntry> {
        match (first, second) {
            (Some(a), Some(b)) => Some(if self.rng.chance(50) { b } else { a }),
            (first, second) => first.or(second),
        }
    }

    /// A special's input as pad frames, the way the move list's Show feeds
    /// it (each step of the motion three frames, the last held, the button
    /// on the last step's second frame), entered cleanly with the level's
    /// execution chance. Under pressure (lower on health, or the opponent
    /// close) it slips more often. A slip drops the motion's middle step or
    /// presses the button a step early, and the pad gives whatever that
    /// makes.
    fn motion(&mut self, entry: &MoveEntry, facing_right: bool, view: Percept, ctx: &Context) -> Vec<u16> {
        let pressure = view.me.health < view.foe.health || view.dist() < 60;
        let clean = ctx.level.clean(pressure);
        if matches!(entry.recognizer, Recognizer::AlphaMove | Recognizer::FamilyMove) {
            let ok = self.rng.chance(clean);
            let strength = self.rng.below(3) as usize;
            return alpha_motion(entry, facing_right, strength, ok);
        }
        let mut motion: Vec<Dir> = entry.input.motion.to_vec();
        let mut early = false;
        if !self.rng.chance(clean) {
            if motion.len() > 2 && self.rng.chance(50) {
                motion.remove(1);
            } else {
                early = true;
            }
        }
        let strength = self.rng.below(3) as usize;
        let b = strength + if entry.input.button.kick() { 3 } else { 0 };
        let steps = motion.len();
        let last_from = 3 * steps.saturating_sub(1);
        let pressed_from = if early { last_from.saturating_sub(2) } else { last_from + 1 };
        (0..last_from + 14)
            .map(|f| {
                let step = (f / 3).min(steps.saturating_sub(1));
                let mut pad = motion.get(step).map_or(0, |d| stick(*d, facing_right));
                if (pressed_from..pressed_from + 4).contains(&f) {
                    pad |= button(b);
                }
                pad
            })
            .collect()
    }
}

/// A walk of `frames` frames toward the opponent as `view` shows him, or
/// away from him.
fn walk(toward: bool, view: Percept, frames: u32) -> Plan {
    let right = toward == (view.toward() > 0);
    Plan::of(PlanKind::Walk, &[if right { RIGHT } else { LEFT }; PLAN_CAP][..(frames as usize).min(PLAN_CAP)])
}

fn alpha_motion(entry: &MoveEntry, facing_right: bool, strength: usize, clean: bool) -> Vec<u16> {
    let strength = [Strength::Light, Strength::Medium, Strength::Heavy][strength];
    entry.input.frames(facing_right, strength).into_iter().map(|frame| {
        let dirs = frame.directions.into_iter().zip([RIGHT, LEFT, DOWN, UP])
            .fold(0, |pad, (held, bit)| if held { pad | bit } else { pad });
        if clean {
            frame.buttons.into_iter().enumerate()
                .fold(dirs, |pad, (b, held)| if held { pad | button(b) } else { pad })
        } else {
            dirs
        }
    }).collect()
}

/// The stick for a facing-relative direction.
fn stick(dir: Dir, facing_right: bool) -> u16 {
    let (fwd, back) = if facing_right { (RIGHT, LEFT) } else { (LEFT, RIGHT) };
    match dir {
        Dir::Neutral => 0,
        Dir::Forward => fwd,
        Dir::Back | Dir::ChargeBack => back,
        Dir::Up | Dir::Jump => UP,
        Dir::Down | Dir::ChargeDown => DOWN,
        Dir::UpForward => UP | fwd,
        Dir::UpBack => UP | back,
        Dir::DownForward => DOWN | fwd,
        Dir::DownBack => DOWN | back,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RYU_SLOTS: [SpecialSlot; 3] = [SpecialSlot::ryu(0x19), SpecialSlot::ryu(0x1a), SpecialSlot::ryu(0x1b)];

    fn body(x: i16) -> Body {
        Body { present: true, x, y: 184, move_id: 1, state: 0x12, hurt: (x - 10, x + 10), health: 48, front: x, ..Body::default() }
    }

    fn ctx(level: Level) -> Context {
        Context::new(level, Kit::of(TournamentFighter::Ryu, RulesetPreset::ClassicCompetitive, Some(RYU_SLOTS)), &CoreToggles::for_preset(RulesetPreset::ClassicCompetitive))
    }

    /// Exact baseline player-table SpecialSlot records from data_bank.rs:
    /// Gen4318-19, Birdie4747-48, Lee3886-87. Empty slots have zero records
    /// and no corresponding registry entry; no shoto role flags substitute.
    fn sc21_own_advance_slots(fighter: TournamentFighter) -> [SpecialSlot; 3] {
        let empty = SpecialSlot { tracker: 0, kick: false, melee: false, low_in_air: false, score: 0 };
        let dp = SpecialSlot { tracker: 0x19, kick: false, melee: true, low_in_air: false, score: 0x45f52 };
        let forward = SpecialSlot { tracker: 0x1c, kick: false, melee: true, low_in_air: false, score: 0x4345e };
        let back_kick = SpecialSlot { tracker: 0x16, kick: true, melee: true, low_in_air: false, score: 0x46012 };
        match fighter {
            TournamentFighter::Gen => [dp, forward, empty],
            TournamentFighter::Birdie => [empty, SpecialSlot { low_in_air: true, ..forward }, back_kick],
            TournamentFighter::Lee => [empty, forward, back_kick],
            _ => panic!("this fixture covers only the three proposed baseline variants"),
        }
    }

    fn sc21_own_advance_context(fighter: TournamentFighter, revision: crate::era::Revision, configured: bool, bank: bool, pure_footsies: bool) -> Context {
        let mut context = Context::new(Level::VeryHard,
            Kit::of(fighter, RulesetPreset::DefinitiveOverhaul, bank.then(|| sc21_own_advance_slots(fighter))),
            &CoreToggles::definitive())
            .with_baseline(crate::era::StylePick::for_revision(revision, crate::era::Loadout::Standard), false);
        context.stronger = configured;
        context.pure_footsies = pure_footsies;
        context
    }

    fn sc21_is_own_advance_plan(plan: &game::GamePlan) -> bool {
        plan.steps.iter().any(|step| step.act == game::Act::Do(Choice::Special(Role::Advance)))
    }

    fn sc21_advance_test_gap(plan: &game::GamePlan) -> i16 {
        assert!(plan.gap.0 <= plan.gap.1, "a declared spacing interval exists");
        let gap = match plan.steps.first().map(|step| step.act) {
            Some(game::Act::Space(target)) => target,
            _ => 80i16.clamp(plan.gap.0, plan.gap.1),
        };
        assert!((plan.gap.0..=plan.gap.1).contains(&gap), "the Space target is also a legal selection gap");
        gap
    }

    fn sc21_advance_view(gap: i16) -> Percept {
        Percept { frame: 88, fighting: true, me: body(300), foe: body(310 + gap), ..Default::default() }
    }

    #[test]
    fn sc21_baseline_advance_kit_contains_only_its_actual_native_registry_specials() {
        use crate::moves::Dir::{Down, DownBack, DownForward, Forward};
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            let kit = Kit::of(fighter, RulesetPreset::DefinitiveOverhaul, Some(sc21_own_advance_slots(fighter)));
            assert!(kit.projectile.is_none() && kit.projectile_alt.is_none());
            let advance: Vec<_> = kit.by_role(Role::Advance).into_iter().flatten()
                .map(|entry| (entry.name, entry.produces, entry.input.motion, entry.input.button)).collect();
            match fighter {
                TournamentFighter::Gen => {
                    assert_eq!(advance, vec![("HYAKURENKO", 0x1a, &[Down, DownForward, Forward][..], ButtonClass::Punch)]);
                    assert_eq!(kit.anti_air.map(|entry| (entry.name, entry.produces)), Some(("GEKIRO", 0x19)));
                }
                TournamentFighter::Birdie => {
                    assert_eq!(advance, vec![("BULL HORN", 0x1b, &[Down, DownBack, crate::moves::Dir::Back][..], ButtonClass::Kick)]);
                    assert_eq!(kit.anti_air.map(|entry| (entry.name, entry.produces)), Some(("BULL HEAD", 0x1a)), "its real low_in_air flag currently makes Bull Head AntiAir");
                }
                TournamentFighter::Lee => {
                    assert_eq!(advance, vec![("ZESSHOUOHKEN", 0x1a, &[Down, DownForward, Forward][..], ButtonClass::Punch),
                        ("DOUBLE KICK", 0x1b, &[Down, DownBack, crate::moves::Dir::Back][..], ButtonClass::Kick)]);
                    assert!(kit.anti_air.is_none());
                }
                _ => unreachable!(),
            }
            assert!(kit.by_role(Role::Advance).into_iter().flatten().all(|entry| matches!(entry.recognizer, Recognizer::Tracker(_))), "no later-era move enters the baseline kit");
            let missing = Kit::of(fighter, RulesetPreset::DefinitiveOverhaul, None);
            assert!(missing.by_role(Role::Advance).into_iter().all(|entry| entry.is_none()), "a missing real player bank supplies no special");
        }
    }

    #[test]
    fn sc21_three_baseline_variants_have_playable_plain_advance_pressure_at_declared_ranges() {
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
                for (configured, forced) in [(true, false), (false, true)] {
                    let raw = sc21_own_advance_context(fighter, revision, configured, true, false);
                    let context = raw.for_stronger();
                    let table = game::plans(context.kit.fighter, context.family);
                    for situation in [Situation::NeutralClose, Situation::NeutralFar] {
                        let (index, entry) = table.iter().enumerate().find(|(_, plan)| plan.situation == situation && sc21_is_own_advance_plan(plan))
                            .expect("each proposed baseline variant has its own neutral Advance pressure plan");
                        assert!(index >= 11, "existing plan records 0..10 retain their indices");
                        let spacing = usize::from(matches!(entry.steps.first().map(|step| step.act), Some(game::Act::Space(_))));
                        if spacing != 0 {
                            assert!(situation == Situation::NeutralFar && matches!(fighter, TournamentFighter::Gen | TournamentFighter::Lee), "only these stationary far-special plans add purposeful spacing");
                        }
                        assert_eq!(entry.steps.len(), spacing + 3, "one optional Space followed by the same plain pressure sequence");
                        let tail = &entry.steps[spacing..];
                        assert_eq!(tail[0].act, game::Act::Do(Choice::Special(Role::Advance)));
                        let game::Act::Reach { crouch, button } = tail[1].act else { panic!("the special follows with a plain normal's reach") };
                        assert!(button < 6, "the follow-up uses a real normal button");
                        assert_eq!(tail[2].act, game::Act::Do(Choice::Normal { crouch, button }), "the follow-up presses the same normal whose reach it approaches");
                        assert!(entry.steps.iter().all(|step| !matches!(step.act, game::Act::Route(_) | game::Act::Tool(_) | game::Act::Era(_) | game::Act::Rush | game::Act::Do(Choice::Cancel { .. } | Choice::Throw))), "plain pressure adds no forbidden offense or later-era mechanic");
                        // Already at a real Space target, follow_game_plan can
                        // advance through spacing and emit the actual motion.
                        let view = sc21_advance_view(sc21_advance_test_gap(entry));
                        let mut selected = 0;
                        let mut button_scripts = 0;
                        for seed in 1..=96 {
                            let mut cpu = FairCpu::seeded(seed).force_stronger(forced);
                            cpu.judged.situation = situation;
                            cpu.observe(view);
                            cpu.observe(Percept { frame: 99, ..view });
                            if cpu.start_game_plan(100, view, &context) && cpu.game.is_some_and(|run| usize::from(run.plan) == index) {
                                selected += 1;
                                let plan = cpu.follow_game_plan(100, view, &context).expect("the selected Advance action starts its real motion");
                                assert_eq!(plan.kind, PlanKind::Act);
                                assert!(plan.len > 0, "an available role yields an actual pad script");
                                button_scripts += usize::from(plan.pads[..usize::from(plan.len)].iter().any(|pad| pad & BUTTONS != 0));
                            }
                        }
                        assert!(selected > 0 && button_scripts > 0, "{fighter:?}/{revision:?}/{situation:?} configured{configured} forced{forced}: a real special pressure plan is selected and emits command buttons");
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_advance_pressure_is_not_selected_without_its_bank_or_under_pure_footsies() {
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
                for (configured, forced) in [(true, false), (false, true)] {
                    for (bank, pure) in [(false, false), (true, true)] {
                        let context = sc21_own_advance_context(fighter, revision, configured, bank, pure).for_stronger();
                        let table = game::plans(context.kit.fighter, context.family);
                        for situation in [Situation::NeutralClose, Situation::NeutralFar] {
                            let mut gaps = vec![80];
                            gaps.extend(table.iter().filter(|plan| plan.situation == situation && sc21_is_own_advance_plan(plan))
                                .map(sc21_advance_test_gap));
                            for gap in gaps {
                                for seed in 1..=96 {
                                    let mut cpu = FairCpu::seeded(seed).force_stronger(forced);
                                    cpu.judged.situation = situation;
                                    if cpu.start_game_plan(100, sc21_advance_view(gap), &context) {
                                        let run = cpu.game.expect("a selected plan has its real table record");
                                        assert!(!sc21_is_own_advance_plan(&table[usize::from(run.plan)]), "{fighter:?}/{revision:?}/{situation:?} bank{bank} pure{pure}: unavailable special cannot crowd out legal plans");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_advance_variant_tables_leave_other_pairs_and_off_selection_alone() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &FairCpu| { let mut out = CanonicalHasher::new(); cpu.encode(&mut out); out.finish() };
        for fighter in TournamentFighter::ALL {
            if !matches!(fighter, TournamentFighter::Gen | TournamentFighter::Birdie | TournamentFighter::Lee) {
                for family in [crate::era::EraFamily::Sf1987, crate::era::EraFamily::Sf1Plus] {
                    assert!(!game::plans(Some(fighter), Some(family)).iter().any(sc21_is_own_advance_plan), "other baseline pairs do not acquire this trial plan");
                }
            }
        }
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
                let tagged = sc21_own_advance_context(fighter, revision, false, true, false);
                let plain = Context { baseline: None, ..tagged };
                for seed in 1..=32 {
                    let mut a = FairCpu::seeded(seed);
                    let mut b = a;
                    a.judged.situation = Situation::NeutralClose;
                    b.judged.situation = Situation::NeutralClose;
                    assert_eq!(a.start_game_plan(100, sc21_advance_view(80), &plain), b.start_game_plan(100, sc21_advance_view(80), &tagged));
                    assert_eq!(hash(&a), hash(&b), "deferred baseline identity changes no OFF controller/RNG state");
                }
            }
            for family in [crate::era::EraFamily::Sf2, crate::era::EraFamily::SfZ, crate::era::EraFamily::Sf3, crate::era::EraFamily::Sf4, crate::era::EraFamily::Sf5, crate::era::EraFamily::Sf6] {
                assert!(!game::plans(Some(fighter), Some(family)).iter().any(|plan| plan.steps.windows(3).any(|tail|
                    tail[0].act == game::Act::Do(Choice::Special(Role::Advance))
                    && tail[1].act == game::Act::Reach { crouch: true, button: 3 }
                    && tail[2].act == game::Act::Do(Choice::Normal { crouch: true, button: 3 }))), "later-era tables do not acquire the baseline trial sequence");
            }
        }
    }


    /// Both views describe the same delayed decision. Frame99 deliberately
    /// reverses pressure as well as facing, so borrowing the whole newest
    /// Body/Percept for motion would change execution-error/RNG behavior.
    fn sc21_special_facing_fixture(context: &Context, forced: bool, left: bool, pressure: bool, seed: u64) -> (FairCpu, Percept, Percept) {
        let toward = if left { -1 } else { 1 };
        let mut view = percept(88, body(300 + toward * if pressure { 40 } else { 180 }));
        view.me.facing_left = left;
        view.me.health = if pressure { 24 } else { 48 };
        view.foe.facing_left = !left;
        let mut cpu = FairCpu::seeded(seed).force_stronger(forced);
        cpu.fighter = context.kit.fighter;
        cpu.last_view = view;
        cpu.last_normal = Some(Normal { crouch: false, button: 0, pressed: 80, from: Some(300), best: 50 });
        for frame in 1..=98 {
            cpu.observe(Percept { frame, ..view });
        }
        let own_x = if left { 0 } else { 600 };
        let mut latest = percept(99, body(own_x - toward * 180));
        latest.me = Body { facing_left: !left, health: if pressure { 48 } else { 1 }, ..body(own_x) };
        latest.foe.health = if pressure { 1 } else { 48 };
        latest.foe.facing_left = left;
        latest.foe.move_id = 0x10;
        latest.foe.state = 0xc2;
        latest.foe.active = true;
        latest.foe.front = own_x;
        (cpu, view, latest)
    }

    fn sc21_special_facing_hash(cpu: &FairCpu) -> u64 {
        use crate::canonical::{Canonical, CanonicalHasher};
        let mut out = CanonicalHasher::new();
        cpu.encode(&mut out);
        out.finish()
    }

    #[test]
    fn sc21_baseline_special_motion_feels_only_current_own_facing() {
        use crate::era::Revision::{Arcade1987, Sf1Plus};
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            for revision in [Arcade1987, Sf1Plus] {
                for (configured, forced) in [(true, false), (false, true)] {
                    let mut context = sc21_own_advance_context(fighter, revision, configured, true, false).for_stronger();
                    assert!(context.kit.by_role(Role::Advance).into_iter().any(|entry| entry.is_some()), "the real native Advance kit is present");
                    for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                        context.level = level;
                        for left in [false, true] {
                            for pressure in [false, true] {
                                for answer in [false, true] {
                                    for seed in 1..=16 {
                                        let (base, delayed, latest) = sc21_special_facing_fixture(&context, forced, left, pressure, seed);
                                        let (mut actual, mut expected) = (base, base);
                                        actual.observe(latest);
                                        // With no frame99, the reference uses the delayed
                                        // view verbatim, except for the known own facing.
                                        let facing_corrected = Percept { me: Body { facing_left: latest.me.facing_left, ..delayed.me }, ..delayed };
                                        expected.plan = expected.plan_for(Choice::Special(Role::Advance), 100, facing_corrected, &context, answer);
                                        actual.plan = actual.plan_for(Choice::Special(Role::Advance), 100, delayed, &context, answer);
                                        assert!(expected.plan.pads[..usize::from(expected.plan.len)].iter().any(|pad| pad & BUTTONS != 0));
                                        assert!(expected.plan.pads[..usize::from(expected.plan.len)].iter().any(|pad| pad & (LEFT | RIGHT) != 0), "native advance motion distinguishes the two facings");
                                        assert_eq!(actual.plan, expected.plan, "{fighter:?}/{revision:?}/{level:?} configured{configured} forced{forced} left{left} pressure{pressure} answer{answer} seed{seed}: use current own facing, with delayed positions/health/foe and the same execution error");
                                        assert_eq!(actual.rng, expected.rng, "facing alone consumes no extra randomness");
                                        // Normalize only fixture history before comparing all
                                        // controller state written by the real callsite.
                                        expected.observe(latest);
                                        assert_eq!(sc21_special_facing_hash(&actual), sc21_special_facing_hash(&expected), "all other special-plan state remains exact");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_special_motion_keeps_off_and_later_delayed_facing_and_full_state() {
        use crate::era::Revision::{Arcade1987, Sf1Plus};
        let check = |context: Context, forced: bool| {
            assert!(context.kit.by_role(Role::Advance).into_iter().any(|entry| entry.is_some()), "a real special is needed for the facing control");
            for left in [false, true] {
                for pressure in [false, true] {
                    for answer in [false, true] {
                        for seed in 1..=16 {
                            let (base, delayed, latest) = sc21_special_facing_fixture(&context, forced, left, pressure, seed);
                            let (mut actual, mut expected) = (base, base);
                            actual.observe(latest);
                            expected.plan = expected.plan_for(Choice::Special(Role::Advance), 100, delayed, &context, answer);
                            actual.plan = actual.plan_for(Choice::Special(Role::Advance), 100, delayed, &context, answer);
                            assert_eq!(expected.plan.kind, PlanKind::Act);
                            assert!(expected.plan.len > 0, "a later-era execution slip may omit the button, but it still enters the real motion");
                            assert_eq!(actual.plan, expected.plan, "OFF/later specials keep the original delayed facing");
                            assert_eq!(actual.rng, expected.rng);
                            expected.observe(latest);
                            assert_eq!(sc21_special_facing_hash(&actual), sc21_special_facing_hash(&expected), "OFF/later whole CPU state stays exact");
                        }
                    }
                }
            }
        };
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            for revision in [Arcade1987, Sf1Plus] {
                check(sc21_own_advance_context(fighter, revision, false, true, false), false);
            }
        }
        let free = EraControl { free: true, ..EraControl::default() };
        for context in [sfz_ctx(Level::VeryHard, crate::era::Loadout::AIsm, free), sf3_ctx(Level::VeryHard), sf4_ctx(Level::VeryHard, free), sf5_ctx(Level::VeryHard, free), sf6_ctx(Level::VeryHard)] {
            for (configured, forced) in [(true, false), (false, true)] {
                check(Context { stronger: configured, ..context }, forced);
            }
        }
    }


    fn sc21_begin_special_clock(context: &Context, forced: bool, seed: u64) -> (FairCpu, Plan, u8, u8) {
        let table = game::plans(context.kit.fighter, context.family);
        let (index, entry) = table.iter().enumerate().find(|(_, plan)| {
            plan.situation == Situation::NeutralClose && plan.steps.iter().any(|step| step.act == game::Act::Do(Choice::Special(Role::Advance)))
        }).expect("real native close Advance plan");
        let at = entry.steps.iter().position(|step| step.act == game::Act::Do(Choice::Special(Role::Advance))).unwrap() as u8;
        let mut cpu = FairCpu::seeded(seed).force_stronger(forced);
        cpu.fighter = context.kit.fighter;
        for frame in 1..=99 { cpu.observe(Percept { frame, ..sc21_advance_view(80) }); }
        let mut pick = cpu;
        let produces = pick.pick_special(context.kit.by_role(Role::Advance)).map_or(0, |entry| entry.produces);
        cpu.game = Some(game::Running { plan: index as u8, step: at, acted: false, pressed: 0, began: 100, hit: false });
        let motion = cpu.follow_game_plan(100, cpu.seen(88).unwrap(), context).expect("the real table step generates its native motion");
        (cpu, motion, at, produces)
    }

    /// Reproduce the observed native boundary: motion starts at100, its
    /// first button is107, release111 starts the special. At120 the CPU's
    /// delayed view108 still shows the preceding non-attacking pose.
    /// A later fresh contact has no active flag left after collision.
    fn sc21_special_clock_observation(frame: u32, release: u32, produces: u8, connected: bool) -> Percept {
        let mut view = Percept { frame, ..sc21_advance_view(80) };
        if connected && frame >= release {
            view.me.move_id = produces;
            view.me.state = 0xc2;
        }
        if connected && frame >= release + 12 {
            view.foe.move_id = 0x30;
            view.foe.state = 0x82;
            view.foe.health = 36;
        }
        view
    }

    #[test]
    fn sc21_baseline_special_outcome_waits_for_the_generated_command_to_reach_its_delayed_view() {
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
                for (configured, forced) in [(true, false), (false, true)] {
                    let context = sc21_own_advance_context(fighter, revision, configured, true, false).for_stronger();
                    let (mut cpu, motion, special_step, produces) = sc21_begin_special_clock(&context, forced, 17);
                    let first = motion.pads[..usize::from(motion.len)].iter().position(|pad| pad & BUTTONS != 0).expect("real native button") as u32;
                    let last = motion.pads[..usize::from(motion.len)].iter().rposition(|pad| pad & BUTTONS != 0).unwrap() as u32;
                    let release = 100 + last + 1;
                    assert_eq!((first, last, motion.len), (7, 10, 20), "the clean native three-step script from the raw witness");
                    cpu.plan = motion;
                    for offset in 0..u32::from(motion.len) {
                        let step = 100 + offset;
                        assert_eq!(cpu.next_pad(step, cpu.seen(88).unwrap(), &context), motion.pads[offset as usize]);
                        cpu.observe(sc21_special_clock_observation(step, release, produces, true));
                    }
                    let boundary = 100 + u32::from(motion.len);
                    let delayed = cpu.seen(boundary - context.level.reaction()).unwrap();
                    assert_eq!(delayed.frame, 108);
                    assert!(!delayed.me.attacking());
                    assert!(cpu.seen(boundary - 1).unwrap().me.attacking(), "the native move is already running while its delayed start is unseen");
                    let pending = cpu.follow_game_plan(boundary, delayed, &context).expect("pending special outcome");
                    assert_eq!(cpu.game.unwrap().step, special_step, "{fighter:?}/{revision:?} configured{configured} forced{forced}: pre-release delayed pose is not a whiff; do not spend the next Reach during this special");
                    assert_eq!(cpu.game.unwrap().pressed, 100 + first, "anchor the outcome clock at the generated button, before native release/contact");
                    cpu.plan = pending;
                    assert_eq!(cpu.next_pad(boundary, delayed, &context), 0);

                    let start_seen_at = release + context.level.reaction();
                    for frame in boundary..start_seen_at { cpu.observe(sc21_special_clock_observation(frame, release, produces, true)); }
                    let seen_start = cpu.seen(release).unwrap();
                    let pending = cpu.follow_game_plan(start_seen_at, seen_start, &context).expect("seen special still awaiting its result");
                    assert_eq!(cpu.game.unwrap().step, special_step);
                    assert!(!cpu.game.unwrap().hit);
                    assert_eq!(pending.pads[..usize::from(pending.len)], [0]);

                    let contact = release + 12;
                    for frame in start_seen_at..contact + context.level.reaction() { cpu.observe(sc21_special_clock_observation(frame, release, produces, true)); }
                    let seen_contact = cpu.seen(contact).unwrap();
                    assert!(!seen_contact.me.active && seen_contact.foe.reacting());
                    let _ = cpu.follow_game_plan(contact + context.level.reaction(), seen_contact, &context);
                    assert!(cpu.game.is_some_and(|run| run.hit), "fresh damage/reaction after the actual button still credits the special, even when collision consumed its active flag");
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_special_outcome_still_lets_a_failed_command_finish_after_its_grace() {
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
                for (configured, forced) in [(true, false), (false, true)] {
                    let context = sc21_own_advance_context(fighter, revision, configured, true, false).for_stronger();
                    let (mut cpu, motion, special_step, produces) = sc21_begin_special_clock(&context, forced, 17);
                    let first = motion.pads[..usize::from(motion.len)].iter().position(|pad| pad & BUTTONS != 0).unwrap() as u32;
                    let release = 100 + motion.pads[..usize::from(motion.len)].iter().rposition(|pad| pad & BUTTONS != 0).unwrap() as u32 + 1;
                    cpu.plan = motion;
                    for offset in 0..u32::from(motion.len) {
                        let step = 100 + offset;
                        let _ = cpu.next_pad(step, cpu.seen(88).unwrap(), &context);
                        cpu.observe(sc21_special_clock_observation(step, release, produces, false));
                    }
                    let boundary = 100 + u32::from(motion.len);
                    let _ = cpu.follow_game_plan(boundary, cpu.seen(boundary - context.level.reaction()).unwrap(), &context);
                    assert_eq!(cpu.game.unwrap().step, special_step, "a command not yet seen complete is not already a failed execution");
                    let failure_view = 100 + first + 7;
                    for frame in boundary..failure_view + context.level.reaction() { cpu.observe(sc21_special_clock_observation(frame, release, produces, false)); }
                    let _ = cpu.follow_game_plan(failure_view + context.level.reaction(), cpu.seen(failure_view).unwrap(), &context);
                    assert!(cpu.game.is_some_and(|run| run.step != special_step && !run.hit), "a genuinely absent native attack eventually takes the whiff continuation rather than waiting forever");
                }
            }
        }
    }

    #[test]
    fn sc21_special_outcome_clock_keeps_off_and_normal_action_origins() {
        for fighter in [TournamentFighter::Gen, TournamentFighter::Birdie, TournamentFighter::Lee] {
            for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
                let off = sc21_own_advance_context(fighter, revision, false, true, false).for_stronger();
                let (cpu, _, _, _) = sc21_begin_special_clock(&off, false, 17);
                assert_eq!(cpu.game.unwrap().pressed, 100, "OFF preserves the original motion-start clock");
                let mut missing = sc21_own_advance_context(fighter, revision, true, false, false).for_stronger();
                missing.kit.advance = None;
                missing.kit.advance_alt = None;
                let (cpu, empty, _, _) = sc21_begin_special_clock(&missing, false, 17);
                assert_eq!(empty.len, 0);
                assert_eq!(cpu.game.unwrap().pressed, 100, "an absent button cannot invent a future origin");
            }
        }
        let check_normal = |context: Context, forced: bool| {
            let table = game::plans(context.kit.fighter, context.family);
            let (index, entry) = table.iter().enumerate().find(|(_, plan)| plan.steps.iter().any(|step| matches!(step.act, game::Act::Do(Choice::Normal { .. })))).expect("existing real normal step");
            let at = entry.steps.iter().position(|step| matches!(step.act, game::Act::Do(Choice::Normal { .. }))).unwrap() as u8;
            let mut cpu = FairCpu::seeded(17).force_stronger(forced);
            for frame in 1..=99 { cpu.observe(Percept { frame, ..sc21_advance_view(40) }); }
            cpu.game = Some(game::Running { plan: index as u8, step: at, acted: false, pressed: 0, began: 100, hit: false });
            let pads = cpu.follow_game_plan(100, cpu.seen(88).unwrap(), &context).expect("real normal inputs");
            assert!(pads.pads[..usize::from(pads.len)].iter().any(|pad| pad & BUTTONS != 0));
            assert_eq!(cpu.game.unwrap().pressed, 100, "normal outcome timing is unchanged");
        };
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true), (false, false)] {
                check_normal(sc21_own_advance_context(TournamentFighter::Gen, revision, configured, true, false).for_stronger(), forced);
            }
        }
        let free = EraControl { free: true, ..EraControl::default() };
        for context in [sfz_ctx(Level::VeryHard, crate::era::Loadout::AIsm, free), sf3_ctx(Level::VeryHard), sf4_ctx(Level::VeryHard, free), sf5_ctx(Level::VeryHard, free), sf6_ctx(Level::VeryHard)] {
            check_normal(context, false);
        }
    }


    fn sc21_hit_context(fighter: TournamentFighter, revision: crate::era::Revision, level: Level) -> Context {
        let mut context = Context::new(level, Kit::of(fighter, RulesetPreset::DefinitiveOverhaul, None), &CoreToggles::definitive())
            .with_baseline(crate::era::StylePick::for_revision(revision, crate::era::Loadout::Standard), false)
            .for_stronger();
        context.stronger = true;
        context
    }

    #[test]
    fn sc21_baseline_it_hit_selection_stops_at_each_limited_levels_pressure_cap() {
        for fighter in TournamentFighter::ALL {
            for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
                for level in [Level::Easy, Level::Normal, Level::Hard] {
                    let context = sc21_hit_context(fighter, revision, level);
                    for seed in 1..=32 {
                        let mut cpu = FairCpu::seeded(seed);
                        cpu.judged.situation = Situation::ItHit;
                        cpu.pressure_run = level.pressure_plans();
                        let rng_before = cpu.rng.0;
                        assert!(!cpu.start_game_plan(100, percept(88, body(340)), &context), "{fighter:?}/{revision:?}/{level:?}/{seed}: post-hit pressure obeys the existing cap");
                        assert!(cpu.game.is_none());
                        assert_eq!(cpu.pressure_run, level.pressure_plans());
                        assert_eq!(cpu.rng.0, rng_before, "no eligible pressure plan consumes no selection randomness");
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_it_hit_selection_charges_the_existing_pressure_counter() {
        for fighter in TournamentFighter::ALL {
            for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
                for level in [Level::Easy, Level::Normal, Level::Hard] {
                    let context = sc21_hit_context(fighter, revision, level);
                    let mut selected = 0;
                    for seed in 1..=32 {
                        let mut cpu = FairCpu::seeded(seed);
                        cpu.judged.situation = Situation::ItHit;
                        cpu.pressure_run = level.pressure_plans() - 1;
                        if cpu.start_game_plan(100, percept(88, body(340)), &context) {
                            selected += 1;
                            assert_eq!(cpu.pressure_run, level.pressure_plans(), "{fighter:?}/{revision:?}/{level:?}: a post-hit plan counts as pressure");
                        }
                    }
                    assert!(selected > 0, "fixed samples exercise a real selected post-hit plan");
                }
            }
        }
    }

    #[test]
    fn sc21_it_hit_pressure_policy_keeps_switch_off_and_later_era_selection_unchanged() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &FairCpu| { let mut out = CanonicalHasher::new(); cpu.encode(&mut out); out.finish() };
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let plain = Context::new(Level::Normal, Kit::of(TournamentFighter::Eagle, RulesetPreset::DefinitiveOverhaul, None), &CoreToggles::definitive());
            let attached = plain.with_baseline(crate::era::StylePick::for_revision(revision, crate::era::Loadout::Standard), false);
            for seed in 1..=32 {
                let mut a = FairCpu::seeded(seed);
                a.judged.situation = Situation::ItHit;
                a.pressure_run = Level::Normal.pressure_plans();
                let mut b = a;
                assert_eq!(a.start_game_plan(100, percept(88, body(340)), &plain), b.start_game_plan(100, percept(88, body(340)), &attached));
                assert_eq!(hash(&a), hash(&b), "the off switch's deferred baseline identity changes no selection state");
            }
        }
        // Canonical SF6 Ken has an affordable ItHit Super plan; its existing nonpressure
        // classification remains independent of the baseline-only correction.
        let config = crate::config::MatchConfig::definitive();
        let mut state = EraFighterState::default();
        state.fighter = Some(TournamentFighter::Ken);
        state.style = crate::era::StylePick::for_revision(crate::era::Revision::Sf6, crate::era::Loadout::Standard);
        state.max_sub_hp = crate::era_fight::FULL_SUB_HP;
        state.set_gauge_mode(crate::era::GaugeMode::EraDefault);
        state.super_meter = 3 * crate::era::STOCK_AT;
        let mut context = Context::with_era(Level::Normal, TournamentFighter::Ken, Some(RYU_SLOTS), config, &state, EraControl { free: true, ..Default::default() }).for_stronger();
        context.stronger = true;
        assert!(era_ready(game::EraAct::Super, &context), "the canonical shoto's real live kit and three-stock meter make its existing Super eligible");
        let mut view = percept(88, body(340));
        view.me.health = 24;
        let mut selected = 0;
        for seed in 1..=32 {
            let mut uncapped = FairCpu::seeded(seed);
            uncapped.judged.situation = Situation::ItHit;
            let mut capped = uncapped;
            capped.pressure_run = Level::Normal.pressure_plans();
            let a = uncapped.start_game_plan(100, view, &context);
            let b = capped.start_game_plan(100, view, &context);
            assert_eq!(a, b, "later-era ItHit selection retains its previous cap behavior");
            assert_eq!(uncapped.rng.0, capped.rng.0);
            assert_eq!(uncapped.game, capped.game);
            if a { selected += 1; assert_eq!(hash(&uncapped), hash(&capped)); }
        }
        assert!(selected > 0, "the later-era control actually selects its existing ItHit plan");
    }

    /// Follow the real baseline ItHit table's Normal step, already pressed.
    /// The active flag is deliberately absent: a native collision may consume
    /// it before the percept is read. The old reaction at the press is retained
    /// only in the controller's existing observation ring.
    fn sc21_follow_hit_normal(context: &Context, at_press: Option<(bool, u16)>, now_reacting: bool, now_hp: u16) -> FairCpu {
        let mut cpu = FairCpu::seeded(17);
        let situation = if context.baseline_rules() { Situation::ItHit } else { Situation::NeutralClose };
        let (plan, entry) = game::plans(context.kit.fighter, context.family).iter().enumerate()
            .find(|(_, plan)| plan.situation == situation && plan.steps.iter().any(|step| matches!(step.act, game::Act::Do(Choice::Normal { .. })))).expect("real pair's plain Normal plan");
        let step = entry.steps.iter().position(|step| matches!(step.act, game::Act::Do(Choice::Normal { .. }))).expect("plain follow-up Normal");
        cpu.game = Some(game::Running { plan: plan as u8, step: step as u8, acted: true, pressed: 100, began: 96, hit: false });
        let own = Body { move_id: 0x0f, state: 0xc2, active: false, ..body(300) };
        if let Some((reacting, hp)) = at_press {
            let foe = Body { move_id: if reacting { 0x30 } else { 1 }, state: if reacting { 0x82 } else { 0x12 }, health: hp, ..body(340) };
            cpu.observe(Percept { me: own, ..percept(100, foe) });
        }
        let foe = Body { move_id: if now_reacting { 0x30 } else { 1 }, state: if now_reacting { 0x82 } else { 0x12 }, health: now_hp, ..body(340) };
        let view = Percept { me: own, ..percept(112, foe) };
        cpu.observe(view);
        let _ = cpu.follow_game_plan(124, view, context);
        cpu
    }

    fn sc21_run_was_credited(cpu: &FairCpu) -> bool {
        cpu.game.is_some_and(|run| run.hit) || cpu.plan_worked.iter().any(|&worked| worked != 0)
    }

    #[test]
    fn sc21_baseline_it_hit_normal_does_not_claim_an_unchanged_old_reaction_as_a_new_hit() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let context = sc21_hit_context(TournamentFighter::Eagle, revision, Level::VeryHard);
            let cpu = sc21_follow_hit_normal(&context, Some((true, 48)), true, 48);
            assert!(!sc21_run_was_credited(&cpu), "{revision:?}: old unchanged reaction cannot credit a new normal");
            assert!(cpu.game.is_some_and(|run| run.acted), "the still-attacking normal waits for a genuine outcome");
        }
    }

    #[test]
    fn sc21_baseline_it_hit_normal_accepts_new_visible_damage_or_a_fresh_reaction_without_an_active_box() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let context = sc21_hit_context(TournamentFighter::Eagle, revision, Level::VeryHard);
            for (old_reaction, old_hp, new_hp) in [(true, 48, 44), (false, 48, 48)] {
                let cpu = sc21_follow_hit_normal(&context, Some((old_reaction, old_hp)), true, new_hp);
                assert!(sc21_run_was_credited(&cpu), "{revision:?}: new visible evidence credits the hit even if collision consumed the active flag");
            }
        }
    }

    #[test]
    fn sc21_baseline_it_hit_normal_with_missing_press_history_does_not_invent_a_hit() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let context = sc21_hit_context(TournamentFighter::Eagle, revision, Level::VeryHard);
            let cpu = sc21_follow_hit_normal(&context, None, true, 44);
            assert!(!sc21_run_was_credited(&cpu), "{revision:?}: missing history cannot establish fresh damage/reaction");
        }
    }

    #[test]
    fn sc21_it_hit_outcome_correction_keeps_off_and_later_existing_reaction_semantics() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let mut off = sc21_hit_context(TournamentFighter::Eagle, revision, Level::VeryHard);
            off.stronger = false;
            let cpu = sc21_follow_hit_normal(&off, Some((true, 48)), true, 48);
            assert!(sc21_run_was_credited(&cpu), "the effective OFF path keeps its old private normal-outcome semantics");
        }
        let later = sf6_ctx(Level::VeryHard).for_stronger();
        let cpu = sc21_follow_hit_normal(&later, Some((true, 48)), true, 48);
        assert!(sc21_run_was_credited(&cpu), "later-era normal outcomes keep their established behavior");
    }


    #[derive(Clone, Copy, Debug)]
    enum Sc21ApproachThreat {
        Startup,
        Active,
        Recovery,
        Projectile,
        AirAttack,
    }

    fn sc21_approach_context(revision: crate::era::Revision, stronger: bool) -> Context {
        let mut context = Context::new(
            Level::VeryHard,
            Kit::of(
                TournamentFighter::Eagle,
                RulesetPreset::DefinitiveOverhaul,
                None,
            ),
            &CoreToggles::definitive(),
        )
        .with_baseline(
            crate::era::StylePick::for_revision(revision, crate::era::Loadout::Standard),
            false,
        );
        context.stronger = stronger;
        context
    }

    /// Populate observations and the judge from standing, startup, active and
    /// recovery poses. Step100 sees frame88; its own newest pose is frame99.
    /// No proposed safety predicate is reproduced in this fixture.
    fn sc21_approach_history(
        context: &Context,
        threat: Sc21ApproachThreat,
        foe_x: i16,
        latest_own_x: i16,
        seed: u64,
    ) -> (FairCpu, Percept) {
        let mut cpu = FairCpu::seeded(seed);
        cpu.fighter = context.kit.fighter;
        cpu.focus = Focus::Attack;
        let mut before = None;
        for frame in 1..=99 {
            let mut view = Percept {
                frame,
                fighting: true,
                me: body(if frame > 88 { latest_own_x } else { 200 }),
                foe: body(foe_x),
                ..Percept::default()
            };
            view.foe.facing_left = true;
            if frame >= 40 {
                match threat {
                    Sc21ApproachThreat::Projectile => {
                        view.shot = true;
                        view.shot_x = 250;
                        view.shot_half = 8;
                    }
                    threat => {
                        view.foe.move_id = 0x10;
                        view.foe.state = if matches!(threat, Sc21ApproachThreat::AirAttack) {
                            0xc3
                        } else {
                            0xc2
                        };
                        view.foe.front = foe_x - 80;
                        view.foe.active = match threat {
                            Sc21ApproachThreat::Startup => false,
                            Sc21ApproachThreat::Recovery => (45..=50).contains(&frame),
                            _ => frame >= 45,
                        };
                    }
                }
            }
            cpu.observe(view);
            let reach = cpu.longest_reach();
            cpu.judged = cpu.judge.judge(view, before, reach);
            before = Some(view);
        }
        let view = cpu.seen(88).unwrap();
        (cpu, view)
    }

    /// Install an actual table step by its action, not its score or index.
    /// Running/Plan are the existing private snapshot state used by decide.
    fn sc21_install_approach_step(
        cpu: &mut FairCpu,
        context: &Context,
        matches: impl Fn(game::Act) -> bool,
    ) {
        let (plan, step) = game::plans(context.kit.fighter, context.family)
            .iter()
            .enumerate()
            .find_map(|(plan, entry)| {
                entry
                    .steps
                    .iter()
                    .position(|step| matches(step.act))
                    .map(|step| (plan, step))
            })
            .expect("the real pair's table contains the requested step");
        cpu.game = Some(game::Running {
            plan: plan as u8,
            step: step as u8,
            acted: false,
            pressed: 0,
            began: 96,
            hit: false,
        });
        cpu.plan = Plan::default();
        cpu.settle = 0;
    }

    /// Observe the opponent only through frame88 when deciding step100.
    /// The CPU's newest own pose is frame99. The opponent's newer actual
    /// position deliberately lies opposite the expected delayed target.
    fn sc21_crossed_own_walk_history(context: &Context, mirrored: bool, meaty: bool) -> FairCpu {
        let mut cpu = FairCpu::seeded(17);
        cpu.fighter = context.kit.fighter;
        let (old_own, latest_own, latest_foe) = if mirrored { (440, 200, 120) } else { (200, 440, 520) };
        let mut before = None;
        for frame in 1..=99 {
            let mut view = Percept {
                frame,
                fighting: true,
                me: body(if frame <= 88 { old_own } else { latest_own }),
                foe: body(if frame <= 88 { 320 } else { latest_foe }),
                ..Percept::default()
            };
            if meaty && frame >= 80 {
                view.foe.move_id = 0x38;
                view.foe.state = 0x31;
            }
            cpu.observe(view);
            if frame <= 88 {
                let reach = cpu.longest_reach();
                cpu.judged = cpu.judge.judge(view, before, reach);
                before = Some(view);
            }
        }
        sc21_install_approach_step(&mut cpu, context, |act| {
            if meaty { matches!(act, game::Act::Meaty { .. }) }
            else { matches!(act, game::Act::Reach { .. }) }
        });
        cpu
    }

    #[test]
    fn sc21_baseline_reach_walks_toward_delayed_foe_from_latest_own_position() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let context = sc21_approach_context(revision, true).for_stronger();
            for (mirrored, expected) in [(false, LEFT), (true, RIGHT)] {
                let mut cpu = sc21_crossed_own_walk_history(&context, mirrored, false);
                assert_eq!(cpu.press(100, true, &context), expected,
                    "{revision:?}/{mirrored}: after its own crossing, Reach must walk toward the same delayed foe");
                assert_eq!(cpu.last_view().frame, 88);
                assert_eq!(cpu.last_view().foe.x, 320, "a newer opponent position must not steer this decision");
                assert!(cpu.following_game_plan());
            }
        }
    }

    #[test]
    fn sc21_baseline_meaty_walks_toward_delayed_foe_from_latest_own_position() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let context = sc21_approach_context(revision, true).for_stronger();
            for (mirrored, expected) in [(false, LEFT), (true, RIGHT)] {
                let mut cpu = sc21_crossed_own_walk_history(&context, mirrored, true);
                assert_eq!(cpu.press(100, true, &context), expected,
                    "{revision:?}/{mirrored}: an early meaty approach must close from its latest own position");
                assert_eq!(cpu.last_view().frame, 88);
                assert_eq!(cpu.last_view().foe.x, 320, "the downed opponent remains delayed");
                assert!(cpu.following_game_plan());
            }
        }
    }

    #[test]
    fn sc21_reach_direction_change_keeps_off_and_later_family_walks_unchanged() {
        for context in [
            sc21_approach_context(crate::era::Revision::Arcade1987, false).for_stronger(),
            sc21_approach_context(crate::era::Revision::Sf1Plus, false).for_stronger(),
            sf6_ctx(Level::VeryHard).for_stronger(),
        ] {
            for meaty in [false, true] {
                for (mirrored, legacy) in [(false, RIGHT), (true, LEFT)] {
                    let mut cpu = sc21_crossed_own_walk_history(&context, mirrored, meaty);
                    assert_eq!(cpu.press(100, true, &context), legacy,
                        "{:?}/{mirrored}/{meaty}: OFF and later families retain their original delayed-own walk direction", context.family);
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_queued_felt_forward_walk_stops_for_a_newly_seen_near_attack() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let context = sc21_approach_context(revision, true).for_stronger();
            for (mirrored, queued_forward, seen_foe_x) in [(false, LEFT, 390), (true, RIGHT, 250)] {
                let mut cpu = sc21_crossed_own_walk_history(&context, mirrored, false);
                let mut seen = cpu.seen(88).unwrap();
                seen.foe = Body { move_id: 0x10, state: 0xc2, ..body(seen_foe_x) };
                cpu.observe(seen);
                cpu.focus = Focus::Attack;
                // One pad of a four-frame felt-forward walk has already run.
                // Delayed own/foe gap180 avoids the separate reactive guard;
                // latest own/that SAME delayed foe has near gap40, so the
                // existing far-attack exception cannot license another pad.
                cpu.plan = Plan::of(PlanKind::Walk, &[queued_forward; 4]);
                cpu.plan.at = 1;
                cpu.pad = queued_forward;
                let run = cpu.game.as_mut().unwrap();
                run.acted = true;
                run.pressed = 99;
                assert_eq!(cpu.press(100, true, &context), 0,
                    "{revision:?}/{mirrored}: a known nearby attack cancels a queued walk toward the delayed foe from the latest own position");
                assert_eq!(cpu.last_view().frame, 88);
                assert_eq!(cpu.last_view().foe.x, seen_foe_x);
                assert!(cpu.last_view().foe.attacking(), "the new attack was actually shown, not hidden by focus delay");
                // History99 still places the actual foe520/120, opposite the
                // delayed target. Fresh opponent aiming would miss this stop.
            }
        }
    }

    #[test]
    fn baseline_reach_and_pending_walk_keep_advancing_past_far_ground_attack_poses() {
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            let raw = sc21_approach_context(revision, true);
            let context = raw.for_stronger();
            for threat in [
                Sc21ApproachThreat::Startup,
                Sc21ApproachThreat::Active,
                Sc21ApproachThreat::Recovery,
            ] {
                let (mut cpu, view) = sc21_approach_history(&context, threat, 500, 200, 17);
                sc21_install_approach_step(&mut cpu, &context, |act| {
                    matches!(act, game::Act::Reach { .. })
                });
                let reach = cpu.game_plan();
                let mut pending = cpu;
                // This is decide's actual follow-then-emit chain: both gates
                // must permit the next forward frame, not just one helper.
                cpu.plan = cpu
                    .follow_game_plan(100, view, &context)
                    .expect("a far ground attack does not discard baseline Reach");
                assert_eq!(cpu.plan.kind, PlanKind::Walk, "{revision:?} {threat:?}");
                assert_eq!(
                    cpu.next_pad(100, view, &context),
                    RIGHT,
                    "{revision:?} {threat:?}: the returned walk is actually emitted"
                );
                assert_eq!(cpu.game_plan(), reach, "the same Reach remains active");

                // A multi-frame walk already in progress reaches next_pad
                // through press/decide, without following a new table step.
                pending.plan = Plan::of(PlanKind::Walk, &[RIGHT; 4]);
                assert_eq!(
                    pending.press(100, true, &raw),
                    RIGHT,
                    "{revision:?} {threat:?}: pending walk survives the real decision path"
                );
                assert_eq!(
                    pending.press(101, true, &raw),
                    RIGHT,
                    "{revision:?} {threat:?}: continuation is not one accidental frame"
                );
                assert_eq!(pending.game_plan(), reach);
            }
        }
    }

    #[test]
    fn baseline_approach_still_stops_for_near_attacks_projectiles_and_air_attacks() {
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            let raw = sc21_approach_context(revision, true);
            let context = raw.for_stronger();
            for (threat, foe_x) in [
                (Sc21ApproachThreat::Active, 270),
                (Sc21ApproachThreat::Projectile, 500),
                (Sc21ApproachThreat::AirAttack, 500),
            ] {
                let (mut cpu, view) = sc21_approach_history(&context, threat, foe_x, 200, 17);
                sc21_install_approach_step(&mut cpu, &context, |act| {
                    matches!(act, game::Act::Reach { .. })
                });
                let mut pending = cpu;
                assert!(
                    cpu.follow_game_plan(100, view, &context).is_none(),
                    "{revision:?} {threat:?}: the committed Reach ends"
                );
                assert!(cpu.game.is_none());
                pending.plan = Plan::of(PlanKind::Walk, &[RIGHT; 4]);
                assert_eq!(
                    pending.press(100, true, &raw),
                    0,
                    "{revision:?} {threat:?}: the pending walk is not emitted into danger"
                );
                assert!(pending.plan.done());
            }
        }
    }

    #[test]
    fn baseline_approach_uses_latest_own_position_and_a_margin_beyond_learned_reach() {
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            let raw = sc21_approach_context(revision, true);
            let context = raw.for_stronger();
            // The delayed own pose is far away. The latest own hurtbox is
            // 81px from the opponent, whose actual observed attack reached80.
            let (mut cpu, view) =
                sc21_approach_history(&context, Sc21ApproachThreat::Active, 500, 409, 17);
            assert_eq!(view.me.x, 200);
            assert_eq!(cpu.seen(99).unwrap().me.x, 409);
            assert_eq!(
                cpu.judge.foe_edge(),
                80,
                "the judge learned a real shown box"
            );
            sc21_install_approach_step(&mut cpu, &context, |act| {
                matches!(act, game::Act::Reach { .. })
            });
            let mut pending = cpu;
            assert!(
                cpu.follow_game_plan(100, view, &context).is_none(),
                "a delayed far pose does not license walking through the safety margin"
            );
            assert!(cpu.game.is_none());
            pending.plan = Plan::of(PlanKind::Walk, &[RIGHT; 4]);
            assert_eq!(
                pending.press(100, true, &raw),
                0,
                "pending walk also uses felt own spacing"
            );
        }
    }

    #[test]
    fn baseline_without_a_game_plan_can_approach_a_far_ground_attack_without_starting_an_attack() {
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            let context = sc21_approach_context(revision, true).for_stronger();
            for threat in [Sc21ApproachThreat::Startup, Sc21ApproachThreat::Active] {
                let mut advanced = false;
                for seed in 1..=64 {
                    let (mut cpu, view) = sc21_approach_history(&context, threat, 500, 200, seed);
                    assert!(cpu.game.is_none());
                    cpu.plan = cpu.choose(100, view, &context);
                    assert!(cpu.plan.pads[..usize::from(cpu.plan.len)].iter().all(|pad| pad & BUTTONS == 0), "{revision:?} {threat:?}: safety is movement, not attacking into an active foe");
                    if cpu.plan.kind == PlanKind::Walk && cpu.plan.pads[0] == RIGHT {
                        assert_eq!(
                            cpu.next_pad(100, view, &context),
                            RIGHT,
                            "the scored approach actually reaches the pad"
                        );
                        advanced = true;
                    }
                }
                assert!(advanced, "{revision:?} {threat:?}: a real forward approach remains available, without pinning its selection frequency");
            }
        }
    }

    #[test]
    fn baseline_far_attack_exception_never_commits_normals_jump_ins_or_routes() {
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            let context = sc21_approach_context(revision, true).for_stronger();
            for wanted in 0..3 {
                if wanted == 2 && revision == crate::era::Revision::Arcade1987 {
                    continue;
                }
                let requested = |act| match wanted {
                    0 => matches!(act, game::Act::Do(Choice::Normal { .. })),
                    1 => act == game::Act::Do(Choice::JumpIn),
                    _ => act == game::Act::Route(crate::route_book::Reach::Ground),
                };
                let (mut cpu, view) =
                    sc21_approach_history(&context, Sc21ApproachThreat::Active, 300, 200, 17);
                sc21_install_approach_step(&mut cpu, &context, requested);
                assert!(
                    cpu.follow_game_plan(100, view, &context).is_none(),
                    "{revision:?} action{wanted}: far-away attacks permit only movement"
                );
                assert!(cpu.game.is_none() && cpu.route.is_none());
            }
        }
    }

    #[test]
    fn baseline_switch_off_and_later_sf6_keep_their_existing_walk_abort_behavior() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            let attached = sc21_approach_context(revision, false);
            let plain = Context::new(attached.level, attached.kit, &CoreToggles::definitive());
            let (mut original, _) =
                sc21_approach_history(&plain, Sc21ApproachThreat::Active, 500, 200, 17);
            original.plan = Plan::of(PlanKind::Walk, &[RIGHT; 4]);
            let mut with_identity = original;
            assert_eq!(
                original.press(100, true, &plain),
                RIGHT,
                "the old scorer's existing walk is not aborted"
            );
            assert_eq!(
                with_identity.press(100, true, &attached),
                RIGHT,
                "the off switch retains that exact behavior"
            );
            assert_eq!(
                hash(&original),
                hash(&with_identity),
                "deferred baseline identity changes no old controller state"
            );
        }
        let context = sf6_ctx(Level::VeryHard).for_stronger();
        for threat in [Sc21ApproachThreat::Startup, Sc21ApproachThreat::Active] {
            let (mut cpu, view) = sc21_approach_history(&context, threat, 500, 200, 17);
            sc21_install_approach_step(&mut cpu, &context, |act| {
                matches!(act, game::Act::Reach { .. })
            });
            let mut pending = cpu;
            assert!(
                cpu.follow_game_plan(100, view, &context).is_none(),
                "SF6 keeps its committed-step abort"
            );
            pending.plan = Plan::of(PlanKind::Walk, &[RIGHT; 4]);
            assert_eq!(
                pending.next_pad(100, view, &context),
                0,
                "SF6 keeps its pending-walk abort"
            );
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Sc21LearnHistory {
        Adjacent,
        Missing,
        Jumped,
        Overflow,
    }

    /// The observed Eagle native capacity samples, replayed through decide's
    /// existing learning call. An idle pad sequence protects last_normal from
    /// being replaced by a newly chosen action while the four poses pass.
    fn sc21_decide_normal_reach_trace(
        context: &Context,
        forced: bool,
        button: usize,
        first_x: i16,
        active_tip: i16,
        mirrored: bool,
        history: Sc21LearnHistory,
    ) -> FairCpu {
        let mirror = |x| if mirrored { 600 - x } else { x };
        let view = |frame, x, attacking, active| {
            let mut me = body(mirror(x));
            me.facing_left = mirrored;
            if attacking {
                me.move_id = if button == 3 { 0x14 } else { 0x0f };
                me.state = 0xc2;
                me.active = active;
                me.front = mirror(active_tip);
            }
            Percept {
                frame,
                fighting: true,
                me,
                foe: body(mirror(500)),
                ..Percept::default()
            }
        };
        let mut cpu = FairCpu::seeded(17).force_stronger(forced);
        cpu.fighter = context.kit.fighter;
        cpu.plan = Plan::of(PlanKind::Idle, &[0; 12]);
        cpu.settle = 1_000;
        cpu.reach[0][button] = 77;
        cpu.reach_seen[0][button] = false;
        for frame in 1..=40 {
            cpu.observe(view(frame, 124, false, false));
        }
        cpu.last_normal = Some(Normal {
            crouch: false,
            button,
            pressed: 40,
            from: None,
            best: 0,
        });
        let before = match history {
            Sc21LearnHistory::Adjacent => Some(view(40, 124, false, false)),
            Sc21LearnHistory::Missing => None,
            Sc21LearnHistory::Jumped => Some(view(38, 124, false, false)),
            Sc21LearnHistory::Overflow => Some(view(u32::MAX, 124, false, false)),
        };
        let poses = [
            view(41, first_x, true, false),
            view(42, first_x, true, true),
            view(43, first_x, true, false),
            view(44, 124, false, false),
        ];
        let mut prior = before;
        for shown in poses {
            cpu.observe(shown);
            assert_eq!(
                cpu.decide(shown.frame + 12, shown, prior, context),
                0,
                "the protected pending idle plan emits no replacement normal"
            );
            prior = Some(shown);
        }
        cpu
    }

    #[test]
    fn effective_baseline_normal_reach_is_learned_from_the_adjacent_standing_center() {
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            for (configured, forced) in [(true, false), (false, true)] {
                let context = sc21_approach_context(revision, configured).for_stronger();
                for mirrored in [false, true] {
                    // Native opening center124: LK first pose132/tip212;
                    // MP first pose112/tip224. Expected stance-relative
                    // reaches are explicit evidence, not a copied predicate.
                    for (button, first_x, active_tip, expected) in
                        [(3, 132, 212, 88), (1, 112, 224, 100)]
                    {
                        let cpu = sc21_decide_normal_reach_trace(
                            &context,
                            forced,
                            button,
                            first_x,
                            active_tip,
                            mirrored,
                            Sc21LearnHistory::Adjacent,
                        );
                        assert_eq!(cpu.reach[0][button], expected, "{revision:?} configured{configured} forced{forced} mirrored{mirrored} button{button}: learned reach uses the stance coordinate that Reach/choose measure from");
                        assert!(cpu.reach_seen[0][button]);
                        assert!(
                            cpu.last_normal.is_none(),
                            "the completed native normal leaves one learned result"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn old_and_later_normal_learning_keep_the_first_drawn_attack_pose_origin() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            let off = sc21_approach_context(revision, false);
            let plain = Context::new(off.level, off.kit, &CoreToggles::definitive());
            for mirrored in [false, true] {
                for history in [
                    Sc21LearnHistory::Adjacent,
                    Sc21LearnHistory::Jumped,
                    Sc21LearnHistory::Overflow,
                ] {
                    for (button, first_x, active_tip, expected) in
                        [(3, 132, 212, 80), (1, 112, 224, 112)]
                    {
                        let old = sc21_decide_normal_reach_trace(
                            &plain, false, button, first_x, active_tip, mirrored, history,
                        );
                        let attached = sc21_decide_normal_reach_trace(
                            &off, false, button, first_x, active_tip, mirrored, history,
                        );
                        assert_eq!(
                            old.reach[0][button], expected,
                            "plain old scorer retains its existing pose origin"
                        );
                        assert_eq!(
                            attached.reach[0][button], expected,
                            "off baseline retains its existing pose origin"
                        );
                        assert_eq!(hash(&old), hash(&attached), "deferred {revision:?} identity changes no old controller state: mirrored{mirrored} {history:?} button{button}");
                        for forced in [false, true] {
                            let mut later = sf6_ctx(Level::VeryHard).for_stronger();
                            later.stronger = !forced;
                            let cpu = sc21_decide_normal_reach_trace(
                                &later, forced, button, first_x, active_tip, mirrored, history,
                            );
                            assert_eq!(
                                cpu.reach[0][button], expected,
                                "SF6 configured/forced stronger retains its established origin"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn effective_baseline_does_not_invent_a_stance_origin_across_missing_or_jumped_history() {
        for revision in [
            crate::era::Revision::Arcade1987,
            crate::era::Revision::Sf1Plus,
        ] {
            for (configured, forced) in [(true, false), (false, true)] {
                let context = sc21_approach_context(revision, configured).for_stronger();
                for mirrored in [false, true] {
                    for history in [
                        Sc21LearnHistory::Missing,
                        Sc21LearnHistory::Jumped,
                        Sc21LearnHistory::Overflow,
                    ] {
                        for (button, first_x, active_tip) in [(3, 132, 212), (1, 112, 224)] {
                            let cpu = sc21_decide_normal_reach_trace(
                                &context, forced, button, first_x, active_tip, mirrored, history,
                            );
                            assert_eq!(cpu.reach[0][button], 77, "{revision:?} configured{configured} forced{forced} mirrored{mirrored} {history:?} button{button}: incomplete pre-pose history preserves prior reach");
                            assert!(
                                !cpu.reach_seen[0][button],
                                "no sample was learned from an invented origin"
                            );
                            assert!(
                                cpu.last_normal.is_none(),
                                "the incomplete sample is abandoned"
                            );
                        }
                    }
                }
            }
        }
    }

    /// The same visible own normal keeps its direction while the delayed
    /// opponent crosses its drawn center. An idle plan protects the normal
    /// record while decide's real learning call observes every adjacent pose.
    fn sc21_decide_reach_after_foe_crosses(
        context: &Context,
        forced: bool,
        mirrored: bool,
        crossed: bool,
    ) -> FairCpu {
        let mirror = |x| if mirrored { 600 - x } else { x };
        let view = |frame, x, attacking, active| {
            let mut me = body(mirror(x));
            me.facing_left = mirrored;
            if attacking {
                me.move_id = 0x0f;
                me.state = 0xc2;
                me.active = active;
                me.front = mirror(370);
            }
            Percept {
                frame,
                fighting: true,
                me,
                // The own attack pose crosses a stationary foe's drawn center.
                foe: body(mirror(if crossed { 306 } else { 500 })),
                ..Percept::default()
            }
        };
        let mut cpu = FairCpu::seeded(17).force_stronger(forced);
        cpu.fighter = context.kit.fighter;
        cpu.plan = Plan::of(PlanKind::Idle, &[0; 12]);
        cpu.settle = 1_000;
        cpu.reach[0][1] = 77;
        cpu.reach_seen[0][1] = false;
        for frame in 1..=40 {
            cpu.observe(view(frame, 300, false, false));
        }
        cpu.last_normal = Some(Normal {
            crouch: false,
            button: 1,
            pressed: 40,
            from: None,
            best: 0,
        });
        let mut prior = Some(view(40, 300, false, false));
        for shown in [
            view(41, 312, true, false),
            view(42, 312, true, true),
            view(43, 312, true, false),
            view(44, 300, false, false),
        ] {
            cpu.observe(shown);
            assert_eq!(cpu.decide(shown.frame + 12, shown, prior, context), 0);
            prior = Some(shown);
        }
        assert!(cpu.last_normal.is_none(), "the completed normal was learned once");
        assert!(cpu.reach_seen[0][1]);
        cpu
    }

    #[test]
    fn sc21_effective_baseline_learns_own_normal_reach_when_the_delayed_foe_crosses() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                let context = sc21_approach_context(revision, configured).for_stronger();
                for mirrored in [false, true] {
                    for crossed in [false, true] {
                        let cpu = sc21_decide_reach_after_foe_crosses(&context, forced, mirrored, crossed);
                        // Stance center300, first attack center312, active tip370:
                        // seventy pixels in its own visible attack direction.
                        assert_eq!(cpu.reach[0][1], 70, "{revision:?} configured{configured} forced{forced} mirrored{mirrored} crossed{crossed}: own reach is independent of the opponent's side");
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_own_facing_reach_correction_keeps_off_and_later_learning_exact() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let off = sc21_approach_context(revision, false);
            let plain = Context::new(off.level, off.kit, &CoreToggles::definitive());
            for mirrored in [false, true] {
                for crossed in [false, true] {
                    let expected = if crossed { 16 } else { 58 };
                    let old = sc21_decide_reach_after_foe_crosses(&plain, false, mirrored, crossed);
                    let attached = sc21_decide_reach_after_foe_crosses(&off, false, mirrored, crossed);
                    assert_eq!(old.reach[0][1], expected);
                    assert_eq!(attached.reach[0][1], expected);
                    assert_eq!(hash(&old), hash(&attached), "{revision:?} mirrored{mirrored} crossed{crossed}: deferred baseline identity keeps the old controller exact");
                    for forced in [false, true] {
                        let mut later = sf6_ctx(Level::VeryHard).for_stronger();
                        later.stronger = !forced;
                        let cpu = sc21_decide_reach_after_foe_crosses(&later, forced, mirrored, crossed);
                        assert_eq!(cpu.reach[0][1], expected, "configured/forced later style keeps first-pose origin and target-side sign");
                    }
                }
            }
        }
    }

    #[test]
    fn baseline_identity_is_deferred_and_keeps_the_actual_offense_rules() {
        use crate::era::{EraFamily, Loadout, Revision, StylePick};
        for revision in [Revision::Arcade1987, Revision::Sf1Plus] {
            let style = StylePick::for_revision(revision, Loadout::Standard);
            for links in [false, true] {
                for cancels in [false, true] {
                    for throws in [false, true] {
                        let mut toggles = CoreToggles::definitive();
                        toggles.true_combos = links;
                        toggles.cancels = cancels;
                        toggles.throws = throws;
                        let raw = Context::new(Level::VeryHard, ctx(Level::VeryHard).kit, &toggles)
                            .with_baseline(style, true);
                        assert_eq!((raw.family, raw.style, raw.own_blockstun), (None, None, false));
                        assert_eq!((raw.true_combos, raw.cancels, raw.throws), (links, cancels, throws));
                        let stronger = raw.for_stronger();
                        assert_eq!(stronger.family, Some(style.family));
                        assert_eq!(stronger.style, Some((revision, Loadout::Standard)));
                        assert!(stronger.era.is_none() && stronger.own_blockstun);
                        let expected = if style.family == EraFamily::Sf1987 { (false, false, false) } else { (links, cancels, throws) };
                        assert_eq!((stronger.true_combos, stronger.cancels, stronger.throws), expected);
                        assert!(!stronger.era_offense(), "baseline identity never overrides offense rules");
                    }
                }
            }
        }
        for preset in [RulesetPreset::ClassicCompetitive, RulesetPreset::PureFootsies1987Plus] {
            let raw = Context::new(Level::VeryHard, ctx(Level::VeryHard).kit, &CoreToggles::for_preset(preset));
            let stronger = raw.with_baseline(StylePick::for_preset(preset), false).for_stronger();
            assert_eq!(stronger.family, Some(EraFamily::Sf1Plus));
            assert!(!stronger.true_combos && !stronger.cancels && !stronger.throws);
        }
    }

    #[test]
    fn a_deferred_baseline_identity_leaves_old_pads_and_controller_state_identical() {
        use crate::canonical::{Canonical, CanonicalHasher};
        use crate::era::{Loadout, Revision, StylePick};
        let hash = |cpu: &FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        for fighter in TournamentFighter::ALL {
            for revision in [Revision::Arcade1987, Revision::Sf1Plus] {
                for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                    let raw = Context::new(level, Kit::of(fighter, RulesetPreset::DefinitiveOverhaul, Some(RYU_SLOTS)), &CoreToggles::definitive());
                    let baseline = raw.with_baseline(StylePick::for_revision(revision, Loadout::Standard), true);
                    let mut old = FairCpu::seeded(17);
                    let mut attached = old;
                    for frame in 1..=160 {
                        let foe = if (frame / 40) % 2 == 0 { body(380) } else {
                            Body { move_id: 0x10, state: 0xc2, active: frame % 20 < 8, front: 310, ..body(350) }
                        };
                        let shown = percept(frame, foe);
                        old.observe(shown);
                        attached.observe(shown);
                        assert_eq!(old.press(frame + 1, true, &raw), attached.press(frame + 1, true, &baseline), "{fighter:?}/{revision:?}/{level:?}/{frame}");
                        assert_eq!(hash(&old), hash(&attached), "{fighter:?}/{revision:?}/{level:?}/{frame}: complete controller state");
                    }
                }
            }
        }
    }

    #[test]
    fn baseline_books_obey_independent_link_and_cancel_rules_and_missing_books_offer_no_route() {
        use crate::era::{Loadout, Revision, StylePick};
        use crate::route_book::Reach;
        for links in [false, true] {
            for cancels in [false, true] {
                let mut raw = ctx(Level::VeryHard);
                raw.true_combos = links;
                raw.cancels = cancels;
                let context = raw.with_baseline(StylePick::for_revision(Revision::Sf1Plus, Loadout::Standard), false).for_stronger();
                assert_eq!(baseline_route(&context, Reach::Ground), links && cancels);
                let mut cpu = FairCpu::seeded(3);
                let view = Percept { frame: 100, fighting: true, me: body(200), foe: body(252), ..Percept::default() };
                assert_eq!(cpu.take_route(&context, view, &[Reach::Ground], game::NEUTRAL_WALK), links && cancels,
                    "the player does not infer later-era combo rules from baseline identity");
            }
        }
        let mut geki = ctx(Level::VeryHard);
        geki.kit = Kit { fighter: Some(TournamentFighter::Geki), ..Kit::default() };
        geki.true_combos = true;
        geki.cancels = true;
        let geki = geki.with_baseline(StylePick::for_revision(Revision::Sf1Plus, Loadout::Standard), false).for_stronger();
        assert!(!baseline_route(&geki, Reach::Ground));
        let mut cpu = FairCpu::seeded(1);
        let view = percept(100, body(325));
        assert!(!cpu.take_route(&geki, view, &[Reach::Ground, Reach::Punish], game::NEUTRAL_WALK));
    }

    #[test]
    fn the_stronger_play_decides_from_its_own_reading_of_the_kit_and_a_mission_takes_it_away() {
        // SC15 (`#207`): where the cast's tables read a kit, the stronger
        // play takes that reading and the tools; without one it keeps the
        // old play's. A mission allowing only normals and throws takes both.
        let plain = ctx(Level::VeryHard);
        assert!(plain.for_stronger().kit.projectile.is_some(), "no reading of its own: the old one");
        let mut read = plain;
        let shot = plain.kit.projectile.unwrap();
        read.strong_kit = Some(Kit { fighter: Some(TournamentFighter::Joe), projectile: Some(shot), ..Kit::default() });
        read.tools = cast::Tools { command_throw: Some(shot), ..cast::Tools::default() };
        let stronger = read.for_stronger();
        assert_eq!(stronger.kit.fighter, Some(TournamentFighter::Joe));
        assert!(stronger.kit.anti_air.is_none() && stronger.tools.get(cast::Tool::CommandThrow).is_some());
        assert_eq!(read.kit.fighter, Some(TournamentFighter::Ryu), "the old play's own kit is left as it is");
        let footsies = Context { pure_footsies: true, ..read }.for_stronger();
        assert!(footsies.kit.projectile.is_none() && footsies.tools.get(cast::Tool::CommandThrow).is_none());
    }

    fn alpha_kit(fighter: TournamentFighter, loadout: crate::era::Loadout) -> Kit {
        let mut era = EraFighterState::default();
        era.fighter = Some(fighter);
        era.max_sub_hp = crate::era_fight::FULL_SUB_HP;
        era.style = crate::era::StylePick::for_revision(crate::era::Revision::Sfz3, loadout);
        let mut slots = RYU_SLOTS;
        if fighter == TournamentFighter::Sagat {
            slots[2] = SpecialSlot { kick: true, ..slots[1] };
        }
        Kit::of_alpha(fighter, RulesetPreset::DefinitiveOverhaul, Some(slots), &era)
    }

    fn percept(frame: u32, foe: Body) -> Percept {
        Percept { frame, fighting: true, me: body(300), foe, ..Percept::default() }
    }

    #[test]
    fn alpha_kit_uses_the_active_ground_commands_and_cancel_rules() {
        use crate::era::Loadout::{AIsm, XIsm};
        let adon = alpha_kit(TournamentFighter::Adon, AIsm);
        assert_eq!(adon.anti_air.unwrap().input.button, ButtonClass::Kick);
        assert_eq!(adon.advance.unwrap().input.motion, &[Dir::Back, Dir::Down, Dir::DownBack]);
        assert_eq!(adon.advance_alt.unwrap().name, "JAGUAR TOOTH");
        assert!(!adon.cancels_with(Role::Advance));
        let adon_x = alpha_kit(TournamentFighter::Adon, XIsm);
        assert_eq!(adon_x.advance.unwrap().name, "JAGUAR TOOTH");
        assert!(adon_x.advance_alt.is_none());

        let gen = alpha_kit(TournamentFighter::Gen, AIsm);
        assert_eq!(gen.anti_air.unwrap().name, "GEKIROU");
        assert_eq!(gen.advance.unwrap().input.button, ButtonClass::MashPunch);
        assert!(gen.cancels_with(Role::Advance));
        assert!(!gen.cancels_with(Role::AntiAir));
    }

    #[test]
    fn alpha_pad_scripts_complete_charge_release_mash_and_up_commands() {
        use crate::era::Loadout::{AIsm, XIsm};
        let birdie = alpha_kit(TournamentFighter::Birdie, AIsm);
        let head = alpha_motion(&birdie.advance.unwrap(), false, 0, true);
        let horn = alpha_motion(&birdie.advance_alt.unwrap(), false, 0, true);
        let charge = crate::moves::CHARGE_FRAMES;
        assert_eq!(&head[..charge], &[RIGHT; crate::moves::CHARGE_FRAMES]);
        assert_eq!(head[charge + 1], LEFT | button(0));
        assert_eq!(&horn[..charge], &[button(0) | button(1); crate::moves::CHARGE_FRAMES]);
        assert_eq!(horn[charge], 0);
        assert!(charge + 1 + Level::Easy.special_cost() < PLAN_CAP);

        let gen = alpha_kit(TournamentFighter::Gen, AIsm);
        let mash = alpha_motion(&gen.advance.unwrap(), true, 1, true);
        let presses: Vec<_> = mash.iter().enumerate().filter_map(|(tick, &pad)| (pad == button(1)).then_some(tick)).collect();
        assert_eq!(presses, [0, 4, 8, 12]);

        let sagat = alpha_kit(TournamentFighter::Sagat, XIsm);
        let tiger_crush = sagat.advance.unwrap();
        assert_eq!(tiger_crush.name, "TIGER CRUSH");
        let knee = alpha_motion(&tiger_crush, true, 0, true);
        let up = knee.iter().copied().find(|pad| pad & UP != 0);
        assert_eq!(up, Some(UP | RIGHT | button(3)));
    }

    #[test]
    fn the_levels_delays_sit_in_the_researchs_bands() {
        assert_eq!(Level::from_difficulty(1).reaction(), 27);
        assert_eq!(Level::from_difficulty(2).reaction(), 19);
        assert_eq!(Level::from_difficulty(3).reaction(), 15);
        assert_eq!(Level::from_difficulty(4).reaction(), 12);
        for d in 0..=5 {
            assert!(Level::from_difficulty(d).reaction() >= 12);
        }
    }

    #[test]
    fn the_roles_come_from_the_slot_records() {
        // Ryu's 1987 records: the Shoryuken rises, the Hadoken is thrown,
        // the Tatsumaki carries him in.
        let k = Kit::of(TournamentFighter::Ryu, RulesetPreset::DefinitiveOverhaul, Some(RYU_SLOTS));
        assert_eq!(k.anti_air.map(|e| e.name), Some("SHORYUKEN"));
        assert_eq!(k.projectile.map(|e| e.name), Some("HADOKEN"));
        assert_eq!(k.advance.map(|e| e.name), Some("TATSUMAKI"));
        // Retsu's Kiai Shot fills the Hadoken's slot with the Hadoken's
        // record: a projectile, whose short range the CPU learns.
        let r = Kit::of(TournamentFighter::Retsu, RulesetPreset::DefinitiveOverhaul, Some(RYU_SLOTS));
        assert_eq!(r.projectile.map(|e| e.name), Some("KIAI SHOT"));
        assert!(r.anti_air.is_none() && r.advance.is_none());
        // Geki's Teleport Strike is a melee special that is an overhead in
        // the air: it carries him in. His Shuriken is thrown.
        let teleport = SpecialSlot { tracker: 0x16, kick: false, melee: true, low_in_air: false, score: 0 };
        let g = Kit::of(TournamentFighter::Geki, RulesetPreset::DefinitiveOverhaul, Some([RYU_SLOTS[0], RYU_SLOTS[1], teleport]));
        assert_eq!(g.projectile.map(|e| e.name), Some("SHURIKEN"));
        assert_eq!(g.advance.map(|e| e.name), Some("TELEPORT STRIKE"));
        // Joe's Dynamite Straight is on the Shoryuken's rising tracker and
        // his Rolling Sobat on the Tatsumaki's.
        let dynamite = SpecialSlot { tracker: 0x19, kick: false, melee: true, low_in_air: false, score: 0 };
        let sobat = SpecialSlot { tracker: 0x16, kick: true, melee: true, low_in_air: false, score: 0 };
        let j = Kit::of(TournamentFighter::Joe, RulesetPreset::DefinitiveOverhaul, Some([dynamite, RYU_SLOTS[1], sobat]));
        assert!(j.projectile.is_none());
        assert_eq!(j.anti_air.map(|e| e.name), Some("DYNAMITE STRAIGHT"));
        assert_eq!(j.advance.map(|e| e.name), Some("ROLLING SOBAT"));
        // Lee's Zesshouohken and Double Kick both carry him in: the kit keeps
        // both so the CPU shares its advance role between them.
        let zesshou = SpecialSlot { tracker: 0x1c, kick: false, melee: true, low_in_air: false, score: 0 };
        let l = Kit::of(TournamentFighter::Lee, RulesetPreset::DefinitiveOverhaul, Some([RYU_SLOTS[0], zesshou, sobat]));
        assert_eq!(l.advance.map(|e| e.name), Some("ZESSHOUOHKEN"));
        assert_eq!(l.advance_alt.map(|e| e.name), Some("DOUBLE KICK"));
        // Without his table a boss has none of them.
        let none = Kit::of(TournamentFighter::Retsu, RulesetPreset::DefinitiveOverhaul, None);
        assert!(none.projectile.is_none());
    }

    /// The CPU's own fighter, attacking with its box reaching `reach` past
    /// where it started, for `frames` frames, then back in the stance.
    fn swing(cpu: &mut FairCpu, from: u32, reach: i16, frames: u32) -> u32 {
        let foe = body(400);
        let mut f = from;
        for i in 0..frames + 2 {
            let mut p = percept(f, foe);
            if i < frames {
                p.me.move_id = 0x0d;
                p.me.state = 0xc2;
                p.me.active = i > 1;
                p.me.front = 300 + reach;
            }
            cpu.observe(p);
            cpu.learn(p, cpu.seen(f - 1), false);
            f += 1;
        }
        f
    }

    #[test]
    fn a_normals_reach_is_learned_shorter_as_well_as_longer() {
        // A short-limbed fighter: his light punch reaches 30, not Ryu's 50.
        let mut cpu = FairCpu::new();
        cpu.observe(percept(1, body(400)));
        cpu.last_normal = Some(Normal { crouch: false, button: 0, pressed: 2, from: None, best: 0 });
        let f = swing(&mut cpu, 2, 30, 6);
        assert_eq!(cpu.reach[0][0], 30, "the first sighting replaces the guess");
        // One odd long reading moves it halfway, and does not stick.
        cpu.last_normal = Some(Normal { crouch: false, button: 0, pressed: f, from: None, best: 0 });
        let f = swing(&mut cpu, f, 70, 6);
        assert_eq!(cpu.reach[0][0], 50);
        cpu.last_normal = Some(Normal { crouch: false, button: 0, pressed: f, from: None, best: 0 });
        swing(&mut cpu, f, 30, 6);
        assert_eq!(cpu.reach[0][0], 40);
    }

    #[test]
    fn a_short_projectile_is_learned_as_short() {
        let mut cpu = FairCpu::new();
        assert!(cpu.shot_goes_far(), "not yet seen: thrown from far");
        for (f, dist) in [(1, 20), (2, 50), (3, 70), (4, 0)] {
            let mut p = percept(f, body(400));
            p.my_shot = dist > 0;
            p.my_shot_dist = dist;
            cpu.learn_shot(p);
        }
        assert_eq!(cpu.shot_range, Some(70));
        assert!(!cpu.shot_goes_far());
    }

    /// A CPU playing the stronger game on `cpu`, pressed through frames
    /// `from..to` of a fight in round `round`, Player 1 standing at 200.
    fn fight_frames(cpu: &mut FairCpu, c: &mut Context, round: u8, from: u32, to: u32) {
        c.round = round;
        for f in from..to {
            cpu.observe(percept(f, body(200)));
            cpu.press(f + 1, true, c);
        }
    }

    #[test]
    fn the_habit_memory_starts_empty_every_match_and_lasts_all_its_rounds() {
        // SC9 (`#201`): a new match is the first round's start, in every
        // mode (the round number is the machine's, whatever the mode).
        let mut c = sf6_ctx(Level::VeryHard);
        let mut cpu = FairCpu::new();
        let mut at = 1;
        let fight = |cpu: &mut FairCpu, c: &mut Context, at: &mut u32, round: u8, frames: u32| {
            fight_frames(cpu, c, round, *at, *at + frames);
            *at += frames;
        };
        let pause = |cpu: &mut FairCpu, c: &Context, at: &mut u32, frames: u32| {
            for f in *at..*at + frames {
                cpu.press(f + 1, false, c);
            }
            *at += frames;
        };
        fight(&mut cpu, &mut c, &mut at, 0, 60);
        cpu.habits.record(Spot::Grabbed, GrabAnswer::NotTeched as usize);
        cpu.plan_worked[0] = 2;
        // A moment with no round fought inside the first round (a fighter
        // not drawn, a frame missing): nothing is forgotten.
        pause(&mut cpu, &c, &mut at, 20);
        fight(&mut cpu, &mut c, &mut at, 0, 60);
        assert_eq!(cpu.habits().sightings(), 1, "kept through a short break in the first round");
        // Between rounds, then the second round.
        pause(&mut cpu, &c, &mut at, BETWEEN_FIGHTS as u32 + 30);
        fight(&mut cpu, &mut c, &mut at, 1, 60);
        assert_eq!(cpu.habits().sightings(), 1, "kept into the match's second round");
        assert_eq!(cpu.plan_worked[0], 2, "what worked is kept into the second round");
        // Between matches, then the next match's first round.
        pause(&mut cpu, &c, &mut at, BETWEEN_FIGHTS as u32 + 30);
        fight(&mut cpu, &mut c, &mut at, 0, 60);
        assert!(cpu.habits().is_empty(), "the next match's first round starts with nothing noted");
        assert_eq!(cpu.plan_worked, [0; game::PLAN_SLOTS]);
        // A new fighter is a new opponent's CPU: nothing is kept.
        cpu.habits.record(Spot::Grabbed, GrabAnswer::NotTeched as usize);
        let mut other = sf6_ctx(Level::VeryHard);
        other.kit.fighter = Some(TournamentFighter::Ken);
        fight(&mut cpu, &mut other, &mut at, 1, 30);
        assert!(cpu.habits().is_empty());
    }

    #[test]
    fn a_game_plan_won_through_an_answer_counts_as_having_worked() {
        // SC9 (`#201`): the stand-and-watch plan's win is the anti-air its
        // answer to his jump throws, not one of its own steps.
        let running = game::Running { plan: 12, step: 1, acted: true, pressed: 90, began: 80, hit: false };
        let (mut hit_him, mut missed) = (FairCpu::new(), FairCpu::new());
        for cpu in [&mut hit_him, &mut missed] {
            cpu.game = Some(running);
            cpu.answer_ends_game_plan(100);
            assert!(cpu.game.is_none() && cpu.last_plan == 13);
        }
        let mut struck = percept(120, body(280));
        struck.foe.move_id = 0x31;
        struck.me.move_id = 0x19;
        struck.me.state = 0xc1;
        hit_him.credit_answer(120, struck);
        assert_eq!(hit_him.plan_worked[12], 1, "its answer landed");
        missed.plan_worked[12] = 2;
        missed.credit_answer(120, percept(120, body(280)));
        assert_eq!(missed.plan_worked[12], 2, "still waiting to see");
        missed.credit_answer(100 + ANSWER_WATCH + 1, percept(200, body(280)));
        assert_eq!(missed.plan_worked[12], 0, "nothing landed in time");
        assert_eq!(missed.answered_plan, None);
    }

    #[test]
    fn at_easy_a_full_habit_memory_changes_nothing_it_does() {
        // Easy notes but never leans on what it notes (SC9, `#201`): the
        // same fight with a memory full of habits and with an empty one
        // gives the same pads.
        let mut c = sf6_ctx(Level::Easy);
        let mut empty = FairCpu::seeded(7);
        fight_frames(&mut empty, &mut c, 0, 1, 40);
        let mut full = empty;
        for (spot, answer) in [
            (Spot::AfterShot, ShotAnswer::Jumped as usize),
            (Spot::Rising, RiseAnswer::Reversal as usize),
            (Spot::AfterGuard, GuardAnswer::KeptGuarding as usize),
            (Spot::Grabbed, GrabAnswer::NotTeched as usize),
            (Spot::JumpIn, JumpFrom::Mid as usize),
        ] {
            for _ in 0..6 {
                full.habits.record(spot, answer);
            }
        }
        let mut pads = [Vec::new(), Vec::new()];
        for (k, cpu) in [&mut empty, &mut full].into_iter().enumerate() {
            for f in 40..1_200u32 {
                // Player 1 walks in, jumps in and swings now and then.
                let mut foe = body(360 - (f % 300) as i16 / 2);
                match f % 97 {
                    0..=2 => foe.move_id = 9,
                    3..=30 => {
                        foe.move_id = 0xa;
                        foe.state = 1;
                    }
                    50..=58 => {
                        foe.move_id = 0x10;
                        foe.state = 0xc2;
                    }
                    _ => {}
                }
                cpu.observe(percept(f, foe));
                pads[k].push(cpu.press(f + 1, true, &c));
            }
        }
        assert_eq!(pads[0], pads[1]);
        assert!(pads[0].iter().any(|&p| p != 0), "it played");
    }

    #[test]
    fn the_canonical_hash_covers_the_habit_memory_and_what_worked() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        let plain = FairCpu::new();
        let mut changed = [plain; 4];
        changed[0].habits.record(Spot::Rising, RiseAnswer::Button as usize);
        changed[1].plan_worked[3] = 1;
        changed[2].last_plan = 2;
        changed[3].route_landed = 4;
        let mut answered = plain;
        answered.answered_plan = Some((2, 90));
        for cpu in changed.iter().chain([&answered]) {
            assert_ne!(hash(cpu), hash(&plain));
        }
    }

    #[test]
    fn nothing_seen_more_recently_than_the_delay_changes_the_pad() {
        // Two CPUs with the same past; one is then shown an attack (or a
        // walk-in under the modern rules). Their pads stay the same until the
        // frame is `reaction` frames old.
        for preset in [RulesetPreset::ClassicCompetitive, RulesetPreset::DefinitiveOverhaul] {
            for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                let c = Context::new(
                    level,
                    Kit::of(TournamentFighter::Ryu, preset, Some(RYU_SLOTS)),
                    &CoreToggles::for_preset(preset),
                );
                let mut a = FairCpu::new();
                for f in 1..=40 {
                    a.observe(percept(f, body(360)));
                }
                for f in 41..=60 {
                    a.press(f, true, &c);
                    a.observe(percept(f, body(360)));
                }
                let mut b = a;
                let mut attack = body(360);
                attack.move_id = 0x10;
                attack.state = 0xc2;
                let shown = 61;
                let mut first_diff = None;
                for f in shown..shown + 120 {
                    let (pa, pb) = (a.press(f, true, &c), b.press(f, true, &c));
                    if pa != pb && first_diff.is_none() {
                        first_diff = Some(f);
                    }
                    a.observe(percept(f, body(360)));
                    b.observe(percept(f, attack));
                }
                let f = first_diff.unwrap_or_else(|| panic!("{preset:?} {level:?}: the attack never changed the pad"));
                assert!(f >= shown + level.reaction(), "{preset:?} {level:?}: answered on {f}, shown on {shown}");
            }
        }
    }

    #[test]
    fn with_the_stronger_play_nothing_seen_more_recently_than_the_applicable_delay_changes_the_pad() {
        // The two-frame rule (SC7, `#199`). The applicable delay is the focused
        // one for a new action of the opponent's that is the CPU's focus and
        // for everything that is not a new action (positions, health, its
        // own fighter), and the slower one for any other new action.
        //
        // Two stronger CPUs with the same past, the opponent out of reach,
        // so each watches for a jump. One is then shown a jump (its focus),
        // an attack (a new action, not its focus) or the opponent standing
        // closer (no new action). Their pads stay the same until the frame
        // is the applicable delay old. The jump and the attack are answered
        // on that frame in some of the tries; the step in, which nothing
        // answers at once, changes the pad before the slower delay in some.
        let mut jump = body(440);
        jump.move_id = 9;
        let mut attack = body(440);
        attack.move_id = 0x10;
        attack.state = 0xc2;
        for preset in [RulesetPreset::ClassicCompetitive, RulesetPreset::DefinitiveOverhaul] {
            for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                let mut c = Context::new(
                    level,
                    Kit::of(TournamentFighter::Ryu, preset, Some(RYU_SLOTS)),
                    &CoreToggles::for_preset(preset),
                );
                c.stronger = true;
                let mut answered_at_once = std::collections::HashMap::new();
                for seed in 1..=24u64 {
                    for (shown_body, delay, what) in [
                        (jump, level.reaction(), "jump"),
                        (attack, level.unfocused_reaction(), "attack"),
                        (body(340), level.reaction(), "step in"),
                    ] {
                        let mut a = FairCpu::seeded(seed);
                        for f in 1..=40 {
                            a.observe(percept(f, body(440)));
                        }
                        for f in 41..=80 {
                            a.press(f, true, &c);
                            a.observe(percept(f, body(440)));
                        }
                        assert_eq!(a.attention().focus, Focus::Jump, "{preset:?} {level:?}: watching for a jump");
                        let mut b = a;
                        let shown = 81;
                        let mut first_diff = None;
                        for f in shown..shown + 120 {
                            let (pa, pb) = (a.press(f, true, &c), b.press(f, true, &c));
                            if pa != pb && first_diff.is_none() {
                                first_diff = Some(f);
                            }
                            a.observe(percept(f, body(440)));
                            b.observe(percept(f, shown_body));
                        }
                        // The level's guard chance may leave one unanswered.
                        let Some(f) = first_diff else { continue };
                        let why = format!("{preset:?} {level:?} seed {seed}: the {what}");
                        assert!(f >= shown + delay, "{why}: answered on {f}, shown on {shown}");
                        let at_once = if what == "step in" { f < shown + level.unfocused_reaction() } else { f == shown + delay };
                        if at_once {
                            *answered_at_once.entry(what).or_insert(0) += 1;
                        }
                    }
                }
                for what in ["jump", "attack", "step in"] {
                    assert!(
                        answered_at_once.get(what).is_some_and(|&n| n >= 1),
                        "{preset:?} {level:?}: the {what} answered at its delay: {answered_at_once:?}"
                    );
                }
            }
        }
    }

    /// Ryu in the SF6 era style with its own gauges as a fight starts them
    /// (a full Drive gauge) and two bars of Super meter, free to act,
    /// playing the stronger game.
    fn sf6_ctx(level: Level) -> Context {
        sf6_ctx_with(level, EraControl { free: true, ..EraControl::default() })
    }

    /// The same, its own fighter's control as given.
    fn sf6_ctx_with(level: Level, control: EraControl) -> Context {
        sf6_ctx_drive(level, control, None)
    }

    /// The same, with `drive` points in its Drive gauge (full with `None`).
    fn sf6_ctx_drive(level: Level, control: EraControl, drive: Option<u16>) -> Context {
        use crate::era::{GaugeMode, Loadout, Revision, StylePick, STOCK_AT};
        let config = crate::config::MatchConfig::definitive();
        let mut state = EraFighterState::default();
        state.fighter = Some(TournamentFighter::Ryu);
        state.style = StylePick::for_revision(Revision::Sf6, Loadout::Standard);
        state.max_sub_hp = crate::era_fight::FULL_SUB_HP;
        state.set_gauge_mode(GaugeMode::EraDefault);
        state.set_sf6_practice_drive(drive);
        state.super_meter = 2 * STOCK_AT;
        let mut c = Context::with_era(level, TournamentFighter::Ryu, Some(RYU_SLOTS), config, &state, control);
        c.set_opponent(crate::era::EraFamily::Sf6, crate::era_fight::FULL_SUB_HP);
        c.stronger = true;
        c
    }

    #[test]
    fn with_the_stronger_play_no_era_command_answers_anything_sooner_than_the_applicable_delay() {
        // SC8 (`#200`): Ryu in the SF6 era style with meter to spend. Two
        // stronger CPUs with the same past; one is then shown an attack in
        // its reach (a Drive Parry or a Drive Impact can answer it) or a
        // Drive Rush from afar (a Drive Impact can). Their pads stay the
        // same until the frame is the applicable delay old: the focused
        // one if it watches for an attack as it decides, else the slower
        // one. Over the tries the answers include the era commands.
        let mut attack = body(340);
        attack.move_id = 0x10;
        attack.state = 0xc2;
        let mut rush = body(420);
        rush.move_id = crate::era_fight::sf6::RUSH;
        rush.state = 0xc2;
        let parry = button(1) | button(4);
        let impact = button(2) | button(5);
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            let c = sf6_ctx(level);
            let mut era_answers = std::collections::HashMap::new();
            for seed in 1..=24u64 {
                for (rest, shown_body, what) in [(body(340), attack, "attack"), (body(420), rush, "Drive Rush")] {
                    let mut a = FairCpu::seeded(seed);
                    for f in 1..=40 {
                        a.observe(percept(f, rest));
                    }
                    for f in 41..=80 {
                        a.press(f, true, &c);
                        a.observe(percept(f, rest));
                    }
                    let mut b = a;
                    let shown = 81;
                    // The earliest it may answer: once the slower delay has
                    // passed, or once the focused one has while it watches
                    // for an attack as it decides (SC7's rule; its focus can
                    // come to the attack after it began).
                    let mut earliest = shown + level.unfocused_reaction();
                    let (mut first_diff, mut era) = (None, false);
                    for f in shown..shown + 120 {
                        if f >= shown + level.reaction() && f < earliest && b.attention().focus == Focus::Attack {
                            earliest = f;
                        }
                        let (pa, pb) = (a.press(f, true, &c), b.press(f, true, &c));
                        if pa != pb {
                            first_diff.get_or_insert(f);
                            era |= pb & parry == parry || pb & impact == impact;
                        }
                        a.observe(percept(f, rest));
                        b.observe(percept(f, shown_body));
                    }
                    // The level's guard chance may leave one unanswered.
                    let Some(f) = first_diff else { continue };
                    assert!(f >= earliest, "{level:?} seed {seed}: the {what} answered on {f}, shown on {shown}, earliest {earliest}");
                    if era {
                        *era_answers.entry(what).or_insert(0) += 1;
                    }
                }
            }
            for what in ["attack", "Drive Rush"] {
                assert!(era_answers.get(what).is_some_and(|&n| n >= 1), "{level:?}: the {what} answered with an era command: {era_answers:?}");
            }
        }
    }

    /// `fighter` in the SFZ3 era style (`loadout`) with two stocks of Super
    /// meter, its own fighter's control as given, playing the stronger game,
    /// with its kit and tools as the stronger play reads them.
    fn sfz_ctx_for(fighter: TournamentFighter, level: Level, loadout: crate::era::Loadout, control: EraControl) -> Context {
        use crate::era::{GaugeMode, Revision, StylePick, STOCK_AT};
        let config = crate::config::MatchConfig::definitive();
        let mut state = EraFighterState::default();
        state.fighter = Some(fighter);
        state.style = StylePick::for_revision(Revision::Sfz3, loadout);
        state.max_sub_hp = crate::era_fight::FULL_SUB_HP;
        state.set_gauge_mode(GaugeMode::EraDefault);
        state.super_meter = 2 * STOCK_AT;
        state.set_what_if(!crate::era::families(fighter).contains(&crate::era::EraFamily::SfZ));
        let mut c = Context::with_era(level, fighter, Some(RYU_SLOTS), config, &state, control);
        c.set_opponent(crate::era::EraFamily::SfZ, crate::era_fight::FULL_SUB_HP);
        c.stronger = true;
        c.for_stronger()
    }

    fn sfz_ctx(level: Level, loadout: crate::era::Loadout, control: EraControl) -> Context {
        sfz_ctx_for(TournamentFighter::Ryu, level, loadout, control)
    }

    fn sf3_ctx(level: Level) -> Context {
        use crate::era::{GaugeMode, Loadout, Revision, StylePick};
        let config = crate::config::MatchConfig::definitive();
        let mut state = EraFighterState::default();
        state.fighter = Some(TournamentFighter::Ryu);
        state.style = StylePick::for_revision(Revision::Sf3ThreeS, Loadout::Art2);
        state.max_sub_hp = crate::era_fight::FULL_SUB_HP;
        state.set_gauge_mode(GaugeMode::EraDefault);
        state.super_meter = 40;
        let mut ctx = Context::with_era(level, TournamentFighter::Ryu, Some(RYU_SLOTS), config, &state, EraControl { free: true, ..EraControl::default() });
        ctx.set_opponent(crate::era::EraFamily::Sf3, crate::era_fight::FULL_SUB_HP);
        ctx.stronger = true;
        ctx.for_stronger()
    }

    #[test]
    fn sf3_high_and_low_parries_wait_for_the_focused_or_unfocused_delay() {
        // SC17: twins with the same past see either rest or a new normal.
        // No pad can differ before the event's applicable delay; the
        // eventual differences must include real high and low parry taps.
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            let ctx = sf3_ctx(level);
            for (low, direction) in [(false, RIGHT), (true, DOWN)] {
                let mut attack = body(340);
                attack.move_id = 0x10;
                attack.state = if low { 0xc6 } else { 0xc2 };
                let mut parries = 0;
                for seed in 1..=96 {
                    let mut a = FairCpu::seeded(seed);
                    for f in 1..=40 {
                        a.observe(percept(f, body(340)));
                    }
                    for f in 41..=80 {
                        a.press(f, true, &ctx);
                        a.observe(percept(f, body(340)));
                    }
                    let mut b = a;
                    let shown = 81;
                    let mut earliest = shown + level.unfocused_reaction();
                    let mut first_diff = None;
                    let mut parried = false;
                    for f in shown..shown + 120 {
                        if f >= shown + level.reaction() && f < earliest && b.attention().focus == Focus::Attack {
                            earliest = f;
                        }
                        let (pa, pb) = (a.press(f, true, &ctx), b.press(f, true, &ctx));
                        if pa != pb {
                            first_diff.get_or_insert(f);
                        }
                        parried |= b.plan.answer && b.plan.pads[..usize::from(b.plan.len)] == [0, direction, 0, 0];
                        a.observe(percept(f, body(340)));
                        b.observe(percept(f, attack));
                    }
                    if let Some(f) = first_diff {
                        assert!(f >= earliest, "{level:?}, low={low}, seed={seed}: answered {f}, earliest {earliest}");
                        parries += usize::from(parried);
                    }
                }
                assert!(parries > 0, "{level:?}, low={low}: no parry under its reaction delay");
            }
        }
    }

    #[test]
    fn sf3_plan_selection_requires_the_era_tool_and_pure_footsies_suppresses_it() {
        let ctx = sf3_ctx(Level::VeryHard);
        let ex = game::Act::Era(game::EraAct::Enhanced);
        assert!(plans_taken(&ctx, 270, 300, Situation::NeutralClose, 96).contains(&ex), "affordable EX gets its own step");
        let overhead = game::Act::Era(game::EraAct::Overhead);
        assert!(plans_taken(&ctx, 270, 300, Situation::ItWasBlocked, 96).contains(&overhead), "an overhead after a guarded attack");
        let mut footsies = ctx;
        footsies.pure_footsies = true;
        let footsies = footsies.for_stronger();
        assert!(!era_ready(game::EraAct::Enhanced, &footsies) && !era_ready(game::EraAct::Overhead, &footsies));
        assert!(!plans_taken(&footsies, 270, 300, Situation::NeutralClose, 96).contains(&ex));
        assert!(!plans_taken(&footsies, 270, 300, Situation::ItWasBlocked, 96).contains(&overhead));
    }

    fn sf4_ctx(level: Level, control: EraControl) -> Context {
        styled_ctx(level, control, crate::era::Revision::Usf4, crate::era::Loadout::Ultra1)
    }

    fn sf5_ctx(level: Level, control: EraControl) -> Context {
        styled_ctx(level, control, crate::era::Revision::Sf5Ce, crate::era::Loadout::Skill1Trigger1)
    }

    fn styled_ctx(level: Level, control: EraControl, revision: crate::era::Revision, loadout: crate::era::Loadout) -> Context {
        use crate::era::{GaugeMode, StylePick};
        let config = crate::config::MatchConfig::definitive();
        let mut state = EraFighterState::default();
        state.fighter = Some(TournamentFighter::Ryu);
        state.style = StylePick::for_revision(revision, loadout);
        state.set_gauge_mode(GaugeMode::EraDefault);
        state.super_meter = 96;
        let mut ctx = Context::with_era(level, TournamentFighter::Ryu, Some(RYU_SLOTS), config, &state, control);
        ctx.stronger = true;
        ctx.for_stronger()
    }

    #[test]
    fn sf4_and_sf5_a_stalled_reach_goes_on_to_its_poke_without_waiting_ninety_frames() {
        let free = EraControl { free: true, ..EraControl::default() };
        for ctx in [sf4_ctx(Level::VeryHard, free), sf5_ctx(Level::VeryHard, free)] {
            let mut just_guarded = FairCpu::seeded(1);
            for f in 1..100 {
                let mut v = percept(f, body(340));
                v.me.move_id = 4;
                assert!(v.me.guarding());
                just_guarded.observe(v);
            }
            just_guarded.plan = Plan::of(PlanKind::Walk, &[RIGHT; 8]);
            just_guarded.plan.at = just_guarded.plan.len;
            assert!(!just_guarded.walk_stalled(100, 80, &ctx), "past guard frames are not walking, even after the current blockstun ends");
            let plan = game::plans(ctx.kit.fighter, ctx.family).iter().position(|plan| {
                plan.situation == Situation::NeutralFar && matches!(plan.steps[0].act, game::Act::Reach { .. })
            }).unwrap();
            // Eagle's observed native walk rocks eight pixels out and back;
            // Retsu's makes small intermittent steps. Only the latter closes.
            for facing_left in [false, true] {
                for movement in ["still", "rocking", "slow forward", "forward", "backward"] {
                    let stalled = matches!(movement, "still" | "rocking" | "backward");
                    let toward = if facing_left { -1 } else { 1 };
                    let foe = body(if facing_left { 240 } else { 360 });
                    let mut cpu = FairCpu::seeded(1);
                    for f in 1..100 {
                        let mut v = percept(f, foe);
                        v.me.move_id = 2;
                        v.me.state = 0x92;
                        v.me.facing_left = facing_left;
                        if f >= 84 {
                            let age = (f - 84) as i16;
                            let advance = match movement {
                                "rocking" => [0, 8, 8, 8, 8, 0, 0, 0][age as usize % 8],
                                "slow forward" => 2 * (age / 8),
                                "forward" => 2 * age,
                                "backward" => -2 * age,
                                _ => 0,
                            };
                            v.me.x += toward * advance;
                        }
                        cpu.observe(v);
                    }
                    cpu.plan = Plan::of(PlanKind::Walk, &[if facing_left { LEFT } else { RIGHT }; 8]);
                    cpu.plan.at = cpu.plan.len;
                    cpu.reach = [[8; 6]; 2];
                    cpu.judged.situation = Situation::NeutralFar;
                    cpu.game = Some(game::Running { plan: plan as u8, step: 0, acted: true, pressed: 84, began: 80, hit: false });
                    assert_eq!(cpu.walk_stalled(100, 84, &ctx), stalled, "{movement}, facing_left={facing_left}");
                    assert!(!cpu.walk_stalled(100, 85, &ctx), "a partial walking cycle is kept");
                    let mut locked = ctx;
                    locked.own_blockstun = true;
                    assert!(!cpu.walk_stalled(100, 84, &locked), "locked guard is not a failed movement decision");
                    let mut old = ctx;
                    old.stronger = false;
                    assert!(!cpu.walk_stalled(100, 84, &old));
                    assert!(!cpu.walk_stalled(100, 84, &sf3_ctx(Level::VeryHard)), "previous rollouts keep their behavior");
                    let next = cpu.follow_game_plan(100, percept(100, foe), &ctx).unwrap();
                    assert_eq!(next.kind == PlanKind::Walk, !stalled, "{movement}, facing_left={facing_left}");
                    if stalled {
                        assert!(next.pads[..usize::from(next.len)].iter().any(|pad| pad & BUTTONS != 0), "the stalled reach actually goes on to press its normal");
                    }
                }
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Sc21GuardScene {
        QuietNear,
        QuietFar,
        MovingNear,
        Startup,
        Active,
        Projectile,
        ClearingAttack,
    }

    fn sc21_guard_view(frame: u32, scene: Sc21GuardScene, low: bool) -> Percept {
        let foe_x = match scene {
            Sc21GuardScene::QuietFar | Sc21GuardScene::ClearingAttack => 420,
            Sc21GuardScene::MovingNear => 340 + (frame % 2) as i16,
            _ => 340,
        };
        let mut view = percept(frame, body(foe_x));
        // These are native, unlocked guard poses. Actual blockstun is the
        // independent Context flag, not an invented pose-state restriction.
        view.me.move_id = if low { 8 } else { 4 };
        view.me.state = if low { 0x24 } else { 0x22 };
        view.foe.facing_left = true;
        if matches!(scene, Sc21GuardScene::Startup | Sc21GuardScene::Active)
            || (matches!(scene, Sc21GuardScene::ClearingAttack) && frame <= 45)
        {
            view.foe.move_id = 0x10;
            view.foe.state = 0xc2;
            view.foe.front = 300;
            view.foe.active = !matches!(scene, Sc21GuardScene::Startup);
        }
        if matches!(scene, Sc21GuardScene::Projectile) {
            view.shot = true;
            view.shot_x = 330;
            view.shot_half = 8;
        }
        view
    }

    fn sc21_guard_cpu(scene: Sc21GuardScene, low: bool, forced: bool) -> FairCpu {
        let mut cpu = FairCpu::seeded(17).force_stronger(forced);
        for frame in 1..=40 {
            cpu.observe(sc21_guard_view(frame, scene, low));
        }
        // An ordinary anticipated guard, with its original minimum pending.
        cpu.plan = Plan::guard(low, 60);
        cpu
    }

    fn sc21_guard_tick(cpu: &mut FairCpu, step: u32, scene: Sc21GuardScene, low: bool, context: &Context) -> u16 {
        cpu.observe(sc21_guard_view(step, scene, low));
        let view = cpu.seen(step - context.level.reaction()).expect("delayed guard observation");
        let before = cpu.seen(view.frame - 1);
        let reach = cpu.longest_reach();
        cpu.judged = cpu.judge.judge(view, before, reach);
        cpu.next_pad(step, view, context)
    }

    fn sc21_guard_context(revision: crate::era::Revision, configured: bool, throws: bool, blockstun: bool) -> Context {
        let mut context = sc21_approach_context(revision, configured);
        context.throws = throws;
        context.with_baseline(
            crate::era::StylePick::for_revision(revision, crate::era::Loadout::Standard),
            blockstun,
        ).for_stronger()
    }

    #[test]
    fn sc21_baseline_guard_releases_a_clear_far_opponent_after_its_initial_hold() {
        use crate::era::Revision::{Arcade1987, Sf1Plus};
        for revision in [Arcade1987, Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                for low in [false, true] {
                    let context = sc21_guard_context(revision, configured, true, false);
                    let mut cpu = sc21_guard_cpu(Sc21GuardScene::QuietFar, low, forced);
                    for step in 41..=48 {
                        let pad = sc21_guard_tick(&mut cpu, step, Sc21GuardScene::QuietFar, low, &context);
                        if step < 48 {
                            assert_eq!(pad, LEFT | if low { DOWN } else { 0 });
                            assert!(matches!(cpu.plan.kind, PlanKind::Guard { .. }), "{revision:?}: keep the initial eight-frame hold");
                        } else {
                            assert_eq!(pad, 0, "{revision:?} configured{configured} forced{forced}: a clear guard releases its stick");
                            assert!(cpu.plan.done(), "the still-pending minimum does not add quiet guard dwell");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_guard_leaves_a_settled_close_opponent_without_waiving_the_throw_window() {
        use crate::era::Revision::{Arcade1987, Sf1Plus};
        for revision in [Arcade1987, Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                for throws in [false, true] {
                    let context = sc21_guard_context(revision, configured, throws, false);
                    assert_eq!(context.throws, throws && revision == Sf1Plus);
                    let mut cpu = sc21_guard_cpu(Sc21GuardScene::QuietNear, true, forced);
                    let mut ended = None;
                    for step in 41..=55 {
                        let pad = sc21_guard_tick(&mut cpu, step, Sc21GuardScene::QuietNear, true, &context);
                        if cpu.plan.done() {
                            assert_eq!(pad, 0);
                            ended = Some(step - 40);
                            break;
                        }
                        assert!(pad == LEFT | DOWN || (context.throws && pad == LEFT | button(2)),
                            "keep the low guard or its permitted delayed throw-tech pulse: {pad:x}");
                        assert!(matches!(cpu.plan.kind, PlanKind::Guard { .. }));
                    }
                    assert!(ended.is_some_and(|held| (8..=12).contains(&held)), "{revision:?} configured{configured} forced{forced} throws{}: keep the throw window, then leave a quiet guard: {ended:?}", context.throws);
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_guard_waits_for_three_genuinely_observed_clear_frames() {
        use crate::era::Revision::{Arcade1987, Sf1Plus};
        for revision in [Arcade1987, Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                let context = sc21_guard_context(revision, configured, true, false);
                let mut cpu = sc21_guard_cpu(Sc21GuardScene::ClearingAttack, false, forced);
                for step in 41..=60 {
                    let pad = sc21_guard_tick(&mut cpu, step, Sc21GuardScene::ClearingAttack, false, &context);
                    if step < 60 {
                        assert_eq!(pad & LEFT, LEFT, "{revision:?} step{step}: live frame46 clearing is not visible until delayed frame46; two clear views still keep guard");
                        assert!(!cpu.plan.done());
                    } else {
                        assert_eq!(pad, 0, "the third observed clear frame releases the guard");
                        assert!(cpu.plan.done(), "{revision:?} configured{configured} forced{forced}: do not wait for the old sixty-frame minimum");
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_guard_keeps_live_threats_actual_blockstun_and_a_held_bait() {
        use crate::era::Revision::{Arcade1987, Sf1Plus};
        for revision in [Arcade1987, Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                for (scene, locked, bait) in [
                    (Sc21GuardScene::Startup, false, false),
                    (Sc21GuardScene::Active, false, false),
                    (Sc21GuardScene::Projectile, false, false),
                    (Sc21GuardScene::QuietFar, true, false),
                    (Sc21GuardScene::QuietFar, false, true),
                ] {
                    let context = sc21_guard_context(revision, configured, true, locked);
                    let mut cpu = sc21_guard_cpu(scene, true, forced);
                    if bait { cpu.bait_until = 100; }
                    for step in 41..=64 {
                        let pad = sc21_guard_tick(&mut cpu, step, scene, true, &context);
                        assert!(matches!(cpu.plan.kind, PlanKind::Guard { .. }), "{revision:?} {scene:?} blockstun{locked} bait{bait}: keep the existing guard");
                        assert_eq!(pad & LEFT, LEFT);
                        assert_eq!(pad & BUTTONS, 0, "a real threat or a held bait receives no throw pulse");
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_close_guard_respects_enabled_throws_until_the_foe_settles() {
        use crate::era::Revision::{Arcade1987, Sf1Plus};
        for revision in [Arcade1987, Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                for throws in [false, true] {
                    let context = sc21_guard_context(revision, configured, throws, false);
                    let mut cpu = sc21_guard_cpu(Sc21GuardScene::MovingNear, false, forced);
                    for step in 41..=48 {
                        let pad = sc21_guard_tick(&mut cpu, step, Sc21GuardScene::MovingNear, false, &context);
                        if step < 48 || context.throws {
                            assert_eq!(pad & LEFT, LEFT);
                            assert!(matches!(cpu.plan.kind, PlanKind::Guard { .. }));
                        } else {
                            assert_eq!(pad, 0, "{revision:?}: disabled throws do not invent a close throw threat");
                            assert!(cpu.plan.done());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_guard_release_keeps_off_and_previous_later_cpu_state() {
        use crate::canonical::{Canonical, CanonicalHasher};
        use crate::era::Revision::{Arcade1987, Sf1Plus};
        let hash = |cpu: &FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        let compare = |tagged: Context, legacy: Context| {
            for scene in [Sc21GuardScene::QuietNear, Sc21GuardScene::QuietFar, Sc21GuardScene::ClearingAttack] {
                let mut tagged_cpu = sc21_guard_cpu(scene, true, false);
                let mut legacy_cpu = tagged_cpu;
                for step in 41..=64 {
                    assert_eq!(sc21_guard_tick(&mut tagged_cpu, step, scene, true, &tagged), sc21_guard_tick(&mut legacy_cpu, step, scene, true, &legacy), "{scene:?}: existing pad sequence stays exact");
                    assert_eq!(hash(&tagged_cpu), hash(&legacy_cpu), "{scene:?}: whole canonical CPU state stays exact");
                    assert!(matches!(tagged_cpu.plan.kind, PlanKind::Guard { .. }), "the existing minimum still holds this guard");
                }
            }
        };
        for revision in [Arcade1987, Sf1Plus] {
            for throws in [false, true] {
                let tagged = sc21_guard_context(revision, false, throws, false);
                let legacy = Context { family: None, style: None, baseline: None, ..tagged };
                compare(tagged, legacy);
            }
        }
        for tagged in [sf3_ctx(Level::VeryHard), sf6_ctx(Level::VeryHard)] {
            let legacy = Context { family: None, style: None, ..tagged };
            compare(tagged, legacy);
        }
    }


    #[test]
    fn sf4_and_sf5_release_a_quiet_close_guard_but_keep_a_threat_locked_guard_and_bait() {
        let free = EraControl { free: true, ..EraControl::default() };
        for ctx in [sf4_ctx(Level::VeryHard, free), sf5_ctx(Level::VeryHard, free)] {
            assert!(ctx.throws);
            for what in ["quiet", "far clear", "recent threat", "attack", "blockstun", "bait", "old", "sf3"] {
                let mut ctx = if what == "sf3" { sf3_ctx(Level::VeryHard) } else { ctx };
                if what == "old" { ctx.stronger = false; }
                if what == "blockstun" { ctx.own_blockstun = true; }
                let mut cpu = FairCpu::seeded(1);
                for f in 1..41 { cpu.observe(percept(f, body(340))); }
                // The release must bypass a still-pending minimum once
                // its initial hold and the observed threat-clear window pass.
                cpu.plan = Plan::guard(true, 60);
                if what == "bait" { cpu.bait_until = 100; }
                let mut ended = None;
                for f in 41..65 {
                    let mut foe = body(if matches!(what, "far clear" | "recent threat") { 420 } else { 340 });
                    let attacking = what == "attack" || (what == "recent threat" && f < 48);
                    if attacking {
                        foe.move_id = 0x10;
                        foe.state = 0xc2;
                        foe.active = true;
                        foe.front = 300;
                    }
                    let v = percept(f, foe);
                    if attacking { assert!(v.threat().is_some()); }
                    cpu.next_pad(f, v, &ctx);
                    cpu.observe(v);
                    if !matches!(cpu.plan.kind, PlanKind::Guard { .. }) { ended = Some(f - 40); break; }
                }
                if what == "quiet" {
                    assert!(ended.is_some_and(|held| (8..=15).contains(&held)), "keep the initial throw-tech window, then act: {ended:?}");
                } else if what == "far clear" {
                    assert_eq!(ended, Some(8), "a clear threat does not waive the initial hold");
                } else if what == "recent threat" {
                    assert_eq!(ended, Some(10), "three genuinely clear frames are needed after the last threat");
                } else {
                    assert_eq!(ended, None, "{what}: preserve its guard");
                }
            }
        }
    }

    #[test]
    fn sf4_and_sf5_remember_an_early_route_hit_when_its_late_continuation_whiffs() {
        let free = EraControl { free: true, ..EraControl::default() };
        for (era, ctx) in [("sf4", sf4_ctx(Level::VeryHard, free)), ("sf5", sf5_ctx(Level::VeryHard, free)), ("sf3", sf3_ctx(Level::VeryHard))] {
            let (revision, loadout) = ctx.style.unwrap();
            let book = crate::route_book::book(TournamentFighter::Ryu, revision, loadout).unwrap();
            let (index, route) = book.routes.iter().enumerate().find(|(_, route)| route.steps.len() >= 2).unwrap();
            for early_hit in [false, true] {
                let mut cpu = FairCpu::seeded(1);
                cpu.route = Some(game::RouteRun::new(TournamentFighter::Ryu, revision, loadout, index as u8, 0, route.len() as u16, true));
                // Replay the real book script. Its opener is seen to hit,
                // but the opponent is free before the last inputs finish.
                for frame in 1..=route.len() as u32 + 1 {
                    let mut foe = body(340);
                    if early_hit && frame == 2 { foe.move_id = 0x30; }
                    cpu.next_route_pad(frame, percept(frame, foe), &ctx);
                }
                assert!(cpu.route.is_none(), "the complete route was entered");
                assert_eq!(cpu.route_outcome, Some(if early_hit { game::Outcome::Hit } else { game::Outcome::Whiff }));
                let watch = cpu.route_check.expect("a multistep route watches its ending");
                // Keep a neutral action in progress so nothing else is
                // chosen while the normal decision path observes the end.
                cpu.plan = Plan::of(PlanKind::Act, &[0; PLAN_CAP]);
                for frame in watch.at - game::ROUTE_WATCH + 1..=watch.at {
                    cpu.decide(frame, percept(frame, body(340)), None, &ctx);
                    if frame < watch.at { assert_eq!(cpu.route_drops[index], 0, "do not judge the continuation before its watch expires"); }
                }
                assert!(cpu.route_check.is_none(), "the ending was observed");
                assert_eq!(cpu.route_drops[index], u8::from(era != "sf3" && early_hit), "{era}, early_hit={early_hit}: learn an early hit followed by a dropped continuation in the SF4/SF5 rollouts");
            }
        }
    }

    #[test]
    fn sf4_focus_and_paid_fadc_wait_for_the_applicable_reaction_delay() {
        use crate::era::{GaugeMode, Loadout, Revision, StylePick};
        let pair = button(1) | button(4);
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            for cancelling in [false, true] {
                let config = crate::config::MatchConfig::definitive();
                let mut state = EraFighterState::default();
                state.fighter = Some(TournamentFighter::Ryu);
                state.style = StylePick::for_revision(Revision::Usf4, Loadout::Ultra1);
                state.set_gauge_mode(GaugeMode::EraDefault);
                state.super_meter = 96;
                let control = EraControl { free: !cancelling, special: cancelling, ..EraControl::default() };
                let mut ctx = Context::with_era(level, TournamentFighter::Ryu, Some(RYU_SLOTS), config, &state, control);
                ctx.stronger = true;
                let ctx = ctx.for_stronger();
                let observation = |frame, foe| {
                    let mut v = percept(frame, foe);
                    if cancelling {
                        v.me.move_id = 0x19;
                        v.me.state = 0xc2;
                    }
                    v
                };
                let mut attack = body(340);
                attack.move_id = 0x10;
                attack.state = 0xc2;
                let mut answers = 0;
                for seed in 1..=96 {
                    let mut a = FairCpu::seeded(seed);
                    for f in 1..=40 {
                        a.observe(observation(f, body(340)));
                    }
                    for f in 41..=80 {
                        a.press(f, true, &ctx);
                        a.observe(observation(f, body(340)));
                    }
                    let mut b = a;
                    let shown = 81;
                    let mut earliest = shown + level.unfocused_reaction();
                    let mut answered = false;
                    for f in shown..shown + 120 {
                        if f >= shown + level.reaction() && f < earliest && b.attention().focus == Focus::Attack {
                            earliest = f;
                        }
                        let (pa, pb) = (a.press(f, true, &ctx), b.press(f, true, &ctx));
                        assert!(pa == pb || f >= earliest, "{level:?}, cancel={cancelling}, seed={seed}: answered {f}, earliest {earliest}");
                        let pads = &b.plan.pads[..usize::from(b.plan.len)];
                        let focus = pads.iter().any(|p| p & BUTTONS == pair);
                        let back_dash = pads.windows(3).any(|p| p == [pair | LEFT, pair, pair | LEFT]);
                        answered |= b.plan.answer && focus && (!cancelling || back_dash);
                        a.observe(observation(f, body(340)));
                        b.observe(observation(f, attack));
                    }
                    answers += usize::from(answered);
                }
                if cancelling && !level.spends_meter() {
                    assert_eq!(answers, 0, "Easy keeps its meter");
                } else {
                    assert!(answers > 0, "{level:?}, cancel={cancelling}: no actual Focus/FADC answer");
                }
            }
        }
    }

    #[test]
    fn sf4_finisher_plans_punish_seen_recovery_but_stop_for_a_live_attack() {
        use crate::era::{GaugeMode, Loadout, Revision, StylePick};
        let config = crate::config::MatchConfig::definitive();
        let mut state = EraFighterState::default();
        state.fighter = Some(TournamentFighter::Ryu);
        state.style = StylePick::for_revision(Revision::Usf4, Loadout::Ultra1);
        state.set_gauge_mode(GaugeMode::EraDefault);
        state.super_meter = 192;
        state.set_sf4_practice_revenge(Some(640));
        let mut ctx = Context::with_era(Level::VeryHard, TournamentFighter::Ryu, Some(RYU_SLOTS), config, &state, EraControl { free: true, ..EraControl::default() });
        ctx.stronger = true;
        let ctx = ctx.for_stronger();
        for act in [game::EraAct::Super, game::EraAct::Ultra] {
            let index = game::plans(ctx.kit.fighter, ctx.family).iter().position(|plan| {
                plan.situation == Situation::OpponentWhiffed && plan.steps[0].act == game::Act::Era(act)
            }).expect("a dedicated finisher punish plan");
            for (situation, active, allowed) in [(Situation::OpponentWhiffed, false, true), (Situation::OpponentWhiffed, true, false), (Situation::Defending, false, false)] {
                let mut cpu = FairCpu::seeded(1);
                for f in 1..100 {
                    cpu.observe(percept(f, body(340)));
                }
                cpu.judged.situation = situation;
                cpu.game = Some(game::Running { plan: index as u8, step: 0, acted: false, pressed: 0, began: 100, hit: false });
                let mut foe = body(340);
                foe.move_id = 0x10;
                foe.state = 0xc2;
                foe.active = active;
                let entered = cpu.follow_game_plan(100, percept(100, foe), &ctx);
                let motion = entered.is_some_and(|plan| {
                    let pads = &plan.pads[..usize::from(plan.len)];
                    pads.iter().any(|p| p & DOWN != 0) && pads.iter().any(|p| p & BUTTONS != 0)
                });
                assert_eq!(motion, allowed, "{act:?}, {situation:?}, active={active}: a real finisher motion only for the recovery");
            }
        }
    }

    #[test]
    fn with_the_stronger_play_no_sfz_tool_answers_anything_sooner_than_the_applicable_delay() {
        // SC16 (`#208`): an Alpha Counter out of a guard, and a guard held in
        // the air, answer an attack the CPU sees. Two stronger CPUs with the
        // same past; one is then shown an attack in its reach. Their pads
        // stay the same until the frame is the applicable delay old (the
        // focused one if it watches for an attack as it decides, else the
        // slower one). Over the tries the answers include the tool itself:
        // the counter's motion (back, down-back, down, then the button with
        // the stick down) and the air guard's twelve frames held back.
        let mut attack = body(340);
        attack.move_id = 0x10;
        attack.state = 0xc2;
        // Its fighter at 300, the opponent at 340: back is left.
        let away = LEFT;
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            let guarding = sfz_ctx(level, crate::era::Loadout::AIsm, EraControl { blockstun: true, ..EraControl::default() });
            let airborne = sfz_ctx(level, crate::era::Loadout::AIsm, EraControl { airborne: true, ..EraControl::default() });
            let mut answers = std::collections::HashMap::new();
            for (c, what) in [(guarding, "Alpha Counter"), (airborne, "air guard")] {
                for seed in 1..=24u64 {
                    let mut a = FairCpu::seeded(seed);
                    for f in 1..=40 {
                        a.observe(percept(f, body(340)));
                    }
                    for f in 41..=80 {
                        a.press(f, true, &c);
                        a.observe(percept(f, body(340)));
                    }
                    let mut b = a;
                    let shown = 81;
                    let mut earliest = shown + level.unfocused_reaction();
                    let mut first_diff = None;
                    let mut era_answer = false;
                    let mut after = Vec::new();
                    for f in shown..shown + 120 {
                        if f >= shown + level.reaction() && f < earliest && b.attention().focus == Focus::Attack {
                            earliest = f;
                        }
                        let (pa, pb) = (a.press(f, true, &c), b.press(f, true, &c));
                        if pa != pb && first_diff.is_none() {
                            first_diff = Some(f);
                            // The era kit's answer (`Tactics::react`): marked as one.
                            era_answer = b.plan.answer;
                        }
                        if first_diff.is_some() {
                            after.push(pb);
                        }
                        a.observe(percept(f, body(340)));
                        b.observe(percept(f, attack));
                    }
                    // The level's guard chance may leave one unanswered.
                    let Some(f) = first_diff else { continue };
                    assert!(f >= earliest, "{level:?} seed {seed}: the {what} came on {f}, shown on {shown}, earliest {earliest}");
                    let tool = if what == "Alpha Counter" {
                        let back = after.iter().position(|&pad| pad & (away | DOWN) == away);
                        let down_back = after.iter().position(|&pad| pad & (away | DOWN) == away | DOWN);
                        let pressed = after.iter().position(|&pad| pad & BUTTONS != 0 && pad & DOWN != 0);
                        matches!((back, down_back, pressed), (Some(x), Some(y), Some(z)) if x < y && y < z)
                    } else {
                        after.len() >= 12 && after[..12].iter().all(|&pad| pad == away)
                    };
                    if tool && era_answer {
                        *answers.entry(what).or_insert(0) += 1;
                    }
                }
            }
            for what in ["Alpha Counter", "air guard"] {
                assert!(answers.get(what).is_some_and(|&n| n >= 1), "{level:?}: the attack answered with an {what}: {answers:?}");
            }
        }
    }

    #[test]
    fn a_sfz_tool_is_used_only_where_it_can_be() {
        // SC16 (`#208`): a charge only with room behind to hold it walking
        // back; a teleport only by a fighter who has one; Custom Combo only
        // in the V-ism with the meter; a change of stance only from Gen's
        // Mantis; none under a mission allowing only normals and throws.
        use crate::era::Loadout::{AIsm, VIsm};
        use TournamentFighter::{Birdie, Gen, Geki};
        let free = EraControl { free: true, ..EraControl::default() };
        let cpu = FairCpu::new();
        let at = |me: i16, foe: i16| Percept { frame: 1, fighting: true, me: body(me), foe: body(foe), ..Percept::default() };
        let birdie = sfz_ctx_for(Birdie, Level::VeryHard, AIsm, free);
        assert!(birdie.tools.get(cast::Tool::CommandThrow).is_some() && birdie.tools.get(cast::Tool::Charge).is_some(), "Birdie's chain and Bull Head");
        assert!(cpu.tool_ready(cast::Tool::Charge, &at(260, 360), &birdie), "mid-screen");
        assert!(!cpu.tool_ready(cast::Tool::Charge, &at(150, 250), &birdie), "his back to the left wall");
        assert!(!cpu.tool_ready(cast::Tool::Charge, &at(370, 270), &birdie), "his back to the right wall");
        assert!(cpu.tool_ready(cast::Tool::Charge, &at(150, 50), &birdie), "the wall in front of him is no matter");
        assert!(!cpu.tool_ready(cast::Tool::Teleport, &at(200, 360), &birdie), "Birdie has no teleport");
        let geki = sfz_ctx_for(Geki, Level::VeryHard, AIsm, free);
        assert!(cpu.tool_ready(cast::Tool::Teleport, &at(200, 360), &geki), "Geki's Utsusemi");
        let ryu = sfz_ctx(Level::VeryHard, AIsm, free);
        assert!(!cpu.tool_ready(cast::Tool::CommandThrow, &at(250, 260), &ryu), "Ryu has no grab");
        assert!(era_ready(game::EraAct::CustomCombo, &sfz_ctx(Level::VeryHard, VIsm, free)), "V-ism with meter");
        assert!(!era_ready(game::EraAct::CustomCombo, &ryu), "the A-ism has no Custom Combo");
        assert!(era_ready(game::EraAct::Stance, &sfz_ctx_for(Gen, Level::VeryHard, AIsm, free)), "Gen in the Mantis");
        assert!(!era_ready(game::EraAct::Stance, &ryu), "Ryu has no stance");
        let mut footsies = sfz_ctx_for(Birdie, Level::VeryHard, VIsm, free);
        footsies.pure_footsies = true;
        let footsies = footsies.for_stronger();
        assert!(!cpu.tool_ready(cast::Tool::CommandThrow, &at(250, 260), &footsies) && !era_ready(game::EraAct::CustomCombo, &footsies));
    }

    /// The first step of the game plan a stronger CPU with this past takes
    /// up in `situation`, its fighter at `me` and the opponent at `foe`, if
    /// it takes one, over `seeds`.
    fn plans_taken(ctx: &Context, me: i16, foe: i16, situation: Situation, seeds: u64) -> Vec<game::Act> {
        let table = game::plans(ctx.kit.fighter, ctx.family);
        let view = |f: u32| Percept { frame: f, fighting: true, me: body(me), foe: body(foe), ..Percept::default() };
        let mut acts = Vec::new();
        for seed in 1..=seeds {
            let mut cpu = FairCpu::seeded(seed);
            for f in 1..=40 {
                cpu.observe(view(f));
            }
            cpu.judged.situation = situation;
            if cpu.start_game_plan(40, view(40), ctx) {
                acts.push(table[usize::from(cpu.game.unwrap().plan)].steps[0].act);
            }
        }
        acts
    }

    #[test]
    fn a_plan_needing_a_tool_it_cannot_use_here_is_not_taken_up() {
        // SC16 (`#208`): plan selection itself asks whether the tool can be
        // used: Birdie's charge plan from mid-screen, never with his back to
        // the wall (his jump-in instead); Custom Combo in the V-ism, never
        // in the A-ism.
        use crate::era::Loadout::{AIsm, VIsm};
        let free = EraControl { free: true, ..EraControl::default() };
        let birdie = sfz_ctx_for(TournamentFighter::Birdie, Level::VeryHard, AIsm, free);
        let charge = game::Act::Tool(cast::Tool::Charge);
        let mid = plans_taken(&birdie, 260, 360, Situation::NeutralFar, 60);
        let wall = plans_taken(&birdie, 150, 250, Situation::NeutralFar, 60);
        assert!(mid.contains(&charge), "mid-screen: {mid:?}");
        assert!(!wall.contains(&charge) && !wall.is_empty(), "at the wall: {wall:?}");
        let custom = game::Act::Era(game::EraAct::CustomCombo);
        let v = plans_taken(&sfz_ctx(Level::VeryHard, VIsm, free), 270, 300, Situation::NeutralClose, 60);
        let a = plans_taken(&sfz_ctx(Level::VeryHard, AIsm, free), 270, 300, Situation::NeutralClose, 60);
        assert!(v.contains(&custom), "V-ism: {v:?}");
        assert!(!a.contains(&custom) && !a.is_empty(), "A-ism: {a:?}");
    }

    #[test]
    fn a_command_throw_walks_in_step_by_step_and_its_motion_starts_only_in_its_catch_range() {
        // SC16 (`#208`, review): from its catch range and 80 pixels more it
        // walks toward him a few frames at a time; inside the range the
        // grab's motion comes, its button inside one plan. A slipped motion
        // leaves out the last Up (no jump).
        let ctx = sfz_ctx_for(TournamentFighter::Birdie, Level::VeryHard, crate::era::Loadout::AIsm, EraControl { free: true, ..EraControl::default() });
        let table = game::plans(Some(TournamentFighter::Birdie), Some(crate::era::EraFamily::SfZ));
        let grab = table.iter().position(|p| p.situation == Situation::NeutralClose && p.steps[0].act == game::Act::Tool(cast::Tool::CommandThrow)).unwrap() as u8;
        let range = grab_range(&ctx);
        let step_at = |gap: i16, seed: u64| {
            let mut cpu = FairCpu::seeded(seed);
            let view = |f: u32| Percept { frame: f, fighting: true, me: body(200), foe: body(200 + gap), ..Percept::default() };
            for f in 1..=40 {
                cpu.observe(view(f));
            }
            cpu.judged.situation = Situation::NeutralClose;
            cpu.game = Some(game::Running { plan: grab, step: 0, acted: false, pressed: 0, began: 40, hit: false });
            cpu.follow_game_plan(40, view(40), &ctx).expect("a step")
        };
        let far = step_at(range + 80, 1);
        assert_eq!(far.kind, PlanKind::Walk, "from range + 80 it walks");
        assert!(far.pads[..usize::from(far.len)].iter().all(|&pad| pad == RIGHT) && far.len >= 1);
        let mut slipped = false;
        for seed in 1..=60 {
            let near = step_at(range, seed);
            assert_eq!(near.kind, PlanKind::Act, "in range the motion");
            let pads = &near.pads[..usize::from(near.len)];
            if pads.iter().all(|&pad| pad & BUTTONS == 0) {
                slipped = true;
                assert!(pads.iter().all(|&pad| pad & UP == 0), "a slipped grab is no jump: {pads:x?}");
            }
        }
        // Very Difficult slips rarely; the button is there otherwise.
        let near = step_at(range, 2);
        assert!(near.pads[..usize::from(near.len)].iter().any(|&pad| pad & BUTTONS != 0) || slipped);
    }

    #[test]
    fn a_drive_reversal_chosen_out_of_a_guard_is_not_dropped_as_a_walk_in() {
        // SC8 (`#200`): the Drive Reversal's first pad is the bare stick
        // forward. With his attack still coming, the stronger play drops a
        // walk in, but not that: the reversal comes out.
        let c = sf6_ctx_with(Level::VeryHard, EraControl { blockstun: true, ..EraControl::default() });
        let mut me = body(300);
        me.move_id = 4;
        let mut attack = body(340);
        attack.move_id = 0x10;
        attack.state = 0xc2;
        let seen = |f: u32| Percept { frame: f, fighting: true, me, foe: attack, ..Percept::default() };
        let reversal = RIGHT | button(2) | button(5);
        let reversed = (1..=24u64)
            .filter(|&seed| {
                let mut cpu = FairCpu::seeded(seed);
                for f in 1..=40 {
                    cpu.observe(seen(f));
                }
                (41..=100).any(|f| {
                    let pad = cpu.press(f, true, &c);
                    cpu.observe(seen(f));
                    pad & reversal == reversal
                })
            })
            .count();
        assert!(reversed > 0, "a Drive Reversal out of a guard while his attack is still coming");
    }

    #[test]
    fn the_stronger_play_spends_a_super_when_behind_with_its_meter_full_or_in_a_deciding_round() {
        // SC8 (`#200`): otherwise it keeps it (a route that wins the round is
        // taken whatever, `game::Moment`). The old play spends whenever it can.
        let cpu = FairCpu::new();
        let mut c = sf6_ctx(Level::VeryHard);
        let mut view = percept(100, body(340));
        assert!(!cpu.spends_super(&c, view), "even, the meter not full: kept");
        view.me.health = 30;
        assert!(cpu.spends_super(&c, view), "behind: spent");
        view.me.health = 48;
        c.meter_full = true;
        assert!(cpu.spends_super(&c, view), "a full meter: spent");
        c.meter_full = false;
        c.round_decides_match = true;
        assert!(cpu.spends_super(&c, view), "a round that could end the match: spent");
        c.round_decides_match = false;
        c.stronger = false;
        assert!(cpu.spends_super(&c, view), "the old play spends whenever it can");
    }

    #[test]
    fn a_motion_special_given_in_answer_keeps_its_extra_cost() {
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            let mut c = ctx(level);
            c.stronger = true;
            let view = percept(100, body(360));
            let lead = |answer: bool| {
                let mut cpu = FairCpu::seeded(7);
                let plan = cpu.plan_for(Choice::Special(Role::AntiAir), 112, view, &c, answer);
                plan.pads[..usize::from(plan.len)].iter().take_while(|&&pad| pad == 0).count()
            };
            assert_eq!(lead(true), lead(false) + level.special_cost(), "{level:?}");
        }
    }

    #[test]
    fn modern_tactics_are_gated_by_their_switches() {
        let classic = ctx(Level::VeryHard);
        let modern = Context::new(
            Level::VeryHard,
            Kit::of(TournamentFighter::Ryu, RulesetPreset::DefinitiveOverhaul, Some(RYU_SLOTS)),
            &CoreToggles::for_preset(RulesetPreset::DefinitiveOverhaul),
        );
        // Under the classic rules, an opponent walking in at close range never
        // triggers a delayed throw tech while guarding.
        let mut walk_in = body(340);
        walk_in.move_id = 2;
        for seed in 1..100u64 {
            let mut cpu = FairCpu::new();
            cpu.rng = Rng(seed);
            cpu.plan = Plan::guard(true, 20);
            for f in 1..20 {
                let pad = cpu.next_pad(100 + f, percept(88 + f, walk_in), &classic);
                assert_eq!(pad & (button(2) | button(5)), 0, "classic guard never presses a throw button");
            }
        }
        // Under the modern rules, guarding against a close walk-in pulses Back + HP.
        let mut tech_pulses = 0;
        let mut cpu = FairCpu::new();
        cpu.rng = Rng(11);
        cpu.plan = Plan::guard(true, 20);
        for f in 1..20 {
            let pad = cpu.next_pad(100 + f, percept(88 + f, walk_in), &modern);
            if pad == (LEFT | button(2)) {
                tech_pulses += 1;
            }
        }
        assert!(tech_pulses > 0, "modern guard pulses delayed throw tech");
    }

    #[test]
    fn no_attack_is_chosen_into_a_hit_reaction_or_an_attack_still_coming() {
        // The opponent in reach, hit (a standing reaction) or attacking: in
        // neutral the CPU guards, waits or moves, and starts nothing.
        for (move_id, state) in [(0x32, 0x8a), (0x10, 0xc2)] {
            for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                let c = ctx(level);
                let mut foe = body(340);
                (foe.move_id, foe.state) = (move_id, state);
                for seed in 1..200u64 {
                    let mut cpu = FairCpu::new();
                    cpu.rng = Rng(seed);
                    let plan = cpu.choose(100, percept(90, foe), &c);
                    assert_ne!(plan.kind, PlanKind::Act, "{level:?}, move {move_id:#x}, seed {seed}");
                }
                // And in the open, it does attack.
                let attacks = (1..200u64).filter(|&seed| {
                    let mut cpu = FairCpu::new();
                    cpu.rng = Rng(seed);
                    cpu.choose(100, percept(90, body(340)), &c).kind == PlanKind::Act
                });
                assert!(attacks.count() > 50, "{level:?}");
            }
        }
    }

    #[test]
    fn a_guarded_move_is_not_punished_as_a_whiff() {
        // The opponent's attack box goes: after the CPU guarded it (one hit
        // a move; under true combos the guard is locked), it is no whiff.
        let c = ctx(Level::VeryHard);
        let mut attack = body(340);
        (attack.move_id, attack.state, attack.active) = (0x10, 0xc2, true);
        let mut after = attack;
        after.active = false;
        for guarded in [true, false] {
            let mut punished = 0;
            for seed in 1..100u64 {
                let mut cpu = FairCpu::new();
                cpu.rng = Rng(seed);
                let (mut before, mut view) = (percept(89, attack), percept(90, after));
                if guarded {
                    before.me.move_id = 8;
                    view.me.move_id = 8;
                }
                punished += usize::from(cpu.react(100, view, before, &c).is_some_and(|p| p.kind == PlanKind::Act));
            }
            assert_eq!(punished > 0, !guarded, "guarded {guarded}: {punished} punishes");
        }
    }

    #[test]
    fn every_route_book_fits_the_drops_the_cpu_remembers() {
        // Route indices are kept as `u8` and drops per route in a fixed
        // array: a larger book would leave routes untracked.
        for book in crate::route_book::books() {
            assert!(book.routes.len() <= ROUTE_SLOTS, "{:?} {:?}: {} routes", book.fighter, book.revision, book.routes.len());
        }
    }

    #[test]
    fn a_dropped_step_is_what_a_slip_does() {
        let entry = crate::moves::REGISTRY.iter().find(|e| e.name == "SHORYUKEN").unwrap();
        let mut cpu = FairCpu::new();
        cpu.rng = Rng(7);
        let view = percept(10, body(360));
        let mut c = ctx(Level::Easy);
        c.level = Level::Easy;
        let mut clean = 0;
        for _ in 0..200 {
            let frames = cpu.motion(entry, true, view, &c);
            let sticks: Vec<u16> = frames.iter().map(|p| p & 0xf).collect();
            if sticks.starts_with(&[RIGHT, RIGHT, RIGHT, DOWN, DOWN, DOWN]) && frames[7] & 0x70 != 0 {
                clean += 1;
            }
        }
        assert!((100..190).contains(&clean), "{clean} of 200 clean at Easy");
    }

    /// Routes taken from close neutral by `n` fresh CPUs at `level` (seeded
    /// 1 to `n`), against an opponent with `foe_health` health bar points,
    /// each CPU's habit memory filled by `notes` first: the routes taken,
    /// those cut on purpose into a combo reset (SC10, `#202`), and those
    /// cut into a throw.
    fn resets_taken(level: Level, foe_health: u16, notes: &[(Spot, usize)], n: u64) -> (u32, u32, u32) {
        let c = sf6_ctx(level);
        let me = Body { facing_left: false, ..body(200) };
        // His hurtbox 50 pixels from its fighter's centre: in reach of
        // Ryu's SF6 ground routes, which start from 52 to 58.
        let foe = Body { facing_left: true, health: foe_health, ..body(260) };
        let view = Percept { frame: 100, fighting: true, me, foe, ..Percept::default() };
        let (mut taken, mut cut, mut throws) = (0, 0, 0);
        for seed in 1..=n {
            let mut cpu = FairCpu::seeded(seed);
            for &(spot, answer) in notes {
                for _ in 0..6 {
                    cpu.habits.record(spot, answer);
                }
            }
            if !cpu.take_route(&c, view, &[crate::route_book::Reach::Ground], game::NEUTRAL_WALK) {
                continue;
            }
            taken += 1;
            let run = cpu.route.unwrap();
            if let Some(reset) = run.reset {
                cut += 1;
                throws += u32::from(reset.kind == ResetKind::Throw);
                // Cut at one of the route's marked points.
                let book = crate::route_book::book(TournamentFighter::Ryu, crate::era::Revision::Sf6, crate::era::Loadout::Standard).unwrap();
                let route = &book.routes[usize::from(run.index)];
                assert!(route.cuts.iter().any(|c| route.step_start(usize::from(c.step)) == run.route_stop()), "{}", route.name());
            }
        }
        (taken, cut, throws)
    }

    #[test]
    fn a_route_that_would_win_the_round_is_never_cut_on_purpose() {
        // SC10 (`#202`): with a point of health left every route wins the
        // round, and none is cut, even with a memory that asks for resets;
        // with his health full the same CPUs cut many.
        let notes = [(Spot::Grabbed, GrabAnswer::NotTeched as usize), (Spot::AfterGuard, GuardAnswer::KeptGuarding as usize)];
        let (full, nearly_out) = (resets_taken(Level::VeryHard, 48, &notes, 600), resets_taken(Level::VeryHard, 1, &notes, 600));
        assert_eq!(full.0, 600);
        assert_eq!(nearly_out.0, 600);
        assert!(full.1 >= 120, "routes cut with his health full: {full:?}");
        assert_eq!(nearly_out.1, 0, "routes that win the round cut: {nearly_out:?}");
    }

    #[test]
    fn how_often_a_route_is_cut_is_a_level_value_and_the_habit_memory_shifts_it() {
        // SC10 (`#202`): Easy never cuts a route on purpose, and the higher
        // levels cut more. Against one who never techs and keeps guarding
        // the memory cuts more, and more into throws, as far as the level
        // uses it: not at all at Easy.
        let n = 2_000;
        let none: [(Spot, usize); 0] = [];
        let notes = [(Spot::Grabbed, GrabAnswer::NotTeched as usize), (Spot::AfterGuard, GuardAnswer::KeptGuarding as usize)];
        let [easy, normal, hard, very] = [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard].map(|level| resets_taken(level, 48, &none, n));
        assert_eq!(easy.1, 0, "Easy cut {easy:?}");
        assert!(normal.1 > 0 && normal.1 < hard.1 && hard.1 < very.1, "{normal:?} {hard:?} {very:?}");
        // Each level's share near its value (the routes all have cuts).
        for (level, (taken, cut, _)) in [(Level::Normal, normal), (Level::Hard, hard), (Level::VeryHard, very)] {
            let share = 100 * cut / taken;
            let want = level.reset_chance();
            assert!(share + 3 >= want && share <= want + 3, "{level:?}: {share}% cut, its value {want}%");
        }
        let very_noted = resets_taken(Level::VeryHard, 48, &notes, n);
        assert!(very_noted.1 >= very.1 + n as u32 / 20, "cut with the memory {very_noted:?}, without {very:?}");
        // Into throws: about 56% of its resets with the memory, 40% without.
        assert!(100 * very_noted.2 / very_noted.1 >= 100 * very.2 / very.1 + 10, "share into throws with the memory {very_noted:?}, without {very:?}");
        assert_eq!(resets_taken(Level::Easy, 48, &notes, n).1, 0, "Easy uses no memory and never resets");
        // A memory of reversals out of its pressure leans to baits.
        let reversals = [(Spot::AfterGuard, GuardAnswer::Button as usize), (Spot::Rising, RiseAnswer::Reversal as usize)];
        let lean = Habits::default();
        let mut noted = Habits::default();
        for &(spot, answer) in &reversals {
            for _ in 0..6 {
                noted.record(spot, answer);
            }
        }
        assert!(noted.reset_lean(Level::VeryHard).bait > lean.reset_lean(Level::VeryHard).bait);
        assert_eq!(noted.reset_lean(Level::Easy), lean.reset_lean(Level::Easy));
    }

    #[test]
    fn the_canonical_hash_covers_a_planned_reset_a_bait_held_and_a_reset_waiting() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        let mut planned = FairCpu::seeded(3);
        let c = sf6_ctx(Level::VeryHard);
        let me = Body { facing_left: false, ..body(200) };
        let view = Percept { frame: 100, fighting: true, me, foe: Body { facing_left: true, ..body(260) }, ..Percept::default() };
        assert!(planned.take_route(&c, view, &[crate::route_book::Reach::Ground], game::NEUTRAL_WALK));
        let mut other = planned;
        let mut run = planned.route.unwrap();
        run.reset = Some(game::Reset { kind: ResetKind::Bait, free: 20, ahead: 3, gap: 60 });
        planned.route = Some(run);
        run.reset = Some(game::Reset { kind: ResetKind::Throw, free: 20, ahead: 3, gap: 60 });
        other.route = Some(run);
        assert_ne!(hash(&planned), hash(&other));
        let mut held = planned;
        held.bait_until = 500;
        assert_ne!(hash(&held), hash(&planned));
        let mut waiting = planned;
        waiting.reset_wait = Some(game::ResetWait { reset: game::Reset { kind: ResetKind::Low, free: 20, ahead: 3, gap: 60 }, cut: 90, facing_right: true, hit: false });
        assert_ne!(hash(&waiting), hash(&planned));
        let mut seen_hit = waiting;
        seen_hit.reset_wait.as_mut().unwrap().hit = true;
        assert_ne!(hash(&seen_hit), hash(&waiting));
    }

    /// What the opponent is shown doing for 20 frames from a route's cut.
    #[derive(Clone, Copy, PartialEq)]
    enum Seen {
        Standing,
        Reeling,
        KnockedDown,
    }

    /// A Very Difficult CPU that has taken a route cut into a throw reset
    /// (SC10, `#202`), fed frames with the opponent standing in reach, and
    /// for 20 frames from the cut as `shown`. The reset, and its pads from
    /// the step its fighter is free on.
    fn throw_reset_played(shown: Seen) -> (game::Reset, Vec<u16>) {
        let c = sf6_ctx(Level::VeryHard);
        let me = Body { facing_left: false, ..body(200) };
        let foe = Body { facing_left: true, ..body(260) };
        let at = |frame: u32, foe: Body| Percept { frame, fighting: true, me, foe, ..Percept::default() };
        for seed in 1..2_000u64 {
            let mut cpu = FairCpu::seeded(seed);
            for f in 1..=40 {
                cpu.observe(at(f, foe));
                cpu.press(f + 1, true, &c);
            }
            cpu.game = None;
            if !cpu.take_route(&c, at(40, foe), &[crate::route_book::Reach::Ground], game::NEUTRAL_WALK) {
                continue;
            }
            let Some(reset) = cpu.route.and_then(|run| run.reset).filter(|r| r.kind == ResetKind::Throw) else { continue };
            let mut cut = None;
            let mut pads = Vec::new();
            for f in 41..200u32 {
                let then = cut.is_some_and(|c: u32| f >= c && f < c + 20);
                let seen = match shown {
                    Seen::Reeling if then => Body { move_id: 0x30, state: 0x8a, ..foe },
                    // A hit reaction drawn off the ground: a knockdown.
                    Seen::KnockedDown if then => Body { move_id: 0x30, state: 0x8b, ..foe },
                    _ => foe,
                };
                cpu.observe(at(f, seen));
                let pad = cpu.press(f + 1, true, &c);
                if cut.is_none() && cpu.route.is_none() {
                    cut = Some(f + 1);
                }
                if let Some(c) = cut {
                    if f + 1 >= c + u32::from(reset.free - reset.ahead) {
                        pads.push(pad);
                    }
                }
            }
            return (reset, pads);
        }
        panic!("no throw reset taken");
    }

    #[test]
    fn a_throw_reset_goes_on_only_once_the_route_is_seen_to_hit() {
        // SC10 (`#202`): the same CPU, the same route cut into a throw. Seen
        // to hit him, it walks in and throws on the book's timing once its
        // fighter is free; seen to meet nothing, or to knock him down, it
        // lets the reset go.
        let (reset, hit) = throw_reset_played(Seen::Reeling);
        let (_, missed) = throw_reset_played(Seen::Standing);
        let (_, down) = throw_reset_played(Seen::KnockedDown);
        let cpu = FairCpu::new();
        let wait = usize::from(reset.free - reset.ahead);
        // Seen to hit by its first free frame, its pads go on from there as
        // they were planned from the cut.
        let want = &cpu.reset_pads(reset, RIGHT, 0)[wait..];
        assert_eq!(cpu.reset_pads(reset, RIGHT, wait), want);
        assert_eq!(&hit[..want.len()], want, "after a hit: {reset:?}");
        assert!(want.contains(&(RIGHT | button(2))));
        assert!(!missed[..want.len()].contains(&(RIGHT | button(2))), "after nothing: the reset's throw went in: {missed:?}");
        assert!(!down[..want.len()].contains(&(RIGHT | button(2))), "after a knockdown: the reset's throw went in: {down:?}");
    }

    #[test]
    fn of_two_routes_the_one_that_wins_the_round_is_never_cut_and_the_other_can_be() {
        // SC10 (`#202`): with 14 health bar points left (280 of an era
        // style's units), Ryu's SF6 routes of 300 damage win the round and
        // those of 260 or less do not. Over many CPUs, none of the winning
        // routes it takes is cut on purpose; some of the others are.
        let c = sf6_ctx(Level::VeryHard);
        let me = Body { facing_left: false, ..body(200) };
        let view = Percept { frame: 100, fighting: true, me, foe: Body { facing_left: true, health: 14, ..body(260) }, ..Percept::default() };
        let book = crate::route_book::book(TournamentFighter::Ryu, crate::era::Revision::Sf6, crate::era::Loadout::Standard).unwrap();
        let (mut winning, mut winning_cut, mut other, mut other_cut) = (0, 0, 0, 0);
        for seed in 1..=4_000u64 {
            let mut cpu = FairCpu::seeded(seed);
            if !cpu.take_route(&c, view, &[crate::route_book::Reach::Ground], game::NEUTRAL_WALK) {
                continue;
            }
            let run = cpu.route.unwrap();
            let route = &book.routes[usize::from(run.index)];
            // Might win on the 14 points shown (261 to 280 units).
            if u32::from(route.damage) * 48 > 13 * u32::from(crate::era_fight::FULL_SUB_HP) {
                winning += 1;
                winning_cut += u32::from(run.reset.is_some());
            } else {
                other += 1;
                other_cut += u32::from(run.reset.is_some());
            }
        }
        assert!(winning >= 1_000 && other >= 100, "winning routes taken {winning}, others {other}");
        assert_eq!(winning_cut, 0, "winning routes cut");
        assert!(other_cut >= 10, "other routes cut: {other_cut} of {other}");
    }

    #[test]
    fn a_route_that_might_win_the_round_on_the_health_shown_is_never_cut() {
        // SC10 (`#202`, second review): 24 points shown is anything from 461
        // to 480 of an era style's units (20 a point), so a 468-damage route
        // may win and is never cut; with 25 points shown it cannot, and is
        // cut sometimes. The same with a whole health of 1,200 (25 a point):
        // 24 points shown is 576 to 600, so 580 damage may win.
        let text = |damage: u16| {
            format!(
                "route-book 2\nfighter Ryu\nstyle Sf6/Standard\n\nroute ground\nname MP, 236+LP\nhits 2\ndamage {damage}\nrules true_combos cancels\nmeter super -0 +0 drive -0 +0\nknockdown yes\ncorner no\nstances standing\nconfirm 0\ngap 52\nfails-on\ncuts 1 free 24 ahead 3 gap 64\nstep MP | wait 0 0..0 | hold 0 | 0:02x2 0:00\nstep 236+LP | wait 0 0..5 | hold 0 | 4:00x2 5:00x2 1:00x2 1:01x3 1:00\nend\n"
            )
        };
        let c = sf6_ctx(Level::VeryHard);
        let cut = |damage: u16, foe_health: u16, foe_max_sub_hp: u16| {
            let book = crate::route_book::RouteBook::parse(&text(damage)).unwrap();
            let route = &book.routes[0];
            let moment = game::Moment {
                reaches: &[crate::route_book::Reach::Ground],
                era: true,
                normals_only: false,
                true_combos: true,
                cancels: true,
                crouching: false,
                meters: [0, 0],
                corner: false,
                gap: 52,
                walk: 0,
                foe_health,
                foe_max_sub_hp,
                spend_super: false,
                drive_reserve: 0,
                avoid_burnout: true,
                foe: None,
            };
            // A memory that asks for resets, so the chance is high.
            let mut cuts = 0;
            for seed in 1..=500u64 {
                let mut cpu = FairCpu::seeded(seed);
                for _ in 0..6 {
                    cpu.habits.record(Spot::Grabbed, GrabAnswer::NotTeched as usize);
                }
                cuts += u32::from(cpu.choose_reset(&c, route, &moment, route.len()).is_some());
            }
            cuts
        };
        let full = crate::era_fight::FULL_SUB_HP;
        assert_eq!(cut(468, 24, full), 0, "468 damage on 24 points shown (461 to 480)");
        assert_eq!(cut(480, 24, full), 0);
        assert!(cut(468, 25, full) >= 50, "468 damage on 25 points shown (481 to 500)");
        assert_eq!(cut(580, 24, 1_200), 0, "580 damage on 24 points of 1,200 (576 to 600)");
        assert!(cut(575, 24, 1_200) >= 50, "575 damage on 24 points of 1,200");
    }

    const LEVELS: [Level; 4] = [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard];

    #[test]
    fn each_level_is_a_set_of_values_scaled_down_from_the_top() {
        // SC11 (`#203`): every level sets each value, and each one below
        // Very Difficult gives up some of every one.
        let table = game::plans(Some(TournamentFighter::Ryu), Some(crate::era::EraFamily::Sf6));
        let plans = |level: Level| table.iter().filter(|p| p.for_level(level)).count();
        for pair in LEVELS.windows(2) {
            let (low, high) = (pair[0], pair[1]);
            assert!(low.rank() < high.rank());
            assert!(low.reaction() > high.reaction() && low.unfocused_reaction() > high.unfocused_reaction(), "{low:?} reacts slower");
            assert!(low.quality() < high.quality(), "{low:?} chooses worse");
            assert!(low.execution() < high.execution() && low.clean(true) < high.clean(true), "{low:?} slips more");
            assert!(low.guard() < high.guard(), "{low:?} guards less");
            assert!(low.route_hits() < high.route_hits(), "{low:?} takes shorter routes");
            assert!(low.pressure_plans() < high.pressure_plans(), "{low:?} backs off sooner");
            assert!(low.drive_kept() > high.drive_kept(), "{low:?} spends less meter");
            assert!(habit::uses(low) < habit::uses(high), "{low:?} uses less habit memory");
            assert!(low.reset_chance() < high.reset_chance(), "{low:?} resets less");
            assert!(plans(low) < plans(high), "{low:?} has fewer game plans");
        }
        // The maintainer's top: 97% clean motions, less under pressure, and
        // 90% best choice; it takes every plan and route, keeps no meter
        // back and stays on the opponent.
        let top = Level::VeryHard;
        assert_eq!((top.execution(), top.clean(false), top.quality()), (97, 97, 90));
        assert!(top.clean(true) < 97);
        assert_eq!(top.route_hits(), u8::MAX);
        assert_eq!(top.pressure_plans(), u8::MAX);
        assert_eq!(top.drive_kept(), 0);
        assert_eq!(plans(top), table.len());
        // Easy: no long route, no habit memory, no meter spent, no reset.
        assert!(Level::Easy.route_hits() < LONG_ROUTE);
        assert_eq!(habit::uses(Level::Easy), 0);
        assert!(!Level::Easy.spends_meter() && LEVELS[1..].iter().all(|l| l.spends_meter()));
        assert_eq!(Level::Easy.reset_chance(), 0);
    }

    #[test]
    fn a_level_takes_up_only_its_own_game_plans() {
        // SC11 (`#203`): each level takes every plan the one below takes.
        // Easy pokes, enters short routes, throws a projectile, jumps in
        // and walks in to throw after a guard, and no more: no frame trap,
        // no plan for a knocked-down opponent (only the poke's own follow-up
        // to a knockdown it caused), no bait, no hovering at the edge of
        // reach, no Drive Rush in behind a projectile.
        let table = game::plans(Some(TournamentFighter::Ryu), Some(crate::era::EraFamily::Sf6));
        for pair in LEVELS.windows(2) {
            assert!(table.iter().all(|p| !p.for_level(pair[0]) || p.for_level(pair[1])), "{:?} takes all {:?} takes", pair[1], pair[0]);
        }
        let easy: Vec<&game::GamePlan> = table.iter().filter(|p| p.for_level(Level::Easy)).collect();
        assert!(!easy.is_empty());
        for plan in &easy {
            let acts: Vec<game::Act> = plan.steps.iter().map(|s| s.act).collect();
            assert!(
                !acts.iter().any(|a| matches!(a, game::Act::GuardRise | game::Act::Edge | game::Act::Step { .. } | game::Act::Rush)),
                "{acts:?}"
            );
            assert!(plan.situation != Situation::OpponentKnockedDown, "{acts:?}");
            assert!(!plan.cornered_only);
            assert!(!(plan.situation == Situation::ItWasBlocked && acts.contains(&game::Act::Do(Choice::Wait))), "a frame trap at Easy");
        }
        // Normal adds no meter-spending or bait plan, Difficult no bait:
        // reading a reversal off a knockdown is the top's alone.
        for plan in table.iter().filter(|p| p.for_level(Level::Normal)) {
            assert!(!plan.steps.iter().any(|s| matches!(s.act, game::Act::Rush | game::Act::GuardRise)));
        }
        for plan in table.iter().filter(|p| p.for_level(Level::Hard)) {
            assert!(!plan.steps.iter().any(|s| s.act == game::Act::GuardRise));
        }
    }

    /// A whiff in punish range, with full gauges: every punish route of
    /// Ryu's SF6 book is in reach, and its ground routes too (the raw Super
    /// Art among them, three hits spending two bars).
    fn punish_moment(level: Level, stronger: bool, foe_health: u16) -> (Context, Percept) {
        let mut c = sf6_ctx(level);
        c.stronger = stronger;
        c.meters = [3 * u16::from(crate::era::STOCK_AT), 6 * u16::from(crate::era::STOCK_AT)];
        c.meter_full = true;
        let me = Body { facing_left: false, ..body(200) };
        // His hurtbox 44 pixels off: the punish routes start from 32 and
        // may walk 6 frames in; the ground routes from 52, and no nearer
        // than 8 inside that.
        let foe = Body { facing_left: true, health: foe_health, ..body(254) };
        (c, Percept { frame: 100, fighting: true, me, foe, ..Percept::default() })
    }

    /// The most hits of a route taken, whether any spent meter, and how
    /// many were taken, over `n` fresh CPUs at `level` in
    /// [`punish_moment`].
    fn routes_by_level(level: Level, stronger: bool, n: u64) -> (u8, bool, u32) {
        routes_at(level, stronger, 48, n)
    }

    /// The same against an opponent with `foe_health` points.
    fn routes_at(level: Level, stronger: bool, foe_health: u16, n: u64) -> (u8, bool, u32) {
        let (c, view) = punish_moment(level, stronger, foe_health);
        let book = crate::route_book::book(TournamentFighter::Ryu, crate::era::Revision::Sf6, crate::era::Loadout::Standard).unwrap();
        let (mut most, mut meter, mut taken) = (0, false, 0);
        for seed in 1..=n {
            let mut cpu = FairCpu::seeded(seed);
            if cpu.take_route(&c, view, &[crate::route_book::Reach::Punish, crate::route_book::Reach::Ground], game::PUNISH_WALK) {
                taken += 1;
                let route = &book.routes[usize::from(cpu.route.unwrap().index)];
                most = most.max(route.hits);
                meter |= route.super_spent + route.drive_spent > 0;
            }
        }
        (most, meter, taken)
    }

    #[test]
    fn easy_takes_no_long_route_and_none_that_spends_meter_and_no_level_a_longer_one_than_its_own() {
        // SC11 (`#203`).
        let n = 400;
        let top = routes_by_level(Level::VeryHard, true, n);
        assert_eq!(top.2, n as u32);
        assert!(top.0 >= LONG_ROUTE && top.1, "the moment allows long and meter-spending routes: {top:?}");
        for level in LEVELS {
            let (most, meter, taken) = routes_by_level(level, true, n);
            assert!(taken > n as u32 / 2, "{level:?} took {taken}");
            assert!(most <= level.route_hits(), "{level:?} took a route of {most} hits");
            if level == Level::Easy {
                assert!(most < LONG_ROUTE, "Easy took a long route of {most} hits");
                assert!(!meter, "Easy spent meter on a route");
            } else if level != Level::VeryHard {
                // Each level below the top takes as long a route as it may.
                assert_eq!(most, level.route_hits(), "{level:?}");
            }
        }
        // With 15 points left the raw Super Art (three hits, two bars) wins
        // the round: Normal spends the meter on it, Easy never does.
        assert!(routes_at(Level::Normal, true, 15, n).1, "the moment allows the Super Art to Normal");
        let easy = routes_at(Level::Easy, true, 15, n);
        assert!(easy.2 > n as u32 / 2 && !easy.1, "Easy spent meter on a route: {easy:?}");
        // The cap is the stronger play's: with the switch off the same
        // moment is not limited.
        let old = routes_by_level(Level::Easy, false, n);
        assert!(old.0 > Level::Easy.route_hits(), "{old:?}");
    }

    #[test]
    fn a_plan_its_level_does_not_take_is_never_taken_up() {
        // SC11 (`#203`): on a knocked-down opponent Easy has no plan; Normal
        // times an attack to his rise but never holds a guard over it to
        // bait a reversal, which only Very Difficult does.
        let view = Percept { frame: 100, fighting: true, me: Body { facing_left: false, ..body(200) }, foe: Body { facing_left: true, ..body(250) }, ..Percept::default() };
        let table = game::plans(Some(TournamentFighter::Ryu), Some(crate::era::EraFamily::Sf6));
        let taken = |level: Level| -> Vec<game::Act> {
            let c = sf6_ctx(level);
            (1..=300)
                .filter_map(|seed| {
                    let mut cpu = FairCpu::seeded(seed);
                    cpu.judged = Judged { situation: Situation::OpponentKnockedDown, ..Judged::default() };
                    cpu.start_game_plan(100, view, &c).then(|| table[usize::from(cpu.game.unwrap().plan)].steps[0].act)
                })
                .collect()
        };
        assert!(taken(Level::Easy).is_empty(), "Easy took {:?}", taken(Level::Easy));
        for level in [Level::Normal, Level::Hard] {
            let acts = taken(level);
            assert!(acts.len() > 100, "{level:?}: {}", acts.len());
            assert!(!acts.contains(&game::Act::GuardRise), "{level:?} baited a reversal");
        }
        assert!(taken(Level::VeryHard).contains(&game::Act::GuardRise), "Very Difficult never baited");
    }

    #[test]
    fn a_game_plans_drive_rush_keeps_the_levels_share_of_the_gauge() {
        // SC11 (`#203`): the projectile plan's Drive Rush step spends Drive
        // only down to the level's share. A Drive Rush costs 50 points;
        // Difficult keeps 24 of its own (the old play's reserve there is
        // none), so with 60 it does not rush in, and with a full gauge it
        // does. Very Difficult keeps none and rushes in with 60.
        let table = game::plans(Some(TournamentFighter::Ryu), Some(crate::era::EraFamily::Sf6));
        let plan = table.iter().position(|p| p.steps.iter().any(|s| s.act == game::Act::Rush)).unwrap();
        let step = table[plan].steps.iter().position(|s| s.act == game::Act::Rush).unwrap();
        let view = Percept { frame: 100, fighting: true, me: Body { facing_left: false, ..body(200) }, foe: Body { facing_left: true, ..body(330) }, ..Percept::default() };
        let rushes = |level: Level, drive: Option<u16>| {
            let c = sf6_ctx_drive(level, EraControl { free: true, ..EraControl::default() }, drive);
            let mut cpu = FairCpu::seeded(1);
            cpu.judged = Judged { situation: Situation::NeutralFar, ..Judged::default() };
            cpu.game = Some(game::Running { plan: plan as u8, step: step as u8, acted: false, pressed: 0, began: 100, hit: false });
            let pads = cpu.follow_game_plan(100, view, &c);
            pads.is_some() && cpu.game.is_some_and(|run| usize::from(run.step) == step && run.acted)
        };
        assert!(rushes(Level::Hard, None), "Difficult rushes in from a full gauge");
        assert!(!rushes(Level::Hard, Some(60)), "Difficult spent its kept share");
        assert!(rushes(Level::VeryHard, Some(60)), "Very Difficult keeps none back");
        assert!(!rushes(Level::Easy, None), "Easy spends no Drive of its own");
    }

    /// Whether a fresh CPU at `level`, seeded `seed`, having taken `run`
    /// pressure plans in a row, takes up another after a guarded attack,
    /// and whether it backs off instead.
    fn presses(level: Level, run: u8, seed: u64) -> (bool, bool) {
        let c = sf6_ctx(level);
        let mut cpu = FairCpu::seeded(seed);
        cpu.judged = Judged { situation: Situation::ItWasBlocked, corner: false, foe_cornered: false };
        cpu.pressure_run = run;
        let view = Percept { frame: 100, fighting: true, me: Body { facing_left: false, ..body(200) }, foe: Body { facing_left: true, ..body(250) }, ..Percept::default() };
        let started = cpu.start_game_plan(100, view, &c);
        let backs = !started && cpu.backs_off(&c);
        if backs {
            assert_eq!(cpu.pressure_run, 0, "backing off starts the count again");
        }
        (started, backs)
    }

    #[test]
    fn after_its_levels_run_of_pressure_plans_it_backs_off_and_very_difficult_never_does() {
        // SC11 (`#203`): how long it stays on the opponent is a level value.
        for level in [Level::Easy, Level::Normal, Level::Hard] {
            let cap = level.pressure_plans();
            let before: Vec<(bool, bool)> = (1..=100).map(|seed| presses(level, cap - 1, seed)).collect();
            assert!(before.iter().filter(|r| r.0).count() > 30, "{level:?} presses before its cap");
            assert!(before.iter().all(|r| !r.1), "{level:?} backed off before its cap");
            let after: Vec<(bool, bool)> = (1..=100).map(|seed| presses(level, cap, seed)).collect();
            assert!(after.iter().all(|&r| r == (false, true)), "{level:?} at its cap: {after:?}");
        }
        let top: Vec<(bool, bool)> = (1..=100).map(|seed| presses(Level::VeryHard, u8::MAX, seed)).collect();
        assert!(top.iter().filter(|r| r.0).count() > 60 && top.iter().all(|r| !r.1), "{top:?}");
        // Counting: a pressure plan adds one, a neutral plan starts again.
        let c = sf6_ctx(Level::Normal);
        let view = Percept { frame: 100, fighting: true, me: Body { facing_left: false, ..body(200) }, foe: Body { facing_left: true, ..body(250) }, ..Percept::default() };
        let mut cpu = (1..=50)
            .map(FairCpu::seeded)
            .find_map(|mut cpu| {
                cpu.judged = Judged { situation: Situation::ItWasBlocked, ..Judged::default() };
                cpu.start_game_plan(100, view, &c).then_some(cpu)
            })
            .unwrap();
        assert_eq!(cpu.pressure_run, 1);
        cpu.game = None;
        cpu.judged = Judged { situation: Situation::NeutralClose, ..Judged::default() };
        while !cpu.start_game_plan(200, view, &c) {}
        assert_eq!(cpu.pressure_run, 0);
        // The count is spent when a round ends, and a new match starts it.
        let mut cpu = FairCpu::seeded(1);
        cpu.pressure_run = 1;
        cpu.press(10, false, &sf6_ctx(Level::Easy));
        assert_eq!(cpu.pressure_run, 0, "a round's end clears it");
        cpu.pressure_run = 1;
        cpu.new_match();
        assert_eq!(cpu.pressure_run, 0);
        // A pair with no game plans never backs off: nothing is counted.
        let mut cpu = FairCpu::seeded(1);
        cpu.judged = Judged { situation: Situation::ItWasBlocked, ..Judged::default() };
        cpu.pressure_run = 5;
        assert!(!cpu.backs_off(&ctx(Level::Easy)));
    }

    #[test]
    fn the_measuring_run_can_set_a_level_of_its_own_and_the_hash_covers_it_and_the_pressure_count() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        let cpu = FairCpu::seeded(3);
        assert_eq!(cpu.forced_level(), None);
        for level in LEVELS {
            assert_eq!(cpu.force_level(Some(level)).forced_level(), Some(level));
            assert_ne!(hash(&cpu.force_level(Some(level))), hash(&cpu));
        }
        assert_eq!(cpu.force_level(Some(Level::Hard)).force_level(None).forced_level(), None);
        let mut pressed = cpu;
        pressed.pressure_run = 1;
        assert_ne!(hash(&pressed), hash(&cpu));
    }

    #[test]
    fn under_a_normals_only_mission_the_stronger_play_takes_no_game_plan_that_needs_a_special() {
        // SC23 (`#215`): Pure Footsies. Its kit loses its specials and the
        // era commands, and keeps its rules; far off it takes no projectile
        // or Drive Rush plan and close in no route plan (Ryu's SF6 book has
        // no route of plain normals), where the same CPUs without the
        // mission take them; it still takes plans.
        let open = sf6_ctx(Level::VeryHard);
        let footsies = Context { pure_footsies: true, ..open }.normals_and_throws();
        assert!(footsies.kit.projectile.is_none() && footsies.kit.anti_air.is_none() && footsies.kit.advance.is_none());
        assert!(footsies.era.is_none() && footsies.kit.fighter == open.kit.fighter && footsies.style == open.style);
        assert_eq!((footsies.true_combos, footsies.cancels, footsies.throws), (open.true_combos, open.cancels, open.throws));
        assert!(open.normals_and_throws().era.is_some(), "without the mission nothing is set aside");
        let table = game::plans(Some(TournamentFighter::Ryu), Some(crate::era::EraFamily::Sf6));
        let me = Body { facing_left: false, ..body(100) };
        for (situation, foe_x) in [(Situation::NeutralFar, 240), (Situation::NeutralClose, 150)] {
            let foe = Body { facing_left: true, ..body(foe_x) };
            let view = Percept { frame: 100, fighting: true, me, foe, ..Percept::default() };
            let (mut taken, mut special, mut route) = ([0; 2], 0, 0);
            for seed in 1..=400u64 {
                for (i, ctx) in [open, footsies].iter().enumerate() {
                    let mut cpu = FairCpu::seeded(seed);
                    cpu.judged = Judged { situation, ..Judged::default() };
                    if !cpu.start_game_plan(100, view, ctx) {
                        continue;
                    }
                    taken[i] += 1;
                    let plan = &table[usize::from(cpu.game.unwrap().plan)];
                    let routes = plan.steps.iter().any(|step| matches!(step.act, game::Act::Route(_)));
                    if i == 0 {
                        special += u32::from(plan.needs_specials());
                        route += u32::from(routes);
                    } else {
                        assert!(!plan.needs_specials() && !routes, "{situation:?}: {plan:?}");
                    }
                }
            }
            assert!(taken[1] >= 100, "{situation:?}: plans taken under the mission {taken:?}");
            match situation {
                Situation::NeutralFar => assert!(special >= 50, "far off without the mission: {special} projectile or Drive Rush plans"),
                _ => assert!(route >= 50, "close in without the mission: {route} route plans"),
            }
        }
    }

    #[test]
    fn a_normals_only_mission_leaves_the_old_play_as_it_was() {
        // SC23 (`#215`): the old play never kept to the mission (the engine
        // refuses its specials), and still does not: the same pads with the
        // mission and without, at every level, while the opponent walks in
        // and out of reach. The stronger play's pads differ, so the check
        // reaches a CPU that presses for specials.
        let foe_at = |f: u32| Body { facing_left: false, ..body(100 + ((f / 3) % 160) as i16) };
        let me = Body { facing_left: true, ..body(300) };
        let mut stronger_parted = false;
        for level in LEVELS {
            for (open, strong) in [(ctx(level), false), (sf6_ctx(level), true)] {
                let open = Context { stronger: strong, ..open };
                let footsies = Context { pure_footsies: true, ..open };
                for seed in 1..=6u64 {
                    let (mut a, mut b) = (FairCpu::seeded(seed), FairCpu::seeded(seed));
                    for f in 1..=900u32 {
                        let p = Percept { frame: f, fighting: true, me, foe: foe_at(f), ..Percept::default() };
                        a.observe(p);
                        b.observe(p);
                        let (pa, pb) = (a.press(f, true, &open), b.press(f, true, &footsies));
                        if strong {
                            stronger_parted |= pa != pb;
                        } else {
                            assert_eq!(pa, pb, "{level:?} seed {seed} frame {f}");
                        }
                    }
                    if !strong {
                        assert_eq!(a, b, "{level:?} seed {seed}");
                    }
                }
            }
        }
        assert!(stronger_parted, "the stronger play keeps to the mission");
    }

    #[test]
    fn sf5_skills_counters_shift_and_reversal_wait_for_the_applicable_reaction_delay() {
        use crate::era::{GaugeMode, Loadout, Revision, StylePick};
        use TournamentFighter::{Ken, Ryu};
        let free = EraControl { free: true, ..EraControl::default() };
        let guarding = EraControl { blockstun: true, ..EraControl::default() };
        let medium = button(1) | button(4);
        let heavy = button(2) | button(5);
        let shift = button(2) | button(4);
        let punches = button(0) | button(1) | button(2);
        let kicks = button(3) | button(4) | button(5);
        let mut attack = body(340);
        attack.move_id = 0x10;
        attack.state = 0xc2;
        // The twins keep the same own control availability, native gauges,
        // seeded choices and percept history. Only B sees the normal begin.
        // For V-Reversal both already feel blockstun; seeing the attack is
        // still needed before the command can differ from A's quiet pad.
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            for (fighter, loadout, v, control, buttons, direction, paid, what) in [
                (Ryu, Loadout::Skill1Trigger1, (0, 0, 0), free, medium, 0, false, "Mind's Eye"),
                (Ryu, Loadout::Skill2Trigger1, (0, 0, 0), free, medium, 0, false, "Thrust Strike"),
                (Ryu, Loadout::Skill1Trigger2, (0, 1500, 1500), free, heavy, 0, true, "Isshin"),
                (Ken, Loadout::Skill1Trigger1, (96, 0, 0), free, shift, 0, true, "V-Shift"),
                (Ryu, Loadout::Skill1Trigger1, (96, 0, 0), guarding, punches, RIGHT, true, "punch V-Reversal"),
                (Ken, Loadout::Skill1Trigger1, (96, 0, 0), guarding, kicks, RIGHT, true, "kick V-Reversal"),
            ] {
                let config = crate::config::MatchConfig::definitive();
                let mut state = EraFighterState::default();
                state.fighter = Some(fighter);
                state.style = StylePick::for_revision(Revision::Sf5Ce, loadout);
                state.max_sub_hp = crate::era_fight::FULL_SUB_HP;
                state.set_gauge_mode(GaugeMode::EraDefault);
                state.set_sf5_practice_points(v);
                let mut ctx = Context::with_era(level, fighter, Some(RYU_SLOTS), config, &state, control);
                ctx.set_opponent(crate::era::EraFamily::Sf5, crate::era_fight::FULL_SUB_HP);
                ctx.stronger = true;
                let ctx = ctx.for_stronger();
                let mut answers = 0;
                for seed in 1..=96 {
                    let mut a = FairCpu::seeded(seed);
                    for f in 1..=40 {
                        a.observe(percept(f, body(340)));
                    }
                    for f in 41..=80 {
                        a.press(f, true, &ctx);
                        a.observe(percept(f, body(340)));
                    }
                    let mut b = a;
                    let shown = 81;
                    let mut earliest = shown + level.unfocused_reaction();
                    let mut answered = false;
                    for f in shown..shown + 120 {
                        if f >= shown + level.reaction() && f < earliest && b.attention().focus == Focus::Attack {
                            earliest = f;
                        }
                        let (pa, pb) = (a.press(f, true, &ctx), b.press(f, true, &ctx));
                        assert!(pa == pb || f >= earliest, "{level:?} {what}, seed={seed}: pad differed at {f}, earliest {earliest}");
                        answered |= pa != pb && f >= earliest && b.plan.answer
                            && pb & BUTTONS == buttons && pb & direction == direction;
                        a.observe(percept(f, body(340)));
                        b.observe(percept(f, attack));
                    }
                    answers += usize::from(answered);
                }
                if paid && !level.spends_meter() {
                    assert_eq!(answers, 0, "{level:?} {what}: Easy keeps its meter");
                } else {
                    assert!(answers > 0, "{level:?} {what}: no distinct real command after its reaction delay");
                }
            }
        }
    }

    #[test]
    fn sf5_game_plan_selection_obeys_live_gauges_levels_and_pure_footsies() {
        use crate::era::{GaugeMode, Loadout, Revision, StylePick};
        let build = |level, gauge, full| {
            let mut config = crate::config::MatchConfig::definitive();
            config.gauge_mode = gauge;
            let mut state = EraFighterState::default();
            state.fighter = Some(TournamentFighter::Ken);
            state.style = StylePick::for_revision(Revision::Sf5Ce, Loadout::Skill1Trigger2);
            state.max_sub_hp = crate::era_fight::FULL_SUB_HP;
            state.set_gauge_mode(gauge);
            if full {
                state.super_meter = 144;
                state.set_sf5_practice_points((96, 0, 0));
            }
            let mut ctx = Context::with_era(level, TournamentFighter::Ken, Some(RYU_SLOTS), config, &state, EraControl { free: true, ..EraControl::default() });
            ctx.stronger = true;
            ctx.for_stronger()
        };
        let skill = game::Act::Era(game::EraAct::VSkill);
        let ex = game::Act::Era(game::EraAct::Enhanced);
        let trigger = game::Act::Era(game::EraAct::VTrigger);
        let art = game::Act::Era(game::EraAct::Super);
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            let full = build(level, GaugeMode::EraDefault, true);
            let paid = [(ex, Situation::NeutralClose), (trigger, Situation::ItHit), (art, Situation::ItHit)];
            for (act, situation) in paid {
                assert_eq!(plans_taken(&full, 270, 300, situation, 96).contains(&act), level.spends_meter(), "{level:?} {act:?}: affordable plan at this level");
                assert!(!plans_taken(&build(level, GaugeMode::EraDefault, false), 270, 300, situation, 96).contains(&act), "{level:?} {act:?}: empty meter");
            }
            assert_eq!(plans_taken(&full, 270, 300, Situation::ItWasBlocked, 96).contains(&skill), level != Level::Easy, "{level:?}: V-Skill has its own pressure step");
            let mut footsies = full;
            footsies.pure_footsies = true;
            let footsies = footsies.for_stronger();
            for ctx in [footsies, build(level, GaugeMode::NoGauges, true)] {
                for (act, situation) in [(skill, Situation::ItWasBlocked), (ex, Situation::NeutralClose), (trigger, Situation::ItHit), (art, Situation::ItHit)] {
                    let game::Act::Era(era) = act else { unreachable!() };
                    assert!(!era_ready(era, &ctx), "{level:?} {act:?}: live controls remove the tool");
                    assert!(!plans_taken(&ctx, 270, 300, situation, 96).contains(&act), "{level:?} {act:?}: unavailable plans stay out");
                }
                assert!(!plans_taken(&ctx, 270, 300, Situation::NeutralClose, 96).is_empty(), "{level:?}: ordinary play remains available");
            }
        }
    }

    // These are gate regressions using delayed foe observations and Mike's
    // real SF1+ book. Direct react calls isolate readiness from plan selection;
    // they do not claim native route completion or public-entry delay coverage.
    fn sc21_punish_ready_context(revision: crate::era::Revision, level: Level) -> Context {
        let empty = SpecialSlot { tracker: 0, kick: false, melee: false, low_in_air: false, score: 0 };
        // Mike's exact data_bank.rs player slots: Iron Upper and Bison Rush.
        let slots = [
            SpecialSlot { tracker: 0x19, kick: false, melee: true, low_in_air: false, score: 0x45f52 },
            SpecialSlot { tracker: 0x1c, kick: false, melee: true, low_in_air: false, score: 0x4345e },
            empty,
        ];
        let mut context = Context::new(level,
            Kit::of(TournamentFighter::Mike, RulesetPreset::DefinitiveOverhaul, Some(slots)),
            &CoreToggles::definitive())
            .with_baseline(crate::era::StylePick::for_revision(revision, crate::era::Loadout::Standard), false);
        context.stronger = true;
        context.foe = Some((TournamentFighter::Ryu, (revision, crate::era::Loadout::Standard)));
        // Isolate the existing single-normal punish under the supported
        // mission; Mike otherwise always chooses Iron Upper here above Easy.
        context.pure_footsies = revision == crate::era::Revision::Arcade1987;
        context.for_stronger()
    }

    fn sc21_punish_ready_latest(cpu: &mut FairCpu, me: Body) {
        // The latest opponent intentionally contradicts the delayed attack.
        // Only own state is available without waiting out the reaction delay.
        cpu.observe(Percept { frame: 99, fighting: true, me, foe: body(500), ..Default::default() });
    }

    fn sc21_punish_ready_fixture(seed: u64, met: bool, active_before: bool) -> (FairCpu, Percept, Percept) {
        let mut cpu = FairCpu::seeded(seed);
        cpu.fighter = Some(TournamentFighter::Mike);
        let mut previous = None;
        for frame in 84..=88 {
            let mut me = body(200);
            me.move_id = 0x10;
            me.state = 0xc2;
            let mut foe = body(242);
            if frame >= 85 {
                foe.move_id = 0x11;
                foe.state = 0xc6;
                foe.active = frame == 86 || (active_before && frame == 87);
                foe.front = if met { 200 } else { 220 };
            }
            if met && frame == 86 { me.move_id = 4; me.state = 0x12; }
            let view = Percept { frame, fighting: true, me, foe, ..Default::default() };
            cpu.judged = cpu.judge.judge(view, previous, 66);
            cpu.observe(view);
            previous = Some(view);
        }
        let view = cpu.seen(88).unwrap();
        let before = cpu.seen(87).unwrap();
        assert!(view.me.attacking() && !view.me.free() && !view.me.reacting());
        assert!(view.foe.attacking() && !view.foe.active && !view.foe.jumping());
        assert_eq!(view.gap(), 32, "Mike's real Punish record gap");
        assert_eq!(cpu.judge.recovering_after_meeting(view), met);
        assert_eq!(cpu.judged.situation == Situation::OpponentWhiffed, !met);
        let mut latest = body(200);
        latest.move_id = 7;
        latest.state = 0x14;
        sc21_punish_ready_latest(&mut cpu, latest);
        cpu.plan = Plan::guard(true, 10);
        cpu.settle = 95;
        assert!(cpu.plan.interruptible() && cpu.bait_until == 0);
        (cpu, view, before)
    }

    #[test]
    fn sc21_punish_readiness_fixture_existing_free_self_paths_are_real_positive_controls() {
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            for (revision, met, edge) in [
                (crate::era::Revision::Sf1Plus, false, false),
                (crate::era::Revision::Sf1Plus, true, false),
                (crate::era::Revision::Arcade1987, false, true),
            ] {
                let context = sc21_punish_ready_context(revision, level);
                let mut entered = 0;
                for seed in 1..=64 {
                    let (mut cpu, view, before) = sc21_punish_ready_fixture(seed, met, edge);
                    let free_view = Percept { me: cpu.seen(99).unwrap().me, ..view };
                    let answer = cpu.react(100, free_view, before, &context);
                    let concrete = if revision == crate::era::Revision::Sf1Plus {
                        cpu.route.is_some()
                    } else {
                        cpu.last_normal.is_some() && cpu.route.is_none()
                    };
                    if revision == crate::era::Revision::Sf1Plus {
                        assert_eq!(answer.is_some(), concrete, "{revision:?}/{level:?}/met={met}/seed{seed}: positive entry has a real own-book route; answer={answer:?}");
                    } else {
                        assert!(!context.true_combos && !context.cancels && !context.throws);
                        assert!(context.pure_footsies && context.kit.anti_air.is_none());
                        assert!(cpu.route.is_none(), "1987 never enters an SF1+ route");
                        assert_eq!(answer.is_some(), concrete, "the normals-only mission enters a real single-normal plan");
                    }
                    entered += usize::from(concrete);
                }
                assert!(entered > 0, "{revision:?}/{level:?}/met={met}: existing free-self path must actually enter a legal book route or 1987 single");
            }
        }
    }

    fn sc21_punish_ready_matches_existing_free_self_path(met: bool) {
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            for forced in [false, true] {
                let mut context = sc21_punish_ready_context(crate::era::Revision::Sf1Plus, level);
                context.stronger = !forced;
                let mut routes = 0;
                for seed in 1..=64 {
                    let (mut actual, view, before) = sc21_punish_ready_fixture(seed, met, false);
                    actual = actual.force_stronger(forced);
                    let mut control = actual;
                    let free_view = Percept { me: actual.seen(99).unwrap().me, ..view };
                    let expected = control.react(100, free_view, before, &context);
                    let answer = actual.react(100, view, before, &context);
                    routes += usize::from(control.route.is_some());
                    assert_eq!(answer, expected, "{level:?}, forced={forced}, seed={seed}: stale own attack must not deny the same delayed punish");
                    assert_eq!(actual, control, "{level:?}, forced={forced}, seed={seed}: same existing quality/route decisions and RNG");
                }
                assert!(routes > 0, "{level:?}, forced={forced}: real legal Mike book positive control must select routes");
            }
        }
    }

    #[test]
    fn sc21_baseline_judged_whiff_uses_felt_self_readiness_without_younger_foe() {
        sc21_punish_ready_matches_existing_free_self_path(false);
    }

    #[test]
    fn sc21_baseline_guarded_recovery_uses_felt_self_readiness() {
        sc21_punish_ready_matches_existing_free_self_path(true);
    }

    #[test]
    fn sc21_baseline_1987_bookless_whiff_uses_felt_self_readiness_for_legal_single() {
        for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
            for forced in [false, true] {
                let mut context = sc21_punish_ready_context(crate::era::Revision::Arcade1987, level);
                context.stronger = !forced;
                assert!(!context.true_combos && !context.cancels && !context.throws);
                let mut normals = 0;
                for seed in 1..=64 {
                    let (mut actual, view, before) = sc21_punish_ready_fixture(seed, false, true);
                    actual = actual.force_stronger(forced);
                    let mut control = actual;
                    let free_view = Percept { me: actual.seen(99).unwrap().me, ..view };
                    let expected = control.react(100, free_view, before, &context);
                    let answer = actual.react(100, view, before, &context);
                    normals += usize::from(control.last_normal.is_some());
                    assert!(actual.route.is_none() && control.route.is_none(), "1987 cannot take an SF1+ route");
                    assert_eq!(answer, expected, "{level:?}, forced={forced}, seed={seed}: same legal single after the actual own recovery");
                    assert_eq!(actual, control, "same old single-normal choice and RNG after passing readiness");
                }
                assert!(normals > 0, "the positive control actually enters existing normal punish inputs");
            }
        }
    }

    #[test]
    fn sc21_baseline_punish_rejects_latest_busy_self_even_if_delayed_self_is_free() {
        for (revision, met, edge) in [
            (crate::era::Revision::Sf1Plus, false, false),
            (crate::era::Revision::Sf1Plus, true, false),
            (crate::era::Revision::Arcade1987, false, true),
        ] {
            let context = sc21_punish_ready_context(revision, Level::VeryHard);
            for (move_id, state) in [(0x10, 0xc2), (0x30, 0x12), (9, 0x13), (1, 0x12)] {
                for seed in 1..=32 {
                    let (mut cpu, view, before) = sc21_punish_ready_fixture(seed, met, edge);
                    let mut latest = body(200);
                    latest.move_id = move_id;
                    latest.state = state;
                    if move_id == 1 { latest.present = false; }
                    assert!(!latest.free() && !latest.guarding());
                    sc21_punish_ready_latest(&mut cpu, latest);
                    let delayed_free = Percept { me: body(200), ..view };
                    assert!(cpu.react(100, delayed_free, before, &context).is_none(), "{revision:?}/{met}/{move_id:x}/{seed}: actual own busy cannot start a punish");
                    assert!(cpu.route.is_none() && cpu.last_normal.is_none());
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_punish_current_blockstun_protects_a_free_guard_pose() {
        for (revision, met, edge) in [
            (crate::era::Revision::Sf1Plus, false, false),
            (crate::era::Revision::Sf1Plus, true, false),
            (crate::era::Revision::Arcade1987, false, true),
        ] {
            let mut context = sc21_punish_ready_context(revision, Level::VeryHard);
            context.own_blockstun = true;
            for seed in 1..=32 {
                let (mut cpu, view, before) = sc21_punish_ready_fixture(seed, met, edge);
                let mut guard = body(200);
                guard.move_id = 4;
                assert!(guard.free() && guard.guarding());
                sc21_punish_ready_latest(&mut cpu, guard);
                let delayed_free = Percept { me: body(200), ..view };
                assert!(cpu.react(100, delayed_free, before, &context).is_none(), "{revision:?}/{met}/{seed}: native current blockstun still locks the guard");
                assert!(cpu.route.is_none() && cpu.last_normal.is_none());
            }
        }
    }

    #[test]
    fn sc21_baseline_punish_missing_or_nonfighting_own_history_keeps_delayed_readiness() {
        let context = sc21_punish_ready_context(crate::era::Revision::Sf1Plus, Level::VeryHard);
        for met in [false, true] {
            for seed in 1..=32 {
                for missing in [false, true] {
                    let (mut cpu, view, before) = sc21_punish_ready_fixture(seed, met, false);
                    cpu.settle = 0;
                    if missing { cpu.seen[99usize % RING] = Percept::default(); }
                    else { let mut latest = cpu.seen(99).unwrap(); latest.fighting = false; cpu.observe(latest); }
                    assert_eq!(cpu.felt(100, view), view);
                    let rng = cpu.rng;
                    assert!(cpu.react(100, view, before, &context).is_none());
                    assert!(cpu.route.is_none() && cpu.last_normal.is_none());
                    assert_eq!(cpu.rng, rng, "missing own observation must not invent readiness or an extra quality roll");
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_punish_ignores_younger_foe_history_and_waits_for_own_settle() {
        let context = sc21_punish_ready_context(crate::era::Revision::Sf1Plus, Level::VeryHard);
        for seed in 1..=32 {
            let (mut a, view, before) = sc21_punish_ready_fixture(seed, false, false);
            let mut b = a;
            let mut latest = b.seen(99).unwrap();
            latest.foe = body(100);
            latest.foe.move_id = 0x10;
            latest.foe.state = 0xc2;
            latest.foe.active = true;
            latest.foe.front = 205;
            b.observe(latest);
            assert_eq!(a.felt(100, view), b.felt(100, view), "felt preserves the exact delayed opponent");
            assert_eq!(a.react(100, view, before, &context), b.react(100, view, before, &context));
            a.seen = b.seen;
            assert_eq!(a, b, "younger foe cannot alter the punish decision/RNG");
            let (mut waiting, view, before) = sc21_punish_ready_fixture(seed, false, false);
            waiting.settle = 101;
            assert!(waiting.react(100, view, before, &context).is_none());
            assert!(waiting.route.is_none() && waiting.last_normal.is_none());
        }
    }

    #[test]
    fn sc21_punish_readiness_off_and_later_eras_keep_old_self_gate_and_rng() {
        let mut off = sc21_punish_ready_context(crate::era::Revision::Sf1Plus, Level::VeryHard);
        off.stronger = false;
        off.family = None;
        off.style = None;
        for context in [off, sf6_ctx(Level::VeryHard).for_stronger()] {
            assert!(!context.stronger || !context.baseline_rules());
            for (met, edge) in [(false, false), (true, false), (false, true)] {
                for seed in 1..=32 {
                    let (mut a, view, before) = sc21_punish_ready_fixture(seed, met, edge);
                    a.settle = 0;
                    let mut b = a;
                    let mut latest = b.seen(99).unwrap();
                    latest.me.move_id = 0x10;
                    latest.me.state = 0xc2;
                    b.observe(latest);
                    let rng = a.rng;
                    assert!(a.react(100, view, before, &context).is_none());
                    assert!(b.react(100, view, before, &context).is_none());
                    assert_eq!(a.rng, rng, "old delayed own attack blocks before quality, even if current own is free");
                    a.seen = b.seen;
                    assert_eq!(a, b, "OFF/later decisions and RNG keep the delayed-own gate");
                }
            }
        }
    }

    // The Gen native witness was old move0x34/state0x89 after its actual
    // body had returned to move1/state0x12. These tests enter real decide,
    // retaining delayed foe/contact history; they isolate cleanup, not Judge.
    fn sc21_hurt_body(hurt: bool) -> Body {
        if hurt { Body { move_id: 0x34, state: 0x89, ..body(200) } }
        else { Body { move_id: 1, state: 0x12, ..body(200) } }
    }

    fn sc21_hurt_context(revision: crate::era::Revision, configured: bool) -> Context {
        let mut context = sc21_own_advance_context(TournamentFighter::Gen, revision, configured, true, false).for_stronger();
        context.foe = Some((TournamentFighter::Ryu, (revision, crate::era::Loadout::Standard)));
        context
    }

    fn sc21_hurt_fixture(context: &Context, delayed_hurt: bool, current_hurt: bool, forced: bool) -> (FairCpu, Percept, Percept) {
        let mut cpu = FairCpu::seeded(17).force_stronger(forced);
        cpu.fighter = context.kit.fighter;
        for frame in 1..=99 {
            cpu.observe(Percept {
                frame, fighting: true,
                me: sc21_hurt_body(if frame <= 88 { delayed_hurt } else { current_hurt }),
                // A younger foe on the opposite side must not become its aim.
                foe: body(if frame <= 88 { 400 } else { 100 }),
                ..Default::default()
            });
        }
        let (index, entry) = game::plans(context.kit.fighter, context.family).iter().enumerate()
            .find(|(_, p)| p.situation == Situation::NeutralFar && matches!(p.steps[0].act, game::Act::Reach { .. }))
            .expect("Gen's actual baseline table has an ordinary far Reach");
        assert!(matches!(entry.steps[0].act, game::Act::Reach { .. }));
        cpu.game = Some(game::Running { plan: index as u8, step: 0, acted: true, pressed: 90, began: 90, hit: false });
        cpu.plan = Plan::of(PlanKind::Walk, &[RIGHT; 8]);
        cpu.plan.at = 1;
        cpu.settle = 0;
        (cpu, cpu.seen(88).unwrap(), cpu.seen(87).unwrap())
    }

    fn sc21_hurt_owned_route(cpu: &mut FairCpu, context: &Context) -> (game::RouteRun, game::Reset) {
        use crate::route_book::{Reach, Stance};
        let fighter = context.kit.fighter.unwrap();
        let (revision, loadout) = context.style.unwrap();
        let book = crate::route_book::book(fighter, revision, loadout).expect("Gen's real SF1+ book");
        let (index, route) = book.routes.iter().enumerate().find(|(_, r)| r.reach == Reach::Ground
            && !r.corner && r.super_spent == 0 && r.drive_spent == 0 && r.stances.contains(&Stance::Standing)
            && !r.cuts.is_empty()).expect("an actual route with a recorded reset cut");
        assert!(context.true_combos && context.cancels && route.hits <= context.level.route_hits());
        let plan = game::plans(context.kit.fighter, context.family).iter().position(|p|
            matches!(p.steps[0].act, game::Act::Route(Reach::Ground))).expect("the actual own-book plan");
        cpu.game = Some(game::Running { plan: plan as u8, step: 0, acted: true, pressed: 90, began: 90, hit: false });
        let run = game::RouteRun::new(fighter, revision, loadout, index as u8, 0, route.len() as u16, true);
        let cut = &route.cuts[0];
        let reset = game::Reset { kind: ResetKind::Bait, free: cut.free, ahead: cut.ahead, gap: cut.gap };
        assert!(reset.free > 5, "the recorded reset is still waiting at step100");
        (run, reset)
    }

    #[test]
    fn sc21_baseline_decide_own_hurt_feedback_ends_or_keeps_real_game_run_now() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for forced in [false, true] {
                let context = sc21_hurt_context(revision, !forced);
                for delayed_hurt in [false, true] {
                    for current_hurt in [false, true] {
                        let (mut cpu, view, before) = sc21_hurt_fixture(&context, delayed_hurt, current_hurt, forced);
                        let run = cpu.game;
                        let pad = cpu.decide(100, view, Some(before), &context);
                        assert_eq!(cpu.game, if current_hurt { None } else { run },
                            "{revision:?}/forced{forced}/old{delayed_hurt}/now{current_hurt}: cleanup follows actual own reaction");
                        assert_eq!(pad, RIGHT, "the already queued native walk still uses the delayed foe");
                        assert_eq!(cpu.plan.at, 2, "the real decide/next_pad chain consumed one queued pad");
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_decide_own_hurt_feedback_keeps_or_clears_actual_route_and_reset_bait() {
        for forced in [false, true] {
            let context = sc21_hurt_context(crate::era::Revision::Sf1Plus, !forced);
            for reset_phase in [false, true] {
                for delayed_hurt in [false, true] {
                    for current_hurt in [false, true] {
                        let (mut cpu, view, before) = sc21_hurt_fixture(&context, delayed_hurt, current_hurt, forced);
                        let (route, reset) = sc21_hurt_owned_route(&mut cpu, &context);
                        let run = cpu.game;
                        let expected_route_pad = route.pad().unwrap();
                        if reset_phase {
                            cpu.reset_wait = Some(game::ResetWait { reset, cut: 95, facing_right: true, hit: false });
                            cpu.plan = cpu.bait_plan(reset, 95);
                        } else {
                            cpu.route = Some(route);
                        }
                        let waiting = cpu.reset_wait;
                        let bait = cpu.bait_until;
                        let pad = cpu.decide(100, view, Some(before), &context);
                        if current_hurt {
                            assert!(cpu.game.is_none() && cpu.route.is_none() && cpu.reset_wait.is_none(), "current own hit cancels every active intent phase");
                            assert_eq!(cpu.bait_until, 0);
                        } else {
                            assert_eq!(cpu.game, run, "stale own reaction cannot discard the live own-book intent");
                            if reset_phase {
                                assert!(cpu.route.is_none());
                                assert_eq!(cpu.reset_wait, waiting, "the real recorded cut keeps waiting");
                                assert_eq!(cpu.bait_until, bait, "its deliberately held bait is not erased by old hurt");
                            } else {
                                let next = cpu.route.expect("the actual owned route continues");
                                assert_eq!(next.index, route.index);
                                assert_eq!(next.at, route.at + 1);
                                assert_eq!(pad, expected_route_pad, "a real book pad was consumed");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_decide_own_hurt_missing_history_retains_original_feedback() {
        let context = sc21_hurt_context(crate::era::Revision::Sf1Plus, true);
        for missing in [false, true] {
            for delayed_hurt in [false, true] {
                let (mut cpu, view, before) = sc21_hurt_fixture(&context, delayed_hurt, !delayed_hurt, false);
                if missing { cpu.seen[99usize % RING] = Percept::default(); }
                else { let mut now = cpu.seen(99).unwrap(); now.fighting = false; cpu.observe(now); }
                assert_eq!(cpu.felt(100, view), view);
                let run = cpu.game;
                cpu.decide(100, view, Some(before), &context);
                assert_eq!(cpu.game, if delayed_hurt { None } else { run }, "fallback preserves the available delayed feedback");
            }
        }
    }

    #[test]
    fn sc21_decide_own_hurt_change_preserves_off_and_later_state_and_rng() {
        let baseline = sc21_hurt_context(crate::era::Revision::Sf1Plus, true);
        let mut off = baseline; off.stronger = false;
        let mut later = sf6_ctx(Level::VeryHard); later.pure_footsies = true;
        let later = later.for_stronger();
        for context in [off, later] {
            assert!(!context.stronger || !context.baseline_rules());
            for delayed_hurt in [false, true] {
                let (mut a, view, before) = sc21_hurt_fixture(&baseline, delayed_hurt, !delayed_hurt, false);
                // Keep an actual queued sequence noninterruptible, avoiding
                // unrelated later-era own-readiness selection differences.
                a.plan = Plan::of(PlanKind::Act, &[0; 8]); a.plan.at = 1;
                a.bait_until = 160;
                let mut b = a;
                let mut now = b.seen(99).unwrap(); now.me = view.me; b.observe(now);
                let answer_a = a.decide(100, view, Some(before), &context);
                let answer_b = b.decide(100, view, Some(before), &context);
                assert_eq!(answer_a, answer_b);
                a.seen = b.seen;
                assert_eq!(a, b, "OFF/later cleanup, queued pad and full RNG/state keep the delayed-own semantics");
            }
        }
    }
    fn sc21_guard_facing_context(revision: crate::era::Revision, configured: bool) -> Context {
        let mut context = sc21_hit_context(TournamentFighter::Mike, revision, Level::Hard);
        context.stronger = configured;
        context.throws = false;
        context
    }

    fn sc21_guard_facing_cpu(seed: u64, forced: bool, view: Percept, latest: Option<(bool, Body, Body)>, low: bool, min: u16) -> FairCpu {
        let mut cpu = FairCpu::seeded(seed).force_stronger(forced);
        for frame in 1..=98 { cpu.observe(Percept { frame, ..view }); }
        if let Some((fighting, me, foe)) = latest {
            cpu.observe(Percept { frame: 99, fighting, me, foe, ..view });
        }
        cpu.plan = Plan::guard(low, min);
        cpu
    }

    fn sc21_guard_facing_hash(cpu: &FairCpu) -> u64 {
        use crate::canonical::{Canonical, CanonicalHasher};
        let mut hash = CanonicalHasher::new();
        cpu.encode(&mut hash);
        hash.finish()
    }

    #[test]
    fn sc21_baseline_guard_uses_latest_own_native_facing_for_high_and_low_pads() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                let context = sc21_guard_facing_context(revision, configured);
                for left in [false, true] {
                    for low in [false, true] {
                        let mut view = percept(85, body(if left { 400 } else { 200 }));
                        view.me.facing_left = !left;
                        let me = Body { facing_left: left, ..view.me };
                        // The younger opponent is deliberately on the other
                        // side. Only the native own facing may set back.
                        let foe = body(if left { 180 } else { 420 });
                        let mut cpu = sc21_guard_facing_cpu(17, forced, view, Some((true, me, foe)), low, 20);
                        let expected = (if left { RIGHT } else { LEFT }) | if low { DOWN } else { 0 };
                        assert_eq!(cpu.next_pad(100, view, &context), expected, "{revision:?}/{configured}/{forced}/{left}/{low}: back follows the last completed own native frame");
                        assert!(matches!(cpu.plan.kind, PlanKind::Guard { held: 1, .. }));
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_guard_facing_positive_control_preserves_matching_native_back() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                let context = sc21_guard_facing_context(revision, configured);
                for left in [false, true] {
                    for low in [false, true] {
                        let mut view = percept(85, body(if left { 200 } else { 400 }));
                        view.me.facing_left = left;
                        let mut cpu = sc21_guard_facing_cpu(17, forced, view, Some((true, view.me, view.foe)), low, 20);
                        let rng = cpu.rng.0;
                        assert_eq!(cpu.next_pad(100, view, &context), (if left { RIGHT } else { LEFT }) | if low { DOWN } else { 0 });
                        assert_eq!(cpu.rng.0, rng, "a no-throw guard heading draws no randomness");
                        assert!(matches!(cpu.plan.kind, PlanKind::Guard { low: chosen, min: 20, clear: 1, held: 1 } if chosen == low));
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_baseline_guard_ignores_younger_foe_and_falls_back_to_as_seen_own_facing() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                let context = sc21_guard_facing_context(revision, configured);
                for left in [false, true] {
                    let mut view = percept(85, body(if left { 400 } else { 200 }));
                    view.me.facing_left = left;
                    let expected = (if left { RIGHT } else { LEFT }) | DOWN;
                    for latest in [None, Some((false, Body { facing_left: !left, ..view.me }, body(300)))] {
                        let mut cpu = sc21_guard_facing_cpu(17, forced, view, latest, true, 20);
                        assert_eq!(cpu.next_pad(100, view, &context), expected, "missing/nonfighting latest history retains the as-seen OWN orientation, not pair geometry");
                    }
                    let mut near = sc21_guard_facing_cpu(17, forced, view, Some((true, view.me, body(180))), true, 20);
                    let mut far = sc21_guard_facing_cpu(17, forced, view, Some((true, view.me, body(420))), true, 20);
                    assert_eq!(near.next_pad(100, view, &context), expected);
                    assert_eq!(far.next_pad(100, view, &context), expected);
                    assert_eq!(near.plan, far.plan);
                    assert_eq!(near.rng.0, far.rng.0, "younger foe changes neither guard timing nor randomness");
                }
            }
        }
    }

    #[test]
    fn sc21_guard_facing_keeps_off_and_later_geometric_pads_and_full_state() {
        let free = EraControl { free: true, ..Default::default() };
        let mut contexts = vec![ctx(Level::Hard), sfz_ctx(Level::Hard, crate::era::Loadout::AIsm, free), sf3_ctx(Level::Hard), sf4_ctx(Level::Hard, free), sf5_ctx(Level::Hard, free), sf6_ctx(Level::Hard)];
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            contexts.push(sc21_guard_facing_context(revision, false));
        }
        for context in contexts {
            for low in [false, true] {
                for foe_x in [200, 400] {
                    let view = percept(85, body(foe_x));
                    let mut a = sc21_guard_facing_cpu(17, false, view, Some((true, Body { facing_left: false, ..view.me }, view.foe)), low, 20);
                    let mut b = a;
                    b.seen[99 % RING].me.facing_left = true;
                    let expected = (if foe_x > view.me.x { LEFT } else { RIGHT }) | if low { DOWN } else { 0 };
                    assert_eq!(a.next_pad(100, view, &context), expected);
                    assert_eq!(b.next_pad(100, view, &context), expected);
                    assert_eq!(a.rng.0, b.rng.0);
                    b.seen[99 % RING].me.facing_left = a.seen[99 % RING].me.facing_left;
                    assert_eq!(sc21_guard_facing_hash(&a), sc21_guard_facing_hash(&b), "OFF/later own-facing perturbation changes no guard state or RNG");
                }
            }
        }
    }

    #[test]
    fn sc21_guard_facing_change_preserves_release_bait_and_blockstun_timing() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for (configured, forced) in [(true, false), (false, true)] {
                let context = sc21_guard_facing_context(revision, configured);
                let view = percept(85, body(400));
                for (blockstun, bait, min) in [(false, false, 0), (false, false, 20), (true, false, 20), (false, true, 20)] {
                    let mut c = context;
                    c.own_blockstun = blockstun;
                    let mut a = sc21_guard_facing_cpu(17, forced, view, Some((true, Body { facing_left: true, ..view.me }, view.foe)), true, min);
                    a.bait_until = if bait { 120 } else { 0 };
                    a.settle = 44;
                    let mut b = a;
                    b.seen[99 % RING].me.facing_left = false;
                    for step in 100..=107 {
                        // Replenish the latest own frame while retaining the
                        // exact same delayed foe and chosen guard state.
                        a.observe(Percept { frame: step - 1, me: Body { facing_left: true, ..view.me }, ..view });
                        b.observe(Percept { frame: step - 1, me: Body { facing_left: false, ..view.me }, ..view });
                        let pa = a.next_pad(step, view, &c);
                        let pb = b.next_pad(step, view, &c);
                        assert_eq!(pa & !3, pb & !3);
                        assert_eq!(a.plan, b.plan, "heading alone changes no held/clear/min/release state");
                        assert_eq!(a.rng.0, b.rng.0);
                        assert_eq!(a.bait_until, b.bait_until);
                        assert_eq!(a.settle, b.settle);
                        for (observed, reference) in b.seen.iter_mut().zip(a.seen.iter()) { observed.me.facing_left = reference.me.facing_left; }
                        assert_eq!(sc21_guard_facing_hash(&a), sc21_guard_facing_hash(&b));
                        if !blockstun && !bait {
                            let release_at = if min == 0 { 104 } else { 107 };
                            assert_eq!(a.plan.done(), step >= release_at, "the existing five-clear/min or eight-held early-release boundary is unchanged");
                        }
                        if blockstun || bait { assert!(matches!(a.plan.kind, PlanKind::Guard { .. }), "native blockstun/chosen bait retain the initial hold"); }
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_guard_facing_preserves_sf1plus_tech_pulse_cadence_and_randomness() {
        for (configured, forced) in [(true, false), (false, true)] {
            let mut context = sc21_guard_facing_context(crate::era::Revision::Sf1Plus, configured);
            context.throws = true;
            let view = percept(85, body(330));
            let mut pulses = 0;
            for seed in 1..=64 {
                let mut a = sc21_guard_facing_cpu(seed, forced, view, Some((true, Body { facing_left: true, ..view.me }, view.foe)), true, 20);
                let mut b = a;
                for step in 100..=108 {
                    a.observe(Percept { frame: step - 1, me: Body { facing_left: true, ..view.me }, ..view });
                    b.observe(Percept { frame: step - 1, me: Body { facing_left: false, ..view.me }, ..view });
                    let pa = a.next_pad(step, view, &context);
                    let pb = b.next_pad(step, view, &context);
                    assert_eq!(pa & !3, pb & !3);
                    assert_eq!(a.plan, b.plan);
                    assert_eq!(a.rng.0, b.rng.0, "only existing eligible pulse frames draw their guard chance");
                    if pa & BUTTONS != 0 {
                        pulses += 1;
                        assert_eq!(pa & BUTTONS, button(2));
                        assert_eq!((step - 99) % 3, 0, "the existing every-third-held-frame tech pulse remains exact");
                    }
                }
            }
            assert!(pulses > 0, "the real SF1+ throw-tech branch must actually be exercised");
        }
    }

    fn sc21_guarded_jump_location(context: &Context) -> (u8, u8) {
        let table = game::plans(context.kit.fighter, context.family);
        let (plan, step) = table.iter().enumerate()
            .find_map(|(plan, entry)| entry.steps.iter()
                .position(|step| matches!(step.act, game::Act::JumpIn { .. }))
                .map(|step| (plan, step)))
            .expect("Mike has an authored bounded jump action");
        assert!(step > 0);
        assert!(matches!(table[plan].steps[step - 1].act, game::Act::Space(120)));
        (plan as u8, step as u8)
    }

    fn sc21_guarded_jump_fixture(
        revision: crate::era::Revision,
        gap: i16,
        crossed: bool,
        mirrored: bool,
        seed: u64,
    ) -> (FairCpu, Percept, Context) {
        let context = sc21_hit_context(TournamentFighter::Mike, revision, Level::Hard);
        let x = |value| if mirrored { 600 - value } else { value };
        let view = Percept {
            frame: 88,
            fighting: true,
            me: body(x(200)),
            foe: body(x(330)),
            ..Default::default()
        };
        let mut cpu = FairCpu::seeded(seed);
        for frame in 1..=98 {
            cpu.observe(Percept { frame, ..view });
        }
        let own_x = if crossed { 340 + gap } else { 320 - gap };
        // Poison younger opponent/projectile history; only this own body
        // belongs in felt. The delayed opponent stays at x330 (or its mirror).
        cpu.observe(Percept {
            frame: 99,
            me: body(x(own_x)),
            foe: Body { move_id: 0x35, state: 0x89, active: true, ..body(x(-250)) },
            shot: true,
            shot_x: x(own_x),
            shot_half: 20,
            ..view
        });
        let (plan, step) = sc21_guarded_jump_location(&context);
        cpu.game = Some(game::Running { plan, step, acted: false, pressed: 0, began: 80, hit: false });
        cpu.judged.situation = Situation::NeutralFar;
        cpu.plan_worked[usize::from(plan)] = 2;
        cpu.last_normal = Some(Normal { crouch: true, button: 0, pressed: 4, from: Some(x(200)), best: 40 });
        (cpu, view, context)
    }

    #[test]
    fn sc21_guarded_jump_uses_the_existing_generator_with_the_same_felt_percept() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for gap in [70, 120, 150] {
                for crossed in [false, true] {
                    for mirrored in [false, true] {
                        for seed in [17, 29, 37] {
                            let (mut actual, view, context) = sc21_guarded_jump_fixture(revision, gap, crossed, mirrored, seed);
                            let felt = actual.felt(100, view);
                            assert_eq!(felt.gap(), gap);
                            assert_eq!(felt.foe, view.foe);
                            assert_ne!(felt.shot, actual.seen(99).unwrap().shot);
                            if crossed {
                                assert_ne!(felt.toward(), view.toward(), "own crossing must exercise the new action's heading");
                            }
                            let mut expected = actual;
                            let reference = expected.plan_for(Choice::JumpIn, 100, felt, &context, false);
                            let mut run = expected.game.unwrap();
                            run.acted = true;
                            run.pressed = 100;
                            expected.game = Some(run);
                            let generated = actual.follow_game_plan(100, view, &context).expect("the inclusive jump band admits this action");
                            assert_eq!(generated, reference, "entire pad stream, kind, length and answer flag stay the existing generator's");
                            assert_eq!(generated.pads[0], UP | if felt.toward() > 0 { RIGHT } else { LEFT });
                            assert_eq!(actual, expected, "all CPU fields, RNG, settle and last-normal effects match the same-felt generator reference");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sc21_guarded_jump_rejects_outside_the_band_without_action_or_credit() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for gap in [20, 69, 151, 260] {
                for mirrored in [false, true] {
                    let (mut actual, view, context) = sc21_guarded_jump_fixture(revision, gap, false, mirrored, 17);
                    assert_eq!(view.gap(), 120, "all range-boundary cases retain this same delayed view");
                    assert_eq!(actual.felt(100, view).gap(), gap, "only the latest own body moves the admission across its boundary");
                    actual.answered_plan = Some((2, 180));
                    let mut expected = actual;
                    expected.end_game_plan();
                    assert_eq!(actual.follow_game_plan(100, view, &context), None);
                    assert_eq!(actual, expected, "only ordinary non-hit plan-end bookkeeping is allowed: no action, settle, RNG, route or answer credit");
                    assert_eq!(actual.plan_worked[usize::from(sc21_guarded_jump_location(&context).0)], 0);
                    assert_eq!(actual.last_plan, sc21_guarded_jump_location(&context).0 + 1);
                }
            }
        }
    }

    #[test]
    fn sc21_guarded_jump_checks_range_after_own_readiness() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let (mut actual, view, context) = sc21_guarded_jump_fixture(revision, 260, false, false, 17);
            let latest = actual.seen(99).unwrap();
            actual.observe(Percept { me: Body { move_id: 0x35, state: 0x89, ..latest.me }, ..latest });
            let before = actual;
            let waiting = actual.follow_game_plan(100, view, &context).expect("recover before deciding whether to discard the jump");
            assert_eq!(waiting, Plan::of(PlanKind::Idle, &[0]));
            assert_eq!(actual, before, "a busy own body retains the unacted game step without draws or bookkeeping");
            actual.observe(latest);
            let mut expected = actual;
            expected.end_game_plan();
            assert_eq!(actual.follow_game_plan(100, view, &context), None);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn sc21_guarded_jump_space_timeout_cannot_launch_from_a_far_gap() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for mirrored in [false, true] {
                let (mut actual, view, context) = sc21_guarded_jump_fixture(revision, 260, false, mirrored, 17);
                let (plan, jump) = sc21_guarded_jump_location(&context);
                actual.game = Some(game::Running { plan, step: jump - 1, acted: true, pressed: 10, began: 10, hit: false });
                let rng = actual.rng;
                let walking = actual.follow_game_plan(100, view, &context).expect("at exactly90 frames Space still returns its bounded walk");
                assert_eq!(walking.kind, PlanKind::Walk);
                assert_eq!(walking.len, game::EDGE_WALK as u8);
                assert_eq!(actual.game.unwrap().step, jump - 1);
                assert_eq!(actual.rng, rng);
                actual.plan = walking;
                let latest = actual.seen(99).unwrap();
                for step in 100..108 {
                    let pad = actual.next_pad(step, view, &context);
                    assert_eq!(pad, if mirrored { LEFT } else { RIGHT });
                    actual.observe(Percept { frame: step, ..latest });
                }
                assert!(actual.plan.done());
                let delayed = actual.seen(96).unwrap();
                assert_eq!(actual.felt(108, delayed).gap(), 260);
                let mut expected = actual;
                expected.end_game_plan();
                assert_eq!(actual.follow_game_plan(108, delayed, &context), None, "Space's deadline may advance, but the following action rejects the far gap");
                assert_eq!(actual, expected);
                assert_eq!(actual.rng, rng);
            }
        }
    }

    #[test]
    fn sc21_guarded_jump_space_corner_exit_rejects_close_and_arrival_can_jump() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for mirrored in [false, true] {
                let (mut close, view, context) = sc21_guarded_jump_fixture(revision, 20, false, mirrored, 17);
                close.game.as_mut().unwrap().step = sc21_guarded_jump_location(&context).1 - 1;
                close.judged.foe_cornered = true;
                let mut expected = close;
                expected.end_game_plan();
                assert_eq!(close.follow_game_plan(100, view, &context), None, "Space must not back out of the corner, nor jump below its band");
                assert_eq!(close, expected);

                let (mut arrived, view, context) = sc21_guarded_jump_fixture(revision, 120, false, mirrored, 17);
                arrived.game.as_mut().unwrap().step = sc21_guarded_jump_location(&context).1 - 1;
                let mut reference = arrived;
                let expected = reference.plan_for(Choice::JumpIn, 100, reference.felt(100, view), &context, false);
                let mut run = reference.game.unwrap();
                run.step = sc21_guarded_jump_location(&context).1;
                run.acted = true;
                run.pressed = 100;
                reference.game = Some(run);
                assert_eq!(arrived.follow_game_plan(100, view, &context), Some(expected));
                assert_eq!(arrived, reference, "reaching Space's target enters the same-felt jump in the same interpreter call");
            }
        }
    }

    #[test]
    fn sc21_guarded_jump_uses_only_latest_own_body_and_falls_back_without_it() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            for mirrored in [false, true] {
                let (base, view, context) = sc21_guarded_jump_fixture(revision, 120, true, mirrored, 17);
                let latest = base.seen(99).unwrap();
                let mut near = base;
                let mut far = base;
                near.observe(Percept { foe: body(latest.me.x + 11), shot: false, ..latest });
                far.observe(Percept { foe: Body { active: true, move_id: 0x10, state: 0x10, ..body(latest.me.x - 1000) }, ..latest });
                assert_eq!(near.follow_game_plan(100, view, &context), far.follow_game_plan(100, view, &context), "younger foe place, attack and projectile do not alter admission or jump pads");
                assert_eq!(near.rng, far.rng);
                assert_eq!(near.game, far.game);

                for latest_fighting in [false, true] {
                    let mut fallback = base;
                    if latest_fighting {
                        fallback.seen[99 % RING] = Percept::default();
                    } else {
                        fallback.observe(Percept { fighting: false, ..latest });
                    }
                    let mut reference = fallback;
                    let expected = reference.plan_for(Choice::JumpIn, 100, view, &context, false);
                    let mut run = reference.game.unwrap();
                    run.acted = true;
                    run.pressed = 100;
                    reference.game = Some(run);
                    assert_eq!(fallback.follow_game_plan(100, view, &context), Some(expected));
                    assert_eq!(fallback, reference, "missing or nonfighting own observation retains the original delayed Percept");
                }
            }
        }
    }

    #[test]
    fn sc21_guarded_jump_keeps_commit_focus_and_plain_action_permissions() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let (base, view, context) = sc21_guarded_jump_fixture(revision, 120, false, false, 17);
            let (index, step) = sc21_guarded_jump_location(&context);
            let plan = &game::plans(context.kit.fighter, context.family)[usize::from(index)];
            let jump = &plan.steps[usize::from(step)];
            assert!(jump.act.commits());
            assert!(!jump.act.meets_the_rise());
            assert!(!jump.act.needs_special());
            assert_eq!(focus::step_focus(jump), Focus::Attack);
            assert!(plan.playable(false, |_| false, |_| false, |_| false));
            assert!(plan.keeps_to_normals(|_| false));
            for incoming in ["attack", "projectile", "knockdown"] {
                let mut actual = base;
                let mut threat = view;
                match incoming {
                    "attack" => threat.foe = Body { move_id: 0x10, state: 0x10, active: true, ..view.foe },
                    "projectile" => {
                        threat.shot = true;
                        threat.shot_x = view.me.x + 20;
                        assert!(threat.shot_coming());
                    }
                    _ => actual.judged.situation = Situation::OpponentKnockedDown,
                }
                let mut expected = actual;
                expected.end_game_plan();
                assert_eq!(actual.follow_game_plan(100, threat, &context), None, "new jump obeys the existing committing-action abort checks");
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn sc21_guarded_jump_completion_does_not_invent_a_hit_from_an_old_reaction() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let (mut actual, view, context) = sc21_guarded_jump_fixture(revision, 120, false, false, 17);
            let pads = actual.follow_game_plan(100, view, &context).unwrap();
            actual.plan = pads;
            for step in 100..100 + u32::from(pads.len) {
                assert_eq!(actual.next_pad(step, view, &context), pads.pads[(step - 100) as usize]);
            }
            assert!(actual.plan.done());
            let reaction = Percept { frame: 132, foe: Body { move_id: 0x35, state: 0x89, health: 44, ..view.foe }, ..view };
            let mut expected = actual;
            expected.end_game_plan();
            assert_eq!(actual.follow_game_plan(144, reaction, &context), None);
            assert_eq!(actual, expected, "jump completion keeps legacy Nothing semantics, even if a delayed reaction is present");
            assert_eq!(actual.plan_worked[usize::from(sc21_guarded_jump_location(&context).0)], 0);
            assert!(actual.answered_plan.is_none());
        }
    }

    #[test]
    fn sc21_guarded_jump_leaves_existing_do_jump_in_off_and_later_paths_exact() {
        let mut contexts = Vec::new();
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let mut off = sc21_hit_context(TournamentFighter::Retsu, revision, Level::Hard);
            off.stronger = false;
            contexts.push(off);
        }
        let mut later = ctx(Level::Hard);
        later.kit = Kit::of(TournamentFighter::Retsu, RulesetPreset::DefinitiveOverhaul, None);
        later.family = Some(crate::era::EraFamily::Sf2);
        later.stronger = true;
        contexts.push(later);
        for context in contexts {
            for mirrored in [false, true] {
                let (mut actual, view, _) = sc21_guarded_jump_fixture(crate::era::Revision::Sf1Plus, 260, true, mirrored, 17);
                let (index, step) = game::plans(context.kit.fighter, context.family).iter().enumerate()
                    .find_map(|(index, entry)| entry.steps.iter().position(|step| step.act == game::Act::Do(Choice::JumpIn)).map(|step| (index, step)))
                    .expect("this unchanged table has a legacy Do jump-in");
                actual.game = Some(game::Running { plan: index as u8, step: step as u8, acted: false, pressed: 0, began: 80, hit: false });
                let mut expected = actual;
                let reference = expected.plan_for(Choice::JumpIn, 100, view, &context, false);
                let mut run = expected.game.unwrap();
                run.acted = true;
                run.pressed = 100;
                expected.game = Some(run);
                assert_eq!(actual.follow_game_plan(100, view, &context), Some(reference));
                assert_eq!(actual, expected, "old Do keeps delayed geometry, full pad stream and all CPU/RNG state");
            }
        }
    }


    #[test]
    fn sc21_guarded_jump_appends_long_approach_without_replacing_the_direct_jump() {
        for revision in [crate::era::Revision::Arcade1987, crate::era::Revision::Sf1Plus] {
            let (mut direct_cpu, view, context) = sc21_guarded_jump_fixture(revision, 260, true, false, 17);
            let table = game::plans(context.kit.fighter, context.family);
            let (long_index, _) = sc21_guarded_jump_location(&context);
            assert_eq!(table.len(), 12);
            assert_eq!(usize::from(long_index), table.len() - 1, "the new intent is appended after all original indices");
            let direct = &table[9];
            assert_eq!((direct.score, direct.gap, direct.corner_bonus, direct.cornered_only), (54, (70, 150), 0, false));
            assert_eq!(direct.steps.len(), 1);
            assert_eq!(direct.steps[0].act, game::Act::Do(Choice::JumpIn));
            assert_eq!(direct.steps[0].then, [None; 5]);
            assert_eq!(direct.lean, habit::Lean::JumpIn);
            assert_eq!(direct.from, Level::Easy);
            direct_cpu.game = Some(game::Running { plan: 9, step: 0, acted: false, pressed: 0, began: 80, hit: false });
            let mut reference = direct_cpu;
            let expected = reference.plan_for(Choice::JumpIn, 100, view, &context, false);
            let mut run = reference.game.unwrap();
            run.acted = true;
            run.pressed = 100;
            reference.game = Some(run);
            assert_eq!(direct_cpu.follow_game_plan(100, view, &context), Some(expected));
            assert_eq!(direct_cpu, reference, "direct9 retains old delayed geometry even with felt gap260");

            for gap in [150, 151, 260, 261] {
                let mut picked_direct = false;
                let mut picked_long = false;
                let mut picked_any = false;
                // A fixed bounded set covers actual utility admission without
                // depending on one particular noise/quality winner.
                for seed in 1..=64 {
                    let (mut cpu, _, context) = sc21_guarded_jump_fixture(revision, 120, false, false, seed);
                    let view = Percept { frame: 88, fighting: true, me: body(200), foe: body(210 + gap), ..Default::default() };
                    for frame in 1..=99 { cpu.observe(Percept { frame, ..view }); }
                    cpu.game = None;
                    cpu.plan_worked = [0; game::PLAN_SLOTS];
                    cpu.last_plan = 0;
                    if cpu.start_game_plan(100, view, &context) {
                        picked_any = true;
                        let index = cpu.game.unwrap().plan;
                        if index == 9 {
                            assert_eq!(gap, 150, "only the direct band reaches this tested endpoint");
                            picked_direct = true;
                        }
                        if index == long_index {
                            assert!((151..=260).contains(&gap));
                            picked_long = true;
                        }
                    }
                }
                assert!(picked_any, "the real selector must produce a plan in this bounded set");
                assert_eq!(picked_direct, gap == 150);
                assert_eq!(picked_long, (151..=260).contains(&gap));
            }
        }
    }

}
