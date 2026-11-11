//! Ticket #201 (`ST9`): a build without the `tracker` feature (no C++
//! compiler, `--no-default-features`) still builds and plays OGG, WAV and
//! FLAC, and a tracker module is an error that says why, so the Jukebox
//! logs it and the native theme plays.
#![cfg(not(feature = "tracker"))]

use sf_host::track::Track;

#[test]
fn a_tracker_module_is_an_error_naming_the_missing_feature() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/loop.xm");
    let e = Track::load(&path).unwrap_err();
    assert!(e.contains("loop.xm") && e.contains("tracker"), "{e}");
    assert!(Track::load(&path.with_file_name("loop.ogg")).is_ok(), "the other formats play");
}
