//! SF5 revisions, V-System, and Critical Arts (`#107`, `O16`).

use sf_engine::era::{costumes, loadouts, revisions, Costume, EraFamily, Loadout, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::sf5::{Sf5Event, Sf5Session};
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

#[test]
fn roster_and_loadouts() {
    assert_eq!(
        revisions(TournamentFighter::Ryu, EraFamily::Sf5),
        vec![Revision::Sf5, Revision::Sf5Ae, Revision::Sf5Ce]
    );
    assert_eq!(revisions(TournamentFighter::Birdie, EraFamily::Sf5), revisions(TournamentFighter::Ryu, EraFamily::Sf5));
    assert_eq!(revisions(TournamentFighter::Sagat, EraFamily::Sf5), vec![Revision::Sf5Ae, Revision::Sf5Ce]);
    assert!(revisions(TournamentFighter::Lee, EraFamily::Sf5).is_empty());
    assert_eq!(loadouts(Revision::Sf5), vec![Loadout::Skill1Trigger1]);
    assert_eq!(loadouts(Revision::Sf5Ae), vec![Loadout::Skill1Trigger1, Loadout::Skill1Trigger2]);
    assert_eq!(loadouts(Revision::Sf5Ce).len(), 4);
    assert_eq!(
        costumes(TournamentFighter::Ryu, EraFamily::Sf5),
        vec![Costume::Classic1987, Costume::Sf5]
    );
    let early = Sf5Session::new(TournamentFighter::Sagat, Revision::Sf5, Loadout::Skill1Trigger1, Costume::Classic1987);
    assert!(!early.available());
}

#[test]
fn v_system_ex_and_critical_art() {
    let mut ryu = Sf5Session::new(TournamentFighter::Ryu, Revision::Sf5Ce, Loadout::Skill2Trigger2, Costume::Sf5);
    assert_eq!(ryu.skill_name(), "Thrust Strike");
    assert_eq!(ryu.trigger_name(), "Kakko Fubatsu");
    assert_eq!(ryu.max_v(), 3);
    assert!(ryu.v_skill());
    assert!(matches!(ryu.events.last(), Some(Sf5Event::VSkill("Thrust Strike"))));
    assert!(!ryu.v_reversal());
    ryu.enter_block(8);
    assert!(!ryu.v_reversal());
    for _ in 0..6 {
        ryu.land_hit(false);
    }
    assert!(ryu.v_bars() >= 1);
    assert!(ryu.v_reversal());
    while ryu.v_bars() < ryu.max_v() {
        ryu.land_hit(true);
    }
    assert!(ryu.events.iter().any(|event| matches!(event, Sf5Event::CrushCounter)));
    assert!(ryu.v_trigger());
    assert!(matches!(ryu.events.last(), Some(Sf5Event::VTrigger("Kakko Fubatsu"))));
    assert!(ryu.trigger_active());

    let mut ken = Sf5Session::new(TournamentFighter::Ken, Revision::Sf5Ae, Loadout::Skill1Trigger1, Costume::Classic1987);
    assert_eq!(ken.skill_name(), "Quick Step");
    assert_eq!(ken.trigger_name(), "Heat Rush");
    assert_eq!(ken.max_v(), 2);
    assert!(!ken.v_shift());

    let mut birdie = Sf5Session::new(TournamentFighter::Birdie, Revision::Sf5Ce, Loadout::Skill1Trigger1, Costume::Sf5);
    assert_eq!(birdie.skill_name(), "Break Time");
    birdie.land_hit(false);
    birdie.land_hit(false);
    assert!(birdie.v_shift());
    assert!(birdie.v_shift_break());
    assert!(matches!(birdie.events.last(), Some(Sf5Event::VShiftBreak)));

    let sagat = Sf5Session::new(TournamentFighter::Sagat, Revision::Sf5Ae, Loadout::Skill1Trigger2, Costume::Classic1987);
    assert_eq!(sagat.trigger_name(), "Tiger Assault");
    assert_eq!(sagat.max_v(), 3);

    let mut art = Sf5Session::new(TournamentFighter::Ryu, Revision::Sf5, Loadout::Skill1Trigger1, Costume::Classic1987);
    assert!(!art.critical_art());
    art.land_hit(false);
    assert!(art.ex_special("Hadoken"));
    assert_eq!(art.ex_stocks(), 0);
    art.land_hit(false);
    art.land_hit(false);
    art.land_hit(false);
    assert!(art.critical_art());
    assert!(matches!(art.events.last(), Some(Sf5Event::CriticalArt)));
    assert_eq!(
        art.hud(),
        Some(EraGauge::two(GaugeRow::stock(3, 0, Fill::EMPTY), GaugeRow::stock(2, 2, Fill::EMPTY))),
        "no EX left over a full two-bar V-Gauge"
    );
    assert!(art.v_trigger());
    assert_eq!(
        art.hud(),
        Some(EraGauge::two(
            GaugeRow::stock(3, 0, Fill::EMPTY),
            GaugeRow::stock(2, 0, Fill::of(4, 30)).with_state(GaugeState::Draining)
        )),
        "V-Trigger drains the V-Gauge"
    );
    assert!(!art.tiles.is_empty());
}

#[test]
fn three_ex_cells_over_the_v_gauge_with_its_meter() {
    let mut ryu = Sf5Session::new(TournamentFighter::Ryu, Revision::Sf5, Loadout::Skill1Trigger1, Costume::Classic1987);
    ryu.land_hit(false);
    assert_eq!(
        ryu.hud(),
        Some(EraGauge::two(GaugeRow::stock(3, 1, Fill::EMPTY), GaugeRow::stock(2, 0, Fill::of(16, 30))))
    );
    let sagat = Sf5Session::new(TournamentFighter::Sagat, Revision::Sf5Ae, Loadout::Skill1Trigger2, Costume::Classic1987);
    assert!(matches!(sagat.hud().unwrap().second, Some(GaugeRow::Stock { cells: 3, .. })), "a three-bar V-Trigger");
    let vanilla_sagat = Sf5Session::new(TournamentFighter::Sagat, Revision::Sf5, Loadout::Skill1Trigger1, Costume::Classic1987);
    assert_eq!(vanilla_sagat.hud(), None);
}
