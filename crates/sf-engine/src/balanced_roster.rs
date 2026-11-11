//! Balanced roster tuning (ticket #268, `SP20`; spec #246, design §9.7).
//!
//! The per-fighter and per-move adjustments the Balanced roster switch
//! applies on top of the 1987 data, as plain tables the engine reads at
//! three points: when a stream frame loads ([`frame_patch`]), when a hit
//! lands ([`cpu_damage`], [`shoto_entry_damage`]) and when the true-combo
//! rule asks for a move's stun ([`second_hit_stun`]). With the switch off
//! none of these are read, so every value is the ROM's.
//!
//! Every row names its reason against the two documents it answers:
//! `docs/reference/rom-box-audit.md` (the SP3 box audit: boxes over
//! nothing, boxes past the drawing, point hurtboxes mid-move, critical boxes
//! outside the hurtbox, dormant CPU attack frames) and
//! `docs/research/sf1plus-reference-numbers.md` (the damage bands of a
//! 48-point life bar: lights 3–5, mediums 6–8, heavies 8–11, specials
//! 9–12; startup and recovery ranges). Animations and which moves exist are
//! never touched: only boxes, steps, durations, flags and damage.

use crate::combo::Stun;
use crate::roster::TournamentFighter;

/// A box in a frame record's order: `[dy, hh, dx, hw]` (signed bytes,
/// facing right; the engine flips `dx` for a fighter facing left).
pub type Box4 = [i8; 4];

/// What Balanced roster changes in one stream frame record (or the move
/// header of the same pose) once it has loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePatch {
    /// The frame record's ROM address.
    pub frame: u32,
    /// The critical (tight) hurtbox, entity `+0x1a..+0x1e`.
    pub tight: Option<Box4>,
    /// The hurtbox (loose), entity `+0x1e..+0x22`.
    pub loose: Option<Box4>,
    /// The attack box, entity `+0x22..+0x26`.
    pub attack: Option<Box4>,
    /// The body's front half-width (entity `+0x18`), which the push-apart
    /// keeps between the two fighters.
    pub body_front: Option<u8>,
    /// The frame's hold in ticks less one (entity `+0xe` / `+0x33`).
    pub duration: Option<u8>,
    /// Bits OR-ed into the frame flags (entity `+0xf`).
    pub flags_or: u8,
    /// Why, against the audit or the research.
    pub reason: &'static str,
}

const fn patch(frame: u32, reason: &'static str) -> FramePatch {
    FramePatch {
        frame,
        tight: None,
        loose: None,
        attack: None,
        body_front: None,
        duration: None,
        flags_or: 0,
        reason,
    }
}

impl FramePatch {
    const fn tight(mut self, b: Box4) -> Self {
        self.tight = Some(b);
        self
    }
    const fn loose(mut self, b: Box4) -> Self {
        self.loose = Some(b);
        self
    }
    const fn attack(mut self, b: Box4) -> Self {
        self.attack = Some(b);
        self
    }
    const fn body_front(mut self, w: u8) -> Self {
        self.body_front = Some(w);
        self
    }
    const fn duration(mut self, d: u8) -> Self {
        self.duration = Some(d);
        self
    }
    const fn flags_or(mut self, f: u8) -> Self {
        self.flags_or = f;
        self
    }
}

/// Joe's standing hurtboxes (his stance header `0x1737c`), which his guard
/// takes under Balanced roster.
pub const JOE_STANDING_TIGHT: Box4 = [36, 8, -4, 8];
/// See [`JOE_STANDING_TIGHT`].
pub const JOE_STANDING_LOOSE: Box4 = [0, 40, -8, 16];

const SHOTO_JUMP_BACK: &str = "audit: critical box outside the hurtbox (jump back): the tight box's dx brought inside the hurtbox";
const SHOTO_TATSU: &str = "audit: hurtbox 27 px past the drawing (Tatsumaki / jump HK): trimmed to the drawn body";
const LEE_AIR: &str = "audit: point hurtbox mid-move (Lee's jumping attacks): his standing hurtbox restored";
const LEE_LAND: &str = "audit: critical box at the hurtbox's edge (Lee's jump landing): hurtbox grown to hold it";
const GEN_HW: &str = "audit: hurtbox narrower than the drawing (Gen): half-width 8 -> 16";
const ADON_BODY: &str = "audit: drawing outgrows the hurtbox (Adon): hurtbox moved over the drawn body";

/// Every frame Balanced roster patches, by ROM frame address.
pub const FRAME_PATCHES: &[FramePatch] = &[
    // ---- SP5 follow-up: Joe's standing guard ----------------------------
    patch(0x0e370, "SP5/the maintainer: Joe's block pose's hurtbox covered only his lower two thirds, so standing punches passed over his guard; it takes his standing hurtbox")
        .tight(JOE_STANDING_TIGHT)
        .loose(JOE_STANDING_LOOSE),
    // ---- SP17 follow-up: Geki's close-spacing normals --------------------
    patch(0x0bb9a, "SP17/the maintainer: Geki's front kick fell 2 px short of a guarding Ken at the closest spacing; its reach grows to his jab's")
        .attack([12, 8, 36, 16]),
    patch(0x0afd2, "SP17/the maintainer + audit: Geki's forward walk draws him 24 px ahead of where he stands (x-offset +24), and its body kept that drawing apart from Ken, so he stopped 24 px short and every normal from the closest spacing whiffed a guard; the body's front edge pulls back by the offset, and the 64 px critical box matches his stance")
        .tight([16, 8, 8, 8])
        .body_front(0),
    patch(0x0aff2, "SP17/the maintainer: the walk's second frame draws him 16 px ahead; its body's front edge pulls back by the offset")
        .body_front(8),
    // ---- audit: attack box over nothing ---------------------------------
    patch(0x01178, "audit: attack box over nothing (Ryu's sweep): moved onto the drawn leg, reach kept")
        .attack([-16, 8, 24, 16]),
    patch(0x03256, "audit: attack box over nothing (Ken's sweep): moved onto the drawn leg, reach kept")
        .attack([-16, 8, 24, 16]),
    // ---- audit: attack box past the drawing / long reach ------------------
    patch(0x0cc2c, "audit: attack box 40 px past the drawing (Retsu's high kick): trimmed to the drawn foot")
        .attack([36, 8, 32, 12]),
    // Retsu's jab `0x0c9dc` (audit: 20 px past the drawing) stays as the
    // ROM has it: any trim leaves it short of a standing guard at the
    // spacing his jab-into-Kiai-Shot cancel works at (`tests/cancels.rs`).
    patch(0x115f2, "audit: attack box 22 px past the drawing (Birdie's low kick): trimmed to the drawn boot")
        .attack([-24, 16, 26, 12]),
    patch(0x0e792, "audit: long reach (Joe's low kick into a straight, 104 px): trimmed to 92")
        .attack([32, 8, 36, 8]),
    patch(0x14076, "audit: long reach (Sagat's high kick, 96 px): trimmed to 88")
        .attack([40, 16, 32, 16]),
    // ---- audit: point hurtbox mid-move ------------------------------------
    patch(0x0ba38, "audit: point hurtbox mid-move (Geki's crouching claw): his crouching hurtbox restored")
        .tight([8, 8, 0, 8])
        .loose([0, 40, 0, 16]),
    patch(0x0bb0a, "audit: point hurtbox mid-move (Geki's crouching claw): his crouching hurtbox restored")
        .tight([8, 8, 0, 8])
        .loose([0, 40, 0, 16]),
    patch(0x0d99c, "audit: point hurtbox mid-move (Mike's straight): his standing hurtbox restored")
        .tight([40, 8, 16, 8])
        .loose([0, 48, 8, 16]),
    patch(0x11542, "audit: point hurtbox mid-move (Birdie's hammer): his standing hurtbox restored")
        .tight([24, 8, 0, 8])
        .loose([-4, 60, -8, 16]),
    patch(0x11bf4, "audit: point hurtbox mid-move (Birdie's headbutt): a hurtbox over his drawn body")
        .loose([-4, 44, 8, 16]),
    patch(0x0f7ae, LEE_AIR).tight([24, 8, 0, 8]).loose([-4, 44, -8, 16]),
    patch(0x0f8da, LEE_AIR).tight([24, 8, 0, 8]).loose([-4, 44, -8, 16]),
    patch(0x0f950, LEE_AIR).tight([24, 8, 0, 8]).loose([-4, 44, -8, 16]),
    patch(0x0fa7c, LEE_AIR).tight([24, 8, 0, 8]).loose([-4, 44, -8, 16]),
    patch(0x0faa2, LEE_AIR).tight([24, 8, 0, 8]).loose([-4, 44, -8, 16]),
    patch(0x0faec, LEE_AIR).tight([24, 8, 0, 8]).loose([-4, 44, -8, 16]),
    patch(0x0fc3e, LEE_AIR).tight([24, 8, 0, 8]).loose([-4, 44, -8, 16]),
    // ---- audit: critical box outside the hurtbox / hurtbox vs drawing -----
    patch(0x0baa4, "audit: critical box 24 px outside the hurtbox (Geki's crouching claw): brought inside")
        .tight([8, 8, -8, 8])
        .loose([0, 24, -8, 16]),
    patch(0x0dba2, "audit: hurtbox half-height typo (Mike's hook recovery, hh 8 for 44): 84% of the drawing was outside it")
        .loose([-4, 44, 8, 16]),
    patch(0x008da, SHOTO_JUMP_BACK).tight([16, 8, 0, 8]),
    patch(0x008fa, SHOTO_JUMP_BACK).tight([16, 8, 0, 8]),
    patch(0x029b8, SHOTO_JUMP_BACK).tight([16, 8, 0, 8]),
    patch(0x029d8, SHOTO_JUMP_BACK).tight([16, 8, 0, 8]),
    patch(0x0f874, LEE_LAND).loose([8, 32, -8, 16]),
    patch(0x0f894, LEE_LAND).loose([8, 32, -8, 16]),
    patch(0x0fa16, LEE_LAND).loose([8, 32, -8, 16]),
    patch(0x0fa36, LEE_LAND).loose([8, 32, -8, 16]),
    patch(0x014d6, SHOTO_TATSU).tight([24, 8, 0, 8]).loose([0, 32, 0, 8]),
    patch(0x0196a, SHOTO_TATSU).tight([24, 8, 0, 8]).loose([0, 32, 0, 8]),
    patch(0x035b4, SHOTO_TATSU).tight([24, 8, 0, 8]).loose([0, 32, 0, 8]),
    patch(0x03a48, SHOTO_TATSU).tight([24, 8, 0, 8]).loose([0, 32, 0, 8]),
    patch(0x11422, "audit: hurtbox narrower than the drawing (Birdie's jab): half-width 8 -> 16")
        .loose([-8, 56, 0, 16]),
    patch(0x10486, GEN_HW).loose([0, 48, 0, 16]),
    patch(0x104f0, GEN_HW).loose([-4, 44, 0, 16]),
    patch(0x10536, GEN_HW).loose([-4, 44, 0, 16]),
    patch(0x105a6, GEN_HW).loose([0, 48, 0, 16]),
    patch(0x1068c, GEN_HW).loose([0, 48, 0, 16]),
    patch(0x1064c, GEN_HW).loose([8, 32, -8, 16]),
    patch(0x1066c, GEN_HW).loose([8, 32, -8, 16]),
    patch(0x133fc, ADON_BODY).loose([-8, 40, -8, 16]),
    patch(0x13442, ADON_BODY).loose([-8, 40, -8, 16]),
    patch(0x13850, ADON_BODY).loose([-8, 40, -8, 16]),
    // ---- audit: dormant CPU attack frames --------------------------------
    patch(0x103d6, "audit: dormant attack frame (Gen's CPU jab carries a box and a hit function with its active bit clear): made live")
        .flags_or(0x80),
    patch(0x13ec6, "audit: dormant attack frame (Sagat's CPU jab, the fair CPU's Tiger Shot stand-in): made live")
        .flags_or(0x80),
    // ---- research: startup ----------------------------------------------
    patch(0x144b4, "research: Sagat's CPU Tiger Shot spawned on tick 9, under the 12-16 tick projectile range; its wind-up holds 10 ticks so it spawns on tick 12, as the Hadoken")
        .duration(9),
];

/// The patch for the frame at `frame`, if Balanced roster changes it.
pub fn frame_patch(frame: u32) -> Option<&'static FramePatch> {
    FRAME_PATCHES.iter().find(|p| p.frame == frame)
}

/// Balanced damage of the 1987 CPU hit function `f` (the bosses' attacks,
/// also on their bank tables), as `(critical, other)`; `None` keeps the
/// ROM's value under the specials' cap of 12.
///
/// The research puts a 48-point bar's lights at 3–5, mediums at 6–8,
/// heavies at 8–11 and specials at 9–12; the late bosses' 1987 entries run
/// two to three times that, so each is brought into its band.
pub fn cpu_damage(f: u32) -> Option<(u8, u8)> {
    Some(match f {
        // Lights: 3-5.
        0x466b8 => (4, 3), // Retsu jab (6/4)
        0x4692e => (5, 4), // Mike jab (8/6)
        0x46ae6 => (4, 3), // Joe jab (4/2)
        0x46c9e => (4, 3), // Joe low kick (6/4)
        0x46e56 => (4, 3), // Lee palm (6/4)
        0x47234 => (4, 3), // Gen jab (8/6)
        0x475a4 => (5, 4), // Birdie jab (6/4)
        0x47ba8 => (5, 4), // Eagle jab (12/10)
        0x47c16 => (4, 3), // Adon jab (10/8)
        0x47e3c => (5, 4), // Adon low kick (14/12)
        0x47eaa => (5, 4), // Sagat jab (10/8)
        0x47f86 => (6, 5), // Sagat low kick (12/10)
        // Mediums: 6-8.
        0x46500 => (6, 5), // Geki crouching claw (6/4)
        0x4656e => (6, 5), // Geki front kick (4/3)
        0x46794 => (8, 7), // Retsu high/mid kick (12/10)
        0x46b54 => (7, 6), // Joe straight (10/8)
        0x46bc2 => (7, 6), // Joe body punch (10/8)
        0x46ec4 => (7, 6), // Lee straight (10/8)
        0x46fa0 => (7, 6), // Lee side kick (10/8)
        0x472a2 => (7, 6), // Gen straight (10/8)
        0x4737e => (7, 6), // Gen front kick (10/8)
        0x47612 => (8, 7), // Birdie hook (10/8)
        0x476ee => (7, 6), // Birdie low kick (10/8)
        0x479f0 => (7, 6), // Eagle low thrust (12/10)
        // Heavies: 8-11.
        0x46492 => (9, 7),  // Geki claw (10/6)
        0x465dc => (9, 7),  // Geki dive claw (10/6)
        0x4664a => (9, 8),  // Retsu palm (12/8)
        0x468de => (9, 8),  // Retsu sweep (10/8)
        0x4699c => (10, 9), // Mike ducking punch (18/16)
        0x46a78 => (11, 9), // Mike straight (18/16)
        0x46d0c => (9, 8),  // Joe high kick (10/8)
        0x4700e => (9, 8),  // Lee high kick (12/10)
        0x471c6 => (9, 8),  // Lee sweep (12/10)
        0x47536 => (9, 8),  // Gen sweep (14/12)
        0x47680 => (10, 8), // Birdie hammer (12/10)
        0x47914 => (9, 8),  // Eagle sticks (14/12)
        0x47c84 => (9, 8),  // Adon body punch (14/12)
        0x47f18 => (10, 8), // Sagat step punch (14/12)
        0x47ff4 => (10, 8), // Sagat high kick (14/12)
        _ => return None,
    })
}

/// The cap every other CPU hit (specials, air attacks) takes.
pub const CPU_DAMAGE_CAP: u8 = 12;

/// Balanced damage of a Ryu/Ken damage entry (the shared normal tables):
/// the 1987 light hits deal 1–2 of 48, under the research's light band.
pub fn shoto_entry_damage(entry: u32) -> Option<u8> {
    Some(match entry {
        0x45b6e => 3, // standing LP, shallow (1)
        0x45b76 => 4, // standing LP, deep (2)
        0x45de0 => 3, // crouching LP, shallow (2)
        0x462e0 => 3, // crouching LK, shallow (1)
        0x462e8 => 4, // crouching LK, deep (2)
        0x45eca => 3, // jumping LP, shallow (2)
        _ => return None,
    })
}

/// Joe's low kick into a straight (`0x15`, stream `0x0e706`): tuned from
/// its first strike in the true-combo rows, so a spaced hit on the second
/// strike alone was +6 on hit and +1 guarded. Balanced roster gives the
/// second strike (frames past `0x0e76c`) its own stun.
pub fn second_hit_stun(fighter: Option<TournamentFighter>, move_id: u8, stream: u32) -> Option<Stun> {
    (fighter == Some(TournamentFighter::Joe) && move_id == 0x15 && stream > 0x0e76c)
        .then_some(Stun { hit: 19, block: 14 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_frame_is_patched_once_and_says_why() {
        for (i, p) in FRAME_PATCHES.iter().enumerate() {
            assert!(!p.reason.is_empty());
            assert!(
                FRAME_PATCHES[i + 1..].iter().all(|q| q.frame != p.frame),
                "{:#07x} twice",
                p.frame
            );
            assert!(
                p.tight.is_some() || p.loose.is_some() || p.attack.is_some() || p.body_front.is_some() || p.duration.is_some() || p.flags_or != 0,
                "{:#07x} changes nothing",
                p.frame
            );
        }
    }

    #[test]
    fn the_damage_tables_sit_in_the_researchs_bands() {
        for f in 0x46400..0x48100u32 {
            if let Some((crit, other)) = cpu_damage(f) {
                assert!(other <= crit, "{f:#x}");
                assert!((3..=11).contains(&other) && (4..=11).contains(&crit), "{f:#x}");
            }
        }
        for e in [0x45b6e, 0x45b76, 0x45de0, 0x462e0, 0x462e8, 0x45eca] {
            assert!((3..=5).contains(&shoto_entry_damage(e).unwrap()));
        }
    }

    #[test]
    fn only_joes_second_strike_takes_its_own_stun() {
        let joe = Some(TournamentFighter::Joe);
        assert_eq!(second_hit_stun(joe, 0x15, 0x0e7b2), Some(Stun { hit: 19, block: 14 }));
        assert_eq!(second_hit_stun(joe, 0x15, 0x0e726), None);
        assert_eq!(second_hit_stun(joe, 0x16, 0x0e7b2), None);
        assert_eq!(second_hit_stun(Some(TournamentFighter::Ryu), 0x15, 0x0e7b2), None);
    }
}
