//! Ticket #215: the two-player front end's VS screens, built in the extended
//! background bank from the ROM (ADR-0009, first amendment).
//!
//! - Every built pair's screen is, pixel for pixel, the left fighter's bust
//!   block and name from a ROM screen where he stands on the left and the
//!   right fighter's from one where he stands on the right, each rendered in
//!   its own ROM palette; Ryu on the right and Ken on the left are their
//!   ROM busts mirrored. The rest is the RYU VS KEN screen's brackets, `VS`
//!   and backdrop. Ryu against a boss and a boss against Ken rebuild the
//!   ROM's own mode-2 and mode-4 screens.
//! - Through the headless game session a Local fight shows the picked pair
//!   (Sagat vs Geki; Retsu vs Mike, whose busts share a colour row in the
//!   ROM; a mirror match; Ken vs Ryu), Training against a dummy does too,
//!   and Ryu vs Ken is the ROM's `0x780` screen frame for frame. A Faithful
//!   1987 Local fight still shows RYU VS KEN.
//!
//! The expected pixels come from this file's own reading of the ROM tables,
//! not from the builder. Local ROMs only; each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_video::art::{load_art_from, Art, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::background_bank::{BackgroundBank, LayerView, MapLayer};
use sf_video::busts::FIGHTERS;
use sf_video::render::{render_frame, WIDTH};
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;
use sf_video::vs_screens::{vs_key, SCREENS_PER_PAGE};

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

fn art(a: &Assets) -> Option<Art> {
    let pack = ArtPack::open(Path::new(PACK)).ok()?;
    load_art_from(a, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).ok()
}
/// The fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;

/// The versus branch's screen (`0x3fa00`): RYU VS KEN, Ken on the right.
const VERSUS: (u16, usize) = (0x780, 0x1ae68);
/// Japan's mode-2 and mode-4 scroll tables and stage palettes.
const MODE_2: usize = 0x3fa66;
const MODE_4: usize = 0x3fad6;
const STAGE_PALETTES: usize = 0x43778;

const LEFT: usize = 0;
const RIGHT: usize = 1;
/// Screen pixels of each side's 96x96 bust block (rows 3..=8) and 64x32
/// name (rows 10..=11).
const BLOCK_X: [usize; 2] = [64, 224];
const BLOCK_Y: usize = 32;
const NAME_X: [usize; 2] = [96, 224];
const NAME_Y: usize = 144;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn vs_bank(a: &Assets) -> BackgroundBank {
    let mut bank = BackgroundBank::new();
    assert_eq!(bank.add_vs_screens(a), Ok(143));
    bank
}

fn word(m: &[u8], at: usize) -> usize {
    u16::from_be_bytes([m[at], m[at + 1]]) as usize
}

fn long(m: &[u8], at: usize) -> usize {
    word(m, at) << 16 | word(m, at + 2)
}

/// The ROM screen that shows fighter `i` on `side` or, for Ryu on the
/// right and Ken on the left, on his only side: (bg scroll, palette, the
/// side he stands on there).
fn rom_source(m: &[u8], i: usize, side: usize) -> (u16, usize, usize) {
    let screen = |table: usize, opponent: usize, side: usize| {
        let scroll = word(m, long(m, table) + 2 * opponent) as u16;
        let palette = long(m, long(m, STAGE_PALETTES) + 4 * (opponent / 2));
        (scroll, palette, side)
    };
    match (FIGHTERS[i], side) {
        ("Ryu", _) => screen(MODE_2, 0, LEFT),
        ("Ken", _) => (VERSUS.0, VERSUS.1, RIGHT),
        (_, LEFT) => screen(MODE_4, i - 2, LEFT),
        _ => screen(MODE_2, i - 2, RIGHT),
    }
}

/// The background layer alone at `page` and `scroll` in `palette`.
fn bg_frame(assets: &Assets, page: u8, scroll: u16, palette: &[u8]) -> Vec<u32> {
    let mut paletteram = vec![0u8; 0x800];
    paletteram[..0x200].copy_from_slice(palette);
    let scene = SceneState {
        videoram: vec![0; 0x1000],
        objectram: vec![0; 0x2000],
        paletteram,
        bg_scroll: scroll,
        fg_scroll: 0,
        active: 0x20,
        flip_screen: false,
        bg_page: page,
        fg_page: 0,
    };
    render_frame(assets, &scene).argb
}

fn rom_frame(rom: &Assets, scroll: u16, palette: usize) -> Vec<u32> {
    bg_frame(rom, 0, scroll, &rom.maincpu[palette..palette + 0x200])
}

fn built_frame(banked: &Assets, bank: &BackgroundBank, view: LayerView) -> Vec<u32> {
    bg_frame(banked, view.page, view.scroll, bank.palette(view.palette).unwrap())
}

/// Check the VS screen pixels `got` (a 384x224 frame) for `left` against
/// `right`: each side's bust block and name are its ROM source's, mirrored
/// when the fighter changes sides, in that source's palette; every other
/// pixel is the RYU VS KEN screen's.
fn check_pair(rom: &Assets, got: &[u32], left: usize, right: usize, what: &str) {
    let mut owned = vec![false; got.len()];
    for (side, i) in [(LEFT, left), (RIGHT, right)] {
        let (scroll, palette, from) = rom_source(&rom.maincpu, i, side);
        let src = rom_frame(rom, scroll, palette);
        let mut colours = std::collections::BTreeSet::new();
        for y in BLOCK_Y..BLOCK_Y + 96 {
            for x in 0..96 {
                let sx = BLOCK_X[from] + if from == side { x } else { 95 - x };
                let at = y * WIDTH + BLOCK_X[side] + x;
                assert_eq!(got[at], src[y * WIDTH + sx], "{what}: {}'s bust at ({}, {y})", FIGHTERS[i], BLOCK_X[side] + x);
                colours.insert(got[at]);
                owned[at] = true;
            }
        }
        assert!(colours.len() > 4, "{what}: {}'s bust draws a picture", FIGHTERS[i]);
        for y in NAME_Y..NAME_Y + 32 {
            for x in 0..64 {
                let at = y * WIDTH + NAME_X[side] + x;
                assert_eq!(got[at], src[y * WIDTH + NAME_X[from] + x], "{what}: {}'s name at ({}, {y})", FIGHTERS[i], NAME_X[side] + x);
                owned[at] = true;
            }
        }
    }
    let versus = rom_frame(rom, VERSUS.0, VERSUS.1);
    for (at, &mine) in owned.iter().enumerate() {
        if !mine {
            assert_eq!(got[at], versus[at], "{what}: the screen at ({}, {})", at % WIDTH, at / WIDTH);
        }
    }
}

#[test]
fn every_pair_but_ryu_vs_ken_is_built_from_its_rom_busts() {
    let Some(rom) = assets() else { return };
    let bank = vs_bank(&rom);
    let banked = rom.with_background_bank(&bank);
    assert_eq!(bank.pages(), 143usize.div_ceil(SCREENS_PER_PAGE));
    assert_eq!(bank.pages(), 2);
    let mut views = std::collections::BTreeSet::new();
    for (left, l) in FIGHTERS.iter().enumerate() {
        for (right, r) in FIGHTERS.iter().enumerate() {
            let key = vs_key(l, r);
            let Some(view) = bank.view(&key) else {
                assert_eq!((left, right), (0, 1), "{key}: every pair but Ryu vs Ken has a screen");
                continue;
            };
            assert!(views.insert((view.page, view.scroll)), "{key}: a screen of its own");
            assert!(view.scroll > 0 && view.scroll as u32 + 447 < 0x8000, "{key}: inside its page");
            check_pair(&rom, &built_frame(&banked, &bank, view), left, right, &key);
        }
    }
}

#[test]
fn ryu_against_a_boss_and_a_boss_against_ken_rebuild_the_rom_screens() {
    let Some(rom) = assets() else { return };
    let bank = vs_bank(&rom);
    let banked = rom.with_background_bank(&bank);
    let m = &rom.maincpu;
    for boss in 2..12 {
        let opponent = boss - 2;
        let palette = long(m, long(m, STAGE_PALETTES) + 4 * (opponent / 2));
        for (table, left, right) in [(MODE_2, 0, boss), (MODE_4, boss, 1)] {
            let key = vs_key(FIGHTERS[left], FIGHTERS[right]);
            let scroll = word(m, long(m, table) + 2 * opponent) as u16;
            let got = built_frame(&banked, &bank, bank.view(&key).unwrap());
            assert!(got == rom_frame(&rom, scroll, palette), "{key}: the ROM's own screen");
        }
    }
}

#[test]
fn a_shared_colour_row_is_moved_and_a_mirror_match_keeps_it() {
    let Some(rom) = assets() else { return };
    let bank = vs_bank(&rom);
    let colour = |key: &str, col: usize| {
        let v = bank.view(key).unwrap();
        let first = (v.scroll as usize + 64) / 16;
        bank.cell(v.page, MapLayer::Bg, first + col, 5).colour
    };
    // Retsu and Mike both draw with row 4 in the ROM, in other colours.
    assert_eq!((colour("vs:Retsu:Mike", 6), colour("vs:Mike:Retsu", 6)), (4, 4));
    assert_ne!(colour("vs:Retsu:Mike", 16), 4, "Mike moved off Retsu's row");
    assert_ne!(colour("vs:Mike:Retsu", 16), 4, "Retsu moved off Mike's row");
    // The same bust twice is the same row in the same colours.
    assert_eq!((colour("vs:Retsu:Retsu", 6), colour("vs:Retsu:Retsu", 16)), (4, 4));
}

// ---- through the game session ---------------------------------------------

const RIGHT_DIR: usize = 0;
const LEFT_DIR: usize = 1;
const DOWN_DIR: usize = 2;

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn tap(s: &mut GameSession, input: FrameInputs) -> &'static str {
    let out = s.step(input);
    s.step(idle());
    out.screen
}

fn session(rom: Assets, preset: RulesetPreset, banked: bool) -> GameSession {
    let loaded_art = art(&rom);
    let mut s = GameSession::new(rom, MatchConfig::from_preset(preset));
    if let Some(art) = &loaded_art {
        s = s.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
    }
    if banked {
        let bank = vs_bank(s.assets());
        s = s.with_background_bank(bank);
    }
    for _ in 0..200 {
        s.step(idle());
    }
    s
}

/// The select cursor from `from` to `to` (a 6x2 grid in [`FIGHTERS`]
/// order), by one player's stick.
fn walk(s: &mut GameSession, p2: bool, from: usize, to: usize) {
    let moves = (to / 6 > from / 6).then_some(DOWN_DIR).into_iter().chain(
        std::iter::repeat_n(if to % 6 > from % 6 { RIGHT_DIR } else { LEFT_DIR }, (to % 6).abs_diff(from % 6)),
    );
    for d in moves.collect::<Vec<_>>() {
        tap(s, with(|i| if p2 { i.p2.directions[d] = true } else { i.p1.directions[d] = true }));
    }
}

/// Main Menu, Multi Player, Local; P1 picks `p1` and P2 `p2`; Japan.
fn local(s: &mut GameSession, p1: usize, p2: usize) {
    tap(s, with(|i| i.menu.start = true));
    tap(s, with(|i| i.p1.directions[DOWN_DIR] = true));
    tap(s, with(|i| i.menu.confirm = true));
    assert_eq!(tap(s, with(|i| i.menu.confirm = true)), "select");
    walk(s, false, 0, p1);
    select::confirm_fighter(s, with(|i| i.p1.buttons[0] = true), tap);
    walk(s, true, 1, p2);
    select::confirm_fighter(s, with(|i| i.p2.buttons[0] = true), tap);
    country(s);
}

/// The country select's first flag, Japan.
fn country(s: &mut GameSession) {
    let mut f = 0;
    while s.screen() != "country select" {
        s.step(idle());
        f += 1;
        assert!(f < 600, "the country select (at {})", s.screen());
    }
    tap(s, with(|i| i.p1.buttons[0] = true));
}

/// Every frame from the country pick to the fight loop, with the frames
/// the VS screen holds (the background alone, its palette loaded) marked.
fn to_fight(s: &mut GameSession) -> Vec<(Vec<u32>, Option<SceneState>)> {
    let mut frames = Vec::new();
    let mut f = 0;
    loop {
        let out = s.step(idle());
        let m = s.fight_machine().unwrap();
        if flow_position(m) == FIGHT_LOOP {
            return frames;
        }
        let scene = m.state.scene();
        let vs = scene.active & 0x60 == 0x20;
        frames.push((out.frame.argb, vs.then_some(scene)));
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

/// A frame 60 into the VS screen's hold (120 frames, then 30, before the
/// stage palette request at `0x3fa4c`) and its scene: the longest run of
/// background-only frames.
fn vs_frame(frames: &[(Vec<u32>, Option<SceneState>)]) -> (&[u32], &SceneState) {
    let (mut best, mut run) = ((0, 0), 0);
    for (i, (_, scene)) in frames.iter().enumerate() {
        run = if scene.is_some() { run + 1 } else { 0 };
        if run > best.1 {
            best = (i + 1 - run, run);
        }
    }
    assert!(best.1 >= 120, "a VS screen held {} frames", best.1);
    let (frame, scene) = &frames[best.0 + 60];
    (frame, scene.as_ref().unwrap())
}

fn index(f: TournamentFighter) -> usize {
    FIGHTERS.iter().position(|&n| n == f.name()).unwrap()
}

/// Single Player, Arcade, the 3-column select, and the chosen country flag.
fn arcade(s: &mut GameSession, fighter: TournamentFighter, country: u8) {
    tap(s, with(|i| i.menu.start = true));
    tap(s, with(|i| i.menu.confirm = true));
    assert_eq!(tap(s, with(|i| i.menu.confirm = true)), "select");
    let slot = index(fighter);
    for d in std::iter::repeat_n(DOWN_DIR, slot / 3).chain(std::iter::repeat_n(RIGHT_DIR, slot % 3)) {
        tap(s, with(|i| i.p1.directions[d] = true));
    }
    select::confirm_fighter(s, with(|i| i.menu.confirm = true), tap);
    let mut f = 0;
    while s.screen() != "country select" {
        s.step(idle());
        f += 1;
        assert!(f < 600, "Arcade country select (at {})", s.screen());
    }
    assert_eq!(s.fight_machine().unwrap().match_config().roster.p1, fighter, "the selected Arcade player");
    let flag = s.assets().maincpu[0x3e2ea..0x3e2ee].iter().position(|&c| c == country).unwrap();
    if flag >= 2 {
        tap(s, with(|i| i.p1.directions[DOWN_DIR] = true));
    }
    if flag % 2 == 1 {
        tap(s, with(|i| i.p1.directions[RIGHT_DIR] = true));
    }
    tap(s, with(|i| i.p1.buttons[0] = true));
}

#[test]
fn arcade_vs_screen_shows_the_player_and_actual_ladder_opponent() {
    let Some(check) = assets() else { return };
    if art(&check).is_none() {
        eprintln!("skipping: local art pack not present");
        return;
    }
    for (player, country, opponent) in [
        (TournamentFighter::Lee, 1, TournamentFighter::Ryu),
        (TournamentFighter::Ken, 0, TournamentFighter::Retsu),
        (TournamentFighter::Lee, 0, TournamentFighter::Retsu),
    ] {
        let mut s = session(load_assets(ROMS, MANIFEST).unwrap(), RulesetPreset::DefinitiveOverhaul, true);
        arcade(&mut s, player, country);
        let frames = to_fight(&mut s);
        let (frame, _) = vs_frame(&frames);
        check_pair(&check, frame, index(player), index(opponent), &format!("Arcade {player:?} vs {opponent:?}"));
        let m = s.fight_machine().unwrap();
        assert_eq!(m.arcade_opponent(), Some(opponent));
        assert_eq!((m.state.fg_page, m.state.bg_page), (0, 0), "the fight returns to its ROM stage");
        let palette = long(&check.maincpu, long(&check.maincpu, STAGE_PALETTES + 4 * country as usize) + 4 * m.work_ram()[0x4003] as usize);
        assert_eq!(m.state.paletteram[..0x200], check.maincpu[palette..palette + 0x200], "the stage palette replaces the VS palette");
    }
}

#[test]
fn arcades_next_match_uses_the_new_ladder_opponent_and_restores_the_stage() {
    let Some(rom) = assets() else { return };
    let bank = vs_bank(&rom);
    let banked = rom.with_background_bank(&bank);
    // Ken is available without an art pack. Keep the ROM CPU for this
    // progression test, and play the first match with pad inputs only.
    let mut s = GameSession::new(load_assets(ROMS, MANIFEST).unwrap(), MatchConfig::definitive()).with_background_bank(bank);
    for _ in 0..200 {
        s.step(idle());
    }
    arcade(&mut s, TournamentFighter::Ken, 0);
    to_fight(&mut s);
    let start = s.fight_machine().unwrap();
    assert_eq!(start.match_config().roster.p1, TournamentFighter::Ken);
    let mut m = (0..40u32).find_map(|offset| {
        let mut m = start.clone();
        let mut inputs = offset;
        for _ in 0..12_000 {
            if m.work_ram()[0x4004] == 1 {
                return Some(m);
            }
            if [0x3f162, 0x42584].contains(&flow_position(&m)) {
                return None;
            }
            let fighting = flow_position(&m) == FIGHT_LOOP;
            inputs += u32::from(fighting);
            let c = inputs % 50;
            m.set_p1_direction(fighting && (2..8).contains(&c), false, fighting && c < 4, false);
            m.set_p1_buttons([fighting && (6..10).contains(&c), false, false, false, false, false]);
            m.step();
        }
        None
    }).expect("one offset of Ken's Hadoken cycle beats Retsu");
    m.set_p1_direction(false, false, false, false);
    m.set_p1_buttons([false; 6]);
    let mut held = 0;
    for f in 0..3_000 {
        m.step();
        let scene = m.state.scene();
        held = if scene.active & 0x60 == 0x20 { held + 1 } else { 0 };
        if held == 60 {
            assert_eq!(m.match_config().roster.p1, TournamentFighter::Ken);
            assert!(scene.bg_page > 0, "Ken/Geki uses a built VS page");
            let frame = render_frame(&banked, &scene);
            check_pair(&rom, &frame.argb, index(TournamentFighter::Ken), index(TournamentFighter::Geki), "Arcade's next match: Ken vs Geki");
            break;
        }
        assert!(f < 2_999, "the next match's VS screen");
    }
    for f in 0..1_000 {
        m.step();
        if flow_position(&m) == FIGHT_LOOP {
            assert_eq!(m.state.bg_page, 0, "the next fight returns to the ROM stage page");
            assert_eq!(m.arcade_opponent(), Some(TournamentFighter::Geki));
            assert!(m.unrecovered().is_empty());
            return;
        }
        assert!(f < 999, "the next fight loop");
    }
}

#[test]
fn local_fights_show_the_picked_pair() {
    let Some(check) = assets() else { return };
    for (p1, p2) in [("Sagat", "Geki"), ("Retsu", "Mike"), ("Mike", "Retsu"), ("Retsu", "Retsu"), ("Ken", "Ryu")] {
        let at = |n| FIGHTERS.iter().position(|&f| f == n).unwrap();
        let mut s = session(load_assets(ROMS, MANIFEST).unwrap(), RulesetPreset::DefinitiveOverhaul, true);
        local(&mut s, at(p1), at(p2));
        let roster = s.fight_machine().unwrap().match_config().roster;
        assert_eq!((roster.mode, roster.p1.name(), roster.p2.name()), (PlayMode::Versus, p1, p2));
        let frames = to_fight(&mut s);
        let (frame, scene) = vs_frame(&frames);
        let what = format!("Local {p1} vs {p2}");
        assert!(scene.bg_page > 0, "{what}: the VS screen is on a bank page");
        check_pair(&check, frame, at(p1), at(p2), &what);
        // The fight that follows is back on the ROM's page.
        assert_eq!(s.fight_machine().unwrap().state.bg_page, 0, "{what}: the fight's page");
    }
}

#[test]
fn ryu_vs_ken_in_local_is_the_rom_screen_frame_for_frame() {
    let Some(rom) = assets() else { return };
    let mut banked = session(rom, RulesetPreset::DefinitiveOverhaul, true);
    let mut plain = session(load_assets(ROMS, MANIFEST).unwrap(), RulesetPreset::DefinitiveOverhaul, false);
    local(&mut banked, 0, 1);
    local(&mut plain, 0, 1);
    let (a, b) = (to_fight(&mut banked), to_fight(&mut plain));
    assert_eq!(a.len(), b.len());
    for (f, (x, y)) in a.iter().zip(&b).enumerate() {
        assert!(x.0 == y.0, "frame {f}");
    }
    let (frame, scene) = vs_frame(&a);
    assert_eq!((scene.bg_page, scene.bg_scroll), (0, VERSUS.0));
    let check = load_assets(ROMS, MANIFEST).unwrap();
    check_pair(&check, frame, 0, 1, "Local Ryu vs Ken");
    assert!(banked.fight_machine().unwrap().background_bank().is_some());
}

#[test]
fn training_against_a_dummy_shows_the_picked_pair() {
    let Some(rom) = assets() else { return };
    let mut s = session(rom, RulesetPreset::DefinitiveOverhaul, true);
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Single Player
    tap(&mut s, with(|i| i.p1.directions[DOWN_DIR] = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)), "training");
    tap(&mut s, with(|i| i.p1.directions[DOWN_DIR] = true));
    assert_eq!(tap(&mut s, with(|i| i.menu.confirm = true)), "select");
    walk(&mut s, false, 0, 11);
    select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), tap); // Sagat
    tap(&mut s, with(|i| i.p1.directions[DOWN_DIR] = true));
    select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), GameSession::step); // the dummy
    country(&mut s);
    let roster = s.fight_machine().unwrap().match_config().roster;
    assert_eq!(roster.mode, PlayMode::Training);
    let (p1, p2) = (index(roster.p1), index(roster.p2));
    assert_ne!((p1, p2), (0, 1), "a built pair");
    let frames = to_fight(&mut s);
    let (frame, scene) = vs_frame(&frames);
    let what = format!("Training {} vs the dummy {}", FIGHTERS[p1], FIGHTERS[p2]);
    assert!(scene.bg_page > 0, "{what}: the VS screen is on a bank page");
    check_pair(&load_assets(ROMS, MANIFEST).unwrap(), frame, p1, p2, &what);
}

/// Faithful 1987 binds P1 to Ryu and P2 to Ken, and its fights never hold
/// the bank, so its versus branch is the ROM's whatever the session holds.
#[test]
fn a_faithful_1987_local_fight_shows_the_rom_screen_without_the_bank() {
    let Some(rom) = assets() else { return };
    let mut s = session(rom, RulesetPreset::Faithful1987, true);
    local(&mut s, 0, 1);
    assert!(s.background_bank().is_some());
    assert!(s.fight_machine().unwrap().background_bank().is_none());
    let frames = to_fight(&mut s);
    let (frame, scene) = vs_frame(&frames);
    assert_eq!((scene.bg_page, scene.bg_scroll), (0, VERSUS.0));
    check_pair(&load_assets(ROMS, MANIFEST).unwrap(), frame, 0, 1, "Faithful 1987 Ryu vs Ken");
}

#[test]
fn a_faithful_1987_arcade_fight_keeps_the_rom_intro_with_a_preloaded_bank() {
    let Some(rom) = assets() else { return };
    let mut banked = session(rom, RulesetPreset::Faithful1987, true);
    let mut plain = session(load_assets(ROMS, MANIFEST).unwrap(), RulesetPreset::Faithful1987, false);
    arcade(&mut banked, TournamentFighter::Ryu, 0);
    arcade(&mut plain, TournamentFighter::Ryu, 0);
    assert!(banked.background_bank().is_some());
    assert!(banked.fight_machine().unwrap().background_bank().is_none());
    let (a, b) = (to_fight(&mut banked), to_fight(&mut plain));
    assert_eq!(a.len(), b.len());
    for (f, (x, y)) in a.iter().zip(&b).enumerate() {
        assert!(x.0 == y.0, "Faithful Arcade frame {f}");
    }
}
