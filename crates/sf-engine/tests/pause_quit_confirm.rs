//! Quitting from a pause box asks first (the maintainer, 2026-11-18): the
//! quit row turns into `QUIT?` with `NO` chosen, and only `YES` leaves.
//! It holds for a local fight, Training, an online match and the replay
//! viewer. Needs no ROMs.

use sf_engine::front::menu::MenuItem;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::Transition;
use sf_engine::roster::MenuInput;

use MenuInput::{Back, Confirm, Down, Left, Right, Start, Up};

/// Every pause box and the row it quits with, the cursor already on it.
fn boxes() -> Vec<(&'static str, PauseBox, MenuItem)> {
    [
        ("fight", PauseBox::open(false), MenuItem::QuitToMenu),
        ("training", PauseBox::open(true), MenuItem::QuitToMenu),
        ("online", PauseBox::open_online(), MenuItem::QuitToMenu),
        ("replay", PauseBox::open_replay(false, [false; 2]), MenuItem::QuitToReplays),
    ]
    .into_iter()
    .map(|(name, mut pause, quit)| {
        assert_eq!(pause.items().last(), Some(&quit), "{name}: quitting is the last row");
        // Up from the first row wraps to the last.
        assert_eq!(pause.apply(Up), Some(Transition::CursorMoved), "{name}");
        (name, pause, quit)
    })
    .collect()
}

#[test]
fn choosing_quit_asks_first_with_no_chosen() {
    for (name, mut pause, quit) in boxes() {
        assert_eq!(pause.asking_to_quit(), None, "{name}");
        assert_ne!(pause.apply(Confirm), Some(Transition::Choose(quit)), "{name}: one press does not quit");
        assert_eq!(pause.asking_to_quit(), Some(false), "{name}: it asks, on NO");
    }
}

#[test]
fn a_double_press_answers_no_and_keeps_the_box() {
    for (name, mut pause, quit) in boxes() {
        pause.apply(Confirm);
        assert_ne!(pause.apply(Confirm), Some(Transition::Choose(quit)), "{name}");
        assert_eq!(pause.asking_to_quit(), None, "{name}: the question is gone");
        // The cursor is still on the quit row: it can be asked again.
        pause.apply(Confirm);
        assert_eq!(pause.asking_to_quit(), Some(false), "{name}");
    }
}

#[test]
fn only_yes_then_confirm_quits() {
    for (name, mut pause, quit) in boxes() {
        for to_yes in [Right, Left] {
            pause.apply(Confirm);
            assert_eq!(pause.apply(to_yes), Some(Transition::CursorMoved), "{name}");
            assert_eq!(pause.asking_to_quit(), Some(true), "{name}");
            // Back again to NO, and to YES once more.
            pause.apply(to_yes);
            assert_eq!(pause.asking_to_quit(), Some(false), "{name}: two answers wrap");
            pause.apply(to_yes);
            assert_eq!(pause.apply(Confirm), Some(Transition::Choose(quit)), "{name}: YES quits");
            assert_eq!(pause.asking_to_quit(), None, "{name}");
        }
    }
}

#[test]
fn back_and_start_cancel_the_question_without_resuming_or_quitting() {
    for (name, pause, quit) in boxes() {
        for cancel in [Back, Start] {
            let mut pause = pause.clone();
            pause.apply(Confirm);
            pause.apply(Right);
            let out = pause.apply(cancel);
            assert_ne!(out, Some(Transition::Choose(quit)), "{name} {cancel:?}: even from YES");
            assert_ne!(out, Some(Transition::Choose(MenuItem::Resume)), "{name} {cancel:?}: the box stays up");
            assert_eq!(pause.asking_to_quit(), None, "{name} {cancel:?}");
        }
    }
}

#[test]
fn moving_the_cursor_cancels_the_question_and_moves() {
    for (name, pause, quit) in boxes() {
        for step in [Up, Down] {
            let mut pause = pause.clone();
            pause.apply(Confirm);
            pause.apply(Right);
            assert_eq!(pause.apply(step), Some(Transition::CursorMoved), "{name}");
            assert_eq!(pause.asking_to_quit(), None, "{name}");
            // Wherever the cursor went, a press there does not quit.
            assert_ne!(pause.apply(Confirm), Some(Transition::Choose(quit)), "{name} {step:?}");
        }
    }
}
