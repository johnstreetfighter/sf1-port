//! Ticket #201 (`ST9`): tracker modules are Jukebox tracks through
//! libopenmpt, each looping at the module's own restart position.
//!
//! The modules in `tests/data` are written by `make_modules.py` (original,
//! no music): one 32-byte sine cycle played as three patterns of eight
//! rows, C-4, then G-4, then C-5 (frequencies 1 : 1.5 : 2), restarting at
//! order 1. So a song plays pattern 0 once and then patterns 1 and 2 for
//! ever, and a test can tell which pattern it hears by its pitch.
#![cfg(feature = "tracker")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_host::track::{Playhead, Track};

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

fn load(name: &str) -> Track {
    Track::load(&data(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The frequency (Hz) of `frames` (mono mix), from the rising zero crossings
/// across the middle of the span.
fn pitch(frames: &[[f32; 2]], rate: u32) -> f64 {
    let (a, b) = (frames.len() / 10, frames.len() * 9 / 10);
    let mono: Vec<f32> = frames[a..b].iter().map(|f| f[0] + f[1]).collect();
    let crossings = mono.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
    crossings as f64 / (mono.len() as f64 / f64::from(rate))
}

/// The modules whose song restarts at order 1: file, and its format.
const RESTARTING: [(&str, &str); 6] = [
    ("loop.mod", "mod"),
    ("loop.xm", "xm"),
    ("loop.s3m", "s3m"),
    ("loop.it", "it"),
    ("loop.669", "669"),
    ("loop.mtm", "mtm"),
];

#[test]
fn modules_in_six_formats_decode_through_libopenmpt() {
    for (name, _) in RESTARTING {
        let t = load(name);
        assert_eq!(t.rate, 48_000, "{name}");
        assert!(t.frames.len() > 48_000, "{name}: {} frames, three patterns of eight rows", t.frames.len());
        assert!(t.frames.iter().any(|f| f[0].abs() > 0.1), "{name}: it is the tone");
    }
}

#[test]
fn each_module_loops_at_its_restart_position() {
    for (name, _) in RESTARTING {
        let t = load(name);
        let looped = t.looped.unwrap_or_else(|| panic!("{name}: a module always has a loop"));
        let (start, end) = (looped.start as f64, looped.end as f64);
        assert_eq!(looped.end as usize, t.frames.len(), "{name}: the loop ends where the song does");
        // Three equal patterns; restart at order 1 puts the loop's start one pattern in.
        assert!((start * 3.0 - end).abs() < end * 0.02, "{name}: loop starts at {start} of {end}, one pattern in");

        if name != "loop.669" {
            // 125 BPM, 6 ticks a row at 48 kHz: 5760 frames a row, 46080 a pattern, to the frame.
            assert_eq!((looped.start, looped.end), (46_080, 3 * 46_080), "{name}");
        }

        // Which pattern is heard where: pattern 0 is the intro, the loop starts on pattern 1.
        let pattern = looped.start as usize;
        let (p0, p1, p2) = (pitch(&t.frames[..pattern], t.rate), pitch(&t.frames[pattern..2 * pattern], t.rate), pitch(&t.frames[2 * pattern..], t.rate));
        assert!((p1 / p0 - 1.5).abs() < 0.05, "{name}: the loop starts a fifth above the intro ({p0} Hz, then {p1} Hz)");
        assert!((p2 / p0 - 2.0).abs() < 0.05, "{name}: and goes on to the octave ({p2} Hz)");
    }
}

#[test]
fn playing_on_past_the_end_goes_back_to_the_restart_position_not_the_top() {
    for (name, _) in RESTARTING {
        let t = Arc::new(load(name));
        let looped = t.looped.unwrap();
        let pattern = looped.start as usize;
        let mut head = Playhead::new(t.clone());
        // The whole song once, then the loop's first pattern again and the second again.
        let played: Vec<[f32; 2]> = (0..t.frames.len() + 2 * pattern).map(|_| head.next(f64::from(t.rate))).collect();
        let (intro, again1, again2) = (&played[..pattern], &played[t.frames.len()..t.frames.len() + pattern], &played[t.frames.len() + pattern..]);
        let (p0, p1, p2) = (pitch(intro, t.rate), pitch(again1, t.rate), pitch(again2, t.rate));
        assert!((p1 / p0 - 1.5).abs() < 0.05, "{name}: back at pattern 1, not pattern 0 ({p0} Hz, then {p1} Hz)");
        assert!((p2 / p0 - 2.0).abs() < 0.05, "{name}: then pattern 2 ({p2} Hz)");
    }
}

#[test]
fn a_module_with_no_restart_position_loops_from_the_top() {
    let t = load("loop-top.mod");
    let looped = t.looped.unwrap();
    assert_eq!((looped.start, looped.end as usize), (0, t.frames.len()), "restart position 0: the whole song repeats");
    // The same song as loop.mod, but the loop is longer by the intro.
    let restarting = load("loop.mod");
    assert_eq!(t.frames.len(), restarting.frames.len());
    assert!(restarting.looped.unwrap().start > 0);
}

#[test]
fn a_tempo_change_in_the_loop_does_not_move_the_restart_position() {
    // Speed 6 in order 0 (46,080 frames a pattern), carried through order 1,
    // and 3 from order 2 on (23,040 frames). The song restarts at order 1,
    // which libopenmpt reaches at 46,080 frames however the speed carries over.
    let t = load("loop-tempo.mod");
    let looped = t.looped.unwrap();
    assert_eq!((looped.start, looped.end), (46_080, 46_080 + 46_080 + 23_040));
    let (p0, p1) = (pitch(&t.frames[..46_080], t.rate), pitch(&t.frames[46_080..2 * 46_080], t.rate));
    assert!((p1 / p0 - 1.5).abs() < 0.05, "the loop starts on pattern 1 ({p0} Hz, then {p1} Hz)");
}

#[test]
fn a_module_is_read_by_content_not_by_name() {
    // The extension is wrong, or there is none; libopenmpt still plays it.
    let dir = std::env::temp_dir().join(format!("sf1-port-modules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, as_name) in [("loop.xm", "renamed.bin"), ("loop.it", "no-extension"), ("loop.s3m", "song.mod")] {
        let to = dir.join(as_name);
        std::fs::copy(data(name), &to).unwrap();
        let t = Track::load(&to).unwrap_or_else(|e| panic!("{as_name}: {e}"));
        let looped = t.looped.unwrap();
        assert!((looped.start as f64 * 3.0 - t.frames.len() as f64).abs() < t.frames.len() as f64 * 0.02, "{as_name}: it loops at its restart");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_broken_module_is_an_error_naming_the_file() {
    let dir = std::env::temp_dir().join(format!("sf1-port-broken-modules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let good = std::fs::read(data("loop.it")).unwrap();
    let cases: [(&str, Vec<u8>); 4] = [
        ("garbage.xm", b"this is not a module at all, only text".to_vec()),
        ("empty.mod", Vec::new()),
        ("truncated.it", good[..120].to_vec()),
        ("garbage-no-extension", vec![0xA5; 4000]),
    ];
    for (name, bytes) in cases {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        let e = Track::load(&path).unwrap_err();
        assert!(e.contains(name), "{e}");
        assert!(e.contains("module"), "it says what it could not read it as: {e}");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_truncated_module_is_never_silence_it_plays_or_it_is_an_error() {
    // Cut at the header, mid-pattern-data and mid-sample, in every format: libopenmpt
    // loads what is left, which may be a song whose samples never came.
    let dir = std::env::temp_dir().join(format!("sf1-port-truncated-modules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, _) in RESTARTING {
        let whole = std::fs::read(data(name)).unwrap();
        for percent in [10, 30, 50, 70, 90, 99] {
            let cut = whole.len() * percent / 100;
            let path = dir.join(format!("cut{percent}-{name}"));
            std::fs::write(&path, &whole[..cut]).unwrap();
            if let Ok(t) = Track::load(&path) {
                let peak = t.frames.iter().fold(0f32, |m, f| m.max(f[0].abs()).max(f[1].abs()));
                assert!(peak >= 1e-4, "{name} cut at {percent}% ({cut} bytes) loaded as {} frames of silence", t.frames.len());
            }
        }
    }
    // A ProTracker module cut inside its pattern data has no samples at all.
    let whole = std::fs::read(data("loop.mod")).unwrap();
    let path = dir.join("mid-patterns.mod");
    std::fs::write(&path, &whole[..1084 + 1024 + 500]).unwrap();
    let e = Track::load(&path).unwrap_err();
    assert!(e.contains("mid-patterns.mod") && e.contains("no sound in the module"), "{e}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_wrong_extension_still_plays_in_either_direction() {
    let dir = std::env::temp_dir().join(format!("sf1-port-wrong-extension-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // An OGG file named as a module.
    for name in ["audio.xm", "audio.it", "audio.mod"] {
        let path = dir.join(name);
        std::fs::copy(data("loop.ogg"), &path).unwrap();
        let t = Track::load(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((t.rate, t.frames.len()), (8000, 4000), "{name}: it is the OGG file");
    }
    // A module named as audio.
    for name in ["song.ogg", "song.wav", "song.flac"] {
        let path = dir.join(name);
        std::fs::copy(data("loop.s3m"), &path).unwrap();
        let t = Track::load(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(t.rate, 48_000, "{name}: it is the module");
    }
    std::fs::remove_dir_all(&dir).ok();
}
