//! SF4 revision and Ultra availability.

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{loadouts, revisions, EraFamily, Loadout, Revision};
use sf_engine::roster::TournamentFighter;

#[test]
fn roster_revisions_and_ultra_loadouts() {
    assert_eq!(
        revisions(TournamentFighter::Ryu, EraFamily::Sf4),
        vec![
            Revision::Sf4,
            Revision::Ssf4,
            Revision::Ae2012,
            Revision::Usf4
        ]
    );
    assert_eq!(
        revisions(TournamentFighter::Sagat, EraFamily::Sf4),
        revisions(TournamentFighter::Ryu, EraFamily::Sf4)
    );
    assert_eq!(
        revisions(TournamentFighter::Adon, EraFamily::Sf4),
        vec![Revision::Ssf4, Revision::Ae2012, Revision::Usf4]
    );
    assert_eq!(
        revisions(TournamentFighter::Gen, EraFamily::Sf4),
        revisions(TournamentFighter::Ryu, EraFamily::Sf4)
    );
    assert!(revisions(TournamentFighter::Birdie, EraFamily::Sf4).is_empty());
    assert_eq!(loadouts(Revision::Sf4), vec![Loadout::Ultra1]);
    assert_eq!(
        loadouts(Revision::Ssf4),
        vec![Loadout::Ultra1, Loadout::Ultra2]
    );
    assert_eq!(
        loadouts(Revision::Usf4),
        vec![Loadout::Ultra1, Loadout::Ultra2, Loadout::UltraDouble]
    );
    assert!(!MatchConfig::from_preset(RulesetPreset::Faithful1987).disable_comeback_scaling);
    let round = MatchConfig::from_toml(&MatchConfig::definitive().to_toml()).unwrap();
    assert!(!round.disable_comeback_scaling);
}
