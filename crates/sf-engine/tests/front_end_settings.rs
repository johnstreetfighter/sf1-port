//! Ticket #158 (`FE8`): settings and persistence.
//!
//! The Settings screen is driven as a pure screen (every row changed and
//! read back, the preset flipping to CUSTOM, RESET TAB, the tabs, greyed
//! rows skipped) and through the headless game session (leaving it saves,
//! and difficulty and round time reach Local, Training and Arcade fights
//! as DIP words). The settings and match-config files round-trip, an old
//! match-config file without `[dips]` still loads, and a corrupted
//! settings file falls back to the defaults with a message.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::settings::{
    frame_delay_cap, refresh_notice, rows, Aspect, Device, DisplayInfo, Row, Settings, SettingsScreen, Tab, DEFAULT_LOOK,
    FRAME_DELAY_CAP_LOOK_MS, FRAME_DELAY_CAP_MS, LOOK_OFF, MAX_SCALE,
};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, Transition};
use sf_engine::moves::InputNotation;
use sf_engine::roster::{MenuInput, TournamentFighter};
use sf_video::compositor::{fallback_text, ink, RomFont};
use sf_video::era_hud::GaugeArt;
use sf_video::render::{HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const TABS: [Tab; 6] = [Tab::Gameplay, Tab::Roster, Tab::Video, Tab::Sound, Tab::Controls, Tab::Netplay];

fn screen() -> SettingsScreen {
    SettingsScreen::open(MatchConfig::default(), Settings::default())
}

#[test]
fn every_settings_tab_shows_the_dim_version_in_the_header() {
    // Transparent ROM font tiles isolate the small version lettering;
    // this rendering check runs even when no local ROM set is installed.
    let tiles = vec![0xff; 0x4000];
    let font = RomFont::new(&tiles, &[]);
    let mut expected = vec![0xff00_0000; WIDTH * HEIGHT];
    let stamp = sf_engine::version::short_stamp();
    let x = 44 * 8 - stamp.chars().count() * 4;
    fallback_text(&mut expected, x, 24, stamp, font.pen(ink::DIM, 0), 1);
    assert!(expected[24 * WIDTH..29 * WIDTH].iter().any(|&p| p != 0xff00_0000));

    let mut settings = screen();
    for tab in TABS {
        assert_eq!(settings.tab(), tab);
        let frame = settings.render(&font);
        assert_eq!(
            &frame.argb[24 * WIDTH..32 * WIDTH],
            &expected[24 * WIDTH..32 * WIDTH],
            "{tab:?}: the version stays above the tabs, inside column 44",
        );
        settings.apply(MenuInput::TabRight);
    }
}

/// Move the cursor of a fresh screen on `tab` to row `index`.
fn at(tab: usize, index: usize) -> SettingsScreen {
    let mut s = screen();
    for _ in 0..tab {
        s.apply(MenuInput::TabRight);
    }
    let rows = rows(TABS[tab]);
    while s.row() != rows[index] {
        s.apply(MenuInput::Down);
    }
    s
}

/// A fresh screen on `tab` with the cursor on the row labelled `label`.
fn at_label(tab: usize, label: &str) -> SettingsScreen {
    let mut s = screen();
    for _ in 0..tab {
        s.apply(MenuInput::TabRight);
    }
    let start = s.row();
    while s.label(s.row()) != label {
        s.apply(MenuInput::Down);
        assert_ne!(s.row(), start, "a row labelled {label}");
    }
    s
}

#[test]
fn every_live_row_changes_and_reads_back() {
    for (t, tab) in TABS.iter().enumerate() {
        for (i, row) in rows(*tab).iter().enumerate() {
            // REBIND rows open the walk (`pad_play.rs`); SCHEME shows
            // CLASSIC only; TUNE CRT LOOK opens the tuning screen
            // (`tuning_screen.rs`); JUKEBOX opens the Jukebox screen
            // (`jukebox_screen.rs`).
            if matches!(row, Row::Greyed(_) | Row::Heading(_) | Row::ResetTab | Row::Scheme(_) | Row::Rebind(_) | Row::TuneLook | Row::Jukebox | Row::PlayerTag) {
                continue;
            }
            let mut s = at(t, i);
            // FRAME DELAY is dim with the default look on (its own test).
            if s.dim(*row) {
                assert_eq!(s.apply(MenuInput::Right), None, "{tab:?} {}", s.label(*row));
                continue;
            }
            let before = (s.value(*row), *s.config(), s.settings().clone());
            // One way changes it (the other may be at its bound); the
            // opposite way puts it back.
            let (there, back) = {
                let mut probe = s.clone();
                probe.apply(MenuInput::Right);
                if probe.value(*row) != before.0 {
                    (MenuInput::Right, MenuInput::Left)
                } else {
                    (MenuInput::Left, MenuInput::Right)
                }
            };
            assert_eq!(s.apply(there), Some(Transition::ValueChanged));
            let changed = s.value(*row);
            assert_ne!(changed, before.0, "{tab:?} {}", s.label(*row));
            assert!(
                *s.config() != before.1 || *s.settings() != before.2,
                "{tab:?} {}: the change reaches the configuration or the settings",
                s.label(*row)
            );
            s.apply(back);
            if !matches!(row, Row::Preset | Row::BanList) {
                assert_eq!(s.value(*row), before.0, "{tab:?} {}: back again", s.label(*row));
            }
        }
    }
}

#[test]
fn gauge_art_cycles_ruled_blocks_plate_and_reset_tab_puts_it_back() {
    let mut s = at_label(2, "GAUGE ART");
    let mut seen = vec![s.value(s.row())];
    for _ in 0..3 {
        assert_eq!(s.apply(MenuInput::Right), Some(Transition::ValueChanged));
        seen.push(s.value(s.row()));
    }
    assert_eq!(seen, ["RULED", "BLOCKS", "PLATE", "RULED"]);
    assert_eq!(s.apply(MenuInput::Left), Some(Transition::ValueChanged));
    assert_eq!(s.settings().gauge_art, GaugeArt::Plate, "left wraps back");
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(s.settings().gauge_art, GaugeArt::Ruled, "confirm steps on");
    s.apply(MenuInput::Right);
    // Down past CRT LOOK (VO4) to RESET TAB.
    while s.row() != Row::ResetTab {
        s.apply(MenuInput::Down);
    }
    s.apply(MenuInput::Confirm);
    assert_eq!(s.row(), Row::ResetTab);
    assert_eq!(s.settings().gauge_art, GaugeArt::Ruled, "RESET TAB");
}

#[test]
fn move_inputs_cycles_arrows_numpad_words_and_reset_tab_puts_it_back() {
    let mut s = at_label(4, "MOVE INPUTS");
    assert_eq!(s.settings().move_inputs, InputNotation::Arrows, "arrows by default");
    let mut seen = vec![s.value(s.row())];
    for _ in 0..3 {
        assert_eq!(s.apply(MenuInput::Right), Some(Transition::ValueChanged));
        seen.push(s.value(s.row()));
    }
    assert_eq!(seen, ["ARROWS", "NUMPAD", "WORDS", "ARROWS"]);
    assert_eq!(s.apply(MenuInput::Left), Some(Transition::ValueChanged));
    assert_eq!(s.settings().move_inputs, InputNotation::Words, "left wraps back");
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(s.settings().move_inputs, InputNotation::Arrows, "confirm steps on");
    s.apply(MenuInput::Right);
    while s.row() != Row::ResetTab {
        s.apply(MenuInput::Down);
    }
    s.apply(MenuInput::Confirm);
    assert_eq!(s.settings().move_inputs, InputNotation::Arrows, "RESET TAB");
}

#[test]
fn aspect_cycles_true_integer_square_and_reset_tab_puts_it_back() {
    let mut s = at_label(2, "ASPECT");
    let mut seen = vec![s.value(s.row())];
    for _ in 0..3 {
        assert_eq!(s.apply(MenuInput::Right), Some(Transition::ValueChanged));
        seen.push(s.value(s.row()));
    }
    assert_eq!(seen, ["TRUE 4:3", "4:3 INTEGER", "SQUARE PIXELS", "TRUE 4:3"]);
    assert_eq!(s.apply(MenuInput::Left), Some(Transition::ValueChanged));
    assert_eq!(s.settings().aspect, Aspect::Square, "left wraps back");
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(s.settings().aspect, Aspect::True4x3, "confirm steps on");
    s.apply(MenuInput::Right);
    // Down past the tab's other rows to RESET TAB.
    while s.row() != Row::ResetTab {
        s.apply(MenuInput::Down);
    }
    s.apply(MenuInput::Confirm);
    assert_eq!(s.settings().aspect, Aspect::True4x3, "RESET TAB");
}

#[test]
fn the_video_tab_rows_step_clamp_and_reset_tab_restores_them_all() {
    let display = DisplayInfo { refresh_mhz: Some(120_000), max_scale: 6 };
    let mut s = at_label(2, "MODE");
    s.set_display(display);
    assert_eq!(s.value(s.row()), "FULLSCREEN");
    assert_eq!(s.apply(MenuInput::Right), Some(Transition::ValueChanged));
    assert_eq!((s.value(s.row()), s.settings().fullscreen), ("WINDOWED".to_string(), false));
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert!(s.settings().fullscreen, "confirm switches too");
    s.apply(MenuInput::Left);
    // WINDOW SCALE: 1X up to the largest that fits, no wrapping.
    s.apply(MenuInput::Down);
    assert_eq!(s.label(s.row()), "WINDOW SCALE");
    for _ in 0..10 {
        s.apply(MenuInput::Right);
    }
    assert_eq!((s.settings().scale, s.value(s.row())), (6, "6X".to_string()), "the display's largest");
    for _ in 0..10 {
        s.apply(MenuInput::Left);
    }
    assert_eq!(s.settings().scale, 1);
    s.set_display(DisplayInfo { max_scale: 3, ..display });
    for _ in 0..5 {
        s.apply(MenuInput::Right);
    }
    assert_eq!(s.settings().scale, 3, "a smaller monitor offers less");
    // VSYNC toggles (FRAME DELAY has its own test below).
    let mut s = at_label(2, "VSYNC");
    assert_eq!(s.value(s.row()), "ON");
    s.apply(MenuInput::Right);
    assert!(!s.settings().vsync);
    assert_eq!(s.value(s.row()), "OFF");
    s.apply(MenuInput::Confirm);
    assert!(s.settings().vsync);
    // Change every Video row, then RESET TAB restores them all. (CRT LOOK
    // steps from ARCADE CABINET to OFF here, so FRAME DELAY can move.)
    let mut s = screen();
    s.set_display(display);
    for _ in 0..2 {
        s.apply(MenuInput::TabRight);
    }
    let start = s.row();
    // (the cursor skips the group headings; TUNE CRT LOOK takes no left or right)
    for _ in 0..rows(Tab::Video).iter().filter(|r| !matches!(r, Row::Heading(_) | Row::Greyed(_))).count() {
        if !matches!(s.row(), Row::ResetTab) {
            s.apply(MenuInput::Right);
        }
        s.apply(MenuInput::Down);
    }
    assert_eq!(s.row(), start);
    let changed = s.settings().clone();
    let fresh = Settings::default();
    assert!(changed.fullscreen != fresh.fullscreen && changed.vsync != fresh.vsync);
    assert!(changed.scale != fresh.scale && changed.aspect != fresh.aspect && changed.gauge_art != fresh.gauge_art);
    assert!(changed.frame_delay_ms != fresh.frame_delay_ms && changed.crt_look != fresh.crt_look);
    while s.row() != Row::ResetTab {
        s.apply(MenuInput::Down);
    }
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(*s.settings(), fresh, "RESET TAB: every Video row at its default, the other tabs' rows untouched");
}

/// `FRAME DELAY` is capped by the look (user, 2026-11-05): 0 to
/// [`FRAME_DELAY_CAP_MS`] with `CRT LOOK` off; with any look on it shows
/// `0MS`, drawn dim, and left and right change nothing. The saved delay is
/// kept under a look and used again when the look goes off.
#[test]
fn frame_delay_is_capped_by_the_look_and_the_saved_delay_is_kept() {
    assert_eq!((frame_delay_cap(LOOK_OFF), frame_delay_cap(DEFAULT_LOOK)), (FRAME_DELAY_CAP_MS, FRAME_DELAY_CAP_LOOK_MS));
    assert_eq!((FRAME_DELAY_CAP_MS, FRAME_DELAY_CAP_LOOK_MS), (4, 0), "the measured caps");

    // With a look on (the default): 0MS, dim, inert.
    let mut s = at_label(2, "FRAME DELAY");
    assert!(s.dim(s.row()));
    assert_eq!(s.value(s.row()), "0MS");
    for input in [MenuInput::Right, MenuInput::Left] {
        assert_eq!(s.apply(input), None, "{input:?} reports no change");
    }
    assert_eq!(s.settings().frame_delay_ms, 0);

    // With the look OFF: 0 up to the cap and no further, either way.
    let off = Settings { crt_look: LOOK_OFF.into(), ..Settings::default() };
    let mut s = SettingsScreen::open(MatchConfig::default(), off);
    for _ in 0..2 {
        s.apply(MenuInput::TabRight);
    }
    while s.label(s.row()) != "FRAME DELAY" {
        s.apply(MenuInput::Down);
    }
    assert!(!s.dim(s.row()));
    assert_eq!(s.apply(MenuInput::Left), Some(Transition::ValueChanged));
    assert_eq!(s.value(s.row()), "0MS", "no lower than 0");
    for _ in 0..20 {
        s.apply(MenuInput::Right);
    }
    assert_eq!((s.settings().frame_delay_ms, s.value(s.row())), (FRAME_DELAY_CAP_MS, format!("{FRAME_DELAY_CAP_MS}MS")));
    s.apply(MenuInput::Left);
    assert_eq!(s.settings().frame_delay(), 3);

    // A look switched on: the delay in use is 0, the saved 3 is kept, and
    // it is back when the look goes OFF again.
    while s.label(s.row()) != "CRT LOOK" {
        s.apply(MenuInput::Up);
    }
    s.apply(MenuInput::Right);
    assert_eq!(s.settings().crt_look, DEFAULT_LOOK);
    assert_eq!((s.settings().frame_delay(), s.settings().frame_delay_ms), (0, 3));
    assert_eq!(s.value(Row::FrameDelay), "0MS");
    assert!(s.dim(Row::FrameDelay));
    s.apply(MenuInput::Left);
    assert_eq!(s.settings().crt_look, LOOK_OFF);
    assert_eq!((s.settings().frame_delay(), s.value(Row::FrameDelay)), (3, "3MS".to_string()));

    // A saved delay above the cap in use is clamped when used, not changed.
    let saved = Settings { crt_look: LOOK_OFF.into(), frame_delay_ms: 9, ..Settings::default() };
    assert_eq!(saved.frame_delay(), FRAME_DELAY_CAP_MS);
    let saved = Settings { frame_delay_ms: 3, ..Settings::default() };
    assert_eq!((saved.frame_delay(), saved.frame_delay_ms), (0, 3));
}

#[test]
fn the_notice_line_names_the_refresh_rate_and_what_it_means() {
    let say = |hz: f64, vsync| refresh_notice(Some((hz * 1000.0) as u32), vsync);
    assert_eq!(say(120.0, true), ["120HZ: EVERY FRAME SHOWN TWICE"]);
    assert_eq!(say(119.88, true), ["120HZ: EVERY FRAME SHOWN TWICE"], "NTSC rates count");
    assert_eq!(say(60.0, true), ["60HZ: EVERY FRAME SHOWN ONCE"]);
    assert_eq!(say(240.0, true), ["240HZ: EVERY FRAME SHOWN 4 TIMES"]);
    assert_eq!(say(144.0, true), ["144HZ: SET YOUR DISPLAY TO 120 OR 60", "FOR SMOOTH MOTION"]);
    assert_eq!(say(165.0, true)[0], "165HZ: SET YOUR DISPLAY TO 120 OR 60");
    assert_eq!(say(120.0, false)[0], "120HZ, VSYNC OFF: FRAMES SHOWN AS SOON");
    assert!(refresh_notice(None, true).is_empty() && refresh_notice(Some(0), true).is_empty());
    // Every line fits the screen's frame (44 cells inside it).
    for hz in [24.0, 59.94, 60.0, 75.0, 120.0, 144.0, 240.0, 360.0] {
        for vsync in [true, false] {
            assert!(say(hz, vsync).iter().all(|l| l.len() <= 44), "{hz} {vsync}");
        }
    }
}

#[test]
fn the_gameplay_rows_wording_and_values() {
    let s = screen();
    let gameplay: Vec<_> = rows(Tab::Gameplay).iter().map(|r| (s.label(*r), s.value(*r))).collect();
    assert_eq!(gameplay[0], ("PRESET", "DEFINITIVE OVERHAUL".to_string()));
    assert!(gameplay.contains(&("RECOVERY WINDOW", "4F".to_string())));
    assert!(gameplay.contains(&("COMMAND WINDOW", "28F".to_string())));
    assert!(gameplay.contains(&("CPU REACTION", "8F".to_string())));
    assert!(gameplay.contains(&("DIFFICULTY", "2".to_string())));
    assert!(gameplay.contains(&("ROUND TIME", "100".to_string())));
    assert!(gameplay.iter().filter(|(_, v)| v == "ON" || v == "OFF").count() >= 14);
    assert_eq!(gameplay.last().unwrap().0, "RESET TAB");
    let video: Vec<_> = rows(Tab::Video).iter().map(|r| (s.label(*r), s.value(*r))).collect();
    let s_ = |label: &str, value: &str| (label.to_string(), value.to_string());
    let video: Vec<_> = video.into_iter().map(|(l, v)| (l.to_string(), v)).collect();
    assert_eq!(
        video,
        [
            s_("DISPLAY", ""),
            s_("MODE", "FULLSCREEN"),
            s_("WINDOW SCALE", "3X"),
            s_("GAUGE ART", "RULED"),
            s_("MONITOR", ""),
            s_("ASPECT", "TRUE 4:3"),
            s_("CRT LOOK", "ARCADE CABINET"),
            s_("TUNE CRT LOOK >", ""),
            s_("MOTION", ""),
            s_("VSYNC", "ON"),
            s_("FRAME DELAY", "0MS"),
            s_("RESET TAB", ""),
        ],
        "the Video tab: DISPLAY, MONITOR and MOTION groups, every default"
    );
    let sound: Vec<_> = rows(Tab::Sound).iter().map(|r| s.label(*r)).collect();
    assert_eq!(sound, ["MASTER", "ANNOUNCER", "FM", "SAMPLES", "JUKEBOX", "RESET TAB"]);
    assert_eq!(s.value(Row::Announcer), "10", "the announcer defaults to 10");
    let roster: Vec<_> = rows(Tab::Roster).iter().map(|r| s.label(*r)).collect();
    assert_eq!(
        roster,
        ["FULL CAST", "FOUR COUNTRIES", "BAN LIST", "WHAT-IF STYLES", "BONUS CHALLENGERS", "SECRET RIVALS", "RESET TAB"]
    );
    let controls: Vec<_> = rows(Tab::Controls).iter().map(|r| (s.label(*r), s.value(*r))).collect();
    let labels: Vec<_> = controls.iter().map(|(l, _)| *l).collect();
    assert_eq!(
        labels,
        ["P1 DEVICE", "P2 DEVICE", "P1 SCHEME", "P2 SCHEME", "MOVE INPUTS", "REBIND P1", "REBIND P2", "REBIND MENU", "RESET TAB"]
    );
    assert_eq!(controls[0].1, "AUTO", "devices default to connection order");
    assert_eq!(controls[2].1, "CLASSIC");
}

#[test]
fn a_toggle_edit_flips_the_preset_to_custom_and_a_preset_change_keeps_the_fighters() {
    let mut s = at(0, 1); // POSITIVE EDGE
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(s.config().preset, RulesetPreset::Custom);
    // The preset row: Classic Competitive's toggles and roster, the select's
    // fighters kept.
    let mut config = MatchConfig::default();
    config.roster.p1 = TournamentFighter::Sagat;
    config.roster.p2 = TournamentFighter::Gen;
    let mut s = SettingsScreen::open(config, Settings::default());
    s.apply(MenuInput::Right);
    s.apply(MenuInput::Right);
    assert_eq!(s.config().preset, RulesetPreset::ClassicCompetitive);
    assert_eq!(s.config().toggles, MatchConfig::from_preset(RulesetPreset::ClassicCompetitive).toggles);
    assert_eq!((s.config().roster.p1, s.config().roster.p2), (TournamentFighter::Sagat, TournamentFighter::Gen));
}

#[test]
fn reset_tab_restores_that_tabs_defaults_only() {
    let mut s = screen();
    // Change a row on every tab.
    s.apply(MenuInput::Right); // PRESET
    s.apply(MenuInput::TabRight);
    s.apply(MenuInput::Right); // FULL CAST
    s.apply(MenuInput::TabRight);
    s.apply(MenuInput::Down); // WINDOW SCALE, under MODE
    s.apply(MenuInput::Left); // 2X
    s.apply(MenuInput::TabRight);
    s.apply(MenuInput::Left); // MASTER 9
    s.apply(MenuInput::Down);
    assert_eq!(s.row(), Row::Announcer, "ANNOUNCER sits under MASTER");
    for _ in 0..12 {
        s.apply(MenuInput::Left);
    }
    assert_eq!(s.settings().announcer, 0, "ANNOUNCER goes down to 0, off");
    // RESET TAB on Sound restores the volumes only.
    let (config, scale) = (*s.config(), s.settings().scale);
    assert_eq!((s.settings().master, scale), (9, 2));
    while s.row() != Row::ResetTab {
        s.apply(MenuInput::Down);
    }
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(s.settings().master, 10);
    assert_eq!(s.settings().announcer, 10);
    assert_eq!(s.settings().scale, scale);
    assert_eq!(*s.config(), config);
    // RESET TAB on Gameplay restores the preset and its toggles.
    let mut s = SettingsScreen::open(config, Settings::default());
    s.apply(MenuInput::Up);
    s.apply(MenuInput::Confirm);
    assert_eq!(s.config().preset, RulesetPreset::DefinitiveOverhaul);
    assert_eq!(s.config().toggles, MatchConfig::default().toggles);
    assert_eq!(s.config().roster, config.roster, "the roster is the Roster tab's");
}

#[test]
fn the_tab_pair_cycles_six_tabs_and_greyed_rows_are_skipped() {
    let mut s = screen();
    let mut seen = Vec::new();
    for _ in 0..7 {
        seen.push(s.tab());
        s.apply(MenuInput::TabRight);
    }
    assert_eq!(seen, [Tab::Gameplay, Tab::Roster, Tab::Video, Tab::Sound, Tab::Controls, Tab::Netplay, Tab::Gameplay]);
    let mut s = screen();
    s.apply(MenuInput::TabLeft);
    assert_eq!(s.tab(), Tab::Netplay, "the pair goes both ways through all six tabs");
    let mut s = at(1, 2); // BAN LIST
    assert_eq!(s.apply(MenuInput::Down), Some(Transition::CursorMoved));
    assert_eq!(s.label(s.row()), "WHAT-IF STYLES");
    assert_eq!(s.apply(MenuInput::Down), Some(Transition::CursorMoved));
    assert_eq!(s.row(), Row::ResetTab, "the two greyed rows are skipped");
}

// ---- persistence ------------------------------------------------------------

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sf1-port-fe8-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn the_settings_and_match_config_files_round_trip_and_bad_files_fall_back() {
    let dir = scratch("files");
    let settings = Settings {
        scale: 9,
        fullscreen: false,
        vsync: false,
        frame_delay_ms: FRAME_DELAY_CAP_MS,
        gauge_art: GaugeArt::Plate,
        aspect: Aspect::Integer4x3,
        master: 3,
        fm: 0,
        samples: 8,
        announcer: 4,
        rom_folder: Some("E:\\games\\roms\\sf".into()),
        cabinet_dsw1: 0x5fff,
        cabinet_dsw2: 0xfbff,
        devices: [Device::Pad2, Device::Keyboard],
        menu_positions: [3, 1, 0, 1],
        hold_combo: true,
        fill_meters: true,
        move_inputs: InputNotation::Numpad,
        crt_look: "off".into(),
        jukebox: Default::default(),
        ..Settings::default()
    };
    let path = dir.join("settings.toml");
    settings.save(&path).unwrap();
    assert_eq!(Settings::load_or_default(&path), (settings.clone(), None));
    assert!(settings.to_toml().contains("\nannouncer = 4\n"));
    // A settings file from before `announcer` loads him at 10; 0 is off and
    // round-trips; 11 is refused.
    let old: String = settings.to_toml().lines().filter(|l| !l.starts_with("announcer")).map(|l| l.to_owned() + "\n").collect();
    let parsed = Settings::from_toml(&old).unwrap();
    assert_eq!(parsed, Settings { announcer: 10, ..settings.clone() });
    let off = Settings { announcer: 0, ..settings.clone() };
    assert_eq!(Settings::from_toml(&off.to_toml()), Ok(off));
    assert!(Settings::from_toml("announcer = 11\n").is_err());
    let mut config = MatchConfig::from_preset(RulesetPreset::PureFootsies1987Plus);
    config.difficulty = 4;
    config.round_time = 150;
    let cpath = dir.join("match_config.toml");
    config.save(&cpath).unwrap();
    assert_eq!(MatchConfig::load_or_default(&cpath), (config, None));
    // A match-config file from before `[dips]` loads with the factory values.
    let old: String = config.to_toml().split("\n[dips]").next().unwrap().to_string();
    let parsed = MatchConfig::from_toml(&old).unwrap();
    assert_eq!((parsed.difficulty, parsed.round_time), (2, 100));
    // Training's `FILL METERS` (TR7) round-trips beside `hold_combo`; a file
    // from before it loads OFF; a stray value is refused.
    assert!(!Settings::default().fill_meters);
    assert!(settings.to_toml().contains("
fill_meters = true
"));
    let off = Settings { fill_meters: false, ..settings.clone() };
    assert!(off.to_toml().contains("
fill_meters = false
"));
    assert_eq!(Settings::from_toml(&off.to_toml()), Ok(off.clone()));
    let old: String = settings.to_toml().lines().filter(|l| !l.starts_with("fill_meters")).map(|l| l.to_owned() + "
").collect();
    assert_eq!(Settings::from_toml(&old), Ok(off));
    for bad in ["fill_meters = 1
", "fill_meters = \"true\"
", "fill_meters = yes
"] {
        assert!(Settings::from_toml(bad).is_err(), "{bad:?} is refused");
    }
    // `MOVE INPUTS` (ML2) round-trips by name; a file from before it loads
    // ARROWS; a stray value is refused.
    assert!(settings.to_toml().contains("move_inputs = \"numpad\""));
    for notation in InputNotation::ALL {
        let saved = Settings { move_inputs: notation, ..settings.clone() };
        assert_eq!(Settings::from_toml(&saved.to_toml()), Ok(saved));
    }
    let old: String = settings.to_toml().lines().filter(|l| !l.starts_with("move_inputs")).map(|l| l.to_owned() + "
").collect();
    assert_eq!(Settings::from_toml(&old).unwrap().move_inputs, InputNotation::Arrows);
    assert!(Settings::from_toml("move_inputs = \"kanji\"
").is_err());
    assert!(!config.to_toml().contains("move_inputs"));
    // A settings file from before `gauge_art` loads RULED, the new row is
    // never in the match config, and a stray value is refused.
    let old: String = settings.to_toml().lines().filter(|l| !l.starts_with("gauge_art")).collect::<Vec<_>>().join("\n");
    assert_eq!(Settings::from_toml(&old).unwrap().gauge_art, GaugeArt::Ruled);
    assert!(settings.to_toml().contains("gauge_art = \"plate\""));
    assert!(!config.to_toml().contains("gauge_art"));
    // A settings file from before the full Video tab loads vsync on and no
    // frame delay; `scale` takes values above 4; the new keys round-trip by
    // name; a scale past the ceiling is refused, but a delay past the cap
    // (the cap was 2 ms once, and may change again) loads clamped, the rest
    // of the file with it, and a delay under a look round-trips unclamped.
    let old: String = settings
        .to_toml()
        .lines()
        .filter(|l| !l.starts_with("vsync") && !l.starts_with("frame_delay_ms"))
        .map(|l| l.to_owned() + "\n")
        .collect();
    assert_eq!(Settings::from_toml(&old).unwrap(), Settings { vsync: true, frame_delay_ms: 0, ..settings.clone() });
    assert!(settings.to_toml().contains(&format!("\nvsync = false\nframe_delay_ms = {FRAME_DELAY_CAP_MS}\n")));
    assert!(settings.to_toml().contains("\nscale = 9\n"));
    let past_cap = format!("scale = 5\nframe_delay_ms = {}\n", FRAME_DELAY_CAP_MS + 5);
    let loaded = Settings::from_toml(&past_cap).unwrap();
    assert_eq!((loaded.frame_delay_ms, loaded.scale), (FRAME_DELAY_CAP_MS, 5));
    assert_eq!(Settings::from_toml("frame_delay_ms = 4000000000\n").unwrap().frame_delay_ms, FRAME_DELAY_CAP_MS);
    let under_look = Settings { crt_look: DEFAULT_LOOK.into(), frame_delay_ms: 3, ..settings.clone() };
    assert_eq!(Settings::from_toml(&under_look.to_toml()).unwrap(), under_look);
    let past_scale = format!("scale = {}\n", MAX_SCALE + 1);
    for bad in [past_scale.as_str(), "vsync = maybe\n", "frame_delay_ms = soon\n"] {
        assert!(Settings::from_toml(bad).is_err(), "{bad:?} is refused");
    }
    assert_eq!(Settings::from_toml(&format!("scale = {MAX_SCALE}\n")).unwrap().scale, MAX_SCALE);
    // A first launch (no file at all, or an empty one) is fullscreen with the
    // look on, at true 4:3, vsync on, with the faithful gameplay untouched.
    let first = Settings::from_toml("").unwrap();
    assert_eq!(first, Settings::default());
    assert!(first.fullscreen && first.vsync && first.frame_delay_ms == 0 && first.scale == 3);
    assert_eq!((first.aspect, first.crt_look.as_str()), (Aspect::True4x3, "arcade_cabinet"));
    assert_eq!(MatchConfig::default().preset, RulesetPreset::DefinitiveOverhaul, "gameplay defaults untouched");
    // A settings file from before `aspect` loads TRUE 4:3; the three values
    // round-trip by their file names; a stray value is refused.
    let old: String = settings.to_toml().lines().filter(|l| !l.starts_with("aspect")).map(|l| l.to_owned() + "\n").collect();
    assert_eq!(Settings::from_toml(&old).unwrap(), Settings { aspect: Aspect::True4x3, ..settings.clone() });
    assert_eq!(Settings::default().aspect, Aspect::True4x3);
    for (aspect, name) in [(Aspect::True4x3, "true_4_3"), (Aspect::Integer4x3, "integer_4_3"), (Aspect::Square, "square")] {
        let s = Settings { aspect, ..settings.clone() };
        assert!(s.to_toml().contains(&format!("aspect = \"{name}\"")));
        assert_eq!(Settings::from_toml(&s.to_toml()), Ok(s));
    }
    // Corrupted settings: the defaults and a message, no crash.
    for bad in ["scale = 21\n", "volume = 3\n", "fullscreen = maybe\n", "gauge_art = \"neon\"\n", "aspect = \"wide\"\n", "garbage"] {
        std::fs::write(&path, bad).unwrap();
        let (s, diag) = Settings::load_or_default(&path);
        assert_eq!(s, Settings::default(), "{bad:?}");
        assert!(diag.is_some(), "{bad:?} is reported");
    }
    let (s, diag) = Settings::load_or_default(&dir.join("missing.toml"));
    assert_eq!(s, Settings::default());
    assert!(diag.is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- through the session -------------------------------------------------------

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(idle());
    out
}

fn rb(m: &sf_engine::run::PowerOnMachine, addr: u32) -> u8 {
    m.work_ram()[(addr - 0xff8000) as usize]
}

#[test]
fn the_aspect_row_changes_the_picture_at_once_and_is_saved_on_leaving() {
    let Some(a) = assets() else { return };
    let mut s = GameSession::new(a, MatchConfig::default());
    for _ in 0..200 {
        s.step(idle());
    }
    assert_eq!(s.live_aspect(), Aspect::True4x3);
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "settings");
    for _ in 0..2 {
        tap(&mut s, with(|i| i.menu.tab_right = true));
    }
    // MODE, then WINDOW SCALE, GAUGE ART (DISPLAY) and ASPECT (MONITOR).
    for _ in 0..3 {
        tap(&mut s, with(|i| i.p1.directions[2] = true));
    }
    // Each press shows in the very step that takes it, the file untouched.
    for want in [Aspect::Integer4x3, Aspect::Square, Aspect::True4x3] {
        s.step(with(|i| i.p1.directions[0] = true));
        assert_eq!(s.live_aspect(), want, "applies in the step that changes it");
        assert_eq!(s.settings().aspect, Aspect::True4x3, "not saved until Settings is left");
        s.step(idle());
    }
    s.step(with(|i| i.p1.directions[0] = true));
    s.step(idle());
    let out = tap(&mut s, with(|i| i.menu.back = true));
    assert!(out.settings_changed, "leaving Settings saves");
    assert_eq!((s.settings().aspect, s.live_aspect()), (Aspect::Integer4x3, Aspect::Integer4x3));
}

#[test]
fn leaving_settings_saves_and_difficulty_and_round_time_reach_every_fight() {
    let Some(a) = assets() else { return };
    let mut s = GameSession::new(a, MatchConfig::default());
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "settings");
    // DIFFICULTY up to 4 and ROUND TIME up to 250, both near the end of the
    // Gameplay rows: Up wraps from PRESET to RESET TAB.
    tap(&mut s, with(|i| i.p1.directions[3] = true)); // RESET TAB
    tap(&mut s, with(|i| i.p1.directions[3] = true)); // ROUND TIME
    for _ in 0..3 {
        tap(&mut s, with(|i| i.p1.directions[0] = true));
    }
    tap(&mut s, with(|i| i.p1.directions[3] = true)); // DIFFICULTY
    for _ in 0..2 {
        tap(&mut s, with(|i| i.p1.directions[0] = true));
    }
    let out = tap(&mut s, with(|i| i.menu.back = true));
    assert_eq!(out.screen, "main menu");
    assert!(out.settings_changed, "leaving Settings saves");
    assert_eq!((s.match_config().difficulty, s.match_config().round_time), (4, 250));

    // Each play mode's fight boots with them: difficulty 4 decodes to 3,
    // round time 250 to 3 (0xffc007, 0xff8e1c). The Main Menu's cursor is
    // still on SETTINGS.
    let up = with(|i| i.p1.directions[3] = true);
    let down = with(|i| i.p1.directions[2] = true);
    let ok = with(|i| i.menu.confirm = true);
    let walks: [(&str, &[FrameInputs]); 3] = [
        ("local", &[up, ok, ok, with(|i| i.p1.buttons[0] = true), with(|i| i.p2.buttons[0] = true)]),
        ("arcade", &[up, up, ok, ok, ok]),
        ("training", &[up, up, ok, down, ok, ok, ok, ok]),
    ];
    for (name, steps) in walks {
        let mut t = s.clone();
        for step in steps {
            if t.screen() == "select" && t.match_config().era_select_enabled() {
                assert_eq!(tap(&mut t, *step).transition, Some(Transition::StyleOpened));
            }
            tap(&mut t, *step);
        }
        let mut f = 0;
        while t.fight_machine().is_none() || t.screen() == "drive" {
            t.step(idle());
            f += 1;
            assert!(f < 60, "{name}: a fight starts (at {})", t.screen());
        }
        let m = t.fight_machine().unwrap();
        assert_eq!((rb(m, 0xffc007), rb(m, 0xff8e1c)), (3, 3), "{name}");
    }
}

#[test]
fn the_match_config_carries_fill_meters_off_in_every_preset_and_does_not_save_it() {
    assert!(!MatchConfig::default().fill_meters);
    for preset in RulesetPreset::ALL {
        let mut config = MatchConfig::from_preset(preset);
        assert!(!config.fill_meters, "{preset:?}");
        // Choosing a preset leaves a stamped switch as it is.
        config.fill_meters = true;
        config.apply_preset(preset);
        assert!(config.fill_meters, "{preset:?}: a preset does not touch it");
        // It is a fight's stamp from settings.toml, not part of the file.
        let text = config.to_toml();
        assert!(!text.contains("fill_meters"), "{preset:?}");
        assert!(!MatchConfig::from_toml(&text).unwrap().fill_meters, "{preset:?}");
    }
}
