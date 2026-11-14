//! Ticket #153 (`FE2`): the game boots to the attract and the Main Menu.
//!
//! Every check drives the headless game session from power-on with
//! per-frame inputs and asserts what a player could see or hear: the
//! presented pixels at known text cells (against the ROM's own font and
//! palette), the screen's name, the transition, the sound latch, and the
//! attract machine against a plain power-on machine stepped alongside.
//! Local ROMs only; each test skips without them.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::era::EraFamily;
use sf_engine::front::attract::FreePlay;
use sf_engine::front::{sound, FrameInputs, FrameOutput, GameSession, MenuItem, MenuScreen, Transition};
use sf_engine::run::PowerOnMachine;
use sf_engine::roster::TournamentFighter;
use sf_engine::world_tour::native::{node, Perk, WorldTourProfile};
use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::decode::decode_char_tile;
use sf_video::palette::Palette;
use sf_video::render::{render_frame, Frame, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn session(assets: Assets) -> GameSession {
    GameSession::new(assets, MatchConfig::default())
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn start() -> FrameInputs {
    let mut i = FrameInputs::default();
    i.menu.start = true;
    i
}

fn back() -> FrameInputs {
    let mut i = FrameInputs::default();
    i.menu.back = true;
    i
}

fn confirm() -> FrameInputs {
    let mut i = FrameInputs::default();
    i.menu.confirm = true;
    i
}

fn p_dir(player: usize, dir: usize) -> FrameInputs {
    let mut i = FrameInputs::default();
    if player == 1 {
        i.p1.directions[dir] = true;
    } else {
        i.p2.directions[dir] = true;
    }
    i
}

const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;

/// Press `input` for one frame, then release it for one frame; returns the
/// press frame's output.
fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(idle());
    out
}

/// Whether `frame` shows `ch` at cell column `col` and pixel `top` as the
/// ROM's 8×8 tile draws it in `pen_color(pen)`, over `background` where the
/// tile is transparent.
fn glyph_shows(
    frame: &Frame,
    gfx4: &[u8],
    col: usize,
    top: usize,
    ch: char,
    pen_color: impl Fn(u8) -> u32,
    background: Option<u32>,
) -> bool {
    let tile = decode_char_tile(gfx4, small_glyph(ch).expect("ROM glyph") as usize);
    (0..8).all(|y| {
        (0..8).all(|x| {
            let px = frame.argb[(top + y) * WIDTH + col * 8 + x];
            match tile[y][x] {
                3 => background.map_or(true, |b| px == b),
                pen => px == pen_color(pen),
            }
        })
    })
}

/// Whether `text` is drawn from cell column `col` and pixel `top` in text
/// colour `color` of the boot palette on black.
fn text_shows(frame: &Frame, a: &Assets, col: usize, top: usize, text: &str, color: u8) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    // Spaces, and the few characters the ROM's font lacks (drawn as made
    // glyphs), are not checked.
    text.chars().enumerate().all(|(i, ch)| {
        ch == ' '
            || small_glyph(ch).is_none()
            || glyph_shows(frame, &a.gfx4, col + i, top, ch, |p| font.pen(color, p), Some(0xff00_0000))
    })
}

/// Whether text row `row` of `frame` is black from cell `first` to cell
/// `last` (inclusive): nothing at a framed screen's foot.
fn row_is_black(frame: &Frame, row: usize, first: usize, last: usize) -> bool {
    (row * 8..row * 8 + 8).all(|y| (first * 8..last * 8 + 8).all(|x| frame.argb[y * WIDTH + x] == 0xff00_0000))
}

/// The colour a row of `menu` is drawn in, from its first letter's cells.
fn row_color(frame: &Frame, a: &Assets, menu: &MenuScreen, item: MenuItem, index: usize) -> Option<u8> {
    let (col, y) = (MenuScreen::item_col(item), menu.item_y(index));
    [ink::YELLOW, ink::BLUE, ink::DIM]
        .into_iter()
        .find(|&c| text_shows(frame, a, col, y, item.label(), c))
}

#[test]
fn attract_is_the_faithful_power_on_machine_with_free_play_over_insert_coin() {
    let Some(a) = assets() else { return };
    let free_play = FreePlay::from_rom(&a.maincpu).expect("INSERT COIN block");
    let (fp_col, fp_row) = free_play.screen_cell();
    assert_eq!((fp_col, fp_row), (18, 23), "INSERT COIN sits at text row 25, column 26");
    let mut reference = PowerOnMachine::new(a.maincpu.clone());
    let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
    let (mut shown, mut hidden) = (0u32, 0u32);
    // Power-on through the warning screen, the wall and title, the ranking,
    // the demo fight and into the second attract loop's demo setup (4697).
    for f in 1..=6000u32 {
        let out = s.step(idle());
        reference.step();
        let m = s.attract_machine();
        assert_eq!(out.screen, "attract");
        assert_eq!(out.transition, None);
        assert_eq!(m.state.frame, f);
        assert_eq!(m.work_ram(), reference.work_ram(), "work RAM at frame {f}");
        assert_eq!(m.task_positions(), reference.task_positions(), "tasks at frame {f}");
        assert_eq!(out.sound_frame, f);
        assert_eq!(out.sound_latch, reference.state.sound_latch, "latch at frame {f}");
        assert_eq!(out.latch_cycles, reference.state.latch_cycles);
        let raw = render_frame(&a, &reference.state.scene());
        // FREE PLAY shows exactly when INSERT COIN would: in video RAM with
        // the text layer on (it is off on frame 794, the join).
        if free_play.showing(&reference.state.videoram) && reference.state.gfxctrl & 0x08 != 0 {
            shown += 1;
            let palette = Palette::from_ram(&reference.state.paletteram);
            let pen = |p: u8| palette.argb(768 + 5 * 4 + p as usize);
            for (i, ch) in " FREE PLAY ".chars().enumerate() {
                if ch != ' ' {
                    assert!(
                        glyph_shows(&out.frame, &a.gfx4, fp_col + i, fp_row * 8, ch, pen, None),
                        "FREE PLAY '{ch}' at frame {f}"
                    );
                }
            }
            let (x0, x1, y0, y1) = (fp_col * 8, (fp_col + 11) * 8, fp_row * 8, fp_row * 8 + 8);
            for (i, (p, r)) in out.frame.argb.iter().zip(raw.argb.iter()).enumerate() {
                let (x, y) = (i % WIDTH, i / WIDTH);
                if !(x0..x1).contains(&x) || !(y0..y1).contains(&y) {
                    assert_eq!(p, r, "pixel ({x}, {y}) outside FREE PLAY at frame {f}");
                }
            }
            assert_ne!(out.frame.argb, raw.argb, "INSERT COIN replaced at frame {f}");
        } else {
            hidden += 1;
            assert_eq!(out.frame.argb, raw.argb, "presented frame equals the machine's at {f}");
        }
    }
    assert!(shown > 1000 && hidden > 1000, "INSERT COIN blinked: {shown} on, {hidden} off");
    assert!(s.attract_machine().unrecovered().is_empty());
}

#[test]
fn start_opens_the_main_menu_in_every_attract_phase_and_esc_resumes_the_attract() {
    let Some(a) = assets() else { return };
    // Warning screen, attract wall and title, ranking, demo fight.
    for press in [300u32, 1200, 1700, 2500] {
        let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
        for _ in 1..press {
            s.step(idle());
        }
        let paused_at = s.attract_machine().state.frame;
        assert_eq!(paused_at, press - 1);
        let snapshot = s.attract_machine().save_snapshot();
        let out = s.step(start());
        assert_eq!(out.screen, "main menu", "Start at frame {press}");
        assert_eq!(out.transition, Some(Transition::OpenMainMenu));
        assert_eq!(out.sound_latch, sound::STOP_ALL, "the attract's sound stops");
        let out = s.step(idle());
        assert_eq!(out.sound_latch, sound::STOP_SAMPLES, "then the samples stop");
        for _ in 0..40 {
            let out = s.step(idle());
            assert_eq!(out.screen, "main menu");
            assert_eq!(out.sound_latch, sound::IDLE, "nothing sounds on an idle menu");
        }
        assert_eq!(s.attract_machine().save_snapshot(), snapshot, "the attract is paused");
        let out = s.step(back());
        assert_eq!(out.screen, "attract");
        assert_eq!(out.transition, Some(Transition::Back));
        for _ in 0..200 {
            s.step(idle());
        }
        let mut reference = PowerOnMachine::new(a.maincpu.clone());
        for _ in 0..paused_at + 201 {
            reference.step();
        }
        let m = s.attract_machine();
        assert_eq!(m.state.frame, reference.state.frame);
        assert_eq!(m.work_ram(), reference.work_ram(), "resumed from frame {paused_at}");
        assert_eq!(m.task_positions(), reference.task_positions());
        assert!(m.unrecovered().is_empty());
    }
}

#[test]
fn title_theme_plays_for_the_logo_and_hands_back_to_the_attract() {
    let Some(a) = assets() else { return };
    let mut s = session(a);
    s.set_title_theme_available(true);
    for f in 1..=1287 {
        let out = s.step(idle());
        assert!(!out.tune_table, "extended table before the title on frame {f}");
    }

    let title = s.step(idle());
    assert_eq!((title.sound_latch, title.tune_table), (sound::TITLE_THEME, true));
    assert!(!title.overhaul_fight, "the title tune table does not enable ROM announcer replacement");
    let menu = s.step(start());
    assert_eq!(menu.screen, "main menu");
    assert_eq!((menu.sound_latch, menu.tune_table), (sound::IDLE, true));
    assert!(!menu.overhaul_fight, "the menu tune table does not enable ROM announcer replacement");
    let title = s.step(back());
    assert_eq!(title.screen, "attract");
    assert_eq!((title.sound_latch, title.tune_table), (sound::IDLE, true));

    for _ in 1290..=1647 {
        let out = s.step(idle());
        assert_eq!((out.sound_latch, out.tune_table), (sound::IDLE, true));
    }
    let stop = s.step(idle());
    assert_eq!((stop.sound_latch, stop.tune_table), (sound::STOP_MUSIC, false));
    let ranking = s.step(idle());
    assert_eq!((ranking.sound_latch, ranking.tune_table), (0x39, false));
}

#[test]
fn country_select_music_works_without_the_local_title_score() {
    let Some(a) = assets() else { return };
    let mut s = session(a);
    s.step(idle());
    tap(&mut s, start());
    tap(&mut s, confirm());
    let select = s.step(confirm());
    assert_eq!((select.screen, select.sound_latch), ("select", sound::CONFIRM));
    let music = s.step(idle());
    assert_eq!((music.sound_latch, music.tune_table), (sound::COUNTRY_SELECT, false));
    let back = s.step(back());
    assert_eq!((back.screen, back.sound_latch), ("single player", sound::STOP_MUSIC));
    assert_eq!(s.step(idle()).sound_latch, sound::IDLE);
}

#[test]
fn start_waits_for_the_ram_clear_and_every_start_source_opens_the_menu() {
    let Some(_) = assets() else { return };
    let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
    // Frame 1 still runs the reset's RAM clear: Start there is ignored.
    let out = s.step(start());
    assert_eq!(out.screen, "attract");
    s.step(idle());
    assert_eq!(s.step(start()).screen, "main menu", "the first frame after the clear");

    let sources: [fn(&mut FrameInputs); 4] = [
        |i| i.menu.start = true,
        |i| i.menu.confirm = true,
        |i| i.cabinet[2] = true,
        |i| i.cabinet[3] = true,
    ];
    for set in sources {
        let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
        for _ in 0..100 {
            s.step(idle());
        }
        let mut input = idle();
        set(&mut input);
        assert_eq!(s.step(input).screen, "main menu");
    }
    // An attack button is not Start.
    let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
    for _ in 0..100 {
        s.step(idle());
    }
    let mut lp = idle();
    lp.p1.buttons[0] = true;
    assert_eq!(s.step(lp).screen, "attract");
}

#[test]
fn main_menu_rows_cursor_greyed_items_and_sounds() {
    let Some(a) = assets() else { return };
    let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
    for _ in 0..100 {
        s.step(idle());
    }
    let out = tap(&mut s, start());
    let main = [
        MenuItem::SinglePlayer,
        MenuItem::MultiPlayer,
        MenuItem::Replays,
        MenuItem::Settings,
        MenuItem::Cabinet1987,
        MenuItem::Exit,
    ];
    let menu = MenuScreen::main();
    let colors: Vec<_> = main.iter().enumerate().map(|(i, it)| row_color(&out.frame, &a, &menu, *it, i)).collect();
    assert_eq!(
        colors,
        [Some(ink::YELLOW), Some(ink::BLUE), Some(ink::BLUE), Some(ink::BLUE), Some(ink::BLUE), Some(ink::BLUE)],
        "cursor on SINGLE PLAYER, every row live"
    );
    let arrow_col = MenuScreen::item_col(MenuItem::SinglePlayer) - 2;
    assert!(text_shows(&out.frame, &a, arrow_col, menu.item_y(0), ">", ink::YELLOW));
    assert!(row_is_black(&out.frame, 25, 6, 41), "no preset line (the maintainer, 2026-11-07)");

    // Down through MULTI PLAYER, REPLAYS, SETTINGS, 1987 and EXIT, wrapping to
    // SINGLE PLAYER; up wraps back to EXIT. Each move sounds 0x09.
    let expect_cursor = |s: &mut GameSession, input: FrameInputs, at: usize| {
        let out = tap(s, input);
        assert_eq!(out.transition, Some(Transition::CursorMoved));
        assert_eq!(out.sound_latch, sound::CURSOR);
        assert_eq!(s.menu_item(), Some(main[at]));
        for _ in 0..4 { s.step(idle()); }
        let settled = s.step(idle());
        assert_eq!(row_color(&settled.frame, &a, &menu, main[at], at), Some(ink::YELLOW), "cursor on {:?}", main[at]);
    };
    expect_cursor(&mut s, p_dir(1, DOWN), 1);
    expect_cursor(&mut s, p_dir(2, DOWN), 2); // either player's stick drives the one cursor
    expect_cursor(&mut s, p_dir(1, DOWN), 3);
    expect_cursor(&mut s, p_dir(1, DOWN), 4);
    expect_cursor(&mut s, p_dir(1, DOWN), 5);
    expect_cursor(&mut s, p_dir(1, DOWN), 0);
    expect_cursor(&mut s, p_dir(1, UP), 5);
    expect_cursor(&mut s, p_dir(1, UP), 4);
    expect_cursor(&mut s, p_dir(1, UP), 3);
    expect_cursor(&mut s, p_dir(1, UP), 2);
    expect_cursor(&mut s, p_dir(1, UP), 1);
    expect_cursor(&mut s, p_dir(1, UP), 0);

    // Confirm opens Single Player: ARCADE MODE (FE6) under the cursor,
    // TRAINING (FE7), World Tour and Dramatic Battle live.
    let out = tap(&mut s, confirm());
    assert_eq!(out.screen, "single player");
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::SinglePlayer)));
    assert_eq!(out.sound_latch, sound::CONFIRM);
    let single = [MenuItem::Arcade, MenuItem::Training, MenuItem::WorldTour, MenuItem::DramaticBattle];
    let menu = MenuScreen::single_player();
    let colors: Vec<_> = single.iter().enumerate().map(|(i, it)| row_color(&out.frame, &a, &menu, *it, i)).collect();
    assert_eq!(colors, [Some(ink::YELLOW), Some(ink::BLUE), Some(ink::BLUE), Some(ink::BLUE)]);
    assert!(row_is_black(&out.frame, 25, 6, 41), "no preset line");
    // Down reaches World Tour and Dramatic Battle in order.
    let out = tap(&mut s, p_dir(1, DOWN));
    assert_eq!((out.transition, out.sound_latch), (Some(Transition::CursorMoved), sound::CURSOR));
    assert_eq!(row_color(&out.frame, &a, &menu, MenuItem::Training, 1), Some(ink::YELLOW));
    assert_eq!(tap(&mut s, p_dir(1, DOWN)).transition, Some(Transition::CursorMoved));
    assert_eq!(s.menu_item(), Some(MenuItem::WorldTour));
    let out = tap(&mut s, confirm());
    assert_eq!(out.screen, "world tour");
    // World travel map foreground at sea and land pixels in the MAME capture.
    assert_eq!(out.frame.argb[48 * WIDTH + 99], 0xffcc_7700);
    assert_eq!(out.frame.argb[38 * WIDTH + 218], 0xff00_7777);
    assert_eq!(tap(&mut s, back()).screen, "single player");
    tap(&mut s, p_dir(1, DOWN));
    assert_eq!(s.menu_item(), Some(MenuItem::DramaticBattle));
    assert_eq!(tap(&mut s, confirm()).screen, "dramatic setup");
    assert_eq!(tap(&mut s, back()).screen, "single player");
    // LK is back, as on the fight bindings.
    let mut lk = idle();
    lk.p2.buttons[3] = true;
    let out = tap(&mut s, lk);
    assert_eq!((out.screen, out.transition), ("main menu", Some(Transition::Back)));

    // Multi Player: LOCAL under the cursor, with ONLINE and Dramatic live.
    tap(&mut s, p_dir(1, DOWN));
    let mut lp = idle();
    lp.p1.buttons[0] = true;
    let out = tap(&mut s, lp);
    assert_eq!(out.screen, "multi player");
    let multi = [MenuItem::Local, MenuItem::Online, MenuItem::LocalCoopDramatic];
    let menu = MenuScreen::multi_player();
    let colors: Vec<_> = multi.iter().enumerate().map(|(i, it)| row_color(&out.frame, &a, &menu, *it, i)).collect();
    assert_eq!(colors, [Some(ink::YELLOW), Some(ink::BLUE), Some(ink::BLUE)]);
    tap(&mut s, p_dir(1, UP));
    assert_eq!(s.menu_item(), Some(MenuItem::LocalCoopDramatic));
    assert_eq!(tap(&mut s, confirm()).screen, "dramatic setup");
    assert_eq!(tap(&mut s, back()).screen, "multi player");
    assert_eq!(tap(&mut s, back()).screen, "main menu");
    let out = tap(&mut s, back());
    assert_eq!((out.screen, out.transition), ("attract", Some(Transition::Back)));
    assert!(s.attract_machine().unrecovered().is_empty());
}

#[test]
fn main_and_multi_player_cursors_flash_and_kick_on_the_accepted_input_frame() {
    let Some(a) = assets() else { return };
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
    for _ in 0..100 { s.step(idle()); }
    tap(&mut s, start());
    const CREAM: [u32; 3] = [0xffff_ffaa, 0xff88_4400, 0xffff_dd88];
    const WARM: [u32; 3] = [0xffff_dd88, 0xff88_4400, 0xffee_aa33];

    let animated = |mut expected: Frame, menu: &MenuScreen, row: usize, kick: isize, pens| {
        let item = menu.items()[row];
        let x = MenuScreen::item_col(item) * 8;
        let y = menu.item_y(row);
        let width = item.label().len() * 8;
        for scan in y..y + 8 {
            expected.argb[scan * WIDTH + x - 17..scan * WIDTH + x + width].fill(0xff00_0000);
        }
        font.glyph_in(&mut expected.argb, (x - 16).saturating_add_signed(kick), y, '>', pens);
        for (cell, ch) in item.label().chars().enumerate() {
            font.glyph_in(&mut expected.argb, x + cell * 8, y, ch, pens);
        }
        expected
    };

    let main = MenuScreen::main().at(1);
    let selected_main = s.step(p_dir(1, DOWN));
    assert_eq!(selected_main.transition, Some(Transition::CursorMoved));
    assert_eq!(selected_main.sound_latch, sound::CURSOR);
    let expected_main = animated(main.render(&font, None, None), &main, 1, 2, CREAM);
    let main_row = main.item_y(1) * WIDTH..(main.item_y(1) + 8) * WIDTH;
    assert_eq!(selected_main.frame.argb[main_row.clone()], expected_main.argb[main_row]);

    let multi = MenuScreen::multi_player();
    let baseline = |row| multi.clone().at(row).render(&font, None, None);
    let rows_match = |got: Frame, expected: Frame| {
        assert!(got.argb[32 * WIDTH..] == expected.argb[32 * WIDTH..], "menu pixels below the title");
    };
    let opened = s.step(confirm());
    assert_eq!(opened.transition, Some(Transition::Choose(MenuItem::MultiPlayer)));
    rows_match(opened.frame, baseline(0));
    let moved = s.step(p_dir(1, DOWN));
    rows_match(moved.frame, animated(baseline(1), &multi, 1, 2, CREAM));
    let reversal = s.step(p_dir(1, UP));
    assert_eq!(reversal.transition, Some(Transition::CursorMoved));
    rows_match(reversal.frame, animated(baseline(0), &multi, 0, 2, CREAM));

    for (kick, pens) in [(2, CREAM), (-1, CREAM), (-1, WARM), (-1, WARM), (0, WARM)] {
        let tick = s.step(p_dir(1, UP));
        assert_eq!(tick.transition, None, "holding Up adds no movement edge");
        rows_match(tick.frame, animated(baseline(0), &multi, 0, kick, pens));
    }
    assert_eq!(s.step(idle()).frame.argb, baseline(0).argb);

    s.step(p_dir(1, DOWN));
    let returned = s.step(back());
    assert_eq!(returned.transition, Some(Transition::Back));
    assert_eq!(row_color(&returned.frame, &a, &main, MenuItem::MultiPlayer, 1), Some(ink::YELLOW));
    rows_match(s.step(confirm()).frame, baseline(1));

    s.step(back());
    let mut move_and_confirm = p_dir(1, UP);
    move_and_confirm.menu.confirm = true;
    let chosen = s.step(move_and_confirm);
    assert_eq!(chosen.transition, Some(Transition::Choose(MenuItem::SinglePlayer)));
    assert_eq!(chosen.screen, "single player", "movement and confirm both take effect on the press frame");
}

#[test]
fn world_tour_controls_change_the_shared_fighter_style_and_rpg_profile() {
    let Some(assets) = assets() else { return };
    let mut profile = WorldTourProfile::default();
    profile.complete(node("jp-1-dojo").unwrap());
    profile.complete(node("jp-3-era").unwrap());
    let mut session = session(assets).with_world_tour_profile(profile);
    for _ in 0..200 { session.step(idle()); }
    tap(&mut session, start());
    tap(&mut session, confirm());
    tap(&mut session, p_dir(1, DOWN));
    tap(&mut session, p_dir(1, DOWN));
    assert_eq!(tap(&mut session, confirm()).screen, "world tour");
    tap(&mut session, p_dir(1, RIGHT));
    assert_eq!(session.world_tour_profile().current_node().id, "jp-2-shuriken");
    let mut tab_left = idle();
    tab_left.menu.tab_left = true;
    assert_eq!(tap(&mut session, tab_left).screen, "world tour fighter select");
    tap(&mut session, p_dir(1, RIGHT));
    assert_eq!(tap(&mut session, confirm()).transition, Some(Transition::StyleOpened));
    tap(&mut session, p_dir(1, RIGHT));
    assert_eq!(tap(&mut session, confirm()).screen, "world tour");
    assert_eq!(session.world_tour_profile().fighter, TournamentFighter::Ken);
    assert_ne!(session.world_tour_profile().style.family, EraFamily::Sf1Plus);
    assert!(session.take_world_tour_changed());

    let mut tab_right = idle();
    tab_right.menu.tab_right = true;
    assert_eq!(tap(&mut session, tab_right).screen, "world tour rpg");
    tap(&mut session, confirm());
    assert!(!session.world_tour_profile().rpg);
    tap(&mut session, p_dir(1, DOWN));
    tap(&mut session, p_dir(1, RIGHT));
    assert_eq!(session.world_tour_profile().attack, 1);
    tap(&mut session, p_dir(1, LEFT));
    assert_eq!(session.world_tour_profile().attack, 0);
    tap(&mut session, p_dir(1, UP));
    tap(&mut session, p_dir(1, UP));
    tap(&mut session, p_dir(1, RIGHT));
    assert_eq!(session.world_tour_profile().perk, Some(Perk::VitalSpirit));
    assert_eq!(tap(&mut session, back()).screen, "world tour");
    assert!(session.take_world_tour_changed());
}

#[test]
fn exit_quits_and_nothing_else_does() {
    let Some(_) = assets() else { return };
    let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
    for _ in 0..100 {
        assert!(!s.step(idle()).quit);
    }
    tap(&mut s, start());
    // Back and every other row's confirm leave the game running.
    for input in [back(), start(), confirm(), back(), back(), start()] {
        assert!(!tap(&mut s, input).quit);
    }
    tap(&mut s, p_dir(1, UP));
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Exit)));
    assert!(out.quit, "Exit closes the game");
}

#[test]
fn every_sub_menu_draws_its_header_in_the_rom_header_font_inside_the_gold_frame() {
    let Some(a) = assets() else { return };
    let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
    for _ in 0..100 {
        s.step(idle());
    }
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let check = |frame: &Frame, header: &str, offset: usize| {
        let mut expected = vec![0xff00_0000u32; frame.argb.len()];
        let width = 2 * header.len();
        let start = (38 - width) / 2;
        font.gold_frame(&mut expected, 5, 2, 38, 25, start..start + width);
        font.header_px(&mut expected, (5 + start) * 8, 8 + offset, header, ink::BLUE);
        // Rows 1..=3 hold the header and the frame's top edge only.
        for y in 8..32 {
            for x in 0..WIDTH {
                assert_eq!(frame.argb[y * WIDTH + x], expected[y * WIDTH + x], "{header} at ({x}, {y})");
            }
        }
        // The frame's left and right edges and bottom corner cells.
        for (col, row) in [(5, 10), (42, 10), (5, 26), (42, 26)] {
            for y in 0..8 {
                for x in 0..8 {
                    let i = (row * 8 + y) * WIDTH + col * 8 + x;
                    assert_eq!(frame.argb[i], expected[i], "{header} frame cell ({col}, {row})");
                }
            }
        }
    };
    let main = tap(&mut s, start());
    let single = s.step(confirm());
    assert_eq!(single.transition, Some(Transition::Choose(MenuItem::SinglePlayer)));
    assert_eq!(single.screen, "single player");
    check(&single.frame, "SINGLE PLAYER", 2);
    check(&s.step(idle()).frame, "SINGLE PLAYER", 2);
    for _ in 0..3 {
        check(&s.step(idle()).frame, "SINGLE PLAYER", 1);
    }
    check(&s.step(idle()).frame, "SINGLE PLAYER", 0);

    let returned = s.step(back());
    assert_eq!(returned.transition, Some(Transition::Back));
    assert_eq!(returned.screen, "main menu");
    let art_row = |y| y * WIDTH + 48..y * WIDTH + 336;
    assert!(returned.frame.argb[..16 * WIDTH] == main.frame.argb[..16 * WIDTH], "fixed top rail");
    for y in 16..110 {
        assert!(returned.frame.argb[art_row(y)] == main.frame.argb[art_row(y + 2)], "returning logo row {y}");
    }
    assert!(returned.frame.argb[112 * WIDTH..] == main.frame.argb[112 * WIDTH..], "fixed menu rows");
    s.step(idle());
    // A new route replaces the stamp immediately, even before the previous
    // title has settled.
    tap(&mut s, p_dir(1, DOWN));
    check(&tap(&mut s, confirm()).frame, "MULTI PLAYER", 2);
    for _ in 0..4 {
        s.step(idle());
    }
    check(&s.step(idle()).frame, "MULTI PLAYER", 0);
}

/// The Main Menu has an unbroken gold frame, the edition logo inside its
/// top edge and the menu rows below. SF1+ adds the prime and subtitle to
/// the 1987 logo without disturbing the ROM lettering.
#[test]
fn the_main_menu_shows_the_title_logo_inside_an_unbroken_frame() {
    let Some(a) = assets() else { return };
    let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
    // Frame 1500 of power-on is the title screen: the logo alone, on the
    // text layer (`sf-video`'s `frame1500` fixture).
    let mut title = None;
    for _ in 0..1500 {
        title = Some(s.step(idle()));
    }
    let title = title.unwrap();
    assert_eq!(title.screen, "attract");
    let menu = tap(&mut s, start());
    assert_eq!(menu.screen, "main menu");

    let edition = sf_engine::version::EDITION_NAME;
    let has_prime = edition.starts_with("Street Fighter':");
    if has_prime || edition == "Street Fighter: The World Tournament" {
        let mut rom_ink = 0;
        let mut prime_ink = 0;
        for y in 2 * 8..13 * 8 {
            for x in 8 * 8..40 * 8 {
                let i = y * WIDTH + x;
                let pixel = menu.frame.argb[i];
                let original = title.frame.argb[i + 8 * WIDTH];
                if has_prime && (304..316).contains(&x) && (33..48).contains(&y) {
                    prime_ink += usize::from(pixel != original);
                } else {
                    assert_eq!(pixel, original, "ROM logo pixel at ({x}, {y})");
                    rom_ink += usize::from(pixel != 0xff00_0000);
                }
            }
        }
        assert!(rom_ink > 2_000, "the ROM logo is drawn ({rom_ink} pixels)");
        if has_prime {
            assert!(prime_ink > 15, "the prime is drawn ({prime_ink} pixels)");
            let mut subtitle_ink = 0;
            for y in 104..112 {
                for x in 110..276 {
                    let i = y * WIDTH + x;
                    let pixel = menu.frame.argb[i];
                    subtitle_ink += usize::from(pixel != 0xff00_0000 && pixel != title.frame.argb[i]);
                }
            }
            assert!(subtitle_ink > 50, "the edition subtitle is drawn ({subtitle_ink} pixels)");
        }
    }

    // The frame, unbroken: every one of its cells is the gold frame's
    // with no gap, and nothing is above it.
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let mut expected = vec![0xff00_0000u32; menu.frame.argb.len()];
    font.gold_frame(&mut expected, 5, 1, 38, 26, 0..0);
    let edge = |col: usize, row: usize| col == 5 || col == 42 || row == 1 || row == 26;
    for row in 0..=26 {
        for col in 0..48 {
            if row >= 1 && !edge(col, row) {
                continue;
            }
            for y in row * 8..row * 8 + 8 {
                for x in col * 8..col * 8 + 8 {
                    let i = y * WIDTH + x;
                    assert_eq!(menu.frame.argb[i], expected[i], "frame cell ({col}, {row}) at ({x}, {y})");
                }
            }
        }
    }

    for (gap, rows) in [("under the subtitle", 112..118)] {
        for y in rows {
            let interior = &menu.frame.argb[y * WIDTH + 6 * 8..y * WIDTH + 42 * 8];
            assert!(interior.iter().all(|&pixel| pixel == 0xff00_0000), "expected blank space {gap} at y={y}");
        }
    }

    // The rows sit under the logo, the last one clear of the frame.
    let main = MenuScreen::main();
    let item_y: Vec<_> = (0..main.items().len()).map(|i| main.item_y(i)).collect();
    assert_eq!(item_y, [118, 132, 146, 160, 174, 188]);
    assert_eq!(row_color(&menu.frame, &a, &main, MenuItem::SinglePlayer, 0), Some(ink::YELLOW));
}

/// The Main Menu's notice (an untranslated branch, a failed look) goes on
/// row 25, the one free row under EXIT.
#[test]
fn the_main_menu_notice_sits_under_its_rows() {
    let Some(a) = assets() else { return };
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let notice = "LOOK FAILED: SHADER MISSING";
    let frame = MenuScreen::main().render(&font, None, Some(notice));
    assert!(text_shows(&frame, &a, (48 - notice.len()) / 2, 25 * 8, notice, ink::YELLOW));
}

/// the maintainer, 2026-11-07: no front-end screen shows the preset line any more.
/// The row it took inside each frame's foot is empty on the menus, player
/// select, Settings and the Jukebox (stage select, which needs the
/// background bank, is checked in `stage_select.rs`).
#[test]
fn no_framed_screen_shows_the_preset_line() {
    let Some(_) = assets() else { return };
    let fresh = || {
        let mut s = session(load_assets(ROMS, MANIFEST).unwrap());
        for _ in 0..200 {
            s.step(idle());
        }
        s
    };
    let clear = |out: &FrameOutput, screen: &str, wide: bool| {
        assert_eq!(out.screen, screen);
        let (first, last) = if wide { (2, 45) } else { (6, 41) };
        assert!(row_is_black(&out.frame, 25, first, last), "{screen} shows something at its foot");
    };
    let mut s = fresh();
    clear(&tap(&mut s, start()), "main menu", false);
    clear(&tap(&mut s, confirm()), "single player", false);
    tap(&mut s, p_dir(1, DOWN));
    clear(&tap(&mut s, confirm()), "training", false);
    tap(&mut s, back());
    tap(&mut s, back());
    tap(&mut s, p_dir(1, DOWN));
    clear(&tap(&mut s, confirm()), "multi player", false);
    clear(&tap(&mut s, confirm()), "select", false);

    let mut s = fresh();
    tap(&mut s, start());
    tap(&mut s, p_dir(1, DOWN));
    tap(&mut s, p_dir(1, DOWN));
    tap(&mut s, p_dir(1, DOWN));
    clear(&tap(&mut s, confirm()), "settings", true);
    let mut tab = idle();
    tab.menu.tab_right = true;
    for _ in 0..3 {
        tap(&mut s, tab.clone());
    }
    for _ in 0..5 {
        tap(&mut s, p_dir(1, DOWN));
    }
    clear(&tap(&mut s, confirm()), "jukebox", true);
}

/// Spec story 69: each menu opens where it was left, and the positions go
/// to the settings file, so a new session opens there too.
#[test]
fn menus_open_where_they_were_left_and_the_positions_survive_the_settings_file() {
    let Some(a) = assets() else { return };
    let mut s = session(a);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    assert_eq!(s.menu_item(), Some(MenuItem::SinglePlayer), "a first launch opens on the first row");
    tap(&mut s, p_dir(1, DOWN));
    assert_eq!(tap(&mut s, confirm()).screen, "multi player");
    assert_eq!(tap(&mut s, back()).screen, "main menu");
    assert_eq!(s.menu_item(), Some(MenuItem::MultiPlayer));
    assert_eq!(tap(&mut s, back()).screen, "attract");
    assert_eq!(tap(&mut s, start()).screen, "main menu");
    assert_eq!(s.menu_item(), Some(MenuItem::MultiPlayer), "the Main Menu reopens on MULTI PLAYER");
    assert_eq!(s.settings().menu_positions, [1, 0, 0, 0]);
    // Single Player also reopens on its saved position.
    let mut settings = s.settings().clone();
    settings.menu_positions[1] = 2;
    let text = settings.to_toml();
    assert!(text.contains("main_menu = 1\n") && text.contains("single_player_menu = 2\n"), "{text}");
    let back_in = sf_engine::front::settings::Settings::from_toml(&text).unwrap();
    assert_eq!(back_in, settings);
    let mut t = session(assets().unwrap()).with_settings(back_in);
    for _ in 0..200 {
        t.step(idle());
    }
    tap(&mut t, start());
    assert_eq!(t.menu_item(), Some(MenuItem::MultiPlayer), "a new session opens where the file says");
    tap(&mut t, p_dir(1, UP));
    assert_eq!(tap(&mut t, confirm()).screen, "single player");
    assert_eq!(t.menu_item(), Some(MenuItem::WorldTour), "Single Player restores its own row");
}
