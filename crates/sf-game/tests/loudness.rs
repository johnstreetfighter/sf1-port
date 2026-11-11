//! Ticket #200 (`ST8`): a Jukebox track plays about as loud as the native
//! stage theme it replaces. The ten 1987 stage themes are played on the
//! music CPU and measured at the output (the chip route's gain applied); a
//! track of a typical mastered level (0.15 RMS, about -16 dBFS) at the
//! track gain lands on the same level.
//!
//! Local ROMs only; skips without them.

use std::path::Path;

use sf_audio::mix::GameSound;
use sf_audio::{LatchTimeline, IRQ_LATENCY_MID};
use sf_host::jukebox::TRACK_GAIN;
use sf_video::romset::load_region;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
/// The RMS of a typical mastered track, full scale being 1.
const TYPICAL_TRACK_RMS: f64 = 0.15;

fn rms_of(audio2: &[u8], audiocpu: &[u8], tune: u8) -> f64 {
    let mut sound = GameSound::power_on(audio2.to_vec(), audiocpu.to_vec(), LatchTimeline::default(), 0.0);
    for f in 1..=700u32 {
        sound.push_frame(f, if f == 5 { tune } else { 0xff }, 0, IRQ_LATENCY_MID);
    }
    let (from, to) = (3.0f64, 10.0f64);
    assert!(sound.wait_ready(to));
    let (mut sum, mut n) = (0.0f64, 0u32);
    let mut t = from;
    while t < to {
        let [l, r] = sound.sample_at(t);
        let m = f64::from((l + r) / 2.0);
        sum += m * m;
        n += 1;
        t += 1.0 / 8000.0;
    }
    (sum / f64::from(n)).sqrt()
}

#[test]
fn a_typical_track_is_about_as_loud_as_the_stage_themes_it_replaces() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let audio2 = load_region(ROMS, MANIFEST, "audio2").unwrap();
    let audiocpu = load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let themes: Vec<f64> = (0x29u8..=0x32).map(|tune| rms_of(&audio2, &audiocpu, tune)).collect();
    let native = themes.iter().sum::<f64>() / themes.len() as f64;
    let track = TYPICAL_TRACK_RMS * f64::from(TRACK_GAIN);
    assert!((0.05..0.15).contains(&native), "the native themes: {themes:?}");
    assert!((track / native - 1.0).abs() < 0.25, "a typical track at {track:.3} RMS against the themes' {native:.3}: {themes:?}");
    // And no theme is far from the mean, so one reference level fits all.
    for (i, rms) in themes.iter().enumerate() {
        assert!((rms / native - 1.0).abs() < 0.5, "tune {:#x}: {rms:.3} against {native:.3}", 0x29 + i);
    }
}
