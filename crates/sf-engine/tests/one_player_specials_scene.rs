//! Acceptance test for ticket #53 (F13): Ryu's Hadoken and Tatsumaki against
//! Retsu in one-player play from the checkpoint-2389 fixture of the
//! `sf-cpu-01` boot path (`docs/reference/hadoken.md`,
//! `docs/reference/specials.md`):
//!
//! - `sf-1p-hadoken-01` (`2389..=2729`): LP Hadoken at 2415 misses under
//!   jumping Retsu (exiting off the right edge at 2475); Ryu walks back
//!   (`2468..2504`); LP Hadoken at 2534 overlaps Retsu's jump-in attack box
//!   at 2549 (reading stage 0 of `0x434d0`) and hits Retsu unblocked at 2550
//!   (`48 -> 28` HP, requesting slow motion event 9 over `2550..=2639`); MP
//!   Hadoken at 2641 is blocked by Retsu at 2697 (reading stage 0 of the
//!   one-player chip table `0x4340a`, chip damage 8: `28 -> 20` HP).
//! - `sf-1p-tatsumaki-01` (`2389..=2649`): HK Tatsumaki at 2390 starts move
//!   `0x1b` at 2406 and hits Retsu unblocked at 2438 (`48 -> 28` HP); HK
//!   Tatsumaki at 2510 starts move `0x1b` at 2520 and is blocked by Retsu at
//!   2564 (reading stage 0 of the one-player chip table `0x45fa8`, chip
//!   damage 8: `28 -> 20` HP).
//!
//! Consumes local-only data (reference captures, ROM set); skips on a clean
//! checkout.

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
}

const SCENARIOS: [ScenarioSpec; 2] = [
    ScenarioSpec {
        id: "sf-1p-hadoken-01",
        scenario: "../../tools/reference/scenarios/sf_1p_hadoken_01.json",
        ref_a: "../../tools/reference/local/captures/1p-hadoken-01-run-a",
        ref_b: "../../tools/reference/local/captures/1p-hadoken-01-run-b",
        last: 2729,
    },
    ScenarioSpec {
        id: "sf-1p-tatsumaki-01",
        scenario: "../../tools/reference/scenarios/sf_1p_tatsumaki_01.json",
        ref_a: "../../tools/reference/local/captures/1p-tatsumaki-01-run-a",
        ref_b: "../../tools/reference/local/captures/1p-tatsumaki-01-run-b",
        last: 2649,
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

fn half_extent(n: u8, big: bool) -> u16 {
    let cell = if big { 0x20 } else { 0x10 };
    ((n as u16).wrapping_mul(cell).wrapping_add(cell)) >> 1
}

fn borrow_abs(a: u16, b: u16) -> u16 {
    let d = a.wrapping_sub(b);
    if a < b {
        d.wrapping_neg()
    } else {
        d
    }
}

fn projectile_overlaps_retsu_attack_box(w: &[u8], obj: &[u8]) -> bool {
    let t = u16::from_be_bytes([w[0x4664 + 0x10], w[0x4664 + 0x11]]) as usize;
    let t_attr = u16::from_be_bytes([obj[t + 2], obj[t + 3]]);
    let t_big = (t_attr as i16) >= 0x400;
    let ty0 = u16::from_be_bytes([obj[t + 4], obj[t + 5]]);
    let tx0 = u16::from_be_bytes([obj[t + 6], obj[t + 7]]);
    let center_y = ty0.wrapping_add(half_extent(obj[t + 0xc], t_big));
    let center_x = tx0.wrapping_add(half_extent(obj[t + 0xd], t_big));

    for k in 0..12usize {
        let s = 0x100 + 0x40 * k;
        if obj[s + 8] != 2 || (obj[s + 0xe] & 0x40) == 0 {
            continue;
        }
        let s_attr = u16::from_be_bytes([obj[s + 2], obj[s + 3]]);
        let half = if s_attr & 0x400 != 0 { 0x10u16 } else { 8 };
        let sy = u16::from_be_bytes([obj[s + 4], obj[s + 5]]);
        let sx = u16::from_be_bytes([obj[s + 6], obj[s + 7]]);
        let (cx, cy) = (sx.wrapping_add(half), sy.wrapping_add(half));

        let ty = center_y.wrapping_add((obj[t + 0x22] as i8 as i16 as u16).wrapping_neg());
        if (obj[t + 0x23] as u16) + (obj[s + 0xc] as u16) <= borrow_abs(ty, cy) {
            continue;
        }
        let tx = center_x.wrapping_add(obj[t + 0x24] as i8 as i16 as u16);
        if (obj[t + 0x25] as u16) + (obj[s + 0xd] as u16) <= borrow_abs(tx, cx) {
            continue;
        }
        return true;
    }
    false
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
        assert_eq!(hp(&end_work, 0x06b0), 48, "{}: Ryu HP at end", s.id);
        assert_eq!(hp(&end_work, 0x4664), 20, "{}: Retsu HP at end", s.id);
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
fn stage_0_one_player_tables_are_exercised_and_logged() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    assert_eq!(rom[0x4340a], 8, "stage 0 Hadoken chip offset");
    assert_eq!(rom[0x434d0], 0, "stage 0 Hadoken vs attack-box flag");
    assert_eq!(rom[0x45fa8], 8, "stage 0 Tatsumaki chip offset");

    // 1. sf-1p-hadoken-01: miss, overlap with Retsu's attack box (0x434d0),
    //    unblocked hit + slow motion (event 9), and blocked chip hit (0x4340a).
    let mut hadoken_miss_frames = Vec::new();
    let mut hadoken_attack_box_frames = Vec::new();
    let mut hadoken_hit_frames = Vec::new();
    let mut hadoken_chip_frames = Vec::new();
    let mut prev_retsu_hp = 48u8;
    let mut active_projectile = false;
    let mut projectile_exploded = false;

    let (end_frame, unrec) = observe_cpu_fight(
        Path::new(SCENARIOS[0].ref_a),
        rom.clone(),
        Some(Path::new(SCENARIOS[0].scenario)),
        FIXTURE,
        SCENARIOS[0].last,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            let obj = &m.state.objectram;
            assert_eq!(w[0x4003], 0, "stage 0 throughout sf-1p-hadoken-01");

            if projectile_overlaps_retsu_attack_box(w, obj) {
                hadoken_attack_box_frames.push(f);
            }

            let has_proj = (0..12usize).any(|k| {
                let s = 0x100 + 0x40 * k;
                obj[s + 8] != 0 && (obj[s + 0xe] & 0x40) != 0
            });
            let has_explosion = (0..12usize).any(|k| {
                let s = 0x100 + 0x40 * k;
                let code = u16::from_be_bytes([obj[s], obj[s + 1]]);
                obj[s + 8] != 0 && (code == 0x1fcc || code == 0x1fce)
            });
            if !active_projectile && has_proj {
                active_projectile = true;
                projectile_exploded = false;
            }
            let cur_hp = hp(w, 0x4664);
            if cur_hp < prev_retsu_hp && has_explosion {
                let dmg = prev_retsu_hp - cur_hp;
                projectile_exploded = true;
                active_projectile = false;
                if dmg == 20 {
                    hadoken_hit_frames.push(f);
                } else if dmg == 8 {
                    hadoken_chip_frames.push(f);
                }
                prev_retsu_hp = cur_hp;
            }
            if active_projectile && !has_proj && !projectile_exploded {
                hadoken_miss_frames.push(f);
                active_projectile = false;
            }
        },
    )
    .unwrap();
    assert_eq!((end_frame, unrec), (SCENARIOS[0].last, Vec::<u32>::new()));
    println!(
        "[sf-1p-hadoken-01] stage=0: miss={hadoken_miss_frames:?}, 0x434d0_read={hadoken_attack_box_frames:?}, unblocked_hit={hadoken_hit_frames:?}, 0x4340a_read={hadoken_chip_frames:?}"
    );
    assert_eq!(hadoken_miss_frames, vec![2475]);
    assert_eq!(hadoken_attack_box_frames, vec![2549]);
    assert_eq!(hadoken_hit_frames, vec![2550]);
    assert_eq!(hadoken_chip_frames, vec![2697]);

    // 2. sf-1p-tatsumaki-01: unblocked hit and blocked chip hit (0x45fa8).
    let mut tatsu_hit_frames = Vec::new();
    let mut tatsu_chip_frames = Vec::new();
    let mut prev_retsu_hp = 48u8;
    let (end_frame, unrec) = observe_cpu_fight(
        Path::new(SCENARIOS[1].ref_a),
        rom,
        Some(Path::new(SCENARIOS[1].scenario)),
        FIXTURE,
        SCENARIOS[1].last,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            assert_eq!(w[0x4003], 0, "stage 0 throughout sf-1p-tatsumaki-01");
            let cur_hp = hp(w, 0x4664);
            if cur_hp < prev_retsu_hp {
                let dmg = prev_retsu_hp - cur_hp;
                if dmg == 20 {
                    tatsu_hit_frames.push(f);
                } else if dmg == 8 {
                    tatsu_chip_frames.push(f);
                }
                prev_retsu_hp = cur_hp;
            }
        },
    )
    .unwrap();
    assert_eq!((end_frame, unrec), (SCENARIOS[1].last, Vec::<u32>::new()));
    println!(
        "[sf-1p-tatsumaki-01] stage=0: unblocked_hit={tatsu_hit_frames:?}, 0x45fa8_read={tatsu_chip_frames:?}"
    );
    assert_eq!(tatsu_hit_frames, vec![2438]);
    assert_eq!(tatsu_chip_frames, vec![2564]);
}

#[test]
fn negative_control_stage_2_chip_and_attack_box_entries_fail_at_first_read() {
    if !prerequisites_present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    // Stage 2's chip-table entry (offset 0x10 -> damage 4) differs from stage 0's
    // (offset 0x08 -> damage 8) in both 0x4340a (Hadoken) and 0x45fa8 (Tatsumaki).
    assert_ne!(rom[0x4340a], rom[0x4340a + 2]);
    assert_ne!(rom[0x45fa8], rom[0x45fa8 + 2]);
    assert_ne!(rom[0x434d0], rom[0x434d0 + 2]);

    // 1. Hadoken chip table 0x4340a mutated to stage 2's byte fails at frame 2697
    //    (the first blocked Hadoken chip hit).
    let mut bad_hadoken_assets =
        sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    bad_hadoken_assets.maincpu[0x4340a] = rom[0x4340a + 2];
    let out_hadoken = ScratchDir::new("sf-engine-neg-1p-hadoken");
    run_cpu_fight_capture_with_inputs(
        Path::new(SCENARIOS[0].ref_a),
        &bad_hadoken_assets,
        &out_hadoken,
        SCENARIOS[0].id,
        Some(Path::new(SCENARIOS[0].scenario)),
        FIXTURE,
        SCENARIOS[0].last,
    )
    .expect("mutated Hadoken run finishes");
    let hadoken_ram = compare_cpu_ram_with_inputs(
        Path::new(SCENARIOS[0].ref_a),
        bad_hadoken_assets.maincpu.clone(),
        Some(Path::new(SCENARIOS[0].scenario)),
        FIXTURE,
        SCENARIOS[0].last,
    )
    .unwrap();
    let CpuRamOutcome::Mismatch(msg) = hadoken_ram else {
        panic!("expected RAM mismatch at frame 2697, got {hadoken_ram:?}");
    };
    assert!(
        msg.starts_with("frame 2697:"),
        "expected failure at first Hadoken chip hit (2697), got: {msg}"
    );
    let scenario = load_scenario(SCENARIOS[0].scenario).unwrap();
    let obs_outcome = compare(
        &scenario,
        &load_capture(SCENARIOS[0].ref_a).unwrap(),
        &load_capture(&out_hadoken).unwrap(),
        CandidateKind::Native,
        &native_scope(SCENARIOS[0].last),
    )
    .unwrap();
    let Outcome::Mismatch(div) = obs_outcome else {
        panic!("expected observable divergence at 2697, got {obs_outcome:?}");
    };
    assert_eq!(div.frame, 2697);

    // 2. Hadoken attack-box table 0x434d0 mutated to stage 2's byte (1) fails at
    //    frame 2549 (where projectile 2 overlaps Retsu's attack box).
    let mut bad_atk_rom = rom.clone();
    bad_atk_rom[0x434d0] = rom[0x434d0 + 2];
    let atk_ram = compare_cpu_ram_with_inputs(
        Path::new(SCENARIOS[0].ref_a),
        bad_atk_rom,
        Some(Path::new(SCENARIOS[0].scenario)),
        FIXTURE,
        SCENARIOS[0].last,
    )
    .unwrap();
    let CpuRamOutcome::Mismatch(atk_msg) = atk_ram else {
        panic!("expected RAM mismatch at frame 2549, got {atk_ram:?}");
    };
    assert!(
        atk_msg.starts_with("frame 2549:"),
        "expected failure at attack-box overlap (2549), got: {atk_msg}"
    );

    // 3. Tatsumaki chip table 0x45fa8 mutated to stage 2's byte fails at frame 2564
    //    (the first blocked Tatsumaki chip hit).
    let mut bad_tatsu_assets = assets;
    bad_tatsu_assets.maincpu[0x45fa8] = rom[0x45fa8 + 2];
    let out_tatsu = ScratchDir::new("sf-engine-neg-1p-tatsumaki");
    run_cpu_fight_capture_with_inputs(
        Path::new(SCENARIOS[1].ref_a),
        &bad_tatsu_assets,
        &out_tatsu,
        SCENARIOS[1].id,
        Some(Path::new(SCENARIOS[1].scenario)),
        FIXTURE,
        SCENARIOS[1].last,
    )
    .expect("mutated Tatsumaki run finishes");
    let tatsu_ram = compare_cpu_ram_with_inputs(
        Path::new(SCENARIOS[1].ref_a),
        bad_tatsu_assets.maincpu.clone(),
        Some(Path::new(SCENARIOS[1].scenario)),
        FIXTURE,
        SCENARIOS[1].last,
    )
    .unwrap();
    let CpuRamOutcome::Mismatch(tatsu_msg) = tatsu_ram else {
        panic!("expected RAM mismatch at frame 2564, got {tatsu_ram:?}");
    };
    assert!(
        tatsu_msg.starts_with("frame 2564:"),
        "expected failure at first Tatsumaki chip hit (2564), got: {tatsu_msg}"
    );
    let tatsu_scenario = load_scenario(SCENARIOS[1].scenario).unwrap();
    let tatsu_obs = compare(
        &tatsu_scenario,
        &load_capture(SCENARIOS[1].ref_a).unwrap(),
        &load_capture(&out_tatsu).unwrap(),
        CandidateKind::Native,
        &native_scope(SCENARIOS[1].last),
    )
    .unwrap();
    let Outcome::Mismatch(tatsu_div) = tatsu_obs else {
        panic!("expected observable divergence at 2564, got {tatsu_obs:?}");
    };
    assert_eq!(tatsu_div.frame, 2564);
}
