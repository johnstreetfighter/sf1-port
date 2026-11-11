//! SF3 revisions, Parry, and Super Arts (`#105`, `O14`).

use sf_engine::era::{loadouts, revisions, EraFamily, Loadout, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::sf3::{Attack, Sf3Event, Sf3Rev, Sf3Session};
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

#[test]
fn ryu_and_ken_get_three_revisions_and_three_arts() {
    assert_eq!(
        revisions(TournamentFighter::Ryu, EraFamily::Sf3),
        vec![Revision::Sf3Ng, Revision::Sf3TwoI, Revision::Sf3ThreeS]
    );
    assert_eq!(revisions(TournamentFighter::Ken, EraFamily::Sf3), revisions(TournamentFighter::Ryu, EraFamily::Sf3));
    assert!(revisions(TournamentFighter::Sagat, EraFamily::Sf3).is_empty());
    assert_eq!(loadouts(Revision::Sf3ThreeS), vec![Loadout::Art1, Loadout::Art2, Loadout::Art3]);
}

#[test]
fn parry_windows_ex_and_all_six_super_arts() {
    let mut high = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::ThreeS, Loadout::Art1);
    high.offer(Attack { age: 4, low: false, projectile: false });
    assert!(high.tap_forward());
    assert!(matches!(high.events.last(), Some(Sf3Event::Parry { red: false, advantage: 8 })));

    let mut late = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::Ng, Loadout::Art1);
    late.offer(Attack { age: 11, low: false, projectile: false });
    assert!(!late.tap_forward());

    let mut ball = Sf3Session::new(TournamentFighter::Ken, Sf3Rev::TwoI, Loadout::Art2);
    ball.offer(Attack { age: 7, low: false, projectile: true });
    assert!(!ball.tap_forward());
    ball.offer(Attack { age: 5, low: false, projectile: true });
    assert!(ball.tap_forward());

    let mut low = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::Ng, Loadout::Art1);
    low.offer(Attack { age: 2, low: true, projectile: false });
    assert!(!low.tap_forward());
    assert!(low.tap_down());

    let mut air = Sf3Session::new(TournamentFighter::Ken, Sf3Rev::ThreeS, Loadout::Art1);
    air.jump();
    air.offer(Attack { age: 3, low: false, projectile: false });
    assert!(air.tap_forward());

    let mut red = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::TwoI, Loadout::Art3);
    red.enter_block(10);
    red.offer(Attack { age: 20, low: false, projectile: false });
    assert!(red.tap_forward());
    assert!(matches!(red.events.last(), Some(Sf3Event::Parry { red: true, advantage: 6 })));

    let mut ng = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::Ng, Loadout::Art1);
    ng.add_meter(120);
    assert_eq!(ng.stocks(), 1);
    assert!(!ng.ex_special("Hadoken"));
    let mut ex = Sf3Session::new(TournamentFighter::Ken, Sf3Rev::TwoI, Loadout::Art2);
    ex.add_meter(80);
    assert!(ex.ex_special("Hadoken"));
    assert_eq!(ex.stocks(), 0);

    let mut art1 = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::Ng, Loadout::Art1);
    assert_eq!(art1.bar_length(), 120);
    assert_eq!(art1.max_stocks(), 1);
    art1.add_meter(500);
    assert_eq!(art1.stocks(), 1);
    assert!(art1.super_art());
    assert!(matches!(art1.events[0], Sf3Event::Super { name: "Shinku Hadoken", stocks: 1 }));

    let mut shin = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::ThreeS, Loadout::Art2);
    shin.add_meter(160);
    assert_eq!(shin.stocks(), 2);
    assert!(shin.super_art());
    assert!(shin.events.iter().any(|event| matches!(event, Sf3Event::Super { name: "Shin Shoryuken", .. })));

    let mut denjin = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::ThreeS, Loadout::Art3);
    denjin.add_meter(120);
    assert_eq!(denjin.stocks(), 3);
    assert!(!denjin.super_art());
    denjin.charge_denjin(30);
    assert!(denjin.super_art());
    assert!(denjin.events.iter().any(|event| matches!(event, Sf3Event::Move { name: "Denjin Hadoken", unblockable: true, .. })));

    let mut ken1 = Sf3Session::new(TournamentFighter::Ken, Sf3Rev::Ng, Loadout::Art1);
    ken1.add_meter(120);
    assert!(ken1.super_art());
    assert!(ken1.events.iter().any(|event| matches!(event, Sf3Event::Super { name: "Shoryureppa", .. })));
    let mut ken2 = Sf3Session::new(TournamentFighter::Ken, Sf3Rev::TwoI, Loadout::Art2);
    ken2.add_meter(160);
    assert!(ken2.super_art());
    assert!(ken2.events.iter().any(|event| matches!(event, Sf3Event::Super { name: "Shinryuken", .. })));
    let mut shippu = Sf3Session::new(TournamentFighter::Ken, Sf3Rev::ThreeS, Loadout::Art3);
    shippu.add_meter(120);
    assert!(shippu.super_art());
    assert!(shippu.events.iter().any(|event| matches!(event, Sf3Event::Move { name: "Shippu Jinraikyaku", startup: 1, .. })));

    let mut ryu = Sf3Session::new(TournamentFighter::Ryu, Sf3Rev::ThreeS, Loadout::Art1);
    assert!(ryu.joudan());
    assert!(ryu.overhead());
    assert!(ryu.target_combo());
    assert_eq!(ryu.events.iter().filter(|event| matches!(event, Sf3Event::Move { .. })).count(), 4);
    assert!(!Sf3Session::new(TournamentFighter::Ken, Sf3Rev::Ng, Loadout::Art1).joudan());
    assert_eq!(
        ryu.hud(),
        Some(EraGauge::one(GaugeRow::level(0, 1, Fill::of(0, 120)).with_length(Fill::of(120, 120))))
    );
}

#[test]
fn the_super_art_bar_fills_from_the_meter_and_its_length_follows_the_art() {
    let bar = |art: Loadout| {
        let mut session = Sf3Session::new(TournamentFighter::Ken, Sf3Rev::ThreeS, art);
        session.add_meter(20);
        session.hud().unwrap()
    };
    assert_eq!(bar(Loadout::Art1), EraGauge::one(GaugeRow::level(0, 1, Fill::of(20, 120)).with_length(Fill::of(120, 120))));
    assert_eq!(bar(Loadout::Art2), EraGauge::one(GaugeRow::level(0, 2, Fill::of(20, 80)).with_length(Fill::of(80, 120))));
    assert_eq!(bar(Loadout::Art3), EraGauge::one(GaugeRow::level(0, 3, Fill::of(20, 40)).with_length(Fill::of(40, 120))));

    let mut shippu = Sf3Session::new(TournamentFighter::Ken, Sf3Rev::ThreeS, Loadout::Art3);
    shippu.add_meter(50);
    assert_eq!(shippu.hud(), Some(EraGauge::one(GaugeRow::level(1, 3, Fill::of(10, 40)).with_length(Fill::of(40, 120)))));
    shippu.add_meter(120);
    let full = GaugeRow::level(3, 3, Fill::FULL).with_length(Fill::of(40, 120));
    assert_eq!(shippu.hud(), Some(EraGauge::one(full)));
    assert!(matches!(full, GaugeRow::Level { state: GaugeState::Full, .. }));
}
