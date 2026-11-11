//! Debug aid: run a versus scenario natively and compare, frame by frame,
//! the memory the translated routines own with the reference capture's
//! work-RAM dumps (local-only data); reports the first differing byte.
//!
//!   cargo run -p sf-engine --release --example compare_ram -- \
//!       <reference capture> <scenario.json> <last frame>

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (reference, scenario) = (Path::new(&args[1]), Path::new(&args[2]));
    let last: u32 = args[3].parse().unwrap();
    let rom = sf_video::romset::load_program_rom(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
    )
    .expect("program ROM");
    match sf_engine::run::compare_versus_ram(reference, rom, scenario, last).expect("inputs") {
        Ok(n) => println!("compared regions identical over {n} frames through {last}"),
        Err(e) => println!("{e}"),
    }
}
