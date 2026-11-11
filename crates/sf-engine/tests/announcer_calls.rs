//! Ticket #224 (EH8): what the overhaul fight asks the announcer for, at
//! the front end's session seam. Every call-out event earns its word's line
//! (CPU sides like human ones, so a line carries no side), a finished combo
//! of five hits and up earns its count, and the Faithful 1987 preset asks
//! for nothing. The lines play on the host's third chip (`sf-audio`'s
//! announcer tests). Needs the local ROM set; skips without it.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_video::announcer::AnnouncerLine;
use sf_video::callouts::{CalloutEvent, CalloutKind, Side};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const DOWN: usize = 2;

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
    assert!(s.step(input).announcements.is_empty(), "nothing is called off the fight");
    assert!(s.step(idle()).announcements.is_empty(), "nothing is called off the fight");
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
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    tap(&mut s, with(|i| i.p2.buttons[0] = true));
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        assert!(s.step(idle()).announcements.is_empty());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    Some(s)
}

/// Every call-out on both sides at once.
fn everything() -> Vec<CalloutEvent> {
    Side::BOTH
        .into_iter()
        .flat_map(|side| CalloutKind::all("SHINKU HADOKEN").map(|kind| CalloutEvent::new(side, kind)))
        .collect()
}

#[test]
fn an_overhaul_fight_asks_for_every_call_out_and_finished_combos_from_five() {
    let Some(mut s) = local_fight(RulesetPreset::DefinitiveOverhaul) else { return };
    let events = everything();
    s.push_callouts(&events);
    s.finish_combo(4);
    s.finish_combo(7);
    let out = s.step(idle());
    let mut expected: Vec<AnnouncerLine> = events.iter().filter_map(|e| AnnouncerLine::for_callout(e.kind)).collect();
    expected.push(AnnouncerLine::Combo { hits: 7 });
    assert_eq!(out.announcements, expected, "each word's line, then the count; four hits are not called");
    for (i, line) in out.announcements.iter().enumerate() {
        // He says the word, never a move's name.
        assert!(!line.file_stem().contains("shinku"), "line {i}: {line:?}");
    }
    for t in 0..30 {
        assert!(s.step(idle()).announcements.is_empty(), "frame {t}: one call, once");
    }
}

#[test]
fn the_faithful_1987_preset_asks_the_announcer_for_nothing() {
    let Some(mut s) = local_fight(RulesetPreset::Faithful1987) else { return };
    s.push_callouts(&everything());
    s.finish_combo(9);
    for t in 0..10 {
        assert!(s.step(idle()).announcements.is_empty(), "frame {t}");
    }
}
