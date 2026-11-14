//! Local presentation for the move effects playtest.

use sf_video::compositor::{AfterimageTrail, IndexedSprite, PaletteTint};
use sf_video::move_effects::accent::accent_frame;
use sf_video::move_effects::signature::{
    denjin_hands, signature_impact, signature_projectile, ImpactKind,
};
use sf_video::move_effects::{effect_frame, EffectKind};

use crate::fight_reader::{MoveEffectCues, SignatureImpact};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OneShot {
    kind: EffectKind,
    age: u8,
    world_x: i32,
    y: i32,
    flip: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct MoveEffects {
    cues: MoveEffectCues,
    one_shots: Vec<OneShot>,
    fire_age: [u8; 2],
    burn_left: [u8; 2],
    accent_age: [u8; 2],
    signature_impacts: Vec<(SignatureImpact, u8)>,
    pub(super) zanei_trails: [AfterimageTrail; 2],
}

impl MoveEffects {
    pub(super) fn step(&mut self, cues: MoveEffectCues) {
        self.signature_impacts.retain_mut(|(impact, age)| {
            *age += 1;
            *age < impact.kind.lifetime()
        });
        self.signature_impacts.extend(
            cues.signature_impacts
                .into_iter()
                .chain(cues.melee_hits)
                .chain(cues.zanei_hits)
                .chain(cues.drive_impact_contacts)
                .chain(cues.slam_dust)
                .flatten()
                .map(|impact| (impact, 0)),
        );
        for (previous, current) in [
            (self.cues.denjin_charge_snap, cues.denjin_charge_snap),
            (self.cues.denjin_release, cues.denjin_release),
        ] {
            for (old, new) in previous.into_iter().zip(current) {
                if let Some((x, y, flip)) = new.filter(|_| old.is_none()) {
                    self.signature_impacts.push((
                        SignatureImpact {
                            kind: ImpactKind::Denjin,
                            world_x: x + cues.camera_x,
                            y,
                            flip,
                        },
                        0,
                    ));
                }
            }
        }
        let camera_delta = self.cues.camera_x - cues.camera_x;
        for (trail, rushing) in self.zanei_trails.iter_mut().zip(cues.zanei_rush) {
            if rushing {
                trail.shift_x(camera_delta);
            } else {
                trail.clear();
            }
        }
        self.one_shots.retain_mut(|effect| {
            effect.age += 1;
            effect.age < effect.kind.lifetime()
        });
        for (kind, points) in [
            (EffectKind::LandingDust, cues.landing),
            (EffectKind::Parry, cues.sf3_parry),
        ] {
            for (side, point) in points.into_iter().enumerate() {
                let Some((x, y)) = point else { continue };
                let flip = kind == EffectKind::Parry
                    && cues.fighter_anchors[side].is_some_and(|(_, _, flip)| flip);
                self.one_shots.push(OneShot {
                    kind,
                    age: 0,
                    world_x: x + cues.camera_x,
                    y,
                    flip,
                });
            }
        }
        for side in 0..2 {
            let old_accent = self.cues.accents[side].map(|(kind, ..)| kind);
            let accent = cues.accents[side].map(|(kind, ..)| kind);
            self.accent_age[side] = if accent.is_some() && accent == old_accent {
                self.accent_age[side].wrapping_add(1)
            } else {
                0
            };
            self.fire_age[side] =
                if cues.ken_fire[side].is_some() && self.cues.ken_fire[side].is_some() {
                    (self.fire_age[side] + 1) % EffectKind::KenFire.lifetime()
                } else {
                    0
                };
            self.burn_left[side] = if cues.ken_burn[side] {
                EffectKind::KenBurn.lifetime()
            } else {
                self.burn_left[side].saturating_sub(1)
            };
        }
        self.cues = cues;
    }

    pub(super) fn settle_knockout(&mut self, cues: MoveEffectCues) {
        self.signature_impacts.retain(|(impact, _)| impact.kind == ImpactKind::SlamDust);
        let signature_impacts = std::mem::take(&mut self.signature_impacts);
        *self = Self { signature_impacts, ..Self::default() };
        self.step(cues);
    }

    pub(super) fn zanei_rush(&self) -> [bool; 2] {
        self.cues.zanei_rush
    }

    pub(super) fn hide_native_sparks(&self, objects: &mut [u8]) {
        for (object, replaced) in objects[0x100..0x400]
            .chunks_exact_mut(0x40)
            .zip(self.cues.native_sparks)
        {
            if replaced {
                object[..8].fill(0);
            }
        }
    }

    pub(super) fn burn_tint(&self, side: usize) -> Option<PaletteTint> {
        let left = self.burn_left[side];
        (left > 12).then(|| {
            let color = if left % 6 < 3 { 0xffffdb72 } else { 0xffff6c24 };
            PaletteTint {
                color,
                strength: 180,
            }
        })
    }

    pub(super) fn sprites(&self, tick: u32) -> Vec<IndexedSprite> {
        let mut sprites: Vec<_> = self
            .one_shots
            .iter()
            .filter_map(|effect| {
                effect_frame(
                    effect.kind,
                    effect.age,
                    effect.world_x - self.cues.camera_x,
                    effect.y,
                    effect.flip,
                )
            })
            .collect();
        for side in 0..2 {
            if let Some((kind, x, y, flip)) = self.cues.accents[side] {
                sprites.extend(accent_frame(kind, self.accent_age[side], x, y, flip));
            }
            if let Some((x, y, flip)) = self.cues.ken_fire[side] {
                sprites.extend(effect_frame(
                    EffectKind::KenFire,
                    self.fire_age[side],
                    x,
                    y,
                    flip,
                ));
            }
            if let Some((x, y, flip)) =
                self.cues.fighter_anchors[side].filter(|_| self.burn_left[side] != 0)
            {
                let age = EffectKind::KenBurn.lifetime() - self.burn_left[side];
                sprites.extend(effect_frame(EffectKind::KenBurn, age, x, y, flip));
            }
        }
        let age = (tick % 24) as u8;
        for (x, y, flip) in self.cues.denjin_hands.into_iter().flatten() {
            sprites.extend(denjin_hands(age, x, y, flip));
        }
        for flight in self.cues.signature_flights.into_iter().flatten() {
            sprites.extend(signature_projectile(
                flight.kind,
                age,
                flight.x,
                flight.y,
                flight.flip,
            ));
        }
        for (impact, age) in &self.signature_impacts {
            sprites.extend(signature_impact(
                impact.kind,
                *age,
                impact.world_x - self.cues.camera_x,
                impact.y,
                impact.flip,
            ));
        }
        sprites
    }

    pub(super) fn demon_sparks(&self) -> Vec<IndexedSprite> {
        self.signature_impacts
            .iter()
            .filter(|(impact, _)| impact.kind == ImpactKind::Satsui)
            .flat_map(|(impact, age)| {
                signature_impact(
                    impact.kind,
                    *age,
                    impact.world_x - self.cues.camera_x,
                    impact.y,
                    impact.flip,
                )
            })
            .collect()
    }
}
