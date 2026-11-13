//! Both sound CPUs and their chips from power-on, mixed as the pinned MAME
//! mixes them (ticket #37): the one implementation behind the measured mix
//! (`check::render_mix`, docs/reference/both-chips.md) and the launcher's
//! live output.
//!
//! sf.cpp routes the YM2151's two outputs to the speaker's two channels at
//! [`YM_ROUTE_GAIN`] and each MSM5205 to both channels at
//! [`MSM_ROUTE_GAIN`]; MAME's recording saturates at full scale. Here:
//!
//! - the samples CPU ([`SamplesSound`]) runs its 8 kHz interrupts; each
//!   MSM5205's output is a step function that changes when a written nibble
//!   is decoded, on the VCK edge [`MSM_DECODE_DELAY`] after the write
//!   (msm5205.rs);
//! - the music CPU runs from its reset vector with the YM2151 — the timed
//!   chip and idle loop of the reference comparisons — on its own thread,
//!   its 55930 Hz output linearly interpolated to the requested time;
//! - a frame's latch write (the engine's per-frame value, at the vblank
//!   handler's cycles plus an interrupt latency, latch-timing.md) reaches
//!   both CPUs: the samples CPU reads it, the music CPU takes it as an NMI;
//! - in overhaul fights with a voice pack, the announcer's own MSM5205
//!   ([`Announcer`], ADR 0011) is routed like the board's two at
//!   [`MSM_ROUTE_GAIN`]. It is a channel the cabinet never had: nothing it
//!   does reaches the samples CPU, its latch or its chips.
//!
//! **Frames pushed while the sound runs** (the launcher): a music-CPU
//! handler cannot stop halfway, and its chains run past a frame boundary
//! (up to 23.6 ms over frames 2389–4353 of the fixed fight, both-chips.md),
//! so the music CPU runs on a thread and
//! waits — inside the handler if need be — whenever it reaches a time the
//! writes pushed so far do not cover (a frame's write lies at or after the
//! frame's start). The samples CPU runs only to a millisecond before that
//! time (its handler is far shorter). The output is therefore the same as
//! a run with every write known from the start, by construction; the
//! both-chips tests check it sample for sample.
//!
//! MAME's band-limiting output resampler is not modelled: the mix is for
//! measurement and listening, while the per-device outputs are compared
//! exactly (`check::compare_dac`, `check::run_music_from_power_on`).

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use sf_video::announcer::AnnouncerLine;

use crate::announcer::Announcer;
use crate::check::{idle_loop, LatchFeed, LatchFrame, TimedChip};
use crate::impact::ImpactMixer;
use crate::music::{ExtensionCell, MusicCpu};
use crate::opm::{OPM_CLOCK, OPM_RATE};
use crate::samples::PortWrite;
use crate::score::NativeScore;
use crate::{LatchTimeline, SamplesSound, FRAME_HZ, TICK_HZ};

pub use crate::impact::ImpactAccent;

/// sf.cpp: `ymsnd.add_route(0, "speaker", 0.60, 0)` and `(1, ..., 1)`.
pub const YM_ROUTE_GAIN: f32 = 0.6;
/// sf.cpp: each MSM5205 `add_route(ALL_OUTPUTS, "speaker", 1.0, ...)` to
/// both channels.
pub const MSM_ROUTE_GAIN: f32 = 1.0;
/// The MSM5205 decodes a written nibble on the VCK edge 15.6 µs after the
/// write (VCK slave mode at 384 kHz: six clocks).
pub const MSM_DECODE_DELAY: f64 = 6.0 / 384_000.0;
/// The samples CPU runs to this long before the first unknown write: an
/// interrupt's latch read lies well within its 125 µs tick.
const SAMPLES_MARGIN: f64 = 0.001;
/// How far past the output the music thread may run (seconds).
const LOOKAHEAD: f64 = 0.25;
/// The music thread hands its output over at least this often (chip
/// clocks; 0.1 s), so a pre-roll never accumulates its discarded samples.
const HAND_OVER: u64 = OPM_CLOCK as u64 / 10;

/// What the host pushes to the music thread, in order: a frame's latch
/// write, or the extended tune table to run with from the next write on.
enum Push {
    Frame(LatchFrame),
    Tunes(Option<Arc<Vec<u8>>>),
}

/// The music thread's output and the consumer's demand.
struct YmOut {
    /// One past the last sample index handed over.
    end: u64,
    left: VecDeque<f32>,
    right: VecDeque<f32>,
    /// Samples before this index are not kept.
    keep_from: u64,
    /// The music CPU runs to this chip clock at most.
    target: u64,
    /// The thread is waiting for the next frame's write.
    starved: bool,
    stop: bool,
    unrecovered: Vec<u16>,
    /// The music CPU's program image carries the extended tune table now.
    has_extension: bool,
}

impl YmOut {
    fn base(&self) -> u64 {
        self.end - self.left.len() as u64
    }
}

struct Shared {
    out: Mutex<YmOut>,
    cv: Condvar,
}

/// The music thread's side of the latch: pushed frames over a channel.
struct ChannelFeed {
    rx: mpsc::Receiver<Push>,
    shared: Arc<Shared>,
    /// Where the music CPU looks for its extension: a change pushed between
    /// two frames takes effect as the second frame's write is handed over.
    tunes: ExtensionCell,
}

impl LatchFeed for ChannelFeed {
    fn flush(&mut self, first: u64, left: &mut Vec<f32>, right: &mut Vec<f32>) {
        hand_over(&self.shared, first, left, right);
    }

    fn next(&mut self) -> Option<LatchFrame> {
        loop {
            // The check and the flag under the lock `push_frame` sends under:
            // a consumer that sees `starved` knows no frame is on its way.
            {
                let mut o = self.shared.out.lock().unwrap();
                match self.rx.try_recv() {
                    Ok(Push::Frame(w)) => return Some(w),
                    Ok(Push::Tunes(table)) => {
                        *self.tunes.lock().unwrap() = table;
                        continue;
                    }
                    Err(_) => o.starved = true,
                }
            }
            self.shared.cv.notify_all();
            match self.rx.recv().ok()? {
                Push::Frame(w) => return Some(w),
                Push::Tunes(table) => *self.tunes.lock().unwrap() = table,
            }
        }
    }
}

fn hand_over(shared: &Shared, first: u64, left: &mut Vec<f32>, right: &mut Vec<f32>) {
    let mut o = shared.out.lock().unwrap();
    let n = left.len();
    let skip = (o.keep_from.saturating_sub(first) as usize).min(n);
    o.left.extend(left.drain(..).skip(skip));
    o.right.extend(right.drain(..).skip(skip));
    o.end = first + n as u64;
    drop(o);
    shared.cv.notify_all();
}

/// The music thread: the CPU from its reset vector, then the idle loop to
/// the consumer's demand, waiting for frames inside as need be.
fn music_thread(
    rom: Vec<u8>,
    latch: LatchTimeline,
    horizon: u64,
    rx: Option<mpsc::Receiver<Push>>,
    shared: Arc<Shared>,
    tunes: ExtensionCell,
) {
    let mut chip = TimedChip::new(latch, u64::MAX, false);
    if let Some(rx) = rx {
        chip.set_feed(
            Box::new(ChannelFeed {
                rx,
                shared: shared.clone(),
                tunes: tunes.clone(),
            }),
            horizon,
        );
    }
    let mut cpu = MusicCpu::new(rom, vec![0; 0x800]).with_extension_cell(tunes);
    cpu.boot(&mut chip);
    loop {
        let target = {
            let mut o = shared.out.lock().unwrap();
            loop {
                if o.stop {
                    return;
                }
                if o.target > cpu.t {
                    break o.target;
                }
                o = shared.cv.wait(o).unwrap();
            }
        };
        // In steps: the loop stops only at its top, so stopping more often
        // changes nothing but how often the output is handed over.
        let end = target.min(cpu.t + HAND_OVER);
        idle_loop(&mut cpu, &mut chip, end, u64::MAX);
        // The CPU is at or past `end`: every sample before it is final.
        chip.sync(end);
        let first = chip.opm.samples() - chip.out.len() as u64;
        hand_over(&shared, first, &mut chip.out, &mut chip.right);
        chip.out.shrink_to(1 << 16);
        chip.right.shrink_to(1 << 16);
        let mut o = shared.out.lock().unwrap();
        o.unrecovered = cpu.unrecovered.iter().copied().collect();
        o.has_extension = cpu.has_extension();
        drop(o);
        shared.cv.notify_all();
    }
}

/// Both translated sound programs and their chips, from power-on.
pub struct GameSound {
    samples: SamplesSound,
    /// The samples CPU's view of the latch (its own read cursor).
    samples_latch: LatchTimeline,
    /// Per MSM5205: output steps (time, level) not yet reached, and the
    /// level reached.
    steps: [VecDeque<(f64, f32)>; 2],
    level: [f32; 2],
    /// The announcer's chip, when a voice pack is loaded.
    announcer: Option<Announcer>,
    impacts: ImpactMixer,
    /// The extended tune table the music CPU runs with (ticket #199).
    tunes: ExtensionCell,
    /// The samples CPU's port writes, kept when asked for (a check's).
    port_log: Option<Vec<PortWrite>>,
    /// Every latch write before this time is known (seconds).
    known_until: f64,
    tx: Option<mpsc::Sender<Push>>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl GameSound {
    /// Both CPUs at power-on, with the latch writes `latch`, complete
    /// before `known_until` (seconds; `f64::INFINITY` when every write is
    /// in it, otherwise later ones come through [`GameSound::push_frame`]).
    pub fn power_on(
        audio2: Vec<u8>,
        audiocpu: Vec<u8>,
        latch: LatchTimeline,
        known_until: f64,
    ) -> Self {
        let shared = Arc::new(Shared {
            out: Mutex::new(YmOut {
                end: 0,
                left: VecDeque::new(),
                right: VecDeque::new(),
                keep_from: 0,
                target: 0,
                starved: false,
                stop: false,
                unrecovered: Vec::new(),
                has_extension: false,
            }),
            cv: Condvar::new(),
        });
        let (tx, rx) = if known_until.is_finite() {
            let (tx, rx) = mpsc::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };
        let horizon = if known_until.is_finite() {
            (known_until * OPM_CLOCK as f64).floor() as u64
        } else {
            u64::MAX
        };
        let (thread_latch, thread_shared) = (latch.clone(), shared.clone());
        let tunes = ExtensionCell::default();
        let thread_tunes = tunes.clone();
        let thread = std::thread::Builder::new()
            .name("sf-audio music CPU".into())
            .spawn(move || music_thread(audiocpu, thread_latch, horizon, rx, thread_shared, thread_tunes))
            .expect("spawn the music CPU thread");
        GameSound {
            samples: SamplesSound::new(audio2),
            samples_latch: latch,
            steps: [VecDeque::new(), VecDeque::new()],
            level: [0.0; 2],
            announcer: None,
            impacts: ImpactMixer::default(),
            tunes,
            port_log: None,
            known_until,
            tx,
            shared,
            thread: Some(thread),
        }
    }

    /// Append a frame's latch write (`latency` from
    /// [`crate::IRQ_LATENCY_CYCLES`]); frames in order, each after the
    /// time already known.
    pub fn push_frame(&mut self, frame: u32, value: u8, cycles: u32, latency: u32) {
        self.push_frame_with_score(frame, value, cycles, latency, None);
    }

    /// Append the latch write and the exact score selected for this frame.
    /// The score is pinned through the music queue and both driver slots.
    pub fn push_frame_with_score(
        &mut self,
        frame: u32,
        value: u8,
        cycles: u32,
        latency: u32,
        score: Option<Arc<NativeScore>>,
    ) {
        debug_assert!(
            score.is_none()
                || (crate::music::EXTENDED_FIRST..=crate::music::EXTENDED_LAST).contains(&value)
        );
        let Some(tx) = &self.tx else {
            panic!("push_frame on a sound whose writes were all given at power-on");
        };
        debug_assert!(
            frame as f64 / FRAME_HZ as f64 >= self.known_until - 1e-9,
            "frame {frame} pushed out of order"
        );
        self.samples_latch.push_frame(frame, value, cycles, latency);
        {
            let mut o = self.shared.out.lock().unwrap();
            // A closed channel means the thread has stopped; nothing to do.
            let _ = tx.send(Push::Frame(LatchFrame {
                frame,
                value,
                cycles,
                latency,
                score,
            }));
            o.starved = false;
        }
        // The samples CPU's reads never look back.
        self.samples_latch.forget_read();
        self.known_until = (frame + 1) as f64 / FRAME_HZ as f64;
    }

    /// Give the music CPU the extended tune table `table` (the bytes of
    /// [`crate::tunes::ExtendedTunes::image`]), or take it away (`None`: the
    /// ROM alone), from the frames pushed after this call: it is pushed in
    /// order with them, so the CPU has it for the command of the next frame
    /// whatever it is doing now. A table in use is kept until the tune
    /// playing from it is over ([`crate::music`]). With every write given
    /// at power-on it takes effect at once.
    pub fn set_tune_table(&self, table: Option<Arc<Vec<u8>>>) {
        match &self.tx {
            Some(tx) => {
                let _ = tx.send(Push::Tunes(table));
            }
            None => *self.tunes.lock().unwrap() = table,
        }
    }

    /// Whether the music CPU runs with the extended tune table now, as of
    /// the last time it handed output over (for tests and diagnostics).
    pub fn has_tune_table(&self) -> bool {
        self.shared.out.lock().unwrap().has_extension
    }

    /// Give the mix the announcer's chip (or take it away). His lines are
    /// asked for with [`GameSound::announce`].
    pub fn set_announcer(&mut self, announcer: Option<Announcer>) {
        self.announcer = announcer;
    }

    pub fn announcer(&self) -> Option<&Announcer> {
        self.announcer.as_ref()
    }

    pub fn announcer_mut(&mut self) -> Option<&mut Announcer> {
        self.announcer.as_mut()
    }

    /// Ask the announcer for `lines` at frame `frame`'s start (the frame
    /// whose latch is pushed with it); nothing without his chip.
    pub fn announce(&mut self, frame: u32, lines: &[AnnouncerLine]) {
        if let Some(a) = self.announcer.as_mut() {
            a.call_at_frame(frame, lines);
        }
    }

    /// Play an accent at the start of a presented sound frame, independently
    /// of the sound latch and announcer.
    pub fn play_impact(&mut self, frame: u32, accent: ImpactAccent) {
        self.impacts.play(frame, accent);
    }

    /// End impacts at a scene boundary, including any that are still queued.
    /// Output before this frame remains audible if playback is behind.
    pub fn clear_impacts(&mut self, frame: u32) {
        self.impacts.clear_at(frame);
    }

    /// Keep the samples CPU's port writes from now on, for
    /// [`GameSound::take_port_writes`].
    pub fn record_port_writes(&mut self) {
        self.port_log.get_or_insert_with(Vec::new);
    }

    /// The port writes kept since the last call.
    pub fn take_port_writes(&mut self) -> Vec<PortWrite> {
        self.port_log.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// The board's two MSM5205s' levels as last mixed.
    pub fn chip_levels(&self) -> [f32; 2] {
        self.level
    }

    fn samples_ready(&self) -> f64 {
        self.known_until - SAMPLES_MARGIN
    }

    fn ym_ready(o: &YmOut) -> f64 {
        o.end.saturating_sub(2) as f64 / OPM_RATE as f64
    }

    /// Ask the music thread to run to `t` plus the lookahead.
    fn demand(&self, o: &mut YmOut, t: f64) {
        let target = ((t + LOOKAHEAD) * OPM_CLOCK as f64).ceil() as u64;
        if target > o.target {
            o.target = target;
            self.shared.cv.notify_all();
        }
    }

    /// The latest time [`GameSound::sample_at`] can serve now (without
    /// waiting); asks the music thread to run on from `t`.
    pub fn ready_until(&self, t: f64) -> f64 {
        let mut o = self.shared.out.lock().unwrap();
        self.demand(&mut o, t);
        Self::ym_ready(&o).min(self.samples_ready())
    }

    /// Wait until output through `t` can be served. False when it cannot
    /// be without more frames (the music thread waits for one).
    pub fn wait_ready(&mut self, t: f64) -> bool {
        if t > self.samples_ready() {
            return false;
        }
        let mut o = self.shared.out.lock().unwrap();
        self.demand(&mut o, t);
        loop {
            if Self::ym_ready(&o) >= t {
                return true;
            }
            if o.starved {
                return false;
            }
            o = self.shared.cv.wait(o).unwrap();
        }
    }

    fn run_samples_to(&mut self, t: f64) {
        while (self.samples.ticks() + 1) as f64 / TICK_HZ as f64 <= t {
            let mut written: [Option<f64>; 2] = [None, None];
            let log = &mut self.port_log;
            self.samples
                .tick(&mut self.samples_latch, &mut |w: PortWrite| {
                    if w.port < 2 {
                        written[w.port as usize] = Some(w.t);
                    }
                    if let Some(log) = log.as_mut() {
                        log.push(w);
                    }
                });
            for (chip, wt) in written.into_iter().enumerate() {
                if let Some(wt) = wt {
                    let v = self.samples.chip_output(chip);
                    let last = self.steps[chip]
                        .back()
                        .map(|s| s.1)
                        .unwrap_or(self.level[chip]);
                    if v != last {
                        self.steps[chip].push_back((wt + MSM_DECODE_DELAY, v));
                    }
                }
            }
        }
    }

    fn msm_at(&mut self, t: f64) -> f32 {
        let mut msm = 0.0f32;
        for chip in 0..2 {
            while let Some(&(st, v)) = self.steps[chip].front() {
                if st > t {
                    break;
                }
                self.level[chip] = v;
                self.steps[chip].pop_front();
            }
            msm += MSM_ROUTE_GAIN * self.level[chip];
        }
        msm
    }

    /// The mixed output (left, right) at time `t` — non-decreasing, at most
    /// [`GameSound::ready_until`] — saturated at full scale. Output before
    /// `t` is discarded.
    pub fn sample_at(&mut self, t: f64) -> [f32; 2] {
        self.sample_at_gains(t, 1.0, 1.0)
    }

    /// [`Self::sample_at`] with the YM2151's route scaled by `fm` and both
    /// MSM5205s and the optional impact accents by `samples` (the front
    /// end's `FM` and `SAMPLES` volumes;
    /// at 1.0 each the output is [`Self::sample_at`]'s exactly).
    pub fn sample_at_gains(&mut self, t: f64, fm: f32, samples: f32) -> [f32; 2] {
        self.sample_at_mix(t, fm, samples, 1.0)
    }

    /// [`Self::sample_at_gains`] with the announcer's chip scaled by
    /// `announcer` (the `ANNOUNCER` volume; 0.0 leaves the board's mix
    /// exactly as it is without him).
    pub fn sample_at_mix(&mut self, t: f64, fm: f32, samples: f32, announcer: f32) -> [f32; 2] {
        debug_assert!(
            t <= self.samples_ready(),
            "output at {t} s beyond the known writes"
        );
        self.run_samples_to(t);
        let mut msm = self.msm_at(t) * samples;
        if let Some(impact) = self.impacts.sample_at(t) {
            msm += impact * samples;
        }
        if let Some(a) = self.announcer.as_mut() {
            // Run even when muted, so turning him up again finds him on time.
            let level = a.output_at(t);
            if announcer != 0.0 {
                msm += MSM_ROUTE_GAIN * level * announcer;
            }
        }
        let x = t * OPM_RATE as f64;
        let mut o = self.shared.out.lock().unwrap();
        let k = (x.floor() as u64).max(o.base());
        while o.base() < k && !o.left.is_empty() {
            o.left.pop_front();
            o.right.pop_front();
        }
        let f = (x - k as f64).clamp(0.0, 1.0) as f32;
        let at = |v: &VecDeque<f32>, i: usize| v.get(i).or(v.back()).copied().unwrap_or(0.0);
        let (l0, l1, r0, r1) = (
            at(&o.left, 0),
            at(&o.left, 1),
            at(&o.right, 0),
            at(&o.right, 1),
        );
        self.demand(&mut o, t);
        drop(o);
        let (l, r) = (l0 + (l1 - l0) * f, r0 + (r1 - r0) * f);
        [
            (msm + YM_ROUTE_GAIN * l * fm).clamp(-1.0, 1.0),
            (msm + YM_ROUTE_GAIN * r * fm).clamp(-1.0, 1.0),
        ]
    }

    /// Run to `t` without output (a pre-roll to a fixture frame), waiting
    /// for the music thread; the output before `t` is discarded. False
    /// when `t` lies beyond the writes known.
    pub fn skip_to(&mut self, t: f64) -> bool {
        if t > self.samples_ready() {
            return false;
        }
        {
            let mut o = self.shared.out.lock().unwrap();
            o.keep_from = o.keep_from.max((t * OPM_RATE as f64).floor() as u64);
        }
        self.run_samples_to(t);
        self.msm_at(t);
        if !self.wait_ready(t) {
            return false;
        }
        self.sample_at(t);
        true
    }

    /// The YM2151's own output (left, right; 1/32768 scale) held now, from
    /// the sample index returned: for comparing the live player's chip with
    /// the batch runner's.
    pub fn ym_samples(&self) -> (u64, Vec<f32>, Vec<f32>) {
        let o = self.shared.out.lock().unwrap();
        (
            o.base(),
            o.left.iter().copied().collect(),
            o.right.iter().copied().collect(),
        )
    }

    /// Untranslated music-CPU branches reached so far.
    pub fn unrecovered(&self) -> Vec<u16> {
        self.shared.out.lock().unwrap().unrecovered.clone()
    }
}

impl Drop for GameSound {
    fn drop(&mut self) {
        self.shared.out.lock().unwrap().stop = true;
        self.shared.cv.notify_all();
        // Closing the channel releases a thread waiting for a frame.
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
