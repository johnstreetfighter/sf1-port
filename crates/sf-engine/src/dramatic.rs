//! 2v1 Dramatic Battle (`#110`, `O19`).
//!
//! Two allies and one boss step in the same frame, including their
//! projectiles. The camera keeps the outermost fighters inside a 384-pixel
//! view. The team can share one lifebar or keep two, and friendly fire is off
//! until the match asks for it. The boss starts with more health and hits harder.

use crate::canonical::{canonical_enum, canonical_struct};
use crate::config::MatchConfig;
use crate::era::{loadouts, Revision, StylePick};
use crate::front::PlayerInputs;
use crate::roster::TournamentFighter;

pub const VIEW_W: i32 = 384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LifebarMode {
    #[default]
    Shared,
    Independent,
}

pub const ALLY_SUB_HP: u16 = 960;
pub const BOSS_SUB_HP: u16 = 1536;

/// Rules stamped on a native Dramatic fight. The roster holds Ally 1 and
/// the boss; the third fighter and stage are independent of those picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DramaticRules {
    pub co_op: bool,
    pub ally2: TournamentFighter,
    pub stage: TournamentFighter,
    pub lifebar: LifebarMode,
    pub friendly_fire: bool,
    /// Native slot order: Ally 1, boss, Ally 2.
    pub styles: [Revision; 3],
    /// The index of a rivalry preset, or None for custom picks.
    pub rivalry: Option<u8>,
}

impl DramaticRules {
    pub const STYLE_REVISIONS: [Revision; 7] = [
        Revision::Sf1Plus, Revision::St, Revision::Sfz3, Revision::Sf3ThreeS,
        Revision::Usf4, Revision::Sf5Ce, Revision::Sf6,
    ];

    pub fn supports_style(style: Revision) -> bool {
        Self::STYLE_REVISIONS.contains(&style)
    }

    pub(crate) fn style_pick(revision: Revision) -> StylePick {
        StylePick::for_revision(revision, loadouts(revision)[0])
    }

    pub(crate) fn style_available(config: &MatchConfig, fighter: TournamentFighter, revision: Revision) -> bool {
        revision == Revision::Sf1Plus || (config.era_select_enabled()
            && Self::supports_style(revision)
            && Self::style_pick(revision).available_for(fighter, config.roster.what_if_styles))
    }

    pub(crate) fn styles_available(self, config: &MatchConfig) -> bool {
        (self.rivalry.is_none() || self.styles == [self.styles[0]; 3])
            && [config.roster.p1, config.roster.p2, self.ally2]
                .into_iter()
                .zip(self.styles)
                .all(|(fighter, revision)| Self::style_available(config, fighter, revision))
    }
}

/// Authoritative state attached to the translated machine. Health indices
/// use the same order as the three object-RAM fighter slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DramaticState {
    pub health: [u16; 3],
    pub boss_pad: PlayerInputs,
    pub boss_trackers: [u8; 9],
    pub boss_latches: [u8; 16],
}

impl DramaticState {
    pub fn reset_round(&mut self) {
        *self = Self { health: [ALLY_SUB_HP, BOSS_SUB_HP, ALLY_SUB_HP], ..Self::default() };
    }
}

canonical_enum!(LifebarMode, "dramatic-lifebar/v1"; Shared => 0, Independent => 1);
canonical_struct!(DramaticRules, "dramatic-rules/v1", {
    co_op, ally2, stage, lifebar, friendly_fire, styles, rivalry,
});
canonical_struct!(DramaticState, "dramatic-state/v1", {
    health, boss_pad, boss_trackers, boss_latches,
});
