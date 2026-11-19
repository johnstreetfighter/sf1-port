//! Stun build-up and the dizzy it leads to (KS17, #234). `SF KESSEN` is the
//! only style that deals stun so far: a Kessen attacker's clean hits add
//! his move's stun to the defender, whatever the defender's style, and a
//! full total leaves the defender dizzy. The rules follow Super Turbo
//! (SuperCombo, `Super_Street_Fighter_2_Turbo/System`, "Dizzy"), with the
//! maintainer's choices of 2026-11-18: each move deals the middle of its
//! Super Turbo range, the total empties all at once when the hit timer runs
//! out, and a dizzy lasts a fixed time with no mashing.

/// The stun that dizzies, by the dizzies already taken this match: Super
/// Turbo's 30, then 34, then 35 from the third on.
pub const KESSEN_STUN_LIMITS: [u8; 3] = [30, 34, 35];
/// How long a dizzy lasts, counted while the fighter stands in it.
pub const KESSEN_DIZZY_FRAMES: u8 = 150;
/// After a dizzy ends, hits add no stun for this long (Super Turbo's "about
/// a second").
pub const KESSEN_STUN_REST_FRAMES: u8 = 60;
/// The dizzy sway: the hit-reaction pose and the stance alternate this often.
pub const KESSEN_SWAY_FRAMES: u8 = 12;

/// One hit's stun: the points it adds and the frames it adds to the timer
/// that keeps the total (Super Turbo's "dizzy time").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StunHit {
    pub points: u8,
    pub time: u8,
}

const fn hit(points: u8, time: u8) -> StunHit {
    StunHit { points, time }
}

/// Ryu's `SF KESSEN` stun, from his Super Turbo move list (SuperCombo,
/// `Super_Street_Fighter_2_Turbo/Ryu`, the Dizzy and Dizzy Time columns,
/// read 2026-11-18). Each value is the middle of the listed range, rounded
/// down. The playtest may change any of them.
pub mod ryu {
    use super::{hit, StunHit};

    /// Light normals, 0~5.
    pub const LIGHT: StunHit = hit(2, 40);
    /// Standing and crouching medium normals, 5~11.
    pub const MEDIUM: StunHit = hit(8, 60);
    /// Close standing and crouching heavy punch, first hitbox 3~9.
    pub const CLOSE_HEAVY_PUNCH: StunHit = hit(6, 40);
    /// Far standing heavy punch and far heavy kick, 10~16.
    pub const FAR_HEAVY: StunHit = hit(13, 80);
    /// Close heavy kick: the first hit 10~16, the second 1~7.
    pub const CLOSE_HEAVY_KICK: [StunHit; 2] = [hit(13, 80), hit(4, 70)];
    /// Crouching heavy kick, 5~11 with the longest timer.
    pub const SWEEP: StunHit = hit(8, 130);
    /// Neutral jumping light punch and kick, and diagonal ones, 1~7.
    pub const JUMP_LIGHT: StunHit = hit(4, 40);
    /// Neutral and diagonal jumping medium kick, and neutral medium punch, 5~11.
    pub const JUMP_MEDIUM: StunHit = hit(8, 50);
    /// Diagonal jumping medium punch: the first hit 3~9, the second 0~5.
    pub const DIAGONAL_JUMP_MEDIUM_PUNCH: [StunHit; 2] = [hit(6, 40), hit(2, 20)];
    /// Neutral jumping heavy punch and kick, 11~17.
    pub const NEUTRAL_JUMP_HEAVY: StunHit = hit(14, 60);
    /// Diagonal jumping heavy punch and kick, 3~9 (Ryu's nerfed ones).
    pub const DIAGONAL_JUMP_HEAVY: StunHit = hit(6, 40);
    /// Collarbone Breaker and Solar Plexus Smasher, each hit 1~7.
    pub const COMMAND_NORMAL: StunHit = hit(4, 70);
    /// Shoulder and circle throws, 7~13.
    pub const THROW: StunHit = hit(10, 100);
    /// Hadoken, 7~13.
    pub const HADOKEN: StunHit = hit(10, 110);
    /// Red Hadoken, 5~11.
    pub const RED_HADOKEN: StunHit = hit(8, 100);
    /// Shoryuken: the grounded hitbox 9~15, the rising one 2~8.
    pub const SHORYUKEN: [StunHit; 2] = [hit(12, 100), hit(5, 90)];
    /// Tatsumaki Senpukyaku, grounded or in the air, each hit 3~9.
    pub const TATSUMAKI: StunHit = hit(6, 90);
    /// Shinku Hadoken, each hit 0~4.
    pub const SHINKU_HADOKEN: StunHit = hit(2, 20);
}

/// A fighter's stun: what he has built, and the dizzy it leads to. Every
/// era fighter holds one, since a Kessen attacker dizzies any style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StunState {
    /// The stun built so far.
    pub(crate) points: u8,
    /// Frames until the built stun empties; each hit adds its move's time.
    pub(crate) timer: u16,
    /// Dizzies taken this match: they raise the limit.
    pub(crate) dizzies: u8,
    /// Frames after a dizzy in which hits add no stun.
    pub(crate) rest: u8,
    /// The dizzy now running came from stun (it sways, and counts down
    /// only while he stands in it).
    pub(crate) dizzied: bool,
    /// The attacker's side: the stun hits his current move has landed,
    /// for moves whose later hits deal less.
    pub(crate) move_hits: u8,
}

crate::canonical::canonical_struct!(StunState, "StunState.v1", {
    points, timer, dizzies, rest, dizzied, move_hits,
});

impl StunState {
    /// A new round: the built stun goes, the match's dizzy count stays.
    pub(crate) fn next_round(self) -> Self {
        Self { dizzies: self.dizzies, ..Self::default() }
    }

    pub(crate) fn limit(&self) -> u8 {
        KESSEN_STUN_LIMITS[usize::from(self.dizzies).min(KESSEN_STUN_LIMITS.len() - 1)]
    }

    /// Add a hit's stun. True when it fills the limit: the stun empties
    /// and the dizzy count rises, and the caller starts the dizzy.
    pub(crate) fn add(&mut self, stun: StunHit) -> bool {
        if self.rest != 0 {
            return false;
        }
        self.points = self.points.saturating_add(stun.points);
        self.timer = self.timer.saturating_add(u16::from(stun.time));
        if self.points < self.limit() {
            return false;
        }
        self.points = 0;
        self.timer = 0;
        self.dizzies = self.dizzies.saturating_add(1);
        self.dizzied = true;
        true
    }

    /// One frame: the timer runs down and empties the total when it ends.
    pub(crate) fn tick(&mut self) {
        self.rest = self.rest.saturating_sub(1);
        if self.timer != 0 {
            self.timer -= 1;
            if self.timer == 0 {
                self.points = 0;
            }
        }
    }

    /// The stun dizzy has ended, by time or by a hit.
    pub(crate) fn recovered(&mut self) {
        if std::mem::take(&mut self.dizzied) {
            self.rest = KESSEN_STUN_REST_FRAMES;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_hadokens_fill_the_first_limit_and_the_next_limit_is_higher() {
        let mut stun = StunState::default();
        assert!(!stun.add(ryu::HADOKEN));
        assert!(!stun.add(ryu::HADOKEN));
        assert!(stun.add(ryu::HADOKEN));
        assert_eq!((stun.points, stun.limit()), (0, 34));
    }

    #[test]
    fn the_total_empties_when_the_timer_runs_out() {
        let mut stun = StunState::default();
        stun.add(ryu::LIGHT);
        for _ in 0..39 {
            stun.tick();
        }
        assert_eq!(stun.points, 2);
        stun.tick();
        assert_eq!(stun.points, 0);
    }

    #[test]
    fn no_stun_builds_while_resting_after_a_dizzy() {
        let mut stun = StunState { dizzied: true, ..StunState::default() };
        stun.recovered();
        assert!(!stun.add(ryu::HADOKEN));
        assert_eq!(stun.points, 0);
    }
}
