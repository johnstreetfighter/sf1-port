//! NP5 (#308): pure online pause and after-match boxes.

use sf_engine::config::RulesetPreset;
use sf_engine::front::after_match::AfterMatchBox;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::{MenuItem, Transition};
use sf_engine::netplay::protocol::AfterMatchChoice;
use sf_engine::roster::MenuInput;
use sf_video::compositor::{ink, RomFont, TEXT_PALETTE_ROM};
use sf_video::era_hud::GaugeArt;
use sf_video::render::{Frame, HEIGHT, WIDTH};

fn font_data() -> (Vec<u8>, Vec<u8>) {
    let mut tiles = vec![0; 0x4000];
    for (code, tile) in tiles.chunks_exact_mut(16).enumerate() {
        tile.fill(code as u8);
    }
    tiles[0x3c * 16..0x3d * 16].fill(0xff); // Spaces really are transparent.
    let mut program = vec![0; TEXT_PALETTE_ROM + 128];
    for color in 0..16 {
        let pens = if color == usize::from(ink::YELLOW) {
            [0x0ff0_u16, 0x0550, 0x0aa0]
        } else {
            [0x000f, 0x0005, 0x000a]
        };
        for (pen, word) in pens.into_iter().enumerate() {
            let at = TEXT_PALETTE_ROM + color * 8 + pen * 2;
            program[at..at + 2].copy_from_slice(&word.to_be_bytes());
        }
    }
    (tiles, program)
}

fn text_at(
    frame: &Frame,
    font: &RomFont<'_>,
    col: usize,
    row: usize,
    text: &str,
    color: u8,
) -> bool {
    let mut expected = vec![0xff00_0000; WIDTH * HEIGHT];
    font.text(&mut expected, col, row, text, color);
    (row * 8..row * 8 + 8).all(|y| {
        let start = y * WIDTH + col * 8;
        let end = start + text.chars().count() * 8;
        frame.argb[start..end] == expected[start..end]
    })
}

fn render_pause(pause: &PauseBox, font: &RomFont<'_>, live: &Frame) -> Frame {
    pause.render(
        font,
        live,
        "OFF",
        "ARCADE CABINET",
        GaugeArt::Ruled,
        RulesetPreset::DefinitiveOverhaul,
        false,
        false,
    )
}

#[test]
fn online_pause_has_only_network_safe_rows_and_renders_the_live_frame_and_notice() {
    let mut pause = PauseBox::open_online();
    let items = [
        MenuItem::Resume,
        MenuItem::Overlays,
        MenuItem::Look,
        MenuItem::GaugeArt,
        MenuItem::QuitToMenu,
        MenuItem::Moves,
    ];
    assert_eq!(pause.items(), items);
    for item in items {
        assert_eq!(
            pause.apply(MenuInput::Confirm),
            Some(Transition::Choose(item))
        );
        assert_eq!(pause.apply(MenuInput::Down), Some(Transition::CursorMoved));
    }
    assert_eq!(
        pause.apply(MenuInput::Back),
        Some(Transition::Choose(MenuItem::Resume))
    );
    assert_eq!(
        pause.apply(MenuInput::Start),
        Some(Transition::Choose(MenuItem::Resume))
    );
    pause.apply(MenuInput::Up);
    assert_eq!(
        pause.apply(MenuInput::Confirm),
        Some(Transition::Choose(MenuItem::Moves))
    );
    assert!(PauseBox::open(false)
        .items()
        .contains(&MenuItem::RestartRound));
    assert!(PauseBox::open(true)
        .items()
        .contains(&MenuItem::ResetPositions));
    assert!(PauseBox::open(true)
        .items()
        .contains(&MenuItem::TrainingSettings));

    let notice = PauseBox::with_online_notice("CONTROLLER DISCONNECTED");
    assert_eq!(notice.notice(), Some("CONTROLLER DISCONNECTED"));
    assert_eq!(notice.items(), items);
    let (tiles, program) = font_data();
    let font = RomFont::new(&tiles, &program);
    let first = Frame {
        argb: vec![0xff20_4060; WIDTH * HEIGHT],
    };
    let next = Frame {
        argb: vec![0xff40_6080; WIDTH * HEIGHT],
    };
    let shown = render_pause(&notice, &font, &first);
    let updated = render_pause(&notice, &font, &next);
    assert_eq!(shown.argb[0], 0xff10_2030);
    assert_eq!(
        updated.argb[0], 0xff20_3040,
        "the box uses the newly stepped frame each time"
    );
    assert_eq!(
        first.argb[0], 0xff20_4060,
        "rendering never edits the live frame"
    );
    assert!(text_at(
        &shown,
        &font,
        12,
        8,
        "CONTROLLER DISCONNECTED",
        ink::YELLOW
    ));
    for (index, label) in [
        "RESUME",
        "OVERLAYS",
        "LOOK",
        "GAUGE ART",
        "QUIT TO MENU",
        "MOVES",
    ]
    .into_iter()
    .enumerate()
    {
        let color = if index == 0 { ink::YELLOW } else { ink::BLUE };
        assert!(
            text_at(
                &shown,
                &font,
                PauseBox::label_col(),
                PauseBox::item_row(index),
                label,
                color
            ),
            "the online row is visible: {label}"
        );
    }
}

#[test]
fn online_after_match_displays_each_peers_live_vote_and_preserves_local_choices() {
    let (tiles, program) = font_data();
    let font = RomFont::new(&tiles, &program);
    let last = Frame {
        argb: vec![0xff20_4060; WIDTH * HEIGHT],
    };
    let mut online = AfterMatchBox::open_online();
    let waiting = online.render(&font, &last);
    assert!(text_at(
        &waiting,
        &font,
        8,
        18,
        "P1: WAITING...   P2: WAITING...",
        ink::YELLOW
    ));
    for (votes, expected) in [
        (
            [
                Some(AfterMatchChoice::Rematch),
                Some(AfterMatchChoice::Reselect),
            ],
            "P1: REMATCH   P2: RESELECT",
        ),
        (
            [
                Some(AfterMatchChoice::Reselect),
                Some(AfterMatchChoice::Leave),
            ],
            "P1: RESELECT   P2: LEAVE",
        ),
        (
            [
                Some(AfterMatchChoice::Leave),
                Some(AfterMatchChoice::Rematch),
            ],
            "P1: LEAVE   P2: REMATCH",
        ),
        ([None, None], "P1: WAITING...   P2: WAITING..."),
    ] {
        online.set_peer_votes(votes);
        let shown = online.render(&font, &last);
        let col = (48 - expected.len()) / 2;
        assert!(
            text_at(&shown, &font, col, 18, expected, ink::YELLOW),
            "the latest peer votes are visible: {expected}"
        );
        assert_eq!(
            shown.argb[0], 0xff10_2030,
            "votes remain over the dimmed final frame"
        );
    }
    online.apply(MenuInput::Down);
    online.set_peer_votes([Some(AfterMatchChoice::Rematch), None]);
    assert_eq!(
        online.apply(MenuInput::Confirm),
        Some(Transition::Choose(MenuItem::Reselect)),
        "a peer update preserves the local cursor"
    );
    online.apply(MenuInput::Up);

    for mut after in [AfterMatchBox::default(), online] {
        for item in [MenuItem::Rematch, MenuItem::Reselect, MenuItem::Menu] {
            assert_eq!(
                after.apply(MenuInput::Confirm),
                Some(Transition::Choose(item))
            );
            assert_eq!(after.apply(MenuInput::Down), Some(Transition::CursorMoved));
        }
        assert_eq!(after.apply(MenuInput::Back), None);
        assert_eq!(
            after.apply(MenuInput::Start),
            Some(Transition::Choose(MenuItem::Rematch))
        );
        after.apply(MenuInput::Up);
        assert_eq!(
            after.apply(MenuInput::Confirm),
            Some(Transition::Choose(MenuItem::Menu))
        );
    }

    let local = AfterMatchBox::default().render(&font, &last);
    assert_eq!(
        local.argb[18 * 8 * WIDTH + 8 * 8],
        0xff10_2030,
        "the local box keeps its original small frame"
    );
    for (index, label) in ["REMATCH", "RESELECT", "MENU"].into_iter().enumerate() {
        let color = if index == 0 { ink::YELLOW } else { ink::BLUE };
        assert!(text_at(
            &local,
            &font,
            AfterMatchBox::item_col(),
            AfterMatchBox::item_row(index),
            label,
            color
        ));
    }
}
