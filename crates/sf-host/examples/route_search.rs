//! Search the real game for a fighter's routes in an era style and write
//! them as a route book (SC3, `#195`, and SC13, `#205`; spec `#192`).
//! Nothing is drawn. What a route book holds and how the search works is in
//! `sf_engine::route_book`.
//!
//! `cargo run -p sf-host --release --example route_search -- <fighter> <style> [nodes] [keep]`
//!
//! A fighter is named as on the select screen (`ryu`). A style is a
//! revision, with a loadout after a slash when it is not the first one
//! (`sf6`, `sf3threes/art2`). `nodes` is how many states the search expands
//! from for each way a route can start (default 40) and `keep` how many
//! routes it keeps for each (default 8).
//!
//! `cargo run -p sf-host --release --example route_search -- windows [only]`
//!
//! measures every checked-in route's step windows again and rewrites the
//! books whose windows change (see `route_book::set_windows`).
//!
//! `cargo run -p sf-host --release --example route_search -- cuts [only]`
//!
//! measures every checked-in route's cut points again (where it may be cut
//! on purpose for a combo reset, SC10 `#202`; see `route_book::Cut`) and
//! rewrites the books whose cuts change. A search marks them as it goes.
//!
//! `cargo run -p sf-host --release --example route_search -- all [threads] [nodes] [keep] [only] [--keep-books]`
//!
//! searches every fighter in every style (only those whose route book file
//! name contains `only`, when given), `threads` at a time (default 8). With
//! `--keep-books` a pair whose book is already written is left as it is:
//! to go on from a search stopped partway, or to keep a book the Fair CPU
//! was tuned against (Ryu's SF6 one).
//!
//! Writes `crates/sf-engine/route-books/<fighter>_<revision>_<loadout>.routes`,
//! and with `all` also `books.rs`, the list `route_book::BOOKS` compiles
//! in (every `.routes` file there), and `unbooked.txt`, each pair with no
//! book and why. A single search leaves the list alone: a new book must
//! then be added to `books.rs`. The route proof test
//! (`tests/route_books.rs`) replays every listed route. A new or searched
//! again book has no `fails-on` entries: run `route_check` after.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{listed_families, listed_revisions, loadouts, Loadout, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::{books, file_name, ground_actions, measure_cuts, opponent_key, opponents, position, replay, search, set_windows, Budget, Pair, RouteBook, Searched, Stance};
use sf_host::log::Log;
use sf_video::romset::load_assets;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const BOOKS: &str = "crates/sf-engine/route-books";
const USAGE: &str = "route_search <fighter> <style> [nodes] [keep]\nroute_search all [threads] [nodes] [keep] [only] [--keep-books]\nroute_search windows [only]\nroute_search cuts [only]";
/// Why a 1987 pair has no book.
const NO_COMBOS: &str = "the 1987 rules allow no combo: a hit fighter cannot be hit again";

fn plain(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase()
}

fn style(fighter: TournamentFighter, text: &str) -> Result<(Revision, Loadout), String> {
    let (revision, wanted) = text.split_once('/').unwrap_or((text, ""));
    let revision = listed_families(fighter, true)
        .into_iter()
        .flat_map(|family| listed_revisions(fighter, family, true))
        .find(|r| plain(&format!("{r:?}")) == plain(revision))
        .ok_or_else(|| format!("{} has no style {revision}", fighter.name()))?;
    let all = loadouts(revision);
    let loadout = if wanted.is_empty() { all.first().copied() } else { all.iter().copied().find(|l| plain(&format!("{l:?}")) == plain(wanted)) };
    Ok((revision, loadout.ok_or_else(|| format!("{revision:?} has no loadout {wanted}"))?))
}

fn number(args: &[String], at: usize, default: usize) -> Result<usize, String> {
    match args.get(at) {
        Some(text) => text.parse().map_err(|_| format!("{text} is not a number\n{USAGE}")),
        None => Ok(default),
    }
}

/// The program ROM and the data bank with the bosses' tables (from the art
/// pack; without it only Ryu and Ken can be searched).
fn load() -> Result<(Vec<u8>, Option<Arc<DataBank>>), String> {
    let assets = load_assets(ROMS, MANIFEST).map_err(|e| e.to_string())?;
    let mut log = Log::create(&std::env::temp_dir().join("route_search.log"));
    let art = sf_host::art::load_with_fallbacks(&assets, &[Path::new(ROMS)], &[Path::new("assets").to_path_buf()], &mut log);
    let data = art.as_ref().map(|a| Arc::new(DataBank::build(&assets.maincpu, &a.catalogues)));
    Ok((assets.maincpu, data))
}

fn search_one(rom: &[u8], data: &Option<Arc<DataBank>>, fighter: TournamentFighter, style: (Revision, Loadout), budget: Budget) -> Result<Searched, String> {
    if data.is_none() && !fighter.is_shoto() {
        return Err("the art pack did not load, and only Ryu and Ken can be searched without it".into());
    }
    let data = Some(data.clone().unwrap_or_else(|| Arc::new(DataBank::build(rom, &[]))));
    search(&Pair { program_rom: rom, data, fighter, style }, &ground_actions(), budget)
}

fn write_book(book: &RouteBook) -> Result<PathBuf, String> {
    let out = PathBuf::from(BOOKS).join(file_name(book.fighter, book.revision, book.loadout));
    std::fs::write(&out, book.to_text()).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(out)
}

fn search_pair(args: &[String]) -> Result<(), String> {
    let name = &args[1];
    let fighter = TournamentFighter::ALL.into_iter().find(|f| plain(f.name()) == plain(name)).ok_or_else(|| format!("no fighter named {name}"))?;
    let style = style(fighter, args.get(2).ok_or(USAGE)?)?;
    let budget = Budget { nodes: number(args, 3, 40)?, keep: number(args, 4, 8)? };
    let (rom, data) = load()?;
    let started = std::time::Instant::now();
    let Searched { book, skipped } = search_one(&rom, &data, fighter, style, budget)?;
    for (reach, why) in &skipped {
        println!("not searched from a {reach:?}: {why}");
    }
    for route in &book.routes {
        println!("{:8} {} hits {:4} damage  {}", format!("{:?}", route.reach), route.hits, route.damage, route.name());
    }
    println!("{} routes in {:?}", book.routes.len(), started.elapsed());
    println!("wrote {}", write_book(&book)?.display());
    Ok(())
}

/// Every pair, `threads` searches at a time; then the list of books and
/// the pairs without one.
fn search_every_pair(args: &[String]) -> Result<(), String> {
    let threads = number(args, 2, 8)?.max(1);
    let budget = Budget { nodes: number(args, 3, 40)?, keep: number(args, 4, 8)? };
    let keep_books = args.iter().any(|a| a == "--keep-books");
    let only = args.get(5).filter(|a| !a.starts_with("--")).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let (rom, data) = load()?;
    let pairs: Vec<_> = opponents()
        .into_iter()
        .filter(|&(f, (r, l))| file_name(f, r, l).contains(&only))
        .filter(|&(f, (r, l))| !(keep_books && Path::new(BOOKS).join(file_name(f, r, l)).exists()))
        .collect();
    println!("{} pairs, {threads} at a time", pairs.len());
    let queue = Mutex::new(pairs.into_iter());
    // Why each pair searched now has no book; `None` when it has one.
    let outcomes: Mutex<BTreeMap<String, Option<String>>> = Mutex::new(BTreeMap::new());
    let started = std::time::Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            // A search is deep in the translated program's calls.
            let (rom, data, queue, outcomes) = (&rom, &data, &queue, &outcomes);
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(scope, move || loop {
                    let Some((fighter, style)) = queue.lock().unwrap().next() else { break };
                    let key = opponent_key(fighter, style);
                    let path = PathBuf::from(BOOKS).join(file_name(fighter, style.0, style.1));
                    let why = if style.0 == Revision::Arcade1987 {
                        let _ = std::fs::remove_file(&path);
                        Some(NO_COMBOS.to_owned())
                    } else {
                        let began = std::time::Instant::now();
                        let found = search_one(rom, data, fighter, style, budget);
                        let skipped: Vec<String> = found.iter().flat_map(|f| &f.skipped).map(|(r, why)| format!("{r:?}: {why}")).collect();
                        let why = match found {
                            Ok(Searched { book, .. }) if !book.routes.is_empty() => write_book(&book).err(),
                            Ok(_) => Some("the search found no route of two hits or more".to_owned()),
                            Err(e) => Some(format!("the search could not run: {e}")),
                        };
                        if why.is_some() {
                            let _ = std::fs::remove_file(&path);
                        }
                        let note = if skipped.is_empty() { String::new() } else { format!("; not searched from {}", skipped.join("; ")) };
                        println!("{key}: {}{note} in {:.0?} ({:.0?} in all)", why.as_deref().unwrap_or("written"), began.elapsed(), started.elapsed());
                        why
                    };
                    outcomes.lock().unwrap().insert(key, why);
                })
                .unwrap();
        }
    });
    let outcomes = outcomes.into_inner().unwrap();

    // Pairs not searched now keep the reason they had.
    let listed = Path::new(BOOKS).join("unbooked.txt");
    let mut unbooked: BTreeMap<String, String> = std::fs::read_to_string(&listed)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once(" | ").map(|(k, w)| (k.to_owned(), w.to_owned())))
        .collect();
    for (key, why) in outcomes {
        match why {
            Some(why) => unbooked.insert(key, why),
            None => unbooked.remove(&key),
        };
    }
    // In the order the cast and styles are listed.
    let order: Vec<String> = opponents().into_iter().map(|(f, s)| opponent_key(f, s)).collect();
    let mut text = String::from("# Pairs with no route book, and why. Written by `route_search all`.\n");
    for key in order.iter().filter(|k| unbooked.contains_key(*k)) {
        text.push_str(&format!("{key} | {}\n", unbooked[key]));
    }
    std::fs::write(&listed, text).map_err(|e| e.to_string())?;

    let mut files: Vec<String> = std::fs::read_dir(BOOKS)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".routes"))
        .collect();
    let file_order: Vec<String> = opponents().into_iter().map(|(f, (r, l))| file_name(f, r, l)).collect();
    files.sort_by_key(|name| file_order.iter().position(|n| n == name).unwrap_or(usize::MAX));
    let mut list = String::from("// The checked-in route books, written by `route_search all`\n// (`crates/sf-host/examples/route_search.rs`). Do not edit by hand.\n&[\n");
    for name in &files {
        list.push_str(&format!("    include_str!(\"{name}\"),\n"));
    }
    list.push_str("]\n");
    std::fs::write(Path::new(BOOKS).join("books.rs"), list).map_err(|e| e.to_string())?;
    println!("{} books, {} pairs without one, in {:.0?}", files.len(), unbooked.len(), started.elapsed());
    Ok(())
}

/// Measure every checked-in route's step windows again from its reference
/// position (`route_book::set_windows`), keeping all else in its book, and
/// write the books whose windows changed: a window must not take in a wait
/// that spends meter the book does not record. Only books whose file name
/// contains `only`, when given.
fn measure_windows_again(args: &[String]) -> Result<(), String> {
    let only = args.get(2).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let (rom, data) = load()?;
    let data = data.unwrap_or_else(|| Arc::new(DataBank::build(&rom, &[])));
    for book in books().iter().filter(|b| file_name(b.fighter, b.revision, b.loadout).contains(&only)) {
        let mut new = book.clone();
        let mut bases = std::collections::HashMap::new();
        for route in &mut new.routes {
            let base = bases.entry(route.reach).or_insert_with(|| {
                position(&rom, Some(data.clone()), book.fighter, (book.revision, book.loadout), route.reach, Stance::Standing).ok()
            });
            let Some(base) = base else { return Err(format!("{}: no reference position for {:?}", opponent_key(book.fighter, (book.revision, book.loadout)), route.reach)) };
            let first_hit = replay(base, &route.script(None), Stance::Standing).first_hit;
            set_windows(base, route, first_hit);
        }
        if new != *book {
            let changed: Vec<String> = new.routes.iter().zip(&book.routes).filter(|(a, b)| a != b).map(|(a, _)| a.name()).collect();
            println!("{}: {}", opponent_key(book.fighter, (book.revision, book.loadout)), changed.join("; "));
            write_book(&new)?;
        }
    }
    Ok(())
}

/// Measure every checked-in route's cut points again from its reference
/// positions (`route_book::measure_cuts`), keeping all else in its book,
/// and write the books whose cuts changed. Only books whose file name
/// contains `only`, when given.
fn measure_cuts_again(args: &[String]) -> Result<(), String> {
    let only = args.get(2).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let (rom, data) = load()?;
    let data = data.unwrap_or_else(|| Arc::new(DataBank::build(&rom, &[])));
    let (mut written, mut marked) = (0, 0);
    for book in books().iter().filter(|b| file_name(b.fighter, b.revision, b.loadout).contains(&only)) {
        let mut new = book.clone();
        let mut bases = std::collections::HashMap::new();
        for route in &mut new.routes {
            let mut base = |stance: Stance| {
                bases
                    .entry((route.reach, stance == Stance::Crouching))
                    .or_insert_with(|| position(&rom, Some(data.clone()), book.fighter, (book.revision, book.loadout), route.reach, stance).ok())
                    .clone()
            };
            let (standing, crouching) = (base(Stance::Standing), base(Stance::Crouching));
            let Some(standing) = standing else {
                return Err(format!("{}: no reference position for {:?}", opponent_key(book.fighter, (book.revision, book.loadout)), route.reach));
            };
            route.cuts = measure_cuts(&standing, crouching.as_ref(), route);
            marked += usize::from(!route.cuts.is_empty());
        }
        if new != *book {
            written += 1;
            write_book(&new)?;
        }
    }
    println!("{written} books written; {marked} routes with a cut point");
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        None => Err(USAGE.into()),
        Some("all") => search_every_pair(&args),
        Some("windows") => measure_windows_again(&args),
        Some("cuts") => measure_cuts_again(&args),
        Some(_) => search_pair(&args),
    }
}

fn main() {
    // A search is deep in the translated program's calls.
    let worker = std::thread::Builder::new().stack_size(256 << 20).spawn(run).unwrap();
    if let Err(e) = worker.join().unwrap() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
