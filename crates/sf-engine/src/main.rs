//! sf-engine: native capture runner for bounded SF1 scene slices.
//!
//! Produces observation captures from native Rust state updates, for
//! comparison against the pinned MAME reference via sf-compare:
//!
//!   sf-engine capture-idle --fixture-capture <reference capture dir> \
//!       --roms tools/reference/local/roms/sf --out <native capture dir>
//!
//!   sf-engine capture-versus --fixture-capture <reference capture dir> \
//!       --scenario tools/reference/scenarios/sf_move_01.json \
//!       --roms tools/reference/local/roms/sf --out <native capture dir>
//!
//!   sf-engine capture-crouch --fixture-capture <reference capture dir> \
//!       --scenario tools/reference/scenarios/sf_crouch_01.json \
//!       --roms tools/reference/local/roms/sf --out <native capture dir>
//!
//!   sf-engine capture-jump --fixture-capture <reference capture dir> \
//!       --scenario tools/reference/scenarios/sf_jump_01.json \
//!       --roms tools/reference/local/roms/sf --out <native capture dir>
//!
//!   sf-engine capture-punch --fixture-capture <reference capture dir> //!       --scenario tools/reference/scenarios/sf_punch_01.json //!       --roms tools/reference/local/roms/sf --out <native capture dir>
//!
//!   sf-engine capture-cpu --fixture-capture <reference capture dir> //!       --roms tools/reference/local/roms/sf --out <native capture dir>
//!
//!   sf-compare compare --scenario tools/reference/scenarios/sf_move_01.json \
//!       --reference <reference capture> --candidate <native capture> \
//!       --candidate-kind native --from 2450 --to 3350 \
//!       --fields screen_hash,latch,latch_changed

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sf-engine",
    about = "Native SF1 machine-state runner (bounded slices)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the idle-scene interval and write a native observation capture.
    #[command(name = "capture-idle")]
    Idle {
        /// Reference capture holding the checkpoint-2280 fixture dumps
        /// (development fixture; disclosed in docs/reference/idle-scene.md).
        #[arg(long)]
        fixture_capture: PathBuf,
        /// Directory holding the extracted ROM files.
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        /// ROM manifest JSON.
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
        /// Output capture directory.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run the versus movement interval (sf-move-01, ticket #6) and write a
    /// native observation capture.
    #[command(name = "capture-versus")]
    Versus {
        /// Reference capture holding the checkpoint-2450 fixture dumps
        /// (development fixture; disclosed in docs/reference/movement.md).
        #[arg(long)]
        fixture_capture: PathBuf,
        /// Scenario JSON carrying the P1 direction input schedule.
        #[arg(long, default_value = "tools/reference/scenarios/sf_move_01.json")]
        scenario: PathBuf,
        /// Directory holding the extracted ROM files.
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        /// ROM manifest JSON.
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
        /// Output capture directory.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run the versus crouch interval (sf-crouch-01, ticket #7) and write a
    /// native observation capture.
    #[command(name = "capture-crouch")]
    Crouch {
        /// Reference capture holding the checkpoint-2450 fixture dumps
        /// (development fixture; disclosed in docs/reference/movement.md).
        #[arg(long)]
        fixture_capture: PathBuf,
        /// Scenario JSON carrying the P1 direction input schedule.
        #[arg(long, default_value = "tools/reference/scenarios/sf_crouch_01.json")]
        scenario: PathBuf,
        /// Directory holding the extracted ROM files.
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        /// ROM manifest JSON.
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
        /// Output capture directory.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run the versus jump interval (sf-jump-01, ticket #8) and write a
    /// native observation capture.
    #[command(name = "capture-jump")]
    Jump {
        /// Reference capture holding the checkpoint-2450 fixture dumps
        /// (development fixture; disclosed in docs/reference/movement.md).
        #[arg(long)]
        fixture_capture: PathBuf,
        /// Scenario JSON carrying the P1 direction input schedule.
        #[arg(long, default_value = "tools/reference/scenarios/sf_jump_01.json")]
        scenario: PathBuf,
        /// Directory holding the extracted ROM files.
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        /// ROM manifest JSON.
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
        /// Output capture directory.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run the versus light-punch interval (sf-punch-01, ticket #9) and
    /// write a native observation capture.
    #[command(name = "capture-punch")]
    Punch {
        /// Reference capture holding the checkpoint-2450 fixture dumps
        /// (development fixture; disclosed in docs/reference/movement.md).
        #[arg(long)]
        fixture_capture: PathBuf,
        /// Scenario JSON carrying the P1 direction/button input schedule.
        #[arg(long, default_value = "tools/reference/scenarios/sf_punch_01.json")]
        scenario: PathBuf,
        /// Directory holding the extracted ROM files.
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        /// ROM manifest JSON.
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
        /// Output capture directory.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run the CPU-decision interval (sf-cpu-01, ticket #12) and write a
    /// native observation capture.
    #[command(name = "capture-cpu")]
    Cpu {
        /// Reference capture holding the checkpoint-2389 fixture dumps
        /// (development fixture; disclosed in docs/reference/cpu-decision.md).
        #[arg(long)]
        fixture_capture: PathBuf,
        /// Directory holding the extracted ROM files.
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
        /// ROM manifest JSON.
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
        /// Output capture directory.
        #[arg(long)]
        out: PathBuf,
    },
}

fn load_assets(
    roms: &std::path::Path,
    manifest: &std::path::Path,
) -> Option<sf_video::romset::Assets> {
    match sf_video::romset::load_assets(roms, manifest) {
        Ok(assets) => Some(assets),
        Err(e) => {
            eprintln!("error: ROM set failed to load: {e}");
            None
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Idle {
            fixture_capture,
            roms,
            manifest,
            out,
        } => {
            let Some(assets) = load_assets(&roms, &manifest) else {
                return ExitCode::from(2);
            };
            match sf_engine::run::run_idle_capture(
                &fixture_capture,
                &assets,
                &out,
                "sf-idle-scene-01",
            ) {
                Ok(frames) => {
                    println!(
                        "native idle capture written to {} ({frames} frames)",
                        out.display()
                    );
                    ExitCode::from(0)
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Commands::Versus {
            fixture_capture,
            scenario,
            roms,
            manifest,
            out,
        } => {
            let Some(assets) = load_assets(&roms, &manifest) else {
                return ExitCode::from(2);
            };
            match sf_engine::run::run_versus_capture(
                &fixture_capture,
                &assets,
                &out,
                "sf-move-01",
                &scenario,
            ) {
                Ok(frames) => {
                    println!(
                        "native versus capture written to {} ({frames} frames)",
                        out.display()
                    );
                    ExitCode::from(0)
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Commands::Crouch {
            fixture_capture,
            scenario,
            roms,
            manifest,
            out,
        } => {
            let Some(assets) = load_assets(&roms, &manifest) else {
                return ExitCode::from(2);
            };
            match sf_engine::run::run_crouch_capture(
                &fixture_capture,
                &assets,
                &out,
                "sf-crouch-01",
                &scenario,
            ) {
                Ok(frames) => {
                    println!(
                        "native crouch capture written to {} ({frames} frames)",
                        out.display()
                    );
                    ExitCode::from(0)
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Commands::Jump {
            fixture_capture,
            scenario,
            roms,
            manifest,
            out,
        } => {
            let Some(assets) = load_assets(&roms, &manifest) else {
                return ExitCode::from(2);
            };
            match sf_engine::run::run_jump_capture(
                &fixture_capture,
                &assets,
                &out,
                "sf-jump-01",
                &scenario,
            ) {
                Ok(frames) => {
                    println!(
                        "native jump capture written to {} ({frames} frames)",
                        out.display()
                    );
                    ExitCode::from(0)
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Commands::Punch {
            fixture_capture,
            scenario,
            roms,
            manifest,
            out,
        } => {
            let Some(assets) = load_assets(&roms, &manifest) else {
                return ExitCode::from(2);
            };
            match sf_engine::run::run_punch_capture(
                &fixture_capture,
                &assets,
                &out,
                "sf-punch-01",
                &scenario,
            ) {
                Ok(frames) => {
                    println!(
                        "native punch capture written to {} ({frames} frames)",
                        out.display()
                    );
                    ExitCode::from(0)
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Commands::Cpu {
            fixture_capture,
            roms,
            manifest,
            out,
        } => {
            let Some(assets) = load_assets(&roms, &manifest) else {
                return ExitCode::from(2);
            };
            match sf_engine::run::run_cpu_capture(&fixture_capture, &assets, &out, "sf-cpu-01") {
                Ok(frames) => {
                    println!(
                        "native cpu capture written to {} ({frames} frames)",
                        out.display()
                    );
                    ExitCode::from(0)
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(2)
                }
            }
        }
    }
}
