//! Integration tests for `#93` (`O2`): 60-cell visual frame meter, numeric
//! frame advantage (`+` / `-`), deterministic full-session state snapshots,
//! and Replay Takeover.

use std::path::PathBuf;

use sf_engine::fight::FightMachine;
use sf_engine::overlay::{
    frame_meter_panel, render_diagnostic_overlay, FrameCellState, InteractionKind, OverlayColors,
    FRAME_METER_CELLS, FRAME_METER_CELL_ALPHA, FRAME_METER_PANEL_ALPHA,
};
use sf_engine::replay::{
    PlayerInputs, ReplayMode, ReplayTakeoverSession, TakeoverPlayer, DEFAULT_TAKEOVER_WINDOW_FRAMES,
};
use sf_engine::run::{apply_inputs, scenario_dsw, scenario_input_events, PowerOnMachine, INPUTS};
use sf_video::render::{render_frame, WIDTH};
use sf_video::romset::load_assets;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_test_assets() -> sf_video::romset::Assets {
    let root = workspace_root();
    let roms = root.join("tools/reference/local/roms/sf");
    let manifest = root.join("tools/reference/rom-manifest.json");
    load_assets(&roms, &manifest).expect("local ROM set should load")
}

fn run_versus_scenario_with_overlay(
    scenario_rel: &str,
    last_frame: u32,
    mut on_step: impl FnMut(u32, &FightMachine),
) -> FightMachine {
    let root = workspace_root();
    let fixture = root.join("tools/reference/local/captures/move-run-a");
    let scenario = root.join(scenario_rel);
    let assets = load_test_assets();
    let mut m = FightMachine::from_checkpoint_2450_with_program(&fixture, assets.maincpu)
        .expect("checkpoint-2450 fixture should load");
    let (dsw1, dsw2) = scenario_dsw(&scenario).expect("scenario dsw");
    m.set_dsw(dsw1, dsw2);
    let events = scenario_input_events(&scenario).expect("scenario input events");
    let mut next = 0usize;
    let mut held = [false; INPUTS];

    while m.state.frame < last_frame {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        on_step(frame, &m);
    }
    m
}

#[test]
fn frame_meter_tracks_all_states_and_signed_frame_advantage_across_scenarios() {
    let mut saw_startup = false;
    let mut saw_active = false;
    let mut saw_recovery = false;
    let mut saw_hitstun = false;
    let mut saw_blockstun = false;
    let mut saw_invulnerable = false;
    let mut saw_cancel_window = false;
    let mut saw_neutral = false;
    let mut saw_positive_hit_adv = false;
    let mut saw_negative_block_adv = false;

    // 1. sf-punch-01: standing light punches hitting P2 -> Startup, Active, Recovery, Hitstun, +advantage on Hit.
    let punch_end = run_versus_scenario_with_overlay(
        "tools/reference/scenarios/sf_punch_01.json",
        2900,
        |_frame, m| {
            let fm = m.frame_meter();
            for cell in [fm.p1.current_cell(), fm.p2.current_cell()] {
                match cell {
                    FrameCellState::Startup => saw_startup = true,
                    FrameCellState::Active => saw_active = true,
                    FrameCellState::Recovery => saw_recovery = true,
                    FrameCellState::Hitstun => saw_hitstun = true,
                    FrameCellState::Blockstun => saw_blockstun = true,
                    FrameCellState::Invulnerable => saw_invulnerable = true,
                    FrameCellState::CancelWindow => saw_cancel_window = true,
                    FrameCellState::Neutral => saw_neutral = true,
                }
            }
            if fm.p1.last_interaction == Some(InteractionKind::Hit) {
                if let (Some(p1_adv), Some(p2_adv)) = (fm.p1.frame_advantage, fm.p2.frame_advantage)
                {
                    assert_eq!(
                        p1_adv, -p2_adv,
                        "P1 and P2 frame advantage must be symmetric"
                    );
                    if p1_adv > 0 {
                        saw_positive_hit_adv = true;
                    }
                }
            }
        },
    );
    assert_eq!(
        punch_end.frame_meter().p1.cells.len(),
        FRAME_METER_CELLS,
        "rolling frame meter must have 60 cells"
    );
    assert!(
        punch_end.frame_meter().p1.recorded_frames >= 60,
        "rolling frame meter should record every stepped frame"
    );
    assert!(saw_startup, "sf-punch-01 should record Startup frames");
    assert!(saw_active, "sf-punch-01 should record Active frames");
    assert!(saw_recovery, "sf-punch-01 should record Recovery frames");
    assert!(saw_hitstun, "sf-punch-01 should record Hitstun frames");
    assert!(saw_neutral, "sf-punch-01 should record Neutral frames");
    assert!(
        saw_positive_hit_adv,
        "sf-punch-01 should compute positive frame advantage (+) for P1 on hit"
    );

    // 2. sf-blocks-01: P2 blocks P1's attacks -> Blockstun and negative frame advantage (-) for P1 on Block.
    run_versus_scenario_with_overlay(
        "tools/reference/scenarios/sf_blocks_01.json",
        3100,
        |_frame, m| {
            let fm = m.frame_meter();
            if fm.p2.current_cell() == FrameCellState::Blockstun {
                saw_blockstun = true;
            }
            if fm.p1.last_interaction == Some(InteractionKind::Block) {
                if let (Some(p1_adv), Some(p2_adv)) = (fm.p1.frame_advantage, fm.p2.frame_advantage)
                {
                    assert_eq!(p1_adv, -p2_adv);
                    if p1_adv < 0 && p2_adv > 0 {
                        saw_negative_block_adv = true;
                    }
                }
            }
        },
    );
    assert!(saw_blockstun, "sf-blocks-01 should record Blockstun frames");
    assert!(
        saw_negative_block_adv,
        "sf-blocks-01 should compute negative frame advantage (-) for attacker and (+) for defender on block"
    );

    // 3. sf-specials-01 & sf-move-01: Shoryuken Invulnerable frames, walk-cycle & special CancelWindow frames.
    run_versus_scenario_with_overlay(
        "tools/reference/scenarios/sf_specials_01.json",
        2550,
        |_frame, m| {
            let fm = m.frame_meter();
            for cell in [fm.p1.current_cell(), fm.p2.current_cell()] {
                if cell == FrameCellState::Invulnerable {
                    saw_invulnerable = true;
                }
                if cell == FrameCellState::CancelWindow {
                    saw_cancel_window = true;
                }
            }
        },
    );
    assert!(
        saw_invulnerable,
        "sf-specials-01 should record Invulnerable frames during Shoryuken startup"
    );
    assert!(
        saw_cancel_window,
        "sf-specials-01 should record CancelWindow frames during walk cycle / special command window"
    );

    // 4. sf-hadoken-02: Projectile chip-damage block (move_id 0x3b) -> classified as Blockstun with calculated Block advantage.
    let mut saw_chip_blockstun = false;
    let mut saw_chip_block_adv = false;
    run_versus_scenario_with_overlay(
        "tools/reference/scenarios/sf_hadoken_02.json",
        2620,
        |frame, m| {
            let t = m.diagnostic_telemetry();
            let fm = &t.frame_meter;
            if fm.p2.current_cell() == FrameCellState::Blockstun {
                saw_chip_blockstun = true;
            }
            assert_ne!(
                fm.p2.current_cell(),
                FrameCellState::Hitstun,
                "frame {frame}: p1=({:#x},{:#x}) p2=({:#x},{:#x},impact={}) should be Blockstun, not Hitstun",
                t.p1.move_id,
                t.p1.state_byte,
                t.p2.move_id,
                t.p2.state_byte,
                t.p2.projectile_impacted_this_frame,
            );
            if fm.p1.last_interaction == Some(InteractionKind::Block) {
                if let (Some(p1_adv), Some(p2_adv)) = (fm.p1.frame_advantage, fm.p2.frame_advantage)
                {
                    assert_eq!(p1_adv, -p2_adv);
                    saw_chip_block_adv = true;
                }
            }
        },
    );
    assert!(
        saw_chip_blockstun,
        "sf-hadoken-02 should record Blockstun frames during projectile chip block"
    );
    assert!(
        saw_chip_block_adv,
        "sf-hadoken-02 should compute projectile frame advantage on block"
    );
}

#[test]
fn frame_meter_renders_all_eight_cell_colors_without_mutating_base_frame() {
    let assets = load_test_assets();
    let mut m = run_versus_scenario_with_overlay(
        "tools/reference/scenarios/sf_specials_01.json",
        2480,
        |_, _| {},
    );
    let base = render_frame(&assets, &m.state.scene());
    let base_copy = base.argb.clone();

    let mut telemetry = m.diagnostic_telemetry();
    let all_states = [
        FrameCellState::Neutral,
        FrameCellState::Startup,
        FrameCellState::Active,
        FrameCellState::Recovery,
        FrameCellState::Hitstun,
        FrameCellState::Blockstun,
        FrameCellState::Invulnerable,
        FrameCellState::CancelWindow,
    ];
    for (i, state) in all_states.into_iter().enumerate() {
        telemetry.frame_meter.p1.cells[i] = state;
        telemetry.frame_meter.p2.cells[FRAME_METER_CELLS - 1 - i] = state;
    }
    telemetry.frame_meter.p1.frame_advantage = Some(5);
    telemetry.frame_meter.p2.frame_advantage = Some(-5);

    let disabled = render_diagnostic_overlay(&base, &telemetry, false);
    assert_eq!(
        disabled.argb, base_copy,
        "disabled overlay must return exact base frame"
    );

    let rendered = render_diagnostic_overlay(&base, &telemetry, true);
    assert_eq!(
        base.argb, base_copy,
        "overlay rendering must not mutate base frame"
    );
    assert_ne!(rendered.argb, base_copy);

    let colors = [
        OverlayColors::FRAME_NEUTRAL,
        OverlayColors::FRAME_STARTUP,
        OverlayColors::FRAME_ACTIVE,
        OverlayColors::FRAME_RECOVERY,
        OverlayColors::FRAME_HITSTUN,
        OverlayColors::FRAME_BLOCKSTUN,
        OverlayColors::FRAME_INVULNERABLE,
        OverlayColors::FRAME_CANCEL_WINDOW,
    ];
    // The cells are blended over the frame, not drawn solid (spec #238,
    // option B): each is its colour laid over the faintly tinted stage.
    let blend = |dst: u32, src: u32, alpha: u32| {
        let ch = |sh: u32| (((dst >> sh) & 0xff) * (256 - alpha) + ((src >> sh) & 0xff) * alpha) >> 8;
        0xff00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
    };
    let panel = frame_meter_panel();
    for (i, c) in colors.into_iter().enumerate() {
        for (row_y, cell) in [(panel.y + 2, i), (panel.y + 10, FRAME_METER_CELLS - 1 - i)] {
            let x = 34 + 5 * cell + 1;
            let y = row_y + 2;
            let under = blend(base_copy[y * WIDTH + x], OverlayColors::HUD_BG, FRAME_METER_PANEL_ALPHA);
            assert_eq!(
                rendered.argb[y * WIDTH + x],
                blend(under, c, FRAME_METER_CELL_ALPHA),
                "60-cell frame meter cell {cell} should show 0x{c:08x} blended over the frame"
            );
        }
    }

    // Enabling/disabling overlay on FightMachine leaves state identical.
    let snap_before = m.save_snapshot();
    m.set_overlay_enabled(true);
    let _ = m.render_with_overlay(&assets);
    m.set_overlay_enabled(false);
    assert_eq!(m.state, snap_before.state);
    assert_eq!(m.work_ram(), snap_before.work.as_slice());
}

#[test]
fn deterministic_snapshot_round_trip_fight_and_power_on_machines() {
    let root = workspace_root();
    let fixture = root.join("tools/reference/local/captures/move-run-a");
    let scenario = root.join("tools/reference/scenarios/sf_hadoken_01.json");
    let assets = load_test_assets();

    // Part A: FightMachine save at frame N=2475, advance 60 frames to N+60=2535,
    // restore N, re-simulate 60 frames with identical inputs, verify bit-identical state, pixels, and audio latches.
    let mut m = FightMachine::from_checkpoint_2450_with_program(&fixture, assets.maincpu.clone())
        .expect("checkpoint-2450 fixture");
    let (dsw1, dsw2) = scenario_dsw(&scenario).expect("scenario dsw");
    m.set_dsw(dsw1, dsw2);
    let events = scenario_input_events(&scenario).expect("scenario input events");
    let mut next = 0usize;
    let mut held = [false; INPUTS];

    let frame_n = 2475u32;
    while m.state.frame < frame_n {
        let f = m.state.frame + 1;
        while next < events.len() && events[next].0 < f {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
    }

    let snapshot_n = m.save_snapshot();
    let saved_next = next;
    let saved_held = held;

    let mut latches_pass_1 = Vec::with_capacity(60);
    for _ in 0..60 {
        let f = m.state.frame + 1;
        while next < events.len() && events[next].0 < f {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        latches_pass_1.push((
            m.state.frame,
            m.state.sound_latch,
            m.state.latch_cycles,
            m.latch_writes().to_vec(),
        ));
    }
    let state_pass_1 = m.save_snapshot();
    let pixels_pass_1 = render_frame(&assets, &m.state.scene()).argb;
    let overlay_pixels_pass_1 = render_diagnostic_overlay(
        &render_frame(&assets, &m.state.scene()),
        &m.diagnostic_telemetry(),
        true,
    )
    .argb;

    // Restore snapshot at frame N and re-simulate the exact 60 frames to N+60.
    m.restore_snapshot(&snapshot_n);
    next = saved_next;
    held = saved_held;
    assert_eq!(m.state.frame, frame_n);

    let mut latches_pass_2 = Vec::with_capacity(60);
    for _ in 0..60 {
        let f = m.state.frame + 1;
        while next < events.len() && events[next].0 < f {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        apply_inputs(&mut m, &held);
        m.step();
        latches_pass_2.push((
            m.state.frame,
            m.state.sound_latch,
            m.state.latch_cycles,
            m.latch_writes().to_vec(),
        ));
    }
    let state_pass_2 = m.save_snapshot();
    let pixels_pass_2 = render_frame(&assets, &m.state.scene()).argb;
    let overlay_pixels_pass_2 = render_diagnostic_overlay(
        &render_frame(&assets, &m.state.scene()),
        &m.diagnostic_telemetry(),
        true,
    )
    .argb;

    assert_eq!(
        state_pass_1, state_pass_2,
        "FightMachine snapshot round-trip must produce bit-identical state at N+60"
    );
    assert_eq!(
        pixels_pass_1, pixels_pass_2,
        "FightMachine snapshot round-trip must produce bit-identical pixels at N+60"
    );
    assert_eq!(
        overlay_pixels_pass_1, overlay_pixels_pass_2,
        "FightMachine snapshot round-trip must produce bit-identical overlay pixels at N+60"
    );
    assert_eq!(
        latches_pass_1, latches_pass_2,
        "FightMachine snapshot round-trip must produce bit-identical audio latches across N..=N+60"
    );

    // Part B: PowerOnMachine save at frame N=100, advance 60 frames to N+60=160,
    // restore N, re-simulate 60 frames to 160, verify bit-identical state, pixels, and audio latches.
    let mut pm = PowerOnMachine::new(assets.maincpu.clone());
    for _ in 0..100 {
        pm.step();
    }
    let pm_snap_n = pm.save_snapshot();
    let mut pm_latches_1 = Vec::with_capacity(60);
    for i in 0..60 {
        pm.set_cabinet_inputs(i == 10, false, i == 30, false, false);
        pm.step();
        pm_latches_1.push((
            pm.state.frame,
            pm.state.sound_latch,
            pm.state.latch_cycles,
            pm.latch_writes().to_vec(),
        ));
    }
    let pm_end_1 = pm.save_snapshot();
    let pm_pixels_1 = render_frame(&assets, &pm.state.scene()).argb;

    pm.restore_snapshot(&pm_snap_n);
    assert_eq!(pm.state.frame, 100);
    let mut pm_latches_2 = Vec::with_capacity(60);
    for i in 0..60 {
        pm.set_cabinet_inputs(i == 10, false, i == 30, false, false);
        pm.step();
        pm_latches_2.push((
            pm.state.frame,
            pm.state.sound_latch,
            pm.state.latch_cycles,
            pm.latch_writes().to_vec(),
        ));
    }
    let pm_end_2 = pm.save_snapshot();
    let pm_pixels_2 = render_frame(&assets, &pm.state.scene()).argb;

    assert_eq!(
        pm_end_1, pm_end_2,
        "PowerOnMachine snapshot round-trip must produce bit-identical state at N+60"
    );
    assert_eq!(
        pm_pixels_1, pm_pixels_2,
        "PowerOnMachine snapshot round-trip must produce bit-identical pixels at N+60"
    );
    assert_eq!(pm_latches_1, pm_latches_2, "PowerOnMachine snapshot round-trip must produce bit-identical audio latches across N..=N+60");
}

#[test]
fn replay_takeover_branches_p1_and_p2_and_rewinds_bit_identically() {
    let root = workspace_root();
    let fixture = root.join("tools/reference/local/captures/move-run-a");
    let scenario = root.join("tools/reference/scenarios/sf_punch_01.json");
    let assets = load_test_assets();

    let m_baseline =
        FightMachine::from_checkpoint_2450_with_program(&fixture, assets.maincpu.clone())
            .expect("checkpoint-2450 fixture");
    let m_takeover = m_baseline.clone();

    // Baseline session: straight playback from 2450 through 2550.
    let mut baseline =
        ReplayTakeoverSession::from_scenario(m_baseline, &scenario).expect("baseline session");
    assert_eq!(
        baseline.takeover_window_frames(),
        DEFAULT_TAKEOVER_WINDOW_FRAMES
    );
    while baseline.current_frame() < 2550 {
        assert!(baseline.step_replay());
    }
    let baseline_2550 = baseline.machine().save_snapshot();
    let baseline_pixels_2550 = render_frame(&assets, &baseline.machine().state.scene()).argb;

    // Session under test: replay to frame N=2490, pause, take over P1 with branched inputs,
    // rewind, then take over P2 with branched inputs, rewind, and resume replay to 2550.
    let mut session =
        ReplayTakeoverSession::from_scenario(m_takeover, &scenario).expect("takeover session");
    while session.current_frame() < 2490 {
        assert!(session.step_replay());
    }
    assert_eq!(session.current_frame(), 2490);

    // Pause at 2490: stepping while paused is a no-op.
    session.pause();
    assert_eq!(session.mode(), ReplayMode::Paused);
    assert!(!session.step_replay());
    assert_eq!(session.current_frame(), 2490);

    // Configure a 30-frame takeover window and take over P1 at frame 2490.
    session.set_takeover_window_frames(30);
    session.take_over(TakeoverPlayer::P1);
    assert!(session.has_bookmark());
    assert_eq!(
        session.mode(),
        ReplayMode::Takeover {
            player: TakeoverPlayer::P1,
            start_frame: 2490,
            window_frames: 30,
            elapsed_frames: 0,
        }
    );

    // Drive P1 backward + jumping during the 30-frame takeover window while P2 replays recorded inputs.
    for _ in 0..30 {
        session.step_with_live_input(PlayerInputs {
            directions: [false, true, false, true],
            buttons: [false; 6],
        });
    }
    assert_eq!(session.current_frame(), 2520);
    assert!(session.is_takeover_window_expired());
    let branched_p1_x = session.machine().diagnostic_telemetry().p1.center_x;

    // Stepping once more after the 30-frame window expires auto-rewinds to 2490 and steps to 2491.
    session.step_with_live_input(PlayerInputs::default());
    assert_eq!(session.current_frame(), 2491);

    // Pause while on the branched takeover timeline, then switch takeover to P2:
    // must rewind to the original 2490 bookmark rather than overwriting it at 2491.
    session.pause();
    assert_eq!(session.mode(), ReplayMode::Paused);
    session.take_over(TakeoverPlayer::P2);
    assert_eq!(session.current_frame(), 2490);
    for _ in 0..25 {
        session.step_with_live_input(PlayerInputs {
            directions: [true, false, false, false],
            buttons: [false; 6],
        });
    }
    assert_eq!(session.current_frame(), 2515);
    let branched_p2_x = session.machine().diagnostic_telemetry().p2.center_x;

    // Rewind to the 2490 takeover bookmark and resume original replay through 2550.
    assert!(session.rewind_to_takeover());
    assert_eq!(session.current_frame(), 2490);
    session.resume_replay();
    assert_eq!(session.mode(), ReplayMode::Playback);

    let mut unbranched_p1_x_at_2520 = 0;
    let mut unbranched_p2_x_at_2515 = 0;
    while session.current_frame() < 2550 {
        assert!(session.step_replay());
        if session.current_frame() == 2515 {
            unbranched_p2_x_at_2515 = session.machine().diagnostic_telemetry().p2.center_x;
        }
        if session.current_frame() == 2520 {
            unbranched_p1_x_at_2520 = session.machine().diagnostic_telemetry().p1.center_x;
        }
    }

    assert_ne!(
        branched_p1_x, unbranched_p1_x_at_2520,
        "P1 takeover with live inputs must branch P1 trajectory"
    );
    assert_ne!(
        branched_p2_x, unbranched_p2_x_at_2515,
        "P2 takeover with live inputs must branch P2 trajectory"
    );

    let resumed_2550 = session.machine().save_snapshot();
    let resumed_pixels_2550 = render_frame(&assets, &session.machine().state.scene()).argb;

    assert_eq!(
        resumed_2550, baseline_2550,
        "Rewinding from takeover and resuming replay must produce bit-identical state at N+60"
    );
    assert_eq!(
        resumed_pixels_2550, baseline_pixels_2550,
        "Rewinding from takeover and resuming replay must produce bit-identical pixels at N+60"
    );
}
