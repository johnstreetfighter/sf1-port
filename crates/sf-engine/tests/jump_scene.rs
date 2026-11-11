//! Acceptance test for ticket #8: the native engine advances the bounded
//! versus jump scene (frames 2450..=2999 of sf-jump-01) and the comparison
//! boundary finds no observable difference against the pinned MAME reference
//! over the scoped observables: per-frame screen hashes, full-pixel
//! checkpoints every frame, video/palette RAM dumps, and sound-latch timing.
//!
//! Also runs the reference-versus-reference repeatability check (two
//! independent baseline captures of the same scenario).
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;

const SCENARIO: &str = "../../tools/reference/scenarios/sf_jump_01.json";
const REF_A: &str = "../../tools/reference/local/captures/jump-01-run-a";
const REF_B: &str = "../../tools/reference/local/captures/jump-01-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn scope() -> CompareScope {
    CompareScope {
        from: Some(2450),
        to: Some(2999),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

fn prerequisites_present() -> bool {
    Path::new(REF_A).is_dir()
        && Path::new(REF_B).is_dir()
        && Path::new(ROMS).is_dir()
        && Path::new(MANIFEST).is_file()
}

#[test]
fn reference_runs_repeat_exactly() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).unwrap();
    let a = load_capture(REF_A).unwrap();
    let b = load_capture(REF_B).unwrap();
    let outcome = compare(&scenario, &a, &b, CandidateKind::Reference, &scope())
        .expect("compatible captures");
    assert!(
        matches!(outcome, Outcome::Match { .. }),
        "two reference runs must repeat exactly, got {outcome:?}"
    );
}

#[test]
fn native_jump_scene_matches_the_reference_over_the_interval() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = sf_engine::run::ScratchDir::new("sf-engine-jump-capture");
    sf_engine::run::run_jump_capture(
        Path::new(REF_A),
        &assets,
        &out,
        "sf-jump-01",
        Path::new(SCENARIO),
    )
    .expect("native capture runs");

    let scenario = load_scenario(SCENARIO).unwrap();
    let reference = load_capture(REF_A).unwrap();
    let candidate = load_capture(&out).unwrap();
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope(),
    )
    .expect("compatible captures");
    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = &outcome
    else {
        panic!("native jump scene must match the reference, got {outcome:?}");
    };
    assert_eq!(*frames_compared, 550);
    assert_eq!(*checkpoints_compared, 550);
}

/// Ticket #15: the scene is produced by the translated posture dispatch and
/// move streams, so the memory the translated routines own (Ryu's entity,
/// both player structures with the command trackers, Ken, effect slots,
/// sound queue, button latches, random state, camera) is byte-identical to
/// the reference's work-RAM dump after every frame.
#[test]
fn translated_dispatch_keeps_the_owned_ram_identical_every_frame() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let result =
        sf_engine::run::compare_versus_ram(Path::new(REF_A), rom, Path::new(SCENARIO), 2999)
            .expect("reference dumps readable");
    assert_eq!(result, Ok(2999 - 2450));
}
