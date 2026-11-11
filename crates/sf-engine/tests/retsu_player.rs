//! Ticket #248 (`SP2`): Retsu as a 1987 player fighter, the first full
//! slice of ADR-0014.
//!
//! At the headless session's seam, driven by inputs: in Local under Classic
//! Competitive, Retsu plays on his 1987 player table in the extended data bank and the
//! translated 1987 player handler runs him: he walks, jumps straight and
//! diagonally, crouches, blocks high and low, has a normal on every button
//! standing, crouching and in the air, and the Kiai Shot comes out of the
//! Hadoken's command tracker. What is checked is read back through the
//! presented frames, the fight reader and the session's log; each move's
//! startup, active and recovery frames are read from the fight reader and
//! compared with the move's data in the bank (and the ROM, for his 1987
//! attacks). The boxes the bank writes for his new poses are checked
//! against the drawings' pens. Local ROMs and the local art pack only; each
//! test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::{in_bank, DataBank, PoseGeom, APEX_POINTER, RETSU_BOXES};
use sf_engine::fight_reader::FighterRead;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::coach::Outcome;
use sf_engine::front::moves::Choice;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::moves::{move_list, Strength};
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art_from, Art, ArtSources, Pens, TRANSPARENT};
use sf_video::art_pack::ArtPack;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const RECIPES: [&str; 2] = [include_str!("../../../art/recipes/ryu.json"), include_str!("../../../art/recipes/retsu.json")];
const REVIEWS: [&str; 2] = [include_str!("../../../art/review/ryu.json"), include_str!("../../../art/review/retsu.json")];

/// The game flow's fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
/// Classic Competitive.
const CLASSIC: RulesetPreset = RulesetPreset::ClassicCompetitive;
/// Where Retsu's slot of the extended data bank starts: his place in
/// `TournamentFighter::ALL` (2) times the slot size.
const RETSU_TABLE: u32 = 0x12_0000;

struct Local {
    a: Assets,
    art: Art,
}

fn local_roms() -> Option<Local> {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROM set or art pack not present");
        return None;
    }
    let a = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(&a, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).expect("the art loads");
    assert!(art.catalogue("retsu").is_some(), "Retsu's poses load: {:?}", art.kept);
    Some(Local { a, art })
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn hold(dirs: &[usize]) -> FrameInputs {
    with(|i| dirs.iter().for_each(|&d| i.p1.directions[d] = true))
}

fn press(dirs: &[usize], button: usize) -> FrameInputs {
    with(|i| {
        dirs.iter().for_each(|&d| i.p1.directions[d] = true);
        i.p1.buttons[button] = true;
    })
}

fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(idle());
    out
}

fn until(s: &mut GameSession, name: &str, limit: u32) {
    for _ in 0..limit {
        if s.step(idle()).screen == name {
            return;
        }
    }
    panic!("never reached {name} in {limit} frames (at {})", s.screen());
}

/// Step until the fight loop, then through the round's intro to where the
/// fighters take control.
fn to_control(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    for _ in 0..240 {
        s.step(idle());
    }
}

/// A session given the extended sprite bank and, with `tables`, the
/// extended data bank built from the loaded catalogues.
fn session(l: &Local, preset: RulesetPreset, tables: bool) -> GameSession {
    let a = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let s = GameSession::new(a, MatchConfig::from_preset(preset)).with_sprite_bank(&l.art.bank);
    if tables {
        s.with_player_tables(&l.art.catalogues)
    } else {
        s
    }
}

/// Local: P1 `p1`, P2 Ken (where P2's cursor starts).
fn local_with(mut s: GameSession, p1: TournamentFighter) -> GameSession {
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, hold(&[DOWN]));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Multi Player
    tap(&mut s, with(|i| i.menu.confirm = true)); // Local
    let at = TournamentFighter::ALL.iter().position(|&f| f == p1).unwrap();
    if at >= 6 {
        tap(&mut s, hold(&[DOWN]));
    }
    for _ in 0..at % 6 {
        tap(&mut s, hold(&[RIGHT]));
    }
    assert_eq!(select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), tap).transition, Some(Transition::Picked(p1)));
    let out = select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), GameSession::step);
    assert_eq!(out.transition, Some(Transition::Fighters(p1, TournamentFighter::Ken)));
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    s
}

/// Local under Classic Competitive, Retsu against Ken, at control.
fn retsu_local(l: &Local) -> GameSession {
    local_with(session(l, CLASSIC, true), TournamentFighter::Retsu)
}

fn p1(s: &GameSession) -> FighterRead {
    s.fight().unwrap().fight_read().fighters[0]
}

fn p2(s: &GameSession) -> FighterRead {
    s.fight().unwrap().fight_read().fighters[1]
}

fn rl(s: &GameSession, addr: u32) -> u32 {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (addr - 0xff8000) as usize;
    u32::from_be_bytes(w[a..a + 4].try_into().unwrap())
}

/// P1's fighter entity in object RAM.
fn p1_entity(s: &GameSession) -> usize {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (P1_STRUCT + 0x10 - 0xff8000) as usize;
    u16::from_be_bytes([w[a], w[a + 1]]) as usize
}

/// P1's grid: its top and left on screen.
fn p1_grid(s: &GameSession) -> (u16, u16) {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = p1_entity(s);
    (u16::from_be_bytes([o[e + 4], o[e + 5]]), u16::from_be_bytes([o[e + 6], o[e + 7]]))
}

fn p1_bytes(s: &GameSession, off: usize, n: usize) -> Vec<u8> {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = p1_entity(s);
    o[e + off..e + off + n].to_vec()
}

// ---- the move's data ---------------------------------------------------------

/// Reads the bank or the ROM by address, as the machine's bus does.
struct Data<'a> {
    rom: &'a [u8],
    bank: &'a DataBank,
}

impl Data<'_> {
    fn b(&self, a: u32) -> u8 {
        if in_bank(a) {
            self.bank.rb(a)
        } else {
            self.rom.get(a as usize).copied().unwrap_or(0xff)
        }
    }

    fn l(&self, a: u32) -> u32 {
        u32::from_be_bytes([self.b(a), self.b(a + 1), self.b(a + 2), self.b(a + 3)])
    }

    /// Each frame of a stream from `a`: whether its attack box is live. A
    /// frame record shows `(hold + 1) * (duration + 1)` frames; the stream
    /// ends at `0xfd` or at the CPU follow-up (`0x45630`), which ends a
    /// player's move; other calls go on.
    fn stream(&self, mut a: u32, out: &mut Vec<bool>) {
        for _ in 0..200 {
            match self.b(a) {
                0xfd => return,
                0xfe => a = self.l(a + 2),
                0xff => {
                    if self.l(a + 2) == 0x45630 {
                        return;
                    }
                    a += 6;
                }
                hold => {
                    let n = (hold as usize + 1) * (self.b(a + 1) as usize + 1);
                    out.extend(std::iter::repeat_n(self.b(a + 0x1a) & 0x80 != 0, n));
                    a += 0x20;
                }
            }
        }
        panic!("a stream that does not end");
    }

    /// A move started from its header: the header's frames, then its stream.
    fn move_frames(&self, table: u32, id: u8) -> Vec<bool> {
        let h = sf_engine::data_bank::move_header(|address| self.l(address), table, id).expect("move header");
        let n = (self.b(h + 0xb) as usize + 1) * (self.b(h + 0xe) as usize + 1);
        let mut out = vec![self.b(h + 0xf) & 0x80 != 0; n];
        self.stream(self.l(h + 0x2c), &mut out);
        out
    }

    /// A jump attack, from the apex record's stream for `jump` (9, 0xa,
    /// 0xb), `kick` or not, at strength level `level` (1..=3).
    fn jump_frames(&self, table: u32, jump: u8, kick: bool, level: u32) -> Vec<bool> {
        let rec = self.l(table + APEX_POINTER);
        let entry = rec + if kick { 0x1c } else { 4 } + (jump as u32 - 9) * 8;
        let stream = self.l(self.l(entry + 4) + (level - 1) * 4);
        let mut out = Vec::new();
        self.stream(stream, &mut out);
        out
    }
}

/// Feed `inputs`, then `after`, and read the move `id` P1 is in from the
/// frame it starts until it ends: whether the attack box is live on each of
/// its frames (startup, active and recovery as the fight reader sees them).
fn measure_then(s: &mut GameSession, inputs: &[FrameInputs], after: FrameInputs, id: u8) -> Vec<bool> {
    let mut frames = Vec::new();
    let mut started = false;
    for n in 0..400 {
        s.step(inputs.get(n).copied().unwrap_or(after));
        let r = p1(s);
        let in_move = r.move_id == id && r.state & 0x80 != 0;
        if in_move {
            started = true;
            frames.push(r.attack_active);
        } else if started {
            return frames;
        }
    }
    panic!("move {id:#04x} did not {} (now {:#04x})", if started { "end" } else { "start" }, p1(s).move_id);
}

fn measure(s: &mut GameSession, inputs: &[FrameInputs], id: u8) -> Vec<bool> {
    measure_then(s, inputs, idle(), id)
}

/// `(startup, active, recovery)` of an active-flag timeline.
fn phases(t: &[bool]) -> (usize, usize, usize) {
    let first = t.iter().position(|&a| a).unwrap_or(t.len());
    let active = t[first..].iter().take_while(|&&a| a).count();
    (first, active, t.len() - first - active)
}

// ---- the tests ---------------------------------------------------------------

#[test]
fn retsu_plays_on_his_bank_table_and_the_other_bosses_keep_theirs() {
    let Some(l) = local_roms() else { return };
    let mut s = retsu_local(&l);
    assert_eq!(rl(&s, P1_STRUCT + 0x12), RETSU_TABLE, "Retsu is bound to his 1987 player table");
    assert_eq!(rl(&s, P2_STRUCT + 0x12), TournamentFighter::Ken.move_table());
    assert_eq!(s.data_bank().unwrap().table(TournamentFighter::Retsu), Some(RETSU_TABLE));
    assert_eq!(s.data_bank().unwrap().fighters(), [TournamentFighter::Retsu]);
    let log = s.take_log();
    assert!(log.iter().any(|l| l.contains("Retsu plays a 1987 player table")), "{log:?}");
    // The player handler runs him: a crouch held is Ryu's kind of still
    // posture (no busy bit), where his boss-only crouch stood him up.
    for _ in 0..90 {
        s.step(hold(&[DOWN]));
        assert_eq!((p1(&s).move_id, p1(&s).state), (0x05, 0x14));
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Geki, with no table in the bank, is greyed out on the select screen;
    // without the bank (no art for him) Retsu is greyed out too.
    let select = sf_engine::front::select::SelectScreen::local(&MatchConfig::from_preset(CLASSIC), &[TournamentFighter::Retsu]);
    assert!(!select.greyed(2) && select.greyed(3));
    let select = sf_engine::front::select::SelectScreen::local(&MatchConfig::from_preset(CLASSIC), &[]);
    assert!(select.greyed(2));
}

#[test]
fn he_walks_crouches_and_jumps_straight_and_diagonally() {
    let Some(l) = local_roms() else { return };
    let start = retsu_local(&l);
    let mut s = start.clone();
    let (_, x0) = p1_grid(&s);
    for _ in 0..30 {
        s.step(hold(&[RIGHT]));
        assert_eq!(p1(&s).move_id, 0x02);
    }
    let (_, x1) = p1_grid(&s);
    assert!(x1 > x0 + 20, "walked forward: {x0} -> {x1}");
    for _ in 0..30 {
        s.step(hold(&[LEFT]));
        assert_eq!(p1(&s).move_id, 0x03);
    }
    // His 1987 walk back moves only by its header's step, 10 pixels a
    // cycle.
    assert!(p1_grid(&s).1 < x1 - 10, "walked back");
    for (dirs, id) in [(vec![DOWN], 0x05), (vec![DOWN, RIGHT], 0x06), (vec![DOWN, LEFT], 0x07)] {
        for _ in 0..20 {
            s.step(hold(&dirs));
        }
        assert_eq!((p1(&s).move_id, p1(&s).state), (id, 0x14), "{dirs:?}");
        assert_eq!(p1_grid(&s).0, 0xa8, "his crouch on the ground");
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
    // Each jump leaves the ground, moves the way it was aimed and lands on
    // the ground line in his stance.
    for (dirs, id, way) in [(vec![UP], 0x09, 0i32), (vec![UP, RIGHT], 0x0a, 1), (vec![UP, LEFT], 0x0b, -1)] {
        let mut s = start.clone();
        let (top0, x0) = p1_grid(&s);
        assert_eq!(top0, 0x88);
        s.step(hold(&dirs));
        s.step(hold(&dirs));
        assert_eq!(p1(&s).move_id, id);
        let mut highest = top0;
        for _ in 0..90 {
            s.step(idle());
            highest = highest.min(p1_grid(&s).0);
            if p1(&s).move_id == 0x01 {
                break;
            }
        }
        assert_eq!(p1(&s).move_id, 0x01, "{dirs:?}: landed in his stance");
        let (top, x) = p1_grid(&s);
        assert_eq!(top, 0x88, "{dirs:?}: on the ground");
        assert!(top0 - highest >= 60, "{dirs:?}: rose {} pixels", top0 - highest);
        let moved = x as i32 - x0 as i32;
        match way {
            0 => assert_eq!(moved, 0, "straight up"),
            _ => assert!(moved * way > 40, "{dirs:?}: moved {moved}"),
        }
        assert!(s.fight_machine().unwrap().unrecovered().is_empty());
    }
}

/// Walk P1 into reach of Ken, then have Ken attack with button `ken.1`
/// (crouching with `ken.0`) while P1 holds `guard`: the moves P1 showed,
/// and his health before and after.
fn guard_against(start: &GameSession, ken: (bool, usize), guard: &[usize]) -> (Vec<u8>, u16, u16) {
    let mut s = start.clone();
    for _ in 0..120 {
        s.step(hold(&[RIGHT]));
    }
    let health = p1(&s).health;
    let mut moves = Vec::new();
    let mut ken_attacked = 0;
    for f in 0..240u32 {
        let mut i = hold(guard);
        i.p2.directions[DOWN] = ken.0;
        if f % 40 < 2 {
            i.p2.buttons[ken.1] = true;
        }
        s.step(i);
        if (0x0d..=0x2f).contains(&p2(&s).move_id) {
            ken_attacked += 1;
        }
        let m = p1(&s).move_id;
        if !moves.contains(&m) {
            moves.push(m);
        }
    }
    assert!(ken_attacked > 20, "Ken attacked");
    (moves, health, p1(&s).health)
}

#[test]
fn he_blocks_high_holding_back_and_low_holding_down_back() {
    let Some(l) = local_roms() else { return };
    let start = retsu_local(&l);
    // Ken's standing medium punch lands unguarded and not through his
    // standing guard.
    let (_, before, after) = guard_against(&start, (false, 1), &[]);
    assert!(after < before, "an unguarded punch lands");
    let (moves, before, after) = guard_against(&start, (false, 1), &[LEFT]);
    assert!(moves.contains(&0x04), "his standing guard: {moves:x?}");
    assert_eq!(after, before, "no damage through the standing guard");
    // Ken's crouching kick, guarded crouching.
    let (moves, before, after) = guard_against(&start, (true, 3), &[DOWN, LEFT]);
    assert!(moves.contains(&0x08), "his crouching guard: {moves:x?}");
    assert_eq!(after, before, "no damage through the crouching guard");
    // Not guarding, the same kick lands.
    let (_, before, after) = guard_against(&start, (true, 3), &[]);
    assert!(after < before, "an unguarded kick lands");
}

/// Every normal: the inputs that bring it out from his stance, what is
/// held afterwards, the move it is and its strength level. Standing,
/// crouching, and from each jump (the button pressed once he is in the
/// air).
fn normals() -> Vec<(String, Vec<FrameInputs>, FrameInputs, u8, u32)> {
    let mut out = Vec::new();
    let names = ["LP", "MP", "HP", "LK", "MK", "HK"];
    let level = |b: usize| (b % 3) as u32 + 1;
    for (b, &id) in [0x0d, 0x0f, 0x10, 0x14, 0x15, 0x16].iter().enumerate() {
        out.push((format!("standing {}", names[b]), vec![press(&[], b), press(&[], b)], idle(), id, level(b)));
    }
    for (b, &id) in [0x11, 0x1c, 0x1c, 0x17, 0x17, 0x18].iter().enumerate() {
        let mut inputs = vec![hold(&[DOWN]); 4];
        inputs.extend([press(&[DOWN], b), press(&[DOWN], b)]);
        out.push((format!("crouching {}", names[b]), inputs, hold(&[DOWN]), id, level(b)));
    }
    for (jump, dirs, ids) in [
        ("straight", vec![UP], [0x20, 0x20, 0x20, 0x23, 0x23, 0x26]),
        ("forward", vec![UP, RIGHT], [0x21, 0x21, 0x21, 0x24, 0x24, 0x24]),
        ("back", vec![UP, LEFT], [0x22, 0x22, 0x22, 0x25, 0x25, 0x25]),
    ] {
        for (b, &id) in ids.iter().enumerate() {
            let mut inputs = vec![hold(&dirs); 2];
            inputs.extend(vec![idle(); 12]);
            inputs.extend([press(&[], b), press(&[], b)]);
            out.push((format!("{jump} jump {}", names[b]), inputs, idle(), id, level(b)));
        }
    }
    out
}

#[test]
fn every_normal_comes_out_and_its_frames_match_its_data() {
    let Some(l) = local_roms() else { return };
    let mut start = retsu_local(&l);
    // Back off from Ken, and Ken from him, so nothing connects (a hit ends
    // the active frames).
    for _ in 0..60 {
        start.step(with(|i| {
            i.p1.directions[LEFT] = true;
            i.p2.directions[RIGHT] = true;
        }));
    }
    for _ in 0..30 {
        start.step(idle());
    }
    let bank = start.data_bank().unwrap().clone();
    let data = Data { rom: &l.a.maincpu, bank: &bank };
    for (name, inputs, after, id, level) in normals() {
        let mut s = start.clone();
        let measured = measure_then(&mut s, &inputs, after, id);
        let expected = match id {
            0x20..=0x26 => {
                let jump = match id {
                    0x20 | 0x23 | 0x26 => 0x09,
                    0x21 | 0x24 => 0x0a,
                    _ => 0x0b,
                };
                data.jump_frames(RETSU_TABLE, jump, id >= 0x23, level)
            }
            _ => data.move_frames(RETSU_TABLE, id),
        };
        let (su, ac, re) = phases(&measured);
        println!("{name:20} move {id:#04x}: startup {su:2}, active {ac:2}, recovery {re:2}");
        assert!(ac > 0, "{name}: an attack");
        assert_eq!(measured, expected, "{name}: the frames as the fight reader reads them are the move's data");
        assert!(s.fight_machine().unwrap().unrecovered().is_empty(), "{name}");
    }
}

/// Hold down, down-forward, forward and press `button` (facing right).
fn qcf(button: usize) -> Vec<FrameInputs> {
    let mut v = vec![hold(&[DOWN]); 3];
    v.extend(vec![hold(&[DOWN, RIGHT]); 3]);
    v.extend(vec![hold(&[RIGHT]); 2]);
    v.extend([press(&[RIGHT], button), press(&[RIGHT], button)]);
    v
}

/// Walk Retsu in to Ken, then throw the Kiai Shot with Ken guarding: the
/// health Ken lost.
fn guarded_chip(mut s: GameSession) -> u16 {
    for _ in 0..150 {
        s.step(hold(&[RIGHT]));
    }
    let health = p2(&s).health;
    let mut inputs = qcf(0);
    inputs.iter_mut().for_each(|i| i.p2.directions[RIGHT] = true);
    for input in inputs.into_iter().chain(vec![with(|i| i.p2.directions[RIGHT] = true); 40]) {
        s.step(input);
    }
    health - p2(&s).health
}

/// The live effect objects: (x, y) of each, slot by slot.
fn effects(s: &GameSession) -> Vec<(u16, u16)> {
    let o = &s.fight_machine().unwrap().state.objectram;
    (0..12usize)
        .map(|k| 0x100 + 0x40 * k)
        .filter(|&e| o[e + 8] != 0)
        .map(|e| (u16::from_be_bytes([o[e + 6], o[e + 7]]), u16::from_be_bytes([o[e + 4], o[e + 5]])))
        .collect()
}

/// P1's fighter's centre x on screen: his grid's left plus half its width.
fn p1_centre_x(s: &GameSession) -> i32 {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = p1_entity(s);
    let cols = o[e + 0xd] as i32 + 1;
    let cell = if u16::from_be_bytes([o[e + 2], o[e + 3]]) & 0x400 != 0 { 32 } else { 16 };
    p1_grid(s).1 as i32 + cols * cell / 2 - o[e + 0x26] as i8 as i32
}

#[test]
fn the_kiai_shot_is_a_short_range_blast_from_its_command() {
    let Some(l) = local_roms() else { return };
    let start = retsu_local(&l);
    let bank = start.data_bank().unwrap().clone();
    let data = Data { rom: &l.a.maincpu, bank: &bank };

    // At range: the blast comes out in front of him, travels a little,
    // fades and is gone, Ken untouched; the move's frames are its data.
    let mut far = start.clone();
    for _ in 0..60 {
        far.step(with(|i| {
            i.p1.directions[LEFT] = true;
            i.p2.directions[RIGHT] = true;
        }));
    }
    let mut s = far.clone();
    let health = p2(&s).health;
    let mut life = Vec::new();
    let mut drawn = Vec::new();
    let mut started = None;
    let mut frames = Vec::new();
    for (n, input) in qcf(0).into_iter().chain(vec![idle(); 90]).enumerate() {
        s.step(input);
        let r = p1(&s);
        if r.move_id == 0x1a && r.state & 0x80 != 0 {
            started.get_or_insert(n);
            frames.push(r.attack_active);
        }
        if let Some(&(x, _)) = effects(&s).first() {
            life.push((n, x as i32 + 16 - p1_centre_x(&s)));
            let o = &s.fight_machine().unwrap().state.objectram;
            let e = (0..12usize).map(|k| 0x100 + 0x40 * k).find(|&e| o[e + 8] != 0).unwrap();
            let (code, attr) = (u16::from_be_bytes([o[e], o[e + 1]]), u16::from_be_bytes([o[e + 2], o[e + 3]]));
            if drawn.last() != Some(&(code, attr)) {
                drawn.push((code, attr));
            }
        }
    }
    assert_eq!(frames, data.move_frames(RETSU_TABLE, 0x1a), "its frames are its data");
    assert!(frames.iter().all(|&a| !a), "the palm itself does not hit: the blast does");
    let started = started.expect("the Kiai Shot came out");
    let (first, reach0) = life[0];
    let (last, reach1) = *life.last().unwrap();
    println!("blast: frames {}..={} of the move, {reach0} to {reach1} pixels ahead of his centre", first - started + 1, last - started + 1);
    assert_eq!(first - started + 1, 13, "the blast is out on the move's 13th frame (stance 3, wind-up 8, palm 1, then it runs)");
    assert!(last - first < 20, "it fades within 20 frames");
    assert!(reach0 >= 48 && reach1 <= 110, "a short range in front of him: {reach0}..{reach1}");
    assert!(life.windows(2).all(|w| w[1].0 == w[0].0 + 1), "one blast, alive frame after frame until it fades");
    assert_eq!(p2(&s).health, health, "Ken out of its reach is not hit");
    // It is drawn with its six approved frames (the maintainer, 2026-11-08) from
    // the extended sprite bank, one 32-pixel entry in his palette, in the
    // recipe's order: full, full, full, then the four fading.
    let retsu = l.art.catalogue("retsu").unwrap();
    let code = |n: usize| retsu.pose(&format!("kiai_blast_{n}")).unwrap().grid.code;
    let codes: Vec<u16> = drawn.iter().map(|&(c, _)| c).collect();
    assert_eq!(codes, [1, 2, 1, 3, 4, 5, 6].map(code), "its frames in order");
    assert!(drawn.iter().all(|&(c, a)| c >= 0x3800 && a & 0x4ff == 0x403), "bank tiles, a 32-pixel entry in his palette 3: {drawn:x?}");

    // Close in, it hits Ken for its 14.
    let mut s = start.clone();
    for _ in 0..150 {
        s.step(hold(&[RIGHT]));
    }
    let health = p2(&s).health;
    let measured = measure(&mut s, &qcf(0), 0x1a);
    assert_eq!(measured, data.move_frames(RETSU_TABLE, 0x1a), "a hit does not cut it short");
    for _ in 0..30 {
        s.step(idle());
    }
    assert_eq!(health - p2(&s).health, 14, "the blast's damage");
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Where it hits, it breaks up: its impact (its own fading frames) plays
    // at the blast's place, and the sounds go launch, launch (the routine,
    // then the blast's first frame), hit reaction, then the explosion's 06,
    // with no third launch.
    let mut s = start.clone();
    for _ in 0..150 {
        s.step(hold(&[RIGHT]));
    }
    let retsu = l.art.catalogue("retsu").unwrap();
    let impact_code = retsu.pose("kiai_blast_4").unwrap().grid.code;
    let mut sounds = Vec::new();
    let mut last_blast = None;
    let mut impact_at = None;
    for input in qcf(0).into_iter().chain(vec![idle(); 40]) {
        let out = s.step(input);
        if out.sound_latch != 0 {
            sounds.push(out.sound_latch);
        }
        let o = &s.fight_machine().unwrap().state.objectram;
        if let Some(e) = (0..12usize).map(|k| 0x100 + 0x40 * k).find(|&e| o[e + 8] != 0) {
            let code = u16::from_be_bytes([o[e], o[e + 1]]);
            let at = (u16::from_be_bytes([o[e + 6], o[e + 7]]) as i32, u16::from_be_bytes([o[e + 4], o[e + 5]]) as i32);
            if code == impact_code && impact_at.is_none() && last_blast.is_some() {
                impact_at = Some(at);
            } else if impact_at.is_none() {
                last_blast = Some(at);
            }
        }
    }
    let (blast, impact) = (last_blast.expect("the blast"), impact_at.expect("its break-up"));
    assert!((impact.0 - blast.0).abs() <= 4 && impact.1 == blast.1, "it breaks up where it hit: blast {blast:?}, impact {impact:?}");
    let launches: Vec<usize> = sounds.iter().enumerate().filter(|&(_, &c)| c == 0x07).map(|(i, _)| i).collect();
    let explosion = sounds.iter().position(|&c| c == 0x06);
    println!("sounds: {sounds:02x?}");
    assert_eq!(launches.len(), 2, "launched once, by the routine and the blast's first frame: {sounds:02x?}");
    assert!(explosion.is_some_and(|x| x > launches[1]), "then the explosion's sound: {sounds:02x?}");

    // Guarded, it chips 4 with Balanced roster off, as Classic Competitive
    // has it; Balanced roster's chip rule makes it 1 (the maintainer, 2026-11-08).
    assert!(!MatchConfig::from_preset(CLASSIC).toggles.balanced_roster);
    assert_eq!(guarded_chip(start.clone()), 4, "the blast's chip on a guard");
    let mut balanced = local_with(session(&l, RulesetPreset::DefinitiveOverhaul, true), TournamentFighter::Retsu);
    assert!(balanced.fight_machine().unwrap().match_config().toggles.balanced_roster);
    balanced.step(idle());
    assert_eq!(guarded_chip(balanced), 1, "under Balanced roster");

    // A forward punch without the command is his jab; the Shoryuken's and
    // the Tatsumaki's commands bring out no special he lacks: the punch and
    // kick they end on.
    let mut s = start.clone();
    assert_eq!(measure(&mut s, &[press(&[RIGHT], 0), press(&[RIGHT], 0)], 0x0d), data.move_frames(RETSU_TABLE, 0x0d));
    let mut s = start.clone();
    let mut dp = vec![hold(&[RIGHT]); 3];
    dp.extend(vec![hold(&[DOWN]); 3]);
    dp.extend([press(&[DOWN, RIGHT], 0), press(&[DOWN, RIGHT], 0)]);
    assert_eq!(measure_then(&mut s, &dp, hold(&[DOWN, RIGHT]), 0x12), data.move_frames(RETSU_TABLE, 0x12));
    let mut s = start.clone();
    let mut tatsu = vec![hold(&[DOWN]); 3];
    tatsu.extend(vec![hold(&[DOWN, LEFT]); 3]);
    tatsu.extend([press(&[LEFT], 3), press(&[LEFT], 3)]);
    assert_eq!(measure(&mut s, &tatsu, 0x14), data.move_frames(RETSU_TABLE, 0x14));
}

/// The fraction of `rect` (in a pose's pixels) that is drawn.
fn drawn(p: &Pens, [x0, y0, x1, y1]: [i32; 4]) -> f64 {
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            if (x as usize) < p.width && (y as usize) < p.height && p.at(x as usize, y as usize) != TRANSPARENT {
                n += 1;
            }
        }
    }
    n as f64 / ((x1 - x0) * (y1 - y0)) as f64
}

fn inside([x0, y0, x1, y1]: [i32; 4], x: usize, y: usize) -> bool {
    (x0..x1).contains(&(x as i32)) && (y0..y1).contains(&(y as i32))
}

fn geom(l: &Local, name: &str) -> PoseGeom {
    let p = l.art.catalogue("retsu").unwrap().pose(name).unwrap();
    PoseGeom { code: p.grid.code, attr: if p.grid.large { 0x403 } else { 3 }, rows: p.grid.rows, cols: p.grid.cols, anchor: p.anchor }
}

#[test]
fn every_box_the_bank_writes_matches_the_drawn_pose() {
    let Some(l) = local_roms() else { return };
    for (name, boxes) in RETSU_BOXES {
        let pens = &l.art.built.iter().find(|(f, n, _)| f == "retsu" && n == name).unwrap_or_else(|| panic!("{name} drawn")).2;
        // The head is drawn; the body is mostly drawn and holds nearly all
        // of the fighter outside the attack.
        assert!(drawn(pens, boxes.head) >= 0.8, "{name}: head {:.2}", drawn(pens, boxes.head));
        assert!(drawn(pens, boxes.body) >= 0.6, "{name}: body {:.2}", drawn(pens, boxes.body));
        let (mut body, mut all) = (0, 0);
        for y in 0..pens.height {
            for x in 0..pens.width {
                if pens.at(x, y) == TRANSPARENT || boxes.attack.is_some_and(|a| inside(a, x, y)) {
                    continue;
                }
                all += 1;
                body += usize::from(inside(boxes.body, x, y));
            }
        }
        assert!(body as f64 >= 0.8 * all as f64, "{name}: the body box holds {body} of {all} drawn pixels");
        assert!(drawn(pens, boxes.push) >= 0.6, "{name}: push {:.2}", drawn(pens, boxes.push));
        // The attack is the limb: mostly drawn, holding the pose's
        // frontmost drawn pixel in its rows.
        if let Some(a) = boxes.attack {
            assert!(drawn(pens, a) >= 0.5, "{name}: attack {:.2}", drawn(pens, a));
            let front = (a[1]..a[3]).filter_map(|y| (0..pens.width).rev().find(|&x| pens.at(x, y as usize) != TRANSPARENT)).max().unwrap();
            assert!((a[0]..a[2]).contains(&(front as i32)), "{name}: the limb's tip at x {front} is in the attack box");
        }
        // The push box reaches the grid's centre, as its format needs.
        let (hx, hy) = geom(&l, name).half();
        assert!(boxes.push[0] <= hx && hx <= boxes.push[2] && boxes.push[1] <= hy && hy <= boxes.push[3], "{name}: push box about the centre");
    }
}

#[test]
fn a_strike_loads_its_own_tiles_and_the_boxes_written_for_its_drawing() {
    let Some(l) = local_roms() else { return };
    let mut start = retsu_local(&l);
    for _ in 0..40 {
        start.step(hold(&[LEFT]));
    }
    let boxes = |name: &str| RETSU_BOXES.iter().find(|(n, _)| *n == name).unwrap().1;
    let jump: Vec<FrameInputs> = [vec![hold(&[UP]); 2], vec![idle(); 6], vec![press(&[], 5); 2]].concat();
    for (name, inputs, after, id) in [
        ("crouching_punch_2", vec![hold(&[DOWN]), press(&[DOWN], 0), press(&[DOWN], 0)], hold(&[DOWN]), 0x11),
        ("crouching_kick", vec![hold(&[DOWN]), press(&[DOWN], 3), press(&[DOWN], 3)], hold(&[DOWN]), 0x17),
        ("jump_punch", [vec![hold(&[UP]); 2], vec![idle(); 6], vec![press(&[], 0); 2]].concat(), idle(), 0x20),
        ("jump_punch_down", [vec![hold(&[UP, RIGHT]); 2], vec![idle(); 6], vec![press(&[], 0); 2]].concat(), idle(), 0x21),
        ("jump_kick", [vec![hold(&[UP]); 2], vec![idle(); 6], vec![press(&[], 3); 2]].concat(), idle(), 0x23),
        ("jump_kick_high", jump.clone(), idle(), 0x26),
    ] {
        let mut s = start.clone();
        let g = geom(&l, name);
        let mut checked = false;
        for n in 0..200 {
            s.step(inputs.get(n).copied().unwrap_or(after));
            if p1(&s).move_id == id && p1(&s).attack_active {
                let tiles = p1_bytes(&s, 0, 2);
                let code = u16::from_be_bytes([tiles[0], tiles[1]]);
                // Facing right the first entry is the grid's own first tile.
                assert_eq!(code, g.code, "{name}: its own tiles from the extended sprite bank");
                assert!(code >= 0x3800, "{name}: a bank tile");
                assert_eq!(p1_bytes(&s, 0x16, 16), boxes(name).bytes(&g), "{name}: its boxes");
                checked = true;
                break;
            }
        }
        assert!(checked, "{name} struck");
    }
}

#[test]
fn the_fight_replays_identically_from_the_same_inputs() {
    let Some(l) = local_roms() else { return };
    let start = retsu_local(&l);
    let script: Vec<FrameInputs> = (0..600u32)
        .map(|f| {
            with(|i| {
                match (f / 40) % 6 {
                    0 => i.p1.directions[RIGHT] = true,
                    1 => i.p1.directions[DOWN] = true,
                    2 => i.p1.directions[UP] = true,
                    3 => i.p1.directions[LEFT] = true,
                    _ => {}
                }
                if f % 23 < 2 {
                    i.p1.buttons[(f / 23 % 6) as usize] = true;
                }
                if f % 31 < 2 {
                    i.p2.buttons[(f / 31 % 6) as usize] = true;
                }
                i.p2.directions[LEFT] = f % 90 < 30;
            })
        })
        .collect();
    let run = |s: &mut GameSession, inputs: &[FrameInputs]| -> Vec<(u64, [FighterRead; 2])> {
        inputs
            .iter()
            .map(|&i| {
                let out = s.step(i);
                let hash = out.frame.argb.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &p| (h ^ p as u64).wrapping_mul(0x100_0000_01b3));
                (hash, s.fight().unwrap().fight_read().fighters)
            })
            .collect()
    };
    let mut a = start.clone();
    let first = run(&mut a, &script);
    let mut b = start.clone();
    let second = run(&mut b, &script);
    assert_eq!(first, second, "the same inputs, the same frames and fight");
    assert!(first.iter().any(|(_, f)| (0x0d..=0x2f).contains(&f[0].move_id)), "Retsu attacked");
    assert_eq!(a.fight_machine().unwrap().save_snapshot(), b.fight_machine().unwrap().save_snapshot());
    // A snapshot taken halfway carries on as the fight played straight
    // through.
    let mut c = start.clone();
    run(&mut c, &script[..300]);
    let mut snapshot = c.clone();
    assert_eq!(run(&mut c, &script[300..]), first[300..].to_vec(), "straight on");
    assert_eq!(run(&mut snapshot, &script[300..]), first[300..].to_vec(), "from the snapshot");
}

fn open_moves(s: &mut GameSession) {
    assert_eq!(s.step(with(|i| i.menu.start = true)).transition, Some(Transition::Paused));
    s.step(idle());
    let at = PauseBox::open(true).items().iter().position(|&i| i == MenuItem::Moves).expect("a MOVES row");
    for _ in 0..at {
        tap(s, hold(&[DOWN]));
    }
    tap(s, with(|i| i.menu.confirm = true));
    assert!(s.move_list().is_some(), "the list is up");
}

fn choose(s: &mut GameSession, choice: Choice) {
    tap(s, with(|i| i.menu.confirm = true));
    if choice == Choice::Try {
        tap(s, hold(&[RIGHT]));
    }
    assert_eq!(s.move_list().unwrap().choosing(), Some(choice));
    let out = s.step(with(|i| i.menu.confirm = true));
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
    s.step(idle());
}

/// Training against the dummy under Classic Competitive, Retsu as P1, with
/// his table in the extended data bank or (`tables` false) without.
fn training(l: &Local, tables: bool) -> GameSession {
    let mut s = session(l, CLASSIC, tables);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Single Player
    tap(&mut s, hold(&[DOWN]));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Training
    tap(&mut s, hold(&[DOWN]));
    tap(&mut s, with(|i| i.menu.confirm = true)); // DUMMY
    tap(&mut s, hold(&[RIGHT]));
    tap(&mut s, hold(&[RIGHT]));
    select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), tap); // Retsu
    select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), GameSession::step); // the opponent under the cursor
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_control(&mut s);
    let table = if tables { RETSU_TABLE } else { TournamentFighter::Retsu.move_table() };
    assert_eq!(rl(&s, P1_STRUCT + 0x12), table);
    s
}

#[test]
fn moves_lists_the_kiai_shot_and_show_and_try_work() {
    let Some(l) = local_roms() else { return };
    for preset in [CLASSIC, RulesetPreset::DefinitiveOverhaul, RulesetPreset::PureFootsies1987Plus] {
        let names: Vec<_> = move_list(TournamentFighter::Retsu, preset).iter().map(|e| e.name).collect();
        assert_eq!(names, ["KIAI SHOT"], "{preset:?}");
    }
    assert!(move_list(TournamentFighter::Retsu, RulesetPreset::Faithful1987).is_empty());
    // Without his table (no art pack, or a pose missing) the fight has no
    // Kiai Shot, and MOVES does not list it.
    let text = sf_engine::front::moves::MovesPanel::open_in_fight(TournamentFighter::Retsu, CLASSIC, false).text();
    assert!(!text.iter().any(|line| line.contains("KIAI")), "{text:?}");
    let start = training(&l, true);
    let mut s = start.clone();
    open_moves(&mut s);
    let list = s.move_list().expect("the list");
    assert!(list.text().iter().any(|line| line.contains("KIAI SHOT")), "{:?}", list.text());
    // SHOW: his own fighter performs it.
    choose(&mut s, Choice::Show);
    let mut came_out = false;
    for _ in 0..400 {
        s.step(idle());
        came_out |= p1(&s).move_id == 0x1a;
    }
    assert!(came_out, "the show brought the Kiai Shot out");
    // TRY: the coach calls out the player's own.
    let mut s = start.clone();
    open_moves(&mut s);
    choose(&mut s, Choice::Try);
    let entry = move_list(TournamentFighter::Retsu, CLASSIC)[0];
    for p in entry.input.frames(true, Strength::Medium).into_iter().chain(vec![Default::default(); 90]) {
        s.step(with(|i| i.p1 = p));
    }
    let coach = s.coach().expect("the coach");
    assert_eq!(coach.entry().name, "KIAI SHOT");
    assert_eq!((coach.successes(), coach.outcome()), (1, Some(Outcome::Success)));
}

#[test]
fn a_whole_match_with_retsu_on_either_side_runs_clean() {
    let Some(l) = local_roms() else { return };
    // Retsu on P1 against Ken, both sides walking in and cycling buttons,
    // crouching and jumping, until the after-match box.
    let mut s = retsu_local(&l);
    let mut frames = 0u32;
    while s.screen() != "after match" {
        let f = frames;
        s.step(with(|i| {
            let phase = f % 160;
            i.p1.directions[RIGHT] = phase < 40;
            i.p1.directions[DOWN] = (60..80).contains(&phase);
            i.p1.directions[UP] = (100..104).contains(&phase);
            if phase % 12 < 2 {
                i.p1.buttons[((f / 12) % 6) as usize] = true;
            }
            i.p2.directions[LEFT] = (f + 80) % 160 < 40;
            if (f + 6) % 14 < 2 {
                i.p2.buttons[((f / 14 + 3) % 6) as usize] = true;
            }
        }));
        frames += 1;
        assert!(frames < 30_000, "the match ends");
        assert!(s.fight_machine().unwrap().unrecovered().is_empty(), "frame {frames}: {:05x?}", s.fight_machine().unwrap().unrecovered());
    }
    assert_eq!(rl(&s, P1_STRUCT + 0x12), RETSU_TABLE);
    println!("Retsu v Ken: {frames} frames to the after-match box");

    // Retsu on P2 (facing left), against Ryu.
    let mut s = session(&l, CLASSIC, true);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, hold(&[DOWN]));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Multi Player
    tap(&mut s, with(|i| i.menu.confirm = true)); // Local
    tap(&mut s, with(|i| i.menu.confirm = true)); // P1 Ryu
    tap(&mut s, with(|i| i.p2.directions[RIGHT] = true)); // from Ken to Retsu
    let out = s.step(with(|i| i.p2.buttons[0] = true));
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, TournamentFighter::Retsu)));
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    assert_eq!(rl(&s, P2_STRUCT + 0x12), RETSU_TABLE);
    // His Kiai Shot from the right side: down, down-forward (left),
    // forward (left) and a punch.
    let mut kiai = vec![with(|i| i.p2.directions[DOWN] = true); 3];
    kiai.extend(vec![with(|i| { i.p2.directions[DOWN] = true; i.p2.directions[LEFT] = true; }); 3]);
    kiai.extend(vec![with(|i| i.p2.directions[LEFT] = true); 2]);
    kiai.extend(vec![with(|i| { i.p2.directions[LEFT] = true; i.p2.buttons[0] = true; }); 2]);
    let mut came_out = false;
    for input in kiai.into_iter().chain(vec![idle(); 30]) {
        s.step(input);
        came_out |= p2(&s).move_id == 0x1a;
    }
    assert!(came_out, "the Kiai Shot from P2's side");
    let mut frames = 0u32;
    while s.screen() != "after match" {
        let f = frames;
        s.step(with(|i| {
            let phase = f % 160;
            i.p2.directions[LEFT] = phase < 40;
            i.p2.directions[DOWN] = (60..80).contains(&phase);
            i.p2.directions[UP] = (100..104).contains(&phase);
            if phase % 12 < 2 {
                i.p2.buttons[((f / 12) % 6) as usize] = true;
            }
            if (f + 6) % 14 < 2 {
                i.p1.buttons[((f / 14) % 6) as usize] = true;
            }
        }));
        frames += 1;
        assert!(frames < 30_000, "the match ends");
        assert!(s.fight_machine().unwrap().unrecovered().is_empty(), "frame {frames}: {:05x?}", s.fight_machine().unwrap().unrecovered());
    }
    println!("Ryu v Retsu: {frames} frames to the after-match box");
}


/// P1's score: the 8 digits at player +0x22.
fn p1_score(s: &GameSession) -> u32 {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (P1_STRUCT + 0x22 - 0xff8000) as usize;
    w[a..a + 8].iter().fold(0, |n, &d| n * 10 + d as u32)
}

#[test]
fn his_1987_attacks_score_as_ryus_do() {
    let Some(l) = local_roms() else { return };
    let mut s = retsu_local(&l);
    for _ in 0..150 {
        s.step(hold(&[RIGHT]));
    }
    // His jab (move 0x0d) scores as Ryu's light punch (0x45b6e: 100).
    let (score, health) = (p1_score(&s), p2(&s).health);
    for input in [press(&[], 0), press(&[], 0)].into_iter().chain(vec![idle(); 30]) {
        s.step(input);
    }
    assert!(p2(&s).health < health, "the jab landed");
    assert_eq!(p1_score(&s) - score, 100, "scored as Ryu's light punch");
    // His high kick (0x16) as Ryu's heavy kick (0x46240: 1,500).
    for _ in 0..60 {
        s.step(hold(&[RIGHT]));
    }
    let (score, health) = (p1_score(&s), p2(&s).health);
    for input in [press(&[], 5), press(&[], 5)].into_iter().chain(vec![idle(); 40]) {
        s.step(input);
    }
    assert!(p2(&s).health < health, "the high kick landed");
    assert_eq!(p1_score(&s) - score, 1500, "scored as Ryu's heavy kick");
}

#[test]
fn the_overlays_frame_meter_reads_his_bank_moves() {
    let Some(l) = local_roms() else { return };
    let mut s = retsu_local(&l);
    // The diagnostic overlay reads his table and streams from the bank as
    // the bus does: the frame his crouching punch starts is a move start,
    // and its first active frame is seen as one.
    let inputs = [hold(&[DOWN]), press(&[DOWN], 0), press(&[DOWN], 0)];
    let (mut started, mut active) = (None, None);
    for n in 0..60 {
        s.step(inputs.get(n).copied().unwrap_or(hold(&[DOWN])));
        let t = s.fight_machine().unwrap().diagnostic_telemetry();
        if t.p1.move_id == 0x11 && t.p1.move_started_this_frame {
            started.get_or_insert(n);
        }
        if t.p1.move_id == 0x11 && t.p1.attack_active {
            active.get_or_insert(n);
        }
    }
    let started = started.expect("the overlay sees the move start");
    assert_eq!(active.expect("and its active frames") - started, 12, "12 frames of startup, as its data");
}

#[test]
fn in_a_retsu_mirror_his_bank_poses_wear_the_mirror_palette_and_the_blast_its_own() {
    let Some(l) = local_roms() else { return };
    let mut s = session(&l, CLASSIC, true);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, hold(&[DOWN]));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Multi Player
    tap(&mut s, with(|i| i.menu.confirm = true)); // Local
    tap(&mut s, hold(&[RIGHT]));
    tap(&mut s, hold(&[RIGHT]));
    tap(&mut s, with(|i| i.menu.confirm = true)); // P1 Retsu
    tap(&mut s, with(|i| i.p2.directions[RIGHT] = true)); // P2 from Ken to Retsu
    let out = s.step(with(|i| i.p2.buttons[0] = true));
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Retsu, TournamentFighter::Retsu)));
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    assert_eq!(rl(&s, P2_STRUCT + 0x12), RETSU_TABLE);
    // P2 (facing left) crouches and punches: his new crouching punch's bank
    // tiles in the mirror slot, P1's in his own palette 3.
    let p2_down = |b: bool| with(|i| {
        i.p2.directions[DOWN] = true;
        i.p2.buttons[0] = b;
    });
    let mut checked = false;
    for n in 0..60 {
        s.step(p2_down((1..3).contains(&n)));
        if p2(&s).move_id == 0x11 && p2(&s).attack_active {
            let o = &s.fight_machine().unwrap().state.objectram;
            let w = s.fight_machine().unwrap().work_ram();
            let region = |ps: u32| u16::from_be_bytes([w[(ps + 0x10 - 0xff8000) as usize], w[(ps + 0x11 - 0xff8000) as usize]]) as usize;
            let attrs = |e: usize, n: usize| (0..n).map(|k| (u16::from_be_bytes([o[e + 0x40 * k], o[e + 0x40 * k + 1]]), u16::from_be_bytes([o[e + 0x40 * k + 2], o[e + 0x40 * k + 3]]))).collect::<Vec<_>>();
            let p2_entries = attrs(region(P2_STRUCT), 16);
            assert!(p2_entries.iter().all(|&(c, a)| c >= 0x3800 && a & 0xf == 14), "P2's bank pose in the mirror slot: {p2_entries:x?}");
            let p1_entries = attrs(region(P1_STRUCT), 1);
            assert_eq!(p1_entries[0].1 & 0xf, 3, "P1 in his own palette");
            checked = true;
            break;
        }
    }
    assert!(checked, "P2's crouching punch struck");
    for _ in 0..60 {
        s.step(idle());
    }
    // P2's Kiai Shot (facing left: down, down-left, left, punch): the blast
    // keeps his own palette, as every effect does in a mirror.
    let mut kiai = vec![with(|i| i.p2.directions[DOWN] = true); 3];
    kiai.extend(vec![with(|i| { i.p2.directions[DOWN] = true; i.p2.directions[LEFT] = true; }); 3]);
    kiai.extend(vec![with(|i| i.p2.directions[LEFT] = true); 2]);
    kiai.extend(vec![with(|i| { i.p2.directions[LEFT] = true; i.p2.buttons[0] = true; }); 2]);
    let mut blast = Vec::new();
    for input in kiai.into_iter().chain(vec![idle(); 30]) {
        s.step(input);
        let o = &s.fight_machine().unwrap().state.objectram;
        if let Some(e) = (0..12usize).map(|k| 0x100 + 0x40 * k).find(|&e| o[e + 8] != 0) {
            blast.push(u16::from_be_bytes([o[e + 2], o[e + 3]]));
        }
    }
    assert!(!blast.is_empty() && blast.iter().all(|&a| a & 0xf == 3 && a & 0x100 != 0), "P2's blast, flipped, in palette 3: {blast:x?}");
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}
