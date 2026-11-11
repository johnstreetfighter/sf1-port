//! Acceptance test for ticket #31: Ryu wins a round and the match. One
//! one-player scenario (sf-win-01) on the sf-cpu-01 boot path, P1 played
//! from its input schedule from the checkpoint-2389 fixture: heavy
//! Shoryukens every 60 frames knock Retsu out in round 1 (3018) and round
//! 2 (4537). The native run follows the reference through both round-win
//! results with their bonus tallies (scene event 7's human branch), the
//! match-won screen (event 0xb), the wipe, Geki's stage intro (event 4),
//! ROUND 1, the new opponent's setup, the walk-in and FIGHT!, into the
//! Geki fight. Ticket #31 originally stopped at 6252 on Geki's 0x4656e;
//! #67 translates that routine and geki.rs continues through the match.
//!
//! The two reference runs repeat exactly; the native run matches every
//! observable through 6251; the owned one-player RAM (including the result
//! tally's numbers, the game state bytes and the palette request) is
//! byte-identical after every frame through 6300.
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::CpuRamOutcome;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_01.json";
const REF_A: &str = "../../tools/reference/local/captures/win-01-run-a";
const REF_B: &str = "../../tools/reference/local/captures/win-01-run-b";
const FIXTURE: u32 = 2389;
/// Retsu is knocked out in round 1 and round 2.
const KO_1: u32 = 3018;
const KO_2: u32 = 4537;
/// The last frame of ticket #31's observable comparison.
const LAST: u32 = 6251;
const CAPTURE_LAST: u32 = 6300;

fn prerequisites_present() -> bool {
    Path::new(ROMS).is_dir() && Path::new(REF_A).is_dir() && Path::new(REF_B).is_dir()
}

/// The fixture row (2389) is excluded: its screen hash is the scanout of
/// 2388 (FIGHT! still shown), which a post-erase fixture cannot reproduce.
fn scope(last: u32) -> CompareScope {
    CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(last as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

#[test]
fn reference_runs_repeat_exactly() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).unwrap();
    let outcome = compare(
        &scenario,
        &load_capture(REF_A).unwrap(),
        &load_capture(REF_B).unwrap(),
        CandidateKind::Reference,
        &scope(CAPTURE_LAST),
    )
    .expect("compatible captures");
    assert!(
        matches!(outcome, Outcome::Match { .. }),
        "sf-win-01: {outcome:?}"
    );
}

#[test]
fn native_match_matches_the_reference_on_every_observable() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = sf_engine::run::ScratchDir::new("sf-engine-win-01");
    sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(REF_A),
        &assets,
        &out,
        "sf-win-01",
        Some(Path::new(SCENARIO)),
        FIXTURE,
        LAST,
    )
    .expect("native capture runs on translated paths only");
    let scenario = load_scenario(SCENARIO).unwrap();
    let outcome = compare(
        &scenario,
        &load_capture(REF_A).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope(LAST),
    )
    .expect("compatible captures");
    let frames = LAST - FIXTURE;
    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = outcome
    else {
        panic!("sf-win-01 must match the reference, got {outcome:?}");
    };
    assert_eq!(
        (frames_compared, checkpoints_compared),
        (frames as u64, frames as usize)
    );
}

#[test]
fn owned_ram_is_identical_every_frame_through_the_fixture_window() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let outcome = sf_engine::run::compare_cpu_ram_with_inputs(
        Path::new(REF_A),
        rom,
        Some(Path::new(SCENARIO)),
        FIXTURE,
        CAPTURE_LAST,
    )
    .expect("reference dumps readable");
    assert_eq!(
        outcome,
        CpuRamOutcome::Match {
            frames: CAPTURE_LAST - FIXTURE
        }
    );
}

#[test]
fn the_reference_has_ryu_winning_both_rounds_and_the_match() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let read =
        |f: u32| std::fs::read(Path::new(REF_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
    let retsu_down = |w: &[u8]| {
        let e = u16::from_be_bytes([w[0x4664 + 0x10], w[0x4664 + 0x11]]) as usize;
        w[0x6000 + e + 8] == 8
    };
    assert!(
        !retsu_down(&read(KO_1 - 1)) && retsu_down(&read(KO_1)),
        "Retsu knocked out at {KO_1}"
    );
    assert!(
        !retsu_down(&read(KO_2 - 1)) && retsu_down(&read(KO_2)),
        "Retsu knocked out at {KO_2}"
    );
    // After round 1's result: one win, the 67-second time bonus and the
    // vitality bonus tallied as 108 ticks of 10 onto the score.
    let w = read(3537);
    assert_eq!(w[0x6b8], 1, "P1 wins after round 1");
    assert_eq!(
        &w[0x6d2..0x6d9],
        &[0, 0, 0, 2, 5, 8, 0],
        "the score after round 1's tally"
    );
    assert_eq!(&w[0xe44..0xe49], &[1, 0, 8, 0, 0], "the bonus digits");
    // The match: two wins, the winner recorded (0xffc009 = 1), then the
    // next opponent (fight 1 of the country) set up for round 0.
    let w = read(5059);
    assert_eq!((w[0x6b8], w[0x4009]), (2, 1));
    let w = read(LAST);
    assert_eq!(
        (w[0x4003], w[0x4004], w[0x400a], w[0x6b8], w[0x466c]),
        (0, 1, 0, 0, 0),
        "stage 0, fight 1, round 0, no wins"
    );
}
