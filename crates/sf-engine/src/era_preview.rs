//! Two era sessions under the era HUD (spec #216, `EH1`), for the
//! development launcher's preview.
//!
//! The era models are standalone combat models: no real fight carries era
//! state until project 4 moves the eras onto the 1987 fighters. The preview
//! drives one model per side with three actions, so every gauge can be seen
//! filling, spending and changing state before then. It adds no meter rules:
//! each action calls the model's own methods.
//!
//! The preview also shows the call-outs (`EH4`): each model's own events
//! become call-out events on its side, COUNTER and PUNISH COUNTER (which no
//! model emits yet) and the real fight's FIRST ATTACK, REVERSAL and PERFECT
//! come from fixtures, and each side has a stand-in fighter so the Burnout
//! grey can be seen.

use crate::alpha::AlphaSession;
use crate::config::RulesetPreset;
use crate::era::{Costume, GaugeMode, Loadout, Revision, Sf2Session};
use crate::roster::TournamentFighter;
use crate::sf1plus::Sf1PlusRules;
use crate::sf3::{Sf3Rev, Sf3Session};
use crate::sf4::{Sf4Rev, Sf4Session};
use crate::sf5::Sf5Session;
use crate::sf6::{Rush, Sf6Session};
use sf_video::callouts::{CalloutBoard, CalloutEvent, CalloutKind, Side};
use sf_video::compositor::{present, IndexedSprite, OverhaulScene, PresentMode, RomFont};
use sf_video::era_hud::{EraGauge, EraHud, GaugeArt};
use sf_video::render::{Frame, HEIGHT, WIDTH};

/// The eras the preview can run on a side. `Sf1` has no gauge.
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
    pub const ALL: [PreviewEra; 7] = [
        PreviewEra::Sf1,
        PreviewEra::St,
        PreviewEra::Alpha,
        PreviewEra::Sf3,
        PreviewEra::Sf4,
        PreviewEra::Sf5,
        PreviewEra::Sf6,
    ];

    /// The launcher's argument for this era.
    pub const fn slug(self) -> &'static str {
        match self {
            PreviewEra::Sf1 => "sf1",
            PreviewEra::St => "st",
            PreviewEra::Alpha => "alpha",
            PreviewEra::Sf3 => "sf3",
            PreviewEra::Sf4 => "sf4",
            PreviewEra::Sf5 => "sf5",
            PreviewEra::Sf6 => "sf6",
        }
    }

    pub fn from_slug(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|era| era.slug() == text)
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|era| *era == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

enum Model {
    Sf1,
    St(Box<Sf2Session>),
    Alpha(AlphaSession),
    Sf3(Sf3Session),
    Sf4(Sf4Session),
    Sf5(Sf5Session),
    Sf6(Sf6Session),
}

/// One side of the preview: Ryu in one era's model.
pub struct PreviewSide {
    era: PreviewEra,
    model: Model,
    /// The model's events already turned into call-outs.
    seen: usize,
}

impl PreviewSide {
    pub fn new(era: PreviewEra) -> Self {
        let ryu = TournamentFighter::Ryu;
        let model = match era {
            PreviewEra::Sf1 => Model::Sf1,
            PreviewEra::St => Model::St(Box::new(Sf2Session::new(
                ryu,
                TournamentFighter::Ken,
                Revision::St,
                Loadout::Standard,
                GaugeMode::EraDefault,
                Sf1PlusRules::for_preset(RulesetPreset::DefinitiveOverhaul),
            ))),
            PreviewEra::Alpha => Model::Alpha(AlphaSession::new(ryu, Revision::Sfz2, Loadout::AIsm)),
            PreviewEra::Sf3 => Model::Sf3(Sf3Session::new(ryu, Sf3Rev::ThreeS, Loadout::Art1)),
            PreviewEra::Sf4 => Model::Sf4(Sf4Session::new(ryu, Sf4Rev::Usf4, Loadout::Ultra1, false)),
            PreviewEra::Sf5 => Model::Sf5(Sf5Session::new(ryu, Revision::Sf5, Loadout::Skill1Trigger1, Costume::Classic1987)),
            PreviewEra::Sf6 => Model::Sf6(Sf6Session::new(ryu, Costume::Classic1987, false, false)),
        };
        PreviewSide { era, model, seen: 0 }
    }

    pub fn era(&self) -> PreviewEra {
        self.era
    }

    /// A landed hit, as the era gains meter from one.
    pub fn hit(&mut self) {
        match &mut self.model {
            Model::Sf1 => {}
            Model::St(s) => {
                s.play("Hadoken");
            }
            Model::Alpha(s) => s.land_hit(),
            Model::Sf3(s) => s.add_meter(20),
            Model::Sf4(s) => s.add_bars(1),
            Model::Sf5(s) => s.land_hit(false),
            Model::Sf6(s) => s.land_hit(),
        }
    }

    /// The era's super, spending what it costs.
    pub fn spend(&mut self) {
        match &mut self.model {
            Model::Sf1 => {}
            Model::St(s) => {
                s.play("Shinku Hadoken");
            }
            Model::Alpha(s) => {
                s.super_combo(s.stocks().max(1));
            }
            Model::Sf3(s) => {
                s.super_art();
            }
            Model::Sf4(s) => {
                if !s.super_combo() {
                    s.ultra();
                }
            }
            Model::Sf5(s) => {
                s.ex_special("Hadoken");
            }
            Model::Sf6(s) => {
                s.super_art(s.super_stock().max(1));
            }
        }
    }

    /// The era's other gauge action: an Alpha Custom Combo, the next SF3
    /// Super Art, SF4 damage taken (Revenge), an SF5 V-Trigger, an SF6 raw
    /// Drive Rush (or, in Burnout, the next round's refill).
    pub fn alt(&mut self) {
        match &mut self.model {
            Model::Sf1 | Model::St(_) => {}
            Model::Alpha(s) => {
                s.custom_combo();
            }
            Model::Sf3(s) => {
                let next = match s.art {
                    Loadout::Art1 => Loadout::Art2,
                    Loadout::Art2 => Loadout::Art3,
                    _ => Loadout::Art1,
                };
                *s = Sf3Session::new(s.fighter, s.revision, next);
            }
            Model::Sf4(s) => s.take_damage(50),
            Model::Sf5(s) => {
                s.v_trigger();
            }
            Model::Sf6(s) => {
                if s.burnout() {
                    s.next_round();
                } else {
                    s.drive_rush(Rush::Raw);
                    s.release_flash();
                }
            }
        }
    }

    /// The era's own call-out moment: an Alpha Counter out of blockstun, an
    /// SF5 heavy hit's Crush Counter, an SF6 perfect parry. Nothing for the
    /// eras without one.
    pub fn moment(&mut self) {
        match &mut self.model {
            Model::Alpha(s) => {
                s.suffer_block(10);
                s.alpha_counter();
            }
            Model::Sf5(s) => s.land_hit(true),
            Model::Sf6(s) => {
                s.drive_parry(0);
            }
            _ => {}
        }
    }

    /// The call-outs the model's events have earned since the last call.
    pub fn take_callouts(&mut self) -> Vec<CalloutKind> {
        let seen = self.seen;
        let (out, len): (Vec<CalloutKind>, usize) = match &self.model {
            Model::Sf1 => (Vec::new(), 0),
            Model::St(s) => {
                let events = s.events();
                (events.iter().skip(seen).filter_map(|e| s.callout(e)).collect(), events.len())
            }
            Model::Alpha(s) => (s.events.iter().skip(seen).filter_map(|e| e.callout()).collect(), s.events.len()),
            Model::Sf3(s) => (s.events.iter().skip(seen).filter_map(|e| e.callout()).collect(), s.events.len()),
            Model::Sf4(s) => (s.events.iter().skip(seen).filter_map(|e| e.callout()).collect(), s.events.len()),
            Model::Sf5(s) => (s.events.iter().skip(seen).filter_map(|e| e.callout()).collect(), s.events.len()),
            Model::Sf6(s) => (s.events.iter().skip(seen).filter_map(|e| e.callout()).collect(), s.events.len()),
        };
        self.seen = len;
        out
    }

    /// Whether the fighter is burnt out (SF6), drawn grey until it ends.
    pub fn burnt_out(&self) -> bool {
        matches!(&self.model, Model::Sf6(s) if s.burnout())
    }

    /// One frame.
    pub fn tick(&mut self) {
        if let Model::Alpha(s) = &mut self.model {
            s.tick();
        }
    }

    /// The side's era gauge, as its model reports it.
    pub fn gauge(&self) -> Option<EraGauge> {
        match &self.model {
            Model::Sf1 => None,
            Model::St(s) => s.hud(),
            Model::Alpha(s) => s.hud(),
            Model::Sf3(s) => s.hud(),
            Model::Sf4(s) => s.hud(),
            Model::Sf5(s) => s.hud(),
            Model::Sf6(s) => s.hud(),
        }
    }
}

/// The fixture call-outs, in the order the preview's fixture action cycles
/// them: the counters no model emits yet, and the real fight's round
/// call-outs.
pub const FIXTURES: [CalloutKind; 5] = [
    CalloutKind::Counter,
    CalloutKind::PunishCounter,
    CalloutKind::FirstAttack,
    CalloutKind::Reversal,
    CalloutKind::Perfect,
];

/// Both sides of the preview, and the call-outs they have earned.
pub struct EraPreview {
    pub p1: PreviewSide,
    pub p2: PreviewSide,
    board: CalloutBoard,
    fixtures: Vec<CalloutEvent>,
    next_fixture: [usize; 2],
}

impl EraPreview {
    pub fn new(p1: PreviewEra, p2: PreviewEra) -> Self {
        EraPreview {
            p1: PreviewSide::new(p1),
            p2: PreviewSide::new(p2),
            board: CalloutBoard::new(),
            fixtures: Vec::new(),
            next_fixture: [0; 2],
        }
    }

    /// The next fixture call-out on `side`, shown from the next tick.
    pub fn fixture(&mut self, side: Side) {
        let i = &mut self.next_fixture[side.index()];
        self.fixtures.push(CalloutEvent::new(side, FIXTURES[*i % FIXTURES.len()]));
        *i += 1;
    }

    /// One frame: both models, then the call-outs they and the fixtures
    /// earned since the last frame.
    pub fn tick(&mut self) {
        self.p1.tick();
        self.p2.tick();
        let mut events = Vec::new();
        for (side, model) in [(Side::P1, &mut self.p1), (Side::P2, &mut self.p2)] {
            events.extend(model.take_callouts().into_iter().map(|kind| CalloutEvent::new(side, kind)));
            self.board.set_greyed(side, model.burnt_out());
        }
        events.append(&mut self.fixtures);
        self.board.step(&events);
    }

    pub fn board(&self) -> &CalloutBoard {
        &self.board
    }

    pub fn hud(&self) -> EraHud {
        EraHud { p1: self.p1.gauge(), p2: self.p2.gauge() }
    }

    /// The HUD over a plain backdrop, captioned with both eras.
    pub fn frame(&self, art: GaugeArt, font: Option<&RomFont<'_>>) -> Frame {
        let hardware = Frame { argb: vec![0xff_18_14_28; WIDTH * HEIGHT] };
        let caption = [format!("P1 {}  P2 {}", self.p1.era().slug(), self.p2.era().slug())];
        let (p1, p2) = (stand_in(Side::P1), stand_in(Side::P2));
        present(
            PresentMode::Overhaul,
            &hardware,
            &OverhaulScene {
                hud: self.hud(),
                gauge_art: art,
                font,
                callouts: self.board.frame(),
                fighters: [&p1, &p2],
                select_lines: &caption,
                ..OverhaulScene::EMPTY
            },
        )
    }
}

/// A stand-in fighter, 32 x 64 pixels, standing in `side`'s half: the
/// preview draws no fighter art, only a palette the Burnout grey can turn.
pub fn stand_in(side: Side) -> Vec<IndexedSprite> {
    let mut palette = [0u32; 16];
    // Gi, skin, belt and hair.
    palette[1] = 0xff_e8_e0_d0;
    palette[2] = 0xff_d0_90_60;
    palette[3] = 0xff_20_20_20;
    palette[4] = 0xff_40_28_18;
    let x0 = match side {
        Side::P1 => 120,
        Side::P2 => WIDTH as i32 - 120 - 32,
    };
    let mut out = Vec::new();
    for row in 0..4 {
        for col in 0..2 {
            let mut pixels = [0u8; 256];
            for py in 0..16 {
                for px in 0..16 {
                    let (x, y) = (col * 16 + px, row * 16 + py);
                    pixels[py * 16 + px] = match y {
                        0..=3 if (10..22).contains(&x) => 4,
                        4..=13 if (10..22).contains(&x) => 2,
                        30..=32 if (6..26).contains(&x) => 3,
                        14..=63 if (6..26).contains(&x) => 1,
                        _ => 0,
                    };
                }
            }
            out.push(IndexedSprite { x: x0 + col as i32 * 16, y: 120 + row as i32 * 16, pixels, palette });
        }
    }
    out
}
