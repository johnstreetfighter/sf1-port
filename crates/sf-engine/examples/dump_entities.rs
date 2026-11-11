//! Debug aid: run a versus scenario natively and print fighter/effect entity
//! records (64 bytes each, hex) for a frame range, in the same layout as the
//! local reference work-RAM dumps, for side-by-side diffing.
//!
//!   cargo run -p sf-engine --release --example dump_entities -- \
//!       <fixture capture> <scenario.json> <from> <to>

use std::path::Path;

use sf_engine::versus::VersusMachine;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (fixture, scenario) = (Path::new(&args[1]), Path::new(&args[2]));
    let (from, to): (u32, u32) = (args[3].parse().unwrap(), args[4].parse().unwrap());
    let rom = sf_video::romset::load_program_rom(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
    )
    .expect("program ROM");
    let mut m = VersusMachine::from_checkpoint_2450_with_program(fixture, rom).expect("fixture");
    let events = sf_engine::run::scenario_input_events(scenario).expect("scenario");
    let mut next = 0;
    let mut held = [false; sf_engine::run::INPUTS];
    while m.state.frame < to {
        let frame = m.state.frame + 1;
        while next < events.len() && events[next].0 < frame {
            held[events[next].1] = events[next].2;
            next += 1;
        }
        sf_engine::run::apply_inputs(&mut m, &held);
        m.step();
        if m.state.frame >= from {
            println!(
                "f={} fg={:04x} latch={:02x} unrec={:x?}",
                m.state.frame,
                m.state.fg_scroll,
                m.state.sound_latch,
                m.unrecovered()
            );
            for base in [0x400usize, 0xa00, 0x100] {
                let e = &m.state.objectram[base..base + 0x40];
                let hex: Vec<String> = e
                    .chunks(8)
                    .map(|c| c.iter().map(|b| format!("{b:02x}")).collect())
                    .collect();
                println!("  ff{:04x}: {}", 0xe000 + base, hex.join(" "));
            }
        }
    }
}
