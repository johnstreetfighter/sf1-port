//! Ticket #240 (`TR3`): no clock and no round furniture in Training.
//!
//! At the front end's fight-screen seam, driven by inputs: in Training,
//! against a CPU or the dummy and under every preset, Faithful 1987
//! included, the ROM's TIME digits never count down, while in a Local
//! fight they do. Training's presented frames show no TIME (neither at the
//! ROM's bottom-left nor under the health bars), no P1 / HI / P2 scores and
//! no round-win marker cells (the ROM's empty slots at each side's marker
//! offset, `0x298` and `0x2e0`); under the overhaul presets the bars'
//! labels are the fighters' names as in every fight, and under Faithful
//! 1987 the frame is otherwise the plain 1987 frame as the hardware
//! renderer draws it, PLAYER1 / ENEMY included. The machine's work RAM and
//! text layer are only read. Local ROMs only; each test skips without them.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::roster::PlayMode;
use sf_video::render::{render_frame, Frame, WIDTH};
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
const TIME_DIGITS: u32 = 0xff8e18;
const DOWN: usize = 2;

const PRESETS: [RulesetPreset; 5] = [
    RulesetPreset::Faithful1987,
    RulesetPreset::DefinitiveOverhaul,
    RulesetPreset::ClassicCompetitive,
    RulesetPreset::PureFootsies1987Plus,
    RulesetPreset::Custom,
];

/// The ROM's TIME (text row 28, columns 9 to 16), the score row (row 3,
/// columns 10 to 53) and each side's round-win marker cells (rows 5 and 6,
/// four columns from the video offset `0x298` or `0x2e0`, the two empty
/// 2 x 2 slots `round_hud` draws), as text cells (row, first and last
/// column); and the overhaul layout's TIME under the bars as a screen
/// rectangle (x, y, w, h).
const TIME_CELLS: (usize, usize, usize) = (28, 9, 16);
const SCORE_CELLS: (usize, usize, usize) = (3, 10, 53);
const MARKER_CELLS: [(usize, usize, usize); 4] = [(5, 12, 15), (6, 12, 15), (5, 48, 51), (6, 48, 51)];
const NEW_TIME: (usize, usize, usize, usize) = (164, 52, 64, 8);

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
enum Kind {
    /// Training against the CPU under the cursor: Capcom's Retsu under
    /// Faithful 1987, the fair CPU's Ken under the overhaul presets (SP21).
    TrainingCpu,
    /// Training against the dummy Ken.
    TrainingDummy,
    /// Local, P1 Ryu against P2 Ken.
    Local,
}

/// A session of `kind` under `preset`, stopped at the first fight loop.
fn fight(kind: Kind, preset: RulesetPreset) -> (GameSession, Assets) {
    let a = assets().expect("local ROMs");
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    match kind {
        Kind::TrainingCpu | Kind::TrainingDummy => {
            tap(&mut s, with(|i| i.menu.confirm = true)); // Single Player
            tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
            tap(&mut s, with(|i| i.menu.confirm = true)); // Training
            if kind == Kind::TrainingDummy {
                tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
            }
            tap(&mut s, with(|i| i.menu.confirm = true)); // CPU or DUMMY
            tap(&mut s, with(|i| i.menu.confirm = true)); // P1 Ryu
            s.step(with(|i| i.menu.confirm = true)); // the opponent under the cursor
            if kind == Kind::TrainingDummy {
                while s.screen() != "country select" {
                    s.step(idle());
                }
                tap(&mut s, with(|i| i.p1.buttons[0] = true));
            }
        }
        Kind::Local => {
            tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
            tap(&mut s, with(|i| i.menu.confirm = true));
            tap(&mut s, with(|i| i.menu.confirm = true));
            tap(&mut s, with(|i| i.p1.buttons[0] = true));
            tap(&mut s, with(|i| i.p2.buttons[0] = true));
            while s.screen() != "country select" {
                s.step(idle());
            }
            tap(&mut s, with(|i| i.p1.buttons[0] = true));
        }
    }
    to_fight_loop(&mut s);
    let a = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    (s, a)
}

fn time_digits(s: &GameSession) -> [u8; 3] {
    let at = (TIME_DIGITS - 0xff8000) as usize;
    s.fight_machine().unwrap().work_ram()[at..at + 3].try_into().unwrap()
}

#[test]
fn the_clock_stands_still_in_training_and_runs_in_local_under_every_preset() {
    if assets().is_none() {
        return;
    }
    for preset in PRESETS {
        for kind in [Kind::TrainingCpu, Kind::TrainingDummy] {
            let (mut s, _) = fight(kind, preset);
            assert_eq!(s.fight_machine().unwrap().match_config().roster.mode, PlayMode::Training);
            let start = time_digits(&s);
            assert_eq!(start, [1, 0, 0], "{kind:?} {preset:?}: the round starts at the factory 100");
            // Twenty of the ROM's 19-frame ticks, and more.
            for f in 0..400 {
                s.step(idle());
                if s.fight_machine().is_none() {
                    break;
                }
                assert_eq!(time_digits(&s), start, "{kind:?} {preset:?} frame {f}");
            }
        }
        let (mut s, _) = fight(Kind::Local, preset);
        let start = time_digits(&s);
        for _ in 0..100 {
            s.step(idle());
        }
        assert_ne!(time_digits(&s), start, "Local {preset:?}: TIME counts down");
    }
}

fn scene(s: &GameSession) -> SceneState {
    s.fight_machine().unwrap().state.scene()
}

fn word(sc: &SceneState, col: usize, row: usize) -> u16 {
    let i = row * 64 + col;
    u16::from_be_bytes([sc.videoram[2 * i], sc.videoram[2 * i + 1]])
}

fn set(sc: &mut SceneState, col: usize, row: usize, w: u16) {
    let i = row * 64 + col;
    sc.videoram[2 * i..2 * i + 2].copy_from_slice(&w.to_be_bytes());
}

/// The scene with TIME, the scores and the round-win marker cells blanked
/// as each row's own blank neighbour (the playfield showing behind), and
/// with `names` written over the bars' labels (rows 5 and 7, from column
/// 22) in the labels' colours, as the overhaul layout names them.
fn blanked(s: &GameSession, names: Option<[&str; 2]>) -> SceneState {
    let mut sc = scene(s);
    for (row, first, last) in [TIME_CELLS, SCORE_CELLS].into_iter().chain(MARKER_CELLS) {
        let blank = word(&sc, first - 1, row);
        assert_eq!(blank & 0x3ff, 0x3c, "the cell before row {row}'s hidden cells is blank");
        for col in first..=last {
            set(&mut sc, col, row, blank);
        }
    }
    if let Some(names) = names {
        for (row, name) in [5, 7].into_iter().zip(names) {
            let blank = word(&sc, 21, row);
            let colour = word(&sc, 22, row) & !0x3ff;
            for col in 22..=28 {
                let code = match name.as_bytes().get(col - 22) {
                    Some(&b) => colour | (10 + u16::from(b - b'A')),
                    None => blank,
                };
                set(&mut sc, col, row, code);
            }
        }
    }
    sc
}

fn cells_rect((row, first, last): (usize, usize, usize)) -> (usize, usize, usize, usize) {
    (8 * first - 64, 8 * row - 16, 8 * (last + 1 - first), 8)
}

fn in_rect(r: (usize, usize, usize, usize), i: usize) -> bool {
    let (x, y) = (i % WIDTH, i / WIDTH);
    (r.0..r.0 + r.2).contains(&x) && (r.1..r.1 + r.3).contains(&y)
}

/// How many of `rect`'s pixels the presented `frame` changed from `plain`.
fn changed(frame: &Frame, plain: &Frame, rect: (usize, usize, usize, usize)) -> usize {
    (0..plain.argb.len()).filter(|&i| in_rect(rect, i) && frame.argb[i] != plain.argb[i]).count()
}

#[test]
fn training_shows_no_time_scores_or_round_markers_under_every_preset() {
    if assets().is_none() {
        return;
    }
    for preset in PRESETS {
        for kind in [Kind::TrainingCpu, Kind::TrainingDummy] {
            let (mut s, a) = fight(kind, preset);
            let faithful = preset == RulesetPreset::Faithful1987;
            let names = match kind {
                _ if faithful => None,
                _ => Some(["RYU", "KEN"]),
            };
            for f in 0..240 {
                let out = s.step(idle());
                assert_eq!(out.screen, "fight");
                let at = format!("{kind:?} {preset:?} frame {f}");
                let plain = render_frame(&a, &scene(&s));
                // What the ROM drew there is gone, the playfield in its
                // place: TIME, the scores and both sides' two marker slots.
                assert!(changed(&out.frame, &plain, cells_rect(TIME_CELLS)) > 20, "TIME is hidden {at}");
                assert!(changed(&out.frame, &plain, cells_rect(SCORE_CELLS)) > 50, "the scores are hidden {at}");
                for cells in MARKER_CELLS {
                    assert!(changed(&out.frame, &plain, cells_rect(cells)) > 20, "the marker cells {cells:?} are hidden {at}");
                }
                // Nothing is drawn in their place, TIME under the bars
                // included, and the rest of the frame is the hardware's
                // (with the overhaul's names on the bars): Faithful 1987
                // exactly, the overhauls but for the call-outs the fight
                // earned (a CPU's FIRST ATTACK), each inside its own box.
                let expected = render_frame(&a, &blanked(&s, names));
                assert_eq!(changed(&out.frame, &expected, NEW_TIME), 0, "no TIME under the bars {at}");
                if faithful {
                    assert!(out.frame.argb == expected.argb, "the plain 1987 frame but for the hidden cells {at}");
                    continue;
                }
                let callouts = s.fight().unwrap().callouts().frame().elements();
                for (i, (got, want)) in out.frame.argb.iter().zip(&expected.argb).enumerate() {
                    if !callouts.iter().any(|e| e.rect.contains(i % WIDTH, i / WIDTH)) {
                        assert_eq!(got, want, "pixel ({}, {}) {at}", i % WIDTH, i / WIDTH);
                    }
                }
            }
        }
    }
}
