//! Debug aid: run the translated samples CPU on a reference tap log and
//! report how its modelled write times compare with the logged ones
//! (local-only data): a histogram of the difference in T-states and the
//! first writes that differ by a T-state or more, with their context.
//!
//!   cargo run -p sf-audio --release --example samples_timing -- <audio_taps.tsv> [last_tick]
//!
//! With SF_DUMP_TICKS=<a>..<b> in the environment it also lists both sides'
//! writes with their times for those ticks.

use std::collections::BTreeMap;

use sf_audio::check::{latch_timeline, read_taps};
use sf_audio::samples::{PortWrite, SamplesCpu, CLOCK_HZ};
use sf_audio::SamplesSound;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let log = read_taps(std::path::Path::new(&args[1])).expect("tap log");
    let last: u64 = args
        .get(2)
        .map(|s| s.parse().unwrap())
        .unwrap_or(log.writes.last().unwrap().tick);
    let audio2 = sf_video::romset::load_region(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
        "audio2",
    )
    .expect("audio2");
    let mut snd = SamplesSound::new(audio2);
    let mut tl = latch_timeline(&log, None);
    let mut native: Vec<PortWrite> = Vec::new();
    SamplesCpu::power_on(&mut |w| native.push(w));
    while snd.ticks() < last {
        snd.tick(&mut tl, &mut |w| native.push(w));
    }
    let reference: Vec<PortWrite> = log
        .writes
        .iter()
        .copied()
        .filter(|w| w.tick < last)
        .collect();
    let native: Vec<PortWrite> = native.into_iter().filter(|w| w.tick < last).collect();
    let n = native.len().min(reference.len());
    if let Ok(range) = std::env::var("SF_DUMP_TICKS") {
        let (a, b) = range.split_once("..").unwrap();
        let (a, b): (u64, u64) = (a.parse().unwrap(), b.parse().unwrap());
        for w in native.iter().filter(|w| w.tick >= a && w.tick <= b) {
            println!(
                "native    tick {} port {} data {:02x} t {:.9} T {:.1}",
                w.tick,
                w.port,
                w.data,
                w.t,
                w.t * CLOCK_HZ
            );
        }
        for w in reference.iter().filter(|w| w.tick >= a && w.tick <= b) {
            println!(
                "reference tick {} port {} data {:02x} t {:.9} T {:.1}",
                w.tick,
                w.port,
                w.data,
                w.t,
                w.t * CLOCK_HZ
            );
        }
    }
    let mut hist: BTreeMap<i64, usize> = BTreeMap::new();
    let mut shown = 0;
    for i in 0..n {
        if native[i] != reference[i] {
            println!(
                "data differs at write #{i}: reference {:?} native {:?}",
                reference[i], native[i]
            );
            break;
        }
        let d = (native[i].t - reference[i].t) * CLOCK_HZ;
        *hist.entry(d.round() as i64).or_default() += 1;
        if d.abs() >= 1.0 && shown < 12 {
            shown += 1;
            println!(
                "write #{i} tick {} port {} data {:02x}: reference {:.9} native {:.9} ({d:+.2} T); previous reference writes: {:?}",
                reference[i].tick,
                reference[i].port,
                reference[i].data,
                reference[i].t,
                native[i].t,
                reference[i.saturating_sub(3)..i].iter().map(|w| (w.tick, w.port, w.data, w.t)).collect::<Vec<_>>()
            );
        }
    }
    println!("compared {n} writes ({} reference, {} native); histogram of native - reference (T-states):", reference.len(), native.len());
    for (d, c) in &hist {
        println!("  {d:+}: {c}");
    }
    // By context: channel (tick parity), what the write is, whether it is
    // the service's first write, and the latch value the service's poll
    // read (from the log's latch writes).
    let mut by: BTreeMap<(u64, &str, bool, &str), BTreeMap<i64, usize>> = BTreeMap::new();
    let mut latch_i = 0;
    let mut latch_now = 0u8;
    for i in 0..n {
        if native[i] != reference[i] {
            break;
        }
        let w = reference[i];
        let tick_t = w.tick as f64 / 8000.0;
        while latch_i < log.latch.len() && log.latch[latch_i].0 <= tick_t {
            latch_now = log.latch[latch_i].1;
            latch_i += 1;
        }
        let kind = match (w.port, w.data) {
            (2, _) => "bank",
            (_, 0xc0) => "reset",
            (_, 0x40) => "release/0",
            _ => "nibble",
        };
        let first = i == 0 || reference[i - 1].tick != w.tick;
        let latch = if latch_now == 0xff {
            "ff"
        } else if latch_now & 0x80 == 0 {
            "music"
        } else {
            "cmd"
        };
        let d = ((native[i].t - w.t) * CLOCK_HZ).round() as i64;
        *by.entry((w.tick % 2, kind, first, latch))
            .or_default()
            .entry(d)
            .or_default() += 1;
    }
    // The first services whose first write is off: the two services before
    // them (tick, kind, difference) and the request's position in the loop
    // lattice the model assumed.
    let mut shown = 0;
    let mut prev: Vec<(u64, &str, i64)> = Vec::new();
    for i in 0..n {
        if native[i] != reference[i] {
            break;
        }
        let w = reference[i];
        let first = i == 0 || reference[i - 1].tick != w.tick;
        if !first {
            continue;
        }
        let kind = match (w.port, w.data) {
            (2, _) => "bank",
            (_, 0xc0) => "reset",
            (_, 0x40) => "release",
            _ => "nibble",
        };
        let d = ((native[i].t - w.t) * CLOCK_HZ).round() as i64;
        if d != 0 && shown < 15 && w.tick > 1000 {
            shown += 1;
            let request = w.tick as f64 * CLOCK_HZ / 8000.0;
            println!(
                "tick {} ({kind}, ch{}): {d:+} T; previous services {:?}; request frac {:.3} T; reference t {:.9}",
                w.tick,
                w.tick % 2,
                prev,
                request.fract(),
                w.t
            );
        }
        prev.push((w.tick, kind, d));
        if prev.len() > 3 {
            prev.remove(0);
        }
    }
    // The lattice shift (-difference mod 16) of each service's acceptance,
    // by the number of silent (countdown) ticks since the previous service
    // with writes and that service's kind.
    let mut shifts: BTreeMap<(u64, &str, u64), BTreeMap<i64, usize>> = BTreeMap::new();
    let mut prev_tick = 0u64;
    let mut prev_kind = "none";
    for i in 0..n {
        if native[i] != reference[i] {
            break;
        }
        let w = reference[i];
        let first = i == 0 || reference[i - 1].tick != w.tick;
        let kind = match (w.port, w.data) {
            (2, _) => "bank",
            (_, 0xc0) => "reset",
            (_, 0x40) => "release",
            _ => "nibble",
        };
        if first && w.tick > 0 {
            let d = ((native[i].t - w.t) * CLOCK_HZ).round() as i64;
            let shift = (-d).rem_euclid(16);
            let silent = w.tick.saturating_sub(prev_tick + 1).min(9);
            *shifts
                .entry((silent, prev_kind, w.tick % 2))
                .or_default()
                .entry(shift)
                .or_default() += 1;
        }
        if !first || w.tick == 0 {
            // the service's last write decides its kind for the next one
        }
        prev_tick = w.tick;
        prev_kind = kind;
    }
    println!(
        "lattice shift by (silent ticks before, previous service's first write, this channel):"
    );
    for (k, h) in &shifts {
        println!(
            "  {k:?}: {:?}",
            h.iter()
                .map(|(d, c)| format!("{d}x{c}"))
                .collect::<Vec<_>>()
        );
    }
    println!("by (tick parity, kind, first write of the service, latch at the tick): differences");
    for (k, h) in &by {
        println!(
            "  {k:?}: {:?}",
            h.iter()
                .map(|(d, c)| format!("{d:+}x{c}"))
                .collect::<Vec<_>>()
        );
    }
}
