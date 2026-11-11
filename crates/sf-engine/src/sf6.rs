//! Street Fighter 6 style, Drive gauge, and Super Arts (`#108`, `O17`).
//!
//! Ryu, Ken, and Retsu start each round with six Drive bars. The three-level
//! Super Art stock stays when the round changes. Overdrive costs two bars.
//! Drive Parry spends half a bar to start and recovers Drive on a successful
//! parry. A raw neutral Drive Rush is one bar unless `disable_raw_drive_rush`
//! is on. Cancel Drive Rush stays at three bars. Empty Drive is Burnout.

use crate::era::Costume;
use crate::roster::TournamentFighter;
use sf_video::asset_pack::{
    encode_adpcm, quantize_sheet, simple_fm_patch, validate_adpcm, validate_fm_patch, validate_tile,
    AdpcmClip, FmPatch, IndexedTile, Rgba,
};
use sf_video::callouts::CalloutKind;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

const BAR: u8 = 2;
const FULL: u8 = 12;
const MAX_SUPER: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sf6Event {
    Special { name: &'static str, damage: u16 },
    Overdrive(&'static str),
    Parry { perfect: bool },
    DriveRush { kind: Rush },
    DriveImpact { armor: u8 },
    DriveReversal,
    Super { level: u8, name: &'static str },
    Burnout,
    WallSplat,
    Followup(&'static str),
}

impl Sf6Event {
    /// The call-out this event earns: a Super Art, PERFECT PARRY or BURNOUT.
    /// Burnout's grey follows [`Sf6Session::burnout`] for as long as it lasts.
    pub fn callout(self) -> Option<CalloutKind> {
        match self {
            Sf6Event::Super { name, .. } => Some(CalloutKind::Super { name }),
            Sf6Event::Parry { perfect: true } => Some(CalloutKind::PerfectParry),
            Sf6Event::Burnout => Some(CalloutKind::Burnout),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rush {
    Parry,
    Raw,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct Sf6Session {
    pub fighter: TournamentFighter,
    pub costume: Costume,
    drive: u8,
    super_stock: u8,
    parry: Option<bool>,
    blocking: bool,
    armor: u8,
    jinrai: bool,
    renkiko: bool,
    flash: u8,
    buffered: bool,
    pub disable_raw_drive_rush: bool,
    pub disable_freeze_input_lockout: bool,
    pub tiles: Vec<IndexedTile>,
    pub voice: AdpcmClip,
    pub patch: FmPatch,
    pub events: Vec<Sf6Event>,
}

impl Sf6Session {
    pub fn new(
        fighter: TournamentFighter,
        costume: Costume,
        disable_raw_drive_rush: bool,
        disable_freeze_input_lockout: bool,
    ) -> Self {
        let tiles = costume_tiles(fighter, costume);
        for tile in &tiles {
            validate_tile(tile).expect("sf6 costume");
        }
        let voice = encode_adpcm(&cue(fighter)).expect("sf6 voice");
        validate_adpcm(&voice).expect("sf6 voice");
        let patch = simple_fm_patch(channel(fighter), 0x24);
        validate_fm_patch(&patch).expect("sf6 patch");
        Self {
            fighter,
            costume,
            drive: FULL,
            super_stock: 0,
            parry: None,
            blocking: false,
            armor: 0,
            jinrai: false,
            renkiko: false,
            flash: 0,
            buffered: false,
            disable_raw_drive_rush,
            disable_freeze_input_lockout,
            tiles,
            voice,
            patch,
            events: Vec::new(),
        }
    }

    pub fn available(&self) -> bool {
        matches!(
            self.fighter,
            TournamentFighter::Ryu | TournamentFighter::Ken | TournamentFighter::Retsu
        )
    }

    pub fn drive_bars(&self) -> u8 {
        self.drive / BAR
    }

    pub fn super_stock(&self) -> u8 {
        self.super_stock
    }

    pub fn burnout(&self) -> bool {
        self.drive == 0
    }

    /// Burnout adds four frames of blockstun.
    pub fn blockstun(&self, base: u8) -> u8 {
        if self.burnout() {
            base.saturating_add(4)
        } else {
            base
        }
    }

    /// Burnout chips on a blocked special. A normal block chips nothing.
    pub fn blocked_special_chip(&self, damage: u16) -> u16 {
        if self.burnout() {
            damage / 2
        } else {
            0
        }
    }

    /// A perfect parry (2 frames or fewer) halves the hit. A late parry takes a fifth off.
    pub fn scaled_damage(&self, damage: u16) -> u16 {
        match self.parry {
            Some(true) => damage / 2,
            Some(false) => damage * 4 / 5,
            None => damage,
        }
    }

    pub fn special(&mut self, name: &'static str) -> bool {
        if !self.owns(name) {
            return false;
        }
        let mut damage = 40;
        if self.renkiko {
            damage += 16;
            self.renkiko = false;
        }
        if name == "Jinrai Kick" {
            self.jinrai = true;
        }
        if name == "Renkiko" {
            self.renkiko = true;
        }
        self.events.push(Sf6Event::Special { name, damage });
        self.begin_flash();
        true
    }

    /// Jinrai Kick follow-ups. They only come out after Jinrai itself.
    pub fn jinrai_followup(&mut self, name: &'static str) -> bool {
        if self.fighter != TournamentFighter::Ken || !self.jinrai {
            return false;
        }
        if !matches!(name, "Low" | "Mid" | "Overhead") {
            return false;
        }
        self.jinrai = false;
        self.events.push(Sf6Event::Followup(name));
        true
    }

    pub fn overdrive(&mut self, name: &'static str) -> bool {
        if !self.owns(name) || self.drive < 4 {
            return false;
        }
        self.drive -= 4;
        self.events.push(Sf6Event::Overdrive(name));
        self.begin_flash();
        self.note_empty();
        true
    }

    /// Age is how old the strike is. Two frames or fewer is a perfect parry.
    /// The startup spends half a bar. A successful parry puts Drive back.
    pub fn drive_parry(&mut self, age: u8) -> bool {
        if !self.available() || self.drive < 1 {
            return false;
        }
        self.drive -= 1;
        let perfect = age <= 2;
        self.recover(if perfect { 4 } else { 2 });
        self.parry = Some(perfect);
        self.events.push(Sf6Event::Parry { perfect });
        self.begin_flash();
        self.note_empty();
        true
    }

    pub fn tick_parry(&mut self) {
        if self.parry.is_some() && self.drive > 0 {
            self.drive -= 1;
            self.note_empty();
        }
    }

    /// A blocked hit puts half a bar back, including the first step out of Burnout.
    pub fn block(&mut self) {
        self.blocking = true;
        self.recover(1);
    }

    pub fn drive_rush(&mut self, kind: Rush) -> bool {
        if !self.available() {
            return false;
        }
        let cost = match kind {
            Rush::Cancel => 6,
            Rush::Parry => {
                if self.parry.is_none() {
                    return false;
                }
                2
            }
            Rush::Raw => {
                if self.disable_raw_drive_rush {
                    return false;
                }
                2
            }
        };
        if self.drive < cost {
            return false;
        }
        self.drive -= cost;
        if kind == Rush::Parry {
            self.parry = None;
        }
        self.events.push(Sf6Event::DriveRush { kind });
        self.begin_flash();
        self.note_empty();
        true
    }

    /// Two hits of armor. A corner, or a defender in Burnout, wall-splats.
    pub fn drive_impact(&mut self, corner: bool, defender_burnout: bool) -> bool {
        if !self.available() || self.drive < 2 {
            return false;
        }
        self.drive -= 2;
        self.armor = 2;
        self.events.push(Sf6Event::DriveImpact { armor: 2 });
        if corner || defender_burnout {
            self.events.push(Sf6Event::WallSplat);
        }
        self.begin_flash();
        self.note_empty();
        true
    }

    pub fn absorb(&mut self) -> bool {
        if self.armor == 0 {
            return false;
        }
        self.armor -= 1;
        true
    }

    pub fn drive_reversal(&mut self) -> bool {
        if !self.available() || !self.blocking || self.drive < 4 {
            return false;
        }
        self.drive -= 4;
        self.blocking = false;
        self.events.push(Sf6Event::DriveReversal);
        self.begin_flash();
        self.note_empty();
        true
    }

    pub fn super_art(&mut self, level: u8) -> bool {
        if !self.available() || !(1..=3).contains(&level) || self.super_stock < level {
            return false;
        }
        let Some(name) = super_name(self.fighter, level) else {
            return false;
        };
        self.super_stock -= level;
        self.events.push(Sf6Event::Super { level, name });
        self.begin_flash();
        true
    }

    pub fn land_hit(&mut self) {
        if self.available() {
            self.super_stock = (self.super_stock + 1).min(MAX_SUPER);
        }
    }

    /// Drive refills. The Super Art stock stays.
    pub fn next_round(&mut self) {
        self.drive = FULL;
        self.parry = None;
        self.armor = 0;
        self.blocking = false;
        self.jinrai = false;
        self.flash = 0;
    }

    /// An attack pressed during a Drive or Super flash. With the lockout
    /// toggle on, the press is kept. With it off, the press is dropped.
    pub fn feed(&mut self, attack: bool) -> bool {
        if self.flash == 0 {
            return attack;
        }
        if self.disable_freeze_input_lockout && attack {
            self.buffered = true;
        }
        false
    }

    pub fn release_flash(&mut self) -> bool {
        self.flash = 0;
        let kept = self.buffered && self.disable_freeze_input_lockout;
        self.buffered = false;
        kept
    }

    /// Six Drive cells, half bars shown, greyed in Burnout; the Super Art
    /// level bar under them. Nothing for a fighter without SF6.
    pub fn hud(&self) -> Option<EraGauge> {
        if !self.available() {
            return None;
        }
        let mut drive = GaugeRow::stock(FULL / BAR, self.drive / BAR, Fill::of((self.drive % BAR).into(), BAR.into()));
        if self.burnout() {
            drive = drive.with_state(GaugeState::Greyed);
        }
        let super_art = GaugeRow::level(self.super_stock, MAX_SUPER, Fill::of(self.super_stock.into(), MAX_SUPER.into()));
        Some(EraGauge::two(drive, super_art))
    }

    fn owns(&self, name: &str) -> bool {
        self.available() && kit(self.fighter).contains(&name)
    }

    fn recover(&mut self, points: u8) {
        self.drive = self.drive.saturating_add(points).min(FULL);
    }

    fn begin_flash(&mut self) {
        self.flash = 8;
    }

    fn note_empty(&mut self) {
        if self.drive == 0 && !self.events.iter().any(|event| matches!(event, Sf6Event::Burnout)) {
            self.events.push(Sf6Event::Burnout);
        }
    }
}

fn kit(fighter: TournamentFighter) -> &'static [&'static str] {
    match fighter {
        TournamentFighter::Ryu => &["Hashogeki", "Denjin Charge"],
        TournamentFighter::Ken => &["Jinrai Kick", "Dragonlash Kick", "Quick Dash"],
        TournamentFighter::Retsu => &["Kiai Shot", "Tobigeri", "Renkiko"],
        _ => &[],
    }
}

fn super_name(fighter: TournamentFighter, level: u8) -> Option<&'static str> {
    let table: &[&str] = match fighter {
        TournamentFighter::Ryu => &["Shinku Hadoken", "Shin Hashogeki", "Shin Shoryuken"],
        TournamentFighter::Ken => &["Dragonlash Flame", "Shippu Jinraikyaku", "Shinryu Reppa"],
        TournamentFighter::Retsu => &["Super Art I", "Super Art II", "Super Art III"],
        _ => return None,
    };
    table.get(level as usize - 1).copied()
}

fn channel(fighter: TournamentFighter) -> u8 {
    TournamentFighter::ALL.iter().position(|one| *one == fighter).unwrap_or(0) as u8 & 7
}

fn cue(fighter: TournamentFighter) -> Vec<i16> {
    let base = 240 + i16::from(channel(fighter)) * 20;
    (0..32).map(|i| base + i * 12).collect()
}

fn costume_tiles(fighter: TournamentFighter, costume: Costume) -> Vec<IndexedTile> {
    let mut pixels = vec![Rgba::TRANSPARENT; 256];
    for y in 1..15 {
        for x in 3..13 {
            let beard = costume == Costume::Sf6 && fighter == TournamentFighter::Ryu && y > 9;
            let jacket = costume == Costume::Sf6 && fighter == TournamentFighter::Ken && x < 6;
            if beard || jacket || (x + y) % 2 == 0 {
                pixels[y * 16 + x] = Rgba {
                    r: if jacket { 40 } else { 150 },
                    g: if beard { 90 } else { 48 },
                    b: 32,
                    a: 255,
                };
            }
        }
    }
    quantize_sheet(&pixels, 16, 16).expect("sf6 sheet")
}
