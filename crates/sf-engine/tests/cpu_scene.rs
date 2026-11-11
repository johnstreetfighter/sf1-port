//! Acceptance test for ticket #12: the native engine advances the fixed
//! fight's first CPU decision (sf-cpu-01: from the checkpoint-2389 fixture,
//! the decision at 2390 and Retsu's forward jump through his landing stance
//! at 2433) and the comparison boundary finds no observable difference
//! against the pinned MAME reference over 2390..=2433: per-frame screen
//! hashes, full-pixel checkpoints every frame, video/palette RAM dumps, and
//! sound-latch timing. The decision runs on the translated AI (0x40bd4)
//! from the fixture's AI tables and random state.
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::cpu::CpuMachine;

const SCENARIO: &str = "../../tools/reference/scenarios/sf_cpu_01.json";
const REF_A: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const REF_B: &str = "../../tools/reference/local/captures/cpu-01-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// The fixture row (2389) is excluded from the per-frame walk: its screen
/// hash is the scanout of 2388 (FIGHT! still shown), which a post-erase
/// fixture cannot reproduce; the decision frame 2390 is the first compared.
fn scope() -> CompareScope {
    CompareScope {
        from: Some(2390),
        to: Some(2433),
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
fn native_cpu_decision_matches_the_reference_over_the_interval() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = sf_engine::run::ScratchDir::new("sf-engine-cpu-capture");
    sf_engine::run::run_cpu_capture(Path::new(REF_A), &assets, &out, "sf-cpu-01")
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
        panic!("native CPU decision must match the reference, got {outcome:?}");
    };
    assert_eq!(*frames_compared, 44);
    assert_eq!(*checkpoints_compared, 44);
}

#[test]
fn the_decision_is_a_forward_jump_from_the_near_table() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let mut m = CpuMachine::from_checkpoint_2389(Path::new(REF_A), rom).expect("fixture");
    let retsu = m.cpu_entity() as usize;
    assert_eq!(retsu, 0xa00, "Retsu occupies the P2 entity");
    // The documented initial conditions (docs/reference/cpu-decision.md).
    let w = m.work_ram();
    assert_eq!(
        &w[0xe4a..0xe4c],
        &[0xa9, 0xeb],
        "random bytes 0/1 seen by the AI"
    );
    assert_eq!(w[0x4058 + 9], 0, "range class for random nibble 9");
    assert_eq!(
        u16::from_be_bytes([w[0x4068], w[0x4069]]) & 0xff,
        0x4c,
        "range 0x4c"
    );
    assert_eq!(
        u16::from_be_bytes([w[0x406e], w[0x406f]]),
        0x47,
        "near limit"
    );
    assert_eq!(w[0x4074 + 0xb], 0x0a, "near table entry 0xb: move 0x0a");
    let obj = &m.state.objectram;
    assert_eq!(obj[retsu + 9], 0x00, "Retsu idle before the fight loop");

    m.step(); // 2390: the decision
    let obj = &m.state.objectram;
    assert_eq!(obj[retsu + 0xa], 0x0a, "move 0x0a started");
    assert_eq!(obj[retsu + 9], 0x81, "jump state");
    assert_eq!(
        u16::from_be_bytes([obj[retsu], obj[retsu + 1]]),
        0x112b,
        "anticipation pose"
    );
    assert_eq!(
        m.work_ram()[0x4664 + 0x18],
        0,
        "range class kept in the CPU structure"
    );

    while m.state.frame < 2433 {
        m.step();
    }
    let obj = &m.state.objectram;
    assert_eq!(
        u16::from_be_bytes([obj[retsu], obj[retsu + 1]]),
        0x1002,
        "landing stance"
    );
    assert_eq!(
        u16::from_be_bytes([obj[retsu + 6], obj[retsu + 7]]),
        0xb6,
        "landed at x 0xb6"
    );
    assert!(
        m.unrecovered().is_empty(),
        "translated paths only: {:x?}",
        m.unrecovered()
    );

    // One frame later the stream calls the follow-up decision (0x45630),
    // translated since ticket #25 (tests/cpu_follow_up.rs).
    m.step();
    assert!(
        m.unrecovered().is_empty(),
        "translated paths only: {:x?}",
        m.unrecovered()
    );
}
