//! Ticket #195 (`ST6`): the 1987 game never sends an extended tune command
//! (ADR-0009). The extended tune table answers sound commands
//! `0x40..=0x7f`; the ROM's table has entries for `0x00..=0x3f` only, and
//! past them its words are tune data. This sweep reads the sound latch after
//! every frame of every committed scenario from power-on to its end
//! (874,344 frames) and of Local fights under Faithful 1987 and under an
//! overhaul preset, and finds no command in that range. The table itself,
//! the converter and the test theme are checked in `sf-audio`
//! (`tests/extended_tunes.rs`, `src/score.rs`). Local ROMs only; each test
//! skips without them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::run::observe_power_on_scenario;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const SCENARIOS: &str = "../../tools/reference/scenarios";
/// The fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;
/// The commands the extended tune table answers (`sf_audio::tunes`'s
/// `EXTENDED_FIRST..=EXTENDED_LAST`, which its own suite pins to these).
const EXTENDED: std::ops::RangeInclusive<u8> = 0x40..=0x7f;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn duration(path: &Path) -> u32 {
    let text = std::fs::read_to_string(path).unwrap();
    let after = text.split("\"duration_frames\":").nth(1).expect("duration_frames");
    let digits: String = after.trim_start().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap()
}

fn extended(command: u8) -> bool {
    EXTENDED.contains(&command)
}

/// The complete one- and two-player games, the service mode (its sound test
/// sends every tune and effect by hand), every boss on the ladder, the
/// bonus stages and the ending are all among them.
#[test]
fn no_committed_scenario_sends_an_extended_tune_command() {
    let Some(a) = assets() else { return };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(SCENARIOS).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    let (mut sent, mut frames) = (BTreeSet::new(), 0u64);
    for p in &paths {
        let (last, unrecovered) = observe_power_on_scenario(a.maincpu.clone(), p, duration(p) - 1, |m| {
            // Every write of the frame: a freeze resume can make two.
            sent.insert(m.state.sound_latch);
            sent.extend(m.latch_writes().iter().map(|w| w.value));
        })
        .unwrap();
        assert!(unrecovered.is_empty(), "{}: untranslated {unrecovered:05x?}", p.display());
        frames += u64::from(last) + 1;
    }
    println!("{} scenarios, {frames} frames, {} distinct latch values", paths.len(), sent.len());
    assert!(paths.len() >= 137);
    // The sweep sees the ROM's tunes (0x28..=0x3e) go by.
    assert!((0x28..=0x3e).filter(|c| sent.contains(c)).count() >= 15, "{sent:02x?}");
    let found: Vec<u8> = sent.iter().copied().filter(|&c| extended(c)).collect();
    assert!(found.is_empty(), "the 1987 game sent {found:02x?}");
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

/// Power-on, the Main Menu, Multi Player, Local; both players take the
/// fighter under their cursor; Japan; the first fight loop and 600 frames
/// of it. Returns every sound latch on the way.
fn local_fight(a: Assets, preset: RulesetPreset) -> Vec<u8> {
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    let mut latches = Vec::new();
    let mut step = |s: &mut GameSession, i: FrameInputs| latches.push(s.step(i).sound_latch);
    for tap in [
        with(|i| i.menu.start = true),
        with(|i| i.p1.directions[2] = true),
        with(|i| i.menu.confirm = true),
        with(|i| i.menu.confirm = true),
        with(|i| i.p1.buttons[0] = true),
        with(|i| i.p2.buttons[0] = true),
    ] {
        let presses = 1 + usize::from(s.screen() == "select" && s.match_config().era_select_enabled());
        for _ in 0..presses {
            step(&mut s, tap);
            step(&mut s, idle());
        }
    }
    while s.screen() != "country select" {
        step(&mut s, idle());
    }
    step(&mut s, with(|i| i.p1.buttons[0] = true));
    step(&mut s, idle());
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        step(&mut s, idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    for _ in 0..600 {
        step(&mut s, idle());
    }
    latches
}

#[test]
fn local_fights_under_faithful_and_overhaul_presets_send_no_extended_command() {
    let Some(a) = assets() else { return };
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        let latches = local_fight(load_assets(ROMS, MANIFEST).unwrap(), preset);
        let sent: BTreeSet<u8> = latches.iter().copied().filter(|&c| c != 0xff).collect();
        assert!(sent.iter().any(|c| (0x28..=0x3e).contains(c)), "{preset:?} plays a stage theme: {sent:02x?}");
        assert!(sent.iter().all(|&c| !extended(c)), "{preset:?} sent {sent:02x?}");
    }
    drop(a);
}
