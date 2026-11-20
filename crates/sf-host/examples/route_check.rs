//! Check every route of every checked-in route book against every opponent
//! and write the ones each fails on into its book (SC13, `#205`; spec
//! `#192`). A developer command, kept out of the suite for its run time:
//! the suite's route proof test plays each route against the reference
//! defender only.
//!
//! `cargo run -p sf-host --release --example route_check -- [threads] [only]`
//!
//! For each book (only those whose file name contains `only`, when given),
//! each opponent fighter in each style he can be picked in, and each stance
//! the route is recorded to work on: the route's position is made with that
//! opponent holding that stance, as the search made it with the reference
//! defender, and the route is played. It fails on him when it lands fewer
//! hits than the book records, or when a punish no longer meets him
//! recovering. Each book's `fails-on` lines are written over with what this
//! run found: the opponent alone when the route fails in every stance it
//! works on, with the stance when in one only. A position that cannot be
//! made (a punish needs the opponent's sweep guarded) is counted and
//! printed, and not written as a failure.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{Loadout, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::{books, file_name, from_opening, opening, opponent_key, opponents, replay, Reach, RouteBook, Stance};
use sf_host::log::Log;
use sf_video::romset::load_assets;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const BOOKS: &str = "crates/sf-engine/route-books";
const USAGE: &str = "route_check [threads] [only]";

/// What the check has found of one book so far.
struct Findings {
    /// Units (an opponent in a stance) still to play.
    left: usize,
    /// Per route: each opponent it failed on, and in which stances.
    fails: Vec<HashMap<String, Vec<Stance>>>,
    /// Positions that could not be made, by reach.
    unmade: HashMap<Reach, usize>,
}

/// Play every route of `book` that works in `stance` against `foe` in it.
/// Each route's failure, and the reaches whose position could not be made.
fn check(
    rom: &[u8],
    data: &Arc<DataBank>,
    book: &RouteBook,
    foe: (TournamentFighter, (Revision, Loadout)),
    stance: Stance,
) -> (Vec<bool>, Vec<Reach>) {
    let mut failed = vec![false; book.routes.len()];
    let mut unmade = Vec::new();
    let wanted = |reach: Reach| book.routes.iter().any(|r| r.reach == reach && r.stances.contains(&stance));
    let Ok(open) = opening(rom, Some(data.clone()), book.fighter, (book.revision, book.loadout), Some(foe), stance) else {
        let mut reaches: Vec<Reach> = Vec::new();
        for reach in book.routes.iter().map(|r| r.reach).filter(|&r| wanted(r)) {
            if !reaches.contains(&reach) {
                reaches.push(reach);
            }
        }
        return (failed, reaches);
    };
    let mut positions = HashMap::new();
    for (i, route) in book.routes.iter().enumerate() {
        if !route.stances.contains(&stance) {
            continue;
        }
        let base = positions.entry(route.reach).or_insert_with(|| from_opening(&open, route.reach, stance).ok());
        let Some(base) = base else {
            if !unmade.contains(&route.reach) {
                unmade.push(route.reach);
            }
            continue;
        };
        let played = replay(base, &route.script(None), stance);
        failed[i] = played.hits < route.hits || (route.reach == Reach::Punish && !played.punished);
    }
    (failed, unmade)
}

/// The book with what was found as its `fails-on` lines: a fighter alone
/// when he fails it the same way in every style he has.
fn with_findings(book: &RouteBook, findings: &Findings) -> RouteBook {
    let mut out = book.clone();
    let foes = opponents();
    for (route, fails) in out.routes.iter_mut().zip(&findings.fails) {
        // In which stances it fails on an opponent: `None` for all it works on.
        let how = |key: &String| -> Option<Option<Stance>> {
            let stances = fails.get(key)?;
            Some(if route.stances.iter().all(|s| stances.contains(s)) { None } else { Some(stances[0]) })
        };
        let written = |name: String, stance: Option<Stance>| stance.map_or(name.clone(), |s| format!("{name} {}", s.name()));
        route.fails_on.clear();
        for fighter in TournamentFighter::ALL {
            let keys: Vec<String> = foes.iter().filter(|(f, _)| *f == fighter).map(|&(f, s)| opponent_key(f, s)).collect();
            let each: Vec<Option<Option<Stance>>> = keys.iter().map(how).collect();
            match each[0] {
                Some(stance) if each.iter().all(|h| *h == Some(stance)) => route.fails_on.push(written(format!("{fighter:?}"), stance)),
                _ => route.fails_on.extend(keys.iter().zip(&each).filter_map(|(key, h)| h.map(|stance| written(key.clone(), stance)))),
            }
        }
    }
    out
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let threads: usize = match args.get(1) {
        Some(t) => t.parse().map_err(|_| format!("{t} is not a number\n{USAGE}"))?,
        None => 8,
    };
    let only = args.get(2).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let assets = load_assets(ROMS, MANIFEST).map_err(|e| e.to_string())?;
    let mut log = Log::create(&std::env::temp_dir().join("route_check.log"));
    let art = sf_host::art::load_with_fallbacks(&assets, &[Path::new(ROMS)], &[Path::new("assets").to_path_buf()], &mut log)
        .ok_or("the art pack did not load: the bosses need its tables")?;
    let data = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
    drop(art);
    let rom = assets.maincpu;

    let chosen: Vec<&RouteBook> = books().iter().filter(|b| file_name(b.fighter, b.revision, b.loadout).contains(&only)).collect();
    let foes = opponents();
    let mut units = Vec::new();
    for (at, book) in chosen.iter().enumerate() {
        for &foe in &foes {
            for stance in [Stance::Standing, Stance::Crouching] {
                if book.routes.iter().any(|r| r.stances.contains(&stance)) {
                    units.push((at, foe, stance));
                }
            }
        }
    }
    let findings: Vec<Mutex<Findings>> = chosen
        .iter()
        .enumerate()
        .map(|(at, book)| {
            Mutex::new(Findings {
                left: units.iter().filter(|unit| unit.0 == at).count(),
                fails: vec![HashMap::new(); book.routes.len()],
                unmade: HashMap::new(),
            })
        })
        .collect();
    println!("{} books, {} opponents, {} positions to make, {threads} at a time", chosen.len(), foes.len(), units.len());
    let queue = Mutex::new(units.into_iter());
    let started = std::time::Instant::now();
    let errors = Mutex::new(Vec::<String>::new());
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            let (rom, data, chosen, queue, findings, errors) = (&rom, &data, &chosen, &queue, &findings, &errors);
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(scope, move || loop {
                    let Some((at, foe, stance)) = queue.lock().unwrap().next() else { break };
                    let book = chosen[at];
                    let (failed, unmade) = check(rom, data, book, foe, stance);
                    let mut found = findings[at].lock().unwrap();
                    let key = opponent_key(foe.0, foe.1);
                    for (i, &f) in failed.iter().enumerate() {
                        if f {
                            found.fails[i].entry(key.clone()).or_default().push(stance);
                        }
                    }
                    for reach in unmade {
                        *found.unmade.entry(reach).or_default() += 1;
                    }
                    found.left -= 1;
                    if found.left == 0 {
                        let out = PathBuf::from(BOOKS).join(file_name(book.fighter, book.revision, book.loadout));
                        let new = with_findings(book, &found);
                        let failing: usize = new.routes.iter().map(|r| r.fails_on.len()).sum();
                        if let Err(e) = std::fs::write(&out, new.to_text()) {
                            errors.lock().unwrap().push(format!("{}: {e}", out.display()));
                        }
                        let unmade: Vec<String> = found.unmade.iter().map(|(r, n)| format!("{r:?} {n}")).collect();
                        println!(
                            "{}: {} routes, {failing} fails-on entries{} ({:.0?} in all)",
                            opponent_key(book.fighter, (book.revision, book.loadout)),
                            new.routes.len(),
                            if unmade.is_empty() { String::new() } else { format!(", positions not made: {}", unmade.join(", ")) },
                            started.elapsed()
                        );
                    }
                })
                .unwrap();
        }
    });
    let errors = errors.into_inner().unwrap();
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    println!("done in {:.0?}", started.elapsed());
    Ok(())
}

fn main() {
    let worker = std::thread::Builder::new().stack_size(256 << 20).spawn(run).unwrap();
    if let Err(e) = worker.join().unwrap() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
