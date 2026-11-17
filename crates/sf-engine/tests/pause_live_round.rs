//! George, 2026-11-17: the pause box opens only while a round is being
//! fought (the ROM's fight loop, where the round timer runs). Start and Esc
//! do nothing through the walk-in, the round call, the knockout and the
//! victory pose. Local ROMs only; each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const DOWN: usize = 2;
const HP: usize = 2;

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

fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(idle());
    out
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn flow(s: &GameSession) -> Option<u32> {
    s.fight_machine().map(flow_position)
}

/// A Local fight (P1 Ryu, P2 Ken, Japan), just after the country is chosen.
fn local_fight(a: Assets) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    select::confirm_fighter(&mut s, with(|i| i.p1.buttons[0] = true), tap);
    select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), tap);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    s
}

/// Frame `t`'s pause key: Start and Esc by turns, pressed every other frame.
fn pause_key(t: u32, input: &mut FrameInputs) {
    match t % 4 {
        0 => input.menu.start = true,
        2 => input.menu.back = true,
        _ => {}
    }
}

#[test]
fn the_pause_box_opens_only_while_the_round_is_being_fought() {
    let Some(a) = assets() else { return };
    let mut s = local_fight(a);

    // Start and Esc through the hidden drive, the walk-in and the round call.
    let mut t = 0;
    while flow(&s) != Some(FIGHT_LOOP) {
        let mut input = idle();
        pause_key(t, &mut input);
        let out = s.step(input);
        assert_ne!(out.transition, Some(Transition::Paused), "paused before the round at frame {t}, flow {:x?}", flow(&s));
        assert_ne!(out.screen, "pause", "frame {t}");
        t += 1;
        assert!(t < 3_000, "the fight loop (at {})", s.screen());
    }
    s.step(idle());

    // In the round both keys open it.
    for key in [0, 2] {
        let mut input = idle();
        pause_key(key, &mut input);
        let out = s.step(input);
        assert_eq!((out.screen, out.transition), ("pause", Some(Transition::Paused)), "key {key}");
        s.step(idle());
        assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::Resume)));
        assert_eq!(flow(&s), Some(FIGHT_LOOP));
    }

    // P1 walks in and knocks Ken out with heavy punches.
    let mut t = 0;
    while flow(&s) == Some(FIGHT_LOOP) {
        let phase = t % 50;
        s.step(with(|i| {
            i.p1.directions[RIGHT] = phase < 20;
            i.p1.buttons[HP] = (30..32).contains(&phase);
        }));
        t += 1;
        assert!(t < 6_000, "the knockout");
    }

    // Start and Esc from the knockout to the next round's fight loop.
    let (mut t, mut seen) = (0, std::collections::BTreeSet::new());
    while flow(&s) != Some(FIGHT_LOOP) {
        seen.extend(flow(&s));
        let mut input = idle();
        pause_key(t, &mut input);
        let out = s.step(input);
        assert_ne!(out.transition, Some(Transition::Paused), "paused after the knockout at frame {t}, flow {:x?}", flow(&s));
        assert_ne!(out.screen, "pause", "frame {t}");
        t += 1;
        assert!(t < 3_000, "the second round (at {}, flows {seen:x?})", s.screen());
    }
    assert!(seen.len() >= 3, "the knockout, the victory pose and the next round's start were crossed: {seen:x?}");
    s.step(idle());
    assert_eq!(s.step(with(|i| i.menu.start = true)).transition, Some(Transition::Paused));
}
