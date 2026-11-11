//! What-if SF2 and SFZ3 styles for fighters who were not in those games (`#115`, `M2`).
//!
//! The canonical era list does not change. These kits appear only when the
//! what-if filter is on. SF2 what-if revisions are Hyper Fighting and Super
//! Turbo. SFZ what-if is SFZ3 with the three ISMs. Mike's SF1+ kit keeps
//! kicks on LK, MK, and HK. His SF2 and SFZ3 kits are six punches.

use crate::era::{what_if_sf2, what_if_sfz, Loadout, Revision};
use crate::roster::TournamentFighter;
use sf_video::asset_pack::{
    encode_adpcm, quantize_sheet, simple_fm_patch, validate_adpcm, validate_fm_patch, validate_tile,
    AdpcmClip, FmPatch, IndexedTile, Rgba,
};
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

const STOCK_AT: u8 = 48;
const CUSTOM_FRAMES: u8 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Lp,
    Mp,
    Hp,
    Lk,
    Mk,
    Hk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhatIfEvent {
    Normal(&'static str),
    Special { name: &'static str, detail: u8 },
    Super { name: &'static str, damage: u16 },
    CustomCombo,
}

#[derive(Debug, Clone)]
pub struct WhatIfSession {
    pub fighter: TournamentFighter,
    pub revision: Revision,
    pub loadout: Loadout,
    stocks: u8,
    meter: u8,
    custom_left: u8,
    pub tiles: Vec<IndexedTile>,
    pub voice: AdpcmClip,
    pub patch: FmPatch,
    pub events: Vec<WhatIfEvent>,
}

impl WhatIfSession {
    pub fn new(fighter: TournamentFighter, revision: Revision, loadout: Loadout) -> Self {
        let tiles = tiles_for(fighter);
        for tile in &tiles {
            validate_tile(tile).expect("what-if tile");
        }
        let voice = encode_adpcm(&cue(fighter)).expect("what-if voice");
        validate_adpcm(&voice).expect("what-if voice");
        let patch = simple_fm_patch(channel(fighter), 0x22);
        validate_fm_patch(&patch).expect("what-if patch");
        let loadout = if revision == Revision::Sfz3 { loadout } else { Loadout::AIsm };
        Self {
            fighter,
            revision,
            loadout,
            stocks: 0,
            meter: 0,
            custom_left: 0,
            tiles,
            voice,
            patch,
            events: Vec::new(),
        }
    }

    pub fn allowed(&self) -> bool {
        match self.revision {
            Revision::Hf | Revision::St => what_if_sf2(self.fighter),
            Revision::Sfz3 => what_if_sfz(self.fighter),
            _ => false,
        }
    }

    pub fn stocks(&self) -> u8 {
        self.stocks
    }

    pub fn normal(&mut self, button: Button) -> bool {
        if !self.allowed() {
            return false;
        }
        self.events.push(WhatIfEvent::Normal(mike_or_normal(self.fighter, self.revision, button)));
        self.note_hit(false);
        true
    }

    pub fn special(&mut self, name: &str, detail: u8) -> bool {
        if !self.allowed() || !specials(self.fighter).contains(&name) {
            return false;
        }
        if name == "Fist Combination" && detail > 3 {
            return false;
        }
        if name == "Utsusemi" && detail > 2 {
            return false;
        }
        if name == "Shuriken" && detail > 2 {
            return false;
        }
        self.events.push(WhatIfEvent::Special { name: intern(name), detail });
        self.note_hit(true);
        true
    }

    pub fn super_move(&mut self) -> bool {
        if !self.allowed() {
            return false;
        }
        let Some(name) = super_name(self.fighter) else {
            return false;
        };
        if self.revision == Revision::Hf || self.stocks == 0 {
            return false;
        }
        if self.revision == Revision::Sfz3 && self.loadout == Loadout::VIsm {
            return false;
        }
        let level = if self.revision == Revision::Sfz3 { self.stocks.min(3) } else { 1 };
        if self.loadout == Loadout::XIsm && level != 3 {
            return false;
        }
        let mut damage = 80 + u16::from(level) * 40;
        if self.loadout == Loadout::XIsm {
            damage = damage.saturating_add(damage / 2);
        }
        self.stocks = 0;
        self.meter = 0;
        self.events.push(WhatIfEvent::Super { name, damage });
        true
    }

    pub fn custom_combo(&mut self) -> bool {
        if self.revision != Revision::Sfz3 || self.loadout != Loadout::VIsm || self.stocks == 0 {
            return false;
        }
        self.stocks -= 1;
        self.custom_left = CUSTOM_FRAMES;
        self.events.push(WhatIfEvent::CustomCombo);
        true
    }

    /// Super Turbo's one stock cell, or Alpha 3's level bar (drained by a
    /// Custom Combo). Nothing for Hyper Fighting or a fighter the era lacks.
    pub fn hud(&self) -> Option<EraGauge> {
        if !self.allowed() {
            return None;
        }
        match self.revision {
            Revision::St => Some(EraGauge::one(GaugeRow::stock(1, self.stocks, Fill::of(self.meter.into(), STOCK_AT.into())))),
            Revision::Sfz3 if self.custom_left > 0 => Some(EraGauge::one(
                GaugeRow::level(self.stocks, 3, Fill::of(self.custom_left.into(), CUSTOM_FRAMES.into()))
                    .with_state(GaugeState::Draining),
            )),
            Revision::Sfz3 => Some(EraGauge::one(GaugeRow::level(self.stocks, 3, Fill::of(self.stocks.into(), 3)))),
            _ => None,
        }
    }

    fn note_hit(&mut self, special: bool) {
        if self.revision == Revision::Hf || self.stocks > 0 && self.revision == Revision::St {
            return;
        }
        if self.revision == Revision::St {
            let gain = if special { 32 } else { 16 };
            self.meter = self.meter.saturating_add(gain).min(STOCK_AT);
            if self.meter >= STOCK_AT {
                self.stocks = 1;
            }
            return;
        }
        if self.revision == Revision::Sfz3 {
            self.stocks = (self.stocks + 1).min(3);
        }
    }
}

pub fn mike_or_normal(fighter: TournamentFighter, revision: Revision, button: Button) -> &'static str {
    if fighter != TournamentFighter::Mike {
        return match button {
            Button::Lp | Button::Mp | Button::Hp => "Punch",
            Button::Lk | Button::Mk | Button::Hk => "Kick",
        };
    }
    if matches!(revision, Revision::Hf | Revision::St | Revision::Sfz3) {
        return match button {
            Button::Lp | Button::Mp | Button::Hp => "Head Punch",
            Button::Lk => "Body Hook",
            Button::Mk | Button::Hk => "Uppercut",
        };
    }
    match button {
        Button::Lp | Button::Mp | Button::Hp => "Punch",
        Button::Lk | Button::Mk | Button::Hk => "Kick",
    }
}

fn specials(fighter: TournamentFighter) -> &'static [&'static str] {
    match fighter {
        TournamentFighter::Retsu => &[
            "Tobi Geri",
            "Shorinji Palm Thrust",
            "Hundred Monk Slaps",
            "Rising Dragon Knee",
            "Izuna Shoulder Throw",
        ],
        TournamentFighter::Geki => &["Shuriken", "Utsusemi", "Geki-Zan", "Izuna Claw Dive"],
        TournamentFighter::Joe => &[
            "Step High Kick",
            "Rolling Sobat",
            "Fist Combination",
            "Double Kick",
            "Sobat Kick",
            "Ground Kick",
        ],
        TournamentFighter::Mike => &["Murder Straight", "Weaving Upper", "Rolling Blow", "Ducking Sway"],
        TournamentFighter::Lee => &[
            "Open Canvas Punch",
            "Open Canvas Kick",
            "Upward Lance",
            "Cut Kick",
            "Slice Kick",
            "Stab Kick",
            "Hurricane Kick",
            "Firewater",
        ],
        TournamentFighter::Gen => &["Hyakurenko", "Gekiro"],
        TournamentFighter::Birdie => &["Bull Head", "Bull Horn"],
        TournamentFighter::Eagle => &["Manchester Black", "St. Andrew's Green"],
        TournamentFighter::Adon => &["Jaguar Kick", "Rising Jaguar"],
        _ => &[],
    }
}

fn super_name(fighter: TournamentFighter) -> Option<&'static str> {
    match fighter {
        TournamentFighter::Retsu => Some("Tenchi Shorinji Drop"),
        TournamentFighter::Geki => Some("Ansatsu Geki-Retsuzan"),
        TournamentFighter::Joe => Some("Thrash Kick"),
        TournamentFighter::Mike => Some("Crazy Rush"),
        TournamentFighter::Lee => Some("Hundred Spears"),
        TournamentFighter::Gen => Some("Zan'ei"),
        TournamentFighter::Birdie => Some("The Birdie"),
        TournamentFighter::Eagle => Some("Manchester Gold"),
        TournamentFighter::Adon => Some("Jaguar Revolver"),
        _ => None,
    }
}

fn intern(name: &str) -> &'static str {
    for fighter in TournamentFighter::ALL {
        if let Some(found) = specials(fighter).iter().copied().find(|one| *one == name) {
            return found;
        }
    }
    ""
}

fn channel(fighter: TournamentFighter) -> u8 {
    TournamentFighter::ALL.iter().position(|one| *one == fighter).unwrap_or(0) as u8 & 7
}

fn cue(fighter: TournamentFighter) -> Vec<i16> {
    let base = 140 + i16::from(channel(fighter)) * 25;
    (0..24).map(|i| base + i * 12).collect()
}

fn tiles_for(fighter: TournamentFighter) -> Vec<IndexedTile> {
    let id = channel(fighter);
    let mut pixels = vec![Rgba::TRANSPARENT; 256];
    for y in 2..14 {
        for x in 4..12 {
            if (x + y + id as usize) % 2 == 0 {
                pixels[y * 16 + x] = Rgba { r: 80 + id * 10, g: 48, b: 32, a: 255 };
            }
        }
    }
    quantize_sheet(&pixels, 16, 16).expect("what-if sheet")
}
