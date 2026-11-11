//! Integration tests for Ticket `#94` (`O3: MatchConfig, RulesetPreset, and Positive-Edge Input + Recovery Buffer Core Toggles`).

use std::path::PathBuf;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::config::{CoreToggles, MatchConfig, RulesetPreset, DEFAULT_RECOVERY_BUFFER_WINDOW};
use sf_engine::fight::FightMachine;
use sf_engine::fighter::{OBJ_BASE, P1_STRUCT, P2_STRUCT};
use sf_engine::run::{
    compare_fight_ram_with_inputs, run_fight_capture, run_punch_capture, CpuRamOutcome, ScratchDir,
};
use sf_video::romset::{load_assets, load_program_rom};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_rom() -> Vec<u8> {
    let root = repo_root();
    load_program_rom(
        &root.join("tools/reference/local/roms/sf"),
        &root.join("tools/reference/rom-manifest.json"),
    )
    .expect("local ROM set")
}

fn versus_machine_with_config(config: MatchConfig) -> FightMachine {
    let root = repo_root();
    let capture = root.join("tools/reference/local/captures/move-run-a");
    FightMachine::from_checkpoint_2450_with_program(&capture, load_rom())
        .expect("checkpoint 2450 fixture")
        .with_match_config(config)
}

fn p1_entity(m: &FightMachine) -> usize {
    let w = m.work_ram();
    let off = (P1_STRUCT - 0xff8000) as usize + 0x10;
    let entity = u16::from_be_bytes([w[off], w[off + 1]]);
    (OBJ_BASE + entity as u32 - 0xffe000) as usize
}

fn p2_entity(m: &FightMachine) -> usize {
    let w = m.work_ram();
    let off = (P2_STRUCT - 0xff8000) as usize + 0x10;
    let entity = u16::from_be_bytes([w[off], w[off + 1]]);
    (OBJ_BASE + entity as u32 - 0xffe000) as usize
}

fn p1_move_id(m: &FightMachine) -> u8 {
    m.state.objectram[p1_entity(m) + 0x0a]
}

fn p2_move_id(m: &FightMachine) -> u8 {
    m.state.objectram[p2_entity(m) + 0x0a]
}

fn p2_state_byte(m: &FightMachine) -> u8 {
    m.state.objectram[p2_entity(m) + 0x09]
}

#[test]
fn five_ruleset_presets_and_toml_persistence_round_trip() {
    // Default MatchConfig once the faithful port is complete is Definitive Overhaul.
    let default_cfg = MatchConfig::default();
    assert_eq!(default_cfg.preset, RulesetPreset::DefinitiveOverhaul);
    assert!(default_cfg.toggles.positive_edge_attacks);
    assert!(default_cfg.toggles.negative_edge_specials);
    assert!(default_cfg.toggles.recovery_input_buffer);
    assert_eq!(
        default_cfg.toggles.recovery_buffer_window,
        DEFAULT_RECOVERY_BUFFER_WINDOW
    );

    // Faithful 1987 disables all overhaul toggles.
    let faithful_cfg = MatchConfig::from_preset(RulesetPreset::Faithful1987);
    assert_eq!(faithful_cfg.preset, RulesetPreset::Faithful1987);
    assert!(!faithful_cfg.toggles.positive_edge_attacks);
    assert!(!faithful_cfg.toggles.negative_edge_specials);
    assert!(!faithful_cfg.toggles.recovery_input_buffer);
    assert_eq!(faithful_cfg.toggles, CoreToggles::faithful());

    // Pure Footsies (1987+) enables responsive input/walk/block toggles while keeping command trackers strict.
    let footsies_cfg = MatchConfig::from_preset(RulesetPreset::PureFootsies1987Plus);
    assert!(footsies_cfg.toggles.positive_edge_attacks);
    assert!(footsies_cfg.toggles.recovery_input_buffer);
    assert!(!footsies_cfg.toggles.lenient_command_trackers);

    // All 5 presets round-trip through TOML files on disk.
    let scratch = ScratchDir::new("match-config-presets");
    for preset in RulesetPreset::ALL {
        let cfg = MatchConfig::from_preset(preset);
        let path = scratch.join(format!("{}.toml", preset.slug()));
        cfg.save(&path).expect("save preset TOML");
        let loaded = MatchConfig::load(&path).expect("load preset TOML");
        assert_eq!(loaded, cfg, "preset {:?} round-trip mismatch", preset);
    }

    // Overriding an individual toggle on a standard preset transitions to Custom and persists.
    let mut custom_cfg = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    custom_cfg.set_negative_edge_specials(false);
    custom_cfg.set_recovery_buffer_window(6);
    assert_eq!(custom_cfg.preset, RulesetPreset::Custom);
    let custom_path = scratch.join("custom_override.toml");
    custom_cfg.save(&custom_path).expect("save custom TOML");
    let (reloaded, diag) = MatchConfig::load_or_default(&custom_path);
    assert!(diag.is_none());
    assert_eq!(reloaded.preset, RulesetPreset::Custom);
    assert!(reloaded.toggles.positive_edge_attacks);
    assert!(!reloaded.toggles.negative_edge_specials);
    assert!(reloaded.toggles.recovery_input_buffer);
    assert_eq!(reloaded.toggles.recovery_buffer_window, 6);
}

#[test]
fn positive_edge_attacks_paired_differential_normal_and_255_frame_hold() {
    let cfg_false = MatchConfig::faithful();
    let cfg_true = MatchConfig::faithful()
        .with_positive_edge_attacks(true)
        .with_negative_edge_specials(true);

    let mut m_false = versus_machine_with_config(cfg_false);
    let mut m_true = versus_machine_with_config(cfg_true);

    // Part 1: Normal attack on button press (F+1) vs release (release + 1).
    // Hold P1 LP over frames 2451..=2460 (10 frames), release at frame 2461.
    for step_idx in 1..=25 {
        let lp_held = (1..=10).contains(&step_idx);
        m_false.set_p1_buttons([lp_held, false, false, false, false, false]);
        m_true.set_p1_buttons([lp_held, false, false, false, false, false]);
        m_false.step();
        m_true.step();

        if step_idx == 1 {
            // Frame F+1 (2451): positive_edge_attacks=true starts Standing LP (0x0d) immediately;
            // positive_edge_attacks=false is still waiting for button release (move 1).
            assert_eq!(
                p1_move_id(&m_true),
                0x0d,
                "positive_edge_attacks=true must start Standing LP on press frame F+1"
            );
            assert_eq!(
                p1_move_id(&m_false),
                0x01,
                "positive_edge_attacks=false must stay in stance while LP is held"
            );
        }
        if step_idx == 11 {
            // Frame 2461 (first frame after release):
            // positive_edge_attacks=false starts Standing LP (0x0d) now;
            // positive_edge_attacks=true already finished its LP and must NOT re-trigger a normal on release.
            assert_eq!(
                p1_move_id(&m_false),
                0x0d,
                "positive_edge_attacks=false starts Standing LP on release + 1"
            );
            assert_eq!(
                p1_move_id(&m_true),
                0x01,
                "positive_edge_attacks=true must not trigger a duplicate normal attack on button release"
            );
        }
    }

    // Part 2: Holding a button past 255 frames never locks out subsequent releases or presses.
    let mut m_hold_false = versus_machine_with_config(cfg_false);
    let mut m_hold_true = versus_machine_with_config(cfg_true);

    // Hold LP for 270 frames (> 255 frames).
    for _ in 0..270 {
        m_hold_false.set_p1_buttons([true, false, false, false, false, false]);
        m_hold_true.set_p1_buttons([true, false, false, false, false, false]);
        m_hold_false.step();
        m_hold_true.step();
    }
    // While still holding LP past 270 frames, input QCF (Down, Down-Forward, Forward) and release LP on Forward.
    for (r, d, lp) in [
        (false, true, true),  // Down (LP still held: frame 271)
        (true, true, true),   // Down-Forward (LP still held: frame 272)
        (true, false, false), // Forward + release LP (frame 273)
    ] {
        m_hold_false.set_p1_direction(r, false, d, false);
        m_hold_false.set_p1_buttons([lp, false, false, false, false, false]);
        m_hold_true.set_p1_direction(r, false, d, false);
        m_hold_true.set_p1_buttons([lp, false, false, false, false, false]);
        m_hold_false.step();
        m_hold_true.step();
    }

    assert_ne!(
        p1_move_id(&m_hold_false),
        0x1a,
        "Faithful 1987 saturates at 255 frames and locks out the release"
    );
    assert_eq!(
        p1_move_id(&m_hold_true),
        0x1a,
        "positive_edge_attacks + negative_edge_specials must never saturate at 255 frames and must fire Hadoken on release"
    );
}

#[test]
fn positive_edge_and_negative_edge_specials_sub_toggle_paired_differential() {
    // 1. Positive-edge special: QCF + press & hold HP fires Hadoken (0x1a) on press frame (F+1)
    // when positive_edge_attacks=true, vs waiting for release when positive_edge_attacks=false.
    let mut m_faithful = versus_machine_with_config(MatchConfig::faithful());
    let mut m_pos_only = versus_machine_with_config(
        MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_negative_edge_specials(false),
    );

    let qcf_press_hold = [
        (false, true, false), // Down
        (true, true, false),  // Down-Forward
        (true, false, true),  // Forward + press HP (keep held)
    ];
    for (r, d, hp) in qcf_press_hold {
        m_faithful.set_p1_direction(r, false, d, false);
        m_faithful.set_p1_buttons([false, false, hp, false, false, false]);
        m_pos_only.set_p1_direction(r, false, d, false);
        m_pos_only.set_p1_buttons([false, false, hp, false, false, false]);
        m_faithful.step();
        m_pos_only.step();
    }
    assert_eq!(
        p1_move_id(&m_pos_only),
        0x1a,
        "positive_edge_attacks=true must trigger Hadoken (0x1a) on the button press frame (F+1)"
    );
    assert_ne!(
        p1_move_id(&m_faithful),
        0x1a,
        "positive_edge_attacks=false must not trigger Hadoken while HP is still held"
    );

    // 2. Negative-edge special sub-toggle: hold HP through initial punch recovery, then input QCF and release HP.
    let mut m_neg_off = versus_machine_with_config(
        MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_negative_edge_specials(false),
    );
    let mut m_neg_on = versus_machine_with_config(
        MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_negative_edge_specials(true),
    );

    // Press and holdLP for 15 frames so the initial standing LP finishes while LP remains held.
    for _ in 0..15 {
        m_neg_off.set_p1_buttons([true, false, false, false, false, false]);
        m_neg_on.set_p1_buttons([true, false, false, false, false, false]);
        m_neg_off.step();
        m_neg_on.step();
    }
    assert_eq!(p1_move_id(&m_neg_off), 0x01);
    assert_eq!(p1_move_id(&m_neg_on), 0x01);

    // Input QCF while holding LP, then release LP on Forward.
    for (r, d, lp) in [
        (false, true, true),  // Down
        (true, true, true),   // Down-Forward
        (true, false, false), // Forward + release LP
    ] {
        m_neg_off.set_p1_direction(r, false, d, false);
        m_neg_off.set_p1_buttons([lp, false, false, false, false, false]);
        m_neg_on.set_p1_direction(r, false, d, false);
        m_neg_on.set_p1_buttons([lp, false, false, false, false, false]);
        m_neg_off.step();
        m_neg_on.step();
    }

    assert_ne!(
        p1_move_id(&m_neg_off),
        0x1a,
        "negative_edge_specials=false must not trigger Hadoken on button release"
    );
    assert_eq!(
        p1_move_id(&m_neg_on),
        0x1a,
        "negative_edge_specials=true must trigger Hadoken on button release"
    );
}

#[test]
fn recovery_input_buffer_paired_differential_across_attack_hitstun_and_blockstun() {
    let cfg_unbuffered = MatchConfig::faithful()
        .with_positive_edge_attacks(true)
        .with_negative_edge_specials(true)
        .with_recovery_input_buffer(false);
    let cfg_buffered = MatchConfig::faithful()
        .with_positive_edge_attacks(true)
        .with_negative_edge_specials(true)
        .with_recovery_input_buffer(true)
        .with_recovery_buffer_window(DEFAULT_RECOVERY_BUFFER_WINDOW);

    // ---- 1. Attack Recovery (normal & special buffered inside 4-frame window vs outside) ----
    {
        let mut m_off = versus_machine_with_config(cfg_unbuffered);
        let mut m_on = versus_machine_with_config(cfg_buffered);

        // Frame 2451: press LP to start Standing LP (0x0d, busy for 7 frames 2451..=2457, first actionable = 2458).
        m_off.set_p1_buttons([true, false, false, false, false, false]);
        m_on.set_p1_buttons([true, false, false, false, false, false]);
        m_off.step();
        m_on.step();
        assert_eq!(p1_move_id(&m_on), 0x0d);

        // Advance through frames 2452..=2457. Input QCF + LP on frames 2454..=2456 (2 frames before 2458 actionable).
        for f in 2452..=2457 {
            let (r, d, lp) = match f {
                2454 => (false, true, false), // Down
                2455 => (true, true, false),  // Down-Forward
                2456 => (true, false, true),  // Forward + LP (2 frames before recovery ends!)
                _ => (false, false, false),
            };
            m_off.set_p1_direction(r, false, d, false);
            m_off.set_p1_buttons([lp, false, false, false, false, false]);
            m_on.set_p1_direction(r, false, d, false);
            m_on.set_p1_buttons([lp, false, false, false, false, false]);
            m_off.step();
            m_on.step();
        }

        // Frame 2458: first actionable frame after Standing LP recovery ends.
        m_off.set_p1_direction(false, false, false, false);
        m_off.set_p1_buttons([false; 6]);
        m_on.set_p1_direction(false, false, false, false);
        m_on.set_p1_buttons([false; 6]);
        m_off.step();
        m_on.step();

        assert_eq!(
            p1_move_id(&m_off),
            0x01,
            "recovery_input_buffer=false must clear inputs entered during attack recovery"
        );
        assert_eq!(
            p1_move_id(&m_on),
            0x1a,
            "recovery_input_buffer=true must execute the buffered Hadoken on the first actionable frame"
        );

        // Verify window boundary: an input 5 frames before actionable expires with window=4,
        // but succeeds when recovery_buffer_window=6.
        let mut m_win4 = versus_machine_with_config(cfg_buffered.with_recovery_buffer_window(4));
        let mut m_win6 = versus_machine_with_config(cfg_buffered.with_recovery_buffer_window(6));
        m_win4.set_p1_buttons([true, false, false, false, false, false]);
        m_win6.set_p1_buttons([true, false, false, false, false, false]);
        m_win4.step(); // 2451: start Standing LP (actionable at 2458)
        m_win6.step();
        for f in 2452..=2457 {
            // Frame 2453 is 5 frames before 2458 (2453, 2454, 2455, 2456, 2457 = 5 recovery frames).
            let lk = f == 2453;
            m_win4.set_p1_buttons([false, false, false, lk, false, false]);
            m_win6.set_p1_buttons([false, false, false, lk, false, false]);
            m_win4.step();
            m_win6.step();
        }
        m_win4.set_p1_buttons([false; 6]);
        m_win6.set_p1_buttons([false; 6]);
        m_win4.step(); // 2458
        m_win6.step();
        assert_eq!(
            p1_move_id(&m_win4),
            0x01,
            "input 5 frames before recovery ends must expire under a 4-frame window"
        );
        assert_eq!(
            p1_move_id(&m_win6),
            0x14,
            "input 5 frames before recovery ends must execute under a 6-frame window (close standing LK = 0x14)"
        );
    }

    // ---- 2. Hitstun Recovery (reversal buffered within 4 frames before hitstun ends) ----
    {
        let mut base = versus_machine_with_config(cfg_buffered);
        // Walk P1 right for 36 frames (two 18-frame walk cycles) to get within hit range of P2 (Ken).
        for _ in 0..36 {
            base.set_p1_direction(true, false, false, false);
            base.step();
        }
        base.set_p1_direction(false, false, false, false);
        // P1 presses LP to hit P2 into hitstun (move 0x30, state 0x8a).
        base.set_p1_buttons([true, false, false, false, false, false]);
        base.step();
        base.set_p1_buttons([false; 6]);
        while p2_move_id(&base) != 0x30 {
            base.step();
        }
        // Snapshot at the start of P2's hitstun to find the exact frame P2 becomes actionable.
        let hitstun_start = base.save_snapshot();
        while (p2_state_byte(&base) & 0x80) != 0 {
            base.step();
        }
        // `base.state.frame` is the last frame where P2's busy bit cleared at end_move;
        // `first_actionable_frame = base.state.frame + 1`.
        let first_actionable = base.state.frame + 1;
        let buffer_input_frame = first_actionable - 2; // 2 frames before hitstun ends

        let mut m_off = versus_machine_with_config(cfg_unbuffered);
        let mut m_on = versus_machine_with_config(cfg_buffered);
        m_off.restore_snapshot(&hitstun_start);
        m_off.set_match_config(cfg_unbuffered);
        m_on.restore_snapshot(&hitstun_start);
        m_on.set_match_config(cfg_buffered);

        while m_on.state.frame < first_actionable {
            let next_f = m_on.state.frame + 1;
            let press_p2_lp = next_f == buffer_input_frame;
            m_off.set_p2_buttons([press_p2_lp, false, false, false, false, false]);
            m_on.set_p2_buttons([press_p2_lp, false, false, false, false, false]);
            m_off.step();
            m_on.step();
        }

        assert_eq!(
            p2_move_id(&m_off),
            0x01,
            "recovery_input_buffer=false must clear P2's attack pressed during hitstun"
        );
        assert_eq!(
            p2_move_id(&m_on),
            0x0d,
            "recovery_input_buffer=true must execute P2's buffered reversal LP on the first actionable frame out of hitstun"
        );
    }

    // ---- 3. Blockstun Recovery (reversal buffered within 4 frames before blockstun ends) ----
    {
        let mut base = versus_machine_with_config(cfg_buffered);
        // Walk P1 right for 36 frames so the Hadoken impacts P2's standing guard immediately.
        for _ in 0..36 {
            base.set_p1_direction(true, false, false, false);
            base.step();
        }
        // P1 inputs QCF + HP to fire a Hadoken while P2 stays neutral until P1's attack starts (state 0xc2).
        for (r, d, hp) in [
            (false, true, false),
            (true, true, false),
            (true, false, true),
        ] {
            base.set_p1_direction(r, false, d, false);
            base.set_p1_buttons([false, false, hp, false, false, false]);
            base.step();
        }
        assert_eq!(p1_move_id(&base), 0x1a);
        base.set_p1_direction(false, false, false, false);
        base.set_p1_buttons([false; 6]);
        // Now P1 is in state 0xc2; P2 holds Back (Right) to enter standing block (move 4, state 0x22)
        // and block the fireball into chip blockstun (state & 0x80 != 0).
        while (p2_state_byte(&base) & 0x80) == 0 {
            base.set_p2_direction(true, false, false, false);
            base.step();
        }
        let blockstun_start = base.save_snapshot();
        base.set_p2_direction(false, false, false, false);
        while (p2_state_byte(&base) & 0x80) != 0 {
            base.step();
        }
        let first_actionable = base.state.frame + 1;
        let buffer_input_frame = first_actionable - 3; // 3 frames before blockstun ends

        let mut m_off = versus_machine_with_config(cfg_unbuffered);
        let mut m_on = versus_machine_with_config(cfg_buffered);
        m_off.restore_snapshot(&blockstun_start);
        m_off.set_match_config(cfg_unbuffered);
        m_on.restore_snapshot(&blockstun_start);
        m_on.set_match_config(cfg_buffered);

        while m_on.state.frame < first_actionable {
            let next_f = m_on.state.frame + 1;
            let press_p2_lk = next_f == buffer_input_frame;
            m_off.set_p2_direction(false, false, false, false);
            m_off.set_p2_buttons([false, false, false, press_p2_lk, false, false]);
            m_on.set_p2_direction(false, false, false, false);
            m_on.set_p2_buttons([false, false, false, press_p2_lk, false, false]);
            m_off.step();
            m_on.step();
        }

        assert_eq!(
            p2_move_id(&m_off),
            0x01,
            "recovery_input_buffer=false must clear P2's attack pressed during blockstun"
        );
        assert_eq!(
            p2_move_id(&m_on),
            0x14,
            "recovery_input_buffer=true must execute P2's buffered LK on the first actionable frame out of blockstun"
        );
    }
}

#[test]
fn faithful_1987_preset_preserves_zero_mame_diffs() {
    let root = repo_root();
    let roms = root.join("tools/reference/local/roms/sf");
    let manifest = root.join("tools/reference/rom-manifest.json");
    let assets = load_assets(&roms, &manifest).expect("local ROM set");

    // 1. Versus sf-punch-01 full pixel/frame/RAM/audio latch comparison under Faithful 1987.
    let fixture = root.join("tools/reference/local/captures/move-run-a");
    let reference = root.join("tools/reference/local/captures/punch-01-run-a");
    let scenario_path = root.join("tools/reference/scenarios/sf_punch_01.json");
    let native_dir = ScratchDir::new("faithful-1987-punch-01");
    run_punch_capture(
        &fixture,
        &assets,
        &native_dir,
        "sf-punch-01",
        &scenario_path,
    )
    .expect("run_punch_capture");
    let scen = load_scenario(&scenario_path).expect("load sf_punch_01");
    let ref_cap = load_capture(&reference).expect("load ref capture");
    let cand_cap = load_capture(&native_dir).expect("load cand capture");
    let fields = CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap();
    let scope = CompareScope::full()
        .with_range(2451, 2599)
        .with_fields(fields.clone());
    let outcome = compare(&scen, &ref_cap, &cand_cap, CandidateKind::Native, &scope)
        .expect("compare sf-punch-01");
    assert!(
        matches!(outcome, Outcome::Match { .. }),
        "Faithful 1987 preset produced diffs on sf-punch-01: {outcome:?}"
    );

    // 2. 1P sf-1p-held-01 (exercises 276-frame button hold saturation) under Faithful 1987.
    let cpu_ref = root.join("tools/reference/local/captures/1p-held-01-run-a");
    let cpu_scen_path = root.join("tools/reference/scenarios/sf_1p_held_01.json");
    let outcome = compare_fight_ram_with_inputs(
        &cpu_ref,
        assets.maincpu.clone(),
        Some(&cpu_scen_path),
        2389,
        2680,
    )
    .expect("compare_fight_ram_with_inputs");
    assert_eq!(outcome, CpuRamOutcome::Match { frames: 291 });

    let held_dir = ScratchDir::new("faithful-1987-1p-held-01");
    run_fight_capture(
        &cpu_ref,
        &assets,
        &held_dir,
        "sf-1p-held-01",
        Some(&cpu_scen_path),
        2389,
        2680,
    )
    .expect("run_fight_capture");
    let cpu_scen = load_scenario(&cpu_scen_path).expect("load sf_1p_held_01");
    let held_ref = load_capture(&cpu_ref).expect("load held ref");
    let held_cand = load_capture(&held_dir).expect("load held cand");
    let held_scope = CompareScope::full()
        .with_range(2390, 2680)
        .with_fields(fields);
    let held_outcome = compare(
        &cpu_scen,
        &held_ref,
        &held_cand,
        CandidateKind::Native,
        &held_scope,
    )
    .expect("compare sf-1p-held-01");
    assert!(
        matches!(held_outcome, Outcome::Match { .. }),
        "Faithful 1987 preset produced diffs on sf-1p-held-01: {held_outcome:?}"
    );
}
