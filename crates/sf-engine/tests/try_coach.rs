//! Ticket #231 (`ML5`): Try and the coach's success.
//!
//! At the headless session's seam, driven by inputs: TRY on a move's row in
//! Training (or right after a show) has the coach watch the player attempt
//! the move; each step of the strip lights as the move's own tracker counts
//! it; a recorded input that brings the move out gives its call-out and
//! resets the strip; walking and crouching start no attempt; an attempt a
//! hit ends counts as nothing; and the coach's reads leave the machine as
//! it was (ADR-0004). Local ROMs only; each test skips without them.
//!
//! Set `SF_TRY_FRAMES` to a directory to have the rendered frames of a Try
//! written there as PNGs.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::FighterRead;
use sf_engine::fighter::{P1_STRUCT, WORK_BASE};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::coach::{Miss, Outcome};
use sf_engine::front::moves::Choice;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::moves::{Dir, Recognizer, Strength, REGISTRY};
use sf_engine::roster::TournamentFighter;
use sf_video::callouts::{regions, CalloutKind, Side, Strip, LETTER_PENS, STRIP_UNLIT_PENS};
use sf_video::compositor::small_glyph;
use sf_video::decode::decode_char_tile;
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
const LP: usize = 0;

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

/// Training against the dummy (whose P2 side never moves) or the CPU, under
/// `preset`, with `fighter` as P1, stopped at the first fight loop and then
/// run through the round's intro to where the fighters take control.
fn training(preset: RulesetPreset, fighter: TournamentFighter, dummy: bool) -> GameSession {
    let a = assets().expect("local ROMs");
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm()); // Training
    if dummy {
        tap(&mut s, stick(DOWN));
    }
    tap(&mut s, confirm()); // CPU or DUMMY
    let at = TournamentFighter::ALL.iter().position(|&f| f == fighter).unwrap();
    for _ in 0..at % 6 {
        tap(&mut s, stick(RIGHT));
    }
    tap(&mut s, confirm()); // P1
    s.step(confirm()); // the opponent under the cursor
    if dummy {
        while s.screen() != "country select" {
            s.step(idle());
        }
        tap(&mut s, with(|i| i.p1.buttons[LP] = true));
    }
    to_fight_loop(&mut s);
    for _ in 0..240 {
        s.step(idle());
    }
    s
}

fn p1_move(s: &GameSession) -> u8 {
    s.fight().unwrap().fight_read().fighters[0].move_id
}

/// Open the pause box, choose MOVES and put the cursor on the registry
/// entry at `row` of the list.
fn open_list_at(s: &mut GameSession, row: usize) {
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    s.step(idle());
    let at = PauseBox::open(true).items().iter().position(|&i| i == MenuItem::Moves).expect("a MOVES row");
    for _ in 0..at {
        tap(s, stick(DOWN));
    }
    tap(s, confirm());
    assert!(s.move_list().is_some());
    for _ in 0..row {
        tap(s, stick(DOWN));
    }
    s.step(idle());
}

/// Confirm the row and then the choice (`SHOW` or `TRY`); the fight is
/// resumed by it.
fn choose(s: &mut GameSession, choice: Choice) {
    tap(s, confirm());
    assert_eq!(s.move_list().unwrap().choosing(), Some(Choice::Show));
    if choice == Choice::Try {
        tap(s, stick(RIGHT));
        assert_eq!(s.move_list().unwrap().choosing(), Some(Choice::Try));
    }
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
    assert!(s.move_list().is_none(), "the choice closes the panel");
    s.step(idle());
}

fn try_move(s: &mut GameSession, row: usize) {
    open_list_at(s, row);
    choose(s, Choice::Try);
}

/// The tracker bytes of the registry entry at `row`, read straight from the
/// work RAM: the steps the engine has counted.
fn engine_stage(s: &GameSession, row: usize) -> usize {
    let Recognizer::Tracker(t) = REGISTRY[row].recognizer;
    let at = (P1_STRUCT + t.offset() - WORK_BASE) as usize;
    let w = s.fight_machine().unwrap().work_ram();
    let (stage, done) = (w[at], w[at + 2]);
    if stage == 3 && done == 0 { 0 } else { usize::from(stage) }
}

fn lit(s: &GameSession) -> usize {
    s.coach().expect("a Try").lit()
}

fn words(s: &GameSession) -> Vec<CalloutKind> {
    s.fight().unwrap().callouts().frame().side(Side::P1).words.iter().flatten().copied().collect()
}

/// The pad frames of the entry's input facing right, then neutral.
fn recorded(row: usize) -> Vec<FrameInputs> {
    let frames = REGISTRY[row].input.frames(true, Strength::Medium);
    frames.into_iter().map(|p| with(|i| i.p1 = p)).collect()
}

/// A pad frame holding `dirs` (right, left, down, up).
fn hold(dirs: [bool; 4]) -> FrameInputs {
    with(|i| i.p1.directions = dirs)
}

/// Whether `text` is lettered at (`x`, `y`) in the ROM font in `pens`
/// (glyph pixels only).
fn lettered(frame: &Frame, a: &Assets, x: usize, y: usize, text: &str, pens: [u32; 3]) -> bool {
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| (0..8).all(|tx| tile[ty][tx] == 3 || frame.argb[(y + ty) * WIDTH + x + 8 * i + tx] == pens[tile[ty][tx] as usize]))
    })
}

/// The strip's steps lettered as the strip draws them with `lit` lit.
fn strip_drawn(frame: &Frame, a: &Assets, steps: &[&str], lit: usize) -> bool {
    let len: usize = steps.iter().map(|s| s.len()).sum::<usize>() + steps.len() - 1;
    let mut x = regions::STRIP.x + 8 * (regions::STRIP_LETTERS - len) / 2;
    steps.iter().enumerate().all(|(n, step)| {
        let pens = if n < lit { LETTER_PENS } else { STRIP_UNLIT_PENS };
        let ok = lettered(frame, a, x, regions::STRIP.y, step, pens);
        x += 8 * (step.len() + 1);
        ok
    })
}

fn write_png(dir: &str, name: &str, frame: &Frame) {
    std::fs::create_dir_all(dir).unwrap();
    let file = std::io::BufWriter::new(std::fs::File::create(Path::new(dir).join(format!("{name}.png"))).unwrap());
    let mut encoder = png::Encoder::new(file, WIDTH as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let rgb: Vec<u8> = frame.argb.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]).collect();
    encoder.write_header().unwrap().write_image_data(&rgb).unwrap();
}

#[test]
fn try_is_chosen_on_a_moves_row_and_only_in_training() {
    if assets().is_none() {
        return;
    }
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    assert!(s.coach().is_none());
    open_list_at(&mut s, 1);
    tap(&mut s, confirm());
    // Left and right pick between SHOW and TRY, both ways.
    tap(&mut s, stick(RIGHT));
    assert_eq!(s.move_list().unwrap().choosing(), Some(Choice::Try));
    tap(&mut s, stick(LEFT));
    assert_eq!(s.move_list().unwrap().choosing(), Some(Choice::Show));
    tap(&mut s, stick(RIGHT));
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
    assert!(s.move_list().is_none());
    let coach = s.coach().expect("the coach watches");
    assert_eq!(coach.entry().name, "SHORYUKEN");
    assert!(s.fight().unwrap().showing().is_none(), "nothing is fed: the pad is the player's");
    assert_eq!((coach.attempts(), coach.outcome()), (0, None));
}

#[test]
fn a_recorded_input_brings_the_move_out_with_its_call_out_and_the_strip_resets() {
    let Some(a) = assets() else { return };
    let dir = std::env::var("SF_TRY_FRAMES").ok();
    for (preset, (row, entry)) in RulesetPreset::ALL
        .into_iter()
        .filter(|p| matches!(p, RulesetPreset::DefinitiveOverhaul | RulesetPreset::Faithful1987))
        .flat_map(|p| REGISTRY.iter().enumerate().filter(|(_, e)| e.available.fighters.contains(&TournamentFighter::Ryu)).map(move |e| (p, e)))
    {
        let overhaul = preset == RulesetPreset::DefinitiveOverhaul;
        let mut s = training(preset, TournamentFighter::Ryu, true);
        try_move(&mut s, row);
        let steps = entry.input.strip_steps();
        let steps: Vec<&str> = steps.iter().map(String::as_str).collect();
        assert_eq!(lit(&s), 0, "{}: nothing counted yet", entry.name);
        let mut succeeded = None;
        let mut max_lit = 0;
        for (n, input) in recorded(row).into_iter().chain(vec![idle(); 150]).enumerate() {
            let frame = s.step(input).frame;
            // The strip follows the engine's own stage byte, frame by frame;
            // on the frame the move comes out it resets, and from then on it
            // follows the tracker again (a direction still held starts a new
            // first step).
            let coach = s.coach().unwrap();
            let expected = if succeeded.is_none() && coach.successes() == 1 { 0 } else { engine_stage(&s, row) };
            assert_eq!(lit(&s), expected, "{preset:?} {}: frame {n}", entry.name);
            assert!(strip_drawn(&frame, &a, &steps, lit(&s)), "{preset:?} {}: frame {n} draws {} lit", entry.name, lit(&s));
            max_lit = max_lit.max(lit(&s));
            if succeeded.is_none() && coach.successes() == 1 {
                succeeded = Some(n);
                // The move came out from the player's side, named by its call-out.
                assert_eq!(p1_move(&s), entry.produces, "{}", entry.name);
                assert_eq!(coach.outcome(), Some(Outcome::Success));
                assert_eq!(words(&s), [CalloutKind::Move { word: entry.call_out }]);
                let slot = regions::word_slot_at(Side::P1, 0, regions::TRY_DROP);
                assert!(lettered(&frame, &a, slot.x, slot.y, entry.call_out, LETTER_PENS), "{}: called out", entry.name);
                if let (Some(dir), 0, true) = (&dir, row, overhaul) {
                    write_png(dir, "success_frame", &frame);
                }
            } else if let (Some(dir), Some(at), 0, true) = (&dir, succeeded, row, overhaul) {
                if n == at + 40 {
                    write_png(dir, "reset_after_success", &frame);
                }
            }
            if let (Some(dir), 0, true) = (&dir, row, overhaul) {
                if matches!(n, 1 | 4 | 7) {
                    write_png(dir, &format!("stage_{}_frame_{n}", lit(&s)), &frame);
                }
            }
        }
        assert!(succeeded.is_some(), "{} came out", entry.name);
        assert_eq!(max_lit, 3, "{}: the motion's three steps lit before it came out", entry.name);
        // One success, the strip back to nothing lit, the attempt over.
        let coach = s.coach().unwrap();
        assert_eq!((coach.attempts(), coach.successes(), coach.attempting()), (1, 1, false), "{}", entry.name);
        assert_eq!(lit(&s), 0, "{}: the strip is back to nothing lit", entry.name);
        assert!(words(&s).is_empty(), "{}: the call-out has run its time", entry.name);
        // The Try runs on: a second go gives a second call-out.
        for input in recorded(row).into_iter().chain(vec![idle(); 150]) {
            s.step(input);
        }
        assert_eq!(s.coach().unwrap().successes(), 2, "{}: again", entry.name);
    }
}

#[test]
fn a_move_is_called_out_in_every_preset_faithful_1987_included() {
    let Some(a) = assets() else { return };
    for preset in RulesetPreset::ALL {
        let mut s = training(preset, TournamentFighter::Ryu, true);
        try_move(&mut s, 0);
        let mut called = false;
        for input in recorded(0).into_iter().chain(vec![idle(); 40]) {
            let frame = s.step(input).frame;
            let slot = regions::word_slot_at(Side::P1, 0, regions::TRY_DROP);
            let now = words(&s).len() == 1 && lettered(&frame, &a, slot.x, slot.y, "HADOKEN!", LETTER_PENS);
            if let (Ok(dir), true, false) = (std::env::var("SF_TRY_FRAMES"), now, called) {
                write_png(&dir, &format!("called_{preset:?}"), &frame);
            }
            called |= now;
        }
        assert!(called, "{preset:?}: HADOKEN! is on the screen");
        let strip = s.coach().unwrap().strip();
        assert_eq!(strip, Strip::Input { steps: REGISTRY[0].input.strip_steps(), lit: 0 });
    }
}

#[test]
fn a_show_that_runs_its_course_goes_on_as_a_try_of_the_same_move() {
    if assets().is_none() {
        return;
    }
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    open_list_at(&mut s, 2);
    choose(&mut s, Choice::Show);
    assert!(s.coach().is_none(), "while it is shown");
    let mut frames = 0;
    while s.fight().unwrap().showing().is_some() {
        s.step(idle());
        frames += 1;
        assert!(frames < 600);
    }
    let coach = s.coach().expect("the Try follows the show");
    assert_eq!(coach.entry().name, "TATSUMAKI");
    // The show's own move is no success of the Try's, and the pad is the player's.
    assert_eq!((coach.attempts(), coach.successes(), coach.outcome()), (0, 0, None));
    for input in recorded(2).into_iter().chain(vec![idle(); 150]) {
        s.step(input);
    }
    assert_eq!(s.coach().unwrap().successes(), 1);

    // A show given up on does not: a press while it waits takes the pad back.
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    for _ in 0..4 {
        s.step(stick(UP));
    }
    for _ in 0..4 {
        s.step(idle());
    }
    open_list_at(&mut s, 0);
    choose(&mut s, Choice::Show);
    s.step(idle());
    s.step(stick(LEFT));
    assert!(s.fight().unwrap().showing().is_none());
    assert!(s.coach().is_none(), "no Try after a show that was cancelled");
}

#[test]
fn the_try_runs_until_the_player_pauses_or_picks_another_move() {
    if assets().is_none() {
        return;
    }
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    try_move(&mut s, 0);
    for _ in 0..300 {
        s.step(idle());
    }
    assert_eq!(s.coach().map(|c| c.entry().name), Some("HADOKEN"), "it runs on");
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    assert!(s.coach().is_none(), "the pause ends it");
    s.step(idle());
    // Resume plainly: no Try. Then pick another move.
    tap(&mut s, confirm());
    assert!(s.coach().is_none());
    open_list_at(&mut s, 1);
    choose(&mut s, Choice::Try);
    assert_eq!(s.coach().map(|c| c.entry().name), Some("SHORYUKEN"));
    // Showing another move replaces a Try, too.
    open_list_at(&mut s, 0);
    choose(&mut s, Choice::Show);
    assert!(s.coach().is_none());
    assert!(s.fight().unwrap().showing().is_some());
}

#[test]
fn walking_and_crouching_start_no_attempt() {
    if assets().is_none() {
        return;
    }
    for row in 0..sf_engine::moves::move_list(TournamentFighter::Ryu, RulesetPreset::DefinitiveOverhaul).len() {
        let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
        try_move(&mut s, row);
        let mut deepest = 0;
        for dirs in [[true, false, false, false], [false, true, false, false], [false, false, true, false]] {
            for _ in 0..80 {
                s.step(hold(dirs));
                deepest = deepest.max(lit(&s));
                assert_eq!(lit(&s), engine_stage(&s, row));
            }
            for _ in 0..40 {
                s.step(idle());
            }
        }
        let coach = s.coach().unwrap();
        assert!(deepest <= 1, "{}: only a first step is ever counted ({deepest})", REGISTRY[row].name);
        assert_eq!((coach.attempts(), coach.outcome(), coach.attempting()), (0, None, false), "{}", REGISTRY[row].name);
        assert!(words(&s).is_empty());
    }
    // The first step does light while it is held: crouching for the Hadoken.
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    try_move(&mut s, 0);
    for _ in 0..10 {
        s.step(stick(DOWN));
    }
    assert_eq!(lit(&s), 1);
}

#[test]
fn a_motion_started_and_dropped_is_an_attempt_that_missed_and_the_view_names_where() {
    if assets().is_none() {
        return;
    }
    for preset in [RulesetPreset::DefinitiveOverhaul, RulesetPreset::Faithful1987] {
        let mut s = training(preset, TournamentFighter::Ryu, true);
        try_move(&mut s, 0);
        // D then DF, and the stick let go: two steps through, the third awaited.
        for _ in 0..3 {
            s.step(hold([false, false, true, false]));
        }
        for _ in 0..3 {
            s.step(hold([true, false, true, false]));
        }
        assert_eq!(lit(&s), 2);
        assert!(s.coach().unwrap().attempting(), "{preset:?}: the second step starts an attempt");
        let mut ended = None;
        for _ in 0..80 {
            s.step(idle());
            ended = ended.or(s.coach().unwrap().outcome());
        }
        let Some(Outcome::Miss(miss)) = ended else { panic!("{preset:?}: a miss, not {ended:?}") };
        let Miss { stage, awaited, accepted, frames, current_move, ended, done_gate_closed, .. } = miss;
        assert_eq!((stage, awaited), (2, Some(Dir::Forward)), "{preset:?}: stalled at step two, awaiting F from the ROM's table");
        assert_eq!(accepted, [Some(Dir::Forward), None], "{preset:?}");
        assert_eq!((ended, done_gate_closed), (sf_engine::front::coach::Ended::WindowRanOut, 0), "{preset:?}");
        // Counted from the first direction: 6 frames of input and the window's run.
        assert!(frames > 18, "{preset:?}: {frames}");
        assert_ne!(current_move, REGISTRY[0].produces);
        let coach = s.coach().unwrap();
        assert_eq!((coach.attempts(), coach.successes(), coach.attempting()), (1, 0, false));
        // The miss is worded (ML6): the awaited direction, from the ROM's table.
        let said: Vec<String> = words(&s).iter().map(|w| w.word().to_owned()).collect();
        assert_eq!(said, ["MISSED \u{2192}"], "{preset:?}: a miss is worded");
        assert_eq!(coach.fix().map(|f| f.kind), Some(sf_engine::front::coach::FixKind::Missed(Dir::Forward)), "{preset:?}");
        assert_eq!(lit(&s), 0);
    }
}

#[test]
fn an_attempt_ended_by_a_hit_counts_as_nothing() {
    if assets().is_none() {
        return;
    }
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    try_move(&mut s, 0);
    for _ in 0..3 {
        s.step(hold([false, false, true, false]));
    }
    for _ in 0..3 {
        s.step(hold([true, false, true, false]));
    }
    assert!(s.coach().unwrap().attempting());
    // The fighter is hit: the fight reader's next read has him in a hit
    // reaction (the 1987 fight has no hit to land on a dummy).
    let mut hit = *s.fight().unwrap().fight_read();
    hit.fighters[0] = FighterRead { move_id: 0x30, state: 0x82, ..hit.fighters[0] };
    s.feed_fight_reads(&[hit]);
    s.step(hold([true, false, true, false]));
    let coach = s.coach().unwrap();
    assert_eq!((coach.outcome(), coach.attempting()), (Some(Outcome::Interrupted), false));
    // The tracker runs out afterwards: no miss, no success, no words, no fix.
    for _ in 0..120 {
        s.step(idle());
        assert!(words(&s).is_empty());
    }
    assert!(s.coach().unwrap().fix().is_none() && s.coach().unwrap().shown_inputs().is_none());
    let coach = s.coach().unwrap();
    assert_eq!((coach.outcome(), coach.attempts(), coach.successes()), (Some(Outcome::Interrupted), 1, 0));

    // A block is the same.
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    try_move(&mut s, 0);
    for _ in 0..3 {
        s.step(hold([false, false, true, false]));
    }
    for _ in 0..3 {
        s.step(hold([true, false, true, false]));
    }
    // The opponent is attacking and the fighter guards it.
    let mut block = *s.fight().unwrap().fight_read();
    block.fighters[0] = FighterRead { move_id: 8, state: 0x24, ..block.fighters[0] };
    block.fighters[1] = FighterRead { move_id: 0x10, state: 0xc2, ..block.fighters[1] };
    s.feed_fight_reads(&[block]);
    s.step(hold([true, false, true, false]));
    assert_eq!(s.coach().unwrap().outcome(), Some(Outcome::Interrupted));

    // The guard posture with nothing coming is not a block.
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    try_move(&mut s, 0);
    for _ in 0..3 {
        s.step(hold([false, false, true, false]));
    }
    for _ in 0..3 {
        s.step(hold([true, false, true, false]));
    }
    let mut posture = *s.fight().unwrap().fight_read();
    posture.fighters[0] = FighterRead { move_id: 8, state: 0x24, ..posture.fighters[0] };
    s.feed_fight_reads(&[posture]);
    s.step(hold([true, false, true, false]));
    assert_ne!(s.coach().unwrap().outcome(), Some(Outcome::Interrupted));
    assert!(s.coach().unwrap().attempting());
}

#[test]
fn the_coach_leaves_the_machine_as_it_was() {
    if assets().is_none() {
        return;
    }
    // The same fight with the same inputs, with a Try watching and with
    // none: bit for bit the same machine, and the recognizer views leave it
    // untouched.
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        let base = training(preset, TournamentFighter::Ryu, true);
        let mut watched = base.clone();
        try_move(&mut watched, 0);
        let mut plain = base;
        assert_eq!(s_pause_resume(&mut plain), Some(Transition::Paused));
        for (n, input) in recorded(0).into_iter().chain(vec![idle(); 200]).enumerate() {
            watched.step(input);
            plain.step(input);
            let (a, b) = (watched.fight_machine().unwrap(), plain.fight_machine().unwrap());
            assert_eq!(a.state.frame, b.state.frame);
            assert_eq!(a.work_ram(), b.work_ram(), "{preset:?}: frame {n}");
            assert_eq!(a.state.objectram, b.state.objectram, "{preset:?}: frame {n}");
        }
        assert_eq!(watched.fight_machine().unwrap().save_snapshot(), plain.fight_machine().unwrap().save_snapshot());
        assert_eq!(watched.coach().unwrap().successes(), 1);
        // And a view, read at will, writes nothing.
        let m = watched.fight_machine().unwrap();
        let before = m.save_snapshot();
        let machine = sf_engine::moves::MachineRead { rom: m.program_rom(), work: m.work_ram(), obj: &m.state.objectram, bank: m.data_bank() };
        for entry in REGISTRY {
            entry.recognizer.view(machine, P1_STRUCT, &MatchConfig::from_preset(preset).toggles);
        }
        assert_eq!(m.save_snapshot(), before);
    }
}

/// Pause and resume with the plain RESUME row: the same pause the Try
/// takes, with nothing chosen.
fn s_pause_resume(s: &mut GameSession) -> Option<Transition> {
    let out = s.step(start());
    s.step(idle());
    tap(s, confirm());
    out.transition
}

#[test]
fn a_real_hit_from_the_cpu_ends_an_attempt_as_nothing() {
    if assets().is_none() {
        return;
    }
    // Against the CPU, a partial motion tried over and over (D then DF, the
    // stick let go) is caught by one of its attacks sooner or later; the
    // fight is deterministic, so the same frame every time.
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, false);
    try_move(&mut s, 0);
    let mut hit_at = None;
    let (mut landed, health) = (false, s.fight().unwrap().fight_read().fighters[0].health);
    for n in 0..6000 {
        let phase = n % 70;
        // A throw can change sides; follow P1's own live fighter entity.
        let m = s.fight_machine().unwrap();
        let facing = sf_engine::moves::facing_right(m.work_ram(), &m.state.objectram, P1_STRUCT);
        let input = match phase {
            0..=5 => hold([false, false, true, false]),
            6..=13 => hold([facing, !facing, true, false]),
            _ => idle(),
        };
        s.step(input);
        landed |= s.fight().unwrap().fight_read().landed[1].is_some();
        let coach = s.coach().unwrap();
        if coach.outcome() == Some(Outcome::Interrupted) {
            let me = s.fight().unwrap().fight_read().fighters[0];
            assert!(me.reacting(), "interrupted by a hit reaction, in move {:#x}", me.move_id);
            assert!(landed, "the reader saw a hit land on the fighter");
            assert!(me.health < health, "and it took health: {} of {health}", me.health);
            hit_at = Some(n);
            break;
        }
    }
    assert!(hit_at.is_some(), "the CPU hit the fighter mid-attempt");
    // It counts as nothing: no success, no call-out, and the hit's attempt is
    // not counted as a miss by the tracker running out afterwards.
    // (The earlier attempts of the loop were misses and have their fixes up still;
    // nothing is said about the hit's.)
    let before = (s.coach().unwrap().attempts(), s.coach().unwrap().successes());
    assert_eq!(before.1, 0);
    let up = words(&s);
    for _ in 0..40 {
        s.step(idle());
        assert!(words(&s).iter().all(|w| up.contains(w)), "no new word after the hit");
        assert_eq!(s.coach().unwrap().outcome(), Some(Outcome::Interrupted));
    }
    assert_eq!((s.coach().unwrap().attempts(), s.coach().unwrap().successes()), before);
}


#[test]
fn a_tatsumaki_against_the_cpu_is_called_out_every_time_it_comes_out() {
    if assets().is_none() {
        return;
    }
    // The motion goes through down-back and back, which the engine takes for
    // a guard whenever the opponent is attacking: the coach must not take that
    // posture for a hit, and every Tatsumaki that comes out is a success.
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        let mut interrupted_before = 0;
        let mut s = training(preset, TournamentFighter::Ryu, false);
        try_move(&mut s, 2);
        let (mut outs, mut prev) = (0u32, 0u8);
        let mut last_before = None;
        let period = REGISTRY[2].input.frames(true, Strength::Medium).len() + 30;
        // Under the overhaul preset the CPU is the fair CPU (SP21), which
        // seldom attacks into a motion on its own: there P1 starts each
        // motion as the CPU's attack starts (a Tatsumaki out of a block).
        let reactive = preset != RulesetPreset::Faithful1987;
        let mut at: Option<usize> = None;
        let mut cpu_was_attacking = false;
        for n in 0..16000 {
            // The registry's own input, for the way the fighter faces now, over
            // and over: the engine blocks mid-motion whenever the CPU attacks.
            // P1's entity slot can change during an attack or a throw.
            let m = s.fight_machine().unwrap();
            let facing_right = sf_engine::moves::facing_right(m.work_ram(), &m.state.objectram, P1_STRUCT);
            let frames = REGISTRY[2].input.frames(facing_right, Strength::Medium);
            let cpu_attacking = s.fight().unwrap().fight_read().fighters[1].attacking();
            let step = if reactive {
                if at.is_none() && cpu_attacking && !cpu_was_attacking {
                    at = Some(0);
                }
                let step = at;
                at = at.map(|f| f + 1).filter(|&f| f < frames.len());
                step
            } else {
                Some(n % period)
            };
            cpu_was_attacking = cpu_attacking;
            let mut input = idle();
            input.p1 = step.and_then(|f| frames.get(f)).copied().unwrap_or_default();
            s.step(input);
            let mv = p1_move(&s);
            if mv != 0x1b {
                last_before = s.coach().unwrap().outcome();
            }
            if mv == 0x1b && prev != 0x1b {
                interrupted_before += usize::from(last_before == Some(Outcome::Interrupted));
                outs += 1;
                assert_eq!(s.coach().unwrap().successes(), outs, "{preset:?} frame {n}: {:?}", s.coach().unwrap().outcome());
                assert!(words(&s).contains(&CalloutKind::Move { word: "TATSUMAKI!" }), "{preset:?} frame {n}: called out");
            }
            prev = mv;
            // Until one comes out of an attempt the guard cut short.
            if outs >= 4 && (interrupted_before > 0 || outs >= 40) {
                break;
            }
        }
        assert!(outs >= 4, "{preset:?}: the Tatsumaki came out {outs} times");
        println!("{preset:?}: {outs} out, {interrupted_before} after an interrupted attempt");
        // At least one of them came out after the engine's guard had ended
        // the attempt: it counts all the same.
        assert!(interrupted_before > 0, "{preset:?}: a Tatsumaki came out after an interrupted attempt");
    }
}

#[test]
fn the_views_gate_is_the_one_the_input_handler_applies() {
    if assets().is_none() {
        return;
    }
    // A Hadoken motion, then forward held with no button: the fighter walks.
    // The engine takes a button on a frame exactly when the view reads the
    // gate as free at the start of it: the overhaul attacks on the press, the
    // 1987 on the release (a press on one frame, let go on the next, so the
    // second frame's gate is the one that counts).
    let read_gate = |s: &GameSession, toggles: &sf_engine::config::CoreToggles| {
        let m = s.fight_machine().unwrap();
        let machine = sf_engine::moves::MachineRead { rom: m.program_rom(), work: m.work_ram(), obj: &m.state.objectram, bank: m.data_bank() };
        REGISTRY[0].recognizer.view(machine, P1_STRUCT, toggles).gate_free
    };
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        let mut s = training(preset, TournamentFighter::Ryu, true);
        try_move(&mut s, 0);
        for _ in 0..3 {
            s.step(hold([false, false, true, false]));
        }
        for _ in 0..3 {
            s.step(hold([true, false, true, false]));
        }
        let (mut free, mut held) = (0, 0);
        let toggles = MatchConfig::from_preset(preset).toggles;
        for n in 0..40 {
            s.step(hold([true, false, false, false]));
            if !matches!(p1_move(&s), 2 | 3) {
                continue;
            }
            let first = read_gate(&s, &toggles);
            let mut next = s.clone();
            next.step(with(|i| {
                i.p1.directions[RIGHT] = true;
                i.p1.buttons[LP] = true;
            }));
            let on_press = !matches!(p1_move(&next), 2 | 3);
            let second = read_gate(&next, &toggles);
            next.step(hold([true, false, false, false]));
            let on_release = !matches!(p1_move(&next), 2 | 3);
            if preset == RulesetPreset::DefinitiveOverhaul {
                assert_eq!(first, on_press, "{preset:?} walk frame {n}: the view says free={first}");
            } else if !on_press {
                assert_eq!(second, on_release, "{preset:?} walk frame {n}: the view says free={second}");
            }
            if first {
                free += 1;
            } else {
                held += 1;
            }
        }
        assert!(free > 0, "{preset:?}: the walk has frames the view reads as free");
        if preset == RulesetPreset::Faithful1987 {
            assert!(held > 0, "{preset:?}: and the 1987 walk's gated frames as held");
        } else {
            assert_eq!(held, 0, "{preset:?}: the interruptible walk is never held");
        }
    }
}

#[test]
fn ordinary_movement_into_a_motion_that_cannot_finish_never_yields_a_success_or_a_word() {
    if assets().is_none() {
        return;
    }
    // Walk forward then crouch (the Shoryuken's F, D) and crouch then crouch
    // back (the Tatsumaki's D, DB): attempts that stop short. Nothing comes
    // out, nothing is called, and the miss is worded by its fix (ML6).
    for preset in [RulesetPreset::DefinitiveOverhaul, RulesetPreset::Faithful1987] {
        for (row, steps) in [
            (1usize, [[true, false, false, false], [false, false, true, false]]),
            (2, [[false, false, true, false], [false, true, true, false]]),
        ] {
            let mut s = training(preset, TournamentFighter::Ryu, true);
            try_move(&mut s, row);
            for dirs in steps {
                for _ in 0..4 {
                    s.step(hold(dirs));
                    assert_eq!(lit(&s), engine_stage(&s, row));
                }
            }
            for _ in 0..120 {
                s.step(idle());
                // Nothing is called out as a move that came out: only a fix.
                assert!(words(&s).iter().all(|w| matches!(w, CalloutKind::Fix { .. })), "{preset:?} row {row}: {:?}", words(&s));
            }
            assert!(matches!(s.coach().unwrap().outcome(), Some(Outcome::Miss(_))), "{preset:?} row {row}");
            let coach = s.coach().unwrap();
            assert_eq!(coach.successes(), 0, "{preset:?} row {row}");
            assert!(!matches!(coach.outcome(), Some(Outcome::Success)), "{preset:?} row {row}");
            assert!(!coach.attempting(), "{preset:?} row {row}: the attempt is settled");
            assert_ne!(p1_move(&s), REGISTRY[row].produces);
        }
    }
}

#[test]
fn another_special_coming_out_is_other_not_a_miss() {
    if assets().is_none() {
        return;
    }
    // The Hadoken tried and the Shoryuken's input given: D then DF are the
    // Hadoken's first two steps, and the Shoryuken comes out.
    let mut s = training(RulesetPreset::Faithful1987, TournamentFighter::Ryu, true);
    try_move(&mut s, 0);
    for input in recorded(1).into_iter().chain(vec![idle(); 60]) {
        s.step(input);
    }
    let coach = s.coach().unwrap();
    assert_eq!((coach.successes(), coach.outcome()), (0, Some(Outcome::Other(0x19))));
    assert!(words(&s).is_empty());
    assert!(coach.fix().is_none(), "the game took an input: no fix");
}

#[test]
fn a_move_the_recovery_buffer_brings_out_after_the_tracker_is_gone_is_still_a_success() {
    if assets().is_none() {
        return;
    }
    // A heavy punch, then the Hadoken's motion with the button pressed in
    // its last frames and the stick let go: the tracker clears on the release
    // (the buffer's presets without lenient trackers), the buffered button
    // brings the Hadoken out when the recovery ends. The miss is held open
    // for the buffer's window and then is a success.
    let mut s = training(RulesetPreset::PureFootsies1987Plus, TournamentFighter::Ryu, true);
    try_move(&mut s, 0);
    let frames = REGISTRY[0].input.frames(true, Strength::Medium);
    let (punch_at, lead) = (4usize, 21usize);
    let (mut gone_while_held, mut out_at) = (false, None);
    for n in 0..80usize {
        let mut p = sf_engine::front::PlayerInputs::default();
        p.buttons[2] = n == punch_at;
        if let Some(k) = n.checked_sub(punch_at + lead) {
            let q = frames.get(k).copied().unwrap_or_default();
            if k < 8 {
                p.directions = q.directions;
            }
            for b in 0..6 {
                p.buttons[b] |= q.buttons[b];
            }
        }
        s.step(with(|i| i.p1 = p));
        let coach = s.coach().unwrap();
        if engine_stage(&s, 0) == 0 && p1_move(&s) == 0x10 && n > punch_at + lead {
            // The tracker is gone and the fighter is still held: not a miss yet.
            gone_while_held = true;
            assert!(coach.attempting() && coach.outcome().is_none(), "frame {n}: held open, not a miss");
        }
        if p1_move(&s) == 0x1a && out_at.is_none() {
            out_at = Some(n);
            assert_eq!(coach.outcome(), Some(Outcome::Success), "frame {n}");
        }
    }
    assert!(gone_while_held, "the tracker was gone before the buffered move came out");
    assert!(out_at.is_some(), "the buffer brought the Hadoken out");
    assert_eq!(s.coach().unwrap().successes(), 1);
}

#[test]
fn under_faithful_1987_a_try_changes_only_the_strip_and_the_call_out() {
    let Some(_) = assets() else { return };
    let base = training(RulesetPreset::Faithful1987, TournamentFighter::Ryu, true);
    let mut watched = base.clone();
    try_move(&mut watched, 0);
    let mut plain = base;
    assert_eq!(s_pause_resume(&mut plain), Some(Transition::Paused));
    let strip = regions::STRIP;
    let column = regions::callout_column_at(Side::P1, regions::TRY_DROP);
    let (mut strip_differed, mut column_differed) = (false, false);
    for (n, input) in recorded(0).into_iter().chain(vec![idle(); 120]).enumerate() {
        let (a, b) = (watched.step(input).frame, plain.step(input).frame);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if a.argb[y * WIDTH + x] == b.argb[y * WIDTH + x] {
                    continue;
                }
                if strip.contains(x, y) {
                    strip_differed = true;
                } else if column.contains(x, y) {
                    column_differed = true;
                } else {
                    panic!("frame {n}: pixel ({x}, {y}) differs outside the strip and the call-out column");
                }
            }
        }
    }
    assert!(strip_differed && column_differed, "the strip and the call-out are drawn");
}

