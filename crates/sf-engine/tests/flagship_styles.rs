//! What-if SF3, USF4, SF5 CE, and SF6 for the rest of the cast (`#116`, `M3`).

use sf_engine::era::{listed_families, listed_revisions, EraFamily, Loadout, Revision};
use sf_engine::flagship::{FlagshipEvent, FlagshipSession};
use sf_engine::roster::{CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, TournamentFighter};
use sf_engine::config::RulesetPreset;

fn open_lee(what_if: bool) -> CharacterSelectScreen {
    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.mode = PlayMode::Arcade;
    settings.what_if_styles = what_if;
    let mut screen = CharacterSelectScreen::open(settings, true);
    for _ in 0..16 {
        if screen.rows().iter().any(|row| row.starts_with("> ") && row.contains("Lee")) {
            screen.apply(MenuInput::Confirm);
            break;
        }
        screen.apply(MenuInput::Right);
    }
    screen.apply(MenuInput::Confirm);
    screen
}

#[test]
fn what_if_styles_stay_hidden_until_the_filter() {
    let hidden = open_lee(true);
    let line = hidden.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(!line.contains("SF3"), "{line}");
    assert!(!line.contains("SF6"), "{line}");
    let mut shown = hidden;
    shown.apply(MenuInput::Filter);
    let line = shown.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(line.contains("SF3 [WHAT-IF]"), "{line}");
    assert!(line.contains("SF4 [WHAT-IF]"), "{line}");
    assert!(line.contains("SF5 [WHAT-IF]"), "{line}");
    assert!(line.contains("SF6 [WHAT-IF]"), "{line}");

    let mut off = open_lee(false);
    off.apply(MenuInput::Filter);
    let line = off.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(!line.contains("SF3"), "{line}");
}

#[test]
fn forgotten_five_and_veterans_get_the_flagship_revisions() {
    for fighter in [TournamentFighter::Geki, TournamentFighter::Joe, TournamentFighter::Mike, TournamentFighter::Lee, TournamentFighter::Retsu] {
        assert_eq!(listed_revisions(fighter, EraFamily::Sf3, true), vec![Revision::Sf3ThreeS]);
        assert_eq!(listed_revisions(fighter, EraFamily::Sf4, true), vec![Revision::Usf4]);
        assert_eq!(listed_revisions(fighter, EraFamily::Sf5, true), vec![Revision::Sf5Ce]);
    }
    assert!(listed_families(TournamentFighter::Retsu, true).contains(&EraFamily::Sf6));
    assert_eq!(listed_revisions(TournamentFighter::Retsu, EraFamily::Sf6, false), vec![Revision::Sf6]);
    for fighter in [TournamentFighter::Sagat, TournamentFighter::Gen, TournamentFighter::Adon, TournamentFighter::Birdie, TournamentFighter::Eagle] {
        assert!(listed_families(fighter, true).contains(&EraFamily::Sf3));
        assert!(listed_families(fighter, true).contains(&EraFamily::Sf6));
        assert_eq!(listed_revisions(fighter, EraFamily::Sf3, true), vec![Revision::Sf3ThreeS]);
    }
    assert!(listed_revisions(TournamentFighter::Birdie, EraFamily::Sf4, false).is_empty());
    assert_eq!(listed_revisions(TournamentFighter::Birdie, EraFamily::Sf4, true), vec![Revision::Usf4]);
    assert!(listed_revisions(TournamentFighter::Geki, EraFamily::Sf6, false).is_empty());
}

#[test]
fn gauges_finishers_and_validated_assets() {
    let mut joe = FlagshipSession::new(TournamentFighter::Joe, EraFamily::Sf6, Loadout::Standard, true);
    assert!(joe.available());
    assert_eq!(joe.drive_bars(), 6);
    assert!(joe.overdrive("Slash Kick"));
    assert_eq!(joe.drive_bars(), 4);
    assert!(joe.drive_parry(1));
    assert!(matches!(joe.events.last(), Some(FlagshipEvent::DriveParry { perfect: true })));
    assert!(joe.drive_impact());
    joe.land_hit(false);
    joe.land_hit(false);
    joe.land_hit(false);
    assert!(joe.level_super(3));
    assert!(matches!(joe.events.last(), Some(FlagshipEvent::Super { name: "Meteor Kick", level: 3 })));

    let mut lee = FlagshipSession::new(TournamentFighter::Lee, EraFamily::Sf6, Loadout::Standard, false);
    assert!(!lee.available());
    assert!(!lee.level_super(3));
    lee.what_if = true;
    lee.land_hit(false);
    lee.land_hit(false);
    lee.land_hit(false);
    assert!(lee.level_super(3));
    assert!(matches!(lee.events.last(), Some(FlagshipEvent::Super { name: "One Man Army", .. })));

    let mut mike = FlagshipSession::new(TournamentFighter::Mike, EraFamily::Sf5, Loadout::Skill2Trigger2, true);
    assert!(mike.punch(0) && mike.punch(5));
    assert!(!mike.punch(6));
    mike.land_hit(true);
    assert!(mike.events.iter().any(|event| matches!(event, FlagshipEvent::CrushCounter)));
    mike.block();
    assert!(mike.v_reversal());
    mike.land_hit(false);
    mike.land_hit(false);
    mike.land_hit(false);
    assert!(mike.v_trigger());
    assert!(matches!(mike.events.last(), Some(FlagshipEvent::VTrigger("V-Trigger II"))));
    mike.land_hit(false);
    mike.land_hit(false);
    mike.land_hit(false);
    assert!(mike.critical_art());

    let mut retsu = FlagshipSession::new(TournamentFighter::Retsu, EraFamily::Sf3, Loadout::Art3, true);
    assert!(retsu.parry());
    assert!(retsu.overhead());
    retsu.land_hit(false);
    assert!(retsu.ex_special("Tobi Geri"));
    retsu.land_hit(false);
    retsu.land_hit(false);
    retsu.land_hit(false);
    assert!(retsu.super_art(3));

    let mut birdie = FlagshipSession::new(TournamentFighter::Birdie, EraFamily::Sf4, Loadout::UltraDouble, true);
    birdie.land_hit(false);
    birdie.land_hit(false);
    birdie.land_hit(false);
    birdie.land_hit(false);
    assert!(birdie.focus(21, true));
    assert!(birdie.fadc());
    birdie.knock_down();
    assert!(birdie.delayed_wakeup(2));
    birdie.land_hit(false);
    birdie.land_hit(false);
    assert!(birdie.ultra());
    assert!(matches!(birdie.events.last(), Some(FlagshipEvent::Ultra("Ultra Double"))));

    let hidden = FlagshipSession::new(TournamentFighter::Eagle, EraFamily::Sf6, Loadout::Standard, false);
    assert!(!hidden.available());
    assert!(!hidden.tiles.is_empty());
}
