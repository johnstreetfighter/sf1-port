//! Ticket #59 (F19): difficulty, round-time and attract-music DIPs in the
//! fixed fight. Local-only captures and ROMs; skips on a clean checkout.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;

use sf_compare::capture::{load_capture, Capture};
use sf_compare::scenario::load_scenario;

const FIXTURE: u32 = 2389;
const AUDIT_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_audit_01.json";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    scenario: &'static str,
    run_a: &'static str,
    run_b: &'static str,
    dsw1: u16,
    dsw2: u16,
    difficulty: Option<u8>,
    time: Option<[u8; 3]>,
    attract_music_off: bool,
    decision: u32,
    end: u32,
}

const CASES: [Case; 4] = [
    Case {
        id: "sf-fixed-fight-dsw-01",
        scenario: "../../tools/reference/scenarios/sf_fixed_fight_dsw_01.json",
        run_a: "../../tools/reference/local/captures/fixed-fight-dsw-01-run-a",
        run_b: "../../tools/reference/local/captures/fixed-fight-dsw-01-run-b",
        dsw1: 0xdfff,
        dsw2: 0xffd7,
        difficulty: Some(1),
        time: Some([1, 5, 0]),
        attract_music_off: false,
        decision: 5100,
        end: 5101,
    },
    Case {
        id: "sf-fixed-fight-dsw-02",
        scenario: "../../tools/reference/scenarios/sf_fixed_fight_dsw_02.json",
        run_a: "../../tools/reference/local/captures/fixed-fight-dsw-02-run-a",
        run_b: "../../tools/reference/local/captures/fixed-fight-dsw-02-run-b",
        dsw1: 0xdfff,
        dsw2: 0xffaf,
        difficulty: Some(2),
        time: Some([2, 0, 0]),
        attract_music_off: false,
        decision: 4768,
        end: 4800,
    },
    Case {
        id: "sf-fixed-fight-dsw-03",
        scenario: "../../tools/reference/scenarios/sf_fixed_fight_dsw_03.json",
        run_a: "../../tools/reference/local/captures/fixed-fight-dsw-03-run-a",
        run_b: "../../tools/reference/local/captures/fixed-fight-dsw-03-run-b",
        dsw1: 0xdfff,
        dsw2: 0xff87,
        difficulty: Some(3),
        time: Some([2, 5, 0]),
        attract_music_off: false,
        decision: 4647,
        end: 4700,
    },
    Case {
        id: "sf-fixed-fight-dsw-04",
        scenario: "../../tools/reference/scenarios/sf_fixed_fight_dsw_04.json",
        run_a: "../../tools/reference/local/captures/fixed-fight-dsw-04-run-a",
        run_b: "../../tools/reference/local/captures/fixed-fight-dsw-04-run-b",
        dsw1: 0xddff,
        dsw2: 0xffff,
        difficulty: None,
        time: None,
        attract_music_off: true,
        decision: 5015,
        end: 5100,
    },
];

fn captures_present() -> bool {
    CASES
        .iter()
        .all(|case| Path::new(case.run_a).is_dir() && Path::new(case.run_b).is_dir())
}

fn native_inputs_present() -> bool {
    captures_present() && Path::new(ROMS).is_dir()
}

fn work_dump(capture: &str, frame: u32) -> Vec<u8> {
    std::fs::read(Path::new(capture).join(format!("ram_{frame:06}_work.bin"))).unwrap()
}

fn observation_files(capture: &str) -> BTreeSet<OsString> {
    std::fs::read_dir(capture)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| {
            let name = name.to_string_lossy();
            name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba")
        })
        .collect()
}

fn latch_stream(capture: &Capture, first: u32, last: u32) -> Vec<(u64, String, bool)> {
    capture
        .frames
        .iter()
        .filter(|frame| (first as u64..=last as u64).contains(&frame.frame))
        .map(|frame| (frame.frame, frame.latch.clone(), frame.latch_changed))
        .collect()
}

fn assert_no_attract_music(stream: &[(u64, String, bool)], label: &str) {
    for (frame, latch, _) in stream {
        let command = u8::from_str_radix(latch, 16).unwrap();
        assert!(
            !(0x28..=0x3e).contains(&command),
            "{label}: filtered command {command:#04x} at frame {frame}"
        );
    }
}

/// C-REF: each scenario preserves the fixed fight's six inputs, has the
/// requested DIP readback and a full checkpoint window through its observed
/// decision, and its two runs are identical on every observation file.
#[test]
fn fixed_fight_dip_reference_runs_and_decision_windows() {
    if !captures_present() {
        eprintln!("skipping: local fixed-fight DIP captures not present");
        return;
    }

    let audit = load_scenario(AUDIT_SCENARIO).unwrap();
    let audit_inputs: Vec<_> = audit
        .inputs
        .iter()
        .map(|event| (event.frame, event.press.as_str(), event.hold_frames))
        .collect();

    for case in CASES {
        let scenario = load_scenario(case.scenario).unwrap();
        assert_eq!(scenario.scenario_id, case.id);
        assert_eq!(
            (scenario.dsw1, scenario.dsw2),
            (case.dsw1, case.dsw2),
            "{}",
            case.id
        );
        assert_eq!(scenario.duration_frames, case.end as u64 + 1, "{}", case.id);
        assert!(
            case.end > case.decision,
            "{} must run past its decision",
            case.id
        );
        assert_eq!(
            scenario.checkpoints,
            (0..=830)
                .chain(FIXTURE as u64..=case.end as u64)
                .collect::<Vec<_>>(),
            "{} checkpoint window",
            case.id
        );
        let inputs: Vec<_> = scenario
            .inputs
            .iter()
            .map(|event| (event.frame, event.press.as_str(), event.hold_frames))
            .collect();
        assert_eq!(
            inputs, audit_inputs,
            "{} must preserve the fixed fight inputs",
            case.id
        );

        let ref_a = load_capture(case.run_a).unwrap();
        let ref_b = load_capture(case.run_b).unwrap();
        for reference in [&ref_a, &ref_b] {
            assert_eq!(
                (reference.meta.dsw1, reference.meta.dsw2),
                (Some(case.dsw1), Some(case.dsw2)),
                "{} DIP readback",
                case.id
            );
            assert_eq!(
                reference.meta.frames_executed,
                case.end as u64 + 1,
                "{}",
                case.id
            );
        }

        let files_a = observation_files(case.run_a);
        let files_b = observation_files(case.run_b);
        assert_eq!(files_a, files_b, "{} observation file sets", case.id);
        assert_eq!(
            files_a.len(),
            (case.end - FIXTURE + 1 + 831) as usize * 5 + 3,
            "{}: five files per checkpoint plus three TSVs",
            case.id
        );
        for name in files_a {
            let a = std::fs::read(Path::new(case.run_a).join(&name)).unwrap();
            let b = std::fs::read(Path::new(case.run_b).join(&name)).unwrap();
            assert_eq!(a, b, "{} run A/B mismatch in {name:?}", case.id);
        }

        for frame in FIXTURE..=case.decision {
            let work = work_dump(case.run_a, frame);
            assert_eq!(
                work[0x4009] != 0,
                frame == case.decision,
                "{}: first nonzero match-winner byte must be the recorded decision {}/{}",
                case.id,
                frame,
                case.decision
            );
        }

        if case.attract_music_off {
            let stream_a = latch_stream(&ref_a, 0, case.end);
            let stream_b = latch_stream(&ref_b, 0, case.end);
            assert_eq!(
                stream_a, stream_b,
                "attract-music-off reference latch streams"
            );
            assert_no_attract_music(&stream_a, "attract-music-off reference run A");
            assert_no_attract_music(&stream_b, "attract-music-off reference run B");
        }
    }
}

const TABLES: std::ops::Range<usize> = 0x4058..0x4664;
const CPU_MOVES: std::ops::Range<usize> = 0x4676..0x467a;

/// Retsu's tables at each fixture are reproduced by the public AI-table
/// setup from the selected difficulty's ROM rows. Easy must not accidentally
/// reuse the default Normal rows.
#[test]
fn retsu_ai_tables_match_each_difficultys_rom_rows() {
    if !native_inputs_present() {
        eprintln!("skipping: local fixed-fight DIP captures/ROMs not present");
        return;
    }

    let program = sf_video::romset::load_assets(ROMS, MANIFEST)
        .unwrap()
        .maincpu;
    let mut images = Vec::new();
    for case in CASES {
        let reference = work_dump(case.run_a, FIXTURE);
        let difficulty = case.difficulty.unwrap_or(0);
        assert_eq!(
            (reference[0x4003], reference[0x4004]),
            (0, 0),
            "{}: Retsu",
            case.id
        );
        assert_eq!(
            reference[0x4007], difficulty,
            "{}: decoded difficulty",
            case.id
        );

        let mut wiped = reference.clone();
        wiped[TABLES].fill(0);
        wiped[CPU_MOVES].fill(0);
        let (native, _) = sf_engine::run::cpu_opponent_setup(
            program.clone(),
            &wiped,
            reference[0x4003],
            reference[0x4004],
            difficulty,
            reference[0x4006],
        )
        .unwrap();
        assert_eq!(native[TABLES], reference[TABLES], "{}: AI tables", case.id);
        assert_eq!(
            native[CPU_MOVES], reference[CPU_MOVES],
            "{}: move-table pointer",
            case.id
        );
        images.push(native[TABLES].to_vec());
    }
    assert_ne!(
        images[0], images[3],
        "Easy and Normal must select different Retsu ROM rows"
    );
}

/// C-NAT from each decodable 2389 fixture through its match decision. Every
/// frame matches the comparison boundary; every checkpoint matches pixels,
/// video and palette RAM; and all work/object RAM matches except the task
/// slots' saved CPU state and stacks. Also checks native TIME reloads and the
/// attract-music-off native/reference latch streams.
#[test]
fn native_fixed_fight_dips_match_through_each_match_decision() {
    if !native_inputs_present() {
        eprintln!("skipping: local fixed-fight DIP captures/ROMs not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let owned = sf_engine::run::cpu_owned_addresses();
    for case in CASES {
        // This is the fixture decoder acceptance, not just a file-existence
        // check: an inconsistent 2389 checkpoint is rejected here.
        let fixture = sf_engine::cpu::CpuMachine::from_fight_checkpoint(
            Path::new(case.run_a),
            FIXTURE,
            assets.maincpu.clone(),
        )
        .unwrap();
        assert_eq!(fixture.state.frame, FIXTURE, "{} fixture", case.id);

        let scenario = load_scenario(case.scenario).unwrap();
        let reference = load_capture(case.run_a).unwrap();
        let out = sf_engine::run::ScratchDir::new(&format!("sf-engine-{}", case.id));
        let executed = sf_engine::run::run_cpu_fight_capture_with_inputs(
            Path::new(case.run_a),
            &assets,
            &out,
            case.id,
            Some(Path::new(case.scenario)),
            FIXTURE,
            case.decision,
        )
        .unwrap();
        assert_eq!(
            executed,
            (case.decision - FIXTURE + 1) as u64,
            "{}",
            case.id
        );

        let scope = sf_compare::compare::CompareScope {
            from: Some((FIXTURE + 1) as u64),
            to: Some(case.decision as u64),
            fields: Some(
                sf_compare::compare::CompareScope::parse_fields("screen_hash,latch,latch_changed")
                    .unwrap(),
            ),
        };
        let outcome = sf_compare::compare::compare(
            &scenario,
            &reference,
            &load_capture(&out).unwrap(),
            sf_compare::compare::CandidateKind::Native,
            &scope,
        )
        .unwrap();
        assert!(
            matches!(
                outcome,
                sf_compare::compare::Outcome::Match { frames_compared, checkpoints_compared }
                    if frames_compared == (case.decision - FIXTURE) as u64
                        && checkpoints_compared == (case.decision - FIXTURE) as usize
            ),
            "{}: native comparison through decision {}: {outcome:?}",
            case.id,
            case.decision
        );

        if let Some(expected) = case.time {
            let reference_2390 = work_dump(case.run_a, FIXTURE + 1);
            assert_eq!(
                &reference_2390[0x0e18..0x0e1b],
                &expected,
                "{}: reference TIME reload",
                case.id
            );
        }

        let mut native_latches = Vec::new();
        let (stopped, unrecovered) = sf_engine::run::observe_cpu_fight(
            Path::new(case.run_a),
            assets.maincpu.clone(),
            Some(Path::new(case.scenario)),
            FIXTURE,
            case.decision,
            |machine| {
                let frame = machine.state.frame;
                let image = sf_engine::run::cpu_work_image(machine);
                let dump = work_dump(case.run_a, frame);
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    assert_eq!(
                        dump[i], image[i],
                        "{} frame {frame}: {name} at 0x{addr:06x}",
                        case.id
                    );
                }
                for (i, (&reference_byte, &native_byte)) in
                    dump.iter().zip(image.iter()).enumerate()
                {
                    let addr = 0xff8000 + i as u32;
                    let in_slot_saved =
                        (0xff8006..=0xff8105).contains(&addr) && (addr - 0xff8006) % 16 >= 2;
                    let in_stack = (0xff8106..=0xff8631).contains(&addr);
                    if !in_slot_saved && !in_stack {
                        assert_eq!(
                            reference_byte, native_byte,
                            "{} frame {frame}: non-slot/non-stack work/object RAM at 0x{addr:06x}",
                            case.id
                        );
                    }
                }
                if let Some(expected) = case.time {
                    if frame == FIXTURE + 1 {
                        assert_eq!(
                            &image[0x0e18..0x0e1b],
                            &expected,
                            "{}: native TIME reload",
                            case.id
                        );
                    }
                }
                if case.attract_music_off {
                    native_latches.push((frame as u64, machine.state.sound_latch));
                }
            },
        )
        .unwrap();
        assert_eq!(
            (stopped, unrecovered),
            (case.decision, vec![]),
            "{}: no untranslated branch through its decision",
            case.id
        );

        if case.attract_music_off {
            let expected: Vec<_> = latch_stream(&reference, FIXTURE + 1, case.decision)
                .into_iter()
                .map(|(frame, latch, _)| (frame, u8::from_str_radix(&latch, 16).unwrap()))
                .collect();
            assert_eq!(native_latches, expected, "native/reference latch streams");
            let native_text: Vec<_> = native_latches
                .iter()
                .map(|(frame, latch)| (*frame, format!("{latch:02x}"), false))
                .collect();
            assert_no_attract_music(&native_text, "attract-music-off native run");
        }
    }
}
