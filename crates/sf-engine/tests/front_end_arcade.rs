//! Ticket #156 (`FE6`): Arcade as Ryu or Ken.
//!
//! The headless game session walks Single Player, Arcade, the select (Ryu
//! and Ken, the bosses greyed), the one-player drive and the real
//! four-flag screen into the first fight from each country; and a whole
//! run: a won match, a lost one with name entry and the ranking, a free
//! continue on Start alone, a second loss whose countdown runs out, the
//! ROM's game over, the Main Menu where the ROM returned to its attract,
//! and the new entry in the attract's ranking table. The ladder beyond the
//! first fight is the full one-player scenario's (`full_1p.rs`). Local ROMs
//! only; each test skips without them.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::fighter::P1_STRUCT;
use sf_engine::front::cabinet::{chosen_country, flow_position};
use sf_engine::front::{FightDips, FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::roster::TournamentFighter;
use sf_engine::run::PowerOnMachine;
use sf_engine::front::select::SelectScreen;
use sf_video::compositor::{ink, RomFont};
use sf_video::render::WIDTH;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const GAME_MODE: u32 = 0xffc002;
const OPPONENT_NO: u32 = 0xffc004;
const FIGHT_LOOP: u32 = 0x3eafa;
const NAME_ENTRY_LOOP: u32 = 0x42584;
/// The ranking table (`0xff8e6e..0xff8f0e`, ten 16-byte entries; the
/// score's digits from `+5`).
const RANKING: u32 = 0xff8e6e;
/// The four-flag screen's country table (row × 2 + column).
const COUNTRY_TABLE4: usize = 0x3e2ea;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn rb(m: &PowerOnMachine, addr: u32) -> u8 {
    m.work_ram()[(addr - 0xff8000) as usize]
}

fn bytes(m: &PowerOnMachine, addr: u32, n: usize) -> Vec<u8> {
    let a = (addr - 0xff8000) as usize;
    m.work_ram()[a..a + n].to_vec()
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

fn stick(dir: usize) -> FrameInputs {
    with(|i| i.p1.directions[dir] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn lp() -> FrameInputs {
    with(|i| i.p1.buttons[0] = true)
}

/// Power-on, 200 attract frames, Start, Single Player, Arcade.
fn to_arcade_select(a: Assets) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::default());
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    assert_eq!(tap(&mut s, confirm()).screen, "single player");
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Arcade)));
    assert_eq!(out.screen, "select");
    s
}

fn until(s: &mut GameSession, name: &str, limit: u32, input: impl Fn(u32) -> FrameInputs) -> u32 {
    for f in 0..limit {
        if s.step(input(f)).screen == name {
            return f + 1;
        }
    }
    panic!("never reached {name} in {limit} frames (at {})", s.screen());
}

/// Pick `country` on the four-flag screen with the stick and LP, as a
/// player would.
fn pick_flag(s: &mut GameSession, rom: &[u8], country: u8) {
    let idx = (0..4).find(|&i| rom[COUNTRY_TABLE4 + i] == country).expect("country on the screen");
    s.step(idle());
    tap(s, stick(if idx >= 2 { DOWN } else { UP }));
    tap(s, stick(if idx % 2 == 1 { RIGHT } else { LEFT }));
    tap(s, lp());
}

/// Ryu's hadoken-style cycle (down, then down-forward, then forward with
/// light punch every 50 frames): it beats the first CPU opponent from
/// Japan 2-0 at factory difficulty.
fn hadoken(f: u32) -> FrameInputs {
    let c = f % 50;
    with(|i| {
        i.p1.directions[DOWN] = c < 4;
        i.p1.directions[RIGHT] = (2..8).contains(&c);
        i.p1.buttons[0] = (6..10).contains(&c);
    })
}

#[test]
fn the_arcade_select_offers_ryu_and_ken_and_greys_the_bosses() {
    let Some(a) = assets() else { return };
    let mut s = to_arcade_select(a);
    let out = s.step(idle());
    let grey = RomFont::new(&s.assets().gfx4, &s.assets().maincpu).pen(ink::DIM, 2);
    let layout = SelectScreen::arcade(&MatchConfig::default(), &[]);
    for slot in 0..12 {
        let (x, y, _, _) = layout.tile_rect(slot);
        let border = out.frame.argb[y * WIDTH + x + 16];
        let boss = slot >= 2;
        assert_eq!(border == grey, boss, "tile {slot} greyed: {boss}");
    }
    // Right moves to Ken; past Ken every tile is greyed, so nothing moves.
    assert_eq!(tap(&mut s, stick(RIGHT)).transition, Some(Transition::CursorMoved));
    for d in [RIGHT, DOWN] {
        let out = tap(&mut s, stick(d));
        assert_eq!(out.transition, None, "greyed tiles are skipped");
    }
    assert_eq!(tap(&mut s, stick(LEFT)).transition, Some(Transition::CursorMoved));
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Fighter(TournamentFighter::Ryu)));
    assert_eq!(out.screen, "drive");
}

#[test]
fn arcade_reaches_the_first_fight_from_each_country_as_ryu_and_ken() {
    let Some(a) = assets() else { return };
    let rom = a.maincpu.clone();
    for (fighter, moves) in [(TournamentFighter::Ryu, 0), (TournamentFighter::Ken, 1)] {
        for country in 0..4u8 {
            let mut s = to_arcade_select(load_assets(ROMS, MANIFEST).unwrap());
            // One run with the Gameplay tab's difficulty 3 and round time 200.
            let dips = (country == 3).then_some(FightDips { difficulty: 3, round_time: 200 });
            if let Some(d) = dips {
                s.set_fight_dips(d);
            }
            for _ in 0..moves {
                tap(&mut s, stick(RIGHT));
            }
            assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Fighter(fighter)));
            until(&mut s, "country select", 40, |_| idle());
            assert_eq!(rb(s.fight_machine().unwrap(), GAME_MODE), 2, "one-player game");
            pick_flag(&mut s, &rom, country);
            until(&mut s, "fight", 60, |_| idle());
            let mut f = 0;
            while flow_position(s.fight_machine().unwrap()) != FIGHT_LOOP {
                s.step(idle());
                f += 1;
                assert!(f < 2_000, "{fighter:?} from country {country}: no fight loop");
            }
            let m = s.fight_machine().unwrap();
            assert_eq!(chosen_country(m), country, "{fighter:?}: the flag picked");
            assert_eq!(m.match_config().preset, MatchConfig::default().preset);
            assert_eq!(m.match_config().roster.p1, fighter);
            // Boot's DSW2 decode: difficulty 3 (Difficult) to 2 and round
            // time 200 to 2; the factory words to 0 and 0.
            let decoded = if dips.is_some() { (2, 2) } else { (0, 0) };
            assert_eq!((rb(m, 0xffc007), rb(m, 0xff8e1c)), decoded, "difficulty, round time");
            assert_eq!(
                u32::from_be_bytes(bytes(m, P1_STRUCT + 0x12, 4).try_into().unwrap()),
                fighter.move_table(),
                "{fighter:?} bound on the human side"
            );
            assert!(m.unrecovered().is_empty());
        }
    }
}

#[test]
fn arcade_run_win_name_entry_free_continue_expiry_and_back_to_the_menu() {
    let Some(a) = assets() else { return };
    let mut s = to_arcade_select(a);
    let factory_ranking = bytes(s.attract_machine(), RANKING, 160);
    tap(&mut s, confirm()); // Ryu
    until(&mut s, "country select", 40, |_| idle());
    tap(&mut s, lp()); // Japan, the default flag
    until(&mut s, "fight", 60, |_| idle());

    // Beat Retsu: from a snapshot of the game at the first fight-loop
    // frame, try the hadoken cycle at successive offsets (the CPU's choices
    // follow the machine's random state) until one wins the match. Then
    // stand still against Geki and lose.
    while flow_position(s.fight_machine().unwrap()) != FIGHT_LOOP {
        s.step(idle());
    }
    let start = s.clone();
    let won = (0..40u32).find_map(|offset| {
        let mut t = start.clone();
        let (mut f, mut c) = (0u32, offset);
        loop {
            let m = t.fight_machine()?;
            if rb(m, OPPONENT_NO) == 1 {
                return Some((t, offset, f));
            }
            if f > 12_000 || [0x3f162, NAME_ENTRY_LOOP].contains(&flow_position(m)) {
                return None;
            }
            let fighting = flow_position(m) == FIGHT_LOOP;
            c += u32::from(fighting);
            t.step(if fighting { hadoken(c) } else { idle() });
            f += 1;
        }
    });
    let (won, offset, f) = won.expect("one offset of the hadoken cycle beats Retsu");
    s = won;
    let score = bytes(s.fight_machine().unwrap(), P1_STRUCT + 0x22, 7);
    println!("beat Retsu with offset {offset} after {f} frames, score {score:02x?}");
    // The loss's ranking check qualifies: name entry; three LP presses
    // commit AAA, then the ranking screen plays.
    let mut f = 0u32;
    while flow_position(s.fight_machine().unwrap()) != NAME_ENTRY_LOOP {
        s.step(idle());
        f += 1;
        assert!(f < 6_000, "name entry after the loss to Geki");
    }
    for _ in 0..3 {
        for _ in 0..24 {
            s.step(idle());
        }
        tap(&mut s, lp());
    }
    let frames = until(&mut s, "continue", 3_000, |_| idle());
    println!("continue countdown {frames} frames after the initials");
    // Start alone continues: no coin key is pressed.
    for _ in 0..60 {
        assert_eq!(s.step(idle()).screen, "continue");
    }
    s.step(with(|i| i.menu.start = true));
    until(&mut s, "fight", 120, |_| idle());
    let m = s.fight_machine().unwrap();
    assert_eq!(rb(m, OPPONENT_NO), 1, "the continue rematches Geki");
    // Lose again and let the countdown run out: game over, then the Main
    // Menu where the ROM returned to its attract.
    until(&mut s, "continue", 6_000, |_| idle());
    let frames = until(&mut s, "main menu", 3_000, |_| idle());
    println!("countdown and game over: {frames} frames");
    assert!(s.fight_machine().is_none());
    // The run's machine is now the attract, toggles off, with the entry.
    let m = s.attract_machine();
    assert_eq!(rb(m, 0xff8003), 0, "the ROM's attract");
    assert_eq!(*m.match_config(), MatchConfig::faithful());
    let ranking = bytes(m, RANKING, 160);
    assert_ne!(ranking, factory_ranking, "a new ranking entry");
    assert!(
        ranking.chunks(16).any(|e| e[5..12] == score[..]),
        "the run's score {score:02x?} is in the ranking table"
    );
    assert!(m.unrecovered().is_empty());
    // Back to the attract: it runs on through the wall, the title and its
    // ranking screen with the table it has.
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "attract");
    let mut f = 0;
    while flow_position(s.attract_machine()) != 0x42736 {
        s.step(idle());
        f += 1;
        assert!(f < 4_000, "the attract's ranking screen");
    }
    assert_eq!(bytes(s.attract_machine(), RANKING, 160), ranking);
    assert!(s.attract_machine().unrecovered().is_empty());
}

const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
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

fn to_arcade_select_with_art(a: Assets) -> Option<GameSession> {
    use sf_video::art::{load_art_from, ArtSources};
    use sf_video::art_pack::ArtPack;
    let pack = ArtPack::open(Path::new(PACK)).ok()?;
    let art = load_art_from(&a, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).ok()?;
    let mut s = GameSession::new(a, MatchConfig::default())
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    assert_eq!(tap(&mut s, confirm()).screen, "single player");
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Arcade)));
    assert_eq!(out.screen, "select");
    Some(s)
}

#[test]
fn with_the_art_pack_arcade_opens_all_twelve_and_ryu_stands_in_for_the_players_own_slot() {
    let Some(a) = assets() else { return };
    let rom = a.maincpu.clone();
    let Some(mut s) = to_arcade_select_with_art(a) else {
        eprintln!("skipping: local art pack not present");
        return;
    };
    let out = s.step(idle());
    let grey = RomFont::new(&s.assets().gfx4, &s.assets().maincpu).pen(ink::DIM, 2);
    let layout = SelectScreen::arcade(&MatchConfig::default(), &TournamentFighter::ALL);
    for slot in 0..12 {
        let (x, y, _, _) = layout.tile_rect(slot);
        let border = out.frame.argb[y * WIDTH + x + 16];
        assert_ne!(border, grey, "tile {slot} is open in Arcade with the art pack");
    }
    // Down twice from Ryu (cols = 3: row 0 Ryu, row 1 Geki, row 2 Lee) lands
    // on Lee. Confirm Lee and pick China (country 1).
    assert_eq!(tap(&mut s, stick(DOWN)).transition, Some(Transition::CursorMoved));
    assert_eq!(tap(&mut s, stick(DOWN)).transition, Some(Transition::CursorMoved));
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Fighter(TournamentFighter::Lee)));
    until(&mut s, "country select", 40, |_| idle());
    pick_flag(&mut s, &rom, 1);
    until(&mut s, "fight", 60, |_| idle());
    while flow_position(s.fight_machine().unwrap()) != FIGHT_LOOP {
        s.step(idle());
    }
    let m = s.fight_machine().unwrap();
    let p1_table = u32::from_be_bytes(bytes(m, P1_STRUCT + 0x12, 4).try_into().unwrap());
    let cpu_table = u32::from_be_bytes(bytes(m, 0xffc664 + 0x12, 4).try_into().unwrap());
    assert!(sf_engine::data_bank::in_bank(p1_table), "Lee plays on his 1987 bank table");
    assert_eq!(
        cpu_table,
        TournamentFighter::Ryu.move_table(),
        "Ryu stands in for Lee on Lee's own stage in China"
    );
    for _ in 0..120 {
        s.step(idle());
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

/// Ticket #267 (`SP15`): in a stand-in fight, Ryu brings his own theme
/// (`RYU_THEME`, `0x41`) when the extended tune table holds it; without it,
/// the stage's own 1987 theme plays and the log says why; a Jukebox choice
/// for the stage or fighter still overrides it.
#[test]
fn stand_in_fights_play_ryus_theme_fall_back_to_the_stage_theme_with_a_log_note_and_honour_the_jukebox() {
    use sf_engine::front::settings::Settings;
    use sf_engine::jukebox::{Jukebox, Resolved, TrackCue, TrackSource};
    use sf_video::background_bank::{BackgroundBank, RYU_THEME};

    let Some(a) = assets() else { return };
    let rom = a.maincpu.clone();
    let mut bank = BackgroundBank::new();
    bank.add_rom_stages(&a).expect("the 1987 stages read");
    let Some(base) = to_arcade_select_with_art(a) else {
        eprintln!("skipping: local art pack not present");
        return;
    };

    // 1. With Ryu's theme in the playable extended tunes, Lee's stand-in
    //    fight against Ryu on Lee's stage in China plays `RYU_THEME` (0x41)
    //    instead of China's 1987 stage theme (0x2e), with no fallback log.
    let run_lee_stand_in = |mut s: GameSession| -> (Vec<u8>, Vec<TrackCue>, Vec<String>) {
        s.take_log();
        assert_eq!(tap(&mut s, stick(DOWN)).transition, Some(Transition::CursorMoved));
        assert_eq!(tap(&mut s, stick(DOWN)).transition, Some(Transition::CursorMoved));
        let mut latches = Vec::new();
        let mut cues = Vec::new();
        let mut step = |s: &mut GameSession, i: FrameInputs| {
            let out = s.step(i);
            if out.sound_latch != 0xff {
                latches.push(out.sound_latch);
            }
            if let Some(cue) = out.track {
                cues.push(cue);
            }
            out.screen
        };
        step(&mut s, confirm());
        step(&mut s, idle());
        for _ in 0..40 {
            if step(&mut s, idle()) == "country select" {
                break;
            }
        }
        let idx = (0..4).find(|&i| rom[COUNTRY_TABLE4 + i] == 1).unwrap();
        step(&mut s, idle());
        step(&mut s, stick(if idx >= 2 { DOWN } else { UP }));
        step(&mut s, idle());
        step(&mut s, stick(if idx % 2 == 1 { RIGHT } else { LEFT }));
        step(&mut s, idle());
        step(&mut s, lp());
        step(&mut s, idle());
        for _ in 0..3_000 {
            step(&mut s, idle());
            if s.fight_machine().is_some_and(|m| flow_position(m) == FIGHT_LOOP) {
                break;
            }
        }
        for _ in 0..30 {
            step(&mut s, idle());
        }
        assert!(s.fight_machine().unwrap().unrecovered().is_empty());
        (latches, cues, s.take_log())
    };

    let mut with_theme = base.clone().with_background_bank(bank.clone());
    with_theme.set_playable_themes(&[RYU_THEME]);
    let (latches, cues, log) = run_lee_stand_in(with_theme);
    assert!(latches.contains(&RYU_THEME), "stand-in fight latches Ryu's theme (0x41): {latches:02x?}");
    assert!(!latches.contains(&0x2e), "China's 1987 stage theme (0x2e) is replaced: {latches:02x?}");
    assert!(cues.is_empty(), "no Jukebox cues without a track set: {cues:?}");
    assert!(
        log.iter().all(|l| !l.contains("Ryu's theme")),
        "no missing-theme log line when Ryu's theme is present: {log:?}"
    );

    // 2. Without Ryu's theme in the pack, the stage's own 1987 theme (0x2e)
    //    plays and the log says why.
    let mut without_theme = base.clone().with_background_bank(bank.clone());
    without_theme.set_playable_themes(&[]);
    let (latches, _, log) = run_lee_stand_in(without_theme);
    assert!(latches.contains(&0x2e), "without the pack, Lee's stage plays its 1987 theme (0x2e): {latches:02x?}");
    assert!(!latches.contains(&RYU_THEME), "missing extended theme is never sent: {latches:02x?}");
    assert!(
        log.iter().any(|l| l.contains("Ryu's theme (0x41) is not in the music pack") && l.contains("0x2e")),
        "the log explains why the stage's 1987 theme played: {log:?}"
    );

    // 3. A Jukebox choice for the stage ("lee") or fighter ("Lee" or "Ryu")
    //    overrides Ryu's theme as the Jukebox does today.
    for (set_track, expected) in [
        (
            (|j: &mut Jukebox| j.set_stage("lee", "great_wall.ogg")) as fn(&mut Jukebox),
            Resolved {
                file: "great_wall.ogg".into(),
                source: TrackSource::Stage("lee".into()),
            },
        ),
        (
            |j: &mut Jukebox| {
                j.set_stage("lee", "great_wall.ogg");
                j.set_fighter("Lee", "lee.ogg");
            },
            Resolved {
                file: "lee.ogg".into(),
                source: TrackSource::Character("Lee".into()),
            },
        ),
        (
            |j: &mut Jukebox| {
                j.set_stage("lee", "great_wall.ogg");
                j.set_fighter("Ryu", "ryu_cd.ogg");
            },
            Resolved {
                file: "ryu_cd.ogg".into(),
                source: TrackSource::Character("Ryu".into()),
            },
        ),
    ] {
        let mut jukebox = Jukebox::default();
        set_track(&mut jukebox);
        let mut s = base
            .clone()
            .with_settings(Settings { jukebox, ..Settings::default() })
            .with_background_bank(bank.clone());
        s.set_playable_themes(&[RYU_THEME]);
        let (_, cues, _) = run_lee_stand_in(s);
        assert!(
            cues.contains(&TrackCue::Prepare(expected.clone())),
            "Jukebox prepares {expected:?} ahead of the fight: {cues:?}"
        );
        assert!(
            cues.contains(&TrackCue::Play(expected.clone())),
            "Jukebox plays {expected:?} when the fight's theme latches: {cues:?}"
        );
    }
}
