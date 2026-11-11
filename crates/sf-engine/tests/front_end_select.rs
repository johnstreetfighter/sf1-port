//! Ticket #155 (`FE5`): the portrait select finished.
//!
//! Pixel-asserting tests of the select through the headless game session,
//! no window: the twelve VS-screen busts decoded from the ROM, drawn 1:1
//! (and the fighters' names where a damaged tile set leaves no bust),
//! Local's VS layout with both players' busts and two cursors, Arcade's one
//! cursor, the greyed tiles of Arcade, the Classic Competitive ban list and
//! Faithful 1987, the mirror rule, and Faithful 1987's Local reaching the
//! two-country screen as Ryu against Ken. Local ROMs only; each test skips
//! without them.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::select::SelectScreen;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, Transition};
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art_from, Art, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::busts::{decode_busts, decode_select_busts, Bust};
use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::decode::decode_char_tile;
use sf_video::render::{Frame, WIDTH};
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

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
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

const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;

fn p1(dir: usize) -> FrameInputs {
    with(|i| i.p1.directions[dir] = true)
}

fn p2(dir: usize) -> FrameInputs {
    with(|i| i.p2.directions[dir] = true)
}

fn p1_lp() -> FrameInputs {
    with(|i| i.p1.buttons[0] = true)
}

fn p2_lp() -> FrameInputs {
    with(|i| i.p2.buttons[0] = true)
}

fn p1_lk() -> FrameInputs {
    with(|i| i.p1.buttons[3] = true)
}

/// The two layouts' geometry.
fn local_layout() -> SelectScreen {
    SelectScreen::local(&MatchConfig::default(), &TournamentFighter::ALL)
}

fn arcade_layout() -> SelectScreen {
    SelectScreen::arcade(&MatchConfig::default(), &TournamentFighter::ALL)
}

/// Power-on, 200 attract frames, Start, then Multi Player, Local (`local`)
/// or Single Player, Arcade.
fn to_select(a: Assets, config: MatchConfig, local: bool) -> (GameSession, Frame) {
    let loaded_art = art(&a);
    let mut s = GameSession::new(a, config);
    if let Some(art) = &loaded_art {
        s = s.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
    }
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    if local {
        tap(&mut s, p1(DOWN));
    }
    tap(&mut s, with(|i| i.menu.confirm = true));
    let out = tap(&mut s, with(|i| i.menu.confirm = true));
    assert_eq!(out.screen, "select");
    (s, out.frame)
}

fn px(frame: &Frame, x: usize, y: usize) -> u32 {
    frame.argb[y * WIDTH + x]
}

/// The select's colours, all from the ROM's boot text palette: the tiles'
/// gold, P1's yellow, P2's blue, the greyed tiles' dim.
struct Colors {
    gold: u32,
    p1: u32,
    p2: u32,
    grey: u32,
}

fn colors(a: &Assets) -> Colors {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    Colors {
        gold: font.pen(ink::GOLD_A, 2),
        p1: font.pen(ink::YELLOW, 0),
        p2: font.pen(ink::P2_BLUE, 2),
        grey: font.pen(ink::DIM, 2),
    }
}

/// The colour of tile `slot`'s outer border (top edge, a third along).
fn border(frame: &Frame, layout: &SelectScreen, slot: usize) -> u32 {
    let (x, y, _, _) = layout.tile_rect(slot);
    px(frame, x + 15, y)
}

/// The colour two pixels inside tile `slot`'s border (P2's cursor when it
/// shares P1's tile).
fn inner_border(frame: &Frame, layout: &SelectScreen, slot: usize) -> u32 {
    let (x, y, _, _) = layout.tile_rect(slot);
    px(frame, x + 15, y + 2)
}

/// Whether `text` is drawn from pixel (`x`, `y`) in the ROM's 8x8 font in
/// text colour `color` (glyph pixels only).
fn text_at(frame: &Frame, a: &Assets, x: usize, y: usize, text: &str, color: u8) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| {
            (0..8).all(|tx| tile[ty][tx] == 3 || px(frame, x + 8 * i + tx, y + ty) == font.pen(color, tile[ty][tx]))
        })
    })
}

/// The number of colours inside tile `slot`.
fn tile_colours(frame: &Frame, layout: &SelectScreen, slot: usize) -> usize {
    let (x, y, w, h) = layout.tile_rect(slot);
    let set: std::collections::BTreeSet<u32> = (y + 2..y + h - 2)
        .flat_map(|yy| (x + 2..x + w - 2).map(move |xx| (xx, yy)))
        .map(|(xx, yy)| px(frame, xx, yy))
        .collect();
    set.len()
}

/// Whether `side`'s large frame shows `bust` whole, pixel for pixel,
/// centred inside the frame's two-pixel border.
fn panel_shows(frame: &Frame, layout: &SelectScreen, side: usize, bust: &Bust) -> bool {
    let (x, y, w, h) = layout.panel_rect(side);
    let (ox, oy) = (x + 2 + (w - 4 - bust.width) / 2, y + 2 + (h - 4 - bust.height) / 2);
    bust.argb
        .iter()
        .enumerate()
        .all(|(i, &p)| px(frame, ox + i % bust.width, oy + i / bust.width) == p)
}

#[test]
fn the_busts_decode_from_the_rom_face_one_way_and_a_damaged_set_shows_names() {
    let Some(a) = assets() else { return };
    // All twelve decode; Ryu, drawn on the VS screen's left facing right,
    // is mirrored to face left like the other eleven.
    let select = decode_select_busts(&a);
    let as_drawn = decode_busts(&a);
    for i in 0..12 {
        let (s, d) = (select[i].as_ref().unwrap(), as_drawn[i].as_ref().unwrap());
        let expect = if i == 0 { d.mirrored().argb } else { d.argb.clone() };
        assert_eq!(s.argb, expect, "{:?}", TournamentFighter::ALL[i]);
    }
    // Arcade: every tile a picture; the large frame shows Ryu whole, 1:1.
    let (arcade, local) = (arcade_layout(), local_layout());
    let (_, frame) = to_select(load_assets(ROMS, MANIFEST).unwrap(), MatchConfig::default(), false);
    for slot in 0..12 {
        assert!(tile_colours(&frame, &arcade, slot) > 8, "tile {slot} shows its bust");
    }
    assert!(panel_shows(&frame, &arcade, 0, select[0].as_ref().unwrap()));
    // Local: both players' busts, whole and 1:1, P1's mirrored to face P2.
    let (_, frame) = to_select(load_assets(ROMS, MANIFEST).unwrap(), MatchConfig::default(), true);
    for slot in 0..12 {
        assert!(tile_colours(&frame, &local, slot) > 8, "tile {slot} shows its bust");
    }
    assert!(panel_shows(&frame, &local, 0, &select[0].as_ref().unwrap().mirrored()), "P1 Ryu");
    assert!(panel_shows(&frame, &local, 1, select[1].as_ref().unwrap()), "P2 Ken");

    // A damaged tile set: the background tiles zeroed, so every bust draws
    // nothing and fails to decode; each tile shows its fighter's name (as
    // much of it as the tile holds).
    let mut damaged = load_assets(ROMS, MANIFEST).unwrap();
    damaged.gfx1.iter_mut().for_each(|b| *b = 0);
    assert!(decode_select_busts(&damaged).iter().all(Option::is_none));
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let (_, frame) = to_select(damaged, MatchConfig::default(), true);
    for slot in 0..12 {
        let name: String = TournamentFighter::ALL[slot].name().to_ascii_uppercase().chars().take(5).collect();
        let (x, y) = local.name_origin(slot);
        assert!(text_at(&frame, &a, x, y, &name, ink::BLUE), "tile {slot} shows {name}");
    }
}

#[test]
fn local_shows_both_busts_and_two_cursors_that_move_at_once_share_tiles_lock_and_unlock() {
    let Some(a) = assets() else { return };
    let c = colors(&a);
    let busts = decode_select_busts(&a);
    let bust = |f: TournamentFighter| busts[TournamentFighter::ALL.iter().position(|&g| g == f).unwrap()].as_ref().unwrap();
    let l = local_layout();
    let (mut s, frame) = to_select(a, MatchConfig::default(), true);
    // P1 starts on Ryu (yellow), P2 on Ken (blue).
    assert_eq!(border(&frame, &l, 0), c.p1);
    assert_eq!(border(&frame, &l, 1), c.p2);
    // Both move on one frame: P1 right onto Ken, P2 down onto Gen; each
    // side's frame shows its own fighter.
    let out = tap(&mut s, with(|i| {
        i.p1.directions[RIGHT] = true;
        i.p2.directions[DOWN] = true;
    }));
    assert_eq!(out.transition, Some(Transition::CursorMoved));
    assert_eq!(border(&out.frame, &l, 1), c.p1);
    assert_eq!(border(&out.frame, &l, 7), c.p2);
    assert_eq!(border(&out.frame, &l, 0), c.gold);
    assert!(panel_shows(&out.frame, &l, 0, &bust(TournamentFighter::Ken).mirrored()));
    assert!(panel_shows(&out.frame, &l, 1, bust(TournamentFighter::Gen)));
    // Both on Gen: one tile, P1's border outside P2's.
    let out = tap(&mut s, p1(DOWN));
    assert_eq!((border(&out.frame, &l, 7), inner_border(&out.frame, &l, 7)), (c.p1, c.p2));
    assert!(panel_shows(&out.frame, &l, 0, &bust(TournamentFighter::Gen).mirrored()));
    tap(&mut s, p1(LEFT)); // P1 to Lee
    // P1 locks Lee: P1's stick no longer moves; back unlocks it.
    assert_eq!(tap(&mut s, p1_lp()).transition, Some(Transition::Picked(TournamentFighter::Lee)));
    assert_eq!(tap(&mut s, p1(RIGHT)).transition, None);
    assert_eq!(border(&s.step(idle()).frame, &l, 6), c.p1);
    assert_eq!(tap(&mut s, p1_lk()).transition, Some(Transition::Unpicked));
    assert_eq!(tap(&mut s, p1(UP)).transition, Some(Transition::CursorMoved)); // to Ryu
    // P2 locks Gen, then P1 locks Ryu: the screen moves on at once.
    assert_eq!(tap(&mut s, p2_lp()).transition, Some(Transition::Picked(TournamentFighter::Gen)));
    let out = s.step(p1_lp());
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, TournamentFighter::Gen)));
    assert_eq!(out.screen, "drive");
}

#[test]
fn arcade_has_one_cursor() {
    let Some(a) = assets() else { return };
    let c = colors(&a);
    let l = arcade_layout();
    let (mut s, _) = to_select(a, MatchConfig::default(), false);
    // Either stick drives the one cursor; there is no blue cursor.
    let out = tap(&mut s, p2(RIGHT));
    assert_eq!(out.transition, Some(Transition::CursorMoved));
    assert_eq!(border(&out.frame, &l, 1), c.p1);
    for slot in 0..12 {
        assert_ne!(border(&out.frame, &l, slot), c.p2);
        assert_ne!(inner_border(&out.frame, &l, slot), c.p2);
    }
}

#[test]
fn greyed_tiles_for_arcade_the_ban_list_and_faithful_1987_are_skipped() {
    let Some(a) = assets() else { return };
    let c = colors(&a);
    let caption = |frame: &Frame, l: &SelectScreen, slot: usize| {
        let (x, y, w, h) = l.tile_rect(slot);
        text_at(frame, &a, x + w - 2 - 32, y + h - 10, "SF1", ink::YELLOW)
    };
    // Arcade with the art pack: all twelve tiles open, none greyed.
    let arcade = arcade_layout();
    let (_, frame) = to_select(load_assets(ROMS, MANIFEST).unwrap(), MatchConfig::default(), false);
    for slot in 0..12 {
        assert_ne!(border(&frame, &arcade, slot), c.grey, "arcade tile {slot} open");
        assert!(!caption(&frame, &arcade, slot), "no SF1+ caption on open tile {slot}");
    }
    // Arcade without the art pack: the ten bosses greyed with the yellow
    // caption SF1+.
    let arcade_no_art = SelectScreen::arcade(&MatchConfig::default(), &[]);
    for slot in 0..2 {
        assert!(!arcade_no_art.greyed(slot));
    }
    for slot in 2..12 {
        assert!(arcade_no_art.greyed(slot), "boss tile {slot}");
    }
    // Arcade under Faithful 1987: only Ryu open; Ken and the ten bosses
    // greyed with no caption.
    let faithful = MatchConfig::from_preset(RulesetPreset::Faithful1987);
    let (_, frame) = to_select(load_assets(ROMS, MANIFEST).unwrap(), faithful, false);
    assert_ne!(border(&frame, &arcade, 0), c.grey, "Ryu open in Faithful 1987 Arcade");
    for slot in 1..12 {
        assert_eq!(border(&frame, &arcade, slot), c.grey, "tile {slot} greyed in Faithful 1987 Arcade");
        assert!(!caption(&frame, &arcade, slot));
    }

    // Classic Competitive with the Standard Tournament format chosen (it has
    // no ban list by default) and Balanced roster on, so Ryu's own rule
    // (banned for P1 while it is off) does not apply: Sagat banned for both
    // sides, greyed with no caption; the cursors skip him.
    let local = local_layout();
    let mut competitive = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive).with_balanced_roster(true);
    competitive.roster.ban = sf_engine::roster::BanFormat::StandardTournament;
    let (mut s, frame) = to_select(load_assets(ROMS, MANIFEST).unwrap(), competitive, true);
    assert_eq!(border(&frame, &local, 11), c.grey);
    assert!(!caption(&frame, &local, 11), "no caption on a banned tile");
    for slot in 0..11 {
        assert_ne!(border(&frame, &local, slot), c.grey);
    }
    for d in [DOWN, RIGHT, RIGHT, RIGHT, RIGHT] {
        tap(&mut s, p1(d)); // to Adon, beside Sagat
    }
    assert_eq!(tap(&mut s, p1(RIGHT)).transition, None, "Sagat skipped");

    // Faithful 1987: only Ryu and Ken; P1 is Ryu and P2 Ken, as the 1987
    // game binds them; the bosses greyed with no caption.
    let faithful = MatchConfig::from_preset(RulesetPreset::Faithful1987);
    let (mut s, frame) = to_select(load_assets(ROMS, MANIFEST).unwrap(), faithful, true);
    for slot in 2..12 {
        assert_eq!(border(&frame, &local, slot), c.grey);
        assert!(!caption(&frame, &local, slot));
    }
    assert_eq!(tap(&mut s, p1(RIGHT)).transition, None, "P1 is Ryu");
    assert_eq!(tap(&mut s, p2(LEFT)).transition, None, "P2 is Ken");
}

#[test]
fn a_mirror_match_is_offered_only_where_the_ladder_report_found_it_clean() {
    let Some(a) = assets() else { return };
    let c = colors(&a);
    let l = local_layout();
    let (mut s, _) = to_select(a, MatchConfig::default(), true);
    // With Joe on his 1987 player table (SP4 + SP13), his mirror ban is
    // gone: P1 locks Joe, P2 lands on Joe (sharing his tile) and locks Joe
    // too, and the fight runs clean.
    for _ in 0..4 {
        tap(&mut s, p1(RIGHT));
    }
    assert_eq!(tap(&mut s, p1_lp()).transition, Some(Transition::Picked(TournamentFighter::Joe)));
    tap(&mut s, p2(RIGHT)); // Retsu
    tap(&mut s, p2(RIGHT)); // Geki
    let out = tap(&mut s, p2(RIGHT)); // onto Joe
    assert_eq!(out.transition, Some(Transition::CursorMoved));
    assert_eq!((border(&out.frame, &l, 4), inner_border(&out.frame, &l, 4)), (c.p1, c.p2));
    let out = s.step(p2_lp());
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Joe, TournamentFighter::Joe)));
    // Ryu's mirror is clean too: both may lock Ryu.
    let (mut s, _) = to_select(load_assets(ROMS, MANIFEST).unwrap(), MatchConfig::default(), true);
    tap(&mut s, p1_lp());
    tap(&mut s, p2(LEFT));
    let out = s.step(p2_lp());
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, TournamentFighter::Ryu)));
}

#[test]
fn without_the_art_pack_the_ten_bosses_are_greyed_out_on_every_select_with_a_log_note_and_ryu_and_ken_still_play() {
    let Some(a) = assets() else { return };
    let c = colors(&a);
    let local = local_layout();
    // Local without `with_player_tables`: all ten bosses greyed out and
    // skipped by both cursors, the log explains why, and Ryu vs Ken still
    // plays a clean fight.
    let mut s = GameSession::new(load_assets(ROMS, MANIFEST).unwrap(), MatchConfig::default());
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, p1(DOWN));
    tap(&mut s, with(|i| i.menu.confirm = true));
    let out = tap(&mut s, with(|i| i.menu.confirm = true));
    assert_eq!(out.screen, "select");
    for slot in 0..2 {
        assert_ne!(border(&out.frame, &local, slot), c.grey);
    }
    for slot in 2..12 {
        assert_eq!(border(&out.frame, &local, slot), c.grey, "boss tile {slot} greyed without art pack");
    }
    let log = s.take_log();
    assert!(
        log.iter().any(|l| l.contains("Retsu") && l.contains("Sagat") && l.contains("greyed out")),
        "log explains why the bosses are greyed out: {log:?}"
    );
    assert_eq!(tap(&mut s, p1(DOWN)).transition, None, "second row skipped");
    tap(&mut s, p1_lp()); // P1 Ryu
    let out = s.step(p2_lp()); // P2 Ken
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, TournamentFighter::Ken)));
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, p1_lp());
    while s.fight_machine().map(flow_position) != Some(0x3eafa) {
        s.step(idle());
    }
    for _ in 0..120 {
        s.step(idle());
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Training CPU and Training Dummy without the art pack also grey out all
    // ten bosses and allow only Ryu and Ken.
    for cpu in [true, false] {
        let screen = SelectScreen::training(&MatchConfig::default(), cpu, &[]);
        for slot in 0..2 {
            assert!(!screen.greyed(slot));
        }
        for slot in 2..12 {
            assert!(screen.greyed(slot), "cpu={cpu} slot={slot}");
        }
    }
}

#[test]
fn faithful_1987_local_is_ryu_against_ken_on_the_two_country_screen() {
    let Some(a) = assets() else { return };
    let faithful = MatchConfig::from_preset(RulesetPreset::Faithful1987);
    let (mut s, _) = to_select(a, faithful, true);
    tap(&mut s, p1_lp());
    let out = s.step(p2_lp());
    assert_eq!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, TournamentFighter::Ken)));
    let mut f = 0;
    while s.screen() != "country select" {
        s.step(idle());
        f += 1;
        assert!(f < 40);
    }
    let m = s.fight_machine().unwrap();
    assert!(
        [0x3e110, 0x3e13a].contains(&flow_position(m)),
        "the two-country screen, at {:05x}",
        flow_position(m)
    );
    tap(&mut s, p1_lp());
    while flow_position(s.fight_machine().unwrap()) != 0x3eafa {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop");
    }
    let m = s.fight_machine().unwrap();
    let table = |ps: u32| {
        let a = (ps + 0x12 - 0xff8000) as usize;
        u32::from_be_bytes(m.work_ram()[a..a + 4].try_into().unwrap())
    };
    assert_eq!(table(P1_STRUCT), TournamentFighter::Ryu.move_table());
    assert_eq!(table(P2_STRUCT), TournamentFighter::Ken.move_table());
    assert!(m.unrecovered().is_empty());
}
