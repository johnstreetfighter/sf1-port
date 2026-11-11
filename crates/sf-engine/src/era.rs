//! Era family, SF2 revision, costume, and the Super gauge (`#103`, `O12`).
//!
//! Character select offers only the families and revisions a fighter appeared in.
//! Ryu, Ken, and Sagat carry the five Street Fighter II revisions. The Super
//! Turbo stock fills from hits and specials, and a full stock is what lets a
//! Super Combo or a Super cancel come out.

use crate::roster::TournamentFighter;
use crate::sf1plus::{CombatEvent, CombatSession, EraStyle, MoveKind, Pad, Sf1PlusRules};
use crate::config::RulesetPreset;
use sf_video::callouts::CalloutKind;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraFamily {
    Sf1987,
    Sf1Plus,
    Sf2,
    SfZ,
    Sf3,
    Sf4,
    Sf5,
    Sf6,
}

crate::canonical::canonical_enum!(EraFamily, "EraFamily.v1";
    Sf1987 => 0, Sf1Plus => 1, Sf2 => 2, SfZ => 3, Sf3 => 4, Sf4 => 5, Sf5 => 6, Sf6 => 7,
);

impl EraFamily {
    pub const fn name(self) -> &'static str {
        match self {
            EraFamily::Sf1987 => "SF1",
            EraFamily::Sf1Plus => "SF1+",
            EraFamily::Sf2 => "SF2",
            EraFamily::SfZ => "SFZ",
            EraFamily::Sf3 => "SF3",
            EraFamily::Sf4 => "SF4",
            EraFamily::Sf5 => "SF5",
            EraFamily::Sf6 => "SF6",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision {
    Arcade1987,
    Sf1Plus,
    Ww,
    Ce,
    Hf,
    Super,
    St,
    Sfz1,
    Sfz2,
    Sfz2A,
    Sfz3,
    Sf3Ng,
    Sf3TwoI,
    Sf3ThreeS,
    Sf4,
    Ssf4,
    Ae2012,
    Usf4,
    Sf5,
    Sf5Ae,
    Sf5Ce,
    Sf6,
}

crate::canonical::canonical_enum!(Revision, "EraRevision.v1";
    Arcade1987 => 0, Sf1Plus => 1, Ww => 2, Ce => 3, Hf => 4, Super => 5,
    St => 6, Sfz1 => 7, Sfz2 => 8, Sfz2A => 9, Sfz3 => 10, Sf3Ng => 11,
    Sf3TwoI => 12, Sf3ThreeS => 13, Sf4 => 14, Ssf4 => 15, Ae2012 => 16,
    Usf4 => 17, Sf5 => 18, Sf5Ae => 19, Sf5Ce => 20, Sf6 => 21,
);

impl Revision {
    pub const fn name(self) -> &'static str {
        match self {
            Revision::Arcade1987 => "1987",
            Revision::Sf1Plus => "SF1+",
            Revision::Ww => "SF2 WW",
            Revision::Ce => "SF2 CE",
            Revision::Hf => "SF2 HF",
            Revision::Super => "SF2 Super",
            Revision::St => "SF2 ST",
            Revision::Sfz1 => "SFZ1",
            Revision::Sfz2 => "SFZ2",
            Revision::Sfz2A => "SFZ2A",
            Revision::Sfz3 => "SFZ3",
            Revision::Sf3Ng => "SF3 NG",
            Revision::Sf3TwoI => "SF3 2I",
            Revision::Sf3ThreeS => "SF3 3S",
            Revision::Sf4 => "SF4",
            Revision::Ssf4 => "SSF4",
            Revision::Ae2012 => "AE2012",
            Revision::Usf4 => "USF4",
            Revision::Sf5 => "SF5",
            Revision::Sf5Ae => "SF5 AE",
            Revision::Sf5Ce => "SF5 CE",
            Revision::Sf6 => "SF6",
        }
    }

    pub const fn family(self) -> EraFamily {
        match self {
            Revision::Arcade1987 => EraFamily::Sf1987,
            Revision::Sf1Plus => EraFamily::Sf1Plus,
            Revision::Ww | Revision::Ce | Revision::Hf | Revision::Super | Revision::St => EraFamily::Sf2,
            Revision::Sfz1 | Revision::Sfz2 | Revision::Sfz2A | Revision::Sfz3 => EraFamily::SfZ,
            Revision::Sf3Ng | Revision::Sf3TwoI | Revision::Sf3ThreeS => EraFamily::Sf3,
            Revision::Sf4 | Revision::Ssf4 | Revision::Ae2012 | Revision::Usf4 => EraFamily::Sf4,
            Revision::Sf5 | Revision::Sf5Ae | Revision::Sf5Ce => EraFamily::Sf5,
            Revision::Sf6 => EraFamily::Sf6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Loadout {
    #[default]
    Standard,
    /// O.Ryu, O.Ken, or O.Sagat on Super Turbo.
    Old,
    AIsm,
    VIsm,
    XIsm,
    Art1,
    Art2,
    Art3,
    Ultra1,
    Ultra2,
    UltraDouble,
    /// SF5 pair. Vanilla and Arcade Edition keep V-Skill I.
    Skill1Trigger1,
    Skill1Trigger2,
    Skill2Trigger1,
    Skill2Trigger2,
}

crate::canonical::canonical_enum!(Loadout, "EraLoadout.v1";
    Standard => 0, Old => 1, AIsm => 2, VIsm => 3, XIsm => 4,
    Art1 => 5, Art2 => 6, Art3 => 7, Ultra1 => 8, Ultra2 => 9, UltraDouble => 10,
    Skill1Trigger1 => 11, Skill1Trigger2 => 12, Skill2Trigger1 => 13, Skill2Trigger2 => 14,
);

impl Loadout {
    pub const fn name(self) -> &'static str {
        match self {
            Loadout::Standard => "Standard",
            Loadout::Old => "Old",
            Loadout::AIsm => "A-ISM",
            Loadout::VIsm => "V-ISM",
            Loadout::XIsm => "X-ISM",
            Loadout::Art1 => "Super Art I",
            Loadout::Art2 => "Super Art II",
            Loadout::Art3 => "Super Art III",
            Loadout::Ultra1 => "Ultra I",
            Loadout::Ultra2 => "Ultra II",
            Loadout::UltraDouble => "Ultra Double",
            Loadout::Skill1Trigger1 => "V-Skill I / V-Trigger I",
            Loadout::Skill1Trigger2 => "V-Skill I / V-Trigger II",
            Loadout::Skill2Trigger1 => "V-Skill II / V-Trigger I",
            Loadout::Skill2Trigger2 => "V-Skill II / V-Trigger II",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Costume {
    #[default]
    Classic1987,
    Sf2,
    Alpha,
    Sf5,
    Sf6,
}

crate::canonical::canonical_enum!(Costume, "EraCostume.v1";
    Classic1987 => 0, Sf2 => 1, Alpha => 2, Sf5 => 3, Sf6 => 4,
);

impl Costume {
    pub const fn name(self) -> &'static str {
        match self {
            Costume::Classic1987 => "1987",
            Costume::Sf2 => "SF2",
            Costume::Alpha => "Alpha",
            Costume::Sf5 => "SF5",
            Costume::Sf6 => "SF6",
        }
    }
}

pub const PALETTES: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeMode {
    EraDefault,
    NoGauges,
    Unified,
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_enum!(GaugeMode, "GaugeMode.v1";
    EraDefault => 0,
    NoGauges => 1,
    Unified => 2,
);


impl GaugeMode {
    pub const fn slug(self) -> &'static str {
        match self {
            GaugeMode::EraDefault => "era_default",
            GaugeMode::NoGauges => "no_gauges",
            GaugeMode::Unified => "unified",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            GaugeMode::EraDefault => "Era Default",
            GaugeMode::NoGauges => "No Gauges",
            GaugeMode::Unified => "Unified Gauge",
        }
    }

    pub fn from_slug(text: &str) -> Result<Self, String> {
        for mode in [GaugeMode::EraDefault, GaugeMode::NoGauges, GaugeMode::Unified] {
            if mode.slug().eq_ignore_ascii_case(text.trim()) || mode.name().eq_ignore_ascii_case(text.trim()) {
                return Ok(mode);
            }
        }
        Err(format!("unknown gauge mode '{text}'"))
    }

    pub const fn for_preset(preset: RulesetPreset) -> Self {
        match preset {
            RulesetPreset::PureFootsies1987Plus => GaugeMode::NoGauges,
            _ => GaugeMode::EraDefault,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StylePick {
    pub family: EraFamily,
    pub revision: Revision,
    pub loadout: Loadout,
    pub costume: Costume,
    pub palette: u8,
}

crate::canonical::canonical_struct!(StylePick, "StylePick.v1", {
    family, revision, loadout, costume, palette,
});

impl Default for StylePick {
    fn default() -> Self {
        Self {
            family: EraFamily::Sf1Plus,
            revision: Revision::Sf1Plus,
            loadout: Loadout::Standard,
            costume: Costume::Classic1987,
            palette: 0,
        }
    }
}

impl StylePick {
    pub(crate) fn available_for(self, fighter: TournamentFighter, what_if: bool) -> bool {
        self == Self::for_revision(self.revision, self.loadout)
            && listed_revisions(fighter, self.family, what_if).contains(&self.revision)
            && loadouts(self.revision).contains(&self.loadout)
    }

    pub(crate) fn for_revision(revision: Revision, loadout: Loadout) -> Self {
        Self { family: revision.family(), revision, loadout, ..Self::default() }
    }

    pub(crate) fn for_preset(preset: RulesetPreset) -> Self {
        match preset {
            RulesetPreset::Faithful1987 => Self::for_revision(Revision::Arcade1987, Loadout::Standard),
            _ => Self::default(),
        }
    }

    /// Resolve the style for a zero-based Arcade ladder step.
    pub fn arcade_opponent(self, fighter: TournamentFighter, what_if: bool, step: usize) -> Self {
        let revisions = listed_revisions(fighter, self.family, what_if);
        let Some(&flagship) = revisions.last() else {
            return Self::default();
        };
        let revision = if revisions.contains(&self.revision) { self.revision } else { flagship };
        let choices = loadouts(revision);
        let index = step.saturating_sub(6) % choices.len();
        Self::for_revision(revision, choices[index])
    }

    pub fn sf2(revision: Revision, loadout: Loadout, costume: Costume, palette: u8) -> Self {
        Self {
            family: EraFamily::Sf2,
            revision,
            loadout,
            costume,
            palette: palette % PALETTES,
        }
    }
}

pub fn families(fighter: TournamentFighter) -> Vec<EraFamily> {
    let mut out = vec![EraFamily::Sf1987, EraFamily::Sf1Plus];
    if matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Sagat) {
        out.push(EraFamily::Sf2);
    }
    if matches!(
        fighter,
        TournamentFighter::Ryu
            | TournamentFighter::Ken
            | TournamentFighter::Adon
            | TournamentFighter::Sagat
            | TournamentFighter::Birdie
            | TournamentFighter::Gen
            | TournamentFighter::Eagle
    ) {
        out.push(EraFamily::SfZ);
    }
    if matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken) {
        out.push(EraFamily::Sf3);
    }
    if matches!(
        fighter,
        TournamentFighter::Ryu
            | TournamentFighter::Ken
            | TournamentFighter::Sagat
            | TournamentFighter::Adon
            | TournamentFighter::Gen
    ) {
        out.push(EraFamily::Sf4);
    }
    if matches!(
        fighter,
        TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Birdie | TournamentFighter::Sagat
    ) {
        out.push(EraFamily::Sf5);
    }
    if matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Retsu) {
        out.push(EraFamily::Sf6);
    }
    out
}

/// Families the select screen shows. What-if families stay off the canonical list.
pub fn listed_families(fighter: TournamentFighter, what_if: bool) -> Vec<EraFamily> {
    let mut out = families(fighter);
    if what_if {
        if what_if_sf2(fighter) && !out.contains(&EraFamily::Sf2) {
            out.push(EraFamily::Sf2);
        }
        if what_if_sfz(fighter) && !out.contains(&EraFamily::SfZ) {
            out.push(EraFamily::SfZ);
        }
        if what_if_sf3(fighter) && !out.contains(&EraFamily::Sf3) {
            out.push(EraFamily::Sf3);
        }
        if what_if_usf4(fighter) && !out.contains(&EraFamily::Sf4) {
            out.push(EraFamily::Sf4);
        }
        if what_if_sf5(fighter) && !out.contains(&EraFamily::Sf5) {
            out.push(EraFamily::Sf5);
        }
        if what_if_sf6(fighter) && !out.contains(&EraFamily::Sf6) {
            out.push(EraFamily::Sf6);
        }
    }
    out
}

/// Revisions in release order, with the flagship last when the era is available.
pub fn listed_revisions(fighter: TournamentFighter, family: EraFamily, what_if: bool) -> Vec<Revision> {
    if what_if && !families(fighter).contains(&family) {
        return match family {
            EraFamily::Sf2 if what_if_sf2(fighter) => vec![Revision::Hf, Revision::St],
            EraFamily::SfZ if what_if_sfz(fighter) => vec![Revision::Sfz3],
            EraFamily::Sf3 if what_if_sf3(fighter) => vec![Revision::Sf3ThreeS],
            EraFamily::Sf4 if what_if_usf4(fighter) => vec![Revision::Usf4],
            EraFamily::Sf5 if what_if_sf5(fighter) => vec![Revision::Sf5Ce],
            EraFamily::Sf6 if what_if_sf6(fighter) => vec![Revision::Sf6],
            _ => Vec::new(),
        };
    }
    revisions(fighter, family)
}

pub fn what_if_sf2(fighter: TournamentFighter) -> bool {
    matches!(
        fighter,
        TournamentFighter::Retsu
            | TournamentFighter::Geki
            | TournamentFighter::Joe
            | TournamentFighter::Mike
            | TournamentFighter::Lee
            | TournamentFighter::Gen
            | TournamentFighter::Birdie
            | TournamentFighter::Eagle
            | TournamentFighter::Adon
    )
}

pub fn what_if_sf3(fighter: TournamentFighter) -> bool {
    !matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken)
}

pub fn what_if_usf4(fighter: TournamentFighter) -> bool {
    !matches!(
        fighter,
        TournamentFighter::Ryu
            | TournamentFighter::Ken
            | TournamentFighter::Sagat
            | TournamentFighter::Adon
            | TournamentFighter::Gen
    )
}

pub fn what_if_sf5(fighter: TournamentFighter) -> bool {
    !matches!(
        fighter,
        TournamentFighter::Ryu
            | TournamentFighter::Ken
            | TournamentFighter::Birdie
            | TournamentFighter::Sagat
    )
}

pub fn what_if_sf6(fighter: TournamentFighter) -> bool {
    !matches!(
        fighter,
        TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Retsu
    )
}

pub fn what_if_sfz(fighter: TournamentFighter) -> bool {
    matches!(
        fighter,
        TournamentFighter::Retsu
            | TournamentFighter::Geki
            | TournamentFighter::Joe
            | TournamentFighter::Mike
            | TournamentFighter::Lee
    )
}

pub fn revisions(fighter: TournamentFighter, family: EraFamily) -> Vec<Revision> {
    match family {
        EraFamily::Sf1987 => vec![Revision::Arcade1987],
        EraFamily::Sf1Plus => vec![Revision::Sf1Plus],
        EraFamily::Sf2 => {
            if families(fighter).contains(&EraFamily::Sf2) {
                vec![Revision::Ww, Revision::Ce, Revision::Hf, Revision::Super, Revision::St]
            } else {
                Vec::new()
            }
        }
        EraFamily::SfZ => match fighter {
            TournamentFighter::Eagle => vec![Revision::Sfz3],
            TournamentFighter::Ryu
            | TournamentFighter::Ken
            | TournamentFighter::Adon
            | TournamentFighter::Sagat
            | TournamentFighter::Birdie
            | TournamentFighter::Gen => {
                vec![Revision::Sfz1, Revision::Sfz2, Revision::Sfz2A, Revision::Sfz3]
            }
            _ => Vec::new(),
        },
        EraFamily::Sf3 => {
            if matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken) {
                vec![Revision::Sf3Ng, Revision::Sf3TwoI, Revision::Sf3ThreeS]
            } else {
                Vec::new()
            }
        }
        EraFamily::Sf4 => match fighter {
            TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Sagat => {
                vec![Revision::Sf4, Revision::Ssf4, Revision::Ae2012, Revision::Usf4]
            }
            TournamentFighter::Adon | TournamentFighter::Gen => {
                vec![Revision::Ssf4, Revision::Ae2012, Revision::Usf4]
            }
            _ => Vec::new(),
        },
        EraFamily::Sf5 => match fighter {
            TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Birdie => {
                vec![Revision::Sf5, Revision::Sf5Ae, Revision::Sf5Ce]
            }
            TournamentFighter::Sagat => vec![Revision::Sf5Ae, Revision::Sf5Ce],
            _ => Vec::new(),
        },
        EraFamily::Sf6 => {
            if matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Retsu) {
                vec![Revision::Sf6]
            } else {
                Vec::new()
            }
        }
    }
}

pub fn loadouts(revision: Revision) -> Vec<Loadout> {
    match revision {
        Revision::St => vec![Loadout::Standard, Loadout::Old],
        Revision::Sfz3 => vec![Loadout::AIsm, Loadout::VIsm, Loadout::XIsm],
        Revision::Sf3Ng | Revision::Sf3TwoI | Revision::Sf3ThreeS => {
            vec![Loadout::Art1, Loadout::Art2, Loadout::Art3]
        }
        Revision::Sf4 => vec![Loadout::Ultra1],
        Revision::Ssf4 | Revision::Ae2012 => vec![Loadout::Ultra1, Loadout::Ultra2],
        Revision::Usf4 => vec![Loadout::Ultra1, Loadout::Ultra2, Loadout::UltraDouble],
        Revision::Sf5 => vec![Loadout::Skill1Trigger1],
        Revision::Sf5Ae => vec![Loadout::Skill1Trigger1, Loadout::Skill1Trigger2],
        Revision::Sf5Ce => vec![
            Loadout::Skill1Trigger1,
            Loadout::Skill1Trigger2,
            Loadout::Skill2Trigger1,
            Loadout::Skill2Trigger2,
        ],
        _ => vec![Loadout::Standard],
    }
}

pub fn costumes(fighter: TournamentFighter, family: EraFamily) -> Vec<Costume> {
    if family == EraFamily::Sf2 && families(fighter).contains(&EraFamily::Sf2) {
        vec![Costume::Classic1987, Costume::Sf2]
    } else if family == EraFamily::SfZ && families(fighter).contains(&EraFamily::SfZ) {
        vec![Costume::Classic1987, Costume::Alpha]
    } else if family == EraFamily::Sf5
        && matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Birdie)
    {
        vec![Costume::Classic1987, Costume::Sf5]
    } else if family == EraFamily::Sf6
        && matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken)
    {
        vec![Costume::Classic1987, Costume::Sf6]
    } else {
        vec![Costume::Classic1987]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveStat {
    pub name: &'static str,
    pub startup: u8,
    pub hitstun: u8,
    pub damage: u16,
    pub hits: u8,
    pub air_ok: bool,
    pub super_move: bool,
}

pub(crate) const STOCK_AT: u8 = 48;

#[derive(Debug, Clone)]
pub struct Sf2Session {
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
    gauge_mode: GaugeMode,
    meter: u8,
    stock: u8,
    seen_hits: usize,
    inner: CombatSession,
}

impl Sf2Session {
    pub fn new(
        fighter: TournamentFighter,
        opponent: TournamentFighter,
        revision: Revision,
        loadout: Loadout,
        gauge_mode: GaugeMode,
        rules: Sf1PlusRules,
    ) -> Self {
        Self::versus(fighter, opponent, revision, loadout, gauge_mode, rules, EraStyle::Sf1Plus)
    }

    pub fn versus(
        fighter: TournamentFighter,
        opponent: TournamentFighter,
        revision: Revision,
        loadout: Loadout,
        gauge_mode: GaugeMode,
        rules: Sf1PlusRules,
        defender: EraStyle,
    ) -> Self {
        let attacker = if revision == Revision::Arcade1987 {
            EraStyle::Sf1987
        } else {
            EraStyle::Sf1Plus
        };
        Self {
            fighter,
            revision,
            loadout,
            gauge_mode,
            meter: 0,
            stock: 0,
            seen_hits: 0,
            inner: CombatSession::with_eras(fighter, opponent, rules, attacker, defender),
        }
    }

    pub fn stock(&self) -> u8 {
        self.stock
    }

    pub fn meter(&self) -> u8 {
        self.meter
    }

    pub fn gauge_enabled(&self) -> bool {
        match self.gauge_mode {
            GaugeMode::NoGauges => false,
            GaugeMode::Unified => true,
            GaugeMode::EraDefault => self.revision == Revision::St,
        }
    }

    pub fn stat(&self, name: &str) -> Option<MoveStat> {
        move_stat(self.fighter, self.revision, self.loadout, name)
    }

    pub fn play(&mut self, name: &str) -> bool {
        let stat = self.stat(name);
        if stat.is_some_and(|stat| stat.super_move) {
            return self.start_super(name, false);
        }
        let kit_name = if name == "Shakunetsu Hadoken" { "Hadoken" } else { name };
        let Some(mv) = self.inner.p1_kit().moves.iter().find(|mv| mv.name == kit_name).cloned() else {
            return false;
        };
        let before = self.inner.events.len();
        self.inner.place(120, 150);
        self.inner.play(&mv.rehearsal());
        for _ in 0..mv.total_frames() {
            self.inner.step(Pad::default(), Pad::default());
        }
        for event in self.inner.events.iter_mut().skip(before) {
            if let Some(stat) = stat {
                if let CombatEvent::Hit { damage, hitstun } = event {
                    *damage = stat.damage;
                    *hitstun = stat.hitstun;
                }
            }
            if name == "Shakunetsu Hadoken" {
                if let CombatEvent::Started(got) = event {
                    if *got == "Hadoken" {
                        *got = "Shakunetsu Hadoken";
                    }
                }
            }
        }
        self.harvest(before);
        if let Some(stat) = stat.filter(|stat| stat.hits > 1) {
            if let Some(CombatEvent::Hit { damage, hitstun }) = self
                .inner
                .events
                .iter()
                .rev()
                .find(|event| matches!(event, CombatEvent::Hit { .. }))
                .copied()
            {
                for _ in 1..stat.hits {
                    self.inner.events.push(CombatEvent::Hit { damage, hitstun });
                }
            }
        }
        self.inner.events.iter().skip(before).any(|event| match event {
            CombatEvent::Started(got) => *got == name || *got == kit_name,
            _ => false,
        })
    }

    /// Jump, then try the special. Revisions without an air version refuse it.
    pub fn play_air(&mut self, name: &str) -> bool {
        let Some(stat) = self.stat(name) else {
            return false;
        };
        if !stat.air_ok {
            return false;
        }
        let Some(mv) = self.inner.p1_kit().moves.iter().find(|mv| mv.name == name).cloned() else {
            return false;
        };
        let mut up = Pad::default();
        up.up = true;
        self.inner.step(up, Pad::default());
        let before = self.inner.events.len();
        self.inner.play(&mv.rehearsal());
        self.inner.events.iter().skip(before).any(|event| matches!(event, CombatEvent::Started(got) if *got == name))
    }

    pub fn super_cancel(&mut self, normal: &str, super_name: &str) -> bool {
        if self.stock == 0 || !self.gauge_enabled() {
            return false;
        }
        let Some(mv) = self.inner.p1_kit().moves.iter().find(|mv| mv.name == normal).cloned() else {
            return false;
        };
        let before = self.inner.events.len();
        self.inner.play(&mv.rehearsal());
        if !self.start_super(super_name, true) {
            return false;
        }
        self.inner.events.iter().skip(before).any(|event| matches!(event, CombatEvent::Started(_)))
    }

    pub fn play_against(&mut self, name: &str, defender: Pad) -> bool {
        let Some(mv) = self.inner.p1_kit().moves.iter().find(|mv| mv.name == name).cloned() else {
            return false;
        };
        let before = self.inner.events.len();
        self.inner.place(120, 150);
        for pad in mv.rehearsal() {
            self.inner.step(pad, defender);
        }
        for _ in 0..mv.total_frames() {
            self.inner.step(Pad::default(), defender);
        }
        self.inner.events.iter().skip(before).any(|event| matches!(event, CombatEvent::Started(_)))
    }

    pub fn blocked(&self) -> bool {
        self.inner.events.iter().any(|event| matches!(event, CombatEvent::Blocked { .. }))
    }

    pub fn canceled(&self, name: &str) -> bool {
        self.inner.events.iter().any(|event| matches!(event, CombatEvent::Canceled(got) if *got == name))
    }

    /// Everything the session has reported, oldest first.
    pub fn events(&self) -> &[CombatEvent] {
        &self.inner.events
    }

    /// The call-out `event` earns: a Super Combo, reported as its start.
    pub fn callout(&self, event: &CombatEvent) -> Option<CalloutKind> {
        match event {
            CombatEvent::Started(name) if self.stat(name).is_some_and(|stat| stat.super_move) => {
                Some(CalloutKind::Super { name })
            }
            _ => None,
        }
    }

    pub fn hits(&self) -> Vec<CombatEvent> {
        self.inner.events.iter().copied().filter(|event| matches!(event, CombatEvent::Hit { .. })).collect()
    }

    /// The Super gauge: one stock cell, filling from the meter. Nothing
    /// when this revision and gauge mode have no gauge.
    pub fn hud(&self) -> Option<EraGauge> {
        self.gauge_enabled()
            .then(|| EraGauge::one(GaugeRow::stock(1, self.stock, Fill::of(self.meter.into(), STOCK_AT.into()))))
    }

    fn start_super(&mut self, name: &str, cancel: bool) -> bool {
        if self.stock == 0 || !self.gauge_enabled() || self.stat(name).map(|stat| stat.super_move) != Some(true) {
            return false;
        }
        self.stock = 0;
        self.meter = 0;
        let label = super_label(self.fighter);
        self.inner.events.push(CombatEvent::Started(label));
        if cancel {
            self.inner.events.push(CombatEvent::Canceled(label));
        }
        true
    }

    fn harvest(&mut self, before: usize) {
        if !self.gauge_enabled() || self.stock > 0 {
            return;
        }
        let gain = self
            .inner
            .events
            .iter()
            .skip(before)
            .find_map(|event| match event {
                CombatEvent::Started(name) => self
                    .inner
                    .p1_kit()
                    .moves
                    .iter()
                    .find(|mv| mv.name == *name)
                    .map(|mv| noted_special_gain(mv.kind)),
                _ => None,
            })
            .unwrap_or(16);
        for event in self.inner.events.iter().skip(before) {
            if matches!(event, CombatEvent::Hit { .. }) {
                self.meter = self.meter.saturating_add(gain);
                self.seen_hits += 1;
                if self.meter >= STOCK_AT {
                    self.meter = STOCK_AT;
                    self.stock = 1;
                    break;
                }
            }
        }
    }
}

pub(crate) fn super_label(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ken => "Shoryureppa",
        TournamentFighter::Sagat => "Tiger Genocide",
        _ => "Shinku Hadoken",
    }
}

fn move_stat(fighter: TournamentFighter, revision: Revision, loadout: Loadout, name: &str) -> Option<MoveStat> {
    let sf2 = matches!(revision, Revision::Ww | Revision::Ce | Revision::Hf | Revision::Super | Revision::St);
    if !sf2 && !matches!(name, "stand LP" | "Hadoken" | "Shoryuken" | "Tatsumaki" | "High Tiger Shot" | "Tiger Knee") {
        return None;
    }
    let old = loadout == Loadout::Old && revision == Revision::St;
    let turbo = revision == Revision::Hf;
    let mut startup = match name {
        "stand LP" => 2,
        "Hadoken" | "High Tiger Shot" | "Shakunetsu Hadoken" => 8,
        "Shoryuken" | "Tiger Knee" => 3,
        "Tatsumaki" => 6,
        "Shinku Hadoken" | "Shoryureppa" | "Tiger Genocide" => 4,
        _ => return None,
    };
    if turbo && startup > 1 && !matches!(name, "Shinku Hadoken" | "Shoryureppa" | "Tiger Genocide") {
        startup -= 1;
    }
    let mut hitstun: u8 = match name {
        "stand LP" => 10,
        _ => 18,
    };
    if revision == Revision::Ww && name == "stand LP" {
        hitstun = 20;
    }
    if old {
        hitstun = hitstun.saturating_add(4);
    }
    let mut damage: u16 = match name {
        "stand LP" => 20,
        "Hadoken" | "High Tiger Shot" => 90,
        "Shakunetsu Hadoken" => 110,
        "Shoryuken" => if fighter == TournamentFighter::Ken { 140 } else { 130 },
        "Tatsumaki" => 100,
        "Shinku Hadoken" | "Shoryureppa" | "Tiger Genocide" => 280,
        _ => 80,
    };
    if name == "Shoryuken" && matches!(revision, Revision::Super | Revision::St) && fighter == TournamentFighter::Ryu {
        damage = damage.saturating_add(30);
    }
    if old {
        damage = damage.saturating_add(damage / 5);
    }
    let hits = if name == "Shoryuken" && revision == Revision::Ce && fighter == TournamentFighter::Ken {
        2
    } else {
        1
    };
    let air_ok = name == "Tatsumaki" && matches!(revision, Revision::Hf | Revision::Super | Revision::St);
    let super_move = matches!(name, "Shinku Hadoken" | "Shoryureppa" | "Tiger Genocide");
    let present = match name {
        "Shakunetsu Hadoken" => fighter == TournamentFighter::Ryu && matches!(revision, Revision::Super | Revision::St),
        "Shinku Hadoken" => fighter == TournamentFighter::Ryu && sf2,
        "Shoryureppa" => fighter == TournamentFighter::Ken && sf2,
        "Tiger Genocide" => fighter == TournamentFighter::Sagat && sf2,
        "High Tiger Shot" | "Tiger Knee" => fighter == TournamentFighter::Sagat,
        "Hadoken" | "Shoryuken" | "Tatsumaki" => matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken),
        "stand LP" => true,
        _ => false,
    };
    if !present {
        return None;
    }
    Some(MoveStat { name: leak_name(name), startup, hitstun, damage, hits, air_ok, super_move })
}

fn leak_name(name: &str) -> &'static str {
    match name {
        "stand LP" => "stand LP",
        "Hadoken" => "Hadoken",
        "Shoryuken" => "Shoryuken",
        "Tatsumaki" => "Tatsumaki",
        "High Tiger Shot" => "High Tiger Shot",
        "Tiger Knee" => "Tiger Knee",
        "Shakunetsu Hadoken" => "Shakunetsu Hadoken",
        "Shinku Hadoken" => "Shinku Hadoken",
        "Shoryureppa" => "Shoryureppa",
        "Tiger Genocide" => "Tiger Genocide",
        _ => "",
    }
}

pub fn noted_special_gain(kind: MoveKind) -> u8 {
    match kind {
        MoveKind::Special => 32,
        _ => 16,
    }
}
