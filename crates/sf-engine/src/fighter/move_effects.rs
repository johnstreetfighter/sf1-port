//! Contacts and sprite anchors for the local Overhaul effects preview.

use sf_video::callouts::Side;
use sf_video::move_effects::signature::{ImpactKind, ProjectileKind};
use sf_video::render::{HEIGHT, VISAREA_X, VISAREA_Y, WIDTH};

use super::{sext8, Bus, CAMERA, GAME_MODE};
use crate::combo::{class_of, level, Class, HitSource};
use crate::config::RulesetPreset;
use crate::data_bank::{GROUND, SF2_SUPER_SLOT};
use crate::era::EraFamily;
use crate::fight_reader::{
    player_struct, FighterAccent, MoveEffectCues, SignatureFlight, SignatureImpact, FIGHTER_SLOTS,
};
use crate::roster::TournamentFighter;

mod bonus;
mod denjin;
mod kicks;
mod power;
mod roster;

const ZANEI: u8 = 0x29;

impl Bus<'_> {
    fn move_effects_enabled(&self) -> bool {
        self.preset == RulesetPreset::DefinitiveOverhaul && self.dramatic_rules().is_none()
    }

    /// A cue follows the style of the fighter that produces it, even when it
    /// lands on an opponent using a different style.
    fn move_effects_for(&self, player_struct: u32) -> bool {
        self.move_effects_enabled()
            && self
                .era_state(player_struct)
                .is_some_and(|fighter| fighter.style.family != EraFamily::Sf1987)
    }

    fn effect_fighter(&self, side: Side) -> Option<u16> {
        let entity = self.rw(player_struct(side, self.rb(GAME_MODE)) + 0x10);
        (FIGHTER_SLOTS[..2].contains(&entity) && matches!(self.eb(entity, 8), 1 | 2 | 8))
            .then_some(entity)
    }

    fn effect_fighter_side(&self, entity: u16) -> Option<usize> {
        Side::BOTH
            .into_iter()
            .position(|side| self.effect_fighter(side) == Some(entity))
    }

    /// Measure the bottom of the presented pose against the shared stage floor.
    fn pose_above_floor(&self, entity: u16) -> bool {
        let cell = if self.is_big(entity) { 32 } else { 16 };
        let bottom =
            i32::from(self.ew(entity, 4) as i16) + (i32::from(self.eb(entity, 0xc)) + 1) * cell;
        bottom < GROUND - 4
    }

    pub(crate) fn begin_move_effects(&mut self, in_fight: bool) -> [Option<bool>; 2] {
        let native_sparks = self.toggle_state.era.move_effects.native_sparks;
        self.toggle_state.era.move_effects = MoveEffectCues::default();
        if !self.move_effects_enabled() || !in_fight {
            return [None; 2];
        }
        self.toggle_state.era.move_effects.native_sparks = native_sparks;
        Side::BOTH.map(|side| {
            self.effect_fighter(side)
                .filter(|&entity| self.move_effects_for(self.el(entity, 0x38)))
                .map(|entity| self.pose_above_floor(entity))
        })
    }

    pub(crate) fn finish_move_effects(&mut self, in_fight: bool, settling_knockout: bool, before: [Option<bool>; 2]) {
        if !self.move_effects_enabled() || !(in_fight || settling_knockout) {
            self.toggle_state.era.move_effects = MoveEffectCues::default();
            self.toggle_state.era.pending_slam_dust = [false; 2];
            return;
        }
        let mut cues = self.toggle_state.era.move_effects;
        let camera = i32::from(self.rw(CAMERA));
        let flipped = *self.flip_screen;
        cues.camera_x = if flipped { -camera } else { camera };
        if !in_fight {
            // The KO loop only presents dust from the defeated fighter settling.
            let slam_dust = cues.slam_dust;
            cues = MoveEffectCues { slam_dust, camera_x: cues.camera_x, ..MoveEffectCues::default() };
        }
        for side in Side::BOTH {
            let index = side.index();
            let entity = self.effect_fighter(side);
            if !entity.is_some_and(|entity| self.eb(entity, 0xa) >= 0x30) {
                self.toggle_state.era.pending_slam_dust[index] = false;
            }
            if !in_fight {
                continue;
            }
            cues.sf3_parry[index] =
                cues.sf3_parry[index].map(|(x, y)| self.effect_screen_point(x - camera, y));
            let Some(entity) = entity else {
                continue;
            };
            let cx = i32::from(self.center_x(entity) as i16);
            let cy = i32::from(self.center_y(entity) as i16);
            let facing_left = self.eb(entity, 0x37) & 4 != 0;
            let (x, y) = self.effect_screen_point(cx, cy);
            cues.fighter_anchors[index] = Some((x, y, facing_left ^ flipped));
            if !self.move_effects_for(self.el(entity, 0x38)) {
                continue;
            }
            if before[index] == Some(true) && !self.pose_above_floor(entity)
                && self.eb(entity, 0xa) < 0x30 && cues.slam_dust[index].is_none()
            {
                cues.landing[index] = Some(self.effect_screen_point(cx, GROUND - 1));
            }
            if let Some((dx, dy)) = self.ken_fire_anchor(entity) {
                let (x, y) =
                    self.effect_screen_point(cx + if facing_left { -dx } else { dx }, cy + dy);
                cues.ken_fire[index] = Some((x, y, facing_left ^ flipped));
            }
            self.collect_denjin_fighter(&mut cues, index, entity);
            cues.accents[index] = self.collect_accent(entity);
            // Zan'ei's opening advance is invulnerable; its palm strikes clear that bit.
            cues.zanei_rush[index] = self.is_zanei(entity) && self.eb(entity, 0xf) & 0x08 != 0;
        }
        for (index, spark) in cues.native_sparks.iter_mut().enumerate() {
            *spark &= self.eb(0x100 + 0x40 * index as u16, 8) != 0;
        }
        for (index, kind) in self
            .toggle_state
            .era
            .signature_projectiles
            .into_iter()
            .enumerate()
        {
            let slot = 0x100 + 0x40 * index as u16;
            let Some(kind) = kind.filter(|_| self.eb(slot, 8) != 0) else {
                continue;
            };
            let (x, y) = self.effect_centre(slot);
            let (x, y) = self.effect_screen_point(i32::from(x as i16), i32::from(y as i16));
            let flip = (self.ew(slot, 2) & 0x100 != 0) ^ flipped;
            cues.signature_flights[index] = Some(SignatureFlight { kind, x, y, flip });
        }
        // Contacts are captured before a projectile can transform or the camera can move.
        for impact in cues
            .signature_impacts
            .iter_mut()
            .chain(&mut cues.melee_hits)
            .chain(&mut cues.zanei_hits)
            .chain(&mut cues.drive_impact_contacts)
            .chain(&mut cues.slam_dust)
            .flatten()
        {
            let (x, y) = self.effect_screen_point(impact.world_x - camera, impact.y);
            impact.world_x = x + cues.camera_x;
            impact.y = y;
            impact.flip ^= flipped;
        }
        self.toggle_state.era.move_effects = cues;
    }

    pub(super) fn arm_knockdown_dust(&mut self, source: u32, victim: u16, hit: HitSource) {
        if !self.move_effects_enabled() {
            return;
        }
        let Some(side) = self.effect_fighter_side(victim) else { return };
        let reaction = self.eb(victim, 0xa);
        let launched = self.eb(victim, 9) & 1 != 0 && (0x30..=0x3f).contains(&reaction);
        self.toggle_state.era.pending_slam_dust[side] = (launched || reaction >= 0x40)
            && (hit != HitSource::Chip || reaction >= 0x40)
            && self.move_effects_for(source);
    }

    pub(super) fn finish_knockdown_dust(&mut self, victim: u16) {
        if !self.move_effects_enabled() || self.eb(victim, 0xa) < 0x30 {
            return;
        }
        let Some(side) = self.effect_fighter_side(victim) else { return };
        if std::mem::take(&mut self.toggle_state.era.pending_slam_dust[side]) {
            self.record_slam_dust(victim, side);
        }
    }

    pub(super) fn record_throw_dust(&mut self, source: u32, victim: u16) {
        if self.move_effects_for(source) {
            if let Some(side) = self.effect_fighter_side(victim) {
                self.toggle_state.era.pending_slam_dust[side] = false;
                self.record_slam_dust(victim, side);
            }
        }
    }

    fn record_slam_dust(&mut self, victim: u16, side: usize) {
        let (world_x, y) = self.contact_point(self.center_x(victim), (GROUND - 1) as u16);
        self.toggle_state.era.move_effects.slam_dust[side] = Some(SignatureImpact {
            kind: ImpactKind::SlamDust, world_x, y, flip: false,
        });
    }

    fn collect_accent(&self, entity: u16) -> Option<FighterAccent> {
        self.collect_kick_effects(entity)
            .or_else(|| self.collect_roster_effect(entity))
            .or_else(|| self.collect_bonus_effect(entity))
            .or_else(|| self.collect_power_effect(entity))
    }

    fn move_accent_anchor(&self, entity: u16, dx: i32, dy: i32) -> (i32, i32, bool) {
        let facing_left = self.ew(entity, 2) & 0x100 != 0;
        let cx = i32::from(self.center_x(entity) as i16);
        let cy = i32::from(self.center_y(entity) as i16);
        let (x, y) = self.effect_screen_point(cx + if facing_left { -dx } else { dx }, cy + dy);
        (x, y, facing_left ^ *self.flip_screen)
    }

    fn attack_accent_anchor(&self, entity: u16) -> (i32, i32, bool) {
        // The stored attack-box x offset already reflects the fighter's facing.
        let x = self
            .center_x(entity)
            .wrapping_add(sext8(self.eb(entity, 0x24)));
        let y = self
            .center_y(entity)
            .wrapping_sub(sext8(self.eb(entity, 0x22)));
        let (x, y) = self.effect_screen_point(i32::from(x as i16), i32::from(y as i16));
        (x, y, (self.ew(entity, 2) & 0x100 != 0) ^ *self.flip_screen)
    }

    pub(crate) fn signature_projectile_kind(&self, ps: u32, move_id: u8) -> Option<ProjectileKind> {
        if !self.move_effects_for(ps) {
            return None;
        }
        if self.mark_denjin_projectile(ps, move_id) {
            return Some(ProjectileKind::Denjin);
        }
        if self.is_geki_shuriken(ps, move_id) {
            return Some(ProjectileKind::GekiShuriken);
        }
        self.power_projectile(ps, move_id)
    }

    pub(crate) fn record_signature_projectile_contact(&mut self, slot: u16) {
        let Some(index) = crate::era_fight::projectile_index(slot) else {
            return;
        };
        let Some(&Some(kind)) = self.toggle_state.era.signature_projectiles.get(index) else {
            return;
        };
        let kind = match kind {
            ProjectileKind::Denjin => ImpactKind::Denjin,
            ProjectileKind::TigerCannon => ImpactKind::TigerCannon,
            ProjectileKind::GekiShuriken => ImpactKind::GekiShuriken,
            ProjectileKind::Shinku => ImpactKind::Shinku,
            ProjectileKind::Metsu => ImpactKind::Metsu,
            ProjectileKind::Shakunetsu => ImpactKind::Shakunetsu,
        };
        self.record_signature_impact(slot, index, kind);
    }

    pub(super) fn record_demon_spark(&mut self, owner: u32, slot: u16) {
        if self.move_effects_for(owner) {
            if let Some(index) = crate::era_fight::projectile_index(slot) {
                self.record_signature_impact(slot, index, ImpactKind::Satsui);
            }
        }
    }

    fn record_signature_impact(&mut self, slot: u16, index: usize, kind: ImpactKind) {
        let (x, y) = self.effect_centre(slot);
        let (world_x, y) = self.contact_point(x, y);
        let flip = self.ew(slot, 2) & 0x100 != 0;
        self.toggle_state.era.move_effects.signature_impacts[index] = Some(SignatureImpact {
            kind,
            world_x,
            y,
            flip,
        });
    }

    pub(crate) fn projectile_hit_spark(
        &self,
        owner: u32,
        move_id: u8,
        strength: u8,
    ) -> Option<ImpactKind> {
        self.move_effects_for(owner).then(|| {
            let strength = if move_id >= 0x29 { 3 } else { strength };
            hit_spark(strength)
        })
    }

    pub(super) fn projectile_spark(&self, slot: u16) -> Option<ImpactKind> {
        let index = crate::era_fight::projectile_index(slot)?;
        self.toggle_state.era.projectile_sparks[index]
    }

    pub(super) fn record_projectile_hit_spark(
        &mut self,
        slot: u16,
        spark: Option<ImpactKind>,
        transformed: bool,
    ) {
        let Some(index) = crate::era_fight::projectile_index(slot) else {
            return;
        };
        let has_signature = self.toggle_state.era.move_effects.signature_impacts[index].is_some();
        if let Some(kind) = spark.filter(|_| !has_signature) {
            self.record_signature_impact(slot, index, kind);
        }
        if transformed && (has_signature || spark.is_some()) {
            self.hide_move_effect_spark(Some(slot));
        }
    }

    fn is_zanei(&self, entity: u16) -> bool {
        let slot = self.eb(entity, 0xa);
        self.era_state(self.el(entity, 0x38)).is_some_and(|era| {
            era.fighter == Some(TournamentFighter::Gen)
                && ((era.is_alpha() && slot == ZANEI && era.alpha_super_level(slot) != 0)
                    || (era.is_sf4()
                        && matches!(slot, ZANEI | 0x2a)
                        && era.alpha_move_active(slot))
                    || (era.what_if_active()
                        && matches!(
                            era.style.family,
                            EraFamily::Sf2 | EraFamily::Sf3 | EraFamily::Sf5 | EraFamily::Sf6
                        )
                        && slot == ZANEI
                        && era.alpha_move_active(slot)))
        })
    }

    pub(super) fn record_zanei_hit(&mut self, attacker: u16) {
        if !self.move_effects_for(self.el(attacker, 0x38)) || !self.is_zanei(attacker) {
            return;
        }
        let Some(side) = self.effect_fighter_side(attacker) else {
            return;
        };
        let x = self
            .center_x(attacker)
            .wrapping_add(sext8(self.eb(attacker, 0x24)));
        let y = self
            .center_y(attacker)
            .wrapping_sub(sext8(self.eb(attacker, 0x22)));
        let (world_x, y) = self.contact_point(x, y);
        let flip = self.ew(attacker, 2) & 0x100 != 0;
        self.toggle_state.era.move_effects.zanei_hits[side] = Some(SignatureImpact {
            kind: ImpactKind::Zanei,
            world_x,
            y,
            flip,
        });
    }

    fn effect_screen_point(&self, x: i32, y: i32) -> (i32, i32) {
        let x = x - VISAREA_X as i32;
        let y = y - VISAREA_Y as i32;
        if *self.flip_screen {
            (WIDTH as i32 - 1 - x, HEIGHT as i32 - 1 - y)
        } else {
            (x, y)
        }
    }

    fn contact_point(&self, x: u16, y: u16) -> (i32, i32) {
        (
            i32::from(x as i16) + i32::from(self.rw(CAMERA)),
            i32::from(y as i16),
        )
    }

    pub(super) fn hide_move_effect_spark(&mut self, slot: Option<u16>) {
        if let Some(index) = slot.and_then(crate::era_fight::projectile_index) {
            self.toggle_state.era.move_effects.native_sparks[index] = true;
        }
    }

    pub(super) fn spawn_melee_hit_effect(&mut self, attacker: u16, template: u32) {
        let slot = self.spawn_contact_effect(attacker, template, false);
        let owner = self.el(attacker, 0x38);
        if !self.move_effects_for(owner) {
            return;
        }
        let Some(side) = self.effect_fighter_side(attacker) else {
            return;
        };
        if self.has_drive_impact_contact(attacker) || self.is_zanei(attacker) {
            self.hide_move_effect_spark(slot);
            return;
        }
        let move_id = self.eb(attacker, 0xa);
        let strength = level(self.eb(attacker, 0x34));
        let era = self.era_state(owner);
        let kind = if era.is_some_and(|era| {
            era.alpha_super_level(move_id) != 0 || (era.is_sf2() && move_id == SF2_SUPER_SLOT)
        }) {
            ImpactKind::HeavyHit
        } else if era.is_some_and(|era| era.alpha_move_active(move_id)) {
            hit_spark(strength)
        } else {
            match class_of(move_id, strength) {
                Class::Light | Class::Jump(1) => ImpactKind::LightHit,
                Class::Heavy | Class::Sweep | Class::Jump(3) => ImpactKind::HeavyHit,
                Class::Special => hit_spark(strength),
                _ => ImpactKind::MediumHit,
            }
        };
        let (x, y) = slot.map_or_else(
            || {
                let x = self
                    .center_x(attacker)
                    .wrapping_add(sext8(self.eb(attacker, 0x24)));
                let y = self
                    .center_y(attacker)
                    .wrapping_sub(sext8(self.eb(attacker, 0x22)));
                (x, y)
            },
            |slot| self.effect_centre(slot),
        );
        let (world_x, y) = self.contact_point(x, y);
        let flip = self.ew(attacker, 2) & 0x100 != 0;
        self.toggle_state.era.move_effects.melee_hits[side] = Some(SignatureImpact {
            kind,
            world_x,
            y,
            flip,
        });
        self.hide_move_effect_spark(slot);
    }

    pub(crate) fn record_sf3_parry(&mut self, side: usize, contact: (u16, u16)) {
        if side < 2 && self.move_effects_for(player_struct(Side::BOTH[side], self.rb(GAME_MODE))) {
            let point = self.contact_point(contact.0, contact.1);
            self.toggle_state.era.move_effects.sf3_parry[side] = Some(point);
        }
    }

    fn ken_fire_anchor(&self, entity: u16) -> Option<(i32, i32)> {
        let era = self.era_state(self.el(entity, 0x38))?;
        if era.fighter != Some(TournamentFighter::Ken) {
            return None;
        }
        let slot = self.eb(entity, 0xa);
        let eligible = match (era.style.family, slot) {
            (EraFamily::SfZ, 0x19) if era.is_alpha() => level(self.eb(entity, 0x34)) == 3,
            // The shared Shoryureppa / Shinryu Reppa finisher uses this strike entry.
            (EraFamily::SfZ, SF2_SUPER_SLOT) if era.is_alpha() => {
                era.alpha_super_level(SF2_SUPER_SLOT) == 3 && self.el(entity, 0x28) == 0x45f04
            }
            (EraFamily::Sf6, 0x2b) => {
                era.alpha_move_active(slot) && self.el(entity, 0x28) == 0x45f04
            }
            _ => false,
        };
        if !eligible {
            return None;
        }
        // Measured from the two 1987 fist poses, both reused by the banked finisher.
        // A flipped 32 px grid starts at its right-hand column's tile code.
        match (self.ew(entity, 0) & !2, self.eb(entity, 0xc)) {
            (0x0d04, 2) => Some((25, -28)),
            (0x0c8c, 3) => Some((8, -50)),
            _ => None,
        }
    }

    pub(super) fn record_ken_burn(&mut self, attacker: u16, defender: u16) {
        if self.move_effects_for(self.el(attacker, 0x38))
            && self.ken_fire_anchor(attacker).is_some()
        {
            if let Some(side) = self.effect_fighter_side(defender) {
                self.toggle_state.era.move_effects.ken_burn[side] = true;
            }
        }
    }
}

fn hit_spark(strength: u8) -> ImpactKind {
    match strength {
        1 => ImpactKind::LightHit,
        2 => ImpactKind::MediumHit,
        _ => ImpactKind::HeavyHit,
    }
}
