//! Tickets #82 (F42), #122 (F50), and #128 (F56): name entry (`0x42486–0x425c2`,
//! `0x42f72`, `0x42dd2`, `0x427e2`, `0x42920`), ranking insertion and marked-entry
//! blink (`0x425c4–0x4268a`, `0x42750–0x427e0`, `0x3b904`), and end-to-end execution
//! from checkpoint `2389` through Retsu win, Geki `FIGHT!`, challenger join, versus
//! loss, name entry, ranking, and Ken's Stage 1 carry-on (`sf-name-entry-01` and `-02`).

use std::collections::BTreeMap;
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, observe_cpu_fight, run_fight_capture,
    scenario_input_events, CpuRamOutcome, ScratchDir, INPUTS,
};

const SCENARIO_01: &str = "../../tools/reference/scenarios/sf_name_entry_01.json";
const RUN_01_A: &str = "../../tools/reference/local/captures/name-entry-01-run-a";
const RUN_01_B: &str = "../../tools/reference/local/captures/name-entry-01-run-b";
const SCENARIO_02: &str = "../../tools/reference/scenarios/sf_name_entry_02.json";
const RUN_02_A: &str = "../../tools/reference/local/captures/name-entry-02-run-a";
const RUN_02_B: &str = "../../tools/reference/local/captures/name-entry-02-run-b";
const CPU_RUN_A: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present_01() -> bool {
    [RUN_01_A, RUN_01_B, CPU_RUN_A, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn present_02() -> bool {
    [RUN_02_A, RUN_02_B, RUN_01_A, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

#[test]
fn sf_name_entry_01_reference_runs_repeat_and_prefix_matches_cpu_01_except_second_credit() {
    if !present_01() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO_01).unwrap();
    let mut expected_checkpoints = vec![2389];
    expected_checkpoints.extend(8343..=10367);
    assert_eq!(scenario.checkpoints, expected_checkpoints);

    let cap_a = load_capture(Path::new(RUN_01_A)).unwrap();
    let cap_b = load_capture(Path::new(RUN_01_B)).unwrap();
    let scope = CompareScope {
        from: Some(0),
        to: Some(10367),
        fields: None,
    };
    let outcome = compare(&scenario, &cap_a, &cap_b, CandidateKind::Reference, &scope).unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 10368,
                checkpoints_compared: 2026
            }
        ),
        "{outcome:?}"
    );

    // Compare 0..=2389 against sf-cpu-01 (which lacks coin2 at 1480).
    let cpu_a = load_capture(Path::new(CPU_RUN_A)).unwrap();
    let mut frame_diffs = Vec::new();
    for f in 0..=2389 {
        let fa = &cap_a.frames[f];
        let fc = &cpu_a.frames[f];
        if (&fa.screen_hash, &fa.latch, fa.latch_changed)
            != (&fc.screen_hash, &fc.latch, fc.latch_changed)
        {
            frame_diffs.push(f as u32);
        }
    }
    // 1481..=1900: coin sound 0x39 in sound latch from coin2 at 1480.
    assert!(!frame_diffs.is_empty());
    assert!(frame_diffs.iter().all(|&f| (1480..=1900).contains(&f)));
    assert_eq!(
        std::fs::read(Path::new(RUN_01_A).join("ram_002389_video.bin")).unwrap(),
        std::fs::read(Path::new(CPU_RUN_A).join("ram_002389_video.bin")).unwrap(),
    );
    assert_eq!(
        std::fs::read(Path::new(RUN_01_A).join("ram_002389_palette.bin")).unwrap(),
        std::fs::read(Path::new(CPU_RUN_A).join("ram_002389_palette.bin")).unwrap(),
    );
    let work_ne = std::fs::read(Path::new(RUN_01_A).join("ram_002389_work.bin")).unwrap();
    let work_cpu = std::fs::read(Path::new(CPU_RUN_A).join("ram_002389_work.bin")).unwrap();
    let work_diffs: Vec<(u32, u8, u8)> = work_ne
        .iter()
        .zip(work_cpu.iter())
        .enumerate()
        .filter_map(|(i, (&a, &b))| (a != b).then_some((0xff8000 + i as u32, a, b)))
        .collect();
    assert_eq!(
        work_diffs,
        vec![
            (0xff8001, 0x07, 0x03),
            (0xff85e1, 0x01, 0x06),
            (0xff85f9, 0x72, 0x71),
            (0xff8605, 0x8f, 0x5a),
            (0xff8620, 0x81, 0x82),
            (0xff8621, 0x9e, 0x70),
            (0xff865d, 0x01, 0x00),
            (0xff865f, 0x04, 0x02),
            (0xff8661, 0x04, 0x02),
        ]
    );
}

#[test]
fn sf_name_entry_02_reference_runs_repeat_and_share_prefix_with_01() {
    if !present_02() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO_02).unwrap();
    let mut expected_checkpoints = vec![2389];
    expected_checkpoints.extend(8343..=12183);
    assert_eq!(scenario.checkpoints, expected_checkpoints);

    let mut cap_a = load_capture(Path::new(RUN_02_A)).unwrap();
    let cap_b = load_capture(Path::new(RUN_02_B)).unwrap();
    let scope = CompareScope {
        from: Some(0),
        to: Some(12183),
        fields: None,
    };
    let outcome = compare(&scenario, &cap_a, &cap_b, CandidateKind::Reference, &scope).unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 12184,
                checkpoints_compared: 3842
            }
        ),
        "{outcome:?}"
    );

    // Through frame 9799 (before P1 presses LP at 9800 in -01), -01 and -02 are identical.
    let cap_01 = load_capture(Path::new(RUN_01_A)).unwrap();
    cap_a.scenario_id = cap_01.scenario_id.clone();
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(9799),
        fields: None,
    };
    let prefix_outcome = compare(
        &load_scenario(SCENARIO_01).unwrap(),
        &cap_01,
        &cap_a,
        CandidateKind::Reference,
        &prefix_scope,
    )
    .unwrap();
    assert!(
        matches!(
            prefix_outcome,
            Outcome::Match {
                frames_compared: 9800,
                checkpoints_compared: 1458
            }
        ),
        "{prefix_outcome:?}"
    );
}

#[test]
fn shared_fight_machine_matches_name_entry_letters_back_and_end_to_f50_boundary() {
    if !present_01() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();

    // Check qualifying score at the versus Round 2 fixture (8343):
    // each ranking entry at 0xff8e6e + 16 * rank stores its 7-byte score at +5..+12
    // (compared against player's +0x22..+0x29 by Bus::compare_bytes_7).
    let work_8343 = std::fs::read(Path::new(RUN_01_A).join("ram_008343_work.bin")).unwrap();
    let p1_score = &work_8343[0x6d2..0x6d9];
    assert_eq!(p1_score, &[0, 0, 0, 5, 3, 2, 0]);
    let rank_8_score = &work_8343[0xef3..0xefa];
    let rank_9_score = &work_8343[0xf03..0xf0a];
    assert_eq!(rank_8_score, &[0, 0, 0, 5, 9, 8, 0]);
    assert_eq!(rank_9_score, &[0, 0, 0, 5, 0, 0, 0]);
    assert_eq!(p1_score.cmp(rank_8_score), std::cmp::Ordering::Less);
    assert_eq!(p1_score.cmp(rank_9_score), std::cmp::Ordering::Greater);

    let outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_01_A),
        rom.clone(),
        Some(Path::new(SCENARIO_01)),
        8343,
        10367,
    )
    .unwrap();
    assert_eq!(outcome, CpuRamOutcome::Match { frames: 2024 });

    let landmarks = [
        8343, 8855, 9745, 9746, 9813, 9833, 9853, 9873, 9889, 9913, 9933, 9953, 9973, 9981, 10005,
        10006, 10212, 10213, 10215, 10366, 10367,
    ];
    let mut snapshots = BTreeMap::new();
    let mut obj_snapshots = BTreeMap::new();
    let mut stream_calls = Vec::new();
    let mut latches = Vec::new();
    let (last, unrecovered) = observe_cpu_fight(
        Path::new(RUN_01_A),
        rom.clone(),
        Some(Path::new(SCENARIO_01)),
        8343,
        10367,
        |m| {
            if m.state.frame >= 10005 && m.state.sound_latch != 0xff {
                latches.push((m.state.frame, m.state.sound_latch));
            }
            if landmarks.contains(&m.state.frame) {
                snapshots.insert(m.state.frame, m.work_ram().to_vec());
                obj_snapshots.insert(m.state.frame, m.state.objectram.clone());
            }
            if m.state.frame == 10005 {
                stream_calls = m.effect_stream_calls();
            }
        },
    )
    .unwrap();
    assert_eq!((last, unrecovered), (10367, vec![]));
    assert_eq!(stream_calls, vec![0x427e2, 0x42920]);

    let w = |f| &snapshots[&f][..];
    let obj = |f| &obj_snapshots[&f][..];

    // 8855: P1 Ryu is knocked out in versus Round 2.
    assert_eq!((w(8855)[0x4008], w(8855)[0x6b6]), (0, 0));

    // 9745: 0x3eeac..0x3eeba sets GAME_MODE = 4, SCENE_MODE = 2, qualifies at rank 9 (0xff8efe),
    // sets up name entry (0x42486..0x42550), arms timer 2 (0xff8e16 = 4), requests palette 0x1ac68,
    // and sleeps 1 frame at 0x42552.
    assert_eq!((w(9745)[0x4002], w(9745)[0x4008]), (4, 2));
    assert_eq!(&w(9745)[0xe5e..0xe62], &[0, 0xff, 0x86, 0xb0]);
    assert_eq!(&w(9745)[0xe62..0xe66], &[0, 0xff, 0x86, 0x8f]);
    assert_eq!(&w(9745)[0xe66..0xe6a], &[0, 0xff, 0x8e, 0xfe]);
    assert_eq!(&w(9745)[0xe6a..0xe6e], &[0x31, 0x31, 0x31, 0x00]);
    assert_eq!(w(9745)[0xe16], 4);
    assert_eq!(
        (&w(9745)[0x634..0x638], &w(9745)[0x638..0x63a]),
        (&[0, 1, 0xac, 0x68][..], &[2, 0][..])
    );

    // 9746: timer 2 is initialized to [3, 0, 0, 0, 2, 0] (30 seconds), 0x42552 spawns cursor
    // 0x1845c via 0x42f72 into slot 0xffe100, queues tune 0x38, runs effects once, and enters 0x42584.
    assert_eq!(&w(9746)[0xe24..0xe2a], &[3, 0, 0, 0, 2, 0]);
    assert_eq!(obj(9746)[0x108], 0x02);
    assert_eq!(
        (&obj(9746)[0x104..0x106], &obj(9746)[0x106..0x108]),
        (&[0x00, 0x5c][..], &[0x00, 0xb4][..]),
        "cursor starts at (Y=0x5c, X=0xb4), row 0 col 0 ('A')"
    );
    assert_eq!(&w(9746)[0xe52..0xe54], &[0, 0], "(row, col) = (0, 0)");
    assert_eq!(&w(9746)[0xe5a..0xe5e], &[0, 0xff, 0x8e, 0x6a]);

    // 9813: 'A' (0x0a) committed -> "A..", write ptr = 0xff8e6b.
    assert_eq!(&w(9813)[0xe6a..0xe6e], &[0x0a, 0x31, 0x31, 0x00]);
    assert_eq!(&w(9813)[0xe5a..0xe5e], &[0, 0xff, 0x8e, 0x6b]);

    // 9833: p1_right moves to (0, 1) ('B', X=0xc4); 9853: 'B' (0x0b) committed -> "AB.", write ptr = 0xff8e6c.
    assert_eq!(
        (
            &obj(9833)[0x104..0x106],
            &obj(9833)[0x106..0x108],
            &w(9833)[0xe52..0xe54]
        ),
        (&[0x00, 0x5c][..], &[0x00, 0xc4][..], &[0, 1][..])
    );
    assert_eq!(&w(9853)[0xe6a..0xe6e], &[0x0a, 0x0b, 0x31, 0x00]);
    assert_eq!(&w(9853)[0xe5a..0xe5e], &[0, 0xff, 0x8e, 0x6c]);

    // 9873: p1_up wraps row 0 -> 2 (Y=0x7c, col 1); 9889: p1_left wraps col 1 -> 0 -> 9 -> 8 (X=0x134, BACK).
    assert_eq!(
        (&obj(9873)[0x104..0x106], &w(9873)[0xe52..0xe54]),
        (&[0x00, 0x7c][..], &[2, 1][..])
    );
    assert_eq!(
        (
            &obj(9889)[0x104..0x106],
            &obj(9889)[0x106..0x108],
            &w(9889)[0xe52..0xe54]
        ),
        (&[0x00, 0x7c][..], &[0x01, 0x34][..], &[2, 8][..])
    );

    // 9913: BACK (0x32) erases 'B' -> "A..", write ptr = 0xff8e6b.
    assert_eq!(&w(9913)[0xe6a..0xe6e], &[0x0a, 0x31, 0x31, 0x00]);
    assert_eq!(&w(9913)[0xe5a..0xe5e], &[0, 0xff, 0x8e, 0x6b]);

    // 9933: p1_up moves to (1, 8) ('S', Y=0x6c, X=0x134); 9953: 'S' (0x1c) committed -> "AS.", write ptr = 0xff8e6c.
    assert_eq!(
        (
            &obj(9933)[0x104..0x106],
            &obj(9933)[0x106..0x108],
            &w(9933)[0xe52..0xe54]
        ),
        (&[0x00, 0x6c][..], &[0x01, 0x34][..], &[1, 8][..])
    );
    assert_eq!(&w(9953)[0xe6a..0xe6e], &[0x0a, 0x1c, 0x31, 0x00]);
    assert_eq!(&w(9953)[0xe5a..0xe5e], &[0, 0xff, 0x8e, 0x6c]);

    // 9973: p1_down -> (2, 8); 9981: p1_right -> (2, 9) (END, Y=0x7c, X=0x144).
    assert_eq!(&w(9973)[0xe52..0xe54], &[2, 8]);
    assert_eq!(
        (
            &obj(9981)[0x104..0x106],
            &obj(9981)[0x106..0x108],
            &w(9981)[0xe52..0xe54]
        ),
        (&[0x00, 0x7c][..], &[0x01, 0x44][..], &[2, 9][..])
    );

    // 10005: END (0x33) advances stream to 0x18f14 -> 0xfd deactivates cursor slot 0xffe100.
    assert_eq!(obj(10005)[0x108], 0);
    assert_eq!(&w(10005)[0xe6a..0xe6e], &[0x0a, 0x1c, 0x31, 0x00]);

    // 10006..=10367 (#122, F50): 0x425c4..0x4268a inserts 'AS.' and 53,200 into rank 9,
    // clears P1's score, runs the marked-entry ranking blink loop (0x42750..0x427e0),
    // clears the marker and SCENE_MODE at 10213, and returns via 0x3eec2 into Ken's mode-4 intro.
    let ref_10006 = std::fs::read(Path::new(RUN_01_A).join("ram_010006_work.bin")).unwrap();
    assert_eq!(
        &w(10006)[0x0e6e..0x0f0e],
        &ref_10006[0x0e6e..0x0f0e],
        "10x16-byte ranking table (0xff8e6e..0xff8f0d) equals reference after insertion"
    );
    assert_eq!(
        &w(10006)[0xf02..0xf0a],
        &[0xff, 0, 0, 0, 5, 3, 2, 0],
        "rank 9 highlight + score updated"
    );
    assert_eq!(
        &w(10006)[0xf0a..0xf0e],
        &[0x00, 0x0a, 0x1c, 0x31],
        "rank 9 initials 'AS.' inserted"
    );
    assert_eq!(
        &w(10006)[0x6d2..0x6da],
        &[0; 8],
        "P1 score cleared after insertion"
    );
    assert_eq!(
        w(10213)[0xf02],
        0,
        "rank 9 marker cleared at 0x42686 on exit at 10213"
    );
    assert_eq!(
        w(10213)[0x4008],
        0,
        "SCENE_MODE cleared at 0x42682 / 0x3eedc"
    );
    assert_eq!(
        w(10215)[0xe36],
        4,
        "Ken's mode-4 Stage 1 intro (0x3fad6) requested at 10215"
    );
    assert_eq!(
        (w(10366)[0xe36], w(10367)[0xe36]),
        (4, 5),
        "intro finishes at 10366 and ROUND 1 starts at 10367"
    );
    assert!(
        latches.contains(&(10007, 0x3f))
            && latches.contains(&(10008, 0x39))
            && latches.contains(&(10214, 0x3f)),
        "0x3f at 10007, ranking tune 0x39 at 10008, and ranking exit 0x3f at 10214: {latches:?}"
    );

    // Full native capture comparison over 8344..=10367 (F42 + F50 window).
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-name-entry-01-native");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_01_A),
            &assets,
            &out,
            "sf-name-entry-01",
            Some(Path::new(SCENARIO_01)),
            8343,
            10367,
        )
        .unwrap(),
        2025,
    );
    let scope = CompareScope {
        from: Some(8344),
        to: Some(10367),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let cmp_outcome = compare(
        &load_scenario(SCENARIO_01).unwrap(),
        &load_capture(Path::new(RUN_01_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            cmp_outcome,
            Outcome::Match {
                frames_compared: 2024,
                checkpoints_compared: 2024
            }
        ),
        "{cmp_outcome:?}"
    );
}

#[test]
fn shared_fight_machine_matches_name_entry_timeout_to_f50_boundary() {
    if !present_02() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_02_A),
        rom.clone(),
        Some(Path::new(SCENARIO_02)),
        8343,
        12183,
    )
    .unwrap();
    assert_eq!(outcome, CpuRamOutcome::Match { frames: 3840 });

    let mut stream_calls = Vec::new();
    let mut work_11787 = Vec::new();
    let mut work_11788 = Vec::new();
    let mut work_11995 = Vec::new();
    let mut work_12148 = Vec::new();
    let mut work_12149 = Vec::new();
    let (last, unrecovered) = observe_cpu_fight(
        Path::new(RUN_02_A),
        rom.clone(),
        Some(Path::new(SCENARIO_02)),
        8343,
        12183,
        |m| match m.state.frame {
            11787 => {
                stream_calls = m.effect_stream_calls();
                work_11787 = m.work_ram().to_vec();
            }
            11788 => work_11788 = m.work_ram().to_vec(),
            11995 => work_11995 = m.work_ram().to_vec(),
            12148 => work_12148 = m.work_ram().to_vec(),
            12149 => work_12149 = m.work_ram().to_vec(),
            _ => {}
        },
    )
    .unwrap();
    assert_eq!((last, unrecovered), (12183, vec![]));
    assert_eq!(stream_calls, vec![0x427e2]);
    assert_eq!(&work_11787[0xe6a..0xe6e], &[0x31, 0x31, 0x31, 0x00]);
    assert_eq!(
        &work_11787[0xe24..0xe2a],
        &[0x00, 0x00, 0x00, 0x00, 0x01, 0x00]
    );

    // 11788..=12183 (#122, F50): timer 2 expires at 11788 (0xff8e29 = 1), 0x425c4 inserts
    // default initials '...' into rank 9, runs the marked-entry ranking blink loop through
    // 11994, clears marker at 11995, and enters Ken's mode-4 Stage 1 intro and ROUND 1.
    let ref_11788 = std::fs::read(Path::new(RUN_02_A).join("ram_011788_work.bin")).unwrap();
    assert_eq!(work_11788[0xe29], 1);
    assert_eq!(
        &work_11788[0x0e6e..0x0f0e],
        &ref_11788[0x0e6e..0x0f0e],
        "10x16-byte ranking table (0xff8e6e..0xff8f0d) equals reference after timeout insertion"
    );
    assert_eq!(
        &work_11788[0xf02..0xf0a],
        &[0xff, 0, 0, 0, 5, 3, 2, 0],
        "rank 9 highlight + score updated"
    );
    assert_eq!(
        &work_11788[0xf0a..0xf0e],
        &[0x00, 0x31, 0x31, 0x31],
        "rank 9 default initials '...' inserted"
    );
    assert_eq!(work_11995[0xf02], 0, "rank 9 marker cleared at 11995");
    assert_eq!((work_12148[0xe36], work_12149[0xe36]), (4, 5));

    // Full native capture comparison over 8344..=12183 (F42 + F50 window).
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-name-entry-02-native");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_02_A),
            &assets,
            &out,
            "sf-name-entry-02",
            Some(Path::new(SCENARIO_02)),
            8343,
            12183,
        )
        .unwrap(),
        3841,
    );
    let scope = CompareScope {
        from: Some(8344),
        to: Some(12183),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let cmp_outcome = compare(
        &load_scenario(SCENARIO_02).unwrap(),
        &load_capture(Path::new(RUN_02_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            cmp_outcome,
            Outcome::Match {
                frames_compared: 3840,
                checkpoints_compared: 3840
            }
        ),
        "{cmp_outcome:?}"
    );
}

#[test]
fn untranslated_effect_stream_call_is_reported_and_delayed_letter_input_is_caught() {
    if !present_01() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    // At frame 9746, the cursor slot 0xffe100 is active; read its stream pointer (+0x10..+0x14)
    // and patch a synthetic 0xff stream call with target 0x0004_2999.
    let work_9746 = std::fs::read(Path::new(RUN_01_A).join("ram_009746_work.bin")).unwrap();
    let ptr = u32::from_be_bytes(work_9746[0x6110..0x6114].try_into().unwrap()) as usize;
    rom[ptr..ptr + 6].copy_from_slice(&[0xff, 0x00, 0x00, 0x04, 0x29, 0x99]);
    let mut m = FightMachine::from_fight_checkpoint(Path::new(RUN_01_A), 9746, rom).unwrap();
    m.state.objectram[0x109] = 0;
    m.step();
    assert_eq!(m.effect_stream_calls(), vec![0x42999]);
    assert_eq!(m.unrecovered(), vec![0x42999]);

    // C-NEG: delaying the first letter press at 9800 by 10 frames (to 9810) fails on owned
    // object RAM at 9802 (cursor commit stream) and on 0xff8e6a at 9813 ('A' committed).
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut m = FightMachine::from_fight_checkpoint(Path::new(RUN_01_A), 9746, rom).unwrap();
    let mut events = scenario_input_events(Path::new(SCENARIO_01)).unwrap();
    for ev in &mut events {
        if ev.0 == 9800 || ev.0 == 9804 {
            ev.0 += 10;
        }
    }
    events.sort_by_key(|e| e.0);
    let mut next = events.partition_point(|e| e.0 < 9746);
    let mut held = [false; INPUTS];
    let mut first_stream_diff = None;
    let mut first_initials_diff = None;
    while m.state.frame < 9820 {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        let ref_work =
            std::fs::read(Path::new(RUN_01_A).join(format!("ram_{:06}_work.bin", m.state.frame)))
                .unwrap();
        if first_stream_diff.is_none()
            && m.state.objectram[0x110..0x114] != ref_work[0x6110..0x6114]
        {
            first_stream_diff = Some(m.state.frame);
        }
        if first_initials_diff.is_none() && m.work_ram()[0xe6a] != ref_work[0xe6a] {
            first_initials_diff = Some(m.state.frame);
        }
    }
    assert!(first_stream_diff.is_some_and(|f| (9801..=9805).contains(&f)));
    assert_eq!(first_initials_diff, Some(9813));
}

#[test]
fn ranking_insertion_shifts_lower_ranks_and_marked_wait_decodes_from_fixtures() {
    if !present_01() || !present_02() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();

    // 1. Test 0x424a4 + 0x425ca..0x42614 rank-shift loop when qualifying at rank 0 (0xff8e6e):
    // Start from checkpoint 9744 of sf-name-entry-01 (just before 0x3eeba calls 0x42486),
    // set P1's score at 0xff86d2..0xff86da to 999,900, step through 9745 (qualifies at
    // rank 0: d2 = 0, 0xff8e66 = 0xff8e6e) and 9746 (enters 0x42584), set initials 'AS.'
    // and deactivate the cursor, and step 9747 to run 0x425c4..0x42676.
    let mut m =
        FightMachine::from_fight_checkpoint(Path::new(RUN_01_A), 9744, rom.clone()).unwrap();
    let table_before = m.work_ram()[0x0e6e..0x0f0e].to_vec();
    for (i, &b) in [0, 9, 9, 9, 9, 0, 0, 0].iter().enumerate() {
        m.set_work_byte(0xff86d2 + i as u32, b);
    }
    m.step(); // 9745: 0x424a4 qualifies at rank 0 (0xff8e6e)
    assert_eq!(&m.work_ram()[0x0e66..0x0e6a], &[0x00, 0xff, 0x8e, 0x6e]);
    m.step(); // 9746: 0x42552 spawns cursor and sleeps at 0x42584
    m.set_work_byte(0xff8e6a, 0x0a);
    m.set_work_byte(0xff8e6b, 0x1c);
    m.set_work_byte(0xff8e6c, 0x31);
    m.state.objectram[0x108] = 0;
    m.step(); // 9747: 0x425c4 shifts ranks 0..=8 into 1..=9 and inserts at rank 0
    let table_after = &m.work_ram()[0x0e6e..0x0f0e];
    // Rank 0 has marker 0xff at +4, score 999,900 at +5..+13, and initials 'AS.' at +13..+16,
    // while its rank-label prefix at +0..+4 ('1ST ') is preserved.
    assert_eq!(&table_after[0..4], &table_before[0..4]);
    assert_eq!(table_after[4], 0xff);
    assert_eq!(&table_after[5..13], &[0, 9, 9, 9, 9, 0, 0, 0]);
    assert_eq!(&table_after[13..16], &[0x0a, 0x1c, 0x31]);
    // Ranks 1..=9 now hold the previous ranks 0..=8 (+5..+16 shifted down by 16 bytes,
    // while each row's +0..+5 rank header is preserved).
    for new_rank in 1..=9 {
        let old_base = (new_rank - 1) * 16;
        let new_base = new_rank * 16;
        assert_eq!(
            &table_after[new_base..new_base + 5],
            &table_before[new_base..new_base + 5]
        );
        assert_eq!(
            &table_after[new_base + 5..new_base + 16],
            &table_before[old_base + 5..old_base + 16],
            "rank {new_rank} received old rank {}'s score + initials",
            new_rank - 1
        );
    }

    // 2. Verify Scheduler::from_fixture decodes 0x4275c and subsequent states cleanly from
    // checkpoints across the F50 window (10006..=10366 in -01 and 11788..=12182 in -02).
    for f in [10006, 10050, 10100, 10212, 10213, 10215, 10300, 10366] {
        let outcome = compare_fight_ram_with_inputs(
            Path::new(RUN_01_A),
            rom.clone(),
            Some(Path::new(SCENARIO_01)),
            f,
            f + 1,
        )
        .unwrap();
        assert_eq!(
            outcome,
            CpuRamOutcome::Match { frames: 1 },
            "fixture {f} in -01"
        );
    }
    for f in [11788, 11850, 11994, 11995, 11997, 12100, 12182] {
        let outcome = compare_fight_ram_with_inputs(
            Path::new(RUN_02_A),
            rom.clone(),
            Some(Path::new(SCENARIO_02)),
            f,
            f + 1,
        )
        .unwrap();
        assert_eq!(
            outcome,
            CpuRamOutcome::Match { frames: 1 },
            "fixture {f} in -02"
        );
    }

    // 3. C-NEG: expiring the marked-entry ranking timer 1 frame early at 10211 causes an
    // early exit from 0x4275c at 10212 instead of 10213 and diverges on owned RAM (0xff8f02
    // and 0xffc008 cleared 1 frame early).
    let mut m_neg = FightMachine::from_fight_checkpoint(Path::new(RUN_01_A), 10211, rom).unwrap();
    m_neg.set_work_byte(0xff8e2d, 0x00);
    m_neg.set_work_byte(0xff8e2e, 0x01); // expires on frame 10212 instead of 10213
    m_neg.step();
    let ref_10212 = std::fs::read(Path::new(RUN_01_A).join("ram_010212_work.bin")).unwrap();
    assert_eq!((m_neg.work_ram()[0x0f02], ref_10212[0x0f02]), (0x00, 0xff));
}

#[test]
fn end_to_end_from_checkpoint_2389_matches_both_name_entry_captures() {
    if !present_01() || !present_02() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();

    // 1. sf-name-entry-01: 2389..=10367 (Retsu match won vs live CPU -> Geki's FIGHT! ->
    // challenger interrupt -> versus lost -> name entry 'AS.' -> ranking -> Ken's carry-on).
    let outcome_01 = compare_fight_ram_with_inputs(
        Path::new(RUN_01_A),
        rom.clone(),
        Some(Path::new(SCENARIO_01)),
        2389,
        10367,
    )
    .unwrap();
    assert_eq!(outcome_01, CpuRamOutcome::Match { frames: 2025 });

    let mut stage_at_geki = None;
    let mut challenger_join_mode = None;
    let (last_01, unrecovered_01) = observe_cpu_fight(
        Path::new(RUN_01_A),
        rom.clone(),
        Some(Path::new(SCENARIO_01)),
        2389,
        10367,
        |m| {
            if m.state.frame == 5840 {
                stage_at_geki = Some((m.work_ram()[0x4004], m.work_ram()[0xe36]));
            }
            if m.state.frame == 5871 {
                challenger_join_mode = Some((m.work_ram()[0x4002], m.work_ram()[0x4008]));
            }
        },
    )
    .unwrap();
    assert_eq!((last_01, unrecovered_01), (10367, vec![]));
    assert_eq!(
        stage_at_geki,
        Some((1, 5)),
        "Geki (opp 1) FIGHT! (sub 5) active at 5840"
    );
    assert_eq!(
        challenger_join_mode,
        Some((2, 0x10)),
        "challenger join (mode 2, scene 0x10) at 5871"
    );

    let out_01 = ScratchDir::new("sf-name-entry-01-e2e-2389");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_01_A),
            &assets,
            &out_01,
            "sf-name-entry-01",
            Some(Path::new(SCENARIO_01)),
            2389,
            10367,
        )
        .unwrap(),
        7979,
    );
    let scope_01 = CompareScope {
        from: Some(2390),
        to: Some(10367),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let cmp_01 = compare(
        &load_scenario(SCENARIO_01).unwrap(),
        &load_capture(Path::new(RUN_01_A)).unwrap(),
        &load_capture(&out_01).unwrap(),
        CandidateKind::Native,
        &scope_01,
    )
    .unwrap();
    assert!(
        matches!(
            cmp_01,
            Outcome::Match {
                frames_compared: 7978,
                checkpoints_compared: 2025
            }
        ),
        "{cmp_01:?}"
    );

    // 2. sf-name-entry-02: 2389..=12183 (same fight and versus loss -> 30-second timeout ->
    // '...' inserted at rank 9 -> ranking -> Ken's carry-on).
    let outcome_02 = compare_fight_ram_with_inputs(
        Path::new(RUN_02_A),
        rom,
        Some(Path::new(SCENARIO_02)),
        2389,
        12183,
    )
    .unwrap();
    assert_eq!(outcome_02, CpuRamOutcome::Match { frames: 3841 });

    let out_02 = ScratchDir::new("sf-name-entry-02-e2e-2389");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_02_A),
            &assets,
            &out_02,
            "sf-name-entry-02",
            Some(Path::new(SCENARIO_02)),
            2389,
            12183,
        )
        .unwrap(),
        9795,
    );
    let scope_02 = CompareScope {
        from: Some(2390),
        to: Some(12183),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let cmp_02 = compare(
        &load_scenario(SCENARIO_02).unwrap(),
        &load_capture(Path::new(RUN_02_A)).unwrap(),
        &load_capture(&out_02).unwrap(),
        CandidateKind::Native,
        &scope_02,
    )
    .unwrap();
    assert!(
        matches!(
            cmp_02,
            Outcome::Match {
                frames_compared: 9794,
                checkpoints_compared: 3841
            }
        ),
        "{cmp_02:?}"
    );
}
