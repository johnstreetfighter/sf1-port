//! Hand and projectile cues for Ryu's Denjin techniques.

use crate::era_fight::sf3::ART_3;
use crate::fight_reader::MoveEffectCues;
use crate::fighter::Bus;
use crate::roster::TournamentFighter;

impl Bus<'_> {
    pub(super) fn collect_denjin_fighter(&self, cues: &mut MoveEffectCues, index: usize, entity: u16) {
        let Some(state) = self.era_state(self.el(entity, 0x38)) else { return };
        if state.fighter != Some(TournamentFighter::Ryu) {
            return;
        }
        let move_id = self.eb(entity, 0xa);
        if state.sf6_denjin_charging(move_id) {
            let hands = self.denjin_anchor(entity, (0, -16));
            cues.denjin_hands[index] = Some(hands);
            // The first header has no hold count until the runner activates
            // it. A live header at zero is on its last charge tick.
            if self.eb(entity, 8) == 2 && self.eb(entity, 0x33) == 0 {
                cues.denjin_charge_snap[index] = Some(hands);
            }
            return;
        }
        let palm = state.sf6_denjin_palm(move_id);
        if !palm && !self.mark_denjin_projectile(self.el(entity, 0x38), move_id) {
            return;
        }

        // Recover the original 32 px grid code from the facing-left column,
        // then attach the effect to the visible hands on that ROM pose.
        let mut code = self.ew(entity, 0);
        if self.ew(entity, 2) & 0x100 != 0 {
            code = code.wrapping_sub(u16::from(self.eb(entity, 0xd)) * 2);
        }
        match (palm, code) {
            (true, 0x0288) => cues.denjin_hands[index] = Some(self.denjin_anchor(entity, (18, -39))),
            (true, 0x022a) => cues.denjin_release[index] = Some(self.denjin_anchor(entity, (42, -30))),
            (false, 0x04a4) => cues.denjin_hands[index] = Some(self.denjin_anchor(entity, (-9, -16))),
            (false, 0x04a8) => cues.denjin_hands[index] = Some(self.denjin_anchor(entity, (21, -15))),
            (false, 0x042a) => cues.denjin_release[index] = Some(self.denjin_anchor(entity, (31, -20))),
            _ => {}
        }
    }

    fn denjin_anchor(&self, entity: u16, (dx, dy): (i32, i32)) -> (i32, i32, bool) {
        let facing_left = self.eb(entity, 0x37) & 4 != 0;
        let cx = i32::from(self.center_x(entity) as i16);
        let cy = i32::from(self.center_y(entity) as i16);
        let (x, y) = self.effect_screen_point(cx + if facing_left { -dx } else { dx }, cy + dy);
        (x, y, facing_left ^ *self.flip_screen)
    }

    pub(crate) fn mark_denjin_projectile(&self, ps: u32, move_id: u8) -> bool {
        self.era_state(ps).is_some_and(|state| {
            state.fighter == Some(TournamentFighter::Ryu)
                && (state.sf6_denjin_projectile(move_id)
                    || (state.is_sf3() && move_id == ART_3 && state.alpha_move_active(move_id)))
        })
    }
}
