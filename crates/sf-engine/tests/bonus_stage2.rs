//! Ticket #130 (`F58`): Power/timing bonus after stage 2 (`0x41a02..0x41f5c`
//! and data tables `0x41f5e..0x41fe6`).
//!
//! Verifies:
//! 1. `sf-win-06` (`24853..=26372`, `1,519` frames): reference runs repeat
//!    byte-for-byte, and native execution matches every captured observable
//!    (`screen_sha256`, `sound_latch`, `latch_cycles`, checkpoint
//!    `pixels_*.rgba`) and owned RAM byte across the stage-2 bonus success path
//!    (`0x41d6a..0x41ed6`, tier 2 break `0x41fae`, 5,000 pts `0x41412`, crash
//!    sound `0x0017` from `0x41de4`, `0x3efc4` at `25582`) and through the
//!    flight to stage 3 (England), VS screen (`RYU VS BIRDIE`), stage-3 intro, and
//!    `ROUND 1` into Birdie's first fight-loop frame (`0x3eafa`, `0x17a9c`) at
//!    `26372`.
//! 2. `sf-bonus-03` (`24853..=25760`, `907` frames): reference runs repeat
//!    byte-for-byte, match `sf-win-06` through the divergence (`24853..=25027`),
//!    and native execution matches every captured observable and owned RAM byte
//!    across the gauge-zero reset (`0x41cbc..0x41cee` into `0x41cf0`), timer-3
//!    expiry (`0x41570: clr.w D2`), phase-1 wind-up sleeps (`0x41d30..0x41d4e`),
//!    and failure path (`0x41eda..0x41f5c`) into `0x3efc4` (`25760`), then
//!    continues natively through the flight to stage 3, VS screen, stage-3
//!    intro, and `ROUND 1` into Birdie's `0x17a9c` fight loop (`26550`).
//! 3. Negative-control test: tampering a bonus checkpoint pixel hash or owned
//!    RAM byte at frame `25089` (stage-2 tier-2 break start) fails at that exact
//!    frame.
//! 4. Both outcomes of the player-pointer branch (`0x41a08..0x41a24`,
//!    `GAME_MODE == 4` vs `!= 4`) and both outcomes of the palette-tail branch
//!    (`0x414f6..0x4150a`, `GAME_MODE == 2` vs `!= 2`), plus the stage-2 grid
//!    descriptor at `0x41f5e..0x41f74`.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, scenario_input_events,
    CpuRamOutcome, ScratchDir, INPUTS,
};

const WIN_SCENARIO: &str = "../../tools/reference/scenarios/sf_win_06.json";
const WIN_RUN_A: &str = "../../tools/reference/local/captures/win-06-run-a";
const WIN_RUN_B: &str = "../../tools/reference/local/captures/win-06-run-b";

const FAIL_SCENARIO: &str = "../../tools/reference/scenarios/sf_bonus_03.json";
const FAIL_RUN_A: &str = "../../tools/reference/local/captures/bonus-03-run-a";
const FAIL_RUN_B: &str = "../../tools/reference/local/captures/bonus-03-run-b";

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 24853;
const WIN_LAST: u32 = 26372;
const WIN_FRAMES: u32 = WIN_LAST - FIRST;

const FAIL_LAST: u32 = 25760;
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
fn win_06_bonus_stage2_and_stage3_arrival_reference_runs_repeat_exactly() {
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
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == WIN_FRAMES as u64 && checkpoints_compared == WIN_FRAMES as usize
        ),
        "sf-win-06 stage-2 bonus + stage-3 arrival: {outcome:?}"
    );
}

#[test]
fn native_win_06_bonus_stage2_and_stage3_arrival_matches_every_observable_and_owned_ram() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-06-bonus-stage2");
    assert_eq!(
        run_fight_capture(
            Path::new(WIN_RUN_A),
            &assets,
            &out,
            "sf-win-06",
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
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == WIN_FRAMES as u64 && checkpoints_compared == WIN_FRAMES as usize
        ),
        "sf-win-06 stage-2 bonus + stage-3 arrival: {outcome:?}"
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
    assert_eq!(
        ram_outcome,
        CpuRamOutcome::Match { frames: WIN_FRAMES },
        "sf-win-06 owned RAM over {FIRST}..={WIN_LAST}"
    );
}

#[test]
fn bonus_03_failure_reference_runs_repeat_and_match_win_06_through_divergence() {
    if !fail_present() || !win_present() {
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
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == FAIL_FRAMES as u64 && checkpoints_compared == FAIL_FRAMES as usize
        ),
        "sf-bonus-03: {outcome:?}"
    );

    // Identical to sf-win-06 on every frame and shared checkpoint through frame 25027
    // (before sf-win-06 presses p1_hp at 25027).
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(25027),
        fields: None,
    };
    let mut win_a = load_capture(Path::new(WIN_RUN_A)).unwrap();
    win_a.scenario_id = Some(scenario.scenario_id.clone());
    let prefix_outcome = compare(
        &scenario,
        &win_a,
        &load_capture(Path::new(FAIL_RUN_A)).unwrap(),
        CandidateKind::Reference,
        &prefix_scope,
    )
    .unwrap();
    assert!(
        matches!(
            prefix_outcome,
            Outcome::Match {
                frames_compared: 25028,
                checkpoints_compared: 175,
            }
        ),
        "sf-bonus-03 vs sf-win-06 prefix: {prefix_outcome:?}"
    );
}

#[test]
fn native_bonus_03_failure_matches_every_observable_owned_ram_and_reaches_lee() {
    if !fail_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-bonus-03-failure");
    assert_eq!(
        run_fight_capture(
            Path::new(FAIL_RUN_A),
            &assets,
            &out,
            "sf-bonus-03",
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
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == FAIL_FRAMES as u64 && checkpoints_compared == FAIL_FRAMES as usize
        ),
        "sf-bonus-03: {outcome:?}"
    );

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome = compare_fight_ram_with_inputs(
        Path::new(FAIL_RUN_A),
        rom.clone(),
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

    // Continue natively from 24853 with sf-bonus-03's inputs through gauge-zero
    // (25129), timer-3 expiry and phase-1 wind-up sleeps (25306..25309), failure
    // sequence (0x41eda..0x41f5c), 0x3efc4 stage step (25760), and the flight to
    // stage 3, VS screen, stage-3 intro, and ROUND 1 into Birdie's fight loop
    // (0x3eafa, move table 0x17a9c) at frame 26550.
    let mut m = FightMachine::from_fight_checkpoint(Path::new(FAIL_RUN_A), FIRST, rom).unwrap();
    let events = scenario_input_events(Path::new(FAIL_SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; INPUTS]);
    let mut windup_sleeps = Vec::new();
    let mut fail_pose_reached = false;
    while m.state.frame < 26550 {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        assert!(
            m.unrecovered().is_empty(),
            "frame {}: {:05x?}",
            m.state.frame,
            m.unrecovered()
        );
        let pos = m.task_positions();
        if pos.contains(&(1, 0x41d4e)) {
            windup_sleeps.push((m.state.frame, m.work_ram()[0xf0e]));
        }
        if pos.contains(&(1, 0x41eee)) {
            fail_pose_reached = true;
        }
    }
    assert_eq!(
        windup_sleeps,
        vec![(25306, 1), (25307, 1), (25308, 1), (25309, 1)],
        "expected phase-1 wind-up sleeps at 25306..25309"
    );
    assert!(
        fail_pose_reached,
        "expected failure pose 0x41eee to be reached"
    );
    assert_eq!(m.work_ram()[0x4003], 3);
    assert!(m.task_positions().contains(&(1, 0x3eafa)));
    assert_eq!(
        u32::from_be_bytes(m.work_ram()[0x4676..0x467a].try_into().unwrap()),
        0x17a9c
    );
}

#[test]
fn tampered_bonus_stage2_checkpoint_and_ram_are_caught_at_exact_frame() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const FRAME: u32 = 25089;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-bonus-stage2-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(WIN_RUN_A),
            &assets,
            &out,
            "sf-win-06",
            Some(Path::new(WIN_SCENARIO)),
            FIRST,
            FRAME,
        )
        .unwrap(),
        (FRAME - FIRST + 1) as u64,
    );
    // Tamper the checkpoint row and pixel buffer at frame 25089 (tier-2 break start).
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
        panic!("tampered stage-2 bonus checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Also verify owned bonus RAM tamper detection at frame 25089 (0xff8f1c).
    for entry in [
        "ram_025088_work.bin",
        "ram_025088_video.bin",
        "ram_025088_palette.bin",
        "ram_025089_work.bin",
    ] {
        std::fs::copy(Path::new(WIN_RUN_A).join(entry), out.join(entry)).unwrap();
    }
    let work_path = out.join("ram_025089_work.bin");
    let mut work = std::fs::read(&work_path).unwrap();
    work[0x0f1c] ^= 1;
    std::fs::write(&work_path, &work).unwrap();
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome =
        compare_fight_ram_with_inputs(&out, rom, Some(Path::new(WIN_SCENARIO)), FRAME - 1, FRAME)
            .unwrap();
    let CpuRamOutcome::Mismatch(msg) = ram_outcome else {
        panic!("tampered stage-2 bonus RAM unexpectedly matched: {ram_outcome:?}");
    };
    assert!(
        msg.contains("frame 25089: bonus stage state differs first at 0xff8f1c"),
        "{msg}"
    );
}

#[test]
fn mode_2_and_mode_4_branches_and_copy2_grid_table_are_exercised() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();

    // Mode 2 (0x41a08 -> 0x41a14: 0xff8f14 = 0xff86b0; 0x414f6 -> 0x414fe: 0x41518).
    let mut m2 =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), FIRST, rom.clone()).unwrap();
    m2.step(); // frame 24854: bonus_japan_start(BonusCopy::Stage2) -> sleeps at 0x41aa6
    assert!(m2.task_positions().contains(&(1, 0x41aa6)));
    assert_eq!(
        u32::from_be_bytes(m2.work_ram()[0xf14..0xf18].try_into().unwrap()),
        0xff86b0
    );
    // Verify Copy 2's 2x4 grid descriptor at 0x41f5e..0x41f74 populated slots 0xffe100..0xffe300
    // with base attr 0x040f and codes [0x3000, 0x3002, 0x3002|flip, 0x3000|flip, ...].
    assert_eq!(
        u16::from_be_bytes(m2.state.objectram[0x100..0x102].try_into().unwrap()),
        0x3000
    );
    assert_eq!(
        u16::from_be_bytes(m2.state.objectram[0x102..0x104].try_into().unwrap()),
        0x040f
    );
    assert_eq!(
        u16::from_be_bytes(m2.state.objectram[0x180..0x182].try_into().unwrap()),
        0x3002
    );
    assert_eq!(
        u16::from_be_bytes(m2.state.objectram[0x182..0x184].try_into().unwrap()),
        0x050f
    );
    m2.step(); // frame 24855: bonus_load_palette_tail (0x414f0)
    assert_eq!(&m2.state.paletteram[0x3e0..0x400], &rom[0x41518..0x41538]);

    // Mode 4 (0x41a08 -> 0x41a1e: 0xff8f14 = 0xff86da; 0x414f6 -> 0x41504: 0x41538).
    let mut m4 =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), FIRST, rom.clone()).unwrap();
    m4.set_work_byte(0xffc002, 4);
    m4.step(); // frame 24854: bonus_japan_start(BonusCopy::Stage2)
    assert!(m4.task_positions().contains(&(1, 0x41aa6)));
    assert_eq!(
        u32::from_be_bytes(m4.work_ram()[0xf14..0xf18].try_into().unwrap()),
        0xff86da
    );
    m4.step(); // frame 24855: bonus_load_palette_tail (0x414f0)
    assert_eq!(&m4.state.paletteram[0x3e0..0x400], &rom[0x41538..0x41558]);
}
