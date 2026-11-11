//! Ticket #65 (F25): the game-over effects and hold, both players' ranking
//! checks, the reset, and the boundary where the flow returns to attract.
//! Consumes local-only reference captures and the pinned ROM set.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::cpu::CpuMachine;
use sf_engine::run::{
    apply_inputs, compare_cpu_ram, compare_versus_ram, cpu_owned_addresses, cpu_work_image,
    observe_cpu_fight, observe_versus_fight, run_cpu_fight_capture_with_inputs,
    run_versus_interval_capture, scenario_input_events, versus_owned_addresses, versus_work_image,
    CpuRamOutcome, ScratchDir,
};
use sf_engine::versus::VersusMachine;
use sf_video::romset::{load_assets, Assets};

const CPU_SCENARIO: &str = "../../tools/reference/scenarios/sf_match_end_01.json";
const CPU_A: &str = "../../tools/reference/local/captures/match-end-01-run-a";
const CPU_FIXTURE: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const CREDIT_SCENARIO: &str = "../../tools/reference/scenarios/sf_gameover_credit_01.json";
const CREDIT_A: &str = "../../tools/reference/local/captures/gameover-credit-01-run-a";
const CREDIT_B: &str = "../../tools/reference/local/captures/gameover-credit-01-run-b";
const FIGHT_A: &str = "../../tools/reference/local/captures/fight-run-a";
const DOUBLEKO_SCENARIO: &str = "../../tools/reference/scenarios/sf_doubleko_02.json";
const DOUBLEKO_A: &str = "../../tools/reference/local/captures/doubleko-02-run-a";
const DOUBLEKO_B: &str = "../../tools/reference/local/captures/doubleko-02-run-b";
const BASE_SCENARIO: &str = "../../tools/reference/scenarios/sf_doubleko_01.json";
const BASE_A: &str = "../../tools/reference/local/captures/doubleko-01-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present() -> bool {
    [
        CPU_A,
        CPU_FIXTURE,
        CREDIT_A,
        CREDIT_B,
        FIGHT_A,
        DOUBLEKO_A,
        DOUBLEKO_B,
        BASE_A,
        ROMS,
    ]
    .iter()
    .all(|p| Path::new(p).is_dir())
}

fn assets() -> Assets {
    load_assets(ROMS, MANIFEST).expect("ROM regions assemble")
}

fn scope(from: u32, to: u32) -> CompareScope {
    CompareScope {
        from: Some(from as u64),
        to: Some(to as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

fn frame_rows(capture: &str) -> BTreeMap<u32, String> {
    std::fs::read_to_string(Path::new(capture).join("frames.tsv"))
        .expect("frames.tsv")
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (frame, rest) = line.split_once('\t')?;
            Some((frame.parse().ok()?, rest.to_string()))
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

fn work_differences(
    reference: &Path,
    frame: u32,
    native: &[u8],
    owned: &BTreeSet<u32>,
) -> Vec<String> {
    let reference = std::fs::read(reference.join(format!("ram_{frame:06}_work.bin"))).unwrap();
    reference
        .iter()
        .zip(native)
        .enumerate()
        .filter_map(|(i, (&expected, &actual))| {
            let addr = 0xff8000 + i as u32;
            (expected != actual && (owned.contains(&addr) || area(addr) == "other")).then(|| {
                format!(
                    "frame {frame}: 0x{addr:06x}: reference {expected:#04x}, native {actual:#04x}"
                )
            })
        })
        .collect()
}

fn assert_match(outcome: Outcome, frames: u64, checkpoints: usize) {
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == frames && checkpoints_compared == checkpoints),
        "capture comparison failed: {outcome:?}"
    );
}

#[test]
fn doubleko_02_repeats_exactly_and_preserves_the_doubleko_01_baseline() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(DOUBLEKO_SCENARIO).unwrap();
    let base = load_scenario(BASE_SCENARIO).unwrap();
    assert_eq!(scenario.duration_frames, 6001);
    assert_eq!(scenario.inputs.len(), 141);
    assert_eq!(scenario.checkpoints.len(), 442);
    assert_eq!(scenario.checkpoints.first().copied(), Some(2450));
    assert_eq!(scenario.checkpoints.last().copied(), Some(6000));
    assert_eq!((scenario.dsw1, scenario.dsw2), (0xdfff, 0xffff));
    for (a, b) in scenario.inputs.iter().zip(&base.inputs) {
        assert_eq!(
            (a.frame, &a.press, a.hold_frames),
            (b.frame, &b.press, b.hold_frames)
        );
    }
    assert_eq!(scenario.inputs.len(), base.inputs.len());

    let outcome = compare(
        &scenario,
        &load_capture(DOUBLEKO_A).unwrap(),
        &load_capture(DOUBLEKO_B).unwrap(),
        CandidateKind::Reference,
        &scope(2450, 6000),
    )
    .unwrap();
    assert_match(outcome, 3551, 442);

    let old_rows = frame_rows(BASE_A);
    let new_rows = frame_rows(DOUBLEKO_A);
    for frame in 0..=5800 {
        assert_eq!(
            new_rows.get(&frame),
            old_rows.get(&frame),
            "baseline frame {frame}"
        );
    }
    let mut shared = 0;
    for frame in 5560..=5800 {
        for suffix in ["pixels", "work", "video", "palette", "music"] {
            let name = if suffix == "pixels" {
                format!("pixels_{frame:06}.rgba")
            } else {
                format!("ram_{frame:06}_{suffix}.bin")
            };
            assert_eq!(
                std::fs::read(Path::new(DOUBLEKO_A).join(&name)).unwrap(),
                std::fs::read(Path::new(BASE_A).join(&name)).unwrap(),
                "baseline artifact {name}"
            );
            shared += 1;
        }
    }
    assert_eq!(shared, 241 * 5);
}

#[test]
fn one_player_game_over_matches_through_6330_then_reports_attract_at_6331() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    let owned: BTreeSet<u32> = cpu_owned_addresses().into_iter().map(|(_, a)| a).collect();
    let mut latches = Vec::new();
    let mut differences = Vec::new();
    let (frame, branches) = observe_cpu_fight(
        Path::new(CPU_FIXTURE),
        assets.maincpu.clone(),
        Some(Path::new(CPU_SCENARIO)),
        2389,
        6331,
        |m| {
            let frame = m.state.frame;
            if frame >= 6100 {
                if m.state.sound_latch != 0xff {
                    latches.push((frame, m.state.sound_latch));
                }
                let palette =
                    std::fs::read(Path::new(CPU_A).join(format!("ram_{frame:06}_palette.bin")))
                        .unwrap();
                assert_eq!(
                    m.state.paletteram, palette,
                    "game-over palette at frame {frame}"
                );
            }
            if frame >= 5000 {
                differences.extend(work_differences(
                    Path::new(CPU_A),
                    frame,
                    &cpu_work_image(m),
                    &owned,
                ));
            }
        },
    )
    .unwrap();
    assert_eq!((frame, branches), (6331, vec![]));
    assert!(
        differences.is_empty(),
        "owned and non-stack RAM differed: {differences:#?}"
    );
    assert_eq!(
        latches,
        [
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
            (6151, 0x95)
        ]
    );

    let out = ScratchDir::new("sf-match-end-01-game-over");
    run_cpu_fight_capture_with_inputs(
        Path::new(CPU_FIXTURE),
        &assets,
        &out,
        "sf-match-end-01",
        Some(Path::new(CPU_SCENARIO)),
        2389,
        6330,
    )
    .unwrap();
    let outcome = compare(
        &load_scenario(CPU_SCENARIO).unwrap(),
        &load_capture(CPU_A).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope(2390, 6330),
    )
    .unwrap();
    assert_match(outcome, 3941, 1331);

    let mut m =
        CpuMachine::from_fight_checkpoint(Path::new(CPU_FIXTURE), 2389, assets.maincpu).unwrap();
    while m.state.frame < 6331 {
        m.step();
    }
    assert!(m.unrecovered().is_empty());
    assert_eq!(
        &m.work_ram()[0x36..0x38],
        &[8, 1],
        "slot 3 spawned at 0x3f374 and sleeping at 0x3f3a2"
    );
    assert_eq!(m.work_ram()[3], 0);
    assert_eq!(m.work_ram()[0x26], 0, "slot 2 killed");
    assert_eq!(
        [2, 3, 4, 8, 9, 10].map(|i| m.work_ram()[0x4000 + i]),
        [0; 6]
    );
}

#[test]
fn versus_game_over_matches_both_captures_and_reports_attract_at_5903() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    assert_eq!(
        compare_versus_ram(
            Path::new(BASE_A),
            assets.maincpu.clone(),
            Path::new(BASE_SCENARIO),
            5559
        )
        .unwrap(),
        Ok(5559 - 2450)
    );
    for (scenario_path, reference, id, from, to, checkpoints) in [
        (BASE_SCENARIO, BASE_A, "sf-doubleko-01", 2450, 5559, 3110),
        (
            DOUBLEKO_SCENARIO,
            DOUBLEKO_A,
            "sf-doubleko-02",
            5560,
            5902,
            343,
        ),
    ] {
        let out = ScratchDir::new(&format!("{id}-game-over"));
        run_versus_interval_capture(
            Path::new(reference),
            &assets,
            &out,
            id,
            Path::new(scenario_path),
            to,
        )
        .unwrap();
        let outcome = compare(
            &load_scenario(scenario_path).unwrap(),
            &load_capture(reference).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &scope(from, to),
        )
        .unwrap();
        assert_match(outcome, (to - from + 1) as u64, checkpoints);
    }

    let owned: BTreeSet<u32> = versus_owned_addresses()
        .into_iter()
        .map(|(_, a)| a)
        .collect();
    let mut differences = Vec::new();
    let (frame, branches) = observe_versus_fight(
        Path::new(DOUBLEKO_A),
        assets.maincpu.clone(),
        Path::new(DOUBLEKO_SCENARIO),
        5903,
        |m| {
            if m.state.frame >= 5560 {
                differences.extend(work_differences(
                    Path::new(DOUBLEKO_A),
                    m.state.frame,
                    &versus_work_image(m),
                    &owned,
                ));
            }
        },
    )
    .unwrap();
    assert_eq!((frame, branches), (5903, vec![]));
    assert!(
        differences.is_empty(),
        "owned and non-stack RAM differed: {differences:#?}"
    );

    let mut m =
        VersusMachine::from_checkpoint_2450_with_program(Path::new(DOUBLEKO_A), assets.maincpu)
            .unwrap();
    let events = scenario_input_events(Path::new(DOUBLEKO_SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; sf_engine::run::INPUTS]);
    let mut scene_at_game_over = 0;
    while m.state.frame < 5903 {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        if m.state.frame == 5662 {
            scene_at_game_over = m.work_ram()[0x4008];
        }
    }
    assert!(m.unrecovered().is_empty());
    assert_eq!(
        &m.work_ram()[0x16..0x18],
        &[8, 1],
        "slot 1 sleeping at 0x3ccec"
    );
    assert_eq!(
        &m.work_ram()[0x36..0x38],
        &[8, 1],
        "slot 3 spawned at 0x3f374 and sleeping at 0x3f3a2"
    );
    assert_eq!(
        scene_at_game_over, 1,
        "P2's ranking check leaves scene byte 1"
    );
    let work = m.work_ram();
    assert_eq!(&work[0x6d2..0x6da], &[0; 8], "P1 score cleared");
    assert_eq!(&work[0x6fc..0x704], &[0; 8], "P2 score cleared");
    assert_eq!(
        &work[0xe5e..0xe62],
        &[0, 0xff, 0x86, 0xda],
        "P2 was checked second"
    );
    assert_eq!(work[3], 0);
    assert_eq!(work[0x26], 0, "slot 2 killed");
    assert_eq!([2, 3, 4, 8, 9, 10].map(|i| work[0x4000 + i]), [0; 6]);
}

#[test]
fn all_game_over_object_stream_calls_are_listed_and_translated() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = assets().maincpu;
    let long = |at: u32| {
        u32::from_be_bytes([
            rom[at as usize],
            rom[at as usize + 1],
            rom[at as usize + 2],
            rom[at as usize + 3],
        ])
    };
    let mut calls = BTreeSet::new();
    for entry in 0..11 {
        let mut visited = BTreeSet::new();
        let template = long(0x3f278 + entry * 8);
        let mut stream = long(template + 0x10);
        loop {
            assert!(
                visited.insert(stream),
                "effect stream loop at 0x{stream:05x}"
            );
            match rom[stream as usize] {
                0xfd => break,
                0xfe => stream = long(stream + 2),
                0xff => {
                    calls.insert(long(stream + 2));
                    stream += 6;
                }
                _ => stream += 8,
            }
        }
        assert_eq!(
            rom[template as usize + 0xe] & 0xc0,
            0,
            "absolute template must not need owner pointers"
        );
    }
    assert!(
        calls.is_empty(),
        "untranslated called routines reached: {calls:05x?}"
    );
}

#[test]
fn shortening_event_10_hold_by_one_frame_is_caught_immediately() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut bad_assets = assets();
    assert_eq!(bad_assets.maincpu[0x3b574..0x3b576], [0, 180]);
    bad_assets.maincpu[0x3b575] = 179;
    let outcome = compare_cpu_ram(Path::new(CPU_A), bad_assets.maincpu, 5014, 6330).unwrap();
    let CpuRamOutcome::Mismatch(message) = outcome else {
        panic!("expected the shortened hold to be caught, got {outcome:?}");
    };
    assert!(
        message.contains("frame 6150")
            && message.contains("0xff8027")
            && message.contains("reference b4, native b3"),
        "unexpected negative-control mismatch: {message}"
    );
}

#[test]
fn gameover_credit_01_repeats_exactly_and_equals_fight_run_a_through_1479() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(CREDIT_SCENARIO).unwrap();
    assert_eq!(scenario.duration_frames, 6401);
    assert_eq!(scenario.inputs.len(), 7);
    assert_eq!(scenario.checkpoints.len(), 1402);
    assert_eq!(scenario.checkpoints.first().copied(), Some(2389));
    assert_eq!(scenario.checkpoints.last().copied(), Some(6400));
    assert_eq!((scenario.dsw1, scenario.dsw2), (0xdfff, 0xffff));

    let outcome = compare(
        &scenario,
        &load_capture(CREDIT_A).unwrap(),
        &load_capture(CREDIT_B).unwrap(),
        CandidateKind::Reference,
        &scope(2389, 6400),
    )
    .unwrap();
    assert_match(outcome, 4012, 1402);

    let fight_rows = frame_rows(FIGHT_A);
    let credit_rows = frame_rows(CREDIT_A);
    for frame in 0..=1479 {
        assert_eq!(
            credit_rows.get(&frame),
            fight_rows.get(&frame),
            "fight-run-a baseline frame {frame}"
        );
    }
}

#[test]
fn game_over_with_credit_returns_to_start_screen_and_matches_through_6400() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    let owned: BTreeSet<u32> = cpu_owned_addresses().into_iter().map(|(_, a)| a).collect();
    let mut latches = Vec::new();
    let mut differences = Vec::new();
    let (frame, branches) = observe_cpu_fight(
        Path::new(CREDIT_A),
        assets.maincpu.clone(),
        Some(Path::new(CREDIT_SCENARIO)),
        2389,
        6400,
        |m| {
            let frame = m.state.frame;
            if frame >= 6070 {
                if m.state.sound_latch != 0xff {
                    latches.push((frame, m.state.sound_latch));
                }
                let palette =
                    std::fs::read(Path::new(CREDIT_A).join(format!("ram_{frame:06}_palette.bin")))
                        .unwrap();
                assert_eq!(
                    m.state.paletteram, palette,
                    "game-over/start-screen palette at frame {frame}"
                );
            }
            if frame >= 5000 {
                differences.extend(work_differences(
                    Path::new(CREDIT_A),
                    frame,
                    &cpu_work_image(m),
                    &owned,
                ));
            }
        },
    )
    .unwrap();
    assert_eq!((frame, branches), (6400, vec![]));
    assert!(
        differences.is_empty(),
        "owned and non-stack RAM differed: {differences:#?}"
    );
    assert_eq!(
        latches,
        [
            (6073, 0x3f),
            (6074, 0x1b),
            (6077, 0x1b),
            (6081, 0x1b),
            (6085, 0x1b),
            (6089, 0x1b),
            (6093, 0x1b),
            (6097, 0x1b),
            (6101, 0x1b),
            (6105, 0x1b),
            (6109, 0x1b),
            (6123, 0x95),
            (6304, 0x3f),
            (6305, 0x80),
        ]
    );

    let out = ScratchDir::new("sf-gameover-credit-01");
    run_cpu_fight_capture_with_inputs(
        Path::new(CREDIT_A),
        &assets,
        &out,
        "sf-gameover-credit-01",
        Some(Path::new(CREDIT_SCENARIO)),
        2389,
        6400,
    )
    .unwrap();
    let outcome = compare(
        &load_scenario(CREDIT_SCENARIO).unwrap(),
        &load_capture(CREDIT_A).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope(2390, 6400),
    )
    .unwrap();
    assert_match(outcome, 4011, 1401);

    let mut m =
        CpuMachine::from_fight_checkpoint(Path::new(CREDIT_A), 2389, assets.maincpu).unwrap();
    let events = scenario_input_events(Path::new(CREDIT_SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; sf_engine::run::INPUTS]);
    while m.state.frame < 6400 {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
    }
    assert!(
        m.unrecovered().is_empty(),
        "must not reach 0x3ccae: {:?}",
        m.unrecovered()
    );
    assert_eq!(
        m.work_ram()[3],
        0xff,
        "MODE_DEMO set to 0xff by start_screen_entry"
    );
    assert_eq!(
        m.work_ram()[0x26],
        8,
        "slot 2 remains sleeping (state 8, not killed to 0) when returning to start screen"
    );
    assert_eq!(
        &m.work_ram()[0x65a..0x65e],
        &[0, 0, 0, 1],
        "1 credit remains on start screen"
    );
}

#[test]
fn reading_zero_credits_at_0x3f358_fails_at_the_first_frame_after_the_branch() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut bad_assets = assets();
    assert_eq!(bad_assets.maincpu[0x3f35e..0x3f360], [0x06, 0x5a]);
    bad_assets.maincpu[0x3f35f] = 0x56;
    let outcome =
        compare_cpu_ram(Path::new(CREDIT_A), bad_assets.maincpu.clone(), 6300, 6400).unwrap();
    assert_eq!(
        outcome,
        CpuRamOutcome::Mismatch(
            "frame 6303: scheduler: vblank flag, current slot, frame counters differs first at 0xff8001: reference 07, native 01"
                .to_string(),
        ),
    );

    let mut m =
        CpuMachine::from_fight_checkpoint(Path::new(CREDIT_A), 6300, bad_assets.maincpu).unwrap();
    let owned: BTreeSet<u32> = cpu_owned_addresses().into_iter().map(|(_, a)| a).collect();
    while m.state.frame < 6302 {
        m.step();
        assert!(work_differences(
            Path::new(CREDIT_A),
            m.state.frame,
            &cpu_work_image(&m),
            &owned
        )
        .is_empty());
    }
    m.step();
    assert_eq!(m.state.frame, 6303);
    assert!(m.unrecovered().is_empty());
    let diffs = work_differences(Path::new(CREDIT_A), 6303, &cpu_work_image(&m), &owned);
    assert_eq!(
        diffs,
        vec![
            "frame 6303: 0xff8001: reference 0x07, native 0x01",
            "frame 6303: 0xff8026: reference 0x08, native 0x00",
            "frame 6303: 0xff8027: reference 0x01, native 0x00",
            "frame 6303: 0xff8077: reference 0x06, native 0x01",
            "frame 6303: 0xff8633: reference 0x00, native 0xff",
            "frame 6303: 0xff8636: reference 0xae, native 0xa2",
            "frame 6303: 0xff8638: reference 0x00, native 0x02",
            "frame 6303: 0xff8f23: reference 0xbe, native 0xd8",
        ]
    );
}
