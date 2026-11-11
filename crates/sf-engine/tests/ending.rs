//! Ticket #136 (`F64`): the ending (`0x44a40..0x44c1c`, resume `0x44a60`),
//! including the five country-ordered ending scenes (`0x44a60..0x44aff` via
//! `0x44c1c`, palettes `0x44c7c`, scrolls `0x44c90`, and 2x2-tile country cards
//! `0x44ca4`), the congratulatory message (`0x44b00..0x44b86`, sound `0xff`,
//! palette `0x1b868`, `0x44b48` mode-2 bg scroll `0x44d5c = 0x7700` vs
//! unexercised `0x44d5e = 0x7880`, and `0x44d60` typed with `D4 = 5` and
//! `TEXT_SOUND = 0x000d` through `0x3bb98`), the staff roll (`0x44b88..0x44c02`,
//! sound `0x36`, 2x2 header `0x44f80`, 8 staff blocks `0x44f90..0x45208` typed
//! with `D4 = 2`, and 2x2 `THE END` `0x452cc`), and the closing `0x3b7a8` +
//! `0x3efc4` (`STAGE_NO`: `4 -> 5`) jump into `0x3f2d0` (`versus_game_over` ->
//! `0x42422` `ranking_check` -> `0x42486` `name_entry_start`).

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, CpuRamOutcome, ScratchDir,
    INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_10.json";
const RUN_A: &str = "../../tools/reference/local/captures/win-10-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/win-10-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const SAGAT_FIRST: u32 = 37153;
const FIRST: u32 = 39776;
const LAST: u32 = 43618;
const FRAMES: u32 = LAST - FIRST;

fn present() -> bool {
    [RUN_A, RUN_B, ROMS].iter().all(|p| Path::new(p).is_dir())
}

fn ref_scope() -> CompareScope {
    CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(LAST as u64),
        fields: None,
    }
}

fn native_scope() -> CompareScope {
    CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

#[test]
fn reference_runs_repeat_over_ending_and_stage_5_transition() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).unwrap();
    let cap_a = load_capture(Path::new(RUN_A)).unwrap();
    let cap_b = load_capture(Path::new(RUN_B)).unwrap();
    let outcome = compare(
        &scenario,
        &cap_a,
        &cap_b,
        CandidateKind::Reference,
        &ref_scope(),
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == FRAMES as u64 && checkpoints_compared == FRAMES as usize
        ),
        "sf-win-10 ending repeat: {outcome:?}"
    );
}

#[test]
fn native_ending_from_fixture_and_from_sagat_matches_every_observable_and_owned_ram() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-10-ending");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-10",
            Some(Path::new(SCENARIO)),
            FIRST,
            LAST,
        )
        .unwrap(),
        (FRAMES + 1) as u64,
    );
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &native_scope(),
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == FRAMES as u64 && checkpoints_compared == FRAMES as usize
        ),
        "sf-win-10 ending native observables: {outcome:?}"
    );

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        rom.clone(),
        Some(Path::new(SCENARIO)),
        FIRST,
        LAST,
    )
    .unwrap();
    assert_eq!(
        ram_outcome,
        CpuRamOutcome::Match { frames: FRAMES },
        "sf-win-10 owned RAM over ending {FIRST}..={LAST}"
    );

    // Also verify continuous native RAM comparison from Sagat's round-1 fixture
    // (37153) across the entire Sagat match, 0x3ef96's 0x44a40 entry (39776),
    // the ending (39776..=43617), and 0x3efc4 -> 0x3f2d0 -> 0x42422 (43617..=43618).
    let full_ram = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        rom,
        Some(Path::new(SCENARIO)),
        SAGAT_FIRST,
        LAST,
    )
    .unwrap();
    assert_eq!(
        full_ram,
        CpuRamOutcome::Match {
            frames: LAST - SAGAT_FIRST
        },
        "sf-win-10 owned RAM over {SAGAT_FIRST}..={LAST}"
    );
}

#[test]
fn tampered_ending_checkpoint_and_missing_0x44a40_or_bad_0x44d5c_scroll_are_caught() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const MSG_DONE_FRAME: u32 = 41823;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-10-ending-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-10",
            Some(Path::new(SCENARIO)),
            FIRST,
            MSG_DONE_FRAME,
        )
        .unwrap(),
        (MSG_DONE_FRAME - FIRST + 1) as u64,
    );
    let checkpoints = out.join("checkpoints.tsv");
    let text = std::fs::read_to_string(&checkpoints).unwrap();
    let suffix = format!("\tpixels_{MSG_DONE_FRAME:06}.rgba");
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{MSG_DONE_FRAME}\t")) && l.ends_with(&suffix))
        .expect("checkpoint row for MSG_DONE_FRAME")
        .to_string();
    let cols: Vec<&str> = line.split('\t').collect();
    let orig_crc = cols[1];
    let bad_crc = if let Some(prefix) = orig_crc.strip_suffix('0') {
        format!("{prefix}1")
    } else {
        format!("{}0", &orig_crc[..orig_crc.len() - 1])
    };
    let altered = text.replacen(
        &line,
        &format!("{MSG_DONE_FRAME}\t{bad_crc}\tpixels_{MSG_DONE_FRAME:06}.rgba"),
        1,
    );
    std::fs::write(&checkpoints, altered).unwrap();

    let negative_scope = CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(MSG_DONE_FRAME as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &negative_scope,
    )
    .unwrap();
    let Outcome::Mismatch(mismatch) = outcome else {
        panic!("tampered ending checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, MSG_DONE_FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Negative control 2: patching stage 4's entry in 0x3efb0 (0x3efc0, 0x44a40)
    // to untranslated 0x492d6 stops native execution from 39775 at frame 39776.
    let mut bad_stage_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    assert_eq!(
        u32::from_be_bytes(bad_stage_rom[0x3efc0..0x3efc4].try_into().unwrap()),
        0x0004_4a40
    );
    bad_stage_rom[0x3efc0..0x3efc4].copy_from_slice(&0x0004_92d6u32.to_be_bytes());
    let stop_outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        bad_stage_rom,
        Some(Path::new(SCENARIO)),
        39775,
        39778,
    )
    .unwrap();
    assert_eq!(
        stop_outcome,
        CpuRamOutcome::Stopped {
            frames: 0,
            frame: 39776,
            unrecovered: vec![0x492d6],
        }
    );

    // Negative control 3: tampering the congratulatory message bg-scroll word at
    // 0x44d5c (0x7700 -> 0x7880) fails owned-RAM comparison at frame 40978.
    let mut bad_scroll_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    assert_eq!(
        u16::from_be_bytes(bad_scroll_rom[0x44d5c..0x44d5e].try_into().unwrap()),
        0x7700
    );
    bad_scroll_rom[0x44d5c..0x44d5e].copy_from_slice(&0x7880u16.to_be_bytes());
    let scroll_mismatch = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        bad_scroll_rom,
        Some(Path::new(SCENARIO)),
        40976,
        40980,
    )
    .unwrap();
    let CpuRamOutcome::Mismatch(msg) = scroll_mismatch else {
        panic!("expected RAM mismatch at 40978, got {scroll_mismatch:?}");
    };
    assert!(
        msg.contains("frame 40978") && msg.contains("0xffc682"),
        "unexpected mismatch message: {msg}"
    );
}

#[test]
fn direct_replay_verifies_scenes_palettes_scrolls_sounds_and_unexercised_mode4_branch() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut machine =
        FightMachine::from_fight_checkpoint(Path::new(RUN_A), 39775, rom.clone()).unwrap();
    let mut scene_steps = Vec::new();
    let mut latches = Vec::new();

    // Step 39775 -> 39776: vblank pops 0x3f (queued by wipe end at 39775), then
    // 0x3ef72 queues 0x3f and 0x3ef96 -> 0x44a40 queues 0x32 (popped at vblanks
    // 39777 and 39778).
    apply_inputs(&mut machine, &[false; INPUTS]);
    machine.step();
    assert_eq!(machine.latch_writes()[0].value, 0x3f);
    assert!(machine.task_positions().contains(&(1, 0x44a60)));

    while machine.state.frame < LAST {
        let frame = machine.state.frame + 1;
        apply_inputs(&mut machine, &[false; INPUTS]);
        machine.step();
        let latch = machine.latch_writes()[0].value;
        assert!(
            machine.unrecovered().is_empty(),
            "frame {}: {:05x?}",
            machine.state.frame,
            machine.unrecovered()
        );
        if latch != 0xff {
            latches.push((frame, latch));
        }
        let w = machine.work_ram();
        if matches!(
            frame,
            39777 | 39897 | 40017 | 40137 | 40257 | 40377 | 40497 | 40617 | 40737 | 40857
        ) {
            let pal = u32::from_be_bytes(w[0x634..0x638].try_into().unwrap());
            let bg = u16::from_be_bytes([w[0x4682], w[0x4683]]);
            scene_steps.push((frame, pal, bg));
        }
        if frame == 40978 {
            let pal = u32::from_be_bytes(w[0x634..0x638].try_into().unwrap());
            let bg = u16::from_be_bytes([w[0x4682], w[0x4683]]);
            let text_snd = u16::from_be_bytes([w[0x68c], w[0x68d]]);
            assert_eq!((pal, bg, text_snd), (0x1b868, 0x7700, 0x000d));
        }
        if frame == 41823 {
            let text_snd = u16::from_be_bytes([w[0x68c], w[0x68d]]);
            assert_eq!(text_snd, 0);
        }
    }

    assert_eq!(
        scene_steps,
        vec![
            (39777, 0x1b068, 0x1b80),
            (39897, 0x1b068, 0x2500),
            (40017, 0x1b268, 0x2e80),
            (40137, 0x1b268, 0x3800),
            (40257, 0x1b468, 0x4180),
            (40377, 0x1b468, 0x4b00),
            (40497, 0x1b668, 0x5480),
            (40617, 0x1b668, 0x5e00),
            (40737, 0x1b868, 0x6780),
            (40857, 0x1b868, 0x7100),
        ]
    );
    assert_eq!(&latches[..2], &[(39777, 0x3f), (39778, 0x32)]);
    assert!(latches.contains(&(40979, 0x0d)));
    assert!(latches.contains(&(41819, 0x0d)));
    assert!(latches.contains(&(41945, 0x36)));
    assert!(latches.contains(&(43618, 0x27)));

    // Continue stepping past 43618 through 0x3f2d0's name-entry timeout, ranking
    // screen, game-over screen, and return to attract (0x3ccec).
    while !machine.task_positions().contains(&(1, 0x3ccec)) {
        apply_inputs(&mut machine, &[false; INPUTS]);
        machine.step();
        assert!(
            machine.unrecovered().is_empty(),
            "frame {}: {:05x?}",
            machine.state.frame,
            machine.unrecovered()
        );
        assert!(
            machine.state.frame <= 46500,
            "did not reach 0x3ccec by 46500"
        );
    }

    // Verify the unexercised GAME_MODE != 2 branch at 0x44b48..0x44b52 (reads
    // 0x44d5e = 0x7880 instead of 0x44d5c = 0x7700).
    let mut mode4_machine =
        FightMachine::from_fight_checkpoint(Path::new(RUN_A), 40977, rom).unwrap();
    mode4_machine.set_work_byte(0xffc002, 4);
    apply_inputs(&mut mode4_machine, &[false; INPUTS]);
    mode4_machine.step();
    assert!(mode4_machine.unrecovered().is_empty());
    let w = mode4_machine.work_ram();
    let bg = u16::from_be_bytes([w[0x4682], w[0x4683]]);
    assert_eq!(bg, 0x7880);
}

#[test]
fn reference_has_the_expected_ending_landmarks() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let read = |frame: u32| {
        std::fs::read(Path::new(RUN_A).join(format!("ram_{frame:06}_work.bin"))).unwrap()
    };
    let slot1_resume = |w: &[u8]| {
        assert_eq!(w[0x16], 8);
        let usp = u32::from_be_bytes(w[0x1e..0x22].try_into().unwrap()) as usize;
        let off = usp - 0xff8000;
        let ret = u32::from_be_bytes(w[off + 60..off + 64].try_into().unwrap());
        assert_eq!(ret, 0x3b5a8, "unexpected sleep return 0x{ret:05x}");
        u32::from_be_bytes(w[off + 64..off + 68].try_into().unwrap())
    };
    assert_eq!(slot1_resume(&read(39776)), 0x44a60);
    assert_eq!(slot1_resume(&read(39777)), 0x44ae8);
    assert_eq!(slot1_resume(&read(40977)), 0x44b20);
    assert_eq!(slot1_resume(&read(40978)), 0x3bc10);
    assert_eq!(slot1_resume(&read(41823)), 0x44b78);
    assert_eq!(slot1_resume(&read(41943)), 0x44b88);
    assert_eq!(slot1_resume(&read(41944)), 0x44ba8);
    assert_eq!(slot1_resume(&read(41945)), 0x44bc8);
    assert_eq!(slot1_resume(&read(41947)), 0x3bc10);
    assert_eq!(slot1_resume(&read(41989)), 0x44bda);
    assert_eq!(slot1_resume(&read(43377)), 0x44bda);
    assert_eq!(slot1_resume(&read(43497)), 0x44c04);
    assert_eq!(
        (
            read(43616)[0x4003],
            read(43617)[0x4003],
            slot1_resume(&read(43617))
        ),
        (4, 5, 0x42552)
    );
    assert_eq!(slot1_resume(&read(43618)), 0x42584);
}
