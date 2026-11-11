//! SF1+ combat session for all twelve tournament fighters (`#101`, `O10`; `#102`, `O11`).

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::roster::TournamentFighter;
use sf_engine::sf1plus::{
    hud_cells, CombatEvent, CombatSession, EraStyle, Guard, MoveKind, Pad, Sf1PlusRules,
    CLASSIC_SPECIAL_DAMAGE, HEALTH_MAX,
};
use sf_video::asset_pack::{validate_adpcm, validate_fm_patch, validate_tile};

fn expected_specials(fighter: TournamentFighter) -> &'static [&'static str] {
    match fighter {
        TournamentFighter::Lee => &["Zesshouohken", "Double Kick"],
        TournamentFighter::Gen => &["Hyakurenko", "Gekiro"],
        TournamentFighter::Birdie => &["Bull Head", "Bull Horn"],
        TournamentFighter::Eagle => &["Manchester Black", "St. Andrew's Green"],
        TournamentFighter::Adon => &["Jaguar Kick", "Rising Jaguar"],
        TournamentFighter::Sagat => &["High Tiger Shot", "Low Tiger Shot", "Tiger Knee"],
        _ => &[],
    }
}

fn started(events: &[CombatEvent], name: &str) -> bool {
    events
        .iter()
        .any(|event| matches!(event, CombatEvent::Started(got) if *got == name))
}

#[test]
fn every_tournament_fighter_plays_the_full_kit_and_cancels() {
    for fighter in TournamentFighter::ALL {
        let mut session = CombatSession::new(
            fighter,
            TournamentFighter::Ken,
            Sf1PlusRules::for_preset(RulesetPreset::DefinitiveOverhaul),
        );
        let kit = session.p1_kit().clone();
        assert!(validate_adpcm(&kit.voice).is_ok(), "{fighter:?} voice");
        assert_eq!(kit.voice.sample_rate, 4_000);
        assert!(validate_fm_patch(&kit.patch).is_ok(), "{fighter:?} FM cue");
        assert!(kit
            .moves
            .iter()
            .any(|mv| mv.kind == MoveKind::CommandNormal));
        assert!(kit.moves.iter().any(|mv| mv.kind == MoveKind::Throw));
        assert!(kit.moves.iter().any(|mv| mv.kind == MoveKind::Special));
        for posture in ["stand", "crouch", "jump"] {
            for button in ["LP", "MP", "HP", "LK", "MK", "HK"] {
                let name = format!("{posture} {button}");
                assert!(
                    kit.moves
                        .iter()
                        .any(|mv| mv.name == name && mv.kind == MoveKind::Normal),
                    "{fighter:?} missing {name}"
                );
            }
        }
        assert!(kit.moves.iter().any(|mv| mv.guard == Guard::High));
        assert!(kit.moves.iter().any(|mv| mv.guard == Guard::Low));
        for name in expected_specials(fighter) {
            assert!(
                kit.moves
                    .iter()
                    .any(|mv| mv.name == *name && mv.kind == MoveKind::Special),
                "{fighter:?} missing {name}"
            );
        }
        if fighter == TournamentFighter::Gen {
            assert!(kit
                .moves
                .iter()
                .all(|mv| !mv.name.to_ascii_lowercase().contains("stance")));
        }
        for mv in &kit.moves {
            validate_tile(&kit.tiles[mv.tile]).unwrap();
            session.events.clear();
            session.play(&mv.rehearsal());
            assert!(
                started(&session.events, mv.name),
                "{fighter:?} did not start {} from {:?}",
                mv.name,
                mv.rehearsal()
            );
            assert_eq!(session.p1_audio(), Some(mv.audio_latch));
            assert!(session.p1_tile().is_some());
            if mv.kind == MoveKind::Special {
                let mut poses = vec![session.p1_tile().unwrap().pixels];
                for _ in 0..8 {
                    session.step(Pad::default(), Pad::default());
                    if let Some(tile) = session.p1_tile() {
                        poses.push(tile.pixels);
                    }
                }
                let mut distinct = poses.clone();
                distinct.dedup();
                assert!(
                    distinct.len() >= 2,
                    "{fighter:?} {} held one pose across {:?}",
                    mv.name,
                    poses.len()
                );
            }
            session = CombatSession::new(
                fighter,
                TournamentFighter::Ken,
                Sf1PlusRules::for_preset(RulesetPreset::DefinitiveOverhaul),
            );
        }
        let lp = kit.moves.iter().find(|mv| mv.name == "stand LP").unwrap();
        let special = kit
            .moves
            .iter()
            .find(|mv| mv.kind == MoveKind::Special)
            .unwrap();
        let mut cancel = lp.rehearsal();
        cancel.extend(special.rehearsal());
        session.play(&cancel);
        assert!(
            session
                .events
                .iter()
                .any(|event| matches!(event, CombatEvent::Canceled(name) if *name == special.name)),
            "{fighter:?} did not cancel stand LP into {} events={:?}",
            special.name,
            session.events
        );
    }
}

#[test]
fn throws_tech_window_corner_push_and_damage_rules() {
    let rules = Sf1PlusRules::for_preset(RulesetPreset::ClassicCompetitive);
    let mut session = CombatSession::new(TournamentFighter::Ryu, TournamentFighter::Retsu, rules);
    let throw = session
        .p1_kit()
        .moves
        .iter()
        .find(|mv| mv.kind == MoveKind::Throw)
        .unwrap()
        .rehearsal();
    session.play(&throw);
    let mut tech = Pad::default();
    tech.lp = true;
    tech.lk = true;
    session.step(Pad::default(), tech);
    assert!(session.events.contains(&CombatEvent::ThrowTech));
    assert_eq!(session.p2_health(), HEALTH_MAX);

    let mut late = CombatSession::new(TournamentFighter::Ryu, TournamentFighter::Retsu, rules);
    late.play(&throw);
    for _ in 0..7 {
        late.step(Pad::default(), Pad::default());
    }
    late.step(Pad::default(), tech);
    assert!(late
        .events
        .iter()
        .any(|event| matches!(event, CombatEvent::ThrowConnect { .. })));
    assert!(late.p2_health() < HEALTH_MAX);

    let mut untech = rules;
    untech.untechable_sf2_throws = true;
    let mut locked = CombatSession::new(TournamentFighter::Mike, TournamentFighter::Joe, untech);
    locked.play(
        &locked
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.kind == MoveKind::Throw)
            .unwrap()
            .rehearsal(),
    );
    locked.step(Pad::default(), tech);
    assert!(!locked.events.contains(&CombatEvent::ThrowTech));
    assert!(locked
        .events
        .iter()
        .any(|event| matches!(event, CombatEvent::ThrowConnect { .. })));

    let mut corner = CombatSession::new(TournamentFighter::Geki, TournamentFighter::Joe, rules);
    corner.place(340, 360);
    corner.play(
        &corner
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.kind == MoveKind::Throw)
            .unwrap()
            .rehearsal(),
    );
    for _ in 0..30 {
        corner.step(Pad::default(), Pad::default());
    }
    assert!(corner.events.contains(&CombatEvent::WakePush));
    assert!(corner.p2_x() < 360);

    let mut loops = rules;
    loops.allow_corner_throw_loops = true;
    let mut stay = CombatSession::new(TournamentFighter::Geki, TournamentFighter::Joe, loops);
    stay.place(340, 360);
    stay.play(
        &stay
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.kind == MoveKind::Throw)
            .unwrap()
            .rehearsal(),
    );
    for _ in 0..30 {
        stay.step(Pad::default(), Pad::default());
    }
    assert!(!stay.events.contains(&CombatEvent::WakePush));
    assert_eq!(stay.p2_x(), 360);
}

#[test]
fn cross_era_guard_hitstun_freeze_buffer_and_hud() {
    assert_eq!(hud_cells(HEALTH_MAX), 11);
    assert_eq!(hud_cells(0), 0);
    assert_eq!(hud_cells(1), 1);

    let rules = Sf1PlusRules::for_preset(RulesetPreset::DefinitiveOverhaul);
    let mut modern = CombatSession::with_eras(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        rules,
        EraStyle::Sf1Plus,
        EraStyle::Sf1Plus,
    );
    let low = modern
        .p1_kit()
        .moves
        .iter()
        .find(|mv| mv.name == "crouch LK")
        .unwrap()
        .rehearsal();
    let block = Pad {
        down: true,
        right: true,
        ..Pad::default()
    };
    for pad in &low {
        modern.step(*pad, block);
    }
    for _ in 0..8 {
        modern.step(Pad::default(), block);
    }
    assert!(
        modern
            .events
            .iter()
            .any(|event| matches!(event, CombatEvent::Blocked { .. })),
        "{:?}",
        modern.events
    );

    let mut classic_guard = CombatSession::with_eras(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        rules,
        EraStyle::Sf1Plus,
        EraStyle::Sf1987,
    );
    for pad in &low {
        classic_guard.step(*pad, block);
    }
    for _ in 0..8 {
        classic_guard.step(Pad::default(), block);
    }
    assert!(classic_guard
        .events
        .iter()
        .any(|event| matches!(event, CombatEvent::Hit { .. })));

    let mut old_attacker = CombatSession::with_eras(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        rules,
        EraStyle::Sf1987,
        EraStyle::Sf1Plus,
    );
    let lp = old_attacker
        .p1_kit()
        .moves
        .iter()
        .find(|mv| mv.name == "stand LP")
        .unwrap();
    let special = old_attacker
        .p1_kit()
        .moves
        .iter()
        .find(|mv| mv.name == "Hadoken")
        .unwrap();
    let mut seq = lp.rehearsal();
    seq.extend(special.rehearsal());
    old_attacker.play(&seq);
    assert!(!old_attacker
        .events
        .iter()
        .any(|event| matches!(event, CombatEvent::Canceled(_))));
    assert!(old_attacker
        .events
        .iter()
        .any(|event| matches!(event, CombatEvent::Hit { hitstun: 5, .. })));

    let mut scaled = CombatSession::from_config(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        &MatchConfig::definitive(),
    );
    scaled.play(
        &scaled
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.name == "Hadoken")
            .unwrap()
            .rehearsal(),
    );
    for _ in 0..20 {
        scaled.step(Pad::default(), Pad::default());
    }
    let scaled_hit = scaled.events.iter().find_map(|event| match event {
        CombatEvent::Hit { damage, .. } => Some(*damage),
        _ => None,
    });
    assert_ne!(scaled_hit, Some(CLASSIC_SPECIAL_DAMAGE));

    let mut classic = CombatSession::from_config(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        &MatchConfig::faithful(),
    );
    classic.play(
        &classic
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.name == "Hadoken")
            .unwrap()
            .rehearsal(),
    );
    for _ in 0..20 {
        classic.step(Pad::default(), Pad::default());
    }
    assert!(
        classic.events.contains(&CombatEvent::Hit {
            damage: CLASSIC_SPECIAL_DAMAGE,
            hitstun: classic
                .events
                .iter()
                .find_map(|e| match e {
                    CombatEvent::Hit { hitstun, .. } => Some(*hitstun),
                    _ => None,
                })
                .unwrap_or(0)
        }) || classic.events.iter().any(|e| matches!(
            e,
            CombatEvent::Hit {
                damage: CLASSIC_SPECIAL_DAMAGE,
                ..
            }
        ))
    );

    let mut lp_pad = Pad::default();
    lp_pad.lp = true;
    let mut buffered = CombatSession::from_config(
        TournamentFighter::Ken,
        TournamentFighter::Ryu,
        &MatchConfig::definitive(),
    );
    buffered.play(
        &buffered
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.name == "stand LP")
            .unwrap()
            .rehearsal(),
    );
    while !buffered
        .events
        .iter()
        .any(|event| matches!(event, CombatEvent::Hit { .. }))
    {
        buffered.step(Pad::default(), Pad::default());
    }
    buffered.step(lp_pad, Pad::default());
    for _ in 0..24 {
        buffered.step(Pad::default(), Pad::default());
    }
    let started_lps = buffered
        .events
        .iter()
        .filter(|event| matches!(event, CombatEvent::Started("stand LP")))
        .count();
    assert!(
        started_lps >= 2,
        "buffered light punch should come out after hitstop: {:?}",
        buffered.events
    );

    let mut locked = CombatSession::from_config(
        TournamentFighter::Ken,
        TournamentFighter::Ryu,
        &MatchConfig::faithful(),
    );
    locked.play(
        &locked
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.name == "stand LP")
            .unwrap()
            .rehearsal(),
    );
    while !locked
        .events
        .iter()
        .any(|event| matches!(event, CombatEvent::Hit { .. }))
    {
        locked.step(Pad::default(), Pad::default());
    }
    locked.step(lp_pad, Pad::default());
    for _ in 0..24 {
        locked.step(Pad::default(), Pad::default());
    }
    let locked_lps = locked
        .events
        .iter()
        .filter(|event| matches!(event, CombatEvent::Started("stand LP")))
        .count();
    assert_eq!(
        locked_lps, 1,
        "faithful lockout dropped the freeze input: {:?}",
        locked.events
    );

    let round = MatchConfig::from_toml(&MatchConfig::definitive().to_toml()).unwrap();
    assert!(!round.sf1plus.classic_1987_special_damage);
    assert!(round.sf1plus.disable_freeze_input_lockout);
    assert!(!round.sf1plus.untechable_sf2_throws);
    assert!(!round.sf1plus.allow_corner_throw_loops);
}

#[test]
fn every_fighter_blocks_high_and_low_and_techs_a_throw() {
    let rules = Sf1PlusRules::for_preset(RulesetPreset::DefinitiveOverhaul);
    for fighter in TournamentFighter::ALL {
        let mut low = CombatSession::new(TournamentFighter::Ryu, fighter, rules);
        let crouch_lk = low
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.name == "crouch LK")
            .unwrap()
            .rehearsal();
        let guard_low = Pad {
            down: true,
            right: true,
            ..Pad::default()
        };
        for pad in &crouch_lk {
            low.step(*pad, guard_low);
        }
        for _ in 0..8 {
            low.step(Pad::default(), guard_low);
        }
        assert!(
            low.events
                .iter()
                .any(|event| matches!(event, CombatEvent::Blocked { .. })),
            "{fighter:?} did not low-block: {:?}",
            low.events
        );

        let mut high = CombatSession::new(TournamentFighter::Ryu, fighter, rules);
        let stand_hp = high
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.name == "stand HP")
            .unwrap()
            .rehearsal();
        let guard_high = Pad {
            right: true,
            ..Pad::default()
        };
        for pad in &stand_hp {
            high.step(*pad, guard_high);
        }
        for _ in 0..10 {
            high.step(Pad::default(), guard_high);
        }
        assert!(
            high.events
                .iter()
                .any(|event| matches!(event, CombatEvent::Blocked { .. })),
            "{fighter:?} did not high-block: {:?}",
            high.events
        );

        let mut teching = CombatSession::new(fighter, TournamentFighter::Ken, rules);
        let throw = teching
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.kind == MoveKind::Throw)
            .unwrap()
            .rehearsal();
        teching.play(&throw);
        let tech = Pad {
            lp: true,
            lk: true,
            ..Pad::default()
        };
        teching.step(Pad::default(), tech);
        assert!(
            teching.events.contains(&CombatEvent::ThrowTech),
            "{fighter:?} throw was not techable: {:?}",
            teching.events
        );
        assert_eq!(teching.p2_health(), HEALTH_MAX);
    }
}
