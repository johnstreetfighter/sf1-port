//! Ticket #193 (`SC1`): the measuring run, the Fair CPU against itself.
//!
//! A smoke test that the run plays a headless match with the Fair CPU on
//! both pads and reports how active each side was. It is not a gate on any
//! number: the numbers are for judging later tickets. Local ROMs only.

use std::path::Path;

use sf_engine::era::{Loadout, Revision};
use sf_engine::measure::{boss_ever_lost, dramatic_round, measure_dramatic, measure_match, DramaticReport, DramaticSpec, MatchReport, MeasureSpec};
use sf_engine::roster::TournamentFighter;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn ryu_mirror(seed: u64) -> Option<MatchReport> {
    ryu_mirror_with(seed, [false; 2])
}

/// A Ryu mirror in the SF6 era style at Very Difficult; `stronger` says
/// which sides play the stronger Fair CPU's game.
fn ryu_mirror_with(seed: u64, stronger: [bool; 2]) -> Option<MatchReport> {
    ryu_mirror_at(seed, stronger, [4, 4])
}

/// The same, each side at its own level.
fn ryu_mirror_at(seed: u64, stronger: [bool; 2], difficulty: [u8; 2]) -> Option<MatchReport> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu; 2],
        styles: [(Revision::Sf6, Loadout::Standard); 2],
        difficulty,
        seed,
        stronger,
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

#[test]
fn the_new_fair_cpu_can_be_put_against_the_old_on_either_side() {
    let Some(old) = ryu_mirror(3) else { return };
    let new_p1 = ryu_mirror_with(3, [true, false]).unwrap();
    let new_p2 = ryu_mirror_with(3, [false, true]).unwrap();
    assert!(new_p1.ended && new_p2.ended, "{new_p1:?} {new_p2:?}");
    assert_ne!(new_p1.sides, old.sides, "the new play on Player 1's pad is another fight");
    assert_ne!(new_p2.sides, old.sides, "the new play on Player 2's pad is another fight");
    assert!(new_p1.row().contains("\tnew\t") && new_p1.row().contains("\told\t"), "{}", new_p1.row());
}

#[test]
fn each_side_can_play_at_a_level_of_its_own() {
    // SC11 (`#203`): one level against the next. The match's setting is the
    // higher level, Very Difficult in all three of these, so a fight that
    // differs from both sides at Very Difficult differs by the side's own
    // level: Player 1's at Easy, then Player 2's at Easy (though the
    // match's difficulty stays Very Difficult). The row names both levels.
    let Some(mixed) = ryu_mirror_at(3, [true; 2], [1, 4]) else { return };
    let same = ryu_mirror_at(3, [true; 2], [4, 4]).unwrap();
    assert_ne!(mixed.sides, same.sides, "Player 1's level reaches its CPU");
    let swapped = ryu_mirror_at(3, [true; 2], [4, 1]).unwrap();
    assert_ne!(swapped.sides, same.sides, "Player 2's own level reaches its CPU, not the match's");
    let level = MatchReport::header().split('\t').position(|h| h == "level").unwrap();
    assert_eq!(mixed.row().split('\t').nth(level), Some("1/4"));
    assert_eq!(same.row().split('\t').nth(level), Some("4"));
    // An old side cannot be measured at a level of its own.
    let assets = load_assets(ROMS, MANIFEST).unwrap();
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu; 2],
        styles: [(Revision::Sf6, Loadout::Standard); 2],
        difficulty: [1, 4],
        seed: 3,
        stronger: [true, false],
    };
    assert!(measure_match(&assets.maincpu, None, &spec).is_err_and(|e| e.contains("both sides")));
}

/// A solo Dramatic match of Ryu in the SF6 era style as both allies and
/// the boss, each played by its own Fair CPU at Very Difficult.
fn dramatic(seed: u64, stronger: [bool; 3], difficulty: [u8; 3]) -> Option<Result<DramaticReport, String>> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let spec = DramaticSpec {
        fighters: [TournamentFighter::Ryu; 3],
        styles: [Revision::Sf6; 3],
        difficulty,
        seed,
        stronger,
    };
    let run = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || measure_dramatic(&assets.maincpu, None, &spec))
        .unwrap();
    Some(run.join().unwrap())
}

#[test]
fn a_dramatic_match_with_three_fair_cpus_is_played_and_reports_the_boss() {
    // SC23 (`#215`): three Fair CPUs at once, the boss's results first.
    let Some(report) = dramatic(2, [true; 3], [4; 3]) else { return };
    let report = report.expect("the match runs");
    assert!(report.ended, "the match reached its end: {report:?}");
    assert!(report.boss_rounds_won + report.boss_rounds_lost + report.rounds_drawn >= 2, "rounds were judged: {report:?}");
    assert!(report.boss_rounds_won.max(report.boss_rounds_lost) >= 2, "someone took the match: {report:?}");
    assert!(report.live_frames > 600, "rounds were fought: {report:?}");
    for (i, side) in report.sides.iter().enumerate() {
        assert!(side.attacks > 0, "fighter {i} attacked: {report:?}");
        assert!(side.idle_frames < report.live_frames, "fighter {i}: {report:?}");
    }
    assert!(report.boss_damage > 0 && report.team_damage > 0, "both sides landed hits: {report:?}");
    let header = DramaticReport::header().split('\t').count();
    assert_eq!(report.row().split('\t').count(), header, "one row per match, a column per heading");
    // The flag: a run whose boss never lost a round shows it.
    assert_eq!(boss_ever_lost(&[report]), report.boss_rounds_lost > 0);
    let unbeaten = DramaticReport { boss_rounds_lost: 0, boss_rounds_won: 2, ..report };
    assert!(!boss_ever_lost(&[unbeaten, unbeaten]));
    assert!(boss_ever_lost(&[unbeaten, DramaticReport { boss_rounds_lost: 1, ..unbeaten }]));
}

#[test]
fn a_dramatic_match_is_set_by_its_seed_and_refuses_mixed_levels_with_an_old_cpu() {
    let Some(first) = dramatic(4, [true; 3], [4; 3]) else { return };
    let first = first.expect("the match runs");
    assert_eq!(dramatic(4, [true; 3], [4; 3]).unwrap().unwrap(), first);
    let old = dramatic(4, [false; 3], [4; 3]).unwrap().unwrap();
    assert_ne!(old.sides, first.sides, "the old play is another fight");
    let error = dramatic(4, [true, false, true], [4, 4, 2]).unwrap().unwrap_err();
    assert!(error.contains("every fighter"), "{error}");
}

#[test]
fn a_dramatic_round_is_judged_as_the_game_judges_it() {
    // SC23 (`#215`): (Ally 1, the boss, Ally 2). A drawn round is not a
    // round the boss lost, so a boss nobody beats is still flagged.
    use sf_engine::dramatic::{ALLY_SUB_HP, BOSS_SUB_HP};
    assert_eq!(dramatic_round([100, 0, 0]), Some(false), "the boss knocked out");
    assert_eq!(dramatic_round([0, 1, 0]), Some(true), "both allies knocked out");
    assert_eq!(dramatic_round([0, 0, 0]), None, "a double knockout");
    assert_eq!(dramatic_round([ALLY_SUB_HP, BOSS_SUB_HP, ALLY_SUB_HP]), None, "nobody hit at time over");
    // Time over: the healthier ally's share against the boss's share.
    assert_eq!(dramatic_round([480, 768, 0]), None, "half and half");
    assert_eq!(dramatic_round([481, 768, 10]), Some(false), "the ally's share higher");
    assert_eq!(dramatic_round([10, 769, 480]), Some(true), "the boss's share higher");
    let drawn = sf_engine::measure::DramaticReport {
        boss_rounds_won: 2,
        boss_rounds_lost: 0,
        rounds_drawn: 1,
        ..a_report()
    };
    assert!(!boss_ever_lost(&[drawn]), "a draw is not a loss");
}

fn a_report() -> DramaticReport {
    DramaticReport {
        spec: DramaticSpec { fighters: [TournamentFighter::Ryu; 3], styles: [Revision::Sf6; 3], difficulty: [4; 3], seed: 1, stronger: [true; 3] },
        ended: true,
        boss_rounds_won: 0,
        boss_rounds_lost: 0,
        rounds_drawn: 0,
        frames: 0,
        live_frames: 0,
        boss_damage: 0,
        team_damage: 0,
        sides: Default::default(),
    }
}
