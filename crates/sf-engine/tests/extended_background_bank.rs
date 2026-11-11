//! Ticket #198 (`ST5`): the extended background bank with the test stage
//! (ADR-0009), and the VS screens #215 builds on the pages after it.
//!
//! - With the bank loaded, every committed scenario from power-on is byte
//!   for byte what it is without it (874,344 frames: the machine state,
//!   work RAM and both map pages after every frame), and its frames draw
//!   the same with the bank's graphics: the 1987 game never names a page or
//!   a bank palette, and never draws a tile code past the ROM's (ST4's
//!   sweep, `background_format.rs`).
//! - The attract, the faithful cabinet and Faithful 1987 fights never load
//!   the bank, and an overhaul Ryu vs Ken fight with no stage picked (the
//!   one pair with no built VS screen) is unchanged by it.
//! - A headless Local fight on the test stage draws its backdrop, scrolls
//!   with the camera, flips with flip screen and drifts as a 1987 stage
//!   does, and plays its theme. The test stage is Joe's foreground over
//!   Geki's background (`art/stages/test-stage.json`), so every frame of it
//!   is checked against the ROM's own map drawn at the source stages'
//!   scroll positions in their ROM palettes.
//!
//! Local ROMs only; each test skips without them.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightKind, FightSetup};
use sf_engine::front::{FightDips, FrameInputs, GameSession, PlayerInputs};
use sf_engine::roster::{PlayMode, StagePick};
use sf_engine::run::{observe_power_on_scenario, observe_power_on_scenario_with, PowerOnMachine};
use sf_video::background_bank::BackgroundBank;
use sf_video::render::render_frame;
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;
use sf_video::stage::pack_records;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const SCENARIOS: &str = "../../tools/reference/scenarios";
const TEST_STAGE: &str = include_str!("../../../art/stages/test-stage.json");

/// The test stage's sources: Joe's foreground and Geki's background words,
/// and the ROM palettes their colour rows come from.
const JOE_FG: u16 = 0x2400;
const GEKI_BG: u16 = 0x2180;
const USA_FG_PALETTE: usize = 0x1a668;
const JAPAN_BG_PALETTE: usize = 0x1b068;
/// The test theme's command (`art/themes/test-theme.score`), which the test
/// stage's record names.
const TEST_THEME: u8 = 0x40;
/// The 1987 stage theme a stage plays when its own is missing.
const FALLBACK_THEME: u8 = 0x29;
/// The fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

/// The test stage, and the VS screens (#215) on the pages after it, as
/// `sf-game` loads them.
fn bank(a: &Assets) -> BackgroundBank {
    let mut bank = pack_records(a, &[TEST_STAGE]).expect("the test stage packs");
    bank.add_vs_screens(a).expect("the VS screens build");
    bank
}

fn duration(path: &Path) -> u32 {
    let text = std::fs::read_to_string(path).unwrap();
    let after = text.split("\"duration_frames\":").nth(1).expect("duration_frames");
    let digits: String = after.trim_start().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap()
}

/// Everything the machine shows or keeps after a frame: its machine-visible
/// state (registers, video, object and palette RAM, both map pages) and
/// work RAM.
fn fingerprint(m: &PowerOnMachine) -> (u32, u8, u8) {
    let s = &m.state;
    let mut h = crc32fast::Hasher::new();
    for bytes in [&s.videoram[..], &s.objectram, &s.paletteram, m.work_ram()] {
        h.update(bytes);
    }
    h.update(&s.bg_scroll.to_be_bytes());
    h.update(&s.fg_scroll.to_be_bytes());
    h.update(&[s.gfxctrl, s.flip_screen as u8, s.sound_latch]);
    h.update(&s.latch_cycles.to_be_bytes());
    (h.finalize(), s.bg_page, s.fg_page)
}

/// The complete one- and two-player games, the service mode (its scroll
/// test included), every boss, the bonus stages and the ending.
#[test]
fn every_committed_scenario_is_byte_identical_with_the_bank_loaded() {
    let Some(a) = assets() else { return };
    let bank = Arc::new(bank(&a));
    let mut paths: Vec<PathBuf> = std::fs::read_dir(SCENARIOS).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    let mut frames = 0u64;
    for p in &paths {
        let last = duration(p) - 1;
        let mut plain = Vec::new();
        let (end, unrecovered) = observe_power_on_scenario(a.maincpu.clone(), p, last, |m| plain.push(fingerprint(m))).unwrap();
        assert!(unrecovered.is_empty(), "{}: untranslated {unrecovered:05x?}", p.display());
        let banked_machine = PowerOnMachine::from_scenario(a.maincpu.clone(), p).unwrap().with_background_bank(bank.clone());
        let mut f = 0usize;
        observe_power_on_scenario_with(banked_machine, p, last, |m| {
            assert!(m.background_bank().is_some());
            let got = fingerprint(m);
            assert_eq!((got.1, got.2), (0, 0), "{} frame {f}: a page", p.display());
            assert_eq!(got, plain[f], "{} frame {f}", p.display());
            f += 1;
        })
        .unwrap();
        assert_eq!(f, plain.len(), "{}", p.display());
        frames += u64::from(end) + 1;
    }
    println!("{} scenarios, {frames} frames, each the same with the bank loaded", paths.len());
    assert!(paths.len() >= 137);
}

#[test]
fn every_1987_frame_draws_the_same_with_the_banks_graphics() {
    let Some(a) = assets() else { return };
    let banked = a.with_background_bank(&bank(&a));
    let mut compared = 0;
    for name in ["sf_full_1p_01.json", "sf_service_03b.json"] {
        let path = Path::new(SCENARIOS).join(name);
        let mut frame = 0u32;
        observe_power_on_scenario(a.maincpu.clone(), &path, duration(&path) - 1, |m| {
            frame += 1;
            if frame.is_multiple_of(97) {
                let scene = m.state.scene();
                assert_eq!(render_frame(&banked, &scene).argb, render_frame(&a, &scene).argb, "{name} frame {frame}");
                compared += 1;
            }
        })
        .unwrap();
    }
    println!("{compared} frames compared");
    assert!(compared > 400);
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

/// Power-on, the Main Menu, Multi Player, Local; both players take the
/// fighter under their cursor; Japan; the first fight loop. Returns every
/// sound latch on the way.
fn local_fight(s: &mut GameSession) -> Vec<u8> {
    for _ in 0..200 {
        s.step(idle());
    }
    let mut latches = Vec::new();
    let mut step = |s: &mut GameSession, i: FrameInputs| latches.push(s.step(i).sound_latch);
    for tap in [
        with(|i| i.menu.start = true),
        with(|i| i.p1.directions[2] = true),
        with(|i| i.menu.confirm = true),
        with(|i| i.menu.confirm = true),
        with(|i| i.p1.buttons[0] = true),
        with(|i| i.p2.buttons[0] = true),
    ] {
        step(s, tap);
        step(s, idle());
    }
    while s.screen() != "country select" {
        step(s, idle());
    }
    step(s, with(|i| i.p1.buttons[0] = true));
    step(s, idle());
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        step(s, idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    latches
}

fn session(a: Assets, preset: RulesetPreset, banked: bool, stage: bool) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    if banked {
        let b = bank(s.assets());
        let index = b.stage_index("test-stage").unwrap();
        s = s.with_background_bank(b);
        if stage {
            s.set_stage(Some(StagePick::Bank(index)));
        }
    }
    s
}

/// Ticket #199 (`ST7`): a stage whose theme the music CPU cannot play (no
/// music pack, or a pack without it) plays a 1987 stage theme, and the fight
/// starts all the same.
#[test]
fn a_stage_whose_theme_is_missing_plays_a_1987_theme_and_the_fight_starts() {
    let Some(a) = assets() else { return };
    let mut s = session(a, RulesetPreset::DefinitiveOverhaul, true, true);
    let missing = s.set_playable_themes(&[]);
    assert_eq!(missing.len(), 1);
    assert_eq!((missing[0].stage.as_str(), missing[0].theme), ("test-stage", u16::from(TEST_THEME)));
    let latches = local_fight(&mut s);
    assert_eq!(s.screen(), "fight");
    assert_eq!(flow_position(s.fight_machine().unwrap()), FIGHT_LOOP, "the fight started");
    assert!(latches.contains(&FALLBACK_THEME), "a 1987 stage theme");
    assert!(!latches.contains(&TEST_THEME), "not the missing theme's command");
    // A bank never told what the music CPU can play says the same.
    let mut unsaid = session(load_assets(ROMS, MANIFEST).unwrap(), RulesetPreset::DefinitiveOverhaul, true, true);
    let latches = local_fight(&mut unsaid);
    assert!(latches.contains(&FALLBACK_THEME) && !latches.contains(&TEST_THEME));
    // Telling it later, between fights, gives the fight its own theme.
    assert!(unsaid.set_playable_themes(&[TEST_THEME]).is_empty());
}

/// The music CPU runs with the extended tune table for a fight under an
/// overhaul preset and for nothing else.
#[test]
fn only_a_fight_under_an_overhaul_preset_asks_for_the_tune_table() {
    let Some(a) = assets() else { return };
    let mut overhaul = session(a, RulesetPreset::DefinitiveOverhaul, true, true);
    let mut faithful = session(load_assets(ROMS, MANIFEST).unwrap(), RulesetPreset::Faithful1987, true, true);
    for s in [&mut overhaul, &mut faithful] {
        let mut asked = Vec::new();
        for _ in 0..200 {
            asked.push(s.step(idle()).tune_table);
        }
        assert!(asked.iter().all(|&t| !t), "the attract");
    }
    // Every frame of the fights, the menus on the way included.
    let (mut over, mut faith) = (Vec::new(), Vec::new());
    for (s, asked) in [(&mut overhaul, &mut over), (&mut faithful, &mut faith)] {
        let mut frames = 0;
        for tap in [with(|i| i.menu.start = true), with(|i| i.p1.directions[2] = true), with(|i| i.menu.confirm = true), with(|i| i.menu.confirm = true), with(|i| i.p1.buttons[0] = true), with(|i| i.p2.buttons[0] = true)] {
            asked.push(s.step(tap).tune_table);
            asked.push(s.step(idle()).tune_table);
        }
        while s.screen() != "country select" {
            asked.push(s.step(idle()).tune_table);
        }
        asked.push(s.step(with(|i| i.p1.buttons[0] = true)).tune_table);
        while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
            asked.push(s.step(idle()).tune_table);
            frames += 1;
            assert!(frames < 3_000);
        }
        asked.push(s.step(idle()).tune_table);
    }
    assert!(faith.iter().all(|&t| !t), "Faithful 1987 never asks");
    assert!(!over[..10].iter().any(|&t| t), "not in the menus before the fighters are picked");
    assert_eq!(over.last(), Some(&true), "the overhaul fight asks");
    let first = over.iter().position(|&t| t).unwrap();
    assert!(over[first..].iter().all(|&t| t), "and keeps asking while it runs");
}

#[test]
fn the_attract_the_cabinet_and_faithful_1987_never_load_the_bank() {
    let Some(a) = assets() else { return };
    let rom = || load_assets(ROMS, MANIFEST).unwrap();
    // The attract under a banked session, a stage picked, is the plain
    // attract frame for frame, on a machine without the bank.
    let mut banked = session(a, RulesetPreset::DefinitiveOverhaul, true, true);
    let mut plain = session(rom(), RulesetPreset::DefinitiveOverhaul, false, false);
    for f in 0..300 {
        assert_eq!(banked.step(idle()).frame.argb, plain.step(idle()).frame.argb, "attract frame {f}");
    }
    assert!(banked.attract_machine().background_bank().is_none());
    // A Faithful 1987 Local fight, a stage picked, never loads it and is the
    // plain Faithful fight frame for frame.
    let mut faithful = session(rom(), RulesetPreset::Faithful1987, true, true);
    let mut plain = session(rom(), RulesetPreset::Faithful1987, false, false);
    assert_eq!(local_fight(&mut faithful), local_fight(&mut plain));
    assert!(faithful.fight_machine().unwrap().background_bank().is_none());
    assert_eq!(faithful.fight_machine().unwrap().match_config().roster.stage, None);
    assert_eq!(faithful.fight_background_pages(), Some(1));
    for f in 0..300u32 {
        let attack = with(|i| i.p1.buttons[(f / 20 % 6) as usize] = f % 20 < 3);
        assert_eq!(faithful.step(attack).frame.argb, plain.step(attack).frame.argb, "Faithful fight frame {f}");
    }
    // The faithful cabinet's machine never holds it.
    let mut s = session(rom(), RulesetPreset::DefinitiveOverhaul, true, true);
    for _ in 0..200 {
        s.step(idle());
    }
    let mut tap = |i: FrameInputs| {
        let out = s.step(i);
        s.step(idle());
        out.screen
    };
    tap(with(|i| i.menu.start = true));
    for _ in 0..3 {
        tap(with(|i| i.p1.directions[2] = true)); // to 1987
    }
    assert_eq!(tap(with(|i| i.menu.confirm = true)), "1987 cabinet");
    for _ in 0..11 {
        tap(with(|i| i.p1.directions[2] = true)); // to POWER ON
    }
    assert_eq!(tap(with(|i| i.menu.confirm = true)), "cabinet");
    for _ in 0..600 {
        s.step(idle());
    }
    assert!(s.cabinet_machine().unwrap().background_bank().is_none());
}

#[test]
fn an_overhaul_fight_loads_the_bank_and_is_unchanged_with_no_stage_picked() {
    let Some(a) = assets() else { return };
    let mut banked = session(a, RulesetPreset::DefinitiveOverhaul, true, false);
    let mut plain = session(load_assets(ROMS, MANIFEST).unwrap(), RulesetPreset::DefinitiveOverhaul, false, false);
    assert_eq!(local_fight(&mut banked), local_fight(&mut plain));
    assert!(banked.fight_machine().unwrap().background_bank().is_some());
    assert_eq!(banked.fight_background_pages(), Some(4), "the ROM's, the test stage's and two of VS screens");
    assert_eq!(plain.fight_background_pages(), Some(1));
    for f in 0..600u32 {
        let attack = with(|i| i.p1.buttons[(f / 20 % 6) as usize] = f % 20 < 3);
        assert_eq!(banked.step(attack).frame.argb, plain.step(attack).frame.argb, "fight frame {f}");
    }
}

/// `scene` with only the layers in `active` (0x20 bg, 0x40 fg).
fn only(scene: &SceneState, active: u8) -> SceneState {
    SceneState { active, ..scene.clone() }
}

/// The test stage's backdrop in `scene` against the ROM's own map: its
/// background is Geki's at the same drift in Japan's stage palette rows,
/// its foreground Joe's at the same camera in USA's foreground rows, either
/// way up. Returns the pixels compared.
fn check_backdrop(rom: &Assets, banked: &Assets, scene: &SceneState, what: &str) {
    assert_eq!((scene.bg_page, scene.fg_page), (1, 1), "{what}: both layers on page 1");
    let bg = only(scene, 0x20);
    let mut bg_rom = only(scene, 0x20);
    bg_rom.bg_page = 0;
    bg_rom.bg_scroll = scene.bg_scroll - 0x1800 + GEKI_BG;
    for (row, from) in [(0, 12), (8, 13)] {
        let src = JAPAN_BG_PALETTE + row * 32;
        assert_eq!(scene.paletteram[from * 32..from * 32 + 32], rom.maincpu[src..src + 32], "{what}: bg row {from}");
        bg_rom.paletteram[row * 32..row * 32 + 32].copy_from_slice(&rom.maincpu[src..src + 32]);
    }
    assert_eq!(render_frame(banked, &bg).argb, render_frame(rom, &bg_rom).argb, "{what}: the background");
    let fg = only(scene, 0x40);
    let mut fg_rom = only(scene, 0x40);
    fg_rom.fg_page = 0;
    fg_rom.fg_scroll = scene.fg_scroll - 0x1c80 + JOE_FG;
    assert_eq!(scene.paletteram[0x200..0x200 + 5 * 32], rom.maincpu[USA_FG_PALETTE..USA_FG_PALETTE + 5 * 32], "{what}: fg rows");
    assert_eq!(render_frame(banked, &fg).argb, render_frame(rom, &fg_rom).argb, "{what}: the foreground");
    // The page is what the frame shows: the ROM's page at the same scroll
    // would show Retsu's stage instead.
    let mut page0 = only(scene, 0x60);
    (page0.bg_page, page0.fg_page) = (0, 0);
    assert_ne!(render_frame(banked, &only(scene, 0x60)).argb, render_frame(banked, &page0).argb, "{what}");
}

#[test]
fn a_local_fight_on_the_test_stage_draws_scrolls_and_drifts_like_a_1987_stage() {
    let Some(a) = assets() else { return };
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let banked_assets = a.with_background_bank(&bank(&a));
    let mut s = session(a, RulesetPreset::DefinitiveOverhaul, true, true);
    assert!(s.set_playable_themes(&[TEST_THEME]).is_empty());
    let latches = local_fight(&mut s);
    assert!(latches.contains(&0x40), "the test stage's own theme, the test theme's command");
    assert!(!latches.contains(&0x29), "not Geki's, which its background is");
    let m = s.fight_machine().unwrap();
    assert_eq!(m.match_config().roster.stage, Some(StagePick::Bank(0)));
    assert_eq!(m.match_config().roster.mode, PlayMode::Versus);
    let start = (m.state.fg_scroll, m.state.bg_scroll);
    assert_eq!(start.0, 0x1c80 + 0xc0, "the stage's word and the camera's 0xc0");
    assert!((0x1800..=0x1a00).contains(&start.1));
    check_backdrop(&rom, &banked_assets, &m.state.scene(), "the first fight frame");

    // The camera: P1 walks back to the left edge and the foreground follows;
    // then forward. The drift runs throughout; idle until it jumps back.
    let (mut fg_seen, mut wrapped, mut f) = (std::collections::BTreeSet::new(), None, 0u32);
    let mut prev_bg = start.1;
    while wrapped.is_none() {
        let input = match f {
            0..=239 => with(|i| i.p1.directions[1] = true),
            240..=599 => with(|i| i.p1.directions[0] = true),
            _ => idle(),
        };
        s.step(input);
        f += 1;
        assert!(f < 4_000, "the drift's jump back");
        let m = s.fight_machine().unwrap();
        assert_eq!(flow_position(m), FIGHT_LOOP, "still fighting at frame {f}");
        let (fg, bg) = (m.state.fg_scroll, m.state.bg_scroll);
        fg_seen.insert(fg);
        assert!((0x1c80..=0x1c80 + 0x180).contains(&fg), "the camera's range: {fg:#06x}");
        assert!((0x1800..=0x1a00).contains(&bg), "the drift's range: {bg:#06x}");
        let drift_edge = bg >= 0x19ff || bg <= 0x1801;
        if bg < prev_bg {
            assert_eq!((prev_bg, bg), (0x1a00, 0x1800), "the jump back is 0x200");
            wrapped = Some(f);
        }
        if f % 60 == 0 || prev_bg != bg && drift_edge || fg_seen.len() < 3 {
            check_backdrop(&rom, &banked_assets, &m.state.scene(), &format!("frame {f}"));
        }
        prev_bg = bg;
    }
    assert!(fg_seen.len() > 20, "the camera moved the foreground: {} positions", fg_seen.len());
    // The jump back shows nothing: the background at drift 0x200 is the
    // background at drift 0.
    let scene = s.fight_machine().unwrap().state.scene();
    let at = |bg_scroll: u16| render_frame(&banked_assets, &SceneState { bg_scroll, ..only(&scene, 0x20) }).argb;
    assert_eq!(at(0x1800), at(0x1a00));
    println!("the camera showed {} foreground positions; the drift jumped back at frame {}", fg_seen.len(), wrapped.unwrap());
}

#[test]
fn a_flipped_local_fight_on_the_test_stage_flips_its_backdrop() {
    let Some(a) = assets() else { return };
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let b = bank(&a);
    let banked_assets = a.with_background_bank(&b);
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    config.roster.stage = Some(StagePick::Bank(b.stage_index("test-stage").unwrap()));
    let setup = FightSetup {
        kind: FightKind::Local,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1 & !0x0100, // Flip Screen on
        dsw2: FightDips::default().dsw2(),
    };
    let mut fight = Fight::start(&a.maincpu, setup, Some(0), Some(Arc::new(b))).expect("the hidden drive");
    let mut f = 0;
    while flow_position(fight.machine()) != FIGHT_LOOP {
        fight.step(&banked_assets, PlayerInputs::default(), PlayerInputs::default(), false);
        f += 1;
        assert!(f < 3_000, "the fight loop");
    }
    let walk = PlayerInputs { directions: [false, true, false, false], ..PlayerInputs::default() };
    for f in 0..300 {
        let scene = fight.machine().state.scene();
        assert!(scene.flip_screen, "flip screen at frame {f}");
        if f % 30 == 0 {
            check_backdrop(&rom, &banked_assets, &scene, &format!("flipped frame {f}"));
            // Flipped, it is the unflipped backdrop turned both ways.
            let up = render_frame(&banked_assets, &SceneState { flip_screen: false, ..only(&scene, 0x20) }).argb;
            let flipped = render_frame(&banked_assets, &only(&scene, 0x20)).argb;
            assert!(up.iter().rev().eq(flipped.iter()), "flipped frame {f}: the background turned");
        }
        fight.step(&banked_assets, walk, PlayerInputs::default(), false);
    }
}
