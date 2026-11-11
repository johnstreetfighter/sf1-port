//! Capture loading: absent or malformed observation input must produce
//! clear errors, never a confusing comparison result.

mod common;

use common::{temp_dir, CaptureBuilder};

#[test]
fn absent_capture_directory_is_a_clear_error() {
    let dir = temp_dir("capture-absent");
    let err = sf_compare::capture::load_capture(dir.join("no-such-capture")).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("no-such-capture"),
        "must name the missing directory: {msg}"
    );
}

#[test]
fn missing_frames_file_is_a_clear_error() {
    let dir = temp_dir("capture-no-frames");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("meta.tsv"), "screen_width\t4\n").unwrap();

    let err = sf_compare::capture::load_capture(&dir).unwrap_err();
    assert!(
        err.to_string().contains("frames.tsv"),
        "must name the missing observation file: {err}"
    );
}

#[test]
fn loads_a_synthetic_capture() {
    let dir = temp_dir("capture-loads");
    let mut builder = CaptureBuilder::new(6);
    builder
        .solid_pixels(2, [1, 2, 3, 255])
        .solid_pixels(4, [9, 9, 9, 255]);
    builder.ram_dumps.push((2, "work".into(), vec![0xAA; 16]));
    builder.write(&dir);

    let cap = sf_compare::capture::load_capture(&dir).expect("synthetic capture loads");
    assert_eq!(cap.frames.len(), 6);
    assert_eq!(cap.frames[3].screen_hash, "00001003");
    assert_eq!(cap.frames[3].ram_crc, None);
    assert_eq!(cap.frames[4].ram_crc.as_deref(), Some("00002004"));
    assert_eq!(cap.checkpoints.len(), 2);
    assert_eq!(cap.checkpoints[0].frame, 2);
    assert_eq!(cap.meta.screen_width, 4);
    assert_eq!(cap.meta.screen_height, 2);
    assert_eq!(cap.meta.frames_executed, 6);
    assert_eq!(cap.scenario_id.as_deref(), Some("sf-fixed-fight-01"));
    assert_eq!(
        cap.mame_version_reported.as_deref(),
        Some("0.289 (mame0289)")
    );
    assert_eq!(cap.ram_dumps.len(), 1);
    assert_eq!(cap.ram_dumps[0].region, "work");
}
