//! Search the real game for a fighter's routes in an era style and write
//! them as a route book (SC3, `#195`; spec `#192`). Nothing is drawn. What a
//! route book holds and how the search works is in `sf_engine::route_book`.
//!
//! `cargo run -p sf-host --release --example route_search -- <fighter> <style> [nodes] [keep]`
//!
//! A fighter is named as on the select screen (`ryu`). A style is a
//! revision, with a loadout after a slash when it is not the first one
//! (`sf6`, `sf3threes/art2`). `nodes` is how many states the search expands
//! from for each way a route can start (default 40) and `keep` how many
//! routes it keeps for each (default 8).
//!
//! Writes `crates/sf-engine/route-books/<fighter>_<revision>_<loadout>.routes`.
//! A new book must also be listed in `route_book::BOOKS` to be loaded; the
//! route proof test (`tests/route_books.rs`) then replays every route in it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_engine::data_bank::DataBank;
use sf_engine::era::{listed_families, listed_revisions, loadouts, Loadout, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::{ground_actions, search, Budget, Pair};
use sf_host::log::Log;
use sf_video::romset::load_assets;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const BOOKS: &str = "crates/sf-engine/route-books";
const USAGE: &str = "route_search <fighter> <style> [nodes] [keep]";

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

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let name = args.get(1).ok_or(USAGE)?;
    let fighter = TournamentFighter::ALL.into_iter().find(|f| plain(f.name()) == plain(name)).ok_or_else(|| format!("no fighter named {name}"))?;
    let style = style(fighter, args.get(2).ok_or(USAGE)?)?;
    let count = |at: usize, default: usize| match args.get(at) {
        Some(text) => text.parse().map_err(|_| format!("{text} is not a number\n{USAGE}")),
        None => Ok(default),
    };
    let budget = Budget { nodes: count(3, 40)?, keep: count(4, 8)? };

    let assets = load_assets(ROMS, MANIFEST).map_err(|e| e.to_string())?;
    let out = PathBuf::from(BOOKS).join(format!("{}_{}_{}.routes", plain(fighter.name()), plain(&format!("{:?}", style.0)), plain(&format!("{:?}", style.1))));
    // The bosses' player tables come from the art pack; Ryu and Ken need none.
    let mut log = Log::create(&std::env::temp_dir().join("route_search.log"));
    let art = sf_host::art::load_with_fallbacks(&assets, &[Path::new(ROMS)], &[Path::new("assets").to_path_buf()], &mut log);
    if art.is_none() && !fighter.is_shoto() {
        return Err("the art pack did not load, and only Ryu and Ken can be searched without it".into());
    }
    let catalogues = art.as_ref().map_or(&[][..], |a| &a.catalogues[..]);
    let data = Arc::new(DataBank::build(&assets.maincpu, catalogues));
    let pair = Pair { program_rom: &assets.maincpu, data: Some(data), fighter, style };
    let started = std::time::Instant::now();
    let book = search(&pair, &ground_actions(), budget)?;
    for route in &book.routes {
        println!("{:8} {} hits {:4} damage  {}", format!("{:?}", route.reach), route.hits, route.damage, route.name());
    }
    println!("{} routes in {:?}", book.routes.len(), started.elapsed());
    std::fs::write(&out, book.to_text()).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("wrote {}", out.display());
    Ok(())
}

fn main() {
    // A search is deep in the translated program's calls.
    let worker = std::thread::Builder::new().stack_size(256 << 20).spawn(run).unwrap();
    if let Err(e) = worker.join().unwrap() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
