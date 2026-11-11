//! What-if SF2 and SFZ3 styles, filters, and expansion toggles (`#115`, `M2`).

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{listed_families, listed_revisions, EraFamily, Loadout, Revision};
use sf_engine::roster::{CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, TournamentFighter};
use sf_engine::what_if::{mike_or_normal, Button, WhatIfEvent, WhatIfSession};

#[test]
fn expansion_toggles_follow_the_preset() {
    let faithful = MatchConfig::from_preset(RulesetPreset::Faithful1987);
    assert!(!faithful.roster.what_if_styles);
    assert!(!faithful.roster.bonus_challengers);
    assert!(!faithful.roster.secret_intrusions);
    assert!(!MatchConfig::from_preset(RulesetPreset::ClassicCompetitive).roster.what_if_styles);
    assert!(!MatchConfig::from_preset(RulesetPreset::PureFootsies1987Plus).roster.what_if_styles);
    let over = MatchConfig::definitive();
    assert!(over.roster.what_if_styles && over.roster.bonus_challengers && over.roster.secret_intrusions);
    let round = MatchConfig::from_toml(&over.to_toml()).unwrap();
    assert!(round.roster.what_if_styles);
}

#[test]
fn filter_hides_what_if_until_the_button() {
    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.mode = PlayMode::Arcade;
    let mut lee = CharacterSelectScreen::open(settings, true);
    for _ in 0..16 {
        if lee.rows().iter().any(|row| row.starts_with("> ") && row.contains("Lee")) {
            lee.apply(MenuInput::Confirm);
            break;
        }
        lee.apply(MenuInput::Right);
    }
    lee.apply(MenuInput::Confirm);
    let hidden = lee.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(!hidden.contains("SF2"), "{hidden}");
    assert!(lee.rows().iter().any(|row| row.contains("Canonical Styles Only")));
    assert!(lee.rows().iter().any(|row| row.contains("1987 Tournament 12")));

    lee.apply(MenuInput::Filter);
    let shown = lee.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(shown.contains("SF2 [WHAT-IF]"), "{shown}");
    assert!(shown.contains("SFZ [WHAT-IF]"), "{shown}");
    assert!(lee.rows().iter().any(|row| row.contains("All Styles (Incl. What-If)")));
    assert!(lee.rows().iter().any(|row| row.contains("Expanded 25 Roster")));

    let mut faithful = CharacterSelectScreen::open(RosterSettings::faithful(), true);
    faithful.apply(MenuInput::Filter);
    assert!(faithful.rows().iter().any(|row| row.contains("Canonical Styles Only")));
}

#[test]
fn flagship_revisions_and_named_kits() {
    assert_eq!(
        listed_revisions(TournamentFighter::Lee, EraFamily::Sf2, true),
        vec![Revision::Hf, Revision::St]
    );
    assert_eq!(listed_revisions(TournamentFighter::Retsu, EraFamily::SfZ, true), vec![Revision::Sfz3]);
    assert!(listed_families(TournamentFighter::Gen, true).contains(&EraFamily::Sf2));
    assert!(!listed_families(TournamentFighter::Gen, true).iter().any(|family| *family == EraFamily::SfZ) || listed_revisions(TournamentFighter::Gen, EraFamily::SfZ, true).len() > 1);
    assert!(listed_revisions(TournamentFighter::Adon, EraFamily::Sf2, true) == vec![Revision::Hf, Revision::St]);
    assert!(listed_revisions(TournamentFighter::Lee, EraFamily::Sf2, false).is_empty());

    assert_eq!(mike_or_normal(TournamentFighter::Mike, Revision::Sf1Plus, Button::Lk), "Kick");
    assert_eq!(mike_or_normal(TournamentFighter::Mike, Revision::St, Button::Lp), "Head Punch");
    assert_eq!(mike_or_normal(TournamentFighter::Mike, Revision::Sfz3, Button::Lk), "Body Hook");
    assert_eq!(mike_or_normal(TournamentFighter::Mike, Revision::Hf, Button::Hk), "Uppercut");

    let mut hf = WhatIfSession::new(TournamentFighter::Retsu, Revision::Hf, Loadout::Standard);
    assert!(hf.special("Tobi Geri", 0));
    assert!(hf.special("Hundred Monk Slaps", 0));
    assert!(!hf.super_move());

    let mut st = WhatIfSession::new(TournamentFighter::Geki, Revision::St, Loadout::Standard);
    assert!(st.special("Shuriken", 0));
    assert!(st.special("Shuriken", 2));
    assert!(!st.special("Shuriken", 3));
    assert!(st.special("Utsusemi", 2));
    st.special("Geki-Zan", 0);
    assert!(st.super_move());
    assert!(matches!(st.events.last(), Some(WhatIfEvent::Super { name: "Ansatsu Geki-Retsuzan", .. })));

    let mut joe = WhatIfSession::new(TournamentFighter::Joe, Revision::St, Loadout::Standard);
    assert!(joe.special("Fist Combination", 1));
    assert!(joe.special("Fist Combination", 3));
    assert!(!joe.special("Fist Combination", 4));
    joe.special("Rolling Sobat", 0);
    assert!(joe.super_move());
    assert!(matches!(joe.events.last(), Some(WhatIfEvent::Super { name: "Thrash Kick", .. })));

    let mut mike = WhatIfSession::new(TournamentFighter::Mike, Revision::Sfz3, Loadout::AIsm);
    assert!(mike.normal(Button::Hp));
    assert!(matches!(mike.events.last(), Some(WhatIfEvent::Normal("Head Punch"))));
    assert!(mike.special("Murder Straight", 0));
    assert!(mike.special("Ducking Sway", 0));
    mike.special("Rolling Blow", 0);
    assert!(mike.super_move());
    assert!(matches!(mike.events.last(), Some(WhatIfEvent::Super { name: "Crazy Rush", .. })));

    let mut lee = WhatIfSession::new(TournamentFighter::Lee, Revision::Sfz3, Loadout::XIsm);
    assert!(lee.special("Open Canvas Punch", 0));
    assert!(lee.special("Open Canvas Kick", 0));
    assert!(!lee.super_move());
    assert!(lee.special("Upward Lance", 0));
    assert_eq!(lee.stocks(), 3);
    assert!(lee.super_move());
    let x = match lee.events.last() {
        Some(WhatIfEvent::Super { name: "Hundred Spears", damage }) => *damage,
        other => panic!("{other:?}"),
    };

    let mut a_ism = WhatIfSession::new(TournamentFighter::Lee, Revision::Sfz3, Loadout::AIsm);
    a_ism.special("Firewater", 0);
    a_ism.special("Hurricane Kick", 0);
    a_ism.special("Cut Kick", 0);
    assert!(a_ism.super_move());
    let a = match a_ism.events.last() {
        Some(WhatIfEvent::Super { damage, .. }) => *damage,
        other => panic!("{other:?}"),
    };
    assert!(x > a);

    let mut v = WhatIfSession::new(TournamentFighter::Retsu, Revision::Sfz3, Loadout::VIsm);
    assert!(v.special("Rising Dragon Knee", 0));
    assert!(!v.super_move());
    assert!(v.custom_combo());

    let mut gen = WhatIfSession::new(TournamentFighter::Gen, Revision::St, Loadout::Standard);
    assert!(gen.special("Hyakurenko", 0));
    assert!(gen.special("Gekiro", 0));
    assert!(gen.super_move());
    let mut birdie = WhatIfSession::new(TournamentFighter::Birdie, Revision::Hf, Loadout::Standard);
    assert!(birdie.special("Bull Horn", 0));
    assert!(!birdie.super_move());
    let mut eagle = WhatIfSession::new(TournamentFighter::Eagle, Revision::St, Loadout::Standard);
    eagle.special("Manchester Black", 0);
    eagle.special("St. Andrew's Green", 0);
    assert!(eagle.super_move());
    let mut adon = WhatIfSession::new(TournamentFighter::Adon, Revision::Hf, Loadout::Standard);
    assert!(adon.special("Jaguar Kick", 0));
    assert!(!WhatIfSession::new(TournamentFighter::Gen, Revision::Sfz3, Loadout::AIsm).allowed());
    assert!(!hf.tiles.is_empty());
}

#[test]
fn what_if_gauges_follow_their_revision() {
    use sf_video::era_hud::{EraGauge, Fill, GaugeRow};
    let mut st = WhatIfSession::new(TournamentFighter::Geki, Revision::St, Loadout::Standard);
    assert_eq!(st.hud(), Some(EraGauge::one(GaugeRow::stock(1, 0, Fill::of(0, 48)))));
    assert!(st.normal(Button::Lp));
    assert_eq!(st.hud(), Some(EraGauge::one(GaugeRow::stock(1, 0, Fill::of(16, 48)))));
    let mut sfz3 = WhatIfSession::new(TournamentFighter::Mike, Revision::Sfz3, Loadout::AIsm);
    assert!(sfz3.normal(Button::Lp));
    assert_eq!(sfz3.hud(), Some(EraGauge::one(GaugeRow::level(1, 3, Fill::of(1, 3)))));
    let hf = WhatIfSession::new(TournamentFighter::Retsu, Revision::Hf, Loadout::Standard);
    assert_eq!(hf.hud(), None, "Hyper Fighting has no gauge");
}
