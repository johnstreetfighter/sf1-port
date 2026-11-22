//! The measuring run (SC1, `#193`; spec `#192`): a headless match with the
//! Fair CPU on both pads, and numbers that say how active each side was.
//!
//! It is a developer tool for judging changes to the Fair CPU. No game mode
//! uses it and none of its numbers is a pass mark.
//!
//! The match is a two-pad fight under the Definitive preset, powered on the
//! way the front end powers a Local fight, then stepped on the bare machine
//! with nothing drawn. Player 2's Fair CPU is the one the match
//! configuration asks for; Player 1's is the machine's measuring one
//! ([`crate::run::PowerOnMachine::seed_measuring_cpus`]). Both go through
//! the same code as a CPU opponent in a fight, the Utility AI's era
//! commands included, and not through replay takeover.
//!
//! What is counted is read from the frames, as the Fair CPU itself sees a
//! fighter ([`crate::fair_cpu::Body`]):
//!
//! - **Rounds won:** at each round's end, the side with more health.
//! - **Attacks started:** each time a fighter is first drawn in an attack
//!   move, or in a different one.
//! - **Idle frames:** frames of a live round in which a fighter is free to
//!   act (not attacking, hit or in the air) and has not moved sideways. A
//!   fighter standing in guard counts as idle.
//! - **Openings:** an opening begins when a fighter's health drops while
//!   he is not drawn in a guard, and lasts while he stays in hit
//!   reactions. Its hits are the health drops in it and its damage their
//!   sum, in health bar points (an era style's finer health is read, so a
//!   hit of less than a point still counts). Chip damage through a guard
//!   is not an opening.
//! - **Followed:** openings in which a later hit came from another attack
//!   than the first hit's, which is how often a hit was followed up. The
//!   further hits of one multi-hit move are not a follow-up.
//!
//! These are approximations read once a frame. Known limits: the frame a
//! health drop is drawn against the frame the guard or reaction is drawn
//! has not been checked move by move, so a few hits may be filed as chip or
//! the reverse; the same attack repeated with no other frame between
//! counts as one start; and a round's winner is taken from the two bars,
//! not from the game's own round result.
//!
//! **Dramatic Battle** (SC23, `#215`): [`measure_dramatic`] plays a solo
//! Dramatic match with three Fair CPUs at once, Ally 1's (the machine's
//! measuring one, on Player 1's pad), the boss's and Ally 2's, each with
//! its own random numbers and level, under the rules a World Tour boss
//! mission sets (two lifebars, no friendly fire). It reports the boss's
//! rounds won and lost, the damage the boss dealt and took, and attacks
//! started and idle frames for each fighter, counted as above. A round's
//! winner is read from the three health values the Dramatic rules keep,
//! judged as the game judges a knockout and a time over. Which ally landed
//! a hit on the boss is not told apart. [`boss_ever_lost`] says whether a
//! run shows the boss beaten at all, so an unbeatable boss is seen.

use std::sync::Arc;

use sf_video::callouts::Side;

use crate::config::MatchConfig;
use crate::data_bank::DataBank;
use crate::era::{Loadout, Revision, StylePick};
use crate::fair_cpu::{perceive, Body};
use crate::front::cabinet::Players;
use crate::front::fight::{Fight, FightDips, FightKind, FightSetup};
use crate::roster::{PlayMode, TournamentFighter};

/// The longest a match may run: three rounds on the clock and the scenes
/// between them fit well inside it.
const FRAME_BUDGET: u32 = 60 * 600;
/// Frames out of hit reactions after which an opening has ended. A health
/// drop can be drawn a frame or two before the reaction it causes.
const OPENING_GAP: u32 = 3;
/// The original's winner byte (`0xffc009`): 1 Player 1, 2 Player 2.
const MATCH_WINNER: usize = 0x4009;
/// An era style's health units in one health bar point.
const UNITS_PER_POINT: u32 = 20;

/// One match to play: each side's fighter and era style, the difficulty
/// level each Fair CPU plays at (1 Easy to 4 Very Difficult), and the seed
/// of their random numbers. Two different levels (SC11, `#203`) set the
/// match's own difficulty to the higher, and each Fair CPU plays at its
/// own; only the stronger play can be measured so, as the old play's
/// Utility AI commands follow the match's difficulty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeasureSpec {
    pub fighters: [TournamentFighter; 2],
    pub styles: [(Revision, Loadout); 2],
    pub difficulty: [u8; 2],
    pub seed: u64,
    /// Which sides play the stronger Fair CPU's game (spec `#192`); the
    /// others play the old one.
    pub stronger: [bool; 2],
}

/// What one side did in a match. See the module docs for each count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SideReport {
    pub rounds_won: u32,
    pub attacks: u32,
    pub idle_frames: u32,
    pub openings: u32,
    pub hits: u32,
    /// In twentieths of a health bar point.
    pub damage: u32,
    pub followed: u32,
}

/// One match's numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchReport {
    pub spec: MeasureSpec,
    /// The match reached its end inside the frame budget.
    pub ended: bool,
    /// The side the game named the winner, if it named one.
    pub winner: Option<usize>,
    /// Frames stepped from the first live frame, and those of live rounds.
    pub frames: u32,
    pub live_frames: u32,
    pub sides: [SideReport; 2],
}

impl SideReport {
    fn columns(&self, live_frames: u32) -> String {
        let per = |a: u32, b: u32| if b == 0 { "0.00".to_string() } else { format!("{:.2}", f64::from(a) / f64::from(b)) };
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.rounds_won,
            per(self.attacks * 3600, live_frames),
            per(self.idle_frames * 100, live_frames),
            self.openings,
            per(self.hits, self.openings),
            per(self.damage, self.openings * UNITS_PER_POINT),
            per(self.followed, self.openings),
        )
    }
}

impl MatchReport {
    /// The column headings of [`Self::row`], tab-separated.
    pub fn header() -> String {
        let mut out = String::from("p1\tp1_style\tp1_cpu\tp2\tp2_style\tp2_cpu\tlevel\tseed\tended\twinner\tframes\tlive_frames");
        for side in ["p1", "p2"] {
            for column in ["rounds", "attacks_per_min", "idle_pct", "openings", "hits_per_opening", "damage_per_opening", "follow_rate"] {
                out.push_str(&format!("\t{side}_{column}"));
            }
        }
        out
    }

    /// This match as one tab-separated row.
    pub fn row(&self) -> String {
        let style = |(revision, loadout): (Revision, Loadout)| format!("{revision:?}/{loadout:?}");
        let cpu = |stronger: bool| if stronger { "new" } else { "old" };
        let s = &self.spec;
        format!(
            "{:?}\t{}\t{}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            s.fighters[0],
            style(s.styles[0]),
            cpu(s.stronger[0]),
            s.fighters[1],
            style(s.styles[1]),
            cpu(s.stronger[1]),
            if s.difficulty[0] == s.difficulty[1] { s.difficulty[1].to_string() } else { format!("{}/{}", s.difficulty[0], s.difficulty[1]) },
            s.seed,
            if self.ended { "yes" } else { "no" },
            self.winner.map_or("-".to_string(), |w| format!("p{}", w + 1)),
            self.frames,
            self.live_frames,
            self.sides[0].columns(self.live_frames),
            self.sides[1].columns(self.live_frames),
        )
    }
}

/// An opening one side has made that is still going on.
#[derive(Clone, Copy, Default)]
struct Opening {
    /// The attacker's count of attacks started when the first hit landed.
    attacks_at_first_hit: u32,
    followed: bool,
    /// Frames the defender has been out of hit reactions.
    quiet: u32,
}

/// Play `spec`'s match on a machine running `program_rom` and count what
/// each side did. `data` is the extended data bank holding the bosses'
/// player tables; without one only Ryu and Ken can be played.
pub fn measure_match(program_rom: &[u8], data: Option<Arc<DataBank>>, spec: &MeasureSpec) -> Result<MatchReport, String> {
    let picks = spec.styles.map(|(revision, loadout)| StylePick::for_revision(revision, loadout));
    for (side, pick) in picks.iter().enumerate() {
        if !pick.available_for(spec.fighters[side], true) {
            return Err(format!("{:?} has no {:?} style", spec.fighters[side], spec.styles[side]));
        }
    }
    if let Some(bad) = spec.difficulty.iter().find(|d| !(1..=4).contains(*d)) {
        return Err(format!("difficulty level {bad} is not 1 to 4"));
    }
    if spec.difficulty[0] != spec.difficulty[1] && !spec.stronger.iter().all(|&new| new) {
        return Err("two different levels need both sides to play the new Fair CPU: the old one's Utility AI follows the match's level".into());
    }
    let match_level = spec.difficulty[0].max(spec.difficulty[1]);
    let mut config = MatchConfig::definitive();
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = spec.fighters[0];
    config.roster.p2 = spec.fighters[1];
    config.roster.what_if_styles = true;
    config.p2_fair_cpu = true;
    config.difficulty = match_level;
    let setup = FightSetup {
        kind: FightKind::Local,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips { difficulty: match_level, round_time: 100 }.dsw2(),
    };
    let data = data.unwrap_or_else(|| Arc::new(DataBank::build(program_rom, &[])));
    let mut fight = Fight::start_banked(program_rom, setup, Some(0), None, Some(data))
        .map_err(|event| format!("the fight did not start: {event:?}"))?;
    fight.set_selected_styles(picks.map(Some));
    let mut machine = fight.machine().clone();
    machine.seed_measuring_cpus(spec.seed, spec.stronger, spec.difficulty.map(crate::fair_cpu::Level::from_difficulty));

    let mut report = MatchReport { spec: *spec, ended: false, winner: None, frames: 0, live_frames: 0, sides: [SideReport::default(); 2] };
    let mut before: Option<([Body; 2], [u32; 2])> = None;
    let mut open: [Option<Opening>; 2] = [None; 2];
    let mut started = false;
    for _ in 0..FRAME_BUDGET {
        machine.set_cabinet_inputs(false, false, false, false, false);
        machine.step();
        let addresses = machine.unrecovered();
        if !addresses.is_empty() {
            return Err(format!("untranslated branches at frame {}: {addresses:x?}", machine.state.frame));
        }
        let view = perceive(machine.state.frame, machine.in_fight(), machine.work_ram(), &machine.state.objectram, Side::P1);
        let now = [view.me, view.foe];
        // Health in an era style's finer units where a fighter has them.
        let era = machine.era_fighters();
        let health: [u32; 2] = std::array::from_fn(|i| {
            if era[i].max_sub_hp != 0 { u32::from(era[i].sub_hp) } else { u32::from(now[i].health) * UNITS_PER_POINT }
        });
        started |= view.fighting;
        report.frames += u32::from(started);
        if !view.fighting {
            if let Some((_, last_health)) = before.take() {
                // The round is over: the bars as they stand name its winner.
                let health = if now[0].present && now[1].present { health } else { last_health };
                match health[0].cmp(&health[1]) {
                    std::cmp::Ordering::Greater => report.sides[0].rounds_won += 1,
                    std::cmp::Ordering::Less => report.sides[1].rounds_won += 1,
                    std::cmp::Ordering::Equal => {}
                }
                open = [None; 2];
            }
            if machine.match_over() {
                report.ended = true;
                report.winner = match machine.work_ram()[MATCH_WINNER] {
                    1 => Some(0),
                    0 => None,
                    _ => Some(1),
                };
                break;
            }
            continue;
        }
        report.live_frames += 1;
        if let Some((last, last_health)) = before {
            for me in 0..2 {
                let foe = 1 - me;
                let side = &mut report.sides[me];
                if now[me].attacking() && (!last[me].attacking() || now[me].move_id != last[me].move_id) {
                    side.attacks += 1;
                }
                if now[me].free() && last[me].free() && now[me].x == last[me].x {
                    side.idle_frames += 1;
                }
                let drop = last_health[foe].saturating_sub(health[foe]);
                if drop > 0 && !now[foe].guarding() {
                    let attacks = side.attacks;
                    let opening = open[me].get_or_insert_with(|| {
                        side.openings += 1;
                        Opening { attacks_at_first_hit: attacks, ..Opening::default() }
                    });
                    opening.quiet = 0;
                    side.hits += 1;
                    side.damage += drop;
                    if !opening.followed && attacks > opening.attacks_at_first_hit {
                        opening.followed = true;
                        side.followed += 1;
                    }
                } else if let Some(opening) = open[me].as_mut() {
                    // Hit reactions only: rising from a throw is not part
                    // of the opening, so `Body::reacting` is too wide here.
                    opening.quiet = if now[foe].move_id >= 0x30 { 0 } else { opening.quiet + 1 };
                    if opening.quiet >= OPENING_GAP {
                        open[me] = None;
                    }
                }
            }
        }
        before = Some((now, health));
    }
    Ok(report)
}

/// One solo Dramatic match to play (SC23, `#215`): Ally 1, the boss and
/// Ally 2, in the native slot order, each with its fighter and era style
/// and played by a Fair CPU of its own (Ally 1's is the machine's
/// measuring one, Player 1's), at its own level, from one seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DramaticSpec {
    pub fighters: [TournamentFighter; 3],
    /// Each fighter's era revision: Dramatic Battle plays each in its
    /// revision's first loadout.
    pub styles: [Revision; 3],
    pub difficulty: [u8; 3],
    pub seed: u64,
    pub stronger: [bool; 3],
}

/// What one fighter of a Dramatic match did. See the module docs for each
/// count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DramaticSide {
    pub attacks: u32,
    pub idle_frames: u32,
}

/// One Dramatic match's numbers: above all the boss's results, so a boss
/// that cannot be beaten is seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DramaticReport {
    pub spec: DramaticSpec,
    pub ended: bool,
    /// Rounds the boss won, lost (the allies won), and drawn (a time over
    /// with the shares of health even, or a double knockout).
    pub boss_rounds_won: u32,
    pub boss_rounds_lost: u32,
    pub rounds_drawn: u32,
    pub frames: u32,
    pub live_frames: u32,
    /// Damage the boss dealt the two allies, and the allies dealt the boss
    /// (together: which of them landed a hit is not told apart), in
    /// twentieths of a health bar point.
    pub boss_damage: u32,
    pub team_damage: u32,
    /// Ally 1, the boss, Ally 2.
    pub sides: [DramaticSide; 3],
}

impl DramaticReport {
    /// The match's winner by rounds: `Some(true)` the boss, `Some(false)`
    /// the allies, `None` even.
    pub fn boss_won(&self) -> Option<bool> {
        match self.boss_rounds_won.cmp(&self.boss_rounds_lost) {
            std::cmp::Ordering::Greater => Some(true),
            std::cmp::Ordering::Less => Some(false),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// The column headings of [`Self::row`], tab-separated.
    pub fn header() -> String {
        let mut out = String::from(
            "ally1\tally1_style\tally1_cpu\tboss\tboss_style\tboss_cpu\tally2\tally2_style\tally2_cpu\tlevel\tseed\tended\twinner\tframes\tlive_frames\tboss_rounds_won\tboss_rounds_lost\trounds_drawn\tboss_damage\tteam_damage",
        );
        for side in ["ally1", "boss", "ally2"] {
            for column in ["attacks_per_min", "idle_pct"] {
                out.push_str(&format!("\t{side}_{column}"));
            }
        }
        out
    }

    /// This match as one tab-separated row.
    pub fn row(&self) -> String {
        let s = &self.spec;
        let per = |a: u32, b: u32| if b == 0 { "0.00".to_string() } else { format!("{:.2}", f64::from(a) / f64::from(b)) };
        let cpu = |stronger: bool| if stronger { "new" } else { "old" };
        let level = if s.difficulty.iter().all(|&d| d == s.difficulty[0]) {
            s.difficulty[0].to_string()
        } else {
            s.difficulty.map(|d| d.to_string()).join("/")
        };
        let mut out = String::new();
        for i in 0..3 {
            out.push_str(&format!("{:?}\t{:?}\t{}\t", s.fighters[i], s.styles[i], cpu(s.stronger[i])));
        }
        out.push_str(&format!(
            "{level}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            s.seed,
            if self.ended { "yes" } else { "no" },
            match self.boss_won() {
                Some(true) => "boss",
                Some(false) => "allies",
                None => "-",
            },
            self.frames,
            self.live_frames,
            self.boss_rounds_won,
            self.boss_rounds_lost,
            self.rounds_drawn,
            per(self.boss_damage, UNITS_PER_POINT),
            per(self.team_damage, UNITS_PER_POINT),
        ));
        for side in &self.sides {
            out.push_str(&format!(
                "\t{}\t{}",
                per(side.attacks * 3600, self.live_frames),
                per(side.idle_frames * 100, self.live_frames),
            ));
        }
        out
    }
}

/// Who won a solo Dramatic round that ended with `health` (Ally 1, the
/// boss, Ally 2), judged as the game judges it: `Some(true)` the boss,
/// `Some(false)` the allies, `None` a draw. The boss out of health loses
/// it, both allies out of health win it for him, all three is a double
/// knockout; at time over the healthier ally's share of his whole health
/// against the boss's share of his, and even shares draw.
pub fn dramatic_round(health: [u16; 3]) -> Option<bool> {
    use crate::dramatic::{ALLY_SUB_HP, BOSS_SUB_HP};
    let [ally1, boss, ally2] = health;
    let allies_out = ally1 == 0 && ally2 == 0;
    match (boss == 0, allies_out) {
        (true, true) => None,
        (true, false) => Some(false),
        (false, true) => Some(true),
        (false, false) => {
            let boss = u32::from(boss) * u32::from(ALLY_SUB_HP);
            let team = u32::from(ally1.max(ally2)) * u32::from(BOSS_SUB_HP);
            match boss.cmp(&team) {
                std::cmp::Ordering::Greater => Some(true),
                std::cmp::Ordering::Less => Some(false),
                std::cmp::Ordering::Equal => None,
            }
        }
    }
}

/// Whether any of `reports` shows the boss losing a round. A boss that
/// never does across a measuring run is flagged for the maintainer (SC23,
/// `#215`).
pub fn boss_ever_lost(reports: &[DramaticReport]) -> bool {
    reports.iter().any(|report| report.boss_rounds_lost > 0)
}

/// The rules of a solo Dramatic fight for `spec`, as a World Tour boss
/// mission sets them: two lifebars, no friendly fire. The match's
/// difficulty is the highest level asked for.
pub fn dramatic_config(spec: &DramaticSpec) -> Result<MatchConfig, String> {
    use crate::dramatic::{DramaticRules, LifebarMode};
    if let Some(bad) = spec.difficulty.iter().find(|d| !(1..=4).contains(*d)) {
        return Err(format!("difficulty level {bad} is not 1 to 4"));
    }
    if spec.difficulty.iter().any(|&d| d != spec.difficulty[0]) && !spec.stronger.iter().all(|&new| new) {
        return Err("different levels need every fighter on the new Fair CPU: the old one's Utility AI follows the match's level".into());
    }
    let mut config = MatchConfig::definitive();
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = spec.fighters[0];
    config.roster.p2 = spec.fighters[1];
    config.roster.what_if_styles = true;
    config.p2_fair_cpu = false;
    config.difficulty = spec.difficulty.into_iter().max().unwrap_or(4);
    let rules = DramaticRules {
        co_op: false,
        ally2: spec.fighters[2],
        stage: spec.fighters[1],
        lifebar: LifebarMode::Independent,
        friendly_fire: false,
        styles: spec.styles,
        rivalry: None,
    };
    if !rules.styles_available(&config) {
        return Err(format!("Dramatic Battle cannot give {:?} the styles {:?}", spec.fighters, spec.styles));
    }
    config.dramatic = Some(rules);
    Ok(config)
}

/// Start `spec`'s solo Dramatic fight on a machine running `program_rom`,
/// with a Fair CPU seeded on each of the three fighters, ready to step.
pub fn dramatic_machine(program_rom: &[u8], data: Option<Arc<DataBank>>, spec: &DramaticSpec) -> Result<crate::run::PowerOnMachine, String> {
    let config = dramatic_config(spec)?;
    let level = config.difficulty;
    let setup = FightSetup {
        kind: FightKind::Dramatic { co_op: false },
        players: Players::One,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips { difficulty: level, round_time: 100 }.dsw2(),
    };
    let data = data.unwrap_or_else(|| Arc::new(DataBank::build(program_rom, &[])));
    let fight = Fight::start_banked(program_rom, setup, Some(0), None, Some(data))
        .map_err(|event| format!("the fight did not start: {event:?}"))?;
    let mut machine = fight.machine().clone();
    machine.seed_measuring_dramatic_cpus(spec.seed, spec.stronger, spec.difficulty.map(crate::fair_cpu::Level::from_difficulty));
    Ok(machine)
}

/// Play `spec`'s solo Dramatic match with three Fair CPUs and count what
/// each fighter did and how the boss fared (SC23, `#215`). A round's
/// winner is read as the game judges it: the boss out of health loses it,
/// both allies out of health win it for him, and at time over the
/// healthier ally's share of his health against the boss's share of his.
pub fn measure_dramatic(program_rom: &[u8], data: Option<Arc<DataBank>>, spec: &DramaticSpec) -> Result<DramaticReport, String> {
    use crate::dramatic::{ALLY_SUB_HP, BOSS_SUB_HP};
    use crate::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
    let mut machine = dramatic_machine(program_rom, data, spec)?;
    let mut report = DramaticReport {
        spec: *spec,
        ended: false,
        boss_rounds_won: 0,
        boss_rounds_lost: 0,
        rounds_drawn: 0,
        frames: 0,
        live_frames: 0,
        boss_damage: 0,
        team_damage: 0,
        sides: [DramaticSide::default(); 3],
    };
    let mut before: Option<([Body; 3], [u16; 3])> = None;
    let mut started = false;
    for _ in 0..FRAME_BUDGET {
        machine.set_cabinet_inputs(false, false, false, false, false);
        machine.step();
        let addresses = machine.unrecovered();
        if !addresses.is_empty() {
            return Err(format!("untranslated branches at frame {}: {addresses:x?}", machine.state.frame));
        }
        let in_fight = machine.in_fight();
        let body = |me: u32, foe: u32| {
            crate::fair_cpu::perceive_fighters(machine.state.frame, in_fight, machine.work_ram(), &machine.state.objectram, me, foe).me
        };
        let now = [body(P1_STRUCT, CPU_STRUCT), body(CPU_STRUCT, P1_STRUCT), body(P2_STRUCT, CPU_STRUCT)];
        let health = machine.toggle_state().dramatic.health;
        let fighting = in_fight && now[0].present && now[1].present;
        started |= fighting;
        report.frames += u32::from(started);
        if !fighting {
            if let Some((_, last)) = before.take() {
                // The round is over: judged from the health as it stands
                // (a knockout may end the fight loop on its own frame),
                // or as it last stood in the round if the next has
                // already filled the bars.
                let full = [ALLY_SUB_HP, BOSS_SUB_HP, ALLY_SUB_HP];
                match dramatic_round(if health == full { last } else { health }) {
                    Some(true) => report.boss_rounds_won += 1,
                    Some(false) => report.boss_rounds_lost += 1,
                    None => report.rounds_drawn += 1,
                }
            }
            if machine.match_over() {
                report.ended = true;
                break;
            }
            continue;
        }
        report.live_frames += 1;
        if let Some((last, last_health)) = before {
            for (side, (now, last)) in report.sides.iter_mut().zip(now.iter().zip(last)) {
                if now.attacking() && (!last.attacking() || now.move_id != last.move_id) {
                    side.attacks += 1;
                }
                if now.free() && last.free() && now.x == last.x {
                    side.idle_frames += 1;
                }
            }
            let drop = |i: usize| u32::from(last_health[i].saturating_sub(health[i]));
            report.boss_damage += drop(0) + drop(2);
            report.team_damage += drop(1);
        }
        before = Some((now, health));
    }
    Ok(report)
}
