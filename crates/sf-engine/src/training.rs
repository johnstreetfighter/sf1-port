//! Training's rules in the engine (spec #238, design §5.1, ADR-0013).
//!
//! In Training, under every preset, Faithful 1987 included, the translated
//! path keeps a few rules the 1987 game has no use for, each a gate keyed on
//! the play mode the engine already reads (`roster.mode`), never set in
//! Arcade, Local, the attract or the faithful cabinet:
//!
//! - the health floor, in [`crate::fighter::TRAINING_HEALTH_FLOOR`]'s drain
//!   (`0x3c034`, ticket #239);
//! - the stopped clock, in [`crate::tasks::round_timer`] (ticket #240);
//! - the refill here (ticket #243);
//! - the hit record here, which the fight reader reads (ticket #244).
//!
//! What the rules keep from frame to frame is the [`TrainingState`]: it
//! lives beside the work RAM in the machine's program state and in its
//! snapshots ([`crate::run::PowerOnSnapshot`], [`crate::fight::FightSnapshot`]),
//! so RESET POSITIONS, replays and rollback reproduce it, and none of it is
//! kept in 1987 work RAM.
//!
//! **Refill.** A fighter's health comes back to full once all of these
//! hold: [`TRAINING_REFILL_FRAMES`] fight frames have passed since they
//! were last hit or blocked an attack; they are out of their hit reaction
//! (moves `0x30..`, `0x4451a`) and out of their guard, which is the 1987
//! block reaction (moves 4 and 8, or entity +9 bit `0x20`, as every block
//! test in the hit functions reads it); and they are below full health.
//! The wait starts over at every landed hit (the reaction, `0x4451a`, runs
//! for every one, chip included) and every block, damaging or not (each
//! hit function's blocked branch, the projectiles' too). Full health is the
//! round start's (`round_hud`: +6 = 4, +7 = 0xb), and the bar's twelve
//! cells snap back to the tile the round start draws them with (`0x3bf22`).
//!
//! **Hit record.** The health floor holds a fighter at one point, so a hit
//! there takes nothing and a hit that reaches it is cut short, and the
//! fight reader, which sees a landed hit by the health it took, would lose
//! or under-count them. So in Training every drain (`0x3c034`, which every
//! landed hit runs just before its reaction, chip included) adds the hit's
//! full damage, before the chip rule's cut and the floor's, to its
//! defender's running total ([`TrainingState::damage_taken`]). The reader
//! reads the total's rise from frame to frame in Training in place of the
//! health's fall, so its hits and their damage are those of a fight with
//! unlimited health.
//!
//! The sides are the fight reader's ([`crate::fight_reader`]): the CPU's
//! player structure stands in for P2 in one-player play (game mode 2) and
//! for P1 in mode 4.

use sf_video::callouts::Side;

use crate::fight_reader::{player_struct, FIGHTER_SLOTS};
use crate::fighter::{Bus, GAME_MODE};
use crate::roster::PlayMode;

/// Fight frames, since a fighter was last hit or blocked, before Training
/// refills their health: about a second.
pub const TRAINING_REFILL_FRAMES: u16 = 60;

/// Full health as the round start sets it (`round_hud`, 0x3e78c..): the
/// +6 remainder and the +7 cells.
const FULL_REMAINDER: u8 = 4;
const FULL_CELLS: u8 = 0xb;
/// The health bar's cells, from the player's +0xa text offset, and the
/// tile the round start draws each with (`health_bars`, 0x3bf22).
const BAR_CELLS: u32 = 12;
const FULL_CELL_TILE: u16 = 0x1120;

/// What Training's rules keep between frames, by side (`[P1, P2]`, the
/// fight reader's sides); part of the machine's snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrainingState {
    /// Fight frames since the side was last hit or blocked an attack,
    /// held at the top once it gets there (the refill's wait).
    pub since_contact: [u16; 2],
    /// The full damage of every hit the side has taken, before any cut, as
    /// a running total that wraps (the hit record; the fight reader reads
    /// its rise from frame to frame).
    pub damage_taken: [u16; 2],
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(TrainingState, "TrainingState.v1", {
    since_contact, damage_taken,
});


/// The side whose fighter has player structure `ps` in game mode `mode`.
fn side_of(ps: u32, mode: u8) -> Side {
    if ps == player_struct(Side::P1, mode) {
        Side::P1
    } else {
        Side::P2
    }
}

impl Bus<'_> {
    /// In Training, the fighter entity `defender` was just hit or blocked an
    /// attack: its side's refill wait starts over. Called from the
    /// reaction (every landed hit) and every blocked branch.
    pub(crate) fn training_contact(&mut self, defender: u16) {
        if self.roster.mode != PlayMode::Training {
            return;
        }
        let side = side_of(self.el(defender, 0x38), self.rb(GAME_MODE));
        self.training.since_contact[side.index()] = 0;
    }

    /// In Training, a hit for `damage` drains the health of the player
    /// structure `ps`: its side's record takes the full damage. Called from
    /// the drain (`0x3c034`) before it cuts anything.
    pub(crate) fn training_hit(&mut self, ps: u32, damage: u8) {
        if self.roster.mode != PlayMode::Training {
            return;
        }
        let side = side_of(ps, self.rb(GAME_MODE));
        let taken = &mut self.training.damage_taken[side.index()];
        *taken = taken.wrapping_add(u16::from(damage));
    }

    /// One fight frame of Training's refill (see the module docs), run from
    /// the fight loop's frame (`0x3eafa`) after both fighters, the effects
    /// and the camera, so a hit or block this frame has already restarted
    /// its side's wait. Nothing outside Training.
    pub fn training_refill(&mut self) {
        if self.roster.mode != PlayMode::Training {
            return;
        }
        let mode = self.rb(GAME_MODE);
        for side in Side::BOTH {
            // A contact this frame left 0: the refill comes on the 60th
            // frame after it at the soonest.
            let wait = &mut self.training.since_contact[side.index()];
            let waited = *wait >= TRAINING_REFILL_FRAMES;
            *wait = wait.saturating_add(1).min(TRAINING_REFILL_FRAMES);
            let ps = player_struct(side, mode);
            let e = self.rw(ps + 0x10);
            if !waited || !FIGHTER_SLOTS.contains(&e) {
                continue;
            }
            let (move_id, state) = (self.eb(e, 0xa), self.eb(e, 9));
            let reacting = move_id >= 0x30;
            let guarding = matches!(move_id, 4 | 8) || state & 0x20 != 0;
            let full = 4 * u16::from(FULL_CELLS) + u16::from(FULL_REMAINDER);
            let health = 4 * u16::from(self.rb(ps + 7)) + u16::from(self.rb(ps + 6));
            if reacting || guarding || health >= full {
                continue;
            }
            self.wb(ps + 6, FULL_REMAINDER);
            self.wb(ps + 7, FULL_CELLS);
            let bar = 0x80_0000 + u32::from(self.rw(ps + 0xa));
            for i in 0..BAR_CELLS {
                self.ww(bar + 2 * i, FULL_CELL_TILE);
            }
        }
    }
}
