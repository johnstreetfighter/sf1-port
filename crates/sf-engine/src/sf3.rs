//! Street Fighter III revisions, Parry, and Super Arts (`#105`, `O14`).
//!
//! Ryu and Ken each pick New Generation, 2nd Impact, or 3rd Strike, then
//! Super Art I, II, or III. A parry is a forward tap against a high or a
//! down tap against a low, inside 10 frames, or 6 against a fireball.
//! Red Parry is the same tap during blockstun. EX specials start in 2nd Impact.

use crate::era::Loadout;
use crate::roster::TournamentFighter;
use sf_video::callouts::CalloutKind;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow};

/// Super Art I's bar, the longest: the HUD draws the others against it.
const LONGEST_BAR: u16 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sf3Rev {
    Ng,
    TwoI,
    ThreeS,
}

impl Sf3Rev {
    pub const fn ex(self) -> bool {
        matches!(self, Sf3Rev::TwoI | Sf3Rev::ThreeS)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attack {
    pub age: u8,
    pub low: bool,
    pub projectile: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sf3Event {
    Parry { red: bool, advantage: u8 },
    Ex(&'static str),
    Super { name: &'static str, stocks: u8 },
    Move { name: &'static str, hits: u8, startup: u8, overhead: bool, unblockable: bool },
}

impl Sf3Event {
    /// The call-out this event earns: a Super Art.
    pub fn callout(self) -> Option<CalloutKind> {
        match self {
            Sf3Event::Super { name, .. } => Some(CalloutKind::Super { name }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sf3Session {
    pub fighter: TournamentFighter,
    pub revision: Sf3Rev,
    pub art: Loadout,
    stocks: u8,
    pub meter: u16,
    blockstun: u8,
    airborne: bool,
    incoming: Option<Attack>,
    pub events: Vec<Sf3Event>,
    denjin: u8,
}

impl Sf3Session {
    pub fn new(fighter: TournamentFighter, revision: Sf3Rev, art: Loadout) -> Self {
        Self {
            fighter,
            revision,
            art,
            stocks: 0,
            meter: 0,
            blockstun: 0,
            airborne: false,
            incoming: None,
            events: Vec::new(),
            denjin: 0,
        }
    }

    pub fn stocks(&self) -> u8 {
        self.stocks
    }

    pub fn max_stocks(&self) -> u8 {
        match self.art {
            Loadout::Art1 => 1,
            Loadout::Art2 => 2,
            _ => 3,
        }
    }

    /// Meter required to gain one stock. Art I is the long bar.
    pub fn bar_length(&self) -> u16 {
        match self.art {
            Loadout::Art1 => 120,
            Loadout::Art2 => 80,
            _ => 40,
        }
    }

    pub fn add_meter(&mut self, amount: u16) {
        if self.stocks >= self.max_stocks() {
            return;
        }
        self.meter = self.meter.saturating_add(amount);
        while self.meter >= self.bar_length() && self.stocks < self.max_stocks() {
            self.meter -= self.bar_length();
            self.stocks += 1;
        }
    }

    pub fn jump(&mut self) {
        self.airborne = true;
    }

    pub fn land(&mut self) {
        self.airborne = false;
    }

    pub fn offer(&mut self, attack: Attack) {
        self.incoming = Some(attack);
    }

    pub fn enter_block(&mut self, frames: u8) {
        self.blockstun = frames;
    }

    pub fn tap_forward(&mut self) -> bool {
        self.try_parry(false)
    }

    pub fn tap_down(&mut self) -> bool {
        if self.airborne {
            return false;
        }
        self.try_parry(true)
    }

    fn try_parry(&mut self, want_low: bool) -> bool {
        let red = self.blockstun > 0;
        let Some(attack) = self.incoming else {
            return false;
        };
        let window = if attack.projectile { 6 } else { 10 };
        let height_ok = if self.airborne { true } else { attack.low == want_low };
        if !red && (attack.age > window || !height_ok) {
            return false;
        }
        if red && want_low {
            return false;
        }
        self.incoming = None;
        self.blockstun = 0;
        self.events.push(Sf3Event::Parry { red, advantage: if red { 6 } else { 8 } });
        true
    }

    pub fn ex_special(&mut self, name: &'static str) -> bool {
        if !self.revision.ex() || self.stocks == 0 {
            return false;
        }
        self.stocks -= 1;
        self.events.push(Sf3Event::Ex(name));
        true
    }

    pub fn joudan(&mut self) -> bool {
        if self.fighter != TournamentFighter::Ryu {
            return false;
        }
        self.events.push(Sf3Event::Move {
            name: "Joudan Sokutogeri",
            hits: 1,
            startup: 8,
            overhead: false,
            unblockable: false,
        });
        true
    }

    pub fn overhead(&mut self) -> bool {
        self.events.push(Sf3Event::Move {
            name: "Universal Overhead",
            hits: 1,
            startup: 12,
            overhead: true,
            unblockable: false,
        });
        true
    }

    /// Target combo: standing medium punch into standing heavy punch.
    pub fn target_combo(&mut self) -> bool {
        self.events.push(Sf3Event::Move {
            name: "stand MP",
            hits: 1,
            startup: 4,
            overhead: false,
            unblockable: false,
        });
        self.events.push(Sf3Event::Move {
            name: "stand HP",
            hits: 1,
            startup: 6,
            overhead: false,
            unblockable: false,
        });
        true
    }

    pub fn charge_denjin(&mut self, frames: u8) {
        if self.super_name() == Some("Denjin Hadoken") {
            self.denjin = self.denjin.saturating_add(frames);
        }
    }

    pub fn super_art(&mut self) -> bool {
        let Some(name) = self.super_name() else {
            return false;
        };
        let need = self.max_stocks();
        if self.stocks < need {
            return false;
        }
        if name == "Denjin Hadoken" && self.denjin < 30 {
            return false;
        }
        self.stocks = 0;
        let startup = if name == "Shippu Jinraikyaku" { 1 } else { 8 };
        let unblockable = name == "Denjin Hadoken";
        self.events.push(Sf3Event::Super { name, stocks: need });
        self.events.push(Sf3Event::Move {
            name,
            hits: 1,
            startup,
            overhead: false,
            unblockable,
        });
        true
    }

    pub fn super_name(&self) -> Option<&'static str> {
        match (self.fighter, self.art) {
            (TournamentFighter::Ryu, Loadout::Art1) => Some("Shinku Hadoken"),
            (TournamentFighter::Ryu, Loadout::Art2) => Some("Shin Shoryuken"),
            (TournamentFighter::Ryu, Loadout::Art3) => Some("Denjin Hadoken"),
            (TournamentFighter::Ken, Loadout::Art1) => Some("Shoryureppa"),
            (TournamentFighter::Ken, Loadout::Art2) => Some("Shinryuken"),
            (TournamentFighter::Ken, Loadout::Art3) => Some("Shippu Jinraikyaku"),
            _ => None,
        }
    }

    /// The Super Art bar: the stock count beside it, filling from the meter
    /// toward the next stock. Its length follows the chosen art's bar.
    pub fn hud(&self) -> Option<EraGauge> {
        let fill = if self.stocks >= self.max_stocks() {
            Fill::FULL
        } else {
            Fill::of(self.meter, self.bar_length())
        };
        let row = GaugeRow::level(self.stocks, self.max_stocks(), fill)
            .with_length(Fill::of(self.bar_length(), LONGEST_BAR));
        Some(EraGauge::one(row))
    }
}
