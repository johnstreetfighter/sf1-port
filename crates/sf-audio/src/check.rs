//! Reference checks and rendering for the translated sound programs: read a
//! reference audio-probe run (tap log, hooked device streams), build the
//! sound-latch timeline (reference before a native start frame, a native
//! capture's latch after), and compare or render the native sound.

use std::path::Path;

use crate::samples::PortWrite;
use crate::{LatchTimeline, SamplesSound, FRAME_HZ, M68K_HZ, TICK_HZ};

/// A reference audio-probe tap log.
pub struct RefLog {
    /// Sound-latch writes (machine seconds, value).
    pub latch: Vec<(f64, u8)>,
    /// Samples-CPU port writes, attributed to interrupt ticks.
    pub writes: Vec<PortWrite>,
    /// YM2151 writes (machine seconds, address 0/1, data).
    pub opm: Vec<(f64, u8, u8)>,
}

/// Read an `audio_taps.tsv` written by tools/reference/run_audio_probe.py.
pub fn read_taps(path: &Path) -> std::io::Result<RefLog> {
    let text = std::fs::read_to_string(path)?;
    let mut latch = Vec::new();
    let mut writes = Vec::new();
    let mut opm = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
        let t: f64 = f[0].parse().unwrap_or(0.0);
        let addr = u32::from_str_radix(f[3], 16).unwrap_or(0);
        let data = u32::from_str_radix(f[4], 16).unwrap_or(0);
        match f[2] {
            "latch" => latch.push((t, (data & 0xff) as u8)),
            "s2io" => {
                // A chip write belongs to its channel's tick parity (port 0
                // odd, port 1 even): the long sample-start path can finish its
                // reset write just past the next tick.
                let mut tick = (t * TICK_HZ as f64).floor() as u64;
                if addr < 2 && tick > 0 && (tick % 2 == 1) != (addr == 0) {
                    tick -= 1;
                }
                writes.push(PortWrite {
                    tick,
                    port: addr as u8,
                    data: data as u8,
                    t,
                });
            }
            "opm" => opm.push((t, addr as u8 & 1, data as u8)),
            _ => {}
        }
    }
    // A bank select (port 2) precedes the chip write of the same service
    // (0x012b / 0x017b, and the start path's reset): when that chip write
    // was moved back to its channel's tick, the bank select belongs there
    // too.
    for i in 0..writes.len().saturating_sub(1) {
        if writes[i].port == 2 && writes[i + 1].port < 2 && writes[i + 1].tick < writes[i].tick {
            writes[i].tick = writes[i + 1].tick;
        }
    }
    Ok(RefLog { latch, writes, opm })
}

/// Per-frame latch writes from a native capture's frames.tsv: (frame,
/// value, the vblank handler's 68000 cycles to the write). The cycles
/// column is required (captures from before ticket #35 lack it).
pub fn native_latch(capture: &Path) -> std::io::Result<Vec<(u32, u8, u32)>> {
    let text = std::fs::read_to_string(capture.join("frames.tsv"))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap_or("").split('\t').collect();
    let fi = header.iter().position(|h| *h == "frame").unwrap_or(0);
    let li = header.iter().position(|h| *h == "latch").unwrap_or(2);
    let ci = header
        .iter()
        .position(|h| *h == "latch_cycles")
        .ok_or_else(|| {
            std::io::Error::other(format!(
                "{}: frames.tsv has no latch_cycles column",
                capture.display()
            ))
        })?;
    Ok(lines
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            Some((
                c.get(fi)?.parse().ok()?,
                u8::from_str_radix(c.get(li)?, 16).ok()?,
                c.get(ci)?.parse().ok()?,
            ))
        })
        .collect())
}

/// A native run's latch writes, to place on a reference timeline.
#[derive(Debug, Clone, Copy)]
pub struct NativeLatch<'a> {
    /// (frame, value, handler cycles) rows, as [`native_latch`] reads them.
    pub frames: &'a [(u32, u8, u32)],
    /// The first native frame; the reference's writes before it stay.
    pub from: u32,
    /// The interrupt latency assumed, cycles ([`crate::IRQ_LATENCY_CYCLES`]).
    pub latency: u32,
}

/// The latch timeline: the reference's writes, or with `native` the
/// reference's before its start frame and the native frames' writes (at
/// [`crate::latch_write_time`]) from it on.
pub fn latch_timeline(log: &RefLog, native: Option<NativeLatch>) -> LatchTimeline {
    let Some(NativeLatch {
        frames,
        from,
        latency,
    }) = native
    else {
        return LatchTimeline::new(log.latch.clone());
    };
    let cut = from as f64 / FRAME_HZ as f64;
    let mut tl = LatchTimeline::new(log.latch.iter().copied().filter(|e| e.0 < cut).collect());
    for &(frame, value, cycles) in frames {
        if frame >= from {
            tl.push_frame(frame, value, cycles, latency);
        }
    }
    LatchTimeline::new(tl.into_events())
}

/// A diagnostic timeline: the native frames' latch values at the reference's
/// own write times (the reference's writes before `from`; from it on, each
/// native frame's value placed where the reference's write in that frame
/// is). Isolates what the interrupt-latency band costs: a run identical on
/// this timeline but not on [`latch_timeline`]'s differs only by the
/// latencies the translation cannot give.
pub fn native_at_reference_times(
    log: &RefLog,
    frames: &[(u32, u8, u32)],
    from: u32,
) -> LatchTimeline {
    let cut = from as f64 / FRAME_HZ as f64;
    let mut events: Vec<(f64, u8)> = log.latch.iter().copied().filter(|e| e.0 < cut).collect();
    let mut by_frame: std::collections::BTreeMap<u32, f64> = std::collections::BTreeMap::new();
    for &(t, _) in &log.latch {
        by_frame
            .entry((t * FRAME_HZ as f64).floor() as u32)
            .or_insert(t);
    }
    for &(frame, value, cycles) in frames {
        if frame >= from {
            let t = by_frame
                .get(&frame)
                .copied()
                .unwrap_or_else(|| crate::latch_write_time(frame, cycles, crate::IRQ_LATENCY_MID));
            events.push((t, value));
        }
    }
    LatchTimeline::new(events)
}

/// One native frame's latch write against the reference's: the latency the
/// reference's write time implies (its offset into the frame in 68000
/// cycles, less the native handler cycles), and both values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatchResidual {
    pub frame: u32,
    pub cycles: u32,
    pub latency: f64,
    pub native: u8,
    pub reference: u8,
}

/// Ticket #35's check: for every native frame with handler cycles, the
/// reference's latch write in that frame (writes land well inside their
/// frame, so the frame is the write time's floor) and the latency it
/// implies, which must lie in [`crate::IRQ_LATENCY_CYCLES`] if the
/// handler's cycles are right (except frame 1 from power-on, where `cycles`
/// is the full cycle count from `t = 0` placed by the reset path and the
/// residual is `t * M68K_HZ - cycles`). Native frames without a reference
/// write, and reference writes in native frames without cycles, are skipped.
pub fn latch_residuals(log: &RefLog, native: &[(u32, u8, u32)]) -> Vec<LatchResidual> {
    let mut by_frame: std::collections::BTreeMap<u32, (f64, u8)> =
        std::collections::BTreeMap::new();
    for &(t, value) in &log.latch {
        by_frame
            .entry((t * FRAME_HZ as f64).floor() as u32)
            .or_insert((t, value));
    }
    native
        .iter()
        .filter(|&&(_, _, cycles)| cycles > 0)
        .filter_map(|&(frame, value, cycles)| {
            let &(t, reference) = by_frame.get(&frame)?;
            let latency = if frame == 1 && cycles > M68K_HZ / FRAME_HZ {
                t * M68K_HZ as f64 - cycles as f64
            } else {
                let offset = (t - frame as f64 / FRAME_HZ as f64) * M68K_HZ as f64;
                offset - cycles as f64
            };
            Some(LatchResidual {
                frame,
                cycles,
                latency,
                native: value,
                reference,
            })
        })
        .collect()
}

/// The interrupt tick at which frame `frame` starts.
pub fn frame_tick(frame: u32) -> u64 {
    frame as u64 * TICK_HZ as u64 / FRAME_HZ as u64
}

/// Run the samples CPU from power-on through `last` ticks and compare its
/// port writes in `first..last` with the reference's. Ok(count) on a match.
pub fn compare_writes(
    audio2: Vec<u8>,
    mut tl: LatchTimeline,
    reference: &[PortWrite],
    first: u64,
    last: u64,
) -> Result<usize, String> {
    let mut snd = SamplesSound::new(audio2);
    let mut native = power_on_writes();
    while snd.ticks() < last {
        snd.tick(&mut tl, &mut |w| native.push(w));
    }
    let in_range = |w: &PortWrite| w.tick >= first && w.tick < last;
    let native: Vec<PortWrite> = native.into_iter().filter(in_range).collect();
    let reference: Vec<PortWrite> = reference.iter().copied().filter(in_range).collect();
    let n = native.len().min(reference.len());
    if let Some(i) = (0..n).find(|&i| native[i] != reference[i]) {
        let mut msg = format!("write #{i} (tick {}) differs:", reference[i].tick);
        for j in i.saturating_sub(2)..(i + 3).min(n) {
            msg += &format!("\n  reference {:?}   native {:?}", reference[j], native[j]);
        }
        return Err(msg);
    }
    if native.len() != reference.len() {
        return Err(format!(
            "{} reference writes, {} native",
            reference.len(),
            native.len()
        ));
    }
    Ok(n)
}

/// The two power-on writes (0x0000..0x0028: both chips reset), which
/// `SamplesSound::new` makes before any tick, with their times.
fn power_on_writes() -> Vec<PortWrite> {
    let mut v = Vec::new();
    crate::samples::SamplesCpu::power_on(&mut |w| v.push(w));
    v
}

/// Run the samples CPU as [`compare_writes`] does and compare each write's
/// modelled time with the reference's logged time: Ok((count, the largest
/// difference in T-states)), or the first write pair whose data differ.
pub fn compare_write_times(
    audio2: Vec<u8>,
    mut tl: LatchTimeline,
    reference: &[PortWrite],
    first: u64,
    last: u64,
) -> Result<(usize, f64), String> {
    let mut snd = SamplesSound::new(audio2);
    let mut native = power_on_writes();
    while snd.ticks() < last {
        snd.tick(&mut tl, &mut |w| native.push(w));
    }
    let in_range = |w: &PortWrite| w.tick >= first && w.tick < last;
    let native: Vec<PortWrite> = native.into_iter().filter(in_range).collect();
    let reference: Vec<PortWrite> = reference.iter().copied().filter(in_range).collect();
    if native.len() != reference.len() {
        return Err(format!(
            "{} reference writes, {} native",
            reference.len(),
            native.len()
        ));
    }
    let mut worst = 0.0f64;
    for (i, (n, r)) in native.iter().zip(&reference).enumerate() {
        if n != r {
            return Err(format!("write #{i} differs: reference {r:?} native {n:?}"));
        }
        let d = (n.t - r.t) * crate::samples::CLOCK_HZ;
        if d.abs() > worst.abs() {
            worst = d;
        }
    }
    Ok((native.len(), worst))
}

/// How the writes' modelled times depart from the reference's logged times
/// when the writes themselves match (as [`compare_write_times`]): the
/// number compared, the number more than half a T-state off, the largest
/// difference (T-states) and the tick of the first write off.
pub fn write_time_drift(
    audio2: Vec<u8>,
    mut tl: LatchTimeline,
    reference: &[PortWrite],
    first: u64,
    last: u64,
) -> Result<(usize, usize, f64, Option<u64>), String> {
    let mut snd = SamplesSound::new(audio2);
    let mut native = power_on_writes();
    while snd.ticks() < last {
        snd.tick(&mut tl, &mut |w| native.push(w));
    }
    let in_range = |w: &PortWrite| w.tick >= first && w.tick < last;
    let native: Vec<PortWrite> = native.into_iter().filter(in_range).collect();
    let reference: Vec<PortWrite> = reference.iter().copied().filter(in_range).collect();
    if native.len() != reference.len() {
        return Err(format!(
            "{} reference writes, {} native",
            reference.len(),
            native.len()
        ));
    }
    let (mut off, mut worst, mut first_off) = (0usize, 0.0f64, None);
    for (i, (n, r)) in native.iter().zip(&reference).enumerate() {
        if n != r {
            return Err(format!("write #{i} differs: reference {r:?} native {n:?}"));
        }
        let d = (n.t - r.t) * crate::samples::CLOCK_HZ;
        if d.abs() > 0.5 {
            off += 1;
            first_off.get_or_insert(n.tick);
        }
        if d.abs() > worst.abs() {
            worst = d;
        }
    }
    Ok((native.len(), off, worst, first_off))
}

/// Compare the two MSM5205 DAC outputs with the reference's hooked streams
/// at every channel tick of the hooked window. Ok((ticks, non-silent)).
pub fn compare_dac(
    audio2: Vec<u8>,
    mut tl: LatchTimeline,
    hook: &Hook,
) -> Result<(u64, u64), String> {
    let mut snd = SamplesSound::new(audio2);
    let first_tick = (hook.start * TICK_HZ as f64).ceil() as u64 + 1;
    // The last tick whose sample time (two ticks on, plus 5 us) is still
    // inside the hooked data.
    let last_tick = (hook.end * TICK_HZ as f64).floor() as u64 - 3;
    let (mut compared, mut active) = (0u64, 0u64);
    while snd.ticks() < last_tick {
        snd.tick(&mut tl, &mut |_| {});
        let k = snd.ticks();
        if k < first_tick {
            continue;
        }
        // Chip c changes only on its channel's ticks (chip 0 odd, chip 1
        // even); a tick's writes (decoded 15.6 us after each) finish before
        // that channel's next tick, even on the long sample-start path.
        // Sample the reference 5 us after the channel's next tick, before
        // its next write lands.
        let chip = if k % 2 == 1 { 0 } else { 1 };
        let t = (k + 2) as f64 / TICK_HZ as f64 + 5.0e-6;
        let r = stream_at(&hook.msm[chip], hook.start, 384_000.0, t);
        let n = snd.chip_output(chip);
        if (r - n).abs() > 1e-7 {
            return Err(format!(
                "tick {k} (t = {t:.6} s), chip {chip}: reference {r} native {n}"
            ));
        }
        if n != 0.0 {
            active += 1;
        }
        compared += 1;
    }
    Ok((compared, active))
}

/// Compare both MSM5205 outputs with the reference's hooked 384 kHz streams
/// at every stream sample, not only once per channel tick: each write's
/// nibble is decoded on the VCK edge six 384 kHz clocks after the write
/// (msm5205.cpp's capture timer), and a stream sample holds the level
/// reached by its end: a decode inside a sample's span shows in that
/// sample (the hooked streams change on the sample boundary just before
/// each decode). Returns (samples compared over both chips, samples
/// differing, the first differing (time, chip)), so a write whose time
/// drifts within its tick shows even when the per-tick levels match.
pub fn compare_dac_stream(
    audio2: Vec<u8>,
    mut tl: LatchTimeline,
    hook: &Hook,
) -> (u64, u64, Option<(f64, usize)>) {
    use crate::msm5205::Msm5205;
    const VCK: f64 = 384_000.0;
    let mut chips = [Msm5205::new(), Msm5205::new()];
    let mut steps: [Vec<(f64, f32)>; 2] = [Vec::new(), Vec::new()];
    for w in power_on_writes() {
        if w.port < 2 {
            chips[w.port as usize].write(w.data);
        }
    }
    let levels = [chips[0].output(), chips[1].output()];
    let mut snd = SamplesSound::new(audio2);
    let last_tick = (hook.end * TICK_HZ as f64).ceil() as u64 + 2;
    while snd.ticks() < last_tick {
        snd.tick(&mut tl, &mut |w: PortWrite| {
            if w.port < 2 {
                let c = w.port as usize;
                chips[c].write(w.data);
                // MAME's Z80 advances m_localtime by floor(10^18 / CLOCK_HZ) =
                // 279,365,114,840 attoseconds per T-state (52,200 attoseconds/s
                // below 1 / CLOCK_HZ).
                steps[c].push((w.t - w.t * 5.22e-14 + 6.0 / VCK, chips[c].output()));
            }
        });
    }
    let (mut compared, mut differing, mut first) = (0u64, 0u64, None);
    for c in 0..2 {
        let (mut k, mut level) = (0usize, levels[c]);
        for (i, r) in hook.msm[c].iter().enumerate() {
            let s = hook.start + i as f64 / VCK;
            while k < steps[c].len() && steps[c][k].0 < s + 1.0 / VCK {
                level = steps[c][k].1;
                k += 1;
            }
            compared += 1;
            if (level - r).abs() > 1e-7 {
                differing += 1;
                if first.is_none_or(|(t, _)| s < t) {
                    first = Some((s, c));
                }
            }
        }
    }
    (compared, differing, first)
}

/// Render the chips' mix (one value per tick) for ticks `first..last`.
pub fn render(audio2: Vec<u8>, mut tl: LatchTimeline, first: u64, last: u64) -> Vec<f32> {
    let mut snd = SamplesSound::new(audio2);
    let mut pcm = Vec::new();
    while snd.ticks() < last {
        let v = snd.tick(&mut tl, &mut |_| {});
        if snd.ticks() >= first {
            pcm.push(v);
        }
    }
    pcm
}

/// MAME's default sample rate: the rate of its `-wavwrite` recording.
pub const WAV_RATE: u32 = 48_000;

/// Both translated sound CPUs from power-on on `tl`, mixed as the pinned
/// MAME mixes them for its recording (`mix::GameSound`: the YM2151 at 0.6,
/// each MSM5205 at 1.0, saturated at full scale), as the mono average of
/// the two speaker channels at [`WAV_RATE`] over frames `first..=last`. For
/// measurement and listening against `audio.wav`, not for sample equality:
/// MAME's band-limiting resampler is not modelled, and the per-device
/// streams are compared exactly by `compare_dac` and the music runners.
pub fn render_mix(
    audio2: Vec<u8>,
    rom: Vec<u8>,
    tl: LatchTimeline,
    first: u32,
    last: u32,
) -> Vec<f32> {
    let (t0, t_end) = (
        first as f64 / FRAME_HZ as f64,
        (last + 1) as f64 / FRAME_HZ as f64,
    );
    let mut snd = crate::mix::GameSound::power_on(audio2, rom, tl, f64::INFINITY);
    snd.skip_to(t0);
    let n = ((t_end - t0) * WAV_RATE as f64).round() as usize;
    let mut pcm = Vec::with_capacity(n);
    let mut ready = t0;
    for i in 0..n {
        let t = t0 + i as f64 / WAV_RATE as f64;
        if t > ready {
            snd.wait_ready(t);
            ready = snd.ready_until(t);
        }
        let [l, r] = snd.sample_at(t);
        pcm.push((l + r) / 2.0);
    }
    pcm
}

/// The reference's hooked MSM5205 streams: contiguous 384 kHz buffers from
/// `start` to `end` (seconds).
pub struct Hook {
    pub start: f64,
    pub end: f64,
    pub msm: [Vec<f32>; 2],
}

pub fn read_hook(dir: &Path) -> std::io::Result<Hook> {
    let index = std::fs::read_to_string(dir.join("hook_index.tsv"))?;
    let (mut start, mut end) = (f64::MAX, 0.0f64);
    for line in index.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        if c.get(2) != Some(&":msm1") {
            continue;
        }
        let t: f64 = c[1].parse().unwrap_or(0.0);
        let n: f64 = c[4].parse().unwrap_or(0.0);
        start = start.min(t - n / 384_000.0);
        end = end.max(t);
    }
    let read = |name: &str| -> std::io::Result<Vec<f32>> {
        let b = std::fs::read(dir.join(name))?;
        Ok(b.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    };
    Ok(Hook {
        start,
        end,
        msm: [read("hook_msm1.f32")?, read("hook_msm2.f32")?],
    })
}

pub fn stream_at(stream: &[f32], start: f64, rate: f64, t: f64) -> f32 {
    let i = ((t - start) * rate).floor();
    if i < 0.0 {
        return 0.0;
    }
    stream.get(i as usize).copied().unwrap_or(0.0)
}

pub fn read_wav_mono(path: &Path) -> Result<(Vec<f32>, u32), String> {
    let mut r = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = r.spec();
    let ch = spec.channels as usize;
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => r
            .samples::<i32>()
            .map(|s| s.map(|v| v as f32 / (1u32 << (spec.bits_per_sample - 1)) as f32))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?,
        hound::SampleFormat::Float => r
            .samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?,
    };
    Ok((
        raw.chunks(ch)
            .map(|c| c.iter().sum::<f32>() / ch as f32)
            .collect(),
        spec.sample_rate,
    ))
}

pub struct Measures {
    pub onset: f64,
    pub active: f64,
    pub rms: f64,
    pub peak: f64,
    pub zcr: f64,
    pub centroid: f64,
}

/// Onset / activity from a 5 ms RMS envelope against 10% of its peak; zero
/// crossings and the spectral centroid (8192-point DFT magnitude) over the
/// active span.
pub fn measure(x: &[f32], rate: f64) -> Measures {
    let win = (0.005 * rate) as usize;
    let env: Vec<f64> = x
        .chunks(win.max(1))
        .map(|c| (c.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / c.len() as f64).sqrt())
        .collect();
    let peak_env = env.iter().cloned().fold(0.0, f64::max);
    let on = env.iter().position(|e| *e > 0.1 * peak_env).unwrap_or(0);
    let off = env.iter().rposition(|e| *e > 0.1 * peak_env).unwrap_or(0);
    let span = &x[(on * win).min(x.len())..((off + 1) * win).min(x.len())];
    let rms = (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt();
    let peak = x.iter().map(|v| v.abs() as f64).fold(0.0, f64::max);
    let zc = span
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    let n = span.len().min(8192);
    let mut num = 0.0;
    let mut den = 0.0;
    for k in 1..n / 2 {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, v) in span[..n].iter().enumerate() {
            let a = -2.0 * std::f64::consts::PI * (k * i) as f64 / n as f64;
            re += *v as f64 * a.cos();
            im += *v as f64 * a.sin();
        }
        let mag = (re * re + im * im).sqrt();
        num += mag * k as f64 * rate / n as f64;
        den += mag;
    }
    Measures {
        onset: on as f64 * win as f64 / rate,
        active: (off + 1 - on) as f64 * win as f64 / rate,
        rms,
        peak,
        zcr: zc as f64 / (span.len().max(1) as f64 / rate),
        centroid: if den > 0.0 { num / den } else { 0.0 },
    }
}

pub fn best_correlation(a: &[f32], b: &[f32], max_lag: isize) -> (f64, isize) {
    let n = a.len().min(b.len()) as isize;
    let mut best = (f64::MIN, 0);
    let mut lag = -max_lag;
    while lag <= max_lag {
        let (mut ab, mut aa, mut bb) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..n {
            let j = i + lag;
            if j < 0 || j >= n {
                continue;
            }
            let (x, y) = (a[i as usize] as f64, b[j as usize] as f64);
            ab += x * y;
            aa += x * x;
            bb += y * y;
        }
        let c = ab / (aa * bb).sqrt().max(1e-12);
        if c > best.0 {
            best = (c, lag);
        }
        lag += 4;
    }
    best
}

/// A hooked device stream of the reference (first output channel).
pub struct DeviceStream {
    pub start: f64,
    pub rate: f64,
    pub samples: Vec<f32>,
    /// The index of `samples[0]` in the device's output: MAME's stream
    /// update at time T delivers the samples with index <= floor(T x rate),
    /// so the hook's first update (time T1, n1 samples) begins at
    /// floor(T1 x rate) + 1 - n1. A replay of a capture's register writes
    /// matches its hooked stream exactly at this offset
    /// (`sf-audio compare-opm`; the fraction of T1 x rate decides whether
    /// it is one above or equal to round(start x rate)).
    pub first: u64,
}

impl DeviceStream {
    /// The index of the stream's first sample in the device's output.
    pub fn first_sample(&self) -> u64 {
        self.first
    }
}

/// Read the reference's hooked stream of `device` (`ymsnd`, `msm1`, ...).
pub fn read_device_stream(dir: &Path, device: &str, rate: f64) -> std::io::Result<DeviceStream> {
    let index = std::fs::read_to_string(dir.join("hook_index.tsv"))?;
    let tag = format!(":{device}");
    let mut start = f64::MAX;
    let mut first = i64::MAX;
    for line in index.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        if c.get(2) != Some(&tag.as_str()) {
            continue;
        }
        let t: f64 = c[1].parse().unwrap_or(0.0);
        let n: f64 = c[4].parse().unwrap_or(0.0);
        start = start.min(t - n / rate);
        first = first.min((t * rate).floor() as i64 + 1 - n as i64);
    }
    let b = std::fs::read(dir.join(format!("hook_{device}.f32")))?;
    let samples = b
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    Ok(DeviceStream {
        start,
        rate,
        samples,
        first: first.max(0) as u64,
    })
}

/// The last sample MAME has generated before a bus access at chip clock `t`.
/// Its stream update includes the current sample. A video update at the same
/// quantized CPU clock can include one more sample when the frame boundary
/// coincides with an FM sample boundary (the screen frame has then already
/// advanced in the write tap, even though the tapped time is just before the
/// nominal frame time).
fn mame_current_sample(t: u64) -> u64 {
    let (clock, rate, frames) = (
        crate::opm::OPM_CLOCK as u64,
        crate::opm::OPM_RATE as u64,
        FRAME_HZ as u64,
    );
    let current = t * rate / clock;
    let frame = (t * frames).div_ceil(clock);
    if frame * clock / frames == t {
        current.max(frame * rate / frames)
    } else {
        current
    }
}

/// Replay the reference's YM2151 writes (machine seconds) into the chip
/// model from power-on, with MAME's stream updates before each write, and
/// return the left output (scaled like MAME's streams, 1/32768) for samples
/// `0..count`. `delta` shifts the write/sample comparison in sample units.
pub fn replay_opm(writes: &[(f64, u8, u8)], count: usize, delta: f64) -> Vec<f32> {
    let mut opm = crate::opm::Opm::new();
    let rate = crate::opm::OPM_RATE as f64;
    let targets: Vec<u64> = writes
        .iter()
        .map(|w| {
            let clock = ((w.0 + delta / rate) * crate::opm::OPM_CLOCK as f64).round() as u64;
            mame_current_sample(clock) + 1
        })
        .collect();
    let mut next = 0;
    let mut out = Vec::with_capacity(count);
    for n in 0..count {
        while next < writes.len() && targets[next] <= n as u64 {
            opm.at_time(writes[next].0);
            opm.write(writes[next].1, writes[next].2);
            next += 1;
        }
        out.push(opm.sample()[0] as f32 / 32768.0);
    }
    out
}

/// One YM2151 register write of a run: output sample index, register, data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpmWrite {
    pub sample: u64,
    pub reg: u8,
    pub data: u8,
    /// The write's chip clock (exact for the reference's, the model's for
    /// the native writes).
    pub clock: u64,
}

/// The reference's YM2151 writes as (sample index, register, data), the
/// sample being the first not yet generated when the device write occurs.
pub fn reference_opm_writes(log: &RefLog) -> Vec<OpmWrite> {
    let mut reg = 0u8;
    let mut out = Vec::new();
    for &(t, addr, data) in &log.opm {
        if addr == 0 {
            reg = data;
        } else {
            let clock = (t * crate::opm::OPM_CLOCK as f64).round() as u64;
            out.push(OpmWrite {
                sample: mame_current_sample(clock) + 1,
                reg,
                data,
                clock,
            });
        }
    }
    out
}

/// A source of latch writes that arrive while the music CPU runs (the live
/// player, `mix::GameSound`): the chip asks for the next frame's write
/// whenever the CPU reaches a time the writes known so far do not cover.
pub(crate) trait LatchFeed {
    /// Take the samples generated so far: `left` / `right` from sample
    /// index `first` (the buffers are left empty).
    fn flush(&mut self, first: u64, left: &mut Vec<f32>, right: &mut Vec<f32>);
    /// Block until the next frame's write is known: (frame, value, the
    /// vblank handler's cycles, the latency); None when no more come.
    fn next(&mut self) -> Option<(u32, u8, u32, u32)>;
}

/// The chip and the 68000-side inputs as the timed music CPU sees them.
pub(crate) struct TimedChip {
    pub(crate) opm: crate::opm::Opm,
    /// Left output per sample (1/32768 scale).
    pub(crate) out: Vec<f32>,
    /// Right output per sample (1/32768 scale).
    pub(crate) right: Vec<f32>,
    /// (left + right) / 2 per sample.
    pub(crate) mix: Vec<f32>,
    log: Vec<OpmWrite>,
    /// Whether chip writes and the mono mix are kept (the reference
    /// comparisons) or not (the live player, which runs indefinitely and
    /// takes the two channels as they are generated).
    keep_log: bool,
    pub(crate) latch: LatchTimeline,
    pub(crate) nmis: Vec<u64>,
    nmi_next: usize,
    addr: u8,
    end_sample: u64,
    /// Every NMI before this clock is in `nmis` and every write before it
    /// in `latch` (`u64::MAX` for the batch runners, which know them all);
    /// `feed` supplies the later ones.
    horizon: u64,
    feed: Option<Box<dyn LatchFeed>>,
    /// Debug aid: a clock range (from `SF_MUSIC_TRACE=t0..t1`, seconds)
    /// within which every chip access and NMI is printed to stderr.
    trace: Option<(u64, u64)>,
}

impl TimedChip {
    /// A chip at power-on whose CPU takes an NMI at `floor(t × clock)` for
    /// every write of `latch` (more can be appended to `nmis` and `latch`
    /// as long as they lie after the CPU's time); samples are generated up
    /// to `end_sample`.
    pub(crate) fn new(latch: LatchTimeline, end_sample: u64, keep_log: bool) -> Self {
        let clock = crate::opm::OPM_CLOCK as f64;
        let nmis: Vec<u64> = latch
            .clone()
            .into_events()
            .iter()
            .map(|e| (e.0 * clock).floor() as u64)
            .collect();
        let capacity = if end_sample == u64::MAX {
            0
        } else {
            end_sample as usize
        };
        TimedChip {
            opm: crate::opm::Opm::new(),
            out: Vec::with_capacity(capacity),
            right: Vec::with_capacity(capacity),
            mix: Vec::with_capacity(capacity),
            log: Vec::new(),
            keep_log,
            latch,
            nmis,
            nmi_next: 0,
            addr: 0,
            end_sample,
            horizon: u64::MAX,
            feed: None,
            trace: trace_range(),
        }
    }

    /// Writes before clock `horizon` are known; `feed` supplies the rest.
    pub(crate) fn set_feed(&mut self, feed: Box<dyn LatchFeed>, horizon: u64) {
        self.feed = Some(feed);
        self.horizon = horizon;
    }

    /// Hand the samples generated so far to the feed (the live player).
    pub(crate) fn flush(&mut self) {
        if let Some(feed) = self.feed.as_mut() {
            let first = self.opm.samples() - self.out.len() as u64;
            feed.flush(first, &mut self.out, &mut self.right);
            self.out.shrink_to(1 << 16);
            self.right.shrink_to(1 << 16);
        }
    }

    /// Make every latch write before clock `t` known, waiting for the feed
    /// (the CPU is at `t`, so every sample before it is final and is handed
    /// over first). Without a feed every write is known already.
    fn ensure_known(&mut self, t: u64) {
        while t >= self.horizon {
            self.sync(t);
            self.flush();
            let next = self.feed.as_mut().and_then(|f| f.next());
            match next {
                Some((frame, value, cycles, latency)) => {
                    let wt = crate::latch_write_time(frame, cycles, latency);
                    // What the CPU has passed is never needed again.
                    self.latch.forget_read();
                    self.nmis.drain(..self.nmi_next);
                    self.nmi_next = 0;
                    self.latch.push_frame(frame, value, cycles, latency);
                    self.nmis
                        .push((wt * crate::opm::OPM_CLOCK as f64).floor() as u64);
                    // The next frame's write lies at or after its start.
                    self.horizon = ((frame + 1) as f64 / FRAME_HZ as f64
                        * crate::opm::OPM_CLOCK as f64)
                        .floor() as u64;
                }
                None => {
                    self.feed = None;
                    self.horizon = u64::MAX;
                }
            }
        }
    }

    fn tracing(&self, t: u64) -> bool {
        self.trace.is_some_and(|(a, b)| t >= a && t <= b)
    }
}

fn trace_range() -> Option<(u64, u64)> {
    let v = std::env::var("SF_MUSIC_TRACE").ok()?;
    let (a, b) = v.split_once("..")?;
    let clock = crate::opm::OPM_CLOCK as f64;
    Some((
        (a.parse::<f64>().ok()? * clock) as u64,
        (b.parse::<f64>().ok()? * clock) as u64,
    ))
}

impl TimedChip {
    /// Update the stream through MAME's current sample before an access at
    /// chip clock `t`; the access then affects the following sample.
    pub(crate) fn sync(&mut self, t: u64) {
        let current = mame_current_sample(t);
        while self.opm.samples() < self.end_sample && self.opm.samples() <= current {
            self.push_sample();
        }
    }

    fn push_sample(&mut self) {
        let [l, r] = self.opm.sample();
        self.out.push(l as f32 / 32768.0);
        self.right.push(r as f32 / 32768.0);
        if self.keep_log {
            self.mix.push((l + r) as f32 / 65536.0);
        }
    }
}

impl crate::music::MusicBus for TimedChip {
    fn status(&mut self, t: u64) -> u8 {
        self.sync(t);
        let st = self.opm.status_at(t);
        if self.tracing(t) {
            eprintln!(
                "{t} status {st:02x} (irq since {}, deadlines {:?}, flags {:?})",
                self.opm.irq_since(),
                self.opm.deadlines(),
                self.opm.flag_state()
            );
        }
        st
    }
    fn write(&mut self, t: u64, port: u8, data: u8) {
        self.sync(t);
        self.opm.at_clock(t);
        self.opm.write(port, data);
        if port == 0 {
            self.addr = data;
        } else {
            if self.keep_log {
                self.log.push(OpmWrite {
                    sample: self.opm.samples(),
                    reg: self.addr,
                    data,
                    clock: t,
                });
            }
            if self.tracing(t) {
                eprintln!(
                    "{t} write {:02x} = {data:02x} (sample {})",
                    self.addr,
                    self.opm.samples()
                );
            }
        }
    }
    fn latch(&mut self, t: u64) -> u8 {
        self.ensure_known(t);
        let v = self.latch.read_at(t as f64 / crate::opm::OPM_CLOCK as f64);
        if self.tracing(t) {
            eprintln!("{t} latch read {v:02x}");
        }
        v
    }
    fn take_nmi(&mut self, t: u64) -> Option<u64> {
        self.ensure_known(t);
        let n = *self.nmis.get(self.nmi_next)?;
        if n <= t {
            self.nmi_next += 1;
            if self.tracing(t) {
                eprintln!("{t} NMI (due {n})");
            }
            Some(n)
        } else {
            None
        }
    }
}

/// The music CPU's idle-loop phase at the sf-music-01 fixture: its `ei`
/// starts 12 T-states after the fixture instant. This is the one piece of
/// CPU state the RAM dump does not carry (the loop is 16 T-states long, so
/// it decides on which loop boundary each interrupt is accepted); it was
/// derived from the reference's write timing (the `music_phase` example):
/// with it every one of the phrase's 2,283 writes lands on the reference's
/// exact chip clock, and every other phase puts interrupt acceptance on a
/// grid shifted from MAME's.
pub const MUSIC_FIXTURE_IDLE_PHASE: u64 = 12;

/// A music run: before `fixture_time` the reference's YM2151 writes are
/// replayed (rebuilding the chip state); from it the translated music CPU,
/// starting from the RAM image `ram` in its idle loop at `idle_phase`
/// T-states past the fixture, runs with its timing:
/// the idle loop (0x0009 `ei` 4, 0x000a `jr` 12) accepts the chip's
/// interrupt at the end of `jr` (13 T-states, IM 1), NMIs (one per latch
/// write in `latch`) are taken first, and a flag still pending after `reti`
/// is accepted at once. Returns the native writes, the left output (1/32768
/// scale) for samples `0..count`, and the untranslated branches reached.
pub fn run_music(
    rom: Vec<u8>,
    ram: Vec<u8>,
    reference: &RefLog,
    fixture_time: f64,
    latch: LatchTimeline,
    idle_phase: u64,
    count: u64,
) -> MusicRun {
    let rate = crate::opm::OPM_RATE as f64;
    let clock = crate::opm::OPM_CLOCK as f64;
    let fixture_sample = (fixture_time * rate).ceil() as u64;
    let nmis: Vec<u64> = latch
        .clone()
        .into_events()
        .iter()
        .filter(|e| e.0 >= fixture_time)
        .map(|e| (e.0 * clock).floor() as u64)
        .collect();
    let mut chip = TimedChip::new(latch, count, true);
    chip.nmis = nmis;
    // Replay: the chip state at the fixture.
    let mut next = 0;
    for n in 0..fixture_sample.min(count) {
        let t = n as f64 / rate;
        while next < reference.opm.len() && reference.opm[next].0 <= t {
            chip.opm.at_time(reference.opm[next].0);
            chip.opm.write(reference.opm[next].1, reference.opm[next].2);
            next += 1;
        }
        chip.push_sample();
    }
    // Native: the idle loop from the fixture.
    let mut cpu = crate::music::MusicCpu::new(rom, ram);
    cpu.t = (fixture_time * clock).ceil() as u64 + idle_phase;
    let end = (count as f64 / rate * clock) as u64;
    idle_loop(&mut cpu, &mut chip, end, end);
    finish(chip, cpu, count)
}

/// A music run from power-on (ticket #36): the chip from its reset state
/// and the CPU from its reset vector at clock 0 — the boot code, then the
/// idle loop — with `latch`'s writes (the reference's, or the native
/// engine's) as NMIs and reads from the start. No RAM fixture, no fitted
/// idle phase: the loop's phase follows from the boot code's T-states.
/// Returns the writes, the left output for samples `0..count`, the
/// untranslated branches reached and the final RAM.
pub fn run_music_from_power_on(rom: Vec<u8>, latch: LatchTimeline, count: u64) -> MusicRun {
    run_music_from_power_on_with_fault(rom, latch, count, crate::music::MusicFault::default())
}

/// [`run_music_from_power_on`] with an optional [crate::music::MusicFault]
/// injected for negative-control (C-NEG) verification (ticket #78).
pub fn run_music_from_power_on_with_fault(
    rom: Vec<u8>,
    latch: LatchTimeline,
    count: u64,
    fault: crate::music::MusicFault,
) -> MusicRun {
    let rate = crate::opm::OPM_RATE as f64;
    let clock = crate::opm::OPM_CLOCK as f64;
    let mut chip = TimedChip::new(latch, count, true);
    let mut cpu = crate::music::MusicCpu::new(rom, vec![0; 0x800]);
    cpu.fault = fault;
    cpu.boot(&mut chip);
    let end = (count as f64 / rate * clock) as u64;
    idle_loop(&mut cpu, &mut chip, end, end);
    finish(chip, cpu, count)
}

/// The idle loop (0x0009 `ei` 4, 0x000a `jr` 12) through clock `end`: NMIs
/// are taken between its instructions; the chip's interrupt (13 T-states,
/// IM 1) at the end of `jr` — `ei` shadows the instruction after it — or,
/// when an NMI was taken after `ei`, at `jr`'s start as the NMI's return
/// ends the shadow. The same rule inside a timer handler can defer its outer
/// `reti` until a nested interrupt chain unwinds. A flag still pending after
/// `reti` is accepted at once —
/// while the CPU's time is before `chain_end` (the batch runners stop at
/// their end; the live player, which resumes the loop later, passes
/// `u64::MAX` so a chain is never cut where a longer run would continue it).
/// The loop stops only at its top, so a later call resumes it exactly.
pub(crate) fn idle_loop(
    cpu: &mut crate::music::MusicCpu,
    chip: &mut TimedChip,
    end: u64,
    chain_end: u64,
) {
    use crate::music::MusicBus;
    let nmi = |cpu: &mut crate::music::MusicCpu, chip: &mut TimedChip| -> bool {
        let mut taken = false;
        while chip.take_nmi(cpu.t).is_some() {
            cpu.t += 11 + 11;
            let v = chip.latch(cpu.t + 10);
            cpu.t += 13 + 13 + 10 + 14;
            cpu.set_nmi_copy(v);
            taken = true;
        }
        taken
    };
    // An IRQ accepted after an NMI at a handler's `ei` runs before that
    // handler's `reti`. Keep those outer returns across the whole chain.
    let mut deferred_reti = 0u32;
    let mut irq = |cpu: &mut crate::music::MusicCpu, chip: &mut TimedChip| loop {
        chip.sync(cpu.t);
        if chip.opm.irq_at(cpu.t) && cpu.t < chain_end {
            if chip.tracing(cpu.t) {
                eprintln!(
                    "{} interrupt accepted (irq since {})",
                    cpu.t,
                    chip.opm.irq_since()
                );
            }
            cpu.t += 13;
            let nmi_after_ei = cpu.interrupt(chip);
            chip.sync(cpu.t);
            if nmi_after_ei && chip.opm.irq_at(cpu.t) {
                deferred_reti += 1;
            } else {
                cpu.reti(chip);
            }
            if chip.tracing(cpu.t) {
                eprintln!("{} handler done", cpu.t);
            }
        } else if deferred_reti > 0 && !chip.opm.irq_at(cpu.t) {
            cpu.reti(chip);
            deferred_reti -= 1;
        } else {
            break;
        }
    };
    while cpu.t < end {
        cpu.t += 4; // ei
        if nmi(cpu, chip) {
            irq(cpu, chip);
        }
        cpu.t += 12; // jr 0x0009
        nmi(cpu, chip);
        irq(cpu, chip);
    }
}

/// One frame end's music-CPU RAM: (frame, the CPU idle there, the RAM).
pub type FrameRam = (u32, bool, Vec<u8>);

/// The music CPU's RAM at frame ends (the milestone audit, ticket #39): run
/// from power-on on `latch` as [`run_music_from_power_on`] does, its idle
/// loop stopped at the end of each frame of `frames` ((F + 1) / 60 s, the
/// instant of the reference's dump of frame F) and resumed (the loop stops
/// only at its top, so stopping changes nothing). Returns, per frame,
/// whether the CPU was idle there — the loop stopped at most 16 T-states
/// past the instant, so at the instant the CPU was in the idle loop or in
/// a handler's last 16 T-states (the timer handler's `ei; reti`, 18; the
/// NMI's `pop af; retn`, 24), which store nothing, so the RAM is the
/// instant's — and the RAM, with the untranslated branches reached.
pub fn music_ram_at_frame_ends(
    rom: Vec<u8>,
    latch: LatchTimeline,
    frames: &[u32],
) -> (Vec<FrameRam>, Vec<u16>) {
    music_ram_at_frame_ends_with_fault(rom, latch, frames, crate::music::MusicFault::default())
}

/// [music_ram_at_frame_ends] with an optional [crate::music::MusicFault]
/// injected for negative-control (C-NEG) verification (ticket #78).
pub fn music_ram_at_frame_ends_with_fault(
    rom: Vec<u8>,
    latch: LatchTimeline,
    frames: &[u32],
    fault: crate::music::MusicFault,
) -> (Vec<FrameRam>, Vec<u16>) {
    let clock = crate::opm::OPM_CLOCK as f64;
    let mut chip = TimedChip::new(latch, u64::MAX, false);
    let mut cpu = crate::music::MusicCpu::new(rom, vec![0; 0x800]);
    cpu.fault = fault;
    cpu.boot(&mut chip);
    let mut out = Vec::with_capacity(frames.len());
    for &f in frames {
        let end = ((f + 1) as f64 / FRAME_HZ as f64 * clock).ceil() as u64;
        idle_loop(&mut cpu, &mut chip, end, u64::MAX);
        out.push((f, cpu.t - end <= 16, cpu.ram().to_vec()));
        chip.out.clear();
        chip.right.clear();
    }
    (out, cpu.unrecovered.iter().copied().collect())
}

/// Generate the run's remaining samples and collect the result.
fn finish(mut chip: TimedChip, cpu: crate::music::MusicCpu, count: u64) -> MusicRun {
    let end = (count as f64 / crate::opm::OPM_RATE as f64 * crate::opm::OPM_CLOCK as f64) as u64;
    chip.sync(end);
    while (chip.out.len() as u64) < count {
        chip.push_sample();
    }
    let unrecovered = cpu.unrecovered.iter().copied().collect();
    let observations = cpu.observations;
    MusicRun {
        writes: chip.log,
        left: chip.out,
        mix: chip.mix,
        unrecovered,
        observations,
        ram: cpu.ram().to_vec(),
    }
}

/// The result of [`run_music`].
pub struct MusicRun {
    /// The native YM2151 writes (from the fixture on).
    pub writes: Vec<OpmWrite>,
    /// Left output per sample (1/32768 scale), the reference hook's channel.
    pub left: Vec<f32>,
    /// (left + right) / 2 per sample.
    pub mix: Vec<f32>,
    /// Untranslated branches reached.
    pub unrecovered: Vec<u16>,
    /// Reached branches that reference RAM alone cannot distinguish.
    pub observations: crate::music::MusicObservations,
    /// The music CPU's RAM (0xc000-0xc7ff) at the end of the run, for
    /// comparison with a reference checkpoint's dump (the stack area the
    /// translation does not keep aside).
    pub ram: Vec<u8>,
}

/// How many of the native writes land on the reference's exact chip clock
/// (paired in order up to the shorter list): the idle-loop phase at a
/// fixture is the one that puts the most there (docs/reference/music-phrase.md).
pub fn exact_clock_matches(native: &[OpmWrite], reference: &[OpmWrite]) -> usize {
    native
        .iter()
        .zip(reference)
        .filter(|(n, r)| n.clock == r.clock && n.reg == r.reg && n.data == r.data)
        .count()
}
