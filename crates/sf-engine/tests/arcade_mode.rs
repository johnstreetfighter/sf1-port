//! Arcade route, positive-edge bonus stages, rival lines, and endings (`#102`, `O11`).

use std::collections::HashSet;

use sf_engine::arcade::{
    clear_route, ending_for, player_intro, press_at, release_at, rival_taunt, route_from,
    win_quote, BonusKind, BonusStage, Country, Sample, ROUTE,
};
use sf_engine::config::MatchConfig;
use sf_engine::roster::TournamentFighter;
use sf_engine::sf1plus::kit_for;
use sf_video::asset_pack::{validate_adpcm, validate_fm_patch, validate_tile};

#[test]
fn positive_edge_scores_the_press_and_release_mode_scores_the_let_go() {
    let plank_press = BonusStage::plank(true);
    let timed = plank_press.judge(&press_at(10, 3));
    assert_eq!(timed.hits, 1);
    assert_eq!(timed.points, 3000);

    let early_hold = vec![
        Sample {
            frame: 2,
            down: true,
            power: 3,
        },
        Sample {
            frame: 10,
            down: true,
            power: 3,
        },
    ];
    assert_eq!(
        plank_press.judge(&early_hold).hits,
        0,
        "a hold already down is not a new press"
    );

    let weak = plank_press.judge(&press_at(10, 1));
    assert_eq!(weak.hits, 0, "light power misses the plank");

    let outside = plank_press.judge(&press_at(4, 3));
    assert_eq!(outside.hits, 0, "a press outside the window misses");

    let release = BonusStage::plank(false);
    assert_eq!(release.judge(&release_at(10, 3)).hits, 1);
    assert_eq!(
        release.judge(&press_at(10, 3)).hits,
        0,
        "a press with no release does not score"
    );
    assert_eq!(release.judge(&release_at(4, 3)).hits, 0);

    let barrels = BonusStage::barrel(true);
    let mut swings = press_at(6, 1);
    swings.extend(press_at(13, 2));
    swings.extend(press_at(19, 3));
    let broken = barrels.judge(&swings);
    assert_eq!(broken.kind, BonusKind::Barrel);
    assert_eq!(broken.hits, 3);
    assert_eq!(broken.points, 6000);

    let mut soft_last = press_at(6, 1);
    soft_last.extend(press_at(13, 2));
    soft_last.extend(press_at(19, 1));
    assert_eq!(
        barrels.judge(&soft_last).hits,
        2,
        "the heavy barrel refuses a light tap"
    );
}

#[test]
fn any_fighter_clears_five_countries_and_sees_an_ending() {
    let overhaul = MatchConfig::definitive();
    assert!(overhaul.toggles.positive_edge_attacks);
    let faithful = MatchConfig::faithful();
    assert!(!faithful.toggles.positive_edge_attacks);

    let mut intros = HashSet::new();
    let mut taunts = HashSet::new();
    let mut wins = HashSet::new();
    let mut first_panels = HashSet::new();

    for fighter in TournamentFighter::ALL {
        intros.insert(player_intro(fighter));
        taunts.insert(rival_taunt(fighter));
        wins.insert(win_quote(fighter));

        let kit = kit_for(fighter);
        assert!(validate_adpcm(&kit.voice).is_ok());
        assert!(validate_fm_patch(&kit.patch).is_ok());
        for tile in &kit.tiles {
            validate_tile(tile).unwrap();
        }

        for config in [&overhaul, &faithful] {
            let clear = clear_route(fighter, config, Country::Japan);
            assert_eq!(clear.stops.len(), ROUTE.len(), "{fighter:?}");
            assert_eq!(
                clear
                    .stops
                    .iter()
                    .map(|stop| stop.country)
                    .collect::<Vec<_>>(),
                ROUTE.to_vec()
            );
            let names: Vec<&str> = kit
                .moves
                .iter()
                .filter(|mv| mv.kind == sf_engine::sf1plus::MoveKind::Special)
                .map(|mv| mv.name)
                .collect();
            assert_eq!(
                clear.stops[0].specials, names,
                "{fighter:?} specials on the Japan stop"
            );
            for stop in &clear.stops {
                assert_ne!(stop.rival, fighter);
                assert_eq!(stop.player_line, player_intro(fighter));
                assert_eq!(stop.rival_line, rival_taunt(stop.rival));
                assert_eq!(stop.win_line, win_quote(fighter));
                assert!(
                    stop.bonus.hits > 0,
                    "{fighter:?} {:?} bonus missed",
                    stop.country
                );
                assert!(stop.bonus.points > 0);
            }
            let ending = clear.ending.expect("Thailand ending");
            assert_eq!(ending.panels.len(), 4);
            for panel in &ending.panels {
                validate_tile(panel).unwrap();
            }
            assert_ne!(ending.panels[0].pixels, ending.panels[1].pixels);
            assert!(ending.credits.iter().any(|line| line == fighter.name()));
            assert!(ending.credits.iter().any(|line| line == "Thailand"));
            assert!(ending.credits.iter().any(|line| line == "The end"));
            first_panels.insert(ending.panels[0].pixels);
        }
    }

    assert_eq!(intros.len(), 12);
    assert_eq!(taunts.len(), 12);
    assert_eq!(wins.len(), 12);
    assert_eq!(first_panels.len(), 12, "ending pictures collided");

    let from_england = clear_route(TournamentFighter::Ken, &overhaul, Country::England);
    assert_eq!(
        from_england
            .stops
            .iter()
            .map(|stop| stop.country)
            .collect::<Vec<_>>(),
        route_from(Country::England)
    );
    assert!(from_england.ending.is_some());

    let as_sagat = clear_route(TournamentFighter::Sagat, &overhaul, Country::Thailand);
    assert_eq!(as_sagat.stops.len(), 1);
    assert_eq!(as_sagat.stops[0].rival, TournamentFighter::Adon);
    assert_eq!(
        as_sagat.stops[0].specials,
        vec!["High Tiger Shot", "Low Tiger Shot", "Tiger Knee"]
    );

    let as_lee = clear_route(TournamentFighter::Lee, &overhaul, Country::China);
    assert_eq!(as_lee.stops[0].rival, TournamentFighter::Gen);
    assert_eq!(
        as_lee.stops[0].specials,
        vec!["Zesshouohken", "Double Kick"]
    );

    let ending = ending_for(TournamentFighter::Eagle);
    assert!(ending
        .credits
        .iter()
        .any(|line| line.contains("Sticks back in the case")));
}
