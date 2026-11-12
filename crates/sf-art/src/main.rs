//! The art pipeline's tools (spec #164).
//!
//!   cargo run -p sf-art --release -- <command> ...
//!
//! - `review <fighter> [--port 8787]` serves the fighter's review page on
//!   localhost: each move of his pose catalogue playing in a real fight
//!   frame, with approve and reject on every pose. Verdicts go to
//!   `art/review/<fighter>.json`. `--recipes <dir>` reads the recipes from
//!   `art/<dir>` instead, and `--review <file>` keeps the verdicts there: a
//!   round awaiting approval (`art/pending/`) is reviewed without touching
//!   the recipes the game loads, where an unapproved pose would keep his
//!   whole set out of the game.
//! - `palettes [--port 8788]` serves the mirror palettes' review page
//!   (spec #246, SP16): all twelve fighters' second palettes, normal beside
//!   mirror on poses and real Local mirror-fight frames, with
//!   approve and reject (a reason is required to reject). Verdicts go to
//!   `art/review/<fighter>-palette.json`. `--dump <folder>` writes the page
//!   and its pictures there instead of serving.
//! - `palette <fighter>` prints his pens' colours, the palette a generation
//!   is held to.
//! - `reference <fighter> <code> <large|small> <rows> <cols> <out.png>`
//!   writes one of his ROM poses as a reference picture.
//! - `pack <fighter> <pose> <picture.png>` reduces a generated picture to
//!   his pens and puts it in the art pack, then prints what his recipe and
//!   the generation ledger need.
//! - `budget` totals the generation ledgers: each move's spend, and each
//!   month's against the cap.
//!
//! Every command takes `--roms <folder>` (default: the local ROM set) and
//! `--pack <folder>` (default: `assets/sf1-port-art`). The recipes, reviews
//! and ledgers are read from `art/` in the working copy.

use std::net::TcpListener;
use std::path::{Path, PathBuf};

use sf_art::ledger::{self, Ledger};
use sf_art::pack::{default_anchor, fighter_colours, hex, reduce, reference, stance_pens, Rgba};
use sf_art::palette_site::PaletteSite;
use sf_art::site::{serve, Site};
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art_from, ArtSources, Grid};
use sf_video::art_pack::{write_pose, ArtPack, PACK_FOLDER};
use sf_video::romset::Assets;

const USAGE: &str = "usage: sf-art review <fighter> [--port 8787] [--recipes <dir under art>] [--review <file>]
       sf-art palettes [--port 8788] [--dump <folder>]
       sf-art palette <fighter>
       sf-art reference <fighter> <code> <large|small> <rows> <cols> <out.png>
       sf-art pack <fighter> <pose> <picture.png>
       sf-art budget
  (each also takes --roms <folder> and --pack <folder>)";

/// The repository root, from this crate's place in it.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn fail(e: impl std::fmt::Display) -> ! {
    eprintln!("{e}");
    std::process::exit(1)
}

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2)
}

/// Every `.json` file in `art/<dir>`, as text.
fn texts(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(repo().join("art").join(dir)) else { return out };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.extension().is_some_and(|e| e == "json") {
            out.push(std::fs::read_to_string(&path).unwrap_or_else(|e| fail(format!("{}: {e}", path.display()))));
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let positional: Vec<&str> = {
        let mut out = Vec::new();
        let mut i = 1;
        while i < args.len() {
            if args[i].starts_with("--") {
                i += 2;
            } else {
                out.push(args[i].as_str());
                i += 1;
            }
        }
        out
    };
    let roms = flag(&args, "--roms").map(PathBuf::from).unwrap_or_else(|| repo().join("tools/reference/local/roms/sf"));
    let pack_dir = flag(&args, "--pack").map(PathBuf::from).unwrap_or_else(|| repo().join("assets").join(PACK_FOLDER));
    if positional.first() == Some(&"budget") {
        return budget();
    }
    let fighter = || {
        let name = positional.get(1).unwrap_or_else(|| usage());
        TournamentFighter::from_name(name).unwrap_or_else(|e| fail(e))
    };
    let rom = || {
        let manifest = repo().join("tools/reference/rom-manifest.json");
        sf_video::romset::load_assets(&roms, &manifest).unwrap_or_else(|e| fail(e))
    };
    match positional.first() {
        Some(&"review") => {
            let recipes = flag(&args, "--recipes").unwrap_or_else(|| "recipes".to_owned());
            let review_file = flag(&args, "--review").map(PathBuf::from);
            review(&rom(), fighter(), &pack_dir, flag(&args, "--port").unwrap_or_else(|| "8787".to_owned()), &recipes, review_file)
        }
        Some(&"palettes") => palettes(&rom(), &pack_dir, flag(&args, "--port").unwrap_or_else(|| "8788".to_owned()), flag(&args, "--dump")),
        Some(&"palette") => {
            let colours = fighter_colours(&rom(), fighter()).unwrap_or_else(|e| fail(e));
            for (pen, c) in colours.iter().enumerate() {
                let note = if pen == 15 { " (transparent)" } else { "" };
                println!("pen {pen:2}: {}{note}", hex(*c));
            }
            let opaque: Vec<String> = colours[..15].iter().map(|c| format!("\"{}\"", hex(*c))).collect();
            println!("[{}]", opaque.join(", "));
        }
        Some(&"reference") => {
            let [code, size, rows, cols, out] = [2, 3, 4, 5, 6].map(|i| *positional.get(i).unwrap_or_else(|| usage()));
            let num = |s: &str| s.parse::<u8>().unwrap_or_else(|_| usage());
            let code = u16::from_str_radix(code.trim_start_matches("0x"), 16).unwrap_or_else(|_| usage());
            let grid = Grid { code, large: size == "large", rows: num(rows), cols: num(cols) };
            let rom = rom();
            let colours = fighter_colours(&rom, fighter()).unwrap_or_else(|e| fail(e));
            let picture = reference(&rom, &grid, &colours);
            std::fs::write(out, picture.png()).unwrap_or_else(|e| fail(format!("{out}: {e}")));
            println!("{out}: {}x{}", picture.width, picture.height);
        }
        Some(&"pack") => {
            let (Some(pose), Some(picture)) = (positional.get(2), positional.get(3)) else { usage() };
            pack(&rom(), fighter(), pose, Path::new(picture), &pack_dir);
        }
        _ => usage(),
    }
}

fn review(rom: &Assets, fighter: TournamentFighter, pack_dir: &Path, port: String, recipes_dir: &str, review_file: Option<PathBuf>) {
    let recipes = texts(recipes_dir);
    let refs: Vec<&str> = recipes.iter().map(String::as_str).collect();
    let pack = ArtPack::open(pack_dir);
    if let Err(e) = &pack {
        println!("no art pack: {e}");
    }
    let no_pack = pack.as_ref().err().cloned().unwrap_or_default();
    let sources = ArtSources { recipes: &refs, reviews: None, pack: pack.as_ref().map_err(|_| no_pack.as_str()) };
    let art = load_art_from(rom, &sources).unwrap_or_else(|e| fail(format!("the recipes: {e}")));
    for (who, why) in &art.kept {
        println!("{who} cannot be shown: {why}");
    }
    let name = fighter.name().to_ascii_lowercase();
    let review_path = review_file.unwrap_or_else(|| repo().join("art/review").join(format!("{name}.json")));
    let site = Site::build(rom, &art, fighter, review_path).unwrap_or_else(|e| fail(e));
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("a free local port");
    println!("review page for {name}: http://127.0.0.1:{port}/");
    site.serve(listener);
}

fn palettes(rom: &Assets, pack_dir: &Path, port: String, dump: Option<String>) {
    let recipes = texts("recipes");
    let refs: Vec<&str> = recipes.iter().map(String::as_str).collect();
    let pack = ArtPack::open(pack_dir);
    if let Err(e) = &pack {
        println!("no art pack, so poses that are generated are left out: {e}");
    }
    let no_pack = pack.as_ref().err().cloned().unwrap_or_default();
    let sources = ArtSources { recipes: &refs, reviews: None, pack: pack.as_ref().map_err(|_| no_pack.as_str()) };
    let art = match load_art_from(rom, &sources) {
        Ok(art) => Some(art),
        Err(e) => {
            println!("no pose catalogues: {e}");
            None
        }
    };
    let site = PaletteSite::build(rom, art.as_ref(), &repo().join("art")).unwrap_or_else(|e| fail(e));
    if let Some(folder) = dump {
        let folder = PathBuf::from(folder);
        std::fs::create_dir_all(&folder).unwrap_or_else(|e| fail(format!("{}: {e}", folder.display())));
        std::fs::write(folder.join("index.html"), site.page()).unwrap_or_else(|e| fail(e));
        for (path, bytes) in site.files() {
            let at = folder.join(path);
            std::fs::create_dir_all(at.parent().unwrap()).unwrap_or_else(|e| fail(e));
            std::fs::write(&at, bytes).unwrap_or_else(|e| fail(format!("{}: {e}", at.display())));
        }
        println!("{}: the page and {} pictures", folder.display(), site.files().len());
        return;
    }
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("a free local port");
    println!("mirror palettes review page: http://127.0.0.1:{port}/");
    serve(site, listener);
}

fn pack(rom: &Assets, fighter: TournamentFighter, pose: &str, picture: &Path, pack_dir: &Path) {
    let colours = fighter_colours(rom, fighter).unwrap_or_else(|e| fail(e));
    let bytes = std::fs::read(picture).unwrap_or_else(|e| fail(format!("{}: {e}", picture.display())));
    let rgba = Rgba::read_png(&bytes).unwrap_or_else(|e| fail(format!("{}: {e}", picture.display())));
    let reduced = reduce(&rgba, &colours, &stance_pens(rom, fighter)).unwrap_or_else(|e| fail(e));
    let name = fighter.name().to_ascii_lowercase();
    let crc = write_pose(pack_dir, &name, pose, &reduced.pens, &colours).unwrap_or_else(|e| fail(e));
    // Read it back as the game will, so a pose that does not load is never
    // reported as packed.
    let back = ArtPack::open(pack_dir).unwrap_or_else(|e| fail(format!("the pack after writing: {e}")));
    assert_eq!(back.pose(&name, pose).map(|p| &p.pens), Some(&reduced.pens), "the pack gives back what was written");
    let p = &reduced.pens;
    println!("{name} {pose}: {}x{} pixels in his pens; {} exact, {} nearest, {} soft alpha", p.width, p.height, reduced.exact, reduced.nearest, reduced.soft_alpha);
    println!("packed as {}/{name}/{pose}.png, CRC-32 {crc:08x}", pack_dir.display());
    let [x, y] = default_anchor(p);
    println!("recipe: {{ \"name\": \"{pose}\", \"pack\": {{ \"crc32\": \"{crc:08x}\" }}, \"anchor\": [{x}, {y}] }}");
}

fn budget() {
    let mut ledgers = Vec::new();
    for text in texts("generations") {
        ledgers.push(serde_json::from_str::<Ledger>(&text).unwrap_or_else(|e| fail(format!("a ledger: {e}"))));
    }
    for l in &ledgers {
        for (m, spent) in l.by_move() {
            println!("{} {m}: {spent} generations", l.fighter);
        }
    }
    for (month, spent) in ledger::by_cycle(&ledgers).unwrap_or_else(|e| fail(e)) {
        println!("the month from {month}: {spent} of the {} the project may spend ({} left)", ledger::CAP, ledger::CAP.saturating_sub(spent));
    }
    if let Err(e) = ledger::check(&ledgers) {
        fail(e);
    }
}
