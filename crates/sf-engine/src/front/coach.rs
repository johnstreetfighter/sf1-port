//! Try and the coach (ticket #231, ML5; spec #226, ADR-0012): in Training
//! the player attempts a move from the move list while the coach watches
//! the engine.
//!
//! The coach reads, it never decides by itself: each stepped frame it is
//! handed the move's recognizer view ([`crate::moves::RecognizerView`], the
//! stage byte of the move's own 1987 tracker, its time left, the directions
//! awaited, the busy gate and the current move) and the fight reader's read
//! of the fight, and it writes nothing (ADR-0004). From them it keeps the
//! strip, the attempts and their outcomes:
//!
//! - **The strip** shows the input to do, each step lit as the engine has
//!   counted it: the tracker's stage, frame by frame.
//! - **An attempt** starts when the tried move's own tracker passes its
//!   first step ([`ATTEMPT_STAGE`]): the stage rising to two. Other
//!   trackers, and a first step alone (a crouch, a walk), never count. A
//!   tracker already past the first step when the Try starts, or when a hit
//!   ended the last attempt, has to fall back before it can start another.
//! - **Success**: the move comes out on the player's side (the fighter
//!   enters the move the entry produces) after the tracker had counted two
//!   steps or more, or while an attempt is open or settling. The strip
//!   starts again.
//! - **Interrupted**: the player's fighter is in a hit reaction, a hit or
//!   chip landed on it, or it guards against an attacking opponent, while an
//!   attempt is open. It counts as nothing: no call-out, no fix. Holding
//!   back in the guard posture alone, with nothing coming, is not a block:
//!   a Tatsumaki's motion passes through down-back and back.
//! - **Miss**: the tracker falls back with no move out. A move may still
//!   come out after the tracker is gone (the recovery input buffer keeps a
//!   button through a recovery), so the miss is held open for the buffer's
//!   window and becomes a success if the move comes out meanwhile
//!   (`settle`). What the engine knew is kept in [`Miss`].
//! - **Other**: a different move of the registry came out instead.
//!
//! **The fix** (ticket #232, ML6; ADR-0012): after a miss the coach words
//! what went wrong. The engine decides that the attempt failed and where
//! ([`Miss`]: the stage it stalled at, the direction the ROM's table awaited
//! there and the ones the engine took, whether the window ran out or the
//! tracker was cleared with time left, and the busy gate); the player's own
//! inputs, recorded a frame at a time with the engine's reading of that frame
//! (the tracker's stage, the gate, and whether the engine *took* a punch or a
//! kick: its latch went up, or an attack began as a button moved), only choose
//! the words ([`FixKind`]). Which frame the engine takes a button on (the
//! press in the overhaul, the release in 1987) is read from the engine, never
//! guessed from the pad:
//!
//! - stalled, the tracker cleared with time left: a button the engine took
//!   before the motion was done is **BUTTON TOO EARLY**, otherwise **MISSED**;
//! - stalled, the window ran out: a direction the engine takes arriving after
//!   is **TOO SLOW**, none arriving is **MISSED** and the direction awaited;
//! - the motion done: a button taken before it was done is **BUTTON TOO
//!   EARLY**; taken (or pressed) while the gate held the fighter, or the gate
//!   shut throughout, **WAIT TO RECOVER**; the other class taken **PRESS P NOT
//!   K**; the right one down and not taken, its release too late, **RELEASE P
//!   SOONER**; otherwise **PRESS P SOONER**.
//!
//! A move of another entry coming out ([`Outcome::Other`]) is no miss: the
//! game took an input, so there is no fix. A stalled motion's words are
//! chosen once the frames its direction could still arrive in have passed (the
//! tracker's window again, [`RecognizerView::window`]), or when the player
//! starts another attempt; the player's last inputs are kept for the strip's
//! second line ([`Coach::shown_inputs`]) for [`INPUTS_FRAMES`].

use std::collections::VecDeque;

use sf_video::callouts::Strip;

use crate::fight_reader::FightRead;
use crate::front::PlayerInputs;
use crate::moves::{ButtonClass, Dir, InputNotation, MoveEntry, RecognizerView, REGISTRY};

/// The stage a tracker has to reach for an attempt to start: the first two
/// steps of the motion are through.
pub const ATTEMPT_STAGE: u8 = 2;

/// Frames the player's last inputs stay on the strip after a miss (four
/// seconds at 60 Hz).
pub const INPUTS_FRAMES: u32 = 240;

/// Frames before the motion's first direction that a button's press still
/// counts as too early for it (a button pressed with the first direction,
/// or a hair before it).
pub const EARLY_LOOKBACK: u64 = 4;

/// The frames of the player's inputs the coach keeps.
const HISTORY_FRAMES: usize = 600;

/// How an attempt's tracker came to fall back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// The window ran out (no time left on the last frame it was counted).
    WindowRanOut,
    /// It was reset with time left: the last direction let go once the
    /// command was done, or a block or a hit cleared it.
    Released,
}

/// What the engine knew when an attempt ended without the move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Miss {
    /// The furthest stage the tracker reached.
    pub stage: u8,
    /// The direction the ROM's table awaited at that stage (none once the
    /// motion was complete: the button was awaited).
    pub awaited: Option<Dir>,
    /// Every direction the engine took at that stage (the lenient trackers
    /// take more than the table's).
    pub accepted: [Option<Dir>; 2],
    /// Frames the tracker counted, from the first direction.
    pub frames: u32,
    /// Frames the command was done and the busy gate held the fighter.
    pub done_gate_closed: u32,
    /// The window's time left on the last frame it was counted.
    pub time_left: u8,
    /// How the tracker fell back.
    pub ended: Ended,
    /// The busy gate and the fighter's move on the frame the miss was
    /// settled.
    pub gate_free: bool,
    pub current_move: u8,
}

/// How an attempt ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The move came out on the player's side.
    Success,
    /// The attempt ended without the move.
    Miss(Miss),
    /// Another move of the registry came out instead (its engine move).
    Other(u8),
    /// The fighter was hit or blocked: neither a success nor a miss.
    Interrupted,
}

/// What went wrong with a missed attempt, as the coach words it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixKind {
    /// The awaited direction arrived after the window ran out.
    TooSlow,
    /// The awaited direction never arrived (the direction the ROM's table
    /// awaited at the stage the motion stalled).
    Missed(Dir),
    /// The motion was done and no button of this class was taken while it
    /// was (it came after, or not at all).
    PressSooner(ButtonClass),
    /// The motion was done and a button of this class was down, but the
    /// engine takes it on the release (the 1987 engine always does; the
    /// overhaul on a special that is ready), which came after the motion was
    /// let go.
    ReleaseSooner(ButtonClass),
    /// The motion was done and the engine took a button of the other class
    /// (the class named is the one the move needs).
    WrongButton(ButtonClass),
    /// The button came before the motion was done.
    ButtonTooEarly,
    /// Motion and button were right, but the busy gate held the fighter.
    WaitToRecover,
}

/// The coach's fix: what to say, and the way the fighter faced when it was
/// chosen (so an arrow points the way the stick goes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fix {
    pub kind: FixKind,
    pub facing_right: bool,
}

impl Fix {
    /// The fix's words in `notation`: `TOO SLOW`, `MISSED ↘` (or `MISSED 3`,
    /// `MISSED DF`), `PRESS P SOONER`, `BUTTON TOO EARLY`, `WAIT TO RECOVER`.
    pub fn text(&self, notation: InputNotation) -> String {
        match self.kind {
            FixKind::TooSlow => "TOO SLOW".to_owned(),
            FixKind::Missed(dir) => format!("MISSED {}", dir.written(notation, self.facing_right)),
            FixKind::PressSooner(class) => format!("PRESS {} SOONER", class.letter()),
            FixKind::ReleaseSooner(class) => format!("RELEASE {} SOONER", class.letter()),
            FixKind::WrongButton(class) => format!("PRESS {} NOT {}", class.letter(), class.other().letter()),
            FixKind::ButtonTooEarly => "BUTTON TOO EARLY".to_owned(),
            FixKind::WaitToRecover => "WAIT TO RECOVER".to_owned(),
        }
    }
}

/// An attempt in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Attempt {
    reached: u8,
    awaited: Option<Dir>,
    accepted: [Option<Dir>; 2],
    done_gate_closed: u32,
    other: Option<u8>,
}

/// Where a miss sits in the recorded frames: the frame the tracker fell
/// back, and the frame of its first direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    fall: u64,
    first: u64,
}

/// A miss held open while the recovery buffer could still bring the move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Settling {
    miss: Miss,
    span: Span,
    left: u32,
}

/// A miss whose fix is not chosen yet: a stalled motion waits to see whether
/// the awaited direction still arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pending {
    miss: Miss,
    span: Span,
}

/// One frame of the player's inputs, with the engine's reading of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Frame {
    no: u64,
    /// The stick, facing-relative, and which way the fighter faced.
    dir: Option<Dir>,
    facing_right: bool,
    /// Buttons of each class down, and newly pressed this frame.
    punch: bool,
    kick: bool,
    punch_pressed: bool,
    kick_pressed: bool,
    /// The engine took a punch or a kick on this frame: its latch went up (a
    /// button taken while the gate held the fighter), or the fighter entered
    /// an attack on a frame a button of the class went down or came up (a
    /// button taken with the gate open starts its attack at once). The
    /// overhaul takes an attack on the press, the 1987 engine on the release;
    /// the engine says which, not the pad.
    took_punch: bool,
    took_kick: bool,
    /// The steps the tracker had counted once the frame was stepped.
    counted: u8,
    /// Whether the busy gate let the fighter act on this frame's input (the
    /// gate as the frame before left it).
    gate_open: bool,
}

impl Frame {
    fn down(&self, class: ButtonClass) -> bool {
        match class {
            ButtonClass::Punch => self.punch,
            ButtonClass::Kick => self.kick,
        }
    }

    fn pressed(&self, class: ButtonClass) -> bool {
        match class {
            ButtonClass::Punch => self.punch_pressed,
            ButtonClass::Kick => self.kick_pressed,
        }
    }

    /// The engine took a button of `class` on this frame.
    fn took(&self, class: ButtonClass) -> bool {
        match class {
            ButtonClass::Punch => self.took_punch,
            ButtonClass::Kick => self.took_kick,
        }
    }

    /// The engine took a button of either class on this frame.
    fn took_any(&self) -> bool {
        self.took_punch || self.took_kick
    }
}

/// A Try in progress: the move being tried and what the coach has seen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coach {
    entry: MoveEntry,
    /// Steps the engine has counted now (the tracker's stage).
    counted: u8,
    /// The stage and the move on the frame before.
    last_stage: u8,
    last_move: Option<u8>,
    /// Frames the tracker has counted since its first direction, and the
    /// time its window had left on the last of them.
    since_first: u32,
    last_time_left: u8,
    attempt: Option<Attempt>,
    settling: Option<Settling>,
    outcome: Option<Outcome>,
    attempts: u32,
    successes: u32,
    /// The player's inputs this frame, handed in by [`Coach::input`], and
    /// the frames recorded so far.
    pad: Option<(PlayerInputs, bool)>,
    frame_no: u64,
    history: VecDeque<Frame>,
    last_gate_open: bool,
    last_buttons: (bool, bool),
    last_latched: [bool; 2],
    /// The window the tracker runs, as the view last said, and the direction
    /// it awaited on the frame before.
    window: u64,
    last_awaiting: Option<Dir>,
    /// A miss waiting for its words, the last fix chosen (and whether the
    /// fight has taken it yet) and the player's inputs held on the strip.
    pending: Option<Pending>,
    fix: Option<Fix>,
    fix_fresh: bool,
    shown: Option<(Vec<String>, u32)>,
}

impl Coach {
    /// A try of `entry`.
    pub fn new(entry: MoveEntry) -> Self {
        Coach {
            entry,
            counted: 0,
            last_stage: 0,
            last_move: None,
            since_first: 0,
            last_time_left: 0,
            attempt: None,
            settling: None,
            outcome: None,
            attempts: 0,
            successes: 0,
            pad: None,
            frame_no: 0,
            history: VecDeque::new(),
            last_gate_open: true,
            last_buttons: (false, false),
            last_latched: [false; 2],
            window: 0x12,
            last_awaiting: None,
            pending: None,
            fix: None,
            fix_fresh: false,
            shown: None,
        }
    }

    pub fn entry(&self) -> &MoveEntry {
        &self.entry
    }

    /// An attempt is open (or a miss is settling).
    pub fn attempting(&self) -> bool {
        self.attempt.is_some() || self.settling.is_some()
    }

    /// How the last attempt ended.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Attempts begun, and the moves that came out.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn successes(&self) -> u32 {
        self.successes
    }

    /// The fix the coach chose for the last miss, once it has one (a stalled
    /// motion waits up to a window's length to see whether its direction still
    /// arrives).
    pub fn fix(&self) -> Option<Fix> {
        self.fix
    }

    /// A fix chosen since the last call: the fight puts it through the
    /// call-outs once.
    pub fn take_fix(&mut self) -> Option<Fix> {
        std::mem::take(&mut self.fix_fresh).then_some(self.fix).flatten()
    }

    /// A miss is waiting for its words.
    pub fn deciding(&self) -> bool {
        self.pending.is_some()
    }

    /// The player's last inputs, one label to a step as the strip's (`D`,
    /// `DF`, `F`, `P`), while they are held on the strip's second line after
    /// a miss.
    pub fn shown_inputs(&self) -> Option<&[String]> {
        self.shown.as_ref().map(|(labels, _)| labels.as_slice())
    }

    /// The steps of the input lit now: the tracker's stage, as far as the
    /// motion goes (the button's step is not a tracker's).
    pub fn lit(&self) -> usize {
        usize::from(self.counted).min(self.entry.input.motion.len())
    }

    /// What the strip draws now: the input to do, its counted steps lit.
    pub fn strip(&self) -> Strip {
        Strip::Input { steps: self.entry.input.strip_steps(), lit: self.lit() }
    }

    /// The player's inputs for the frame about to be observed, and whether
    /// the fighter faces right: the fight hands them in before
    /// [`Coach::observe`], and the words of a fix are chosen from them.
    pub fn input(&mut self, inputs: PlayerInputs, facing_right: bool) {
        self.pad = Some((inputs, facing_right));
    }

    /// Records the frame just stepped: the player's inputs and the engine's
    /// reading of it. `entered_attack` is the fighter entering an attack that
    /// is not the move being tried on this frame.
    fn record(&mut self, stage: u8, view: &RecognizerView, entered_attack: bool) {
        self.frame_no += 1;
        self.window = u64::from(view.window);
        let (inputs, facing_right) = self.pad.take().unwrap_or((PlayerInputs::default(), true));
        let (punch, kick) = (ButtonClass::Punch.down(inputs.buttons), ButtonClass::Kick.down(inputs.buttons));
        let (punch_edge, kick_edge) = (punch != self.last_buttons.0, kick != self.last_buttons.1);
        // The engine's word that it took a button: a latch that went up, or an
        // attack begun on a frame the class's button moved (the press or the
        // release; which one the engine takes is its own business).
        let took_punch = (view.latched[0] && !self.last_latched[0]) || (entered_attack && punch_edge);
        let took_kick = (view.latched[1] && !self.last_latched[1]) || (entered_attack && kick_edge);
        self.history.push_back(Frame {
            no: self.frame_no,
            dir: Dir::from_stick(inputs.directions, facing_right),
            facing_right,
            punch,
            kick,
            punch_pressed: punch && !self.last_buttons.0,
            kick_pressed: kick && !self.last_buttons.1,
            took_punch,
            took_kick,
            counted: stage,
            gate_open: self.last_gate_open,
        });
        if self.history.len() > HISTORY_FRAMES {
            self.history.pop_front();
        }
        self.last_buttons = (punch, kick);
        self.last_latched = view.latched;
        self.last_gate_open = view.gate_free;
    }

    /// One stepped frame's observation: the entry's recognizer `view` for
    /// the player's fighter, the fight reader's `read` of both fighters,
    /// whether the fight loop is running, and how many frames a miss is held
    /// open (`settle`: the recovery buffer's window plus one, or 0 without
    /// one). Returns how an attempt ended on this frame, if one did.
    pub fn observe(&mut self, view: &RecognizerView, read: &FightRead, in_fight: bool, settle: u32) -> Option<Outcome> {
        let [me, foe] = read.fighters;
        let complete = self.entry.input.motion.len() as u8;
        let stage = view.state.counted_for(complete);
        if !in_fight || !me.present {
            // Between rounds, or before a fighter stands: nothing is
            // attempted, and the tracker's old progress is not a new one.
            self.attempt = None;
            self.settling = None;
            self.pending = None;
            self.shown = None;
            self.history.clear();
            self.pad = None;
            self.last_gate_open = view.gate_free;
            self.counted = 0;
            self.last_stage = stage;
            self.last_move = None;
            self.since_first = 0;
            return None;
        }
        if let Some((_, left)) = &mut self.shown {
            *left -= 1;
        }
        if matches!(self.shown, Some((_, 0))) {
            self.shown = None;
        }
        let entered_attack =
            self.last_move.is_some_and(|m| m != me.move_id) && me.attacking() && me.move_id != self.entry.produces;
        self.record(stage, view, entered_attack);
        if self.last_move.is_none() {
            // The first frame after the Try starts (or the fight loop
            // resumes) only takes note: progress already made is not an
            // attempt of this Try.
            (self.counted, self.last_stage, self.last_move) = (stage, stage, Some(me.move_id));
            self.since_first = if stage > 0 { u32::from(view.state.frames) + 1 } else { 0 };
            self.last_time_left = view.time_left;
            return None;
        }
        let entered = self.last_move != Some(me.move_id);
        let came_out = entered && me.move_id == self.entry.produces;
        let other = (entered && me.move_id != self.entry.produces && REGISTRY.iter().any(|e| e.produces == me.move_id))
            .then_some(me.move_id);
        // A hit or chip landing on P1, a hit reaction, or a block of an
        // attack: not the guard posture alone.
        let interrupted = me.reacting() || read.landed[1].is_some() || read.blocked[1] || (me.guarding() && foe.attacking());

        // The move out. It needs the motion behind it: an open or settling
        // attempt, or a tracker that had counted two steps a frame ago.
        if came_out && (self.attempt.is_some() || self.settling.is_some() || self.last_stage >= ATTEMPT_STAGE) {
            self.attempt = None;
            self.settling = None;
            self.pending = None;
            self.shown = None;
            self.successes += 1;
            self.outcome = Some(Outcome::Success);
            // The strip starts again with the move.
            (self.counted, self.last_stage, self.last_move, self.since_first) = (0, stage, Some(me.move_id), 0);
            return self.outcome;
        }

        let mut ended = None;
        let mut miss_span = None;
        if let Some(settling) = &mut self.settling {
            let outcome = if let Some(mv) = other {
                Some((Outcome::Other(mv), None))
            } else {
                settling.left = settling.left.saturating_sub(1);
                (settling.left == 0).then(|| {
                    (Outcome::Miss(Miss { gate_free: view.gate_free, current_move: view.current_move, ..settling.miss }), Some(settling.span))
                })
            };
            if let Some((outcome, span)) = outcome {
                (ended, miss_span) = (Some(outcome), span);
                self.settling = None;
            }
        }
        let mut started = false;
        if self.attempt.is_none() && stage >= ATTEMPT_STAGE && self.last_stage < ATTEMPT_STAGE && !interrupted {
            // A new attempt settles the old miss at once.
            if let Some(settling) = self.settling.take() {
                if ended.is_none() {
                    (ended, miss_span) = (Some(Outcome::Miss(settling.miss)), Some(settling.span));
                }
            }
            self.attempt = Some(Attempt { reached: 0, awaited: None, accepted: [None; 2], done_gate_closed: 0, other: None });
            self.attempts += 1;
            started = true;
        }
        if let Some(attempt) = &mut self.attempt {
            if stage >= attempt.reached {
                attempt.reached = stage;
                (attempt.awaited, attempt.accepted) = (view.awaiting, view.accepts);
            }
            if stage == complete && !view.gate_free {
                attempt.done_gate_closed += 1;
            }
            attempt.other = attempt.other.or(other);
            if interrupted {
                ended = Some(Outcome::Interrupted);
                self.attempt = None;
            } else if stage < ATTEMPT_STAGE {
                let miss = Miss {
                    stage: attempt.reached,
                    awaited: attempt.awaited,
                    accepted: attempt.accepted,
                    frames: self.since_first,
                    done_gate_closed: attempt.done_gate_closed,
                    time_left: self.last_time_left,
                    ended: if self.last_time_left == 0 { Ended::WindowRanOut } else { Ended::Released },
                    gate_free: view.gate_free,
                    current_move: view.current_move,
                };
                let other = attempt.other;
                let span = Span { fall: self.frame_no, first: self.frame_no.saturating_sub(u64::from(self.since_first)) };
                self.attempt = None;
                if let Some(mv) = other {
                    ended = Some(Outcome::Other(mv));
                } else if settle == 0 {
                    (ended, miss_span) = (Some(Outcome::Miss(miss)), Some(span));
                } else {
                    self.settling = Some(Settling { miss, span, left: settle });
                }
            }
        }
        // A button the engine took on the very frame of the motion's second
        // direction clears the tracker before stage two is ever seen: the stick
        // shows the direction the table awaited at step one and an attack began.
        // It is an attempt all the same, worded as an early button.
        if self.attempt.is_none()
            && stage == 0
            && self.last_stage == 1
            && entered_attack
            && !interrupted
            && ended.is_none()
            && self.history.back().is_some_and(|f| f.took_any() && f.dir.is_some() && f.dir == self.last_awaiting)
        {
            self.attempts += 1;
            let miss = Miss {
                stage: 2,
                awaited: None,
                accepted: [None; 2],
                frames: self.since_first,
                done_gate_closed: 0,
                time_left: self.last_time_left,
                ended: Ended::Released,
                gate_free: view.gate_free,
                current_move: view.current_move,
            };
            let span = Span { fall: self.frame_no, first: self.frame_no.saturating_sub(u64::from(self.since_first)) };
            (ended, miss_span) = (Some(Outcome::Miss(miss)), Some(span));
        }
        match (ended, miss_span) {
            (Some(Outcome::Miss(miss)), Some(span)) => self.pending = Some(Pending { miss, span }),
            (Some(Outcome::Interrupted | Outcome::Other(_)), _) => self.pending = None,
            _ => {}
        }
        if ended.is_some() {
            self.outcome = ended;
        }
        self.counted = stage;
        self.last_stage = stage;
        self.last_move = Some(me.move_id);
        self.last_awaiting = view.awaiting;
        if stage > 0 {
            self.since_first += 1;
            self.last_time_left = view.time_left;
        } else {
            self.since_first = 0;
        }
        // The player has started again: the words for the last miss are
        // chosen from what there is, and its inputs make way.
        self.choose_fix(started);
        if started {
            self.shown = None;
        }
        ended
    }

    /// Chooses the words for the pending miss once they can be: the engine
    /// has said that the attempt failed and where ([`Miss`]); the frames of
    /// the player's inputs, and what the engine took of them, say which words
    /// fit. `force` chooses with what there is, without waiting out the
    /// window again for a late direction.
    fn choose_fix(&mut self, force: bool) {
        let Some(Pending { miss, span }) = self.pending else { return };
        let class = self.entry.input.button;
        let until = span.fall + self.window;
        let frames: Vec<Frame> = self.history.iter().filter(|f| f.no >= span.first && f.no <= until).copied().collect();
        let to_fall = |f: &&Frame| f.no <= span.fall;
        let complete = self.entry.input.motion.len() as u8;
        let first_done = frames.iter().find(|f| f.counted == complete).map_or(u64::MAX, |f| f.no);
        // A button, of either class, the engine took before the motion was done
        // (a little before its first direction counts): its attack took the
        // fighter, or cleared the tracker.
        let taken_early = {
            let lead = self.history.iter().filter(|f| f.no < span.first && f.no + EARLY_LOOKBACK >= span.first);
            lead.chain(frames.iter()).filter(|f| f.no <= span.fall && f.no < first_done).any(Frame::took_any)
        };
        let kind = if miss.stage >= complete {
            // The motion was done: what the engine took, and the gate, say why
            // the move never came out.
            // The frames of the command's done window, the one the tracker was
            // cleared on (a button taken then clears it) included.
            let window: Vec<&Frame> = frames.iter().filter(to_fall).filter(|f| f.no >= first_done).collect();
            let done: Vec<&Frame> = window.iter().copied().filter(|f| f.counted == complete).collect();
            let held = window.iter().any(|f| (f.took(class) || f.pressed(class)) && !f.gate_open);
            let wrong = window.iter().any(|f| f.took(class.other()));
            Some(if taken_early {
                FixKind::ButtonTooEarly
            } else if held {
                FixKind::WaitToRecover
            } else if wrong {
                FixKind::WrongButton(class)
            } else if done.iter().any(|f| f.down(class) && f.gate_open) {
                // Down while the command was done and the gate open, and not
                // taken: the engine takes it on the release, which came too late.
                FixKind::ReleaseSooner(class)
            } else if !done.iter().any(|f| f.gate_open) {
                FixKind::WaitToRecover
            } else {
                FixKind::PressSooner(class)
            })
        } else if miss.ended == Ended::Released {
            // The tracker was cleared with time left: a button the engine took
            // did it (its attack took the motion); otherwise the direction the
            // table awaited was missed. A late direction is no word here.
            if taken_early { Some(FixKind::ButtonTooEarly) } else { miss.awaited.map(FixKind::Missed) }
        } else {
            // The window ran out: a direction the engine takes, arriving once the
            // tracker had fallen back, was too late; none arrived at all, and the
            // one the table awaited was missed.
            let late = frames.iter().filter(|f| f.no >= span.fall).any(|f| f.dir.is_some_and(|d| miss.accepted.contains(&Some(d))));
            if late {
                Some(FixKind::TooSlow)
            } else if force || self.frame_no >= until {
                miss.awaited.map(FixKind::Missed)
            } else {
                return;
            }
        };
        self.pending = None;
        if let Some(kind) = kind {
            // An arrow points the way the stick went on the frame the motion stalled.
            let facing_right = frames.iter().find(|f| f.no == span.fall).or(frames.last()).is_none_or(|f| f.facing_right);
            self.fix = Some(Fix { kind, facing_right });
            self.fix_fresh = true;
            self.shown = Some((labels(&frames), INPUTS_FRAMES));
        }
    }
}

/// The player's inputs as the strip's labels: each direction the stick
/// moved to (neutral between lets the same one count again), and the button
/// of a class when it went down (`P`, `K`).
fn labels(frames: &[Frame]) -> Vec<String> {
    let mut out = Vec::new();
    let mut last = None;
    let mut buttons = (false, false);
    for f in frames {
        if f.dir != last {
            out.extend(f.dir.map(|d| d.initials().to_owned()));
            last = f.dir;
        }
        if f.punch && !buttons.0 {
            out.push(ButtonClass::Punch.letter().to_string());
        }
        if f.kick && !buttons.1 {
            out.push(ButtonClass::Kick.letter().to_string());
        }
        buttons = (f.punch, f.kick);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ROM's window, as the views below give it.
    const LATE_FRAMES: u64 = 0x12;
    use crate::fight_reader::{FighterRead, LandedHit};
    use crate::moves::{RecognizerState, REGISTRY};

    fn view(stage: u8, done: bool, mv: u8) -> RecognizerView {
        RecognizerView {
            state: RecognizerState { stage, frames: 0, done },
            time_left: 10,
            awaiting: Some(Dir::Forward),
            accepts: [Some(Dir::Forward), None],
            gate_free: true,
            current_move: mv,
            window: 0x12,
            latched: [false; 2],
        }
    }

    fn fighter(mv: u8) -> FighterRead {
        FighterRead { present: true, health: 0x30, move_id: mv, state: 0, ..FighterRead::default() }
    }

    fn read(mv: u8) -> FightRead {
        FightRead { fighters: [fighter(mv), fighter(1)], ..FightRead::default() }
    }

    fn hadoken() -> Coach {
        Coach::new(REGISTRY[0])
    }

    /// Observe a frame with the tracker at `stage` and the fighter in `mv`.
    fn frame(coach: &mut Coach, stage: u8, mv: u8) -> Option<Outcome> {
        coach.observe(&view(stage, stage == 3, mv), &read(mv), true, 0)
    }

    fn settling(coach: &mut Coach, stage: u8, mv: u8, settle: u32) -> Option<Outcome> {
        coach.observe(&view(stage, stage == 3, mv), &read(mv), true, settle)
    }

    #[test]
    fn the_strip_lights_the_trackers_stage() {
        let mut coach = hadoken();
        for stage in [0, 1, 2, 3, 0] {
            frame(&mut coach, stage, 1);
            let Strip::Input { lit, steps } = coach.strip() else { panic!("the input") };
            assert_eq!((lit, steps.len()), (usize::from(stage), 4), "stage {stage}");
        }
    }

    #[test]
    fn a_first_step_alone_is_no_attempt() {
        let mut coach = hadoken();
        for _ in 0..30 {
            assert_eq!(frame(&mut coach, 1, 5), None);
        }
        frame(&mut coach, 0, 5);
        assert_eq!((coach.attempting(), coach.attempts(), coach.outcome()), (false, 0, None));
    }

    #[test]
    fn the_second_step_starts_an_attempt_and_the_move_out_ends_it() {
        let mut coach = hadoken();
        frame(&mut coach, 1, 5);
        frame(&mut coach, 2, 6);
        assert!(coach.attempting());
        frame(&mut coach, 3, 2);
        // The tracker is cleared on the frame the move starts.
        assert_eq!(frame(&mut coach, 0, 0x1a), Some(Outcome::Success));
        assert_eq!((coach.attempting(), coach.attempts(), coach.successes()), (false, 1, 1));
        assert_eq!(coach.lit(), 0, "the strip starts again");
        // The move running on is not another success.
        assert_eq!(frame(&mut coach, 0, 0x1a), None);
    }

    #[test]
    fn a_move_out_counts_without_an_open_attempt_when_the_tracker_was_done() {
        // An interruption closed the attempt, the tracker went on to done, and
        // the move came out: it is a success all the same.
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 2, 5);
        let hurt = FightRead { fighters: [fighter(5), FighterRead { move_id: 0x10, ..fighter(1) }], landed: [None, Some(LandedHit { damage: 4, into_hitstun: false, chip: false })], ..FightRead::default() };
        assert_eq!(coach.observe(&view(2, false, 5), &hurt, true, 0), Some(Outcome::Interrupted));
        frame(&mut coach, 3, 5);
        assert!(!coach.attempting());
        assert_eq!(frame(&mut coach, 0, 0x1a), Some(Outcome::Success));
        // But a move out with no motion behind it counts for nothing.
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 1, 5);
        assert_eq!(frame(&mut coach, 0, 0x1a), None);
        assert_eq!(coach.successes(), 0);
    }

    #[test]
    fn a_move_already_out_when_the_attempt_starts_does_not_count() {
        let mut coach = hadoken();
        frame(&mut coach, 0, 0x1a);
        frame(&mut coach, 1, 0x1a);
        frame(&mut coach, 2, 0x1a);
        assert!(coach.attempting());
        assert_eq!(frame(&mut coach, 3, 0x1a), None, "the old Hadoken is not the new one");
    }

    #[test]
    fn a_tracker_that_falls_back_is_a_miss_with_what_the_engine_knew() {
        let mut coach = hadoken();
        let v = |stage: u8, frames: u8, time_left: u8, done: bool, gate: bool| RecognizerView {
            state: RecognizerState { stage, frames, done },
            time_left,
            awaiting: Some(Dir::Forward),
            accepts: [Some(Dir::Forward), Some(Dir::DownForward)],
            gate_free: gate,
            current_move: 6,
            window: 0x12,
            latched: [false; 2],
        };
        let r = read(6);
        coach.observe(&v(1, 0, 18, false, true), &r, true, 0);
        coach.observe(&v(1, 1, 17, false, true), &r, true, 0);
        coach.observe(&v(2, 2, 16, false, true), &r, true, 0);
        coach.observe(&v(3, 3, 15, true, false), &r, true, 0);
        coach.observe(&v(3, 4, 14, true, false), &r, true, 0);
        coach.observe(&v(3, 5, 13, true, true), &r, true, 0);
        // The stick is let go with time left: reset, not timed out.
        let miss = coach.observe(&v(0, 0, 0, false, true), &r, true, 0);
        let Some(Outcome::Miss(m)) = miss else { panic!("a miss, not {miss:?}") };
        assert_eq!((m.stage, m.awaited, m.accepted), (3, Some(Dir::Forward), [Some(Dir::Forward), Some(Dir::DownForward)]));
        assert_eq!((m.frames, m.done_gate_closed, m.time_left, m.ended), (6, 2, 13, Ended::Released));
        assert_eq!((coach.successes(), coach.outcome(), coach.attempting()), (0, miss, false));
    }

    #[test]
    fn a_window_that_ran_out_is_told_from_a_release() {
        let mut coach = hadoken();
        let r = read(6);
        let v = |stage: u8, frames: u8, time_left: u8| RecognizerView {
            state: RecognizerState { stage, frames, done: stage == 3 },
            time_left,
            awaiting: Some(Dir::Forward),
            accepts: [Some(Dir::Forward), None],
            gate_free: true,
            current_move: 6,
            window: 0x12,
            latched: [false; 2],
        };
        coach.observe(&v(1, 0, 18), &r, true, 0);
        coach.observe(&v(1, 1, 17), &r, true, 0);
        coach.observe(&v(2, 2, 16), &r, true, 0);
        for n in 3..=18u8 {
            coach.observe(&v(2, n, 18 - n), &r, true, 0);
        }
        let Some(Outcome::Miss(m)) = coach.observe(&v(0, 0, 0), &r, true, 0) else { panic!("a miss") };
        assert_eq!((m.stage, m.awaited, m.ended, m.time_left), (2, Some(Dir::Forward), Ended::WindowRanOut, 0));
        assert_eq!(m.frames, 19, "counted from the first direction, not from step two");
    }

    #[test]
    fn a_hit_counts_as_nothing_but_the_guard_posture_alone_does_not() {
        let landed = Some(LandedHit { damage: 4, into_hitstun: false, chip: false });
        let hurt = |p1: FighterRead, foe: FighterRead, landed_on_p1: bool, blocked: bool| FightRead {
            fighters: [p1, foe],
            landed: [None, landed.filter(|_| landed_on_p1)],
            blocked: [false, blocked],
        };
        let attacking = fighter(0x10);
        let cases = [
            ("a hit reaction", hurt(fighter(0x30), fighter(1), false, false), true),
            ("a hit landed", hurt(fighter(5), attacking, true, false), true),
            ("chip landed", hurt(fighter(5), attacking, true, false), true),
            ("a block the reader saw", hurt(fighter(5), fighter(1), false, true), true),
            ("a guard against an attack", hurt(fighter(4), attacking, false, false), true),
            ("a crouching guard against an attack", hurt(fighter(8), attacking, false, false), true),
            ("guard posture and nothing coming", hurt(fighter(4), fighter(1), false, false), false),
            ("crouching guard and nothing coming", hurt(fighter(8), fighter(1), false, false), false),
            ("guard state bit and nothing coming", hurt(FighterRead { state: 0x20, ..fighter(5) }, fighter(1), false, false), false),
        ];
        for (what, r, interrupts) in cases {
            let mut coach = hadoken();
            frame(&mut coach, 1, 5);
            frame(&mut coach, 2, 5);
            let out = coach.observe(&view(2, false, r.fighters[0].move_id), &r, true, 0);
            assert_eq!(out, interrupts.then_some(Outcome::Interrupted), "{what}");
            // The tracker times out afterwards: no miss after an interruption.
            let after = frame(&mut coach, 0, 1);
            assert_eq!(after.is_some(), !interrupts, "{what}");
        }
    }

    #[test]
    fn a_tracker_left_high_by_an_interruption_has_to_fall_back_first() {
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 2, 5);
        frame(&mut coach, 2, 5);
        coach.observe(&view(2, false, 0x30), &read(0x30), true, 0);
        assert_eq!(coach.attempts(), 1);
        // Still at stage two when the fighter stands again: not a new attempt.
        frame(&mut coach, 2, 1);
        frame(&mut coach, 3, 1);
        assert_eq!((coach.attempts(), coach.attempting()), (1, false));
        frame(&mut coach, 0, 1);
        frame(&mut coach, 1, 1);
        frame(&mut coach, 2, 1);
        assert_eq!(coach.attempts(), 2);
    }

    #[test]
    fn a_hit_while_the_fighter_is_stunned_starts_no_attempt() {
        let mut coach = hadoken();
        coach.observe(&view(1, false, 0x30), &read(0x30), true, 0);
        assert_eq!(coach.observe(&view(2, false, 0x30), &read(0x30), true, 0), None);
        assert_eq!((coach.attempting(), coach.attempts()), (false, 0));
    }

    #[test]
    fn a_tracker_already_high_when_the_try_starts_waits_to_fall_back() {
        let mut coach = hadoken();
        frame(&mut coach, 3, 1);
        assert_eq!((coach.attempting(), coach.attempts()), (false, 0));
        frame(&mut coach, 0, 1);
        frame(&mut coach, 2, 1);
        assert!(coach.attempting());
    }

    #[test]
    fn nothing_is_attempted_outside_the_fight_loop() {
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 2, 5);
        assert!(coach.attempting());
        assert_eq!(coach.observe(&view(2, false, 5), &read(5), false, 0), None);
        assert!(!coach.attempting());
        assert_eq!(coach.lit(), 0);
    }

    #[test]
    fn a_miss_is_held_open_for_the_recovery_buffer_and_a_move_out_meanwhile_is_a_success() {
        let window = 5;
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 2, 5);
        frame(&mut coach, 3, 5);
        // The tracker is gone, the fighter still held by the gate: not a miss yet.
        for _ in 0..window {
            assert_eq!(settling(&mut coach, 0, 0x14, window + 1), None);
            assert!(coach.attempting());
        }
        assert_eq!(settling(&mut coach, 0, 0x1a, window + 1), Some(Outcome::Success));
        assert_eq!((coach.successes(), coach.attempting()), (1, false));
        // Without the move, the miss is settled once the window is through.
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 2, 5);
        frame(&mut coach, 3, 5);
        // The frame the tracker fell back, then the window's frames.
        for _ in 0..=window {
            assert_eq!(settling(&mut coach, 0, 5, window + 1), None);
        }
        assert!(matches!(settling(&mut coach, 0, 5, window + 1), Some(Outcome::Miss(_))));
        assert!(!coach.attempting());
    }

    #[test]
    fn a_new_attempt_settles_the_held_miss_at_once() {
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 2, 5);
        assert_eq!(settling(&mut coach, 0, 5, 9), None);
        frame(&mut coach, 1, 5);
        let out = settling(&mut coach, 2, 5, 9);
        assert!(matches!(out, Some(Outcome::Miss(_))), "{out:?}");
        assert_eq!(coach.attempts(), 2);
        assert!(coach.attempting());
    }

    #[test]
    fn another_move_of_the_registry_coming_out_is_other_not_a_miss() {
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 2, 5);
        // The Shoryuken (0x19) comes out as the tracker clears.
        assert_eq!(frame(&mut coach, 0, 0x19), Some(Outcome::Other(0x19)));
        assert_eq!((coach.successes(), coach.attempting()), (0, false));
        // A move that is no entry's (a punch) is not.
        let mut coach = hadoken();
        frame(&mut coach, 0, 5);
        frame(&mut coach, 2, 5);
        assert!(matches!(frame(&mut coach, 0, 0x10), Some(Outcome::Miss(_))));
    }

    #[test]
    fn a_consumed_tracker_counts_no_steps() {
        let consumed = RecognizerState { stage: 3, frames: 0, done: false };
        assert_eq!(consumed.counted(), 0);
        assert_eq!(RecognizerState { stage: 3, frames: 5, done: true }.counted(), 3);
    }

    // The fixes (ML6): synthetic frames, the engine's reading (tracker stage,
    // gate, the move, the latches) and the player's inputs given a frame at a time.

    const N: [bool; 4] = [false; 4];
    const D: [bool; 4] = [false, false, true, false];
    const DF: [bool; 4] = [true, false, true, false];
    const F: [bool; 4] = [true, false, false, false];
    const B: [bool; 4] = [false, true, false, false];
    /// A stage code for the last frame at step two with the window run out.
    const WINDOW_OUT: u8 = 0xf2;

    /// One frame, in full: the tracker at `stage` (`WINDOW_OUT` is step two
    /// with no time left), the stick, the buttons held (`P` a medium punch,
    /// `K` a medium kick), the gate, the fighter's move and the latches.
    #[allow(clippy::too_many_arguments)]
    fn obs_full(coach: &mut Coach, stage: u8, dirs: [bool; 4], held: (bool, bool), gate: bool, mv: u8, latched: [bool; 2]) -> Option<Outcome> {
        let mut pad = PlayerInputs { directions: dirs, ..PlayerInputs::default() };
        pad.buttons[1] = held.0;
        pad.buttons[4] = held.1;
        coach.input(pad, true);
        let (stage, time_left) = if stage == WINDOW_OUT { (2, 0) } else { (stage, 10) };
        // The direction the ROM's table awaits at the step: D, DF, then F.
        let awaiting = [Dir::Down, Dir::DownForward, Dir::Forward][usize::from(stage.min(2))];
        let v = RecognizerView {
            state: RecognizerState { stage, frames: 0, done: stage == 3 },
            time_left,
            awaiting: Some(awaiting),
            accepts: [Some(awaiting), None],
            gate_free: gate,
            current_move: mv,
            window: 0x12,
            latched,
        };
        coach.observe(&v, &read(mv), true, 0)
    }

    /// One frame with a medium punch held or not, the fighter in its stance.
    fn obs(coach: &mut Coach, stage: u8, dirs: [bool; 4], punch: bool, gate: bool) -> Option<Outcome> {
        obs_full(coach, stage, dirs, (punch, false), gate, 5, [false; 2])
    }

    type Frames = Vec<(u8, [bool; 4], bool, bool)>;

    /// Runs `frames` (stage, stick, punch, gate) from a tracker at rest and
    /// returns the coach.
    fn run(frames: &Frames) -> Coach {
        let mut coach = hadoken();
        obs(&mut coach, 0, N, false, true);
        for &(stage, dirs, punch, gate) in frames {
            obs(&mut coach, stage, dirs, punch, gate);
        }
        coach
    }

    fn kind(coach: &Coach) -> Option<FixKind> {
        coach.fix().map(|f| f.kind)
    }

    /// D, DF held three frames each and the tracker at 1 then 2.
    fn two_steps() -> Frames {
        [vec![(1, D, false, true); 3], vec![(2, DF, false, true); 3]].concat()
    }

    /// Two steps and a stall until the window ran out: `hold` frames more at
    /// step two, the last with no time left, then the tracker gone.
    fn stalled(hold: usize) -> Frames {
        let mut frames = two_steps();
        frames.extend(vec![(2, DF, false, true); hold]);
        frames.push((WINDOW_OUT, DF, false, true));
        frames.push((0, DF, false, true));
        frames
    }

    #[test]
    fn the_direction_arriving_after_the_window_ran_out_is_too_slow_the_frame_it_comes() {
        let mut frames = stalled(8);
        frames.extend(vec![(0, DF, false, true); 3]);
        let mut coach = run(&frames);
        assert!(coach.deciding() && coach.fix().is_none(), "it waits to see whether the direction still comes");
        obs(&mut coach, 0, F, false, true);
        assert_eq!(kind(&coach), Some(FixKind::TooSlow));
        assert_eq!(coach.take_fix().map(|f| f.kind), Some(FixKind::TooSlow));
        assert_eq!(coach.take_fix(), None, "put through the call-outs once");
        assert_eq!(kind(&coach), Some(FixKind::TooSlow), "and still the last fix");
        let labels: Vec<String> = ["D", "DF", "F"].map(String::from).to_vec();
        assert_eq!(coach.shown_inputs(), Some(labels.as_slice()));
        assert!(!coach.deciding());
    }

    #[test]
    fn a_direction_that_never_comes_is_missed_once_the_window_has_gone_again() {
        let mut coach = run(&stalled(6));
        // A wrong direction (B) is no arrival of the awaited one.
        for n in 0..LATE_FRAMES {
            assert!(coach.fix().is_none(), "frame {n}: still waiting");
            obs(&mut coach, 0, if n % 2 == 0 { B } else { N }, false, true);
        }
        obs(&mut coach, 0, N, false, true);
        assert_eq!(kind(&coach), Some(FixKind::Missed(Dir::Forward)));
        assert_eq!(coach.fix().unwrap().text(InputNotation::Words), "MISSED F");
        assert_eq!(coach.fix().unwrap().text(InputNotation::Numpad), "MISSED 6");
    }

    #[test]
    fn a_tracker_cleared_with_time_left_is_never_too_slow() {
        // The tracker fell back with time left (a button cleared it, or the stick
        // was let go), and the direction came within the window: it was not late.
        let mut frames = two_steps();
        frames.extend([(0, DF, false, true), (0, F, false, true)]);
        let coach = run(&frames);
        assert_eq!(kind(&coach), Some(FixKind::Missed(Dir::Forward)), "no wait, no TOO SLOW");
        assert!(!coach.deciding());
    }

    #[test]
    fn a_direction_the_lenient_trackers_take_counts_as_arriving() {
        // The engine took DF or F at this step: either arriving late is too slow.
        for late in [DF, F] {
            let mut coach = hadoken();
            obs(&mut coach, 0, N, false, true);
            let frame = |coach: &mut Coach, stage: u8, dirs: [bool; 4], time_left: u8| {
                let v = RecognizerView {
                    state: RecognizerState { stage, frames: 0, done: false },
                    time_left,
                    awaiting: Some(Dir::DownForward),
                    accepts: [Some(Dir::DownForward), Some(Dir::Forward)],
                    gate_free: true,
                    current_move: 5,
                    window: 0x12,
                    latched: [false; 2],
                };
                coach.input(PlayerInputs { directions: dirs, ..PlayerInputs::default() }, true);
                coach.observe(&v, &read(5), true, 0);
            };
            for (stage, dirs, left) in [(1, D, 10), (1, D, 10), (2, DF, 10), (2, DF, 0)] {
                frame(&mut coach, stage, dirs, left);
            }
            frame(&mut coach, 0, N, 0);
            frame(&mut coach, 0, late, 0);
            assert_eq!(kind(&coach), Some(FixKind::TooSlow), "{late:?}");
        }
    }

    #[test]
    fn a_motion_done_and_dropped_with_no_button_is_press_sooner() {
        let mut frames = two_steps();
        frames.extend(vec![(3, F, false, true); 8]);
        frames.push((0, N, false, true));
        let coach = run(&frames);
        assert_eq!(kind(&coach), Some(FixKind::PressSooner(ButtonClass::Punch)));
        assert_eq!(coach.fix().unwrap().text(InputNotation::Arrows), "PRESS P SOONER");
        assert_eq!(coach.shown_inputs().map(<[String]>::len), Some(3));
    }

    /// A tracker already at stage two, then the frames given in full.
    fn up_to_step_two() -> Coach {
        let mut coach = hadoken();
        obs(&mut coach, 0, N, false, true);
        for (stage, dirs) in [(1, D), (1, D), (1, D), (2, DF), (2, DF)] {
            obs(&mut coach, stage, dirs, false, true);
        }
        coach
    }

    #[test]
    fn a_button_the_engine_took_before_the_motion_was_done_is_too_early() {
        // A kick tapped on the second step with the gate open: the engine took
        // it (a kick attack began, 0x17) and cleared the tracker with time left.
        // Either class counts, and the late F is no TOO SLOW (the probe's case).
        let mut coach = up_to_step_two();
        obs_full(&mut coach, 2, DF, (false, true), true, 0x17, [false; 2]);
        // The tracker was cleared on the frame the kick was taken.
        obs_full(&mut coach, 0, DF, (false, false), false, 0x17, [false; 2]);
        for _ in 0..3 {
            obs_full(&mut coach, 0, F, (false, false), false, 0x17, [false; 2]);
        }
        assert_eq!(kind(&coach), Some(FixKind::ButtonTooEarly), "{:?}", coach.outcome());
        assert_eq!(coach.shown_inputs().map(|l| l.join(" ")), Some("D DF K".to_owned()));
    }

    #[test]
    fn a_motion_dropped_by_a_button_the_engine_took_before_it_was_done_stays_too_early_when_done() {
        // The punch taken on the second step; the attack held the fighter, so
        // the motion ran on to done with the gate shut.
        let mut coach = up_to_step_two();
        obs_full(&mut coach, 2, DF, (true, false), true, 0x1d, [false; 2]);
        for _ in 0..3 {
            obs_full(&mut coach, 2, DF, (true, false), false, 0x1d, [false; 2]);
        }
        for _ in 0..6 {
            obs_full(&mut coach, 3, F, (false, false), false, 0x1d, [false; 2]);
        }
        obs_full(&mut coach, 0, N, (false, false), true, 0x1d, [false; 2]);
        assert_eq!(kind(&coach), Some(FixKind::ButtonTooEarly));
        assert_eq!(coach.shown_inputs().map(|l| l.join(" ")), Some("D DF P F".to_owned()));
    }

    #[test]
    fn a_press_the_engine_did_not_take_is_not_early() {
        // The probe's overhaul case: P held from before and let go on the second
        // direction. The engine took nothing (no attack, no latch), so the motion
        // ran to done with no button: press sooner.
        let mut coach = hadoken();
        obs(&mut coach, 0, N, true, true);
        for (stage, dirs, punch) in [(1, D, true), (1, D, true), (1, D, true), (2, DF, true), (2, DF, true), (2, DF, false)] {
            obs(&mut coach, stage, dirs, punch, true);
        }
        for _ in 0..8 {
            obs(&mut coach, 3, F, false, true);
        }
        obs(&mut coach, 0, N, false, true);
        assert_eq!(kind(&coach), Some(FixKind::PressSooner(ButtonClass::Punch)));
    }

    #[test]
    fn a_button_down_through_the_motion_and_taken_only_on_a_late_release_is_release_sooner() {
        // Faithful 1987: the punch went down on the second step and was held
        // through the motion; the engine takes it on the release, which came
        // after the window ran out (or after the stick was let go). Not early: the
        // engine took nothing before the motion was done; not "press sooner": the
        // button was down.
        let mut coach = up_to_step_two();
        for _ in 0..6 {
            obs(&mut coach, 3, F, true, true);
        }
        obs(&mut coach, 0, N, true, true);
        // The release, taken (the attack begins) once the tracker is gone.
        obs_full(&mut coach, 0, N, (false, false), true, 0x0f, [false; 2]);
        assert_eq!(kind(&coach), Some(FixKind::ReleaseSooner(ButtonClass::Punch)));
        assert_eq!(coach.fix().unwrap().text(InputNotation::Words), "RELEASE P SOONER");
    }

    #[test]
    fn a_button_pressed_after_the_motion_was_dropped_is_press_sooner() {
        // The probe's "stick released before P": the engine took the punch after
        // the fall, and the punch was not down while the command was done.
        let mut coach = up_to_step_two();
        for _ in 0..3 {
            obs(&mut coach, 3, F, false, true);
        }
        for _ in 0..2 {
            obs(&mut coach, 0, N, false, true);
        }
        obs_full(&mut coach, 0, N, (true, false), true, 0x0f, [false; 2]);
        for _ in 0..LATE_FRAMES + 1 {
            obs_full(&mut coach, 0, N, (false, false), true, 0x0f, [false; 2]);
        }
        assert_eq!(kind(&coach), Some(FixKind::PressSooner(ButtonClass::Punch)));
    }

    #[test]
    fn the_other_classs_button_taken_while_the_motion_was_done_is_the_wrong_button() {
        // The probe's kick in time: the engine took a kick and the Hadoken wants a punch.
        let mut coach = up_to_step_two();
        obs(&mut coach, 3, F, false, true);
        obs_full(&mut coach, 3, F, (false, true), true, 0x17, [false; 2]);
        for _ in 0..4 {
            obs_full(&mut coach, 3, F, (false, false), false, 0x17, [false; 2]);
        }
        obs_full(&mut coach, 0, N, (false, false), true, 0x17, [false; 2]);
        assert_eq!(kind(&coach), Some(FixKind::WrongButton(ButtonClass::Punch)));
        assert_eq!(coach.fix().unwrap().text(InputNotation::Words), "PRESS P NOT K");
    }

    #[test]
    fn a_button_taken_while_the_gate_held_the_fighter_is_wait_to_recover() {
        // The latch went up on a frame the gate was shut: the engine took the
        // button and kept it, and the gate held the fighter.
        let mut frames: Frames = vec![(1, D, false, false); 3];
        frames.extend(vec![(2, DF, false, false); 3]);
        let mut coach = hadoken();
        obs(&mut coach, 0, N, false, false);
        for (stage, dirs, punch, gate) in frames {
            obs(&mut coach, stage, dirs, punch, gate);
        }
        obs(&mut coach, 3, F, false, false);
        obs_full(&mut coach, 3, F, (true, false), false, 0x0f, [true, false]);
        for _ in 0..6 {
            obs_full(&mut coach, 3, F, (false, false), false, 0x0f, [true, false]);
        }
        obs_full(&mut coach, 0, N, (false, false), true, 0x0f, [false; 2]);
        assert_eq!(kind(&coach), Some(FixKind::WaitToRecover));
        // With no button at all but the gate shut on every frame the command
        // was done, a button could not have helped: wait.
        let mut coach = hadoken();
        obs(&mut coach, 0, N, false, false);
        for _ in 0..3 {
            obs(&mut coach, 1, D, false, false);
        }
        for _ in 0..3 {
            obs(&mut coach, 2, DF, false, false);
        }
        for _ in 0..8 {
            obs(&mut coach, 3, F, false, false);
        }
        obs(&mut coach, 0, N, false, true);
        assert_eq!(kind(&coach), Some(FixKind::WaitToRecover));
        // With a frame the gate was open and no button: press sooner.
        let mut frames = two_steps();
        frames.extend(vec![(3, F, false, false); 4]);
        frames.extend(vec![(3, F, false, true); 4]);
        frames.push((0, N, false, true));
        assert_eq!(kind(&run(&frames)), Some(FixKind::PressSooner(ButtonClass::Punch)));
    }

    #[test]
    fn a_button_on_the_frame_of_the_second_direction_is_an_attempt_that_was_too_early() {
        // The tracker was at step one, the stick goes to DF (the direction the
        // table awaited) and a punch attack begins on the same frame: the tracker
        // is cleared before step two is ever seen.
        let mut coach = hadoken();
        obs(&mut coach, 0, N, false, true);
        for _ in 0..3 {
            obs(&mut coach, 1, D, false, true);
        }
        obs_full(&mut coach, 0, DF, (true, false), true, 0x1d, [false; 2]);
        assert_eq!((coach.attempts(), kind(&coach)), (1, Some(FixKind::ButtonTooEarly)));
        assert!(matches!(coach.outcome(), Some(Outcome::Miss(m)) if m.stage == 2));
        // Crouching and punching (the stick still on D) is no attempt.
        let mut coach = hadoken();
        obs(&mut coach, 0, N, false, true);
        for _ in 0..3 {
            obs(&mut coach, 1, D, false, true);
        }
        obs_full(&mut coach, 0, D, (true, false), true, 0x1d, [false; 2]);
        assert_eq!((coach.attempts(), coach.fix()), (0, None));
    }

    #[test]
    fn a_new_attempt_ends_the_wait_and_clears_the_old_inputs() {
        let mut frames = stalled(2);
        frames.extend(vec![(0, N, false, true); 3]);
        let mut coach = run(&frames);
        assert!(coach.deciding());
        // D, DF again, a few frames after the miss.
        for (stage, dirs) in [(1, D), (1, D), (2, DF)] {
            obs(&mut coach, stage, dirs, false, true);
        }
        assert_eq!(kind(&coach), Some(FixKind::Missed(Dir::Forward)), "worded as the second attempt starts");
        assert!(coach.take_fix().is_some());
        assert!(coach.shown_inputs().is_none(), "the old inputs make way");
        assert!(coach.attempting());
    }

    #[test]
    fn the_inputs_are_held_for_a_few_seconds_and_a_move_out_clears_them() {
        let mut frames = stalled(2);
        frames.extend(vec![(0, N, false, true); LATE_FRAMES as usize]);
        let mut coach = run(&frames);
        assert!(coach.shown_inputs().is_some());
        // Held for INPUTS_FRAMES frames, the one it was chosen on among them.
        for n in 0..INPUTS_FRAMES - 1 {
            assert!(coach.shown_inputs().is_some(), "frame {n}");
            obs(&mut coach, 0, N, false, true);
        }
        assert!(coach.shown_inputs().is_some(), "still up on the last of its frames");
        obs(&mut coach, 0, N, false, true);
        assert!(coach.shown_inputs().is_none(), "gone after {INPUTS_FRAMES} frames");
        assert!(coach.fix().is_some(), "the fix stays the last one");
        // Out of the fight loop: nothing is held over a round.
        let mut coach = run(&frames);
        assert!(coach.shown_inputs().is_some());
        coach.observe(&view(0, false, 5), &read(5), false, 0);
        assert!(coach.shown_inputs().is_none());
        // A move that comes out clears the line.
        let mut coach = run(&frames);
        for (stage, dirs) in [(1, D), (2, DF), (3, F)] {
            obs(&mut coach, stage, dirs, false, true);
        }
        coach.input(PlayerInputs::default(), true);
        assert_eq!(coach.observe(&view(0, false, 0x1a), &read(0x1a), true, 0), Some(Outcome::Success));
        assert!(coach.shown_inputs().is_none());
    }

    #[test]
    fn the_arrow_points_the_way_the_fighter_faced_when_the_motion_stalled() {
        // Facing left on the frame of the stall, right after: the arrow is the stall's.
        let mut coach = hadoken();
        let step = |coach: &mut Coach, stage: u8, dirs: [bool; 4], right: bool| {
            coach.input(PlayerInputs { directions: dirs, ..PlayerInputs::default() }, right);
            let (stage, time_left) = if stage == WINDOW_OUT { (2, 0) } else { (stage, 10) };
            let v = RecognizerView {
                state: RecognizerState { stage, frames: 0, done: stage == 3 },
                time_left,
                awaiting: Some(Dir::DownForward),
                accepts: [Some(Dir::DownForward), None],
                gate_free: true,
                current_move: 5,
                window: 0x12,
                latched: [false; 2],
            };
            coach.observe(&v, &read(5), true, 0);
        };
        step(&mut coach, 0, N, false);
        for (stage, dirs) in [(1, B), (1, B), (2, [false, true, true, false]), (WINDOW_OUT, [false, true, true, false]), (0, N)] {
            step(&mut coach, stage, dirs, false);
        }
        // The fighter crosses over afterwards, while the wait runs.
        for _ in 0..LATE_FRAMES + 1 {
            step(&mut coach, 0, N, true);
        }
        let fix = coach.fix().unwrap();
        assert_eq!(fix.kind, FixKind::Missed(Dir::DownForward));
        assert!(!fix.facing_right, "the stall was facing left");
        assert_eq!(fix.text(InputNotation::Arrows), "MISSED \u{2199}");
    }

    #[test]
    fn other_and_interrupted_leave_no_fix() {
        // A hit mid-attempt: nothing is chosen when the tracker runs out.
        let mut coach = hadoken();
        obs(&mut coach, 0, N, false, true);
        obs(&mut coach, 1, D, false, true);
        obs(&mut coach, 2, DF, false, true);
        let hurt = FightRead { fighters: [fighter(0x30), fighter(1)], ..FightRead::default() };
        coach.input(PlayerInputs::default(), true);
        assert_eq!(coach.observe(&view(2, false, 0x30), &hurt, true, 0), Some(Outcome::Interrupted));
        for _ in 0..60 {
            obs(&mut coach, 0, N, false, true);
        }
        assert!(coach.fix().is_none() && !coach.deciding() && coach.shown_inputs().is_none());
        // The other special coming out as the tracker clears: no miss to word.
        let mut coach = hadoken();
        obs(&mut coach, 0, N, false, true);
        obs(&mut coach, 1, D, false, true);
        obs(&mut coach, 2, DF, false, true);
        coach.input(PlayerInputs::default(), true);
        assert_eq!(coach.observe(&view(0, false, 0x19), &read(0x19), true, 0), Some(Outcome::Other(0x19)));
        for _ in 0..60 {
            obs(&mut coach, 0, N, false, true);
        }
        assert!(coach.fix().is_none() && !coach.deciding() && coach.shown_inputs().is_none());
    }

    #[test]
    fn a_fix_is_worded_in_each_notation_and_facing() {
        let missed = Fix { kind: FixKind::Missed(Dir::DownForward), facing_right: true };
        assert_eq!(missed.text(InputNotation::Arrows), "MISSED \u{2198}");
        assert_eq!(Fix { facing_right: false, ..missed }.text(InputNotation::Arrows), "MISSED \u{2199}");
        assert_eq!(missed.text(InputNotation::Numpad), "MISSED 3");
        assert_eq!(missed.text(InputNotation::Words), "MISSED DF");
        let kick = Fix { kind: FixKind::PressSooner(ButtonClass::Kick), facing_right: true };
        assert_eq!(kick.text(InputNotation::Arrows), "PRESS K SOONER");
        assert_eq!(Fix { kind: FixKind::TooSlow, facing_right: true }.text(InputNotation::Words), "TOO SLOW");
        assert_eq!(Fix { kind: FixKind::ButtonTooEarly, facing_right: true }.text(InputNotation::Words), "BUTTON TOO EARLY");
        assert_eq!(Fix { kind: FixKind::WrongButton(ButtonClass::Kick), facing_right: true }.text(InputNotation::Words), "PRESS K NOT P");
        // Every fix fits a call-out column.
        for text in ["PRESS P SOONER", "RELEASE P SOONER", "PRESS P NOT K", "BUTTON TOO EARLY", "WAIT TO RECOVER", "MISSED DF"] {
            assert!(text.len() <= sf_video::callouts::regions::LINE_LETTERS);
        }
    }
}
