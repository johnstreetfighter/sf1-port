//! Grounded contact behavior measured for New and Old Ryu in World Super Turbo.

use super::Bus;
use crate::combo::{FrozenCancel, StRyuPushback, StRyuRecoil, StRyuSweep};
use crate::data_bank::st_ryu_normals::{CLOSE_HK, CLOSE_LK, CLOSE_MK, FAR_HK, FAR_HP, FAR_LK, FAR_MK, FAR_MP};
use crate::era::{Loadout, Revision};

const HITSTOP: u8 = 14;
const CROUCHING_KICK: u8 = 0x17;
// Per-tick world X deltas from the four-phase World Turbo 2 traces.
const LIGHT_PUSH: &[u8] = &[4, 4, 4, 4, 3, 3, 3, 2, 1, 1, 1];
const MEDIUM_PUSH: &[u8] = &[6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 2, 1, 1, 1, 1, 1];
const HEAVY_PUSH: &[u8] = &[8, 8, 8, 7, 7, 7, 6, 5, 5, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1];
const SWEEP_HEIGHT: [u16; 40] = [
    5, 10, 15, 20, 24, 28, 32, 35, 39, 42, 44, 47, 49, 50, 52, 53, 54, 55, 55, 55,
    55, 55, 54, 53, 52, 50, 48, 46, 44, 41, 38, 35, 31, 28, 24, 19, 15, 10, 4, 0,
];
const TRACKER_OFFSETS: [u32; 3] = [0x18, 0x1b, 0x1e];

impl StRyuPushback {
    pub(super) fn stun(self) -> u8 {
        self.kind.stun()
    }

    fn step_at(self, elapsed: u8) -> i16 {
        elapsed
            .checked_sub(1)
            .and_then(|index| self.kind.steps().get(usize::from(index)))
            .map_or(0, |&step| i16::from(self.direction) * i16::from(step))
    }
}

impl StRyuRecoil {
    fn steps(self) -> &'static [u8] {
        match self {
            StRyuRecoil::Light => LIGHT_PUSH,
            StRyuRecoil::Medium => MEDIUM_PUSH,
            StRyuRecoil::Heavy => HEAVY_PUSH,
        }
    }

    pub(super) fn stun(self) -> u8 {
        self.steps().len() as u8
    }
}

impl Bus<'_> {
    pub(super) fn st_ken_grounded_kick(&self, fighter: u16) -> Option<(Loadout, u8)> {
        self.st_ken_crouching_kick(self.el(fighter, 0x38), fighter, self.eb(fighter, 0xa))
    }

    pub(super) fn st_ken_crouching_kick(
        &self, player: u32, fighter: u16, move_id: u8,
    ) -> Option<(Loadout, u8)> {
        use crate::era::Revision;
        use crate::roster::TournamentFighter::Ken;

        if move_id != CROUCHING_KICK {
            return None;
        }
        let strength = crate::combo::level(self.eb(fighter, 0x34));
        let style = self.sf2_state(player)?;
        (style.fighter == Some(Ken) && style.style.revision == Revision::St
            && matches!(strength, 1 | 2)).then_some((style.style.loadout, strength))
    }

    pub(super) fn st_ken_late_light_hadoken(&mut self, player: u32, fighter: u16, dir: u8) -> bool {
        if self.eb(fighter, 9) & 0xc0 != 0xc0
            || self.st_ken_grounded_kick(fighter).is_none()
            || self.rb(player) & 0xfe != 0x40
            || self.command_done(player, 0x1c, false) != Some(0x1a)
            || !self.punch_special_ready_for_dir(player, fighter, dir)
        {
            return false;
        }
        // The first input outside Ken's cancel window is discarded while
        // the kick is still running. A fresh input after recovery can fire.
        self.wb(player, 0);
        self.clear_tracker(self.tracker_base(player) + 0x1c);
        self.clear_recovery_buffer(player);
        true
    }

    pub(super) fn st_ryu_sweep_contact(&self, attacker: u16, defender: u16) -> bool {
        self.eb(attacker, 0xa) == 0x18
            && self.st_ryu_loadout(self.el(attacker, 0x38)).is_some()
            && !self.era_airborne(defender)
    }

    pub(super) fn start_st_ryu_sweep(&mut self, attacker: u16, defender: u16) {
        self.start_st_ryu_melee_freeze(attacker, defender, None);
        if self.st_new_ken(defender) {
            let direction = if self.fighter_axis_x(defender) < self.fighter_axis_x(attacker) { -1 } else { 1 };
            let slot = self.combo_slot(defender);
            self.toggle_state.combo.st_ryu_sweep[slot] = Some(StRyuSweep { tick: 0, direction });
        }
    }

    pub(super) fn run_st_ryu_sweep(&mut self, fighter: u16) -> bool {
        let slot = self.combo_slot(fighter);
        let Some(sweep) = self.toggle_state.combo.st_ryu_sweep[slot] else { return false };
        if sweep.tick == 95 {
            self.toggle_state.combo.st_ryu_sweep[slot] = None;
            let player = self.el(fighter, 0x38);
            self.start_move(fighter, player, 1);
            return true;
        }
        let pose = match sweep.tick {
            1 => Some(0x3ed6),
            56 => Some(0x3f1c),
            77 => Some(0x3f3c),
            _ => None,
        };
        if let Some(frame) = pose {
            let axis = self.fighter_axis_x(fighter);
            self.load_frame(fighter, frame);
            self.shift_grid_x(fighter, axis.wrapping_sub(self.fighter_axis_x(fighter)));
        }
        let height = if (17..=56).contains(&sweep.tick) {
            self.shift_grid_x(fighter, (2 * i16::from(sweep.direction)) as u16);
            self.clamp_x(fighter);
            SWEEP_HEIGHT[usize::from(sweep.tick - 17)]
        } else {
            0
        };
        if pose.is_some() || (17..=56).contains(&sweep.tick) {
            self.set_st_ryu_knockdown_height(fighter, height);
        }
        if sweep.tick == 56 {
            self.set_eb(fighter, 9, 0x8a);
            self.finish_knockdown_dust(fighter);
        }
        true
    }

    pub(super) fn st_ryu_close_hk_separation(&self, me: u16, other: u16) -> Option<(i16, u16)> {
        let is_kicking = |entity| self.st_ryu_close_hk(self.el(entity, 0x38), self.eb(entity, 0xa));
        if !((is_kicking(me) && self.st_new_ken(other))
            || (is_kicking(other) && self.st_new_ken(me)))
            || self.era_airborne(me)
            || self.era_airborne(other)
        {
            return None;
        }
        // The arcade uses axis-centered, 16 px half-widths through the
        // close HK poses and tolerates 1 px of overlap. SF1's sprite-centered
        // pushboxes would move Ryu back as his drawing changes.
        let gap = self.fighter_axis_x(me).wrapping_sub(self.fighter_axis_x(other));
        Some((gap as i16, 31))
    }

    fn st_ryu_grounded_kick(&self, attacker: u16) -> Option<(Loadout, bool)> {
        let player = self.el(attacker, 0x38);
        let style = self.ryu_pacing_style(player)?;
        let level = crate::combo::level(self.eb(attacker, 0x34));
        (style.revision == Revision::St
            && matches!(style.loadout, Loadout::Standard | Loadout::Old)
            && self.eb(attacker, 0xa) == CROUCHING_KICK
            && matches!(level, 1 | 2))
        .then_some((style.loadout, level == 2))
    }

    pub(super) fn st_ryu_pushback(
        &self,
        attacker: u16,
        defender: u16,
        guarded: bool,
    ) -> Option<StRyuPushback> {
        if self.era_airborne(defender) {
            return None;
        }
        let player = self.el(attacker, 0x38);
        let move_id = self.eb(attacker, 0xa);
        let kind = if let Some((_, strength)) = self.st_ken_grounded_kick(attacker) {
            if strength == 1 { StRyuRecoil::Light } else { StRyuRecoil::Medium }
        } else {
            self.st_ryu_loadout(player)?;
            match move_id {
                0x0d | CLOSE_LK | FAR_LK => StRyuRecoil::Light,
                CLOSE_MK | FAR_MP | FAR_MK => StRyuRecoil::Medium,
                CLOSE_HK | FAR_HP | FAR_HK => StRyuRecoil::Heavy,
                0x11..=0x13 => StRyuRecoil::Light,
                0x1c..=0x1e if self.eb(attacker, 0x34) == 3 => StRyuRecoil::Heavy,
                0x1c..=0x1e => StRyuRecoil::Medium,
                0x18 if guarded => StRyuRecoil::Heavy,
                CROUCHING_KICK => match (self.st_ryu_grounded_kick(attacker)?.1, guarded) {
                    (false, _) => StRyuRecoil::Light,
                    (true, false) => StRyuRecoil::Medium,
                    (true, true) => return None,
                },
                _ => return None,
            }
        };
        let direction = if self.center_x(defender) < self.center_x(attacker) {
            -1
        } else {
            1
        };
        let defender_slot = self.combo_slot(defender);
        let combo = &self.toggle_state.combo;
        // A second close HK can land while the first reaction is advancing.
        // Its recoil begins immediately after freeze, without a setup tick.
        let rehit = move_id == CLOSE_HK
            && self.sf2_state(player).is_some_and(|era| era.move_hits != 0)
            && (combo.hitstun[defender_slot] != 0 || combo.blockstun[defender_slot] != 0);
        Some(StRyuPushback {
            elapsed: u8::from(rehit),
            direction,
            kind,
        })
    }

    pub(super) fn st_ryu_close_hk_can_rehit(&self, attacker: u16, defender: u16) -> bool {
        let ps = self.el(attacker, 0x38);
        self.st_ryu_close_hk(ps, self.eb(attacker, 0xa))
            && self.sf2_state(ps).is_some_and(|era| era.move_hits == 1)
            && self.toggle_state.combo.st_ryu_pushback[self.combo_slot(defender)]
                .is_some_and(|pushback| pushback.kind == StRyuRecoil::Heavy)
            && (self.combo_reacting(defender)
                || self.toggle_state.combo.blockstun[self.combo_slot(defender)] != 0)
    }

    pub(super) fn st_ryu_close_hk_reaches_reacting_ken(
        &self,
        attacker: u16,
        defender: u16,
    ) -> Option<bool> {
        if !self.st_new_ken(defender) || self.era_airborne(defender) {
            return None;
        }
        // In the source, the second strike reaches 95 px. Its vertical range
        // intersects Ken's hit-reaction torso or his guard pose's head.
        let reach = match self.eb(defender, 0xa) {
            4 => 95 + 16,
            8 => 95 + 26,
            _ => 95 - 13,
        };
        Some(self.st_ryu_standing_gap(attacker, defender) <= reach)
    }

    pub(super) fn start_st_ryu_contact(
        &mut self,
        attacker: u16,
        defender: u16,
        pushback: StRyuPushback,
    ) {
        self.start_st_ryu_melee_freeze(attacker, defender, Some(pushback));
    }

    pub(super) fn start_st_ryu_melee_freeze(
        &mut self,
        attacker: u16,
        defender: u16,
        pushback: Option<StRyuPushback>,
    ) {
        let attacker = self.combo_slot(attacker);
        self.toggle_state.combo.contact_freeze[attacker] = HITSTOP;
        self.start_st_ryu_reaction(defender, pushback);
    }

    pub(super) fn start_st_ryu_reaction(&mut self, defender: u16, pushback: Option<StRyuPushback>) {
        let slot = self.combo_slot(defender);
        let combo = &mut self.toggle_state.combo;
        combo.contact_freeze[slot] = HITSTOP;
        combo.st_ryu_pushback[slot] = pushback;
        combo.frozen_cancel[slot] = None;
    }

    pub(super) fn start_st_ken_shoryureppa_guard(&mut self, attacker: u16, defender: u16) {
        let direction = if self.fighter_axis_x(defender) < self.fighter_axis_x(attacker) { -1 } else { 1 };
        let pushback = StRyuPushback { elapsed: 1, direction, kind: StRyuRecoil::Heavy };
        let slot = self.combo_slot(defender);
        self.start_st_ryu_reaction(defender, Some(pushback));
        self.toggle_state.combo.contact_freeze[slot] = 10;
        self.toggle_state.combo.blockstun[slot] = pushback.stun();
    }

    pub(super) fn st_ken_tatsumaki_recoil(&self, attacker: u16, defender: u16) -> StRyuPushback {
        let direction = if self.fighter_axis_x(defender) < self.fighter_axis_x(attacker) { -1 } else { 1 };
        StRyuPushback { elapsed: 0, direction, kind: StRyuRecoil::Heavy }
    }

    pub(super) fn st_sagat_melee_recoil(
        &self, attacker: u16, defender: u16, phase: u8, blocked: bool,
    ) -> StRyuPushback {
        let direction = if self.fighter_axis_x(defender) < self.fighter_axis_x(attacker) { -1 } else { 1 };
        // A second strike can arrive during the opener's recoil.
        let kind = if self.eb(attacker, 0xa) == 0x19 || (blocked && phase != 1) {
            StRyuRecoil::Heavy
        } else {
            StRyuRecoil::Light
        };
        StRyuPushback { elapsed: 0, direction, kind }
    }

    pub(super) fn st_sagat_normal_recoil(&self, attacker: u16, defender: u16) -> StRyuPushback {
        let direction = if self.fighter_axis_x(defender) < self.fighter_axis_x(attacker) { -1 } else { 1 };
        let kind = match self.eb(attacker, 0x34) {
            1 => StRyuRecoil::Light,
            2 => StRyuRecoil::Medium,
            _ => StRyuRecoil::Heavy,
        };
        let rehit = self.st_sagat_standing_entry(attacker, self.eb(attacker, 0xa))
            .is_some_and(|entry| self.rb(entry + 12) == 2)
            && self.toggle_state.combo.st_melee_phases[self.combo_slot(attacker)] & 1 != 0;
        StRyuPushback { elapsed: u8::from(rehit), direction, kind }
    }

    pub(super) fn start_st_sagat_melee_guard(&mut self, attacker: u16, defender: u16, phase: u8) {
        let genocide = self.st_sagat_genocide(self.el(attacker, 0x38), self.eb(attacker, 0xa));
        let mut pushback = self.st_sagat_melee_recoil(attacker, defender, phase, true);
        if genocide {
            pushback.kind = StRyuRecoil::Heavy;
        }
        self.toggle_state.combo.blockstun[self.combo_slot(defender)] = pushback.stun();
        self.start_st_ryu_reaction(defender, Some(pushback));
        if genocide {
            self.toggle_state.combo.contact_freeze[self.combo_slot(defender)] = 8;
        }
    }

    pub(super) fn contact_frozen(&self, entity: u16) -> bool {
        self.rb(super::SCENE_MODE) == 0x80
            && self.toggle_state.combo.contact_freeze[self.combo_slot(entity)] != 0
    }

    pub(crate) fn contact_freeze(&self) -> [u8; 3] {
        self.toggle_state.combo.contact_freeze
    }

    pub(crate) fn clear_contact_freeze(&mut self) {
        let combo = &mut self.toggle_state.combo;
        combo.contact_freeze = [0; 3];
        combo.st_ryu_pushback = [None; 3];
        combo.st_red_launch = [None; 3];
        combo.st_ryu_sweep = [None; 3];
        combo.st_ken_jump_attack = [None; 3];
        combo.st_ryu_air_cancel = [0; 3];
        combo.st_grounded_super_cancel = [0; 3];
        combo.st_melee_phases = [0; 3];
        combo.frozen_cancel = [None; 3];
    }

    pub(crate) fn tick_contact_freeze(&mut self, before: [u8; 3]) {
        for (current, previous) in self
            .toggle_state
            .combo
            .contact_freeze
            .iter_mut()
            .zip(before)
        {
            if *current == previous {
                *current = current.saturating_sub(1);
            }
        }
    }

    /// The first post-freeze tick loads the reaction. Recoil begins on the
    /// following tick, and the defender can act after its final recoil tick.
    pub(super) fn step_st_ryu_pushback(&mut self, defender: u16, slot: usize) -> bool {
        let Some(mut pushback) = self.toggle_state.combo.st_ryu_pushback[slot] else {
            return false;
        };
        let elapsed = pushback.elapsed;
        if elapsed > pushback.stun() {
            self.toggle_state.combo.st_ryu_pushback[slot] = None;
        } else {
            let dx = pushback.step_at(elapsed);
            pushback.elapsed += 1;
            self.toggle_state.combo.st_ryu_pushback[slot] = Some(pushback);
            if dx != 0 {
                self.shift_grid_x(defender, dx as u16);
                self.clamp_x(defender);
            }
        }
        elapsed <= 1
    }

    pub(super) fn st_ryu_pushback_step(&self, defender: u16) -> Option<i16> {
        self.toggle_state.combo.st_ryu_pushback[self.combo_slot(defender)]
            .map(|pushback| pushback.step_at(pushback.elapsed.saturating_sub(1)))
    }

    pub(super) fn st_ryu_crouching_cancel_window(&self, attacker: u16, guarded: bool) -> Option<u8> {
        if let Some((loadout, strength)) = self.st_ken_grounded_kick(attacker) {
            return Some(if strength == 1 && loadout == Loadout::Old { 17 } else { 16 });
        }
        let loadout = self.st_ryu_loadout(self.el(attacker, 0x38))?;
        let old = loadout == Loadout::Old;
        // Reproduce the recorded successful and late-input pairs at a fixed
        // 60 Hz. World Turbo 2 can sample Old's boundary on either side.
        let window = match self.eb(attacker, 0xa) {
            0x11..=0x13 => if old { 17 } else { 16 },
            0x1c..=0x1e if old && !guarded && self.eb(attacker, 0x34) == 3 => 15,
            0x18 | 0x1c..=0x1e => 16,
            CROUCHING_KICK => {
                let (_, medium) = self.st_ryu_grounded_kick(attacker)?;
                if !medium && old { 17 } else { 16 }
            }
            _ => return None,
        };
        Some(window)
    }

    pub(super) fn capture_frozen_cancel(&mut self, ps: u32, entity: u16, direction: u8) {
        let slot = self.combo_slot(entity);
        if self.toggle_state.cancel.window[slot] == 0
            || self.toggle_state.cancel.move_id[slot] != self.eb(entity, 0xa)
        {
            return;
        }
        self.toggle_state.cancel.window[slot] -= 1;
        if let Some(cancel) = self.frozen_cancel_input(ps, entity, direction) {
            self.toggle_state.combo.frozen_cancel[slot] = Some(cancel);
        }
    }

    fn frozen_cancel_input(&self, ps: u32, entity: u16, direction: u8) -> Option<FrozenCancel> {
        if let Some(side) = self.player_slot_index(ps) {
            let state = self.toggle_state.schemes[side];
            if let Some(normal) = state
                .assist_normal
                .filter(|_| state.assist_can_chain_normal())
            {
                return Some(FrozenCancel::ModernAssist { normal });
            }
            if let Some(shortcut) = state.shortcut {
                return Some(FrozenCancel::ModernShortcut {
                    shortcut,
                    strength: state.shortcut_strength,
                    scaled: state.shortcut_scaled,
                });
            }
        }
        let era_move = self
            .latch_fires(ps, false)
            .then(|| self.era_special_command(ps, false))
            .flatten();
        let ready = era_move.is_some()
            || [false, true].into_iter().any(|kick| {
                self.latch_fires(ps + u32::from(kick), kick)
                    && self.special_ready_for_dir(ps, entity, direction, kick)
            });
        let base = self.tracker_base(ps);
        ready.then(|| FrozenCancel::Classic {
            direction,
            latches: self.rw(ps),
            trackers: TRACKER_OFFSETS.map(|offset| self.rb(base + offset)),
            era_move,
        })
    }

    pub(super) fn release_frozen_cancel(&mut self, ps: u32, entity: u16, other: u16) -> bool {
        let slot = self.combo_slot(entity);
        let Some(cancel) = self.toggle_state.combo.frozen_cancel[slot].take() else {
            return false;
        };
        let started = match cancel {
            FrozenCancel::Classic {
                direction,
                latches,
                trackers,
                era_move,
            } => {
                return self.release_classic_cancel(
                    ps, entity, other, direction, latches, trackers, era_move,
                );
            }
            FrozenCancel::ModernAssist { normal } => self
                .player_slot_index(ps)
                .is_some_and(|side| self.start_modern_assist_normal(ps, entity, side, normal)),
            FrozenCancel::ModernShortcut {
                shortcut,
                strength,
                scaled,
            } => self.player_slot_index(ps).is_some_and(|side| {
                let state = &mut self.toggle_state.schemes[side];
                let saved = (
                    state.shortcut,
                    state.shortcut_strength,
                    state.shortcut_scaled,
                );
                (
                    state.shortcut,
                    state.shortcut_strength,
                    state.shortcut_scaled,
                ) = (Some(shortcut), strength, scaled);
                if self.try_modern_shortcut(ps, entity, true) {
                    return true;
                }
                let state = &mut self.toggle_state.schemes[side];
                (
                    state.shortcut,
                    state.shortcut_strength,
                    state.shortcut_scaled,
                ) = saved;
                false
            }),
        };
        if started {
            let count = &mut self.toggle_state.cancel.count[slot];
            *count = count.wrapping_add(1);
        }
        started
    }

    fn release_classic_cancel(
        &mut self,
        ps: u32,
        entity: u16,
        other: u16,
        direction: u8,
        buffered_latches: u16,
        buffered_trackers: [u8; 3],
        era_move: Option<u8>,
    ) -> bool {
        let latches = self.rw(ps);
        let base = self.tracker_base(ps);
        let trackers = TRACKER_OFFSETS.map(|offset| self.rb(base + offset));
        self.ww(ps, buffered_latches);
        for (offset, value) in TRACKER_OFFSETS.into_iter().zip(buffered_trackers) {
            self.wb(base + offset, value);
        }
        if era_move.is_some_and(|move_id| {
            let stock_available = move_id != crate::era_fight::SF2_SUPER
                || self.sf2_state(ps).is_some_and(|era| era.super_stock_full());
            stock_available && self.start_sf2_special(ps, entity, move_id, false)
        }) {
            let slot = self.combo_slot(entity);
            let count = &mut self.toggle_state.cancel.count[slot];
            *count = count.wrapping_add(1);
            return true;
        }
        if self.cancel_into_special(ps, entity, other, direction) {
            return true;
        }
        self.ww(ps, latches);
        for (offset, value) in TRACKER_OFFSETS.into_iter().zip(trackers) {
            self.wb(base + offset, value);
        }
        false
    }
}
