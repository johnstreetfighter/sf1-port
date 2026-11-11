//! What-if SF3 3rd Strike, Ultra SF4, SF5 Champion Edition, and SF6 (`#116`, `M3`).
//!
//! These styles stay off the select list until what-if styles are on. Each
//! fighter who did not ship in that game gets the flagship revision only.

use crate::era::{listed_revisions, EraFamily, Loadout, Revision};
use crate::roster::TournamentFighter;
use sf_video::asset_pack::{
    encode_adpcm, quantize_sheet, simple_fm_patch, validate_adpcm, validate_fm_patch, validate_tile,
    AdpcmClip, FmPatch, IndexedTile, Rgba,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagshipEvent {
    Parry,
    Overhead,
    Ex(&'static str),
    Super { level: u8, name: &'static str },
    Focus { level: u8, red: bool },
    Fadc,
    DelayedWakeup,
    Ultra(&'static str),
    CrushCounter,
    VSkill(&'static str),
    VReversal,
    VTrigger(&'static str),
    VShift,
    CriticalArt,
    Overdrive(&'static str),
    DriveParry { perfect: bool },
    DriveImpact,
    DriveRush,
    Punch(&'static str),
}

#[derive(Debug, Clone)]
pub struct FlagshipSession {
    pub fighter: TournamentFighter,
    pub family: EraFamily,
    pub loadout: Loadout,
    pub what_if: bool,
    stocks: u8,
    drive: u8,
    v_bars: u8,
    cancel_ok: bool,
    blocking: bool,
    knockdown: bool,
    pub tiles: Vec<IndexedTile>,
    pub voice: AdpcmClip,
    pub patch: FmPatch,
    pub events: Vec<FlagshipEvent>,
}

impl FlagshipSession {
    pub fn new(fighter: TournamentFighter, family: EraFamily, loadout: Loadout, what_if: bool) -> Self {
        let tiles = tiles_for(fighter);
        for tile in &tiles {
            validate_tile(tile).expect("flagship tile");
        }
        let voice = encode_adpcm(&cue(fighter)).expect("flagship voice");
        validate_adpcm(&voice).expect("flagship voice");
        let patch = simple_fm_patch(channel(fighter), 0x18);
        validate_fm_patch(&patch).expect("flagship patch");
        Self {
            fighter,
            family,
            loadout,
            what_if,
            stocks: 0,
            drive: 6,
            v_bars: 0,
            cancel_ok: false,
            blocking: false,
            knockdown: false,
            tiles,
            voice,
            patch,
            events: Vec::new(),
        }
    }

    pub fn available(&self) -> bool {
        let revision = match self.family {
            EraFamily::Sf3 => Revision::Sf3ThreeS,
            EraFamily::Sf4 => Revision::Usf4,
            EraFamily::Sf5 => Revision::Sf5Ce,
            EraFamily::Sf6 => Revision::Sf6,
            _ => return false,
        };
        listed_revisions(self.fighter, self.family, self.what_if).contains(&revision)
            && loadout_fits(self.family, self.loadout)
    }

    pub fn drive_bars(&self) -> u8 {
        self.drive
    }

    pub fn land_hit(&mut self, heavy: bool) {
        if !self.available() {
            return;
        }
        self.stocks = (self.stocks + 1).min(match self.family {
            EraFamily::Sf4 => 4,
            _ => 3,
        });
        if self.family == EraFamily::Sf5 && heavy {
            self.events.push(FlagshipEvent::CrushCounter);
        }
        if self.family == EraFamily::Sf5 {
            self.v_bars = (self.v_bars + 1).min(if self.trigger_two() { 3 } else { 2 });
        }
        if self.family == EraFamily::Sf6 {
            self.drive = (self.drive + 1).min(6);
        }
    }

    pub fn parry(&mut self) -> bool {
        self.mark(self.family == EraFamily::Sf3, FlagshipEvent::Parry)
    }

    pub fn overhead(&mut self) -> bool {
        self.mark(self.family == EraFamily::Sf3, FlagshipEvent::Overhead)
    }

    pub fn ex_special(&mut self, name: &'static str) -> bool {
        if !self.available() || self.family != EraFamily::Sf3 || self.stocks == 0 {
            return false;
        }
        self.stocks -= 1;
        self.events.push(FlagshipEvent::Ex(name));
        true
    }

    pub fn super_art(&mut self, level: u8) -> bool {
        if !self.available() || self.family != EraFamily::Sf3 || !(1..=3).contains(&level) || self.stocks < level {
            return false;
        }
        if !matches!(self.loadout, Loadout::Art1 | Loadout::Art2 | Loadout::Art3) {
            return false;
        }
        if level != art_level(self.loadout) {
            return false;
        }
        self.stocks -= level;
        self.events.push(FlagshipEvent::Super { level, name: super_name(self.fighter, level) });
        true
    }

    pub fn focus(&mut self, held: u8, red: bool) -> bool {
        if !self.available() || self.family != EraFamily::Sf4 {
            return false;
        }
        let level = if held >= 21 { 3 } else if held >= 11 { 2 } else { 1 };
        self.cancel_ok = true;
        self.events.push(FlagshipEvent::Focus { level, red });
        true
    }

    pub fn fadc(&mut self) -> bool {
        if !self.available() || !self.cancel_ok || self.stocks < 2 {
            return false;
        }
        self.stocks -= 2;
        self.cancel_ok = false;
        self.events.push(FlagshipEvent::Fadc);
        true
    }

    pub fn delayed_wakeup(&mut self, buttons: u8) -> bool {
        if !self.available() || self.family != EraFamily::Sf4 || !self.knockdown || buttons < 2 {
            return false;
        }
        self.knockdown = false;
        self.events.push(FlagshipEvent::DelayedWakeup);
        true
    }

    pub fn knock_down(&mut self) {
        self.knockdown = true;
    }

    pub fn ultra(&mut self) -> bool {
        if !self.available() || self.family != EraFamily::Sf4 || self.stocks < 4 {
            return false;
        }
        self.stocks -= 4;
        self.events.push(FlagshipEvent::Ultra(ultra_name(self.fighter, self.loadout)));
        true
    }

    pub fn v_skill(&mut self) -> bool {
        if !self.available() || self.family != EraFamily::Sf5 {
            return false;
        }
        self.events.push(FlagshipEvent::VSkill(if self.skill_two() { "V-Skill II" } else { "V-Skill I" }));
        true
    }

    pub fn v_reversal(&mut self) -> bool {
        if !self.available() || self.family != EraFamily::Sf5 || !self.blocking || self.v_bars == 0 {
            return false;
        }
        self.v_bars -= 1;
        self.blocking = false;
        self.events.push(FlagshipEvent::VReversal);
        true
    }

    pub fn v_trigger(&mut self) -> bool {
        if !self.available() || self.family != EraFamily::Sf5 || self.v_bars < self.max_v() {
            return false;
        }
        self.v_bars = 0;
        self.events.push(FlagshipEvent::VTrigger(if self.trigger_two() { "V-Trigger II" } else { "V-Trigger I" }));
        true
    }

    pub fn v_shift(&mut self) -> bool {
        if !self.available() || self.family != EraFamily::Sf5 || self.v_bars == 0 {
            return false;
        }
        self.v_bars -= 1;
        self.events.push(FlagshipEvent::VShift);
        true
    }

    pub fn critical_art(&mut self) -> bool {
        if !self.available() || self.family != EraFamily::Sf5 || self.stocks < 3 {
            return false;
        }
        self.stocks = 0;
        self.events.push(FlagshipEvent::CriticalArt);
        true
    }

    pub fn block(&mut self) {
        self.blocking = true;
    }

    pub fn overdrive(&mut self, name: &'static str) -> bool {
        if !self.available() || self.family != EraFamily::Sf6 || self.drive < 2 {
            return false;
        }
        self.drive -= 2;
        self.events.push(FlagshipEvent::Overdrive(name));
        true
    }

    pub fn drive_parry(&mut self, age: u8) -> bool {
        if !self.available() || self.family != EraFamily::Sf6 || self.drive == 0 {
            return false;
        }
        self.drive -= 1;
        self.events.push(FlagshipEvent::DriveParry { perfect: age <= 2 });
        true
    }

    pub fn drive_impact(&mut self) -> bool {
        if !self.available() || self.family != EraFamily::Sf6 || self.drive == 0 {
            return false;
        }
        self.drive -= 1;
        self.events.push(FlagshipEvent::DriveImpact);
        true
    }

    pub fn drive_rush(&mut self) -> bool {
        if !self.available() || self.family != EraFamily::Sf6 || self.drive < 3 {
            return false;
        }
        self.drive -= 3;
        self.events.push(FlagshipEvent::DriveRush);
        true
    }

    pub fn level_super(&mut self, level: u8) -> bool {
        if !self.available() || self.family != EraFamily::Sf6 || !(1..=3).contains(&level) || self.stocks < level {
            return false;
        }
        self.stocks -= level;
        self.events.push(FlagshipEvent::Super { level, name: level3(self.fighter, level) });
        true
    }

    /// Mike's six punches. Other fighters have no boxer layout.
    pub fn punch(&mut self, index: u8) -> bool {
        if !self.available() || self.fighter != TournamentFighter::Mike || index > 5 {
            return false;
        }
        let name = ["Jab", "Straight", "Hook", "Body", "Upper", "Sway"][index as usize];
        self.events.push(FlagshipEvent::Punch(name));
        true
    }

    fn max_v(&self) -> u8 {
        if self.trigger_two() { 3 } else { 2 }
    }

    fn skill_two(&self) -> bool {
        matches!(self.loadout, Loadout::Skill2Trigger1 | Loadout::Skill2Trigger2)
    }

    fn trigger_two(&self) -> bool {
        matches!(self.loadout, Loadout::Skill1Trigger2 | Loadout::Skill2Trigger2)
    }

    fn mark(&mut self, ok: bool, event: FlagshipEvent) -> bool {
        if !self.available() || !ok {
            return false;
        }
        self.events.push(event);
        true
    }
}

fn loadout_fits(family: EraFamily, loadout: Loadout) -> bool {
    match family {
        EraFamily::Sf3 => matches!(loadout, Loadout::Art1 | Loadout::Art2 | Loadout::Art3),
        EraFamily::Sf4 => matches!(loadout, Loadout::Ultra1 | Loadout::Ultra2 | Loadout::UltraDouble),
        EraFamily::Sf5 => matches!(
            loadout,
            Loadout::Skill1Trigger1 | Loadout::Skill1Trigger2 | Loadout::Skill2Trigger1 | Loadout::Skill2Trigger2
        ),
        EraFamily::Sf6 => true,
        _ => false,
    }
}

fn art_level(loadout: Loadout) -> u8 {
    match loadout {
        Loadout::Art2 => 2,
        Loadout::Art3 => 3,
        _ => 1,
    }
}

fn super_name(fighter: TournamentFighter, level: u8) -> &'static str {
    if level == 3 {
        return level3(fighter, 3);
    }
    if level == 2 { "Super Art II" } else { "Super Art I" }
}

fn level3(fighter: TournamentFighter, level: u8) -> &'static str {
    if level < 3 {
        return if level == 2 { "Level 2" } else { "Level 1" };
    }
    match fighter {
        TournamentFighter::Joe => "Meteor Kick",
        TournamentFighter::Lee => "One Man Army",
        TournamentFighter::Geki => "Ansatsu",
        TournamentFighter::Mike => "Crazy Rush",
        TournamentFighter::Retsu => "Renkiko Cannon",
        _ => "Level 3",
    }
}

fn ultra_name(fighter: TournamentFighter, loadout: Loadout) -> &'static str {
    match (fighter, loadout) {
        (_, Loadout::UltraDouble) => "Ultra Double",
        (_, Loadout::Ultra2) => "Ultra II",
        _ => "Ultra I",
    }
}

fn channel(fighter: TournamentFighter) -> u8 {
    TournamentFighter::ALL.iter().position(|one| *one == fighter).unwrap_or(0) as u8 & 7
}

fn cue(fighter: TournamentFighter) -> Vec<i16> {
    let base = 80 + i16::from(channel(fighter)) * 18;
    (0..32).map(|i| base + i * 9).collect()
}

fn tiles_for(fighter: TournamentFighter) -> Vec<IndexedTile> {
    let mut pixels = vec![Rgba::TRANSPARENT; 256];
    let ink = channel(fighter).wrapping_mul(24);
    for y in 1..15 {
        for x in 2..14 {
            if (x + y) % 2 == 0 {
                pixels[y * 16 + x] = Rgba { r: 90 + ink, g: 40, b: 28, a: 255 };
            }
        }
    }
    quantize_sheet(&pixels, 16, 16).expect("flagship sheet")
}
