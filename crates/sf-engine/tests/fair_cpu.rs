//! Tickets #264 (`SP21`) and #265 (`SP22`): the fair CPU, Training's CPU
//! opponent under the overhaul presets, and its tactics under the modern
//! rules (`true_combos`, `cancels`, `throws`).
//!
//! At the headless session's seam, driven by inputs: Training against the
//! CPU under an overhaul preset is a two-player fight whose Player 2 pad the
//! fair CPU plays, through the 1987 player code. What is checked is read
//! back through the fight reader (each fighter's move, health, attack box
//! and the hits landed), the presented frames and the session's log, and
//! the stick and buttons the machine took on Player 2's side. Local ROMs
//! only (and the local art pack for Retsu); each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::collections::BTreeMap;
use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fair_cpu::perceive;
use sf_engine::fight_reader::FighterRead;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::FightDips;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, Transition};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::run::PowerOnMachine;
use sf_video::art::{load_art_from, Art, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::Side;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
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

/// The game flow's fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
const CLASSIC: RulesetPreset = RulesetPreset::ClassicCompetitive;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

/// The art for Retsu's, Geki's and Joe's 1987 player tables, when the pack
/// is here.
fn art(a: &Assets) -> Option<Art> {
    if !Path::new(PACK).is_dir() {
        eprintln!("skipping: local art pack not present");
        return None;
    }
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(a, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).expect("the art loads");
    assert!(
        art.catalogue("retsu").is_some()
            && art.catalogue("geki").is_some()
            && art.catalogue("joe").is_some()
            && art.catalogue("mike").is_some()
            && art.catalogue("lee").is_some(),
        "Retsu's, Geki's, Joe's, Mike's and Lee's poses load: {:?}",
        art.kept
    );
    Some(art)
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

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn stick(d: usize) -> FrameInputs {
    with(|i| i.p1.directions[d] = true)
}

/// A session under `preset` at `difficulty`, with Retsu's table when `art`
/// is given.
fn session(a: Assets, art: Option<&Art>, preset: RulesetPreset, difficulty: u8) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    if let Some(art) = art {
        s = s.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
    }
    s.set_fight_dips(FightDips { difficulty, round_time: 100 });
    s
}

/// Training against the CPU: P1 `p1`, the CPU `cpu`, from the Main Menu
/// to where the fighters take control.
fn cpu_fight(mut s: GameSession, p1: TournamentFighter, cpu: TournamentFighter) -> GameSession {
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, stick(DOWN));
    assert_eq!(tap(&mut s, confirm()).screen, "training");
    assert_eq!(tap(&mut s, confirm()).screen, "select"); // CPU
    let at = |f: TournamentFighter| TournamentFighter::ALL.iter().position(|&g| g == f).unwrap();
    if at(p1) >= 6 {
        tap(&mut s, stick(DOWN));
    }
    for _ in 0..(at(p1) % 6) {
        tap(&mut s, stick(RIGHT));
    }
    assert_eq!(select::confirm_fighter(&mut s, confirm(), tap).transition, Some(Transition::Picked(p1)));
    // The opponent's cursor starts on Ken (row 0, col 1), the first fighter
    // after Ryu the fair CPU can play.
    let to = at(cpu);
    if to >= 6 {
        tap(&mut s, stick(LEFT));
        tap(&mut s, stick(DOWN));
        for _ in 0..(to % 6) {
            tap(&mut s, stick(RIGHT));
        }
    } else if to == 0 {
        tap(&mut s, stick(LEFT));
    } else {
        for _ in 1..to {
            tap(&mut s, stick(RIGHT));
        }
    }
    let out = select::confirm_fighter(&mut s, confirm(), GameSession::step);
    assert_eq!(out.transition, Some(Transition::Fighters(p1, cpu)));
    s.step(idle());
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    for _ in 0..240 {
        s.step(idle());
    }
    s
}

fn fighters(s: &GameSession) -> [FighterRead; 2] {
    s.fight().unwrap().fight_read().fighters
}

/// Player 2's stick and buttons as the machine took them on the last frame
/// (`IN1` bits 8 to 15, `IN0`'s HP and HK bits, active low), as bits: right,
/// left, down, up, then LP, MP, HP, LK, MK, HK.
fn p2_pad(m: &PowerOnMachine) -> u16 {
    let io = m.save_snapshot().io;
    let held = |word: u16, bit: u16| word & bit == 0;
    let bits = [
        held(io.in1, 1 << 8),
        held(io.in1, 1 << 9),
        held(io.in1, 1 << 10),
        held(io.in1, 1 << 11),
        held(io.in1, 1 << 12),
        held(io.in1, 1 << 13),
        held(io.in0, 0x0400),
        held(io.in1, 1 << 14),
        held(io.in1, 1 << 15),
        held(io.in0, 0x0100),
    ];
    bits.iter().enumerate().fold(0, |pad, (i, &b)| pad | (u16::from(b) << i))
}

/// A sparring partner for the CPU: P1 walks in and out, pokes standing and
/// crouching, jumps in and throws Hadokens, from a fixed little generator,
/// so the same frames always give the same inputs.
struct Sparring {
    seed: u32,
    plan: Vec<FrameInputs>,
    /// Only jump-ins, from the middle distance.
    jumps_only: bool,
    /// A finite jump-in opening before varied sparring; zero keeps the mode.
    jump_frames: u32,
}

impl Sparring {
    fn new(seed: u32) -> Self {
        Sparring { seed, plan: Vec::new(), jumps_only: false, jump_frames: 0 }
    }

    fn jumps(seed: u32) -> Self {
        Sparring { seed, plan: Vec::new(), jumps_only: true, jump_frames: 0 }
    }

    fn jump_then_spar(seed: u32, frames: u32) -> Self {
        Sparring { seed, plan: Vec::new(), jumps_only: true, jump_frames: frames }
    }

    fn next_u(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.seed >> 16
    }

    /// The next frame's inputs; `p1` and `p2` are where the fighters stand.
    fn step(&mut self, p1_x: i32, p2_x: i32) -> FrameInputs {
        if self.plan.is_empty() {
            let (fwd, back) = if p2_x >= p1_x { (RIGHT, LEFT) } else { (LEFT, RIGHT) };
            let dist = (p2_x - p1_x).abs();
            let pick = if !self.jumps_only {
                self.next_u() % 10
            } else if dist > 150 {
                0
            } else if dist < 90 {
                1
            } else {
                6
            };
            let mut plan = Vec::new();
            let hold = |dirs: &[usize], buttons: &[usize]| {
                with(|i| {
                    dirs.iter().for_each(|&d| i.p1.directions[d] = true);
                    buttons.iter().for_each(|&b| i.p1.buttons[b] = true);
                })
            };
            match pick {
                0 | 1 if dist > 70 && !self.jumps_only => plan.extend(std::iter::repeat_n(hold(&[fwd], &[]), 20)),
                0 => plan.extend(std::iter::repeat_n(hold(&[fwd], &[]), 8)),
                1 => plan.extend(std::iter::repeat_n(hold(&[back], &[]), 14)),
                2 | 3 => {
                    let b = (self.next_u() % 6) as usize;
                    plan.extend([hold(&[], &[b]), hold(&[], &[b]), hold(&[], &[b])]);
                    plan.extend(std::iter::repeat_n(idle(), 24));
                }
                4 | 5 => {
                    let b = (self.next_u() % 6) as usize;
                    plan.extend(std::iter::repeat_n(hold(&[DOWN], &[]), 2));
                    plan.extend(std::iter::repeat_n(hold(&[DOWN], &[b]), 3));
                    plan.extend(std::iter::repeat_n(hold(&[DOWN], &[]), 22));
                }
                6 => {
                    // A jump-in with a heavy kick on the way down.
                    plan.extend(std::iter::repeat_n(hold(&[UP, fwd], &[]), 3));
                    plan.extend(std::iter::repeat_n(idle(), 18));
                    plan.extend(std::iter::repeat_n(hold(&[], &[5]), 3));
                    plan.extend(std::iter::repeat_n(idle(), 26));
                }
                7 => {
                    // A Hadoken: down, down-forward, forward and punch.
                    plan.extend(std::iter::repeat_n(hold(&[DOWN], &[]), 3));
                    plan.extend(std::iter::repeat_n(hold(&[DOWN, fwd], &[]), 3));
                    plan.extend([hold(&[fwd], &[]), hold(&[fwd], &[1]), hold(&[fwd], &[1]), hold(&[fwd], &[1])]);
                    plan.extend(std::iter::repeat_n(idle(), 30));
                }
                _ => plan.extend(std::iter::repeat_n(idle(), 10 + (self.next_u() % 20) as usize)),
            }
            plan.reverse();
            self.plan = plan;
        }
        let input = self.plan.pop().unwrap_or_default();
        if self.jump_frames > 0 {
            self.jump_frames -= 1;
            if self.jump_frames == 0 {
                self.jumps_only = false;
                self.plan.clear();
            }
        }
        input
    }
}

/// What a stretch of fight showed of the CPU.
#[derive(Debug, Default)]
struct Watch {
    /// Frames of each move P2 was drawn in.
    moves: BTreeMap<u8, u32>,
    /// P1's attacks that met P2: landed clean on a P2 who was free to
    /// guard, landed while P2 was attacking or in the air (a trade or a
    /// counter-hit, which no guard could stop), and stopped by P2's
    /// standing guard (move 4) or crouching guard (move 8).
    landed: u32,
    traded: u32,
    blocked_high: u32,
    blocked_low: u32,
    /// Hits P2 landed on P1.
    cpu_hits: u32,
    /// Hits P2 landed on P1 while P1 was in hitstun (true combos, SP17/SP22).
    cpu_combos: u32,
    /// Highest combo count drawn on P2's HUD slot.
    cpu_max_combo: u16,
    /// Normal-to-special cancels P2 executed (`toggle_state.cancel.count[1]`, SP18/SP22).
    cpu_cancels: u8,
    /// Frames on which P2's pad held something.
    pressed: u32,
}

impl Watch {
    fn saw(&self, f: impl Fn(u8) -> bool) -> u32 {
        self.moves.iter().filter(|(&m, _)| f(m)).map(|(_, &n)| n).sum()
    }

    fn blocked(&self) -> u32 {
        self.blocked_high + self.blocked_low
    }
}

/// Where a fighter stands: his entity's grid x (`+6`).
fn grid_x(m: &PowerOnMachine, ps: u32) -> i32 {
    let (o, w) = (&m.state.objectram, m.work_ram());
    let at = (ps + 0x10 - 0xff8000) as usize;
    let e = usize::from(u16::from_be_bytes([w[at], w[at + 1]]));
    i32::from(u16::from_be_bytes([o[e + 6], o[e + 7]]))
}

/// Play `frames` frames against `sparring` and watch the CPU. A hit P1
/// lands is read from the fight reader; a block from Training's contact
/// clock, which every hit and every block on P2 starts over
/// (`TrainingState::since_contact`), on a frame with no clean hit (chip on
/// the guard counts as blocked), by the guard P2 is drawn in. The digest
/// holds each presented frame and the pad P2's side took.
fn spar_with(s: &mut GameSession, mut sparring: Sparring, frames: u32) -> (Watch, Vec<u64>) {
    let mut watch = Watch::default();
    let mut digest = Vec::new();
    let mut contact = s.fight_machine().unwrap().training().since_contact[1];
    let mut was = s.fight().unwrap().fight_read().fighters[1];
    for _ in 0..frames {
        let m = s.fight_machine().unwrap();
        let input = sparring.step(grid_x(m, P1_STRUCT), grid_x(m, P2_STRUCT));
        let out = s.step(input);
        assert_eq!(out.screen, "fight");
        let read = *s.fight().unwrap().fight_read();
        let p2 = read.fighters[1];
        *watch.moves.entry(p2.move_id).or_default() += 1;
        let m = s.fight_machine().unwrap();
        let now = m.training().since_contact[1];
        let guarded = match read.landed[0] {
            Some(hit) if !hit.chip => {
                if was.attacking() || jumping(was.move_id) {
                    watch.traded += 1;
                } else {
                    watch.landed += 1;
                }
                false
            }
            Some(_) => true,
            None => now < contact,
        };
        if guarded {
            if p2.move_id == 8 {
                watch.blocked_low += 1;
            } else {
                watch.blocked_high += 1;
            }
        }
        contact = now;
        was = p2;
        if let Some(hit) = read.landed[1] {
            watch.cpu_hits += 1;
            if hit.into_hitstun {
                watch.cpu_combos += 1;
            }
        }
        if let Some(c) = s.fight().unwrap().callouts().frame().side(Side::P2).combo {
            watch.cpu_max_combo = watch.cpu_max_combo.max(c.hits);
        }
        watch.cpu_cancels = m.toggle_state().cancel.count[1];
        let pad = p2_pad(m);
        watch.pressed += u32::from(pad != 0);
        let pixels = out.frame.argb.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &p| (h ^ u64::from(p)).wrapping_mul(0x100_0000_01b3));
        digest.push(pixels ^ u64::from(pad) << 48);
    }
    (watch, digest)
}

fn spar(s: &mut GameSession, seed: u32, frames: u32) -> (Watch, Vec<u64>) {
    spar_with(s, Sparring::new(seed), frames)
}

/// Ryu, Ken and Retsu's moves, as their 1987 player tables number them.
fn jumping(m: u8) -> bool {
    matches!(m, 9..=0xb)
}
fn normal(m: u8) -> bool {
    (0x0d..=0x2f).contains(&m)
        && !matches!(
            m,
            0x19..=0x1b | sf_engine::combo::throw::THROW_MOVE | sf_engine::combo::throw::GET_UP_MOVE
        )
}

/// The game mode byte (`0xffc002`): 1 two players, 2 one against the CPU.
fn game_mode(m: &PowerOnMachine) -> u8 {
    m.work_ram()[(sf_engine::fighter::GAME_MODE - 0xff8000) as usize]
}

#[test]
fn the_cpu_walks_jumps_blocks_high_and_low_and_uses_normals_and_specials() {
    let Some(a) = assets() else { return };
    let Some(art) = art(&a) else { return };
    // Ryu's and Ken's specials, Retsu's Kiai Shot in the Hadoken's move,
    // Geki's Shuriken and Teleport Strike in the Hadoken's and the
    // Tatsumaki's (SP4), and Joe's Dynamite Straight and Rolling Sobat in
    // the Shoryuken's and the Tatsumaki's (SP5).
    let cases = [
        (TournamentFighter::Ken, RulesetPreset::DefinitiveOverhaul, &[0x19u8, 0x1a, 0x1b][..]),
        (TournamentFighter::Ryu, CLASSIC, &[0x19, 0x1a, 0x1b][..]),
        (TournamentFighter::Retsu, CLASSIC, &[0x1a][..]),
        (TournamentFighter::Geki, CLASSIC, &[0x1a, 0x1b][..]),
        (TournamentFighter::Joe, CLASSIC, &[0x19, 0x1b][..]),
        (TournamentFighter::Mike, CLASSIC, &[0x19, 0x1a][..]),
        (TournamentFighter::Lee, CLASSIC, &[0x1a, 0x1b][..]),
        (TournamentFighter::Gen, CLASSIC, &[0x19, 0x1a][..]),
        (TournamentFighter::Birdie, CLASSIC, &[0x1a, 0x1b][..]),
        (TournamentFighter::Eagle, CLASSIC, &[0x1a, 0x1b][..]),
        (TournamentFighter::Adon, CLASSIC, &[0x19, 0x1b][..]),
        (TournamentFighter::Sagat, CLASSIC, &[0x19, 0x1a, 0x1b][..]),
    ];
    for (cpu, preset, specials) in cases {
        let a = assets().unwrap();
        let mut s = cpu_fight(session(a, Some(&art), preset, 2), TournamentFighter::Ryu, cpu);
        let m = s.fight_machine().unwrap();
        // A two-player Training fight: the CPU's fighter is bound to P2 and
        // runs on the 1987 player code, his pad the fair CPU's.
        assert_eq!(m.match_config().roster.mode, PlayMode::Training);
        assert!(m.match_config().p2_fair_cpu);
        assert_eq!(game_mode(m), 1, "two players");
        let at = (P2_STRUCT + 0x12 - 0xff8000) as usize;
        let table = u32::from_be_bytes(m.work_ram()[at..at + 4].try_into().unwrap());
        assert_eq!(TournamentFighter::from_move_table(table), Some(cpu), "P2's table is {cpu:?}'s");
        // Give modern Ken's anti-air deliberate opportunities before the
        // varied fight, within the same 8,000 frames and fixed sparring seed.
        let sparring = if cpu == TournamentFighter::Ken && preset == RulesetPreset::DefinitiveOverhaul {
            Sparring::jump_then_spar(11, 1_500)
        } else {
            Sparring::new(11)
        };
        let (w, _) = spar_with(&mut s, sparring, 8_000);
        println!("{cpu:?} under {preset:?}: {w:?}");
        assert!(w.saw(|m| m == 2) > 0 && w.saw(|m| m == 3) > 0, "{cpu:?} walks forward and back");
        assert!(w.saw(jumping) > 0, "{cpu:?} jumps");
        assert!(w.saw(|m| matches!(m, 5..=7)) > 0, "{cpu:?} crouches");
        assert!(w.blocked_high > 0 && w.blocked_low > 0, "{cpu:?} blocks high and low: {w:?}");
        assert!(w.saw(normal) > 0, "{cpu:?} uses normals");
        for &special in specials {
            assert!(w.saw(|m| m == special) > 0, "{cpu:?} uses special move {special:#x}");
        }
        assert!(w.cpu_hits > 0, "{cpu:?} lands hits");
        assert!(s.fight_machine().unwrap().unrecovered().is_empty());
        assert!(s.take_log().iter().all(|l| !l.contains("untranslated")));
    }
}

/// P1's actions for the reaction trials, by the way P1 faces: a standing
/// heavy kick, a crouching medium kick, a forward jump, a Hadoken.
fn action(kind: usize, fwd: usize) -> Vec<FrameInputs> {
    let hold = |dirs: &[usize], b: Option<usize>| {
        with(|i| {
            dirs.iter().for_each(|&d| i.p1.directions[d] = true);
            if let Some(b) = b {
                i.p1.buttons[b] = true;
            }
        })
    };
    match kind % 4 {
        0 => vec![hold(&[], Some(5)); 3],
        1 => vec![hold(&[DOWN], Some(4)); 3],
        2 => vec![hold(&[UP, fwd], None); 3],
        _ => {
            let mut v = vec![hold(&[DOWN], None); 3];
            v.extend([hold(&[DOWN, fwd], None); 3]);
            v.extend([hold(&[fwd], None), hold(&[fwd], Some(0)), hold(&[fwd], Some(0))]);
            v
        }
    }
}

/// Present an actionable player within threat range. Release earlier pad
/// inputs before the trial so a held button cannot make the action a no-op.
fn prepare_reaction_trial(s: &mut GameSession) {
    let ready = |s: &GameSession| {
        let m = s.fight_machine().unwrap();
        let seen = perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2);
        let machine = sf_engine::moves::MachineRead {
            rom: m.program_rom(), work: m.work_ram(), obj: &m.state.objectram, bank: m.data_bank(),
        };
        let distance = (seen.me.x - seen.foe.x).abs();
        distance <= 130
            && seen.foe.free()
            && sf_engine::moves::input_gate_free(machine, P1_STRUCT, &m.match_config().toggles)
    };
    for _ in 0..240 {
        let m = s.fight_machine().unwrap();
        let seen = perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2);
        let distance = (seen.me.x - seen.foe.x).abs();
        let toward = if seen.me.x >= seen.foe.x { RIGHT } else { LEFT };
        let input = if distance > 130 { stick(toward) }
            else { idle() };
        s.step(input);
        if ready(s) {
            s.step(idle());
            s.step(idle());
            if ready(s) { return; }
        }
    }
    let m = s.fight_machine().unwrap();
    let seen = perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2);
    let machine = sf_engine::moves::MachineRead {
        rom: m.program_rom(), work: m.work_ram(), obj: &m.state.objectram, bank: m.data_bank(),
    };
    let gate = sf_engine::moves::input_gate_free(machine, P1_STRUCT, &m.match_config().toggles);
    panic!("no actionable in-range reaction trial: gate={gate}, view={seen:?}");
}

#[derive(Default)]
struct ReactionSamples {
    /// First pad changes in every trial, including interrupted actions.
    pad_changes: Vec<u32>,
    /// Replies that followed an actual visible attack, jump or projectile.
    visible_replies: Vec<u32>,
    visible_actions: u32,
}

/// Over `trials` moments of a fight, measure the frames from P1's first
/// input to the first difference from the CPU pad in an idle-input copy.
fn answer_latencies(preset: RulesetPreset, difficulty: u8, trials: u32) -> ReactionSamples {
    let a = assets().unwrap();
    let mut s = cpu_fight(session(a, None, preset, difficulty), TournamentFighter::Ryu, TournamentFighter::Ken);
    let mut sparring = Sparring::new(23);
    let mut samples = ReactionSamples::default();
    for trial in 0..trials {
        for _ in 0..40 + trial % 23 {
            let m = s.fight_machine().unwrap();
            let input = sparring.step(grid_x(m, P1_STRUCT), grid_x(m, P2_STRUCT));
            s.step(input);
        }
        prepare_reaction_trial(&mut s);
        // From here on, one copy of the fight sees P1 act and the other
        // sees him stand still; only what the CPU sees differs.
        let m = s.fight_machine().unwrap();
        let fwd = if grid_x(m, P2_STRUCT) >= grid_x(m, P1_STRUCT) { RIGHT } else { LEFT };
        let script = action(trial as usize, fwd);
        let (mut acted, mut still) = (s.clone(), s.clone());
        let mut action_seen = false;
        for f in 0..150 {
            acted.step(script.get(f).copied().unwrap_or_default());
            still.step(idle());
            let view = |s: &GameSession| {
                let m = s.fight_machine().unwrap();
                perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2)
            };
            let (acted_view, still_view) = (view(&acted), view(&still));
            action_seen |= (acted_view.foe != still_view.foe
                && (acted_view.foe.attacking() || acted_view.foe.jumping()))
                || (acted_view.shot && !still_view.shot);
            if p2_pad(acted.fight_machine().unwrap()) != p2_pad(still.fight_machine().unwrap()) {
                samples.pad_changes.push(f as u32);
                if action_seen { samples.visible_replies.push(f as u32); }
                break;
            }
        }
        samples.visible_actions += u32::from(action_seen);
        // The fight goes on from the copy where P1 acted.
        s = acted;
    }
    samples
}

fn median(v: &[u32]) -> u32 {
    let mut v = v.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

#[test]
fn the_cpu_never_answers_sooner_than_its_difficultys_reaction_delay() {
    if assets().is_none() {
        return;
    }
    // Easy 27, Normal 19, Hard 15, Very Hard 12 frames (the research's
    // proposed bands, `docs/research/sf1plus-reference-numbers.md` 5.4).
    let levels = [(1u8, 27u32), (2, 19), (3, 15), (4, 12)];
    let measured: Vec<_> = levels.iter().map(|&(difficulty, _)| answer_latencies(CLASSIC, difficulty, 40)).collect();
    let (mut medians, mut fastest) = (Vec::new(), Vec::new());
    for (&(difficulty, delay), samples) in levels.iter().zip(&measured) {
        let latencies = &samples.visible_replies;
        // Easy lets most threats go (it answers 35 in 100), so a change can
        // come late, or not within the window at all. At least a quarter of
        // trials must show a real action and a pad reply to it.
        assert!(latencies.len() >= 10, "difficulty {difficulty}: the CPU answers ({latencies:?})");
        let (min, med) = (*latencies.iter().min().unwrap(), median(latencies));
        let first_pad = *samples.pad_changes.iter().min().unwrap();
        println!("difficulty {difficulty}: {} of 40 visible actions, {} visible replies among {} pad changes; fastest {min}, all-pad minimum {first_pad}, median {med}: {latencies:?}",
            samples.visible_actions, latencies.len(), samples.pad_changes.len());
        // An interrupted action cannot establish a prompt reply, but any
        // pad change still has to respect the CPU's reaction floor.
        assert!(first_pad >= delay, "difficulty {difficulty}: a pad change at {first_pad}, sooner than {delay}");
        // And it does answer as soon as it may: the delay is what holds it.
        assert!(min <= delay + 3, "difficulty {difficulty}: fastest answer {min} frames, delay {delay}");
        medians.push(med);
        fastest.push(min);
    }
    // The reaction delay, measured as each level's fastest answer, is
    // ordered from Easy to Very Hard. The median first change is not a
    // reaction measure (it includes changes that come only when a plan the
    // CPU was in ends, or from later choices), so only its ends are compared.
    assert!(fastest.windows(2).all(|w| w[0] > w[1]), "the fastest answers: {fastest:?}");
    assert!(medians[0] > medians[3], "Easy answers later than Very Hard: {medians:?}");
    // Under the modern rules (`DefinitiveOverhaul`, SP22), the reaction-delay
    // floor holds just the same.
    for (difficulty, delay) in [(2u8, 19u32), (4, 12)] {
        let samples = answer_latencies(RulesetPreset::DefinitiveOverhaul, difficulty, 24);
        let modern = &samples.visible_replies;
        assert!(modern.len() >= 6, "modern difficulty {difficulty}: the CPU answers ({modern:?})");
        let min = *modern.iter().min().unwrap();
        let first_pad = *samples.pad_changes.iter().min().unwrap();
        println!("modern difficulty {difficulty}: {} of 24 visible actions, {} visible replies among {} pad changes; fastest {min}, all-pad minimum {first_pad}: {modern:?}",
            samples.visible_actions, modern.len(), samples.pad_changes.len());
        assert!(first_pad >= delay, "modern difficulty {difficulty}: a pad change at {first_pad} < delay {delay}");
        assert!(min <= delay + 3, "modern difficulty {difficulty}: fastest {min}, delay {delay}");
    }
}

#[test]
fn a_jump_with_a_button_held_for_later_looks_like_an_empty_jump_until_the_attack_is_drawn() {
    let Some(a) = assets() else { return };
    // Custom with ANYTIME JUMP ATTACKS off: a button pressed early in a jump
    // is kept in P1's state byte (+9 = 0xc1) until the jump attack can
    // start, tens of frames later, with nothing new drawn meanwhile.
    let mut config = MatchConfig::from_preset(RulesetPreset::Custom);
    config.toggles.anytime_jump_attacks = false;
    let mut s = GameSession::new(a, config);
    s.set_fight_dips(FightDips { difficulty: 4, round_time: 100 });
    let mut s = cpu_fight(s, TournamentFighter::Ryu, TournamentFighter::Ken);
    while fighters(&s)[0].move_id != 1 || fighters(&s)[0].reacting() {
        s.step(idle());
    }
    let delay = 12;
    let m = s.fight_machine().unwrap();
    let fwd = if grid_x(m, P2_STRUCT) >= grid_x(m, P1_STRUCT) { RIGHT } else { LEFT };
    let (mut empty, mut held) = (s.clone(), s.clone());
    let (mut state_differs, mut drawn, mut pads_differ) = (None, None, None);
    for f in 0..120u32 {
        let jump = with(|i| {
            i.p1.directions[UP] = f < 3;
            i.p1.directions[fwd] = f < 3;
        });
        let button = with(|i| {
            i.p1.directions[UP] = f < 3;
            i.p1.directions[fwd] = f < 3;
            i.p1.buttons[5] = (6..9).contains(&f);
        });
        empty.step(jump);
        held.step(button);
        // What the CPU makes of P1 in each copy: the same, until the attack
        // is drawn.
        let seen = |s: &GameSession| {
            let m = s.fight_machine().unwrap();
            let foe = perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2).foe;
            (foe.attacking(), foe.jumping(), foe.low_attack(), foe.move_id)
        };
        let (a, b) = (fighters(&empty)[0], fighters(&held)[0]);
        if drawn.is_none() && a.move_id == b.move_id {
            assert_eq!(seen(&empty), seen(&held), "frame {f}: the CPU saw the held button");
        }
        if a.state != b.state && state_differs.is_none() {
            state_differs = Some(f);
        }
        if a.move_id != b.move_id && drawn.is_none() {
            drawn = Some(f);
        }
        if p2_pad(empty.fight_machine().unwrap()) != p2_pad(held.fight_machine().unwrap()) && pads_differ.is_none() {
            pads_differ = Some(f);
        }
    }
    println!("state byte differs from {state_differs:?}, the attack drawn from {drawn:?}, the CPU's pad differs from {pads_differ:?}");
    let (state_differs, drawn) = (state_differs.expect("the button was kept"), drawn.expect("the jump attack came out"));
    assert!(drawn > state_differs + delay, "the button was kept longer than the CPU's delay before it was drawn");
    if let Some(p) = pads_differ {
        assert!(p >= drawn + delay, "the CPU answered on {p}, before it could have seen the attack drawn on {drawn}");
    }
}

#[test]
fn two_fights_from_the_same_inputs_are_the_same_fight_cpu_included() {
    let Some(a) = assets() else { return };
    let Some(art) = art(&a) else { return };
    let play = |seed: u32| {
        let a = assets().unwrap();
        let mut s = cpu_fight(session(a, Some(&art), CLASSIC, 3), TournamentFighter::Ryu, TournamentFighter::Retsu);
        let (w, digest) = spar(&mut s, seed, 3_000);
        (w, digest, s.fight_machine().unwrap().save_snapshot())
    };
    let (w, first, end) = play(5);
    let (_, second, end_again) = play(5);
    assert!(w.pressed > 1_000, "the CPU played: {w:?}");
    assert_eq!(first, second, "every presented frame and P2's pad");
    assert!(end == end_again, "the machines, the CPU's state included");
    assert!(end.fair_cpu.is_some(), "the CPU's state is in the snapshot");
    // The digest tells fights apart: other inputs, another fight.
    let (_, other, _) = play(6);
    assert_ne!(first, other);
}

#[test]
fn a_snapshot_restored_mid_fight_plays_on_as_the_fight_did() {
    let Some(a) = assets() else { return };
    let mut s = cpu_fight(session(a, None, CLASSIC, 4), TournamentFighter::Ryu, TournamentFighter::Ken);
    spar(&mut s, 2, 600);
    // Rollback's own path: save the machine, play on, restore it into a
    // machine that has played something else meanwhile, and play the same
    // P1 inputs again.
    let mut m = s.fight_machine().unwrap().clone();
    let snap = m.save_snapshot();
    let inputs: Vec<([bool; 4], [bool; 6])> = (0..900u32)
        .map(|f| {
            let r = f.wrapping_mul(2_654_435_761) >> 7;
            let dirs = [r & 1 != 0 && f % 50 < 30, r & 2 != 0 && f % 50 >= 30, f % 90 > 70, f % 120 == 3];
            (dirs, std::array::from_fn(|b| f % 37 == 5 * b as u32))
        })
        .collect();
    let run = |m: &mut PowerOnMachine| {
        let mut pads = Vec::new();
        for &([r, l, d, u], buttons) in &inputs {
            m.set_p1_direction(r, l, d, u);
            m.set_p1_buttons(buttons);
            m.step();
            pads.push(p2_pad(m));
        }
        pads
    };
    let straight = run(&mut m);
    let end = m.save_snapshot();
    let mut other = s.fight_machine().unwrap().clone();
    for _ in 0..300 {
        other.set_p1_direction(true, false, false, false);
        other.step();
    }
    other.restore_snapshot(&snap);
    let replayed = run(&mut other);
    assert!(straight.iter().filter(|&&p| p != 0).count() > 200, "the CPU played");
    assert_eq!(straight, replayed);
    assert!(end == other.save_snapshot());
}

/// What one level did against many sparring partners: of P1's attacks that
/// met the CPU while it was free to guard, how many it guarded and how many
/// landed; its own hits; and, against partners who only jump in, how many
/// of their jump-ins landed.
#[derive(Debug, Default, Clone, Copy)]
struct Level {
    blocked: u32,
    landed: u32,
    hits: u32,
    jump_ins_landed: u32,
}

impl Level {
    fn guard_rate(&self) -> f64 {
        f64::from(self.blocked) / f64::from((self.blocked + self.landed).max(1))
    }
}

/// Partners for the comparison: enough that one fight's luck does not decide it.
const PARTNERS: std::ops::Range<u32> = 1..13;
const JUMP_PARTNERS: std::ops::Range<u32> = 1..7;

fn measure_level(difficulty: u8) -> Level {
    let fight = || {
        let a = assets().unwrap();
        cpu_fight(session(a, None, CLASSIC, difficulty), TournamentFighter::Ryu, TournamentFighter::Ken)
    };
    // Each partner from the same fight start.
    let start = fight();
    let mut level = Level::default();
    for seed in PARTNERS {
        let w = spar_with(&mut start.clone(), Sparring::new(seed), 3_000).0;
        level.blocked += w.blocked();
        level.landed += w.landed;
        level.hits += w.cpu_hits;
    }
    for seed in JUMP_PARTNERS {
        level.jump_ins_landed += spar_with(&mut start.clone(), Sparring::jumps(seed), 3_000).0.landed;
    }
    level
}

#[test]
fn the_four_levels_are_ordered_on_guarding_and_stopping_jump_ins() {
    if assets().is_none() {
        return;
    }
    let levels: Vec<Level> = std::thread::scope(|scope| {
        let runs: Vec<_> = (1..=4u8).map(|d| scope.spawn(move || measure_level(d))).collect();
        runs.into_iter().map(|r| r.join().unwrap()).collect()
    });
    for (d, l) in levels.iter().enumerate() {
        println!("difficulty {}: {l:?}, guard rate {:.3}", d + 1, l.guard_rate());
    }
    for pair in levels.windows(2) {
        let (lower, higher) = (pair[0], pair[1]);
        assert!(higher.guard_rate() > lower.guard_rate(), "a harder level guards more of what meets it: {levels:?}");
        assert!(higher.jump_ins_landed <= lower.jump_ins_landed, "a harder level lets no more jump-ins land: {levels:?}");
    }
    // Easy against Very Hard, by a wide margin.
    let (easy, hard) = (levels[0], levels[3]);
    assert!(hard.guard_rate() > easy.guard_rate() + 0.2);
    assert!(easy.jump_ins_landed >= 3 * hard.jump_ins_landed.max(1));
    assert!(hard.hits > easy.hits, "and Very Hard hits back more");
}

#[test]
fn under_an_overhaul_preset_the_cpu_can_be_any_fighter_the_1987_player_code_runs() {
    let Some(a) = assets() else { return };
    // Without the art pack: Ryu and Ken only. The opponent's cursor starts
    // on Ken; Ryu is to his left and nothing to his right or below is live.
    let mut s = session(a, None, CLASSIC, 2);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, confirm());
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Picked(TournamentFighter::Ryu)));
    assert_eq!(tap(&mut s, stick(RIGHT)).transition, None, "no live opponent right of Ken");
    assert_eq!(tap(&mut s, stick(DOWN)).transition, None, "nor below");
    assert_eq!(tap(&mut s, stick(LEFT)).transition, Some(Transition::CursorMoved));
    assert_eq!(s.step(confirm()).transition, Some(Transition::Fighters(TournamentFighter::Ryu, TournamentFighter::Ryu)));
    // With it, Retsu, Geki and Joe too.
    let a = assets().unwrap();
    let Some(art) = art(&a) else { return };
    let s = cpu_fight(session(a, Some(&art), CLASSIC, 2), TournamentFighter::Ken, TournamentFighter::Retsu);
    assert!(s.fight_machine().unwrap().fair_cpu_pad().is_some());
}

#[test]
fn faithful_1987_training_keeps_capcoms_cpu() {
    let Some(a) = assets() else { return };
    let mut s = session(a, None, RulesetPreset::Faithful1987, 2);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, confirm());
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    tap(&mut s, confirm()); // Ryu
    let out = s.step(confirm()); // the first live opponent, a flag's first CPU
    let Some(Transition::Fighters(_, opponent)) = out.transition else { panic!("{:?}", out.transition) };
    assert!(!opponent.is_shoto());
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000);
    }
    for _ in 0..600 {
        s.step(idle());
    }
    let m = s.fight_machine().unwrap();
    assert!(!m.match_config().p2_fair_cpu);
    assert!(m.fair_cpu_pad().is_none(), "no fair CPU in the machine");
    assert_eq!(game_mode(m), 2, "one player against Capcom's CPU");
    assert!(fighters(&s)[1].present);
}

#[test]
fn under_the_modern_rules_the_cpu_uses_combos_cancels_throws_and_throw_techs_and_never_under_classic() {
    let Some(a) = assets() else { return };
    let Some(art) = art(&a) else { return };
    let throw_mv = sf_engine::combo::throw::THROW_MOVE;
    let thrown_mv = sf_engine::combo::throw::THROWN_MOVE;

    // 1. With the modern switches ON (`DefinitiveOverhaul`), the CPU lands
    //    true combos (`into_hitstun`, HUD combo count >= 2), cancels normals
    //    into specials (`cancel.count[1] > 0`), and throws (`THROW_MOVE`).
    for cpu in [TournamentFighter::Ken, TournamentFighter::Retsu] {
        let a = assets().unwrap();
        let mut s = cpu_fight(
            session(a, Some(&art), RulesetPreset::DefinitiveOverhaul, 4),
            TournamentFighter::Ryu,
            cpu,
        );
        let (w, _) = spar(&mut s, 17, 6_000);
        println!("modern {cpu:?}: {w:?}");
        assert!(
            w.cpu_combos > 0 && w.cpu_max_combo >= 2,
            "{cpu:?} lands true combos under DefinitiveOverhaul: {w:?}"
        );
        assert!(
            w.cpu_cancels > 0,
            "{cpu:?} cancels normals into specials under DefinitiveOverhaul: {w:?}"
        );
        assert!(
            w.saw(|m| m == throw_mv) > 0,
            "{cpu:?} throws under DefinitiveOverhaul: {w:?}"
        );
    }

    // 2. Throw techs under `DefinitiveOverhaul`: when P1 walks into throw
    //    range and throws P2 (`Forward + HP`), the CPU answers the walk-in
    //    with a delayed throw tech inside the 7-frame `TECH_WINDOW`.
    let a = assets().unwrap();
    let mut s = cpu_fight(
        session(a, Some(&art), RulesetPreset::DefinitiveOverhaul, 4),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    );
    let mut grabs = 0u32;
    let mut teched = 0u32;
    let mut prev_window = 0u8;
    let mut in_range_frames = 0u8;
    let mut toggle_hp = false;
    for _ in 0..2_400 {
        let [p1, p2] = fighters(&s);
        let m = s.fight_machine().unwrap();
        let invuln = m.toggle_state().throw.wakeup_invuln[1];
        let win = m.toggle_state().throw.tech_window[1];
        let seen = perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2);
        let fwd = if seen.me.x >= seen.foe.x { RIGHT } else { LEFT };
        let dist = (seen.me.x - seen.foe.x).abs();
        let can_try = !p1.attacking() && !p1.reacting() && !jumping(p1.move_id) && !p1.getting_up;
        let p2_throwable = !p2.reacting()
            && !p2.getting_up
            && !jumping(p2.move_id)
            && !p2.blockstun
            && invuln == 0
            && win == 0;
        let input = if !can_try || !p2_throwable {
            in_range_frames = 0;
            toggle_hp = false;
            idle()
        } else if dist > 46 {
            in_range_frames = 0;
            toggle_hp = false;
            with(|i| i.p1.directions[fwd] = true)
        } else if in_range_frames < 8 {
            in_range_frames += 1;
            toggle_hp = false;
            with(|i| i.p1.directions[fwd] = true)
        } else {
            toggle_hp = !toggle_hp;
            with(|i| {
                i.p1.directions[fwd] = true;
                if toggle_hp {
                    i.p1.buttons[2] = true;
                }
            })
        };
        s.step(input);
        let [p1_now, p2_now] = fighters(&s);
        let win_now = s.fight_machine().unwrap().toggle_state().throw.tech_window[1];
        if prev_window == 0 && win_now > 0 && p2_now.move_id == thrown_mv {
            grabs += 1;
        }
        if prev_window > 0 && win_now == 0 && p1_now.move_id == 1 && p2_now.move_id == 1 {
            teched += 1;
        }
        prev_window = win_now;
    }
    println!("throw tech trials: {teched} teched of {grabs} grabs");
    assert!(grabs > 0 && teched > 0, "the CPU techs throws under DefinitiveOverhaul ({teched}/{grabs})");

    // 3. With the modern switches OFF (`ClassicCompetitive`), the CPU never
    //    lands hitstun combos, never cancels a normal into a special, and
    //    never throws.
    let a = assets().unwrap();
    let mut s = cpu_fight(
        session(a, Some(&art), CLASSIC, 4),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    );
    let (w, _) = spar(&mut s, 17, 6_000);
    assert_eq!(w.cpu_combos, 0, "no true combos under ClassicCompetitive");
    assert_eq!(w.cpu_max_combo, 0, "no combo HUD under ClassicCompetitive");
    assert_eq!(w.cpu_cancels, 0, "no cancels under ClassicCompetitive");
    assert_eq!(w.saw(|m| m == throw_mv), 0, "no throws under ClassicCompetitive");
}


