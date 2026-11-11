//! Acceptance test for ticket #47 (F7): Ken's attacks against an idle Ryu in
//! the versus fixture. Five versus scenarios (`sf-ken-01..05`, from the
//! checkpoint-2450 fixture `move-run-a`) exercise Ken landing and whiffing
//! every standing normal, crouching, and jump attack, the Shoryuken, the
//! Tatsumaki Senpukyaku (including chip damage on a blocking Ryu), and the
//! Hadoken (`docs/reference/ken-attacks.md`).
//!
//! Verifies:
//! - `C-REF`: two independent MAME reference captures per scenario repeat
//!   exactly on every frame from `2450` to the scenario's end.
//! - `C-NAT`: native replay from the `2450` fixture matches the reference on
//!   every observable (screen hashes, full-pixel checkpoints, video/palette
//!   RAM, sound latch) and owned work/object RAM (`compare_versus_ram`), with
//!   no untranslated branches reached.
//! - Twin coverage: all 16 twin hit functions in `fighter.rs:144-185` land at
//!   least once, `0x453b6` with tables `0x4545c` / `0x45474` and the spin-loop
//!   trackers `0x4554a` / `0x45566` (`0xff870a` / `0xff870c`) are exercised.
//! - `C-NEG`: shifting one of Ken's inputs by one frame is caught.
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

use std::collections::BTreeSet;
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{apply_inputs, scenario_input_events, ScratchDir, INPUTS};
use sf_engine::versus::VersusMachine;

const REF_MOVE: &str = "../../tools/reference/local/captures/move-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// `(scenario number, last frame)`.
const SCENARIOS: [(u32, u32); 5] = [(1, 3300), (2, 3550), (3, 3910), (4, 3120), (5, 3100)];

/// All 16 Ken twin hit functions in `crates/sf-engine/src/fighter.rs:144-185`.
const KEN_TWIN_HIT_FUNCTIONS: [u32; 16] = [
    0x45afa, // standing LP (0d)
    0x45c16, // standing MP (0f)
    0x45ca2, // standing HP (10)
    0x46034, // standing LK (14)
    0x460d8, // standing MK (15)
    0x461a4, // standing HK frame 1 (16)
    0x461c4, // standing HK frame 2 (16)
    0x45d44, // crouching punches (11..13, 1c..1e)
    0x4625a, // crouching L/M kick (17)
    0x4630a, // crouching HK sweep (18)
    0x45e2e, // jump punches (20..22)
    0x46398, // neutral jump kicks frame 1 (23, 26)
    0x463b8, // neutral jump LK/MK frame 2 (23)
    0x463d8, // diagonal jump kicks (24, 25)
    0x45f04, // Shoryuken (19)
    0x45f64, // Tatsumaki Senpukyaku (1b)
];

fn paths(n: u32) -> (String, String, String) {
    (
        format!("../../tools/reference/scenarios/sf_ken_{n:02}.json"),
        format!("../../tools/reference/local/captures/ken-{n:02}-run-a"),
        format!("../../tools/reference/local/captures/ken-{n:02}-run-b"),
    )
}

fn prerequisites_present() -> bool {
    Path::new(ROMS).is_dir()
        && Path::new(REF_MOVE).is_dir()
        && SCENARIOS.iter().all(|&(n, _)| {
            let (_, a, b) = paths(n);
            Path::new(&a).is_dir() && Path::new(&b).is_dir()
        })
}

fn scope(last: u32) -> CompareScope {
    CompareScope {
        from: Some(2450),
        to: Some(last as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

fn assert_observation_dirs_identical(dir_a: &Path, dir_b: &Path) {
    let obs_files = |dir: &Path| -> BTreeSet<String> {
        std::fs::read_dir(dir)
            .expect("read capture dir")
            .filter_map(|e| {
                let name = e.ok()?.file_name().into_string().ok()?;
                let is_obs =
                    name.ends_with(".tsv") || name.ends_with(".bin") || name.ends_with(".rgba");
                is_obs.then_some(name)
            })
            .collect()
    };
    let files_a = obs_files(dir_a);
    let files_b = obs_files(dir_b);
    assert_eq!(files_a, files_b, "observation file sets must match");
    assert!(!files_a.is_empty(), "capture must have observation files");
    for name in &files_a {
        let bytes_a = std::fs::read(dir_a.join(name)).expect("read run-a file");
        let bytes_b = std::fs::read(dir_b.join(name)).expect("read run-b file");
        assert_eq!(
            bytes_a, bytes_b,
            "reference runs differed in {name} ({:?} vs {:?})",
            dir_a, dir_b
        );
    }
}

#[test]
fn reference_runs_repeat_exactly() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    for (n, last) in SCENARIOS {
        let (scenario, a, b) = paths(n);
        assert_observation_dirs_identical(Path::new(&a), Path::new(&b));
        let scenario = load_scenario(&scenario).unwrap();
        let outcome = compare(
            &scenario,
            &load_capture(&a).unwrap(),
            &load_capture(&b).unwrap(),
            CandidateKind::Reference,
            &scope(last),
        )
        .expect("compatible captures");
        assert!(
            matches!(outcome, Outcome::Match { .. }),
            "sf-ken-{n:02}: {outcome:?}"
        );
    }
}

#[test]
fn native_ken_attacks_match_the_reference_on_every_observable() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    for (n, last) in SCENARIOS {
        let (scenario_path, a, _) = paths(n);
        let out = ScratchDir::new(&format!("sf-engine-ken-{n:02}"));
        let id = format!("sf-ken-{n:02}");
        sf_engine::run::run_versus_interval_capture(
            Path::new(REF_MOVE),
            &assets,
            &out,
            &id,
            Path::new(&scenario_path),
            last,
        )
        .expect("native capture runs on translated paths only");
        let scenario = load_scenario(&scenario_path).unwrap();
        let outcome = compare(
            &scenario,
            &load_capture(&a).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &scope(last),
        )
        .expect("compatible captures");
        let frames = last - 2450 + 1;
        let Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } = outcome
        else {
            panic!("{id} must match the reference, got {outcome:?}");
        };
        assert_eq!(
            (frames_compared, checkpoints_compared),
            (frames as u64, frames as usize),
            "{id}"
        );
    }
}

#[test]
fn owned_ram_is_identical_every_frame() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    for (n, last) in SCENARIOS {
        let (scenario, a, _) = paths(n);
        let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
        let result = sf_engine::run::compare_versus_ram(
            Path::new(&a),
            rom.clone(),
            Path::new(&scenario),
            last,
        )
        .expect("reference dumps readable");
        assert_eq!(result, Ok(last - 2450), "sf-ken-{n:02}");

        // Verify that across the entire 0xff8000..=0xffffff work/object RAM dump,
        // any difference outside the owned regions is strictly confined to the task
        // slots' saved SR/PC/USP words (0xff8006..=0xff8105, offset % 16 >= 2) and
        // task stack memory (0xff8106..=0xff8631).
        let mut m = VersusMachine::from_checkpoint_2450_with_program(Path::new(&a), rom)
            .expect("checkpoint 2450");
        let events = scenario_input_events(Path::new(&scenario)).expect("scenario events");
        let (mut next, mut held) = (0usize, [false; INPUTS]);
        while m.state.frame < last {
            let frame = m.state.frame + 1;
            while next < events.len() && events[next].0 < frame {
                held[events[next].1] = events[next].2;
                next += 1;
            }
            apply_inputs(&mut m, &held);
            m.step();
            let ref_work = std::fs::read(Path::new(&a).join(format!("ram_{frame:06}_work.bin")))
                .expect("ref work dump");
            let native_work = m.work_ram();
            for (i, &ref_b) in ref_work.iter().enumerate() {
                let addr = 0xff8000 + i as u32;
                let native_b = if addr >= 0xffe000 {
                    m.state.objectram[i - 0x6000]
                } else {
                    native_work[i]
                };
                if ref_b != native_b {
                    let in_slot_context =
                        (0xff8006..0xff8106).contains(&addr) && ((addr - 0xff8006) % 16 >= 2);
                    let in_stack = (0xff8106..=0xff8631).contains(&addr);
                    assert!(
                        in_slot_context || in_stack,
                        "sf-ken-{n:02} frame {frame}: unexpected RAM diff at 0x{addr:06x} (ref {ref_b:02x} vs native {native_b:02x})"
                    );
                }
            }
        }
    }
}

#[test]
fn all_sixteen_twin_hit_functions_and_ken_helpers_are_exercised() {
    if !Path::new(REF_MOVE).is_dir() || !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local checkpoint-2450 fixture/ROMs not present");
        return;
    }

    let mut landed_hfs = BTreeSet::new();
    let mut whiffed_moves = BTreeSet::new();
    let mut hit_moves = BTreeSet::new();
    let mut jump_transitions = BTreeSet::new();
    let mut tatsu_counts = BTreeSet::new();
    let mut tatsu_starts = BTreeSet::new();
    let mut command_trackers_completed = [false; 3];
    let mut saw_tatsu_block_chip = false;
    let mut saw_hadoken_unblocked_slowmo = false;
    let mut saw_hadoken_block_chip = false;

    for (n, last) in SCENARIOS {
        let (scenario_path, _, _) = paths(n);
        let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
        let mut m = VersusMachine::from_checkpoint_2450_with_program(Path::new(REF_MOVE), rom)
            .expect("checkpoint 2450");
        let events = scenario_input_events(Path::new(&scenario_path)).expect("scenario events");
        let (mut next, mut held) = (0usize, [false; INPUTS]);

        let ryu_hp = |m: &VersusMachine| -> u8 {
            let w = m.work_ram();
            w[0x6b7] * 4 + w[0x6b6]
        };
        let ken_slot = |m: &VersusMachine| -> usize {
            let w = m.work_ram();
            u16::from_be_bytes([w[0x6da + 0x10], w[0x6da + 0x11]]) as usize
        };
        let ryu_slot = |m: &VersusMachine| -> usize {
            let w = m.work_ram();
            u16::from_be_bytes([w[0x6b0 + 0x10], w[0x6b0 + 0x11]]) as usize
        };

        let mut prev_hp = ryu_hp(&m);
        let mut prev_mv = 1u8;
        let mut cur_attack_hit = false;

        while m.state.frame < last {
            let frame = m.state.frame + 1;
            while next < events.len() && events[next].0 < frame {
                held[events[next].1] = events[next].2;
                next += 1;
            }
            apply_inputs(&mut m, &held);

            let ks_before = ken_slot(&m);
            let rs_before = ryu_slot(&m);
            let mv_before = m.state.objectram[ks_before + 0x0a];
            let ryu_st_before = m.state.objectram[rs_before + 0x09];
            let hf_before = u32::from_be_bytes([
                m.state.objectram[ks_before + 0x28],
                m.state.objectram[ks_before + 0x29],
                m.state.objectram[ks_before + 0x2a],
                m.state.objectram[ks_before + 0x2b],
            ]);

            m.step();
            assert!(
                m.unrecovered().is_empty(),
                "sf-ken-{n:02} frame {}: unrecovered {:05x?}",
                m.state.frame,
                m.unrecovered()
            );

            let ks = ken_slot(&m);
            let rs = ryu_slot(&m);
            let mv = m.state.objectram[ks + 0x0a];
            let hf_after = u32::from_be_bytes([
                m.state.objectram[ks + 0x28],
                m.state.objectram[ks + 0x29],
                m.state.objectram[ks + 0x2a],
                m.state.objectram[ks + 0x2b],
            ]);
            let hp = ryu_hp(&m);

            // Track 0x453b6 jump -> jump-attack transitions (9/0a/0b -> 20..26).
            if matches!(mv_before, 0x09..=0x0b) && matches!(mv, 0x20..=0x26) {
                jump_transitions.insert((mv_before, mv));
            }

            // Track Ken's special-command trackers at P2_STRUCT (0xff86da) +0x16 (Tatsumaki, done at 0xff86f2),
            // +0x19 (Shoryuken, done at 0xff86f5), and +0x1c (Hadoken, done at 0xff86f8).
            {
                let w = m.work_ram();
                if w[0x6f2] == 0xff {
                    command_trackers_completed[0] = true;
                }
                if w[0x6f5] == 0xff {
                    command_trackers_completed[1] = true;
                }
                if w[0x6f8] == 0xff {
                    command_trackers_completed[2] = true;
                }
            }

            // Track 0x4554a / 0x45566 Tatsumaki spin-loop globals at 0xff870a / 0xff870c.
            if mv == 0x1b {
                let w = m.work_ram();
                tatsu_counts.insert(w[0x70a]);
                let start = u32::from_be_bytes([w[0x70c], w[0x70d], w[0x70e], w[0x70f]]);
                if start != 0 {
                    tatsu_starts.insert(start);
                }
            }

            if mv != prev_mv {
                if prev_mv >= 0x0d && !cur_attack_hit {
                    whiffed_moves.insert(prev_mv);
                }
                cur_attack_hit = false;
                prev_mv = mv;
            }

            if hp < prev_hp {
                let dmg = prev_hp - hp;
                let hf = if hf_after != 0 { hf_after } else { hf_before };
                if hf != 0 {
                    landed_hfs.insert(hf);
                }
                cur_attack_hit = true;
                hit_moves.insert(mv);

                let ryu_mv_after = m.state.objectram[rs + 0x0a];
                if mv == 0x1b && ((ryu_st_before & 0x20 != 0) || ryu_mv_after == 0x30) && dmg == 8 {
                    saw_tatsu_block_chip = true;
                }
                if mv == 0x1a {
                    let w = m.work_ram();
                    if dmg == 20 && w[0x400b] != 0 {
                        saw_hadoken_unblocked_slowmo = true;
                    }
                    if dmg == 8 && ((ryu_st_before & 0x20 != 0) || ryu_mv_after == 0x30) {
                        saw_hadoken_block_chip = true;
                    }
                }
                prev_hp = hp;
            }
        }
        assert!(prev_hp > 0, "sf-ken-{n:02}: Ryu must not be KO'd");
    }

    assert_eq!(
        landed_hfs,
        KEN_TWIN_HIT_FUNCTIONS.into_iter().collect::<BTreeSet<_>>(),
        "all 16 twin hit functions must land at least once"
    );

    let expected_moves: BTreeSet<u8> = [
        0x0d, 0x0f, 0x10, 0x14, 0x15, 0x16, // standing normals
        0x11, 0x12, 0x13, 0x1c, 0x1d, 0x1e, 0x17, 0x18, // crouching
        0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, // jump attacks
        0x19, 0x1a, 0x1b, // Shoryuken, Hadoken, Tatsumaki
    ]
    .into_iter()
    .collect();
    assert_eq!(
        whiffed_moves, expected_moves,
        "every normal, crouching, jump, and special move must whiff"
    );
    assert_eq!(
        hit_moves, expected_moves,
        "every normal, crouching, jump, and special move must land"
    );

    // 0x453b6 with 0x4545c (punches 20, 21, 22) and 0x45474 (kicks 23, 26, 24, 25):
    let expected_jumps: BTreeSet<(u8, u8)> = [
        (0x09, 0x20),
        (0x0a, 0x21),
        (0x0b, 0x22),
        (0x09, 0x23),
        (0x09, 0x26),
        (0x0a, 0x24),
        (0x0b, 0x25),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        jump_transitions, expected_jumps,
        "0x453b6 must exercise every entry of 0x4545c and 0x45474"
    );

    // 0x4554a / 0x45566 spin loop at 0xff870a / 0xff870c:
    assert_eq!(
        tatsu_counts,
        [0u8, 1, 2, 3, 4].into_iter().collect::<BTreeSet<_>>(),
        "0x4554a / 0x45566 must count 0xff870a from 4 down to 0"
    );
    assert_eq!(
        tatsu_starts,
        [0x00358eu32].into_iter().collect::<BTreeSet<_>>(),
        "0x4554a must store loop start 0x00358e at 0xff870c"
    );

    assert_eq!(
        command_trackers_completed,
        [true, true, true],
        "Ken's Tatsumaki (0xff86f0), Shoryuken (0xff86f3), and Hadoken (0xff86f6) trackers must all complete"
    );
    assert!(
        saw_tatsu_block_chip,
        "Tatsumaki chip damage on blocking Ryu"
    );
    assert!(
        saw_hadoken_unblocked_slowmo,
        "Hadoken unblocked hit with slow-motion event 9"
    );
    assert!(
        saw_hadoken_block_chip,
        "Hadoken chip damage on blocking Ryu"
    );
}

#[test]
fn negative_check_catches_a_shifted_ken_input() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let (scenario_path, a, _) = paths(1);
    let raw = std::fs::read_to_string(&scenario_path).expect("scenario JSON readable");
    let shifted = raw.replacen("\"frame\": 2455,", "\"frame\": 2456,", 1);
    assert_ne!(
        raw, shifted,
        "frame 2455 input must exist in sf_ken_01.json"
    );

    let scratch = ScratchDir::new("sf-engine-ken-neg");
    std::fs::create_dir_all(&scratch).expect("scratch dir created");
    let shifted_path = scratch.join("sf_ken_01_shifted.json");
    std::fs::write(&shifted_path, shifted).expect("shifted scenario written");

    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let ram_res = sf_engine::run::compare_versus_ram(Path::new(&a), rom, &shifted_path, 2470)
        .expect("reference dumps readable");
    let Err(msg) = ram_res else {
        panic!("shifted Ken input must fail compare_versus_ram, got {ram_res:?}");
    };
    assert!(
        msg.contains("frame 2456") && msg.contains("button latches"),
        "expected button-latch divergence on frame 2456, got: {msg}"
    );

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let out = ScratchDir::new("sf-engine-ken-neg-cap");
    sf_engine::run::run_versus_interval_capture(
        Path::new(REF_MOVE),
        &assets,
        &out,
        "sf-ken-01",
        &shifted_path,
        2470,
    )
    .expect("native capture runs");
    let scenario = load_scenario(&scenario_path).unwrap();
    let outcome = compare(
        &scenario,
        &load_capture(&a).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope(2470),
    )
    .expect("compatible captures");
    assert!(
        !matches!(outcome, Outcome::Match { .. }),
        "shifted Ken input must fail observable comparison, got {outcome:?}"
    );
}
