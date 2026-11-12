//! Show (ticket #230, ML4; spec #226): in Training the player's own
//! fighter performs a move from the move list.
//!
//! A [`Show`] stands in for the player's pad. Each stepped frame it gives
//! the input the fight feeds to P1 in place of the pad's, so the move comes
//! out of the real recognizer, and it writes nothing to the machine
//! (ADR-0004). It reads the fighter the way the fight reader does:
//!
//! 1. **Waiting.** The pad is the player's still: a direction or button
//!    newly pressed while waiting cancels the show (a held key from the
//!    menu does not). Nothing is fed until the busy gate (`0x403d4`,
//!    `docs/reference/posture-dispatch.md`) frees the fighter and the
//!    fighter stands: +9 & 0xc9 is 0, 1, 8 or 9 (or the move is 0 or 0xc),
//!    and the move is the stance. The strip says `WAIT`.
//! 2. **Feeding.** The entry's frames ([`crate::moves::MoveInput::frames`])
//!    for the way the fighter faces on that frame, one a frame; the strip
//!    draws the input with each step lighting as it is fed.
//! 3. **Finishing.** Neutral until the move the entry produces has come out
//!    and ended, which hands control back. A move that never comes out
//!    (the fighter was hit while feeding) ends the show too, after
//!    [`COME_OUT_LIMIT`] frames. A show that ran its course
//!    ([`Show::completed`]) goes on as a Try of the same move (ML5,
//!    [`super::coach`]); one that was given up does not.
//!
//! A show also ends when the flow leaves the fight loop (a time-over or a
//! round's end), when either fighter is knocked out, and a wait that
//! the fighter never leaves is given up after [`WAIT_LIMIT`] frames, so
//! the pad is never held back for long.

use sf_video::callouts::Strip;

use super::session::PlayerInputs;
use crate::fight_reader::FighterRead;
use crate::canonical::{canonical_enum, Canonical, CanonicalHasher};
use crate::moves::{busy_gate_free, ButtonClass, MoveEntry, MoveKind, Recognizer, Strength};

/// Frames a show waits for the fighter to be free and standing before it
/// gives up (ten seconds at 60 Hz).
pub const WAIT_LIMIT: u32 = 600;
/// Frames after the input is fed that a move which has not come out is
/// given before the show ends.
pub const COME_OUT_LIMIT: u32 = 90;
/// Frames a move that has come out is given to end (the Tatsumaki spins
/// for about 110) before the show gives the pad back anyway.
pub const MOVE_LIMIT: u32 = 600;
/// The stance move (entity +0xa); 0 is the idle before one.
const STANCE: [u8; 2] = [0, 1];

/// Whether the busy gate (`0x403d4`) frees the fighter: +9 & 0xc9 is 0, 1,
/// 8 or 9, or its move is 0 or 0xc.
pub fn gate_free(fighter: &FighterRead) -> bool {
    busy_gate_free(fighter.state, fighter.move_id)
}

/// Free and standing: the gate is open and the fighter is in its stance.
pub fn ready(fighter: &FighterRead) -> bool {
    fighter.present && gate_free(fighter) && STANCE.contains(&fighter.move_id)
}

/// Whether `now` holds a direction or button `was` did not.
fn newly_pressed(was: &PlayerInputs, now: &PlayerInputs) -> bool {
    now.directions.iter().zip(&was.directions).any(|(n, w)| *n && !*w)
        || now.buttons.iter().zip(&was.buttons).any(|(n, w)| *n && !*w)
}

/// Some system actions need a real incoming attack to demonstrate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PracticeContact {
    Guard = 1,
    Strike = 2,
    Counter = 3,
}

pub(super) fn practice_contact(entry: &MoveEntry) -> Option<PracticeContact> {
    use crate::era_fight::sf5;
    if entry.recognizer == Recognizer::AlphaCounter {
        return Some(PracticeContact::Guard);
    }
    if entry.recognizer == Recognizer::FamilyMove
        && matches!(entry.name, "ST ANDREWS GREEN" | "EX ST ANDREWS GREEN")
    {
        return Some(PracticeContact::Counter);
    }
    if entry.recognizer != Recognizer::FamilyMove || entry.kind != MoveKind::System {
        return None;
    }
    match (entry.produces, entry.input.button) {
        (sf5::REVERSAL, ButtonClass::TriplePunch | ButtonClass::TripleKick) => Some(PracticeContact::Guard),
        (sf5::SHIFT_BREAK, ButtonClass::ShiftPair) => Some(PracticeContact::Strike),
        _ if entry.name == "DRIVE REVERSAL" => Some(PracticeContact::Guard),
        _ if matches!(entry.name, "MIND'S EYE" | "ISSHIN") => Some(PracticeContact::Strike),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Phase {
    Waiting { frames: u32, strike_ticks: u8 },
    /// `fed` frames of the input are through.
    Feeding { fed: usize },
    Finishing { frames: u32 },
}

canonical_enum!(Phase, "show-phase/v2";
    Waiting { frames, strike_ticks } => 0,
    Feeding { fed } => 1,
    Finishing { frames } => 2,
);

/// A show in progress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Show {
    entry: MoveEntry,
    phase: Phase,
    /// The entry's frames for a fighter facing right and facing left.
    right: Vec<PlayerInputs>,
    left: Vec<PlayerInputs>,
    /// The move has come out (the fighter was in `entry.produces`) since
    /// the input began.
    came_out: bool,
    /// The pad as the last frame left it, while waiting: the first frame's is
    /// the baseline (what the menu left held), a new press after it cancels.
    pad: Option<PlayerInputs>,
    /// The show ran its course: the move came out and ended.
    completed: bool,
}

impl Show {
    /// Only this state can change the pad a Show feeds next.
    pub(crate) fn encode_gameplay(&self, out: &mut CanonicalHasher) {
        let Self { entry, phase, right, left, came_out, pad, completed } = self;
        let MoveEntry {
            produces, name: _, kind: _, input: _, recognizer: _,
            call_out: _, available: _,
        } = entry;
        out.tag("training-show/v2");
        produces.encode(out);
        practice_contact(entry).map(|contact| contact as u8).encode(out);
        phase.encode(out);
        right.encode(out);
        left.encode(out);
        came_out.encode(out);
        pad.encode(out);
        completed.encode(out);
    }

    /// A show of `entry`, waiting for the fighter to be free.
    pub fn new(entry: MoveEntry) -> Self {
        let pads = |facing_right| {
            let mut frames = entry.input.frames(facing_right, Strength::Medium);
            if entry.recognizer == Recognizer::FamilyMove
                && entry.produces == crate::era_fight::sf5::SHIFT_BREAK
                && entry.input.button == ButtonClass::ShiftPair
            {
                // Holding HP + MK requests the follow-up when the shift
                // actually evades the scripted strike.
                for frame in frames.iter_mut().skip(1).take(18) {
                    frame.buttons[2] = true;
                    frame.buttons[4] = true;
                }
            }
            frames
        };
        Show {
            entry,
            phase: Phase::Waiting { frames: 0, strike_ticks: 0 },
            right: pads(true),
            left: pads(false),
            came_out: false,
            pad: None,
            completed: false,
        }
    }

    /// Whether the show ended because the move came out and ended, not
    /// because it was given up (a new press, a knockout, a time limit, the
    /// fight loop left). A finished show goes on as a Try (ML5).
    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn entry(&self) -> &MoveEntry {
        &self.entry
    }

    /// Whether the input is being fed (not waiting, not finishing).
    pub fn feeding(&self) -> bool {
        matches!(self.phase, Phase::Feeding { .. })
    }

    /// Whether the show is still waiting for the fighter.
    pub fn waiting(&self) -> bool {
        matches!(self.phase, Phase::Waiting { .. })
    }

    /// What the strip draws now.
    pub fn strip(&self) -> Strip {
        let input = &self.entry.input;
        let steps = input.strip_steps();
        match self.phase {
            Phase::Waiting { .. } => Strip::Wait,
            Phase::Feeding { fed } => Strip::Input { lit: input.lit_at(fed.saturating_sub(1)), steps },
            Phase::Finishing { .. } => Strip::Input { lit: steps.len(), steps },
        }
    }

    /// The input for the next frame, from the fighters as the last frame
    /// left them (`me` is the player's, `foe` the other side), the way the
    /// player's fighter faces, the pad's input this frame and whether the
    /// fight loop is running; `None` once the show is over and the pad has
    /// control again.
    pub fn input(
        &mut self,
        me: &FighterRead,
        foe: &FighterRead,
        facing_right: bool,
        pad: PlayerInputs,
        in_fight: bool,
    ) -> Option<PlayerInputs> {
        if !in_fight || me.knocked_out() || foe.knocked_out() {
            return None;
        }
        self.came_out |= me.present && me.move_id == self.entry.produces;
        loop {
            match self.phase {
                Phase::Waiting { frames, strike_ticks } => {
                    // A new press while waiting is the player taking the pad back.
                    let before = self.pad.replace(pad);
                    if before.is_some_and(|was| newly_pressed(&was, &pad)) || frames >= WAIT_LIMIT {
                        return None;
                    }
                    let contact = practice_contact(&self.entry);
                    let strike_ticks = if foe.attacking() { strike_ticks.saturating_add(1) } else { 0 };
                    // The scripted heavy kick for V-Shift Break starts later than a punch.
                    let strike_startup = if self.entry.input.button == ButtonClass::ShiftPair {
                        18
                    } else {
                        9
                    };
                    let can_start = match contact {
                        Some(PracticeContact::Guard) => me.blockstun && me.guarding(),
                        Some(PracticeContact::Strike) => ready(me) && strike_ticks >= strike_startup,
                        Some(PracticeContact::Counter) => ready(me) && strike_ticks != 0,
                        None => ready(me),
                    };
                    if can_start {
                        // The move must come out of this input, not an earlier one.
                        self.came_out = false;
                        self.phase = Phase::Feeding { fed: 0 };
                        continue;
                    }
                    self.phase = Phase::Waiting { frames: frames + 1, strike_ticks };
                    let mut input = PlayerInputs::default();
                    if contact == Some(PracticeContact::Guard) && foe.attacking() {
                        input.directions = [!facing_right, facing_right, false, false];
                    }
                    return Some(input);
                }
                Phase::Feeding { fed } => {
                    let frames = if facing_right { &self.right } else { &self.left };
                    let Some(&input) = frames.get(fed) else {
                        self.phase = Phase::Finishing { frames: 0 };
                        continue;
                    };
                    self.phase = Phase::Feeding { fed: fed + 1 };
                    return Some(input);
                }
                Phase::Finishing { frames } => {
                    // The move has come out and ended, or never came out.
                    // A Super can finish recovering one frame before the
                    // engine replaces its move ID. Return that frame's pad.
                    let ended = self.came_out
                        && (me.move_id != self.entry.produces
                            || (self.entry.kind == MoveKind::Super && gate_free(me)));
                    let limit = if self.came_out { MOVE_LIMIT } else { COME_OUT_LIMIT };
                    if ended || frames >= limit {
                        self.completed = self.came_out;
                        return None;
                    }
                    self.phase = Phase::Finishing { frames: frames + 1 };
                    return Some(PlayerInputs::default());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::REGISTRY;

    fn standing() -> FighterRead {
        FighterRead { present: true, health: 0x30, move_id: 1, state: 0, ..FighterRead::default() }
    }

    fn move_of(id: u8, state: u8) -> FighterRead {
        FighterRead { move_id: id, state, ..standing() }
    }

    #[test]
    fn the_gate_frees_the_fighter_by_the_state_byte_or_the_move() {
        for state in [0x00, 0x01, 0x08, 0x09] {
            assert!(gate_free(&move_of(5, state)), "{state:#x}");
        }
        assert!(!gate_free(&move_of(0x1a, 0x91)));
        assert!(!gate_free(&move_of(9, 0xc0)));
        assert!(gate_free(&move_of(0xc, 0xc0)), "move 0xc");
        // Free is not standing: a crouch is free and not ready.
        assert!(!ready(&move_of(5, 0)));
        assert!(ready(&standing()));
    }

    #[test]
    fn it_waits_with_no_input_until_the_fighter_is_free_and_standing() {
        let mut show = Show::new(REGISTRY[0]);
        let jumping = move_of(9, 0x91);
        for _ in 0..5 {
            assert_eq!(show.input(&jumping, &standing(), true, PlayerInputs::default(), true), Some(PlayerInputs::default()));
            assert!(show.waiting());
            assert_eq!(show.strip(), Strip::Wait);
        }
        // Standing: the first frame of the input is fed at once.
        let first = show.input(&standing(), &standing(), true, PlayerInputs::default(), true).unwrap();
        assert_eq!(first, REGISTRY[0].input.frames(true, Strength::Medium)[0]);
        assert!(show.feeding());
    }

    #[test]
    fn it_feeds_every_frame_then_hands_control_back_when_the_move_ends() {
        let entry = REGISTRY[0];
        let frames = entry.input.frames(true, Strength::Medium);
        let mut show = Show::new(entry);
        let me = standing();
        for (i, expected) in frames.iter().enumerate() {
            assert_eq!(show.input(&me, &me, true, PlayerInputs::default(), true), Some(*expected), "frame {i}");
            let Strip::Input { lit, steps } = show.strip() else { panic!("the input") };
            assert_eq!((lit, steps.len()), (entry.input.lit_at(i), 4));
        }
        // The move comes out, runs, and ends; neutral is fed meanwhile.
        let busy = move_of(entry.produces, 0x91);
        assert_eq!(show.input(&busy, &me, true, PlayerInputs::default(), true), Some(PlayerInputs::default()));
        assert_eq!(show.input(&busy, &me, true, PlayerInputs::default(), true), Some(PlayerInputs::default()));
        assert_eq!(show.input(&standing(), &me, true, PlayerInputs::default(), true), None);
    }

    #[test]
    fn only_a_show_that_ran_its_course_is_completed() {
        let entry = REGISTRY[0];
        let me = standing();
        let run_to_the_end = |came_out: bool| {
            let mut show = Show::new(entry);
            for _ in 0..entry.input.frames(true, Strength::Medium).len() {
                show.input(&me, &me, true, PlayerInputs::default(), true);
            }
            if came_out {
                let out = move_of(entry.produces, 0x91);
                show.input(&out, &me, true, PlayerInputs::default(), true);
            }
            let after = if came_out { standing() } else { me };
            let mut n = 0;
            while show.input(&after, &me, true, PlayerInputs::default(), true).is_some() {
                n += 1;
                assert!(n < 1000);
            }
            show
        };
        assert!(run_to_the_end(true).completed(), "the move came out and ended");
        assert!(!run_to_the_end(false).completed(), "the move never came out");
        // Given up on, as by a knockout.
        let mut show = Show::new(entry);
        let down = FighterRead { move_id: 0x41, ..standing() };
        assert_eq!(show.input(&down, &standing(), true, PlayerInputs::default(), true), None);
        assert!(!show.completed());
    }

    #[test]
    fn a_long_move_is_waited_out_past_the_come_out_limit() {
        let entry = REGISTRY[2];
        let mut show = Show::new(entry);
        let me = standing();
        for _ in 0..entry.input.frames(true, Strength::Medium).len() {
            show.input(&me, &me, true, PlayerInputs::default(), true);
        }
        let spinning = move_of(entry.produces, 0xc1);
        for _ in 0..COME_OUT_LIMIT + 30 {
            assert!(show.input(&spinning, &me, true, PlayerInputs::default(), true).is_some());
        }
        assert_eq!(show.input(&standing(), &me, true, PlayerInputs::default(), true), None);
    }

    #[test]
    fn it_feeds_the_mirror_when_the_fighter_faces_left() {
        let mut show = Show::new(REGISTRY[0]);
        let left = REGISTRY[0].input.frames(false, Strength::Medium);
        assert_eq!(show.input(&standing(), &standing(), false, PlayerInputs::default(), true), Some(left[0]));
        let mut other = Show::new(REGISTRY[0]);
        for _ in 0..4 {
            other.input(&standing(), &standing(), true, PlayerInputs::default(), true);
        }
        // Facing changes mid-feed: the next frame is the mirror's.
        assert_eq!(other.input(&standing(), &standing(), false, PlayerInputs::default(), true), Some(left[4]));
    }

    #[test]
    fn a_move_that_never_comes_out_ends_the_show_after_its_limit() {
        let mut show = Show::new(REGISTRY[0]);
        let me = standing();
        let fed = REGISTRY[0].input.frames(true, Strength::Medium).len();
        for _ in 0..fed + COME_OUT_LIMIT as usize {
            assert!(show.input(&me, &me, true, PlayerInputs::default(), true).is_some());
        }
        assert_eq!(show.input(&me, &me, true, PlayerInputs::default(), true), None);
    }

    #[test]
    fn a_knockout_or_a_wait_that_never_ends_gives_the_pad_back() {
        let mut show = Show::new(REGISTRY[0]);
        let down = FighterRead { move_id: 0x41, ..standing() };
        assert_eq!(show.input(&down, &standing(), true, PlayerInputs::default(), true), None);
        assert_eq!(show.input(&standing(), &down, true, PlayerInputs::default(), true), None);
        let mut show = Show::new(REGISTRY[0]);
        let jumping = move_of(9, 0x91);
        for _ in 0..WAIT_LIMIT {
            assert!(show.input(&jumping, &standing(), true, PlayerInputs::default(), true).is_some());
        }
        assert_eq!(show.input(&jumping, &standing(), true, PlayerInputs::default(), true), None);
    }

    #[test]
    fn a_new_press_while_waiting_returns_the_pad_but_a_held_one_does_not() {
        let jumping = move_of(9, 0x91);
        let held = PlayerInputs { buttons: [false, false, false, true, false, false], ..PlayerInputs::default() };
        // LK held from the menu's confirm: the baseline, no cancel.
        let mut show = Show::new(REGISTRY[0]);
        for _ in 0..5 {
            assert!(show.input(&jumping, &standing(), true, held, true).is_some());
        }
        // Released, then a new direction: cancelled.
        assert!(show.input(&jumping, &standing(), true, PlayerInputs::default(), true).is_some());
        let block = PlayerInputs { directions: [false, true, false, false], ..PlayerInputs::default() };
        assert_eq!(show.input(&jumping, &standing(), true, block, true), None);
        // Once feeding, the pad is ignored.
        let mut show = Show::new(REGISTRY[0]);
        show.input(&standing(), &standing(), true, PlayerInputs::default(), true);
        assert!(show.feeding());
        assert!(show.input(&standing(), &standing(), true, block, true).is_some());
    }

    #[test]
    fn the_show_ends_when_the_flow_leaves_the_fight_loop() {
        let mut show = Show::new(REGISTRY[0]);
        assert!(show.input(&standing(), &standing(), true, PlayerInputs::default(), true).is_some());
        assert_eq!(show.input(&standing(), &standing(), true, PlayerInputs::default(), false), None);
    }

    #[test]
    fn a_move_already_out_when_the_input_begins_does_not_count() {
        let entry = REGISTRY[0];
        let mut show = Show::new(entry);
        // Out while waiting (busy), then the fighter is free and standing.
        let out = move_of(entry.produces, 0x91);
        show.input(&out, &standing(), true, PlayerInputs::default(), true);
        show.input(&standing(), &standing(), true, PlayerInputs::default(), true);
        assert!(show.feeding());
        // The input is fed and the move never comes out: the show must not
        // treat the earlier one as having ended.
        let fed = entry.input.frames(true, Strength::Medium).len();
        for _ in 1..fed {
            show.input(&standing(), &standing(), true, PlayerInputs::default(), true);
        }
        let mut frames = 0;
        while show.input(&standing(), &standing(), true, PlayerInputs::default(), true).is_some() {
            frames += 1;
        }
        assert_eq!(frames, COME_OUT_LIMIT, "ended by the come-out limit, not as a move that ended");
    }
}

