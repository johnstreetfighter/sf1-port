//! Ticket #157 (`FE7`): the pause box and Training.
//!
//! Headless walks through the game session: the pause box in Local (from
//! the keyboard and from either player's Start), its rows (resume equal to
//! never pausing, the overlay modes, restart round bit for bit, quit to
//! the menu with the attract as it was), Training against a CPU (the one
//! a flag brings, a fight that runs on with no match end), Training
//! against a dummy (whose P2 side never moves), and Training's pause box (ticket #242,
//! TR6): its eight rows, the Training settings box that `TRAINING` opens
//! (the live preset, `HOLD COMBO` and `FILL METERS`, ticket #245, TR7), and
//! `RESET POSITIONS`. Local ROMs only; each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{drive_to_country_select, flow_position, pick_country, Players};
use sf_engine::front::fight::{OverlayMode, FIRST_OPPONENT_TABLES};
use sf_engine::front::pause::{next_live_preset, short_look_name, short_preset_name, PauseBox, TrainingBox, LOOK_CELLS};
use sf_engine::front::settings::{LookName, Settings, DEFAULT_LOOK, LOOK_OFF};
use sf_engine::front::{FightDips, FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::run::PowerOnMachine;
use sf_engine::tasks::Io;
use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::decode::decode_char_tile;
use sf_video::era_hud::GaugeArt;
use sf_video::render::{render_frame, Frame, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;

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

const DOWN: usize = 2;

fn down() -> FrameInputs {
    with(|i| i.p1.directions[DOWN] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn start() -> FrameInputs {
    with(|i| i.menu.start = true)
}

fn table(m: &PowerOnMachine, ps: u32) -> u32 {
    let a = (ps + 0x12 - 0xff8000) as usize;
    u32::from_be_bytes(m.work_ram()[a..a + 4].try_into().unwrap())
}

/// Step until the fight's flow reaches the fight loop.
fn to_fight_loop(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

/// Power-on, the Main Menu, Multi Player, Local; P1 Ryu, P2 Ken; Japan;
/// the first fight loop.
fn local_fight(a: Assets) -> GameSession {
    local_fight_with(a, Settings::default())
}

fn local_fight_with(a: Assets, settings: Settings) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::default()).with_settings(settings);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, down());
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    select::confirm_fighter(&mut s, with(|i| i.p1.buttons[0] = true), tap);
    select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), tap);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    s
}

/// Power-on, the Main Menu, Single Player, Training, CPU or DUMMY; P1 Ryu,
/// then the opponent under the cursor after `moves` steps right.
fn training(a: Assets, dummy: bool, moves: usize) -> GameSession {
    training_with(a, dummy, moves, Settings::default())
}

fn training_with(a: Assets, dummy: bool, moves: usize, settings: Settings) -> GameSession {
    training_under(a, MatchConfig::default(), dummy, moves, settings)
}

fn training_under(a: Assets, config: MatchConfig, dummy: bool, moves: usize, settings: Settings) -> GameSession {
    let mut s = GameSession::new(a, config).with_settings(settings);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, down());
    assert_eq!(tap(&mut s, confirm()).screen, "training");
    if dummy {
        tap(&mut s, down());
    }
    assert_eq!(tap(&mut s, confirm()).screen, "select");
    assert_eq!(select::confirm_fighter(&mut s, confirm(), tap).transition, Some(Transition::Picked(TournamentFighter::Ryu)));
    for _ in 0..moves {
        tap(&mut s, with(|i| i.p1.directions[0] = true));
    }
    s
}

/// Whether `text` is drawn from text cell (`col`, `row`) in the ROM's 8x8
/// font in colour `color` (glyph pixels only).
fn text_at(frame: &Frame, a: &Assets, col: usize, row: usize, text: &str, color: u8) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| {
            (0..8).all(|tx| {
                let px = frame.argb[(row * 8 + ty) * WIDTH + (col + i) * 8 + tx];
                tile[ty][tx] == 3 || px == font.pen(color, tile[ty][tx])
            })
        })
    })
}

#[test]
fn pause_opens_from_every_start_steps_nothing_and_resume_equals_never_pausing() {
    let Some(a) = assets() else { return };
    let mut s = local_fight(a);
    for opener in [
        start(),
        with(|i| i.menu.back = true),
        with(|i| i.cabinet[2] = true),
        with(|i| i.cabinet[3] = true),
    ] {
        for _ in 0..30 {
            s.step(idle());
        }
        let never = s.clone();
        let frame = s.fight_machine().unwrap().state.frame;
        let out = s.step(opener);
        assert_eq!((out.screen, out.transition), ("pause", Some(Transition::Paused)));
        for _ in 0..90 {
            assert_eq!(s.step(idle()).screen, "pause");
        }
        assert_eq!(s.fight_machine().unwrap().state.frame, frame, "no frame steps under the box");
        let out = tap(&mut s, start());
        assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
        assert_eq!(s.screen(), "fight");
        // The paused game and one that never paused are the same machine
        // once both have stepped the same frames.
        let mut never = never;
        let target = s.fight_machine().unwrap().state.frame;
        while never.fight_machine().unwrap().state.frame < target {
            never.step(idle());
        }
        assert_eq!(
            s.fight_machine().unwrap().save_snapshot(),
            never.fight_machine().unwrap().save_snapshot(),
            "resume equals never having paused"
        );
    }
    // P2's buttons are not Start: LK never opens the box in a fight.
    let out = s.step(with(|i| i.p2.buttons[3] = true));
    assert_ne!(out.screen, "pause");
}

#[test]
fn overlays_restart_round_and_quit_to_menu() {
    let Some(a) = assets() else { return };
    let assets = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = local_fight(a);
    let attract = s.attract_machine().save_snapshot();
    // OVERLAYS cycles off, hitboxes, frame meter, both.
    tap(&mut s, start());
    tap(&mut s, down());
    for mode in [OverlayMode::Hitboxes, OverlayMode::FrameMeter, OverlayMode::Both, OverlayMode::Off, OverlayMode::Hitboxes] {
        assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::Overlays)));
        assert_eq!(s.fight().unwrap().overlay(), mode);
    }
    tap(&mut s, start()); // resume with the hitboxes on
    let out = s.step(idle());
    let plain = render_frame(&assets, &s.fight_machine().unwrap().state.scene());
    assert_ne!(out.frame.argb, plain.argb, "the hitboxes are drawn over the fight");

    // RESTART ROUND restores the round's start bit for bit.
    for _ in 0..300 {
        s.step(with(|i| i.p1.directions[0] = true));
    }
    let round_start = s.fight().unwrap().round_start().expect("a round has started").clone();
    assert_ne!(s.fight_machine().unwrap().save_snapshot(), round_start);
    tap(&mut s, start());
    for _ in 0..4 {
        tap(&mut s, down());
    }
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::RestartRound)));
    assert_eq!(out.screen, "fight");
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), round_start, "bit for bit");
    s.step(idle());

    // QUIT TO MENU: the Main Menu at once, the attract as it was.
    tap(&mut s, start());
    for _ in 0..5 {
        tap(&mut s, down());
    }
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::QuitToMenu)));
    assert_eq!(out.screen, "main menu");
    assert!(s.fight_machine().is_none());
    assert_eq!(s.attract_machine().save_snapshot(), attract);
}

#[test]
fn the_gauge_art_row_changes_the_live_fight_and_the_settings_value() {
    let Some(a) = assets() else { return };
    // A fight opens in the Settings value.
    let settings = Settings { gauge_art: GaugeArt::Blocks, ..Settings::default() };
    let mut s = local_fight_with(load_assets(ROMS, MANIFEST).unwrap(), settings);
    assert_eq!(s.fight().unwrap().gauge_art(), GaugeArt::Blocks);
    assert!(!s.step(idle()).settings_changed);
    // The row sits under LOOK and says the value.
    let out = s.step(start());
    s.step(idle());
    let col = PauseBox::label_col();
    assert!(text_at(&out.frame, &a, col, PauseBox::item_row(3), "GAUGE ART", ink::BLUE));
    for _ in 0..3 {
        tap(&mut s, down());
    }
    // Each choice changes both, flags the save, and the box shows it.
    for art in [GaugeArt::Plate, GaugeArt::Ruled, GaugeArt::Blocks] {
        let out = tap(&mut s, confirm());
        assert_eq!(out.transition, Some(Transition::Choose(MenuItem::GaugeArt)));
        assert!(out.settings_changed, "{art:?}: the settings file is saved");
        assert_eq!(s.fight().unwrap().gauge_art(), art);
        assert_eq!(s.settings().gauge_art, art);
        assert_eq!(s.screen(), "pause");
    }
    let shown = s.step(idle());
    assert!(text_at(&shown.frame, &a, 36 - "BLOCKS".len(), PauseBox::item_row(3), "BLOCKS", ink::YELLOW));
    // The match config, which netplay carries, has no such field.
    assert!(!s.fight().unwrap().setup().config.to_toml().contains("gauge_art"));
}

const RIGHT: usize = 0;
const LEFT: usize = 1;

fn look(id: &str, name: &str) -> LookName {
    LookName { id: id.into(), name: name.into() }
}

#[test]
fn the_look_row_sits_above_gauge_art_in_every_pause_box() {
    for training in [false, true] {
        let items = PauseBox::open(training).items();
        assert_eq!(
            items[..4],
            [MenuItem::Resume, MenuItem::Overlays, MenuItem::Look, MenuItem::GaugeArt],
            "training: {training}"
        );
    }
    assert_eq!(MenuItem::Look.label(), "LOOK");
    // Long names are cut to the cells between the label and the box's edge.
    assert_eq!(short_look_name("arcade cabinet"), "ARCADE CABINET");
    let long = short_look_name("my own crt look with a very long name");
    assert_eq!(long.chars().count(), LOOK_CELLS);
    assert!(long.ends_with("..") && long.starts_with("MY OWN CRT LOOK "), "{long}");
    // The label (4 cells), a gap and the value fit the box's 28 cells.
    assert!(PauseBox::label_col() + "LOOK".len() + 1 + LOOK_CELLS <= 36);
}

#[test]
fn the_look_row_steps_the_full_list_over_the_live_fight_and_saves_as_a_settings_change() {
    let Some(a) = assets() else { return };
    let long = look("saved/my_own_crt_look_with_a_very_long_name", "MY OWN CRT LOOK WITH A VERY LONG NAME");
    let looks = vec![look(LOOK_OFF, "OFF"), look(DEFAULT_LOOK, "ARCADE CABINET"), look("pro_monitor", "PRO MONITOR"), long.clone()];
    let mut s = local_fight(load_assets(ROMS, MANIFEST).unwrap()).with_looks(looks.clone());
    assert_eq!(s.video().look, DEFAULT_LOOK, "a fight opens in the Settings look");
    let out = s.step(start());
    s.step(idle());
    let col = PauseBox::label_col();
    assert!(text_at(&out.frame, &a, col, PauseBox::item_row(2), "LOOK", ink::BLUE));
    assert!(text_at(&out.frame, &a, 36 - "ARCADE CABINET".len(), PauseBox::item_row(2), "ARCADE CABINET", ink::BLUE));
    for _ in 0..2 {
        tap(&mut s, down());
    }
    let paused = s.step(idle()).frame;
    // Right steps on through the whole list and wraps; left steps back. Each
    // choice reaches the very frame that takes it (`video`) and the Settings
    // value, and flags the save, with the fight behind the box untouched.
    let steps = [
        (RIGHT, "pro_monitor"),
        (RIGHT, long.id.as_str()),
        (RIGHT, LOOK_OFF),
        (RIGHT, DEFAULT_LOOK),
        (LEFT, LOOK_OFF),
        (LEFT, long.id.as_str()),
    ];
    for (press, want) in steps {
        let out = s.step(with(|i| i.p1.directions[press] = true));
        s.step(idle());
        assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Look)), "{want}");
        assert!(out.settings_changed, "{want}: the settings file is saved");
        assert_eq!(out.video.look, want, "applies in the step that changes it");
        assert_eq!(s.settings().crt_look, want, "the Settings value");
        assert_eq!(s.screen(), "pause");
    }
    // The long name is shortened to fit; the label stays clear of it.
    let shown = s.step(idle()).frame;
    let short = short_look_name(&long.name);
    assert!(short.chars().count() <= LOOK_CELLS);
    assert!(text_at(&shown, &a, 36 - short.chars().count(), PauseBox::item_row(2), &short, ink::YELLOW));
    assert!(text_at(&shown, &a, col, PauseBox::item_row(2), "LOOK", ink::YELLOW));
    // The fight behind the box is the look's to draw, not the engine's: every
    // other row of the frame is as it was before the look was touched.
    let (row, rows) = (PauseBox::item_row(2) * 8, 8);
    for (i, (a, b)) in paused.argb.iter().zip(&shown.argb).enumerate() {
        let y = i / WIDTH;
        assert!(a == b || (row..row + rows).contains(&y), "pixel row {y} changed");
    }
    // The match config, which netplay carries, has no look.
    assert!(!s.fight().unwrap().setup().config.to_toml().contains("crt_look"));
    // Resuming keeps the look.
    tap(&mut s, start());
    assert_eq!(s.video().look, long.id);
}

#[test]
fn the_first_cpu_opponent_of_each_flag_is_the_same_on_every_difficulty() {
    let Some(a) = assets() else { return };
    for difficulty in 1..=4u8 {
        for flag in 0..4u8 {
            let mut cfg = MatchConfig::default();
            cfg.roster.mode = PlayMode::Training;
            let dips = FightDips { difficulty, round_time: 100 };
            let io = Io { dsw1: FightDips::DSW1, dsw2: dips.dsw2(), ..Io::idle() };
            let mut m = PowerOnMachine::with_io(a.maincpu.clone(), io).with_match_config(cfg);
            drive_to_country_select(&mut m, Players::One, 4_000).unwrap();
            pick_country(&mut m, &a.maincpu, flag, 600).unwrap();
            while flow_position(&m) != FIGHT_LOOP {
                m.step();
            }
            assert_eq!(table(&m, CPU_STRUCT), FIRST_OPPONENT_TABLES[flag as usize], "difficulty {difficulty}, flag {flag}");
        }
    }
}

#[test]
fn training_against_a_cpu_fights_the_chosen_opponent_and_runs_on_with_no_match_end() {
    let Some(a) = assets() else { return };
    // Under Faithful 1987 Training's CPU is Capcom's (under the overhaul
    // presets it is the fair CPU, `tests/fair_cpu.rs`). The CPU's tiles:
    // only the four first opponents are live.
    let mut s = training_under(a, MatchConfig::faithful(), false, 0, Settings::default());
    // The cursor starts on the first live opponent; one step right takes
    // it to the next live one, skipping the greyed tiles between.
    let out = tap(&mut s, with(|i| i.p1.directions[0] = true));
    assert_eq!(out.transition, Some(Transition::CursorMoved));
    let out = s.step(confirm());
    let Some(Transition::Fighters(p1, opponent)) = out.transition else {
        panic!("Training picks two fighters, got {:?}", out.transition)
    };
    assert_eq!(p1, TournamentFighter::Ryu);
    let flag = FIRST_OPPONENT_TABLES.iter().position(|&t| t == opponent.move_table());
    assert!(flag.is_some(), "{opponent:?} is a flag's first opponent");
    assert_eq!(out.screen, "drive");
    to_fight_loop(&mut s);
    let m = s.fight_machine().unwrap();
    assert_eq!(table(m, P1_STRUCT), TournamentFighter::Ryu.move_table());
    assert_eq!(table(m, CPU_STRUCT), opponent.move_table(), "the chosen CPU");
    assert_eq!(m.match_config().roster.mode, PlayMode::Training);
    // Standing still, P1 is beaten on by the CPU for longer than a whole
    // 1987 match takes, and the fight runs on: Training has no rounds
    // (spec #238), so there is no after-match box to reach.
    for _ in 0..6_000 {
        assert_eq!(s.step(idle()).screen, "fight");
    }
    let m = s.fight_machine().unwrap();
    assert!(!m.match_over());
    assert_eq!(flow_position(m), FIGHT_LOOP);
    assert_eq!(table(m, CPU_STRUCT), opponent.move_table(), "the same opponent");
    assert!(m.unrecovered().is_empty());
}

#[test]
fn a_training_dummy_never_moves_and_the_fight_runs_on_with_no_match_end() {
    let Some(a) = assets() else { return };
    let mut s = training(a, true, 0); // P1 Ryu, the dummy Ken (where its cursor starts)
    let out = select::confirm_fighter(&mut s, confirm(), GameSession::step);
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, TournamentFighter::Ken)));
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    let m = s.fight_machine().unwrap();
    assert_eq!((table(m, P1_STRUCT), table(m, P2_STRUCT)), (TournamentFighter::Ryu.move_table(), TournamentFighter::Ken.move_table()));
    // P2's stick and buttons reach nothing: the same frames with and
    // without them leave the same machine.
    let mut quiet = s.clone();
    for f in 0..300u32 {
        let p2 = with(|i| {
            i.p2.directions[1] = f % 40 < 30;
            i.p2.buttons[(f / 10 % 6) as usize] = f % 10 < 3;
        });
        s.step(p2);
        quiet.step(idle());
    }
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), quiet.fight_machine().unwrap().save_snapshot());
    // P1 walks in and attacks for longer than a 1987 match against the
    // dummy took, and the fight runs on with no match end.
    for f in 0..6_000u32 {
        let phase = f % 120;
        let out = s.step(with(|i| {
            i.p1.directions[0] = phase < 40;
            if phase >= 40 && phase % 10 < 3 {
                i.p1.buttons[((f / 10) % 6) as usize] = true;
            }
        }));
        assert_eq!(out.screen, "fight");
    }
    let m = s.fight_machine().unwrap();
    assert!(!m.match_over());
    assert_eq!(flow_position(m), FIGHT_LOOP);
    assert!(m.unrecovered().is_empty());
}


/// P1 Ryu against the dummy Ken, at the first fight loop.
fn dummy_fight(a: Assets) -> GameSession {
    dummy_fight_with(a, Settings::default())
}

fn dummy_fight_with(a: Assets, settings: Settings) -> GameSession {
    let mut s = training_with(a, true, 0, settings);
    select::confirm_fighter(&mut s, confirm(), GameSession::step);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    s
}

fn back() -> FrameInputs {
    with(|i| i.menu.back = true)
}

fn up() -> FrameInputs {
    with(|i| i.p1.directions[UP] = true)
}

const UP: usize = 3;

/// Training's pause box rows by name.
const RESET_POSITIONS_ROW: usize = 4;
const TRAINING_ROW: usize = 6;

/// Open the pause box and put the cursor on `row`; the box's last frame.
fn pause_on(s: &mut GameSession, row: usize) -> FrameOutput {
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    s.step(idle());
    for _ in 0..row {
        tap(s, down());
    }
    s.step(idle())
}

/// Whether the TRAINING title is drawn at the settings box's top.
fn title_drawn(frame: &Frame, a: &Assets) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let blank = 0xff00_0000;
    let mut reference = vec![blank; frame.argb.len()];
    let (col, row) = TrainingBox::title_cell();
    font.header(&mut reference, col, row, "TRAINING", ink::BLUE);
    let drawn = reference.iter().filter(|&&p| p != blank).count();
    drawn > 0 && reference.iter().zip(&frame.argb).all(|(&want, &got)| want == blank || want == got)
}

#[test]
fn trainings_pause_box_has_eight_rows_with_moves_last_and_the_other_fights_keep_seven() {
    assert_eq!(
        PauseBox::open(true).items(),
        [
            MenuItem::Resume,
            MenuItem::Overlays,
            MenuItem::Look,
            MenuItem::GaugeArt,
            MenuItem::ResetPositions,
            MenuItem::QuitToMenu,
            MenuItem::TrainingSettings,
            MenuItem::Moves,
        ]
    );
    assert_eq!(
        PauseBox::open(false).items(),
        [
            MenuItem::Resume,
            MenuItem::Overlays,
            MenuItem::Look,
            MenuItem::GaugeArt,
            MenuItem::RestartRound,
            MenuItem::QuitToMenu,
            MenuItem::Moves,
        ]
    );
    assert_eq!(MenuItem::ResetPositions.label(), "RESET POSITIONS");
    assert_eq!(MenuItem::TrainingSettings.label(), "TRAINING");
    assert_eq!(MenuItem::RestartRound.label(), "RESTART ROUND");
    // The eighth row and the box's foot are on the 28-row screen.
    assert!(PauseBox::item_row(7) + 2 <= 28);
    // The settings box sits inside the pause box, FILL METERS under HOLD COMBO.
    assert_eq!(TrainingBox::items(), [MenuItem::LivePreset, MenuItem::HoldCombo, MenuItem::FillMeters,
        MenuItem::TrainingDrill, MenuItem::ComboTrial]);
    assert_eq!(MenuItem::FillMeters.label(), "FILL METERS");
    assert!(TrainingBox::item_row(4) < TrainingBox::bottom_row());
    assert!(TrainingBox::bottom_row() < PauseBox::item_row(7));
}

#[test]
fn the_pause_boxes_draw_trainings_eight_rows_and_locals_seven() {
    let Some(a) = assets() else { return };
    let font_assets = load_assets(ROMS, MANIFEST).unwrap();
    let col = PauseBox::label_col();
    // Training: the rows in order, the cursor's yellow, no PRESET or HOLD
    // COMBO row (they are in the settings box).
    let mut s = dummy_fight(a);
    let out = s.step(start());
    let rows = ["RESUME", "OVERLAYS", "LOOK", "GAUGE ART", "RESET POSITIONS", "QUIT TO MENU", "TRAINING", "MOVES"];
    for (i, label) in rows.iter().enumerate() {
        let color = if i == 0 { ink::YELLOW } else { ink::BLUE };
        assert!(text_at(&out.frame, &font_assets, col, PauseBox::item_row(i), label, color), "row {i}: {label}");
    }
    assert!(!text_at(&out.frame, &font_assets, col, PauseBox::item_row(4), "RESTART ROUND", ink::BLUE));
    // Local: RESTART ROUND, QUIT TO MENU, MOVES, and nothing under it.
    let mut s = local_fight(load_assets(ROMS, MANIFEST).unwrap());
    let out = s.step(start());
    let rows = ["RESUME", "OVERLAYS", "LOOK", "GAUGE ART", "RESTART ROUND", "QUIT TO MENU", "MOVES"];
    for (i, label) in rows.iter().enumerate() {
        let color = if i == 0 { ink::YELLOW } else { ink::BLUE };
        assert!(text_at(&out.frame, &font_assets, col, PauseBox::item_row(i), label, color), "Local row {i}: {label}");
    }
    assert!(!text_at(&out.frame, &font_assets, col, PauseBox::item_row(6), "TRAINING", ink::BLUE));
    assert!(!text_at(&out.frame, &font_assets, col, PauseBox::item_row(7), "MOVES", ink::BLUE));
}

#[test]
fn the_training_row_opens_the_settings_box_over_the_pause_box() {
    let Some(a) = assets() else { return };
    let font_assets = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = dummy_fight(a);
    let preset = s.fight_machine().unwrap().match_config().preset;
    let shown = pause_on(&mut s, TRAINING_ROW);
    assert!(!title_drawn(&shown.frame, &font_assets), "no settings box yet");
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::TrainingSettings)));
    assert_eq!(out.screen, "pause", "it is the pause box's box: the fight stays held");
    assert!(title_drawn(&out.frame, &font_assets), "the box is titled TRAINING");
    let (col, rows) = (TrainingBox::label_col(), [0, 1, 2].map(TrainingBox::item_row));
    assert!(text_at(&out.frame, &font_assets, col, rows[0], "PRESET", ink::YELLOW), "the cursor starts on PRESET");
    assert!(text_at(&out.frame, &font_assets, col, rows[1], "HOLD COMBO", ink::BLUE));
    assert!(text_at(&out.frame, &font_assets, col, rows[2], "FILL METERS", ink::BLUE), "FILL METERS is the third row");
    let (value, end) = (short_preset_name(preset), TrainingBox::value_end_col());
    assert!(text_at(&out.frame, &font_assets, end - value.len(), rows[0], value, ink::YELLOW), "the preset's name");
    assert!(text_at(&out.frame, &font_assets, end - 3, rows[1], "OFF", ink::BLUE), "HOLD COMBO's value");
    assert!(text_at(&out.frame, &font_assets, end - 3, rows[2], "OFF", ink::BLUE), "FILL METERS' value");
    // It covers the pause box's rows under it, and leaves the foot: the
    // cursor on TRAINING, and MOVES.
    assert!(!text_at(&out.frame, &font_assets, PauseBox::label_col(), PauseBox::item_row(2), "LOOK", ink::BLUE));
    assert!(text_at(&out.frame, &font_assets, PauseBox::label_col(), PauseBox::item_row(TRAINING_ROW), "TRAINING", ink::YELLOW));
    assert!(text_at(&out.frame, &font_assets, PauseBox::label_col(), PauseBox::item_row(7), "MOVES", ink::BLUE));
}

#[test]
fn the_settings_box_moves_cycles_and_saves_its_rows_and_steps_no_frames() {
    let Some(a) = assets() else { return };
    let font_assets = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = dummy_fight(a);
    pause_on(&mut s, TRAINING_ROW);
    tap(&mut s, confirm());
    let (col, rows) = (TrainingBox::label_col(), [0, 1, 2].map(TrainingBox::item_row));
    let end = TrainingBox::value_end_col();
    let frame = s.fight_machine().unwrap().state.frame;
    let hold = |s: &GameSession| s.fight().unwrap().hold_combo();
    // Up and down move the cursor, wrapping around the box's rows.
    let out = tap(&mut s, down());
    assert!(text_at(&out.frame, &font_assets, col, rows[0], "PRESET", ink::BLUE));
    assert!(text_at(&out.frame, &font_assets, col, rows[1], "HOLD COMBO", ink::YELLOW));
    for _ in 1..TrainingBox::items().len() {
        tap(&mut s, down());
    }
    let out = tap(&mut s, up());
    assert!(text_at(&out.frame, &font_assets, col, TrainingBox::item_row(4), "TRIAL", ink::YELLOW), "up wraps to the last row");
    let out = tap(&mut s, down());
    assert!(text_at(&out.frame, &font_assets, col, rows[0], "PRESET", ink::YELLOW), "down wraps to the first");

    // PRESET: right, left and confirm each step the live preset on, in the
    // machine and in the fight, keeping the fighters.
    let mut preset = s.fight_machine().unwrap().match_config().preset;
    for press in [with(|i| i.p1.directions[RIGHT] = true), with(|i| i.p1.directions[LEFT] = true), confirm()] {
        let out = tap(&mut s, press);
        assert_eq!(out.transition, Some(Transition::Choose(MenuItem::LivePreset)));
        assert!(!out.settings_changed, "the preset is a fight's choice, not a saved setting");
        preset = next_live_preset(preset);
        assert_eq!(s.fight_machine().unwrap().match_config().preset, preset);
        assert_eq!(s.fight().unwrap().setup().config.preset, preset);
        let cfg = s.fight_machine().unwrap().match_config();
        assert_eq!((cfg.roster.p1, cfg.roster.p2), (TournamentFighter::Ryu, TournamentFighter::Ken));
    }
    let shown = s.step(idle());
    let value = short_preset_name(preset);
    assert!(text_at(&shown.frame, &font_assets, end - value.len(), rows[0], value, ink::YELLOW));

    // HOLD COMBO: confirm, left and right each flip it, and it is saved.
    tap(&mut s, down());
    assert!(!hold(&s));
    let mut want = false;
    for press in [confirm(), with(|i| i.p1.directions[LEFT] = true), with(|i| i.p1.directions[RIGHT] = true)] {
        let out = tap(&mut s, press);
        want = !want;
        assert_eq!(out.transition, Some(Transition::Choose(MenuItem::HoldCombo)));
        assert!(out.settings_changed, "saved with the settings");
        assert_eq!((hold(&s), s.settings().hold_combo), (want, want));
        let shown = s.step(idle());
        let text = if want { "ON" } else { "OFF" };
        assert!(text_at(&shown.frame, &font_assets, end - text.len(), rows[1], text, ink::YELLOW));
    }
    assert!(hold(&s), "left on");

    // FILL METERS: confirm, left and right each flip it, in the machine, the
    // fight and the saved setting, and nothing else of the configuration.
    tap(&mut s, down());
    let fill = |s: &GameSession| s.fight_machine().unwrap().match_config().fill_meters;
    let before = MatchConfig { fill_meters: false, ..*s.fight_machine().unwrap().match_config() };
    assert!(!fill(&s) && !s.settings().fill_meters);
    let mut want = false;
    for press in [confirm(), with(|i| i.p1.directions[LEFT] = true), with(|i| i.p1.directions[RIGHT] = true)] {
        let out = tap(&mut s, press);
        want = !want;
        assert_eq!(out.transition, Some(Transition::Choose(MenuItem::FillMeters)));
        assert!(out.settings_changed, "saved with the settings");
        assert_eq!((fill(&s), s.fight().unwrap().setup().config.fill_meters, s.settings().fill_meters), (want, want, want));
        assert_eq!(*s.fight_machine().unwrap().match_config(), MatchConfig { fill_meters: want, ..before });
        let shown = s.step(idle());
        let text = if want { "ON" } else { "OFF" };
        assert!(text_at(&shown.frame, &font_assets, end - text.len(), rows[2], text, ink::YELLOW));
        assert!(text_at(&shown.frame, &font_assets, col, rows[1], "HOLD COMBO", ink::BLUE), "HOLD COMBO's row is untouched");
    }
    assert!(fill(&s), "left on");
    assert!(hold(&s), "HOLD COMBO stayed on");

    // No frame steps while the box is up, whatever the pad does.
    for f in 0..90u32 {
        s.step(with(|i| {
            i.p1.buttons[(f % 6) as usize] = true;
            i.p2.directions[LEFT] = f % 2 == 0;
        }));
        assert_eq!(s.screen(), "pause");
    }
    assert_eq!(s.fight_machine().unwrap().state.frame, frame, "the fight was held all along");
}

#[test]
fn back_leaves_the_settings_box_for_the_pause_box_on_training_and_start_resumes_from_either() {
    let Some(a) = assets() else { return };
    let font_assets = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = dummy_fight(a);
    pause_on(&mut s, TRAINING_ROW);
    tap(&mut s, confirm());
    tap(&mut s, down()); // the cursor inside the box does not carry out
    let out = tap(&mut s, back());
    assert_eq!(out.transition, Some(Transition::Back));
    assert_eq!(out.screen, "pause", "back goes to the pause box, not into the fight");
    assert!(!title_drawn(&out.frame, &font_assets), "the settings box is gone");
    let (col, row) = (PauseBox::label_col(), PauseBox::item_row(TRAINING_ROW));
    assert!(text_at(&out.frame, &font_assets, col, row, "TRAINING", ink::YELLOW), "the cursor is on TRAINING");
    assert!(text_at(&out.frame, &font_assets, col, PauseBox::item_row(2), "LOOK", ink::BLUE), "the rows it covered are back");
    // Confirm opens it again, with the cursor back on PRESET.
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::TrainingSettings)));
    assert!(text_at(&out.frame, &font_assets, TrainingBox::label_col(), TrainingBox::item_row(0), "PRESET", ink::YELLOW));

    // Start in the settings box resumes the fight, nothing else changed.
    let frame = s.fight_machine().unwrap().state.frame;
    let out = tap(&mut s, start());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
    assert_eq!(out.screen, "fight");
    for _ in 0..30 {
        s.step(idle());
    }
    assert!(s.fight_machine().unwrap().state.frame > frame, "the fight runs again");
    // And from the pause box, as ever; the box opens fresh, on RESUME.
    let out = s.step(start());
    assert_eq!(out.transition, Some(Transition::Paused));
    assert!(text_at(&out.frame, &font_assets, PauseBox::label_col(), PauseBox::item_row(0), "RESUME", ink::YELLOW));
    assert!(!title_drawn(&out.frame, &font_assets));
    s.step(idle());
    assert_eq!(tap(&mut s, start()).transition, Some(Transition::Choose(MenuItem::Resume)));
    assert_eq!(s.screen(), "fight");
}

#[test]
fn reset_positions_keeps_the_preset_and_hold_combo_chosen_since_the_round_began() {
    let Some(a) = assets() else { return };
    let mut s = dummy_fight(a);
    let round_start = s.fight().unwrap().round_start().expect("a round has started").clone();
    // Nothing changed: RESET POSITIONS is a restart of the round, bit for bit.
    for _ in 0..200 {
        s.step(with(|i| i.p1.directions[RIGHT] = true));
    }
    assert_ne!(s.fight_machine().unwrap().save_snapshot(), round_start);
    pause_on(&mut s, RESET_POSITIONS_ROW);
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::ResetPositions)));
    assert_eq!(out.screen, "fight");
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), round_start, "bit for bit");
    s.step(idle());

    // PRESET and HOLD COMBO changed in the settings box, the fight played on.
    pause_on(&mut s, TRAINING_ROW);
    tap(&mut s, confirm());
    let new_preset = next_live_preset(round_start.match_config.preset);
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::LivePreset)));
    tap(&mut s, down());
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::HoldCombo)));
    tap(&mut s, start());
    for _ in 0..200 {
        s.step(with(|i| i.p1.directions[RIGHT] = true));
    }
    assert_eq!(s.fight_machine().unwrap().match_config().preset, new_preset);
    assert_ne!(round_start.match_config.preset, new_preset, "the round began under another preset");

    pause_on(&mut s, RESET_POSITIONS_ROW);
    let out = s.step(confirm());
    assert_eq!((out.screen, out.transition), ("fight", Some(Transition::Choose(MenuItem::ResetPositions))));
    // The machine and the fight both carry the new preset, the fighters are
    // back at their places on full health: the round's start, but for the
    // configuration.
    let config = s.fight().unwrap().setup().config;
    assert_eq!(config.preset, new_preset);
    assert_eq!(*s.fight_machine().unwrap().match_config(), config, "machine and fight agree");
    let mut expected = round_start.clone();
    expected.match_config = config;
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), expected);
    assert!(s.fight().unwrap().hold_combo(), "HOLD COMBO is kept too");
    // The fight carries on from there under the new preset, fighters kept.
    s.step(idle());
    for f in 0..300u32 {
        s.step(with(|i| i.p1.directions[RIGHT] = f % 60 < 30));
    }
    let m = s.fight_machine().expect("the fight goes on");
    assert_eq!(m.match_config().preset, new_preset);
    assert_eq!(table(m, P1_STRUCT), TournamentFighter::Ryu.move_table());
    assert_eq!(table(m, P2_STRUCT), TournamentFighter::Ken.move_table());
    assert!(m.unrecovered().is_empty());
}

#[test]
fn restart_round_in_local_still_restores_the_whole_snapshot() {
    let Some(a) = assets() else { return };
    let mut s = local_fight(a);
    let round_start = s.fight().unwrap().round_start().expect("a round has started").clone();
    for _ in 0..200 {
        s.step(with(|i| i.p1.directions[RIGHT] = true));
    }
    pause_on(&mut s, 4);
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::RestartRound)));
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), round_start, "bit for bit, configuration included");
}

#[test]
fn a_preset_change_keeps_fill_meters_in_the_machine_and_the_fight() {
    let Some(a) = assets() else { return };
    let mut s = dummy_fight(a);
    pause_on(&mut s, TRAINING_ROW);
    tap(&mut s, confirm());
    // FILL METERS on, then PRESET stepped twice: every preset rebuilds the
    // configuration, none drops the switch.
    tap(&mut s, down());
    tap(&mut s, down());
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::FillMeters)));
    tap(&mut s, up());
    tap(&mut s, up());
    let mut preset = s.fight_machine().unwrap().match_config().preset;
    for _ in 0..2 {
        assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::LivePreset)));
        preset = next_live_preset(preset);
        let config = s.fight().unwrap().setup().config;
        assert_eq!(config.preset, preset);
        assert!(config.fill_meters, "the fight keeps FILL METERS");
        assert_eq!(*s.fight_machine().unwrap().match_config(), config, "machine and fight agree");
    }
    // And turned off again under the new preset, it is off in both.
    tap(&mut s, down());
    tap(&mut s, down());
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::FillMeters)));
    assert!(!s.fight().unwrap().setup().config.fill_meters);
    assert!(!s.fight_machine().unwrap().match_config().fill_meters);
    assert_eq!(s.fight_machine().unwrap().match_config().preset, preset);
}

#[test]
fn reset_positions_after_turning_fill_meters_on_leaves_it_on_in_the_machine_the_fight_and_the_box() {
    let Some(a) = assets() else { return };
    let font_assets = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = dummy_fight(a);
    let round_start = s.fight().unwrap().round_start().expect("a round has started").clone();
    assert!(!round_start.match_config.fill_meters, "the round began with it off");
    pause_on(&mut s, TRAINING_ROW);
    tap(&mut s, confirm());
    tap(&mut s, down());
    tap(&mut s, down());
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::FillMeters)));
    tap(&mut s, start());
    for _ in 0..200 {
        s.step(with(|i| i.p1.directions[RIGHT] = true));
    }

    pause_on(&mut s, RESET_POSITIONS_ROW);
    let out = s.step(confirm());
    assert_eq!((out.screen, out.transition), ("fight", Some(Transition::Choose(MenuItem::ResetPositions))));
    let config = s.fight().unwrap().setup().config;
    assert!(config.fill_meters, "the fight still has it");
    assert!(s.fight_machine().unwrap().match_config().fill_meters, "the machine has it");
    assert!(s.settings().fill_meters, "and the saved setting");
    // The round's start but for the configuration.
    let mut expected = round_start.clone();
    expected.match_config = config;
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), expected);
    // The box shows it ON when it is opened again.
    s.step(idle());
    pause_on(&mut s, TRAINING_ROW);
    let out = tap(&mut s, confirm());
    let end = TrainingBox::value_end_col();
    assert!(text_at(&out.frame, &font_assets, end - 2, TrainingBox::item_row(2), "ON", ink::BLUE));
    // A second reset keeps it too.
    tap(&mut s, back());
    tap(&mut s, up());
    tap(&mut s, up());
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::ResetPositions)));
    assert!(s.fight_machine().unwrap().match_config().fill_meters);
}

#[test]
fn fill_meters_is_stamped_on_a_training_fight_from_the_saved_setting_and_means_nothing_elsewhere() {
    let Some(a) = assets() else { return };
    let on = Settings { fill_meters: true, ..Settings::default() };
    // Training against a dummy and against a CPU start with it on.
    let s = dummy_fight_with(a, on.clone());
    assert!(s.fight_machine().unwrap().match_config().fill_meters);
    assert!(s.fight().unwrap().setup().config.fill_meters);
    let mut s = training_with(load_assets(ROMS, MANIFEST).unwrap(), false, 0, on.clone());
    select::confirm_fighter(&mut s, confirm(), GameSession::step);
    to_fight_loop(&mut s);
    assert_eq!(s.fight_machine().unwrap().match_config().roster.mode, PlayMode::Training);
    assert!(s.fight_machine().unwrap().match_config().fill_meters);
    // With it off, they start off.
    let s = dummy_fight(load_assets(ROMS, MANIFEST).unwrap());
    assert!(!s.fight_machine().unwrap().match_config().fill_meters);
    // Local, whatever the setting: off in the machine and in the fight.
    let s = local_fight_with(load_assets(ROMS, MANIFEST).unwrap(), on);
    assert!(s.settings().fill_meters);
    assert_eq!(s.fight_machine().unwrap().match_config().roster.mode, PlayMode::Versus);
    assert!(!s.fight_machine().unwrap().match_config().fill_meters);
    assert!(!s.fight().unwrap().setup().config.fill_meters);
}
