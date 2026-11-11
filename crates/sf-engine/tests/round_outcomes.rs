//! Acceptance test for ticket #30: the remaining round outcomes — the
//! time-over judged by health, the time-out draw, the double KO, and the
//! three-rounds rule — on two versus scenarios from the checkpoint-2450
//! fixture (docs/reference/round-outcomes.md):
//!
//! - sf-doubleko-01: mirrored play; every 200 frames both walk in and throw
//!   Hadokens that land on each other on the same frame; the third pair
//!   knocks both out together (2922, 4122, 5322). Each double KO shows
//!   DOUBLE K.O. and ends the round with no winner; after the third the
//!   three-rounds rule finds no winner. Its game over is covered through
//!   the attract boundary by game_over.rs (ticket #65).
//! - sf-timeover-01: one jab from Ryu, then both idle: round 1 ends at
//!   the time-over with Ryu ahead on health (TIME OVER, his result and
//!   bonus tally); rounds 2 and 3 are time-out draws (DRAW GAME); the
//!   three-rounds rule gives Ryu the match 1–0, the versus winner's
//!   match-won screen and wipe follow; after #66 the run continues through
//!   his ranking check, mode-2 placement and Retsu's stage intro to 10300.
//!
//! For each: the two reference runs repeat exactly; the native run
//! matches every observable through the last translated frame; the owned
//! versus RAM is byte-identical after every one of those frames.
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIXTURE: u32 = 2450;

struct Scenario {
    name: &'static str,
    /// The capture's last frame.
    capture_last: u32,
    /// The last frame on translated paths.
    last: u32,
    /// An untranslated branch the next frame reaches, if this test's interval
    /// still ends at one.
    stop: Option<u32>,
}

const SCENARIOS: [Scenario; 2] = [
    Scenario {
        name: "doubleko-01",
        capture_last: 5800,
        last: 5661,
        stop: None,
    },
    Scenario {
        name: "timeover-01",
        capture_last: 10300,
        last: 10300,
        stop: None,
    },
];

fn paths(s: &Scenario) -> (String, String, String) {
    (
        format!(
            "../../tools/reference/scenarios/sf_{}.json",
            s.name.replace('-', "_")
        ),
        format!("../../tools/reference/local/captures/{}-run-a", s.name),
        format!("../../tools/reference/local/captures/{}-run-b", s.name),
    )
}

fn prerequisites_present() -> bool {
    Path::new(ROMS).is_dir()
        && SCENARIOS.iter().all(|s| {
            let (_, a, b) = paths(s);
            Path::new(&a).is_dir() && Path::new(&b).is_dir()
        })
}

fn scope(last: u32) -> CompareScope {
    CompareScope {
        from: Some(FIXTURE as u64),
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
    for s in &SCENARIOS {
        let (scenario, a, b) = paths(s);
        let scenario = load_scenario(&scenario).unwrap();
        let outcome = compare(
            &scenario,
            &load_capture(&a).unwrap(),
            &load_capture(&b).unwrap(),
            CandidateKind::Reference,
            &scope(s.capture_last),
        )
        .expect("compatible captures");
        assert!(
            matches!(outcome, Outcome::Match { .. }),
            "{}: {outcome:?}",
            s.name
        );
    }
}

#[test]
fn native_outcomes_match_the_reference_on_every_observable() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    for s in &SCENARIOS {
        let (scenario_path, a, _) = paths(s);
        let out = sf_engine::run::ScratchDir::new(&format!("sf-engine-{}", s.name));
        let id = format!("sf-{}", s.name);
        sf_engine::run::run_versus_interval_capture(
            Path::new(&a),
            &assets,
            &out,
            &id,
            Path::new(&scenario_path),
            s.last,
        )
        .expect("native capture runs on translated paths only");
        let scenario = load_scenario(&scenario_path).unwrap();
        let outcome = compare(
            &scenario,
            &load_capture(&a).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &scope(s.last),
        )
        .expect("compatible captures");
        let frames = s.last - FIXTURE + 1;
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
fn owned_ram_is_identical_and_remaining_untranslated_branches_are_reported() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    for s in &SCENARIOS {
        let (scenario, a, _) = paths(s);
        let rom = || sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
        let result =
            sf_engine::run::compare_versus_ram(Path::new(&a), rom(), Path::new(&scenario), s.last)
                .expect("reference dumps readable");
        assert_eq!(result, Ok(s.last - FIXTURE), "{}", s.name);
        let Some(stop) = s.stop else {
            continue;
        };
        let result = sf_engine::run::compare_versus_ram(
            Path::new(&a),
            rom(),
            Path::new(&scenario),
            s.last + 1,
        )
        .expect("reference dumps readable");
        assert_eq!(
            result,
            Err(format!(
                "frame {}: untranslated branches [{:05x}] reached after {} matching frames",
                s.last + 1,
                stop,
                s.last - FIXTURE
            )),
            "{}",
            s.name
        );
    }
}

#[test]
fn the_reference_has_three_double_kos_and_no_winner() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let (_, a, _) = paths(&SCENARIOS[0]);
    let read = |f: u32| std::fs::read(Path::new(&a).join(format!("ram_{f:06}_work.bin"))).unwrap();
    for (ko, round) in [(2922, 0), (4122, 1), (5322, 2)] {
        let before = read(ko - 1);
        let w = read(ko);
        assert_eq!(
            (before[0x6408], before[0x6a08]),
            (2, 2),
            "both standing at {}",
            ko - 1
        );
        assert_eq!(
            (w[0x6408], w[0x6a08], w[0x400a]),
            (8, 8, round),
            "both down at {ko} in round {round}"
        );
    }
    let w = read(5661);
    assert_eq!(
        (w[0x400a], w[0x4009], w[0x6b8], w[0x6e2]),
        (3, 0, 0, 0),
        "three rounds, no winner, no wins"
    );
}

#[test]
fn the_reference_has_a_time_over_win_two_draws_and_the_match_to_ryu() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let (_, a, _) = paths(&SCENARIOS[1]);
    let read = |f: u32| std::fs::read(Path::new(&a).join(format!("ram_{f:06}_work.bin"))).unwrap();
    // Round 1: the jab leaves Ken at 11 cells + 2; TIME reaches zero at
    // 4330; the result gives Ryu the round and tallies his 48 vitality
    // points (480 on the bonus, 10 a tick onto the score's 10).
    let w = read(4330);
    assert_eq!(
        (&w[0xe18..0xe1b], w[0x6b7], w[0x6b6], w[0x6e1], w[0x6e0]),
        (&[0u8, 0, 0][..], 11, 4, 11, 2)
    );
    let w = read(4745);
    assert_eq!((w[0x6b8], w[0x6e2]), (1, 0), "Ryu wins round 1");
    assert_eq!(
        &w[0x6d2..0x6d9],
        &[0, 0, 0, 0, 4, 9, 0],
        "the score after the tally"
    );
    assert_eq!(&w[0xe44..0xe49], &[0, 4, 8, 0, 0], "the bonus digits");
    // Rounds 2 and 3: time-out draws (event 8), no wins added.
    for (draw, round) in [(7155, 1), (9775, 2)] {
        let w = read(draw);
        assert_eq!(
            (w[0xe36], w[0x400a], w[0x6b8], w[0x6e2]),
            (8, round, 1, 0),
            "a draw at {draw}"
        );
    }
    // Three rounds played: Ryu's one win takes the match (0xffc009 = 1);
    // the match-won screen (event 0xb), then the wipe, then mode 2.
    let w = read(9986);
    assert_eq!((w[0x400a], w[0x4009], w[0xe36]), (3, 1, 11));
    let w = read(10267);
    assert_eq!(
        (w[0x4002], w[0x4009], w[0x400a]),
        (2, 0, 0),
        "P1 goes on against the CPU"
    );
}
