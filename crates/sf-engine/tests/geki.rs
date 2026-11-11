//! Ticket #67: Ryu beats Geki and enters the Japan bonus stage (`0x40d30`,
//! translated in ticket #69, F29). Both reference runs repeat exactly. The
//! native run matches every captured observable and every owned RAM byte
//! through 9278 (`0x40dd4`).

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, scenario_input_events,
    CpuRamOutcome, ScratchDir, INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_02.json";
const RUN_A: &str = "../../tools/reference/local/captures/win-02-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/win-02-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIRST: u32 = 5860;
const LAST: u32 = 9277;
const BOUNDARY: u32 = 9278;
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
        "sf-win-02: {outcome:?}"
    );
}

#[test]
fn native_geki_match_matches_every_observable() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-02-geki");
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
        "sf-win-02: {outcome:?}"
    );
}

#[test]
fn tampered_shuriken_checkpoint_is_caught_at_its_frame() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const FRAME: u32 = 7534;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-02-geki-negative");
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
    let row = format!("{FRAME}\t030744cd\tpixels_{FRAME:06}.rgba");
    assert!(text.contains(&row), "unchanged native checkpoint row");
    let altered = text.replace(&row, &row.replace("030744cd", "030744cc"));
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
        panic!("tampered shuriken checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");
}

#[test]
fn native_geki_match_reaches_the_bonus_stage_boundary() {
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
fn country_end_dispatches_stage_zero_and_clears_the_match_counters() {
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
        assert!(
            machine.unrecovered().is_empty(),
            "frame {}",
            machine.state.frame
        );
    }
    assert_eq!(machine.state.frame, BOUNDARY);
    assert!(machine.task_positions().contains(&(1, 0x40dd4)));
    assert_eq!(
        (
            machine.work_ram()[0x4003],
            machine.work_ram()[0x4004],
            machine.work_ram()[0x4009],
            machine.work_ram()[0x400a],
        ),
        (0, 0, 0, 0),
    );
}

#[test]
fn reference_has_the_expected_ko_and_match_landmarks() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let read = |frame: u32| {
        std::fs::read(Path::new(RUN_A).join(format!("ram_{frame:06}_work.bin"))).unwrap()
    };
    assert_eq!(
        (read(7076)[0x6b8], read(7077)[0x6b8], read(7077)[0xe36]),
        (0, 1, 7)
    );
    assert_eq!(
        (read(8411)[0x6b8], read(8412)[0x6b8], read(8412)[0xe36]),
        (1, 2, 7)
    );
    assert_eq!(read(8908)[0xe36], 0x0b, "Geki's match-won screen");
    assert_eq!(read(9180)[0xe36], 6, "the match-end wipe");
    let w = read(LAST);
    assert_eq!((w[0x4003], w[0x4004], w[0x4009], w[0x400a]), (0, 1, 1, 1));
}
