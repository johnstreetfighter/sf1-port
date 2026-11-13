//! sf-audio: native sound for the SF1 port.
//!
//! Per ADR-0001 the original sound programs are translated into Rust and
//! only the sound chips are modelled:
//!
//! - [`samples`] (ticket #10): the samples CPU's program — the coroutine
//!   scheduler that streams ADPCM nibbles to two MSM5205s.
//! - [`msm5205`]: the ADPCM decoder model (provenance in its module doc).
//! - [`music`] (tickets #11, #33, #34): the music CPU's program — the
//!   YM2151 sound driver's interrupt handler, command intake, sequencer,
//!   special commands and stops, and the sound-effect player.
//! - [`tunes`] and [`score`] (ticket #195, ADR-0009): the extended tune
//!   table overhaul modes add behind the ROM's, and the converter from a
//!   readable score to the driver's channel streams.
//! - [`opm`] (ticket #11): the YM2151, synthesized by ymfm.
//! - [`mix`] (ticket #37): both programs and their chips from power-on,
//!   mixed as the pinned MAME mixes them — the measured mix and the
//!   launcher's live output.
//! - [`announcer`] (`EH8`, ADR 0011): the overhaul announcer's third
//!   MSM5205, mixed beside the board's two, and his voice pack.
//! - [`voice_pipeline`] (`EH9`): the voice pipeline's codec steps (encode a
//!   take, decode it for listening, a ROM sample, write the pack).
//!
//! The 68000 side of the boundary is the sound latch: one command byte per
//! frame (0xff when idle), written at its logged time from a reference tap
//! log, or — when a native run supplies it — at the frame start plus the
//! interrupt latency ([`IRQ_LATENCY_CYCLES`]) and the vblank handler's
//! cycles to the write, which the engine accounts per frame (ticket #35,
//! [`latch_write_time`]). The samples CPU reads it at the T-state its
//! poll's `in` runs (ticket #32; [`samples`]). See docs/reference/hit-sound.md,
//! fight-samples.md and latch-timing.md.

pub mod announcer;
pub mod check;
mod impact;
pub mod mix;
pub mod msm5205;
pub mod music;
pub mod music_pack;
pub mod opm;
pub mod samples;
pub mod score;
pub mod tunes;
pub mod voice_pipeline;

use msm5205::Msm5205;
use samples::{PortWrite, SampleRom, SamplesCpu};

/// Samples-CPU interrupt rate (sf.cpp: periodic irq0 at 8000 Hz).
pub const TICK_HZ: u32 = 8000;
/// Screen refresh (sf.cpp: `set_refresh_hz(60)`).
pub const FRAME_HZ: u32 = 60;
/// The pinned MAME's scheduling quantum for this machine, seconds: sf.cpp
/// sets none, so the scheduler uses the executing devices' minimum — the
/// 68000's four cycles at 8 MHz. Each quantum runs the 68000 before the
/// Z80s.
pub const SCHEDULER_QUANTUM: f64 = 4.0 / 8_000_000.0;
/// The main CPU's clock (sf.cpp: 8 MHz).
pub const M68K_HZ: u32 = 8_000_000;
/// The vblank interrupt's acceptance latency: 68000 cycles from the frame
/// start to the handler's first instruction. The handler's own cycles to
/// the latch write are accounted by the engine per frame; the latency
/// depends on where the main loop stood when the interrupt arrived — the
/// interrupted instruction's remainder, the task scheduler's
/// interrupt-masked stretches, the 44-cycle exception entry — which the
/// translation does not model (ADR-0001). Observed over 52 traced frames of
/// two reference runs: 45..=138 (docs/reference/latch-timing.md).
pub const IRQ_LATENCY_CYCLES: std::ops::RangeInclusive<u32> = 45..=138;
/// The latency assumed when a run supplies the latch from the native
/// engine: the observed band's middle. The checks run the band's ends too.
pub const IRQ_LATENCY_MID: u32 = 92;

/// The time of frame `frame`'s sound-latch write: the frame start, the
/// interrupt latency and the handler's `cycles` to the write. On frame 1
/// from power-on (`cycles > M68K_HZ / FRAME_HZ`), the reset path
/// (`0x3b000..0x3b050`) holds vblank 1 with interrupts masked (`SR = 0x2700`)
/// across the end of frame 0 into frame 1, so `cycles` is the full 68000
/// cycle count from power-on (`t = 0`) to the write (`186,957` cycles =
/// `0.023369625 s`) and the slot-scan latency band ([`IRQ_LATENCY_CYCLES`])
/// does not apply.
pub fn latch_write_time(frame: u32, cycles: u32, latency: u32) -> f64 {
    if frame == 1 && cycles > M68K_HZ / FRAME_HZ {
        return cycles as f64 / M68K_HZ as f64;
    }
    frame as f64 / FRAME_HZ as f64 + (latency + cycles) as f64 / M68K_HZ as f64
}

/// The sound latch over time: (seconds, value) writes in order.
#[derive(Debug, Clone, Default)]
pub struct LatchTimeline {
    events: Vec<(f64, u8)>,
    cursor: usize,
    current: u8,
}

impl LatchTimeline {
    pub fn new(mut events: Vec<(f64, u8)>) -> Self {
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        LatchTimeline {
            events,
            cursor: 0,
            current: 0,
        }
    }

    /// Append one native frame's latch write: `cycles` is the vblank
    /// handler's cycles to the write (`MachineState::latch_cycles`),
    /// `latency` the interrupt latency assumed ([`IRQ_LATENCY_CYCLES`]).
    pub fn push_frame(&mut self, frame: u32, value: u8, cycles: u32, latency: u32) {
        self.events
            .push((latch_write_time(frame, cycles, latency), value));
    }

    /// The writes, in order.
    pub fn into_events(self) -> Vec<(f64, u8)> {
        self.events
    }

    /// The latch value at exactly `t` (latest write at or before it).
    /// Calls must use non-decreasing t.
    pub fn read_at(&mut self, t: f64) -> u8 {
        while self.cursor < self.events.len() && self.events[self.cursor].0 <= t {
            self.current = self.events[self.cursor].1;
            self.cursor += 1;
        }
        self.current
    }

    /// Drop the writes already read past (a reader that runs indefinitely:
    /// reads use non-decreasing times, so they never look back).
    pub fn forget_read(&mut self) {
        self.events.drain(..self.cursor);
        self.cursor = 0;
    }

    /// The latch value just before `t` (latest write strictly before it).
    /// Calls must use non-decreasing t.
    pub fn read_before(&mut self, t: f64) -> u8 {
        while self.cursor < self.events.len() && self.events[self.cursor].0 < t {
            self.current = self.events[self.cursor].1;
            self.cursor += 1;
        }
        self.current
    }
}

/// The samples CPU and its two chips, rendered one interrupt at a time.
pub struct SamplesSound {
    rom: Vec<u8>,
    cpu: SamplesCpu,
    msm: [Msm5205; 2],
}

impl SamplesSound {
    /// Power-on state with the `audio2` region.
    pub fn new(audio2: Vec<u8>) -> Self {
        let mut msm = [Msm5205::new(), Msm5205::new()];
        let cpu = SamplesCpu::power_on(&mut |w: PortWrite| {
            if w.port < 2 {
                msm[w.port as usize].write(w.data);
            }
        });
        SamplesSound {
            rom: audio2,
            cpu,
            msm,
        }
    }

    /// Interrupts taken so far.
    pub fn ticks(&self) -> u64 {
        self.cpu.ticks()
    }

    /// One chip's DAC output.
    pub fn chip_output(&self, chip: usize) -> f32 {
        self.msm[chip].output()
    }

    /// Take the next interrupt, reading the latch from `latch`, feeding the
    /// chips, and returning the mixed output after it (both chips, gain 1).
    /// Port writes are also passed to `log`.
    pub fn tick(&mut self, latch: &mut LatchTimeline, log: &mut impl FnMut(PortWrite)) -> f32 {
        let rom = SampleRom(&self.rom);
        let msm = &mut self.msm;
        // What the poll's `in` reads at machine time t: the pinned MAME
        // runs the 68000's whole scheduler slice before the samples CPU's,
        // so a latch write later in the same slice is already visible. The
        // slices are [`SCHEDULER_QUANTUM`] long, counted from the last timer
        // expiry — the 8 kHz interrupt's tick, or a frame boundary inside
        // it (docs/reference/fight-samples.md). And because a Z80 step
        // starting at t finishes one Z80 T-state later (`t + 1 / CLOCK_HZ`),
        // a 68000 write on the same Z80 T-state is also visible when the
        // slice ends inside that T-state (tick 538801 of sf-win-01), while a
        // poll finishing 4.78 ns past a slice boundary at the 0.2 s rational
        // alignment (tick 1758401 of sf-full-2p-01) still executes inside the
        // slice ending at ceil(slice_end * CLOCK_HZ).
        let mut read = |t: f64| {
            let tick_start = (t * TICK_HZ as f64).floor() / TICK_HZ as f64;
            let frame_start = (t * FRAME_HZ as f64).floor() / FRAME_HZ as f64;
            let anchor = tick_start.max(frame_start);
            let slice_end = anchor
                + SCHEDULER_QUANTUM * (((t - 10.0e-9 - anchor) / SCHEDULER_QUANTUM).floor() + 1.0);
            latch.read_before(slice_end.max(t + 1.0 / samples::CLOCK_HZ))
        };
        self.cpu.interrupt(&rom, &mut read, &mut |w: PortWrite| {
            if w.port < 2 {
                msm[w.port as usize].write(w.data);
            }
            log(w);
        });
        self.msm[0].output() + self.msm[1].output()
    }
}

/// Write mono samples as a 16-bit WAV, holding each value for `hold`
/// output samples (zero-order hold, as the chips' DAC does).
pub fn write_wav(
    path: &std::path::Path,
    samples: &[f32],
    rate: u32,
    hold: usize,
) -> std::io::Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(std::io::Error::other)?;
    for s in samples {
        let v = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
        for _ in 0..hold {
            w.write_sample(v).map_err(std::io::Error::other)?;
        }
    }
    w.finalize().map_err(std::io::Error::other)
}
