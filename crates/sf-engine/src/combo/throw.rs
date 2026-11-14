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
crate::canonical::canonical_struct!(ThrowState, "ThrowState.v5", {
    tech_window, wakeup_invuln, crosses, partners, st_ryu, st_order_seed,
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
    /// A measured Super Turbo throw, indexed by the captured fighter.
    pub(crate) st_ryu: [Option<StRyuThrow>; 3],
    /// Independent World Super Turbo fighter-order random state.
    pub(crate) st_order_seed: Option<u16>,
}

impl ThrowState {
    /// Whether this fighter is still attached to a measured Super Turbo throw.
    /// The airborne recovery after release does not count as capture.
    pub fn st_ryu_capture_active(&self, victim_slot: usize) -> bool {
        self.st_ryu[victim_slot].is_some_and(StRyuThrow::capture_active)
    }

    /// Seed used by the scoped Super Turbo order selector on its next update.
    pub fn st_order_seed(&self) -> Option<u16> {
        self.st_order_seed
    }
}

/// World Super Turbo ground throws with captured trajectories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StRyuThrowFamily {
    Shoulder,
    Flip,
    Sagat,
    KenShoulder,
    HellWheel,
    KneeBash,
    KenAir,
}

crate::canonical::canonical_enum!(StRyuThrowFamily, "StRyuThrowFamily.v4";
    Shoulder => 0, Flip => 1, Sagat => 2, KenShoulder => 3, HellWheel => 4, KneeBash => 5, KenAir => 6,
);

impl StRyuThrowFamily {
    pub(crate) const fn release_tick(self) -> u8 {
        match self {
            Self::Shoulder | Self::KenShoulder => 44,
            Self::Flip => 38,
            Self::Sagat => 39,
            Self::HellWheel => 62,
            Self::KneeBash => 179,
            Self::KenAir => 46,
        }
    }

    pub(crate) const fn attacker_recovery_tick(self) -> u8 {
        match self {
            Self::Shoulder | Self::KenShoulder => 74,
            Self::Flip => 80,
            Self::Sagat => 59,
            Self::HellWheel => 109,
            Self::KneeBash => 179,
            Self::KenAir => 84,
        }
    }

    pub(crate) const fn can_soften(self) -> bool {
        !matches!(self, Self::KneeBash)
    }
}

/// Capture-time inputs and the elapsed game clock for a Super Turbo throw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StRyuThrow {
    pub family: StRyuThrowFamily,
    pub origin_x: u16,
    pub leftward: bool,
    pub elapsed: u16,
    pub softened: bool,
    pub back: bool,
    pub start_height: u8,
    pub knee: KneeBash,
}

crate::canonical::canonical_struct!(StRyuThrow, "StRyuThrow.v4", {
    family, origin_x, leftward, elapsed, softened, back, start_height, knee,
});

/// World Ken's hold subtracts escape effort from the defender's grip and
/// uses the attacker's effort to advance the next knee animation frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KneeBash {
    pub grip: i8,
    pub attack_budget: i8,
    pub hold_ticks: u8,
    pub freeze: u8,
    pub pose_tick: u8,
    pub phase: u8,
    pub directions: [u8; 2],
    pub released_at: Option<u16>,
    pub release_pose: u8,
    pub hits: u8,
    pub damage_hp: u8,
    pub knee_pending: bool,
}

crate::canonical::canonical_struct!(KneeBash, "KneeBash.v1", {
    grip, attack_budget, hold_ticks, freeze, pose_tick, phase, directions, released_at, release_pose, hits, damage_hp, knee_pending,
});

impl KneeBash {
    pub(crate) const fn new() -> Self {
        // Representative values from the repeatable World 940223 fixture.
        Self {
            grip: 42,
            attack_budget: 8,
            hold_ticks: 119,
            freeze: 0,
            pose_tick: 13,
            phase: 0,
            directions: [0; 2],
            released_at: None,
            release_pose: 0,
            hits: 0,
            damage_hp: 0,
            knee_pending: false,
        }
    }
}

impl StRyuThrow {
    pub(crate) const fn new(family: StRyuThrowFamily, origin_x: u16, leftward: bool) -> Self {
        Self {
            family,
            origin_x,
            leftward,
            elapsed: 0,
            softened: false,
            back: false,
            start_height: 0,
            knee: KneeBash::new(),
        }
    }

    pub(crate) const fn capture_active(self) -> bool {
        if matches!(self.family, StRyuThrowFamily::KneeBash) {
            self.knee.released_at.is_none()
        } else {
            self.elapsed < self.release_tick()
        }
    }

    pub(crate) const fn release_tick(self) -> u16 {
        match self.knee.released_at {
            Some(tick) => tick,
            None => self.family.release_tick() as u16,
        }
    }

    pub(crate) const fn attacker_recovery_tick(self) -> u16 {
        if matches!(self.family, StRyuThrowFamily::KneeBash) {
            self.release_tick()
        } else {
            self.family.attacker_recovery_tick() as u16
        }
    }

    pub(crate) const fn landing_tick(self) -> u16 {
        use StRyuThrowFamily::*;
        match (self.family, self.softened) {
            (Shoulder | KenShoulder, true) => 94,
            (Shoulder | KenShoulder, false) => 79,
            (Flip, true) => 87,
            (Flip, false) => 71,
            (Sagat, true) => 96,
            (Sagat, false) => 82,
            (HellWheel, true) => {
                if self.back {
                    116
                } else {
                    111
                }
            }
            (HellWheel, false) => {
                if self.back {
                    101
                } else {
                    95
                }
            }
            (KneeBash, _) => self.release_tick() + self.knee_landing_delay() as u16,
            (KenAir, true) => 95,
            (KenAir, false) => 79,
        }
    }

    pub(crate) const fn damage_tick(self) -> u16 {
        if self.softened {
            self.release_tick()
        } else {
            match self.family {
                StRyuThrowFamily::Sagat => self.landing_tick() + 1,
                _ => self.landing_tick(),
            }
        }
    }

    pub(crate) const fn defender_recovery_tick(self) -> u16 {
        match (self.family, self.softened) {
            (StRyuThrowFamily::Shoulder, false) => 153,
            (StRyuThrowFamily::Flip, false) => 144,
            (StRyuThrowFamily::Shoulder, true) => 100,
            (StRyuThrowFamily::Flip, true) => 93,
            (StRyuThrowFamily::Sagat, false) => {
                if self.leftward {
                    145
                } else {
                    146
                }
            }
            (StRyuThrowFamily::Sagat, true) => 102,
            (StRyuThrowFamily::KenShoulder, false) => 153,
            (StRyuThrowFamily::KenShoulder, true) => 100,
            (StRyuThrowFamily::HellWheel, false) => {
                if self.back {
                    162
                } else {
                    168
                }
            }
            (StRyuThrowFamily::HellWheel, true) => self.landing_tick() + 6,
            (StRyuThrowFamily::KneeBash, _) => self.landing_tick() + 6,
            (StRyuThrowFamily::KenAir, false) => 153,
            (StRyuThrowFamily::KenAir, true) => 101,
        }
    }

    pub(crate) const fn damage_hp(self) -> Option<u16> {
        if matches!(self.family, StRyuThrowFamily::KneeBash) {
            return if self.knee.damage_hp == 0 {
                None
            } else {
                Some(self.knee.damage_hp as u16)
            };
        }
        if self.elapsed == self.damage_tick() {
            Some(if self.softened { 16 } else { 32 })
        } else {
            None
        }
    }

    pub(crate) const fn knee_landing_delay(self) -> u8 {
        match self.knee.release_pose {
            1 => 47,
            2 => 44,
            _ => 44,
        }
    }
}
