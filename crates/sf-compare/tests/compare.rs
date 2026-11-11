//! The comparison contract: identical observations pass; the first
//! observable divergence is reported with its frame, expected and actual
//! values, and baseline identity; a reference-versus-reference self-check
//! is identified as such.

mod common;

use common::{scenario_json, temp_dir, CaptureBuilder};
use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::parse_scenario;

fn scenario(frame_count: usize, checkpoints: &[usize]) -> sf_compare::scenario::Scenario {
    parse_scenario(&scenario_json(frame_count, checkpoints)).unwrap()
}

#[test]
fn identical_reference_observations_pass_and_are_labelled_a_self_check() {
    let dir = temp_dir("compare-match");
    let mut builder = CaptureBuilder::new(6);
    builder.solid_pixels(2, [1, 2, 3, 255]);
    builder.ram_dumps.push((2, "work".into(), vec![0xAA; 16]));
    builder.write(&dir.join("a"));
    builder.write(&dir.join("b"));

    let scenario = scenario(6, &[2]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &CompareScope::full(),
    )
    .expect("compatible captures");

    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = &outcome
    else {
        panic!("identical captures must match, got {outcome:?}");
    };
    assert_eq!(*frames_compared, 6);
    assert_eq!(*checkpoints_compared, 1);

    let report = sf_compare::report::render(
        &outcome,
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &CompareScope::full(),
    );
    assert!(
        report.contains("self-check"),
        "reference-vs-reference must be identified as a self-check: {report}"
    );
}

#[test]
fn a_shifted_gameplay_event_fails_at_the_first_diverging_frame() {
    let dir = temp_dir("compare-shifted");
    let mut builder = CaptureBuilder::new(6);
    builder.solid_pixels(2, [1, 2, 3, 255]);
    builder.write(&dir.join("a"));

    // Candidate diverges: the RAM CRC sampled at frame 4 changes.
    let mut shifted = CaptureBuilder::new(6);
    shifted.solid_pixels(2, [1, 2, 3, 255]);
    shifted.frames[4].ram_crc = "deadbeef".into();
    shifted.write(&dir.join("b"));

    let scenario = scenario(6, &[2]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &CompareScope::full(),
    )
    .expect("compatible captures");

    let Outcome::Mismatch(m) = &outcome else {
        panic!("diverging captures must mismatch, got {outcome:?}");
    };
    assert_eq!(m.frame, 4);
    assert_eq!(m.observable, "ram_crc");
    assert_eq!(m.expected, "00002004");
    assert_eq!(m.actual, "deadbeef");

    let report = sf_compare::report::render(
        &outcome,
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &CompareScope::full(),
    );
    assert!(
        report.contains("frame 4"),
        "report names the frame: {report}"
    );
    assert!(
        report.contains("sf-fixed-fight-01"),
        "report names the scenario: {report}"
    );
    assert!(
        report.contains("0.289"),
        "report names the baseline: {report}"
    );
}

#[test]
fn a_changed_game_pixel_fails_at_its_checkpoint_frame() {
    let dir = temp_dir("compare-pixel");
    let mut reference_builder = CaptureBuilder::new(6);
    reference_builder.solid_pixels(2, [1, 2, 3, 255]);
    reference_builder.write(&dir.join("a"));

    // Same checkpoint frame, but pixel index 5 (x=1, y=1 at width 4) changes.
    let mut pixels = [[1u8, 2, 3, 255]].repeat(4 * 2);
    pixels[5] = [9, 9, 9, 255];
    let flat: Vec<u8> = pixels.into_iter().flatten().collect();
    let mut candidate_builder = CaptureBuilder::new(6);
    candidate_builder.checkpoints.push((2, flat));
    candidate_builder.write(&dir.join("b"));

    let scenario = scenario(6, &[2]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &CompareScope::full(),
    )
    .expect("compatible captures");

    let Outcome::Mismatch(m) = &outcome else {
        panic!("a changed pixel must fail the comparison: {outcome:?}");
    };
    assert_eq!(m.frame, 2);
    assert_eq!(m.observable, "checkpoint screen pixels");
    let detail = m.detail.as_ref().expect("pixel detail present");
    assert!(
        detail.contains("(1, 1)"),
        "detail locates the changed pixel: {detail}"
    );
}

#[test]
fn a_ram_dump_byte_difference_is_reported_with_region_and_offset() {
    let dir = temp_dir("compare-ram");
    let mut reference_builder = CaptureBuilder::new(6);
    reference_builder
        .ram_dumps
        .push((2, "work".into(), vec![0xAA; 16]));
    reference_builder.write(&dir.join("a"));

    let mut changed = vec![0xAA; 16];
    changed[9] = 0xBB;
    let mut candidate_builder = CaptureBuilder::new(6);
    candidate_builder
        .ram_dumps
        .push((2, "work".into(), changed));
    candidate_builder.write(&dir.join("b"));

    let scenario = scenario(6, &[]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &CompareScope::full(),
    )
    .expect("compatible captures");

    let Outcome::Mismatch(m) = &outcome else {
        panic!("a RAM difference must fail the comparison: {outcome:?}");
    };
    assert_eq!(m.frame, 2);
    assert!(m.observable.contains("work"), "names the region: {m:?}");
    assert!(
        m.detail.as_ref().unwrap().contains("0x0009"),
        "names the offset: {}",
        m.detail.as_ref().unwrap()
    );
}

#[test]
fn a_capture_from_a_different_baseline_is_an_incompatible_error() {
    let dir = temp_dir("compare-baseline");
    let builder = CaptureBuilder::new(6);
    builder.write(&dir.join("a"));

    let mut wrong_version = CaptureBuilder::new(6);
    wrong_version.mame_version = "0.250 (mame0250)".into();
    wrong_version.write(&dir.join("b"));

    let scenario = scenario(6, &[]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let err = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &CompareScope::full(),
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("0.250"), "names the actual baseline: {msg}");
    assert!(msg.contains("0.289"), "names the pinned baseline: {msg}");
}

#[test]
fn a_capture_of_a_different_scenario_is_an_incompatible_error() {
    let dir = temp_dir("compare-scenario-id");
    let builder = CaptureBuilder::new(6);
    builder.write(&dir.join("a"));

    let mut other = CaptureBuilder::new(6);
    other.scenario_id = "sf-other-scenario".into();
    other.write(&dir.join("b"));

    let scenario = scenario(6, &[]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let err = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &CompareScope::full(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("sf-other-scenario"),
        "names the capture's scenario: {err}"
    );
}

#[test]
fn a_frame_scoped_comparison_ignores_divergence_outside_the_range() {
    let dir = temp_dir("compare-scope-range");
    let mut builder = CaptureBuilder::new(6);
    builder.solid_pixels(2, [1, 2, 3, 255]);
    builder.write(&dir.join("a"));

    // Candidate diverges at frames 1 and 5, both outside 2..=4.
    let mut diverged = CaptureBuilder::new(6);
    diverged.solid_pixels(2, [1, 2, 3, 255]);
    diverged.frames[1].screen_hash = "deadbeef".into();
    diverged.frames[5].screen_hash = "deadbeef".into();
    diverged.write(&dir.join("b"));

    let scenario = scenario(6, &[2]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let scope = CompareScope::full().with_range(2, 4);
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &scope,
    )
    .expect("compatible captures");

    let Outcome::Match {
        frames_compared,
        checkpoints_compared,
    } = &outcome
    else {
        panic!("out-of-range divergence must be ignored, got {outcome:?}");
    };
    assert_eq!(*frames_compared, 3, "frames 2, 3, 4 compared");
    assert_eq!(*checkpoints_compared, 1, "checkpoint 2 is inside the range");
}

#[test]
fn a_frame_scoped_comparison_reports_divergence_inside_the_range() {
    let dir = temp_dir("compare-scope-inside");
    let builder = CaptureBuilder::new(6);
    builder.write(&dir.join("a"));

    let mut diverged = CaptureBuilder::new(6);
    diverged.frames[3].screen_hash = "deadbeef".into();
    diverged.write(&dir.join("b"));

    let scenario = scenario(6, &[]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let scope = CompareScope::full().with_range(2, 4);
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Reference,
        &scope,
    )
    .expect("compatible captures");

    let Outcome::Mismatch(m) = &outcome else {
        panic!("in-range divergence must be reported, got {outcome:?}");
    };
    assert_eq!(m.frame, 3);
    assert_eq!(m.observable, "screen_hash");
}

#[test]
fn a_field_scoped_comparison_ignores_unlisted_observables() {
    let dir = temp_dir("compare-scope-fields");
    let builder = CaptureBuilder::new(6);
    builder.write(&dir.join("a"));

    // Candidate diverges on ram_crc only; the scope lists screen_hash/latch.
    let mut diverged = CaptureBuilder::new(6);
    diverged.frames[2].ram_crc = "deadbeef".into();
    diverged.write(&dir.join("b"));

    let scenario = scenario(6, &[]);
    let reference = load_capture(dir.join("a")).unwrap();
    let candidate = load_capture(dir.join("b")).unwrap();
    let scope = CompareScope::full()
        .with_fields(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap());
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope,
    )
    .expect("compatible captures");
    assert!(
        matches!(outcome, Outcome::Match { .. }),
        "unlisted observables must be ignored, got {outcome:?}"
    );

    // Listing the diverging field surfaces the mismatch again.
    let scope = CompareScope::full().with_fields(CompareScope::parse_fields("ram_crc").unwrap());
    let outcome = compare(
        &scenario,
        &reference,
        &candidate,
        CandidateKind::Native,
        &scope,
    )
    .expect("compatible captures");
    let Outcome::Mismatch(m) = &outcome else {
        panic!("listed diverging field must be reported, got {outcome:?}");
    };
    assert_eq!(m.observable, "ram_crc");
}

#[test]
fn an_unknown_scope_field_is_rejected() {
    let err = CompareScope::parse_fields("screen_hash,bogus").unwrap_err();
    assert!(err.contains("bogus"), "names the unknown field: {err}");
}
