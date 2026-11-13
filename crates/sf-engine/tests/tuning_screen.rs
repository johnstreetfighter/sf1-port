//! Ticket #212 (`VO9`): the tuning screen. `TUNE CRT LOOK >` on the Video
//! tab opens it, not while the look is `OFF`. It lists the look's
//! parameters as the host gives them (one row a knob, under the shader's
//! own headings), steps each by its own step within its own limits,
//! scrolls, puts the look's own values back with `RESET LOOK`, and hides
//! its panel on Start. Through the session, the panel is a layer of its
//! own beside the frame, the frame is the attract's demo fight looping, and
//! the tuning stays the look's for the session after leaving.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::front::settings::{rows, LookName, Row, Settings, SettingsScreen, Tab, DEFAULT_LOOK, LOOK_OFF};
use sf_engine::front::tune::{DemoScene, LookParam, TuneRow, TuneScreen, DEMO_FROM, DEMO_TO, LIST_ROWS, PANEL_COLS, PANEL_ROWS};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::roster::MenuInput;
use sf_video::render::render_frame;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn param(id: &str, description: &str, initial: f32, min: f32, max: f32, step: f32) -> LookParam {
    LookParam { id: id.into(), description: description.into(), initial, min, max, step }
}

/// A small look: two sections of knobs.
fn params() -> Vec<LookParam> {
    vec![
        param("bogus_scan", "[ SCANLINE OPTIONS ]:", 0.0, 0.0, 1.0, 1.0),
        param("gsl", "          Scanline Type", 0.0, -1.0, 2.0, 1.0),
        param("beam_min", "          Scanline Beam Shape Center", 1.4, 0.25, 5.0, 0.05),
        param("bogus_mask", "[ MASK OPTIONS ]:", 0.0, 0.0, 1.0, 1.0),
        param("maskstr", "          Mask Strength (0.5..1.0)", 0.4, 0.0, 1.0, 0.025),
    ]
}

/// A Guest-sized look: 12 sections of 12 knobs, 156 rows.
fn many() -> Vec<LookParam> {
    let mut out = Vec::new();
    for s in 0..12 {
        out.push(param(&format!("bogus_{s}"), &format!("[ SECTION {s} SETTINGS ]:"), 0.0, 0.0, 1.0, 1.0));
        for k in 0..12 {
            out.push(param(&format!("p{s}_{k}"), &format!("          Knob {s} {k} With A Very Long Description Indeed"), 1.0, 0.0, 2.0, 0.1));
        }
    }
    out
}

#[test]
fn one_row_a_knob_under_its_headings_then_save_as_and_reset_look() {
    let s = TuneScreen::open("arcade_cabinet", "ARCADE CABINET", params(), &[]);
    assert_eq!(
        s.rows(),
        [
            TuneRow::Heading(0),
            TuneRow::Param(1),
            TuneRow::Param(2),
            TuneRow::Heading(3),
            TuneRow::Param(4),
            TuneRow::SaveAs,
            TuneRow::ResetLook
        ]
    );
    let labels: Vec<_> = s.rows().iter().map(|&r| s.label(r)).collect();
    assert_eq!(
        labels,
        ["[ SCANLINE OPTIONS ]", "SCANLINE TYPE", "SCAN BEAM SHAPE CENTER", "[ MASK OPTIONS ]", "MASK STRENGTH", "SAVE AS >", "RESET LOOK"],
        "upper-cased, long words shortened"
    );
    assert_eq!(s.row(), TuneRow::Param(1), "the cursor starts on the first knob");
    assert_eq!(s.value_text(TuneRow::Param(2)), "1.40");
    assert_eq!(s.value_text(TuneRow::Param(4)), "0.400");
    assert!(s.panel_shown());
    assert!(s.overrides().is_empty());
}

#[test]
fn knobs_step_by_their_own_step_between_their_own_limits() {
    let mut s = TuneScreen::open("arcade_cabinet", "ARCADE CABINET", params(), &[]);
    // SCANLINE TYPE: -1..2 by 1.
    assert_eq!(s.apply(MenuInput::Right), Some(Transition::ValueChanged));
    assert_eq!(s.value("gsl"), Some(1.0));
    s.apply(MenuInput::Right);
    assert_eq!(s.apply(MenuInput::Right), None, "held at the maximum");
    assert_eq!(s.value("gsl"), Some(2.0));
    for _ in 0..3 {
        s.apply(MenuInput::Left);
    }
    assert_eq!(s.apply(MenuInput::Left), None, "held at the minimum");
    assert_eq!(s.value("gsl"), Some(-1.0));
    assert_eq!(s.value_text(TuneRow::Param(1)), "-1");
    // BEAM SHAPE: by 0.05.
    assert_eq!(s.apply(MenuInput::Down), Some(Transition::CursorMoved));
    s.apply(MenuInput::Right);
    assert_eq!(s.value("beam_min"), Some(1.45));
    // Down skips the heading to MASK STRENGTH: by 0.025.
    s.apply(MenuInput::Down);
    assert_eq!(s.row(), TuneRow::Param(4));
    s.apply(MenuInput::Left);
    assert_eq!(s.value("maskstr"), Some(0.375));
    assert_eq!(s.value_text(TuneRow::Param(4)), "0.375");
    assert_eq!(
        s.overrides(),
        [("gsl".to_owned(), -1.0), ("beam_min".to_owned(), 1.45), ("maskstr".to_owned(), 0.375)],
        "the values away from the look's own"
    );
    // Confirm on a knob does nothing; SAVE AS comes before RESET LOOK.
    assert_eq!(s.apply(MenuInput::Confirm), None);
    s.apply(MenuInput::Down);
    assert_eq!(s.row(), TuneRow::SaveAs);
    s.apply(MenuInput::Down);
    assert_eq!(s.row(), TuneRow::ResetLook);
    assert_eq!(s.apply(MenuInput::Right), None);
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged));
    assert!(s.overrides().is_empty(), "RESET LOOK puts back the look's own values");
    assert_eq!(s.value("beam_min"), Some(1.4));
    // Down wraps to the first knob.
    s.apply(MenuInput::Down);
    assert_eq!(s.row(), TuneRow::Param(1));
    assert_eq!(s.apply(MenuInput::Back), Some(Transition::Back));
}

#[test]
fn a_session_tuning_opens_where_it_was_left() {
    let s = TuneScreen::open("arcade_cabinet", "ARCADE CABINET", params(), &[("maskstr".into(), 0.7)]);
    assert_eq!(s.value("maskstr"), Some(0.7));
    assert_eq!(s.overrides(), [("maskstr".to_owned(), 0.7)]);
    assert_eq!(s.tuning().look, "arcade_cabinet");
}

#[test]
fn start_hides_the_panel_and_start_confirm_or_back_shows_it() {
    let mut s = TuneScreen::open("arcade_cabinet", "ARCADE CABINET", params(), &[]);
    for show in [MenuInput::Start, MenuInput::Confirm, MenuInput::Back] {
        assert_eq!(s.apply(MenuInput::Start), Some(Transition::CursorMoved));
        assert!(!s.panel_shown());
        // Hidden, the knobs and the cursor rest.
        assert_eq!(s.apply(MenuInput::Right), None);
        assert_eq!(s.apply(MenuInput::Down), None);
        assert_eq!(s.apply(show), Some(Transition::CursorMoved), "{show:?} shows it");
        assert!(s.panel_shown(), "{show:?} does not leave");
    }
    assert!(s.overrides().is_empty());
    assert_eq!(s.row(), TuneRow::Param(1));
}

#[test]
fn a_hundred_and_more_knobs_scroll_and_the_tab_pair_jumps_a_section() {
    let mut s = TuneScreen::open("guest", "GUEST", many(), &[]);
    assert_eq!(s.rows().len(), 12 * 13 + 2);
    assert_eq!(s.top(), 0);
    // Down through the first section and into the second: the list keeps
    // the cursor in view.
    for _ in 0..30 {
        s.apply(MenuInput::Down);
        let at = s.rows().iter().position(|&r| r == s.row()).unwrap();
        assert!((s.top()..s.top() + LIST_ROWS).contains(&at), "row {at} in view from {}", s.top());
    }
    // The tab pair: the first knob of the next section, its heading shown.
    let mut s = TuneScreen::open("guest", "GUEST", many(), &[]);
    for section in 1..12 {
        assert_eq!(s.apply(MenuInput::TabRight), Some(Transition::CursorMoved));
        let at = 13 * section + 1;
        assert_eq!(s.row(), TuneRow::Param(at), "section {section}");
        assert!(s.top() < at, "its heading {} is in view from {}", at - 1, s.top());
    }
    s.apply(MenuInput::TabRight);
    assert_eq!(s.row(), TuneRow::SaveAs, "the foot after the last section");
    s.apply(MenuInput::TabRight);
    assert_eq!(s.row(), TuneRow::Param(1), "and round to the first");
    assert_eq!(s.top(), 0);
    s.apply(MenuInput::TabLeft);
    assert_eq!(s.row(), TuneRow::SaveAs);
    // Up from the first knob wraps to the foot.
    let mut s = TuneScreen::open("guest", "GUEST", many(), &[]);
    s.apply(MenuInput::Up);
    assert_eq!(s.row(), TuneRow::ResetLook);
    // Every long name fits beside its value.
    for &row in s.rows() {
        assert!(s.label(row).len() <= 28, "{}", s.label(row));
    }
}

#[test]
fn tune_crt_look_follows_crt_look_in_the_monitor_group_and_off_disables_it() {
    // MONITOR: ASPECT, CRT LOOK, TUNE CRT LOOK >, then the MOTION group.
    let video = rows(Tab::Video);
    let at = video.iter().position(|&r| r == Row::TuneLook).expect("a TUNE CRT LOOK row");
    assert_eq!(video[at - 3..at + 2], [Row::Heading("MONITOR"), Row::Aspect, Row::CrtLook, Row::TuneLook, Row::Heading("MOTION")]);
    let open = |look: &str| {
        let mut s = SettingsScreen::open(MatchConfig::default(), Settings { crt_look: look.into(), ..Settings::default() });
        s.apply(MenuInput::TabRight);
        s.apply(MenuInput::TabRight);
        while s.row() != Row::TuneLook {
            s.apply(MenuInput::Down);
        }
        s
    };
    let mut s = open(DEFAULT_LOOK);
    assert_eq!(s.label(Row::TuneLook), "TUNE CRT LOOK >");
    assert!(s.tune_enabled());
    assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::Choose(MenuItem::TuneLook)));
    assert_eq!(s.apply(MenuInput::Right), None, "left and right do nothing");
    let mut s = open(LOOK_OFF);
    assert!(!s.tune_enabled());
    assert_eq!(s.apply(MenuInput::Confirm), None, "disabled while the look is OFF");
}

/// The demo loop: the playfield pans within its first two seconds, and the
/// loop comes back to its first frame. No input reaches it.
#[test]
fn the_demo_scene_pans_within_seconds_and_loops() {
    let Some(assets) = assets() else { return };
    let mut demo = DemoScene::new(&assets.maincpu);
    assert_eq!(demo.machine().state.frame, DEMO_FROM);
    let first = render_frame(&assets, &demo.machine().state.scene()).argb;
    let mut panned_at = None;
    for f in 1..=120 {
        let before = demo.machine().state.fg_scroll;
        demo.step();
        if panned_at.is_none() && demo.machine().state.fg_scroll != before {
            panned_at = Some(f);
        }
    }
    let at = panned_at.expect("a pan in the first two seconds");
    assert!(at <= 120, "the playfield pans {at} frames in");
    for _ in 120..(DEMO_TO - DEMO_FROM) {
        demo.step();
    }
    assert_eq!(demo.machine().state.frame, DEMO_TO);
    demo.step();
    assert_eq!(demo.machine().state.frame, DEMO_FROM, "back to the start");
    assert!(render_frame(&assets, &demo.machine().state.scene()).argb == first, "the same picture again");
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

fn looks() -> Vec<LookName> {
    [("off", "OFF"), ("arcade_cabinet", "ARCADE CABINET")]
        .into_iter()
        .map(|(id, name)| LookName { id: id.into(), name: name.into() })
        .collect()
}

const DOWN: fn(&mut FrameInputs) = |i| i.p1.directions[2] = true;
const RIGHT: fn(&mut FrameInputs) = |i| i.p1.directions[0] = true;

/// The session on the Video tab's `TUNE CRT LOOK >` row, `look` chosen.
fn session_on_tune(look: &str) -> Option<GameSession> {
    let settings = Settings { crt_look: look.into(), ..Settings::default() };
    let mut s = GameSession::new(assets()?, MatchConfig::default()).with_looks(looks()).with_settings(settings);
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
    // MODE (the first live row), WINDOW SCALE, GAUGE ART, ASPECT and CRT
    // LOOK are above it.
    for _ in 0..5 {
        tap(&mut s, with(DOWN));
    }
    Some(s)
}

#[test]
fn off_leaves_tune_crt_look_closed() {
    let Some(mut s) = session_on_tune(LOOK_OFF) else { return };
    let out = tap(&mut s, with(|i| i.menu.confirm = true));
    assert_eq!(out.screen, "settings");
    assert!(out.panel.is_none());
}

#[test]
fn the_tuning_screen_shows_the_demo_behind_its_own_panel_and_keeps_the_tuning() {
    let Some(mut s) = session_on_tune(DEFAULT_LOOK) else { return };
    let settings_frame = s.step(FrameInputs::default()).frame;
    let out = tap(&mut s, with(|i| i.menu.confirm = true));
    assert_eq!(out.screen, "tune");
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::TuneLook)));
    // The parameters arrive from the host once the look has loaded.
    let out = s.step(FrameInputs::default());
    let panel = out.panel.expect("the panel is a layer of its own");
    assert_eq!((panel.width, panel.height), (PANEL_COLS * 8, PANEL_ROWS * 8));
    assert_eq!(panel.argb.len(), panel.width * panel.height);
    assert!(out.frame.argb != settings_frame.argb, "the frame is the scene, not a menu");
    assert_eq!(out.video.look, DEFAULT_LOOK, "drawn through the current look");
    assert!(s.look_tuning().is_none());
    s.set_look_params(DEFAULT_LOOK, params());
    let lit = |p: &sf_engine::front::Panel| p.argb.iter().filter(|&&px| px != 0xff00_0000).count();
    let empty = lit(&panel);
    let listed = s.step(FrameInputs::default()).panel.unwrap();
    assert!(lit(&listed) > empty, "the knobs are drawn when they arrive");
    // A knob: the frame that changes it carries the tuning.
    let out = s.step(with(RIGHT));
    assert_eq!(out.transition, Some(Transition::ValueChanged));
    let tuning = s.look_tuning().expect("the current look's tuning").clone();
    assert_eq!(tuning.look, DEFAULT_LOOK);
    assert_eq!(tuning.values, [("gsl".to_owned(), 1.0)]);
    s.step(FrameInputs::default());
    // Start hides the panel; the scene goes on.
    let hidden = tap(&mut s, with(|i| i.menu.start = true));
    assert!(hidden.panel.is_none());
    assert_eq!(hidden.screen, "tune");
    let shown = tap(&mut s, with(|i| i.menu.start = true));
    assert!(shown.panel.is_some());
    // Back asks about the changes; NO goes back to Settings, and out: the
    // tuning stays the look's.
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "tune");
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "settings");
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "main menu");
    assert_eq!(s.look_tuning().map(|t| t.values.clone()), Some(vec![("gsl".to_owned(), 1.0)]));
    // A reload of the look (the host reports its parameters again) keeps it.
    s.set_look_params(DEFAULT_LOOK, params());
    assert_eq!(s.look_tuning().map(|t| t.values.len()), Some(1));
}

#[test]
fn the_tuning_screen_reopens_with_the_sessions_tuning() {
    let Some(mut s) = session_on_tune(DEFAULT_LOOK) else { return };
    s.set_look_params(DEFAULT_LOOK, params());
    tap(&mut s, with(|i| i.menu.confirm = true));
    tap(&mut s, with(RIGHT));
    tap(&mut s, with(RIGHT));
    tap(&mut s, with(|i| i.menu.back = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "settings");
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "tune");
    assert_eq!(s.look_tuning().unwrap().values, [("gsl".to_owned(), 2.0)]);
    // RESET LOOK: down past BEAM SHAPE, MASK STRENGTH, SAVE AS.
    for _ in 0..4 {
        tap(&mut s, with(DOWN));
    }
    let out = tap(&mut s, with(|i| i.menu.confirm = true));
    assert_eq!(out.transition, Some(Transition::ValueChanged));
    assert!(s.look_tuning().unwrap().values.is_empty(), "RESET LOOK");
}

/// The throwaway test's session reached Settings during a sweep that sent
/// no keys (design section 4). The session itself never leaves the
/// attract without an input edge: with none, it runs a full attract loop
/// and more and stays there; and the tuning screen, with none, stays on its
/// looping scene, the demo machine taking no input at all.
#[test]
fn with_no_input_the_session_stays_in_the_attract_and_on_the_tuning_screen() {
    let Some(assets) = assets() else { return };
    let mut s = GameSession::new(assets, MatchConfig::default());
    for f in 0..9000 {
        let out = s.step(FrameInputs::default());
        assert_eq!(out.screen, "attract", "frame {f}");
    }
    let Some(mut s) = session_on_tune(DEFAULT_LOOK) else { return };
    tap(&mut s, with(|i| i.menu.confirm = true));
    for f in 0..2400 {
        let out = s.step(FrameInputs::default());
        assert_eq!(out.screen, "tune", "frame {f}");
        assert!(out.panel.is_some());
    }
}
