use std::path::Path;

use sf_audio::mix::{GameSound, ImpactAccent};
use sf_audio::{LatchTimeline, FRAME_HZ};

#[test]
fn heavy_accent_overlays_the_board_on_the_samples_bus_at_its_frame() {
    const ROMS: &str = "../../tools/reference/local/roms/sf";
    const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let load = |name| sf_video::romset::load_region(ROMS, MANIFEST, name).unwrap();
    let audio2 = load("audio2");
    let music = load("audiocpu");
    const IMPACT_FRAME: u32 = 6;
    let onset = f64::from(IMPACT_FRAME) / f64::from(FRAME_HZ);
    let make_sound = || {
        let mut sound = GameSound::power_on(
            audio2.clone(),
            music.clone(),
            LatchTimeline::new(vec![(onset, 0x84)]),
            f64::INFINITY,
        );
        assert!(sound.wait_ready(0.4));
        sound.record_port_writes();
        sound
    };
    let mut board = make_sound();
    let mut full = make_sound();
    let mut half = make_sound();
    let mut mute = make_sound();
    for sound in [&mut full, &mut half, &mut mute] {
        sound.play_impact(IMPACT_FRAME, ImpactAccent::HeavyStrike);
    }
    let mut peak = 0.0_f32;
    let mut original_peak = 0.0_f32;
    for sample in 0..9_600 {
        let t = f64::from(sample) / 24_000.0;
        let original = board.sample_at_mix(t, 0.0, 1.0, 0.0)[0];
        let [left, right] = full.sample_at_mix(t, 0.0, 1.0, 0.0);
        let half = half.sample_at_mix(t, 0.0, 0.5, 0.0)[0];
        assert_eq!(mute.sample_at_mix(t, 0.0, 0.0, 1.0), [0.0; 2]);
        assert_eq!(left, right);
        if t < onset || t > onset + 0.19 {
            assert_eq!(left, original, "impact at {t} outside the cue");
        } else if original.abs() < 0.65 {
            peak = peak.max((left - original).abs());
            original_peak = original_peak.max(original.abs());
        }
        if left.abs() < 0.99 {
            assert!((left * 0.5 - half).abs() < 1e-6);
        }
    }
    assert!(original_peak > 0.1, "the original impact is playing too");
    assert!(peak > 0.1, "the accent must be audible over the board");
    assert_eq!(board.take_port_writes(), full.take_port_writes());
}

#[test]
fn clearing_impacts_preserves_output_before_the_boundary_and_cancels_queued_cues() {
    let make_sound = || {
        GameSound::power_on(
            vec![0; 0x40000],
            vec![0; 0x8000],
            LatchTimeline::default(),
            f64::INFINITY,
        )
    };
    let mut full = make_sound();
    let mut cleared = make_sound();
    for sound in [&mut full, &mut cleared] {
        sound.play_impact(6, ImpactAccent::HeavyStrike);
        sound.play_impact(8, ImpactAccent::MatSlam);
        assert!(sound.wait_ready(0.3));
    }
    cleared.clear_impacts(7);
    let time = |frame| frame / f64::from(FRAME_HZ);
    let before = full.sample_at_mix(time(6.5), 0.0, 1.0, 0.0);
    assert_eq!(cleared.sample_at_mix(time(6.5), 0.0, 1.0, 0.0), before);
    assert!(before[0].abs() > 0.01);
    for range in [2_800..3_100, 3_300..3_600] {
        let mut audible = false;
        for sample in range {
            let t = f64::from(sample) / 24_000.0;
            assert_eq!(cleared.sample_at_mix(t, 0.0, 1.0, 0.0), [0.0; 2]);
            audible |= full.sample_at_mix(t, 0.0, 1.0, 0.0)[0].abs() > 0.01;
        }
        assert!(audible);
    }
}
