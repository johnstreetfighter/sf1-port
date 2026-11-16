use super::Bus;
use crate::combo::Class;
use crate::data_bank::st_ryu_normals::{
    CLOSE_HK, CLOSE_HP_HEADER, CLOSE_LK, CLOSE_LP_HEADER, CLOSE_MK, CLOSE_MP_HEADER,
    COLLARBONE_BREAKER, CROUCH_HP_HEADER, CROUCH_LP_HEADER, CROUCH_MP_HEADER, FAR_HK, FAR_HP,
    FAR_LK, FAR_MK, FAR_MP, OLD_FAR_LK_HEADER, OLD_FAR_MK_HEADER, SOLAR_PLEXUS_SMASHER, SWEEP_HEADER,
};
use crate::era::EraFamily;
use crate::era::{Loadout, Revision};
use crate::roster::TournamentFighter;

// World 940223 at Turbo 2 selects far LK and MK at an integer X gap of 41.
// Use the same pixel boundary between the port's fighter position axes.
const CLOSE_KICK_RANGE: u16 = 40;

struct ArcadeBox {
    forward: i16,
    height: i16,
    radius_x: u16,
    radius_y: u16,
}

impl ArcadeBox {
    const fn new(forward: i16, height: i16, radius_x: u16, radius_y: u16) -> Self {
        Self { forward, height, radius_x, radius_y }
    }

    fn overlaps(&self, other: &Self, x_gap: i16, same_facing: bool) -> bool {
        let other_x = if same_facing { other.forward } else { -other.forward };
        self.forward.abs_diff(x_gap + other_x) <= self.radius_x + other.radius_x
            && self.height.abs_diff(other.height) <= self.radius_y + other.radius_y
    }
}

const KEN_STANDING: [ArcadeBox; 3] = [
    ArcadeBox::new(4, 79, 12, 8),
    ArcadeBox::new(-6, 52, 20, 21),
    ArcadeBox::new(-6, 14, 20, 16),
];
const KEN_CROUCHING: [ArcadeBox; 3] = [
    ArcadeBox::new(14, 51, 12, 9),
    ArcadeBox::new(6, 33, 22, 12),
    ArcadeBox::new(6, 11, 22, 12),
];
const LIGHT_CROUCH_KICK: ArcadeBox = ArcadeBox::new(56, 9, 33, 12);
const MEDIUM_CROUCH_KICK: ArcadeBox = ArcadeBox::new(58, 9, 40, 12);
const KEN_LIGHT_RECOIL: [ArcadeBox; 3] = [
    ArcadeBox::new(-1, 75, 14, 9),
    ArcadeBox::new(-10, 47, 21, 21),
    ArcadeBox::new(-9, 16, 21, 17),
];
const KEN_MEDIUM_RECOIL: [ArcadeBox; 3] = [
    ArcadeBox::new(-3, 72, 14, 9),
    ArcadeBox::new(-14, 46, 19, 18),
    ArcadeBox::new(-13, 16, 21, 17),
];
const KEN_HEAVY_RECOIL_EARLY: [ArcadeBox; 3] = [
    ArcadeBox::new(-14, 57, 14, 9),
    ArcadeBox::new(-22, 43, 19, 15),
    ArcadeBox::new(-19, 16, 21, 17),
];
const KEN_HEAVY_RECOIL_LATE: [ArcadeBox; 3] = [
    ArcadeBox::new(-8, 81, 14, 9),
    ArcadeBox::new(-9, 51, 21, 20),
    ArcadeBox::new(-3, 18, 23, 18),
];
const RYU_TATSUMAKI_RECOIL_LATE: [ArcadeBox; 3] = [
    ArcadeBox::new(-4, 80, 10, 10),
    ArcadeBox::new(-16, 52, 20, 23),
    ArcadeBox::new(-3, 20, 20, 19),
];

impl Bus<'_> {
    pub(super) fn st_tiger_shot_reaches_ken(
        &self, effect: u16, defender: u16, (x, y, width, height): (u16, u16, u16, u16),
    ) -> Option<bool> {
        if !self.st_new_ken(defender) || self.era_airborne(defender) {
            return None;
        }
        let hurtboxes = match self.eb(defender, 0xa) {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            _ => return None,
        };
        let attack = ArcadeBox::new(0, crate::data_bank::GROUND as i16 - y as i16, width, height);
        let faces_left = self.ew(effect, 2) & 0x100 != 0;
        let same_facing = faces_left == (self.eb(defender, 0x37) & 4 != 0);
        let gap = self.fighter_axis_x(defender).wrapping_sub(x) as i16;
        let forward_gap = if faces_left { gap.wrapping_neg() } else { gap };
        Some(hurtboxes.iter().any(|hurtbox| attack.overlaps(hurtbox, forward_gap, same_facing)))
    }

    pub(super) fn st_ryu_jump_mp_stream(&self, player: u32, fighter: u16, jump: u8) -> Option<u32> {
        if !matches!(jump, 0x0a | 0x0b) || self.eb(fighter, 0x34) != 2 {
            return None;
        }
        let old = self.st_ryu_loadout(player)? == Loadout::Old;
        self.data_bank?.st_ryu_jump_mp_stream(old, jump == 0x0b)
    }

    pub(super) fn st_ryu_directional_jump_mp(&self, fighter: u16) -> Option<Loadout> {
        let mv = self.eb(fighter, 0xa);
        if !matches!(mv, 0x21 | 0x22) || self.eb(fighter, 0x34) != 2 || self.eb(fighter, 9) & 3 != 1
            || !crate::data_bank::in_bank(self.el(fighter, 0x2c))
        {
            return None;
        }
        let owner = self.el(fighter, 0x38);
        let state = self.sf2_state(owner)?;
        if state.alpha_move_active(mv) {
            return None;
        }
        self.st_ryu_loadout(owner)
    }

    pub(super) fn st_ryu_air_tatsumaki_cancel(&self, player: u32, fighter: u16) -> bool {
        let move_id = self.eb(fighter, 0xa);
        let slot = self.combo_slot(fighter);
        // The measured early whiff cancels occur in the first live punch
        // phase. Connected punches have a separate window across hitstop.
        let early_attack = self.eb(fighter, 0xf) & 0x80 != 0
            && self.player_slot_index(player).is_some_and(|side| self.toggle_state.human_attack_frames[side] <= 8);
        self.st_ryu_loadout(player).is_some()
            && move_id == 0x21
            && self.eb(fighter, 9) & 0xc3 == 0xc1
            && self.sf2_state(player).is_some_and(|era| !era.alpha_move_active(move_id))
            && (self.toggle_state.combo.st_ryu_air_cancel[slot] != 0 || early_attack)
            && self.frames_until_landing(fighter) != 0
    }

    pub(super) fn st_ryu_forward_jump_punch(&self, fighter: u16) -> bool {
        self.eb(fighter, 0xa) == 0x21 && self.eb(fighter, 9) & 3 == 1
            && self.st_ryu_loadout(self.el(fighter, 0x38)).is_some()
    }

    pub(super) fn st_standard_hadoken_reaches_shoto(
        &self,
        effect: u16,
        defender: u16,
        owner: TournamentFighter,
        (x, y, width, height): (u16, u16, u16, u16),
    ) -> Option<bool> {
        use crate::combo::StRyuRecoil::{Heavy, Light, Medium};

        let measured_defender = match owner {
            TournamentFighter::Ken => self.st_new_ryu(self.el(defender, 0x38)),
            _ => self.st_new_ken(defender),
        };
        if !measured_defender || self.era_airborne(defender) {
            return None;
        }
        let hurtboxes = match self.eb(defender, 0xa) {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            0x31 => {
                let reaction = self.toggle_state.combo.st_ryu_pushback[self.combo_slot(defender)]?;
                match reaction.kind {
                    Light => &KEN_LIGHT_RECOIL,
                    Medium => &KEN_MEDIUM_RECOIL,
                    Heavy if reaction.elapsed >= 13 => &KEN_HEAVY_RECOIL_LATE,
                    Heavy => &KEN_HEAVY_RECOIL_EARLY,
                }
            }
            _ => return None,
        };
        let attack = ArcadeBox::new(0, crate::data_bank::GROUND as i16 - y as i16, width, height);
        let faces_left = self.ew(effect, 2) & 0x100 != 0;
        let same_facing = faces_left == (self.eb(defender, 0x37) & 4 != 0);
        let gap = self.fighter_axis_x(defender).wrapping_sub(x) as i16;
        let forward_gap = if faces_left { gap.wrapping_neg() } else { gap };
        Some(hurtboxes.iter().any(|hurtbox| attack.overlaps(hurtbox, forward_gap, same_facing)))
    }

    /// For the measured Ken poses, test the arcade rectangles independently
    /// of the artwork used by the translated engine.
    pub(super) fn st_ryu_normal_reaches_ken(&self, attacker: u16, defender: u16) -> Option<bool> {
        if !self.st_new_ken(defender) || self.era_airborne(defender) {
            return None;
        }
        let hurtboxes = match self.eb(defender, 0xa) {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            _ => return None,
        };
        let player = self.el(attacker, 0x38);
        let loadout = self.st_ryu_loadout(player)?;
        let attack = match self.eb(attacker, 0xa) {
            0x0d => {
                // Both jab headers keep the standing LP move ID. Identify
                // the close stream at its single active record.
                let close_stream = self.rl(self.st_ryu_close_lp_header(player)? + 0x2c);
                if self.el(attacker, 0x2c) == close_stream + 0x20 {
                    ArcadeBox::new(33, 85, 19, 8)
                } else if loadout == Loadout::Old {
                    ArcadeBox::new(35, 75, 25, 9)
                } else {
                    ArcadeBox::new(35, 69, 25, 3)
                }
            }
            CLOSE_LK => ArcadeBox::new(52, 21, 33, 21),
            CLOSE_MK => ArcadeBox::new(53, 57, 20, 21),
            FAR_LK | FAR_MK => ArcadeBox::new(16, 83, 33, 14),
            // World 940223, first active rectangles from the Turbo 2
            // 32 px clean and crouch-guard fixtures.
            0x17 if loadout == Loadout::Standard => match self.eb(attacker, 0x34) {
                1 => LIGHT_CROUCH_KICK,
                2 => MEDIUM_CROUCH_KICK,
                _ => return None,
            },
            _ => return None,
        };
        Some(self.st_arcade_normal_overlap(attacker, defender, &attack, hurtboxes))
    }

    pub(super) fn st_ken_crouching_kick_reaches_ryu(&self, attacker: u16, defender: u16) -> Option<bool> {
        let (_, strength) = self.st_ken_grounded_kick(attacker)?;
        if !self.st_new_ryu(self.el(defender, 0x38)) || self.era_airborne(defender) {
            return None;
        }
        // The audited New Ryu neutral and crouch guard poses use the
        // same measured boxes as the corresponding New Ken poses.
        let hurtboxes = match self.eb(defender, 0xa) {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            _ => return None,
        };
        let attack = if strength == 1 { &LIGHT_CROUCH_KICK } else { &MEDIUM_CROUCH_KICK };
        Some(self.st_arcade_normal_overlap(attacker, defender, attack, hurtboxes))
    }

    pub(super) fn st_ken_shoryureppa_reaches_ryu(
        &self, attacker: u16, defender: u16, phase: u8,
    ) -> Option<bool> {
        if phase == 0 {
            return None;
        }
        if !self.st_new_ryu(self.el(defender, 0x38)) || self.era_airborne(defender) {
            return None;
        }
        let stance = self.eb(defender, 0xa);
        let hurtboxes = match stance {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            _ => return None,
        };
        let height = self.st_ryu_height_above_floor(attacker);
        let (x, y, rx, ry) = match phase {
            1 | 4 => (30, 45, 28, 22),
            2 | 5 => (27, 79, 26, 44),
            3 => (24, 49, 25, 13),
            _ => return None,
        };
        // The source keeps a fractional axis through the first hop and its
        // guard push. Allow for rounding at the grounded final strike.
        let rx = rx + u16::from(phase == 5 && height == 0 && stance == 4);
        let strike = ArcadeBox::new(x, y + height, rx, ry);
        Some(self.st_arcade_normal_overlap(attacker, defender, &strike, hurtboxes))
    }

    pub(super) fn st_ken_tatsumaki_reaches_ryu(
        &self, attacker: u16, defender: u16, strike: u8,
    ) -> Option<bool> {
        if !self.st_new_ryu(self.el(defender, 0x38)) || self.era_airborne(defender) {
            return None;
        }
        let hurtboxes = match self.eb(defender, 0xa) {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            0x31 => {
                let recoil = self.toggle_state.combo.st_ryu_pushback[self.combo_slot(defender)]?;
                if recoil.elapsed >= 13 { &RYU_TATSUMAKI_RECOIL_LATE } else { &KEN_HEAVY_RECOIL_EARLY }
            }
            _ => return None,
        };
        let (x, y, rx, ry) = crate::data_bank::st_ken_tatsumaki::attack_box(strike);
        let strike = ArcadeBox::new(x, y + self.st_ryu_height_above_floor(attacker), rx, ry);
        Some(self.st_arcade_normal_overlap(attacker, defender, &strike, hurtboxes))
    }

    pub(super) fn st_ken_jump_reaches_ryu(&self, attacker: u16, defender: u16) -> Option<bool> {
        if !self.st_new_ryu(self.el(defender, 0x38)) || self.era_airborne(defender) {
            return None;
        }
        const WALKING: [ArcadeBox; 3] = [
            ArcadeBox::new(9, 79, 12, 8),
            ArcadeBox::new(-6, 52, 20, 21),
            ArcadeBox::new(-6, 14, 20, 16),
        ];
        let hurtboxes = match self.eb(defender, 0xa) {
            2 | 3 => &WALKING,
            1 | 4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            _ => return None,
        };
        let offset = self.eb(attacker, 0x26) as i8 as i16 + self.eb(attacker, 0x24) as i8 as i16;
        let forward = if self.eb(attacker, 0x37) & 4 != 0 { -offset } else { offset };
        let vertical = self.st_ryu_height_above_floor(attacker)
            + Self::half_extent(self.eb(attacker, 0xc), self.is_big(attacker)) as i16
            + i16::from(self.eb(attacker, 0x22) as i8);
        let strike = ArcadeBox::new(
            forward,
            vertical,
            u16::from(self.eb(attacker, 0x25)),
            u16::from(self.eb(attacker, 0x23)),
        );
        Some(self.st_arcade_normal_overlap(attacker, defender, &strike, hurtboxes))
    }

    pub(super) fn st_sagat_normal_reaches_ken(
        &self, attacker: u16, defender: u16, move_id: u8,
    ) -> Option<bool> {
        if !self.st_new_ken(defender) || self.era_airborne(defender) {
            return None;
        }
        let standing = self.st_sagat_standing_entry(attacker, move_id);
        let hurtboxes = match self.eb(defender, 0xa) {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            0x31 if standing.is_some() => &KEN_LIGHT_RECOIL,
            _ => return None,
        };
        let strike = if let Some(entry) = standing {
            ArcadeBox::new(
                i16::from(self.rb(entry + 8)), i16::from(self.rb(entry + 9)),
                u16::from(self.rb(entry + 10)), u16::from(self.rb(entry + 11)),
            )
        } else {
            let (x, y, rx, ry) =
                crate::data_bank::st_sagat_normals::attack_box(move_id, self.eb(attacker, 0x34))?;
            ArcadeBox::new(x as i16, y as i16, u16::from(rx), u16::from(ry))
        };
        Some(self.st_arcade_normal_overlap(attacker, defender, &strike, hurtboxes))
    }

    pub(super) fn st_sagat_air_reaches_ken(&self, attacker: u16, defender: u16, entry: u32) -> Option<bool> {
        if !self.st_new_ken(defender) || self.era_airborne(defender) {
            return None;
        }
        let hurtboxes = match self.eb(defender, 0xa) {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            _ => return None,
        };
        let strike = ArcadeBox::new(
            i16::from(self.rb(entry + 8)),
            self.st_ryu_height_above_floor(attacker) + i16::from(self.rb(entry + 9)),
            u16::from(self.rb(entry + 10)),
            u16::from(self.rb(entry + 11)),
        );
        let facing = self.eb(attacker, 0x37) & 4;
        let gap = self.fighter_axis_x(defender).wrapping_sub(self.fighter_axis_x(attacker)) as i16;
        let forward_gap = if facing == 0 { gap } else { -gap };
        // A settled Ken faces the jumping Sagat. His held crouch may carry
        // the old grid facing until its next grounded pose change.
        Some(hurtboxes.iter().any(|hurtbox| strike.overlaps(hurtbox, forward_gap, forward_gap < 0)))
    }

    pub(super) fn st_sagat_melee_reaches_ken(
        &self, attacker: u16, defender: u16, move_id: u8, phase: u8,
    ) -> Option<bool> {
        if !self.st_new_ken(defender) || self.era_airborne(defender) {
            return None;
        }
        let hurtboxes = match self.eb(defender, 0xa) {
            1..=4 => &KEN_STANDING,
            5..=8 => &KEN_CROUCHING,
            0x31 if move_id == 0x19 => &[
                ArcadeBox::new(-42, 76, 10, 10),
                ArcadeBox::new(-33, 47, 20, 23),
                ArcadeBox::new(-13, 17, 20, 19),
            ],
            0x31 => &KEN_MEDIUM_RECOIL,
            _ => return None,
        };
        let (x, y, rx, ry) = crate::data_bank::st_sagat_melee::attack_box(move_id, phase)?;
        let height = self.st_ryu_height_above_floor(attacker);
        let strike = ArcadeBox::new(x as i16, y as i16 + height, u16::from(rx), u16::from(ry));
        Some(self.st_arcade_normal_overlap(attacker, defender, &strike, hurtboxes))
    }

    pub(super) fn st_ryu_height_above_floor(&self, entity: u16) -> i16 {
        let tile = if self.is_big(entity) { 32 } else { 16 };
        let bottom = self.ew(entity, 4) as i16 + (i16::from(self.eb(entity, 0xc)) + 1) * tile;
        crate::data_bank::GROUND as i16 - bottom
    }

    fn st_arcade_normal_overlap(
        &self, attacker: u16, defender: u16, attack: &ArcadeBox, hurtboxes: &[ArcadeBox],
    ) -> bool {
        let facing = self.eb(attacker, 0x37) & 4;
        let same_facing = facing == self.eb(defender, 0x37) & 4;
        let gap = self.fighter_axis_x(defender).wrapping_sub(self.fighter_axis_x(attacker)) as i16;
        let forward_gap = if facing == 0 { gap } else { -gap };
        hurtboxes.iter().any(|hurtbox| attack.overlaps(hurtbox, forward_gap, same_facing))
    }

    pub(super) fn st_ryu_standing_gap(&self, me: u16, other: u16) -> u16 {
        let axis = |entity| self.fighter_axis_x(entity) as i16;
        axis(me).abs_diff(axis(other))
    }

    pub(super) fn st_ryu_loadout(&self, ps: u32) -> Option<Loadout> {
        let state = self.sf2_state(ps)?;
        (state.fighter == Some(TournamentFighter::Ryu) && state.style.revision == Revision::St
            && self.evil_ryu_profile(ps).is_none())
            .then_some(state.style.loadout)
    }

    pub(super) fn st_new_ryu(&self, ps: u32) -> bool {
        self.st_ryu_loadout(ps) == Some(Loadout::Standard)
    }

    pub(super) fn st_ryu_command_normal(&self, ps: u32, move_id: u8) -> bool {
        matches!(move_id, COLLARBONE_BREAKER | SOLAR_PLEXUS_SMASHER) && self.st_new_ryu(ps)
    }

    pub(super) fn st_ryu_close_hk(&self, ps: u32, move_id: u8) -> bool {
        move_id == CLOSE_HK && self.st_ryu_loadout(ps).is_some()
    }

    pub(super) fn st_ryu_collarbone_breaker(&self, ps: u32, move_id: u8) -> bool {
        move_id == COLLARBONE_BREAKER && self.st_new_ryu(ps)
    }

    pub(super) fn st_ryu_close_lk(&self, ps: u32, move_id: u8) -> bool {
        move_id == CLOSE_LK && self.st_ryu_loadout(ps).is_some()
    }

    pub(super) fn st_ryu_banked_standing_hit(&self, ps: u32, move_id: u8) -> bool {
        matches!(move_id, FAR_MK | FAR_MP | FAR_HP | FAR_HK) && self.st_ryu_loadout(ps).is_some()
    }

    pub(super) fn st_ryu_standing_class(&self, ps: u32, move_id: u8) -> Option<Class> {
        match move_id {
            CLOSE_LK if self.st_ryu_loadout(ps).is_some() => Some(Class::Light),
            CLOSE_MK if self.st_ryu_loadout(ps).is_some() => Some(Class::Medium),
            CLOSE_HK if self.st_ryu_loadout(ps).is_some() => Some(Class::Heavy),
            COLLARBONE_BREAKER if self.st_new_ryu(ps) => Some(Class::Medium),
            SOLAR_PLEXUS_SMASHER if self.st_new_ryu(ps) => Some(Class::Heavy),
            _ => None,
        }
    }

    pub(super) fn st_ryu_standing_header(&self, ps: u32, entity: u16, move_id: u8) -> Option<u32> {
        let loadout = self.st_ryu_loadout(ps)?;
        let within = |range| {
            let (_, opponent) = self.player_entities(ps);
            opponent != 0 && self.st_ryu_standing_gap(entity, opponent) <= range
        };
        let slot = match (move_id, loadout) {
            (FAR_LK, Loadout::Old) => OLD_FAR_LK_HEADER,
            (FAR_MK, Loadout::Old) => OLD_FAR_MK_HEADER,
            (FAR_MP, _) if within(70) => CLOSE_MP_HEADER,
            (FAR_HP, _) if within(80) => CLOSE_HP_HEADER,
            (FAR_LK | FAR_MK | FAR_MP | FAR_HP | FAR_HK, _) => move_id,
            _ => return None,
        };
        self.data_bank?.era_move_header(TournamentFighter::Ryu, EraFamily::Sf2, slot)
    }

    pub(super) fn st_ryu_close_lp_header(&self, ps: u32) -> Option<u32> {
        self.st_ryu_loadout(ps)?;
        self.data_bank?.era_move_header(TournamentFighter::Ryu, EraFamily::Sf2, CLOSE_LP_HEADER)
    }

    pub(super) fn st_ryu_crouching_header(&self, ps: u32, entity: u16, move_id: u8) -> Option<u32> {
        self.st_ryu_loadout(ps)?;
        let slot = match move_id {
            0x11..=0x13 => CROUCH_LP_HEADER,
            0x1c..=0x1e if self.eb(entity, 0x34) == 3 => CROUCH_HP_HEADER,
            0x1c..=0x1e => CROUCH_MP_HEADER,
            0x18 => SWEEP_HEADER,
            _ => return None,
        };
        self.data_bank?.era_move_header(TournamentFighter::Ryu, EraFamily::Sf2, slot)
    }

    pub(super) fn st_ryu_crouching_punch(&self, ps: u32, move_id: u8) -> bool {
        matches!(move_id, 0x11..=0x13 | 0x1c..=0x1e) && self.st_ryu_loadout(ps).is_some()
    }

    pub(super) fn try_st_ryu_close_kick(&mut self, ps: u32, me: u16, other: u16, dir: u8) -> bool {
        let Some(loadout) = self.st_ryu_loadout(ps) else { return false };
        if !matches!(dir, 0..=2) || self.latch_fires(ps, false)
            || self.is_neg_edge_latch(ps + 1) || self.kick_special_ready_for_dir(ps, me, dir)
        {
            return false;
        }
        let (slot, strength, range) = match self.rb(ps + 1) & 0xfe {
            0x40 => (CLOSE_LK, 1, CLOSE_KICK_RANGE),
            0xe0 => (CLOSE_MK, 2, CLOSE_KICK_RANGE),
            0xfe => (CLOSE_HK, 3, if loadout == Loadout::Old { 79 } else { 95 }),
            _ => return false,
        };
        if self.st_ryu_standing_gap(me, other) > range || !self.era_start_banked_move(ps, me, slot, 0, None) {
            return false;
        }
        self.set_eb(me, 0x34, strength);
        true
    }

    pub(super) fn st_ryu_forward_mp(&self, ps: u32, me: u16, dir: u8) -> bool {
        self.st_ryu_forward_punch(ps, me, dir) == Some((COLLARBONE_BREAKER, 2))
    }

    pub(super) fn st_ryu_forward_punch(&self, ps: u32, me: u16, dir: u8) -> Option<(u8, u8)> {
        if !self.st_new_ryu(ps) || self.relative_directions(me, dir) != 1
            || self.is_neg_edge_latch(ps)
        {
            return None;
        }
        match self.rb(ps) & 0xfe {
            0xe0 => Some((COLLARBONE_BREAKER, 2)),
            0xfe => Some((SOLAR_PLEXUS_SMASHER, 3)),
            _ => None,
        }
    }

    pub(super) fn try_st_ryu_command_normal(&mut self, ps: u32, me: u16, dir: u8) -> bool {
        let Some((slot, strength)) = self.st_ryu_forward_punch(ps, me, dir) else {
            return false;
        };
        if self.punch_special_ready_for_dir(ps, me, dir) || !self.era_start_banked_move(ps, me, slot, 0, None) {
            return false;
        }
        self.set_eb(me, 0x34, strength);
        true
    }
}
