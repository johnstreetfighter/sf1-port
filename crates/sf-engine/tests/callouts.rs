//! Ticket #220 (EH4): the era models' call-out events, the dev preview's
//! call-outs, and the super dim at the front end's session seam.
//!
//! The era models are standalone: each reports the call-outs its own events
//! earn, and the preview shows them. In a real fight the call-out board is
//! fed through the session (`GameSession::push_callouts`); a super's dim
//! there changes only the presented frame, so every input during it reaches
//! the machine exactly as without it. The session tests need the local ROM
//! set and skip without it.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::alpha::{AlphaEvent, AlphaSession};
use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{Costume, GaugeMode, Loadout, Revision, Sf2Session};
use sf_engine::era_preview::{EraPreview, PreviewEra, FIXTURES};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::roster::TournamentFighter;
use sf_engine::sf1plus::Sf1PlusRules;
use sf_engine::sf3::{Sf3Rev, Sf3Session};
use sf_engine::sf4::{Sf4Rev, Sf4Session};
use sf_engine::sf5::{Sf5Event, Sf5Session};
use sf_engine::sf6::{Sf6Event, Sf6Session};
use sf_video::callouts::{regions, CalloutEvent, CalloutKind, Side};
use sf_video::era_hud::{pens, GaugeArt, GaugeRow, GaugeState};
use sf_video::render::{Frame, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const DOWN: usize = 2;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const RYU: TournamentFighter = TournamentFighter::Ryu;

#[test]
fn each_era_model_reports_the_call_outs_its_events_earn() {
    // Alpha: a super, ALPHA COUNTER and CUSTOM COMBO.
    let mut alpha = AlphaSession::new(RYU, Revision::Sfz2, Loadout::AIsm);
    for _ in 0..3 {
        alpha.land_hit();
    }
    assert!(alpha.super_combo(1));
    alpha.suffer_block(10);
    assert!(alpha.alpha_counter());
    assert!(alpha.custom_combo());
    let got: Vec<_> = alpha.events.iter().filter_map(|e| e.callout()).collect();
    assert_eq!(
        got,
        [CalloutKind::Super { name: "Shinku Hadoken" }, CalloutKind::AlphaCounter, CalloutKind::CustomCombo]
    );
    assert_eq!(AlphaEvent::Normal("stand LP").callout(), None);

    // SF3: a Super Art.
    let mut sf3 = Sf3Session::new(RYU, Sf3Rev::ThreeS, Loadout::Art1);
    sf3.add_meter(500);
    assert!(sf3.super_art());
    let got: Vec<_> = sf3.events.iter().filter_map(|e| e.callout()).collect();
    assert_eq!(got, [CalloutKind::Super { name: "Shinku Hadoken" }]);

    // SF4: a Super and an Ultra, each a super.
    let mut sf4 = Sf4Session::new(RYU, Sf4Rev::Usf4, Loadout::Ultra1, false);
    sf4.add_bars(4);
    assert!(sf4.super_combo());
    sf4.take_damage(400);
    assert!(sf4.ultra());
    let got: Vec<_> = sf4.events.iter().filter_map(|e| e.callout()).collect();
    assert_eq!(
        got,
        [CalloutKind::Super { name: "Shinku Hadoken" }, CalloutKind::Super { name: "Metsu Hadoken" }]
    );

    // SF5: CRUSH COUNTER from a heavy hit only, and a Critical Art.
    let mut sf5 = Sf5Session::new(RYU, Revision::Sf5, Loadout::Skill1Trigger1, Costume::Classic1987);
    sf5.land_hit(false);
    sf5.land_hit(true);
    sf5.land_hit(false);
    assert!(sf5.critical_art());
    let got: Vec<_> = sf5.events.iter().filter_map(|e| e.callout()).collect();
    assert_eq!(got, [CalloutKind::CrushCounter, CalloutKind::Super { name: "Critical Art" }]);
    assert_eq!(Sf5Event::VReversal.callout(), None);

    // SF6: a Super Art, PERFECT PARRY (not a late parry) and BURNOUT.
    let mut sf6 = Sf6Session::new(RYU, Costume::Classic1987, false, false);
    sf6.land_hit();
    assert!(sf6.super_art(1));
    assert!(sf6.drive_parry(5));
    assert!(sf6.drive_parry(0));
    while !sf6.burnout() {
        sf6.tick_parry();
    }
    let got: Vec<_> = sf6.events.iter().filter_map(|e| e.callout()).collect();
    assert_eq!(
        got,
        [CalloutKind::Super { name: "Shinku Hadoken" }, CalloutKind::PerfectParry, CalloutKind::Burnout]
    );
    assert_eq!(Sf6Event::Parry { perfect: false }.callout(), None);

    // ST: a Super Combo, reported as its start; a special earns nothing.
    let mut st = Sf2Session::new(
        RYU,
        TournamentFighter::Ken,
        Revision::St,
        Loadout::Standard,
        GaugeMode::EraDefault,
        Sf1PlusRules::for_preset(RulesetPreset::DefinitiveOverhaul),
    );
    for _ in 0..3 {
        st.play("Hadoken");
    }
    assert!(st.play("Shinku Hadoken"));
    let got: Vec<_> = st.events().iter().filter_map(|e| st.callout(e)).collect();
    assert_eq!(got, [CalloutKind::Super { name: "Shinku Hadoken" }]);
}

#[test]
fn every_model_super_name_fits_its_move_name_line() {
    use sf_engine::alpha::{super_name, Stance};
    for fighter in TournamentFighter::ALL {
        for stance in [Stance::Mantis, Stance::Crane] {
            for level in 1..=3 {
                if let Some(name) = super_name(fighter, stance, level) {
                    assert!(name.len() <= regions::LINE_LETTERS, "{name}");
                }
            }
        }
    }
}

fn gold_in(frame: &Frame, rect: regions::Rect) -> bool {
    (rect.y..rect.y + rect.h).any(|y| (rect.x..rect.x + rect.w).any(|x| frame.argb[y * WIDTH + x] == pens::GOLD))
}

#[test]
fn the_dev_preview_shows_the_call_outs() {
    let Some(assets) = assets() else { return };
    let font = sf_video::compositor::RomFont::new(&assets.gfx4, &assets.maincpu);
    let mut preview = EraPreview::new(PreviewEra::Sf6, PreviewEra::Alpha);
    preview.p1.moment();
    preview.p2.hit();
    preview.p2.moment();
    preview.tick();
    let shown = preview.board().frame();
    assert_eq!(shown.side(Side::P1).words[0], Some(CalloutKind::PerfectParry));
    assert_eq!(shown.side(Side::P2).words[0], Some(CalloutKind::AlphaCounter));
    assert_eq!(shown.side(Side::P2).flash, Some(true), "the Alpha Counter flashes the gauge");
    let frame = preview.frame(GaugeArt::Ruled, Some(&font));
    assert!(gold_in(&frame, regions::word_slot(Side::P1, 0)));
    assert!(gold_in(&frame, regions::word_slot(Side::P2, 0)));

    // A super: SUPER, its name and the dim; the fixtures cycle on.
    preview.p1.hit();
    preview.p1.spend();
    preview.fixture(Side::P2);
    preview.fixture(Side::P2);
    preview.tick();
    let shown = preview.board().frame();
    assert!(shown.dim);
    assert_eq!(shown.side(Side::P1).words[1], Some(CalloutKind::Super { name: "Shinku Hadoken" }));
    assert_eq!(shown.side(Side::P1).move_name, Some("Shinku Hadoken"));
    assert_eq!(shown.side(Side::P2).words[1..3], [Some(FIXTURES[0]), Some(FIXTURES[1])]);
    let frame = preview.frame(GaugeArt::Ruled, Some(&font));
    assert!(gold_in(&frame, regions::move_name_line(Side::P1)));

    // Burnout: BURNOUT, and the fighter and gauge grey until the next round.
    let mut preview = EraPreview::new(PreviewEra::Sf6, PreviewEra::Sf1);
    for _ in 0..6 {
        preview.p1.alt();
    }
    preview.tick();
    let shown = preview.board().frame();
    assert!(shown.side(Side::P1).words.contains(&Some(CalloutKind::Burnout)));
    assert!(shown.side(Side::P1).greyed);
    assert!(matches!(preview.hud().p1.unwrap().main, GaugeRow::Stock { state: GaugeState::Greyed, .. }));
    let frame = preview.frame(GaugeArt::Ruled, Some(&font));
    let gi = frame.argb[160 * WIDTH + 130];
    assert_eq!(gi, sf_video::callouts::grey(0xff_e8_e0_d0), "the stand-in fighter is grey");
    preview.p1.alt();
    preview.tick();
    assert!(!preview.board().frame().side(Side::P1).greyed, "the grey ends with Burnout");
}

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn tap(s: &mut GameSession, input: FrameInputs) {
    s.step(input);
    s.step(idle());
}

/// A Local fight under `preset`, stopped at the first fight loop.
fn local_fight(preset: RulesetPreset) -> Option<GameSession> {
    let a = assets()?;
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut s, with(|i| i.menu.confirm = true));
    tap(&mut s, with(|i| i.menu.confirm = true));
    select::confirm_fighter(&mut s, with(|i| i.p1.buttons[0] = true), tap);
    select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), tap);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    Some(s)
}

/// Frame `t` of a script that walks both fighters in and has them attack,
/// all inside the dim.
fn script(t: usize) -> FrameInputs {
    with(|i| {
        i.p1.directions[RIGHT] = t < 20;
        i.p2.directions[LEFT] = t < 16;
        i.p1.buttons[0] = t == 22;
        i.p2.buttons[2] = t == 26;
        i.p1.buttons[4] = (34..36).contains(&t);
        i.p2.directions[DOWN] = (30..40).contains(&t);
    })
}

fn supers() -> [CalloutEvent; 2] {
    [
        CalloutEvent::new(Side::P1, CalloutKind::Super { name: "Shinku Hadoken" }),
        CalloutEvent::new(Side::P2, CalloutKind::Super { name: "Shippu Jinraikyaku" }),
    ]
}

fn half(color: u32) -> u32 {
    0xff00_0000 | ((color >> 1) & 0x007f_7f7f)
}

#[test]
fn inputs_during_a_super_dim_are_never_eaten_at_the_session_seam() {
    let Some(start) = local_fight(RulesetPreset::DefinitiveOverhaul) else { return };
    let mut plain = start.clone();
    let mut dimmed = start.clone();
    let mut still = start;
    dimmed.push_callouts(&supers());
    let mut moved = false;
    for t in 0..100 {
        let a = plain.step(script(t));
        let b = dimmed.step(script(t));
        still.step(idle());
        let (ram_a, ram_b) = (plain.fight_machine().unwrap().work_ram(), dimmed.fight_machine().unwrap().work_ram());
        assert!(ram_a == ram_b, "frame {t}: the dim changed the machine");
        moved |= ram_a != still.fight_machine().unwrap().work_ram();
        let dim = dimmed.fight().unwrap().callouts().frame().dim;
        assert_eq!(dim, t < sf_video::callouts::SUPER_DIM_FRAMES as usize, "frame {t}");
        if dim {
            // The stage dims one level; the HUD's text layer and the sprites do not.
            let mut kept = 0;
            let mut halved = 0;
            for (pa, pb) in a.frame.argb.iter().zip(&b.frame.argb).take(60 * WIDTH) {
                assert!(pb == pa || *pb == half(*pa), "frame {t}: the health block dims only by the stage");
                if pb == pa && *pa != half(*pa) {
                    kept += 1;
                } else if pb != pa {
                    halved += 1;
                }
            }
            assert!(kept > 0 && halved > 0, "frame {t}: {kept} kept, {halved} dimmed");
            for side in Side::BOTH {
                let line = regions::move_name_line(side);
                assert!(gold_in(&b.frame, line), "frame {t}: {side:?}'s move name");
            }
        }
        if t >= sf_video::callouts::WORD_FRAMES as usize {
            assert!(a.frame.argb == b.frame.argb, "frame {t}: the call-outs have gone");
        }
    }
    assert!(moved, "the script's inputs moved the fight");
}

#[test]
fn the_faithful_1987_preset_never_shows_call_outs() {
    let Some(start) = local_fight(RulesetPreset::Faithful1987) else { return };
    let mut plain = start.clone();
    let mut pushed = start;
    pushed.push_callouts(&supers());
    for t in 0..20 {
        let a = plain.step(script(t));
        let b = pushed.step(script(t));
        assert!(a.frame.argb == b.frame.argb, "frame {t}");
    }
}
