//! The samples CPU's program (ticket #10), translated, with its timing
//! (ticket #32).
//!
//! The second Z80 (`audio2`: sfu-00.1h + sf-01.1k, no RAM) plays ADPCM
//! samples on two MSM5205s. Its whole program is a two-channel coroutine
//! scheduler that runs inside the 8 kHz interrupt (sf.cpp:
//! `set_periodic_int(irq0_line_hold, 8000 Hz)`), with the channels' state
//! in the two Z80 register sets:
//!
//! - Each interrupt (0x0038) swaps to the other channel's register set
//!   (`ex af,af'` + `exx`), so channel 0 (E bit 0 clear, port 0, IX) runs on
//!   odd ticks and channel 1 (E bit 0 set, port 1, IY) on even ticks.
//! - A is the channel's countdown: the channel acts when it reaches 0, at
//!   the continuation in IX / IY, which sets the next countdown and
//!   continuation.
//! - The idle continuation (0x0046) polls the sound latch (port 1 read):
//!   commands with bit 7 set are for this CPU. 0xff marks the latch idle
//!   (I = 0xff); 0x80 stops the channel; any other command different from
//!   the last accepted one (I) starts it if the channel table at 0x01b1
//!   assigns it to this channel (0x008d). Then (0x0065) a playing channel
//!   outputs its next nibble immediately; an idle one holds its chip in
//!   reset and polls again three services later.
//! - Starting (0x008d..0x0128): the bank bits come from 0x01f1, the
//!   sample pointer from the table at 0x0231 (commands < 0x10) or from a
//!   table at 0x8000 in the command range's bank; the sample header is a
//!   big-endian byte length. The chip is held in reset for one service,
//!   released (0x40, a 0 nibble) the next, and the first nibble follows two
//!   services later.
//! - Playing (0x012b / 0x017b): each service re-selects the channel's bank
//!   (port 2, shared by both channels) and writes one nibble with bit 6 set,
//!   high nibble first, so a sample plays at 4 kHz per channel.
//!
//! **Timing.** The CPU runs at [`CLOCK_HZ`]; every translated path charges
//! the T-states of the original instructions it stands for (the standard
//! Z80 costs, branch-dependent), so each port write and the latch read get
//! their exact time. Between interrupts the program sits in `ei; jr $-1`
//! (0x0028, 16 T-states a lap). An interrupt request at time r (the k-th
//! at k / 8 kHz) is accepted as the pinned MAME does it: the CPU runs the
//! whole T-states up to r and stops there (inside an instruction if need
//! be), the request is taken at the next instruction start where
//! interrupts are enabled and not blocked by `ei`'s delay — the end of a
//! `jr` — and the IM 1 response costs 13 T-states before 0x0038 runs. So
//! the acceptance is the first `jr` end at or after floor(r). A request that
//! arrives while a handler runs (interrupts disabled) is taken 16 T-states
//! after it returns to the loop. A MAME instruction trace of the samples
//! CPU confirmed each of these (docs/reference/fight-samples.md). I/O happens 4 T-states before the end of
//! an `in` / `out` (the I/O cycle's first T-state): the writes' times match
//! the reference's tap log, and the latch is read at that point of the
//! poll's `in`.
//!
//! The register-level outputs (port writes per tick, with their times) are
//! what the reference comparison checks; [`crate::msm5205`] turns them into
//! audio.

use crate::TICK_HZ;

/// The samples CPU's clock (sf.cpp: 3.579545 MHz).
pub const CLOCK_HZ: f64 = 3_579_545.0;
/// The IM 1 interrupt response, T-states.
const IRQ_RESPONSE: f64 = 13.0;
/// The idle loop's lap (`ei` 4 + `jr` 12).
const LOOP: f64 = 16.0;
/// T-states before the end of an `in` / `out` at which the I/O happens.
const IO_LEAD: f64 = 4.0;

/// One port write of the samples CPU: port 0 / 1 = MSM5205 #0 / #1 (bit 7
/// reset, bits 0-3 data, each write clocks one decode), port 2 = bank;
/// `t` is the write's machine time in seconds. Equality compares the tick,
/// port and data (the reference's attribution); the time is checked
/// separately.
#[derive(Debug, Clone, Copy)]
pub struct PortWrite {
    pub tick: u64,
    pub port: u8,
    pub data: u8,
    pub t: f64,
}

impl PartialEq for PortWrite {
    fn eq(&self, other: &Self) -> bool {
        (self.tick, self.port, self.data) == (other.tick, other.port, other.data)
    }
}

/// Where a channel resumes when its countdown expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Continuation {
    /// 0x0046: poll the latch, then play or idle (0x0065).
    Poll,
    /// 0x011e / 0x016e: release the chip from reset.
    Release,
    /// 0x012b / 0x017b: output the next nibble.
    Play,
}

/// One channel's Z80 register set.
#[derive(Debug, Clone, Copy)]
struct Channel {
    /// Countdown (A): services until the continuation runs.
    a: u8,
    /// BC: sample bytes left.
    bc: u16,
    /// D: last command seen by this channel's poll.
    d: u8,
    /// E: bit 0 channel, bit 4 next nibble is the high one, bits 5-7 bank.
    e: u8,
    /// HL: sample data pointer.
    hl: u16,
    cont: Continuation,
}

/// The samples CPU's state: both register sets, the I register (the last
/// accepted command, shared by both channels), the bank register, and the
/// T-state clock.
pub struct SamplesCpu {
    ch: [Channel; 2],
    /// The register set the last interrupt switched to.
    active: usize,
    i: u8,
    bank: u8,
    /// Interrupts taken so far (tick k is requested at k / 8000 s).
    tick: u64,
    /// The T-state at which the CPU last entered the idle loop (0x0028).
    idle_at: f64,
}

/// The `audio2` region (0x40000 bytes): 0x0000-0x7fff is fixed program/data,
/// 0x8000-0xffff a 32 KiB window selected by port 2 (entry n = region
/// offset 0x8000 + n * 0x8000).
pub struct SampleRom<'a>(pub &'a [u8]);

impl SampleRom<'_> {
    fn read(&self, bank: u8, addr: u16) -> u8 {
        let off = if addr < 0x8000 {
            addr as usize
        } else {
            0x8000 + bank as usize * 0x8000 + (addr as usize - 0x8000)
        };
        self.0.get(off).copied().unwrap_or(0xff)
    }
}

impl Default for SamplesCpu {
    fn default() -> Self {
        Self::power_on(&mut |_| {})
    }
}

/// The handler's T-state clock: `c` is the T-state at which the next
/// instruction starts.
struct Clock {
    c: f64,
    tick: u64,
}

impl Clock {
    /// Charge an instruction's T-states.
    fn charge(&mut self, t: f64) {
        self.c += t;
    }

    /// An `out (n),a` (11 T-states), its write 4 T-states before its end.
    fn out(&mut self, port: u8, data: u8, out: &mut impl FnMut(PortWrite)) {
        self.c += 11.0;
        out(PortWrite {
            tick: self.tick,
            port,
            data,
            t: (self.c - IO_LEAD) / CLOCK_HZ,
        });
    }

    /// An `in a,(n)` (11 T-states), its read 4 T-states before its end.
    fn input(&mut self, latch: &mut impl FnMut(f64) -> u8) -> u8 {
        self.c += 11.0;
        latch((self.c - IO_LEAD) / CLOCK_HZ)
    }
}

impl SamplesCpu {
    /// Power-on (0x0000..0x0028): I = 0; channel 0 in the alternate set
    /// (E = 0x10), channel 1 in the main set (E = 0x11); both chips put in
    /// reset; both countdowns 3, both continuations the poll; the main set
    /// is active, so the first interrupt switches to channel 0. The reset
    /// code's 153 T-states lead into the idle loop.
    pub fn power_on(out: &mut impl FnMut(PortWrite)) -> Self {
        let idle = |e| Channel {
            a: 3,
            bc: 0,
            d: 0,
            e,
            hl: 0,
            cont: Continuation::Poll,
        };
        let mut c = Clock { c: 0.0, tick: 0 };
        // im 1, di, xor a, ld i,a, exx, ld bc, ld e, exx, ex af,af', ld a
        c.charge(8.0 + 4.0 + 4.0 + 9.0 + 4.0 + 10.0 + 7.0 + 4.0 + 4.0 + 7.0);
        c.out(0, 0xc0, out);
        // ld a, ex af,af', ld ix, ld bc, ld e, ld a
        c.charge(7.0 + 4.0 + 14.0 + 10.0 + 7.0 + 7.0);
        c.out(1, 0xc0, out);
        // ld a, ld iy
        c.charge(7.0 + 14.0);
        SamplesCpu {
            ch: [idle(0x10), idle(0x11)],
            active: 1,
            i: 0,
            bank: 0,
            tick: 0,
            idle_at: c.c,
        }
    }

    /// Interrupts taken so far.
    pub fn ticks(&self) -> u64 {
        self.tick
    }

    /// Take the next 8 kHz interrupt (0x0038). `latch` reads the sound
    /// latch at a machine time in seconds; port writes go to `out`.
    pub fn interrupt(
        &mut self,
        rom: &SampleRom,
        latch: &mut impl FnMut(f64) -> u8,
        out: &mut impl FnMut(PortWrite),
    ) {
        self.tick += 1;
        let request = self.tick as f64 * CLOCK_HZ / TICK_HZ as f64;
        // Accepted at the first end of a `jr` lap at or after the request's
        // whole T-state (see the module doc), at least one lap after the
        // handler returned.
        let first = self.idle_at + LOOP;
        let laps = ((request.floor() - first) / LOOP).ceil().max(0.0);
        let accept = first + LOOP * laps;
        let mut c = Clock {
            c: accept + IRQ_RESPONSE,
            tick: self.tick,
        };
        // 0x0038: di, ex af,af', exx, dec a.
        c.charge(4.0 + 4.0 + 4.0 + 4.0);
        self.active ^= 1;
        let n = self.active;
        self.ch[n].a = self.ch[n].a.wrapping_sub(1);
        if self.ch[n].a != 0 {
            c.charge(12.0); // jr nz,0x0028 taken
            self.idle_at = c.c;
            return;
        }
        // jr not taken; bit 0,e; jr nz,0x0044 (channel 1); jp (ix) / (iy).
        c.charge(7.0 + 8.0 + if n == 1 { 12.0 } else { 7.0 } + 8.0);
        match self.ch[n].cont {
            Continuation::Poll => self.poll(n, rom, latch, out, &mut c),
            Continuation::Release => {
                // 0x011e / 0x016e: ld a,0x40; out; ld a,2; ld ix/iy; jp.
                c.charge(7.0);
                c.out(self.port(n), 0x40, out);
                c.charge(7.0 + 14.0 + 10.0);
                self.ch[n].a = 2;
                self.ch[n].cont = Continuation::Play;
            }
            Continuation::Play => self.play(n, rom, out, &mut c),
        }
        self.idle_at = c.c;
    }

    fn port(&self, n: usize) -> u8 {
        self.ch[n].e & 1
    }

    /// 0x0046 / 0x0065.
    fn poll(
        &mut self,
        n: usize,
        rom: &SampleRom,
        latch: &mut impl FnMut(f64) -> u8,
        out: &mut impl FnMut(PortWrite),
        c: &mut Clock,
    ) {
        let value = c.input(latch);
        c.charge(8.0); // bit 7,a
        if value & 0x80 == 0 {
            c.charge(12.0); // jr z,0x0065 taken
        } else {
            c.charge(7.0 + 7.0); // jr not taken; cp 0xff
            if value == 0xff {
                c.charge(7.0 + 9.0 + 12.0); // jr nz not taken; ld i,a; jr 0x0065
                self.i = 0xff;
            } else {
                c.charge(12.0 + 7.0); // jr nz taken; and 0x7f
                let cmd = value & 0x7f;
                if cmd == 0 {
                    c.charge(7.0 + 9.0 + 10.0 + 12.0); // jr nz not taken; ld i,a; ld bc,0; jr
                    self.i = 0;
                    self.ch[n].bc = 0;
                } else {
                    c.charge(12.0 + 4.0 + 9.0 + 4.0); // jr nz taken; ld d,a; ld a,i; cp d
                    self.ch[n].d = cmd;
                    if self.i != cmd {
                        c.charge(12.0); // jr nz,0x008d taken
                        if self.start(n, rom, cmd, out, c) {
                            return;
                        }
                    } else {
                        c.charge(7.0);
                    }
                }
            }
        }
        // 0x0065: bit 0,e; jr nz,0x007b; ld a,b; or c; jp nz.
        c.charge(8.0 + if n == 1 { 12.0 } else { 7.0 } + 4.0 + 4.0 + 10.0);
        if self.ch[n].bc != 0 {
            self.play(n, rom, out, c);
        } else {
            // ld a,0xc0; out; ld a,3; ld ix/iy; jp 0x0028.
            c.charge(7.0);
            c.out(self.port(n), 0xc0, out);
            c.charge(7.0 + 14.0 + 10.0);
            self.ch[n].a = 3;
            self.ch[n].cont = Continuation::Poll;
        }
    }

    /// 0x008d: start `cmd` if the channel table assigns it to this channel;
    /// otherwise back to 0x0065 (false).
    fn start(
        &mut self,
        n: usize,
        rom: &SampleRom,
        cmd: u8,
        out: &mut impl FnMut(PortWrite),
        c: &mut Clock,
    ) -> bool {
        // ld sp,hl; ld a,d; ld hl,0x01b1; add a,l; ld l,a; jr nc (+ inc h).
        c.charge(6.0 + 4.0 + 10.0 + 4.0 + 4.0);
        c.charge(if 0xb1u16 + cmd as u16 >= 0x100 {
            7.0 + 4.0
        } else {
            12.0
        });
        // ld a,(hl); ld h,a; ld a,e; and 1; cp h; jr z,0x00a6.
        c.charge(7.0 + 4.0 + 4.0 + 7.0 + 4.0);
        if rom.read(0, 0x01b1 + cmd as u16) != self.port(n) {
            c.charge(7.0 + 10.0 + 11.0 + 10.0); // jr not taken; ld hl,0; add hl,sp; jp 0x0065
            return false;
        }
        c.charge(12.0);
        self.i = cmd;
        // 0x00a6: ld a,d; ld i,a; ld hl,0x01f1; add a,l; ld l,a; jr nc (+ inc h);
        // ld a,e; and 0x1f; or (hl); ld e,a.
        c.charge(4.0 + 9.0 + 10.0 + 4.0 + 4.0);
        c.charge(if 0xf1u16 + cmd as u16 >= 0x100 {
            7.0 + 4.0
        } else {
            12.0
        });
        c.charge(4.0 + 7.0 + 7.0 + 4.0);
        let ch = &mut self.ch[n];
        ch.e = (ch.e & 0x1f) | rom.read(0, 0x01f1 + cmd as u16);
        // 0x00b6..0x00f1: pointer table and index by command range.
        // ld a,d; ld bc,0x0231; cp 0x10; jr c,0x00f3.
        c.charge(4.0 + 10.0 + 7.0);
        let (base, idx) = if cmd < 0x10 {
            c.charge(12.0);
            (0x0231u16, cmd)
        } else {
            c.charge(7.0 + 10.0); // jr not taken; ld bc,0x8000
            let mut idx = cmd.wrapping_sub(0x3e);
            for (limit, sub) in [
                (0x1cu8, 0x10u8),
                (0x1f, 0x1c),
                (0x29, 0x1f),
                (0x32, 0x29),
                (0x3a, 0x32),
                (0x3e, 0x3a),
            ] {
                c.charge(7.0); // cp limit
                if cmd >= limit {
                    c.charge(12.0); // jr nc taken: on to the next range
                } else {
                    c.charge(7.0 + 7.0 + 12.0); // jr not taken; sub; jr 0x00f3
                    idx = cmd - sub;
                    break;
                }
            }
            if cmd >= 0x3e {
                c.charge(7.0); // 0x00f1: sub 0x3e
            }
            (0x8000, idx)
        };
        // 0x00f3: add a,a; add a,c; ld c,a; jr nc (+ inc b).
        c.charge(4.0 + 4.0 + 4.0);
        let doubled = idx.wrapping_add(idx);
        let sum = doubled as u16 + (base & 0xff);
        c.charge(if sum >= 0x100 { 7.0 + 4.0 } else { 12.0 });
        let entry = base.wrapping_add(doubled as u16);
        // ld a,e; rlca x3; and 7; out (2).
        c.charge(4.0 + 12.0 + 7.0);
        self.bank = ch.e >> 5;
        c.out(2, self.bank, out);
        // ld a,(bc); ld l,a; inc bc; ld a,(bc); ld h,a; ld b,(hl); inc hl;
        // ld c,(hl); inc hl; set 4,e; bit 0,e; jp nz,0x0161.
        c.charge(7.0 + 4.0 + 6.0 + 7.0 + 4.0 + 7.0 + 6.0 + 7.0 + 6.0 + 8.0 + 8.0 + 10.0);
        let lo = rom.read(self.bank, entry);
        let hi = rom.read(self.bank, entry.wrapping_add(1));
        let mut hl = u16::from_le_bytes([lo, hi]);
        let b = rom.read(self.bank, hl);
        hl = hl.wrapping_add(1);
        let cc = rom.read(self.bank, hl);
        hl = hl.wrapping_add(1);
        let ch = &mut self.ch[n];
        ch.hl = hl;
        ch.bc = u16::from_be_bytes([b, cc]);
        ch.e |= 0x10;
        // 0x0111 / 0x0161: ld a,0xc0; out; ld a,3; ld ix/iy; jp 0x0028.
        c.charge(7.0);
        c.out(self.port(n), 0xc0, out);
        c.charge(7.0 + 14.0 + 10.0);
        self.ch[n].a = 3;
        self.ch[n].cont = Continuation::Release;
        true
    }

    /// 0x012b / 0x017b: one nibble, high first.
    fn play(&mut self, n: usize, rom: &SampleRom, out: &mut impl FnMut(PortWrite), c: &mut Clock) {
        // ld a,e; rlca x3; and 7; out (2); bit 4,e; jr nz.
        c.charge(4.0 + 12.0 + 7.0);
        self.bank = self.ch[n].e >> 5;
        c.out(2, self.bank, out);
        c.charge(8.0);
        let byte = rom.read(self.bank, self.ch[n].hl);
        let ch = &mut self.ch[n];
        let nibble = if ch.e & 0x10 != 0 {
            // taken: ld a,(hl); rrca x4; and 0x0f; or 0x40.
            c.charge(12.0 + 7.0 + 16.0 + 7.0 + 7.0);
            ch.e &= !0x10;
            byte >> 4
        } else {
            // not taken: ld a,(hl); and 0x0f; or 0x40.
            c.charge(7.0 + 7.0 + 7.0 + 7.0);
            ch.hl = ch.hl.wrapping_add(1);
            ch.bc = ch.bc.wrapping_sub(1);
            ch.e |= 0x10;
            byte & 0x0f
        };
        let high = self.ch[n].e & 0x10 == 0;
        c.out(self.port(n), nibble | 0x40, out);
        // high: res 4,e; low: inc hl; dec bc; set 4,e. Then ld a,1; ld
        // ix/iy; jp 0x0028.
        c.charge(if high { 8.0 } else { 6.0 + 6.0 + 8.0 });
        c.charge(7.0 + 14.0 + 10.0);
        self.ch[n].a = 1;
        self.ch[n].cont = Continuation::Poll;
    }
}
