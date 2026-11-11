//! Ticket #45 (F5), #65 (F25), and #79 (F39): CPU match win: lose quote,
//! ranking check, continue countdown, continue expiry to game over, and
//! continue with a credit into the rematch against the same opponent.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::cpu::CpuMachine;
use sf_engine::run::{
    compare_cpu_ram, compare_cpu_ram_with_inputs, cpu_owned_addresses, cpu_work_image,
    observe_cpu_fight, run_cpu_fight_capture, run_cpu_fight_capture_with_inputs, CpuRamOutcome,
    ScratchDir,
};
use sf_video::romset::{load_assets, Assets};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_match_end_01.json";
const SCENARIO_ID: &str = "sf-match-end-01";
const ROUND2_02_SCENARIO: &str = "../../tools/reference/scenarios/sf_round2_02.json";
const RUN_A: &str = "../../tools/reference/local/captures/match-end-01-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/match-end-01-run-b";
const AUDIT_A: &str = "../../tools/reference/local/captures/fight-audit-run-a";
const MUSIC_03_A: &str = "../../tools/reference/local/captures/music-03-audio-a";
const CPU_RUN_A: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const ROUND2_02_A: &str = "../../tools/reference/local/captures/round2-02-run-a";
const CONTINUE_SCENARIO: &str = "../../tools/reference/scenarios/sf_continue_01.json";
const CONTINUE_ID: &str = "sf-continue-01";
const CONTINUE_A: &str = "../../tools/reference/local/captures/continue-01-run-a";
const CONTINUE_B: &str = "../../tools/reference/local/captures/continue-01-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present() -> bool {
    [
        RUN_A,
        RUN_B,
        AUDIT_A,
        MUSIC_03_A,
        CPU_RUN_A,
        ROUND2_02_A,
        CONTINUE_A,
        CONTINUE_B,
        ROMS,
    ]
    .iter()
    .all(|p| Path::new(p).is_dir())
}

fn assets() -> Assets {
    load_assets(ROMS, MANIFEST).expect("ROM regions assemble")
}

fn area(addr: u32) -> &'static str {
    match addr {
        0xff8006..=0xff8105 if (addr - 0xff8006) % 16 >= 2 => "task slots' saved SR/PC/USP",
        0xff8106..=0xff8631 => "stack memory",
        _ => "other",
    }
}

fn frame_rows(capture: &str) -> BTreeMap<u32, String> {
    std::fs::read_to_string(Path::new(capture).join("frames.tsv"))
        .expect("frames.tsv")
        .lines()
        .skip(1)
        .filter_map(|l| {
            let (f, rest) = l.split_once('\t')?;
            Some((f.parse().ok()?, rest.to_string()))
        })
        .collect()
}

#[test]
fn sf_match_end_01_reference_runs_match_and_equal_audit_and_music_03() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).expect("scenario loads");
    assert_eq!(scenario.checkpoints.len(), 6400 - 5000 + 1);
    assert_eq!(scenario.checkpoints.first().copied(), Some(5000));
    assert_eq!(scenario.checkpoints.last().copied(), Some(6400));

    let run_a = Path::new(RUN_A);
    let run_b = Path::new(RUN_B);
    let audit_a = Path::new(AUDIT_A);

    let mut files: Vec<_> = std::fs::read_dir(run_a)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut compared = 0;
    for a in &files {
        let name = a.file_name().unwrap().to_string_lossy().to_string();
        if !(name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba")) {
            continue;
        }
        let b = run_b.join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: match-end-01-run-a vs match-end-01-run-b differed"
        );
        compared += 1;
    }
    // 1,401 checkpoints * 5 files + 3 .tsv files = 7,008 files.
    assert_eq!(compared, 7008);

    let match_rows = frame_rows(RUN_A);
    let audit_rows = frame_rows(AUDIT_A);
    let music_rows = frame_rows(MUSIC_03_A);
    assert_eq!(match_rows.len(), 6401, "frames 0..=6400");

    // Equals sf-fixed-fight-audit-01 through 5040 (frames.tsv and checkpoints 5000..=5040).
    for f in 0..=5040u32 {
        assert_eq!(
            match_rows.get(&f),
            audit_rows.get(&f),
            "frames.tsv row {f} differs between sf-match-end-01 and fight-audit-run-a"
        );
    }
    for cp in 5000..=5040u32 {
        for suffix in [
            format!("pixels_{cp:06}.rgba"),
            format!("ram_{cp:06}_work.bin"),
            format!("ram_{cp:06}_video.bin"),
            format!("ram_{cp:06}_palette.bin"),
            format!("ram_{cp:06}_music.bin"),
        ] {
            assert_eq!(
                std::fs::read(run_a.join(&suffix)).unwrap(),
                std::fs::read(audit_a.join(&suffix)).unwrap(),
                "checkpoint {cp} file {suffix} differs from fight-audit-run-a"
            );
        }
    }

    // Equals music-03-audio-a's frames.tsv over 0..=6400.
    for f in 0..=6400u32 {
        assert_eq!(
            match_rows.get(&f),
            music_rows.get(&f),
            "frames.tsv row {f} differs between sf-match-end-01 and music-03-audio-a"
        );
    }
}

#[test]
fn native_cpu_match_win_matches_2389_to_6330_reports_0x3ccae_at_6331_and_round2_02_matches_to_5600()
{
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    let run_a = Path::new(RUN_A);
    let cpu_a = Path::new(CPU_RUN_A);
    let r2_a = Path::new(ROUND2_02_A);

    let owned = cpu_owned_addresses();
    let owned_set: BTreeSet<u32> = owned.iter().map(|(_, a)| *a).collect();
    for required in [
        0xff8e16, 0xff8e1e, 0xff8e23, 0xff8e35, 0xff8e52, 0xff8e5e, 0xff8e62, 0xff8e6d, 0xff86d2,
        0xff86d9, 0xffc000, 0xffc008, 0xffc00b,
    ] {
        assert!(
            owned_set.contains(&required),
            "0x{required:06x} must be owned"
        );
    }

    let mut latches = Vec::new();
    let mut others = Vec::new();
    let (end_frame, unrec) = observe_cpu_fight(
        cpu_a,
        assets.maincpu.clone(),
        Some(Path::new(SCENARIO)),
        2389,
        6331,
        |m| {
            let f = m.state.frame;
            if f >= 5015 && m.state.sound_latch != 0xff {
                latches.push((f, m.state.sound_latch));
            }
            if f >= 5000 {
                let ref_work = std::fs::read(run_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
                let nat_work = cpu_work_image(m);
                for (i, (&r, &n)) in ref_work.iter().zip(nat_work.iter()).enumerate() {
                    let addr = 0xff8000 + i as u32;
                    if r != n && (owned_set.contains(&addr) || area(addr) == "other") {
                        others.push(format!(
                            "frame {f}: 0x{addr:06x}: ref {r:#04x}, native {n:#04x}"
                        ));
                    }
                }
            }
        },
    )
    .unwrap();
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed over 5000..=6330: {others:#?}"
    );
    assert_eq!((end_frame, unrec), (6331, vec![]));
    assert_eq!(
        latches,
        vec![
            (5076, 0x9d),
            (5415, 0x3f),
            (5418, 0x3c),
            (6101, 0x3f),
            (6102, 0x1b),
            (6105, 0x1b),
            (6109, 0x1b),
            (6113, 0x1b),
            (6117, 0x1b),
            (6121, 0x1b),
            (6125, 0x1b),
            (6129, 0x1b),
            (6133, 0x1b),
            (6137, 0x1b),
            (6151, 0x95),
        ]
    );

    // Full C-NAT capture compare over 5000..=6330 via sf_compare::compare.
    let out_dir = ScratchDir::new("sf-match-end-01-native");
    let frames = run_cpu_fight_capture_with_inputs(
        cpu_a,
        &assets,
        &out_dir,
        SCENARIO_ID,
        Some(Path::new(SCENARIO)),
        2389,
        6330,
    )
    .expect("native match-end capture through 6330");
    assert_eq!(frames, 6330 - 2389 + 1);

    let scenario = load_scenario(SCENARIO).unwrap();
    let ref_cap = load_capture(run_a).unwrap();
    let nat_cap = load_capture(&out_dir).unwrap();
    let scope = CompareScope {
        from: Some(2390),
        to: Some(6330),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(&scenario, &ref_cap, &nat_cap, CandidateKind::Native, &scope)
        .expect("compatible captures");
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 3941,
                checkpoints_compared: 1331
            }
        ),
        "C-NAT full capture compare over 2390..=6330 failed: {outcome:?}"
    );

    // Acceptance 3: sf-round2-02 continues natively past 0x3eefa and matches
    // through 5600, including its continue screen from 5200.
    let r2_ram = compare_cpu_ram(r2_a, assets.maincpu.clone(), 3900, 5600).unwrap();
    assert_eq!(
        r2_ram,
        CpuRamOutcome::Match {
            frames: 5600 - 3900
        }
    );

    let out_r2 = ScratchDir::new("sf-round2-02-continue");
    run_cpu_fight_capture(r2_a, &assets, &out_r2, "sf-round2-02", 3900, 5600)
        .expect("sf-round2-02 native capture through 5600");
    let r2_scenario = load_scenario(ROUND2_02_SCENARIO).unwrap();
    let r2_ref = load_capture(r2_a).unwrap();
    let r2_nat = load_capture(&out_r2).unwrap();
    let r2_scope = CompareScope {
        from: Some(3901),
        to: Some(5600),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let r2_outcome = compare(
        &r2_scenario,
        &r2_ref,
        &r2_nat,
        CandidateKind::Native,
        &r2_scope,
    )
    .expect("compatible captures");
    assert!(
        matches!(
            r2_outcome,
            Outcome::Match {
                frames_compared: 1700,
                checkpoints_compared: 1700
            }
        ),
        "sf-round2-02 compare over 3901..=5600 failed: {r2_outcome:?}"
    );
}

#[test]
fn ranking_check_continue_limit_and_continue_taken_branches_are_reported() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    let run_a = Path::new(RUN_A);

    // 1. A qualifying score (> 10th place at 0xff8f03) records its entry and
    //    enters name entry (0x42486..0x4254c -> resume 0x42552) in frame 5415.
    let mut m = CpuMachine::from_fight_checkpoint(run_a, 5014, assets.maincpu.clone()).unwrap();
    while m.state.frame < 5414 {
        m.step();
    }
    m.set_work_byte(0xff86d2, 9);
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (5415, vec![]));
    assert!(m.task_positions().contains(&(1, 0x42552)));
    assert_eq!(&m.work_ram()[0xe66..0xe6a], &[0, 0xff, 0x8e, 0x6e]);
    assert_eq!(&m.work_ram()[0xe6a..0xe6e], &[0x31, 0x31, 0x31, 0x00]);

    // 2. The tie rule: tying 1st place (entry 0 at 0xff8e73) qualifies at
    //    rank 1, whereas tying 10th place (entry 9 at 0xff8f03) does NOT
    //    qualify and clears P1's score at 0xff86d2..0xff86da.
    let mut m = CpuMachine::from_fight_checkpoint(run_a, 5014, assets.maincpu.clone()).unwrap();
    while m.state.frame < 5414 {
        m.step();
    }
    for i in 0..7u32 {
        let b = m.work_ram()[(0xff8e73 - 0xff8000 + i) as usize];
        m.set_work_byte(0xff86d2 + i, b);
    }
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (5415, vec![]));
    assert!(m.task_positions().contains(&(1, 0x42552)));
    assert_eq!(&m.work_ram()[0xe66..0xe6a], &[0, 0xff, 0x8e, 0x7e]);

    let mut m = CpuMachine::from_fight_checkpoint(run_a, 5014, assets.maincpu.clone()).unwrap();
    while m.state.frame < 5414 {
        m.step();
    }
    for i in 0..7u32 {
        let b = m.work_ram()[(0xff8f03 - 0xff8000 + i) as usize];
        m.set_work_byte(0xff86d2 + i, b);
    }
    m.step();
    assert!(m.unrecovered().is_empty());
    assert_eq!(&m.work_ram()[0x6d2..0x6da], &[0u8; 8]);

    // 3. When continuation limit 0xffc000 <= stage 0xffc003 signed (e.g.
    //    0xff = None), 0x3efda enters 0x3f2f0's screen at 5415. Its one-frame
    //    palette sleep and 240-frame hold reach the attract boundary at 5656.
    let mut m = CpuMachine::from_fight_checkpoint(run_a, 5014, assets.maincpu.clone()).unwrap();
    while m.state.frame < 5414 {
        m.step();
    }
    m.set_work_byte(0xffc000, 0xff);
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (5415, vec![]));
    while m.state.frame < 5656 {
        m.step();
    }
    assert_eq!(m.unrecovered(), Vec::<u32>::new());
    assert!(m.task_positions().contains(&(1, 0x3ccec)));
    assert!(m.task_positions().contains(&(3, 0x3f3a2)));

    // 4. 0x3f166: during the continue loop (frame 5418), a credit (0xff865a..5e >= 1) and P1
    //    start pressed (SYSTEM bit 0 low) accepts the continue (F39, ticket #79): queues the
    //    boot sounds (0x27, 0x3f, 0x80) at 5418, then at 5419 erases CONTINUE and the digits,
    //    clears 0xff8e1e, takes 1 credit (0xff865d = 0), and requests the country palette.
    let mut m = CpuMachine::from_fight_checkpoint(run_a, 5014, assets.maincpu).unwrap();
    while m.state.frame < 5417 {
        m.step();
    }
    m.set_work_byte(0xff865d, 1);
    m.set_system(0xff7e);
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (5418, vec![]));
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (5419, vec![]));
    assert_eq!(m.work_ram()[0x65d], 0);
    assert_eq!(&m.work_ram()[0xe1e..0xe22], &[0, 0, 0, 0]);
}

#[test]
fn starting_counter_1_one_pass_late_fails_comparison() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut bad_assets = assets();
    let run_a = Path::new(RUN_A);
    // 0x3f070: move.b #2, ($e16,A5) (1b7c 0002 0e16) -> zero 0x3f073 so counter 1 is started
    // one pass late (on the first pass of 0x3f092 in frame 5418 instead of 0x3f070 in frame 5417).
    assert_eq!(bad_assets.maincpu[0x3f073], 2);
    bad_assets.maincpu[0x3f073] = 0;

    // 1. Owned CPU RAM comparison from checkpoint 5014 catches 0xff8e16 at frame 5417
    //    (0x00 instead of 0x02).
    let ram_outcome = compare_cpu_ram(run_a, bad_assets.maincpu.clone(), 5014, 5425).unwrap();
    match ram_outcome {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 5417") && msg.contains("0xff8e16"),
                "expected frame 5417 / 0xff8e16 mismatch, got {msg}"
            );
        }
        other => panic!("expected Mismatch at frame 5417, got {other:?}"),
    }

    // 2. Full capture comparison from checkpoint 5014 catches the continue screen's big digits
    //    at frame 5418 (00.00 drawn instead of 10.00 when counter 1 starts one pass late).
    let out_dir = ScratchDir::new("sf-match-end-01-late-counter1");
    run_cpu_fight_capture(run_a, &bad_assets, &out_dir, SCENARIO_ID, 5014, 5420)
        .expect("perturbed match-end capture");
    let scenario = load_scenario(SCENARIO).unwrap();
    let ref_cap = load_capture(run_a).unwrap();
    let bad_cap = load_capture(&out_dir).unwrap();
    let scope = CompareScope {
        from: Some(5015),
        to: Some(5420),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(&scenario, &ref_cap, &bad_cap, CandidateKind::Native, &scope)
        .expect("compatible captures");
    match outcome {
        Outcome::Mismatch(m) => {
            assert_eq!(m.frame, 5418);
        }
        other => panic!("expected Mismatch at frame 5418, got {other:?}"),
    }
}

#[test]
fn sf_continue_01_repeats_equals_match_end_before_coin_and_matches_2389_through_7000() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(CONTINUE_SCENARIO).expect("continue scenario loads");
    assert_eq!(scenario.duration_frames, 7001);
    assert_eq!(scenario.checkpoints.len(), 7000 - 5400 + 1);
    assert_eq!(scenario.checkpoints.first().copied(), Some(5400));
    assert_eq!(scenario.checkpoints.last().copied(), Some(7000));

    let cont_a = Path::new(CONTINUE_A);
    let cont_b = Path::new(CONTINUE_B);
    let match_a = Path::new(RUN_A);
    let cpu_a = Path::new(CPU_RUN_A);

    let mut files: Vec<_> = std::fs::read_dir(cont_a)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut compared = 0;
    for a in &files {
        let name = a.file_name().unwrap().to_string_lossy().to_string();
        if !(name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba")) {
            continue;
        }
        let b = cont_b.join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: continue-01-run-a vs continue-01-run-b differed"
        );
        compared += 1;
    }
    // 1,601 checkpoints * 5 files + 3 .tsv files = 8,008 files.
    assert_eq!(compared, 8008);

    // Equals sf-match-end-01 before the coin at frame 5500 (frames.tsv 0..=5500 and checkpoints 5400..=5500).
    let cont_rows = frame_rows(CONTINUE_A);
    let match_rows = frame_rows(RUN_A);
    assert_eq!(cont_rows.len(), 7001);
    for f in 0..=5500u32 {
        assert_eq!(
            cont_rows.get(&f),
            match_rows.get(&f),
            "frames.tsv row {f} differs between sf-continue-01 and sf-match-end-01 before coin"
        );
    }
    for cp in 5400..=5500u32 {
        for suffix in [
            format!("pixels_{cp:06}.rgba"),
            format!("ram_{cp:06}_work.bin"),
            format!("ram_{cp:06}_video.bin"),
            format!("ram_{cp:06}_palette.bin"),
            format!("ram_{cp:06}_music.bin"),
        ] {
            assert_eq!(
                std::fs::read(cont_a.join(&suffix)).unwrap(),
                std::fs::read(match_a.join(&suffix)).unwrap(),
                "checkpoint {cp} file {suffix} differs from sf-match-end-01 before coin"
            );
        }
    }

    let assets = assets();
    let owned = cpu_owned_addresses();
    let owned_set: BTreeSet<u32> = owned.iter().map(|(_, a)| *a).collect();
    for required in [
        0xff863a, 0xff8641, 0xff865a, 0xff865d, 0xff86b8, 0xff86e2, 0xffc66c, 0xffc003, 0xffc004,
    ] {
        assert!(
            owned_set.contains(&required),
            "0x{required:06x} must be owned"
        );
    }

    let mut latches = Vec::new();
    let mut others = Vec::new();
    let mut credit_at_5514 = 0u8;
    let mut credit_at_5562 = 0xffu8;
    let mut rematch_stage_fight_wins = (0xffu8, 0xffu8, 0xffu8, 0xffu8);
    let (end_frame, unrec) = observe_cpu_fight(
        cpu_a,
        assets.maincpu.clone(),
        Some(Path::new(CONTINUE_SCENARIO)),
        2389,
        7000,
        |m| {
            let f = m.state.frame;
            if (5400..=6000).contains(&f) && m.state.sound_latch != 0xff {
                latches.push((f, m.state.sound_latch));
            }
            if f == 5514 {
                credit_at_5514 = m.work_ram()[0x65d];
            }
            if f == 5562 {
                credit_at_5562 = m.work_ram()[0x65d];
            }
            if f == 5967 {
                rematch_stage_fight_wins = (
                    m.work_ram()[0x4003],
                    m.work_ram()[0x4004],
                    m.work_ram()[0x6b8],
                    m.work_ram()[0x466c],
                );
            }
            if f >= 5400 {
                let ref_pal =
                    std::fs::read(cont_a.join(format!("ram_{f:06}_palette.bin"))).unwrap();
                assert_eq!(m.state.paletteram, ref_pal, "palette mismatch at frame {f}");
                let ref_work = std::fs::read(cont_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
                let nat_work = cpu_work_image(m);
                for (i, (&r, &n)) in ref_work.iter().zip(nat_work.iter()).enumerate() {
                    let addr = 0xff8000 + i as u32;
                    if r != n && (owned_set.contains(&addr) || area(addr) == "other") {
                        others.push(format!(
                            "frame {f}: 0x{addr:06x}: ref {r:#04x}, native {n:#04x}"
                        ));
                    }
                }
            }
        },
    )
    .unwrap();
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed over 5400..=7000: {others:#?}"
    );
    assert_eq!((end_frame, unrec), (7000, vec![]));
    assert_eq!(credit_at_5514, 1, "coin1 awards 1 credit before continue");
    assert_eq!(
        credit_at_5562, 0,
        "continue acceptance takes 1 credit at 5562"
    );
    assert_eq!(
        rematch_stage_fight_wins,
        (0, 0, 0, 0),
        "Retsu (stage 0, fight 0) is the opponent again with both fighters wins cleared"
    );
    assert_eq!(
        latches,
        vec![
            (5415, 0x3f),
            (5418, 0x3c),
            (5514, 0x0f),
            (5562, 0x27),
            (5563, 0x3f),
            (5564, 0x80),
            (5565, 0x33),
            (5723, 0x97),
            (5724, 0x0b),
            (5881, 0x27),
            (5883, 0x3f),
            (5886, 0x28),
            (5967, 0x94),
        ]
    );

    // Full C-NAT capture compare over 2390..=7000 (4,611 frames and 1,601 checkpoints 5400..=7000).
    let out_dir = ScratchDir::new("sf-continue-01-native");
    let frames = run_cpu_fight_capture_with_inputs(
        cpu_a,
        &assets,
        &out_dir,
        CONTINUE_ID,
        Some(Path::new(CONTINUE_SCENARIO)),
        2389,
        7000,
    )
    .expect("native continue capture through 7000");
    assert_eq!(frames, 7000 - 2389 + 1);

    let ref_cap = load_capture(cont_a).unwrap();
    let nat_cap = load_capture(&out_dir).unwrap();
    let scope = CompareScope {
        from: Some(2390),
        to: Some(7000),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(&scenario, &ref_cap, &nat_cap, CandidateKind::Native, &scope)
        .expect("compatible captures");
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 4611,
                checkpoints_compared: 1601
            }
        ),
        "C-NAT full capture compare over 2390..=7000 failed: {outcome:?}"
    );
}

#[test]
fn not_taking_credit_at_0x3f1a2_fails_at_frame_5562() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut bad_assets = assets();
    let cont_a = Path::new(CONTINUE_A);
    // 0x3f1a2: lea ($65e,A5), A0 (41ed 065e) -> zero the displacement word 0x3f1a4..0x3f1a6
    // so Flow::ContinueAccept does not decrement 0xff865d when continue is accepted at 5562.
    assert_eq!(&bad_assets.maincpu[0x3f1a4..0x3f1a6], &[0x06, 0x5e]);
    bad_assets.maincpu[0x3f1a4] = 0;
    bad_assets.maincpu[0x3f1a5] = 0;

    let ram_outcome = compare_cpu_ram_with_inputs(
        cont_a,
        bad_assets.maincpu,
        Some(Path::new(CONTINUE_SCENARIO)),
        5550,
        5570,
    )
    .unwrap();
    match ram_outcome {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 5562")
                    && msg.contains("0xff865d")
                    && msg.contains("reference 00, native 01"),
                "expected frame 5562 / 0xff865d mismatch, got {msg}"
            );
        }
        other => panic!("expected Mismatch at frame 5562, got {other:?}"),
    }
}
