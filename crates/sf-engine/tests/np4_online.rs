use sf_engine::front::online::{LobbyView, OnlineAction, OnlineScreen};
use sf_engine::front::TypedKey;
use sf_engine::front::{FrameInputs, GameSession, Transition};
use sf_engine::roster::MenuInput;
use sf_engine::netplay::{Link, NetTransportCommand, NetTransportEvent, Room, RoomKind};
use sf_video::compositor::{ink, RomFont};
use sf_video::render::{Frame, HEIGHT, WIDTH};

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

fn online_session() -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
    ).expect("online session checks require the verified local ROMs");
    let mut session = GameSession::new(assets, sf_engine::config::MatchConfig::from_preset(
        sf_engine::config::RulesetPreset::DefinitiveOverhaul,
    ));
    session.step(FrameInputs::default());
    for key in [MenuInput::Start, MenuInput::Down, MenuInput::Confirm, MenuInput::Down, MenuInput::Confirm] {
        pulse(&mut session, key);
    }
    assert_eq!(session.screen(), "online");
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::RefreshRooms]);
    session
}

#[test]
fn browse_refresh_emits_an_action_and_routes_once_through_the_session_queue() {
    let mut screen = OnlineScreen::open(false, 7000);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Down);
    assert_eq!(screen.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(screen.drain_actions(), [OnlineAction::Refresh]);
    assert!(screen.drain_actions().is_empty());
    assert_eq!(screen.view(), LobbyView::Browse);

    let mut session = online_session();
    pulse(&mut session, MenuInput::Down);
    pulse(&mut session, MenuInput::Down);
    pulse(&mut session, MenuInput::Confirm);
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::RefreshRooms]);
    assert!(session.drain_net_commands().is_empty());
}

fn advertised_room(code: &str, kind: RoomKind, in_match: bool) -> Room {
    Room {
        code: code.into(), host: "PLAYER".into(), region: String::new(),
        ping_ms: 30, rollback_frames: 8, jitter_ms: 2, link: Link::Ethernet,
        kind, spectators: 0, what_if: false, bonus: false,
        preset: sf_engine::config::RulesetPreset::DefinitiveOverhaul, in_match,
    }
}

#[test]
fn browse_can_join_a_waiting_versus_room_as_a_spectator() {
    let mut screen = OnlineScreen::open(false, 7000);
    screen.set_rooms(vec![advertised_room("127.0.0.1:7000", RoomKind::Versus, false)]);
    for _ in 0..3 { screen.apply(MenuInput::Down); }
    screen.apply(MenuInput::Right); // JOIN AS: PLAYER -> SPECTATOR
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.drain_actions(), [OnlineAction::Join {
        addr: "127.0.0.1:7000".into(), kind: RoomKind::Spectate,
    }]);
}

#[test]
fn selected_rooms_join_their_parsed_endpoint_as_a_challenger_or_spectator() {
    let cases = [
        ("SF1-7F00-0001", RoomKind::Versus, false, "127.0.0.1:7000", RoomKind::Versus),
        ("127.0.0.2:7123", RoomKind::Versus, true, "127.0.0.2:7123", RoomKind::Spectate),
        ("SF1-C0A8-010A-1B59", RoomKind::Spectate, false, "192.168.1.10:7001", RoomKind::Spectate),
    ];
    for (selected, &(_, _, _, addr, kind)) in cases.iter().enumerate() {
        let mut screen = OnlineScreen::open(false, 7000);
        let rooms = cases.iter().map(|&(code, kind, in_match, _, _)| advertised_room(code, kind, in_match)).collect();
        screen.set_rooms(rooms);
        assert_eq!(screen.visible_rooms().len(), cases.len());
        for _ in 0..4 + selected { screen.apply(MenuInput::Down); }
        let confirm = if selected == 1 { MenuInput::Start } else { MenuInput::Confirm };
        assert_eq!(screen.apply(confirm), Some(Transition::ValueChanged));
        assert_eq!(screen.drain_actions(), [OnlineAction::Join { addr: addr.into(), kind }]);
        assert_eq!(screen.notice(), "CONNECTING...");
    }

    let mut invalid = OnlineScreen::open(false, 7000);
    invalid.set_rooms(vec![advertised_room("BAD ROOM CODE", RoomKind::Versus, false)]);
    for _ in 0..4 { invalid.apply(MenuInput::Down); }
    invalid.apply(MenuInput::Confirm);
    assert!(invalid.drain_actions().is_empty());
    assert!(invalid.notice().contains("INVALID"));
}

#[test]
fn browse_scrolls_to_the_selected_room_and_clamps_after_discovery_changes() {
    let mut screen = OnlineScreen::open(false, 7000);
    let rooms: Vec<_> = (1..=9).map(|index| {
        let mut room = advertised_room(&format!("127.0.0.{index}:7000"), RoomKind::Versus, false);
        room.host = format!("P_{index}");
        room
    }).collect();
    screen.set_rooms(rooms.clone());
    for _ in 0..12 { screen.apply(MenuInput::Down); }
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
    ).expect("browse scroll checks require the verified local ROMs");
    let font = sf_video::compositor::RomFont::new(&assets.gfx4, &assets.maincpu);
    let frame = screen.render(&font);
    for dy in 0..5 {
        for dx in 0..3 {
            let expected = if dy == 4 { 0xffff_d438 } else { 0xff00_0000 };
            assert_eq!(frame.argb[(160 + dy) * 384 + 98 + dx], expected,
                "the selected last row is visible, including its literal underscore");
        }
    }
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.drain_actions(), [OnlineAction::Join { addr: "127.0.0.9:7000".into(), kind: RoomKind::Versus }]);
    screen.set_rooms(rooms[..2].to_vec());
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.drain_actions(), [OnlineAction::Join { addr: "127.0.0.2:7000".into(), kind: RoomKind::Versus }]);
    screen.set_rooms(Vec::new());
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.drain_actions(), [OnlineAction::Refresh], "an emptied list clamps to the refresh row");
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Right);
    assert_eq!(screen.view(), LobbyView::Host, "navigation wraps back to the mode header");
}

fn text_at(frame: &Frame, font: &RomFont<'_>, col: usize, row: usize, text: &str, color: u8) -> bool {
    let mut expected = vec![0xff00_0000; WIDTH * HEIGHT];
    font.text(&mut expected, col, row, text, color);
    (row * 8..row * 8 + 8).all(|y| {
        let start = y * WIDTH + col * 8;
        let end = start + text.len() * 8;
        frame.argb[start..end] == expected[start..end]
    })
}

#[test]
fn a_host_bound_event_shows_the_shareable_code_and_waiting_status_but_client_bound_does_not() {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
    ).expect("bound-room render checks require the verified local ROMs");
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let code = "SF1-7F00-0001-1BD3";
    let mut screen = OnlineScreen::open(false, 7123);
    assert_eq!(screen.room_code(), None);
    screen.apply(MenuInput::Left);
    screen.set_bound(code);
    assert_eq!(screen.view(), LobbyView::Host);
    assert_eq!(screen.room_code(), Some(code));
    assert_eq!(screen.notice(), "WAITING FOR CHALLENGER...");
    let frame = screen.render(&font);
    assert!(text_at(&frame, &font, 5, 19, "ROOM CODE", ink::BLUE));
    assert!(text_at(&frame, &font, 20, 19, code, ink::YELLOW));
    assert!(text_at(&frame, &font, 3, 22, "WAITING FOR CHALLENGER...", ink::BLUE));
    assert!((200..208).all(|y| (16..368).all(|x| frame.argb[y * WIDTH + x] == 0xff00_0000)));

    let mut host = online_session();
    pulse(&mut host, MenuInput::Right);
    for _ in 0..4 { pulse(&mut host, MenuInput::Down); }
    pulse(&mut host, MenuInput::Confirm);
    assert!(matches!(host.drain_net_commands().as_slice(), [NetTransportCommand::BindHost { .. }]));
    host.push_net_events(&[NetTransportEvent::Bound {
        local_addr: "127.0.0.1:7123".into(), room_code: code.into(), link: Link::Ethernet,
    }]);
    let shown = host.step(FrameInputs::default());
    assert!(text_at(&shown.frame, &font, 20, 19, code, ink::YELLOW));
    assert!(text_at(&shown.frame, &font, 3, 22, "WAITING FOR CHALLENGER...", ink::BLUE));

    let mut client = online_session();
    pulse(&mut client, MenuInput::Left);
    pulse(&mut client, MenuInput::Down);
    pulse(&mut client, MenuInput::Confirm);
    client.type_keys("127.0.0.1:7000".chars().map(TypedKey::Char).chain([TypedKey::Enter]));
    let before = client.step(FrameInputs::default());
    client.push_net_events(&[NetTransportEvent::Bound {
        local_addr: "127.0.0.2:50000".into(), room_code: "SF1-7F00-0002-C350".into(), link: Link::Ethernet,
    }]);
    let after = client.step(FrameInputs::default());
    assert_eq!(after.frame.argb, before.frame.argb, "binding a client never advertises its local socket as a hosted room");
}

#[test]
fn the_mode_row_cycles_between_browse_host_and_direct_join() {
    let mut screen = OnlineScreen::open(false, 7000);
    assert_eq!(screen.view(), LobbyView::Browse);
    screen.apply(MenuInput::Right);
    assert_eq!(screen.view(), LobbyView::Host);
    screen.apply(MenuInput::Right);
    assert_eq!(screen.view(), LobbyView::Direct);
    screen.apply(MenuInput::Right);
    assert_eq!(screen.view(), LobbyView::Browse);
    screen.apply(MenuInput::Left);
    assert_eq!(screen.view(), LobbyView::Direct);
}

#[test]
fn direct_join_accepts_a_typed_room_code_or_ipv4_and_rejects_malformed_entry() {
    for text in ["SF1-7F00-0001", "127.0.0.1:7000"] {
        let mut screen = OnlineScreen::open(false, 7000);
        screen.apply(MenuInput::Left);
        screen.apply(MenuInput::Down);
        screen.apply(MenuInput::Confirm);
        assert!(screen.typing());
        for ch in text.chars() { screen.type_key(TypedKey::Char(ch)); }
        screen.type_key(TypedKey::Enter);
        assert_eq!(screen.drain_actions(), [OnlineAction::Join {
            addr: "127.0.0.1:7000".into(), kind: RoomKind::Versus,
        }]);
    }
    let mut screen = OnlineScreen::open(false, 7000);
    screen.apply(MenuInput::Left);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm);
    for ch in "999.0.0.1:0".chars() { screen.type_key(TypedKey::Char(ch)); }
    screen.type_key(TypedKey::Enter);
    assert!(screen.drain_actions().is_empty());
    assert!(screen.notice().contains("INVALID"));
}

#[test]
fn direct_join_can_request_the_spectator_slot_with_a_private_room_code() {
    let mut screen = OnlineScreen::open(false, 7000);
    screen.apply(MenuInput::Left);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Right); // JOIN AS: PLAYER -> SPECTATOR
    screen.apply(MenuInput::Up);
    screen.apply(MenuInput::Confirm);
    for ch in "SF1-7F00-0001".chars() { screen.type_key(TypedKey::Char(ch)); }
    screen.type_key(TypedKey::Enter);
    assert_eq!(screen.drain_actions(), [OnlineAction::Join {
        addr: "127.0.0.1:7000".into(), kind: RoomKind::Spectate,
    }]);
}

#[test]
fn wired_only_immediately_filters_wifi_rooms() {
    let mut screen = OnlineScreen::open(false, 7000);
    let room = |host: &str, link| Room {
        code: "SF1-7F00-0001".into(), host: host.into(), region: String::new(),
        ping_ms: 30, rollback_frames: 8, jitter_ms: 2, link,
        kind: RoomKind::Versus, spectators: 0, what_if: false, bonus: false,
        preset: sf_engine::config::RulesetPreset::DefinitiveOverhaul, in_match: false,
    };
    screen.set_rooms(vec![room("WIRED", Link::Ethernet), room("WIRELESS", Link::Wifi)]);
    assert_eq!(screen.visible_rooms().len(), 2);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.visible_rooms().iter().map(|r| r.host.as_str()).collect::<Vec<_>>(), ["WIRED"]);
    screen.apply(MenuInput::Left);
    assert_eq!(screen.visible_rooms().len(), 2);
    assert_eq!(screen.view(), LobbyView::Browse, "changing a filter stays in the browse view");
}

#[test]
fn direct_join_can_be_entered_with_only_the_arcade_pad() {
    let mut screen = OnlineScreen::open(false, 7000);
    screen.apply(MenuInput::Left);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm);
    for (at, wanted) in "127.0.0.1:7000".chars().enumerate() {
        for _ in 0..45 {
            if screen.entry().chars().nth(at) == Some(wanted) { break; }
            screen.apply(MenuInput::Up);
        }
        assert_eq!(screen.entry().chars().nth(at), Some(wanted), "the pad can choose {wanted}");
        screen.apply(MenuInput::Right);
    }
    screen.apply(MenuInput::Confirm);
    assert!(!screen.typing(), "confirm finishes the text edit");
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.drain_actions(), [OnlineAction::Join {
        addr: "127.0.0.1:7000".into(), kind: RoomKind::Versus,
    }]);
}

#[test]
fn hosting_emits_the_current_port_visibility_and_wired_requirement() {
    let mut screen = OnlineScreen::open(false, 7123);
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Right); // public -> private
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm); // wired only
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.drain_actions(), [OnlineAction::Host {
        port: 7123, public: false, wired_only: true, kind: RoomKind::Versus,
    }]);
    assert_eq!(screen.view(), LobbyView::Host);
}

#[test]
fn dramatic_rooms_can_be_hosted_and_joined_from_each_lobby_view() {
    let mut host = OnlineScreen::open(false, 7123);
    for input in [
        MenuInput::Right, MenuInput::Down, MenuInput::Right, MenuInput::Down,
        MenuInput::Down, MenuInput::Down, MenuInput::Confirm,
    ] {
        host.apply(input);
    }
    assert_eq!(host.drain_actions(), [OnlineAction::Host {
        port: 7123, public: true, wired_only: false, kind: RoomKind::Dramatic,
    }]);

    for (in_match, expected_kind) in [(false, RoomKind::Dramatic), (true, RoomKind::Spectate)] {
        let mut browser = OnlineScreen::open(false, 7000);
        browser.set_rooms(vec![advertised_room("127.0.0.1:7123", RoomKind::Dramatic, in_match)]);
        for _ in 0..4 {
            browser.apply(MenuInput::Down);
        }
        browser.apply(MenuInput::Confirm);
        assert_eq!(browser.drain_actions(), [OnlineAction::Join {
            addr: "127.0.0.1:7123".into(), kind: expected_kind,
        }]);
    }

    let mut direct = OnlineScreen::open(false, 7000);
    for input in [
        MenuInput::Left, MenuInput::Down, MenuInput::Down, MenuInput::Left,
        MenuInput::Up, MenuInput::Confirm,
    ] {
        direct.apply(input);
    }
    for ch in "127.0.0.1:7123".chars() {
        direct.type_key(TypedKey::Char(ch));
    }
    direct.type_key(TypedKey::Enter);
    assert_eq!(direct.drain_actions(), [OnlineAction::Join {
        addr: "127.0.0.1:7123".into(), kind: RoomKind::Dramatic,
    }]);
}

#[test]
fn each_lobby_view_renders_inside_the_wide_frame_with_an_empty_foot_row() {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
    ).expect("lobby render checks require the verified local ROMs");
    let font = sf_video::compositor::RomFont::new(&assets.gfx4, &assets.maincpu);
    let mut screen = OnlineScreen::open(false, 7000);
    let mut rendered = Vec::new();
    for _ in 0..3 {
        let frame = screen.render(&font);
        if matches!(screen.view(), LobbyView::Browse | LobbyView::Direct) {
            let row = if screen.view() == LobbyView::Browse { 9 } else { 13 };
            assert!(text_at(&frame, &font, 5, row, "JOIN AS", ink::BLUE));
            assert!(text_at(&frame, &font, 20, row, "PLAYER", ink::BLUE));
        }
        assert_eq!(frame.argb.len(), 384 * 224);
        assert!(frame.argb.iter().filter(|&&pixel| pixel != 0xff00_0000).count() > 1_000);
        assert!((200..208).all(|y| (16..368).all(|x| frame.argb[y * 384 + x] == 0xff00_0000)),
            "the foot stays empty in {:?}", screen.view());
        rendered.push(frame.argb);
        screen.apply(MenuInput::Right);
    }
    assert!(rendered[0] != rendered[1] && rendered[1] != rendered[2]);
}
