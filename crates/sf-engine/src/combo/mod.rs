//! True combos (spec #246, ticket #261, `SP17`): the modern rule that makes
//! a fighter in a hit reaction hittable, with hitstun and blockstun taken
//! from the attacking move's data instead of 1987's whole-reaction
//! invulnerability (`docs/design/2026-11-07-sf1plus-on-the-real-engine-design.md`
//! 5.2, ADR-0014).
//!
//! **The switch.** [`CoreToggles::true_combos`](crate::config::CoreToggles)
//! (on in Definitive Overhaul, off in Classic Competitive and Pure Footsies,
//! never on in Faithful 1987). Every gate that reads it is on the translated
//! path in [`crate::fighter`]; with it off the fight is the 1987 fight byte
//! for byte, and the state here stays at its default.
//!
//! **What 1987 does.** A landed hit starts a reaction stream from the
//! defender's hit table (`0x4451a`). Every reaction frame carries the
//! invulnerable flag (`+0xf` bit 3) and a zero-size hurtbox
//! (`docs/reference/rom-box-audit.md`, "Hit reactions and knock-outs"), so
//! nothing can hit him until the stream ends, and the stream's own length is
//! the hitstun: Ryu's and Ken's standing reactions 26 or 44 frames, their
//! crouching ones 9, the bosses' 6 to 33. A block costs nothing: the guard
//! (moves 4 and 8) passes the busy gate (`0x403d4`), so a blocker can act on
//! any frame.
//!
//! **What the switch changes** (each through a gate in `fighter.rs`):
//!
//! - A hit that puts the defender in a **ground** reaction (the reaction
//!   table entry's state `& 7` is not 1: standing `0x8a`, crouching `0x8c`)
//!   while he is still alive starts a *combo-able reaction*: its frames lose
//!   the invulnerable flag and take real hurtboxes ([`reaction_boxes`]),
//!   and it lasts [`Stun::hit`] frames of the attacking move, cut short or
//!   held on its last frame to fit. Airborne reactions (the knock-backs of
//!   the 1987 heavies and specials, and any hit on a jumping fighter), the
//!   knock-outs, and the chip reaction a guarded projectile or Tatsumaki
//!   gives stay as 1987 has them: no juggles, combos on the ground (my
//!   call, listed for the maintainer).
//! - A guarded melee hit locks a player-driven guard for [`Stun::block`]
//!   frames (the 1987 CPU's guards are its own timed streams and keep them).
//!   During blockstun the guard can switch between high and low, nothing
//!   else; inputs pressed then go to the recovery input buffer as in any
//!   other lock.
//! - One hit per move: once a move connects (hit or guarded), its later
//!   active frames no longer hit until a new move starts. In 1987 the
//!   reaction's invulnerability already stopped a second hit; without it a
//!   move whose active frames span several records would hit on each.
//! - Damage in a combo scales by [`scaled`] (SF6's general schedule).
//!
//! **Hitstun and blockstun live in move data** ([`move_stun`]): one row per
//! move id of each fighter's player table, Ryu's, Ken's and Retsu's tuned
//! here, Geki's, Joe's and Mike's; any other fighter, and the 1987 CPU's
//! attack streams, take the strength class's research medians until his own
//! ticket tunes his rows.
//! Projectiles take [`PROJECTILE`].
//!
//! **Frames.** A stun of `n` frames set on frame `h` (the hit) ends at the
//! start of frame `h + n`: the countdown runs once a fight frame before the
//! input handlers ([`crate::fighter::Bus::combo_frame`]), so the defender's
//! input handler can act on frame `h + n` whichever fighter's objects the
//! dispatcher runs first. An attacker whose move hits on its tick `s` of
//! `t` acts again on frame `h + (t - s) + 2` (the stream's end is processed
//! on tick `t + 1`, and the input handler starts the next move on the frame
//! after), so the advantage on hit is `n - (t - s) - 2`.
//!
//! **Determinism.** Everything here is in [`ComboState`], which rides in
//! [`crate::config::CoreToggleState`] and so in every machine snapshot.

pub mod cancel;
pub mod derive;
pub mod reaction_boxes;
pub mod throw;

use crate::roster::TournamentFighter;

/// The hurtbox bytes (`+0x1a..=+0x21`, facing right) of ground
/// hit-reaction frame record `frame` ([`derive`]), or `None` for a frame no
/// ground reaction plays.
pub fn frame_boxes(frame: u32) -> Option<[u8; 8]> {
    let table = reaction_boxes::REACTION_BOXES;
    table
        .binary_search_by_key(&frame, |&(a, _)| a)
        .ok()
        .map(|i| table[i].1)
}

/// Per-fighter combo state, by [`slot`]: one per player structure, so it
/// follows the fighter when frame flags `0x10` / `0x20` trade the two
/// fighters' object regions mid-move (`swap_regions`, `0x3b852`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ComboState {
    /// Frames left of a combo-able hit reaction (0: none). While it runs the
    /// fighter's reaction frames are hittable.
    pub hitstun: [u8; 3],
    /// The combo-able reaction's first frame has not loaded yet: its hold is
    /// stretched when it does, so the reaction's pose lasts the stun.
    pub fresh: [bool; 3],
    /// Frames left of blockstun (0: none).
    pub blockstun: [u8; 3],
    /// Hits landed on this fighter in the combo running (for [`scaled`]).
    pub hits: [u8; 3],
    /// This fighter's attack connected (hit or guarded) since its move began.
    pub connected: [bool; 3],
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(ComboState, "ComboState.v2", {
    hitstun, fresh, blockstun, hits, connected,
});


/// The combo state's slot of the fighter on player structure `ps` in game
/// mode `game_mode` (`0xffc002`): P1's structure is 0 and P2's 1; the 1987
/// CPU's takes the slot the human side leaves free (1 beside P1 in mode 2,
/// 0 beside P2 in mode 4). Keyed by structure, not by entity, because the
/// two fighters' object regions can trade places mid-move (an entity's
/// +0x38 names its structure wherever its region sits).
pub fn slot(ps: u32, game_mode: u8) -> usize {
    use crate::fighter::{P1_STRUCT, P2_STRUCT};
    match ps {
        P1_STRUCT => 0,
        P2_STRUCT => 1,
        _ => usize::from(game_mode != 4),
    }
}

pub(crate) fn slot_in_fight(ps: u32, game_mode: u8, dramatic: bool) -> usize {
    if dramatic && ps == crate::fighter::P2_STRUCT { 2 } else { slot(ps, game_mode) }
}

/// What landed a hit, for the stun it gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitSource {
    /// The attacker's own attack box: the stun of the move he is in.
    Melee,
    /// A projectile ([`PROJECTILE`]).
    Projectile,
    /// Chip damage through a guard (a guarded projectile or Tatsumaki): the
    /// 1987 reaction, never combo-able.
    Chip,
}

/// Hitstun and blockstun of one move, in frames (see the module docs'
/// "Frames"). A `hit` of 0 leaves the 1987 reaction as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stun {
    pub hit: u8,
    pub block: u8,
}

const fn stun(hit: u8, block: u8) -> Stun {
    Stun { hit, block }
}

/// A move's strength class, for the advantage it is tuned to and the
/// research's numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Light,
    Medium,
    Heavy,
    Sweep,
    /// A jump attack, by its strength.
    Jump(u8),
    Special,
}

/// The research's medians (SF6, `docs/research/sf1plus-reference-numbers.md`
/// 3.2 and 5.2) as stun frames, for a move no fighter row covers: light
/// 15 / 10, medium 22 / 17, heavy 26 / 20, the sweep's 26 / 18. Jump attacks
/// take the same numbers by strength and specials the heavy ones.
pub fn class_default(class: Class) -> Stun {
    match class {
        Class::Light | Class::Jump(1) => stun(15, 10),
        Class::Medium | Class::Jump(2) => stun(22, 17),
        Class::Sweep => stun(26, 18),
        _ => stun(26, 20),
    }
}

/// A projectile's hit (the Hadoken's, the Kiai Shot's, the CPU's
/// projectiles): the heavy research median. A guarded projectile's chip
/// keeps its 1987 reaction, so it has no blockstun here.
pub const PROJECTILE: Stun = stun(26, 0);

/// The strength level 1..=3 a move's entity holds at +0x34: the level the
/// strength-indexed hit functions read, or a latched strength (`0x40`,
/// `0xe0`, `0xfe`) by the 1987 threshold table (`0x405b6`).
pub fn level(b: u8) -> u8 {
    match b {
        1..=3 => b,
        0xf0..=0xff => 3,
        0xd0..=0xef => 2,
        _ => 1,
    }
}

/// The class of player-table move `id` at strength `level`, by the 1987
/// input handler's strength tables (`0x405b6` standing punches 0x0d / 0x0f /
/// 0x10, `0x405be` standing kicks 0x14 / 0x15 / 0x16, `0x40884` / `0x40976` /
/// `0x4097e` crouching punches 0x11..=0x13 light and 0x1c..=0x1e medium /
/// heavy, `0x4088c` crouching kicks 0x17 / 0x18).
pub fn class_of(id: u8, level: u8) -> Class {
    match id {
        0x0d | 0x0e | 0x11..=0x14 => Class::Light,
        0x0f | 0x15 => Class::Medium,
        0x10 | 0x16 => Class::Heavy,
        0x17 if level >= 2 => Class::Medium,
        0x17 => Class::Light,
        0x1c..=0x1e if level >= 3 => Class::Heavy,
        0x1c..=0x1e => Class::Medium,
        0x18 => Class::Sweep,
        0x19..=0x1b => Class::Special,
        0x20..=0x26 => Class::Jump(level.clamp(1, 3)),
        _ => Class::Medium,
    }
}

/// One row of a fighter's stun table: the move id and its stun at strength
/// level 1, 2 and 3 (the same three times for a move whose class does not
/// depend on the button).
type Row = (u8, [Stun; 3]);

const fn same(s: Stun) -> [Stun; 3] {
    [s, s, s]
}

/// Ryu's and Ken's moves (their 1987 frame data is the same for every
/// normal; the Tatsumaki's recovery differs, and specials take fixed
/// numbers). Each normal is tuned to an advantage by class: light +5 on hit
/// and -1 guarded, medium +3 / -2, heavy +2 / -4, the sweep 0 / -10 (the
/// research's SF6 medians, 3.2: lights +5 / -1, mediums +3.5 / -1 to -3,
/// heavies +2.5 / -3 to -6, sweeps -11 guarded), so `hit = r + adv + 2`
/// where `r` is the ticks the move runs after its first active tick (the
/// ROM box audit's total less its startup) and 2 the frames from the module
/// docs. Capped at the research's largest stun for the class (light 18 / 14,
/// medium 26 / 20, heavy 28 / 22). Jump attacks and specials take the class
/// numbers ([`class_default`]): a jump attack's advantage depends on how
/// deep it lands, and nothing links after a special.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e standing LP | 2 | 9 (+5) | 3 (-1) |
/// | 0x0f standing MP | 8 | 13 (+3) | 8 (-2) |
/// | 0x10 standing HP | 10 | 14 (+2) | 8 (-4) |
/// | 0x11 crouching LP (down) | 10 | 17 (+5) | 11 (-1) |
/// | 0x12 crouching LP (down-forward) | 7 | 14 (+5) | 8 (-1) |
/// | 0x13 crouching LP (down-back) | 13 | 18 (+3, cap) | 14 (-1) |
/// | 0x14 standing LK | 12 | 18 (+4, cap) | 13 (-1) |
/// | 0x15 standing MK | 23 | 26 (+1, cap) | 20 (-5, cap) |
/// | 0x16 standing HK | 18 | 22 (+2) | 16 (-4) |
/// | 0x17 crouching LK / MK | 11 | 18 / 16 | 12 / 11 |
/// | 0x18 sweep | 15 | 17 (0) | 7 (-10) |
/// | 0x1c..=0x1e crouching MP / HP | 10 | 15 / 14 | 10 / 8 |
const SHOTO: &[Row] = &[
    (0x0d, same(stun(9, 3))),
    (0x0e, same(stun(9, 3))),
    (0x0f, same(stun(13, 8))),
    (0x10, same(stun(14, 8))),
    (0x11, same(stun(17, 11))),
    (0x12, same(stun(14, 8))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(18, 13))),
    (0x15, same(stun(26, 20))),
    (0x16, same(stun(22, 16))),
    (0x17, [stun(18, 12), stun(16, 11), stun(16, 11)]),
    (0x18, same(stun(17, 7))),
    (0x1c, [stun(15, 10), stun(15, 10), stun(14, 8)]),
    (0x1d, [stun(15, 10), stun(15, 10), stun(14, 8)]),
    (0x1e, [stun(15, 10), stun(15, 10), stun(14, 8)]),
];

/// Retsu's moves on his bank table (ticket #248), by the same rule as
/// [`SHOTO`]. His 1987 attacks keep their ROM streams (the box audit's CPU
/// attack rows); his crouching punch and kick are the bank's (their
/// recovery after the strike is 24 and 14 ticks, `data_bank::retsu`), so
/// they hit long after a light's reach for a link and sit at the caps.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e jab (`0xc9b6`) | 9 | 16 (+5) | 10 (-1) |
/// | 0x0f lunge as MP (`0xca46`) | 18 | 23 (+3) | 18 (-2) |
/// | 0x10 lunge as HP | 18 | 22 (+2) | 16 (-4) |
/// | 0x14 mid kick as LK (`0xcb16`) | 21 | 18 (-5, cap) | 14 (-9, cap) |
/// | 0x15 mid kick as MK | 21 | 26 (+3) | 20 (-3, cap) |
/// | 0x16 high kick (`0xcbe6`) | 25 | 28 (+1, cap) | 22 (-5, cap) |
/// | 0x18 sweep (`0xcf1a`) | 17 | 19 (0) | 9 (-10) |
/// | 0x11..=0x13 crouching LP | 35 | 18 (cap) | 14 (cap) |
/// | 0x1c..=0x1e crouching MP / HP | 35 | 26 / 28 (cap) | 20 / 22 (cap) |
/// | 0x17 crouching kick | 27 | 18 / 26 | 14 / 20 |
const RETSU: &[Row] = &[
    (0x0d, same(stun(16, 10))),
    (0x0e, same(stun(16, 10))),
    (0x0f, same(stun(23, 18))),
    (0x10, same(stun(22, 16))),
    (0x11, same(stun(18, 14))),
    (0x12, same(stun(18, 14))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(18, 14))),
    (0x15, same(stun(26, 20))),
    (0x16, same(stun(28, 22))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(19, 9))),
    (0x1c, [stun(26, 20), stun(26, 20), stun(28, 22)]),
    (0x1d, [stun(26, 20), stun(26, 20), stun(28, 22)]),
    (0x1e, [stun(26, 20), stun(26, 20), stun(28, 22)]),
];

/// Geki's moves on his bank table (ticket #251), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams). Every one of them recovers slowly, so
/// none is plus on hit: his normals are pokes, not combo starters, and the
/// lights and the sweep sit at the caps. His Shuriken is a projectile
/// ([`PROJECTILE`]); his Teleport Strike takes the specials' numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e jab | 19 | 18 (-3, cap) | 14 (-7, cap) |
/// | 0x0f claw thrust as MP | 21 | 26 (+3) | 20 (-3, cap) |
/// | 0x10 claw thrust as HP | 21 | 25 (+2) | 19 (-4) |
/// | 0x11..=0x13 crouching LP | 20 | 18 (-4, cap) | 14 (-8, cap) |
/// | 0x1c..=0x1e crouching MP | 20 | 25 (+3) | 20 (-2) |
/// | 0x1c..=0x1e crouching HP | 20 | 24 (+2) | 18 (-4) |
/// | 0x14 light kick | 24 | 18 (-8, cap) | 14 (-12, cap) |
/// | 0x15 front kick as MK | 19 | 24 (+3) | 19 (-2) |
/// | 0x16 front kick as HK | 19 | 23 (+2) | 17 (-4) |
/// | 0x17 crouching LK / MK | 27 | 18 / 26 | 14 / 20 |
/// | 0x18 sweep | 31 | 28 (-5, cap) | 22 (-11, cap) |
const GEKI: &[Row] = &[
    (0x0d, same(stun(18, 14))),
    (0x0e, same(stun(18, 14))),
    (0x0f, same(stun(26, 20))),
    (0x10, same(stun(25, 19))),
    (0x11, same(stun(18, 14))),
    (0x12, same(stun(18, 14))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(18, 14))),
    (0x15, same(stun(24, 19))),
    (0x16, same(stun(23, 17))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(28, 22))),
    (0x1c, [stun(25, 20), stun(25, 20), stun(24, 18)]),
    (0x1d, [stun(25, 20), stun(25, 20), stun(24, 18)]),
    (0x1e, [stun(25, 20), stun(25, 20), stun(24, 18)]),
];

/// Joe's moves on his bank table (ticket #252), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams, read through the fight reader in
/// `tests/joe_player.rs`). His 1987 jab, straight and low kick recover
/// quickly, so his jab is +5 on hit and links; his body punch (the ROM's
/// stand-up cut, as Geki's crouching claw) is short too. His low kick into
/// a straight (two strikes, the first at tick 4 with `r = 25` and the
/// second at tick 15 with `r = 14`; tuned from the first active strike here,
/// while a spaced hit on the second alone is for Balanced roster) and his
/// new crouching kick and sweep sit at the caps. His Rolling Sobat and
/// Dynamite Straight take the specials' numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e jab (`0xe3ba`) | 5 | 12 (+5) | 6 (-1) |
/// | 0x0f straight as MP (`0xe44a`) | 14 | 19 (+3) | 14 (-2) |
/// | 0x10 straight as HP | 14 | 18 (+2) | 12 (-4) |
/// | 0x11..=0x13 body punch as LP (`0xe60a`) | 6 | 13 (+5) | 7 (-1) |
/// | 0x1c..=0x1e body punch as MP | 6 | 11 (+3) | 6 (-2) |
/// | 0x1c..=0x1e body punch as HP | 6 | 10 (+2) | 4 (-4) |
/// | 0x14 low kick (`0xe7fc`) | 11 | 18 (+5) | 12 (-1) |
/// | 0x15 low kick into a straight (`0xe706`) | 25 | 26 (-1, cap) | 20 (-7, cap) |
/// | 0x16 high kick (`0xe8cc`) | 16 | 20 (+2) | 14 (-4) |
/// | 0x17 crouching LK / MK | 27 | 18 / 26 | 14 / 20 |
/// | 0x18 sweep | 31 | 28 (-5, cap) | 22 (-11, cap) |
const JOE: &[Row] = &[
    (0x0d, same(stun(12, 6))),
    (0x0e, same(stun(12, 6))),
    (0x0f, same(stun(19, 14))),
    (0x10, same(stun(18, 12))),
    (0x11, same(stun(13, 7))),
    (0x12, same(stun(13, 7))),
    (0x13, same(stun(13, 7))),
    (0x14, same(stun(18, 12))),
    (0x15, same(stun(26, 20))),
    (0x16, same(stun(20, 14))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(28, 22))),
    (0x1c, [stun(11, 6), stun(11, 6), stun(10, 4)]),
    (0x1d, [stun(11, 6), stun(11, 6), stun(10, 4)]),
    (0x1e, [stun(11, 6), stun(11, 6), stun(10, 4)]),
];

/// Mike's moves on his bank table (ticket #253), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams, read through the fight reader in
/// `tests/mike_player.rs`). His 1987 jab, straight, lunging hook and
/// ducking body punch are all short: his jab (`0x0d`, `0x0e`, `0x14`) and
/// ducking punch (`0x11..=0x13`, `0x17` light) are +5 on hit and link; his
/// straight (`0x0f`, `0x15`) and medium ducking punch (`0x1c..=0x1e`,
/// `0x17` medium) are +3 on hit. His Bison Rush and Iron Upper take the
/// specials' numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e / 0x14 jab (`0xd8cc`) | 5 | 12 (+5) | 6 (-1) |
/// | 0x0f / 0x15 straight (`0xd956`) | 9 | 14 (+3) | 9 (-2) |
/// | 0x10 / 0x16 lunging hook (`0xdb1c`) | 11 | 15 (+2) | 9 (-4) |
/// | 0x11..=0x13 ducking punch as LP (`0xda46`) | 8 | 15 (+5) | 9 (-1) |
/// | 0x1c..=0x1e ducking punch as MP | 8 | 13 (+3) | 8 (-2) |
/// | 0x1c..=0x1e ducking punch as HP | 8 | 12 (+2) | 6 (-4) |
/// | 0x17 ducking punch as LK / MK | 8 | 15 / 13 | 9 / 8 |
/// | 0x18 ducking punch as HK | 8 | 10 (0) | 4 (-6) |
const MIKE: &[Row] = &[
    (0x0d, same(stun(12, 6))),
    (0x0e, same(stun(12, 6))),
    (0x0f, same(stun(14, 9))),
    (0x10, same(stun(15, 9))),
    (0x11, same(stun(15, 9))),
    (0x12, same(stun(15, 9))),
    (0x13, same(stun(15, 9))),
    (0x14, same(stun(12, 6))),
    (0x15, same(stun(14, 9))),
    (0x16, same(stun(15, 9))),
    (0x17, [stun(15, 9), stun(13, 8), stun(13, 8)]),
    (0x18, same(stun(10, 4))),
    (0x1c, [stun(13, 8), stun(13, 8), stun(12, 6)]),
    (0x1d, [stun(13, 8), stun(13, 8), stun(12, 6)]),
    (0x1e, [stun(13, 8), stun(13, 8), stun(12, 6)]),
];

/// Lee's moves on his bank table (ticket #254), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams, read through the fight reader in
/// `tests/lee_player.rs`). His 1987 palm strike (`0x0d`, `0x0e`, `r = 8`)
/// is +5 on hit and links into a second palm strike or his straight; his
/// straight (`0x0f`, `r = 16`), side kick as MK (`0x15`, `r = 14`) and
/// crouching medium punch (`0x1c..=0x1e`, `r = 21`) are +3 on hit; his
/// crouching lights (`0x11..=0x13`, `0x17` light) sit at the light cap. His
/// Zesshouohken and Double Kick take the specials' numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e palm strike (`0xf36c`) | 8 | 15 (+5) | 9 (-1) |
/// | 0x0f straight (`0xf3d6`) | 16 | 21 (+3) | 16 (-2) |
/// | 0x10 lunge punch (`0xf4a6`) | 16 | 20 (+2) | 14 (-4) |
/// | 0x11..=0x13 crouching LP | 21 | 18 (-5, cap) | 14 (-9, cap) |
/// | 0x1c..=0x1e crouching MP | 21 | 26 (+3) | 20 (-3, cap) |
/// | 0x1c..=0x1e crouching HP | 21 | 25 (+2) | 19 (-4) |
/// | 0x14 side kick as LK (`0xf5c8`) | 14 | 18 (+2, cap) | 14 (-2, cap) |
/// | 0x15 side kick as MK | 14 | 19 (+3) | 14 (-2) |
/// | 0x16 high kick (`0xf678`) | 21 | 25 (+2) | 19 (-4) |
/// | 0x17 crouching LK / MK | 27 | 18 / 26 | 14 / 20 |
/// | 0x18 sweep (`0xfc8e`) | 18 | 20 (0) | 10 (-10) |
const LEE: &[Row] = &[
    (0x0d, same(stun(15, 9))),
    (0x0e, same(stun(15, 9))),
    (0x0f, same(stun(21, 16))),
    (0x10, same(stun(20, 14))),
    (0x11, same(stun(18, 14))),
    (0x12, same(stun(18, 14))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(18, 14))),
    (0x15, same(stun(19, 14))),
    (0x16, same(stun(25, 19))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(20, 10))),
    (0x1c, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1d, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1e, [stun(26, 20), stun(26, 20), stun(25, 19)]),
];

/// Gen's moves on his bank table (ticket #255), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams, read through the fight reader in
/// `tests/gen_player.rs`). His 1987 jab (`0x0d`, `0x0e`, `r = 6`) is +5 on
/// hit and links into a second jab or his front kick; his straight as MP
/// (`0x0f`, `r = 21`), front kick as MK (`0x15`, `r = 17`) and crouching
/// medium punch (`0x1c..=0x1e`, `r = 21`) are +3 on hit; his low kick,
/// crouching lights and sweep sit at their class caps. His Hyakurenko and
/// Gekiro take the specials' numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e jab (`0x103b0`) | 6 | 13 (+5) | 7 (-1) |
/// | 0x0f straight as MP (`0x10440`) | 21 | 26 (+3) | 20 (-3, cap) |
/// | 0x10 straight as HP (`0x10440`) | 21 | 25 (+2) | 19 (-4) |
/// | 0x11..=0x13 crouching LP | 21 | 18 (-5, cap) | 14 (-9, cap) |
/// | 0x1c..=0x1e crouching MP | 21 | 26 (+3) | 20 (-3, cap) |
/// | 0x1c..=0x1e crouching HP | 21 | 25 (+2) | 19 (-4) |
/// | 0x14 low kick | 19 | 18 (-3, cap) | 14 (-7, cap) |
/// | 0x15 front kick as MK (`0x104f0`) | 17 | 22 (+3) | 17 (-2) |
/// | 0x16 front kick as HK (`0x104f0`) | 17 | 21 (+2) | 15 (-4) |
/// | 0x17 crouching LK / MK | 27 | 18 / 26 | 14 / 20 |
/// | 0x18 sweep (`0x10a26`) | 32 | 28 (-6, cap) | 22 (-12, cap) |
const GEN: &[Row] = &[
    (0x0d, same(stun(13, 7))),
    (0x0e, same(stun(13, 7))),
    (0x0f, same(stun(26, 20))),
    (0x10, same(stun(25, 19))),
    (0x11, same(stun(18, 14))),
    (0x12, same(stun(18, 14))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(18, 14))),
    (0x15, same(stun(22, 17))),
    (0x16, same(stun(21, 15))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(28, 22))),
    (0x1c, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1d, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1e, [stun(26, 20), stun(26, 20), stun(25, 19)]),
];

/// Birdie's moves on his bank table (ticket #256), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams, read through the fight reader in
/// `tests/birdie_player.rs`). His 1987 jab (`0x0d`, `0x0e`, `r = 7`) is +5
/// on hit and links into a second jab or his 4-frame standing light kick
/// (`0x14`, `r = 12`, +4 on hit); his hook (`0x0f`, `r = 13`), low kick as
/// MK (`0x15`, `r = 12`) and crouching medium punch (`0x1c..=0x1e`, `r = 21`)
/// are +3 on hit; his double-fist hammer (`0x10`, `r = 17`) is +2 on hit; his
/// standing high kick, crouching lights and sweep sit at their class caps.
/// His Bull Head and Bull Horn take the specials' numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e jab (`0x1141c`) | 7 | 14 (+5) | 8 (-1) |
/// | 0x0f hook (`0x1146c`) | 13 | 18 (+3) | 13 (-2) |
/// | 0x10 hammer (`0x1151c`) | 17 | 21 (+2) | 15 (-4) |
/// | 0x11..=0x13 crouching LP | 21 | 18 (-5, cap) | 14 (-9, cap) |
/// | 0x1c..=0x1e crouching MP | 21 | 26 (+3) | 20 (-3, cap) |
/// | 0x1c..=0x1e crouching HP | 21 | 25 (+2) | 19 (-4) |
/// | 0x14 low kick as LK (`0x115cc`) | 12 | 18 (+4, cap) | 13 (-1) |
/// | 0x15 low kick as MK (`0x115cc`) | 12 | 17 (+3) | 12 (-2) |
/// | 0x16 high kick | 29 | 28 (-3, cap) | 22 (-9, cap) |
/// | 0x17 crouching LK / MK | 27 | 18 / 26 | 14 / 20 |
/// | 0x18 sweep | 33 | 28 (-7, cap) | 22 (-13, cap) |
const BIRDIE: &[Row] = &[
    (0x0d, same(stun(14, 8))),
    (0x0e, same(stun(14, 8))),
    (0x0f, same(stun(18, 13))),
    (0x10, same(stun(21, 15))),
    (0x11, same(stun(18, 14))),
    (0x12, same(stun(18, 14))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(18, 13))),
    (0x15, same(stun(17, 12))),
    (0x16, same(stun(28, 22))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(28, 22))),
    (0x1c, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1d, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1e, [stun(26, 20), stun(26, 20), stun(25, 19)]),
];

/// Eagle's moves on his bank table (ticket #257), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams, read through the fight reader in
/// `tests/eagle_player.rs`). His 1987 stick jab (`0x0d`, `0x0e`, `0x14`,
/// `r = 7`) is +5 on hit and links into a second stick jab; his 1987 low
/// stick thrust on standing MP (`0x0f`, `r = 15`) and crouching MP
/// (`0x1c..=0x1e`, `r = 15`) is +3 on hit; his 1987 spinning sticks on
/// standing HP (`0x10`, `r = 19`) and low stick thrust on crouching HP
/// (`0x1c..=0x1e`, `r = 15`) are +2 on hit; his crouching light punches,
/// rising stick strike (`0x15`), two-stick smash (`0x16`), crouching stick
/// sweep (`0x17`) and crouching stick uppercut (`0x18`) sit at their class
/// caps. His Manchester Black and St. Andrew's Green take the specials'
/// numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e stick jab (`0x1239e`) | 7 | 14 (+5) | 8 (-1) |
/// | 0x0f low thrust as MP (`0x12628`) | 15 | 20 (+3) | 15 (-2) |
/// | 0x10 spinning sticks (`0x12408`) | 19 | 23 (+2) | 17 (-4) |
/// | 0x11..=0x13 crouching LP (`0x12628`) | 15 | 18 (+1, cap) | 14 (-3, cap) |
/// | 0x1c..=0x1e crouching MP (`0x12628`) | 15 | 20 (+3) | 15 (-2) |
/// | 0x1c..=0x1e crouching HP (`0x12628`) | 15 | 19 (+2) | 13 (-4) |
/// | 0x14 low stick poke (`0x1239e`) | 7 | 14 (+5) | 8 (-1) |
/// | 0x15 rising stick strike | 23 | 26 (+1, cap) | 20 (-5, cap) |
/// | 0x16 two-stick smash | 27 | 28 (-1, cap) | 22 (-7, cap) |
/// | 0x17 crouching stick sweep | 25 | 18 / 26 | 14 / 20 |
/// | 0x18 crouching stick uppercut | 29 | 28 (-3, cap) | 22 (-9, cap) |
const EAGLE: &[Row] = &[
    (0x0d, same(stun(14, 8))),
    (0x0e, same(stun(14, 8))),
    (0x0f, same(stun(20, 15))),
    (0x10, same(stun(23, 17))),
    (0x11, same(stun(18, 14))),
    (0x12, same(stun(18, 14))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(14, 8))),
    (0x15, same(stun(26, 20))),
    (0x16, same(stun(28, 22))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(28, 22))),
    (0x1c, [stun(20, 15), stun(20, 15), stun(19, 13)]),
    (0x1d, [stun(20, 15), stun(20, 15), stun(19, 13)]),
    (0x1e, [stun(20, 15), stun(20, 15), stun(19, 13)]),
];

/// Adon's moves on his bank table (ticket #258), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams, read through the fight reader in
/// `tests/adon_player.rs`). His 1987 body punch as MP (`0x0f`, `r = 14`), low
/// kick as MK (`0x15`, `r = 15`) and crouching medium punch (`0x1c..=0x1e`,
/// `r = 21`) are +3 on hit; his body punch as HP (`0x10`, `r = 14`) and
/// crouching heavy punch (`0x1c..=0x1e`, `r = 21`) are +2 on hit; his 1987
/// low kick as LK (`0x14`, `r = 15`, +1 on hit at the light cap) and his
/// double jab (`0x0d`, `0x0e`, `r = 24`), crouching lights (`0x11..=0x13`,
/// `0x17` at L1), crouching MK (`0x17` at L2), two-kick combination (`0x16`,
/// `r = 30`) and sweep (`0x18`, `r = 33`) sit at their class caps. His
/// Rising Jaguar and Jaguar Kick take the specials' numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e double jab (`0x132dc`) | 24 | 18 (-8, cap) | 14 (-12, cap) |
/// | 0x0f body punch as MP (`0x1336c`) | 14 | 19 (+3) | 14 (-2) |
/// | 0x10 body punch as HP (`0x1336c`) | 14 | 18 (+2) | 12 (-4) |
/// | 0x11..=0x13 crouching LP | 21 | 18 (-5, cap) | 14 (-9, cap) |
/// | 0x1c..=0x1e crouching MP | 21 | 26 (+3) | 20 (-3, cap) |
/// | 0x1c..=0x1e crouching HP | 21 | 25 (+2) | 19 (-4) |
/// | 0x14 low kick as LK (`0x133fc`) | 15 | 18 (+1, cap) | 14 (-3, cap) |
/// | 0x15 low kick as MK (`0x133fc`) | 15 | 20 (+3) | 15 (-2) |
/// | 0x16 two-kick combination (`0x13850`) | 30 | 28 (-4, cap) | 22 (-10, cap) |
/// | 0x17 crouching LK / MK | 27 | 18 / 26 | 14 / 20 |
/// | 0x18 sweep | 33 | 28 (-7, cap) | 22 (-13, cap) |
const ADON: &[Row] = &[
    (0x0d, same(stun(18, 14))),
    (0x0e, same(stun(18, 14))),
    (0x0f, same(stun(19, 14))),
    (0x10, same(stun(18, 12))),
    (0x11, same(stun(18, 14))),
    (0x12, same(stun(18, 14))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(18, 14))),
    (0x15, same(stun(20, 15))),
    (0x16, same(stun(28, 22))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(28, 22))),
    (0x1c, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1d, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1e, [stun(26, 20), stun(26, 20), stun(25, 19)]),
];

/// Sagat's moves on his bank table (ticket #259), by the same rule as
/// [`SHOTO`], from the ticks each runs after its first active tick (his
/// bank table's headers and streams, read through the fight reader in
/// `tests/sagat_player.rs`). His 1987 jab (`0x0d`, `0x0e`, `r = 10`) is +5
/// on hit and links into a second jab or his step punch (`0x0f`); his 1987
/// step punch as MP (`0x0f`, `r = 20`) and crouching medium punch
/// (`0x1c..=0x1e`, `r = 21`) are +3 on hit; his step punch as HP (`0x10`,
/// `r = 20`) and crouching heavy punch (`0x1c..=0x1e`, `r = 21`) are +2 on
/// hit; his standing kicks (`0x14..=0x16`), crouching lights (`0x11..=0x13`,
/// `0x17` at L1), crouching MK (`0x17` at L2) and sweep (`0x18`) sit at their
/// class caps. His High Tiger Shot, Low Tiger Shot and Tiger Knee take the
/// projectiles' and specials' numbers.
///
/// | Move | r | hit | guarded |
/// | --- | --- | --- | --- |
/// | 0x0d / 0x0e jab (`0x13ec0`) | 10 | 17 (+5) | 11 (-1) |
/// | 0x0f step punch as MP (`0x13f10`) | 20 | 25 (+3) | 20 (-2) |
/// | 0x10 step punch as HP (`0x13f10`) | 20 | 24 (+2) | 18 (-4) |
/// | 0x11..=0x13 crouching LP | 21 | 18 (-5, cap) | 14 (-9, cap) |
/// | 0x1c..=0x1e crouching MP | 21 | 26 (+3) | 20 (-3, cap) |
/// | 0x1c..=0x1e crouching HP | 21 | 25 (+2) | 19 (-4) |
/// | 0x14 low kick (`0x13fa0`) | 17 | 18 (-1, cap) | 14 (-5, cap) |
/// | 0x15 high kick as MK (`0x14030`) | 24 | 26 (0, cap) | 20 (-6, cap) |
/// | 0x16 high kick as HK (`0x14030`) | 24 | 28 (+2, cap) | 22 (-4, cap) |
/// | 0x17 crouching LK / MK | 25 | 18 / 26 | 14 / 20 |
/// | 0x18 sweep | 43 | 28 (-17, cap) | 22 (-23, cap) |
const SAGAT: &[Row] = &[
    (0x0d, same(stun(17, 11))),
    (0x0e, same(stun(17, 11))),
    (0x0f, same(stun(25, 20))),
    (0x10, same(stun(24, 18))),
    (0x11, same(stun(18, 14))),
    (0x12, same(stun(18, 14))),
    (0x13, same(stun(18, 14))),
    (0x14, same(stun(18, 14))),
    (0x15, same(stun(26, 20))),
    (0x16, same(stun(28, 22))),
    (0x17, [stun(18, 14), stun(26, 20), stun(26, 20)]),
    (0x18, same(stun(28, 22))),
    (0x1c, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1d, [stun(26, 20), stun(26, 20), stun(25, 19)]),
    (0x1e, [stun(26, 20), stun(26, 20), stun(25, 19)]),
];

/// The stun table of `fighter`'s player moves, when his rows are tuned.
fn rows(fighter: TournamentFighter) -> Option<&'static [Row]> {
    match fighter {
        TournamentFighter::Ryu | TournamentFighter::Ken => Some(SHOTO),
        TournamentFighter::Retsu => Some(RETSU),
        TournamentFighter::Geki => Some(GEKI),
        TournamentFighter::Joe => Some(JOE),
        TournamentFighter::Mike => Some(MIKE),
        TournamentFighter::Lee => Some(LEE),
        TournamentFighter::Gen => Some(GEN),
        TournamentFighter::Birdie => Some(BIRDIE),
        TournamentFighter::Eagle => Some(EAGLE),
        TournamentFighter::Adon => Some(ADON),
        TournamentFighter::Sagat => Some(SAGAT),
    }
}

/// The stun of move `id` at strength `level` (1..=3) by `fighter` (`None`:
/// a move table no fighter owns): his row, else the class's research
/// numbers.
pub fn move_stun(fighter: Option<TournamentFighter>, id: u8, level: u8) -> Stun {
    let level = level.clamp(1, 3);
    let row = fighter
        .and_then(rows)
        .and_then(|rows| rows.iter().find(|(m, _)| *m == id));
    match row {
        Some((_, by_level)) => by_level[level as usize - 1],
        None => class_default(class_of(id, level)),
    }
}

/// The share of a hit's damage the combo's `n`th hit keeps, in percent:
/// SF6's general schedule (research 3.5: 100, 100, 80, 70, 60, 50, 40, 30,
/// 20, then 10 from the tenth hit). The 1987 game's damage is coarse (a
/// light hit takes 1 or 2 of 48, a heavy 8 or 10, a special 20), and the
/// ROM's knock-backs end most combos after two or three hits, so scaling
/// mostly bites on long strings against a crouching guard.
pub fn scale_percent(n: u8) -> u16 {
    match n {
        0..=2 => 100,
        3 => 80,
        4 => 70,
        5 => 60,
        6 => 50,
        7 => 40,
        8 => 30,
        9 => 20,
        _ => 10,
    }
}

/// `damage` as the combo's `n`th hit takes it: [`scale_percent`], rounded
/// to the nearest point, and never below one point for a hit that did
/// damage.
pub fn scaled(damage: u8, n: u8) -> u8 {
    if damage == 0 {
        return 0;
    }
    let d = (u16::from(damage) * scale_percent(n) + 50) / 100;
    d.clamp(1, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_cover_the_three_fighters_and_others_take_the_class_numbers() {
        let ryu = Some(TournamentFighter::Ryu);
        assert_eq!(move_stun(ryu, 0x0d, 1), stun(9, 3));
        assert_eq!(move_stun(Some(TournamentFighter::Ken), 0x0d, 1), stun(9, 3));
        assert_eq!(
            move_stun(Some(TournamentFighter::Retsu), 0x0d, 1),
            stun(16, 10)
        );
        assert_eq!(move_stun(ryu, 0x12, 1), stun(14, 8));
        assert_eq!(move_stun(ryu, 0x13, 1), stun(18, 14));
        assert_eq!(move_stun(ryu, 0x1d, 2), stun(15, 10));
        assert_eq!(move_stun(ryu, 0x1d, 3), stun(14, 8));
        assert_eq!(move_stun(ryu, 0x17, 1), stun(18, 12));
        assert_eq!(move_stun(ryu, 0x17, 2), stun(16, 11));
        // Diagonal crouching punches in class_of (0x12 / 0x13 light, 0x1d medium / heavy).
        assert_eq!(class_of(0x12, 1), Class::Light);
        assert_eq!(class_of(0x13, 1), Class::Light);
        assert_eq!(class_of(0x1d, 2), Class::Medium);
        assert_eq!(class_of(0x1d, 3), Class::Heavy);
        // A jump attack and a special take the class numbers.
        assert_eq!(move_stun(ryu, 0x23, 3), stun(26, 20));
        assert_eq!(move_stun(ryu, 0x20, 1), stun(15, 10));
        assert_eq!(move_stun(ryu, 0x19, 1), stun(26, 20));
        assert_eq!(
            move_stun(Some(TournamentFighter::Geki), 0x0d, 1),
            stun(18, 14)
        );
        let joe = Some(TournamentFighter::Joe);
        assert_eq!(move_stun(joe, 0x0d, 1), stun(12, 6));
        for id in [0x11, 0x12, 0x13] {
            assert_eq!(
                move_stun(joe, id, 1),
                stun(13, 7),
                "Joe crouching LP {id:#04x}"
            );
        }
        for id in [0x1c, 0x1d, 0x1e] {
            assert_eq!(
                move_stun(joe, id, 2),
                stun(11, 6),
                "Joe crouching MP {id:#04x}"
            );
            assert_eq!(
                move_stun(joe, id, 3),
                stun(10, 4),
                "Joe crouching HP {id:#04x}"
            );
        }
        let mike = Some(TournamentFighter::Mike);
        assert_eq!(move_stun(mike, 0x0d, 1), stun(12, 6));
        assert_eq!(move_stun(mike, 0x11, 1), stun(15, 9));
        assert_eq!(move_stun(mike, 0x1c, 2), stun(13, 8));
        assert_eq!(move_stun(mike, 0x1c, 3), stun(12, 6));
        let lee = Some(TournamentFighter::Lee);
        assert_eq!(move_stun(lee, 0x0d, 1), stun(15, 9));
        assert_eq!(move_stun(lee, 0x11, 1), stun(18, 14));
        assert_eq!(move_stun(lee, 0x1c, 2), stun(26, 20));
        assert_eq!(move_stun(lee, 0x1c, 3), stun(25, 19));
        let gen = Some(TournamentFighter::Gen);
        assert_eq!(move_stun(gen, 0x0d, 1), stun(13, 7));
        assert_eq!(move_stun(gen, 0x15, 2), stun(22, 17));
        assert_eq!(move_stun(gen, 0x18, 3), stun(28, 22));
        let birdie = Some(TournamentFighter::Birdie);
        assert_eq!(move_stun(birdie, 0x0d, 1), stun(14, 8));
        assert_eq!(move_stun(birdie, 0x0f, 2), stun(18, 13));
        assert_eq!(move_stun(birdie, 0x10, 3), stun(21, 15));
        assert_eq!(move_stun(birdie, 0x14, 1), stun(18, 13));
        let eagle = Some(TournamentFighter::Eagle);
        assert_eq!(move_stun(eagle, 0x0d, 1), stun(14, 8));
        assert_eq!(move_stun(eagle, 0x0f, 2), stun(20, 15));
        assert_eq!(move_stun(eagle, 0x10, 3), stun(23, 17));
        assert_eq!(move_stun(eagle, 0x14, 1), stun(14, 8));
        assert_eq!(move_stun(eagle, 0x1c, 2), stun(20, 15));
        assert_eq!(move_stun(eagle, 0x1c, 3), stun(19, 13));
        let adon = Some(TournamentFighter::Adon);
        assert_eq!(move_stun(adon, 0x0d, 1), stun(18, 14));
        assert_eq!(move_stun(adon, 0x0f, 2), stun(19, 14));
        assert_eq!(move_stun(adon, 0x10, 3), stun(18, 12));
        assert_eq!(move_stun(adon, 0x15, 2), stun(20, 15));
        assert_eq!(move_stun(adon, 0x1c, 2), stun(26, 20));
        assert_eq!(move_stun(adon, 0x1c, 3), stun(25, 19));
        let sagat = Some(TournamentFighter::Sagat);
        assert_eq!(move_stun(sagat, 0x0d, 1), stun(17, 11));
        assert_eq!(move_stun(sagat, 0x0f, 2), stun(25, 20));
        assert_eq!(move_stun(sagat, 0x10, 3), stun(24, 18));
        assert_eq!(move_stun(sagat, 0x14, 1), stun(18, 14));
        assert_eq!(move_stun(sagat, 0x15, 2), stun(26, 20));
        assert_eq!(move_stun(sagat, 0x16, 3), stun(28, 22));
        assert_eq!(move_stun(sagat, 0x1c, 2), stun(26, 20));
        assert_eq!(move_stun(sagat, 0x1c, 3), stun(25, 19));
        // A move table no fighter owns.
        assert_eq!(move_stun(None, 0x0d, 1), stun(15, 10));
        assert_eq!(move_stun(None, 0x18, 1), stun(26, 18));
    }

    #[test]
    fn scaling_keeps_two_hits_whole_and_never_drops_a_hit_to_nothing() {
        assert_eq!(scaled(8, 1), 8);
        assert_eq!(scaled(8, 2), 8);
        assert_eq!(scaled(8, 3), 6);
        assert_eq!(scaled(10, 4), 7);
        assert_eq!(scaled(2, 9), 1);
        assert_eq!(scaled(1, 20), 1);
        assert_eq!(scaled(0, 3), 0);
    }

    #[test]
    fn levels_read_both_levels_and_latched_strengths() {
        assert_eq!(level(2), 2);
        assert_eq!(level(0x40), 1);
        assert_eq!(level(0xe0), 2);
        assert_eq!(level(0xfe), 3);
        assert_eq!(level(0), 1);
    }
}
