//! Timing and projectile geometry measured for New Ryu in World Super Turbo.

use super::Bus;
use crate::data_bank::st_ryu_normals::{CLOSE_HK, CLOSE_LK, CLOSE_MK};
use crate::era_fight::{EraProjectileState, SF2_SUPER, ST_RYU_SHINKU_FREEZE};

/// Input checks the source timer before its seven post-entry decrements.
/// The port decrements before input, so seed it with one additional tick.
const GROUNDED_SUPER_CANCEL: u8 = 8;

impl Bus<'_> {
    pub(super) fn st_ryu_air_reaction(&self, defender: u16) -> bool {
        self.airborne_reaction(defender)
            && self.era_state(self.el(defender, 0x38)).is_some_and(|state| state.st_ryu_air_reaction)
    }

    pub(super) fn st_ryu_air_juggle_allowed(&self, defender: u16, limit: u8) -> bool {
        self.st_ryu_air_reaction(defender) && self.era_can_juggle(defender, limit)
    }

    pub(super) fn st_ryu_shinku(&self, player: u32, move_id: u8) -> bool {
        move_id == SF2_SUPER && self.st_new_ryu(player)
    }

    pub(super) fn st_ken_shoryureppa(&self, player: u32, move_id: u8) -> bool {
        move_id == SF2_SUPER && self.sf2_state(player).is_some_and(|era| {
            era.fighter == Some(crate::roster::TournamentFighter::Ken)
                && era.style.revision == crate::era::Revision::St
                && era.style.loadout == crate::era::Loadout::Standard
        })
    }

    pub(super) fn st_grounded_super_cancelable(&self, fighter: u16) -> bool {
        self.eb(fighter, 9) & 0xc0 == 0xc0
            && self.toggle_state.combo.st_grounded_super_cancel[self.combo_slot(fighter)] != 0
    }

    pub(super) fn st_grounded_super_cancel_clock(&self, fighter: u16, move_id: u8) -> u8 {
        let normal = matches!(move_id, 0x0d | 0x0f..=0x18 | 0x1c..=0x1e | CLOSE_LK | CLOSE_MK | CLOSE_HK);
        let player = self.el(fighter, 0x38);
        let ken_kick = self.st_ken_shoryureppa(player, SF2_SUPER)
            && self.st_ken_crouching_kick(player, fighter, move_id).is_some();
        let sagat_crouching_normal = self.st_sagat_genocide(player, SF2_SUPER)
            && self.st_sagat_crouching_normal(player, move_id, self.eb(fighter, 0x34)).is_some();
        if (normal && self.st_new_ryu(player)) || ken_kick || sagat_crouching_normal {
            GROUNDED_SUPER_CANCEL
        } else {
            0
        }
    }

    pub(super) fn try_st_grounded_super_cancel(&mut self, player: u32, fighter: u16) -> bool {
        if !self.st_grounded_super_cancelable(fighter) {
            return false;
        }
        let kick = if self.st_sagat_genocide(player, SF2_SUPER) {
            let Some(kick) = [true, false].into_iter()
                .find(|&kick| self.era_special_command(player, kick) == Some(SF2_SUPER))
            else { return false };
            kick
        } else {
            if !(self.st_new_ryu(player) || self.st_ken_shoryureppa(player, SF2_SUPER))
                || !self.latch_fires(player, false)
                || self.era_special_command(player, false) != Some(SF2_SUPER)
            {
                return false;
            }
            false
        };
        if !self.start_sf2_special(player, fighter, SF2_SUPER, kick) {
            return false;
        }
        let slot = self.combo_slot(fighter);
        let count = &mut self.toggle_state.cancel.count[slot];
        *count = count.wrapping_add(1);
        true
    }

    /// The ordinary Uppercut route is still available after the super gate closes.
    pub(super) fn try_st_sagat_late_super_as_uppercut(&mut self, player: u32, fighter: u16, direction: u8) -> bool {
        let move_id = self.eb(fighter, 0xa);
        let slot = self.combo_slot(fighter);
        if direction != crate::moves::Dir::Forward.bits()
            || !matches!(move_id, 0x11..=0x13)
            || self.toggle_state.cancel.window[slot] == 0
            || self.toggle_state.cancel.move_id[slot] != move_id
            || self.st_grounded_super_cancelable(fighter)
            || !self.latch_fires(player, false)
            || self.sf2_state(player).and_then(|era| era.st_sagat_genocide_strength(false)).is_none()
        {
            return false;
        }
        if !self.start_sf2_special(player, fighter, crate::era_fight::SF2_EXTRA_SPECIAL, false) {
            return false;
        }
        self.era_consume_super_motion(player);
        let count = &mut self.toggle_state.cancel.count[slot];
        *count = count.wrapping_add(1);
        true
    }

    pub(crate) fn st_pending_super_flash(&mut self) {
        if self.era_frozen() {
            return;
        }
        for player in self.era_player_slots().into_iter().flatten() {
            let (fighter, _) = self.player_entities(player);
            let ready = self.player_slot_index(player).is_some_and(|side| {
                self.toggle_state.human_attack_frames[side] == 9
            });
            if self.contact_frozen(fighter)
                || !self.sf2_state(player).is_some_and(|state| state.super_stock_full())
            {
                continue;
            }
            let move_id = self.eb(fighter, 0xa);
            if ready && self.st_ryu_shinku(player, move_id) {
                self.era_spend_super_with_freeze(player, ST_RYU_SHINKU_FREEZE);
                return;
            }
            if self.eb(fighter, 8) == 1 && self.st_ken_shoryureppa(player, move_id) {
                self.era_spend_super_with_freeze(player, crate::era_fight::ST_KEN_SHORYUREPPA_FREEZE);
                return;
            }
            if self.eb(fighter, 8) == 1 && self.st_sagat_genocide(player, move_id) {
                self.era_spend_super_with_freeze(player, crate::era_fight::ST_SAGAT_GENOCIDE_FREEZE);
                return;
            }
        }
    }

    pub(super) fn st_ryu_shinku_reaches_ken(
        &self,
        defender: u16,
        projectile: EraProjectileState,
        (x, y, width, height): (u16, u16, u16, u16),
    ) -> Option<bool> {
        if !self.st_new_ken(defender) {
            return None;
        }
        if self.st_ryu_air_reaction(defender) {
            let tx = self.center_x(defender)
                .wrapping_add_signed(i16::from(self.eb(defender, 0x20) as i8));
            let ty = self.center_y(defender)
                .wrapping_add_signed(-i16::from(self.eb(defender, 0x1e) as i8));
            let rx = u16::from(self.eb(defender, 0x21));
            let ry = u16::from(self.eb(defender, 0x1f));
            if rx != 0 && ry != 0 {
                return Some(rx + width >= Self::borrow_abs(tx, x)
                    && ry + height >= Self::borrow_abs(ty, y));
            }
        }
        if self.era_airborne(defender) {
            return None;
        }
        let torso = match self.eb(defender, 0xa) {
            1..=4 => (6, 52, 20, 21),
            5..=8 => (-6, 33, 22, 12),
            0x31 if self.contact_frozen(defender) => match projectile.hits_remaining {
                4 | 2 => (10, 47, 21, 21),
                3 => (13, 50, 20, 23),
                1 => (33, 47, 20, 23),
                _ => return None,
            },
            _ => return None,
        };
        let axis = self.fighter_axis_x(defender);
        let facing = if self.eb(defender, 0x37) & 4 != 0 { 1 } else { -1 };
        let overlaps = |(offset, above_floor, rx, ry): (i16, u16, u16, u16)| {
            let tx = axis.wrapping_add_signed(offset * facing);
            let ty = crate::data_bank::GROUND as u16 - above_floor;
            rx + width >= Self::borrow_abs(tx, x) && ry + height >= Self::borrow_abs(ty, y)
        };
        let crouching_head = matches!(self.eb(defender, 0xa), 5..=8)
            && overlaps((-14, 51, 12, 9));
        Some(overlaps(torso) || crouching_head)
    }
}
