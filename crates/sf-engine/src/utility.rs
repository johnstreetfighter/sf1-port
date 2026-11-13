//! Style-aware CPU and training drills (`#109`, `O18`).
//!
//! A raw SF1 CPU stays on the 1987 table. SF1+ through SF6 use a utility
//! controller whose choices come from the session seed. Training mode runs
//! the four scenario drills and a combo trial for each fighter and era.

use crate::era::EraFamily;
use crate::roster::{PlayMode, TournamentFighter};

mod live;
pub use live::{UtilityAi, UtilityState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
    VeryHard,
}

impl Difficulty {
    pub const fn delay(self) -> u8 {
        match self {
            Difficulty::Easy => 12,
            Difficulty::Normal => 8,
            Difficulty::Hard => 4,
            Difficulty::VeryHard => 1,
        }
    }

    pub const fn rate(self) -> u8 {
        match self {
            Difficulty::Easy => 40,
            Difficulty::Normal => 70,
            Difficulty::Hard => 90,
            Difficulty::VeryHard => 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuKind {
    Table,
    Utility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Hold,
    Normal,
    SpecialCancel,
    AntiAir,
    ThrowTech,
    Super,
    AlphaCounter,
    Parry,
    Fadc,
    VTrigger,
    DriveImpact,
    DriveRush,
    CommandThrow,
    Teleport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spot {
    pub distance: i32,
    pub airborne: bool,
    pub throw_attempt: bool,
    pub can_cancel: bool,
    pub attack_incoming: bool,
    pub blocking: bool,
    pub stocks: u8,
    pub cancel_ok: bool,
    pub command_throw: bool,
    pub teleport: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub kind: CpuKind,
    pub action: Action,
    pub delay: u8,
}

/// Same seed and spot always return the same decision.
pub fn decide(
    mode: PlayMode,
    family: EraFamily,
    difficulty: Difficulty,
    fair_cpu: bool,
    seed: u64,
    spot: Spot,
) -> Decision {
    let _ = mode;
    if family == EraFamily::Sf1987 {
        return Decision {
            kind: CpuKind::Table,
            action: Action::Normal,
            delay: if fair_cpu { 8 } else { 0 },
        };
    }
    let delay = difficulty.delay();
    if roll(seed) >= difficulty.rate() {
        return Decision { kind: CpuKind::Utility, action: Action::Hold, delay };
    }
    let action = if spot.command_throw {
        Action::CommandThrow
    } else if spot.teleport {
        Action::Teleport
    } else if spot.airborne && spot.distance < 96 {
        Action::AntiAir
    } else if spot.throw_attempt {
        Action::ThrowTech
    } else if spot.can_cancel {
        Action::SpecialCancel
    } else if let Some(gauge) = gauge_action(family, spot) {
        gauge
    } else {
        Action::Normal
    };
    Decision { kind: CpuKind::Utility, action, delay }
}

fn gauge_action(family: EraFamily, spot: Spot) -> Option<Action> {
    match family {
        EraFamily::Sf2 if spot.stocks > 0 => Some(Action::Super),
        EraFamily::SfZ if spot.blocking => Some(Action::AlphaCounter),
        EraFamily::SfZ if spot.stocks > 0 => Some(Action::Super),
        EraFamily::Sf3 if spot.attack_incoming => Some(Action::Parry),
        EraFamily::Sf3 if spot.stocks > 0 => Some(Action::Super),
        EraFamily::Sf4 if spot.cancel_ok && spot.stocks >= 2 => Some(Action::Fadc),
        EraFamily::Sf4 if spot.stocks >= 4 => Some(Action::Super),
        EraFamily::Sf5 if spot.stocks > 0 => Some(Action::VTrigger),
        EraFamily::Sf6 if spot.distance < 48 && spot.stocks > 0 => Some(Action::DriveImpact),
        EraFamily::Sf6 if spot.stocks > 0 => Some(Action::DriveRush),
        _ => None,
    }
}

fn roll(seed: u64) -> u8 {
    let mut x = seed | 1;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    (x % 100) as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drill {
    AntiAir,
    WhiffPunish,
    ThrowTech,
    Survival,
}

mod training;
pub use training::{ComboTrialSpec, DrillController, RosterFighter, TrialStep};
pub(crate) use training::{TrialController, TrialFrame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillRun {
    pub drill: Drill,
    hits: u8,
    pub health: u16,
    pub complete: bool,
    pub failed: bool,
}

impl DrillRun {
    pub fn start(drill: Drill) -> Self {
        Self { drill, hits: 0, health: 160, complete: false, failed: false }
    }

    pub fn respond(&mut self, action: Action) -> bool {
        if self.complete || self.failed {
            return self.complete;
        }
        let ok = match self.drill {
            Drill::AntiAir => action == Action::AntiAir,
            Drill::WhiffPunish => action == Action::Normal,
            Drill::ThrowTech => action == Action::ThrowTech,
            Drill::Survival => action == Action::Hold,
        };
        if self.drill == Drill::Survival {
            if ok {
                self.hits += 1;
                self.complete = self.hits >= 4;
            } else {
                self.health = self.health.saturating_sub(40);
                self.failed = self.health == 0;
            }
            return ok;
        }
        if ok {
            self.hits += 1;
            self.complete = self.hits >= 3;
        }
        ok
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComboTrial {
    pub fighter: TournamentFighter,
    pub family: EraFamily,
    cursor: usize,
    pub failed: bool,
}

impl ComboTrial {
    pub fn new(fighter: TournamentFighter, family: EraFamily) -> Self {
        Self { fighter, family, cursor: 0, failed: false }
    }

    pub fn steps(&self) -> [&'static str; 3] {
        ["Normal", "Special", finisher(self.family)]
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.cursor, 3)
    }

    pub fn advance(&mut self, step: &str) -> bool {
        if self.failed || self.cursor >= 3 {
            return self.cursor >= 3 && !self.failed;
        }
        if step == self.steps()[self.cursor] {
            self.cursor += 1;
            true
        } else {
            self.failed = true;
            false
        }
    }

    pub fn complete(&self) -> bool {
        self.cursor == 3 && !self.failed
    }
}

fn finisher(family: EraFamily) -> &'static str {
    match family {
        EraFamily::Sf1987 | EraFamily::Sf1Plus => "Special",
        EraFamily::Sf2 | EraFamily::SfZ => "Super",
        EraFamily::Sf3 => "Super Art",
        EraFamily::Sf4 => "Ultra",
        EraFamily::Sf5 => "Critical Art",
        EraFamily::Sf6 => "Level 3",
    }
}
