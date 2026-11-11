//! Acceptance test for ticket #11: one bounded phrase of the fixed fight's
//! round music (command 0x28; frames 2280..=2359 of sf-music-01), played by
//! the translated music-CPU program with its timing and the YM2151 model,
//! matches the pinned MAME reference — the chip model reproduces the
//! reference's YM2151 stream from its register writes; from the fixture the
//! native program makes the same writes in the same order (timer ticks
//! included, within one output sample) and the output is identical to the
//! reference's YM2151 stream on every sample.
//!
//! Consumes local-only data (reference audio runs from
//! tools/reference/run_audio_probe.py with --hook-window 2279 2362, the
//! sf-music-01 reference capture, ROM set); skips on a clean checkout.
//! docs/reference/music-phrase.md.

use std::path::Path;

use sf_audio::check::{
    latch_timeline, read_device_stream, read_taps, reference_opm_writes, replay_opm, run_music,
    MUSIC_FIXTURE_IDLE_PHASE,
};
use sf_audio::opm::OPM_RATE;

const AUDIO_A: &str = "../../tools/reference/local/captures/music-audio-a";
const AUDIO_B: &str = "../../tools/reference/local/captures/music-audio-b";
const CAPTURE: &str = "../../tools/reference/local/captures/music-01-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// The fixture: the music RAM dumped at the end of frame 2279 (38.0 s); the
/// phrase ends with frame 2359.
const FIXTURE_FRAME: u32 = 2279;
const LAST_FRAME: u32 = 2359;

fn present() -> bool {
    [AUDIO_A, AUDIO_B, CAPTURE, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn window() -> (f64, u64, u64) {
    let rate = OPM_RATE as f64;
    let t_fix = (FIXTURE_FRAME + 1) as f64 / 60.0;
    let t_end = (LAST_FRAME + 1) as f64 / 60.0;
    (
        t_fix,
        (t_fix * rate).ceil() as u64,
        (t_end * rate).ceil() as u64,
    )
}

#[test]
fn reference_audio_runs_repeat_exactly() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    for f in ["audio_taps.tsv", "hook_ymsnd.f32"] {
        let a = std::fs::read(Path::new(AUDIO_A).join(f)).unwrap();
        let b = std::fs::read(Path::new(AUDIO_B).join(f)).unwrap();
        assert!(a == b, "{f} must repeat exactly");
    }
}

#[test]
fn the_chip_model_reproduces_the_reference_stream_from_its_writes() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(AUDIO_A).join("audio_taps.tsv")).unwrap();
    let hook = read_device_stream(Path::new(AUDIO_A), "ymsnd", OPM_RATE as f64).unwrap();
    let n0 = hook.first_sample() as usize;
    let native = replay_opm(&log.opm, n0 + hook.samples.len(), 0.0);
    let differing = hook
        .samples
        .iter()
        .enumerate()
        .filter(|(i, r)| native[n0 + i] != **r)
        .count();
    assert_eq!(
        differing, 0,
        "replaying 38 s of register writes reproduces every sample"
    );
    assert!(hook.samples.len() > 70_000);
}

#[test]
fn the_native_program_plays_the_phrase_like_the_reference() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(AUDIO_A).join("audio_taps.tsv")).unwrap();
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let ram = std::fs::read(Path::new(CAPTURE).join(format!("ram_{FIXTURE_FRAME:06}_music.bin")))
        .unwrap();
    let (t_fix, s_fix, s_end) = window();
    let run = run_music(
        rom,
        ram,
        &log,
        t_fix,
        latch_timeline(&log, None),
        MUSIC_FIXTURE_IDLE_PHASE,
        s_end,
    );
    assert!(
        run.unrecovered.is_empty(),
        "translated paths only: {:04x?}",
        run.unrecovered
    );

    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample >= s_fix && w.sample < s_end)
        .collect();
    let native: Vec<_> = run
        .writes
        .into_iter()
        .filter(|w| w.sample < s_end)
        .collect();
    let ticks = |v: &[sf_audio::check::OpmWrite], reg| v.iter().filter(|w| w.reg == reg).count();
    assert_eq!(
        ticks(&reference, 0x10),
        430,
        "timer A ticks (14 lost to long handlers)"
    );
    assert_eq!(ticks(&native, 0x10), 430);
    assert_eq!(
        ticks(&native, 0x12),
        ticks(&reference, 0x12),
        "timer B polls"
    );
    assert_eq!(native.len(), reference.len(), "every write");
    for (i, (r, w)) in reference.iter().zip(&native).enumerate() {
        assert_eq!((r.reg, r.data), (w.reg, w.data), "write #{i} in order");
        assert!(
            r.sample.abs_diff(w.sample) <= 1,
            "write #{i} within one sample"
        );
    }

    let hook = read_device_stream(Path::new(AUDIO_A), "ymsnd", OPM_RATE as f64).unwrap();
    let n0 = hook.first_sample();
    let mut compared = 0;
    for (i, r) in hook.samples.iter().enumerate() {
        let n = n0 + i as u64;
        if n >= s_fix && n < s_end {
            assert_eq!(run.left[n as usize], *r, "output sample {n}");
            compared += 1;
        }
    }
    assert!(
        compared > 74_000,
        "the whole phrase is compared ({compared} samples)"
    );
}
