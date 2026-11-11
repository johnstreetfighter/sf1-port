//! Ticket #133 (`F61`): 10-second concrete-block bonus after stage 3
//! (`0x41fe6..0x422b8`, templates `0x18e44`, `0x18e64`, `0x18e84`, `0x18ea4`,
//! and effect handlers `0x4352c`, `0x4357a`, `0x435c2`, `0x4360a`).
//!
//! Verifies:
//! 1. `sf-win-08` (`32315..=34184`, `1,869` frames): reference runs repeat
//!    byte-for-byte, and native execution matches every captured observable
//!    (`screen_sha256`, `sound_latch`, `latch_cycles`, checkpoint
//!    `pixels_*.rgba`) and owned RAM byte across the 10-second timeout path
//!    (`0x41fe6..0x422b8`, `0x14f74` / `0x96` at `33077`, `BONUS PTS. 0000` at
//!    `33219`, `0x3efc4` at `33359`) and through the flight to stage 4
//!    (Thailand), VS screen (`RYU VS ADON`), stage-4 intro, and `ROUND 1` into
//!    Adon's first fight-loop frame (`0x3eafa`, `0x17f7c`) at `34184`.
//! 2. `sf-bonus-04` (`32315..=33017`, `702` frames): reference runs repeat
//!    byte-for-byte, match `sf-win-08` through the divergence (`32315..=32426`),
//!    and native execution matches every captured observable and owned RAM byte
//!    across all four stack breaks (`0x18ea4` at `32498`, `0x18e84` at `32563`,
//!    `0x18e64` at `32678`, `0x18e44` at `32743`), the `8,000`-point `0x3baf6`
//!    early exit (`32744`), settle (`32767`), `BONUS PTS. 8000` (`32897`), and
//!    `0x3efc4` (`33017`), then continues natively through the flight to
//!    Thailand, VS screen, stage-4 intro, and `ROUND 1` into Adon's `0x17f7c`
//!    fight loop (`33842`).
//! 3. Negative-control test: tampering a checkpoint pixel hash or owned RAM
//!    byte at frame `32563` (`0x18e84` upper-right break via `0x435c2`) fails at
//!    that exact frame.
//! 4. Both outcomes (`GAME_MODE != 4` vs `GAME_MODE == 4`, unexercised) of the
//!    three player-selection checks at `0x42004`, `0x42164`, and `0x4221a`.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{compare_fight_ram_with_inputs, run_fight_capture, CpuRamOutcome, ScratchDir};

const WIN_SCENARIO: &str = "../../tools/reference/scenarios/sf_win_08.json";
const WIN_RUN_A: &str = "../../tools/reference/local/captures/win-08-run-a";
const WIN_RUN_B: &str = "../../tools/reference/local/captures/win-08-run-b";

const BREAK_SCENARIO: &str = "../../tools/reference/scenarios/sf_bonus_04.json";
const BREAK_RUN_A: &str = "../../tools/reference/local/captures/bonus-04-run-a";
const BREAK_RUN_B: &str = "../../tools/reference/local/captures/bonus-04-run-b";

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 32315;
const WIN_LAST: u32 = 34184;
const WIN_FRAMES: u32 = WIN_LAST - FIRST;

const BREAK_LAST: u32 = 33017;
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
fn win_08_bonus_stage3_and_stage4_arrival_reference_runs_repeat_exactly() {
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
        "sf-win-08 stage-3 bonus + stage-4 arrival: {outcome:?}"
    );
}

#[test]
fn native_win_08_bonus_stage3_timeout_and_stage4_arrival_matches_every_observable_and_owned_ram() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-08-bonus-stage3");
    assert_eq!(
        run_fight_capture(
            Path::new(WIN_RUN_A),
            &assets,
            &out,
            "sf-win-08",
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
        "sf-win-08 stage-3 bonus + stage-4 arrival: {outcome:?}"
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
fn bonus_04_break_all_reference_runs_repeat_and_match_win_08_through_divergence() {
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
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == BREAK_FRAMES as u64 && checkpoints_compared == BREAK_FRAMES as usize
        ),
        "sf-bonus-04: {outcome:?}"
    );

    // Identical to sf-win-08 on every frame and shared checkpoint through frame 32426
    // (when 0x421d4 starts the 10s loop and P1 presses right in sf-bonus-04).
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(32426),
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
                frames_compared: 32427,
                checkpoints_compared: 112,
            }
        ),
        "sf-bonus-04 vs sf-win-08 prefix: {prefix_outcome:?}"
    );
}

#[test]
fn native_bonus_04_break_all_matches_every_observable_owned_ram_and_reaches_adon() {
    if !break_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-bonus-04-break-all");
    assert_eq!(
        run_fight_capture(
            Path::new(BREAK_RUN_A),
            &assets,
            &out,
            "sf-bonus-04",
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
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == BREAK_FRAMES as u64 && checkpoints_compared == BREAK_FRAMES as usize
        ),
        "sf-bonus-04: {outcome:?}"
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

    // Verify each of the 4 stack breaks on its exact frame and continue past
    // 0x3efc4 (frame 33017) through the flight to Thailand, VS screen, stage 4's
    // intro, and ROUND 1 into Adon's fight loop (0x17f7c) at frame 33842.
    let mut m =
        FightMachine::from_fight_checkpoint(Path::new(BREAK_RUN_A), 32497, rom.clone()).unwrap();
    m.step(); // frame 32498: lower-right stack (0x18ea4 @ 0x1c0, handler 0x4360a) breaks
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x1d0..0x1d4].try_into().unwrap()),
        0x194aa,
        "slot 0x1c0 stream should switch to broken stream 0x194aa"
    );
    assert_eq!(m.state.objectram[0x1c9], 0);
    assert_eq!(m.state.objectram[0x1ce], 0);
    assert_eq!(
        m.state.objectram[0x208], 2,
        "debris 0x18ec4 spawned and stepped in slot 0x200"
    );
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x210..0x214].try_into().unwrap()),
        0x1904a,
        "debris 0x18ec4 stream after first step"
    );
    assert_eq!(&m.work_ram()[0xf18..0xf20], &[0, 0, 0, 0, 1, 0, 0, 0]);
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 4, 0, 6, 7, 0, 0]);

    let mut m =
        FightMachine::from_fight_checkpoint(Path::new(BREAK_RUN_A), 32562, rom.clone()).unwrap();
    m.step(); // frame 32563: upper-right stack (0x18e84 @ 0x180, handler 0x435c2) breaks
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x190..0x194].try_into().unwrap()),
        0x1948e,
        "slot 0x180 stream should switch to broken stream 0x1948e"
    );
    assert_eq!(m.state.objectram[0x189], 0);
    assert_eq!(m.state.objectram[0x18e], 0);
    assert_eq!(
        m.state.objectram[0x208], 2,
        "debris 0x18ec4 spawned and stepped in slot 0x200"
    );
    assert_eq!(&m.work_ram()[0xf18..0xf20], &[0, 0, 0, 0, 4, 0, 0, 0]);
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 4, 0, 6, 7, 0, 0]);

    let mut m =
        FightMachine::from_fight_checkpoint(Path::new(BREAK_RUN_A), 32677, rom.clone()).unwrap();
    m.step(); // frame 32678: lower-left stack (0x18e64 @ 0x140, handler 0x4357a) breaks
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x150..0x154].try_into().unwrap()),
        0x19472,
        "slot 0x140 stream should switch to broken stream 0x19472"
    );
    assert_eq!(m.state.objectram[0x149], 0);
    assert_eq!(m.state.objectram[0x14e], 0);
    assert_eq!(
        m.state.objectram[0x208], 2,
        "debris 0x18ec4 spawned and stepped in slot 0x200"
    );
    assert_eq!(&m.work_ram()[0xf18..0xf20], &[0, 0, 0, 0, 5, 0, 0, 0]);
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 4, 0, 6, 7, 0, 0]);

    let mut m = FightMachine::from_fight_checkpoint(Path::new(BREAK_RUN_A), 32742, rom).unwrap();
    m.step(); // frame 32743: upper-left stack (0x18e44 @ 0x100, handler 0x4352c) breaks
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x110..0x114].try_into().unwrap()),
        0x19456,
        "slot 0x100 stream should switch to broken stream 0x19456"
    );
    assert_eq!(m.state.objectram[0x109], 0);
    assert_eq!(m.state.objectram[0x10e], 0);
    assert_eq!(
        m.state.objectram[0x208], 2,
        "debris 0x18ec4 spawned and stepped in slot 0x200"
    );
    assert_eq!(&m.work_ram()[0xf18..0xf20], &[0, 0, 0, 0, 8, 0, 0, 0]);
    // 0x4352c also adds 3,000 directly onto P1's score (0x43566 jsr $3ba0e.l).
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 4, 0, 9, 7, 0, 0]);

    m.step(); // frame 32744: 0x3baf6 early exit (0x421ee = 8,000) -> (1, 0x4221a) settle loop
    assert!(m.task_positions().contains(&(1, 0x4221a)));

    while m.state.frame < 32777 {
        m.step();
    }
    // Frame 32777: 10-frame post-settle drain finishes and enters 0x4226c
    // (120-frame BONUS PTS. display hold).
    assert!(m.task_positions().contains(&(1, 0x4226c)));
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0, 0, 4, 0, 9, 7, 0, 0]);
    assert_eq!(&m.work_ram()[0x4012..0x401a], &[0, 0, 4, 0, 6, 7, 0, 0]);

    while m.state.frame < 33842 {
        m.step();
    }
    // Frame 33842: reaches Adon's fight loop (0x3eafa, move table 0x17f7c).
    assert_eq!(m.work_ram()[0x4003], 4);
    assert!(m.task_positions().contains(&(1, 0x3eafa)));
    assert_eq!(
        u32::from_be_bytes(m.work_ram()[0x4676..0x467a].try_into().unwrap()),
        0x17f7c
    );
    assert!(m.unrecovered().is_empty());
}

#[test]
fn tampered_bonus_stage3_checkpoint_and_ram_are_caught_at_exact_frame() {
    if !break_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const FRAME: u32 = 32563;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-bonus-stage3-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(BREAK_RUN_A),
            &assets,
            &out,
            "sf-bonus-04",
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
        panic!("tampered stage-3 bonus checkpoint unexpectedly matched: {outcome:?}");
    };
    assert_eq!(mismatch.frame, FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Also tamper an owned work-RAM byte (0xff8f1c bonus thousands digit) at frame 32563.
    for entry in [
        "ram_032562_work.bin",
        "ram_032562_video.bin",
        "ram_032562_palette.bin",
        "ram_032563_work.bin",
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
        panic!("tampered stage-3 bonus RAM unexpectedly matched: {ram_outcome:?}");
    };
    assert!(
        msg.contains("frame 32563: bonus stage state differs first at 0xff8f1c"),
        "{msg}"
    );
}

#[test]
fn mode_4_branches_select_p2_at_0x42004_0x42164_and_0x4221a() {
    if !win_present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();

    // 1. 0x42004..0x42024 (bonus_stage1_start Copy 2): GAME_MODE == 4 calls
    //    0x44192 (place_bonus_player(P2_STRUCT)) and passes P2's entity
    //    (ew(0xff86da, 0x10)) to 0x42f1e for 0x18e44, 0x18e64, 0x18e84, 0x18ea4
    //    so all four stacks' +0x24 target is 0xff86da.
    let mut m_p1 =
        FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), FIRST, rom.clone()).unwrap();
    m_p1.step();
    for slot in [0x100usize, 0x140, 0x180, 0x1c0] {
        assert_eq!(
            u32::from_be_bytes(
                m_p1.state.objectram[slot + 0x24..slot + 0x28]
                    .try_into()
                    .unwrap()
            ),
            0xff86b0,
            "GAME_MODE == 2 should bind stack slot {slot:#x} target +0x24 to P1_STRUCT (0xff86b0)"
        );
    }
    assert_eq!(m_p1.state.fg_scroll, 0x1800);

    let mut m_p2 = FightMachine::from_fight_checkpoint(Path::new(WIN_RUN_A), FIRST, rom).unwrap();
    m_p2.set_work_byte(0xffc002, 4); // GAME_MODE = 4 (1P on P2 side vs CPU, unexercised)
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
    m_p2.step(); // frame 32316: runs 0x41fe6..0x420ca with GAME_MODE == 4
    for slot in [0x100usize, 0x140, 0x180, 0x1c0] {
        assert_eq!(
            u32::from_be_bytes(
                m_p2.state.objectram[slot + 0x24..slot + 0x28]
                    .try_into()
                    .unwrap()
            ),
            0xff86da,
            "GAME_MODE == 4 should bind stack slot {slot:#x} target +0x24 to P2_STRUCT (0xff86da)"
        );
    }
    assert_eq!(m_p2.state.objectram[0xa37], 2);
    assert_eq!(m_p2.state.objectram[0xa0a], 1);

    // 2. 0x42164..0x42180 (bonus_stage1_frame Copy 2): advance to 32426 (0x421d4)
    //    and verify P2 right input moves P2's entity 0xa00 in GAME_MODE == 4.
    while m_p2.state.frame < 32426 {
        m_p2.step();
    }
    let x_before = u16::from_be_bytes(m_p2.state.objectram[0xa06..0xa08].try_into().unwrap());
    m_p2.set_p2_direction(true, false, false, false);
    m_p2.step();
    m_p2.step();
    let x_after = u16::from_be_bytes(m_p2.state.objectram[0xa06..0xa08].try_into().unwrap());
    assert_eq!(m_p2.state.objectram[0xa0a], 2);
    assert!(x_after > x_before);

    // 3. 0x4221a..0x4226a (bonus_stage1_settle_step Copy 2): trigger 0x3baf6 early
    //    exit by setting 0xff8f1c = 8 (0x421ee = 8,000) in GAME_MODE == 4, verify
    //    0x4221a checks P2's entity (0xffea09 bit 7: stays at 0x4221a while busy,
    //    advances to 0x42258 and 0x4226c once clear).
    m_p2.set_p2_direction(false, false, false, false);
    m_p2.state.objectram[0xa09] = 0x81;
    m_p2.set_work_byte(0xff8f1c, 8);
    m_p2.step(); // exits 0x421d4 via 0x3baf6 -> enters 0x4221a settle
    assert!(m_p2.task_positions().contains(&(1, 0x4221a)));
    m_p2.step(); // 0x4221a sees P2 entity 0xa00 busy (+9 bit 7) -> stays at 0x4221a
    assert!(m_p2.task_positions().contains(&(1, 0x4221a)));
    m_p2.state.objectram[0xa09] = 0x01;
    m_p2.step(); // 0x4221a -> 0x42258 (left = 10)
    assert!(m_p2.task_positions().contains(&(1, 0x42258)));
    for _ in 0..10 {
        m_p2.step();
    }
    assert!(m_p2.task_positions().contains(&(1, 0x4226c)));
}
