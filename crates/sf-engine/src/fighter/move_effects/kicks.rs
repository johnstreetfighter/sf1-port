use sf_video::move_effects::accent::AccentKind;

use super::Bus;
use crate::era::EraFamily;
use crate::era_fight::sf5::{CRITICAL_ART, TRIGGER_ATTACK};
use crate::fight_reader::FighterAccent;
use crate::roster::TournamentFighter;

const ADON_FLYING_KICK: u16 = 0x32a8;
const ADON_ROUNDHOUSE: u16 = 0x3184;
const ADON_RISING_HIT: u32 = 0x47cf2;
const KEN_LOW_KICK: u16 = 0x0b44;
const KEN_HIGH_KICK: u16 = 0x0ae6;
const KEN_SPIN_KICK: u16 = 0x0da0;
const KEN_UPPERCUT: u16 = 0x0d04;
const KEN_UPPERCUT_RISE: u16 = 0x0c8c;

impl Bus<'_> {
    pub(super) fn collect_kick_effects(&self, entity: u16) -> Option<FighterAccent> {
        use AccentKind::*;
        use EraFamily::*;

        let era = self.era_state(self.el(entity, 0x38))?;
        let slot = self.eb(entity, 0xa);
        let mut pose = self.ew(entity, 0);
        if self.ew(entity, 2) & 0x100 != 0 {
            // These poses use 32px cells. The left-facing grid starts at the
            // rightmost column of the original drawing.
            pose = pose.wrapping_sub(2 * u16::from(self.eb(entity, 0xd)));
        }

        // Offsets are measured from the pose grid's centre to the heel.
        // Matching the drawn strike keeps it visible when a hit clears the
        // active collision bit partway through that pose.
        let (kind, dx, dy) = match (era.fighter?, era.style.family, slot, pose) {
            (TournamentFighter::Ken, SfZ | Sf3 | Sf4, 0x2a, KEN_UPPERCUT)
            | (TournamentFighter::Ken, Sf5, TRIGGER_ATTACK, KEN_UPPERCUT)
                if era.alpha_move_active(slot) =>
            {
                (Shinryuken, 14, -6)
            }
            (TournamentFighter::Ken, SfZ | Sf3 | Sf4, 0x2a, KEN_UPPERCUT_RISE)
            | (TournamentFighter::Ken, Sf5, TRIGGER_ATTACK, KEN_UPPERCUT_RISE)
                if era.alpha_move_active(slot) =>
            {
                (Shinryuken, 3, -16)
            }
            (TournamentFighter::Ken, Sf6, 0x2a, pose) if era.alpha_move_active(slot) => {
                match pose {
                    KEN_LOW_KICK => (ShippuKick, 24, -13),
                    KEN_HIGH_KICK => (ShippuKick, 36, -30),
                    KEN_SPIN_KICK => (ShippuSpiral, 13, -3),
                    _ => return None,
                }
            }
            (TournamentFighter::Ken, Sf6, 0x29, KEN_HIGH_KICK) if era.alpha_move_active(slot) => {
                (DragonlashFlame, 35, -28)
            }
            (TournamentFighter::Ken, Sf4, 0x2b, pose)
            | (TournamentFighter::Ken, Sf5, CRITICAL_ART, pose)
                if era.alpha_move_active(slot) =>
            {
                match pose {
                    KEN_LOW_KICK => (ShippuKick, 24, -13),
                    KEN_HIGH_KICK => (ShippuKick, 36, -30),
                    KEN_SPIN_KICK => (GurenSweep, 19, 3),
                    _ => return None,
                }
            }
            (TournamentFighter::Adon, family, slot, pose) if family != Sf1987 => {
                match (family, slot, pose) {
                    (_, 0x1b, ADON_FLYING_KICK)
                    | (SfZ, 0x2c | 0x2d, ADON_FLYING_KICK)
                    | (Sf2 | Sf3 | Sf4 | Sf5 | Sf6, 0x20 | 0x21, ADON_FLYING_KICK)
                    | (Sf5, 0x2c, ADON_FLYING_KICK) => (JaguarKick, 35, 20),
                    (SfZ | Sf3 | Sf4 | Sf6, 0x2a, ADON_FLYING_KICK)
                    | (Sf3 | Sf4 | Sf6, 0x2b, ADON_FLYING_KICK) => (JaguarRevolver, 35, 20),
                    (SfZ | Sf3 | Sf4 | Sf6, 0x2a, ADON_ROUNDHOUSE)
                    | (Sf3 | Sf4 | Sf6, 0x2b, ADON_ROUNDHOUSE)
                    | (_, 0x29, ADON_ROUNDHOUSE) => (JaguarRise, 36, -33),
                    (_, 0x19, _) | (Sf5, 0x2b, _) if self.el(entity, 0x28) == ADON_RISING_HIT => {
                        (JaguarRise, 12, -12)
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };
        let (x, y, flip) = self.move_accent_anchor(entity, dx, dy);
        Some((kind, x, y, flip))
    }
}
