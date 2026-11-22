//! The SFZ era style's game plans (SC16, `#208`; spec `#192`): every
//! fighter who can be picked in it plays the stronger Fair CPU's game, as
//! Ryu does in the SF6 era style.
//!
//! Each fighter's table holds the plans the whole cast shares (Ryu SF6's
//! poke, routes, pressure, knockdown play and jump-in: they press buttons,
//! not named moves, and each normal's reach is learned in the fight; the
//! poke's normal is the fighter's own), the projectile plans for those whose projectile crosses the
//! screen, the era style's Custom Combo, and the plans for the fighter's
//! own kind of move: a command throw, a teleport, a charge move or a change
//! of stance. The style's other tools are answers to what the CPU sees and
//! are chosen outside the plans, under the same reaction delays: an Alpha
//! Counter out of a guard and a guard in the air (`Tactics::react`), and a
//! super on a hit it sees, on its wake-up, or inside a route of its book,
//! at the level its meter allows.

use super::super::habit::Lean;
use super::super::{button, Choice, Level, DOWN};
use super::{always, step, Act, GamePlan, Situation, Step, ANY, JUMP_IN_GAP, SHOT_GAP, WATCHED_SHOT_GAPS};
use super::{RYU_SF6_BAIT, RYU_SF6_FRAME_TRAP, RYU_SF6_JUMP_IN, RYU_SF6_ROUTE, RYU_SF6_SHOT, RYU_SF6_SHOT_WATCH, RYU_SF6_WALK_THROW};
use crate::moves::{ButtonClass, Dir, MoveEntry, MoveKind};
use crate::roster::TournamentFighter;

/// A kind of move some fighters have and others do not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signature {
    /// A special that grabs: walked in to close range first.
    CommandThrow,
    /// A special that moves him across the screen at once.
    Teleport,
    /// A special entered by holding a direction (or two buttons) first.
    Charge,
}

/// One of the era style's own tools a game plan can step into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EraAct {
    /// Start Custom Combo (the SFZ3 V-ism), walk in and chain attacks.
    CustomCombo,
    /// Go on chaining while Custom Combo lasts.
    CustomChain,
    /// Change stance (Gen's Mantis and Crane).
    Stance,
}

/// Its fighter's moves of each [`Signature`] kind, from the moves the fight
/// gives him now, where the SFZ era style is in force.
#[derive(Clone, Copy, Debug, Default)]
pub struct Signatures {
    pub command_throw: Option<MoveEntry>,
    pub teleport: Option<MoveEntry>,
    pub charge: Option<MoveEntry>,
}

/// The command throws and the teleport by name: their inputs look like any
/// other special's.
const COMMAND_THROWS: [&str; 3] = ["MURDERER CHAIN", "IZUNA SHOULDER THROW", "EXECUTION DROP"];
const TELEPORTS: [&str; 1] = ["UTSUSEMI"];

impl Signatures {
    pub(crate) fn of(entries: &[MoveEntry]) -> Signatures {
        let ground = |entry: &&MoveEntry| entry.kind == MoveKind::Special && entry.input.motion.first() != Some(&Dir::Jump);
        let named = |names: &[&str]| entries.iter().filter(ground).find(|entry| names.contains(&entry.name)).copied();
        let charge = entries
            .iter()
            .filter(ground)
            .find(|entry| entry.input.motion.iter().any(|dir| dir.is_charge()) || entry.input.button == ButtonClass::ReleasePair)
            .copied();
        Signatures { command_throw: named(&COMMAND_THROWS), teleport: named(&TELEPORTS), charge }
    }

    pub(crate) fn get(&self, kind: Signature) -> Option<MoveEntry> {
        match kind {
            Signature::CommandThrow => self.command_throw,
            Signature::Teleport => self.teleport,
            Signature::Charge => self.charge,
        }
    }
}

/// The gap a command throw walks in to before its motion: as near as two
/// fighters stand in the measured fights (about 14 to 16 pixels from its
/// centre to his hurtbox; a nearer mark is never reached).
pub(in crate::fair_cpu) const THROW_SPACE: i16 = 16;
/// The farthest a command throw is walked in from: past it the plan ends.
pub(in crate::fair_cpu) const THROW_REACH: i16 = 56;
/// The gaps a charge move is entered from: it carries him in from the
/// middle distance.
const CHARGE_GAPS: (i16, i16) = (60, 170);
/// The gaps the teleport is taken from: from far off, past a jump-in's
/// reach (from 90 it lost Geki matches to the old CPU; from 160 the gap
/// hardly ever came up).
const TELEPORT_GAPS: (i16, i16) = (130, i16::MAX);
/// The gap Custom Combo is started from: close enough that the chain's
/// first attacks meet him after a short walk.
pub(in crate::fair_cpu) const CUSTOM_GAP: i16 = 60;

/// After a guarded attack (his guard is still up), or up close: walk in
/// and grab him with the command throw, the walk and the motion one action
/// as the plain throw's are (a walk of its own was cut short by any button
/// he pressed). On a hit the pressure goes on from the knockdown's plans.
const COMMAND_THROW: [Step; 1] = [step(Act::Signature(Signature::CommandThrow), always(None))];

/// From afar: across the screen at once, then whatever the situation he
/// lands in asks for.
const TELEPORT: [Step; 1] = [step(Act::Signature(Signature::Teleport), always(None))];

/// From the middle distance: the charge move that carries him in (the
/// charge is held walking back, which also guards).
const CHARGE: [Step; 1] = [step(Act::Signature(Signature::Charge), always(None))];

/// From afar: change stance, for the other stance's moves.
const STANCE: [Step; 1] = [step(Act::Era(EraAct::Stance), always(None))];

/// Up close with the meter for it: start Custom Combo and chain attacks
/// for as long as it lasts.
const CUSTOM_COMBO: [Step; 3] = [
    step(Act::Era(EraAct::CustomCombo), always(Some(1))),
    step(Act::Era(EraAct::CustomChain), always(Some(2))),
    step(Act::Era(EraAct::CustomChain), always(None)),
];

const fn plan(situation: Situation, score: i32, corner_bonus: i32, gap: (i16, i16), steps: &'static [Step], lean: Lean, from: Level) -> GamePlan {
    GamePlan { situation, score, corner_bonus, cornered_only: false, gap, steps, lean, from }
}

/// Close neutral: walk to this normal's reach and press it; then as Ryu
/// SF6's poke ([`super::RYU_SF6_POKE`]) with this normal in place of his
/// crouching medium kick.
const fn poke(crouch: bool, button: usize) -> [Step; 6] {
    [
        step(Act::Reach { crouch, button }, always(Some(1))),
        step(Act::Do(Choice::Normal { crouch, button }), [Some(2), Some(3), Some(4), None, None]),
        step(Act::Meaty { crouch, button }, [None, None, None, None, Some(0)]),
        step(Act::Do(Choice::Throw), always(None)),
        step(Act::Do(Choice::Retreat), always(Some(5))),
        step(Act::Do(Choice::Guard), always(None)),
    ]
}

/// A knocked-down opponent: this normal timed to his rise, as Ryu SF6's
/// ([`super::RYU_SF6_MEATY`]).
const fn meaty(crouch: bool, button: usize) -> [Step; 4] {
    [
        step(Act::Meaty { crouch, button }, [None, Some(1), Some(2), None, None]),
        step(Act::Do(Choice::Throw), always(None)),
        step(Act::Do(Choice::Retreat), always(Some(3))),
        step(Act::Do(Choice::Guard), always(None)),
    ]
}

// Each fighter's poke, and whether he walks a cornered opponent down,
// chosen by the measuring run: for each fighter, four normals with and
// without the walk-down, 160 matches a side against the old Fair CPU at
// each level and against the level below, keeping a set that met every bar.
// Ryu's SF6 poke, the crouching medium kick, won as often in the SFZ style
// but left him standing more than the old CPU.
const POKE_ST_HK: [Step; 6] = poke(false, 5);
const MEATY_ST_HK: [Step; 4] = meaty(false, 5);
const POKE_ST_MK: [Step; 6] = poke(false, 4);
const MEATY_ST_MK: [Step; 4] = meaty(false, 4);
const POKE_ST_MP: [Step; 6] = poke(false, 1);
const MEATY_ST_MP: [Step; 4] = meaty(false, 1);
const POKE_ST_LK: [Step; 6] = poke(false, 3);
const MEATY_ST_LK: [Step; 4] = meaty(false, 3);

/// The plans the whole cast shares, scored as Ryu's in the SF6 era style,
/// with the fighter's own poke and the attack he times to a rise. Ryu SF6's
/// hover at the edge of reach is left out: in the measuring run it lost
/// matches for every SFZ fighter and left him standing more.
macro_rules! shared {
    ($poke:expr, $meaty:expr $(, $more:expr)* $(,)?) => {
        &[
            plan(Situation::NeutralClose, 60, 0, ANY, $poke, Lean::None, Level::Easy),
            plan(Situation::NeutralClose, 62, 6, ANY, &RYU_SF6_ROUTE, Lean::None, Level::Easy),
            plan(Situation::ItWasBlocked, 60, 4, ANY, &RYU_SF6_FRAME_TRAP, Lean::FrameTrap, Level::Normal),
            plan(Situation::ItWasBlocked, 56, 12, ANY, &RYU_SF6_WALK_THROW, Lean::WalkThrow, Level::Easy),
            plan(Situation::OpponentKnockedDown, 62, 8, ANY, $meaty, Lean::Meaty, Level::Normal),
            plan(Situation::OpponentKnockedDown, 52, -10, ANY, &RYU_SF6_BAIT, Lean::Bait, Level::VeryHard),
            plan(Situation::NeutralFar, 40, 0, JUMP_IN_GAP, &RYU_SF6_JUMP_IN, Lean::JumpIn, Level::Easy),
            // Only with the V-ism's meter to start it (SC16); then above the
            // poke and the route even once they have kept working (the
            // hardest levels add up to 24 to those).
            plan(Situation::NeutralClose, 90, 4, (i16::MIN, CUSTOM_GAP), &CUSTOM_COMBO, Lean::None, Level::Normal),
            $($more),*
        ]
    };
}

/// A cornered opponent out of reach: walk him down with the poke, as Ryu
/// SF6 does. Lost Retsu, Joe, Lee and Adon matches (Adon's Normal over Easy
/// went from 230 to 439 of 480 without it), so their tables leave it out.
const fn walk_down(poke: &'static [Step]) -> GamePlan {
    GamePlan { cornered_only: true, ..plan(Situation::NeutralFar, 60, 0, ANY, poke, Lean::None, Level::Normal) }
}

/// A projectile from afar, and the watch for a jump over it.
const SHOT_PLAN: GamePlan = plan(Situation::NeutralFar, 70, 0, (SHOT_GAP, i16::MAX), &RYU_SF6_SHOT, Lean::Shot, Level::Easy);
const SHOT_WATCH_PLAN: GamePlan = plan(Situation::NeutralFar, 0, 0, WATCHED_SHOT_GAPS, &RYU_SF6_SHOT_WATCH, Lean::ShotWatch, Level::Normal);
const COMMAND_THROW_BLOCKED: GamePlan = plan(Situation::ItWasBlocked, 58, 8, (i16::MIN, THROW_REACH), &COMMAND_THROW, Lean::WalkThrow, Level::Normal);
/// Scored with the poke and the route: lower, it was hardly ever taken
/// (Retsu, whose only grab plan this is, never in 12,000 frames).
const COMMAND_THROW_CLOSE: GamePlan = plan(Situation::NeutralClose, 62, 4, (i16::MIN, THROW_REACH), &COMMAND_THROW, Lean::None, Level::Hard);
const TELEPORT_PLAN: GamePlan = plan(Situation::NeutralFar, 56, -6, TELEPORT_GAPS, &TELEPORT, Lean::None, Level::Normal);
const CHARGE_PLAN: GamePlan = plan(Situation::NeutralFar, 58, 0, CHARGE_GAPS, &CHARGE, Lean::None, Level::Easy);
/// Lee's Firewater only from beyond a jump-in's range: charged nearer, it
/// lost him matches to the old CPU and left him standing at the wall.
const FAR_CHARGE_PLAN: GamePlan = plan(Situation::NeutralFar, 58, 0, (150, i16::MAX), &CHARGE, Lean::None, Level::Easy);
const STANCE_PLAN: GamePlan = plan(Situation::NeutralFar, 50, -10, ANY, &STANCE, Lean::None, Level::Easy);

const RYU: &[GamePlan] = shared!(&POKE_ST_HK, &MEATY_ST_HK, walk_down(&POKE_ST_HK), SHOT_PLAN, SHOT_WATCH_PLAN);
const KEN: &[GamePlan] = shared!(&POKE_ST_HK, &MEATY_ST_HK, walk_down(&POKE_ST_HK), SHOT_PLAN, SHOT_WATCH_PLAN);
const SAGAT: &[GamePlan] = shared!(&POKE_ST_MP, &MEATY_ST_MP, walk_down(&POKE_ST_MP), SHOT_PLAN, SHOT_WATCH_PLAN);
/// Retsu's Kiai Shot falls short (the single choices keep it for close
/// range). His Izuna Shoulder Throw grabs up close, scored as the others'
/// grab, where he hardly ever takes it: scored to be taken (64 and up), or
/// after a guarded attack, it lost him matches to the old CPU at Very
/// Difficult (SC16 lists him for this).
const RETSU: &[GamePlan] = shared!(&POKE_ST_MP, &MEATY_ST_MP, COMMAND_THROW_CLOSE);
/// Geki: Execution Drop grabs, Utsusemi teleports.
const GEKI: &[GamePlan] = shared!(&POKE_ST_MP, &MEATY_ST_MP, walk_down(&POKE_ST_MP), COMMAND_THROW_BLOCKED, COMMAND_THROW_CLOSE, TELEPORT_PLAN);
/// Birdie: the Murderer Chain grabs, the Bull Head is charged.
const BIRDIE: &[GamePlan] = shared!(&POKE_ST_MP, &MEATY_ST_MP, walk_down(&POKE_ST_MP), COMMAND_THROW_BLOCKED, COMMAND_THROW_CLOSE, CHARGE_PLAN);
/// Lee: Firewater is charged.
const LEE: &[GamePlan] = shared!(&POKE_ST_MK, &MEATY_ST_MK, FAR_CHARGE_PLAN);
/// Gen: the Mantis and Crane stances (Crane's Jasen is charged).
const GEN: &[GamePlan] = shared!(&POKE_ST_HK, &MEATY_ST_HK, walk_down(&POKE_ST_HK), STANCE_PLAN, CHARGE_PLAN);
const JOE: &[GamePlan] = shared!(&POKE_ST_MK, &MEATY_ST_MK);
const MIKE: &[GamePlan] = shared!(&POKE_ST_MP, &MEATY_ST_MP, walk_down(&POKE_ST_MP));
const EAGLE: &[GamePlan] = shared!(&POKE_ST_LK, &MEATY_ST_LK, walk_down(&POKE_ST_LK));
const ADON: &[GamePlan] = shared!(&POKE_ST_MP, &MEATY_ST_MP);

/// The game plans `fighter` has in the SFZ era style.
pub(super) fn plans(fighter: TournamentFighter) -> &'static [GamePlan] {
    use TournamentFighter::*;
    match fighter {
        Ryu => RYU,
        Ken => KEN,
        Sagat => SAGAT,
        Retsu => RETSU,
        Geki => GEKI,
        Birdie => BIRDIE,
        Lee => LEE,
        Gen => GEN,
        Joe => JOE,
        Mike => MIKE,
        Eagle => EAGLE,
        Adon => ADON,
    }
}

/// The pads that start Custom Combo (a punch and a kick of one strength
/// pressed together), then walk in and chain attacks, each a new button,
/// toward the opponent's side `forward`.
pub(in crate::fair_cpu) fn custom_combo_pads(forward: u16) -> Vec<u16> {
    let start = button(0) | button(3);
    let mut pads = vec![0, start, start, 0];
    // The start's freeze, then the chain.
    pads.extend([0; 8]);
    pads.extend(chain_pads(forward, 36));
    pads
}

/// `frames` of a chain played in Custom Combo: walking in, a new attack
/// every few frames, light to heavy, punches and kicks in turn, standing
/// and crouching.
pub(in crate::fair_cpu) fn chain_pads(forward: u16, frames: usize) -> Vec<u16> {
    const ORDER: [(bool, usize); 6] = [(false, 0), (true, 3), (false, 1), (true, 4), (false, 2), (true, 5)];
    (0..frames)
        .map(|frame| {
            let (crouch, press) = ORDER[(frame / 4) % ORDER.len()];
            let stick = if crouch { DOWN } else { forward };
            if frame % 4 == 1 { stick | button(press) } else { stick }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fair_cpu::game::PLAN_SLOTS;
    use crate::fair_cpu::{LEFT, RIGHT, UP};

    #[test]
    fn every_sfz_fighter_has_game_plans_for_every_situation_a_plan_is_for_and_one_that_takes_routes() {
        for fighter in TournamentFighter::ALL {
            let table = plans(fighter);
            assert!(table.len() <= PLAN_SLOTS, "{fighter:?}: {} plans", table.len());
            for situation in [Situation::NeutralFar, Situation::NeutralClose, Situation::ItWasBlocked, Situation::OpponentKnockedDown] {
                for level in [Level::Easy, Level::Normal, Level::Hard, Level::VeryHard] {
                    let open = level != Level::Easy || situation != Situation::OpponentKnockedDown;
                    let has = table.iter().any(|p| p.situation == situation && !p.cornered_only && p.for_level(level));
                    assert_eq!(has, open, "{fighter:?} {situation:?} {level:?}");
                }
            }
            assert!(table.iter().any(|p| p.steps.iter().any(|s| matches!(s.act, Act::Route(_)))), "{fighter:?} enters routes");
            for plan in table {
                for step in plan.steps {
                    for next in step.then.into_iter().flatten() {
                        assert!(usize::from(next) < plan.steps.len(), "{fighter:?}: {step:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_plans_for_a_kind_of_move_are_in_the_tables_of_the_fighters_who_have_one() {
        let uses = |fighter: TournamentFighter, act: Act| plans(fighter).iter().any(|p| p.steps.iter().any(|s| s.act == act));
        use TournamentFighter::*;
        for fighter in TournamentFighter::ALL {
            let throw = matches!(fighter, Retsu | Geki | Birdie);
            assert_eq!(uses(fighter, Act::Signature(Signature::CommandThrow)), throw, "{fighter:?} command throw");
            assert_eq!(uses(fighter, Act::Signature(Signature::Teleport)), fighter == Geki, "{fighter:?} teleport");
            assert_eq!(uses(fighter, Act::Signature(Signature::Charge)), matches!(fighter, Birdie | Lee | Gen), "{fighter:?} charge");
            assert_eq!(uses(fighter, Act::Era(EraAct::Stance)), fighter == Gen, "{fighter:?} stance");
            assert!(uses(fighter, Act::Era(EraAct::CustomCombo)), "{fighter:?} Custom Combo");
            assert_eq!(uses(fighter, Act::Shot), matches!(fighter, Ryu | Ken | Sagat), "{fighter:?} projectile");
        }
    }

    #[test]
    fn a_custom_combo_starts_with_a_punch_and_kick_of_one_strength_and_chains_new_buttons() {
        let pads = custom_combo_pads(RIGHT);
        assert!(pads.len() <= crate::fair_cpu::PLAN_CAP, "it fits one plan");
        assert_eq!(pads[1], button(0) | button(3));
        let presses: Vec<u16> = pads[12..].iter().map(|pad| pad & 0x3f0).filter(|&b| b != 0).collect();
        assert!(presses.len() >= 6);
        assert!(presses.windows(2).all(|w| w[0] != w[1]), "each attack a new button: {presses:x?}");
        assert!(pads[12..].iter().all(|pad| pad & (LEFT | UP) == 0), "never back or up");
    }
}
