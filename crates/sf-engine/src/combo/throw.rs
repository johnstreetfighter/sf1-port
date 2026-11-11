//! Throws and throw techs (spec #246, ticket #263, `SP19`): the modern rule
//! that gives every fighter on the roster his close-range throw, the victim's
//! being-thrown arc and get-up, a 7-frame throw tech window, and throw
//! invulnerability on wakeup (`docs/design/2026-11-07-sf1plus-on-the-real-engine-design.md`
//! 5.2, ADR-0014).
//!
//! **The switch.** [`CoreToggles::throws`](crate::config::CoreToggles) (on in
//! Definitive Overhaul and Pure Footsies, off in Classic Competitive, never on
//! in Faithful 1987). Every gate that reads it is on the translated path in
//! [`crate::fighter`]; with it off nothing here runs, [`ThrowState`] stays at
//! its default, and the fight is the 1987 fight byte for byte.
//!
//! **What 1987 does.** Street Fighter (1987) has no throws: holding forward or
//! back and pressing heavy punch or heavy kick starts the standing heavy punch
//! (`0x10`) or heavy kick (`0x16`) at any range.
//!
//! **What the switch changes.**
//!
//! - **Close-range proximity throw (`Forward/Back + HP/HK`).** When a grounded,
//!   actionable fighter holds forward or back (`d0` 1 or 2) with a latched
//!   heavy punch or heavy kick (`0xfe`), no special command is ready for that
//!   direction and button, and the opponent is alive, grounded, within
//!   [`RANGE`] pixels centre-to-centre, and not in hitstun, blockstun, a throw,
//!   a get-up, or post-wakeup throw invulnerability ([`WAKEUP_INVULN`]), the
//!   throw connects: the attacker starts his catalogue's `"throw"` stream
//!   ([`THROW_MOVE`]) and the defender starts his catalogue's `"thrown"` stream
//!   ([`THROWN_MOVE`]). Outside [`RANGE`], or when the opponent cannot be
//!   thrown, the normal heavy attack comes out as in 1987.
//! - **Authored throw motion.** Ryu, Ken, Geki, Joe, Lee and Eagle carry the
//!   victim across their original side through their shoulder, circle, hip,
//!   over-the-back and rear-shoulder throws. Either forward or back starts
//!   that crossing motion. Retsu, Mike, Gen, Birdie, Adon and Sagat throw or
//!   shove in front when forward is held; back reverses their destination.
//!   The victim travels through the arc instead of changing sides at once.
//! - **7-frame throw tech window ([`TECH_WINDOW`]).** While the victim is in
//!   the seized `grabbed` frame during the first [`TECH_WINDOW`] frames of a
//!   throw, pressing forward or back with heavy punch or heavy kick (or
//!   holding either heavy button with forward or back) techs the throw: both
//!   fighters return to their standing stance (`move 1`), push apart by
//!   [`TECH_PUSH`] pixels, and take no damage.
//! - **Slam, knockdown, and get-up.** Once the tech window passes without a
//!   tech, the victim travels through his airborne `thrown_1` and `thrown_2`
//!   poses and slams onto the mat (`lying`), taking [`DAMAGE`] (`6 / 48` =
//!   12.5% life, `500` points). If the slam empties his life bar, the round
//!   ends in a knock-out; otherwise he rises through his catalogue's
//!   `"get-up"` poses ([`GET_UP_MOVE`]) and receives [`WAKEUP_INVULN`] frames
//!   of throw invulnerability when the get-up finishes.
//!
//! **Determinism.** All throw timers live in [`ThrowState`], which rides in
//! [`crate::config::CoreToggleState`] and so in every machine snapshot.

/// Maximum centre-to-centre distance (in pixels) at which a throw connects.
///
/// Leaves footsies range to the normal heavy attacks. Eligibility also
/// depends on the participants' current grounded and actionable states.
pub const RANGE: u16 = 48;

/// Initial centre-to-centre separation (in pixels) when the grab connects.
pub const GRAB_GAP: u16 = 32;

/// Horizontal pixels per airborne tick for a throw that crosses the
/// thrower's original side. Sixteen airborne ticks carry the victim across
/// the grab gap and onto the other side without a position jump.
pub const CROSS_ARC_SPEED: i8 = 4;

/// Whether this throw's authored motion carries the victim across the
/// thrower's original side. The shoulder throws and other over-the-body
/// throws cross for either input; front shoves/slams reverse only on back.
///
/// The table follows the approved pose catalogue: Retsu's wrist flip lands
/// in front, while Eagle's stick fling goes over his rear shoulder.
pub const fn crosses_original_side(fighter: crate::roster::TournamentFighter, back: bool) -> bool {
    use crate::roster::TournamentFighter::*;
    let authored_cross = match fighter {
        Ryu | Ken | Geki | Joe | Lee | Eagle => true,
        Retsu | Mike | Gen | Birdie | Adon | Sagat => false,
    };
    back || authored_cross
}

/// Frames after a throw connects during which the victim can tech the throw
/// (`docs/research/sf1plus-reference-numbers.md` 3.7: SF3's 5-frame / SF6's
/// 7-frame window).
pub const TECH_WINDOW: u8 = 7;

/// Horizontal separation (in pixels) applied to the two fighters on a throw
/// tech.
pub const TECH_PUSH: u16 = 28;

/// Damage dealt when the throw slams the victim to the mat (`6 / 48` = 12.5%
/// of the 48-point life bar; `docs/research/sf1plus-reference-numbers.md` 3.7:
/// ST / USF4 / SF6 throws deal 12–17% of full life).
pub const DAMAGE: u8 = 6;

/// Frames of throw invulnerability after the victim's `"get-up"` finishes, so
/// the thrower cannot loop an inescapable throw on the first wakeup frames
/// (`docs/research/sf1plus-reference-numbers.md` 3.7).
pub const WAKEUP_INVULN: u8 = 13;

/// Move id of the thrower's `"throw"` stream (`0x0d..=0x2f` so
/// [`FighterRead::attacking`](crate::fight_reader::FighterRead::attacking)
/// reads true).
pub const THROW_MOVE: u8 = 0x27;

/// Move id of the victim's `"get-up"` stream after lying on the mat (excluded
/// from [`FighterRead::attacking`](crate::fight_reader::FighterRead::attacking)
/// and reported as [`FighterRead::getting_up`](crate::fight_reader::FighterRead::getting_up)).
pub const GET_UP_MOVE: u8 = 0x28;

/// Move id of the victim's `"thrown"` stream (`0x30..=0x3f` so
/// [`FighterRead::reacting`](crate::fight_reader::FighterRead::reacting) reads
/// true when the slam deals damage, while excluded from
/// [`FighterRead::hitstun`](crate::fight_reader::FighterRead::hitstun)).
pub const THROWN_MOVE: u8 = 0x3e;

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(ThrowState, "ThrowState.v3", {
    tech_window, wakeup_invuln, crosses, partners,
});


/// Per-fighter throw runtime state, indexed by [`super::slot`] (`0 = P1`,
/// `1 = P2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThrowState {
    /// Frames remaining in the victim's [`TECH_WINDOW`], indexed by the
    /// victim's slot (`0` when not in a techable grab).
    pub tech_window: [u8; 3],
    /// Frames of post-wakeup throw invulnerability remaining after finishing
    /// `"get-up"`, indexed by the fighter's slot.
    pub wakeup_invuln: [u8; 3],
    /// Whether the active throw on this victim slot crosses the thrower's
    /// original side. Set by the authored motion and input at the grab;
    /// retained throughout the airborne arc, cleared by a slam or tech.
    pub crosses: [bool; 3],
    /// The other participant's player structure during a Dramatic throw.
    pub partners: [u32; 3],
}
