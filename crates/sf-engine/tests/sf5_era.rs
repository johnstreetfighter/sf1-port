//! Street Fighter V selector eligibility and loadouts.

use sf_engine::era::{costumes, loadouts, revisions, Costume, EraFamily, Loadout, Revision};
use sf_engine::roster::TournamentFighter::{Birdie, Ken, Lee, Ryu, Sagat};

#[test]
fn roster_and_loadouts() {
    let revisions_for = |fighter| revisions(fighter, EraFamily::Sf5);
    for fighter in [Ryu, Ken, Birdie] {
        assert_eq!(
            revisions_for(fighter),
            vec![Revision::Sf5, Revision::Sf5Ae, Revision::Sf5Ce]
        );
    }
    assert_eq!(revisions_for(Sagat), vec![Revision::Sf5Ae, Revision::Sf5Ce]);
    assert!(revisions_for(Lee).is_empty());
    assert_eq!(loadouts(Revision::Sf5), vec![Loadout::Skill1Trigger1]);
    assert_eq!(
        loadouts(Revision::Sf5Ae),
        vec![Loadout::Skill1Trigger1, Loadout::Skill1Trigger2]
    );
    assert_eq!(
        loadouts(Revision::Sf5Ce),
        vec![
            Loadout::Skill1Trigger1,
            Loadout::Skill1Trigger2,
            Loadout::Skill2Trigger1,
            Loadout::Skill2Trigger2,
        ]
    );
    assert_eq!(
        costumes(Ryu, EraFamily::Sf5),
        vec![Costume::Classic1987, Costume::Sf5]
    );
}
