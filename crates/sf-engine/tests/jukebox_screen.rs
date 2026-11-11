//! Ticket #202 (`ST10`): the Jukebox screen as a pure screen. It lists the
//! global track, every stage the stage select shows and every fighter; each
//! is set to `NATIVE` or to a file of the `jukebox` folder (which the host
//! lists: the engine reads no folder), only files of the formats the host
//! plays are offered, `NATIVE` on a stage or fighter overrides the global
//! track and is kept, and the tracks survive the settings file.
//!
//! The same screen through the session, with fights, is in
//! `jukebox_in_fights.rs`. Nothing here needs the ROMs.

use sf_engine::front::jukebox::{entries, listed_tracks, shown_name, Entry, JukeboxScreen, Row};
use sf_engine::front::settings::{rows, Row as SettingsRow, Settings, SettingsScreen, Tab};
use sf_engine::front::{MenuItem, Transition};
use sf_engine::jukebox::{Jukebox, Pick, TrackFormats, TRACK_EXTENSIONS};
use sf_engine::roster::{MenuInput, TournamentFighter};
use sf_engine::config::MatchConfig;
use sf_video::background_bank::{BackgroundBank, LayerView, StageView, RELEASED_WAVES};

fn files() -> Vec<String> {
    ["zeta.flac", "Alpha.ogg", "beta.WAV", "gamma.mp3", "notes.txt", "cover.png", ".hidden.ogg", "alpha.ogg", "Alpha.ogg", "noext"].map(String::from).to_vec()
}

fn entry(key: &str) -> Entry {
    Entry { key: key.into(), label: key.to_uppercase() }
}

/// A screen over two stages and two fighters, with the files above.
fn screen(jukebox: Jukebox) -> JukeboxScreen {
    JukeboxScreen::open(jukebox, vec![entry("retsu"), entry("geki")], vec![entry("Ryu"), entry("Ken")], files())
}

/// The cursor on the row labelled `label`.
fn goto(s: &mut JukeboxScreen, label: &str) {
    for _ in 0..30 {
        if s.label(s.row()) == label {
            return;
        }
        s.apply(MenuInput::Down);
    }
    panic!("a row labelled {label}");
}

/// The values the row steps through, once round (it wraps), from where it is.
fn ring(s: &mut JukeboxScreen) -> Vec<String> {
    let row = s.row();
    let first = s.value(row);
    let mut out = vec![first.clone()];
    loop {
        s.apply(MenuInput::Right);
        let value = s.value(row);
        if value == first {
            return out;
        }
        out.push(value);
        assert!(out.len() < 50, "the ring wraps");
    }
}

#[test]
fn the_list_shows_native_and_only_files_of_the_supported_formats() {
    // The host's folder listing, filtered by the one list of extensions,
    // each name once, in name order without regard to case.
    let listed = listed_tracks(files(), &TrackFormats::default());
    assert_eq!(listed, ["Alpha.ogg", "alpha.ogg", "beta.WAV", "zeta.flac"]);
    for file in &listed {
        assert!(TRACK_EXTENSIONS.iter().any(|e| file.to_lowercase().ends_with(e)), "{file}");
    }
    let mut s = screen(Jukebox::default());
    assert_eq!(s.tracks(), listed, "the screen filters the same way");
    // Every row steps through NATIVE then those files, and only those.
    for label in ["GLOBAL TRACK", "RETSU", "KEN"] {
        goto(&mut s, label);
        assert_eq!(ring(&mut s), ["NATIVE", "ALPHA.OGG", "ALPHA.OGG", "BETA.WAV", "ZETA.FLAC"], "{label}");
    }
    // A host that lists a folder with nothing playable gets NATIVE alone, and says what goes in it.
    let mut empty = JukeboxScreen::open(Jukebox::default(), vec![entry("retsu")], vec![entry("Ryu")], vec!["a.mp3".into()]);
    assert_eq!(ring(&mut empty), ["NATIVE"]);
    assert_eq!(empty.apply(MenuInput::Right), Some(Transition::ValueChanged));
    assert!(empty.jukebox().is_empty(), "stepping among one choice sets nothing");
    // The host lists the folder again: the open screen takes it.
    empty.set_tracks(vec!["one.ogg".into(), "skip.mid".into()]);
    assert_eq!(ring(&mut empty), ["NATIVE", "ONE.OGG"]);
}

#[test]
fn tracker_modules_are_offered_only_when_the_host_says_it_plays_them() {
    let files = || vec!["song.xm".to_owned(), "a.ogg".to_owned(), "tune.MOD".to_owned()];
    // A host with no tracker player: the audio formats alone, and the notice names only them.
    let mut plain = JukeboxScreen::open(Jukebox::default(), vec![entry("retsu")], vec![entry("Ryu")], files());
    assert_eq!(plain.tracks(), ["a.ogg"]);
    assert_eq!(ring(&mut plain), ["NATIVE", "A.OGG"]);
    let none = JukeboxScreen::open(Jukebox::default(), vec![entry("retsu")], vec![entry("Ryu")], vec!["song.xm".into()]);
    assert_eq!(none.notice(), ["NO TRACKS: PUT OGG, WAV OR FLAC FILES", "IN THE JUKEBOX FOLDER BESIDE THE ROMS"]);
    // A host with one: the modules are choices, and the notice says modules go in the folder too.
    let formats = TrackFormats::new(["ogg", "wav", "flac", "xm", "mod"].map(String::from));
    let mut s = JukeboxScreen::open_with(Jukebox::default(), vec![entry("retsu")], vec![entry("Ryu")], files(), formats.clone());
    assert_eq!(s.tracks(), ["a.ogg", "song.xm", "tune.MOD"]);
    assert_eq!(ring(&mut s), ["NATIVE", "A.OGG", "SONG.XM", "TUNE.MOD"]);
    s.set_tracks(vec!["only.s3m".into()]);
    assert!(s.tracks().is_empty(), "a format the host does not list is not offered");
    let empty = JukeboxScreen::open_with(Jukebox::default(), Vec::new(), vec![entry("Ryu")], Vec::new(), formats);
    assert_eq!(empty.notice()[0], "NO TRACKS: PUT OGG, WAV, FLAC OR MODULE FILES");
}

#[test]
fn a_stage_a_fighter_and_the_global_track_are_set_from_the_list() {
    let mut s = screen(Jukebox::default());
    assert_eq!(s.row(), Row::Global);
    // Right once: the first file.
    assert_eq!(s.apply(MenuInput::Right), Some(Transition::ValueChanged));
    assert_eq!(s.jukebox().global(), Some("Alpha.ogg"));
    // A stage: two files along; a fighter: left from NATIVE wraps to the last.
    goto(&mut s, "GEKI");
    s.apply(MenuInput::Right);
    s.apply(MenuInput::Right);
    goto(&mut s, "KEN");
    s.apply(MenuInput::Left);
    // With the global track set the entries step GLOBAL, NATIVE, the files:
    // Geki (GLOBAL) went two along, to the first file; Ken (GLOBAL) went
    // left, wrapping to the last.
    assert_eq!(s.jukebox().stage("geki"), Some("Alpha.ogg"));
    assert_eq!(s.jukebox().fighter("Ken"), Some("zeta.flac"));
    assert_eq!(s.jukebox().stage("retsu"), None);
    // Confirm steps on, like right.
    goto(&mut s, "RYU");
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert_eq!(s.jukebox().fighter_pick("Ryu"), Some(&Pick::Native), "GLOBAL, then NATIVE");
    // A tab press steps ten choices: from NATIVE round a ring of six
    // (GLOBAL, NATIVE, four files) lands on the last file.
    assert_eq!(s.apply(MenuInput::TabRight), Some(Transition::ValueChanged));
    assert_eq!(s.jukebox().fighter("Ryu"), Some("zeta.flac"));
    // Back leaves, and the tracks are in the screen's copy.
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::Back));
}

#[test]
fn native_on_a_stage_or_fighter_overrides_the_global_track_and_is_kept() {
    let mut j = Jukebox::default();
    j.set_global("all.ogg");
    let mut s = screen(j);
    // With a global track set, an entry with nothing shows GLOBAL and the
    // ring starts there: GLOBAL, NATIVE, the files.
    goto(&mut s, "RETSU");
    assert_eq!(ring(&mut s)[..2], ["GLOBAL", "NATIVE"]);
    assert!(s.jukebox().stage_pick("retsu").is_none(), "stepping round changed nothing in the end");
    // One step right: NATIVE, stored, and it wins over the global track.
    s.apply(MenuInput::Right);
    assert_eq!(s.value(s.row()), "NATIVE");
    assert_eq!(s.jukebox().stage_pick("retsu"), Some(&Pick::Native));
    assert!(s.jukebox().resolve(Some("retsu"), Some("Ryu"), Some("Ken")).is_none(), "the native theme plays on Retsu's stage");
    assert_eq!(s.jukebox().resolve(Some("geki"), Some("Ryu"), Some("Ken")).unwrap().file, "all.ogg", "other stages keep the global track");
    // The same for a fighter, who beats the stage and the global track.
    goto(&mut s, "KEN");
    s.apply(MenuInput::Right);
    assert_eq!(s.jukebox().fighter_pick("Ken"), Some(&Pick::Native));
    assert!(s.jukebox().resolve(Some("geki"), Some("Ryu"), Some("Ken")).is_none());
    // Back to GLOBAL removes the entry again.
    s.apply(MenuInput::Left);
    assert_eq!(s.jukebox().fighter_pick("Ken"), None);
    assert_eq!(s.value(s.row()), "GLOBAL");
    // NATIVE is stored in the settings file and read back.
    let settings = Settings { jukebox: s.jukebox().clone(), ..Settings::default() };
    let text = settings.to_toml();
    assert!(text.contains("[jukebox.stages]\n\"retsu\" = \"NATIVE\""), "{text}");
    let back = Settings::from_toml(&text).unwrap();
    assert_eq!(back.jukebox, *s.jukebox());
    assert_eq!(back.jukebox.stage_pick("retsu"), Some(&Pick::Native));
}

#[test]
fn without_a_global_track_an_entry_with_none_shows_native_and_a_chosen_native_stays() {
    let mut s = screen(Jukebox::default());
    goto(&mut s, "RETSU");
    assert_eq!(s.value(s.row()), "NATIVE", "what plays");
    // Right to a file, left back to NATIVE: that is a choice, kept.
    s.apply(MenuInput::Right);
    s.apply(MenuInput::Left);
    assert_eq!(s.jukebox().stage_pick("retsu"), Some(&Pick::Native));
    // Setting the global track afterwards leaves Retsu on NATIVE.
    goto(&mut s, "GLOBAL TRACK");
    s.apply(MenuInput::Right);
    assert!(s.jukebox().resolve(Some("retsu"), None, None).is_none());
    assert!(s.jukebox().resolve(Some("geki"), None, None).is_some());
    // The global row's NATIVE clears it.
    s.apply(MenuInput::Left);
    assert_eq!(s.jukebox().global(), None);
}

#[test]
fn a_file_the_folder_lacks_stays_shown_until_the_entry_changes_and_says_so() {
    let mut j = Jukebox::default();
    j.set_stage("retsu", "gone.ogg");
    let mut s = screen(j);
    // Untouched, it is not lost: it shows, the settings keep it, a notice says it is missing.
    goto(&mut s, "RETSU");
    assert_eq!(s.value(s.row()), "GONE.OGG");
    assert_eq!(s.jukebox().stage("retsu"), Some("gone.ogg"));
    assert_eq!(s.notice(), ["THIS FILE IS NOT IN THE JUKEBOX FOLDER"]);
    goto(&mut s, "KEN");
    assert!(s.notice().is_empty(), "the notice is the cursor row's");
    // It is the last choice on the ring; stepping off it leaves it.
    goto(&mut s, "RETSU");
    s.apply(MenuInput::Left);
    assert_eq!(s.value(s.row()), "ZETA.FLAC");
    assert_eq!(s.jukebox().stage("retsu"), Some("zeta.flac"));
    assert!(s.notice().is_empty());
}

#[test]
fn an_empty_folder_says_what_goes_in_it() {
    let s = JukeboxScreen::open(Jukebox::default(), vec![entry("retsu")], vec![entry("Ryu")], vec!["a.mp3".into()]);
    assert_eq!(s.notice(), ["NO TRACKS: PUT OGG, WAV OR FLAC FILES", "IN THE JUKEBOX FOLDER BESIDE THE ROMS"]);
    let s = screen(Jukebox::default());
    assert!(s.notice().is_empty());
}

#[test]
fn clear_all_puts_every_entry_back() {
    let mut j = Jukebox::default();
    j.set_global("all.ogg");
    j.set_stage("retsu", "a.ogg");
    j.set_fighter_native("Ken");
    let mut s = screen(j);
    goto(&mut s, "CLEAR ALL");
    assert_eq!(s.apply(MenuInput::Right), None, "left and right leave it alone");
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert!(s.jukebox().is_empty());
}

#[test]
fn up_and_down_skip_the_headings_and_wrap() {
    let mut s = screen(Jukebox::default());
    let rows = s.rows();
    assert_eq!(
        rows,
        [
            Row::Global,
            Row::Heading("STAGES"),
            Row::Stage(0),
            Row::Stage(1),
            Row::Heading("FIGHTERS"),
            Row::Fighter(0),
            Row::Fighter(1),
            Row::ClearAll
        ]
    );
    let mut seen = vec![s.row()];
    for _ in 0..6 {
        s.apply(MenuInput::Down);
        seen.push(s.row());
    }
    let live: Vec<Row> = rows.iter().copied().filter(|r| !matches!(r, Row::Heading(_))).collect();
    assert_eq!(seen, [live.clone(), vec![live[0]]].concat(), "down wraps past CLEAR ALL to the top");
    assert!(seen.iter().all(|r| !matches!(r, Row::Heading(_))));
    s.apply(MenuInput::Up);
    assert_eq!(s.row(), Row::ClearAll, "up wraps from the top");
    // A heading is left out with nothing under it.
    let bare = JukeboxScreen::open(Jukebox::default(), Vec::new(), vec![entry("Ryu")], Vec::new());
    assert_eq!(bare.rows(), [Row::Global, Row::Heading("FIGHTERS"), Row::Fighter(0), Row::ClearAll]);
}

fn listing_bank() -> BackgroundBank {
    let mut bank = BackgroundBank::new();
    let layer = LayerView { page: 0, scroll: 0, palette: 0 };
    let view = StageView { fg: layer, bg: layer, theme: 0 };
    // The test stage, which no screen lists, then two 1987 stages and a
    // challenger's, which ships with wave 1.
    let _test = bank.add_stage("test-stage", view);
    let retsu = bank.add_stage("retsu", view);
    let geki = bank.add_stage("geki", view);
    let akuma = bank.add_stage("akuma", view);
    bank.list_stage(retsu, "Retsu", "Japan");
    bank.list_stage(geki, "Geki", "Japan");
    bank.list_wave_stage(akuma, "Akuma", "Japan", 1);
    bank
}

#[test]
fn every_shown_stage_and_every_fighter_is_listed_released_challengers_included() {
    let bank = listing_bank();
    let keys = |list: &[Entry]| list.iter().map(|e| e.key.clone()).collect::<Vec<_>>();
    let twelve: Vec<String> = TournamentFighter::ALL.iter().map(|f| f.name().to_owned()).collect();
    // No wave has shipped: the stages the select shows, never the test
    // stage, and the twelve.
    assert_eq!(RELEASED_WAVES, 0);
    for bonus in [false, true] {
        let (stages, fighters) = entries(Some(&bank), bonus, RELEASED_WAVES);
        assert_eq!(keys(&stages), ["retsu", "geki"], "bonus {bonus}");
        assert_eq!(keys(&fighters), twelve, "bonus {bonus}");
    }
    // Wave 1 shipped: its stage and its challenger come with the switch.
    let (stages, fighters) = entries(Some(&bank), false, 1);
    assert_eq!((keys(&stages), keys(&fighters)), (vec!["retsu".to_owned(), "geki".to_owned()], twelve.clone()));
    let (stages, fighters) = entries(Some(&bank), true, 1);
    assert_eq!(keys(&stages), ["retsu", "geki", "akuma"]);
    assert_eq!(keys(&fighters), [twelve.clone(), vec!["Akuma".to_owned()]].concat());
    assert_eq!(stages[2].label, "AKUMA", "drawn as the stage select spells it");
    // No background bank (no ROM stages): the fighters alone.
    let (stages, fighters) = entries(None, true, 1);
    assert!(stages.is_empty());
    assert_eq!(keys(&fighters), twelve);
}

#[test]
fn file_names_are_drawn_in_the_rom_fonts_characters() {
    assert_eq!(shown_name("boss_theme.ogg"), "BOSS THEME.OGG");
    assert_eq!(shown_name("Stage (2) - Final.flac"), "STAGE (2) - FINAL.FLAC");
    assert_eq!(shown_name("caf\u{e9} \u{30c6}.wav"), "CAF\u{c9} ?.WAV".replace('\u{c9}', "?"));
}

#[test]
fn the_sound_tabs_jukebox_row_opens_the_screen() {
    let sound = rows(Tab::Sound);
    assert!(sound.contains(&SettingsRow::Jukebox), "{sound:?}");
    let mut s = SettingsScreen::open(MatchConfig::default(), Settings::default());
    for _ in 0..3 {
        s.apply(MenuInput::TabRight);
    }
    while s.row() != SettingsRow::Jukebox {
        s.apply(MenuInput::Down);
    }
    assert_eq!(s.label(s.row()), "JUKEBOX");
    assert_eq!(s.apply(MenuInput::Left), None);
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::Choose(MenuItem::Jukebox)));
    // Back from the Jukebox screen hands the tracks to Settings.
    let mut j = Jukebox::default();
    j.set_stage_native("retsu");
    s.set_jukebox(j.clone());
    assert_eq!(s.settings().jukebox, j);
    // RESET TAB on Sound resets the volumes, not the tracks.
    while s.row() != SettingsRow::ResetTab {
        s.apply(MenuInput::Down);
    }
    s.apply(MenuInput::Confirm);
    assert_eq!(s.settings().jukebox, j);
}

#[test]
fn the_choices_survive_a_restart() {
    let mut j = Jukebox::default();
    j.set_global("main theme.ogg");
    j.set_stage("retsu", "dojo.flac");
    j.set_stage_native("geki");
    j.set_fighter("Ken", "ken \"live\".wav");
    j.set_fighter_native("Ryu");
    let mut s = screen(j);
    // Change something through the screen too.
    goto(&mut s, "RETSU");
    s.apply(MenuInput::Right);
    let saved = Settings { jukebox: s.jukebox().clone(), master: 7, ..Settings::default() };
    let path = std::env::temp_dir().join(format!("sf1-port-jukebox-screen-{}.toml", std::process::id()));
    saved.save(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let (loaded, warnings) = Settings::from_toml_lenient(&text).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(loaded, saved, "{text}");
    // A screen opened on what was loaded shows the same choices.
    let again = screen(loaded.jukebox);
    for label in ["GLOBAL TRACK", "RETSU", "GEKI", "RYU", "KEN"] {
        let (mut a, mut b) = (s.clone(), again.clone());
        goto(&mut a, label);
        goto(&mut b, label);
        assert_eq!(a.value(a.row()), b.value(b.row()), "{label}");
    }
}
