//! Ticket #232 (`ML6`): the coach's fixes.
//!
//! At the headless session's seam, driven by recorded inputs. A miss is an
//! attempt that ends without the move; the engine says that it failed and
//! where, and the player's own inputs choose the words: TOO SLOW, MISSED and
//! the direction, PRESS P SOONER, BUTTON TOO EARLY, WAIT TO RECOVER. Each
//! kind of miss is recorded for every move of the registry under the 1987
//! and an overhaul preset, run through the session, and each fix's claim is
//! checked against the engine: the same inputs inside the window bring the
//! move out (TOO SLOW), the awaited direction added does (MISSED), a button
//! in time does (PRESS P SOONER), the button after the motion does (BUTTON TOO
//! EARLY), and the busy gate held on the button (WAIT TO RECOVER, with the
//! same inputs after the recovery bringing it out). Local ROMs only; each
//! test skips without them.
//!
//! Set `SF_FIX_FRAMES` to a directory to have the rendered frames of each
//! kind of miss written there as PNGs.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fighter::{P1_STRUCT, WORK_BASE};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::coach::{FixKind, Outcome, INPUTS_FRAMES};
use sf_engine::front::moves::Choice;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, MenuItem, Transition};
use sf_engine::moves::{input_gate_free, ButtonClass, Dir, InputNotation, MachineRead, MoveEntry, Recognizer, REGISTRY};
use sf_video::callouts::{regions, CalloutKind, Side, STRIP_INPUTS_PENS};
use sf_video::compositor::small_glyph;
use sf_video::decode::decode_char_tile;
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const DOWN: usize = 2;

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

/// Training against the dummy under `preset`, Ryu as P1, with `notation`
/// chosen for MOVE INPUTS, run through the round's intro to where the
/// fighters take control.
fn training(preset: RulesetPreset, notation: InputNotation) -> GameSession {
    let a = assets().expect("local ROMs");
    let settings = Settings { move_inputs: notation, ..Settings::default() };
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset)).with_settings(settings);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm()); // Training
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm()); // DUMMY
    select::confirm_fighter(&mut s, confirm(), tap); // P1 (Ryu is the first on the select)
    select::confirm_fighter(&mut s, confirm(), GameSession::step); // the opponent under the cursor
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    for _ in 0..240 {
        s.step(idle());
    }
    s
}

/// A Try of the registry's entry at `row`, started from the pause box's
/// MOVES row.
fn try_move(s: &mut GameSession, row: usize) {
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    s.step(idle());
    let at = sf_engine::front::pause::PauseBox::open(true).items().iter().position(|&i| i == MenuItem::Moves).unwrap();
    for _ in 0..at {
        tap(s, stick(DOWN));
    }
    tap(s, confirm());
    for _ in 0..row {
        tap(s, stick(DOWN));
    }
    s.step(idle());
    tap(s, confirm());
    tap(s, stick(RIGHT));
    assert_eq!(s.move_list().unwrap().choosing(), Some(Choice::Try));
    assert_eq!(s.step(confirm()).transition, Some(Transition::Choose(MenuItem::Resume)));
    s.step(idle());
    assert_eq!(s.coach().map(|c| c.entry().name), Some(REGISTRY[row].name));
}

fn words(s: &GameSession) -> Vec<CalloutKind> {
    s.fight().unwrap().callouts().frame().side(Side::P1).words.iter().flatten().copied().collect()
}

fn fix_words(s: &GameSession) -> Vec<String> {
    words(s).into_iter().filter(|w| matches!(w, CalloutKind::Fix { .. })).map(|w| w.word().to_owned()).collect()
}

/// Whether `text` is lettered at (`x`, `y`) in the ROM font in `pens`
/// (glyph pixels only; characters the ROM font has no tile for are skipped).
fn lettered(frame: &Frame, a: &Assets, x: usize, y: usize, text: &str, pens: [u32; 3]) -> bool {
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| (0..8).all(|tx| tile[ty][tx] == 3 || frame.argb[(y + ty) * WIDTH + x + 8 * i + tx] == pens[tile[ty][tx] as usize]))
    })
}

/// Whether the cell at (`x`, `y`) has a pixel in `pen`.
fn inked(frame: &Frame, x: usize, y: usize, pen: u32) -> bool {
    (0..8).any(|dy| (0..8).any(|dx| frame.argb[(y + dy) * WIDTH + x + dx] == pen))
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

fn stick_of(d: Option<Dir>) -> [bool; 4] {
    match d {
        None | Some(Dir::Neutral) => [false; 4],
        Some(Dir::Forward) => [true, false, false, false],
        Some(Dir::Back | Dir::ChargeBack) => [false, true, false, false],
        Some(Dir::Down | Dir::ChargeDown) => [false, false, true, false],
        Some(Dir::Up | Dir::Jump) => [false, false, false, true],
        Some(Dir::DownForward) => [true, false, true, false],
        Some(Dir::DownBack) => [false, true, true, false],
        Some(Dir::UpForward) => [true, false, false, true],
        Some(Dir::UpBack) => [false, true, false, true],
    }
}

/// One stretch of a recorded attempt: the stick (facing right), the buttons
/// held (a bit each for LP, MP, HP, LK, MK, HK) and how many frames.
type Seg = (Option<Dir>, u8, usize);

const NONE: u8 = 0;
const MP: u8 = 1 << 1;
const HP: u8 = 1 << 2;
const MK: u8 = 1 << 4;
const HK: u8 = 1 << 5;

fn inputs(segs: &[Seg]) -> Vec<FrameInputs> {
    segs.iter()
        .flat_map(|&(d, buttons, n)| {
            (0..n).map(move |_| {
                with(|i| {
                    i.p1.directions = stick_of(d);
                    for k in 0..6 {
                        i.p1.buttons[k] = buttons & (1 << k) != 0;
                    }
                })
            })
        })
        .collect()
}

/// The buttons of a frame as the bit mask `Seg` uses.
fn bits(i: &FrameInputs) -> u8 {
    (0..6).fold(0, |m, k| m | u8::from(i.p1.buttons[k]) << k)
}

fn set_bits(i: &mut FrameInputs, mask: u8) {
    for k in 0..6 {
        i.p1.buttons[k] = mask & (1 << k) != 0;
    }
}

/// A Try of `row` on a session under `preset`, ready to be cloned.
fn base(preset: RulesetPreset, row: usize, notation: InputNotation) -> GameSession {
    let mut s = training(preset, notation);
    try_move(&mut s, row);
    s
}

/// What the engine did on one frame, read straight from the machine and
/// independent of the coach: the tracker's bytes, the input handler's gate,
/// the punch and kick latches and the fighter's move.
#[derive(Clone, Copy, Debug)]
struct Facts {
    /// The tracker's counted steps before and after the frame.
    before: u8,
    stage: u8,
    /// Whether the input handler's gate let the fighter act on this frame.
    gate: bool,
    dir: Option<Dir>,
    /// A button of the move's class is down, and went down on this frame.
    down: bool,
    pressed: bool,
    /// The engine took a punch / a kick on this frame: its latch went up, or an
    /// attack began (not the move tried) on a frame a button of the class went
    /// down or came up.
    took: [bool; 2],
}

impl Facts {
    fn took_any(&self) -> bool {
        self.took[0] || self.took[1]
    }
}

/// A recorded attempt run through a clone of the session.
struct Run {
    session: GameSession,
    inputs: Vec<FrameInputs>,
    facts: Vec<Facts>,
    /// Every distinct word the call-outs showed, in order.
    words: Vec<String>,
    /// The frame (index into the inputs) the fix first showed.
    fix_at: Option<usize>,
    /// The rendered frames when the fix first showed and a little later.
    shots: Vec<Frame>,
}

impl Run {
    fn coach(&self) -> &sf_engine::front::coach::Coach {
        self.session.coach().unwrap()
    }

    fn fix_kind(&self) -> Option<FixKind> {
        self.coach().fix().map(|f| f.kind)
    }

    fn first(&self, f: impl Fn(&Facts) -> bool) -> Option<usize> {
        self.facts.iter().position(f)
    }

    /// The first frame the command was done (step three counted).
    fn first_done(&self) -> Option<usize> {
        self.first(|x| x.stage == 3)
    }

    /// The frame the tracker fell back from step two or three.
    fn fell(&self) -> Option<usize> {
        self.first(|x| x.before >= 2 && x.stage == 0)
    }
}

fn is_attack(mv: u8, entry: &MoveEntry) -> bool {
    (0x0d..=0x2f).contains(&mv) && mv != entry.produces
}

fn play(base: &GameSession, preset: RulesetPreset, row: usize, frames: Vec<FrameInputs>, tail: usize) -> Run {
    let entry = REGISTRY[row];
    let toggles = MatchConfig::from_preset(preset).toggles;
    let Recognizer::Tracker(tracker) = entry.recognizer else { panic!("expected a 1987 tracker") };
    let at = (P1_STRUCT + tracker.offset() - WORK_BASE) as usize;
    let latch = (P1_STRUCT - WORK_BASE) as usize;
    let stage_of = |s: &GameSession| {
        let w = s.fight_machine().unwrap().work_ram();
        if w[at] == 3 && w[at + 2] == 0 { 0 } else { w[at] }
    };
    let latches = |s: &GameSession| {
        let w = s.fight_machine().unwrap().work_ram();
        [w[latch] != 0, w[latch + 1] != 0]
    };
    let my_move = |s: &GameSession| s.fight().unwrap().fight_read().fighters[0].move_id;
    let frames: Vec<FrameInputs> = frames.into_iter().chain(vec![idle(); tail]).collect();
    let mut s = base.clone();
    let mut run = Run { session: s.clone(), inputs: frames.clone(), facts: Vec::new(), words: Vec::new(), fix_at: None, shots: Vec::new() };
    let mut was = [false; 2];
    let mut was_latched = latches(&s);
    let mut was_move = my_move(&s);
    let mut shot_due = None;
    for (n, input) in frames.into_iter().enumerate() {
        let m = s.fight_machine().unwrap();
        let gate = input_gate_free(MachineRead { rom: m.program_rom(), work: m.work_ram(), obj: &m.state.objectram, bank: m.data_bank() }, P1_STRUCT, &toggles);
        let before = stage_of(&s);
        let out = s.step(input);
        let held = [ButtonClass::Punch.down(input.p1.buttons), ButtonClass::Kick.down(input.p1.buttons)];
        let (now_latched, now_move) = (latches(&s), my_move(&s));
        let attack_began = now_move != was_move && is_attack(now_move, &entry);
        let class = usize::from(entry.input.button == ButtonClass::Kick);
        run.facts.push(Facts {
            before,
            stage: stage_of(&s),
            gate,
            dir: Dir::from_stick(input.p1.directions, true),
            down: held[class],
            pressed: held[class] && !was[class],
            took: [0, 1].map(|c| (now_latched[c] && !was_latched[c]) || (attack_began && held[c] != was[c])),
        });
        (was, was_latched, was_move) = (held, now_latched, now_move);
        for w in words(&s) {
            let w = w.word().to_owned();
            if !run.words.contains(&w) {
                run.words.push(w);
            }
        }
        if run.fix_at.is_none() && !fix_words(&s).is_empty() {
            run.fix_at = Some(n);
            shot_due = Some(n + 6);
            run.shots.push(out.frame.clone());
        } else if shot_due == Some(n) {
            run.shots.push(out.frame.clone());
        }
    }
    run.session = s;
    run
}

/// The kinds of recorded attempt, by the fix each is to be given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    TooSlow,
    Missed,
    PressSooner,
    ButtonTooEarly,
    WaitToRecover,
}

impl Kind {
    const ALL: [Kind; 5] = [Kind::TooSlow, Kind::Missed, Kind::PressSooner, Kind::ButtonTooEarly, Kind::WaitToRecover];

    fn slug(self) -> &'static str {
        match self {
            Kind::TooSlow => "too_slow",
            Kind::Missed => "missed",
            Kind::PressSooner => "press_sooner",
            Kind::ButtonTooEarly => "button_too_early",
            Kind::WaitToRecover => "wait_to_recover",
        }
    }
}

/// The button the recorded attempts press: a medium punch or kick; and the
/// other class's.
fn button_of(entry: &MoveEntry) -> u8 {
    match entry.input.button {
        ButtonClass::Kick => MK,
        _ => MP,
    }
}

fn other_button_of(entry: &MoveEntry) -> u8 {
    match entry.input.button {
        ButtonClass::Kick => MP,
        _ => MK,
    }
}

fn motion(entry: &MoveEntry) -> [Option<Dir>; 3] {
    [entry.input.motion[0], entry.input.motion[1], entry.input.motion[2]].map(Some)
}

/// A button pressed and let go `at` frames into the motion's second step, the
/// motion finished afterwards and the stick let go with no button after.
fn early(entry: &MoveEntry, at: usize) -> Vec<Seg> {
    let [m0, m1, m2] = motion(entry);
    vec![(m0, NONE, 3), (m1, NONE, at), (m1, button_of(entry), 2), (m1, NONE, 2), (m2, NONE, 10), (None, NONE, 20)]
}

/// The recorded attempt of `kind` for `entry`. `window` is the frames the
/// trackers give a motion under the preset.
fn recorded(entry: &MoveEntry, kind: Kind, window: usize) -> Vec<Seg> {
    let [m0, m1, m2] = motion(entry);
    let heavy = match entry.input.button {
        ButtonClass::Kick => HK,
        _ => HP,
    };
    match kind {
        // The last step held back until the window has run out, then given with the button.
        Kind::TooSlow => vec![(m0, NONE, 3), (m1, NONE, window + 6), (m2, NONE, 1), (m2, button_of(entry), 2), (m2, NONE, 12), (None, NONE, 20)],
        // The last step never given.
        Kind::Missed => vec![(m0, NONE, 3), (m1, NONE, 3), (None, NONE, 40)],
        // The motion through and let go with no button.
        Kind::PressSooner => vec![(m0, NONE, 3), (m1, NONE, 3), (m2, NONE, 10), (None, NONE, 20)],
        // The button pressed and let go on the second step.
        Kind::ButtonTooEarly => early(entry, 1),
        // A heavy attack, then the motion and the button while it still recovers.
        Kind::WaitToRecover => vec![
            (None, heavy, 2),
            (None, NONE, 6),
            (m0, NONE, 3),
            (m1, NONE, 3),
            (m2, NONE, 1),
            (m2, button_of(entry), 2),
            (m2, NONE, 12),
            (None, NONE, 20),
        ],
    }
}

/// The fix `kind` is expected to give for `entry`.
fn expected(entry: &MoveEntry, kind: Kind) -> FixKind {
    match kind {
        Kind::TooSlow => FixKind::TooSlow,
        Kind::Missed => FixKind::Missed(entry.input.motion[2]),
        Kind::PressSooner => FixKind::PressSooner(entry.input.button),
        Kind::ButtonTooEarly => FixKind::ButtonTooEarly,
        Kind::WaitToRecover => FixKind::WaitToRecover,
    }
}

/// The recorded attempt with the fault the fix names put right, derived from
/// the recorded inputs and what the engine did with them (never from the
/// script that made them): the claim each fix makes.
fn put_right(entry: &MoveEntry, fix: FixKind, run: &Run) -> Vec<FrameInputs> {
    let [m0, _, m2] = motion(entry);
    let (b, f) = (button_of(entry), &run.inputs);
    let press_on = |mut frames: Vec<FrameInputs>, from: usize| {
        // One frame down and up again: inside the command's done window even when
        // the stick is let go a couple of frames after.
        for i in frames.iter_mut().skip(from).take(1) {
            set_bits(i, bits(i) | b);
        }
        frames
    };
    match fix {
        // The same inputs without the wait: the stalled step held a few frames,
        // then the late direction and what came with it.
        FixKind::TooSlow => {
            let s2 = run.first(|x| x.stage == 2).unwrap();
            let late = (run.fell().unwrap()..f.len()).find(|&i| run.facts[i].dir == m2).unwrap();
            [&f[..s2 + 3], &f[late..]].concat()
        }
        // The same inputs up to the stall, then the awaited direction and a button.
        FixKind::Missed(_) => {
            let s2 = run.first(|x| x.stage == 2).unwrap();
            let given = inputs(&[(m2, NONE, 1), (m2, b, 2), (m2, NONE, 12)]);
            [&f[..s2 + 3], &given[..]].concat()
        }
        // The same inputs with a button in time: pressed on the second frame the
        // command was done.
        FixKind::PressSooner(_) => press_on(f.clone(), run.first_done().unwrap() + 1),
        // The same inputs with the button let go while the command was done.
        FixKind::ReleaseSooner(_) => {
            let done = run.first_done().unwrap();
            let mut frames = f.clone();
            for i in frames.iter_mut().skip(done + 1) {
                set_bits(i, bits(i) & !b);
            }
            frames
        }
        // The same inputs with the right class's button.
        FixKind::WrongButton(_) => {
            let mut frames = f.clone();
            for i in frames.iter_mut() {
                let mask = bits(i);
                if mask & other_button_of(entry) != 0 {
                    set_bits(i, mask & !other_button_of(entry) | b);
                }
            }
            frames
        }
        // The same inputs with the early button dropped and pressed after the
        // motion: on the second frame the stick shows the last direction.
        FixKind::ButtonTooEarly => {
            let last = run.first(|x| x.dir == m2).unwrap();
            let mut frames = f.clone();
            for i in frames.iter_mut() {
                set_bits(i, 0);
            }
            press_on(frames, last + 1)
        }
        // The same inputs, after waiting to recover: the motion starts 60 frames later.
        FixKind::WaitToRecover => {
            let start = run.first(|x| x.dir == m0).unwrap();
            [&f[..start], &vec![idle(); 60][..], &f[start..]].concat()
        }
        FixKind::TimeParry => unreachable!("three-direction commands do not parry"),
    }
}

/// The fix's claim, read from what the engine did on the recorded attempt
/// (its trackers, its gate, its latches and moves), not from the coach.
fn claim_holds(entry: &MoveEntry, fix: FixKind, run: &Run) -> Result<(), String> {
    let f = &run.facts;
    let m2 = Some(entry.input.motion[2]);
    let first_done = run.first_done();
    let fell = run.fell();
    // A button the engine took before the motion was done (before it fell back,
    // if it never was).
    let horizon = first_done.or(fell.map(|x| x + 1)).unwrap_or(f.len());
    let took_before_done = f.iter().take(horizon).any(Facts::took_any);
    match fix {
        FixKind::TooSlow => {
            // The awaited direction came once the tracker was gone, the engine did
            // not take it as the motion's last step, and no button had cleared it.
            let fell = fell.ok_or("the tracker never fell back")?;
            let late = (fell..f.len()).find(|&i| f[i].dir == m2).ok_or("the direction never came after the window")?;
            if f[late].stage >= 2 || first_done.is_some() || took_before_done {
                return Err("the engine took the late direction, or a button cleared the tracker".into());
            }
        }
        FixKind::Missed(_) => {
            if f.iter().any(|x| x.dir == m2) || first_done.is_some() || took_before_done {
                return Err("the direction did arrive, the motion was done, or a button was taken".into());
            }
        }
        FixKind::PressSooner(_) => {
            let done = first_done.ok_or("the motion was never done")?;
            let in_done = |x: &&Facts| x.stage == 3;
            if f.iter().skip(done).filter(in_done).any(|x| x.took_any() || (x.down && x.gate)) {
                return Err("a button was taken or held while the command was done".into());
            }
            if !f.iter().skip(done).filter(in_done).any(|x| x.gate) {
                return Err("the gate held on every frame the command was done".into());
            }
        }
        FixKind::ReleaseSooner(_) => {
            // Down on a frame the command was done and the gate open, and the
            // engine did not take it until after.
            let done = first_done.ok_or("the motion was never done")?;
            let held = f.iter().skip(done).any(|x| x.stage == 3 && x.down && x.gate);
            let taken = f.iter().skip(done).filter(|x| x.stage == 3).any(Facts::took_any);
            if !held || taken || took_before_done {
                return Err(format!("held while done: {held}; taken while done: {taken}; before: {took_before_done}"));
            }
        }
        FixKind::WrongButton(_) => {
            let done = first_done.ok_or("the motion was never done")?;
            let class = usize::from(entry.input.button == ButtonClass::Kick);
            // The done window, up to and including the frame the tracker was cleared on.
            let end = f.iter().skip(done).position(|x| x.stage == 0).map_or(f.len(), |p| done + p + 1);
            let window = &f[done..end];
            if !window.iter().any(|x| x.took[1 - class]) || window.iter().any(|x| x.took[class]) {
                return Err("the engine did not take the other class while the command was done".into());
            }
        }
        FixKind::ButtonTooEarly => {
            if !took_before_done {
                return Err("the engine took no button before the motion was done".into());
            }
        }
        FixKind::WaitToRecover => {
            // The command was done and the button pressed on a frame the gate held
            // the fighter (or the gate held it throughout), and the move never came out.
            let done = first_done.ok_or("the motion was never done")?;
            let held = f.iter().skip(done).any(|x| x.stage == 3 && x.pressed && !x.gate);
            let shut = !f.iter().skip(done).any(|x| x.stage == 3 && x.gate);
            if !(held || shut) {
                return Err("no button was pressed on a frame the gate held, and the gate was open".into());
            }
        }
        FixKind::TimeParry => return Err("three-direction commands do not parry".into()),
    }
    Ok(())
}

fn window_of(preset: RulesetPreset) -> usize {
    sf_engine::moves::tracker_window(&MatchConfig::from_preset(preset).toggles) as usize
}

/// Every Fix word the call-outs showed on the run.
fn fix_words_of(run: &Run) -> Vec<String> {
    run.words
        .iter()
        .filter(|w| ["TOO SLOW", "MISSED", "PRESS", "RELEASE", "BUTTON", "WAIT TO"].iter().any(|p| w.starts_with(p)))
        .cloned()
        .collect()
}

/// A recorded attempt run, its fix checked against the expected one, its claim
/// read from the engine, and the attempt put right and put to the engine: the
/// move comes out and nothing is said.
fn check(base: &GameSession, preset: RulesetPreset, row: usize, what: &str, segs: &[Seg], want: FixKind) -> Run {
    let entry = &REGISTRY[row];
    let run = play(base, preset, row, inputs(segs), 80);
    let coach = run.coach();
    assert_eq!(coach.successes(), 0, "{what}: the move did not come out");
    let Some(Outcome::Miss(m)) = coach.outcome() else { panic!("{what}: a miss, not {:?}", coach.outcome()) };
    assert_eq!(run.fix_kind(), Some(want), "{what}: {m:?}");
    let text = coach.fix().unwrap().text(InputNotation::Arrows);
    assert_eq!(fix_words_of(&run), std::slice::from_ref(&text), "{what}: one fix, put through the call-outs once");
    if let Err(why) = claim_holds(entry, want, &run) {
        panic!("{what}: the fix's claim does not hold: {why}\n{:?}", run.facts);
    }
    let right = play(base, preset, row, put_right(entry, want, &run), 80);
    assert_eq!(right.coach().successes(), 1, "{what}: the claim's inputs bring {} out ({:?})", entry.name, right.coach().outcome());
    assert_eq!(right.coach().outcome(), Some(Outcome::Success), "{what}");
    assert!(fix_words_of(&right).is_empty(), "{what}: a move that comes out gets no fix");
    run
}

#[test]
fn each_kind_of_miss_gets_its_fix_and_the_fixs_claim_holds_against_the_engine() {
    if assets().is_none() {
        return;
    }
    let dir = std::env::var("SF_FIX_FRAMES").ok();
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        let window = window_of(preset);
        for (row, entry) in REGISTRY.iter().enumerate().filter(|(_, e)| e.available.fighters.contains(&sf_engine::roster::TournamentFighter::Ryu)) {
            let base = base(preset, row, InputNotation::Arrows);
            for kind in Kind::ALL {
                let what = format!("{preset:?} {} {kind:?}", entry.name);
                let mut segs = recorded(entry, kind, window);
                if kind == Kind::ButtonTooEarly {
                    // The engine takes a button the gate shuts out later (a walk's
                    // held frames in 1987), once the motion is done, so the press
                    // has to come with the gate open: the first offset into the step
                    // whose attempt misses is the recorded one, and any that comes out
                    // is the engine taking a late button, with no fix.
                    let mut found = None;
                    for at in [1, 2, 3, 4, 6, 8] {
                        let tried = play(&base, preset, row, inputs(&early(entry, at)), 80);
                        if tried.coach().successes() == 1 {
                            assert!(fix_words_of(&tried).is_empty(), "{what}: at {at}: it came out, so no fix");
                        } else {
                            found = Some(at);
                            break;
                        }
                    }
                    segs = early(entry, found.unwrap_or_else(|| panic!("{what}: no early press missed")));
                }
                let run = check(&base, preset, row, &what, &segs, expected(entry, kind));
                let m = match run.coach().outcome() {
                    Some(Outcome::Miss(m)) => m,
                    _ => unreachable!(),
                };
                // Where the engine says it stalled is where the fix points.
                match kind {
                    Kind::TooSlow | Kind::Missed => assert_eq!((m.stage, m.awaited), (2, Some(entry.input.motion[2])), "{what}"),
                    Kind::PressSooner | Kind::WaitToRecover => assert_eq!(m.stage, 3, "{what}"),
                    Kind::ButtonTooEarly => assert!(m.stage >= 2, "{what}"),
                }
                if let (Some(dir), 0, RulesetPreset::DefinitiveOverhaul) = (&dir, row, preset) {
                    for (i, shot) in run.shots.iter().enumerate() {
                        write_png(dir, &format!("{}_{}_{i}", entry.name.to_lowercase(), kind.slug()), shot);
                    }
                }
            }
        }
    }
}

/// A fixture from the probe of the review: a Hadoken attempt, the preset it
/// runs under and the fix the engine's own behaviour calls for.
struct Fixture {
    name: &'static str,
    preset: RulesetPreset,
    segs: Vec<Seg>,
    fix: FixKind,
}

#[test]
fn the_reviews_probes_press_and_release_are_read_from_what_the_engine_took() {
    if assets().is_none() {
        return;
    }
    use Dir::*;
    let (d, df, f) = (Some(Down), Some(DownForward), Some(Forward));
    let punch = ButtonClass::Punch;
    let faithful = RulesetPreset::Faithful1987;
    let overhaul = RulesetPreset::DefinitiveOverhaul;
    let fixtures = vec![
        // Faithful 1987 takes the attack on the release: P went down on the second
        // step and was held 14 frames, through the motion, the release coming after
        // the window ran out. Not early (nothing was taken before the motion was
        // done): the release came too late.
        Fixture { name: "P held 14 frames, taken on release", preset: faithful, segs: vec![(d, NONE, 3), (df, MP, 2), (f, MP, 14), (f, NONE, 10), (None, NONE, 20)], fix: FixKind::ReleaseSooner(punch) },
        // The stick let go before the button: P came after the motion was dropped.
        Fixture { name: "stick released before P", preset: faithful, segs: vec![(d, NONE, 3), (df, NONE, 3), (f, NONE, 3), (None, NONE, 2), (None, MP, 3), (None, NONE, 20)], fix: FixKind::PressSooner(punch) },
        // P went down on the second step and came up after the stick did.
        Fixture { name: "P on DF, released after the stick", preset: faithful, segs: vec![(d, NONE, 3), (df, MP, 3), (f, MP, 3), (None, MP, 3), (None, NONE, 20)], fix: FixKind::ReleaseSooner(punch) },
        // The overhaul takes the press, and the release only when a special is
        // ready: P held from before and let go on DF was taken by nobody.
        Fixture { name: "P held from before, released on DF", preset: overhaul, segs: vec![(None, MP, 30), (d, MP, 3), (df, MP, 2), (df, NONE, 1), (f, NONE, 10), (None, NONE, 20)], fix: FixKind::PressSooner(punch) },
        // A kick tapped on the second step clears the tracker; F within the window
        // is not late.
        Fixture { name: "kick tapped, then F within the window", preset: overhaul, segs: vec![(d, NONE, 3), (df, NONE, 1), (df, MK, 2), (df, NONE, 2), (f, NONE, 3), (f, MP, 2), (f, NONE, 10)], fix: FixKind::ButtonTooEarly },
        // A kick on the last step, in time: the engine took the wrong class.
        Fixture { name: "kick in time", preset: overhaul, segs: vec![(d, NONE, 3), (df, NONE, 3), (f, NONE, 1), (f, MK, 2), (f, NONE, 10), (None, NONE, 20)], fix: FixKind::WrongButton(punch) },
        Fixture { name: "kick in time", preset: faithful, segs: vec![(d, NONE, 3), (df, NONE, 3), (f, NONE, 1), (f, MK, 2), (f, NONE, 10), (None, NONE, 20)], fix: FixKind::WrongButton(punch) },
        // A punch on the very frame of the second direction clears the tracker
        // before step two is ever seen.
        Fixture { name: "P on the frame of DF", preset: overhaul, segs: vec![(d, NONE, 3), (df, MP, 2), (df, NONE, 2), (f, NONE, 10), (None, NONE, 20)], fix: FixKind::ButtonTooEarly },
    ];
    for fixture in fixtures {
        let base = base(fixture.preset, 0, InputNotation::Arrows);
        let what = format!("{:?}: {}", fixture.preset, fixture.name);
        check(&base, fixture.preset, 0, &what, &fixture.segs, fixture.fix);
    }
}

#[test]
fn a_held_button_let_go_in_time_is_a_move_and_no_fix() {
    if assets().is_none() {
        return;
    }
    // Release timing, the other way: press and hold through the motion and let
    // go while the command is done. The 1987 engine takes the release and the
    // Hadoken comes out; the overhaul takes the press, early, and it does not.
    use Dir::*;
    let (d, df, f) = (Some(Down), Some(DownForward), Some(Forward));
    let segs = vec![(d, NONE, 3), (df, MP, 2), (f, MP, 3), (f, NONE, 10), (None, NONE, 20)];
    let faithful = RulesetPreset::Faithful1987;
    let base_f = base(faithful, 0, InputNotation::Arrows);
    let run = play(&base_f, faithful, 0, inputs(&segs), 80);
    assert_eq!(run.coach().successes(), 1, "taken on the release while done");
    assert!(fix_words_of(&run).is_empty());
    // The same inputs held and let go a few frames before the third step: the
    // engine takes the release early, and the fix says so.
    let overhaul = RulesetPreset::DefinitiveOverhaul;
    let base_o = base(overhaul, 0, InputNotation::Arrows);
    check(&base_o, overhaul, 0, "overhaul, pressed on the second step", &segs, FixKind::ButtonTooEarly);
}

#[test]
fn the_wait_to_recover_gate_held_on_the_button_and_waiting_brings_the_move_out_under_every_preset() {
    if assets().is_none() {
        return;
    }
    // WAIT TO RECOVER needs the fighter held when the button comes. A preset
    // with a recovery input buffer may carry such a button through; where the
    // gate held and the move did not come out, the fix says to wait, and where
    // the buffer rescued it there is no miss at all.
    for preset in RulesetPreset::ALL {
        let window = window_of(preset);
        let entry = &REGISTRY[0];
        let base = base(preset, 0, InputNotation::Arrows);
        let segs = recorded(entry, Kind::WaitToRecover, window);
        let run = play(&base, preset, 0, inputs(&segs), 80);
        if run.coach().successes() == 0 {
            assert_eq!(run.fix_kind(), Some(FixKind::WaitToRecover), "{preset:?}");
            claim_holds(entry, FixKind::WaitToRecover, &run).unwrap();
            let right = play(&base, preset, 0, put_right(entry, FixKind::WaitToRecover, &run), 80);
            assert_eq!(right.coach().successes(), 1, "{preset:?}: after recovering it comes out");
        } else {
            assert!(fix_words_of(&run).is_empty(), "{preset:?}: the buffer brought it out: no miss, no fix");
        }
    }
}

#[test]
fn the_fix_is_written_in_the_chosen_notation_and_drawn_in_the_column() {
    let Some(a) = assets() else { return };
    // The Shoryuken (F, D, DF): stopped at step two, the one missed is DF.
    for (notation, text) in [
        (InputNotation::Arrows, "MISSED \u{2198}"),
        (InputNotation::Numpad, "MISSED 3"),
        (InputNotation::Words, "MISSED DF"),
    ] {
        for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
            let base = base(preset, 1, notation);
            let miss = recorded(&REGISTRY[1], Kind::Missed, window_of(preset));
            let run = play(&base, preset, 1, inputs(&miss), 80);
            assert_eq!(run.fix_kind(), Some(FixKind::Missed(Dir::DownForward)), "{preset:?} {notation:?}");
            assert_eq!(fix_words_of(&run), [text], "{preset:?} {notation:?}");
            // And it is on the screen, from the column's edge in its slot, in gold.
            let shot = &run.shots[1];
            if let Ok(dir) = std::env::var("SF_FIX_FRAMES") {
                write_png(&dir, &format!("shoryuken_missed_{notation:?}_{preset:?}").to_lowercase(), shot);
            }
            let slot = regions::word_slot_at(Side::P1, 0, regions::TRY_DROP);
            assert!(lettered(shot, &a, slot.x, slot.y, text, sf_video::callouts::LETTER_PENS), "{preset:?} {notation:?}: {text} is lettered");
            if notation == InputNotation::Arrows {
                // The arrow is a made glyph, inked in its cell.
                let cell = slot.x + 8 * "MISSED ".chars().count();
                assert!(inked(shot, cell, slot.y, sf_video::callouts::LETTER_PENS[2]), "{preset:?}: the arrow is drawn");
            }
        }
    }
    // A fighter who has crossed over writes the arrow the other way.
    let fix = sf_engine::front::coach::Fix { kind: FixKind::Missed(Dir::DownForward), facing_right: false };
    assert_eq!(fix.text(InputNotation::Arrows), "MISSED \u{2199}");
    assert_eq!(fix.text(InputNotation::Numpad), "MISSED 3", "numpad is always written facing right");
}

#[test]
fn the_players_last_inputs_stay_under_the_input_to_do_for_a_few_seconds() {
    let Some(a) = assets() else { return };
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        // Hadoken: D, DF, and the stick let go.
        let base = base(preset, 0, InputNotation::Arrows);
        let miss = recorded(&REGISTRY[0], Kind::Missed, window_of(preset));
        let mut s = base.clone();
        let mut seen = Vec::new();
        for (n, input) in inputs(&miss).into_iter().chain(vec![idle(); 400]).enumerate() {
            let out = s.step(input);
            let shown = s.coach().unwrap().shown_inputs().map(<[String]>::to_vec);
            if let Some(labels) = &shown {
                seen.push(n);
                assert_eq!(labels, &["D", "DF"], "{preset:?}: what the player did, a step a label");
                // The line is drawn under the strip, centred, in the whitest pen,
                // and the input to do stays above it.
                let line = regions::STRIP_INPUTS;
                let x = line.x + 8 * (regions::STRIP_LETTERS - "D DF".len()) / 2;
                assert!(lettered(&out.frame, &a, x, line.y, "D DF", STRIP_INPUTS_PENS), "{preset:?} frame {n}: the player's inputs are drawn");
                let steps = REGISTRY[0].input.strip_steps();
                assert_eq!(s.coach().unwrap().strip(), sf_video::callouts::Strip::Input { steps, lit: 0 });
            } else if !seen.is_empty() {
                // Gone for good once its time is up: no pixel of its pen is left.
                let line = regions::STRIP_INPUTS;
                let any = (line.y..line.y + 8).any(|y| (line.x..line.x + line.w).any(|x| out.frame.argb[y * WIDTH + x] == STRIP_INPUTS_PENS[2]));
                assert!(!any, "{preset:?} frame {n}: the line is gone");
            }
        }
        let (first, last) = (seen[0], *seen.last().unwrap());
        assert_eq!(last - first + 1, INPUTS_FRAMES as usize, "{preset:?}: held for {INPUTS_FRAMES} frames, four seconds");
        assert_eq!(seen.len(), INPUTS_FRAMES as usize, "{preset:?}: with no gap");

        // Started again: a new attempt takes the line away at once.
        let mut s = base.clone();
        for input in inputs(&miss).into_iter().chain(vec![idle(); 60]) {
            s.step(input);
        }
        assert!(s.coach().unwrap().shown_inputs().is_some());
        for input in inputs(&[(Some(Dir::Down), NONE, 3), (Some(Dir::DownForward), NONE, 3)]) {
            s.step(input);
        }
        assert!(s.coach().unwrap().attempting());
        assert!(s.coach().unwrap().shown_inputs().is_none(), "{preset:?}: a new attempt clears the old inputs");
        // And a move that comes out clears it too.
        let mut s = base.clone();
        for input in inputs(&miss).into_iter().chain(vec![idle(); 60]) {
            s.step(input);
        }
        let (d, df, f) = (Some(Dir::Down), Some(Dir::DownForward), Some(Dir::Forward));
        for input in inputs(&[(d, NONE, 3), (df, NONE, 3), (f, NONE, 1), (f, MP, 2), (f, NONE, 12)]) {
            s.step(input);
        }
        assert_eq!(s.coach().unwrap().successes(), 1);
        assert!(s.coach().unwrap().shown_inputs().is_none(), "{preset:?}");
    }
}

#[test]
fn a_fix_waits_to_see_whether_the_direction_still_arrives_and_a_new_attempt_ends_the_wait() {
    if assets().is_none() {
        return;
    }
    // The wait is the window the tracker runs: the ROM's under 1987, the
    // configured one under the overhaul's lenient trackers.
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
    let base = base(preset, 0, InputNotation::Arrows);
    let miss = recorded(&REGISTRY[0], Kind::Missed, window_of(preset));
    // MISSED is said once the frames the direction could still arrive in have
    // gone by (a window's length after the tracker fell back, the configured
    // one under the overhaul presets), not before.
    let run = play(&base, preset, 0, inputs(&miss), 80);
    let fell = run.first(|x| x.before == 2 && x.stage == 0).unwrap();
    let said = run.fix_at.unwrap();
    assert!(said >= fell + window_of(preset), "MISSED came at {said}, the tracker fell at {fell}");
    assert!(said <= fell + window_of(preset) + 2, "and not much later: {said} vs {fell}");
    // The direction arriving late is TOO SLOW at once, not after the wait.
    let slow = recorded(&REGISTRY[0], Kind::TooSlow, window_of(preset));
    let run = play(&base, preset, 0, inputs(&slow), 80);
    let fell = run.first(|x| x.before == 2 && x.stage == 0).unwrap();
    let late = run.first(|x| x.dir == Some(Dir::Forward) && x.before == 0 && x.stage == 0).unwrap();
    assert!(late >= fell && run.fix_at.unwrap() <= late + 1, "TOO SLOW the frame the late direction came: {late}, said at {:?}", run.fix_at);
    // A player who starts again before the wait is over is told what there is.
    let mut s = base.clone();
    let seq = [(Some(Dir::Down), NONE, 3), (Some(Dir::DownForward), NONE, 3), (None, NONE, 30), (Some(Dir::Down), NONE, 3), (Some(Dir::DownForward), NONE, 3)];
    for input in inputs(&seq) {
        s.step(input);
    }
    assert_eq!(s.coach().unwrap().attempts(), 2);
    assert_eq!(s.coach().unwrap().fix().map(|f| f.kind), Some(FixKind::Missed(Dir::Forward)), "the first miss is worded as the second begins");
    assert_eq!(fix_words(&s), ["MISSED \u{2192}"]);
    }
}

#[test]
fn other_and_interrupted_attempts_get_no_fix() {
    if assets().is_none() {
        return;
    }
    // The Hadoken tried and the Shoryuken's input given: the game took an input.
    let preset = RulesetPreset::Faithful1987;
    let base0 = base(preset, 0, InputNotation::Arrows);
    let shoryuken = REGISTRY[1].input.frames(true, sf_engine::moves::Strength::Medium);
    let frames: Vec<FrameInputs> = shoryuken.into_iter().map(|p| with(|i| i.p1 = p)).collect();
    let run = play(&base0, preset, 0, frames, 120);
    assert_eq!(run.coach().outcome(), Some(Outcome::Other(0x19)));
    assert!(run.coach().fix().is_none() && fix_words_of(&run).is_empty() && run.coach().shown_inputs().is_none());
    // A hit mid-motion ends the attempt as nothing, and nothing is said.
    let mut s = base0.clone();
    for input in inputs(&[(Some(Dir::Down), NONE, 3), (Some(Dir::DownForward), NONE, 3)]) {
        s.step(input);
    }
    let mut hit = *s.fight().unwrap().fight_read();
    hit.fighters[0] = sf_engine::fight_reader::FighterRead { move_id: 0x30, state: 0x82, ..hit.fighters[0] };
    s.feed_fight_reads(&[hit]);
    s.step(with(|i| i.p1.directions = stick_of(Some(Dir::DownForward))));
    assert_eq!(s.coach().unwrap().outcome(), Some(Outcome::Interrupted));
    for _ in 0..120 {
        s.step(idle());
        assert!(fix_words(&s).is_empty());
    }
    assert!(s.coach().unwrap().fix().is_none() && s.coach().unwrap().shown_inputs().is_none());
}

#[test]
fn a_miss_and_its_fix_leave_the_machine_as_it_was() {
    if assets().is_none() {
        return;
    }
    // The same fight with the same inputs, with a Try watching and with none:
    // the same machine bit for bit, fixes and all (ADR-0004).
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        let base = training(preset, InputNotation::Arrows);
        let mut watched = base.clone();
        try_move(&mut watched, 0);
        let mut plain = base;
        assert_eq!(plain.step(start()).transition, Some(Transition::Paused));
        plain.step(idle());
        tap(&mut plain, confirm());
        let miss = early(&REGISTRY[0], 1);
        for (n, input) in inputs(&miss).into_iter().chain(vec![idle(); 120]).enumerate() {
            watched.step(input);
            plain.step(input);
            let (a, b) = (watched.fight_machine().unwrap(), plain.fight_machine().unwrap());
            assert_eq!(a.work_ram(), b.work_ram(), "{preset:?}: frame {n}");
            assert_eq!(a.state.objectram, b.state.objectram, "{preset:?}: frame {n}");
        }
        assert!(watched.coach().unwrap().fix().is_some(), "{preset:?}: a fix was chosen");
        assert_eq!(watched.fight_machine().unwrap().save_snapshot(), plain.fight_machine().unwrap().save_snapshot());
    }
}

#[test]
fn a_fix_and_the_players_inputs_keep_clear_of_the_call_outs_the_strip_and_the_era_hud() {
    if assets().is_none() {
        return;
    }
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        // The era gauges drawn, as in a fight that has them.
        let hud = sf_video::compositor::EraHud {
            p1: Some(sf_video::era_hud::EraGauge::one(sf_video::era_hud::GaugeRow::level(2, 3, sf_video::era_hud::Fill::of(2, 3)))),
            p2: Some(sf_video::era_hud::EraGauge::one(sf_video::era_hud::GaugeRow::level(1, 3, sf_video::era_hud::Fill::of(1, 3)))),
        };
        let mut start_of = training(preset, InputNotation::Arrows);
        start_of.set_era_hud(hud);
        // One fight with the Try, the same fight (the same inputs) with none.
        let mut watched = start_of.clone();
        try_move(&mut watched, 0);
        let mut plain = start_of;
        assert_eq!(plain.step(start()).transition, Some(Transition::Paused));
        plain.step(idle());
        tap(&mut plain, confirm());
        let (strip, line) = (regions::STRIP, regions::STRIP_INPUTS);
        // The Try drops the columns; the same fight without it keeps them where
        // they are, so the column is where either sits.
        let column = regions::callout_column_at(Side::P1, regions::TRY_DROP);
        let usual = regions::callout_column(Side::P1);
        let (mut line_drawn, mut column_drawn) = (false, false);
        // A button too early, so the fix and the player's inputs are up together.
        for (n, input) in inputs(&early(&REGISTRY[0], 1)).into_iter().chain(vec![idle(); 100]).enumerate() {
            let frame = watched.step(input).frame;
            let other = plain.step(input).frame;
            let shown = watched.fight().unwrap().callouts().frame();
            // Every call-out element (the fix among them) is inside its own column and
            // clear of every other region: the health block with TIME, the other
            // side's column, the gauge corners and the strip's two lines.
            for element in shown.elements() {
                for (name, rect) in regions::all_try() {
                    let own = matches!(name, regions::Region::CalloutColumn(side) if side == element.side);
                    assert!(own || !element.rect.intersects(&rect), "{preset:?} frame {n}: {element:?} meets {name:?}");
                }
                assert!(regions::callout_column_at(element.side, regions::TRY_DROP).holds(&element.rect), "{preset:?} frame {n}: {element:?} inside its column");
            }
            // The Try, set against the same frame without it, changes the strip's
            // two lines and P1's column and no other pixel: not the era HUD, not
            // the health block, not the fight.
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    if frame.argb[y * WIDTH + x] == other.argb[y * WIDTH + x] {
                        continue;
                    }
                    assert!(
                        strip.contains(x, y) || line.contains(x, y) || column.contains(x, y) || usual.contains(x, y),
                        "{preset:?} frame {n}: pixel ({x}, {y}) differs outside the strip, its line and the column"
                    );
                    line_drawn |= line.contains(x, y);
                    column_drawn |= column.contains(x, y);
                }
            }
        }
        assert!(line_drawn && column_drawn, "{preset:?}: the player's inputs and the fix were drawn");
    }
}

#[test]
fn the_same_attempt_gives_the_same_fix_on_the_same_frame() {
    if assets().is_none() {
        return;
    }
    // The fix is a function of the engine's facts and the player's inputs and
    // nothing else: the same attempt on two Tries is worded on the same frame.
    let preset = RulesetPreset::DefinitiveOverhaul;
    let base = base(preset, 2, InputNotation::Words);
    let miss = early(&REGISTRY[2], 1);
    let a = play(&base, preset, 2, inputs(&miss), 80);
    let b = play(&base, preset, 2, inputs(&miss), 80);
    assert_eq!((a.fix_at, a.coach().fix()), (b.fix_at, b.coach().fix()));
    assert_eq!(a.coach().fix().map(|f| f.kind), Some(FixKind::ButtonTooEarly));
}



#[test]
fn the_call_out_columns_drop_only_while_a_try_runs() {
    if assets().is_none() {
        return;
    }
    // The era HUD's approved layout holds in every fight but a Training Try:
    // the columns sit at their usual top outside one, ten pixels lower during
    // one, and go back when the pause box ends it.
    for preset in [RulesetPreset::Faithful1987, RulesetPreset::DefinitiveOverhaul] {
        let mut s = training(preset, InputNotation::Arrows);
        for _ in 0..4 {
            s.step(idle());
        }
        assert_eq!(s.fight().unwrap().callouts().frame().drop, 0, "{preset:?}: Training with no Try");
        try_move(&mut s, 0);
        for _ in 0..4 {
            s.step(idle());
        }
        assert_eq!(s.fight().unwrap().callouts().frame().drop, regions::TRY_DROP, "{preset:?}: during a Try");
        assert_eq!(regions::callout_column_at(Side::P1, regions::TRY_DROP).y, 82);
        // The pause box ends the Try: the columns are back where they were.
        assert_eq!(s.step(start()).transition, Some(Transition::Paused));
        s.step(idle());
        tap(&mut s, confirm());
        for _ in 0..4 {
            s.step(idle());
        }
        assert!(s.coach().is_none());
        assert_eq!(s.fight().unwrap().callouts().frame().drop, 0, "{preset:?}: after the Try");
        assert_eq!(regions::callout_column(Side::P1).y, 72);
    }
}
