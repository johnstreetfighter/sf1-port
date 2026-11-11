//! Ticket #134 (`F62`): Ryu beats stage 4's first opponent (`0x17f7c`, Adon),
//! including his six hit functions (`0x47c16..0x47ea8`) and mid-combo turn
//! helper (`0x4558a..0x455d8`), and reaches Sagat's (`0x181ec`) setup and
//! round-1 fight loop.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, scenario_input_events,
    CpuRamOutcome, ScratchDir, INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_09.json";
const RUN_A: &str = "../../tools/reference/local/captures/win-09-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/win-09-run-b";
const WIN_08_A: &str = "../../tools/reference/local/captures/win-08-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 34183;
const LAST: u32 = 37154;
const FRAMES: u32 = LAST - FIRST;

fn present() -> bool {
    [RUN_A, RUN_B, WIN_08_A, ROMS]
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
fn reference_runs_repeat_and_equal_win_08_through_divergence() {
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
        "sf-win-09 repeat: {outcome:?}"
    );

    let mut win_08_a = load_capture(Path::new(WIN_08_A)).unwrap();
    win_08_a.scenario_id = Some(scenario.scenario_id.clone());
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(34184),
        fields: None,
    };
    let prefix_outcome = compare(
        &scenario,
        &win_08_a,
        &cap_a,
        CandidateKind::Reference,
        &prefix_scope,
    )
    .unwrap();
    assert!(
        matches!(
            prefix_outcome,
            Outcome::Match {
                frames_compared: 34185,
                checkpoints_compared: 2,
            }
        ),
        "sf-win-09 prefix vs sf-win-08: {prefix_outcome:?}"
    );
}

#[test]
fn native_adon_match_matches_every_observable_and_owned_ram() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-09-adon");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-09",
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
        "sf-win-09 native observables: {outcome:?}"
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
        "sf-win-09 owned RAM over {FIRST}..={LAST}"
    );
}

#[test]
fn tampered_adon_ko_checkpoint_and_missing_0x47cf2_and_0x4558a_are_caught() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const KO_FRAME: u32 = 34518;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-09-adon-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-09",
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
        panic!("tampered Adon KO checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, KO_FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Negative control 2: patching Adon's active frame 0x13b90 (+0x1c = 0x13bac)
    // from 0x47cf2 to untranslated 0x492d6 stops native execution at frame 34296
    // when Adon's 0x47cf2 attack lands.
    let mut bad_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    assert_eq!(
        u32::from_be_bytes(bad_rom[0x13bac..0x13bb0].try_into().unwrap()),
        0x0004_7cf2
    );
    bad_rom[0x13bac..0x13bb0].copy_from_slice(&0x0004_92d6u32.to_be_bytes());
    let stop_outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        bad_rom,
        Some(Path::new(SCENARIO)),
        FIRST,
        34300,
    )
    .unwrap();
    assert_eq!(
        stop_outcome,
        CpuRamOutcome::Stopped {
            frames: 34296 - FIRST - 1,
            frame: 34296,
            unrecovered: vec![0x492d6],
        }
    );

    // Negative control 3: patching Adon's stream call 0x139ec (+2 = 0x139ee)
    // from 0x4558a to untranslated 0x492d6 stops native execution at frame 34239
    // when Adon's 0x4558a turn helper runs.
    let mut bad_turn_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    assert_eq!(
        u32::from_be_bytes(bad_turn_rom[0x139ee..0x139f2].try_into().unwrap()),
        0x0004_558a
    );
    bad_turn_rom[0x139ee..0x139f2].copy_from_slice(&0x0004_92d6u32.to_be_bytes());
    let turn_stop = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        bad_turn_rom,
        Some(Path::new(SCENARIO)),
        FIRST,
        34245,
    )
    .unwrap();
    assert_eq!(
        turn_stop,
        CpuRamOutcome::Stopped {
            frames: 34239 - FIRST - 1,
            frame: 34239,
            unrecovered: vec![0x492d6],
        }
    );
}

#[test]
fn direct_replay_exercises_adon_hit_functions_0x4558a_turns_and_reaches_sagats_setup() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut machine =
        FightMachine::from_fight_checkpoint(Path::new(RUN_A), FIRST, rom.clone()).unwrap();
    let events = scenario_input_events(Path::new(SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; INPUTS]);
    let mut prev_p1_health = 48u8;
    let mut prev_cpu_face = 4u8;
    let mut prev_cpu_stream = 0u32;
    let mut landed_hits = Vec::new();
    let mut turn_events = Vec::new();

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
        let cpu_ent = u16::from_be_bytes([w[0x4674], w[0x4675]]) as usize;
        let cpu_stream = u32::from_be_bytes(
            machine.state.objectram[cpu_ent + 0x2c..cpu_ent + 0x30]
                .try_into()
                .unwrap(),
        );
        let cpu_face = machine.state.objectram[cpu_ent + 0x37];
        if cpu_stream != prev_cpu_stream
            && matches!(cpu_stream, 0x138dc | 0x13a12 | 0x13a9e | 0x13b4a)
        {
            turn_events.push((frame, cpu_stream, prev_cpu_face, cpu_face));
        }
        prev_cpu_face = cpu_face;
        prev_cpu_stream = cpu_stream;

        let p1_h = w[0x6b7] * 4 + w[0x6b6];
        if p1_h < prev_p1_health && w[0x4008] == 0x80 {
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
        turn_events,
        vec![
            (34239, 0x13a12, 4, 2),
            (34259, 0x13a9e, 2, 2),
            (34281, 0x13b4a, 2, 4),
        ]
    );
    assert_eq!(
        landed_hits,
        vec![
            (34296, 0x47cf2, 48, 34),
            (35425, 0x47c84, 48, 34),
            (35486, 0x47c16, 34, 24),
            (35545, 0x47dce, 24, 9),
        ]
    );
    assert_eq!(machine.state.frame, LAST);
    assert!(machine.task_positions().contains(&(1, 0x3eafa)));
    let w = machine.work_ram();
    assert_eq!(
        u32::from_be_bytes(w[0x4676..0x467a].try_into().unwrap()),
        0x181ec
    );
    assert_eq!((w[0x4003], w[0x4004], w[0x4009], w[0x400a]), (4, 1, 0, 0));

    // Continue into Sagat's fight to frame 37206, where his first attack (0x47ff4) lands (48 -> 34).
    while machine.state.frame < 37206 {
        apply_inputs(&mut machine, &[false; INPUTS]);
        machine.step();
    }
    assert_eq!(machine.state.frame, 37206);
    assert!(machine.unrecovered().is_empty());
    let w = machine.work_ram();
    assert_eq!(w[0x6b7] * 4 + w[0x6b6], 34);

    // Also verify the two unexercised branches of 0x4558a..0x455d8 on checkpoint 34238
    // (one frame before 0x139ec invokes 0x4558a):
    // 1. 0x455a2..0x455a8: my_cx > ot_cx with +0x37 == 4 (already facing left -> no turn).
    let mut m_gt_same =
        FightMachine::from_fight_checkpoint(Path::new(RUN_A), 34238, rom.clone()).unwrap();
    let cpu_ent =
        u16::from_be_bytes([m_gt_same.work_ram()[0x4674], m_gt_same.work_ram()[0x4675]]) as usize;
    let p1_ent =
        u16::from_be_bytes([m_gt_same.work_ram()[0x6c0], m_gt_same.work_ram()[0x6c1]]) as usize;
    // Place Adon to the right of Ryu while Adon already has +0x37 == 4.
    m_gt_same.state.objectram[cpu_ent + 6..cpu_ent + 8].copy_from_slice(&0x0180u16.to_be_bytes());
    m_gt_same.state.objectram[p1_ent + 6..p1_ent + 8].copy_from_slice(&0x0080u16.to_be_bytes());
    assert_eq!(m_gt_same.state.objectram[cpu_ent + 0x37], 4);
    m_gt_same.step();
    assert_eq!(m_gt_same.state.objectram[cpu_ent + 0x37], 4);
    assert_eq!(
        u32::from_be_bytes(
            m_gt_same.state.objectram[cpu_ent + 0x2c..cpu_ent + 0x30]
                .try_into()
                .unwrap()
        ),
        0x13a12
    );

    // 2. 0x4559e (beq 0x455d2): my_cx == ot_cx (leaves facing unchanged).
    let mut m_eq = FightMachine::from_fight_checkpoint(Path::new(RUN_A), 34238, rom).unwrap();
    // Copy Ryu's x and cols/extent so center_x(me) == center_x(other) after Adon's step_xy.
    // On frame 34239 before 0x4558a runs, Adon's step_xy adds dx = -5 (0xfb) to his x,
    // and Adon's half-width is ((eb(cpu, 0x0d) + 1) << 3) = ((2 + 1) << 3) = 24, while
    // Ryu's half-width is ((eb(p1, 0x0d) + 1) << 3) = ((2 + 1) << 3) = 24.
    let p1_x = u16::from_be_bytes(
        m_eq.state.objectram[p1_ent + 6..p1_ent + 8]
            .try_into()
            .unwrap(),
    );
    let adon_dx = m_eq.state.objectram[cpu_ent + 0x10] as i8 as i16;
    let target_x = (p1_x as i16 - adon_dx) as u16;
    m_eq.state.objectram[cpu_ent + 6..cpu_ent + 8].copy_from_slice(&target_x.to_be_bytes());
    m_eq.state.objectram[cpu_ent + 0x0d] = m_eq.state.objectram[p1_ent + 0x0d];
    m_eq.state.objectram[cpu_ent + 0x37] = 4;
    m_eq.step();
    assert_eq!(m_eq.state.objectram[cpu_ent + 0x37], 4);
    assert_eq!(
        u32::from_be_bytes(
            m_eq.state.objectram[cpu_ent + 0x2c..cpu_ent + 0x30]
                .try_into()
                .unwrap()
        ),
        0x13a12
    );
}

#[test]
fn reference_has_the_expected_ko_match_and_sagat_setup_landmarks() {
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
    assert_eq!(slot1_resume(&read(34183)), 0x3ea3e);
    assert_eq!(slot1_resume(&read(34184)), 0x3eafa);
    assert_eq!(
        (read(34517)[0x6b8], read(34518)[0x6b8], read(34518)[0xe36]),
        (0, 1, 7)
    );
    assert_eq!(read(35020)[0xe36], 6, "round-end wipe at 35020");
    assert_eq!(read(35119)[0xe36], 5, "Adon ROUND 2 announce at 35119");
    assert_eq!(
        (read(35966)[0x6b8], read(35967)[0x6b8], read(35967)[0xe36]),
        (1, 2, 7)
    );
    assert_eq!(read(36352)[0xe36], 0x0b, "Adon's match-won screen");
    assert_eq!(read(36623)[0xe36], 6, "the match-end wipe");
    assert_eq!((read(36722)[0xe36], read(36722)[0x4004]), (4, 1));
    assert_eq!(read(36874)[0xe36], 5, "Sagat's ROUND 1 event");
    let setup = read(37043);
    assert_eq!(
        u32::from_be_bytes(setup[0x4676..0x467a].try_into().unwrap()),
        0x181ec
    );
    assert_eq!(slot1_resume(&read(37153)), 0x3ea3e);
    let first_loop = read(37154);
    assert_eq!(slot1_resume(&first_loop), 0x3eafa);
    assert_eq!(first_loop[0x4008], 0x80);
}
