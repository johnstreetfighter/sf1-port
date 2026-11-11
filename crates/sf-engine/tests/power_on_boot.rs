//! Tickets #41 (F1) & #55 (F15): power-on reset, task spawning, rescan
//! scheduler, the boot task's first half (0x3c0c4..0x3c20a) over frames 0..=7,
//! the warning typewriter and 240-frame hold (0x3c210..0x3c290) over frames
//! 8..=794, and the join into the attract wall, title and INSERT COIN loop
//! (0x3ccae..0x3ce4c, 0x3f374..0x3f3b0, 0x3f4de, 0x4813e) over frames
//! 794..=1647 from power-on with no fixture.
//!
//! Evidence: docs/reference/power-on.md, docs/reference/attract.md. Local-only
//! reference data; skips on a clean checkout.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{
    cpu_owned_addresses, observe_power_on, run_power_on_scenario_capture, PowerOnMachine,
    ScratchDir, BOOT_SETUP_LAST, RESET_CLEAR_FRAME0_END, WARNING_TITLE_LAST,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_boot_01.json";
const SCENARIO_ID: &str = "sf-boot-01";
const ATTRACT_SCENARIO: &str = "../../tools/reference/scenarios/sf_attract_01.json";
const ATTRACT_SCENARIO_ID: &str = "sf-attract-01";
const BASELINE: &str = "../../tools/reference/local/captures/fight-run-a";
const BOOT_A: &str = "../../tools/reference/local/captures/boot-01-run-a";
const BOOT_B: &str = "../../tools/reference/local/captures/boot-01-run-b";
const ATTRACT_A: &str = "../../tools/reference/local/captures/attract-01-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const ATTRACT_LAST: u32 = 1900;

fn present() -> bool {
    [BASELINE, BOOT_A, BOOT_B, ATTRACT_A, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn program() -> Vec<u8> {
    sf_video::romset::load_assets(ROMS, MANIFEST)
        .expect("ROM regions assemble")
        .maincpu
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

/// Acceptance 1 (C-REF): sf-boot-01's two runs are byte-for-byte identical
/// over all observation files (.tsv, .bin, .rgba), and its frames.tsv rows
/// equal fight-run-a's on every frame it covers (0..=829).
#[test]
fn sf_boot_01_reference_runs_are_identical_and_equal_the_fixed_fight() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(BOOT_A)
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
        let b = Path::new(BOOT_B).join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: the two sf-boot-01 runs must be identical"
        );
        compared += 1;
    }
    assert_eq!(
        compared, 4003,
        "800 full checkpoints * 5 files + 3 .tsv files"
    );

    let boot = frame_rows(BOOT_A);
    let fight = frame_rows(BASELINE);
    assert_eq!(boot.len(), 830, "frames 0..=829");
    for (&f, row) in &boot {
        assert_eq!(
            fight.get(&f),
            Some(row),
            "sf-boot-01 against fight-run-a at frame {f}"
        );
    }
}

/// Acceptance 3 & 4: MAME's power-on contents read from sf-boot-01's frame-0
/// dumps where the reset cannot have changed them by then (object RAM
/// 0xffe000..0x1000000, video RAM, palette RAM, and the work-RAM suffix
/// RESET_CLEAR_FRAME0_END..0xffe000 not yet cleared at frame 0's end, placed
/// by the summed 68000 instruction costs of 0x3b000..0x3b01a) match the
/// native power-on state's zero assumption in every region.
#[test]
fn mame_power_on_contents_at_frame_0_agree_with_the_zero_initial_state() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    // 88 cycles before the loop + 4,441 completed passes of 30 cycles =
    // 133,318 cycles <= 133,333 1/3 cycles/frame: 4,441 longwords
    // (0xff8000..0xffc564) cleared, 0xffc564..0xffe000 not yet reached.
    assert_eq!(RESET_CLEAR_FRAME0_END, 0xff8000 + 4_441 * 4);

    let m = PowerOnMachine::new(program());
    let work_dump = std::fs::read(Path::new(BOOT_A).join("ram_000000_work.bin")).unwrap();
    let video_dump = std::fs::read(Path::new(BOOT_A).join("ram_000000_video.bin")).unwrap();
    let palette_dump = std::fs::read(Path::new(BOOT_A).join("ram_000000_palette.bin")).unwrap();

    // Object RAM 0xffe000..0x1000000 (work dump offset 0x6000..0x8000).
    assert_eq!(
        &work_dump[0x6000..0x8000],
        m.state.objectram.as_slice(),
        "object RAM at frame 0"
    );
    // Video RAM and palette RAM.
    assert_eq!(video_dump, m.state.videoram, "video RAM at frame 0");
    assert_eq!(palette_dump, m.state.paletteram, "palette RAM at frame 0");
    // The work-RAM tail not yet reached by the reset's clear loop at frame 0,
    // and the prefix already cleared by it.
    let split = (RESET_CLEAR_FRAME0_END - 0xff8000) as usize;
    assert_eq!(
        &work_dump[split..0x6000],
        &m.work_ram()[split..0x6000],
        "uncleared work RAM at frame 0"
    );
    assert_eq!(
        &work_dump[..split],
        &m.work_ram()[..split],
        "cleared work RAM at frame 0"
    );
}

/// Acceptance 2, 4 & 7 (C-NAT, C-STOP): from power-on, every frame 0..=7
/// matches sf-boot-01 on the comparison boundary (screen hash, sound latch,
/// latch changed, checkpoint pixels, video and palette RAM) and on all owned
/// work RAM, with latch 0x00, 0x27, 0x3f, 0x80 at frames 0..=3 and the frame
/// word equal to the frame number; frame 8 stops and reports 0x3c210.
#[test]
fn power_on_matches_sf_boot_01_through_the_boot_setup_frames_and_stops_at_0x3c210() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = ScratchDir::new("sf-engine-power-on");
    let executed = run_power_on_scenario_capture(
        &assets,
        &out,
        SCENARIO_ID,
        Path::new(SCENARIO),
        BOOT_SETUP_LAST,
    )
    .expect("native power-on capture runs on translated paths only");
    assert_eq!(executed, (BOOT_SETUP_LAST + 1) as u64);

    let scope = CompareScope {
        from: Some(0),
        to: Some(BOOT_SETUP_LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(BOOT_A).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .expect("compatible captures");
    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = outcome
    else {
        panic!("power-on run must match sf-boot-01 over 0..={BOOT_SETUP_LAST}, got {outcome:?}");
    };
    assert_eq!(
        (frames_compared, checkpoints_compared),
        ((BOOT_SETUP_LAST + 1) as u64, (BOOT_SETUP_LAST + 1) as usize)
    );

    let owned = cpu_owned_addresses();
    let owned_set: BTreeSet<u32> = owned.iter().map(|&(_, a)| a).collect();
    // Acceptance 2's required owned addresses must all be present in the
    // owned list.
    for required in (0xffc000..=0xffc007)
        .chain([0xff8f28, 0xff8e1c, 0xff8682, 0xff8006, 0xff8007])
        .chain(0xff863c..0xff863e)
        .chain(0xff8644..0xff8646)
        .chain(0xff8e6e..=0xff8f0d)
        .chain(0xffc012..0xffc01a)
        .chain(0xff86b0..0xff8710)
        .chain(0xff8622..0xff8624)
    {
        assert!(
            owned_set.contains(&required),
            "0x{required:06x} must be in cpu_owned_addresses"
        );
    }

    let mut latches = Vec::new();
    let mut owned_failures = Vec::new();
    let mut others = Vec::new();
    let (stopped_frame, unrecovered) = observe_power_on(program(), BOOT_SETUP_LAST, |m| {
        let frame = m.state.frame;
        latches.push((frame, m.state.sound_latch));
        let w = m.work_ram();
        let frame_word = u16::from_be_bytes([w[4], w[5]]);
        assert_eq!(frame_word as u32, frame, "frame word at frame {frame}");

        let image = m.work_image();
        let dump = std::fs::read(Path::new(BOOT_A).join(format!("ram_{frame:06}_work.bin")))
            .expect("reference work dump");
        for &(name, addr) in &owned {
            let i = (addr - 0xff8000) as usize;
            if dump[i] != image[i] {
                owned_failures.push(format!(
                    "frame {frame}: {name} at 0x{addr:06x}: reference {:02x}, native {:02x}",
                    dump[i], image[i]
                ));
            }
        }
        for i in 0..dump.len() {
            if dump[i] != image[i] {
                let addr = 0xff8000 + i as u32;
                if area(addr) == "other" {
                    others.push(format!(
                        "frame {frame}: 0x{addr:06x}: reference {:02x}, native {:02x}",
                        dump[i], image[i]
                    ));
                }
            }
        }
    });
    assert!(
        owned_failures.is_empty(),
        "owned RAM mismatches: {owned_failures:#?}"
    );
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed: {others:#?}"
    );
    assert_eq!(
        &latches[..4],
        &[(0, 0x00), (1, 0x27), (2, 0x3f), (3, 0x80)],
        "latch at frames 0..=3"
    );
    assert_eq!(
        latches.len(),
        (BOOT_SETUP_LAST + 1) as usize,
        "frames 0..=7 observed"
    );
    assert_eq!(
        (stopped_frame, unrecovered),
        (BOOT_SETUP_LAST, Vec::new()),
        "frames 0..=7 complete without unrecovered branches"
    );
}

/// Ticket #55 (F15) Acceptance 1, 2 & 4 (C-NAT, C-STOP):
/// 1. From power-on with no fixture over 0..=1647, native execution matches
///    sf-boot-01 over 0..=829 (full checkpoints over 0..=799) and sf-attract-01
///    over 794..=1647 (full checkpoints over 794..=1647).
/// 2. At the end of frame 794, the owned state equals checkpoint 794, and the
///    random state (0xff8e4a..0xff8e52) equals one pass from zero with an idle
///    byte ([0x03, 0x06, 0x0b, 0x12, 0x1b, 0x26, 0x33, 0x42]).
/// 4. At frame 1648, the run stops and reports 0x4268c.
#[test]
fn power_on_matches_sf_boot_01_and_sf_attract_01_through_1647_and_stops_at_0x4268c() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");

    // 1a. Against sf-boot-01 over 0..=829 (full checkpoints over 0..=799).
    let out_boot = ScratchDir::new("sf-engine-power-on-boot-829");
    let executed_boot =
        run_power_on_scenario_capture(&assets, &out_boot, SCENARIO_ID, Path::new(SCENARIO), 829)
            .expect("native power-on capture over 0..=829");
    assert_eq!(executed_boot, 830);

    let scope_boot = CompareScope {
        from: Some(0),
        to: Some(829),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_boot = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(BOOT_A).unwrap(),
        &load_capture(&out_boot).unwrap(),
        CandidateKind::Native,
        &scope_boot,
    )
    .expect("compatible captures");
    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = outcome_boot
    else {
        panic!("power-on run must match sf-boot-01 over 0..=829, got {outcome_boot:?}");
    };
    assert_eq!(
        (frames_compared, checkpoints_compared),
        (830, 800),
        "power-on run must match sf-boot-01 over 0..=829 (800 full checkpoints 0..=799)"
    );

    // 1b. Against sf-attract-01 over 794..=1647 (full checkpoints over 794..=1647)
    // and frames.tsv over 0..=1449 (where sf-attract-01 has no coin input).
    let out_attract = ScratchDir::new("sf-engine-power-on-attract-1647");
    let executed_attract = run_power_on_scenario_capture(
        &assets,
        &out_attract,
        ATTRACT_SCENARIO_ID,
        Path::new(ATTRACT_SCENARIO),
        WARNING_TITLE_LAST,
    )
    .expect("native power-on capture over 0..=1647");
    assert_eq!(executed_attract, (WARNING_TITLE_LAST + 1) as u64);

    let attract_scenario = load_scenario(ATTRACT_SCENARIO).unwrap();
    let attract_ref = load_capture(ATTRACT_A).unwrap();
    let attract_nat = load_capture(&out_attract).unwrap();
    let scope_attract = CompareScope {
        from: Some(794),
        to: Some(WARNING_TITLE_LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_attract = compare(
        &attract_scenario,
        &attract_ref,
        &attract_nat,
        CandidateKind::Native,
        &scope_attract,
    )
    .expect("compatible captures");
    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = outcome_attract
    else {
        panic!("power-on run must match sf-attract-01 over 794..=1647, got {outcome_attract:?}");
    };
    assert_eq!(
        (frames_compared, checkpoints_compared),
        (
            (WARNING_TITLE_LAST - 794 + 1) as u64,
            (WARNING_TITLE_LAST - 794 + 1) as usize,
        ),
        "power-on run must match sf-attract-01 over 794..=1647 (854 full checkpoints)"
    );

    // Also verify all 1,648 frames (0..=1647) against sf-attract-01 in a single scope.
    let scope_all = CompareScope {
        from: Some(0),
        to: Some(WARNING_TITLE_LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_all = compare(
        &attract_scenario,
        &attract_ref,
        &attract_nat,
        CandidateKind::Native,
        &scope_all,
    )
    .expect("compatible captures");
    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = outcome_all
    else {
        panic!("power-on run must match sf-attract-01 over 0..=1647, got {outcome_all:?}");
    };
    assert_eq!(
        (frames_compared, checkpoints_compared),
        (
            (WARNING_TITLE_LAST + 1) as u64,
            (WARNING_TITLE_LAST - 794 + 1) as usize,
        ),
        "power-on run must match sf-attract-01 over 0..=1647"
    );

    // 2 & 4. Owned RAM and non-slot/stack work RAM across all checkpoints
    // (0..=799 in boot-01-run-a, 794..=1900 in attract-01-run-a) and random
    // state at 794 (with 0x3ce54..0x3ce86 translated in #88, F48).
    let owned = cpu_owned_addresses();
    let mut owned_failures = Vec::new();
    let mut others = Vec::new();
    let mut random_at_794 = None;
    let (stopped_frame, unrecovered) = observe_power_on(program(), ATTRACT_LAST, |m| {
        let frame = m.state.frame;
        let image = m.work_image();
        let mut check = |cap_dir: &str| {
            let dump = std::fs::read(Path::new(cap_dir).join(format!("ram_{frame:06}_work.bin")))
                .expect("reference work dump");
            for &(name, addr) in &owned {
                let i = (addr - 0xff8000) as usize;
                if dump[i] != image[i] {
                    owned_failures.push(format!(
                        "{cap_dir} frame {frame}: {name} at 0x{addr:06x}: reference {:02x}, native {:02x}",
                        dump[i], image[i]
                    ));
                }
            }
            for i in 0..dump.len() {
                if dump[i] != image[i] {
                    let addr = 0xff8000 + i as u32;
                    if area(addr) == "other" {
                        others.push(format!(
                            "{cap_dir} frame {frame}: 0x{addr:06x}: reference {:02x}, native {:02x}",
                            dump[i], image[i]
                        ));
                    }
                }
            }
        };
        if frame <= 799 {
            check(BOOT_A);
        }
        if frame >= 794 {
            check(ATTRACT_A);
        }
        if frame == 794 {
            let off = (0xff8e4a - 0xff8000) as usize;
            random_at_794 = Some(image[off..off + 8].to_vec());
        }
    });
    assert!(
        owned_failures.is_empty(),
        "owned RAM mismatches: {owned_failures:#?}"
    );
    assert!(
        others.is_empty(),
        "work RAM outside slots/stack differed: {others:#?}"
    );
    assert_eq!(
        random_at_794.as_deref(),
        Some(&[0x03, 0x06, 0x0b, 0x12, 0x1b, 0x26, 0x33, 0x42][..]),
        "random state at frame 794 equals one pass from zero with an idle IN1 low byte (0xff)"
    );
    assert_eq!(
        (stopped_frame, unrecovered),
        (ATTRACT_LAST, Vec::<u32>::new()),
        "runs cleanly through frame 1900 after 0x3ce54..0x3ce86 (#88, F48)"
    );
}

/// Ticket #55 (F15) Acceptance 6 (C-NEG): shortening the 240-frame warning
/// hold (`0x3b57c: move.w #$00f0, D0`) by 1 (`0x00ef` = 239 frames) is caught
/// at frame 554 on owned work RAM (`0xff8007` slot 0 sleep count) and at frame
/// 793 by `sf_compare::compare` when the warning screen is erased one frame
/// early.
#[test]
fn negative_control_warning_hold_shortened_by_one_is_caught() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut bad_assets =
        sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    assert_eq!(
        u16::from_be_bytes([bad_assets.maincpu[0x3b57c], bad_assets.maincpu[0x3b57d]]),
        0x00f0,
        "0x3b57c holds the 240-frame sleep word 0x00f0"
    );
    bad_assets.maincpu[0x3b57c..0x3b57e].copy_from_slice(&0x00efu16.to_be_bytes());

    // 1. Owned work RAM catches the shortened hold at frame 554 (slot 0 sleep
    //    count 0xff8007 is 0xef instead of 0xf0).
    let owned = cpu_owned_addresses();
    let mut first_ram_diff = None;
    observe_power_on(bad_assets.maincpu.clone(), 560, |m| {
        if first_ram_diff.is_some() {
            return;
        }
        let frame = m.state.frame;
        let image = m.work_image();
        let dump = std::fs::read(Path::new(BOOT_A).join(format!("ram_{frame:06}_work.bin")))
            .expect("reference work dump");
        for &(name, addr) in &owned {
            let i = (addr - 0xff8000) as usize;
            if dump[i] != image[i] {
                first_ram_diff = Some((frame, name, addr, dump[i], image[i]));
                break;
            }
        }
    });
    assert_eq!(
        first_ram_diff,
        Some((554, "task slot state / sleep", 0xff8007, 0xf0, 0xef)),
        "shortened 240-frame hold caught at frame 554 on slot 0 sleep count 0xff8007"
    );

    // 2. sf_compare::compare catches the early warning erase at frame 793
    //    (checkpoint pixels and video RAM).
    let out = ScratchDir::new("sf-engine-power-on-neg-hold");
    run_power_on_scenario_capture(&bad_assets, &out, SCENARIO_ID, Path::new(SCENARIO), 795)
        .expect("perturbed warning hold capture runs");
    let scope = CompareScope {
        from: Some(0),
        to: Some(795),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(BOOT_A).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .expect("compatible captures");
    let Outcome::Mismatch(diff) = outcome else {
        panic!("shortened 240-frame warning hold must fail comparison, got {outcome:?}");
    };
    assert_eq!(
        diff.frame, 793,
        "warning screen erases at 793 instead of 794 when the 240-frame hold is 239: {diff:?}"
    );
}

/// Acceptance 8 (C-NEG): moving the first palette upload a frame later is
/// caught by sf_compare at its first differing frame (frame 2).
#[test]
fn negative_control_one_palette_upload_moved_a_frame_later_is_caught() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = ScratchDir::new("sf-engine-power-on-neg");
    run_power_on_scenario_capture(
        &assets,
        &out,
        SCENARIO_ID,
        Path::new(SCENARIO),
        BOOT_SETUP_LAST,
    )
    .expect("native power-on capture runs");
    // Shift each palette upload one frame later (frames 5..=2 take the
    // previous frame's palette RAM).
    for f in (2..=5u32).rev() {
        let prev = std::fs::read(out.join(format!("ram_{:06}_palette.bin", f - 1))).unwrap();
        std::fs::write(out.join(format!("ram_{f:06}_palette.bin")), prev).unwrap();
    }
    let scope = CompareScope {
        from: Some(0),
        to: Some(BOOT_SETUP_LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(BOOT_A).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .expect("compatible captures");
    let Outcome::Mismatch(diff) = outcome else {
        panic!("delayed palette upload must fail comparison, got {outcome:?}");
    };
    assert_eq!(
        diff.frame, 2,
        "first palette upload belongs to frame 2: {diff:?}"
    );
    assert!(
        diff.observable.contains("palette"),
        "expected palette RAM mismatch at frame 2, got {diff:?}"
    );
}

/// Ticket #89 (`F49`): the fixed fight from power-on, with no fixture:
/// 1. `C-REF`:
///    - `sf-fixed-fight-power-on-01` (`fixed-fight-power-on-01-run-{a,b}`)
///      has full checkpoints on every frame `0..=2389`, `run-a == run-b`
///      byte-for-byte, and its `frames.tsv` rows `0..=2389` equal
///      `fight-run-a`'s.
///    - `sf-rng-inputs-01` (`rng-inputs-01-run-{a,b}`) has full checkpoints on
///      every frame `0..=2910` (through round 1's KO at `2861` and win award at
///      `2897`), `run-a == run-b` byte-for-byte, and its random state
///      `0xff8e4a..0xff8e51` at `2389` differs from the fixed fight's.
/// 2. `C-NAT` from power-on over `0..=2389` for both, including
///    `0xff8e4a..0xff8e51`, slot 7 (`0xff8076..0xff8078`), the coin slots
///    (`0xff863a..0xff865a`), and the credits (`0xff865a..0xff865e`), plus
///    full `sf_compare::compare` (`screen_hash`, `latch`, `latch_changed`,
///    checkpoint pixels and RAM).
/// 3. At `2389` the owned RAM of `sf-fixed-fight-power-on-01` equals
///    `sf-cpu-01`'s `2389` fixture (`cpu-run-a`), and continuing from power-on
///    with no fixture matches through `5014` against `fight-audit-run-a`.
///    `sf-rng-inputs-01` from power-on matches through round 1's KO (`2861`)
///    and win award (`2897`) through `2910`.
/// 4. `C-NEG`: moving the held P2 input in `sf-rng-inputs-01` by 7 frames (one
///    fold pass: `p2_left` at `1207` instead of `1200`) is caught on `0xff868e`
///    at `1201`, on `0xff8e4a..0xff8e51` at the fold pass (`1207`) and at
///    `2389`, and by `sf_compare::compare`.
#[test]
fn fixed_fight_and_rng_inputs_from_power_on_with_no_fixture_match_reference() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let po_a = Path::new("../../tools/reference/local/captures/fixed-fight-power-on-01-run-a");
    let po_b = Path::new("../../tools/reference/local/captures/fixed-fight-power-on-01-run-b");
    let rng_a = Path::new("../../tools/reference/local/captures/rng-inputs-01-run-a");
    let rng_b = Path::new("../../tools/reference/local/captures/rng-inputs-01-run-b");
    let cpu_a = Path::new("../../tools/reference/local/captures/cpu-01-run-a");
    let audit_a = Path::new("../../tools/reference/local/captures/fight-audit-run-a");
    if ![po_a, po_b, rng_a, rng_b, cpu_a, audit_a]
        .iter()
        .all(|p| p.is_dir())
    {
        eprintln!("skipping: F49 reference captures not present");
        return;
    }
    let po_scen = Path::new("../../tools/reference/scenarios/sf_fixed_fight_power_on_01.json");
    let rng_scen = Path::new("../../tools/reference/scenarios/sf_rng_inputs_01.json");
    let fixed_scen = Path::new("../../tools/reference/scenarios/sf_fixed_fight_01.json");
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");

    // 1. C-REF: run-a == run-b for both captures, rows 0..=2389 of
    // sf-fixed-fight-power-on-01 equal fight-run-a's, and random state at 2389
    // in sf-rng-inputs-01 differs from sf-fixed-fight-power-on-01's.
    for (dir_a, dir_b, label) in [
        (po_a, po_b, "sf-fixed-fight-power-on-01"),
        (rng_a, rng_b, "sf-rng-inputs-01"),
    ] {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir_a)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        let mut count = 0usize;
        for a in &files {
            let name = a.file_name().unwrap().to_string_lossy().to_string();
            if !(name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba")) {
                continue;
            }
            let b = dir_b.join(&name);
            assert_eq!(
                std::fs::read(a).unwrap(),
                std::fs::read(&b).unwrap(),
                "{label}: {name} differs between run-a and run-b"
            );
            count += 1;
        }
        assert!(
            count > 11_000,
            "{label}: expected >11,000 files, got {count}"
        );
    }

    let po_rows = frame_rows(po_a.to_str().unwrap());
    let fight_rows = frame_rows(BASELINE);
    assert_eq!(po_rows.len(), 2390);
    for (f, row) in &po_rows {
        assert_eq!(
            fight_rows.get(f),
            Some(row),
            "sf-fixed-fight-power-on-01 frame {f} differs from fight-run-a"
        );
    }

    let po_2389 = std::fs::read(po_a.join("ram_002389_work.bin")).unwrap();
    let rng_2389 = std::fs::read(rng_a.join("ram_002389_work.bin")).unwrap();
    assert_eq!(
        &po_2389[0x0e4a..0x0e52],
        &[0xa9, 0xeb, 0x5f, 0x66, 0x63, 0xfb, 0x45, 0xbe]
    );
    assert_eq!(
        &rng_2389[0x0e4a..0x0e52],
        &[0xb0, 0x16, 0x18, 0xf6, 0xe7, 0x13, 0xf9, 0x7e]
    );
    assert_ne!(
        &po_2389[0x0e4a..0x0e52],
        &rng_2389[0x0e4a..0x0e52],
        "sf-rng-inputs-01 random state at 2389 must differ from fixed fight's"
    );

    // 2 & 3. C-NAT from power-on over 0..=2389 for sf-fixed-fight-power-on-01
    // and continuing through 5014 against fight-audit-run-a, plus equality with
    // sf-cpu-01's 2389 fixture.
    let owned = cpu_owned_addresses();
    let cpu_2389 = std::fs::read(cpu_a.join("ram_002389_work.bin")).unwrap();
    let mut nat_2389 = None;
    let (stopped_po, unrec_po) =
        sf_engine::run::observe_power_on_scenario(assets.maincpu.clone(), fixed_scen, 5014, |m| {
            let f = m.state.frame;
            let img = m.work_image();
            if f <= 2389 {
                let ref_w = std::fs::read(po_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
                let owned_limit = if f == 0 { RESET_CLEAR_FRAME0_END } else { 0x100_0000 };
                for &(name, addr) in &owned {
                    if addr >= owned_limit {
                        continue;
                    }
                    let i = (addr - 0xff8000) as usize;
                    assert_eq!(
                        img[i], ref_w[i],
                        "sf-fixed-fight-power-on-01 frame {f}: {name} at 0x{addr:06x} ref={:02x} nat={:02x}",
                        ref_w[i], img[i]
                    );
                }
            }
            if f == 2389 {
                nat_2389 = Some(img.clone());
            }
            if f >= 2389 {
                let ref_w = std::fs::read(audit_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    assert_eq!(
                        img[i], ref_w[i],
                        "audit frame {f}: {name} at 0x{addr:06x} ref={:02x} nat={:02x}",
                        ref_w[i], img[i]
                    );
                }
            }
        })
        .unwrap();
    assert_eq!((stopped_po, unrec_po), (5014, Vec::new()));

    let nat_2389 = nat_2389.expect("native work image at 2389");
    for &(name, addr) in &owned {
        let i = (addr - 0xff8000) as usize;
        assert_eq!(
            nat_2389[i], cpu_2389[i],
            "at 2389 owned RAM {name} at 0x{addr:06x} must equal sf-cpu-01's 2389 fixture"
        );
    }

    // Full sf_compare::compare for sf-fixed-fight-power-on-01 over 0..=2389
    // and over 2389..=5014 against fight-audit-run-a.
    let po_out = ScratchDir::new("sf-fixed-fight-power-on-cand");
    let po_frames = run_power_on_scenario_capture(
        &assets,
        &po_out,
        "sf-fixed-fight-power-on-01",
        fixed_scen,
        5014,
    )
    .unwrap();
    assert_eq!(po_frames, 5015);
    let po_cand = load_capture(&po_out).unwrap();
    let scope_2389 = CompareScope {
        from: Some(0),
        to: Some(2389),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let cmp_po = compare(
        &load_scenario(po_scen).unwrap(),
        &load_capture(po_a).unwrap(),
        &po_cand,
        CandidateKind::Native,
        &scope_2389,
    )
    .unwrap();
    assert!(
        matches!(
            cmp_po,
            Outcome::Match {
                frames_compared: 2390,
                checkpoints_compared: 2390,
            }
        ),
        "sf-fixed-fight-power-on-01 compare failed: {cmp_po:?}"
    );

    let audit_scen = Path::new("../../tools/reference/scenarios/sf_fixed_fight_audit_01.json");
    let scope_5014 = CompareScope {
        from: Some(2389),
        to: Some(5014),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let mut po_cand_audit = po_cand;
    po_cand_audit.scenario_id = Some("sf-fixed-fight-audit-01".to_string());
    let cmp_audit = compare(
        &load_scenario(audit_scen).unwrap(),
        &load_capture(audit_a).unwrap(),
        &po_cand_audit,
        CandidateKind::Native,
        &scope_5014,
    )
    .unwrap();
    assert!(
        matches!(
            cmp_audit,
            Outcome::Match {
                frames_compared: 2626,
                checkpoints_compared: 2626,
            }
        ),
        "sf-fixed-fight-audit-01 compare failed: {cmp_audit:?}"
    );

    // 2 & 3 (sf-rng-inputs-01): C-NAT from power-on over 0..=2389 and through
    // round 1's KO (2861) and win award (2897) through 2910.
    let mut ko_seen = None;
    let mut win_seen = None;
    let (stopped_rng, unrec_rng) =
        sf_engine::run::observe_power_on_scenario(assets.maincpu.clone(), rng_scen, 2910, |m| {
            let f = m.state.frame;
            let img = m.work_image();
            let ref_w = std::fs::read(rng_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
            let owned_limit = if f == 0 {
                RESET_CLEAR_FRAME0_END
            } else {
                0x100_0000
            };
            for &(name, addr) in &owned {
                if addr >= owned_limit {
                    continue;
                }
                let i = (addr - 0xff8000) as usize;
                assert_eq!(
                    img[i], ref_w[i],
                    "sf-rng-inputs-01 frame {f}: {name} at 0x{addr:06x} ref={:02x} nat={:02x}",
                    ref_w[i], img[i]
                );
            }
            if f > 2400 && ko_seen.is_none() && (img[0x6408] == 8 || img[0x6a08] == 8) {
                ko_seen = Some(f);
            }
            if f > 2400 && win_seen.is_none() && (img[0x06b8] == 1 || img[0x466c] == 1) {
                win_seen = Some(f);
            }
        })
        .unwrap();
    assert_eq!((stopped_rng, unrec_rng), (2910, Vec::new()));
    assert_eq!(
        ko_seen,
        Some(2861),
        "round 1 KO in sf-rng-inputs-01 at frame 2861"
    );
    assert_eq!(
        win_seen,
        Some(2897),
        "round 1 win award in sf-rng-inputs-01 at frame 2897"
    );

    let rng_out = ScratchDir::new("sf-rng-inputs-01-cand");
    let rng_frames =
        run_power_on_scenario_capture(&assets, &rng_out, "sf-rng-inputs-01", rng_scen, 2910)
            .unwrap();
    assert_eq!(rng_frames, 2911);
    let scope_rng = CompareScope {
        from: Some(0),
        to: Some(2910),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let cmp_rng = compare(
        &load_scenario(rng_scen).unwrap(),
        &load_capture(rng_a).unwrap(),
        &load_capture(&rng_out).unwrap(),
        CandidateKind::Native,
        &scope_rng,
    )
    .unwrap();
    assert!(
        matches!(
            cmp_rng,
            Outcome::Match {
                frames_compared: 2911,
                checkpoints_compared: 2911,
            }
        ),
        "sf-rng-inputs-01 compare failed: {cmp_rng:?}"
    );

    // 4. C-NEG: moving the held P2 input in sf-rng-inputs-01 by 7 frames (one
    // fold pass: p2_left at 1207 instead of 1200) is caught.
    let orig_rng = std::fs::read_to_string(rng_scen).unwrap();
    let shifted_rng = orig_rng.replace(r#""frame": 1200"#, r#""frame": 1207"#);
    assert_ne!(orig_rng, shifted_rng, "p2_left at 1200 replaced with 1207");
    let neg_dir = ScratchDir::new("sf-rng-inputs-01-neg");
    std::fs::create_dir_all(&*neg_dir).unwrap();
    let shifted_path = neg_dir.join("shifted_rng.json");
    std::fs::write(&shifted_path, shifted_rng).unwrap();

    let mut first_in1_diff = None;
    let mut first_rng_diff = None;
    let mut shifted_rng_2389 = [0u8; 8];
    sf_engine::run::observe_power_on_scenario(assets.maincpu.clone(), &shifted_path, 2389, |m| {
        let f = m.state.frame;
        let w = m.work_ram();
        if (1200..=1220).contains(&f) {
            let ref_w = std::fs::read(rng_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
            if first_in1_diff.is_none() && w[0x068e] != ref_w[0x068e] {
                first_in1_diff = Some((f, ref_w[0x068e], w[0x068e]));
            }
            if first_rng_diff.is_none() && w[0x0e4a..0x0e52] != ref_w[0x0e4a..0x0e52] {
                first_rng_diff = Some(f);
            }
        }
        if f == 2389 {
            shifted_rng_2389.copy_from_slice(&w[0x0e4a..0x0e52]);
        }
    })
    .unwrap();
    assert_eq!(
        first_in1_diff,
        Some((1201, 0xfd, 0xff)),
        "7-frame shifted p2_left caught on 0xff868e at frame 1201"
    );
    assert_eq!(
        first_rng_diff,
        Some(1207),
        "7-frame shifted p2_left caught on 0xff8e4a..0xff8e51 at the fold pass (frame 1207)"
    );
    assert_ne!(
        &shifted_rng_2389[..],
        &rng_2389[0x0e4a..0x0e52],
        "7-frame shifted p2_left changes random state at frame 2389"
    );
}
