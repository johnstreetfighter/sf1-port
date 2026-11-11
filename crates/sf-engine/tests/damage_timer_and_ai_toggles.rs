//! Integration tests for Ticket `#97` (`O6: Balanced Damage/Chip Scaling, Fireball Slowdown Toggle, True-Second Timer, and Sagat AI Scaling`).

use std::path::PathBuf;

use sf_engine::config::{MatchConfig, DEFAULT_CPU_REACTION_DELAY};
use sf_engine::fight::FightMachine;
use sf_engine::fighter::{CPU_STRUCT, GAME_MODE, OBJ_BASE, P1_STRUCT, P2_STRUCT, SCENE_MODE};
use sf_engine::run::{compare_cpu_ram, compare_versus_ram, CpuRamOutcome};
use sf_video::romset::load_program_rom;

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

fn p2_health(m: &FightMachine) -> u16 {
    let w = m.work_ram();
    let off = (P2_STRUCT - 0xff8000) as usize;
    (w[off + 7] as u16) * 4 + (w[off + 6] as u16)
}

#[test]
fn balanced_roster_damage_and_chip_paired_differential() {
    // 1. Unblocked Special Move Damage Scaling (Hadoken, Shoryuken, Tatsumaki, Tiger Shot, CPU melee):
    //    - Faithful 1987: Hadoken = 20 HP (41.7%), Shoryuken = 20 HP (41.7%), Tatsumaki = 20 HP (41.7%),
    //      Tiger Shot = 30 HP (62.5%), Sagat Tiger Knee (0x46a34) = 30 HP (62.5%).
    //    - balanced_roster=true: Hadoken = 9 HP (18.75%), Shoryuken = 10 HP (20.8%),
    //      Tatsumaki = 9 HP (18.75%), Tiger Shot = 10 HP (20.8%), CPU melee capped at 12 HP (25%).
    let measure_special_damages = |balanced: bool| -> (u16, u16, u16, u16, u16) {
        let cfg = MatchConfig::faithful().with_balanced_roster(balanced);
        let mut m = versus_machine_with_config(cfg);
        let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let p2_e = (p2_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;

        // (a) Unblocked Hadoken (template 0x186dc -> 0x433ba -> entry 0x4345e):
        let hp_before = p2_health(&m);
        m.with_bus(|bus| {
            bus.set_eb(p2_e, 9, 0x02);
            bus.set_eb(p2_e, 0xa, 1);
            bus.spawn_effect(p1_e, 0x186dc);
            bus.set_ew(0x100, 4, bus.center_y(p2_e) - 8);
            bus.set_ew(0x100, 6, bus.center_x(p2_e) - 8);
            bus.run_effects();
        });
        let hadoken_dmg = hp_before - p2_health(&m);

        // Reset P2 health to 48 HP (11 cells + 4 rem):
        let reset_p2 = |m: &mut FightMachine| {
            m.with_bus(|bus| {
                bus.wb(P2_STRUCT + 7, 11);
                bus.wb(P2_STRUCT + 6, 4);
                bus.set_eb(0x100, 8, 0);
                bus.set_el(0x100, 4, 0);
                bus.set_eb(p2_e, 8, 2);
                bus.set_eb(p2_e, 9, 0x02);
                bus.set_eb(p2_e, 0xa, 1);
                bus.set_eb(p2_e, 0xf, 0);
            });
        };

        // (b) Unblocked Shoryuken (HitFn::Shoryuken 0x45efa -> entry 0x45f52):
        reset_p2(&mut m);
        let hp_before = p2_health(&m);
        m.with_bus(|bus| {
            let p2_cx = bus.center_x(p2_e);
            bus.start_move_at(p1_e, P1_STRUCT, 0x19, p2_cx - 24);
            bus.set_eb(p1_e, 8, 2);
            bus.set_eb(p1_e, 9, 0xc1);
            bus.set_eb(p1_e, 0x33, 5);
            bus.set_eb(p1_e, 0xf, 0x80);
            bus.set_eb(p1_e, 0x22, 0);
            bus.set_eb(p1_e, 0x23, 80);
            bus.set_eb(p1_e, 0x24, 0);
            bus.set_eb(p1_e, 0x25, 80);
            bus.set_el(p1_e, 0x28, 0x45efa);
            bus.run_fighter(p1_e, p2_e);
        });
        let shoryuken_dmg = hp_before - p2_health(&m);

        // (c) Unblocked Tatsumaki (HitFn::Tatsumaki 0x45f5a -> entry 0x46012):
        reset_p2(&mut m);
        let hp_before = p2_health(&m);
        m.with_bus(|bus| {
            let p2_cx = bus.center_x(p2_e);
            bus.start_move_at(p1_e, P1_STRUCT, 0x1b, p2_cx - 24);
            bus.set_eb(p1_e, 8, 2);
            bus.set_eb(p1_e, 9, 0xc1);
            bus.set_eb(p1_e, 0x33, 5);
            bus.set_eb(p1_e, 0xf, 0x80);
            bus.set_eb(p1_e, 0x22, 0);
            bus.set_eb(p1_e, 0x23, 80);
            bus.set_eb(p1_e, 0x24, 0);
            bus.set_eb(p1_e, 0x25, 80);
            bus.set_el(p1_e, 0x28, 0x45f5a);
            bus.run_fighter(p1_e, p2_e);
        });
        let tatsumaki_dmg = hp_before - p2_health(&m);

        // (d) Unblocked Sagat Tiger Shot (template 0x1873c -> 0x43652 -> entry 0x436a8):
        reset_p2(&mut m);
        let hp_before = p2_health(&m);
        m.with_bus(|bus| {
            bus.spawn_effect(p1_e, 0x1873c);
            bus.set_ew(0x100, 4, bus.center_y(p2_e) - 8);
            bus.set_ew(0x100, 6, bus.center_x(p2_e) - 8);
            bus.run_effects();
        });
        let tiger_shot_dmg = hp_before - p2_health(&m);

        // (e) Unblocked Sagat Tiger Knee / CPU heavy hit (HitFn::Cpu 0x48062 -> entry 0x480c8 = 28 HP):
        reset_p2(&mut m);
        let hp_before = p2_health(&m);
        m.with_bus(|bus| {
            let p2_cx = bus.center_x(p2_e);
            bus.start_move_at(p1_e, P1_STRUCT, 0x0f, p2_cx - 24);
            bus.set_eb(p1_e, 8, 2);
            bus.set_eb(p1_e, 9, 0xc2);
            bus.set_eb(p1_e, 0x33, 5);
            bus.set_eb(p1_e, 0xf, 0x80);
            bus.set_eb(p1_e, 0x22, 0);
            bus.set_eb(p1_e, 0x23, 80);
            bus.set_eb(p1_e, 0x24, 0);
            bus.set_eb(p1_e, 0x25, 80);
            bus.set_el(p1_e, 0x28, 0x48062);
            bus.run_fighter(p1_e, p2_e);
        });
        let sagat_melee_dmg = hp_before - p2_health(&m);

        (
            hadoken_dmg,
            shoryuken_dmg,
            tatsumaki_dmg,
            tiger_shot_dmg,
            sagat_melee_dmg,
        )
    };

    assert_eq!(
        measure_special_damages(false),
        (20, 20, 20, 30, 15),
        "Faithful 1987: Hadoken/DP/Tatsu deal 20 HP (41.7%), Tiger Shot deals 30 HP (62.5%), and Tiger Knee deals 15-30 HP"
    );
    assert_eq!(
        measure_special_damages(true),
        (9, 10, 9, 10, 12),
        "balanced_roster: Hadoken/Tatsu = 9 HP, DP/Tiger Shot = 10 HP, CPU melee capped at 12 HP"
    );

    // 2. Stage Chip Damage Normalization Across All Countries (including Stage 4 Thailand):
    //    - Faithful 1987: Stage 0/1 = 8 HP, Stage 2/3 = 4 HP, Stage 4 (Thailand) = 0 HP!
    //    - balanced_roster=true: 1 HP across all stages 0..=4 (including Thailand)!
    let measure_stage_hadoken_chip = |balanced: bool, stage: u8| -> u16 {
        let cfg = MatchConfig::faithful().with_balanced_roster(balanced);
        let mut m = versus_machine_with_config(cfg);
        let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let p2_e = (p2_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let hp_before = p2_health(&m);
        m.with_bus(|bus| {
            bus.wb(GAME_MODE, 0); // 1P Arcade mode (uses stage chip table 0x4340a)
            bus.wb(0xffc003, stage);
            bus.set_eb(p2_e, 9, 0x22); // Blocking high
            bus.set_eb(p2_e, 0xa, 4);
            bus.spawn_effect(p1_e, 0x186dc);
            bus.set_ew(0x100, 4, bus.center_y(p2_e) - 8);
            bus.set_ew(0x100, 6, bus.center_x(p2_e) - 8);
            bus.run_effects();
        });
        hp_before - p2_health(&m)
    };
    let faithful_chips: Vec<u16> = (0..=4)
        .map(|s| measure_stage_hadoken_chip(false, s))
        .collect();
    let balanced_chips: Vec<u16> = (0..=4)
        .map(|s| measure_stage_hadoken_chip(true, s))
        .collect();
    assert_eq!(
        faithful_chips,
        vec![8, 8, 4, 4, 0],
        "Faithful 1987 stage chip damage is 8 HP on stages 0-1, 4 HP on stages 2-3, and 0 HP on stage 4 (Thailand)"
    );
    assert_eq!(
        balanced_chips,
        vec![1, 1, 1, 1, 1],
        "balanced_roster normalizes blocked special chip damage to 1 HP across all stages including Thailand"
    );

    // 3. `chip_death` Sub-Toggle at 1 HP:
    //    - When `chip_death == false` (default): chip damage at 1 HP clamps at 1 HP and does not KO.
    //    - When `chip_death == true`: chip damage at 1 HP drains to 0 HP and KOs (`+8 == 8`).
    let run_chip_at_1hp = |chip_death: bool| -> (u16, u8) {
        let cfg = MatchConfig::faithful()
            .with_balanced_roster(true)
            .with_chip_death(chip_death);
        let mut m = versus_machine_with_config(cfg);
        let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let p2_e = (p2_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        m.with_bus(|bus| {
            // Set P2 to 1 HP (0 cells, 1 remainder) in Standing Guard (move 4, state 0x22):
            bus.wb(P2_STRUCT + 7, 0);
            bus.wb(P2_STRUCT + 6, 1);
            bus.set_eb(p2_e, 8, 2);
            bus.set_eb(p2_e, 9, 0x22);
            bus.set_eb(p2_e, 0xa, 4);
            // Hit blocking P2 with a Hadoken:
            bus.spawn_effect(p1_e, 0x186dc);
            bus.set_ew(0x100, 4, bus.center_y(p2_e) - 8);
            bus.set_ew(0x100, 6, bus.center_x(p2_e) - 8);
            bus.run_effects();
        });
        let state_8 = m.state.objectram[p2_entity(&m) + 8];
        (p2_health(&m), state_8)
    };
    let (hp_no_chip_ko, st_no_chip_ko) = run_chip_at_1hp(false);
    let (hp_chip_ko, st_chip_ko) = run_chip_at_1hp(true);
    assert_eq!(
        (hp_no_chip_ko, st_no_chip_ko),
        (1, 2),
        "chip_death=false leaves a 1 HP blocking defender alive at 1 HP (+8 == 2) without KO"
    );
    assert_eq!(
        (hp_chip_ko, st_chip_ko),
        (0, 8),
        "chip_death=true allows chip damage to drain the final 1 HP and KO the defender (+8 == 8)"
    );
}

#[test]
fn disable_fireball_slowdown_paired_differential() {
    // Run a projectile hit with (disable_slowdown, lethal_hit) and check:
    // - SCENE_MODE bit 0x20 (0xff8e42)
    // - Scene event request 0xff8e36 (9 = SlowMotion)
    // - SLOW_MOTION flag 0xffc00b after stepping the scheduler
    let run_projectile_slowdown = |disable_slowdown: bool, lethal: bool| -> (u8, u8, u8) {
        let cfg = MatchConfig::faithful().with_disable_fireball_slowdown(disable_slowdown);
        let mut m = versus_machine_with_config(cfg);
        let p1_e = (p1_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let p2_e = (p2_entity(&m) + 0xffe000 - OBJ_BASE as usize) as u16;
        let (scene_mode, event_req) = m.with_bus(|bus| {
            bus.wb(0xff8e36, 0);
            if lethal {
                // Set P2 to 10 HP so a 20 HP Hadoken is a round-winning KO hit:
                bus.wb(P2_STRUCT + 7, 2);
                bus.wb(P2_STRUCT + 6, 2);
            }
            bus.set_eb(p2_e, 9, 0x02);
            bus.set_eb(p2_e, 0xa, 1);
            bus.spawn_effect(p1_e, 0x186dc);
            bus.set_ew(0x100, 4, bus.center_y(p2_e) - 8);
            bus.set_ew(0x100, 6, bus.center_x(p2_e) - 8);
            bus.run_effects();
            (bus.rb(SCENE_MODE), bus.rb(0xff8e36))
        });
        m.step();
        let slow_motion = m.work_ram()[(0xffc00b - 0xff8000) as usize];
        (scene_mode & 0x20, event_req, slow_motion)
    };

    // 1. Non-lethal hit with disable_fireball_slowdown=false (Faithful 1987): triggers slow-motion crawl.
    assert_eq!(
        run_projectile_slowdown(false, false),
        (0x20, 9, 0xff),
        "Faithful 1987 triggers Scene Event 9 (SLOW_MOTION = 0xff) on non-lethal fireball hits"
    );

    // 2. Non-lethal hit with disable_fireball_slowdown=true: runs at continuous 60 FPS without slow-motion.
    assert_eq!(
        run_projectile_slowdown(true, false),
        (0, 0, 0),
        "disable_fireball_slowdown skips Scene Event 9 on non-lethal fireball hits so combat stays at 60 FPS"
    );

    // 3. Round-winning KO hit with disable_fireball_slowdown=true: retains dramatic slow-motion finish!
    assert_eq!(
        run_projectile_slowdown(true, true),
        (0x20, 9, 0xff),
        "disable_fireball_slowdown retains Scene Event 9 slow-motion on round-winning KO fireball hits"
    );
}

#[test]
fn real_second_timer_and_draws_paired_differential() {
    let idle = [false; 6];

    // 1. Timer Tick Cadence (19 frames in Faithful 1987 vs 60 frames = 1.0 real second in Overhaul):
    let count_frames_for_one_timer_tick = |real_timer: bool| -> u32 {
        let cfg = MatchConfig::faithful().with_real_second_timer_and_draws(real_timer);
        let mut m = versus_machine_with_config(cfg);
        // Align TIME_FRAMES (0xff8e1b) to 0 and record initial 3-digit timer at 0xff8e18..0xff8e1a:
        let init_digits = m.with_bus(|bus| {
            bus.wb(0xff8e1b, 0);
            [bus.rb(0xff8e18), bus.rb(0xff8e19), bus.rb(0xff8e1a)]
        });
        let mut frames = 0u32;
        for _ in 0..100 {
            m.set_p1_direction(false, false, false, false);
            m.set_p1_buttons(idle);
            m.set_p2_direction(false, false, false, false);
            m.set_p2_buttons(idle);
            m.step();
            frames += 1;
            let w = m.work_ram();
            let cur = [w[0xe18], w[0xe19], w[0xe1a]];
            if cur != init_digits {
                break;
            }
        }
        frames
    };
    assert_eq!(
        count_frames_for_one_timer_tick(false),
        19,
        "Faithful 1987 ticks the round timer every 19 frames (~3.15 ticks/sec)"
    );
    assert_eq!(
        count_frames_for_one_timer_tick(true),
        60,
        "real_second_timer_and_draws ticks the round timer every 60 frames (1.0 real second at 60 Hz)"
    );

    // 2. Tied 3-Round Match / Double KO Judgment:
    //    Set up Round 3 (`0xffc00a == 2`) tied at 1-1 (`P1 +8 == 1, P2 +8 == 1`) with 1 timer tick left
    //    and equal vitality (40 HP vs 40 HP) so Time Over triggers a Draw at the end of Round 3:
    //    - Under `real_second_timer_and_draws=false` (Faithful 1987): after the Draw text holds and clears,
    //      `after_round` increments `0xffc00a` to `3`, `three_rounds` sees `d2 == d0`, sets `0xffc009 = 0`,
    //      and ends the match with a double Game Over (`AwaitGameOverWipe` -> `GameOverCycle`).
    //    - Under `real_second_timer_and_draws=true`: `three_rounds` sees `d2 == d0`, keeps `0xffc00a = 2`,
    //      requests round wipe event `6` (`0xff8e36 == 6`), and starts a Sudden Death / Draw decider round
    //      (both fighters reset to 48 HP and re-enter `FightLoop` with `SCENE_MODE == 0x80`)!
    let run_tied_round3_outcome = |real_timer: bool| -> (bool, u8, u8) {
        let cfg = MatchConfig::faithful().with_real_second_timer_and_draws(real_timer);
        let mut m = versus_machine_with_config(cfg);

        // Set Round 3 (`0xffc00a = 2`), equal wins (`1-1`), and 1 frame remaining on the round timer (`001`):
        m.with_bus(|bus| {
            bus.wb(0xffc00a, 2);
            bus.wb(P1_STRUCT + 8, 1);
            bus.wb(P2_STRUCT + 8, 1);
            bus.wb(0xff8e18, 0);
            bus.wb(0xff8e19, 0);
            bus.wb(0xff8e1a, 1);
            bus.wb(0xff8e1b, if real_timer { 59 } else { 0x12 });
            // Damage both fighters equally to 40 HP (10 cells, 0 rem) before Time Over so we can
            // detect when the Sudden Death decider round resets both fighters back to full 48 HP:
            bus.wb(P1_STRUCT + 7, 10);
            bus.wb(P1_STRUCT + 6, 0);
            bus.wb(P2_STRUCT + 7, 10);
            bus.wb(P2_STRUCT + 6, 0);
        });

        let mut saw_sudden_death_fight = false;
        let mut max_round_no = 2u8;
        for _ in 0..750 {
            m.set_p1_direction(false, false, false, false);
            m.set_p1_buttons(idle);
            m.set_p2_direction(false, false, false, false);
            m.set_p2_buttons(idle);
            m.step();
            let w = m.work_ram();
            let round_no = w[0x400a];
            let scene_mode = w[0x4008];
            let p1_hp = (w[0x06b7] as u16) * 4 + (w[0x06b6] as u16);
            if round_no > max_round_no {
                max_round_no = round_no;
            }
            if p1_hp == 48 && scene_mode == 0x80 {
                saw_sudden_death_fight = true;
                break;
            }
        }
        let match_winner = m.work_ram()[0x4009];
        (saw_sudden_death_fight, max_round_no, match_winner)
    };

    let (sd_false, max_rnd_false, winner_false) = run_tied_round3_outcome(false);
    let (sd_true, max_rnd_true, winner_true) = run_tied_round3_outcome(true);
    assert_eq!(
        (sd_false, max_rnd_false, winner_false),
        (false, 3, 0),
        "Faithful 1987 ends a 1-1 tied 3-round match with round=3, winner=0 (Game Over)"
    );
    assert_eq!(
        (sd_true, max_rnd_true, winner_true),
        (true, 2, 0),
        "real_second_timer_and_draws advances a 1-1 tied 3-round match into a Sudden Death decider round at 48 HP"
    );
}

#[test]
fn fair_cpu_and_sagat_scaling_paired_differential() {
    let idle = [false; 6];

    // 1. CPU Input-Reading Reaction Delay (`cpu_follow_up` at `0x45630`):
    //    test directly at a CPU follow-up decision frame when P1 has just started a standing punch (+9 = 0xc2, mv = 0x0d):
    //    - With `fair_cpu_and_sagat_scaling = false`: CPU reads P1's input on frame 1 (`human_attack_frames == 1`)
    //      and immediately takes the `cpu_react` (0x4571a) branch.
    //    - With `fair_cpu_and_sagat_scaling = true`: CPU does NOT read P1's input on frames `1..delay`,
    //      and only takes `cpu_react` once P1's attack has been visible for `>= cpu_reaction_delay` frames!
    let test_cpu_input_reading_at_frame = |fair_cpu: bool, attack_frames_elapsed: u8| -> (u8, u8) {
        let root = repo_root();
        let capture = root.join("tools/reference/local/captures/cpu-01-run-a");
        let cfg = MatchConfig::faithful()
            .with_fair_cpu_and_sagat_scaling(fair_cpu)
            .with_cpu_reaction_delay(DEFAULT_CPU_REACTION_DELAY);
        let mut m = FightMachine::from_checkpoint_2389(&capture, load_rom())
            .expect("checkpoint 2389 fixture")
            .with_match_config(cfg);
        m.step();
        m.step();

        m.with_bus(|bus| {
            let p1_e = 0x400u16; // Ryu
            let cpu_e = 0xa00u16; // Retsu
                                  // Place Ryu and Retsu 0x20 pixels apart with timer active (99s), mark Ryu as
                                  // attacking (+8 = 2, +9 = 0xc0, mv = 0x0d) and set Retsu's range band to 0xd0..0xf0
                                  // with range-shift roll = 0x20 so only the input-reading `cpu_react` branch can
                                  // switch Retsu from move 2 to `react_mv`:
            bus.ww(0xff8e18, 0x0909);
            bus.set_eb(p1_e, 8, 2);
            bus.set_eb(p1_e, 9, 0xc0);
            bus.set_eb(p1_e, 0xa, 0x0d);
            bus.set_ew(p1_e, 6, 0x100);
            bus.set_ew(cpu_e, 6, 0x120);
            bus.wb(CPU_STRUCT + 0x19, 0);
            bus.wb(CPU_STRUCT + 0x16, 0xd0);
            bus.wb(CPU_STRUCT + 0x17, 0xf0);
            bus.wb(0xff8e4a + 3, 0);
            bus.wb(0xffc0d4, 0x20);
            bus.wb(0xff8e4a + 4, 0);
            let react_mv = bus.rb(0xffc164 + 5);
            bus.wb(0xff8e4a + 6, 0);
            bus.wb(0xffc0c4, 0x20);

            let follow_up_entry = (0x0cc00u32..0x0d400)
                .find(|&a| bus.rb(a) == 0xff && bus.rb(a + 1) == 0 && bus.rl(a + 2) == 0x45630)
                .expect("Retsu cpu_follow_up stream entry");
            bus.set_eb(cpu_e, 8, 2);
            bus.set_eb(cpu_e, 9, 0x91);
            bus.set_eb(cpu_e, 0xa, 2);
            bus.set_eb(cpu_e, 0x21, 0x40);
            bus.set_eb(cpu_e, 0x33, 0);
            bus.set_eb(cpu_e, 0xb, 0);
            bus.set_el(cpu_e, 0x2c, follow_up_entry);
            bus.toggle_state.human_attack_frames[0] = attack_frames_elapsed;
            bus.run_fighter(cpu_e, p1_e);
            (bus.eb(cpu_e, 0xa), react_mv)
        })
    };

    let (mv_f1_false, react_mv) = test_cpu_input_reading_at_frame(false, 1);
    let (mv_f1_true, _) = test_cpu_input_reading_at_frame(true, 1);
    let (mv_before_delay_true, _) =
        test_cpu_input_reading_at_frame(true, DEFAULT_CPU_REACTION_DELAY - 1);
    let (mv_at_delay_true, _) = test_cpu_input_reading_at_frame(true, DEFAULT_CPU_REACTION_DELAY);

    assert_ne!(react_mv, 2);
    assert_eq!(
        mv_f1_false, react_mv,
        "Faithful 1987 CPU reads player attack input on frame 1 and immediately triggers cpu_react"
    );
    assert_eq!(
        mv_f1_true, 2,
        "fair_cpu_and_sagat_scaling suppresses frame-1 CPU input reading (stays in walk move 2)"
    );
    assert_eq!(
        mv_before_delay_true, 2,
        "fair_cpu_and_sagat_scaling suppresses CPU input reading before cpu_reaction_delay ({}f) elapses",
        DEFAULT_CPU_REACTION_DELAY
    );
    assert_eq!(
        mv_at_delay_true, react_mv,
        "fair_cpu_and_sagat_scaling allows CPU reaction once player's attack has been visible for cpu_reaction_delay ({}f) frames",
        DEFAULT_CPU_REACTION_DELAY
    );

    // 2. Retsu's Unblockable Kick (`0x468de`, `sf-1p-block-01`):
    //    Verify that `fair_cpu_and_sagat_scaling=true` alone (even with `standard_high_low_block=false`)
    //    makes Retsu's `0x468de` low kick blockable!
    let run_retsu_unblockable = |fair_cpu: bool| -> u8 {
        let root = repo_root();
        let capture = root.join("tools/reference/local/captures/cpu-01-run-a");
        let cfg = MatchConfig::faithful().with_fair_cpu_and_sagat_scaling(fair_cpu);
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
    assert_eq!(
        run_retsu_unblockable(false),
        32,
        "Faithful 1987 Retsu's 0x468de kick is unblockable and hits through Crouching Guard twice (48 -> 32 HP)"
    );
    assert_eq!(
        run_retsu_unblockable(true),
        48,
        "fair_cpu_and_sagat_scaling makes Retsu's 0x468de kick blockable (48 HP)"
    );

    // 3. Sagat & Adon Difficulty Scaling Across Easy (0), Normal (1), Hard (2), and Very Hard (3):
    for (name, fight) in [("Adon", 0u8), ("Sagat", 1u8)] {
        let collect_profiles = |fair_cpu: bool| -> Vec<(usize, usize, u16, usize)> {
            let root = repo_root();
            let capture = root.join("tools/reference/local/captures/cpu-01-run-a");
            let cfg = MatchConfig::faithful().with_fair_cpu_and_sagat_scaling(fair_cpu);
            let mut m = FightMachine::from_checkpoint_2389(&capture, load_rom())
                .expect("checkpoint 2389 fixture")
                .with_match_config(cfg);
            (0..=3u8)
                .map(|d1| {
                    m.with_bus(|bus| {
                        bus.wb(0xffc003, 4); // Stage 4: Thailand
                        bus.wb(0xffc004, fight);
                        bus.wb(0xffc006, 0);
                        bus.wb(0xffc007, d1);
                        bus.setup_cpu_opponent();

                        let active_shift =
                            (0..16u32).filter(|&i| bus.rb(0xffc0c4 + i) == 0x40).count();
                        let active_react =
                            (0..16u32).filter(|&i| bus.rb(0xffc0d4 + i) == 0x20).count();
                        let stun_wait_sum: u16 = (0..16u32)
                            .map(|i| bus.rb(0xffc0a4 + 2 * i + 1) as u16)
                            .sum();
                        let special_atk_count = (0..144u32)
                            .filter(|&idx| {
                                let mv = bus.rb(0xffc1e4 + 8 * idx + 5);
                                matches!(mv, 0x19 | 0x20 | 0x21 | 0x23 | 0x24)
                            })
                            .count();
                        (active_shift, active_react, stun_wait_sum, special_atk_count)
                    })
                })
                .collect()
        };

        let faithful_profiles = collect_profiles(false);
        let scaled_profiles = collect_profiles(true);

        if name == "Sagat" {
            assert!(
                faithful_profiles.windows(2).all(|w| w[0] == w[1]),
                "Faithful 1987 Sagat has 1 identical AI profile across all 4 difficulty settings: {:?}",
                faithful_profiles
            );
        }
        for d in 0..3 {
            let (sh_a, re_a, wait_a, sp_a) = scaled_profiles[d];
            let (sh_b, re_b, wait_b, sp_b) = scaled_profiles[d + 1];
            assert!(
                sh_a < sh_b && re_a < re_b && wait_a > wait_b && sp_a < sp_b,
                "[{name}] fair_cpu_and_sagat_scaling must scale monotonically from difficulty {d} {:?} to {} {:?}",
                scaled_profiles[d],
                d + 1,
                scaled_profiles[d + 1]
            );
        }
    }
}

#[test]
fn faithful_1987_parity_zero_regression_for_o6() {
    let root = repo_root();
    let rom = load_rom();

    // 1. Hadoken & slow-motion capture (`hadoken-01-run-a`, 2450..2940):
    let hadoken_cap = root.join("tools/reference/local/captures/hadoken-01-run-a");
    let hadoken_scen = root.join("tools/reference/scenarios/sf_hadoken_01.json");
    assert_eq!(
        compare_versus_ram(&hadoken_cap, rom.clone(), &hadoken_scen, 2940)
            .expect("hadoken-01-run-a"),
        Ok(2940 - 2450)
    );

    // 2. Double KO 3-round tie capture (`doubleko-01-run-a`, 2450..5661):
    let doubleko_cap = root.join("tools/reference/local/captures/doubleko-01-run-a");
    let doubleko_scen = root.join("tools/reference/scenarios/sf_doubleko_01.json");
    assert_eq!(
        compare_versus_ram(&doubleko_cap, rom.clone(), &doubleko_scen, 5661)
            .expect("doubleko-01-run-a"),
        Ok(5661 - 2450)
    );

    // 3. Retsu (`cpu-01-run-a`, 2389..2433) & Round 2 follow-up (`round2-02-run-a`, 3900..5600):
    let cpu_cap = root.join("tools/reference/local/captures/cpu-01-run-a");
    assert_eq!(
        compare_cpu_ram(&cpu_cap, rom.clone(), 2389, 2433).expect("cpu-01-run-a"),
        CpuRamOutcome::Match { frames: 44 }
    );
    let r2_cap = root.join("tools/reference/local/captures/round2-02-run-a");
    assert_eq!(
        compare_cpu_ram(&r2_cap, rom, 3900, 5600).expect("round2-02-run-a"),
        CpuRamOutcome::Match { frames: 1700 }
    );
}
