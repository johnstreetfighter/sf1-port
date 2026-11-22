//! Era family, SF2 revision, costume, and the Super gauge (`#103`, `O12`).
//!
//! Character select offers only the families and revisions a fighter appeared in.
//! Ryu, Ken, and Sagat carry the five Street Fighter II revisions. The Super
//! Turbo stock fills from hits and specials, and a full stock is what lets a
//! Super Combo or a Super cancel come out.

use crate::roster::TournamentFighter;
use crate::config::RulesetPreset;

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
    /// `SF KESSEN`, the first original style (ADR-0027): no Street Fighter
    /// entry stands behind it.
    Kessen,
}

crate::canonical::canonical_enum!(EraFamily, "EraFamily.v1";
    Sf1987 => 0, Sf1Plus => 1, Sf2 => 2, SfZ => 3, Sf3 => 4, Sf4 => 5, Sf5 => 6, Sf6 => 7,
    Kessen => 8,
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
            EraFamily::Kessen => "SF KESSEN",
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
    Kessen,
}

crate::canonical::canonical_enum!(Revision, "EraRevision.v1";
    Arcade1987 => 0, Sf1Plus => 1, Ww => 2, Ce => 3, Hf => 4, Super => 5,
    St => 6, Sfz1 => 7, Sfz2 => 8, Sfz2A => 9, Sfz3 => 10, Sf3Ng => 11,
    Sf3TwoI => 12, Sf3ThreeS => 13, Sf4 => 14, Ssf4 => 15, Ae2012 => 16,
    Usf4 => 17, Sf5 => 18, Sf5Ae => 19, Sf5Ce => 20, Sf6 => 21, Kessen => 22,
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
            Revision::Kessen => "SF KESSEN",
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
            Revision::Kessen => EraFamily::Kessen,
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
    /// `SF KESSEN`: two Super stocks and a middling ration of Versatile stocks.
    Balanced,
    /// `SF KESSEN`: one slow Super stock and the most Versatile stocks.
    Versatile,
    /// `SF KESSEN`: three Super stocks and the fewest Versatile stocks.
    SuperHeavy,
}

crate::canonical::canonical_enum!(Loadout, "EraLoadout.v1";
    Standard => 0, Old => 1, AIsm => 2, VIsm => 3, XIsm => 4,
    Art1 => 5, Art2 => 6, Art3 => 7, Ultra1 => 8, Ultra2 => 9, UltraDouble => 10,
    Skill1Trigger1 => 11, Skill1Trigger2 => 12, Skill2Trigger1 => 13, Skill2Trigger2 => 14,
    Balanced => 15, Versatile => 16, SuperHeavy => 17,
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
            Loadout::Balanced => "Balanced",
            Loadout::Versatile => "Versatile",
            Loadout::SuperHeavy => "Super",
        }
    }

    /// An `SF KESSEN` loadout's ration, or `None` for any other loadout.
    pub const fn kessen_ration(self) -> Option<KessenRation> {
        Some(match self {
            Loadout::Balanced => KessenRation { super_stocks: 2, opening: 3, cap: 5, round_two: 2, round_three_on: 1 },
            Loadout::Versatile => KessenRation { super_stocks: 1, opening: 4, cap: 6, round_two: 2, round_three_on: 2 },
            Loadout::SuperHeavy => KessenRation { super_stocks: 3, opening: 3, cap: 3, round_two: 1, round_three_on: 1 },
            _ => return None,
        })
    }
}

/// What an `SF KESSEN` loadout grants: its Super stocks, and its Versatile
/// stocks by round. Nothing during a round adds a Versatile stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KessenRation {
    /// Super stocks the Super meter holds.
    pub super_stocks: u8,
    /// Versatile stocks held in round 1.
    pub opening: u8,
    /// The most Versatile stocks he can hold.
    pub cap: u8,
    /// Versatile stocks gained at the start of round 2.
    pub round_two: u8,
    /// Versatile stocks gained at the start of round 3 and of every round after it.
    pub round_three_on: u8,
}

impl KessenRation {
    /// The Versatile stocks held at the start of `round` (1 for the first),
    /// having ended the round before with `held`.
    pub const fn at_round(self, round: u8, held: u8) -> u8 {
        let granted = match round {
            ..=1 => return self.opening,
            2 => held.saturating_add(self.round_two),
            _ => held.saturating_add(self.round_three_on),
        };
        if granted > self.cap { self.cap } else { granted }
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

    pub fn for_revision(revision: Revision, loadout: Loadout) -> Self {
        Self { family: revision.family(), revision, loadout, ..Self::default() }
    }

    /// The style whose moves this pick plays. `SF KESSEN` is built on the
    /// fighter's Super Turbo moves (ADR-0027); every other pick plays its own.
    pub fn moveset(self) -> Self {
        match self.family {
            EraFamily::Kessen => Self {
                family: EraFamily::Sf2,
                revision: Revision::St,
                loadout: Loadout::Standard,
                ..self
            },
            _ => self,
        }
    }

    pub(crate) fn for_preset(preset: RulesetPreset) -> Self {
        match preset {
            RulesetPreset::Faithful1987 => Self::for_revision(Revision::Arcade1987, Loadout::Standard),
            _ => Self::default(),
        }
    }

    /// Resolve the style for a zero-based Arcade ladder step. A CPU
    /// opponent is not given `SF KESSEN` until every kit exists.
    pub fn arcade_opponent(self, fighter: TournamentFighter, what_if: bool, step: usize) -> Self {
        if self.family == EraFamily::Kessen {
            return Self::default();
        }
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

/// Whether `fighter`'s `SF KESSEN` kit is built. The style is offered for
/// a fighter only once it is, with or without what-if style options.
pub fn kessen_kit_built(fighter: TournamentFighter) -> bool {
    matches!(fighter, TournamentFighter::Ryu)
}

/// Families the select screen shows. What-if families stay off the canonical
/// list, and so does `SF KESSEN`: an original style is shown for a fighter
/// whose kit is built whether or not what-if styles are on.
pub fn listed_families(fighter: TournamentFighter, what_if: bool) -> Vec<EraFamily> {
    let mut out = families(fighter);
    if kessen_kit_built(fighter) {
        out.push(EraFamily::Kessen);
    }
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
    if family == EraFamily::Kessen {
        return revisions(fighter, family);
    }
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
            TournamentFighter::Gen => vec![Revision::Sfz2, Revision::Sfz2A, Revision::Sfz3],
            TournamentFighter::Ryu
            | TournamentFighter::Ken
            | TournamentFighter::Adon
            | TournamentFighter::Sagat
            | TournamentFighter::Birdie => {
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
            TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Sagat | TournamentFighter::Gen => {
                vec![Revision::Sf4, Revision::Ssf4, Revision::Ae2012, Revision::Usf4]
            }
            TournamentFighter::Adon => {
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
        EraFamily::Kessen => {
            if kessen_kit_built(fighter) {
                vec![Revision::Kessen]
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
        Revision::Kessen => vec![Loadout::Balanced, Loadout::Versatile, Loadout::SuperHeavy],
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

pub(crate) const STOCK_AT: u8 = 48;

pub(crate) fn super_label(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ken => "Shoryureppa",
        TournamentFighter::Sagat => "Tiger Genocide",
        _ => "Shinku Hadoken",
    }
}
