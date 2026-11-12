//! SF3 Style Card options. Native match coverage lives in `sf3_native`.

use sf_engine::era::{loadouts, revisions, EraFamily, Loadout, Revision};
use sf_engine::roster::TournamentFighter;

#[test]
fn ryu_and_ken_get_three_revisions_and_three_arts() {
    let expected = vec![Revision::Sf3Ng, Revision::Sf3TwoI, Revision::Sf3ThreeS];
    assert_eq!(revisions(TournamentFighter::Ryu, EraFamily::Sf3), expected);
    assert_eq!(revisions(TournamentFighter::Ken, EraFamily::Sf3), expected);
    assert!(revisions(TournamentFighter::Sagat, EraFamily::Sf3).is_empty());
    for revision in expected {
        assert_eq!(
            loadouts(revision),
            vec![Loadout::Art1, Loadout::Art2, Loadout::Art3]
        );
    }
}
