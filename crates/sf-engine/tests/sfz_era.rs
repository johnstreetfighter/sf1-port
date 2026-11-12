//! Alpha revisions, the 3-level gauge, and Gen's stances (`#104`, `O13`).

use sf_engine::config::RulesetPreset;
use sf_engine::era::{costumes, families, loadouts, revisions, Costume, EraFamily, Loadout, Revision};
use sf_engine::roster::{CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, TournamentFighter};

#[test]
fn alpha_roster_is_the_canonical_cast_and_sfz3_has_isms() {
    for fighter in [
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        TournamentFighter::Adon,
        TournamentFighter::Sagat,
        TournamentFighter::Birdie,
    ] {
        assert_eq!(
            revisions(fighter, EraFamily::SfZ),
            vec![Revision::Sfz1, Revision::Sfz2, Revision::Sfz2A, Revision::Sfz3]
        );
    }
    assert_eq!(
        revisions(TournamentFighter::Gen, EraFamily::SfZ),
        vec![Revision::Sfz2, Revision::Sfz2A, Revision::Sfz3]
    );
    assert_eq!(revisions(TournamentFighter::Eagle, EraFamily::SfZ), vec![Revision::Sfz3]);
    assert!(revisions(TournamentFighter::Lee, EraFamily::SfZ).is_empty());
    assert!(!families(TournamentFighter::Lee).contains(&EraFamily::SfZ));
    assert_eq!(
        loadouts(Revision::Sfz3),
        vec![Loadout::AIsm, Loadout::VIsm, Loadout::XIsm]
    );
    assert_eq!(
        costumes(TournamentFighter::Birdie, EraFamily::SfZ),
        vec![Costume::Classic1987, Costume::Alpha]
    );

    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.mode = PlayMode::Arcade;
    let mut screen = CharacterSelectScreen::open(settings, true);
    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Confirm);
    for _ in 0..3 {
        screen.apply(MenuInput::Right);
    }
    screen.apply(MenuInput::Up);
    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.p1_style().revision, Revision::Sfz3);
    assert_eq!(screen.p1_style().loadout, Loadout::VIsm);
    assert_eq!(screen.p1_style().costume, Costume::Alpha);
}
