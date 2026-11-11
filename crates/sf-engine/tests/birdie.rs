//! Ticket #131 (`F59`): Ryu beats stage 3's first opponent (`0x17a9c`, Birdie)
//! and reaches Eagle's (`0x17d0c`) setup and round-1 fight loop.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, scenario_input_events,
    CpuRamOutcome, ScratchDir, INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_07.json";
const RUN_A: &str = "../../tools/reference/local/captures/win-07-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/win-07-run-b";
const WIN_06_A: &str = "../../tools/reference/local/captures/win-06-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 26371;
const LAST: u32 = 29427;
const FRAMES: u32 = LAST - FIRST;

fn present() -> bool {
    [RUN_A, RUN_B, WIN_06_A, ROMS]
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
fn reference_runs_repeat_and_equal_win_06_through_divergence() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).unwrap();
    let cap_a = load_capture(Path::new(RUN_A)).unwrap();
    let cap_b = load_capture(Path::new(RUN_B)).unwrap();
    let outcome = compare(
        &scenario,
        &cap_a,
        &cap_b,
        CandidateKind::Reference,
        &scope(),
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
        "sf-win-07 repeat: {outcome:?}"
    );

    let mut win_06_a = load_capture(Path::new(WIN_06_A)).unwrap();
    win_06_a.scenario_id = Some(scenario.scenario_id.clone());
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(26372),
        fields: None,
    };
    let prefix_outcome = compare(
        &scenario,
        &win_06_a,
        &cap_a,
        CandidateKind::Reference,
        &prefix_scope,
    )
    .unwrap();
    assert!(
        matches!(
            prefix_outcome,
            Outcome::Match {
                frames_compared: 26373,
                checkpoints_compared: 2,
            }
        ),
        "sf-win-07 prefix vs sf-win-06: {prefix_outcome:?}"
    );
}

#[test]
fn native_lee_match_matches_every_observable_and_owned_ram() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-07-birdie");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-07",
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
        "sf-win-07 native observables: {outcome:?}"
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
    assert_eq!(
        ram_outcome,
        CpuRamOutcome::Match { frames: FRAMES },
        "sf-win-07 owned RAM over {FIRST}..={LAST}"
    );
}

#[test]
fn tampered_lee_ko_checkpoint_and_missing_0x477ca_are_caught() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const KO_FRAME: u32 = 26974;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-07-birdie-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-07",
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
        panic!("tampered Birdie KO checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, KO_FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Negative control 2: patching Birdie's active frame 0x11768 (+0x1c = 0x11784)
    // from 0x477ca to untranslated 0x492d6 stops native execution at frame 26695
    // when Birdie's 0x477ca attack lands.
    let mut bad_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    assert_eq!(
        u32::from_be_bytes(bad_rom[0x11784..0x11788].try_into().unwrap()),
        0x0004_77ca
    );
    bad_rom[0x11784..0x11788].copy_from_slice(&0x0004_92d6u32.to_be_bytes());
    let stop_outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        bad_rom,
        Some(Path::new(SCENARIO)),
        FIRST,
        26700,
    )
    .unwrap();
    assert_eq!(
        stop_outcome,
        CpuRamOutcome::Stopped {
            frames: 26695 - FIRST - 1,
            frame: 26695,
            unrecovered: vec![0x492d6],
        }
    );
}

#[test]
fn direct_replay_exercises_four_lee_hit_functions_and_reaches_gens_setup() {
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
    while machine.state.frame < LAST {
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
            (26419, 0x47612, 48, 40),
            (26507, 0x47680, 40, 28),
            (26695, 0x477ca, 28, 14),
            (27797, 0x478a6, 48, 36),
            (27798, 0x478a6, 36, 22),
        ]
    );
    assert_eq!(machine.state.frame, LAST);
    assert!(machine.task_positions().contains(&(1, 0x3eafa)));
    let w = machine.work_ram();
    assert_eq!(
        u32::from_be_bytes(w[0x4676..0x467a].try_into().unwrap()),
        0x17d0c
    );
    assert_eq!((w[0x4003], w[0x4004], w[0x4009], w[0x400a]), (3, 1, 0, 0));
}

#[test]
fn reference_has_the_expected_ko_match_and_gen_setup_landmarks() {
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
        assert_eq!(
            u32::from_be_bytes(w[off + 60..off + 64].try_into().unwrap()),
            0x3b5a8
        );
        u32::from_be_bytes(w[off + 64..off + 68].try_into().unwrap())
    };
    assert_eq!(slot1_resume(&read(26371)), 0x3ea3e);
    assert_eq!(slot1_resume(&read(26372)), 0x3eafa);
    assert_eq!(
        (read(26973)[0x6b8], read(26974)[0x6b8], read(26974)[0xe36]),
        (0, 1, 7)
    );
    assert_eq!(
        (read(28173)[0x6b8], read(28174)[0x6b8], read(28174)[0xe36]),
        (1, 2, 7)
    );
    assert_eq!(read(28625)[0xe36], 0x0b, "Birdie's match-won screen");
    assert_eq!(read(28896)[0xe36], 6, "the match-end wipe");
    assert_eq!((read(28995)[0xe36], read(28995)[0x4004]), (4, 1));
    assert_eq!(read(29147)[0xe36], 5, "Eagle's ROUND 1 event");
    let setup = read(29316);
    assert_eq!(
        u32::from_be_bytes(setup[0x4676..0x467a].try_into().unwrap()),
        0x17d0c
    );
    assert_eq!(slot1_resume(&read(29426)), 0x3ea3e);
    let first_loop = read(29427);
    assert_eq!(slot1_resume(&first_loop), 0x3eafa);
    assert_eq!(first_loop[0x4008], 0x80);
}
