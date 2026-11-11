//! Playable 12-fighter SF1 (*SF1++*) roster, character select, and ban lists (`#98`, `O7`).
//!
//! Overhaul presets unlock every 1987 tournament fighter for Player 1 and Player 2.
//! Ryu and Ken keep the human input handler on their own move tables. The ten
//! formerly CPU-only fighters keep their recovered 1987 move slots and attack
//! streams, started from the controller instead of the AI decision tree.
//! `Faithful 1987` leaves `full_cast` and `four_country` off, so the original
//! start and country flow is unchanged.

use crate::config::RulesetPreset;
use crate::era::{
    costumes, families, listed_families, listed_revisions, loadouts, EraFamily, StylePick,
    PALETTES,
};

/// One of the twelve 1987 tournament fighters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TournamentFighter {
    Ryu,
    Ken,
    Retsu,
    Geki,
    Joe,
    Mike,
    Lee,
    Gen,
    Birdie,
    Eagle,
    Adon,
    Sagat,
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(RosterSettings, "RosterSettings.v1", {
    full_cast, four_country, what_if_styles, bonus_challengers, secret_intrusions, ban, mode, p1, p2, country, stage,
});

crate::canonical::canonical_enum!(TournamentFighter, "TournamentFighter.v1";
    Ryu => 0,
    Ken => 1,
    Retsu => 2,
    Geki => 3,
    Joe => 4,
    Mike => 5,
    Lee => 6,
    Gen => 7,
    Birdie => 8,
    Eagle => 9,
    Adon => 10,
    Sagat => 11,
);

crate::canonical::canonical_enum!(PlayMode, "PlayMode.v1";
    Arcade => 0,
    Versus => 1,
    Training => 2,
    Cpu => 3,
);

crate::canonical::canonical_enum!(StartingCountry, "StartingCountry.v1";
    Japan => 0,
    China => 1,
    Usa => 2,
    England => 3,
);

crate::canonical::canonical_enum!(BanFormat, "BanFormat.v1";
    StandardTournament => 0,
    LowTierNoShoto => 1,
    Unrestricted => 2,
);

crate::canonical::canonical_enum!(StagePick, "StagePick.v1";
    Bank(value0) => 0,
);


impl TournamentFighter {
    pub const ALL: [TournamentFighter; 12] = [
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        TournamentFighter::Retsu,
        TournamentFighter::Geki,
        TournamentFighter::Joe,
        TournamentFighter::Mike,
        TournamentFighter::Lee,
        TournamentFighter::Gen,
        TournamentFighter::Birdie,
        TournamentFighter::Eagle,
        TournamentFighter::Adon,
        TournamentFighter::Sagat,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            TournamentFighter::Ryu => "Ryu",
            TournamentFighter::Ken => "Ken",
            TournamentFighter::Retsu => "Retsu",
            TournamentFighter::Geki => "Geki",
            TournamentFighter::Joe => "Joe",
            TournamentFighter::Mike => "Mike",
            TournamentFighter::Lee => "Lee",
            TournamentFighter::Gen => "Gen",
            TournamentFighter::Birdie => "Birdie",
            TournamentFighter::Eagle => "Eagle",
            TournamentFighter::Adon => "Adon",
            TournamentFighter::Sagat => "Sagat",
        }
    }

    pub fn from_name(s: &str) -> Result<Self, String> {
        for fighter in Self::ALL {
            if fighter.name().eq_ignore_ascii_case(s.trim()) {
                return Ok(fighter);
            }
        }
        Err(format!("unknown tournament fighter '{s}'"))
    }

    /// ROM move-table pointer (`player structure +0x12`).
    pub const fn move_table(self) -> u32 {
        match self {
            TournamentFighter::Ryu => 0x1605c,
            TournamentFighter::Ken => 0x1662c,
            TournamentFighter::Geki => 0x16bfc,
            TournamentFighter::Retsu => 0x16e6c,
            TournamentFighter::Mike => 0x170dc,
            TournamentFighter::Joe => 0x1734c,
            TournamentFighter::Lee => 0x175bc,
            TournamentFighter::Gen => 0x1782c,
            TournamentFighter::Birdie => 0x17a9c,
            TournamentFighter::Eagle => 0x17d0c,
            TournamentFighter::Adon => 0x17f7c,
            TournamentFighter::Sagat => 0x181ec,
        }
    }

    /// The fighter a move table belongs to: his ROM table, or his 1987
    /// player table in the extended data bank (ADR-0014).
    pub fn from_move_table(table: u32) -> Option<Self> {
        crate::data_bank::fighter_of_table(table).or_else(|| Self::ALL.into_iter().find(|f| f.move_table() == table))
    }

    /// The ROM sprite palette (0 to 15) his frame records draw him in. Two
    /// more, 2 and 4, are the shared fireball palettes Ryu's and Ken's
    /// Hadokens flicker through, Geki's flash and Sagat's Tiger Shot use 4,
    /// and 13 is every hit spark's.
    pub const fn sprite_palette(self) -> u8 {
        match self {
            TournamentFighter::Ryu => 0,
            TournamentFighter::Ken => 1,
            TournamentFighter::Geki => 2,
            TournamentFighter::Retsu => 3,
            TournamentFighter::Joe => 5,
            TournamentFighter::Lee => 6,
            TournamentFighter::Mike => 7,
            TournamentFighter::Gen => 8,
            TournamentFighter::Birdie => 9,
            TournamentFighter::Eagle => 10,
            TournamentFighter::Adon => 11,
            TournamentFighter::Sagat => 12,
        }
    }

    /// A boss's hit and knock-out tables: the ones the 1987 game's
    /// opponent arrays (`0x3e38a`, arrays 13 and 14) give him as the CPU,
    /// the same from every country. Ryu and Ken have the human players'
    /// tables instead.
    pub const fn reaction_tables(self) -> Option<(u32, u32)> {
        match self {
            TournamentFighter::Ryu | TournamentFighter::Ken => None,
            TournamentFighter::Geki => Some((0x19548, 0x1957c)),
            TournamentFighter::Retsu => Some((0x195a8, 0x195dc)),
            TournamentFighter::Mike => Some((0x19608, 0x1963c)),
            TournamentFighter::Joe => Some((0x19668, 0x1969c)),
            TournamentFighter::Lee => Some((0x196c8, 0x196fc)),
            TournamentFighter::Gen => Some((0x19728, 0x1975c)),
            TournamentFighter::Birdie => Some((0x19788, 0x197bc)),
            TournamentFighter::Eagle => Some((0x197e8, 0x1981c)),
            TournamentFighter::Adon => Some((0x19848, 0x1987c)),
            TournamentFighter::Sagat => Some((0x198a8, 0x198dc)),
        }
    }

    /// Ryu and Ken are driven by the original human input handler.
    pub const fn is_shoto(self) -> bool {
        matches!(self, TournamentFighter::Ryu | TournamentFighter::Ken)
    }
}

/// Where the roster select is being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    Arcade,
    Versus,
    Training,
    /// CPU versus CPU exhibition.
    Cpu,
}

impl PlayMode {
    pub const fn slug(self) -> &'static str {
        match self {
            PlayMode::Arcade => "arcade",
            PlayMode::Versus => "versus",
            PlayMode::Training => "training",
            PlayMode::Cpu => "cpu",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            PlayMode::Arcade => "Arcade",
            PlayMode::Versus => "Versus",
            PlayMode::Training => "Training",
            PlayMode::Cpu => "CPU vs CPU",
        }
    }

    pub fn from_name(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "arcade" => Ok(PlayMode::Arcade),
            "versus" => Ok(PlayMode::Versus),
            "training" => Ok(PlayMode::Training),
            "cpu" => Ok(PlayMode::Cpu),
            other => Err(format!("unknown play mode '{other}'")),
        }
    }
}

/// Starting country byte written to `0xffc006`.
///
/// `0` and `2` are the original two-country pair (Japan, USA). `1` and `3`
/// are the four-country additions (the China and Union Jack flags of the
/// four-country select): China meets Lee and Gen first, England meets Birdie
/// and Eagle first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartingCountry {
    Japan = 0,
    China = 1,
    Usa = 2,
    England = 3,
}

impl StartingCountry {
    pub const ALL: [StartingCountry; 4] = [
        StartingCountry::Japan,
        StartingCountry::Usa,
        StartingCountry::China,
        StartingCountry::England,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            StartingCountry::Japan => "Japan",
            StartingCountry::Usa => "USA",
            StartingCountry::China => "China",
            StartingCountry::England => "England",
        }
    }

    pub const fn byte(self) -> u8 {
        self as u8
    }

    pub fn from_name(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "japan" => Ok(StartingCountry::Japan),
            "usa" | "u.s.a." | "us" => Ok(StartingCountry::Usa),
            "china" => Ok(StartingCountry::China),
            "england" | "uk" => Ok(StartingCountry::England),
            other => Err(format!("unknown starting country '{other}'")),
        }
    }
}

/// `Classic Competitive` sub-format ban list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BanFormat {
    /// Sagat is banned. Player 1 Ryu is banned only while shotos are unpatched
    /// (`balanced_roster` is false).
    StandardTournament,
    /// Ryu and Ken are banned for both players.
    LowTierNoShoto,
    /// Every tournament fighter is legal.
    Unrestricted,
}

impl BanFormat {
    pub const ALL: [BanFormat; 3] = [
        BanFormat::StandardTournament,
        BanFormat::LowTierNoShoto,
        BanFormat::Unrestricted,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            BanFormat::StandardTournament => "standard_tournament",
            BanFormat::LowTierNoShoto => "low_tier_no_shoto",
            BanFormat::Unrestricted => "unrestricted",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            BanFormat::StandardTournament => "Standard Tournament",
            BanFormat::LowTierNoShoto => "Low-Tier / No-Shoto",
            BanFormat::Unrestricted => "Unrestricted",
        }
    }

    pub fn from_name(s: &str) -> Result<Self, String> {
        for format in Self::ALL {
            if format.slug().eq_ignore_ascii_case(s.trim())
                || format.name().eq_ignore_ascii_case(s.trim())
            {
                return Ok(format);
            }
        }
        Err(format!("unknown ban format '{s}'"))
    }

    /// Whether `fighter` may be chosen for this side.
    ///
    /// `side` is `0` for Player 1 and `1` for Player 2. `shotos_patched` is
    /// the `balanced_roster` toggle.
    pub const fn allows(self, fighter: TournamentFighter, side: u8, shotos_patched: bool) -> bool {
        match self {
            BanFormat::Unrestricted => true,
            BanFormat::LowTierNoShoto => !fighter.is_shoto(),
            BanFormat::StandardTournament => {
                if matches!(fighter, TournamentFighter::Sagat) {
                    return false;
                }
                if matches!(fighter, TournamentFighter::Ryu) && side == 0 && !shotos_patched {
                    return false;
                }
                true
            }
        }
    }
}

/// The stage a Local or Training fight is fought on (spec #192), as the
/// roster carries it to the stage gates (ADR-0004, ADR-0009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagePick {
    /// A stage in the extended background bank, by its place in the bank's
    /// stage list (`BackgroundBank::stage_index`).
    Bank(u8),
}

/// Session roster choices carried on [`crate::config::MatchConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosterSettings {
    /// Full 12-fighter select and human binding for the ten bosses.
    pub full_cast: bool,
    /// Force the four-country start (Japan, USA, China, England).
    pub four_country: bool,
    /// What-if era styles. On for Definitive Overhaul, off for the 1987 presets.
    pub what_if_styles: bool,
    /// Bonus challengers. On for Definitive Overhaul.
    pub bonus_challengers: bool,
    /// Secret arcade rival intrusions. On for Definitive Overhaul.
    pub secret_intrusions: bool,
    pub ban: BanFormat,
    pub mode: PlayMode,
    pub p1: TournamentFighter,
    pub p2: TournamentFighter,
    pub country: StartingCountry,
    /// The picked stage, read by the overhaul-only stage gates in Local and
    /// Training; `None` (always under Faithful 1987, in Arcade and in the
    /// attract) reads the 1987 country tables. There is no stage select
    /// yet: the session's stage hook sets it (tests and `sf-game --stage`).
    /// Not saved with the configuration.
    pub stage: Option<StagePick>,
}

impl RosterSettings {
    pub const fn for_preset(preset: RulesetPreset) -> Self {
        match preset {
            RulesetPreset::Faithful1987 => Self {
                full_cast: false,
                four_country: false,
                what_if_styles: false,
                bonus_challengers: false,
                secret_intrusions: false,
                ban: BanFormat::Unrestricted,
                mode: PlayMode::Arcade,
                p1: TournamentFighter::Ryu,
                p2: TournamentFighter::Ken,
                country: StartingCountry::Japan,
                stage: None,
            },
            // No ban list by default (the maintainer, 2026-11-08): all twelve are
            // pickable on both sides. Standard Tournament stays a format a
            // player can choose, with its own rule (SP1, `#247`).
            RulesetPreset::ClassicCompetitive => Self {
                full_cast: true,
                four_country: true,
                what_if_styles: false,
                bonus_challengers: false,
                secret_intrusions: false,
                ban: BanFormat::Unrestricted,
                mode: PlayMode::Arcade,
                p1: TournamentFighter::Ryu,
                p2: TournamentFighter::Ken,
                country: StartingCountry::Japan,
                stage: None,
            },
            RulesetPreset::DefinitiveOverhaul => Self {
                full_cast: true,
                four_country: true,
                what_if_styles: true,
                bonus_challengers: true,
                secret_intrusions: true,
                ban: BanFormat::Unrestricted,
                mode: PlayMode::Arcade,
                p1: TournamentFighter::Ryu,
                p2: TournamentFighter::Ken,
                country: StartingCountry::Japan,
                stage: None,
            },
            RulesetPreset::PureFootsies1987Plus | RulesetPreset::Custom => Self {
                full_cast: true,
                four_country: true,
                what_if_styles: false,
                bonus_challengers: false,
                secret_intrusions: false,
                ban: BanFormat::Unrestricted,
                mode: PlayMode::Arcade,
                p1: TournamentFighter::Ryu,
                p2: TournamentFighter::Ken,
                country: StartingCountry::Japan,
                stage: None,
            },
        }
    }

    pub const fn faithful() -> Self {
        Self::for_preset(RulesetPreset::Faithful1987)
    }
}

/// Full-cast character select shared by Arcade, Versus, and Training.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSelectScreen {
    pub mode: PlayMode,
    pub ban: BanFormat,
    pub shotos_patched: bool,
    pub four_country: bool,
    pub what_if_styles: bool,
    pub bonus_challengers: bool,
    pub secret_intrusions: bool,
    show_extra: bool,
    phase: SelectPhase,
    cursor: usize,
    pub p1: Option<TournamentFighter>,
    pub p2: Option<TournamentFighter>,
    country_cursor: usize,
    pub country: Option<StartingCountry>,
    p1_style: StylePick,
    p2_style: StylePick,
    style_slot: u8,
    family_at: usize,
    revision_at: usize,
    loadout_at: usize,
    costume_at: usize,
    palette_at: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectPhase {
    P1,
    P2,
    Country,
    EraFamily,
    Revision,
    Costume,
    Ready,
}

/// Discrete menu inputs. The caller passes edges, not held keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuInput {
    Left,
    Right,
    Up,
    Down,
    Confirm,
    Back,
    /// One button. Switches the roster view and the style view when those toggles are on.
    Filter,
    /// The front end's Start (keyboard Start, a pad's Options, either cabinet start).
    Start,
    /// The front end's tab-switch pair (Q / E, L1 / R1).
    TabLeft,
    TabRight,
}

impl CharacterSelectScreen {
    pub fn open(settings: RosterSettings, shotos_patched: bool) -> Self {
        Self {
            mode: settings.mode,
            ban: settings.ban,
            shotos_patched,
            four_country: settings.four_country,
            what_if_styles: settings.what_if_styles,
            bonus_challengers: settings.bonus_challengers,
            secret_intrusions: settings.secret_intrusions,
            show_extra: false,
            phase: SelectPhase::P1,
            cursor: 0,
            p1: None,
            p2: None,
            country_cursor: 0,
            country: None,
            p1_style: StylePick::default(),
            p2_style: StylePick::default(),
            style_slot: 0,
            family_at: 1,
            revision_at: 0,
            loadout_at: 0,
            costume_at: 0,
            palette_at: 0,
        }
    }

    pub fn p1_style(&self) -> StylePick {
        self.p1_style
    }

    pub fn p2_style(&self) -> StylePick {
        self.p2_style
    }

    pub fn is_ready(&self) -> bool {
        self.phase == SelectPhase::Ready
    }

    /// Index into the twelve-fighter grid. Banned fighters stay on the grid and are skipped by the cursor.
    pub fn portrait_slot(&self) -> usize {
        let legal = self.legal();
        if legal.is_empty() {
            return 0;
        }
        let fighter = legal[self.cursor % legal.len()];
        TournamentFighter::ALL
            .iter()
            .position(|one| *one == fighter)
            .unwrap_or(0)
    }

    pub fn portrait_caption(&self) -> String {
        let legal = self.legal();
        if legal.is_empty() {
            return String::new();
        }
        legal[self.cursor % legal.len()].name().to_string()
    }

    /// The portrait select frame. Tiles without a bust show the fighter's name.
    pub fn portrait_frame(&self) -> sf_video::render::Frame {
        let names = std::array::from_fn(|i| TournamentFighter::ALL[i].name());
        let empty = [None, None, None, None, None, None, None, None, None, None, None, None];
        sf_video::compositor::render_player_select(&names, self.portrait_slot(), &self.portrait_caption(), &empty)
    }

    pub fn phase_name(&self) -> &'static str {
        match self.phase {
            SelectPhase::P1 => "Player 1",
            SelectPhase::P2 => "Player 2",
            SelectPhase::Country => "Country",
            SelectPhase::EraFamily => "Era family",
            SelectPhase::Revision => "Revision",
            SelectPhase::Costume => "Costume",
            SelectPhase::Ready => "Ready",
        }
    }

    fn side(&self) -> u8 {
        if self.phase == SelectPhase::P2 {
            1
        } else {
            0
        }
    }

    fn legal(&self) -> Vec<TournamentFighter> {
        TournamentFighter::ALL
            .into_iter()
            .filter(|f| self.ban.allows(*f, self.side(), self.shotos_patched))
            .collect()
    }

    /// Visible rows of the select screen, including every tournament fighter.
    pub fn rows(&self) -> Vec<String> {
        let mut rows = vec![
            format!("SF1++ CHARACTER SELECT  {}", self.mode.name()),
            format!("SUB-FORMAT  {}", self.ban.name()),
            format!("CURSOR  {}", self.phase_name()),
            format!(
                "ROSTER  {}",
                if self.show_extra && self.bonus_challengers {
                    "Expanded 25 Roster"
                } else {
                    "1987 Tournament 12"
                }
            ),
            format!(
                "STYLES  {}",
                if self.show_extra && self.what_if_styles {
                    "All Styles (Incl. What-If)"
                } else {
                    "Canonical Styles Only"
                }
            ),
        ];
        if self.show_extra && self.bonus_challengers {
            rows.push(crate::bonus::roster_row());
        }
        let legal = self.legal();
        let current = legal.get(self.cursor).copied();
        for (i, fighter) in TournamentFighter::ALL.into_iter().enumerate() {
            let allowed = self.ban.allows(fighter, self.side(), self.shotos_patched);
            let mark = if Some(fighter) == current {
                ">"
            } else if !allowed {
                "x"
            } else {
                " "
            };
            let note = if allowed { "" } else { "  BANNED" };
            rows.push(format!("{mark} {i:2} {}{note}", fighter.name()));
        }
        if self.four_country || self.phase == SelectPhase::Country {
            rows.push("COUNTRY  Japan  USA  China  England".to_string());
            if self.phase == SelectPhase::Country {
                rows.push(format!(
                    "COUNTRY CURSOR  {}",
                    StartingCountry::ALL[self.country_cursor].name()
                ));
            }
        }
        if let Some(p1) = self.p1 {
            rows.push(format!("P1  {}", p1.name()));
        }
        if let Some(p2) = self.p2 {
            rows.push(format!("P2  {}", p2.name()));
        }
        if let Some(country) = self.country {
            rows.push(format!("START  {}", country.name()));
        }
        if matches!(self.phase, SelectPhase::EraFamily | SelectPhase::Revision | SelectPhase::Costume) {
            if let Some(fighter) = self.style_fighter() {
                let fams = self.style_families(fighter);
                let canonical = families(fighter);
                rows.push(format!(
                    "ERA FAMILY  {}",
                    fams.iter()
                        .map(|family| {
                            if canonical.contains(family) {
                                family.name().to_string()
                            } else {
                                format!("{} [WHAT-IF]", family.name())
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("  ")
                ));
                if let Some(family) = fams.get(self.family_at) {
                    rows.push(format!("ERA CURSOR  {}", family.name()));
                    let revs = self.style_revisions(fighter, *family);
                    rows.push(format!(
                        "REVISION  {}",
                        revs.iter().map(|rev| rev.name()).collect::<Vec<_>>().join("  ")
                    ));
                    if let Some(rev) = revs.get(self.revision_at) {
                        rows.push(format!("REVISION CURSOR  {}", rev.name()));
                        rows.push(format!(
                            "LOADOUT  {}",
                            loadouts(*rev).iter().map(|one| one.name()).collect::<Vec<_>>().join("  ")
                        ));
                    }
                }
                rows.push(format!("PALETTE  {}", self.palette_at));
            }
        }
        rows.push(format!(
            "P1 STYLE  {}  {}  {}  {}",
            self.p1_style.revision.name(),
            self.p1_style.loadout.name(),
            self.p1_style.costume.name(),
            self.p1_style.palette
        ));
        rows
    }

    pub fn apply(&mut self, input: MenuInput) {
        if input == MenuInput::Filter {
            if self.what_if_styles || self.bonus_challengers {
                self.show_extra = !self.show_extra;
                self.write_style();
            }
            return;
        }
        if self.phase == SelectPhase::Ready && input != MenuInput::Back {
            return;
        }
        match self.phase {
            SelectPhase::P1 | SelectPhase::P2 => self.apply_fighter(input),
            SelectPhase::Country => self.apply_country(input),
            SelectPhase::EraFamily => self.apply_family(input),
            SelectPhase::Revision => self.apply_revision(input),
            SelectPhase::Costume => self.apply_costume(input),
            SelectPhase::Ready => {
                if input == MenuInput::Back {
                    self.reopen_last();
                }
            }
        }
    }

    fn step_fighter(&mut self, delta: isize) {
        let n = self.legal().len();
        if n == 0 {
            return;
        }
        self.cursor = (self.cursor as isize + delta).rem_euclid(n as isize) as usize;
    }

    fn step_column(&mut self, drow: i32) {
        let legal = self.legal();
        if legal.is_empty() {
            return;
        }
        let current = legal[self.cursor % legal.len()];
        let slot = TournamentFighter::ALL
            .iter()
            .position(|one| *one == current)
            .unwrap_or(0);
        let col = slot % 3;
        let mut row = (slot / 3) as i32;
        for _ in 0..4 {
            row = (row + drow).rem_euclid(4);
            let idx = row as usize * 3 + col;
            if let Some(pos) = legal.iter().position(|one| *one == TournamentFighter::ALL[idx]) {
                self.cursor = pos;
                return;
            }
        }
    }

    fn apply_fighter(&mut self, input: MenuInput) {
        let n = self.legal().len();
        if n == 0 {
            return;
        }
        self.cursor %= n;
        match input {
            MenuInput::Left => self.step_fighter(-1),
            MenuInput::Right => self.step_fighter(1),
            MenuInput::Up => self.step_column(-1),
            MenuInput::Down => self.step_column(1),
            MenuInput::Confirm => {
                let pick = self.legal()[self.cursor];
                match self.phase {
                    SelectPhase::P1 => {
                        self.p1 = Some(pick);
                        if self.mode == PlayMode::Arcade {
                            self.after_players();
                        } else {
                            self.phase = SelectPhase::P2;
                            self.cursor = 0;
                        }
                    }
                    SelectPhase::P2 => {
                        self.p2 = Some(pick);
                        self.after_players();
                    }
                    _ => {}
                }
            }
            MenuInput::Back => {
                if self.phase == SelectPhase::P2 {
                    self.phase = SelectPhase::P1;
                    self.p1 = None;
                    self.cursor = 0;
                }
            }
            MenuInput::Filter | MenuInput::Start | MenuInput::TabLeft | MenuInput::TabRight => {}
        }
    }

    fn after_players(&mut self) {
        if self.four_country {
            self.phase = SelectPhase::Country;
            self.country_cursor = 0;
        } else {
            self.country = Some(StartingCountry::Japan);
            self.begin_style(0);
        }
    }

    fn apply_country(&mut self, input: MenuInput) {
        let n = StartingCountry::ALL.len();
        match input {
            MenuInput::Left | MenuInput::Up => {
                self.country_cursor = (self.country_cursor + n - 1) % n;
            }
            MenuInput::Right | MenuInput::Down => {
                self.country_cursor = (self.country_cursor + 1) % n;
            }
            MenuInput::Confirm => {
                self.country = Some(StartingCountry::ALL[self.country_cursor]);
                self.begin_style(0);
            }
            MenuInput::Back => {
                self.country = None;
                if self.mode == PlayMode::Arcade {
                    self.phase = SelectPhase::P1;
                    self.p1 = None;
                } else {
                    self.phase = SelectPhase::P2;
                    self.p2 = None;
                }
                self.cursor = 0;
            }
            MenuInput::Filter | MenuInput::Start | MenuInput::TabLeft | MenuInput::TabRight => {}
        }
    }

    fn style_families(&self, fighter: TournamentFighter) -> Vec<EraFamily> {
        listed_families(fighter, self.show_extra && self.what_if_styles)
    }

    fn style_revisions(
        &self,
        fighter: TournamentFighter,
        family: EraFamily,
    ) -> Vec<crate::era::Revision> {
        listed_revisions(fighter, family, self.show_extra && self.what_if_styles)
    }

    fn reopen_last(&mut self) {
        self.phase = SelectPhase::Costume;
    }

    fn style_fighter(&self) -> Option<TournamentFighter> {
        if self.style_slot == 1 {
            self.p2
        } else {
            self.p1
        }
    }

    fn begin_style(&mut self, slot: u8) {
        self.style_slot = slot;
        self.family_at = 1;
        self.revision_at = 0;
        self.loadout_at = 0;
        self.costume_at = 0;
        self.palette_at = 0;
        self.write_style();
        self.phase = SelectPhase::EraFamily;
    }

    fn write_style(&mut self) {
        let Some(fighter) = self.style_fighter() else {
            return;
        };
        let fams = self.style_families(fighter);
        if fams.is_empty() {
            return;
        }
        self.family_at %= fams.len();
        let family = fams[self.family_at];
        let revs = self.style_revisions(fighter, family);
        if revs.is_empty() {
            return;
        }
        self.revision_at %= revs.len();
        let revision = revs[self.revision_at];
        let loads = loadouts(revision);
        self.loadout_at %= loads.len();
        let suits = costumes(fighter, family);
        self.costume_at %= suits.len();
        let pick = StylePick {
            family,
            revision,
            loadout: loads[self.loadout_at],
            costume: suits[self.costume_at],
            palette: self.palette_at % PALETTES,
        };
        if self.style_slot == 1 {
            self.p2_style = pick;
        } else {
            self.p1_style = pick;
        }
    }

    fn apply_family(&mut self, input: MenuInput) {
        let Some(fighter) = self.style_fighter() else {
            return;
        };
        let n = self.style_families(fighter).len();
        match input {
            MenuInput::Left | MenuInput::Up => self.family_at = (self.family_at + n - 1) % n,
            MenuInput::Right | MenuInput::Down => self.family_at = (self.family_at + 1) % n,
            MenuInput::Confirm => {
                self.revision_at = 0;
                self.loadout_at = 0;
                self.phase = SelectPhase::Revision;
            }
            MenuInput::Filter | MenuInput::Start | MenuInput::TabLeft | MenuInput::TabRight => {}
            MenuInput::Back => {
                if self.four_country {
                    self.phase = SelectPhase::Country;
                } else if self.style_slot == 1 {
                    self.phase = SelectPhase::P2;
                    self.p2 = None;
                } else {
                    self.phase = SelectPhase::P1;
                    self.p1 = None;
                }
            }
        }
        self.write_style();
    }

    fn apply_revision(&mut self, input: MenuInput) {
        let Some(fighter) = self.style_fighter() else {
            return;
        };
        let fams = self.style_families(fighter);
        let family = fams[self.family_at % fams.len()];
        let revs = self.style_revisions(fighter, family);
        let n = revs.len().max(1);
        match input {
            MenuInput::Left => self.revision_at = (self.revision_at + n - 1) % n,
            MenuInput::Right => self.revision_at = (self.revision_at + 1) % n,
            MenuInput::Up | MenuInput::Down => {
                let loads = loadouts(revs[self.revision_at % revs.len()]);
                let count = loads.len().max(1);
                self.loadout_at = (self.loadout_at + 1) % count;
            }
            MenuInput::Confirm => {
                self.costume_at = 0;
                self.phase = SelectPhase::Costume;
            }
            MenuInput::Back => self.phase = SelectPhase::EraFamily,
            MenuInput::Filter | MenuInput::Start | MenuInput::TabLeft | MenuInput::TabRight => {}
        }
        self.write_style();
    }

    fn apply_costume(&mut self, input: MenuInput) {
        let Some(fighter) = self.style_fighter() else {
            return;
        };
        let fams = self.style_families(fighter);
        let family = fams[self.family_at % fams.len()];
        let suits = costumes(fighter, family);
        let n = suits.len().max(1);
        match input {
            MenuInput::Left => self.costume_at = (self.costume_at + n - 1) % n,
            MenuInput::Right => self.costume_at = (self.costume_at + 1) % n,
            MenuInput::Up => self.palette_at = (self.palette_at + 1) % PALETTES,
            MenuInput::Down => self.palette_at = (self.palette_at + PALETTES - 1) % PALETTES,
            MenuInput::Confirm => {
                self.write_style();
                if self.style_slot == 0 && self.mode != PlayMode::Arcade && self.p2.is_some() {
                    self.begin_style(1);
                    return;
                }
                self.phase = SelectPhase::Ready;
                return;
            }
            MenuInput::Back => self.phase = SelectPhase::Revision,
            MenuInput::Filter | MenuInput::Start | MenuInput::TabLeft | MenuInput::TabRight => {}
        }
        self.write_style();
    }

    /// Write the confirmed choices back onto roster settings.
    pub fn commit(&self, settings: &mut RosterSettings) -> bool {
        let Some(p1) = self.p1 else {
            return false;
        };
        if !self.is_ready() {
            return false;
        }
        settings.p1 = p1;
        settings.mode = self.mode;
        if self.mode != PlayMode::Arcade {
            let Some(p2) = self.p2 else {
                return false;
            };
            settings.p2 = p2;
        }
        if let Some(country) = self.country {
            settings.country = country;
        }
        true
    }
}
