//! Acceptance test for ticket #24: the round-timer task (slot 0, 0x3f4e2)
//! predicts its own memory (0xff8e16..0xff8e36: flags, TIME digits, frame
//! counter, round-time setting, score tallies) one frame at a time from the
//! reference's work RAM, on every frame of three captures: the fixed
//! fight's round-1 KO window (sf-round-end-01, 3300..=3700: TIME keeps
//! ticking after the KO while the fight loop no longer draws it), a long
//! versus scene (sf-blocks-01) and round 2 (sf-round2-01). The task runs
//! first in slot order, so the other tasks' writes of frame f-1 are what
//! it sees at frame f. (Inside the scene tests the whole task runs under
//! the scheduler and its memory is compared with the owned RAM.)
//!
//! Consumes local-only data; skips on a clean checkout.

use std::path::Path;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const CAPTURES: &str = "../../tools/reference/local/captures";

fn mismatches(capture: &str, first: u32, last: u32) -> Option<Vec<u32>> {
    let dir = Path::new(CAPTURES).join(capture);
    if !dir.is_dir() || !Path::new(ROMS).is_dir() {
        return None;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let (a, b) = sf_engine::run::TIMER_RAM;
    let (a, b) = ((a - 0xff8000) as usize, (b - 0xff8000) as usize);
    let mut bad = Vec::new();
    for f in first + 1..=last {
        let before = std::fs::read(dir.join(format!("ram_{:06}_work.bin", f - 1))).unwrap();
        let after = std::fs::read(dir.join(format!("ram_{f:06}_work.bin"))).unwrap();
        let predicted = sf_engine::run::timer_pass(rom.clone(), &before).unwrap();
        if predicted[a..b] != after[a..b] {
            bad.push(f);
        }
    }
    Some(bad)
}

#[test]
fn the_timer_task_predicts_its_memory_on_every_frame() {
    for (capture, first, last) in [
        ("round-end-run-a", 3300, 3700),
        ("blocks-01-run-a", 2450, 4320),
        ("round2-run-a", 3900, 4100),
    ] {
        let Some(bad) = mismatches(capture, first, last) else {
            eprintln!("skipping: {capture} or ROMs not present");
            return;
        };
        assert!(bad.is_empty(), "{capture}: frames {bad:?}");
    }
}

#[test]
fn the_round_end_window_repeats_exactly() {
    let (a, b) = (
        Path::new(CAPTURES).join("round-end-run-a"),
        Path::new(CAPTURES).join("round-end-run-b"),
    );
    if !a.is_dir() || !b.is_dir() {
        eprintln!("skipping: round-end captures not present");
        return;
    }
    assert_eq!(
        std::fs::read(a.join("frames.tsv")).unwrap(),
        std::fs::read(b.join("frames.tsv")).unwrap()
    );
    for f in [3300, 3328, 3500, 3700] {
        let name = format!("ram_{f:06}_work.bin");
        assert_eq!(
            std::fs::read(a.join(&name)).unwrap(),
            std::fs::read(b.join(&name)).unwrap(),
            "{name}"
        );
    }
}
