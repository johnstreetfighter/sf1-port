//! The bosses' command normals in play (spec #259; design
//! `docs/design/2026-11-20-eagle-lee-command-normals-design.md`).
//!
//! A command normal's header lives in its fighter's bank slot, out of his
//! move table ([`crate::data_bank::command_normals`]). Its input is one the
//! 1987 dispatch already turns into a table move, so the move id stays and
//! only the header is swapped: Eagle's down-forward + HP is move `0x1d`
//! (shared with down-forward + MP; the strength byte `+0x34` tells them
//! apart) and starts his two-stick thrust instead of his 1987 low thrust.
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
}
