//! Tickets #145 (F73), #146 (F74), #147 (F75), and #148 (F76):
//! - F73 (#145): Service mode from power-on — the reset's self-test branch
//!   (0x3b01e..0x3b02a), the RAM test (0x4834e..0x483f2) placed by summed 68000
//!   instruction costs, the input/DIP/sound test task (0x483f8..0x48642), the
//!   exit trap #3 (0x3b29a..0x3b2b2), and the helpers 0x3b5c8, 0x3bffc /
//!   0x3c014, and 0x3b6b4.
//! - F74 (#146): Service menu (0x48644..0x487f6, 7 items, created and killed as
//!   task 2, with erase helper 0x4929c..0x492d5) and items 1..4: Crosshatch
//!   (0x487f8..0x48856), Color Bar (0x488f8..0x4898e), Character Test 1
//!   (0x48858..0x488f6), and Character ROM Test (0x48990..0x48b68).
//! - F75 (#147): Service item 5 (Sprite / Object ROM Test, 0x48b6a..0x48da2)
//!   and item 7 (Output Port Test, 0x490f8..0x4929a), with helpers 0x3b664
//!   (`aux_ctrl_or`) and 0x3b93c (`clear_words_8`).
//! - F76 (#148): Service item 6 (Scroll Test, 0x48da4..0x490f6), completing all
//!   7 service-menu items.
//!
//! Evidence: docs/reference/service-mode.md. Local-only reference data; skips
//! on a clean checkout.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{
    cpu_owned_addresses, observe_power_on_scenario, run_power_on_scenario_capture, ScratchDir,
    RESET_CLEAR_FRAME0_END,
};
use sf_engine::tasks::{
    Io, SERVICE_RAM_TEST_DONE_FRAME, SERVICE_RAM_TEST_END_CYCLES, SERVICE_RAM_TEST_FRAMES,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_service_01.json";
const SCENARIO_ID: &str = "sf-service-01";
const RUN_A: &str = "../../tools/reference/local/captures/service-01-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/service-01-run-b";

const SCENARIO_02: &str = "../../tools/reference/scenarios/sf_service_02.json";
const SCENARIO_02_ID: &str = "sf-service-02";
const RUN_02_A: &str = "../../tools/reference/local/captures/service-02-run-a";
const RUN_02_B: &str = "../../tools/reference/local/captures/service-02-run-b";

const SCENARIO_03A: &str = "../../tools/reference/scenarios/sf_service_03a.json";
const SCENARIO_03A_ID: &str = "sf-service-03a";
const RUN_03A_A: &str = "../../tools/reference/local/captures/service-03a-run-a";
const RUN_03A_B: &str = "../../tools/reference/local/captures/service-03a-run-b";
const SERVICE_03A_LAST_FRAME: u32 = 2234;

const SCENARIO_03B: &str = "../../tools/reference/scenarios/sf_service_03b.json";
const SCENARIO_03B_ID: &str = "sf-service-03b";
const RUN_03B_A: &str = "../../tools/reference/local/captures/service-03b-run-a";
const RUN_03B_B: &str = "../../tools/reference/local/captures/service-03b-run-b";
const SERVICE_03B_LAST_FRAME: u32 = 910;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// First test-screen frame recorded in F2 (#42, `tests/dip_switches.rs`).
const SERVICE_TEST_SCREEN_FRAME: u32 = 20;
/// Frame at which P1 Up + P1 HK release exits 0x48618 and enters 0x48644 in
/// `sf-service-01`.
const SERVICE_EXIT_FRAME: u32 = 41;

/// Last frame of `sf-service-02` (after entering items 5, 6, and 7 at frames
/// 2386, 2417, and 2448).
const SERVICE_02_LAST_FRAME: u32 = 2448;

/// Expected differing byte ranges `(work_range, video_range)` on the 15 frames
/// (`1..=15`) whose ends fall inside the service RAM test `0x4834e..0x483d4`.
const RAM_TEST_DIFF_RANGES: [(u32, Option<(u32, u32)>, Option<(u32, u32)>); 15] = [
    (1, Some((0xff8000, 0xffa39f)), None),
    (2, Some((0xff8000, 0xffdd7f)), None),
    (3, Some((0xff8000, 0xffffff)), None),
    (4, Some((0xff8000, 0xffffff)), None),
    (5, Some((0xff8000, 0xffffff)), None),
    (6, Some((0xff8000, 0xffffff)), None),
    (7, Some((0xff8000, 0xffffff)), None),
    (8, Some((0xff8000, 0xffffff)), None),
    (9, Some((0xff8000, 0xffffff)), None),
    (10, Some((0xff8000, 0xffffff)), None),
    (11, Some((0xff8000, 0xffffff)), None),
    (12, Some((0xff8000, 0xffffff)), Some((0x800000, 0x800fff))),
    (13, Some((0xff8000, 0xffffff)), Some((0x800000, 0x800fff))),
    (14, Some((0xffbe2c, 0xffffff)), Some((0x800000, 0x800fff))),
    (15, None, Some((0x800000, 0x800fff))),
];

fn present() -> bool {
    [RUN_A, RUN_B, ROMS].iter().all(|p| Path::new(p).is_dir())
}

fn present_02() -> bool {
    [RUN_02_A, RUN_02_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn present_03a() -> bool {
    [RUN_03A_A, RUN_03A_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn present_03b() -> bool {
    [RUN_03B_A, RUN_03B_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn area(addr: u32) -> &'static str {
    match addr {
        0xff8006..=0xff8105 if (addr - 0xff8006) % 16 >= 2 => "task slots' saved SR/PC/USP",
        0xff8106..=0xff8631 => "stack memory",
        _ => "other",
    }
}

fn diff_span(a: &[u8], b: &[u8], base: u32) -> Option<(u32, u32)> {
    let first = a.iter().zip(b.iter()).position(|(x, y)| x != y)?;
    let last = a.iter().zip(b.iter()).rposition(|(x, y)| x != y)?;
    Some((base + first as u32, base + last as u32))
}

/// Acceptance 1 (C-REF): sf-service-01's two runs are byte-for-byte identical
/// across all 213 observation files (42 checkpoints * 5 files + 3 .tsv files).
#[test]
fn sf_service_01_reference_runs_are_identical() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(RUN_A)
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
        let b = Path::new(RUN_B).join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: the two sf-service-01 runs must be identical"
        );
        compared += 1;
    }
    assert_eq!(
        compared, 213,
        "42 full checkpoints * 5 files + 3 .tsv files"
    );
}

/// Acceptance 2 & 3 (C-NAT, C-STOP):
/// - Summed 68000 costs place the end of `0x4834e..0x483d4` at 2,171,478 cycles
///   from power-on (`SERVICE_RAM_TEST_DONE_FRAME = 16`,
///   `SERVICE_RAM_TEST_FRAMES = 15`).
/// - Frame 0 (inside the reset clear `0x3b012..0x3b01a`) matches F1 (#41)'s
///   rule with zero allowance (`RESET_CLEAR_FRAME0_END = 0xffc564`).
/// - On frames `1..=15` (`0x4834e..0x483d4`), `screen_hash`, `latch`,
///   `latch_changed`, `pixels_*.rgba`, and `ram_*_palette.bin` match on every
///   frame, and only work RAM (`0xff8000..=0xffffff`) and video RAM
///   (`0x800000..=0x800fff`) differ on the exact listed byte ranges.
/// - From frame 16 (first frame end after `0x483d8` writes `0xff8003 = 0x55`)
///   through frame 40, full `sf_compare::compare` (`screen_hash`, `latch`,
///   `latch_changed`, checkpoint pixels, video and palette RAM) and all owned
///   work RAM match on every frame; outside owned RAM, only the task slots'
///   saved SR/PC/USP words and stack memory `0xff8106..=0xff8631` differ.
/// - The first test-screen frame equals `20` (`SERVICE_TEST_SCREEN_FRAME`, as
///   recorded in F2 / #42).
/// - Frame 41 exercises exit at `0x48618` (`trap #3` at `0x3b29a`, clearing
///   slot 0 and spawning slot 1 at `0x48644`) and reports `0x48644`.
#[test]
fn service_mode_from_power_on_matches_reference_and_exits_to_0x48644() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    assert_eq!(SERVICE_RAM_TEST_END_CYCLES, 2_171_478);
    assert_eq!(SERVICE_RAM_TEST_DONE_FRAME, 16);
    assert_eq!(SERVICE_RAM_TEST_FRAMES, 15);
    assert_eq!(RESET_CLEAR_FRAME0_END, 0xff8000 + 4_441 * 4);

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let scenario = load_scenario(SCENARIO).unwrap();
    let reference = load_capture(RUN_A).unwrap();

    // Run native capture over 0..=41 (including frame 41's exit to 0x48644).
    let out = ScratchDir::new("sf-engine-service-01");
    let executed = run_power_on_scenario_capture(
        &assets,
        &out,
        SCENARIO_ID,
        Path::new(SCENARIO),
        SERVICE_EXIT_FRAME,
    )
    .expect("native service-mode capture runs through frame 41");
    assert_eq!(executed, (SERVICE_EXIT_FRAME + 1) as u64);
    let candidate = load_capture(&out).unwrap();

    // 1. Frame 0: full comparison with no allowance (F1 #41 rule).
    let scope_frame0 = CompareScope {
        from: Some(0),
        to: Some(0),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_0 = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope_frame0,
    )
    .unwrap();
    assert!(
        matches!(
            outcome_0,
            Outcome::Match {
                frames_compared: 1,
                checkpoints_compared: 1
            }
        ),
        "frame 0 must match with no allowance: {outcome_0:?}"
    );
    let work0 = std::fs::read(Path::new(RUN_A).join("ram_000000_work.bin")).unwrap();
    let split = (RESET_CLEAR_FRAME0_END - 0xff8000) as usize;
    assert!(
        work0[..split].iter().all(|&b| b == 0),
        "cleared prefix at frame 0"
    );
    assert!(
        work0[split..0x6000].iter().all(|&b| b == 0),
        "uncleared work tail at frame 0"
    );
    assert!(
        work0[0x6000..0x8000].iter().all(|&b| b == 0),
        "object RAM at frame 0"
    );

    // 2. Frames 1..=15 (RAM test): screen_hash, latch, latch_changed, pixels,
    // and palette RAM match on every frame; work and video RAM differ only on
    // the listed byte ranges.
    for f in 1..=SERVICE_RAM_TEST_FRAMES {
        let ref_row = reference
            .frames
            .iter()
            .find(|r| r.frame == f as u64)
            .unwrap();
        let cand_row = candidate
            .frames
            .iter()
            .find(|r| r.frame == f as u64)
            .unwrap();
        assert_eq!(
            ref_row.screen_hash, cand_row.screen_hash,
            "frame {f} screen_hash"
        );
        assert_eq!(ref_row.latch, cand_row.latch, "frame {f} latch");
        assert_eq!(
            ref_row.latch_changed, cand_row.latch_changed,
            "frame {f} latch_changed"
        );

        let ref_px = std::fs::read(Path::new(RUN_A).join(format!("pixels_{f:06}.rgba"))).unwrap();
        let cand_px = std::fs::read(out.join(format!("pixels_{f:06}.rgba"))).unwrap();
        assert_eq!(ref_px, cand_px, "frame {f} checkpoint pixels");

        let ref_pal =
            std::fs::read(Path::new(RUN_A).join(format!("ram_{f:06}_palette.bin"))).unwrap();
        let cand_pal = std::fs::read(out.join(format!("ram_{f:06}_palette.bin"))).unwrap();
        assert_eq!(ref_pal, cand_pal, "frame {f} palette RAM");
    }

    // 3. Frames 16..=41: full sf_compare::compare (screen_hash, latch,
    // latch_changed, checkpoint pixels, video and palette RAM) matches on every
    // frame (including frame 41's exit to 0x48644, translated in F74 / #146).
    let scope_16_41 = CompareScope {
        from: Some(SERVICE_RAM_TEST_DONE_FRAME as u64),
        to: Some(SERVICE_EXIT_FRAME as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_16_41 = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope_16_41,
    )
    .unwrap();
    let expected_post_ram = (SERVICE_EXIT_FRAME - SERVICE_RAM_TEST_DONE_FRAME + 1) as u64;
    assert!(
        matches!(
            outcome_16_41,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == expected_post_ram
                && checkpoints_compared == expected_post_ram as usize
        ),
        "frames 16..=41 must match completely under C-NAT: {outcome_16_41:?}"
    );

    // 4. First test-screen frame in sf-service-01 equals F2 (#42)'s
    // SERVICE_TEST_SCREEN_FRAME (20), after video RAM holds only 0x103c at
    // frames 16..=19.
    for f in SERVICE_RAM_TEST_DONE_FRAME..SERVICE_TEST_SCREEN_FRAME {
        let v = std::fs::read(Path::new(RUN_A).join(format!("ram_{f:06}_video.bin"))).unwrap();
        assert!(
            v.chunks_exact(2)
                .all(|w| u16::from_be_bytes([w[0], w[1]]) == 0x103c),
            "frame {f} video RAM must hold only 0x103c"
        );
    }
    let v20 = std::fs::read(Path::new(RUN_A).join("ram_000020_video.bin")).unwrap();
    assert!(
        v20.chunks_exact(2)
            .any(|w| u16::from_be_bytes([w[0], w[1]]) != 0x103c),
        "frame 20 video RAM must show the service test screen"
    );

    // 5. Observe frames 0..=41: verify RAM-test diff ranges on 1..=15, owned
    // and non-owned RAM on 16..=41, and exit at 41 (0x48618 -> 0x48644).
    let owned = cpu_owned_addresses();
    let owned_set: BTreeSet<u32> = owned.iter().map(|&(_, a)| a).collect();
    for required in (0xff8686..0xff8688).chain(0xff872c..0xff886c) {
        assert!(
            owned_set.contains(&required),
            "0x{required:06x} must be in cpu_owned_addresses"
        );
    }

    let mut owned_failures = Vec::new();
    let mut others = Vec::new();
    let mut slot0_at_41 = 0xffu8;
    let mut slot1_at_41 = 0x00u8;
    let (stopped_frame, unrecovered) = observe_power_on_scenario(
        assets.maincpu.clone(),
        Path::new(SCENARIO),
        SERVICE_EXIT_FRAME,
        |m| {
            let f = m.state.frame;
            let image = m.work_image();
            let ref_work =
                std::fs::read(Path::new(RUN_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_A).join(format!("ram_{f:06}_video.bin"))).unwrap();

            if f == 0 {
                assert_eq!(image, ref_work, "frame 0 work+object RAM");
                assert_eq!(m.state.videoram, ref_video, "frame 0 video RAM");
            } else if f <= SERVICE_RAM_TEST_FRAMES {
                let (exp_f, exp_w, exp_v) = RAM_TEST_DIFF_RANGES[(f - 1) as usize];
                assert_eq!(f, exp_f);
                assert_eq!(
                    diff_span(&ref_work, &image, 0xff8000),
                    exp_w,
                    "frame {f} work RAM diff range"
                );
                assert_eq!(
                    diff_span(&ref_video, &m.state.videoram, 0x800000),
                    exp_v,
                    "frame {f} video RAM diff range"
                );
            } else {
                assert_eq!(image[0x0003], 0x55, "frame {f}: 0xff8003 must be 0x55");
                if f == SERVICE_EXIT_FRAME {
                    slot0_at_41 = image[0x0006];
                    slot1_at_41 = image[0x0016];
                }
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    if ref_work[i] != image[i] {
                        owned_failures.push(format!(
                            "frame {f}: {name} at 0x{addr:06x}: reference {:02x}, native {:02x}",
                            ref_work[i], image[i]
                        ));
                    }
                }
                for i in 0..ref_work.len() {
                    if ref_work[i] != image[i] {
                        let addr = 0xff8000 + i as u32;
                        if area(addr) == "other" {
                            others.push(format!(
                                "frame {f}: 0x{addr:06x}: reference {:02x}, native {:02x}",
                                ref_work[i], image[i]
                            ));
                        }
                    }
                }
            }
        },
    )
    .unwrap();
    assert!(
        owned_failures.is_empty(),
        "owned RAM mismatches: {owned_failures:#?}"
    );
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed: {others:#?}"
    );
    assert_eq!(
        (slot0_at_41, slot1_at_41),
        (0, 8),
        "frame 41 must clear slot 0 via trap #3 at 0x48618 and enter slot 1 at 0x48644 (sleeping 6 frames via 0x3b532 / trap #2 at 0x48716)"
    );
    assert_eq!(
        (stopped_frame, unrecovered),
        (SERVICE_EXIT_FRAME, Vec::<u32>::new()),
        "frame 41 must complete with no unrecovered PCs once 0x48644 is translated"
    );
}

/// Negative control (C-NEG) & Acceptance 4: MAME's `unmap_value_high` (`0xffff`)
/// on the unmapped ports `0xc00004`, `0xc00006`, and `0xc0000e` (`0x484de..0x484f2`
/// does not invert `D2 == 5` or `D2 == 4`, i.e. `0xc00004` and `0xc00006`, while
/// inverting `0xc0000e`). Zeroing `unmap_c00004` flips the formatted row at
/// `0xff8776..0xff8798` from `0x0001` cells to `0x0000` cells at frame 20 and is
/// caught by owned-RAM comparison.
#[test]
fn negative_control_wrong_unmapped_port_value_caught_at_frame_20() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let bad_io = Io {
        dsw1: 0x5fff,
        dsw2: 0xffff,
        unmap_c00004: 0x0000,
        ..Io::idle()
    };
    let mut m = sf_engine::run::PowerOnMachine::with_io(assets.maincpu, bad_io);
    for _ in 0..SERVICE_TEST_SCREEN_FRAME {
        m.step();
    }
    let ref_work20 = std::fs::read(Path::new(RUN_A).join("ram_000020_work.bin")).unwrap();
    let ref_video20 = std::fs::read(Path::new(RUN_A).join("ram_000020_video.bin")).unwrap();
    let nat_work20 = m.work_image();
    // Row 2 of the 8x17 block (idx == 2, 0xc00004) starts at 0xff8732 + 2 * 34 = 0xff8776.
    assert_ne!(
        &nat_work20[0x0776..0x0798],
        &ref_work20[0x0776..0x0798],
        "wrong unmapped port 0xc00004 value must be caught in 0xff8776..0xff8798 at frame 20"
    );
    assert_ne!(
        m.state.videoram, ref_video20,
        "wrong unmapped port 0xc00004 value must be caught in video RAM at frame 20"
    );
}

/// Ticket #146 (F74) Acceptance 1 (C-REF): `sf-service-02`'s two reference runs
/// are byte-for-byte identical across all 12,248 observation files (2,449
/// checkpoints * 5 files + 3 `.tsv` files).
#[test]
fn sf_service_02_reference_runs_are_identical() {
    if !present_02() {
        eprintln!("skipping: local sf-service-02 captures/ROMs not present");
        return;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(RUN_02_A)
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
        let b = Path::new(RUN_02_B).join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: the two sf-service-02 runs must be identical"
        );
        compared += 1;
    }
    assert_eq!(
        compared, 12_248,
        "2,449 full checkpoints * 5 files + 3 .tsv files"
    );
}

/// Ticket #146 (F74) Acceptance 2 & 3 (C-NAT, C-STOP), updated after F75 (#147)
/// and F76 (#148) translated items 5..7:
/// - C-NAT across the entire `sf-service-02` scenario (`0..=2448`,
///   `SERVICE_02_LAST_FRAME`):
///   - Frame 0 matches with zero allowance (`RESET_CLEAR_FRAME0_END`).
///   - Frames `1..=15` match on `screen_hash`, `latch`, `latch_changed`,
///     checkpoint pixels, and palette RAM, differing on work/video RAM only on
///     `RAM_TEST_DIFF_RANGES`.
///   - Frames `16..=2448` match completely under `sf_compare::compare`
///     (`screen_hash`, `latch`, `latch_changed`, checkpoint pixels, video RAM,
///     and palette RAM) and across all CPU-owned work RAM; outside owned RAM,
///     only the task slots' saved SR/PC/USP words and stack memory
///     `0xff8106..=0xff8631` differ.
/// - Items 5 (`0x48b6a`), 6 (`0x48da4`), and 7 (`0x490f8`) are entered on frames
///   2386, 2417, and 2448 and now complete with zero unrecovered PCs.
#[test]
fn service_menu_and_first_four_items_match_reference_and_report_items_5_to_7() {
    if !present_02() {
        eprintln!("skipping: local sf-service-02 captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let scenario = load_scenario(SCENARIO_02).unwrap();
    let reference = load_capture(RUN_02_A).unwrap();

    // Run native capture over 0..=2448 (the entire sf-service-02 scenario).
    let out = ScratchDir::new("sf-engine-service-02");
    let executed = run_power_on_scenario_capture(
        &assets,
        &out,
        SCENARIO_02_ID,
        Path::new(SCENARIO_02),
        SERVICE_02_LAST_FRAME,
    )
    .expect("native sf-service-02 capture runs through frame 2448");
    assert_eq!(executed, (SERVICE_02_LAST_FRAME + 1) as u64);
    let candidate = load_capture(&out).unwrap();

    // 1. Frame 0: full comparison with no allowance.
    let scope_frame0 = CompareScope {
        from: Some(0),
        to: Some(0),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_0 = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope_frame0,
    )
    .unwrap();
    assert!(
        matches!(
            outcome_0,
            Outcome::Match {
                frames_compared: 1,
                checkpoints_compared: 1
            }
        ),
        "frame 0 must match with no allowance: {outcome_0:?}"
    );

    // 2. Frames 1..=15 (RAM test): screen_hash, latch, latch_changed, pixels,
    // and palette RAM match on every frame.
    for f in 1..=SERVICE_RAM_TEST_FRAMES {
        let ref_row = reference
            .frames
            .iter()
            .find(|r| r.frame == f as u64)
            .unwrap();
        let cand_row = candidate
            .frames
            .iter()
            .find(|r| r.frame == f as u64)
            .unwrap();
        assert_eq!(
            ref_row.screen_hash, cand_row.screen_hash,
            "frame {f} screen_hash"
        );
        assert_eq!(ref_row.latch, cand_row.latch, "frame {f} latch");
        assert_eq!(
            ref_row.latch_changed, cand_row.latch_changed,
            "frame {f} latch_changed"
        );

        let ref_px =
            std::fs::read(Path::new(RUN_02_A).join(format!("pixels_{f:06}.rgba"))).unwrap();
        let cand_px = std::fs::read(out.join(format!("pixels_{f:06}.rgba"))).unwrap();
        assert_eq!(ref_px, cand_px, "frame {f} checkpoint pixels");

        let ref_pal =
            std::fs::read(Path::new(RUN_02_A).join(format!("ram_{f:06}_palette.bin"))).unwrap();
        let cand_pal = std::fs::read(out.join(format!("ram_{f:06}_palette.bin"))).unwrap();
        assert_eq!(ref_pal, cand_pal, "frame {f} palette RAM");
    }

    // 3. Frames 16..=2448: full sf_compare::compare matches on every frame.
    let scope_16_2448 = CompareScope {
        from: Some(SERVICE_RAM_TEST_DONE_FRAME as u64),
        to: Some(SERVICE_02_LAST_FRAME as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_16_2448 = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope_16_2448,
    )
    .unwrap();
    let expected_post_ram = (SERVICE_02_LAST_FRAME - SERVICE_RAM_TEST_DONE_FRAME + 1) as u64;
    assert!(
        matches!(
            outcome_16_2448,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == expected_post_ram
                && checkpoints_compared == expected_post_ram as usize
        ),
        "frames 16..=2448 must match completely under C-NAT: {outcome_16_2448:?}"
    );

    // 4. Observe frames 0..=2448: verify RAM-test diff ranges on 1..=15, owned
    // and non-owned RAM on 16..=2448, and zero unrecovered PCs across all 7
    // service items.
    let owned = cpu_owned_addresses();
    let mut owned_failures = Vec::new();
    let mut others = Vec::new();
    let (stopped_frame, first_unrecovered) = observe_power_on_scenario(
        assets.maincpu.clone(),
        Path::new(SCENARIO_02),
        SERVICE_02_LAST_FRAME,
        |m| {
            let f = m.state.frame;
            let image = m.work_image();
            let ref_work =
                std::fs::read(Path::new(RUN_02_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_02_A).join(format!("ram_{f:06}_video.bin"))).unwrap();

            if f == 0 {
                assert_eq!(image, ref_work, "frame 0 work+object RAM");
                assert_eq!(m.state.videoram, ref_video, "frame 0 video RAM");
            } else if f <= SERVICE_RAM_TEST_FRAMES {
                let (exp_f, exp_w, exp_v) = RAM_TEST_DIFF_RANGES[(f - 1) as usize];
                assert_eq!(f, exp_f);
                assert_eq!(
                    diff_span(&ref_work, &image, 0xff8000),
                    exp_w,
                    "frame {f} work RAM diff range"
                );
                assert_eq!(
                    diff_span(&ref_video, &m.state.videoram, 0x800000),
                    exp_v,
                    "frame {f} video RAM diff range"
                );
            } else {
                assert_eq!(image[0x0003], 0x55, "frame {f}: 0xff8003 must be 0x55");
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    if ref_work[i] != image[i] {
                        owned_failures.push(format!(
                            "frame {f}: {name} at 0x{addr:06x}: reference {:02x}, native {:02x}",
                            ref_work[i], image[i]
                        ));
                    }
                }
                for i in 0..ref_work.len() {
                    if ref_work[i] != image[i] {
                        let addr = 0xff8000 + i as u32;
                        if area(addr) == "other" {
                            others.push(format!(
                                "frame {f}: 0x{addr:06x}: reference {:02x}, native {:02x}",
                                ref_work[i], image[i]
                            ));
                        }
                    }
                }
            }
        },
    )
    .unwrap();
    assert!(
        owned_failures.is_empty(),
        "owned RAM mismatches: {owned_failures:#?}"
    );
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed: {others:#?}"
    );
    assert_eq!(
        (stopped_frame, first_unrecovered),
        (SERVICE_02_LAST_FRAME, Vec::<u32>::new()),
        "sf-service-02 must complete through frame 2448 with zero unrecovered PCs"
    );
}

/// Ticket #146 (F74) Negative control (C-NEG): omitting Item 4's P1 Up press at
/// frame 1812 (active at frame 1813, where `0x48a40..0x48a72` increments the
/// 2-bit character page word `0xff884c` from `0` to `1` and `0x48b08` updates
/// `0x8003a2` and the `16 x 16` table at `0x800624`) matches through frame 1812
/// and fails at the exact first differing frame 1813 in both CPU-owned work RAM
/// (`0xff884c`) and video RAM (`0x8003a2` and `0x800624`).
#[test]
fn negative_control_wrong_char_rom_page_caught_in_item_4() {
    if !present_02() {
        eprintln!("skipping: local sf-service-02 captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let dir = ScratchDir::new("sf-service-02-neg-scen");
    std::fs::create_dir_all(&*dir).unwrap();
    let neg_scen = dir.join("sf_service_02_no_item4_up.json");
    let raw = std::fs::read_to_string(SCENARIO_02).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let inputs = json["inputs"].as_array_mut().unwrap();
    let before_len = inputs.len();
    inputs.retain(|ev| !(ev["frame"] == 1812 && ev["press"] == "p1_up"));
    assert_eq!(
        inputs.len() + 1,
        before_len,
        "removed frame-1812 p1_up from item 4"
    );
    std::fs::write(&neg_scen, serde_json::to_string(&json).unwrap()).unwrap();

    let mut matched_1812 = false;
    let mut caught_1813_work = false;
    let mut caught_1813_video = false;
    let _ = observe_power_on_scenario(assets.maincpu, &neg_scen, 1813, |m| {
        let f = m.state.frame;
        if f == 1812 {
            let ref_work = std::fs::read(Path::new(RUN_02_A).join("ram_001812_work.bin")).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_02_A).join("ram_001812_video.bin")).unwrap();
            let image = m.work_image();
            matched_1812 =
                image[0x084c..0x0850] == ref_work[0x084c..0x0850] && m.state.videoram == ref_video;
        } else if f == 1813 {
            let ref_work = std::fs::read(Path::new(RUN_02_A).join("ram_001813_work.bin")).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_02_A).join("ram_001813_video.bin")).unwrap();
            let image = m.work_image();
            caught_1813_work = image[0x084c..0x084e] != ref_work[0x084c..0x084e];
            caught_1813_video = m.state.videoram[0x03a2..0x03a6] != ref_video[0x03a2..0x03a6]
                && m.state.videoram[0x0624..0x0626] != ref_video[0x0624..0x0626];
        }
    })
    .unwrap();
    assert!(
        matched_1812,
        "mutated scenario must match through frame 1812"
    );
    assert!(
        caught_1813_work,
        "missing P1 Up in item 4 must fail on 0xff884c at frame 1813"
    );
    assert!(
        caught_1813_video,
        "missing P1 Up in item 4 must fail on 0x8003a2 and 0x800624 at frame 1813"
    );
}

/// Ticket #147 (F75) Acceptance 1 (C-REF): `sf-service-03a`'s two reference runs
/// are byte-for-byte identical across all 11,178 observation files (2,235
/// checkpoints * 5 files + 3 `.tsv` files).
#[test]
fn sf_service_03a_reference_runs_are_identical() {
    if !present_03a() {
        eprintln!("skipping: local sf-service-03a captures/ROMs not present");
        return;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(RUN_03A_A)
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
        let b = Path::new(RUN_03A_B).join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: the two sf-service-03a runs must be identical"
        );
        compared += 1;
    }
    assert_eq!(
        compared, 11_178,
        "2,235 full checkpoints * 5 files + 3 .tsv files"
    );
}

/// Ticket #147 (F75) Acceptance 2 (C-NAT): `sf-service-03a` (reaching item 5
/// `0x48b6a` at frame 130, holding each of the 8 P1/P2 directions alone for 60
/// frames, exiting item 5 at frame 644, reaching item 7 `0x490f8` at frame 693,
/// running for 1,537 frames across all 25 entries of `0x491ee..0x49283` and the
/// `0x96 -> 0` wrap, and exiting item 7 at frame 2230) matches the reference on
/// every frame `16..=2234` across `screen_hash`, `latch`, `latch_changed`,
/// checkpoint pixels, video RAM, palette RAM, and all CPU-owned RAM (including
/// object RAM `0xffe000..=0xffffff` and the output copies `0xff8684 / 0xff8686`).
#[test]
fn service_sprite_test_and_output_cycle_match_reference() {
    if !present_03a() {
        eprintln!("skipping: local sf-service-03a captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let scenario = load_scenario(SCENARIO_03A).unwrap();
    let reference = load_capture(RUN_03A_A).unwrap();

    let out = ScratchDir::new("sf-engine-service-03a");
    let executed = run_power_on_scenario_capture(
        &assets,
        &out,
        SCENARIO_03A_ID,
        Path::new(SCENARIO_03A),
        SERVICE_03A_LAST_FRAME,
    )
    .expect("native sf-service-03a capture runs through frame 2234");
    assert_eq!(executed, (SERVICE_03A_LAST_FRAME + 1) as u64);
    let candidate = load_capture(&out).unwrap();

    // 1. Frame 0: full comparison with no allowance.
    let scope_frame0 = CompareScope {
        from: Some(0),
        to: Some(0),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_0 = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope_frame0,
    )
    .unwrap();
    assert!(
        matches!(
            outcome_0,
            Outcome::Match {
                frames_compared: 1,
                checkpoints_compared: 1
            }
        ),
        "frame 0 must match with no allowance: {outcome_0:?}"
    );

    // 2. Frames 1..=15 (RAM test): screen_hash, latch, latch_changed, pixels,
    // and palette RAM match on every frame.
    for f in 1..=SERVICE_RAM_TEST_FRAMES {
        let ref_row = reference
            .frames
            .iter()
            .find(|r| r.frame == f as u64)
            .unwrap();
        let cand_row = candidate
            .frames
            .iter()
            .find(|r| r.frame == f as u64)
            .unwrap();
        assert_eq!(
            ref_row.screen_hash, cand_row.screen_hash,
            "frame {f} screen_hash"
        );
        assert_eq!(ref_row.latch, cand_row.latch, "frame {f} latch");
        assert_eq!(
            ref_row.latch_changed, cand_row.latch_changed,
            "frame {f} latch_changed"
        );

        let ref_px =
            std::fs::read(Path::new(RUN_03A_A).join(format!("pixels_{f:06}.rgba"))).unwrap();
        let cand_px = std::fs::read(out.join(format!("pixels_{f:06}.rgba"))).unwrap();
        assert_eq!(ref_px, cand_px, "frame {f} checkpoint pixels");

        let ref_pal =
            std::fs::read(Path::new(RUN_03A_A).join(format!("ram_{f:06}_palette.bin"))).unwrap();
        let cand_pal = std::fs::read(out.join(format!("ram_{f:06}_palette.bin"))).unwrap();
        assert_eq!(ref_pal, cand_pal, "frame {f} palette RAM");
    }

    // 3. Frames 16..=2234: full sf_compare::compare matches on every frame.
    let scope_16_2234 = CompareScope {
        from: Some(SERVICE_RAM_TEST_DONE_FRAME as u64),
        to: Some(SERVICE_03A_LAST_FRAME as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_16_2234 = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope_16_2234,
    )
    .unwrap();
    let expected_post_ram = (SERVICE_03A_LAST_FRAME - SERVICE_RAM_TEST_DONE_FRAME + 1) as u64;
    assert!(
        matches!(
            outcome_16_2234,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == expected_post_ram
                && checkpoints_compared == expected_post_ram as usize
        ),
        "frames 16..=2234 must match completely under C-NAT: {outcome_16_2234:?}"
    );

    // 4. Observe frames 0..=2234: verify RAM-test diff ranges on 1..=15, owned
    // and non-owned RAM (including object RAM 0xffe000..=0xffffff and output
    // copies 0xff8684 / 0xff8686) on 16..=2234, and zero unrecovered PCs.
    let owned = cpu_owned_addresses();
    let owned_set: BTreeSet<u32> = owned.iter().map(|&(_, a)| a).collect();
    for required in (0xff8684..0xff8688).chain(0xffe000..0x1000000) {
        assert!(
            owned_set.contains(&required),
            "0x{required:06x} must be in cpu_owned_addresses"
        );
    }

    let mut owned_failures = Vec::new();
    let mut others = Vec::new();
    let (stopped_frame, unrecovered) = observe_power_on_scenario(
        assets.maincpu.clone(),
        Path::new(SCENARIO_03A),
        SERVICE_03A_LAST_FRAME,
        |m| {
            let f = m.state.frame;
            let image = m.work_image();
            let ref_work =
                std::fs::read(Path::new(RUN_03A_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_03A_A).join(format!("ram_{f:06}_video.bin"))).unwrap();

            if f == 0 {
                assert_eq!(image, ref_work, "frame 0 work+object RAM");
                assert_eq!(m.state.videoram, ref_video, "frame 0 video RAM");
            } else if f <= SERVICE_RAM_TEST_FRAMES {
                let (exp_f, exp_w, exp_v) = RAM_TEST_DIFF_RANGES[(f - 1) as usize];
                assert_eq!(f, exp_f);
                assert_eq!(
                    diff_span(&ref_work, &image, 0xff8000),
                    exp_w,
                    "frame {f} work RAM diff range"
                );
                assert_eq!(
                    diff_span(&ref_video, &m.state.videoram, 0x800000),
                    exp_v,
                    "frame {f} video RAM diff range"
                );
            } else {
                assert_eq!(image[0x0003], 0x55, "frame {f}: 0xff8003 must be 0x55");
                assert_eq!(
                    &image[0x6000..0x8000],
                    &ref_work[0x6000..0x8000],
                    "frame {f}: object RAM 0xffe000..0xffffff must match"
                );
                assert_eq!(
                    &image[0x0684..0x0688],
                    &ref_work[0x0684..0x0688],
                    "frame {f}: output copies 0xff8684..0xff8688 must match"
                );
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    if ref_work[i] != image[i] {
                        owned_failures.push(format!(
                            "frame {f}: {name} at 0x{addr:06x}: reference {:02x}, native {:02x}",
                            ref_work[i], image[i]
                        ));
                    }
                }
                for i in 0..ref_work.len() {
                    if ref_work[i] != image[i] {
                        let addr = 0xff8000 + i as u32;
                        if area(addr) == "other" {
                            others.push(format!(
                                "frame {f}: 0x{addr:06x}: reference {:02x}, native {:02x}",
                                ref_work[i], image[i]
                            ));
                        }
                    }
                }
            }
        },
    )
    .unwrap();
    assert!(
        owned_failures.is_empty(),
        "owned RAM mismatches: {owned_failures:#?}"
    );
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed: {others:#?}"
    );
    assert_eq!(
        (stopped_frame, unrecovered),
        (SERVICE_03A_LAST_FRAME, Vec::<u32>::new()),
        "sf-service-03a must complete through frame 2234 with zero unrecovered PCs"
    );
}

/// Ticket #147 (F75) Negative control (C-NEG): omitting Item 5's 5th P1 Up press
/// at frame 142 (active at frame 143, where `0x48c82..0x48cd0` advances
/// `0xff8851` past `0x80`, increments `0xff8850` from `0` to `1`, resets
/// `0xff8851` to `0`, writes `01` at `0x8003a2`, and updates row headers at
/// `0x800618` and sprite `+0` words at `0xffe080`) matches through frame 142 and
/// fails at the exact first differing frame 143 across CPU-owned work RAM
/// (`0xff8850..0xff8852`), object RAM (`0xffe080`), and video RAM (`0x8003a2`).
#[test]
fn negative_control_wrong_sprite_page_caught_in_item_5() {
    if !present_03a() {
        eprintln!("skipping: local sf-service-03a captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let dir = ScratchDir::new("sf-service-03a-neg-scen");
    std::fs::create_dir_all(&*dir).unwrap();
    let neg_scen = dir.join("sf_service_03a_no_item5_up5.json");
    let raw = std::fs::read_to_string(SCENARIO_03A).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let inputs = json["inputs"].as_array_mut().unwrap();
    let before_len = inputs.len();
    inputs.retain(|ev| !(ev["frame"] == 142 && ev["press"] == "p1_up"));
    assert_eq!(
        inputs.len() + 1,
        before_len,
        "removed frame-142 p1_up from item 5"
    );
    std::fs::write(&neg_scen, serde_json::to_string(&json).unwrap()).unwrap();

    let mut matched_142 = false;
    let mut caught_143_work = false;
    let mut caught_143_obj = false;
    let mut caught_143_video = false;
    let _ = observe_power_on_scenario(assets.maincpu, &neg_scen, 143, |m| {
        let f = m.state.frame;
        if f == 142 {
            let ref_work = std::fs::read(Path::new(RUN_03A_A).join("ram_000142_work.bin")).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_03A_A).join("ram_000142_video.bin")).unwrap();
            let image = m.work_image();
            matched_142 = image[0x0850..0x0852] == ref_work[0x0850..0x0852]
                && image[0x6080..0x6082] == ref_work[0x6080..0x6082]
                && m.state.videoram == ref_video;
        } else if f == 143 {
            let ref_work = std::fs::read(Path::new(RUN_03A_A).join("ram_000143_work.bin")).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_03A_A).join("ram_000143_video.bin")).unwrap();
            let image = m.work_image();
            caught_143_work = image[0x0850..0x0852] != ref_work[0x0850..0x0852];
            caught_143_obj = image[0x6080..0x6082] != ref_work[0x6080..0x6082];
            caught_143_video = m.state.videoram[0x03a2..0x03a6] != ref_video[0x03a2..0x03a6];
        }
    })
    .unwrap();
    assert!(matched_142, "mutated scenario must match through frame 142");
    assert!(
        caught_143_work,
        "missing 5th P1 Up in item 5 must fail on 0xff8850..0xff8852 at frame 143"
    );
    assert!(
        caught_143_obj,
        "missing 5th P1 Up in item 5 must fail on sprite 0 (+0 at 0xffe080) at frame 143"
    );
    assert!(
        caught_143_video,
        "missing 5th P1 Up in item 5 must fail on 0x8003a2 at frame 143"
    );
}

/// Ticket #148 (F76) Acceptance 1 (C-REF): `sf-service-03b`'s two reference runs
/// are byte-for-byte identical across all 4,558 observation files (911
/// checkpoints * 5 files + 3 `.tsv` files).
#[test]
fn sf_service_03b_reference_runs_are_identical() {
    if !present_03b() {
        eprintln!("skipping: local sf-service-03b captures/ROMs not present");
        return;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(RUN_03B_A)
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
        let b = Path::new(RUN_03B_B).join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: the two sf-service-03b runs must be identical"
        );
        compared += 1;
    }
    assert_eq!(
        compared, 4_558,
        "911 full checkpoints * 5 files + 3 .tsv files"
    );
}

/// Ticket #148 (F76) Acceptance 2 (C-NAT): `sf-service-03b` (reaching item 6
/// `0x48da4` at frame 70, holding each of the 8 P1/P2 cardinal directions alone
/// for 60 frames, holding a diagonal `p1_up + p1_left` at frames 570..572,
/// pulsing `p1_up + p2_up` across the BG `0x1500` and FG `0x1b00` palette
/// thresholds through frame 900, and exiting item 6 at frame 906) matches the
/// reference on every frame `16..=910` across `screen_hash`, `latch`,
/// `latch_changed`, checkpoint pixels, video RAM, palette RAM, and all
/// CPU-owned RAM (including the scroll control blocks `0xff8852..0xff8862` and
/// the scroll copies `0xffc680 / 0xffc682`).
#[test]
fn service_scroll_test_matches_reference() {
    if !present_03b() {
        eprintln!("skipping: local sf-service-03b captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let scenario = load_scenario(SCENARIO_03B).unwrap();
    let reference = load_capture(RUN_03B_A).unwrap();

    let out = ScratchDir::new("sf-engine-service-03b");
    let executed = run_power_on_scenario_capture(
        &assets,
        &out,
        SCENARIO_03B_ID,
        Path::new(SCENARIO_03B),
        SERVICE_03B_LAST_FRAME,
    )
    .expect("native sf-service-03b capture runs through frame 910");
    assert_eq!(executed, (SERVICE_03B_LAST_FRAME + 1) as u64);
    let candidate = load_capture(&out).unwrap();

    // 1. Frame 0: full comparison with no allowance.
    let scope_frame0 = CompareScope {
        from: Some(0),
        to: Some(0),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_0 = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope_frame0,
    )
    .unwrap();
    assert!(
        matches!(
            outcome_0,
            Outcome::Match {
                frames_compared: 1,
                checkpoints_compared: 1
            }
        ),
        "frame 0 must match with no allowance: {outcome_0:?}"
    );

    // 2. Frames 1..=15 (RAM test): screen_hash, latch, latch_changed, pixels,
    // and palette RAM match on every frame.
    for f in 1..=SERVICE_RAM_TEST_FRAMES {
        let ref_row = reference
            .frames
            .iter()
            .find(|r| r.frame == f as u64)
            .unwrap();
        let cand_row = candidate
            .frames
            .iter()
            .find(|r| r.frame == f as u64)
            .unwrap();
        assert_eq!(
            ref_row.screen_hash, cand_row.screen_hash,
            "frame {f} screen_hash"
        );
        assert_eq!(ref_row.latch, cand_row.latch, "frame {f} latch");
        assert_eq!(
            ref_row.latch_changed, cand_row.latch_changed,
            "frame {f} latch_changed"
        );

        let ref_px =
            std::fs::read(Path::new(RUN_03B_A).join(format!("pixels_{f:06}.rgba"))).unwrap();
        let cand_px = std::fs::read(out.join(format!("pixels_{f:06}.rgba"))).unwrap();
        assert_eq!(ref_px, cand_px, "frame {f} checkpoint pixels");

        let ref_pal =
            std::fs::read(Path::new(RUN_03B_A).join(format!("ram_{f:06}_palette.bin"))).unwrap();
        let cand_pal = std::fs::read(out.join(format!("ram_{f:06}_palette.bin"))).unwrap();
        assert_eq!(ref_pal, cand_pal, "frame {f} palette RAM");
    }

    // 3. Frames 16..=910: full sf_compare::compare matches on every frame.
    let scope_16_910 = CompareScope {
        from: Some(SERVICE_RAM_TEST_DONE_FRAME as u64),
        to: Some(SERVICE_03B_LAST_FRAME as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_16_910 = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope_16_910,
    )
    .unwrap();
    let expected_post_ram = (SERVICE_03B_LAST_FRAME - SERVICE_RAM_TEST_DONE_FRAME + 1) as u64;
    assert!(
        matches!(
            outcome_16_910,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == expected_post_ram
                && checkpoints_compared == expected_post_ram as usize
        ),
        "frames 16..=910 must match completely under C-NAT: {outcome_16_910:?}"
    );

    // 4. Observe frames 0..=910: verify RAM-test diff ranges on 1..=15, owned
    // and non-owned RAM (including the scroll control blocks 0xff8852..0xff8862
    // and scroll copies 0xffc680..0xffc684) on 16..=910, and zero unrecovered
    // PCs.
    let owned = cpu_owned_addresses();
    let owned_set: BTreeSet<u32> = owned.iter().map(|&(_, a)| a).collect();
    for required in (0xff8852..0xff8862).chain(0xffc680..0xffc684) {
        assert!(
            owned_set.contains(&required),
            "0x{required:06x} must be in cpu_owned_addresses"
        );
    }

    let mut owned_failures = Vec::new();
    let mut others = Vec::new();
    let (stopped_frame, unrecovered) = observe_power_on_scenario(
        assets.maincpu.clone(),
        Path::new(SCENARIO_03B),
        SERVICE_03B_LAST_FRAME,
        |m| {
            let f = m.state.frame;
            let image = m.work_image();
            let ref_work =
                std::fs::read(Path::new(RUN_03B_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_03B_A).join(format!("ram_{f:06}_video.bin"))).unwrap();

            if f == 0 {
                assert_eq!(image, ref_work, "frame 0 work+object RAM");
                assert_eq!(m.state.videoram, ref_video, "frame 0 video RAM");
            } else if f <= SERVICE_RAM_TEST_FRAMES {
                let (exp_f, exp_w, exp_v) = RAM_TEST_DIFF_RANGES[(f - 1) as usize];
                assert_eq!(f, exp_f);
                assert_eq!(
                    diff_span(&ref_work, &image, 0xff8000),
                    exp_w,
                    "frame {f} work RAM diff range"
                );
                assert_eq!(
                    diff_span(&ref_video, &m.state.videoram, 0x800000),
                    exp_v,
                    "frame {f} video RAM diff range"
                );
            } else {
                assert_eq!(image[0x0003], 0x55, "frame {f}: 0xff8003 must be 0x55");
                assert_eq!(
                    &image[0x0852..0x0862],
                    &ref_work[0x0852..0x0862],
                    "frame {f}: scroll control blocks 0xff8852..0xff8862 must match"
                );
                assert_eq!(
                    &image[0x4680..0x4684],
                    &ref_work[0x4680..0x4684],
                    "frame {f}: scroll copies 0xffc680..0xffc684 must match"
                );
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    if ref_work[i] != image[i] {
                        owned_failures.push(format!(
                            "frame {f}: {name} at 0x{addr:06x}: reference {:02x}, native {:02x}",
                            ref_work[i], image[i]
                        ));
                    }
                }
                for i in 0..ref_work.len() {
                    if ref_work[i] != image[i] {
                        let addr = 0xff8000 + i as u32;
                        if area(addr) == "other" {
                            others.push(format!(
                                "frame {f}: 0x{addr:06x}: reference {:02x}, native {:02x}",
                                ref_work[i], image[i]
                            ));
                        }
                    }
                }
            }
        },
    )
    .unwrap();
    assert!(
        owned_failures.is_empty(),
        "owned RAM mismatches: {owned_failures:#?}"
    );
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed: {others:#?}"
    );
    assert_eq!(
        (stopped_frame, unrecovered),
        (SERVICE_03B_LAST_FRAME, Vec::<u32>::new()),
        "sf-service-03b must complete through frame 910 with zero unrecovered PCs"
    );
}

/// Ticket #148 (F76) Negative control (C-NEG): omitting Item 6's P1 Down press
/// at frame 74 (active at frame 75, where `0x48ff4..0x4906b` sets
/// `0xff8852..0xff8856 = [0x01, 0xc0, 0x04, 0x00]` and `0x48ea0..0x48ea6`
/// decrements the delay at `0xff8852` to `0x00`, and at frame 76 where
/// `0x48eae..0x48ec8` consumes `0xff8853 = 0xc0` (`-64`), advances
/// `FG_SCROLL_COPY` (`0xffc680`) from `0x0000` to `0xffc0`, and writes `FFC0` /
/// `00C0` at `0x80069c..0x8006b0`) matches through frame 74, fails at the exact
/// first differing frame 75 on CPU-owned work RAM (`0xff8852..0xff8856`), and
/// fails at frame 76 on `0xffc680..0xffc682` and video RAM (`0x80069c..0x8006b0`).
#[test]
fn negative_control_wrong_fg_scroll_step_caught_in_item_6() {
    if !present_03b() {
        eprintln!("skipping: local sf-service-03b captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let dir = ScratchDir::new("sf-service-03b-neg-scen");
    std::fs::create_dir_all(&*dir).unwrap();
    let neg_scen = dir.join("sf_service_03b_no_item6_down.json");
    let raw = std::fs::read_to_string(SCENARIO_03B).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let inputs = json["inputs"].as_array_mut().unwrap();
    let before_len = inputs.len();
    inputs.retain(|ev| !(ev["frame"] == 74 && ev["press"] == "p1_down"));
    assert_eq!(
        inputs.len() + 1,
        before_len,
        "removed frame-74 p1_down from item 6"
    );
    std::fs::write(&neg_scen, serde_json::to_string(&json).unwrap()).unwrap();

    let mut matched_74 = false;
    let mut caught_75_ctrl = false;
    let mut caught_76_scroll = false;
    let mut caught_76_video = false;
    let _ = observe_power_on_scenario(assets.maincpu, &neg_scen, 76, |m| {
        let f = m.state.frame;
        if f == 74 {
            let ref_work = std::fs::read(Path::new(RUN_03B_A).join("ram_000074_work.bin")).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_03B_A).join("ram_000074_video.bin")).unwrap();
            let image = m.work_image();
            matched_74 = image[0x0852..0x0862] == ref_work[0x0852..0x0862]
                && image[0x4680..0x4684] == ref_work[0x4680..0x4684]
                && m.state.videoram == ref_video;
        } else if f == 75 {
            let ref_work = std::fs::read(Path::new(RUN_03B_A).join("ram_000075_work.bin")).unwrap();
            let image = m.work_image();
            caught_75_ctrl = image[0x0852..0x0856] != ref_work[0x0852..0x0856];
        } else if f == 76 {
            let ref_work = std::fs::read(Path::new(RUN_03B_A).join("ram_000076_work.bin")).unwrap();
            let ref_video =
                std::fs::read(Path::new(RUN_03B_A).join("ram_000076_video.bin")).unwrap();
            let image = m.work_image();
            caught_76_scroll = image[0x4680..0x4682] != ref_work[0x4680..0x4682];
            caught_76_video = m.state.videoram[0x069c..0x06b0] != ref_video[0x069c..0x06b0];
        }
    })
    .unwrap();
    assert!(matched_74, "mutated scenario must match through frame 74");
    assert!(
        caught_75_ctrl,
        "missing P1 Down in item 6 must fail on 0xff8852..0xff8856 at frame 75"
    );
    assert!(
        caught_76_scroll,
        "missing P1 Down in item 6 must fail on 0xffc680..0xffc682 at frame 76"
    );
    assert!(
        caught_76_video,
        "missing P1 Down in item 6 must fail on 0x80069c..0x8006b0 at frame 76"
    );
}
