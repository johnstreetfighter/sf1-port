//! Ticket #250 (`SP16`): mirror palettes in a Local mirror match.
//!
//! The headless game session walks Multi Player, Local (or Single Player,
//! Training) and the select with both sides picking one fighter, takes the
//! flag, and plays the fight frame by frame (inputs in; the presented
//! frames, the machine's state and the log out).
//!
//! - **A mirror match.** The second side's fighter is drawn in his mirror
//!   palette: in object RAM, in palette RAM, and in the presented frame. The
//!   first side, the projectiles and effects (the Hadokens, the hit sparks)
//!   and everything else keep the ROM's colours (the maintainer, 2026-11-08). Local
//!   and both Training kinds do this.
//! - **Everything else.** A fight of two different fighters, Faithful 1987
//!   and the other play modes draw as the ROM does: palette RAM's sprite
//!   palettes are the ROM's, and no sprite uses the mirror slot.
//!
//! Faithful 1987 never does this, even with FULL CAST switched on.
//!
//! Joe cannot be picked against himself in Local yet (`MIRROR_NOT_CLEAN`,
//! until SP13), so his mirror is checked on the machine itself. Local ROMs
//! only; each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::collections::BTreeMap;
use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{drive_to_country_select, flow_position, pick_country, Players};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::roster::{PlayMode, RosterSettings, TournamentFighter};
use sf_engine::run::PowerOnMachine;
use sf_video::art::{load_art_from, Art, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::mirror_palette::{self, MirrorPalette};
use sf_video::render::{render_frame, Frame};
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

fn art(a: &Assets) -> Option<Art> {
    let pack = ArtPack::open(Path::new(PACK)).ok()?;
    load_art_from(a, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).ok()
}
/// The ROM's sprite palettes, which the boot copies to palette RAM 0x400.
const ROM_SPRITE_PALETTES: usize = 0x1ba68;
const FIGHT_LOOP: u32 = 0x3eafa;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load())
}

/// The ROM set again (`Assets` is not `Clone`): one for the session, one to
/// check it against.
fn load() -> Assets {
    load_assets(ROMS, MANIFEST).expect("local ROM set assembles")
}

fn mirror(f: TournamentFighter) -> &'static MirrorPalette {
    mirror_palette::for_fighter(f.name()).expect("a committed mirror palette")
}

// ---- the session ---------------------------------------------------------------

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

const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;

/// Power-on, 200 attract frames, Start, Multi Player, Local.
fn to_local_select(a: Assets, config: MatchConfig) -> GameSession {
    let loaded_art = art(&a);
    let mut s = GameSession::new(a, config);
    if let Some(art) = &loaded_art {
        s = s.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
    }
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "multi player");
    let out = tap(&mut s, with(|i| i.menu.confirm = true));
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Local)));
    s
}

fn tile(f: TournamentFighter) -> usize {
    TournamentFighter::ALL.iter().position(|&x| x == f).unwrap()
}

/// Step until the screen is `name`.
fn until(s: &mut GameSession, name: &str, limit: u32) {
    for _ in 0..limit {
        if s.step(idle()).screen == name {
            return;
        }
    }
    panic!("never reached {name} in {limit} frames (at {})", s.screen());
}

/// A Local fight, P1 `p1` and P2 `p2`, on the fight's first frame. P1's
/// cursor starts on the first tile and P2's on the second; the grid is two
/// rows of six.
fn local_fight(a: Assets, p1: TournamentFighter, p2: TournamentFighter) -> GameSession {
    let mut s = to_local_select(a, MatchConfig::default());
    let (row, col) = (tile(p1) / 6, tile(p1) % 6);
    for d in std::iter::repeat(DOWN).take(row).chain(std::iter::repeat(RIGHT).take(col)) {
        assert_eq!(tap(&mut s, with(|i| i.p1.directions[d] = true)).transition, Some(Transition::CursorMoved));
    }
    assert_eq!(select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), tap).transition, Some(Transition::Picked(p1)));
    let (row, col) = (tile(p2) / 6, tile(p2) % 6);
    let mut moves: Vec<usize> = std::iter::repeat(DOWN).take(row).collect();
    if col >= 1 {
        moves.extend(std::iter::repeat(RIGHT).take(col - 1));
    } else {
        moves.push(LEFT);
    }
    for d in moves {
        assert_eq!(tap(&mut s, with(|i| i.p2.directions[d] = true)).transition, Some(Transition::CursorMoved));
    }
    let out = select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), GameSession::step);
    assert_eq!(out.transition, Some(Transition::Fighters(p1, p2)));
    s.step(idle());
    until(&mut s, "country select", 30);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 30);
    s
}

// ---- reading the fight -----------------------------------------------------------

fn word(ram: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([ram[at], ram[at + 1]])
}

fn long(ram: &[u8], at: usize) -> u32 {
    (word(ram, at) as u32) << 16 | word(ram, at + 2) as u32
}

/// Where a side's 24-entry fighter region starts in object RAM, if the
/// fighter is on screen.
fn region(m: &PowerOnMachine, ps: u32) -> Option<usize> {
    let e = word(m.work_ram(), (ps - 0xff8000) as usize + 0x10) as usize;
    ([0x400, 0xa00].contains(&e) && m.state.objectram[e + 8] != 0).then_some(e)
}

/// One sprite's palette: the low four bits of its attribute word.
fn pal(o: &[u8], at: usize) -> u16 {
    word(o, at + 2) & 0xf
}

/// Whether the sprite entry at `at` is on screen: a parked entry has its
/// position zeroed and keeps whatever attribute it had last.
fn drawn(o: &[u8], at: usize) -> bool {
    word(o, at + 4) != 0 || word(o, at + 6) != 0
}

/// Object RAM offsets of the 12 effect objects.
fn effects() -> impl Iterator<Item = usize> {
    (0..12).map(|k| 0x100 + k * 0x40)
}

/// The ROM's palette `p`, as its 16 words of palette RAM bytes.
fn rom_palette(rom: &Assets, p: usize) -> &[u8] {
    &rom.maincpu[ROM_SPRITE_PALETTES + p * 32..ROM_SPRITE_PALETTES + p * 32 + 32]
}

/// What a fight showed, over its frames.
#[derive(Default, Debug)]
struct Seen {
    frames: u32,
    /// How many drawn sprites of each side's region, and of each side's
    /// effects, were in each palette.
    p1_body: BTreeMap<u16, u32>,
    p2_body: BTreeMap<u16, u32>,
    p1_effects: BTreeMap<u16, u32>,
    p2_effects: BTreeMap<u16, u32>,
}

/// The one palette a mirror uses: no sprite but the second side's body may
/// be in it.
fn slot(f: TournamentFighter) -> u16 {
    u16::from(mirror(f).slot)
}

/// Check one frame of a mirror match of `f`, the second side being the
/// player structure `second`, against the gate's rules, and add what it
/// showed to `seen`.
fn check_mirror_frame(rom: &Assets, m: &PowerOnMachine, f: TournamentFighter, second: u32, seen: &mut Seen) {
    let (Some(p1), Some(p2)) = (region(m, P1_STRUCT), region(m, second)) else { return };
    let o = &m.state.objectram;
    let at_frame = m.state.frame;
    seen.frames += 1;
    let (own, slot) = (u16::from(f.sprite_palette()), slot(f));
    // The second side's body is out of his own palette, parked entries too
    // (a pose of his in another palette, Geki's flash, is left); the first
    // side's drawn sprites are never in the slot (a parked entry of his keeps
    // what the region held before the two swapped places).
    for k in 0..24 {
        let (a, b) = (pal(o, p1 + k * 0x40), pal(o, p2 + k * 0x40));
        assert!(b != own, "{f:?} frame {at_frame}: the second side's sprite {k} is still in his own palette");
        if drawn(o, p2 + k * 0x40) {
            *seen.p2_body.entry(b).or_default() += 1;
        }
        if drawn(o, p1 + k * 0x40) {
            assert!(a != slot, "{f:?} frame {at_frame}: the first side's sprite {k} is in the mirror slot");
            *seen.p1_body.entry(a).or_default() += 1;
        }
    }
    // Effects keep the ROM's colours, whoever owns them.
    for at in effects().filter(|&at| o[at + 8] != 0) {
        let (owner, p) = (long(o, at + 0x20), pal(o, at));
        assert!(p != slot, "{f:?} frame {at_frame}: an effect of {owner:#x} is in the mirror slot");
        if owner == second {
            *seen.p2_effects.entry(p).or_default() += 1;
        } else if owner == P1_STRUCT {
            *seen.p1_effects.entry(p).or_default() += 1;
        }
    }
    // Nothing else drawn is in the slot.
    for at in (0..o.len()).step_by(0x40) {
        let theirs = (p2..p2 + 24 * 0x40).contains(&at);
        assert!(!drawn(o, at) || theirs || pal(o, at) != slot, "{f:?} frame {at_frame}: the sprite at {at:#x} is in the mirror slot");
    }
    // The slot holds the mirror's colours and the replaced palette's pen
    // 15; every other palette is the ROM's.
    let ram = &m.state.paletteram[0x400..0x600];
    for p in 0..16u16 {
        let bytes = &ram[p as usize * 32..p as usize * 32 + 32];
        if p == slot {
            for (pen, w) in mirror(f).pens.iter().enumerate() {
                assert_eq!(word(bytes, 2 * pen), *w, "{f:?} slot pen {pen}");
            }
            assert_eq!(&bytes[30..], &rom_palette(rom, own as usize)[30..], "{f:?} slot pen 15");
        } else {
            assert_eq!(bytes, rom_palette(rom, p as usize), "{f:?} frame {at_frame}: palette {p}");
        }
    }
}

/// Every sprite palette in RAM is the ROM's.
fn sprite_palettes_are_the_roms(rom: &Assets, m: &PowerOnMachine) {
    let ram = &m.state.paletteram[0x400..0x600];
    assert_eq!(ram, &rom.maincpu[ROM_SPRITE_PALETTES..ROM_SPRITE_PALETTES + 0x200], "frame {}", m.state.frame);
}

/// Nothing is drawn in the free palette 14, and each side's
/// fighter is in his own palette (or one of the shared effect palettes his
/// own poses sometimes use).
fn check_plain_frame(m: &PowerOnMachine, p1: TournamentFighter, p2: TournamentFighter) {
    let o = &m.state.objectram;
    for at in (0..o.len()).step_by(0x40) {
        if !drawn(o, at) {
            continue;
        }
        let p = pal(o, at);
        assert!(p != 14, "frame {}: the sprite at {at:#x} is in palette {p}", m.state.frame);
    }
    if let (Some(a), Some(b)) = (region(m, P1_STRUCT), region(m, P2_STRUCT)) {
        for k in 0..24 {
            let (x, y) = (pal(o, a + k * 0x40), pal(o, b + k * 0x40));
            if drawn(o, a + k * 0x40) {
                assert!(x == u16::from(p1.sprite_palette()) || x == 4, "P1's sprite {k} is in palette {x}");
            }
            if drawn(o, b + k * 0x40) {
                assert!(y == u16::from(p2.sprite_palette()) || y == 4, "P2's sprite {k} is in palette {y}");
            }
        }
    }
}

// ---- the inputs ------------------------------------------------------------------

/// A small seeded generator, so a fight is the same every time.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
}

/// One side's pad: the shotos throw a Hadoken, a Shoryuken and a Tatsumaki
/// in turn at the start of every 60 frames; for the rest of the time, and
/// for the bosses always, the pad walks and presses buttons at random.
struct Pad {
    rng: Rng,
    dirs: [bool; 4],
    buttons: [bool; 6],
    shoto: bool,
    /// The stick's forward and back directions.
    fwd: usize,
    back: usize,
}

impl Pad {
    fn new(seed: u64, shoto: bool, fwd: usize) -> Pad {
        Pad { rng: Rng(seed), dirs: [false; 4], buttons: [false; 6], shoto, fwd, back: 1 - fwd }
    }

    fn step(&mut self, f: u32) -> ([bool; 4], [bool; 6]) {
        let t = f % 60;
        if self.shoto && t < 14 {
            let (mut d, mut b) = ([false; 4], [false; 6]);
            let (fwd, back) = (self.fwd, self.back);
            match (f / 60) % 3 {
                // Hadoken: down, down and forward, forward and a punch.
                0 => match t {
                    0..=3 => d[DOWN] = true,
                    4..=7 => {
                        d[DOWN] = true;
                        d[fwd] = true;
                    }
                    _ => {
                        d[fwd] = true;
                        b[0] = t >= 10;
                    }
                },
                // Shoryuken: forward, down, down and forward and a punch.
                1 => match t {
                    0..=3 => d[fwd] = true,
                    4..=7 => d[DOWN] = true,
                    _ => {
                        d[DOWN] = true;
                        d[fwd] = true;
                        b[0] = t >= 10;
                    }
                },
                // Tatsumaki: down, down and back, back and a kick.
                _ => match t {
                    0..=3 => d[DOWN] = true,
                    4..=7 => {
                        d[DOWN] = true;
                        d[back] = true;
                    }
                    _ => {
                        d[back] = true;
                        b[3] = t >= 10;
                    }
                },
            }
            return (d, b);
        }
        if f % 7 == 0 {
            let r = self.rng.next();
            self.dirs = [r & 1 != 0, r & 2 != 0, r & 4 != 0, false];
            if self.dirs[0] && self.dirs[1] {
                self.dirs[self.back] = false;
            }
        }
        if f % 5 == 0 {
            let r = self.rng.next();
            self.buttons = [false; 6];
            if r & 3 == 0 {
                self.buttons[((r >> 4) % 6) as usize] = true;
            }
        }
        (self.dirs, self.buttons)
    }
}

/// Play `frames` fight frames (or until the fight is left), calling `look`
/// after each. Returns the frames played.
fn play(
    s: &mut GameSession,
    p1: TournamentFighter,
    p2: TournamentFighter,
    frames: u32,
    mut look: impl FnMut(&mut GameSession, &FrameOutput),
) -> u32 {
    let mut a = Pad::new(7 + tile(p1) as u64, p1.is_shoto(), RIGHT);
    let mut b = Pad::new(99 + tile(p2) as u64, p2.is_shoto(), LEFT);
    for f in 0..frames {
        let (d1, b1) = a.step(f);
        let (d2, b2) = b.step(f + 60);
        let out = s.step(with(|i| {
            i.p1.directions = d1;
            i.p1.buttons = b1;
            i.p2.directions = d2;
            i.p2.buttons = b2;
        }));
        if out.screen != "fight" {
            return f;
        }
        look(s, &out);
    }
    frames
}

// ---- the presented frame ---------------------------------------------------------

/// The second side's pixels in the presented frame are the mirror's: the
/// scene drawn with his sprites put back in his own palette differs from the
/// scene as it is in some pixels, and in each of those the presented frame
/// is the scene as it is, not the ROM's colours. Returns how many pixels it
/// checked.
fn presented_second_is_mirrored(rom: &Assets, m: &PowerOnMachine, f: TournamentFighter, second: u32, presented: &Frame) -> usize {
    let scene = m.state.scene();
    let as_is = render_frame(rom, &scene);
    let mut plain = scene.clone();
    let Some(p2) = region(m, second) else { return 0 };
    for at in p2..p2 + 24 * 0x40 {
        if (at - p2) % 0x40 == 2 {
            let attr = word(&plain.objectram, at);
            if attr & 0xf == slot(f) {
                plain.objectram[at..at + 2].copy_from_slice(&(attr & !0xf | u16::from(f.sprite_palette())).to_be_bytes());
            }
        }
    }
    let with_roms_colours = render_frame(rom, &plain);
    let mut checked = 0;
    for i in 0..as_is.argb.len() {
        if as_is.argb[i] != with_roms_colours.argb[i] {
            assert_eq!(presented.argb[i], as_is.argb[i], "pixel {i} of frame {} is not the mirror's", m.state.frame);
            checked += 1;
        }
    }
    checked
}

// ---- tests -------------------------------------------------------------------------

#[test]
fn local_ryu_against_ryu_draws_p2_in_his_mirror_palette_and_his_hadokens_as_the_rom_does() {
    let Some(a) = assets() else { return };
    let rom = load();
    let f = TournamentFighter::Ryu;
    let mut s = local_fight(a, f, f);
    assert_eq!(s.fight_machine().unwrap().match_config().preset, MatchConfig::default().preset);
    let mut seen = Seen::default();
    let mut checked_pixels = 0;
    let played = play(&mut s, f, f, 1_400, |s, out| {
        let m = s.fight_machine().unwrap();
        check_mirror_frame(&rom, m, f, P2_STRUCT, &mut seen);
        // Every 20th frame, the presented frame shows P2's pixels in the
        // mirror's colours.
        if m.state.frame % 20 == 0 {
            checked_pixels += presented_second_is_mirrored(&rom, m, f, P2_STRUCT, &out.frame);
        }
    });
    println!("{played} frames; P2 body {:?}, P2 effects {:?}, P1 effects {:?}", seen.p2_body, seen.p2_effects, seen.p1_effects);
    assert!(played > 1_000, "the fight ran {played} frames");
    // P1 stays in Ryu's own palette throughout; P2's body is the mirror slot.
    assert_eq!(seen.p1_body.keys().copied().collect::<Vec<_>>(), vec![0]);
    assert_eq!(seen.p2_body.keys().copied().collect::<Vec<_>>(), vec![14]);
    // Both sides threw Hadokens, and both are in the ROM's shared palettes:
    // projectiles are not recoloured.
    assert!(seen.p1_effects.keys().any(|p| *p == 2 || *p == 4), "P1 threw a Hadoken: {:?}", seen.p1_effects);
    assert!(seen.p2_effects.keys().any(|p| *p == 2 || *p == 4), "P2 threw a Hadoken: {:?}", seen.p2_effects);
    assert!(seen.p2_effects.keys().all(|p| [2, 4, 13].contains(p)), "P2 effects: {:?}", seen.p2_effects);
    assert!(checked_pixels > 3_000, "{checked_pixels} presented pixels were checked");
    let log = s.take_log();
    assert!(log.iter().all(|l| !l.contains("untranslated")), "{log:?}");
}

#[test]
fn local_mirrors_of_eleven_fighters_draw_p2_in_the_mirror_and_p1_as_the_rom_does() {
    let Some(a) = assets() else { return };
    for f in TournamentFighter::ALL {
        let mut s = local_fight(load(), f, f);
        let mut seen = Seen::default();
        let played = play(&mut s, f, f, 1_200, |s, _| check_mirror_frame(&a, s.fight_machine().unwrap(), f, P2_STRUCT, &mut seen));
        println!("{f:?}: {played} frames, P2 body {:?}, P2 effects {:?}, P1 effects {:?}", seen.p2_body, seen.p2_effects, seen.p1_effects);
        assert!(played > 400 && seen.frames > 300, "{f:?}: the fight ran {played} frames, {} with both on screen", seen.frames);
        assert!(!seen.p1_body.contains_key(&slot(f)), "{f:?}: P1 palettes {:?}", seen.p1_body);
        // The slot alone, but for Geki's teleport, whose flash frames are
        // in palette 4 and keep it (`mirror.rs`): the random fight reaches
        // one under some presets (true combos change its course).
        let other = |p: &u16| *p != slot(f) && !(f == TournamentFighter::Geki && *p == 4);
        assert!(seen.p2_body.contains_key(&slot(f)), "{f:?}: P2 drew in the slot: {:?}", seen.p2_body);
        assert!(!seen.p2_body.keys().any(other), "{f:?}: P2 drew in the slot alone: {:?}", seen.p2_body);
        if f.is_shoto() {
            // Both sides Hadokens flew, in the shared palettes.
            assert!(seen.p1_effects.keys().any(|p| *p == 2 || *p == 4), "{f:?}: P1 Hadoken: {:?}", seen.p1_effects);
            assert!(seen.p2_effects.keys().any(|p| *p == 2 || *p == 4), "{f:?}: P2 Hadoken: {:?}", seen.p2_effects);
        }
        assert!(s.take_log().iter().all(|l| !l.contains("untranslated")));
    }
}

/// Power-on to a Training fight, P1 `p1` (picked `p1_moves` tiles right of
/// Ryu) and the opponent the cursor sits on after the stick steps
/// `opp_moves` (the dummy cursor starts on Ken's tile, the CPU cursor on the
/// first live opponent), to the first frame of the fight.
fn training_fight(a: Assets, dummy: bool, p1: TournamentFighter, p1_moves: usize, opponent: TournamentFighter, opp_moves: &[usize]) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::default());
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Single Player
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "training");
    if dummy {
        tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    }
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)).screen, "select");
    for _ in 0..p1_moves {
        tap(&mut s, with(|i| i.p1.directions[RIGHT] = true));
    }
    assert_eq!(select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), tap).transition, Some(Transition::Picked(p1)));
    for &d in opp_moves {
        tap(&mut s, with(|i| i.p1.directions[d] = true));
    }
    let out = select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), GameSession::step);
    assert_eq!(out.transition, Some(Transition::Fighters(p1, opponent)));
    if dummy {
        until(&mut s, "country select", 60);
        tap(&mut s, with(|i| i.p1.buttons[0] = true));
    }
    until(&mut s, "fight", 60);
    s
}

/// Play a Training fight of `f` against himself for 1,000 frames, the
/// second side being the player structure `second`, checking every frame;
/// the second side must be in the mirror and the first in his own palette.
fn training_mirror(rom: &Assets, mut s: GameSession, f: TournamentFighter, second: u32) {
    assert_eq!(s.fight_machine().unwrap().match_config().roster.mode, PlayMode::Training);
    let mut seen = Seen::default();
    let mut checked_pixels = 0;
    let played = play(&mut s, f, f, 1_000, |s, out| {
        let m = s.fight_machine().unwrap();
        check_mirror_frame(rom, m, f, second, &mut seen);
        if m.state.frame % 20 == 0 {
            checked_pixels += presented_second_is_mirrored(rom, m, f, second, &out.frame);
        }
    });
    println!("{f:?} Training: {played} frames, second side {:?}, P1 {:?}", seen.p2_body, seen.p1_body);
    assert!(played > 500 && seen.frames > 300, "{f:?}: {played} frames, {} with both on screen", seen.frames);
    assert_eq!(seen.p2_body.keys().copied().collect::<Vec<_>>(), vec![slot(f)], "{f:?}: the second side");
    assert_eq!(seen.p1_body.keys().copied().collect::<Vec<_>>(), vec![u16::from(f.sprite_palette())], "{f:?}: P1");
    assert!(checked_pixels > 3_000, "{f:?}: {checked_pixels} presented pixels were checked");
    assert!(s.take_log().iter().all(|l| !l.contains("untranslated")));
}

#[test]
fn training_against_a_dummy_of_the_same_fighter_draws_the_dummy_in_the_mirror() {
    let Some(_) = assets() else { return };
    let rom = load();
    // Ryu's tile is the first and Ken's the second; the dummy cursor starts
    // on Ken's.
    for (f, p1_moves, opp) in [(TournamentFighter::Ryu, 0, vec![LEFT]), (TournamentFighter::Ken, 1, vec![])] {
        training_mirror(&rom, training_fight(load(), true, f, p1_moves, f, &opp), f, P2_STRUCT);
    }
}

#[test]
fn training_against_a_cpu_of_the_same_fighter_draws_the_cpu_in_the_mirror() {
    let Some(_) = assets() else { return };
    let rom = load();
    // Under an overhaul preset the fair CPU plays Player 2's side of a
    // two-player Training fight (SP21), so the CPU wears the mirror as P2.
    // Its cursor starts on Ken, Ryu's tile to his left.
    for (f, p1_moves, opp) in [(TournamentFighter::Ryu, 0, vec![LEFT]), (TournamentFighter::Ken, 1, vec![])] {
        training_mirror(&rom, training_fight(load(), false, f, p1_moves, f, &opp), f, P2_STRUCT);
    }
}

#[test]
fn joes_mirror_wears_his_palette_too_though_local_does_not_offer_it_yet() {
    let Some(a) = assets() else { return };
    let Some(loaded_art) = art(&a) else { return };
    let bank = std::sync::Arc::new(sf_engine::data_bank::DataBank::build(&a.maincpu, &loaded_art.catalogues));
    let f = TournamentFighter::Joe;
    let mut config = MatchConfig::default();
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = f;
    config.roster.p2 = f;
    let mut m = PowerOnMachine::new(a.maincpu.clone())
        .with_match_config(config)
        .with_data_bank(bank);
    drive_to_country_select(&mut m, Players::Two, 4_000).expect("country select");
    pick_country(&mut m, &a.maincpu, 0, 2_000).expect("the flag");
    while flow_position(&m) != FIGHT_LOOP {
        m.step();
    }
    let mut seen = Seen::default();
    let mut rng = Rng(5);
    for fr in 0..900u32 {
        // Joe's crouch reaches an untranslated branch (0x44506): no down.
        let r = rng.next();
        m.set_p1_direction(r & 1 != 0, false, false, false);
        m.set_p1_buttons(if fr % 9 < 2 { [true, false, false, false, false, false] } else { [false; 6] });
        m.set_p2_direction(false, r & 2 != 0, false, false);
        m.set_p2_buttons(if fr % 11 < 2 { [false, true, false, false, false, false] } else { [false; 6] });
        m.step();
        check_mirror_frame(&a, &m, f, P2_STRUCT, &mut seen);
    }
    assert!(m.unrecovered().is_empty(), "{:05x?}", m.unrecovered());
    assert!(seen.frames > 300);
    assert_eq!(seen.p1_body.keys().copied().collect::<Vec<_>>(), vec![5]);
    assert_eq!(seen.p2_body.keys().copied().collect::<Vec<_>>(), vec![14]);
}

#[test]
fn a_fight_of_two_different_fighters_draws_as_the_rom_does() {
    let Some(a) = assets() else { return };
    for (p1, p2) in [
        (TournamentFighter::Ryu, TournamentFighter::Ken),
        (TournamentFighter::Ken, TournamentFighter::Ryu),
        (TournamentFighter::Sagat, TournamentFighter::Gen),
        (TournamentFighter::Geki, TournamentFighter::Retsu),
    ] {
        let mut s = local_fight(load(), p1, p2);
        let mut frames = 0;
        let played = play(&mut s, p1, p2, 1_000, |s, _| {
            let m = s.fight_machine().unwrap();
            sprite_palettes_are_the_roms(&a, m);
            check_plain_frame(m, p1, p2);
            frames += 1;
        });
        assert!(played > 300 && frames == played, "{p1:?} v {p2:?}: {played} frames");
        assert!(s.take_log().iter().all(|l| !l.contains("untranslated")));
    }
}

#[test]
fn faithful_1987_never_applies_the_gate_whatever_the_roster_says() {
    let Some(a) = assets() else { return };
    // Under Faithful 1987 the roster is never read: a machine told both
    // sides are Ryu in Local is the same machine, frame for frame, as one
    // left on the 1987 defaults.
    let machine = |p1: TournamentFighter, p2: TournamentFighter| {
        let mut config = MatchConfig::faithful();
        config.roster = RosterSettings { mode: PlayMode::Versus, p1, p2, ..config.roster };
        assert!(!config.roster.full_cast, "Faithful 1987 does not bind the full cast");
        let mut m = PowerOnMachine::new(a.maincpu.clone()).with_match_config(config);
        drive_to_country_select(&mut m, Players::Two, 4_000).expect("country select");
        pick_country(&mut m, &a.maincpu, 0, 2_000).expect("the flag");
        while flow_position(&m) != FIGHT_LOOP {
            m.step();
        }
        m
    };
    let mut plain = machine(TournamentFighter::Ryu, TournamentFighter::Ken);
    let mut told = machine(TournamentFighter::Ryu, TournamentFighter::Ryu);
    let mut rng = Rng(3);
    for fr in 0..900u32 {
        let r = rng.next();
        for m in [&mut plain, &mut told] {
            m.set_p1_direction(r & 1 != 0, false, false, false);
            m.set_p1_buttons(if fr % 13 < 2 { [true, false, false, false, false, false] } else { [false; 6] });
            m.set_p2_direction(false, r & 2 != 0, false, false);
            m.set_p2_buttons(if fr % 17 < 2 { [false, false, true, false, false, false] } else { [false; 6] });
            m.step();
        }
        assert_eq!(plain.state.objectram, told.state.objectram, "frame {fr}");
        assert_eq!(plain.state.paletteram, told.state.paletteram, "frame {fr}");
        sprite_palettes_are_the_roms(&a, &told);
    }
    assert!(plain.state == told.state && plain.work_ram() == told.work_ram(), "the two machines end in one state");
}

#[test]
fn faithful_1987_with_full_cast_switched_on_still_draws_ryu_against_ryu_as_the_rom_does() {
    let Some(a) = assets() else { return };
    // The Roster tab can turn FULL CAST on under any preset. Under Faithful
    // 1987 it binds Ryu to both sides, and the gate must still not act: the
    // fight draws exactly as the ROM does. Its twin, the same configuration
    // under Custom (every toggle off, so the same 1987 rules), has the gate
    // on, and differs from it in the gate's bytes alone.
    let machine = |preset: RulesetPreset| {
        let mut config = MatchConfig::faithful();
        config.preset = preset;
        config.roster = RosterSettings { mode: PlayMode::Versus, full_cast: true, p1: TournamentFighter::Ryu, p2: TournamentFighter::Ryu, ..config.roster };
        let mut m = PowerOnMachine::new(a.maincpu.clone()).with_match_config(config);
        drive_to_country_select(&mut m, Players::Two, 4_000).expect("country select");
        pick_country(&mut m, &a.maincpu, 0, 2_000).expect("the flag");
        while flow_position(&m) != FIGHT_LOOP {
            m.step();
        }
        m
    };
    let mut faithful = machine(RulesetPreset::Faithful1987);
    let mut twin = machine(RulesetPreset::Custom);
    let mut gate_acted = false;
    let mut rng = Rng(9);
    for fr in 0..900u32 {
        let r = rng.next();
        for m in [&mut faithful, &mut twin] {
            m.set_p1_direction(r & 1 != 0, false, false, false);
            m.set_p1_buttons(if fr % 13 < 2 { [true, false, false, false, false, false] } else { [false; 6] });
            m.set_p2_direction(false, r & 2 != 0, false, false);
            m.set_p2_buttons(if fr % 17 < 2 { [false, false, true, false, false, false] } else { [false; 6] });
            m.step();
        }
        // Faithful 1987: the ROM's sprite palettes, and nothing in the slot.
        sprite_palettes_are_the_roms(&a, &faithful);
        check_plain_frame(&faithful, TournamentFighter::Ryu, TournamentFighter::Ryu);
        // The twin, with the gate's bytes put back, is the Faithful machine.
        let mut undone = twin.state.clone();
        for at in (0..undone.objectram.len()).step_by(0x40) {
            let attr = word(&undone.objectram, at + 2);
            if attr & 0xf == 14 {
                undone.objectram[at + 2..at + 4].copy_from_slice(&(attr & !0xf).to_be_bytes());
            }
        }
        gate_acted |= undone.objectram != twin.state.objectram;
        undone.paletteram[0x400 + 14 * 32..0x400 + 15 * 32].copy_from_slice(rom_palette(&a, 14));
        assert!(undone.objectram == faithful.state.objectram, "frame {fr}: object RAM");
        assert!(undone.paletteram == faithful.state.paletteram, "frame {fr}: palette RAM");
        assert!(faithful.work_ram() == twin.work_ram(), "frame {fr}: work RAM");
    }
    assert!(gate_acted, "the twin was recoloured, so the comparison could have failed");
}

#[test]
fn a_snapshot_taken_mid_fight_replays_the_mirror_exactly_as_the_fight_played_straight_through() {
    let Some(a) = assets() else { return };
    let f = TournamentFighter::Ryu;
    let mut config = MatchConfig::default();
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = f;
    config.roster.p2 = f;
    let mut played = PowerOnMachine::new(a.maincpu.clone()).with_match_config(config);
    drive_to_country_select(&mut played, Players::Two, 4_000).expect("country select");
    pick_country(&mut played, &a.maincpu, 0, 2_000).expect("the flag");
    while flow_position(&played) != FIGHT_LOOP {
        played.step();
    }
    // The inputs of the whole fight, fixed up front so the straight-through
    // run and the restored one are fed the same.
    let (mut p1, mut p2) = (Pad::new(21, true, RIGHT), Pad::new(22, true, LEFT));
    let inputs: Vec<_> = (0..700u32).map(|fr| (p1.step(fr), p2.step(fr + 60))).collect();
    let feed = |m: &mut PowerOnMachine, ((d1, b1), (d2, b2)): &(([bool; 4], [bool; 6]), ([bool; 4], [bool; 6]))| {
        m.set_p1_direction(d1[0], d1[1], d1[2], d1[3]);
        m.set_p1_buttons(*b1);
        m.set_p2_direction(d2[0], d2[1], d2[2], d2[3]);
        m.set_p2_buttons(*b2);
        m.step();
    };
    // Play on to a frame with both fighters on screen and the mirror drawn,
    // and snapshot there.
    let mut at = None;
    for (i, input) in inputs.iter().enumerate().take(300) {
        feed(&mut played, input);
        if i >= 200 && region(&played, P1_STRUCT).is_some() && region(&played, P2_STRUCT).is_some() {
            at = Some(i + 1);
            break;
        }
    }
    let at = at.expect("both fighters on screen mid-fight");
    let snapshot = played.save_snapshot();
    let mut restored = PowerOnMachine::new(a.maincpu.clone()).with_match_config(config);
    restored.restore_snapshot(&snapshot);
    assert!(restored.state == played.state, "the restored frame is the saved one");
    // The original plays on; the restored machine follows it frame for frame.
    let mut seen = Seen::default();
    for input in &inputs[at..] {
        feed(&mut played, input);
        feed(&mut restored, input);
        assert!(restored.state == played.state, "frame {}", played.state.frame);
        assert!(restored.work_ram() == played.work_ram(), "frame {} work RAM", played.state.frame);
        check_mirror_frame(&a, &restored, f, P2_STRUCT, &mut seen);
    }
    assert!(seen.frames > 200, "both fighters were on screen for {} of the {} replayed frames", seen.frames, inputs.len() - at);
    assert!(seen.p2_body.contains_key(&14));
}
