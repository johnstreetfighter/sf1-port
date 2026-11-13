//! Ticket #218 (`EH2`): the overhaul fight layout.
//!
//! At the front end's fight-screen seam, driven by inputs: in Local and
//! Arcade fights under every overhaul preset the presented frame has TIME
//! centred under the health bars (the ROM's own TIME tiles drawn there from
//! its timer, its own lettering and colours) with the ROM's TIME at the
//! bottom-left gone and the playfield showing in its place; the P1 / HI /
//! P2 scores are gone in Local and kept in Arcade; each health bar's
//! PLAYER1, PLAYER2 or ENEMY is the name of the fighter on it, and so is
//! the round result's RYU, KEN or ENEMY.
//! Under the Faithful 1987 preset Local and Arcade present as the hardware
//! renderer draws them, and so does the faithful cabinet. Training, which
//! has no TIME, scores or round-win markers under any preset (ticket #240),
//! is checked in `training_round_furniture.rs`, its labels named there.
//! The machine's work RAM and text layer are only read. Local ROMs only;
//! each test skips without them. The Lee/Mike fixtures also need their
//! approved local art pack, as the player select does (SP13).

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::run::PowerOnMachine;
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::render::{render_frame, Frame, WIDTH};
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../assets/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
const TIME_DIGITS: u32 = 0xff8e18;
const DOWN: usize = 2;
const UP: usize = 3;
const LEFT: usize = 1;

/// The ROM's TIME (text row 28, columns 9 to 16), the score band (row 3,
/// columns 10 to 53) and the layout's TIME, as screen rectangles
/// (x, y, w, h).
const OLD_TIME: (usize, usize, usize, usize) = (8, 208, 64, 8);
const SCORES: (usize, usize, usize, usize) = (16, 8, 352, 8);
const NEW_TIME: (usize, usize, usize, usize) = (164, 52, 64, 8);
/// The bars' labels (text rows 5 and 7, columns 22 to 28).
const LABELS: [(usize, usize, usize, usize); 2] = [(112, 24, 56, 8), (112, 40, 56, 8)];

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

fn tap(s: &mut GameSession, input: FrameInputs) {
    s.step(input);
    s.step(idle());
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
    Arcade,
}

/// A session of `mode` under `preset`, stopped at the first fight loop.
fn fight(mode: Mode, preset: RulesetPreset) -> (GameSession, Assets) {
    let a = assets().expect("local ROMs");
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    match mode {
        Mode::Local => {
            tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
            tap(&mut s, with(|i| i.menu.confirm = true));
            tap(&mut s, with(|i| i.menu.confirm = true));
            select::confirm_fighter(&mut s, with(|i| i.p1.buttons[0] = true), tap);
            select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), tap);
            while s.screen() != "country select" {
                s.step(idle());
            }
            tap(&mut s, with(|i| i.p1.buttons[0] = true));
        }
        Mode::Arcade => {
            tap(&mut s, with(|i| i.menu.confirm = true)); // Single Player
            tap(&mut s, with(|i| i.menu.confirm = true)); // Arcade
            select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), tap); // Ryu
            while s.screen() != "country select" {
                s.step(idle());
            }
            // The four-flag screen's first flag (up-left), picked with LP.
            s.step(idle());
            tap(&mut s, with(|i| i.p1.directions[UP] = true));
            tap(&mut s, with(|i| i.p1.directions[LEFT] = true));
            tap(&mut s, with(|i| i.p1.buttons[0] = true));
        }
    }
    to_fight_loop(&mut s);
    let a = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    (s, a)
}

fn scene(s: &GameSession) -> SceneState {
    s.fight_machine().unwrap().state.scene()
}

fn plain(s: &GameSession, a: &Assets) -> Frame {
    render_frame(a, &scene(s))
}

/// The scene with the ROM's TIME cells (and the score cells when asked)
/// blanked as the ROM's own blank neighbour: the playfield showing behind;
/// and each bar's label (row 5, then row 7, from column 22) written over
/// with `names`, in the label's colours.
fn blanked(s: &GameSession, scores: bool, names: [&str; 2]) -> SceneState {
    let mut sc = scene(s);
    let word = |sc: &SceneState, col: usize, row: usize| {
        let i = row * 64 + col;
        [sc.videoram[2 * i], sc.videoram[2 * i + 1]]
    };
    let set = |sc: &mut SceneState, col: usize, row: usize, w: [u8; 2]| {
        let i = row * 64 + col;
        sc.videoram[2 * i..2 * i + 2].copy_from_slice(&w);
    };
    let blank = word(&sc, 8, 28);
    for col in 9..=16 {
        set(&mut sc, col, 28, blank);
    }
    if scores {
        let blank = word(&sc, 9, 3);
        for col in 10..=53 {
            set(&mut sc, col, 3, blank);
        }
    }
    for (row, name) in [5, 7].into_iter().zip(names) {
        let blank = word(&sc, 21, row);
        let colour = u16::from_be_bytes(word(&sc, 22, row)) & !0x3ff;
        for col in 22..=28 {
            let code = match name.as_bytes().get(col - 22) {
                Some(&b) => colour | (10 + u16::from(b - b'A')),
                None => u16::from_be_bytes(blank),
            };
            set(&mut sc, col, row, code.to_be_bytes());
        }
    }
    sc
}

fn in_rect(r: (usize, usize, usize, usize), i: usize) -> bool {
    let (x, y) = (i % WIDTH, i / WIDTH);
    (r.0..r.0 + r.2).contains(&x) && (r.1..r.1 + r.3).contains(&y)
}

/// The presented `frame` against the layout's expectation: the hidden
/// cells show the playfield, the ROM's TIME pixels are moved to the layout
/// position unchanged, and nothing else differs but the call-outs the fight
/// earned (a CPU's FIRST ATTACK, EH6), each inside its own box.
fn check_layout(s: &GameSession, a: &Assets, frame: &Frame, scores_hidden: bool, names: [&str; 2], at: &str) {
    let plain = plain(s, a);
    let callouts = s.fight().unwrap().callouts().frame().elements();
    let in_callout = |i: usize| callouts.iter().any(|e| e.rect.contains(i % WIDTH, i / WIDTH));
    let expected = render_frame(a, &blanked(s, scores_hidden, names));
    let mut moved = 0;
    for (i, (got, want)) in frame.argb.iter().zip(&expected.argb).enumerate() {
        if in_rect(NEW_TIME, i) {
            let (dx, dy) = (i % WIDTH - NEW_TIME.0, i / WIDTH - NEW_TIME.1);
            let old = (OLD_TIME.1 + dy) * WIDTH + OLD_TIME.0 + dx;
            if plain.argb[old] != expected.argb[old] {
                // A pixel of the ROM's TIME text: moved, same colour.
                assert_eq!(*got, plain.argb[old], "TIME pixel ({dx}, {dy}) {at}");
                moved += 1;
                continue;
            }
        }
        if !in_callout(i) {
            assert_eq!(got, want, "pixel ({}, {}) {at}", i % WIDTH, i / WIDTH);
        }
    }
    assert!(moved > 20, "TIME is drawn under the health bars {at} ({moved} pixels)");
    // The ROM's TIME is gone from the bottom-left, its scores follow the
    // mode, and the labels are names.
    for (i, (got, was)) in frame.argb.iter().zip(&plain.argb).enumerate() {
        if in_rect(OLD_TIME, i) || (scores_hidden && in_rect(SCORES, i)) || LABELS.iter().any(|&r| in_rect(r, i)) {
            continue;
        }
        if !in_rect(NEW_TIME, i) && !in_callout(i) {
            assert_eq!(got, was, "pixel ({}, {}) {at}", i % WIDTH, i / WIDTH);
        }
    }
    let (changed, _) = (0..plain.argb.len()).filter(|&i| in_rect(OLD_TIME, i)).fold((0, 0), |(n, k), i| {
        (n + (frame.argb[i] != plain.argb[i]) as usize, k + 1)
    });
    assert!(changed > 20, "the ROM's TIME is hidden at the bottom-left {at}");
    if !scores_hidden {
        for i in (0..plain.argb.len()).filter(|&i| in_rect(SCORES, i)) {
            assert_eq!(frame.argb[i], plain.argb[i], "the score band {at}");
        }
    } else {
        let hidden = (0..plain.argb.len())
            .filter(|&i| in_rect(SCORES, i) && frame.argb[i] != plain.argb[i])
            .count();
        assert!(hidden > 50, "the scores are hidden {at} ({hidden} pixels)");
    }
}

#[test]
fn every_overhaul_preset_puts_time_under_the_health_bars_and_drops_the_scores_in_local() {
    if assets().is_none() {
        return;
    }
    let presets = [
        RulesetPreset::DefinitiveOverhaul,
        RulesetPreset::ClassicCompetitive,
        RulesetPreset::PureFootsies1987Plus,
        RulesetPreset::Custom,
    ];
    for preset in presets {
        let (mut s, a) = fight(Mode::Local, preset);
        for f in 0..240 {
            let out = s.step(idle());
            assert_eq!(out.screen, "fight");
            check_layout(&s, &a, &out.frame, true, ["RYU", "KEN"], &format!("Local {preset:?} frame {f}"));
            // The ROM's timer is what is drawn.
            let m = s.fight_machine().unwrap();
            let ram = &m.work_ram()[(TIME_DIGITS - 0xff8000) as usize..][..3];
            let sc = m.state.scene();
            for (k, digit) in ram.iter().enumerate() {
                let i = 28 * 64 + 14 + k;
                assert_eq!(sc.videoram[2 * i + 1], *digit, "the ROM's TIME digit {k}");
            }
        }
    }
}

#[test]
fn arcade_keeps_its_score_with_time_moved() {
    if assets().is_none() {
        return;
    }
    for preset in [RulesetPreset::DefinitiveOverhaul, RulesetPreset::PureFootsies1987Plus] {
        let (mut s, a) = fight(Mode::Arcade, preset);
        for f in 0..240 {
            let out = s.step(idle());
            check_layout(&s, &a, &out.frame, false, ["RYU", "RETSU"], &format!("Arcade {preset:?} frame {f}"));
        }
    }
}

/// The round's column wipe (tile `0x4a`, right to left) covers the hidden
/// score band, the named labels and the ROM's TIME as it crosses them, as on the 1987 screen,
/// and the TIME under the health bars drops each tile the wipe has reached
/// instead of copying the wipe's black there.
#[test]
fn the_round_wipe_covers_the_hidden_cells() {
    if assets().is_none() {
        return;
    }
    let (mut s, a) = fight(Mode::Local, RulesetPreset::DefinitiveOverhaul);
    let wiped = |sc: &SceneState, col: usize, row: usize| {
        let i = row * 64 + col;
        u16::from_be_bytes([sc.videoram[2 * i], sc.videoram[2 * i + 1]]) & 0x3ff == 0x4a
    };
    let (mut checked, mut moved_dropped) = (0, 0);
    for f in 0..8_000 {
        let out = s.step(idle());
        let Some(m) = s.fight_machine() else { break };
        if out.screen != "fight" {
            break;
        }
        let sc = m.state.scene();
        let plain = render_frame(&a, &sc);
        let callouts = s.fight().unwrap().callouts().frame().elements();
        // The tile at `(x, y)` shows exactly what the hardware draws.
        let mut same = |x: usize, y: usize, at: &str| {
            for i in (0..8).flat_map(|dy| (0..8).map(move |dx| (y + dy) * WIDTH + x + dx)) {
                if !callouts.iter().any(|e| e.rect.contains(i % WIDTH, i / WIDTH)) {
                    assert_eq!(out.frame.argb[i], plain.argb[i], "{at} pixel ({}, {}) frame {f}", i % WIDTH, i / WIDTH);
                }
            }
            checked += 1;
        };
        for col in (10..=53).filter(|&c| wiped(&sc, c, 3)) {
            same(col * 8 - 64, 3 * 8 - 16, "wiped score cell");
        }
        for row in [5, 7] {
            for col in (22..=28).filter(|&c| wiped(&sc, c, row)) {
                same(col * 8 - 64, row * 8 - 16, "wiped label cell");
            }
        }
        let time = (9..=16).any(|c| !wiped(&sc, c, 28));
        for (k, col) in (9..=16).enumerate().filter(|&(_, c)| wiped(&sc, c, 28)) {
            same(col * 8 - 64, 28 * 8 - 16, "wiped TIME cell");
            if time {
                same(NEW_TIME.0 + 8 * k, NEW_TIME.1, "moved TIME over a wiped cell");
                moved_dropped += 1;
            }
        }
    }
    assert!(checked > 1_000, "the wipe crossed the hidden cells ({checked} tiles checked)");
    assert!(moved_dropped > 10, "the moved TIME dropped wiped tiles ({moved_dropped})");
}

#[test]
fn the_faithful_1987_preset_presents_local_and_arcade_as_the_hardware_renderer_draws_them() {
    if assets().is_none() {
        return;
    }
    for mode in [Mode::Local, Mode::Arcade] {
        let (mut s, a) = fight(mode, RulesetPreset::Faithful1987);
        for f in 0..240 {
            let out = s.step(idle());
            assert_eq!(out.frame.argb, plain(&s, &a).argb, "{mode:?} frame {f}");
        }
    }
}

#[test]
fn the_presenter_only_reads_the_machine() {
    if assets().is_none() {
        return;
    }
    let (mut s, a) = fight(Mode::Local, RulesetPreset::DefinitiveOverhaul);
    let mut with_extra_renders = s.clone();
    for f in 0..200 {
        s.step(idle());
        // The same inputs, with the layout presented over and over.
        with_extra_renders.step(idle());
        let fight = with_extra_renders.fight().unwrap();
        let before = fight.machine().save_snapshot();
        let _ = fight.render(&a);
        let _ = fight.render(&a);
        assert!(fight.machine().save_snapshot() == before, "rendering changed the machine at {f}");
    }
    let (m, n) = (s.fight_machine().unwrap(), with_extra_renders.fight_machine().unwrap());
    assert_eq!(m.work_ram(), n.work_ram(), "work RAM");
    assert_eq!(m.state.videoram, n.state.videoram, "the text layer");
    assert_eq!(m.save_snapshot(), n.save_snapshot());
}

#[test]
fn the_faithful_cabinet_is_frame_identical() {
    let Some(a) = assets() else { return };
    let mut s = GameSession::new(a, MatchConfig::default()).with_settings(Settings::default());
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    for _ in 0..4 {
        tap(&mut s, with(|i| i.p1.directions[DOWN] = true)); // to 1987
    }
    tap(&mut s, with(|i| i.menu.confirm = true));
    for _ in 0..11 {
        tap(&mut s, with(|i| i.p1.directions[DOWN] = true)); // to POWER ON
    }
    s.step(with(|i| i.menu.confirm = true));
    assert_eq!(s.screen(), "cabinet");
    for _ in 0..900 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.cabinet[0] = true));
    for _ in 0..30 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.cabinet[2] = true));
    let a = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut f = 0;
    let mut at_fight = 0;
    while at_fight < 120 {
        let lp = f % 60 == 30;
        let out = s.step(with(|i| i.p1.buttons[0] = lp));
        f += 1;
        assert!(f < 4_000, "the first fight");
        let m: &PowerOnMachine = s.cabinet_machine().unwrap();
        if flow_position(m) == FIGHT_LOOP {
            at_fight += 1;
            assert_eq!(out.frame.argb, render_frame(&a, &m.state.scene()).argb, "cabinet frame {f}");
        }
    }
}

/// A Local fight under Definitive Overhaul between P1 Lee and P2 Mike,
/// stopped at the first fight loop.
fn lee_against_mike() -> (GameSession, Assets) {
    let a = assets().expect("local ROMs");
    let pack = ArtPack::open(Path::new(PACK)).expect("the local art pack opens");
    let art = load_art_from(&a, &ArtSources {
        recipes: &[
            include_str!("../../../art/recipes/lee.json"),
            include_str!("../../../art/recipes/mike.json"),
        ],
        reviews: Some(&[
            include_str!("../../../art/review/lee.json"),
            include_str!("../../../art/review/mike.json"),
        ]),
        pack: Ok(&pack),
    }).expect("Lee's and Mike's approved poses load");
    let mut s = GameSession::new(a, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul))
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut s, with(|i| i.menu.confirm = true));
    tap(&mut s, with(|i| i.menu.confirm = true));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true)); // Lee
    select::confirm_fighter(&mut s, with(|i| i.p1.buttons[0] = true), tap);
    // Only Lee and Mike's boss tables are loaded: Mike is the next
    // selectable tile to Ken's right, skipping the unavailable bosses.
    tap(&mut s, with(|i| i.p2.directions[0] = true));
    select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), tap);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    let roster = s.fight().unwrap().setup().config.roster;
    assert_eq!(roster.p1, TournamentFighter::Lee, "the result fixture selects Lee");
    assert_eq!(roster.p2, TournamentFighter::Mike, "the result fixture selects Mike");
    let a = load_assets(ROMS, MANIFEST).expect("local ROM set assembles").with_sprite_bank(&art.bank);
    (s, a)
}

fn result_art_available() -> bool {
    if !Path::new(PACK).is_dir() {
        eprintln!("skipping: the Lee/Mike fixture needs the local art pack");
        return false;
    }
    true
}

/// The round result (text row 10) names the fighter who won it: where the
/// ROM types `RYU WON THE 1ST ROUND` for P1 (`KEN WON` for P2, `ENEMY WON`
/// for the CPU), the overhaul layout shows the winner's own name with the
/// ROM's rest of the line, centred where the ROM centres its own (column
/// 31), in the ROM's lettering and colours.
#[test]
fn the_round_result_names_the_fighter_who_won() {
    if assets().is_none() || !result_art_available() {
        return;
    }
    // Who wins (P1, P2, or the CPU against an idle P1 in Arcade), the ROM's
    // name, its line's first column and cells, and the name shown.
    let cases = [(Some(true), "RYU", 21, 21, "LEE"), (Some(false), "KEN", 21, 21, "MIKE"), (None, "ENEMY", 20, 23, "RETSU")];
    for (p1_wins, rom_name, first_cell, line_cells, name) in cases {
        let (mut s, a) = match p1_wins {
            Some(_) => lee_against_mike(),
            None => fight(Mode::Arcade, RulesetPreset::DefinitiveOverhaul),
        };
        const ROW: usize = 10;
        let cell = |sc: &SceneState, col: usize| {
            let i = ROW * 64 + col;
            u16::from_be_bytes([sc.videoram[2 * i], sc.videoram[2 * i + 1]])
        };
        let mut seen = false;
        for f in 0..9_000u32 {
            // The winner walks in and throws a heavy kick each second; the
            // other stands still.
            let (forward, phase) = (f % 60 < 30, f % 60 == 45);
            let out = s.step(with(|i| match p1_wins {
                Some(true) => {
                    i.p1.directions[0] = forward;
                    i.p1.buttons[5] = phase;
                }
                Some(false) => {
                    i.p2.directions[LEFT] = forward;
                    i.p2.buttons[5] = phase;
                }
                None => {}
            }));
            let Some(m) = s.fight_machine() else { break };
            let sc = m.state.scene();
            // Wait for the ROM's whole line: its last cell typed.
            if cell(&sc, first_cell) & 0x3ff != 10 + u16::from(rom_name.as_bytes()[0] - b'A')
                || cell(&sc, first_cell + line_cells - 1) & 0x3ff == 0x3c
            {
                continue;
            }
            let colour = cell(&sc, first_cell) & !0x3ff;
            let blank = cell(&sc, first_cell - 2);
            let mut want = sc.clone();
            let mut set = |col: usize, w: u16| {
                let i = ROW * 64 + col;
                want.videoram[2 * i..2 * i + 2].copy_from_slice(&w.to_be_bytes());
            };
            for col in first_cell..first_cell + line_cells {
                set(col, blank);
            }
            let rest: Vec<u16> = (first_cell + rom_name.len()..first_cell + line_cells).map(|col| cell(&sc, col)).collect();
            let first = (63 - (name.len() + rest.len())) / 2;
            for (i, b) in name.bytes().enumerate() {
                set(first + i, colour | (10 + u16::from(b - b'A')));
            }
            for (i, &w) in rest.iter().enumerate() {
                set(first + name.len() + i, w);
            }
            let expected = render_frame(&a, &want);
            let callouts = s.fight().unwrap().callouts().frame().elements();
            let y0 = ROW * 8 - 16;
            for i in (y0 * WIDTH..(y0 + 8) * WIDTH).filter(|&i| !callouts.iter().any(|e| e.rect.contains(i % WIDTH, i / WIDTH))) {
                assert_eq!(out.frame.argb[i], expected.argb[i], "{name} WON: pixel ({}, {}) frame {f}", i % WIDTH, i / WIDTH);
            }
            seen = true;
            break;
        }
        assert!(seen, "{name} won a round within the frames given");
    }
}

/// The bonus tally after a won round (`docs/reference/match-win.md`) heads
/// its block `ROUND BONUS` on text row 9, under the layout's TIME: while
/// the ROM has text on the rows the moved TIME covers, TIME gives way and
/// the ROM's own screen shows there, as the hardware draws it.
#[test]
fn the_moved_time_gives_way_to_the_round_bonus() {
    if assets().is_none() || !result_art_available() {
        return;
    }
    let (mut s, a) = lee_against_mike();
    const ROW: usize = 9;
    const COL: usize = 26;
    let mut seen = false;
    for f in 0..9_000u32 {
        let (forward, phase) = (f % 60 < 30, f % 60 == 45);
        let out = s.step(with(|i| {
            i.p1.directions[0] = forward;
            i.p1.buttons[5] = phase;
        }));
        let Some(m) = s.fight_machine() else { break };
        let sc = m.state.scene();
        let i = ROW * 64 + COL;
        // ROUND BONUS's R, typed with the block.
        if u16::from_be_bytes([sc.videoram[2 * i], sc.videoram[2 * i + 1]]) & 0x3ff != 10 + u16::from(b'R' - b'A') {
            continue;
        }
        let expected = render_frame(&a, &sc);
        let callouts = s.fight().unwrap().callouts().frame().elements();
        let (x0, y0, w, h) = NEW_TIME;
        for y in y0..y0 + h {
            for x in (x0..x0 + w).filter(|&x| !callouts.iter().any(|e| e.rect.contains(x, y))) {
                assert_eq!(out.frame.argb[y * WIDTH + x], expected.argb[y * WIDTH + x], "pixel ({x}, {y}) frame {f}");
            }
        }
        seen = true;
        break;
    }
    assert!(seen, "the bonus tally showed within the frames given");
}
