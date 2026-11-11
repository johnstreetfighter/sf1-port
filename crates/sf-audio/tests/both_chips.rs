//! Acceptance tests for tickets #37 and #48 (F8): both translated sound
//! programs driven from the native engine's latch over every recorded fight
//! (`sf-fixed-fight-01`, `sf-punch-01`, and `sf-win-01`). Each native run
//! supplies the per-frame latch values and the vblank handler's cycles from
//! the splice frame after the fixture; each write is placed at the frame
//! start plus the handler's cycles plus an interrupt latency from the
//! observed band (`latch-timing.md`). The samples CPU and the music CPU run
//! from power-on on that timeline (the reference's latch through the
//! fixture frame), and their outputs are compared with the reference's
//! hooked device streams (`docs/reference/both-chips.md`).
//!
//! At the reference's own write times (`native_at_reference_times`), every
//! device is identical across all three fights: every samples-CPU port write
//! at its logged time, both MSM5205 streams at every 384 kHz sample (round 1
//! and round 2 of `sf-fixed-fight-01`, `sf-punch-01`, and `sf-win-01`), every
//! music-CPU YM2151 write on the exact chip clock, and every YM2151 output
//! sample. Under a constant interrupt latency from `45..=138`:
//! - `sf-fixed-fight-01` (`2389..=4999`): samples-CPU data and per-tick
//!   MSM5205 levels match on `45..=54` (`55..=138` differ from frame `2619`),
//!   while write times and 384 kHz stream samples drift from frame `2391`;
//!   YM2151 output matches over the full fight on `53..=76` (and over round 2
//!   on `49..=76`), giving a joint full-fight window of `53..=54` (`49..=54`
//!   on round 2).
//! - `sf-punch-01` (`2450..=3399`): samples-CPU data and per-tick MSM5205
//!   levels match across `45..=138`, write times and 384 kHz stream samples
//!   match identically on `45..=113` (differing from frame `2474` on
//!   `114..=138`), and YM2151 output matches on `46..=83` (differing at
//!   frame `2853` on `45`, frame `3162` on `84..=133`, and frame `2856` on
//!   `134..=138`), giving a joint window `46..=83` where every device and
//!   stream is identical.
//! - `sf-win-01` (`2389..=6251`): no single constant latency in `45..=138`
//!   fits: samples-CPU data and per-tick MSM5205 levels differ at frame
//!   `2535` on `45..=81` and at frame `2595` on `82..=138`, while YM2151
//!   output matches through frame `5636` on `45..=78` (first differing at
//!   frame `5637`, `238` samples; `5,602` at frame `5637` on `79..=80`) and
//!   first differs at frame `3511` on `81..=138`.
//!
//! Local-only data; skips without it.

use std::path::Path;

use sf_audio::check::{
    best_correlation, compare_dac, compare_dac_stream, exact_clock_matches, frame_tick,
    latch_timeline, measure, native_at_reference_times, native_latch, read_device_stream,
    read_hook, read_taps, read_wav_mono, reference_opm_writes, render_mix, run_music_from_power_on,
    write_time_drift, NativeLatch, RefLog, WAV_RATE,
};
use sf_audio::opm::OPM_RATE;
use sf_audio::{LatchTimeline, IRQ_LATENCY_CYCLES, IRQ_LATENCY_MID};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
/// The round-1 fixture capture (checkpoint 2389) the native run starts from.
const FIGHT_REF: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_01.json";
/// The reference's taps and the whole-run YM2151 stream.
const YM_HOOK: &str = "../../tools/reference/local/captures/fight-audio-ym";
/// Both MSM5205 streams over frames 3830..=5000.
const MSM_HOOK: &str = "../../tools/reference/local/captures/fight-audio-r2";
/// The reference's mixed recording (MAME `-wavwrite`, 48 kHz stereo).
const WAV_REF: &str = "../../tools/reference/local/captures/fight-audio-a/audio.wav";
const NATIVE_FROM: u32 = 2389;
const NATIVE_LAST: u32 = 5000;
/// The first native latch row: the fixture frame's row is the checkpoint's
/// latch with no handler cycles computed (the engine did not run that
/// frame's vblank), so the reference's own write is kept for it.
const SPLICE: u32 = NATIVE_FROM + 1;
const ROUND2_FIRST: u32 = 3830;
const ROUND2_LAST: u32 = 4999;
/// A constant interrupt latency inside the window (49..=53 cycles) at which
/// both devices reproduce this fight's round 2 exactly. Fitted to this
/// fight, not a model of the latency; the launcher keeps the band's middle.
const FITTED_LATENCY: u32 = 51;

fn present() -> bool {
    [FIGHT_REF, YM_HOOK, MSM_HOOK, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

/// The native fixed-fight run's per-frame latch (value, handler cycles),
/// produced by the native engine once per test process.
fn native_fight_latch() -> Vec<(u32, u8, u32)> {
    native_fight().0.clone()
}

/// A native run's latch rows (frame, value, handler cycles) and its
/// (frame, screen hash) rows.
type NativeRun = (Vec<(u32, u8, u32)>, Vec<(u32, String)>);

/// The native fixed-fight run: its latch rows and its screen hashes.
fn native_fight() -> &'static NativeRun {
    static RUN: std::sync::OnceLock<NativeRun> = std::sync::OnceLock::new();
    RUN.get_or_init(|| {
        let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions");
        let out = sf_engine::run::ScratchDir::new("sf-audio-both");
        sf_engine::run::run_cpu_fight_capture_with_inputs(
            Path::new(FIGHT_REF),
            &assets,
            &out,
            "sf-fixed-fight-01",
            Some(Path::new(SCENARIO)),
            NATIVE_FROM,
            NATIVE_LAST,
        )
        .expect("native fixed-fight run");
        (
            native_latch(&out).expect("native latch"),
            screen_hashes(&out),
        )
    })
}

/// (frame, screen hash) per row of a capture's frames.tsv.
fn screen_hashes(capture: &Path) -> Vec<(u32, String)> {
    std::fs::read_to_string(capture.join("frames.tsv"))
        .expect("frames.tsv")
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut c = l.split('\t');
            Some((c.next()?.parse().ok()?, c.next()?.to_string()))
        })
        .collect()
}

/// The native run the sound is driven from is the reference's fight: its
/// screen hash equals the reference's on every frame after the fixture row
/// (the fixture row's hash is the checkpoint's own scanout), and its latch
/// value on every frame.
#[test]
fn the_native_fight_matches_the_reference_frame_for_frame() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let (latch, hashes) = native_fight();
    const FIGHT_AUDIO: &str = "../../tools/reference/local/captures/fight-audio-a";
    let reference: std::collections::HashMap<u32, String> =
        screen_hashes(Path::new(FIGHT_AUDIO)).into_iter().collect();
    let mut compared = 0;
    for (frame, hash) in hashes.iter().filter(|(f, _)| *f > NATIVE_FROM) {
        let Some(r) = reference.get(frame) else {
            continue;
        };
        assert_eq!(
            hash, r,
            "frame {frame}: native screen hash {hash}, reference {r}"
        );
        compared += 1;
    }
    assert_eq!(
        compared,
        (ROUND2_LAST - NATIVE_FROM) as usize,
        "every frame from {} through {ROUND2_LAST} compared",
        NATIVE_FROM + 1
    );
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let by_frame: std::collections::HashMap<u32, u8> = log
        .latch
        .iter()
        .map(|&(t, v)| ((t * 60.0).floor() as u32, v))
        .collect();
    let mut values = 0;
    for &(frame, value, _) in latch.iter().filter(|r| r.0 <= ROUND2_LAST) {
        if let Some(&r) = by_frame.get(&frame) {
            assert_eq!(
                value, r,
                "frame {frame}: native latch {value:02x}, reference {r:02x}"
            );
            values += 1;
        }
    }
    assert!(
        values as u32 > ROUND2_LAST - NATIVE_FROM,
        "latch values compared ({values})"
    );
    println!("native fight: {compared} screen hashes (frames {}..={ROUND2_LAST}) and {values} latch values equal the reference's", NATIVE_FROM + 1);
}

struct Devices {
    /// (writes compared, writes off their logged time, the largest
    /// difference in T-states), or the first differing write.
    samples_writes: Result<(usize, usize, f64), String>,
    dac: Result<(u64, u64), String>,
    /// Both 384 kHz streams: (samples compared, samples differing).
    dac_stream: (u64, u64),
    first_dac_stream_diff_frame: Option<u32>,
    music_writes: usize,
    music_in_order: bool,
    music_lag: u64,
    music_exact: usize,
    ym_compared: u64,
    ym_differing: u64,
    first_ym_diff_frame: Option<u32>,
    unrecovered: Vec<u16>,
}

/// Both sound CPUs from power-on on `tl`, each device against its hooked
/// stream over round 2.
fn run_devices(log: &RefLog, tl: LatchTimeline) -> Devices {
    run_devices_window(log, YM_HOOK, MSM_HOOK, ROUND2_FIRST, ROUND2_LAST, tl)
}

/// Both sound CPUs from power-on on `tl`, each device against its hooked
/// stream over frames `first..=last`.
fn run_devices_window(
    log: &RefLog,
    ym_hook_dir: &str,
    msm_hook_dir: &str,
    first: u32,
    last: u32,
    tl: LatchTimeline,
) -> Devices {
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2 region");
    let samples_writes = write_time_drift(
        audio2.clone(),
        tl.clone(),
        &log.writes,
        frame_tick(first),
        frame_tick(last + 1),
    )
    .map(|(n, off, worst, _)| (n, off, worst));
    let hook = read_hook(Path::new(msm_hook_dir)).expect("MSM5205 hook");
    let dac = compare_dac(audio2.clone(), tl.clone(), &hook);
    let (stream_compared, stream_differing, first_stream_diff) =
        compare_dac_stream(audio2, tl.clone(), &hook);
    let first_dac_stream_diff_frame = first_stream_diff.map(|(t, _)| (t * 60.0).floor() as u32);

    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region");
    let rate = OPM_RATE as f64;
    let (s0, s_end) = (
        (first as f64 / 60.0 * rate).ceil() as u64,
        ((last + 1) as f64 / 60.0 * rate).ceil() as u64,
    );
    let run = run_music_from_power_on(rom, tl, s_end);
    let reference: Vec<_> = reference_opm_writes(log)
        .into_iter()
        .filter(|w| w.sample >= s0 && w.sample < s_end)
        .collect();
    let native: Vec<_> = run
        .writes
        .iter()
        .copied()
        .filter(|w| w.sample >= s0 && w.sample < s_end)
        .collect();
    let music_in_order = native.len() == reference.len()
        && native
            .iter()
            .zip(&reference)
            .all(|(n, r)| (n.reg, n.data) == (r.reg, r.data));
    let music_lag = native
        .iter()
        .zip(&reference)
        .map(|(n, r)| n.sample.abs_diff(r.sample))
        .max()
        .unwrap_or(0);
    let music_exact = exact_clock_matches(&native, &reference);
    let ym = read_device_stream(Path::new(ym_hook_dir), "ymsnd", rate).expect("ymsnd hook");
    let n0 = ym.first_sample();
    let (mut ym_compared, mut ym_differing, mut first_ym_diff_frame) = (0u64, 0u64, None);
    for (i, r) in ym.samples.iter().enumerate() {
        let n = n0 + i as u64;
        if n >= s0 && n < s_end {
            ym_compared += 1;
            if run.left[n as usize] != *r {
                ym_differing += 1;
                first_ym_diff_frame.get_or_insert(((n as f64 / rate) * 60.0).floor() as u32);
            }
        }
    }
    Devices {
        samples_writes,
        dac,
        dac_stream: (stream_compared, stream_differing),
        first_dac_stream_diff_frame,
        music_writes: reference.len(),
        music_in_order,
        music_lag,
        music_exact,
        ym_compared,
        ym_differing,
        first_ym_diff_frame,
        unrecovered: run.unrecovered,
    }
}

/// The native values at the reference's write times: both devices identical
/// over round 2, the samples CPU's writes at their logged times and both
/// MSM5205 streams at every 384 kHz sample — the translation's whole
/// residual is the latency.
#[test]
fn both_chips_are_identical_over_round_2_at_the_reference_write_times() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let d = run_devices(&log, native_at_reference_times(&log, &frames, SPLICE));
    assert!(
        d.unrecovered.is_empty(),
        "translated paths only: {:04x?}",
        d.unrecovered
    );
    let (n, off, _) = d
        .samples_writes
        .unwrap_or_else(|m| panic!("samples CPU: {m}"));
    assert!(
        n > 140_000 && off == 0,
        "the round's samples-CPU writes ({n}), {off} off their logged time"
    );
    let (ticks, active) = d.dac.unwrap_or_else(|m| panic!("MSM5205 outputs: {m}"));
    assert!(
        ticks > 150_000 && active > 40_000,
        "both MSM5205 streams compared over the round ({ticks} ticks, {active} active)"
    );
    let (samples, differing) = d.dac_stream;
    assert!(
        samples > 14_000_000 && differing == 0,
        "MSM5205 streams at 384 kHz: {differing} of {samples} samples differ"
    );
    assert!(
        d.music_in_order && d.music_lag == 0 && d.music_exact == d.music_writes,
        "music CPU: {} writes, {} exact, lag {}",
        d.music_writes,
        d.music_exact,
        d.music_lag
    );
    assert!(
        d.ym_compared > 1_000_000 && d.ym_differing == 0,
        "YM2151 output: {} of {} samples differ",
        d.ym_differing,
        d.ym_compared
    );
    println!(
        "reference times: samples {n} writes at their logged times; MSM5205 {samples} stream samples identical; music {} writes exact, {} YM2151 samples identical",
        d.music_writes, d.ym_compared
    );
}

/// From native gameplay at a constant latency inside the window: the
/// samples CPU's port writes (tick, port, data) and both MSM5205 levels at
/// every channel tick identical, the YM2151's output identical at every
/// sample with the music CPU's writes in order. The samples CPU's writes
/// drift within their ticks (the constant latency puts some writes on the
/// other side of a poll than the reference's own latency did, which
/// changes a branch's timing but not the data), so a fraction of the
/// MSM5205 streams' 384 kHz samples differ; reported and bounded.
#[test]
fn both_chips_are_identical_over_round_2_from_native_gameplay() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let l = FITTED_LATENCY;
    let d = run_devices(
        &log,
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE,
                latency: l,
            }),
        ),
    );
    assert!(
        d.unrecovered.is_empty(),
        "translated paths only: {:04x?}",
        d.unrecovered
    );
    let (n, off, worst) = d
        .samples_writes
        .unwrap_or_else(|m| panic!("samples CPU at latency {l}: {m}"));
    assert!(n > 140_000, "the round's samples-CPU writes ({n})");
    let (ticks, active) = d
        .dac
        .unwrap_or_else(|m| panic!("MSM5205 outputs at latency {l}: {m}"));
    assert!(
        ticks > 150_000 && active > 40_000,
        "both MSM5205 streams compared ({ticks} ticks, {active} active)"
    );
    let (samples, differing) = d.dac_stream;
    assert!(
        differing * 1000 / samples <= 5,
        "MSM5205 streams at 384 kHz: {differing} of {samples} samples differ (more than 0.5%)"
    );
    assert!(
        d.music_in_order && d.music_lag <= 2,
        "music CPU at latency {l}: in order within two samples (lag {})",
        d.music_lag
    );
    assert!(
        d.ym_compared > 1_000_000 && d.ym_differing == 0,
        "YM2151 output at latency {l}: {} of {} samples differ",
        d.ym_differing,
        d.ym_compared
    );
    println!(
        "latency {l}: samples {n} writes + {ticks} DAC ticks identical ({off} writes off their logged time, largest {worst:+.0} T-states; {differing} of {samples} MSM5205 stream samples differ); music {} writes in order ({} on the exact clock, lag {}); YM2151 {} samples identical",
        d.music_writes, d.music_exact, d.music_lag, d.ym_compared
    );
}

/// From native gameplay at the band's low end, middle and high end: the
/// music CPU's writes in order within two samples at each; at the low end
/// the samples CPU and both MSM5205 outputs identical and the YM2151's
/// output within 0.5% of its samples; the rest reported.
#[test]
fn both_chips_from_native_gameplay_over_round_2() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let low = *IRQ_LATENCY_CYCLES.start();
    let d = run_devices(
        &log,
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE,
                latency: low,
            }),
        ),
    );
    assert!(
        d.unrecovered.is_empty(),
        "translated paths only: {:04x?}",
        d.unrecovered
    );
    let (n, _, _) = d
        .samples_writes
        .unwrap_or_else(|m| panic!("samples CPU at latency {low}: {m}"));
    let (ticks, _) = d
        .dac
        .unwrap_or_else(|m| panic!("MSM5205 outputs at latency {low}: {m}"));
    assert!(
        d.music_in_order,
        "music CPU: every write in order at latency {low}"
    );
    assert!(
        d.music_lag <= 2,
        "music CPU: largest lag {} samples",
        d.music_lag
    );
    let permille = d.ym_differing * 1000 / d.ym_compared;
    assert!(
        permille <= 5,
        "YM2151 output: {} of {} samples differ (more than 0.5%)",
        d.ym_differing,
        d.ym_compared
    );
    println!(
        "latency {low}: samples {n} writes + {ticks} DAC ticks identical; music {} writes in order, {} exact, lag {}; YM2151 {} of {} samples differ",
        d.music_writes, d.music_exact, d.music_lag, d.ym_differing, d.ym_compared
    );
    // The band's middle and high end: the samples CPU's outcome depends on
    // the latency (a command's write window straddles a poll); reported.
    for latency in [IRQ_LATENCY_MID, *IRQ_LATENCY_CYCLES.end()] {
        let d = run_devices(
            &log,
            latch_timeline(
                &log,
                Some(NativeLatch {
                    frames: &frames,
                    from: SPLICE,
                    latency,
                }),
            ),
        );
        assert!(
            d.music_in_order && d.music_lag <= 2,
            "music CPU at latency {latency}: in order within two samples"
        );
        println!(
            "latency {latency}: samples CPU {}; MSM5205 {}; music {} exact of {}, lag {}; YM2151 {} of {} samples differ",
            d.samples_writes.as_ref().map(|(n, _, _)| format!("{n} writes match")).unwrap_or_else(|m| format!("differ ({})", m.lines().next().unwrap_or(""))),
            d.dac.as_ref().map(|(t, _)| format!("identical over {t} ticks")).unwrap_or_else(|m| format!("differ ({})", m.lines().next().unwrap_or(""))),
            d.music_exact,
            d.music_writes,
            d.music_lag,
            d.ym_differing,
            d.ym_compared
        );
    }
}

/// Both chips mixed as MAME mixes them (YM2151 at 0.6, each MSM5205 at
/// 1.0, the mono average at 48 kHz), measured against the reference's
/// recording over round 2 at the fitted latency: the same onset and active
/// span, RMS within 2%, correlation above 0.98 at no more than 8 samples'
/// lag. The residual is
/// the output resampling (MAME band-limits; the native mix holds the DACs
/// and interpolates the chip), not the synthesis — the per-device streams
/// are compared exactly above. Machine-measured; no listening judgement.
#[test]
fn the_mixed_output_over_round_2_measures_like_the_reference_recording() {
    if !present() || !Path::new(WAV_REF).is_file() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let low = FITTED_LATENCY;
    let tl = latch_timeline(
        &log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE,
            latency: low,
        }),
    );
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2 region");
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region");
    let native = render_mix(audio2, rom, tl, ROUND2_FIRST, ROUND2_LAST);
    let (reference, rate) = read_wav_mono(Path::new(WAV_REF)).expect("reference recording");
    assert_eq!(rate, WAV_RATE, "the reference recording's rate");
    let start = (ROUND2_FIRST as f64 / 60.0 * WAV_RATE as f64).round() as usize;
    let reference = &reference[start..start + native.len()];
    let (mn, mr) = (
        measure(&native, WAV_RATE as f64),
        measure(reference, WAV_RATE as f64),
    );
    let (c, lag) = best_correlation(&native, reference, (0.020 * WAV_RATE as f64) as isize);
    println!(
        "mix at latency {low}: onset {:.0}/{:.0} ms, active {:.0}/{:.0} ms, rms {:.4}/{:.4}, correlation {c:.4} at lag {} samples",
        mn.onset * 1e3, mr.onset * 1e3, mn.active * 1e3, mr.active * 1e3, mn.rms, mr.rms, lag
    );
    assert!(
        (mn.onset - mr.onset).abs() < 0.006 && (mn.active - mr.active).abs() < 0.006,
        "onset and active span within a 5 ms envelope window"
    );
    assert!(
        (mn.rms / mr.rms - 1.0).abs() < 0.02,
        "rms {:.4} vs {:.4}",
        mn.rms,
        mr.rms
    );
    assert!(
        c > 0.98 && lag.abs() <= 8,
        "correlation {c:.4} at lag {lag}"
    );
}

/// The live player (`mix::GameSound`, the launcher's output) runs the same
/// music CPU as the verified batch runner: its YM2151 output over round 2
/// is the batch run's, sample for sample, on both channels.
#[test]
fn the_live_players_ym2151_output_is_the_batch_runners() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let low = *IRQ_LATENCY_CYCLES.start();
    let tl = latch_timeline(
        &log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE,
            latency: low,
        }),
    );
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2 region");
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region");
    let rate = OPM_RATE as f64;
    let (t0, t_end) = (ROUND2_FIRST as f64 / 60.0, (ROUND2_LAST + 1) as f64 / 60.0);
    let s_end = (t_end * rate).ceil() as u64;
    let batch = run_music_from_power_on(rom.clone(), tl.clone(), s_end);
    let mut live = sf_audio::mix::GameSound::power_on(audio2, rom, tl, f64::INFINITY);
    assert!(live.skip_to(t0));
    assert!(live.wait_ready(t_end + 0.001));
    let (base, left, right) = live.ym_samples();
    assert!(
        base <= (t0 * rate).ceil() as u64,
        "the live buffer starts by the round ({base})"
    );
    let mut compared = 0u64;
    for n in (t0 * rate).ceil() as u64..s_end {
        let i = (n - base) as usize;
        let batch_right = 2.0 * batch.mix[n as usize] - batch.left[n as usize];
        assert!(
            left[i] == batch.left[n as usize] && right[i] == batch_right,
            "sample {n}: live ({}, {}) batch ({}, {batch_right})",
            left[i],
            right[i],
            batch.left[n as usize]
        );
        compared += 1;
    }
    assert!(compared > 1_000_000);
    assert!(
        live.unrecovered().is_empty(),
        "translated paths only: {:04x?}",
        live.unrecovered()
    );
    println!(
        "live player: {compared} YM2151 samples (both channels) identical to the batch runner's"
    );
}

/// Frames pushed one at a time while the sound runs — the launcher's way:
/// the fixture frame's row, a pre-roll to it on the reference's latch, then
/// each stepped frame pushed and the sound run as far as the known writes
/// allow — give the same mixed output as a run with every write known
/// from the start, from the fixture through round 2.
#[test]
fn streaming_frame_by_frame_gives_the_same_mix() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let low = *IRQ_LATENCY_CYCLES.start();
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2 region");
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region");
    // From the fixture frame through round 2: the span includes frame
    // 2545, where a player that stopped the music CPU at frame starts (a
    // handler chain then ran past the next frame's write) first diverged.
    let (first, last) = (NATIVE_FROM, ROUND2_LAST);
    let batch = render_mix(
        audio2.clone(),
        rom.clone(),
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE,
                latency: low,
            }),
        ),
        first,
        last,
    );
    // The launcher's order of events: the reference's writes through the
    // fixture frame, the pre-roll to the fixture frame's start, then each
    // stepped frame pushed and the sound run as far as it can before the
    // next — the music thread then waits for that frame inside whatever it
    // is doing.
    let known = SPLICE as f64 / 60.0;
    let preroll = LatchTimeline::new(log.latch.iter().copied().filter(|e| e.0 < known).collect());
    let mut live = sf_audio::mix::GameSound::power_on(audio2, rom, preroll, known);
    let (t0, rate) = (first as f64 / 60.0, WAV_RATE as f64);
    assert!(live.skip_to(t0), "the pre-roll");
    let mut out = Vec::with_capacity(batch.len());
    let mut i = 0usize;
    for &(frame, value, cycles) in frames.iter().filter(|r| r.0 >= SPLICE) {
        live.push_frame(frame, value, cycles, low);
        live.wait_ready((frame + 1) as f64 / 60.0 - 0.001);
        let t_now = t0 + i as f64 / rate;
        let ready = live.ready_until(t_now);
        loop {
            let t = t0 + i as f64 / rate;
            if out.len() == batch.len() || t > ready {
                break;
            }
            let [l, r] = live.sample_at(t);
            out.push((l + r) / 2.0);
            i += 1;
        }
        if out.len() == batch.len() {
            break;
        }
    }
    assert_eq!(out.len(), batch.len(), "the streamed window is complete");
    let differing = out.iter().zip(&batch).filter(|(a, b)| a != b).count();
    assert_eq!(
        differing,
        0,
        "streamed vs batch mix: {differing} of {} samples differ",
        batch.len()
    );
    println!(
        "streaming: {} mixed samples (frames {first}..={last}) identical to the batch mix",
        batch.len()
    );
}

/// The launcher's versus fixture (checkpoint 2450 of sf-move-01) shares its
/// history with sf-punch-01's audio run; the music CPU from power-on on
/// that run's latch reproduces every YM2151 write through the fixture
/// frame on the exact chip clock, so the launcher's pre-roll reaches the
/// reference's sound state there (the one-player fixture's history is the
/// fixed fight's, covered by music_power_on.rs).
#[test]
fn the_versus_fixtures_pre_roll_reproduces_the_reference_music() {
    const PUNCH_AUDIO: &str = "../../tools/reference/local/captures/hit-sound-run-a";
    if !present() || !Path::new(PUNCH_AUDIO).is_dir() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(PUNCH_AUDIO).join("audio_taps.tsv")).expect("tap log");
    let cut = 2451.0 / 60.0;
    let rate = OPM_RATE as f64;
    let count = (cut * rate).floor() as u64;
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region");
    let preroll = LatchTimeline::new(log.latch.iter().copied().filter(|e| e.0 < cut).collect());
    let run = run_music_from_power_on(rom, preroll, count);
    assert!(
        run.unrecovered.is_empty(),
        "translated paths only: {:04x?}",
        run.unrecovered
    );
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample < count)
        .collect();
    let native: Vec<_> = run
        .writes
        .iter()
        .copied()
        .filter(|w| w.sample < count)
        .collect();
    assert!(
        reference.len() > 50_000,
        "the pre-fixture music ({} writes)",
        reference.len()
    );
    assert_eq!(
        native.len(),
        reference.len(),
        "write count before the fixture"
    );
    let exact = exact_clock_matches(&native, &reference);
    assert_eq!(
        exact,
        reference.len(),
        "every write on the exact clock ({exact} of {})",
        reference.len()
    );
    println!(
        "versus pre-roll: {} YM2151 writes through frame 2450, all on the exact clock",
        reference.len()
    );
}

/// The music CPU's RAM against the reference's per-frame dumps (the
/// milestone audit, ticket #39, docs/reference/milestone-audit.md): at
/// every frame end of 2390..=4999 where the native music CPU is idle, its
/// RAM below the stack (0xc000-0xc5cf; the translation does not keep the
/// Z80's stack) equals the dump of the audit capture (sf-fixed-fight-audit-01:
/// the fixed fight's inputs, a dump after every frame) — at the reference's
/// write times and from native gameplay at the fitted latency.
#[test]
fn the_music_cpus_ram_matches_the_reference_at_idle_frame_ends() {
    const AUDIT: &str = "../../tools/reference/local/captures/fight-audit-run-a";
    if !present() || !Path::new(AUDIT).join("ram_004999_music.bin").is_file() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    const BELOW_STACK: usize = 0x5d0;
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region");
    let window: Vec<u32> = (SPLICE..=ROUND2_LAST).collect();
    let timelines = [
        (
            "the reference's write times",
            native_at_reference_times(&log, &frames, SPLICE),
        ),
        (
            "latency 51",
            latch_timeline(
                &log,
                Some(NativeLatch {
                    frames: &frames,
                    from: SPLICE,
                    latency: FITTED_LATENCY,
                }),
            ),
        ),
    ];
    for (name, tl) in timelines {
        let (rams, unrecovered) =
            sf_audio::check::music_ram_at_frame_ends(rom.clone(), tl, &window);
        assert!(
            unrecovered.is_empty(),
            "translated paths only: {unrecovered:04x?}"
        );
        let (mut idle, mut equal, mut first_diff) = (0u32, 0u32, None);
        for (f, is_idle, ram) in &rams {
            if !is_idle {
                continue;
            }
            idle += 1;
            let dump = std::fs::read(Path::new(AUDIT).join(format!("ram_{f:06}_music.bin")))
                .expect("music dump");
            match (0..BELOW_STACK).find(|&i| dump[i] != ram[i]) {
                None => equal += 1,
                Some(i) => {
                    first_diff.get_or_insert((*f, 0xc000 + i, dump[i], ram[i]));
                }
            }
        }
        println!(
            "music RAM at {name}: {idle} of {} frame ends idle; {equal} equal{}",
            rams.len(),
            first_diff.map(|(f, a, r, n)| format!(", the first differing at frame {f}, 0x{a:04x}: reference {r:02x}, native {n:02x}")).unwrap_or_default()
        );
        assert!(idle > 1000, "idle frame ends compared ({idle})");
        assert_eq!(
            equal,
            idle,
            "music RAM at {name}: {} of {idle} idle frame ends differ",
            idle - equal
        );
    }
}

const MSM_R1_HOOK_A: &str = "../../tools/reference/local/captures/fight-audio-r1-a";
const MSM_R1_HOOK_B: &str = "../../tools/reference/local/captures/fight-audio-r1-b";
const PUNCH_REF: &str = "../../tools/reference/local/captures/punch-01-run-a";
const PUNCH_SCENARIO: &str = "../../tools/reference/scenarios/sf_punch_01.json";
const PUNCH_HOOK_A: &str = "../../tools/reference/local/captures/punch-audio-hook-a";
const PUNCH_HOOK_B: &str = "../../tools/reference/local/captures/punch-audio-hook-b";
const WIN01_REF: &str = "../../tools/reference/local/captures/win-01-run-a";
const WIN01_SCENARIO: &str = "../../tools/reference/scenarios/sf_win_01.json";
const WIN01_HOOK_A: &str = "../../tools/reference/local/captures/win-01-hook-a";
const WIN01_HOOK_B: &str = "../../tools/reference/local/captures/win-01-hook-b";

fn assert_hook_pair_equal(a: &str, b: &str, files: &[&str]) {
    for name in files {
        let xa = std::fs::read(Path::new(a).join(name)).expect(name);
        let xb = std::fs::read(Path::new(b).join(name)).expect(name);
        assert!(xa == xb, "{a} vs {b}: {name} differs");
    }
    let rows = |dir: &str| {
        let mut v: Vec<String> = std::fs::read_to_string(Path::new(dir).join("hook_index.tsv"))
            .expect("hook_index.tsv")
            .lines()
            .map(str::to_string)
            .collect();
        v.sort();
        v
    };
    assert_eq!(rows(a), rows(b), "{a} vs {b}: hook_index.tsv differs");
}

/// Ticket #48 (F8), criterion 1: the three new audio-hook capture pairs
/// (`fight-audio-r1-a/b`, `punch-audio-hook-a/b`, and `win-01-hook-a/b`)
/// repeat byte-for-byte on their tap logs and hooked device streams.
#[test]
fn the_new_hook_runs_repeat_identically() {
    let dirs = [
        MSM_R1_HOOK_A,
        MSM_R1_HOOK_B,
        PUNCH_HOOK_A,
        PUNCH_HOOK_B,
        WIN01_HOOK_A,
        WIN01_HOOK_B,
    ];
    if !dirs.iter().all(|p| Path::new(p).is_dir()) {
        eprintln!("skipping: local F8 hook captures not present");
        return;
    }
    assert_hook_pair_equal(
        MSM_R1_HOOK_A,
        MSM_R1_HOOK_B,
        &["audio_taps.tsv", "hook_msm1.f32", "hook_msm2.f32"],
    );
    assert_hook_pair_equal(
        PUNCH_HOOK_A,
        PUNCH_HOOK_B,
        &[
            "audio_taps.tsv",
            "hook_msm1.f32",
            "hook_msm2.f32",
            "hook_ymsnd.f32",
        ],
    );
    assert_hook_pair_equal(
        WIN01_HOOK_A,
        WIN01_HOOK_B,
        &[
            "audio_taps.tsv",
            "hook_msm1.f32",
            "hook_msm2.f32",
            "hook_ymsnd.f32",
        ],
    );
    println!(
        "new hook pairs: fight-audio-r1, punch-audio-hook, and win-01-hook repeat identically"
    );
}

/// Ticket #48 (F8): round 1 (`2389..=3829`) MSM5205 comparison and full-span
/// (`2389..=4999`) comparison on the fixed fight (`sf-fixed-fight-01`) at
/// the reference's own write times and at the full-fight window boundaries.
#[test]
fn round_1_and_the_whole_fixed_fight_at_the_reference_write_times_and_window_edges() {
    if !present() || !Path::new(MSM_R1_HOOK_A).is_dir() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    const ROUND1_LAST: u32 = 3829;
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");

    // 1. At the reference's own write times over Round 1 (2389..=3829) and the full fight (2389..=4999).
    let d_r1_ref = run_devices_window(
        &log,
        YM_HOOK,
        MSM_R1_HOOK_A,
        NATIVE_FROM,
        ROUND1_LAST,
        native_at_reference_times(&log, &frames, SPLICE),
    );
    assert!(d_r1_ref.unrecovered.is_empty());
    assert_eq!(d_r1_ref.samples_writes.expect("R1 samples").0, 189_711);
    assert_eq!(d_r1_ref.dac.expect("R1 MSM5205"), (192_317, 67_186));
    assert_eq!(d_r1_ref.dac_stream, (18_462_720, 0));

    let d_full_ref = run_devices_window(
        &log,
        YM_HOOK,
        MSM_R1_HOOK_A,
        NATIVE_FROM,
        ROUND2_LAST,
        native_at_reference_times(&log, &frames, SPLICE),
    );
    let (n_full, off_full, _) = d_full_ref.samples_writes.expect("full samples");
    assert_eq!((n_full, off_full), (333_280, 0));
    assert_eq!(
        (d_full_ref.music_writes, d_full_ref.music_exact),
        (79_521, 79_521)
    );
    assert!(d_full_ref.music_in_order && d_full_ref.music_lag <= 1);
    assert_eq!(
        (d_full_ref.ym_compared, d_full_ref.ym_differing),
        (2_433_887, 0)
    );
    println!(
        "fixed fight (reference times): R1 189,711 writes, 192,317 DAC ticks, 18,462,720 stream samples identical; full 2389..=4999 333,280 writes, 79,521 exact YM2151 writes, 2,433,887 YM2151 samples identical"
    );

    // 2. Full-fight window boundaries:
    // - At 53 (inside the joint full-fight window 53..=54): samples data (333,280 writes),
    //   R1 MSM5205 ticks (192,317), and full-fight YM2151 output (2,433,887 samples) all match;
    //   R1 384 kHz stream first differs at frame 2391.
    let d_53 = run_devices_window(
        &log,
        YM_HOOK,
        MSM_R1_HOOK_A,
        NATIVE_FROM,
        ROUND2_LAST,
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE,
                latency: 53,
            }),
        ),
    );
    assert_eq!(d_53.samples_writes.expect("lat 53 samples").0, 333_280);
    assert_eq!(d_53.dac.expect("lat 53 R1 DAC"), (192_317, 67_186));
    assert_eq!(
        (d_53.dac_stream, d_53.first_dac_stream_diff_frame),
        ((18_462_720, 51_393), Some(2391))
    );
    assert_eq!((d_53.ym_compared, d_53.ym_differing), (2_433_887, 0));

    // - At 52: YM2151 first differs in Round 1 at frame 2453 (2,453 samples).
    let d_52 = run_devices_window(
        &log,
        YM_HOOK,
        MSM_R1_HOOK_A,
        NATIVE_FROM,
        ROUND2_LAST,
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE,
                latency: 52,
            }),
        ),
    );
    assert_eq!(
        (d_52.ym_differing, d_52.first_ym_diff_frame),
        (2_453, Some(2453))
    );

    // - At 55: samples-CPU data first differ at frame 2619 (tick 349201), while YM2151 is 0 differing.
    let tl_55 = latch_timeline(
        &log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE,
            latency: 55,
        }),
    );
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2");
    let err_55 = sf_audio::check::compare_writes(
        audio2,
        tl_55.clone(),
        &log.writes,
        frame_tick(NATIVE_FROM),
        frame_tick(ROUND2_LAST + 1),
    )
    .expect_err("lat 55 samples differ");
    assert!(
        err_55.contains("tick 349201"),
        "unexpected diff at lat 55: {err_55}"
    );
    let d_55 = run_devices_window(
        &log,
        YM_HOOK,
        MSM_R1_HOOK_A,
        NATIVE_FROM,
        ROUND2_LAST,
        tl_55,
    );
    assert!(d_55
        .dac
        .expect_err("lat 55 R1 DAC differs")
        .contains("tick 349201"));
    assert_eq!(d_55.ym_differing, 0);
}

fn verify_native_run_matches_reference(
    latch: &[(u32, u8, u32)],
    hashes: &[(u32, String)],
    ref_dir: &str,
    log: &RefLog,
    first: u32,
    last: u32,
) {
    let reference: std::collections::HashMap<u32, String> =
        screen_hashes(Path::new(ref_dir)).into_iter().collect();
    let mut compared = 0usize;
    for (frame, hash) in hashes.iter().filter(|(f, _)| *f > first && *f <= last) {
        let r = reference.get(frame).expect("reference frame hash");
        assert_eq!(
            hash, r,
            "frame {frame}: native screen hash {hash}, reference {r}"
        );
        compared += 1;
    }
    assert_eq!(compared, (last - first) as usize);
    let by_frame: std::collections::HashMap<u32, u8> = log
        .latch
        .iter()
        .map(|&(t, v)| ((t * 60.0).floor() as u32, v))
        .collect();
    let mut values = 0usize;
    for &(frame, value, _) in latch.iter().filter(|r| r.0 >= first && r.0 <= last) {
        let r = *by_frame.get(&frame).expect("reference frame latch");
        assert_eq!(
            value, r,
            "frame {frame}: native latch {value:02x}, reference {r:02x}"
        );
        values += 1;
    }
    assert_eq!(values, (last - first + 1) as usize);
}

/// Ticket #48 (F8): both sound programs on the native latch over the versus
/// punch run (`sf-punch-01`, frames `2450..=3399`).
#[test]
fn the_versus_punch_run_on_the_native_latch() {
    if ![ROMS, PUNCH_REF, PUNCH_HOOK_A]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local sf-punch-01 captures/ROMs not present");
        return;
    }
    const FIRST: u32 = 2450;
    const SPLICE_P: u32 = 2451;
    const LAST: u32 = 3399;

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions");
    let out = sf_engine::run::ScratchDir::new("sf-audio-punch");
    sf_engine::run::run_punch_capture(
        Path::new(PUNCH_REF),
        &assets,
        &out,
        "sf-punch-01",
        Path::new(PUNCH_SCENARIO),
    )
    .expect("native sf-punch-01 run");
    let frames = native_latch(&out).expect("native latch");
    let hashes = screen_hashes(&out);
    let log = read_taps(&Path::new(PUNCH_HOOK_A).join("audio_taps.tsv")).expect("tap log");
    verify_native_run_matches_reference(&frames, &hashes, PUNCH_HOOK_A, &log, FIRST, LAST);

    // 1. At the reference's own write times: every device identical.
    let d_ref = run_devices_window(
        &log,
        PUNCH_HOOK_A,
        PUNCH_HOOK_A,
        FIRST,
        LAST,
        native_at_reference_times(&log, &frames, SPLICE_P),
    );
    assert!(d_ref.unrecovered.is_empty());
    let (n_writes, off, _) = d_ref.samples_writes.expect("punch samples writes");
    assert_eq!((n_writes, off), (68_968, 0));
    assert_eq!(d_ref.dac.expect("punch MSM5205"), (126_877, 13_864));
    assert_eq!(d_ref.dac_stream, (12_180_480, 0));
    assert_eq!(
        (d_ref.music_writes, d_ref.music_exact, d_ref.music_lag),
        (27_705, 27_705, 0)
    );
    assert_eq!((d_ref.ym_compared, d_ref.ym_differing), (885_558, 0));
    println!(
        "sf-punch-01 (reference times): {n_writes} writes at logged times; 126,877 DAC ticks & 12,180,480 stream samples identical; 27,705 exact YM2151 writes & 885,558 output samples identical"
    );

    // 2. Inside the joint window 46..=83 (tested at 51 and 83): every device
    //    AND every 384 kHz stream sample AND every YM2151 sample is identical.
    for lat in [51, 83] {
        let d = run_devices_window(
            &log,
            PUNCH_HOOK_A,
            PUNCH_HOOK_A,
            FIRST,
            LAST,
            latch_timeline(
                &log,
                Some(NativeLatch {
                    frames: &frames,
                    from: SPLICE_P,
                    latency: lat,
                }),
            ),
        );
        assert!(d.unrecovered.is_empty());
        let (n, off, _) = d.samples_writes.expect("samples");
        assert_eq!((n, off), (68_968, 0));
        assert_eq!(d.dac.expect("MSM5205"), (126_877, 13_864));
        assert_eq!(d.dac_stream, (12_180_480, 0));
        assert!(d.music_in_order && d.music_lag <= 1);
        assert_eq!((d.ym_compared, d.ym_differing), (885_558, 0));
    }

    // 3. Band edges and window boundaries on sf-punch-01:
    // - At 45: YM2151 first differs at frame 2853 (344 samples); MSM5205 stream still 0 differing.
    let d_45 = run_devices_window(
        &log,
        PUNCH_HOOK_A,
        PUNCH_HOOK_A,
        FIRST,
        LAST,
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE_P,
                latency: 45,
            }),
        ),
    );
    assert_eq!(d_45.dac_stream, (12_180_480, 0));
    assert_eq!(
        (d_45.ym_differing, d_45.first_ym_diff_frame),
        (344, Some(2853))
    );

    // - At 84 and 92: YM2151 first differs at frame 3162 (5,946 samples); MSM5205 stream still 0 differing.
    for lat in [84, IRQ_LATENCY_MID] {
        let d = run_devices_window(
            &log,
            PUNCH_HOOK_A,
            PUNCH_HOOK_A,
            FIRST,
            LAST,
            latch_timeline(
                &log,
                Some(NativeLatch {
                    frames: &frames,
                    from: SPLICE_P,
                    latency: lat,
                }),
            ),
        );
        assert_eq!(d.dac_stream, (12_180_480, 0));
        assert_eq!((d.ym_differing, d.first_ym_diff_frame), (5_946, Some(3162)));
    }

    // - Through 113, write times and 384 kHz stream samples remain identical (0 differing);
    //   on 114..=138, samples data and per-tick MSM5205 levels still match while write times
    //   and 384 kHz stream samples first differ at frame 2474 (3,559 samples).
    let d_113 = run_devices_window(
        &log,
        PUNCH_HOOK_A,
        PUNCH_HOOK_A,
        FIRST,
        LAST,
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE_P,
                latency: 113,
            }),
        ),
    );
    let (n_113, off_113, _) = d_113.samples_writes.expect("lat 113 samples");
    assert_eq!((n_113, off_113), (68_968, 0));
    assert_eq!(d_113.dac_stream, (12_180_480, 0));

    for (lat, expected_ym_diff, expected_ym_frame) in [(114, 5_946, 3162), (138, 5_950, 2856)] {
        let d = run_devices_window(
            &log,
            PUNCH_HOOK_A,
            PUNCH_HOOK_A,
            FIRST,
            LAST,
            latch_timeline(
                &log,
                Some(NativeLatch {
                    frames: &frames,
                    from: SPLICE_P,
                    latency: lat,
                }),
            ),
        );
        let (n, off, worst) = d
            .samples_writes
            .expect("samples data match across 45..=138");
        assert_eq!((n, off, worst.round() as i64), (68_968, 19_233, -144));
        assert_eq!(
            d.dac.expect("MSM5205 ticks match across 45..=138"),
            (126_877, 13_864)
        );
        assert_eq!(
            (d.dac_stream, d.first_dac_stream_diff_frame),
            ((12_180_480, 3_559), Some(2474))
        );
        assert_eq!(
            (d.ym_differing, d.first_ym_diff_frame),
            (expected_ym_diff, Some(expected_ym_frame))
        );
    }
}

/// Ticket #48 (F8): both sound programs on the native latch over the winning
/// fight (`sf-win-01`, frames `2389..=6251`).
#[test]
fn the_winning_fight_sf_win_01_on_the_native_latch() {
    if ![ROMS, WIN01_REF, WIN01_HOOK_A]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local sf-win-01 captures/ROMs not present");
        return;
    }
    const FIRST: u32 = 2389;
    const SPLICE_W: u32 = 2390;
    const LAST: u32 = 6251;

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions");
    let out = sf_engine::run::ScratchDir::new("sf-audio-win01");
    sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(WIN01_REF),
        &assets,
        &out,
        "sf-win-01",
        Some(Path::new(WIN01_SCENARIO)),
        FIRST,
        LAST,
    )
    .expect("native sf-win-01 run");
    let frames = native_latch(&out).expect("native latch");
    let hashes = screen_hashes(&out);
    let log = read_taps(&Path::new(WIN01_HOOK_A).join("audio_taps.tsv")).expect("tap log");
    verify_native_run_matches_reference(&frames, &hashes, WIN01_HOOK_A, &log, FIRST, LAST);

    // 1. At the reference's own write times: every device identical.
    let d_ref = run_devices_window(
        &log,
        WIN01_HOOK_A,
        WIN01_HOOK_A,
        FIRST,
        LAST,
        native_at_reference_times(&log, &frames, SPLICE_W),
    );
    assert!(d_ref.unrecovered.is_empty());
    let (n_writes, off, _) = d_ref.samples_writes.expect("win-01 samples writes");
    assert_eq!((n_writes, off), (528_845, 0));
    assert_eq!(d_ref.dac.expect("win-01 MSM5205"), (515_197, 185_535));
    assert_eq!(d_ref.dac_stream, (49_459_200, 0));
    assert_eq!((d_ref.music_writes, d_ref.music_exact), (126_015, 126_015));
    assert!(d_ref.music_in_order && d_ref.music_lag <= 1);
    assert_eq!((d_ref.ym_compared, d_ref.ym_differing), (3_600_959, 0));
    println!(
        "sf-win-01 (reference times): {n_writes} writes at logged times; 515,197 DAC ticks & 49,459,200 stream samples identical; 126,015 exact YM2151 writes & 3,600,959 output samples identical"
    );

    // 2. No single constant latency in 45..=138 fits sf-win-01:
    // - On 45..=81 (tested at 45, 78, and 81): samples-CPU data and per-tick MSM5205
    //   levels first differ at frame 2535 (tick 338001).
    // - On 82..=138 (tested at 82, 92, and 138): samples-CPU data and per-tick MSM5205
    //   levels pass frame 2535 and first differ at frame 2595 (tick 346001 / DAC tick 346015).
    // - On 45..=78 (tested at 45 and 78): music-CPU writes stay in order within 2 samples and
    //   YM2151 output matches through frame 5636, first differing at frame 5637 (238 samples).
    // - On 81..=138 (tested at 81, 82, 92, and 138): YM2151 output first differs at frame 3511.
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2");
    for (
        lat,
        expected_tick,
        expected_dac_tick,
        expected_in_order,
        expected_ym_diff,
        expected_ym_frame,
    ) in [
        (45, "tick 338001", "tick 338001", true, 238, 5637),
        (78, "tick 338001", "tick 338001", true, 238, 5637),
        (81, "tick 338001", "tick 338001", false, 78_456, 3511),
        (82, "tick 346001", "tick 346015", false, 78_456, 3511),
        (
            IRQ_LATENCY_MID,
            "tick 346001",
            "tick 346015",
            false,
            78_218,
            3511,
        ),
        (138, "tick 346001", "tick 346015", false, 105_049, 3511),
    ] {
        let tl = latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE_W,
                latency: lat,
            }),
        );
        let cw_err = sf_audio::check::compare_writes(
            audio2.clone(),
            tl.clone(),
            &log.writes,
            frame_tick(FIRST),
            frame_tick(LAST + 1),
        )
        .expect_err("samples differ on sf-win-01");
        assert!(
            cw_err.contains(expected_tick),
            "lat {lat} expected {expected_tick}, got {cw_err}"
        );
        let d = run_devices_window(&log, WIN01_HOOK_A, WIN01_HOOK_A, FIRST, LAST, tl);
        assert!(d.unrecovered.is_empty());
        let dac_err = d.dac.expect_err("DAC ticks differ on sf-win-01");
        assert!(
            dac_err.contains(expected_dac_tick),
            "lat {lat} expected {expected_dac_tick}, got {dac_err}"
        );
        assert_eq!(
            d.music_in_order, expected_in_order,
            "lat {lat} music_in_order"
        );
        if expected_in_order {
            assert!(d.music_lag <= 2, "lat {lat} music_lag {}", d.music_lag);
        }
        assert_eq!(
            (d.ym_differing, d.first_ym_diff_frame),
            (expected_ym_diff, Some(expected_ym_frame))
        );
    }
}

/// Ticket #49 (F9): fixed-fight listening kit over Round 2 (`3830..=4999`):
/// 1. The 51 render's round-2 measures equal `both-chips.md`'s: onset 180 ms,
///    RMS ratio within 2%, correlation >= 0.98 within 8 samples of lag.
/// 2. The 92 render's round-2 measures are recorded (`onset = 180 ms`,
///    `active = 19,225 ms`, `rms = 0.1003`, `zcr = 1858 /s`,
///    `centroid = 3445 Hz`, correlation `0.9743` at `4` samples lag,
///    least-correlated second `0.8750` at `64.83 s`), with no pass threshold.
/// 3. The live capture (pushed frame by frame at `IRQ_LATENCY_MID = 92`)
///    equals the batch mix at latency 92 sample for sample (`0` differing).
/// 4. The renders in `tools/reference/local/renders/` stay local (gitignored).
#[test]
fn fixed_fight_listening_kit_renders_and_live_capture() {
    const RENDERS_DIR: &str = "../../tools/reference/local/renders";
    if !present() || !Path::new(WAV_REF).is_file() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let frames = native_fight_latch();
    let log = read_taps(&Path::new(YM_HOOK).join("audio_taps.tsv")).expect("tap log");
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2");
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu");

    let tl_51 = latch_timeline(
        &log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE,
            latency: FITTED_LATENCY,
        }),
    );
    let tl_92 = latch_timeline(
        &log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE,
            latency: IRQ_LATENCY_MID,
        }),
    );
    let round2_51 = render_mix(
        audio2.clone(),
        rom.clone(),
        tl_51,
        ROUND2_FIRST,
        ROUND2_LAST,
    );
    let round2_92 = render_mix(
        audio2.clone(),
        rom.clone(),
        tl_92,
        ROUND2_FIRST,
        ROUND2_LAST,
    );

    let (reference, rate) = read_wav_mono(Path::new(WAV_REF)).expect("reference audio.wav");
    assert_eq!(rate, WAV_RATE);
    let start = (ROUND2_FIRST as f64 / 60.0 * WAV_RATE as f64).round() as usize;
    let round2_mame = &reference[start..start + round2_51.len()];

    // Stream frame by frame at latency 92 (as the launcher does) and drain Round 2.
    let known = SPLICE as f64 / 60.0;
    let preroll = LatchTimeline::new(log.latch.iter().copied().filter(|e| e.0 < known).collect());
    let mut live = sf_audio::mix::GameSound::power_on(audio2, rom, preroll, known);
    let (t_fix, t0, rate_f) = (
        NATIVE_FROM as f64 / 60.0,
        ROUND2_FIRST as f64 / 60.0,
        WAV_RATE as f64,
    );
    assert!(live.skip_to(t_fix));
    let mut t_adv = t_fix;
    let mut live_r2 = Vec::with_capacity(round2_92.len());
    for &(frame, value, cycles) in frames.iter().filter(|r| r.0 >= SPLICE) {
        live.push_frame(frame, value, cycles, IRQ_LATENCY_MID);
        let frame_ready = (frame + 1) as f64 / 60.0 - 0.001;
        live.wait_ready(frame_ready);
        if t_adv < t0 {
            let step = frame_ready.min(t0);
            if step > t_adv {
                live.skip_to(step);
                t_adv = step;
            }
            if t_adv < t0 {
                continue;
            }
        }
        let t_now = t0 + live_r2.len() as f64 / rate_f;
        let ready = live.ready_until(t_now);
        while live_r2.len() < round2_92.len() {
            let t = t0 + live_r2.len() as f64 / rate_f;
            if t > ready {
                break;
            }
            let [l, r] = live.sample_at(t);
            live_r2.push((l + r) / 2.0);
        }
    }
    assert_eq!(live_r2.len(), round2_92.len());
    let live_diff = live_r2
        .iter()
        .zip(&round2_92)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        live_diff, 0,
        "live capture must equal batch mix at latency 92 sample for sample"
    );

    // Write local renders into tools/reference/local/renders/.
    let renders = Path::new(RENDERS_DIR);
    std::fs::create_dir_all(renders).expect("create local renders dir");
    for (name, pcm) in [
        ("round2-native-mix-51.wav", &round2_51[..]),
        ("round2-native-mix-92.wav", &round2_92[..]),
        ("round2-mame-reference.wav", round2_mame),
        ("round2-launcher-live-92.wav", &live_r2[..]),
    ] {
        sf_audio::write_wav(&renders.join(name), pcm, WAV_RATE, 1).expect("write local render");
    }

    // Criterion 1: 51 render's round-2 measures equal both-chips.md's.
    let (m51, m92, mr) = (
        measure(&round2_51, rate_f),
        measure(&round2_92, rate_f),
        measure(round2_mame, rate_f),
    );
    let max_lag = (0.020 * rate_f) as isize;
    let (c51, lag51) = best_correlation(&round2_51, round2_mame, max_lag);
    let (c92, lag92) = best_correlation(&round2_92, round2_mame, max_lag);
    assert_eq!(
        (m51.onset * 1e3).round() as u32,
        180,
        "51 render onset 180 ms"
    );
    assert_eq!(
        (m51.active * 1e3).round() as u32,
        19_225,
        "51 render active span 19,225 ms"
    );
    assert!(
        (m51.rms / mr.rms - 1.0).abs() < 0.02,
        "51 render RMS ratio within 2%: {:.4}",
        m51.rms / mr.rms
    );
    assert!(
        c51 >= 0.98 && lag51.abs() <= 8,
        "51 render correlation {c51:.4} at lag {lag51}"
    );

    // Criterion 2: 92 render's measures recorded (no pass threshold).
    println!(
        "92 render round-2: onset {:.0} ms, active {:.0} ms, rms {:.4} (ratio {:.4}), zcr {:.0}/s, centroid {:.0} Hz, corr {c92:.4} at lag {lag92} samples",
        m92.onset * 1e3, m92.active * 1e3, m92.rms, m92.rms / mr.rms, m92.zcr, m92.centroid
    );
}

const BOOT01_REF: &str = "../../tools/reference/local/captures/boot-01-run-a";
const ATTRACT01_REF: &str = "../../tools/reference/local/captures/attract-01-run-a";
const ATTRACT01_SCENARIO: &str = "../../tools/reference/scenarios/sf_attract_01.json";
const ATTRACT_HOOK_A: &str = "../../tools/reference/local/captures/attract-01-hook-a";
const ATTRACT_HOOK_B: &str = "../../tools/reference/local/captures/attract-01-hook-b";

/// Ticket #55 (F15), criterion 3: both sound programs from power-on over
/// `0..=1647` with no pre-roll:
/// - `attract-01-hook-a` and `attract-01-hook-b` repeat identically;
/// - frame 1's write time (`186,957` 68000 cycles = `0.023369625 s`, placed by
///   `RESET_TO_VBLANK1_CYCLES + 682`, with 3 cycles of E-clock autovector
///   alignment after `186,272` instruction cycles) is exact (`0.0` residual),
///   and every frame `2..=1647` lies inside `IRQ_LATENCY_CYCLES` (`45..=138`);
/// - at the reference's own write times every device is identical over
///   `0..=1647`, and music RAM matches at every idle frame end;
/// - from the native latch (`from: 1`, no pre-roll) at latency 51 and at 92,
///   each device's first differing frame is listed, with no tolerance adopted.
#[test]
fn power_on_warning_and_title_on_the_native_latch() {
    if ![
        ROMS,
        BOOT01_REF,
        ATTRACT01_REF,
        ATTRACT_HOOK_A,
        ATTRACT_HOOK_B,
    ]
    .iter()
    .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local power-on/attract captures or ROMs not present");
        return;
    }
    const FIRST: u32 = 0;
    const SPLICE_PO: u32 = 1;
    const LAST: u32 = 1647;

    assert_hook_pair_equal(
        ATTRACT_HOOK_A,
        ATTRACT_HOOK_B,
        &[
            "audio_taps.tsv",
            "hook_msm1.f32",
            "hook_msm2.f32",
            "hook_ymsnd.f32",
        ],
    );

    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions");
    let out = sf_engine::run::ScratchDir::new("sf-audio-power-on-1647");
    sf_engine::run::run_power_on_scenario_capture(
        &assets,
        &out,
        "sf-attract-01",
        Path::new(ATTRACT01_SCENARIO),
        LAST,
    )
    .expect("native power-on run 0..=1647");
    let frames = native_latch(&out).expect("native latch");
    let hashes = screen_hashes(&out);
    let log = read_taps(&Path::new(ATTRACT_HOOK_A).join("audio_taps.tsv")).expect("tap log");
    verify_native_run_matches_reference(&frames, &hashes, ATTRACT_HOOK_A, &log, SPLICE_PO, LAST);
    assert_eq!(
        hashes.first().map(|(f, h)| (*f, h.as_str())),
        screen_hashes(Path::new(ATTRACT_HOOK_A))
            .first()
            .map(|(f, h)| (*f, h.as_str())),
        "frame 0 screen hash"
    );

    // Verify frame 1's reset-path write time and frames 2..=1647's latency band.
    let residuals = sf_audio::check::latch_residuals(&log, &frames);
    assert_eq!(
        residuals.len(),
        LAST as usize,
        "frames 1..=1647 all have handler cycles"
    );
    let r1 = &residuals[0];
    assert_eq!(
        (r1.frame, r1.cycles, r1.native, r1.reference),
        (1, 186_957, 0x27, 0x27),
        "frame 1 reset-path cycles (186,275 + 682 = 186,957)"
    );
    assert!(
        r1.latency.abs() < 1e-6,
        "frame 1 write time is exact (186,957 / 8,000,000 = 0.023369625 s): residual {}",
        r1.latency
    );
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    for r in &residuals[1..] {
        assert_eq!(r.native, r.reference, "frame {} latch value", r.frame);
        assert!(
            r.latency >= *IRQ_LATENCY_CYCLES.start() as f64 - 0.01
                && r.latency <= *IRQ_LATENCY_CYCLES.end() as f64 + 0.5,
            "frame {} implied latency {:.2} outside 45..=138 (cycles {})",
            r.frame,
            r.latency,
            r.cycles
        );
        min_lat = min_lat.min(r.latency);
        max_lat = max_lat.max(r.latency);
    }
    println!(
        "power-on 0..=1647 latch residuals: frame 1 = {} cycles (residual {:.3}), frames 2..=1647 in {:.2}..={:.2} cycles",
        r1.cycles, r1.latency, min_lat, max_lat
    );

    // 1. At the reference's own write times: every device identical over 0..=1647.
    let tl_ref = native_at_reference_times(&log, &frames, SPLICE_PO);
    let d_ref = run_devices_window(
        &log,
        ATTRACT_HOOK_A,
        ATTRACT_HOOK_A,
        FIRST,
        LAST,
        tl_ref.clone(),
    );
    assert!(d_ref.unrecovered.is_empty());
    let (n_writes, off, _) = d_ref.samples_writes.expect("power-on samples writes");
    assert_eq!((n_writes, off), (73_246, 0));
    assert_eq!(d_ref.dac.expect("power-on MSM5205"), (219_837, 0));
    assert_eq!(
        (d_ref.dac_stream, d_ref.first_dac_stream_diff_frame),
        ((21_104_642, 0), None)
    );
    assert_eq!(
        (d_ref.music_writes, d_ref.music_exact, d_ref.music_lag),
        (42_759, 42_759, 0)
    );
    assert!(d_ref.music_in_order);
    assert_eq!(
        (
            d_ref.ym_compared,
            d_ref.ym_differing,
            d_ref.first_ym_diff_frame
        ),
        (1_536_211, 0, None)
    );
    println!(
        "power-on 0..=1647 (reference times): {n_writes} writes at logged times; 219,837 DAC ticks & 21,104,642 stream samples identical; 42,759 exact YM2151 writes & 1,536,211 output samples identical"
    );

    // 2. Music RAM at idle frame ends over 0..=1647 (0..=793 from boot-01-run-a,
    //    794..=1647 from attract-01-run-a): 1,211 idle frame ends, all equal.
    const BELOW_STACK: usize = 0x5d0;
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2");
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu");
    let empty_log = RefLog {
        latch: Vec::new(),
        writes: Vec::new(),
        opm: Vec::new(),
    };
    let tl_51 = latch_timeline(
        &empty_log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE_PO,
            latency: FITTED_LATENCY,
        }),
    );
    let tl_92 = latch_timeline(
        &empty_log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE_PO,
            latency: IRQ_LATENCY_MID,
        }),
    );
    let window: Vec<u32> = (FIRST..=LAST).collect();
    for (name, tl) in [
        ("reference write times", tl_ref),
        ("latency 51", tl_51.clone()),
        ("latency 92", tl_92.clone()),
    ] {
        let (rams, unrecovered) =
            sf_audio::check::music_ram_at_frame_ends(rom.clone(), tl, &window);
        assert!(unrecovered.is_empty(), "{name}: {unrecovered:04x?}");
        let (mut idle, mut equal, mut first_diff) = (0u32, 0u32, None);
        for (f, is_idle, ram) in &rams {
            if !is_idle {
                continue;
            }
            idle += 1;
            let dir = if *f < 794 { BOOT01_REF } else { ATTRACT01_REF };
            let dump = std::fs::read(Path::new(dir).join(format!("ram_{f:06}_music.bin")))
                .expect("music dump");
            match (0..BELOW_STACK).find(|&i| dump[i] != ram[i]) {
                None => equal += 1,
                Some(i) => {
                    first_diff.get_or_insert((*f, 0xc000 + i, dump[i], ram[i]));
                }
            }
        }
        assert_eq!(
            (idle, equal, first_diff),
            (1_211, 1_211, None),
            "{name}: music RAM at idle frame ends"
        );
    }

    // 3. From the native latch with no pre-roll (from: 1) at latency 51 and 92
    //    (and at the band edges 45 and 138): every device has 0 differing
    //    samples (first differing frame: None) over 0..=1647, with no tolerance
    //    adopted.
    for (lat, expected_exact) in [
        (45, 42_706),
        (FITTED_LATENCY, 42_704),
        (80, 42_712),
        (IRQ_LATENCY_MID, 42_711),
        (138, 42_708),
    ] {
        let tl = latch_timeline(
            &empty_log,
            Some(NativeLatch {
                frames: &frames,
                from: SPLICE_PO,
                latency: lat,
            }),
        );
        let d = run_devices_window(&log, ATTRACT_HOOK_A, ATTRACT_HOOK_A, FIRST, LAST, tl);
        assert!(d.unrecovered.is_empty());
        let (n, off, _) = d.samples_writes.expect("samples writes");
        assert_eq!((n, off), (73_246, 0), "lat {lat} samples writes");
        assert_eq!(
            d.dac.expect("MSM5205 ticks"),
            (219_837, 0),
            "lat {lat} MSM5205 ticks"
        );
        assert_eq!(
            (d.dac_stream, d.first_dac_stream_diff_frame),
            ((21_104_642, 0), None),
            "lat {lat} MSM5205 384 kHz streams"
        );
        assert_eq!(
            (d.music_writes, d.music_exact, d.music_in_order, d.music_lag),
            (42_759, expected_exact, true, 2),
            "lat {lat} music writes"
        );
        assert_eq!(
            (d.ym_compared, d.ym_differing, d.first_ym_diff_frame),
            (1_536_211, 0, None),
            "lat {lat} YM2151 output"
        );
    }

    // 4. Streaming frame-by-frame from power-on with no pre-roll (as the
    //    launcher's `power-on` mode does: `SoundStart` at frame 0 with an empty
    //    `LatchTimeline`, then `push_frame` for 1..=1647 at `IRQ_LATENCY_MID`)
    //    produces the exact same mixed output as the batch runner sample for
    //    sample over 0..=1646 (frame 1647's push makes output ready through
    //    1648/60 - 1 ms, past the end of frame 1646).
    let batch_92 = render_mix(audio2.clone(), rom.clone(), tl_92, FIRST, LAST - 1);
    let known = 1.0 / 60.0;
    let mut live = sf_audio::mix::GameSound::power_on(audio2, rom, LatchTimeline::default(), known);
    assert!(live.skip_to(0.0));
    let rate = WAV_RATE as f64;
    let mut streamed = Vec::with_capacity(batch_92.len());
    for &(frame, value, cycles) in frames.iter().filter(|r| r.0 >= SPLICE_PO) {
        live.push_frame(frame, value, cycles, IRQ_LATENCY_MID);
        live.wait_ready((frame + 1) as f64 / 60.0 - 0.001);
        let t_now = streamed.len() as f64 / rate;
        let ready = live.ready_until(t_now);
        while streamed.len() < batch_92.len() {
            let t = streamed.len() as f64 / rate;
            if t > ready {
                break;
            }
            let [l, r] = live.sample_at(t);
            streamed.push((l + r) / 2.0);
        }
    }
    assert_eq!(streamed.len(), batch_92.len());
    let stream_diff = streamed
        .iter()
        .zip(&batch_92)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        stream_diff, 0,
        "power-on frame-by-frame streaming vs batch mix at latency 92"
    );
    assert!(live.unrecovered().is_empty());
}

const FULL_2P_REF: &str = "../../tools/reference/local/captures/full-2p-01-run-a";
const FULL_2P_SCENARIO: &str = "../../tools/reference/scenarios/sf_full_2p_01.json";
const FULL_2P_HOOK_A: &str = "../../tools/reference/local/captures/full-2p-01-hook-a";
const FULL_2P_HOOK_B: &str = "../../tools/reference/local/captures/full-2p-01-hook-b";

const FULL_1P_REF: &str = "../../tools/reference/local/captures/full-1p-01-run-a";
const FULL_1P_SCENARIO: &str = "../../tools/reference/scenarios/sf_full_1p_01.json";
const FULL_1P_HOOK_A: &str = "../../tools/reference/local/captures/full-1p-01-hook-a";
const FULL_1P_HOOK_B: &str = "../../tools/reference/local/captures/full-1p-01-hook-b";

fn collect_power_on_latch(scenario: &str, last: u32) -> Vec<(u32, u8, u32)> {
    let maincpu = sf_video::romset::load_program_rom(ROMS, MANIFEST).expect("maincpu");
    let mut rows = Vec::with_capacity(last as usize + 1);
    let (stopped, unrecovered) =
        sf_engine::run::observe_power_on_scenario(maincpu, Path::new(scenario), last, |m| {
            rows.push((m.state.frame, m.state.sound_latch, m.state.latch_cycles));
        })
        .expect("power-on scenario run");
    assert_eq!((stopped, unrecovered), (last, vec![]));
    rows
}

fn assert_hook_pair_equal_streaming(a: &str, b: &str, files: &[&str]) {
    use std::io::Read;
    for name in files {
        let pa = Path::new(a).join(name);
        let pb = Path::new(b).join(name);
        let ma = std::fs::metadata(&pa).expect(name);
        let mb = std::fs::metadata(&pb).expect(name);
        assert_eq!(ma.len(), mb.len(), "{a} vs {b}: {name} size differs");
        let mut fa =
            std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(&pa).expect(name));
        let mut fb =
            std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(&pb).expect(name));
        let mut ba = vec![0u8; 1 << 20];
        let mut bb = vec![0u8; 1 << 20];
        loop {
            let na = fa.read(&mut ba).expect(name);
            let nb = fb.read(&mut bb).expect(name);
            assert_eq!(na, nb, "{a} vs {b}: {name} read len differs");
            if na == 0 {
                break;
            }
            assert_eq!(&ba[..na], &bb[..nb], "{a} vs {b}: {name} differs");
        }
    }
    let rows = |dir: &str| {
        let mut v: Vec<String> = std::fs::read_to_string(Path::new(dir).join("hook_index.tsv"))
            .expect("hook_index.tsv")
            .lines()
            .map(str::to_string)
            .collect();
        v.sort();
        v
    };
    assert_eq!(rows(a), rows(b), "{a} vs {b}: hook_index.tsv differs");
}

/// Per-frame diff summary at constant latency over a full-game run.
struct FullGameLatency92Report {
    /// Per-frame count of differing 384 kHz MSM5205 stream samples.
    msm_stream_diff_frames: Vec<u64>,
    /// Per-frame count of differing per-tick MSM5205 DAC levels.
    dac_tick_diff_frames: Vec<u64>,
    /// Per-frame count of differing YM2151 output samples.
    ym_diff_frames: Vec<u64>,
    unrecovered: Vec<u16>,
}

fn measure_full_game_at_latency(
    hook: &sf_audio::check::Hook,
    ym: &sf_audio::check::DeviceStream,
    tl: LatchTimeline,
    last: u32,
) -> FullGameLatency92Report {
    use sf_audio::msm5205::Msm5205;
    const VCK: f64 = 384_000.0;
    const TICK_HZ: f64 = sf_audio::TICK_HZ as f64;
    let n_frames = last as usize + 2;

    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2");

    // 1. Per-tick MSM5205 DAC comparison + 2. 384 kHz stream steps in a single SamplesSound pass:
    let mut dac_tick_diff_frames = vec![0u64; n_frames];
    let mut msm_stream_diff_frames = vec![0u64; n_frames];
    {
        let mut chips = [Msm5205::new(), Msm5205::new()];
        let mut steps: [Vec<(f64, f32)>; 2] = [Vec::new(), Vec::new()];
        let mut power_writes = Vec::new();
        sf_audio::samples::SamplesCpu::power_on(&mut |w| power_writes.push(w));
        for w in power_writes {
            if w.port < 2 {
                chips[w.port as usize].write(w.data);
            }
        }
        let levels = [chips[0].output(), chips[1].output()];
        let mut snd = sf_audio::SamplesSound::new(audio2);
        let mut tl_samples = tl.clone();
        let first_tick = (hook.start * TICK_HZ).ceil() as u64 + 1;
        let last_dac_tick = (hook.end * TICK_HZ).floor() as u64 - 3;
        let last_stream_tick = (hook.end * TICK_HZ).ceil() as u64 + 2;
        while snd.ticks() < last_stream_tick {
            snd.tick(&mut tl_samples, &mut |w| {
                if w.port < 2 {
                    let c = w.port as usize;
                    chips[c].write(w.data);
                    steps[c].push((w.t + 6.0 / VCK, chips[c].output()));
                }
            });
            let k = snd.ticks();
            if k >= first_tick && k <= last_dac_tick {
                let chip = if k % 2 == 1 { 0 } else { 1 };
                let t = (k + 2) as f64 / TICK_HZ + 5.0e-6;
                let r = sf_audio::check::stream_at(&hook.msm[chip], hook.start, VCK, t);
                let n = snd.chip_output(chip);
                if (r - n).abs() > 1e-7 {
                    let f = ((t * 60.0).floor() as usize).min(n_frames - 1);
                    dac_tick_diff_frames[f] += 1;
                }
            }
        }
        let inv_vck = 1.0 / VCK;
        for c in 0..2 {
            let (mut k, mut level) = (0usize, levels[c]);
            let mut next_step_t = steps[c].first().map_or(f64::INFINITY, |x| x.0);
            for (i, &r) in hook.msm[c].iter().enumerate() {
                let s_end = hook.start + (i + 1) as f64 * inv_vck;
                while next_step_t < s_end {
                    level = steps[c][k].1;
                    k += 1;
                    next_step_t = steps[c].get(k).map_or(f64::INFINITY, |x| x.0);
                }
                if (level - r).abs() > 1e-7 {
                    let s = s_end - inv_vck;
                    let f = ((s * 60.0).floor() as usize).min(n_frames - 1);
                    msm_stream_diff_frames[f] += 1;
                }
            }
        }
    }

    // 3. YM2151 output sample comparison across the whole game:
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu");
    let rate = OPM_RATE as f64;
    let s_end = ((last + 1) as f64 / 60.0 * rate).ceil() as u64;
    let run = run_music_from_power_on(rom, tl, s_end);
    let n0 = ym.first_sample();
    let mut ym_diff_frames = vec![0u64; n_frames];
    let scale = 60.0 / rate;
    for (i, &r) in ym.samples.iter().enumerate() {
        let n = n0 + i as u64;
        if n < s_end && run.left[n as usize] != r {
            let f = ((n as f64 * scale).floor() as usize).min(n_frames - 1);
            ym_diff_frames[f] += 1;
        }
    }

    FullGameLatency92Report {
        msm_stream_diff_frames,
        dac_tick_diff_frames,
        ym_diff_frames,
        unrecovered: run.unrecovered,
    }
}

fn summarize_frame_ranges(frames: &[u64], first: u32, last: u32) -> (usize, u64, String) {
    let mut count = 0usize;
    let mut samples = 0u64;
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for f in first..=last {
        let n = frames[f as usize];
        if n == 0 {
            continue;
        }
        count += 1;
        samples += n;
        if let Some(last_r) = ranges.last_mut() {
            if f == last_r.1 + 1 {
                last_r.1 = f;
                continue;
            }
        }
        ranges.push((f, f));
    }
    let formatted: Vec<String> = ranges
        .iter()
        .map(|&(a, b)| {
            if a == b {
                format!("{a}")
            } else {
                format!("{a}..={b}")
            }
        })
        .collect();
    (count, samples, formatted.join(", "))
}

#[test]
fn full_game_2p_and_1p_on_the_native_latch() {
    if ![
        ROMS,
        FULL_2P_REF,
        FULL_2P_HOOK_A,
        FULL_2P_HOOK_B,
        FULL_1P_REF,
        FULL_1P_HOOK_A,
        FULL_1P_HOOK_B,
    ]
    .iter()
    .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local full-game hook captures or ROMs not present");
        return;
    }

    let t0 = std::time::Instant::now();
    // Criterion 1: Two identical audio runs of each.
    assert_hook_pair_equal_streaming(
        FULL_2P_HOOK_A,
        FULL_2P_HOOK_B,
        &[
            "audio_taps.tsv",
            "hook_msm1.f32",
            "hook_msm2.f32",
            "hook_ymsnd.f32",
        ],
    );
    eprintln!(
        "[{:.1}s] verified FULL_2P_HOOK_A == FULL_2P_HOOK_B",
        t0.elapsed().as_secs_f64()
    );
    assert_hook_pair_equal_streaming(
        FULL_1P_HOOK_A,
        FULL_1P_HOOK_B,
        &[
            "audio_taps.tsv",
            "hook_msm1.f32",
            "hook_msm2.f32",
            "hook_ymsnd.f32",
        ],
    );
    eprintln!(
        "[{:.1}s] verified FULL_1P_HOOK_A == FULL_1P_HOOK_B",
        t0.elapsed().as_secs_f64()
    );
    assert_eq!(
        screen_hashes(Path::new(FULL_2P_REF)),
        screen_hashes(Path::new(FULL_2P_HOOK_A))
    );
    assert_eq!(
        screen_hashes(Path::new(FULL_1P_REF)),
        screen_hashes(Path::new(FULL_1P_HOOK_A))
    );

    let scenes_2p: &[(&str, u32, u32)] = &[
        (
            "Power-on, warning, and attract (wall, title, ranking, demo)",
            0,
            2012,
        ),
        ("Coins, start screen, and 2P country select", 2013, 2449),
        ("Versus match #1 (P2 Ken wins 2-0)", 2450, 6990),
        (
            "P2 mode-4 intro and P1 challenger join (`HERE COMES A NEW CHALLENGER`)",
            6991,
            7958,
        ),
        ("Versus match #2 (P1 Ryu wins 2-0)", 7959, 12259),
        ("Mode-2 Retsu match #1 (P1 loses 0-2)", 12260, 16533),
        (
            "Continue #1 (`start1`) and Retsu match #2 (P1 loses 0-2)",
            16534,
            20364,
        ),
        (
            "Continue countdown expiry, game over, and return to attract",
            20365,
            21299,
        ),
    ];

    let scenes_1p: &[(&str, u32, u32)] = &[
        ("Power-on, warning, and attract", 0, 1513),
        (
            "Coin, start screen, and 1P country select (Japan)",
            1514,
            2388,
        ),
        ("Stage 0 fight 1: Retsu", 2389, 5013),
        ("Stage 0 fight 2: Geki", 5014, 9018),
        ("Japan power/timing bonus and flight to USA", 9019, 10065),
        ("Stage 1 fight 1: Joe", 10066, 14033),
        ("Stage 1 fight 2: Mike", 14034, 17483),
        ("Stage-1 10-second bonus and flight to China", 17484, 19375),
        ("Stage 2 fight 1: Lee", 19376, 23161),
        ("Stage 2 fight 2: Gen", 23162, 26070),
        (
            "Stage-2 power/timing bonus and flight to England",
            26071,
            26955,
        ),
        ("Stage 3 fight 1: Birdie", 26956, 29775),
        ("Stage 3 fight 2: Eagle", 29776, 32314),
        (
            "Stage-3 10-second bonus and flight to Thailand",
            32315,
            34183,
        ),
        ("Stage 4 fight 1: Adon", 34184, 37152),
        ("Stage 4 fight 2: Sagat", 37153, 39776),
        (
            "Ending (5 country scenes, message, staff roll, `THE END`)",
            39777,
            43616,
        ),
        (
            "Name entry (`AAA`), ranking blink, game over, and return to attract",
            43617,
            44199,
        ),
    ];

    for (label, scenario, hook_dir, last, scenes) in [
        (
            "sf-full-2p-01",
            FULL_2P_SCENARIO,
            FULL_2P_HOOK_A,
            21299u32,
            scenes_2p,
        ),
        (
            "sf-full-1p-01",
            FULL_1P_SCENARIO,
            FULL_1P_HOOK_A,
            44199u32,
            scenes_1p,
        ),
    ] {
        let frames = collect_power_on_latch(scenario, last);
        eprintln!(
            "[{:.1}s] {label}: collected {} native latch rows",
            t0.elapsed().as_secs_f64(),
            frames.len()
        );
        let log = read_taps(&Path::new(hook_dir).join("audio_taps.tsv")).expect("tap log");
        let hook = read_hook(Path::new(hook_dir)).expect("MSM5205 hook");
        let rate = OPM_RATE as f64;
        let ym = read_device_stream(Path::new(hook_dir), "ymsnd", rate).expect("ymsnd hook");
        eprintln!(
            "[{:.1}s] {label}: loaded taps & hooks",
            t0.elapsed().as_secs_f64()
        );

        // Verify latch residuals over the whole game: frame 1 is exact (186,957 cycles),
        // and every subsequent latch write lies inside IRQ_LATENCY_CYCLES (45..=138).
        let residuals = sf_audio::check::latch_residuals(&log, &frames);
        let r1 = &residuals[0];
        assert_eq!(
            (r1.frame, r1.cycles, r1.native, r1.reference),
            (1, 186_957, 0x27, 0x27)
        );
        assert!(r1.latency.abs() < 1e-6);
        let mut min_lat = f64::MAX;
        let mut max_lat = f64::MIN;
        for r in &residuals[1..] {
            assert_eq!(
                r.native, r.reference,
                "{label} frame {} latch value",
                r.frame
            );
            assert!(
                r.latency >= *IRQ_LATENCY_CYCLES.start() as f64 - 0.01
                    && r.latency <= *IRQ_LATENCY_CYCLES.end() as f64 + 0.5,
                "{label} frame {} implied latency {:.2} outside 45..=138 (cycles {})",
                r.frame,
                r.latency,
                r.cycles
            );
            min_lat = min_lat.min(r.latency);
            max_lat = max_lat.max(r.latency);
        }
        println!(
            "{label} latch residuals: {} writes, frame 1 exact (186,957 cycles), frames 2..={last} in {min_lat:.2}..={max_lat:.2} cycles",
            residuals.len()
        );

        // Criterion 2: At the reference's write times every device is identical over the whole game.
        let tl_ref = native_at_reference_times(&log, &frames, 1);
        let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2");
        let (n_writes, off, _, _) = write_time_drift(
            audio2.clone(),
            tl_ref.clone(),
            &log.writes,
            frame_tick(0),
            frame_tick(last + 1),
        )
        .expect("samples writes at ref times");
        assert_eq!(off, 0, "{label} samples writes off time at ref times");
        let (dac_ticks, dac_active) =
            compare_dac(audio2.clone(), tl_ref.clone(), &hook).expect("MSM5205 ticks at ref times");
        let (stream_compared, stream_differing, first_stream_diff) =
            compare_dac_stream(audio2, tl_ref.clone(), &hook);
        assert_eq!(
            stream_differing, 0,
            "{label} MSM5205 384 kHz stream diff at ref times (first: {first_stream_diff:?})"
        );

        let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu");
        let s_end = ((last + 1) as f64 / 60.0 * rate).ceil() as u64;
        let run_ref = run_music_from_power_on(rom, tl_ref, s_end);
        assert!(
            run_ref.unrecovered.is_empty(),
            "{label} ref times unrecovered: {:04x?}",
            run_ref.unrecovered
        );
        let ref_opm: Vec<_> = reference_opm_writes(&log)
            .into_iter()
            .filter(|w| w.sample < s_end)
            .collect();
        let nat_opm: Vec<_> = run_ref
            .writes
            .iter()
            .copied()
            .filter(|w| w.sample < s_end)
            .collect();
        let music_in_order = nat_opm.len() == ref_opm.len()
            && nat_opm
                .iter()
                .zip(&ref_opm)
                .all(|(n, r)| (n.reg, n.data) == (r.reg, r.data));
        let music_lag = nat_opm
            .iter()
            .zip(&ref_opm)
            .map(|(n, r)| n.sample.abs_diff(r.sample))
            .max()
            .unwrap_or(0);
        let music_exact = exact_clock_matches(&nat_opm, &ref_opm);
        assert!(music_in_order, "{label} music writes in order at ref times");
        assert_eq!(
            (music_exact, music_lag),
            (ref_opm.len(), 0),
            "{label} music exact at ref times"
        );
        let n0 = ym.first_sample();
        let mut ym_compared = 0u64;
        let mut ym_differing = 0u64;
        for (i, r) in ym.samples.iter().enumerate() {
            let n = n0 + i as u64;
            if n < s_end {
                ym_compared += 1;
                if run_ref.left[n as usize] != *r {
                    ym_differing += 1;
                }
            }
        }
        assert_eq!(ym_differing, 0, "{label} YM2151 diff at ref times");
        drop(run_ref);
        let expected_ref = if last == 21299 {
            (
                1_762_231,
                2_839_837,
                453_190,
                272_624_642,
                639_631,
                19_854_032,
            )
        } else {
            (
                4_821_331,
                5_893_277,
                1_530_499,
                565_754_882,
                1_412_668,
                41_201_394,
            )
        };
        assert_eq!(
            (
                n_writes,
                dac_ticks,
                dac_active,
                stream_compared,
                ref_opm.len(),
                ym_compared
            ),
            expected_ref,
            "{label} reference-time counts"
        );
        eprintln!(
            "[{:.1}s] {label}: verified reference write times",
            t0.elapsed().as_secs_f64()
        );
        println!(
            "{label} (reference write times, 0..={last}): samples {n_writes} writes (0 off), MSM5205 {dac_ticks} ticks ({dac_active} active) & {stream_compared} stream samples (0 diff), music {} exact writes, YM2151 {ym_compared} samples (0 diff)",
            ref_opm.len()
        );

        // Criterion 3 & 4: At the launcher's latency (IRQ_LATENCY_MID = 92, from: 1, no pre-roll).
        let empty_log = RefLog {
            latch: Vec::new(),
            writes: Vec::new(),
            opm: Vec::new(),
        };
        let tl_92 = latch_timeline(
            &empty_log,
            Some(NativeLatch {
                frames: &frames,
                from: 1,
                latency: IRQ_LATENCY_MID,
            }),
        );
        let rep = measure_full_game_at_latency(&hook, &ym, tl_92, last);
        eprintln!(
            "[{:.1}s] {label}: measured latency 92",
            t0.elapsed().as_secs_f64()
        );
        assert!(
            rep.unrecovered.is_empty(),
            "{label} lat 92 unrecovered: {:04x?}",
            rep.unrecovered
        );
        println!("=== {label} at latency 92 (0..={last}) ===");
        for &(scene, s_first, s_last) in scenes {
            let scene_res: Vec<_> = residuals
                .iter()
                .filter(|r| {
                    r.frame >= s_first
                        && r.frame <= s_last
                        && r.frame > 1
                        && r.reference != 0xff
                        && r.reference != 0x00
                })
                .collect();
            let cmd_summary = if scene_res.is_empty() {
                "none".to_string()
            } else {
                let s_min = scene_res.iter().map(|r| r.latency).fold(f64::MAX, f64::min);
                let s_max = scene_res.iter().map(|r| r.latency).fold(f64::MIN, f64::max);
                format!("{} cmds (lat {s_min:.1}..={s_max:.1})", scene_res.len())
            };
            let (dac_f, dac_t, dac_r) =
                summarize_frame_ranges(&rep.dac_tick_diff_frames, s_first, s_last);
            let (msm_f, msm_s, msm_r) =
                summarize_frame_ranges(&rep.msm_stream_diff_frames, s_first, s_last);
            let (ym_f, ym_s, ym_r) = summarize_frame_ranges(&rep.ym_diff_frames, s_first, s_last);
            println!(
                "  [{s_first}..={s_last}] {scene} [{cmd_summary}]:\n    DAC ticks: {dac_f} frames ({dac_t} ticks) [{dac_r}]\n    MSM 384k: {msm_f} frames ({msm_s} samples) [{msm_r}]\n    YM2151:   {ym_f} frames ({ym_s} samples) [{ym_r}]"
            );
        }
    }
}
