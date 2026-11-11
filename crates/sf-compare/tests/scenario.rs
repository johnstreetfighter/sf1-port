//! Scenario boundary: the runner consumes the same scenario JSON the
//! reference tooling owns — reference identity, machine configuration,
//! frame-indexed logical inputs, bounded duration, observation settings.

mod common;

use common::{scenario_json, temp_dir, write_file};

#[test]
fn loads_the_committed_fixed_fight_scenario() {
    let s = sf_compare::scenario::load_scenario(
        "../../tools/reference/scenarios/sf_fixed_fight_01.json",
    )
    .expect("the committed scenario must load");
    assert_eq!(s.scenario_id, "sf-fixed-fight-01");
    assert_eq!(s.reference_rom_set, "sf");
    assert_eq!(s.reference_mame_version, "0.289");
    assert_eq!(s.duration_frames, 5000);
    assert_eq!(s.inputs.len(), 6);
    assert_eq!(s.checkpoints.len(), 14);
    assert_eq!(s.checkpoints[6], 2280);
    // 790: static warning screen — pixels/RAM consistent there, unlike the
    // 794 transition frame (see docs/reference/first-frame.md).
    assert_eq!(s.checkpoints[0], 790);
    // One logical input resolved to its machine-level control definition.
    let right = &s.controls["p1_right"];
    assert_eq!((right.port.as_str(), right.bit), ("IN1", 0));
}

#[test]
fn rejects_input_events_pressing_unknown_controls() {
    let dir = temp_dir("scenario-unknown-control");
    let mut json: serde_json::Value = serde_json::from_str(&scenario_json(10, &[])).unwrap();
    json["inputs"] = serde_json::json!([
        { "frame": 3, "press": "p1_punch", "hold_frames": 2 }
    ]);
    write_file(&dir, "s.json", json.to_string().as_bytes());

    let err = sf_compare::scenario::load_scenario(dir.join("s.json")).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("p1_punch"),
        "must name the unknown control: {msg}"
    );
    assert!(msg.contains("frame 3"), "must locate the event: {msg}");
}

#[test]
fn rejects_overlapping_presses_of_the_same_control() {
    let dir = temp_dir("scenario-overlap");
    let mut json: serde_json::Value = serde_json::from_str(&scenario_json(10, &[])).unwrap();
    json["inputs"] = serde_json::json!([
        { "frame": 3, "press": "p1_right", "hold_frames": 4 },
        { "frame": 5, "press": "p1_right", "hold_frames": 2 }
    ]);
    write_file(&dir, "s.json", json.to_string().as_bytes());

    let err = sf_compare::scenario::load_scenario(dir.join("s.json")).unwrap_err();
    assert!(
        err.to_string().contains("overlap"),
        "must call out the overlap: {err}"
    );
}

#[test]
fn rejects_missing_required_fields() {
    let dir = temp_dir("scenario-missing-field");
    let mut json: serde_json::Value = serde_json::from_str(&scenario_json(10, &[])).unwrap();
    json.as_object_mut().unwrap().remove("duration_frames");
    write_file(&dir, "s.json", json.to_string().as_bytes());

    let err = sf_compare::scenario::load_scenario(dir.join("s.json")).unwrap_err();
    assert!(
        err.to_string().contains("duration_frames"),
        "must name the missing field: {err}"
    );
}

#[test]
fn loads_the_committed_idle_scene_scenario() {
    let s = sf_compare::scenario::load_scenario(
        "../../tools/reference/scenarios/sf_idle_scene_01.json",
    )
    .expect("the committed idle-scene scenario must load");
    assert_eq!(s.scenario_id, "sf-idle-scene-01");
    assert_eq!(s.reference_rom_set, "sf");
    assert_eq!(s.reference_mame_version, "0.289");
    // Intro inputs only; no fight inputs — the interval isolates the game's
    // own idle behavior (docs/reference/idle-scene.md).
    assert_eq!(s.inputs.len(), 3);
    assert!(s.inputs.iter().all(|e| e.frame < 2280));
    // Checkpoints cover every frame of the bounded interval 2280..=2388.
    assert_eq!(s.checkpoints.len(), 109);
    assert_eq!(s.checkpoints[0], 2280);
    assert_eq!(s.checkpoints[108], 2388);
    assert!(s.duration_frames > 2388);
    assert_eq!((s.dsw1, s.dsw2), (0xdfff, 0xffff));
}

#[test]
fn resolves_dipswitch_fields_and_port_words_in_scenarios() {
    let s1 =
        sf_compare::scenario::load_scenario("../../tools/reference/scenarios/sf_dip_smoke_01.json")
            .expect("smoke 1 loads");
    assert_eq!((s1.dsw1, s1.dsw2), (0xdfda, 0xfbff));

    let s2 =
        sf_compare::scenario::load_scenario("../../tools/reference/scenarios/sf_dip_smoke_02.json")
            .expect("smoke 2 loads");
    assert_eq!((s2.dsw1, s2.dsw2), (0xdeff, 0xffff));

    let s3 =
        sf_compare::scenario::load_scenario("../../tools/reference/scenarios/sf_dip_smoke_03.json")
            .expect("smoke 3 loads");
    assert_eq!((s3.dsw1, s3.dsw2), (0x5fff, 0xffff));

    let mut json: serde_json::Value = serde_json::from_str(&scenario_json(10, &[])).unwrap();
    json["machine"]["dipswitches"] = serde_json::json!({
        "dsw1": "0xdfda",
        "dsw2": 0xfbff
    });
    let s_words = sf_compare::scenario::parse_scenario(&json.to_string()).unwrap();
    assert_eq!((s_words.dsw1, s_words.dsw2), (0xdfda, 0xfbff));
}

#[test]
fn timed_dip_changes_are_cumulative_and_preserve_other_fields() {
    let mut json: serde_json::Value = serde_json::from_str(&scenario_json(3000, &[])).unwrap();
    json["machine"]["dipswitches"] = serde_json::json!({ "Speed": "Slow" });
    json["dip_changes"] = serde_json::json!([
        { "frame": 2500, "dipswitches": { "Freeze": "On" } },
        { "frame": 2560, "dipswitches": { "Freeze": "Off" } }
    ]);

    let s = sf_compare::scenario::parse_scenario(&json.to_string()).unwrap();
    let changes: Vec<_> = s
        .dip_changes
        .iter()
        .map(|change| (change.frame, change.dsw1, change.dsw2))
        .collect();
    assert_eq!(changes, [(2500, 0x8fff, 0xffff), (2560, 0xcfff, 0xffff)]);
}

#[test]
fn timed_dip_changes_require_strictly_increasing_frames() {
    for frames in [(2500, 2500), (2560, 2500)] {
        let mut json: serde_json::Value = serde_json::from_str(&scenario_json(3000, &[])).unwrap();
        json["dip_changes"] = serde_json::json!([
            { "frame": frames.0, "dipswitches": { "Freeze": "On" } },
            { "frame": frames.1, "dipswitches": { "Freeze": "On" } }
        ]);
        let err = sf_compare::scenario::parse_scenario(&json.to_string()).unwrap_err();
        assert!(err.to_string().contains("strictly increasing"), "{err}");
    }
}

#[test]
fn rejects_unknown_dip_names_and_out_of_range_values() {
    let mut json: serde_json::Value = serde_json::from_str(&scenario_json(10, &[])).unwrap();
    json["machine"]["dipswitches"] = serde_json::json!({ "Free Play": "On" });
    let err = sf_compare::scenario::parse_scenario(&json.to_string()).unwrap_err();
    assert!(
        err.to_string().contains("Free Play"),
        "must name the unknown DIP switch: {err}"
    );

    for bad in [
        serde_json::json!({ "dsw1": 0x10000 }),
        serde_json::json!({ "dsw1": -1 }),
        serde_json::json!({ "dsw2": "0x10000" }),
        serde_json::json!({ "dsw2": "0xfff8" }),
        serde_json::json!({ "dsw2": "0xfd7f" }),
        serde_json::json!({ "Flip Screen": "Maybe" }),
        serde_json::json!({ "Number of Countries Selected": "3" }),
        serde_json::json!({ "Game Continuation": "0" }),
        serde_json::json!("unknown-preset"),
    ] {
        let mut json: serde_json::Value = serde_json::from_str(&scenario_json(10, &[])).unwrap();
        json["machine"]["dipswitches"] = bad.clone();
        assert!(
            sf_compare::scenario::parse_scenario(&json.to_string()).is_err(),
            "must reject invalid dipswitches {bad}"
        );
    }
}
