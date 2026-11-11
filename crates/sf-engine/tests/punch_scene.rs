//! Acceptance test for ticket #9: the native engine plays the standing
//! light punch of sf-punch-01 (frames 2450..=3399) from recovered ROM data
//! and translated routines — input recognition, move start, animation
//! stream, collision judge, damage/score, the opponent's hit-stun reaction,
//! the hit spark, the sound queue, and the camera push — and the comparison
//! boundary finds no observable difference against the pinned MAME
//! reference: per-frame screen hashes, full-pixel checkpoints every frame,
//! video/palette RAM dumps, and sound-latch timing.
//!
//! Also checks the reference-versus-reference repeatability, that the run
//! never reaches an untranslated original branch, and the hit/miss outcomes
//! at the documented distances (docs/reference/punch.md).
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::versus::VersusMachine;

const SCENARIO: &str = "../../tools/reference/scenarios/sf_punch_01.json";
const REF_A: &str = "../../tools/reference/local/captures/punch-01-run-a";
const REF_B: &str = "../../tools/reference/local/captures/punch-01-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn scope() -> CompareScope {
    CompareScope {
        from: Some(2450),
        to: Some(3399),
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
fn native_punch_scene_matches_the_reference_over_the_interval() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = sf_engine::run::ScratchDir::new("sf-engine-punch-capture");
    // The runner fails if any untranslated original branch is reached.
    sf_engine::run::run_punch_capture(
        Path::new(REF_A),
        &assets,
        &out,
        "sf-punch-01",
        Path::new(SCENARIO),
    )
    .expect("native capture runs on translated paths only");

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
        panic!("native punch scene must match the reference, got {outcome:?}");
    };
    assert_eq!(*frames_compared, 950);
    assert_eq!(*checkpoints_compared, 950);
}

/// The scenario's outcomes, read from the native machine-visible state:
/// P2 health (player +6/+7), P1 score digits, the two fighters' anchors,
/// and Ken's move/state bytes.
#[derive(Debug, PartialEq, Eq)]
struct Outcomes {
    p2_rem: u8,
    p2_cells: u8,
    score_hundreds: u8,
    ken_sx: u16,
    ryu_sx: u16,
    ken_move: u8,
}

fn outcomes(m: &VersusMachine) -> Outcomes {
    let work = m.work_ram();
    let obj = &m.state.objectram;
    let w = |i: usize| u16::from_be_bytes([obj[i], obj[i + 1]]);
    Outcomes {
        p2_rem: work[0x6e0],
        p2_cells: work[0x6e1],
        score_hundreds: work[0x6d7],
        ken_sx: w(0xa06),
        ryu_sx: w(0x406),
        ken_move: obj[0xa0a],
    }
}

#[test]
fn hits_and_misses_follow_the_documented_distances() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let mut m =
        VersusMachine::from_checkpoint_2450_with_program(Path::new(REF_A), rom).expect("fixture");
    let events = sf_engine::run::scenario_input_events(Path::new(SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; sf_engine::run::INPUTS]);
    let mut at = |m: &mut VersusMachine, frame: u32| {
        while m.state.frame < frame {
            let f = m.state.frame + 1;
            while next < events.len() && events[next].0 < f {
                held[events[next].1] = events[next].2;
                next += 1;
            }
            sf_engine::run::apply_inputs(m, &held);
            m.step();
        }
        outcomes(m)
    };
    let fresh = |ken_sx, ryu_sx, rem, cells, score| Outcomes {
        p2_rem: rem,
        p2_cells: cells,
        score_hundreds: score,
        ken_sx,
        ryu_sx,
        ken_move: 1,
    };
    // Punch 1 at gap 0x84 (2473-2479) and punch 2 at gap 0x5c (2593-2599)
    // whiff: no damage, no score, Ken standing.
    assert_eq!(at(&mut m, 2600), fresh(0x122, 0xc6, 4, 11, 0), "two whiffs");
    // Punch 3 at gap 0x34: partial (loose-box) hit on 2716, 1 damage, +100.
    assert_eq!(
        at(&mut m, 2715),
        fresh(0x122, 0xee, 4, 11, 0),
        "before the hit frame"
    );
    let hit = at(&mut m, 2716);
    assert_eq!(
        (hit.p2_rem, hit.p2_cells, hit.score_hundreds),
        (3, 11, 1),
        "partial hit"
    );
    assert_eq!(hit.ken_move, 0x30, "reaction move 0x30 starts");
    // Ken's hit-stun pushes him +0x20 and returns him to the stance (2744).
    assert_eq!(
        at(&mut m, 2744),
        fresh(0x142, 0xee, 3, 11, 1),
        "stun over, stance again"
    );
    // Punch 4 at gap 0x2c: clean (tight-box) hit on 2846, 2 damage.
    let hit = at(&mut m, 2846);
    assert_eq!(
        (hit.p2_rem, hit.p2_cells, hit.score_hundreds),
        (1, 11, 2),
        "clean hit"
    );
    // Punch 5 at gap 0x4c after the pushback whiffs.
    assert_eq!(
        at(&mut m, 3000),
        fresh(0x162, 0x116, 1, 11, 2),
        "whiff after pushback"
    );
    // Punch 6 at gap 0x24: clean hit on 3136 crossing into the next cell;
    // the stun pushes Ken against the screen edge, and the camera scrolls
    // (+0x10 at 3137, +8 at 3162) instead of moving him further.
    let hit = at(&mut m, 3136);
    assert_eq!(
        (hit.p2_rem, hit.p2_cells, hit.score_hundreds),
        (3, 10, 3),
        "cell lost"
    );
    assert_eq!(m.state.fg_scroll, 0x1d40, "camera still at the fixture");
    at(&mut m, 3137);
    assert_eq!(m.state.fg_scroll, 0x1d50, "camera push +0x10");
    assert_eq!(
        at(&mut m, 3164),
        fresh(0x166, 0x126, 3, 10, 3),
        "stance after the edge push"
    );
    assert_eq!(m.state.fg_scroll, 0x1d58, "camera push +8");
    assert!(
        m.unrecovered().is_empty(),
        "translated paths only: {:x?}",
        m.unrecovered()
    );
}
