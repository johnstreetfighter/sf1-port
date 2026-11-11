//! Acceptance test for ticket #22: the Hadoken and its projectile. Three
//! versus scenarios (sf-hadoken-01..03, from the checkpoint-2450 fixture):
//! Ryu's Hadoken thrown from the start position hits the standing Ken and
//! slows the fight (scene event 9); against Ken's block it does chip
//! damage; thrown at the walk pin it hits as it spawns; Ryu's and Ken's
//! Hadokens meet and cancel; and Ken's hits Ryu (docs/reference/hadoken.md).
//! For each: the two reference runs repeat exactly; the native run matches
//! every observable; and the memory the translated routines own is
//! byte-identical to the reference after every frame.
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// (scenario number, last frame).
const SCENARIOS: [(u32, u32); 3] = [(1, 2940), (2, 2960), (3, 2880)];

fn paths(n: u32) -> (String, String, String) {
    (
        format!("../../tools/reference/scenarios/sf_hadoken_{n:02}.json"),
        format!("../../tools/reference/local/captures/hadoken-{n:02}-run-a"),
        format!("../../tools/reference/local/captures/hadoken-{n:02}-run-b"),
    )
}

fn prerequisites_present() -> bool {
    Path::new(ROMS).is_dir()
        && SCENARIOS.iter().all(|&(n, _)| {
            let (_, a, b) = paths(n);
            Path::new(&a).is_dir() && Path::new(&b).is_dir()
        })
}

fn scope(last: u32) -> CompareScope {
    CompareScope {
        from: Some(2450),
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
            "sf-hadoken-{n:02}: {outcome:?}"
        );
    }
}

#[test]
fn native_hadokens_match_the_reference_on_every_observable() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    for (n, last) in SCENARIOS {
        let (scenario_path, a, _) = paths(n);
        let out = sf_engine::run::ScratchDir::new(&format!("sf-engine-hadoken-{n:02}"));
        let id = format!("sf-hadoken-{n:02}");
        sf_engine::run::run_versus_interval_capture(
            Path::new(&a),
            &assets,
            &out,
            &id,
            Path::new(&scenario_path),
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
        let frames = last - 2450 + 1;
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
        let result =
            sf_engine::run::compare_versus_ram(Path::new(&a), rom, Path::new(&scenario), last)
                .expect("reference dumps readable");
        assert_eq!(result, Ok(last - 2450), "sf-hadoken-{n:02}");
    }
}
