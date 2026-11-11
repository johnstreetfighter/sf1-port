//! Acceptance test for ticket #28: the round result after a CPU win. Both
//! of the fixed fight's rounds, from fixtures in their fights (P1 idle):
//!
//! - round 1 (sf-round-end-01, fixture 3300, KO 3328): the KO path, the
//!   win marker, Retsu's victory pose and the "ENEMY WON" result text run
//!   to 3569; at 3570 the next round is requested (event 6), which ticket
//!   #29 translated since (tests/round_transition.rs);
//! - round 2 (sf-round2-02, fixture 3900, KO 4568): the same to 4796; at
//!   4797 the second win ends the match (0x3eefa: the CPU's match win).
//!
//! Over each window the native run matches every observable (per-frame
//! screen hashes, checkpoint pixels and video/palette RAM every frame,
//! sound-latch timing) and the owned one-player RAM after every frame, and
//! stops exactly where the reference moves on to untranslated round flow.
//!
//! Consumes local-only data; skips on a clean checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::CpuRamOutcome;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// (scenario, capture, fixture, last matched frame, next frame's branch).
const ROUNDS: [(&str, &str, u32, u32, u32); 2] = [
    ("sf_round_end_01", "round-end-run-a", 3300, 3569, 0x3ec4e),
    ("sf_round2_02", "round2-02-run-a", 3900, 4796, 0x3eefa),
];

fn capture(name: &str) -> String {
    format!("../../tools/reference/local/captures/{name}")
}

fn present() -> bool {
    Path::new(ROMS).is_dir() && ROUNDS.iter().all(|r| Path::new(&capture(r.1)).is_dir())
}

#[test]
fn the_round_results_match_on_every_observable() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    for (scenario, cap, first, last, _) in ROUNDS {
        let out = sf_engine::run::ScratchDir::new("sf-engine-round-result");
        let id = scenario.replace('_', "-");
        sf_engine::run::run_cpu_fight_capture(
            Path::new(&capture(cap)),
            &assets,
            &out,
            &id,
            first,
            last,
        )
        .expect("native capture runs on translated paths only");
        let scope = CompareScope {
            from: Some((first + 1) as u64),
            to: Some(last as u64),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        };
        let scenario_path = format!("../../tools/reference/scenarios/{scenario}.json");
        let outcome = compare(
            &load_scenario(&scenario_path).unwrap(),
            &load_capture(capture(cap)).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &scope,
        )
        .expect("compatible captures");
        let Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } = outcome
        else {
            panic!("{scenario}: {outcome:?}");
        };
        assert_eq!(
            (frames_compared, checkpoints_compared),
            ((last - first) as u64, (last - first) as usize),
            "{scenario}"
        );
    }
}

#[test]
fn the_owned_ram_matches_until_the_next_round_flow() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    for (scenario, cap, first, last, _next) in ROUNDS {
        let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
        let outcome =
            sf_engine::run::compare_cpu_ram(Path::new(&capture(cap)), rom, first, last + 50)
                .unwrap();
        // Both round 1's next-round wipe (0x3ec4e, ticket #29) and round 2's
        // CPU match win (0x3eefa, ticket #45) are now translated and run on.
        let want = CpuRamOutcome::Match {
            frames: last + 50 - first,
        };
        assert_eq!(outcome, want, "{scenario}");
    }
}
