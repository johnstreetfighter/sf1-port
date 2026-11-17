//! Internet rooms in the online lobby (#124): what the menus ask the host
//! transport to do. The transport and relay have their own tests in sf-host
//! and sf-relay.
use sf_engine::front::online::{OnlineAction, OnlineScreen};
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, TypedKey};
use sf_engine::netplay::{Link, NetTransportCommand, NetTransportEvent, RoomKind};
use sf_engine::roster::MenuInput;

fn type_in_direct_join(screen: &mut OnlineScreen, text: &str) {
    screen.apply(MenuInput::Left);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm);
    for ch in text.chars() {
        screen.type_key(TypedKey::Char(ch));
    }
    screen.type_key(TypedKey::Enter);
}

#[test]
fn direct_join_reads_an_internet_room_code_in_any_case() {
    for text in ["k7q-mpx", "K7QMPX", "K7q-Mpx"] {
        let mut screen = OnlineScreen::open(false, 7000);
        type_in_direct_join(&mut screen, text);
        assert_eq!(
            screen.drain_actions(),
            [OnlineAction::JoinInternet { code: "K7Q-MPX".into(), kind: RoomKind::Versus }],
            "{text:?}"
        );
    }
    // The address forms keep their own join.
    for text in ["sf1-7f00-0001", "127.0.0.1:7000"] {
        let mut screen = OnlineScreen::open(false, 7000);
        type_in_direct_join(&mut screen, text);
        assert_eq!(
            screen.drain_actions(),
            [OnlineAction::Join { addr: "127.0.0.1:7000".into(), kind: RoomKind::Versus }],
            "{text:?}"
        );
    }
}

/// Opens Host Match, steps VISIBILITY right `turns` times, then opens the room.
fn open_room(relay: bool, turns: usize) -> Vec<OnlineAction> {
    let mut screen = OnlineScreen::open(false, 7000).with_relay(relay);
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Down);
    for _ in 0..turns {
        screen.apply(MenuInput::Right);
    }
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm);
    screen.drain_actions()
}

#[test]
fn host_match_offers_an_internet_room_only_when_a_relay_is_set() {
    let lan = |public| OnlineAction::Host { port: 7000, public, wired_only: false, kind: RoomKind::Versus };
    let internet = OnlineAction::HostInternet { wired_only: false, kind: RoomKind::Versus };
    // Without a relay the row keeps its two LAN choices.
    assert_eq!(open_room(false, 0), [lan(true)]);
    assert_eq!(open_room(false, 1), [lan(false)]);
    assert_eq!(open_room(false, 2), [lan(true)]);
    // With one, Internet comes first and the LAN choices follow.
    assert_eq!(open_room(true, 0), [internet.clone()]);
    assert_eq!(open_room(true, 1), [lan(true)]);
    assert_eq!(open_room(true, 2), [lan(false)]);
    assert_eq!(open_room(true, 3), [internet]);
}

fn pulse(session: &mut GameSession, key: MenuInput) {
    let mut input = FrameInputs::default();
    match key {
        MenuInput::Down => input.p1.directions[2] = true,
        MenuInput::Left => input.p1.directions[1] = true,
        MenuInput::Right => input.p1.directions[0] = true,
        MenuInput::Confirm => input.menu.confirm = true,
        MenuInput::Start => input.menu.start = true,
        _ => panic!("unsupported test key"),
    }
    session.step(input);
    session.step(FrameInputs::default());
}

fn online_session_with_relay() -> GameSession {
    online_session(Settings { relay: Some("relay.example.test".into()), ..Settings::default() })
}

fn online_session(settings: Settings) -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("online session checks require the verified local ROMs");
    let mut session = GameSession::new(
        assets,
        sf_engine::config::MatchConfig::from_preset(sf_engine::config::RulesetPreset::DefinitiveOverhaul),
    )
    .with_settings(settings);
    session.step(FrameInputs::default());
    for key in [MenuInput::Start, MenuInput::Down, MenuInput::Confirm, MenuInput::Down, MenuInput::Confirm] {
        pulse(&mut session, key);
    }
    assert_eq!(session.screen(), "online");
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::RefreshRooms]);
    session
}

#[test]
fn an_internet_host_keeps_one_relay_room_through_announcement_updates() {
    let mut session = online_session_with_relay();
    pulse(&mut session, MenuInput::Right);
    for _ in 0..4 {
        pulse(&mut session, MenuInput::Down);
    }
    pulse(&mut session, MenuInput::Confirm);
    let commands = session.drain_net_commands();
    let [NetTransportCommand::HostInternetRoom { room }] = commands.as_slice() else {
        panic!("expected one Internet room request, got {commands:?}");
    };
    assert!(!room.public, "Internet rooms never answer LAN discovery or the directory");
    assert_eq!(room.kind, RoomKind::Versus);
    assert_eq!(session.room_code_for_copy(), None, "no code before the relay is ready");

    session.push_net_events(&[NetTransportEvent::Bound {
        local_addr: "relay.example.test".into(),
        room_code: "K7Q-MPX".into(),
        link: Link::Ethernet,
    }]);
    session.step(FrameInputs::default());
    assert_eq!(session.room_code_for_copy(), Some("K7Q-MPX"));
    // The bound host re-announces its room; for a relay room that is a
    // metadata update of the same room, never a LAN bind.
    let updates = session.drain_net_commands();
    assert!(!updates.is_empty());
    assert!(
        updates.iter().all(|command| matches!(command, NetTransportCommand::HostInternetRoom { room } if !room.public)),
        "{updates:?}"
    );
}

/// #170: the shipped Default relay makes Internet rooms the first choice.
#[test]
fn a_fresh_install_hosts_an_internet_room_and_can_still_pick_a_lan_room() {
    let mut session = online_session(Settings::default());
    pulse(&mut session, MenuInput::Right);
    for _ in 0..4 {
        pulse(&mut session, MenuInput::Down);
    }
    pulse(&mut session, MenuInput::Confirm);
    let commands = session.drain_net_commands();
    assert!(matches!(commands.as_slice(), [NetTransportCommand::HostInternetRoom { .. }]), "{commands:?}");

    // One step right on VISIBILITY is PUBLIC, a room on the local network.
    let mut session = online_session(Settings::default());
    pulse(&mut session, MenuInput::Right);
    pulse(&mut session, MenuInput::Down);
    pulse(&mut session, MenuInput::Down);
    pulse(&mut session, MenuInput::Right);
    pulse(&mut session, MenuInput::Down);
    pulse(&mut session, MenuInput::Down);
    pulse(&mut session, MenuInput::Confirm);
    let commands = session.drain_net_commands();
    assert!(matches!(commands.as_slice(), [NetTransportCommand::BindHost { .. }]), "{commands:?}");
}

#[test]
fn an_internet_room_code_joins_through_the_relay() {
    let mut session = online_session_with_relay();
    pulse(&mut session, MenuInput::Left);
    pulse(&mut session, MenuInput::Down);
    session.paste_room_code(Some("k7q-mpx"));
    pulse(&mut session, MenuInput::Confirm);
    assert_eq!(
        session.drain_net_commands(),
        [NetTransportCommand::JoinInternetRoom { code: "K7Q-MPX".into() }]
    );
}

/// #171: the game font has capitals only, so a relay host name written in
/// lower case showed as a gap in the lobby message.
#[test]
fn a_relay_failure_names_the_relay_in_letters_the_font_can_draw() {
    let mut session = online_session(Settings::default());
    session.push_net_events(&[NetTransportEvent::TransportError {
        message: "RELAY relay.almamoto.com DID NOT ANSWER".into(),
    }]);
    session.step(FrameInputs::default());
    assert_eq!(session.notice(), Some("RELAY RELAY.ALMAMOTO.COM DID NOT ANSWER"));
}
