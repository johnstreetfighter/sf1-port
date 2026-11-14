use super::Bus;
use crate::combo::{StRyuPushback, StRyuRecoil, StSagatJump};
use crate::data_bank::st_sagat_air::{active, jump_position, landing, slot};
use crate::data_bank::{self, st_sagat_air};
use crate::era::{EraFamily, Loadout};
use crate::roster::TournamentFighter::Sagat;

impl Bus<'_> {
    pub(super) fn capture_st_sagat_jump_attack(&mut self, ps: u32, fighter: u16) -> bool {
        let slot = self.combo_slot(fighter);
        let Some(mut jump) = self.toggle_state.combo.st_sagat_jump[slot] else {
            return false;
        };
        let buttons = self.era_buttons(ps).1;
        if self.eb(fighter, 9) & 3 == 1 && jump.attack.is_none() && buttons != 0 {
            let button = buttons.trailing_zeros() as u8;
            let kick = button >= 3;
            let offset = u8::from(kick) * 3;
            let direction = jump.direction - 9;
            let move_id = if kick && direction == 0 && button == 5 {
                0x26
            } else {
                0x20 + offset + direction
            };
            jump.attack = Some((button, jump.tick + 1));
            self.toggle_state.combo.st_sagat_jump[slot] = Some(jump);
            self.set_eb(fighter, 0xa, move_id);
            self.set_eb(fighter, 0x34, button - offset + 1);
            self.set_eb(fighter, 9, 0xc1);
            self.ww(ps, 0);
            self.queue_sound(if kick { 0xa2 } else { 0xa1 });
        }
        true
    }

    pub(super) fn run_st_sagat_jump(&mut self, fighter: u16, defender: u16) -> bool {
        let ps = self.el(fighter, 0x38);
        let Some(era) = self.sf2_state(ps).filter(|era| era.st_sagat()) else {
            return false;
        };
        let old = era.style.loadout == Loadout::Old;
        let side = self.combo_slot(fighter);
        let move_id = self.eb(fighter, 0xa);
        let Some(mut jump) = self.toggle_state.combo.st_sagat_jump[side] else {
            if !self.is_player_driven(ps)
                || !matches!(move_id, 9..=0x0b)
                || self.eb(fighter, 8) != 1
            {
                return false;
            }
            let facing_left = self.eb(fighter, 0x37) & 4 != 0;
            self.toggle_state.combo.st_sagat_jump[side] = Some(StSagatJump {
                direction: move_id,
                facing_left,
                tick: 0,
                attack: None,
            });
            self.set_eb(fighter, 8, 2);
            self.set_eb(fighter, 9, 0x82);
            let axis = self.fighter_axis_x(fighter);
            self.load_frame(fighter, st_sagat_air::LAND);
            self.shift_grid_x(fighter, axis.wrapping_sub(self.fighter_axis_x(fighter)));
            self.set_st_ryu_knockdown_height(fighter, 0);
            return true;
        };
        jump.tick += 1;
        self.toggle_state.combo.st_sagat_jump[side] = Some(jump);
        let takeoff = if old { 2 } else { 4 };
        let flight = jump.tick.saturating_sub(takeoff);
        let land = landing(jump.direction);
        if flight == land + 7 {
            self.start_move(fighter, ps, 1);
            self.set_eb(fighter, 8, 2);
            return true;
        }
        let (x, height) = jump_position(jump.direction, flight);
        let (prev_x, _) = jump_position(jump.direction, flight.saturating_sub(1));
        let sign = if jump.facing_left { -1 } else { 1 };
        let dx = ((sign * x) >> 16) - ((sign * prev_x) >> 16);
        let mut frame = match jump.tick {
            tick if tick == takeoff => {
                self.set_eb(fighter, 9, 0x81);
                Some(st_sagat_air::LIFT)
            }
            _ if flight == land => {
                self.set_eb(fighter, 0xa, jump.direction);
                self.set_eb(fighter, 9, 0x82);
                Some(st_sagat_air::LAND)
            }
            _ => None,
        };
        if let Some((button, started)) = jump.attack.filter(|_| flight < land) {
            let directional = jump.direction != 9;
            let age = jump.tick - started;
            let live = active(button, directional, age);
            let was_live = age != 0 && active(button, directional, age - 1);
            if live && !was_live {
                frame = self.data_bank.and_then(|bank| {
                    bank.era_move_header(Sagat, EraFamily::Sf2, slot(button, directional, old))
                });
            } else if was_live && !live {
                frame = Some(st_sagat_air::LIFT);
            }
        }
        let axis = self.fighter_axis_x(fighter);
        if let Some(frame) = frame {
            self.load_frame(fighter, frame);
        }
        let target_x = axis.wrapping_add_signed(dx as i16);
        self.shift_grid_x(fighter, target_x.wrapping_sub(self.fighter_axis_x(fighter)));
        self.set_st_ryu_knockdown_height(fighter, height);
        if flight == land {
            self.finish_knockdown_dust(fighter);
        }
        self.push_apart(fighter, defender);
        self.clamp_x(fighter);
        self.judge_targets(fighter, defender);
        true
    }

    pub(super) fn st_sagat_air_entry(&self, fighter: u16) -> Option<u32> {
        let jump = self.toggle_state.combo.st_sagat_jump[self.combo_slot(fighter)]?;
        jump.attack?;
        let entry = self.el(fighter, 0x28);
        (matches!(self.eb(fighter, 0xa), 0x20..=0x26) && data_bank::in_bank(entry)).then_some(entry)
    }

    pub(super) fn push_st_sagat_jump_apart(&mut self, fighter: u16, other: u16) -> bool {
        let (sagat, ken) = if self.st_new_ken(other) {
            (fighter, other)
        } else {
            (other, fighter)
        };
        let Some(jump) = self.toggle_state.combo.st_sagat_jump[self.combo_slot(sagat)] else {
            return false;
        };
        if !self.st_new_ken(ken) || self.era_airborne(ken) {
            return false;
        }
        let old = self
            .sf2_state(self.el(sagat, 0x38))
            .is_some_and(|era| era.style.loadout == Loadout::Old);
        let takeoff = if old { 2 } else { 4 };
        let flight = jump.tick.saturating_sub(takeoff);
        let land = landing(jump.direction);
        // The low jump-in still uses the airborne pushbox. Landing installs
        // the ground box before the two fighters separate.
        if flight < land && jump.tick >= takeoff {
            return true;
        }
        if fighter == sagat {
            let gap = self
                .fighter_axis_x(ken)
                .wrapping_sub(self.fighter_axis_x(sagat)) as i16;
            let overlap = 39 - gap.abs();
            if overlap > 0 {
                let direction = if gap < 0 || gap == 0 && jump.facing_left {
                    -1
                } else {
                    1
                };
                self.shift_grid_x(sagat, (-direction * (overlap / 2)) as u16);
                self.shift_grid_x(ken, (direction * ((overlap + 1) / 2)) as u16);
                self.clamp_x(ken);
            }
        }
        true
    }

    pub(super) fn st_sagat_air_damage(
        &self,
        fighter: u16,
        defender: u16,
        entry: u32,
        old: bool,
    ) -> u16 {
        let Some(jump) = self.toggle_state.combo.st_sagat_jump[self.combo_slot(fighter)] else {
            return u16::from(self.rb(entry + 1));
        };
        match jump.attack {
            Some((4, _)) if jump.direction != 9 && self.st_new_ken(defender) => {
                match (old, self.eb(defender, 0xa)) {
                    (false, 5..=8) if self.pad_directions(self.el(defender, 0x38)) & 3 != 0 => 18,
                    (false, 5..=7) | (true, _) => 19,
                    (false, _) => 21,
                }
            }
            Some((0, _)) if jump.direction != 9 && old => 13,
            Some((5, _)) if jump.direction != 9 && old => 26,
            _ => u16::from(self.rb(entry + 1)),
        }
    }

    pub(super) fn st_sagat_air_recoil(
        &self,
        attacker: u16,
        defender: u16,
        guarded: bool,
    ) -> StRyuPushback {
        let direction = if self.fighter_axis_x(defender) < self.fighter_axis_x(attacker) {
            -1
        } else {
            1
        };
        let strength = self.eb(attacker, 0x34);
        let crouching = self.eb(defender, 9) & 7 == 4;
        let kind = if guarded || crouching {
            match strength {
                1 => StRyuRecoil::Light,
                2 => StRyuRecoil::Medium,
                _ => StRyuRecoil::Heavy,
            }
        } else {
            StRyuRecoil::Light
        };
        StRyuPushback {
            elapsed: 0,
            direction,
            kind,
        }
    }
}
