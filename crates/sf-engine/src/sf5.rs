//! Street Fighter V revisions and the V-System (`#107`, `O16`).
//!
//! Ryu, Ken, and Birdie pick SF5, Arcade Edition, and Champion Edition.
//! Sagat starts at Arcade Edition. The EX bar holds three stocks. The V-Gauge
//! holds two bars on V-Trigger I and three on V-Trigger II. Champion Edition
//! adds V-Shift.

use crate::era::{Costume, Loadout, Revision};
use crate::roster::TournamentFighter;
use sf_video::asset_pack::{
    encode_adpcm, quantize_sheet, simple_fm_patch, validate_adpcm, validate_fm_patch, validate_tile,
    AdpcmClip, FmPatch, IndexedTile, Rgba,
};
use sf_video::callouts::CalloutKind;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

const V_BAR: u16 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sf5Event {
    CrushCounter,
    VSkill(&'static str),
    VReversal,
    VTrigger(&'static str),
    VShift,
    VShiftBreak,
    Ex(&'static str),
    CriticalArt,
}

impl Sf5Event {
    /// The call-out this event earns: CRUSH COUNTER, or a Critical Art as a
    /// super. The model holds no Critical Art names, so the line reads
    /// CRITICAL ART.
    pub fn callout(self) -> Option<CalloutKind> {
        match self {
            Sf5Event::CrushCounter => Some(CalloutKind::CrushCounter),
            Sf5Event::CriticalArt => Some(CalloutKind::Super { name: "Critical Art" }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sf5Session {
    pub fighter: TournamentFighter,
    pub revision: Revision,
    pub loadout: Loadout,
    pub costume: Costume,
    ex: u8,
    v_bars: u8,
    v_meter: u16,
    trigger: bool,
    shifted: bool,
    blockstun: u8,
    pub tiles: Vec<IndexedTile>,
    pub voice: AdpcmClip,
    pub patch: FmPatch,
    pub events: Vec<Sf5Event>,
}

impl Sf5Session {
    pub fn new(fighter: TournamentFighter, revision: Revision, loadout: Loadout, costume: Costume) -> Self {
        let tiles = costume_tiles(fighter, costume);
        for tile in &tiles {
            validate_tile(tile).expect("sf5 costume");
        }
        let voice = encode_adpcm(&cue(fighter)).expect("sf5 voice");
        validate_adpcm(&voice).expect("sf5 voice");
        let patch = simple_fm_patch(channel(fighter), 0x20);
        validate_fm_patch(&patch).expect("sf5 patch");
        Self {
            fighter,
            revision,
            loadout,
            costume,
            ex: 0,
            v_bars: 0,
            v_meter: 0,
            trigger: false,
            shifted: false,
            blockstun: 0,
            tiles,
            voice,
            patch,
            events: Vec::new(),
        }
    }

    pub fn available(&self) -> bool {
        match (self.fighter, self.revision) {
            (TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Birdie, Revision::Sf5 | Revision::Sf5Ae | Revision::Sf5Ce) => true,
            (TournamentFighter::Sagat, Revision::Sf5Ae | Revision::Sf5Ce) => true,
            _ => false,
        }
    }

    pub fn ex_stocks(&self) -> u8 {
        self.ex
    }

    pub fn v_bars(&self) -> u8 {
        self.v_bars
    }

    pub fn max_v(&self) -> u8 {
        if matches!(self.loadout, Loadout::Skill1Trigger2 | Loadout::Skill2Trigger2) { 3 } else { 2 }
    }

    pub fn trigger_active(&self) -> bool {
        self.trigger
    }

    pub fn skill_name(&self) -> &'static str {
        match (self.fighter, self.loadout) {
            (TournamentFighter::Ryu, Loadout::Skill2Trigger1 | Loadout::Skill2Trigger2) => "Thrust Strike",
            (TournamentFighter::Ryu, _) => "Mind's Eye",
            (TournamentFighter::Ken, Loadout::Skill2Trigger1 | Loadout::Skill2Trigger2) => "Ryusenkyaku",
            (TournamentFighter::Ken, _) => "Quick Step",
            (TournamentFighter::Birdie, Loadout::Skill2Trigger1 | Loadout::Skill2Trigger2) => "Chewing Time",
            (TournamentFighter::Birdie, _) => "Break Time",
            (TournamentFighter::Sagat, Loadout::Skill2Trigger1 | Loadout::Skill2Trigger2) => "Tigerylight",
            _ => "Angry Charge",
        }
    }

    pub fn trigger_name(&self) -> &'static str {
        match (self.fighter, self.loadout) {
            (TournamentFighter::Ryu, Loadout::Skill1Trigger2 | Loadout::Skill2Trigger2) => "Kakko Fubatsu",
            (TournamentFighter::Ryu, _) => "Denjin Renki",
            (TournamentFighter::Ken, Loadout::Skill1Trigger2 | Loadout::Skill2Trigger2) => "Shinryuken",
            (TournamentFighter::Ken, _) => "Heat Rush",
            (TournamentFighter::Birdie, Loadout::Skill1Trigger2 | Loadout::Skill2Trigger2) => "Birdie Time",
            (TournamentFighter::Birdie, _) => "Enjoy Time",
            (TournamentFighter::Sagat, Loadout::Skill1Trigger2 | Loadout::Skill2Trigger2) => "Tiger Assault",
            _ => "Tiger Charge",
        }
    }

    pub fn land_hit(&mut self, heavy: bool) {
        if !self.available() {
            return;
        }
        if heavy {
            self.events.push(Sf5Event::CrushCounter);
        }
        self.ex = (self.ex + 1).min(3);
        self.v_meter = self.v_meter.saturating_add(16);
        while self.v_meter >= V_BAR && self.v_bars < self.max_v() {
            self.v_meter -= V_BAR;
            self.v_bars += 1;
        }
    }

    pub fn v_skill(&mut self) -> bool {
        if !self.available() {
            return false;
        }
        self.events.push(Sf5Event::VSkill(self.skill_name()));
        true
    }

    pub fn enter_block(&mut self, frames: u8) {
        self.blockstun = frames;
    }

    pub fn v_reversal(&mut self) -> bool {
        if !self.available() || self.blockstun == 0 || self.v_bars == 0 {
            return false;
        }
        self.v_bars -= 1;
        self.blockstun = 0;
        self.events.push(Sf5Event::VReversal);
        true
    }

    pub fn v_trigger(&mut self) -> bool {
        if !self.available() || self.trigger || self.v_bars < self.max_v() {
            return false;
        }
        self.v_bars = 0;
        self.trigger = true;
        self.events.push(Sf5Event::VTrigger(self.trigger_name()));
        true
    }

    pub fn v_shift(&mut self) -> bool {
        if self.revision != Revision::Sf5Ce || !self.available() || self.v_bars == 0 {
            return false;
        }
        self.v_bars -= 1;
        self.shifted = true;
        self.events.push(Sf5Event::VShift);
        true
    }

    pub fn v_shift_break(&mut self) -> bool {
        if !self.shifted {
            return false;
        }
        self.shifted = false;
        self.events.push(Sf5Event::VShiftBreak);
        true
    }

    pub fn ex_special(&mut self, name: &'static str) -> bool {
        if !self.available() || self.ex == 0 {
            return false;
        }
        self.ex -= 1;
        self.events.push(Sf5Event::Ex(name));
        true
    }

    pub fn critical_art(&mut self) -> bool {
        if !self.available() || self.ex < 3 {
            return false;
        }
        self.ex = 0;
        self.events.push(Sf5Event::CriticalArt);
        true
    }

    /// Three EX cells over the V-Gauge, which shows the V-meter toward its
    /// next bar and drains while V-Trigger is on. Nothing for a fighter this
    /// revision does not carry.
    pub fn hud(&self) -> Option<EraGauge> {
        if !self.available() {
            return None;
        }
        let mut v_gauge = GaugeRow::stock(self.max_v(), self.v_bars, Fill::of(self.v_meter, V_BAR));
        if self.trigger {
            v_gauge = v_gauge.with_state(GaugeState::Draining);
        }
        Some(EraGauge::two(GaugeRow::stock(3, self.ex, Fill::EMPTY), v_gauge))
    }
}

fn channel(fighter: TournamentFighter) -> u8 {
    TournamentFighter::ALL.iter().position(|one| *one == fighter).unwrap_or(0) as u8 & 7
}

fn cue(fighter: TournamentFighter) -> Vec<i16> {
    let base = 200 + i16::from(channel(fighter)) * 30;
    (0..32).map(|i| base + i * 15).collect()
}

fn costume_tiles(fighter: TournamentFighter, costume: Costume) -> Vec<IndexedTile> {
    let mut pixels = vec![Rgba::TRANSPARENT; 256];
    for y in 1..15 {
        for x in 3..13 {
            let hot = costume == Costume::Sf5 && fighter == TournamentFighter::Ryu && y < 6;
            let birdie = costume == Costume::Sf5 && fighter == TournamentFighter::Birdie && y < 4 && x > 5 && x < 11;
            if hot || birdie || (x + y) % 2 == 0 {
                pixels[y * 16 + x] = Rgba {
                    r: if hot { 180 } else { 70 },
                    g: 36 + y as u8 * 3,
                    b: if birdie { 90 } else { 28 },
                    a: 255,
                };
            }
        }
    }
    quantize_sheet(&pixels, 16, 16).expect("sf5 sheet")
}
