use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn sf_dev(args: &[&str], path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sf-dev"))
        .args(args)
        .arg(path)
        .output()
        .expect("run sf-dev")
}

fn assert_success(output: Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_folder() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let folder =
        std::env::temp_dir().join(format!("sf1-authored-combo-{}-{stamp}", std::process::id()));
    fs::create_dir(&folder).unwrap();
    folder
}

fn assert_failure(output: Output, expected: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && stderr.contains(expected),
        "{stderr}"
    );
}

fn local_roms_available() -> bool {
    let available = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/reference/local/roms/sf")
        .is_dir();
    if !available {
        eprintln!("skipping: local ROM set not present");
    }
    available
}

#[test]
fn authored_ryu_command_reproduces_the_original_combo_and_replays() {
    if !local_roms_available() {
        return;
    }
    let folder = temp_folder();
    let command = folder.join("hadoken.json");
    fs::write(
        &command,
        json!({
            "move": "Hadoken",
            "strength": "light",
            "inputs": {
                "fixed": [4, 4, 5, 5, 1],
                "activation": { "button": "LP", "frame": 7, "hold": 1 }
            }
        })
        .to_string(),
    )
    .unwrap();

    assert_success(sf_dev(
        &["combo-sweep", "Ryu", folder.to_str().unwrap(), "--command"],
        &command,
    ));
    let report: Value =
        serde_json::from_slice(&fs::read(folder.join("coverage.json")).unwrap()).unwrap();
    let standing = &report["stances"][0];
    assert_eq!(standing["setup"]["defender_stance"], "standing");
    assert_eq!(standing["setup"]["separation_px"], 40);
    let pair = standing["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| pair["opener"] == "crouching LK")
        .expect("crouching light kick was tested");
    assert_eq!(pair["status"], "confirmed");
    assert_eq!(
        pair["windows"],
        json!([{ "first": -2, "last": 6, "frames": 9 }])
    );
    assert_eq!(pair["successes"][0]["press_frame"], 4);
    let route = folder.join(pair["replay"].as_str().unwrap());
    let replay: Value = serde_json::from_slice(&fs::read(&route).unwrap()).unwrap();
    assert_eq!(replay["expected"]["opener_hits"][0]["frame"], 6);
    assert_eq!(replay["expected"]["follow_up_hit"]["frame"], 19);
    let route_start = replay["route_start"].as_u64().unwrap() as usize;
    let pads = replay["inputs"].as_array().unwrap();
    assert_eq!(
        pads[route_start..route_start + 10]
            .iter()
            .map(|pad| pad["p1"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [132, 4, 4, 5, 5, 1, 1, 1, 17, 1]
    );

    fs::remove_file(&command).unwrap();
    assert_success(sf_dev(&["combo-verify"], &route));

    let modified = folder.join("modified.json");
    let mut changed_inputs = replay.clone();
    changed_inputs["inputs"][route_start + 1]["p1"] = json!(0);
    fs::write(&modified, changed_inputs.to_string()).unwrap();
    assert_failure(
        sf_dev(&["combo-verify"], &modified),
        "recorded route inputs differ",
    );

    let mut changed_hit = replay;
    changed_hit["expected"]["follow_up_hit"]["frame"] = json!(20);
    fs::write(&modified, changed_hit.to_string()).unwrap();
    assert_failure(sf_dev(&["combo-verify"], &modified), "hit mismatch");

    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn authored_command_reports_invalid_unsupported_and_tested_no_window() {
    if !local_roms_available() {
        return;
    }
    let folder = temp_folder();
    let command = folder.join("hadoken.json");
    let run = |strength: &str, fixed: Vec<u16>, activation: usize| {
        fs::write(
            &command,
            json!({
                "move": "Hadoken",
                "strength": strength,
                "inputs": {
                    "fixed": fixed,
                    "activation": { "button": "LP", "frame": activation, "hold": 1 }
                }
            })
            .to_string(),
        )
        .unwrap();
        sf_dev(
            &["combo-sweep", "Ryu", folder.to_str().unwrap(), "--command"],
            &command,
        )
    };
    let crouching_lk = || {
        let report: Value =
            serde_json::from_slice(&fs::read(folder.join("coverage.json")).unwrap()).unwrap();
        report["stances"][0]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pair| pair["opener"] == "crouching LK")
            .unwrap()
            .clone()
    };

    assert_failure(run("light", vec![0], 89), "activation.frame");
    assert!(!folder.join("coverage.json").exists());

    assert_success(run("light", vec![0], 7));
    let unsupported = crouching_lk();
    assert_eq!(unsupported["status"], "unsupported");
    assert!(unsupported["reason"]
        .as_str()
        .unwrap()
        .contains("activation.frame"));

    assert_success(run("light", vec![4, 4, 5, 5, 1, 1, 1, 33, 1], 7));
    let independent = crouching_lk();
    assert_eq!(independent["status"], "unsupported");
    assert!(independent["reason"]
        .as_str()
        .unwrap()
        .contains("without the movable button"));

    assert_success(run("heavy", vec![4, 4, 5, 5, 1], 7));
    let wrong_strength = crouching_lk();
    assert_eq!(wrong_strength["status"], "unsupported");
    assert!(wrong_strength["reason"]
        .as_str()
        .unwrap()
        .contains("different strength"));

    let mut delayed = vec![0; 30];
    delayed.extend([4, 4, 5, 5, 1]);
    assert_success(run("light", delayed, 37));
    let no_window = crouching_lk();
    assert_eq!(no_window["status"], "no_success");
    assert!(no_window["attempts"].as_u64().unwrap() > 0);

    fs::remove_dir_all(folder).unwrap();
}
