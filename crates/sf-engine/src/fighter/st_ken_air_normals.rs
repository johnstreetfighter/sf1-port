use super::Bus;
use crate::combo::{StKenJumpAttack, StRyuPushback, StRyuRecoil};
use crate::data_bank::{self, st_ken_air_normals};
use crate::era::Loadout;

impl Bus<'_> {
    pub(super) fn capture_st_ken_jump_attack(&mut self, ps: u32, fighter: u16) -> bool {
        let direction = self.eb(fighter, 0xa);
        if self.eb(fighter, 9) != 0x81 || self.st_ken_jump_header(ps, direction).is_none() {
            return false;
        }
        let buttons = self.era_buttons(ps).1;
        if buttons == 0 {
            return false;
        }
        let button = buttons.trailing_zeros() as u8;
        let kick = button >= 3;
        let strength = button % 3 + 1;
        let move_id = if direction == 9 && button == 5 {
            0x26
        } else {
            0x20 + u8::from(kick) * 3 + (direction - 9)
        };
        let slot = self.combo_slot(fighter);
        self.toggle_state.combo.st_ken_jump_attack[slot] = Some(StKenJumpAttack {
            direction,
            button,
            started: self.eb(fighter, 0x30).wrapping_add(1),
        });
        self.set_eb(fighter, 0xa, move_id);
        self.set_eb(fighter, 0x34, strength);
        self.set_eb(fighter, 9, 0xc1);
        self.queue_sound(if kick { 0xa2 } else { 0xa1 });
        true
    }

    pub(super) fn st_ken_jump_attack(&self, entity: u16) -> Option<StKenJumpAttack> {
        if !matches!(self.eb(entity, 0xa), 0x20..=0x26) {
            return None;
        }
        self.toggle_state.combo.st_ken_jump_attack[self.combo_slot(entity)]
    }

    /// The art overlay keeps the flight axis and floor height. When the
    /// unarmed stream advances, restore the previous frame's vertical pivot.
    pub(super) fn st_ken_jump_pivot(&self, entity: u16) -> Option<u16> {
        self.st_ken_jump_attack(entity)?;
        let frame = self.el(entity, 0x2c).checked_sub(0x20)?;
        if !data_bank::in_bank(frame) {
            return None;
        }
        let half = Self::half_extent(self.rb(frame + 8), self.rw(frame + 4) as i16 >= 0x400);
        Some(
            (data_bank::GROUND as u16)
                .wrapping_sub(self.st_ryu_height_above_floor(entity) as u16)
                .wrapping_sub(half),
        )
    }

    pub(super) fn draw_st_ken_jump_attack(&mut self, entity: u16) {
        let Some(attack) = self.st_ken_jump_attack(entity) else {
            return;
        };
        let side = self.combo_slot(entity);
        if self.eb(entity, 9) & 3 != 1 {
            self.toggle_state.combo.st_ken_jump_attack[side] = None;
            self.set_eb(entity, 0xa, attack.direction);
            return;
        }
        let ps = self.el(entity, 0x38);
        let old = self
            .sf2_state(ps)
            .is_some_and(|era| era.style.loadout == Loadout::Old);
        let Some(table) = self
            .data_bank
            .and_then(|bank| bank.st_ken_air_normal(old, attack.direction != 9, attack.button))
        else {
            return;
        };
        let age = self.eb(entity, 0x30).wrapping_sub(attack.started);
        let first = table + 1;
        let count = self.rb(table);
        let Some(index) = (0..count)
            .rev()
            .find(|&index| self.rb(first + u32::from(index) * 5) <= age)
        else {
            return;
        };
        let frame = self.rl(first + u32::from(index) * 5 + 1);
        let axis = self.fighter_axis_x(entity);
        let height = self.st_ryu_height_above_floor(entity);
        let attr = self.rw(frame + 4);
        let rows = self.rb(frame + 8);
        let cols = self.rb(frame + 9);
        self.set_eb(entity, 0xc, rows);
        self.set_eb(entity, 0xd, cols);
        let left = self.eb(entity, 0x37) & 4 != 0;
        let orient = |off: u32, value: u8| {
            if left && matches!(off, 0x1c | 0x20 | 0x24 | 0x26) {
                value.wrapping_neg()
            } else {
                value
            }
        };
        for index in 4..0x10 {
            let off = 0x16 + index;
            self.set_eb(entity, off, orient(off, self.rb(frame + 0xa + index)));
        }
        let offset = orient(0x26, self.rb(frame + 0x1b));
        self.set_eb(entity, 0x26, offset);
        let big = attr as i16 >= 0x400;
        let y = (data_bank::GROUND as u16)
            .wrapping_sub(height as u16)
            .wrapping_sub(2 * Self::half_extent(rows, big));
        let x = axis
            .wrapping_add_signed(i16::from(offset as i8))
            .wrapping_sub(Self::half_extent(cols, big));
        let code = self.rw(frame + 2);
        if left {
            self.write_grid_flipped(entity, code, attr, x, y);
        } else {
            self.write_grid(entity, code, attr, x, y);
        }
        let flags = if self.toggle_state.combo.connected[side] {
            0
        } else {
            self.rb(frame + 0x1a)
        };
        self.set_eb(entity, 0xf, flags);
        self.set_el(entity, 0x28, self.rl(frame + 0x1c));
    }

    pub(super) fn st_ken_air_normal_damage(&self, entity: u16, defender: u16) -> Option<u16> {
        let attack = self.st_ken_jump_attack(entity)?;
        let old = self.sf2_state(self.el(entity, 0x38))?.style.loadout == Loadout::Old;
        let crouching = !self.era_airborne(defender) && self.eb(defender, 9) & 7 == 4;
        Some(st_ken_air_normals::clean_hp(
            old,
            attack.direction,
            attack.button,
            crouching,
        ))
    }

    pub(super) fn st_ken_jump_recoil(
        &self,
        attacker: u16,
        defender: u16,
        guarded: bool,
    ) -> Option<StRyuPushback> {
        if !self.st_new_ryu(self.el(defender, 0x38)) || self.era_airborne(defender) {
            return None;
        }
        let attack = self.st_ken_jump_attack(attacker)?;
        let direction = if self.fighter_axis_x(defender) < self.fighter_axis_x(attacker) {
            -1
        } else {
            1
        };
        let kind = if guarded || self.eb(defender, 9) & 7 == 4 {
            match attack.button % 3 {
                0 => StRyuRecoil::Light,
                1 => StRyuRecoil::Medium,
                _ => StRyuRecoil::Heavy,
            }
        } else {
            StRyuRecoil::Light
        };
        Some(StRyuPushback {
            elapsed: 0,
            direction,
            kind,
        })
    }
}
