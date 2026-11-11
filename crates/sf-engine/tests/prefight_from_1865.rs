//! Ticket #44 (F4): Pre-fight screens of the fixed fight from country
//! select's exit (frames 1865..=2389, continuing into the fight through 5014).
//!
//! Verifies:
//! 1. C-REF: `sf-prefight-01` (fixed-fight inputs, 2,400 frames, checkpoints
//!    every frame 1860..=2400) matches across two runs (`2,708` files),
//!    equals `fight-run-a` over `frames.tsv` rows `0..=2399` and checkpoints
//!    `1860`, `1980`, `2160`, `2280`, equals `cpu-01-run-a` at checkpoint
//!    `2389`, and matches the no-1900-start probe capture across all `2,708`
//!    files.
//! 2. C-NAT: from checkpoint `1865` over `1866..=2389`, native execution
//!    matches `sf-prefight-01` on every frame (screen hash, 384x224 pixels,
//!    video RAM, palette RAM, sound latch with `0x3f` at 1866, `0x8f` at 1868,
//!    `0x33` at 1959, `0x97`/`0x0b` at 2117/2118, `0x27`/`0x3f` at 2275/2277,
//!    `0x28` at 2280, `0x94` at 2361, and owned work RAM), and continues from
//!    `1865` with `sf-fixed-fight-01`'s inputs matching `sf-fixed-fight-audit-01`
//!    through frame `5014`.
//! 3. C-STOP: map (`0x3f812` when `0xffc003 != 0` in event 2) and demo branches
//!    (`0x3e5c6` and `0x3e6fe` when `0xff8003 == 0`) are reported.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::cpu::CpuMachine;
use sf_engine::run::{
    compare_cpu_ram, compare_cpu_ram_with_inputs, cpu_owned_addresses, cpu_work_image,
    observe_cpu_fight, run_cpu_fight_capture, CpuRamOutcome, ScratchDir,
};
use sf_video::romset::{load_assets, Assets};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_prefight_01.json";
const SCENARIO_ID: &str = "sf-prefight-01";
const FIXED_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_01.json";
const RUN_A: &str = "../../tools/reference/local/captures/prefight-01-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/prefight-01-run-b";
const NO1900_PROBE: &str = "../../tools/reference/local/captures/prefight-no1900-probe";
const FIGHT_RUN_A: &str = "../../tools/reference/local/captures/fight-run-a";
const CPU_RUN_A: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const AUDIT_RUN_A: &str = "../../tools/reference/local/captures/fight-audit-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present() -> bool {
    [
        RUN_A,
        RUN_B,
        NO1900_PROBE,
        FIGHT_RUN_A,
        CPU_RUN_A,
        AUDIT_RUN_A,
        ROMS,
    ]
    .iter()
    .all(|p| Path::new(p).is_dir())
}

fn assets() -> Assets {
    load_assets(ROMS, MANIFEST).expect("ROM regions assemble")
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

fn area(addr: u32) -> &'static str {
    match addr {
        0xff8006..=0xff8105 if (addr - 0xff8006) % 16 >= 2 => "task slots' saved SR/PC/USP",
        0xff8106..=0xff8631 => "stack memory",
        _ => "other",
    }
}

fn assert_dirs_identical(dir_a: &Path, dir_b: &Path, expected_count: usize) {
    let mut files: Vec<_> = std::fs::read_dir(dir_a)
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
        let b = dir_b.join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: {} vs {} differed",
            dir_a.display(),
            dir_b.display()
        );
        compared += 1;
    }
    assert_eq!(compared, expected_count);
}

#[test]
fn sf_prefight_01_reference_runs_and_no1900_probe_match_and_equal_baselines() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).expect("scenario loads");
    assert_eq!(scenario.checkpoints.len(), 2400 - 1860 + 1);
    assert_eq!(scenario.checkpoints.first().copied(), Some(1860));
    assert_eq!(scenario.checkpoints.last().copied(), Some(2400));

    let run_a = Path::new(RUN_A);
    let run_b = Path::new(RUN_B);
    let no1900 = Path::new(NO1900_PROBE);
    let fight_a = Path::new(FIGHT_RUN_A);
    let cpu_a = Path::new(CPU_RUN_A);

    // 541 checkpoints * 5 files + 3 .tsv files = 2,708 files.
    assert_dirs_identical(run_a, run_b, 2708);
    // Acceptance 5: omitting the 1900 start press is byte-for-byte identical
    // on every frames.tsv field and every checkpoint over 0..=2400.
    assert_dirs_identical(run_a, no1900, 2708);

    let prefight_rows = frame_rows(RUN_A);
    let fight_rows = frame_rows(FIGHT_RUN_A);
    assert_eq!(prefight_rows.len(), 2401, "frames 0..=2400");
    for f in 0..=2399u32 {
        assert_eq!(
            prefight_rows.get(&f),
            fight_rows.get(&f),
            "frames.tsv row {f} differs between sf-prefight-01 and fight-run-a"
        );
    }

    for cp in [1860u32, 1980, 2160, 2280] {
        for suffix in [
            format!("pixels_{cp:06}.rgba"),
            format!("ram_{cp:06}_work.bin"),
            format!("ram_{cp:06}_video.bin"),
            format!("ram_{cp:06}_palette.bin"),
            format!("ram_{cp:06}_music.bin"),
        ] {
            assert_eq!(
                std::fs::read(run_a.join(&suffix)).unwrap(),
                std::fs::read(fight_a.join(&suffix)).unwrap(),
                "checkpoint {cp} file {suffix} differs from fight-run-a"
            );
        }
    }

    for suffix in [
        "pixels_002389.rgba",
        "ram_002389_work.bin",
        "ram_002389_video.bin",
        "ram_002389_palette.bin",
        "ram_002389_music.bin",
    ] {
        assert_eq!(
            std::fs::read(run_a.join(suffix)).unwrap(),
            std::fs::read(cpu_a.join(suffix)).unwrap(),
            "checkpoint 2389 file {suffix} differs from cpu-01-run-a"
        );
    }
}

#[test]
fn native_prefight_from_1865_matches_1866_to_2389_and_continues_to_5014() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    let run_a = Path::new(RUN_A);
    let audit_a = Path::new(AUDIT_RUN_A);

    let owned = cpu_owned_addresses();
    let owned_set: BTreeSet<u32> = owned.iter().map(|(_, a)| *a).collect();
    for required in [
        0xff86ba, 0xff86bc, 0xff86be, 0xff86c0, 0xff86d0, 0xffc66e, 0xffc670, 0xffc672, 0xffc674,
        0xffe438, 0xffe43c, 0xffea38, 0xffea3c, 0xff8026, 0xff8027, 0xff8633, 0xff8639, 0xff8e36,
    ] {
        assert!(
            owned_set.contains(&required),
            "0x{required:06x} must be owned"
        );
    }

    let outcome = compare_cpu_ram(run_a, assets.maincpu.clone(), 1865, 2389).unwrap();
    assert_eq!(
        outcome,
        CpuRamOutcome::Match {
            frames: 2389 - 1865
        }
    );

    let mut latches = Vec::new();
    let mut others = Vec::new();
    let (end_frame, unrec) =
        observe_cpu_fight(run_a, assets.maincpu.clone(), None, 1865, 2389, |m| {
            let f = m.state.frame;
            if m.state.sound_latch != 0xff {
                latches.push((f, m.state.sound_latch));
            }
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
        })
        .unwrap();
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed: {others:#?}"
    );
    assert_eq!((end_frame, unrec), (2389, Vec::<u32>::new()));
    assert_eq!(
        latches,
        vec![
            (1866, 0x3f),
            (1868, 0x8f),
            (1959, 0x33),
            (2117, 0x97),
            (2118, 0x0b),
            (2275, 0x27),
            (2277, 0x3f),
            (2280, 0x28),
            (2361, 0x94),
        ]
    );

    // Full C-NAT capture compare over 1866..=2389 via sf_compare::compare.
    let out_dir = ScratchDir::new("sf-prefight-01-native");
    let frames = run_cpu_fight_capture(run_a, &assets, &out_dir, SCENARIO_ID, 1865, 2389)
        .expect("native prefight capture");
    assert_eq!(frames, 2389 - 1865 + 1);

    let scenario = load_scenario(SCENARIO).unwrap();
    let ref_cap = load_capture(run_a).unwrap();
    let nat_cap = load_capture(&out_dir).unwrap();
    let scope = CompareScope {
        from: Some(1866),
        to: Some(2389),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(&scenario, &ref_cap, &nat_cap, CandidateKind::Native, &scope)
        .expect("compatible captures");
    assert!(
        matches!(outcome, Outcome::Match { .. }),
        "C-NAT full capture compare over 1866..=2389 failed: {outcome:?}"
    );

    // Acceptance 4: continue from 1865 through 5014 with sf-fixed-fight-01's
    // inputs and compare against sf-fixed-fight-audit-01 over 2389..=5014.
    let mut continued_checks = 0u32;
    let (end_cont, unrec_cont) = observe_cpu_fight(
        run_a,
        assets.maincpu.clone(),
        Some(Path::new(FIXED_SCENARIO)),
        1865,
        5014,
        |m| {
            let f = m.state.frame;
            if f < 2389 {
                return;
            }
            let ref_work = std::fs::read(audit_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
            let nat_work = cpu_work_image(m);
            for &(name, addr) in &owned {
                let i = (addr - 0xff8000) as usize;
                assert_eq!(
                    nat_work[i], ref_work[i],
                    "frame {f}: {name} at {addr:#08x} differs from fight-audit-run-a"
                );
            }
            continued_checks += 1;
        },
    )
    .unwrap();
    assert_eq!((end_cont, unrec_cont), (5014, Vec::<u32>::new()));
    assert_eq!(continued_checks, 5014 - 2389 + 1);
    let cont_match = compare_cpu_ram_with_inputs(
        audit_a,
        assets.maincpu,
        Some(Path::new(FIXED_SCENARIO)),
        2389,
        5040,
    )
    .unwrap();
    assert_eq!(
        cont_match,
        CpuRamOutcome::Match {
            frames: 5040 - 2389
        }
    );
}

#[test]
fn prefight_map_and_demo_branches_are_reported() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    let run_a = Path::new(RUN_A);

    // 0x3e5c6..0x3e5d0 (translated in #88, F48): demo mode (0xff8003 == 0) at
    // 0x3e59c requests the country palette (0x436b8) and sleeps into 0x436e6.
    let mut m = CpuMachine::from_fight_checkpoint(run_a, 1865, assets.maincpu.clone()).unwrap();
    m.set_work_byte(0xff8003, 0);
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (1866, Vec::<u32>::new()));
    assert!(m.task_positions().contains(&(1, 0x436e6)));

    // 0x3f812..0x3f87a (translated in #83, F43): stage > 0 (0xffc003 != 0) in
    // event 2 draws the travel map and sleeps slot 2 at 0x3f87c on frame 1866.
    let mut m = CpuMachine::from_fight_checkpoint(run_a, 1865, assets.maincpu.clone()).unwrap();
    m.set_work_byte(0xffc003, 1);
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (1866, Vec::<u32>::new()));
    assert!(m.task_positions().contains(&(2, 0x3f87c)));

    // 0x3e6fe..0x3e70e (translated in #88, F48): demo mode (0xff8003 == 0)
    // after slot 2 finishes event 3 at frame 1957 requests the stage palette
    // (0x43748) and sleeps into 0x43776 when slot 1 runs 0x3e6ca on frame 1958.
    let mut m = CpuMachine::from_fight_checkpoint(run_a, 1865, assets.maincpu).unwrap();
    while m.state.frame < 1957 {
        m.step();
    }
    assert!(m.unrecovered().is_empty());
    m.set_work_byte(0xff8003, 0);
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (1958, Vec::<u32>::new()));
    assert!(m.task_positions().contains(&(1, 0x43776)));
}

#[test]
fn shortening_event_3_hold_by_one_frame_fails_comparison() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut bad_assets = assets();
    let run_a = Path::new(RUN_A);
    // 0x3b562: move.w #$003c, D0 (303c 003c) -> shorten event 3's 60-frame hold to 59.
    assert_eq!(&bad_assets.maincpu[0x3b564..0x3b566], &[0x00, 0x3c]);
    bad_assets.maincpu[0x3b565] = 0x3b;

    // 1. Owned CPU RAM comparison catches slot 2's sleep byte (0xff8027) at frame 1867:
    //    59 (0x3b) instead of 60 (0x3c).
    let ram_outcome = compare_cpu_ram(run_a, bad_assets.maincpu.clone(), 1865, 2389).unwrap();
    match ram_outcome {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 1867") && msg.contains("0xff8027"),
                "expected frame 1867 / 0xff8027 mismatch, got {msg}"
            );
        }
        other => panic!("expected Mismatch at frame 1867, got {other:?}"),
    }

    // 2. Full capture comparison catches the screen_hash / videoram divergence at
    //    frame 1956, when event 3 erases the country text block one frame early.
    let out_dir = ScratchDir::new("sf-prefight-01-short-hold");
    run_cpu_fight_capture(run_a, &bad_assets, &out_dir, SCENARIO_ID, 1865, 1960)
        .expect("perturbed prefight capture");
    let scenario = load_scenario(SCENARIO).unwrap();
    let ref_cap = load_capture(run_a).unwrap();
    let bad_cap = load_capture(&out_dir).unwrap();
    let scope = CompareScope {
        from: Some(1866),
        to: Some(1960),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(&scenario, &ref_cap, &bad_cap, CandidateKind::Native, &scope)
        .expect("compatible captures");
    match outcome {
        Outcome::Mismatch(m) => {
            assert_eq!(m.frame, 1956);
        }
        other => panic!("expected Mismatch at frame 1956, got {other:?}"),
    }
}
