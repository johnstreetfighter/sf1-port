//! Visual accents on the existing roster move streams.

use super::Bus;
use crate::data_bank::GROUND;
use crate::era::EraFamily::{Sf1Plus, Sf4, Sf5, Sf6, SfZ};
use crate::fight_reader::FighterAccent;
use crate::roster::TournamentFighter::{Birdie, Eagle, Joe, Lee, Mike, Retsu};
use sf_video::move_effects::accent::AccentKind::*;

impl Bus<'_> {
    pub(super) fn collect_roster_effect(&self, entity: u16) -> Option<FighterAccent> {
        let state = self.era_state(self.el(entity, 0x38))?;
        let move_id = self.eb(entity, 0xa);
        let what_if = state.what_if_active() && state.alpha_move_active(move_id);
        let active = self.eb(entity, 0xf) & 0x80 != 0;
        let attr = self.ew(entity, 2);
        let mut pose = self.ew(entity, 0);
        if attr & 0x100 != 0 {
            let code_step = if self.is_big(entity) { 2 } else { 1 };
            pose = pose.wrapping_sub(u16::from(self.eb(entity, 0xd)) * code_step);
        }

        let family = state.style.family;
        let (kind, (x, y, flip)) = match (state.fighter?, family, move_id, pose) {
            // The Bull Head ROM stream and The Birdie's three rushes share this
            // headbutt pose. Keep the wake behind him on the floor.
            (Birdie, Sf1Plus | SfZ | Sf5, 0x1a, 0x28a0) => {
                (BirdieRush, self.birdie_wake_anchor(entity))
            }
            (Birdie, Sf5, 0x20, 0x28a0) => (BirdieSuper, self.birdie_wake_anchor(entity)),
            (Birdie, _, 0x19, 0x28a0) if what_if => (BirdieRush, self.birdie_wake_anchor(entity)),
            (Birdie, SfZ, 0x29, 0x2726 | 0x28a0) if state.alpha_super_level(move_id) != 0 => {
                (BirdieSuper, self.birdie_wake_anchor(entity))
            }
            (Birdie, _, 0x29 | 0x2a, 0x2726 | 0x28a0)
                if what_if && move_id == if family == Sf4 { 0x2a } else { 0x29 } =>
            {
                (BirdieSuper, self.birdie_wake_anchor(entity))
            }

            // Manchester Black's four alternating stick-swing poses, plus
            // Canterbury Blue and Oxford Red's thrusts.
            (Eagle, Sf1Plus | SfZ, 0x1a, 0x2b00 | 0x2b08 | 0x2a4a | 0x23a8)
            | (Eagle, SfZ, 0x2c, 0x2b00 | 0x2980)
            | (Eagle, SfZ, 0x2d, 0x2980)
            | (Eagle, Sf1Plus, 0x1b, 0x2a44)
            | (Eagle, SfZ, 0x2f, 0x2a44) => (EagleStrike, self.attack_accent_anchor(entity)),
            // St Andrews Green's planted rising swing uses banked art.
            (Eagle, SfZ, 0x1f, _) if active => (EagleStrike, self.attack_accent_anchor(entity)),
            // Manchester Gold and Union Jack Platinum both finish with
            // banked strike poses. Their hit box follows the striking stick.
            (Eagle, SfZ, 0x29 | 0x2a, _) if active && state.alpha_super_level(move_id) != 0 => {
                (EagleSuper, self.attack_accent_anchor(entity))
            }
            // The What-If bank reuses the Alpha swings and supers and adds
            // Clobbering and a separate strike on a successful riposte.
            (Eagle, _, 0x1a, 0x2b00 | 0x2b08 | 0x2a4a | 0x23a8)
            | (Eagle, _, 0x20, 0x2b00 | 0x2980)
            | (Eagle, _, 0x21, 0x2980)
            | (Eagle, _, 0x22, 0x2a44)
            | (Eagle, _, 0x19 | 0x23, 0x2b00)
                if what_if =>
            {
                (EagleStrike, self.attack_accent_anchor(entity))
            }
            (Eagle, _, 0x29..=0x2b, _) if what_if && active => {
                (EagleSuper, self.attack_accent_anchor(entity))
            }

            // Kiai Shot has no hand hit box. Use the release pose itself to
            // place the ring and let the existing projectile render normally.
            (Retsu, _, 0x1a, 0x1320) | (Retsu, Sf6, 0x24 | 0x29, 0x1320) => {
                (RetsuKiai, self.move_accent_anchor(entity, 43, -32))
            }
            (Retsu, Sf6, 0x19 | 0x25, 0x1248) => {
                (RetsuKick, self.move_accent_anchor(entity, 32, 24))
            }
            (Retsu, Sf6, 0x2a | 0x2b, 0x1320) => {
                (RetsuKiai, self.move_accent_anchor(entity, 43, -32))
            }
            (Retsu, _, 0x20 | 0x29..=0x2b, 0x1320) if what_if => {
                (RetsuKiai, self.move_accent_anchor(entity, 43, -32))
            }
            (Retsu, _, 0x1b | 0x1f | 0x29..=0x2b, 0x1248) if what_if => {
                (RetsuKick, self.move_accent_anchor(entity, 32, 24))
            }

            // Anchor to the hands or feet in the original poses. The banked
            // series use standardized gameplay boxes for attacks of different
            // heights, so their centers are not good visual anchors.
            (Lee, _, 0x20 | 0x29..=0x2b, 0x1d20) if what_if => {
                (LeePalm, self.move_accent_anchor(entity, 40, -24))
            }
            (Lee, _, 0x1a, _) if active && (family == Sf1Plus || what_if) => {
                (LeePalm, self.attack_accent_anchor(entity))
            }
            (Mike, Sf1Plus, 0x1a, 0x14e0) => (MikePunch, self.move_accent_anchor(entity, 38, -24)),
            (Mike, _, 0x19, _) if active && (family == Sf1Plus || what_if) => {
                (MikePunch, self.attack_accent_anchor(entity))
            }
            (Mike, _, 0x1a | 0x1b | 0x29..=0x2b, 0x14e0) if what_if => {
                (MikePunch, self.move_accent_anchor(entity, 38, -24))
            }
            (Mike, _, 0x1b | 0x29..=0x2b, 0x1560) if what_if => {
                (MikePunch, self.move_accent_anchor(entity, 40, -32))
            }
            (Mike, _, 0x29..=0x2b, 0x16a0) if what_if => {
                (MikePunch, self.move_accent_anchor(entity, 38, -16))
            }
            (Joe, _, 0x1f | 0x21 | 0x29..=0x2b, 0x1944) if what_if => {
                (JoeKick, self.move_accent_anchor(entity, 40, 8))
            }
            (Joe, _, 0x20 | 0x29..=0x2b, 0x1826) if what_if => {
                (JoeKick, self.move_accent_anchor(entity, 32, -40))
            }
            (Joe, _, 0x1b, _) if active && (family == Sf1Plus || what_if) => {
                (JoeKick, self.attack_accent_anchor(entity))
            }
            _ => return None,
        };
        Some((kind, x, y, flip))
    }

    fn birdie_wake_anchor(&self, entity: u16) -> (i32, i32, bool) {
        let dy = GROUND - 2 - i32::from(self.center_y(entity) as i16);
        self.move_accent_anchor(entity, -26, dy)
    }
}
