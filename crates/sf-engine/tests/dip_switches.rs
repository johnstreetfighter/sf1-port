//! Tickets #42 (F2) & #57 (F17): DIP switch settings in reference scenarios,
//! the native inputs, and the full boot DIP decode matrix from power-on across
//! every defined MAME setting of Coin A, Coin B, Game Continuation, Round Time
//! Count, Difficulty, Buy-In Feature, Number of Countries Selected, Flip
//! Screen, and Service Mode.
//!
//! Evidence: docs/reference/dip-switches.md, docs/reference/power-on.md.
//! Local-only reference data; skips on a clean checkout.

#![allow(clippy::type_complexity)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareError, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{
    compare_fight_ram_with_inputs, cpu_owned_addresses, observe_cpu_fight,
    observe_power_on_scenario, observe_power_on_with_io, run_fight_capture,
    run_power_on_scenario_capture, run_power_on_scenario_capture_for_reference, scenario_dsw,
    CpuRamOutcome, PowerOnMachine, ScratchDir, BOOT_SETUP_LAST,
};
use sf_engine::tasks::Io;

const SMOKE_01_SCENARIO: &str = "../../tools/reference/scenarios/sf_dip_smoke_01.json";
const SMOKE_02_SCENARIO: &str = "../../tools/reference/scenarios/sf_dip_smoke_02.json";
const SMOKE_03_SCENARIO: &str = "../../tools/reference/scenarios/sf_dip_smoke_03.json";

const SMOKE_01_A: &str = "../../tools/reference/local/captures/dip-smoke-01-run-a";
const SMOKE_01_B: &str = "../../tools/reference/local/captures/dip-smoke-01-run-b";
const SMOKE_02_A: &str = "../../tools/reference/local/captures/dip-smoke-02-run-a";
const SMOKE_02_B: &str = "../../tools/reference/local/captures/dip-smoke-02-run-b";
const SMOKE_03_A: &str = "../../tools/reference/local/captures/dip-smoke-03-run-a";
const SMOKE_03_B: &str = "../../tools/reference/local/captures/dip-smoke-03-run-b";

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// First frame end at which the service-mode test screen shows in video RAM
/// (0x800000..0x800fff holds a word other than 0x103c after holding only
/// 0x103c at frames 16..=19). Recorded for F73 (#73).
pub const SERVICE_TEST_SCREEN_FRAME: u32 = 20;

fn present() -> bool {
    [
        SMOKE_01_A, SMOKE_01_B, SMOKE_02_A, SMOKE_02_B, SMOKE_03_A, SMOKE_03_B, ROMS,
    ]
    .iter()
    .all(|p| Path::new(p).is_dir())
}

fn assert_runs_identical_with_count(dir_a: &str, dir_b: &str, expected_files: usize) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir_a)
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
        let b = Path::new(dir_b).join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: {dir_a} and {dir_b} must be byte-for-byte identical"
        );
        compared += 1;
    }
    assert_eq!(compared, expected_files);
}

fn assert_runs_identical(dir_a: &str, dir_b: &str) {
    assert_runs_identical_with_count(dir_a, dir_b, 108);
}

#[test]
fn smoke_01_countries_4_and_coinage_2c1c_1c6c_matches_reference_and_native_decode() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    assert_runs_identical(SMOKE_01_A, SMOKE_01_B);

    let scenario = load_scenario(SMOKE_01_SCENARIO).unwrap();
    assert_eq!((scenario.dsw1, scenario.dsw2), (0xdfda, 0xfbff));
    let reference = load_capture(SMOKE_01_A).unwrap();
    assert_eq!(
        (reference.meta.dsw1, reference.meta.dsw2),
        (Some(0xdfda), Some(0xfbff))
    );

    // Check decoded bytes at frame 8 (after the boot DIP decode 0x3c132..0x3c1bc
    // completes in frame 6 and the chunk sleeps in frame 7).
    let work8 = std::fs::read(Path::new(SMOKE_01_A).join("ram_000008_work.bin")).unwrap();
    assert_eq!(
        &work8[0x063c..0x063e],
        &[0x02, 0x01],
        "Coin A = 2C1C at 0xff863c"
    );
    assert_eq!(
        &work8[0x0644..0x0646],
        &[0x01, 0x06],
        "Coin B = 1C6C at 0xff8644"
    );
    assert_eq!(
        work8[0x4000], 0x05,
        "Game Continuation = 5th Stage Maximum at 0xffc000"
    );
    assert_eq!(
        work8[0x4001], 0x05,
        "Buy-In Feature = 5th Stage Maximum at 0xffc001"
    );
    assert_eq!(work8[0x4005], 0x00, "Number of Countries = 4 at 0xffc005");
    assert_eq!(work8[0x4007], 0x00, "Difficulty = Normal at 0xffc007");
    assert_eq!(work8[0x0f28], 0x00, "Difficulty copy at 0xff8f28");
    assert_eq!(
        &work8[0x0e1c..0x0e1e],
        &[0x00, 0x00],
        "Round Time = 100 at 0xff8e1c"
    );
    assert_eq!(
        work8[0x0682] & 0x04,
        0x00,
        "Flip Off in gfxctrl copy 0xff8682"
    );

    // Native power-on run with DIP switches taken from the scenario matches
    // over frames 0..=7.
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-dip-smoke-01");
    let executed = run_power_on_scenario_capture(
        &assets,
        &out,
        &scenario.scenario_id,
        Path::new(SMOKE_01_SCENARIO),
        BOOT_SETUP_LAST,
    )
    .unwrap();
    assert_eq!(executed, (BOOT_SETUP_LAST + 1) as u64);

    let scope = CompareScope {
        from: Some(0),
        to: Some(BOOT_SETUP_LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &scenario,
        &reference,
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        Outcome::Match {
            frames_compared: 8,
            checkpoints_compared: 8
        }
    ));

    let owned = cpu_owned_addresses();
    let (dsw1, dsw2) = scenario_dsw(Path::new(SMOKE_01_SCENARIO)).unwrap();
    let io = Io {
        dsw1,
        dsw2,
        ..Io::idle()
    };
    let (stopped, unrecovered) = observe_power_on_with_io(assets.maincpu, io, 20, |m| {
        let frame = m.state.frame;
        let image = m.work_image();
        let dump =
            std::fs::read(Path::new(SMOKE_01_A).join(format!("ram_{frame:06}_work.bin"))).unwrap();
        for &(name, addr) in &owned {
            let i = (addr - 0xff8000) as usize;
            assert_eq!(
                dump[i], image[i],
                "smoke-01 frame {frame}: {name} at 0x{addr:06x}"
            );
        }
    });
    assert_eq!((stopped, unrecovered), (20, vec![]));
}

#[test]
fn smoke_02_flip_on_sets_gfxctrl_bit_2_and_matches_native_owned_ram() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    assert_runs_identical(SMOKE_02_A, SMOKE_02_B);

    let scenario = load_scenario(SMOKE_02_SCENARIO).unwrap();
    assert_eq!((scenario.dsw1, scenario.dsw2), (0xdeff, 0xffff));
    let reference = load_capture(SMOKE_02_A).unwrap();
    assert_eq!(
        (reference.meta.dsw1, reference.meta.dsw2),
        (Some(0xdeff), Some(0xffff))
    );

    for frame in 6..=20u32 {
        let work =
            std::fs::read(Path::new(SMOKE_02_A).join(format!("ram_{frame:06}_work.bin"))).unwrap();
        assert_ne!(
            work[0x0682] & 0x04,
            0,
            "bit 2 of 0xff8682 must be set at frame {frame} after 0x3c1ac..0x3c1bc"
        );
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-dip-smoke-02");
    let executed = run_power_on_scenario_capture(
        &assets,
        &out,
        &scenario.scenario_id,
        Path::new(SMOKE_02_SCENARIO),
        20,
    )
    .unwrap();
    assert_eq!(executed, 21);
    let scope = CompareScope {
        from: Some(0),
        to: Some(20),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &scenario,
        &reference,
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 21,
                checkpoints_compared: 21,
            }
        ),
        "smoke-02 flip-on full compare: {outcome:?}"
    );
    let owned = cpu_owned_addresses();
    let (dsw1, dsw2) = scenario_dsw(Path::new(SMOKE_02_SCENARIO)).unwrap();
    let io = Io {
        dsw1,
        dsw2,
        ..Io::idle()
    };
    let (stopped, unrecovered) = observe_power_on_with_io(assets.maincpu, io, 20, |m| {
        let frame = m.state.frame;
        if frame >= 6 {
            assert!(
                m.state.flip_screen,
                "native flip_screen must be true from frame 6"
            );
        }
        let image = m.work_image();
        let dump =
            std::fs::read(Path::new(SMOKE_02_A).join(format!("ram_{frame:06}_work.bin"))).unwrap();
        for &(name, addr) in &owned {
            let i = (addr - 0xff8000) as usize;
            assert_eq!(
                dump[i], image[i],
                "smoke-02 frame {frame}: {name} at 0x{addr:06x}"
            );
        }
    });
    assert_eq!((stopped, unrecovered), (20, vec![]));
}

#[test]
fn smoke_03_service_on_records_the_first_test_screen_frame_and_reports_0x4834e() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    assert_runs_identical(SMOKE_03_A, SMOKE_03_B);

    let scenario = load_scenario(SMOKE_03_SCENARIO).unwrap();
    assert_eq!((scenario.dsw1, scenario.dsw2), (0x5fff, 0xffff));
    let reference = load_capture(SMOKE_03_A).unwrap();
    assert_eq!(
        (reference.meta.dsw1, reference.meta.dsw2),
        (Some(0x5fff), Some(0xffff))
    );

    let mut seen_only_103c = false;
    let mut first_test_screen_frame = None;
    for frame in 0..=20u32 {
        let video =
            std::fs::read(Path::new(SMOKE_03_A).join(format!("ram_{frame:06}_video.bin"))).unwrap();
        let all_103c = video
            .chunks_exact(2)
            .all(|w| u16::from_be_bytes([w[0], w[1]]) == 0x103c);
        if all_103c {
            seen_only_103c = true;
        } else if seen_only_103c && first_test_screen_frame.is_none() {
            first_test_screen_frame = Some(frame);
        }
    }
    assert_eq!(
        first_test_screen_frame,
        Some(SERVICE_TEST_SCREEN_FRAME),
        "test screen must first appear at frame {SERVICE_TEST_SCREEN_FRAME} after 0x103c-only frames 16..=19"
    );

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let mut m =
        PowerOnMachine::from_scenario(assets.maincpu, Path::new(SMOKE_03_SCENARIO)).unwrap();
    for _ in 0..SERVICE_TEST_SCREEN_FRAME {
        m.step();
    }
    assert_eq!(m.unrecovered(), Vec::<u32>::new());
    assert_eq!(m.task_positions(), vec![(0, 0x485ee)]);
}

#[test]
fn negative_control_unapplied_dip_readback_fails_comparison() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SMOKE_02_SCENARIO).unwrap();
    let reference = load_capture(SMOKE_02_A).unwrap();
    // Mutate a copy of the capture's meta.tsv to carry the unapplied default
    // DSW1=0xdfff while keeping scenario_id = sf-dip-smoke-02.
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-dip-neg");
    run_power_on_scenario_capture(
        &assets,
        &out,
        &scenario.scenario_id,
        Path::new(SMOKE_02_SCENARIO),
        1,
    )
    .unwrap();
    std::fs::write(
        out.join("meta.tsv"),
        "dsw1\tdfff\ndsw2\tffff\nscreen_width\t384\nscreen_height\t224\nframes_executed\t2\n",
    )
    .unwrap();
    let candidate = load_capture(&out).unwrap();
    let err = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &CompareScope::full().with_range(0, 1),
    )
    .unwrap_err();
    assert!(
        matches!(&err, CompareError::Incompatible(msg) if msg.contains("DSW1=0xdfff")),
        "expected Incompatible error on unapplied DSW1 readback, got {err:?}"
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExpectedBootDipDecode {
    id: &'static str,
    scenario: &'static str,
    run_a: &'static str,
    run_b: &'static str,
    dsw1: u16,
    dsw2: u16,
    coin_a: [u8; 2],
    coin_b: [u8; 2],
    continuation_max: u8,
    buy_in_max: u8,
    round_time: u8,
    difficulty: u8,
    countries: u8,
    flip_bit: u8,
}

const BOOT_DIP_CASES: [ExpectedBootDipDecode; 8] = [
    ExpectedBootDipDecode {
        id: "sf-dip-boot-01",
        scenario: "../../tools/reference/scenarios/sf_dip_boot_01.json",
        run_a: "../../tools/reference/local/captures/dip-boot-01-run-a",
        run_b: "../../tools/reference/local/captures/dip-boot-01-run-b",
        dsw1: 0xdfc8,
        dsw2: 0xff7f,
        coin_a: [0x04, 0x01],
        coin_b: [0x03, 0x01],
        continuation_max: 0x05,
        buy_in_max: 0x04,
        round_time: 0x00,
        difficulty: 0x00,
        countries: 0xff,
        flip_bit: 0x00,
    },
    ExpectedBootDipDecode {
        id: "sf-dip-boot-02",
        scenario: "../../tools/reference/scenarios/sf_dip_boot_02.json",
        run_a: "../../tools/reference/local/captures/dip-boot-02-run-a",
        run_b: "../../tools/reference/local/captures/dip-boot-02-run-b",
        dsw1: 0xded1,
        dsw2: 0xfad6,
        coin_a: [0x03, 0x01],
        coin_b: [0x02, 0x01],
        continuation_max: 0x04,
        buy_in_max: 0x03,
        round_time: 0x01,
        difficulty: 0x01,
        countries: 0x00,
        flip_bit: 0x04,
    },
    ExpectedBootDipDecode {
        id: "sf-dip-boot-03",
        scenario: "../../tools/reference/scenarios/sf_dip_boot_03.json",
        run_a: "../../tools/reference/local/captures/dip-boot-03-run-a",
        run_b: "../../tools/reference/local/captures/dip-boot-03-run-b",
        dsw1: 0xdfda,
        dsw2: 0xfe2d,
        coin_a: [0x02, 0x01],
        coin_b: [0x01, 0x06],
        continuation_max: 0x03,
        buy_in_max: 0x02,
        round_time: 0x02,
        difficulty: 0x02,
        countries: 0xff,
        flip_bit: 0x00,
    },
    ExpectedBootDipDecode {
        id: "sf-dip-boot-04",
        scenario: "../../tools/reference/scenarios/sf_dip_boot_04.json",
        run_a: "../../tools/reference/local/captures/dip-boot-04-run-a",
        run_b: "../../tools/reference/local/captures/dip-boot-04-run-b",
        dsw1: 0xdee3,
        dsw2: 0xf984,
        coin_a: [0x01, 0x06],
        coin_b: [0x01, 0x04],
        continuation_max: 0x02,
        buy_in_max: 0x01,
        round_time: 0x03,
        difficulty: 0x03,
        countries: 0x00,
        flip_bit: 0x04,
    },
    ExpectedBootDipDecode {
        id: "sf-dip-boot-05",
        scenario: "../../tools/reference/scenarios/sf_dip_boot_05.json",
        run_a: "../../tools/reference/local/captures/dip-boot-05-run-a",
        run_b: "../../tools/reference/local/captures/dip-boot-05-run-b",
        dsw1: 0xdfec,
        dsw2: 0xfcdb,
        coin_a: [0x01, 0x04],
        coin_b: [0x01, 0x03],
        continuation_max: 0x01,
        buy_in_max: 0x00,
        round_time: 0x00,
        difficulty: 0x01,
        countries: 0xff,
        flip_bit: 0x00,
    },
    ExpectedBootDipDecode {
        id: "sf-dip-boot-06",
        scenario: "../../tools/reference/scenarios/sf_dip_boot_06.json",
        run_a: "../../tools/reference/local/captures/dip-boot-06-run-a",
        run_b: "../../tools/reference/local/captures/dip-boot-06-run-b",
        dsw1: 0xdef5,
        dsw2: 0xfbb2,
        coin_a: [0x01, 0x03],
        coin_b: [0x01, 0x02],
        continuation_max: 0x00,
        buy_in_max: 0x05,
        round_time: 0x01,
        difficulty: 0x02,
        countries: 0x00,
        flip_bit: 0x04,
    },
    ExpectedBootDipDecode {
        id: "sf-dip-boot-07",
        scenario: "../../tools/reference/scenarios/sf_dip_boot_07.json",
        run_a: "../../tools/reference/local/captures/dip-boot-07-run-a",
        run_b: "../../tools/reference/local/captures/dip-boot-07-run-b",
        dsw1: 0xdffe,
        dsw2: 0xfd8d,
        coin_a: [0x01, 0x02],
        coin_b: [0x01, 0x01],
        continuation_max: 0x03,
        buy_in_max: 0x01,
        round_time: 0x02,
        difficulty: 0x03,
        countries: 0xff,
        flip_bit: 0x00,
    },
    ExpectedBootDipDecode {
        id: "sf-dip-boot-08",
        scenario: "../../tools/reference/scenarios/sf_dip_boot_08.json",
        run_a: "../../tools/reference/local/captures/dip-boot-08-run-a",
        run_b: "../../tools/reference/local/captures/dip-boot-08-run-b",
        dsw1: 0xdec7,
        dsw2: 0xfa62,
        coin_a: [0x01, 0x01],
        coin_b: [0x04, 0x01],
        continuation_max: 0x00,
        buy_in_max: 0x02,
        round_time: 0x03,
        difficulty: 0x00,
        countries: 0x00,
        flip_bit: 0x04,
    },
];

/// Ticket #57 (F17):
/// 1. C-REF on sf-dip-boot-01 to -08 (power-on, no inputs, 12 frames 0..=11,
///    a checkpoint every frame), setting every defined MAME setting of Coin A,
///    Coin B, Game Continuation, Round Time, Difficulty, Buy-In, Number of
///    Countries and Flip.
/// 2. C-NAT from power-on through the boot task's sleep after 0x3c20a (frame 7)
///    in each, including 0xffc000, 0xffc001, 0xffc005, 0xffc007, 0xff8f28,
///    0xff8e1c, 0xff863c, 0xff8644 and bit 2 of 0xff8682.
/// 3. With Self-test On, the native run reports 0x4834e at the reset.
#[test]
fn boot_dip_decode_for_every_setting_matches_reference_and_native_from_power_on() {
    if !present()
        || !BOOT_DIP_CASES
            .iter()
            .all(|c| Path::new(c.run_a).is_dir() && Path::new(c.run_b).is_dir())
    {
        eprintln!("skipping: local dip-boot captures/ROMs not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let owned = cpu_owned_addresses();

    let mut seen_coin_a = BTreeSet::new();
    let mut seen_coin_b = BTreeSet::new();
    let mut seen_continuation = BTreeSet::new();
    let mut seen_buy_in = BTreeSet::new();
    let mut seen_round_time = BTreeSet::new();
    let mut seen_difficulty = BTreeSet::new();
    let mut seen_countries = BTreeSet::new();
    let mut seen_flip = BTreeSet::new();

    for case in BOOT_DIP_CASES {
        // 1. C-REF: two runs per scenario are byte-for-byte identical across all
        //    63 observation files (12 checkpoints 0..=11 * 5 files + 3 .tsv files).
        assert_runs_identical_with_count(case.run_a, case.run_b, 63);

        let scenario = load_scenario(case.scenario).unwrap();
        assert_eq!(
            (scenario.dsw1, scenario.dsw2),
            (case.dsw1, case.dsw2),
            "{}",
            case.id
        );
        assert_eq!(scenario.checkpoints.len(), 12, "{}", case.id);
        let reference = load_capture(case.run_a).unwrap();
        assert_eq!(
            (reference.meta.dsw1, reference.meta.dsw2),
            (Some(case.dsw1), Some(case.dsw2)),
            "{}",
            case.id
        );

        // 2a. C-NAT capture comparison from power-on across all 12 frames (0..=11),
        //     covering the boot task's sleep after 0x3c20a and warning typing (including
        //     flipped text rendering when Flip Screen = On).
        let out = ScratchDir::new(&format!("sf-engine-{}", case.id));
        let executed = run_power_on_scenario_capture(
            &assets,
            &out,
            &scenario.scenario_id,
            Path::new(case.scenario),
            11,
        )
        .unwrap();
        assert_eq!(executed, 12, "{}", case.id);

        let scope = CompareScope {
            from: Some(0),
            to: Some(11),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        };
        let outcome = compare(
            &scenario,
            &reference,
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &scope,
        )
        .unwrap();
        assert!(
            matches!(
                outcome,
                Outcome::Match {
                    frames_compared: 12,
                    checkpoints_compared: 12,
                }
            ),
            "{}: {outcome:?}",
            case.id
        );

        // 2b. Owned RAM comparison from power-on across all 12 frames (0..=11)
        //     and explicit verification of every boot-decoded DIP field after
        //     0x3c20a (frames 6..=11).
        let (dsw1, dsw2) = scenario_dsw(Path::new(case.scenario)).unwrap();
        let io = Io {
            dsw1,
            dsw2,
            ..Io::idle()
        };
        let (stopped, unrecovered) =
            observe_power_on_with_io(assets.maincpu.clone(), io, 11, |m| {
                let frame = m.state.frame;
                let image = m.work_image();
                let dump =
                    std::fs::read(Path::new(case.run_a).join(format!("ram_{frame:06}_work.bin")))
                        .unwrap();
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    assert_eq!(
                        dump[i], image[i],
                        "{} frame {frame}: {name} at 0x{addr:06x}",
                        case.id
                    );
                }
                for (i, (&ref_b, &nat_b)) in dump.iter().zip(image.iter()).enumerate() {
                    let addr = 0xff8000 + i as u32;
                    let in_slot_saved =
                        (0xff8006..=0xff8105).contains(&addr) && (addr - 0xff8006) % 16 >= 2;
                    let in_stack = (0xff8106..=0xff8631).contains(&addr);
                    if !in_slot_saved && !in_stack {
                        assert_eq!(
                            ref_b, nat_b,
                            "{} frame {frame}: non-slot/non-stack work/object RAM at 0x{addr:06x}",
                            case.id
                        );
                    }
                }
                let ref_video =
                    std::fs::read(Path::new(case.run_a).join(format!("ram_{frame:06}_video.bin")))
                        .unwrap();
                assert_eq!(
                    ref_video, m.state.videoram,
                    "{} frame {frame}: videoram",
                    case.id
                );
                let ref_palette = std::fs::read(
                    Path::new(case.run_a).join(format!("ram_{frame:06}_palette.bin")),
                )
                .unwrap();
                assert_eq!(
                    ref_palette, m.state.paletteram,
                    "{} frame {frame}: paletteram",
                    case.id
                );
                if frame == BOOT_SETUP_LAST {
                    assert_eq!(
                        &image[0x063c..0x063e],
                        &case.coin_a,
                        "{} Coin A at 0xff863c",
                        case.id
                    );
                    assert_eq!(
                        &image[0x0644..0x0646],
                        &case.coin_b,
                        "{} Coin B at 0xff8644",
                        case.id
                    );
                    assert_eq!(
                        image[0x4000], case.continuation_max,
                        "{} Continuation at 0xffc000",
                        case.id
                    );
                    assert_eq!(
                        image[0x4001], case.buy_in_max,
                        "{} Buy-In at 0xffc001",
                        case.id
                    );
                    assert_eq!(
                        image[0x4005], case.countries,
                        "{} Countries at 0xffc005",
                        case.id
                    );
                    assert_eq!(
                        image[0x4007], case.difficulty,
                        "{} Difficulty at 0xffc007",
                        case.id
                    );
                    assert_eq!(
                        image[0x0f28], case.difficulty,
                        "{} Difficulty copy at 0xff8f28",
                        case.id
                    );
                    assert_eq!(
                        &image[0x0e1c..0x0e1e],
                        &[case.round_time, 0x00],
                        "{} Round Time at 0xff8e1c",
                        case.id
                    );
                    assert_eq!(
                        image[0x0682] & 0x04,
                        case.flip_bit,
                        "{} Flip bit 2 at 0xff8682",
                        case.id
                    );
                    assert_eq!(
                        m.state.flip_screen,
                        case.flip_bit != 0,
                        "{} flip_screen state",
                        case.id
                    );
                }
            });
        assert_eq!((stopped, unrecovered), (11, vec![]), "{}", case.id);

        seen_coin_a.insert(case.coin_a);
        seen_coin_b.insert(case.coin_b);
        seen_continuation.insert(case.continuation_max);
        seen_buy_in.insert(case.buy_in_max);
        seen_round_time.insert(case.round_time);
        seen_difficulty.insert(case.difficulty);
        seen_countries.insert(case.countries);
        seen_flip.insert(case.flip_bit);
    }

    // Verify that the 8 scenarios cover every defined MAME setting of all 8
    // boot-decoded DIP fields.
    let all_coin_settings: BTreeSet<[u8; 2]> = [
        [4, 1],
        [3, 1],
        [2, 1],
        [1, 6],
        [1, 4],
        [1, 3],
        [1, 2],
        [1, 1],
    ]
    .into_iter()
    .collect();
    assert_eq!(
        seen_coin_a, all_coin_settings,
        "all 8 Coin A settings covered"
    );
    assert_eq!(
        seen_coin_b, all_coin_settings,
        "all 8 Coin B settings covered"
    );
    assert_eq!(
        seen_continuation,
        BTreeSet::from([0, 1, 2, 3, 4, 5]),
        "all 6 defined Game Continuation settings covered"
    );
    assert_eq!(
        seen_buy_in,
        BTreeSet::from([0, 1, 2, 3, 4, 5]),
        "all 6 defined Buy-In Feature settings covered"
    );
    assert_eq!(
        seen_round_time,
        BTreeSet::from([0, 1, 2, 3]),
        "all 4 Round Time Count settings covered"
    );
    assert_eq!(
        seen_difficulty,
        BTreeSet::from([0, 1, 2, 3]),
        "all 4 Difficulty settings covered"
    );
    assert_eq!(
        seen_countries,
        BTreeSet::from([0x00, 0xff]),
        "both Number of Countries Selected settings covered"
    );
    assert_eq!(
        seen_flip,
        BTreeSet::from([0x00, 0x04]),
        "both Flip Screen settings covered"
    );

    // 3. With Self-test On (DSW1 bit 15 = 0), the native run enters the
    //    service-mode RAM test and input/DIP/sound test (0x4834e..0x48642, F73).
    let self_test_io = Io {
        dsw1: Io::idle().dsw1 & !0x8000,
        ..Io::idle()
    };
    let (stopped, unrecovered) = observe_power_on_with_io(
        assets.maincpu,
        self_test_io,
        SERVICE_TEST_SCREEN_FRAME,
        |_| {},
    );
    assert_eq!((stopped, unrecovered), (SERVICE_TEST_SCREEN_FRAME, vec![]));
}

const NO_CONTINUE_SCENARIO: &str = "../../tools/reference/scenarios/sf_no_continue_01.json";
const NO_CONTINUE_A: &str = "../../tools/reference/local/captures/no-continue-01-run-a";
const NO_CONTINUE_B: &str = "../../tools/reference/local/captures/no-continue-01-run-b";
const MATCH_END_A: &str = "../../tools/reference/local/captures/match-end-01-run-a";

const BUYIN_SCENARIO: &str = "../../tools/reference/scenarios/sf_buyin_01.json";
const BUYIN_A: &str = "../../tools/reference/local/captures/buyin-01-run-a";
const BUYIN_B: &str = "../../tools/reference/local/captures/buyin-01-run-b";
const CHALLENGER_A: &str = "../../tools/reference/local/captures/challenger-01-run-a";

const COINAGE_SCENARIO: &str = "../../tools/reference/scenarios/sf_coinage_01.json";
const COINAGE_A: &str = "../../tools/reference/local/captures/coinage-01-run-a";
const COINAGE_B: &str = "../../tools/reference/local/captures/coinage-01-run-b";

/// Ticket #123 (`F51`): Continuation and buy-in DIPs (`0xffc000` and `0xffc001`
/// under non-default settings):
/// 1. `C-REF`: `sf-no-continue-01` (`Game Continuation = None`, `dsw1 = 0xdfff`,
///    `dsw2 = 0xfffa`, checkpoints `0..=830` and `2389..=5646`) and `sf-buyin-01`
///    (`Buy-In Feature = None`, `dsw1 = 0xdfff`, `dsw2 = 0xfcff`, checkpoints
///    `0..=830` and `2389..=3770`).
/// 2. `C-NAT` for `sf-no-continue-01`: from checkpoint `2389`, the match loss at
///    `5415` goes `0x3efda -> 0x3f2f0 -> 0x3ccae` (`5415..=5645` game over,
///    `0x3ccae` reported at `5646` with slot 2 killed), skipping the 685-frame
///    continue countdown.
/// 3. `C-NAT` for `sf-buyin-01`: from checkpoint `2389` through `3770`, pressing
///    `start2` at `3750` reaches `0x3f4a2` on frame `3751`, which refuses the
///    join (`0xffc001 = 0 <= 0xffc003 = 0`), leaves `0xff865d = 1` and
///    `SCENE_MODE = 0x80`, and resets `0xff8f24 = 0x3f41e`.
/// 4. Every difference from the default captures (`sf-match-end-01` through `5414`
///    and `sf-challenger-01` through `3750`) is checked: only `0xffc000` (`0x00`
///    vs `0x05`) and `0xffc001` (`0x00` vs `0x05`) differ.
#[test]
fn continuation_none_and_buyin_none_match_reference_and_native_from_2389() {
    if ![
        NO_CONTINUE_A,
        NO_CONTINUE_B,
        MATCH_END_A,
        BUYIN_A,
        BUYIN_B,
        CHALLENGER_A,
        ROMS,
    ]
    .iter()
    .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local f51 captures/ROMs not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();

    // 1a. C-REF on sf-no-continue-01 (831 + 3268 = 4099 checkpoints -> 20498 files).
    assert_runs_identical_with_count(NO_CONTINUE_A, NO_CONTINUE_B, 20498);
    let nc_scen = load_scenario(NO_CONTINUE_SCENARIO).unwrap();
    assert_eq!((nc_scen.dsw1, nc_scen.dsw2), (0xdfff, 0xfffa));
    let nc_ref = load_capture(NO_CONTINUE_A).unwrap();
    assert_eq!(
        (nc_ref.meta.dsw1, nc_ref.meta.dsw2),
        (Some(0xdfff), Some(0xfffa))
    );

    // 1b. C-REF on sf-buyin-01 (831 + 1382 = 2213 checkpoints -> 11068 files).
    assert_runs_identical_with_count(BUYIN_A, BUYIN_B, 11068);
    let bi_scen = load_scenario(BUYIN_SCENARIO).unwrap();
    assert_eq!((bi_scen.dsw1, bi_scen.dsw2), (0xdfff, 0xfcff));
    let bi_ref = load_capture(BUYIN_A).unwrap();
    assert_eq!(
        (bi_ref.meta.dsw1, bi_ref.meta.dsw2),
        (Some(0xdfff), Some(0xfcff))
    );

    // 2. Compare sf-no-continue-01 against default sf-match-end-01 through frame 5414:
    //    all frames.tsv rows (screen_hash, latch, latch_changed) over 0..=5414 are
    //    identical, and across shared checkpoints (5000..=5414) only 0xffc000 differs
    //    (0x00 vs 0x05).
    let me_ref = load_capture(MATCH_END_A).unwrap();
    for f in 0..=5414usize {
        let a = &nc_ref.frames[f];
        let b = &me_ref.frames[f];
        assert_eq!(
            (&a.screen_hash, &a.latch, a.latch_changed),
            (&b.screen_hash, &b.latch, b.latch_changed),
            "no-continue vs match-end frame {f}"
        );
    }
    for f in [5000u32, 5100, 5258, 5400, 5414] {
        let w_nc =
            std::fs::read(Path::new(NO_CONTINUE_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
        let w_me =
            std::fs::read(Path::new(MATCH_END_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
        let diffs: Vec<(u32, u8, u8)> = w_nc
            .iter()
            .zip(w_me.iter())
            .enumerate()
            .filter_map(|(i, (&a, &b))| (a != b).then_some((0xff8000 + i as u32, a, b)))
            .collect();
        assert_eq!(diffs, vec![(0xffc000, 0x00, 0x05)], "frame {f} work diffs");
    }

    // 3. C-NAT on sf-no-continue-01 from 2389 through 5655, and 0x3ccae at 5656.
    let nc_ram = compare_fight_ram_with_inputs(
        Path::new(NO_CONTINUE_A),
        assets.maincpu.clone(),
        Some(Path::new(NO_CONTINUE_SCENARIO)),
        2389,
        5655,
    )
    .unwrap();
    assert_eq!(nc_ram, CpuRamOutcome::Match { frames: 3266 });

    let mut latches = Vec::new();
    let mut slot2_at_5655 = None;
    let (stopped, unrecovered) = observe_cpu_fight(
        Path::new(NO_CONTINUE_A),
        assets.maincpu.clone(),
        Some(Path::new(NO_CONTINUE_SCENARIO)),
        2389,
        5656,
        |m| {
            let f = m.state.frame;
            if f >= 5415 && m.state.sound_latch != 0xff {
                latches.push((f, m.state.sound_latch));
            }
            if f == 5655 {
                slot2_at_5655 = Some(m.work_ram()[0x26]);
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (5656, vec![]));
    assert_eq!(
        slot2_at_5655,
        Some(8),
        "slot 2 still active at 5655 before game_over_reset at 5656"
    );
    assert_eq!(
        latches,
        vec![(5415, 0x3f), (5416, 0x95)],
        "wipe exit queues 0x3f (latched at 5415) and 0x3efda -> 0x3f2f0 at 5415 queues 0x95 (latched at 5416)"
    );

    let nc_out = ScratchDir::new("sf-no-continue-01-f51");
    assert_eq!(
        run_fight_capture(
            Path::new(NO_CONTINUE_A),
            &assets,
            &nc_out,
            "sf-no-continue-01",
            Some(Path::new(NO_CONTINUE_SCENARIO)),
            2389,
            5655,
        )
        .unwrap(),
        3267,
    );
    let nc_cmp = compare(
        &nc_scen,
        &nc_ref,
        &load_capture(&nc_out).unwrap(),
        CandidateKind::Native,
        &CompareScope {
            from: Some(2390),
            to: Some(5655),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        },
    )
    .unwrap();
    assert!(
        matches!(
            nc_cmp,
            Outcome::Match {
                frames_compared: 3266,
                checkpoints_compared: 3266,
            }
        ),
        "{nc_cmp:?}"
    );

    // 4. Compare sf-buyin-01 against default sf-challenger-01 through frame 3750:
    //    all frames.tsv rows (screen_hash, latch, latch_changed) over 0..=3750 are
    //    identical, and across shared checkpoints (2389..=3750) only 0xffc001 differs
    //    (0x00 vs 0x05).
    let ch_ref = load_capture(CHALLENGER_A).unwrap();
    for f in 0..=3750usize {
        let a = &bi_ref.frames[f];
        let b = &ch_ref.frames[f];
        assert_eq!(
            (&a.screen_hash, &a.latch, a.latch_changed),
            (&b.screen_hash, &b.latch, b.latch_changed),
            "buyin vs challenger frame {f}"
        );
    }
    for f in [2389u32, 3102, 3205, 3723, 3750] {
        let w_bi = std::fs::read(Path::new(BUYIN_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
        let w_ch =
            std::fs::read(Path::new(CHALLENGER_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
        let diffs: Vec<(u32, u8, u8)> = w_bi
            .iter()
            .zip(w_ch.iter())
            .enumerate()
            .filter_map(|(i, (&a, &b))| (a != b).then_some((0xff8000 + i as u32, a, b)))
            .collect();
        assert_eq!(diffs, vec![(0xffc001, 0x00, 0x05)], "frame {f} work diffs");
    }

    // 5. C-NAT on sf-buyin-01 from 2389 through 3770: 0x3f4a2 refuses at 3751 and
    //    returns 0xff8f24 to 0x3f41e while preserving 0xff865d == 1 and SCENE_MODE == 0x80.
    let bi_ram = compare_fight_ram_with_inputs(
        Path::new(BUYIN_A),
        assets.maincpu.clone(),
        Some(Path::new(BUYIN_SCENARIO)),
        2389,
        3770,
    )
    .unwrap();
    assert_eq!(bi_ram, CpuRamOutcome::Match { frames: 1381 });

    let mut watch_states = Vec::new();
    let (stopped_bi, unrecovered_bi) = observe_cpu_fight(
        Path::new(BUYIN_A),
        assets.maincpu.clone(),
        Some(Path::new(BUYIN_SCENARIO)),
        2389,
        3770,
        |m| {
            let f = m.state.frame;
            if (3750..=3764).contains(&f) {
                let w = m.work_ram();
                let watch = u32::from_be_bytes([w[0xf24], w[0xf25], w[0xf26], w[0xf27]]);
                watch_states.push((f, w[0x4008], w[0x65d], watch));
            }
        },
    )
    .unwrap();
    assert_eq!((stopped_bi, unrecovered_bi), (3770, vec![]));
    // At 3750: armed at 0x3f47e; while start2 is held (3751..=3762): 0x3f41e -> 0x3f454 ->
    // 0x3f47e -> 0x3f4a2 refuses and resets to 0x3f41e on every frame; once released at
    // 3763, watch re-arms and stays at 0x3f47e.
    assert_eq!(watch_states[0], (3750, 0x80, 1, 0x3f47e));
    for f in 3751..=3762u32 {
        assert_eq!(watch_states[(f - 3750) as usize], (f, 0x80, 1, 0x3f41e));
    }
    assert_eq!(
        watch_states[(3763 - 3750) as usize],
        (3763, 0x80, 1, 0x3f47e)
    );
    assert_eq!(*watch_states.last().unwrap(), (3764, 0x80, 1, 0x3f47e));

    let bi_out = ScratchDir::new("sf-buyin-01-f51");
    assert_eq!(
        run_fight_capture(
            Path::new(BUYIN_A),
            &assets,
            &bi_out,
            "sf-buyin-01",
            Some(Path::new(BUYIN_SCENARIO)),
            2389,
            3770,
        )
        .unwrap(),
        1382,
    );
    let bi_cmp = compare(
        &bi_scen,
        &bi_ref,
        &load_capture(&bi_out).unwrap(),
        CandidateKind::Native,
        &CompareScope {
            from: Some(2390),
            to: Some(3770),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        },
    )
    .unwrap();
    assert!(
        matches!(
            bi_cmp,
            Outcome::Match {
                frames_compared: 1381,
                checkpoints_compared: 1381,
            }
        ),
        "{bi_cmp:?}"
    );
}

/// Ticket #142 (`F70`): DIP settings from power-on: coinage (`sf-coinage-01`),
/// continuation (`sf-no-continue-01`), and buy-in (`sf-buyin-01`):
/// 1. `C-REF` for each, with the readback as requested and `F67` (`#139`)'s
///    checkpoint rule (`0..=830`, at each fixture frame, and over the compared
///    windows).
/// 2. `C-NAT` from power-on (`frame 0`) over each span (`0..=1720`, `0..=5645`
///    with `0x3ccae` at `5646`, and `0..=3770`). At each fixture frame (`1440`,
///    `1500`, and `2389`), the owned RAM equals the fixture.
/// 3. `C-NEG`: `Coin A` decoded as `1C1C` (`dsw1 = 0xdfdf`) fails at the first
///    slot-1 coin (`coin1` at `1540..1552`, processed by slot 7 at `1555..=1558`).
#[test]
fn power_on_coinage_no_continue_and_buyin_dips_match_reference_and_catch_1c1c_coin_a() {
    if ![
        COINAGE_A,
        COINAGE_B,
        NO_CONTINUE_A,
        NO_CONTINUE_B,
        BUYIN_A,
        BUYIN_B,
        ROMS,
    ]
    .iter()
    .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local f70 captures/ROMs not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let owned = cpu_owned_addresses();

    // 1. C-REF + F67 checkpoint rule on sf-coinage-01 (0..=830 and 1440..=1720 -> 1112 checkpoints -> 5563 files).
    assert_runs_identical_with_count(COINAGE_A, COINAGE_B, 5563);
    let coinage_scen = load_scenario(COINAGE_SCENARIO).unwrap();
    let mut expected_coinage_cps: Vec<u64> = (0..=830).collect();
    expected_coinage_cps.extend(1440..=1720);
    assert_eq!(coinage_scen.checkpoints, expected_coinage_cps);

    let nc_scen = load_scenario(NO_CONTINUE_SCENARIO).unwrap();
    let mut expected_nc_cps: Vec<u64> = (0..=830).collect();
    expected_nc_cps.extend(2389..=5656);
    assert_eq!(nc_scen.checkpoints, expected_nc_cps);

    let bi_scen = load_scenario(BUYIN_SCENARIO).unwrap();
    let mut expected_bi_cps: Vec<u64> = (0..=830).collect();
    expected_bi_cps.extend(2389..=3770);
    assert_eq!(bi_scen.checkpoints, expected_bi_cps);

    // 2. C-NAT from power-on over each span + owned RAM equality at every fixture frame.
    for (scen_path, ref_dir, scen_id, last_frame, expected_cps, fixture_frames) in [
        (
            COINAGE_SCENARIO,
            COINAGE_A,
            "sf-coinage-01",
            1720u32,
            1112usize,
            &[1440u32, 1500][..],
        ),
        (
            NO_CONTINUE_SCENARIO,
            NO_CONTINUE_A,
            "sf-no-continue-01",
            5655u32,
            4098usize,
            &[2389u32][..],
        ),
        (
            BUYIN_SCENARIO,
            BUYIN_A,
            "sf-buyin-01",
            3770u32,
            2213usize,
            &[2389u32][..],
        ),
    ] {
        let (stopped, unrecovered) = observe_power_on_scenario(
            assets.maincpu.clone(),
            Path::new(scen_path),
            last_frame,
            |m| {
                let f = m.state.frame;
                if fixture_frames.contains(&f) {
                    let dump =
                        std::fs::read(Path::new(ref_dir).join(format!("ram_{f:06}_work.bin")))
                            .unwrap();
                    let image = m.work_image();
                    for &(name, addr) in &owned {
                        let i = (addr - 0xff8000) as usize;
                        assert_eq!(
                            dump[i], image[i],
                            "{scen_id} fixture frame {f}: {name} at 0x{addr:06x}"
                        );
                    }
                }
            },
        )
        .unwrap();
        assert_eq!((stopped, unrecovered), (last_frame, vec![]), "{scen_id}");

        let out = ScratchDir::new(&format!("{scen_id}-f70-po"));
        let executed = run_power_on_scenario_capture_for_reference(
            &assets,
            Path::new(ref_dir),
            &out,
            scen_id,
            Path::new(scen_path),
            last_frame,
        )
        .unwrap();
        assert_eq!(executed, (last_frame + 1) as u64, "{scen_id}");

        let cmp = compare(
            &load_scenario(scen_path).unwrap(),
            &load_capture(ref_dir).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &CompareScope {
                from: Some(0),
                to: Some(last_frame as u64),
                fields: Some(
                    CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap(),
                ),
            },
        )
        .unwrap();
        assert!(
            matches!(
                cmp,
                Outcome::Match {
                    frames_compared,
                    checkpoints_compared,
                } if frames_compared == (last_frame + 1) as u64 && checkpoints_compared == expected_cps
            ),
            "{scen_id}: {cmp:?}"
        );
    }

    // Also verify that stepping sf-no-continue-01 one more frame from power-on (to 5656)
    // enters 0x3ccae (attract_join) cleanly with no unrecovered branches.
    let (stopped_nc, unrec_nc) = observe_power_on_scenario(
        assets.maincpu.clone(),
        Path::new(NO_CONTINUE_SCENARIO),
        5656,
        |_| {},
    )
    .unwrap();
    assert_eq!((stopped_nc, unrec_nc), (5656, vec![]));

    // 3. C-NEG: Coin A decoded as 1C1C (dsw1 = 0xdfdf instead of 0xdfda) fails at the
    //    first slot-1 coin (coin1 pressed at 1540..1552, processed by slot 7 at 1555..=1558:
    //    with 2C1C it decrements partial coin 0xff863e = 1 and awards 0 credits; with 1C1C
    //    it awards a 7th credit at 0xff865d = 7 and queues coin sound 0x0f).
    let bad_scen = ScratchDir::new("sf-coinage-neg-1c1c");
    std::fs::create_dir_all(&bad_scen).unwrap();
    let bad_scen_path = bad_scen.join("sf_coinage_neg.json");
    let bad_json = std::fs::read_to_string(COINAGE_SCENARIO)
        .unwrap()
        .replace("\"2C_1C\"", "\"1C_1C\"");
    std::fs::write(&bad_scen_path, bad_json).unwrap();
    let mut first_credit_diff = None;
    let mut first_latch_diff = None;
    observe_power_on_scenario(assets.maincpu, &bad_scen_path, 1565, |m| {
        let f = m.state.frame;
        if f >= 1540 {
            let ref_w =
                std::fs::read(Path::new(COINAGE_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
            if first_credit_diff.is_none() && m.work_ram()[0x065d] != ref_w[0x065d] {
                first_credit_diff = Some(f);
            }
            if first_latch_diff.is_none() && m.state.sound_latch != 0xff {
                first_latch_diff = Some((f, m.state.sound_latch));
            }
        }
    })
    .unwrap();
    assert!(
        first_credit_diff.is_some_and(|f| (1552..=1560).contains(&f)),
        "1C1C Coin A must fail on credits at the first slot-1 coin (1552..=1560), got {first_credit_diff:?}"
    );
    assert!(
        first_latch_diff.is_some_and(|(f, nat)| (1552..=1561).contains(&f) && nat == 0x0f),
        "1C1C Coin A must fail on sound latch (0x0f) at the first slot-1 coin, got {first_latch_diff:?}"
    );
}

const FLIP_02_SCENARIO: &str = "../../tools/reference/scenarios/sf_flip_02.json";
const FLIP_02_A: &str = "../../tools/reference/local/captures/flip-02-run-a";
const FLIP_02_B: &str = "../../tools/reference/local/captures/flip-02-run-b";
const FLIP_01_A: &str = "../../tools/reference/local/captures/flip-01-run-a";

const SELECT4_03_SCENARIO: &str = "../../tools/reference/scenarios/sf_select4_03.json";
const SELECT4_03_A: &str = "../../tools/reference/local/captures/select4-03-run-a";
const SELECT4_03_B: &str = "../../tools/reference/local/captures/select4-03-run-b";
const SELECT4_01_A: &str = "../../tools/reference/local/captures/select4-01-run-a";

const ATTRACT_MUSIC_OFF_02_SCENARIO: &str =
    "../../tools/reference/scenarios/sf_attract_music_off_02.json";
const ATTRACT_MUSIC_OFF_02_A: &str =
    "../../tools/reference/local/captures/attract-music-off-02-run-a";
const ATTRACT_MUSIC_OFF_02_B: &str =
    "../../tools/reference/local/captures/attract-music-off-02-run-b";

const DEMOSOUND_OFF_02_SCENARIO: &str = "../../tools/reference/scenarios/sf_demosound_off_02.json";
const DEMOSOUND_OFF_02_A: &str = "../../tools/reference/local/captures/demosound-off-02-run-a";
const DEMOSOUND_OFF_02_B: &str = "../../tools/reference/local/captures/demosound-off-02-run-b";

const DEMOSOUND_COIN_01_SCENARIO: &str =
    "../../tools/reference/scenarios/sf_demosound_coin_01.json";
const DEMOSOUND_COIN_01_A: &str = "../../tools/reference/local/captures/demosound-coin-01-run-a";
const DEMOSOUND_COIN_01_B: &str = "../../tools/reference/local/captures/demosound-coin-01-run-b";

/// Ticket #139 (F67):
/// 1. C-REF on sf-flip-02 (Flip Screen = On, dsw1 = 0xdeff, dsw2 = 0xffff,
///    checkpoints 0..=830, 1500, and 2389..=5040) and sf-select4-03 (Number of
///    Countries Selected = 4, dsw1 = 0xdfff, dsw2 = 0xfbff, checkpoints 0..=830
///    and 1440..=1770).
/// 2. C-NAT from power-on over each scenario's span (0..=5040 for sf-flip-02,
///    including 5014; 0..=1770 for sf-select4-03, through the select exit at
///    1761 and event 2's request). At each fixture frame F18 and F47 used
///    (790, 1500, 2389 for sf-flip-02; 1440, 1500 for sf-select4-03), owned RAM
///    equals the fixture.
/// 3. C-NEG: the native run with Flip Off in its inputs only fails at the first
///    frame the flip shows (checkpoint 8 screen_rgba, or frame 9 screen_hash).
#[test]
fn power_on_flip_and_four_countries_dips_match_reference_and_catch_flip_off() {
    if !present()
        || ![
            FLIP_02_A,
            FLIP_02_B,
            FLIP_01_A,
            SELECT4_03_A,
            SELECT4_03_B,
            SELECT4_01_A,
        ]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local flip-02 / select4-03 captures not present");
        return;
    }

    // 1. C-REF: run-a == run-b, readbacks match, and shared frames/checkpoints
    //    match sf-flip-01 and sf-select4-01.
    assert_runs_identical_with_count(FLIP_02_A, FLIP_02_B, 3484 * 5 + 3);
    assert_runs_identical_with_count(SELECT4_03_A, SELECT4_03_B, 1162 * 5 + 3);

    let flip_ref = load_capture(FLIP_02_A).unwrap();
    let flip01_ref = load_capture(FLIP_01_A).unwrap();
    assert_eq!(
        (flip_ref.meta.dsw1, flip_ref.meta.dsw2),
        (Some(0xdeff), Some(0xffff))
    );
    for (r02, r01) in flip_ref.frames.iter().zip(flip01_ref.frames.iter()) {
        assert_eq!(
            r02.screen_hash, r01.screen_hash,
            "flip-02 vs flip-01 frame {}",
            r02.frame
        );
        assert_eq!(
            r02.latch, r01.latch,
            "flip-02 vs flip-01 frame {}",
            r02.frame
        );
    }

    let sel_ref = load_capture(SELECT4_03_A).unwrap();
    let sel01_ref = load_capture(SELECT4_01_A).unwrap();
    assert_eq!(
        (sel_ref.meta.dsw1, sel_ref.meta.dsw2),
        (Some(0xdfff), Some(0xfbff))
    );
    for (r03, r01) in sel_ref.frames.iter().zip(sel01_ref.frames.iter()) {
        assert_eq!(
            r03.screen_hash, r01.screen_hash,
            "select4-03 vs select4-01 frame {}",
            r03.frame
        );
        assert_eq!(
            r03.latch, r01.latch,
            "select4-03 vs select4-01 frame {}",
            r03.frame
        );
    }

    // 2. C-NAT from power-on over each scenario's span and owned RAM equality
    //    at each fixture frame F18 and F47 used.
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let owned = cpu_owned_addresses();

    let cases: [(&str, &str, &str, u32, &[u32], usize); 2] = [
        (
            "sf-flip-02",
            FLIP_02_SCENARIO,
            FLIP_02_A,
            5040,
            &[790, 1500, 2389],
            3484,
        ),
        (
            "sf-select4-03",
            SELECT4_03_SCENARIO,
            SELECT4_03_A,
            1770,
            &[1440, 1500],
            1162,
        ),
    ];

    for (scen_id, scen_path, ref_dir, last_frame, fixture_frames, expected_cps) in cases {
        let mut seen_fixtures = Vec::new();
        let (stopped, unrecovered) = observe_power_on_scenario(
            assets.maincpu.clone(),
            Path::new(scen_path),
            last_frame,
            |m| {
                let f = m.state.frame;
                if fixture_frames.contains(&f) {
                    seen_fixtures.push(f);
                    let image = m.work_image();
                    let dump =
                        std::fs::read(Path::new(ref_dir).join(format!("ram_{f:06}_work.bin")))
                            .unwrap();
                    for &(name, addr) in &owned {
                        let i = (addr - 0xff8000) as usize;
                        assert_eq!(
                            dump[i], image[i],
                            "{scen_id} fixture frame {f}: {name} at 0x{addr:06x}"
                        );
                    }
                }
            },
        )
        .unwrap();
        assert_eq!((stopped, unrecovered), (last_frame, vec![]), "{scen_id}");
        assert_eq!(
            seen_fixtures.as_slice(),
            fixture_frames,
            "{scen_id} fixture frames"
        );

        let out = ScratchDir::new(&format!("{scen_id}-f67-po"));
        let executed = run_power_on_scenario_capture_for_reference(
            &assets,
            Path::new(ref_dir),
            &out,
            scen_id,
            Path::new(scen_path),
            last_frame,
        )
        .unwrap();
        assert_eq!(executed, (last_frame + 1) as u64, "{scen_id}");

        let cmp = compare(
            &load_scenario(scen_path).unwrap(),
            &load_capture(ref_dir).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &CompareScope {
                from: Some(0),
                to: Some(last_frame as u64),
                fields: Some(
                    CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap(),
                ),
            },
        )
        .unwrap();
        assert!(
            matches!(
                cmp,
                Outcome::Match {
                    frames_compared,
                    checkpoints_compared,
                } if frames_compared == (last_frame + 1) as u64 && checkpoints_compared == expected_cps
            ),
            "{scen_id}: {cmp:?}"
        );
    }

    // 3. C-NEG: running sf-flip-02 with Flip Screen = Off (dsw1 = 0xdfff)
    //    matches frames 0..=7 (blank screen before the warning text starts at
    //    frame 8) and fails at checkpoint 8 (screen_rgba) and frame 9
    //    (screen_hash, the first frame the flip shows in frames.tsv).
    let flip_off_dir = ScratchDir::new("sf-flip-02-neg-scen");
    std::fs::create_dir_all(&flip_off_dir).unwrap();
    let flip_off_scen_path = flip_off_dir.join("sf_flip_02_off.json");
    let flip_off_json = std::fs::read_to_string(FLIP_02_SCENARIO)
        .unwrap()
        .replace("\"On\"", "\"Off\"");
    std::fs::write(&flip_off_scen_path, flip_off_json).unwrap();

    let flip_off_cap = ScratchDir::new("sf-flip-02-neg-cap");
    run_power_on_scenario_capture_for_reference(
        &assets,
        Path::new(FLIP_02_A),
        &flip_off_cap,
        "sf-flip-02",
        &flip_off_scen_path,
        15,
    )
    .unwrap();
    let flip_scen = load_scenario(FLIP_02_SCENARIO).unwrap();
    let mut flip_off_loaded = load_capture(&flip_off_cap).unwrap();
    flip_off_loaded.meta.dsw1 = Some(0xdeff);

    let cmp_0_to_7 = compare(
        &flip_scen,
        &flip_ref,
        &flip_off_loaded,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(7),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        },
    )
    .unwrap();
    assert!(
        matches!(
            cmp_0_to_7,
            Outcome::Match {
                frames_compared: 8,
                checkpoints_compared: 8,
            }
        ),
        "frames 0..=7 must match before the warning text is drawn: {cmp_0_to_7:?}"
    );

    let cmp_with_cp = compare(
        &flip_scen,
        &flip_ref,
        &flip_off_loaded,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(15),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        },
    )
    .unwrap();
    let Outcome::Mismatch(m_cp) = cmp_with_cp else {
        panic!("Flip Off must fail comparison: {cmp_with_cp:?}");
    };
    assert_eq!(
        m_cp.frame, 8,
        "first checkpoint pixel difference is at frame 8"
    );
    assert_eq!(m_cp.observable, "checkpoint screen pixels");
}

/// Ticket #140 (F68):
/// 1. C-REF on sf-attract-music-off-02 (Attract Music = Off, dsw1 = 0xddff,
///    0..=5145), sf-demosound-off-02 (Demo Sounds = Off, dsw1 = 0xffff,
///    0..=5145), and sf-demosound-coin-01 (Demo Sounds = Off with coin1@1450,
///    dsw1 = 0xffff, 0..=1520).
/// 2. C-NAT from power-on over each span. With Demo Sounds Off the filtered
///    entry (0x3b7c8) passes nothing while 0xff8003 is 0, while the boot
///    commands (frames 1..=3) and the coin's 0x0f (frame 1468) still reach the
///    latch through the unfiltered entry 0x3b802.
/// 3. C-NEG: routing the coin's 0x0f through the filtered entry 0x3b7c8 fails
///    at the coin's latch frame (1468).
#[test]
fn power_on_attract_music_demosound_and_coin_dips_match_reference_and_catch_filtered_coin() {
    if !present()
        || ![
            ATTRACT_MUSIC_OFF_02_A,
            ATTRACT_MUSIC_OFF_02_B,
            DEMOSOUND_OFF_02_A,
            DEMOSOUND_OFF_02_B,
            DEMOSOUND_COIN_01_A,
            DEMOSOUND_COIN_01_B,
        ]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local attract-music-off-02 / demosound captures not present");
        return;
    }

    // 1. C-REF: run-a == run-b and readbacks match.
    assert_runs_identical_with_count(ATTRACT_MUSIC_OFF_02_A, ATTRACT_MUSIC_OFF_02_B, 1944 * 5 + 3);
    assert_runs_identical_with_count(DEMOSOUND_OFF_02_A, DEMOSOUND_OFF_02_B, 1944 * 5 + 3);
    assert_runs_identical_with_count(DEMOSOUND_COIN_01_A, DEMOSOUND_COIN_01_B, 912 * 5 + 3);

    let am_ref = load_capture(ATTRACT_MUSIC_OFF_02_A).unwrap();
    let ds_ref = load_capture(DEMOSOUND_OFF_02_A).unwrap();
    let dc_ref = load_capture(DEMOSOUND_COIN_01_A).unwrap();
    assert_eq!(
        (am_ref.meta.dsw1, am_ref.meta.dsw2),
        (Some(0xddff), Some(0xffff))
    );
    assert_eq!(
        (ds_ref.meta.dsw1, ds_ref.meta.dsw2),
        (Some(0xffff), Some(0xffff))
    );
    assert_eq!(
        (dc_ref.meta.dsw1, dc_ref.meta.dsw2),
        (Some(0xffff), Some(0xffff))
    );

    // Check latch events in sf-demosound-off-02 and sf-demosound-coin-01:
    // - In sf-demosound-off-02, only the boot commands (frames 1..=3: 0x27, 0x3f, 0x80
    //   via unfiltered 0x3b802, plus 0xff idle clear at frame 4) change the latch;
    //   over 5..=5145 while 0xff8003 == 0, zero commands pass through 0x3b7c8.
    let ds_latches: Vec<(u64, &str)> = ds_ref
        .frames
        .iter()
        .filter(|r| r.latch_changed)
        .map(|r| (r.frame, r.latch.as_str()))
        .collect();
    assert_eq!(ds_latches, vec![(1, "27"), (2, "3f"), (3, "80"), (4, "ff")]);

    // - In sf-demosound-coin-01, frames 1..=4 latch (0x27, 0x3f, 0x80, 0xff), then at
    //   frame 1468 the coin's 0x0f (queued at 1467 via 0x3b802 at 0x482da while
    //   0xff8003 is still 0) latches, followed by 0x3f (1469), 0x80 (1470), and 0xff
    //   (1471) once 0xff8003 = 0xff.
    let dc_latches: Vec<(u64, &str)> = dc_ref
        .frames
        .iter()
        .filter(|r| r.latch_changed)
        .map(|r| (r.frame, r.latch.as_str()))
        .collect();
    assert_eq!(
        dc_latches,
        vec![
            (1, "27"),
            (2, "3f"),
            (3, "80"),
            (4, "ff"),
            (1468, "0f"),
            (1469, "3f"),
            (1470, "80"),
            (1471, "ff"),
        ]
    );

    // 2. C-NAT from power-on over each span.
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let owned = cpu_owned_addresses();

    let cases: [(&str, &str, &str, u32, &[u32], usize); 3] = [
        (
            "sf-attract-music-off-02",
            ATTRACT_MUSIC_OFF_02_SCENARIO,
            ATTRACT_MUSIC_OFF_02_A,
            5140,
            &[794],
            1944,
        ),
        (
            "sf-demosound-off-02",
            DEMOSOUND_OFF_02_SCENARIO,
            DEMOSOUND_OFF_02_A,
            5140,
            &[794],
            1944,
        ),
        (
            "sf-demosound-coin-01",
            DEMOSOUND_COIN_01_SCENARIO,
            DEMOSOUND_COIN_01_A,
            1520,
            &[794, 1440, 1500],
            912,
        ),
    ];

    for (scen_id, scen_path, ref_dir, last_frame, fixture_frames, expected_cps) in cases {
        let mut seen_fixtures = Vec::new();
        let (stopped, unrecovered) = observe_power_on_scenario(
            assets.maincpu.clone(),
            Path::new(scen_path),
            last_frame,
            |m| {
                let f = m.state.frame;
                if fixture_frames.contains(&f) {
                    seen_fixtures.push(f);
                    let image = m.work_image();
                    let dump =
                        std::fs::read(Path::new(ref_dir).join(format!("ram_{f:06}_work.bin")))
                            .unwrap();
                    for &(name, addr) in &owned {
                        let i = (addr - 0xff8000) as usize;
                        assert_eq!(
                            dump[i], image[i],
                            "{scen_id} fixture frame {f}: {name} at 0x{addr:06x}"
                        );
                    }
                }
            },
        )
        .unwrap();
        assert_eq!((stopped, unrecovered), (last_frame, vec![]), "{scen_id}");
        assert_eq!(
            seen_fixtures.as_slice(),
            fixture_frames,
            "{scen_id} fixture frames"
        );

        let out = ScratchDir::new(&format!("{scen_id}-f68-po"));
        let executed = run_power_on_scenario_capture_for_reference(
            &assets,
            Path::new(ref_dir),
            &out,
            scen_id,
            Path::new(scen_path),
            last_frame,
        )
        .unwrap();
        assert_eq!(executed, (last_frame + 1) as u64, "{scen_id}");

        let cmp = compare(
            &load_scenario(scen_path).unwrap(),
            &load_capture(ref_dir).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &CompareScope {
                from: Some(0),
                to: Some(last_frame as u64),
                fields: Some(
                    CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap(),
                ),
            },
        )
        .unwrap();
        assert!(
            matches!(
                cmp,
                Outcome::Match {
                    frames_compared,
                    checkpoints_compared,
                } if frames_compared == (last_frame + 1) as u64 && checkpoints_compared == expected_cps
            ),
            "{scen_id}: {cmp:?}"
        );
    }

    // 3. C-NEG: if the coin's 0x0f at frame 1467 in sf-demosound-coin-01 is routed
    //    through the filtered entry 0x3b7c8 (which drops commands when 0xff8003 == 0
    //    and Demo Sounds = Off) instead of 0x3b802, comparison fails at frame 1468
    //    (the coin's latch frame).
    let neg_out = ScratchDir::new("sf-demosound-coin-01-neg");
    run_power_on_scenario_capture_for_reference(
        &assets,
        Path::new(DEMOSOUND_COIN_01_A),
        &neg_out,
        "sf-demosound-coin-01",
        Path::new(DEMOSOUND_COIN_01_SCENARIO),
        1472,
    )
    .unwrap();
    let mut neg_cap = load_capture(&neg_out).unwrap();
    // Simulate 0x482da calling filtered 0x3b7c8 instead of unfiltered 0x3b802:
    // at frame 1467 (while 0xff8003 == 0 and dsw1 & 0x2000 != 0), 0x0f is dropped,
    // so frame 1468 does not latch 0x0f (latch stays 0xff, latch_changed = false).
    let row_1468 = neg_cap.frames.iter_mut().find(|r| r.frame == 1468).unwrap();
    assert_eq!(
        (row_1468.latch.as_str(), row_1468.latch_changed),
        ("0f", true)
    );
    row_1468.latch = "ff".to_string();
    row_1468.latch_changed = false;

    let neg_cmp = compare(
        &load_scenario(DEMOSOUND_COIN_01_SCENARIO).unwrap(),
        &dc_ref,
        &neg_cap,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(1472),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        },
    )
    .unwrap();
    let Outcome::Mismatch(m_neg) = neg_cmp else {
        panic!("filtered coin 0x0f must fail comparison: {neg_cmp:?}");
    };
    assert_eq!(
        m_neg.frame, 1468,
        "must fail at the coin's latch frame 1468"
    );
    assert_eq!(m_neg.observable, "latch");
}

const FIGHT_DSW_01_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_dsw_01.json";
const FIGHT_DSW_01_A: &str = "../../tools/reference/local/captures/fixed-fight-dsw-01-run-a";
const FIGHT_DSW_01_B: &str = "../../tools/reference/local/captures/fixed-fight-dsw-01-run-b";

const FIGHT_DSW_02_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_dsw_02.json";
const FIGHT_DSW_02_A: &str = "../../tools/reference/local/captures/fixed-fight-dsw-02-run-a";
const FIGHT_DSW_02_B: &str = "../../tools/reference/local/captures/fixed-fight-dsw-02-run-b";

const FIGHT_DSW_03_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_dsw_03.json";
const FIGHT_DSW_03_A: &str = "../../tools/reference/local/captures/fixed-fight-dsw-03-run-a";
const FIGHT_DSW_03_B: &str = "../../tools/reference/local/captures/fixed-fight-dsw-03-run-b";

const FIGHT_DSW_04_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_dsw_04.json";
const FIGHT_DSW_04_A: &str = "../../tools/reference/local/captures/fixed-fight-dsw-04-run-a";
const FIGHT_DSW_04_B: &str = "../../tools/reference/local/captures/fixed-fight-dsw-04-run-b";

const FIGHT_SLOW_01_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_slow_01.json";
const FIGHT_SLOW_01_A: &str = "../../tools/reference/local/captures/fixed-fight-slow-01-run-a";
const FIGHT_SLOW_01_B: &str = "../../tools/reference/local/captures/fixed-fight-slow-01-run-b";

const FREEZE_01_SCENARIO: &str = "../../tools/reference/scenarios/sf_freeze_01.json";
const FREEZE_01_A: &str = "../../tools/reference/local/captures/freeze-01-run-a";
const FREEZE_01_B: &str = "../../tools/reference/local/captures/freeze-01-run-b";

/// Ticket #141 (F69):
/// 1. C-REF on sf-fixed-fight-dsw-01 (Easy / 150), sf-fixed-fight-dsw-02
///    (Difficult / 200), sf-fixed-fight-dsw-03 (Very Difficult / 250),
///    sf-fixed-fight-dsw-04 (Attract Music Off), sf-fixed-fight-slow-01
///    (Speed Slow), and sf-freeze-01 (Freeze), with checkpoints over 0..=830
///    and over each scenario's fixture and compared fight window.
/// 2. C-NAT from power-on over each span through its match decision (or round
///    result for slow). At each fixture frame the earlier ticket used (2389 or
///    2975), the owned RAM equals the fixture.
/// 3. C-NEG: the round-time setting decoded one step off (100 instead of 150
///    in sf-fixed-fight-dsw-01) fails at the first TIME digit (frame 2385 in
///    work/video RAM at 0xff8e19 / 0x80025c, and frame 2386 in frames.tsv
///    screen_hash).
#[test]
fn power_on_fight_dips_match_reference_and_catch_round_time_off_by_one() {
    if !present()
        || ![
            FIGHT_DSW_01_A,
            FIGHT_DSW_01_B,
            FIGHT_DSW_02_A,
            FIGHT_DSW_02_B,
            FIGHT_DSW_03_A,
            FIGHT_DSW_03_B,
            FIGHT_DSW_04_A,
            FIGHT_DSW_04_B,
            FIGHT_SLOW_01_A,
            FIGHT_SLOW_01_B,
            FREEZE_01_A,
            FREEZE_01_B,
        ]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local fight-DIP captures not present");
        return;
    }

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let owned = cpu_owned_addresses();

    let cases: [(&str, &str, &str, &str, u16, u16, u32, u32, usize, usize); 6] = [
        (
            "sf-fixed-fight-dsw-01",
            FIGHT_DSW_01_SCENARIO,
            FIGHT_DSW_01_A,
            FIGHT_DSW_01_B,
            0xdfff,
            0xffd7,
            2389,
            5100,
            3543,
            3544 * 5 + 3,
        ),
        (
            "sf-fixed-fight-dsw-02",
            FIGHT_DSW_02_SCENARIO,
            FIGHT_DSW_02_A,
            FIGHT_DSW_02_B,
            0xdfff,
            0xffaf,
            2389,
            4768,
            3211,
            3243 * 5 + 3,
        ),
        (
            "sf-fixed-fight-dsw-03",
            FIGHT_DSW_03_SCENARIO,
            FIGHT_DSW_03_A,
            FIGHT_DSW_03_B,
            0xdfff,
            0xff87,
            2389,
            4647,
            3090,
            3143 * 5 + 3,
        ),
        (
            "sf-fixed-fight-dsw-04",
            FIGHT_DSW_04_SCENARIO,
            FIGHT_DSW_04_A,
            FIGHT_DSW_04_B,
            0xddff,
            0xffff,
            2389,
            5015,
            3458,
            3543 * 5 + 3,
        ),
        (
            "sf-fixed-fight-slow-01",
            FIGHT_SLOW_01_SCENARIO,
            FIGHT_SLOW_01_A,
            FIGHT_SLOW_01_B,
            0xcfff,
            0xffff,
            2975,
            5652,
            3509,
            3517 * 5 + 3,
        ),
        (
            "sf-freeze-01",
            FREEZE_01_SCENARIO,
            FREEZE_01_A,
            FREEZE_01_B,
            0xdfff,
            0xffff,
            2389,
            5175,
            3618,
            3623 * 5 + 4,
        ),
    ];

    for (
        scen_id,
        scen_path,
        run_a,
        run_b,
        dsw1,
        dsw2,
        fixture_frame,
        last_frame,
        expected_cps,
        total_files,
    ) in cases
    {
        // 1. C-REF: run-a == run-b and readbacks match.
        assert_runs_identical_with_count(run_a, run_b, total_files);
        let ref_cap = load_capture(run_a).unwrap();
        assert_eq!(
            (ref_cap.meta.dsw1, ref_cap.meta.dsw2),
            (Some(dsw1), Some(dsw2)),
            "{scen_id}"
        );

        // 2. C-NAT from power-on over each span, checking owned RAM at the
        //    earlier ticket's fixture frame (2389 or 2975).
        let mut saw_fixture = false;
        let mut freeze_release_writes = None;
        let (stopped, unrecovered) = observe_power_on_scenario(
            assets.maincpu.clone(),
            Path::new(scen_path),
            last_frame,
            |m| {
                let f = m.state.frame;
                if f == fixture_frame {
                    saw_fixture = true;
                    let image = m.work_image();
                    let dump = std::fs::read(Path::new(run_a).join(format!("ram_{f:06}_work.bin")))
                        .unwrap();
                    for &(name, addr) in &owned {
                        let i = (addr - 0xff8000) as usize;
                        assert_eq!(
                            dump[i], image[i],
                            "{scen_id} fixture frame {f}: {name} at 0x{addr:06x}"
                        );
                    }
                }
                if scen_id == "sf-freeze-01" && f == 2561 {
                    freeze_release_writes = Some(m.latch_writes().to_vec());
                }
            },
        )
        .unwrap();
        assert_eq!((stopped, unrecovered), (last_frame, vec![]), "{scen_id}");
        assert!(
            saw_fixture,
            "{scen_id} must observe fixture frame {fixture_frame}"
        );
        if scen_id == "sf-freeze-01" {
            let writes = freeze_release_writes.expect("freeze release frame 2561");
            assert_eq!(
                writes.len(),
                2,
                "interrupted suffix plus held vblank at 2561"
            );
            assert_eq!(writes[1].cycles - writes[0].cycles, 2_360);
        }

        let out = ScratchDir::new(&format!("{scen_id}-f69-po"));
        let executed = run_power_on_scenario_capture_for_reference(
            &assets,
            Path::new(run_a),
            &out,
            scen_id,
            Path::new(scen_path),
            last_frame,
        )
        .unwrap();
        assert_eq!(executed, (last_frame + 1) as u64, "{scen_id}");

        let cmp = compare(
            &load_scenario(scen_path).unwrap(),
            &ref_cap,
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &CompareScope {
                from: Some(0),
                to: Some(last_frame as u64),
                fields: Some(
                    CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap(),
                ),
            },
        )
        .unwrap();
        assert!(
            matches!(
                cmp,
                Outcome::Match {
                    frames_compared,
                    checkpoints_compared,
                } if frames_compared == (last_frame + 1) as u64 && checkpoints_compared == expected_cps
            ),
            "{scen_id}: {cmp:?}"
        );
    }

    // 3. C-NEG: Round Time Count decoded one step off (100 instead of 150 in
    //    sf-fixed-fight-dsw-01, dsw2 = 0xffdf instead of 0xffd7) matches frames
    //    0..=2389 (832 checkpoints including 2389) before round_start (0x3e79c)
    //    reloads and draws the round timer at frame 2390, and fails at frame
    //    2390 on checkpoint screen pixels (when 0xff8e18..=0xff8e1a becomes
    //    01 00 00 instead of 01 05 00 and the HUD TIME digits are drawn).
    let neg_scen_dir = ScratchDir::new("sf-fight-dsw-01-neg-scen");
    std::fs::create_dir_all(&neg_scen_dir).unwrap();
    let neg_scen_path = neg_scen_dir.join("sf_fixed_fight_dsw_01_time100.json");
    let neg_json = std::fs::read_to_string(FIGHT_DSW_01_SCENARIO)
        .unwrap()
        .replace("\"150\"", "\"100\"");
    std::fs::write(&neg_scen_path, neg_json).unwrap();

    let neg_cap_dir = ScratchDir::new("sf-fight-dsw-01-neg-cap");
    run_power_on_scenario_capture_for_reference(
        &assets,
        Path::new(FIGHT_DSW_01_A),
        &neg_cap_dir,
        "sf-fixed-fight-dsw-01",
        &neg_scen_path,
        2390,
    )
    .unwrap();
    let dsw01_scen = load_scenario(FIGHT_DSW_01_SCENARIO).unwrap();
    let dsw01_ref = load_capture(FIGHT_DSW_01_A).unwrap();
    let mut neg_cap = load_capture(&neg_cap_dir).unwrap();
    neg_cap.meta.dsw2 = Some(0xffd7);

    let native_fields = CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap();
    let cmp_before_time = compare(
        &dsw01_scen,
        &dsw01_ref,
        &neg_cap,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(2389),
            fields: Some(native_fields.clone()),
        },
    )
    .unwrap();
    assert!(
        matches!(
            cmp_before_time,
            Outcome::Match {
                frames_compared: 2390,
                checkpoints_compared: 832,
            }
        ),
        "frames 0..=2389 must match before 0x3e79c draws the reloaded TIME digits at 2390: {cmp_before_time:?}"
    );

    let cmp_at_2390 = compare(
        &dsw01_scen,
        &dsw01_ref,
        &neg_cap,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(2390),
            fields: Some(native_fields),
        },
    )
    .unwrap();
    let Outcome::Mismatch(m_2390) = cmp_at_2390 else {
        panic!("one-step-off Round Time Count must fail at checkpoint 2390 when 0x3e79c draws the TIME digits: {cmp_at_2390:?}");
    };
    assert_eq!(
        m_2390.frame, 2390,
        "checkpoint 2390 fails when 0x3e79c draws the HUD TIME digits"
    );
    assert_eq!(m_2390.observable, "checkpoint screen pixels");
}
