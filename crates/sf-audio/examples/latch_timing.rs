//! Ticket #35 check: a native capture's per-frame vblank-handler cycles
//! against the reference's logged latch-write times (local-only data). For
//! every native frame with a reference write in it, the latency the write
//! implies (its offset into the frame in 68000 cycles, less the native
//! cycles) must lie in the observed interrupt-latency band.
//!
//!   cargo run -p sf-audio --release --example latch_timing -- <native capture> <reference audio capture>
//!
//! Prints the latencies' range and histogram, the frames outside the band,
//! the value mismatches, and the latencies by handler cost (path).

use std::collections::BTreeMap;
use std::path::Path;

use sf_audio::check::{latch_residuals, native_latch, read_taps};
use sf_audio::IRQ_LATENCY_CYCLES;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let native = native_latch(Path::new(&args[1])).expect("native frames.tsv with latch_cycles");
    let log = read_taps(&Path::new(&args[2]).join("audio_taps.tsv")).expect("reference tap log");
    let residuals = latch_residuals(&log, &native);
    let with_cycles = native.iter().filter(|r| r.2 > 0).count();
    println!(
        "{} native frames with cycles, {} with a reference write in the frame ({} reference writes in all)",
        with_cycles,
        residuals.len(),
        log.latch.len()
    );
    let mut hist: BTreeMap<i64, usize> = BTreeMap::new();
    let mut by_cost: BTreeMap<u32, (usize, f64, f64)> = BTreeMap::new();
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    let mut outside = Vec::new();
    let mut mismatched = Vec::new();
    for r in &residuals {
        lo = lo.min(r.latency);
        hi = hi.max(r.latency);
        *hist
            .entry((r.latency / 10.0).floor() as i64 * 10)
            .or_default() += 1;
        let e = by_cost.entry(r.cycles).or_insert((0, f64::MAX, f64::MIN));
        e.0 += 1;
        e.1 = e.1.min(r.latency);
        e.2 = e.2.max(r.latency);
        if !IRQ_LATENCY_CYCLES.contains(&(r.latency.round() as u32)) {
            outside.push(*r);
        }
        if r.native != r.reference {
            mismatched.push(*r);
        }
    }
    println!("implied latency: {lo:.1}..{hi:.1} cycles (band {IRQ_LATENCY_CYCLES:?}); {} outside, {} value mismatches", outside.len(), mismatched.len());
    println!("histogram (10-cycle bins):");
    for (b, c) in &hist {
        println!("  {b:>4}..{:<4} {c}", b + 9);
    }
    println!("by handler cost (cycles): frames, latency range");
    for (cost, (n, a, b)) in &by_cost {
        println!("  {cost:>5}: {n:>5} frames, {a:.1}..{b:.1}");
    }
    for r in outside.iter().take(20) {
        println!(
            "outside: frame {} cost {} latency {:.1} (native {:02x}, reference {:02x})",
            r.frame, r.cycles, r.latency, r.native, r.reference
        );
    }
    for r in mismatched.iter().take(20) {
        println!(
            "mismatch: frame {} native {:02x} reference {:02x}",
            r.frame, r.native, r.reference
        );
    }
}
