//! Command normals for the bosses (spec #259; design
//! `docs/design/2026-11-20-eagle-lee-command-normals-design.md`): normals
//! that need a stick direction held with their button. Each one's header is
//! authored once in its fighter's slot and kept out of the move table: a
//! pointer to it sits in the slot's command-normal area, right after
//! [`ERA_HEADER_END`], and the fighter code starts it by redirecting the
//! table move its input would otherwise start.

use super::ERA_HEADER_END;
use crate::roster::TournamentFighter;

/// The command-normal pointers a fighter that has them keeps.
const POINTERS: usize = 2;
/// The bytes `new_slot` keeps for them.
pub(super) const POINTER_BYTES: usize = POINTERS * 4;

/// One of the bosses' command normals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandNormal {
    /// Eagle's down-forward + HP on the ground: a two-stick low thrust.
    EagleTwoStickThrust,
}

impl CommandNormal {
    /// The fighter whose slot holds it.
    pub const fn fighter(self) -> TournamentFighter {
        match self {
            CommandNormal::EagleTwoStickThrust => TournamentFighter::Eagle,
        }
    }

    const fn index(self) -> u32 {
        match self {
            CommandNormal::EagleTwoStickThrust => 0,
        }
    }

    /// Where its header's pointer sits, from the fighter's table.
    pub(super) const fn offset(self) -> u32 {
        ERA_HEADER_END + self.index() * 4
    }
}

/// Whether `fighter`'s slot keeps a command-normal area.
pub(super) const fn has_area(fighter: TournamentFighter) -> bool {
    matches!(fighter, TournamentFighter::Eagle | TournamentFighter::Lee)
}
