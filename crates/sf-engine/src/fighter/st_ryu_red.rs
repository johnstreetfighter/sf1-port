//! Red Hadoken flight and hitboxes measured in World Super Turbo.

use super::Bus;
use crate::combo::{StRyuPushback, StRyuRecoil};
use crate::era::{Loadout, Revision};
use crate::era_fight::StRyuRedHadoken;
use crate::roster::TournamentFighter;

const FLIGHT_HEIGHT: [u16; 38] = [
    20, 25, 29, 33, 36, 40, 43, 45, 48, 50, 52, 54, 55, 56, 57, 57, 57, 57, 57, 56, 56, 54, 53, 51,
    49, 47, 44, 41, 38, 35, 31, 27, 23, 18, 13, 8, 3, 0,
];
const BOUNCE_HEIGHT: [u16; 18] = [2, 4, 6, 7, 8, 9, 10, 10, 10, 10, 9, 8, 7, 6, 4, 2, 0, 0];
const KEN_RED_WAKEUP: u8 = 127;
const KEN_SHINKU_WAKEUP: u8 = 120;
const SHINKU_SLIDE: [i16; 7] = [2, 2, 1, 2, 1, 1, 1];

impl StRyuRedHadoken {
    pub(super) fn speed(self) -> u16 {
        u16::from(self.strength) + 2
    }

    pub(super) fn launches(self) -> bool {
        self.age <= 8
    }

    pub(super) fn damage(self) -> u16 {
        // Source damage at the first grounded distance on a 144 HP bar.
        let hp = [16, 17, 19][usize::from(self.strength - 1)];
        (hp * crate::era_fight::FULL_SUB_HP + 72) / 144
    }

    pub(super) fn recoil(direction: i8) -> StRyuPushback {
        StRyuPushback {
            elapsed: 0,
            direction,
            kind: StRyuRecoil::Heavy,
        }
    }

    fn attack_box(self) -> (u16, u16, u16) {
        // Offset behind the projectile origin, horizontal and vertical radii.
        match (self.age, self.strength) {
            (0..=2, _) => (16, 31, 13),
            (3..=8, _) => (0, 15, 14),
            (_, 1) => (0, 15, 12),
            (_, 2) => (1, 17, 12),
            (_, _) => (3, 21, 12),
        }
    }
}

impl Bus<'_> {
    pub(super) fn st_new_ken(&self, defender: u16) -> bool {
        let ps = self.el(defender, 0x38);
        self.sf2_state(ps).is_some_and(|era| {
            era.fighter == Some(TournamentFighter::Ken)
                && era.style.revision == Revision::St
                && era.style.loadout == Loadout::Standard
        })
    }

    pub(super) fn release_st_red_launch(&mut self, fighter: u16, slot: usize) {
        if self.toggle_state.combo.st_red_launch[slot]
            .is_some_and(|launch| {
                let wakeup = if launch.shinku { KEN_SHINKU_WAKEUP } else { KEN_RED_WAKEUP };
                launch.tick == wakeup - 1
            })
        {
            self.toggle_state.combo.st_red_launch[slot] = None;
            let player = self.el(fighter, 0x38);
            self.start_move(fighter, player, 1);
        }
    }

    pub(super) fn run_st_red_launch(&mut self, fighter: u16) -> bool {
        let slot = self.combo_slot(fighter);
        let Some(mut launch) = self.toggle_state.combo.st_red_launch[slot] else {
            return false;
        };
        // Ordinary hitstop still owns the initial contact pause. The
        // launch pose itself becomes visible on the first frozen tick.
        if self.contact_frozen(fighter) && launch.tick != 0 {
            launch.tick += 1;
            self.toggle_state.combo.st_red_launch[slot] = Some(launch);
            return true;
        }
        launch.tick = launch.tick.saturating_add(1);
        self.toggle_state.combo.st_red_launch[slot] = Some(launch);
        let flight_start = if launch.shinku { 15 } else { 16 };
        let floor = flight_start + 37;
        let bounce = floor + 5;
        let down = bounce + 17;
        let (dx, height) = match launch.tick {
            tick if tick < flight_start => (0, 16),
            tick if (flight_start..=floor).contains(&tick) => {
                let dx = if launch.shinku { 4 + i16::from(tick % 8 == 6) } else { 2 };
                (dx, FLIGHT_HEIGHT[usize::from(tick - flight_start)])
            }
            tick if (bounce..=down).contains(&tick) => {
                let dx = if launch.shinku { 3 + i16::from(tick % 2) } else { 2 + i16::from(tick % 2) };
                (dx, BOUNCE_HEIGHT[usize::from(tick - bounce)])
            }
            75..=81 if launch.shinku => (SHINKU_SLIDE[usize::from(launch.tick - 75)], 0),
            77 | 78 | 79 | 81 | 83 if !launch.shinku => (1, 0),
            _ => (0, 0),
        };
        let axis = self.fighter_axis_x(fighter);
        let pose = match launch.tick {
            1 => Some(0x3ed6),  // Airborne knockback.
            tick if tick == floor => Some(0x3ef6), // Floor contact and bounce.
            tick if tick == down => Some(0x3f1c), // Downed.
            98 => Some(0x3f3c), // Getting up.
            _ => None,
        };
        if let Some(frame) = pose {
            self.load_frame(fighter, frame);
            let correction = axis.wrapping_sub(self.fighter_axis_x(fighter));
            self.shift_grid_x(fighter, correction);
        }
        if launch.tick == down {
            self.set_eb(fighter, 9, 0x8a);
        }
        let dx = (dx * i16::from(launch.direction)) as u16;
        self.shift_grid_x(fighter, dx);
        self.set_st_ryu_knockdown_height(fighter, height);
        if launch.tick == floor {
            self.finish_knockdown_dust(fighter);
        }
        self.set_eb(fighter, 0x15, dx as u8);
        self.clamp_x(fighter);
        true
    }

    pub(super) fn set_st_ryu_knockdown_height(&mut self, fighter: u16, height: u16) {
        let tile_height = if self.is_big(fighter) { 32 } else { 16 };
        let grid_bottom = self.ew(fighter, 4)
            .wrapping_add((u16::from(self.eb(fighter, 0xc)) + 1) * tile_height);
        let bottom = crate::data_bank::GROUND as u16 - height;
        self.shift_grid_y(fighter, bottom.wrapping_sub(grid_bottom));
    }

    pub(super) fn st_ryu_red_move(&self, player: u32, move_id: u8) -> bool {
        move_id == crate::era_fight::SF2_EXTRA_SPECIAL
            && self
                .ryu_pacing_style(player)
                .is_some_and(|style| style.revision == Revision::St)
    }

    pub(super) fn place_st_ryu_hadoken(
        &mut self,
        effect: u16,
        ryu: u16,
        half: u16,
        speed: u16,
    ) {
        self.place_st_hadoken(effect, ryu, half, speed, (93, 54));
    }

    pub(super) fn place_st_hadoken(
        &mut self, effect: u16, owner: u16, half: u16, speed: u16, (forward, height): (u16, u16),
    ) {
        let dx = self.projectile_forward(effect, forward);
        let x = self.fighter_axis_x(owner).wrapping_add(dx);
        self.set_ew(effect, 6, x.wrapping_sub(half));
        self.set_ew(effect, 4, crate::data_bank::GROUND as u16 - height - half);
        self.set_eb(
            effect,
            0xb,
            self.projectile_forward(effect, speed) as u8,
        );
    }

    fn projectile_forward(&self, effect: u16, distance: u16) -> u16 {
        if self.ew(effect, 2) & 0x100 == 0 {
            distance
        } else {
            distance.wrapping_neg()
        }
    }

    pub(super) fn fighter_axis_x(&self, fighter: u16) -> u16 {
        self.center_x(fighter)
            .wrapping_sub(super::sext8(self.eb(fighter, 0x26)))
    }

    pub(super) fn effect_horizontal_step(&self, effect: u16, red: Option<StRyuRedHadoken>) -> u16 {
        let projectile = self.era_projectile(effect);
        if let Some(shot) = projectile.st_tiger_shot {
            return self.projectile_forward(effect, shot.speed());
        }
        if projectile.st_ryu_shinku_age.is_some() {
            return self.projectile_forward(effect, 5);
        }
        if let Some(shot) = projectile.st_standard_hadoken {
            return self.projectile_forward(effect, shot.speed());
        }
        red.map_or_else(
            || super::sext8(self.eb(effect, 0xb)),
            |red| self.projectile_forward(effect, red.speed()),
        )
    }

    pub(super) fn projectile_attack_box(&self, effect: u16) -> (u16, u16, u16, u16) {
        let (x, y) = self.effect_centre(effect);
        let projectile = self.era_projectile(effect);
        let bounds = projectile.st_ryu_red.map(StRyuRedHadoken::attack_box)
            .or_else(|| projectile.st_tiger_shot.map(|shot| {
                if shot.age < 2 { (25, 34, 5) } else { (0, 9, 5) }
            }))
            .or_else(|| projectile.st_ryu_shinku_age.map(|age| match age {
                0..=2 => (16, 31, 13),
                3..=8 => (0, 15, 14),
                _ => (3, 21, 12),
            }))
            .or_else(|| projectile.st_standard_hadoken.map(|shot| {
                if shot.fighter == TournamentFighter::Ken {
                    if shot.age < 2 { (16, 31, 12) } else { (0, 15, 12) }
                } else if shot.age <= 2 {
                    (16, 31, 13)
                } else {
                    (0, 15, 14)
                }
            }));
        let Some((back, width, height)) = bounds else {
            return (
                x,
                y,
                self.eb(effect, 0xd).into(),
                self.eb(effect, 0xc).into(),
            );
        };
        (
            x.wrapping_sub(self.projectile_forward(effect, back)),
            y,
            width,
            height,
        )
    }

    pub(super) fn st_ryu_red_overlaps_fighter(&self, effect: u16, defender: u16) -> bool {
        let (cx, cy, w, h) = self.projectile_attack_box(effect);
        let st_ken = self.st_new_ken(defender);
        let posture = self.eb(defender, 0xa);
        let (tx, ty, tw, th) = if st_ken && matches!(posture, 1..=8) && !self.era_airborne(defender)
        {
            // Standing and crouching torso used in the arcade Ken fixtures.
            let (mut offset, height, width, radius) = if matches!(posture, 5..=8) {
                (-6i16, 33, 22, 12)
            } else {
                (6i16, 52, 20, 21)
            };
            if self.eb(defender, 0x37) & 4 == 0 {
                offset = -offset;
            }
            (
                self.fighter_axis_x(defender).wrapping_add(offset as u16),
                crate::data_bank::GROUND as u16 - height,
                width,
                radius,
            )
        } else {
            (
                self.center_x(defender)
                    .wrapping_add(super::sext8(self.eb(defender, 0x20))),
                self.center_y(defender)
                    .wrapping_sub(super::sext8(self.eb(defender, 0x1e))),
                self.eb(defender, 0x21).into(),
                self.eb(defender, 0x1f).into(),
            )
        };
        tw + w >= Self::borrow_abs(tx, cx) && th + h >= Self::borrow_abs(ty, cy)
    }
}
