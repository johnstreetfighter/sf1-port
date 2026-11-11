//! Ticket #165 (`AP1`): the extended sprite bank, empty for now (ADR-0007).
//!
//! The 1987 game never draws a sprite tile code at or past `0x3800`, where
//! the bank starts, so with a bank loaded every ROM pose draws exactly as
//! before. Fights under an overhaul preset draw with the bank. The attract,
//! the faithful cabinet and Faithful 1987 fights never load it. The sweep
//! covers every committed scenario from power-on to its end (874,344
//! frames, about two seconds). Local ROMs only; each test skips without
//! them.

use std::path::{Path, PathBuf};

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::run::{observe_power_on_scenario, PowerOnMachine};
use sf_video::render::render_frame;
use sf_video::romset::{load_assets, Assets};
use sf_video::sprite_bank::{SpriteBank, ROM_SPRITE_TILES};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const SCENARIOS: &str = "../../tools/reference/scenarios";

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

/// A bank whose every tile is solid pen 1, from `0x3800` to `0x3bff`: if
/// any of it were ever drawn, the frame would show it.
fn poison_bank() -> SpriteBank {
    let mut bank = SpriteBank::new();
    for code in 0x3800..0x3c00u16 {
        bank.put(code, &[[1; 16]; 16]);
    }
    bank
}

/// Every sprite tile code the machine's object RAM would draw this frame
/// (a large entry draws four).
fn codes(m: &PowerOnMachine) -> impl Iterator<Item = u16> + '_ {
    let o = &m.state.objectram;
    (0..128usize).flat_map(move |e| {
        let code = u16::from_be_bytes([o[e * 0x40], o[e * 0x40 + 1]]);
        let large = u16::from_be_bytes([o[e * 0x40 + 2], o[e * 0x40 + 3]]) & 0x400 != 0;
        let n = if large { 4 } else { 1 };
        [0u16, 1, 16, 17].into_iter().take(n).map(move |d| code.wrapping_add(d))
    })
}

fn duration(path: &Path) -> u32 {
    let text = std::fs::read_to_string(path).unwrap();
    let after = text.split("\"duration_frames\":").nth(1).expect("duration_frames");
    let digits: String = after.trim_start().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap()
}

/// The highest tile code a scenario draws, from power-on to its end.
fn highest_code(rom: &[u8], path: &Path) -> (u16, u32) {
    let mut highest = 0u16;
    let (last, unrecovered) =
        observe_power_on_scenario(rom.to_vec(), path, duration(path) - 1, |m| {
            highest = codes(m).fold(highest, u16::max);
        })
        .unwrap();
    assert!(unrecovered.is_empty(), "{}: untranslated {unrecovered:05x?}", path.display());
    (highest, last)
}

/// The complete one- and two-player games, the service mode, every boss on
/// the ladder, the bonus stages and the ending are all among them.
#[test]
fn no_committed_scenario_draws_a_bank_tile_code() {
    let Some(a) = assets() else { return };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(SCENARIOS).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    let (mut overall, mut frames) = (0u16, 0u64);
    for p in &paths {
        let (highest, last) = highest_code(&a.maincpu, p);
        overall = overall.max(highest);
        frames += u64::from(last) + 1;
        assert!((highest as usize) < ROM_SPRITE_TILES, "{} draws {highest:#06x}", p.display());
    }
    println!("{} scenarios, {frames} frames, highest tile code {overall:#06x}", paths.len());
    assert!(paths.len() >= 137);
}

#[test]
fn every_rom_pose_draws_exactly_as_before_with_a_bank_loaded() {
    let Some(a) = assets() else { return };
    let banked = a.with_sprite_bank(&poison_bank());
    let path = Path::new(SCENARIOS).join("sf_full_1p_01.json");
    let mut compared = 0;
    let mut frame = 0u32;
    observe_power_on_scenario(a.maincpu.clone(), &path, duration(&path) - 1, |m| {
        frame += 1;
        if frame.is_multiple_of(97) {
            let scene = m.state.scene();
            assert_eq!(render_frame(&banked, &scene).argb, render_frame(&a, &scene).argb, "frame {frame}");
            compared += 1;
        }
    })
    .unwrap();
    println!("{compared} frames of the complete one-player game compared");
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
/// fighter under their cursor; Japan; the first fight loop.
fn local_fight(s: &mut GameSession) {
    for _ in 0..200 {
        s.step(idle());
    }
    let mut tap = |i: FrameInputs| {
        s.step(i);
        s.step(idle());
    };
    tap(with(|i| i.menu.start = true));
    tap(with(|i| i.p1.directions[2] = true));
    tap(with(|i| i.menu.confirm = true));
    tap(with(|i| i.menu.confirm = true));
    tap(with(|i| i.p1.buttons[0] = true));
    tap(with(|i| i.p2.buttons[0] = true));
    while s.screen() != "country select" {
        s.step(idle());
    }
    s.step(with(|i| i.p1.buttons[0] = true));
    s.step(idle());
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(0x3eafa) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

#[test]
fn overhaul_fights_draw_with_the_bank_and_faithful_play_never_loads_it() {
    let Some(a) = assets() else { return };
    let bank = poison_bank();
    let rom_only = load_assets(ROMS, MANIFEST).unwrap();
    // An overhaul fight draws with the bank; every frame of the walk equals
    // the same walk without one, because no ROM pose reaches it.
    let mut banked = GameSession::new(a, MatchConfig::default()).with_sprite_bank(&bank);
    let mut plain = GameSession::new(rom_only, MatchConfig::default());
    for f in 0..200 {
        assert_eq!(banked.step(idle()).frame.argb, plain.step(idle()).frame.argb, "attract frame {f}");
    }
    local_fight(&mut banked);
    local_fight(&mut plain);
    assert_eq!(banked.fight_sprite_tiles(), Some(ROM_SPRITE_TILES + bank.tiles()));
    assert_eq!(plain.fight_sprite_tiles(), Some(ROM_SPRITE_TILES));
    for f in 0..600u32 {
        let attack = with(|i| i.p1.buttons[(f / 20 % 6) as usize] = f % 20 < 3);
        assert_eq!(banked.step(attack).frame.argb, plain.step(attack).frame.argb, "fight frame {f}");
    }
    // A Faithful 1987 fight never loads it.
    let mut faithful = GameSession::new(load_assets(ROMS, MANIFEST).unwrap(), MatchConfig::from_preset(RulesetPreset::Faithful1987))
        .with_sprite_bank(&bank);
    local_fight(&mut faithful);
    assert_eq!(faithful.fight_sprite_tiles(), Some(ROM_SPRITE_TILES), "Faithful 1987");
    assert_eq!(faithful.fight_machine().unwrap().match_config().preset, RulesetPreset::Faithful1987);
}
