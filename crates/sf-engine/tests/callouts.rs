//! Ticket #220 (EH4): call-out events and the super dim at the session seam.
//!
//! The session feeds the call-out board (`GameSession::push_callouts`); a super's dim
//! there changes only the presented frame, so every input during it reaches
//! the machine exactly as without it. The session tests need the local ROM
//! set and skip without it.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_video::callouts::{regions, CalloutEvent, CalloutKind, Side};
use sf_video::era_hud::pens;
use sf_video::render::{Frame, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const DOWN: usize = 2;
const RIGHT: usize = 0;
const LEFT: usize = 1;

fn gold_in(frame: &Frame, rect: regions::Rect) -> bool {
    (rect.y..rect.y + rect.h).any(|y| (rect.x..rect.x + rect.w).any(|x| frame.argb[y * WIDTH + x] == pens::GOLD))
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
