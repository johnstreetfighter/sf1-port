//! Ticket #152 (`FE3`): the boss ladder report.
//!
//! An ignored-by-default probe, not a gate. For each of the ten bosses it
//! drives a `PowerOnMachine` from power-on under the Definitive Overhaul
//! preset (`MatchConfig::default()` with `roster.p1` set to the boss) through
//! the one-player game with cabinet and player inputs only: coin, 1P Start,
//! the Japan flag, the ladder's ten fights, the bonus stages, the flights, the
//! ending, name entry and the return to the attract. It records the stage the
//! played line reached and every untranslated branch the probe reached, the
//! first of them in ladder order, and a continue probe at the first fight.
//! Then, for each of the twelve tournament fighters, it drives a two-player
//! versus from power-on with the same fighter on both sides and records
//! whether the first round runs cleanly. It writes
//! `docs/reference/boss-ladder-report.md` and passes whenever it does.
//!
//! The probe never writes work RAM or fight state. It reads RAM to time its
//! inputs (the flow task, the fighters' x positions, health and round wins)
//! and uses snapshots to search for winning inputs: a round is played in
//! 60-frame segments; from each segment's snapshot several seeded scripted
//! players are tried, the continuation with the best health difference is
//! kept, and a snapshot whose tries all lose (or that has used up its tries)
//! is backed out of.
//!
//! Run: `cargo test --release -p sf-engine --test boss_ladder_report -- --ignored --nocapture`
//! (about five seconds). Set `BOSS_LADDER_VERBOSE=1` to also print each
//! boss's stage log.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use sf_engine::config::MatchConfig;
use sf_engine::data_bank::DataBank;
use sf_engine::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
use sf_engine::roster::TournamentFighter;
use sf_engine::run::{PowerOnMachine, PowerOnSnapshot};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../assets/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const RECIPES: [&str; 11] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/retsu.json"),
    include_str!("../../../art/recipes/geki.json"),
    include_str!("../../../art/recipes/joe.json"),
    include_str!("../../../art/recipes/mike.json"),
    include_str!("../../../art/recipes/lee.json"),
    include_str!("../../../art/recipes/gen.json"),
    include_str!("../../../art/recipes/birdie.json"),
    include_str!("../../../art/recipes/eagle.json"),
    include_str!("../../../art/recipes/adon.json"),
    include_str!("../../../art/recipes/sagat.json"),
];
const REVIEWS: [&str; 11] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/retsu.json"),
    include_str!("../../../art/review/geki.json"),
    include_str!("../../../art/review/joe.json"),
    include_str!("../../../art/review/mike.json"),
    include_str!("../../../art/review/lee.json"),
    include_str!("../../../art/review/gen.json"),
    include_str!("../../../art/review/birdie.json"),
    include_str!("../../../art/review/eagle.json"),
    include_str!("../../../art/review/adon.json"),
    include_str!("../../../art/review/sagat.json"),
];
const REPORT: &str = "../../docs/reference/boss-ladder-report.md";
const COMMAND: &str =
    "cargo test --release -p sf-engine --test boss_ladder_report -- --ignored --nocapture";

/// Factory DIP words (`sf-full-1p-01`, `sf-full-2p-01`).
const DSW1: u16 = 0xdfff;
const DSW2: u16 = 0xffff;

/// Frames per search segment.
const SEGMENT: u32 = 60;
/// Seeded continuations tried from each kept segment snapshot.
const TRIES_PER_SEGMENT: u32 = 8;
/// Continuations tried from one snapshot before the search backs out of it.
const TRIES_PER_NODE: u32 = 48;
/// Continuations tried in one round before the probe gives up on it.
const TRIES_PER_ROUND: u32 = 20_000;
/// Frames the flow may run between fights (pre-fight, bonus, flight, ending).
const SEGMENT_BUDGET: u32 = 20_000;
/// Seeded first rounds played from each mirror match's round-1 snapshot.
const MIRROR_SAMPLES: u32 = 16;

const GAME_MODE: u32 = 0xffc002;
const STAGE_NO: u32 = 0xffc003;
const OPPONENT_NO: u32 = 0xffc004;
const MATCH_WINNER: u32 = 0xffc009;
const ROUND_NO: u32 = 0xffc00a;
/// The fighter entities (`+0x10` of the player structures in modes 1 and 2).
const P1_ENTITY: u32 = 0xffe400;
const P2_ENTITY: u32 = 0xffea00;

/// The game-flow task's fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;
/// The four-country select loop (`0x3e32c`).
const COUNTRY_SELECT_LOOP: u32 = 0x3e32c;
/// The stage start after the flag (`0x3e59c`).
const STAGE_START: u32 = 0x3e59c;
/// The continue countdown loop (`0x3f162`).
const CONTINUE_LOOP: u32 = 0x3f162;
/// The shared text writer (`0x3bc10`): typing for whichever screen is up.
const TEXT_WRITER: u32 = 0x3bc10;
/// The flow addresses of a round in progress or being decided.
const ROUND_FLOW: [u32; 10] = [
    0x3ea3e, 0x3eafa, 0x3eb28, 0x3eb62, 0x3ec0e, 0x3ec2a, 0x3ed26, 0x3ed96, 0x3ecca, 0x3ed00,
];
const ATTRACT_FLOW: [u32; 8] = [
    0x3ccae, 0x3ccec, 0x3cd50, 0x3cd86, 0x3cda8, 0x3b588, 0x3cdb8, 0x42736,
];

/// The ladder from a Japan start (`0xffc006 = 0`): `STAGE_NO` 0..=4, the two
/// opponents of each (`OPPONENT_NO` 0, 1) and the bonus stage after it, as
/// `sf-full-1p-01` verifies.
const LADDER: [(&str, [TournamentFighter; 2], &str); 5] = [
    (
        "Japan",
        [TournamentFighter::Retsu, TournamentFighter::Geki],
        "power/timing",
    ),
    (
        "USA",
        [TournamentFighter::Joe, TournamentFighter::Mike],
        "10-second",
    ),
    (
        "China",
        [TournamentFighter::Lee, TournamentFighter::Gen],
        "power/timing",
    ),
    (
        "England",
        [TournamentFighter::Birdie, TournamentFighter::Eagle],
        "10-second",
    ),
    (
        "Thailand",
        [TournamentFighter::Adon, TournamentFighter::Sagat],
        "none",
    ),
];

// ---- machine reads -------------------------------------------------------

fn rb(m: &PowerOnMachine, addr: u32) -> u8 {
    if addr >= 0xffe000 {
        m.state.objectram[(addr - 0xffe000) as usize]
    } else {
        m.work_ram()[(addr - 0xff8000) as usize]
    }
}

fn rw(m: &PowerOnMachine, addr: u32) -> u16 {
    u16::from_be_bytes([rb(m, addr), rb(m, addr + 1)])
}

fn rl(m: &PowerOnMachine, addr: u32) -> u32 {
    ((rw(m, addr) as u32) << 16) | rw(m, addr + 2) as u32
}

fn slot(m: &PowerOnMachine, k: u32) -> u32 {
    m.task_positions()
        .iter()
        .find(|&&(s, _)| s == k)
        .map(|&(_, a)| a)
        .unwrap_or(0)
}

fn flow(m: &PowerOnMachine) -> u32 {
    slot(m, 1)
}

/// Health points as the time-over judgement counts them (cells × 4 + rest).
fn health(m: &PowerOnMachine, ps: u32) -> i32 {
    rb(m, ps + 7) as i32 * 4 + rb(m, ps + 6) as i32
}

fn table_name(table: u32) -> String {
    TournamentFighter::from_move_table(table)
        .map(|f| f.name().to_string())
        .unwrap_or_else(|| format!("0x{table:05x}"))
}

fn country_name(stage: u8) -> &'static str {
    LADDER.get(stage as usize).map(|s| s.0).unwrap_or("?")
}

fn opponent_name(stage: u8, opp: u8) -> &'static str {
    LADDER
        .get(stage as usize)
        .and_then(|s| s.1.get(opp as usize))
        .map(|f| f.name())
        .unwrap_or("?")
}

fn in_bonus(fl: u32) -> bool {
    // Both power/timing copies (0x40d30.., 0x41a02..) and both 10-second
    // copies (0x41740.., 0x41fe6..0x422b8).
    (0x40d30..=0x422b8).contains(&fl)
}

fn in_continue(fl: u32) -> bool {
    (0x3f04c..=0x3f1b8).contains(&fl)
}

fn in_game_over(fl: u32) -> bool {
    (0x3f204..=0x3f372).contains(&fl)
}

/// Where the one-player flow is, in the report's terms, from the game-flow
/// task (slot 1), the screen task (slot 2) and the stage bytes. `None` while
/// the shared text writer runs, which belongs to whatever screen is up.
fn one_player_stage(m: &PowerOnMachine) -> Option<String> {
    let fl = flow(m);
    if fl == TEXT_WRITER {
        return None;
    }
    let stage = rb(m, STAGE_NO);
    let opp = rb(m, OPPONENT_NO);
    let round = rb(m, ROUND_NO);
    let country = country_name(stage);
    let name = if ATTRACT_FLOW.contains(&fl) {
        "attract".to_string()
    } else if (0x3dec2..=0x3dfce).contains(&fl) {
        "start screen".into()
    } else if (0x3e0e6..=COUNTRY_SELECT_LOOP).contains(&fl) {
        "country select".into()
    } else if (0x44a40..=0x44c76).contains(&fl) {
        "ending".into()
    } else if stage >= 5 {
        if (0x42422..=0x427e0).contains(&fl) {
            "after the ending: name entry and ranking".into()
        } else {
            "after the ending: game over".into()
        }
    } else if in_continue(fl) {
        "continue".into()
    } else if (0x42422..=0x427e0).contains(&fl) {
        "name entry and ranking".into()
    } else if in_game_over(fl) {
        "game over".into()
    } else if in_bonus(fl) {
        format!("{country} bonus stage")
    } else if (0x3f812..=0x3f8e2).contains(&slot(m, 2)) {
        format!("flight to {country}")
    } else {
        let vs = opponent_name(stage, opp);
        if ROUND_FLOW.contains(&fl) || (round > 0 && rb(m, MATCH_WINNER) == 0) {
            format!("{country} round {} vs {vs}", round + 1)
        } else if rb(m, MATCH_WINNER) != 0 {
            format!("{country} match end vs {vs}")
        } else {
            format!("{country} pre-fight vs {vs}")
        }
    };
    Some(name)
}

/// The same for the two-player versus fight.
fn versus_stage(m: &PowerOnMachine) -> String {
    let fl = flow(m);
    if (0x3dec2..=0x3dfce).contains(&fl) {
        return "start screen".into();
    }
    if (0x3e0e6..=COUNTRY_SELECT_LOOP).contains(&fl) {
        return "country select".into();
    }
    let round = rb(m, ROUND_NO);
    if ROUND_FLOW.contains(&fl) || round > 0 {
        return format!("round {}", round + 1);
    }
    "pre-fight".into()
}

/// What each fighter entity is doing: its move-table slot (`+0xa`) and its
/// move stream (`+0x2c`), read before a step and named only if it matters.
#[derive(Clone, Copy)]
struct Doing([u32; 4]);

impl Doing {
    fn of(m: &PowerOnMachine) -> Self {
        Doing([
            rb(m, P1_ENTITY + 0xa) as u32,
            rl(m, P1_ENTITY + 0x2c),
            rb(m, P2_ENTITY + 0xa) as u32,
            rl(m, P2_ENTITY + 0x2c),
        ])
    }

    fn text(self, other: &str) -> String {
        let [a, b, c, d] = self.0;
        format!("P1 move 0x{a:02x} stream 0x{b:05x}, {other} move 0x{c:02x} stream 0x{d:05x}")
    }
}

// ---- inputs --------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Pad {
    right: bool,
    left: bool,
    down: bool,
    up: bool,
    buttons: [bool; 6],
}

fn set_p1(m: &mut PowerOnMachine, p: Pad) {
    m.set_p1_direction(p.right, p.left, p.down, p.up);
    m.set_p1_buttons(p.buttons);
}

fn set_p2(m: &mut PowerOnMachine, p: Pad) {
    m.set_p2_direction(p.right, p.left, p.down, p.up);
    m.set_p2_buttons(p.buttons);
}

fn tap(button: usize) -> Pad {
    let mut p = Pad::default();
    p.buttons[button] = true;
    p
}

/// A cabinet input held on frames `from..from + 12`, the scenarios' presses.
fn held(frame: u32, from: u32) -> bool {
    (from..from + 12).contains(&frame)
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }

    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 32) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

fn seed_for(parts: &[u64]) -> u64 {
    parts.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &p| {
        (h ^ p).wrapping_mul(0x0100_0000_01b3)
    })
}

#[derive(Clone, Copy)]
enum Stick {
    Neutral,
    Down,
    Forward,
    Back,
    DownForward,
    UpForward,
}

#[derive(Clone, Copy)]
enum Act {
    Idle,
    Walk {
        forward: bool,
    },
    Crouch,
    Attack {
        button: usize,
        stick: Stick,
    },
    Jump {
        forward: bool,
        button: usize,
        at: u16,
    },
}

/// The scripted player: a seeded mix of walking, crouching, jumping and
/// button taps, attacking when within a seeded range of the opponent. Each
/// seed also fixes a style (two favoured buttons, the range, aggression).
struct Bot {
    rng: Rng,
    favoured: [usize; 2],
    favour: u32,
    range: i32,
    aggression: u32,
    act: Act,
    t: u16,
    left: u16,
}

impl Bot {
    fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let favoured = [rng.below(6) as usize, rng.below(6) as usize];
        let favour = 30 + rng.below(70);
        let range = [40, 56, 72, 96, 128][rng.below(5) as usize];
        let aggression = 35 + rng.below(50);
        Bot {
            rng,
            favoured,
            favour,
            range,
            aggression,
            act: Act::Idle,
            t: 0,
            left: 0,
        }
    }

    fn attack(&mut self) -> (Act, u16) {
        let button = if self.rng.below(100) < self.favour {
            self.favoured[self.rng.below(2) as usize]
        } else {
            self.rng.below(6) as usize
        };
        let stick = match self.rng.below(12) {
            0..=5 => Stick::Neutral,
            6..=7 => Stick::Down,
            8 => Stick::Forward,
            9 => Stick::Back,
            10 => Stick::DownForward,
            _ => Stick::UpForward,
        };
        (Act::Attack { button, stick }, 4 + self.rng.below(20) as u16)
    }

    fn jump(&mut self, forward: bool) -> (Act, u16) {
        let button = self.rng.below(6) as usize;
        let at = 4 + self.rng.below(16) as u16;
        (
            Act::Jump {
                forward,
                button,
                at,
            },
            40,
        )
    }

    fn choose(&mut self, distance: i32) {
        let r = self.rng.below(100);
        let (act, len) = if distance > self.range {
            match r {
                0..=54 => (Act::Walk { forward: true }, 6 + self.rng.below(24) as u16),
                55..=69 => self.attack(),
                70..=79 => self.jump(true),
                80..=91 => (Act::Idle, 4 + self.rng.below(12) as u16),
                _ => (Act::Walk { forward: false }, 4 + self.rng.below(12) as u16),
            }
        } else if r < self.aggression {
            self.attack()
        } else {
            match r - self.aggression {
                0..=9 => (Act::Walk { forward: false }, 4 + self.rng.below(16) as u16),
                10..=17 => (Act::Crouch, 4 + self.rng.below(16) as u16),
                18..=25 => {
                    let forward = self.rng.below(2) == 0;
                    self.jump(forward)
                }
                26..=33 => (Act::Walk { forward: true }, 2 + self.rng.below(8) as u16),
                _ => (Act::Idle, 2 + self.rng.below(10) as u16),
            }
        };
        self.act = act;
        self.t = 0;
        self.left = len;
    }

    /// This frame's pad, given the two fighters' x positions.
    fn pad(&mut self, me_x: i32, opp_x: i32) -> Pad {
        if self.left == 0 {
            self.choose((opp_x - me_x).abs());
        }
        let toward_right = opp_x >= me_x;
        let stick = |p: &mut Pad, forward: bool, down: bool, up: bool| {
            let right = forward == toward_right;
            p.right = right;
            p.left = !right;
            p.down = down;
            p.up = up;
        };
        let mut p = Pad::default();
        match self.act {
            Act::Idle => {}
            Act::Walk { forward } => stick(&mut p, forward, false, false),
            Act::Crouch => p.down = true,
            Act::Attack { button, stick: s } => {
                if self.t < 4 {
                    match s {
                        Stick::Neutral => {}
                        Stick::Down => p.down = true,
                        Stick::Forward => stick(&mut p, true, false, false),
                        Stick::Back => stick(&mut p, false, false, false),
                        Stick::DownForward => stick(&mut p, true, true, false),
                        Stick::UpForward => stick(&mut p, true, false, true),
                    }
                }
                if self.t < 2 {
                    p.buttons[button] = true;
                }
            }
            Act::Jump {
                forward,
                button,
                at,
            } => {
                if self.t < 4 {
                    stick(&mut p, forward, false, true);
                }
                if (at..at + 2).contains(&self.t) {
                    p.buttons[button] = true;
                }
            }
        }
        self.t += 1;
        self.left -= 1;
        p
    }
}

fn x_of(m: &PowerOnMachine, entity: u32) -> i32 {
    rw(m, entity + 0x1e) as i32
}

// ---- untranslated branches -------------------------------------------------

/// An untranslated branch reached: the `unrecovered()` set, where and what
/// the fighters were doing.
#[derive(Clone)]
struct Gap {
    addresses: Vec<u32>,
    frame: u32,
    stage: String,
    doing: Option<String>,
}

impl Gap {
    fn at(m: &PowerOnMachine, stage: String, doing: Option<String>) -> Self {
        Gap {
            addresses: m.unrecovered(),
            frame: m.state.frame,
            stage,
            doing,
        }
    }

    fn addresses(&self) -> String {
        self.addresses
            .iter()
            .map(|a| format!("`0x{a:05x}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn describe(&self) -> String {
        let mut s = format!(
            "{} at frame {} ({}",
            self.addresses(),
            self.frame,
            self.stage
        );
        if let Some(d) = &self.doing {
            let _ = write!(s, "; {d}");
        }
        s.push(')');
        s
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GapKind {
    /// On the played line: the run ended there.
    PlayedLine,
    /// In search tries the played line did not keep.
    Discarded,
    /// In the continue probe's detour from the first fight.
    ContinueProbe,
}

struct FoundGap {
    gap: Gap,
    kind: GapKind,
    /// Search tries that reached this set of branches.
    hits: u32,
}

// ---- power-on to the fight -------------------------------------------------

fn power_on(rom: &[u8], bank: &Arc<DataBank>, cfg: MatchConfig) -> PowerOnMachine {
    let mut m = PowerOnMachine::new(rom.to_vec())
        .with_match_config(cfg)
        .with_data_bank(Arc::clone(bank));
    m.set_dsw(DSW1, DSW2);
    m
}

/// Step with cabinet inputs until the four-country select loop runs, then
/// take the default flag (Japan) with a light-punch tap 20 frames in
/// (`0xff86b0 != 0`), and stop at the stage start (`0x3e59c`). One player:
/// `coin1`, `start1`. Two players: the verified `sf-start-2p-01` sequence
/// (`coin1`, `coin2`, `start2`).
fn drive_to_stage_start(m: &mut PowerOnMachine, two_players: bool) -> Result<(), Gap> {
    let mut pick_at = None;
    while m.state.frame < 4_000 {
        let f = m.state.frame + 1;
        if two_players {
            m.set_cabinet_inputs(held(f, 1451), held(f, 1481), false, held(f, 1561), false);
        } else {
            m.set_cabinet_inputs(held(f, 1451), false, held(f, 1521), false, false);
        }
        if flow(m) == COUNTRY_SELECT_LOOP && pick_at.is_none() {
            pick_at = Some(f + 20);
        }
        let press = pick_at.is_some_and(|p| (p..p + 3).contains(&f));
        set_p1(m, if press { tap(0) } else { Pad::default() });
        set_p2(m, Pad::default());
        let stage = versus_stage(m);
        m.step();
        if !m.unrecovered().is_empty() {
            return Err(Gap::at(m, stage, None));
        }
        if pick_at.is_some() && flow(m) == STAGE_START {
            return Ok(());
        }
    }
    let mut gap = Gap::at(m, versus_stage(m), None);
    gap.stage = format!(
        "the country select was not left by frame {} (flow 0x{:05x})",
        m.state.frame,
        flow(m)
    );
    Err(gap)
}

// ---- the round search ------------------------------------------------------

/// The round being searched: the wins and round counter at its start.
struct RoundStart {
    p1_wins: u8,
    cpu_wins: u8,
    round: u8,
}

impl RoundStart {
    fn of(m: &PowerOnMachine) -> Self {
        RoundStart {
            p1_wins: rb(m, P1_STRUCT + 8),
            cpu_wins: rb(m, CPU_STRUCT + 8),
            round: rb(m, ROUND_NO),
        }
    }
}

enum SegmentEnd {
    Won,
    Lost,
    Draw,
    Gap(Gap),
    Going(i32),
}

/// One search segment: up to `SEGMENT` frames of the scripted player.
fn play_segment(
    m: &mut PowerOnMachine,
    bot: &mut Bot,
    start: &RoundStart,
    stage: &str,
) -> SegmentEnd {
    for _ in 0..SEGMENT {
        let pad = if flow(m) == FIGHT_LOOP {
            bot.pad(x_of(m, P1_ENTITY), x_of(m, P2_ENTITY))
        } else {
            Pad::default()
        };
        set_p1(m, pad);
        let doing = Doing::of(m);
        m.step();
        if !m.unrecovered().is_empty() {
            return SegmentEnd::Gap(Gap::at(m, stage.to_string(), Some(doing.text("CPU"))));
        }
        if rb(m, P1_STRUCT + 8) > start.p1_wins {
            return SegmentEnd::Won;
        }
        // The CPU's round win, or the player knocked out (`+8 == 8`) on the
        // way to it.
        if rb(m, CPU_STRUCT + 8) > start.cpu_wins || rb(m, P1_ENTITY + 8) == 8 {
            return SegmentEnd::Lost;
        }
        if rb(m, ROUND_NO) != start.round {
            return SegmentEnd::Draw;
        }
    }
    SegmentEnd::Going(health(m, P1_STRUCT) - health(m, CPU_STRUCT))
}

struct RoundRecord {
    round: u8,
    won: bool,
    tries: u32,
    segments: u32,
    lost: u32,
    draws: u32,
    gaps: u32,
}

struct FightRecord {
    country: &'static str,
    expected: &'static str,
    cpu_table: String,
    player_table: String,
    rounds: Vec<RoundRecord>,
}

enum Outcome {
    Complete,
    Gap,
    NotWon { stage: String, tries: u32 },
    Stuck(String),
}

struct BossRun {
    boss: TournamentFighter,
    reached: String,
    outcome: Outcome,
    fights: Vec<FightRecord>,
    gaps: Vec<FoundGap>,
    continue_probe: String,
    frames: u32,
    tries: u32,
    /// Every stage change on the played line, and each round's search result.
    log: Vec<(u32, String)>,
}

impl BossRun {
    fn note(&mut self, gap: Gap, kind: GapKind) {
        if kind == GapKind::Discarded {
            if let Some(found) = self
                .gaps
                .iter_mut()
                .find(|g| g.kind == kind && g.gap.addresses == gap.addresses)
            {
                found.hits += 1;
                return;
            }
        }
        self.gaps.push(FoundGap { gap, kind, hits: 1 });
    }
}

/// Search one round from its first fight-loop frame. Returns with the machine
/// just past the player's round win, or `false` when the try cap ran out.
fn search_round(
    m: &mut PowerOnMachine,
    run: &mut BossRun,
    record: &mut RoundRecord,
    stage: &str,
    seed_base: u64,
) -> bool {
    struct Node {
        snapshot: PowerOnSnapshot,
        tried: u32,
    }
    let start = RoundStart::of(m);
    let mut stack = vec![Node {
        snapshot: m.save_snapshot(),
        tried: 0,
    }];
    while record.tries < TRIES_PER_ROUND {
        let depth = stack.len() as u64;
        // A snapshot whose continuations keep dying is backed out of, so the
        // search changes course further back. The round's start stays.
        if depth > 1 && stack.last().is_some_and(|n| n.tried >= TRIES_PER_NODE) {
            stack.pop();
            continue;
        }
        let Some(top) = stack.last_mut() else {
            return false;
        };
        let mut best: Option<(i32, PowerOnSnapshot)> = None;
        for k in 0..TRIES_PER_SEGMENT {
            m.restore_snapshot(&top.snapshot);
            record.tries += 1;
            run.tries += 1;
            let mut bot = Bot::new(seed_for(&[seed_base, depth, (top.tried + k) as u64]));
            match play_segment(m, &mut bot, &start, stage) {
                SegmentEnd::Won => {
                    record.won = true;
                    record.segments = depth as u32;
                    return true;
                }
                SegmentEnd::Lost => record.lost += 1,
                SegmentEnd::Draw => record.draws += 1,
                SegmentEnd::Gap(gap) => {
                    record.gaps += 1;
                    run.note(gap, GapKind::Discarded);
                }
                SegmentEnd::Going(score) => {
                    let better = match &best {
                        Some((b, _)) => score > *b,
                        None => true,
                    };
                    if better {
                        best = Some((score, m.save_snapshot()));
                    }
                }
            }
        }
        top.tried += TRIES_PER_SEGMENT;
        match best {
            Some((_, snapshot)) => stack.push(Node { snapshot, tried: 0 }),
            None if depth > 1 => {
                stack.pop();
            }
            None => {}
        }
    }
    false
}

/// Continue probe, from the first fight's round-1 snapshot: the player idles
/// until the CPU takes the match; when the continue screen opens (`0x3f162`)
/// a coin pulse goes in and then 1P Start, and the probe runs to the rematch's
/// round-1 fight loop.
fn continue_probe(m: &mut PowerOnMachine, run: &mut BossRun) {
    let start = m.state.frame;
    let mut coin_at = None;
    let mut stage = String::from("continue probe");
    while m.state.frame < start + 40_000 {
        let f = m.state.frame + 1;
        if flow(m) == CONTINUE_LOOP && coin_at.is_none() {
            coin_at = Some(f);
        }
        let (coin, start1) = match coin_at {
            Some(c) => (held(f, c), held(f, c + 30)),
            None => (false, false),
        };
        m.set_cabinet_inputs(coin, false, start1, false, false);
        set_p1(m, Pad::default());
        if let Some(s) = one_player_stage(m) {
            stage = s;
        }
        let doing = Doing::of(m);
        m.step();
        if !m.unrecovered().is_empty() {
            let gap = Gap::at(
                m,
                format!("continue probe: {stage}"),
                Some(doing.text("CPU")),
            );
            run.continue_probe = format!("untranslated {}", gap.describe());
            run.note(gap, GapKind::ContinueProbe);
            return;
        }
        if let Some(c) = coin_at {
            if flow(m) == FIGHT_LOOP {
                run.continue_probe = format!(
                    "clean: the CPU took the match, the continue screen opened at frame {c}, coin and 1P Start continued, and the rematch reached its round-1 fight loop at frame {} with nothing untranslated",
                    m.state.frame
                );
                return;
            }
        }
        if in_game_over(flow(m)) {
            run.continue_probe = format!(
                "not accepted: the flow entered game over at frame {}",
                m.state.frame
            );
            return;
        }
    }
    run.continue_probe = format!(
        "stuck: no rematch fight loop by frame {} (flow 0x{:05x})",
        m.state.frame,
        flow(m)
    );
}

/// Pad for the one-player flow outside the fights: a heavy-punch tap every
/// 16 frames in the bonus stages, nothing elsewhere.
fn flow_pad(m: &PowerOnMachine) -> Pad {
    if in_bonus(flow(m)) && m.state.frame % 16 < 2 {
        tap(2)
    } else {
        Pad::default()
    }
}

fn run_boss(rom: &[u8], bank: &Arc<DataBank>, boss: TournamentFighter) -> BossRun {
    let mut cfg = MatchConfig::default();
    cfg.roster.p1 = boss;
    cfg.p2_fair_cpu = true;
    let mut m = power_on(rom, bank, cfg);
    let mut run = BossRun {
        boss,
        reached: "power-on".into(),
        outcome: Outcome::Stuck(String::new()),
        fights: Vec::new(),
        gaps: Vec::new(),
        continue_probe: "not run".into(),
        frames: 0,
        tries: 0,
        log: Vec::new(),
    };
    if let Err(gap) = drive_to_stage_start(&mut m, false) {
        run.frames = m.state.frame;
        if gap.addresses.is_empty() {
            run.outcome = Outcome::Stuck(gap.stage);
        } else {
            run.reached = gap.stage.clone();
            run.outcome = Outcome::Gap;
            run.note(gap, GapKind::PlayedLine);
        }
        return run;
    }
    let mut stage = String::from("country select");
    let mut after_ending = false;
    'ladder: loop {
        // The flow, to the next round's fight loop or to the end.
        let mut steps = 0;
        loop {
            if let Some(s) = one_player_stage(&m) {
                if s != stage {
                    run.log.push((m.state.frame, s.clone()));
                    stage = s;
                }
            }
            if stage == "ending" || stage.starts_with("after the ending") {
                after_ending = true;
            }
            if stage == "attract" {
                if after_ending {
                    run.reached = "returned to the attract after the ending".into();
                    run.outcome = Outcome::Complete;
                } else {
                    run.outcome = Outcome::Stuck(format!(
                        "the flow returned to the attract at frame {} before the ending",
                        m.state.frame
                    ));
                }
                break 'ladder;
            }
            run.reached = stage.clone();
            if flow(&m) == FIGHT_LOOP {
                break;
            }
            if steps > SEGMENT_BUDGET {
                run.outcome = Outcome::Stuck(format!(
                    "no fight or end within {SEGMENT_BUDGET} frames (frame {}, flow 0x{:05x}, {stage})",
                    m.state.frame,
                    flow(&m)
                ));
                break 'ladder;
            }
            let pad = flow_pad(&m);
            set_p1(&mut m, pad);
            let doing = Doing::of(&m);
            let fighting = ROUND_FLOW.contains(&flow(&m));
            m.step();
            steps += 1;
            if !m.unrecovered().is_empty() {
                let doing = fighting.then(|| doing.text("CPU"));
                run.note(Gap::at(&m, stage.clone(), doing), GapKind::PlayedLine);
                run.outcome = Outcome::Gap;
                break 'ladder;
            }
        }

        // A round starts.
        let country_no = rb(&m, STAGE_NO);
        let opp = rb(&m, OPPONENT_NO);
        let round = rb(&m, ROUND_NO);
        if round == 0 || run.fights.is_empty() {
            run.fights.push(FightRecord {
                country: country_name(country_no),
                expected: opponent_name(country_no, opp),
                cpu_table: table_name(rl(&m, CPU_STRUCT + 0x12)),
                player_table: table_name(rl(&m, P1_STRUCT + 0x12)),
                rounds: Vec::new(),
            });
        }
        if run.continue_probe == "not run" {
            let snapshot = m.save_snapshot();
            continue_probe(&mut m, &mut run);
            m.restore_snapshot(&snapshot);
        }
        let mut record = RoundRecord {
            round: round + 1,
            won: false,
            tries: 0,
            segments: 0,
            lost: 0,
            draws: 0,
            gaps: 0,
        };
        let seed_base = seed_for(&[
            boss.move_table() as u64,
            country_no as u64,
            opp as u64,
            round as u64,
        ]);
        let won = search_round(&mut m, &mut run, &mut record, &stage, seed_base);
        run.log.push((
            m.state.frame,
            format!(
                "{stage}: {} after {} tries ({} lost, {} drawn, {} untranslated)",
                if won { "won" } else { "not won" },
                record.tries,
                record.lost,
                record.draws,
                record.gaps
            ),
        ));
        let tries = record.tries;
        if let Some(fight) = run.fights.last_mut() {
            fight.rounds.push(record);
        }
        if !won {
            run.outcome = Outcome::NotWon {
                stage: stage.clone(),
                tries,
            };
            break;
        }
    }
    run.frames = m.state.frame;
    run
}

// ---- mirror matches ------------------------------------------------------

struct MirrorRun {
    fighter: TournamentFighter,
    p1_table: String,
    p2_table: String,
    fight_at: Option<u32>,
    /// Why the fight was not reached, when it was not.
    setup: Option<String>,
    samples: u32,
    clean_samples: u32,
    /// Round-1 results over the clean samples: P1, P2, drawn.
    results: [u32; 3],
    /// Untranslated sets reached, with the samples that reached each.
    gaps: Vec<(Gap, u32)>,
    /// Samples that never reached round 2's fight loop.
    stuck: u32,
    clean: bool,
}

enum Sample {
    Clean(usize),
    Gap(Gap),
    Stuck,
}

/// One seeded round 1 from the round-1 fight-loop snapshot, both sides driven
/// by scripted players, through its result to round 2's fight loop.
fn mirror_sample(m: &mut PowerOnMachine, seed: u64) -> Sample {
    let mut p1 = Bot::new(seed);
    let mut p2 = Bot::new(seed ^ 0x5555_5555);
    let mut result = None;
    let limit = m.state.frame + 20_000;
    while m.state.frame < limit {
        let in_fight = flow(m) == FIGHT_LOOP;
        if let Some(r) = result {
            if in_fight && rb(m, ROUND_NO) == 1 {
                return Sample::Clean(r);
            }
        }
        let (a, b) = (x_of(m, P1_ENTITY), x_of(m, P2_ENTITY));
        if in_fight {
            let (pa, pb) = (p1.pad(a, b), p2.pad(b, a));
            set_p1(m, pa);
            set_p2(m, pb);
        } else {
            set_p1(m, Pad::default());
            set_p2(m, Pad::default());
        }
        let stage = versus_stage(m);
        let doing = Doing::of(m);
        let before = (rb(m, P1_STRUCT + 8), rb(m, P2_STRUCT + 8), rb(m, ROUND_NO));
        m.step();
        if !m.unrecovered().is_empty() {
            return Sample::Gap(Gap::at(m, stage, Some(doing.text("P2"))));
        }
        let after = (rb(m, P1_STRUCT + 8), rb(m, P2_STRUCT + 8), rb(m, ROUND_NO));
        if result.is_none() {
            if after.0 > before.0 {
                result = Some(0);
            } else if after.1 > before.1 {
                result = Some(1);
            } else if after.2 != before.2 {
                result = Some(2);
            }
        }
    }
    Sample::Stuck
}

/// A two-player versus from power-on with `fighter` on both sides, to round
/// 1's fight loop, then `MIRROR_SAMPLES` seeded first rounds from there.
fn run_mirror(rom: &[u8], bank: &Arc<DataBank>, fighter: TournamentFighter) -> MirrorRun {
    let mut cfg = MatchConfig::default();
    cfg.roster.p1 = fighter;
    cfg.roster.p2 = fighter;
    let mut m = power_on(rom, bank, cfg);
    let mut run = MirrorRun {
        fighter,
        p1_table: "-".into(),
        p2_table: "-".into(),
        fight_at: None,
        setup: None,
        samples: 0,
        clean_samples: 0,
        results: [0; 3],
        gaps: Vec::new(),
        stuck: 0,
        clean: false,
    };
    if let Err(gap) = drive_to_stage_start(&mut m, true) {
        run.setup = Some(if gap.addresses.is_empty() {
            gap.stage
        } else {
            format!("untranslated {}", gap.describe())
        });
        return run;
    }
    if rb(&m, GAME_MODE) != 1 {
        run.setup = Some(format!(
            "the two-player start left GAME_MODE = {} (expected 1)",
            rb(&m, GAME_MODE)
        ));
        return run;
    }
    let limit = m.state.frame + SEGMENT_BUDGET;
    while flow(&m) != FIGHT_LOOP {
        if m.state.frame >= limit {
            run.setup = Some(format!(
                "no round-1 fight loop by frame {} (flow 0x{:05x})",
                m.state.frame,
                flow(&m)
            ));
            return run;
        }
        set_p1(&mut m, Pad::default());
        set_p2(&mut m, Pad::default());
        let stage = versus_stage(&m);
        m.step();
        if !m.unrecovered().is_empty() {
            run.setup = Some(format!(
                "untranslated {}",
                Gap::at(&m, stage, None).describe()
            ));
            return run;
        }
    }
    run.fight_at = Some(m.state.frame);
    run.p1_table = table_name(rl(&m, P1_STRUCT + 0x12));
    run.p2_table = table_name(rl(&m, P2_STRUCT + 0x12));
    let snapshot = m.save_snapshot();
    for sample in 0..MIRROR_SAMPLES {
        m.restore_snapshot(&snapshot);
        run.samples += 1;
        let seed = seed_for(&[fighter.move_table() as u64, 0x2_0000, sample as u64]);
        match mirror_sample(&mut m, seed) {
            Sample::Clean(r) => {
                run.clean_samples += 1;
                run.results[r] += 1;
            }
            Sample::Gap(gap) => match run
                .gaps
                .iter_mut()
                .find(|(g, _)| g.addresses == gap.addresses)
            {
                Some((_, n)) => *n += 1,
                None => run.gaps.push((gap, 1)),
            },
            Sample::Stuck => run.stuck += 1,
        }
    }
    run.clean = run.p1_table == fighter.name()
        && run.p2_table == fighter.name()
        && run.clean_samples == run.samples;
    run
}

fn mirror_result(r: &MirrorRun) -> String {
    if let Some(why) = &r.setup {
        return why.clone();
    }
    let mut parts = vec![format!(
        "{} of {} samples clean (round 1: P1 {}, P2 {}, drawn {})",
        r.clean_samples, r.samples, r.results[0], r.results[1], r.results[2]
    )];
    for (g, n) in &r.gaps {
        parts.push(format!(
            "untranslated in {n} {}, first {}",
            if *n == 1 { "sample" } else { "samples" },
            g.describe()
        ));
    }
    if r.stuck > 0 {
        parts.push(format!("{} samples did not reach round 2", r.stuck));
    }
    parts.join("; ")
}

// ---- the report ----------------------------------------------------------

fn kind_note(g: &FoundGap) -> String {
    match g.kind {
        GapKind::PlayedLine => "on the played line; the run ended there".into(),
        GapKind::Discarded => format!(
            "in {} discarded search {}; the played line went round it",
            g.hits,
            if g.hits == 1 { "try" } else { "tries" }
        ),
        GapKind::ContinueProbe => {
            "in the continue probe's detour; the ladder went on from the snapshot".into()
        }
    }
}

fn first_gap_cell(run: &BossRun) -> String {
    match (run.gaps.first(), &run.outcome) {
        (Some(g), _) => format!("{}, {}", g.gap.describe(), kind_note(g)),
        (None, Outcome::Complete) => "none: complete".into(),
        (None, _) => "none reached before the run stopped".into(),
    }
}

/// What the engine source says about a branch this report has met.
fn address_note(addresses: &[u32]) -> Option<&'static str> {
    addresses.contains(&0x44506).then_some(
        "`0x44506` is noted by `park_from` in `crates/sf-engine/src/fighter.rs` when a loaded frame's sprite grid has run past the 24 entries (`0x600` bytes) of the fighter's object region, a case the translation does not cover",
    )
}

/// The stages the played line went through, counted from its stage log.
fn stages_passed(run: &BossRun) -> String {
    let distinct = |pred: &dyn Fn(&str) -> bool| {
        let mut seen: Vec<&str> = run
            .log
            .iter()
            .map(|(_, l)| l.as_str())
            .filter(|l| pred(l))
            .collect();
        seen.sort_unstable();
        seen.dedup();
        seen.len()
    };
    let rounds: usize = run
        .fights
        .iter()
        .map(|f| f.rounds.iter().filter(|r| r.won).count())
        .sum();
    let mut parts = vec![
        format!("{} pre-fights", distinct(&|l| l.contains(" pre-fight vs "))),
        format!("{rounds} rounds won"),
        format!(
            "{} bonus stages",
            distinct(&|l| l.ends_with(" bonus stage"))
        ),
        format!("{} flights", distinct(&|l| l.starts_with("flight to "))),
    ];
    if distinct(&|l| l == "ending") > 0 {
        parts.push("the ending".into());
    }
    if distinct(&|l| l == "after the ending: name entry and ranking") > 0 {
        parts.push("name entry and the ranking".into());
    }
    if matches!(run.outcome, Outcome::Complete) {
        parts.push("the game over and the return to the attract".into());
    }
    parts.join(", ")
}

fn write_findings(w: &mut String, bosses: &[BossRun], mirrors: &[MirrorRun]) {
    let complete: Vec<&str> = bosses
        .iter()
        .filter(|r| matches!(r.outcome, Outcome::Complete))
        .map(|r| r.boss.name())
        .collect();
    let _ = writeln!(
        w,
        "- **Played line.** Complete for {} of {} bosses (power-on through the ten fights, the bonus stages, the flights, the ending, name entry and back to the attract with `unrecovered()` empty){}{}.",
        complete.len(),
        bosses.len(),
        if complete.is_empty() { "" } else { ": " },
        complete.join(", ")
    );
    for r in bosses
        .iter()
        .filter(|r| !matches!(r.outcome, Outcome::Complete))
    {
        let _ = writeln!(
            w,
            "  - {} {} (reached {}).",
            r.boss.name(),
            played_line_cell(r),
            r.reached
        );
    }
    // Every distinct untranslated set, with who reached it.
    let mut sets: Vec<(Vec<u32>, Vec<String>, String)> = Vec::new();
    let mut add = |addresses: &[u32], who: String, first: String| match sets
        .iter_mut()
        .find(|(a, _, _)| a == addresses)
    {
        Some((_, list, _)) => list.push(who),
        None => sets.push((addresses.to_vec(), vec![who], first)),
    };
    for r in bosses {
        for g in &r.gaps {
            let how = match g.kind {
                GapKind::PlayedLine => "on the played line".to_string(),
                GapKind::Discarded => format!(
                    "{} discarded search {}",
                    g.hits,
                    if g.hits == 1 { "try" } else { "tries" }
                ),
                GapKind::ContinueProbe => "continue probe".to_string(),
            };
            add(
                &g.gap.addresses,
                format!("{} ({how})", r.boss.name()),
                format!("{}: {}", r.boss.name(), g.gap.describe()),
            );
        }
    }
    for r in mirrors {
        for (g, n) in &r.gaps {
            add(
                &g.addresses,
                format!("{} mirror ({n} of {} samples)", r.fighter.name(), r.samples),
                format!("{} mirror: {}", r.fighter.name(), g.describe()),
            );
        }
    }
    if sets.is_empty() {
        let _ = writeln!(
            w,
            "- **Untranslated branches.** None reached anywhere in this run."
        );
    } else {
        let _ = writeln!(
            w,
            "- **Untranslated branches.** {} distinct {} reached:",
            sets.len(),
            if sets.len() == 1 { "set" } else { "sets" }
        );
        for (addresses, who, first) in &sets {
            let shown = addresses
                .iter()
                .map(|a| format!("`0x{a:05x}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let note = address_note(addresses)
                .map(|n| format!(" {n}."))
                .unwrap_or_default();
            let _ = writeln!(
                w,
                "  - {shown}: reached by {}. First seen in {first}.{note}",
                who.join(", ")
            );
        }
    }
    let clean_continue: Vec<&str> = bosses
        .iter()
        .filter(|r| r.continue_probe.starts_with("clean"))
        .map(|r| r.boss.name())
        .collect();
    let _ = writeln!(
        w,
        "- **Continue.** The continue probe ran clean for {} of {} bosses.",
        clean_continue.len(),
        bosses.len()
    );
    for r in bosses
        .iter()
        .filter(|r| !r.continue_probe.starts_with("clean"))
    {
        let _ = writeln!(w, "  - {}: {}.", r.boss.name(), r.continue_probe);
    }
    let clean_mirrors: Vec<&str> = mirrors
        .iter()
        .filter(|r| r.clean)
        .map(|r| r.fighter.name())
        .collect();
    let _ = writeln!(
        w,
        "- **Mirror matches.** A clean first round in every sample for {} of {} fighters: {}.",
        clean_mirrors.len(),
        mirrors.len(),
        clean_mirrors.join(", ")
    );
    for r in mirrors.iter().filter(|r| !r.clean) {
        let _ = writeln!(
            w,
            "  - {}: not clean: {}.",
            r.fighter.name(),
            mirror_result(r)
        );
    }
}

fn played_line_cell(run: &BossRun) -> String {
    match &run.outcome {
        Outcome::Complete => "complete to the attract".into(),
        Outcome::Gap => "ended on an untranslated branch".into(),
        Outcome::NotWon { stage, tries } => format!(
            "stopped at {stage}: the scripted player did not win the round in {tries} tries"
        ),
        Outcome::Stuck(why) => format!("stopped: {why}"),
    }
}

fn write_report(bosses: &[BossRun], mirrors: &[MirrorRun], seconds: f64) -> String {
    let mut s = String::new();
    let w = &mut s;
    let _ = writeln!(w, "# Boss ladder report");
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "Generated by `crates/sf-engine/tests/boss_ladder_report.rs` (issue #152, `FE3`) as the SF1+ project's starting map. It is a map, not a gate: the test never fails on what it finds. Regenerate it with:"
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "```");
    let _ = writeln!(w, "{COMMAND}");
    let _ = writeln!(w, "```");
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "Run time: {seconds:.1} s of wall time for this report in a release build ({} boss runs and {} mirror matches).",
        bosses.len(),
        mirrors.len()
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "## Findings");
    let _ = writeln!(w);
    write_findings(w, bosses, mirrors);
    let _ = writeln!(w);
    let _ = writeln!(w, "## How the stages were reached");
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "- **Machine.** `PowerOnMachine` from power-on with `MatchConfig::default()` (Definitive Overhaul: full cast, four-country select, every Tier 1 toggle on) and `roster.p1` set to the boss; for a mirror match `roster.p2` too. Factory DIP words `DSW1 = 0x{DSW1:04x}`, `DSW2 = 0x{DSW2:04x}`: Normal difficulty, round time 100, continuation to the 5th stage."
    );
    let _ = writeln!(
        w,
        "- **Cabinet and player inputs only.** One player: `coin1` held on frames 1451..=1462 and `start1` on 1521..=1532; in the four-country select (`0x3e32c`) a light-punch tap 20 frames in takes the default flag, Japan, so every boss meets the ladder Japan (Retsu, Geki), USA (Joe, Mike), China (Lee, Gen), England (Birdie, Eagle), Thailand (Adon, Sagat). Two players: the verified `sf-start-2p-01` sequence (`coin1` 1451, `coin2` 1481, `start2` 1561) and the same flag tap. The probe never writes work RAM or fight state; it reads RAM only to time its inputs, score its search and name the stages."
    );
    let _ = writeln!(
        w,
        "- **Winning the fights.** No boss has a verified winning script, so each round is searched from snapshots. From the round's first fight-loop frame (`0x3eafa`) the round is played in {SEGMENT}-frame segments: from each kept snapshot {TRIES_PER_SEGMENT} seeded scripted players (walks, crouches, jumps and button taps, attacking inside a seeded range) each play one segment; a try in which the CPU takes the round, the round is drawn or a branch is untranslated is discarded; of the rest, the one with the best health difference (player minus CPU) is kept as the next snapshot; a segment with nothing left to keep is backed out of. The search stops at the player's round win or after {TRIES_PER_ROUND} tries. A run that stops on a round no try won is marked *not won*: that is the scripted player's limit, not a translation gap, unless the round's tries were mostly ending on untranslated branches, which the fight table counts."
    );
    let _ = writeln!(
        w,
        "- **Outside the fights** the player holds nothing, except a heavy-punch tap every 16 frames in the bonus stages; name entry runs out on its timer."
    );
    let _ = writeln!(
        w,
        "- **Untranslated branches.** `unrecovered()` is read after every frame. On the played line the first non-empty set ends the run. Sets reached in discarded search tries or in the continue probe are listed with the stage and the number of tries that reached them, because a player playing that way would stop there. *First untranslated branch* is the first set the probe reached in ladder order, whichever kind. With each fight-time branch the report gives the fighters' move-table slot (`+0xa`) and move stream (`+0x2c`) on the frame before. Frames count from power-on along the line that reached the branch: for a discarded try, that try's line."
    );
    let _ = writeln!(
        w,
        "- **Continue probe.** At each boss's first round (Japan round 1 vs Retsu), from the round's snapshot, the player idles until the CPU takes the match; when the continue countdown opens (`0x3f162`) a coin pulse goes in and 1P Start 30 frames later, and the probe runs to the rematch's round-1 fight loop. Then the snapshot is restored and the ladder goes on."
    );
    let _ = writeln!(
        w,
        "- **Stage names** come from the game-flow task (slot 1), the screen task (slot 2), `STAGE_NO` (`0xffc003`), the opponent index (`0xffc004`), the round counter (`0xffc00a`) and the match winner (`0xffc009`). *Pre-fight* runs from the stage or opponent setup to round 1's fight loop; *round N* covers that round's intro, fight and result; *match end* is the match-won screen; the bonus stages follow Japan and China (power/timing) and USA and England (10 seconds); a *flight* is the travel map into the named country."
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "## Limits of this map");
    let _ = writeln!(w);
    for line in [
        "The probe sees only the branches the engine marks untranslated through `unrecovered()`. A path that is translated but wrong for a boss (art, text, sound, a move that does nothing) is not measured here.",
        "Each boss's played line is one line through the game, chosen by a search for wins. Other play reaches other branches; the discarded tries are listed for that reason, and they are a sample, not a sweep of every move.",
        "The stand-in rule (SP14, `#266`) substitutes Ryu on the ladder slot that matches the human player's own fighter, and the fair CPU plays every opponent on his 1987 player table.",
        "The continue probe covers one continue, at the first fight. Continues at later stages and the continuation limit are not probed.",
        "The bonus stages were played with periodic heavy-punch taps only, and name entry ran out on its timer with no initials entered.",
        "Factory DIP switches only: other difficulty and round-time settings are not covered.",
        "Mirror matches cover round 1 only, from one snapshot, in seeded samples.",
    ] {
        let _ = writeln!(w, "- {line}");
    }
    let _ = writeln!(w);
    let _ = writeln!(w, "## Summary");
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "| Boss | Stage reached | First untranslated branch | Played line | Continue probe | Search tries | Frames on the played line |"
    );
    let _ = writeln!(w, "| --- | --- | --- | --- | --- | --- | --- |");
    for run in bosses {
        let cont = run
            .continue_probe
            .split(':')
            .next()
            .unwrap_or("")
            .to_string();
        let _ = writeln!(
            w,
            "| {} | {} | {} | {} | {} | {} | {} |",
            run.boss.name(),
            run.reached,
            first_gap_cell(run),
            played_line_cell(run),
            cont,
            run.tries,
            run.frames
        );
    }
    let _ = writeln!(w);
    for run in bosses {
        let _ = writeln!(w, "## {}", run.boss.name());
        let _ = writeln!(w);
        let _ = writeln!(w, "- **Stage reached:** {}.", run.reached);
        let _ = writeln!(w, "- **Played line:** {}.", played_line_cell(run));
        let _ = writeln!(
            w,
            "- **Stages on the played line:** {}.",
            stages_passed(run)
        );
        let _ = writeln!(
            w,
            "- **First untranslated branch:** {}.",
            first_gap_cell(run)
        );
        if run.gaps.len() > 1 {
            let _ = writeln!(w, "- **Every untranslated set reached, in order:**");
            for g in &run.gaps {
                let _ = writeln!(w, "  - {}, {}.", g.gap.describe(), kind_note(g));
            }
        }
        let _ = writeln!(w, "- **Continue probe:** {}.", run.continue_probe);
        let _ = writeln!(w);
        let _ = writeln!(
            w,
            "| Country | Opponent (ladder) | CPU's bound table | Player's bound table | Rounds: tries (lost / drawn / untranslated), kept segments |"
        );
        let _ = writeln!(w, "| --- | --- | --- | --- | --- |");
        for f in &run.fights {
            let rounds = f
                .rounds
                .iter()
                .map(|r| {
                    format!(
                        "R{} {}: {} ({} / {} / {}), {}",
                        r.round,
                        if r.won { "won" } else { "not won" },
                        r.tries,
                        r.lost,
                        r.draws,
                        r.gaps,
                        r.segments
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            let _ = writeln!(
                w,
                "| {} | {} | {} | {} | {} |",
                f.country, f.expected, f.cpu_table, f.player_table, rounds
            );
        }
        let _ = writeln!(w);
        let _ = writeln!(w, "<details><summary>Stage log (frame, stage)</summary>");
        let _ = writeln!(w);
        let _ = writeln!(w, "```");
        for (frame, line) in &run.log {
            let _ = writeln!(w, "{frame:>6}  {line}");
        }
        let _ = writeln!(w, "```");
        let _ = writeln!(w);
        let _ = writeln!(w, "</details>");
        let _ = writeln!(w);
    }
    let _ = writeln!(w, "## Mirror matches");
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "Two-player versus from power-on with the same fighter on both sides (`roster.p1 = roster.p2`), idle to round 1's fight loop, then {MIRROR_SAMPLES} samples of round 1 from that frame's snapshot, both sides driven by seeded scripted players (no search). A sample is clean when round 1 is decided and round 2's fight loop is reached with `unrecovered()` empty. *Clean first round* is *yes* when both player structures were bound to the fighter's move table at round 1's fight loop and every sample was clean. Samples are evidence, not proof: a *yes* says no sample reached an untranslated branch; a *no* names the branch a player reached."
    );
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "| Fighter | Clean first round | P1 table | P2 table | Round-1 fight loop | Samples |"
    );
    let _ = writeln!(w, "| --- | --- | --- | --- | --- | --- |");
    for r in mirrors {
        let _ = writeln!(
            w,
            "| {} | {} | {} | {} | {} | {} |",
            r.fighter.name(),
            if r.clean { "yes" } else { "no" },
            r.p1_table,
            r.p2_table,
            r.fight_at
                .map(|f| format!("frame {f}"))
                .unwrap_or_else(|| "not reached".into()),
            mirror_result(r)
        );
    }
    s
}

#[test]
#[ignore = "writes docs/reference/boss-ladder-report.md; run in release with --ignored"]
fn boss_ladder_report() {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: the local ROM set ({ROMS}) or art pack ({PACK}) is not present");
        return;
    }
    let assets = match load_assets(ROMS, MANIFEST) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("skipping: the local ROM set ({ROMS}) did not load: {e:?}");
            return;
        }
    };
    let pack = ArtPack::open(Path::new(PACK)).expect("open local art pack");
    let art = load_art_from(&assets, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) })
        .expect("load local art pack");
    let bank = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
    let rom = assets.maincpu;
    let verbose = std::env::var_os("BOSS_LADDER_VERBOSE").is_some();
    let t0 = Instant::now();
    let mut bosses = Vec::new();
    for boss in TournamentFighter::ALL {
        let started = Instant::now();
        let run = run_boss(&rom, &bank, boss);
        if verbose {
            println!("{}:", boss.name());
            for (frame, line) in &run.log {
                println!("  {frame:>6}  {line}");
            }
        }
        println!(
            "{:>7}: {} | {} | {} tries, {:.1} s",
            boss.name(),
            run.reached,
            first_gap_cell(&run),
            run.tries,
            started.elapsed().as_secs_f64()
        );
        bosses.push(run);
    }
    let mut mirrors = Vec::new();
    for fighter in TournamentFighter::ALL {
        let run = run_mirror(&rom, &bank, fighter);
        println!(
            "{:>7} mirror: clean {} | {}",
            fighter.name(),
            run.clean,
            mirror_result(&run)
        );
        mirrors.push(run);
    }
    let report = write_report(&bosses, &mirrors, t0.elapsed().as_secs_f64());
    std::fs::write(REPORT, &report).expect("write the ladder report");
    println!("wrote {REPORT}");
    assert!(Path::new(REPORT).is_file());
}
