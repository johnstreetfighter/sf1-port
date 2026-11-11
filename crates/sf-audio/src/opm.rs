//! The YM2151 (OPM) sound chip (ticket #11), synthesized by ymfm.
//!
//! Provenance: ymfm (Aaron Giles, BSD-3-Clause) through the `ymfm-sys`
//! 0.2.0 crate (h1romas4/ymfm-sys, BSD-3-Clause), which vendors the ymfm
//! sources and wraps the per-chip interface. The pinned comparison baseline
//! (MAME 0.289) emulates the YM2151 with the same library, so the model is
//! the reference's own chip core; `sf-audio compare-opm` checks the outputs
//! against the reference's hooked device stream.
//!
//! Configuration (sf.cpp): 3.579545 MHz clock; output rate clock / 64
//! (55930.4 Hz, two outputs); the IRQ line drives the music CPU's maskable
//! interrupt. ymfm leaves timers to the host: [`Opm`] keeps the two timer
//! deadlines in chip clocks of real time, as MAME's glue does (its timers
//! run on the 3.579545 MHz clock while its stream runs at the integer
//! 55930 Hz rate, so sample n is at n / 55930 s and the clock there is
//! floor(n * 3579545 / 55930)). A timer the chip re-arms on expiry
//! continues from its previous deadline, so periods stay exact; a write
//! made at a known time (a replayed reference write) is stamped with that
//! time ([`Opm::at_time`]). The chip core learns of an expiry at the next
//! output-sample boundary (~18 us), but the timer flags and the IRQ line
//! the music CPU sees are kept here at exact clocks: a flag is set at its
//! deadline and cleared by a reset write only if the deadline had passed
//! by the write's time (MAME's timers fire at the exact clock; a reset
//! written a few clocks before the deadline leaves the new flag standing,
//! ticket #33).

use std::cell::RefCell;
use std::rc::Rc;

use ymfm_sys::ffi::{self, ChipType};
use ymfm_sys::{InterfaceCallbacks, InterfaceHandler};

/// sf.cpp: `YM2151(config, "ymsnd", 3.579545_MHz_XTAL)`.
pub const OPM_CLOCK: u32 = 3_579_545;
/// Chip clocks per output sample.
pub const CLOCKS_PER_SAMPLE: u64 = 64;
/// Output sample rate as ymfm reports it (integer division, 55930 Hz):
/// the rate of MAME's stream for the chip, and the rate this model's
/// samples are placed at in time.
pub const OPM_RATE: u32 = OPM_CLOCK / CLOCKS_PER_SAMPLE as u32;

#[derive(Default)]
struct Timers {
    /// Output samples generated.
    samples: u64,
    /// Chip clocks of real time at the current point.
    now: i64,
    /// Deadline of each timer (None = stopped).
    deadline: [Option<i64>; 2],
    /// Set while a timer's expiry is being signalled: the chip's re-arm
    /// continues from this deadline.
    rearm_from: [Option<i64>; 2],
    /// The chip reports busy until this clock (64 clocks after a data write).
    busy_until: i64,
    /// The exact deadline from which each timer's flag stands: the earliest
    /// expiry not yet reset (a flag already standing keeps its time when a
    /// later deadline is processed); 0 = none yet.
    expired_at: [i64; 2],
    /// The latest deadline the core has processed (at a sample boundary,
    /// possibly ahead of the current access time): after a reset written
    /// before it, the flag stands again from it.
    last_expiry: [i64; 2],
    /// The time of the last write resetting each timer's flag (register
    /// 0x14 bits 4 / 5); the flag is clear while `reset_at >= expired_at`.
    reset_at: [i64; 2],
    /// Register 0x14 bits 2 / 3: the timer raises its flag and the IRQ.
    enabled: [bool; 2],
    /// The register address last written (port 0).
    addr: u8,
}

impl Timers {
    /// Timer `i`'s flag stands (set at `expired_at`, not reset since).
    fn standing(&self, i: usize) -> bool {
        self.expired_at[i] != 0 && self.reset_at[i] < self.expired_at[i]
    }

    /// Timer `i`'s flag at clock `t`: set from its expiry deadline on unless
    /// a reset write came at or after that deadline.
    fn flag_at(&self, i: usize, t: i64) -> bool {
        self.standing(i) && self.expired_at[i] <= t
    }

    fn irq_at(&self, t: i64) -> bool {
        (0..2).any(|i| self.enabled[i] && self.flag_at(i, t))
    }

    /// When the IRQ line rose: the earliest standing flag's deadline.
    fn irq_since(&self) -> i64 {
        (0..2)
            .filter(|&i| {
                self.enabled[i] && self.expired_at[i] != 0 && self.reset_at[i] < self.expired_at[i]
            })
            .map(|i| self.expired_at[i])
            .min()
            .unwrap_or(i64::MAX)
    }
}

/// One YM2151 with its host-side timers and IRQ line.
pub struct Opm {
    chip: cxx::UniquePtr<ffi::Chip>,
    timers: Rc<RefCell<Timers>>,
}

impl Default for Opm {
    fn default() -> Self {
        Self::new()
    }
}

impl Opm {
    pub fn new() -> Self {
        let timers = Rc::new(RefCell::new(Timers::default()));
        let (t1, t2, t4, t5) = (
            timers.clone(),
            timers.clone(),
            timers.clone(),
            timers.clone(),
        );
        let handler = InterfaceHandler {
            ymfm_set_timer: Some(Box::new(move |tnum, duration| {
                let mut t = t1.borrow_mut();
                let i = tnum as usize;
                t.deadline[i] = if duration < 0 {
                    None
                } else {
                    let base = t.rearm_from[i].unwrap_or(t.now);
                    Some(base + duration as i64)
                };
            })),
            advance_clock: Some(Box::new(move |_clocks| {
                let mut t = t2.borrow_mut();
                t.samples += 1;
                t.now = (t.samples * OPM_CLOCK as u64 / OPM_RATE as u64) as i64;
                let mut expired = 0u8;
                for i in 0..2 {
                    if let Some(d) = t.deadline[i] {
                        if d <= t.now {
                            // The core is told (it re-arms the timer from
                            // this deadline); the flag is kept here.
                            expired |= 1 << i;
                            t.deadline[i] = None;
                            t.rearm_from[i] = Some(d);
                            if t.enabled[i] {
                                t.last_expiry[i] = d;
                                if !t.standing(i) {
                                    t.expired_at[i] = d;
                                }
                            }
                        }
                    }
                }
                expired
            })),
            ymfm_set_busy_end: Some(Box::new(move |clocks| {
                let mut t = t4.borrow_mut();
                t.busy_until = t.now + clocks as i64;
            })),
            ymfm_is_busy: Some(Box::new(move || {
                let t = t5.borrow();
                t.now < t.busy_until
            })),
            ..Default::default()
        };
        let chip = ffi::create_chip_with_callbacks(
            ChipType::Ym2151,
            OPM_CLOCK,
            Box::new(InterfaceCallbacks::new(handler)),
        );
        Opm { chip, timers }
    }

    /// Stamp the following writes with time `t` (seconds), so a timer they
    /// load starts there (the time must lie within the current sample).
    pub fn at_time(&mut self, t: f64) {
        self.timers.borrow_mut().now = (t * OPM_CLOCK as f64).floor() as i64;
    }

    /// Stamp the following accesses with chip clock `t`.
    pub fn at_clock(&mut self, t: u64) {
        self.timers.borrow_mut().now = t as i64;
    }

    /// Output samples generated so far.
    pub fn samples(&self) -> u64 {
        self.timers.borrow().samples
    }

    /// Write the address (offset 0) or data (offset 1) port, at the time
    /// last stamped ([`Opm::at_clock`] / [`Opm::at_time`]).
    pub fn write(&mut self, offset: u8, data: u8) {
        {
            let mut t = self.timers.borrow_mut();
            if offset & 1 == 0 {
                t.addr = data;
            } else if t.addr == 0x14 {
                // Timer control: the flags reset now (a deadline still
                // ahead sets its flag later), the IRQ enables.
                let now = t.now;
                for i in 0..2 {
                    if data & (0x10 << i) != 0 {
                        t.reset_at[i] = now;
                        // A deadline the core processed early but which lies
                        // after this write sets the flag again from itself.
                        if t.last_expiry[i] > now {
                            t.expired_at[i] = t.last_expiry[i];
                        }
                    }
                    t.enabled[i] = data & (0x04 << i) != 0;
                }
            }
        }
        self.chip.pin_mut().write(offset as u32 & 1, data);
    }

    /// Write one register (address then data).
    pub fn write_reg(&mut self, reg: u8, data: u8) {
        self.write(0, reg);
        self.write(1, data);
    }

    /// Read the status port at the time last stamped (bit 0 timer A, bit 1
    /// timer B, bit 7 busy); the timer flags are this model's exact-time
    /// ones, not the core's sample-resolution ones.
    pub fn status(&mut self) -> u8 {
        let st = self.chip.pin_mut().read(1) & !0x03;
        let tm = self.timers.borrow();
        st | (0..2)
            .filter(|&i| tm.flag_at(i, tm.now))
            .map(|i| 1 << i)
            .sum::<u8>()
    }

    /// The IRQ line at the time last stamped.
    pub fn irq(&self) -> bool {
        let tm = self.timers.borrow();
        tm.irq_at(tm.now)
    }

    /// The IRQ line at chip clock `t`, with expiries at their exact
    /// deadlines (the chip must have been advanced to `t`).
    pub fn irq_at(&self, t: u64) -> bool {
        self.timers.borrow().irq_at(t as i64)
    }

    /// When the IRQ line rose (chip clock): the earliest standing flag's
    /// deadline, i64::MAX when the line is low.
    pub fn irq_since(&self) -> i64 {
        self.timers.borrow().irq_since()
    }

    /// The timers' deadlines (chip clocks; None = stopped), for tracing.
    pub fn deadlines(&self) -> [Option<i64>; 2] {
        self.timers.borrow().deadline
    }

    /// The flag model's state per timer (expired_at, last_expiry, reset_at,
    /// enabled), for tracing.
    pub fn flag_state(&self) -> [(i64, i64, i64, bool); 2] {
        let t = self.timers.borrow();
        [
            (
                t.expired_at[0],
                t.last_expiry[0],
                t.reset_at[0],
                t.enabled[0],
            ),
            (
                t.expired_at[1],
                t.last_expiry[1],
                t.reset_at[1],
                t.enabled[1],
            ),
        ]
    }

    /// The status port at chip clock `t` (the chip must have been advanced
    /// to `t`).
    pub fn status_at(&mut self, t: u64) -> u8 {
        self.at_clock(t);
        self.status()
    }

    /// Generate one output sample (left, right; raw 16-bit-range values),
    /// advancing the timers by 64 clocks.
    pub fn sample(&mut self) -> [i32; 2] {
        let mut buf = [0i32; 2];
        self.chip.pin_mut().generate(&mut buf);
        let mut t = self.timers.borrow_mut();
        t.rearm_from = [None, None];
        buf
    }

    /// Chip clocks elapsed.
    pub fn clocks(&self) -> i64 {
        self.timers.borrow().now
    }
}
