//! Ticket #227 (`ML1`): the pause box's MOVES row and the move list.
//!
//! At the headless session's seam, driven by inputs: MOVES opens from the
//! pause box in Local, Arcade and Training under every preset, with the
//! sections and rows each fighter has (Ryu and Ken the three 1987
//! specials, a boss the normals line only), and back returns; the panel is
//! a layer beside the fight, which is drawn as it was; every registry
//! entry, shown from the list in Training (Show feeds its input as P1's
//! player input, ML4), brings out its move under every preset where it is
//! listed; a list kept by hand here
//! matches the registry's per fighter and preset; and opening the list
//! leaves the machine as it was. Local ROMs only; each test skips without
//! them.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fighter::P1_STRUCT;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::moves::{MovesPanel, MOVES_COLS};
use sf_engine::front::pause::PauseBox;
use sf_engine::front::{MenuKeys, FrameInputs, FrameOutput, GameSession, MenuItem, Panel, PanelSide, Transition};
use sf_engine::moves::{move_list, Recognizer, Tracker1987, NORMALS_LINE, REGISTRY};
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art_from, Art, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::decode::decode_char_tile;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const ADON_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/adon.json"),
];
const ADON_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/adon.json"),
];

fn adon_art(a: &Assets) -> Option<Art> {
    let pack = ArtPack::open(Path::new(PACK)).ok()?;
    load_art_from(a, &ArtSources { recipes: &ADON_RECIPES, reviews: Some(&ADON_REVIEWS), pack: Ok(&pack) }).ok()
}

const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;

/// The presets a player can pick.
const PRESETS: [RulesetPreset; 4] = [
    RulesetPreset::Faithful1987,
    RulesetPreset::DefinitiveOverhaul,
    RulesetPreset::ClassicCompetitive,
    RulesetPreset::PureFootsies1987Plus,
];

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

fn start() -> FrameInputs {
    with(|i| i.menu.start = true)
}

fn to_fight_loop(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Local,
    Training,
    /// Training against the dummy, whose P2 side never moves.
    Dummy,
    Arcade,
}

/// The select's cursor steps from Ryu to `fighter` (two rows of six).
fn cursor_steps(fighter: TournamentFighter) -> Vec<usize> {
    let at = TournamentFighter::ALL.iter().position(|&f| f == fighter).unwrap();
    let mut steps = Vec::new();
    if at >= 6 {
        steps.push(DOWN);
    }
    steps.extend(std::iter::repeat(RIGHT).take(at % 6));
    steps
}

/// A session of `mode` under `preset` with `fighter` as P1, stopped at the
/// first fight loop.
fn fight(mode: Mode, preset: RulesetPreset, fighter: TournamentFighter) -> GameSession {
    fight_vs(mode, preset, fighter, &[])
}

/// As [`fight`], with Player 2's cursor moved `p2_steps` from where it
/// starts in Local (the other modes ignore it).
fn fight_vs(mode: Mode, preset: RulesetPreset, fighter: TournamentFighter, p2_steps: &[usize]) -> GameSession {
    let a = assets().expect("local ROMs");
    let art = (!fighter.is_shoto()).then(|| adon_art(&a)).flatten();
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    if let Some(art) = &art {
        s = s.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
    }
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    let pick = |s: &mut GameSession| {
        for d in cursor_steps(fighter) {
            tap(s, stick(d));
        }
    };
    match mode {
        Mode::Local => {
            tap(&mut s, stick(DOWN));
            tap(&mut s, confirm()); // Multi Player
            tap(&mut s, confirm()); // Local
            pick(&mut s);
            tap(&mut s, confirm());
            for &d in p2_steps {
                tap(&mut s, with(|i| i.p2.directions[d] = true));
            }
            tap(&mut s, with(|i| i.p2.buttons[0] = true));
            while s.screen() != "country select" {
                s.step(idle());
            }
            tap(&mut s, with(|i| i.p1.buttons[0] = true));
        }
        Mode::Training | Mode::Dummy => {
            tap(&mut s, confirm()); // Single Player
            tap(&mut s, stick(DOWN));
            tap(&mut s, confirm()); // Training
            if mode == Mode::Dummy {
                tap(&mut s, stick(DOWN));
            }
            tap(&mut s, confirm()); // CPU or DUMMY
            pick(&mut s);
            tap(&mut s, confirm()); // P1
            s.step(confirm()); // the opponent under the cursor
            if mode == Mode::Dummy {
                while s.screen() != "country select" {
                    s.step(idle());
                }
                tap(&mut s, with(|i| i.p1.buttons[0] = true));
            }
        }
        Mode::Arcade => {
            tap(&mut s, confirm()); // Single Player
            tap(&mut s, confirm()); // Arcade
            pick(&mut s);
            tap(&mut s, confirm());
            while s.screen() != "country select" {
                s.step(idle());
            }
            // The four-flag screen's first flag (up-left), picked with LP.
            s.step(idle());
            tap(&mut s, stick(UP));
            tap(&mut s, stick(LEFT));
            tap(&mut s, with(|i| i.p1.buttons[0] = true));
        }
    }
    to_fight_loop(&mut s);
    s
}

/// Whether `text` is drawn from cell (`col`, `row`) of a `width`-pixel
/// wide ARGB image in the ROM's font in colour `color` (glyph pixels only).
fn text_in(argb: &[u32], width: usize, a: &Assets, col: usize, row: usize, text: &str, color: u8) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| {
            (0..8).all(|tx| {
                let px = argb[(row * 8 + ty) * width + (col + i) * 8 + tx];
                tile[ty][tx] == 3 || px == font.pen(color, tile[ty][tx])
            })
        })
    })
}

/// The arrow tiles and the button chips, as rows of pixels kept by hand
/// (bit 7 the left pixel): the made glyphs' body, whose shadow falls one
/// pixel down and right.
const ARROW_DOWN: [u8; 8] = [0x30, 0x30, 0x30, 0xfc, 0x78, 0x30, 0x00, 0x00];
const ARROW_RIGHT: [u8; 8] = [0x10, 0x18, 0xfc, 0xfc, 0x18, 0x10, 0x00, 0x00];
const ARROW_DOWN_RIGHT: [u8; 8] = [0xc0, 0xe0, 0x70, 0x3c, 0x1c, 0x3c, 0x00, 0x00];
const ARROW_DOWN_LEFT: [u8; 8] = [0x0c, 0x1c, 0x38, 0xf0, 0xe0, 0xf0, 0x00, 0x00];
/// The `P` chip: seven by seven, corners cut, the letter knocked out.
const CHIP_P: [u8; 8] = [0x7c, 0xc6, 0xd6, 0xc6, 0xde, 0xde, 0x7c, 0x00];

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

fn panel_text(p: &Panel, a: &Assets, col: usize, row: usize, text: &str, color: u8) -> bool {
    text_in(&p.argb, p.width, a, col, row, text, color)
}

/// Open the pause box and choose MOVES; returns the choice's frame.
fn open_moves(s: &mut GameSession, training: bool) -> FrameOutput {
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    s.step(idle());
    let at = PauseBox::open(training).items().iter().position(|&i| i == MenuItem::Moves).expect("a MOVES row");
    for _ in 0..at {
        tap(s, stick(DOWN));
    }
    let out = s.step(confirm());
    s.step(idle());
    out
}

/// The list's expected text: the sections and rows each fighter has in a
/// real fight, kept by hand from the reference docs (`specials.md`,
/// `hadoken.md`: Ryu's and Ken's Hadoken, Shoryuken and Tatsumaki are the
/// only moves a player can make that real fights run; ticket #248 adds
/// Retsu's Kiai Shot on the Hadoken's command outside Faithful 1987, ticket
/// #251 Geki's Shuriken and Teleport Strike, and ticket #252 Joe's Rolling
/// Sobat and Dynamite Straight), then the normals.
fn expected_text(fighter: TournamentFighter) -> Vec<String> {
    expected_text_in(fighter, RulesetPreset::DefinitiveOverhaul)
}

fn expected_text_in(fighter: TournamentFighter, preset: RulesetPreset) -> Vec<String> {
    let mut text = Vec::new();
    if fighter == TournamentFighter::Retsu && preset != RulesetPreset::Faithful1987 {
        text.extend(["SPECIALS", "KIAI SHOT ↓↘→ [P]"].map(String::from));
    }
    if fighter == TournamentFighter::Geki && preset != RulesetPreset::Faithful1987 {
        text.extend(["SPECIALS", "SHURIKEN ↓↘→ [P]", "TELEPORT STRIKE ↓↙← [P]"].map(String::from));
    }
    if fighter == TournamentFighter::Joe && preset != RulesetPreset::Faithful1987 {
        text.extend(["SPECIALS", "ROLLING SOBAT ↓↙← [K]", "DYNAMITE STRAIGHT →↓↘ [P]"].map(String::from));
    }
    if fighter == TournamentFighter::Mike && preset != RulesetPreset::Faithful1987 {
        text.extend(["SPECIALS", "BISON RUSH ↓↘→ [P]", "IRON UPPER →↓↘ [P]"].map(String::from));
    }
    if fighter == TournamentFighter::Lee && preset != RulesetPreset::Faithful1987 {
        text.extend(["SPECIALS", "ZESSHOUOHKEN ↓↘→ [P]", "DOUBLE KICK ↓↙← [K]"].map(String::from));
    }
    if fighter == TournamentFighter::Gen && preset != RulesetPreset::Faithful1987 {
        text.extend(["SPECIALS", "HYAKURENKO ↓↘→ [P]", "GEKIRO →↓↘ [P]"].map(String::from));
    }
    if fighter == TournamentFighter::Birdie && preset != RulesetPreset::Faithful1987 {
        text.extend(["SPECIALS", "BULL HEAD ↓↘→ [P]", "BULL HORN ↓↙← [K]"].map(String::from));
    }
    if fighter == TournamentFighter::Eagle && preset != RulesetPreset::Faithful1987 {
        text.extend(
            [
                "SPECIALS",
                "MANCHESTER BLACK ↓↘→ [P]",
                "ST. ANDREW'S GREEN ↓↙← [K]",
            ]
            .map(String::from),
        );
    }
    if fighter == TournamentFighter::Adon && preset != RulesetPreset::Faithful1987 {
        text.extend(["SPECIALS", "RISING JAGUAR →↓↘ [P]", "JAGUAR KICK ↓↙← [K]"].map(String::from));
    }
    if fighter == TournamentFighter::Sagat && preset != RulesetPreset::Faithful1987 {
        text.extend(
            [
                "SPECIALS",
                "HIGH TIGER SHOT ↓↘→ [P]",
                "LOW TIGER SHOT ↓↘→ [K]",
                "TIGER KNEE →↓↘ [K]",
            ]
            .map(String::from),
        );
    }
    if matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken) {
        // Drawn arrows are the default notation, for a fighter facing right.
        text.extend(
            ["SPECIALS", "HADOKEN ↓↘→ [P]", "SHORYUKEN →↓↘ [P]", "TATSUMAKI ↓↙← [K]"].map(String::from),
        );
    }
    text.extend(["NORMALS".to_owned(), "LP MP HP LK MK HK · STAND CROUCH JUMP".to_owned()]);
    text
}

#[test]
fn a_hand_kept_list_per_fighter_and_preset_matches_the_move_list() {
    for preset in RulesetPreset::ALL {
        for fighter in TournamentFighter::ALL {
            let list = MovesPanel::open(fighter, preset);
            assert_eq!(list.text(), expected_text_in(fighter, preset), "{fighter:?} under {preset:?}");
        }
    }
}

#[test]
fn moves_opens_in_local_arcade_and_training_under_every_preset_and_back_returns() {
    let Some(a) = assets() else { return };
    for mode in [Mode::Local, Mode::Arcade, Mode::Training] {
        for preset in PRESETS {
            let mut s = fight(mode, preset, TournamentFighter::Ryu);
            let training = mode == Mode::Training;
            let out = open_moves(&mut s, training);
            let label = format!("{mode:?} under {preset:?}");
            assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Moves)), "{label}");
            let list = s.move_list().unwrap_or_else(|| panic!("the list is up ({label})"));
            assert_eq!(list.fighter(), TournamentFighter::Ryu, "{label}");
            assert_eq!(list.text(), expected_text(TournamentFighter::Ryu), "{label}");
            // Drawn: the heading, the rows with the cursor row yellow, the line.
            let panel = s.step(idle()).panel.unwrap_or_else(|| panic!("the panel layer ({label})"));
            assert_eq!((panel.side, panel.width), (PanelSide::Left, MOVES_COLS * 8), "{label}");
            let centred = |t: &str| (MOVES_COLS - t.chars().count()) / 2;
            assert!(panel_text(&panel, &a, centred("[ SPECIALS ]"), 3, "[ SPECIALS ]", ink::BLUE), "{label}");
            assert!(panel_text(&panel, &a, 2, 4, "HADOKEN", ink::YELLOW), "{label}");
            // The inputs are arrows (the default) and a chip: made tiles.
            let value = MOVES_COLS - 1 - 5;
            assert!(made_cell(&panel, &a, value, 4, ARROW_DOWN, ink::YELLOW), "{label}");
            assert!(made_cell(&panel, &a, value + 1, 4, ARROW_DOWN_RIGHT, ink::YELLOW), "{label}");
            assert!(made_cell(&panel, &a, value + 2, 4, ARROW_RIGHT, ink::YELLOW), "{label}");
            assert!(made_cell(&panel, &a, value + 4, 4, CHIP_P, ink::YELLOW), "{label}");
            assert!(panel_text(&panel, &a, 2, 5, "SHORYUKEN", ink::BLUE), "{label}");
            assert!(made_cell(&panel, &a, value + 1, 6, ARROW_DOWN_LEFT, ink::BLUE), "{label}");
            assert!(panel_text(&panel, &a, centred(NORMALS_LINE), 8, NORMALS_LINE, ink::BLUE), "{label}");
            // The middle dot (cell 18 of the line) is drawn: the made glyph's
            // body pixels in the font's pen 0, the cell's corners black.
            let font = RomFont::new(&a.gfx4, &a.maincpu);
            let (dx, dy) = ((centred(NORMALS_LINE) + 18) * 8, 8 * 8);
            let at = |x: usize, y: usize| panel.argb[(dy + y) * panel.width + dx + x];
            assert_eq!(NORMALS_LINE.chars().nth(18), Some('·'), "{label}");
            for (x, y) in [(3, 3), (4, 3), (3, 4), (4, 4)] {
                assert_eq!(at(x, y), font.pen(ink::BLUE, 0), "{label}: the dot's body at ({x}, {y})");
            }
            assert_eq!([at(0, 0), at(7, 0), at(0, 7), at(7, 7)], [0xff00_0000; 4], "{label}");
            // Back returns to the pause box, the panel gone.
            let out = tap(&mut s, back());
            assert_eq!(out.transition, Some(Transition::Back), "{label}");
            assert!(s.move_list().is_none(), "{label}");
            assert_eq!(out.screen, "pause", "{label}");
            assert!(s.step(idle()).panel.is_none(), "{label}");
            // The pause box is the one up again, its cursor where it was:
            // confirm chooses MOVES once more, and Start resumes.
            let out = tap(&mut s, confirm());
            assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Moves)), "{label}");
            tap(&mut s, back());
            let out = tap(&mut s, start());
            assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)), "{label}");
        }
    }
}

#[test]
fn ken_has_the_same_specials_and_a_boss_the_normals_line_only() {
    let Some(a) = assets() else { return };
    // Ken as P1 where the preset lets him (Faithful 1987 binds Ryu to P1
    // and Ken to P2, as the 1987 game does; that list is the hand-kept
    // test's, and P2's list opens with ML3).
    for preset in &PRESETS[1..] {
        let preset = *preset;
        let mut s = fight(Mode::Local, preset, TournamentFighter::Ken);
        open_moves(&mut s, false);
        let list = s.move_list().expect("the list");
        assert_eq!(list.fighter(), TournamentFighter::Ken, "{preset:?}");
        assert_eq!(list.text(), expected_text(TournamentFighter::Ken), "{preset:?}");
        let panel = s.step(idle()).panel.expect("the panel");
        assert!(panel_text(&panel, &a, 2, 4, "HADOKEN", ink::YELLOW), "{preset:?}");
    }
    // Ken in Arcade, which takes Ryu or Ken.
    let mut s = fight(Mode::Arcade, RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ken);
    open_moves(&mut s, false);
    assert_eq!(s.move_list().expect("the list").text(), expected_text(TournamentFighter::Ken));
    // A boss (Adon: Sagat is banned in Competitive), where the preset's
    // full cast and the art pack make one playable: with his extended
    // data-bank table loaded on the session, his move list has his specials
    // and the normals line; without his table in the bank, the panel has
    // the normals line only.
    for mode in [Mode::Local, Mode::Training] {
        for preset in &PRESETS[1..] {
            let mut s = fight(mode, *preset, TournamentFighter::Adon);
            open_moves(&mut s, mode == Mode::Training);
            let list = s.move_list().expect("the list");
            assert_eq!(list.fighter(), TournamentFighter::Adon, "{mode:?} {preset:?}");
            assert_eq!(list.text(), expected_text(TournamentFighter::Adon), "{mode:?} {preset:?}");
            let without_bank = MovesPanel::open_in_fight(TournamentFighter::Adon, *preset, false);
            assert_eq!(without_bank.text(), ["NORMALS", NORMALS_LINE], "{mode:?} {preset:?}");
            let font = RomFont::new(&a.gfx4, &a.maincpu);
            let panel = without_bank.panel(&font);
            let centred = |t: &str| (MOVES_COLS - t.chars().count()) / 2;
            assert!(panel_text(&panel, &a, centred("[ NORMALS ]"), 3, "[ NORMALS ]", ink::BLUE), "{mode:?} {preset:?}");
            assert!(panel_text(&panel, &a, centred(NORMALS_LINE), 4, NORMALS_LINE, ink::BLUE), "{mode:?} {preset:?}");
            // No section, no cursor row.
            assert!(!panel_text(&panel, &a, centred("[ SPECIALS ]"), 3, "[ SPECIALS ]", ink::BLUE));
            assert!(without_bank.selected().is_none());
        }
    }
}

#[test]
fn the_fight_beside_the_panel_is_drawn_as_it_was_and_the_pause_box_comes_back() {
    let Some(_) = assets() else { return };
    for preset in PRESETS {
        let mut s = fight(Mode::Local, preset, TournamentFighter::Ryu);
        for _ in 0..30 {
            s.step(idle());
        }
        let before = s.step(idle()).frame;
        // The pause box dims the fight and draws over it.
        let boxed = s.step(start()).frame;
        assert_ne!(boxed.argb, before.argb, "{preset:?}");
        s.step(idle());
        let at = PauseBox::open(false).items().iter().position(|&i| i == MenuItem::Moves).unwrap();
        for _ in 0..at {
            tap(&mut s, stick(DOWN));
        }
        // The box with its cursor on MOVES, to compare with on the way back.
        let on_moves = s.step(idle()).frame;
        let opened = s.step(confirm());
        // The list's frame is the fight as it was, whole: the panel is a layer.
        let frame = s.step(idle()).frame;
        assert!(frame.argb == before.argb, "{preset:?}: the fight beside the panel changed");
        assert!(opened.frame.argb == before.argb, "{preset:?}: the fight beside the panel changed on opening");
        assert!(opened.panel.is_some() && s.step(idle()).panel.is_some(), "{preset:?}");
        // Back: the box over the dimmed fight again.
        let back_to_box = tap(&mut s, back()).frame;
        assert!(back_to_box.argb == on_moves.argb, "{preset:?}: the box came back changed");
    }
}

#[test]
fn opening_the_list_leaves_the_machine_and_the_fight_unchanged() {
    let Some(_) = assets() else { return };
    for preset in PRESETS {
        // The stick walks forward and back by the machine's own frame count.
        let frame_of = |s: &GameSession| s.fight_machine().unwrap().state.frame;
        let script = |f: u32| with(|i| i.p1.directions[RIGHT] = f % 40 < 20);
        let mut plain = fight(Mode::Local, preset, TournamentFighter::Ryu);
        let mut looked = plain.clone();
        for _ in 0..60 {
            let (a, b) = (frame_of(&plain), frame_of(&looked));
            plain.step(script(a));
            looked.step(script(b));
        }
        // Pause, open the list, scroll it, come back, resume.
        open_moves(&mut looked, false);
        let paused = looked.fight_machine().unwrap().save_snapshot();
        for _ in 0..3 {
            tap(&mut looked, stick(DOWN));
        }
        tap(&mut looked, stick(UP));
        assert_eq!(looked.fight_machine().unwrap().save_snapshot(), paused, "{preset:?}");
        tap(&mut looked, back());
        looked.step(back()); // Back resumes from the box; that frame holds the fight
        // Both play on: the same frames, the same machines.
        assert_eq!(frame_of(&plain), frame_of(&looked), "{preset:?}");
        for _ in 0..120 {
            let f = frame_of(&plain);
            assert_eq!(f, frame_of(&looked));
            let (p, l) = (plain.step(script(f)), looked.step(script(f)));
            assert!(p.frame.argb == l.frame.argb, "{preset:?} machine frame {f}");
        }
        assert_eq!(
            plain.fight_machine().unwrap().work_ram(),
            looked.fight_machine().unwrap().work_ram(),
            "{preset:?}"
        );
    }
}

#[test]
fn every_listed_move_comes_out_through_shows_input_path_under_every_preset() {
    let Some(_) = assets() else { return };
    for preset in RulesetPreset::ALL {
        for fighter in [TournamentFighter::Ryu, TournamentFighter::Ken] {
            // Faithful 1987 binds Ryu to P1: Ken would run as Ryu.
            if fighter == TournamentFighter::Ken && preset == RulesetPreset::Faithful1987 {
                continue;
            }
            let mut ready = fight(Mode::Dummy, preset, fighter);
            assert_eq!(ready.fight().unwrap().setup().config.roster.p1, fighter, "{preset:?}");
            // The round's intro, until the fighters take control.
            for _ in 0..240 {
                ready.step(idle());
            }
            for (row, entry) in move_list(fighter, preset).into_iter().enumerate() {
                let mut s = ready.clone();
                // The list's row, then SHOW: the fed input is P1's player input.
                open_moves(&mut s, true);
                for _ in 0..row {
                    tap(&mut s, stick(DOWN));
                }
                tap(&mut s, confirm());
                assert_eq!(s.move_list().unwrap().selected().map(|e| e.name), Some(entry.name));
                let out = s.step(confirm());
                assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)), "SHOW resumes the fight");
                let Recognizer::Tracker(own_tracker) = entry.recognizer;
                let reached = |m: &sf_engine::run::PowerOnMachine, t: Tracker1987| {
                    let st = Recognizer::Tracker(t).read(m.work_ram(), P1_STRUCT);
                    st.stage >= 3 || st.done
                };
                let steps = entry.input.strip_steps().len();
                let (mut came_out, mut own_done_before_button, mut f) = (false, false, 0);
                while s.fight().unwrap().showing().is_some() {
                    s.step(idle());
                    f += 1;
                    assert!(f < 600, "{}'s show never ended ({fighter:?}, {preset:?})", entry.name);
                    let m = s.fight_machine().unwrap();
                    // The button is fed once the strip has every step lit.
                    let button_fed = matches!(
                        s.fight().unwrap().showing().map(|show| show.strip()),
                        Some(sf_video::callouts::Strip::Input { lit, .. }) if lit == steps
                    );
                    if !button_fed && reached(m, own_tracker) {
                        own_done_before_button = true;
                    }
                    // The other two never do: the input is this move's alone.
                    for t in [Tracker1987::Tatsumaki, Tracker1987::Shoryuken, Tracker1987::Hadoken] {
                        if t != own_tracker {
                            assert!(!reached(m, t), "{}'s input completed {t:?} (frame {f}) for {fighter:?} under {preset:?}", entry.name);
                        }
                    }
                    came_out |= s.fight().unwrap().fight_read().fighters[0].move_id == entry.produces;
                }
                assert!(came_out, "{} did not come out for {fighter:?} under {preset:?}", entry.name);
                assert!(own_done_before_button, "{}'s own tracker never completed before the button ({fighter:?}, {preset:?})", entry.name);
            }
        }
    }
}

#[test]
fn every_registry_entry_is_listed_for_someone() {
    // The registry holds nothing nobody can see.
    for entry in REGISTRY {
        assert!(
            RulesetPreset::ALL
                .iter()
                .any(|&p| TournamentFighter::ALL.iter().any(|&f| entry.available.includes(f, p))),
            "{}",
            entry.name
        );
    }
}

#[test]
fn start_in_the_list_resumes_the_fight_and_a_lost_pad_closes_it_to_the_pause_box() {
    let Some(a) = assets() else { return };
    // Start resumes straight from the list; the fight steps on as after RESUME.
    let mut s = fight(Mode::Local, RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu);
    open_moves(&mut s, false);
    let frame = s.fight_machine().unwrap().state.frame;
    let out = tap(&mut s, start());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
    assert!(s.move_list().is_none());
    assert!(s.fight_machine().unwrap().state.frame > frame, "the fight runs again");
    assert!(out.panel.is_none());

    // A pad lost with the list up: the list gives way to the pause box,
    // which shows the notice.
    let mut s = fight(Mode::Local, RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu);
    open_moves(&mut s, false);
    let out = s.step(with(|i| i.pad_lost[0] = true));
    assert!(s.move_list().is_none());
    assert!(out.panel.is_none());
    assert_eq!(out.screen, "pause");
    let notice = "P1 PAD DISCONNECTED";
    let col = 10 + (28 - notice.len()) / 2;
    assert!(text_in(&out.frame.argb, 384, &a, col, 8, notice, ink::YELLOW), "the notice shows");
}

fn p2_of(s: &GameSession) -> TournamentFighter {
    s.fight().unwrap().setup().config.roster.p2
}

fn pad_start() -> MenuKeys {
    MenuKeys { start: true, ..MenuKeys::default() }
}

fn pad_tab() -> FrameInputs {
    with(|i| i.pads[0] = Some(MenuKeys { tab_right: true, ..MenuKeys::default() }))
}

/// Pause with `opener`, then choose MOVES (the keyboard's stick moves the
/// cursor); returns the list's fighter.
fn opened_by(s: &mut GameSession, opener: FrameInputs) -> TournamentFighter {
    assert_eq!(s.step(opener).transition, Some(Transition::Paused));
    s.step(idle());
    let at = PauseBox::open(false).items().iter().position(|&i| i == MenuItem::Moves).expect("a MOVES row");
    for _ in 0..at {
        tap(s, stick(DOWN));
    }
    s.step(confirm());
    s.step(idle());
    s.move_list().expect("the list is up").fighter()
}

/// The panel's header (the frame's top edge) is the one drawn for
/// `fighter`: the same pixels as a list opened for them.
fn header_names(s: &mut GameSession, a: &Assets, fighter: TournamentFighter) -> bool {
    let live = s.step(idle()).panel.expect("the panel");
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let want = MovesPanel::open(fighter, RulesetPreset::DefinitiveOverhaul).panel(&font);
    let row = live.width * 8;
    live.argb[..row] == want.argb[..row]
}

#[test]
fn the_side_whose_pad_paused_gets_its_fighter_and_a_key_or_a_lost_pad_gets_p1s() {
    let Some(a) = assets() else { return };
    let ryu = TournamentFighter::Ryu;
    let preset = RulesetPreset::DefinitiveOverhaul;
    let ken = p2_of(&fight_vs(Mode::Local, preset, ryu, &[]));
    assert_ne!(ken, ryu, "Player 2 has a fighter of its own");

    // P2's pad: P2's fighter, named in the header.
    let mut s = fight_vs(Mode::Local, preset, ryu, &[]);
    assert_eq!(opened_by(&mut s, with(|i| i.pads[1] = Some(pad_start()))), ken);
    assert!(header_names(&mut s, &a, ken));

    // P1's pad, the keyboard's Start, Esc (back) and a lost pad: P1's.
    for opener in [
        with(|i| i.pads[0] = Some(pad_start())),
        start(),
        back(),
        with(|i| i.pad_lost[1] = true),
    ] {
        let mut s = fight_vs(Mode::Local, preset, ryu, &[]);
        let lost = opener.pad_lost[1];
        if lost {
            // A lost pad opens the box with its notice; choose MOVES then.
            assert_eq!(s.step(opener).transition, Some(Transition::Paused));
            s.step(idle());
            let at = PauseBox::open(false).items().iter().position(|&i| i == MenuItem::Moves).unwrap();
            for _ in 0..at {
                tap(&mut s, stick(DOWN));
            }
            s.step(confirm());
            s.step(idle());
            assert_eq!(s.move_list().expect("the list").fighter(), ryu, "a lost pad is P1's");
        } else {
            assert_eq!(opened_by(&mut s, opener), ryu);
            assert!(header_names(&mut s, &a, ryu));
        }
    }
}

#[test]
fn the_tab_pair_switches_the_list_in_local_and_does_nothing_against_the_cpu() {
    let Some(a) = assets() else { return };
    let ryu = TournamentFighter::Ryu;
    let preset = RulesetPreset::DefinitiveOverhaul;
    let ken = p2_of(&fight_vs(Mode::Local, preset, ryu, &[]));
    assert_ne!(ken, ryu, "Player 2 has a fighter of its own");

    // Local: the tab pair switches both ways, from either opener, with the
    // header following.
    let mut s = fight_vs(Mode::Local, preset, ryu, &[]);
    assert_eq!(opened_by(&mut s, start()), ryu);
    for want in [ken, ryu, ken] {
        let out = tap(&mut s, pad_tab());
        assert_eq!(out.transition, Some(Transition::CursorMoved));
        assert_eq!(s.move_list().unwrap().fighter(), want);
        assert!(header_names(&mut s, &a, want));
    }
    // Back returns to the pause box, and the next opening is the opener's again.
    tap(&mut s, back());
    tap(&mut s, with(|i| i.menu.tab_left = true));
    assert!(s.move_list().is_none(), "the tab pair does nothing on the pause box");

    // The tab-left key switches too.
    let mut s = fight_vs(Mode::Local, preset, ryu, &[]);
    opened_by(&mut s, with(|i| i.pads[1] = Some(pad_start())));
    assert_eq!(s.move_list().unwrap().fighter(), ken);
    tap(&mut s, with(|i| i.menu.tab_left = true));
    assert_eq!(s.move_list().unwrap().fighter(), ryu);

    // Against the CPU (Arcade one player, Training): the player's list only,
    // whichever pad paused, and the tab pair changes nothing.
    for mode in [Mode::Arcade, Mode::Training] {
        let mut s = fight(mode, preset, ryu);
        let training = mode == Mode::Training;
        assert_eq!(s.step(with(|i| i.pads[1] = Some(pad_start()))).transition, Some(Transition::Paused));
        s.step(idle());
        let at = PauseBox::open(training).items().iter().position(|&i| i == MenuItem::Moves).unwrap();
        for _ in 0..at {
            tap(&mut s, stick(DOWN));
        }
        s.step(confirm());
        s.step(idle());
        let before = s.move_list().expect("the list").clone();
        assert_eq!(before.fighter(), ryu, "{mode:?}: the player's fighter");
        for tab in [pad_tab(), with(|i| i.menu.tab_left = true)] {
            let out = tap(&mut s, tab);
            assert_eq!(out.transition, None, "{mode:?}");
            assert_eq!(s.move_list(), Some(&before), "{mode:?}: the tab pair does nothing");
        }
    }
}
