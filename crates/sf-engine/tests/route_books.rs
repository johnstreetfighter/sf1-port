//! Tickets #195 (`SC3`) and #205 (`SC13`), spec #192: route books for the
//! whole cast, the widened search and the route proofs.
//!
//! Every route in every checked-in route book is replayed in the engine
//! against the reference defender and its hits checked, so a moveset change
//! that breaks a route fails here until the route is searched again
//! (`cargo run -p sf-host --release --example route_search`). Each pair
//! with no book is listed with the reason. The check against every other
//! opponent is a developer command (`route_check`), not run here. Local ROMs
//! and the art pack only (the bosses' tables come from it); each test that
//! plays skips without them.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{Loadout, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::{
    book, book_texts, books, does_something, ground_actions, measure_cuts, opponent_key, opponents, position, replay, search, Action, Budget, Pair, Reach,
    RouteBook, Stance,
};
use sf_engine::run::PowerOnMachine;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../assets/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const UNBOOKED: &str = include_str!("../route-books/unbooked.txt");
const RECIPES: [&str; 11] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/retsu.json"),
    include_str!("../../../art/recipes/geki.json"),
    include_str!("../../../art/recipes/joe.json"),
    include_str!("../../../art/recipes/mike.json"),
    include_str!("../../../art/recipes/lee.json"),
    include_str!("../../../art/recipes/gen.json"),
    include_str!("../../../art/recipes/birdie.json"),
    include_str!("../../../art/recipes/eagle.json"),
    include_str!("../../../art/recipes/adon.json"),
    include_str!("../../../art/recipes/sagat.json"),
];
const REVIEWS: [&str; 11] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/retsu.json"),
    include_str!("../../../art/review/geki.json"),
    include_str!("../../../art/review/joe.json"),
    include_str!("../../../art/review/mike.json"),
    include_str!("../../../art/review/lee.json"),
    include_str!("../../../art/review/gen.json"),
    include_str!("../../../art/review/birdie.json"),
    include_str!("../../../art/review/eagle.json"),
    include_str!("../../../art/review/adon.json"),
    include_str!("../../../art/review/sagat.json"),
];

/// Run `body` with the program ROM, on a thread with room for the
/// translated program's deep calls.
fn with_rom(body: impl FnOnce(Vec<u8>) + Send + 'static) {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let rom = load_assets(ROMS, MANIFEST).expect("local ROM set assembles").maincpu;
    std::thread::Builder::new().stack_size(256 << 20).spawn(move || body(rom)).unwrap().join().unwrap();
}

/// The program ROM and the data bank with every fighter's tables, from the
/// art pack, or `None` without them.
fn rom_and_bank() -> Option<(Vec<u8>, Arc<DataBank>)> {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROM set or art pack not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(&assets, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).expect("the art loads");
    let bank = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
    Some((assets.maincpu, bank))
}

/// Replay every route of `book` against the reference defender: hits in
/// each stance it lists; damage, knockdown and meter against a standing
/// one; and each step at both ends of its window. The faults found.
fn prove(rom: &[u8], bank: &Arc<DataBank>, book: &RouteBook) -> Vec<String> {
    let mut faults = Vec::new();
    let mut positions: HashMap<(Reach, bool), Option<PowerOnMachine>> = HashMap::new();
    for route in &book.routes {
        let who = format!("{:?} {:?}/{:?}: {} ({:?})", book.fighter, book.revision, book.loadout, route.name(), route.reach);
        let mut fault = |what: String| faults.push(format!("{who}: {what}"));
        for &stance in &route.stances {
            let base = positions.entry((route.reach, stance == Stance::Crouching)).or_insert_with(|| {
                position(rom, Some(bank.clone()), book.fighter, (book.revision, book.loadout), route.reach, stance).ok()
            });
            let Some(base) = base else {
                fault(format!("no reference position against a {stance:?} defender"));
                continue;
            };
            let played = replay(base, &route.script(None), stance);
            if played.hits != route.hits {
                fault(format!("{} hits against a {stance:?} defender, the book says {}", played.hits, route.hits));
            }
            if stance != Stance::Standing {
                continue;
            }
            // The rest of what the book says of it, as measured against the
            // standing defender.
            if played.damage != route.damage || played.knockdown != route.knockdown {
                fault(format!("damage {} knockdown {}, the book says {} {}", played.damage, played.knockdown, route.damage, route.knockdown));
            }
            let meter = [played.super_spent, played.super_gained, played.drive_spent, played.drive_gained];
            if meter != [route.super_spent, route.super_gained, route.drive_spent, route.drive_gained] {
                fault(format!("meter {meter:?}"));
            }
            // Each step still lands at either end of its window.
            for (i, step) in route.steps.iter().enumerate() {
                for edge in [step.window.0, step.window.1] {
                    let mut waits: Vec<u8> = route.steps.iter().map(|s| s.wait).collect();
                    waits[i] = edge;
                    let played = replay(base, &route.script(Some(&waits)), stance);
                    if played.hits != route.hits {
                        fault(format!("step {} after {edge} frames: {} hits", step.name, played.hits));
                    }
                    // Nor does any wait in a window spend meter the book
                    // does not say: two quarter circles close together
                    // must not read as a Super Art's motion (the meters
                    // are full here).
                    if played.super_spent != route.super_spent {
                        fault(format!("step {} after {edge} frames: super -{}", step.name, played.super_spent));
                    }
                }
            }
        }
        // Its cut points (SC10, `#202`) are exactly those that hold: each
        // marked one leaves him reeling on his feet with the attacker free
        // first, as many frames as it says, and none that holds is missing.
        let mut base = |stance: Stance| {
            positions
                .entry((route.reach, stance == Stance::Crouching))
                .or_insert_with(|| position(rom, Some(bank.clone()), book.fighter, (book.revision, book.loadout), route.reach, stance).ok())
                .clone()
        };
        if let Some(standing) = base(Stance::Standing) {
            let crouching = base(Stance::Crouching);
            let measured = measure_cuts(&standing, crouching.as_ref(), route);
            if measured != route.cuts {
                faults.push(format!("{who}: cuts {measured:?}, the book says {:?}", route.cuts));
            }
        }
    }
    faults
}

#[test]
fn every_checked_in_book_is_the_text_it_would_be_written_as() {
    assert_eq!(books().len(), book_texts().len());
    for (book, text) in books().iter().zip(book_texts()) {
        assert_eq!(book.to_text(), text.replace("\r\n", "\n"), "{:?} {:?}", book.fighter, book.revision);
        assert_eq!(&RouteBook::parse(&book.to_text()).unwrap(), book);
    }
}

#[test]
fn every_route_in_every_checked_in_book_replays_against_the_reference_defender() {
    let Some((rom, bank)) = rom_and_bank() else { return };
    // The books shared out among a few threads, each with room for the
    // translated program's deep calls.
    let next = Mutex::new(books().iter());
    let faults = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 8);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let (rom, bank, next, faults) = (&rom, &bank, &next, &faults);
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(scope, move || loop {
                    let Some(book) = next.lock().unwrap().next() else { break };
                    let found = prove(rom, bank, book);
                    faults.lock().unwrap().extend(found);
                })
                .unwrap();
        }
    });
    let faults = faults.into_inner().unwrap();
    assert!(faults.is_empty(), "{} faults:\n{}", faults.len(), faults.join("\n"));
}

#[test]
fn every_pair_whose_rules_allow_combos_has_a_book_or_is_listed_with_the_reason() {
    let listed: HashMap<&str, &str> = UNBOOKED.lines().filter(|l| !l.starts_with('#')).filter_map(|l| l.split_once(" | ")).collect();
    let mut booked = 0;
    for (fighter, style) in opponents() {
        let key = opponent_key(fighter, style);
        let found = book(fighter, style.0, style.1);
        booked += usize::from(found.is_some());
        assert!(found.is_some() != listed.contains_key(key.as_str()), "{key}: a book and listed, or neither");
        if style.0 == Revision::Arcade1987 {
            assert!(listed[key.as_str()].contains("1987 rules allow no combo"), "{key}");
        }
        assert!(found.is_none_or(|b| !b.routes.is_empty()), "{key}: an empty book");
    }
    assert_eq!(booked, books().len(), "every book is of a pair someone can pick, once");
    assert_eq!(listed.len() + booked, opponents().len(), "nothing listed that is not a pair");
    // A pair is without a book by rule or because the search found nothing,
    // never because the search could not run.
    for (key, why) in &listed {
        assert!(why.contains("1987 rules allow no combo") || why.contains("found no route of two hits or more"), "{key}: {why}");
    }
}

#[test]
fn ryus_sf6_route_book_loads_with_several_routes_short_ones_included() {
    let ryu = book(TournamentFighter::Ryu, Revision::Sf6, Loadout::Standard).expect("Ryu's SF6 route book is checked in");
    // From a ground hit and as a punish. The search has found no route from
    // a jump-in for this pair: none is in the book, and that is on the ticket.
    for reach in [Reach::Ground, Reach::Punish] {
        let routes: Vec<_> = ryu.routes.iter().filter(|r| r.reach == reach).collect();
        assert!(routes.len() >= 3, "{reach:?}: {} routes", routes.len());
        assert!(routes.iter().all(|r| r.hits >= 2 && r.damage > 0 && !r.stances.is_empty() && r.true_combos), "{reach:?}");
    }
    assert!(ryu.routes.iter().any(|r| r.hits == 2), "a two-hit route");
    assert!(ryu.routes.iter().any(|r| r.hits >= 4), "a longer route");
    assert!(ryu.routes.iter().any(|r| r.super_spent == 0) && ryu.routes.iter().any(|r| r.super_spent > 0), "with and without meter");
    assert!(ryu.routes.iter().all(|r| !r.corner));
    // Proven against the reference defender, so never recorded failing on him.
    let reference = opponent_key(TournamentFighter::Ryu, (Revision::Sf6, Loadout::Standard));
    assert!(ryu.routes.iter().all(|r| !r.fails_against(&reference, Stance::Standing) && !r.fails_against(&reference, Stance::Crouching)));
    assert!(book(TournamentFighter::Ryu, Revision::Arcade1987, Loadout::Standard).is_none(), "the 1987 rules allow no route");
}

#[test]
fn the_search_finds_short_routes_and_keeps_none_that_begins_with_a_press_that_does_nothing() {
    with_rom(|rom| {
        let style = (Revision::Sf6, Loadout::Standard);
        let base = position(&rom, None, TournamentFighter::Ryu, style, Reach::Ground, Stance::Standing).unwrap();
        let nothing = Action { name: "nothing".into(), pads: vec![(0, 0); 4], hold: 0 };
        let all = ground_actions();
        let named = |name: &str| all.iter().find(|a| a.name == name).cloned().unwrap();
        assert!(!does_something(&base, &nothing, Stance::Standing));
        assert!(does_something(&base, &named("HP+HK"), Stance::Standing), "Drive Impact is something");
        assert!(does_something(&base, &named("cr.MK"), Stance::Standing));
        // A small search over a few presses, the idle one among them.
        let actions = vec![nothing, named("LP"), named("cr.LP"), named("cr.MK"), named("236+HP"), named("623+HP")];
        let pair = Pair { program_rom: &rom, data: None, fighter: TournamentFighter::Ryu, style };
        let found = search(&pair, &actions, Budget { nodes: 3, keep: 4 }).unwrap().book;
        assert!(found.routes.len() >= 2, "{} routes", found.routes.len());
        assert!(found.routes.iter().any(|r| r.hits == 2), "a two-hit route: {:?}", found.routes.iter().map(|r| r.name()).collect::<Vec<_>>());
        for route in &found.routes {
            assert!(route.steps.iter().all(|s| s.name != "nothing"), "{}", route.name());
            assert!(!route.steps[0].pads.iter().all(|&(_, buttons)| buttons == 0), "{} opens with a button", route.name());
        }
    });
}
