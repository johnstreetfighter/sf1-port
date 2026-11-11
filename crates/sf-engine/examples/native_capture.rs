//! Debug aid: write a native capture (frames.tsv with the per-frame latch
//! and the vblank handler's cycles to its write, checkpoints) for one of
//! the translated runs, from a reference capture's fixture (local-only
//! data).
//!
//!   cargo run -p sf-engine --release --example native_capture -- punch <reference capture> <scenario.json> <out>
//!   cargo run -p sf-engine --release --example native_capture -- cpu <reference capture> <out>
//!   cargo run -p sf-engine --release --example native_capture -- fight <reference capture> <scenario.json> <first> <last> <out>
//!
//! `punch` is the versus light-punch interval (ticket #9), `cpu` the
//! CPU-decision interval from checkpoint 2389 (ticket #12), `fight`
//! one-player play from the fight checkpoint `first` through `last` with
//! P1 on the scenario's input schedule (tickets #26/#27).

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let assets = sf_video::romset::load_assets(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
    )
    .expect("ROM regions");
    let frames = match args[1].as_str() {
        "punch" => sf_engine::run::run_punch_capture(
            Path::new(&args[2]),
            &assets,
            Path::new(&args[4]),
            "sf-punch-01",
            Path::new(&args[3]),
        ),
        "cpu" => sf_engine::run::run_cpu_capture(
            Path::new(&args[2]),
            &assets,
            Path::new(&args[3]),
            "sf-cpu-01",
        ),
        "fight" => {
            let (first, last): (u32, u32) = (args[4].parse().unwrap(), args[5].parse().unwrap());
            let id = Path::new(&args[3])
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .replace('_', "-");
            sf_engine::run::run_cpu_fight_capture_with_inputs(
                Path::new(&args[2]),
                &assets,
                Path::new(&args[6]),
                &id,
                Some(Path::new(&args[3])),
                first,
                last,
            )
        }
        other => panic!("unknown run {other}"),
    }
    .expect("native run");
    println!("{frames} frames written");
}
