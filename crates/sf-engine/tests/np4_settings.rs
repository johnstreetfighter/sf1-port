//! NP4 (#307): Netplay settings through the public file and screen seams.

use sf_engine::config::MatchConfig;
use sf_engine::front::settings::{rows, InputDelaySetting, NetplayHudMode, Row, Settings, SettingsScreen, Tab, FIRST_ROW, TAB_ROW};
use sf_engine::front::{Transition, TypedKey};
use sf_engine::roster::MenuInput;
use sf_engine::run::ScratchDir;
use sf_video::compositor::{ink, RomFont, TEXT_PALETTE_ROM};
use sf_video::render::WIDTH;

fn font_palette() -> Vec<u8> {
    let mut program = vec![0; TEXT_PALETTE_ROM + 128];
    let yellow = TEXT_PALETTE_ROM + usize::from(ink::YELLOW) * 8;
    program[yellow..yellow + 2].copy_from_slice(&0x0ff0_u16.to_be_bytes());
    program
}

#[test]
fn old_settings_get_netplay_defaults_and_new_settings_round_trip_through_the_file() {
    let mut settings = Settings::from_toml("scale = 2\nmaster = 7\n")
        .expect("an older settings file still loads");
    assert_eq!(settings.player_tag, "PLAYER");
    assert_eq!(settings.netplay_input_delay, InputDelaySetting::Auto);
    assert_eq!(settings.netplay_input_delay.frames(), None);
    assert_eq!(settings.netplay_max_rollback, 8);
    assert!(!settings.wired_only);
    assert_eq!(settings.netplay_hud, NetplayHudMode::Compact);
    assert_eq!(settings.netplay_port, 7000);
    assert_eq!(settings.lobby_url, None);

    settings.player_tag = "RYU_87".to_owned();
    settings.netplay_input_delay = InputDelaySetting::Frames(3);
    settings.netplay_max_rollback = 5;
    settings.wired_only = true;
    settings.netplay_hud = NetplayHudMode::Full;
    settings.netplay_port = 9000;
    settings.lobby_url = Some("https://rooms.example.test/sf1".to_owned());
    assert_eq!(settings.netplay_input_delay.frames(), Some(3));
    assert_eq!(Settings::from_toml(&settings.to_toml()).unwrap(), settings);

    let dir = ScratchDir::new("np4-settings");
    let path = dir.join("settings.toml");
    settings.save(&path).expect("save Netplay and existing settings");
    let (loaded, notice) = Settings::load_or_default(&path);
    assert_eq!(notice, None);
    assert_eq!(loaded, settings);
    assert_eq!(loaded.scale, 2, "Netplay persistence preserves old fields");
    assert_eq!(loaded.master, 7);
}

#[test]
fn netplay_file_values_enforce_tag_alphabet_and_numeric_boundaries() {
    for line in [
        "player_tag = \"\"",
        "player_tag = \"   \"",
        "player_tag = \"lowercase\"",
        "player_tag = \"RYU:87\"",
        "player_tag = \"RYUé\"",
        "player_tag = \"THIRTEENCHARS\"",
        "netplay_input_delay = 5",
        "netplay_input_delay = -1",
        "netplay_max_rollback = 0",
        "netplay_max_rollback = 9",
        "netplay_hud = \"unknown\"",
        "netplay_port = 1023",
        "netplay_port = 65536",
    ] {
        assert!(Settings::from_toml(line).is_err(), "invalid Netplay value must be refused: {line}");
    }
    for (delay, rollback, port) in [(0, 1, 1024), (4, 8, 65535)] {
        let file = format!(
            "player_tag = \"A B_C-D01234\"\nnetplay_input_delay = {delay}\nnetplay_max_rollback = {rollback}\nnetplay_port = {port}\n"
        );
        let settings = Settings::from_toml(&file).expect("the documented boundaries are valid");
        assert_eq!(settings.netplay_input_delay.frames(), Some(delay));
        assert_eq!(settings.netplay_max_rollback, rollback);
        assert_eq!(settings.netplay_port, port);
        assert_eq!(Settings::from_toml(&settings.to_toml()).unwrap(), settings);
    }
}

#[test]
fn sixth_tab_edits_network_choices_and_resets_only_its_own_settings() {
    let config = MatchConfig::default();
    let mut settings = Settings::default();
    settings.master = 7;
    settings.lobby_url = Some("https://rooms.example.test".to_owned());
    let mut screen = SettingsScreen::open(config, settings);
    for _ in 0..5 {
        screen.apply(MenuInput::TabRight);
    }
    assert_eq!(screen.tab(), Tab::Netplay);
    let labels: Vec<_> = rows(Tab::Netplay).into_iter().map(|row| screen.label(row)).collect();
    assert_eq!(labels, ["PLAYER TAG", "INPUT DELAY", "MAX ROLLBACK", "WIRED ONLY", "NETPLAY HUD", "UDP PORT", "RESET TAB"]);
    assert_eq!(screen.row(), Row::PlayerTag);
    assert_eq!(screen.value(screen.row()), "PLAYER");
    let tiles = vec![0xff; 0x4000];
    let program = font_palette();
    let font = RomFont::new(&tiles, &program);
    assert_eq!(font.pen(ink::YELLOW, 0), 0xffff_ff00);
    let frame = screen.render(&font);
    let underline = (TAB_ROW * 8 + 9) * WIDTH;
    assert!(frame.argb[underline + 39 * 8..underline + 46 * 8].iter().all(|&pixel| pixel == 0xffff_ff00), "NETPLAY is visibly selected instead of dimmed");

    screen.apply(MenuInput::Down);
    assert_eq!(screen.value(screen.row()), "AUTO");
    let mut delays = Vec::new();
    for _ in 0..6 {
        assert_eq!(screen.apply(MenuInput::Right), Some(Transition::ValueChanged));
        delays.push(screen.value(screen.row()));
    }
    assert_eq!(delays, ["0F", "1F", "2F", "3F", "4F", "AUTO"]);
    screen.apply(MenuInput::Left);
    assert_eq!(screen.settings().netplay_input_delay, InputDelaySetting::Frames(4));

    screen.apply(MenuInput::Down);
    for _ in 0..20 {
        screen.apply(MenuInput::Left);
    }
    assert_eq!(screen.settings().netplay_max_rollback, 1);
    screen.apply(MenuInput::Right);
    assert_eq!(screen.value(screen.row()), "2F");
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Confirm);
    assert!(screen.settings().wired_only);
    screen.apply(MenuInput::Down);
    let mut modes = Vec::new();
    for _ in 0..3 {
        screen.apply(MenuInput::Right);
        modes.push(screen.value(screen.row()));
    }
    assert_eq!(modes, ["FULL", "OFF", "COMPACT"]);
    screen.apply(MenuInput::Left);
    assert_eq!(screen.settings().netplay_hud, NetplayHudMode::Off);
    screen.apply(MenuInput::Down);
    screen.apply(MenuInput::Right);
    assert_eq!(screen.settings().netplay_port, 7001);

    screen.apply(MenuInput::Down);
    assert_eq!(screen.row(), Row::ResetTab);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.settings().player_tag, "PLAYER");
    assert_eq!(screen.settings().netplay_input_delay, InputDelaySetting::Auto);
    assert_eq!(screen.settings().netplay_max_rollback, 8);
    assert!(!screen.settings().wired_only);
    assert_eq!(screen.settings().netplay_hud, NetplayHudMode::Compact);
    assert_eq!(screen.settings().netplay_port, 7000);
    assert_eq!(screen.settings().lobby_url, None);
    assert_eq!(screen.settings().master, 7, "RESET TAB preserves Sound settings");
    assert_eq!(*screen.config(), config, "Netplay never edits gameplay rules");
    screen.apply(MenuInput::TabRight);
    assert_eq!(screen.tab(), Tab::Gameplay);
    screen.apply(MenuInput::TabLeft);
    assert_eq!(screen.tab(), Tab::Netplay);
    assert_eq!(screen.row(), Row::ResetTab, "the sixth tab remembers its own cursor");
}

fn netplay(settings: Settings) -> SettingsScreen {
    let mut screen = SettingsScreen::open(MatchConfig::default(), settings);
    for _ in 0..5 {
        screen.apply(MenuInput::TabRight);
    }
    screen
}

fn clear_entry(screen: &mut SettingsScreen) {
    screen.type_key(TypedKey::Home);
    for _ in 0..12 {
        screen.type_key(TypedKey::EraseAhead);
    }
    assert_eq!(screen.value(screen.row()), "");
}

#[test]
fn player_tag_entry_accepts_keyboard_and_pad_input_without_saving_cancelled_text() {
    let mut screen = netplay(Settings::default());
    let tiles = vec![0xff; 0x4000];
    let program = font_palette();
    let font = RomFont::new(&tiles, &program);
    let before = screen.render(&font);
    assert!(!screen.typing());
    assert_eq!(screen.apply(MenuInput::Confirm), Some(Transition::CursorMoved));
    assert!(screen.typing());
    let caret = (FIRST_ROW * 8 + 7) * WIDTH + 44 * 8;
    assert!(before.argb[caret..caret + 7].iter().all(|&pixel| pixel == 0xff00_0000));
    let editing = screen.render(&font);
    assert!(editing.argb[caret..caret + 7].iter().all(|&pixel| pixel == 0xffff_ff00), "the inline editor has a visible caret at the tag's end");
    clear_entry(&mut screen);
    for ch in "ryu_87 abcdef".chars() {
        screen.type_key(TypedKey::Char(ch));
    }
    assert_eq!(screen.value(Row::PlayerTag), "RYU_87 ABCDE", "typed letters uppercase and stop at twelve characters");
    assert_eq!(screen.type_key(TypedKey::Char('é')), None);
    assert_eq!(screen.type_key(TypedKey::Char(':')), None);
    assert_eq!(screen.settings().player_tag, "PLAYER", "edits remain uncommitted");
    assert_eq!(screen.type_key(TypedKey::Enter), Some(Transition::ValueChanged));
    assert!(!screen.typing());
    assert_eq!(screen.settings().player_tag, "RYU_87 ABCDE");

    screen.apply(MenuInput::Confirm);
    screen.type_key(TypedKey::Home);
    screen.type_key(TypedKey::EraseAhead);
    screen.type_key(TypedKey::Char('k'));
    assert_eq!(screen.value(Row::PlayerTag), "KYU_87 ABCDE");
    screen.type_key(TypedKey::Escape);
    assert_eq!(screen.settings().player_tag, "RYU_87 ABCDE", "Esc cancels without changing the tag");

    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Left);
    screen.apply(MenuInput::Up);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.settings().player_tag, "RYU_87 ABCDF", "pad navigation edits and confirms the last character");

    screen.apply(MenuInput::Confirm);
    clear_entry(&mut screen);
    assert_eq!(screen.type_key(TypedKey::Enter), None, "an empty tag cannot be saved");
    assert!(screen.typing());
    assert!(screen.notice().is_some());
    assert_eq!(screen.settings().player_tag, "RYU_87 ABCDF");
    screen.apply(MenuInput::Back);
    assert!(!screen.typing());
    assert_eq!(screen.settings().player_tag, "RYU_87 ABCDF");
}

#[test]
fn player_tag_underscore_is_visible_in_the_settings_value() {
    let mut settings = Settings::default();
    settings.player_tag = "RYU_87".to_owned();
    let screen = netplay(settings);
    // Transparent ROM glyphs isolate the made-in-style underscore. The
    // explicitly bright palette makes an invisible glyph a real failure.
    let tiles = vec![0xff; 0x4000];
    let program = font_palette();
    let font = RomFont::new(&tiles, &program);
    let frame = screen.render(&font);
    let x = 41 * 8; // Right-aligned RYU_87 ends at column 44; '_' is fourth.
    let y = FIRST_ROW * 8;
    assert!((0..8).any(|dy| (0..8).any(|dx| frame.argb[(y + dy) * WIDTH + x + dx] == 0xffff_ff00)), "the required tag underscore must be visible in the row");
}

#[test]
fn udp_port_entry_validates_keyboard_values_and_supports_pad_edits() {
    let mut screen = netplay(Settings::default());
    for _ in 0..5 {
        screen.apply(MenuInput::Down);
    }
    assert_eq!(screen.row(), Row::UdpPort);
    assert_eq!(screen.apply(MenuInput::Confirm), Some(Transition::CursorMoved));
    assert!(screen.typing());
    for invalid in ["", "1023", "65536"] {
        clear_entry(&mut screen);
        assert_eq!(screen.type_key(TypedKey::Char('A')), None, "port entry accepts digits only");
        for digit in invalid.chars() {
            screen.type_key(TypedKey::Char(digit));
        }
        assert_eq!(screen.type_key(TypedKey::Enter), None);
        assert!(screen.typing(), "an invalid port stays editable");
        assert_eq!(screen.settings().netplay_port, 7000, "invalid text cannot replace a valid listening port");
        let notice = screen.notice().expect("the player sees the allowed range");
        assert!(notice.contains("1024") && notice.contains("65535"));
    }
    screen.type_key(TypedKey::End);
    screen.type_key(TypedKey::Erase);
    screen.type_key(TypedKey::Char('5'));
    assert_eq!(screen.value(Row::UdpPort), "65535");
    assert_eq!(screen.type_key(TypedKey::Char('9')), None, "a port takes at most five digits");
    assert_eq!(screen.type_key(TypedKey::Enter), Some(Transition::ValueChanged));
    assert_eq!(screen.settings().netplay_port, 65535);
    screen.apply(MenuInput::Right);
    assert_eq!(screen.settings().netplay_port, 65535, "row stepping also respects the upper bound");

    screen.apply(MenuInput::Confirm);
    clear_entry(&mut screen);
    for digit in "1024".chars() {
        screen.type_key(TypedKey::Char(digit));
    }
    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Left);
    assert_eq!(screen.settings().netplay_port, 1024);
    assert_eq!(screen.settings().player_tag, "PLAYER", "port edits never change the tag");

    let mut pad = netplay(Settings::default());
    for _ in 0..5 {
        pad.apply(MenuInput::Down);
    }
    pad.apply(MenuInput::Confirm);
    pad.apply(MenuInput::Left);
    pad.apply(MenuInput::Up);
    pad.apply(MenuInput::Confirm);
    assert_eq!(pad.settings().netplay_port, 7001, "the pad edits and confirms a digit");
    pad.apply(MenuInput::Confirm);
    pad.apply(MenuInput::Start);
    assert_eq!(pad.value(Row::UdpPort), "700");
    pad.apply(MenuInput::Back);
    assert_eq!(pad.settings().netplay_port, 7001, "cancel discards pad text edits");
    assert_eq!(Settings::from_toml(&pad.settings().to_toml()).unwrap(), *pad.settings());
}
