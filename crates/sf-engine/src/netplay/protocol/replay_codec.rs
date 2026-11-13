//! Stable replay metadata using the same typed rule codec as online play.
use super::*;
use crate::config::{EvilRyuPlaytest, WorldTourBonuses, WorldTourRules};
use crate::era::{Costume, PALETTES};
use crate::front::fight::FightKind;
use crate::utility::RosterFighter;
use crate::world_tour::native::{MissionKind, Perk, RuleModifier};

enum_codec!(write_costume, read_costume, Costume, {
    Classic1987 => 0, Sf2 => 1, Alpha => 2, Sf5 => 3, Sf6 => 4,
});
enum_codec!(write_perk, read_perk, Perk, {
    VitalSpirit => 0, MeterSurge => 1, GuardMaster => 2, AirGuard => 3, IsmPlus => 4,
});
enum_codec!(write_modifier, read_modifier, RuleModifier, {
    NoBlocking => 0, ProjectileDefense => 1, ComboOnly => 2, PureFootsies => 3,
});
enum_codec!(write_kit, read_kit, EvilRyuPlaytest, {
    UltraSf2 => 0, Alpha2 => 1, Alpha3A => 2, Alpha3V => 3, Alpha3X => 4,
    Ae2012 => 5, Ae2012Ultra2 => 6, Usf4 => 7, Kage => 8, KageII => 9, Sf6 => 10,
});

pub(crate) fn write_replay_header(
    config: &MatchConfig,
    kind: FightKind,
    fighters: [RosterFighter; 2],
    styles: [StylePick; 2],
    out: &mut Vec<u8>,
) {
    write_config(config, out);
    write_scheme(config.p1_scheme, out);
    write_scheme(config.p2_scheme, out);
    match kind {
        FightKind::Local => 0u8.write(out),
        FightKind::Arcade => 1u8.write(out),
        FightKind::TrainingCpu => 2u8.write(out),
        FightKind::TrainingDummy => 3u8.write(out),
        FightKind::Dramatic { co_op } => {
            4u8.write(out);
            co_op.write(out);
        }
        FightKind::WorldTour => 5u8.write(out),
    }
    config.dramatic.is_some().write(out);
    if let Some(rules) = config.dramatic {
        write_dramatic_rules(rules, out);
    }
    config.world_tour.is_some().write(out);
    if let Some(rules) = config.world_tour {
        write_mission(rules.mission, out);
        rules.bonuses.is_some().write(out);
        if let Some(WorldTourBonuses {
            attack,
            defense,
            speed,
            perk,
        }) = rules.bonuses
        {
            attack.write(out);
            defense.write(out);
            speed.write(out);
            perk.is_some().write(out);
            if let Some(perk) = perk {
                write_perk(perk, out);
            }
        }
        rules.starting_hp.is_some().write(out);
        if let Some(hp) = rules.starting_hp {
            hp.write(out);
        }
    }
    config.evil_ryu_playtest.is_some().write(out);
    if let Some(kit) = config.evil_ryu_playtest {
        write_kit(kit, out);
    }
    for (fighter, style) in fighters.into_iter().zip(styles) {
        match fighter {
            RosterFighter::Tournament(fighter) => {
                0u8.write(out);
                write_fighter(fighter, out);
            }
            RosterFighter::EvilRyu => 1u8.write(out),
            RosterFighter::MBison => 2u8.write(out),
        }
        write_revision(style.revision, out);
        write_loadout(style.loadout, out);
        write_costume(style.costume, out);
        style.palette.write(out);
    }
}

pub(crate) fn read_replay_header(
    bytes: &[u8],
) -> Option<(MatchConfig, FightKind, [RosterFighter; 2], [StylePick; 2])> {
    let mut read = Reader::new(bytes);
    let mut config = read_config(&mut read)?;
    config.p1_scheme = read_scheme(&mut read)?;
    config.p2_scheme = read_scheme(&mut read)?;
    let kind = match read.u8()? {
        0 => FightKind::Local,
        1 => FightKind::Arcade,
        2 => FightKind::TrainingCpu,
        3 => FightKind::TrainingDummy,
        4 => FightKind::Dramatic {
            co_op: read.boolean()?,
        },
        5 => FightKind::WorldTour,
        _ => return None,
    };
    if read.boolean()? {
        config.dramatic = Some(read_dramatic_rules(&mut read)?);
    }
    if read.boolean()? {
        let mission = read_mission(&mut read)?;
        let bonuses = if read.boolean()? {
            Some(WorldTourBonuses {
                attack: read.u8()?,
                defense: read.u8()?,
                speed: read.u8()?,
                perk: if read.boolean()? {
                    Some(read_perk(&mut read)?)
                } else {
                    None
                },
            })
        } else {
            None
        };
        let starting_hp = if read.boolean()? {
            Some(read.u16()?)
        } else {
            None
        };
        config.world_tour = Some(WorldTourRules {
            mission,
            bonuses,
            starting_hp,
        });
    }
    if read.boolean()? {
        config.evil_ryu_playtest = Some(read_kit(&mut read)?);
    }
    let (p1, style1) = read_player(&mut read)?;
    let (p2, style2) = read_player(&mut read)?;
    read.done()
        .then_some((config, kind, [p1, p2], [style1, style2]))
}

fn read_player(read: &mut Reader<'_>) -> Option<(RosterFighter, StylePick)> {
    let fighter = match read.u8()? {
        0 => RosterFighter::Tournament(read_fighter(read)?),
        1 => RosterFighter::EvilRyu,
        2 => RosterFighter::MBison,
        _ => return None,
    };
    let revision = read_revision(read)?;
    let loadout = read_loadout(read)?;
    let costume = read_costume(read)?;
    let palette = read.u8()?;
    (palette < PALETTES && loadouts(revision).contains(&loadout)).then_some((
        fighter,
        StylePick {
            family: revision.family(),
            revision,
            loadout,
            costume,
            palette,
        },
    ))
}

fn write_mission(mission: MissionKind, out: &mut Vec<u8>) {
    match mission {
        MissionKind::RivalBout => 0u8.write(out),
        MissionKind::SurvivalGauntlet { recovery_percent } => {
            1u8.write(out);
            recovery_percent.write(out);
        }
        MissionKind::TimeAttack { max_ticks } => {
            2u8.write(out);
            max_ticks.write(out);
        }
        MissionKind::RuleModifierTrial(modifier) => {
            3u8.write(out);
            write_modifier(modifier, out);
        }
        MissionKind::EraStyleMastery { revision } => {
            4u8.write(out);
            write_revision(revision, out);
        }
        MissionKind::DramaticBoss { ally, finale } => {
            5u8.write(out);
            write_fighter(ally, out);
            finale.write(out);
        }
    }
}

fn read_mission(read: &mut Reader<'_>) -> Option<MissionKind> {
    Some(match read.u8()? {
        0 => MissionKind::RivalBout,
        1 => MissionKind::SurvivalGauntlet {
            recovery_percent: read.u8()?,
        },
        2 => MissionKind::TimeAttack {
            max_ticks: read.u8()?,
        },
        3 => MissionKind::RuleModifierTrial(read_modifier(read)?),
        4 => MissionKind::EraStyleMastery {
            revision: read_revision(read)?,
        },
        5 => MissionKind::DramaticBoss {
            ally: read_fighter(read)?,
            finale: read.boolean()?,
        },
        _ => return None,
    })
}
