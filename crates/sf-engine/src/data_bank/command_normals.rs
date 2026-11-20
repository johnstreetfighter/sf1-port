//! Command normals for the bosses (spec #259; design
//! `docs/design/2026-11-20-eagle-lee-command-normals-design.md`): normals
//! that need a stick direction held with their button. Each one is authored
//! once in its fighter's slot and kept out of the move table: pointers to it
//! sit in the slot's command-normal area, right after [`ERA_HEADER_END`],
//! and the fighter code starts it by redirecting the table move (on the
//! ground) or the apex record's stream (in the air) its input would
//! otherwise start.

use super::ERA_HEADER_END;
use crate::roster::TournamentFighter;

/// The command-normal pointers a fighter that has them keeps: a ground
/// move's header takes one, an air move's streams one for each jump
/// (straight up, forward, back).
const POINTERS: usize = 6;
/// The bytes `new_slot` keeps for them.
pub(super) const POINTER_BYTES: usize = POINTERS * 4;

/// One of the bosses' command normals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandNormal {
    /// Eagle's down-forward + HP on the ground: a two-stick low thrust.
    EagleTwoStickThrust,
    /// Eagle's down + HP in a jump: a two-stick thrust driven down.
    EagleDiveThrust,
    /// Lee's forward + HK in a straight-up or back jump: a knee-up kick
    /// (no stream for the forward jump, whose HK stays his flying kick).
    LeeKneeUpKick,
    /// Lee's down + MK in a jump: a dive kick that drops steeply toward the
    /// opponent.
    LeeDiveKick,
}

impl CommandNormal {
    /// The fighter whose slot holds it.
    pub const fn fighter(self) -> TournamentFighter {
        match self {
            CommandNormal::EagleTwoStickThrust | CommandNormal::EagleDiveThrust => TournamentFighter::Eagle,
            CommandNormal::LeeKneeUpKick | CommandNormal::LeeDiveKick => TournamentFighter::Lee,
        }
    }

    /// Whether it is started in a jump (its pointers name streams, one a
    /// jump) rather than on the ground (one header).
    pub const fn airborne(self) -> bool {
        matches!(self, CommandNormal::EagleDiveThrust | CommandNormal::LeeKneeUpKick | CommandNormal::LeeDiveKick)
    }

    /// Its first pointer's index in the fighter's area.
    const fn first(self) -> u32 {
        match self {
            CommandNormal::EagleTwoStickThrust => 0,
            CommandNormal::EagleDiveThrust => 1,
            CommandNormal::LeeKneeUpKick => 0,
            CommandNormal::LeeDiveKick => 3,
        }
    }

    /// Where its header's pointer sits, from the fighter's table (a ground
    /// move's).
    pub(super) const fn offset(self) -> u32 {
        ERA_HEADER_END + self.first() * 4
    }

    /// Where the pointer to its stream for `jump` (0 straight up, 1
    /// forward, 2 back) sits, from the fighter's table (an air move's).
    pub(super) const fn jump_offset(self, jump: u32) -> u32 {
        ERA_HEADER_END + (self.first() + jump) * 4
    }
}

/// Whether `fighter`'s slot keeps a command-normal area.
pub(super) const fn has_area(fighter: TournamentFighter) -> bool {
    matches!(fighter, TournamentFighter::Eagle | TournamentFighter::Lee)
}
