//! Ticket #257 (`SP10`): Eagle as a 1987 player fighter, ADR-0014's eighth
//! boss after Retsu (`retsu_player.rs`), Geki (`geki_player.rs`), Joe
//! (`joe_player.rs`), Mike (`mike_player.rs`), Lee (`lee_player.rs`), Gen
//! (`gen_player.rs`) and Birdie (`birdie_player.rs`), whose pattern this
//! follows.
//!
//! At the headless session's seam, driven by inputs: in Local under Classic
//! Competitive, Eagle plays on his 1987 player table in the extended data
//! bank and the translated 1987 player handler runs him: he walks, jumps
//! straight and diagonally, crouches (his ROM table has no crouch, so his
//! catalogue's crouch and crouching block fill `0x05..=0x08`), blocks high and
//! low, has a normal on every button standing, crouching and in the air, the
//! Manchester Black comes out of the Hadoken's command tracker with a punch
//! and the St. Andrew's Green out of the Tatsumaki's with a kick (his table's
//! special commands). What is checked is read back through the presented
//! frames, the fight reader and the session's log; each move's startup, active
//! and recovery frames are read from the fight reader and compared with the
//! move's data in the bank (and the ROM, for his 1987 attacks). The boxes the
//! bank writes for his new poses are checked against the drawings' pens, and
//! his ROM attacks keep their ROM boxes. Local ROMs and the local art pack
//! only; each test skips without them.

#[path = "support/descending_jump_in.rs"]
mod descending_jump_in;

use std::collections::BTreeSet;
use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::{
    eagle_manchester_black, eagle_st_andrews_green, in_bank, DataBank, PoseGeom, APEX_POINTER,
    EAGLE_BOXES,
};
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
const RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/eagle.json"),
];
const REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/eagle.json"),
];

/// The game flow's fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
/// Classic Competitive.
const CLASSIC: RulesetPreset = RulesetPreset::ClassicCompetitive;
/// Where Eagle's slot of the extended data bank starts: his place in
/// `TournamentFighter::ALL` (9) times the slot size.
const EAGLE_TABLE: u32 = 0x19_0000;
/// His ROM move table and the ROM frames his kept attacks and postures use.
const ROM_TABLE: u32 = 0x17d0c;
const JAB_FRAME: u32 = 0x123be;
const SPIN_STICKS_FRAME: u32 = 0x1242e;
const BARRAGE_FRAME: u32 = 0x124fe;
const LOW_THRUST_FRAME: u32 = 0x1266e;
const FORWARD_LEAP_SMASH_FRAME: u32 = 0x127c4;
const NEUTRAL_LEAP_SMASH_FRAME: u32 = 0x12a88;
/// His 1987 attack streams (his CPU's), whose poses are his.
const ROM_ATTACKS: [u32; 7] = [
    0x1239e, 0x12408, 0x124d8, 0x12628, 0x126f8, 0x1285a, 0x129bc,
];

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
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &RECIPES,
            reviews: Some(&REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    assert!(
        art.catalogue("eagle").is_some(),
        "Eagle's poses load: {:?}",
        art.kept
    );
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
    let at = TournamentFighter::ALL
        .iter()
        .position(|&f| f == p1)
        .unwrap();
    if at >= 6 {
        tap(&mut s, hold(&[DOWN]));
    }
    for _ in 0..at % 6 {
        tap(&mut s, hold(&[RIGHT]));
    }
    assert_eq!(
        tap(&mut s, with(|i| i.menu.confirm = true)).transition,
        Some(Transition::Picked(p1))
    );
    let out = s.step(with(|i| i.p2.buttons[0] = true));
    assert_eq!(
        out.transition,
        Some(Transition::Fighters(p1, TournamentFighter::Ken))
    );
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    s
}

/// Local under Classic Competitive, Eagle against Ken, at control.
fn eagle_local(l: &Local) -> GameSession {
    local_with(session(l, CLASSIC, true), TournamentFighter::Eagle)
}

/// Eagle and Ken backed away from each other, so nothing connects.
fn apart(l: &Local) -> GameSession {
    let mut s = eagle_local(l);
    for _ in 0..35 {
        s.step(with(|i| {
            i.p2.directions[RIGHT] = true;
        }));
    }
    for _ in 0..30 {
        s.step(idle());
    }
    s
}

/// Eagle walked in to Ken.
fn close(l: &Local) -> GameSession {
    let mut s = eagle_local(l);
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
    (
        u16::from_be_bytes([o[e + 4], o[e + 5]]),
        u16::from_be_bytes([o[e + 6], o[e + 7]]),
    )
}

/// A fighter's grid centre x (`0x43be8`): the left edge plus half the width.
fn grid_centre_x(s: &GameSession, ps: u32) -> i32 {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = entity(s, ps);
    let x = u16::from_be_bytes([o[e + 6], o[e + 7]]) as i16 as i32;
    let cols = o[e + 0xd] as i32 + 1;
    let cell = if u16::from_be_bytes([o[e + 2], o[e + 3]]) & 0x400 != 0 {
        32
    } else {
        16
    };
    x + cols * cell / 2
}

/// Bytes of P1's entity from `offset`.
fn p1_bytes(s: &GameSession, offset: usize, len: usize) -> Vec<u8> {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = p1_entity(s);
    o[e + offset..e + offset + len].to_vec()
}

/// All sprite codes P1's live object entries draw this frame.
fn p1_codes(s: &GameSession) -> Vec<u16> {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = p1_entity(s);
    let n = (o[e + 0xc] as usize + 1) * (o[e + 0xd] as usize + 1);
    (0..n.min(24))
        .map(|k| u16::from_be_bytes([o[e + 0x40 * k], o[e + 0x40 * k + 1]]))
        .collect()
}

/// Every sprite code a grid of `(code, large, rows, cols)` draws.
fn grid_codes(code: u16, large: bool, rows: u8, cols: u8, out: &mut BTreeSet<u16>) {
    let (dc, dr) = if large { (2u16, 0x20u16) } else { (1, 0x10) };
    for r in 0..=rows as u16 {
        for c in 0..=cols as u16 {
            out.insert(code + r * dr + c * dc);
        }
    }
}

fn unrecovered(s: &GameSession) -> Vec<u32> {
    s.fight_machine()
        .map(|m| m.unrecovered().to_vec())
        .unwrap_or_default()
}

/// Read the ROM and the extended data bank together, as the 68000 bus does.
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
        (self.w(a) as u32) << 16 | self.w(a + 2) as u32
    }

    /// Walk the move's header and stream from `table` and return its active
    /// flag (`+0xf` bit 7) on each frame it runs, as the engine's
    /// `run_fighter` / `animate` steps through it when nothing connects.
    fn move_frames(&self, table: u32, id: u8) -> Vec<bool> {
        let h = table + id as u32 * 0x30;
        assert_eq!(self.b(h + 8), 1, "move {id:#04x} present");
        let mut out = Vec::new();
        let ticks = (self.b(h + 0xb) as usize + 1) * (self.b(h + 0xe) as usize + 1);
        out.extend(std::iter::repeat_n(self.b(h + 0xf) & 0x80 != 0, ticks));
        let stream = self.l(h + 0x2c);
        if stream != 0 {
            self.stream_frames(stream, &mut out);
        }
        out
    }

    fn stream_frames(&self, mut a: u32, out: &mut Vec<bool>) {
        loop {
            match self.b(a) {
                0xfd => break,
                0xfe => a = self.l(a + 2),
                0xff => {
                    let r = self.l(a + 2);
                    // `0x45630` ends a player's move on the frame it is
                    // met; the other calls run in zero frames and step on.
                    if r == 0x45630 {
                        break;
                    }
                    a += 6;
                }
                hold => {
                    let dur = self.b(a + 1);
                    let active = self.b(a + 0x1a) & 0x80 != 0;
                    let ticks = (hold as usize + 1) * (dur as usize + 1);
                    out.extend(std::iter::repeat_n(active, ticks));
                    a += 0x20;
                }
            }
        }
    }

    /// Every sprite code a stream loads, up to its end (`0xfd` or `0x45630`).
    fn stream_codes(&self, mut a: u32, out: &mut BTreeSet<u16>) {
        for _ in 0..64 {
            match self.b(a) {
                0xfd => break,
                0xfe => a = self.l(a + 2),
                0xff => {
                    if self.l(a + 2) == 0x45630 {
                        break;
                    }
                    a += 6;
                }
                _ => {
                    grid_codes(
                        self.w(a + 2),
                        self.w(a + 4) & 0x400 != 0,
                        self.b(a + 8),
                        self.b(a + 9),
                        out,
                    );
                    a += 0x20;
                }
            }
        }
    }

    /// The frames of a jump attack started at the jump's apex (as the test
    /// presses the button before the apex and `ANYTIME JUMP ATTACKS` is off
    /// under Classic Competitive): the stream the apex table at
    /// `APEX_POINTER` dispatches to for `(jump, kick, level)`.
    fn jump_frames(&self, table: u32, jump: u8, kick: bool, level: u32) -> Vec<bool> {
        let rec = self.l(table + APEX_POINTER);
        let entry = rec + if kick { 0x1c } else { 4 } + (jump as u32 - 9) * 8;
        let stream = self.l(self.l(entry + 4) + (level - 1) * 4);
        let mut out = Vec::new();
        self.stream_frames(stream, &mut out);
        out
    }
}

/// Feed `inputs` then `after` and record `attack_active` on every frame P1
/// is in move `id` (with the busy bit set, so the stance is not counted).
fn measure_then(
    s: &mut GameSession,
    inputs: &[FrameInputs],
    after: FrameInputs,
    id: u8,
) -> Vec<bool> {
    let mut out = Vec::new();
    let mut seen = false;
    for n in 0..200 {
        let i = inputs.get(n).copied().unwrap_or(after);
        s.step(i);
        let r = p1(s);
        if r.move_id == id && r.state & 0x80 != 0 {
            seen = true;
            out.push(r.attack_active);
        } else if seen {
            break;
        }
    }
    assert!(seen, "move {id:#04x} came out (now in {:#04x})", p1(s).move_id);
    out
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

/// Hold down, down-back, back and press `button` (facing right).
fn qcb(button: usize) -> Vec<FrameInputs> {
    let mut v = vec![hold(&[DOWN]); 3];
    v.extend(vec![hold(&[DOWN, LEFT]); 3]);
    v.extend(vec![hold(&[LEFT]); 2]);
    v.extend([press(&[LEFT], button), press(&[LEFT], button)]);
    v
}

/// Hold down, down-forward, forward and press `button` (facing right).
fn qcf(button: usize) -> Vec<FrameInputs> {
    let mut v = vec![hold(&[DOWN]); 3];
    v.extend(vec![hold(&[DOWN, RIGHT]); 3]);
    v.extend(vec![hold(&[RIGHT]); 2]);
    v.extend([press(&[RIGHT], button), press(&[RIGHT], button)]);
    v
}

/// Hold forward, down, down-forward and press `button` (facing right).
fn dp(button: usize) -> Vec<FrameInputs> {
    let mut v = vec![hold(&[RIGHT]); 3];
    v.extend(vec![hold(&[DOWN]); 3]);
    v.extend([press(&[DOWN, RIGHT], button), press(&[DOWN, RIGHT], button)]);
    v
}

/// The first attack (moves `0x0d..=0x2f`) P1 is in while `inputs` and then
/// idle frames are fed.
fn first_attack(start: &GameSession, inputs: &[FrameInputs]) -> u8 {
    let mut s = start.clone();
    for n in 0..60 {
        s.step(inputs.get(n).copied().unwrap_or(idle()));
        let r = p1(&s);
        if (0x0d..=0x2f).contains(&r.move_id) && r.state & 0x80 != 0 {
            return r.move_id;
        }
    }
    panic!("no attack came out");
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
    rom[0x146d6 + idx..0x146d6 + idx + 7]
        .iter()
        .fold(0, |n, &d| n * 10 + d as u32)
        * 10
}

/// The damage of a damage entry (its second byte).
fn damage(rom: &[u8], entry: u32) -> u16 {
    rom[entry as usize + 1] as u16
}

// ---- the tests ---------------------------------------------------------------

#[test]
fn eagle_plays_on_his_bank_table_and_the_other_bosses_keep_theirs() {
    let Some(l) = local_roms() else { return };
    let mut s = eagle_local(&l);
    assert_eq!(
        rl(&s, P1_STRUCT + 0x12),
        EAGLE_TABLE,
        "Eagle is bound to his 1987 player table"
    );
    assert_eq!(
        rl(&s, P2_STRUCT + 0x12),
        TournamentFighter::Ken.move_table()
    );
    assert_eq!(
        s.data_bank().unwrap().table(TournamentFighter::Eagle),
        Some(EAGLE_TABLE)
    );
    assert_eq!(
        s.data_bank().unwrap().fighters(),
        [TournamentFighter::Eagle]
    );
    let log = s.take_log();
    assert!(
        log.iter()
            .any(|l| l.contains("Eagle plays a 1987 player table")),
        "{log:?}"
    );
    // The player handler runs him: a crouch held is Ryu's kind of still
    // posture (no busy bit), where his ROM table had no crouch (`0x05..=0x08`
    // are empty).
    for _ in 0..90 {
        s.step(hold(&[DOWN]));
        assert_eq!((p1(&s).move_id, p1(&s).state), (0x05, 0x14));
    }
    assert!(unrecovered(&s).is_empty(), "{:05x?}", unrecovered(&s));

    // Geki, with no table in this bank (his poses are not loaded), is
    // greyed out on the select screen; without the bank Eagle is too.
    let select = sf_engine::front::select::SelectScreen::local(
        &MatchConfig::from_preset(CLASSIC),
        &[TournamentFighter::Eagle],
    );
    assert!(select.greyed(3) && !select.greyed(9));
    let select = sf_engine::front::select::SelectScreen::local(
        &MatchConfig::from_preset(CLASSIC),
        &[],
    );
    assert!(select.greyed(9));
}

#[test]
fn he_walks_crouches_and_jumps_straight_and_diagonally() {
    let Some(l) = local_roms() else { return };
    let start = eagle_local(&l);
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
    for (dirs, id) in [
        (vec![DOWN], 0x05),
        (vec![DOWN, RIGHT], 0x06),
        (vec![DOWN, LEFT], 0x07),
    ] {
        for _ in 0..20 {
            s.step(hold(&dirs));
        }
        assert_eq!((p1(&s).move_id, p1(&s).state), (id, 0x14), "{dirs:?}");
        assert_eq!(
            p1_grid(&s).0,
            0xa0,
            "his crouch on the ground (72 pixels tall)"
        );
    }
    assert!(unrecovered(&s).is_empty());
    // Each jump leaves the ground, moves the way it was aimed and lands on
    // the ground line in his stance.
    for (dirs, id, way) in [
        (vec![UP], 0x09, 0i32),
        (vec![UP, RIGHT], 0x0a, 1),
        (vec![UP, LEFT], 0x0b, -1),
    ] {
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
        assert!(
            top0 - highest >= 40,
            "{dirs:?}: rose {} pixels",
            top0 - highest
        );
        let moved = x as i32 - x0 as i32;
        match way {
            0 => assert_eq!(moved, 0, "straight up"),
            _ => assert!(moved * way > 40, "{dirs:?}: moved {moved}"),
        }
        assert!(unrecovered(&s).is_empty());
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
    let start = eagle_local(&l);
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
    let (_, before, after) = guard_against(&start, (true, 3), &[]);
    assert!(after < before, "an unguarded kick lands");
    assert!(unrecovered(&start).is_empty());
}

#[test]
fn he_blocks_a_jump_in_standing_and_not_crouching() {
    let Some(mut l) = local_roms() else { return };
    // Geki's descending kick overlaps Eagle's crouch; a high kick passing
    // over it would not exercise whether an overhead defeats a low guard.
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    l.art = load_art_from(
        &l.a,
        &ArtSources {
            recipes: &[
                include_str!("../../../art/recipes/ryu.json"),
                include_str!("../../../art/recipes/geki.json"),
                include_str!("../../../art/recipes/eagle.json"),
            ],
            reviews: Some(&[
                include_str!("../../../art/review/ryu.json"),
                include_str!("../../../art/review/geki.json"),
                include_str!("../../../art/review/eagle.json"),
            ]),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let mut start = session(&l, CLASSIC, true);
    for _ in 0..200 {
        start.step(idle());
    }
    tap(&mut start, with(|i| i.menu.start = true));
    tap(&mut start, hold(&[DOWN]));
    tap(&mut start, with(|i| i.menu.confirm = true)); // Multi Player
    tap(&mut start, with(|i| i.menu.confirm = true)); // Local
    tap(&mut start, hold(&[DOWN]));
    for _ in 0..3 {
        tap(&mut start, hold(&[RIGHT]));
    }
    tap(&mut start, with(|i| i.menu.confirm = true)); // P1 Eagle (row 1, col 3)
    for _ in 0..2 {
        tap(&mut start, with(|i| i.p2.directions[RIGHT] = true));
    }
    assert_eq!(
        start.step(with(|i| i.p2.buttons[0] = true)).transition,
        Some(Transition::Fighters(
            TournamentFighter::Eagle,
            TournamentFighter::Geki
        ))
    );
    start.step(idle());
    until(&mut start, "country select", 40);
    tap(&mut start, with(|i| i.p1.buttons[0] = true));
    until(&mut start, "fight", 40);
    to_control(&mut start);
    // Unguarded, Geki's downward jump kick lands.
    let fixture = descending_jump_in::Setup {
        approach_frames: 0,
        guard_from_takeoff: false,
    };
    let outcome = descending_jump_in::observe(&start, descending_jump_in::Guard::None, fixture);
    assert!(outcome.airborne_attack_frames > 0, "Geki attacked from the air");
    assert!(outcome.health_after < outcome.health_before, "an unguarded jump-in lands");
    // His standing guard takes it.
    let outcome = descending_jump_in::observe(&start, descending_jump_in::Guard::Standing, fixture);
    assert!(outcome.moves.contains(&0x04), "his standing guard: {:x?}", outcome.moves);
    assert_eq!(outcome.health_after, outcome.health_before, "no damage through the standing guard");
    // Under standard high and low blocking a jump attack is an overhead:
    // his crouching guard does not take it.
    assert!(
        MatchConfig::from_preset(CLASSIC)
            .toggles
            .standard_high_low_block
    );
    let outcome = descending_jump_in::observe(&start, descending_jump_in::Guard::Crouching, fixture);
    assert!(outcome.health_after < outcome.health_before, "a jump-in lands on the crouching guard");
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
        out.push((
            format!("standing {}", names[b]),
            vec![press(&[], b), press(&[], b)],
            idle(),
            id,
            level(b),
        ));
    }
    for (b, &id) in [0x11, 0x1c, 0x1c, 0x17, 0x17, 0x18].iter().enumerate() {
        let mut inputs = vec![hold(&[DOWN]); 4];
        inputs.extend([press(&[DOWN], b), press(&[DOWN], b)]);
        out.push((
            format!("crouching {}", names[b]),
            inputs,
            hold(&[DOWN]),
            id,
            level(b),
        ));
    }
    for (dir_name, dirs, ids) in [
        ("down-forward", vec![DOWN, RIGHT], [0x12, 0x1d, 0x1d]),
        ("down-back", vec![DOWN, LEFT], [0x13, 0x1e, 0x1e]),
    ] {
        for (b, &id) in ids.iter().enumerate() {
            let mut inputs = vec![hold(&dirs); 4];
            inputs.extend([press(&dirs, b), press(&dirs, b)]);
            out.push((
                format!("{dir_name} {}", names[b]),
                inputs,
                hold(&dirs),
                id,
                level(b),
            ));
        }
    }
    for (jump, dirs, ids) in [
        ("straight", vec![UP], [0x20, 0x20, 0x20, 0x23, 0x23, 0x26]),
        (
            "forward",
            vec![UP, RIGHT],
            [0x21, 0x21, 0x21, 0x24, 0x24, 0x24],
        ),
        ("back", vec![UP, LEFT], [0x22, 0x22, 0x22, 0x25, 0x25, 0x25]),
    ] {
        for (b, &id) in ids.iter().enumerate() {
            let mut inputs = vec![hold(&dirs); 2];
            inputs.extend(vec![idle(); 10]);
            inputs.extend([press(&[], b), press(&[], b)]);
            out.push((
                format!("{jump} jump {}", names[b]),
                inputs,
                idle(),
                id,
                level(b),
            ));
        }
    }
    out
}

#[test]
fn every_normal_comes_out_and_its_frames_match_its_data() {
    let Some(l) = local_roms() else { return };
    let start = apart(&l);
    let bank = start.data_bank().unwrap().clone();
    let data = Data {
        rom: &l.a.maincpu,
        bank: &bank,
    };
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
                data.jump_frames(EAGLE_TABLE, jump, id >= 0x23, level)
            }
            _ => data.move_frames(EAGLE_TABLE, id),
        };
        let (su, ac, re) = phases(&measured);
        println!("{name:20} move {id:#04x}: startup {su:2}, active {ac:2}, recovery {re:2}");
        assert!(ac > 0, "{name}: an attack");
        assert_eq!(
            measured, expected,
            "{name}: the frames as the fight reader reads them are the move's data"
        );
        assert!(unrecovered(&s).is_empty(), "{name}");
    }
}

/// From `start`, feed `inputs` then idle frames and read the special `id`:
/// its active flags frame by frame, the frame (of the move, from 1) Ken was
/// first hurt on, and the gap from Eagle's grid centre to Ken's on the
/// move's first frame and after it.
fn special_with_guard(
    start: &GameSession,
    inputs: Vec<FrameInputs>,
    id: u8,
    p2_dirs: &[usize],
) -> (Vec<bool>, Option<usize>, i32, i32, GameSession) {
    let mut s = start.clone();
    let gap = |s: &GameSession| grid_centre_x(s, P2_STRUCT) - grid_centre_x(s, P1_STRUCT);
    let mut gap0 = None;
    let mut frames = Vec::new();
    let mut hit_on = None;
    for mut input in inputs.into_iter().chain(vec![idle(); 90]) {
        for &d in p2_dirs {
            input.p2.directions[d] = true;
        }
        let before = p2(&s).health;
        s.step(input);
        let r = p1(&s);
        if r.move_id == id && r.state & 0x80 != 0 {
            gap0.get_or_insert(gap(&s));
            frames.push(r.attack_active);
            if p2(&s).health < before {
                hit_on.get_or_insert(frames.len());
            }
        }
    }
    let gap1 = gap(&s);
    (frames, hit_on, gap0.expect("the special came out"), gap1, s)
}

#[test]
fn the_manchester_black_comes_from_its_command_and_advances_spinning_both_sticks() {
    let Some(l) = local_roms() else { return };
    let rom = &l.a.maincpu;
    let far = apart(&l);
    let bank = far.data_bank().unwrap().clone();
    let data = Data { rom, bank: &bank };
    let expected = data.move_frames(EAGLE_TABLE, 0x1a);
    let d = |n: u8| n as usize;
    assert_eq!(
        phases(&expected),
        (
            d(eagle_manchester_black::WINDUP),
            d(eagle_manchester_black::SPINS * eagle_manchester_black::SPIN_STEP),
            d(eagle_manchester_black::SETTLE),
        ),
        "its numbers (`manchester_black`)"
    );
    for button in 0..3 {
        let (frames, hit_on, gap0, gap1, s) = special_with_guard(&far, qcf(button), 0x1a, &[]);
        assert_eq!(frames, expected, "button {button}: its frames are its data");
        assert_eq!(
            (hit_on, gap0 - gap1),
            (None, 80),
            "button {button}: advances across the stage"
        );
        assert!(unrecovered(&s).is_empty());
    }

    // Up close it lands on its first active frame for his barrage's damage
    // (`0x47982`) and scores as the Hadoken.
    let start = close(&l);
    let score = p1_score(&start);
    let (frames, hit_on, _, _, s) = special_with_guard(&start, qcf(2), 0x1a, &[]);
    assert_eq!(
        frames.len(),
        expected.len(),
        "the move lasts its data's frames"
    );
    assert_eq!(
        hit_on,
        Some(7),
        "the barrage lands on its first active frame"
    );
    let hurt = p2(&start).health - p2(&s).health;
    let hit = 0x479e8; // his barrage's entries (0x47982)
    assert!(
        hurt == damage(rom, hit) || hurt == damage(rom, hit - 8),
        "his barrage's damage: {hurt}"
    );
    assert_eq!(
        p1_score(&s) - score,
        points(rom, 0x45ff6),
        "scored as the Hadoken"
    );
    // Ken holding back blocks it.
    let (_, hit_on, _, _, s) = special_with_guard(&start, qcf(2), 0x1a, &[RIGHT]);
    assert_eq!(
        (hit_on, p2(&s).health),
        (None, p2(&start).health),
        "guarded"
    );

    // The Hadoken's command with a kick is no special of his: the kick it
    // ends on.
    let id = first_attack(&far, &qcf(3));
    assert!((0x14..=0x16).contains(&id), "a standing kick: {id:#04x}");
    let mut s = far.clone();
    assert_eq!(
        measure(&mut s, &qcf(3), id),
        data.move_frames(EAGLE_TABLE, id)
    );
}

#[test]
fn the_st_andrews_green_comes_from_its_command_and_leaps_into_a_two_stick_smash() {
    let Some(l) = local_roms() else { return };
    let rom = &l.a.maincpu;
    let far = apart(&l);
    let bank = far.data_bank().unwrap().clone();
    let data = Data { rom, bank: &bank };
    let expected = data.move_frames(EAGLE_TABLE, 0x1b);
    assert_eq!(
        phases(&expected),
        (18, 14, 20),
        "its numbers (`st_andrews_green`)"
    );
    let d = |n: u8| n as usize;
    assert_eq!(
        phases(&expected),
        (
            d(
                eagle_st_andrews_green::SQUAT
                    + eagle_st_andrews_green::RISE
                    + eagle_st_andrews_green::APEX
            ),
            d(eagle_st_andrews_green::SMASH),
            d(
                eagle_st_andrews_green::FALL
                    + eagle_st_andrews_green::LAND
                    + eagle_st_andrews_green::SETTLE
            ),
        )
    );

    for button in 3..6 {
        let (frames, hit_on, gap0, gap1, s) = special_with_guard(&far, qcb(button), 0x1b, &[]);
        assert_eq!(frames, expected, "button {button}: its frames are its data");
        let step = (eagle_st_andrews_green::RISE as i32
            + eagle_st_andrews_green::APEX as i32
            + eagle_st_andrews_green::SMASH as i32
            + eagle_st_andrews_green::FALL as i32)
            * eagle_st_andrews_green::SPEED;
        assert_eq!(
            (hit_on, gap0 - gap1),
            (None, step),
            "button {button}: leaps {step} pixels across the stage"
        );
        assert!(unrecovered(&s).is_empty());
    }

    // Up close the leaping two-stick smash lands on the 19th frame for his
    // leaping stick strike's damage (`0x47acc`) and scores as the Tatsumaki.
    let start = close(&l);
    let score = p1_score(&start);
    let (frames, hit_on, _, _, s) = special_with_guard(&start, qcb(5), 0x1b, &[]);
    assert_eq!(
        frames.len(),
        expected.len(),
        "the move lasts its data's frames"
    );
    assert_eq!(
        hit_on,
        Some(19),
        "the leaping smash lands on its first active frame"
    );
    let hurt = p2(&start).health - p2(&s).health;
    let hit = 0x47b32; // his leaping stick strike's entries (0x47acc)
    assert!(
        hurt == damage(rom, hit) || hurt == damage(rom, hit - 8),
        "his leaping smash's damage: {hurt}"
    );
    assert_eq!(
        p1_score(&s) - score,
        points(rom, 0x46012),
        "scored as the Tatsumaki"
    );
    // Ken holding back blocks it.
    let (_, hit_on, _, _, s) = special_with_guard(&start, qcb(5), 0x1b, &[RIGHT]);
    assert_eq!(
        (hit_on, p2(&s).health),
        (None, p2(&start).health),
        "guarded"
    );

    // The Tatsumaki's command with a punch, and the Shoryuken's with a punch,
    // are no specials of his: the punch they end on.
    for inputs in [qcb(0), dp(0)] {
        let id = first_attack(&far, &inputs);
        assert!(!(0x19..=0x1b).contains(&id), "no special: {id:#04x}");
        let mut s = far.clone();
        let after = *inputs.last().unwrap();
        let after = with(|i| i.p1.directions = after.p1.directions);
        assert_eq!(
            measure_then(&mut s, &inputs, after, id),
            data.move_frames(EAGLE_TABLE, id)
        );
    }
}

#[test]
fn both_specials_come_out_under_pure_footsies_strict_1987_commands() {
    let Some(l) = local_roms() else { return };
    // Pure Footsies: 1987-strict commands (no lenient trackers), so each
    // special fires only from its command's last direction, held as the
    // button goes: forward for Manchester Black, back for St. Andrew's Green.
    let start = local_with(
        session(&l, RulesetPreset::PureFootsies1987Plus, true),
        TournamentFighter::Eagle,
    );
    assert!(
        !start
            .fight_machine()
            .unwrap()
            .match_config()
            .toggles
            .lenient_command_trackers
    );
    assert_eq!(rl(&start, P1_STRUCT + 0x12), EAGLE_TABLE);
    let data_bank = start.data_bank().unwrap().clone();
    let data = Data {
        rom: &l.a.maincpu,
        bank: &data_bank,
    };
    let mut s = start.clone();
    let frames = measure_then(&mut s, &qcf(1), hold(&[RIGHT]), 0x1a);
    assert_eq!(
        frames.len(),
        data.move_frames(EAGLE_TABLE, 0x1a).len(),
        "Manchester Black"
    );
    let mut s = start.clone();
    let frames = measure_then(&mut s, &qcb(4), hold(&[LEFT]), 0x1b);
    assert_eq!(
        frames.len(),
        data.move_frames(EAGLE_TABLE, 0x1b).len(),
        "St. Andrew's Green"
    );
    assert!(unrecovered(&s).is_empty());
}

/// The fraction of `rect` (in a pose's pixels) that is drawn.
fn drawn(p: &Pens, [x0, y0, x1, y1]: [i32; 4]) -> f64 {
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            if (x as usize) < p.width
                && (y as usize) < p.height
                && p.at(x as usize, y as usize) != TRANSPARENT
            {
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
    let p = l.art.catalogue("eagle").unwrap().pose(name).unwrap();
    PoseGeom {
        code: p.grid.code,
        attr: if p.grid.large { 0x40a } else { 0x0a },
        rows: p.grid.rows,
        cols: p.grid.cols,
        anchor: p.anchor,
    }
}

#[test]
fn every_box_the_bank_writes_matches_the_drawn_pose() {
    let Some(l) = local_roms() else { return };
    for (name, boxes) in EAGLE_BOXES {
        let pens = &l
            .art
            .built
            .iter()
            .find(|(f, n, _)| f == "eagle" && n == name)
            .unwrap_or_else(|| panic!("{name} drawn"))
            .2;
        assert!(
            drawn(pens, boxes.head) >= 0.8,
            "{name}: head {:.2}",
            drawn(pens, boxes.head)
        );
        assert!(
            drawn(pens, boxes.body) >= 0.6,
            "{name}: body {:.2}",
            drawn(pens, boxes.body)
        );
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
        assert!(
            body as f64 >= 0.8 * all as f64,
            "{name}: the body box holds {body} of {all} drawn pixels"
        );
        assert!(
            drawn(pens, boxes.push) >= 0.6,
            "{name}: push {:.2}",
            drawn(pens, boxes.push)
        );
        if let Some(a) = boxes.attack {
            assert!(
                drawn(pens, a) >= 0.5,
                "{name}: attack {:.2}",
                drawn(pens, a)
            );
            let front = (a[1]..a[3])
                .filter_map(|y| {
                    (0..pens.width)
                        .rev()
                        .find(|&x| pens.at(x, y as usize) != TRANSPARENT)
                })
                .max()
                .unwrap();
            assert!(
                (a[0]..a[2]).contains(&(front as i32)),
                "{name}: the limb's tip at x {front} is in the attack box"
            );
        }
        let (hx, hy) = geom(&l, name).half();
        assert!(
            boxes.push[0] <= hx
                && hx <= boxes.push[2]
                && boxes.push[1] <= hy
                && hy <= boxes.push[3],
            "{name}: push box about the centre ({hx}, {hy}) vs {:?}",
            boxes.push
        );
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
    let boxes = |name: &str| EAGLE_BOXES.iter().find(|(n, _)| *n == name).unwrap().1;
    let jump = |dirs: &[usize], b: usize| {
        [vec![hold(dirs); 2], vec![idle(); 6], vec![press(&[], b); 2]].concat()
    };
    let crouching = |b: usize| vec![hold(&[DOWN]), press(&[DOWN], b), press(&[DOWN], b)];
    let standing = |b: usize| vec![press(&[], b), press(&[], b)];
    // (what, inputs, held after, move, the tiles and boxes it strikes with)
    enum Want {
        Pose(&'static str),
        Rom(u32),
    }
    for (what, inputs, after, id, want) in [
        (
            "rising stick strike",
            standing(4),
            idle(),
            0x15,
            Want::Pose("rising_strike"),
        ),
        (
            "two-stick smash",
            standing(5),
            idle(),
            0x16,
            Want::Pose("smash"),
        ),
        (
            "crouching stick sweep",
            crouching(3),
            hold(&[DOWN]),
            0x17,
            Want::Pose("crouching_sweep"),
        ),
        (
            "crouching stick uppercut",
            crouching(5),
            hold(&[DOWN]),
            0x18,
            Want::Pose("crouching_uppercut"),
        ),
        (
            "straight jump punch",
            jump(&[UP], 0),
            idle(),
            0x20,
            Want::Pose("jump_high"),
        ),
        (
            "diagonal jump punch",
            jump(&[UP, RIGHT], 0),
            idle(),
            0x21,
            Want::Pose("jump_jab"),
        ),
        (
            "straight jump kick",
            jump(&[UP], 3),
            idle(),
            0x23,
            Want::Pose("jump_poke"),
        ),
        (
            "diagonal jump kick",
            jump(&[UP, RIGHT], 3),
            idle(),
            0x24,
            Want::Pose("jump_swing"),
        ),
        ("stick jab", standing(0), idle(), 0x0d, Want::Rom(JAB_FRAME)),
        (
            "low stick thrust (MP)",
            standing(1),
            idle(),
            0x0f,
            Want::Rom(LOW_THRUST_FRAME),
        ),
        (
            "spinning sticks (HP)",
            standing(2),
            idle(),
            0x10,
            Want::Rom(SPIN_STICKS_FRAME),
        ),
        (
            "crouching punch",
            crouching(0),
            hold(&[DOWN]),
            0x11,
            Want::Rom(LOW_THRUST_FRAME),
        ),
        (
            "low stick poke (LK)",
            standing(3),
            idle(),
            0x14,
            Want::Rom(JAB_FRAME),
        ),
        (
            "straight jump heavy kick",
            jump(&[UP], 5),
            idle(),
            0x26,
            Want::Rom(NEUTRAL_LEAP_SMASH_FRAME),
        ),
        (
            "diagonal jump heavy kick",
            jump(&[UP, RIGHT], 5),
            idle(),
            0x24,
            Want::Rom(FORWARD_LEAP_SMASH_FRAME),
        ),
        (
            "manchester black",
            qcf(0),
            idle(),
            0x1a,
            Want::Rom(BARRAGE_FRAME),
        ),
        (
            "st. andrew's green",
            qcb(3),
            idle(),
            0x1b,
            Want::Rom(NEUTRAL_LEAP_SMASH_FRAME),
        ),
    ] {
        let mut s = start.clone();
        let mut checked = false;
        for n in 0..200 {
            s.step(inputs.get(n).copied().unwrap_or(after));
            if p1(&s).move_id == id && p1(&s).attack_active {
                let code = u16::from_be_bytes(p1_bytes(&s, 0, 2).try_into().unwrap());
                match want {
                    Want::Pose(name) => {
                        let g = geom(&l, name);
                        // Facing right the first entry is the grid's own first tile.
                        assert_eq!(
                            code, g.code,
                            "{what}: its own tiles from the extended sprite bank"
                        );
                        assert!(code >= 0x3800, "{what}: a bank tile");
                        assert_eq!(
                            p1_bytes(&s, 0x16, 16),
                            boxes(name).bytes(&g),
                            "{what}: the boxes written for its drawing"
                        );
                    }
                    Want::Rom(frame) => {
                        assert_eq!(
                            code,
                            u16::from_be_bytes([rom[frame as usize + 2], rom[frame as usize + 3]]),
                            "{what}: its ROM tiles"
                        );
                        assert_eq!(
                            p1_bytes(&s, 0x16, 16),
                            rom_boxes(rom, frame),
                            "{what}: its ROM boxes"
                        );
                    }
                }
                checked = true;
                break;
            }
        }
        assert!(checked, "{what} struck");
    }
    // His crouch: its new pose and the boxes written for it.
    let mut s = start.clone();
    for _ in 0..10 {
        s.step(hold(&[DOWN]));
    }
    let g = geom(&l, "crouch");
    assert_eq!(
        u16::from_be_bytes(p1_bytes(&s, 0, 2).try_into().unwrap()),
        g.code,
        "his crouch's own tiles"
    );
    assert_eq!(
        p1_bytes(&s, 0x16, 16),
        boxes("crouch").bytes(&g),
        "his crouch's boxes"
    );
    // His standing block: his ROM block's pose (`0x2aa0`) with its attack box
    // cleared.
    let mut s = eagle_local(&l);
    for _ in 0..120 {
        s.step(hold(&[RIGHT]));
    }
    let mut blocked = false;
    for f in 0..120u32 {
        s.step(with(|i| {
            i.p1.directions[LEFT] = true;
            i.p2.buttons[1] = f % 40 < 2;
        }));
        if p1(&s).move_id == 0x04 {
            assert_eq!(
                u16::from_be_bytes(p1_bytes(&s, 0, 2).try_into().unwrap()),
                0x2aa0,
                "his ROM block's tiles"
            );
            let h = ROM_TABLE as usize + 4 * 0x30 + 0x16;
            let mut expected = rom[h..h + 16].to_vec();
            expected[12..16].fill(0);
            assert_eq!(
                p1_bytes(&s, 0x16, 16),
                expected,
                "his ROM block's boxes with its attack box cleared"
            );
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
    let data = Data {
        rom: &l.a.maincpu,
        bank: &bank,
    };
    // His: every catalogue pose's grid, and every frame of his ROM table's
    // streams and of his CPU's attack streams.
    let mut his = BTreeSet::new();
    for p in &l.art.catalogue("eagle").unwrap().poses {
        grid_codes(
            p.grid.code,
            p.grid.large,
            p.grid.rows,
            p.grid.cols,
            &mut his,
        );
    }
    for id in 0..13u32 {
        let h = ROM_TABLE + id * 0x30;
        if data.w(h) == 0 {
            continue; // an unused slot (`0x05..=0x08`, his empty crouches and crouching block)
        }
        grid_codes(
            data.w(h),
            data.w(h + 2) & 0x400 != 0,
            data.b(h + 0xc),
            data.b(h + 0xd),
            &mut his,
        );
        if data.l(h + 0x2c) != 0 {
            data.stream_codes(data.l(h + 0x2c), &mut his);
        }
    }
    for stream in ROM_ATTACKS {
        data.stream_codes(stream, &mut his);
    }
    // His hit and knock-out reactions (his 1987 tables, `reaction_tables`:
    // 13 entries of a 16-bit stream address and two bytes).
    let (hits, knock_outs) = TournamentFighter::Eagle.reaction_tables().unwrap();
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
                grid_codes(
                    data.w(h),
                    data.w(h + 2) & 0x400 != 0,
                    data.b(h + 0xc),
                    data.b(h + 0xd),
                    &mut theirs,
                );
                if data.l(h + 0x2c) != 0 {
                    data.stream_codes(data.l(h + 0x2c), &mut theirs);
                }
            }
        }
    }
    let shared: Vec<String> = his
        .intersection(&theirs)
        .map(|c| format!("{c:#06x}"))
        .collect();
    assert!(
        shared.is_empty(),
        "his ROM poses share no tile with Ryu's or Ken's: {shared:?}"
    );
    // Every move of his, standing, crouching, jumping, blocking and both
    // specials: each tile he shows is his.
    let mut scripts: Vec<(Vec<FrameInputs>, FrameInputs)> = normals()
        .into_iter()
        .map(|(_, inputs, after, _, _)| (inputs, after))
        .collect();
    scripts.push((qcf(0), idle()));
    scripts.push((qcb(3), idle()));
    scripts.push((vec![hold(&[RIGHT]); 30], idle()));
    scripts.push((vec![hold(&[LEFT]); 30], idle()));
    scripts.push((vec![hold(&[DOWN]); 30], idle()));
    scripts.push((vec![hold(&[UP, RIGHT]); 2], idle()));
    for (inputs, after) in scripts {
        let mut s = start.clone();
        for n in 0..120 {
            s.step(inputs.get(n).copied().unwrap_or(after));
            for code in p1_codes(&s) {
                assert!(
                    his.contains(&code),
                    "a tile not his: {code:#06x} in move {:#04x}",
                    p1(&s).move_id
                );
            }
        }
    }
    let mut s = eagle_local(&l);
    for f in 0..240u32 {
        s.step(with(|i| {
            i.p1.directions[RIGHT] = f < 120;
            i.p1.directions[LEFT] = f >= 120;
            i.p1.directions[DOWN] = f >= 180;
            i.p2.buttons[(f / 40 % 6) as usize] = f % 40 < 2;
        }));
        for code in p1_codes(&s) {
            assert!(
                his.contains(&code),
                "a tile not his: {code:#06x} in move {:#04x}",
                p1(&s).move_id
            );
        }
    }
}

#[test]
fn the_fight_replays_identically_from_the_same_inputs() {
    let Some(l) = local_roms() else { return };
    let start = eagle_local(&l);
    // His specials first (facing right), then both sides at random.
    let script: Vec<FrameInputs> = qcf(1)
        .into_iter()
        .chain(vec![idle(); 60])
        .chain(qcb(4))
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
                let hash = out
                    .frame
                    .argb
                    .iter()
                    .fold(0xcbf2_9ce4_8422_2325u64, |h, &p| {
                        (h ^ p as u64).wrapping_mul(0x100_0000_01b3)
                    });
                (hash, s.fight().unwrap().fight_read().fighters)
            })
            .collect()
    };
    let mut a = start.clone();
    let first = run(&mut a, &script);
    let mut b = start.clone();
    let second = run(&mut b, &script);
    assert_eq!(first, second, "the same inputs, the same frames and fight");
    assert!(
        first
            .iter()
            .any(|(_, f)| (0x0d..=0x2f).contains(&f[0].move_id)),
        "Eagle attacked"
    );
    assert!(
        first.iter().any(|(_, f)| f[0].move_id == 0x1a),
        "and threw his Manchester Black"
    );
    assert!(
        first.iter().any(|(_, f)| f[0].move_id == 0x1b),
        "and threw his St. Andrew's Green"
    );
    assert_eq!(
        a.fight_machine().unwrap().save_snapshot(),
        b.fight_machine().unwrap().save_snapshot()
    );
    let mut c = start.clone();
    run(&mut c, &script[..300]);
    let mut snapshot = c.clone();
    assert_eq!(
        run(&mut c, &script[300..]),
        first[300..].to_vec(),
        "straight on"
    );
    assert_eq!(
        run(&mut snapshot, &script[300..]),
        first[300..].to_vec(),
        "from the snapshot"
    );
}

fn open_moves(s: &mut GameSession) {
    assert_eq!(
        s.step(with(|i| i.menu.start = true)).transition,
        Some(Transition::Paused)
    );
    s.step(idle());
    let at = PauseBox::open(true)
        .items()
        .iter()
        .position(|&i| i == MenuItem::Moves)
        .expect("a MOVES row");
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

/// Training against the dummy under Classic Competitive, Eagle as P1, with
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
    tap(&mut s, hold(&[DOWN])); // second row
    tap(&mut s, hold(&[RIGHT]));
    tap(&mut s, hold(&[RIGHT]));
    tap(&mut s, hold(&[RIGHT])); // column 3 (Eagle)
    tap(&mut s, with(|i| i.menu.confirm = true)); // Eagle
    s.step(with(|i| i.menu.confirm = true)); // the opponent under the cursor
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_control(&mut s);
    let table = if tables {
        EAGLE_TABLE
    } else {
        TournamentFighter::Eagle.move_table()
    };
    assert_eq!(rl(&s, P1_STRUCT + 0x12), table);
    s
}

#[test]
fn moves_lists_his_specials_and_show_and_try_work_for_each() {
    let Some(l) = local_roms() else { return };
    for preset in [
        CLASSIC,
        RulesetPreset::DefinitiveOverhaul,
        RulesetPreset::PureFootsies1987Plus,
    ] {
        let names: Vec<_> = move_list(TournamentFighter::Eagle, preset)
            .iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(
            names,
            ["MANCHESTER BLACK", "ST. ANDREW'S GREEN"],
            "{preset:?}"
        );
    }
    assert!(move_list(TournamentFighter::Eagle, RulesetPreset::Faithful1987).is_empty());
    // Without his table the fight has neither, and MOVES lists neither.
    let text = sf_engine::front::moves::MovesPanel::open_in_fight(
        TournamentFighter::Eagle,
        CLASSIC,
        false,
    )
    .text();
    assert!(
        !text
            .iter()
            .any(|line| line.contains("MANCHESTER BLACK") || line.contains("ST. ANDREW'S GREEN")),
        "{text:?}"
    );
    let start = training(&l, true);
    let mut s = start.clone();
    open_moves(&mut s);
    let text = s.move_list().expect("the list").text();
    assert!(
        text.iter().any(|line| line.contains("MANCHESTER BLACK")),
        "{text:?}"
    );
    assert!(
        text.iter().any(|line| line.contains("ST. ANDREW'S GREEN")),
        "{text:?}"
    );
    for (row, entry) in move_list(TournamentFighter::Eagle, CLASSIC)
        .into_iter()
        .enumerate()
    {
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
        for p in entry
            .input
            .frames(true, Strength::Medium)
            .into_iter()
            .chain(vec![Default::default(); 120])
        {
            s.step(with(|i| i.p1 = p));
        }
        let coach = s.coach().expect("the coach");
        assert_eq!(coach.entry().name, entry.name);
        assert_eq!(
            (coach.successes(), coach.outcome()),
            (1, Some(Outcome::Success)),
            "{}",
            entry.name
        );
    }
}

#[test]
fn a_whole_match_with_eagle_on_either_side_runs_clean() {
    let Some(l) = local_roms() else { return };
    // Eagle on P1 against Ken, both sides walking in and cycling buttons,
    // crouching, jumping and throwing his specials, until the after-match
    // box.
    let mut s = eagle_local(&l);
    let mut frames = 0u32;
    let specials: Vec<FrameInputs> = [qcf(1), qcb(4)].concat();
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
        assert!(
            unrecovered(&s).is_empty(),
            "frame {frames}: {:05x?}",
            unrecovered(&s)
        );
    }
    assert_eq!(rl(&s, P1_STRUCT + 0x12), EAGLE_TABLE);
    println!("Eagle v Ken: {frames} frames to the after-match box");

    // Eagle on P2 (facing left), against Ryu: P2's cursor starts on Ken
    // (row 0, col 1), so DOWN then RIGHT twice reaches Eagle (row 1, col 3).
    let mut s = session(&l, CLASSIC, true);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, hold(&[DOWN]));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Multi Player
    tap(&mut s, with(|i| i.menu.confirm = true)); // Local
    tap(&mut s, with(|i| i.menu.confirm = true)); // P1 Ryu
    tap(&mut s, with(|i| i.p2.directions[DOWN] = true));
    tap(&mut s, with(|i| i.p2.directions[RIGHT] = true));
    tap(&mut s, with(|i| i.p2.directions[RIGHT] = true));
    let out = s.step(with(|i| i.p2.buttons[0] = true));
    assert_eq!(
        out.transition,
        Some(Transition::Fighters(
            TournamentFighter::Ryu,
            TournamentFighter::Eagle
        ))
    );
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    assert_eq!(rl(&s, P2_STRUCT + 0x12), EAGLE_TABLE);
    // His specials from the right side: Manchester Black (down, down-left,
    // left, punch) and St. Andrew's Green (down, down-right, right, kick),
    // mirrored.
    let p2_of = |v: Vec<FrameInputs>| -> Vec<FrameInputs> {
        v.into_iter()
            .map(|i| {
                let mut o = FrameInputs::default();
                o.p2.directions = [
                    i.p1.directions[LEFT],
                    i.p1.directions[RIGHT],
                    i.p1.directions[DOWN],
                    i.p1.directions[UP],
                ];
                o.p2.buttons = i.p1.buttons;
                o
            })
            .collect()
    };
    for (inputs, id) in [(p2_of(qcf(0)), 0x1a), (p2_of(qcb(3)), 0x1b)] {
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
        assert!(
            unrecovered(&s).is_empty(),
            "frame {frames}: {:05x?}",
            unrecovered(&s)
        );
    }
    println!("Ryu v Eagle: {frames} frames to the after-match box");
}

#[test]
fn his_1987_attacks_score_as_ryus_do() {
    let Some(l) = local_roms() else { return };
    let rom = &l.a.maincpu;
    // (inputs, held after, Ryu's entry it scores as (either depth))
    for (what, inputs, after, entry) in [
        (
            "stick jab (LP)",
            vec![press(&[], 0), press(&[], 0)],
            idle(),
            0x45b6eu32,
        ),
        (
            "low thrust (MP)",
            vec![press(&[], 1), press(&[], 1)],
            idle(),
            0x45c88,
        ),
        (
            "spinning sticks (HP)",
            vec![press(&[], 2), press(&[], 2)],
            idle(),
            0x45d16,
        ),
        (
            "low stick poke (LK)",
            vec![press(&[], 3), press(&[], 3)],
            idle(),
            0x460a8,
        ),
    ] {
        let mut s = close(&l);
        let (score, health) = (p1_score(&s), p2(&s).health);
        for n in 0..60 {
            s.step(inputs.get(n).copied().unwrap_or(after));
        }
        assert!(p2(&s).health < health, "{what} landed");
        let scored = p1_score(&s) - score;
        let (deep, other) = (points(rom, entry), points(rom, entry + 8));
        assert!(
            scored == deep || scored == other,
            "{what}: scored {scored}, as Ryu's {deep} or {other}"
        );
    }
}

#[test]
fn the_overlays_frame_meter_reads_his_bank_moves() {
    let Some(l) = local_roms() else { return };
    let mut s = eagle_local(&l);
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
    assert_eq!(
        active.expect("and its active frames") - started,
        6,
        "6 frames of startup, as its data"
    );
    // And both specials, each seen to start and strike on its data's frame.
    for (inputs, id, startup) in [(qcf(1), 0x1au8, 6), (qcb(4), 0x1b, 18)] {
        let mut s = apart(&l);
        let (mut started, mut active) = (None, None);
        for (n, input) in inputs.into_iter().chain(vec![idle(); 40]).enumerate() {
            s.step(input);
            let t = s.fight_machine().unwrap().diagnostic_telemetry();
            if t.p1.move_id == id && t.p1.move_started_this_frame {
                started.get_or_insert(n);
            }
            if t.p1.move_id == id && t.p1.attack_active {
                active.get_or_insert(n);
            }
        }
        let started = started.unwrap_or_else(|| panic!("the overlay sees move {id:#04x} start"));
        assert_eq!(
            active.expect("and its active frames") - started,
            startup,
            "move {id:#04x}: its startup"
        );
    }
}
