//! Ticket #129 (`F57`): Ryu beats stage 2's second opponent (`0x1782c`, Gen),
//! reaches `0x3ef96`'s dispatch to the stage-2 bonus (`0x41a02`), and in the
//! reference capture continues through the stage-2 bonus into stage 3 (`0x17a9c`,
//! Birdie).

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, scenario_input_events,
    CpuRamOutcome, ScratchDir, INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_06.json";
const RUN_A: &str = "../../tools/reference/local/captures/win-06-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/win-06-run-b";
const WIN_05_A: &str = "../../tools/reference/local/captures/win-05-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 22265;
const LAST: u32 = 24853;
const BOUNDARY: u32 = 24854;
const FRAMES: u32 = LAST - FIRST;

const REF_LAST: u32 = 26372;
const REF_FRAMES: u32 = REF_LAST - FIRST;

fn present() -> bool {
    [RUN_A, RUN_B, WIN_05_A, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn scope() -> CompareScope {
    CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(LAST as u64),
        fields: None,
    }
}

#[test]
fn reference_runs_repeat_and_equal_win_05_through_divergence() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).unwrap();
    let cap_a = load_capture(Path::new(RUN_A)).unwrap();
    let cap_b = load_capture(Path::new(RUN_B)).unwrap();
    let ref_scope = CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(REF_LAST as u64),
        fields: None,
    };
    let outcome = compare(
        &scenario,
        &cap_a,
        &cap_b,
        CandidateKind::Reference,
        &ref_scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == REF_FRAMES as u64 && checkpoints_compared == REF_FRAMES as usize
        ),
        "sf-win-06 repeat: {outcome:?}"
    );

    let mut win_05_a = load_capture(Path::new(WIN_05_A)).unwrap();
    win_05_a.scenario_id = Some(scenario.scenario_id.clone());
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(22266),
        fields: None,
    };
    let prefix_outcome = compare(
        &scenario,
        &win_05_a,
        &cap_a,
        CandidateKind::Reference,
        &prefix_scope,
    )
    .unwrap();
    assert!(
        matches!(
            prefix_outcome,
            Outcome::Match {
                frames_compared: 22267,
                checkpoints_compared: 2,
            }
        ),
        "sf-win-06 prefix vs sf-win-05: {prefix_outcome:?}"
    );
}

#[test]
fn native_eagle_match_matches_every_observable_and_owned_ram() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-06-gen");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-06",
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
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == FRAMES as u64 && checkpoints_compared == FRAMES as usize
        ),
        "sf-win-06 native observables: {outcome:?}"
    );

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let ram_outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        rom.clone(),
        Some(Path::new(SCENARIO)),
        FIRST,
        LAST,
    )
    .unwrap();
    assert_eq!(
        ram_outcome,
        CpuRamOutcome::Match { frames: FRAMES },
        "sf-win-06 owned RAM over {FIRST}..={LAST}"
    );

    let stop_outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        rom,
        Some(Path::new(SCENARIO)),
        FIRST,
        BOUNDARY,
    )
    .unwrap();
    assert_eq!(
        stop_outcome,
        CpuRamOutcome::Match { frames: FRAMES + 1 },
        "sf-win-06 reaches 0x3ef96's dispatch to 0x41a02 (sleeping at 0x41aa6) on frame {BOUNDARY}"
    );
}

#[test]
fn tampered_eagle_ko_checkpoint_and_missing_0x47310_are_caught() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const KO_FRAME: u32 = 22488;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-06-gen-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-06",
            Some(Path::new(SCENARIO)),
            FIRST,
            KO_FRAME,
        )
        .unwrap(),
        (KO_FRAME - FIRST + 1) as u64,
    );
    let checkpoints = out.join("checkpoints.tsv");
    let text = std::fs::read_to_string(&checkpoints).unwrap();
    let suffix = format!("\tpixels_{KO_FRAME:06}.rgba");
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{KO_FRAME}\t")) && l.ends_with(&suffix))
        .expect("checkpoint row for KO_FRAME")
        .to_string();
    let cols: Vec<&str> = line.split('\t').collect();
    let orig_crc = cols[1];
    let bad_crc = if let Some(prefix) = orig_crc.strip_suffix('0') {
        format!("{prefix}1")
    } else {
        format!("{}0", &orig_crc[..orig_crc.len() - 1])
    };
    let altered = text.replacen(
        &line,
        &format!("{KO_FRAME}\t{bad_crc}\tpixels_{KO_FRAME:06}.rgba"),
        1,
    );
    std::fs::write(&checkpoints, altered).unwrap();

    let negative_scope = CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(KO_FRAME as u64),
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
        panic!("tampered Gen KO checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, KO_FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Negative control 2: patching Gen's active frames 0x10bd4, 0x10bf4, and
    // 0x10c14 (+0x1c = 0x10bf0, 0x10c10, 0x10c30) from 0x47310 to untranslated
    // 0x492d6 stops native execution at frame 23845 when Gen's 0x47310 attack
    // lands.
    let mut bad_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    for off in [0x10bf0usize, 0x10c10, 0x10c30] {
        assert_eq!(
            u32::from_be_bytes(bad_rom[off..off + 4].try_into().unwrap()),
            0x0004_7310
        );
        bad_rom[off..off + 4].copy_from_slice(&0x0004_92d6u32.to_be_bytes());
    }
    let stop_outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        bad_rom,
        Some(Path::new(SCENARIO)),
        FIRST,
        23850,
    )
    .unwrap();
    assert_eq!(
        stop_outcome,
        CpuRamOutcome::Stopped {
            frames: 23845 - FIRST - 1,
            frame: 23845,
            unrecovered: vec![0x492d6],
        }
    );
}

#[test]
fn direct_replay_exercises_eagle_hit_functions_and_reaches_bonus_boundary() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut machine = FightMachine::from_fight_checkpoint(Path::new(RUN_A), FIRST, rom).unwrap();
    let events = scenario_input_events(Path::new(SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; INPUTS]);
    let mut prev_p1_health = 48u8;
    let mut landed_hits = Vec::new();
    while machine.state.frame < BOUNDARY {
        let frame = machine.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut machine, &held);
        machine.step();
        assert!(
            machine.unrecovered().is_empty(),
            "frame {}: {:05x?}",
            machine.state.frame,
            machine.unrecovered()
        );
        let w = machine.work_ram();
        let p1_h = w[0x6b7] * 4 + w[0x6b6];
        if p1_h < prev_p1_health && w[0x4008] == 0x80 {
            let cpu_ent = u16::from_be_bytes([w[0x4674], w[0x4675]]) as usize;
            let hf = u32::from_be_bytes(
                machine.state.objectram[cpu_ent + 0x28..cpu_ent + 0x2c]
                    .try_into()
                    .unwrap(),
            );
            landed_hits.push((frame, hf, prev_p1_health, p1_h));
        }
        prev_p1_health = p1_h;
    }
    assert_eq!(
        landed_hits,
        vec![
            (23691, 0x4745a, 48, 36),
            (23845, 0x47310, 36, 24),
            (23862, 0x47310, 24, 12),
        ]
    );
    assert_eq!(machine.state.frame, BOUNDARY);
    assert!(machine.task_positions().contains(&(1, 0x41aa6)));
}

#[test]
fn reference_has_the_expected_ko_match_bonus_and_lee_setup_landmarks() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let read = |frame: u32| {
        std::fs::read(Path::new(RUN_A).join(format!("ram_{frame:06}_work.bin"))).unwrap()
    };
    let slot1_resume = |w: &[u8]| {
        assert_eq!(w[0x16], 8);
        let usp = u32::from_be_bytes(w[0x1e..0x22].try_into().unwrap()) as usize;
        let off = usp - 0xff8000;
        let ret = u32::from_be_bytes(w[off + 60..off + 64].try_into().unwrap());
        assert!(
            ret == 0x3b5a8 || ret == 0x3b518,
            "unexpected sleep return 0x{ret:05x}"
        );
        u32::from_be_bytes(w[off + 64..off + 68].try_into().unwrap())
    };
    assert_eq!(
        (read(22487)[0x6b8], read(22488)[0x6b8], read(22488)[0xe36]),
        (0, 1, 7),
        "Ryu wins round 1 at 22488"
    );
    assert_eq!(read(23047)[0xe36], 6, "round-end wipe at 23047");
    assert_eq!(read(23146)[0xe36], 5, "Gen ROUND 2 announce at 23146");
    assert_eq!(
        (read(24107)[0x6b8], read(24108)[0x6b8], read(24108)[0xe36]),
        (1, 2, 7),
        "Ryu wins round 2 at 24108"
    );
    assert_eq!(read(24484)[0xe36], 0x0b, "Gen's match-won screen at 24484");
    assert_eq!(read(24755)[0xe36], 6, "the match-end wipe at 24755");
    assert_eq!(
        slot1_resume(&read(BOUNDARY)),
        0x41aa6,
        "stage-2 bonus setup at 24854 sleeps into 0x41aa6"
    );
    assert_eq!(
        (read(25581)[0x4003], read(25582)[0x4003]),
        (2, 3),
        "stage step 0x3efc4 advances stage 2 -> 3 at 25582"
    );
    assert_eq!(
        u32::from_be_bytes(read(26261)[0x4676..0x467a].try_into().unwrap()),
        0x17a9c,
        "Birdie's move-table pointer 0x17a9c at 26261"
    );
    assert_eq!(
        slot1_resume(&read(26371)),
        0x3ea3e,
        "Birdie round-1 fixture at 26371"
    );
    let first_loop = read(26372);
    assert_eq!(
        slot1_resume(&first_loop),
        0x3eafa,
        "Birdie first fight-loop frame at 26372"
    );
    assert_eq!(first_loop[0x4008], 0x80);
}
