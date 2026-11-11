//! Replay the reference's sound-latch writes (from a local audio-probe tap
//! log) through the translated samples CPU from power-on, and compare every
//! samples-CPU port write (tick, port, data) with the reference's.
//!
//!   cargo run -p sf-audio --release --example compare_samples -- <audio_taps.tsv> [last_tick]

use sf_audio::samples::PortWrite;
use sf_audio::{LatchTimeline, SamplesSound};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&args[1]).expect("tap log");
    let mut latch = Vec::new();
    let mut reference = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        let t: f64 = f[0].parse().unwrap();
        let addr = u32::from_str_radix(f[3], 16).unwrap();
        let data = u32::from_str_radix(f[4], 16).unwrap();
        match f[2] {
            "latch" => latch.push((t, (data & 0xff) as u8)),
            "s2io" => {
                // Attribute each write to the interrupt whose handler made
                // it: a chip write belongs to its channel's tick parity
                // (port 0 odd, port 1 even); the long sample-start path
                // can finish its reset write just past the next tick.
                let mut tick = (t * 8000.0).floor() as u64;
                if addr < 2 && tick > 0 && (tick % 2 == 1) != (addr == 0) {
                    tick -= 1;
                }
                reference.push(PortWrite {
                    tick,
                    port: addr as u8,
                    data: data as u8,
                    t,
                });
            }
            _ => {}
        }
    }
    let last_tick: u64 = args
        .get(2)
        .map(|s| s.parse().unwrap())
        .unwrap_or(reference.last().unwrap().tick);
    let audio2 = sf_video::romset::load_region(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
        "audio2",
    )
    .expect("audio2 region");
    let mut sound = SamplesSound::new(audio2);
    let mut timeline = LatchTimeline::new(latch);
    // Power-on (0x0000..0x0028) puts both chips in reset before any tick.
    let mut native = Vec::new();
    sf_audio::samples::SamplesCpu::power_on(&mut |w| native.push(w));
    while sound.ticks() < last_tick {
        sound.tick(&mut timeline, &mut |w| native.push(w));
    }
    // The power-on writes are logged at tick 0 natively; the reference's
    // fall in the first tick too.
    let reference: Vec<PortWrite> = reference
        .into_iter()
        .filter(|w| w.tick <= last_tick)
        .collect();
    let n = native.len().min(reference.len());
    match (0..n).find(|&i| native[i] != reference[i]) {
        Some(i) => {
            println!("first difference at write #{i}:");
            for j in i.saturating_sub(3)..(i + 4).min(n) {
                println!("  ref {:?}   native {:?}", reference[j], native[j]);
            }
        }
        None if native.len() == reference.len() => {
            println!(
                "IDENTICAL: {} samples-CPU port writes through tick {last_tick}",
                n
            )
        }
        None => println!(
            "prefix identical ({n} writes); lengths differ: ref {} native {}",
            reference.len(),
            native.len()
        ),
    }
}
