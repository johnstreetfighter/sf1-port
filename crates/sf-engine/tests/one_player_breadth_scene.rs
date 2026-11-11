//! Acceptance test for ticket #54 (F14): crouching attacks, jump attacks,
//! blocks against Retsu, and held inputs on the `sf-cpu-01` boot path
//! (`docs/reference/crouch-attacks.md`, `docs/reference/jump-attacks.md`,
//! `docs/reference/blocking.md`):
//!
//! - `sf-1p-crouch-01` (`2389..=3223`): all six crouching normals (`LP`, `MP`,
//!   `HP`, `LK`, `MK`, `HK`) miss over `2390..2690` and land unblocked on Retsu
//!   over `2696..3203` (`48 -> 23` HP).
//! - `sf-1p-jump-01` (`2389..=2957`): neutral (`0x20`, `0x23`, `0x26`), forward
//!   (`0x21`, `0x24`) and back (`0x22`, `0x25`) jump punches and kicks, with
//!   `0x21`, `0x24`, `0x20` and `0x26` landing on Retsu (`48 -> 14` HP).
//! - `sf-1p-block-01` (`2389..=2700`): Ryu holds back (`2434..2530`), then
//!   down-back (`2530..2680`) against Retsu's attacks — high blocks (`move 4`,
//!   `+9 = 0x22`, sound `0x08` at `2461, 2487, 2514`), low blocks (`move 8`,
//!   `+9 = 0x24` at `2539..2547`, `2571..2579`, `2647..2672`), and wrong blocks
//!   (`move 8` hit by Retsu's `0x18` at `2548` and `2580`, `48 -> 40 -> 32` HP).
//! - `sf-1p-held-01` (`2389..=2680`): both horizontal directions held
//!   (`2395..2410`, `d0 = 3`), both vertical directions held (`2415..2430`,
//!   `d0 = 12`), and `p1_lp` held 276 frames (`2390..2666`, saturating
//!   `0xff8693` at `0xff` and releasing at `2667` in stance without firing).
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

use std::collections::BTreeSet;
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{
    compare_cpu_ram_with_inputs, observe_cpu_fight, run_cpu_fight_capture_with_inputs,
    CpuRamOutcome, ScratchDir,
};

const CPU_SCENARIO: &str = "../../tools/reference/scenarios/sf_cpu_01.json";
const CPU_REF_A: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIXTURE: u32 = 2389;

struct ScenarioSpec {
    id: &'static str,
    scenario: &'static str,
    ref_a: &'static str,
    ref_b: &'static str,
    last: u32,
    end_ryu_hp: u8,
    end_retsu_hp: u8,
}

const SCENARIOS: [ScenarioSpec; 4] = [
    ScenarioSpec {
        id: "sf-1p-crouch-01",
        scenario: "../../tools/reference/scenarios/sf_1p_crouch_01.json",
        ref_a: "../../tools/reference/local/captures/1p-crouch-01-run-a",
        ref_b: "../../tools/reference/local/captures/1p-crouch-01-run-b",
        last: 3223,
        end_ryu_hp: 10,
        end_retsu_hp: 23,
    },
    ScenarioSpec {
        id: "sf-1p-jump-01",
        scenario: "../../tools/reference/scenarios/sf_1p_jump_01.json",
        ref_a: "../../tools/reference/local/captures/1p-jump-01-run-a",
        ref_b: "../../tools/reference/local/captures/1p-jump-01-run-b",
        last: 2957,
        end_ryu_hp: 48,
        end_retsu_hp: 14,
    },
    ScenarioSpec {
        id: "sf-1p-block-01",
        scenario: "../../tools/reference/scenarios/sf_1p_block_01.json",
        ref_a: "../../tools/reference/local/captures/1p-block-01-run-a",
        ref_b: "../../tools/reference/local/captures/1p-block-01-run-b",
        last: 2700,
        end_ryu_hp: 32,
        end_retsu_hp: 48,
    },
    ScenarioSpec {
        id: "sf-1p-held-01",
        scenario: "../../tools/reference/scenarios/sf_1p_held_01.json",
        ref_a: "../../tools/reference/local/captures/1p-held-01-run-a",
        ref_b: "../../tools/reference/local/captures/1p-held-01-run-b",
        last: 2680,
        end_ryu_hp: 34,
        end_retsu_hp: 48,
    },
];

fn prerequisites_present() -> bool {
    Path::new(ROMS).is_dir()
        && Path::new(MANIFEST).is_file()
        && Path::new(CPU_REF_A).is_dir()
        && SCENARIOS
            .iter()
            .all(|s| Path::new(s.ref_a).is_dir() && Path::new(s.ref_b).is_dir())
}

/// The fixture row (2389) is excluded from the per-frame walk: its screen
/// hash is the scanout of 2388 (FIGHT! still shown), which a post-erase
/// fixture cannot reproduce.
fn native_scope(last: u32) -> CompareScope {
    CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(last as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

fn hp(w: &[u8], ps_off: usize) -> u8 {
    w[ps_off + 7] * 4 + w[ps_off + 6]
}

#[test]
fn reference_runs_repeat_and_equal_sf_cpu_01_through_2389() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let cpu_scenario = load_scenario(CPU_SCENARIO).unwrap();
    let cpu_a = load_capture(CPU_REF_A).unwrap();
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(FIXTURE as u64),
        fields: None,
    };

    for s in &SCENARIOS {
        let scenario = load_scenario(s.scenario).unwrap();
        let a = load_capture(s.ref_a).unwrap();
        let b = load_capture(s.ref_b).unwrap();

        let full_scope = CompareScope {
            from: Some(0),
            to: Some(s.last as u64),
            fields: None,
        };
        let repeat = compare(&scenario, &a, &b, CandidateKind::Reference, &full_scope)
            .expect("compatible captures");
        let expected_checkpoints = (s.last - FIXTURE + 1) as usize;
        let Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } = repeat
        else {
            panic!("{}: reference repeatability: {repeat:?}", s.id);
        };
        assert_eq!(
            (frames_compared, checkpoints_compared),
            ((s.last + 1) as u64, expected_checkpoints),
            "{}: reference repeatability",
            s.id
        );

        let mut a_as_cpu = load_capture(s.ref_a).unwrap();
        a_as_cpu.scenario_id = Some(cpu_scenario.scenario_id.clone());
        let prefix = compare(
            &cpu_scenario,
            &cpu_a,
            &a_as_cpu,
            CandidateKind::Reference,
            &prefix_scope,
        )
        .expect("compatible prefix");
        let Outcome::Match {
            frames_compared: prefix_frames,
            checkpoints_compared: prefix_checkpoints,
        } = prefix
        else {
            panic!("{}: prefix mismatch vs sf-cpu-01: {prefix:?}", s.id);
        };
        assert_eq!(
            (prefix_frames, prefix_checkpoints),
            ((FIXTURE + 1) as u64, 1),
            "{}: must equal sf-cpu-01 on every field through 2389",
            s.id
        );

        // Each scenario ends before its match decision (both fighters alive in
        // state 2, no round/match outcome flagged).
        let end_work =
            std::fs::read(Path::new(s.ref_a).join(format!("ram_{:06}_work.bin", s.last))).unwrap();
        let ryu_e = u16::from_be_bytes([end_work[0x06c0], end_work[0x06c1]]) as usize;
        let retsu_e = u16::from_be_bytes([end_work[0x4674], end_work[0x4675]]) as usize;
        assert_eq!(
            end_work[0x6000 + ryu_e + 8],
            2,
            "{}: Ryu alive at end",
            s.id
        );
        assert_eq!(
            end_work[0x6000 + retsu_e + 8],
            2,
            "{}: Retsu alive at end",
            s.id
        );
        assert_eq!(
            hp(&end_work, 0x06b0),
            s.end_ryu_hp,
            "{}: Ryu HP at end",
            s.id
        );
        assert_eq!(
            hp(&end_work, 0x4664),
            s.end_retsu_hp,
            "{}: Retsu HP at end",
            s.id
        );
        assert_eq!(end_work[0x4008], 0x80, "{}: active fight mode at end", s.id);
        assert_eq!(end_work[0x005a], 0, "{}: before round/match decision", s.id);
    }
}

#[test]
fn native_runs_match_the_reference_and_owned_ram() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    for s in &SCENARIOS {
        let out = ScratchDir::new(&format!("sf-engine-{}", s.id));
        run_cpu_fight_capture_with_inputs(
            Path::new(s.ref_a),
            &assets,
            &out,
            s.id,
            Some(Path::new(s.scenario)),
            FIXTURE,
            s.last,
        )
        .expect("native capture runs on translated paths only");

        let scenario = load_scenario(s.scenario).unwrap();
        let outcome = compare(
            &scenario,
            &load_capture(s.ref_a).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &native_scope(s.last),
        )
        .expect("compatible captures");
        let frames = s.last - FIXTURE;
        let Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } = outcome
        else {
            panic!("{}: native observables: {outcome:?}", s.id);
        };
        assert_eq!(
            (frames_compared, checkpoints_compared),
            (frames as u64, frames as usize),
            "{}: native observables",
            s.id
        );

        let ram = compare_cpu_ram_with_inputs(
            Path::new(s.ref_a),
            rom.clone(),
            Some(Path::new(s.scenario)),
            FIXTURE,
            s.last,
        )
        .expect("reference dumps readable");
        assert_eq!(ram, CpuRamOutcome::Match { frames }, "{}: owned RAM", s.id);
    }
}

#[test]
fn crouch_jump_block_and_held_paths_are_exercised() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    // 1. sf-1p-crouch-01: all 6 crouching normals ((0x11,1), (0x1c,2), (0x1c,3),
    //    (0x17,1), (0x17,2), (0x18,3)) both miss and land unblocked.
    let mut crouch_hits = BTreeSet::new();
    let mut crouch_whiffs = BTreeSet::new();
    let mut cur_crouch: Option<(u8, u8, bool)> = None;
    let mut prev_retsu_hp = 48u8;
    let (end_f, unrec) = observe_cpu_fight(
        Path::new(SCENARIOS[0].ref_a),
        rom.clone(),
        Some(Path::new(SCENARIOS[0].scenario)),
        FIXTURE,
        SCENARIOS[0].last,
        |m| {
            let w = m.work_ram();
            let obj = &m.state.objectram;
            let ryu_e = u16::from_be_bytes([w[0x06c0], w[0x06c1]]) as usize;
            let mv = obj[ryu_e + 0xa];
            let lvl = obj[ryu_e + 0x34];
            let is_crouch_atk = matches!(mv, 0x11..=0x13 | 0x17 | 0x18 | 0x1c..=0x1e);
            if is_crouch_atk {
                match &mut cur_crouch {
                    Some((m0, l0, _)) if (*m0, *l0) == (mv, lvl) => {}
                    prev => {
                        if let Some((m0, l0, landed)) = prev.take() {
                            if landed {
                                crouch_hits.insert((m0, l0));
                            } else {
                                crouch_whiffs.insert((m0, l0));
                            }
                        }
                        cur_crouch = Some((mv, lvl, false));
                    }
                }
            } else if let Some((m0, l0, landed)) = cur_crouch.take() {
                if landed {
                    crouch_hits.insert((m0, l0));
                } else {
                    crouch_whiffs.insert((m0, l0));
                }
            }
            let r_hp = hp(w, 0x4664);
            if r_hp < prev_retsu_hp {
                if let Some((_, _, landed)) = &mut cur_crouch {
                    *landed = true;
                }
                prev_retsu_hp = r_hp;
            }
        },
    )
    .unwrap();
    assert_eq!((end_f, unrec), (SCENARIOS[0].last, Vec::<u32>::new()));
    let expected_crouch: BTreeSet<(u8, u8)> = [
        (0x11, 1),
        (0x1c, 2),
        (0x1c, 3),
        (0x17, 1),
        (0x17, 2),
        (0x18, 3),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        crouch_whiffs, expected_crouch,
        "all 6 crouching normals whiff"
    );
    assert_eq!(crouch_hits, expected_crouch, "all 6 crouching normals hit");

    // 2. sf-1p-jump-01: neutral (0x20, 0x23, 0x26), forward (0x21, 0x24) and back
    //    (0x22, 0x25) jump punches and kicks execute.
    let mut jump_moves = BTreeSet::new();
    let mut jump_hit_moves = BTreeSet::new();
    let mut prev_retsu_hp = 48u8;
    let (end_f, unrec) = observe_cpu_fight(
        Path::new(SCENARIOS[1].ref_a),
        rom.clone(),
        Some(Path::new(SCENARIOS[1].scenario)),
        FIXTURE,
        SCENARIOS[1].last,
        |m| {
            let w = m.work_ram();
            let obj = &m.state.objectram;
            let ryu_e = u16::from_be_bytes([w[0x06c0], w[0x06c1]]) as usize;
            let mv = obj[ryu_e + 0xa];
            if (0x20..=0x26).contains(&mv) {
                jump_moves.insert(mv);
            }
            let r_hp = hp(w, 0x4664);
            if r_hp < prev_retsu_hp {
                jump_hit_moves.insert(mv);
                prev_retsu_hp = r_hp;
            }
        },
    )
    .unwrap();
    assert_eq!((end_f, unrec), (SCENARIOS[1].last, Vec::<u32>::new()));
    let expected_jump: BTreeSet<u8> = (0x20..=0x26).collect();
    assert_eq!(
        jump_moves, expected_jump,
        "all 7 jump-attack moves (0x20..=0x26) execute"
    );
    assert_eq!(
        jump_hit_moves,
        [0x20, 0x21, 0x24, 0x26].into_iter().collect(),
        "neutral and forward jump punches and kicks land on Retsu"
    );

    // 3. sf-1p-block-01: high block (move 4, +9 = 0x22, sound 0x08), low block
    //    (move 8, +9 = 0x24), and wrong block (move 8 hit by Retsu).
    let mut high_block_sound_frames = Vec::new();
    let mut low_block_frames = Vec::new();
    let mut wrong_block_hit_frames = Vec::new();
    let mut prev_ryu_hp = 48u8;
    let mut prev_ryu_mv = 1u8;
    let mut prev_ryu_st = 0x12u8;
    let (end_f, unrec) = observe_cpu_fight(
        Path::new(SCENARIOS[2].ref_a),
        rom.clone(),
        Some(Path::new(SCENARIOS[2].scenario)),
        FIXTURE,
        SCENARIOS[2].last,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            let obj = &m.state.objectram;
            let ryu_e = u16::from_be_bytes([w[0x06c0], w[0x06c1]]) as usize;
            let mv = obj[ryu_e + 0xa];
            let st = obj[ryu_e + 0x9];
            let r_hp = hp(w, 0x06b0);
            if mv == 4 && st == 0x22 && m.state.sound_latch == 0x08 && r_hp == prev_ryu_hp {
                high_block_sound_frames.push(f);
            }
            if mv == 8 && st == 0x24 {
                low_block_frames.push(f);
            }
            if r_hp < prev_ryu_hp && prev_ryu_mv == 8 && prev_ryu_st == 0x24 {
                wrong_block_hit_frames.push(f);
            }
            prev_ryu_hp = r_hp;
            prev_ryu_mv = mv;
            prev_ryu_st = st;
        },
    )
    .unwrap();
    assert_eq!((end_f, unrec), (SCENARIOS[2].last, Vec::<u32>::new()));
    assert_eq!(high_block_sound_frames, vec![2461, 2487, 2514]);
    assert!(low_block_frames.contains(&2539) && low_block_frames.contains(&2647));
    assert_eq!(wrong_block_hit_frames, vec![2548, 2580]);

    // 4. sf-1p-held-01: both horizontal directions (d0 = 3), both vertical
    //    directions (d0 = 12), and LP held > 255 frames saturating 0xff8693 at 0xff
    //    and releasing at 2667 in stance without firing.
    let mut horiz_both_frames = Vec::new();
    let mut vert_both_frames = Vec::new();
    let mut saturated_hold_frames = Vec::new();
    let mut state_at_2667 = (0u8, 0u8, 0u8);
    let (end_f, unrec) = observe_cpu_fight(
        Path::new(SCENARIOS[3].ref_a),
        rom,
        Some(Path::new(SCENARIOS[3].scenario)),
        FIXTURE,
        SCENARIOS[3].last,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            let obj = &m.state.objectram;
            let ryu_e = u16::from_be_bytes([w[0x06c0], w[0x06c1]]) as usize;
            let mv = obj[ryu_e + 0xa];
            let st = obj[ryu_e + 0x9];
            let d0 = !w[0x068f] & 0x0f;
            if d0 == 3 && st == 0x12 {
                horiz_both_frames.push(f);
            }
            if d0 == 12 && st == 0x12 {
                vert_both_frames.push(f);
            }
            if w[0x0693] == 0xff {
                saturated_hold_frames.push(f);
            }
            if f == 2667 {
                state_at_2667 = (w[0x0690], w[0x06b0], mv);
            }
        },
    )
    .unwrap();
    assert_eq!((end_f, unrec), (SCENARIOS[3].last, Vec::<u32>::new()));
    assert!(
        !horiz_both_frames.is_empty(),
        "both horizontal directions reached in stance"
    );
    assert!(
        !vert_both_frames.is_empty(),
        "both vertical directions reached in stance"
    );
    assert_eq!(
        saturated_hold_frames,
        (2646..=2666).collect::<Vec<u32>>(),
        "punch hold counter 0xff8693 saturated at 0xff over 2646..=2666"
    );
    assert_eq!(
        state_at_2667,
        (0, 0, 1),
        "releasing saturated LP at 2667 clears latch state without firing a punch"
    );
}

#[test]
fn negative_control_moving_one_input_by_one_frame_is_caught_at_first_differing_frame() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    // For each of the four scenarios, moving the first fight input at frame 2390
    // by one frame (2390 -> 2391) is caught at frame 2391 (the first differing
    // frame) in owned RAM, and in native observables for crouch, jump and block
    // (where 2390 is a movement/posture input; in sf-1p-held-01, shifting the
    // back-hold input at 2435 -> 2436 is caught at 2436 in native observables).
    let scratch = ScratchDir::new("sf-engine-neg-1p-breadth");
    std::fs::create_dir_all(&scratch).unwrap();

    for s in &SCENARIOS {
        let raw = std::fs::read_to_string(s.scenario).unwrap();
        let shifted_json = raw.replacen("\"frame\": 2390,", "\"frame\": 2391,", 1);
        assert_ne!(
            raw, shifted_json,
            "{}: scenario input must be shifted by 1 frame",
            s.id
        );
        let shifted_scenario_path = scratch.as_ref().join(format!("{}_2391.json", s.id));
        std::fs::write(&shifted_scenario_path, shifted_json).unwrap();

        let ram = compare_cpu_ram_with_inputs(
            Path::new(s.ref_a),
            rom.clone(),
            Some(&shifted_scenario_path),
            FIXTURE,
            2400,
        )
        .unwrap();
        let CpuRamOutcome::Mismatch(msg) = ram else {
            panic!("{}: expected RAM mismatch at frame 2391, got {ram:?}", s.id);
        };
        assert!(
            msg.starts_with("frame 2391:"),
            "{}: expected mismatch at first differing frame 2391, got: {msg}",
            s.id
        );
    }

    let s = &SCENARIOS[0];
    let shifted_crouch_path = scratch.as_ref().join("sf-1p-crouch-01_2391.json");
    let out_cap = ScratchDir::new("sf-engine-neg-1p-crouch-cap");
    run_cpu_fight_capture_with_inputs(
        Path::new(s.ref_a),
        &assets,
        &out_cap,
        s.id,
        Some(&shifted_crouch_path),
        FIXTURE,
        2400,
    )
    .expect("shifted run finishes");
    let scenario = load_scenario(s.scenario).unwrap();
    let obs = compare(
        &scenario,
        &load_capture(s.ref_a).unwrap(),
        &load_capture(&out_cap).unwrap(),
        CandidateKind::Native,
        &native_scope(2400),
    )
    .unwrap();
    let Outcome::Mismatch(div) = obs else {
        panic!("expected observable mismatch at 2391, got {obs:?}");
    };
    assert_eq!(div.frame, 2391);
}
