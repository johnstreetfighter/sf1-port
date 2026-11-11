//! Acceptance test for ticket #25: Retsu's follow-up decisions (0x45630)
//! and attacks. From the round-2 fixture of the fixed fight (sf-round2-02,
//! P1 idle; at 3900 Ryu is in object slot 0xa00, Retsu in 0x400) the
//! native run matches the owned one-player RAM (object RAM, both player
//! structures, AI tables, timer, scheduler, queues, random state, camera)
//! after every frame from 3901 to 4568, while Retsu walks, attacks (moves
//! 0x14, 0x18, 0x0f) and jumps, Ryu's health drains through Retsu's hit
//! functions, and Ryu is knocked out (4568); since tickets #28 and #45 the
//! run goes on through the KO path, round result, CPU match win (0x3eefa),
//! ranking check and continue countdown to the scenario end at 5600.
//!
//! Consumes local-only data; skips on a clean checkout.

use std::path::Path;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const REF_A: &str = "../../tools/reference/local/captures/round2-02-run-a";
const REF_B: &str = "../../tools/reference/local/captures/round2-02-run-b";

fn present() -> bool {
    Path::new(REF_A).is_dir() && Path::new(REF_B).is_dir() && Path::new(ROMS).is_dir()
}

#[test]
fn reference_runs_repeat_exactly() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let read = |dir: &str, f: &str| std::fs::read(Path::new(dir).join(f)).unwrap();
    assert_eq!(read(REF_A, "frames.tsv"), read(REF_B, "frames.tsv"));
    for f in [3900, 4200, 4568, 5600] {
        let name = format!("ram_{f:06}_work.bin");
        assert_eq!(read(REF_A, &name), read(REF_B, &name), "{name}");
    }
}

#[test]
fn retsu_fights_round_two_natively_until_the_ko() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let outcome = sf_engine::run::compare_cpu_ram(Path::new(REF_A), rom, 3900, 5600).unwrap();
    assert_eq!(
        outcome,
        sf_engine::run::CpuRamOutcome::Match { frames: 1700 }
    );
}
