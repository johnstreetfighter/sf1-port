//! The measuring run (SC1, `#193`): headless matches with the Fair CPU on
//! both pads, one row of numbers per match. Nothing is drawn and no window
//! opens. What each number means is in `sf_engine::measure`.
//!
//! `cargo run -p sf-host --release --example cpu_measure -- <out.tsv> <p1> <p1 style> <p1 cpu> <p2> <p2 style> <p2 cpu> <level> <first seed> <count> [threads]`
//!
//! A fighter is named as on the select screen (`ryu`, `sagat`). A style is a
//! revision, with a loadout after a slash when it is not the first one:
//! `sf6`, `st`, `sf3threes/art2`, `sf5/skill1trigger2`. A side's CPU is `old`
//! (the Fair CPU players meet today) or `new` (the stronger play of spec
//! #192, as far as it is built). The level is 1 (Easy) to 4 (Very
//! Difficult), for both sides. Seeds run from `<first seed>` for
//! `<count>` matches; the same seed gives the same row.
//!
//! Rows are appended to `<out.tsv>` in seed order once every match has
//! run, under a heading row when the file is new, and printed. A match that
//! fails leaves a `# seed N failed` line in its place. The art pack's
//! loading notes go to a `.log` file beside the output.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{listed_families, listed_revisions, loadouts, Loadout, Revision};
use sf_engine::measure::{measure_match, MatchReport, MeasureSpec};
use sf_engine::roster::TournamentFighter;
use sf_host::log::Log;
use sf_video::romset::load_assets;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const USAGE: &str = "cpu_measure <out.tsv> <p1> <p1 style> <p1 cpu: old|new> <p2> <p2 style> <p2 cpu: old|new> <level> <first seed> <count> [threads]";

fn plain(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase()
}

fn fighter(name: &str) -> Result<TournamentFighter, String> {
    TournamentFighter::ALL
        .into_iter()
        .find(|f| plain(f.name()) == plain(name))
        .ok_or_else(|| format!("no fighter named {name}"))
}

fn style(fighter: TournamentFighter, text: &str) -> Result<(Revision, Loadout), String> {
    let (revision, wanted) = text.split_once('/').unwrap_or((text, ""));
    let revision = listed_families(fighter, true)
        .into_iter()
        .flat_map(|family| listed_revisions(fighter, family, true))
        .find(|r| plain(&format!("{r:?}")) == plain(revision))
        .ok_or_else(|| format!("{} has no style {revision}", fighter.name()))?;
    let all = loadouts(revision);
    let loadout = if wanted.is_empty() {
        all.first().copied()
    } else {
        all.iter().copied().find(|l| plain(&format!("{l:?}")) == plain(wanted))
    };
    Ok((revision, loadout.ok_or_else(|| format!("{revision:?} has no loadout {wanted}"))?))
}

fn stronger(text: &str) -> Result<bool, String> {
    match text {
        "old" => Ok(false),
        "new" => Ok(true),
        _ => Err(format!("a CPU is old or new, not {text}\n{USAGE}")),
    }
}

fn number<T: std::str::FromStr>(args: &[String], at: usize, what: &str) -> Result<T, String> {
    args.get(at).and_then(|t| t.parse().ok()).ok_or_else(|| format!("{what} missing or not a number\n{USAGE}"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(args.get(1).ok_or(USAGE)?);
    let names = |at: usize| args.get(at).map(String::as_str).ok_or(USAGE);
    let fighters = [fighter(names(2)?)?, fighter(names(5)?)?];
    let styles = [style(fighters[0], names(3)?)?, style(fighters[1], names(6)?)?];
    let cpus = [stronger(names(4)?)?, stronger(names(7)?)?];
    let difficulty: u8 = number(&args, 8, "level")?;
    let first: u64 = number(&args, 9, "first seed")?;
    let count: u64 = number(&args, 10, "count")?;
    let threads: usize = if args.len() > 11 { number(&args, 11, "threads")? } else { 4 };
    let threads = threads.max(1);

    let assets = load_assets(ROMS, MANIFEST).map_err(|e| e.to_string())?;
    // The bosses' player tables come from the art pack; Ryu and Ken need none.
    let mut log = Log::create(&out.with_extension("log"));
    let art = sf_host::art::load_with_fallbacks(&assets, &[Path::new(ROMS)], &[Path::new("assets").to_path_buf()], &mut log);
    if art.is_none() && fighters.iter().any(|f| !f.is_shoto()) {
        return Err("the art pack did not load, and only Ryu and Ken can be played without it".into());
    }
    let catalogues = art.as_ref().map_or(&[][..], |a| &a.catalogues[..]);
    let data = Arc::new(DataBank::build(&assets.maincpu, catalogues));
    let rom = Arc::new(assets.maincpu);

    let queue = Arc::new(Mutex::new(first..first.saturating_add(count)));
    let workers: Vec<_> = (0..threads)
        .map(|_| {
            let (queue, rom, data) = (queue.clone(), rom.clone(), data.clone());
            // A whole match is deep in the translated program's calls.
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn(move || {
                    let mut rows = Vec::new();
                    loop {
                        let Some(seed) = queue.lock().unwrap().next() else { break };
                        let spec = MeasureSpec { fighters, styles, difficulty, seed, stronger: cpus };
                        rows.push((seed, measure_match(&rom, Some(data.clone()), &spec)));
                    }
                    rows
                })
                .unwrap()
        })
        .collect();
    let mut rows: Vec<_> = workers.into_iter().flat_map(|w| w.join().unwrap()).collect();
    rows.sort_by_key(|(seed, _)| *seed);

    let new = !out.exists();
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&out).map_err(|e| e.to_string())?;
    let mut put = |line: String| {
        println!("{line}");
        writeln!(file, "{line}").map_err(|e| e.to_string())
    };
    if new {
        put(MatchReport::header())?;
    }
    let mut failed = 0;
    for (seed, row) in rows {
        match row {
            Ok(report) => put(report.row())?,
            Err(e) => {
                failed += 1;
                put(format!("# seed {seed} failed: {e}"))?;
            }
        }
    }
    if failed == 0 { Ok(()) } else { Err(format!("{failed} matches failed")) }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
