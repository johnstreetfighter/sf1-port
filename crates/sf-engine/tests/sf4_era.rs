//! SF4 revisions, Focus, FADC, and Revenge Ultras (`#106`, `O15`).

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{loadouts, revisions, EraFamily, Loadout, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::sf4::{Sf4Event, Sf4Rev, Sf4Session, Stance, REVENGE_AT};
use sf_video::era_hud::{EraGauge, Fill, GaugeRow};

#[test]
fn roster_revisions_and_ultra_loadouts() {
    assert_eq!(
        revisions(TournamentFighter::Ryu, EraFamily::Sf4),
        vec![Revision::Sf4, Revision::Ssf4, Revision::Ae2012, Revision::Usf4]
    );
    assert_eq!(
        revisions(TournamentFighter::Sagat, EraFamily::Sf4),
        revisions(TournamentFighter::Ryu, EraFamily::Sf4)
    );
    assert_eq!(
        revisions(TournamentFighter::Adon, EraFamily::Sf4),
        vec![Revision::Ssf4, Revision::Ae2012, Revision::Usf4]
    );
    assert_eq!(revisions(TournamentFighter::Gen, EraFamily::Sf4), revisions(TournamentFighter::Adon, EraFamily::Sf4));
    assert!(revisions(TournamentFighter::Birdie, EraFamily::Sf4).is_empty());
    assert_eq!(loadouts(Revision::Sf4), vec![Loadout::Ultra1]);
    assert_eq!(loadouts(Revision::Ssf4), vec![Loadout::Ultra1, Loadout::Ultra2]);
    assert_eq!(
        loadouts(Revision::Usf4),
        vec![Loadout::Ultra1, Loadout::Ultra2, Loadout::UltraDouble]
    );
    assert!(!MatchConfig::from_preset(RulesetPreset::Faithful1987).disable_comeback_scaling);
    let round = MatchConfig::from_toml(&MatchConfig::definitive().to_toml()).unwrap();
    assert!(!round.disable_comeback_scaling);
}

#[test]
fn focus_armor_fadc_and_four_bar_super() {
    let mut early = Sf4Session::new(TournamentFighter::Adon, Sf4Rev::Sf4, Loadout::Ultra1, false);
    assert!(!early.available());
    assert!(!early.focus(30, false));

    let mut focus = Sf4Session::new(TournamentFighter::Ryu, Sf4Rev::Ae2012, Loadout::Ultra1, false);
    assert!(focus.focus(5, false));
    assert!(focus.absorb());
    assert!(!focus.absorb());
    assert!(matches!(
        focus.events[0],
        Sf4Event::Focus { level: 1, crumple: false, unblockable: false, red: false }
    ));

    let mut level3 = Sf4Session::new(TournamentFighter::Ken, Sf4Rev::Ssf4, Loadout::Ultra2, false);
    assert!(level3.focus(21, true));
    assert!(matches!(
        level3.events[0],
        Sf4Event::Focus { level: 3, red: false, crumple: true, unblockable: true }
    ));

    let mut red = Sf4Session::new(TournamentFighter::Ryu, Sf4Rev::Usf4, Loadout::Ultra1, false);
    assert!(red.focus(12, true));
    assert!(red.absorb() && red.absorb() && red.absorb());
    assert!(!red.absorb());
    assert!(matches!(red.events[0], Sf4Event::Focus { level: 2, red: true, crumple: true, .. }));

    let mut cancel = Sf4Session::new(TournamentFighter::Sagat, Sf4Rev::Usf4, Loadout::Ultra1, false);
    cancel.add_bars(2);
    assert!(!cancel.fadc());
    assert!(cancel.focus(11, false));
    assert!(cancel.fadc());
    assert_eq!(cancel.bars(), 0);

    let mut ex = Sf4Session::new(TournamentFighter::Ryu, Sf4Rev::Sf4, Loadout::Ultra1, false);
    ex.add_bars(4);
    assert!(ex.ex_special("Hadoken"));
    assert_eq!(ex.bars(), 3);
    assert!(ex.fadc());
    assert_eq!(ex.bars(), 1);
    ex.add_bars(3);
    assert!(ex.super_combo());
    assert!(matches!(ex.events.last(), Some(Sf4Event::Super("Shinku Hadoken"))));
    assert_eq!(ex.bars(), 0);
}

#[test]
fn revenge_ultras_red_focus_and_delayed_wakeup() {
    let mut ryu = Sf4Session::new(TournamentFighter::Ryu, Sf4Rev::Ssf4, Loadout::Ultra1, false);
    ryu.take_damage(REVENGE_AT - 1);
    assert!(!ryu.ultra());
    ryu.take_damage(1);
    assert!(ryu.ultra());
    assert!(matches!(ryu.events.last(), Some(Sf4Event::Ultra { name: "Metsu Hadoken", damage: 400 })));

    let mut ken = Sf4Session::new(TournamentFighter::Ken, Sf4Rev::Ae2012, Loadout::Ultra2, false);
    ken.take_damage(REVENGE_AT);
    assert!(ken.ultra());
    assert!(matches!(ken.events.last(), Some(Sf4Event::Ultra { name: "Guren Senpukyaku", .. })));

    let mut sagat = Sf4Session::new(TournamentFighter::Sagat, Sf4Rev::Usf4, Loadout::Ultra2, false);
    assert!(sagat.angry_charge());
    sagat.take_damage(REVENGE_AT);
    assert!(sagat.ultra());
    assert!(matches!(sagat.events.last(), Some(Sf4Event::Ultra { name: "Tiger Cannon", damage: 440 })));

    let mut vanilla = Sf4Session::new(TournamentFighter::Ryu, Sf4Rev::Sf4, Loadout::Ultra2, false);
    vanilla.take_damage(REVENGE_AT);
    assert!(!vanilla.ultra());

    let mut scaled = Sf4Session::new(TournamentFighter::Ken, Sf4Rev::Usf4, Loadout::UltraDouble, false);
    scaled.take_damage(REVENGE_AT);
    assert!(scaled.ultra());
    assert!(matches!(scaled.events.last(), Some(Sf4Event::Ultra { name: "Ultra Double", damage: 280 })));

    let mut full = Sf4Session::new(TournamentFighter::Ryu, Sf4Rev::Usf4, Loadout::UltraDouble, true);
    full.take_damage(REVENGE_AT);
    assert!(full.ultra());
    assert!(matches!(full.events.last(), Some(Sf4Event::Ultra { name: "Ultra Double", damage: 400 })));

    let mut adon = Sf4Session::new(TournamentFighter::Adon, Sf4Rev::Ssf4, Loadout::Ultra1, false);
    adon.take_damage(REVENGE_AT);
    assert!(adon.ultra());
    assert!(matches!(adon.events.last(), Some(Sf4Event::Ultra { name: "Jaguar Revolver", .. })));

    let mut gen = Sf4Session::new(TournamentFighter::Gen, Sf4Rev::Usf4, Loadout::Ultra1, false);
    assert!(gen.switch_stance(false));
    gen.take_damage(REVENGE_AT);
    assert!(gen.ultra());
    assert!(matches!(gen.events.last(), Some(Sf4Event::Ultra { name: "Teiga", .. })));
    assert!(gen.events.iter().any(|event| matches!(event, Sf4Event::Stance(Stance::Crane))));

    let mut wake = Sf4Session::new(TournamentFighter::Ken, Sf4Rev::Usf4, Loadout::Ultra1, false);
    assert!(!wake.delayed_wakeup(2));
    wake.knock_down();
    assert!(!wake.delayed_wakeup(1));
    assert!(wake.delayed_wakeup(2));
    let mut earlier = Sf4Session::new(TournamentFighter::Ken, Sf4Rev::Ae2012, Loadout::Ultra1, false);
    earlier.knock_down();
    assert!(!earlier.delayed_wakeup(2));

    let hud = Sf4Session::new(TournamentFighter::Ryu, Sf4Rev::Usf4, Loadout::Ultra1, false);
    let mut hud = hud;
    hud.add_bars(4);
    hud.take_damage(REVENGE_AT);
    assert_eq!(
        hud.hud(),
        Some(EraGauge::two(GaugeRow::stock(4, 4, Fill::EMPTY), GaugeRow::stock(1, 1, Fill::FULL))),
        "four full Super cells over a full Revenge gauge"
    );
}

#[test]
fn four_super_cells_over_the_revenge_gauge() {
    let mut ryu = Sf4Session::new(TournamentFighter::Ryu, Sf4Rev::Usf4, Loadout::Ultra1, false);
    assert_eq!(
        ryu.hud(),
        Some(EraGauge::two(GaugeRow::stock(4, 0, Fill::EMPTY), GaugeRow::stock(1, 0, Fill::of(0, REVENGE_AT))))
    );
    ryu.add_bars(2);
    ryu.take_damage(50);
    assert_eq!(
        ryu.hud(),
        Some(EraGauge::two(GaugeRow::stock(4, 2, Fill::EMPTY), GaugeRow::stock(1, 0, Fill::of(50, REVENGE_AT))))
    );
    let adon = Sf4Session::new(TournamentFighter::Adon, Sf4Rev::Sf4, Loadout::Ultra1, false);
    assert_eq!(adon.hud(), None, "a fighter the revision lacks shows nothing");
}
