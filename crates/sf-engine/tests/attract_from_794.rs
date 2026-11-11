//! Tickets #43 (F3) & #56 (F16): Attract wall, title, INSERT COIN, and the
//! attract ranking screen (`0x4268c..0x4274e`) from the attract-entry fixture
//! (checkpoint 794) and from `sf-music-03`'s ranking-wait fixture (checkpoint
//! 7185).
//!
//! Verifies:
//! 1. C-REF: `sf-attract-01` (power-on, no inputs, 1,900 frames, checkpoints
//!    every frame 794..=1900) matches across two runs, equals `fight-run-a`
//!    over `frames.tsv` rows 0..=1449 and checkpoint 794, and checkpoint 794
//!    is a consistent state whose rendered pixels match `pixels_000794.rgba`
//!    byte-for-byte.
//! 2. C-NAT & C-STOP (#43 & #56): from checkpoint 794 over 795..=1854, native
//!    execution matches `sf-attract-01` on every frame (screen hash, 384x224
//!    pixels, video RAM, palette RAM, sound latch with `0x1c` at 976 and `0x39`
//!    at 1649, and owned work RAM), and at frame 1855 stops reporting `0x3ce54`
//!    (and reports `0x42750` at 1648 when a ranking entry is marked `0xff`).
//! 3. C-NAT & C-STOP (#56): from `sf-music-03`'s checkpoint 7185 over
//!    7186..=7391, native execution matches on every frame (with `0x39` at
//!    7186) and at frame 7392 stops reporting `0x3ce54`.
//! 4. C-NEG (#43 & #56): slot 3 D2 off by one (caught at 813), scroll sleep
//!    one frame short (caught at 795 / 974), and attract ranking exit one frame
//!    early (caught at 1854).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{
    compare_cpu_ram, cpu_owned_addresses, cpu_work_image, observe_cpu_fight, run_cpu_fight_capture,
    CpuRamOutcome, ScratchDir,
};
use sf_engine::tasks::Scheduler;
use sf_video::render::render_frame;
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;

const SCENARIO: &str = "../../tools/reference/scenarios/sf_attract_01.json";
const SCENARIO_ID: &str = "sf-attract-01";
const RUN_A: &str = "../../tools/reference/local/captures/attract-01-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/attract-01-run-b";
const FIGHT_RUN_A: &str = "../../tools/reference/local/captures/fight-run-a";
const MUSIC_03_SCENARIO: &str = "../../tools/reference/scenarios/sf_music_03.json";
const MUSIC_03_SCENARIO_ID: &str = "sf-music-03";
const MUSIC_03_RUN_A: &str = "../../tools/reference/local/captures/music-03-audio-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present() -> bool {
    [RUN_A, RUN_B, FIGHT_RUN_A, MUSIC_03_RUN_A, ROMS]
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

#[test]
fn sf_attract_01_reference_runs_match_and_equal_fight_run_a_through_1449_and_checkpoint_794() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).expect("scenario loads");
    assert_eq!(scenario.checkpoints.len(), 1900 - 794 + 1);
    assert_eq!(scenario.checkpoints.first().copied(), Some(794));
    assert_eq!(scenario.checkpoints.last().copied(), Some(1900));

    let mut files: Vec<_> = std::fs::read_dir(RUN_A)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut compared_files = 0;
    for a in &files {
        let name = a.file_name().unwrap().to_string_lossy().to_string();
        if !(name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba")) {
            continue;
        }
        let b = Path::new(RUN_B).join(&name);
        assert!(
            std::fs::read(a).unwrap() == std::fs::read(&b).unwrap_or_default(),
            "{name}: the two sf-attract-01 runs must be byte-for-byte identical"
        );
        compared_files += 1;
    }
    assert_eq!(
        compared_files, 5538,
        "1,107 full checkpoints * 5 files + 3 .tsv files"
    );

    let attract_rows = frame_rows(RUN_A);
    let fight_rows = frame_rows(FIGHT_RUN_A);
    assert_eq!(attract_rows.len(), 1901, "frames 0..=1900");
    for f in 0..=1449u32 {
        assert_eq!(
            attract_rows.get(&f),
            fight_rows.get(&f),
            "frames.tsv row {f} differs between sf-attract-01 and fight-run-a"
        );
    }

    let run_a_dir = Path::new(RUN_A);
    let fight_dir = Path::new(FIGHT_RUN_A);
    for file in [
        "pixels_000794.rgba",
        "ram_000794_work.bin",
        "ram_000794_video.bin",
        "ram_000794_palette.bin",
        "ram_000794_music.bin",
    ] {
        let a_bytes = std::fs::read(run_a_dir.join(file)).expect(file);
        let fight_bytes = std::fs::read(fight_dir.join(file)).expect(file);
        assert_eq!(
            a_bytes, fight_bytes,
            "checkpoint 794 file {file} differs between sf-attract-01 and fight-run-a"
        );
    }

    // Verify checkpoint 794 is a consistent state: rendering from its dumps
    // reproduces pixels_000794.rgba byte-for-byte (86,016 pixels).
    let work = std::fs::read(run_a_dir.join("ram_000794_work.bin")).unwrap();
    let videoram = std::fs::read(run_a_dir.join("ram_000794_video.bin")).unwrap();
    let paletteram = std::fs::read(run_a_dir.join("ram_000794_palette.bin")).unwrap();
    let pixels_ref = std::fs::read(run_a_dir.join("pixels_000794.rgba")).unwrap();
    let scene = SceneState {
        videoram,
        objectram: work[0x6000..0x8000].to_vec(),
        paletteram,
        bg_scroll: u16::from_be_bytes([work[0x4682], work[0x4683]]),
        fg_scroll: u16::from_be_bytes([work[0x4680], work[0x4681]]),
        active: work[0x682],
        flip_screen: work[0x682] & 4 != 0,
        bg_page: 0,
        fg_page: 0,
    };
    let rendered = render_frame(&assets(), &scene);
    let rendered_bytes: Vec<u8> = rendered
        .argb
        .iter()
        .flat_map(|px| px.to_le_bytes())
        .collect();
    assert_eq!(
        rendered_bytes, pixels_ref,
        "checkpoint 794 rendered pixels match pixels_000794.rgba byte-for-byte"
    );
}

#[test]
fn native_attract_from_794_matches_795_through_1854_and_stops_at_1855_with_0x3ce54() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    let run_a = Path::new(RUN_A);

    // Every checkpoint 794..=1854 in attract-01-run-a decodes cleanly.
    for f in 794..=1854u32 {
        let w = std::fs::read(run_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
        let s = Scheduler::from_fixture(&w).unwrap_or_else(|e| panic!("frame {f}: {e}"));
        let pos = s.positions();
        assert_eq!(pos.len(), 4, "frame {f}: 4 active tasks");
    }

    // Owned RAM comparison over 795..=1900 (with 0x3ce54..0x3ce86 translated in #88).
    let outcome = compare_cpu_ram(run_a, assets.maincpu.clone(), 794, 1900).unwrap();
    assert_eq!(outcome, CpuRamOutcome::Match { frames: 1900 - 794 });

    // Marked-entry path 0x42750 is reported at 1648 when a ranking entry has
    // byte +4 == 0xff (0x4270a..0x42726).
    let marked_fix = ScratchDir::new("sf-attract-marked-ranking-fixture");
    std::fs::create_dir_all(&*marked_fix).unwrap();
    for file in [
        "frames.tsv",
        "meta.tsv",
        "checkpoints.tsv",
        "pixels_001647.rgba",
        "ram_001647_video.bin",
        "ram_001647_palette.bin",
        "ram_001647_music.bin",
    ] {
        std::fs::copy(run_a.join(file), marked_fix.join(file)).unwrap();
    }
    let mut work_1647 = std::fs::read(run_a.join("ram_001647_work.bin")).unwrap();
    work_1647[(0xff8e6e + 4 - 0xff8000) as usize] = 0xff;
    std::fs::write(marked_fix.join("ram_001647_work.bin"), &work_1647).unwrap();
    let mut marked_m = sf_engine::cpu::CpuMachine::from_fight_checkpoint(
        &marked_fix,
        1647,
        assets.maincpu.clone(),
    )
    .unwrap();
    marked_m.step();
    assert_eq!(
        (marked_m.state.frame, marked_m.unrecovered()),
        (1648, Vec::<u32>::new())
    );
    assert!(
        marked_m.task_positions().contains(&(1, 0x4275c)),
        "ranking entry with marker 0xff at +4 enters 0x42750..0x4275c at frame 1648 (#122, F50)"
    );

    // Every work-RAM byte outside unowned stack (0xff8106..0xff8632 excluding
    // owned 0xff8622..0xff8624) and slot SR/PC/USP headers matches over 795..=1854.
    let owned_set: BTreeSet<u32> = cpu_owned_addresses().into_iter().map(|(_, a)| a).collect();
    let mut latches = Vec::new();
    let mut others = Vec::new();
    let (end_frame, unrec) =
        observe_cpu_fight(run_a, assets.maincpu.clone(), None, 794, 1854, |m| {
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
    assert_eq!((end_frame, unrec), (1854, Vec::<u32>::new()));
    assert_eq!(
        latches,
        vec![(976, 0x1c), (1649, 0x39)],
        "0x1c latched at frame 976 and 0x39 latched at frame 1649 over 795..=1854"
    );

    // Full C-NAT capture comparison over 795..=1854 via sf_compare::compare.
    let out_dir = ScratchDir::new("sf-attract-01-native");
    let frames = run_cpu_fight_capture(run_a, &assets, &out_dir, SCENARIO_ID, 794, 1854)
        .expect("native attract capture");
    assert_eq!(frames, 1854 - 794 + 1);

    let scenario = load_scenario(SCENARIO).unwrap();
    let ref_cap = load_capture(run_a).unwrap();
    let nat_cap = load_capture(&out_dir).unwrap();
    let scope = CompareScope {
        from: Some(795),
        to: Some(1854),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(&scenario, &ref_cap, &nat_cap, CandidateKind::Native, &scope)
        .expect("compatible captures");
    assert!(
        matches!(outcome, Outcome::Match { .. }),
        "C-NAT full capture compare over 795..=1854 failed: {outcome:?}"
    );
}

#[test]
fn native_attract_ranking_from_music_03_checkpoint_7185_matches_7186_through_7391_and_stops_at_7392_with_0x3ce54(
) {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = assets();
    let music_03 = Path::new(MUSIC_03_RUN_A);

    // Every checkpoint 7185..=7391 in music-03-audio-a decodes cleanly to
    // slot 1 at 0x42736 -> 0x3ce54.
    for f in 7185..=7391u32 {
        let w = std::fs::read(music_03.join(format!("ram_{f:06}_work.bin"))).unwrap();
        let s = Scheduler::from_fixture(&w).unwrap_or_else(|e| panic!("frame {f}: {e}"));
        let pos = s.positions();
        assert_eq!(pos.len(), 4, "frame {f}: 4 active tasks");
        assert!(
            pos.contains(&(1, 0x42736)),
            "frame {f}: attract ranking wait active on slot 1: {pos:x?}"
        );
    }

    // Owned RAM comparison over 7186..=7391, and frame 7392 runs 0x3ce54..0x3ce86 cleanly (#88).
    let outcome = compare_cpu_ram(music_03, assets.maincpu.clone(), 7185, 7391).unwrap();
    assert_eq!(
        outcome,
        CpuRamOutcome::Match {
            frames: 7391 - 7185
        }
    );
    let (end_7392, unrec_7392) =
        observe_cpu_fight(music_03, assets.maincpu.clone(), None, 7185, 7392, |_| {}).unwrap();
    assert_eq!((end_7392, unrec_7392), (7392, Vec::<u32>::new()));

    // Every work-RAM byte outside unowned stack and slot SR/PC/USP headers matches.
    let owned_set: BTreeSet<u32> = cpu_owned_addresses().into_iter().map(|(_, a)| a).collect();
    let mut latches = Vec::new();
    let mut others = Vec::new();
    let (end_frame, unrec) =
        observe_cpu_fight(music_03, assets.maincpu.clone(), None, 7185, 7391, |m| {
            let f = m.state.frame;
            if m.state.sound_latch != 0xff {
                latches.push((f, m.state.sound_latch));
            }
            let ref_work = std::fs::read(music_03.join(format!("ram_{f:06}_work.bin"))).unwrap();
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
    assert_eq!((end_frame, unrec), (7391, Vec::<u32>::new()));
    assert_eq!(
        latches,
        vec![(7186, 0x39)],
        "0x39 latched at frame 7186 over 7186..=7391"
    );

    // Full C-NAT capture comparison over 7186..=7391 via sf_compare::compare.
    let out_dir = ScratchDir::new("sf-music-03-ranking-native");
    let frames = run_cpu_fight_capture(
        music_03,
        &assets,
        &out_dir,
        MUSIC_03_SCENARIO_ID,
        7185,
        7391,
    )
    .expect("native music-03 ranking capture");
    assert_eq!(frames, 7391 - 7185 + 1);

    let scenario = load_scenario(MUSIC_03_SCENARIO).unwrap();
    let mut ref_cap = load_capture(music_03).unwrap();
    // music-03-audio-a was captured via audio_tap.lua without runner.py's summary.json.
    ref_cap
        .scenario_id
        .get_or_insert_with(|| MUSIC_03_SCENARIO_ID.to_string());
    ref_cap
        .mame_version_reported
        .get_or_insert_with(|| "0.289 (mame0289)".to_string());
    let nat_cap = load_capture(&out_dir).unwrap();
    let scope = CompareScope {
        from: Some(7186),
        to: Some(7391),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(&scenario, &ref_cap, &nat_cap, CandidateKind::Native, &scope)
        .expect("compatible captures");
    assert!(
        matches!(outcome, Outcome::Match { .. }),
        "C-NAT full capture compare over 7186..=7391 failed: {outcome:?}"
    );
}

#[test]
fn negative_controls_slot3_d2_off_by_one_scroll_sleep_one_frame_short_and_ranking_exit_one_frame_early_are_caught(
) {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let base_assets = assets();
    let run_a = Path::new(RUN_A);
    let scenario = load_scenario(SCENARIO).unwrap();
    let ref_cap = load_capture(run_a).unwrap();

    // 1. Slot 3's D2 changed by 1 (0x13 instead of 0x14 in the 794 fixture) is
    //    caught at the first INSERT COIN toggle (frame 813 instead of 814).
    let fix_dir = ScratchDir::new("sf-attract-neg-d2-fixture");
    std::fs::create_dir_all(&*fix_dir).unwrap();
    for file in [
        "frames.tsv",
        "meta.tsv",
        "checkpoints.tsv",
        "pixels_000794.rgba",
        "ram_000794_video.bin",
        "ram_000794_palette.bin",
        "ram_000794_music.bin",
    ] {
        std::fs::copy(run_a.join(file), fix_dir.join(file)).unwrap();
    }
    let mut work_794 = std::fs::read(run_a.join("ram_000794_work.bin")).unwrap();
    let slot3_usp = u32::from_be_bytes(work_794[0x36 + 8..0x36 + 12].try_into().unwrap());
    let d2_off = (slot3_usp + 8 + 2 - 0xff8000) as usize;
    assert_eq!(
        u16::from_be_bytes([work_794[d2_off], work_794[d2_off + 1]]),
        0x0014
    );
    work_794[d2_off..d2_off + 2].copy_from_slice(&0x0013u16.to_be_bytes());
    std::fs::write(fix_dir.join("ram_000794_work.bin"), &work_794).unwrap();

    let out_d2 = ScratchDir::new("sf-attract-neg-d2-out");
    run_cpu_fight_capture(&fix_dir, &base_assets, &out_d2, SCENARIO_ID, 794, 820)
        .expect("perturbed D2 capture runs");
    let nat_d2 = load_capture(&out_d2).unwrap();
    let scope_d2 = CompareScope {
        from: Some(795),
        to: Some(820),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_d2 = compare(
        &scenario,
        &ref_cap,
        &nat_d2,
        CandidateKind::Native,
        &scope_d2,
    )
    .expect("compatible captures");
    let Outcome::Mismatch(diff_d2) = outcome_d2 else {
        panic!("slot 3 D2 off by one must fail at first INSERT COIN toggle, got {outcome_d2:?}");
    };
    assert_eq!(
        diff_d2.frame, 813,
        "first INSERT COIN toggle shifts from 814 to 813 when D2 is 19: {diff_d2:?}"
    );

    // 2. Scroll-entry 0 sleep one frame short (0xb3 = 179 instead of 0xb4 = 180
    //    at 0x3cde4) is caught at that scroll step (frame 795 sleep count in
    //    work RAM, and frame 974 screen_hash/pixels when scroll step 1 runs one
    //    frame early).
    let mut bad_assets = assets();
    assert_eq!(
        u16::from_be_bytes([bad_assets.maincpu[0x3cde4], bad_assets.maincpu[0x3cde5]]),
        0x00b4
    );
    bad_assets.maincpu[0x3cde4..0x3cde6].copy_from_slice(&0x00b3u16.to_be_bytes());

    let ram_outcome = compare_cpu_ram(run_a, bad_assets.maincpu.clone(), 794, 980).unwrap();
    assert!(
        matches!(&ram_outcome, CpuRamOutcome::Mismatch(msg) if msg.contains("frame 795") && msg.contains("0xff8017")),
        "expected slot 1 sleep count mismatch at frame 795, got {ram_outcome:?}"
    );

    let out_scroll = ScratchDir::new("sf-attract-neg-scroll-out");
    run_cpu_fight_capture(run_a, &bad_assets, &out_scroll, SCENARIO_ID, 794, 980)
        .expect("perturbed scroll sleep capture runs");
    let nat_scroll = load_capture(&out_scroll).unwrap();
    let scope_scroll = CompareScope {
        from: Some(795),
        to: Some(980),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_scroll = compare(
        &scenario,
        &ref_cap,
        &nat_scroll,
        CandidateKind::Native,
        &scope_scroll,
    )
    .expect("compatible captures");
    let Outcome::Mismatch(diff_scroll) = outcome_scroll else {
        panic!(
            "scroll sleep one frame short must fail at that scroll step, got {outcome_scroll:?}"
        );
    };
    assert_eq!(
        diff_scroll.frame, 974,
        "scroll step 1 wakes at 974 instead of 975 when entry 0 sleeps 179 frames: {diff_scroll:?}"
    );

    // 3. Ticket #56 (F16) Acceptance 3 (C-NEG): an attract-ranking exit one
    //    frame early is caught at 1854. Shortening timer 3's initial inner
    //    counter (0x3f6f8..0x3f6fb: move.b #$02, ($e2e,A5) patched to #$01)
    //    causes timer 3 to set 0xff8e2f = 1 at frame 1854 instead of 1855, so
    //    0x42736..0x4274e clears the ranking text layer and exits to 0x3ce54
    //    one frame early at frame 1854.
    let mut early_ranking_assets = assets();
    assert_eq!(early_ranking_assets.maincpu[0x3f6fb], 0x02);
    early_ranking_assets.maincpu[0x3f6fb] = 0x01;

    let out_ranking = ScratchDir::new("sf-attract-neg-ranking-out");
    // run_cpu_fight_capture writes frames 794..=1854 (including the early exit's
    // text-layer clear and 0x3ce54..0x3ce86 demo setup at 1854).
    let frames_ranking = run_cpu_fight_capture(
        run_a,
        &early_ranking_assets,
        &out_ranking,
        SCENARIO_ID,
        794,
        1854,
    )
    .expect("early ranking exit runs 0x3ce54 at 1854");
    assert_eq!(frames_ranking, 1854 - 794 + 1);
    let scope_ranking = CompareScope {
        from: Some(795),
        to: Some(1854),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome_ranking = compare(
        &scenario,
        &ref_cap,
        &load_capture(&out_ranking).unwrap(),
        CandidateKind::Native,
        &scope_ranking,
    )
    .expect("compatible captures");
    let Outcome::Mismatch(diff_ranking) = outcome_ranking else {
        panic!("early ranking exit at 1854 must fail comparison, got {outcome_ranking:?}");
    };
    assert_eq!(
        diff_ranking.frame, 1854,
        "ranking exit one frame early is caught at frame 1854: {diff_ranking:?}"
    );
}

/// Ticket #77 (`F37`): Demo Sounds (`DSW1` bit 13) and Attract Music (`DSW1`
/// bit 9, `0xff8f25`) DIPs over the attract (`794..=1854`, plus power-on
/// `0..=1647`).
/// Verifies:
/// 1. `C-REF`: `attract-demosound-off-01-run-a == attract-demosound-off-01-run-b`
///    (`dsw1 = 0xffff`, `dsw2 = 0xffff`, 5,538 files) and
///    `attract-music-off-01-run-a == attract-music-off-01-run-b`
///    (`dsw1 = 0xddff`, `dsw2 = 0xffff`, 5,538 files).
/// 2. `C-NAT` & `C-STOP` from checkpoint `794` over `795..=1854` (stopping at
///    `1855` with `0x3ce54`):
///    - `sf-attract-demosound-off-01`: `0x3b80c` filters out both `0x1c` (975)
///      and `0x39` (1648) while `0xff8003 != 0` and `DSW1` bit 13 is high
///      (`Demo Sounds = Off`), so no sound command is latched over `795..=1854`
///      (`latches == []`);
///    - `sf-attract-music-off-01`: `0x3b286` clears `0xff8f25 = 0` when `DSW1`
///      bit 9 is low (`Attract Music = Off`), so `0x1c` is still latched at
///      frame `976` while `0x39` is skipped at `0x426f8` (`latches == [(976, 0x1c)]`);
///    - full work RAM (owned + non-stack/non-TCB bytes), `screen_hash`, `latch`,
///      `latch_changed`, checkpoint pixels, video RAM, and palette RAM match
///      both reference captures on every frame `795..=1854`.
#[test]
fn demo_sounds_off_and_attract_music_off_dips_match_reference_over_attract() {
    const DEMOSOUND_SCEN: &str = "../../tools/reference/scenarios/sf_attract_demosound_off_01.json";
    const DEMOSOUND_A: &str = "../../tools/reference/local/captures/attract-demosound-off-01-run-a";
    const DEMOSOUND_B: &str = "../../tools/reference/local/captures/attract-demosound-off-01-run-b";
    const MUSIC_OFF_SCEN: &str = "../../tools/reference/scenarios/sf_attract_music_off_01.json";
    const MUSIC_OFF_A: &str = "../../tools/reference/local/captures/attract-music-off-01-run-a";
    const MUSIC_OFF_B: &str = "../../tools/reference/local/captures/attract-music-off-01-run-b";

    if ![DEMOSOUND_A, DEMOSOUND_B, MUSIC_OFF_A, MUSIC_OFF_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: attract DIP reference captures/ROMs not present");
        return;
    }

    let assets = assets();
    let owned_set: BTreeSet<u32> = cpu_owned_addresses().into_iter().map(|(_, a)| a).collect();

    for (scen_path_str, scen_id, run_a_str, run_b_str, expected_dsw1, expected_latches) in [
        (
            DEMOSOUND_SCEN,
            "sf-attract-demosound-off-01",
            DEMOSOUND_A,
            DEMOSOUND_B,
            0xffffu16,
            vec![],
        ),
        (
            MUSIC_OFF_SCEN,
            "sf-attract-music-off-01",
            MUSIC_OFF_A,
            MUSIC_OFF_B,
            0xddffu16,
            vec![(976u32, 0x1cu8)],
        ),
    ] {
        let scen_path = Path::new(scen_path_str);
        let run_a = Path::new(run_a_str);
        let run_b = Path::new(run_b_str);
        let scenario = load_scenario(scen_path).expect("scenario loads");
        assert_eq!((scenario.dsw1, scenario.dsw2), (expected_dsw1, 0xffff));

        // 1. C-REF: run-a == run-b across all 5,538 observation files.
        let mut files: Vec<_> = std::fs::read_dir(run_a)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        let mut compared_files = 0;
        for a in &files {
            let name = a.file_name().unwrap().to_string_lossy().to_string();
            if !(name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba")) {
                continue;
            }
            let b = run_b.join(&name);
            assert_eq!(
                std::fs::read(a).unwrap(),
                std::fs::read(&b).unwrap_or_default(),
                "{scen_id}/{name}: run-a and run-b must be byte-for-byte identical"
            );
            compared_files += 1;
        }
        assert_eq!(
            compared_files, 5538,
            "{scen_id}: 1,107 checkpoints * 5 + 3 .tsv"
        );

        // 2. C-NAT over 795..=1900 (with scenario DSW applied and 0x3ce54..0x3ce86 translated in #88).
        let outcome = sf_engine::run::compare_cpu_ram_with_inputs(
            run_a,
            assets.maincpu.clone(),
            Some(scen_path),
            794,
            1900,
        )
        .unwrap();
        assert_eq!(
            outcome,
            CpuRamOutcome::Match { frames: 1900 - 794 },
            "{scen_id}: owned RAM 795..=1900"
        );

        let mut latches = Vec::new();
        let mut others = Vec::new();
        let (end_frame, unrec) = observe_cpu_fight(
            run_a,
            assets.maincpu.clone(),
            Some(scen_path),
            794,
            1854,
            |m| {
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
                            "{scen_id} frame {f}: 0x{addr:06x}: ref {r:#04x}, native {n:#04x}"
                        ));
                    }
                }
            },
        )
        .unwrap();
        assert!(
            others.is_empty(),
            "{scen_id} work RAM differed: {others:#?}"
        );
        assert_eq!((end_frame, unrec), (1854, Vec::<u32>::new()));
        assert_eq!(
            latches, expected_latches,
            "{scen_id}: expected sound latches over 795..=1854"
        );

        let out_dir = ScratchDir::new(&format!("{scen_id}-native"));
        let frames = sf_engine::run::run_cpu_fight_capture_with_inputs(
            run_a,
            &assets,
            &out_dir,
            scen_id,
            Some(scen_path),
            794,
            1854,
        )
        .expect("native attract DIP capture");
        assert_eq!(frames, 1854 - 794 + 1);

        let ref_cap = load_capture(run_a).unwrap();
        let nat_cap = load_capture(&out_dir).unwrap();
        let scope = CompareScope {
            from: Some(795),
            to: Some(1854),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        };
        let cmp = compare(&scenario, &ref_cap, &nat_cap, CandidateKind::Native, &scope)
            .expect("compatible captures");
        assert!(
            matches!(cmp, Outcome::Match { .. }),
            "{scen_id}: C-NAT full capture compare over 795..=1854 failed: {cmp:?}"
        );
    }
}

/// Ticket #88 (`F48`): Attract demo: setup through `FIGHT!` (`0x3ce54..0x3ce86`,
/// `0x3e5c6..0x3e5d0`, `0x3e6fe..0x3e70e`, and `0x4030c..0x40334`).
///
/// Verifies:
/// 1. `C-REF`: `sf-attract-demo-01` (`6,001` frames `0..=6000`, checkpoints at
///    `794` and `1850..=4750` through the second ranking's exit at `4697`) has
///    `attract-demo-01-run-a == attract-demo-01-run-b` (`14,513` observation
///    files), `frames.tsv` rows `0..=1900` equal `sf-attract-01`'s, and the
///    capture contains the demo's end (`3636`, `0x3ce8c`) and the second
///    ranking's exit (`4697`, `0x3ce54`).
/// 2. `C-NAT`: from checkpoint `794` through `1855` (slot 1 at `0x3e59c`, slot 2
///    at `0x3f7d6`), `0x3f` at `1856`, `0x28` at `1859`, the walk-in and `0x94`
///    at `1940`, and the entire demo fight up to the first untranslated branch
///    `0x3ce8c` at frame `3636` (`sf_compare::compare` over `794..=3635` and
///    full work RAM on every checkpoint `1850..=3635`).
/// 3. Every frame's `0xff8f2a` (demo input offset), P1's input byte (`0xff868f`)
///    and latches (`0xff86b0`, `0xff86b1`), and the random state
///    (`0xff8e4a..0xff8e51`) match on every checkpoint `1850..=3635`.
/// 4. A PC-coverage / routine probe over `1940..=3636` verifies every move,
///    effect stream, sound command, and routine exercised by the demo fight and
///    confirms that `0x3ce8c` (`0x3ce8c..0x3ceb8`, 9 instructions, reached once
///    at `3636`) is the only untranslated branch reached through the round end.
/// 5. `C-NEG`: starting the demo input index (`0xff8f2a`) at record 1 (byte
///    offset `3`) is caught on `0xff8f2a` at `1855`, on P1's input/latches at
///    `1858`, and on `sf_compare::compare`.
#[test]
fn attract_demo_setup_through_fight_and_round_end_matches_reference() {
    const DEMO_SCEN: &str = "../../tools/reference/scenarios/sf_attract_demo_01.json";
    const DEMO_A: &str = "../../tools/reference/local/captures/attract-demo-01-run-a";
    const DEMO_B: &str = "../../tools/reference/local/captures/attract-demo-01-run-b";

    if ![DEMO_A, DEMO_B, RUN_A, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: attract-demo-01 reference captures/ROMs not present");
        return;
    }

    let demo_scen = Path::new(DEMO_SCEN);
    let demo_a = Path::new(DEMO_A);
    let demo_b = Path::new(DEMO_B);
    let scenario = load_scenario(demo_scen).expect("sf-attract-demo-01 scenario");
    assert_eq!(scenario.duration_frames, 6001);
    assert_eq!(scenario.checkpoints.len(), 1 + (4750 - 1850 + 1));
    assert_eq!(scenario.checkpoints[0], 794);
    assert_eq!(scenario.checkpoints[1], 1850);
    assert_eq!(*scenario.checkpoints.last().unwrap(), 4750);

    // 1. C-REF: run-a == run-b across all 14,513 observation files, and rows
    //    0..=1900 equal sf-attract-01's.
    let mut files: Vec<_> = std::fs::read_dir(demo_a)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut compared_files = 0;
    for a in &files {
        let name = a.file_name().unwrap().to_string_lossy().to_string();
        if !(name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba")) {
            continue;
        }
        let b = demo_b.join(&name);
        assert_eq!(
            std::fs::read(a).unwrap(),
            std::fs::read(&b).unwrap_or_default(),
            "sf-attract-demo-01/{name}: run-a and run-b must be byte-for-byte identical"
        );
        compared_files += 1;
    }
    assert_eq!(
        compared_files, 14513,
        "2,902 checkpoints * 5 + 3 .tsv files"
    );

    let demo_rows = frame_rows(DEMO_A);
    let attract_rows = frame_rows(RUN_A);
    assert_eq!(demo_rows.len(), 6001);
    for f in 0..=1900u32 {
        assert_eq!(
            demo_rows.get(&f),
            attract_rows.get(&f),
            "frames.tsv row {f} must equal sf-attract-01"
        );
    }

    // Verify the reference capture contains the demo's end (3636, where 0x3ce8c
    // queues 0x3f and kills slot 2) and the second ranking's exit (4697, where
    // 0x3ce54 clears 0xff8f2a and spawns slot 2 at 0x3f7d6).
    let w_3635 = std::fs::read(demo_a.join("ram_003635_work.bin")).unwrap();
    let w_3636 = std::fs::read(demo_a.join("ram_003636_work.bin")).unwrap();
    let w_4696 = std::fs::read(demo_a.join("ram_004696_work.bin")).unwrap();
    let w_4697 = std::fs::read(demo_a.join("ram_004697_work.bin")).unwrap();
    assert_eq!(
        (w_3635[0x0026], w_3636[0x0026]),
        (8, 0),
        "slot 2 killed at demo end 3636"
    );
    assert_eq!(
        (
            u16::from_be_bytes([w_4696[0x0f2a], w_4696[0x0f2b]]),
            u16::from_be_bytes([w_4697[0x0f2a], w_4697[0x0f2b]]),
            w_4696[0x0026],
            w_4697[0x0026],
        ),
        (0x015c, 0x0000, 0, 8),
        "second ranking exits at frame 4697 (0xff8f2a cleared, slot 2 spawned)"
    );

    // 2 & 3 & 4. C-NAT from 794 through 4750 (covering the first demo setup at
    // 1855, 0x28 at 1859, 0x94 at 1940, the entire demo fight, 0x3ce8c..0x3ceb8
    // at 3636 with 0x3f latched at 3637 and slot 2 killed, and the second wall,
    // title and ranking through the second demo setup at 4697..=4750; tickets
    // #88 / F48 and #127 / F55).
    let assets = assets();
    let owned_set: BTreeSet<u32> = cpu_owned_addresses().into_iter().map(|(_, a)| a).collect();
    let mut latches = Vec::new();
    let mut others = Vec::new();
    let mut pos_1855 = Vec::new();
    let mut pos_1856 = Vec::new();
    let mut pos_1857 = Vec::new();
    let mut pos_1858 = Vec::new();
    let mut pos_3636 = Vec::new();
    let mut pos_4697 = Vec::new();
    let mut p1_moves = BTreeSet::new();
    let mut cpu_moves = BTreeSet::new();
    let mut effect_streams = BTreeSet::new();
    let mut max_f2a = 0u16;
    let mut prev_time_digits: Option<[u8; 3]> = None;
    let mut timer_tick_frames = Vec::new();

    let (end_frame, unrec) = observe_cpu_fight(
        demo_a,
        assets.maincpu.clone(),
        Some(demo_scen),
        794,
        4750,
        |m| {
            let f = m.state.frame;
            if f >= 1850 && m.state.sound_latch != 0xff {
                latches.push((f, m.state.sound_latch));
            }
            match f {
                1855 => pos_1855 = m.task_positions(),
                1856 => pos_1856 = m.task_positions(),
                1857 => pos_1857 = m.task_positions(),
                1858 => pos_1858 = m.task_positions(),
                3636 => pos_3636 = m.task_positions(),
                4697 => pos_4697 = m.task_positions(),
                _ => {}
            }
            if (1850..=4750).contains(&f) {
                let ref_work = std::fs::read(demo_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
                let nat_work = cpu_work_image(m);
                for (i, (&r, &n)) in ref_work.iter().zip(nat_work.iter()).enumerate() {
                    let addr = 0xff8000 + i as u32;
                    if r != n && (owned_set.contains(&addr) || area(addr) == "other") {
                        others.push(format!(
                            "frame {f}: 0x{addr:06x}: ref {r:#04x}, native {n:#04x}"
                        ));
                    }
                }
                // Explicit Acceptance 3 checks on every frame 1850..=4750:
                // 0xff8f2a..0xff8f2b, P1 input 0xff868f, P1 latches 0xff86b0..0xff86b1,
                // and random state 0xff8e4a..0xff8e51.
                assert_eq!(
                    &nat_work[0x0f2a..0x0f2c],
                    &ref_work[0x0f2a..0x0f2c],
                    "frame {f}: 0xff8f2a"
                );
                assert_eq!(
                    nat_work[0x068f], ref_work[0x068f],
                    "frame {f}: P1 input 0xff868f"
                );
                assert_eq!(
                    &nat_work[0x06b0..0x06b2],
                    &ref_work[0x06b0..0x06b2],
                    "frame {f}: P1 latches"
                );
                assert_eq!(
                    &nat_work[0x0e4a..0x0e52],
                    &ref_work[0x0e4a..0x0e52],
                    "frame {f}: random state"
                );

                if (1970..=3615).contains(&f) {
                    let cur = [nat_work[0x0e18], nat_work[0x0e19], nat_work[0x0e1a]];
                    if let Some(prev) = prev_time_digits {
                        if cur != prev {
                            timer_tick_frames.push(f);
                        }
                    }
                    prev_time_digits = Some(cur);
                }

                if f <= 3635 {
                    let f2a = u16::from_be_bytes([nat_work[0x0f2a], nat_work[0x0f2b]]);
                    max_f2a = max_f2a.max(f2a);
                    let p1_e = u16::from_be_bytes([nat_work[0x06c0], nat_work[0x06c1]]) as usize;
                    let cpu_e = u16::from_be_bytes([nat_work[0x4674], nat_work[0x4675]]) as usize;
                    if p1_e != 0 {
                        p1_moves.insert(nat_work[0x6000 + p1_e + 0x0a]);
                    }
                    if cpu_e != 0 {
                        cpu_moves.insert(nat_work[0x6000 + cpu_e + 0x0a]);
                    }
                    for k in 0..12usize {
                        let s = 0x100 + 0x40 * k;
                        if nat_work[0x6000 + s + 8] != 0 {
                            let stream = u32::from_be_bytes([
                                nat_work[0x6000 + s + 0x10],
                                nat_work[0x6000 + s + 0x11],
                                nat_work[0x6000 + s + 0x12],
                                nat_work[0x6000 + s + 0x13],
                            ]);
                            effect_streams.insert(stream);
                        }
                    }
                }
            }
        },
    )
    .unwrap();
    assert!(others.is_empty(), "work RAM differed: {others:#?}");
    assert_eq!(
        (end_frame, unrec),
        (4750, Vec::<u32>::new()),
        "demo fight, round end 0x3ce8c..0x3ceb8, and second attract loop run through 4750 with 0 untranslated branches"
    );
    // Ticket #127 (F55) Acceptance 3: every round-timer tick in the demo fight
    // is 19 frames apart throughout, with no frame overrun.
    assert!(!timer_tick_frames.is_empty());
    for pair in timer_tick_frames.windows(2) {
        assert_eq!(
            pair[1] - pair[0],
            19,
            "round timer ticks must be 19 frames apart throughout demo fight: {:?}",
            pair
        );
    }
    assert!(pos_1855.contains(&(1, 0x3e59c)) && pos_1855.contains(&(2, 0x3f7d6)));
    assert!(pos_1856.contains(&(1, 0x436e6)) && pos_1856.contains(&(2, 0x3f7d6)));
    assert!(pos_1857.contains(&(1, 0x43776)) && pos_1857.contains(&(2, 0x3f7d6)));
    assert!(pos_1858.contains(&(1, 0x3ea30)) && pos_1858.contains(&(2, 0x3f7d6)));
    assert!(
        pos_3636.contains(&(1, 0x3ccec)) && !pos_3636.iter().any(|&(slot, _)| slot == 2),
        "at 3636 0x3ce8c..0x3ceb8 kills slot 2 and sleeps 1 frame into 0x3ccec: {pos_3636:?}"
    );
    assert!(
        pos_4697.contains(&(1, 0x3e59c)) && pos_4697.contains(&(2, 0x3f7d6)),
        "at 4697 the second ranking exits into the next demo setup (0x3e59c / 0x3f7d6): {pos_4697:?}"
    );
    assert_eq!(
        &latches[..3],
        &[(1856, 0x3f), (1859, 0x28), (1940, 0x94)],
        "0x3f at 1856, 0x28 at 1859, and 0x94 (FIGHT!) at 1940"
    );
    assert!(
        latches.contains(&(3635, 0x09)) && latches.contains(&(3637, 0x3f)),
        "KO body thud 0x09 at 3635 and demo-end sound stop 0x3f latched at 3637"
    );
    assert!(
        latches.contains(&(3818, 0x1c))
            && latches.contains(&(4491, 0x39))
            && latches.contains(&(4698, 0x3f)),
        "second wall 0x1c at 3818, second ranking 0x39 at 4491, and second demo 0x3f at 4698"
    );
    assert_eq!(
        max_f2a, 0x0ffc,
        "demo input table reaches 0x0ffc (4,092) and wraps at 0x0fff (1,365 records) to 0, ending at 0x015c"
    );
    assert_eq!(
        p1_moves.len(),
        25,
        "25 distinct P1 moves exercised in demo fight"
    );
    assert_eq!(
        cpu_moves.len(),
        15,
        "15 distinct CPU Retsu moves exercised in demo fight"
    );
    assert_eq!(
        effect_streams.len(),
        20,
        "20 distinct effect stream positions exercised"
    );

    // Full sf_compare::compare over 794..=4750 (2,902 checkpoints: 794 and 1850..=4750).
    let out_demo = ScratchDir::new("sf-attract-demo-01-native");
    let frames_demo = sf_engine::run::run_cpu_fight_capture_with_inputs(
        demo_a,
        &assets,
        &out_demo,
        "sf-attract-demo-01",
        Some(demo_scen),
        794,
        4750,
    )
    .expect("native attract demo capture over 794..=4750");
    assert_eq!(frames_demo, 4750 - 794 + 1);

    let ref_demo = load_capture(demo_a).unwrap();
    let nat_demo = load_capture(&out_demo).unwrap();
    let scope_demo = CompareScope {
        from: Some(795),
        to: Some(4750),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let cmp_demo = compare(
        &scenario,
        &ref_demo,
        &nat_demo,
        CandidateKind::Native,
        &scope_demo,
    )
    .expect("compatible captures");
    assert!(
        matches!(
            cmp_demo,
            Outcome::Match {
                frames_compared: 3956,
                checkpoints_compared: 2901,
            }
        ),
        "C-NAT full capture compare over 795..=4750 failed: {cmp_demo:?}"
    );

    // 5. C-NEG: starting the demo input index (0xff8f2a) at record 1 (byte
    // offset 3) instead of record 0 (byte offset 0) is caught on 0xff8f2a at
    // frame 1855, on P1's input byte / latches at 1858, and on sf_compare::compare.
    let mut neg_m =
        sf_engine::cpu::CpuMachine::from_fight_checkpoint(demo_a, 1855, assets.maincpu.clone())
            .unwrap();
    neg_m.set_work_byte(0xff8f2b, 3);
    assert_ne!(
        &neg_m.work_ram()[0x0f2a..0x0f2c],
        &std::fs::read(demo_a.join("ram_001855_work.bin")).unwrap()[0x0f2a..0x0f2c],
        "record-1 start offset caught on 0xff8f2a at frame 1855"
    );
    for _ in 1855..1858 {
        neg_m.step();
    }
    let ref_1858 = std::fs::read(demo_a.join("ram_001858_work.bin")).unwrap();
    assert_eq!(
        (
            u16::from_be_bytes([neg_m.work_ram()[0x0f2a], neg_m.work_ram()[0x0f2b]]),
            neg_m.work_ram()[0x068f],
            ref_1858[0x068f],
        ),
        (6, 0xfe, 0xff),
        "at 1858 record 1 (0xfe) is read instead of record 0 (0xff), and 0xff8f2a advances to 6"
    );
    while neg_m.state.frame < 1971 {
        neg_m.step();
    }
    let ref_1971 = std::fs::read(demo_a.join("ram_001971_work.bin")).unwrap();
    assert_eq!(
        (neg_m.work_ram()[0x068f], ref_1971[0x068f]),
        (0xf6, 0xfe),
        "record-1 start offset shifts P1's forward-jump input (0xf6) one frame early to 1971"
    );
}

/// Ticket #127 (`F55`): Attract demo to its round end, and the attract loop
/// from power-on (`0x3ce8c..0x3ceb8`).
///
/// Verifies:
/// 1. `C-NAT` from power-on (`frame 0`) with no fixture through `6000` (the
///    demo's round end at `3636`, `0x3f` latched at `3637`, slot 2 killed at
///    `0x3ceb2`, the second wall `3637..=3817`, title `3818..=4509` and ranking
///    `4510..=4696`, up to the next demo setup at `4697` and through `6000`),
///    using `sf-boot-01` (`0..=829`, checkpoints `0..=799`), `sf-attract-01`
///    (`794..=1900`, checkpoints `794..=1900`), and `sf-attract-demo-01`
///    (`0..=6000`, checkpoints `794` and `1850..=4750`).
/// 2. `C-NEG`: delaying the demo's round end by one frame (`3636 -> 3637`, by
///    holding P1's KO fall busy for one extra frame at `3635`) is caught at
///    frame `3636` on work RAM (`0xff8026` slot 2 still active), video RAM, and
///    `sf_compare::compare`.
#[test]
fn attract_demo_round_end_and_loop_from_power_on_matches_reference_and_catches_late_end() {
    const BOOT_SCEN: &str = "../../tools/reference/scenarios/sf_boot_01.json";
    const BOOT_A: &str = "../../tools/reference/local/captures/boot-01-run-a";
    const DEMO_SCEN: &str = "../../tools/reference/scenarios/sf_attract_demo_01.json";
    const DEMO_A: &str = "../../tools/reference/local/captures/attract-demo-01-run-a";

    if ![BOOT_A, RUN_A, DEMO_A, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: power-on attract demo reference captures/ROMs not present");
        return;
    }

    let assets = assets();
    let owned_set: BTreeSet<u32> = cpu_owned_addresses().into_iter().map(|(_, a)| a).collect();
    let boot_scen = Path::new(BOOT_SCEN);
    let boot_a = Path::new(BOOT_A);
    let attract_scen = Path::new(SCENARIO);
    let attract_a = Path::new(RUN_A);
    let demo_scen = Path::new(DEMO_SCEN);
    let demo_a = Path::new(DEMO_A);

    // 1a. Power-on work-RAM observation over all three checkpoint windows:
    //     0..=799 (sf-boot-01), 794..=1900 (sf-attract-01), 1850..=4750 (sf-attract-demo-01).
    let mut others = Vec::new();
    let mut pos_3636 = Vec::new();
    let mut pos_4697 = Vec::new();
    let (end_f, unrec) = sf_engine::run::observe_power_on(assets.maincpu.clone(), 6000, |m| {
        let f = m.state.frame;
        if f == 3636 {
            pos_3636 = m.task_positions();
        }
        if f == 4697 {
            pos_4697 = m.task_positions();
        }
        let ref_dir = if f == 0 {
            None
        } else if f <= 799 {
            Some(boot_a)
        } else if f <= 1849 {
            Some(attract_a)
        } else if f <= 4750 {
            Some(demo_a)
        } else {
            None
        };
        if let Some(dir) = ref_dir {
            let ref_work = std::fs::read(dir.join(format!("ram_{f:06}_work.bin"))).unwrap();
            let nat_work = m.work_image();
            for (i, (&r, &n)) in ref_work.iter().zip(nat_work.iter()).enumerate() {
                let addr = 0xff8000 + i as u32;
                if r != n && (owned_set.contains(&addr) || area(addr) == "other") {
                    others.push(format!(
                        "power-on frame {f}: 0x{addr:06x}: ref {r:#04x}, native {n:#04x}"
                    ));
                }
            }
        }
    });
    assert!(others.is_empty(), "power-on work RAM differed: {others:#?}");
    assert_eq!((end_f, unrec), (6000, Vec::<u32>::new()));
    assert!(
        pos_3636.contains(&(1, 0x3ccec)) && !pos_3636.iter().any(|&(slot, _)| slot == 2),
        "power-on 3636: 0x3ce8c..0x3ceb8 kills slot 2 and enters 0x3ccec: {pos_3636:?}"
    );
    assert!(
        pos_4697.contains(&(1, 0x3e59c)) && pos_4697.contains(&(2, 0x3f7d6)),
        "power-on 4697: second ranking exits into second demo setup: {pos_4697:?}"
    );

    // 1b. Full power-on capture comparison over sf-boot-01 (0..=829),
    //     sf-attract-01 (794..=1900), and sf-attract-demo-01 (0..=6000).
    for (scen_path, ref_dir, scen_id, from_f, to_f, exp_frames, exp_ckpts) in [
        (
            boot_scen,
            boot_a,
            "sf-boot-01",
            0u64,
            829u64,
            830u64,
            800usize,
        ),
        (
            attract_scen,
            attract_a,
            "sf-attract-01",
            794,
            1900,
            1107,
            1107,
        ),
        (demo_scen, demo_a, "sf-attract-demo-01", 0, 6000, 6001, 2902),
    ] {
        let out = ScratchDir::new(&format!("{scen_id}-power-on-f55"));
        let frames = sf_engine::run::run_power_on_scenario_capture(
            &assets,
            &out,
            scen_id,
            scen_path,
            to_f as u32,
        )
        .expect("power-on capture runs with 0 untranslated branches");
        assert_eq!(frames, to_f + 1);
        let scen = load_scenario(scen_path).unwrap();
        let scope = CompareScope {
            from: Some(from_f),
            to: Some(to_f),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        };
        let cmp = compare(
            &scen,
            &load_capture(ref_dir).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &scope,
        )
        .expect("compatible captures");
        assert!(
            matches!(
                cmp,
                Outcome::Match {
                    frames_compared,
                    checkpoints_compared,
                } if frames_compared == exp_frames && checkpoints_compared == exp_ckpts
            ),
            "{scen_id}: power-on compare failed: {cmp:?}"
        );
    }

    // 2. C-NEG: the demo round end one frame late (3636 -> 3637, by keeping P1's
    //    entity busy in state 4 for one extra frame at 3635) is caught at 3636.
    let fix_dir = ScratchDir::new("sf-attract-demo-late-end-fix");
    std::fs::create_dir_all(&*fix_dir).unwrap();
    for entry in [
        "frames.tsv",
        "ram_003635_work.bin",
        "ram_003635_video.bin",
        "ram_003635_palette.bin",
        "ram_003636_work.bin",
    ] {
        std::fs::copy(demo_a.join(entry), fix_dir.join(entry)).unwrap();
    }
    let work_3635_path = fix_dir.join("ram_003635_work.bin");
    let mut work_3635 = std::fs::read(&work_3635_path).unwrap();
    // At 3635 slot 1 (TCB at 0xff8016) is sleeping at 0x3eb62 (Flow::KoSettle)
    // with sleep timer 0xff8017 == 1; setting 0xff8017 = 2 delays the wake-up
    // and 0x3ce8c by one frame (3636 -> 3637).
    assert_eq!(work_3635[0x0017], 1);
    work_3635[0x0017] = 2;
    std::fs::write(&work_3635_path, &work_3635).unwrap();

    let out_late = ScratchDir::new("sf-attract-demo-late-end-out");
    sf_engine::run::run_cpu_fight_capture_with_inputs(
        &fix_dir,
        &assets,
        &out_late,
        "sf-attract-demo-01",
        Some(demo_scen),
        3635,
        3638,
    )
    .expect("1-frame-late demo round end capture runs");
    let scen = load_scenario(demo_scen).unwrap();
    let scope_late = CompareScope {
        from: Some(3636),
        to: Some(3638),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let cmp_late = compare(
        &scen,
        &load_capture(demo_a).unwrap(),
        &load_capture(&out_late).unwrap(),
        CandidateKind::Native,
        &scope_late,
    )
    .expect("compatible captures");
    let Outcome::Mismatch(diff_late) = cmp_late else {
        panic!("demo round end 1 frame late must fail at 3636, got {cmp_late:?}");
    };
    assert_eq!(
        diff_late.frame, 3636,
        "demo round end 1 frame late is caught at frame 3636: {diff_late:?}"
    );
}
