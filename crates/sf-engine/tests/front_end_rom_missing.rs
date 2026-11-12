//! Ticket #161 (`FE11`): the ROM-missing screen. With no ROM set it is
//! drawn in the compositor's ROM-independent 3×5 font: the gold frame, the
//! set and its description, each folder looked in with what was wrong
//! there, and `RETRY` and `EXIT`. Up and down move, confirm chooses, back
//! does nothing. Needs no ROMs.

use sf_engine::front::rom_missing::{Looked, RomMissing};
use sf_engine::front::{FrameInputs, MenuItem, Transition};
use sf_video::compositor::{fallback_text, SELECT_BLUE, SELECT_GOLD, SELECT_YELLOW};
use sf_video::render::{Frame, HEIGHT, WIDTH};

fn looked() -> Vec<Looked> {
    vec![
        Looked {
            folder: r"C:\Games\SF\roms\sf".into(),
            problem: "the folder does not exist".into(),
        },
        Looked {
            folder: r"C:\Users\Player\AppData\Roaming\sf1-port\roms\sf".into(),
            problem: "sfd-19.2a has CRC-32 00000000, not faaf6255".into(),
        },
        Looked {
            folder: "rom_folder in settings.toml".into(),
            problem: "not set".into(),
        },
    ]
}

fn screen() -> RomMissing {
    RomMissing::open("SF", "US SET 1 (MAME SET SF), SIX-BUTTON CONTROLS", looked())
}

/// Every pixel `text` sets, drawn alone, is `color` in `frame`.
fn drawn(frame: &Frame, x: usize, y: usize, text: &str, color: u32, scale: usize) -> bool {
    let mut alone = vec![0u32; WIDTH * HEIGHT];
    fallback_text(&mut alone, x, y, text, color, scale);
    let set: Vec<usize> = (0..alone.len()).filter(|&i| alone[i] != 0).collect();
    !set.is_empty() && set.iter().all(|&i| frame.argb[i] == color)
}

fn tap(screen: &mut RomMissing, f: impl Fn(&mut FrameInputs)) -> Option<Transition> {
    let mut input = FrameInputs::default();
    f(&mut input);
    let t = screen.step(input);
    screen.step(FrameInputs::default());
    t
}

#[test]
fn the_screen_names_the_set_and_each_folder_with_its_problem() {
    let s = screen();
    assert_eq!(s.name(), "rom missing");
    let text = s.text();
    assert_eq!(
        text,
        [
            "THE GAME NEEDS THE ROM SET 'SF':",
            "US SET 1 (MAME SET SF), SIX-BUTTON CONTROLS",
            "",
            "LOOKED IN:",
            r"C:\Games\SF\roms\sf",
            "  the folder does not exist",
            r"C:\Users\Player\AppData\Roaming\sf1-port\roms\sf",
            "  sfd-19.2a has CRC-32 00000000, not faaf6255",
            "rom_folder in settings.toml",
            "  not set",
            "",
            "PUT THE FILES IN ONE OF THESE FOLDERS, THEN CHOOSE RETRY.",
        ]
    );
    let frame = s.render();
    assert_eq!(frame.argb[4 * WIDTH + 4], SELECT_GOLD, "the frame's outer corner");
    let title = "ROM SET NOT FOUND";
    assert!(drawn(&frame, (WIDTH - title.len() * 8) / 2, 16, title, SELECT_GOLD, 2), "the title");
    assert!(drawn(&frame, 16, 36, &text[0], SELECT_BLUE, 1), "the set line");
    assert!(drawn(&frame, 16, 36 + 4 * 8, &text[4], SELECT_BLUE, 1), "the first folder");
    assert!(drawn(&frame, 16, 36 + 5 * 8, &text[5], 0xff_80_80_88, 1), "its problem, dimmer");
    assert!(drawn(&frame, (WIDTH - 5 * 8) / 2, HEIGHT - 48, "RETRY", SELECT_YELLOW, 2), "RETRY under the cursor");
    assert!(drawn(&frame, (WIDTH - 4 * 8) / 2, HEIGHT - 32, "EXIT", SELECT_BLUE, 2), "EXIT");
}

#[test]
fn up_and_down_move_confirm_chooses_and_back_does_nothing() {
    let mut s = screen();
    assert_eq!(s.cursor(), MenuItem::Retry);
    assert_eq!(tap(&mut s, |i| i.menu.back = true), None, "back has nowhere to go");
    assert_eq!(tap(&mut s, |i| i.p1.directions[2] = true), Some(Transition::CursorMoved));
    assert_eq!(s.cursor(), MenuItem::Exit);
    assert_eq!(tap(&mut s, |i| i.p2.directions[3] = true), Some(Transition::CursorMoved));
    assert_eq!(s.cursor(), MenuItem::Retry);
    assert_eq!(tap(&mut s, |i| i.menu.confirm = true), Some(Transition::Choose(MenuItem::Retry)));
    // Held, confirm chooses once.
    let held = FrameInputs {
        menu: sf_engine::front::MenuKeys { confirm: true, ..Default::default() },
        ..Default::default()
    };
    assert_eq!(s.step(held), Some(Transition::Choose(MenuItem::Retry)));
    assert_eq!(s.step(held), None);
    s.step(FrameInputs::default());
    tap(&mut s, |i| i.p1.directions[2] = true);
    assert_eq!(tap(&mut s, |i| i.p1.buttons[0] = true), Some(Transition::Choose(MenuItem::Exit)), "LP confirms");
    let frame = s.render();
    assert!(drawn(&frame, (WIDTH - 4 * 8) / 2, HEIGHT - 32, "EXIT", SELECT_YELLOW, 2));
}

#[test]
fn a_failed_retry_shows_the_new_results_and_long_folders_keep_their_end() {
    let mut s = screen();
    let long = format!(r"C:\{}\roms\sf", "Very Long Folder Name".repeat(5));
    s.update(vec![Looked { folder: long.clone(), problem: "sfd-22.2c is missing".into() }]);
    assert_eq!(s.looked().len(), 1);
    let text = s.text();
    assert!(text[4].starts_with("...") && text[4].ends_with(r"Name\roms\sf"), "{}", text[4]);
    assert_eq!(text[4].chars().count(), (WIDTH - 32) / 4 - 2);
    assert_eq!(text[5], "  sfd-22.2c is missing");
}

#[test]
fn the_missing_screen_can_browse_for_a_folder_or_an_archive() {
    let mut s = screen().with_browse(true);
    assert_eq!(s.cursor(), MenuItem::Retry);
    tap(&mut s, |i| i.p1.directions[2] = true);
    assert_eq!(s.cursor(), MenuItem::BrowseRomFolder);
    assert_eq!(tap(&mut s, |i| i.menu.confirm = true), Some(Transition::Choose(MenuItem::BrowseRomFolder)));
    tap(&mut s, |i| i.p1.directions[2] = true);
    assert_eq!(s.cursor(), MenuItem::BrowseRomArchive);
    assert_eq!(tap(&mut s, |i| i.menu.start = true), Some(Transition::Choose(MenuItem::BrowseRomArchive)));
    assert!(drawn(&s.render(), (WIDTH - 14 * 8) / 2, HEIGHT - 38, "BROWSE ARCHIVE", SELECT_YELLOW, 2));

    s.failed_choice(Looked { folder: "D:\\sf.7z".into(), problem: "a ROM is missing".into() });
    assert_eq!(&s.text()[4..6], ["D:\\sf.7z", "  a ROM is missing"]);
    assert!(s.text().contains(&looked()[0].folder));
    s.update(looked());
    assert_eq!(s.text()[4], looked()[0].folder);
}
