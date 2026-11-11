//! Ticket #160 (`FE10`): the faithful cabinet.
//!
//! The `1987 CABINET` DIP screen lists the verified settings at factory
//! defaults; `POWER ON` boots the original machine with those DIP words
//! (checked through boot's DIP decode), coins, starts and the service
//! credit live, test mode booting into the service screen, nothing from
//! the overhaul settings reaching it; a full attract loop inside it equals
//! the power-on machine frame for frame, the Esc hold bar being a
//! presentation overlay; a one-second hold returns to the overhaul attract
//! and a shorter one does nothing; the DIP words persist. Local ROMs only;
//! each test skips without them.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::faithful::{Cabinet, DipScreen, BAR, DIPS, HOLD_FRAMES, HOLD_HINT, POWER_ON_ROW};
use sf_engine::front::settings::Settings;
use sf_engine::front::{FightDips, FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::roster::MenuInput;
use sf_engine::run::PowerOnMachine;
use sf_engine::tasks::Io;
use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::decode::decode_char_tile;
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

fn rb(m: &PowerOnMachine, addr: u32) -> u8 {
    m.work_ram()[(addr - 0xff8000) as usize]
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
fn the_dip_screen_lists_the_verified_settings_at_factory_defaults() {
    let names: Vec<_> = DIPS.iter().map(|d| d.name).collect();
    assert_eq!(
        names,
        [
            "COINAGE A",
            "COINAGE B",
            "CONTINUATION",
            "BUY-IN",
            "DIFFICULTY",
            "ROUND TIME",
            "FLIP SCREEN",
            "DEMO SOUNDS",
            "ATTRACT MUSIC",
            "COUNTRY COUNT",
            "TEST MODE"
        ]
    );
    // Factory words: every row on its first option, the factory one.
    let screen = DipScreen::open(0xdfff, 0xffff);
    for i in 0..DIPS.len() {
        assert_eq!(screen.option(i), 0, "{} at its factory setting", DIPS[i].name);
    }
    // Every option of every row lands in its own bits and nowhere else.
    for (i, dip) in DIPS.iter().enumerate() {
        let mut s = DipScreen::open(0xdfff, 0xffff);
        for _ in 0..i {
            s.apply(MenuInput::Down);
        }
        for (k, &(bits, _)) in dip.options.iter().enumerate().skip(1) {
            assert_eq!(s.apply(MenuInput::Right), Some(Transition::ValueChanged));
            assert_eq!(s.option(i), k);
            let (d1, d2) = s.dsw();
            let (word, other, factory_other) = if dip.port == 1 { (d1, d2, 0xffff) } else { (d2, d1, 0xdfff) };
            let factory = if dip.port == 1 { 0xdfff } else { 0xffff };
            assert_eq!(word & dip.mask, bits, "{} option {k}", dip.name);
            assert_eq!(word & !dip.mask, factory & !dip.mask, "{}: no other bit moves", dip.name);
            assert_eq!(other, factory_other);
        }
    }
    let Some(a) = assets() else { return };
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let frame = DipScreen::open(0xdfff, 0xffff).render(&font);
    assert!(text_at(&frame, &a, (48 - HOLD_HINT.len()) / 2, 25, HOLD_HINT, ink::DIM), "the hold hint");
    assert!(text_at(&frame, &a, 20, POWER_ON_ROW, "POWER ON", ink::BLUE));
}

#[test]
fn power_on_boots_the_machine_with_the_rows_dip_words() {
    let Some(a) = assets() else { return };
    // Difficulty Easy, round time 200, four countries, continuation none,
    // flip on: boot's decode (0x3c132..0x3c1bc) reads them.
    let mut s = DipScreen::open(0xdfff, 0xffff);
    let set = |s: &mut DipScreen, row: usize, presses: usize| {
        let mut t = DipScreen::open(s.dsw().0, s.dsw().1);
        for _ in 0..row {
            t.apply(MenuInput::Down);
        }
        for _ in 0..presses {
            t.apply(MenuInput::Right);
        }
        *s = t;
    };
    set(&mut s, 4, 3); // DIFFICULTY: EASY
    set(&mut s, 5, 2); // ROUND TIME: 200
    set(&mut s, 9, 1); // COUNTRY COUNT: 4
    set(&mut s, 2, 5); // CONTINUATION: NONE
    set(&mut s, 6, 1); // FLIP SCREEN: ON
    let (d1, d2) = s.dsw();
    let mut cab = Cabinet::power_on(&a.maincpu, d1, d2);
    for _ in 0..12 {
        assert!(cab.step(&idle()).is_none());
    }
    let reference = {
        let mut m = PowerOnMachine::with_io(a.maincpu.clone(), Io { dsw1: d1, dsw2: d2, ..Io::idle() });
        for _ in 0..12 {
            m.step();
        }
        m
    };
    let m = cab.machine();
    assert_eq!(m.work_ram(), reference.work_ram(), "the same machine as powering on with these words");
    assert_eq!(rb(m, 0xffc007), 1, "difficulty Easy (0x40) decodes to 1");
    assert_eq!(rb(m, 0xff8e1c), 2, "round time 200 decodes to 2");
    assert_eq!(rb(m, 0xffc005), 0, "four countries decode to 0");
    assert_eq!(rb(m, 0xffc000), 0, "no continuation");
    assert_ne!(rb(m, 0xff8682) & 4, 0, "flip on");
    assert_eq!(*m.match_config(), MatchConfig::faithful());
}

#[test]
fn coins_starts_the_service_credit_and_test_mode_work_in_the_cabinet() {
    let Some(a) = assets() else { return };
    let mut cab = Cabinet::power_on(&a.maincpu, 0xdfff, 0xffff);
    let credits = |c: &Cabinet| rb(c.machine(), 0xff865d);
    for _ in 0..900 {
        cab.step(&idle());
    }
    for f in 0..40 {
        cab.step(&with(|i| i.cabinet[0] = f < 12));
    }
    assert_eq!(credits(&cab), 1, "coin1 credits");
    for f in 0..40 {
        cab.step(&with(|i| i.cabinet[4] = f < 12));
    }
    assert_eq!(credits(&cab), 2, "service1 credits");
    for f in 0..60 {
        cab.step(&with(|i| i.cabinet[2] = f < 12));
    }
    assert_eq!(rb(cab.machine(), 0xffc002), 2, "start1 starts a one-player game");
    assert!(cab.machine().unrecovered().is_empty());
    // Test mode on: the service screen.
    let mut cab = Cabinet::power_on(&a.maincpu, 0x5fff, 0xffff);
    for _ in 0..30 {
        assert!(cab.step(&idle()).is_none(), "the service screen runs translated");
    }
    let text = cab.machine().state.videoram.chunks(2).filter(|w| *w != [0x10, 0x3c]).count();
    assert!(text > 100, "the service screen's text is up ({text} cells)");
}

#[test]
fn an_attract_loop_in_the_cabinet_is_the_power_on_machine_and_the_hold_bar_is_only_drawn() {
    let Some(a) = assets() else { return };
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let mut cab = Cabinet::power_on(&a.maincpu, 0xdfff, 0xffff);
    let mut reference = PowerOnMachine::new(a.maincpu.clone());
    for f in 1..=6000u32 {
        // Esc held for 40 frames at a time, never long enough to leave.
        let held = f % 100 < 40;
        assert!(cab.step(&with(|i| i.menu.back = held)).is_none());
        reference.step();
        assert_eq!(cab.machine().work_ram(), reference.work_ram(), "work RAM at {f}");
        let shown = cab.render(&a, &font);
        let raw = render_frame(&a, &reference.state.scene());
        let (x0, y0, w, h) = BAR;
        for (i, (p, r)) in shown.argb.iter().zip(&raw.argb).enumerate() {
            let (x, y) = (i % WIDTH, i / WIDTH);
            if !held || !(x0..x0 + w).contains(&x) || !(y0..y0 + h).contains(&y) {
                assert_eq!(p, r, "pixel ({x}, {y}) at {f}");
            }
        }
        if held && f % 100 == 39 {
            assert_ne!(shown.argb, raw.argb, "the bar is drawn at {f}");
        }
    }
    assert!(cab.machine().unrecovered().is_empty());
}

#[test]
fn the_walk_into_the_cabinet_a_fight_and_the_esc_hold_back() {
    let Some(a) = assets() else { return };
    // Overhaul settings that must not reach the cabinet.
    let mut config = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive);
    config.difficulty = 4;
    let mut s = GameSession::new(a, config).with_settings(Settings::default());
    s.set_fight_dips(FightDips { difficulty: 4, round_time: 250 });
    for _ in 0..200 {
        s.step(idle());
    }
    let attract = s.attract_machine().save_snapshot();
    tap(&mut s, with(|i| i.menu.start = true));
    for _ in 0..3 {
        tap(&mut s, with(|i| i.p1.directions[2] = true)); // to 1987
    }
    let out = tap(&mut s, with(|i| i.menu.confirm = true));
    assert_eq!((out.screen, out.transition), ("1987 cabinet", Some(Transition::Choose(MenuItem::Cabinet1987))));
    // TEST MODE stays off; DEMO SOUNDS off to check the words persist.
    for _ in 0..7 {
        tap(&mut s, with(|i| i.p1.directions[2] = true));
    }
    tap(&mut s, with(|i| i.p1.directions[0] = true));
    for _ in 0..4 {
        tap(&mut s, with(|i| i.p1.directions[2] = true)); // to POWER ON
    }
    let out = s.step(with(|i| i.menu.confirm = true));
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::PowerOn)));
    assert!(out.settings_changed, "the DIP words are saved");
    assert_eq!(out.screen, "cabinet");
    assert_eq!(s.settings().cabinet_dsw1 & 0x2000, 0x2000, "demo sounds off persisted");
    let m = s.cabinet_machine().unwrap();
    assert_eq!(*m.match_config(), MatchConfig::faithful(), "no overhaul toggle");
    // Coin and start with the cabinet keys, then Japan, then the fight.
    for _ in 0..900 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.cabinet[0] = true));
    for _ in 0..30 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.cabinet[2] = true));
    let mut f = 0;
    while flow_position(s.cabinet_machine().unwrap()) != 0x3eafa {
        let lp = f % 60 == 30;
        s.step(with(|i| i.p1.buttons[0] = lp));
        f += 1;
        assert!(f < 3_000, "the first fight");
    }
    let m = s.cabinet_machine().unwrap();
    assert_eq!(rb(m, 0xffc007), 0, "the cabinet's own Normal difficulty, not the overhaul's 4");
    // A short Esc hold does nothing (the cabinet has no pause box, so no
    // LOOK row either: its look is set in Settings); a one-second hold returns.
    let look = s.video().look;
    for _ in 0..HOLD_FRAMES - 1 {
        let out = s.step(with(|i| i.menu.back = true));
        assert_eq!((out.screen, out.video.look.as_str()), ("cabinet", look.as_str()));
    }
    s.step(idle());
    assert_eq!(s.screen(), "cabinet");
    let mut out = s.step(with(|i| i.menu.back = true));
    for _ in 1..HOLD_FRAMES {
        out = s.step(with(|i| i.menu.back = true));
    }
    assert_eq!(out.screen, "attract");
    assert!(s.cabinet_machine().is_none());
    assert_eq!(s.attract_machine().save_snapshot(), attract, "the overhaul attract as it was");
}
