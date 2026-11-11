//! SF6 Drive gauge, Super Arts, and safety toggles (`#108`, `O17`).

use sf_engine::config::MatchConfig;
use sf_engine::era::{costumes, revisions, Costume, EraFamily, Revision};
use sf_engine::roster::TournamentFighter;
use sf_engine::sf6::{Rush, Sf6Event, Sf6Session};
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};

fn ryu(raw_off: bool, buffer: bool) -> Sf6Session {
    Sf6Session::new(TournamentFighter::Ryu, Costume::Classic1987, raw_off, buffer)
}

#[test]
fn roster_costumes_and_supers() {
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
    let ryu = Sf6Session::new(TournamentFighter::Ryu, Costume::Sf6, false, true);
    let ken = Sf6Session::new(TournamentFighter::Ken, Costume::Sf6, false, true);
    assert!(!ryu.tiles.is_empty() && !ken.tiles.is_empty());

    let mut ken = Sf6Session::new(TournamentFighter::Ken, Costume::Classic1987, false, true);
    assert!(ken.special("Jinrai Kick"));
    assert!(ken.jinrai_followup("Low"));
    assert!(!ken.jinrai_followup("Mid"));
    assert!(ken.special("Dragonlash Kick"));
    assert!(ken.special("Quick Dash"));
    ken.land_hit();
    ken.land_hit();
    ken.land_hit();
    assert!(ken.super_art(1));
    assert!(matches!(ken.events.iter().rev().nth(0), Some(Sf6Event::Super { name: "Dragonlash Flame", .. })));
    ken.land_hit();
    ken.land_hit();
    assert!(ken.super_art(2));
    assert!(ken.events.iter().any(|event| matches!(event, Sf6Event::Super { name: "Shippu Jinraikyaku", .. })));

    let mut retsu = Sf6Session::new(TournamentFighter::Retsu, Costume::Classic1987, false, true);
    assert!(retsu.special("Kiai Shot"));
    assert!(retsu.special("Renkiko"));
    assert!(retsu.special("Tobigeri"));
    let boosted = retsu.events.iter().find_map(|event| match event {
        Sf6Event::Special { name: "Tobigeri", damage } => Some(*damage),
        _ => None,
    });
    assert_eq!(boosted, Some(56));
    retsu.land_hit();
    retsu.land_hit();
    retsu.land_hit();
    assert!(retsu.super_art(3));
    assert!(matches!(retsu.events.last(), Some(Sf6Event::Super { level: 3, name: "Super Art III" })));
}

#[test]
fn drive_spend_burnout_and_parry_scale() {
    let mut session = ryu(false, true);
    assert_eq!(session.drive_bars(), 6);
    assert!(session.overdrive("Hashogeki"));
    assert_eq!(session.drive_bars(), 4);
    assert!(session.overdrive("Denjin Charge"));
    assert!(session.overdrive("Hashogeki"));
    assert!(session.burnout());
    assert!(session.events.iter().any(|event| matches!(event, Sf6Event::Burnout)));
    assert_eq!(session.blockstun(10), 14);
    assert_eq!(session.blocked_special_chip(10), 5);
    assert!(!session.overdrive("Hashogeki"));

    let mut fresh = ryu(false, true);
    assert_eq!(fresh.blocked_special_chip(10), 0);
    assert!(fresh.drive_parry(2));
    assert!(matches!(fresh.events.last(), Some(Sf6Event::Parry { perfect: true })));
    assert_eq!(fresh.scaled_damage(100), 50);
    let mut late = ryu(false, true);
    assert!(late.drive_parry(3));
    assert_eq!(late.scaled_damage(100), 80);
    assert!(late.drive_bars() >= 6);
}

#[test]
fn rush_impact_reversal_and_toggles() {
    let mut open = ryu(false, false);
    assert!(open.drive_rush(Rush::Raw));
    assert_eq!(open.drive_bars(), 5);
    let mut locked = ryu(true, false);
    assert!(!locked.drive_rush(Rush::Raw));
    assert_eq!(locked.drive_bars(), 6);
    assert!(locked.drive_rush(Rush::Cancel));
    assert_eq!(locked.drive_bars(), 3);
    assert!(locked.drive_parry(1));
    assert!(locked.drive_rush(Rush::Parry));

    let mut impact = ryu(false, true);
    assert!(impact.drive_impact(false, false));
    assert!(impact.absorb());
    assert!(impact.absorb());
    assert!(!impact.absorb());
    assert!(!impact.events.iter().any(|event| matches!(event, Sf6Event::WallSplat)));
    let mut corner = ryu(false, true);
    assert!(corner.drive_impact(true, false));
    assert!(corner.events.iter().any(|event| matches!(event, Sf6Event::WallSplat)));
    let mut burnt = ryu(false, true);
    assert!(burnt.drive_impact(false, true));
    assert!(burnt.events.iter().any(|event| matches!(event, Sf6Event::WallSplat)));

    let mut reversal = ryu(false, true);
    assert!(!reversal.drive_reversal());
    reversal.block();
    assert!(reversal.drive_reversal());
    assert_eq!(reversal.drive_bars(), 4);

    let mut held = ryu(false, true);
    assert!(held.special("Hashogeki"));
    assert!(!held.feed(true));
    assert!(held.release_flash());
    let mut dropped = ryu(false, false);
    assert!(dropped.special("Hashogeki"));
    dropped.feed(true);
    assert!(!dropped.release_flash());

    held.land_hit();
    held.next_round();
    assert_eq!(held.drive_bars(), 6);
    assert_eq!(held.super_stock(), 1);

    let mut cfg = MatchConfig::definitive();
    cfg.disable_raw_drive_rush = true;
    let round = MatchConfig::from_toml(&cfg.to_toml()).unwrap();
    assert!(round.disable_raw_drive_rush);
    assert!(round.sf1plus.disable_freeze_input_lockout);
}

#[test]
fn six_drive_cells_with_half_bars_over_the_super_art_bar() {
    let gauge = |drive: GaugeRow, level: u8| Some(EraGauge::two(drive, GaugeRow::level(level, 3, Fill::of(level.into(), 3))));
    let mut session = ryu(false, true);
    assert_eq!(session.hud(), gauge(GaugeRow::stock(6, 6, Fill::EMPTY), 0));
    session.land_hit();
    assert!(session.drive_rush(Rush::Raw));
    assert_eq!(session.hud(), gauge(GaugeRow::stock(6, 5, Fill::of(0, 2)), 1));
    while session.drive_rush(Rush::Raw) {}
    assert!(session.burnout());
    assert_eq!(
        session.hud(),
        gauge(GaugeRow::stock(6, 0, Fill::of(0, 2)).with_state(GaugeState::Greyed), 1),
        "Burnout greys the Drive gauge"
    );
    session.block();
    assert_eq!(session.hud(), gauge(GaugeRow::stock(6, 0, Fill::of(1, 2)), 1), "half a bar back");
    let lee = Sf6Session::new(TournamentFighter::Lee, Costume::Classic1987, false, false);
    assert_eq!(lee.hud(), None);
}
