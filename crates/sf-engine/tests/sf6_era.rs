//! SF6 Drive gauge, Super Arts, and safety toggles (`#108`, `O17`).

use sf_engine::config::MatchConfig;
use sf_engine::era::{costumes, revisions, Costume, EraFamily, Revision};
use sf_engine::roster::TournamentFighter;

#[test]
fn canonical_roster_and_costumes() {
    assert_eq!(revisions(TournamentFighter::Retsu, EraFamily::Sf6), vec![Revision::Sf6]);
    assert!(revisions(TournamentFighter::Lee, EraFamily::Sf6).is_empty());
    assert_eq!(
        costumes(TournamentFighter::Ryu, EraFamily::Sf6),
        vec![Costume::Classic1987, Costume::Sf6]
    );
    assert_eq!(
        costumes(TournamentFighter::Ken, EraFamily::Sf6),
        vec![Costume::Classic1987, Costume::Sf6]
    );
}

#[test]
fn safety_toggles_round_trip() {
    let mut config = MatchConfig::definitive();
    config.disable_raw_drive_rush = true;
    let reloaded = MatchConfig::from_toml(&config.to_toml()).unwrap();
    assert!(reloaded.disable_raw_drive_rush);
    assert!(reloaded.sf1plus.disable_freeze_input_lockout);
}
