//! Debug aid: predict the round-timer task's memory one frame at a time
//! over a capture's every-frame work dumps and report the frames where the
//! prediction differs from the reference.
//!
//!   cargo run -p sf-engine --release --example timer_predict -- <capture> <first> <last>

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cap = Path::new(&args[1]);
    let (first, last): (u32, u32) = (args[2].parse().unwrap(), args[3].parse().unwrap());
    let rom = sf_video::romset::load_program_rom(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
    )
    .unwrap();
    let (a, b) = sf_engine::run::TIMER_RAM;
    let (a, b) = ((a - 0xff8000) as usize, (b - 0xff8000) as usize);
    let mut bad = 0;
    for f in first + 1..=last {
        let before = std::fs::read(cap.join(format!("ram_{:06}_work.bin", f - 1))).unwrap();
        let after = std::fs::read(cap.join(format!("ram_{f:06}_work.bin"))).unwrap();
        let pred = sf_engine::run::timer_pass(rom.clone(), &before).unwrap();
        if pred[a..b] != after[a..b] {
            bad += 1;
            if bad <= 12 {
                println!(
                    "frame {f}: before {} predicted {} reference {}",
                    hex(&before[a..b]),
                    hex(&pred[a..b]),
                    hex(&after[a..b])
                );
            }
        }
    }
    println!("{} of {} frames differ", bad, last - first);
}

fn hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02x}"))
        .collect::<Vec<_>>()
        .join("")
}
