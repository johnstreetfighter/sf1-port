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
//! for the SF2 era style (SC15), the SFZ era style (SC16, `#208`) and the
//! SF3 era style (SC17, `#209`), SF4 era style (SC18, `#210`) and
//! SF5 era style (SC19, `#211`); a
//! fighter or style with no table keeps the old reading.
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

/// How near, centre to centre, a fighter's command throw catches: the catch
/// record the data bank writes for it (Retsu's Izuna 43, Geki's Execution
/// Drop 48; Birdie's chain is his Alpha chain's, 54). Read for the SF2 era
/// style (SC15); the SFZ and SF3 styles' what-if kits have the same moves, and
/// Birdie's chain there is the Alpha chain itself. `None` for a fighter
/// without one.
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

/// The SFZ era style's tables (SC16, `#208`). Retsu, Geki, Joe, Mike and
/// Lee play the same what-if kits as in the SF2 style, read the same way.
/// Birdie and Gen, whose Alpha kits the old play reads by move slot, keep
/// that reading of their roles and add their tools: Birdie's chain and
/// Bull Head, Gen's Jasen (his Crane stance's; in the Mantis it is not in
/// his list, and the plans that need it are not taken). Ryu, Ken, Sagat,
/// Eagle and Adon have no tool and keep the old reading.
const SFZ: &[(TournamentFighter, &[(&str, Use)])] = &[
    (TournamentFighter::Retsu, SF2[0].1),
    (TournamentFighter::Geki, SF2[1].1),
    (TournamentFighter::Joe, SF2[2].1),
    (TournamentFighter::Mike, SF2[3].1),
    (TournamentFighter::Lee, SF2[4].1),
    (
        TournamentFighter::Birdie,
        &[
            ("BULL HEAD", R(Role::Advance)),
            ("BULL HORN", R(Role::Advance)),
            ("BULL HEAD", T(Tool::Charge)),
            ("MURDERER CHAIN", T(Tool::CommandThrow)),
            ("BANDIT CHAIN", Skip),
        ],
    ),
    (
        TournamentFighter::Gen,
        &[
            ("GEKIROU", R(Role::AntiAir)),
            ("HYAKU RENKOU", R(Role::Advance)),
            ("JASEN", R(Role::Advance)),
            ("OUGA", R(Role::Advance)),
            ("JASEN", T(Tool::Charge)),
        ],
    ),
];

/// The SF3 era style's what-if kits (SC17, `#209`). The five fighters
/// without later canon kits, Birdie and Adon share the SF2 style's move
/// names. Gen keeps the Mantis/Crane tools, and Eagle and Sagat read their
/// own what-if kits rather than treating every unfamiliar name as an
/// advancing attack. Eagle's counter needs an incoming attack; a neutral
/// special must not choose it blindly.
const SF3: &[(TournamentFighter, &[(&str, Use)])] = &[
    (TournamentFighter::Retsu, SF2[0].1),
    (TournamentFighter::Geki, SF2[1].1),
    (TournamentFighter::Joe, SF2[2].1),
    (TournamentFighter::Mike, SF2[3].1),
    (TournamentFighter::Lee, SF2[4].1),
    (TournamentFighter::Birdie, SF2[5].1),
    (TournamentFighter::Adon, SF2[6].1),
    (TournamentFighter::Gen, SFZ[6].1),
    (
        TournamentFighter::Eagle,
        &[
            ("CLOBBERING", R(Role::Advance)),
            ("MANCHESTER BLACK", R(Role::Advance)),
            ("CANTERBURY BLUE", R(Role::Advance)),
            ("OXFORD RED", R(Role::Advance)),
            ("LIVERPOOL WHITE", Skip),
            ("ST ANDREWS GREEN", Skip),
        ],
    ),
    (
        TournamentFighter::Sagat,
        &[
            ("TIGER SHOT", R(Role::Projectile)),
            ("GROUND TIGER SHOT", R(Role::Projectile)),
            ("TIGER UPPERCUT", R(Role::AntiAir)),
            ("TIGER KNEE CRUSH", R(Role::Advance)),
        ],
    ),
];

/// The SF4 style's kits (SC18, `#210`). The what-if fighters keep the
/// same tools as in SF3; the native Gen kit spells its names differently,
/// and Sagat's low shot is `LOW TIGER SHOT`. His Angry Scar and Eagle's
/// counter need their own setup, so neither is a blind advancing attack.
const SF4: &[(TournamentFighter, &[(&str, Use)])] = &[
    (TournamentFighter::Retsu, SF2[0].1),
    (TournamentFighter::Geki, SF2[1].1),
    (TournamentFighter::Joe, SF2[2].1),
    (TournamentFighter::Mike, SF2[3].1),
    (
        TournamentFighter::Lee,
        &[
            ("UPWARD LANCE", R(Role::AntiAir)),
            ("OPEN CANVAS PUNCH", R(Role::Advance)),
            ("OPEN CANVAS KICK", R(Role::Advance)),
            ("FIREWATER", T(Tool::Charge)),
            ("HURRICANE KICK", Skip),
            ("STAB KICK", Skip),
        ],
    ),
    (TournamentFighter::Birdie, SF2[5].1),
    (TournamentFighter::Adon, SF2[6].1),
    (TournamentFighter::Eagle, SF3[8].1),
    (
        TournamentFighter::Gen,
        &[
            ("GEKIRO", R(Role::AntiAir)),
            ("HYAKURENKO", R(Role::Advance)),
            ("JASEN", R(Role::Advance)),
            ("OGA", R(Role::Advance)),
            ("JASEN", T(Tool::Charge)),
        ],
    ),
    (
        TournamentFighter::Sagat,
        &[
            ("TIGER SHOT", R(Role::Projectile)),
            ("LOW TIGER SHOT", R(Role::Projectile)),
            ("TIGER UPPERCUT", R(Role::AntiAir)),
            ("TIGER KNEE CRUSH", R(Role::Advance)),
            ("ANGRY SCAR", Skip),
        ],
    ),
];

/// SF5's native Birdie has quarter-circle Bull Head and Bull Horn, rather
/// than the what-if charge/chain kit. Sagat keeps his live player-table
/// names. The other fighters' what-if tools use their own kit names.
const SF5_SHOTO: &[(&str, Use)] = &[
    ("HADOKEN", R(Role::Projectile)),
    ("SHORYUKEN", R(Role::AntiAir)),
    ("TATSUMAKI", R(Role::Advance)),
];

const SF5: &[(TournamentFighter, &[(&str, Use)])] = &[
    (TournamentFighter::Ryu, SF5_SHOTO),
    (TournamentFighter::Ken, SF5_SHOTO),
    (TournamentFighter::Retsu, SF2[0].1),
    (TournamentFighter::Geki, SF2[1].1),
    (TournamentFighter::Joe, SF2[2].1),
    (
        TournamentFighter::Mike,
        &[
            ("WEAVING UPPER", R(Role::AntiAir)),
            ("MURDER STRAIGHT", R(Role::Advance)),
            ("ROLLING BLOW", R(Role::Advance)),
            ("DUCKING SWAY", Skip),
        ],
    ),
    (TournamentFighter::Lee, SF4[4].1),
    (TournamentFighter::Gen, SFZ[6].1),
    (
        TournamentFighter::Birdie,
        &[
            ("BULL HEAD", R(Role::Advance)),
            ("BULL HORN", R(Role::Advance)),
        ],
    ),
    (TournamentFighter::Eagle, SF3[8].1),
    (TournamentFighter::Adon, SF2[6].1),
    (
        TournamentFighter::Sagat,
        &[
            ("HIGH TIGER SHOT", R(Role::Projectile)),
            ("LOW TIGER SHOT", R(Role::Projectile)),
            ("TIGER UPPERCUT", R(Role::AntiAir)),
            ("TIGER KNEE", R(Role::Advance)),
        ],
    ),
];

/// The tables of `family`, if the stronger play has any.
fn tables(family: EraFamily) -> &'static [(TournamentFighter, &'static [(&'static str, Use)])] {
    match family {
        EraFamily::Sf2 => SF2,
        EraFamily::SfZ => SFZ,
        EraFamily::Sf3 => SF3,
        EraFamily::Sf4 => SF4,
        EraFamily::Sf5 => SF5,
        _ => &[],
    }
}

/// The table for `fighter` in `family`, when the stronger play has one.
fn table(fighter: TournamentFighter, family: EraFamily) -> Option<&'static [(&'static str, Use)]> {
    tables(family).iter().find(|(who, _)| *who == fighter).map(|(_, uses)| *uses)
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
    tables(family).iter().map(|(who, _)| *who).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MatchConfig;
    use crate::era::{GaugeMode, Loadout, Revision, StylePick};
    use crate::era_fight::EraFighterState;
    use crate::moves::live_move_list;

    #[test]
    fn sf3_tools_are_read_from_the_fighters_real_what_if_moves() {
        for fighter in covered(EraFamily::Sf3) {
            let mut state = EraFighterState::default();
            state.fighter = Some(fighter);
            state.style = StylePick::for_revision(Revision::Sf3ThreeS, Loadout::Art1);
            state.max_sub_hp = 960;
            state.set_what_if(true);
            state.set_gauge_mode(GaugeMode::EraDefault);
            let moves = live_move_list(fighter, MatchConfig::definitive().preset, true, &state);
            let (kit, tools) = stronger_kit(fighter, EraFamily::Sf3, &moves).expect("SF3 has a table for every what-if fighter");
            assert_eq!(tools.command_throw.is_some(), matches!(fighter, TournamentFighter::Retsu | TournamentFighter::Geki | TournamentFighter::Birdie), "{fighter:?} command throw");
            assert_eq!(tools.teleport.is_some(), fighter == TournamentFighter::Geki, "{fighter:?} teleport");
            assert_eq!(tools.charge.is_some(), matches!(fighter, TournamentFighter::Lee | TournamentFighter::Birdie), "{fighter:?} charge in its starting stance");
            for role in [Role::Projectile, Role::AntiAir, Role::Advance] {
                for entry in kit.by_role(role).into_iter().flatten() {
                    assert!(moves.contains(&entry), "{fighter:?} {entry:?}");
                    assert!(!matches!(entry.input.button, crate::moves::ButtonClass::PairPunch | crate::moves::ButtonClass::PairKick));
                    assert_ne!(entry.name, "ST ANDREWS GREEN", "a counter is not a neutral advance");
                }
            }
            if fighter == TournamentFighter::Sagat {
                assert_eq!(kit.projectile.unwrap().name, "TIGER SHOT");
                assert_eq!(kit.anti_air.unwrap().name, "TIGER UPPERCUT");
            }
        }
        assert!(stronger_kit(TournamentFighter::Ryu, EraFamily::Sf3, &[]).is_none(), "Ryu keeps the canon reading");
        assert!(stronger_kit(TournamentFighter::Ken, EraFamily::Sf3, &[]).is_none(), "Ken keeps the canon reading");
    }

    #[test]
    fn sf4_tools_and_roles_use_the_native_or_what_if_kit_names() {
        for fighter in covered(EraFamily::Sf4) {
            let mut state = EraFighterState::default();
            state.fighter = Some(fighter);
            state.style = StylePick::for_revision(Revision::Usf4, Loadout::Ultra1);
            state.max_sub_hp = 960;
            state.set_what_if(!crate::era::families(fighter).contains(&EraFamily::Sf4));
            state.set_gauge_mode(GaugeMode::EraDefault);
            let moves = live_move_list(fighter, MatchConfig::definitive().preset, true, &state);
            let (kit, tools) = stronger_kit(fighter, EraFamily::Sf4, &moves).expect("SF4 has a kit table");
            assert_eq!(tools.command_throw.is_some(), matches!(fighter, TournamentFighter::Retsu | TournamentFighter::Geki | TournamentFighter::Birdie), "{fighter:?} command throw");
            assert_eq!(tools.teleport.is_some(), fighter == TournamentFighter::Geki, "{fighter:?} teleport");
            assert_eq!(tools.charge.is_some(), matches!(fighter, TournamentFighter::Lee | TournamentFighter::Birdie), "{fighter:?} charge in its starting stance");
            for role in [Role::Projectile, Role::AntiAir, Role::Advance] {
                for entry in kit.by_role(role).into_iter().flatten() {
                    assert!(moves.contains(&entry), "{fighter:?} {entry:?}");
                    assert!(!matches!(entry.name, "ANGRY SCAR" | "ST ANDREWS GREEN"), "no setup move as a blind advance");
                }
            }
            if fighter == TournamentFighter::Sagat {
                assert_eq!(kit.projectile_alt.unwrap().name, "LOW TIGER SHOT");
            }
            if fighter == TournamentFighter::Gen {
                assert_eq!(kit.anti_air.unwrap().name, "GEKIRO");
                assert_eq!(kit.advance.unwrap().name, "HYAKURENKO");
            }
        }
    }

    #[test]
    fn sf5_roles_and_tools_match_every_selectable_live_kit() {
        use TournamentFighter::*;
        let mut pairs = 0;
        for fighter in TournamentFighter::ALL {
            for revision in crate::era::listed_revisions(fighter, EraFamily::Sf5, true) {
                for loadout in crate::era::loadouts(revision) {
                    pairs += 1;
                    let mut state = EraFighterState::default();
                    state.fighter = Some(fighter);
                    state.style = StylePick::for_revision(revision, loadout);
                    state.max_sub_hp = 960;
                    state.set_what_if(!crate::era::families(fighter).contains(&EraFamily::Sf5));
                    state.set_gauge_mode(GaugeMode::EraDefault);
                    let moves = live_move_list(fighter, MatchConfig::definitive().preset, true, &state);
                    let (kit, tools) = stronger_kit(fighter, EraFamily::Sf5, &moves).expect("every SF5 fighter has a table");
                    let label = format!("{fighter:?} {revision:?}/{loadout:?}");
                    assert_eq!(tools.command_throw.is_some(), matches!(fighter, Retsu | Geki), "{label}: ordinary command throw");
                    assert_eq!(tools.teleport.is_some(), fighter == Geki, "{label}: teleport");
                    assert_eq!(tools.charge.is_some(), fighter == Lee, "{label}: starting-stance charge");
                    for role in [Role::Projectile, Role::AntiAir, Role::Advance] {
                        for entry in kit.by_role(role).into_iter().flatten() {
                            assert!(moves.contains(&entry), "{label}: {entry:?}");
                            assert!(!matches!(entry.input.button, crate::moves::ButtonClass::PairPunch | crate::moves::ButtonClass::PairKick), "{label}: ordinary role");
                            assert!(!matches!(entry.name, "ST ANDREWS GREEN" | "DUCKING SWAY"), "{label}: no blind counter or sway");
                        }
                    }
                    if matches!(fighter, Ryu | Ken | Retsu | Geki | Joe | Sagat) {
                        assert!(kit.projectile.is_some(), "{label}: projectile");
                    }
                    if fighter != Birdie {
                        assert!(kit.anti_air.is_some() || fighter == Eagle, "{label}: anti-air");
                    }
                    if fighter == Birdie {
                        assert_eq!(kit.advance.unwrap().input.motion, &[crate::moves::Dir::Down, crate::moves::Dir::DownForward, crate::moves::Dir::Forward]);
                        assert_eq!(kit.advance_alt.unwrap().name, "BULL HORN");
                    }
                    if fighter == Sagat {
                        assert_eq!(kit.projectile.unwrap().name, "HIGH TIGER SHOT");
                        assert_eq!(kit.projectile_alt.unwrap().name, "LOW TIGER SHOT");
                        assert_eq!(kit.advance.unwrap().name, "TIGER KNEE");
                    }
                    if fighter == Mike {
                        assert_eq!(kit.advance.unwrap().name, "MURDER STRAIGHT");
                    }
                }
            }
        }
        assert_eq!(pairs, 59, "all supported fighter/revision/loadout pairs");
    }
}
