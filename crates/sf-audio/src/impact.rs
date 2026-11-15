//! Original, mono impact accents mixed alongside the arcade's sound.

use std::collections::VecDeque;
use std::f64::consts::TAU;
use std::sync::OnceLock;

use crate::FRAME_HZ;

const RATE: f64 = 48_000.0;

/// The extra sound on a clean heavy strike or a hard surface contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactAccent {
    HeavyStrike,
    MatSlam,
    WallSlam,
}

impl ImpactAccent {
    fn waveform(self) -> &'static [f32] {
        static HEAVY: OnceLock<Box<[f32]>> = OnceLock::new();
        static MAT: OnceLock<Box<[f32]>> = OnceLock::new();
        static WALL: OnceLock<Box<[f32]>> = OnceLock::new();
        let (waveform, duration) = match self {
            Self::HeavyStrike => (&HEAVY, 0.190),
            Self::MatSlam => (&MAT, 0.240),
            Self::WallSlam => (&WALL, 0.215),
        };
        waveform.get_or_init(|| self.synthesize(duration))
    }

    // Match the listening preview's pitch sweeps, noise bands and envelopes.
    fn synthesize(self, duration: f64) -> Box<[f32]> {
        let len = (duration * RATE).ceil() as usize;
        let mut wave = vec![0.0; len];
        match self {
            Self::HeavyStrike => {
                add_noise(&mut wave, 0.0, (720.0, 5_200.0), 0.021, 1.55, 33);
                add_tone(&mut wave, 0.0, (190.0, 58.0), 0.039, 0.77, 0.36);
                add_tone(&mut wave, 0.003, (390.0, 190.0), 0.013, 0.32, 0.42);
            }
            Self::MatSlam => {
                add_tone(&mut wave, 0.0, (125.0, 48.0), 0.056, 0.90, 0.44);
                add_noise(&mut wave, 0.0, (115.0, 1_050.0), 0.038, 0.92, 77);
                add_noise(&mut wave, 0.022, (105.0, 790.0), 0.048, 0.51, 91);
            }
            Self::WallSlam => {
                add_noise(&mut wave, 0.0, (1_800.0, 6_700.0), 0.015, 1.95, 119);
                add_tone(&mut wave, 0.0, (270.0, 92.0), 0.033, 0.70, 0.46);
                add_tone(&mut wave, 0.006, (920.0, 445.0), 0.020, 0.24, 0.31);
                add_noise(&mut wave, 0.020, (540.0, 3_150.0), 0.030, 0.57, 127);
            }
        }
        for (index, sample) in wave.iter_mut().enumerate() {
            let tail = ((len - 1 - index) as f64 / (RATE * 0.012)).min(1.0);
            *sample = ((f64::from(*sample) * 0.95).tanh() * tail * 0.48) as f32;
        }
        wave.into_boxed_slice()
    }
}

fn add_tone(
    wave: &mut [f32],
    onset: f64,
    (from, to): (f64, f64),
    decay: f64,
    volume: f64,
    harmonic: f64,
) {
    let mut phase = 0.0;
    let start = (onset * RATE).round() as usize;
    for (i, sample) in wave[start..].iter_mut().enumerate() {
        let t = i as f64 / RATE;
        let frequency = to + (from - to) * (-t / 0.021).exp();
        phase += TAU * frequency / RATE;
        let envelope = (t / 0.0015).min(1.0) * (-t / decay).exp();
        let tone = phase.sin() + harmonic * (phase * 2.0).sin();
        *sample = (f64::from(*sample) + tone * envelope * volume) as f32;
    }
}

fn add_noise(
    wave: &mut [f32],
    onset: f64,
    (low, high): (f64, f64),
    decay: f64,
    volume: f64,
    mut seed: u32,
) {
    let low_step = 1.0 - (-TAU * low / RATE).exp();
    let high_step = 1.0 - (-TAU * high / RATE).exp();
    let mut lower = 0.0;
    let mut upper = 0.0;
    let start = (onset * RATE).round() as usize;
    for (i, sample) in wave[start..].iter_mut().enumerate() {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = f64::from(seed) / 2_147_483_648.0 - 1.0;
        lower += (noise - lower) * low_step;
        upper += (noise - upper) * high_step;
        let t = i as f64 / RATE;
        let envelope = (t / 0.001).min(1.0) * (-t / decay).exp();
        *sample = (f64::from(*sample) + (upper - lower) * envelope * volume) as f32;
    }
}

struct Voice {
    start: f64,
    end: f64,
    waveform: &'static [f32],
}

#[derive(Default)]
pub(crate) struct ImpactMixer {
    voices: VecDeque<Voice>,
}

impl ImpactMixer {
    pub(crate) fn play(&mut self, frame: u32, accent: ImpactAccent) {
        let waveform = accent.waveform();
        let start = f64::from(frame) / f64::from(FRAME_HZ);
        self.voices.push_back(Voice {
            start,
            end: start + waveform.len() as f64 / RATE,
            waveform,
        });
    }

    pub(crate) fn clear_at(&mut self, frame: u32) {
        let t = f64::from(frame) / f64::from(FRAME_HZ);
        self.voices.retain_mut(|voice| {
            voice.end = voice.end.min(t);
            voice.start < voice.end
        });
    }

    pub(crate) fn sample_at(&mut self, t: f64) -> Option<f32> {
        self.voices.retain(|voice| t < voice.end);
        let mut active = self.voices.iter().filter(|voice| voice.start <= t);
        let interpolate = |voice: &Voice| {
            let x = (t - voice.start) * RATE;
            let i = x as usize;
            let f = x.fract() as f32;
            let a = voice.waveform.get(i).copied().unwrap_or(0.0);
            let b = voice.waveform.get(i + 1).copied().unwrap_or(0.0);
            a + (b - a) * f
        };
        let first = active.next()?;
        Some(interpolate(first) + active.map(interpolate).sum::<f32>())
    }
}
