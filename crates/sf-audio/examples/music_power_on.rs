//! Debug aid: run the translated music CPU from power-on on a reference
//! capture's latch writes through frame G (local-only data) and compare
//! its YM2151 writes with the reference's — count, order, the exact-clock
//! matches, the largest sample lag, the first differing or lagging writes
//! with context — and, when the capture holds a hooked YM2151 stream, the
//! output samples that differ.
//!
//!   cargo run -p sf-audio --release --example music_power_on -- <audio capture> G

use std::path::Path;

use sf_audio::check::{
    exact_clock_matches, latch_timeline, read_device_stream, read_taps, reference_opm_writes,
    run_music_from_power_on,
};
use sf_audio::opm::{OPM_CLOCK, OPM_RATE};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = Path::new(&args[1]);
    let g: u32 = args[2].parse().unwrap();
    let log = read_taps(&dir.join("audio_taps.tsv")).expect("tap log");
    let rom = sf_video::romset::load_region(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
        "audiocpu",
    )
    .expect("audiocpu");
    let s_end = ((g + 1) as f64 / 60.0 * OPM_RATE as f64).ceil() as u64;
    let run = run_music_from_power_on(rom, latch_timeline(&log, None), s_end);
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample < s_end)
        .collect();
    let native: Vec<_> = run
        .writes
        .into_iter()
        .filter(|w| w.sample < s_end)
        .collect();
    let k = native.len().min(reference.len());
    let differing = (0..k)
        .filter(|&i| (native[i].reg, native[i].data) != (reference[i].reg, reference[i].data))
        .count();
    let lag = (0..k)
        .map(|i| native[i].sample.abs_diff(reference[i].sample))
        .max()
        .unwrap_or(0);
    println!(
        "through frame {g}: {} reference writes, {} native, {} on the exact clock, {differing} differ in order, largest lag {lag} samples, unrecovered {:04x?}",
        reference.len(),
        native.len(),
        exact_clock_matches(&native, &reference),
        run.unrecovered
    );
    let show = |i: usize| {
        let (n, r) = (&native[i], &reference[i]);
        println!(
            "  #{i} reference {:02x}={:02x} sample {} clock {} | native {:02x}={:02x} sample {} clock {} | {:+} T",
            r.reg,
            r.data,
            r.sample,
            r.clock,
            n.reg,
            n.data,
            n.sample,
            n.clock,
            n.clock as i64 - r.clock as i64
        );
    };
    let mut shown = 0;
    for i in 0..k {
        let (n, r) = (&native[i], &reference[i]);
        if (n.reg, n.data) != (r.reg, r.data) || n.sample.abs_diff(r.sample) > 1 {
            println!(
                "write #{i} {} (frame {}):",
                if (n.reg, n.data) != (r.reg, r.data) {
                    "differs"
                } else {
                    "lags"
                },
                r.sample as f64 / OPM_RATE as f64 * 60.0
            );
            for j in i.saturating_sub(3)..(i + 4).min(k) {
                show(j);
            }
            shown += 1;
            if shown >= 4 || (n.reg, n.data) != (r.reg, r.data) {
                break;
            }
        }
    }
    if native.len() != reference.len() {
        println!("counts differ around #{k}:");
        for j in k.saturating_sub(4)..k {
            show(j);
        }
    }
    // SF_SHOW_OFF=n: the first n writes off the exact clock, with context and
    // the latch writes (NMIs) just before them.
    if let Ok(n) = std::env::var("SF_SHOW_OFF") {
        let n: usize = n.parse().unwrap();
        let mut shown = 0;
        for i in 1..k {
            if native[i].clock != reference[i].clock
                && native[i - 1].clock == reference[i - 1].clock
            {
                println!(
                    "off from write #{i} (frame {:.1}):",
                    reference[i].sample as f64 / OPM_RATE as f64 * 60.0
                );
                for j in i.saturating_sub(2)..(i + 3).min(k) {
                    show(j);
                }
                let (t0, t1) = (
                    reference[i - 1].clock as f64 / OPM_CLOCK as f64,
                    reference[i].clock as f64 / OPM_CLOCK as f64,
                );
                for &(t, v) in log.latch.iter().filter(|e| e.0 >= t0 - 0.0005 && e.0 <= t1) {
                    println!(
                        "  latch {v:02x} at {t:.9} s (clock {})",
                        (t * OPM_CLOCK as f64).ceil() as u64
                    );
                }
                shown += 1;
                if shown >= n {
                    break;
                }
            }
        }
    }
    if let Ok(hook) = read_device_stream(dir, "ymsnd", OPM_RATE as f64) {
        let n0 = hook.first_sample();
        let (mut compared, mut differing) = (0u64, 0u64);
        let mut runs: Vec<(u64, u64)> = Vec::new();
        for (i, r) in hook.samples.iter().enumerate() {
            let n = n0 + i as u64;
            if n < s_end {
                compared += 1;
                if run.left[n as usize] != *r {
                    differing += 1;
                    match runs.last_mut() {
                        Some(last) if last.0 + last.1 == n => last.1 += 1,
                        _ => runs.push((n, 1)),
                    }
                }
            }
        }
        println!(
            "hooked output: {compared} samples compared, {differing} differ in {} runs",
            runs.len()
        );
        for (start, len) in runs.iter().take(8) {
            println!(
                "  samples {start}..{} ({len}): frame {:.2}, {:.6} s",
                start + len,
                *start as f64 / OPM_RATE as f64 * 60.0,
                *start as f64 / OPM_RATE as f64
            );
        }
    }
    let _ = OPM_CLOCK;
}
