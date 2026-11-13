//! Ticket #228 (`ML2`): drawn arrows and the MOVE INPUTS setting.
//!
//! At the headless session's seam, driven by inputs: the Controls tab's
//! `MOVE INPUTS` is changed on the Settings screen, a Local fight is
//! started, and the move list is opened from the pause box. The panel is
//! drawn in each notation (arrows by default, numpad, words), the arrows
//! for the way the fighter faces, and the setting changes the rows through
//! the session. Local ROMs only; each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fighter::P1_STRUCT;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::moves::{MovesPanel, MOVES_COLS};
use sf_engine::front::pause::PauseBox;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Panel};
use sf_engine::moves::{facing_right, InputNotation};
use sf_engine::roster::TournamentFighter;
use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::decode::decode_char_tile;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const DOWN: usize = 2;
const UP: usize = 3;

/// The first value column: the input is five cells at the panel's right.
const VALUE: usize = MOVES_COLS - 1 - 5;

/// The arrow tiles and the button chips, as rows of pixels kept by hand
/// (bit 7 the left pixel): the made glyphs' body, whose shadow falls one
/// pixel down and right.
const ARROW_DOWN: [u8; 8] = [0x30, 0x30, 0x30, 0xfc, 0x78, 0x30, 0x00, 0x00];
const ARROW_LEFT: [u8; 8] = [0x20, 0x60, 0xfc, 0xfc, 0x60, 0x20, 0x00, 0x00];
const ARROW_RIGHT: [u8; 8] = [0x10, 0x18, 0xfc, 0xfc, 0x18, 0x10, 0x00, 0x00];
const ARROW_UP: [u8; 8] = [0x30, 0x78, 0xfc, 0x30, 0x30, 0x30, 0x00, 0x00];
const ARROW_DOWN_RIGHT: [u8; 8] = [0xc0, 0xe0, 0x70, 0x3c, 0x1c, 0x3c, 0x00, 0x00];
const ARROW_DOWN_LEFT: [u8; 8] = [0x0c, 0x1c, 0x38, 0xf0, 0xe0, 0xf0, 0x00, 0x00];
const ARROW_UP_RIGHT: [u8; 8] = [0x3c, 0x1c, 0x3c, 0x70, 0xe0, 0xc0, 0x00, 0x00];
const ARROW_UP_LEFT: [u8; 8] = [0xf0, 0xe0, 0xf0, 0x38, 0x1c, 0x0c, 0x00, 0x00];
/// The `P` and `K` chips: seven by seven, corners cut, the letter knocked out.
const CHIP_P: [u8; 8] = [0x7c, 0xc6, 0xd6, 0xc6, 0xde, 0xde, 0x7c, 0x00];
const CHIP_K: [u8; 8] = [0x7c, 0xd6, 0xd6, 0xce, 0xd6, 0xd6, 0x7c, 0x00];

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

fn stick(dir: usize) -> FrameInputs {
    with(|i| i.p1.directions[dir] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn back() -> FrameInputs {
    with(|i| i.menu.back = true)
}

fn to_fight_loop(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

/// A session at the Main Menu, past the boot, its cursor on Single Player.
fn main_menu(preset: RulesetPreset, settings: Option<Settings>) -> GameSession {
    let a = assets().expect("local ROMs");
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    if let Some(settings) = settings {
        s = s.with_settings(settings);
    }
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    s
}

/// From the Main Menu, into Settings, the Controls tab's `MOVE INPUTS`
/// stepped `steps` times to the right, and out again to the Main Menu with
/// its cursor on SETTINGS.
fn choose_in_settings(s: &mut GameSession, steps: usize) {
    tap(s, stick(DOWN));
    tap(s, stick(DOWN));
    tap(s, stick(DOWN));
    assert_eq!(tap(s, confirm()).screen, "settings");
    for _ in 0..4 {
        tap(s, with(|i| i.menu.tab_right = true));
    }
    // P1 DEVICE, P2 DEVICE, P1 SCHEME, P2 SCHEME, then MOVE INPUTS.
    for _ in 0..4 {
        tap(s, stick(DOWN));
    }
    for _ in 0..steps {
        tap(s, stick(RIGHT));
    }
    assert_eq!(tap(s, back()).screen, "main menu");
}

/// From the Main Menu, a Local fight with Ryu as P1, stopped at the first
/// fight loop; `to_multi` moves the cursor from where it stands to Multi
/// Player.
fn local_fight(s: &mut GameSession, to_multi: usize) {
    tap(s, stick(to_multi));
    tap(s, confirm()); // Multi Player
    tap(s, confirm()); // Local
    select::confirm_fighter(s, confirm(), tap); // P1: Ryu
    select::confirm_fighter(s, with(|i| i.p2.buttons[0] = true), tap);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(s);
}

/// Whether P1 faces right, read from the machine.
fn p1_faces_right(s: &GameSession) -> bool {
    let m = s.fight_machine().expect("a fight");
    facing_right(m.work_ram(), &m.state.objectram, P1_STRUCT)
}

/// Pause and choose MOVES.
fn open_moves(s: &mut GameSession) {
    s.step(with(|i| i.menu.start = true));
    s.step(idle());
    let at = PauseBox::open(false).items().iter().position(|&i| i == MenuItem::Moves).expect("a MOVES row");
    for _ in 0..at {
        tap(s, stick(DOWN));
    }
    s.step(confirm());
    s.step(idle());
}

/// Whether cell (`col`, `row`) of the panel is the made glyph `rows` in
/// colour `color`: body pixels in pen 0, a shadow pixel in pen 1 one down
/// and right of each, the rest black.
fn made_cell(p: &Panel, a: &Assets, col: usize, row: usize, rows: [u8; 8], color: u8) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let body = |x: usize, y: usize| rows[y] & (0x80 >> x) != 0;
    (0..8).all(|y| {
        (0..8).all(|x| {
            let want = if body(x, y) {
                font.pen(color, 0)
            } else if x > 0 && y > 0 && body(x - 1, y - 1) {
                font.pen(color, 1)
            } else {
                0xff00_0000
            };
            p.argb[(row * 8 + y) * p.width + col * 8 + x] == want
        })
    })
}

/// Whether `text` is drawn from cell (`col`, `row`) of the panel in the
/// ROM's font in `color` (glyph pixels only).
fn panel_text(p: &Panel, a: &Assets, col: usize, row: usize, text: &str, color: u8) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| {
            (0..8).all(|tx| {
                let px = p.argb[(row * 8 + ty) * p.width + (col + i) * 8 + tx];
                tile[ty][tx] == 3 || px == font.pen(color, tile[ty][tx])
            })
        })
    })
}

/// The pixels of the three rows' input cells, to compare.
fn input_pixels(p: &Panel) -> Vec<u32> {
    (4 * 8..7 * 8)
        .flat_map(|y| p.argb[y * p.width + VALUE * 8..y * p.width + (MOVES_COLS - 1) * 8].to_vec())
        .collect()
}

/// The three rows of Ryu's list in each notation, facing right.
fn rows(notation: InputNotation) -> [String; 3] {
    match notation {
        InputNotation::Arrows => ["HADOKEN ↓↘→ [P]", "SHORYUKEN →↓↘ [P]", "TATSUMAKI ↓↙← [K]"],
        InputNotation::Numpad => ["HADOKEN 236 P", "SHORYUKEN 623 P", "TATSUMAKI 214 K"],
        InputNotation::Words => ["HADOKEN QCF+P", "SHORYUKEN DP+P", "TATSUMAKI QCB+K"],
    }
    .map(String::from)
}

/// Whether the three rows' inputs are drawn as arrows and chips: `shapes`
/// are the three arrows of each row, `chips` its button.
fn arrows_drawn(p: &Panel, a: &Assets, shapes: [[[u8; 8]; 3]; 3]) -> Result<(), String> {
    for (i, cells) in shapes.into_iter().enumerate() {
        let row = 4 + i;
        let color = if i == 0 { ink::YELLOW } else { ink::BLUE };
        for (n, cell) in cells.into_iter().enumerate() {
            if !made_cell(p, a, VALUE + n, row, cell, color) {
                return Err(format!("row {row}: arrow {n}"));
            }
        }
        let chip = if i == 2 { CHIP_K } else { CHIP_P };
        if !made_cell(p, a, VALUE + 4, row, chip, color) {
            return Err(format!("row {row}: the button chip"));
        }
        if !panel_text(p, a, VALUE + 3, row, " ", color) {
            return Err(format!("row {row}: the gap before the chip"));
        }
    }
    Ok(())
}

const FACING_RIGHT: [[[u8; 8]; 3]; 3] = [
    [ARROW_DOWN, ARROW_DOWN_RIGHT, ARROW_RIGHT],
    [ARROW_RIGHT, ARROW_DOWN, ARROW_DOWN_RIGHT],
    [ARROW_DOWN, ARROW_DOWN_LEFT, ARROW_LEFT],
];
const FACING_LEFT: [[[u8; 8]; 3]; 3] = [
    [ARROW_DOWN, ARROW_DOWN_LEFT, ARROW_LEFT],
    [ARROW_LEFT, ARROW_DOWN, ARROW_DOWN_LEFT],
    [ARROW_DOWN, ARROW_DOWN_RIGHT, ARROW_RIGHT],
];

#[test]
fn the_setting_changes_the_rows_through_the_session_and_arrows_are_the_default() {
    let Some(a) = assets() else { return };
    let cases = [(0, InputNotation::Arrows), (1, InputNotation::Numpad), (2, InputNotation::Words), (3, InputNotation::Arrows)];
    for (steps, notation) in cases {
        let mut s = main_menu(RulesetPreset::Faithful1987, None);
        choose_in_settings(&mut s, steps);
        assert_eq!(s.settings().move_inputs, notation, "{steps} steps");
        local_fight(&mut s, UP);
        assert!(p1_faces_right(&s), "P1 starts facing right");
        open_moves(&mut s);
        let list = s.move_list().expect("the list is up");
        assert_eq!(list.notation(), notation);
        assert!(list.facing_right());
        assert_eq!(list.text()[1..4], rows(notation), "{notation:?}");
        // The pixels: the rows' inputs are drawn as the notation writes them.
        let panel = s.step(idle()).panel.expect("the panel layer");
        let at = MOVES_COLS - 1 - 5;
        match notation {
            InputNotation::Arrows => assert_eq!(arrows_drawn(&panel, &a, FACING_RIGHT), Ok(())),
            InputNotation::Numpad => {
                assert!(panel_text(&panel, &a, at, 4, "236 P", ink::YELLOW));
                assert!(panel_text(&panel, &a, at, 5, "623 P", ink::BLUE));
                assert!(panel_text(&panel, &a, at, 6, "214 K", ink::BLUE));
            }
            InputNotation::Words => {
                assert!(panel_text(&panel, &a, at, 4, "QCF+P", ink::YELLOW));
                assert!(panel_text(&panel, &a, at + 1, 5, "DP+P", ink::BLUE));
                assert!(panel_text(&panel, &a, at, 6, "QCB+K", ink::BLUE));
            }
        }
    }
}

#[test]
fn a_saved_choice_is_the_one_a_session_opens_with_in_other_presets_too() {
    let Some(a) = assets() else { return };
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        let settings = Settings { move_inputs: InputNotation::Words, ..Settings::default() };
        let mut s = main_menu(preset, Some(settings));
        local_fight(&mut s, DOWN);
        open_moves(&mut s);
        assert_eq!(s.move_list().expect("the list").text()[1..4], rows(InputNotation::Words), "{preset:?}");
        let panel = s.step(idle()).panel.expect("the panel layer");
        assert!(panel_text(&panel, &a, MOVES_COLS - 1 - 5, 4, "QCF+P", ink::YELLOW), "{preset:?}");
    }
}

#[test]
fn the_arrows_are_drawn_facing_each_way() {
    let Some(a) = assets() else { return };
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let ryu = |facing_right| {
        let list = MovesPanel::open(TournamentFighter::Ryu, RulesetPreset::Faithful1987).with_inputs(InputNotation::Arrows, facing_right);
        let panel = list.panel(&font);
        (list, panel)
    };
    // Facing right: the Hadoken is down, down-forward, forward; the
    // Tatsumaki down, down-back, back.
    let (list, right) = ryu(true);
    assert_eq!(list.text()[1], "HADOKEN ↓↘→ [P]");
    assert_eq!(list.text()[3], "TATSUMAKI ↓↙← [K]");
    assert_eq!(arrows_drawn(&right, &a, FACING_RIGHT), Ok(()), "facing right");
    // Facing left, the horizontals turn over, so the arrows match the
    // stick as it is held.
    let (list, left) = ryu(false);
    assert_eq!(list.text()[1], "HADOKEN ↓↙← [P]");
    assert_eq!(list.text()[3], "TATSUMAKI ↓↘→ [K]");
    assert_eq!(arrows_drawn(&left, &a, FACING_LEFT), Ok(()), "facing left");
    assert_ne!(input_pixels(&right), input_pixels(&left));
    // The up arrows and the four diagonals each have their own tile.
    let eight = [ARROW_DOWN, ARROW_LEFT, ARROW_RIGHT, ARROW_UP, ARROW_DOWN_RIGHT, ARROW_DOWN_LEFT, ARROW_UP_RIGHT, ARROW_UP_LEFT];
    for (i, one) in eight.iter().enumerate() {
        for other in &eight[i + 1..] {
            assert_ne!(one, other, "eight different arrows");
        }
    }
}

#[test]
fn numpad_and_words_do_not_turn_with_the_fighter() {
    let Some(a) = assets() else { return };
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    for notation in [InputNotation::Numpad, InputNotation::Words] {
        let at = |facing_right| {
            MovesPanel::open(TournamentFighter::Ryu, RulesetPreset::Faithful1987)
                .with_inputs(notation, facing_right)
                .panel(&font)
        };
        let (right, left) = (at(true), at(false));
        assert!(right.argb == left.argb, "{notation:?}: the same panel either way");
    }
    let numpad = MovesPanel::open(TournamentFighter::Ryu, RulesetPreset::Faithful1987).with_inputs(InputNotation::Numpad, false);
    assert!(panel_text(&numpad.panel(&font), &a, VALUE, 4, "236 P", ink::YELLOW), "236, facing left as well");
}

#[test]
fn a_fighter_facing_left_gets_the_arrows_turned_over_through_the_session() {
    let Some(a) = assets() else { return };
    let mut s = main_menu(RulesetPreset::Faithful1987, None);
    local_fight(&mut s, DOWN);
    // Jump over P2: up and forward until P1 faces left.
    let mut f = 0;
    while p1_faces_right(&s) {
        s.step(with(|i| {
            i.p1.directions[UP] = f % 80 < 30;
            i.p1.directions[RIGHT] = true;
        }));
        f += 1;
        assert!(f < 1_500, "P1 turns to face left");
    }
    // Let it settle on its feet, then pause.
    for _ in 0..90 {
        s.step(idle());
    }
    assert!(!p1_faces_right(&s), "P1 still faces left once landed");
    open_moves(&mut s);
    let list = s.move_list().expect("the list");
    assert!(!list.facing_right());
    assert_eq!(list.text()[1], "HADOKEN ↓↙← [P]");
    let panel = s.step(idle()).panel.expect("the panel layer");
    assert_eq!(arrows_drawn(&panel, &a, FACING_LEFT), Ok(()));
}
