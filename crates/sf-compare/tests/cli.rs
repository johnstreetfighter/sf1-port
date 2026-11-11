//! CLI seam: the documented invocation validates local input data, runs
//! the comparison, prints the report, and encodes the result in the exit
//! code (0 match, 1 mismatch, 2 input/validation failure).

mod common;

use std::process::Command;

use common::{manifest_json, scenario_json, temp_dir, write_file, CaptureBuilder};

fn sf_compare() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sf-compare"))
}

/// Writes a scenario file plus reference/candidate captures built by
/// `build_ref`/`build_cand`-style closures; returns the paths.
fn setup(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let scenario = dir.join("scenario.json");
    std::fs::write(&scenario, scenario_json(6, &[2])).unwrap();
    let mut b = CaptureBuilder::new(6);
    b.solid_pixels(2, [1, 2, 3, 255]);
    b.write(&dir.join("ref"));
    b.write(&dir.join("cand"));
    (scenario, dir.join("ref"), dir.join("cand"))
}

#[test]
fn matching_captures_exit_zero() {
    let dir = temp_dir("cli-match");
    let (scenario, reference, candidate) = setup(&dir);

    let out = sf_compare()
        .args([
            "compare",
            "--scenario",
            scenario.to_str().unwrap(),
            "--reference",
            reference.to_str().unwrap(),
            "--candidate",
            candidate.to_str().unwrap(),
            "--candidate-kind",
            "reference",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("MATCH"), "{stdout}");
    assert!(stdout.contains("self-check"), "{stdout}");
}

#[test]
fn diverging_captures_exit_one_with_a_first_mismatch_report() {
    let dir = temp_dir("cli-mismatch");
    let (scenario, reference, candidate) = setup(&dir);
    // Change one checkpoint pixel in the candidate.
    let mut pixels = [[1u8, 2, 3, 255]].repeat(4 * 2);
    pixels[0] = [0, 0, 0, 255];
    let flat: Vec<u8> = pixels.into_iter().flatten().collect();
    let mut b = CaptureBuilder::new(6);
    b.checkpoints.push((2, flat));
    b.write(&candidate);

    let out = sf_compare()
        .args([
            "compare",
            "--scenario",
            scenario.to_str().unwrap(),
            "--reference",
            reference.to_str().unwrap(),
            "--candidate",
            candidate.to_str().unwrap(),
            "--candidate-kind",
            "reference",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("MISMATCH at frame 2"), "{stdout}");
}

#[test]
fn invalid_roms_exit_two_and_name_the_entry() {
    let dir = temp_dir("cli-bad-roms");
    let (scenario, reference, candidate) = setup(&dir);
    let roms = dir.join("roms");
    std::fs::create_dir_all(&roms).unwrap();
    let good = vec![0x11u8; 32];
    write_file(
        &dir,
        "manifest.json",
        manifest_json("sf", &[("sf-02.7k", "audiocpu", 0, &good)]).as_bytes(),
    );
    write_file(&roms, "sf-02.7k", &[0x99u8; 32]);

    let out = sf_compare()
        .args([
            "compare",
            "--scenario",
            scenario.to_str().unwrap(),
            "--reference",
            reference.to_str().unwrap(),
            "--candidate",
            candidate.to_str().unwrap(),
            "--candidate-kind",
            "reference",
            "--roms",
            roms.to_str().unwrap(),
            "--manifest",
            dir.join("manifest.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("sf-02.7k"), "{stderr}");
}

#[test]
fn an_absent_candidate_capture_exits_two() {
    let dir = temp_dir("cli-absent");
    let (scenario, reference, _) = setup(&dir);

    let out = sf_compare()
        .args([
            "compare",
            "--scenario",
            scenario.to_str().unwrap(),
            "--reference",
            reference.to_str().unwrap(),
            "--candidate",
            dir.join("not-here").to_str().unwrap(),
            "--candidate-kind",
            "reference",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("not-here"), "{stderr}");
}

#[test]
fn verify_roms_subcommand_accepts_a_valid_set() {
    let dir = temp_dir("cli-verify-roms");
    let roms = dir.join("roms");
    std::fs::create_dir_all(&roms).unwrap();
    let bytes = vec![0x42u8; 64];
    write_file(
        &dir,
        "manifest.json",
        manifest_json("sf", &[("sfd-19.2a", "maincpu", 0, &bytes)]).as_bytes(),
    );
    write_file(&roms, "sfd-19.2a", &bytes);

    let out = sf_compare()
        .args([
            "verify-roms",
            "--manifest",
            dir.join("manifest.json").to_str().unwrap(),
            "--roms",
            roms.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
}
