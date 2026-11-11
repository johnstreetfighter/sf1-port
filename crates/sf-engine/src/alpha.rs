//! Street Fighter Alpha / Zero revisions and the 3-level gauge (`#104`, `O13`).
//!
//! Ryu, Ken, Adon, Sagat, Birdie, and Gen have SFZ1 through SFZ3. Eagle joins
//! at SFZ3. A-ISM spends levels 1 through 3, V-ISM spends a stock on a timed
//! Custom Combo, and X-ISM keeps a single level-3 super that hits harder.
//! Gen switches Mantis and Crane with PPP and KKK.

use crate::era::{Loadout, Revision};
use crate::roster::TournamentFighter;
use sf_video::asset_pack::{
    encode_adpcm, simple_fm_patch, validate_adpcm, validate_fm_patch, validate_tile, AdpcmClip,
    FmPatch, IndexedTile, Rgba,
};
use sf_video::callouts::CalloutKind;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

/// The gauge's top level.
const MAX_LEVEL: u8 = 3;
/// How long a Custom Combo runs.
const CUSTOM_FRAMES: u8 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stance {
    Mantis,
    Crane,
}

impl Stance {
    pub const fn name(self) -> &'static str {
        match self {
            Stance::Mantis => "Mantis",
            Stance::Crane => "Crane",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaEvent {
    Super { level: u8, name: &'static str, damage: u16 },
    AlphaCounter,
    CustomCombo { frames: u8 },
    Chain(&'static str),
    Stance(Stance),
    Normal(&'static str),
}

impl AlphaEvent {
    /// The call-out this event earns: a super, ALPHA COUNTER or CUSTOM COMBO.
    pub fn callout(self) -> Option<CalloutKind> {
        match self {
            AlphaEvent::Super { name, .. } => Some(CalloutKind::Super { name }),
            AlphaEvent::AlphaCounter => Some(CalloutKind::AlphaCounter),
            AlphaEvent::CustomCombo { .. } => Some(CalloutKind::CustomCombo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlphaSession {
    pub fighter: TournamentFighter,
    pub revision: Revision,
    pub loadout: Loadout,
    pub stance: Stance,
    stocks: u8,
    blockstun: u8,
    custom_left: u8,
    pub events: Vec<AlphaEvent>,
    pub voice: AdpcmClip,
    pub patch: FmPatch,
    pub costume: Vec<IndexedTile>,
}

impl AlphaSession {
    pub fn new(fighter: TournamentFighter, revision: Revision, loadout: Loadout) -> Self {
        let voice = encode_adpcm(&cue_pcm(fighter)).expect("alpha voice");
        validate_adpcm(&voice).expect("alpha voice");
        let patch = simple_fm_patch(channel(fighter), 0x18);
        validate_fm_patch(&patch).expect("alpha patch");
        let costume = costume_tiles(fighter);
        for tile in &costume {
            validate_tile(tile).expect("alpha costume");
        }
        Self {
            fighter,
            revision,
            loadout: if revision == Revision::Sfz3 { loadout } else { Loadout::AIsm },
            stance: Stance::Mantis,
            stocks: 0,
            blockstun: 0,
            custom_left: 0,
            events: Vec::new(),
            voice,
            patch,
            costume,
        }
    }

    pub fn stocks(&self) -> u8 {
        self.stocks
    }

    pub fn custom_left(&self) -> u8 {
        self.custom_left
    }

    pub fn x_ism(&self) -> bool {
        self.loadout == Loadout::XIsm
    }

    pub fn v_ism(&self) -> bool {
        self.loadout == Loadout::VIsm
    }

    /// One connecting hit adds a stock. X-ISM still counts to 3, and only level 3 spends it.
    pub fn land_hit(&mut self) {
        self.stocks = (self.stocks + 1).min(3);
    }

    pub fn super_combo(&mut self, level: u8) -> bool {
        if !(1..=3).contains(&level) {
            return false;
        }
        if self.v_ism() || (self.x_ism() && level != 3) || self.stocks < level {
            return false;
        }
        let Some(name) = super_name(self.fighter, self.stance, level) else {
            return false;
        };
        self.stocks -= level;
        let mut damage = 80 + u16::from(level) * 70;
        if self.x_ism() {
            damage = damage.saturating_add(damage / 2);
        }
        self.events.push(AlphaEvent::Super { level, name, damage });
        true
    }

    pub fn suffer_block(&mut self, frames: u8) {
        self.blockstun = frames;
    }

    /// Alpha Counter spends one stock and leaves blockstun.
    pub fn alpha_counter(&mut self) -> bool {
        if self.blockstun == 0 || self.stocks == 0 {
            return false;
        }
        self.stocks -= 1;
        self.blockstun = 0;
        self.events.push(AlphaEvent::AlphaCounter);
        true
    }

    /// Custom Combos exist in SFZ2, SFZ2A, and any V-ISM.
    pub fn custom_combo(&mut self) -> bool {
        let legal = matches!(self.revision, Revision::Sfz2 | Revision::Sfz2A) || self.v_ism();
        if !legal || self.stocks == 0 {
            return false;
        }
        self.stocks -= 1;
        self.custom_left = CUSTOM_FRAMES;
        self.events.push(AlphaEvent::CustomCombo { frames: CUSTOM_FRAMES });
        true
    }

    pub fn tick(&mut self) {
        if self.custom_left > 0 {
            self.custom_left -= 1;
        }
        if self.blockstun > 0 {
            self.blockstun -= 1;
        }
    }

    pub fn chain(&mut self, into: &'static str) -> bool {
        if self.custom_left == 0 {
            return false;
        }
        self.events.push(AlphaEvent::Chain(into));
        true
    }

    /// PPP selects Mantis. KKK selects Crane. Only Gen has both.
    pub fn switch_stance(&mut self, punches: bool) -> bool {
        if self.fighter != TournamentFighter::Gen {
            return false;
        }
        self.stance = if punches { Stance::Mantis } else { Stance::Crane };
        self.events.push(AlphaEvent::Stance(self.stance));
        true
    }

    pub fn normal(&mut self, name: &'static str) -> bool {
        let legal = match (self.fighter, self.stance, name) {
            (TournamentFighter::Gen, Stance::Mantis, "Mantis Jab" | "Hyakurenko") => true,
            (TournamentFighter::Gen, Stance::Crane, "Crane Jab" | "Gekiro") => true,
            (TournamentFighter::Gen, _, _) => false,
            (_, _, "stand LP" | "stand HP") => true,
            _ => false,
        };
        if !legal {
            return false;
        }
        self.events.push(AlphaEvent::Normal(name));
        true
    }

    /// The level bar: the stock count beside it, filled a whole level at a
    /// time. A Custom Combo drains it with the combo's timer.
    pub fn hud(&self) -> Option<EraGauge> {
        let row = if self.custom_left > 0 {
            GaugeRow::level(self.stocks, MAX_LEVEL, Fill::of(self.custom_left.into(), CUSTOM_FRAMES.into()))
                .with_state(GaugeState::Draining)
        } else {
            GaugeRow::level(self.stocks, MAX_LEVEL, Fill::of(self.stocks.into(), MAX_LEVEL.into()))
        };
        Some(EraGauge::one(row))
    }
}

pub fn super_name(fighter: TournamentFighter, stance: Stance, level: u8) -> Option<&'static str> {
    let table: &[&str] = match fighter {
        TournamentFighter::Ryu => &["Shinku Hadoken", "Shin Shoryuken", "Shun Goku Satsu"],
        TournamentFighter::Ken => &["Shoryureppa", "Shinryuken", "Shippu Jinraikyaku"],
        TournamentFighter::Sagat => &["Tiger Genocide", "Tiger Raid", "Tiger Cannon"],
        TournamentFighter::Adon => &["Jaguar Revolver", "Jaguar Varied Assault", "Jaguar Thousand"],
        TournamentFighter::Birdie => &["Murderer Chain", "Bull Revenger", "The Birdie"],
        TournamentFighter::Eagle => &["Manchester Gold", "Canterbury Blue", "Union Jack"],
        TournamentFighter::Gen => match stance {
            Stance::Mantis => &["Zan'ei", "Zan'ei", "Shitenshuu"],
            Stance::Crane => &["Jakorifu", "Jakorifu", "Kouga"],
        },
        _ => return None,
    };
    table.get(level as usize - 1).copied()
}

fn channel(fighter: TournamentFighter) -> u8 {
    TournamentFighter::ALL.iter().position(|one| *one == fighter).unwrap_or(0) as u8 & 7
}

fn cue_pcm(fighter: TournamentFighter) -> Vec<i16> {
    let base = 300 + i16::from(channel(fighter)) * 40;
    (0..24).map(|i| base + i * 20).collect()
}

fn costume_tiles(fighter: TournamentFighter) -> Vec<IndexedTile> {
    let id = channel(fighter);
    let mut pixels = vec![Rgba::TRANSPARENT; 256];
    for y in 2..14 {
        for x in 4..12 {
            let mohawk = fighter == TournamentFighter::Birdie && y < 5 && x > 6 && x < 10;
            let young = fighter == TournamentFighter::Ryu && y < 6 && (x == 4 || x == 11);
            if mohawk || young || (x + y) % 2 == 0 {
                pixels[y * 16 + x] = Rgba {
                    r: 90 + id * 12,
                    g: 40 + y as u8 * 4,
                    b: 36,
                    a: 255,
                };
            }
        }
    }
    sf_video::asset_pack::quantize_sheet(&pixels, 16, 16).expect("alpha costume")
}
