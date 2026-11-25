//! Situations and game plans (SC2, `#194`; spec `#192`, ADR-0026): the
//! stronger Fair CPU's play, behind the developer-only switch
//! ([`crate::config::MatchConfig::stronger_cpu`]).
//!
//! **Situations.** Each frame the CPU is shown, through its reaction delay,
//! is judged to be one [`Situation`], with the corner as a modifier
//! ([`Judge::judge`]). Nothing but percepts goes in.
//!
//! **Game plans.** A [`GamePlan`] is a short script of [`Step`]s for one
//! situation. A step says what to do and which step follows each
//! [`Outcome`] the CPU then sees. The CPU's state holds only where it is in
//! a plan ([`Running`]); the plans themselves are tables here, looked up by
//! fighter and era style, so choosing and following one costs a few
//! comparisons a frame.

use super::cast::Tool;
use super::habit::Lean;
use super::{Body, Choice, Level, Percept};
use crate::era::EraFamily;
use crate::roster::TournamentFighter;
use crate::route_book::{Reach, Route, Stance};

mod sf3;
mod sf4;
mod sf5;
mod sfz;
pub(super) use sfz::{chain_pads, custom_combo_pads};

/// One of an era style's own tools a game plan can step into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EraAct {
    /// Start Custom Combo (the SFZ3 V-ism), walk in and chain attacks.
    CustomCombo,
    /// Go on chaining while Custom Combo lasts.
    CustomChain,
    /// Change Gen's stance from the Mantis to the Crane, never back.
    Stance,
    /// SF3's universal overhead, to open a crouching guard.
    Overhead,
    /// An EX special, when this revision has one and can pay for it.
    Enhanced,
    /// Charge and release an SF4 level-two Focus Attack.
    Focus,
    /// Spend Super meter on an SF4 finisher or an SF5 Critical Art.
    Super,
    /// Spend SF4 Revenge on the selected Ultra.
    Ultra,
    /// Use the selected SF5 V-Skill where its fighter's plan calls for it.
    VSkill,
    /// Activate SF5 V-Trigger or use an offensive move it unlocks.
    VTrigger,
}

/// The playfield's walls as a fighter's centre meets them, and how near one
/// counts as the corner. Measured by walking Ryu into each wall of
/// Training's stage in the SF6 era style; other stages are not checked.
const LEFT_WALL: i16 = 114;
const RIGHT_WALL: i16 = 400;
const CORNER: i16 = 40;

/// A reach to assume for the opponent's attacks until one is seen.
const DEFAULT_FOE_REACH: i16 = 66;
/// The farthest the edge of his reach is put, however far a travelling
/// special has carried an attack box: movement around the edge is for his
/// normals.
const EDGE_CAP: i16 = 100;
/// Frames from a knockdown showing to the opponent being free, until the
/// CPU has seen one: about a sweep's, as Training's Ryu shows it.
const DEFAULT_RISE: u16 = 43;
/// Frames one knockdown seen moves the CPU's idea of their length, at most.
const RISE_STEP: i64 = 3;
/// Frames before a rising opponent is free at which a timed attack is
/// pressed: a little under a quick normal's start-up, so it is out just
/// after he rises. (This engine lets a hit land on a knockdown's last
/// frame and knock him down again; six frames' lead met that frame.)
pub(super) const MEATY_LEAD: u32 = 4;
/// How far inside a timed attack's reach the SF2 era style's shimmy over
/// a knocked-down opponent steps back to before walking in again (SC15,
/// `#207`).
pub(super) const SHIMMY: i16 = 10;
/// Frames a guard held over a rise lasts past it, for a reversal to show.
pub(super) const BAIT_FRAMES: u32 = 24;
/// Frames before a rising opponent is free at which the guard over his
/// rise goes up, ending the walk in.
pub(super) const GUARD_LEAD: u32 = 4;
/// Frames "it was blocked" outlasts the last frame its attack was seen on a
/// guard: about the time a guarded normal takes to recover.
const BLOCKED_FRAMES: u8 = 14;
/// How far past its longest reach a recovering opponent still counts as
/// having whiffed in reach: a step forward closes it.
const WHIFF_MARGIN: i16 = 24;
/// Frames after its attack's input by which the CPU has seen what came of
/// it, if anything did.
const WATCH_FRAMES: u32 = 45;
/// The longest a game plan may take from its first step, in frames. A
/// plan still unfinished then is dropped.
pub(super) const PLAN_FRAMES: u32 = 300;

/// Which state the Fair CPU judges the fight to be in, from the frame it is
/// shown through its reaction delay. The corner is not one of these: it
/// modifies whichever holds ([`Judged::corner`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Situation {
    /// Outside both fighters' longest reach.
    #[default]
    NeutralFar,
    /// Inside someone's reach, nobody committed.
    NeutralClose,
    /// The opponent is in a hit reaction.
    ItHit,
    /// Its attack met a guard.
    ItWasBlocked,
    /// The opponent is knocked down or rising.
    OpponentKnockedDown,
    /// The opponent's attack missed and he is recovering in reach.
    OpponentWhiffed,
    /// The opponent's attack, jump or projectile is seen, or it is being hit.
    Defending,
    /// It is knocked down or rising.
    KnockedDown,
}

/// A judged frame: the situation, and whether either fighter is near a wall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Judged {
    pub situation: Situation,
    pub corner: bool,
    /// The opponent is the one near a wall.
    pub foe_cornered: bool,
}

/// Knocked down or rising: a hit reaction drawn off the ground (state bit 0,
/// as a sweep's and a launch's are), or the rise from a throw.
pub(super) fn knocked_down(body: &Body) -> bool {
    body.present && (body.move_id == crate::combo::throw::GET_UP_MOVE || (body.move_id >= 0x30 && body.state & 1 != 0))
}

fn cornered(body: &Body) -> bool {
    body.present && (body.x <= LEFT_WALL + CORNER || body.x >= RIGHT_WALL - CORNER)
}

/// How far from the walls of Training's stage a fighter must stand to have
/// room to walk back. Other stages' walls stand nearer: in the measuring
/// run's matches an SFZ fighter walking back met one at about x 375, 25 in
/// from Training's; this margin covers that, not a stage it has not seen.
const BACK_ROOM: i16 = 70;

/// The CPU's fighter has room behind it to walk back, as far as the walls
/// it knows go (SC16, `#208`): a charge is held walking back, and against a
/// wall it would stand still charging.
pub(super) fn room_behind(view: &Percept) -> bool {
    if view.foe.x >= view.me.x { view.me.x > LEFT_WALL + BACK_ROOM } else { view.me.x < RIGHT_WALL - BACK_ROOM }
}

/// What judging remembers from one seen frame to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Judge {
    /// The opponent's current attack has shown a live attack box, and that
    /// box has reached the CPU's fighter (or hit him).
    foe_attack_shown: bool,
    foe_attack_met: bool,
    /// The frame the opponent's current attack was first seen, and the
    /// step the CPU's latest spell of blockstun, as it feels its own
    /// fighter, began and the last step of it (SC15, `#207`): some attacks
    /// show no box it can read, and its guard pose alone is drawn as soon
    /// as an attack is near, so a guarded one is known by the blockstun it
    /// put it in.
    foe_attack_began: u32,
    blockstun_from: u32,
    blockstun_at: u32,
    /// The opponent's current attack began on the ground: one that rises
    /// (a Shoryuken) is still a whiff to punish as it falls.
    foe_attack_grounded: bool,
    /// The longest any of the opponent's attack boxes has been seen to reach.
    foe_reach: i16,
    /// How far his ground attacks have been seen to reach from where he
    /// stood as each began (a step in the move counted), with no guess
    /// before one is seen (0 until then): the edge of his reach, for
    /// movement around it. And where he stood as his current one began.
    foe_reach_seen: i16,
    foe_attack_from: i16,
    /// Frames left of "it was blocked".
    blocked: u8,
    /// The opponent was knocked down on the last frame judged, the frame
    /// his knockdown was first seen, and how long a knockdown lasts until
    /// he is free, as the CPU has seen them last.
    foe_down: bool,
    foe_down_since: u32,
    rise_frames: u16,
}

impl Default for Judge {
    fn default() -> Self {
        Judge {
            foe_attack_shown: false,
            foe_attack_met: false,
            foe_attack_began: 0,
            blockstun_from: 0,
            blockstun_at: 0,
            foe_attack_grounded: false,
            foe_reach: DEFAULT_FOE_REACH,
            foe_reach_seen: 0,
            foe_attack_from: 0,
            blocked: 0,
            foe_down: false,
            foe_down_since: 0,
            rise_frames: DEFAULT_RISE,
        }
    }
}

impl Judge {
    /// Judge `view`, the frame after `before`. `my_reach` is the CPU's
    /// longest normal as far as it knows.
    pub(super) fn judge(&mut self, view: Percept, before: Option<Percept>, my_reach: i16) -> Judged {
        let (me, foe) = (view.me, view.foe);
        let foe_was = before.map(|b| b.foe);
        if foe.attacking() {
            if foe_was.is_none_or(|was| !was.attacking() || was.move_id != foe.move_id) {
                self.foe_attack_shown = false;
                self.foe_attack_met = false;
                self.foe_attack_began = view.frame;
                self.foe_attack_grounded = foe_was.is_some_and(|was| !was.jumping());
                self.foe_attack_from = foe_was.map_or(foe.x, |was| was.x);
            }
            if foe.active {
                self.foe_attack_shown = true;
                let toward_me = if me.x >= foe.x { 1 } else { -1 };
                let reach = ((foe.front - foe.x) * toward_me).clamp(16, 120);
                self.foe_reach = self.foe_reach.max(reach);
                if self.foe_attack_grounded && !foe.jumping() {
                    let from_start = ((foe.front - self.foe_attack_from) * toward_me).clamp(16, EDGE_CAP);
                    self.foe_reach_seen = self.foe_reach_seen.max(from_start);
                }
                // The box's forward edge at or past the near edge of the
                // CPU's hurtbox: it reached him, guarded or not.
                let near = if toward_me > 0 { me.hurt.0 } else { me.hurt.1 };
                if (foe.front - near) * toward_me >= 0 {
                    self.foe_attack_met = true;
                }
            }
            if me.reacting() {
                self.foe_attack_met = true;
            }
        } else {
            self.foe_attack_shown = false;
            self.foe_attack_met = false;
        }
        // A guard pose alone is not a guarded attack: the pose is drawn as
        // soon as an attack is near. The attack must also be in reach.
        if me.attacking() && !me.jumping() && foe.guarding() && view.gap() <= my_reach + 8 {
            self.blocked = BLOCKED_FRAMES;
        } else if foe.attacking() || foe.reacting() || me.reacting() {
            self.blocked = 0;
        } else {
            self.blocked = self.blocked.saturating_sub(1);
        }

        // A knockdown's length, from the first frame it shows to the first
        // frame he is free, learned as a person learns it by watching.
        let down = knocked_down(&foe);
        if down && !self.foe_down {
            self.foe_down_since = view.frame;
        } else if !down && self.foe_down && foe.free() {
            let seen = view.frame.saturating_sub(self.foe_down_since);
            if (20..=150).contains(&seen) {
                // The usual length: one sighting moves it a few frames at
                // most, so an odd long knockdown (about one in five lasts
                // twice the usual) does not throw the timing off.
                let shift = (i64::from(seen) - i64::from(self.rise_frames)).clamp(-RISE_STEP, RISE_STEP);
                self.rise_frames = (i64::from(self.rise_frames) + shift) as u16;
            }
        }
        self.foe_down = down;

        let situation = if knocked_down(&me) {
            Situation::KnockedDown
        } else if me.reacting() {
            Situation::Defending
        } else if knocked_down(&foe) {
            Situation::OpponentKnockedDown
        } else if foe.reacting() {
            Situation::ItHit
        } else if self.blocked > 0 {
            Situation::ItWasBlocked
        } else if foe.attacking()
            && !foe.active
            && (!foe.jumping() || self.foe_attack_grounded)
            && self.foe_attack_shown
            && !self.foe_attack_met
            && view.gap() <= my_reach + WHIFF_MARGIN
        {
            Situation::OpponentWhiffed
        } else if view.threat().is_some() {
            Situation::Defending
        } else if view.gap() > my_reach.max(self.foe_reach) {
            Situation::NeutralFar
        } else {
            Situation::NeutralClose
        };
        Judged { situation, corner: cornered(&me) || cornered(&foe), foe_cornered: cornered(&foe) }
    }

    /// The CPU felt its own fighter in blockstun on machine step `step`.
    pub(super) fn felt_blockstun(&mut self, step: u32) {
        if step != self.blockstun_at + 1 {
            self.blockstun_from = step;
        }
        self.blockstun_at = step;
    }

    /// The opponent's ground attack in `view`, the frame last judged, has
    /// met the CPU's fighter and is no longer live (SC15, `#207`): its box
    /// reached it and has gone; or, for an attack that shows no box it can
    /// read, a spell of blockstun began after it began, so it is the one
    /// that met it and its start-up is behind it (not a spell from the
    /// attack before, still running as it began).
    pub(super) fn recovering_after_meeting(&self, view: Percept) -> bool {
        let boxed = self.foe_attack_shown && self.foe_attack_met && !view.foe.active;
        let boxless = !self.foe_attack_shown && self.blockstun_at != 0 && self.blockstun_from >= self.foe_attack_began;
        (boxed || boxless) && self.foe_attack_grounded && view.foe.attacking() && !view.foe.jumping()
    }

    /// The edge of the opponent's reach: the farthest his ground attacks
    /// have been seen to reach from where he stood as each began (capped at
    /// [`EDGE_CAP`]), or a guess until one is seen.
    pub(super) fn foe_edge(&self) -> i16 {
        if self.foe_reach_seen > 0 { self.foe_reach_seen } else { DEFAULT_FOE_REACH }
    }

    /// Something is coming at the CPU in `view`, the frame last judged: a
    /// projectile in front of it, or an attack whose box has not yet come
    /// and gone (its start-up or its active frames, not its recovery).
    pub(super) fn coming(&self, view: Percept) -> bool {
        // An attack whose box has been shown and is gone is in recovery.
        let recovering = self.foe_attack_shown && !view.foe.active;
        view.shot_coming() || (view.foe.attacking() && !recovering)
    }

    /// The machine frame a knocked-down opponent is expected to be free on,
    /// while he is down.
    pub(super) fn rise_at(&self) -> Option<u32> {
        self.foe_down.then(|| self.foe_down_since + u32::from(self.rise_frames))
    }
}

/// What the CPU sees come of a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Hit,
    Guard,
    Whiff,
    Jumped,
    Nothing,
}

/// What a step does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Act {
    /// Walk in, once, as far as it takes for this normal to reach where
    /// the opponent was last seen. The next step follows wherever he is.
    Reach { crouch: bool, button: usize },
    /// One of the CPU's actions, pressed as it presses it today.
    Do(Choice),
    /// A route from the fighter's route book that starts this way, entered
    /// whole from its opener: its first cancel is committed before any hit
    /// is seen. With no route allowed, the plan ends.
    Route(Reach),
    /// Against a knocked-down opponent, walk in while he is down and press
    /// this normal so it is out as he rises, from the knockdown's length as
    /// the CPU has seen it. Out of its reach when the moment comes, nothing
    /// is pressed (the outcome is `Nothing`).
    Meaty { crouch: bool, button: usize },
    /// Walk in while he is down, then hold a guard over the opponent's
    /// rise, so a reversal meets it and can be punished.
    GuardRise,
    /// Walk, in or out, to stand just outside the opponent's reach as the
    /// CPU has seen it ([`EDGE_OUT`] past it). Never back out from a
    /// cornered opponent.
    Edge,
    /// A short walk in, across the edge of his reach, or back out of it:
    /// an opening he may swing at and whiff. Never back out from a
    /// cornered opponent.
    Step { forward: bool },
    /// Its projectile, when none of its own is on screen; otherwise
    /// nothing is pressed (the outcome is `Nothing`).
    Shot,
    /// Walk, in or out, to this gap from the opponent: the spacing the
    /// next step works from. Never back out from a cornered opponent.
    Space(i16),
    /// The era style's dash that spends meter to close in (SF6's Drive
    /// Rush, SC8 `#200`). Without the gauge for it the plan ends.
    Rush,
    /// One of the fighter's own tools (SC15, `#207`): a command throw, a
    /// teleport or a charge move. A plan with such a
    /// step is taken only by a fighter who has the tool
    /// ([`GamePlan::playable`]).
    Tool(Tool),
    /// One of the era style's own tools (SC16, `#208`): starting Custom
    /// Combo and the chain played in it, a change of stance. A plan with
    /// such a step is taken only when the tool is there now
    /// ([`GamePlan::playable`]); without it the step ends the plan.
    Era(EraAct),
}

impl Act {
    /// The step commits it to walking in or attacking.
    pub(super) fn commits(self) -> bool {
        matches!(
            self,
            Act::Reach { .. }
                | Act::Route(_)
                | Act::Meaty { .. }
                | Act::Edge
                | Act::Space(_)
                | Act::Step { forward: true }
                | Act::Shot
                | Act::Rush
                | Act::Tool(_)
                | Act::Era(_)
                | Act::Do(Choice::Normal { .. } | Choice::Throw | Choice::Special(_) | Choice::Approach | Choice::JumpIn)
        )
    }

    /// The step waits for a knocked-down opponent's rise, or walks in to
    /// meet it: it is no waste against him while he is down.
    pub(super) fn meets_the_rise(self) -> bool {
        matches!(self, Act::Meaty { .. } | Act::GuardRise | Act::Reach { .. })
    }

    /// The step needs a special or an era command: its projectile, the
    /// era style's meter dash, or a special or a cancel into one. A route
    /// step does not by itself: which routes may be taken is the moment's
    /// call ([`Moment::allows`]).
    pub(super) fn needs_special(self) -> bool {
        matches!(self, Act::Shot | Act::Rush | Act::Tool(_) | Act::Era(_) | Act::Do(Choice::Special(_) | Choice::Cancel { .. }))
    }
}

/// Frames past a route's last pad by which what it met shows (its last
/// hit can be a projectile's, some frames after its fighter is free).
pub(super) const ROUTE_WATCH: u32 = 16;
/// What a route the CPU has seen drop costs it in score, for each drop it
/// has seen more than landings; one seen to drop this many times more is
/// no longer taken in the fight.
pub(super) const DROP_COST: i32 = 12;
pub(super) const DROPS_KEPT: u8 = 3;
/// Frames an opponent must be seen standing in one place before a route
/// from neutral is begun on him.
pub(super) const SETTLED_FRAMES: u32 = 4;
/// Frames the CPU may walk in to start a route: from neutral, and to
/// punish a whiff, when the recovering opponent will not wait long.
pub(super) const NEUTRAL_WALK: u8 = 20;
pub(super) const PUNISH_WALK: u8 = 6;
/// Frames before a falling opponent lands at which a punish begins: about
/// the time a route's opener takes to reach him.
pub(super) const LANDING_LEAD: i16 = 6;

/// A walk covers about two pixels a frame.
const WALK_SPEED: i16 = 2;
/// How much nearer than its widest start a ground route is still taken.
/// The book proves each route from its widest start; much nearer, a normal
/// can come out as its close version, with other timing, and the route
/// drops (seen in play: Ryu's SF6 medium punch into a Hadoken from 30 to
/// 38 pixels where the book has 52). A punish that spends a Super Art is
/// held to it too: begun nearer than the book proves it, it drops and its
/// Super whiffs, wasting the bar (seen in play, SC8 `#200`; SC13 `#205`).
/// Other punishes keep no such bound. Not done: Ryu's SF6 punishes, proven
/// from the reference punish position, land whole after standing there up
/// to two frames but only their first hit after a walk of a frame or more,
/// so a walked-in punish likely drops; the walk in is kept.
const ROUTE_NEAR: i16 = 8;
/// How far outside the opponent's reach an [`Act::Edge`] stands, and how
/// near that counts as there.
pub(super) const EDGE_OUT: i16 = 8;
pub(super) const EDGE_NEAR: i16 = 4;
/// The longest an [`Act::Edge`] goes on walking and looking, from its
/// first walk, before the plan moves on wherever it stands.
pub(super) const EDGE_FRAMES: u32 = 90;
/// The most frames an [`Act::Edge`] walks before it looks again.
pub(super) const EDGE_WALK: i16 = 8;
/// Frames of an [`Act::Step`]: about 12 to 18 pixels at Ryu's walk, across
/// the edge of his reach and back.
pub(super) const STEP_FRAMES: u32 = 4;
pub(super) const STEP_SPREAD: u32 = 3;

/// A punish that spends a Super Art: not taken much nearer than the book
/// proves it (see [`ROUTE_NEAR`]).
fn super_punish(route: &Route) -> bool {
    route.reach == Reach::Punish && route.super_spent > 0
}

/// What the moment allows when the CPU looks for a route.
pub(super) struct Moment<'a> {
    pub reaches: &'a [Reach],
    /// An era style is in force: it brings true combos and cancels with it.
    pub era: bool,
    /// A mission allows only normals and throws (SC23, `#215`): only a
    /// route of plain normals ([`Route::normals_only`]) may be taken.
    pub normals_only: bool,
    pub true_combos: bool,
    pub cancels: bool,
    pub crouching: bool,
    pub meters: [u16; 2],
    pub corner: bool,
    pub gap: i16,
    pub walk: u8,
    /// In health bar points, and his whole health in an era style's finer
    /// units, which a route's damage is in (48 bar points of it).
    pub foe_health: u16,
    pub foe_max_sub_hp: u16,
    /// A route that spends a super may be taken though it does not win the
    /// round (SC8, `#200`: the round state says spend, or the old play).
    pub spend_super: bool,
    /// Drive the route must leave in the gauge: the level's reserve for
    /// the stronger play.
    pub drive_reserve: u16,
    /// The stronger play spends no Drive down to burnout, unless the route
    /// wins the round.
    pub avoid_burnout: bool,
    /// The opponent as route books name him ([`crate::route_book::opponent_key`]),
    /// when known: a route recorded failing on him is not taken.
    pub foe: Option<&'a str>,
}

impl Moment<'_> {
    /// The route can be taken now.
    pub(super) fn allows(&self, route: &Route) -> bool {
        let stance = if self.crouching { Stance::Crouching } else { Stance::Standing };
        self.reaches.contains(&route.reach)
            && (!route.true_combos || self.true_combos || self.era)
            && (!route.cancels || self.cancels || self.era)
            && route.stances.contains(&stance)
            && (!route.corner || self.corner)
            && route.super_spent <= self.meters[0]
            && (route.super_spent == 0 || self.spend_super || self.wins_round(route))
            && (route.drive_spent == 0
                || (route.drive_spent.saturating_add(self.drive_reserve) <= self.meters[1]
                    && (!self.avoid_burnout || route.drive_spent < self.meters[1] || self.wins_round(route))))
            && self.gap <= route.gap + i16::from(self.walk) * WALK_SPEED
            && (!matches!(route.reach, Reach::Ground | Reach::JumpIn) || self.gap >= route.gap - ROUTE_NEAR)
            && (!super_punish(route) || self.gap >= route.gap - ROUTE_NEAR)
            && !self.foe.is_some_and(|foe| route.fails_against(foe, stance))
            && (!self.normals_only || route.normals_only())
    }

    /// Frames to walk before the route reaches from here.
    pub(super) fn walk_frames(&self, route: &Route) -> u8 {
        ((self.gap - route.gap).max(0) + WALK_SPEED - 1).div_euclid(WALK_SPEED).min(i16::from(self.walk)) as u8
    }

    /// The route's damage takes the rest of the opponent's health.
    pub(super) fn wins_round(&self, route: &Route) -> bool {
        // His bar shows a point while any of it is left, so the points
        // shown are the most he can have.
        self.foe_health > 0 && u32::from(route.damage) * 48 >= u32::from(self.foe_health) * u32::from(self.foe_max_sub_hp)
    }

    /// The route's damage may take the rest of the opponent's health: his
    /// bar shows a point while any of it is left, so with `foe_health`
    /// points shown he has more than `foe_health - 1` points' worth. The
    /// safe side for not cutting a route (SC10, `#202`), where
    /// [`Self::wins_round`] is the safe side for spending meter.
    pub(super) fn may_win_round(&self, route: &Route) -> bool {
        self.foe_health > 0 && u32::from(route.damage) * 48 > u32::from(self.foe_health - 1) * u32::from(self.foe_max_sub_hp)
    }

    /// How good the route is now: one that wins the round above all;
    /// otherwise its damage, a knockdown at its end, and meter kept; a
    /// route taken lately scores lower.
    pub(super) fn score(&self, route: &Route, recent: bool) -> i32 {
        let points = i32::from(route.damage / 20);
        if self.wins_round(route) {
            return 10_000 + points;
        }
        let meter = i32::from(route.super_spent) / 4 + i32::from(route.drive_spent) / 8;
        points * 4 + if route.knockdown { 12 } else { 0 } - meter - if recent { 30 } else { 0 }
    }
}

/// What a route cut on purpose at one of its marked points goes into: a
/// combo reset (SC10, `#202`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResetKind {
    /// A walk in and a throw as he comes out of his hit reaction.
    Throw,
    /// A crouching short kick timed to meet him as he comes out of it: a
    /// low he must guard crouching.
    Low,
    /// Nothing: a crouching guard held past his recovery, so a reversal he
    /// presses out of it meets the guard and can be punished.
    Bait,
}

/// A combo reset planned for a route: what it goes into, and what the
/// book's cut point says of the moment it is cut ([`crate::route_book::Cut`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Reset {
    pub kind: ResetKind,
    /// Frames from the cut to his first free frame, frames the CPU's fighter
    /// is free before him, and the gap to him then.
    pub free: u8,
    pub ahead: u8,
    pub gap: i16,
}

/// A combo reset whose route has been cut, until it goes on (it has seen
/// the route hit and its fighter is free) or is let go (nothing seen by
/// his first free frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ResetWait {
    pub reset: Reset,
    /// The step the route was cut on, the way the route faced him, and
    /// whether it has seen him in a hit reaction since the cut.
    pub cut: u32,
    pub facing_right: bool,
    pub hit: bool,
}

/// A route being entered: which one, where it is in its script, where it
/// stops (a slip, or a combo reset at a marked point, cuts it short), and
/// what the CPU has seen of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RouteRun {
    fighter: TournamentFighter,
    revision: crate::era::Revision,
    loadout: crate::era::Loadout,
    pub index: u8,
    walk: u8,
    pub at: u16,
    stop: u16,
    facing_right: bool,
    hit: bool,
    guarded: bool,
    /// The reset its cut goes into, when it is cut on purpose.
    pub reset: Option<Reset>,
}

impl RouteRun {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        fighter: TournamentFighter,
        revision: crate::era::Revision,
        loadout: crate::era::Loadout,
        index: u8,
        walk: u8,
        stop: u16,
        facing_right: bool,
    ) -> Self {
        RouteRun { fighter, revision, loadout, index, walk, at: 0, stop, facing_right, hit: false, guarded: false, reset: None }
    }

    fn route(&self) -> Option<&'static Route> {
        crate::route_book::book(self.fighter, self.revision, self.loadout)?.routes.get(usize::from(self.index))
    }

    /// The pad for this frame of the run, the walk first: `None` when the
    /// route is over or cut.
    pub(super) fn pad(&self) -> Option<u16> {
        let forward = if self.facing_right { super::RIGHT } else { super::LEFT };
        if self.at < u16::from(self.walk) {
            return Some(forward);
        }
        let frame = self.at - u16::from(self.walk);
        if frame >= self.stop {
            return None;
        }
        let (stick, buttons) = self.route()?.pad_at(usize::from(frame))?;
        // Routes are written for an attacker facing right.
        let stick = u16::from(stick);
        let stick = if self.facing_right { stick } else { (stick & !3) | ((stick & 1) << 1) | ((stick & 2) >> 1) };
        Some(stick | u16::from(buttons) << 4)
    }

    /// It ran to its end (no slip cut it) and is a combo of two steps or
    /// more.
    pub(super) fn combo_in_full(&self) -> bool {
        self.route().is_some_and(|route| route.steps.len() >= 2 && usize::from(self.stop) == route.len())
    }

    /// The step of its route the pad it gives now belongs to, once its
    /// walk in is over.
    pub(super) fn step_index(&self) -> Option<usize> {
        let route = self.route()?;
        let frame = usize::from(self.at.checked_sub(u16::from(self.walk))?);
        (0..route.steps.len()).rev().find(|&index| route.step_start(index) <= frame)
    }

    /// The script frame it stops on: its end, or where it is cut.
    #[cfg(test)]
    pub(super) fn route_stop(&self) -> usize {
        usize::from(self.stop)
    }

    /// It was entered facing right (its pads as the book writes them).
    pub(super) fn facing_right(&self) -> bool {
        self.facing_right
    }

    /// It is still walking in, before the route's first pad.
    pub(super) fn walking(&self) -> bool {
        self.at < u16::from(self.walk)
    }

    /// Note what the frame shows of the route.
    pub(super) fn watch(&mut self, view: Percept) {
        self.hit |= view.foe.reacting();
        self.guarded |= view.foe.guarding() && view.me.attacking();
    }

    pub(super) fn outcome(&self) -> Outcome {
        if self.hit {
            Outcome::Hit
        } else if self.guarded {
            Outcome::Guard
        } else {
            Outcome::Whiff
        }
    }
}

/// One step of a game plan: what to do, and the step that follows each
/// outcome, in [`Outcome`]'s order. `None` ends the plan.
#[derive(Clone, Copy, Debug)]
pub(super) struct Step {
    pub act: Act,
    pub then: [Option<u8>; 5],
}

impl Step {
    pub(super) fn next(&self, outcome: Outcome) -> Option<u8> {
        self.then[outcome as usize]
    }
}

/// A short script for one situation, and how it scores against the other
/// things the CPU could do there.
#[derive(Clone, Copy, Debug)]
pub(super) struct GamePlan {
    pub situation: Situation,
    pub score: i32,
    /// Added to the score while the opponent is cornered.
    pub corner_bonus: i32,
    /// Taken up only while the opponent is cornered.
    pub cornered_only: bool,
    /// Taken up only while the gap to the opponent is in this range.
    pub gap: (i16, i16),
    pub steps: &'static [Step],
    /// What it leans on, for the habit memory (SC9, `#201`).
    pub lean: Lean,
    /// The lowest difficulty level that takes it up (SC11, `#203`).
    pub from: Level,
}

impl GamePlan {
    /// The level may take it up: each level takes every plan the one below
    /// it takes, and more.
    pub(super) fn for_level(&self, level: Level) -> bool {
        level.rank() >= self.from.rank()
    }

    /// A plan that stays on the opponent: after a guarded attack, on a
    /// knocked-down opponent, or walking a cornered one down. How many of
    /// these it takes in a row is a level value ([`Level::pressure_plans`]).
    pub(super) fn pressure(&self) -> bool {
        self.cornered_only || matches!(self.situation, Situation::ItWasBlocked | Situation::OpponentKnockedDown)
    }

    /// Some step of it needs a special or an era command
    /// ([`Act::needs_special`]): not taken under a mission that allows only
    /// normals and throws (SC23, `#215`).
    pub(super) fn needs_specials(&self) -> bool {
        self.steps.iter().any(|step| step.act.needs_special())
    }

    /// Its fighter has what each step needs (SC15, `#207`): a projectile for
    /// a projectile step, the tool for a tool step and a route in his book
    /// for a route step that starts other than on the ground; and the era
    /// style's tool for an era step that starts one (SC16, `#208`: Custom
    /// Combo with the meter for it, a change of stance). A plan he cannot
    /// play is not taken.
    pub(super) fn playable(&self, projectile: bool, tool: impl Fn(Tool) -> bool, route: impl Fn(Reach) -> bool, era: impl Fn(EraAct) -> bool) -> bool {
        self.steps.iter().all(|step| match step.act {
            Act::Shot => projectile,
            Act::Tool(t) => tool(t),
            Act::Era(EraAct::CustomChain) => true,
            Act::Era(act) => era(act),
            Act::Route(reach) if reach != Reach::Ground => route(reach),
            _ => true,
        })
    }

    /// It can be played with normals and throws alone: no step needs a
    /// special or an era command, and each route step has a route of
    /// plain normals to take (`normals_route` says whether the fighter's
    /// book holds one starting that way). A plan whose route step could
    /// take none would walk to its spacing and end there.
    pub(super) fn keeps_to_normals(&self, normals_route: impl Fn(Reach) -> bool) -> bool {
        !self.needs_specials()
            && self.steps.iter().all(|step| match step.act {
                Act::Route(reach) => normals_route(reach),
                _ => true,
            })
    }
}

/// Any gap.
const ANY: (i16, i16) = (i16::MIN, i16::MAX);

const fn step(act: Act, then: [Option<u8>; 5]) -> Step {
    Step { act, then }
}

/// After any outcome, the same next step (or the plan's end).
const fn always(next: Option<u8>) -> [Option<u8>; 5] {
    [next; 5]
}

/// Close neutral: reach with this normal, then branch on what it met.
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

/// Time this normal to the opponent's rise, then keep pressure or back off.
const fn meaty(crouch: bool, button: usize) -> [Step; 4] {
    [
        step(Act::Meaty { crouch, button }, [None, Some(1), Some(2), None, None]),
        step(Act::Do(Choice::Throw), always(None)),
        step(Act::Do(Choice::Retreat), always(Some(3))),
        step(Act::Do(Choice::Guard), always(None)),
    ]
}

/// Ryu in the SF6 era style, close neutral: walk to crouching medium kick
/// range and press it. On a hit (it knocks down in this engine) follow him
/// in and time the same kick to his rise, or, if he rises out of its reach,
/// walk in and poke again; on a guard walk in and throw; on a whiff give
/// ground and guard.
const RYU_SF6_POKE: [Step; 6] = [
    step(Act::Reach { crouch: true, button: 4 }, always(Some(1))),
    step(Act::Do(Choice::Normal { crouch: true, button: 4 }), [Some(2), Some(3), Some(4), None, None]),
    step(Act::Meaty { crouch: true, button: 4 }, [None, None, None, None, Some(0)]),
    step(Act::Do(Choice::Throw), always(None)),
    step(Act::Do(Choice::Retreat), always(Some(5))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// Ryu in the SF6 era style, close neutral: take the spacing the book's
/// ground routes start from, then walk in and enter one. On a guard walk in
/// and throw; on a whiff give ground and guard.
const RYU_SF6_ROUTE: [Step; 5] = [
    step(Act::Space(ROUTE_SPACE), always(Some(1))),
    step(Act::Route(Reach::Ground), [None, Some(2), Some(3), None, None]),
    step(Act::Do(Choice::Throw), always(None)),
    step(Act::Do(Choice::Retreat), always(Some(4))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// The gap the route plan takes before entering a ground route: a little
/// inside the widest start of most of Ryu's SF6 ground routes (52 to 58).
const ROUTE_SPACE: i16 = 50;

/// Ryu in the SF6 era style, after an attack was guarded: a short wait,
/// then a medium punch that hits him if he pressed a button in the gap. On
/// a guard again the pressure has run out: give ground and guard.
const RYU_SF6_FRAME_TRAP: [Step; 4] = [
    step(Act::Do(Choice::Wait), always(Some(1))),
    step(Act::Do(Choice::Normal { crouch: false, button: 1 }), [None, Some(2), Some(2), None, None]),
    step(Act::Do(Choice::Retreat), always(Some(3))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// After an attack was guarded: walk in and throw.
const RYU_SF6_WALK_THROW: [Step; 1] = [step(Act::Do(Choice::Throw), always(None))];

/// Against a knocked-down opponent: walk to crouching medium kick range
/// and time it to meet his rise. On a guard throw; on a whiff back off.
const RYU_SF6_MEATY: [Step; 4] = [
    step(Act::Meaty { crouch: true, button: 4 }, [None, Some(1), Some(2), None, None]),
    step(Act::Do(Choice::Throw), always(None)),
    step(Act::Do(Choice::Retreat), always(Some(3))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// Against a knocked-down opponent: walk in close and guard over his rise,
/// so a reversal is guarded and its whiff punished.
const RYU_SF6_BAIT: [Step; 1] = [step(Act::GuardRise, always(None))];

/// Ryu in the SF6 era style, in neutral: stand just outside the opponent's
/// reach, step in across its edge and back out, and out again while it
/// watches what he does; back to the edge and in and out once more, so a
/// button he presses at the CPU whiffs (the whiff is punished as any is);
/// then walk in and press crouching medium kick, followed as the poke is.
/// The watch outlasts the reaction delay, so a button pressed at the first
/// step is seen before the second, and it is spent moving, not standing.
const RYU_SF6_HOVER: [Step; 13] = [
    step(Act::Edge, always(Some(1))),
    step(Act::Step { forward: true }, always(Some(2))),
    step(Act::Step { forward: false }, always(Some(3))),
    step(Act::Step { forward: false }, always(Some(4))),
    step(Act::Edge, always(Some(5))),
    step(Act::Step { forward: true }, always(Some(6))),
    step(Act::Step { forward: false }, always(Some(7))),
    step(Act::Reach { crouch: true, button: 4 }, always(Some(8))),
    step(Act::Do(Choice::Normal { crouch: true, button: 4 }), [Some(9), Some(10), Some(11), None, None]),
    step(Act::Meaty { crouch: true, button: 4 }, [None, None, None, None, Some(7)]),
    step(Act::Do(Choice::Throw), always(None)),
    step(Act::Do(Choice::Retreat), always(Some(12))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// Ryu in the SF6 era style, far off: a projectile, then walk in behind it
/// to take the space it wins.
const RYU_SF6_SHOT: [Step; 2] = [
    step(Act::Shot, always(Some(1))),
    step(Act::Do(Choice::Approach), always(None)),
];

/// Ryu in the SF6 era style, far off, against an opponent its habit memory
/// says jumps its projectile (SC9, `#201`): a projectile, then standing its
/// ground while it recovers, watching for the jump to anti-air it rather
/// than walking in under him. Never taken without that habit.
const RYU_SF6_SHOT_WATCH: [Step; 6] = [
    step(Act::Shot, always(Some(1))),
    step(Act::Do(Choice::Wait), always(Some(2))),
    step(Act::Do(Choice::Wait), always(Some(3))),
    step(Act::Do(Choice::Wait), always(Some(4))),
    step(Act::Do(Choice::Wait), always(Some(5))),
    step(Act::Do(Choice::Wait), always(None)),
];

/// Ryu in the SF6 era style, far off (SC8, `#200`): a projectile, then a
/// Drive Rush in behind it while he deals with it, and a standing medium
/// punch as it arrives (a rushed normal gains frames). With a projectile
/// of its own already out it rushes in behind that. Whatever the punch
/// meets, the situation's own plans go on from there.
const RYU_SF6_SHOT_RUSH: [Step; 4] = [
    step(Act::Shot, always(Some(1))),
    step(Act::Rush, always(Some(2))),
    step(Act::Reach { crouch: false, button: 1 }, always(Some(3))),
    step(Act::Do(Choice::Normal { crouch: false, button: 1 }), always(None)),
];

/// Ryu in the SF6 era style, from the range a forward jump carries him to
/// the opponent: a jump-in with an attack on the way down.
const RYU_SF6_JUMP_IN: [Step; 1] = [step(Act::Do(Choice::JumpIn), always(None))];

/// The gap a projectile is thrown from (a fireball from closer is jumped
/// on its way out), and the gaps a forward jump-in lands on him from.
const SHOT_GAP: i16 = 90;
/// The gaps the projectile is thrown from to stand and watch for a jump
/// over it (SC9, `#201`): measured against an opponent who jumps on its
/// start-up, from 140 to 180 pixels his jump lands in front of it as it
/// recovers and can be anti-aired; from nearer he lands on it while it
/// still recovers, and from farther he lands short (4 of 234 such jumps
/// anti-aired).
pub(super) const WATCHED_SHOT_GAPS: (i16, i16) = (140, 160);
const JUMP_IN_GAP: (i16, i16) = (70, 150);

const RYU_SF6: [GamePlan; 13] = [
    GamePlan { situation: Situation::NeutralClose, score: 60, corner_bonus: 0, cornered_only: false, gap: ANY, steps: &RYU_SF6_POKE, lean: Lean::None, from: Level::Easy },
    GamePlan { situation: Situation::NeutralClose, score: 62, corner_bonus: 6, cornered_only: false, gap: ANY, steps: &RYU_SF6_ROUTE, lean: Lean::None, from: Level::Easy },
    GamePlan { situation: Situation::ItWasBlocked, score: 60, corner_bonus: 4, cornered_only: false, gap: ANY, steps: &RYU_SF6_FRAME_TRAP, lean: Lean::FrameTrap, from: Level::Normal },
    GamePlan { situation: Situation::ItWasBlocked, score: 56, corner_bonus: 12, cornered_only: false, gap: ANY, steps: &RYU_SF6_WALK_THROW, lean: Lean::WalkThrow, from: Level::Easy },
    GamePlan { situation: Situation::OpponentKnockedDown, score: 62, corner_bonus: 8, cornered_only: false, gap: ANY, steps: &RYU_SF6_MEATY, lean: Lean::Meaty, from: Level::Normal },
    GamePlan { situation: Situation::OpponentKnockedDown, score: 52, corner_bonus: -10, cornered_only: false, gap: ANY, steps: &RYU_SF6_BAIT, lean: Lean::Bait, from: Level::VeryHard },
    // A cornered opponent out of reach (its own guarded attacks push it
    // back off him): walk him down rather than let him out.
    GamePlan { situation: Situation::NeutralFar, score: 60, corner_bonus: 0, cornered_only: true, gap: ANY, steps: &RYU_SF6_POKE, lean: Lean::None, from: Level::Normal },
    // Scores set by the measuring run (240 Ryu SF6 mirror matches a side
    // against the old Fair CPU): the projectile from afar won the most and
    // the jump-in least.
    GamePlan { situation: Situation::NeutralClose, score: 61, corner_bonus: -6, cornered_only: false, gap: ANY, steps: &RYU_SF6_HOVER, lean: Lean::None, from: Level::Normal },
    GamePlan { situation: Situation::NeutralFar, score: 60, corner_bonus: -6, cornered_only: false, gap: ANY, steps: &RYU_SF6_HOVER, lean: Lean::None, from: Level::Normal },
    GamePlan { situation: Situation::NeutralFar, score: 70, corner_bonus: 0, cornered_only: false, gap: (SHOT_GAP, i16::MAX), steps: &RYU_SF6_SHOT, lean: Lean::Shot, from: Level::Easy },
    GamePlan { situation: Situation::NeutralFar, score: 40, corner_bonus: 0, cornered_only: false, gap: JUMP_IN_GAP, steps: &RYU_SF6_JUMP_IN, lean: Lean::JumpIn, from: Level::Easy },
    GamePlan { situation: Situation::NeutralFar, score: 66, corner_bonus: 6, cornered_only: false, gap: (SHOT_GAP, i16::MAX), steps: &RYU_SF6_SHOT_RUSH, lean: Lean::ShotRush, from: Level::Hard },
    // Its score comes from the habit memory alone (SC9, `#201`).
    GamePlan { situation: Situation::NeutralFar, score: 0, corner_bonus: 0, cornered_only: false, gap: WATCHED_SHOT_GAPS, steps: &RYU_SF6_SHOT_WATCH, lean: Lean::ShotWatch, from: Level::Normal },
];

/// After a guarded attack, or in close neutral, a command throw (SC15,
/// `#207`): it takes a guarding opponent and cannot be teched. The step
/// walks in to its fighter's catch range first ([`super::cast::catch_range`]).
const COMMAND_THROW: [Step; 1] = [step(Act::Tool(Tool::CommandThrow), always(None))];

/// Far off, a projectile and a teleport while he deals with it (SC15,
/// `#207`): the situation's own plans go on from wherever it lands.
const TELEPORT: [Step; 2] = [
    step(Act::Shot, always(Some(1))),
    step(Act::Tool(Tool::Teleport), always(None)),
];

/// From the middle distance, a charge built walking back and released into
/// the charge move (SC15, `#207`).
const CHARGE: [Step; 1] = [step(Act::Tool(Tool::Charge), always(None))];

/// From the range a jump carries him to the opponent: a jump-in route from
/// the fighter's route book (SC15, `#207`), taken only by a fighter whose
/// book has one.
const JUMP_IN_ROUTE: [Step; 1] = [step(Act::Route(Reach::JumpIn), always(None))];

/// The gaps the charge move is released from: about its reach, and far
/// enough that the charge's crouch is not walked into.
const CHARGE_GAP: (i16, i16) = (60, 150);
/// The gaps a jump-in route is begun from: the jump-in routes of the SF2
/// era style's books start from 98 to 124 pixels.
const JUMP_IN_ROUTE_GAP: (i16, i16) = (90, 132);

/// The SF2 era style's frame trap after a guarded attack (SC15, `#207`):
/// the same short wait, then a crouching light punch, which starts sooner
/// than Ryu's SF6 standing medium punch in every fighter's kit.
const SF2_FRAME_TRAP: [Step; 4] = [
    step(Act::Do(Choice::Wait), always(Some(1))),
    step(Act::Do(Choice::Normal { crouch: true, button: 0 }), [None, Some(2), Some(2), None, None]),
    step(Act::Do(Choice::Retreat), always(Some(3))),
    step(Act::Do(Choice::Guard), always(None)),
];

/// A plan of the SF2 era style's tables.
const fn plan(situation: Situation, score: i32, corner_bonus: i32, gap: (i16, i16), steps: &'static [Step], lean: Lean, from: Level) -> GamePlan {
    GamePlan { situation, score, corner_bonus, cornered_only: false, gap, steps, lean, from }
}

// The SF2 era style's plans (SC15, `#207`): Ryu's SF6 game without the
// Drive Rush the style has not got, a frame trap of a crouching light
// punch, a jump-in route where the book has one, and plans for each
// fighter's own tools. Which of them a fighter takes was set by the
// measuring run (mirror matches against the old Fair CPU at Very
// Difficult, in Super Turbo, each side): from all of them, a plan was left
// out while that raised the new CPU's wins on its worse side, keeping a
// plan for each of close and far neutral, a guarded attack and a
// knocked-down opponent, and every plan for a tool of the fighter's own. A
// plan needing a projectile, a tool or a jump-in route the fighter has not
// got is never taken ([`GamePlan::playable`]).
const POKE: GamePlan = plan(Situation::NeutralClose, 60, 0, ANY, &RYU_SF6_POKE, Lean::None, Level::Easy);
const ROUTE: GamePlan = plan(Situation::NeutralClose, 62, 6, ANY, &RYU_SF6_ROUTE, Lean::None, Level::Easy);
const FRAME_TRAP: GamePlan = plan(Situation::ItWasBlocked, 60, 4, ANY, &SF2_FRAME_TRAP, Lean::FrameTrap, Level::Normal);
const WALK_THROW: GamePlan = plan(Situation::ItWasBlocked, 56, 12, ANY, &RYU_SF6_WALK_THROW, Lean::WalkThrow, Level::Easy);
const MEATY: GamePlan = plan(Situation::OpponentKnockedDown, 62, 8, ANY, &RYU_SF6_MEATY, Lean::Meaty, Level::Normal);
const BAIT: GamePlan = plan(Situation::OpponentKnockedDown, 52, -10, ANY, &RYU_SF6_BAIT, Lean::Bait, Level::VeryHard);
const CORNERED: GamePlan = GamePlan { cornered_only: true, ..plan(Situation::NeutralFar, 60, 0, ANY, &RYU_SF6_POKE, Lean::None, Level::Normal) };
const HOVER_CLOSE: GamePlan = plan(Situation::NeutralClose, 61, -6, ANY, &RYU_SF6_HOVER, Lean::None, Level::Normal);
const HOVER_FAR: GamePlan = plan(Situation::NeutralFar, 60, -6, ANY, &RYU_SF6_HOVER, Lean::None, Level::Normal);
const SHOT: GamePlan = plan(Situation::NeutralFar, 70, 0, (SHOT_GAP, i16::MAX), &RYU_SF6_SHOT, Lean::Shot, Level::Easy);
const JUMP_IN: GamePlan = plan(Situation::NeutralFar, 40, 0, JUMP_IN_GAP, &RYU_SF6_JUMP_IN, Lean::JumpIn, Level::Easy);
const SHOT_WATCH: GamePlan = plan(Situation::NeutralFar, 0, 0, WATCHED_SHOT_GAPS, &RYU_SF6_SHOT_WATCH, Lean::ShotWatch, Level::Normal);
const JUMP_ROUTE: GamePlan = plan(Situation::NeutralFar, 44, 0, JUMP_IN_ROUTE_GAP, &JUMP_IN_ROUTE, Lean::JumpIn, Level::Easy);
/// A command throw in close neutral and after a guarded attack.
const THROW_CLOSE: GamePlan = plan(Situation::NeutralClose, 58, 8, ANY, &COMMAND_THROW, Lean::None, Level::Easy);
const THROW_BLOCKED: GamePlan = plan(Situation::ItWasBlocked, 58, 12, ANY, &COMMAND_THROW, Lean::WalkThrow, Level::Easy);
const TELEPORT_FAR: GamePlan = plan(Situation::NeutralFar, 66, 0, (SHOT_GAP, i16::MAX), &TELEPORT, Lean::Shot, Level::Normal);
const CHARGE_FAR: GamePlan = plan(Situation::NeutralFar, 64, 0, CHARGE_GAP, &CHARGE, Lean::None, Level::Easy);

const SF2_RYU: [GamePlan; 10] = [POKE, ROUTE, FRAME_TRAP, MEATY, CORNERED, HOVER_CLOSE, HOVER_FAR, SHOT, SHOT_WATCH, JUMP_ROUTE];
const SF2_KEN: [GamePlan; 10] = [POKE, WALK_THROW, MEATY, BAIT, CORNERED, HOVER_CLOSE, HOVER_FAR, SHOT, JUMP_IN, SHOT_WATCH];
const SF2_SAGAT: [GamePlan; 7] = [ROUTE, WALK_THROW, BAIT, SHOT, JUMP_IN, SHOT_WATCH, JUMP_ROUTE];
const SF2_RETSU: [GamePlan; 11] = [ROUTE, WALK_THROW, MEATY, BAIT, HOVER_CLOSE, SHOT, JUMP_IN, SHOT_WATCH, JUMP_ROUTE, THROW_CLOSE, THROW_BLOCKED];
const SF2_GEKI: [GamePlan; 12] = [POKE, ROUTE, FRAME_TRAP, BAIT, CORNERED, HOVER_CLOSE, SHOT, JUMP_IN, SHOT_WATCH, THROW_CLOSE, THROW_BLOCKED, TELEPORT_FAR];
const SF2_JOE: [GamePlan; 5] = [ROUTE, WALK_THROW, BAIT, SHOT, SHOT_WATCH];
const SF2_MIKE: [GamePlan; 4] = [ROUTE, WALK_THROW, MEATY, JUMP_IN];
const SF2_LEE: [GamePlan; 6] = [ROUTE, WALK_THROW, MEATY, JUMP_IN, JUMP_ROUTE, CHARGE_FAR];
// Birdie idles more than the old Fair CPU with this set (his slow walk
// back while his charge builds stands still on many frames); with the far
// hover added he idles less but no longer wins half his Super Turbo
// matches as Player 1. Listed on #207.
const SF2_BIRDIE: [GamePlan; 10] = [POKE, ROUTE, FRAME_TRAP, WALK_THROW, BAIT, JUMP_IN, JUMP_ROUTE, THROW_CLOSE, THROW_BLOCKED, CHARGE_FAR];
const SF2_ADON: [GamePlan; 6] = [ROUTE, FRAME_TRAP, WALK_THROW, BAIT, JUMP_IN, JUMP_ROUTE];

/// How often, in percent, a fighter takes up a game plan where one would
/// do (SC15, `#207`); otherwise it plays its single choices for that
/// decision. Mike in the SF2 era style: the measuring run had his single
/// choices win more of his fights than his plans.
pub(super) fn plan_rate(fighter: Option<TournamentFighter>, family: Option<EraFamily>) -> u32 {
    match (fighter, family) {
        (Some(TournamentFighter::Mike), Some(EraFamily::Sf2)) => 40,
        _ => 100,
    }
}

/// Game plans of one table whose working the CPU remembers.
pub(super) const PLAN_SLOTS: usize = 16;
/// The most times in a row a plan's working is counted.
pub(super) const WORKED_KEPT: u8 = 3;
/// What each time in a row a plan has worked adds to its score where the
/// level leans on what works, and what having just been taken costs a plan
/// where the level puts variety first.
const WORKED: i32 = 8;
const VARIETY: i32 = 12;

/// Very Difficult and Difficult may repeat a winning option for as long as
/// it keeps working; Normal and Easy put variety first (the maintainer,
/// 2026-11-18).
pub(super) fn leans_on_what_works(level: Level) -> bool {
    matches!(level, Level::Hard | Level::VeryHard)
}

/// What a game plan's record in this match adds to its score: `worked` is
/// how many times in a row it has worked lately, `last` whether it was the
/// plan taken last.
pub(super) fn repeat_score(level: Level, worked: u8, last: bool) -> i32 {
    if leans_on_what_works(level) {
        WORKED * i32::from(worked.min(WORKED_KEPT))
    } else if last {
        -VARIETY
    } else {
        0
    }
}

/// The game plans a fighter has in an era style. A pair with none plays
/// the CPU's single choices.
pub(super) fn plans(fighter: Option<TournamentFighter>, family: Option<EraFamily>) -> &'static [GamePlan] {
    match (fighter, family) {
        (Some(TournamentFighter::Ryu), Some(EraFamily::Sf6)) => &RYU_SF6,
        (Some(fighter), Some(EraFamily::SfZ)) => sfz::plans(fighter),
        (Some(fighter), Some(EraFamily::Sf3)) => sf3::plans(fighter),
        (Some(fighter), Some(EraFamily::Sf4)) => sf4::plans(fighter),
        (Some(fighter), Some(EraFamily::Sf5)) => sf5::plans(fighter),
        // Gen and Eagle have none yet: the measuring run found no set of
        // these that beats the old Fair CPU with them (SC15, `#207`); a
        // follow-up builds their own game.
        (Some(fighter), Some(EraFamily::Sf2)) => match fighter {
            TournamentFighter::Ryu => &SF2_RYU,
            TournamentFighter::Ken => &SF2_KEN,
            TournamentFighter::Sagat => &SF2_SAGAT,
            TournamentFighter::Retsu => &SF2_RETSU,
            TournamentFighter::Geki => &SF2_GEKI,
            TournamentFighter::Joe => &SF2_JOE,
            TournamentFighter::Mike => &SF2_MIKE,
            TournamentFighter::Lee => &SF2_LEE,
            TournamentFighter::Birdie => &SF2_BIRDIE,
            TournamentFighter::Adon => &SF2_ADON,
            TournamentFighter::Gen | TournamentFighter::Eagle => &[],
        },
        _ => &[],
    }
}

/// The CPU's live attack box reaches the near edge of the opponent's
/// hurtbox in `view`.
fn reached(view: Percept) -> bool {
    let toward = if view.foe.x >= view.me.x { 1 } else { -1 };
    let near = if toward > 0 { view.foe.hurt.0 } else { view.foe.hurt.1 };
    view.me.active && (view.me.front - near) * toward >= 0
}

/// Where the CPU is in a game plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Running {
    /// The plan's place in its pair's table, and the step in it.
    pub plan: u8,
    pub step: u8,
    /// The step's action has been pressed, on this step of the machine.
    pub acted: bool,
    pub pressed: u32,
    /// The machine step the plan began on.
    pub began: u32,
    /// One of its steps was seen to hit him: the plan worked.
    pub hit: bool,
}

impl Running {
    /// What has come of the pressed normal `(crouch, button)` as far as
    /// `view` shows, once it shows anything: `reach` is that normal's.
    pub(super) fn outcome(&self, view: Percept, reach: i16) -> Option<Outcome> {
        if view.frame <= self.pressed {
            return None;
        }
        if view.foe.reacting() {
            Some(Outcome::Hit)
        } else if view.me.attacking() && view.foe.guarding() && view.gap() <= reach + 8 && reached(view) {
            // Guarded: his guard pose alone is drawn as soon as an attack
            // is near, so its attack box must be seen to reach him.
            Some(Outcome::Guard)
        } else if view.foe.jumping() {
            Some(Outcome::Jumped)
        } else if !view.me.attacking() && view.frame > self.pressed + 6 {
            Some(Outcome::Whiff)
        } else if view.frame > self.pressed + WATCH_FRAMES {
            Some(Outcome::Nothing)
        } else {
            None
        }
    }
}

crate::canonical::canonical_enum!(Situation, "FairCpuSituation.v1";
    NeutralFar => 0, NeutralClose => 1, ItHit => 2, ItWasBlocked => 3, OpponentKnockedDown => 4,
    OpponentWhiffed => 5, Defending => 6, KnockedDown => 7,
);
crate::canonical::canonical_struct!(Judged, "FairCpuJudged.v2", { situation, corner, foe_cornered });
crate::canonical::canonical_struct!(Judge, "FairCpuJudge.v6", {
    foe_attack_shown, foe_attack_met, foe_attack_began, blockstun_from, blockstun_at, foe_attack_grounded, foe_reach, foe_reach_seen, foe_attack_from, blocked, foe_down, foe_down_since, rise_frames,
});
crate::canonical::canonical_struct!(Running, "FairCpuGamePlan.v4", { plan, step, acted, pressed, began, hit });
crate::canonical::canonical_struct!(RouteRun, "FairCpuRouteRun.v2", {
    fighter, revision, loadout, index, walk, at, stop, facing_right, hit, guarded, reset,
});
crate::canonical::canonical_struct!(Reset, "FairCpuReset.v1", { kind, free, ahead, gap });
crate::canonical::canonical_struct!(ResetWait, "FairCpuResetWait.v1", { reset, cut, facing_right, hit });
crate::canonical::canonical_enum!(ResetKind, "FairCpuResetKind.v1"; Throw => 0, Low => 1, Bait => 2);
crate::canonical::canonical_enum!(Outcome, "FairCpuOutcome.v1"; Hit => 0, Guard => 1, Whiff => 2, Jumped => 3, Nothing => 4);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canonical_hash_covers_the_situation_the_judging_memory_and_the_game_plan() {
        use crate::canonical::{Canonical, CanonicalHasher};
        let hash = |cpu: &crate::fair_cpu::FairCpu| {
            let mut out = CanonicalHasher::new();
            cpu.encode(&mut out);
            out.finish()
        };
        let plain = crate::fair_cpu::FairCpu::new();
        let mut changed = [plain; 4];
        changed[0].judged.situation = Situation::ItHit;
        changed[1].judged.corner = true;
        changed[2].judge.blocked = 3;
        changed[3].game = Some(Running { plan: 0, step: 1, acted: true, pressed: 9, began: 4, hit: false });
        for cpu in &changed {
            assert_ne!(hash(cpu), hash(&plain), "{cpu:?}");
        }
        assert_ne!(hash(&plain.force_stronger(true)), hash(&plain));
    }

    fn ryu_sf6() -> &'static crate::route_book::RouteBook {
        crate::route_book::book(TournamentFighter::Ryu, crate::era::Revision::Sf6, crate::era::Loadout::Standard).unwrap()
    }

    #[test]
    fn a_super_is_spent_in_a_route_only_when_the_round_state_allows_and_drive_above_the_reserve() {
        // SC8 (`#200`).
        let book = ryu_sf6();
        let super_route = book.routes.iter().find(|r| r.super_spent > 0 && r.reach == Reach::Punish).expect("a punish into a Super Art");
        // From where the book proves it: a Super punish is not taken much
        // nearer (SC13).
        let at = Moment { gap: super_route.gap, ..moment(&[Reach::Punish]) };
        let keep = Moment { spend_super: false, ..at };
        assert!(!keep.allows(super_route), "kept while the round state says keep");
        assert!(Moment { spend_super: true, ..at }.allows(super_route), "spent when it says spend");
        let lethal = Moment { foe_health: super_route.damage / 20, ..keep };
        assert!(lethal.allows(super_route), "spent whatever to win the round");
        // The same points of a sturdier fighter's bar (Sagat's 1008 in the
        // modern styles) take more damage: not a round won; of a frailer
        // one's (Gen's 864), a round won from more points left.
        assert!(!Moment { foe_max_sub_hp: 1008, ..lethal }.allows(super_route), "not lethal against a 1008-point bar");
        let points = u16::try_from(u32::from(super_route.damage) * 48 / 864).unwrap();
        assert!(Moment { foe_health: points, foe_max_sub_hp: 864, ..keep }.allows(super_route), "lethal against an 864-point bar");
        // A route that spends Drive leaves the reserve in the gauge.
        let mut drive_route = super_route.clone();
        drive_route.super_spent = 0;
        drive_route.drive_spent = 100;
        let low = Moment { meters: [0, 150], ..moment(&[Reach::Punish]) };
        assert!(low.allows(&drive_route));
        assert!(!Moment { drive_reserve: 72, ..low }.allows(&drive_route));
        assert!(Moment { drive_reserve: 50, ..low }.allows(&drive_route), "down to the reserve exactly");
        // Not down to burnout, unless it wins the round.
        let last = Moment { meters: [0, 100], avoid_burnout: true, ..moment(&[Reach::Punish]) };
        assert!(!last.allows(&drive_route));
        assert!(Moment { foe_health: drive_route.damage / 20, ..last }.allows(&drive_route));
        assert!(Moment { avoid_burnout: false, ..last }.allows(&drive_route), "the old play spends it");
    }

    fn moment(reaches: &[Reach]) -> Moment<'_> {
        Moment {
            reaches,
            era: true,
            normals_only: false,
            true_combos: false,
            cancels: false,
            crouching: false,
            meters: [300, 60_000],
            corner: false,
            gap: 20,
            walk: 0,
            foe_health: 48,
            foe_max_sub_hp: crate::era_fight::FULL_SUB_HP,
            spend_super: true,
            drive_reserve: 0,
            avoid_burnout: false,
            foe: None,
        }
    }

    #[test]
    fn a_punish_that_spends_a_super_is_not_taken_much_nearer_than_it_is_proven() {
        let routes = &ryu_sf6().routes;
        let with = routes.iter().find(|r| r.reach == Reach::Punish && r.super_spent > 0).unwrap();
        let without = routes.iter().find(|r| r.reach == Reach::Punish && r.super_spent == 0).unwrap();
        let at = |route: &Route, gap: i16, walk: u8| Moment { gap, walk, ..moment(&[Reach::Punish]) }.allows(route);
        assert!(at(with, with.gap, 6) && at(with, with.gap - ROUTE_NEAR, 6), "from its gap or a little nearer");
        assert!(!at(with, with.gap - ROUTE_NEAR - 1, 6), "much nearer");
        assert!(at(without, without.gap - ROUTE_NEAR - 10, 0) && at(without, without.gap + 2, 6), "a punish spending none keeps its range");
    }

    #[test]
    fn a_route_the_book_records_failing_on_the_opponent_is_not_taken_against_him() {
        let mut route = ryu_sf6().routes.iter().find(|r| r.stances.contains(&Stance::Crouching)).unwrap().clone();
        route.fails_on = vec!["Sagat Sf6/Standard crouching".into(), "Gen Sfz3/XIsm".into()];
        let open = Moment { gap: route.gap, ..moment(&[Reach::Ground, Reach::Punish]) };
        assert!(open.allows(&route), "nobody named");
        let against = |foe: &'static str, crouching: bool| Moment { foe: Some(foe), crouching, ..open }.allows(&route);
        assert!(!against("Gen Sfz3/XIsm", false) && !against("Gen Sfz3/XIsm", true), "every stance");
        assert!(!against("Sagat Sf6/Standard", true) && against("Sagat Sf6/Standard", false), "one stance");
        assert!(against("Ken Sf6/Standard", true), "another opponent");
    }

    #[test]
    fn only_routes_the_rules_the_stance_the_meter_and_the_distance_allow_are_taken() {
        let routes = &ryu_sf6().routes;
        let all = [Reach::Ground, Reach::Punish];
        // Each route from its own widest start, unless the distance is what
        // is checked.
        let at = |m: &Moment<'_>, r: &Route| Moment { gap: r.gap, reaches: m.reaches, ..*m }.allows(r);
        let open = moment(&all);
        assert!(routes.iter().all(|r| at(&open, r)), "with everything in its favour every route is allowed");
        let only = |m: &Moment, keep: &dyn Fn(&Route) -> bool| routes.iter().all(|r| at(m, r) == keep(r));
        assert!(only(&moment(&[Reach::Ground]), &|r| r.reach == Reach::Ground));
        assert!(only(&Moment { era: false, ..moment(&all) }, &|r| !r.true_combos && !r.cancels));
        assert!(only(&Moment { crouching: true, ..moment(&all) }, &|r| r.stances.contains(&Stance::Crouching)));
        assert!(only(&Moment { meters: [0, 60_000], ..moment(&all) }, &|r| r.super_spent == 0));
        // From 50 pixels: routes whose widest start is 50 or more, and no
        // more than `ROUTE_NEAR` past it; a walk of 3 frames reaches 6 more.
        let from = |m: &Moment, keep: &dyn Fn(&Route) -> bool| routes.iter().all(|r| m.allows(r) == keep(r));
        let near = |r: &Route| r.reach != Reach::Ground || r.gap <= 50 + ROUTE_NEAR;
        assert!(from(&Moment { gap: 50, ..moment(&all) }, &|r| r.gap >= 50 && near(r)));
        assert!(from(&Moment { gap: 50, walk: 3, ..moment(&all) }, &|r| r.gap >= 44 && near(r)));
        assert!(routes.iter().any(|r| r.gap < 50 - ROUTE_NEAR) && routes.iter().any(|r| r.gap >= 50), "both kinds are in the book");
        assert!(routes.iter().any(|r| !r.stances.contains(&Stance::Crouching)) && routes.iter().any(|r| r.super_spent > 0));
    }

    #[test]
    fn a_route_that_wins_the_round_scores_above_any_that_does_not_and_a_recent_one_lower() {
        let routes = &ryu_sf6().routes;
        let weakest = routes.iter().min_by_key(|r| r.damage).unwrap();
        let strongest = routes.iter().max_by_key(|r| r.damage).unwrap();
        // The opponent has just less health than the weakest route does.
        let low = Moment { foe_health: weakest.damage / 20, ..moment(&[Reach::Ground]) };
        assert!(low.score(weakest, true) > moment(&[Reach::Ground]).score(strongest, false));
        let full = moment(&[Reach::Ground]);
        assert!(full.score(strongest, true) < full.score(strongest, false));
    }

    #[test]
    fn far_neutral_plans_walk_in_and_throw_projectiles() {
        let far: Vec<&GamePlan> = RYU_SF6.iter().filter(|p| p.situation == Situation::NeutralFar && !p.cornered_only).collect();
        let has = |act: &dyn Fn(Act) -> bool| far.iter().any(|p| p.steps.iter().any(|s| act(s.act)));
        assert!(has(&|a| a == Act::Shot), "a projectile from afar");
        assert!(has(&|a| matches!(a, Act::Edge | Act::Do(Choice::Approach))), "walking in to take space");
        assert!(has(&|a| a == Act::Do(Choice::JumpIn)), "a jump-in from its range");
        // The projectile only from far enough off, the jump-in only from the
        // range a forward jump carries him.
        for plan in &far {
            if plan.steps[0].act == Act::Shot {
                assert!(plan.gap.0 >= SHOT_GAP);
            }
            if plan.steps[0].act == Act::Do(Choice::JumpIn) {
                assert_eq!(plan.gap, JUMP_IN_GAP);
            }
        }
        let close = RYU_SF6.iter().filter(|p| p.situation == Situation::NeutralClose);
        assert!(close.clone().any(|p| p.steps.iter().any(|s| matches!(s.act, Act::Step { forward: false }))), "stepping out at close range");
    }

    #[test]
    fn the_hardest_levels_repeat_what_keeps_working_and_the_others_put_variety_first() {
        for level in [Level::Hard, Level::VeryHard] {
            assert!(repeat_score(level, 1, true) > repeat_score(level, 0, false), "{level:?} repeats a plan that worked");
            assert!(repeat_score(level, 2, true) > repeat_score(level, 1, true), "{level:?} the more it keeps working");
            assert_eq!(repeat_score(level, 0, true), 0, "{level:?} does not avoid the last plan");
        }
        for level in [Level::Easy, Level::Normal] {
            assert!(repeat_score(level, 3, true) < repeat_score(level, 0, false), "{level:?} puts variety first");
            assert_eq!(repeat_score(level, 3, false), 0, "{level:?} does not lean on what worked");
        }
        assert!(RYU_SF6.len() <= PLAN_SLOTS);
    }

    #[test]
    fn every_game_plan_step_points_inside_its_plan() {
        for plan in RYU_SF6 {
            for step in plan.steps {
                for next in step.then.into_iter().flatten() {
                    assert!(usize::from(next) < plan.steps.len(), "{step:?}");
                }
            }
        }
    }

    #[test]
    fn under_a_normals_only_mission_a_route_is_taken_only_if_every_step_is_a_plain_normal() {
        // SC23 (`#215`): a World Tour mission that allows only normals and
        // throws. Every route of every book that a moment allows is still
        // allowed under the mission exactly when it is all plain normals
        // and spends no meter; there are both kinds in the books.
        let all = [Reach::Ground, Reach::JumpIn, Reach::Punish, Reach::AntiAir, Reach::DriveRush];
        let (mut plain, mut other) = (0, 0);
        for book in crate::route_book::books() {
            for route in &book.routes {
                let crouching = !route.stances.contains(&Stance::Standing);
                let open = Moment { gap: route.gap, crouching, corner: true, ..moment(&all) };
                if !open.allows(route) {
                    continue;
                }
                let kept = Moment { normals_only: true, ..open }.allows(route);
                assert_eq!(kept, route.normals_only(), "{:?} {:?}: {}", book.fighter, book.revision, route.name());
                if kept {
                    plain += 1;
                } else {
                    other += 1;
                }
            }
        }
        assert!(plain >= 20 && other >= 1_000, "plain-normal routes {plain}, others {other}");
    }

    #[test]
    fn a_plain_normal_route_is_one_of_one_button_normals_and_no_meter() {
        // SC23 (`#215`): Joe's 1987+ links and jump-ins are plain normals;
        // a special, a command normal, a charge move and a two-button era
        // command are not, nor are Ryu's SF6 routes (each has a special).
        let joe = crate::route_book::book(TournamentFighter::Joe, crate::era::Revision::Sf1Plus, crate::era::Loadout::Standard).unwrap();
        let named = |name: &str| joe.routes.iter().find(|r| r.name() == name).unwrap_or_else(|| panic!("{name}"));
        for name in ["LP, LP, LK", "LK, MK", "j.HK, HK"] {
            assert!(named(name).normals_only(), "{name}");
        }
        for name in ["LK, 214+LK", "LP, [2]8+LP, LK", "j.HK, 421+LP"] {
            assert!(!named(name).normals_only(), "{name}");
        }
        let mut route = named("LK, MK").clone();
        route.drive_spent = 1;
        assert!(!route.normals_only(), "meter spent");
        let mut route = named("LK, MK").clone();
        route.steps[1].name = "MP+MK".into();
        assert!(!route.normals_only(), "a two-button era command");
        route.steps[1].name = "f+LK".into();
        assert!(!route.normals_only(), "a command normal");
        assert!(ryu_sf6().routes.iter().all(|r| !r.normals_only()));
    }

    #[test]
    fn a_game_plan_that_needs_a_special_or_an_era_command_is_told_apart() {
        // SC23 (`#215`): the projectile plans and the Drive Rush plan need
        // one; the pokes, the hover, the frame trap, the throw and the
        // jump-in do not. A route step keeps to normals only if the book
        // has a plain-normal route starting that way.
        let plan = |steps: &'static [Step]| GamePlan { steps, ..RYU_SF6[0] };
        for steps in [&RYU_SF6_SHOT[..], &RYU_SF6_SHOT_WATCH[..], &RYU_SF6_SHOT_RUSH[..]] {
            assert!(plan(steps).needs_specials(), "{steps:?}");
        }
        for steps in [&RYU_SF6_POKE[..], &RYU_SF6_HOVER[..], &RYU_SF6_FRAME_TRAP[..], &RYU_SF6_WALK_THROW[..], &RYU_SF6_MEATY[..], &RYU_SF6_BAIT[..], &RYU_SF6_JUMP_IN[..], &RYU_SF6_ROUTE[..]] {
            assert!(!plan(steps).needs_specials(), "{steps:?}");
        }
        assert!(plan(&RYU_SF6_ROUTE).keeps_to_normals(|reach| reach == Reach::Ground));
        assert!(!plan(&RYU_SF6_ROUTE).keeps_to_normals(|_| false), "no plain-normal ground route: the plan cannot be played");
        assert!(plan(&RYU_SF6_POKE).keeps_to_normals(|_| false));
        assert!(!plan(&RYU_SF6_SHOT).keeps_to_normals(|_| true));
    }

    const SF2_CAST: [TournamentFighter; 12] = TournamentFighter::ALL;

    #[test]
    fn every_sf2_table_but_gen_and_eagle_has_a_plan_for_each_situation_any_fighter_can_play() {
        // SC15 (`#207`): close and far neutral, a guarded attack and a
        // knocked-down opponent, each with a plan the fighter can play with
        // what he has in Hyper Fighting and in Super Turbo (at Very
        // Difficult): a projectile (the canon three's Hadoken and Tiger
        // Shot, Retsu's Kiai Shot, Geki's Shuriken, Joe's Shockwave), his
        // tools, the jump-in routes of his books. Gen and Eagle have none
        // yet.
        let wanted = [Situation::NeutralClose, Situation::NeutralFar, Situation::ItWasBlocked, Situation::OpponentKnockedDown];
        for fighter in SF2_CAST {
            let table = plans(Some(fighter), Some(EraFamily::Sf2));
            if matches!(fighter, TournamentFighter::Gen | TournamentFighter::Eagle) {
                assert!(table.is_empty(), "{fighter:?}");
                continue;
            }
            assert!(table.len() <= PLAN_SLOTS, "{fighter:?}");
            use TournamentFighter::{Geki, Joe, Ken, Retsu, Ryu, Sagat};
            let projectile = matches!(fighter, Ryu | Ken | Sagat | Retsu | Geki | Joe);
            let tools: Vec<Tool> = super::super::cast::named(fighter, EraFamily::Sf2).into_iter().filter_map(|(_, tool)| tool).collect();
            for revision in [crate::era::Revision::Hf, crate::era::Revision::St] {
                let book = crate::route_book::book(fighter, revision, crate::era::Loadout::Standard);
                let has_route = |reach: Reach| book.is_some_and(|b| b.routes.iter().any(|r| r.reach == reach));
                for situation in wanted {
                    assert!(
                        table.iter().any(|p| p.situation == situation && p.from.rank() <= Level::VeryHard.rank() && p.playable(projectile, |t| tools.contains(&t), has_route, |_| true)),
                        "{fighter:?} in {revision:?} has no plan for {situation:?} he can play"
                    );
                }
            }
            for plan in table {
                assert!(plan.steps.iter().all(|step| !matches!(step.act, Act::Rush)), "{fighter:?}: the SF2 style has no Drive Rush");
                for step in plan.steps {
                    for next in step.then.into_iter().flatten() {
                        assert!(usize::from(next) < plan.steps.len());
                    }
                }
            }
        }
    }

    #[test]
    fn each_fighter_has_a_plan_for_every_tool_his_kit_names_and_none_for_another() {
        // SC15 (`#207`): a command throw, a teleport or a charge the
        // stronger play reads in his kit is one of his plans, and no plan
        // uses a tool he has not got.
        for fighter in SF2_CAST {
            let named: Vec<Tool> = super::super::cast::named(fighter, EraFamily::Sf2).into_iter().filter_map(|(_, tool)| tool).collect();
            let used: Vec<Tool> = plans(Some(fighter), Some(EraFamily::Sf2))
                .iter()
                .flat_map(|p| p.steps.iter())
                .filter_map(|s| if let Act::Tool(tool) = s.act { Some(tool) } else { None })
                .collect();
            for tool in &named {
                assert!(used.contains(tool), "{fighter:?} has no plan for its {tool:?}");
            }
            for tool in &used {
                assert!(named.contains(tool), "{fighter:?} has a plan for a {tool:?} it has not got");
            }
        }
        // The what-if kits are read by the tables; the canon three's ROM
        // specials keep the old reading, and Gen and Eagle wait for their
        // own game.
        assert_eq!(super::super::cast::covered(EraFamily::Sf2).len(), 7);
        assert!(super::super::cast::covered(EraFamily::Sf6).is_empty());
    }

    #[test]
    fn a_boxless_attack_counts_as_guarded_only_from_the_blockstun_it_caused() {
        // SC15 (`#207`, review): his attack shows no box; the CPU's guard
        // pose alone is no sign it met him. A spell of blockstun that began
        // after it began is; the spell of the attack before it, still felt
        // as it began, is not.
        let body = |x: i16, move_id: u8| Body { x, present: true, move_id, front: x, hurt: (x - 10, x + 10), ..Body::default() };
        let frame = |f: u32, foe_move: u8, me_move: u8| Percept {
            frame: f,
            fighting: true,
            me: body(200, me_move),
            foe: body(160, foe_move),
            ..Percept::default()
        };
        let (guard, stand, attack) = (8, 1, 0x1e);
        let run = |blockstun: &[u32]| {
            // His attack begins on frame 100; before it he stands.
            let mut judge = Judge::default();
            let mut before = frame(94, stand, guard);
            let mut seen = Vec::new();
            for f in 95..130 {
                let view = frame(f, if f < 100 { stand } else { attack }, guard);
                judge.judge(view, Some(before), 60);
                if blockstun.contains(&f) {
                    judge.felt_blockstun(f);
                }
                if f >= 100 {
                    seen.push(judge.recovering_after_meeting(view));
                }
                before = view;
            }
            seen
        };
        assert!(run(&[]).iter().all(|&r| !r), "its guard pose alone");
        assert!(run(&[97, 98, 99, 100, 101, 102]).iter().all(|&r| !r), "a spell begun before it: the last attack's");
        let met = run(&[104, 105, 106]);
        assert!(met[..4].iter().all(|&r| !r), "not before it met him: {met:?}");
        assert!(met[4..].iter().all(|&r| r), "met, and after: {met:?}");
    }

    #[test]
    fn only_mike_in_the_sf2_style_takes_plans_less_than_always() {
        // SC15 (`#207`): a rate below 100 draws a random number, so every
        // other pair, Ryu's SF6 included, plays as before.
        assert_eq!(plan_rate(Some(TournamentFighter::Mike), Some(EraFamily::Sf2)), 40);
        for fighter in SF2_CAST.into_iter().filter(|f| *f != TournamentFighter::Mike) {
            assert_eq!(plan_rate(Some(fighter), Some(EraFamily::Sf2)), 100, "{fighter:?}");
        }
        assert_eq!(plan_rate(Some(TournamentFighter::Mike), Some(EraFamily::Sf6)), 100);
        assert_eq!(plan_rate(Some(TournamentFighter::Ryu), Some(EraFamily::Sf6)), 100);
    }

    #[test]
    fn a_plan_is_playable_only_with_what_its_steps_need() {
        let plan = |steps: &'static [Step]| GamePlan { steps, ..RYU_SF6[0] };
        assert!(plan(&RYU_SF6_POKE).playable(false, |_| false, |_| false, |_| true));
        assert!(!plan(&RYU_SF6_SHOT).playable(false, |_| true, |_| true, |_| true), "a projectile");
        assert!(plan(&RYU_SF6_SHOT).playable(true, |_| false, |_| false, |_| true));
        assert!(!plan(&COMMAND_THROW).playable(true, |tool| tool != Tool::CommandThrow, |_| true, |_| true), "the tool");
        assert!(plan(&COMMAND_THROW).playable(false, |tool| tool == Tool::CommandThrow, |_| false, |_| true));
        assert!(!plan(&JUMP_IN_ROUTE).playable(true, |_| true, |reach| reach != Reach::JumpIn, |_| true), "a jump-in route");
        assert!(plan(&JUMP_IN_ROUTE).playable(false, |_| false, |reach| reach == Reach::JumpIn, |_| true));
        assert!(plan(&RYU_SF6_ROUTE).playable(false, |_| false, |_| false, |_| true), "a ground route is the book's own");
        // Ryu's SF6 table is played as before: he has a projectile and none
        // of his plans uses a tool or a jump-in route.
        assert!(RYU_SF6.iter().all(|p| p.playable(true, |_| false, |_| false, |_| true)));
    }

    #[test]
    fn a_jump_in_route_is_not_begun_much_nearer_than_it_is_proven() {
        // SC15 (`#207`): as a ground route, so the jump does not carry him
        // past the opponent.
        let joe = crate::route_book::book(TournamentFighter::Joe, crate::era::Revision::St, crate::era::Loadout::Standard).unwrap();
        let route = joe.routes.iter().find(|r| r.reach == Reach::JumpIn).expect("Joe's Super Turbo book has jump-ins");
        let at = |gap: i16, walk: u8| Moment { gap, walk, crouching: !route.stances.contains(&Stance::Standing), ..moment(&[Reach::JumpIn]) }.allows(route);
        assert!(at(route.gap, 0) && at(route.gap - ROUTE_NEAR, 0) && at(route.gap + 20, 10));
        assert!(!at(route.gap - ROUTE_NEAR - 1, 0), "much nearer");
        assert!(!at(route.gap + 21, 10), "beyond its walk");
    }
}
