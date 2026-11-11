//! Ticket #162 (`FE12`): the front end's acceptance walk. One headless
//! game session from power-on: the attract, the Main Menu, Arcade (a lost
//! fight, a free continue, a second loss whose countdown runs out, back to
//! the menu), Local (a won match, a rematch, quit from the pause box),
//! Training against a CPU (every pause row, the live preset) and against a
//! dummy, Settings (a change and `RESET TAB` on every tab), the 1987
//! cabinet and back, and Exit, with no untranslated branch anywhere.
//! Local ROMs only; it skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::faithful::{DIPS, HOLD_FRAMES};
use sf_engine::front::fight::OverlayMode;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::roster::TournamentFighter;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
const NAME_ENTRY_LOOP: u32 = 0x42584;
const RIGHT: usize = 0;
const DOWN: usize = 2;
const UP: usize = 3;

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn stick(dir: usize) -> FrameInputs {
    with(|i| i.p1.directions[dir] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn start() -> FrameInputs {
    with(|i| i.menu.start = true)
}

fn back() -> FrameInputs {
    with(|i| i.menu.back = true)
}

fn lp(side: usize) -> FrameInputs {
    with(|i| if side == 0 { i.p1.buttons[0] = true } else { i.p2.buttons[0] = true })
}

/// `input` for one frame, then a frame of nothing held.
fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(idle());
    out
}

/// Step until the screen is `name`, feeding `input(frame)`.
fn until(s: &mut GameSession, name: &str, limit: u32, input: impl Fn(u32) -> FrameInputs) {
    for f in 0..limit {
        if s.step(input(f)).screen == name {
            return;
        }
    }
    panic!("never reached {name} in {limit} frames (at {})", s.screen());
}

fn to_fight_loop(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

/// The Main Menu from the attract (back out to the attract first if a
/// menu is up). It opens where it was last left.
fn main_menu(s: &mut GameSession) {
    if s.screen() == "main menu" {
        assert_eq!(tap(s, back()).screen, "attract");
    }
    assert_eq!(s.screen(), "attract");
    assert_eq!(tap(s, start()).screen, "main menu");
}

/// Move the menu's cursor down to `item` and choose it.
fn choose(s: &mut GameSession, item: MenuItem) -> FrameOutput {
    for _ in 0..6 {
        if s.menu_item() == Some(item) {
            let out = tap(s, confirm());
            assert_eq!(out.transition, Some(Transition::Choose(item)));
            return out;
        }
        tap(s, stick(DOWN));
    }
    panic!("{item:?} is not on the menu (at {:?})", s.menu_item());
}

/// Wait out a lost fight to the continue countdown, committing initials if
/// the ranking check asks for them.
fn lose_to_continue(s: &mut GameSession) {
    let mut f = 0u32;
    while s.screen() != "continue" {
        let entering = s.fight_machine().map(flow_position) == Some(NAME_ENTRY_LOOP);
        s.step(if entering && f.is_multiple_of(30) { lp(0) } else { idle() });
        f += 1;
        assert!(f < 12_000, "the continue countdown (at {})", s.screen());
    }
}

/// P1 walks in, then stands and cycles the six buttons; P2 stands.
fn p1_attacks(f: u32) -> FrameInputs {
    with(|i| {
        let phase = f % 120;
        i.p1.directions[RIGHT] = phase < 40;
        if phase >= 40 && phase % 10 < 3 {
            i.p1.buttons[((f / 10) % 6) as usize] = true;
        }
    })
}

/// Open the pause box and choose row `row` (0 is RESUME). The choice's
/// own frame is the last stepped: a box that closes holds the fight on it,
/// so a restarted round is still exactly at its snapshot.
fn pause_row(s: &mut GameSession, row: usize) -> FrameOutput {
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    s.step(idle());
    for _ in 0..row {
        tap(s, stick(DOWN));
    }
    s.step(confirm())
}

#[test]
fn the_front_end_from_power_on_to_exit() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut s = GameSession::new(assets, MatchConfig::default()).with_settings(Settings::default());

    // Power-on and the attract.
    for _ in 0..300 {
        assert_eq!(s.step(idle()).screen, "attract");
    }
    main_menu(&mut s);

    // Arcade: Ryu from Japan, a lost fight, a free continue on Start, a
    // second loss whose countdown runs out, the Main Menu.
    assert_eq!(choose(&mut s, MenuItem::SinglePlayer).screen, "single player");
    assert_eq!(choose(&mut s, MenuItem::Arcade).screen, "select");
    assert_eq!(select::confirm_fighter(&mut s, confirm(), tap).transition, Some(Transition::Fighter(TournamentFighter::Ryu)));
    until(&mut s, "country select", 60, |_| idle());
    tap(&mut s, lp(0));
    to_fight_loop(&mut s);
    lose_to_continue(&mut s);
    for _ in 0..60 {
        assert_eq!(s.step(idle()).screen, "continue");
    }
    s.step(start());
    until(&mut s, "fight", 120, |_| idle());
    lose_to_continue(&mut s);
    until(&mut s, "main menu", 3_000, |_| idle());
    assert!(s.fight_machine().is_none());
    println!("arcade done");

    // Local: Ryu against Ken, a won match, a rematch, quit to the menu.
    main_menu(&mut s);
    assert_eq!(s.menu_item(), Some(MenuItem::SinglePlayer), "the menu opens where it was left");
    assert_eq!(choose(&mut s, MenuItem::MultiPlayer).screen, "multi player");
    assert_eq!(choose(&mut s, MenuItem::Local).screen, "select");
    select::confirm_fighter(&mut s, lp(0), tap);
    select::confirm_fighter(&mut s, lp(1), tap);
    until(&mut s, "country select", 60, |_| idle());
    tap(&mut s, lp(0));
    to_fight_loop(&mut s);
    until(&mut s, "after match", 20_000, p1_attacks);
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Rematch)));
    to_fight_loop(&mut s);
    let out = pause_row(&mut s, 5);
    assert_eq!((out.screen, out.transition), ("main menu", Some(Transition::Choose(MenuItem::QuitToMenu))));
    println!("local done");

    // Training against a CPU: every pause row and the live preset.
    main_menu(&mut s);
    assert_eq!(s.menu_item(), Some(MenuItem::MultiPlayer), "left on MULTI PLAYER");
    choose(&mut s, MenuItem::SinglePlayer);
    assert_eq!(s.menu_item(), Some(MenuItem::Arcade), "Single Player left on ARCADE MODE");
    assert_eq!(choose(&mut s, MenuItem::Training).screen, "training");
    assert_eq!(choose(&mut s, MenuItem::TrainingCpu).screen, "select");
    select::confirm_fighter(&mut s, confirm(), tap); // Ryu
    assert!(matches!(select::confirm_fighter(&mut s, confirm(), tap).transition, Some(Transition::Fighters(..))));
    to_fight_loop(&mut s);
    assert_eq!(pause_row(&mut s, 0).transition, Some(Transition::Choose(MenuItem::Resume)));
    assert_eq!(s.screen(), "fight");
    assert_eq!(pause_row(&mut s, 1).transition, Some(Transition::Choose(MenuItem::Overlays)));
    assert_eq!(s.fight().unwrap().overlay(), OverlayMode::Hitboxes);
    tap(&mut s, start()); // the box closes
    for _ in 0..120 {
        s.step(with(|i| i.p1.directions[RIGHT] = true));
    }
    let round_start = s.fight().unwrap().round_start().expect("a round has started").clone();
    assert_eq!(pause_row(&mut s, 4).transition, Some(Transition::Choose(MenuItem::ResetPositions)));
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), round_start, "the round's start, bit for bit");
    // TRAINING opens the settings box, whose PRESET row cycles the live preset.
    assert_eq!(pause_row(&mut s, 6).transition, Some(Transition::Choose(MenuItem::TrainingSettings)));
    s.step(idle());
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::LivePreset)));
    assert_eq!(s.fight_machine().unwrap().match_config().preset, RulesetPreset::ClassicCompetitive);
    tap(&mut s, start());
    for _ in 0..120 {
        s.step(idle());
    }
    let out = pause_row(&mut s, 5);
    assert_eq!(out.screen, "main menu");
    println!("training cpu done");

    // Training against a dummy, quit from the box.
    main_menu(&mut s);
    choose(&mut s, MenuItem::SinglePlayer);
    choose(&mut s, MenuItem::Training);
    assert_eq!(choose(&mut s, MenuItem::TrainingDummy).screen, "select");
    assert_eq!(select::confirm_fighter(&mut s, confirm(), tap).transition, Some(Transition::Picked(TournamentFighter::Ryu)));
    select::confirm_fighter(&mut s, confirm(), GameSession::step); // the dummy's fighter
    until(&mut s, "country select", 60, |_| idle());
    tap(&mut s, lp(0));
    to_fight_loop(&mut s);
    for _ in 0..300 {
        s.step(p1_attacks(0));
    }
    assert_eq!(pause_row(&mut s, 5).screen, "main menu");
    println!("training dummy done");

    // Settings: on every tab, change the first row, then RESET TAB (up
    // from the first row wraps to it); leaving saves.
    main_menu(&mut s);
    assert_eq!(choose(&mut s, MenuItem::Settings).screen, "settings");
    for tab in 0..5 {
        assert_eq!(tap(&mut s, stick(RIGHT)).transition, Some(Transition::ValueChanged), "tab {tab}'s first row");
        tap(&mut s, stick(UP));
        assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::ValueChanged), "tab {tab}'s RESET TAB");
        tap(&mut s, stick(DOWN)); // back to the first row
        tap(&mut s, with(|i| i.menu.tab_right = true));
    }
    let out = tap(&mut s, back());
    assert_eq!(out.screen, "main menu");
    assert!(out.settings_changed);
    let positions = s.settings().menu_positions;
    assert_eq!(
        Settings { menu_positions: positions, ..s.settings().clone() },
        Settings { menu_positions: positions, ..Settings::default() },
        "every tab back at its defaults"
    );
    assert_eq!(s.match_config().preset, RulesetPreset::DefinitiveOverhaul);
    println!("settings done");

    // The 1987 cabinet: the DIP screen, POWER ON, a while in the machine,
    // the Esc hold back to the overhaul attract.
    assert_eq!(choose(&mut s, MenuItem::Cabinet1987).screen, "1987 cabinet");
    for _ in 0..DIPS.len() {
        tap(&mut s, stick(DOWN));
    }
    assert_eq!(s.step(confirm()).screen, "cabinet");
    for _ in 0..600 {
        s.step(idle());
    }
    let mut out = s.step(back());
    for _ in 1..HOLD_FRAMES {
        out = s.step(back());
    }
    assert_eq!(out.screen, "attract");
    println!("cabinet done");

    // Exit.
    s.step(idle());
    main_menu(&mut s);
    assert_eq!(s.menu_item(), Some(MenuItem::Cabinet1987), "left on 1987");
    assert!(choose(&mut s, MenuItem::Exit).quit);

    // No untranslated branch anywhere.
    assert!(s.take_log().iter().all(|l| !l.contains("untranslated")), "no untranslated-branch notice");
    assert!(s.attract_machine().unrecovered().is_empty());
}
