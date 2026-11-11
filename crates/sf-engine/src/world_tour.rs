//! 1987 World Tour (`#111`, `O20`).
//!
//! Any of the twelve fighters walks a five-country map. Missions write their
//! rules onto [`MatchConfig`]. RPG boosts stay inside the tour. Arcade,
//! Versus, Training, and online keep the full roster unlocked.

use crate::config::{MatchConfig, TourRules};
use crate::dramatic::{DramaticBattle, Pad};
use crate::era::{families, costumes, PALETTES};
use crate::roster::{PlayMode, TournamentFighter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    Japan,
    Usa,
    China,
    England,
    Thailand,
}

impl Country {
    pub const fn name(self) -> &'static str {
        match self {
            Country::Japan => "Japan",
            Country::Usa => "USA",
            Country::China => "China",
            Country::England => "England",
            Country::Thailand => "Thailand",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mission {
    ProjectileDefense,
    NoBlocking,
    RawSurvival,
    DojoGauntlet,
    DramaticBoss,
    CrossEra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub id: &'static str,
    pub country: Country,
    pub mission: Mission,
    pub next: &'static [&'static str],
}

const MAP: &[Node] = &[
    Node { id: "japan-dojo", country: Country::Japan, mission: Mission::DojoGauntlet, next: &["usa-footsies", "china-showdown"] },
    Node { id: "usa-footsies", country: Country::Usa, mission: Mission::NoBlocking, next: &["england-survival", "thailand-boss"] },
    Node { id: "china-showdown", country: Country::China, mission: Mission::CrossEra, next: &["england-survival"] },
    Node { id: "england-survival", country: Country::England, mission: Mission::RawSurvival, next: &["thailand-boss", "usa-time"] },
    Node { id: "usa-time", country: Country::Usa, mission: Mission::ProjectileDefense, next: &["thailand-boss"] },
    Node { id: "thailand-boss", country: Country::Thailand, mission: Mission::DramaticBoss, next: &[] },
];

pub fn map() -> &'static [Node] {
    MAP
}

pub fn node(id: &str) -> Option<&'static Node> {
    MAP.iter().find(|node| node.id == id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perk {
    IsmPlus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Campaign {
    pub fighter: TournamentFighter,
    pub rpg: bool,
    pub xp: u16,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub perk: Option<Perk>,
    pub node: String,
    pub waves_left: u8,
    pub bonus: Option<crate::bonus::Challenger>,
}

impl Campaign {
    pub fn new(fighter: TournamentFighter, rpg: bool) -> Self {
        Self {
            fighter,
            rpg,
            xp: 0,
            attack: 0,
            defense: 0,
            speed: 0,
            perk: None,
            node: "japan-dojo".into(),
            waves_left: 0,
            bonus: None,
        }
    }

    pub fn for_challenger(challenger: crate::bonus::Challenger, rpg: bool) -> Self {
        let mut tour = Self::new(crate::roster::TournamentFighter::Ryu, rpg);
        tour.bonus = Some(challenger);
        tour
    }

    pub fn at(&self) -> &'static Node {
        node(&self.node).unwrap_or(&MAP[0])
    }

    pub fn travel(&mut self, id: &str) -> bool {
        if !self.at().next.contains(&id) || node(id).is_none() {
            return false;
        }
        self.node = id.into();
        if self.at().mission == Mission::RawSurvival {
            self.waves_left = 3;
        }
        true
    }

    /// Copy the mission onto the match. Boosts are not written when RPG is off.
    pub fn start_match(&self, mut config: MatchConfig) -> MatchConfig {
        config.tour = match self.at().mission {
            Mission::ProjectileDefense => TourRules { projectile_only: true, time_attack_frames: 1800, ..TourRules::off() },
            Mission::NoBlocking => TourRules { no_blocking: true, ..TourRules::off() },
            Mission::RawSurvival => TourRules { survival_waves: self.waves_left.max(3), ..TourRules::off() },
            Mission::DojoGauntlet => TourRules { survival_waves: 3, ..TourRules::off() },
            Mission::DramaticBoss => TourRules { dramatic: true, ..TourRules::off() },
            Mission::CrossEra => TourRules { cross_era: true, ..TourRules::off() },
        };
        config.tour.rpg = self.rpg;
        config
    }

    pub fn allows_attack(&self, projectile: bool) -> bool {
        !self.start_match(MatchConfig::faithful()).tour.projectile_only || projectile
    }

    pub fn allows_block(&self) -> bool {
        !self.start_match(MatchConfig::faithful()).tour.no_blocking
    }

    pub fn win_wave(&mut self) -> bool {
        if self.waves_left == 0 {
            return true;
        }
        self.waves_left -= 1;
        self.waves_left == 0
    }

    pub fn run_dramatic(&self) -> bool {
        if !self.start_match(MatchConfig::faithful()).tour.dramatic {
            return false;
        }
        let mut battle = DramaticBattle::preset();
        battle.step(Pad { left: false, right: true, attack: true }, Pad::default(), Pad { left: true, right: false, attack: false });
        true
    }

    pub fn win(&mut self) {
        if self.rpg {
            self.xp = self.xp.saturating_add(10);
        }
    }

    pub fn equip(&mut self) -> bool {
        if !self.rpg || self.xp < 10 {
            return false;
        }
        self.xp -= 10;
        self.attack = (self.attack + 1).min(5);
        self.defense = (self.defense + 1).min(5);
        self.speed = (self.speed + 1).min(5);
        self.perk = Some(Perk::IsmPlus);
        true
    }

    pub fn outgoing(&self, base: u16) -> u16 {
        if !self.rpg {
            return base;
        }
        base.saturating_add(u16::from(self.attack) * 2)
    }

    pub fn incoming(&self, base: u16) -> u16 {
        if !self.rpg {
            return base;
        }
        base.saturating_sub(u16::from(self.defense))
    }

    pub fn save(&self) -> String {
        format!(
            "fighter={};rpg={};xp={};atk={};def={};spd={};perk={};node={};bonus={}",
            self.fighter.name(),
            self.rpg as u8,
            self.xp,
            self.attack,
            self.defense,
            self.speed,
            self.perk.map(|_| "ism").unwrap_or("none"),
            self.node,
            self.bonus.map(|one| one.name()).unwrap_or("none")
        )
    }

    pub fn load(text: &str) -> Option<Self> {
        let mut fighter = None;
        let mut rpg = false;
        let mut xp = 0;
        let mut attack = 0;
        let mut defense = 0;
        let mut speed = 0;
        let mut perk = None;
        let mut node = None;
        let mut bonus = None;
        for part in text.split(';') {
            let (key, value) = part.split_once('=')?;
            match key {
                "fighter" => fighter = TournamentFighter::ALL.into_iter().find(|one| one.name() == value),
                "rpg" => rpg = value == "1",
                "xp" => xp = value.parse().ok()?,
                "atk" => attack = value.parse().ok()?,
                "def" => defense = value.parse().ok()?,
                "spd" => speed = value.parse().ok()?,
                "perk" => perk = if value == "ism" { Some(Perk::IsmPlus) } else { None },
                "node" => node = Some(value.to_string()),
                "bonus" => {
                    bonus = crate::bonus::Challenger::ALL.into_iter().find(|one| one.name() == value);
                    if value != "none" && bonus.is_none() {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some(Self {
            fighter: fighter?,
            rpg,
            xp,
            attack,
            defense,
            speed,
            perk,
            node: node?,
            waves_left: 0,
            bonus,
        })
    }
}

/// Arcade, Versus, Training, and online never lock a fighter, style, costume, or palette behind the tour.
pub fn catalog_unlocked(_xp: u16, mode: PlayMode) -> bool {
    let _ = mode;
    TournamentFighter::ALL.iter().all(|fighter| {
        let fams = families(*fighter);
        !fams.is_empty()
            && fams.iter().all(|family| !costumes(*fighter, *family).is_empty())
            && PALETTES == 8
    }) && matches!(mode, PlayMode::Arcade | PlayMode::Versus | PlayMode::Training | PlayMode::Cpu)
}

pub fn countries_on_the_map() -> [Country; 5] {
    [Country::Japan, Country::Usa, Country::China, Country::England, Country::Thailand]
}

pub fn era_count(fighter: TournamentFighter) -> usize {
    families(fighter).len()
}

const LORE: &[(&str, &str, &str)] = &[
    ("Ryu & Ken vs. Akuma / Gouken", "Ryu", "Akuma"),
    ("Gouken & Retsu vs. Goutetsu", "Gouken", "Goutetsu"),
    ("Cody & Guy vs. Mike Haggar", "Cody", "Haggar"),
    ("Gen & Chun-Li vs. M. Bison", "Gen", "M. Bison"),
    ("Sagat vs. Gou Hibiki & Dan", "Sagat", "Gou Hibiki"),
];

pub fn lore_episodes(bonus_challengers: bool) -> &'static [(&'static str, &'static str, &'static str)] {
    if bonus_challengers { LORE } else { &[] }
}
