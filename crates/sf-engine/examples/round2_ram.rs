//! Debug aid: run one-player play natively from a fight checkpoint and
//! compare the owned RAM with the reference dumps (local-only data).
//!
//!   cargo run -p sf-engine --release --example round2_ram -- <capture> <first> <last> [scenario.json]
//!
//! With a scenario, P1 plays its input schedule (ticket #27); without one
//! P1 is idle.

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (first, last): (u32, u32) = (args[2].parse().unwrap(), args[3].parse().unwrap());
    let rom = sf_video::romset::load_program_rom(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
    )
    .expect("program ROM");
    let scenario = args.get(4).map(Path::new);
    let outcome = sf_engine::run::compare_cpu_ram_with_inputs(
        Path::new(&args[1]),
        rom,
        scenario,
        first,
        last,
    )
    .expect("inputs");
    println!("{outcome:x?}");
}
