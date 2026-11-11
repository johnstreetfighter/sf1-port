//! Mirror palettes in a same-fighter fight (spec #246, SP16, design section 5.6).
//!
//! In the 1987 game a fighter's sprites take their palette from the frame
//! records of his move table, so Ryu against Ryu is two fighters in one
//! palette. Under the overhaul, in a fight of a fighter against himself, the
//! second side is drawn in his mirror palette (`art/palettes/<fighter>.json`,
//! [`sf_video::mirror_palette`]) so the two can be told apart. It is an
//! overhaul-only gate on the translated path (ADR-0004), never a write from
//! outside the engine, and it acts only when all of these hold:
//!
//! - the preset is not Faithful 1987 (`Bus::preset`), whatever else is
//!   switched on: the Roster tab can turn FULL CAST on under any preset, and
//!   Faithful 1987 never recolours anyone;
//! - the full cast is bound (`roster.full_cast`), so the 1987 cabinet and
//!   the attract, which bind no one, never see it;
//! - the fight is Local (`roster.mode` Versus, the 1987 two-player versus,
//!   game mode 1) or Training (`roster.mode` Training: the dummy is game mode
//!   1 and the CPU's fight is game mode 2, where the CPU's player structure
//!   stands in for P2);
//! - both sides picked the same fighter, and in Training's CPU fight the
//!   CPU's move table is his;
//! - the second side's fighter is on screen.
//!
//! Then at the end of every machine frame ([`crate::run::PowerOnMachine::step`]),
//! after the frame's tasks and the vblank have drawn, [`Bus::mirror_palette_pass`]:
//!
//! 1. writes the palette into its free sprite palette in palette RAM (the
//!    colours the file gives, pen 15 as the palette it stands in for has it);
//! 2. points every sprite of the second side's 24-entry object region that
//!    is drawn in the fighter's own palette at the slot instead.
//!
//! Only his body is recoloured (the maintainer, 2026-11-08): projectiles, bursts and
//! the hit sparks are effect objects, outside the region, and keep the ROM's
//! colours, as does anything of his own region drawn in another palette (the
//! flash of Geki's own pose).
//!
//! Only the four palette bits of a sprite's attribute word change, and
//! nothing in the 1987 code reads them: it reads the size bit (`0x400`) and
//! the flip bits (`0x100`, `0x200`), and each new animation frame writes the
//! ROM's palette again, which the next pass maps again. The new bytes are in
//! the machine's own object and palette RAM, so snapshots, rollback and
//! replays carry them. The first side's sprites and everything outside such
//! a fight are never touched.
//!
//! The free slot, 14, is a palette no sprite uses in a versus or Training
//! fight; the tests check that over every mirrored fighter.

use sf_video::mirror_palette::{self, MirrorPalette};

use crate::fight_reader::FIGHTER_SLOTS;
use crate::config::RulesetPreset;
use crate::fighter::{Bus, CPU_STRUCT, GAME_MODE, P2_STRUCT};
use crate::roster::PlayMode;

/// The sprite palettes' place in palette RAM, and the bytes of one (16 words).
const SPRITE_PALETTES: usize = 0x400;
const PALETTE_BYTES: usize = 32;
/// Object RAM: a fighter's sprite entries are 0x40 apart and he has 24.
const ENTRY: u16 = 0x40;
const REGION_ENTRIES: u16 = 24;

impl Bus<'_> {
    /// The mirror palette the second side wears in this fight and that
    /// side's player structure, if the gate applies (see the module docs).
    pub fn mirror_match(&self) -> Option<(&'static MirrorPalette, u32)> {
        let r = &self.roster;
        if let Some(rules) = self.dramatic_rules() {
            if rules.ally2 == r.p1 || rules.ally2 == r.p2 {
                return Some((mirror_palette::for_fighter(rules.ally2.name())?, P2_STRUCT));
            }
            if r.p1 == r.p2 {
                return Some((mirror_palette::for_fighter(r.p2.name())?, CPU_STRUCT));
            }
            return None;
        }
        if self.preset == RulesetPreset::Faithful1987 || !r.full_cast || r.p1 != r.p2 {
            return None;
        }
        let side = match (r.mode, self.rb(GAME_MODE)) {
            (PlayMode::Versus | PlayMode::Training, 1) => P2_STRUCT,
            (PlayMode::Training, 2) if self.rl(CPU_STRUCT + 0x12) == r.p2.move_table() => CPU_STRUCT,
            _ => return None,
        };
        Some((mirror_palette::for_fighter(r.p2.name())?, side))
    }

    /// One machine frame of the gate: see the module docs. Nothing outside a
    /// same-fighter fight.
    pub fn mirror_palette_pass(&mut self) {
        let Some((mirror, side)) = self.mirror_match() else { return };
        let region = self.rw(side + 0x10);
        if !FIGHTER_SLOTS.contains(&region) || self.eb(region, 8) == 0 {
            return;
        }
        self.write_mirror_slot(mirror);
        for k in 0..REGION_ENTRIES {
            let s = region.wrapping_add(k * ENTRY);
            let attr = self.ew(s, 2);
            if attr & 0xf == u16::from(mirror.from) {
                self.set_ew(s, 2, attr & !0xf | u16::from(mirror.slot));
            }
        }
    }

    /// The palette's colours in its slot: pens 0..14 from the file, pen 15
    /// (transparent) the replaced palette's.
    fn write_mirror_slot(&mut self, mirror: &MirrorPalette) {
        let slot = SPRITE_PALETTES + mirror.slot as usize * PALETTE_BYTES;
        let pen15 = SPRITE_PALETTES + mirror.from as usize * PALETTE_BYTES + 2 * mirror_palette::PENS;
        let transparent = [self.palette[pen15], self.palette[pen15 + 1]];
        for (pen, word) in mirror.pens.iter().enumerate() {
            self.palette[slot + 2 * pen..slot + 2 * pen + 2].copy_from_slice(&word.to_be_bytes());
        }
        let last = slot + 2 * mirror_palette::PENS;
        self.palette[last..last + 2].copy_from_slice(&transparent);
    }
}

#[cfg(test)]
mod tests;
