//! Ground-walk targets from the spec #138 pacing audits.

use super::Bus;
use crate::era::{Loadout, Revision};

impl Bus<'_> {
    /// Distance and number of double steps per 16 port ticks. Arcade
    /// distances use the most frequent sample, converted to the 60 Hz clock
    /// and rounded to a whole world pixel. SFIV uses the close/far jab cutoff
    /// (1.2 source units to 64 port px). SFV and SF6 use Ryu's standing pushbox.
    pub(super) fn ryu_forward_walk(&self, player: u32, move_id: u8) -> Option<(i16, i16)> {
        if move_id != 2 {
            return None;
        }
        let style = self.ryu_pacing_style(player)?;
        Some(match (style.revision, style.loadout) {
            (Revision::Ww | Revision::Ce | Revision::Super, _) => (48, 0),
            (Revision::Hf, _) => (69, 7),
            (Revision::St, Loadout::Standard | Loadout::Old) => (63, 5),
            (Revision::Sfz1, _) | (Revision::Sfz3, Loadout::AIsm) => (60, 4),
            (Revision::Sfz2 | Revision::Sfz2A, _) => (63, 5),
            (Revision::Sfz3, Loadout::XIsm) => (61, 4),
            (Revision::Sfz3, Loadout::VIsm) => (62, 4),
            (Revision::Sf3Ng | Revision::Sf3TwoI | Revision::Sf3ThreeS, _) => (52, 0),
            // 0.045 source units/tick × 16 × 64 px / 1.2 source units = 38.4 px.
            (Revision::Sf4 | Revision::Ssf4 | Revision::Ae2012 | Revision::Usf4, _) => (38, 0),
            (Revision::Sf5 | Revision::Sf5Ae | Revision::Sf5Ce, _) => (63, 0),
            (Revision::Sf6, _) => (45, 0),
            _ => return None,
        })
    }

    /// Advancing updates per 32 port ticks, and distance in sixteenths of a
    /// pixel per advancing update. The elapsed cadence comes from the World
    /// Turbo 2 captures and includes the arcade's doubled display updates.
    pub(super) fn st_ground_walk(&self, player: u32, move_id: u8) -> Option<(i16, i16)> {
        use crate::roster::TournamentFighter::{Ken, Sagat};

        let state = self.sf2_state(player)?;
        if state.style.revision != Revision::St {
            return None;
        }
        Some(match (state.fighter?, state.style.loadout, move_id) {
            (Sagat, _, 2) => (42, 40),
            (Sagat, _, 3) => (41, -32),
            (Ken, Loadout::Standard | Loadout::Old, 2) => (43, 51),
            (Ken, Loadout::Standard, 3) => (41, -34),
            (Ken, Loadout::Old, 3) => (42, -34),
            _ => return None,
        })
    }

    pub(super) fn measured_walk(&self, player: u32, move_id: u8) -> bool {
        self.st_ground_walk(player, move_id).is_some()
            || self.ryu_forward_walk(player, move_id).is_some()
    }

    pub(super) fn step_measured_walk(&mut self, entity: u16) -> bool {
        let player = self.el(entity, 0x38);
        let move_id = self.eb(entity, 0xa);
        let age = i16::from(self.eb(entity, 0x30));
        let step = if let Some((distance, doubles)) = self.ryu_forward_walk(player, move_id) {
            // Object RAM keeps the phase through pose-cycle restarts and rollback.
            // The offset reproduces the accepted Ryu World Turbo 2 cadence.
            const PERIOD: i16 = 16;
            let phase = age % PERIOD;
            let spread = |extra| ((phase + 1) * extra + 5) / PERIOD - (phase * extra + 5) / PERIOD;
            let base = distance / (PERIOD + doubles);
            let remainder = distance % (PERIOD + doubles);
            base * (1 + spread(doubles)) + spread(remainder)
        } else {
            return false;
        };
        self.move_walk_step(entity, player, move_id, step);
        true
    }

    pub(super) fn step_st_ground_walk(
        &mut self,
        entity: u16,
        player: u32,
        move_id: u8,
        timing: (i16, i16),
    ) {
        const PERIOD: i16 = 32;
        let (updates, sixteenths_per_update) = timing;
        let phase = i16::from(self.eb(entity, 0x31)) % PERIOD;
        let x = |tick: i16| tick * updates / PERIOD * sixteenths_per_update / 16;
        let step = x(phase + 1) - x(phase);
        self.set_eb(entity, 0x31, (phase + 1) as u8);
        self.move_walk_step(entity, player, move_id, step);
    }

    fn move_walk_step(&mut self, entity: u16, player: u32, move_id: u8, step: i16) {
        let dx = if self.eb(entity, 0x37) & 4 != 0 {
            -step
        } else {
            step
        };
        let dx = self.world_tour_horizontal_step(player, move_id, dx);
        // clamp_x forwards this step to the camera at the screen edge.
        self.set_eb(entity, 0x15, dx as u8);
        self.shift_grid_x(entity, dx as u16);
    }
}
