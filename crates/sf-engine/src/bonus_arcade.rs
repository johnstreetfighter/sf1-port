//! Bonus-challenger arcade routes, rival intrusions, and endings (`#120`, `M7`).
//!
//! Each of the thirteen can clear the ten country opponents and the bonus
//! stages. A strong run can insert one secret rival. Losing that fight does
//! not spend a credit.

use crate::arcade::{route_from, Country};
use crate::bonus::Challenger;
use crate::roster::{RosterSettings, TournamentFighter};
use sf_video::asset_pack::{quantize_sheet, validate_tile, IndexedTile, Rgba};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    Fight(TournamentFighter),
    Bonus(&'static str),
    Intrusion(Challenger),
    Ending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterLoss {
    Continue,
    GameOver,
    Rematch,
    Advance,
}

#[derive(Debug, Clone)]
pub struct BonusRun {
    pub player: Challenger,
    pub settings: RosterSettings,
    pub credits: u8,
    pub perfects: u8,
    pub supers: u8,
    pub continues: u8,
    ladder: Vec<Stop>,
    at: usize,
    pub dialogue: Option<String>,
    pub ending: Vec<IndexedTile>,
}

impl BonusRun {
    pub fn new(player: Challenger, start: Country, settings: RosterSettings) -> Self {
        Self {
            player,
            settings,
            credits: 1,
            perfects: 0,
            supers: 0,
            continues: 0,
            ladder: ladder(start),
            at: 0,
            dialogue: None,
            ending: Vec::new(),
        }
    }

    pub fn current(&self) -> Stop {
        self.ladder.get(self.at).copied().unwrap_or(Stop::Ending)
    }

    pub fn opponents(&self) -> Vec<TournamentFighter> {
        self.ladder.iter().filter_map(|stop| match stop {
            Stop::Fight(fighter) => Some(*fighter),
            _ => None,
        }).collect()
    }

    pub fn ladder_has_bonus(&self) -> bool {
        self.ladder.iter().any(|stop| matches!(stop, Stop::Bonus(_)))
    }

    pub fn intrusion(&self) -> Option<Challenger> {
        self.ladder.iter().find_map(|stop| match stop {
            Stop::Intrusion(fighter) => Some(*fighter),
            _ => None,
        })
    }

    pub fn note_perfect(&mut self) {
        self.perfects += 1;
        self.maybe_insert();
    }

    pub fn note_super(&mut self) {
        self.supers += 1;
        self.maybe_insert();
    }

    pub fn win(&mut self) {
        if self.at + 1 >= self.ladder.len() || matches!(self.current(), Stop::Fight(TournamentFighter::Sagat)) {
            self.finish();
            return;
        }
        self.at += 1;
        if matches!(self.current(), Stop::Fight(TournamentFighter::Adon)) {
            self.maybe_insert();
        }
    }

    pub fn lose(&mut self) -> AfterLoss {
        if matches!(self.current(), Stop::Intrusion(_)) {
            return AfterLoss::Rematch;
        }
        if self.credits == 0 {
            return AfterLoss::GameOver;
        }
        self.credits -= 1;
        self.continues += 1;
        AfterLoss::Continue
    }

    pub fn rematch(&mut self) -> bool {
        matches!(self.current(), Stop::Intrusion(_))
    }

    pub fn advance_past_intrusion(&mut self) -> bool {
        if !matches!(self.current(), Stop::Intrusion(_)) {
            return false;
        }
        self.at += 1;
        true
    }

    fn intrusions_allowed(&self) -> bool {
        self.settings.bonus_challengers && self.settings.secret_intrusions
    }

    fn milestone(&self) -> bool {
        self.perfects > 0 || self.supers > 0
    }

    fn maybe_insert(&mut self) {
        if !self.intrusions_allowed() || !self.milestone() {
            return;
        }
        if self.ladder.iter().any(|stop| matches!(stop, Stop::Intrusion(_))) {
            return;
        }
        let Some(at) = self.ladder.iter().position(|stop| matches!(stop, Stop::Fight(TournamentFighter::Adon))) else {
            return;
        };
        self.ladder.insert(at, Stop::Intrusion(rival_of(self.player)));
    }

    fn finish(&mut self) {
        self.at = self.ladder.len();
        self.dialogue = Some(dialogue(self.player));
        self.ending = ending_panels(self.player);
    }
}

pub fn ladder(start: Country) -> Vec<Stop> {
    let order = route_from(Country::Japan);
    let mut countries: Vec<Country> = order.to_vec();
    if let Some(at) = countries.iter().position(|country| *country == start) {
        if start != Country::Thailand {
            let tail = countries.split_off(at);
            let head = countries;
            countries = tail.into_iter().filter(|country| *country != Country::Thailand).collect();
            countries.extend(head.into_iter().filter(|country| *country != Country::Thailand));
            countries.push(Country::Thailand);
        }
    }
    let mut stops = Vec::new();
    for country in countries {
        let (first, second) = pair(country);
        stops.push(Stop::Fight(first));
        stops.push(Stop::Fight(second));
        if country != Country::Thailand {
            stops.push(Stop::Bonus(bonus_name(country)));
        }
    }
    stops
}

fn pair(country: Country) -> (TournamentFighter, TournamentFighter) {
    match country {
        Country::Japan => (TournamentFighter::Retsu, TournamentFighter::Geki),
        Country::Usa => (TournamentFighter::Joe, TournamentFighter::Mike),
        Country::China => (TournamentFighter::Lee, TournamentFighter::Gen),
        Country::England => (TournamentFighter::Birdie, TournamentFighter::Eagle),
        Country::Thailand => (TournamentFighter::Adon, TournamentFighter::Sagat),
    }
}

fn bonus_name(country: Country) -> &'static str {
    match country {
        Country::Japan => "Plank",
        Country::Usa => "Barrels",
        Country::China => "Drums",
        Country::England => "Glass",
        Country::Thailand => "None",
    }
}

fn rival_of(player: Challenger) -> Challenger {
    match player {
        Challenger::Gouken => Challenger::Akuma,
        Challenger::Akuma => Challenger::Gouken,
        Challenger::EvilRyu => Challenger::Gouken,
        Challenger::Goutetsu => Challenger::Akuma,
        Challenger::ShengLong => Challenger::Gouken,
        Challenger::GouHibiki => Challenger::Dan,
        Challenger::Dan => Challenger::GouHibiki,
        Challenger::Cody => Challenger::Guy,
        Challenger::Guy => Challenger::Cody,
        Challenger::Haggar => Challenger::Cody,
        Challenger::Nash => Challenger::Bison,
        Challenger::Bison => Challenger::Nash,
        Challenger::ChunLi => Challenger::Bison,
    }
}

fn dialogue(player: Challenger) -> String {
    match player {
        Challenger::GouHibiki => "Gou Hibiki walks away from Sagat. The what-if survival ending.".into(),
        Challenger::Bison => "M. Bison recruits Sagat into Shadaloo.".into(),
        other => format!("{} faces the rival, then the ending.", other.name()),
    }
}

fn ending_panels(player: Challenger) -> Vec<IndexedTile> {
    let id = Challenger::ALL.iter().position(|one| *one == player).unwrap_or(0);
    let ink = Rgba { r: 60 + (id as u8) * 10, g: 32, b: 24, a: 255 };
    let accent = Rgba { r: 170, g: 120, b: 36, a: 255 };
    let mut pixels = vec![Rgba::TRANSPARENT; 64 * 16];
    for panel in 0..4 {
        let x0 = panel * 16;
        for y in 1..15 {
            for x in 2..14 {
                if (x + y + panel + id) % 3 != 0 {
                    pixels[y * 64 + x0 + x] = if panel == 3 { accent } else { ink };
                }
            }
        }
    }
    let panels = quantize_sheet(&pixels, 64, 16).expect("bonus ending");
    for panel in &panels {
        validate_tile(panel).expect("bonus ending panel");
    }
    panels
}
