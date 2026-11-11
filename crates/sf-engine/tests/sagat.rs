//! Ticket #135 (`F63`): Ryu beats stage 4's second opponent (`0x181ec`, Sagat),
//! including his six hit functions (`0x47eaa..0x4813d`), his Tiger Shot
//! projectile spawn helper (`0x45ace..0x45aee` -> template `0x1873c`), and his
//! Tiger Shot collision handler (`0x43652..0x436b6`: explosion `0x186fc`,
//! blocked chip damage `0x436b0`, and unblocked 30-damage hit `0x436a8` with
//! slow motion event 9), through Sagat's match-won card (`0x159b2`, voice
//! `0xba`) and `0x3ef96`'s dispatch of `STAGE_NO == 4` to the ending (`0x44a40`).

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{
    apply_inputs, compare_fight_ram_with_inputs, run_fight_capture, scenario_input_events,
    CpuRamOutcome, ScratchDir, INPUTS,
};

const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_10.json";
const RUN_A: &str = "../../tools/reference/local/captures/win-10-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/win-10-run-b";
const WIN_09_A: &str = "../../tools/reference/local/captures/win-09-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIRST: u32 = 37153;
const LAST: u32 = 39775;
const SCENARIO_LAST: u32 = 43618;
const FRAMES: u32 = LAST - FIRST;
const SCENARIO_FRAMES: u32 = SCENARIO_LAST - FIRST;

fn present() -> bool {
    [RUN_A, RUN_B, WIN_09_A, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn ref_scope() -> CompareScope {
    CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(SCENARIO_LAST as u64),
        fields: None,
    }
}

fn native_fight_scope() -> CompareScope {
    CompareScope {
        from: Some((FIRST + 1) as u64),
        to: Some(LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

#[test]
fn reference_runs_repeat_and_equal_win_09_through_divergence() {
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
        &ref_scope(),
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared,
                checkpoints_compared,
            } if frames_compared == SCENARIO_FRAMES as u64
                && checkpoints_compared == SCENARIO_FRAMES as usize
        ),
        "sf-win-10 repeat: {outcome:?}"
    );

    let mut win_09_a = load_capture(Path::new(WIN_09_A)).unwrap();
    win_09_a.scenario_id = Some(scenario.scenario_id.clone());
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(37154),
        fields: None,
    };
    let prefix_outcome = compare(
        &scenario,
        &win_09_a,
        &cap_a,
        CandidateKind::Reference,
        &prefix_scope,
    )
    .unwrap();
    assert!(
        matches!(
            prefix_outcome,
            Outcome::Match {
                frames_compared: 37155,
                checkpoints_compared: 2,
            }
        ),
        "sf-win-10 prefix vs sf-win-09: {prefix_outcome:?}"
    );
}

#[test]
fn native_sagat_match_matches_every_observable_and_owned_ram() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-10-sagat");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-10",
            Some(Path::new(SCENARIO)),
            FIRST,
            LAST,
        )
        .unwrap(),
        (FRAMES + 1) as u64,
    );
    let outcome = compare(
        &load_scenario(SCENARIO).unwrap(),
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &native_fight_scope(),
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
        "sf-win-10 native observables: {outcome:?}"
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
        "sf-win-10 owned RAM over {FIRST}..={LAST}"
    );
}

#[test]
fn tampered_sagat_ko_checkpoint_and_missing_0x45ace_0x43652_and_0x47ff4_are_caught() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    const KO_FRAME: u32 = 37645;
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let out = ScratchDir::new("sf-engine-win-10-sagat-negative");
    assert_eq!(
        run_fight_capture(
            Path::new(RUN_A),
            &assets,
            &out,
            "sf-win-10",
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
        panic!("tampered Sagat KO checkpoint unexpectedly matched");
    };
    assert_eq!(mismatch.frame, KO_FRAME as u64, "{mismatch:?}");
    assert_eq!(mismatch.observable, "checkpoint screen pixels");

    // Negative control 2: patching Sagat's Tiger Shot stream call at 0x144fc
    // from 0x45ace to untranslated 0x492d6 stops native execution at frame 37170.
    let mut bad_spawn_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    assert_eq!(
        u32::from_be_bytes(bad_spawn_rom[0x144fc..0x14500].try_into().unwrap()),
        0x0004_5ace
    );
    bad_spawn_rom[0x144fc..0x14500].copy_from_slice(&0x0004_92d6u32.to_be_bytes());
    let spawn_stop = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        bad_spawn_rom,
        Some(Path::new(SCENARIO)),
        FIRST,
        37175,
    )
    .unwrap();
    assert_eq!(
        spawn_stop,
        CpuRamOutcome::Stopped {
            frames: 37170 - FIRST - 1,
            frame: 37170,
            unrecovered: vec![0x492d6],
        }
    );

    // Negative control 3: patching Sagat's Tiger Shot template 0x1873c (+0x14 = 0x18750)
    // from 0x43652 to untranslated 0x492d6 stops FightMachine at frame 37187 when the
    // first Tiger Shot hits Ryu.
    let mut bad_hit_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    assert_eq!(
        u32::from_be_bytes(bad_hit_rom[0x18750..0x18754].try_into().unwrap()),
        0x0004_3652
    );
    bad_hit_rom[0x18750..0x18754].copy_from_slice(&0x0004_92d6u32.to_be_bytes());
    let mut hit_machine =
        FightMachine::from_fight_checkpoint(Path::new(RUN_A), FIRST, bad_hit_rom).unwrap();
    let events = scenario_input_events(Path::new(SCENARIO)).unwrap();
    let (mut next, mut held) = (0usize, [false; INPUTS]);
    while hit_machine.state.frame < 37190 && hit_machine.unrecovered().is_empty() {
        let frame = hit_machine.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut hit_machine, &held);
        hit_machine.step();
    }
    assert_eq!(hit_machine.state.frame, 37187);
    assert_eq!(hit_machine.unrecovered().to_vec(), vec![0x492d6]);

    // Negative control 4: patching Sagat's 0x47ff4 hit function pointers to
    // untranslated 0x492d6 stops FightMachine at frame 38437 when 0x47ff4 lands.
    let mut bad_hf_rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    for off in [0x14072usize, 0x14092, 0x143e6, 0x14406, 0x14426] {
        assert_eq!(
            u32::from_be_bytes(bad_hf_rom[off..off + 4].try_into().unwrap()),
            0x0004_7ff4
        );
        bad_hf_rom[off..off + 4].copy_from_slice(&0x0004_92d6u32.to_be_bytes());
    }
    let mut hf_machine =
        FightMachine::from_fight_checkpoint(Path::new(RUN_A), FIRST, bad_hf_rom).unwrap();
    let (mut next, mut held) = (0usize, [false; INPUTS]);
    while hf_machine.state.frame < 38440 && hf_machine.unrecovered().is_empty() {
        let frame = hf_machine.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut hf_machine, &held);
        hf_machine.step();
    }
    assert_eq!(hf_machine.state.frame, 38437);
    assert_eq!(hf_machine.unrecovered().to_vec(), vec![0x492d6]);
}

#[test]
fn direct_replay_exercises_sagat_tiger_shot_chip_and_slow_motion_and_hit_functions() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let mut machine = FightMachine::from_fight_checkpoint(Path::new(RUN_A), FIRST, rom).unwrap();
    let events = scenario_input_events(Path::new(SCENARIO)).unwrap();
    let (mut next, mut held) = (0usize, [false; INPUTS]);
    let mut prev_p1_health = 48u8;
    let mut prev_round = 0u8;
    let mut prev_slow = 0u8;
    let mut prev_slot0_stream = 0u32;
    let mut tiger_spawns = Vec::new();
    let mut tiger_explosions = Vec::new();
    let mut slow_transitions = Vec::new();
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
        let rd = w[0x400a];
        if rd != prev_round {
            prev_round = rd;
            prev_p1_health = 48;
        }
        let slow = w[0x400b];
        if slow != prev_slow {
            slow_transitions.push((frame, prev_slow, slow));
            prev_slow = slow;
        }
        let s0_st = machine.state.objectram[0x108];
        let s0_stream = if s0_st != 0 {
            u32::from_be_bytes(machine.state.objectram[0x110..0x114].try_into().unwrap())
        } else {
            0
        };
        if s0_stream != prev_slot0_stream {
            if s0_stream == 0x190b6 {
                tiger_spawns.push(frame);
            } else if s0_stream == 0x1904a && s0_st == 1 {
                let p1_h = w[0x6b7] * 4 + w[0x6b6];
                tiger_explosions.push((frame, prev_p1_health, p1_h));
            }
            prev_slot0_stream = s0_stream;
        }
        let p1_h = w[0x6b7] * 4 + w[0x6b6];
        if p1_h < prev_p1_health && (w[0x4008] & 0x80) != 0 {
            if !(s0_st == 1 && s0_stream == 0x1904a) {
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
    }

    assert_eq!(tiger_spawns, vec![37170, 37274, 37462, 38696]);
    assert_eq!(tiger_explosions, vec![(37187, 48, 33), (37292, 33, 3)]);
    assert_eq!(
        slow_transitions,
        vec![(37292, 0x00, 0xff), (37382, 0xff, 0x00)]
    );
    assert_eq!(
        landed_hits,
        vec![
            (38437, 0x47ff4, 48, 36),
            (38500, 0x480d0, 36, 23),
            (38640, 0x47f86, 23, 13),
        ]
    );
    assert_eq!(machine.state.frame, LAST);

    // Step one more frame to 39776: 0x3ef96 dispatches stage 4 to 0x44a40 (the ending,
    // ticket #136, F64), sleeping into 0x44a60.
    apply_inputs(&mut machine, &[false; INPUTS]);
    machine.step();
    assert_eq!(machine.state.frame, 39776);
    assert!(
        machine.unrecovered().is_empty(),
        "{:?}",
        machine.unrecovered()
    );
    assert!(
        machine.task_positions().contains(&(1, 0x44a60)),
        "{:?}",
        machine.task_positions()
    );
    let w = machine.work_ram();
    assert_eq!((w[0x4003], w[0x4004], w[0x4009], w[0x400a]), (4, 0, 0, 0));
}

#[test]
fn reference_has_the_expected_ko_match_and_ending_landmarks() {
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
    assert_eq!(slot1_resume(&read(37153)), 0x3ea3e);
    assert_eq!(slot1_resume(&read(37154)), 0x3eafa);
    let w37187 = read(37187);
    assert_eq!(
        w37187[0x6b7] * 4 + w37187[0x6b6],
        33,
        "blocked Tiger Shot chip damage at 37187 (48 -> 33)"
    );
    let w37292 = read(37292);
    assert_eq!(
        (w37292[0x6b7] * 4 + w37292[0x6b6], w37292[0x400b]),
        (3, 0xff),
        "unblocked Tiger Shot hit + slow motion at 37292 (33 -> 3)"
    );
    assert_eq!(
        (read(37644)[0x6b8], read(37645)[0x6b8], read(37645)[0xe36]),
        (0, 1, 7),
        "Ryu wins round 1 at 37645"
    );
    assert_eq!(
        (read(39008)[0x6b8], read(39009)[0x6b8], read(39009)[0xe36]),
        (1, 2, 7),
        "Ryu wins round 2 at 39009"
    );
    assert_eq!(
        read(39406)[0xe36],
        0x0b,
        "Sagat's match-won screen (0x159b2, voice 0xba) at 39406"
    );
    assert_eq!(read(39677)[0xe36], 6, "the match-end wipe at 39677");
    assert_eq!(
        slot1_resume(&read(39776)),
        0x44a60,
        "ending entry 0x44a40 sleeps into 0x44a60 at 39776"
    );
    assert_eq!(
        (
            read(43616)[0x4003],
            read(43617)[0x4003],
            slot1_resume(&read(43617))
        ),
        (4, 5, 0x42552),
        "ending finishes at 43617 via 0x3efc4 (stage 4 -> 5) into 0x3f2d0 / 0x42422 (0x42552)"
    );
}
