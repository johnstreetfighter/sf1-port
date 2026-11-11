//! Street Fighter IV revisions, Focus, and the Revenge gauge (`#106`, `O15`).
//!
//! Ryu, Ken, and Sagat pick SF4, Super, Arcade Edition, or Ultra. Adon and Gen
//! start at Super. The super bar has four stocks: one for an EX special, two
//! for a Focus dash cancel, four for a Super Combo. Revenge fills when you
//! take damage and spends on an Ultra.

use crate::era::Loadout;
use crate::roster::TournamentFighter;
use sf_video::callouts::CalloutKind;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow};

pub const REVENGE_AT: u16 = 200;
const ULTRA_DAMAGE: u16 = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sf4Rev {
    Sf4,
    Ssf4,
    Ae2012,
    Usf4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stance {
    #[default]
    Mantis,
    Crane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sf4Event {
    Focus { level: u8, red: bool, crumple: bool, unblockable: bool },
    Fadc,
    Ex(&'static str),
    Super(&'static str),
    Ultra { name: &'static str, damage: u16 },
    AngryCharge,
    DelayedWake { extra: u8 },
    Stance(Stance),
}

impl Sf4Event {
    /// The call-out this event earns: a Super or an Ultra, each a super.
    pub fn callout(self) -> Option<CalloutKind> {
        match self {
            Sf4Event::Super(name) | Sf4Event::Ultra { name, .. } => Some(CalloutKind::Super { name }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sf4Session {
    pub fighter: TournamentFighter,
    pub revision: Sf4Rev,
    pub ultra: Loadout,
    pub stance: Stance,
    bars: u8,
    revenge: u16,
    armor: u8,
    cancel_ok: bool,
    down: bool,
    pub disable_comeback_scaling: bool,
    pub events: Vec<Sf4Event>,
}

impl Sf4Session {
    pub fn new(
        fighter: TournamentFighter,
        revision: Sf4Rev,
        ultra: Loadout,
        disable_comeback_scaling: bool,
    ) -> Self {
        Self {
            fighter,
            revision,
            ultra,
            stance: Stance::Mantis,
            bars: 0,
            revenge: 0,
            armor: 0,
            cancel_ok: false,
            down: false,
            disable_comeback_scaling,
            events: Vec::new(),
        }
    }

    pub fn available(&self) -> bool {
        match (self.fighter, self.revision) {
            (TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Sagat, _) => true,
            (TournamentFighter::Adon | TournamentFighter::Gen, Sf4Rev::Sf4) => false,
            (TournamentFighter::Adon | TournamentFighter::Gen, _) => true,
            _ => false,
        }
    }

    pub fn bars(&self) -> u8 {
        self.bars
    }

    pub fn revenge(&self) -> u16 {
        self.revenge
    }

    pub fn add_bars(&mut self, n: u8) {
        if self.available() {
            self.bars = self.bars.saturating_add(n).min(4);
        }
    }

    /// Damage taken fills the Revenge gauge.
    pub fn take_damage(&mut self, amount: u16) {
        if self.available() {
            self.revenge = self.revenge.saturating_add(amount).min(REVENGE_AT);
        }
    }

    /// Charge Focus. Level 2 and 3 crumple. Level 3 is unblockable.
    /// Red Focus, multi-hit armor, is Ultra only.
    pub fn focus(&mut self, held: u8, red: bool) -> bool {
        if !self.available() {
            return false;
        }
        let level = if held >= 21 { 3 } else if held >= 11 { 2 } else { 1 };
        let red = red && self.revision == Sf4Rev::Usf4;
        self.armor = if red { 3 } else { 1 };
        self.cancel_ok = true;
        self.events.push(Sf4Event::Focus {
            level,
            red,
            crumple: level >= 2,
            unblockable: level == 3,
        });
        true
    }

    pub fn absorb(&mut self) -> bool {
        if self.armor == 0 {
            return false;
        }
        self.armor -= 1;
        true
    }

    pub fn fadc(&mut self) -> bool {
        if !self.available() || !self.cancel_ok || self.bars < 2 {
            return false;
        }
        self.bars -= 2;
        self.cancel_ok = false;
        self.armor = 0;
        self.events.push(Sf4Event::Fadc);
        true
    }

    pub fn ex_special(&mut self, name: &'static str) -> bool {
        if !self.available() || self.bars < 1 {
            return false;
        }
        self.bars -= 1;
        self.cancel_ok = true;
        self.events.push(Sf4Event::Ex(name));
        true
    }

    pub fn super_combo(&mut self) -> bool {
        if !self.available() || self.bars < 4 {
            return false;
        }
        self.bars = 0;
        self.cancel_ok = false;
        self.events.push(Sf4Event::Super(super_name(self.fighter)));
        true
    }

    pub fn angry_charge(&mut self) -> bool {
        if self.fighter != TournamentFighter::Sagat || !self.available() {
            return false;
        }
        self.events.push(Sf4Event::AngryCharge);
        true
    }

    pub fn switch_stance(&mut self, punches: bool) -> bool {
        if self.fighter != TournamentFighter::Gen || !self.available() {
            return false;
        }
        self.stance = if punches { Stance::Mantis } else { Stance::Crane };
        self.events.push(Sf4Event::Stance(self.stance));
        true
    }

    pub fn ultra(&mut self) -> bool {
        if !self.available() || self.revenge < REVENGE_AT {
            return false;
        }
        if self.ultra == Loadout::Ultra2 && self.revision == Sf4Rev::Sf4 {
            return false;
        }
        if self.ultra == Loadout::UltraDouble && self.revision != Sf4Rev::Usf4 {
            return false;
        }
        let Some(name) = ultra_name(self.fighter, self.stance, self.ultra) else {
            return false;
        };
        let name = if self.ultra == Loadout::UltraDouble { "Ultra Double" } else { name };
        self.revenge = 0;
        let mut damage = ULTRA_DAMAGE;
        if self.events.iter().any(|event| matches!(event, Sf4Event::AngryCharge)) {
            damage = damage.saturating_add(40);
        }
        if self.ultra == Loadout::UltraDouble && !self.disable_comeback_scaling {
            damage = damage * 7 / 10;
        }
        self.events.push(Sf4Event::Ultra { name, damage });
        true
    }

    pub fn knock_down(&mut self) {
        self.down = true;
    }

    /// Two buttons during knockdown. Ultra only.
    pub fn delayed_wakeup(&mut self, buttons: u8) -> bool {
        if self.revision != Sf4Rev::Usf4 || !self.down || buttons < 2 || !self.available() {
            return false;
        }
        self.down = false;
        self.events.push(Sf4Event::DelayedWake { extra: 20 });
        true
    }

    /// Four Super cells over the Revenge gauge. Nothing for a fighter this
    /// revision does not carry.
    pub fn hud(&self) -> Option<EraGauge> {
        if !self.available() {
            return None;
        }
        let ultra = u8::from(self.revenge >= REVENGE_AT);
        Some(EraGauge::two(
            GaugeRow::stock(4, self.bars, Fill::EMPTY),
            GaugeRow::stock(1, ultra, Fill::of(self.revenge, REVENGE_AT)),
        ))
    }
}

fn super_name(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ken => "Shoryureppa",
        TournamentFighter::Sagat => "Tiger Genocide",
        TournamentFighter::Adon | TournamentFighter::Gen => "Super Combo",
        _ => "Shinku Hadoken",
    }
}

fn ultra_name(fighter: TournamentFighter, stance: Stance, ultra: Loadout) -> Option<&'static str> {
    match (fighter, stance, ultra) {
        (TournamentFighter::Ryu, _, Loadout::Ultra1) => Some("Metsu Hadoken"),
        (TournamentFighter::Ryu, _, Loadout::Ultra2 | Loadout::UltraDouble) => Some("Metsu Shoryuken"),
        (TournamentFighter::Ken, _, Loadout::Ultra1) => Some("Shinryuken"),
        (TournamentFighter::Ken, _, Loadout::Ultra2 | Loadout::UltraDouble) => Some("Guren Senpukyaku"),
        (TournamentFighter::Sagat, _, Loadout::Ultra1) => Some("Tiger Destruction"),
        (TournamentFighter::Sagat, _, Loadout::Ultra2 | Loadout::UltraDouble) => Some("Tiger Cannon"),
        (TournamentFighter::Adon, _, Loadout::Ultra1) => Some("Jaguar Revolver"),
        (TournamentFighter::Adon, _, Loadout::Ultra2 | Loadout::UltraDouble) => Some("Jaguar Avalanche"),
        (TournamentFighter::Gen, Stance::Mantis, Loadout::Ultra1) => Some("Zetsuei"),
        (TournamentFighter::Gen, Stance::Mantis, Loadout::Ultra2 | Loadout::UltraDouble) => Some("Ryukoha"),
        (TournamentFighter::Gen, Stance::Crane, Loadout::Ultra1) => Some("Teiga"),
        (TournamentFighter::Gen, Stance::Crane, Loadout::Ultra2 | Loadout::UltraDouble) => Some("Kouga"),
        _ => None,
    }
}
