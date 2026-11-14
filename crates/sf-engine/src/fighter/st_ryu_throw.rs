//! Measured World Super Turbo throw positions in native pixels.

use super::{Bus, CAMERA, GAME_MODE, P1_STRUCT, P2_STRUCT};
use crate::combo::throw::{StRyuThrow, StRyuThrowFamily, GET_UP_MOVE, THROWN_MOVE, THROW_MOVE};
use crate::era::{Loadout, Revision};
use crate::fight_reader::player_struct;
use crate::roster::{PlayMode, TournamentFighter};
use crate::session_io::ControlScheme;
use sf_video::callouts::Side;

#[path = "st_ken_throw.rs"]
mod ken;
#[path = "st_ken_air_throw.rs"]
mod st_ken_air_throw;

const RELEASE_X: i16 = 127;
const GRAVITY_Q8: i32 = 72;
const THROW_RANGE: u16 = 77;
const SOFTEN_WINDOW: u8 = 13;
const THROW_PUNCHES: u8 = 0b000110;
const THROW_KICKS: u8 = 0b110000;
// First order selection in the audited World 940223 versus match.
const ST_ORDER_START_SEED: u16 = 0x7420;
// 16 of 144 arcade HP rounded to the nearest point on the 960 Sub-HP
// scale, doubled for a full throw.
const SOFT_DAMAGE: u16 = 107;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThrowPosition {
    pub x_offset: i16,
    pub height: u8,
}

impl ThrowPosition {
    const fn new(x_offset: i16, height: u8) -> Self {
        Self { x_offset, height }
    }
}

// Captured poses attach to authored points on Ryu, then keep the last
// attachment point until release. X faces the direction of the final throw.
const SHOULDER_CAPTURE: &[(u8, ThrowPosition)] = &[
    (0, ThrowPosition::new(-25, 0)),
    (15, ThrowPosition::new(-45, 29)),
    (25, ThrowPosition::new(-44, 71)),
    (31, ThrowPosition::new(21, 51)),
    (37, ThrowPosition::new(56, 33)),
    (43, ThrowPosition::new(RELEASE_X, 44)),
];
const FLIP_CAPTURE: &[(u8, ThrowPosition)] = &[
    (0, ThrowPosition::new(-25, 0)),
    (15, ThrowPosition::new(-15, 13)),
    (25, ThrowPosition::new(36, 46)),
    (31, ThrowPosition::new(76, 25)),
    (35, ThrowPosition::new(114, 28)),
    (37, ThrowPosition::new(RELEASE_X, 32)),
];
const SAGAT_CAPTURE: &[(u8, ThrowPosition)] = &[
    (0, ThrowPosition::new(-56, 2)),
    (6, ThrowPosition::new(-34, 55)),
    (26, ThrowPosition::new(12, 89)),
    (32, ThrowPosition::new(125, 107)),
    (38, ThrowPosition::new(RELEASE_X, 107)),
];

// Second bounce, beginning at landing +5. The flip's bounce uses one
// pixel less at sample 13 and ends one update before the shoulder's.
const BOUNCE_HEIGHT: [u8; 17] = [2, 4, 6, 7, 8, 9, 10, 10, 10, 10, 9, 8, 7, 6, 4, 2, 0];
const SAGAT_BOUNCE_HEIGHT: [u8; 17] = [2, 4, 6, 7, 8, 9, 10, 10, 10, 9, 9, 8, 7, 5, 4, 2, 0];

// After the second bounce the horizontal speed decays into the downed
// pose. These are integer screen-pixel offsets after the last 2.5 px step.
const SHOULDER_SLIDE_RIGHT: [i16; 8] = [0, 1, 2, 3, 3, 4, 4, 5];
const SHOULDER_SLIDE_LEFT: [i16; 8] = [1, 2, 3, 4, 4, 5, 5, 6];
const FLIP_SLIDE: [i16; 10] = [1, 2, 3, 3, 4, 4, 5, 5, 5, 6];

impl StRyuThrow {
    pub(crate) fn position(self) -> ThrowPosition {
        if self.family == StRyuThrowFamily::KenAir {
            let frame = ken_air_frame(self);
            return ThrowPosition::new(frame.victim_x, frame.victim_height.max(0) as u8);
        }
        if let Some(position) = ken::victim_position(self) {
            return position;
        }
        let direction = if self.leftward { -1 } else { 1 };
        let release = self.release_tick();
        let capture = match self.family {
            StRyuThrowFamily::Shoulder | StRyuThrowFamily::KenShoulder => SHOULDER_CAPTURE,
            StRyuThrowFamily::Flip => FLIP_CAPTURE,
            StRyuThrowFamily::Sagat => SAGAT_CAPTURE,
            _ => unreachable!("resolved by the Ken throw profile"),
        };
        if self.elapsed <= release {
            let mut pose = capture
                .iter()
                .rev()
                .find(|(tick, _)| u16::from(*tick) <= self.elapsed)
                .expect("capture begins at tick zero")
                .1;
            pose.x_offset *= direction;
            return pose;
        }

        let release_height = capture.last().expect("release pose").1.height;
        let flight = self.elapsed - release;
        let landing = self.landing_tick();
        if self.softened {
            let steps = (self.elapsed.min(landing) - release - 1) as i16;
            let x = direction * (RELEASE_X + 2 * steps);
            let height = ballistic_height(release_height + 16, steps, 1400);
            return ThrowPosition::new(x, height);
        }

        let last_flight_step = if self.family == StRyuThrowFamily::Sagat {
            self.damage_tick()
        } else {
            landing
        };
        if self.elapsed <= last_flight_step {
            let steps = (flight - 1) as i16;
            let x = signed_q8_step(RELEASE_X, steps, 896, direction);
            let height = ballistic_height(release_height, steps, 824);
            return ThrowPosition::new(x, height);
        }

        if self.family == StRyuThrowFamily::Sagat {
            self.sagat_bounce_position(direction)
        } else {
            self.full_landing_position(direction)
        }
    }

    fn sagat_bounce_position(self, direction: i16) -> ThrowPosition {
        // Ken advances one last flight step on the damage tick. The bounce
        // resumes after a four-tick pause, retaining the half-pixel fraction.
        let bounce = self.elapsed.saturating_sub(self.damage_tick() + 4);
        let x = 277 * 256 + 128 + i32::from(bounce.min(8)) * 640 + 256 * i32::from(bounce >= 9);
        let slide_at = if self.leftward { 105 } else { 106 };
        let x = x + 256 * i32::from(self.elapsed >= slide_at);
        let height = bounce
            .checked_sub(1)
            .and_then(|index| SAGAT_BOUNCE_HEIGHT.get(usize::from(index)))
            .copied()
            .unwrap_or(0);
        ThrowPosition::new(((x * i32::from(direction)) >> 8) as i16, height)
    }

    fn full_landing_position(self, direction: i16) -> ThrowPosition {
        let landing = self.landing_tick();
        let after_landing = self.elapsed - landing;
        let flight_steps = (landing - self.release_tick() - 1) as i16;
        let base_x = RELEASE_X + flight_steps * 7 / 2;
        let (bounce_steps, slide) = match (self.family, self.leftward) {
            (StRyuThrowFamily::Shoulder | StRyuThrowFamily::KenShoulder, false) => {
                (18, SHOULDER_SLIDE_RIGHT.as_slice())
            }
            (StRyuThrowFamily::Shoulder | StRyuThrowFamily::KenShoulder, true) => {
                (18, SHOULDER_SLIDE_LEFT.as_slice())
            }
            (StRyuThrowFamily::Flip, _) => (17, FLIP_SLIDE.as_slice()),
            _ => unreachable!("throw has a separate bounce"),
        };
        let after_pause = usize::from(after_landing.saturating_sub(4));
        let bounce_x = signed_q8_step(base_x, after_pause.min(bounce_steps) as i16, 640, direction);
        let slide_step = after_pause.saturating_sub(bounce_steps);
        let slide_x = if slide_step == 0 {
            0
        } else {
            slide[(slide_step - 1).min(slide.len() - 1)] * direction
        };
        let height = if after_pause == 0 {
            0
        } else if self.family == StRyuThrowFamily::Flip && after_pause == 14 {
            5
        } else {
            BOUNCE_HEIGHT.get(after_pause - 1).copied().unwrap_or(0)
        };
        ThrowPosition::new(bounce_x + slide_x, height)
    }
}

fn ken_air_frame(motion: StRyuThrow) -> st_ken_air_throw::Frame {
    debug_assert_eq!(motion.family.release_tick(), st_ken_air_throw::RELEASE);
    debug_assert_eq!(
        motion.family.attacker_recovery_tick(),
        st_ken_air_throw::KEN_READY
    );
    debug_assert_eq!(
        motion.landing_tick(),
        if motion.softened {
            u16::from(st_ken_air_throw::SOFT_LAND)
        } else {
            u16::from(st_ken_air_throw::FULL_LAND)
        }
    );
    debug_assert_eq!(
        motion.defender_recovery_tick(),
        if motion.softened {
            u16::from(st_ken_air_throw::SOFT_RYU_READY)
        } else {
            u16::from(st_ken_air_throw::FULL_RYU_READY)
        }
    );
    st_ken_air_throw::frame(
        motion.elapsed.min(u16::from(u8::MAX)) as u8,
        i16::from(motion.start_height),
        motion.softened,
        motion.leftward,
    )
}

// Both throws use the same fixed-point gravity during their first flight.
// The full throw has one held tick after release. The softened throw instead
// rises 16 pixels on that tick, then begins its longer ballistic arc.
fn ballistic_height(start: u8, steps: i16, velocity_q8: i32) -> u8 {
    let n = i32::from(steps);
    let height = (i32::from(start) << 8) + n * velocity_q8 - n * (n - 1) / 2 * GRAVITY_Q8;
    (height >> 8).max(0) as u8
}

// Integer samples round toward the floor on either side of the origin.
fn signed_q8_step(base: i16, steps: i16, velocity_q8: i32, direction: i16) -> i16 {
    let x = i32::from(direction) * ((i32::from(base) << 8) + i32::from(steps) * velocity_q8);
    (x >> 8) as i16
}

fn step_st_order_seed(seed: u16) -> u16 {
    (seed >> 1) | ((((seed >> 1) ^ (seed >> 9)) & 1) << 15)
}

impl Bus<'_> {
    /// World Super Turbo throw rules apply to the audited fighter pairs.
    pub(super) fn try_st_ryu_throw(
        &mut self,
        player: u32,
        attacker: u16,
        defender: u16,
        direction: u8,
    ) -> Option<bool> {
        let defender_player = self.el(defender, 0x38);
        let throw_macro = self
            .player_slot_index(player)
            .is_some_and(|slot| self.toggle_state.schemes[slot].throw_macro);
        let family = if self.st_throw_sagat_pair(player, defender_player) {
            if self.era_buttons(player).1 & THROW_PUNCHES == 0 && !throw_macro {
                return Some(false);
            }
            StRyuThrowFamily::Sagat
        } else {
            let (attacker_ryu, _) = self.st_throw_shotos(player, defender_player)?;
            let Some(family) = self.st_throw_input_family(player, attacker_ryu, throw_macro) else {
                return Some(false);
            };
            family
        };
        if !matches!(direction, 1 | 2) && !(throw_macro && direction == 0)
            || !self.st_throw_available(player, attacker, defender, direction, family)
        {
            return Some(false);
        }
        let Some((attacker_entry, defender_entry)) = self.throw_entries(attacker, defender) else {
            return Some(false);
        };
        let attacker_header = match family {
            StRyuThrowFamily::Shoulder
            | StRyuThrowFamily::Sagat
            | StRyuThrowFamily::KenShoulder => attacker_entry.throw_header,
            StRyuThrowFamily::Flip => {
                let Some((header, _)) = self.data_bank.and_then(|bank| bank.evil_ryu_kick_throw())
                else {
                    return Some(false);
                };
                header
            }
            StRyuThrowFamily::HellWheel | StRyuThrowFamily::KneeBash | StRyuThrowFamily::KenAir => {
                self.data_bank?.st_ken_throw(family)?
            }
        };
        let leftward = direction == 2
            || (direction == 0 && self.fighter_axis_x(defender) < self.fighter_axis_x(attacker));
        let origin_x = self.fighter_axis_x(attacker).wrapping_add(self.rw(CAMERA));
        let back = leftward != (self.fighter_axis_x(defender) < self.fighter_axis_x(attacker));
        self.start_throw(
            player,
            attacker,
            defender,
            attacker_header,
            defender_entry.thrown_header,
            back,
        );
        if family == StRyuThrowFamily::Sagat {
            // Both headers start unmirrored. The initial animation step
            // applies the chosen facing before setting the captured axes.
            self.set_eb(attacker, 0x37, if leftward { 4 } else { 2 });
            self.set_eb(defender, 0x37, if leftward { 2 } else { 4 });
        }
        let slot = self.combo_slot(defender);
        self.toggle_state.throw.tech_window[slot] = 0;
        self.toggle_state.throw.crosses[slot] = false;
        let mut motion = StRyuThrow {
            back,
            ..StRyuThrow::new(family, origin_x, leftward)
        };
        if family == StRyuThrowFamily::KneeBash {
            motion.knee.directions = [self.pad_directions(defender_player), direction];
        }
        self.toggle_state.throw.st_ryu[slot] = Some(motion);
        Some(true)
    }

    pub(super) fn try_st_ken_air_throw(
        &mut self,
        player: u32,
        attacker: u16,
        defender: u16,
        direction: u8,
    ) -> bool {
        let Some(state) = self.sf2_state(player) else {
            return false;
        };
        let defender_airborne = self.is_jump_airborne(defender)
            || self.frames_until_landing(defender) > 0;
        if !state.st_new_ken_kicks()
            || self.st_throw_shotos(player, self.el(defender, 0x38)) != Some((false, true))
            || !matches!(direction & 3, 1 | 2)
            || self.era_buttons(player).1 & THROW_KICKS == 0
            || !self.throw_target_is_vulnerable(defender)
            || !defender_airborne
        {
            return false;
        }
        let leftward = self.eb(attacker, 0x37) & 4 != 0;
        let attacker_x = self.fighter_axis_x(attacker);
        let gap = self.fighter_axis_x(defender).wrapping_sub(attacker_x) as i16;
        let gap = if leftward { -gap } else { gap };
        let ken_height = self.height_above_mat(attacker) as i16;
        let victim_height = self.height_above_mat(defender) as i16;
        let falling = (self.eb(attacker, 0x14) as i8) > 0;
        if !st_ken_air_throw::eligible(falling, ken_height, gap, victim_height, 29, 53) {
            return false;
        }
        let family = StRyuThrowFamily::KenAir;
        let Some(attacker_header) = self.data_bank.and_then(|bank| bank.st_ken_throw(family))
        else {
            return false;
        };
        let Some((_, defender_entry)) = self.throw_entries(attacker, defender) else {
            return false;
        };
        self.start_throw(
            player,
            attacker,
            defender,
            attacker_header,
            defender_entry.thrown_header,
            false,
        );
        let slot = self.combo_slot(defender);
        self.toggle_state.throw.tech_window[slot] = 0;
        self.toggle_state.throw.crosses[slot] = false;
        self.toggle_state.throw.st_ryu[slot] = Some(StRyuThrow {
            start_height: ken_height as u8,
            ..StRyuThrow::new(family, attacker_x.wrapping_add(self.rw(CAMERA)), leftward)
        });
        true
    }

    fn st_throw_sagat_pair(&self, attacker: u32, defender: u32) -> bool {
        self.sf2_state(attacker)
            .is_some_and(|state| state.st_sagat())
            && self.sf2_state(defender).is_some_and(|state| {
                state.style.revision == Revision::St
                    && state.style.loadout == Loadout::Standard
                    && state.fighter == Some(TournamentFighter::Ken)
            })
            && self.dramatic_rules().is_none()
    }

    fn st_throw_shotos(&self, p1: u32, p2: u32) -> Option<(bool, bool)> {
        use TournamentFighter::{Ken, Ryu};
        let first = self.sf2_state(p1)?;
        let second = self.sf2_state(p2)?;
        if first.style.revision != Revision::St
            || second.style.revision != Revision::St
            || self.evil_ryu_profile(p1).is_some()
            || self.evil_ryu_profile(p2).is_some()
            || self.dramatic_rules().is_some()
        {
            return None;
        }
        match (first.fighter?, second.fighter?) {
            (Ryu, Ken) => Some((true, false)),
            (Ken, Ryu) => Some((false, true)),
            (Ryu, Ryu) => Some((true, true)),
            _ => None,
        }
    }

    fn st_throw_input_family(
        &self,
        player: u32,
        ryu: bool,
        throw_macro: bool,
    ) -> Option<StRyuThrowFamily> {
        let (_, pressed) = self.era_buttons(player);
        if pressed & THROW_PUNCHES != 0 || throw_macro {
            return Some(if ryu {
                StRyuThrowFamily::Shoulder
            } else {
                StRyuThrowFamily::KenShoulder
            });
        }
        if pressed & THROW_KICKS == 0 {
            return None;
        }
        if ryu {
            return Some(StRyuThrowFamily::Flip);
        }
        let new_ken = self.sf2_state(player)?.style.loadout == Loadout::Standard;
        Some(if new_ken && pressed & 0x10 != 0 {
            StRyuThrowFamily::KneeBash
        } else {
            StRyuThrowFamily::HellWheel
        })
    }

    fn st_throw_available(
        &self,
        player: u32,
        attacker: u16,
        defender: u16,
        direction: u8,
        family: StRyuThrowFamily,
    ) -> bool {
        let special_ready = match family {
            StRyuThrowFamily::Shoulder
            | StRyuThrowFamily::Sagat
            | StRyuThrowFamily::KenShoulder => {
                self.punch_special_ready_for_dir(player, attacker, direction)
            }
            StRyuThrowFamily::Flip => self.kick_special_ready_for_dir(player, attacker, direction),
            StRyuThrowFamily::HellWheel | StRyuThrowFamily::KneeBash => {
                self.kick_special_ready_for_dir(player, attacker, direction)
                    || self
                        .sf2_state(player)
                        .and_then(|state| state.st_ken_kick_command())
                        .is_some()
            }
            StRyuThrowFamily::KenAir => return false,
        };
        !special_ready
            && self.throw_target_is_vulnerable(defender)
            && self.eb(defender, 9) & 3 != 1
            && (self.eb(attacker, 9) & 3 == 2 || matches!(self.eb(attacker, 0xa), 2 | 3))
            && self
                .fighter_axis_x(attacker)
                .abs_diff(self.fighter_axis_x(defender))
                <= THROW_RANGE
    }

    pub(super) fn st_throw_p2_input_first(&mut self) -> bool {
        let Some(p1_ryu) = self.st_throw_shotos(P1_STRUCT, P2_STRUCT).map(|(ryu, _)| ryu) else {
            return false;
        };
        let Some(p2_ryu) = self.st_throw_shotos(P2_STRUCT, P1_STRUCT).map(|(ryu, _)| ryu) else {
            return false;
        };
        let throws = &mut self.toggle_state.throw;
        let seed = throws.st_order_seed.unwrap_or(ST_ORDER_START_SEED);
        // Arcade skips the selector during a capture, but its ordinary
        // trailing RNG call still runs. This clock is independent of SF1's.
        let captured = [0, 1]
            .into_iter()
            .any(|slot| throws.st_ryu_capture_active(slot));
        let selector = if captured {
            seed
        } else {
            step_st_order_seed(seed)
        };
        throws.st_order_seed = Some(step_st_order_seed(selector));
        !captured
            && selector & 1 == 0
            && self.st_throw_order_candidate(P1_STRUCT, p1_ryu)
            && self.st_throw_order_candidate(P2_STRUCT, p2_ryu)
    }

    fn st_throw_order_candidate(&self, player: u32, ryu: bool) -> bool {
        let (fighter, other) = self.player_entities(player);
        let direction = self.pad_directions(player);
        let Some(family) = self.st_throw_input_family(player, ryu, false) else {
            return false;
        };
        // Reorder direct Classic throws only. Leave motion, shortcuts and
        // ordinary dispatch order with their existing input handlers.
        self.player_slot_index(player)
            .is_some_and(|slot| self.toggle_state.schemes[slot].mode == ControlScheme::Classic)
            && matches!(direction, 1 | 2)
            && !self.contact_frozen(fighter)
            && self.eb(fighter, 8) == 2
            && (self.eb(fighter, 9) & 0xc9 == 0 || matches!(self.eb(fighter, 0xa), 2 | 3))
            && self.era_special_command(player, false).is_none()
            && self.era_special_command(player, true).is_none()
            && self.st_throw_available(player, fighter, other, direction, family)
    }

    pub(super) fn advance_st_ryu_throws(&mut self) {
        let mode = self.rb(GAME_MODE);
        for side in [Side::P1, Side::P2] {
            let slot = side.index();
            let Some(mut motion) = self.toggle_state.throw.st_ryu[slot] else {
                continue;
            };
            let player = player_struct(side, mode);
            let (fighter, _) = self.player_entities(player);
            self.toggle_state.throw.st_ryu[slot] =
                if matches!(self.eb(fighter, 0xa), THROWN_MOVE | GET_UP_MOVE) {
                    motion.elapsed = motion.elapsed.saturating_add(1);
                    if motion.family == StRyuThrowFamily::KneeBash {
                        let (_, attacker) = self.player_entities(player);
                        let thrower = self.el(attacker, 0x38);
                        let input = |ps| (self.pad_directions(ps), self.era_buttons(ps).1);
                        ken::advance_knee_bash(&mut motion, [input(player), input(thrower)]);
                    }
                    Some(motion)
                } else {
                    None
                };
        }
    }

    pub(super) fn st_ryu_throw_input(&mut self, player: u32, fighter: u16, direction: u8) -> bool {
        let slot = self.combo_slot(fighter);
        if let Some(motion) = self.toggle_state.throw.st_ryu[slot] {
            let new = self
                .sf2_state(player)
                .is_some_and(|state| state.style.loadout == Loadout::Standard);
            let response = matches!(direction, 1 | 2)
                && self.era_buttons(player).1 & (THROW_PUNCHES | THROW_KICKS) != 0;
            let window = if motion.family == StRyuThrowFamily::KenAir {
                st_ken_air_throw::RESPONSE_LAST
            } else {
                SOFTEN_WINDOW
            };
            if motion.family.can_soften() && motion.elapsed <= u16::from(window) && new && response
            {
                self.toggle_state.throw.st_ryu[slot] = Some(StRyuThrow {
                    softened: true,
                    ..motion
                });
            }
            self.ww(player, 0);
            self.clear_recovery_buffer(player);
            return true;
        }
        false
    }

    pub(super) fn run_st_ryu_throw(&mut self, fighter: u16, other: u16) -> bool {
        let slot = self.combo_slot(fighter);
        if let Some(motion) = self.toggle_state.throw.st_ryu[slot] {
            self.run_st_ryu_throw_victim(fighter, other, motion);
            return true;
        }
        if self.eb(fighter, 0xa) == THROW_MOVE {
            let other_slot = self.combo_slot(other);
            if let Some(motion) = self.toggle_state.throw.st_ryu[other_slot] {
                self.run_st_ryu_throw_attacker(fighter, other, motion);
                return true;
            }
        }
        false
    }

    fn run_st_ryu_throw_attacker(&mut self, fighter: u16, defender: u16, motion: StRyuThrow) {
        let ps = self.el(fighter, 0x38);
        if motion.elapsed >= motion.attacker_recovery_tick() && !motion.capture_active() {
            self.finish_st_ryu_throw_pose(fighter, defender, ps);
            return;
        }
        if motion.family == StRyuThrowFamily::KneeBash {
            let phase = motion.knee.phase;
            if motion.elapsed == 0 {
                self.step_st_ryu_throw_animation(fighter);
                self.set_eb(fighter, 0x31, phase);
            } else if phase != self.eb(fighter, 0x31) {
                let header = self
                    .data_bank
                    .and_then(|bank| bank.st_ken_throw(motion.family))
                    .expect("active Ken throw has a header");
                let stream = self.rl(header + 0x2c);
                let pose = match phase {
                    1 => stream,
                    2 => stream + 0x20,
                    _ => stream + 0x40,
                };
                self.load_frame(fighter, pose);
                self.set_eb(fighter, 0x31, phase);
            }
        } else if motion.family == StRyuThrowFamily::Sagat {
            match motion.elapsed {
                0 => self.step_st_ryu_throw_animation(fighter),
                6 | 26 => self.load_st_ryu_thrown_pose(fighter),
                _ => {}
            }
        } else {
            self.step_st_ryu_throw_animation(fighter);
        }
        let position = if motion.family == StRyuThrowFamily::KenAir {
            let frame = ken_air_frame(motion);
            ThrowPosition::new(frame.ken_x, frame.ken_height.max(0) as u8)
        } else {
            ken::attacker_position(motion)
        };
        self.set_st_ryu_throw_position(fighter, motion.origin_x, position);
    }

    fn run_st_ryu_throw_victim(&mut self, fighter: u16, attacker: u16, motion: StRyuThrow) {
        let slot = self.combo_slot(fighter);
        let player = self.el(fighter, 0x38);
        if motion.elapsed >= motion.defender_recovery_tick() && !motion.capture_active() {
            self.toggle_state.throw.st_ryu[slot] = None;
            self.finish_st_ryu_throw_pose(fighter, attacker, player);
            return;
        }
        let release = motion.release_tick();
        let elapsed = motion.elapsed;
        let landing = motion.landing_tick();
        let position = motion.position();
        let (lift, turn) = match motion.family {
            StRyuThrowFamily::Sagat => (6, 26),
            StRyuThrowFamily::KneeBash => (13, release),
            StRyuThrowFamily::KenAir => (14, 31),
            _ => (15, 31),
        };
        if elapsed == 0 {
            self.step_st_ryu_throw_animation(fighter);
        } else if elapsed == lift || elapsed == turn || elapsed == landing && !motion.softened {
            self.load_st_ryu_thrown_pose(fighter);
        } else if motion.softened && elapsed == landing {
            self.start_move(fighter, player, 1);
            self.step_st_ryu_throw_animation(fighter);
        }
        if motion.family == StRyuThrowFamily::Sagat && elapsed == turn {
            self.set_st_ryu_throw_facing(fighter, motion.leftward);
        }
        if elapsed == release || motion.family == StRyuThrowFamily::Sagat && elapsed == 32 {
            self.set_st_ryu_throw_facing(fighter, !ken::victim_travel_leftward(motion));
        }
        let is_air = motion.family == StRyuThrowFamily::KenAir;
        if is_air {
            let frame = ken_air_frame(motion);
            self.set_st_ryu_throw_facing(fighter, motion.leftward == frame.victim_turn);
        }
        if elapsed >= landing && !motion.capture_active() {
            self.set_eb(fighter, 9, 0x82);
            if motion.softened || elapsed > motion.damage_tick() {
                self.set_eb(fighter, 0xa, GET_UP_MOVE);
            }
        } else {
            let knee_on_floor = motion.family == StRyuThrowFamily::KneeBash && position.height == 0;
            self.set_eb(
                fighter,
                9,
                if (!is_air && elapsed < lift) || knee_on_floor {
                    0x82
                } else {
                    0x81
                },
            );
            self.set_eb(fighter, 0xa, THROWN_MOVE);
        }
        self.set_eb(fighter, 0xf, 0x08);
        self.set_st_ryu_throw_position(fighter, motion.origin_x, position);
        if let Some(damage_hp) = motion.damage_hp() {
            self.apply_st_ryu_throw_damage(fighter, attacker, motion, damage_hp);
        }
    }

    fn step_st_ryu_throw_animation(&mut self, fighter: u16) {
        if self.eb(fighter, 8) == 1 {
            self.set_eb(fighter, 8, 2);
            self.set_eb(fighter, 0x33, self.eb(fighter, 0xe));
            if self.eb(fighter, 0x37) & 4 != 0 {
                self.face_left(fighter);
            }
        } else if self.eb(fighter, 0x33) != 0 {
            self.set_eb(fighter, 0x33, self.eb(fighter, 0x33) - 1);
        } else {
            let pose = self.el(fighter, 0x2c);
            if self.rb(pose) < 0xfd {
                self.load_frame(fighter, pose);
            }
        }
    }

    fn load_st_ryu_thrown_pose(&mut self, fighter: u16) {
        let mut pose = self.el(fighter, 0x2c);
        if self.rb(pose) == 0xff {
            pose += 6;
        }
        if self.rb(pose) < 0xfd {
            self.load_frame(fighter, pose);
        }
    }

    fn set_st_ryu_throw_facing(&mut self, fighter: u16, leftward: bool) {
        if (self.eb(fighter, 0x37) & 4 != 0) != leftward {
            if leftward {
                self.face_left(fighter);
            } else {
                self.face_right(fighter);
            }
        }
        self.set_eb(fighter, 0x37, if leftward { 4 } else { 2 });
    }

    fn finish_st_ryu_throw_pose(&mut self, fighter: u16, other: u16, player: u32) {
        let axis = self.fighter_axis_x(fighter);
        let leftward = self.fighter_axis_x(other) < axis;
        self.set_eb(fighter, 0x37, if leftward { 4 } else { 2 });
        self.start_move(fighter, player, 1);
        self.step_st_ryu_throw_animation(fighter);
        self.shift_grid_x(fighter, axis.wrapping_sub(self.fighter_axis_x(fighter)));
    }

    fn set_st_ryu_throw_position(&mut self, fighter: u16, origin_x: u16, pos: ThrowPosition) {
        let camera = self.rw(CAMERA);
        let target_x = i32::from(origin_x) + i32::from(pos.x_offset) - i32::from(camera);
        let current_x = i32::from(self.fighter_axis_x(fighter));
        // Preserve the signed travel request for the native camera. Keep
        // the actual grid addressable until its ordinary screen clamp runs.
        let left = sf_video::render::VISAREA_X as i32;
        let x = target_x.max(left);
        self.shift_grid_x(fighter, (x - current_x) as u16);
        let tile_height = if self.is_big(fighter) { 32 } else { 16 };
        let grid_bottom = self
            .ew(fighter, 4)
            .wrapping_add((u16::from(self.eb(fighter, 0xc)) + 1) * tile_height);
        let bottom = crate::data_bank::GROUND as u16 - u16::from(pos.height);
        self.shift_grid_y(fighter, bottom.wrapping_sub(grid_bottom));
        self.set_eb(fighter, 0x15, (target_x - current_x).clamp(-127, 127) as u8);
        self.clamp_x(fighter);
    }

    fn apply_st_ryu_throw_damage(
        &mut self,
        fighter: u16,
        attacker: u16,
        motion: StRyuThrow,
        damage_hp: u16,
    ) {
        let defender = self.el(fighter, 0x38);
        let thrower = self.el(attacker, 0x38);
        let damage = match (motion.family, motion.softened) {
            (_, true) => SOFT_DAMAGE,
            (StRyuThrowFamily::Shoulder | StRyuThrowFamily::Flip, false) => SOFT_DAMAGE * 2,
            _ => (damage_hp * crate::era_fight::FULL_SUB_HP + 72) / 144,
        };
        let record = damage.div_ceil(crate::era_fight::SUB_HP_PER_POINT) as u8;
        self.training_contact(fighter);
        self.training_hit(defender, record);
        self.era_damage(defender, damage, self.roster.mode == PlayMode::Training);
        if !motion.softened {
            self.record_throw_dust(thrower, fighter);
            self.queue_sound(9);
        }
        if matches!(thrower, P1_STRUCT | P2_STRUCT) {
            self.add_score(attacker, self.rb(0x4637e));
            self.show_score(thrower);
            self.check_hiscore(attacker);
        }
        if !self.defender_alive(defender) {
            self.toggle_state.throw.st_ryu[self.combo_slot(fighter)] = None;
            if motion.family == StRyuThrowFamily::KneeBash {
                self.finish_st_ryu_throw_pose(attacker, fighter, thrower);
            }
            self.set_eb(fighter, 8, 8);
            self.set_eb(fighter, 0xa, 0x40);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sagat_positions_match_the_unclamped_world_turbo_two_capture() {
        let evidence =
            include_str!("../../../../docs/research/sagat-super-turbo-throw-trajectory.tsv");
        for row in evidence.lines().skip(1) {
            let [direction, outcome, tick, sagat_x, sagat_y, ken_x, ken_y] =
                <[&str; 7]>::try_from(row.split('\t').collect::<Vec<_>>()).unwrap();
            assert_eq!((sagat_x, sagat_y), ("0", "0"));
            let mut motion = StRyuThrow::new(StRyuThrowFamily::Sagat, 0, direction == "back");
            motion.elapsed = tick.parse().unwrap();
            motion.softened = outcome == "soft";
            assert_eq!(
                motion.position(),
                ThrowPosition::new(ken_x.parse().unwrap(), ken_y.parse().unwrap()),
                "{row}"
            );
        }
    }

    #[test]
    fn positions_match_the_unclamped_world_turbo_two_capture() {
        let evidence = include_str!("../../../../docs/research/ryu-st-turbo2-throw-trajectory.tsv");
        for row in evidence.lines().skip(1) {
            let mut fields = row.split('\t');
            let family = match fields.next().unwrap() {
                "shoulder" => StRyuThrowFamily::Shoulder,
                "flip" => StRyuThrowFamily::Flip,
                field => panic!("unknown family {field}"),
            };
            let leftward = fields.next().unwrap() == "back";
            let softened = fields.next().unwrap() == "softened";
            let elapsed = fields.next().unwrap().parse().unwrap();
            assert_eq!(fields.next(), Some("0"), "Ryu's axis holds");
            let x = fields.next().unwrap().parse().unwrap();
            let height = fields.next().unwrap().parse().unwrap();
            let events = fields.next().unwrap_or_default();
            let mut motion = StRyuThrow::new(family, 0, leftward);
            motion.elapsed = elapsed;
            motion.softened = softened;
            assert_eq!(motion.position(), ThrowPosition::new(x, height), "{row}");
            assert_eq!(
                motion.capture_active(),
                elapsed < u16::from(family.release_tick())
            );
            for (event, tick) in [
                ("release", motion.release_tick()),
                ("ryu_neutral", motion.attacker_recovery_tick()),
                ("damage", motion.damage_tick()),
                ("land", motion.landing_tick()),
                ("ken_recovery", motion.defender_recovery_tick()),
            ] {
                assert_eq!(
                    events.split(',').any(|value| value == event),
                    elapsed == tick,
                    "{row}"
                );
            }
        }
    }
}
