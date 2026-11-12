use sf_engine::config::MatchConfig;
use sf_engine::dramatic::{DramaticRules, LifebarMode};
use sf_engine::era::Revision;
use sf_engine::front::fight::FightDips;
use sf_engine::netplay::protocol::{tag_from_str, HandshakePolicy, LobbyPhase};
use sf_engine::netplay::{Link, NetPacket, RejectReason, RoomKind, SyncedMatchRules};
use sf_engine::roster::{PlayMode, StagePick, TournamentFighter};

fn dramatic_rules() -> SyncedMatchRules {
    use TournamentFighter::{Ken, Ryu, Sagat};
    let mut config = MatchConfig {
        dramatic: Some(DramaticRules {
            co_op: true,
            ally2: Ken,
            stage: Sagat,
            lifebar: LifebarMode::Independent,
            friendly_fire: true,
            styles: [Revision::Sf1Plus; 3],
            rivalry: Some(0),
        }),
        ..MatchConfig::definitive()
    };
    config.roster.mode = PlayMode::Versus;
    SyncedMatchRules::new(
        config,
        [Ryu, Sagat],
        FightDips::default(),
        StagePick::Bank(9),
        0x1122_3344_5566_7788,
        0x1020_3040,
        0x5060_7080,
    )
}

fn mixed_dramatic_rules() -> SyncedMatchRules {
    let mut rules = dramatic_rules();
    let dramatic = rules.config.dramatic.as_mut().unwrap();
    dramatic.rivalry = None;
    dramatic.styles = [Revision::Sf6, Revision::St, Revision::Sf3ThreeS];
    rules
}

#[test]
fn dramatic_rules_round_trip_the_native_roster_and_second_ally() {
    let rules = mixed_dramatic_rules();
    let bytes = rules.to_bytes();
    assert_eq!(bytes[0], 2, "Dramatic rules carry an extended schema");
    assert_eq!(SyncedMatchRules::from_bytes(&bytes), Some(rules));
    assert_eq!(rules.room_kind(), RoomKind::Dramatic);
    assert_eq!(
        rules.fighters,
        [TournamentFighter::Ryu, TournamentFighter::Sagat]
    );
    assert_eq!(
        rules.player_fighters(),
        [TournamentFighter::Ryu, TournamentFighter::Ken]
    );
}

#[test]
fn malformed_dramatic_rules_are_rejected() {
    let bytes = dramatic_rules().to_bytes();
    let payload = bytes.len() - 9;
    for index in payload..bytes.len() {
        let mut bad_tag = bytes.clone();
        bad_tag[index] = u8::MAX;
        assert_eq!(
            SyncedMatchRules::from_bytes(&bad_tag),
            None,
            "field {index}"
        );
        assert_eq!(
            SyncedMatchRules::from_bytes(&bytes[..index]),
            None,
            "cut at {index}"
        );
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(SyncedMatchRules::from_bytes(&trailing), None);
}

#[test]
fn dramatic_changes_are_included_in_the_advertised_config_hash() {
    let rules = dramatic_rules();
    let original = rules.config.dramatic.unwrap();
    let changes = [
        DramaticRules {
            co_op: false,
            ..original
        },
        DramaticRules {
            ally2: TournamentFighter::Adon,
            ..original
        },
        DramaticRules {
            stage: TournamentFighter::Ryu,
            ..original
        },
        DramaticRules {
            lifebar: LifebarMode::Shared,
            ..original
        },
        DramaticRules {
            friendly_fire: false,
            ..original
        },
        DramaticRules {
            styles: [Revision::Sf1Plus, Revision::Ce, Revision::Sf1Plus],
            ..original
        },
        DramaticRules {
            rivalry: None,
            ..original
        },
    ];
    for changed in changes {
        let mut alternate = rules;
        alternate.config.dramatic = Some(changed);
        assert_ne!(rules.config_hash(), alternate.config_hash(), "{changed:?}");
    }
}

#[test]
fn setup_packets_round_trip_the_host_draft_reference_and_ally_lock() {
    let rules = mixed_dramatic_rules();
    for packet in [
        NetPacket::DramaticSetup {
            generation: 9,
            seq: 12,
            rules,
        },
        NetPacket::DramaticAllyPick {
            generation: 9,
            setup_seq: 12,
            fighter: TournamentFighter::Ken,
            revision: Revision::Sf3ThreeS,
        },
    ] {
        assert_eq!(NetPacket::from_bytes(&packet.to_bytes()), Some(packet));
    }
    let mut versus = rules;
    versus.config.dramatic = None;
    let invalid = NetPacket::DramaticSetup {
        generation: 9,
        seq: 12,
        rules: versus,
    };
    assert_eq!(NetPacket::from_bytes(&invalid.to_bytes()), None);
}

fn policy(rules: SyncedMatchRules) -> HandshakePolicy {
    HandshakePolicy {
        rules,
        wired_only: true,
        host_tag: tag_from_str("HOST").unwrap(),
        max_rollback: 8,
        generation: 9,
        frame_base: 0,
        phase: LobbyPhase::Fighters,
        picker: 0,
        input_delay: None,
    }
}

fn request(kind: RoomKind, rules: SyncedMatchRules) -> NetPacket {
    NetPacket::Hello {
        request_id: 77,
        tag: tag_from_str("ALLY2").unwrap(),
        kind,
        link: Link::Ethernet,
        protocol_hash: rules.protocol_hash,
        config_hash: rules.config_hash(),
        bank_hash: rules.bank_hash,
        art_pack_hash: rules.art_pack_hash,
    }
}

#[test]
fn dramatic_version_rejection_uses_original_rules_schema() {
    let rules = mixed_dramatic_rules();
    let mut outdated = rules;
    outdated.protocol_hash ^= 1;
    let reply = policy(rules)
        .welcome(&request(RoomKind::Dramatic, outdated), 0, false)
        .unwrap();
    let bytes = reply.to_bytes();
    let Some(NetPacket::Welcome {
        accepted: false,
        reason,
        config,
        ..
    }) = NetPacket::from_bytes(&bytes)
    else {
        panic!("expected a readable rejected Welcome");
    };
    assert_eq!(reason, RejectReason::VersionMismatch);
    assert_eq!(config.to_bytes()[0], 1);
    assert_eq!(config.protocol_hash, rules.protocol_hash);
}

#[test]
fn dramatic_admission_rejects_versus_and_unsupported_era_rules() {
    let mut rules = dramatic_rules();
    let rejected = policy(rules).welcome(&request(RoomKind::Versus, rules), 0, false);
    assert!(matches!(
        rejected,
        Some(NetPacket::Welcome {
            accepted: false,
            reason: RejectReason::UnsupportedMode,
            ..
        })
    ));

    rules.config.dramatic.as_mut().unwrap().styles[2] = Revision::Ce;
    for role in [RoomKind::Dramatic, RoomKind::Spectate] {
        let rejected = policy(rules).welcome(&request(role, rules), 0, false);
        assert!(matches!(
            rejected,
            Some(NetPacket::Welcome {
                accepted: false,
                reason: RejectReason::UnsupportedMode,
                ..
            })
        ));
    }
}

#[test]
fn dramatic_admission_respects_the_host_what_if_rule() {
    let mut rules = mixed_dramatic_rules();
    rules.config.dramatic.as_mut().unwrap().styles[1] = Revision::Sf6;
    assert!(rules.supports_online(), "Sagat has a What-If SF6 style");
    rules.config.roster.what_if_styles = false;
    let reply = policy(rules).welcome(&request(RoomKind::Dramatic, rules), 0, false);
    assert!(matches!(reply, Some(NetPacket::Welcome {
        accepted: false, reason: RejectReason::UnsupportedMode, ..
    })));
}

#[test]
fn dramatic_admission_uses_host_rules_for_ally_and_spectator() {
    let rules = mixed_dramatic_rules();
    let mut preferred = rules;
    preferred.config.dramatic = None;
    for role in [RoomKind::Dramatic, RoomKind::Spectate] {
        let reply = policy(rules)
            .welcome(&request(role, preferred), 50, false)
            .unwrap();
        assert!(
            matches!(NetPacket::from_bytes(&reply.to_bytes()), Some(NetPacket::Welcome {
            accepted: true, config, assigned_delay: 1, ..
        }) if config == rules)
        );
    }
}
