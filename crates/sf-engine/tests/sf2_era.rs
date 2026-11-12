//! SF2 revisions, the era selector, and the Super gauge (`#103`, `O12`).

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{
    costumes, families, revisions, Costume, EraFamily, GaugeMode, Loadout, Revision, StylePick,
    PALETTES,
};
use sf_engine::roster::{
    CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, TournamentFighter,
};
#[test]
fn selector_lists_only_canonical_eras_and_eight_palettes() {
    assert!(!families(TournamentFighter::Lee).contains(&EraFamily::Sf2));
    assert_eq!(families(TournamentFighter::Ryu).len(), 8);
    assert_eq!(
        revisions(TournamentFighter::Sagat, EraFamily::Sf2),
        vec![
            Revision::Ww,
            Revision::Ce,
            Revision::Hf,
            Revision::Super,
            Revision::St
        ]
    );
    assert_eq!(
        revisions(TournamentFighter::Gen, EraFamily::Sf2),
        Vec::<Revision>::new()
    );
    assert_eq!(
        costumes(TournamentFighter::Ken, EraFamily::Sf2),
        vec![Costume::Classic1987, Costume::Sf2]
    );
    assert_eq!(
        costumes(TournamentFighter::Lee, EraFamily::Sf1Plus),
        vec![Costume::Classic1987]
    );
    assert_eq!(PALETTES, 8);

    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.mode = PlayMode::Arcade;
    let mut screen = CharacterSelectScreen::open(settings, true);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Country");
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Era family");
    let listed = screen
        .rows()
        .into_iter()
        .find(|row| row.starts_with("ERA FAMILY"))
        .unwrap();
    assert!(listed.contains("SF2"));
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Revision");
    for _ in 0..4 {
        screen.apply(MenuInput::Right);
    }
    screen.apply(MenuInput::Up);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Costume");
    screen.apply(MenuInput::Right);
    for _ in 0..7 {
        screen.apply(MenuInput::Up);
    }
    screen.apply(MenuInput::Confirm);
    assert!(screen.is_ready());
    assert_eq!(
        screen.p1_style(),
        StylePick::sf2(Revision::St, Loadout::Old, Costume::Sf2, 7)
    );

    let mut lee = CharacterSelectScreen::open(settings, true);
    for _ in 0..16 {
        if lee
            .rows()
            .iter()
            .any(|row| row.starts_with("> ") && row.contains("Lee"))
        {
            lee.apply(MenuInput::Confirm);
            break;
        }
        lee.apply(MenuInput::Right);
    }
    lee.apply(MenuInput::Confirm);
    let lee_families = lee
        .rows()
        .into_iter()
        .find(|row| row.starts_with("ERA FAMILY"))
        .unwrap();
    assert!(!lee_families.contains("SF2"), "{lee_families}");
}

#[test]
fn pure_footsies_defaults_to_no_gauges_and_round_trips() {
    let footsies = MatchConfig::from_preset(RulesetPreset::PureFootsies1987Plus);
    assert_eq!(footsies.gauge_mode, GaugeMode::NoGauges);
    let reloaded = MatchConfig::from_toml(&footsies.to_toml()).unwrap();
    assert_eq!(reloaded.gauge_mode, GaugeMode::NoGauges);
}
