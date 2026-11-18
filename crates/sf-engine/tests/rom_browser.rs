//! The ROM browser (#129): choosing the ROM set's folder or archive with a
//! pad alone, on hosts with no file picker of their own. The screen asks
//! the host for what it cannot do itself: read a folder.
use sf_engine::front::rom_browser::{Browse, RomBrowser, Source};
use sf_engine::front::{FrameInputs, MenuKeys};
use sf_engine::roster::MenuInput;

use MenuInput::{Back, Confirm, Down, Up};

fn folder(name: &str) -> Source {
    Source { name: name.to_owned(), archive: false }
}

fn archive(name: &str) -> Source {
    Source { name: name.to_owned(), archive: true }
}

fn home() -> RomBrowser {
    RomBrowser::open("/home/deck", vec![folder("Downloads"), folder("Games"), archive("sf.zip")])
}

/// One frame of a pad with `key` held.
fn held(key: Option<MenuInput>) -> FrameInputs {
    let mut input = FrameInputs::default();
    let mut pad = MenuKeys::default();
    match key {
        Some(Up) => input.p1.directions[3] = true,
        Some(Down) => input.p1.directions[2] = true,
        Some(Confirm) => pad.confirm = true,
        Some(Back) => pad.back = true,
        Some(_) => panic!("unsupported test key"),
        None => {}
    }
    input.pads[0] = Some(pad);
    input
}

fn press(screen: &mut RomBrowser, key: MenuInput) -> Option<Browse> {
    let step = screen.step(held(Some(key)));
    screen.step(held(None));
    step
}

#[test]
fn the_rows_are_this_folder_the_folder_above_and_then_what_the_folder_holds() {
    let screen = home();
    assert_eq!(screen.folder(), "/home/deck");
    assert_eq!(screen.rows(), ["USE THIS FOLDER", "UP ONE FOLDER", "Downloads/", "Games/", "sf.zip"]);
    assert_eq!(screen.cursor(), 0);
}

#[test]
fn confirm_asks_the_host_for_the_row_under_the_cursor() {
    let mut screen = home();
    assert_eq!(press(&mut screen, Confirm), Some(Browse::Use));
    press(&mut screen, Down);
    assert_eq!(press(&mut screen, Confirm), Some(Browse::Up));
    press(&mut screen, Down);
    assert_eq!(press(&mut screen, Confirm), Some(Browse::Enter("Downloads".to_owned())));
    press(&mut screen, Down);
    press(&mut screen, Down);
    assert_eq!(press(&mut screen, Confirm), Some(Browse::Choose("sf.zip".to_owned())));
}

#[test]
fn the_cursor_wraps_and_back_cancels() {
    let mut screen = home();
    press(&mut screen, Up);
    assert_eq!(screen.cursor(), 4, "up from the first row reaches the last");
    press(&mut screen, Down);
    assert_eq!(screen.cursor(), 0);
    assert_eq!(press(&mut screen, Back), Some(Browse::Cancel));
}

#[test]
fn a_new_folder_starts_at_the_top_and_a_problem_stays_until_the_next_folder() {
    let mut screen = home();
    press(&mut screen, Down);
    press(&mut screen, Down);
    screen.problem("NO ROM SET IN THIS FOLDER");
    assert_eq!(screen.notice(), Some("NO ROM SET IN THIS FOLDER"));
    assert_eq!(screen.cursor(), 2, "a problem leaves the cursor where it was");
    screen.show("/home/deck/Downloads", vec![folder("roms")]);
    assert_eq!(screen.folder(), "/home/deck/Downloads");
    assert_eq!(screen.rows(), ["USE THIS FOLDER", "UP ONE FOLDER", "roms/"]);
    assert_eq!(screen.cursor(), 0);
    assert_eq!(screen.notice(), None);
}

#[test]
fn holding_down_keeps_moving_through_a_long_folder() {
    let mut screen = RomBrowser::open("/", (0..40).map(|i| folder(&format!("f{i}"))).collect());
    // Held for 31 frames: the press, then a repeat on the 20th, 25th and
    // 30th held frame.
    for _ in 0..31 {
        screen.step(held(Some(Down)));
    }
    assert_eq!(screen.cursor(), 4);
}

#[test]
fn a_long_folder_scrolls_to_keep_the_cursor_on_screen() {
    let mut screen = RomBrowser::open("/", (0..40).map(|i| folder(&format!("f{i}"))).collect());
    let shown = screen.visible();
    assert_eq!(shown.start, 0);
    for _ in 0..30 {
        press(&mut screen, Down);
    }
    let shown = screen.visible();
    assert!(shown.contains(&30), "{shown:?}");
    assert_eq!(shown.len(), 14, "one screen holds fourteen rows");
    // The picture is drawn without a ROM set.
    assert_eq!(screen.render().argb.len(), 384 * 224);
}
