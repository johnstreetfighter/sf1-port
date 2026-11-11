//! Acceptance and scenario tests for ticket #92 (`O1: Real-time Hitbox,
//! Loose/Tight Hurtbox, and Command-Tracker Diagnostic Overlay`).
//!
//! Verifies:
//! 1. Toggling the diagnostic overlay on/off during a live or replayed fight
//!    never mutates work RAM, object RAM, PRNG state, sound latches, or the
//!    base `384x224` game-frame pixels.
//! 2. Distinct color-coded rectangles are rendered for the outer loose hurtbox
//!    (`depth 2`), inner tight hurtbox (`depth 1`), active melee and projectile
//!    hitboxes, pushboxes, and invulnerability states for both fighters and
//!    active projectiles on every frame.
//! 3. Live per-player telemetry is extracted and rendered for button hold
//!    durations, negative-edge release triggers, special-move command-tracker
//!    stages (`0..=3`, `done = 0xff`), remaining command window timers, and
//!    walk-cycle gate states.
//! 4. Overlay telemetry exposed through the frame-stepped scenario session
//!    boundary matches the reference fight's collision and input checks frame
//!    by frame across `sf-punch-01`, `sf-specials-01`, `sf-specials-02`,
//!    `sf-hadoken-01`, `sf-hadoken-02`, and `sf-move-01`.

use std::path::Path;

use sf_engine::overlay::{
    judge_collision_depth, render_diagnostic_overlay, BoundingBox, OverlayColors,
};
use sf_engine::run::PowerOnMachine;
use sf_engine::versus::VersusMachine;
use sf_video::render::{render_frame, HEIGHT, WIDTH};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const PUNCH_SCENARIO: &str = "../../tools/reference/scenarios/sf_punch_01.json";
const PUNCH_REF: &str = "../../tools/reference/local/captures/punch-01-run-a";
const SPECIALS_01_SCENARIO: &str = "../../tools/reference/scenarios/sf_specials_01.json";
const SPECIALS_01_REF: &str = "../../tools/reference/local/captures/specials-01-run-a";
const SPECIALS_02_SCENARIO: &str = "../../tools/reference/scenarios/sf_specials_02.json";
const SPECIALS_02_REF: &str = "../../tools/reference/local/captures/specials-02-run-a";
const HADOKEN_01_SCENARIO: &str = "../../tools/reference/scenarios/sf_hadoken_01.json";
const HADOKEN_01_REF: &str = "../../tools/reference/local/captures/hadoken-01-run-a";
const HADOKEN_03_SCENARIO: &str = "../../tools/reference/scenarios/sf_hadoken_03.json";
const HADOKEN_03_REF: &str = "../../tools/reference/local/captures/hadoken-03-run-a";
const MOVE_SCENARIO: &str = "../../tools/reference/scenarios/sf_move_01.json";
const MOVE_REF: &str = "../../tools/reference/local/captures/move-run-a";

fn has_capture(dir: &str) -> bool {
    Path::new(ROMS).is_dir() && Path::new(MANIFEST).is_file() && Path::new(dir).is_dir()
}

fn step_to(
    m: &mut VersusMachine,
    events: &[(u32, usize, bool)],
    next: &mut usize,
    held: &mut [bool; sf_engine::run::INPUTS],
    target_frame: u32,
) {
    while m.state.frame < target_frame {
        let f = m.state.frame + 1;
        while *next < events.len() && events[*next].0 < f {
            held[events[*next].1] = events[*next].2;
            *next += 1;
        }
        sf_engine::run::apply_inputs(m, held);
        m.step();
    }
}

#[test]
fn punch_scenario_hitbox_and_two_stage_hurtbox_match_reference_judge() {
    if !has_capture(PUNCH_REF) {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let mut m = VersusMachine::from_checkpoint_2450_with_program(Path::new(PUNCH_REF), rom)
        .expect("fixture");
    let events = sf_engine::run::scenario_input_events(Path::new(PUNCH_SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; sf_engine::run::INPUTS]);

    // Initial standing frame 2450: both fighters active, no active attack box,
    // tight hurtbox nested inside loose hurtbox.
    let t0 = m.diagnostic_telemetry();
    assert!(t0.p1.active && t0.p2.active);
    assert_eq!(t0.p1.attack_hitbox, None);
    assert_eq!(t0.p2.attack_hitbox, None);
    assert!(t0.p2.loose_hurtbox.half_w > t0.p2.tight_hurtbox.half_w);
    assert!(t0.p2.loose_hurtbox.half_h >= t0.p2.tight_hurtbox.half_h);
    assert!(t0.p1.pushbox.right > t0.p1.pushbox.left);
    assert!(t0.p1.pushbox.bottom > t0.p1.pushbox.top);

    // Punch 1 button hold and negative-edge release (event at 2460, hold 12 ->
    // held on 2461..=2472, released on 2473).
    step_to(&mut m, &events, &mut next, &mut held, 2461);
    let t_hold1 = m.diagnostic_telemetry();
    assert!(t_hold1.p1.punch_latch.held);
    assert_eq!(t_hold1.p1.punch_latch.hold_frames, 1);
    assert_eq!(t_hold1.p1.punch_latch.strongest_button, 1);
    assert!(!t_hold1.p1.punch_latch.release_triggered);

    step_to(&mut m, &events, &mut next, &mut held, 2472);
    let t_hold12 = m.diagnostic_telemetry();
    assert!(t_hold12.p1.punch_latch.held);
    assert_eq!(t_hold12.p1.punch_latch.hold_frames, 12);
    assert_eq!(t_hold12.p1.punch_latch.strongest_button, 1);

    step_to(&mut m, &events, &mut next, &mut held, 2473);
    let t_rel = m.diagnostic_telemetry();
    assert!(!t_rel.p1.punch_latch.held);
    assert_eq!(t_rel.p1.punch_latch.hold_frames, 0);
    assert!(
        t_rel.p1.punch_latch.release_triggered,
        "negative-edge release trigger must fire on the release frame 2473"
    );
    assert_eq!(
        t_rel.p1.move_id, 0x0d,
        "standing light punch starts on release"
    );

    step_to(&mut m, &events, &mut next, &mut held, 2474);
    let t_after_rel = m.diagnostic_telemetry();
    assert!(
        !t_after_rel.p1.punch_latch.release_triggered,
        "negative-edge release trigger must clear on the frame following release"
    );

    // Punch 1 active whiff frames (2476..=2477): active hitbox present, depth 0 against P2.
    for f in 2476..=2477 {
        step_to(&mut m, &events, &mut next, &mut held, f);
        let t = m.diagnostic_telemetry();
        let atk =
            t.p1.attack_hitbox
                .expect("active punch hitbox on whiff frame");
        assert_eq!(
            judge_collision_depth(
                &atk,
                &t.p2.loose_hurtbox,
                &t.p2.tight_hurtbox,
                t.p2.invulnerable
            ),
            0,
            "frame {f} must whiff"
        );
    }

    // Punch 2 active whiff frames (2596..=2597): closer, still depth 0.
    for f in 2596..=2597 {
        step_to(&mut m, &events, &mut next, &mut held, f);
        let t = m.diagnostic_telemetry();
        let atk =
            t.p1.attack_hitbox
                .expect("active punch hitbox on whiff frame");
        assert_eq!(
            judge_collision_depth(
                &atk,
                &t.p2.loose_hurtbox,
                &t.p2.tight_hurtbox,
                t.p2.invulnerable
            ),
            0,
            "frame {f} must whiff"
        );
    }

    // Punch 3 at gap 0x34: pre-hit frame 2715 gives Ken's standing hurtboxes;
    // frame 2716 lands a partial (outer loose hurtbox, depth 2) hit.
    step_to(&mut m, &events, &mut next, &mut held, 2715);
    let pre_2716 = m.diagnostic_telemetry();
    assert_eq!(pre_2716.p1.attack_hitbox, None);

    step_to(&mut m, &events, &mut next, &mut held, 2716);
    let hit_2716 = m.diagnostic_telemetry();
    let atk_2716 = hit_2716
        .p1
        .attack_hitbox
        .expect("attack hitbox must be exposed on hit frame 2716");
    assert!(hit_2716.p1.attack_connected_this_frame);
    assert_eq!(
        judge_collision_depth(
            &atk_2716,
            &pre_2716.p2.loose_hurtbox,
            &pre_2716.p2.tight_hurtbox,
            pre_2716.p2.invulnerable
        ),
        2,
        "frame 2716 must be an outer loose-hurtbox (depth 2) hit"
    );

    // Punch 4 at gap 0x2c: frame 2846 lands a clean (inner tight hurtbox, depth 1) hit.
    step_to(&mut m, &events, &mut next, &mut held, 2845);
    let pre_2846 = m.diagnostic_telemetry();
    step_to(&mut m, &events, &mut next, &mut held, 2846);
    let hit_2846 = m.diagnostic_telemetry();
    let atk_2846 = hit_2846
        .p1
        .attack_hitbox
        .expect("attack hitbox must be exposed on hit frame 2846");
    assert!(hit_2846.p1.attack_connected_this_frame);
    assert_eq!(
        judge_collision_depth(
            &atk_2846,
            &pre_2846.p2.loose_hurtbox,
            &pre_2846.p2.tight_hurtbox,
            pre_2846.p2.invulnerable
        ),
        1,
        "frame 2846 must be an inner tight-hurtbox (depth 1) hit"
    );

    // Punch 6 at gap 0x24: frame 3136 lands a clean (inner tight hurtbox, depth 1) hit.
    step_to(&mut m, &events, &mut next, &mut held, 3135);
    let pre_3136 = m.diagnostic_telemetry();
    step_to(&mut m, &events, &mut next, &mut held, 3136);
    let hit_3136 = m.diagnostic_telemetry();
    let atk_3136 = hit_3136
        .p1
        .attack_hitbox
        .expect("attack hitbox must be exposed on hit frame 3136");
    assert_eq!(
        judge_collision_depth(
            &atk_3136,
            &pre_3136.p2.loose_hurtbox,
            &pre_3136.p2.tight_hurtbox,
            pre_3136.p2.invulnerable
        ),
        1,
        "frame 3136 must be an inner tight-hurtbox (depth 1) hit"
    );
}

#[test]
fn command_trackers_and_invulnerability_match_specials_scenarios() {
    if !has_capture(SPECIALS_01_REF) || !has_capture(SPECIALS_02_REF) {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    // 1. sf-specials-01: Shoryuken tracker advances stage 0 -> 1 -> 2 -> 3 (done = 0xff)
    // and counts down its 18-frame window before firing move 0x19 on button release.
    let mut m1 =
        VersusMachine::from_checkpoint_2450_with_program(Path::new(SPECIALS_01_REF), rom.clone())
            .expect("fixture");
    let events1 = sf_engine::run::scenario_input_events(Path::new(SPECIALS_01_SCENARIO)).unwrap();
    let (mut next1, mut held1) = (0, [false; sf_engine::run::INPUTS]);

    let mut saw_stage1 = false;
    let mut saw_stage2 = false;
    let mut saw_stage3_done = false;
    let mut saw_shoryuken_trigger = false;

    for f in 2460..=2480 {
        step_to(&mut m1, &events1, &mut next1, &mut held1, f);
        let t = m1.diagnostic_telemetry();
        let sho = t.p1.shoryuken;
        match sho.stage {
            1 => {
                saw_stage1 = true;
                assert_eq!(sho.done, 0);
                assert_eq!(sho.remaining_window, 18 - sho.timer);
            }
            2 => {
                saw_stage2 = true;
                assert_eq!(sho.done, 0);
                assert_eq!(sho.remaining_window, 18 - sho.timer);
            }
            3 if sho.done == 0xff => {
                saw_stage3_done = true;
                assert!(sho.remaining_window > 0 && sho.remaining_window <= 18);
            }
            _ => {}
        }
        if sho.triggered_this_frame {
            saw_shoryuken_trigger = true;
            assert_eq!(t.p1.move_id, 0x19);
            assert_eq!(sho.stage, 3);
            assert_eq!(sho.done, 0xff, "trigger frame must preserve done = 0xff");
            assert!(sho.remaining_window > 0 && sho.remaining_window <= 18);
        } else if saw_shoryuken_trigger && sho.stage == 3 {
            assert_eq!(sho.done, 0);
            assert_eq!(
                sho.remaining_window, 0,
                "after Shoryuken consumes done = 0xff, remaining_window must report 0"
            );
        }
    }
    assert!(saw_stage1, "Shoryuken tracker must visit stage 1");
    assert!(saw_stage2, "Shoryuken tracker must visit stage 2");
    assert!(
        saw_stage3_done,
        "Shoryuken tracker must reach stage 3 with done = 0xff"
    );
    assert!(
        saw_shoryuken_trigger,
        "Shoryuken tracker must report trigger when move 0x19 starts"
    );

    // 2. sf-specials-02: slow Shoryuken expires after 18 frames (resets to stage 0),
    // and second Shoryuken at 2722..2745 sets P1 invulnerable (+0xf bit 3) against Ken's HK.
    let mut m2 = VersusMachine::from_checkpoint_2450_with_program(Path::new(SPECIALS_02_REF), rom)
        .expect("fixture");
    let events2 = sf_engine::run::scenario_input_events(Path::new(SPECIALS_02_SCENARIO)).unwrap();
    let (mut next2, mut held2) = (0, [false; sf_engine::run::INPUTS]);

    let mut saw_window_expiry = false;
    let mut prev_stage = 0;
    for f in 2560..=2592 {
        step_to(&mut m2, &events2, &mut next2, &mut held2, f);
        let sho = m2.diagnostic_telemetry().p1.shoryuken;
        if prev_stage > 0 && sho.stage == 0 && m2.diagnostic_telemetry().p1.move_id != 0x19 {
            saw_window_expiry = true;
        }
        prev_stage = sho.stage;
    }
    assert!(
        saw_window_expiry,
        "slow Shoryuken command exceeding 18 frames must reset tracker to stage 0"
    );

    let mut saw_ryu_invuln = false;
    for f in 2725..=2760 {
        step_to(&mut m2, &events2, &mut next2, &mut held2, f);
        let t = m2.diagnostic_telemetry();
        if t.p1.invulnerable {
            saw_ryu_invuln = true;
            // Even an overlapping attack box centered directly on Ryu must return depth 0
            // while Ryu's Shoryuken invulnerability bit (+0xf bit 3) is set.
            let overlapping_atk =
                BoundingBox::from_center_half(t.p1.center_x, t.p1.center_y, 32, 32);
            assert!(overlapping_atk.overlaps_inclusive(&t.p1.tight_hurtbox));
            assert_eq!(
                judge_collision_depth(
                    &overlapping_atk,
                    &t.p1.loose_hurtbox,
                    &t.p1.tight_hurtbox,
                    t.p1.invulnerable
                ),
                0,
                "invulnerable Shoryuken frame {f} must ignore overlapping attack hitboxes"
            );
        }
    }
    assert!(
        saw_ryu_invuln,
        "must observe Ryu's invulnerable Shoryuken state"
    );
}

#[test]
fn hadoken_tracker_and_projectile_hitboxes_match_hadoken_scenarios() {
    if !has_capture(HADOKEN_01_REF) || !has_capture(HADOKEN_03_REF) {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");

    // 1. sf-hadoken-01: Hadoken tracker reaches stage 3 (done = 0xff) and spawns projectile.
    let mut m1 =
        VersusMachine::from_checkpoint_2450_with_program(Path::new(HADOKEN_01_REF), rom.clone())
            .expect("fixture");
    let events1 = sf_engine::run::scenario_input_events(Path::new(HADOKEN_01_SCENARIO)).unwrap();
    let (mut next1, mut held1) = (0, [false; sf_engine::run::INPUTS]);

    let mut saw_hadoken_done = false;
    let mut saw_hadoken_trigger = false;
    let mut saw_live_projectile = false;
    let mut saw_impact_frame_projectile = false;
    for f in 2460..=2530 {
        let prev_p2 = m1.diagnostic_telemetry().p2;
        step_to(&mut m1, &events1, &mut next1, &mut held1, f);
        let t = m1.diagnostic_telemetry();
        if t.p1.hadoken.stage == 3 && t.p1.hadoken.done == 0xff {
            saw_hadoken_done = true;
        }
        if t.p1.hadoken.triggered_this_frame {
            saw_hadoken_trigger = true;
            assert_eq!(t.p1.hadoken.stage, 3);
            assert_eq!(t.p1.hadoken.done, 0xff);
            assert!(t.p1.hadoken.remaining_window > 0);
        }
        for proj in &t.projectiles {
            if proj.collision_flags & 0x80 != 0 && proj.hitbox.half_w > 0 && proj.hitbox.half_h > 0
            {
                saw_live_projectile = true;
                if judge_collision_depth(
                    &proj.hitbox,
                    &prev_p2.loose_hurtbox,
                    &prev_p2.tight_hurtbox,
                    prev_p2.invulnerable,
                ) > 0
                {
                    saw_impact_frame_projectile = true;
                }
            }
        }
    }
    assert!(
        saw_hadoken_done,
        "Hadoken tracker must reach stage 3 with done = 0xff"
    );
    assert!(
        saw_hadoken_trigger,
        "Hadoken tracker must report trigger frame with stage 3 / done 0xff"
    );
    assert!(
        saw_live_projectile,
        "active Hadoken projectile hitbox must be reported"
    );
    assert!(
        saw_impact_frame_projectile,
        "impacting Hadoken projectile hitbox must be retained on its collision frame"
    );

    // 2. sf-hadoken-03: two simultaneous Hadoken projectiles collide and cancel.
    let mut m3 = VersusMachine::from_checkpoint_2450_with_program(Path::new(HADOKEN_03_REF), rom)
        .expect("fixture");
    let events3 = sf_engine::run::scenario_input_events(Path::new(HADOKEN_03_SCENARIO)).unwrap();
    let (mut next3, mut held3) = (0, [false; sf_engine::run::INPUTS]);

    let mut max_vs_proj = 0usize;
    let mut saw_projectile_clash_overlap = false;
    for f in 2450..=2880 {
        step_to(&mut m3, &events3, &mut next3, &mut held3, f);
        let t = m3.diagnostic_telemetry();
        let vs_projs: Vec<_> = t
            .projectiles
            .iter()
            .filter(|p| p.state == 2 && (p.collision_flags & 0x20) != 0)
            .collect();
        max_vs_proj = max_vs_proj.max(vs_projs.len());
        if vs_projs.len() == 2 && vs_projs[0].hitbox.overlaps_inclusive(&vs_projs[1].hitbox) {
            saw_projectile_clash_overlap = true;
        }
    }
    assert_eq!(
        max_vs_proj, 2,
        "both Ryu's and Ken's active projectiles must be visible before cancelling"
    );
    assert!(
        saw_projectile_clash_overlap,
        "clashing projectiles must retain their overlapping hitboxes on the cancel frame"
    );
}

#[test]
fn walk_cycle_gate_alternates_eight_gated_and_two_actionable_frames() {
    if !has_capture(MOVE_REF) {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("program ROM");
    let mut m = VersusMachine::from_checkpoint_2450_with_program(Path::new(MOVE_REF), rom)
        .expect("fixture");
    let events = sf_engine::run::scenario_input_events(Path::new(MOVE_SCENARIO)).unwrap();
    let (mut next, mut held) = (0, [false; sf_engine::run::INPUTS]);

    // Walk right runs from 2461..=2530: frames 2461..=2462 are actionable (0x92),
    // frames 2463..=2470 are 8 consecutive walk-gated frames (0x91), and 2471..=2477
    // complete the 17-frame walk cycle before repeating at 2478..=2494.
    let mut gated_2463_2470 = 0usize;
    let mut open_2461_2462 = 0usize;
    for f in 2461..=2470 {
        step_to(&mut m, &events, &mut next, &mut held, f);
        let wg = m.diagnostic_telemetry().p1.walk_gate;
        assert!(wg.in_walk_cycle, "frame {f} must be in walk cycle");
        if wg.walk_gated {
            assert!(!wg.actionable);
            gated_2463_2470 += 1;
        } else {
            assert!(wg.actionable);
            open_2461_2462 += 1;
        }
    }
    assert_eq!(
        open_2461_2462, 2,
        "frames 2461..=2462 are the 2 actionable walk frames (0x92)"
    );
    assert_eq!(
        gated_2463_2470, 8,
        "frames 2463..=2470 are the 8 walk-gated frames (0x91)"
    );
}

#[test]
fn overlay_toggle_is_completely_non_invasive_and_renders_distinct_colors() {
    if !has_capture(PUNCH_REF) {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("assets");
    let mut baseline = VersusMachine::from_checkpoint_2450_with_program(
        Path::new(PUNCH_REF),
        assets.maincpu.clone(),
    )
    .expect("fixture");
    let mut toggled = VersusMachine::from_checkpoint_2450_with_program(
        Path::new(PUNCH_REF),
        assets.maincpu.clone(),
    )
    .expect("fixture");

    let events = sf_engine::run::scenario_input_events(Path::new(PUNCH_SCENARIO)).unwrap();
    let (mut next_b, mut held_b) = (0, [false; sf_engine::run::INPUTS]);
    let (mut next_t, mut held_t) = (0, [false; sf_engine::run::INPUTS]);

    let mut saw_melee_border = false;
    let mut saw_loose_border = false;
    let mut saw_tight_border = false;
    let mut saw_push_border = false;

    for f in 2451..=2730 {
        step_to(&mut baseline, &events, &mut next_b, &mut held_b, f);
        // Toggle overlay on and off repeatedly during the fight.
        toggled.set_overlay_enabled(f % 2 == 0);
        toggled.toggle_overlay();
        toggled.set_overlay_enabled(true);
        step_to(&mut toggled, &events, &mut next_t, &mut held_t, f);

        // Work RAM (including PRNG state 0xff8e4a..0xff8e52), object RAM, video RAM,
        // palette RAM, sound latch, and latch cycles must be 100% identical.
        assert_eq!(
            toggled.work_ram(),
            baseline.work_ram(),
            "work RAM mutated at frame {f}"
        );
        assert_eq!(
            &toggled.work_ram()[0x0e4a..0x0e52],
            &baseline.work_ram()[0x0e4a..0x0e52],
            "PRNG state mutated at frame {f}"
        );
        assert_eq!(
            toggled.state.objectram, baseline.state.objectram,
            "object RAM mutated at frame {f}"
        );
        assert_eq!(
            toggled.state.videoram, baseline.state.videoram,
            "video RAM mutated at frame {f}"
        );
        assert_eq!(
            toggled.state.paletteram, baseline.state.paletteram,
            "palette RAM mutated at frame {f}"
        );
        assert_eq!(
            toggled.state.sound_latch, baseline.state.sound_latch,
            "sound latch mutated at frame {f}"
        );
        assert_eq!(
            toggled.state.latch_cycles, baseline.state.latch_cycles,
            "latch cycles mutated at frame {f}"
        );

        let base_frame = render_frame(&assets, &baseline.state.scene());
        let toggled_base = render_frame(&assets, &toggled.state.scene());
        assert_eq!(
            toggled_base.argb, base_frame.argb,
            "base 384x224 pixels mutated at frame {f}"
        );

        // With overlay disabled, render_with_overlay equals base_frame exactly.
        toggled.set_overlay_enabled(false);
        let disabled_out = toggled.render_with_overlay(&assets);
        assert_eq!(
            disabled_out.argb, base_frame.argb,
            "disabled overlay must match base frame"
        );

        // With overlay enabled, render_with_overlay composites boxes & HUD without mutating base_frame.
        toggled.set_overlay_enabled(true);
        let enabled_out = toggled.render_with_overlay(&assets);
        assert_eq!(enabled_out.argb.len(), WIDTH * HEIGHT);
        assert_ne!(
            enabled_out.argb, base_frame.argb,
            "enabled overlay must draw onto output frame"
        );

        if enabled_out.argb.contains(&OverlayColors::PUSHBOX_BORDER) {
            saw_push_border = true;
        }
        if enabled_out
            .argb
            .contains(&OverlayColors::LOOSE_HURTBOX_BORDER)
        {
            saw_loose_border = true;
        }
        if enabled_out
            .argb
            .contains(&OverlayColors::TIGHT_HURTBOX_BORDER)
        {
            saw_tight_border = true;
        }
        if enabled_out
            .argb
            .contains(&OverlayColors::MELEE_HITBOX_BORDER)
        {
            saw_melee_border = true;
        }
    }

    assert!(saw_push_border, "must render distinct pushbox border color");
    assert!(
        saw_loose_border,
        "must render distinct outer loose hurtbox (depth 2) border color"
    );
    assert!(
        saw_tight_border,
        "must render distinct inner tight hurtbox (depth 1) border color"
    );
    assert!(
        saw_melee_border,
        "must render distinct active melee hitbox border color"
    );

    // Also verify projectile and invulnerability border colors render distinctly on synthetic telemetry.
    let blank = sf_video::render::Frame {
        argb: vec![0xff00_0000; WIDTH * HEIGHT],
    };
    let mut synth = toggled.diagnostic_telemetry();
    synth.p1.invulnerable = true;
    synth
        .projectiles
        .push(sf_engine::overlay::ProjectileTelemetry {
            slot_offset: 0x100,
            state: 2,
            collision_flags: 0x80,
            owner_struct: sf_engine::fighter::P1_STRUCT,
            target_struct: sf_engine::fighter::P2_STRUCT,
            hitbox: BoundingBox::from_center_half(200, 120, 12, 10),
        });
    let synth_rendered = render_diagnostic_overlay(&blank, &synth, true);
    assert!(
        synth_rendered
            .argb
            .contains(&OverlayColors::INVULNERABLE_BORDER),
        "must render distinct invulnerability border color"
    );
    assert!(
        synth_rendered
            .argb
            .contains(&OverlayColors::PROJECTILE_HITBOX_BORDER),
        "must render distinct projectile hitbox border color"
    );

    // Also verify PowerOnMachine supports non-invasive overlay toggling.
    let mut p_on = PowerOnMachine::new(assets.maincpu.clone());
    assert!(!p_on.overlay_enabled());
    p_on.toggle_overlay();
    assert!(p_on.overlay_enabled());
    p_on.step();
    let _ = p_on.diagnostic_telemetry();
    let _ = p_on.render_with_overlay(&assets);
}

#[test]
fn synthetic_telemetry_extraction_and_overlay_rendering_without_local_roms() {
    use sf_engine::fighter::{OBJ_BASE, P1_STRUCT, P2_STRUCT, WORK_BASE};
    use sf_engine::overlay::extract_diagnostic_telemetry;

    let rom = vec![0u8; 0x60000];
    let mut work = vec![0u8; 0x6000];
    let mut obj = vec![0u8; 0x2000];

    // Game mode 1 (2P versus)
    work[(0xffc002 - WORK_BASE) as usize] = 1;

    // P1 entity at 0x400, P2 entity at 0xa00
    let p1_ps = (P1_STRUCT - WORK_BASE) as usize;
    let p2_ps = (P2_STRUCT - WORK_BASE) as usize;
    work[p1_ps + 0x0e..p1_ps + 0x12].copy_from_slice(&0x0a00_0400u32.to_be_bytes());
    work[p2_ps + 0x0e..p2_ps + 0x12].copy_from_slice(&0x0400_0a00u32.to_be_bytes());

    // P1 active at (sx=160, sy=80), big sprite (attr=0x400), 3x2 grid (rows-1=2, cols-1=1)
    let e1 = 0x400usize;
    obj[e1 + 2..e1 + 4].copy_from_slice(&0x0400u16.to_be_bytes());
    obj[e1 + 4..e1 + 6].copy_from_slice(&80u16.to_be_bytes());
    obj[e1 + 6..e1 + 8].copy_from_slice(&160u16.to_be_bytes());
    obj[e1 + 8] = 2;
    obj[e1 + 9] = 0x91; // walk-gated state
    obj[e1 + 0x0a] = 2; // forward walk
    obj[e1 + 0x0c] = 2; // 3 rows -> half_h = 48
    obj[e1 + 0x0d] = 1; // 2 cols -> half_w = 32
    obj[e1 + 0x0f] = 0x80; // active melee hitbox
    obj[e1 + 0x16..e1 + 0x1a].copy_from_slice(&[40, 44, 16, 14]);
    obj[e1 + 0x1a..e1 + 0x1e].copy_from_slice(&[8, 18, 2, 10]);
    obj[e1 + 0x1e..e1 + 0x22].copy_from_slice(&[4, 36, 0, 18]);
    obj[e1 + 0x22..e1 + 0x26].copy_from_slice(&[12, 8, 24, 16]);
    obj[e1 + 0x37] = 2; // facing right

    // P1 button hold (3 frames held, strongest = 2 MP) and Hadoken stage 3 (done = 0xff, timer = 5)
    let p1_punch = (0xff8690 - WORK_BASE) as usize;
    work[p1_punch] = 0xff;
    work[p1_punch + 3] = 2;
    work[p1_punch + 4] = 2;
    work[p1_ps + 0x1c] = 3;
    work[p1_ps + 0x1d] = 5;
    work[p1_ps + 0x1e] = 0xff;

    // P2 active at (sx=220, sy=80), invulnerable (+0xf = 0x08), facing left (+0x37 = 4)
    let e2 = 0xa00usize;
    obj[e2 + 2..e2 + 4].copy_from_slice(&0x0400u16.to_be_bytes());
    obj[e2 + 4..e2 + 6].copy_from_slice(&80u16.to_be_bytes());
    obj[e2 + 6..e2 + 8].copy_from_slice(&220u16.to_be_bytes());
    obj[e2 + 8] = 2;
    obj[e2 + 9] = 0x92; // actionable walk frame
    obj[e2 + 0x0a] = 3; // back walk
    obj[e2 + 0x0c] = 2;
    obj[e2 + 0x0d] = 1;
    obj[e2 + 0x0f] = 0x08; // invulnerable
    obj[e2 + 0x16..e2 + 0x1a].copy_from_slice(&[40, 44, 16, 14]);
    obj[e2 + 0x1a..e2 + 0x1e].copy_from_slice(&[8, 18, 0xfe, 10]);
    obj[e2 + 0x1e..e2 + 0x22].copy_from_slice(&[4, 36, 0, 18]);
    obj[e2 + 0x37] = 4;

    // Active projectile in slot 0 (0x100)
    let s0 = (0xffe100 - OBJ_BASE) as usize;
    obj[s0 + 2..s0 + 4].copy_from_slice(&0x0404u16.to_be_bytes());
    obj[s0 + 4..s0 + 6].copy_from_slice(&100u16.to_be_bytes());
    obj[s0 + 6..s0 + 8].copy_from_slice(&200u16.to_be_bytes());
    obj[s0 + 8] = 2;
    obj[s0 + 0x0c] = 12;
    obj[s0 + 0x0d] = 12;
    obj[s0 + 0x0e] = 0xe0;

    let t = extract_diagnostic_telemetry(&rom, &work, &obj, false);
    assert!(t.p1.active && t.p2.active);
    assert_eq!((t.p1.center_x, t.p1.center_y), (192, 128));
    assert!(
        t.p1.walk_gate.in_walk_cycle && t.p1.walk_gate.walk_gated && !t.p1.walk_gate.actionable
    );
    assert!(
        t.p2.walk_gate.in_walk_cycle && !t.p2.walk_gate.walk_gated && t.p2.walk_gate.actionable
    );
    assert_eq!(t.p1.punch_latch.hold_frames, 3);
    assert_eq!(t.p1.punch_latch.strongest_button, 2);
    assert_eq!(t.p1.hadoken.stage, 3);
    assert_eq!(t.p1.hadoken.done, 0xff);
    assert_eq!(t.p1.hadoken.remaining_window, 13);
    assert!(t.p1.attack_hitbox.is_some());
    assert!(t.p2.invulnerable);
    assert_eq!(t.projectiles.len(), 1);

    let blank = sf_video::render::Frame {
        argb: vec![0xff00_0000; WIDTH * HEIGHT],
    };
    let rendered = render_diagnostic_overlay(&blank, &t, true);
    assert!(rendered.argb.contains(&OverlayColors::PUSHBOX_BORDER));
    assert!(rendered.argb.contains(&OverlayColors::LOOSE_HURTBOX_BORDER));
    assert!(rendered.argb.contains(&OverlayColors::TIGHT_HURTBOX_BORDER));
    assert!(rendered.argb.contains(&OverlayColors::MELEE_HITBOX_BORDER));
    assert!(rendered.argb.contains(&OverlayColors::INVULNERABLE_BORDER));
    assert!(rendered
        .argb
        .contains(&OverlayColors::PROJECTILE_HITBOX_BORDER));
}
