//! Ticket #251 (`SP4`): Geki as a 1987 player fighter, ADR-0014's second
//! boss after Retsu (`retsu_player.rs`, whose pattern this follows).
//!
//! At the headless session's seam, driven by inputs: in Local under Classic
//! Competitive, Geki plays on his 1987 player table in the extended data
//! bank and the translated 1987 player handler runs him: he walks, jumps
//! straight and diagonally, crouches, blocks high and low (a real standing
//! block: his 1987 move 4 is his teleport), has a normal on every button
//! standing, crouching and in the air, the Shuriken comes out of the
//! Hadoken's command tracker and the Teleport Strike out of the Tatsumaki's
//! with a punch (his table's special commands). What is checked is read
//! back through the presented frames, the fight reader and the session's
//! log; each move's startup, active and recovery frames are read from the
//! fight reader and compared with the move's data in the bank (and the ROM,
//! for his 1987 attacks). The boxes the bank writes for his new poses are
//! checked against the drawings' pens, and his ROM attacks keep their ROM
//! boxes. Local ROMs and the local art pack only; each test skips without
//! them.

use std::collections::BTreeSet;
use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::{geki_teleport, in_bank, DataBank, PoseGeom, APEX_POINTER, GEKI_BOXES};
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
const RECIPES: [&str; 2] = [include_str!("../../../art/recipes/ryu.json"), include_str!("../../../art/recipes/geki.json")];
const REVIEWS: [&str; 2] = [include_str!("../../../art/review/ryu.json"), include_str!("../../../art/review/geki.json")];

/// The game flow's fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
/// Classic Competitive.
const CLASSIC: RulesetPreset = RulesetPreset::ClassicCompetitive;
/// Where Geki's slot of the extended data bank starts: his place in
/// `TournamentFighter::ALL` (3) times the slot size.
const GEKI_TABLE: u32 = 0x13_0000;
/// His ROM move table and the ROM frames his kept attacks strike with.
const ROM_TABLE: u32 = 0x16bfc;
const CLAW_FRAME: u32 = 0xb9ae;
const FRONT_KICK_FRAME: u32 = 0xbb9a;
const CROUCHING_CLAW_FRAME: u32 = 0xbaa4;
const DIVE_FRAME: u32 = 0xbcca;
/// His 1987 attack streams (his CPU's), whose poses are his.
const ROM_ATTACKS: [u32; 7] = [0xb968, 0xba38, 0xbb54, 0xbc24, 0xbe5c, 0xbf98, 0xc0f4];

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
    assert!(art.catalogue("geki").is_some(), "Geki's poses load: {:?}", art.kept);
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
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).transition, Some(Transition::Picked(p1)));
    let out = s.step(with(|i| i.p2.buttons[0] = true));
    assert_eq!(out.transition, Some(Transition::Fighters(p1, TournamentFighter::Ken)));
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    s
}

/// Local under Classic Competitive, Geki against Ken, at control.
fn geki_local(l: &Local) -> GameSession {
    local_with(session(l, CLASSIC, true), TournamentFighter::Geki)
}

/// Geki and Ken backed away from each other, so nothing connects.
fn apart(l: &Local) -> GameSession {
    let mut s = geki_local(l);
    for _ in 0..60 {
        s.step(with(|i| {
            i.p1.directions[LEFT] = true;
            i.p2.directions[RIGHT] = true;
        }));
    }
    for _ in 0..30 {
        s.step(idle());
    }
    s
}

/// Geki walked in to Ken.
fn close(l: &Local) -> GameSession {
    let mut s = geki_local(l);
    for _ in 0..150 {
        s.step(hold(&[RIGHT]));
    }
    for _ in 0..10 {
        s.step(idle());
    }
    s
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

/// A player's fighter entity in object RAM.
fn entity(s: &GameSession, ps: u32) -> usize {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (ps + 0x10 - 0xff8000) as usize;
    u16::from_be_bytes([w[a], w[a + 1]]) as usize
}

fn p1_entity(s: &GameSession) -> usize {
    entity(s, P1_STRUCT)
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

/// A fighter's grid centre on screen (the engine's `center_x`: its grid's
/// left plus half its width).
fn grid_centre_x(s: &GameSession, ps: u32) -> i32 {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = entity(s, ps);
    let cols = o[e + 0xd] as i32 + 1;
    let cell = if u16::from_be_bytes([o[e + 2], o[e + 3]]) & 0x400 != 0 { 32 } else { 16 };
    u16::from_be_bytes([o[e + 6], o[e + 7]]) as i32 + cols * cell / 2
}

/// P1's tile codes on screen: each entry of his grid.
fn p1_codes(s: &GameSession) -> Vec<u16> {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = p1_entity(s);
    let n = (o[e + 0xc] as usize + 1) * (o[e + 0xd] as usize + 1);
    (0..n.min(24)).map(|k| u16::from_be_bytes([o[e + 0x40 * k], o[e + 0x40 * k + 1]])).collect()
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

    fn w(&self, a: u32) -> u16 {
        u16::from_be_bytes([self.b(a), self.b(a + 1)])
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
        let h = table + id as u32 * 0x30;
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

    /// The tile codes of every frame of the stream from `a` and of the
    /// ROM's alternatives (a goto's target, a teleport's other frame groups
    /// as frames on), each record's grid entry by entry.
    fn stream_codes(&self, a: u32, out: &mut BTreeSet<u16>) {
        let mut a = a;
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
                _ => {
                    grid_codes(self.w(a + 2), self.w(a + 4) & 0x400 != 0, self.b(a + 8), self.b(a + 9), out);
                    a += 0x20;
                }
            }
        }
    }
}

/// The tile codes of a grid: `code` plus a cell's step across and a row's
/// down (32-pixel entries step 2 and 0x20, 16-pixel 1 and 0x10).
fn grid_codes(code: u16, large: bool, rows: u8, cols: u8, out: &mut BTreeSet<u16>) {
    let (across, down) = if large { (2, 0x20) } else { (1, 0x10) };
    for r in 0..=rows as u16 {
        for c in 0..=cols as u16 {
            out.insert(code + c * across + r * down);
        }
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

/// Hold down, down-forward, forward and press `button` (facing right).
fn qcf(button: usize) -> Vec<FrameInputs> {
    let mut v = vec![hold(&[DOWN]); 3];
    v.extend(vec![hold(&[DOWN, RIGHT]); 3]);
    v.extend(vec![hold(&[RIGHT]); 2]);
    v.extend([press(&[RIGHT], button), press(&[RIGHT], button)]);
    v
}

/// Hold down, down-back, back and press `button` (facing right).
fn qcb(button: usize) -> Vec<FrameInputs> {
    let mut v = vec![hold(&[DOWN]); 3];
    v.extend(vec![hold(&[DOWN, LEFT]); 3]);
    v.extend(vec![hold(&[LEFT]); 2]);
    v.extend([press(&[LEFT], button), press(&[LEFT], button)]);
    v
}

/// P1's score: the 8 digits at player +0x22.
fn p1_score(s: &GameSession) -> u32 {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (P1_STRUCT + 0x22 - 0xff8000) as usize;
    w[a..a + 8].iter().fold(0, |n, &d| n * 10 + d as u32)
}

/// The points a damage entry's score byte adds (the 7 digits of the score
/// table `0x146d6` at the byte, shown with a trailing 0).
fn points(rom: &[u8], entry: u32) -> u32 {
    let idx = rom[entry as usize] as usize;
    rom[0x146d6 + idx..0x146d6 + idx + 7].iter().fold(0, |n, &d| n * 10 + d as u32) * 10
}

// ---- the tests ---------------------------------------------------------------

#[test]
fn geki_plays_on_his_bank_table_and_the_other_bosses_keep_theirs() {
    let Some(l) = local_roms() else { return };
    let mut s = geki_local(&l);
    assert_eq!(rl(&s, P1_STRUCT + 0x12), GEKI_TABLE, "Geki is bound to his 1987 player table");
    assert_eq!(rl(&s, P2_STRUCT + 0x12), TournamentFighter::Ken.move_table());
    assert_eq!(s.data_bank().unwrap().table(TournamentFighter::Geki), Some(GEKI_TABLE));
    assert_eq!(s.data_bank().unwrap().fighters(), [TournamentFighter::Geki]);
    let log = s.take_log();
    assert!(log.iter().any(|l| l.contains("Geki plays a 1987 player table")), "{log:?}");
    // The player handler runs him: a crouch held is Ryu's kind of still
    // posture (no busy bit), where his boss-only crouch stood him up.
    for _ in 0..90 {
        s.step(hold(&[DOWN]));
        assert_eq!((p1(&s).move_id, p1(&s).state), (0x05, 0x14));
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Retsu, with no table in this bank (his poses are not loaded), is
    // greyed out on the select screen; without the bank Geki is too.
    let select = sf_engine::front::select::SelectScreen::local(&MatchConfig::from_preset(CLASSIC), &[TournamentFighter::Geki]);
    assert!(select.greyed(2) && !select.greyed(3));
    let select = sf_engine::front::select::SelectScreen::local(&MatchConfig::from_preset(CLASSIC), &[]);
    assert!(select.greyed(3));
}

#[test]
fn he_walks_crouches_and_jumps_straight_and_diagonally() {
    let Some(l) = local_roms() else { return };
    let start = geki_local(&l);
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
    assert!(p1_grid(&s).1 + 10 < x1, "walked back");
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
        assert!(top0 - highest >= 40, "{dirs:?}: rose {} pixels", top0 - highest);
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
    let start = geki_local(&l);
    // Ken's standing medium punch lands unguarded and not through his
    // standing guard, his new block pose (not his 1987 teleport, move 4).
    let (_, before, after) = guard_against(&start, (false, 1), &[]);
    assert!(after < before, "an unguarded punch lands");
    let (moves, before, after) = guard_against(&start, (false, 1), &[LEFT]);
    assert!(moves.contains(&0x04), "his standing guard: {moves:x?}");
    assert_eq!(after, before, "no damage through the standing guard");
    // Ken's crouching kick, guarded crouching.
    let (moves, before, after) = guard_against(&start, (true, 3), &[DOWN, LEFT]);
    assert!(moves.contains(&0x08), "his crouching guard: {moves:x?}");
    assert_eq!(after, before, "no damage through the crouching guard");
    let (_, before, after) = guard_against(&start, (true, 3), &[]);
    assert!(after < before, "an unguarded kick lands");
}

/// Geki walked in to Ken, then Ken jumps straight up and kicks hard on the
/// way down, again and again, while Geki holds `guard`: the moves Geki
/// showed, his health before and after, and how often Ken was in his jump
/// attack.
fn jump_in(start: &GameSession, guard: &[usize]) -> (Vec<u8>, u16, u16, u32) {
    let mut s = start.clone();
    for _ in 0..150 {
        s.step(hold(&[RIGHT]));
    }
    let health = p1(&s).health;
    let mut moves = Vec::new();
    let mut ken_jump_attacks = 0;
    for f in 0..300u32 {
        let mut i = hold(guard);
        let phase = f % 60;
        i.p2.directions[UP] = phase < 2;
        i.p2.buttons[5] = (14..16).contains(&phase);
        s.step(i);
        if (0x20..=0x26).contains(&p2(&s).move_id) {
            ken_jump_attacks += 1;
        }
        let m = p1(&s).move_id;
        if !moves.contains(&m) {
            moves.push(m);
        }
    }
    (moves, health, p1(&s).health, ken_jump_attacks)
}

#[test]
fn he_blocks_a_jump_in_standing_and_not_crouching() {
    let Some(l) = local_roms() else { return };
    let start = geki_local(&l);
    // Unguarded, Ken's jumping heavy kick lands.
    let (_, before, after, attacks) = jump_in(&start, &[]);
    assert!(attacks > 0, "Ken attacked from the air");
    assert!(after < before, "an unguarded jump-in lands");
    // His standing guard takes it.
    let (moves, before, after, _) = jump_in(&start, &[LEFT]);
    assert!(moves.contains(&0x04), "his standing guard: {moves:x?}");
    assert_eq!(after, before, "no damage through the standing guard");
    // Under standard high and low blocking a jump attack is an overhead:
    // his crouching guard does not take it.
    assert!(MatchConfig::from_preset(CLASSIC).toggles.standard_high_low_block);
    let (_, before, after, _) = jump_in(&start, &[DOWN, LEFT]);
    assert!(after < before, "a jump-in lands on the crouching guard");
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
    let start = apart(&l);
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
                data.jump_frames(GEKI_TABLE, jump, id >= 0x23, level)
            }
            _ => data.move_frames(GEKI_TABLE, id),
        };
        let (su, ac, re) = phases(&measured);
        println!("{name:20} move {id:#04x}: startup {su:2}, active {ac:2}, recovery {re:2}");
        assert!(ac > 0, "{name}: an attack");
        assert_eq!(measured, expected, "{name}: the frames as the fight reader reads them are the move's data");
        assert!(s.fight_machine().unwrap().unrecovered().is_empty(), "{name}");
    }
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

/// The first effect object's tile code, if any is live.
fn effect_code(s: &GameSession) -> Option<u16> {
    let o = &s.fight_machine().unwrap().state.objectram;
    (0..12usize).map(|k| 0x100 + 0x40 * k).find(|&e| o[e + 8] != 0).map(|e| u16::from_be_bytes([o[e], o[e + 1]]))
}

#[test]
fn the_shuriken_comes_from_its_command_its_star_by_the_buttons_strength() {
    let Some(l) = local_roms() else { return };
    let far = apart(&l);
    let bank = far.data_bank().unwrap().clone();
    let data = Data { rom: &l.a.maincpu, bank: &bank };
    // A step in from the round's start: about 108 pixels apart.
    let mut mid = geki_local(&l);
    for _ in 0..10 {
        mid.step(hold(&[RIGHT]));
    }
    for _ in 0..10 {
        mid.step(idle());
    }

    // From far off, each strength: the move's frames are its data, a star
    // (his ROM shuriken, tile 0x0e6e/0x0e6f) leaves on the hop's throw, the
    // light one falls steepest, the heavy one flattest.
    let mut drops = Vec::new();
    for button in 0..3 {
        let mut s = far.clone();
        let mut frames = Vec::new();
        let mut started = None;
        let mut path = Vec::new();
        for (n, input) in qcf(button).into_iter().chain(vec![idle(); 90]).enumerate() {
            s.step(input);
            let r = p1(&s);
            if r.move_id == 0x1a && r.state & 0x80 != 0 {
                started.get_or_insert(n);
                frames.push(r.attack_active);
            }
            // His star in flight (where it hits it becomes its ROM hit
            // effect, `0x18944`).
            if let (Some(&(x, y)), Some(0x0e6e | 0x0e6f)) = (effects(&s).first(), effect_code(&s)) {
                path.push((n, x as i32, y as i32));
            }
        }
        assert_eq!(frames, data.move_frames(GEKI_TABLE, 0x1a), "button {button}: its frames are its data");
        let started = started.expect("the Shuriken came out");
        let (first, x0, y0) = path[0];
        println!("button {button}: the star is out on the move's frame {}", first - started + 1);
        assert_eq!(first - started + 1, 30, "out on the hop's throw, as the ROM throws it");
        let (_, x1, y1) = path[12.min(path.len() - 1)];
        assert!(x1 > x0 + 30, "it flies at Ken: {x0} -> {x1}");
        drops.push(y1 - y0);
        assert!(s.fight_machine().unwrap().unrecovered().is_empty());
    }
    println!("drops over 12 frames, light to heavy: {drops:?}");
    assert!(drops[0] > drops[1] && drops[1] > drops[2], "the light star falls steepest: {drops:?}");
    // At mid range each reaches Ken standing there for his ROM damage, 4.
    for button in 0..3 {
        let mut s = mid.clone();
        let health = p2(&s).health;
        for input in qcf(button).into_iter().chain(vec![idle(); 80]) {
            s.step(input);
        }
        assert_eq!(health - p2(&s).health, 4, "button {button}: the star reaches Ken at mid range");
    }

    // Close in, the light star hits Ken and scores as Ryu's Hadoken; the
    // flatter medium one flies over him.
    let mut s = close(&l);
    let (health, score) = (p2(&s).health, p1_score(&s));
    for input in qcf(0).into_iter().chain(vec![idle(); 80]) {
        s.step(input);
    }
    assert_eq!(health - p2(&s).health, 4, "the star's damage");
    assert_eq!(p1_score(&s) - score, points(&l.a.maincpu, 0x4345e), "scored as the Hadoken");
    let mut s = close(&l);
    let health = p2(&s).health;
    for input in qcf(1).into_iter().chain(vec![idle(); 80]) {
        s.step(input);
    }
    assert_eq!(p2(&s).health, health, "the medium star passes over Ken up close");
    // Far off, the light star falls short of him.
    let mut s = far.clone();
    let health = p2(&s).health;
    for input in qcf(0).into_iter().chain(vec![idle(); 80]) {
        s.step(input);
    }
    assert_eq!(p2(&s).health, health, "the light star falls short far off");

    // The Shoryuken's command brings out nothing he lacks: the punch it
    // ends on, his crouching claw.
    let mut s = far.clone();
    let mut dp = vec![hold(&[RIGHT]); 3];
    dp.extend(vec![hold(&[DOWN]); 3]);
    dp.extend([press(&[DOWN, RIGHT], 0), press(&[DOWN, RIGHT], 0)]);
    assert_eq!(measure_then(&mut s, &dp, hold(&[DOWN, RIGHT]), 0x12), data.move_frames(GEKI_TABLE, 0x12));
}

#[test]
fn the_teleport_strike_reappears_in_front_of_ken_and_claws_him() {
    let Some(l) = local_roms() else { return };
    let far = apart(&l);
    let bank = far.data_bank().unwrap().clone();
    let data = Data { rom: &l.a.maincpu, bank: &bank };

    // From range: he vanishes in his smoke, unhittable there, reappears
    // `GAP` in front of Ken and his claw strikes. The claw lands on its
    // first active frame (a hit clears the active bit at once, so the fight
    // reader sees none): Ken is hit on the move's frame after its startup,
    // and the move lasts its data's frames.
    let expected = data.move_frames(GEKI_TABLE, 0x1b);
    let (su, ac, re) = phases(&expected);
    assert_eq!((su, ac, re), (30, 13, 14), "its numbers (`teleport`)");
    let mut s = far.clone();
    let (health, score) = (p2(&s).health, p1_score(&s));
    let gap0 = grid_centre_x(&s, P2_STRUCT) - grid_centre_x(&s, P1_STRUCT);
    let mut frames = Vec::new();
    let mut unhittable = Vec::new();
    let mut gaps = Vec::new();
    let mut hit_on = None;
    for input in qcb(0).into_iter().chain(vec![idle(); 90]) {
        let before = p2(&s).health;
        s.step(input);
        let r = p1(&s);
        if r.move_id == 0x1b && r.state & 0x80 != 0 {
            frames.push(r.attack_active);
            unhittable.push(p1_bytes(&s, 0xf, 1)[0] & 0x08 != 0);
            gaps.push(grid_centre_x(&s, P2_STRUCT) - grid_centre_x(&s, P1_STRUCT));
            if p2(&s).health < before {
                hit_on.get_or_insert(frames.len());
            }
        }
    }
    println!("Teleport Strike from {gap0} pixels: startup {su}, active {ac}, recovery {re}; gaps {gaps:?}");
    assert_eq!(frames.len(), expected.len(), "the move lasts its data's frames");
    assert_eq!(hit_on, Some(su + 1), "the claw lands on its first active frame");
    let smoke: Vec<usize> = unhittable.iter().enumerate().filter(|(_, &u)| u).map(|(i, _)| i + 1).collect();
    assert_eq!(smoke, (5..=26).collect::<Vec<_>>(), "unhittable in his smoke, frames 5 to 26");
    assert!(gap0 > 150, "he started far off: {gap0}");
    assert_eq!(gaps[20], geki_teleport::GAP as i32, "reappearing {} pixels in front of Ken", geki_teleport::GAP);
    assert!(health - p2(&s).health == 6 || health - p2(&s).health == 10, "the claw's ROM damage: {}", health - p2(&s).health);
    assert_eq!(p1_score(&s) - score, points(&l.a.maincpu, 0x46012), "scored as the Tatsumaki");
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Ken holding back blocks it.
    let mut s = far.clone();
    let health = p2(&s).health;
    let mut inputs = qcb(0);
    inputs.extend(vec![idle(); 90]);
    for mut input in inputs {
        input.p2.directions[RIGHT] = true;
        s.step(input);
    }
    assert_eq!(p2(&s).health, health, "guarded");

    // The Tatsumaki's command with a kick is no special of his: the back
    // kick it ends on, his light kick.
    let mut s = far.clone();
    assert_eq!(measure(&mut s, &qcb(3), 0x14), data.move_frames(GEKI_TABLE, 0x14));
}

#[test]
fn both_specials_come_out_under_pure_footsies_strict_1987_commands() {
    let Some(l) = local_roms() else { return };
    // Pure Footsies: 1987-strict commands (no lenient trackers), so each
    // special fires only from its command's last direction, held as the
    // button goes: forward for the Shuriken, back for the Teleport Strike.
    let start = local_with(session(&l, RulesetPreset::PureFootsies1987Plus, true), TournamentFighter::Geki);
    assert!(!start.fight_machine().unwrap().match_config().toggles.lenient_command_trackers);
    assert_eq!(rl(&start, P1_STRUCT + 0x12), GEKI_TABLE);
    let data_bank = start.data_bank().unwrap().clone();
    let data = Data { rom: &l.a.maincpu, bank: &data_bank };
    let mut s = start.clone();
    let frames = measure_then(&mut s, &qcf(0), hold(&[RIGHT]), 0x1a);
    assert_eq!(frames, data.move_frames(GEKI_TABLE, 0x1a), "the Shuriken");
    let mut s = start.clone();
    let frames = measure_then(&mut s, &qcb(0), hold(&[LEFT]), 0x1b);
    assert_eq!(frames.len(), data.move_frames(GEKI_TABLE, 0x1b).len(), "the Teleport Strike");
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
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
    let p = l.art.catalogue("geki").unwrap().pose(name).unwrap();
    PoseGeom { code: p.grid.code, attr: if p.grid.large { 0x402 } else { 2 }, rows: p.grid.rows, cols: p.grid.cols, anchor: p.anchor }
}

#[test]
fn every_box_the_bank_writes_matches_the_drawn_pose() {
    let Some(l) = local_roms() else { return };
    for (name, boxes) in GEKI_BOXES {
        let pens = &l.art.built.iter().find(|(f, n, _)| f == "geki" && n == name).unwrap_or_else(|| panic!("{name} drawn")).2;
        // A kick held straight out is not one rectangle of body: his light
        // kick's and diagonal jump kick's body boxes hold his body, the leg
        // he stands or tucks on and his thigh, the attack his shin and foot,
        // and some of the drawn leg between is in neither (a 1987 box limit,
        // reported to the maintainer). The light kick's push box reaches the grid's
        // centre, which is out along his leg.
        let (holds, push) = match name {
            "light_kick" => (0.7, 0.45),
            "jump_kick_down" => (0.7, 0.6),
            _ => (0.8, 0.6),
        };
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
        assert!(body as f64 >= holds * all as f64, "{name}: the body box holds {body} of {all} drawn pixels");
        assert!(drawn(pens, boxes.push) >= push, "{name}: push {:.2}", drawn(pens, boxes.push));
        if let Some(a) = boxes.attack {
            assert!(drawn(pens, a) >= 0.5, "{name}: attack {:.2}", drawn(pens, a));
            let front = (a[1]..a[3]).filter_map(|y| (0..pens.width).rev().find(|&x| pens.at(x, y as usize) != TRANSPARENT)).max().unwrap();
            assert!((a[0]..a[2]).contains(&(front as i32)), "{name}: the limb's tip at x {front} is in the attack box");
        }
        let (hx, hy) = geom(&l, name).half();
        assert!(boxes.push[0] <= hx && hx <= boxes.push[2] && boxes.push[1] <= hy && hy <= boxes.push[3], "{name}: push box about the centre");
    }
}

/// The ROM record's sixteen box bytes, as the entity holds them (facing
/// right).
fn rom_boxes(rom: &[u8], frame: u32) -> Vec<u8> {
    rom[frame as usize + 0xa..frame as usize + 0x1a].to_vec()
}

#[test]
fn a_strike_loads_its_own_tiles_and_boxes_new_poses_theirs_and_rom_attacks_the_roms() {
    let Some(l) = local_roms() else { return };
    let start = apart(&l);
    let rom = &l.a.maincpu;
    let boxes = |name: &str| GEKI_BOXES.iter().find(|(n, _)| *n == name).unwrap().1;
    let jump = |dirs: &[usize], b: usize| [vec![hold(dirs); 2], vec![idle(); 6], vec![press(&[], b); 2]].concat();
    let crouching = |b: usize| vec![hold(&[DOWN]), press(&[DOWN], b), press(&[DOWN], b)];
    let standing = |b: usize| vec![press(&[], b), press(&[], b)];
    // (what, inputs, held after, move, the tiles and boxes it strikes with)
    enum Want {
        Pose(&'static str),
        Rom(u32),
    }
    for (what, inputs, after, id, want) in [
        ("jab", standing(0), idle(), 0x0d, Want::Pose("jab_2")),
        ("light kick", standing(3), idle(), 0x14, Want::Pose("light_kick")),
        ("crouching kick", crouching(3), hold(&[DOWN]), 0x17, Want::Pose("crouching_kick")),
        ("sweep", crouching(5), hold(&[DOWN]), 0x18, Want::Pose("sweep")),
        ("jump punch", jump(&[UP], 0), idle(), 0x20, Want::Pose("jump_punch")),
        ("jump kick", jump(&[UP], 3), idle(), 0x23, Want::Pose("jump_kick")),
        ("jump heavy kick", jump(&[UP], 5), idle(), 0x26, Want::Pose("jump_kick_high")),
        ("diagonal jump kick", jump(&[UP, RIGHT], 3), idle(), 0x24, Want::Pose("jump_kick_down")),
        ("claw thrust", standing(1), idle(), 0x0f, Want::Rom(CLAW_FRAME)),
        ("front kick", standing(4), idle(), 0x15, Want::Rom(FRONT_KICK_FRAME)),
        ("crouching claw", crouching(1), hold(&[DOWN]), 0x1c, Want::Rom(CROUCHING_CLAW_FRAME)),
        ("diving claw", jump(&[UP, RIGHT], 0), idle(), 0x21, Want::Rom(DIVE_FRAME)),
        ("teleport strike", qcb(0), idle(), 0x1b, Want::Rom(CLAW_FRAME)),
    ] {
        let mut s = start.clone();
        let mut checked = false;
        for n in 0..200 {
            let health = p2(&s).health;
            s.step(inputs.get(n).copied().unwrap_or(after));
            // The Teleport Strike reappears at Ken and lands on its first
            // active frame, whose hit clears the active bit at once.
            let landed = id == 0x1b && p2(&s).health < health;
            if p1(&s).move_id == id && (p1(&s).attack_active || landed) {
                let code = u16::from_be_bytes(p1_bytes(&s, 0, 2).try_into().unwrap());
                match want {
                    Want::Pose(name) => {
                        let g = geom(&l, name);
                        // Facing right the first entry is the grid's own first tile.
                        assert_eq!(code, g.code, "{what}: its own tiles from the extended sprite bank");
                        assert!(code >= 0x3800, "{what}: a bank tile");
                        assert_eq!(p1_bytes(&s, 0x16, 16), boxes(name).bytes(&g), "{what}: the boxes written for its drawing");
                    }
                    Want::Rom(frame) => {
                        assert_eq!(code, u16::from_be_bytes([rom[frame as usize + 2], rom[frame as usize + 3]]), "{what}: its ROM tiles");
                        assert_eq!(p1_bytes(&s, 0x16, 16), rom_boxes(rom, frame), "{what}: its ROM boxes");
                    }
                }
                checked = true;
                break;
            }
        }
        assert!(checked, "{what} struck");
    }
    // His standing block: its new pose and the boxes written for it.
    let mut s = geki_local(&l);
    for _ in 0..120 {
        s.step(hold(&[RIGHT]));
    }
    let g = geom(&l, "standing_block");
    let mut blocked = false;
    for f in 0..120u32 {
        s.step(with(|i| {
            i.p1.directions[LEFT] = true;
            i.p2.buttons[1] = f % 40 < 2;
        }));
        if p1(&s).move_id == 0x04 {
            assert_eq!(u16::from_be_bytes(p1_bytes(&s, 0, 2).try_into().unwrap()), g.code, "his block's own tiles");
            assert_eq!(p1_bytes(&s, 0x16, 16), boxes("standing_block").bytes(&g), "his block's boxes");
            blocked = true;
            break;
        }
    }
    assert!(blocked, "he blocked");
}

#[test]
fn only_his_own_poses_are_drawn() {
    let Some(l) = local_roms() else { return };
    let start = apart(&l);
    let bank = start.data_bank().unwrap().clone();
    let data = Data { rom: &l.a.maincpu, bank: &bank };
    // His: every catalogue pose's grid, and every frame of his ROM table's
    // streams and of his CPU's attack streams.
    let mut his = BTreeSet::new();
    for p in &l.art.catalogue("geki").unwrap().poses {
        grid_codes(p.grid.code, p.grid.large, p.grid.rows, p.grid.cols, &mut his);
    }
    for id in 0..13u32 {
        let h = ROM_TABLE + id * 0x30;
        if data.w(h) == 0 {
            continue; // an unused slot (his crouches forward and back)
        }
        grid_codes(data.w(h), data.w(h + 2) & 0x400 != 0, data.b(h + 0xc), data.b(h + 0xd), &mut his);
        if data.l(h + 0x2c) != 0 {
            data.stream_codes(data.l(h + 0x2c), &mut his);
        }
    }
    for stream in ROM_ATTACKS {
        data.stream_codes(stream, &mut his);
    }
    // His teleport's other frame groups (0x45a34's choices) and the smoke
    // between: the frames after its goto.
    data.stream_codes(0xb75a, &mut his);
    data.stream_codes(0xb780, &mut his);
    // His hit and knock-out reactions (his 1987 tables, `reaction_tables`:
    // 13 entries of a 16-bit stream address and two bytes).
    let (hits, knock_outs) = TournamentFighter::Geki.reaction_tables().unwrap();
    for table in [hits, knock_outs] {
        for k in 0..13 {
            data.stream_codes(data.w(table + 4 * k) as u32, &mut his);
        }
    }
    // Ryu's and Ken's: none of theirs is his.
    let mut theirs = BTreeSet::new();
    for table in [0x1605cu32, 0x1662c] {
        for id in 0..0x1f {
            let h = table + id * 0x30;
            if data.b(h + 8) != 0 {
                grid_codes(data.w(h), data.w(h + 2) & 0x400 != 0, data.b(h + 0xc), data.b(h + 0xd), &mut theirs);
                if data.l(h + 0x2c) != 0 {
                    data.stream_codes(data.l(h + 0x2c), &mut theirs);
                }
            }
        }
    }
    let shared: Vec<String> = his.intersection(&theirs).map(|c| format!("{c:#06x}")).collect();
    assert!(shared.is_empty(), "his ROM poses share no tile with Ryu's or Ken's: {shared:?}");
    // Every move of his, standing, crouching, jumping, blocking and both
    // specials: each tile he shows is his.
    let mut scripts: Vec<(Vec<FrameInputs>, FrameInputs)> = normals().into_iter().map(|(_, inputs, after, _, _)| (inputs, after)).collect();
    scripts.push((qcf(0), idle()));
    scripts.push((qcb(0), idle()));
    scripts.push((vec![hold(&[RIGHT]); 30], idle()));
    scripts.push((vec![hold(&[LEFT]); 30], idle()));
    scripts.push((vec![hold(&[DOWN]); 30], idle()));
    scripts.push((vec![hold(&[UP, RIGHT]); 2], idle()));
    for (inputs, after) in scripts {
        let mut s = start.clone();
        for n in 0..120 {
            s.step(inputs.get(n).copied().unwrap_or(after));
            for code in p1_codes(&s) {
                assert!(his.contains(&code), "a tile not his: {code:#06x} in move {:#04x}", p1(&s).move_id);
            }
        }
    }
    let mut s = geki_local(&l);
    for f in 0..240u32 {
        s.step(with(|i| {
            i.p1.directions[RIGHT] = f < 120;
            i.p1.directions[LEFT] = f >= 120;
            i.p1.directions[DOWN] = f >= 180;
            i.p2.buttons[(f / 40 % 6) as usize] = f % 40 < 2;
        }));
        for code in p1_codes(&s) {
            assert!(his.contains(&code), "a tile not his: {code:#06x} in move {:#04x}", p1(&s).move_id);
        }
    }
}

#[test]
fn the_fight_replays_identically_from_the_same_inputs() {
    let Some(l) = local_roms() else { return };
    let start = geki_local(&l);
    // His specials first (facing right), then both sides at random.
    let script: Vec<FrameInputs> = qcb(1)
        .into_iter()
        .chain(vec![idle(); 60])
        .chain(qcf(2))
        .chain(vec![idle(); 60])
        .chain((0..600u32).map(|f| {
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
        }))
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
    assert!(first.iter().any(|(_, f)| (0x0d..=0x2f).contains(&f[0].move_id)), "Geki attacked");
    assert!(first.iter().any(|(_, f)| f[0].move_id == 0x1b), "and teleported");
    assert_eq!(a.fight_machine().unwrap().save_snapshot(), b.fight_machine().unwrap().save_snapshot());
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

/// Choose `choice` for the list's row `row` (its specials first).
fn choose(s: &mut GameSession, row: usize, choice: Choice) {
    for _ in 0..row {
        tap(s, hold(&[DOWN]));
    }
    tap(s, with(|i| i.menu.confirm = true));
    if choice == Choice::Try {
        tap(s, hold(&[RIGHT]));
    }
    assert_eq!(s.move_list().unwrap().choosing(), Some(choice));
    let out = s.step(with(|i| i.menu.confirm = true));
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
    s.step(idle());
}

/// Training against the dummy under Classic Competitive, Geki as P1, with
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
    for _ in 0..3 {
        tap(&mut s, hold(&[RIGHT]));
    }
    tap(&mut s, with(|i| i.menu.confirm = true)); // Geki
    s.step(with(|i| i.menu.confirm = true)); // the opponent under the cursor
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_control(&mut s);
    let table = if tables { GEKI_TABLE } else { TournamentFighter::Geki.move_table() };
    assert_eq!(rl(&s, P1_STRUCT + 0x12), table);
    s
}

#[test]
fn moves_lists_his_specials_and_show_and_try_work_for_each() {
    let Some(l) = local_roms() else { return };
    for preset in [CLASSIC, RulesetPreset::DefinitiveOverhaul, RulesetPreset::PureFootsies1987Plus] {
        let names: Vec<_> = move_list(TournamentFighter::Geki, preset).iter().map(|e| e.name).collect();
        assert_eq!(names, ["SHURIKEN", "TELEPORT STRIKE"], "{preset:?}");
    }
    assert!(move_list(TournamentFighter::Geki, RulesetPreset::Faithful1987).is_empty());
    // Without his table the fight has neither, and MOVES lists neither.
    let text = sf_engine::front::moves::MovesPanel::open_in_fight(TournamentFighter::Geki, CLASSIC, false).text();
    assert!(!text.iter().any(|line| line.contains("SHURIKEN") || line.contains("TELEPORT")), "{text:?}");
    let start = training(&l, true);
    let mut s = start.clone();
    open_moves(&mut s);
    let text = s.move_list().expect("the list").text();
    assert!(text.iter().any(|line| line.contains("SHURIKEN")), "{text:?}");
    assert!(text.iter().any(|line| line.contains("TELEPORT STRIKE")), "{text:?}");
    for (row, entry) in move_list(TournamentFighter::Geki, CLASSIC).into_iter().enumerate() {
        // SHOW: his own fighter performs it.
        let mut s = start.clone();
        open_moves(&mut s);
        choose(&mut s, row, Choice::Show);
        let mut came_out = false;
        for _ in 0..400 {
            s.step(idle());
            came_out |= p1(&s).move_id == entry.produces;
        }
        assert!(came_out, "the show brought {} out", entry.name);
        // TRY: the coach calls out the player's own.
        let mut s = start.clone();
        open_moves(&mut s);
        choose(&mut s, row, Choice::Try);
        for p in entry.input.frames(true, Strength::Medium).into_iter().chain(vec![Default::default(); 120]) {
            s.step(with(|i| i.p1 = p));
        }
        let coach = s.coach().expect("the coach");
        assert_eq!(coach.entry().name, entry.name);
        assert_eq!((coach.successes(), coach.outcome()), (1, Some(Outcome::Success)), "{}", entry.name);
    }
}

#[test]
fn a_whole_match_with_geki_on_either_side_runs_clean() {
    let Some(l) = local_roms() else { return };
    // Geki on P1 against Ken, both sides walking in and cycling buttons,
    // crouching, jumping and throwing his specials, until the after-match
    // box.
    let mut s = geki_local(&l);
    let mut frames = 0u32;
    let specials: Vec<FrameInputs> = [qcf(0), qcb(1)].concat();
    while s.screen() != "after match" {
        let f = frames;
        let mut input = with(|i| {
            let phase = f % 200;
            i.p1.directions[RIGHT] = phase < 40;
            i.p1.directions[DOWN] = (60..80).contains(&phase);
            i.p1.directions[UP] = (100..104).contains(&phase);
            if phase < 140 && phase % 12 < 2 {
                i.p1.buttons[((f / 12) % 6) as usize] = true;
            }
            i.p2.directions[LEFT] = (f + 80) % 160 < 40;
            if (f + 6) % 14 < 2 {
                i.p2.buttons[((f / 14 + 3) % 6) as usize] = true;
            }
        });
        if (150..150 + specials.len() as u32).contains(&(f % 200)) {
            input.p1 = specials[(f % 200 - 150) as usize].p1;
        }
        s.step(input);
        frames += 1;
        assert!(frames < 30_000, "the match ends");
        assert!(s.fight_machine().unwrap().unrecovered().is_empty(), "frame {frames}: {:05x?}", s.fight_machine().unwrap().unrecovered());
    }
    assert_eq!(rl(&s, P1_STRUCT + 0x12), GEKI_TABLE);
    println!("Geki v Ken: {frames} frames to the after-match box");

    // Geki on P2 (facing left), against Ryu.
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
    tap(&mut s, with(|i| i.p2.directions[RIGHT] = true)); // to Geki
    let out = s.step(with(|i| i.p2.buttons[0] = true));
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, TournamentFighter::Geki)));
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    assert_eq!(rl(&s, P2_STRUCT + 0x12), GEKI_TABLE);
    // His specials from the right side: the Shuriken (down, down-left,
    // left, punch) and the Teleport Strike (down, down-right, right,
    // punch), mirrored.
    let p2_of = |v: Vec<FrameInputs>| -> Vec<FrameInputs> {
        v.into_iter()
            .map(|i| {
                let mut o = FrameInputs::default();
                o.p2.directions = [i.p1.directions[LEFT], i.p1.directions[RIGHT], i.p1.directions[DOWN], i.p1.directions[UP]];
                o.p2.buttons = i.p1.buttons;
                o
            })
            .collect()
    };
    for (inputs, id) in [(p2_of(qcf(0)), 0x1a), (p2_of(qcb(0)), 0x1b)] {
        let mut came_out = false;
        for input in inputs.into_iter().chain(vec![idle(); 90]) {
            s.step(input);
            came_out |= p2(&s).move_id == id;
        }
        assert!(came_out, "move {id:#04x} from P2's side");
    }
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
    println!("Ryu v Geki: {frames} frames to the after-match box");
}

#[test]
fn his_1987_attacks_score_as_ryus_do() {
    let Some(l) = local_roms() else { return };
    let rom = &l.a.maincpu;
    // (inputs, held after, Ryu's entry it scores as (either depth))
    for (what, inputs, after, entry) in [
        ("claw thrust (MP)", vec![press(&[], 1), press(&[], 1)], idle(), 0x45c88u32),
        ("front kick (MK)", vec![press(&[], 4), press(&[], 4)], idle(), 0x46154),
        ("crouching claw (LP)", vec![hold(&[DOWN]), press(&[DOWN], 0), press(&[DOWN], 0)], hold(&[DOWN]), 0x45de0),
    ] {
        let mut s = close(&l);
        let (score, health) = (p1_score(&s), p2(&s).health);
        for n in 0..60 {
            s.step(inputs.get(n).copied().unwrap_or(after));
        }
        assert!(p2(&s).health < health, "{what} landed");
        let scored = p1_score(&s) - score;
        let (deep, other) = (points(rom, entry), points(rom, entry + 8));
        assert!(scored == deep || scored == other, "{what}: scored {scored}, as Ryu's {deep} or {other}");
    }
}

#[test]
fn the_overlays_frame_meter_reads_his_bank_moves() {
    let Some(l) = local_roms() else { return };
    let mut s = geki_local(&l);
    // The diagnostic overlay reads his table and streams from the bank as
    // the bus does: the frame his crouching kick starts is a move start,
    // and its first active frame is seen as one.
    let inputs = [hold(&[DOWN]), press(&[DOWN], 3), press(&[DOWN], 3)];
    let (mut started, mut active) = (None, None);
    for n in 0..60 {
        s.step(inputs.get(n).copied().unwrap_or(hold(&[DOWN])));
        let t = s.fight_machine().unwrap().diagnostic_telemetry();
        if t.p1.move_id == 0x17 && t.p1.move_started_this_frame {
            started.get_or_insert(n);
        }
        if t.p1.move_id == 0x17 && t.p1.attack_active {
            active.get_or_insert(n);
        }
    }
    let started = started.expect("the overlay sees the move start");
    assert_eq!(active.expect("and its active frames") - started, 6, "6 frames of startup, as its data");
}
