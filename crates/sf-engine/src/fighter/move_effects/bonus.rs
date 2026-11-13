//! Cues for Geki's ninja arts and Evil Ryu's Satsui no Hado.

use sf_video::move_effects::accent::AccentKind;

use super::super::{Bus, DEMON_WHIFF_FRAMES, EVIL_RYU_ASHURA_FORWARD};
use crate::data_bank::{
    KAGE_ASHURA_AIR_BACK, KAGE_ASHURA_AIR_FORWARD, KAGE_ASHURA_BACK, KAGE_ASHURA_FORWARD,
    SF6_ASHURA_GLIDE,
};
use crate::era_fight::sf6::ASHURA_BACK;
use crate::fight_reader::FighterAccent;
use crate::roster::TournamentFighter;

const SHURIKEN: u8 = 0x1a;
const TELEPORT_STRIKE: u8 = 0x1b;
const UTSU_AIR: u8 = 0x21;
const UTSU_GROUND: u8 = 0x22;
const SHURIKEN_VOLLEY: u8 = 0x24;
const MUGEN_SHURIKEN: u8 = 0x29;
const UTSU_SMOKE: u32 = 0xb8be;

impl Bus<'_> {
    pub(super) fn is_geki_shuriken(&self, ps: u32, move_id: u8) -> bool {
        self.era_state(ps).is_some_and(|state| {
            state.fighter == Some(TournamentFighter::Geki)
                && (move_id == SHURIKEN
                    || (matches!(move_id, SHURIKEN_VOLLEY | MUGEN_SHURIKEN)
                        && state.what_if_active()
                        && state.alpha_move_active(move_id)))
        })
    }

    pub(super) fn collect_bonus_effect(&self, entity: u16) -> Option<FighterAccent> {
        let ps = self.el(entity, 0x38);
        let state = self.era_state(ps)?;
        let slot = self.eb(entity, 0xa);
        let flags = self.eb(entity, 0xf);

        if state.fighter == Some(TournamentFighter::Geki) {
            // Teleport Strike keeps the ROM smoke's unhittable flag at both
            // ends of the warp. The What-If Utsus use its tile grid directly.
            let teleport_smoke = slot == TELEPORT_STRIKE && flags & 8 != 0;
            let utsu_smoke = matches!(slot, UTSU_AIR | UTSU_GROUND)
                && state.what_if_active()
                && state.alpha_move_active(slot)
                && self
                    .el(entity, 0x2c)
                    .checked_sub(0x1e)
                    .is_some_and(|tile| self.rw(tile) == self.rw(UTSU_SMOKE + 2));
            if teleport_smoke || utsu_smoke {
                let (x, y, flip) = self.move_accent_anchor(entity, 0, -4);
                return Some((AccentKind::GekiVanish, x, y, flip));
            }
        }

        let demon_rush = self.evil_ryu_has_demon(ps)
            && slot == self.demon_rush_move(ps)
            && self.eb(entity, 0x33) > DEMON_WHIFF_FRAMES;
        let backwards = if demon_rush {
            false
        } else if flags & 8 != 0 && self.legacy_evil_ryu(ps) && self.is_ashura_move(ps, slot) {
            slot != EVIL_RYU_ASHURA_FORWARD
        } else if flags & 8 != 0
            && state.is_kage()
            && state.alpha_move_active(slot)
            && matches!(
                slot,
                KAGE_ASHURA_FORWARD
                    | KAGE_ASHURA_BACK
                    | KAGE_ASHURA_AIR_FORWARD
                    | KAGE_ASHURA_AIR_BACK
            )
        {
            matches!(slot, KAGE_ASHURA_BACK | KAGE_ASHURA_AIR_BACK)
        } else if flags & SF6_ASHURA_GLIDE != 0 && state.sf6_evil_ryu_ashura(slot) {
            matches!(slot, ASHURA_BACK)
        } else {
            return None;
        };

        let (x, y, flip) = self.move_accent_anchor(entity, if backwards { 7 } else { -7 }, 15);
        Some((AccentKind::SatsuiGlide, x, y, flip ^ backwards))
    }
}
