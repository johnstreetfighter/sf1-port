use sf_video::move_effects::{
    accent::AccentKind,
    signature::{ImpactKind, ProjectileKind},
};

use super::{Bus, FighterAccent};
use crate::data_bank::{SF2_SPECIAL_SLOT, SF2_SUPER_SLOT};
use crate::era::EraFamily::{Sf2, Sf3, Sf4, Sf5, Sf6, SfZ};
use crate::era_fight::{sf4, sf5, sf6};
use crate::roster::TournamentFighter::{Ryu, Sagat};

impl Bus<'_> {
    pub(super) fn power_projectile(&self, ps: u32, slot: u8) -> Option<ProjectileKind> {
        let era = self.era_state(ps)?;
        match (era.fighter?, era.style.family, slot) {
            (Ryu, Sf2, SF2_SPECIAL_SLOT) if era.is_sf2() => Some(ProjectileKind::Shakunetsu),
            (Ryu, Sf4, 0x2a) if era.alpha_move_active(slot) => Some(ProjectileKind::Metsu),
            (Ryu, Sf2 | SfZ | Sf3 | Sf4 | Sf5 | Sf6, SF2_SUPER_SLOT)
                if self.evil_ryu_profile(ps).is_none()
                    && (era.is_sf2() || era.alpha_move_active(slot)) =>
            {
                Some(ProjectileKind::Shinku)
            }
            (Sagat, SfZ, 0x2a) if era.alpha_super_level(slot) != 0 => {
                Some(ProjectileKind::TigerCannon)
            }
            (Sagat, Sf4, 0x2b) | (Sagat, Sf5, sf5::COUNTER_STRIKE)
                if era.alpha_move_active(slot) =>
            {
                Some(ProjectileKind::TigerCannon)
            }
            (Sagat, Sf3 | Sf6, _) if era.what_if_others_projectile_hits(slot).is_some() => {
                Some(ProjectileKind::TigerCannon)
            }
            _ => None,
        }
    }

    pub(super) fn collect_power_effect(&self, entity: u16) -> Option<FighterAccent> {
        let ps = self.el(entity, 0x38);
        let era = self.era_state(ps)?;
        let slot = self.eb(entity, 0xa);
        if self.eb(entity, 8) != 2 {
            return None;
        }
        if era.alpha_move_active(slot) {
            let system = match (era.style.family, slot) {
                (Sf6, sf6::IMPACT) => {
                    // The live bit clears on contact. The strike record remains current
                    // through the rest of the swing and has a banked hit function.
                    if crate::data_bank::in_bank(self.el(entity, 0x28)) {
                        let (x, y, flip) = self.attack_accent_anchor(entity);
                        return Some((AccentKind::DriveImpactSwing, x + if flip { -8 } else { 8 }, y - 16, flip));
                    }
                    (!era.attack_reached_active).then_some((AccentKind::DriveImpact, 4, -25))
                }
                (Sf6, sf6::RUSH) => Some((AccentKind::DriveRush, -11, 12)),
                (Sf4, sf4::FOCUS_CHARGE | sf4::FOCUS) => Some((AccentKind::FocusInk, 2, -6)),
                _ => None,
            };
            if let Some((kind, dx, dy)) = system {
                let (x, y, flip) = self.move_accent_anchor(entity, dx, dy);
                return Some((kind, x, y, flip));
            }
        }
        if self.eb(entity, 0xf) & 0x80 == 0 {
            return None;
        }
        let kind = match (era.fighter?, era.style.family, slot) {
            (Ryu, Sf3, 0x2a) | (Ryu, Sf4 | Sf6, 0x2b)
                if era.alpha_move_active(slot) && self.evil_ryu_profile(ps).is_none() =>
            {
                if era.is_sf4() {
                    AccentKind::MetsuRise
                } else {
                    AccentKind::ShinRise
                }
            }
            (Sagat, Sf2 | SfZ | Sf3 | Sf4 | Sf5 | Sf6, 0x19) => AccentKind::TigerKnee,
            (Sagat, Sf5, sf5::EX_KICK) if era.alpha_move_active(slot) => AccentKind::TigerKnee,
            (Sagat, SfZ | Sf3 | Sf4 | Sf6, 0x1f) if era.alpha_move_active(slot) => {
                AccentKind::TigerRise
            }
            (Sagat, Sf5, sf5::UPPERCUT | sf5::EX_RISE) if era.alpha_move_active(slot) => {
                AccentKind::TigerRise
            }
            (Sagat, SfZ | Sf4, 0x29) | (Sagat, Sf4, 0x2a) if era.alpha_move_active(slot) => {
                AccentKind::TigerRise
            }
            _ => return None,
        };
        let (x, y, flip) = self.attack_accent_anchor(entity);
        Some((kind, x, y, flip))
    }

    pub(in crate::fighter) fn record_drive_impact_contact(
        &mut self,
        attacker: u16,
        defender: u16,
        guarded: bool,
    ) {
        let owner = self.el(attacker, 0x38);
        if self.eb(attacker, 0xa) != sf6::IMPACT || !self.move_effects_for(owner)
            || !self.era_state(owner).is_some_and(|era| era.is_sf6() && era.alpha_move_active(sf6::IMPACT))
        {
            return;
        }
        let Some(side) = self.effect_fighter_side(attacker) else { return };
        let attack_x = self.center_x(attacker).wrapping_add(super::sext8(self.eb(attacker, 0x24)));
        let attack_y = self.center_y(attacker).wrapping_sub(super::sext8(self.eb(attacker, 0x22)));
        let hurt_x = self.center_x(defender).wrapping_add(super::sext8(self.eb(defender, 0x20)));
        let hurt_y = self.center_y(defender).wrapping_sub(super::sext8(self.eb(defender, 0x1e)));
        let overlap = |attack: u16, attack_half: u8, hurt: u16, hurt_half: u8| {
            let (attack, hurt) = (attack as i16, hurt as i16);
            let (attack_half, hurt_half) = (i16::from(attack_half), i16::from(hurt_half));
            let start = (attack - attack_half).max(hurt - hurt_half);
            let end = (attack + attack_half).min(hurt + hurt_half);
            ((start + end) / 2) as u16
        };
        let x = overlap(attack_x, self.eb(attacker, 0x25), hurt_x, self.eb(defender, 0x21));
        let y = overlap(attack_y, self.eb(attacker, 0x23), hurt_y, self.eb(defender, 0x1f));
        let (world_x, y) = self.contact_point(x, y);
        let flip = self.ew(attacker, 2) & 0x100 != 0;
        let kind = if guarded { ImpactKind::DriveImpactGuard } else { ImpactKind::DriveImpactHit };
        self.toggle_state.era.move_effects.drive_impact_contacts[side] = Some(crate::fight_reader::SignatureImpact {
            kind, world_x, y, flip,
        });
    }

    pub(in crate::fighter) fn has_drive_impact_contact(&self, attacker: u16) -> bool {
        self.eb(attacker, 0xa) == sf6::IMPACT
            && self.effect_fighter_side(attacker).is_some_and(|side| {
                self.toggle_state.era.move_effects.drive_impact_contacts[side].is_some()
            })
    }
}
