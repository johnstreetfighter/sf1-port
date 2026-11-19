//! Ticket #195 (`SC3`, spec #192): route books, the widened search and the
//! route proofs.
//!
//! Every route in every checked-in route book is replayed in the engine
//! against the reference defender and its hits checked, so a moveset change
//! that breaks a route fails here until the route is searched again
//! (`cargo run -p sf-host --release --example route_search`). Local ROMs
//! only; each test skips without them.

use std::collections::HashMap;
use std::path::Path;

use sf_engine::era::{Loadout, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::{
    book, book_texts, books, does_something, ground_actions, position, replay, search, Action, Budget, Pair, Reach, RouteBook, Stance,
};
use sf_engine::run::PowerOnMachine;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

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
    with_rom(|rom| {
        for book in books() {
            let mut positions: HashMap<(Reach, bool), PowerOnMachine> = HashMap::new();
            for route in &book.routes {
                let who = format!("{:?} {:?}/{:?}: {} ({:?})", book.fighter, book.revision, book.loadout, route.name(), route.reach);
                for &stance in &route.stances {
                    let base = positions.entry((route.reach, stance == Stance::Crouching)).or_insert_with(|| {
                        position(&rom, None, book.fighter, (book.revision, book.loadout), route.reach, stance).expect("the reference position")
                    });
                    let played = replay(base, &route.script(None), stance);
                    assert_eq!(played.hits, route.hits, "{who} against a {stance:?} defender");
                    if stance != Stance::Standing {
                        continue;
                    }
                    // The rest of what the book says of it, as measured
                    // against the standing defender.
                    assert_eq!(played.damage, route.damage, "{who}: damage");
                    assert_eq!(played.knockdown, route.knockdown, "{who}: knockdown");
                    assert_eq!(
                        [played.super_spent, played.super_gained, played.drive_spent, played.drive_gained],
                        [route.super_spent, route.super_gained, route.drive_spent, route.drive_gained],
                        "{who}: meter"
                    );
                    // Each step still lands at either end of its window.
                    for (i, step) in route.steps.iter().enumerate() {
                        for edge in [step.window.0, step.window.1] {
                            let mut waits: Vec<u8> = route.steps.iter().map(|s| s.wait).collect();
                            waits[i] = edge;
                            let played = replay(base, &route.script(Some(&waits)), stance);
                            assert_eq!(played.hits, route.hits, "{who}: step {} after {edge} frames", step.name);
                        }
                    }
                }
            }
        }
    });
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
    assert!(ryu.routes.iter().all(|r| !r.corner && r.fails_on.is_empty()));
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
        let found = search(&pair, &actions, Budget { nodes: 3, keep: 4 }).unwrap();
        assert!(found.routes.len() >= 2, "{} routes", found.routes.len());
        assert!(found.routes.iter().any(|r| r.hits == 2), "a two-hit route: {:?}", found.routes.iter().map(|r| r.name()).collect::<Vec<_>>());
        for route in &found.routes {
            assert!(route.steps.iter().all(|s| s.name != "nothing"), "{}", route.name());
            assert!(!route.steps[0].pads.iter().all(|&(_, buttons)| buttons == 0), "{} opens with a button", route.name());
        }
    });
}
