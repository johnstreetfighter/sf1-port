//! The music CPU's program (ticket #11), translated with its timing.
//!
//! The first Z80 (`audiocpu`, sf-02.7k; RAM 0xc000-0xc7ff; 3.579545 MHz,
//! the YM2151's clock) drives the YM2151. Its main loop only idles (0x0009
//! `ei` / 0x000a `jr 0x0009`); all work happens in the maskable interrupt
//! the YM2151's timers raise (0x0038):
//!
//! - **Timer A** (0x356 = 856: 168 x 64 clocks, 3.0 ms): re-arm (0x00ca),
//!   the two sound-effect players (0x0a9b, slots 0xc500 / 0xc580), and the
//!   music sequencer (0x0424).
//! - **Timer B** (0xf2: 4.0 ms): re-arm (0x00e2) and the command intake
//!   (0x019d), which polls the sound latch (0xc800).
//! - The NMI the main CPU pulses with each latch write (0x0066) copies the
//!   latch to 0xc000.
//!
//! **Timing is part of the behavior.** The handler runs with interrupts
//! disabled; loading patches for several channels takes longer than timer
//! A's period, so later expiries wait and two of them can coalesce into one
//! pending flag — the original loses those sequencer ticks (14 in the
//! bounded phrase). The translation therefore keeps the CPU's clock: every
//! translated block charges the T-states of the original instructions it
//! stands for (taken / not-taken costs on conditional branches), chip
//! accesses happen at their instruction's memory cycle, the write routine
//! (0x00fc) polls the chip's busy flag as the original does, and NMIs are
//! taken between blocks (72 T-states). One T-state is one chip clock.
//!
//! Music state: eight channels of 0x30 bytes at 0xc100 (and a saved copy
//! at 0xc300 while a higher-priority tune plays): +0/+1 stream pointer
//! (big-endian), +2/+3 duration counter, +4 length unit, +6/+7 opcode 1's
//! parameter and a counter, +8/+9 gate counter, +0xa gate percentage, +0xb
//! flags (bit 7 double speed, bit 0 opcode 5), +0xc tie count, +0xd
//! transpose, +0xe current note, +0x12 opcode 4's parameter, +0x17 patch
//! number, +0x18/+0x19 patch pointer (0x6b00 + 0x29 x patch), +0x1a the
//! RL/FB/CON copy, +0x2d..+0x2f repeat counters. Globals: 0xc001 last
//! command, 0xc002 key code, 0xc010 "silent" (the saved tune advancing),
//! 0xc011 / 0xc012 current / saved tune header, 0xc013 tick count, 0xc014
//! channel being processed, 0xc015..0xc018 LFO registers, 0xc023
//! per-channel "an effect owns this channel" bits (bit 7 = current).
//!
//! Translated: the interrupt dispatch, command intake for music commands
//! and the idle latch, channel loading and silencing, the sequencer with
//! its note, rest, tie, double-speed, and opcode 0x0-0x9 / 0xb-0xf paths,
//! durations and gates, key-on with the operator-level computation, patch
//! loading, and (ticket #33) the special commands — the stops of the music
//! (0x03a4: commands 0x35 / 0x3d / 0x3f) and of the effect slots (0x02bb /
//! 0x02f3, which hand their channels back to the music, 0x032b), the saved
//! tune's pop and discard (0x03a7 / 0x0412) — and opcode 0xf, the end of a
//! tune; (ticket #34) the sound-effect start (0x0240) and the effects
//! player (0x0a9b: slots at 0xc500 / 0xc580 with +0 mask, +1/+2 stream,
//! +3/+4 countdown, +6/+7 noise, +0xf loop counter, 8-byte channel records
//! from +0x10 with channel 7 first; the player's channel in 0xc022, the
//! slot priorities in 0xc020 / 0xc021, 0xc024 while an effect ends by
//! itself). Other branches (opcode 0xa, opcode 0xf inside a nested event
//! read, effect opcode bytes beyond 5) record their address in
//! [`MusicCpu::unrecovered`] instead of being guessed.
//!
//! **The extended tune table** (ticket #195, ADR-0009) is an overhaul-only
//! extension of the program image: a program longer than the ROM's 0x8000
//! bytes carries more at 0x8000.. (which the real board leaves unmapped),
//! and commands 0x40..=0x7f — which the 1987 game never sends and whose
//! entries in the ROM's table are tune data read as addresses — are looked
//! up in a table there instead ([`crate::tunes`]). The tunes it names are
//! played, saved, stopped and handed back by the same translated code as
//! the ROM's. A ROM-length image never takes that path.
//!
//! **Handing the table over** (ticket #199, ADR-0009): the music CPU is one
//! object for a whole session while the ruleset can change between fights,
//! so the host gives it the table through an [`ExtensionCell`] only for a
//! fight under an overhaul preset, and takes it away after. The CPU looks at
//! the cell where a command is taken (0x019d), after the latch read
//! and before the table lookup, and a change reaches it before the command of any frame pushed
//! after the change. A table in use is never pulled from under a tune: a
//! take-away waits until neither the current nor the saved tune has a
//! stream in the extension.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

/// The first address past the ROM, where the extended tune table starts
/// when the program image is longer than the ROM.
pub const EXTENSION_BASE: u16 = 0x8000;
/// The first address the extension cannot reach: RAM starts here.
pub const EXTENSION_END: u16 = 0xc000;
/// What the host wants the music CPU's extension to be: the extended tune
/// table's bytes (what [`crate::tunes::ExtendedTunes::image`] holds), or
/// none, the ROM alone.
pub type ExtensionCell = Arc<Mutex<Option<Arc<Vec<u8>>>>>;

/// The first command looked up in the extended tune table.
pub const EXTENDED_FIRST: u8 = 0x40;
/// The last command looked up in the extended tune table.
pub const EXTENDED_LAST: u8 = 0x7f;

/// The music CPU's view of the rest of the machine, all at chip clock `t`.
pub trait MusicBus {
    /// Read the YM2151 status port (0xe001).
    fn status(&mut self, t: u64) -> u8;
    /// Write the YM2151 address (port 0, 0xe000) or data (port 1) port.
    fn write(&mut self, t: u64, port: u8, data: u8);
    /// Read the sound latch (0xc800).
    fn latch(&mut self, t: u64) -> u8;
    /// Take the next NMI if one is due at or before `t` (returns its time).
    fn take_nmi(&mut self, t: u64) -> Option<u64>;
}

const C000_NMI_COPY: u16 = 0xc000;
const C001_LAST_CMD: u16 = 0xc001;
const C002_KEYCODE: u16 = 0xc002;
const C010_SILENT: u16 = 0xc010;
const C011_MUSIC: u16 = 0xc011;
const C012_SAVED: u16 = 0xc012;
const C013_TICKS: u16 = 0xc013;
const C014_CHANNEL: u16 = 0xc014;
/// 0xc015-0xc018: the LFO registers' last values (0x1b, 0x18, 0x19, 0x19).
const C015_LFO: u16 = 0xc015;
/// Effect slot A / B priorities (cleared only by the slot stops).
const C020_EFFECT_A_PRIORITY: u16 = 0xc020;
const C021_EFFECT_B_PRIORITY: u16 = 0xc021;
/// The effects player's channel being processed (7 down to 0), the
/// player's counterpart of 0xc014.
const C022_EFFECT_CHANNEL: u16 = 0xc022;
const C023_OWNED: u16 = 0xc023;
/// Non-zero only while an effect ends by itself (0x0f7d): 0x032b then
/// leaves a free channel's levels alone.
const C024_EFFECT_ENDING: u16 = 0xc024;
const MUSIC_CHANNELS: u16 = 0xc100;
const SAVED_CHANNELS: u16 = 0xc300;
const EFFECT_SLOT_A: u16 = 0xc500;
const EFFECT_SLOT_B: u16 = 0xc580;

// Extension-only 0x0a state, in unused bytes of each copied 0x30-byte
// channel record. Keeping it here makes the native saved-slot copy own it.
const RAW_FLAG: u16 = 0x0f;
const RAW_MAGIC: u8 = 0xa7;
const RAW_KC: u16 = 0x10;
const RAW_KF: u16 = 0x11;
const RAW_CARRIER: u16 = 0x13;
const RAW_MODULATOR: u16 = 0x14;
const RAW_FEEDBACK: u16 = 0x15;
const RAW_GATE_ARMED: u16 = 0x16;
const RAW_WAIT: u16 = 0x1b;
const RAW_GATE: u16 = 0x1f;
const RAW_CLOCK: u16 = 0x23;
const RAW_OVERDUE: u16 = 0x27;
/// One explicit-duration unit is 256 YM2151 clocks, about 71.52 us.
pub const RAW_DURATION_CLOCKS: u32 = 256;

/// The operator-level computation exists twice in the ROM: the music's
/// (0x059b: the channel from 0xc014, writes gated by 0xc023 through 0x09f8,
/// the curve and product tables at 0x0675 / 0x0701) and the effects
/// player's copy (0x0bfc: the channel from 0xc022, direct writes through
/// 0x00fc, the same tables copied to 0x0cd6 / 0x0d62 — addresses that
/// change the `rst 0x20` page-carry costs).
#[derive(Debug, Clone, Copy)]
struct Levels {
    channel: u16,
    curve: u16,
    product: u16,
    direct: bool,
}

const MUSIC_LEVELS: Levels = Levels {
    channel: C014_CHANNEL,
    curve: 0x0675,
    product: 0x0701,
    direct: false,
};
const EFFECT_LEVELS: Levels = Levels {
    channel: C022_EFFECT_CHANNEL,
    curve: 0x0cd6,
    product: 0x0d62,
    direct: true,
};

/// How an event read, a channel's tick or a tick ends: normally, or unwound
/// by opcode 0xf — the audible tune ended (0x03a7 next: the saved tune is
/// popped) or the silently advancing saved tune ended (0x0412 next).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Done,
    TuneEnd,
    SavedEnd,
}

/// T-states from an instruction's start to the start of its memory cycle:
/// a 13-T-state absolute load/store (`ld a,(nn)` / `ld (nn),a`: fetch 4,
/// two operand reads 6, the access in the last 3) and a 7-T-state indirect
/// access (`ld a,(bc)` / `ld (bc),a`: fetch 4, the access in the last 3).
/// The pinned MAME stamps a bus access at its cycle's start; the music
/// stops' windows (ticket #33) showed the value matters beyond the fitted
/// idle phase — with 5 here, handlers the reference accepts within one
/// clock of a timer's deadline came one idle-loop lap late.
const ABS_ACCESS: u64 = 10;
const IND_ACCESS: u64 = 4;

/// Reached branches that reference RAM alone cannot distinguish.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MusicObservations {
    /// Music writes/key-offs skipped because an effect owns the channel.
    pub owned_skips: u64,
    /// Effect slide steps whose signed step is zero.
    pub zero_slides: u64,
    /// Bitset of latch commands active when a zero slide was reached.
    pub zero_slide_commands: u64,
    /// Repeated non-0xff commands ignored at 0x01a4 ( == 0xc001).
    pub repeated_cmd_ignores: u64,
    /// Tune started at equal priority (0x01cf: cur == c).
    pub tune_equal_priority: u64,
    /// Tune started at higher priority, saving 0xc100 to 0xc300 (0x01e5: cur < c).
    pub tune_higher_priority_saves: u64,
    /// Lower-priority tune loaded into the saved slot 0xc300 (0x01d3: cur > c).
    pub tune_lower_priority_saved_loads: u64,
    /// Saved tune popped from 0xc300 to 0xc100 (0x03a7).
    pub saved_pops: u64,
    /// Saved tune discarded at 0xc300 (0x0412).
    pub saved_discards: u64,
}

/// Fault injection for negative-control (C-NEG) verification of tune
/// priority comparisons and the 0xc300 saved-tune copy (ticket #78).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MusicFault {
    /// Swap the priority comparison at 0x01d1 (cur < c <-> cur > c).
    pub swap_priority_comparison: bool,
    /// Drop the 0xc100 -> 0xc300 saved-channel copy at 0x01e5.
    pub drop_saved_copy: bool,
}

/// The music CPU: RAM, program ROM, and clock.
pub struct MusicCpu {
    rom: Vec<u8>,
    ram: Vec<u8>,
    /// The clock (chip clocks) the CPU has reached.
    pub t: u64,
    /// Original addresses of untranslated branches reached.
    pub unrecovered: BTreeSet<u16>,
    /// Reached branches observed during translation.
    pub observations: MusicObservations,
    /// Optional fault injection for negative controls.
    pub fault: MusicFault,
    /// Event reads nested by a `call` the original would have to unwind
    /// through (the double-speed extra count-down 0x0489, a tie's nested
    /// read 0x04f7): opcode 0xf's three pops assume none.
    nesting: u32,
    /// Where the host says what extension to run with, and what the program
    /// image holds now.
    cell: Option<ExtensionCell>,
    attached: Option<Arc<Vec<u8>>>,
    /// The program image's length without an extension.
    base_len: usize,
}

impl MusicCpu {
    /// From the program ROM (sf-02.7k) and a RAM image (0xc000-0xc7ff, e.g.
    /// a reference checkpoint's `ram_*_music.bin`).
    pub fn new(rom: Vec<u8>, ram: Vec<u8>) -> Self {
        MusicCpu {
            base_len: rom.len(),
            cell: None,
            attached: None,
            rom,
            ram,
            t: 0,
            unrecovered: BTreeSet::new(),
            observations: MusicObservations::default(),
            fault: MusicFault::default(),
            nesting: 0,
        }
    }

    pub fn ram(&self) -> &[u8] {
        &self.ram
    }

    /// Run with whatever extension `cell` holds from now on (the ROM alone
    /// while it holds none).
    pub fn with_extension_cell(mut self, cell: ExtensionCell) -> Self {
        self.cell = Some(cell);
        self
    }

    /// Whether the program image carries an extension now.
    pub fn has_extension(&self) -> bool {
        self.attached.is_some()
    }

    /// Whether a tune that is current or saved has a stream in the
    /// extension (the stream pointers are big-endian at each channel's +0).
    fn extension_in_use(&self) -> bool {
        let streams = |base: u16| {
            (0..8u16).any(|ch| {
                (EXTENSION_BASE..EXTENSION_END).contains(&self.rd16be(base + 0x30 * ch))
            })
        };
        (self.rd(C011_MUSIC) != 0 && streams(MUSIC_CHANNELS))
            || (self.rd(C012_SAVED) != 0 && streams(SAVED_CHANNELS))
    }

    /// Take the extension the cell holds, unless a tune is playing from the
    /// one the image has.
    fn sync_extension(&mut self) {
        let Some(cell) = &self.cell else { return };
        let want = cell.lock().unwrap().clone();
        let same = match (&want, &self.attached) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        if same || (self.attached.is_some() && self.extension_in_use()) {
            return;
        }
        self.rom.truncate(self.base_len);
        if let Some(table) = &want {
            self.rom.resize(EXTENSION_BASE as usize, 0xff);
            self.rom.extend_from_slice(table);
        }
        self.attached = want;
    }

    /// The NMI handler's store (0x006a: ld (0xc000),a), for a runner that
    /// takes an NMI while the CPU idles.
    pub fn set_nmi_copy(&mut self, v: u8) {
        self.wr(C000_NMI_COPY, v);
    }

    // ---- clock, memory ---------------------------------------------------

    /// Charge `n` T-states; NMIs that came due are taken here (0x0066:
    /// acceptance 11, push af 11, ld a,(0xc800) 13, ld (0xc000),a 13, pop af
    /// 10, retn 14).
    fn c(&mut self, bus: &mut impl MusicBus, n: u64) {
        self.t += n;
        while bus.take_nmi(self.t).is_some() {
            self.t += 11 + 11;
            let v = bus.latch(self.t + ABS_ACCESS);
            self.t += 13;
            self.wr(C000_NMI_COPY, v);
            self.t += 13 + 10 + 14;
        }
    }

    fn rd(&self, a: u16) -> u8 {
        match a {
            0x0000..=0xbfff => self.rom.get(a as usize).copied().unwrap_or(0xff),
            0xc000..=0xc7ff => self.ram[(a - 0xc000) as usize],
            _ => 0xff,
        }
    }

    fn wr(&mut self, a: u16, v: u8) {
        if (0xc000..=0xc7ff).contains(&a) {
            self.ram[(a - 0xc000) as usize] = v;
        }
    }

    fn rd16be(&self, a: u16) -> u16 {
        u16::from_be_bytes([self.rd(a), self.rd(a.wrapping_add(1))])
    }

    fn wr16be(&mut self, a: u16, v: u16) {
        let [h, l] = v.to_be_bytes();
        self.wr(a, h);
        self.wr(a.wrapping_add(1), l);
    }

    fn raw_word(&self, a: u16) -> u32 {
        u32::from_le_bytes(std::array::from_fn(|i| self.rd(a + i as u16)))
    }

    fn raw_store(&mut self, a: u16, v: u32) {
        for (i, byte) in v.to_le_bytes().into_iter().enumerate() {
            self.wr(a + i as u16, byte);
        }
    }

    fn raw_channel(&self, ix: u16) -> bool {
        self.rom.len() > EXTENSION_BASE as usize
            && self.rd(ix + RAW_FLAG) == RAW_MAGIC
            && (EXTENSION_BASE..EXTENSION_END).contains(&self.rd16be(ix))
    }

    fn raw_control(&self, ix: u16, value: u8) -> u8 {
        if self.raw_channel(ix) && self.rd(ix + RAW_FEEDBACK) < 8 {
            (value & !0x38) | (self.rd(ix + RAW_FEEDBACK) << 3)
        } else {
            value
        }
    }

    fn raw_duration(&mut self, bus: &mut impl MusicBus, ix: u16) -> u32 {
        let lo = self.read_byte(bus, ix);
        let hi = self.read_byte(bus, ix);
        u16::from_le_bytes([lo, hi]) as u32 * RAW_DURATION_CLOCKS
    }

    fn raw_wait(&mut self, ix: u16, duration: u32) {
        let overdue = self.raw_word(ix + RAW_OVERDUE);
        self.raw_store(ix + RAW_WAIT, duration.saturating_sub(overdue));
        self.raw_store(ix + RAW_OVERDUE, overdue.saturating_sub(duration));
    }

    /// Explicit spans keep wall-clock time, including time spent loading
    /// patches and coalesced native timer interrupts. Overdue credit carries
    /// into subsequent spans instead of accumulating one delay per note.
    fn raw_tick(&mut self, bus: &mut impl MusicBus, ix: u16) -> Flow {
        let now = self.t as u32;
        let elapsed = now.wrapping_sub(self.raw_word(ix + RAW_CLOCK));
        self.raw_store(ix + RAW_CLOCK, now);
        let wait = self.raw_word(ix + RAW_WAIT);
        if self.rd(ix + RAW_GATE_ARMED) != 0 {
            let gate = self.raw_word(ix + RAW_GATE).saturating_sub(elapsed);
            self.raw_store(ix + RAW_GATE, gate);
            if gate == 0 {
                self.wr(ix + RAW_GATE_ARMED, 0);
                self.wr(ix + 0x0e, 0);
                if self.rd(C010_SILENT) == 0 {
                    self.key_off_ch(bus, self.rd(C014_CHANNEL));
                }
            }
        }
        self.raw_store(ix + RAW_WAIT, wait.saturating_sub(elapsed));
        if elapsed < wait {
            return Flow::Done;
        }
        let credit = self.raw_word(ix + RAW_OVERDUE).saturating_add(elapsed - wait);
        self.raw_store(ix + RAW_OVERDUE, credit);
        for _ in 0..256 {
            let flow = self.next_event(bus, ix);
            if flow != Flow::Done || !self.raw_channel(ix) || self.raw_word(ix + RAW_WAIT) != 0 {
                return flow;
            }
        }
        // Valid converted streams always contain a positive span. Bound a
        // malformed image's catch-up work rather than hanging the CPU.
        self.note(0x000a);
        Flow::Done
    }

    fn raw_key_on(&mut self, bus: &mut impl MusicBus, ix: u16, attack: bool) {
        let patch = self.rd16be(ix + 0x18);
        // Reapply the SF1 patch/LFO on every attack, as in the approved
        // listening version. All writes still honor native effect ownership.
        if attack {
            self.load_patch(bus, ix, patch);
        }
        let kc = self.rd(ix + RAW_KC);
        self.wr(C002_KEYCODE, kc);
        self.levels(bus, patch, MUSIC_LEVELS);
        let ch = self.rd(C014_CHANNEL);
        self.write_ch(bus, ch + 0x28, kc);
        self.write_ch(bus, ch + 0x30, self.rd(ix + RAW_KF));
        if self.rd(patch + 2) & 2 != 0 {
            self.write_ch(bus, 0x01, 2);
            self.write_ch(bus, 0x01, 0);
        }
        self.write_ch(bus, 0x08, ch | self.rd(patch + 8));
    }

    /// 0x0a is deliberately defined only for an extended stream. Legacy
    /// ROM streams retain their untranslated 0x0a behavior and cycle costs.
    fn raw_opcode(&mut self, bus: &mut impl MusicBus, ix: u16) -> Option<Flow> {
        let sub = self.read_byte(bus, ix);
        if !self.raw_channel(ix) {
            self.wr(ix + RAW_FLAG, RAW_MAGIC);
            self.wr(ix + RAW_FEEDBACK, 0xff);
            self.raw_store(ix + RAW_WAIT, 0);
            self.raw_store(ix + RAW_GATE, 0);
            self.raw_store(ix + RAW_OVERDUE, 0);
            self.raw_store(ix + RAW_CLOCK, self.t as u32);
        }
        match sub {
            0 => {
                let patch = self.read_byte(bus, ix);
                let carrier = self.read_byte(bus, ix);
                let modulator = self.read_byte(bus, ix);
                let feedback = self.read_byte(bus, ix);
                self.wr(ix + 0x17, patch);
                self.wr16be(ix + 0x18, 0x6b00 + 0x29 * patch as u16);
                self.wr(ix + RAW_CARRIER, carrier);
                self.wr(ix + RAW_MODULATOR, modulator);
                self.wr(ix + RAW_FEEDBACK, feedback);
                None
            }
            1 => {
                let kc = self.read_byte(bus, ix);
                let kf = self.read_byte(bus, ix);
                let duration = self.raw_duration(bus, ix);
                let gate = self.raw_duration(bus, ix);
                let overdue = self.raw_word(ix + RAW_OVERDUE);
                self.wr(ix + RAW_KC, kc);
                self.wr(ix + RAW_KF, kf);
                self.wr(ix + 0x0e, 1);
                self.wr(ix + RAW_GATE_ARMED, u8::from(gate != 0));
                self.raw_store(ix + RAW_GATE, gate.saturating_sub(overdue));
                self.raw_wait(ix, duration);
                if self.rd(C010_SILENT) == 0 {
                    self.raw_key_on(bus, ix, true);
                }
                if gate != 0 && gate <= overdue {
                    self.wr(ix + RAW_GATE_ARMED, 0);
                    self.wr(ix + 0x0e, 0);
                    if self.rd(C010_SILENT) == 0 {
                        self.key_off_ch(bus, self.rd(C014_CHANNEL));
                    }
                }
                Some(Flow::Done)
            }
            2 => {
                let kc = self.read_byte(bus, ix);
                let kf = self.read_byte(bus, ix);
                self.wr(ix + RAW_KC, kc);
                self.wr(ix + RAW_KF, kf);
                if self.rd(C010_SILENT) == 0 {
                    let ch = self.rd(C014_CHANNEL);
                    self.write_ch(bus, ch + 0x28, kc);
                    self.write_ch(bus, ch + 0x30, kf);
                }
                None
            }
            3 => {
                let duration = self.raw_duration(bus, ix);
                self.raw_wait(ix, duration);
                Some(Flow::Done)
            }
            4 => {
                self.wr(ix + 0x0e, 0);
                self.wr(ix + RAW_GATE_ARMED, 0);
                if self.rd(C010_SILENT) == 0 {
                    self.key_off_ch(bus, self.rd(C014_CHANNEL));
                }
                None
            }
            _ => {
                self.note(0x000a);
                Some(Flow::Done)
            }
        }
    }

    fn note(&mut self, addr: u16) {
        self.unrecovered.insert(addr);
    }

    /// rst 0x20 (A = (HL + A)), with its cost: rst 11, add 4, ld 4, jr nc
    /// 12 / (7 + inc h 4), ld a,(hl) 7, ret 10.
    fn rst20(&mut self, bus: &mut impl MusicBus, hl: u16, a: u8) -> u8 {
        let carry = (hl & 0xff) + a as u16 > 0xff;
        self.c(bus, 11 + 4 + 4 + if carry { 11 } else { 12 } + 7 + 10);
        self.rd(hl.wrapping_add(a as u16))
    }

    // ---- chip helpers (0x00fc and its users) ---------------------------------

    /// 0x00fc: wait for the chip, then write `reg` = `d`.
    fn opm_write(&mut self, bus: &mut impl MusicBus, reg: u8, d: u8) {
        self.c(bus, 10 + 11); // ld bc,0xe001; push af
        loop {
            let st = bus.status(self.t + IND_ACCESS); // ld a,(bc)
            self.c(bus, 7 + 8); // ld a,(bc); bit 7,a
            if st & 0x80 != 0 {
                self.c(bus, 12); // jr nz,0x0100
            } else {
                self.c(bus, 7);
                break;
            }
        }
        self.c(bus, 10 + 4); // pop af; dec c
        bus.write(self.t + IND_ACCESS, 0, reg);
        self.c(bus, 7 + 4 + 11 + 4); // ld (bc),a; inc c; push af; ld a,d
        bus.write(self.t + IND_ACCESS, 1, d);
        self.c(bus, 7 + 10 + 10); // ld (bc),a; pop af; ret
    }

    /// 0x009d: total level 0x7f on the channel's four operators.
    fn mute(&mut self, bus: &mut impl MusicBus, c: u8) {
        self.c(bus, 7 + 4 + 7 + 7);
        let mut reg = 0x60u8.wrapping_add(c);
        for i in 0..4 {
            self.c(bus, 11 + 17);
            self.opm_write(bus, reg, 0x7f);
            reg = reg.wrapping_add(8);
            self.c(bus, 7 + 10 + if i < 3 { 13 } else { 8 });
        }
        self.c(bus, 10);
    }

    /// 0x00b9: key off (register 8 = channel, no operators).
    fn key_off(&mut self, bus: &mut impl MusicBus, c: u8) {
        self.c(bus, 7 + 4 + 11 + 17);
        self.opm_write(bus, 0x08, c);
        self.c(bus, 10 + 10);
    }

    /// 0x0a10: does an effect own the current channel (0xc023 bit 7)? The
    /// music's write is then skipped (0x0a16: the pushed IY dropped, `ret`).
    /// Charges the check and its exit (ex af,af' at either end).
    fn owned(&mut self, bus: &mut impl MusicBus) -> bool {
        self.c(bus, 4 + 17 + 13 + 8);
        if self.rd(C023_OWNED) & 0x80 == 0 {
            self.c(bus, 11 + 4);
            false
        } else {
            self.c(bus, 5 + 14 + 4 + 10);
            self.observations.owned_skips += 1;
            true
        }
    }

    /// 0x09f8: write unless an effect owns the channel (the caller charges
    /// its `call`).
    fn write_ch(&mut self, bus: &mut impl MusicBus, reg: u8, d: u8) {
        if !self.owned(bus) {
            self.c(bus, 10); // jp 0x00fc
            self.opm_write(bus, reg, d);
        }
    }

    /// 0x0a00: key off unless an effect owns the channel.
    fn key_off_ch(&mut self, bus: &mut impl MusicBus, c: u8) {
        if !self.owned(bus) {
            self.c(bus, 10); // jp 0x00b9
            self.key_off(bus, c);
        }
    }

    // ---- power-on (ticket #36) --------------------------------------------------

    /// 0x0000, reset: `di` 4, `im 1` 8, `ld sp,0xc800` 10, `jp 0x0070` 10;
    /// 0x0070: the RAM cleared (ld hl / de / bc 30, ld (hl),0 10, ldir 0x7ff
    /// bytes 42,982), every channel muted (0x0092: ld c,8 7; per channel 7
    /// down to 0 dec c 4, call 0x009d 17, ld a,c 4, or a 4, jr nz 12 / 7;
    /// ret 10) and keyed off (0x00ae, likewise with 0x00b9), the timers
    /// stopped (0x00c2: ld d,0 7, ld a,0x14 7, call 0x00fc 17, ret 10),
    /// timer A loaded (0x00ca), a delay (0x00f3: ld bc,3 10, three rounds of
    /// 256 `djnz` 13 / 8 with dec c 4 and jr nz 12 / 7, ret 10), timer B
    /// loaded (0x00e2), then `jp 0x0009` 10 into the idle loop. From reset
    /// at clock 0 the first chip write lands at clock 43,213, as in the
    /// reference. Leaves `t` at the idle loop's first `ei`.
    pub fn boot(&mut self, bus: &mut impl MusicBus) {
        self.c(bus, 4 + 8 + 10 + 10);
        self.c(bus, 10 + 10 + 10 + 10 + 21 * 0x7fe + 16);
        for i in 0..0x800u16 {
            self.wr(0xc000 + i, 0);
        }
        self.c(bus, 17 + 7); // call 0x0092; ld c,8
        for ch in (0..8u8).rev() {
            self.c(bus, 4 + 17); // dec c; call 0x009d
            self.mute(bus, ch);
            self.c(bus, 4 + 4 + if ch > 0 { 12 } else { 7 }); // ld a,c; or a; jr nz
        }
        self.c(bus, 10 + 17 + 7); // ret; call 0x00ae; ld c,8
        for ch in (0..8u8).rev() {
            self.c(bus, 4 + 17); // dec c; call 0x00b9
            self.key_off(bus, ch);
            self.c(bus, 4 + 4 + if ch > 0 { 12 } else { 7 });
        }
        self.c(bus, 10 + 17 + 7 + 7 + 17); // ret; call 0x00c2; ld d,0; ld a,0x14; call 0x00fc
        self.opm_write(bus, 0x14, 0x00);
        self.c(bus, 10 + 17); // ret; call 0x00ca
        self.timer_a_rearm(bus);
        // call 0x00f3 17; ld bc,3 10; three rounds (255 x djnz 13 + 8, dec c 4,
        // jr nz 12 / 7); ret 10
        self.c(bus, 17 + 10 + 3 * (255 * 13 + 8 + 4) + 12 + 12 + 7 + 10);
        self.c(bus, 17); // call 0x00e2
        self.timer_b_rearm(bus);
        self.c(bus, 10); // jp 0x0009
    }

    // ---- interrupts ---------------------------------------------------------

    /// 0x0038: the timer interrupt, from its first instruction (the caller
    /// charges the 13-T-state acceptance) through `ei`. The caller finishes
    /// with [`Self::reti`] unless an NMI after `ei` leaves an interrupt
    /// pending: the NMI's `retn` ends the shadow and that interrupt is then
    /// accepted before `reti`. Returns whether such an NMI was taken.
    pub fn interrupt(&mut self, bus: &mut impl MusicBus) -> bool {
        self.c(bus, 4 + 10); // di; ld bc,0xe001
        let st = bus.status(self.t + IND_ACCESS);
        self.c(bus, 7 + 8); // ld a,(bc); bit 0,a
        let mut timer_b = true;
        if st & 1 != 0 {
            self.c(bus, 7 + 17);
            self.timer_a_rearm(bus);
            self.c(bus, 17);
            self.effects(bus);
            self.c(bus, 17);
            self.music_tick(bus);
            self.c(bus, 10);
            let st = bus.status(self.t + IND_ACCESS);
            self.c(bus, 7 + 8);
            if st & 2 == 0 {
                self.c(bus, 12);
                timer_b = false;
            } else {
                self.c(bus, 7);
            }
        } else {
            self.c(bus, 12);
        }
        if timer_b {
            self.c(bus, 17);
            self.timer_b_rearm(bus);
            self.c(bus, 17);
            self.commands(bus);
        }
        let end = self.t + 4;
        self.c(bus, 4); // ei
        self.t != end
    }

    /// 0x0059: return from the timer interrupt. An NMI can arrive in this
    /// block too; the pending timer interrupt then follows its `retn`.
    pub fn reti(&mut self, bus: &mut impl MusicBus) {
        self.c(bus, 14);
        // A stop can finish the last extended stream after command intake
        // deferred its removal. Idle 0xff commands do not enter that lookup
        // again, so release it here, safely between handlers.
        self.sync_extension();
    }

    /// 0x00ca: timer A = 0x356, load and reset (0x14 = 0x1f).
    fn timer_a_rearm(&mut self, bus: &mut impl MusicBus) {
        self.c(bus, 7 + 7 + 17);
        self.opm_write(bus, 0x10, 0xd6);
        self.c(bus, 7 + 7 + 17);
        self.opm_write(bus, 0x11, 0x00);
        self.c(bus, 7 + 8 + 7 + 17);
        self.opm_write(bus, 0x14, 0x1f);
        self.c(bus, 10);
    }

    /// 0x00e2: timer B = 0xf2, load and reset (0x14 = 0x2f).
    fn timer_b_rearm(&mut self, bus: &mut impl MusicBus) {
        self.c(bus, 7 + 7 + 17);
        self.opm_write(bus, 0x12, 0xf2);
        self.c(bus, 7 + 8 + 7 + 17);
        self.opm_write(bus, 0x14, 0x2f);
        self.c(bus, 10);
    }

    /// 0x0a9b: the two effect slots' ticks, before the sequencer's (ticket
    /// #34): slot A by `call` — an effect ending there (opcode 5) returns to
    /// the slot-B check — then slot B by fall-through (its `ret`s are the
    /// routine's). An empty slot (mask 0) is skipped.
    fn effects(&mut self, bus: &mut impl MusicBus) {
        self.c(bus, 14 + 14 + 19 + 4); // ld iy; ld ix; ld a,(iy+0); or a
        if self.rd(EFFECT_SLOT_A) != 0 {
            self.c(bus, 7 + 17); // jr z not taken; call 0x0ab9
            self.effect_tick(bus, EFFECT_SLOT_A);
        } else {
            self.c(bus, 12);
        }
        self.c(bus, 14 + 14 + 19 + 4);
        if self.rd(EFFECT_SLOT_B) != 0 {
            self.c(bus, 5); // ret z not taken
            self.effect_tick(bus, EFFECT_SLOT_B);
        } else {
            self.c(bus, 11);
        }
    }

    // ---- commands ---------------------------------------------------------------

    /// 0x019d: take a new command from the latch.
    fn commands(&mut self, bus: &mut impl MusicBus) {
        self.c(bus, 13 + 4); // ld a,(0xc001); ld c,a
        let a = bus.latch(self.t + ABS_ACCESS);
        self.c(bus, 13 + 4); // ld a,(0xc800); cp c
        if a == self.rd(C001_LAST_CMD) {
            if a != 0xff {
                self.observations.repeated_cmd_ignores += 1;
            }
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 8);
        if a & 0x80 != 0 {
            self.c(bus, 7 + 7);
            if a != 0xff {
                self.c(bus, 11);
                return;
            }
            self.c(bus, 5 + 13 + 10);
            self.wr(C001_LAST_CMD, a);
            return;
        }
        self.c(bus, 12 + 13 + 10); // jr z; ld (0xc001),a; ld hl,0x1011
        self.wr(C001_LAST_CMD, a);
        // rst 0x28: rst 11, add a,a 4, rst 0x18 (41 / 40), ld e 7, inc 6,
        // ld d 7, inc 6, ret 10.
        let off = a.wrapping_add(a);
        let carry = 0x11u16 + off as u16 > 0xff;
        self.c(
            bus,
            11 + 4 + 11 + 4 + 4 + if carry { 11 } else { 12 } + 10 + 7 + 6 + 7 + 6 + 10,
        );
        // The extension the host wants is taken after the latch was read (a
        // frame's write is handed over after any change pushed before it)
        // and before the lookup that may need it.
        self.sync_extension();
        // The extended table takes the commands the 1987 game never sends,
        // at the 1987 lookup's cost, when the image carries one.
        let extended = a >= EXTENDED_FIRST && self.rom.len() > EXTENSION_BASE as usize;
        let entry = if extended {
            EXTENSION_BASE + 2 * (a - EXTENDED_FIRST) as u16
        } else {
            0x1011u16.wrapping_add(off as u16)
        };
        let de = u16::from_le_bytes([self.rd(entry), self.rd(entry + 1)]);
        self.c(bus, 4 + 7 + 10); // ld a,d; cp 0xff; jp z
        if extended && de == 0 {
            // No tune is named: the command does nothing.
            return;
        }
        if de >> 8 == 0xff {
            self.special(bus, de as u8);
            return;
        }
        let header = self.rd(de);
        self.c(bus, 7 + 8 + 10); // ld a,(de); bit 7,a; jp nz
        if header & 0x80 != 0 {
            self.effect_start(bus, de, header);
            return;
        }
        self.c(bus, 4 + 7 + 4 + 13 + 4 + 7 + 4); // ... cp c
        let c = header & 0x0f;
        let b = self.rd(C011_MUSIC);
        let cur = b & 0x0f;
        if cur != c {
            self.c(bus, 7);
            let outranks = if self.fault.swap_priority_comparison {
                cur > c
            } else {
                cur < c
            };
            if outranks {
                // 0x01e5: the new tune outranks the current one: save it.
                self.observations.tune_higher_priority_saves += 1;
                self.c(
                    bus,
                    12 + 4 + 13 + 11 + 11 + 10 + 10 + 10 + 21 * 0x17e + 16 + 10 + 10,
                );
                self.wr(C012_SAVED, b);
                if !self.fault.drop_saved_copy {
                    for i in 0..0x17f {
                        let v = self.rd(MUSIC_CHANNELS + i);
                        self.wr(SAVED_CHANNELS + i, v);
                    }
                }
            } else {
                // 0x01d3: into the saved slot if it ranks there.
                self.c(bus, 7 + 13 + 7 + 4);
                let saved = self.rd(C012_SAVED) & 0x0f;
                if saved == c {
                    self.c(bus, 12);
                } else {
                    self.c(bus, 7);
                    if saved > c {
                        self.c(bus, 11);
                        return;
                    }
                    self.c(bus, 5);
                }
                self.observations.tune_lower_priority_saved_loads += 1;
                self.c(bus, 4 + 13 + 10 + 12);
                self.wr(C012_SAVED, header);
                self.load_channels(bus, SAVED_CHANNELS, de);
                return;
            }
        } else {
            self.observations.tune_equal_priority += 1;
            self.c(bus, 12);
        }
        // 0x01f8
        self.c(bus, 4 + 13 + 10 + 17);
        self.wr(C011_MUSIC, header);
        self.load_channels(bus, MUSIC_CHANNELS, de);
        // 0x0202: silence every channel no effect owns.
        self.c(bus, 14 + 19 + 14 + 19 + 4 + 7);
        let mut e = self.rd(EFFECT_SLOT_A) | self.rd(EFFECT_SLOT_B);
        for ch in 0..8u8 {
            self.c(bus, 11 + 7 + 4 + 4 + 8);
            if e & 0x80 == 0 {
                self.c(bus, 7 + 17);
                self.mute(bus, ch);
                self.c(bus, 17);
                self.key_off(bus, ch);
            } else {
                self.c(bus, 12);
            }
            e = e.rotate_left(1);
            self.c(bus, 8 + 10 + if ch < 7 { 13 } else { 8 });
        }
        self.c(bus, 10);
    }

    /// 0x0228: eight channel stream pointers from the header (little-endian
    /// pairs after the header byte), the rest of each channel cleared.
    fn load_channels(&mut self, bus: &mut impl MusicBus, base: u16, de: u16) {
        self.c(bus, 7);
        let mut de = de;
        let mut hl = base;
        for ch in 0..8 {
            de = de.wrapping_add(1);
            let c = self.rd(de);
            de = de.wrapping_add(1);
            let b = self.rd(de);
            self.wr(hl, b);
            self.wr(hl + 1, c);
            hl += 2;
            for _ in 0..0x2e {
                self.wr(hl, 0);
                hl += 1;
            }
            self.c(bus, 11 + 6 + 7 + 4 + 6 + 7 + 4 + 7 + 6 + 7 + 6 + 7 + 4);
            self.c(bus, (7 + 6) * 0x2e + 13 * 0x2d + 8);
            self.c(bus, 10 + if ch < 7 { 13 } else { 8 });
        }
        self.c(bus, 10);
    }

    // ---- special commands (ticket #33) ------------------------------------------

    /// 0x02a5: a special command (the command table's entry has 0xff for its
    /// high byte; `e` is the low byte): `ld a,e; rst 0x30` through the table
    /// at 0x02a7 (rst 11, pop hl 10, rst 0x28: rst 11, add a,a 4, rst 0x18
    /// 41, ld/inc/ld/inc 26, ret 10; ex de,hl 4; jp (hl) 4). The handler's
    /// `ret` returns to 0x0058, so the intake ends with it. This ROM's
    /// command table selects entries 0 (command 0x00), 1 (0x0c, 0x0e, 0x15,
    /// 0x1a, 0x1d-0x1f, 0x23-0x27) and 4 (0x35, 0x3d, 0x3f); entries 2, 3,
    /// 5 and 6 are reached as calls and from opcode 0xf.
    fn special(&mut self, bus: &mut impl MusicBus, e: u8) {
        self.c(bus, 4 + 11 + 10 + 11 + 4 + 41 + 26 + 10 + 4 + 4);
        match e {
            0 => {
                // 0x02b5: stop everything: the music, then both effect slots.
                self.c(bus, 17);
                self.stop_music(bus);
                self.c(bus, 17);
                self.stop_effect(bus, EFFECT_SLOT_B);
                self.stop_effect(bus, EFFECT_SLOT_A);
            }
            1 => {
                // 0x02b8: stop both effect slots.
                self.c(bus, 17);
                self.stop_effect(bus, EFFECT_SLOT_B);
                self.stop_effect(bus, EFFECT_SLOT_A);
            }
            2 => self.stop_effect(bus, EFFECT_SLOT_A),
            3 => self.stop_effect(bus, EFFECT_SLOT_B),
            4 => self.stop_music(bus),
            5 => self.pop_saved(bus),
            6 => self.discard_saved(bus),
            // Past the table: the original jumps through tune data.
            _ => self.note(0x02a5),
        }
    }

    /// 0x02bb (slot A, 0xc500) / 0x02f3 (slot B, 0xc580): stop one effect
    /// slot: its priority byte cleared; the noise register (0x0f) cleared
    /// when the slot used channel 7 (mask bit 0); the music re-established
    /// (0x032b) on every channel the slot owned that the other slot does
    /// not — with E = ~mask | the other mask, or, for an empty slot, the
    /// other mask alone: then every free channel is re-established; the
    /// slot cleared (0x50 bytes). Charges through its `ret`.
    fn stop_effect(&mut self, bus: &mut impl MusicBus, slot: u16) {
        let (priority, other) = if slot == EFFECT_SLOT_A {
            (C020_EFFECT_A_PRIORITY, EFFECT_SLOT_B)
        } else {
            (C021_EFFECT_B_PRIORITY, EFFECT_SLOT_A)
        };
        // xor a 4; ld (nn),a 13; ld iy,nn 14; bit 0,(iy+0) 20; jr z 12 / 7
        self.c(bus, 4 + 13 + 14 + 20);
        self.wr(priority, 0);
        let mask = self.rd(slot);
        if mask & 1 != 0 {
            self.c(bus, 7 + 7 + 7 + 17); // ld a,0x0f; ld d,0; call 0x00fc
            self.opm_write(bus, 0x0f, 0x00);
        } else {
            self.c(bus, 12);
        }
        // ld a,(iy+0) 19; or a 4; jr z 12 / 7 + cpl 4
        self.c(bus, 19 + 4);
        let a = if mask == 0 {
            self.c(bus, 12);
            0
        } else {
            self.c(bus, 7 + 4);
            !mask
        };
        // ld iy,nn 14; or (iy+0) 19; ld e,a 4; ld (0xc023),a 13; call 0x032b 17
        self.c(bus, 14 + 19 + 4 + 13 + 17);
        let e = a | self.rd(other);
        self.wr(C023_OWNED, e);
        self.reestablish(bus, e);
        // ld hl 10; ld de 10; ld bc,0x4f 10; ld (hl),0 10; ldir 1654; ret 10
        self.c(bus, 10 + 10 + 10 + 10 + 21 * 0x4e + 16 + 10);
        for i in 0..0x50 {
            self.wr(slot + i, 0);
        }
    }

    /// 0x032b: re-establish the music on every channel whose bit of `e` is
    /// clear (bit 7 = channel 0, rotated per channel; 0xc023 rotates in
    /// step): a channel without a stream is muted (unless an effect is
    /// ending by itself, 0xc024) and keyed off; one with a stream is muted,
    /// keyed off, its patch registers rewritten (0x0924 from patch + 5: no
    /// LFO block) and its current note keyed on again. Then the four LFO
    /// registers are rewritten from 0xc015-0xc018. Charges through its
    /// `ret`.
    fn reestablish(&mut self, bus: &mut impl MusicBus, e: u8) {
        self.c(bus, 14 + 7); // ld ix,0xc100; ld b,8
        let mut e = e;
        let mut ix = MUSIC_CHANNELS;
        for ch in 0..8u8 {
            // push bc 11; ld a,8 7; sub b 4; ld (0xc014),a 13; push de 11;
            // bit 7,e 8; jr nz 12 / 7
            self.c(bus, 11 + 7 + 4 + 13 + 11 + 8);
            self.wr(C014_CHANNEL, ch);
            if e & 0x80 != 0 {
                self.c(bus, 12);
            } else {
                // ld a,(ix+0) 19; or (ix+1) 19; jr nz 12 / 7
                self.c(bus, 7 + 19 + 19);
                if self.rd(ix) | self.rd(ix + 1) == 0 {
                    // 0x0345: ld a,(0xc014) 13; ld c,a 4; ld a,(0xc024) 13;
                    // or a 4; call z,0x009d 17 / 10; call 0x00b9 17; jr 12
                    self.c(bus, 7 + 13 + 4 + 13 + 4);
                    if self.rd(C024_EFFECT_ENDING) == 0 {
                        self.c(bus, 17);
                        self.mute(bus, ch);
                    } else {
                        self.c(bus, 10);
                    }
                    self.c(bus, 17);
                    self.key_off(bus, ch);
                    self.c(bus, 12);
                } else {
                    // 0x0355: ld a,(0xc014) 13; ld c,a 4; call 0x009d 17;
                    // call 0x00b9 17; ld h,(ix+0x18) 19; ld l,(ix+0x19) 19;
                    // ld bc,5 10; add hl,bc 11; call 0x0924 17
                    self.c(bus, 12 + 13 + 4 + 17);
                    self.mute(bus, ch);
                    self.c(bus, 17);
                    self.key_off(bus, ch);
                    self.c(bus, 19 + 19 + 10 + 11 + 17);
                    let patch = self.rd16be(ix + 0x18);
                    self.load_patch_registers(bus, ix, patch.wrapping_add(5));
                    // ld a,(ix+0x0e) 19; or a 4; call nz,0x0551 17 / 10
                    self.c(bus, 19 + 4);
                    if self.rd(ix + 0x0e) != 0 {
                        self.c(bus, 17);
                        self.key_on(bus, ix);
                    } else {
                        self.c(bus, 10);
                    }
                }
            }
            // 0x0373: pop de 10; rlc e 8; ld a,(0xc023) 13; rlca 4;
            // ld (0xc023),a 13; ld bc,0x30 10; add ix,bc 15; pop bc 10;
            // djnz 13 / 8
            self.c(
                bus,
                10 + 8 + 13 + 4 + 13 + 10 + 15 + 10 + if ch < 7 { 13 } else { 8 },
            );
            e = e.rotate_left(1);
            let o = self.rd(C023_OWNED).rotate_left(1);
            self.wr(C023_OWNED, o);
            ix += 0x30;
        }
        // 0x0385: ld hl,0xc015 10; four x (ld d,(hl) 7; ld a,n 7; call 0x00fc
        // 17), inc hl 6 between; ret 10.
        for (i, reg) in [0x1bu8, 0x18, 0x19, 0x19].into_iter().enumerate() {
            self.c(bus, if i == 0 { 10 } else { 6 } + 7 + 7 + 17);
            let v = self.rd(C015_LFO + i as u16);
            self.opm_write(bus, reg, v);
        }
        self.c(bus, 10);
    }

    /// 0x03a4: stop the music (commands 0x35, 0x3d, 0x3f): the saved slot
    /// discarded, then 0x03a7 pops the now empty saved tune — 0xc011 = 0,
    /// the channels zeroed, every channel no effect owns muted and keyed
    /// off. Effects, their priorities and the LFO registers are untouched.
    fn stop_music(&mut self, bus: &mut impl MusicBus) {
        self.c(bus, 17); // call 0x0412
        self.discard_saved(bus);
        self.pop_saved(bus);
    }

    /// 0x0412: discard the saved tune: 0xc012 = 0 and 0xc300-0xc47f cleared
    /// (xor a 4; ld (nn),a 13; ld hl / de / bc 30; ld (hl),0 10; ldir 0x17f
    /// 8038; ret 10).
    fn discard_saved(&mut self, bus: &mut impl MusicBus) {
        self.observations.saved_discards += 1;
        self.c(bus, 4 + 13 + 10 + 10 + 10 + 10 + 21 * 0x17e + 16 + 10);
        self.wr(C012_SAVED, 0);
        for i in 0..0x180 {
            self.wr(SAVED_CHANNELS + i, 0);
        }
    }

    /// 0x03a7: pop the saved tune into the current slot (opcode 0xf's
    /// target too): 0xc011 = 0xc012, the saved channels copied over the
    /// current ones (0x17f bytes: channel 7's last byte stays), the saved
    /// slot discarded; then 0xc023 = both effect masks and, per channel no
    /// effect owns: mute, key off, and with a stream the patch loaded in
    /// full (0x08e4) and the current note keyed on. Charges its `ret`.
    fn pop_saved(&mut self, bus: &mut impl MusicBus) {
        // ld a,(0xc012) 13; ld (0xc011),a 13; ld hl / de / bc 30; ldir 0x17f
        // 8038; call 0x0412 17
        self.observations.saved_pops += 1;
        self.c(bus, 13 + 13 + 10 + 10 + 10 + 21 * 0x17e + 16 + 17);
        let header = self.rd(C012_SAVED);
        self.wr(C011_MUSIC, header);
        for i in 0..0x17f {
            let v = self.rd(SAVED_CHANNELS + i);
            self.wr(MUSIC_CHANNELS + i, v);
        }
        // A saved tune that did not silently advance was paused. Its
        // remaining spans resume now, without counting the interlude.
        for ch in 0..8 {
            let ix = MUSIC_CHANNELS + 0x30 * ch;
            if self.raw_channel(ix) {
                self.raw_store(ix + RAW_CLOCK, self.t as u32);
            }
        }
        self.discard_saved(bus);
        // ld iy 14; ld a,(iy+0) 19; ld iy 14; or (iy+0) 19; ld (0xc023),a 13;
        // ld ix,0xc100 14; ld b,8 7
        self.c(bus, 14 + 19 + 14 + 19 + 13 + 14 + 7);
        let owned = self.rd(EFFECT_SLOT_A) | self.rd(EFFECT_SLOT_B);
        self.wr(C023_OWNED, owned);
        let mut ix = MUSIC_CHANNELS;
        for ch in 0..8u8 {
            // push bc 11; ld a,8 7; sub b 4; ld (0xc014),a 13; ld a,(0xc023)
            // 13; bit 7,a 8; jr nz 12 / 7
            self.c(bus, 11 + 7 + 4 + 13 + 13 + 8);
            self.wr(C014_CHANNEL, ch);
            if self.rd(C023_OWNED) & 0x80 != 0 {
                self.c(bus, 12);
            } else {
                // ld a,(0xc014) 13; ld c,a 4; call 0x009d 17; call 0x00b9 17;
                // ld a,(ix+0) 19; or (ix+1) 19; jr z 12 / 7
                self.c(bus, 7 + 13 + 4 + 17);
                self.mute(bus, ch);
                self.c(bus, 17);
                self.key_off(bus, ch);
                self.c(bus, 19 + 19);
                if self.rd(ix) | self.rd(ix + 1) == 0 {
                    self.c(bus, 12);
                } else {
                    // ld h,(ix+0x18) 19; ld l,(ix+0x19) 19; call 0x08e4 17;
                    // ld a,(ix+0x0e) 19; or a 4; call nz,0x0551 17 / 10
                    self.c(bus, 7 + 19 + 19 + 17);
                    let patch = self.rd16be(ix + 0x18);
                    self.load_patch(bus, ix, patch);
                    self.c(bus, 19 + 4);
                    if self.rd(ix + 0x0e) != 0 {
                        self.c(bus, 17);
                        self.key_on(bus, ix);
                    } else {
                        self.c(bus, 10);
                    }
                }
            }
            // 0x0402: ld a,(0xc023) 13; rlca 4; ld (0xc023),a 13; ld bc,0x30
            // 10; add ix,bc 15; pop bc 10; djnz 13 / 8
            self.c(
                bus,
                13 + 4 + 13 + 10 + 15 + 10 + if ch < 7 { 13 } else { 8 },
            );
            let o = self.rd(C023_OWNED).rotate_left(1);
            self.wr(C023_OWNED, o);
            ix += 0x30;
        }
        self.c(bus, 10); // ret
    }

    // ---- sound effects (ticket #34) ----------------------------------------------

    /// 0x0240: start a sound effect — the intake found a header with bit 7
    /// set at `de`. The slot by header bit 6 (0xc500 / 0xc580) and its
    /// priority byte (0xc020 / 0xc021) against the header's low nibble: equal
    /// starts the effect, higher is recorded first, lower is ignored (`ret
    /// nc`). The slot is zeroed and loaded — mask = header + 1 (bit 7 - ch =
    /// channel ch), stream = header + 2 — the noise register cleared when
    /// the mask has channel 7, and, when the mask differs from the slot's
    /// previous one, the music re-established on every channel whose
    /// ownership changed (0x032b with E = ~(new ^ old); 0xc023 is left as
    /// it is). The `ret`s return to 0x0058.
    fn effect_start(&mut self, bus: &mut impl MusicBus, de: u16, header: u8) {
        self.c(bus, 8 + 10); // bit 6,a; jp nz
        let slot_a = header & 0x40 == 0;
        let (slot, priority) = if slot_a {
            (EFFECT_SLOT_A, C020_EFFECT_A_PRIORITY)
        } else {
            (EFFECT_SLOT_B, C021_EFFECT_B_PRIORITY)
        };
        // 0x0245 / 0x025e: ld iy 14, ld hl 10, and 0x0f 7, ld b,a 4, ld a,(nn) 13,
        // cp b 4, then jp z 10 (slot A) / jr z 12 taken, 7 not (slot B).
        let new = header & 0x0f;
        let cur = self.rd(priority);
        self.c(bus, 14 + 10 + 7 + 4 + 13 + 4);
        if slot_a {
            self.c(bus, 10);
        } else {
            self.c(bus, if cur == new { 12 } else { 7 });
        }
        if cur != new {
            if cur > new {
                self.c(bus, 11); // ret nc: a running effect outranks it
                return;
            }
            // ret nc not taken 5; ld a,b 4; ld (nn),a 13; jp 0x0273 10 (slot A only)
            self.c(bus, 5 + 4 + 13 + if slot_a { 10 } else { 0 });
            self.wr(priority, new);
        }
        // 0x0273: inc de 6, ld a,(de) 7, xor (iy+0) 19, push af 11, push de 11,
        // push hl 11, pop de 10, inc de 6, ld bc,0x4f 10, ld (hl),0 10, ldir 1654.
        self.c(
            bus,
            6 + 7 + 19 + 11 + 11 + 11 + 10 + 6 + 10 + 10 + 21 * 0x4e + 16,
        );
        let mask = self.rd(de.wrapping_add(1));
        let changed = mask ^ self.rd(slot);
        for i in 0..0x50 {
            self.wr(slot + i, 0);
        }
        // 0x0284: pop de 10, ld a,(de) 7, ld (iy+0),a 19, inc de 6, ld (iy+1),d 19,
        // ld (iy+2),e 19, bit 0,(iy+0) 20, jr z 12 / 7 + ld a 7, ld d 7, call 17.
        self.c(bus, 10 + 7 + 19 + 6 + 19 + 19 + 20);
        self.wr(slot, mask);
        self.wr16be(slot + 1, de.wrapping_add(2));
        if mask & 1 != 0 {
            self.c(bus, 7 + 7 + 7 + 17);
            self.opm_write(bus, 0x0f, 0x00);
        } else {
            self.c(bus, 12);
        }
        // 0x029d: pop af 10, or a 4, ret z 11 / 5; cpl 4, ld e,a 4, jp 0x032b 10.
        self.c(bus, 10 + 4);
        if changed == 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 4 + 4 + 10);
        self.reestablish(bus, !changed);
    }

    /// 0x0ab9: one slot's tick (IY = slot, IX = its first channel record).
    /// A running countdown (+3/+4, big-endian) is decremented and every
    /// channel of the mask slides (0x0afe), then the noise frequency
    /// advances (+6 += +7) when the increment is set; a zero countdown reads
    /// each channel's events (0x0b49) instead. Channels run 7 down to 0
    /// (0xc022), the mask rotating right per channel (eight rotations
    /// restore it), the 8-byte records from slot + 0x10 (channel 7 first).
    /// Charges through the `ret`; an effect's end (opcode 5) abandons the
    /// loop and returns through 0x02bb / 0x02f3.
    fn effect_tick(&mut self, bus: &mut impl MusicBus, slot: u16) {
        self.c(bus, 19 + 19 + 4 + 4); // ld h,(iy+3); ld l,(iy+4); ld a,h; or l
        let countdown = self.rd16be(slot + 3);
        if countdown == 0 {
            self.c(bus, 12 + 7); // jr z; ld b,8
            for k in 0..8u8 {
                let ch = 7 - k;
                // ld a,b 4; dec a 4; ld (0xc022),a 13; push bc 11; bit 0,(iy+0) 20; call nz 17 / 10
                self.c(bus, 4 + 4 + 13 + 11 + 20);
                self.wr(C022_EFFECT_CHANNEL, ch);
                if self.rd(slot) & 1 != 0 {
                    self.c(bus, 17);
                    if self.effect_events(bus, slot, slot + 0x10 + 8 * k as u16, ch) {
                        return;
                    }
                } else {
                    self.c(bus, 10);
                }
                // rrc (iy+0) 23; ld bc,8 10; add ix,bc 15; pop bc 10; djnz 13 / 8
                self.c(bus, 23 + 10 + 15 + 10 + if k < 7 { 13 } else { 8 });
                let m = self.rd(slot).rotate_right(1);
                self.wr(slot, m);
            }
            self.c(bus, 10); // ret
            return;
        }
        // 0x0ac3: jr z not taken 7; dec hl 6; ld (iy+3),h 19; ld (iy+4),l 19; ld b,8 7
        self.c(bus, 7 + 6 + 19 + 19 + 7);
        self.wr16be(slot + 3, countdown - 1);
        for k in 0..8u8 {
            let ch = 7 - k;
            self.c(bus, 4 + 4 + 13 + 11 + 20);
            self.wr(C022_EFFECT_CHANNEL, ch);
            if self.rd(slot) & 1 != 0 {
                self.c(bus, 17);
                self.effect_slide(bus, slot + 0x10 + 8 * k as u16, ch);
            } else {
                self.c(bus, 10);
            }
            self.c(bus, 23 + 10 + 15 + 10 + if k < 7 { 13 } else { 8 });
            let m = self.rd(slot).rotate_right(1);
            self.wr(slot, m);
        }
        // 0x0ae5: ld a,(iy+7) 19; or a 4; ret z 11 / 5
        self.c(bus, 19 + 4);
        let inc = self.rd(slot + 7);
        if inc == 0 {
            self.c(bus, 11);
            return;
        }
        // add a,(iy+6) 19; ld (iy+6),a 19; rrca x3 12; and 0x1f 7; or 0x80 7;
        // ld d,a 4; ld a,0x0f 7; call 0x00fc 17; ret 10
        self.c(bus, 5 + 19 + 19 + 12 + 7 + 7 + 4 + 7 + 17);
        let acc = self.rd(slot + 6).wrapping_add(inc);
        self.wr(slot + 6, acc);
        self.opm_write(bus, 0x0f, 0x80 | (acc.rotate_right(3) & 0x1f));
        self.c(bus, 10);
    }

    /// 0x0afe: one channel's slide step: the signed step (record +2) added
    /// to note.fraction (+0.+1); a changed integer part re-keys the note
    /// (0x0b9f, without a key-off); then KF (0x30 + ch) = the fraction.
    fn effect_slide(&mut self, bus: &mut impl MusicBus, rec: u16, ch: u8) {
        self.c(bus, 19 + 4); // ld a,(ix+2); or a
        let step = self.rd(rec + 2);
        if step == 0 {
            self.c(bus, 11);
            self.observations.zero_slides += 1;
            let command = self.rd(C001_LAST_CMD);
            if command < 64 {
                self.observations.zero_slide_commands |= 1 << command;
            }
            return;
        }
        // ret z not taken 5; ld h,(ix+0) 19; ld l,(ix+1) 19; ld d,0 7; ld e,a 4;
        // bit 7,e 8; jr z 12 / not taken 7 + dec d 4
        self.c(bus, 5 + 19 + 19 + 7 + 4 + 8);
        let hl = u16::from_be_bytes([self.rd(rec), self.rd(rec + 1)]);
        let de = if step & 0x80 != 0 {
            self.c(bus, 7 + 4);
            0xff00 | step as u16
        } else {
            self.c(bus, 12);
            step as u16
        };
        // add hl,de 11; ld (ix+1),l 19; ld a,h 4; cp (ix+0) 19; jr z 12 / 7
        self.c(bus, 11 + 19 + 4 + 19);
        let hl = hl.wrapping_add(de);
        self.wr(rec + 1, hl as u8);
        if (hl >> 8) as u8 != self.rd(rec) {
            self.c(bus, 7 + 19 + 17); // ld (ix+0),a; call 0x0b9f
            self.wr(rec, (hl >> 8) as u8);
            self.effect_key_on(bus, rec, ch);
        } else {
            self.c(bus, 12);
        }
        // 0x0b21: ld d,(ix+1) 19; ld a,(0xc022) 13; add 0x30 7; call 17; ret 10
        self.c(bus, 19 + 13 + 7 + 17);
        let f = self.rd(rec + 1);
        self.opm_write(bus, 0x30 + ch, f);
        self.c(bus, 10);
    }

    /// 0x0b49: one channel's events: stream bytes until a note (0x81-0xfe,
    /// then a slide-step byte), a rest (0x80, a discarded byte) or 0xff (a
    /// slide-step byte, no key event). Bytes below 0x80 are opcodes 0-5
    /// (0x0e62 dispatch), read in a loop. Returns true when the effect ended
    /// (opcode 5). Charges the final `ret`.
    fn effect_events(&mut self, bus: &mut impl MusicBus, slot: u16, rec: u16, ch: u8) -> bool {
        loop {
            self.c(bus, 17); // call 0x0fa0
            let b = self.effect_byte(bus, slot);
            self.c(bus, 8 + 10); // bit 7,a; jp z
            if b & 0x80 == 0 {
                // 0x0e62: call 0x0e68 17; rst 0x30 through the table at 0x0e69
                // (rst 11, pop hl 10, rst 0x28 11 + 4 + 41 + 26 + 10, ex de,hl 4,
                // jp (hl) 4); the handler's ret 10 and jp 0x0b49 10 follow.
                self.c(bus, 17 + 11 + 10 + 11 + 4 + 41 + 26 + 10 + 4 + 4);
                if self.effect_opcode(bus, slot, rec, ch, b) {
                    return true;
                }
                self.c(bus, 10 + 10);
                continue;
            }
            self.c(bus, 7); // cp 0xff
            if b == 0xff {
                // jr nz not taken 7; call 0x0fa0 17; ld (ix+2),a 19; ret 10
                self.c(bus, 7 + 17);
                let step = self.effect_byte(bus, slot);
                self.c(bus, 19 + 10);
                self.wr(rec + 2, step);
                return false;
            }
            // jr nz 12; push af 11; ld a,(0xc022) 13; and 7 7; ld c,a 4; call 0x00b9 17;
            // pop af 10; and 0x7f 7; jr nz 12 / 7
            self.c(bus, 12 + 11 + 13 + 7 + 4 + 17);
            self.key_off(bus, ch);
            self.c(bus, 10 + 7);
            let note = b & 0x7f;
            if note == 0 {
                // 0x0b6b rest: ld (ix+0),a 19; call 0x0fa0 17; xor a 4; ld (ix+1),a 19;
                // ld (ix+2),a 19; ld b,0x30 7; ld a,(0xc022) 13; add a,b 4; ld d,0 7;
                // call 0x00fc 17; ret 10
                self.c(bus, 7 + 19 + 17);
                self.wr(rec, 0);
                self.effect_byte(bus, slot);
                self.c(bus, 4 + 19 + 19 + 7 + 13 + 4 + 7 + 17);
                self.wr(rec + 1, 0);
                self.wr(rec + 2, 0);
                self.opm_write(bus, 0x30 + ch, 0);
                self.c(bus, 10);
                return false;
            }
            // 0x0b84 note: ld (ix+0),a 19; call 0x0fa0 17; ld (ix+2),a 19; xor a 4;
            // ld (ix+1),a 19; ld b,0x30 7; ld a,(0xc022) 13; add a,b 4; ld d,0 7;
            // call 0x00fc 17; call 0x0ebc 17; then 0x0b9f (its ret ends the read).
            self.c(bus, 12 + 19 + 17);
            self.wr(rec, note);
            let step = self.effect_byte(bus, slot);
            self.c(bus, 19 + 4 + 19 + 7 + 13 + 4 + 7 + 17);
            self.wr(rec + 2, step);
            self.wr(rec + 1, 0);
            self.opm_write(bus, 0x30 + ch, 0);
            self.c(bus, 17);
            self.effect_patch_load(bus, rec, ch);
            self.effect_key_on(bus, rec, ch);
            return false;
        }
    }

    /// 0x0e75..0x0f7c: the effect opcodes (the dispatch was charged by the
    /// caller): 0 countdown = 0x00nn (the high byte is the opcode byte
    /// itself), 1 noise register and accumulator / increment, 2 the RL bits
    /// of register 0x20 + ch, 3 the patch number for the next note, 4 jump
    /// (count 0) or loop `count` times through slot + 0xf, 5 the end: three
    /// pops, 0xc024 set around the slot's stop (0x02bb / 0x02f3), the `ret`
    /// to the slot-B check (slot A) or the handler (slot B). Returns true
    /// when the effect ended. Opcode bytes 6-0x7f jump through code bytes in
    /// the original and are reported.
    fn effect_opcode(
        &mut self,
        bus: &mut impl MusicBus,
        slot: u16,
        rec: u16,
        ch: u8,
        b: u8,
    ) -> bool {
        match b {
            0 => {
                // ld (iy+3),b 19; call 0x0fa0 17; ld (iy+4),a 19 (ret charged by the caller)
                self.c(bus, 19 + 17);
                self.wr(slot + 3, b);
                let n = self.effect_byte(bus, slot);
                self.c(bus, 19);
                self.wr(slot + 4, n);
            }
            1 => {
                self.c(bus, 17);
                let v = self.effect_byte(bus, slot);
                self.c(bus, 7); // cp 0xff
                if v != 0xff {
                    // jr z not taken 7; ld d,a 4; ld a,0x0f 7; call 17; ld a,d 4;
                    // and 0x1f 7; rlca x3 12; ld (iy+6),a 19
                    self.c(bus, 7 + 4 + 7 + 17);
                    self.opm_write(bus, 0x0f, v);
                    self.c(bus, 4 + 7 + 12 + 19);
                    self.wr(slot + 6, (v & 0x1f) << 3);
                } else {
                    self.c(bus, 12);
                }
                self.c(bus, 17);
                let inc = self.effect_byte(bus, slot);
                self.c(bus, 19);
                self.wr(slot + 7, inc);
            }
            2 => {
                // call 0x0fa0 17; rrca x2 8; ld b,a 4; ld a,(ix+6) 19; and 0x3f 7; or b 4;
                // ld (ix+6),a 19; ld d,a 4; ld a,(0xc022) 13; add 0x20 7; call 17
                self.c(bus, 17);
                let v = self.effect_byte(bus, slot);
                self.c(bus, 8 + 4 + 19 + 7 + 4 + 19 + 4 + 13 + 7 + 17);
                let a = (self.rd(rec + 6) & 0x3f) | v.rotate_right(2);
                self.wr(rec + 6, a);
                self.opm_write(bus, 0x20 + ch, a);
            }
            3 => {
                self.c(bus, 17);
                let p = self.effect_byte(bus, slot);
                self.c(bus, 19);
                self.wr(rec + 3, p);
            }
            4 => {
                self.c(bus, 17);
                let count = self.effect_byte(bus, slot);
                self.c(bus, 4); // or a
                let jump = if count == 0 {
                    self.c(bus, 12);
                    true
                } else {
                    // jr z not taken 7; inc (iy+0xf) 23; cp (iy+0xf) 19; jr nz 12 / 7
                    self.c(bus, 7 + 23 + 19);
                    let n = self.rd(slot + 0xf).wrapping_add(1);
                    self.wr(slot + 0xf, n);
                    if n != count {
                        self.c(bus, 12);
                        true
                    } else {
                        self.c(bus, 7 + 4 + 19 + 12); // xor a; ld (iy+0xf),a; jr 0x0f6c
                        self.wr(slot + 0xf, 0);
                        false
                    }
                };
                if jump {
                    // 0x0f5e: call 17; ld e,a 4; call 17; ld (iy+2),e 19; ld (iy+1),a 19
                    self.c(bus, 17);
                    let lo = self.effect_byte(bus, slot);
                    self.c(bus, 4 + 17);
                    let hi = self.effect_byte(bus, slot);
                    self.c(bus, 19 + 19);
                    self.wr16be(slot + 1, u16::from_be_bytes([hi, lo]));
                } else {
                    // 0x0f6c: ld h 19; ld l 19; ld de,2 10; add hl,de 11; ld (iy+1),h 19; ld (iy+2),l 19
                    self.c(bus, 19 + 19 + 10 + 11 + 19 + 19);
                    let p = self.rd16be(slot + 1).wrapping_add(2);
                    self.wr16be(slot + 1, p);
                }
            }
            5 => {
                // pop hl 10; pop hl 10; pop bc 10; ld a,1 7; ld (0xc024),a 13; push iy 15;
                // pop hl 10; ld bc,0xc500 10; or a 4; sbc hl,bc 15; jr nz 12 / 7; call 17
                self.c(bus, 10 + 10 + 10 + 7 + 13 + 15 + 10 + 10 + 4 + 15);
                self.wr(C024_EFFECT_ENDING, 1);
                self.c(bus, if slot == EFFECT_SLOT_A { 7 } else { 12 } + 17);
                self.stop_effect(bus, slot);
                self.c(bus, 4 + 13 + 10); // xor a; ld (0xc024),a; ret
                self.wr(C024_EFFECT_ENDING, 0);
                return true;
            }
            _ => {
                self.note(0x0e62);
                return true;
            }
        }
        false
    }

    /// 0x0fa0: the slot's next stream byte (A = B = *pointer++; 103 T).
    fn effect_byte(&mut self, bus: &mut impl MusicBus, slot: u16) -> u8 {
        self.c(bus, 19 + 19 + 7 + 4 + 6 + 19 + 19 + 10);
        let hl = self.rd16be(slot + 1);
        let a = self.rd(hl);
        self.wr16be(slot + 1, hl.wrapping_add(1));
        a
    }

    /// 0x0ebc: load the record's patch (+3) into the channel: the pointer
    /// 0x6b00 + 0x29 x patch kept at +4/+5; the LFO block (patch + 2 bit 7)
    /// written directly, without the 0xc015 copies the music keeps; register
    /// 0x20 + ch = the record's RL bits (+6, from opcode 2) with the patch's
    /// FB/CON, 0x38 + ch = PMS/AMS, then the 20 operator registers 0x40-0x58,
    /// 0x80-0xf8. All writes direct. Charges its `ret`.
    fn effect_patch_load(&mut self, bus: &mut impl MusicBus, rec: u16, ch: u8) {
        // ld a,(ix+3) 19; ld h,a 4; ld l,0 7; ld d,0 7; ld e,0x29 7; call 0x010e 17
        self.c(bus, 19 + 4 + 7 + 7 + 7 + 17);
        let p = self.rd(rec + 3);
        let patch = self.mul8(bus, p, 0x29).wrapping_add(0x6b00);
        // ld de,0x6b00 10; add hl,de 11; ld (ix+4),h 19; ld (ix+5),l 19; ld bc,2 10;
        // add hl,bc 11; bit 7,(hl) 12; jr nz 12 / 7
        self.c(bus, 10 + 11 + 19 + 19 + 10 + 11 + 12);
        self.wr16be(rec + 4, patch);
        let mut hl = patch.wrapping_add(2);
        if self.rd(hl) & 0x80 != 0 {
            // ld a,(hl) 7; rlca x3 12; and 3 7; ld d,a 4; ld a,0x1b 7; call 17
            self.c(bus, 12 + 7 + 12 + 7 + 4 + 7 + 17);
            let w = (self.rd(hl) >> 5) & 3;
            self.opm_write(bus, 0x1b, w);
            self.c(bus, 6 + 7 + 7 + 17); // inc hl; ld d,(hl); ld a,0x18; call
            hl += 1;
            let f = self.rd(hl);
            self.opm_write(bus, 0x18, f);
            self.c(bus, 6 + 7 + 7 + 4 + 7 + 17); // inc hl; ld a,(hl); or 0x80; ld d,a; ld a,0x19; call
            hl += 1;
            let pm = self.rd(hl) | 0x80;
            self.opm_write(bus, 0x19, pm);
            self.c(bus, 6 + 7 + 7 + 17); // inc hl; ld d,(hl); ld a,0x19; call
            hl += 1;
            let am = self.rd(hl);
            self.opm_write(bus, 0x19, am);
        } else {
            self.c(bus, 7 + 10 + 11 + 12); // ld de,3; add hl,de; jr 0x0f05
            hl = hl.wrapping_add(3);
        }
        // 0x0f05: inc hl 6; ld a,(hl) 7; and 0x3f 7; ld b,a 4; ld a,(ix+6) 19; and 0xc0 7;
        // or b 4; ld (ix+6),a 19; ld d,a 4; ld a,(0xc022) 13; add 0x20 7; call 17
        self.c(bus, 6 + 7 + 7 + 4 + 19 + 7 + 4 + 19 + 4 + 13 + 7 + 17);
        hl += 1;
        let a = (self.rd(rec + 6) & 0xc0) | (self.rd(hl) & 0x3f);
        self.wr(rec + 6, a);
        self.opm_write(bus, 0x20 + ch, a);
        // inc hl 6; ld d,(hl) 7; ld a,(0xc022) 13; add 0x38 7; call 17
        self.c(bus, 6 + 7 + 13 + 7 + 17);
        hl += 1;
        let v = self.rd(hl);
        self.opm_write(bus, 0x38 + ch, v);
        // ld bc,0xe 10; add hl,bc 11; ld a,(0xc022) 13; add 0x40 7; ld b,0x14 7
        self.c(bus, 10 + 11 + 13 + 7 + 7);
        hl = hl.wrapping_add(0x0e);
        let mut reg = 0x40 + ch;
        for i in 0..20u8 {
            // push bc 11; ld d,(hl) 7; call 0x00fc 17; add 8 7; ld e,a 4; pop bc 10;
            // ld a,b 4; cp 0x11 7; call z,0x0f45 17 (ld a,0x20 7; add a,e 4; ld e,a 4;
            // ret 10) / 10; ld a,e 4; inc hl 6; djnz 13 / 8
            self.c(bus, 11 + 7 + 17);
            let v = self.rd(hl);
            self.opm_write(bus, reg, v);
            self.c(bus, 7 + 4 + 10 + 4 + 7);
            reg = reg.wrapping_add(8);
            if i == 3 {
                self.c(bus, 17 + 7 + 4 + 4 + 10);
                reg = reg.wrapping_add(0x20);
            } else {
                self.c(bus, 10);
            }
            self.c(bus, 4 + 6 + if i < 19 { 13 } else { 8 });
            hl += 1;
        }
        self.c(bus, 10); // ret
    }

    /// 0x0b9f: key the record's note on: transposed by the patch's first
    /// byte and folded into 0x00..0x60 by octaves — the folded value
    /// replaces the record's note, so a slide's re-keys transpose again —
    /// the key code from 0x0fb0 into 0xc002, the four operator levels
    /// (0x0bfc: the music's computation with direct writes and the player's
    /// copies of the tables), KC (0x28 + ch), key on (0x08 = ch | patch + 8),
    /// and the LFO reset when patch + 2 bit 1 is set. No KF write. Charges
    /// its `ret`.
    fn effect_key_on(&mut self, bus: &mut impl MusicBus, rec: u16, ch: u8) {
        // ld a,(ix+0) 19; ld h,(ix+4) 19; ld l,(ix+5) 19; add a,(hl) 7; jp m 10
        self.c(bus, 19 + 19 + 19 + 7 + 10);
        let patch = self.rd16be(rec + 4);
        let mut a = self.rd(rec).wrapping_add(self.rd(patch));
        if (a as i8) < 0 {
            loop {
                self.c(bus, 7 + 10); // add a,0x0c; jp m
                a = a.wrapping_add(0x0c);
                if (a as i8) >= 0 {
                    break;
                }
            }
        } else {
            loop {
                self.c(bus, 7); // cp 0x61
                if a < 0x61 {
                    self.c(bus, 12); // jr c taken
                    break;
                }
                self.c(bus, 7 + 7 + 12); // jr c not taken; sub 0x0c; jr
                a -= 0x0c;
            }
        }
        // 0x0bb9: ld (ix+0),a 19; ld hl,0x0fb0 10; rst 0x20; ld (0xc002),a 13; call 0x0bfc 17
        self.c(bus, 19 + 10);
        self.wr(rec, a);
        let kc = self.rst20(bus, 0x0fb0, a);
        self.c(bus, 13 + 17);
        self.wr(C002_KEYCODE, kc);
        self.levels(bus, patch, EFFECT_LEVELS);
        // 0x0bc6: ld a,(0xc002) 13; ld d,a 4; ld a,(0xc022) 13; add 0x28 7; call 17
        self.c(bus, 13 + 4 + 13 + 7 + 17);
        self.opm_write(bus, 0x28 + ch, kc);
        // 0x0bd2: ld h 19; ld l 19; push hl 11; ld bc,8 10; add hl,bc 11; ld a,(0xc022) 13;
        // or (hl) 7; ld d,a 4; ld a,8 7; call 17
        self.c(bus, 19 + 19 + 11 + 10 + 11 + 13 + 7 + 4 + 7 + 17);
        let mask = self.rd(patch.wrapping_add(8));
        self.opm_write(bus, 0x08, ch | mask);
        // 0x0be7: pop hl 10; ld bc,2 10; add hl,bc 11; bit 1,(hl) 12; ret z 11 / 5
        self.c(bus, 10 + 10 + 11 + 12);
        if self.rd(patch.wrapping_add(2)) & 2 == 0 {
            self.c(bus, 11);
            return;
        }
        // ld d,2 7; ld a,1 7; call 17; ld d,0 7; call 17; ret 10
        self.c(bus, 5 + 7 + 7 + 17);
        self.opm_write(bus, 0x01, 0x02);
        self.c(bus, 7 + 17);
        self.opm_write(bus, 0x01, 0x00);
        self.c(bus, 10);
    }

    // ---- sequencer -------------------------------------------------------------------

    /// 0x0424: one sequencer tick. A tick a tune's end (opcode 0xf) unwinds
    /// continues at 0x0440 after 0x03a7 / 0x0412 when it was the 0xc100
    /// channels' (their `call 0x0453` left its return there), and ends when
    /// it was the saved channels' (entered by fall-through: the `ret` is
    /// the handler's).
    fn music_tick(&mut self, bus: &mut impl MusicBus) {
        self.c(bus, 10 + 11 + 14 + 19 + 14 + 19 + 13 + 14 + 17);
        let t = self.rd(C013_TICKS).wrapping_add(1);
        self.wr(C013_TICKS, t);
        let owned = self.rd(EFFECT_SLOT_A) | self.rd(EFFECT_SLOT_B);
        self.wr(C023_OWNED, owned);
        match self.tick_channels(bus, MUSIC_CHANNELS) {
            Flow::Done => {}
            Flow::TuneEnd => self.pop_saved(bus),
            Flow::SavedEnd => self.discard_saved(bus),
        }
        // 0x0440
        self.c(bus, 4 + 13 + 13 + 8);
        self.wr(C023_OWNED, 0);
        if self.rd(C011_MUSIC) & 0x40 == 0 {
            self.c(bus, 11);
            return;
        }
        // The saved tune keeps time silently.
        self.c(bus, 5 + 7 + 13 + 14);
        self.wr(C010_SILENT, 1);
        match self.tick_channels(bus, SAVED_CHANNELS) {
            Flow::Done => {}
            Flow::TuneEnd => self.pop_saved(bus),
            Flow::SavedEnd => self.discard_saved(bus),
        }
    }

    /// 0x0453: tick eight channels from `base` (charges its `ret`). A tune
    /// end abandons the loop: the remaining channels, the loop tail and the
    /// 0x0477 clear are skipped.
    fn tick_channels(&mut self, bus: &mut impl MusicBus, base: u16) -> Flow {
        self.c(bus, 7);
        let mut ix = base;
        for ch in 0..8u8 {
            self.c(bus, 7 + 4 + 13 + 11 + 23 + 19 + 19);
            self.wr(C014_CHANNEL, ch);
            let n = self.rd(ix + 7).wrapping_add(1);
            self.wr(ix + 7, n);
            if self.rd(ix) | self.rd(ix + 1) != 0 {
                self.c(bus, 17);
                let flow = self.channel_tick(bus, ix);
                if flow != Flow::Done {
                    return flow;
                }
            } else {
                self.c(bus, 10);
            }
            ix += 0x30;
            let o = self.rd(C023_OWNED).rotate_left(1);
            self.wr(C023_OWNED, o);
            self.c(
                bus,
                10 + 15 + 13 + 4 + 13 + 10 + if ch < 7 { 13 } else { 8 },
            );
        }
        self.c(bus, 4 + 13 + 10);
        self.wr(C010_SILENT, 0);
        Flow::Done
    }

    /// 0x047c: a channel's tick (twice on odd ticks in double-speed mode;
    /// the first of the two is a `call`, nesting its event read).
    fn channel_tick(&mut self, bus: &mut impl MusicBus, ix: u16) -> Flow {
        self.c(bus, 20);
        if self.rd(ix + 0x0b) & 0x80 == 0 {
            self.c(bus, 12);
        } else {
            self.c(bus, 7 + 13 + 7);
            if self.rd(C013_TICKS) & 1 == 0 {
                self.c(bus, 12);
            } else {
                self.c(bus, 7 + 17);
                self.nesting += 1;
                let flow = self.count_down(bus, ix);
                self.nesting -= 1;
                if flow != Flow::Done {
                    return flow;
                }
            }
        }
        self.count_down(bus, ix)
    }

    /// 0x048c: count the note down; at zero, read the next event (charges
    /// the `ret`).
    fn count_down(&mut self, bus: &mut impl MusicBus, ix: u16) -> Flow {
        if self.raw_channel(ix) {
            return self.raw_tick(bus, ix);
        }
        self.c(bus, 19 + 19 + 4 + 4);
        let hl = self.rd16be(ix + 2);
        if hl == 0 {
            self.c(bus, 12);
            return self.next_event(bus, ix);
        }
        self.c(bus, 7 + 6 + 19 + 19 + 11 + 17);
        let hl = hl - 1;
        self.wr16be(ix + 2, hl);
        self.gate(bus, ix);
        self.c(bus, 10 + 4 + 4);
        if hl != 0 {
            self.c(bus, 11);
            return Flow::Done;
        }
        self.c(bus, 5 + 12);
        self.next_event(bus, ix)
    }

    /// 0x04a7: the gate counter ends the sound before the note's length.
    fn gate(&mut self, bus: &mut impl MusicBus, ix: u16) {
        self.c(bus, 19 + 7);
        if self.rd(ix + 0x0c) & 0x0f != 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 19 + 19 + 4 + 4);
        let hl = self.rd16be(ix + 8);
        if hl == 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 6 + 19 + 19 + 4 + 4);
        let hl = hl - 1;
        self.wr16be(ix + 8, hl);
        if hl != 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 13 + 4);
        if self.rd(C010_SILENT) != 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 13 + 7 + 4 + 10);
        let c = self.rd(C014_CHANNEL) & 7;
        self.key_off_ch(bus, c);
    }

    /// 0x0a1a: the channel's next stream byte.
    fn read_byte(&mut self, bus: &mut impl MusicBus, ix: u16) -> u8 {
        self.c(bus, 19 + 19 + 7 + 4 + 6 + 19 + 19 + 10);
        let hl = self.rd16be(ix);
        let a = self.rd(hl);
        self.wr16be(ix, hl.wrapping_add(1));
        a
    }

    /// 0x04ce: clear double speed and read events.
    fn next_event(&mut self, bus: &mut impl MusicBus, ix: u16) -> Flow {
        self.c(bus, 23);
        let f = self.rd(ix + 0x0b) & 0x7f;
        self.wr(ix + 0x0b, f);
        self.read_events(bus, ix)
    }

    /// 0x04d2: read events until a note, rest, or tied note is set (charges
    /// the final `ret`), or until opcode 0xf unwinds the read.
    fn read_events(&mut self, bus: &mut impl MusicBus, ix: u16) -> Flow {
        loop {
            self.c(bus, 17);
            let b = self.read_byte(bus, ix);
            self.c(bus, 7 + 10); // and 0xf0; jp z
            match b & 0xf0 {
                0x00 => {
                    // 0x085b: call 0x0861, then jp 0x04d2.
                    self.c(bus, 17);
                    if let Some(flow) = self.opcode(bus, ix, b) {
                        return flow;
                    }
                    self.c(bus, 10);
                }
                0x20 => {
                    // 0x04ef: a tie count for the following note (a nested
                    // read).
                    self.c(bus, 7 + 12 + 4 + 7 + 11 + 19 + 17);
                    let count = b & 0x0f;
                    self.wr(ix + 0x0c, 0);
                    self.nesting += 1;
                    let flow = self.read_events(bus, ix);
                    self.nesting -= 1;
                    if flow != Flow::Done {
                        return flow;
                    }
                    self.c(bus, 10 + 19 + 10);
                    self.wr(ix + 0x0c, count);
                    return Flow::Done;
                }
                0x30 => {
                    // 0x04ff: double speed for the following event.
                    self.c(bus, 7 + 7 + 7 + 12 + 23 + 12);
                    let f = self.rd(ix + 0x0b) | 0x80;
                    self.wr(ix + 0x0b, f);
                }
                _ => {
                    // 0x04e2: a note or rest; a pending tie counts down.
                    self.c(bus, 7 + 7 + 7 + 7 + 19 + 7);
                    let t = self.rd(ix + 0x0c) & 0x0f;
                    if t != 0 {
                        self.c(bus, 7 + 4 + 19 + 12);
                        self.wr(ix + 0x0c, t - 1);
                    } else {
                        self.c(bus, 12);
                    }
                    self.c(bus, 10); // ld hl,0x0a2a
                    self.note_event(bus, ix, b, 0x0a2a);
                    return Flow::Done;
                }
            }
        }
    }

    /// 0x0508: a note or rest event `b`, its length from the table at
    /// `lengths` (charges the final `ret`).
    fn note_event(&mut self, bus: &mut impl MusicBus, ix: u16, b: u8, lengths: u16) {
        self.c(bus, 11 + 17);
        self.set_duration(bus, ix, b, lengths);
        self.c(bus, 10 + 4 + 7);
        let pitch = b & 0x1f;
        if pitch == 0 {
            // A rest.
            self.c(bus, 7 + 19 + 13 + 4);
            self.wr(ix + 0x0e, 0);
            if self.rd(C010_SILENT) != 0 {
                self.c(bus, 11);
                return;
            }
            self.c(bus, 5 + 19 + 7);
            if self.rd(ix + 0x0c) & 0x0f != 0 {
                self.c(bus, 11);
                return;
            }
            self.c(bus, 5 + 13 + 7 + 4 + 10);
            let c = self.rd(C014_CHANNEL) & 7;
            self.key_off_ch(bus, c);
            return;
        }
        // 0x0529: transpose by the channel and the patch, folded into
        // 0x00..=0x60 by octaves.
        self.c(bus, 12 + 19 + 19 + 19 + 7 + 10);
        let patch = self.rd16be(ix + 0x18);
        let mut a = pitch
            .wrapping_add(self.rd(ix + 0x0d))
            .wrapping_add(self.rd(patch));
        if (a as i8) < 0 {
            loop {
                self.c(bus, 7 + 10);
                a = a.wrapping_add(0x0c);
                if (a as i8) >= 0 {
                    break;
                }
            }
        } else {
            loop {
                self.c(bus, 7);
                if a < 0x61 {
                    self.c(bus, 12);
                    break;
                }
                self.c(bus, 7 + 7 + 12);
                a -= 0x0c;
            }
        }
        // 0x0543
        self.c(bus, 19 + 13 + 4);
        self.wr(ix + 0x0e, a);
        if self.rd(C010_SILENT) != 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 19 + 7);
        if self.rd(ix + 0x0c) & 0x0f != 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5);
        self.key_on(bus, ix);
    }

    /// 0x0801: the note's duration (length unit x the length-table entry for
    /// bits 5-7) and gate (duration x gate % / 255).
    fn set_duration(&mut self, bus: &mut impl MusicBus, ix: u16, b: u8, lengths: u16) {
        self.c(bus, 4 + 4 + 4 + 4 + 4 + 7);
        let len = self.rst20(bus, lengths, (b >> 5) & 7);
        self.c(bus, 19 + 7 + 7 + 4 + 17);
        let dur = self.mul8(bus, self.rd(ix + 4), len);
        self.c(bus, 19 + 19 + 19 + 7);
        self.wr16be(ix + 2, dur);
        if self.rd(ix + 0x0c) & 0x0f != 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 4 + 7);
        if b & 0x1f == 0 {
            self.c(bus, 7 + 19 + 19 + 10);
            self.wr(ix + 8, 0);
            self.wr(ix + 9, 0);
            return;
        }
        // 0x082c
        self.c(bus, 12 + 10 + 17);
        let (q, r) = self.div16(bus, dur, 0x00ff);
        self.c(bus, 17);
        let f = self.frac8(bus, r, 0x00ff);
        self.c(bus, 4 + 7 + 7 + 19 + 17);
        let pct = self.rd(ix + 0x0a);
        let mut hl = self.mul8(bus, q as u8, pct);
        self.c(bus, 4);
        if f != 0 {
            self.c(bus, 7 + 11 + 19 + 7 + 7 + 4 + 17);
            let p = self.mul8(bus, pct, f);
            self.c(bus, 7 + 4 + 10 + 11);
            hl = hl.wrapping_add(p >> 8);
        } else {
            self.c(bus, 12);
        }
        self.c(bus, 19 + 19 + 10);
        self.wr16be(ix + 8, hl);
    }

    /// 0x010e: HL = H * E (L = D = 0 at every call); charges its body.
    fn mul8(&mut self, bus: &mut impl MusicBus, h: u8, e: u8) -> u16 {
        self.c(bus, 7);
        let mut hl = (h as u16) << 8;
        for i in 0..8 {
            let carry = hl & 0x8000 != 0;
            hl <<= 1;
            if carry {
                self.c(bus, 11 + 7 + 11);
                hl = hl.wrapping_add(e as u16);
            } else {
                self.c(bus, 11 + 12);
            }
            self.c(bus, if i < 7 { 13 } else { 8 });
        }
        self.c(bus, 10);
        hl
    }

    /// 0x0155: (quotient, remainder) of HL / BC by shift-and-subtract.
    fn div16(&mut self, bus: &mut impl MusicBus, hl: u16, bc: u16) -> (u16, u16) {
        self.c(bus, 10 + 4);
        if bc == 0 {
            self.note(0x0159);
            return (0, hl);
        }
        let mut bc = bc;
        let mut a: u8 = 0;
        loop {
            a += 1;
            self.c(bus, 4 + 8);
            if bc & 0x8000 != 0 {
                self.c(bus, 12);
                break;
            }
            self.c(bus, 7 + 8 + 8 + 12);
            bc <<= 1;
        }
        let (mut hl, mut de) = (hl, 0u16);
        loop {
            self.c(bus, 4 + 15);
            if hl >= bc {
                self.c(bus, 12 + 8);
                hl -= bc;
                de |= 1;
            } else {
                self.c(bus, 7 + 11 + 12);
            }
            self.c(bus, 4);
            a -= 1;
            if a == 0 {
                self.c(bus, 12);
                break;
            }
            self.c(bus, 7 + 8 + 8 + 8 + 8 + 12);
            de <<= 1;
            bc >>= 1;
        }
        self.c(bus, 4 + 10);
        (de, hl)
    }

    /// 0x017d: A = DE * 256 / BC (8 quotient bits), 0 when DE = 0.
    fn frac8(&mut self, bus: &mut impl MusicBus, de: u16, bc: u16) -> u8 {
        self.c(bus, 4 + 4);
        if de == 0 {
            self.c(bus, 11);
            return 0;
        }
        self.c(bus, 5 + 11 + 11 + 4 + 11 + 10 + 4 + 7);
        let mut hl = de;
        let mut a: u8 = 0;
        for i in 0..8 {
            self.c(bus, 8 + 8 + 8 + 4 + 15);
            a <<= 1;
            hl = hl.wrapping_shl(1);
            if hl >= bc {
                self.c(bus, 12 + 8);
                hl -= bc;
                a |= 1;
            } else {
                self.c(bus, 7 + 11 + 12);
            }
            self.c(bus, if i < 7 { 13 } else { 8 });
        }
        self.c(bus, 10 + 10 + 10);
        a
    }

    /// 0x0861: opcodes 0x00-0x0f (the caller charged `call 0x0861`).
    /// Returns how the event read ends when it does (opcode 6 plays a note;
    /// opcode 0xf unwinds), None to keep reading.
    fn opcode(&mut self, bus: &mut impl MusicBus, ix: u16, b: u8) -> Option<Flow> {
        // ld a,b; rst 0x30 (rst 11, pop hl 10, rst 0x28: rst 11, add a,a 4,
        // rst 0x18 41, ld/inc/ld/inc 26, ret 10; ex de,hl 4; jp (hl) 4).
        self.c(bus, 4 + 11 + 10 + 11 + 4 + 41 + 26 + 10 + 4 + 4);
        match b {
            0x0..=0x4 => {
                self.c(bus, 17);
                let v = self.read_byte(bus, ix);
                let field = [4u16, 6, 0x0a, 0x0d, 0x12][b as usize];
                self.wr(ix + field, v);
                if b == 1 {
                    self.c(bus, 19 + 4 + 19 + 10);
                    self.wr(ix + 7, 0);
                } else {
                    self.c(bus, 19 + 10);
                }
            }
            0x5 => {
                self.c(bus, 17);
                let v = self.read_byte(bus, ix);
                self.c(bus, 4);
                let f = self.rd(ix + 0x0b);
                if v != 0 {
                    self.c(bus, 7 + 23 + 10);
                    self.wr(ix + 0x0b, f | 1);
                } else {
                    self.c(bus, 12 + 23 + 10);
                    self.wr(ix + 0x0b, f & !1);
                }
            }
            0x6 => {
                // 0x08ba: the next byte is a note with the second length
                // table (0x0a32); this ends the event read.
                self.c(bus, 10 + 17);
                let n = self.read_byte(bus, ix);
                self.c(bus, 10 + 10);
                self.note_event(bus, ix, n, 0x0a32);
                return Some(Flow::Done);
            }
            0x7 => {
                // 0x08c4: select and load a patch.
                self.c(bus, 17);
                let p = self.read_byte(bus, ix);
                self.c(bus, 19 + 13 + 4);
                self.wr(ix + 0x17, p);
                if self.rd(C010_SILENT) != 0 {
                    self.c(bus, 11);
                } else {
                    self.c(bus, 5 + 4 + 4 + 7 + 7 + 7 + 17);
                    let hl = self.mul8(bus, p, 0x29).wrapping_add(0x6b00);
                    self.c(bus, 10 + 11 + 19 + 19);
                    self.wr16be(ix + 0x18, hl);
                    self.load_patch(bus, ix, hl);
                }
            }
            0x8 | 0xb => {
                self.c(bus, 17);
                self.read_byte(bus, ix);
                self.c(bus, 10);
            }
            0x9 => {
                // 0x096c: output (RL) bits.
                self.c(bus, 17);
                let v = self.read_byte(bus, ix).rotate_right(2);
                self.c(bus, 4 + 4 + 4 + 19 + 7 + 4 + 19 + 4 + 13 + 7 + 17);
                let a = self.raw_control(ix, (self.rd(ix + 0x1a) & 0x3f) | v);
                self.wr(ix + 0x1a, a);
                let reg = self.rd(C014_CHANNEL).wrapping_add(0x20);
                if !self.raw_channel(ix) || self.rd(C010_SILENT) == 0 {
                    self.write_ch(bus, reg, a);
                }
                self.c(bus, 10);
            }
            0xc..=0xe => {
                // 0x0989 / 0x099e / 0x09b3: repeat counters +0x2d..+0x2f.
                let counter = ix + 0x2d + (b - 0xc) as u16;
                self.c(bus, 17);
                let count = self.read_byte(bus, ix);
                self.c(bus, 4);
                let mut jump = true;
                if count == 0 {
                    self.c(bus, 12);
                } else {
                    self.c(bus, 7 + 4 + 23 + 19);
                    let n = self.rd(counter).wrapping_add(1);
                    self.wr(counter, n);
                    if count.wrapping_add(1) == n {
                        self.c(bus, 7 + 4 + 19 + 12);
                        self.wr(counter, 0);
                        jump = false;
                    } else {
                        self.c(bus, 12);
                    }
                }
                if jump {
                    // 0x09c8: jump to the following little-endian address.
                    self.c(bus, 17);
                    let lo = self.read_byte(bus, ix);
                    self.c(bus, 4 + 17);
                    let hi = self.read_byte(bus, ix);
                    self.c(bus, 19 + 19 + 10);
                    self.wr(ix, hi);
                    self.wr(ix + 1, lo);
                } else {
                    // 0x09d6: skip the address.
                    self.c(bus, 19 + 19 + 10 + 11 + 19 + 19 + 10);
                    let p = self.rd16be(ix).wrapping_add(2);
                    self.wr16be(ix, p);
                }
            }
            0xa => {
                if self.rom.len() > EXTENSION_BASE as usize
                    && (EXTENSION_BASE..EXTENSION_END).contains(&self.rd16be(ix))
                {
                    return self.raw_opcode(bus, ix);
                }
                self.note(0x0000);
                return Some(Flow::Done);
            }
            _ => {
                // 0x09e7: the end of the tune: three pops (10 each: the
                // returns to 0x085e and 0x0468, the channel loop's BC), ld
                // a,(0xc010) 13, or a 4, jp z,0x03a7 10 — the audible tune
                // ended: the saved tune is popped; else xor a 4, ld
                // (0xc010),a 13, jp 0x0412 10 — the silently advancing
                // saved tune ended: it is discarded. The pops assume the
                // event read is not nested in a `call` (the double-speed
                // extra count-down, a tie's nested read): there the
                // original's unwinding differs and is not translated.
                self.c(bus, 10 + 10 + 10 + 13 + 4);
                if self.nesting != 0 {
                    self.note(0x09e7);
                    return Some(Flow::Done);
                }
                if self.rd(C010_SILENT) == 0 {
                    self.c(bus, 10);
                    return Some(Flow::TuneEnd);
                }
                self.c(bus, 10 + 4 + 13 + 10);
                self.wr(C010_SILENT, 0);
                return Some(Flow::SavedEnd);
            }
        }
        None
    }

    /// 0x08e4: write a patch (0x29 bytes at `patch`) to the current channel:
    /// optional LFO registers (+2 bit 7), then its registers (0x0924).
    fn load_patch(&mut self, bus: &mut impl MusicBus, ix: u16, patch: u16) {
        self.c(bus, 10 + 11 + 12);
        let mut hl = patch.wrapping_add(2);
        if self.rd(hl) & 0x80 != 0 {
            self.c(bus, 12 + 7 + 4 + 4 + 4 + 7 + 13 + 4 + 7 + 17);
            let w = (self.rd(hl) >> 5) & 3;
            self.wr(0xc015, w);
            self.write_ch(bus, 0x1b, w);
            self.c(bus, 6 + 7 + 4 + 13 + 7 + 17);
            hl += 1;
            let f = self.rd(hl);
            self.wr(0xc016, f);
            self.write_ch(bus, 0x18, f);
            self.c(bus, 6 + 7 + 7 + 13 + 4 + 7 + 17);
            hl += 1;
            let p = self.rd(hl) | 0x80;
            self.wr(0xc017, p);
            self.write_ch(bus, 0x19, p);
            self.c(bus, 6 + 7 + 4 + 13 + 7 + 17);
            hl += 1;
            let am = self.rd(hl);
            self.wr(0xc018, am);
            self.write_ch(bus, 0x19, am);
        } else {
            self.c(bus, 7 + 10 + 11 + 12);
            hl += 3;
        }
        self.load_patch_registers(bus, ix, hl);
    }

    /// 0x0924: the patch's RL/FB/CON (+6), PMS/AMS (+7), then the 20
    /// operator registers 0x40-0x58, 0x80-0xf8 (+0x15..), from `hl` = patch
    /// + 5 (0x032b enters here, without the LFO block; charges the `ret`).
    fn load_patch_registers(&mut self, bus: &mut impl MusicBus, ix: u16, hl: u16) {
        let mut hl = hl;
        self.c(bus, 6 + 7 + 19 + 7 + 4 + 19 + 4 + 13 + 7 + 17);
        hl += 1;
        let c = self.rd(C014_CHANNEL);
        let a = self.raw_control(ix, (self.rd(ix + 0x1a) & 0xc0) | self.rd(hl));
        self.wr(ix + 0x1a, a);
        self.write_ch(bus, c.wrapping_add(0x20), a);
        self.c(bus, 6 + 7 + 13 + 7 + 17);
        hl += 1;
        let v = self.rd(hl);
        self.write_ch(bus, c.wrapping_add(0x38), v);
        self.c(bus, 10 + 11 + 13 + 7 + 4 + 7);
        hl = hl.wrapping_add(0x0e);
        let mut reg = c.wrapping_add(0x40);
        for i in 0..20 {
            self.c(bus, 11 + 4 + 7 + 17);
            let v = self.rd(hl);
            self.write_ch(bus, reg, v);
            self.c(bus, 7 + 4 + 7 + 7);
            reg = reg.wrapping_add(8);
            if reg & 0xf0 == 0x60 {
                self.c(bus, 17 + 4 + 7 + 4 + 10);
                reg = reg.wrapping_add(0x20);
            } else {
                self.c(bus, 10);
            }
            hl += 1;
            self.c(bus, 6 + 10 + if i < 19 { 13 } else { 8 });
        }
        self.c(bus, 10);
    }

    /// 0x0551: key on the channel's current note (charges the `ret`).
    fn key_on(&mut self, bus: &mut impl MusicBus, ix: u16) {
        if self.raw_channel(ix) {
            // Reestablish/pop already restored the native patch registers.
            // Do not replace their saved global LFO cache here.
            self.raw_key_on(bus, ix, false);
            return;
        }
        self.c(bus, 19 + 10);
        let kc = self.rst20(bus, 0x0a3a, self.rd(ix + 0x0e));
        self.c(bus, 13 + 17);
        self.wr(C002_KEYCODE, kc);
        let patch = self.rd16be(ix + 0x18);
        self.levels(bus, patch, MUSIC_LEVELS);
        self.c(bus, 13 + 4 + 13 + 7 + 17);
        let c = self.rd(C014_CHANNEL);
        self.write_ch(bus, c.wrapping_add(0x28), kc);
        self.c(bus, 7 + 7 + 17);
        self.write_ch(bus, c.wrapping_add(0x30), 0);
        self.c(bus, 19 + 19 + 11 + 10 + 11 + 13 + 7 + 4 + 7 + 17);
        let mask = self.rd(patch.wrapping_add(8));
        self.write_ch(bus, 0x08, c | mask);
        self.c(bus, 10 + 10 + 11 + 12);
        if self.rd(patch.wrapping_add(2)) & 2 == 0 {
            self.c(bus, 11);
            return;
        }
        self.c(bus, 5 + 7 + 7 + 17);
        self.write_ch(bus, 0x01, 0x02);
        self.c(bus, 7 + 17);
        self.write_ch(bus, 0x01, 0x00);
        self.c(bus, 10);
    }

    /// 0x059b / 0x05ac (and the effects player's copy 0x0bfc / 0x0c0d): the
    /// four operators' total levels from the patch (ix = the patch): level
    /// byte + key scaling (0x0600 / 0x0c61), clamped to 0x7f (0x05ed /
    /// 0x0c4e); `v` selects the variant.
    fn levels(&mut self, bus: &mut impl MusicBus, p: u16, v: Levels) {
        self.c(bus, 15 + 19 + 19 + 11 + 14 + 17);
        for (k, (cc, aa, bb)) in [
            (0x0a, 0x0b, 0x09),
            (0x0d, 0x0e, 0x0c),
            (0x10, 0x11, 0x0f),
            (0x13, 0x14, 0x12),
        ]
        .into_iter()
        .enumerate()
        {
            self.c(bus, 19 + 19 + 17);
            let c = self.rd(p + cc);
            let a = self.rd(p + aa);
            let scaled = self.key_scale(bus, p, a, c, v);
            self.c(bus, 19 + 7 + if k < 3 { 17 } else { 0 });
            let b = self.rd(p + bb);
            // 0x05ed
            self.c(bus, 4 + 4);
            let (sum, carry) = scaled.overflowing_add(b);
            let mut d = if carry {
                self.c(bus, 12 + 7);
                0x7f
            } else if (sum as i8) < 0 {
                self.c(bus, 7 + 10 + 7);
                0x7f
            } else {
                self.c(bus, 7 + 10);
                sum
            };
            if !v.direct {
                let ix = MUSIC_CHANNELS + 0x30 * self.rd(C014_CHANNEL) as u16;
                if self.raw_channel(ix) {
                    let algorithm = self.rd(p + 6) & 7;
                    let carriers = match algorithm {
                        0..=3 => 0b1000,
                        4 => 0b1100,
                        5 | 6 => 0b1110,
                        _ => 0b1111,
                    };
                    let offset = if carriers & (1 << k) != 0 { RAW_CARRIER } else { RAW_MODULATOR };
                    d = (d as u16 + self.rd(ix + offset) as u16).min(127) as u8;
                }
            }
            self.c(bus, 13 + 7 + 4 + 17);
            let reg = self
                .rd(v.channel)
                .wrapping_add(0x60)
                .wrapping_add(8 * k as u8);
            if v.direct {
                self.opm_write(bus, reg, d);
            } else {
                self.write_ch(bus, reg, d);
            }
            self.c(bus, 10);
        }
        self.c(bus, 14 + 10); // pop ix; ret
    }

    /// 0x0600: an operator's level adjustment from parameter `a` (low nibble
    /// added after the curve, bits 4-6 the depth) and weight `c`.
    fn key_scale(&mut self, bus: &mut impl MusicBus, p: u16, a: u8, c: u8, tables: Levels) -> u8 {
        self.c(bus, 4 + 7 + 4 + 4 + 16 + 7 + 4 + 11 + 13 + 17);
        let h = a & 0x0f;
        let l = (a >> 4) & 7;
        let kc = self.rd(C002_KEYCODE);
        let curve = self.curve(bus, kc, c, tables);
        self.c(bus, 10 + 4);
        let (s, carry) = curve.overflowing_add(h);
        let h2 = if carry {
            self.c(bus, 12 + 7);
            0x7f
        } else if (s as i8) < 0 {
            self.c(bus, 7 + 10 + 7);
            0x7f
        } else {
            self.c(bus, 7 + 10);
            s
        };
        self.c(bus, 4 + 11 + 4 + 4 + 7 + 4 + 19 + 4 + 8);
        let h3 = (l.rotate_left(1)) & 0x0f;
        let b = self.rd(p + 1);
        let mut v = b;
        if v & 0x80 == 0 {
            self.c(bus, 7 + 7);
            v ^= 0xff;
        } else {
            self.c(bus, 12);
        }
        self.c(bus, 4 + 7 + 17);
        v = v.rotate_left(1);
        let m = self.mul_table(bus, v, h3, tables.product);
        self.c(bus, 10 + 16 + 7 + 8);
        let mut a = m.rotate_right(4) & 0x0f;
        if b & 0x80 != 0 {
            self.c(bus, 7 + 7 + 7);
            a = (a ^ 0xff).wrapping_add(1);
        } else {
            self.c(bus, 12);
        }
        self.c(bus, 4);
        let (r, carry) = a.overflowing_add(h2);
        let out = if carry {
            self.c(bus, 12 + 7);
            0x7f
        } else if (r as i8) < 0 {
            self.c(bus, 7 + 10 + 7);
            0x7f
        } else {
            self.c(bus, 7 + 10);
            r
        };
        self.c(bus, 10);
        out
    }

    /// 0x064b: the key-scaling curve (table 0x0675, 64 entries per half)
    /// for key code `a`, weighted by `c`.
    fn curve(&mut self, bus: &mut impl MusicBus, a: u8, c: u8, v: Levels) -> u8 {
        self.c(bus, 4 + 4 + 7 + 8);
        let mut idx = a.rotate_right(1) & 0x3f;
        if c & 0x80 != 0 {
            self.c(bus, 7 + 7);
            idx = idx.wrapping_add(0x40);
        } else {
            self.c(bus, 12);
        }
        self.c(bus, 10);
        let t = self.rst20(bus, v.curve, idx);
        self.c(bus, 11 + 4 + 17);
        let b = self.mul_table(bus, t, c, v.product);
        self.c(bus, 4 + 10 + 4 + 16 + 17);
        let hi = self.mul_table(bus, t.rotate_left(4), c, v.product);
        self.c(bus, 16 + 7 + 4 + 12 + 7 + 10);
        let a = (hi.rotate_right(4) & 0x0f).wrapping_add(b);
        a.rotate_right(3) & 0x1f
    }

    /// 0x06f5 (0x0d56 for the effects): the 16x16 product table at `table`
    /// (0x0701 / 0x0d62), indexed by the high nibble of `a` and the low
    /// nibble of `h` (charges its body and `ret`).
    fn mul_table(&mut self, bus: &mut impl MusicBus, a: u8, h: u8, table: u16) -> u8 {
        self.c(bus, 7 + 4 + 4 + 7 + 4 + 10);
        let v = self.rst20(bus, table, (a & 0xf0) | (h & 0x0f));
        self.c(bus, 10);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NullBus;
    impl MusicBus for NullBus {
        fn status(&mut self, _t: u64) -> u8 {
            0
        }
        fn write(&mut self, _t: u64, _port: u8, _data: u8) {}
        fn latch(&mut self, _t: u64) -> u8 {
            0xff
        }
        fn take_nmi(&mut self, _t: u64) -> Option<u64> {
            None
        }
    }

    #[test]
    fn arithmetic_helpers_match_the_routines() {
        let mut cpu = MusicCpu::new(vec![0; 0x8000], vec![0; 0x800]);
        let mut bus = NullBus;
        assert_eq!(cpu.mul8(&mut bus, 0x09, 0x30), 0x09 * 0x30);
        assert_eq!(cpu.mul8(&mut bus, 0xff, 0xff), 0xfe01);
        assert_eq!(cpu.div16(&mut bus, 1000, 255), (3, 235));
        assert_eq!(cpu.div16(&mut bus, 254, 255), (0, 254));
        assert_eq!(cpu.frac8(&mut bus, 235, 255), (235u32 * 256 / 255) as u8);
        assert_eq!(cpu.frac8(&mut bus, 0, 255), 0);
    }

    #[test]
    fn a_chip_write_costs_110_t_states_when_not_busy() {
        let mut cpu = MusicCpu::new(vec![0; 0x8000], vec![0; 0x800]);
        let mut bus = NullBus;
        cpu.opm_write(&mut bus, 0x20, 0x00);
        assert_eq!(cpu.t, 110);
    }

    #[test]
    fn raw_clock_wrap_and_overdue_spans_do_not_accumulate_dispatch_delay() {
        let mut image = vec![0; 0x8200];
        image[0x8100..0x8104].copy_from_slice(&[0x0a, 3, 10, 0]);
        let mut cpu = MusicCpu::new(image, vec![0; 0x800]);
        let ix = MUSIC_CHANNELS;
        cpu.wr16be(ix, 0x8100);
        cpu.wr(ix + RAW_FLAG, RAW_MAGIC);
        cpu.raw_store(ix + RAW_CLOCK, u32::MAX - 99);
        cpu.raw_store(ix + RAW_WAIT, 100);
        cpu.t = u32::MAX as u64 + 401;
        assert_eq!(cpu.raw_tick(&mut NullBus, ix), Flow::Done);
        assert_eq!(cpu.raw_word(ix + RAW_WAIT), 10 * 256 - 400);
        assert_eq!(cpu.raw_word(ix + RAW_OVERDUE), 0);
        // A raw marker alone never enables this path for a ROM stream.
        cpu.wr16be(ix, 0x1234);
        assert!(!cpu.raw_channel(ix));
    }

    fn with_cell() -> (MusicCpu, ExtensionCell) {
        let cell = ExtensionCell::default();
        let cpu = MusicCpu::new(vec![0; 0x8000], vec![0; 0x800]).with_extension_cell(cell.clone());
        (cpu, cell)
    }

    #[test]
    fn the_extension_is_taken_from_the_cell_and_given_back_when_idle() {
        let (mut cpu, cell) = with_cell();
        cpu.sync_extension();
        assert!(!cpu.has_extension());
        assert_eq!(cpu.rom.len(), 0x8000);
        let table = Arc::new(vec![7u8; 16]);
        *cell.lock().unwrap() = Some(table.clone());
        cpu.sync_extension();
        assert!(cpu.has_extension());
        assert_eq!(cpu.rd(0x8000), 7);
        assert_eq!(cpu.rom.len(), 0x8010);
        *cell.lock().unwrap() = None;
        cpu.sync_extension();
        assert!(!cpu.has_extension());
        assert_eq!(cpu.rom.len(), 0x8000);
        assert_eq!(cpu.rd(0x8000), 0xff);
    }

    #[test]
    fn a_tune_playing_from_the_extension_keeps_it_until_the_tune_is_over() {
        let (mut cpu, cell) = with_cell();
        *cell.lock().unwrap() = Some(Arc::new(vec![7u8; 16]));
        cpu.sync_extension();
        // A current tune with a channel's stream at 0x8100.
        cpu.wr(C011_MUSIC, 0x08);
        cpu.wr16be(MUSIC_CHANNELS + 0x30 * 2, 0x8100);
        *cell.lock().unwrap() = None;
        cpu.sync_extension();
        assert!(cpu.has_extension(), "taken away from under a tune");
        // The same for a saved tune.
        cpu.wr(C011_MUSIC, 0);
        cpu.wr(C012_SAVED, 0x08);
        cpu.wr16be(SAVED_CHANNELS, 0x8100);
        cpu.sync_extension();
        assert!(cpu.has_extension(), "taken away from under a saved tune");
        // A 1987 tune's streams are in the ROM, and a stale pointer with no
        // tune is nothing.
        cpu.wr(C012_SAVED, 0);
        cpu.wr(C011_MUSIC, 0x08);
        cpu.wr16be(MUSIC_CHANNELS + 0x30 * 2, 0x1234);
        cpu.sync_extension();
        assert!(!cpu.has_extension());
    }

    #[test]
    fn a_stale_pointer_after_the_tune_ended_does_not_hold_the_extension() {
        let (mut cpu, cell) = with_cell();
        *cell.lock().unwrap() = Some(Arc::new(vec![7u8; 16]));
        cpu.sync_extension();
        cpu.wr16be(MUSIC_CHANNELS, 0x8100);
        cpu.wr16be(SAVED_CHANNELS, 0x8100);
        *cell.lock().unwrap() = None;
        cpu.sync_extension();
        assert!(!cpu.has_extension(), "no tune is current or saved");
    }

    #[test]
    fn a_deferred_extension_removal_is_rechecked_when_the_stop_handler_returns() {
        let (mut cpu, cell) = with_cell();
        *cell.lock().unwrap() = Some(Arc::new(vec![0; 16]));
        cpu.sync_extension();
        cpu.wr(C011_MUSIC, 8);
        cpu.wr16be(MUSIC_CHANNELS, 0x8100);
        *cell.lock().unwrap() = None;
        cpu.sync_extension();
        assert!(cpu.has_extension());
        // The native stop clears the header. No further non-idle command
        // is needed to release its now-unused image.
        cpu.wr(C011_MUSIC, 0);
        let before = cpu.t;
        cpu.reti(&mut NullBus);
        assert!(!cpu.has_extension());
        assert_eq!(cpu.t - before, 14, "no added emulated CPU cycles");
    }

    #[test]
    fn a_cpu_with_no_cell_runs_the_image_it_was_given() {
        let mut cpu = MusicCpu::new(vec![0; 0x8010], vec![0; 0x800]);
        cpu.sync_extension();
        assert_eq!(cpu.rom.len(), 0x8010);
    }
}
