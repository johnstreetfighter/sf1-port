//! The bosses' command normals in play (spec #259; design
//! `docs/design/2026-11-20-eagle-lee-command-normals-design.md`).
//!
//! A command normal's header lives in its fighter's bank slot, out of his
//! move table ([`crate::data_bank::command_normals`]). Its input is one the
//! 1987 dispatch already turns into a table move, so the move id stays and
//! only the header is swapped: Eagle's down-forward + HP is move `0x1d`
//! (shared with down-forward + MP; the strength byte `+0x34` tells them
//! apart) and starts his two-stick thrust instead of his 1987 low thrust.
//! In the air the jump attack keeps its move id (`0x20..=0x22` for a
//! punch) and loads the command normal's stream for its jump instead of
//! the apex record's: Eagle's down + HP is his dive thrust.
//!
//! They come out in SF1+ only for now. Faithful 1987 never has a bank, so
//! never reaches them; Eagle's canon SFZ3 kit and the 1987 era style are
//! left out by the style check.

use super::Bus;
use crate::data_bank::command_normals::CommandNormal;
use crate::data_bank::fighter_of_table;
use crate::era::EraFamily;
use crate::roster::TournamentFighter;

/// The 1987 dispatch's down-forward punch at medium and heavy strength.
const DOWN_FORWARD_PUNCH: u8 = 0x1d;
/// The strength byte's heavy value.
const HEAVY: u8 = 3;
/// The pad's down bit (`pad_directions`; forward and back are bits 0
/// and 1 once made relative to his facing).
const DOWN: u8 = 4;

impl Bus<'_> {
    /// The fighter whose bank table the player structure `ps` runs, when it
    /// is a bank table.
    fn bank_fighter(&self, ps: u32) -> Option<TournamentFighter> {
        self.data_bank?;
        fighter_of_table(self.rl(ps + 0x12))
    }

    /// Whether `ps` plays a style with the bosses' command normals: SF1+
    /// (an overhaul fight without a chosen style plays as SF1+).
    fn command_normal_style(&self, ps: u32) -> bool {
        matches!(self.chosen_era(ps), None | Some(EraFamily::Sf1Plus))
    }

    /// The header move `mv` should start from instead of its table's, when
    /// its input makes a command normal.
    pub(super) fn command_normal_header(&self, ps: u32, entity: u16, mv: u8) -> Option<u32> {
        let normal = match self.bank_fighter(ps)? {
            TournamentFighter::Eagle if mv == DOWN_FORWARD_PUNCH && self.eb(entity, 0x34) == HEAVY => {
                CommandNormal::EagleTwoStickThrust
            }
            _ => return None,
        };
        if !self.command_normal_style(ps) {
            return None;
        }
        self.data_bank?.command_normal_header(normal)
    }

    /// The stream a jump attack should load instead of its apex record's,
    /// when the stick held as it starts makes it a command normal: `kick`
    /// and `strength` (1 to 3) are the button, `jump` the jump (0 straight
    /// up, 1 forward, 2 back). Both jump-attack paths (the apex record and
    /// ANYTIME JUMP ATTACKS) come through here, so the toggle means the
    /// same for these as for every jump attack.
    pub(super) fn air_command_normal(&self, ps: u32, entity: u16, kick: bool, strength: u8, jump: u32) -> Option<u32> {
        let held = self.relative_directions(entity, self.pad_directions(ps));
        let down = held & DOWN != 0;
        let normal = match self.bank_fighter(ps)? {
            TournamentFighter::Eagle if !kick && strength == HEAVY && down => CommandNormal::EagleDiveThrust,
            _ => return None,
        };
        if !self.command_normal_style(ps) {
            return None;
        }
        self.data_bank?.command_normal_stream(normal, jump)
    }
}
