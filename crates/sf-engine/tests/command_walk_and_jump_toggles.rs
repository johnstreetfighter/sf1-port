//! Integration tests for Ticket `#95` (`O4: Lenient Special Command Trackers, Walk-Gate Interrupts, and Anytime Jump Attacks`).

use std::path::PathBuf;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::config::{
    MatchConfig, DEFAULT_COMMAND_RELEASE_GRACE, DEFAULT_COMMAND_TRACKER_WINDOW,
};
use sf_engine::fight::FightMachine;
use sf_engine::fighter::{OBJ_BASE, P1_STRUCT, P2_STRUCT};
use sf_engine::run::{compare_versus_ram, run_versus_interval_capture, ScratchDir};
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

fn p1_move_id(m: &FightMachine) -> u8 {
    m.state.objectram[p1_entity(m) + 0x0a]
}

fn p1_state_byte(m: &FightMachine) -> u8 {
    m.state.objectram[p1_entity(m) + 0x09]
}

fn p1_sy(m: &FightMachine) -> u16 {
    let e = p1_entity(m);
    u16::from_be_bytes([m.state.objectram[e + 4], m.state.objectram[e + 5]])
}

fn p1_sx(m: &FightMachine) -> u16 {
    let e = p1_entity(m);
    u16::from_be_bytes([m.state.objectram[e + 6], m.state.objectram[e + 7]])
}

fn p2_health(m: &FightMachine) -> u16 {
    let w = m.work_ram();
    let off = (P2_STRUCT - 0xff8000) as usize;
    4 * w[off + 7] as u16 + w[off + 6] as u16
}

#[test]
fn lenient_command_trackers_paired_differential_motion_window_release_grace_and_dp_shortcuts() {
    let def_cfg = MatchConfig::definitive();
    assert!(def_cfg.toggles.lenient_command_trackers);
    assert_eq!(
        def_cfg.toggles.command_tracker_window,
        DEFAULT_COMMAND_TRACKER_WINDOW
    );
    assert_eq!(
        def_cfg.toggles.command_release_grace,
        DEFAULT_COMMAND_RELEASE_GRACE
    );

    let run_special_sequence =
        |lenient: bool, steps: &[((bool, bool, bool, bool), [bool; 6])]| -> u8 {
            let cfg = MatchConfig::faithful()
                .with_positive_edge_attacks(true)
                .with_interruptible_walk_cycle(true)
                .with_lenient_command_trackers(lenient);
            let mut m = versus_machine_with_config(cfg);
            for &((r, l, d, u), btns) in steps {
                m.set_p1_direction(r, l, d, u);
                m.set_p1_buttons(btns);
                m.step();
            }
            p1_move_id(&m)
        };

    let idle = [false; 6];
    let lp = [true, false, false, false, false, false];
    let lk = [false, false, false, true, false, false];

    // 1. 24-frame motion window (> 18-frame 1987 limit, <= 28-frame lenient window):
    //    - Hadoken: Down (1f), Neutral (11f), Down-Forward (1f), Neutral (10f), Forward + LP (1f) = 24f
    let mut slow_hadoken = Vec::new();
    slow_hadoken.push(((false, false, true, false), idle));
    for _ in 0..11 {
        slow_hadoken.push(((false, false, false, false), idle));
    }
    slow_hadoken.push(((true, false, true, false), idle));
    for _ in 0..10 {
        slow_hadoken.push(((false, false, false, false), idle));
    }
    slow_hadoken.push(((true, false, false, false), lp));
    assert_eq!(
        run_special_sequence(false, &slow_hadoken),
        0x0d,
        "Faithful 1987 must expire Hadoken tracker after 18 frames and fire standing LP (0x0d)"
    );
    assert_eq!(
        run_special_sequence(true, &slow_hadoken),
        0x1a,
        "lenient_command_trackers must accept a 24-frame Hadoken motion within the 28-frame window"
    );

    //    - Shoryuken: Forward (1f), Neutral (11f), Down (1f), Neutral (10f), Down-Forward + LP (1f) = 24f
    let mut slow_dp = Vec::new();
    slow_dp.push(((true, false, false, false), idle));
    for _ in 0..11 {
        slow_dp.push(((false, false, false, false), idle));
    }
    slow_dp.push(((false, false, true, false), idle));
    for _ in 0..10 {
        slow_dp.push(((false, false, false, false), idle));
    }
    slow_dp.push(((true, false, true, false), lp));
    assert_eq!(
        run_special_sequence(false, &slow_dp),
        0x12,
        "Faithful 1987 must expire Shoryuken tracker after 18 frames and fire diagonal crouching LP (0x12)"
    );
    assert_eq!(
        run_special_sequence(true, &slow_dp),
        0x19,
        "lenient_command_trackers must accept a 24-frame Shoryuken motion within the 28-frame window"
    );

    //    - Tatsumaki: Down (1f), Neutral (11f), Down-Back (1f), Neutral (10f), Back + LK (1f) = 24f
    let mut slow_tatsu = Vec::new();
    slow_tatsu.push(((false, false, true, false), idle));
    for _ in 0..11 {
        slow_tatsu.push(((false, false, false, false), idle));
    }
    slow_tatsu.push(((false, true, true, false), idle));
    for _ in 0..10 {
        slow_tatsu.push(((false, false, false, false), idle));
    }
    slow_tatsu.push(((false, true, false, false), lk));
    assert_eq!(
        run_special_sequence(false, &slow_tatsu),
        0x14,
        "Faithful 1987 must expire Tatsumaki tracker after 18 frames and fire standing LK (0x14)"
    );
    assert_eq!(
        run_special_sequence(true, &slow_tatsu),
        0x1b,
        "lenient_command_trackers must accept a 24-frame Tatsumaki motion within the 28-frame window"
    );

    // 2. Directional release grace window (default 8 frames after releasing the final direction):
    //    Complete motion, release stick to neutral for 5 frames, then press LP / LK at neutral.
    let mut grace_hadoken = vec![
        ((false, false, true, false), idle),
        ((true, false, true, false), idle),
        ((true, false, false, false), idle),
    ];
    for _ in 0..5 {
        grace_hadoken.push(((false, false, false, false), idle));
    }
    grace_hadoken.push(((false, false, false, false), lp));
    assert_eq!(
        run_special_sequence(false, &grace_hadoken),
        0x0d,
        "Faithful 1987 clears Hadoken tracker on the first frame Forward is released"
    );
    assert_eq!(
        run_special_sequence(true, &grace_hadoken),
        0x1a,
        "lenient_command_trackers retains completed Hadoken for 8 frames after releasing Forward"
    );

    let mut grace_dp = vec![
        ((true, false, false, false), idle),
        ((false, false, true, false), idle),
        ((true, false, true, false), idle),
    ];
    for _ in 0..5 {
        grace_dp.push(((false, false, false, false), idle));
    }
    grace_dp.push(((false, false, false, false), lp));
    assert_eq!(
        run_special_sequence(false, &grace_dp),
        0x0d,
        "Faithful 1987 clears Shoryuken tracker on the first frame Down-Forward is released"
    );
    assert_eq!(
        run_special_sequence(true, &grace_dp),
        0x19,
        "lenient_command_trackers retains completed Shoryuken for 8 frames after releasing Down-Forward"
    );

    let mut grace_tatsu = vec![
        ((false, false, true, false), idle),
        ((false, true, true, false), idle),
        ((false, true, false, false), idle),
    ];
    for _ in 0..5 {
        grace_tatsu.push(((false, false, false, false), idle));
    }
    grace_tatsu.push(((false, false, false, false), lk));
    assert_eq!(
        run_special_sequence(false, &grace_tatsu),
        0x14,
        "Faithful 1987 clears Tatsumaki tracker on the first frame Back is released"
    );
    assert_eq!(
        run_special_sequence(true, &grace_tatsu),
        0x1b,
        "lenient_command_trackers retains completed Tatsumaki for 8 frames after releasing Back"
    );

    //    Releasing for 10 frames (> 8-frame grace) must expire the grace window and fire a normal:
    let mut expired_grace_hadoken = vec![
        ((false, false, true, false), idle),
        ((true, false, true, false), idle),
        ((true, false, false, false), idle),
    ];
    for _ in 0..10 {
        expired_grace_hadoken.push(((false, false, false, false), idle));
    }
    expired_grace_hadoken.push(((false, false, false, false), lp));
    assert_eq!(
        run_special_sequence(true, &expired_grace_hadoken),
        0x0d,
        "releasing the final direction for more than 8 frames must expire the grace window"
    );

    // 3. Diagonal Shoryuken shortcuts (DF, D, DF and DF, DB, DF):
    let shortcut_df_d_df = vec![
        ((true, false, true, false), idle),  // DF
        ((false, false, true, false), idle), // D
        ((true, false, true, false), lp),    // DF + LP
    ];
    assert_eq!(
        run_special_sequence(false, &shortcut_df_d_df),
        0x12,
        "Faithful 1987 rejects DF, D, DF shortcut and fires diagonal crouching LP (0x12)"
    );
    assert_eq!(
        run_special_sequence(true, &shortcut_df_d_df),
        0x19,
        "lenient_command_trackers accepts DF, D, DF crouching Shoryuken shortcut (0x19)"
    );

    let shortcut_df_db_df = vec![
        ((true, false, true, false), idle), // DF
        ((false, true, true, false), idle), // DB
        ((true, false, true, false), lp),   // DF + LP
    ];
    assert_eq!(
        run_special_sequence(false, &shortcut_df_db_df),
        0x12,
        "Faithful 1987 rejects DF, DB, DF shortcut and fires diagonal crouching LP (0x12)"
    );
    assert_eq!(
        run_special_sequence(true, &shortcut_df_db_df),
        0x19,
        "lenient_command_trackers accepts DF, DB, DF diagonal Shoryuken shortcut (0x19)"
    );

    // Walking forward into QCF (F, D, DF, F + LP) must prioritize Hadoken (0x1a) over Shoryuken:
    let walk_fwd_into_qcf = vec![
        ((false, false, true, false), idle), // D (while neutral)
        ((true, false, true, false), idle),  // DF
        ((true, false, false, false), lp),   // F + LP
    ];
    assert_eq!(run_special_sequence(true, &walk_fwd_into_qcf), 0x1a);
}

#[test]
fn interruptible_walk_cycle_paired_differential_walk_gate_and_back_walk_shift() {
    let idle = [false; 6];
    let lp = [true, false, false, false, false, false];
    let lk = [false, false, false, true, false, false];

    // 1. Interrupting gated 0x91 walk frames with a special move (Hadoken 0x1a):
    let run_walk_into_hadoken = |interruptible: bool| -> (u8, u8) {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_interruptible_walk_cycle(interruptible);
        let mut m = versus_machine_with_config(cfg);
        // Step 3 frames of Walk Forward (move 2) so P1 enters the 8-frame gated 0x91 walk phase
        // (frames 1..=2 hold the initial 0x92 header pose; frame 3 calls 0x45a7a -> 0x91).
        for _ in 0..3 {
            m.set_p1_direction(true, false, false, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        assert_eq!(p1_move_id(&m), 2);
        assert_eq!(
            p1_state_byte(&m),
            0x91,
            "walk cycle must be in gated 0x91 state"
        );

        m.set_p1_direction(false, false, true, false); // D
        m.step();
        m.set_p1_direction(true, false, true, false); // DF
        m.step();
        m.set_p1_direction(true, false, false, false); // F + LP on gated 0x91 walk frame
        m.set_p1_buttons(lp);
        m.step();
        (p1_move_id(&m), p1_state_byte(&m))
    };
    let (mv_false, st_false) = run_walk_into_hadoken(false);
    let (mv_true, _st_true) = run_walk_into_hadoken(true);
    assert_eq!(
        (mv_false, st_false),
        (2, 0x91),
        "Faithful 1987 eats the Hadoken input during a gated 0x91 walk frame"
    );
    assert_eq!(
        mv_true, 0x1a,
        "interruptible_walk_cycle immediately cancels the 0x91 walk frame into Hadoken (0x1a)"
    );

    // 2. Interrupting gated 0x91 walk frames with Crouch (move 5), Block (move 4), and Jump (move 0x0a):
    for (label, dir, p2_atk, expected_mv) in [
        ("crouch", (false, false, true, false), false, 5u8),
        ("standing_block", (false, true, false, false), true, 4u8),
        ("forward_jump", (true, false, false, true), false, 0x0au8),
    ] {
        let run_interrupt = |interruptible: bool| -> u8 {
            let cfg = MatchConfig::faithful()
                .with_positive_edge_attacks(true)
                .with_interruptible_walk_cycle(interruptible);
            let mut m = versus_machine_with_config(cfg);
            // Start P1 walking forward (move 2), and advance 3 frames into gated 0x91 state.
            for _ in 0..3 {
                m.set_p1_direction(true, false, false, false);
                m.set_p1_buttons(idle);
                m.step();
            }
            assert_eq!(p1_state_byte(&m), 0x91);

            if p2_atk {
                // Have P2 start an attack so P1's Back input triggers Standing Block (move 4).
                m.set_p2_direction(false, false, false, false);
                m.set_p2_buttons(lp);
            }
            m.set_p1_direction(dir.0, dir.1, dir.2, dir.3);
            m.step();
            if p2_atk {
                // Step 1 more frame if needed after P2 enters 0xc2 attack state.
                m.set_p2_buttons(idle);
                m.step();
            }
            p1_move_id(&m)
        };
        assert_eq!(
            run_interrupt(false),
            2,
            "[{label}] Faithful 1987 must keep P1 locked in walk (move 2) during 0x91 frames"
        );
        assert_eq!(
            run_interrupt(true),
            expected_mv,
            "[{label}] interruptible_walk_cycle must immediately enter move {expected_mv:#x}"
        );
    }

    // 3. Suppressing the 16-pixel first-frame backward shift when starting a normal attack while holding Back:
    //    Walk P1 up to P2 (2 complete walk cycles = 36 frames), then hold Back for 1 frame before pressing LK.
    let run_back_lk = |interruptible: bool| -> (u16, u16, u16) {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_interruptible_walk_cycle(interruptible);
        let mut m = versus_machine_with_config(cfg);
        for _ in 0..80 {
            m.set_p1_direction(true, false, false, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        for _ in 0..20 {
            m.set_p1_direction(false, false, false, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        let sx_before_back = p1_sx(&m);
        let hp_before = p2_health(&m);

        // Frame 1: hold Back (starts Walk Back, move 3)
        m.set_p1_direction(false, true, false, false);
        m.set_p1_buttons(idle);
        m.step();
        let sx_on_back_frame1 = p1_sx(&m);

        // Frame 2: while still holding Back, press LK
        m.set_p1_direction(false, true, false, false);
        m.set_p1_buttons(lk);
        m.step();
        assert_eq!(
            p1_move_id(&m),
            0x14,
            "Back + LK on frame 2 of Walk Back starts standing LK (0x14)"
        );

        for _ in 0..25 {
            m.set_p1_direction(false, true, false, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        let hp_after = p2_health(&m);
        (
            sx_before_back.wrapping_sub(sx_on_back_frame1),
            hp_before,
            hp_after,
        )
    };

    let (shift_false, hp_before_false, hp_after_false) = run_back_lk(false);
    let (shift_true, hp_before_true, hp_after_true) = run_back_lk(true);

    assert_eq!(
        shift_false, 16,
        "Faithful 1987 shifts Ryu 16 pixels backward on the very first frame of Walk Back"
    );
    assert_eq!(
        hp_after_false, hp_before_false,
        "Faithful 1987 Back + LK whiffs because of the 16-pixel first-frame backward shift"
    );
    assert_eq!(
        shift_true, 0,
        "interruptible_walk_cycle suppresses the 16-pixel first-frame backward shift"
    );
    assert!(
        hp_after_true < hp_before_true,
        "with interruptible_walk_cycle enabled, point-blank Back + LK connects and damages Ken ({hp_before_true} -> {hp_after_true})"
    );
}

#[test]
fn anytime_jump_attacks_paired_differential_ascent_apex_descent_and_prejump() {
    let idle = [false; 6];
    let hp = [false, false, true, false, false, false];
    let hk = [false, false, false, false, false, true];

    // 1. Pre-jump crouch duration: 9 frames in Faithful 1987 vs 4 frames with fast_prejump_crouch:
    let measure_prejump_frames = |anytime: bool, fast_prejump: bool| -> u32 {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_anytime_jump_attacks(anytime)
            .with_fast_prejump_crouch(fast_prejump);
        let mut m = versus_machine_with_config(cfg);
        m.set_p1_direction(false, false, false, true); // Neutral jump (move 9)
        m.set_p1_buttons(idle);
        m.step();
        let prejump_sy = p1_sy(&m);
        let mut crouch_frames = 1u32;
        m.set_p1_direction(false, false, false, false);
        for _ in 0..20 {
            m.step();
            if p1_sy(&m) == prejump_sy {
                crouch_frames += 1;
            } else {
                break;
            }
        }
        crouch_frames
    };
    assert_eq!(
        measure_prejump_frames(false, false),
        9,
        "Faithful 1987 pre-jump crouch lasts 9 frames"
    );
    assert_eq!(
        measure_prejump_frames(true, false),
        9,
        "anytime_jump_attacks with fast_prejump_crouch=false preserves 9-frame pre-jump crouch"
    );
    assert_eq!(
        measure_prejump_frames(true, true),
        4,
        "anytime_jump_attacks with fast_prejump_crouch=true shortens pre-jump crouch to 4 frames"
    );

    // 2. Neutral jump attack (move 9 -> 0x20 / 0x26) and Directional jump attack (move 10 -> 0x21 / 0x24)
    //    triggered during Ascent, Apex, and Descent (up to 4 frames before landing):
    //    Neutral jump has 27 airborne frames (apex at airborne frame 18);
    //    Forward jump has 35 airborne frames (1987 trigger at airborne frame 27).
    for (jump_label, jump_dir, btn, expected_air_mv, ascent_f, apex_f, descent_f, too_late_f) in [
        (
            "neutral_jump_hp",
            (false, false, false, true),
            hp,
            0x20u8,
            4u32,  // ascent (4f after takeoff)
            14u32, // near apex (before 1987's f=18)
            22u32, // descent (5f before landing at f=27, >= 4f)
            25u32, // too late (2f before landing at f=27, < 4f)
        ),
        (
            "forward_jump_hk",
            (true, false, false, true),
            hk,
            0x24u8,
            4u32,  // ascent (4f after takeoff)
            17u32, // apex (f=17, well before 1987's f=27)
            30u32, // late descent (5f before landing at f=35, >= 4f)
            33u32, // too late (2f before landing at f=35, < 4f)
        ),
    ] {
        for (phase_name, press_air_frame, should_trigger) in [
            ("ascent", ascent_f, true),
            ("apex", apex_f, true),
            ("descent", descent_f, true),
            ("too_late_before_landing", too_late_f, false),
        ] {
            let run_air_attack = |anytime: bool| -> (u8, u16) {
                let cfg = MatchConfig::faithful()
                    .with_positive_edge_attacks(true)
                    .with_anytime_jump_attacks(anytime)
                    .with_fast_prejump_crouch(false);
                let mut m = versus_machine_with_config(cfg);
                // Start jump (9 pre-jump crouch frames).
                m.set_p1_direction(jump_dir.0, jump_dir.1, jump_dir.2, jump_dir.3);
                m.set_p1_buttons(idle);
                m.step();
                m.set_p1_direction(false, false, false, false);
                for _ in 0..8 {
                    m.step();
                }
                // Advance to `press_air_frame` in the air and press the attack button.
                let mut mv_on_press = 0u8;
                for f in 1..=press_air_frame {
                    m.set_p1_buttons(if f == press_air_frame { btn } else { idle });
                    m.step();
                    if f == press_air_frame {
                        mv_on_press = p1_move_id(&m);
                    }
                }
                // Step until the jump finishes and P1 returns to ground stance, tracking
                // the minimum and maximum vertical sprite position (sy) during the jump.
                let mut min_sy = p1_sy(&m);
                let mut max_sy = p1_sy(&m);
                m.set_p1_buttons(idle);
                for _ in 0..60 {
                    m.step();
                    let sy = p1_sy(&m);
                    min_sy = min_sy.min(sy);
                    max_sy = max_sy.max(sy);
                    if (p1_state_byte(&m) & 0x80) == 0 {
                        break;
                    }
                }
                assert!(
                    min_sy <= 0x0078 && max_sy <= 0x0098,
                    "[{jump_label}:{phase_name}:anytime={anytime}] jump arc must stay airborne and above ground crouch (min_sy={min_sy:#06x}, max_sy={max_sy:#06x})"
                );
                (mv_on_press, p1_sy(&m))
            };

            let (mv_false, sy_false) = run_air_attack(false);
            let (mv_true, sy_true) = run_air_attack(true);

            assert_eq!(
                sy_false, 0x0088,
                "[{jump_label}:{phase_name}] Faithful 1987 must land at ground sy=0x0088"
            );
            assert_eq!(
                sy_true, 0x0088,
                "[{jump_label}:{phase_name}] anytime_jump_attacks must land cleanly at ground sy=0x0088"
            );

            if should_trigger {
                assert_ne!(
                    mv_false, expected_air_mv,
                    "[{jump_label}:{phase_name}] Faithful 1987 must NOT trigger {expected_air_mv:#x} immediately on frame {press_air_frame}"
                );
                assert_eq!(
                    mv_true, expected_air_mv,
                    "[{jump_label}:{phase_name}] anytime_jump_attacks must trigger {expected_air_mv:#x} immediately on frame {press_air_frame}"
                );
            } else {
                assert_ne!(
                    mv_true, expected_air_mv,
                    "[{jump_label}:{phase_name}] < 4 frames before landing must NOT start an air attack"
                );
            }
        }
    }
}

#[test]
fn faithful_1987_preset_preserves_exact_mame_parity_on_specials_walk_and_jump_baselines() {
    let root = repo_root();
    let rom_dir = root.join("tools/reference/local/roms/sf");
    let manifest = root.join("tools/reference/rom-manifest.json");
    let assets = load_assets(&rom_dir, &manifest).expect("ROM assets");
    let program_rom = assets.maincpu.clone();
    let fields = CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap();

    for (scen_id, json_file, cap_dir_name, last_frame) in [
        (
            "sf-specials-02",
            "sf_specials_02.json",
            "specials-02-run-a",
            2860u32,
        ),
        (
            "sf-normals-04",
            "sf_normals_04.json",
            "normals-04-run-a",
            3540u32,
        ),
        ("sf-jump-01", "sf_jump_01.json", "jump-01-run-a", 2999u32),
        (
            "sf-jump-attacks-01",
            "sf_jump_attacks_01.json",
            "jump-attacks-01-run-a",
            3580u32,
        ),
    ] {
        let scenario_path = root.join(format!("tools/reference/scenarios/{json_file}"));
        let ref_dir = root.join(format!("tools/reference/local/captures/{cap_dir_name}"));
        let scenario = load_scenario(&scenario_path).expect("scenario JSON");

        // 1. Verify owned CPU RAM matches MAME 0.289 at every frame under Faithful 1987.
        let ram_outcome =
            compare_versus_ram(&ref_dir, program_rom.clone(), &scenario_path, last_frame)
                .expect("compare_versus_ram");
        assert_eq!(
            ram_outcome,
            Ok(last_frame - 2450),
            "Faithful 1987 CPU-owned RAM must match MAME 0.289 on {scen_id}"
        );

        // 2. Verify rendered screen hashes and sound latches match MAME 0.289 at every frame.
        let scratch = ScratchDir::new(&format!("o4-faithful-{scen_id}"));
        run_versus_interval_capture(
            &ref_dir,
            &assets,
            &scratch,
            scen_id,
            &scenario_path,
            last_frame,
        )
        .expect("run_versus_interval_capture");
        let reference = load_capture(&ref_dir).expect("reference capture");
        let candidate = load_capture(&scratch).expect("candidate capture");
        let scope = CompareScope {
            from: Some(2450),
            to: Some(last_frame as u64),
            fields: Some(fields.clone()),
        };
        let outcome = compare(
            &scenario,
            &reference,
            &candidate,
            CandidateKind::Native,
            &scope,
        )
        .expect("compare");
        assert!(
            matches!(outcome, Outcome::Match { .. }),
            "Faithful 1987 observables must pass with zero diffs on {scen_id}: {outcome:?}"
        );
    }
}
