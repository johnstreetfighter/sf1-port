//! The development launcher's Era HUD preview runs a native local fight.

use std::sync::Arc;

use crate::config::MatchConfig;
use crate::data_bank::DataBank;
use crate::era::{Loadout, Revision, StylePick};
use crate::front::cabinet::Players;
use crate::front::fight::{Fight, FightDips, FightEvent, FightFrame, FightKind, FightSetup};
use crate::front::PlayerInputs;
use crate::roster::{PlayMode, TournamentFighter};
use sf_video::callouts::{CalloutBoard, CalloutEvent, CalloutKind, Side};
use sf_video::era_hud::{EraHud, GaugeArt};
use sf_video::render::Frame;
use sf_video::romset::Assets;

/// The launcher's existing era arguments and their default style picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewEra {
    Sf1,
    St,
    Alpha,
    Sf3,
    Sf4,
    Sf5,
    Sf6,
}

impl PreviewEra {
    pub const ALL: [Self; 7] = [
        Self::Sf1,
        Self::St,
        Self::Alpha,
        Self::Sf3,
        Self::Sf4,
        Self::Sf5,
        Self::Sf6,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Sf1 => "sf1",
            Self::St => "st",
            Self::Alpha => "alpha",
            Self::Sf3 => "sf3",
            Self::Sf4 => "sf4",
            Self::Sf5 => "sf5",
            Self::Sf6 => "sf6",
        }
    }

    pub fn from_slug(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|era| era.slug() == text)
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|era| *era == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    fn style(self) -> StylePick {
        let (revision, loadout) = match self {
            Self::Sf1 => (Revision::Arcade1987, Loadout::Standard),
            Self::St => (Revision::St, Loadout::Standard),
            Self::Alpha => (Revision::Sfz3, Loadout::AIsm),
            Self::Sf3 => (Revision::Sf3ThreeS, Loadout::Art1),
            Self::Sf4 => (Revision::Usf4, Loadout::Ultra1),
            Self::Sf5 => (Revision::Sf5, Loadout::Skill1Trigger1),
            Self::Sf6 => (Revision::Sf6, Loadout::Standard),
        };
        StylePick::for_revision(revision, loadout)
    }
}

/// Explicit developer fixtures to inspect the call-out layout and effects.
pub const FIXTURES: [CalloutKind; 7] = [
    CalloutKind::Counter,
    CalloutKind::PunishCounter,
    CalloutKind::FirstAttack,
    CalloutKind::Reversal,
    CalloutKind::Perfect,
    CalloutKind::AlphaCounter,
    CalloutKind::Super {
        name: "SHINKU HADOKEN",
    },
];

pub struct EraPreview {
    fight: Fight,
    data: Arc<DataBank>,
    next_fixture: [usize; 2],
}

impl EraPreview {
    pub fn new(assets: &Assets, p1: PreviewEra, p2: PreviewEra) -> Result<Self, FightEvent> {
        let data = Arc::new(DataBank::build(&assets.maincpu, &[]));
        Ok(Self {
            fight: Self::start(assets, &data, [p1, p2])?,
            data,
            next_fixture: [0; 2],
        })
    }

    fn start(
        assets: &Assets,
        data: &Arc<DataBank>,
        eras: [PreviewEra; 2],
    ) -> Result<Fight, FightEvent> {
        let mut config = MatchConfig::default();
        config.roster.mode = PlayMode::Versus;
        config.roster.p1 = TournamentFighter::Ryu;
        config.roster.p2 = TournamentFighter::Ken;
        let setup = FightSetup {
            kind: FightKind::Local,
            players: Players::Two,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        };
        let mut fight =
            Fight::start_banked(&assets.maincpu, setup, Some(0), None, Some(data.clone()))?;
        fight.set_selected_styles(eras.map(|era| Some(era.style())));
        Ok(fight)
    }

    pub fn reset(
        &mut self,
        assets: &Assets,
        p1: PreviewEra,
        p2: PreviewEra,
    ) -> Result<(), FightEvent> {
        let mut fight = Self::start(assets, &self.data, [p1, p2])?;
        fight.set_gauge_art(self.fight.gauge_art());
        self.fight = fight;
        self.next_fixture = [0; 2];
        Ok(())
    }

    /// Queue the next explicit call-out on this side through the native HUD.
    pub fn fixture(&mut self, side: Side) {
        let i = &mut self.next_fixture[side.index()];
        self.fight
            .push_callouts(&[CalloutEvent::new(side, FIXTURES[*i])]);
        *i = (*i + 1) % FIXTURES.len();
    }

    pub fn step(&mut self, assets: &Assets, [p1, p2]: [PlayerInputs; 2]) -> FightFrame {
        self.fight.step(assets, p1, p2, false)
    }

    pub fn set_gauge_art(&mut self, art: GaugeArt) {
        self.fight.set_gauge_art(art);
    }

    pub fn fight(&self) -> &Fight {
        &self.fight
    }

    pub fn board(&self) -> &CalloutBoard {
        self.fight.callouts()
    }

    pub fn hud(&self) -> EraHud {
        self.fight.era_hud()
    }

    pub fn frame(&self, assets: &Assets) -> Frame {
        self.fight.render(assets)
    }
}
