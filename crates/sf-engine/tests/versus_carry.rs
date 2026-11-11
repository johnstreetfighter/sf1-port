//! Ticket #66: a versus winner continues against the CPU on the shared
//! fight machine, with per-frame RAM equality across the mode transition.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, fight_owned_addresses, observe_cpu_fight,
    run_fight_capture, scenario_input_events, versus_owned_addresses, CpuRamOutcome, ScratchDir,
    INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_versus_carry_01.json";
const RUN_A: &str = "../../tools/reference/local/captures/versus-carry-01-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/versus-carry-01-run-b";
const SCENARIO_02: &str = "../../tools/reference/scenarios/sf_versus_carry_02.json";
const RUN_02_A: &str = "../../tools/reference/local/captures/versus-carry-02-run-a";
const RUN_02_B: &str = "../../tools/reference/local/captures/versus-carry-02-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present() -> bool {
    [RUN_A, RUN_B, ROMS].iter().all(|p| Path::new(p).is_dir())
}

fn present_02() -> bool {
    [RUN_02_A, RUN_02_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

#[test]
fn sf_versus_carry_01_reference_runs_repeat_exactly() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).unwrap();
    assert_eq!(scenario.checkpoints, (2450..=6300).collect::<Vec<_>>());
    let scope = CompareScope {
        from: Some(2450),
        to: Some(6300),
        fields: None,
    };
    let outcome = compare(
        &scenario,
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(Path::new(RUN_B)).unwrap(),
        CandidateKind::Reference,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 3851,
                checkpoints_compared: 3851
            }
        ),
        "{outcome:?}"
    );
}

#[test]
fn shared_fight_machine_matches_versus_carry_over_and_the_following_cpu_round() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        rom.clone(),
        Some(Path::new(SCENARIO)),
        2450,
        6300,
    )
    .unwrap();
    assert_eq!(outcome, CpuRamOutcome::Match { frames: 3850 });

    let landmarks = [3973, 5215, 5397, 5496, 5497, 5498, 5650, 5819, 5930, 6300];
    let mut snapshots = BTreeMap::new();
    let (last, unrecovered) = observe_cpu_fight(
        Path::new(RUN_A),
        rom,
        Some(Path::new(SCENARIO)),
        2450,
        6300,
        |m| {
            if landmarks.contains(&m.state.frame) {
                snapshots.insert(m.state.frame, m.work_ram().to_vec());
            }
        },
    )
    .unwrap();
    assert_eq!((last, unrecovered), (6300, vec![]));
    let w = |frame| &snapshots[&frame][..];
    assert_eq!(
        (&w(3973)[0x6b6..0x6b8], &w(3973)[0x6e0..0x6e2]),
        (&[2, 11][..], &[4, 11][..])
    );
    assert_eq!(
        &w(3973)[0x6fc..0x704],
        &[0, 0, 0, 0, 0, 1, 0, 0],
        "Ken's one jab scores"
    );
    assert_eq!(
        (
            w(5215)[0x4009],
            w(5215)[0xe36],
            w(5215)[0x6b8],
            w(5215)[0x6e2]
        ),
        (1, 11, 2, 0)
    );
    assert_eq!(w(5397)[0xe36], 6, "the versus wipe runs");
    assert_eq!(
        (
            w(5496)[0x4002],
            w(5496)[0x4008],
            w(5496)[0x4009],
            w(5496)[0x400a]
        ),
        (2, 0, 0, 0)
    );
    assert_eq!(
        &w(5496)[0x6fc..0x704],
        &[0; 8],
        "Ken did not qualify; his score clears"
    );
    assert_eq!(&w(5496)[0xe5e..0xe62], &[0, 0xff, 0x86, 0xda]);
    assert_eq!(&w(5496)[0xe62..0xe66], &[0, 0xff, 0x86, 0x8e]);
    assert_eq!(
        (&w(5496)[0x634..0x638], &w(5496)[0x638..0x63a]),
        (&[0, 1, 0xb0, 0x68][..], &[0, 0][..])
    );
    assert_eq!(
        (&w(5497)[0x634..0x638], &w(5497)[0x638..0x63a]),
        (&[0, 1, 0xa4, 0x68][..], &[2, 0][..])
    );
    assert_eq!(
        &w(5498)[0x6ba..0x6c2],
        &[2, 0xbc, 2, 0x98, 0x0a, 0, 4, 0],
        "mode-2 P1 placement"
    );
    assert_eq!(
        &w(5498)[0x466e..0x4676],
        &[3, 0xbc, 2, 0xe0, 4, 0, 0x0a, 0],
        "mode-2 CPU placement"
    );
    assert_eq!(w(5650)[0xe36], 5, "Retsu's ROUND 1 announcement");
    assert_eq!(
        &w(5819)[0x4058..0x4068],
        &[0, 1, 0, 0, 1, 0, 2, 0, 1, 0, 0, 0, 0, 1, 0, 2]
    );
    assert_eq!(
        (w(5930)[0x4008], &w(5930)[0x6b6..0x6b8]),
        (0x80, &[4, 11][..]),
        "the CPU fight begins"
    );
    assert_eq!(
        &w(6300)[0x6b6..0x6b8],
        &[2, 10],
        "Retsu acts in the following CPU round"
    );

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-versus-carry-01-native");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-versus-carry-01",
            Some(Path::new(SCENARIO)),
            2450,
            6300
        )
        .unwrap(),
        3851,
    );
    let scope = CompareScope {
        from: Some(2451),
        to: Some(6300),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 3850,
                checkpoints_compared: 3850
            }
        ),
        "{outcome:?}"
    );
}

#[test]
fn qualifying_versus_loser_reports_name_entry_and_its_ranking_pointer() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut m = FightMachine::from_fight_checkpoint(Path::new(RUN_A), 2450, rom).unwrap();
    let events = scenario_input_events(Path::new(SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; INPUTS]);
    while m.state.frame < 5495 {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        assert!(m.unrecovered().is_empty());
    }
    m.set_work_byte(0xff86fc, 9);
    m.step();
    assert_eq!((m.state.frame, m.unrecovered()), (5496, vec![]));
    assert!(m.task_positions().contains(&(1, 0x42552)));
    assert_eq!(&m.work_ram()[0xe5e..0xe62], &[0, 0xff, 0x86, 0xda]);
    assert_eq!(&m.work_ram()[0xe66..0xe6a], &[0, 0xff, 0x8e, 0x6e]);
    assert_eq!(&m.work_ram()[0xe6a..0xe6e], &[0x31, 0x31, 0x31, 0x00]);
}

#[test]
fn union_only_ram_change_is_caught_at_the_changed_frame() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let versus: BTreeSet<u32> = versus_owned_addresses()
        .into_iter()
        .map(|(_, a)| a)
        .collect();
    let owned = fight_owned_addresses();
    let unified: BTreeSet<u32> = owned.iter().map(|(_, a)| *a).collect();
    assert!(versus.is_subset(&unified));
    assert!((0xff8e52..=0xff8f0d).all(|a| unified.contains(&a)));
    assert!(!versus.contains(&0xffc050) && unified.contains(&0xffc050));

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut m = FightMachine::from_fight_checkpoint(Path::new(RUN_A), 3972, rom).unwrap();
    let events = scenario_input_events(Path::new(SCENARIO)).unwrap();
    let mut held = [false; INPUTS];
    for &(frame, input, value) in &events {
        if frame < 3973 {
            held[input] = value;
        }
    }
    apply_inputs(&mut m, &held);
    m.step();
    assert_eq!(m.state.frame, 3973);
    assert!(m.unrecovered().is_empty());

    let mut reference = std::fs::read(Path::new(RUN_A).join("ram_003973_work.bin")).unwrap();
    let i = (0xffc050 - 0xff8000) as usize;
    assert_eq!(reference[i], m.work_ram()[i]);
    reference[i] ^= 1;
    let mismatch = owned.into_iter().find(|(_, addr)| {
        let i = (*addr - 0xff8000) as usize;
        let native = if *addr >= 0xffe000 {
            m.state.objectram[i - 0x6000]
        } else {
            m.work_ram()[i]
        };
        reference[i] != native
    });
    assert_eq!(mismatch, Some(("CPU reaction table pointers", 0xffc050)));
}

#[test]
fn sf_versus_carry_02_reference_runs_repeat_exactly() {
    if !present_02() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO_02).unwrap();
    assert_eq!(scenario.checkpoints, (2450..=8510).collect::<Vec<_>>());
    let scope = CompareScope {
        from: Some(2450),
        to: Some(8510),
        fields: None,
    };
    let outcome = compare(
        &scenario,
        &load_capture(Path::new(RUN_02_A)).unwrap(),
        &load_capture(Path::new(RUN_02_B)).unwrap(),
        CandidateKind::Reference,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 6061,
                checkpoints_compared: 6061
            }
        ),
        "{outcome:?}"
    );
}

#[test]
fn shared_fight_machine_matches_p2_versus_carry_over_and_mode_4_cpu_match() {
    if !present_02() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_02_A),
        rom.clone(),
        Some(Path::new(SCENARIO_02)),
        2450,
        8510,
    )
    .unwrap();
    assert_eq!(outcome, CpuRamOutcome::Match { frames: 6060 });

    let landmarks = [
        5222, 5404, 5503, 5504, 5505, 5657, 5826, 5937, 6005, 6018, 6045, 6310, 8079, 8380, 8479,
        8503, 8510,
    ];
    let mut snapshots = BTreeMap::new();
    let mut obj_5505 = Vec::new();
    let (last, unrecovered) = observe_cpu_fight(
        Path::new(RUN_02_A),
        rom,
        Some(Path::new(SCENARIO_02)),
        2450,
        8510,
        |m| {
            if landmarks.contains(&m.state.frame) {
                snapshots.insert(m.state.frame, m.work_ram().to_vec());
            }
            if m.state.frame == 5505 {
                obj_5505 = m.state.objectram.clone();
            }
        },
    )
    .unwrap();
    assert_eq!((last, unrecovered), (8510, vec![]));
    let w = |frame| &snapshots[&frame][..];

    // P2 Ken wins the versus match 2-0 with score 4,880 (< 5,000 10th-place threshold).
    assert_eq!(
        (
            w(5222)[0x4009],
            w(5222)[0xe36],
            w(5222)[0x6b8],
            w(5222)[0x6e2]
        ),
        (2, 11, 0, 2)
    );
    assert_eq!(&w(5222)[0x6fc..0x704], &[0, 0, 0, 4, 8, 8, 0, 0]);
    assert_eq!(w(5404)[0xe36], 6, "versus wipe runs");

    // 0x3eeac..0x3eebc: GAME_MODE = 4, P1 ranking check with scene byte 2 clears Ryu's score,
    // Ken's score is preserved, and 0x3e6ca requests the stage and country palettes.
    assert_eq!(
        (
            w(5503)[0x4002],
            w(5503)[0x4008],
            w(5503)[0x4009],
            w(5503)[0x400a]
        ),
        (4, 0, 0, 0)
    );
    assert_eq!(
        &w(5503)[0x6d2..0x6da],
        &[0; 8],
        "Ryu did not qualify; his score clears"
    );
    assert_eq!(
        &w(5503)[0x6fc..0x704],
        &[0, 0, 0, 4, 8, 8, 0, 0],
        "Ken's score carries into mode 4"
    );
    assert_eq!(&w(5503)[0xe5e..0xe62], &[0, 0xff, 0x86, 0xb0]);
    assert_eq!(&w(5503)[0xe62..0xe66], &[0, 0xff, 0x86, 0x8f]);
    assert_eq!(
        (&w(5503)[0x634..0x638], &w(5503)[0x638..0x63a]),
        (&[0, 1, 0xb0, 0x68][..], &[0, 0][..])
    );
    assert_eq!(
        (&w(5504)[0x634..0x638], &w(5504)[0x638..0x63a]),
        (&[0, 1, 0xa4, 0x68][..], &[2, 0][..])
    );

    // 0x3e674 mode-4 placement and 0x3fad6 mode-4 opponent intro (event 4).
    assert_eq!(
        &w(5505)[0x6e4..0x6ec],
        &[2, 0xbc, 2, 0xe0, 0x0a, 0, 4, 0],
        "mode-4 P2 placement"
    );
    assert_eq!(
        &w(5505)[0x466e..0x4676],
        &[3, 0xbc, 2, 0x98, 4, 0, 0x0a, 0],
        "mode-4 CPU placement"
    );
    assert_eq!(
        &obj_5505[0x438..0x440],
        &[0, 0xff, 0x86, 0xda, 0, 0xff, 0xc6, 0x64]
    );
    assert_eq!(w(5505)[0xe36], 4, "Retsu mode-4 intro");
    assert_eq!(
        w(5657)[0xe36],
        5,
        "mode-4 ROUND 1 announcement (0x3e714 / 0x3e896)"
    );
    assert_eq!(
        (&w(5826)[0x6e0..0x6e2], &w(5826)[0x466a..0x466c]),
        (&[4, 11][..], &[4, 11][..])
    );
    assert_eq!(
        &w(5826)[0x4058..0x4068],
        &[0, 1, 0, 0, 1, 0, 2, 0, 1, 0, 0, 0, 0, 1, 0, 2]
    );

    // Mode-4 fight loop (0x3ea88, 0x42ac0, camera (P2_STRUCT, CPU_STRUCT), P2 button latches,
    // step_random on P2_STRUCT + 2, and 0x446f6 / 0x4473a / 0x4479c CPU hits on P2 Ken).
    assert_eq!(w(5937)[0x4008], 0x80, "mode-4 CPU fight begins");
    assert_ne!(
        &w(6005)[0x6a0..0x6a5],
        &[0; 5],
        "P2 button latches active in mode 4"
    );
    assert_eq!(
        (
            &w(6018)[0x400c..0x400e],
            &w(6018)[0x466a..0x466c],
            &w(6018)[0x6fc..0x704]
        ),
        (&[0, 0xd6][..], &[2, 11][..], &[0, 0, 0, 4, 8, 9, 0, 0][..]),
        "camera tracks (P2_STRUCT, CPU_STRUCT) and Ken's LP scores on Retsu"
    );
    assert_eq!(
        &w(6045)[0x6e0..0x6e2],
        &[0, 9],
        "CPU Retsu hits P2 Ken via 0x4473a alive reaction table"
    );
    assert_eq!(
        &w(6310)[0x6e0..0x6e2],
        &[0, 0],
        "CPU Retsu KOs P2 Ken via 0x4479c KO reaction table"
    );

    // 0x3ef22 -> 0x3ef2c mode-4 CPU match win, 0x40140, wipe, 0x3ef40..0x3ef50 P2 ranking check,
    // and 0x3f146 continue acceptance on P2's start2 bit.
    assert_eq!(
        (w(8079)[0x4009], w(8079)[0xe36], w(8079)[0x466c]),
        (4, 11, 2),
        "mode-4 CPU match win decision"
    );
    assert_eq!(w(8380)[0xe36], 6, "mode-4 match wipe");
    assert_eq!(
        (w(8479)[0x4002], w(8479)[0x4008], w(8479)[0x65d]),
        (4, 8, 1),
        "mode-4 continue screen entered with 1 credit"
    );
    assert_eq!(&w(8479)[0xe5e..0xe62], &[0, 0xff, 0x86, 0xda]);
    assert_eq!(&w(8479)[0xe62..0xe66], &[0, 0xff, 0x86, 0x8e]);
    assert_eq!(
        &w(8479)[0x6fc..0x704],
        &[0; 8],
        "Ken's non-qualifying score clears at 0x3ef4a"
    );
    assert_eq!(
        (w(8503)[0x4002], w(8503)[0xe36], w(8503)[0x65d]),
        (4, 4, 0),
        "start2 at 0x3f146 consumes credit and restarts Retsu in mode 4"
    );

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-versus-carry-02-native");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_02_A),
            &assets,
            &out,
            "sf-versus-carry-02",
            Some(Path::new(SCENARIO_02)),
            2450,
            8510
        )
        .unwrap(),
        6061,
    );
    let scope = CompareScope {
        from: Some(2451),
        to: Some(8510),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(SCENARIO_02).unwrap(),
        &load_capture(Path::new(RUN_02_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 6060,
                checkpoints_compared: 6060
            }
        ),
        "{outcome:?}"
    );
}

#[test]
fn mode_4_p2_match_win_runs_ken_win_quote_and_advances_to_geki() {
    if !present_02() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut m = FightMachine::from_fight_checkpoint(Path::new(RUN_02_A), 5937, rom).unwrap();
    let mut saw_match_win = false;
    let mut saw_geki_intro = false;
    for _ in 0..3500 {
        let mut held = [false; INPUTS];
        let w = m.work_ram();
        if w[0x4008] & 0x80 != 0 {
            // Walk right and fire a heavy Shoryuken (F, D, DF + HP) every 60 frames.
            let phase = (m.state.frame - 5937) % 60;
            if (0..20).contains(&phase) || (22..25).contains(&phase) || (28..36).contains(&phase) {
                held[10] = true; // p2_right (towards CPU on the right)
            }
            if (25..36).contains(&phase) {
                held[12] = true; // p2_down
            }
            if (29..33).contains(&phase) {
                held[16] = true; // p2_hp
            }
        }
        apply_inputs(&mut m, &held);
        m.step();
        assert!(
            m.unrecovered().is_empty(),
            "unrecovered at {}: {:x?}",
            m.state.frame,
            m.unrecovered()
        );
        let w = m.work_ram();
        if w[0x4009] == 2 && w[0xe36] == 11 {
            saw_match_win = true;
        }
        if saw_match_win && w[0x4002] == 4 && w[0x4004] == 1 && w[0xe36] == 4 {
            saw_geki_intro = true;
            break;
        }
    }
    assert!(
        saw_match_win,
        "mode-4 P2 match win (0x3ef54, MATCH_WINNER = 2, event 0xb) reached"
    );
    assert!(
        saw_geki_intro,
        "mode-4 advances to Geki (FIGHT_NO = 1, event 4 at 0x3e6ca)"
    );
}
