//! Debug aid (ticket #37): both translated sound CPUs from power-on on a
//! latch timeline that is the reference's before frame `from` and the native
//! engine's per-frame latch (with its handler cycles and an interrupt
//! latency) from it on; each device's output compared with the reference's
//! hooked streams over frames F0..=F1 (local-only data).
//!
//!   cargo run -p sf-audio --release --example native_round2 -- <taps + ymsnd hook capture> <msm hook capture> <native capture> <from> <F0> <F1> [latency]

use std::path::Path;

use sf_audio::check::{
    best_correlation, compare_dac, compare_dac_stream, compare_writes, exact_clock_matches,
    frame_tick, latch_timeline, measure, native_at_reference_times, native_latch,
    read_device_stream, read_hook, read_taps, read_wav_mono, reference_opm_writes, render_mix,
    run_music_from_power_on, write_time_drift, NativeLatch, WAV_RATE,
};
use sf_audio::opm::OPM_RATE;
use sf_audio::IRQ_LATENCY_MID;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (audio, msm, native) = (
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
    );
    let from: u32 = args[4].parse().unwrap();
    let (f0, f1): (u32, u32) = (args[5].parse().unwrap(), args[6].parse().unwrap());
    // The latency, or "ref": the native values at the reference's write times.
    let latency = args
        .get(7)
        .cloned()
        .unwrap_or_else(|| IRQ_LATENCY_MID.to_string());
    let log = read_taps(&audio.join("audio_taps.tsv")).expect("tap log");
    let frames = native_latch(native).expect("native frames.tsv with latch_cycles");
    let tl = if latency == "ref" {
        native_at_reference_times(&log, &frames, from)
    } else {
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from,
                latency: latency.parse().unwrap(),
            }),
        )
    };
    let roms = (
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
    );
    println!("native latch from frame {from} (latency {latency}), window {f0}..={f1}");
    let samples_only = std::env::var("SF_SAMPLES_ONLY").is_ok();

    // The samples CPU: every port write of the window, then both DAC streams.
    let audio2 = sf_video::romset::load_region(roms.0, roms.1, "audio2").expect("audio2");
    match compare_writes(
        audio2.clone(),
        tl.clone(),
        &log.writes,
        frame_tick(f0),
        frame_tick(f1 + 1),
    ) {
        Ok(n) => {
            // The writes' times within their ticks, against the logged times.
            let drift = write_time_drift(
                audio2.clone(),
                tl.clone(),
                &log.writes,
                frame_tick(f0),
                frame_tick(f1 + 1),
            )
            .map(|(_, off, w, first)| {
                let at = first
                    .map(|k| format!(", the first at tick {k} (frame {})", k * 60 / 8000))
                    .unwrap_or_default();
                format!("{off} writes off their logged time, largest {w:+.1} T-states{at}")
            })
            .unwrap_or_else(|m| m);
            println!("samples CPU: {n} port writes match (tick, port, data); {drift}")
        }
        Err(m) => println!("samples CPU: MISMATCH: {m}"),
    }
    match read_hook(msm) {
        Ok(hook) => {
            match compare_dac(audio2.clone(), tl.clone(), &hook) {
                Ok((ticks, active)) => println!("MSM5205 outputs: identical at every channel tick over the hook ({ticks} ticks, {active} active)"),
                Err(m) => println!("MSM5205 outputs: MISMATCH: {m}"),
            }
            let (n, d, first) = compare_dac_stream(audio2.clone(), tl.clone(), &hook);
            let at = first
                .map(|(t, c)| {
                    format!(
                        ", the first at {t:.7} s (frame {:.1}) on chip {c}",
                        t * 60.0
                    )
                })
                .unwrap_or_default();
            println!("MSM5205 streams at 384 kHz: {d} of {n} samples differ{at}");
        }
        Err(e) => println!("MSM5205 hook not read ({e}); skipped"),
    }

    if samples_only {
        return;
    }
    // The music CPU from power-on: the window's writes and the YM2151 stream.
    let rom = sf_video::romset::load_region(roms.0, roms.1, "audiocpu").expect("audiocpu");
    let rate = OPM_RATE as f64;
    let (s0, s_end) = (
        ((f0) as f64 / 60.0 * rate).ceil() as u64,
        ((f1 + 1) as f64 / 60.0 * rate).ceil() as u64,
    );
    let run = run_music_from_power_on(rom, tl.clone(), s_end);
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample >= s0 && w.sample < s_end)
        .collect();
    let native: Vec<_> = run
        .writes
        .iter()
        .copied()
        .filter(|w| w.sample >= s0 && w.sample < s_end)
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
        "music CPU: {} reference writes, {} native, {} on the exact clock, {differing} differ in order, largest lag {lag} samples, unrecovered {:04x?}",
        reference.len(),
        native.len(),
        exact_clock_matches(&native, &reference),
        run.unrecovered
    );
    if let Some(i) = (0..k).find(|&i| {
        (native[i].reg, native[i].data) != (reference[i].reg, reference[i].data)
            || native[i].sample.abs_diff(reference[i].sample) > 1
    }) {
        for j in i.saturating_sub(3)..(i + 3).min(k) {
            let (n, r) = (&native[j], &reference[j]);
            println!("  #{j} reference {:02x}={:02x} sample {} clock {} | native {:02x}={:02x} sample {} clock {}", r.reg, r.data, r.sample, r.clock, n.reg, n.data, n.sample, n.clock);
        }
    }
    match read_device_stream(audio, "ymsnd", rate) {
        Ok(hook) => {
            let n0 = hook.first_sample();
            let (mut compared, mut differing) = (0u64, 0u64);
            let mut first = None;
            for (i, r) in hook.samples.iter().enumerate() {
                let n = n0 + i as u64;
                if n >= s0 && n < s_end {
                    compared += 1;
                    if run.left[n as usize] != *r {
                        differing += 1;
                        first.get_or_insert(n);
                    }
                }
            }
            println!(
                "YM2151 output: {compared} samples compared, {differing} differ{}",
                first
                    .map(|n| format!(
                        " (first at sample {n}, frame {:.1})",
                        n as f64 / rate * 60.0
                    ))
                    .unwrap_or_default()
            );
        }
        Err(e) => println!("ymsnd hook not read ({e}); skipped"),
    }

    // The mixed recording: both chips as MAME mixes them, measured against
    // the reference's audio.wav (SF_WAV_REF) over the window; the native
    // mix written to SF_MIX_OUT when set.
    let wav_ref = match std::env::var("SF_WAV_REF") {
        Ok(p) => p,
        Err(_) => return,
    };
    let audio2 = sf_video::romset::load_region(roms.0, roms.1, "audio2").expect("audio2");
    let rom = sf_video::romset::load_region(roms.0, roms.1, "audiocpu").expect("audiocpu");
    let native = render_mix(audio2, rom, tl, f0, f1);
    let (reference, wav_rate) = read_wav_mono(Path::new(&wav_ref)).expect("reference audio.wav");
    assert_eq!(wav_rate, WAV_RATE, "the reference recording's rate");
    let start = (f0 as f64 / 60.0 * WAV_RATE as f64).round() as usize;
    let reference = &reference[start..(start + native.len()).min(reference.len())];
    let (mn, mr) = (
        measure(&native, WAV_RATE as f64),
        measure(reference, WAV_RATE as f64),
    );
    for (name, m) in [("native", &mn), ("reference", &mr)] {
        println!(
            "mix {name}: onset {:.2} ms, active {:.1} ms, rms {:.4}, peak {:.4}, zero-crossings {:.0}/s, centroid {:.0} Hz",
            m.onset * 1e3,
            m.active * 1e3,
            m.rms,
            m.peak,
            m.zcr,
            m.centroid
        );
    }
    let (c, lag) = best_correlation(&native, reference, (0.020 * WAV_RATE as f64) as isize);
    println!("mix: {} samples, normalized correlation {c:.4} at lag {:.2} ms (reference relative to native), rms ratio {:.4}", native.len(), lag as f64 / WAV_RATE as f64 * 1e3, mn.rms / mr.rms);
    // Per second of the window: where the two recordings agree least.
    let sec = WAV_RATE as usize;
    let mut worst = (1.0f64, 0usize);
    for (j, (a, b)) in native.chunks(sec).zip(reference.chunks(sec)).enumerate() {
        let (c, _) = best_correlation(a, b, (0.002 * WAV_RATE as f64) as isize);
        if c < worst.0 {
            worst = (c, j);
        }
    }
    println!(
        "mix: least-correlated second {} (t = {:.2} s): {:.4}",
        worst.1,
        f0 as f64 / 60.0 + worst.1 as f64,
        worst.0
    );
    if let Ok(out) = std::env::var("SF_MIX_OUT") {
        sf_audio::write_wav(Path::new(&out), &native, WAV_RATE, 1).expect("write the native mix");
        println!("native mix written to {out}");
    }
}
