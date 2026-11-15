use sf_engine::netplay::protocol::{tag_from_str, tag_to_string, AfterMatchChoice, HandshakePolicy, NetPacket, RejectReason, RttEstimator, SyncedMatchRules, PLAYER_TAG_CHARS};
use sf_engine::netplay::{Link, RoomKind};
use sf_engine::netplay::bus::{PacketFault, VirtualNetBus, VirtualNetConfig};
use sf_engine::netplay::transport::{NetTransportCommand as Command, NetTransportEvent as Event, RoomAnnouncement};
use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{Costume, GaugeMode, Loadout, Revision, StylePick};
use sf_engine::front::fight::FightDips;
use sf_engine::roster::{BanFormat, PlayMode, StagePick, StartingCountry, TournamentFighter};
use sf_engine::session_io::ControlScheme;
use sf_engine::netplay::protocol::LobbyPhase;

#[test]
fn welcome_carries_the_authoritative_lobby_phase_and_stage_picker() {
    for (phase, picker, suffix) in [(LobbyPhase::Fighters, 0, [0, 0]), (LobbyPhase::Stage, 1, [1, 1])] {
        let config = rules();
        let packet = NetPacket::Welcome {
            request_id: 77, generation: 3, frame_base: 900, phase, picker,
            accepted: true, reason: RejectReason::None, config, seed: config.seed,
            assigned_delay: 2, max_rollback: 8, host_tag: tag_from_str("HOST").unwrap(),
        };
        let bytes = packet.to_bytes();
        assert_eq!(&bytes[bytes.len() - 6..bytes.len() - 4], &suffix,
            "phase and picker precede the monotonic frame base");
        assert_eq!(NetPacket::from_bytes(&bytes), Some(packet));
    }
}

#[test]
fn lobby_packets_distinguish_character_locks_and_random_stage_locks() {
    for (phase, random, suffix) in [
        (LobbyPhase::Fighters, false, [0, 0]),
        (LobbyPhase::Stage, true, [1, 1]),
    ] {
        let packet = NetPacket::LobbyPick {
            generation: 3, seq: 4, side: 0, fighter: TournamentFighter::Ryu,
            stage: StagePick::Bank(3), locked: true, phase, random, style: None,
        };
        let bytes = packet.to_bytes();
        assert_eq!(&bytes[bytes.len() - 2..], &suffix);
        assert_eq!(NetPacket::from_bytes(&bytes), Some(packet));
    }
}

#[test]
fn style_lock_and_committed_rules_round_trip_and_reject_invalid_choices() {
    let old = StylePick::sf2(Revision::St, Loadout::Old, Costume::Classic1987, 0);
    let packet = NetPacket::LobbyPick {
        generation: 3, seq: 4, side: 0, fighter: TournamentFighter::Ryu,
        stage: StagePick::Bank(0), locked: true, phase: LobbyPhase::Fighters,
        random: false, style: Some(old),
    };
    let wire = packet.to_bytes();
    assert_eq!(NetPacket::from_bytes(&wire), Some(packet));
    let mut invalid = wire.clone();
    *invalid.last_mut().unwrap() = 14; // V-Skill II / V-Trigger II cannot be an ST loadout.
    assert!(NetPacket::from_bytes(&invalid).is_none());
    assert!(NetPacket::from_bytes(&wire[..wire.len() - 1]).is_none());

    let mut rules = rules();
    let default_hash = rules.config_hash();
    rules.styles = [Some(old), Some(StylePick::default())];
    assert_eq!(SyncedMatchRules::from_bytes(&rules.to_bytes()), Some(rules));
    assert_ne!(rules.config_hash(), default_hash);
    rules.fighters[0] = TournamentFighter::Joe;
    rules.config.roster.p1 = TournamentFighter::Joe;
    rules.config.roster.what_if_styles = false;
    assert!(SyncedMatchRules::from_bytes(&rules.to_bytes()).is_none());
}

#[test]
fn welcome_identifies_the_monotonic_wire_frame_base_for_a_new_match() {
    let config = rules();
    let packet = NetPacket::Welcome {
        request_id: 77, generation: 3, frame_base: 0x0102_0304, phase: LobbyPhase::Fighters, picker: 0,
        accepted: true, reason: RejectReason::None, config, seed: config.seed,
        assigned_delay: 2, max_rollback: 8, host_tag: tag_from_str("HOST").unwrap(),
    };
    let bytes = packet.to_bytes();
    assert_eq!(&bytes[bytes.len() - 4..], &[4, 3, 2, 1]);
    assert_eq!(NetPacket::from_bytes(&bytes), Some(packet));
}

fn rules() -> SyncedMatchRules {
    SyncedMatchRules::new(
        MatchConfig::definitive(), [TournamentFighter::Ryu, TournamentFighter::Ken],
        FightDips::default(), StagePick::Bank(0), 42, 123, 456,
    )
}

#[test]
fn ping_uses_the_documented_header_and_stable_little_endian_fields() {
    let packet = NetPacket::Ping { seq: 0x0102_0304, sent_ms: 0x0506_0708, link: Link::Ethernet };
    let wire = [0x53, 0x46, 0x31, 0x4e, 1, 1, 4, 3, 2, 1, 8, 7, 6, 5, 0];
    assert_eq!(packet.to_bytes(), wire);
    assert_eq!(NetPacket::from_bytes(&wire), Some(packet));
    for end in 0..wire.len() {
        assert_eq!(NetPacket::from_bytes(&wire[..end]), None);
    }
    assert_eq!(NetPacket::from_bytes(&[wire.as_slice(), &[0]].concat()), None);
}

#[test]
fn synchronized_rules_carry_the_live_control_schemes() {
    let base = rules();
    let mut changed = base;
    changed.config.toggles.recovery_buffer_window += 1;
    assert_ne!(base.config_hash(), changed.config_hash());
    changed = base;
    changed.config.toggles.command_tracker_window += 1;
    assert_ne!(base.config_hash(), changed.config_hash());
    changed = base;
    changed.config.p1_scheme = ControlScheme::Modern;
    changed.config.p2_scheme = ControlScheme::Pneumatic;
    assert_ne!(base.config_hash(), changed.config_hash());
    let bytes = changed.to_bytes();
    assert_eq!(SyncedMatchRules::from_bytes(&bytes), Some(changed));
    for end in 0..bytes.len() {
        assert_eq!(SyncedMatchRules::from_bytes(&bytes[..end]), None);
    }
    assert_eq!(SyncedMatchRules::from_bytes(&[bytes.as_slice(), &[0]].concat()), None);
}

#[test]
fn player_tags_use_the_shared_twelve_character_protocol_alphabet() {
    assert_eq!(PLAYER_TAG_CHARS, " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_");
    let tag = *b"RYU87\0\0\0\0\0\0\0";
    assert_eq!(tag_from_str("RYU87"), Some(tag));
    assert_eq!(tag_to_string(tag), "RYU87");
    assert!(tag_from_str("A B-9_").is_some());
    assert!(tag_from_str("ABCDEFGHIJKL").is_some());
    for text in ["", "   ", "ryu", "A\0B", "ABCDEFGHIJKLM", "RYU!", "勇者"] {
        assert_eq!(tag_from_str(text), None);
    }
}

fn packets() -> Vec<NetPacket> {
    let rules = rules();
    let tag = tag_from_str("RYU_87").unwrap();
    let mut pairs = [(0, 0); 16];
    pairs[..2].copy_from_slice(&[(1, 2), (0x10, 0x20)]);
    vec![
        NetPacket::Ping { seq: 1, sent_ms: 2, link: Link::Wifi },
        NetPacket::Pong { seq: 1, echo_ms: 2, link: Link::Ethernet },
        NetPacket::Hello { request_id: 9, tag, kind: RoomKind::Versus, link: Link::Ethernet,
            protocol_hash: rules.protocol_hash, config_hash: rules.config_hash(), bank_hash: 123, art_pack_hash: 456 },
        NetPacket::Welcome { request_id: 9, generation: 3, frame_base: 0, phase: LobbyPhase::Fighters, picker: 0, accepted: true, reason: RejectReason::None,
            config: rules, seed: 42, assigned_delay: 2, max_rollback: 8, host_tag: tag },
        NetPacket::LobbyPick { generation: 3, seq: 4, side: 1, fighter: TournamentFighter::Sagat, stage: StagePick::Bank(9), locked: true,
            phase: LobbyPhase::Fighters, random: false, style: None },
        NetPacket::Input { current_frame: 0, start_frame: 4, count: 1,
            bits: [0x10, 0, 0, 0, 0, 0, 0, 0, 0], ack_frame: u32::MAX, frame_advantage: -2,
            checksum_frame: u32::MAX, checksum: 0 },
        NetPacket::SpectatorChunk {
            generation: 3,
            start_frame: 0,
            count: 2,
            pairs,
            theme: Some(sf_engine::front::fight::MatchTheme {
                id: sf_engine::theme::ThemeId::Akuma,
                phase: 1,
            }),
        },
        NetPacket::SpectatorAck { generation: 3, next_expected_frame: 2 },
        NetPacket::AfterMatchVote { generation: 3, seq: 4, choice: AfterMatchChoice::Reselect },
    ]
}

#[test]
fn every_packet_variant_has_a_stable_tag_and_rejects_noncanonical_buffers() {
    for (index, packet) in packets().into_iter().enumerate() {
        let bytes = packet.to_bytes();
        assert_eq!(&bytes[..6], &[0x53, 0x46, 0x31, 0x4e, 1, index as u8 + 1]);
        assert_eq!(NetPacket::from_bytes(&bytes), Some(packet), "tag {}", index + 1);
        for end in 0..bytes.len() { assert_eq!(NetPacket::from_bytes(&bytes[..end]), None); }
        assert_eq!(NetPacket::from_bytes(&[bytes.as_slice(), &[0]].concat()), None);
        for (at, bad) in [(0, 0), (4, 2), (5, 255)] {
            let mut invalid = bytes.clone(); invalid[at] = bad;
            assert_eq!(NetPacket::from_bytes(&invalid), None);
        }
    }
}

#[test]
fn redundant_inputs_keep_the_exact_fifty_byte_wire_layout() {
    let packet = NetPacket::Input { current_frame: 7, start_frame: 0, count: 9,
        bits: [1, 2, 4, 8, 16, 32, 64, 128, 256], ack_frame: u32::MAX,
        frame_advantage: -2, checksum_frame: u32::MAX, checksum: 0 };
    let bytes = [0x53, 0x46, 0x31, 0x4e, 1, 6, 7, 0, 0, 0, 0, 0, 0, 0, 9,
        1, 0, 2, 0, 4, 0, 8, 0, 16, 0, 32, 0, 64, 0, 128, 0, 0, 1,
        255, 255, 255, 255, 254, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(packet.to_bytes().len(), 50);
    assert_eq!(packet.to_bytes(), bytes);
    assert_eq!(NetPacket::from_bytes(&bytes), Some(packet));
    for count in [0, 10, 255] {
        let mut invalid = bytes; invalid[14] = count;
        assert_eq!(NetPacket::from_bytes(&invalid), None);
    }
    let mut overflow = bytes;
    overflow[10..14].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(NetPacket::from_bytes(&overflow), None);
    let mut reserved = bytes; reserved[16] = 4; // bit10, not a button or forfeit sentinel
    assert_eq!(NetPacket::from_bytes(&reserved), None);
}

#[test]
fn packet_fields_reject_invalid_ids_padding_counts_and_frame_ranges() {
    let list = packets();
    let mut invalid = list[0].to_bytes(); invalid[14] = 2;
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[2].to_bytes(); invalid[14] = b'a';
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[4].to_bytes(); invalid[14] = 2; // side
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[4].to_bytes(); invalid[15] = 12; // fighter
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[4].to_bytes(); invalid[17] = 2; // boolean
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[5].to_bytes(); invalid[17] = 1; // unused input word
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[5].to_bytes(); invalid[10..14].copy_from_slice(&5u32.to_le_bytes()); // future > delay4
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[6].to_bytes(); invalid[14] = 17;
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[6].to_bytes(); invalid[10..14].fill(255); // start frame overflow
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut invalid = list[8].to_bytes(); invalid[14] = 3;
    assert_eq!(NetPacket::from_bytes(&invalid), None);
}

fn announcement() -> RoomAnnouncement {
    let rules = rules();
    RoomAnnouncement { host: tag_from_str("HOST").unwrap(), kind: RoomKind::Versus,
        link: Link::Ethernet, wired_only: true, public: true, preset: rules.config.preset, in_match: false, spectators: 0, max_rollback: 8,
        config_hash: rules.config_hash(), bank_hash: rules.bank_hash, art_pack_hash: rules.art_pack_hash,
        protocol_hash: rules.protocol_hash }
}

#[test]
fn virtual_transport_reports_the_actual_link_instead_of_an_advertisement_claim() {
    for (actual, claimed) in [(Link::Ethernet, Link::Wifi), (Link::Wifi, Link::Ethernet)] {
        let mut bus = VirtualNetBus::new(VirtualNetConfig::default());
        bus.set_link(0, actual);
        let mut room = announcement();
        room.link = claimed;
        bus.submit(0, &[Command::BindHost { port: 7000, room }]);
        assert!(bus.drain_events(0).iter().any(|event| matches!(event,
            Event::Bound { link, .. } if *link == actual)), "the transport owns the bound link");
        bus.submit(1, &[Command::RefreshRooms]);
        let discovered = bus.drain_events(1);
        assert!(discovered.iter().any(|event| matches!(event,
            Event::RoomsDiscovered { rooms } if rooms.len() == 1 && rooms[0].link == actual)),
            "discovery uses the same transport truth");
    }
}

fn connected_bus(config: VirtualNetConfig) -> VirtualNetBus {
    let mut bus = VirtualNetBus::new(config);
    bus.submit(0, &[Command::BindHost { port: 7000, room: announcement() }]);
    bus.submit(1, &[Command::ConnectPeer { addr: "127.0.0.1:7000".into() }]);
    assert!(matches!(bus.drain_events(0).as_slice(), [Event::Bound { .. }]));
    assert!(matches!(bus.drain_events(1).as_slice(), [Event::Bound { .. }]));
    bus
}

#[test]
fn the_virtual_bus_delivers_real_packets_only_after_the_scripted_latency() {
    let mut bus = connected_bus(VirtualNetConfig { latency_ms: 10, ..VirtualNetConfig::default() });
    let packet = NetPacket::Ping { seq: 42, sent_ms: 0, link: Link::Ethernet };
    bus.submit(1, &[Command::SendPacket { to_peer: 0, bytes: packet.to_bytes() }]);
    bus.step_ms(9);
    assert!(bus.drain_events(0).is_empty());
    bus.step_ms(1);
    let events = bus.drain_events(0);
    let [Event::PacketReceived { from_peer, bytes, .. }] = events.as_slice() else { panic!("{events:?}") };
    assert_eq!(*from_peer, 1);
    assert_eq!(NetPacket::from_bytes(bytes), Some(packet));
    assert_eq!(bus.now_ms(), 10);
}

#[test]
fn closing_a_sender_preserves_dispatched_packets_but_clears_its_incoming_state() {
    let mut bus = connected_bus(VirtualNetConfig { latency_ms: 10, ..VirtualNetConfig::default() });
    let final_packet = NetPacket::Ping { seq: 99, sent_ms: 0, link: Link::Ethernet };
    bus.submit(0, &[Command::SendPacket { to_peer: 1, bytes: final_packet.to_bytes() }]);
    bus.submit(1, &[Command::SendPacket { to_peer: 0, bytes: final_packet.to_bytes() }, Command::Close]);
    bus.step_ms(10);
    assert_eq!(received_packets(&mut bus, 0), [final_packet]);
    assert!(bus.drain_events(1).is_empty(), "the closed destination drops its own pending input");
}

#[test]
fn host_handshake_adopts_host_rules_and_rejects_incompatible_peers_without_mutation() {
    let base = rules();
    let policy = HandshakePolicy { rules: base, wired_only: true, host_tag: tag_from_str("HOST").unwrap(),
        max_rollback: 8, generation: 3, frame_base: 0, phase: LobbyPhase::Fighters, picker: 0, input_delay: None };
    let request = |kind, link, protocol_hash, bank_hash, art_pack_hash| NetPacket::Hello {
        request_id: 77, tag: tag_from_str("CLIENT").unwrap(), kind, link, protocol_hash,
        config_hash: 999, bank_hash, art_pack_hash,
    };
    let hello = request(RoomKind::Versus, Link::Ethernet, base.protocol_hash, 123, 456);
    assert_eq!(policy.welcome(&hello, 68, false), Some(NetPacket::Welcome {
        request_id: 77, generation: 3, frame_base: 0, phase: LobbyPhase::Fighters, picker: 0, accepted: true, reason: RejectReason::None,
        config: base, seed: 42, assigned_delay: 2, max_rollback: 8, host_tag: policy.host_tag,
    }));
    for (hello, full, expected) in [
        (request(RoomKind::Versus, Link::Wifi, base.protocol_hash, 123, 456), false, RejectReason::WiredOnlyRequired),
        (request(RoomKind::Versus, Link::Ethernet, base.protocol_hash, 124, 456), false, RejectReason::PackHashMismatch),
        (request(RoomKind::Versus, Link::Ethernet, base.protocol_hash, 123, 457), false, RejectReason::PackHashMismatch),
        (request(RoomKind::Versus, Link::Ethernet, base.protocol_hash ^ 1, 123, 456), false, RejectReason::VersionMismatch),
        (request(RoomKind::Dramatic, Link::Ethernet, base.protocol_hash, 123, 456), false, RejectReason::UnsupportedMode),
        (request(RoomKind::Spectate, Link::Ethernet, base.protocol_hash, 123, 456), true, RejectReason::RoomFull),
    ] {
        let Some(NetPacket::Welcome { request_id: 77, accepted: false, reason, config, .. }) = policy.welcome(&hello, 0, full)
            else { panic!("missing explicit rejected Welcome") };
        assert_eq!(reason, expected);
        assert_eq!(config, base);
        assert_eq!(policy.rules, base);
    }
}

#[test]
fn round_trip_measurements_assign_delay_and_an_integer_jitter_average() {
    let mut timing = RttEstimator::default();
    assert!(timing.observe(100, 66));
    assert_eq!((timing.rtt_ms(), timing.jitter_ms(), timing.auto_delay()), (34, 0, 1));
    assert!(timing.observe(200, 98));
    assert_eq!((timing.rtt_ms(), timing.jitter_ms(), timing.auto_delay()), (102, 8, 3));
    assert!(!timing.observe(100, 101));
    assert_eq!(timing.rtt_ms(), 102);
}

#[test]
fn duplicated_rule_choices_have_one_authority_and_conflicting_wire_records_are_rejected() {
    let mut preferred = MatchConfig::definitive();
    preferred.roster.p1 = TournamentFighter::Geki;
    preferred.roster.p2 = TournamentFighter::Joe;
    preferred.roster.stage = Some(StagePick::Bank(9));
    preferred.difficulty = 4;
    preferred.round_time = 250;
    let normalized = SyncedMatchRules::new(preferred, [TournamentFighter::Ryu, TournamentFighter::Ken],
        FightDips::default(), StagePick::Bank(0), 42, 123, 456);
    assert_eq!([normalized.config.roster.p1, normalized.config.roster.p2], normalized.fighters);
    assert_eq!(normalized.config.difficulty, normalized.dips.difficulty);
    assert_eq!(normalized.config.round_time, normalized.dips.round_time);
    assert_eq!(normalized.config.roster.stage, Some(normalized.stage));
    assert_eq!(SyncedMatchRules::from_bytes(&normalized.to_bytes()), Some(normalized));
    for change in [
        (|r: &mut SyncedMatchRules| r.config.roster.p1 = TournamentFighter::Sagat) as fn(&mut SyncedMatchRules),
        |r| r.config.roster.p2 = TournamentFighter::Sagat,
        |r| r.config.difficulty = 3,
        |r| r.config.round_time = 200,
        |r| r.config.roster.stage = Some(StagePick::Bank(1)),
        |r| r.config.roster.stage = None,
    ] {
        let mut contradictory = normalized;
        change(&mut contradictory);
        assert_eq!(SyncedMatchRules::from_bytes(&contradictory.to_bytes()), None);
    }
    let welcome = NetPacket::Welcome { request_id: 1, generation: 1, frame_base: 0, phase: LobbyPhase::Fighters, picker: 0, accepted: true, reason: RejectReason::None,
        config: normalized, seed: 43, assigned_delay: 0, max_rollback: 8, host_tag: tag_from_str("HOST").unwrap() };
    assert_eq!(NetPacket::from_bytes(&welcome.to_bytes()), None);
}

fn received_packets(bus: &mut VirtualNetBus, peer: u8) -> Vec<NetPacket> {
    bus.drain_events(peer).into_iter().filter_map(|event| match event {
        Event::PacketReceived { bytes, .. } => NetPacket::from_bytes(&bytes), _ => None,
    }).collect()
}

#[test]
fn a_seventh_requester_gets_room_full_on_its_own_route_without_replacing_a_member() {
    let mut bus = connected_bus(VirtualNetConfig::default());
    assert_eq!(sf_engine::netplay::transport::MAX_ROUTES, 16);
    for route in 2..16 {
        bus.submit(route, &[Command::ConnectPeer { addr: "127.0.0.1:7000".into() }]);
        assert!(matches!(bus.drain_events(route).as_slice(), [Event::Bound { .. }]), "route {route}");
    }
    let base = rules();
    let hello = NetPacket::Hello { request_id: 71, tag: tag_from_str("VISITOR").unwrap(), kind: RoomKind::Spectate,
        link: Link::Ethernet, protocol_hash: base.protocol_hash, config_hash: 999, bank_hash: 123, art_pack_hash: 456 };
    bus.submit(6, &[Command::SendPacket { to_peer: 0, bytes: hello.to_bytes() }]);
    bus.step_ms(0);
    assert_eq!(received_packets(&mut bus, 0), [hello.clone()]);
    let policy = HandshakePolicy { rules: base, wired_only: true, host_tag: tag_from_str("HOST").unwrap(),
        max_rollback: 8, generation: 3, frame_base: 0, phase: LobbyPhase::Fighters, picker: 0, input_delay: None };
    let reply = policy.welcome(&hello, 0, true).unwrap();
    bus.submit(0, &[Command::SendPacket { to_peer: 6, bytes: reply.to_bytes() }]);
    let ping = NetPacket::Ping { seq: 99, sent_ms: 0, link: Link::Ethernet };
    bus.submit(0, &[Command::SendPacket { to_peer: 1, bytes: ping.to_bytes() }]);
    bus.step_ms(0);
    assert!(matches!(received_packets(&mut bus, 6).as_slice(), [NetPacket::Welcome { accepted: false, reason: RejectReason::RoomFull, .. }]));
    assert_eq!(received_packets(&mut bus, 1), [ping]);
}

#[test]
fn scripted_loss_duplication_and_reordering_have_exact_observable_delivery() {
    let mut bus = connected_bus(VirtualNetConfig::default());
    bus.set_fault_script(vec![
        PacketFault { delay_ms: 30, drop: false, copies: 1 },
        PacketFault { delay_ms: 0, drop: true, copies: 1 },
        PacketFault { delay_ms: 10, drop: false, copies: 2 },
        PacketFault { delay_ms: 0, drop: false, copies: 1 },
    ]);
    for seq in 1..=4 {
        bus.submit(1, &[Command::SendPacket { to_peer: 0,
            bytes: NetPacket::Ping { seq, sent_ms: 0, link: Link::Ethernet }.to_bytes() }]);
    }
    let sequences = |packets: Vec<NetPacket>| packets.into_iter().map(|p| match p {
        NetPacket::Ping { seq, .. } => seq, _ => panic!("not a Ping"),
    }).collect::<Vec<_>>();
    bus.step_ms(0);
    assert_eq!(sequences(received_packets(&mut bus, 0)), [4]);
    bus.step_ms(10);
    assert_eq!(sequences(received_packets(&mut bus, 0)), [3, 3]);
    bus.step_ms(20);
    assert_eq!(sequences(received_packets(&mut bus, 0)), [1]);
}

#[test]
fn seeded_network_conditions_reproduce_events_and_closing_a_destination_purges_old_datagrams() {
    let config = VirtualNetConfig { latency_ms: 30, jitter_ms: 20, drop_percent: 25,
        duplicate_percent: 50, reorder_ms: 40, seed: 1234 };
    let mut a = connected_bus(config);
    let mut b = connected_bus(config);
    for seq in 0..100 {
        let command = Command::SendPacket { to_peer: 0,
            bytes: NetPacket::Ping { seq, sent_ms: 0, link: Link::Ethernet }.to_bytes() };
        a.submit(1, &[command.clone()]); b.submit(1, &[command]);
    }
    for _ in 0..20 {
        a.step_ms(10); b.step_ms(10);
        assert_eq!(a.drain_events(0), b.drain_events(0));
    }
    let mut bus = connected_bus(VirtualNetConfig { latency_ms: 20, ..VirtualNetConfig::default() });
    bus.submit(1, &[Command::SendPacket { to_peer: 0,
        bytes: NetPacket::Ping { seq: 1, sent_ms: 0, link: Link::Ethernet }.to_bytes() }]);
    bus.submit(0, &[Command::Close, Command::BindHost { port: 7000, room: announcement() }]);
    bus.drain_events(0);
    bus.step_ms(30);
    assert!(received_packets(&mut bus, 0).is_empty());
}

#[test]
fn discovery_uses_session_metadata_and_same_port_updates_keep_existing_routes_alive() {
    let mut bus = connected_bus(VirtualNetConfig::default());
    let mut room = announcement(); room.in_match = true; room.spectators = 3; room.max_rollback = 4;
    bus.submit(0, &[Command::BindHost { port: 7000, room: room.clone() }]);
    assert!(bus.drain_events(0).is_empty());
    bus.submit(1, &[Command::RefreshRooms]);
    let events = bus.drain_events(1);
    let [Event::RoomsDiscovered { rooms }] = events.as_slice() else { panic!("{events:?}") };
    assert_eq!(rooms.len(), 1);
    assert_eq!((rooms[0].spectators, rooms[0].rollback_frames, rooms[0].in_match), (3, 4, true));
    let ping = NetPacket::Ping { seq: 4, sent_ms: 0, link: Link::Ethernet };
    bus.submit(1, &[Command::SendPacket { to_peer: 0, bytes: ping.to_bytes() }]); bus.step_ms(0);
    assert_eq!(received_packets(&mut bus, 0), [ping]);
    room.public = false;
    bus.submit(0, &[Command::BindHost { port: 7000, room }]);
    bus.submit(1, &[Command::RefreshRooms]);
    assert!(matches!(bus.drain_events(1).as_slice(), [Event::RoomsDiscovered { rooms }] if rooms.is_empty()));
}

#[test]
fn real_handshakes_cross_the_virtual_bus_and_measure_their_delay() {
    for (link, bank_hash, protocol_hash, expected) in [
        (Link::Ethernet, 123, rules().protocol_hash, RejectReason::None),
        (Link::Wifi, 123, rules().protocol_hash, RejectReason::WiredOnlyRequired),
        (Link::Ethernet, 124, rules().protocol_hash, RejectReason::PackHashMismatch),
        (Link::Ethernet, 123, rules().protocol_hash ^ 1, RejectReason::VersionMismatch),
    ] {
        let mut bus = connected_bus(VirtualNetConfig { latency_ms: 34, ..VirtualNetConfig::default() });
        let ping = NetPacket::Ping { seq: 1, sent_ms: 0, link };
        bus.submit(1, &[Command::SendPacket { to_peer: 0, bytes: ping.to_bytes() }]); bus.step_ms(34);
        assert_eq!(received_packets(&mut bus, 0), [ping]);
        bus.submit(0, &[Command::SendPacket { to_peer: 1,
            bytes: NetPacket::Pong { seq: 1, echo_ms: 0, link: Link::Ethernet }.to_bytes() }]); bus.step_ms(34);
        let replies = received_packets(&mut bus, 1);
        let [NetPacket::Pong { echo_ms, .. }] = replies.as_slice() else { panic!("{replies:?}") };
        let mut timing = RttEstimator::default(); assert!(timing.observe(bus.now_ms() as u32, *echo_ms));
        assert_eq!(timing.rtt_ms(), 68);
        let base = rules();
        let hello = NetPacket::Hello { request_id: 77, tag: tag_from_str("CLIENT").unwrap(), kind: RoomKind::Versus,
            link, protocol_hash, config_hash: 999, bank_hash, art_pack_hash: 456 };
        bus.submit(1, &[Command::SendPacket { to_peer: 0, bytes: hello.to_bytes() }]); bus.step_ms(34);
        let incoming = received_packets(&mut bus, 0);
        let policy = HandshakePolicy { rules: base, wired_only: true, host_tag: tag_from_str("HOST").unwrap(),
            max_rollback: 8, generation: 2, frame_base: 0, phase: LobbyPhase::Fighters, picker: 0, input_delay: None };
        let reply = policy.welcome(&incoming[0], timing.rtt_ms(), false).unwrap();
        bus.submit(0, &[Command::SendPacket { to_peer: 1, bytes: reply.to_bytes() }]); bus.step_ms(34);
        let replies = received_packets(&mut bus, 1);
        let [NetPacket::Welcome { reason, assigned_delay, config, .. }] = replies.as_slice() else { panic!("{replies:?}") };
        assert_eq!((*reason, *assigned_delay), (expected, 2));
        assert_eq!(*config, base);
    }
}

#[test]
fn every_current_gameplay_setting_and_asset_identity_contributes_to_the_rules_hash() {
    let changes: &[fn(&mut SyncedMatchRules)] = &[
        |r| r.config.preset = RulesetPreset::Custom,
        |r| r.config.toggles.positive_edge_attacks ^= true,
        |r| r.config.toggles.negative_edge_specials ^= true,
        |r| r.config.toggles.recovery_input_buffer ^= true,
        |r| r.config.toggles.recovery_buffer_window += 1,
        |r| r.config.toggles.lenient_command_trackers ^= true,
        |r| r.config.toggles.command_tracker_window += 1,
        |r| r.config.toggles.command_release_grace += 1,
        |r| r.config.toggles.interruptible_walk_cycle ^= true,
        |r| r.config.toggles.anytime_jump_attacks ^= true,
        |r| r.config.toggles.fast_prejump_crouch ^= true,
        |r| r.config.toggles.standard_high_low_block ^= true,
        |r| r.config.toggles.balanced_roster ^= true,
        |r| r.config.toggles.true_combos ^= true,
        |r| r.config.toggles.cancels ^= true,
        |r| r.config.toggles.throws ^= true,
        |r| r.config.toggles.chip_death ^= true,
        |r| r.config.toggles.disable_fireball_slowdown ^= true,
        |r| r.config.toggles.real_second_timer_and_draws ^= true,
        |r| r.config.toggles.fair_cpu_and_sagat_scaling ^= true,
        |r| r.config.toggles.cpu_reaction_delay += 1,
        |r| r.config.roster.full_cast ^= true,
        |r| r.config.roster.four_country ^= true,
        |r| r.config.roster.what_if_styles ^= true,
        |r| r.config.roster.bonus_challengers ^= true,
        |r| r.config.roster.secret_intrusions ^= true,
        |r| r.config.roster.ban = BanFormat::LowTierNoShoto,
        |r| r.config.roster.mode = PlayMode::Training,
        |r| { r.fighters[0] = TournamentFighter::Geki; r.config.roster.p1 = TournamentFighter::Geki; },
        |r| { r.fighters[1] = TournamentFighter::Joe; r.config.roster.p2 = TournamentFighter::Joe; },
        |r| r.config.roster.country = StartingCountry::China,
        |r| { r.stage = StagePick::Bank(9); r.config.roster.stage = Some(r.stage); },
        |r| r.config.fill_meters ^= true,
        |r| r.config.p2_fair_cpu ^= true,
        |r| r.config.modern_control_damage_penalty ^= true,
        |r| r.config.sf1plus.classic_1987_special_damage ^= true,
        |r| r.config.sf1plus.disable_freeze_input_lockout ^= true,
        |r| r.config.sf1plus.untechable_sf2_throws ^= true,
        |r| r.config.sf1plus.allow_corner_throw_loops ^= true,
        |r| r.config.gauge_mode = GaugeMode::NoGauges,
        |r| r.config.disable_comeback_scaling ^= true,
        |r| r.config.disable_raw_drive_rush ^= true,
        |r| r.config.tour.rpg ^= true,
        |r| r.config.tour.projectile_only ^= true,
        |r| r.config.tour.no_blocking ^= true,
        |r| r.config.tour.survival_waves = 1,
        |r| r.config.tour.time_attack_frames = 1,
        |r| r.config.tour.dramatic ^= true,
        |r| r.config.tour.cross_era ^= true,
        |r| { r.dips.difficulty = 3; r.config.difficulty = 3; },
        |r| { r.dips.round_time = 150; r.config.round_time = 150; },
        |r| r.seed ^= 1,
        |r| r.bank_hash ^= 1,
        |r| r.art_pack_hash ^= 1,
        |r| r.protocol_hash ^= 1,
    ];
    let base = rules();
    assert_eq!(base.protocol_hash, sf_engine::version::protocol_version_hash());
    for (index, change) in changes.iter().enumerate() {
        let mut changed = base;
        change(&mut changed);
        assert_ne!(base.config_hash(), changed.config_hash(), "setting {index}");
        assert_eq!(SyncedMatchRules::from_bytes(&changed.to_bytes()), Some(changed), "setting {index}");
    }
}

#[test]
fn percentage_fault_controls_actually_drop_duplicate_delay_and_reorder() {
    let packet = NetPacket::Ping { seq: 1, sent_ms: 0, link: Link::Ethernet };
    let mut drop = connected_bus(VirtualNetConfig { drop_percent: 100, ..VirtualNetConfig::default() });
    drop.submit(1, &[Command::SendPacket { to_peer: 0, bytes: packet.to_bytes() }]); drop.step_ms(1000);
    assert!(received_packets(&mut drop, 0).is_empty());
    let mut duplicate = connected_bus(VirtualNetConfig { duplicate_percent: 100, ..VirtualNetConfig::default() });
    duplicate.submit(1, &[Command::SendPacket { to_peer: 0, bytes: packet.to_bytes() }]); duplicate.step_ms(0);
    assert_eq!(received_packets(&mut duplicate, 0), [packet.clone(), packet]);
    for config in [
        VirtualNetConfig { jitter_ms: 20, seed: 123, ..VirtualNetConfig::default() },
        VirtualNetConfig { reorder_ms: 20, seed: 123, ..VirtualNetConfig::default() },
    ] {
        let mut bus = connected_bus(config);
        for seq in 0..100 {
            bus.submit(1, &[Command::SendPacket { to_peer: 0,
                bytes: NetPacket::Ping { seq, sent_ms: 0, link: Link::Ethernet }.to_bytes() }]);
        }
        bus.step_ms(0);
        let mut observed = received_packets(&mut bus, 0);
        assert!(observed.len() < 100, "jitter/reorder must delay some packets");
        bus.step_ms(20); observed.extend(received_packets(&mut bus, 0));
        let seqs: Vec<_> = observed.into_iter().map(|p| match p { NetPacket::Ping { seq, .. } => seq, _ => panic!() }).collect();
        assert_eq!(seqs.len(), 100);
        assert_ne!(seqs, (0..100).collect::<Vec<_>>());
    }
}

#[test]
fn delayed_frame_boundaries_padding_and_explicit_enum_ids_stay_canonical() {
    let mut packet = packets()[5].clone();
    if let NetPacket::Input { current_frame, start_frame, .. } = &mut packet {
        *current_frame = u32::MAX - 1; *start_frame = u32::MAX - 1;
    }
    assert_eq!(NetPacket::from_bytes(&packet.to_bytes()), Some(packet));
    for (reason, id) in [(RejectReason::WiredOnlyRequired, 1), (RejectReason::PackHashMismatch, 2),
        (RejectReason::RoomFull, 3), (RejectReason::VersionMismatch, 4), (RejectReason::UnsupportedMode, 5)]
    {
        let reply = NetPacket::Welcome { request_id: 1, generation: 2, frame_base: 0, phase: LobbyPhase::Fighters, picker: 0, accepted: false, reason,
            config: rules(), seed: 42, assigned_delay: 0, max_rollback: 8, host_tag: tag_from_str("HOST").unwrap() };
        let bytes = reply.to_bytes(); assert_eq!(bytes[19], id);
        assert_eq!(NetPacket::from_bytes(&bytes), Some(reply));
    }
    let ack = NetPacket::SpectatorAck { generation: 3, next_expected_frame: 2 };
    assert_eq!(ack.to_bytes(), [0x53, 0x46, 0x31, 0x4e, 1, 8, 3, 0, 0, 0, 2, 0, 0, 0]);
    let vote = NetPacket::AfterMatchVote { generation: 3, seq: 4, choice: AfterMatchChoice::Reselect };
    assert_eq!(vote.to_bytes(), [0x53, 0x46, 0x31, 0x4e, 1, 9, 3, 0, 0, 0, 4, 0, 0, 0, 1]);
    let mut invalid = packets()[6].to_bytes(); invalid[23] = 1; // first unused spectator pair
    assert_eq!(NetPacket::from_bytes(&invalid), None);
    let mut timing = RttEstimator::default();
    assert!(timing.observe(5, u32::MAX - 4));
    assert_eq!(timing.rtt_ms(), 10);
    let policy = HandshakePolicy { rules: rules(), wired_only: false, host_tag: tag_from_str("HOST").unwrap(),
        max_rollback: 4, generation: 2, frame_base: 0, phase: LobbyPhase::Fighters, picker: 0, input_delay: Some(3) };
    let hello = NetPacket::Hello { request_id: 1, tag: tag_from_str("WATCHER").unwrap(), kind: RoomKind::Spectate,
        link: Link::Wifi, protocol_hash: rules().protocol_hash, config_hash: 0, bank_hash: 123, art_pack_hash: 456 };
    assert!(matches!(policy.welcome(&hello, 0, false), Some(NetPacket::Welcome { accepted: true, assigned_delay: 3, max_rollback: 4, .. })));
}
