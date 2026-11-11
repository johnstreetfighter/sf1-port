//! Ticket #69 (F29): Power/timing bonus after Japan (`0x40d30..0x412a4` and
//! helpers `0x3b96c`, `0x3ef96..0x3efd8`, `0x4141a..0x4159c`).
//!
//! Verifies:
//! 1. `sf-win-02` Japan bonus stage (`9277..=10028`, reaching `0x3efc4 ->
//!    0x3e59c -> 0x3f812` on frame `10029`): native execution matches every
//!    captured observable (`screen_sha256`, `sound_latch`, `latch_cycles`,
//!    checkpoint `pixels_*.rgba`) and owned RAM byte across the success path
//!    (`0x410a4..0x41220`).
//! 2. `sf-bonus-01` Japan bonus stage (`9277..=10179`, reaching `0x3efc4 ->
//!    0x3e59c -> 0x3f812` on frame `10180`): reference runs repeat byte-for-byte,
//!    and native execution matches every captured observable and owned RAM byte
//!    across the gauge-zero reset (`0x40fea..0x4101c`), timer-expiry wind-up
//!    sleeps (`0x41072..0x41078`), and failure path (`0x41226..0x412a4`).
//! 3. Negative-control test: tampering a bonus checkpoint pixel hash or owned
//!    RAM byte fails at that exact frame.
//! 4. Both outcomes of the player-pointer branch (`0x40d36..0x40d52`,
//!    `GAME_MODE == 4` vs `!= 4`) and both outcomes of the palette-tail branch
//!    (`0x414f6..0x4150a`, `GAME_MODE == 2` vs `!= 2`), plus `0x3efc4..0x3efd8`
//!    (`STAGE_NO == 5` vs `!= 5`).

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, scenario_input_events,
    CpuRamOutcome, ScratchDir, INPUTS,
};

const WIN_SCENARIO: &str = "../../tools/reference/scenarios/sf_win_02.json";
const WIN_RUN_A: &str = "../../tools/reference/local/captures/win-02-run-a";
const WIN_RUN_B: &str = "../../tools/reference/local/captures/win-02-run-b";

const FAIL_SCENARIO: &str = "../../tools/reference/scenarios/sf_bonus_01.json";
const FAIL_RUN_A: &str = "../../tools/reference/local/captures/bonus-01-run-a";
const FAIL_RUN_B: &str = "../../tools/reference/local/captures/bonus-01-run-b";

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 9277;
const WIN_LAST: u32 = 10029;
const WIN_FRAMES: u32 = WIN_LAST - FIRST;

const FAIL_LAST: u32 = 10180;
const FAIL_FRAMES: u32 = FAIL_LAST - FIRST;

fn win_present() -> bool {
    [WIN_RUN_A, WIN_RUN_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn fail_present() -> bool {
    [FAIL_RUN_A, FAIL_RUN_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn scope(last: u32) -> CompareScope {
    CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(last as u64),
        fields: None,
    }
}

#[test]
fn win_02_bonus_japan_reference_runs_repeat_exactly() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(WIN_SCENARIO).unwrap();
    let outcome = compare(
        &scenario,
        &load_capture(Path::new(WIN_RUN_A)).unwrap(),
        &load_capture(Path::new(WIN_RUN_B)).unwrap(),
        CandidateKind::Reference,
        &scope(WIN_LAST),
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == WIN_FRAMES as u64 && checkpoints_compared == WIN_FRAMES as usize),
        "sf-win-02 bonus: {outcome:?}"
    );
}

#[test]
fn native_win_02_bonus_japan_matches_every_observable_and_owned_ram() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-02-bonus-japan");
    assert_eq!(
        run_fight_capture(
            Path::new(WIN_RUN_A),
            &assets,
            &out,
            "sf-win-02",
            Some(Path::new(WIN_SCENARIO)),
            FIRST,
            WIN_LAST,
        )
        .unwrap(),
        (WIN_FRAMES + 1) as u64,
    );
    let native_scope = CompareScope {
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        ..scope(WIN_LAST)
    };
    let outcome = compare(
        &load_scenario(WIN_SCENARIO).unwrap(),
        &load_capture(Path::new(WIN_RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &native_scope,
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == WIN_FRAMES as u64 && checkpoints_compared == WIN_FRAMES as usize),
        "sf-win-02 bonus: {outcome:?}"
    );

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome = compare_fight_ram_with_inputs(
        Path::new(WIN_RUN_A),
        rom,
        Some(Path::new(WIN_SCENARIO)),
        FIRST,
        WIN_LAST,
    )
    .unwrap();
    assert_eq!(ram_outcome, CpuRamOutcome::Match { frames: WIN_FRAMES });
}

#[test]
fn bonus_01_failure_reference_runs_repeat_exactly() {
    if !fail_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(FAIL_SCENARIO).unwrap();
    let outcome = compare(
        &scenario,
        &load_capture(Path::new(FAIL_RUN_A)).unwrap(),
        &load_capture(Path::new(FAIL_RUN_B)).unwrap(),
        CandidateKind::Reference,
        &scope(FAIL_LAST),
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == FAIL_FRAMES as u64 && checkpoints_compared == FAIL_FRAMES as usize),
        "sf-bonus-01: {outcome:?}"
    );
}

#[test]
fn native_bonus_01_failure_matches_every_observable_and_owned_ram() {
    if !fail_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-bonus-01-failure");
    assert_eq!(
        run_fight_capture(
            Path::new(FAIL_RUN_A),
            &assets,
            &out,
            "sf-bonus-01",
            Some(Path::new(FAIL_SCENARIO)),
            FIRST,
            FAIL_LAST,
        )
        .unwrap(),
        (FAIL_FRAMES + 1) as u64,
    );
    let native_scope = CompareScope {
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        ..scope(FAIL_LAST)
    };
    let outcome = compare(
        &load_scenario(FAIL_SCENARIO).unwrap(),
        &load_capture(Path::new(FAIL_RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &native_scope,
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == FAIL_FRAMES as u64 && checkpoints_compared == FAIL_FRAMES as usize),
        "sf-bonus-01: {outcome:?}"
    );

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome = compare_fight_ram_with_inputs(
        Path::new(FAIL_RUN_A),
        rom,
        Some(Path::new(FAIL_SCENARIO)),
        FIRST,
        FAIL_LAST,
    )
    .unwrap();
    assert_eq!(
        ram_outcome,
        CpuRamOutcome::Match {
            frames: FAIL_FRAMES
        }
    );
}

#[test]
fn tampered_bonus_checkpoint_and_ram_are_caught_at_exact_frame() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const FRAME: u32 = 9457;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-bonus-japan-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(WIN_RUN_A),
            &assets,
            &out,
            "sf-win-02",
            Some(Path::new(WIN_SCENARIO)),
            FIRST,
            FRAME,
        )
        .unwrap(),
        (FRAME - FIRST + 1) as u64,
    );
    // Tamper the checkpoint row and pixel buffer at frame 9457 (plank break start).
    let checkpoints = out.join("checkpoints.tsv");
    let text = std::fs::read_to_string(&checkpoints).unwrap();
    let prefix = format!("{FRAME}\t");
    let line = text
        .lines()
        .find(|l| l.starts_with(&prefix))
        .unwrap()
        .to_string();
    let parts: Vec<&str> = line.split('\t').collect();
    let crc = u32::from_str_radix(parts[1], 16).unwrap() ^ 1;
    let altered_line = format!("{FRAME}\t{crc:08x}\t{}", parts[2]);
    std::fs::write(&checkpoints, text.replace(&line, &altered_line)).unwrap();

    let pixels_path = out.join(format!("pixels_{FRAME:06}.rgba"));
    let mut pixels = std::fs::read(&pixels_path).unwrap();
    pixels[0] ^= 0xff;
    std::fs::write(&pixels_path, &pixels).unwrap();

    let negative_scope = CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(FRAME as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(WIN_SCENARIO).unwrap(),
        &load_capture(Path::new(WIN_RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &negative_scope,
    )
    .unwrap();
    let Outcome::Mismatch(mismatch) = outcome else {
        panic!("tampered bonus checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Also verify owned bonus RAM tamper detection at frame 9457 (0xff8f1c).
    for entry in [
        "ram_009456_work.bin",
        "ram_009456_video.bin",
        "ram_009456_palette.bin",
        "ram_009457_work.bin",
    ] {
        std::fs::copy(Path::new(WIN_RUN_A).join(entry), out.join(entry)).unwrap();
    }
    let work_path = out.join("ram_009457_work.bin");
    let mut work = std::fs::read(&work_path).unwrap();
    work[0x0f1c] ^= 1;
    std::fs::write(&work_path, &work).unwrap();
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome =
        compare_fight_ram_with_inputs(&out, rom, Some(Path::new(WIN_SCENARIO)), FRAME - 1, FRAME)
            .unwrap();
    let CpuRamOutcome::Mismatch(msg) = ram_outcome else {
        panic!("tampered bonus RAM unexpectedly matched: {ram_outcome:?}");
    };
    assert!(
        msg.contains("frame 9457: bonus stage state differs first at 0xff8f1c"),
        "{msg}"
    );
}

#[test]
fn mode_2_and_mode_4_branches_and_stage_step_outcomes_are_exercised() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();

    // Mode 2 (0x40d36 -> 0x40d50: 0xff8f14 = 0xff86b0; 0x414f6 -> 0x414fe: 0x41518).
    let mut m2 =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), FIRST, rom.clone()).unwrap();
    m2.step(); // frame 9278: bonus_japan_start
    assert_eq!(
        u32::from_be_bytes(m2.work_ram()[0xf14..0xf18].try_into().unwrap()),
        0xff86b0
    );
    m2.step(); // frame 9279: bonus_load_palette_tail
    assert_eq!(&m2.state.paletteram[0x3e0..0x400], &rom[0x41518..0x41538]);

    // Mode 4 (0x40d36 -> 0x40d48: 0xff8f14 = 0xff86da; 0x414f6 -> 0x41504: 0x41538).
    let mut m4 =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), FIRST, rom.clone()).unwrap();
    m4.set_work_byte(0xffc002, 4);
    m4.step(); // frame 9278: bonus_japan_start
    assert_eq!(
        u32::from_be_bytes(m4.work_ram()[0xf14..0xf18].try_into().unwrap()),
        0xff86da
    );
    m4.step(); // frame 9279: bonus_load_palette_tail
    assert_eq!(&m4.state.paletteram[0x3e0..0x400], &rom[0x41538..0x41558]);

    // 0x3efc4..0x3efd8: stage 0 -> 1 enters 0x3e59c and slot 2 enters 0x3f812..0x3f87c;
    // stage 4 -> 5 branches to 0x3f2d0 (versus_game_over -> name_entry_start at 0x42486).
    let mut m_stage =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), WIN_LAST - 1, rom.clone())
            .unwrap();
    let events = scenario_input_events(Path::new(WIN_SCENARIO)).unwrap();
    let _ = events;
    apply_inputs(&mut m_stage, &[false; INPUTS]);
    m_stage.step();
    assert_eq!(m_stage.work_ram()[0x4003], 1);
    assert!(m_stage.unrecovered().is_empty());
    assert!(m_stage.task_positions().contains(&(2, 0x3f87c)));

    let mut m_stage5 =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), WIN_LAST - 1, rom).unwrap();
    m_stage5.set_work_byte(0xffc003, 4);
    apply_inputs(&mut m_stage5, &[false; INPUTS]);
    m_stage5.step();
    assert_eq!(m_stage5.work_ram()[0x4003], 5);
    assert!(m_stage5.unrecovered().is_empty());
    assert!(m_stage5.task_positions().contains(&(1, 0x42552)));
}
