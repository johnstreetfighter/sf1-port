//! Bonus arcade ladders, secret rivals, and endings (`#120`, `M7`).

use sf_engine::arcade::Country;
use sf_engine::bonus::Challenger;
use sf_engine::bonus_arcade::{AfterLoss, BonusRun, Stop};
use sf_engine::config::RulesetPreset;
use sf_engine::roster::{RosterSettings, TournamentFighter};

fn settings(bonus: bool, secret: bool) -> RosterSettings {
    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.bonus_challengers = bonus;
    settings.secret_intrusions = secret;
    settings
}

#[test]
fn every_challenger_clears_ten_opponents_and_gets_an_ending() {
    let ten = [
        TournamentFighter::Retsu,
        TournamentFighter::Geki,
        TournamentFighter::Joe,
        TournamentFighter::Mike,
        TournamentFighter::Lee,
        TournamentFighter::Gen,
        TournamentFighter::Birdie,
        TournamentFighter::Eagle,
        TournamentFighter::Adon,
        TournamentFighter::Sagat,
    ];
    for player in Challenger::ALL {
        let mut run = BonusRun::new(player, Country::Japan, settings(true, false));
        let faced = run.opponents();
        assert_eq!(faced, ten);
        assert!(run.ladder_has_bonus());
        while !matches!(run.current(), Stop::Ending) {
            run.win();
        }
        let line = run.dialogue.clone().unwrap();
        assert!(line.contains(player.name()) || line.contains("Sagat") || line.contains("Shadaloo"));
        assert_eq!(run.ending.len(), 4);
    }
    let gou = BonusRun::new(Challenger::GouHibiki, Country::Usa, settings(true, false));
    assert_eq!(gou.opponents().len(), 10);
    assert_eq!(gou.opponents()[0], TournamentFighter::Joe);
    assert_eq!(*gou.opponents().last().unwrap(), TournamentFighter::Sagat);
}

#[test]
fn a_perfect_inserts_a_rival_without_dropping_the_ladder() {
    let mut run = BonusRun::new(Challenger::Bison, Country::Japan, settings(true, true));
    run.note_perfect();
    let fights = run.opponents();
    assert_eq!(fights.len(), 10);
    assert!(run.intrusion().is_some());
    while !matches!(run.current(), Stop::Intrusion(_)) {
        run.win();
    }
    assert_eq!(run.credits, 1);
    assert_eq!(run.lose(), AfterLoss::Rematch);
    assert_eq!(run.credits, 1);
    assert!(run.rematch());
    assert!(run.advance_past_intrusion());
    assert!(!matches!(run.current(), Stop::Intrusion(_)));

    let mut quiet = BonusRun::new(Challenger::Nash, Country::England, settings(true, false));
    quiet.note_perfect();
    quiet.note_super();
    assert!(quiet.intrusion().is_none());
    let mut off = BonusRun::new(Challenger::ChunLi, Country::China, settings(false, true));
    off.note_super();
    assert!(off.intrusion().is_none());
}
