//! Acceptance test for ticket #16: the fighters' entity-slot swap. In the
//! versus scene sf-swap-01 (frames 2450..=2950) each standing heavy kick
//! moves Ryu from object slot 0x400 into 0xa00 and back (frame flags
//! 0x10 / 0x20, routine 0x3b852); the native run matches the reference on
//! every observable and on the owned RAM after every frame. A fixture inside
//! round 2 of the fixed fight (sf-round2-01, after round 1's swap: P1 in
//! slot 0xa00) loads and runs, matching until the first untranslated branch.
//!
//! The comparison
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

const SCENARIO: &str = "../../tools/reference/scenarios/sf_swap_01.json";
const REF_A: &str = "../../tools/reference/local/captures/swap-01-run-a";
const REF_B: &str = "../../tools/reference/local/captures/swap-01-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn scope() -> CompareScope {
    CompareScope {
        from: Some(2450),
        to: Some(2950),
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
fn native_swap_scene_matches_the_reference_over_the_interval() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = sf_engine::run::ScratchDir::new("sf-engine-swap-capture");
    sf_engine::run::run_versus_interval_capture(
        Path::new(REF_A),
        &assets,
        &out,
        "sf-swap-01",
        Path::new(SCENARIO),
        2950,
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
        panic!("native swap scene must match the reference, got {outcome:?}");
    };
    assert_eq!(*frames_compared, 501);
    assert_eq!(*checkpoints_compared, 501);
}

/// Both fighters' regions, the player structures' entity words and the
/// rest of the owned RAM are byte-identical to the reference after every
/// frame, through both swaps of each kick.
#[test]
fn both_swaps_keep_the_owned_ram_identical_every_frame() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let result =
        sf_engine::run::compare_versus_ram(Path::new(REF_A), rom, Path::new(SCENARIO), 2950)
            .expect("reference dumps readable");
    assert_eq!(result, Ok(2950 - 2450));
}

const ROUND2: &str = "../../tools/reference/local/captures/round2-run-a";

/// Inside round 2 of the fixed fight (P1 idle), after round 1's swap: the
/// fixture at 3900 has Ryu in slot 0xa00 and Retsu in 0x400, and the native
/// run matches the owned one-player RAM over the whole capture (ticket #25
/// translated the follow-up decision that stopped it at 3932 before).
#[test]
fn a_fixture_after_the_round_one_swap_runs_until_the_next_untranslated_branch() {
    if !Path::new(ROUND2).is_dir() || !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local round-2 capture/ROMs not present");
        return;
    }
    let work = std::fs::read(Path::new(ROUND2).join("ram_003900_work.bin")).unwrap();
    assert_eq!(
        &work[0x6b0 + 0xe..0x6b0 + 0x12],
        &[0x04, 0x00, 0x0a, 0x00],
        "P1 in slot 0xa00"
    );
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let outcome = sf_engine::run::compare_cpu_ram(Path::new(ROUND2), rom, 3900, 4100).unwrap();
    assert_eq!(
        outcome,
        sf_engine::run::CpuRamOutcome::Match { frames: 200 }
    );
}
