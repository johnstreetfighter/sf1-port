//! Acceptance test for ticket #27: Retsu takes hits, blocks and is knocked
//! out. Two one-player scenarios on the sf-cpu-01 boot path, P1 played from
//! their input schedules from the checkpoint-2389 fixture
//! (docs/reference/retsu-reactions.md):
//!
//! - sf-retsu-01: Ryu stands and throws every normal in turn while Retsu
//!   comes in; Retsu blocks crouching and standing through the decision's
//!   reaction branch, and takes punches and kicks through the CPU reaction
//!   path (0x447c8) — 2390..=3400 all match.
//! - sf-retsu-02: Ryu walks in and throws heavy Shoryukens; Retsu blocks
//!   some, takes two, and is knocked out at 3018 through the CPU knock-down
//!   table; since ticket #31 the run goes on through the player's round-win
//!   result into its bonus tally — 2390..=3400 all match.
//!
//! For each: the two reference runs repeat exactly; the native run matches
//! every observable; and the owned one-player RAM (object RAM, both player
//! structures, the AI tables and the reaction table pointers, the timer,
//! the scheduler, the queues, the random state and the camera) is
//! byte-identical after every frame.
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
const FIXTURE: u32 = 2389;
/// Retsu is knocked out (+8 = 8) in sf-retsu-02.
const KO: u32 = 3018;

/// (scenario number, last frame compared).
const SCENARIOS: [(u32, u32); 2] = [(1, 3400), (2, 3400)];

fn paths(n: u32) -> (String, String, String) {
    (
        format!("../../tools/reference/scenarios/sf_retsu_{n:02}.json"),
        format!("../../tools/reference/local/captures/retsu-{n:02}-run-a"),
        format!("../../tools/reference/local/captures/retsu-{n:02}-run-b"),
    )
}

fn prerequisites_present() -> bool {
    Path::new(ROMS).is_dir()
        && SCENARIOS.iter().all(|&(n, _)| {
            let (_, a, b) = paths(n);
            Path::new(&a).is_dir() && Path::new(&b).is_dir()
        })
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
    for (n, last) in SCENARIOS {
        let (scenario, a, b) = paths(n);
        let scenario = load_scenario(&scenario).unwrap();
        let outcome = compare(
            &scenario,
            &load_capture(&a).unwrap(),
            &load_capture(&b).unwrap(),
            CandidateKind::Reference,
            &scope(last),
        )
        .expect("compatible captures");
        assert!(
            matches!(outcome, Outcome::Match { .. }),
            "sf-retsu-{n:02}: {outcome:?}"
        );
    }
}

#[test]
fn native_runs_match_the_reference_on_every_observable() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    for (n, last) in SCENARIOS {
        let (scenario_path, a, _) = paths(n);
        let out = sf_engine::run::ScratchDir::new(&format!("sf-engine-retsu-{n:02}"));
        let id = format!("sf-retsu-{n:02}");
        sf_engine::run::run_cpu_fight_capture_with_inputs(
            Path::new(&a),
            &assets,
            &out,
            &id,
            Some(Path::new(&scenario_path)),
            FIXTURE,
            last,
        )
        .expect("native capture runs on translated paths only");
        let scenario = load_scenario(&scenario_path).unwrap();
        let outcome = compare(
            &scenario,
            &load_capture(&a).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &scope(last),
        )
        .expect("compatible captures");
        let frames = last - FIXTURE;
        let Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } = outcome
        else {
            panic!("{id} must match the reference, got {outcome:?}");
        };
        assert_eq!(
            (frames_compared, checkpoints_compared),
            (frames as u64, frames as usize),
            "{id}"
        );
    }
}

#[test]
fn owned_ram_is_identical_every_frame() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    for (n, last) in SCENARIOS {
        let (scenario, a, _) = paths(n);
        let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
        let outcome = sf_engine::run::compare_cpu_ram_with_inputs(
            Path::new(&a),
            rom,
            Some(Path::new(&scenario)),
            FIXTURE,
            last,
        )
        .expect("reference dumps readable");
        assert_eq!(
            outcome,
            CpuRamOutcome::Match {
                frames: last - FIXTURE
            },
            "sf-retsu-{n:02}"
        );
    }
}

#[test]
fn the_reference_has_retsu_knocked_out_at_the_ko_frame() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let (_, a, _) = paths(2);
    let read = |f: u32| std::fs::read(Path::new(&a).join(format!("ram_{f:06}_work.bin"))).unwrap();
    let retsu = |w: &[u8]| u16::from_be_bytes([w[0x4664 + 0x10], w[0x4664 + 0x11]]) as usize;
    let before = read(KO - 1);
    let ko = read(KO);
    assert_eq!(
        before[0x6000 + retsu(&before) + 8],
        2,
        "Retsu stands at {}",
        KO - 1
    );
    assert_eq!(
        ko[0x6000 + retsu(&ko) + 8],
        8,
        "the reference has Retsu knocked out at {KO}"
    );
    assert_eq!((ko[0x466a], ko[0x466b]), (0, 0), "no health left");
}
