//! #128, #170: the Netplay tab shows the selected relay, saves a custom one
//! and restores the default, `relay.almamoto.com`.

use sf_engine::config::MatchConfig;
use sf_engine::front::settings::{rows, Row, Settings, SettingsScreen, Tab};
use sf_engine::front::{Transition, TypedKey};
use sf_engine::roster::MenuInput;

fn on_relay_row(settings: Settings) -> SettingsScreen {
    let mut screen = SettingsScreen::open(MatchConfig::default(), settings);
    for _ in 0..5 {
        screen.apply(MenuInput::TabRight);
    }
    assert_eq!(screen.tab(), Tab::Netplay);
    let at = rows(Tab::Netplay).iter().position(|&row| row == Row::Relay).expect("a RELAY row");
    for _ in 0..at {
        screen.apply(MenuInput::Down);
    }
    assert_eq!(screen.row(), Row::Relay);
    assert_eq!(screen.label(Row::Relay), "RELAY");
    screen
}

fn type_text(screen: &mut SettingsScreen, text: &str) {
    for ch in text.chars() {
        assert_eq!(screen.type_key(TypedKey::Char(ch)), Some(Transition::CursorMoved), "{ch:?}");
    }
}

#[test]
fn with_no_custom_relay_the_row_shows_the_default_and_the_game_uses_it() {
    let screen = on_relay_row(Settings::default());
    assert_eq!(screen.value(Row::Relay), "RELAY.ALMAMOTO.COM");
    assert_eq!(screen.settings().relay_endpoint(), Some("relay.almamoto.com"));
}

#[test]
fn a_typed_host_with_an_optional_port_is_saved_and_used() {
    let mut screen = on_relay_row(Settings::default());
    assert_eq!(screen.apply(MenuInput::Confirm), Some(Transition::CursorMoved));
    assert!(screen.typing());
    assert_eq!(screen.value(Row::Relay), "", "a custom relay starts from an empty entry");
    type_text(&mut screen, "relay.fight-club.example:9000");
    assert_eq!(screen.type_key(TypedKey::Enter), Some(Transition::ValueChanged));
    assert!(!screen.typing());
    assert_eq!(screen.value(Row::Relay), "RELAY.FIGHT-CLUB.EXAMPLE:9000");
    let settings = screen.settings().clone();
    assert_eq!(settings.relay.as_deref(), Some("relay.fight-club.example:9000"));
    assert_eq!(settings.relay_endpoint(), Some("relay.fight-club.example:9000"));
    let reloaded = Settings::from_toml(&settings.to_toml()).expect("the saved file loads");
    assert_eq!(reloaded.relay_endpoint(), Some("relay.fight-club.example:9000"));
}

#[test]
fn a_malformed_relay_is_refused_with_a_reason_and_keeps_the_old_one() {
    let mut settings = Settings::default();
    settings.relay = Some("old.example.net".to_owned());
    let mut screen = on_relay_row(settings);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.value(Row::Relay), "OLD.EXAMPLE.NET", "editing starts from the custom relay");
    type_text(&mut screen, "..X:PORT");
    assert_eq!(screen.type_key(TypedKey::Enter), None);
    assert!(screen.typing(), "the entry stays open on a bad value");
    assert_eq!(screen.type_key(TypedKey::Char('/')), None, "a URL is not a relay host");
    screen.type_key(TypedKey::Escape);
    assert_eq!(screen.settings().relay.as_deref(), Some("old.example.net"));
}

#[test]
fn saving_an_empty_entry_or_resetting_the_tab_restores_the_default() {
    let mut settings = Settings::default();
    settings.relay = Some("ab.example".to_owned());
    let mut screen = on_relay_row(settings.clone());
    screen.apply(MenuInput::Confirm);
    for _ in 0.."ab.example".len() {
        screen.type_key(TypedKey::Erase);
    }
    assert_eq!(screen.type_key(TypedKey::Enter), Some(Transition::ValueChanged));
    assert_eq!(screen.settings().relay, None);
    assert_eq!(screen.value(Row::Relay), "RELAY.ALMAMOTO.COM");
    assert_eq!(screen.settings().relay_endpoint(), Some("relay.almamoto.com"));

    let mut screen = on_relay_row(settings);
    screen.apply(MenuInput::Down);
    assert_eq!(screen.row(), Row::ResetTab);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.settings().relay, None, "RESET TAB goes back to the default relay");
    assert_eq!(screen.value(Row::Relay), "RELAY.ALMAMOTO.COM");
    assert_eq!(screen.settings().relay_endpoint(), Some("relay.almamoto.com"));
}

#[test]
fn a_pad_can_enter_a_relay_and_a_long_one_stays_in_view_while_editing() {
    let mut screen = on_relay_row(Settings::default());
    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Up);
    assert_eq!(screen.value(Row::Relay), "A");
    screen.apply(MenuInput::Back);

    let long = "relay.a-very-long-community-name.tournament.example.net:17443";
    let mut settings = Settings::default();
    settings.relay = Some(long.to_owned());
    let mut screen = on_relay_row(settings);
    let cells = screen.value_cells(Row::Relay);
    assert!(long.len() > cells);
    screen.apply(MenuInput::Confirm);
    let shown = screen.value(Row::Relay);
    assert_eq!(shown.chars().count(), cells);
    assert!(shown.ends_with("NET:17443"), "the caret's end of the text is on screen: {shown}");
    screen.type_key(TypedKey::Left);
    let shown = screen.value(Row::Relay);
    assert_eq!(shown.chars().count(), cells);
    assert!(shown.ends_with("NET:17443"), "the character under the caret stays on screen: {shown}");
    screen.type_key(TypedKey::Home);
    assert!(screen.value(Row::Relay).starts_with("RELAY.A-VERY"), "and so is its start after HOME");
    assert_eq!(screen.type_key(TypedKey::Enter), Some(Transition::ValueChanged));
    assert_eq!(screen.settings().relay.as_deref(), Some(long));
    let shown = screen.value(Row::Relay);
    assert_eq!(shown.chars().count(), cells);
    assert!(shown.starts_with("RELAY.A-VERY") && shown.ends_with("..NET:17443"), "both ends show: {shown}");
}
