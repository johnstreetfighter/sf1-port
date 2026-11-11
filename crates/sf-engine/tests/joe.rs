//! Ticket #68: Ryu beats Joe and reaches Mike's first new hit function.
//! Both reference runs repeat exactly. The native run matches every captured
//! observable and every owned RAM byte through 15543, then reports 0x4699c.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, scenario_input_events,
    CpuRamOutcome, ScratchDir, INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_03.json";
const RUN_A: &str = "../../tools/reference/local/captures/win-03-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/win-03-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIRST: u32 = 10840;
const LAST: u32 = 15543;
const BOUNDARY: u32 = 15544;
const FRAMES: u32 = BOUNDARY - FIRST - 1;

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
fn reference_runs_repeat_exactly() {
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
        "sf-win-03: {outcome:?}"
    );
}

#[test]
fn native_joe_match_matches_every_observable() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-03-joe");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-03",
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
        "sf-win-03: {outcome:?}"
    );
}

#[test]
fn tampered_joe_hit_checkpoint_is_caught_at_its_frame() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const FRAME: u32 = 12192;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-03-joe-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-03",
            Some(Path::new(SCENARIO)),
            FIRST,
            FRAME,
        )
        .unwrap(),
        (FRAME - FIRST + 1) as u64,
    );
    let checkpoints = out.join("checkpoints.tsv");
    let text = std::fs::read_to_string(&checkpoints).unwrap();
    let row = format!("{FRAME}\t3b1b0813\tpixels_{FRAME:06}.rgba");
    assert!(text.contains(&row), "unchanged native checkpoint row");
    let altered = text.replace(&row, &row.replace("3b1b0813", "3b1b0812"));
    std::fs::write(checkpoints, altered).unwrap();
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
        panic!("tampered Joe checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");
}

#[test]
fn native_joe_match_reaches_mikes_first_new_hit_function() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        rom,
        Some(Path::new(SCENARIO)),
        FIRST,
        BOUNDARY,
    )
    .unwrap();
    assert_eq!(outcome, CpuRamOutcome::Match { frames: FRAMES + 1 });
}

#[test]
fn direct_replay_reaches_the_same_boundary_and_next_fight_state() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut machine = FightMachine::from_fight_checkpoint(Path::new(RUN_A), FIRST, rom).unwrap();
    let events = scenario_input_events(Path::new(SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; INPUTS]);
    while machine.state.frame < BOUNDARY {
        let frame = machine.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut machine, &held);
        machine.step();
        if machine.state.frame < BOUNDARY {
            assert!(
                machine.unrecovered().is_empty(),
                "frame {}: {:05x?}",
                machine.state.frame,
                machine.unrecovered()
            );
        }
    }
    assert_eq!(
        (machine.state.frame, machine.unrecovered()),
        (BOUNDARY, vec![])
    );
    assert_eq!(
        (
            machine.work_ram()[0x4003],
            machine.work_ram()[0x4004],
            machine.work_ram()[0x4009],
            machine.work_ram()[0x400a],
        ),
        (1, 1, 0, 1),
    );
}

#[test]
fn reference_has_the_expected_ko_match_and_next_setup_landmarks() {
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
    assert_eq!(
        (read(11171)[0x6b8], read(11172)[0x6b8], read(11172)[0xe36]),
        (0, 1, 7)
    );
    assert_eq!(
        (read(12371)[0x6b8], read(12372)[0x6b8], read(12372)[0xe36]),
        (1, 2, 7)
    );
    assert_eq!(read(12910)[0xe36], 0x0b, "Joe's match-won screen");
    assert_eq!(read(13181)[0xe36], 6, "the match-end wipe");
    assert_eq!((read(13280)[0xe36], read(13280)[0x4004]), (4, 1));
    assert_eq!(read(13432)[0xe36], 5, "Mike's ROUND 1 event");
    let setup = read(13601);
    assert_eq!(
        u32::from_be_bytes(setup[0x4676..0x467a].try_into().unwrap()),
        0x170dc
    );
    assert_eq!(slot1_resume(&read(13711)), 0x3ea3e);
    let first_loop = read(13712);
    assert_eq!(slot1_resume(&first_loop), 0x3eafa);
    assert_eq!(first_loop[0x4008], 0x80);
}
