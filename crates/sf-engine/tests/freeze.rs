//! Ticket #61 (F21): the Freeze DIP in a fixed fight.
//! Local-only captures and ROMs; skips on a clean checkout.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;

use sf_engine::run::CpuRamOutcome;

const FIXTURE: u32 = 2389;
const DECISION: u32 = 5175;
const CAPTURE_LAST: u32 = 5180;
const SCENARIO: &str = "../../tools/reference/scenarios/sf_freeze_01.json";
const RUN_A: &str = "../../tools/reference/local/captures/freeze-01-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/freeze-01-run-b";
const AUDIO_PROBE: &str = "../../tools/reference/local/captures/freeze-audio-probe/audio_taps.tsv";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn captures_present() -> bool {
    Path::new(RUN_A).is_dir() && Path::new(RUN_B).is_dir()
}

fn inputs_present() -> bool {
    captures_present() && Path::new(ROMS).is_dir()
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

fn ram_dump(capture: &str, frame: u32, region: &str) -> Vec<u8> {
    std::fs::read(Path::new(capture).join(format!("ram_{frame:06}_{region}.bin"))).unwrap()
}

/// C-REF: two identical captures prove the DIP schedule, frozen RAM window,
/// held-vblank release and the match decision used by the native boundary.
#[test]
fn freeze_reference_runs_and_frame_windows() {
    if !captures_present() {
        eprintln!("skipping: local Freeze captures not present");
        return;
    }

    let scenario = sf_compare::scenario::load_scenario(SCENARIO).unwrap();
    assert_eq!(scenario.scenario_id, "sf-freeze-01");
    assert_eq!((scenario.dsw1, scenario.dsw2), (0xdfff, 0xffff));
    assert_eq!(scenario.duration_frames, CAPTURE_LAST as u64 + 1);
    assert_eq!(
        scenario.checkpoints,
        (0..=830)
            .chain(FIXTURE as u64..=CAPTURE_LAST as u64)
            .collect::<Vec<_>>()
    );
    let inputs: Vec<_> = scenario
        .inputs
        .iter()
        .map(|event| (event.frame, event.press.as_str(), event.hold_frames))
        .collect();
    assert_eq!(
        inputs,
        [
            (1450, "coin1", 12),
            (1520, "start1", 12),
            (1900, "start1", 12),
            (2400, "p1_right", 180),
            (2590, "p1_button1", 10),
            (2630, "p1_button4", 10),
        ]
    );
    let dip_changes: Vec<_> = scenario
        .dip_changes
        .iter()
        .map(|change| (change.frame, change.dsw1, change.dsw2))
        .collect();
    assert_eq!(
        dip_changes,
        [(2500, 0x9fff, 0xffff), (2560, 0xdfff, 0xffff)]
    );

    let expected_readbacks = "change_frame\treadback_frame\tdsw1\tdsw2\n\
                              2500\t2501\t9fff\tffff\n\
                              2560\t2561\tdfff\tffff\n";
    for capture in [RUN_A, RUN_B] {
        let reference = sf_compare::capture::load_capture(capture).unwrap();
        assert_eq!(
            (reference.meta.dsw1, reference.meta.dsw2),
            (Some(0xdfff), Some(0xffff))
        );
        assert_eq!(reference.meta.frames_executed, CAPTURE_LAST as u64 + 1);
        assert_eq!(
            std::fs::read_to_string(Path::new(capture).join("dip_readbacks.tsv")).unwrap(),
            expected_readbacks
        );
    }

    let files_a = observation_files(RUN_A);
    let files_b = observation_files(RUN_B);
    assert_eq!(files_a, files_b);
    assert_eq!(files_a.len(), 18_119);
    for name in files_a {
        assert_eq!(
            std::fs::read(Path::new(RUN_A).join(&name)).unwrap(),
            std::fs::read(Path::new(RUN_B).join(&name)).unwrap(),
            "run A/B mismatch in {name:?}"
        );
    }

    let changed_bytes = |before: u32, after: u32, region: &str| {
        ram_dump(RUN_A, before, region)
            .iter()
            .zip(ram_dump(RUN_A, after, region).iter())
            .filter(|(a, b)| a != b)
            .count()
    };
    assert_eq!(changed_bytes(2500, 2501, "work"), 10);
    assert_eq!(changed_bytes(2560, 2561, "work"), 67);
    for frame in 2501..=2560 {
        for region in ["work", "video", "palette"] {
            assert_eq!(
                ram_dump(RUN_A, 2501, region),
                ram_dump(RUN_A, frame, region),
                "frozen {region} RAM at frame {frame}"
            );
        }
    }
    for frame in FIXTURE..=DECISION {
        let winner = ram_dump(RUN_A, frame, "work")[0x4009];
        assert_eq!(
            winner != 0,
            frame == DECISION,
            "first nonzero match-winner byte at frame {frame}"
        );
    }
}

/// C-NAT: the interrupted vblank prefix runs on entry, then game state stays
/// frozen on the next frame instead of running the suffix and scheduler.
#[test]
fn native_freeze_entry_and_frozen_frame_match_reference() {
    if !inputs_present() {
        eprintln!("skipping: local Freeze capture/ROMs not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let last = 2502;
    let outcome = sf_engine::run::compare_cpu_ram_with_inputs(
        Path::new(RUN_A),
        assets.maincpu,
        Some(Path::new(SCENARIO)),
        FIXTURE,
        last,
    )
    .unwrap();
    assert_eq!(
        outcome,
        CpuRamOutcome::Match {
            frames: last - FIXTURE
        }
    );
}

/// C-NAT: release finishes the interrupted handler and runs the one held
/// vblank before the scheduler, matching the first post-release game frame.
#[test]
fn native_freeze_release_matches_reference() {
    if !inputs_present() {
        eprintln!("skipping: local Freeze capture/ROMs not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let last = 2562;
    let outcome = sf_engine::run::compare_cpu_ram_with_inputs(
        Path::new(RUN_A),
        assets.maincpu,
        Some(Path::new(SCENARIO)),
        FIXTURE,
        last,
    )
    .unwrap();
    assert_eq!(
        outcome,
        CpuRamOutcome::Match {
            frames: last - FIXTURE
        }
    );
}

/// The release's two native latch writes have the same values and separation
/// as the reference tap; the interval is derived from the original code path.
#[test]
fn native_release_latch_writes_match_the_tap_interval() {
    if !inputs_present() || !Path::new(AUDIO_PROBE).is_file() {
        eprintln!("skipping: local Freeze capture/audio probe/ROMs not present");
        return;
    }

    let tap_text = std::fs::read_to_string(AUDIO_PROBE).unwrap();
    let taps: Vec<_> = tap_text
        .lines()
        .filter_map(|line| {
            let cells: Vec<_> = line.split('\t').collect();
            (cells.get(1) == Some(&"2561") && cells.get(2) == Some(&"latch"))
                .then(|| (cells[0], cells[4]))
        })
        .collect();
    assert_eq!(taps, [("42.683355375", "ffff"), ("42.683650375", "ffff")]);
    let times: Vec<f64> = taps.iter().map(|(time, _)| time.parse().unwrap()).collect();
    let expected_gap = ((times[1] - times[0]) * 8_000_000.0).round() as u32;
    assert_eq!(expected_gap, 2_360);

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let mut release_writes = None;
    let (stopped, unrecovered) = sf_engine::run::observe_cpu_fight(
        Path::new(RUN_A),
        assets.maincpu,
        Some(Path::new(SCENARIO)),
        FIXTURE,
        2561,
        |machine| {
            if machine.state.frame == 2561 {
                release_writes = Some(machine.latch_writes().to_vec());
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (2561, vec![]));
    let writes = release_writes.unwrap();
    assert_eq!(writes.len(), 2, "interrupted suffix plus one held vblank");
    assert_eq!(
        writes.iter().map(|write| write.value).collect::<Vec<_>>(),
        [0xff, 0xff]
    );
    assert_eq!(writes[1].cycles - writes[0].cycles, expected_gap);
}

/// C-NAT: the whole fixed fight matches from the decodable fixture through
/// the match decision, including frozen state and the held-vblank release.
#[test]
fn native_freeze_matches_through_match_decision() {
    if !inputs_present() {
        eprintln!("skipping: local Freeze capture/ROMs not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let owned = sf_engine::run::cpu_owned_addresses();
    let fixture = sf_engine::cpu::CpuMachine::from_fight_checkpoint(
        Path::new(RUN_A),
        FIXTURE,
        assets.maincpu.clone(),
    )
    .unwrap();
    assert_eq!(fixture.state.frame, FIXTURE);

    let scenario = sf_compare::scenario::load_scenario(SCENARIO).unwrap();
    let reference = sf_compare::capture::load_capture(RUN_A).unwrap();
    let out = sf_engine::run::ScratchDir::new("sf-engine-sf-freeze-01");
    let executed = sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(RUN_A),
        &assets,
        &out,
        "sf-freeze-01",
        Some(Path::new(SCENARIO)),
        FIXTURE,
        DECISION,
    )
    .unwrap();
    assert_eq!(executed, 2_787);

    let scope = sf_compare::compare::CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(DECISION as u64),
        fields: Some(
            sf_compare::compare::CompareScope::parse_fields("screen_hash,latch,latch_changed")
                .unwrap(),
        ),
    };
    let outcome = sf_compare::compare::compare(
        &scenario,
        &reference,
        &sf_compare::capture::load_capture(&out).unwrap(),
        sf_compare::compare::CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            sf_compare::compare::Outcome::Match {
                frames_compared: 2_786,
                checkpoints_compared: 2_786,
            }
        ),
        "native comparison through decision {DECISION}: {outcome:?}"
    );

    let (stopped, unrecovered) = sf_engine::run::observe_cpu_fight(
        Path::new(RUN_A),
        assets.maincpu,
        Some(Path::new(SCENARIO)),
        FIXTURE,
        DECISION,
        |machine| {
            let frame = machine.state.frame;
            let image = sf_engine::run::cpu_work_image(machine);
            let dump =
                std::fs::read(Path::new(RUN_A).join(format!("ram_{frame:06}_work.bin"))).unwrap();
            for &(name, addr) in &owned {
                let i = (addr - 0xff8000) as usize;
                assert_eq!(dump[i], image[i], "frame {frame}: {name} at 0x{addr:06x}");
            }
            for (i, (&reference_byte, &native_byte)) in dump.iter().zip(image.iter()).enumerate() {
                let addr = 0xff8000 + i as u32;
                let in_slot_saved =
                    (0xff8006..=0xff8105).contains(&addr) && (addr - 0xff8006) % 16 >= 2;
                let in_stack = (0xff8106..=0xff8631).contains(&addr);
                if !in_slot_saved && !in_stack {
                    assert_eq!(
                        reference_byte, native_byte,
                        "frame {frame}: non-slot/non-stack work/object RAM at 0x{addr:06x}"
                    );
                }
            }

            let expected_writes = match frame {
                2501..=2560 => 0,
                2561 => 2,
                _ => 1,
            };
            assert_eq!(
                machine.latch_writes().len(),
                expected_writes,
                "frame {frame}: latch write count"
            );
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (DECISION, vec![]));
}

/// C-NEG: releasing Freeze one frame early must diverge on the first frame
/// where the reference is still frozen, rather than merely at the capture end.
#[test]
fn early_freeze_release_is_caught_at_the_first_differing_frame() {
    if !inputs_present() {
        eprintln!("skipping: local Freeze capture/ROMs not present");
        return;
    }

    let scratch = sf_engine::run::ScratchDir::new("sf-engine-neg-freeze-release");
    std::fs::create_dir_all(&scratch).unwrap();
    let raw = std::fs::read_to_string(SCENARIO).unwrap();
    let mut shifted: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let changes = shifted["dip_changes"].as_array_mut().unwrap();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[1]["frame"], 2560);
    assert_eq!(changes[1]["dipswitches"]["Freeze"], "Off");
    changes[1]["frame"] = serde_json::Value::from(2559);
    let scenario_path = scratch.as_ref().join("sf_freeze_01_off_2559.json");
    std::fs::write(&scenario_path, serde_json::to_vec_pretty(&shifted).unwrap()).unwrap();

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let outcome = sf_engine::run::compare_cpu_ram_with_inputs(
        Path::new(RUN_A),
        assets.maincpu,
        Some(&scenario_path),
        FIXTURE,
        2562,
    )
    .unwrap();
    let CpuRamOutcome::Mismatch(message) = outcome else {
        panic!("expected early release to mismatch, got {outcome:?}");
    };
    assert!(
        message.starts_with("frame 2560:"),
        "expected first mismatch while reference remains frozen: {message}"
    );
}
