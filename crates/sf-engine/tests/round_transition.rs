//! Acceptance test for ticket #29: the transition from round 1 into round
//! 2. From the fixed fight's round-1 fixture at 3300 (sf-round-transition-01,
//! P1 idle from there) the native one-player machine runs the KO path and
//! round result (ticket #28), the next-round wipe (scene event 6), the
//! object clears and round initialisation (0x3e77c: the stage position,
//! the ROUND 2 announcement as scene event 5 with its layers off and its
//! voice, the HUD texts, scores, names, full health, the fighters' starting
//! moves, health bars and markers), the stage music, the walk-in intro,
//! FIGHT! and the round timer's reload, and the first frames of round 2's
//! fight. Over 3301..=3999 it matches the reference on every observable
//! (per-frame screen hashes, checkpoint pixels and video/palette RAM every
//! frame, sound-latch timing) and in the owned RAM after every frame.
//!
//! Consumes local-only data; skips on a clean checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;

const SCENARIO: &str = "../../tools/reference/scenarios/sf_round_transition_01.json";
const REF_A: &str = "../../tools/reference/local/captures/round-transition-run-a";
const REF_B: &str = "../../tools/reference/local/captures/round-transition-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIRST: u32 = 3300;
const LAST: u32 = 3999;

fn present() -> bool {
    Path::new(REF_A).is_dir() && Path::new(REF_B).is_dir() && Path::new(ROMS).is_dir()
}

fn scope() -> CompareScope {
    CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

#[test]
fn reference_runs_repeat_exactly() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(REF_A).unwrap(),
        &load_capture(REF_B).unwrap(),
        CandidateKind::Reference,
        &scope(),
    )
    .expect("compatible captures");
    assert!(matches!(outcome, Outcome::Match { .. }), "{outcome:?}");
}

#[test]
fn the_transition_matches_on_every_observable() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = sf_engine::run::ScratchDir::new("sf-engine-round-transition");
    sf_engine::run::run_cpu_fight_capture(
        Path::new(REF_A),
        &assets,
        &out,
        "sf-round-transition-01",
        FIRST,
        LAST,
    )
    .expect("native capture runs on translated paths only");
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(REF_A).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope(),
    )
    .expect("compatible captures");
    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = outcome
    else {
        panic!("the transition must match the reference, got {outcome:?}");
    };
    assert_eq!(
        (frames_compared, checkpoints_compared),
        ((LAST - FIRST) as u64, (LAST - FIRST) as usize)
    );
}

#[test]
fn the_owned_ram_matches_every_frame() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let outcome = sf_engine::run::compare_cpu_ram(Path::new(REF_A), rom, FIRST, LAST).unwrap();
    assert_eq!(
        outcome,
        sf_engine::run::CpuRamOutcome::Match {
            frames: LAST - FIRST
        }
    );
}
