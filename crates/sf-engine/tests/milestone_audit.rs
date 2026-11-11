//! The first playable milestone audit (ticket #39): the whole fixed fight
//! natively — sf-fixed-fight-01's inputs from the round-1 fixture
//! (checkpoint 2389 of sf-cpu-01) through the match end — compared with the
//! pinned MAME reference on the 68000-side observables a capture records,
//! frame by frame (the music RAM dumps are compared at the music CPU's idle
//! frame ends by sf-audio's both_chips.rs; the music CRC is not compared).
//! docs/reference/milestone-audit.md.
//!
//! The pinned baseline fight-run-a ends at frame 4999 with full checkpoints
//! at 7 frames of the span. The audit capture sf-fixed-fight-audit-01 runs
//! the same inputs to frame 5040 with a full checkpoint (screen pixels,
//! work, object, video, palette and music RAM) after every frame from
//! 2389; its two runs are identical and its frames through 4999 equal the
//! baseline's. The native run goes from the fixture to the CPU's match win
//! (0x3eefa, the continue screen, untranslated), which it reaches at frame
//! 5015 after round 2's result and the match decision.
//!
//! Compared after every frame 2390..=5014:
//! - the comparison boundary (`sf_compare`): screen hash, sound latch, its
//!   changed flag, and the full screen pixels with video and palette RAM;
//! - work RAM: the regions the translated routines own, byte for byte, and
//!   every other differing byte classified (only the CPU state the
//!   translation does not keep may differ: the task slots' saved
//!   SR / PC / USP words and the stack memory);
//! - the reference's work-RAM CRC every ten frames, reported.
//!
//! The audio side (both sound programs from this run's latch, the music
//! CPU's RAM at idle frame ends) is sf-audio's both_chips.rs. Local-only
//! data; skips without it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{cpu_owned_addresses, cpu_work_image, observe_cpu_fight};

/// The same inputs with a checkpoint after every frame from 2389 to 5040.
const AUDIT_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_audit_01.json";
const AUDIT_ID: &str = "sf-fixed-fight-audit-01";
/// The pinned baseline capture of the fixed fight (frames 0..=4999).
const BASELINE: &str = "../../tools/reference/local/captures/fight-run-a";
/// The audit capture's two runs.
const AUDIT_A: &str = "../../tools/reference/local/captures/fight-audit-run-a";
const AUDIT_B: &str = "../../tools/reference/local/captures/fight-audit-run-b";
/// The round-1 fixture (checkpoint 2389 of sf-cpu-01).
const FIXTURE_CAPTURE: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIXTURE: u32 = 2389;
/// The last frame the native run produces: the next one reaches the CPU's
/// match win (0x3eefa).
const LAST: u32 = 5014;

fn present() -> bool {
    [BASELINE, AUDIT_A, AUDIT_B, FIXTURE_CAPTURE, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn program() -> Vec<u8> {
    sf_video::romset::load_assets(ROMS, MANIFEST)
        .expect("ROM regions assemble")
        .maincpu
}

/// A capture's frames.tsv rows: frame -> the row's fields after the frame.
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

/// The ram_crc column: frame -> crc.
fn ram_crcs(capture: &str) -> BTreeMap<u32, u32> {
    frame_rows(capture)
        .into_iter()
        .filter_map(|(f, rest)| Some((f, u32::from_str_radix(rest.split('\t').nth(3)?, 16).ok()?)))
        .collect()
}

/// Where a work-RAM byte lies, for the bytes the translation does not keep
/// (task-scheduler.md): a task slot's saved SR / PC / USP words (bytes
/// 2..16 of each 16-byte slot from 0xff8006; bytes 0 and 1, the state and
/// sleep, are owned), or the stack memory 0xff8106-0xff8631 (the
/// supervisor stack under the reset SSP 0xff81a2, and the tasks' stacks,
/// whose saved USPs lie at 0xff825e-0xff85de, holding the register images
/// and return addresses). Everything else is "other".
fn area(addr: u32) -> &'static str {
    match addr {
        0xff8006..=0xff8105 if (addr - 0xff8006) % 16 >= 2 => "task slots' saved SR/PC/USP",
        0xff8106..=0xff8631 => "stack memory",
        _ => "other",
    }
}

/// The premises: the fixture's history is the fixed fight's, and the audit
/// capture is the fixed fight's, deterministic and equal to the baseline.
#[test]
fn the_fixture_and_the_audit_capture_are_the_fixed_fight() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let baseline = frame_rows(BASELINE);
    // The fixture capture's every recorded field equals the baseline's
    // through the fixture frame.
    let fixture = frame_rows(FIXTURE_CAPTURE);
    for f in 0..=FIXTURE {
        assert_eq!(
            fixture.get(&f),
            baseline.get(&f),
            "sf-cpu-01 against sf-fixed-fight-01 at frame {f}"
        );
    }
    // The sound comparisons' reference run (fight-audio-a, sf-audio's
    // both_chips.rs) is the same fight: every field of every frame.
    const FIGHT_AUDIO: &str = "../../tools/reference/local/captures/fight-audio-a";
    if Path::new(FIGHT_AUDIO).is_dir() {
        assert_eq!(
            frame_rows(FIGHT_AUDIO),
            baseline,
            "fight-audio-a against fight-run-a"
        );
    }
    // The audit capture's two runs have identical observation files (.tsv,
    // .bin, .rgba); runner metadata and PNG previews are not compared
    // (mame-baseline.md).
    let mut files: Vec<PathBuf> = std::fs::read_dir(AUDIT_A)
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
        let b = Path::new(AUDIT_B).join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: the two audit runs must be identical"
        );
        compared += 1;
    }
    // Its frames through 4999 are the baseline's, on every field, and so
    // are the baseline's checkpoint screens in range.
    let audit = frame_rows(AUDIT_A);
    for (f, row) in &baseline {
        assert_eq!(
            audit.get(f),
            Some(row),
            "sf-fixed-fight-audit-01 against sf-fixed-fight-01 at frame {f}"
        );
    }
    let checkpoints = |dir: &str| -> BTreeMap<u32, String> {
        std::fs::read_to_string(Path::new(dir).join("checkpoints.tsv"))
            .unwrap()
            .lines()
            .skip(1)
            .filter_map(|l| {
                let c: Vec<&str> = l.split('\t').collect();
                Some((c[0].parse().ok()?, c.get(1)?.to_string()))
            })
            .collect()
    };
    let (base_cp, audit_cp) = (checkpoints(BASELINE), checkpoints(AUDIT_A));
    let mut screens = 0;
    for (f, crc) in base_cp.iter().filter(|(f, _)| **f >= FIXTURE) {
        assert_eq!(audit_cp.get(f), Some(crc), "checkpoint screen {f}");
        for region in ["work", "video", "palette", "music"] {
            let name = format!("ram_{f:06}_{region}.bin");
            assert!(
                std::fs::read(Path::new(BASELINE).join(&name)).unwrap()
                    == std::fs::read(Path::new(AUDIT_A).join(&name)).unwrap(),
                "{name}"
            );
        }
        screens += 1;
    }
    assert!(audit.keys().max() >= Some(&5040));
    println!(
        "premises: sf-cpu-01 equals the fixed fight on every field through {FIXTURE}; the audit runs identical over {compared} files; the audit capture equals the baseline on {} frames and its {screens} checkpoints in range",
        baseline.len()
    );
}

/// The comparison boundary after every frame from the fixture to the
/// match win: screen hash, latch, latch changed, full screen pixels, video
/// and palette RAM.
#[test]
fn the_fixed_fight_matches_the_reference_on_the_comparison_boundary_through_the_match_end() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = sf_engine::run::ScratchDir::new("sf-engine-milestone");
    sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(FIXTURE_CAPTURE),
        &assets,
        &out,
        AUDIT_ID,
        Some(Path::new(AUDIT_SCENARIO)),
        FIXTURE,
        LAST,
    )
    .expect("native capture runs on translated paths only");
    let scope = CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(AUDIT_SCENARIO).unwrap(),
        &load_capture(AUDIT_A).unwrap(),
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
        panic!("the fixed fight must match the reference, got {outcome:?}");
    };
    assert_eq!(
        (frames_compared, checkpoints_compared),
        ((LAST - FIXTURE) as u64, (LAST - FIXTURE) as usize)
    );
    println!("boundary: {frames_compared} frames (screen hash, latch, latch changed) and {checkpoints_compared} full checkpoints (pixels, video and palette RAM) match, {}..={LAST}", FIXTURE + 1);
}

/// Work RAM against the reference after every frame: the regions the
/// translated routines own equal on every frame; every other differing byte
/// in the CPU state the translation does not keep; the ten-frame CRCs
/// reported as they are.
#[test]
fn the_fixed_fights_work_ram_against_the_reference_every_frame() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let crcs = ram_crcs(AUDIT_A);
    let owned = cpu_owned_addresses();
    let (mut crc_equal, mut crc_differ) = (0u32, 0u32);
    let (mut frames, mut frames_differing) = (0u32, 0u32);
    let mut by_area: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut owned_failures: Vec<String> = Vec::new();
    let mut others: Vec<String> = Vec::new();
    let (last, unrecovered) = observe_cpu_fight(
        Path::new(FIXTURE_CAPTURE),
        program(),
        Some(Path::new(AUDIT_SCENARIO)),
        FIXTURE,
        LAST,
        |m| {
            let frame = m.state.frame;
            let image = cpu_work_image(m);
            assert_eq!(image.len(), 0x8000, "the 0xff8000-0xffffff image");
            if let Some(&r) = crcs.get(&frame) {
                if crc32fast::hash(&image) == r {
                    crc_equal += 1;
                } else {
                    crc_differ += 1;
                }
            }
            let dump = std::fs::read(Path::new(AUDIT_A).join(format!("ram_{frame:06}_work.bin")))
                .expect("reference work dump");
            for &(name, addr) in &owned {
                let i = (addr - 0xff8000) as usize;
                if dump[i] != image[i] && owned_failures.len() < 20 {
                    owned_failures.push(format!(
                        "frame {frame}: {name} at 0x{addr:06x}: reference {:02x}, native {:02x}",
                        dump[i], image[i]
                    ));
                }
            }
            let mut any = false;
            for i in 0..dump.len() {
                if dump[i] != image[i] {
                    any = true;
                    let addr = 0xff8000 + i as u32;
                    *by_area.entry(area(addr)).or_default() += 1;
                    if area(addr) == "other" && others.len() < 20 {
                        others.push(format!(
                            "frame {frame}: 0x{addr:06x}: reference {:02x}, native {:02x}",
                            dump[i], image[i]
                        ));
                    }
                }
            }
            frames += 1;
            frames_differing += any as u32;
        },
    )
    .expect("native run");
    assert!(
        unrecovered.is_empty(),
        "the run reached untranslated branches {unrecovered:05x?} at frame {last}"
    );
    assert_eq!((last, frames), (LAST, LAST - FIXTURE));
    println!("work RAM after {frames} frames: owned regions equal on every frame; {frames_differing} frames differ elsewhere, by area (byte-frames): {by_area:?}");
    println!("work-RAM CRC every ten frames: {crc_equal} equal, {crc_differ} differ");
    assert!(
        owned_failures.is_empty(),
        "owned regions differ:\n{}",
        owned_failures.join("\n")
    );
    assert!(
        others.is_empty(),
        "work RAM differs outside the CPU-state areas:\n{}",
        others.join("\n")
    );
}

/// Past the match decision: since ticket #45 (F5) translated the CPU's match
/// win (`0x3eefa`), the audit capture continues through its end at 5040 with
/// no untranslated branch reached (see `tests/cpu_match_win.rs` for the full
/// run to the continue expiry at 6100).
#[test]
fn past_the_match_end_the_run_stops_at_the_cpus_match_win() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let (last, unrecovered) = observe_cpu_fight(
        Path::new(FIXTURE_CAPTURE),
        program(),
        Some(Path::new(AUDIT_SCENARIO)),
        FIXTURE,
        5040,
        |_| {},
    )
    .expect("native run");
    assert_eq!((last, unrecovered), (5040, Vec::<u32>::new()));
}

/// The negative control: the same comparison catches a native run whose
/// light punch comes ten frames late (2600 instead of 2590), at the frame
/// the difference first shows.
#[test]
fn a_shifted_input_is_caught_where_it_shows() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let mut scenario: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(AUDIT_SCENARIO).unwrap()).unwrap();
    let punch = scenario["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["press"] == "p1_button1")
        .expect("the light punch");
    assert_eq!(punch["frame"], 2590);
    punch["frame"] = 2600.into();
    let out = sf_engine::run::ScratchDir::new("sf-engine-milestone-shifted");
    std::fs::create_dir_all(&*out).unwrap();
    let shifted = out.join("shifted.json");
    std::fs::write(&shifted, serde_json::to_string(&scenario).unwrap()).unwrap();
    let capture = out.join("capture");
    std::fs::create_dir_all(&capture).unwrap();
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(FIXTURE_CAPTURE),
        &assets,
        &capture,
        AUDIT_ID,
        Some(&shifted),
        FIXTURE,
        2700,
    )
    .expect("native capture");
    let scope = CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(2700),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(AUDIT_SCENARIO).unwrap(),
        &load_capture(AUDIT_A).unwrap(),
        &load_capture(&capture).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .expect("compatible captures");
    let Outcome::Mismatch(m) = outcome else {
        panic!("a shifted punch must not match, got {outcome:?}");
    };
    println!(
        "negative control: the punch 10 frames late is caught at frame {} ({}: {} vs {})",
        m.frame, m.observable, m.expected, m.actual
    );
    assert!(
        (2590..=2620).contains(&m.frame),
        "caught where the punch differs ({})",
        m.frame
    );
}
