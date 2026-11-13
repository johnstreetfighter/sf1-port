//! Ticket #213 (`VO10`): saved looks, the engine's side. `SAVE AS >` on the
//! tuning screen asks for a name (suggesting `ARCADE CABINET 2`), refuses the
//! names the built-in looks and drop-ins have, asks before replacing a saved
//! look, and hands the host a request to write it; leaving with changes
//! asks whether to save them first, and not otherwise. Through the session,
//! a saved look becomes `CRT LOOK` and the tuning screen goes on to it.
//! (The `.slangp` is the host's: `sf-host`'s `shaders` tests.)

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::front::settings::{LookName, Settings, DEFAULT_LOOK};
use sf_engine::front::tune::{is_saved, name_problem, LookParam, SaveLook, TuneRow, TuneScreen, TypedKey, NAME_MAX};
use sf_engine::front::{FrameInputs, GameSession, MenuKeys, Transition};
use sf_engine::roster::MenuInput;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn param(id: &str, description: &str, initial: f32, min: f32, max: f32, step: f32) -> LookParam {
    LookParam { id: id.into(), description: description.into(), initial, min, max, step }
}

fn params() -> Vec<LookParam> {
    vec![
        param("bogus_scan", "[ SCANLINE OPTIONS ]:", 0.0, 0.0, 1.0, 1.0),
        param("gsl", "          Scanline Type", 0.0, -1.0, 2.0, 1.0),
        param("beam_min", "          Scanline Beam Shape Center", 1.4, 0.25, 5.0, 0.05),
    ]
}

fn look(id: &str, name: &str) -> LookName {
    LookName { id: id.into(), name: name.into() }
}

/// `OFF`, the built-ins, then `saved` (names), then a drop-in.
fn list(saved: &[&str]) -> Vec<LookName> {
    let mut out = vec![
        look("off", "OFF"),
        look("arcade_cabinet", "ARCADE CABINET"),
        look("consumer_tv", "CONSUMER TV"),
        look("pro_monitor", "PRO MONITOR"),
        look("curved_tube", "CURVED TUBE"),
    ];
    out.extend(saved.iter().map(|n| look(&format!("saved/{n}.slangp"), n)));
    out.push(look("crt/crt-royale.slangp", "CRT-ROYALE"));
    out
}

fn screen(saved: &[&str]) -> TuneScreen {
    TuneScreen::open("arcade_cabinet", "ARCADE CABINET", params(), &[]).with_looks(&list(saved))
}

/// Press `inputs` in turn, each of which the screen takes.
fn press(s: &mut TuneScreen, inputs: &[MenuInput]) {
    for &input in inputs {
        assert!(s.apply(input).is_some(), "{input:?} is taken");
    }
}

/// On `SAVE AS >`, and into name entry.
fn open_name(s: &mut TuneScreen) {
    while s.row() != TuneRow::SaveAs {
        s.apply(MenuInput::Down);
    }
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::CursorMoved));
}

#[test]
fn leaving_with_changes_asks_whether_to_save_and_not_otherwise() {
    // Nothing changed: back leaves.
    let mut s = screen(&[]);
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::Back));
    assert!(!s.asking());
    // A knob changed and put back by hand: nothing to save.
    let mut s = screen(&[]);
    s.apply(MenuInput::Right);
    s.apply(MenuInput::Left);
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::Back), "no change left to save");
    // RESET LOOK: the same.
    let mut s = screen(&[]);
    s.apply(MenuInput::Right);
    while s.row() != TuneRow::ResetLook {
        s.apply(MenuInput::Down);
    }
    s.apply(MenuInput::Confirm);
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::Back));
    // A change: back asks, and NO (or back again) leaves, the changes kept.
    let mut s = screen(&[]);
    s.apply(MenuInput::Right);
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::CursorMoved));
    assert!(s.asking(), "the question shows");
    assert!(s.take_save().is_none());
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::Back), "back answers NO");
    assert_eq!(s.overrides(), [("gsl".to_owned(), 1.0)], "the changes stay the look's for the session");
    // NO on the question: the same.
    let mut s = screen(&[]);
    s.apply(MenuInput::Right);
    s.apply(MenuInput::Back);
    s.apply(MenuInput::Right);
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::Back));
    // A screen opened on earlier tuning, left untouched, does not ask again.
    let mut s = TuneScreen::open("arcade_cabinet", "ARCADE CABINET", params(), &[("gsl".into(), 1.0)]);
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::Back));
    // YES goes to the name.
    let mut s = screen(&[]);
    s.apply(MenuInput::Right);
    s.apply(MenuInput::Back);
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::CursorMoved));
    assert_eq!(s.entry().as_deref(), Some("ARCADE CABINET 2"));
}

#[test]
fn the_first_name_is_the_looks_own_with_the_first_free_number() {
    assert_eq!(screen(&[]).suggested_name(), "ARCADE CABINET 2");
    assert_eq!(screen(&["ARCADE CABINET 2"]).suggested_name(), "ARCADE CABINET 3");
    assert_eq!(screen(&["ARCADE CABINET 3"]).suggested_name(), "ARCADE CABINET 2", "the first free number");
    // A saved look being tuned suggests its own name's next number.
    let s = TuneScreen::open("saved/ARCADE CABINET 2.slangp", "ARCADE CABINET 2", params(), &[]).with_looks(&list(&["ARCADE CABINET 2"]));
    assert_eq!(s.suggested_name(), "ARCADE CABINET 3");
    // A name too long for its number is cut to fit; one with characters the
    // entry can't hold is made of the rest.
    let long = TuneScreen::open("x", "A VERY LONG DROP-IN LOOK NAME", params(), &[]);
    let name = long.suggested_name();
    assert!(name.chars().count() <= NAME_MAX && name.ends_with(" 2"), "{name}");
    assert_eq!(TuneScreen::open("x", "CRT/ROYALE_2", params(), &[]).suggested_name(), "CRT ROYALE 2");
    assert_eq!(TuneScreen::open("x", "~~~", params(), &[]).suggested_name(), "MY LOOK 2");
}

#[test]
fn name_entry_steps_characters_like_the_arcades_initials() {
    let mut s = screen(&[]);
    open_name(&mut s);
    // The cursor is past the last character: Start deletes the 2, up adds an
    // A and steps it on to B.
    press(&mut s, &[MenuInput::Start, MenuInput::Up, MenuInput::Up]);
    assert_eq!(s.entry().as_deref(), Some("ARCADE CABINET B"));
    // Up to C; down is the other way round, through B and A to
    // the space, and on round the end of the list.
    press(&mut s, &[MenuInput::Up]);
    assert_eq!(s.entry().as_deref(), Some("ARCADE CABINET C"));
    press(&mut s, &[MenuInput::Down, MenuInput::Down, MenuInput::Down]);
    assert_eq!(s.entry().as_deref(), Some("ARCADE CABINET  "));
    press(&mut s, &[MenuInput::Down]);
    assert_eq!(s.entry().as_deref(), Some("ARCADE CABINET ,"), "round to the last character");
    press(&mut s, &[MenuInput::Up, MenuInput::Up]);
    assert_eq!(s.entry().as_deref(), Some("ARCADE CABINET A"), "and round to the first");
    // Start deletes under the cursor and the rest closes up.
    press(&mut s, &[MenuInput::Left, MenuInput::Left, MenuInput::Start]);
    assert_eq!(s.entry().as_deref(), Some("ARCADE CABINE A"));
}

#[test]
fn name_entry_stays_inside_the_name_and_its_characters() {
    // Grow to the longest name: a character, a step right, and so on.
    let mut s = screen(&[]);
    open_name(&mut s);
    for _ in 0..10 {
        s.apply(MenuInput::Up);
        s.apply(MenuInput::Right);
    }
    assert_eq!(s.entry().unwrap().chars().count(), NAME_MAX);
    // No further than the last place, and none before the first.
    assert_eq!(s.apply(MenuInput::Right), None);
    for _ in 0..NAME_MAX - 1 {
        s.apply(MenuInput::Left);
    }
    assert_eq!(s.apply(MenuInput::Left), None);
    // Deleting it all leaves an empty name, which cannot be saved.
    for _ in 0..40 {
        s.apply(MenuInput::Start);
    }
    assert_eq!(s.entry().as_deref(), Some(""));
    assert_eq!(s.apply(MenuInput::Start), None);
    // Back cancels to the list, nothing asked of the host.
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::CursorMoved));
    assert!(s.entry().is_none() && s.take_save().is_none());
}

#[test]
fn confirming_a_name_asks_the_host_to_save_the_tuned_look() {
    let mut s = screen(&[]);
    s.apply(MenuInput::Right);
    s.apply(MenuInput::Down);
    s.apply(MenuInput::Right);
    open_name(&mut s);
    press(&mut s, &[MenuInput::Start, MenuInput::Up, MenuInput::Up]);
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::SaveLook));
    assert_eq!(
        s.take_save(),
        Some(SaveLook {
            look: "arcade_cabinet".into(),
            name: "ARCADE CABINET B".into(),
            overrides: vec![("gsl".into(), 1.0), ("beam_min".into(), 1.45)],
        })
    );
    assert!(s.take_save().is_none(), "once");
    assert!(s.entry().is_none(), "back on the list while the host writes it");
    // Saved is not a change left to ask about.
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::Back));
}

#[test]
fn the_suggested_name_saves_as_it_stands_and_spaces_are_tidied() {
    let mut s = screen(&[]);
    open_name(&mut s);
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::SaveLook));
    let save = s.take_save().unwrap();
    assert_eq!((save.name.as_str(), save.overrides.is_empty()), ("ARCADE CABINET 2", true));
    // Spaces at the ends go and a run of them becomes one.
    let mut s = screen(&[]);
    open_name(&mut s);
    type_name(&mut s, " A  B ");
    assert_eq!(s.entry().as_deref(), Some(" A  B "));
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::SaveLook));
    assert_eq!(s.take_save().unwrap().name, "A B");
}

#[test]
fn names_the_other_looks_have_are_refused_and_a_saved_looks_asks_before_replacing() {
    // A built-in look's, OFF's or a drop-in's name.
    for taken in ["ARCADE CABINET", "OFF", "CRT-ROYALE", "PRO MONITOR"] {
        let mut s = screen(&[]);
        open_name(&mut s);
        type_name(&mut s, taken);
        assert_eq!(s.apply(MenuInput::Confirm), None, "{taken}");
        assert_eq!(s.message(), Some("THAT NAME IS TAKEN"), "{taken}");
        assert!(s.take_save().is_none() && s.entry().is_some());
    }
    // Empty, and the names Windows keeps.
    let mut s = screen(&[]);
    open_name(&mut s);
    for _ in 0..20 {
        s.apply(MenuInput::Start);
    }
    assert_eq!(s.apply(MenuInput::Confirm), None);
    assert_eq!(s.message(), Some("GIVE IT A NAME"));
    type_name(&mut s, "CON");
    assert_eq!(s.apply(MenuInput::Confirm), None);
    assert_eq!(s.message(), Some("THAT NAME CAN'T BE USED"));
    assert_eq!(name_problem("NUL"), Some("THAT NAME CAN'T BE USED"));
    assert_eq!(name_problem("COM1"), Some("THAT NAME CAN'T BE USED"));
    assert_eq!(name_problem("CONSOLE"), None);
    assert_eq!(name_problem("MY LOOK (2)"), None);
    assert_eq!(name_problem("A/B"), Some("THAT NAME CAN'T BE USED"));
    assert_eq!(name_problem("a b"), Some("THAT NAME CAN'T BE USED"));
    assert_eq!(name_problem(" A"), Some("THAT NAME CAN'T BE USED"));
    assert_eq!(name_problem("A  B"), Some("THAT NAME CAN'T BE USED"));
    assert_eq!(name_problem(&"X".repeat(NAME_MAX + 1)), Some("THAT NAME IS TOO LONG"));
    assert_eq!(name_problem(&"X".repeat(NAME_MAX)), None);

    // A saved look's name: the question defaults to NO, which goes back to
    // the name; YES asks for the save.
    let mut s = screen(&["NEON"]);
    s.apply(MenuInput::Right);
    open_name(&mut s);
    type_name(&mut s, "NEON");
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::CursorMoved));
    assert!(s.replacing() && s.take_save().is_none());
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::CursorMoved), "NO is chosen at first");
    assert_eq!(s.entry().as_deref(), Some("NEON"));
    s.apply(MenuInput::Confirm);
    s.apply(MenuInput::Right);
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::SaveLook));
    let save = s.take_save().unwrap();
    assert_eq!((save.name.as_str(), save.overrides.len()), ("NEON", 1));
    // The same, any case: the names are upper case, as they are listed.
    assert!(is_saved("saved/NEON.slangp") && !is_saved("crt/saved/x.slangp") && !is_saved("saved"));
}

/// Clear the name and enter `name` (its characters by stepping from the
/// space).
fn type_name(s: &mut TuneScreen, name: &str) {
    for _ in 0..NAME_MAX {
        s.apply(MenuInput::Start);
    }
    let chars: Vec<char> = sf_engine::front::tune::ENTRY_CHARS.chars().collect();
    for c in name.chars() {
        s.apply(MenuInput::Up);
        let mut guard = 0;
        // Up adds an A; step from it to `c`.
        while s.entry().unwrap().chars().last() != Some(c) {
            s.apply(MenuInput::Up);
            guard += 1;
            assert!(guard <= chars.len(), "{c} is enterable");
        }
        s.apply(MenuInput::Right);
    }
}

/// Types `text` into the open name entry, a key at a time.
fn type_text(s: &mut TuneScreen, text: &str) {
    for c in text.chars() {
        s.type_key(TypedKey::Char(c));
    }
}

/// Clears the open name entry with Backspace from its end.
fn clear(s: &mut TuneScreen) {
    s.type_key(TypedKey::End);
    for _ in 0..NAME_MAX {
        s.type_key(TypedKey::Erase);
    }
    assert_eq!(s.entry().as_deref(), Some(""));
}

#[test]
fn the_keyboard_types_a_name() {
    let mut s = screen(&[]);
    open_name(&mut s);
    clear(&mut s);
    // Letters go in upper-cased; characters the entry can't hold don't.
    type_text(&mut s, "my look-2!:é");
    assert_eq!(s.entry().as_deref(), Some("MY LOOK-2!"));
    // The arrows move the place; Backspace takes the character before it
    // and Delete the one at it.
    assert_eq!(s.type_key(TypedKey::Left), Some(Transition::CursorMoved));
    s.type_key(TypedKey::Left);
    assert_eq!(s.type_key(TypedKey::Erase), Some(Transition::CursorMoved));
    assert_eq!(s.entry().as_deref(), Some("MY LOOK2!"));
    s.type_key(TypedKey::EraseAhead);
    assert_eq!(s.entry().as_deref(), Some("MY LOOK!"));
    // Home and End jump; a character goes in at the place.
    s.type_key(TypedKey::Home);
    type_text(&mut s, "a");
    s.type_key(TypedKey::End);
    type_text(&mut s, "1");
    assert_eq!(s.entry().as_deref(), Some("AMY LOOK!1"));
    assert_eq!(s.type_key(TypedKey::Home), Some(Transition::CursorMoved));
    assert_eq!(s.type_key(TypedKey::Home), None, "already there");
    assert_eq!(s.type_key(TypedKey::Erase), None, "nothing before the first place");
    // Enter saves.
    assert_eq!(s.type_key(TypedKey::Enter), Some(Transition::SaveLook));
    assert_eq!(s.take_save().unwrap().name, "AMY LOOK!1");
    assert_eq!(s.type_key(TypedKey::Char('A')), None, "the list takes no typing");
    assert_eq!(s.type_key(TypedKey::Enter), None);
    // Esc cancels.
    let mut s = screen(&[]);
    open_name(&mut s);
    type_text(&mut s, "x");
    assert_eq!(s.type_key(TypedKey::Escape), Some(Transition::CursorMoved));
    assert!(s.entry().is_none() && s.take_save().is_none());
}

#[test]
fn a_full_name_takes_no_more_and_backspace_takes_its_last() {
    let mut s = screen(&[]);
    open_name(&mut s);
    clear(&mut s);
    type_text(&mut s, &"AB".repeat(NAME_MAX));
    let full = "AB".repeat(NAME_MAX / 2);
    assert_eq!(s.entry(), Some(full.clone()));
    assert_eq!(s.type_key(TypedKey::Char('C')), None, "full");
    s.type_key(TypedKey::Erase);
    assert_eq!(s.entry().as_deref(), Some(&full[..NAME_MAX - 1]));
    type_text(&mut s, "Z");
    assert_eq!(s.entry(), Some(format!("{}Z", &full[..NAME_MAX - 1])));
    // The arrows and the steps still work as before.
    s.type_key(TypedKey::Home);
    s.type_key(TypedKey::Up);
    assert!(s.entry().unwrap().starts_with('B'));
}

#[test]
fn a_failed_save_comes_back_to_the_list_with_why() {
    let mut s = screen(&[]);
    open_name(&mut s);
    s.apply(MenuInput::Confirm);
    s.take_save();
    s.save_failed("COULD NOT SAVE THE LOOK");
    assert_eq!(s.message(), Some("COULD NOT SAVE THE LOOK"));
    assert!(s.entry().is_none());
}

#[test]
fn the_panel_draws_the_question_and_the_name_entry() {
    let Some(assets) = assets() else { return };
    let font = sf_video::compositor::RomFont::new(&assets.gfx4, &assets.maincpu);
    let lit = |s: &TuneScreen| s.panel(&font).unwrap().argb;
    let mut s = screen(&[]);
    s.apply(MenuInput::Right);
    let list = lit(&s);
    s.apply(MenuInput::Back);
    let asking = lit(&s);
    assert!(asking != list, "the question replaces the list");
    s.apply(MenuInput::Right);
    let no = lit(&s);
    assert!(no != asking, "the chosen answer moves");
    s.apply(MenuInput::Left);
    s.apply(MenuInput::Confirm);
    let name = lit(&s);
    assert!(name != asking && name != no && name != list, "the name entry");
    s.apply(MenuInput::Up);
    assert!(lit(&s) != name, "a character change shows");
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

fn tap(s: &mut GameSession, input: FrameInputs) -> sf_engine::front::FrameOutput {
    let out = s.step(input);
    s.step(FrameInputs::default());
    out
}

const DOWN: fn(&mut FrameInputs) = |i| i.p1.directions[2] = true;
const RIGHT: fn(&mut FrameInputs) = |i| i.p1.directions[0] = true;

/// The session on the tuning screen of `ARCADE CABINET`, its parameters in.
fn session_on_tune() -> Option<GameSession> {
    let settings = Settings { crt_look: DEFAULT_LOOK.into(), ..Settings::default() };
    let mut s = GameSession::new(assets()?, MatchConfig::default()).with_looks(list(&[])).with_settings(settings);
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(DOWN));
    tap(&mut s, with(DOWN));
    tap(&mut s, with(DOWN));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "settings");
    tap(&mut s, with(|i| i.menu.tab_right = true));
    tap(&mut s, with(|i| i.menu.tab_right = true));
    for _ in 0..5 {
        tap(&mut s, with(DOWN));
    }
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "tune");
    s.set_look_params(DEFAULT_LOOK, params());
    s.step(FrameInputs::default());
    Some(s)
}

#[test]
fn saving_makes_the_new_look_crt_look_and_the_tuning_screen_goes_on_to_it() {
    let Some(mut s) = session_on_tune() else { return };
    // Tune a knob, then SAVE AS: down past BEAM SHAPE to the foot.
    tap(&mut s, with(RIGHT));
    for _ in 0..2 {
        tap(&mut s, with(DOWN));
    }
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "tune");
    assert!(s.take_save_look().is_none(), "the name comes first");
    s.type_keys([TypedKey::Enter]);
    let out = tap(&mut s, FrameInputs::default());
    assert_eq!(out.transition, Some(Transition::SaveLook));
    let save = s.take_save_look().expect("the request for the host");
    assert_eq!(save.look, DEFAULT_LOOK);
    assert_eq!(save.name, "ARCADE CABINET 2");
    assert_eq!(save.overrides, [("gsl".to_owned(), 1.0)]);
    assert!(s.take_save_look().is_none(), "once");
    // The host wrote it and lists it: the answer makes it the look.
    s.set_looks(list(&["ARCADE CABINET 2"]));
    s.look_saved(Ok("saved/ARCADE CABINET 2.slangp".into()));
    let out = s.step(FrameInputs::default());
    assert_eq!(out.screen, "tune", "the tuning screen goes on, to the saved look");
    assert_eq!(out.video.look, "saved/ARCADE CABINET 2.slangp");
    assert!(s.look_tuning().is_none(), "the changes are the new look's own values");
    // Its parameters arrive once the host has loaded it, with the saved
    // value as the look's own.
    let mut saved = params();
    saved[1].initial = 1.0;
    s.set_look_params("saved/ARCADE CABINET 2.slangp", saved);
    // Leaving keeps it as the player's look: nothing left to ask.
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "settings");
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "main menu");
    assert_eq!(s.settings().crt_look, "saved/ARCADE CABINET 2.slangp");
    // The old look keeps its own values.
    assert!(s.look_tuning().is_none());
}

#[test]
fn a_save_that_fails_stays_on_the_tuning_screen_with_the_reason() {
    let Some(mut s) = session_on_tune() else { return };
    tap(&mut s, with(RIGHT));
    for _ in 0..2 {
        tap(&mut s, with(DOWN));
    }
    tap(&mut s, with(|i| i.menu.confirm = true));
    s.type_keys([TypedKey::Enter]);
    tap(&mut s, FrameInputs::default());
    assert!(s.take_save_look().is_some());
    s.look_saved(Err("COULD NOT SAVE THE LOOK".into()));
    let out = s.step(FrameInputs::default());
    assert_eq!(out.screen, "tune");
    assert_eq!(out.video.look, DEFAULT_LOOK, "CRT LOOK is unchanged");
    assert_eq!(s.look_tuning().map(|t| t.values.clone()), Some(vec![("gsl".to_owned(), 1.0)]), "the changes are still there");
}

#[test]
fn settings_toml_round_trips_a_saved_look() {
    let s = Settings { crt_look: "saved/MY LOOK (2).slangp".into(), ..Settings::default() };
    assert_eq!(Settings::from_toml(&s.to_toml()).unwrap().crt_look, "saved/MY LOOK (2).slangp");
}

/// While the name entry is open the keyboard types it: the keyboard's
/// menu keys and fighter keys (LP is confirm in a menu, Backspace back)
/// stand down, and a pad still steps it and saves it.
#[test]
fn the_keyboard_types_the_name_while_its_menu_keys_stand_down() {
    let Some(mut s) = session_on_tune() else { return };
    // Down past BEAM SHAPE to SAVE AS.
    for _ in 0..2 {
        tap(&mut s, with(DOWN));
    }
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "tune");
    s.type_keys(std::iter::repeat(TypedKey::Erase).take(NAME_MAX));
    s.type_keys("keys 1".chars().map(TypedKey::Char));
    tap(&mut s, FrameInputs::default());
    // The keyboard's confirm, back, LP, LK and stick do nothing here.
    for input in [
        with(|i| i.menu.confirm = true),
        with(|i| i.menu.back = true),
        with(|i| i.menu.start = true),
        with(|i| i.p1.buttons[0] = true),
        with(|i| i.p1.buttons[3] = true),
        with(DOWN),
    ] {
        let out = tap(&mut s, input);
        assert_eq!((out.screen, out.transition), ("tune", None), "{input:?}");
    }
    assert!(s.take_save_look().is_none());
    // A pad's stick adds a character at the blank place; its confirm saves.
    let pad = |f: fn(&mut FrameInputs)| {
        with(move |i| {
            i.pads[0] = Some(MenuKeys::default());
            f(i)
        })
    };
    tap(&mut s, pad(|i| i.p1.directions[3] = true));
    let out = tap(&mut s, pad(|i| i.pads[0].as_mut().unwrap().confirm = true));
    assert_eq!(out.transition, Some(Transition::SaveLook));
    assert_eq!(s.take_save_look().unwrap().name, "KEYS 1A");
    // Typing reaches nothing once the entry is closed.
    s.type_keys([TypedKey::Char('Q'), TypedKey::Escape]);
    assert_eq!(s.step(FrameInputs::default()).transition, None);
}
