//! Ticket #60 (F20): the Speed DIP's slow game in a fixed fight.
//! Local-only captures and ROMs; skips on a clean checkout.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;

use sf_compare::capture::{load_capture, Capture};
use sf_compare::scenario::load_scenario;

const FIXTURE: u32 = 2975;
const RESULT: u32 = 5652;
const SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_slow_01.json";
const RUN_A: &str = "../../tools/reference/local/captures/fixed-fight-slow-01-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/fixed-fight-slow-01-run-b";
const CAPTURE_END: u32 = 5660;
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn inputs_present() -> bool {
    Path::new(RUN_A).is_dir() && Path::new(ROMS).is_dir()
}

fn work_dump(frame: u32) -> Vec<u8> {
    std::fs::read(Path::new(RUN_A).join(format!("ram_{frame:06}_work.bin"))).unwrap()
}

/// C-NAT: the slow fight's observable output and work/object RAM match the
/// reference from its decodable fixture through the round-result jingle.
#[test]
fn native_slow_fixed_fight_matches_through_the_round_result() {
    if !inputs_present() {
        eprintln!("skipping: local slow fixed-fight capture/ROMs not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let fixture = sf_engine::cpu::CpuMachine::from_fight_checkpoint(
        Path::new(RUN_A),
        FIXTURE,
        assets.maincpu.clone(),
    )
    .unwrap();
    assert_eq!(fixture.state.frame, FIXTURE);

    let scenario = load_scenario(SCENARIO).unwrap();
    let reference = load_capture(RUN_A).unwrap();
    let out = sf_engine::run::ScratchDir::new("sf-engine-fixed-fight-slow");
    let executed = sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(RUN_A),
        &assets,
        &out,
        "sf-fixed-fight-slow-01",
        Some(Path::new(SCENARIO)),
        FIXTURE,
        RESULT,
    )
    .unwrap();
    assert_eq!(executed, (RESULT - FIXTURE + 1) as u64);

    let scope = sf_compare::compare::CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(RESULT as u64),
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
                if frames_compared == (RESULT - FIXTURE) as u64
                    && checkpoints_compared == (RESULT - FIXTURE) as usize
        ),
        "native slow fixed fight: {outcome:?}"
    );

    let owned = sf_engine::run::cpu_owned_addresses();
    let (stopped, unrecovered) = sf_engine::run::observe_cpu_fight(
        Path::new(RUN_A),
        assets.maincpu.clone(),
        Some(Path::new(SCENARIO)),
        FIXTURE,
        RESULT,
        |machine| {
            let frame = machine.state.frame;
            let image = sf_engine::run::cpu_work_image(machine);
            let dump = work_dump(frame);
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
        },
    )
    .unwrap();
    assert_eq!(
        (stopped, unrecovered),
        (RESULT, vec![]),
        "no untranslated branch through the round result"
    );
}

fn captures_present() -> bool {
    Path::new(RUN_A).is_dir() && Path::new(RUN_B).is_dir()
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

fn latch_at(capture: &Capture, frame: u32) -> (String, bool) {
    let frame = capture
        .frames
        .iter()
        .find(|candidate| candidate.frame == frame as u64)
        .unwrap();
    (frame.latch.clone(), frame.latch_changed)
}

/// C-REF: both reference runs agree byte-for-byte. Their independently
/// observed state pins the fixture, timer expiry and round-result boundaries.
#[test]
fn slow_fixed_fight_reference_runs_and_boundaries() {
    if !captures_present() {
        eprintln!("skipping: local slow fixed-fight captures not present");
        return;
    }

    let scenario = load_scenario(SCENARIO).unwrap();
    assert_eq!(scenario.scenario_id, "sf-fixed-fight-slow-01");
    assert_eq!((scenario.dsw1, scenario.dsw2), (0xcfff, 0xffff));
    assert_eq!(scenario.duration_frames, CAPTURE_END as u64 + 1);
    assert_eq!(
        scenario.checkpoints,
        (0..=830)
            .chain(FIXTURE as u64..=CAPTURE_END as u64)
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
            (2986, "p1_right", 180),
            (3176, "p1_button1", 10),
            (3216, "p1_button4", 10),
        ]
    );

    let ref_a = load_capture(RUN_A).unwrap();
    let ref_b = load_capture(RUN_B).unwrap();
    for reference in [&ref_a, &ref_b] {
        assert_eq!(
            (reference.meta.dsw1, reference.meta.dsw2),
            (Some(0xcfff), Some(0xffff))
        );
        assert_eq!(reference.meta.frames_executed, CAPTURE_END as u64 + 1);
    }
    let files_a = observation_files(RUN_A);
    let files_b = observation_files(RUN_B);
    assert_eq!(files_a, files_b);
    assert_eq!(files_a.len(), 17_588);
    for name in files_a {
        let a = std::fs::read(Path::new(RUN_A).join(&name)).unwrap();
        let b = std::fs::read(Path::new(RUN_B).join(&name)).unwrap();
        assert_eq!(a, b, "run A/B mismatch in {name:?}");
    }

    assert_eq!(work_dump(FIXTURE)[0x4008], 0x80);
    assert_eq!(&work_dump(FIXTURE)[0x0e18..0x0e1b], &[1, 0, 0]);
    assert_eq!(&work_dump(2988)[0x0e18..0x0e1b], &[1, 0, 0]);
    assert_eq!(&work_dump(2989)[0x0e18..0x0e1b], &[0, 9, 9]);
    assert_eq!(&work_dump(4869)[0x0e18..0x0e1b], &[0, 0, 1]);
    assert_eq!(&work_dump(4870)[0x0e18..0x0e1b], &[0, 0, 0]);
    assert_eq!(work_dump(4876)[0x4008], 0x80);
    assert_eq!(work_dump(4877)[0x4008], 0);
    assert_eq!(work_dump(5650)[0x4008], 0);
    assert_eq!(work_dump(5650)[0x0e36], 5);
    assert_eq!(work_dump(5651)[0x4008], 0x20);
    assert_eq!(work_dump(5651)[0x0e36], 7);
    for (frame, command) in [(2796, "94"), (4878, "96"), (RESULT, "a7")] {
        for reference in [&ref_a, &ref_b] {
            assert_eq!(latch_at(reference, frame), (command.to_string(), true));
        }
    }
}

/// C-NEG: starting from the same fixture and inputs, changing only Speed to
/// Normal must diverge from the slow reference on the next game-flow wake.
#[test]
fn speed_normal_diverges_from_the_slow_reference() {
    if !inputs_present() {
        eprintln!("skipping: local slow fixed-fight capture/ROMs not present");
        return;
    }

    let scratch = sf_engine::run::ScratchDir::new("sf-engine-speed-normal-negative");
    std::fs::create_dir_all(&scratch).unwrap();
    let normal_path = scratch.join("speed-normal.json");
    let normal_json = std::fs::read_to_string(SCENARIO)
        .unwrap()
        .replace("\"Speed\": \"Slow\"", "\"Speed\": \"Normal\"");
    assert_ne!(normal_json, std::fs::read_to_string(SCENARIO).unwrap());
    std::fs::write(&normal_path, normal_json).unwrap();
    let normal = load_scenario(&normal_path).unwrap();
    assert_eq!((normal.dsw1, normal.dsw2), (0xdfff, 0xffff));

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = sf_engine::run::ScratchDir::new("sf-engine-speed-normal-capture");
    let last = FIXTURE + 25;
    let executed = sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(RUN_A),
        &assets,
        &out,
        "sf-fixed-fight-slow-01",
        Some(&normal_path),
        FIXTURE,
        last,
    )
    .unwrap();
    assert_eq!(executed, (last - FIXTURE + 1) as u64);
    let scope = sf_compare::compare::CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(last as u64),
        fields: Some(
            sf_compare::compare::CompareScope::parse_fields("screen_hash,latch,latch_changed")
                .unwrap(),
        ),
    };
    // The slow capture supplies observations, not this variant's identity.
    // Re-label only its in-memory readback so the public comparator accepts
    // the intentional DIP perturbation; every expected observation is intact.
    let mut reference = load_capture(RUN_A).unwrap();
    reference.meta.dsw1 = Some(normal.dsw1);
    reference.meta.dsw2 = Some(normal.dsw2);
    let outcome = sf_compare::compare::compare(
        &normal,
        &reference,
        &load_capture(&out).unwrap(),
        sf_compare::compare::CandidateKind::Native,
        &scope,
    )
    .unwrap();
    let sf_compare::compare::Outcome::Mismatch(diff) = outcome else {
        panic!("Speed Normal unexpectedly matched the slow reference");
    };
    assert_eq!(diff.frame, 2982, "first Speed Normal divergence: {diff:?}");
}
