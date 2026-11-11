//! sf-compare: scenario comparison runner.
//!
//! Validates local input data (reference ROM set), loads a scenario and two
//! observation captures, and reports the first observable mismatch.
//!
//! Exit codes: 0 = observations match, 1 = first mismatch reported,
//! 2 = input/validation failure (invalid ROMs, absent capture, incompatible
//! baseline, invalid scenario).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope};
use sf_compare::scenario::load_scenario;

#[derive(Parser)]
#[command(
    name = "sf-compare",
    about = "Scenario comparison runner for the SF1 port"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify the local ROM set against the committed manifest.
    VerifyRoms {
        /// Path to the ROM manifest JSON.
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
        /// Directory holding the extracted ROM files.
        #[arg(long, default_value = "tools/reference/local/roms/sf")]
        roms: PathBuf,
    },
    /// Compare a candidate capture against a reference capture.
    Compare {
        /// Scenario JSON describing reference identity, inputs, duration, observations.
        #[arg(long)]
        scenario: PathBuf,
        /// Capture directory produced by the pinned reference baseline.
        #[arg(long)]
        reference: PathBuf,
        /// Capture directory produced by the candidate (native or reference).
        #[arg(long)]
        candidate: PathBuf,
        /// What produced the candidate capture. 'reference' marks the run as a
        /// reference-versus-reference self-check; it is never native behavior.
        #[arg(long, value_enum)]
        candidate_kind: CliCandidateKind,
        /// Also verify the local ROM set before comparing (documented invocation).
        #[arg(long)]
        roms: Option<PathBuf>,
        /// ROM manifest path used together with --roms.
        #[arg(long, default_value = "tools/reference/rom-manifest.json")]
        manifest: PathBuf,
        /// First frame to compare (inclusive). Bounds the walk to a slice.
        #[arg(long)]
        from: Option<u64>,
        /// Last frame to compare (inclusive). Bounds the walk to a slice.
        #[arg(long)]
        to: Option<u64>,
        /// Comma-separated subset of frame observables to check
        /// (screen_hash,latch,latch_changed,ram_crc,music_crc). Columns not
        /// listed are explicitly not validated by this run.
        #[arg(long)]
        fields: Option<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CliCandidateKind {
    Reference,
    Native,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::VerifyRoms { manifest, roms } => {
            match sf_compare::roms::verify_roms(&manifest, &roms) {
                Ok(()) => {
                    println!("ROM set verified against {}", manifest.display());
                    ExitCode::from(0)
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::from(2)
                }
            }
        }
        Commands::Compare {
            scenario,
            reference,
            candidate,
            candidate_kind,
            roms,
            manifest,
            from,
            to,
            fields,
        } => {
            let scope = match build_scope(from, to, fields) {
                Ok(scope) => scope,
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            };
            run_compare(
                scenario,
                reference,
                candidate,
                candidate_kind,
                roms,
                manifest,
                scope,
            )
        }
    }
}

fn build_scope(
    from: Option<u64>,
    to: Option<u64>,
    fields: Option<String>,
) -> Result<CompareScope, String> {
    let parsed = fields
        .as_deref()
        .map(CompareScope::parse_fields)
        .transpose()?;
    Ok(CompareScope {
        from,
        to,
        fields: parsed,
    })
}

fn run_compare(
    scenario_path: PathBuf,
    reference_path: PathBuf,
    candidate_path: PathBuf,
    candidate_kind: CliCandidateKind,
    roms: Option<PathBuf>,
    manifest: PathBuf,
    scope: CompareScope,
) -> ExitCode {
    if let Some(roms) = roms {
        if let Err(e) = sf_compare::roms::verify_roms(&manifest, &roms) {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
        println!("ROM set verified against {}", manifest.display());
    }

    let run = (|| -> Result<(String, bool), Box<dyn std::error::Error>> {
        let scenario = load_scenario(&scenario_path)?;
        let reference = load_capture(&reference_path)?;
        let candidate = load_capture(&candidate_path)?;
        let kind = match candidate_kind {
            CliCandidateKind::Reference => CandidateKind::Reference,
            CliCandidateKind::Native => CandidateKind::Native,
        };
        let outcome = compare(&scenario, &reference, &candidate, kind, &scope)?;
        let matched = matches!(outcome, sf_compare::compare::Outcome::Match { .. });
        let report =
            sf_compare::report::render(&outcome, &scenario, &reference, &candidate, kind, &scope);
        Ok((report, matched))
    })();

    match run {
        Ok((report, true)) => {
            print!("{report}");
            ExitCode::from(0)
        }
        Ok((report, false)) => {
            print!("{report}");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}
