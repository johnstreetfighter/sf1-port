//! Debug aid: the music CPU's idle-loop phase at a reference fixture
//! (local-only data). Runs the translated music CPU from the fixture at
//! the end of frame F through frame G for each of the loop's 16 phases
//! and reports, per phase, how many native YM2151 writes fall on the
//! reference's exact chip clock, how many writes differ in order, and the
//! largest sample lag — the phase with the most exact clocks is the
//! fixture's (docs/reference/music-phrase.md, latch-timing.md).
//!
//!   cargo run -p sf-audio --release --example music_phase -- <audio capture> <capture with ram_F_music.bin> F G

use std::path::Path;

use sf_audio::check::{
    exact_clock_matches, latch_timeline, read_taps, reference_opm_writes, run_music,
};
use sf_audio::opm::OPM_RATE;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (audio, capture) = (Path::new(&args[1]), Path::new(&args[2]));
    let (f, g): (u32, u32) = (args[3].parse().unwrap(), args[4].parse().unwrap());
    let log = read_taps(&audio.join("audio_taps.tsv")).expect("tap log");
    let rom = sf_video::romset::load_region(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
        "audiocpu",
    )
    .expect("audiocpu");
    let ram =
        std::fs::read(capture.join(format!("ram_{f:06}_music.bin"))).expect("fixture music RAM");
    let rate = OPM_RATE as f64;
    let t_fix = (f + 1) as f64 / 60.0;
    let s_fix = (t_fix * rate).ceil() as u64;
    let s_end = ((g + 1) as f64 / 60.0 * rate).ceil() as u64;
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample >= s_fix && w.sample < s_end)
        .collect();
    println!(
        "fixture end of frame {f} ({t_fix:.6} s), through frame {g}: {} reference writes",
        reference.len()
    );
    let phases: Vec<u64> = match args.get(5) {
        Some(p) => vec![p.parse().unwrap()],
        None => (0..16).collect(),
    };
    for phase in phases {
        let run = run_music(
            rom.clone(),
            ram.clone(),
            &log,
            t_fix,
            latch_timeline(&log, None),
            phase,
            s_end,
        );
        let native: Vec<_> = run
            .writes
            .into_iter()
            .filter(|w| w.sample < s_end)
            .collect();
        let exact = exact_clock_matches(&native, &reference);
        let differing = native
            .iter()
            .zip(&reference)
            .filter(|(n, r)| (n.reg, n.data) != (r.reg, r.data))
            .count();
        let lag = native
            .iter()
            .zip(&reference)
            .map(|(n, r)| n.sample.abs_diff(r.sample))
            .max()
            .unwrap_or(0);
        println!(
            "phase {phase:>2}: {} native writes, {exact} on the exact clock, {differing} differ in order, largest lag {lag} samples, unrecovered {:04x?}",
            native.len(),
            run.unrecovered
        );
        // With a single phase: every write off by more than a sample, or
        // the first difference in order, with its neighbours.
        if args.get(5).is_some() {
            let clock = sf_audio::opm::OPM_CLOCK as i64;
            let show = |i: usize| {
                let (n, r) = (&native[i], &reference[i]);
                println!(
                    "  #{i} reg {:02x} data {:02x} | native sample {} clock {} | reference sample {} clock {} | {:+} T ({:+.3} ms)",
                    r.reg,
                    r.data,
                    n.sample,
                    n.clock,
                    r.sample,
                    r.clock,
                    n.clock as i64 - r.clock as i64,
                    (n.clock as i64 - r.clock as i64) as f64 / clock as f64 * 1000.0
                );
            };
            let mut shown = 0;
            for i in 0..native.len().min(reference.len()) {
                let (n, r) = (&native[i], &reference[i]);
                if (n.reg, n.data) != (r.reg, r.data) || n.sample.abs_diff(r.sample) > 1 {
                    println!(
                        "write #{i} {}:",
                        if (n.reg, n.data) != (r.reg, r.data) {
                            "differs"
                        } else {
                            "lags"
                        }
                    );
                    for j in i.saturating_sub(3)..(i + 4).min(native.len().min(reference.len())) {
                        show(j);
                    }
                    shown += 1;
                    if shown >= 6 || (n.reg, n.data) != (r.reg, r.data) {
                        break;
                    }
                }
            }
            if native.len() != reference.len() {
                let k = native.len().min(reference.len());
                println!(
                    "counts differ: native {} reference {}; around #{k}:",
                    native.len(),
                    reference.len()
                );
                for j in k.saturating_sub(4)..k {
                    show(j);
                }
            }
            // SF_SHOW_OFF=n: the first n writes off the exact clock, each
            // with the two writes before it and the latch writes (NMIs)
            // between its predecessor and itself.
            if let Ok(n) = std::env::var("SF_SHOW_OFF") {
                let n: usize = n.parse().unwrap();
                let k = native.len().min(reference.len());
                let mut shown = 0;
                for i in 1..k {
                    if native[i].clock != reference[i].clock
                        && native[i - 1].clock == reference[i - 1].clock
                    {
                        println!(
                            "off from write #{i} (sample lag {}):",
                            native[i].sample as i64 - reference[i].sample as i64
                        );
                        for j in i.saturating_sub(2)..(i + 3).min(k) {
                            show(j);
                        }
                        let (t0, t1) = (
                            reference[i - 1].clock as f64 / clock as f64,
                            reference[i].clock as f64 / clock as f64,
                        );
                        for &(t, v) in log.latch.iter().filter(|e| e.0 >= t0 - 0.0005 && e.0 <= t1)
                        {
                            println!(
                                "  latch {v:02x} at {t:.9} s (clock {})",
                                (t * clock as f64).ceil() as u64
                            );
                        }
                        shown += 1;
                        if shown >= n {
                            break;
                        }
                    }
                }
            }
            // SF_SHOW=a..b: those writes on both sides, and the latch
            // writes (the NMIs) between the first and the last of them.
            if let Ok(range) = std::env::var("SF_SHOW") {
                let (a, b) = range.split_once("..").unwrap();
                let (a, b): (usize, usize) = (a.parse().unwrap(), b.parse().unwrap());
                let k = native.len().min(reference.len());
                for j in a..b.min(k) {
                    show(j);
                }
                let (t0, t1) = (
                    reference[a].clock as f64 / clock as f64,
                    reference[b.min(k) - 1].clock as f64 / clock as f64,
                );
                for &(t, v) in log
                    .latch
                    .iter()
                    .filter(|e| e.0 >= t0 - 0.02 && e.0 <= t1 + 0.001)
                {
                    println!(
                        "  latch {v:02x} at {t:.9} s (clock {})",
                        (t * clock as f64).ceil() as u64
                    );
                }
            }
        }
    }
}
