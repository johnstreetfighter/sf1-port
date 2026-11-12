//! What-if SF2 and SFZ3 styles, filters, and expansion toggles (`#115`, `M2`).

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{listed_families, listed_revisions, EraFamily, Revision};
use sf_engine::roster::{CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, TournamentFighter};

#[test]
fn expansion_toggles_follow_the_preset() {
    let faithful = MatchConfig::from_preset(RulesetPreset::Faithful1987);
    assert!(!faithful.roster.what_if_styles);
    assert!(!faithful.roster.bonus_challengers);
    assert!(!faithful.roster.secret_intrusions);
    assert!(!MatchConfig::from_preset(RulesetPreset::ClassicCompetitive).roster.what_if_styles);
    assert!(!MatchConfig::from_preset(RulesetPreset::PureFootsies1987Plus).roster.what_if_styles);
    let over = MatchConfig::definitive();
    assert!(over.roster.what_if_styles && over.roster.bonus_challengers && over.roster.secret_intrusions);
    let round = MatchConfig::from_toml(&over.to_toml()).unwrap();
    assert!(round.roster.what_if_styles);
}

#[test]
fn filter_hides_what_if_until_the_button() {
    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.mode = PlayMode::Arcade;
    let mut lee = CharacterSelectScreen::open(settings, true);
    for _ in 0..16 {
        if lee.rows().iter().any(|row| row.starts_with("> ") && row.contains("Lee")) {
            lee.apply(MenuInput::Confirm);
            break;
        }
        lee.apply(MenuInput::Right);
    }
    lee.apply(MenuInput::Confirm);
    let hidden = lee.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(!hidden.contains("SF2"), "{hidden}");
    assert!(lee.rows().iter().any(|row| row.contains("Canonical Styles Only")));
    assert!(lee.rows().iter().any(|row| row.contains("1987 Tournament 12")));

    lee.apply(MenuInput::Filter);
    let shown = lee.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(shown.contains("SF2 [WHAT-IF]"), "{shown}");
    assert!(shown.contains("SFZ [WHAT-IF]"), "{shown}");
    assert!(lee.rows().iter().any(|row| row.contains("All Styles (Incl. What-If)")));
    assert!(lee.rows().iter().any(|row| row.contains("Expanded 25 Roster")));

    let mut faithful = CharacterSelectScreen::open(RosterSettings::faithful(), true);
    faithful.apply(MenuInput::Filter);
    assert!(faithful.rows().iter().any(|row| row.contains("Canonical Styles Only")));
}

#[test]
fn flagship_revisions_follow_the_what_if_filter() {
    assert_eq!(
        listed_revisions(TournamentFighter::Lee, EraFamily::Sf2, true),
        vec![Revision::Hf, Revision::St]
    );
    assert_eq!(listed_revisions(TournamentFighter::Retsu, EraFamily::SfZ, true), vec![Revision::Sfz3]);
    assert!(listed_families(TournamentFighter::Gen, true).contains(&EraFamily::Sf2));
    assert!(!listed_families(TournamentFighter::Gen, true).iter().any(|family| *family == EraFamily::SfZ) || listed_revisions(TournamentFighter::Gen, EraFamily::SfZ, true).len() > 1);
    assert!(listed_revisions(TournamentFighter::Adon, EraFamily::Sf2, true) == vec![Revision::Hf, Revision::St]);
    assert!(listed_revisions(TournamentFighter::Lee, EraFamily::Sf2, false).is_empty());
}
