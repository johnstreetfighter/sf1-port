//! Ticket #207 (`VO4`): the Video tab's `CRT LOOK` row. It steps through
//! the look list the host gives (`OFF` first), `RESET TAB` puts back
//! `ARCADE CABINET`, `settings.toml` round-trips `crt_look`, and a look the
//! host could not load falls back to `OFF` with a notice. Through the
//! session, the frame output's video state follows the row the frame it
//! changes, before Settings is left (the host applies it at once).

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::front::settings::{rows, LookName, Row, Settings, SettingsScreen, Tab, DEFAULT_LOOK, LOOK_OFF};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, Transition};
use sf_engine::roster::MenuInput;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn looks() -> Vec<LookName> {
    [("off", "OFF"), ("arcade_cabinet", "ARCADE CABINET"), ("pro_monitor", "PRO MONITOR")]
        .into_iter()
        .map(|(id, name)| LookName { id: id.into(), name: name.into() })
        .collect()
}

/// A fresh screen on the Video tab's `CRT LOOK` row.
fn crt_look_row(settings: Settings) -> SettingsScreen {
    let mut s = SettingsScreen::open(MatchConfig::default(), settings).with_looks(looks());
    s.apply(MenuInput::TabRight);
    s.apply(MenuInput::TabRight);
    assert_eq!(s.tab(), Tab::Video);
    while s.row() != Row::CrtLook {
        s.apply(MenuInput::Down);
    }
    s
}

#[test]
fn crt_look_steps_through_the_list_and_reset_tab_puts_back_arcade_cabinet() {
    assert!(rows(Tab::Video).contains(&Row::CrtLook));
    let mut s = crt_look_row(Settings::default());
    assert_eq!(s.label(s.row()), "CRT LOOK");
    assert_eq!(s.value(Row::CrtLook), "ARCADE CABINET", "the default look");
    let mut seen = vec![s.value(Row::CrtLook)];
    for _ in 0..3 {
        assert_eq!(s.apply(MenuInput::Right), Some(Transition::ValueChanged));
        seen.push(s.value(Row::CrtLook));
    }
    assert_eq!(seen, ["ARCADE CABINET", "PRO MONITOR", "OFF", "ARCADE CABINET"], "right wraps through OFF");
    assert_eq!(s.apply(MenuInput::Left), Some(Transition::ValueChanged));
    assert_eq!(s.settings().crt_look, LOOK_OFF);
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(s.settings().crt_look, DEFAULT_LOOK, "confirm steps on");
    s.apply(MenuInput::Left);
    while s.row() != Row::ResetTab {
        s.apply(MenuInput::Down);
    }
    s.apply(MenuInput::Confirm);
    assert_eq!(s.settings().crt_look, DEFAULT_LOOK, "RESET TAB");
}

#[test]
fn a_look_that_failed_falls_back_to_off_with_a_notice() {
    let mut s = crt_look_row(Settings { crt_look: "pro_monitor".into(), ..Settings::default() });
    assert_eq!(s.notice(), None);
    s.look_failed("pro_monitor", "LOOK FAILED: SHADER MISSING");
    assert_eq!(s.look(), LOOK_OFF, "drawn as OFF");
    assert_eq!(s.settings().crt_look, "pro_monitor", "the choice is kept, never the fallback");
    assert_eq!(s.value(Row::CrtLook), "OFF");
    assert!(!s.tune_enabled() && s.dim(Row::TuneLook));
    assert_eq!(s.value(Row::FrameDelay), "0MS");
    assert!(!s.dim(Row::FrameDelay), "OFF's cap");
    assert_eq!(s.notice(), Some("LOOK FAILED: SHADER MISSING"));
    // A failure for a look no longer chosen leaves the row alone.
    s.apply(MenuInput::Right);
    assert_eq!(s.notice(), None, "the next change clears the notice");
    s.look_failed("pro_monitor", "LOOK FAILED: LATE");
    assert_eq!(s.settings().crt_look, DEFAULT_LOOK);
    // An id the list does not hold is shown as it is.
    let s = crt_look_row(Settings { crt_look: "gone".into(), ..Settings::default() });
    assert_eq!(s.value(Row::CrtLook), "GONE");
}

/// Review finding 1: a look's name (a drop-in's file or path, or an id the
/// list does not hold) longer than the row's room is cut with `..`, and the
/// screen draws it without overflowing the value column.
#[test]
fn a_long_look_name_or_unknown_id_is_cut_to_fit_and_draws() {
    let long = "A".repeat(60);
    let mut list = looks();
    list.push(LookName { id: "deep/long.slangp".into(), name: long.clone() });
    // A font of blank tiles: the drawing is what is checked, not the pixels.
    let (gfx4, program) = (vec![0u8; 0x8000], Vec::new());
    let font = sf_video::compositor::RomFont::new(&gfx4, &program);
    for id in ["deep/long.slangp", &"my looks/".repeat(8)] {
        let s = SettingsScreen::open(MatchConfig::default(), Settings { crt_look: id.into(), ..Settings::default() })
            .with_looks(list.clone());
        let value = s.value(Row::CrtLook);
        let cells = s.value_cells(Row::CrtLook);
        assert!(value.chars().count() <= cells && value.ends_with(".."), "{id}: {value:?} in {cells} cells");
        let mut s = s;
        s.apply(MenuInput::TabRight);
        s.apply(MenuInput::TabRight);
        assert_eq!(s.tab(), Tab::Video);
        s.render(&font);
    }
    // A name that fits is as it is.
    let s = SettingsScreen::open(MatchConfig::default(), Settings::default()).with_looks(looks());
    assert_eq!(s.value(Row::CrtLook), "ARCADE CABINET");
}

#[test]
fn settings_toml_round_trips_crt_look_and_an_old_file_takes_the_default() {
    let s = Settings { crt_look: "off".into(), ..Settings::default() };
    assert!(s.to_toml().contains("\ncrt_look = \"off\"\n"));
    assert_eq!(Settings::from_toml(&s.to_toml()), Ok(s.clone()));
    let dropin = Settings { crt_look: "my looks/neon.slangp".into(), ..Settings::default() };
    assert_eq!(Settings::from_toml(&dropin.to_toml()), Ok(dropin));
    let old: String = s.to_toml().lines().filter(|l| !l.starts_with("crt_look")).map(|l| l.to_owned() + "\n").collect();
    assert_eq!(Settings::from_toml(&old).unwrap().crt_look, DEFAULT_LOOK, "a file from before crt_look");
    for bad in ["crt_look = \"\"\n", "crt_look = off\n"] {
        assert!(Settings::from_toml(bad).is_err(), "{bad:?} is refused");
    }
}

/// Review finding 2: a `#`, `"`, `\` or space in a drop-in's path or the
/// ROM folder round-trips, and a `#` outside the quotes is still a comment.
#[test]
fn settings_toml_round_trips_paths_with_hashes_quotes_backslashes_and_spaces() {
    for text in [
        "CRT #2.slangp",
        "my looks/CRT #2.slangp",
        "say \"hi\".slangp",
        "back\\slash.slangp",
        "ends in a backslash\\",
        "ends in a quote\"",
        "\\\\server\\share # x\\",
        "  spaced  out  .slangp",
    ] {
        let s = Settings { crt_look: text.into(), rom_folder: Some(text.into()), ..Settings::default() };
        assert_eq!(Settings::from_toml(&s.to_toml()), Ok(s.clone()), "{text:?}\n{}", s.to_toml());
    }
    let commented = "crt_look = \"a#b.slangp\" # a comment with \"quotes\"\nscale = 2 # two\n";
    let s = Settings::from_toml(commented).unwrap();
    assert_eq!((s.crt_look.as_str(), s.scale), ("a#b.slangp", 2));
    // A file from before the escapes: its backslashes are kept as written.
    let old = Settings::from_toml("rom_folder = \"D:\\roms\\sf\\\"\n").unwrap();
    assert_eq!(old.rom_folder.as_deref(), Some("D:\\roms\\sf\\"));
    for bad in ["crt_look = \"open\n", "crt_look = \"a\" b\n"] {
        assert!(Settings::from_toml(bad).is_err(), "{bad:?} is refused");
    }
}

// ---- through the session -------------------------------------------------------

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(FrameInputs::default());
    out
}

/// The session on the Video tab's `CRT LOOK` row.
fn session_on_crt_look() -> Option<GameSession> {
    let mut s = GameSession::new(assets()?, MatchConfig::default()).with_looks(looks());
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "settings");
    tap(&mut s, with(|i| i.menu.tab_right = true));
    tap(&mut s, with(|i| i.menu.tab_right = true));
    // SCALE, FULLSCREEN, ASPECT and GAUGE ART are above it.
    for _ in 0..4 {
        tap(&mut s, with(|i| i.p1.directions[2] = true));
    }
    Some(s)
}

#[test]
fn the_video_state_follows_crt_look_at_once_and_leaving_keeps_it() {
    let Some(mut s) = session_on_crt_look() else { return };
    assert_eq!(s.video().look, DEFAULT_LOOK);
    let out = s.step(with(|i| i.p1.directions[0] = true));
    assert_eq!(out.transition, Some(Transition::ValueChanged));
    assert_eq!(out.video.look, "pro_monitor", "the frame that changes the row carries the new look");
    assert!(!out.settings_changed, "nothing is saved until Settings is left");
    assert_eq!(s.settings().crt_look, DEFAULT_LOOK, "the session's settings wait for leaving");
    s.step(FrameInputs::default());
    let out = tap(&mut s, with(|i| i.menu.back = true));
    assert_eq!(out.screen, "main menu");
    assert!(out.settings_changed);
    assert_eq!(s.settings().crt_look, "pro_monitor");
    assert_eq!(out.video.look, "pro_monitor");
}

/// Review finding 8: a look the host could not load is drawn as `OFF` for
/// the run, but the fallback is never saved, with Settings open or not: the
/// settings keep the look, so the next launch tries it again.
#[test]
fn a_look_the_host_could_not_load_shows_off_this_run_and_is_never_saved_as_off() {
    // Settings open.
    let Some(mut s) = session_on_crt_look() else { return };
    s.look_failed(DEFAULT_LOOK, "shaders/looks/arcade-cabinet.slangp not found");
    let out = s.step(FrameInputs::default());
    assert_eq!(out.video.look, LOOK_OFF, "the open Settings screen shows OFF");
    assert_eq!(out.video.frame_delay_ms, 0);
    let out = tap(&mut s, with(|i| i.menu.back = true));
    assert_eq!(out.screen, "main menu");
    assert!(out.settings_changed, "leaving saves");
    assert_eq!(s.settings().crt_look, DEFAULT_LOOK, "what is saved is the look chosen, not OFF");
    assert_eq!(out.video.look, LOOK_OFF, "still OFF for this run");
    assert_eq!(s.video().look, LOOK_OFF);
    // Opening Settings again (the Main Menu kept its row) shows OFF.
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "settings");
    assert_eq!(s.video().look, LOOK_OFF, "the reopened screen knows it failed");
    let out = tap(&mut s, with(|i| i.menu.back = true));
    assert_eq!(s.settings().crt_look, DEFAULT_LOOK);
    assert_eq!(out.video.look, LOOK_OFF);

    // Settings closed: the fallback alone flags no save, and a save for
    // something else (here leaving Settings) keeps the look.
    let Some(assets) = assets() else { return };
    let mut s = GameSession::new(assets, MatchConfig::default())
        .with_looks(looks())
        .with_settings(Settings { crt_look: "pro_monitor".into(), ..Settings::default() });
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    s.look_failed("pro_monitor", "pro-monitor.slangp: shader error");
    let out = s.step(FrameInputs::default());
    assert_eq!(out.video.look, LOOK_OFF);
    assert!(!out.settings_changed, "the fallback is not a change to save");
    assert_eq!(s.settings().crt_look, "pro_monitor");
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    tap(&mut s, with(|i| i.p1.directions[2] = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "settings");
    assert_eq!(s.video().look, LOOK_OFF, "Settings opens showing OFF");
    let out = tap(&mut s, with(|i| i.menu.back = true));
    assert!(out.settings_changed);
    assert_eq!(s.settings().crt_look, "pro_monitor", "saved as chosen: the next launch tries it again");
}

/// Review finding 3: a look's error naming an accented file or folder is
/// put in the ROM font's characters and cut by characters, never inside one.
#[test]
fn a_look_error_with_accents_is_cut_by_characters_into_the_rom_fonts_set() {
    let Some(assets) = assets() else { return };
    let mut s = GameSession::new(assets, MatchConfig::default()).with_looks(looks());
    // "LOOK FAILED: " and 32 more: the `é` straddles the 46th byte.
    let error = format!("{}é_{}", "a".repeat(32), "C:\\Users\\José\\shaders\\x.slangp");
    s.look_failed("pro_monitor", &error);
    let notice = s.notice().expect("the notice").to_owned();
    assert_eq!(notice.chars().count(), sf_video::compositor::TEXT_COLS - 2, "{notice}");
    assert!(notice.chars().all(sf_video::compositor::drawable), "{notice}");
    assert_eq!(notice, format!("LOOK FAILED: {}?", "A".repeat(32)), "the `é` is the last character, as `?`");
    s.look_failed("pro_monitor", "José/crt-ç.slangp: not found");
    assert_eq!(s.notice(), Some("LOOK FAILED: JOS?/CRT-?.SLANGP: NOT FOUND"));
}

/// Ticket #210 (`VO7`): the host re-reads its catalogue as Settings opens
/// and hands the session the new list; an open Settings screen's `CRT
/// LOOK` steps through it at once, drop-ins after the built-in looks.
#[test]
fn a_new_look_list_reaches_an_open_settings_screen() {
    let Some(mut s) = session_on_crt_look() else { return };
    let mut list = looks();
    list.push(LookName { id: "crt/neon.slangp".into(), name: "NEON".into() });
    s.set_looks(list);
    s.set_looks(Vec::new());
    let mut seen = Vec::new();
    for _ in 0..3 {
        seen.push(tap(&mut s, with(|i| i.p1.directions[0] = true)).video.look);
    }
    assert_eq!(seen, ["pro_monitor", "crt/neon.slangp", "off"], "the drop-in follows the built-ins; an empty list is ignored");
}
