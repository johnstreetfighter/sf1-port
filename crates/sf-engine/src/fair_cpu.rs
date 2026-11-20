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

mod era;
mod focus;
mod game;
pub(crate) use era::EraControl;
pub use focus::Focus;
pub use game::{Judged, Situation};
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

    /// Percent chance a special's motion is entered cleanly (execution).
    pub const fn execution(self) -> u32 {
        match self {
            Level::Easy => 70,
            Level::Normal => 85,
            Level::Hard => 93,
            Level::VeryHard => 97,
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

crate::canonical::canonical_struct!(FairCpu, "FairCpu.v6", {
    seen, rng, seeded, plan, settle, reach, reach_seen, last_normal, shot_range, shot_far, fighter, pad,
    forced_stronger, judge, judged, game, route, recent_routes, route_outcome, route_drops, route_check,
    focus, action_frame, last_view,
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
pub(crate) fn perceive_fighters(
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
        }
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

    /// Play the stronger game whatever the match's switch says.
    pub(crate) fn force_stronger(mut self, on: bool) -> Self {
        self.forced_stronger = on;
        self
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

    /// Whether it is in the middle of a game plan.
    pub fn following_game_plan(&self) -> bool {
        self.game.is_some()
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
        }
        let stronger = self.stronger(ctx);
        let delay = ctx.level.reaction();
        let view = step.checked_sub(delay).and_then(|f| self.shown(f, step, ctx.level, stronger));
        let pad = match view {
            Some((view, action_frame)) if in_fight && view.fighting => {
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
                }
                pad
            }
            _ => {
                self.plan = Plan::default();
                self.game = None;
                self.route = None;
                self.route_outcome = None;
                self.judge = game::Judge::default();
                self.judged = Judged::default();
                self.focus = Focus::default();
                self.action_frame = 0;
                self.last_view = Percept::default();
                0
            }
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
        self.learn(view, before);
        let stronger = self.stronger(ctx);
        // Whether its pads come from a game plan, before being hit ends it.
        let in_plan = self.game.is_some();
        if stronger {
            self.judged = self.judge.judge(view, before, self.longest_reach());
            if view.me.reacting() {
                // Being hit ends whatever it was following.
                self.game = None;
            }
        }
        if view.me.reacting() {
            self.route = None;
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
                self.route_check = None;
            } else {
                self.route_check = Some(RouteCheck { index, at, hit });
            }
        }
        if self.route.is_some() {
            // A route is entered to its end, or to where it is cut.
            return self.next_route_pad(step, view, stronger);
        }
        if let Some(before) = before {
            let interruptible = self.plan.interruptible();
            // The stronger play leaves a knocked-down opponent to its
            // knockdown plans: the era kit's answer to a hit (a super on
            // the hit confirm) would only whiff over him.
            let foe_down = stronger && self.judged.situation == Situation::OpponentKnockedDown;
            // A ground attack he starts out of its reach is left to whiff,
            // not guarded or parried: the CPU stays free to move and to
            // punish it.
            let baiting = stronger && self.out_of_reach(step, view);
            let spend_super = self.spends_super(ctx, view);
            let era_response = ctx.era.filter(|tactics| !foe_down && !baiting && (interruptible || tactics.can_interrupt()))
                .and_then(|tactics| tactics.react(&mut self.rng, ctx.level, view, before, stronger, spend_super));
            let mut answered = false;
            if let Some(mut plan) = era_response {
                if stronger {
                    plan.end_at_last_press();
                    plan.answer = true;
                }
                self.plan = plan;
                self.game = None;
                answered = true;
            } else if interruptible {
                let answer = self.react(step, view, before, ctx).filter(|plan| !(baiting && matches!(plan.kind, PlanKind::Guard { .. })));
                if let Some(plan) = answer {
                    // A threat it answers ends the game plan it was following.
                    self.plan = plan;
                    self.game = None;
                    answered = true;
                }
            }
            // A game plan's attack not yet begun is not pressed into an
            // attack it has just seen start: it guards instead. (Not an
            // answer it has just chosen to that attack.)
            let foe_started = view.foe.attacking() && (!before.foe.attacking() || before.foe.move_id != view.foe.move_id);
            if stronger && !answered && (in_plan || self.game.is_some()) && self.plan.press_to_come() && foe_started {
                self.plan = Plan::guard(view.foe.low_attack(), 0);
                self.game = None;
            }
        }
        if self.route.is_some() {
            // An answer that is a route: a punish.
            return self.next_route_pad(step, view, stronger);
        }
        if self.plan.done() {
            if let Some(plan) = self.follow_game_plan(step, view, ctx) {
                self.plan = plan;
                if self.route.is_some() {
                    // A step that is a route: its first pad is this frame's.
                    return self.next_route_pad(step, view, stronger);
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
            } else {
                let tactics = ctx.era.map(|tactics| if stronger { tactics.spending_safely() } else { tactics });
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
            return self.next_route_pad(step, view, stronger);
        }
        self.next_pad(step, view, ctx)
    }

    /// The next pad of the route being entered, turned to face the
    /// opponent as it faced him when the route began. What came of it is
    /// kept for the game plan that chose it.
    fn next_route_pad(&mut self, step: u32, view: Percept, stronger: bool) -> u16 {
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
                    self.route_check = Some(RouteCheck { index: run.index, at: step + game::ROUTE_WATCH, hit: false });
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
        let moment = game::Moment {
            reaches,
            era: ctx.era.is_some(),
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
            drive_reserve: if self.stronger(ctx) { drive_reserve(ctx.level) } else { 0 },
            avoid_burnout: self.stronger(ctx),
        };
        let mut options: Vec<(usize, i32)> = Vec::new();
        for (index, route) in book.routes.iter().enumerate() {
            let drops = self.route_drops.get(index).copied().unwrap_or(0);
            if moment.allows(route) && drops < game::DROPS_KEPT {
                let recent = self.recent_routes.contains(&(index as u8 + 1));
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
        let clean = ctx.level.execution().saturating_sub(if pressure { (100 - ctx.level.execution()) / 2 } else { 0 });
        let stop = if route.steps.len() >= 2 && !self.rng.chance(clean) {
            route.step_start(1 + self.rng.below(route.steps.len() as u32 - 1) as usize)
        } else {
            route.len()
        };
        let walk = moment.walk_frames(route);
        self.recent_routes.rotate_right(1);
        self.recent_routes[0] = pick as u8 + 1;
        self.route = Some(game::RouteRun::new(fighter, revision, loadout, pick as u8, walk, stop as u16, !view.me.facing_left));
        self.route_outcome = None;
        self.last_normal = None;
        self.plan = Plan::default();
        true
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
    /// have one and the level's decision quality chooses it over the single
    /// choices. Among several the best score wins, after a little noise.
    fn start_game_plan(&mut self, step: u32, view: Percept, ctx: &Context) -> bool {
        let table = game::plans(ctx.kit.fighter, ctx.family);
        let mut best: Option<(u8, i32)> = None;
        let gap = view.gap();
        for (index, plan) in table.iter().enumerate() {
            if plan.situation == self.judged.situation
                && (!plan.cornered_only || self.judged.foe_cornered)
                && (plan.gap.0..=plan.gap.1).contains(&gap)
            {
                // A cornered opponent is pressed harder.
                let corner = if self.judged.foe_cornered { plan.corner_bonus } else { 0 };
                let score = plan.score + corner + self.rng.below(12) as i32;
                if best.is_none_or(|(_, top)| score > top) {
                    best = Some((index as u8, score));
                }
            }
        }
        let Some((plan, _)) = best else { return false };
        if !self.rng.chance(ctx.level.quality()) {
            return false;
        }
        self.game = Some(game::Running { plan, step: 0, acted: false, pressed: 0, began: step });
        true
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
            self.game = None;
            return None;
        }
        let table = game::plans(ctx.kit.fighter, ctx.family);
        let wait = Plan::of(PlanKind::Idle, &[0]);
        // A plan has few steps, and each pass either returns or moves on one.
        for _ in 0..8 {
            let Some(at) = table.get(usize::from(run.plan)).and_then(|plan| plan.steps.get(usize::from(run.step))) else { break };
            // It does not walk or attack into an attack or a projectile it
            // has seen start: the plan ends, and its answer takes over.
            let commits = !run.acted && at.act.commits();
            if commits && (view.foe.attacking() || view.shot_coming()) {
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
                    let Some(plan) = ctx.era.and_then(|tactics| tactics.rush_in(view)) else { break };
                    run.acted = true;
                    run.pressed = step;
                    self.game = Some(run);
                    self.settle = step + plan.len as u32;
                    return Some(plan);
                }
                (Act::Reach { crouch, button }, false) => {
                    let reach = self.reach[usize::from(crouch)][button] - 4;
                    let gap = self.felt(step, view).gap();
                    if gap > reach {
                        // A walk covers about two pixels a frame.
                        let frames = ((gap - reach + 1) / 2).clamp(1, 40) as usize;
                        let forward = if view.toward() > 0 { RIGHT } else { LEFT };
                        run.acted = true;
                        run.pressed = step;
                        self.game = Some(run);
                        return Some(Plan::of(PlanKind::Walk, &[forward; PLAN_CAP][..frames]));
                    }
                    Outcome::Nothing
                }
                (Act::Do(choice), false) => {
                    self.game = Some(run);
                    if !self.ready(step, view, stronger) {
                        return Some(wait);
                    }
                    run.acted = true;
                    run.pressed = step;
                    self.game = Some(run);
                    return Some(self.plan_for(choice, step, view, ctx, false));
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
                            return Some(self.close_in(run, step, view, reach));
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
                (Act::Meaty { crouch, button }, true) => match run.outcome(view, self.reach[usize::from(crouch)][button]) {
                    Some(outcome) => outcome,
                    None => return Some(wait),
                },
                (Act::GuardRise, false) => {
                    let close = self.reach[0][1] - 4;
                    let rise = self.judge.rise_at();
                    if let Some(rise) = rise {
                        if step + game::GUARD_LEAD < rise {
                            return Some(self.close_in(run, step, view, close));
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
                    match run.outcome(view, self.reach[usize::from(crouch)][button]) {
                        Some(outcome) => outcome,
                        None => return Some(wait),
                    }
                }
                (_, true) => Outcome::Nothing,
            };
            match at.next(outcome) {
                Some(next) => {
                    run.step = next;
                    run.acted = false;
                }
                None => break,
            }
        }
        self.game = None;
        None
    }

    /// A frame of a timed step's walk in toward a knocked-down opponent, or
    /// of waiting once within `reach` of him; the plan is kept with it.
    fn close_in(&mut self, run: game::Running, step: u32, view: Percept, reach: i16) -> Plan {
        // Its own place is felt as it is now; his is seen late.
        let walk = self.ready(step, view, true) && self.felt(step, view).gap() > reach;
        self.game = Some(run);
        if walk {
            Plan::of(PlanKind::Walk, &[if view.toward() > 0 { RIGHT } else { LEFT }])
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
    fn learn(&mut self, view: Percept, before: Option<Percept>) {
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
        // Where the attack started: its first frame seen, measured from
        // there as the stance's centre (a pose's own offset and a lunge's
        // steps come after).
        let from = match from {
            Some(x) => x,
            None if before.is_some_and(|b| !b.me.attacking()) => view.me.x,
            None => {
                // Started unseen (the view jumped): measure nothing.
                self.last_normal = None;
                return;
            }
        };
        let best = if view.me.active { best.max(((view.me.front - from) * view.toward()).clamp(16, 120)) } else { best };
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
        if self.stronger(ctx) && self.judge.coming(view) && self.walking_in_next(view) {
            // It does not walk into an attack or a projectile it has seen:
            // the walk (or the walk before an attack) is dropped, and what
            // it does instead is chosen on the next frame.
            self.plan = Plan::default();
            self.settle = 0;
            return 0;
        }
        let back = if view.toward() > 0 { LEFT } else { RIGHT };
        if let PlanKind::Await { choice, wait } = self.plan.kind {
            // The stronger play lets a jump it sees carry him away go by
            // (SC8, `#200`): there is nothing there to meet.
            let away = self.stronger(ctx)
                && self.seen(view.frame - 1).is_some_and(|was| (view.foe.x - was.foe.x) * view.toward() > 0);
            if !view.foe.jumping() || away {
                self.plan = Plan::default();
                return 0;
            }
            if wait > 0 {
                self.plan.kind = PlanKind::Await { choice, wait: wait - 1 };
                return 0;
            }
            self.plan = self.plan_for(choice, step, view, ctx, true);
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
            let throw_threat = ctx.throws
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
            if (clear >= 5 && held >= min) || held > 120 {
                self.plan = Plan::default();
                return 0;
            }
            // Delayed throw tech (research 4.3, SP22): while guarding against
            // an opponent who has walked into throw range, pulse `Back + HP`
            // on a short delay so a grab is teched inside its 7-frame window
            // (and an opponent standing in throw range is thrown).
            if ctx.throws
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
        // A jump toward the CPU, from its start (the pre-jump crouch shows).
        if matches!(foe.move_id, 9..=0xb) && !was.jumping() && dist < 200 {
            if !self.rng.chance(level.guard()) {
                return None;
            }
            let mut options = vec![(Choice::Guard, 55), (Choice::Retreat, 12)];
            if ctx.kit.anti_air.is_some() {
                options.push((Choice::Special(Role::AntiAir), 62 * level.specials() / 100));
            }
            if dist < 130 {
                // The crouching heavy punch rises sooner than the standing one.
                options.push((Choice::Normal { crouch: true, button: 2 }, 38));
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
            let wait = 32u32.saturating_sub(lead + level.reaction()) as u16;
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
        // A move that missed: its attack box has gone and it is recovering,
        // within reach of a heavy normal.
        // Not a move the CPU guarded: one hit a move, so a guarded move's box
        // goes too, and under true combos (SP17) the guard is then locked in
        // blockstun, which the CPU cannot see the end of.
        let whiffed = foe.attacking() && !foe.active && was.active && !foe.jumping() && !view.me.guarding() && !before.me.guarding();
        let stronger = self.stronger(ctx);
        // The stronger play knows a whiff by its attack box never reaching
        // the CPU, so a move it guarded in case can be a whiff too.
        let whiff_seen = stronger
            && self.judged.situation == Situation::OpponentWhiffed
            && (view.me.free() || view.me.guarding())
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
        if whiffed && view.me.free() && self.settled(step, view, stronger) {
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
        let threatened = view.foe.attacking();
        // The CPU attacks only into an opening it can use. An attack still
        // coming is guarded, not answered with one of its own (under true
        // combos, SP17, a guard locked in blockstun holds through the string
        // this way). A hit reaction is no opening: it is seen `reaction`
        // frames late, so whatever hitstun it shows (true combos make it
        // hittable) has mostly gone by the time an attack would land, and
        // under the classic rules the reaction cannot be hit at all. Combos
        // as tactics are SP22's.
        let open = !threatened && !foe_busy;
        let mut options: Vec<(Choice, i32)> = vec![(Choice::Wait, level.wait())];
        let stronger = self.stronger(ctx);
        if stronger && self.judge.coming(view) {
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
            options.push((Choice::Throw, 44 + close + vs_guard));
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
                    _ if gap > 90 => options.push((Choice::Special(Role::Projectile), (40 + if gap > 150 { 18 } else { 0 }) * specials / 100)),
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
                Plan::of(PlanKind::Act, &[stick, press, press, press, stick, 0])
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
                let clean = level
                    .execution()
                    .saturating_sub(if pressure { (100 - level.execution()) / 2 } else { 0 });
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
                let frames = self.motion(&entry, !view.me.facing_left, view, ctx);
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
        let clean = ctx.level.execution().saturating_sub(if pressure { (100 - ctx.level.execution()) / 2 } else { 0 });
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
            cpu.learn(p, cpu.seen(f - 1));
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
        use crate::era::{GaugeMode, Loadout, Revision, StylePick, STOCK_AT};
        let config = crate::config::MatchConfig::definitive();
        let mut state = EraFighterState::default();
        state.fighter = Some(TournamentFighter::Ryu);
        state.style = StylePick::for_revision(Revision::Sf6, Loadout::Standard);
        state.max_sub_hp = crate::era_fight::FULL_SUB_HP;
        state.set_gauge_mode(GaugeMode::EraDefault);
        state.set_sf6_practice_drive(None);
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
}
