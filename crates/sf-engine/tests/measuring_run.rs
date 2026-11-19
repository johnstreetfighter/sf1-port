//! Ticket #193 (`SC1`): the measuring run, the Fair CPU against itself.
//!
//! A smoke test that the run plays a headless match with the Fair CPU on
//! both pads and reports how active each side was. It is not a gate on any
//! number: the numbers are for judging later tickets. Local ROMs only.

use std::path::Path;

use sf_engine::era::{Loadout, Revision};
use sf_engine::measure::{measure_match, MatchReport, MeasureSpec};
use sf_engine::roster::TournamentFighter;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn ryu_mirror(seed: u64) -> Option<MatchReport> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu; 2],
        styles: [(Revision::Sf6, Loadout::Standard); 2],
        difficulty: 4,
        seed,
    };
    // A whole match is deep in the translated program's calls.
    let run = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || measure_match(&assets.maincpu, None, &spec))
        .unwrap();
    Some(run.join().unwrap().expect("the match runs"))
}

#[test]
fn a_match_between_two_fair_cpus_is_played_and_reported() {
    let Some(report) = ryu_mirror(1) else { return };
    assert!(report.ended, "the match reached its end: {report:?}");
    // A drawn match names no winner; a won one was won by rounds.
    if let Some(winner) = report.winner {
        assert!(report.sides[winner].rounds_won > report.sides[1 - winner].rounds_won, "the winner took more rounds: {report:?}");
    }
    assert!(report.live_frames > 600, "rounds were fought: {report:?}");
    for (i, side) in report.sides.iter().enumerate() {
        // Each pad is played: a side nobody plays starts no attack.
        assert!(side.attacks > 0, "side {} attacked: {report:?}", i + 1);
        assert!(side.idle_frames < report.live_frames, "side {} idle time is part of the fight: {report:?}", i + 1);
        assert!(side.hits >= side.openings && side.followed <= side.openings, "side {}: {report:?}", i + 1);
        assert!(side.openings == 0 || side.damage > 0, "side {} openings did damage: {report:?}", i + 1);
    }
    let header = MatchReport::header().split('\t').count();
    assert_eq!(report.row().split('\t').count(), header, "one row per match, a column per heading");
}

#[test]
fn the_same_seed_gives_the_same_report_and_another_seed_another_fight() {
    let Some(first) = ryu_mirror(7) else { return };
    assert_eq!(ryu_mirror(7).unwrap(), first);
    assert_ne!(ryu_mirror(8).unwrap(), first, "the seed reaches the fight");
}
