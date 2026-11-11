//! Acceptance tests for ticket #72 (F32): one-player round 3 after a 1-1
//! split, a time-over decided on health, an equal-health time-over draw, and a
//! double KO, on the `sf-cpu-01` boot path from checkpoint `2389`:
//!
//! - `sf-1p-round3-01` (`2389..=7600`): Ryu wins round 1 by heavy Shoryuken KO
//!   at `2898` (`1-0`), idles through round 2 for Retsu to even the match at
//!   `4307` (`1-1`), enters round 3 (`round = 2`) at `4911`, and throws heavy
//!   Hadokens to survive round 3 to its time-over at `6800` (`0x3ed26`, text
//!   `0x14f74`, sound `0x96`). The health comparison at `6930` (`0x3ed98`)
//!   finds Ryu ahead (`48 > 16`), giving Ryu round 3 (`2-1`), the match
//!   decision at `7216` (`0xffc009 = 1`, `0x3ef54`), and Geki's setup at
//!   `7586..=7600` (`0x3e6ca`, `stage = 0, fight = 1, round = 0`).
//! - `sf-1p-draw-01` (`2389..=7560`): after the same `1-1` split through
//!   `4911`, Ryu blocks high in round 3 to reach time-over at `6800` with
//!   equal health (`48 == 48`). The judgement at `6930` requests event 8
//!   (`DRAW GAME`, `0x3fef2`), after which `0x3edf6 -> 0x3ee04` (`three_rounds`)
//!   sees equal wins (`1 == 1`, `0xffc009 = 0`) and enters the CPU's match win
//!   at `0x3eefa` and `F5`'s continue screen (`0x3efda..0x3f162`) at
//!   `7541..=7560`.
//! - `sf-1p-doubleko-01` (`2389..=6320`): after the same `1-1` split through
//!   `4911`, Ryu and Retsu trade a final blow in round 3 for a double KO at
//!   `5770..5771` (`0x3eca2..0x3ed00`, `DOUBLE K.O.` `0x14f56`). At `5900`,
//!   `0x3edf6 -> 0x3ee04` sees equal wins (`1 == 1`, `0xffc009 = 0`) and
//!   enters the CPU's match win at `0x3eefa` and `F5`'s continue screen at
//!   `6300..=6320`.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{
    compare_cpu_ram_with_inputs, observe_cpu_fight, run_cpu_fight_capture_with_inputs,
    CpuRamOutcome, ScratchDir,
};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const CPU_01_A: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const CPU_01_SCENARIO: &str = "../../tools/reference/scenarios/sf_cpu_01.json";
const FIXTURE: u32 = 2389;

struct ScenarioSpec {
    name: &'static str,
    capture_last: u32,
}

const SCENARIOS: [ScenarioSpec; 3] = [
    ScenarioSpec {
        name: "1p-round3-01",
        capture_last: 7600,
    },
    ScenarioSpec {
        name: "1p-draw-01",
        capture_last: 7560,
    },
    ScenarioSpec {
        name: "1p-doubleko-01",
        capture_last: 6320,
    },
];

fn paths(s: &ScenarioSpec) -> (String, String, String) {
    (
        format!(
            "../../tools/reference/scenarios/sf_{}.json",
            s.name.replace('-', "_")
        ),
        format!("../../tools/reference/local/captures/{}-run-a", s.name),
        format!("../../tools/reference/local/captures/{}-run-b", s.name),
    )
}

fn prerequisites_present() -> bool {
    Path::new(ROMS).is_dir()
        && Path::new(CPU_01_A).is_dir()
        && SCENARIOS.iter().all(|s| {
            let (_, a, b) = paths(s);
            Path::new(&a).is_dir() && Path::new(&b).is_dir()
        })
}

fn ref_scope(last: u32) -> CompareScope {
    CompareScope {
        from: Some(0),
        to: Some(last as u64),
        fields: None,
    }
}

/// The fixture row (`2389`) is excluded on native screen-hash comparisons: its
/// `frames.tsv` hash is the scanout of `2388` (`FIGHT!` still shown), which a
/// post-erase fixture at `2389` does not reproduce.
fn native_scope(last: u32) -> CompareScope {
    CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(last as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    }
}

#[test]
fn reference_runs_repeat_exactly_and_match_cpu_01_through_2389() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let _cpu_01_scen = load_scenario(CPU_01_SCENARIO).unwrap();
    let prefix_scope = CompareScope {
        from: Some(0),
        to: Some(FIXTURE as u64),
        fields: None,
    };

    for s in &SCENARIOS {
        let (scen_path, a, b) = paths(s);
        let scenario = load_scenario(&scen_path).unwrap();
        let cap_a = load_capture(&a).unwrap();
        let cap_b = load_capture(&b).unwrap();
        let outcome = compare(
            &scenario,
            &cap_a,
            &cap_b,
            CandidateKind::Reference,
            &ref_scope(s.capture_last),
        )
        .expect("compatible reference captures");
        let expected_checkpoints = (s.capture_last - FIXTURE + 1) as usize;
        let Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } = outcome
        else {
            panic!("{}: {outcome:?}", s.name);
        };
        assert_eq!(
            (frames_compared, checkpoints_compared),
            ((s.capture_last + 1) as u64, expected_checkpoints),
            "{}",
            s.name
        );

        // Equals sf-cpu-01 through 2389 (frames.tsv rows 0..=2389 and checkpoint 2389).
        let mut cpu_as_scen = load_capture(CPU_01_A).unwrap();
        cpu_as_scen.scenario_id = Some(scenario.scenario_id.clone());
        let prefix_outcome = compare(
            &scenario,
            &cpu_as_scen,
            &cap_a,
            CandidateKind::Reference,
            &prefix_scope,
        )
        .expect("compatible prefix captures");
        assert!(
            matches!(
                prefix_outcome,
                Outcome::Match {
                    frames_compared: 2390,
                    checkpoints_compared: 1,
                }
            ),
            "{} prefix vs sf-cpu-01: {prefix_outcome:?}",
            s.name
        );
    }
}

#[test]
fn native_matches_reference_on_every_observable_and_owned_ram() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    for s in &SCENARIOS {
        let (scen_path, a, _) = paths(s);
        let id = format!("sf-{}", s.name);
        let out = ScratchDir::new(&format!("sf-engine-{}", s.name));
        let executed = run_cpu_fight_capture_with_inputs(
            Path::new(&a),
            &assets,
            &out,
            &id,
            Some(Path::new(&scen_path)),
            FIXTURE,
            s.capture_last,
        )
        .expect("native capture runs on translated paths only");
        let frames = s.capture_last - FIXTURE;
        assert_eq!(executed, (frames + 1) as u64, "{id}");

        let scenario = load_scenario(&scen_path).unwrap();
        let outcome = compare(
            &scenario,
            &load_capture(&a).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &native_scope(s.capture_last),
        )
        .expect("compatible captures");
        let Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } = outcome
        else {
            panic!("{id}: {outcome:?}");
        };
        assert_eq!(
            (frames_compared, checkpoints_compared),
            (frames as u64, frames as usize),
            "{id}"
        );

        let ram_outcome = compare_cpu_ram_with_inputs(
            Path::new(&a),
            rom.clone(),
            Some(Path::new(&scen_path)),
            FIXTURE,
            s.capture_last,
        )
        .expect("reference dumps readable");
        assert_eq!(ram_outcome, CpuRamOutcome::Match { frames }, "{id}");
    }
}

#[test]
fn round_3_outcomes_and_continuations_are_exercised() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    // 1. sf-1p-round3-01: 1-1 split -> round 3 time-over at 6800 (0x14f74,
    // sound 0x96) -> Ryu wins on health (48 > 16) at 6930 -> match decision
    // at 7216 (0xffc009 = 1) -> Geki's setup at 7586..=7600 (stage 0, fight 1).
    {
        let (scen_path, a, _) = paths(&SCENARIOS[0]);
        let read =
            |f: u32| std::fs::read(Path::new(&a).join(format!("ram_{f:06}_work.bin"))).unwrap();
        let w_r3 = read(4911);
        assert_eq!(
            (w_r3[0x400a], w_r3[0x6b8], w_r3[0x466c]),
            (2, 1, 1),
            "round 3 starts at 4911 after a 1-1 split"
        );
        let w_to = read(6800);
        assert_eq!(
            (
                &w_to[0xe18..0xe1b],
                w_to[0x6b7] * 4 + w_to[0x6b6],
                w_to[0x466b] * 4 + w_to[0x466a]
            ),
            (&[0u8, 0, 0][..], 48, 16),
            "round 3 time-over at 6800 with Ryu ahead on health 48 vs 16"
        );
        let w_win = read(6930);
        assert_eq!(
            (w_win[0x6b8], w_win[0x466c], w_win[0xe36]),
            (2, 1, 7),
            "health comparison at 6930 awards round 3 to Ryu (2-1) and requests event 7"
        );
        let w_match = read(7216);
        assert_eq!(
            (w_match[0x4009], w_match[0xe36]),
            (1, 11),
            "match decision at 7216 records P1 match win (0xffc009 = 1) and requests event 11"
        );
        let w_geki = read(7600);
        assert_eq!(
            (
                w_geki[0x4003],
                w_geki[0x4004],
                w_geki[0x400a],
                w_geki[0xe36]
            ),
            (0, 1, 0, 4),
            "continues into Geki's setup (0x3e6ca -> event 4, stage 0, fight 1, round 0)"
        );

        let mut saw_time_over_sound = false;
        let (end_f, unrec) = observe_cpu_fight(
            Path::new(&a),
            rom.clone(),
            Some(Path::new(&scen_path)),
            FIXTURE,
            SCENARIOS[0].capture_last,
            |m| {
                if m.state.frame == 6800 {
                    assert!(m.task_positions().contains(&(1, 0x3ed26)));
                }
                if (6800..=6803).contains(&m.state.frame) && m.state.sound_latch == 0x96 {
                    saw_time_over_sound = true;
                }
            },
        )
        .unwrap();
        assert_eq!((end_f, unrec), (SCENARIOS[0].capture_last, Vec::new()));
        assert!(
            saw_time_over_sound,
            "time-over at 6800 must queue sound 0x96"
        );
    }

    // 2. sf-1p-draw-01: 1-1 split -> round 3 time-over at 6800 with equal
    // health (48 == 48) -> event 8 (DRAW GAME) at 6930 -> 3-rounds rule equal
    // wins (0xffc009 = 0) -> CPU match win at 0x3eefa -> continue screen at 7541..=7560.
    {
        let (scen_path, a, _) = paths(&SCENARIOS[1]);
        let read =
            |f: u32| std::fs::read(Path::new(&a).join(format!("ram_{f:06}_work.bin"))).unwrap();
        let w_to = read(6800);
        assert_eq!(
            (
                &w_to[0xe18..0xe1b],
                w_to[0x6b7] * 4 + w_to[0x6b6],
                w_to[0x466b] * 4 + w_to[0x466a]
            ),
            (&[0u8, 0, 0][..], 48, 48),
            "round 3 time-over at 6800 with equal health 48 == 48"
        );
        let w_draw = read(6930);
        assert_eq!(
            (w_draw[0xe36], w_draw[0x6b8], w_draw[0x466c]),
            (8, 1, 1),
            "equal-health judgement at 6930 requests event 8 (DRAW GAME) with wins still 1-1"
        );
        let w_cpu_win = read(7141);
        assert_eq!(
            (w_cpu_win[0x400a], w_cpu_win[0x4009], w_cpu_win[0xe36]),
            (3, 0, 11),
            "three rounds played with equal wins leaves 0xffc009 = 0 and enters CPU match win (event 11)"
        );
        let w_cont = read(7560);
        assert_eq!(
            (w_cont[0x4008], w_cont[0x4009], w_cont[0x400a]),
            (8, 0, 0),
            "continues into F5's continue screen (SCENE_MODE = 8)"
        );

        let mut saw_continue_loop = false;
        let (end_f, unrec) = observe_cpu_fight(
            Path::new(&a),
            rom.clone(),
            Some(Path::new(&scen_path)),
            FIXTURE,
            SCENARIOS[1].capture_last,
            |m| {
                if m.state.frame == 7543 {
                    saw_continue_loop = m.task_positions().contains(&(1, 0x3f162));
                }
            },
        )
        .unwrap();
        assert_eq!((end_f, unrec), (SCENARIOS[1].capture_last, Vec::new()));
        assert!(saw_continue_loop, "must reach continue loop 0x3f162");
    }

    // 3. sf-1p-doubleko-01: 1-1 split -> round 3 double KO at 5770..5771
    // (0x3eca2..0x3ed00, DOUBLE K.O. 0x14f56) -> 3-rounds rule equal wins at
    // 5900 (0xffc009 = 0) -> CPU match win at 0x3eefa -> continue screen at 6300..=6320.
    {
        let (scen_path, a, _) = paths(&SCENARIOS[2]);
        let read =
            |f: u32| std::fs::read(Path::new(&a).join(format!("ram_{f:06}_work.bin"))).unwrap();
        let w_dko = read(5770);
        assert_eq!(
            (
                w_dko[0x6408],
                w_dko[0x6a08],
                w_dko[0x400a],
                w_dko[0x6b8],
                w_dko[0x466c]
            ),
            (8, 8, 2, 1, 1),
            "both fighters knocked out in round 3 at 5770"
        );
        let w_cpu_win = read(5900);
        assert_eq!(
            (w_cpu_win[0x400a], w_cpu_win[0x4009], w_cpu_win[0xe36]),
            (3, 0, 11),
            "three rounds played after double KO leaves 0xffc009 = 0 and enters CPU match win (event 11)"
        );
        let w_cont = read(6320);
        assert_eq!(
            (w_cont[0x4008], w_cont[0x4009], w_cont[0x400a]),
            (8, 0, 0),
            "continues into F5's continue screen (SCENE_MODE = 8)"
        );

        let mut saw_double_ko_hold = false;
        let mut saw_continue_loop = false;
        let (end_f, unrec) = observe_cpu_fight(
            Path::new(&a),
            rom,
            Some(Path::new(&scen_path)),
            FIXTURE,
            SCENARIOS[2].capture_last,
            |m| {
                if m.state.frame == 5771 {
                    saw_double_ko_hold = m.task_positions().contains(&(1, 0x3ed00));
                }
                if m.state.frame == 6302 {
                    saw_continue_loop = m.task_positions().contains(&(1, 0x3f162));
                }
            },
        )
        .unwrap();
        assert_eq!((end_f, unrec), (SCENARIOS[2].capture_last, Vec::new()));
        assert!(saw_double_ko_hold, "must reach double-KO hold 0x3ed00");
        assert!(saw_continue_loop, "must reach continue loop 0x3f162");
    }
}

#[test]
fn round_3_time_over_moved_one_tick_late_is_caught() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let (scen_path, a, _) = paths(&SCENARIOS[0]);
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    // At frame 6780 (one 19-frame tick before the final countdown to 000 at
    // 6799 that triggers TIME OVER at 6800), add one tick to the round timer
    // (`0xff8e1a`: 1 -> 2) so round 3's time-over fires one tick (19 frames)
    // late at 6819 instead of 6800.
    let fixture_dir = ScratchDir::new("sf-engine-round3-late-fixture");
    std::fs::create_dir_all(&*fixture_dir).unwrap();
    for entry in [
        "frames.tsv",
        "ram_006780_work.bin",
        "ram_006780_video.bin",
        "ram_006780_palette.bin",
        "ram_006781_work.bin",
    ] {
        std::fs::copy(Path::new(&a).join(entry), fixture_dir.join(entry)).unwrap();
    }
    let work_path = fixture_dir.join("ram_006780_work.bin");
    let mut work = std::fs::read(&work_path).unwrap();
    assert_eq!(work[0x0e1a], 2, "TIME ones digit at 6780 must be 2");
    work[0x0e1a] = 3;
    std::fs::write(&work_path, &work).unwrap();

    // 1. Owned RAM comparison catches the 1-tick-late timer immediately at 6781.
    let ram_outcome =
        compare_cpu_ram_with_inputs(&fixture_dir, rom, Some(Path::new(&scen_path)), 6780, 6781)
            .unwrap();
    let CpuRamOutcome::Mismatch(msg) = ram_outcome else {
        panic!("one-tick-late timer unexpectedly matched RAM: {ram_outcome:?}");
    };
    assert!(
        msg.contains("frame 6781: round-timer task") && msg.contains("0xff8e1a"),
        "{msg}"
    );

    // 2. Native capture comparison over 6781..=6819 catches the missing
    // time-over (TIME OVER 0x14f74 and sound 0x96 moved from 6800 to 6819).
    let out = ScratchDir::new("sf-engine-round3-late-capture");
    run_cpu_fight_capture_with_inputs(
        &fixture_dir,
        &assets,
        &out,
        "sf-1p-round3-01",
        Some(Path::new(&scen_path)),
        6780,
        6819,
    )
    .unwrap();
    let scope = CompareScope {
        from: Some(6781),
        to: Some(6819),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let outcome = compare(
        &load_scenario(&scen_path).unwrap(),
        &load_capture(&a).unwrap(),
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    let Outcome::Mismatch(mismatch) = outcome else {
        panic!("one-tick-late time-over capture unexpectedly matched");
    };
    assert!(
        mismatch.frame <= 6800,
        "expected mismatch by time-over frame 6800, got {mismatch:?}"
    );
}
