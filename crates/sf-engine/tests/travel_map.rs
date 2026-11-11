//! Ticket #83 (F43): Country to country: the first bonus to stage 1's first
//! fight (the travel map `0x3f812..0x3f8e2` and plane effect stream routines
//! `0x432c4..0x43339`).
//!
//! Verifies:
//! 1. `sf-win-02` travel-map to stage-1 fight loop (`10028..=10841`): reference
//!    runs repeat byte-for-byte (`C-REF`).
//! 2. `sf-win-02` travel-map to stage-1 fight loop (`10028..=10841`): native
//!    execution matches every captured observable (`screen_sha256`,
//!    `sound_latch`, `latch_cycles`, checkpoint `pixels_*.rgba`) and every
//!    owned RAM byte through event 2's map flight (`0x3f812..0x3f8a0`),
//!    event 3's country intro (`0x3f8e4..0x3f97a`), event 4's stage intro
//!    (`0x3f9ec..0x3fb3e`), and event 5's `ROUND 1` (`0x3fba2..0x3fc60`) to
//!    Joe's first fight-loop frame (`10841`, `0x3eafa`, `0xffe430 == 0x1734c`).
//! 3. Negative-control test: tampering a travel-map checkpoint pixel hash or
//!    owned effect RAM byte fails at that exact frame.
//! 4. Plane stream routines (`0x432c4` loop start, `0x432e0` loop step/exit,
//!    `0x43306` y-flip, `0x4331a` sound queue) and all four starting-country
//!    plane tables (`0x3f8a4..0x3f8d4`) are exercised and verified on their
//!    frames.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, CpuRamOutcome, ScratchDir,
    INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_02.json";
const RUN_A: &str = "../../tools/reference/local/captures/win-02-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/win-02-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 10028;
const LAST: u32 = 10841;
const FRAMES: u32 = LAST - FIRST;

fn present() -> bool {
    [RUN_A, RUN_B, ROMS].iter().all(|p| Path::new(p).is_dir())
}

fn scope() -> CompareScope {
    CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(LAST as u64),
        fields: None,
    }
}

#[test]
fn win_02_travel_map_to_stage_1_reference_runs_repeat_exactly() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).unwrap();
    let outcome = compare(
        &scenario,
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(Path::new(RUN_B)).unwrap(),
        CandidateKind::Reference,
        &scope(),
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == FRAMES as u64 && checkpoints_compared == FRAMES as usize),
        "sf-win-02 travel map: {outcome:?}"
    );
}

#[test]
fn native_win_02_travel_map_to_stage_1_matches_every_observable_and_owned_ram() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-02-travel-map");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-02",
            Some(Path::new(SCENARIO)),
            FIRST,
            LAST,
        )
        .unwrap(),
        (FRAMES + 1) as u64,
    );
    let native_scope = CompareScope {
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        ..scope()
    };
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &native_scope,
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Match { frames_compared, checkpoints_compared }
            if frames_compared == FRAMES as u64 && checkpoints_compared == FRAMES as usize),
        "sf-win-02 travel map: {outcome:?}"
    );

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        rom,
        Some(Path::new(SCENARIO)),
        FIRST,
        LAST,
    )
    .unwrap();
    assert_eq!(ram_outcome, CpuRamOutcome::Match { frames: FRAMES });
}

#[test]
fn tampered_travel_map_checkpoint_and_ram_are_caught_at_exact_frame() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const FRAME: u32 = 10160;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-travel-map-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-02",
            Some(Path::new(SCENARIO)),
            FIRST,
            FRAME,
        )
        .unwrap(),
        (FRAME - FIRST + 1) as u64,
    );
    let checkpoints = out.join("checkpoints.tsv");
    let text = std::fs::read_to_string(&checkpoints).unwrap();
    let prefix = format!("{FRAME}\t");
    let line = text
        .lines()
        .find(|l| l.starts_with(&prefix))
        .unwrap()
        .to_string();
    let parts: Vec<&str> = line.split('\t').collect();
    let crc = u32::from_str_radix(parts[1], 16).unwrap() ^ 1;
    let altered_line = format!("{FRAME}\t{crc:08x}\t{}", parts[2]);
    std::fs::write(&checkpoints, text.replace(&line, &altered_line)).unwrap();

    let pixels_path = out.join(format!("pixels_{FRAME:06}.rgba"));
    let mut pixels = std::fs::read(&pixels_path).unwrap();
    pixels[0] ^= 0xff;
    std::fs::write(&pixels_path, &pixels).unwrap();

    let negative_scope = CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(FRAME as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &negative_scope,
    )
    .unwrap();
    let Outcome::Mismatch(mismatch) = outcome else {
        panic!("tampered travel-map checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Also verify owned effect RAM tamper detection at frame 10160 (plane loop counter 0xffe128).
    for entry in [
        "ram_010159_work.bin",
        "ram_010159_video.bin",
        "ram_010159_palette.bin",
        "ram_010160_work.bin",
    ] {
        std::fs::copy(Path::new(RUN_A).join(entry), out.join(entry)).unwrap();
    }
    let work_path = out.join("ram_010160_work.bin");
    let mut work = std::fs::read(&work_path).unwrap();
    work[0x6128] ^= 1;
    std::fs::write(&work_path, &work).unwrap();
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome =
        compare_fight_ram_with_inputs(&out, rom, Some(Path::new(SCENARIO)), FRAME - 1, FRAME)
            .unwrap();
    let CpuRamOutcome::Mismatch(msg) = ram_outcome else {
        panic!("tampered travel-map RAM unexpectedly matched: {ram_outcome:?}");
    };
    assert!(
        msg.contains("frame 10160: object RAM (both fighters, effects) differs first at 0xffe128"),
        "{msg}"
    );
}

#[test]
fn plane_loop_sound_and_flip_routines_and_all_country_tables_are_exercised() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();

    // 1. Step sf-win-02 from 10028 through 10841 and verify key travel-map and stage-1 landmarks.
    let mut m = FightMachine::from_fight_checkpoint(Path::new(RUN_A), FIRST, rom.clone()).unwrap();
    apply_inputs(&mut m, &[false; INPUTS]);
    m.step(); // 10029: 0x3efc4 -> 0x3e59c -> 0x3f812 (sleeps 120 frames at 0x3f87c)
    assert_eq!(m.state.frame, 10029);
    assert_eq!(m.work_ram()[0x4003], 1);
    assert_eq!(m.state.fg_scroll, 0x1980);
    assert!(m.task_positions().contains(&(2, 0x3f87c)));

    while m.state.frame < 10149 {
        m.step();
    }
    // Frame 10149: 0x3f87c spawns plane template 0x18b20 (30-tick initial hold, stream 0x190d4)
    // into effect slot 0 (`0xffe100`) and ticks once (`eb(s, 8): 1 -> 2`).
    assert!(m.task_positions().contains(&(2, 0x3f890)));
    assert_eq!(
        m.state.objectram[0x108], 2,
        "effect slot 0 live on frame 10149"
    );

    // Frame 10182: after the template hold and 6-tick opening stream frame at 0x190d4,
    // 0x4331a queues sound 0xff and 0x432c4 arms loop count 0x14 at 0xffe128 and loop target
    // 0x190e8 (2 ticks per iteration) at 0xffe12a.
    while m.state.frame < 10182 {
        m.step();
    }
    assert_eq!(m.state.objectram[0x128], 0x14);
    assert_eq!(
        u32::from_be_bytes(m.state.objectram[0x12a..0x12e].try_into().unwrap()),
        0x190e8
    );

    // Frame 10183: vblank pops sound 0xff; frame 10184: first 0x432e0 loop tick decrements
    // 0xffe128 to 0x13.
    m.step();
    assert_eq!(m.state.sound_latch, 0xff);
    m.step();
    assert_eq!(m.state.objectram[0x128], 0x13);

    // Frame 10224: loop finishes (21 iterations * 2 frames = 42 frames; 0xffe128 == 0 ->
    // 0x432f8) and 0x4331a queues landing sound 0xbe; frame 10225 pops sound 0xbe.
    while m.state.frame < 10224 {
        m.step();
    }
    assert_eq!(m.state.objectram[0x128], 0);
    m.step(); // 10225: pops sound 0xbe
    assert_eq!(m.state.sound_latch, 0xbe);

    // Run through the end of the plane flight (0xfd despawns the plane at 10256 and enters the
    // 60-frame hold at 0x3f8a0) and all the way to frame 10841 (Joe's first fight-loop frame).
    while m.state.objectram[0x108] != 0 {
        m.step();
    }
    assert_eq!(m.state.frame, 10256);
    m.step();
    assert!(m.task_positions().contains(&(2, 0x3f8a0)));

    while m.state.frame < LAST {
        m.step();
    }
    assert_eq!(m.state.frame, LAST);
    assert!(m.task_positions().contains(&(1, 0x3eafa)));
    assert_eq!(
        u32::from_be_bytes(m.work_ram()[0x4676..0x467a].try_into().unwrap()),
        0x1734c,
        "Joe's move table at stage 1 fight 0"
    );

    // 2. Exercise 0x43306 (plane_flip) and all 16 entries across the 4 starting-country tables
    // (`0x3f822` -> `0x3f8a4` Japan, `0x3f8b4` USA, `0x3f8c4` China, `0x3f8d4` England; stages 1..=4).
    let mut flipped_routes = Vec::new();
    for country in 0..4u8 {
        let table_ptr = 0x3f822 + (country as usize) * 4;
        let table = u32::from_be_bytes(rom[table_ptr..table_ptr + 4].try_into().unwrap()) as usize;
        for stage in 1..=4u8 {
            let entry = table + ((stage - 1) as usize) * 4;
            let expected_template =
                u32::from_be_bytes(rom[entry..entry + 4].try_into().unwrap()) as usize;
            let expected_stream = u32::from_be_bytes(
                rom[expected_template + 0x10..expected_template + 0x14]
                    .try_into()
                    .unwrap(),
            );
            let mut mc =
                FightMachine::from_fight_checkpoint(Path::new(RUN_A), FIRST, rom.clone()).unwrap();
            mc.set_work_byte(0xffc006, country);
            mc.set_work_byte(0xffc003, stage - 1);
            apply_inputs(&mut mc, &[false; INPUTS]);
            // Step 1 frame to enter travel_map_start (120-frame hold), then 120 frames to spawn.
            for _ in 0..121 {
                mc.step();
            }
            let stream_ptr =
                u32::from_be_bytes(mc.state.objectram[0x110..0x114].try_into().unwrap());
            assert_eq!(
                stream_ptr, expected_stream,
                "country={country} stage={stage}"
            );
            let initial_flip = mc.state.objectram[0x102] & 2 != 0;
            // Step through the plane flight until it despawns and verify no unrecovered branches.
            let mut flipped_during_flight = false;
            for _ in 0..250 {
                mc.step();
                assert!(
                    mc.unrecovered().is_empty(),
                    "country={country} stage={stage} unrecovered={:?}",
                    mc.unrecovered()
                );
                if !initial_flip && (mc.state.objectram[0x102] & 2 != 0) {
                    flipped_during_flight = true;
                }
                if mc.state.objectram[0x108] == 0 {
                    break;
                }
            }
            assert_eq!(
                mc.state.objectram[0x108], 0,
                "plane finished flight for country={country} stage={stage}"
            );
            if flipped_during_flight {
                flipped_routes.push((country, stage));
            }
        }
    }
    assert!(
        !flipped_routes.is_empty(),
        "0x43306 (plane_flip) must be exercised by curved travel-map routes"
    );
}
