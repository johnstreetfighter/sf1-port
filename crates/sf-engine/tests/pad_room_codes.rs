//! Room codes entered with a pad alone (#129): what a Steam Deck player's
//! built-in controls ask the host transport to do, through `GameSession`.
use sf_engine::front::online::{OnlineScreen, NET_ENTRY_CHARS};
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, MenuKeys};
use sf_engine::netplay::{Link, NetTransportCommand, NetTransportEvent};
use sf_engine::roster::MenuInput;

/// One frame of Player 1's pad with `key` held.
fn held(key: Option<MenuInput>) -> FrameInputs {
    let mut input = FrameInputs::default();
    let mut pad = MenuKeys::default();
    match key {
        Some(MenuInput::Up) => input.p1.directions[3] = true,
        Some(MenuInput::Down) => input.p1.directions[2] = true,
        Some(MenuInput::Left) => input.p1.directions[1] = true,
        Some(MenuInput::Right) => input.p1.directions[0] = true,
        Some(MenuInput::Confirm) => pad.confirm = true,
        Some(MenuInput::Start) => pad.start = true,
        Some(MenuInput::Back) => pad.back = true,
        Some(_) => panic!("unsupported test key"),
        None => {}
    }
    input.pads[0] = Some(pad);
    input
}

/// Press and release `key` on the pad.
fn press(session: &mut GameSession, key: MenuInput) {
    session.step(held(Some(key)));
    session.step(held(None));
}

fn presses(session: &mut GameSession, keys: &[MenuInput]) {
    for &key in keys {
        press(session, key);
    }
}

/// The online lobby with a relay set, reached with the pad.
fn lobby() -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("online session checks require the verified local ROMs");
    let mut session = GameSession::new(
        assets,
        sf_engine::config::MatchConfig::from_preset(sf_engine::config::RulesetPreset::DefinitiveOverhaul),
    )
    .with_settings(Settings { relay: Some("relay.example.test".into()), ..Settings::default() });
    session.step(held(None));
    presses(&mut session, &[MenuInput::Start, MenuInput::Down, MenuInput::Confirm, MenuInput::Down, MenuInput::Confirm]);
    assert_eq!(session.screen(), "online");
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::RefreshRooms]);
    session
}

/// Direct Join's code row, open for entry.
fn code_row() -> GameSession {
    let mut session = lobby();
    presses(&mut session, &[MenuInput::Left, MenuInput::Down, MenuInput::Confirm]);
    session
}

use MenuInput::{Back, Confirm, Down, Left, Right, Start, Up};

#[test]
fn a_pad_enters_a_room_code_and_joins() {
    let mut session = code_row();
    // Up on an empty place gives 2, the first room-code symbol. Each new
    // place starts on the symbol before it, so one step up from 2 is 3.
    presses(&mut session, &[Up, Right, Up, Right, Up, Right, Down, Right, Up, Right, Up]);
    // Confirm ends the entry on the JOIN row; Confirm again joins.
    presses(&mut session, &[Confirm, Confirm]);
    assert_eq!(
        session.drain_net_commands(),
        [NetTransportCommand::JoinInternetRoom { code: "234-345".into() }]
    );
}

#[test]
fn a_pad_corrects_a_symbol_and_erases_one() {
    let mut session = code_row();
    // 2 3 4 4 5 6, with the fourth place wrong.
    presses(&mut session, &[Up, Right, Up, Right, Up, Right, Up, Down, Right, Up, Right, Up]);
    // Back two places to the wrong 4 and step it down to 3.
    presses(&mut session, &[Left, Left, Down]);
    // To the end, add a seventh symbol by mistake, and erase it with Start
    // while it is still under the cursor.
    presses(&mut session, &[Right, Right, Right, Up, Start]);
    presses(&mut session, &[Confirm, Confirm]);
    assert_eq!(
        session.drain_net_commands(),
        [NetTransportCommand::JoinInternetRoom { code: "234-356".into() }]
    );
}

#[test]
fn back_cancels_a_pad_entry_and_then_leaves_the_lobby_without_joining() {
    let mut session = code_row();
    presses(&mut session, &[Up, Right, Up]);
    press(&mut session, Back);
    assert_eq!(session.screen(), "online", "the first Back only ends the entry");
    press(&mut session, Back);
    assert_ne!(session.screen(), "online");
    // Leaving the lobby closes the transport; nothing was joined.
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::Close]);
}

#[test]
fn start_past_the_end_erases_the_last_symbol() {
    let mut session = code_row();
    // 2 3 4 3 4 5 and a seventh symbol, with the cursor moved past it.
    presses(&mut session, &[Up, Right, Up, Right, Up, Right, Down, Right, Up, Right, Up, Right, Up, Right]);
    presses(&mut session, &[Start, Confirm, Confirm]);
    assert_eq!(
        session.drain_net_commands(),
        [NetTransportCommand::JoinInternetRoom { code: "234-345".into() }]
    );
}

#[test]
fn a_pad_opens_an_internet_room_shows_its_code_and_closes_it_with_back() {
    let mut session = lobby();
    // Right to HOST MATCH, down to OPEN ROOM, confirm.
    presses(&mut session, &[Right, Down, Down, Down, Down, Confirm]);
    let commands = session.drain_net_commands();
    assert!(matches!(commands.as_slice(), [NetTransportCommand::HostInternetRoom { .. }]), "{commands:?}");
    session.push_net_events(&[NetTransportEvent::Bound {
        local_addr: "relay.example.test".into(),
        room_code: "K7Q-MPX".into(),
        link: Link::Ethernet,
    }]);
    session.step(held(None));
    assert_eq!(session.room_code_for_copy(), Some("K7Q-MPX"), "the code the host screen shows");
    session.drain_net_commands();

    press(&mut session, Back);
    assert_eq!(session.screen(), "online");
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::Close]);
    assert_eq!(session.room_code_for_copy(), None);
}

#[test]
fn back_cancels_a_pending_join_made_with_the_pad() {
    let mut session = code_row();
    presses(&mut session, &[Up, Right, Up, Right, Up, Right, Down, Right, Up, Right, Up, Confirm, Confirm]);
    session.drain_net_commands();
    press(&mut session, Back);
    assert_eq!(session.screen(), "online", "the lobby stays open");
    assert_eq!(session.drain_net_commands(), [NetTransportCommand::Close]);
}

#[test]
fn holding_up_keeps_stepping_through_the_symbols() {
    let mut session = code_row();
    // Held for 31 frames: the press, then a repeat on the 20th, 25th and
    // 30th held frame. Four steps from an empty place reach 5.
    for _ in 0..31 {
        session.step(held(Some(Up)));
    }
    session.step(held(None));
    presses(&mut session, &[Right, Up, Right, Up, Right, Up, Right, Up, Right, Up]);
    presses(&mut session, &[Confirm, Confirm]);
    assert_eq!(
        session.drain_net_commands(),
        [NetTransportCommand::JoinInternetRoom { code: "567-89A".into() }]
    );
}

#[test]
fn pad_entry_offers_room_code_symbols_first_and_every_address_character_after() {
    let mut screen = OnlineScreen::open(false, 7000);
    for key in [Left, Down, Confirm] {
        screen.apply(key);
    }
    let mut offered = String::new();
    for _ in 0..NET_ENTRY_CHARS.len() {
        screen.apply(Up);
        offered.push_str(screen.entry());
    }
    assert!(offered.starts_with("23456789ABCDEFGHJKMNPQRSTUVWXYZ"), "{offered}");
    let mut rest: Vec<char> = offered[31..].chars().collect();
    rest.sort_unstable();
    assert_eq!(rest.into_iter().collect::<String>(), " -.01:ILO_");
}

#[test]
fn down_on_an_empty_first_place_gives_the_last_room_code_symbol() {
    let mut screen = OnlineScreen::open(false, 7000);
    for key in [Left, Down, Confirm, Down] {
        screen.apply(key);
    }
    assert_eq!(screen.entry(), "Z", "not the blank at the end of the address characters");
}
