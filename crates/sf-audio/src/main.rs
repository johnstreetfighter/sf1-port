//! sf-audio: run the translated samples CPU against a sound-latch timeline
//! and check / render it (tickets #10/#11).
//!
//! The latch timeline comes from a reference audio-probe tap log
//! (`audio_taps.tsv`, local-only) and, from `--native-from` on, from a
//! native capture's per-frame latch (`frames.tsv`), so a native run's sound
//! starts from the reference's sound state at the fixture frame:
//!
//!   sf-audio compare-writes --taps <ref>/audio_taps.tsv \
//!       [--native-capture <native dir> --native-from 2450] [--from-frame F --to-frame G]
//!   sf-audio compare-dac --taps <ref>/audio_taps.tsv --hook-dir <ref hook dir> \
//!       [--native-capture <dir> --native-from 2450]
//!   sf-audio render --taps <ref>/audio_taps.tsv [--native-capture ...] \
//!       --from-frame F --to-frame G --out hit.wav
//!
//! A readable score on the native YM2151 driver:
//!
//!   sf-audio render-score --score local.score --seconds 20 --out local.wav
//!
//! The voice pipeline's codec steps (`tools/voice/pipeline.py`, EH9):
//!
//!   sf-audio voice-encode --wav take.4k.wav --vox take.vox --heard take.heard.wav
//!   sf-audio voice-rom-sample --command 0x94 --out fight.heard.wav
//!   sf-audio voice-pack --plan plan.json --out assets/sf1-port-voice

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use sf_audio::check::{
    best_correlation, compare_dac, compare_writes, frame_tick, latch_timeline, measure,
    native_latch, read_device_stream, read_hook, read_taps, read_wav_mono, reference_opm_writes,
    render, replay_opm, run_music, run_music_from_power_on, NativeLatch, OpmWrite, RefLog,
};
use sf_audio::{voice_pipeline, write_wav, LatchTimeline, FRAME_HZ, TICK_HZ};

#[derive(Parser)]
#[command(
    name = "sf-audio",
    about = "Translated SF1 samples CPU: check and render"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Clone)]
struct Source {
    /// Reference audio-probe tap log (latch and samples-CPU writes).
    #[arg(long)]
    taps: PathBuf,
    /// Native capture whose per-frame sound latch replaces the reference's
    /// from `--native-from` on.
    #[arg(long)]
    native_capture: Option<PathBuf>,
    /// First frame taken from the native capture.
    #[arg(long, default_value_t = 2450)]
    native_from: u32,
    /// Directory holding the extracted ROM files.
    #[arg(long, default_value = "tools/reference/local/roms/sf")]
    roms: PathBuf,
    /// ROM manifest JSON.
    #[arg(long, default_value = "tools/reference/rom-manifest.json")]
    manifest: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Compare every samples-CPU port write with the reference's.
    CompareWrites {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        from_frame: Option<u32>,
        #[arg(long)]
        to_frame: Option<u32>,
    },
    /// Compare the MSM5205 DAC outputs at every tick with the reference's
    /// hooked device streams (hook_msm1.f32 / hook_msm2.f32).
    CompareDac {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        hook_dir: PathBuf,
    },
    /// Measure and compare two recordings (mono mix of each WAV): onset,
    /// duration, RMS, zero-crossing rate, spectral centroid, and the best
    /// normalized cross-correlation within +/-20 ms.
    Measure {
        #[arg(long)]
        a: PathBuf,
        #[arg(long)]
        b: PathBuf,
        /// Start of the excerpt in A / B (seconds).
        #[arg(long, default_value_t = 0.0)]
        a_start: f64,
        #[arg(long, default_value_t = 0.0)]
        b_start: f64,
        /// Excerpt length (seconds; default: all of A).
        #[arg(long)]
        seconds: Option<f64>,
    },
    /// Write the reference's hooked MSM5205 streams (sum of both chips) as a
    /// 48 kHz WAV (384 kHz averaged in blocks of 8), starting at `--from`
    /// seconds of machine time.
    HookWav {
        #[arg(long)]
        hook_dir: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Replay the reference's YM2151 writes into the chip model and compare
    /// its output with the reference's hooked YM2151 stream.
    CompareOpm {
        #[arg(long)]
        taps: PathBuf,
        #[arg(long)]
        hook_dir: PathBuf,
    },
    /// Run the translated music CPU from a reference checkpoint's music RAM
    /// (frame F, the state at the start of frame F+1) and compare its YM2151
    /// writes with the reference's through frame G.
    CompareMusic {
        /// Reference audio-probe run (YM2151 writes, latch).
        #[arg(long)]
        taps: PathBuf,
        /// Reference capture holding ram_<F>_music.bin.
        #[arg(long)]
        capture: PathBuf,
        #[arg(long, default_value_t = 2279)]
        fixture_frame: u32,
        #[arg(long, default_value_t = 2359)]
        to_frame: u32,
        /// The idle loop's phase at the fixture (T-states past it).
        #[arg(long, default_value_t = sf_audio::check::MUSIC_FIXTURE_IDLE_PHASE)]
        idle_phase: u64,
        /// Run from power-on (the Z80 from its reset vector at clock 0, no
        /// RAM fixture, no idle phase) and compare from frame 0; `capture`,
        /// `fixture_frame` and `idle_phase` are then unused.
        #[arg(long)]
        power_on: bool,
        /// Optional WAV of the native YM2151 output over the compared frames.
        #[arg(long)]
        wav: Option<PathBuf>,
        /// Reference hook directory (hook_ymsnd.f32) to compare the output.
        #[arg(long)]
        hook_dir: Option<PathBuf>,
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
    },
    /// Render the two chips' mix for a frame range to a 48 kHz WAV.
    Render {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        from_frame: u32,
        #[arg(long)]
        to_frame: u32,
        #[arg(long)]
        out: PathBuf,
    },
    /// Audition a readable score through the live native YM2151 driver.
    RenderScore {
        #[arg(long)]
        score: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        seconds: f64,
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
    },
    /// Encode a take (mono 16-bit PCM WAV at 4 kHz) as the voice pack's
    /// `.vox` (4-bit MSM5205 ADPCM, high nibble first), and write what the
    /// chip plays from it as a 48 kHz WAV for listening. Prints
    /// `{"nibbles", "crc32", "seconds"}` as JSON.
    VoiceEncode {
        #[arg(long)]
        wav: PathBuf,
        #[arg(long)]
        vox: PathBuf,
        #[arg(long)]
        heard: PathBuf,
    },
    /// Write one of the ROM's own samples (samples-CPU command, e.g. 0x94
    /// "FIGHT!") as its chip plays it, as a 48 kHz WAV for listening.
    /// Prints `{"command", "nibbles", "seconds"}` as JSON.
    VoiceRomSample {
        /// The command, decimal or 0x-prefixed hex.
        #[arg(long)]
        command: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
    },
    /// Write the voice pack from a plan of chosen takes
    /// (`{"lines": {"super": {"vox": "...", "nibbles": N}}}`) with
    /// `VoiceStore::write`, then open it as the game does. Prints the lines
    /// written as JSON.
    VoicePack {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

/// The decoded chip output (4 kHz) as a 48 kHz WAV, each value held for
/// 12 samples as the chip's DAC holds it.
fn write_heard(path: &std::path::Path, out: &[f32]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    write_wav(path, out, 48_000, 12).map_err(|e| format!("{}: {e}", path.display()))
}

fn sound_rom(source: &Source) -> Result<Vec<u8>, String> {
    sf_video::romset::load_region(&source.roms, &source.manifest, "audio2")
        .map_err(|e| e.to_string())
}

fn source_timeline(source: &Source, log: &RefLog) -> Result<LatchTimeline, String> {
    match &source.native_capture {
        None => Ok(latch_timeline(log, None)),
        Some(dir) => {
            let frames = native_latch(dir).map_err(|e| e.to_string())?;
            Ok(latch_timeline(
                log,
                Some(NativeLatch {
                    frames: &frames,
                    from: source.native_from,
                    latency: sf_audio::IRQ_LATENCY_MID,
                }),
            ))
        }
    }
}

fn render_score(
    program: Vec<u8>,
    samples: Vec<u8>,
    compiled: sf_audio::score::CompiledScore,
    seconds: f64,
    out: &std::path::Path,
) -> Result<(), String> {
    use sf_audio::mix::GameSound;

    const RATE: u32 = 48_000;
    const START: u32 = 6;
    const CHUNK: u32 = 120;
    let start_time = f64::from(START) / f64::from(FRAME_HZ);
    let count = (seconds * f64::from(RATE)).round() as usize;
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav =
        hound::WavWriter::create(out, spec).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut sound = GameSound::power_on(samples, program, LatchTimeline::default(), 0.0);
    let mut written = 0;
    let mut frame = 0;
    while written < count {
        let until = frame + CHUNK;
        while frame < until {
            let (command, score) = if frame == START {
                (compiled.command, Some(compiled.score.clone()))
            } else {
                (0xff, None)
            };
            sound.push_frame_with_score(frame, command, 0, sf_audio::IRQ_LATENCY_MID, score);
            frame += 1;
        }
        let ready = f64::from(frame - 2) / f64::from(FRAME_HZ);
        let end = (((ready - start_time) * f64::from(RATE)).floor() as usize).min(count);
        if !sound.wait_ready(ready) {
            return Err("the native music driver did not reach the audition horizon".into());
        }
        for n in written..end {
            let t = start_time + n as f64 / f64::from(RATE);
            for value in sound.sample_at_gains(t, 1.0, 0.0) {
                wav.write_sample((value * 32767.0) as i16)
                    .map_err(|e| e.to_string())?;
            }
        }
        written = end;
        let faults = sound.unrecovered();
        if !faults.is_empty() {
            return Err(format!(
                "the native music driver reported faults: {faults:04x?}"
            ));
        }
    }
    wav.finalize().map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, String> {
    match cli.command {
        Commands::CompareWrites {
            source,
            from_frame,
            to_frame,
        } => {
            let log = read_taps(&source.taps).map_err(|e| e.to_string())?;
            let tl = source_timeline(&source, &log)?;
            let first = from_frame.map(frame_tick).unwrap_or(0);
            let last = to_frame
                .map(|f| frame_tick(f + 1))
                .unwrap_or_else(|| log.writes.last().map(|w| w.tick).unwrap_or(0));
            match compare_writes(sound_rom(&source)?, tl, &log.writes, first, last) {
                Ok(n) => {
                    println!("MATCH: {n} samples-CPU port writes, ticks {first}..{last}");
                    Ok(ExitCode::from(0))
                }
                Err(m) => {
                    println!("MISMATCH: {m}");
                    Ok(ExitCode::from(1))
                }
            }
        }
        Commands::CompareDac { source, hook_dir } => {
            let log = read_taps(&source.taps).map_err(|e| e.to_string())?;
            let hook = read_hook(&hook_dir).map_err(|e| e.to_string())?;
            let tl = source_timeline(&source, &log)?;
            match compare_dac(sound_rom(&source)?, tl, &hook) {
                Ok((n, active)) => {
                    println!(
                        "MATCH: both MSM5205 DAC outputs equal at {n} channel ticks ({:.3} s; {active} of them non-silent)",
                        n as f64 / TICK_HZ as f64
                    );
                    Ok(ExitCode::from(0))
                }
                Err(m) => {
                    println!("MISMATCH at {m}");
                    Ok(ExitCode::from(1))
                }
            }
        }
        Commands::Measure {
            a,
            b,
            a_start,
            b_start,
            seconds,
        } => {
            let (wa, ra) = read_wav_mono(&a)?;
            let (wb, rb) = read_wav_mono(&b)?;
            if ra != rb {
                return Err(format!("sample rates differ: {ra} vs {rb}"));
            }
            let rate = ra as f64;
            let len = seconds
                .map(|s| (s * rate) as usize)
                .unwrap_or(wa.len().saturating_sub((a_start * rate) as usize));
            let cut = |w: &[f32], start: f64| -> Vec<f32> {
                let s = (start * rate) as usize;
                w.get(s..(s + len).min(w.len())).unwrap_or(&[]).to_vec()
            };
            let (xa, xb) = (cut(&wa, a_start), cut(&wb, b_start));
            let (ma, mb) = (measure(&xa, rate), measure(&xb, rate));
            println!("excerpts: {} samples at {ra} Hz", xa.len().min(xb.len()));
            for (name, m) in [("A", &ma), ("B", &mb)] {
                println!(
                    "  {name}: onset {:.2} ms, active {:.1} ms, rms {:.4}, peak {:.4}, zero-crossings {:.0}/s, centroid {:.0} Hz",
                    m.onset * 1e3, m.active * 1e3, m.rms, m.peak, m.zcr, m.centroid
                );
            }
            let (c, lag) = best_correlation(&xa, &xb, (0.020 * rate) as isize);
            println!(
                "  best normalized correlation {c:.3} at lag {:.2} ms (B relative to A)",
                lag as f64 / rate * 1e3
            );
            Ok(ExitCode::from(0))
        }
        Commands::HookWav { hook_dir, out } => {
            let hook = read_hook(&hook_dir).map_err(|e| e.to_string())?;
            let n = hook.msm[0].len().min(hook.msm[1].len());
            let pcm: Vec<f32> = (0..n / 8)
                .map(|i| {
                    (0..8)
                        .map(|j| hook.msm[0][i * 8 + j] + hook.msm[1][i * 8 + j])
                        .sum::<f32>()
                        / 8.0
                })
                .collect();
            write_wav(&out, &pcm, 48_000, 1).map_err(|e| e.to_string())?;
            println!(
                "reference MSM5205 mix from t = {:.6} s ({} samples) to {}",
                hook.start,
                pcm.len(),
                out.display()
            );
            Ok(ExitCode::from(0))
        }
        Commands::CompareOpm { taps, hook_dir } => {
            let log = read_taps(&taps).map_err(|e| e.to_string())?;
            let rate = sf_audio::opm::OPM_RATE as f64;
            let hook = read_device_stream(&hook_dir, "ymsnd", rate).map_err(|e| e.to_string())?;
            let n0 = (hook.start * rate).round() as i64;
            let count = (n0 as usize) + hook.samples.len() + 8;
            for delta in [0.0, 1.0, -1.0, 0.5, 2.0] {
                let native = replay_opm(&log.opm, count, delta);
                for shift in -3i64..=3 {
                    let mut equal = 0usize;
                    let mut max_diff = 0f32;
                    for (i, r) in hook.samples.iter().enumerate() {
                        let n = n0 + i as i64 + shift;
                        let v = native.get(n.max(0) as usize).copied().unwrap_or(0.0);
                        let d = (v - r).abs();
                        if d < 1e-6 {
                            equal += 1;
                        }
                        max_diff = max_diff.max(d);
                    }
                    println!(
                        "delta {delta:+} shift {shift:+}: {equal}/{} samples equal, max |diff| {max_diff:.6}",
                        hook.samples.len()
                    );
                }
            }
            Ok(ExitCode::from(0))
        }
        Commands::CompareMusic {
            taps,
            capture,
            fixture_frame,
            to_frame,
            idle_phase,
            power_on,
            wav,
            hook_dir,
            roms,
            manifest,
        } => {
            let log = read_taps(&taps).map_err(|e| e.to_string())?;
            let rom = sf_video::romset::load_region(&roms, &manifest, "audiocpu")
                .map_err(|e| e.to_string())?;
            let rate = sf_audio::opm::OPM_RATE as f64;
            let t_end = (to_frame + 1) as f64 / FRAME_HZ as f64;
            let s_end = (t_end * rate).ceil() as u64;
            let (s_fix, run) = if power_on {
                (
                    0,
                    run_music_from_power_on(rom, latch_timeline(&log, None), s_end),
                )
            } else {
                let ram = std::fs::read(capture.join(format!("ram_{fixture_frame:06}_music.bin")))
                    .map_err(|e| e.to_string())?;
                let t_fix = (fixture_frame + 1) as f64 / FRAME_HZ as f64;
                (
                    (t_fix * rate).ceil() as u64,
                    run_music(
                        rom,
                        ram,
                        &log,
                        t_fix,
                        latch_timeline(&log, None),
                        idle_phase,
                        s_end,
                    ),
                )
            };
            let (native, out, unrecovered) = (run.writes, run.left, run.unrecovered);
            if !unrecovered.is_empty() {
                println!("untranslated branches reached: {unrecovered:04x?}");
            }
            let native: Vec<OpmWrite> = native.into_iter().filter(|w| w.sample < s_end).collect();
            // Timer re-arm writes (0x10-0x14) are counted separately: the
            // reference's interrupt handler takes real time (up to several ms
            // while loading patches), which reorders re-arms against the next
            // handler; the native handler runs at its interrupt's time.
            let timer = |w: &OpmWrite| (0x10..=0x14).contains(&w.reg);
            let ticks = |v: &[OpmWrite]| v.iter().filter(|w| w.reg == 0x10).count();
            let reference_all: Vec<OpmWrite> = reference_opm_writes(&log)
                .into_iter()
                .filter(|w| w.sample >= s_fix && w.sample < s_end)
                .collect();
            println!(
                "timer A ticks: reference {}, native {}; timer B polls: reference {}, native {}",
                ticks(&reference_all),
                ticks(&native),
                reference_all.iter().filter(|w| w.reg == 0x12).count(),
                native.iter().filter(|w| w.reg == 0x12).count()
            );
            let full_in_order = reference_all.len() == native.len()
                && reference_all
                    .iter()
                    .zip(&native)
                    .all(|(r, w)| (r.reg, r.data) == (w.reg, w.data));
            let full_lag = reference_all
                .iter()
                .zip(&native)
                .map(|(r, w)| (r.sample as i64 - w.sample as i64).abs())
                .max()
                .unwrap_or(0);
            println!(
                "all {} writes (timer re-arms included) in order: {}; largest timing difference {full_lag} samples",
                reference_all.len(),
                if full_in_order { "yes" } else { "no" }
            );
            let reference: Vec<OpmWrite> =
                reference_all.into_iter().filter(|w| !timer(w)).collect();
            let native: Vec<OpmWrite> = native.into_iter().filter(|w| !timer(w)).collect();
            let n = native.len().min(reference.len());
            let mut max_lag = 0i64;
            for i in 0..n {
                let (r, w) = (reference[i], native[i]);
                if (r.reg, r.data) != (w.reg, w.data) {
                    println!("MISMATCH at write #{i}:");
                    for j in i.saturating_sub(3)..(i + 4).min(n) {
                        println!(
                            "  reference s{} {:02x}={:02x}   native s{} {:02x}={:02x}",
                            reference[j].sample,
                            reference[j].reg,
                            reference[j].data,
                            native[j].sample,
                            native[j].reg,
                            native[j].data
                        );
                    }
                    return Ok(ExitCode::from(1));
                }
                max_lag = max_lag.max((r.sample as i64 - w.sample as i64).abs());
            }
            if native.len() != reference.len() {
                println!(
                    "MISMATCH: {} reference writes, {} native",
                    reference.len(),
                    native.len()
                );
                return Ok(ExitCode::from(1));
            }
            println!(
                "MATCH: {n} YM2151 sound writes in order, frames {}..={to_frame}; largest timing difference {max_lag} samples ({:.3} ms)",
                fixture_frame + 1,
                max_lag as f64 / rate * 1e3
            );
            if let Some(dir) = &hook_dir {
                let hook = read_device_stream(dir, "ymsnd", rate).map_err(|e| e.to_string())?;
                let n0 = hook.first_sample() as i64;
                let (mut equal, mut compared, mut max_diff) = (0usize, 0usize, 0f32);
                let (mut ab, mut aa, mut bb) = (0f64, 0f64, 0f64);
                for (i, r) in hook.samples.iter().enumerate() {
                    let n = n0 + i as i64;
                    if n < s_fix as i64 || n >= s_end as i64 {
                        continue;
                    }
                    let v = out[n as usize];
                    compared += 1;
                    if (v - r).abs() < 1e-6 {
                        equal += 1;
                    }
                    max_diff = max_diff.max((v - r).abs());
                    ab += (v * r) as f64;
                    aa += (v * v) as f64;
                    bb += (r * r) as f64;
                }
                println!(
                    "output vs the reference's YM2151 stream: {equal}/{compared} samples identical, max |diff| {max_diff:.4}, correlation {:.5}",
                    ab / (aa * bb).sqrt().max(1e-12)
                );
            }
            if let Some(path) = wav {
                // The stereo mix (left + right) / 2 at 48 kHz (linear
                // interpolation from the chip's 55930 Hz), from the fixture
                // frame's end, for listening and `measure`.
                let pcm = &run.mix[s_fix as usize..s_end as usize];
                let n48 = (pcm.len() as f64 * 48_000.0 / rate) as usize;
                let resampled: Vec<f32> = (0..n48)
                    .map(|i| {
                        let x = i as f64 * rate / 48_000.0;
                        let (k, f) = (x.floor() as usize, (x - x.floor()) as f32);
                        let a = pcm[k.min(pcm.len() - 1)];
                        let b = pcm[(k + 1).min(pcm.len() - 1)];
                        a + (b - a) * f
                    })
                    .collect();
                write_wav(&path, &resampled, 48_000, 1).map_err(|e| e.to_string())?;
                println!(
                    "native YM2151 output (48 kHz) written to {}",
                    path.display()
                );
            }
            Ok(if unrecovered.is_empty() {
                ExitCode::from(0)
            } else {
                ExitCode::from(1)
            })
        }
        Commands::Render {
            source,
            from_frame,
            to_frame,
            out,
        } => {
            let log = read_taps(&source.taps).map_err(|e| e.to_string())?;
            let tl = source_timeline(&source, &log)?;
            let pcm = render(
                sound_rom(&source)?,
                tl,
                frame_tick(from_frame),
                frame_tick(to_frame + 1),
            );
            write_wav(&out, &pcm, 48_000, 6).map_err(|e| e.to_string())?;
            println!(
                "rendered frames {from_frame}..={to_frame} ({} ticks) to {}",
                pcm.len(),
                out.display()
            );
            Ok(ExitCode::from(0))
        }
        Commands::RenderScore {
            score,
            out,
            seconds,
            roms,
            manifest,
        } => {
            if !seconds.is_finite() || seconds <= 0.0 || seconds > 3600.0 {
                return Err("--seconds must be greater than zero and no more than 3600".into());
            }
            let text =
                std::fs::read_to_string(&score).map_err(|e| format!("{}: {e}", score.display()))?;
            let region = |name| {
                sf_video::romset::load_region(&roms, &manifest, name).map_err(|e| e.to_string())
            };
            let program = region("audiocpu")?;
            let compiled = sf_audio::score::compile(&text, &program)
                .map_err(|e| format!("{}: {e}", score.display()))?;
            let bytes = compiled.score.bytes();
            let samples = region("audio2")?;
            render_score(program, samples, compiled, seconds, &out)?;
            println!(
                "{} native score bytes; wrote {:.3} s to {}",
                bytes,
                seconds,
                out.display()
            );
            Ok(ExitCode::SUCCESS)
        }
        Commands::VoiceEncode { wav, vox, heard } => {
            let pcm = voice_pipeline::read_take_wav(&wav)?;
            let clip = voice_pipeline::encode_take(&pcm).map_err(|e| format!("{}: {e}", wav.display()))?;
            let bytes = voice_pipeline::pack_nibbles(&clip.nibbles);
            if let Some(dir) = vox.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            std::fs::write(&vox, &bytes).map_err(|e| format!("{}: {e}", vox.display()))?;
            write_heard(&heard, &voice_pipeline::decode(&clip.nibbles))?;
            println!(
                "{}",
                serde_json::json!({
                    "nibbles": clip.nibbles.len(),
                    "crc32": format!("{:08x}", sf_video::romset::crc32(&bytes)),
                    "seconds": clip.nibbles.len() as f64 / clip.sample_rate as f64,
                })
            );
            Ok(ExitCode::from(0))
        }
        Commands::VoiceRomSample { command, out, roms, manifest } => {
            let cmd = match command.strip_prefix("0x") {
                Some(hex) => u8::from_str_radix(hex, 16),
                None => command.parse(),
            }
            .map_err(|e| format!("--command {command}: {e}"))?;
            let audio2 = sf_video::romset::load_region(&roms, &manifest, "audio2").map_err(|e| e.to_string())?;
            let nibbles = voice_pipeline::rom_sample(audio2, cmd)?;
            write_heard(&out, &voice_pipeline::decode(&nibbles))?;
            println!(
                "{}",
                serde_json::json!({
                    "command": format!("0x{cmd:02x}"),
                    "nibbles": nibbles.len(),
                    "seconds": nibbles.len() as f64 / 4000.0,
                })
            );
            Ok(ExitCode::from(0))
        }
        Commands::VoicePack { plan, out } => {
            let text = std::fs::read_to_string(&plan).map_err(|e| format!("{}: {e}", plan.display()))?;
            let parsed: voice_pipeline::PackPlan =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", plan.display()))?;
            let store = voice_pipeline::build_pack(&parsed, &out)?;
            println!(
                "{}",
                serde_json::json!({ "folder": out.display().to_string(), "lines": store.len() })
            );
            Ok(ExitCode::from(0))
        }
    }
}
