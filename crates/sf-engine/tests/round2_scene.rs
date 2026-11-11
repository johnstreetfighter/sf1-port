//! Acceptance test for ticket #26: Retsu wins round 2 natively, through the
//! KO. From the round-2 fixture of sf-round2-02 (frame 3900, P1 idle) the
//! native one-player machine plays the round until Ryu is knocked out
//! (4568), and the comparison boundary finds no observable difference over
//! 3901..=4568: per-frame screen hashes, checkpoint pixels and
//! video/palette RAM every frame (health bars, TIME HUD, sprites), and
//! sound-latch timing (Retsu's attack sounds, the hit impacts, the KO
//! voice). The round result that follows is ticket #28's
//! (tests/round_result.rs).
//!
//! Consumes local-only data; skips on a clean checkout.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;

const SCENARIO: &str = "../../tools/reference/scenarios/sf_round2_02.json";
const REF_A: &str = "../../tools/reference/local/captures/round2-02-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIXTURE: u32 = 3900;
const KO: u32 = 4568;

fn present() -> bool {
    Path::new(REF_A).is_dir() && Path::new(ROMS).is_dir()
}

#[test]
fn round_two_matches_the_reference_on_every_observable_through_the_ko() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = sf_engine::run::ScratchDir::new("sf-engine-round2");
    sf_engine::run::run_cpu_fight_capture(
        Path::new(REF_A),
        &assets,
        &out,
        "sf-round2-02",
        FIXTURE,
        KO,
    )
    .expect("native capture runs on translated paths only");
    let scope = CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(KO as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(REF_A).unwrap(),
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
        panic!("round 2 must match the reference, got {outcome:?}");
    };
    assert_eq!(
        (frames_compared, checkpoints_compared),
        ((KO - FIXTURE) as u64, (KO - FIXTURE) as usize)
    );
}

#[test]
fn the_reference_has_ryu_knocked_out_at_the_ko_frame() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let ko = std::fs::read(Path::new(REF_A).join(format!("ram_{KO:06}_work.bin"))).unwrap();
    let ryu = u16::from_be_bytes([ko[0x6b0 + 0x10], ko[0x6b0 + 0x11]]) as usize;
    assert_eq!(
        ko[0x6000 + ryu + 8],
        8,
        "the reference has Ryu knocked out at {KO}"
    );
}
