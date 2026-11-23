//! The cast's own tools for the stronger play (SC15, `#207`; spec `#192`,
//! ADR-0026): what each fighter's specials are for in an era style, where
//! the old play's reading of them by name falls short, and the moves a
//! fighter has that are neither a projectile, an anti-air nor a move that
//! carries him in: command throws, teleports and charge moves.
//!
//! The old play reads a what-if kit's specials by name
//! ([`super::Kit::from_moves`]): only `SHORYUKEN`, `UPPERCUT` and `GEKIROU`
//! are anti-airs and only `HADOKEN`, `TIGER SHOT` and `KIAI SHOT`
//! projectiles, so Joe's Shockwave is taken for a move that carries him in
//! and Retsu's command throw is thrown from the middle distance. That stays
//! as it is with the switch off. The stronger play reads them from the
//! tables here, by fighter and by the move's name as the move list shows it,
//! for the SF2 era style; a fighter or style with no table keeps the old
//! reading.
//!
//! The kit is read wherever an era style's context is built (a few name
//! comparisons a frame); only the switch-on play uses it
//! ([`super::FairCpu::press`]).

use super::{Kit, Role};
use crate::era::EraFamily;
use crate::moves::{MoveEntry, MoveKind};
use crate::roster::TournamentFighter;

/// A fighter's move that is none of the old play's three roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    /// A throw entered as a special: it takes a guarding opponent and
    /// cannot be teched.
    CommandThrow,
    /// A move that puts the fighter somewhere else at once.
    Teleport,
    /// A move entered by holding a direction first.
    Charge,
}

/// The tools a fighter has in this fight, as the move list gives them.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tools {
    pub command_throw: Option<MoveEntry>,
    pub teleport: Option<MoveEntry>,
    pub charge: Option<MoveEntry>,
}

impl Tools {
    pub fn get(&self, tool: Tool) -> Option<MoveEntry> {
        match tool {
            Tool::CommandThrow => self.command_throw,
            Tool::Teleport => self.teleport,
            Tool::Charge => self.charge,
        }
    }

    fn slot(&mut self, tool: Tool) -> &mut Option<MoveEntry> {
        match tool {
            Tool::CommandThrow => &mut self.command_throw,
            Tool::Teleport => &mut self.teleport,
            Tool::Charge => &mut self.charge,
        }
    }
}

/// How near, centre to centre, a fighter's command throw catches in the
/// SF2 era style: the catch record the data bank writes for it (Retsu's
/// Izuna 43, Geki's Execution Drop 48; Birdie's chain is his Alpha chain's,
/// 54). `None` for a fighter without one.
pub fn catch_range(fighter: TournamentFighter) -> Option<i16> {
    match fighter {
        TournamentFighter::Retsu => Some(43),
        TournamentFighter::Geki => Some(48),
        TournamentFighter::Birdie => Some(54),
        _ => None,
    }
}

/// What a move is for in the stronger play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Use {
    Role(Role),
    Tool(Tool),
    /// Not chosen as a special: a mashed or counter move, an overhead kept
    /// for the era kit, or one that needs a set-up first.
    Skip,
}

use Use::{Role as R, Skip, Tool as T};

/// The SF2 era style's what-if kits (`era_fight::what_if`), by move name.
/// The first of a role is its first choice. Gen and Eagle are not here:
/// they have no game plans in the style yet, and keep the old reading of
/// their specials (SC15, `#207`).
const SF2: &[(TournamentFighter, &[(&str, Use)])] = &[
    (
        TournamentFighter::Retsu,
        &[
            ("KIAI SHOT", R(Role::Projectile)),
            ("SHORYUKEN", R(Role::AntiAir)),
            ("TOBI GERI", R(Role::Advance)),
            ("SENPUU KYAKU", R(Role::Advance)),
            ("IZUNA SHOULDER THROW", T(Tool::CommandThrow)),
            ("HUNDRED MONK SLAPS", Skip),
        ],
    ),
    (
        TournamentFighter::Geki,
        &[
            ("SHURIKEN", R(Role::Projectile)),
            ("GEKI-ZAN", R(Role::AntiAir)),
            ("UTSUSEMI", T(Tool::Teleport)),
            ("EXECUTION DROP", T(Tool::CommandThrow)),
            ("UTSUSEMI FEINT", Skip),
        ],
    ),
    (
        TournamentFighter::Joe,
        &[
            ("SHOCKWAVE", R(Role::Projectile)),
            ("CORKSCREW CROSS", R(Role::AntiAir)),
            ("ROLLING SOBAT", R(Role::Advance)),
            ("DUCKING RUSH", R(Role::Advance)),
            ("KNEE BASH", Skip),
            ("STEP HIGH KICK", Skip),
            ("FIST COMBINATION", Skip),
        ],
    ),
    (
        TournamentFighter::Mike,
        &[
            ("WEAVING UPPER", R(Role::AntiAir)),
            ("MURDER STRAIGHT", R(Role::Advance)),
            ("ROLLING BLOW", R(Role::Advance)),
            ("DUCKING SWAY", Skip),
        ],
    ),
    (
        TournamentFighter::Lee,
        &[
            ("UPWARD LANCE", R(Role::AntiAir)),
            ("HURRICANE KICK", R(Role::Advance)),
            ("OPEN CANVAS PUNCH", R(Role::Advance)),
            ("FIREWATER", T(Tool::Charge)),
            ("OPEN CANVAS KICK", Skip),
            ("STAB KICK", Skip),
        ],
    ),
    (
        TournamentFighter::Birdie,
        &[
            ("BULL HEAD", T(Tool::Charge)),
            ("MURDERER CHAIN", T(Tool::CommandThrow)),
            ("HANGING CHAIN", R(Role::Advance)),
            ("BULL HORN", Skip),
            ("BAD HAMMER", Skip),
        ],
    ),
    (
        TournamentFighter::Adon,
        &[
            ("RISING JAGUAR", R(Role::AntiAir)),
            ("JAGUAR KICK", R(Role::Advance)),
            ("JAGUAR TOOTH", Skip),
        ],
    ),
];

/// The table for `fighter` in `family`, when the stronger play has one.
fn table(fighter: TournamentFighter, family: EraFamily) -> Option<&'static [(&'static str, Use)]> {
    if family != EraFamily::Sf2 {
        return None;
    }
    SF2.iter().find(|(who, _)| *who == fighter).map(|(_, uses)| *uses)
}

/// The stronger play's kit and tools for `fighter` in `family`, from the
/// fight's live move list, when a table here covers him; `None` keeps the
/// old reading. A special the table does not name is left out, and a named
/// one the list has not got is skipped: the fight tests then see the plans
/// that need it go untaken (`tests/stronger_cpu_sf2.rs`).
pub(super) fn stronger_kit(fighter: TournamentFighter, family: EraFamily, entries: &[MoveEntry]) -> Option<(Kit, Tools)> {
    let uses = table(fighter, family)?;
    let mut kit = Kit { fighter: Some(fighter), ..Kit::default() };
    let mut tools = Tools::default();
    for (name, use_) in uses {
        // The move as this fight's list has it: a special, not an enhanced
        // one (its pair of buttons spends meter).
        let Some(entry) = entries.iter().copied().find(|entry| {
            entry.name == *name
                && entry.kind == MoveKind::Special
                && !matches!(entry.input.button, crate::moves::ButtonClass::PairPunch | crate::moves::ButtonClass::PairKick)
        }) else {
            continue;
        };
        match *use_ {
            Use::Role(role) => {
                let (first, second) = kit.slots(role);
                if first.is_none() {
                    *first = Some(entry);
                } else if second.is_none() {
                    *second = Some(entry);
                }
            }
            Use::Tool(tool) => {
                let slot = tools.slot(tool);
                if slot.is_none() {
                    *slot = Some(entry);
                }
            }
            Use::Skip => {}
        }
    }
    Some((kit, tools))
}

/// The fighters the tables cover in `family`.
#[cfg(test)]
pub(super) fn covered(family: EraFamily) -> Vec<TournamentFighter> {
    if family != EraFamily::Sf2 {
        return Vec::new();
    }
    SF2.iter().map(|(who, _)| *who).collect()
}

/// Every move name a table gives `fighter` in `family`, with the tool it is
/// (`None` for a role or a move left out).
#[cfg(test)]
pub(super) fn named(fighter: TournamentFighter, family: EraFamily) -> Vec<(&'static str, Option<Tool>)> {
    table(fighter, family)
        .unwrap_or(&[])
        .iter()
        .map(|(name, use_)| (*name, if let Use::Tool(tool) = use_ { Some(*tool) } else { None }))
        .collect()
}
