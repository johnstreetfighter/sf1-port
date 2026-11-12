//! The native campaign graph and the player's persistent World Tour profile.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::era::{listed_families, listed_revisions, loadouts, Loadout, Revision, StylePick};
use crate::roster::TournamentFighter;

use super::Country;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionKind {
    RivalBout,
    SurvivalGauntlet {
        recovery_percent: u8,
    },
    TimeAttack {
        max_ticks: u8,
    },
    RuleModifierTrial(RuleModifier),
    EraStyleMastery {
        revision: Revision,
    },
    DramaticBoss {
        ally: TournamentFighter,
        finale: bool,
    },
}

crate::canonical::canonical_enum!(MissionKind, "WorldTourMissionKind.v1";
    RivalBout => 0, SurvivalGauntlet { recovery_percent } => 1, TimeAttack { max_ticks } => 2,
    RuleModifierTrial(modifier) => 3, EraStyleMastery { revision } => 4, DramaticBoss { ally, finale } => 5,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleModifier {
    NoBlocking,
    ProjectileDefense,
    ComboOnly,
    PureFootsies,
}

crate::canonical::canonical_enum!(RuleModifier, "WorldTourRuleModifier.v1";
    NoBlocking => 0, ProjectileDefense => 1, ComboOnly => 2, PureFootsies => 3,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Perk {
    VitalSpirit,
    MeterSurge,
    GuardMaster,
    AirGuard,
    IsmPlus,
}

crate::canonical::canonical_enum!(Perk, "WorldTourPerk.v1";
    VitalSpirit => 0, MeterSurge => 1, GuardMaster => 2, AirGuard => 3, IsmPlus => 4,
);

impl Perk {
    const ALL: [Self; 5] = [
        Self::VitalSpirit,
        Self::MeterSurge,
        Self::GuardMaster,
        Self::AirGuard,
        Self::IsmPlus,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::VitalSpirit => "VITAL SPIRIT",
            Self::MeterSurge => "METER SURGE",
            Self::GuardMaster => "GUARD MASTER",
            Self::AirGuard => "AIR GUARD",
            Self::IsmPlus => "ISM PLUS",
        }
    }

    const fn key(self) -> &'static str {
        match self {
            Self::VitalSpirit => "VitalSpirit",
            Self::MeterSurge => "MeterSurge",
            Self::GuardMaster => "GuardMaster",
            Self::AirGuard => "AirGuard",
            Self::IsmPlus => "IsmPlus",
        }
    }

    fn from_key(key: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|p| p.key() == key)
            .ok_or_else(|| format!("unknown perk {key:?}"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldTourNode {
    pub id: &'static str,
    pub country: Country,
    pub title: &'static str,
    pub mission: MissionKind,
    pub opponents: &'static [TournamentFighter],
    /// Fighter whose home stage the mission uses.
    pub stage: TournamentFighter,
    pub xp_reward: u16,
    pub next: &'static [&'static str],
    pub unlocks_perk: Option<Perk>,
}

impl WorldTourNode {
    pub const fn is_finale(&self) -> bool {
        matches!(self.mission, MissionKind::DramaticBoss { finale: true, .. })
    }

    pub fn player_style(&self, selected: StylePick) -> StylePick {
        let revision = match self.mission {
            MissionKind::EraStyleMastery { revision } => revision,
            _ if self.is_finale() => Revision::Sf6,
            _ => return selected,
        };
        style_with_revision(revision, selected.loadout)
    }

    pub fn opponent_style(&self) -> StylePick {
        match self.mission {
            MissionKind::EraStyleMastery { revision } => {
                style_with_revision(revision, Loadout::Standard)
            }
            _ => StylePick::default(),
        }
    }

    pub const fn ally_for(&self, fighter: TournamentFighter) -> Option<TournamentFighter> {
        match self.mission {
            MissionKind::DramaticBoss { finale: true, .. }
                if matches!(fighter, TournamentFighter::Ken) =>
            {
                Some(TournamentFighter::Ryu)
            }
            MissionKind::DramaticBoss { ally, .. } => Some(ally),
            _ => None,
        }
    }
}

fn style_with_revision(revision: Revision, selected: Loadout) -> StylePick {
    let available = loadouts(revision);
    let loadout = if available.contains(&selected) {
        selected
    } else {
        available[0]
    };
    StylePick::for_revision(revision, loadout)
}

use Country::{China, England, Japan, Thailand, Usa};
use MissionKind::{
    DramaticBoss, EraStyleMastery, RivalBout, RuleModifierTrial, SurvivalGauntlet, TimeAttack,
};
use RuleModifier::{ComboOnly, NoBlocking, ProjectileDefense, PureFootsies};
use TournamentFighter::{Adon, Birdie, Eagle, Geki, Gen, Joe, Ken, Lee, Mike, Retsu, Ryu, Sagat};

pub const NODES: [WorldTourNode; 18] = [
    WorldTourNode {
        id: "jp-1-dojo",
        country: Japan,
        title: "SHORINJI DOJO",
        mission: RivalBout,
        opponents: &[Retsu],
        stage: Retsu,
        xp_reward: 150,
        next: &["jp-2-shuriken", "jp-3-era"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "jp-2-shuriken",
        country: Japan,
        title: "SHURIKEN DEFENSE",
        mission: RuleModifierTrial(ProjectileDefense),
        opponents: &[Geki],
        stage: Geki,
        xp_reward: 200,
        next: &["jp-4-boss"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "jp-3-era",
        country: Japan,
        title: "SUPER TURBO",
        mission: EraStyleMastery {
            revision: Revision::St,
        },
        opponents: &[Ryu],
        stage: Retsu,
        xp_reward: 250,
        next: &["jp-4-boss"],
        unlocks_perk: Some(Perk::VitalSpirit),
    },
    WorldTourNode {
        id: "jp-4-boss",
        country: Japan,
        title: "NINJA SHOWDOWN",
        mission: DramaticBoss {
            ally: Retsu,
            finale: false,
        },
        opponents: &[Geki],
        stage: Geki,
        xp_reward: 300,
        next: &["us-1-ring", "cn-1-wall"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "us-1-ring",
        country: Usa,
        title: "STREET RING",
        mission: RivalBout,
        opponents: &[Joe],
        stage: Joe,
        xp_reward: 150,
        next: &["us-2-rush", "us-3-era"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "us-2-rush",
        country: Usa,
        title: "NO BLOCKING",
        mission: RuleModifierTrial(NoBlocking),
        opponents: &[Mike],
        stage: Mike,
        xp_reward: 200,
        next: &["us-4-boss"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "us-3-era",
        country: Usa,
        title: "ISM TRIAL",
        mission: EraStyleMastery {
            revision: Revision::Sfz3,
        },
        opponents: &[Ken],
        stage: Joe,
        xp_reward: 250,
        next: &["us-4-boss"],
        unlocks_perk: Some(Perk::IsmPlus),
    },
    WorldTourNode {
        id: "us-4-boss",
        country: Usa,
        title: "HEAVYWEIGHT BOUT",
        mission: DramaticBoss {
            ally: Joe,
            finale: false,
        },
        opponents: &[Mike],
        stage: Mike,
        xp_reward: 300,
        next: &["en-1-punk"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "cn-1-wall",
        country: China,
        title: "GREAT WALL",
        mission: TimeAttack { max_ticks: 40 },
        opponents: &[Lee],
        stage: Lee,
        xp_reward: 200,
        next: &["cn-2-combo", "cn-3-era"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "cn-2-combo",
        country: China,
        title: "COMBO TRIAL",
        mission: RuleModifierTrial(ComboOnly),
        opponents: &[Gen],
        stage: Gen,
        xp_reward: 200,
        next: &["cn-4-boss"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "cn-3-era",
        country: China,
        title: "THIRD STRIKE",
        mission: EraStyleMastery {
            revision: Revision::Sf3ThreeS,
        },
        opponents: &[Lee],
        stage: Lee,
        xp_reward: 250,
        next: &["cn-4-boss"],
        unlocks_perk: Some(Perk::GuardMaster),
    },
    WorldTourNode {
        id: "cn-4-boss",
        country: China,
        title: "ASSASSIN SHOWDOWN",
        mission: DramaticBoss {
            ally: Lee,
            finale: false,
        },
        opponents: &[Gen],
        stage: Gen,
        xp_reward: 300,
        next: &["en-1-punk"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "en-1-punk",
        country: England,
        title: "PUNK GAUNTLET",
        mission: SurvivalGauntlet {
            recovery_percent: 20,
        },
        opponents: &[Joe, Birdie, Eagle],
        stage: Birdie,
        xp_reward: 300,
        next: &["en-2-footsies", "en-3-era"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "en-2-footsies",
        country: England,
        title: "PURE FOOTSIES",
        mission: RuleModifierTrial(PureFootsies),
        opponents: &[Eagle],
        stage: Eagle,
        xp_reward: 200,
        next: &["en-4-boss"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "en-3-era",
        country: England,
        title: "ULTRA TRIAL",
        mission: EraStyleMastery {
            revision: Revision::Usf4,
        },
        opponents: &[Birdie],
        stage: Birdie,
        xp_reward: 250,
        next: &["en-4-boss"],
        unlocks_perk: Some(Perk::AirGuard),
    },
    WorldTourNode {
        id: "en-4-boss",
        country: England,
        title: "PUNK BOSS",
        mission: DramaticBoss {
            ally: Eagle,
            finale: false,
        },
        opponents: &[Birdie],
        stage: Birdie,
        xp_reward: 300,
        next: &["th-1-gauntlet", "th-2-era"],
        unlocks_perk: None,
    },
    WorldTourNode {
        id: "th-1-gauntlet",
        country: Thailand,
        title: "THAI GAUNTLET",
        mission: SurvivalGauntlet {
            recovery_percent: 25,
        },
        opponents: &[Retsu, Gen, Eagle, Adon],
        stage: Adon,
        xp_reward: 400,
        next: &["th-2-era"],
        unlocks_perk: Some(Perk::MeterSurge),
    },
    WorldTourNode {
        id: "th-2-era",
        country: Thailand,
        title: "GRAND FINALE",
        mission: DramaticBoss {
            ally: Ken,
            finale: true,
        },
        opponents: &[Sagat],
        stage: Sagat,
        xp_reward: 500,
        next: &[],
        unlocks_perk: None,
    },
];

pub fn map() -> &'static [WorldTourNode] {
    &NODES
}

pub fn node(id: &str) -> Option<&'static WorldTourNode> {
    map().iter().find(|node| node.id == id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Attack,
    Defense,
    Speed,
}

pub const STAT_COST: u16 = 100;
const MAX_STAT: u8 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldTourProfile {
    pub fighter: TournamentFighter,
    pub style: StylePick,
    pub rpg: bool,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub perk: Option<Perk>,
    current_node: &'static str,
    xp: u16,
    cleared_nodes: BTreeSet<&'static str>,
    unlocked_perks: BTreeSet<Perk>,
}

impl Default for WorldTourProfile {
    fn default() -> Self {
        Self {
            fighter: Ryu,
            style: StylePick::default(),
            rpg: true,
            attack: 0,
            defense: 0,
            speed: 0,
            perk: None,
            current_node: NODES[0].id,
            xp: 0,
            cleared_nodes: BTreeSet::new(),
            unlocked_perks: BTreeSet::new(),
        }
    }
}

impl WorldTourProfile {
    pub fn current_node(&self) -> &'static WorldTourNode {
        node(self.current_node).expect("the profile stores a valid World Tour node")
    }

    pub fn xp(&self) -> u16 {
        self.xp
    }

    pub fn unlocked_perks(&self) -> &BTreeSet<Perk> {
        &self.unlocked_perks
    }

    pub fn is_cleared(&self, node: &WorldTourNode) -> bool {
        self.cleared_nodes.contains(node.id)
    }

    pub fn is_unlocked(&self, node: &WorldTourNode) -> bool {
        node.id == NODES[0].id
            || self.is_cleared(node)
            || self.cleared_nodes.contains("th-2-era")
            || map()
                .iter()
                .any(|from| self.is_cleared(from) && from.next.contains(&node.id))
    }

    pub fn select_node(&mut self, id: &str) -> bool {
        let Some(node) = node(id).filter(|node| self.is_unlocked(node)) else {
            return false;
        };
        if node.id == self.current_node {
            return false;
        }
        self.current_node = node.id;
        true
    }

    /// A replay remains playable and awards XP only on the first victory.
    pub fn complete(&mut self, node: &'static WorldTourNode) -> bool {
        if !self.is_unlocked(node) || !self.cleared_nodes.insert(node.id) {
            return false;
        }
        self.xp = self.xp.saturating_add(node.xp_reward);
        self.unlocked_perks.extend(node.unlocks_perk);
        true
    }

    pub fn adjust(&mut self, stat: Stat, upgrade: bool) -> bool {
        let value = match stat {
            Stat::Attack => &mut self.attack,
            Stat::Defense => &mut self.defense,
            Stat::Speed => &mut self.speed,
        };
        if upgrade && *value < MAX_STAT && self.xp >= STAT_COST {
            *value += 1;
            self.xp -= STAT_COST;
        } else if !upgrade && *value > 0 {
            *value -= 1;
            self.xp = self.xp.saturating_add(STAT_COST);
        } else {
            return false;
        }
        true
    }

    pub fn equip(&mut self, perk: Option<Perk>) -> bool {
        if self.perk == perk || perk.is_some_and(|p| !self.unlocked_perks.contains(&p)) {
            return false;
        }
        self.perk = perk;
        true
    }

    fn to_toml(&self) -> String {
        // These strings and lists use basic TOML's JSON-compatible quoting.
        let loadout = if self.style.loadout == Loadout::Standard {
            "Default"
        } else {
            self.style.loadout.name()
        };
        let cleared =
            serde_json::to_string(&self.cleared_nodes).expect("node IDs serialize as strings");
        let perks = self
            .unlocked_perks
            .iter()
            .map(|perk| perk.key())
            .collect::<Vec<_>>();
        let perks = serde_json::to_string(&perks).expect("perk keys serialize as strings");
        format!(
            "fighter = {:?}\nera = {:?}\nrevision = {:?}\nloadout = {loadout:?}\ncurrent_node = {:?}\nrpg = {}\nxp = {}\nattack = {}\ndefense = {}\nspeed = {}\nperk = {:?}\ncleared_nodes = {cleared}\nunlocked_perks = {perks}\n",
            self.fighter.name(), self.style.family.name(), self.style.revision.name(), self.current_node,
            self.rpg, self.xp, self.attack, self.defense, self.speed, self.perk.map_or("None", Perk::key),
        )
    }

    fn from_toml(text: &str) -> Result<Self, String> {
        let mut profile = Self::default();
        let mut style = [
            String::from("SF1+"),
            String::from("SF1+"),
            String::from("Default"),
        ];
        let mut equipped = None;
        for (line_no, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("expected key = value on line {}", line_no + 1))?;
            let value = value.trim();
            let string = || serde_json::from_str::<String>(value).map_err(|e| e.to_string());
            let array = || serde_json::from_str::<Vec<String>>(value).map_err(|e| e.to_string());
            let stat = || {
                value
                    .parse::<u8>()
                    .ok()
                    .filter(|n| *n <= MAX_STAT)
                    .ok_or_else(|| format!("expected a stat between 0 and {MAX_STAT}"))
            };
            match key.trim() {
                "fighter" => {
                    let name = string()?;
                    profile.fighter = TournamentFighter::ALL
                        .into_iter()
                        .find(|f| f.name().eq_ignore_ascii_case(&name))
                        .ok_or_else(|| format!("unknown fighter {name:?}"))?;
                }
                "era" => style[0] = string()?,
                "revision" => style[1] = string()?,
                "loadout" => style[2] = string()?,
                "current_node" => profile.current_node = read_node(&string()?)?.id,
                "rpg" => {
                    profile.rpg = value
                        .parse()
                        .map_err(|_| "expected true or false for rpg")?
                }
                "xp" => profile.xp = value.parse().map_err(|_| "invalid XP")?,
                "attack" => profile.attack = stat()?,
                "defense" => profile.defense = stat()?,
                "speed" => profile.speed = stat()?,
                "perk" => {
                    let name = string()?;
                    if name != "None" {
                        equipped = Some(Perk::from_key(&name)?);
                    }
                }
                "cleared_nodes" => {
                    for id in array()? {
                        profile.cleared_nodes.insert(read_node(&id)?.id);
                    }
                }
                "unlocked_perks" => {
                    for name in array()? {
                        profile.unlocked_perks.insert(Perk::from_key(&name)?);
                    }
                }
                _ => {}
            }
        }
        let [family, revision, loadout] = style;
        let family = listed_families(profile.fighter, true)
            .into_iter()
            .find(|f| f.name() == family)
            .ok_or_else(|| format!("unknown era {family:?}"))?;
        let revision = listed_revisions(profile.fighter, family, true)
            .into_iter()
            .find(|r| r.name() == revision)
            .ok_or_else(|| format!("unknown revision {revision:?}"))?;
        let loadout = loadouts(revision)
            .into_iter()
            .find(|pick| {
                pick.name() == loadout || (*pick == Loadout::Standard && loadout == "Default")
            })
            .ok_or_else(|| format!("unknown loadout {loadout:?}"))?;
        profile.style = StylePick::for_revision(revision, loadout);
        profile.equip(equipped);
        if !profile.is_unlocked(profile.current_node()) {
            profile.current_node = NODES[0].id;
        }
        Ok(profile)
    }

    pub fn load_or_default(path: &Path) -> (Self, Option<String>) {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return (Self::default(), None),
            Err(e) => {
                return (
                    Self::default(),
                    Some(format!("cannot read {}: {e}", path.display())),
                )
            }
        };
        match Self::from_toml(&text) {
            Ok(profile) => (profile, None),
            Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        static TEMP_ID: AtomicUsize = AtomicUsize::new(0);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_extension(format!("toml.{}.{}.tmp", std::process::id(), id));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| format!("cannot create {}: {e}", temp.display()))?;
        let written = file
            .write_all(self.to_toml().as_bytes())
            .and_then(|()| file.sync_all());
        drop(file);
        if let Err(e) = written.and_then(|()| fs::rename(&temp, path)) {
            let _ = fs::remove_file(&temp);
            return Err(format!("cannot save {}: {e}", path.display()));
        }
        Ok(())
    }
}

fn read_node(id: &str) -> Result<&'static WorldTourNode, String> {
    node(id).ok_or_else(|| format!("unknown World Tour node {id:?}"))
}
