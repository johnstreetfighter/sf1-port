//! Ticket #158 (`FE8`): the Sound tab's volumes scale the mixer. The demo
//! fight's sound from power-on (the headless session's latch stream) is
//! mixed three times: at full volume, at master 5, and with FM and samples
//! at 0. Half master halves every sample; FM and samples at 0 are silence;
//! full volume is the mixer's own output. Local ROMs only.

use std::path::Path;

use sf_audio::{FRAME_HZ, IRQ_LATENCY_MID};
use sf_engine::config::MatchConfig;
use sf_engine::front::{FrameInputs, GameSession};
use sf_host::audio::{Gains, SoundStart};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

#[test]
fn master_fm_and_samples_volumes_scale_the_mixer_output() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let region = |name| sf_video::romset::load_region(ROMS, MANIFEST, name).unwrap();
    let start = SoundStart {
        audio2: region("audio2"),
        audiocpu: region("audiocpu"),
        frame: 0,
        preroll: sf_audio::LatchTimeline::default(),
    };
    // The demo fight's round music and voices play over 2200..=2299.
    const LAST: u32 = 2300;
    let mut session = GameSession::new(assets, MatchConfig::default());
    let latches: Vec<_> = (1..=LAST)
        .map(|_| {
            let out = session.step(FrameInputs::default());
            (out.sound_frame, out.sound_latch, out.latch_cycles)
        })
        .collect();
    let mix = |gains: Gains| -> Vec<[f32; 2]> {
        let mut sound = start.build();
        for &(f, latch, cycles) in &latches {
            sound.push_frame(f, latch, cycles, IRQ_LATENCY_MID);
        }
        let t_end = LAST as f64 / FRAME_HZ as f64;
        sound.wait_ready(t_end);
        let rate = sf_audio::check::WAV_RATE as f64;
        let t0 = 2200.0 / FRAME_HZ as f64;
        (0..((t_end - 0.01 - t0) * rate) as usize)
            .map(|i| gains.sample(&mut sound, t0 + i as f64 / rate))
            .collect()
    };
    let full = mix(Gains::FULL);
    let half = mix(Gains::from_volumes(5, 10, 10, 10));
    let mute = mix(Gains::from_volumes(10, 0, 0, 10));
    let rms = |v: &[[f32; 2]]| (v.iter().map(|[l, r]| (l * l + r * r) as f64).sum::<f64>() / v.len() as f64).sqrt();
    println!("rms: full {:.5}, master 5 {:.5}, fm and samples 0 {:.5}", rms(&full), rms(&half), rms(&mute));
    assert!(rms(&full) > 0.05, "the demo fight is audible");
    for (i, (f, h)) in full.iter().zip(&half).enumerate() {
        for ch in 0..2 {
            if f[ch].abs() < 0.999 {
                assert!((h[ch] - f[ch] / 2.0).abs() < 1e-6, "sample {i}: {} vs {}", h[ch], f[ch]);
            }
        }
    }
    assert!(mute.iter().all(|[l, r]| *l == 0.0 && *r == 0.0), "FM and samples at 0 are silence");
}
