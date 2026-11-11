//! Ticket #124 (F52): 10-second concrete-block bonus after stage 1
//! (`0x41740..0x419fc`, `0x44174` / `0x44192`, and effect handlers `0x434e4`,
//! `0x4352c`, `0x4357a`).
//!
//! Verifies:
//! 1. `sf-win-04` (`17483..=19380`, `1,897` frames): reference runs repeat
//!    byte-for-byte, and native execution matches every captured observable
//!    (`screen_sha256`, `sound_latch`, `latch_cycles`, checkpoint
//!    `pixels_*.rgba`) and owned RAM byte across the 10-second timeout path
//!    (`0x14f74, 0x96` at `18287`, clear at `18407`, `0x3efc4` at `18528`) and
//!    through the flight to China, VS screen, stage 2's intro, and `ROUND 1`
//!    into Lee's fight loop (`0x3eafa`, `0x175bc`) at `19380`.
//! 2. `sf-bonus-02` (`17483..=18107`, `624` frames): reference runs repeat
//!    byte-for-byte, match `sf-win-04` through the divergence (`17483..=17594`),
//!    and native execution matches every captured observable and owned RAM byte
//!    across all three stack breaks (`0x18e24` at `17667`, `0x18e64` at `17768`,
//!    `0x18e44` at `17833`), the `0x3baf6` early exit (`17834`), settle
//!    (`17857`), `BONUS PTS. 5000` (`17867`), clear (`17987`), and `0x3efc4`
//!    (`18107`), then continues natively through the flight, VS, stage-2 intro,
//!    and `ROUND 1` into Lee's `0x175bc` fight loop (`18959`).
//! 3. Negative-control test: tampering a checkpoint pixel hash or owned RAM
//!    byte at frame `17667` fails at that exact frame.
//! 4. Both outcomes (`GAME_MODE != 4` vs `GAME_MODE == 4`, unexercised) of the
//!    three player-selection checks at `0x4175e`, `0x418ac`, and `0x41962`.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{compare_fight_ram_with_inputs, run_fight_capture, CpuRamOutcome, ScratchDir};

const WIN_SCENARIO: &str = "../../tools/reference/scenarios/sf_win_04.json";
const WIN_RUN_A: &str = "../../tools/reference/local/captures/win-04-run-a";
const WIN_RUN_B: &str = "../../tools/reference/local/captures/win-04-run-b";

const BREAK_SCENARIO: &str = "../../tools/reference/scenarios/sf_bonus_02.json";
const BREAK_RUN_A: &str = "../../tools/reference/local/captures/bonus-02-run-a";
const BREAK_RUN_B: &str = "../../tools/reference/local/captures/bonus-02-run-b";

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 17483;
const WIN_LAST: u32 = 19380;
const WIN_FRAMES: u32 = WIN_LAST - FIRST;

const BREAK_LAST: u32 = 18107;
const BREAK_FRAMES: u32 = BREAK_LAST - FIRST;

fn win_present() -> bool {
    [WIN_RUN_A, WIN_RUN_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn break_present() -> bool {
    [BREAK_RUN_A, BREAK_RUN_B, ROMS]
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
fn win_04_bonus_stage1_and_stage2_arrival_reference_runs_repeat_exactly() {
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
        "sf-win-04 stage-1 bonus + stage-2 arrival: {outcome:?}"
    );
}

#[test]
fn native_win_04_bonus_stage1_timeout_and_stage2_arrival_matches_every_observable_and_owned_ram() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-04-bonus-stage1");
    assert_eq!(
        run_fight_capture(
            Path::new(WIN_RUN_A),
            &assets,
            &out,
            "sf-win-04",
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
        "sf-win-04 stage-1 bonus + stage-2 arrival: {outcome:?}"
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
fn bonus_02_break_all_reference_runs_repeat_and_match_win_04_through_divergence() {
    if !break_present() || !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(BREAK_SCENARIO).unwrap();
    let outcome = compare(
        &scenario,
        &load_capture(Path::new(BREAK_RUN_A)).unwrap(),
        &load_capture(Path::new(BREAK_RUN_B)).unwrap(),
        CandidateKind::Reference,
        &scope(BREAK_LAST),
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == BREAK_FRAMES as u64 && checkpoints_compared == BREAK_FRAMES as usize),
        "sf-bonus-02: {outcome:?}"
    );

    // Identical to sf-win-04 on every frame and shared checkpoint through frame 17594
    // (when 0x4191c starts the 10s loop and P1 presses right in sf-bonus-02).
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(17594),
        fields: None,
    };
    let mut win_a = load_capture(Path::new(WIN_RUN_A)).unwrap();
    win_a.scenario_id = Some(scenario.scenario_id.clone());
    let prefix_outcome = compare(
        &scenario,
        &win_a,
        &load_capture(Path::new(BREAK_RUN_A)).unwrap(),
        CandidateKind::Reference,
        &prefix_scope,
    )
    .unwrap();
    assert!(
        matches!(
            prefix_outcome,
            Outcome::Match {
                frames_compared: 17595,
                checkpoints_compared: 112
            }
        ),
        "sf-bonus-02 vs sf-win-04 prefix: {prefix_outcome:?}"
    );
}

#[test]
fn native_bonus_02_break_all_matches_every_observable_owned_ram_and_reaches_birdie() {
    if !break_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-bonus-02-break-all");
    assert_eq!(
        run_fight_capture(
            Path::new(BREAK_RUN_A),
            &assets,
            &out,
            "sf-bonus-02",
            Some(Path::new(BREAK_SCENARIO)),
            FIRST,
            BREAK_LAST,
        )
        .unwrap(),
        (BREAK_FRAMES + 1) as u64,
    );
    let native_scope = CompareScope {
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        ..scope(BREAK_LAST)
    };
    let outcome = compare(
        &load_scenario(BREAK_SCENARIO).unwrap(),
        &load_capture(Path::new(BREAK_RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &native_scope,
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == BREAK_FRAMES as u64 && checkpoints_compared == BREAK_FRAMES as usize),
        "sf-bonus-02: {outcome:?}"
    );

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome = compare_fight_ram_with_inputs(
        Path::new(BREAK_RUN_A),
        rom.clone(),
        Some(Path::new(BREAK_SCENARIO)),
        FIRST,
        BREAK_LAST,
    )
    .unwrap();
    assert_eq!(
        ram_outcome,
        CpuRamOutcome::Match {
            frames: BREAK_FRAMES
        }
    );

    // Verify each stack break on its exact frame and continue past 0x3efc4
    // (frame 18107) through the flight to China, VS screen, stage 2's intro,
    // and ROUND 1 into Lee's fight loop (0x175bc) at frame 18959.
    let mut m =
        FightMachine::from_fight_checkpoint(Path::new(BREAK_RUN_A), 17666, rom.clone()).unwrap();
    m.step(); // frame 17667: right stack (0x18e24 @ 0x100, handler 0x434e4) breaks
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x110..0x114].try_into().unwrap()),
        0x1943a,
        "slot 0x100 stream should switch to broken stream 0x1943a"
    );
    assert_eq!(m.state.objectram[0x109], 0);
    assert_eq!(m.state.objectram[0x10e], 0);
    assert_eq!(
        m.state.objectram[0x1c8], 2,
        "debris 0x18ec4 spawned and stepped in slot 0x1c0"
    );
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x1d0..0x1d4].try_into().unwrap()),
        0x1904a,
        "debris 0x18ec4 stream after first step"
    );
    assert_eq!(&m.work_ram()[0xf18..0xf20], &[0, 0, 0, 0, 1, 0, 0, 0]);
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 2, 1, 4, 9, 0, 0]);

    let mut m =
        FightMachine::from_fight_checkpoint(Path::new(BREAK_RUN_A), 17767, rom.clone()).unwrap();
    m.step(); // frame 17768: lower-left stack (0x18e64 @ 0x180, handler 0x4357a) breaks
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x190..0x194].try_into().unwrap()),
        0x19472,
        "slot 0x180 stream should switch to broken stream 0x19472"
    );
    assert_eq!(m.state.objectram[0x189], 0);
    assert_eq!(m.state.objectram[0x18e], 0);
    assert_eq!(
        m.state.objectram[0x1c8], 2,
        "debris 0x18ec4 spawned and stepped in slot 0x1c0"
    );
    assert_eq!(&m.work_ram()[0xf18..0xf20], &[0, 0, 0, 0, 2, 0, 0, 0]);
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 2, 1, 4, 9, 0, 0]);

    let mut m = FightMachine::from_fight_checkpoint(Path::new(BREAK_RUN_A), 17832, rom).unwrap();
    m.step(); // frame 17833: upper-left stack (0x18e44 @ 0x140, handler 0x4352c) breaks
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x150..0x154].try_into().unwrap()),
        0x19456,
        "slot 0x140 stream should switch to broken stream 0x19456"
    );
    assert_eq!(m.state.objectram[0x149], 0);
    assert_eq!(m.state.objectram[0x14e], 0);
    assert_eq!(
        m.state.objectram[0x1c8], 2,
        "debris 0x18ec4 spawned and stepped in slot 0x1c0"
    );
    assert_eq!(&m.work_ram()[0xf18..0xf20], &[0, 0, 0, 0, 5, 0, 0, 0]);
    // 0x4352c also adds 3,000 directly onto P1's score (0x43566 jsr $3ba0e.l).
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 2, 1, 7, 9, 0, 0]);

    m.step(); // frame 17834: 0x3baf6 early exit -> (1, 0x41962) settle loop
    assert!(m.task_positions().contains(&(1, 0x41962)));

    while m.state.frame < 17867 {
        m.step();
    }
    // Frame 17867: 10-frame post-settle drain finishes and enters 0x419b4
    // (120-frame BONUS PTS. display hold).
    assert!(m.task_positions().contains(&(1, 0x419b4)));
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 2, 1, 7, 9, 0, 0]);
    assert_eq!(&m.work_ram()[0x4012..0x401a], &[0, 0, 2, 1, 4, 9, 0, 0]);

    while m.state.frame < 18959 {
        m.step();
    }
    // Frame 18959: reaches Lee's fight loop (0x3eafa, move table 0x175bc).
    assert_eq!(m.work_ram()[0x4003], 2);
    assert!(m.task_positions().contains(&(1, 0x3eafa)));
    assert_eq!(
        u32::from_be_bytes(m.work_ram()[0x4676..0x467a].try_into().unwrap()),
        0x175bc
    );
    assert!(m.unrecovered().is_empty());

    // Continuing 120 frames into Lee's fight runs cleanly with Lee
    // translated (ticket #125, F53).
    for _ in 0..120 {
        m.step();
    }
    assert!(m.unrecovered().is_empty());
}

#[test]
fn tampered_bonus_stage1_checkpoint_and_ram_are_caught_at_exact_frame() {
    if !break_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const FRAME: u32 = 17667;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-bonus-stage1-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(BREAK_RUN_A),
            &assets,
            &out,
            "sf-bonus-02",
            Some(Path::new(BREAK_SCENARIO)),
            FIRST,
            FRAME,
        )
        .unwrap(),
        (FRAME - FIRST + 1) as u64,
    );
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

    let pixels = out.join(parts[2]);
    let mut rgba = std::fs::read(&pixels).unwrap();
    rgba[0] ^= 0xff;
    std::fs::write(&pixels, rgba).unwrap();

    let native_scope = CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(FRAME as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(BREAK_SCENARIO).unwrap(),
        &load_capture(Path::new(BREAK_RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &native_scope,
    )
    .unwrap();
    let Outcome::Mismatch(mismatch) = outcome else {
        panic!("tampered bonus checkpoint unexpectedly matched: {outcome:?}");
    };
    assert_eq!(mismatch.frame, FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Also tamper an owned work-RAM byte (0xff8f1c bonus thousands digit) at frame 17667.
    for entry in [
        "ram_017666_work.bin",
        "ram_017666_video.bin",
        "ram_017666_palette.bin",
        "ram_017667_work.bin",
    ] {
        std::fs::copy(Path::new(BREAK_RUN_A).join(entry), out.join(entry)).unwrap();
    }
    let work_path = out.join(format!("ram_{FRAME:06}_work.bin"));
    let mut work = std::fs::read(&work_path).unwrap();
    work[0x0f1c] ^= 1;
    std::fs::write(&work_path, work).unwrap();

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome =
        compare_fight_ram_with_inputs(&out, rom, Some(Path::new(BREAK_SCENARIO)), FRAME - 1, FRAME)
            .unwrap();
    let CpuRamOutcome::Mismatch(msg) = ram_outcome else {
        panic!("tampered bonus RAM unexpectedly matched: {ram_outcome:?}");
    };
    assert!(
        msg.contains("frame 17667: bonus stage state differs first at 0xff8f1c"),
        "{msg}"
    );
}

#[test]
fn mode_4_branches_select_p2_at_0x4175e_0x418ac_and_0x41962() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();

    // 1. 0x4175e..0x4177e (bonus_stage1_start): GAME_MODE == 4 calls 0x44192
    //    (place_bonus_player(P2_STRUCT)) and passes P2's entity (ew(0xff86da, 0x10))
    //    to 0x42f1e for 0x18e24, 0x18e44, 0x18e64 so their +0x24 target is 0xff86da.
    let mut m_p1 =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), FIRST, rom.clone()).unwrap();
    m_p1.step();
    assert_eq!(
        u32::from_be_bytes(m_p1.state.objectram[0x124..0x128].try_into().unwrap()),
        0xff86b0,
        "GAME_MODE == 2 should bind stack target +0x24 to P1_STRUCT (0xff86b0)"
    );

    let mut m_p2 =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), FIRST, rom.clone()).unwrap();
    m_p2.set_work_byte(0xffc002, 4); // GAME_MODE = 4 (1P on P2 side vs CPU, unexercised)
                                     // Configure mode-4 player/entity pointers as place_players (0x3e5d6) sets them:
                                     // P2_STRUCT (0xff86da): +0x0e = 0x0400 (CPU entity), +0x10 = 0x0a00 (P2 entity);
                                     // entity 0x0400: +0x38 = CPU_STRUCT (0xffc664), +0x3c = P2_STRUCT (0xff86da);
                                     // entity 0x0a00: +0x38 = P2_STRUCT (0xff86da), +0x3c = CPU_STRUCT (0xffc664).
    let p1_move_tbl = m_p2.work_ram()[0x6c2..0x6c6].to_vec();
    for (i, b) in p1_move_tbl.into_iter().enumerate() {
        m_p2.set_work_byte(0xff86ec + i as u32, b);
    }
    m_p2.set_work_byte(0xff86e8, 0x04);
    m_p2.set_work_byte(0xff86e9, 0x00);
    m_p2.set_work_byte(0xff86ea, 0x0a);
    m_p2.set_work_byte(0xff86eb, 0x00);
    m_p2.set_work_byte(0xff86de, 0x86);
    m_p2.set_work_byte(0xff86df, 0x8e);
    m_p2.set_work_byte(0xffc672, 0x0a);
    m_p2.set_work_byte(0xffc673, 0x00);
    m_p2.set_work_byte(0xffc674, 0x04);
    m_p2.set_work_byte(0xffc675, 0x00);
    m_p2.state.objectram[0x438..0x43c].copy_from_slice(&0x00ff_c664u32.to_be_bytes());
    m_p2.state.objectram[0x43c..0x440].copy_from_slice(&0x00ff_86dau32.to_be_bytes());
    m_p2.state.objectram[0xa38..0xa3c].copy_from_slice(&0x00ff_86dau32.to_be_bytes());
    m_p2.state.objectram[0xa3c..0xa40].copy_from_slice(&0x00ff_c664u32.to_be_bytes());
    m_p2.step(); // frame 17484: runs 0x41740..0x41812 with GAME_MODE == 4
    assert_eq!(
        u32::from_be_bytes(m_p2.state.objectram[0x124..0x128].try_into().unwrap()),
        0xff86da,
        "GAME_MODE == 4 should bind stack target +0x24 to P2_STRUCT (0xff86da)"
    );
    assert_eq!(
        m_p2.state.objectram[0xa37], 2,
        "0x44192 sets P2 entity +0x37 = 2 (face right)"
    );
    assert_eq!(
        m_p2.state.objectram[0xa0a], 1,
        "0x44192 starts stance move 1 on P2 entity"
    );

    // 2. 0x418ac..0x418c8 (bonus_stage1_frame): advance to 17594 (0x4191c) and
    //    verify P2 right input moves P2's entity 0xa00 in GAME_MODE == 4.
    while m_p2.state.frame < 17594 {
        m_p2.step();
    }
    let x_before = u16::from_be_bytes(m_p2.state.objectram[0xa06..0xa08].try_into().unwrap());
    m_p2.set_p2_direction(true, false, false, false);
    m_p2.step();
    m_p2.step();
    let x_after = u16::from_be_bytes(m_p2.state.objectram[0xa06..0xa08].try_into().unwrap());
    assert_eq!(
        m_p2.state.objectram[0xa0a], 2,
        "P2 right input starts walk move 2 in GAME_MODE == 4"
    );
    assert!(
        x_after > x_before,
        "P2 entity moves right in GAME_MODE == 4"
    );

    // 3. 0x41962..0x419b2 (bonus_stage1_settle_step): trigger 0x3baf6 early exit
    //    by setting 0xff8f1c = 5 in GAME_MODE == 4, verify 0x41962 checks P2's
    //    entity (0xffea09 bit 7: stays at 0x41962 while busy, advances to
    //    0x419a0 and 0x419b4 once clear).
    m_p2.set_p2_direction(false, false, false, false);
    m_p2.state.objectram[0xa09] = 0x81; // P2 move busy (+9 bit 7 set)
    m_p2.set_work_byte(0xff8f1c, 5);
    m_p2.step(); // exits 0x4191c via 0x3baf6 -> enters 0x41962 settle
    assert!(m_p2.task_positions().contains(&(1, 0x41962)));
    m_p2.step(); // 0x41962 sees P2 entity 0xa00 busy (+9 bit 7) -> stays at 0x41962
    assert!(m_p2.task_positions().contains(&(1, 0x41962)));
    m_p2.state.objectram[0xa09] = 0x01; // P2 move no longer busy (+9 bit 7 clear)
    m_p2.step(); // 0x41962 -> 0x419a0 (left = 10)
    assert!(m_p2.task_positions().contains(&(1, 0x419a0)));
    for _ in 0..10 {
        m_p2.step();
    }
    assert!(m_p2.task_positions().contains(&(1, 0x419b4)));
}
