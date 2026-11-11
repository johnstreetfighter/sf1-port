//! Integration tests for Ticket `#96` (`O5: Standard High/Low/Jump Blocking, Lee/Adon Omniblock Fix, and Symmetrical Shoto Balance`).

use std::path::PathBuf;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::config::MatchConfig;
use sf_engine::fight::FightMachine;
use sf_engine::fighter::{CPU_STRUCT, OBJ_BASE, P1_STRUCT, P2_STRUCT};
use sf_engine::run::{
    compare_cpu_ram_with_inputs, compare_versus_ram, run_cpu_fight_capture_with_inputs,
    run_versus_interval_capture, CpuRamOutcome, ScratchDir,
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
        root.join("tools/reference/local/roms/sf"),
        root.join("tools/reference/rom-manifest.json"),
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

fn p1_state_byte(m: &FightMachine) -> u8 {
    m.state.objectram[p1_entity(m) + 0x09]
}

fn p2_state_byte(m: &FightMachine) -> u8 {
    m.state.objectram[p2_entity(m) + 0x09]
}

fn p2_health(m: &FightMachine) -> u16 {
    let w = m.work_ram();
    let off = (P2_STRUCT - 0xff8000) as usize + 6;
    (w[off + 1] as u16) * 4 + (w[off] as u16)
}

#[test]
fn standard_high_low_block_paired_differential_standing_crouching_and_jump_guards() {
    let idle = [false; 6];
    let lp = [true, false, false, false, false, false];
    let mk = [false, false, false, false, true, false];
    let hk = [false, false, false, false, false, true];

    // 1. Crouching Guard (move 8) blocks standing attacks (MK 0x15 / HK 0x16 / punches) under standard_high_low_block:
    let run_down_back_vs_standing = |standard_block: bool| -> (u8, u8, bool) {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_standard_high_low_block(standard_block);
        let mut m = versus_machine_with_config(cfg);

        m.set_p1_direction(false, false, false, false);
        m.set_p1_buttons(mk);
        // P2 holds Down-Back (Right + Down, facing left):
        m.set_p2_direction(true, false, true, false);
        m.set_p2_buttons(idle);
        m.step();

        // Step 1 more frame so P2's input handler sees P1 in 0xc2:
        m.set_p1_buttons(idle);
        m.step();
        let mv_during_startup = p2_move_id(&m);
        let st_during_startup = p2_state_byte(&m);

        // Also test hit_redirect resolution directly against move 8:
        let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let p2_e = (p2_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let hp_before = p2_health(&m);
        m.with_bus(|bus| {
            bus.set_eb(p2_e, 0xa, 8);
            bus.set_eb(p2_e, 9, 0x24);
            bus.set_eb(p1_e, 8, 2);
            bus.set_eb(p1_e, 0x33, 5);
            bus.set_eb(p1_e, 0xf, 0x80);
            bus.set_eb(p1_e, 0x22, 0);
            bus.set_eb(p1_e, 0x23, 32);
            bus.set_eb(p1_e, 0x24, 0);
            bus.set_eb(p1_e, 0x25, 220);
            bus.set_el(p1_e, 0x28, 0x460b8);
            bus.run_fighter(p1_e, p2_e);
        });
        let hp_after = p2_health(&m);
        (mv_during_startup, st_during_startup, hp_after == hp_before)
    };
    let (mv_false, st_false, blocked_false) = run_down_back_vs_standing(false);
    let (mv_true, st_true, blocked_true) = run_down_back_vs_standing(true);
    assert_eq!(
        (mv_false, st_false, blocked_false),
        (7, 0x14, false),
        "Faithful 1987 keeps Down-Back in move 7 (0x14) against standing attacks and hits through move 8"
    );
    assert_eq!(
        (mv_true, st_true, blocked_true),
        (8, 0x24, true),
        "standard_high_low_block enters Crouching Guard (move 8, 0x24) against standing attacks and blocks them"
    );

    // 2. Holding Back vs Down-Back against a Low Attack (0xc4):
    let run_vs_low = |standard_block: bool, crouch_guard: bool| -> (u8, u16, u16) {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_standard_high_low_block(standard_block);
        let mut m = versus_machine_with_config(cfg);
        for _ in 0..85 {
            m.set_p1_direction(true, false, false, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        for _ in 0..20 {
            m.set_p1_direction(false, false, false, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        let hp_before = p2_health(&m);

        // P1 starts Crouching HK / Sweep (Down + HK -> move 0x18, state 0xc4):
        m.set_p1_direction(false, false, true, false);
        m.set_p1_buttons(hk);
        // P2 holds Back (Right, move 4) or Down-Back (Right + Down, move 8):
        m.set_p2_direction(true, false, crouch_guard, false);
        m.set_p2_buttons(idle);
        m.step();

        m.set_p1_buttons(idle);
        m.step();
        let guard_move = p2_move_id(&m);

        for _ in 0..25 {
            m.set_p1_direction(false, false, true, false);
            m.set_p1_buttons(idle);
            m.set_p2_direction(true, false, crouch_guard, false);
            m.set_p2_buttons(idle);
            m.step();
        }
        let hp_after = p2_health(&m);
        (guard_move, hp_before, hp_after)
    };
    let (guard_mv_false_stand, hp_b_fs, hp_a_fs) = run_vs_low(false, false);
    let (guard_mv_true_stand, hp_b_ts, hp_a_ts) = run_vs_low(true, false);
    let (guard_mv_true_crouch, hp_b_tc, hp_a_tc) = run_vs_low(true, true);
    assert_eq!(
        guard_mv_false_stand, 3,
        "Faithful 1987 walks backward (move 3) when holding Back against a low attack"
    );
    assert!(hp_a_fs < hp_b_fs);
    assert_eq!(
        guard_mv_true_stand, 4,
        "standard_high_low_block enters Standing Guard (move 4) when holding Back against a low attack"
    );
    assert!(
        hp_a_ts < hp_b_ts,
        "low sweep must hit Standing Guard (move 4) as a true low"
    );
    assert_eq!(
        guard_mv_true_crouch, 8,
        "standard_high_low_block enters Crouching Guard (move 8) when holding Down-Back against a low attack"
    );
    assert_eq!(
        hp_a_tc, hp_b_tc,
        "Crouching Guard (move 8) must block the low sweep with zero damage"
    );

    // 3. Jumping Overhead vs Crouching Guard (sf-blocks-05 bug fix):
    //    In sf-blocks-05 (2450..=2600), P1 forward-jumps with HK (0x24, +9 = 0xc1) while P2 holds Down-Back.
    //    - Under standard_high_low_block=false, P2 falsely enters Crouching Guard (move 8) and is hit anyway (48 -> 38 HP).
    //    - Under standard_high_low_block=true, P2 does NOT falsely enter move 8 against a jumping attack (stays in move 7)
    //      and is hit by the jumping overhead, whereas holding Standing Back (Right) enters Standing Guard (move 4) and blocks it!
    let run_jump_overhead = |standard_block: bool, stand_guard: bool| -> (bool, bool, u16, u16) {
        let cfg = MatchConfig::faithful().with_standard_high_low_block(standard_block);
        let mut m = versus_machine_with_config(cfg);
        let hp_before = p2_health(&m);
        let mut entered_move_8 = false;
        let mut entered_move_4 = false;

        for frame in 2450..2600 {
            let p2_back = (2450..2540).contains(&frame);
            let p1_jump = (2460..2472).contains(&frame);
            let p1_hk_btn = (2468..2474).contains(&frame);

            m.set_p1_direction(p1_jump, false, false, p1_jump);
            m.set_p1_buttons(if p1_hk_btn { hk } else { idle });
            m.set_p2_direction(p2_back, false, p2_back && !stand_guard, false);
            m.set_p2_buttons(idle);
            m.step();

            if p2_move_id(&m) == 8 {
                entered_move_8 = true;
            }
            if p2_move_id(&m) == 4 {
                entered_move_4 = true;
            }
        }
        (entered_move_8, entered_move_4, hp_before, p2_health(&m))
    };
    let (mv8_false, _, hp_b_f, hp_a_f) = run_jump_overhead(false, false);
    let (mv8_true_crouch, _, hp_b_tc, hp_a_tc) = run_jump_overhead(true, false);
    let (_, mv4_true_stand, hp_b_ts, hp_a_ts) = run_jump_overhead(true, true);
    assert!(
        mv8_false && hp_a_f < hp_b_f,
        "Faithful 1987 (sf-blocks-05) falsely enters Crouching Guard (move 8) against Jump HK and gets hit anyway"
    );
    assert!(
        !mv8_true_crouch && hp_a_tc < hp_b_tc,
        "standard_high_low_block never falsely enters Crouching Guard (move 8) against a jumping overhead, and the overhead hits low-crouching defenders"
    );
    assert!(
        mv4_true_stand && hp_a_ts == hp_b_ts,
        "Standing Guard (move 4) blocks the jumping HK overhead with zero damage"
    );

    // 4. Proximity Guard vs Incoming Projectiles (after thrower recovers) & Gated Walk Interrupt:
    let run_projectile_guard = |standard_block: bool, crouch_guard: bool| -> u8 {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_lenient_command_trackers(true)
            .with_interruptible_walk_cycle(true)
            .with_standard_high_low_block(standard_block);
        let mut m = versus_machine_with_config(cfg);

        // Walk P1 and P2 to full-screen distance first so the projectile is still in flight
        // after P1 recovers to neutral stance:
        for _ in 0..45 {
            m.set_p1_direction(false, true, false, false);
            m.set_p2_direction(true, false, false, false);
            m.step();
        }
        m.set_p1_direction(false, false, false, false);
        m.set_p2_direction(false, false, false, false);
        m.step();

        // P1 throws Hadoken (D, DF, F + LP):
        m.set_p1_direction(false, false, true, false);
        m.step();
        m.set_p1_direction(true, false, true, false);
        m.step();
        m.set_p1_direction(true, false, false, false);
        m.set_p1_buttons(lp);
        m.step();
        m.set_p1_direction(false, false, false, false);
        m.set_p1_buttons(idle);

        // Wait until P1 spawns the Hadoken projectile in slot 0x100, then return P1 to neutral stance (mv = 1)
        // while the Hadoken projectile is still flying across the screen toward P2:
        while m.state.objectram[0x100 + 8] == 0 {
            m.step();
        }
        let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        m.with_bus(|bus| {
            bus.start_move(p1_e, P1_STRUCT, 1);
        });
        assert_eq!(
            p1_move_id(&m),
            1,
            "P1 is in neutral stance while projectile is still in flight"
        );
        assert_ne!(
            m.state.objectram[0x100 + 8],
            0,
            "Hadoken projectile is active in flight"
        );

        // Now P2 holds Back (Right) or Down-Back (Right + Down):
        m.set_p2_direction(true, false, crouch_guard, false);
        m.step();
        p2_move_id(&m)
    };
    assert_eq!(
        run_projectile_guard(false, false),
        3,
        "Faithful 1987 fails to enter block once the fireball thrower recovers (walks back in move 3)"
    );
    assert_eq!(
        run_projectile_guard(true, false),
        4,
        "standard_high_low_block enters Standing Guard (move 4) while an incoming projectile is active"
    );
    assert_eq!(
        run_projectile_guard(true, true),
        8,
        "standard_high_low_block enters Crouching Guard (move 8) while an incoming projectile is active"
    );

    // 5. 1P CPU Blocking (sf-1p-block-01): Retsu's 0x18 low kick (0x468de) lacked a block check in 1987!
    //    In sf-1p-block-01 (2389..=2700), Ryu holds Down-Back over 2530..2680.
    //    - Under standard_high_low_block=false, Retsu's 0x18 hits through Ryu's crouching block twice (48 -> 40 -> 32 HP).
    //    - Under standard_high_low_block=true, Ryu's crouching block (move 8, state 0x24) blocks both hits (48 HP)!
    let run_1p_retsu_block = |standard_block: bool| -> u8 {
        let root = repo_root();
        let capture = root.join("tools/reference/local/captures/cpu-01-run-a");
        let cfg = MatchConfig::faithful().with_standard_high_low_block(standard_block);
        let mut m = FightMachine::from_checkpoint_2389(&capture, load_rom())
            .expect("checkpoint 2389 fixture")
            .with_match_config(cfg);

        for frame in 2389..2610 {
            let p1_right = (2390..2400).contains(&frame);
            let p1_left = (2434..2610).contains(&frame);
            let p1_down = (2530..2610).contains(&frame);
            m.set_p1_direction(p1_right, p1_left, p1_down, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        let w = m.work_ram();
        let off = (P1_STRUCT - 0xff8000) as usize;
        w[off + 7] * 4 + w[off + 6]
    };
    let retsu_hp_false = run_1p_retsu_block(false);
    let retsu_hp_true = run_1p_retsu_block(true);
    assert_eq!(
        retsu_hp_false, 32,
        "Faithful 1987 (sf-1p-block-01): Retsu's 0x468de low kick hits through Crouching Guard twice (48 -> 32 HP)"
    );
    assert_eq!(
        retsu_hp_true, 48,
        "standard_high_low_block: Crouching Guard blocks Retsu's 0x468de low kick with zero damage (48 HP)"
    );
}

#[test]
fn lee_birdie_and_adon_omniblock_hurtbox_fix_paired_differential() {
    // Verify that Lee (0x175bc / 0x0f236), Birdie (0x17a9c / 0x1138c), and Adon (0x17f7c / 0x1324c, 0x1326c)
    // have zeroed/shrunken hurtboxes during block (mv 4/8) and reaction frames in Faithful 1987,
    // and retain their full standing hurtboxes under standard_high_low_block=true (#96, O5).
    for (name, table, reaction_frame, expected_tight, expected_loose) in [
        (
            "Lee",
            0x175bcu32,
            0x0f236u32,
            [32u8, 8, 0xf8, 8],
            [0xfcu8, 44, 0xf0, 16],
        ),
        (
            "Birdie",
            0x17a9c,
            0x1138c,
            [24, 8, 0, 8],
            [0xfc, 60, 0xf8, 16],
        ),
        ("Adon", 0x17f7c, 0x1324c, [24, 8, 0, 8], [0, 48, 0xf8, 16]),
    ] {
        let inspect_boxes = |standard_block: bool| -> ([u8; 4], [u8; 4], [u8; 4], [u8; 4], bool) {
            let cfg = MatchConfig::faithful().with_standard_high_low_block(standard_block);
            let mut m = versus_machine_with_config(cfg);
            let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
            let p2_e = (p2_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;

            let (mv4_tight, mv4_loose) = m.with_bus(|bus| {
                bus.wl(P2_STRUCT + 0x12, table);
                bus.set_eb(p2_e, 0x37, 2); // facing right so boxes are unflipped
                bus.start_move_at(p2_e, P2_STRUCT, 4, 0x120);
                let tight = [
                    bus.eb(p2_e, 0x1a),
                    bus.eb(p2_e, 0x1b),
                    bus.eb(p2_e, 0x1c),
                    bus.eb(p2_e, 0x1d),
                ];
                let loose = [
                    bus.eb(p2_e, 0x1e),
                    bus.eb(p2_e, 0x1f),
                    bus.eb(p2_e, 0x20),
                    bus.eb(p2_e, 0x21),
                ];
                (tight, loose)
            });

            // Now test whether a high attack aimed at upper body (ay = cy - 28, ah = 4) connects
            // as a depth-1 clean hit during the reaction frame when P2 is not blocking (mv = 1):
            let (react_tight, react_loose) = m.with_bus(|bus| {
                // Place P2 at reaction_frame (next frame after an initial reaction step):
                bus.set_eb(p2_e, 0x37, 2);
                bus.set_eb(p2_e, 9, 0x82);
                bus.set_eb(p2_e, 0x33, 0);
                bus.set_eb(p2_e, 0xb, 0);
                bus.set_el(p2_e, 0x2c, reaction_frame);
                bus.run_fighter(p2_e, p1_e);
                let tight = [
                    bus.eb(p2_e, 0x1a),
                    bus.eb(p2_e, 0x1b),
                    bus.eb(p2_e, 0x1c),
                    bus.eb(p2_e, 0x1d),
                ];
                let loose = [
                    bus.eb(p2_e, 0x1e),
                    bus.eb(p2_e, 0x1f),
                    bus.eb(p2_e, 0x20),
                    bus.eb(p2_e, 0x21),
                ];

                // Set P2 to non-blocking move 1 (state 0x02) while keeping the loaded reaction boxes,
                // and have P1 attack at P2's tight-box height (ay = p2_cy - 28, ah = 4) to test depth-1 connection:
                bus.set_eb(p2_e, 0xa, 1);
                bus.set_eb(p2_e, 9, 0x02);
                bus.set_eb(p1_e, 0xf, 0x80);
                let dy_to_tight =
                    (bus.center_y(p1_e) as i16 - bus.center_y(p2_e) as i16 + 28) as u8;
                bus.set_eb(p1_e, 0x22, dy_to_tight);
                bus.set_eb(p1_e, 0x23, 4);
                bus.set_eb(p1_e, 0x24, 0);
                bus.set_eb(p1_e, 0x25, 220);
                bus.set_el(p1_e, 0x28, 0x45c98); // Standing HP (depth 1 = 12 dmg, depth 2 = 4 dmg)
                bus.set_eb(p1_e, 8, 2);
                bus.set_eb(p1_e, 9, 0xc2);
                bus.set_eb(p1_e, 0x33, 5);
                bus.run_fighter(p1_e, p2_e);
                (tight, loose)
            });
            // Depth-1 clean hit awards 1,000 points (digit +0x26 == 1) vs depth-2 grazed hit (500 points) or whiff (0):
            let depth1_score_digit = m.work_ram()[(P1_STRUCT - 0xff8000) as usize + 0x26];
            (
                mv4_tight,
                mv4_loose,
                react_tight,
                react_loose,
                depth1_score_digit == 1,
            )
        };

        let (t_f, l_f, rt_f, _rl_f, clean_hit_f) = inspect_boxes(false);
        let (t_t, l_t, rt_t, rl_t, clean_hit_t) = inspect_boxes(true);

        assert_eq!(
            t_f,
            [0, 0, 0, 0],
            "[{name}] Faithful 1987 zeroes tight hurtbox on move 4"
        );
        assert_eq!(
            rt_f,
            [0, 0, 0, 0],
            "[{name}] Faithful 1987 zeroes tight hurtbox on reaction frame {reaction_frame:#x}"
        );
        if name == "Adon" {
            assert_eq!(
                l_f,
                [0, 0, 0, 0],
                "[Adon] Faithful 1987 zeroes loose hurtbox on move 4 (Omniblock)"
            );
        }
        assert!(
            !clean_hit_f,
            "[{name}] Faithful 1987 prevents depth-1 clean hit on upper body during block/reaction"
        );

        assert_eq!(
            (t_t, l_t),
            (expected_tight, expected_loose),
            "[{name}] standard_high_low_block restores full standing hurtboxes on move 4"
        );
        assert_eq!(
            (rt_t, rl_t),
            (expected_tight, expected_loose),
            "[{name}] standard_high_low_block restores full standing hurtboxes on reaction frame {reaction_frame:#x}"
        );
        assert!(
            clean_hit_t,
            "[{name}] standard_high_low_block allows clean depth-1 follow-up hits during block/reaction"
        );
    }
}

#[test]
fn balanced_roster_symmetric_shotos_and_blockable_dp_paired_differential() {
    let idle = [false; 6];
    let lp = [true, false, false, false, false, false];
    let mk = [false, false, false, false, true, false];

    // 1. Blockable Shoryuken (0x45efa / 0x45f04) & Single-Hit Shoryuken (no multi-hit instant KO):
    let run_dp_vs_defender = |symmetric_dp: bool, defender_blocks: bool| -> (u16, u16, u8) {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_lenient_command_trackers(true)
            .with_standard_high_low_block(symmetric_dp)
            .with_balanced_roster(symmetric_dp);
        let mut m = versus_machine_with_config(cfg);

        // Walk P1 right up to P2 and push P2 into the corner so all rising frames of Shoryuken overlap P2:
        for _ in 0..170 {
            m.set_p1_direction(true, false, false, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        for _ in 0..20 {
            m.set_p1_direction(false, false, false, false);
            m.set_p1_buttons(idle);
            m.step();
        }
        let hp_before = p2_health(&m);

        // P1 executes Heavy Shoryuken (F, D, DF + HP -> move 0x19):
        let hp_btn = [false, false, true, false, false, false];
        m.set_p1_direction(true, false, false, false);
        m.step();
        m.set_p1_direction(false, false, true, false);
        m.step();
        m.set_p1_direction(true, false, true, false);
        m.set_p1_buttons(hp_btn);
        if defender_blocks {
            m.set_p2_direction(true, false, false, false); // Back (P2 faces left)
        }
        m.step();
        assert_eq!(p1_move_id(&m), 0x19, "P1 starts Shoryuken (0x19)");

        let mut hit_frames = 0u8;
        let mut prev_hp = hp_before;
        for _ in 0..35 {
            if !defender_blocks && hit_frames >= 1 {
                // Keep P2 standing in place right in front of P1 so any re-armed active hitbox
                // on Shoryuken's subsequent rising frames (0x01260 / 0x01280) will connect:
                let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
                let p2_e = (p2_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
                m.with_bus(|bus| {
                    let p1_cx = bus.center_x(p1_e);
                    bus.start_move_at(p2_e, P2_STRUCT, 1, p1_cx + 24);
                });
            }
            m.set_p1_direction(false, false, false, false);
            m.set_p1_buttons(idle);
            m.set_p2_direction(defender_blocks, false, false, false);
            m.set_p2_buttons(idle);
            m.step();
            let cur_hp = p2_health(&m);
            if cur_hp < prev_hp {
                hit_frames += 1;
                prev_hp = cur_hp;
            }
        }
        (hp_before, p2_health(&m), hit_frames)
    };

    // (a) Against a blocking opponent:
    let (hp_b_block_false, hp_a_block_false, hits_block_false) = run_dp_vs_defender(false, true);
    let (hp_b_block_true, hp_a_block_true, hits_block_true) = run_dp_vs_defender(true, true);
    assert!(
        hits_block_false >= 1 && hp_a_block_false < hp_b_block_false,
        "Faithful 1987 Shoryuken is unblockable and damages a blocking opponent"
    );
    assert_eq!(
        (hits_block_true, hp_a_block_true),
        (0, hp_b_block_true),
        "balanced_roster makes Shoryuken blockable with zero damage"
    );

    // (b) Against an unblocking opponent at point-blank range:
    let (_hp_b_hit_false, _hp_a_hit_false, hits_unblocked_false) = run_dp_vs_defender(false, false);
    let (hp_b_hit_true, hp_a_hit_true, hits_unblocked_true) = run_dp_vs_defender(true, false);
    assert!(
        hits_unblocked_false >= 2,
        "Faithful 1987 Shoryuken multi-hits ({hits_unblocked_false} hits) at point-blank range"
    );
    assert_eq!(
        hits_unblocked_true, 1,
        "balanced_roster caps Shoryuken at strictly 1 hit per uppercut"
    );
    assert!(hp_a_hit_true < hp_b_hit_true);

    // 2. Vulnerable Shoryuken Descent & Landing Recovery:
    //    Verify that during descent and landing recovery (0x012c0..=0x01340 for Ryu, 0x0339e..=0x0341e for Ken),
    //    invulnerability bit 0x08 is set in Faithful 1987 and cleared when balanced_roster=true.
    let check_dp_descent_vulnerability = |symmetric_dp: bool| -> (bool, bool) {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_lenient_command_trackers(true)
            .with_balanced_roster(symmetric_dp);
        let mut m = versus_machine_with_config(cfg);

        // Start P1 (Ryu) and P2 (Ken) in Shoryuken (move 0x19):
        m.set_p1_direction(true, false, false, false);
        m.set_p2_direction(false, true, false, false);
        m.step();
        m.set_p1_direction(false, false, true, false);
        m.set_p2_direction(false, false, true, false);
        m.step();
        m.set_p1_direction(true, false, true, false);
        m.set_p1_buttons(lp);
        m.set_p2_direction(false, true, true, false);
        m.set_p2_buttons(lp);
        m.step();
        assert_eq!(p1_move_id(&m), 0x19);
        assert_eq!(p2_move_id(&m), 0x19);

        // On startup frame 1, Ken's 0x0f bit 0x08 is 0 in Faithful 1987 vs 0x08 in symmetric:
        let ken_startup_invuln = (m.state.objectram[p2_entity(&m) + 0x0f] & 0x08) != 0;

        // Advance into the descending/landing recovery frames (step 24 of Shoryuken):
        m.set_p1_direction(false, false, false, false);
        m.set_p1_buttons(idle);
        m.set_p2_direction(false, false, false, false);
        m.set_p2_buttons(idle);
        let mut descent_vulnerable = false;
        for _ in 0..32 {
            m.step();
            let p1_e = p1_entity(&m);
            let stream_ptr = u32::from_be_bytes([
                m.state.objectram[p1_e + 0x2c],
                m.state.objectram[p1_e + 0x2d],
                m.state.objectram[p1_e + 0x2e],
                m.state.objectram[p1_e + 0x2f],
            ]);
            // When stream_ptr is in 0x012e0..=0x01360, the loaded frame is 0x012c0..=0x01340:
            if (0x012e0..=0x01360).contains(&stream_ptr) {
                let flags_0f = m.state.objectram[p1_e + 0x0f];
                if (flags_0f & 0x08) == 0 {
                    descent_vulnerable = true;
                }
            }
        }
        (ken_startup_invuln, descent_vulnerable)
    };
    let (ken_inv_false, descent_vuln_false) = check_dp_descent_vulnerability(false);
    let (ken_inv_true, descent_vuln_true) = check_dp_descent_vulnerability(true);
    assert_eq!(
        (ken_inv_false, descent_vuln_false),
        (false, false),
        "Faithful 1987 leaves Ken vulnerable on DP frame 1 and keeps both shotos invulnerable during DP descent/landing"
    );
    assert_eq!(
        (ken_inv_true, descent_vuln_true),
        (true, true),
        "balanced_roster gives Ken frame-1 DP invulnerability and makes DP descent/landing vulnerable"
    );

    // 3. Identical Ryu (P1) and Ken (P2) Frame Durations for Standing MK (0x15) and Hadoken (0x1a):
    let measure_move_frames = |symmetric: bool, use_hadoken: bool| -> (u16, u16) {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_lenient_command_trackers(true)
            .with_balanced_roster(symmetric);
        let mut m = versus_machine_with_config(cfg);

        if use_hadoken {
            m.set_p1_direction(false, false, true, false);
            m.set_p2_direction(false, false, true, false);
            m.step();
            m.set_p1_direction(true, false, true, false);
            m.set_p2_direction(false, true, true, false);
            m.step();
            m.set_p1_direction(true, false, false, false);
            m.set_p1_buttons(lp);
            m.set_p2_direction(false, true, false, false);
            m.set_p2_buttons(lp);
            m.step();
            assert_eq!(p1_move_id(&m), 0x1a);
            assert_eq!(p2_move_id(&m), 0x1a);
        } else {
            m.set_p1_buttons(mk);
            m.set_p2_buttons(mk);
            m.step();
            assert_eq!(p1_move_id(&m), 0x15);
            assert_eq!(p2_move_id(&m), 0x15);
        }

        m.set_p1_direction(false, false, false, false);
        m.set_p1_buttons(idle);
        m.set_p2_direction(false, false, false, false);
        m.set_p2_buttons(idle);

        let (mut p1_frames, mut p2_frames) = (1u16, 1u16);
        for _ in 0..90 {
            m.step();
            if (p1_state_byte(&m) & 0x80) != 0 {
                p1_frames += 1;
            }
            if (p2_state_byte(&m) & 0x80) != 0 {
                p2_frames += 1;
            }
        }
        (p1_frames, p2_frames)
    };
    let (ryu_mk_false, ken_mk_false) = measure_move_frames(false, false);
    let (ryu_mk_true, ken_mk_true) = measure_move_frames(true, false);
    assert_eq!(
        (ryu_mk_false, ken_mk_false),
        (45, 50),
        "Faithful 1987 Ken Standing MK is 5 frames slower than Ryu (50 vs 45 frames)"
    );
    assert_eq!(
        (ryu_mk_true, ken_mk_true),
        (45, 45),
        "balanced_roster makes Ryu and Ken Standing MK identical (45 frames)"
    );

    let (ryu_hado_false, ken_hado_false) = measure_move_frames(false, true);
    let (ryu_hado_true, ken_hado_true) = measure_move_frames(true, true);
    assert_eq!(
        ken_hado_false - ryu_hado_false,
        12,
        "Faithful 1987 Ken Hadoken recovery is 12 frames slower than Ryu"
    );
    assert_eq!(
        ryu_hado_true, ken_hado_true,
        "balanced_roster makes Ryu and Ken Hadoken recovery identical"
    );

    // 4. Input Symmetry in input_left (0x40798 Hadoken->Tatsumaki overwrite fix & 0x40714 block tracker preservation):
    let test_left_hadoken_priority = |symmetric: bool| -> u8 {
        let cfg = MatchConfig::faithful()
            .with_positive_edge_attacks(true)
            .with_balanced_roster(symmetric);
        let mut m = versus_machine_with_config(cfg);
        m.with_bus(|bus| {
            // Both Hadoken (+0x1c) and Tatsumaki (+0x16) trackers have done=0xff on P2 (facing left):
            bus.wb(P2_STRUCT + 0x1c, 3);
            bus.wb(P2_STRUCT + 0x1c + 2, 0xff);
            bus.wb(P2_STRUCT + 0x16, 0);
            bus.wb(P2_STRUCT + 0x16 + 2, 0xff);
        });
        // P2 presses Forward (Left, since P2 faces left) + LP:
        m.set_p2_direction(false, true, false, false);
        m.set_p2_buttons(lp);
        m.step();
        p2_move_id(&m)
    };
    assert_eq!(
        test_left_hadoken_priority(false),
        0x1b,
        "Faithful 1987 input_left overwrites Hadoken (0x1a) with Tatsumaki (0x1b) when both trackers are done"
    );
    assert_eq!(
        test_left_hadoken_priority(true),
        0x1a,
        "balanced_roster fires Hadoken (0x1a) on Forward + Punch just like input_right"
    );
}

#[test]
fn geki_shuriken_and_smoke_attack_deflection_and_sprite_offset_audit() {
    // 1. Geki shuriken (0x1849c..0x184dc) and disappearing smoke (0x1847c) deflection via 0x43396:
    //    When the player's attack box (+0x22..+0x25) overlaps Geki's shuriken or smoke (+0x0e = 0xc0),
    //    effect_vs_attack (0x43144) invokes handler 0x43396 with `other = None`.
    //    Verify that it deflects the effect (+9 = 0x63, +0xa = 4, +0xe = 0, sound 0x22) and NEVER
    //    records an unrecovered address (which previously caused sf-dev to restart from power-on).
    for template in [0x1847cu32, 0x1849c, 0x184bc, 0x184dc] {
        let mut m = versus_machine_with_config(MatchConfig::default());
        let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;

        m.with_bus(|bus| {
            // Spawn Geki's shuriken / smoke in effect slot 0x100 targeting P1:
            let s = 0x100u16;
            for i in 0..0x20u32 {
                let b = bus.rb(template + i);
                bus.set_eb(s, i, b);
            }
            bus.set_el(s, 0x20, P2_STRUCT);
            bus.set_el(s, 0x24, P1_STRUCT);
            let p1_cx = bus.center_x(p1_e);
            let p1_cy = bus.center_y(p1_e);
            // Place the shuriken/smoke 48px in front of P1 (outside P1's body hurtbox, inside P1's outstretched attack box):
            bus.set_ew(s, 6, p1_cx + 40);
            bus.set_ew(s, 4, p1_cy - 8);

            // Give P1 an active attack hitbox overlapping the shuriken/smoke and run_effects:
            bus.set_eb(p1_e, 0xf, 0x80);
            bus.set_eb(p1_e, 0x22, 0);
            bus.set_eb(p1_e, 0x23, 32);
            bus.set_eb(p1_e, 0x24, 48);
            bus.set_eb(p1_e, 0x25, 24);
            bus.run_effects();
        });
        assert!(
            m.unrecovered().is_empty(),
            "striking Geki effect template {template:#x} must not trigger unrecovered restart: {:?}",
            m.unrecovered()
        );
        let eff_state = m.state.objectram[0x100 + 9];
        let eff_step = m.state.objectram[0x100 + 0xa];
        let eff_col = m.state.objectram[0x100 + 0xe];
        assert_eq!(
            (eff_state, eff_step, eff_col),
            (0x63, 4, 0),
            "Geki effect {template:#x} must deflect cleanly via 0x43396"
        );
    }

    // 2. Sprite Vertical & Horizontal Offset Audit across all 10 CPU opponents
    //    (Geki, Retsu, Mike, Joe, Lee, Gen, Birdie, Eagle, Adon, Sagat) under DefinitiveOverhaul:
    for (name, table, expected_sy, expected_mv3_sx) in [
        ("Geki", 0x16bfcu32, 0x0088u16, 8i16),
        ("Retsu", 0x16e6c, 0x0088, -10),
        ("Mike", 0x170dc, 0x0078, -10),
        ("Joe", 0x1734c, 0x0088, -10),
        ("Lee", 0x175bc, 0x0088, 0),
        ("Gen", 0x1782c, 0x0088, -11),
        ("Birdie", 0x17a9c, 0x0068, 0),
        ("Eagle", 0x17d0c, 0x0088, -8),
        ("Adon", 0x17f7c, 0x0088, 0),
        ("Sagat", 0x181ec, 0x0068, -16),
    ] {
        let mut m = versus_machine_with_config(MatchConfig::default());
        let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let p2_e = (p2_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let (sy_stance, sy_after_jump, dx_mv3) = m.with_bus(|bus| {
            bus.wl(CPU_STRUCT + 0x12, table);
            bus.set_el(p2_e, 0x38, CPU_STRUCT);
            bus.set_eb(p2_e, 0x37, 2); // facing right
            bus.start_move_at(p2_e, CPU_STRUCT, 1, 0x140);
            let sy_1 = bus.ew(p2_e, 4);

            // Check Walk Back (mv = 3) preserves the CPU fighter's exact ROM horizontal header offset:
            let cx_1 = bus.center_x(p2_e);
            bus.start_move(p2_e, CPU_STRUCT, 3);
            let cx_3 = bus.center_x(p2_e);
            let dx_3 = cx_3.wrapping_sub(cx_1) as i16;

            // Check 0x454ee during a jump (mv = 9) does NOT shift CPU vertical centre to 0xb0:
            bus.start_move_at(p2_e, CPU_STRUCT, 1, 0x140);
            let cy_before = bus.center_y(p2_e);
            bus.set_eb(p2_e, 0xa, 9);
            bus.set_eb(p2_e, 9, 0x81);
            bus.set_eb(p2_e, 8, 2);
            bus.set_eb(p2_e, 0x33, 0);
            bus.set_eb(p2_e, 0xb, 0);
            bus.set_el(p2_e, 0x2c, 0x0b6a8);
            bus.run_fighter(p2_e, p1_e);
            let cy_after = bus.center_y(p2_e);
            assert_eq!(
                cy_after, cy_before,
                "[{name}] 0x454ee must never shift CPU vertical centre ({cy_before:#x} -> {cy_after:#x})"
            );
            (sy_1, sy_1, dx_3)
        });
        assert_eq!(
            (sy_stance, sy_after_jump, dx_mv3),
            (expected_sy, expected_sy, expected_mv3_sx),
            "[{name}] CPU ground sy and mv=3 horizontal offset must match ROM exactly"
        );
    }
}

#[test]
fn faithful_1987_preset_preserves_bit_exact_blocking_and_specials_baselines() {
    let root = repo_root();
    let roms = root.join("tools/reference/local/roms/sf");
    let manifest = root.join("tools/reference/rom-manifest.json");
    let assets = load_assets(&roms, &manifest).expect("ROM assets");
    let rom = load_program_rom(&roms, &manifest).expect("program ROM");

    // 1. sf-blocks-01, sf-blocks-05, and sf-specials-03 (Versus blocking & Shoryuken baselines):
    for (id, scen_rel, cap_rel, last) in [
        (
            "sf-blocks-01",
            "tools/reference/scenarios/sf_blocks_01.json",
            "tools/reference/local/captures/blocks-01-run-a",
            4320u32,
        ),
        (
            "sf-blocks-05",
            "tools/reference/scenarios/sf_blocks_05.json",
            "tools/reference/local/captures/blocks-05-run-a",
            2600,
        ),
        (
            "sf-specials-03",
            "tools/reference/scenarios/sf_specials_03.json",
            "tools/reference/local/captures/specials-03-run-a",
            2580,
        ),
    ] {
        let scen_path = root.join(scen_rel);
        let cap_a = root.join(cap_rel);
        let out = ScratchDir::new(&format!("o5-faithful-{id}"));

        run_versus_interval_capture(&cap_a, &assets, &out, id, &scen_path, last)
            .expect("versus capture");
        let scen = load_scenario(&scen_path).expect("scenario");
        let scope = CompareScope {
            from: Some(2450),
            to: Some(last as u64),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        };
        let outcome = compare(
            &scen,
            &load_capture(&cap_a).unwrap(),
            &load_capture(&out).unwrap(),
            CandidateKind::Native,
            &scope,
        )
        .expect("compare");
        assert!(
            matches!(outcome, Outcome::Match { .. }),
            "Faithful 1987 {id} observable mismatch: {outcome:?}"
        );

        let ram_outcome =
            compare_versus_ram(&cap_a, rom.clone(), &scen_path, last).expect("versus RAM compare");
        assert_eq!(
            ram_outcome,
            Ok(last - 2450),
            "Faithful 1987 {id} RAM mismatch"
        );
    }

    // 2. sf-1p-block-01 (1P Retsu blocking baseline):
    {
        let scen_path = root.join("tools/reference/scenarios/sf_1p_block_01.json");
        let cpu_ref_a = root.join("tools/reference/local/captures/cpu-01-run-a");
        let cap_a = root.join("tools/reference/local/captures/1p-block-01-run-a");
        let out = ScratchDir::new("o5-faithful-1p-block-01");
        run_cpu_fight_capture_with_inputs(
            &cpu_ref_a,
            &assets,
            &out,
            "sf-1p-block-01",
            Some(&scen_path),
            2389,
            2700,
        )
        .expect("1p block capture");
        let ram_outcome = compare_cpu_ram_with_inputs(&cap_a, rom, Some(&scen_path), 2389, 2700)
            .expect("1p block RAM compare");
        assert_eq!(
            ram_outcome,
            CpuRamOutcome::Match {
                frames: 2700 - 2389
            }
        );
    }
}
