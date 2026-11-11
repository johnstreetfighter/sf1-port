//! Ticket #154 (`FE4`): Local into a real fight.
//!
//! The cabinet driver is checked on the real power-on machine (coin and
//! Start pulses only, from several attract frames, for one and two
//! players), every tournament fighter on each side is bound and plays a
//! clean first round after the real country select, and the headless game
//! session walks Multi Player, Local, the select, the country screen, the
//! fight, the after-match box, a rematch and the way back to the attract.
//! Local ROMs only; each test skips without them.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{
    at_country_select, chosen_country, drive_to_country_select, flow_position, pick_country, Players,
    STAGE_START,
};
use sf_engine::front::{FightDips, FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::run::PowerOnMachine;
use sf_video::art::{load_art_from, Art, ArtSources};
use sf_video::art_pack::ArtPack;
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

fn expected_table(f: TournamentFighter) -> u32 {
    if f.is_shoto() {
        f.move_table()
    } else {
        let slot = TournamentFighter::ALL.iter().position(|&x| x == f).unwrap() as u32;
        0x10_0000 + slot * 0x1_0000
    }
}

const GAME_MODE: u32 = 0xffc002;
const DIFFICULTY: u32 = 0xffc007;
const TIME_SETTING: u32 = 0xff8e1c;
/// The game flow's fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;

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

fn rl(m: &PowerOnMachine, addr: u32) -> u32 {
    let a = (addr - 0xff8000) as usize;
    u32::from_be_bytes(m.work_ram()[a..a + 4].try_into().unwrap())
}

#[test]
fn the_driver_reaches_the_country_select_from_any_attract_phase() {
    let Some(a) = assets() else { return };
    // Power-on, the warning screen, the wall and title, the ranking, the
    // demo fight, the second loop.
    for from in [0u32, 400, 900, 1700, 2500, 4000] {
        for players in [Players::One, Players::Two] {
            let mut m = PowerOnMachine::new(a.maincpu.clone());
            for _ in 0..from {
                m.step();
            }
            let drive = drive_to_country_select(&mut m, players, 4_000)
                .unwrap_or_else(|e| panic!("{players:?} from frame {from}: {e:?}"));
            assert!(at_country_select(&m), "{players:?} from {from}");
            let mode = match players {
                Players::One => 2,
                Players::Two => 1,
            };
            assert_eq!(rb(&m, GAME_MODE), mode, "{players:?} from {from}: GAME_MODE");
            assert!(m.unrecovered().is_empty());
            assert!(drive.frames < 2_000, "{players:?} from {from}: {} frames", drive.frames);
            println!("{players:?} from frame {from}: country select after {} hidden frames", drive.frames);
        }
    }
}

/// Both players walk in and attack (no crouch, no jump): a light script
/// that exercises each side's bound move table.
fn round_script(f: u32) -> ([bool; 4], [bool; 6], [bool; 4], [bool; 6]) {
    let mut b1 = [false; 6];
    let mut b2 = [false; 6];
    if f % 20 < 3 {
        b1[((f / 20) % 6) as usize] = true;
    }
    if (f + 10) % 20 < 3 {
        b2[((f / 20 + 3) % 6) as usize] = true;
    }
    let walk = f % 40 < 25;
    ([walk, false, false, false], b1, [false, walk, false, false], b2)
}

#[test]
fn every_fighter_on_either_side_is_bound_and_plays_a_clean_first_round() {
    let Some(a) = assets() else { return };
    let Some(loaded_art) = art(&a) else { return };
    let bank = std::sync::Arc::new(sf_engine::data_bank::DataBank::build(&a.maincpu, &loaded_art.catalogues));
    let all = TournamentFighter::ALL;
    for (i, &fighter) in all.iter().enumerate() {
        let other = all[(i + 1) % all.len()];
        for (p1, p2) in [(fighter, other), (other, fighter)] {
            let mut cfg = MatchConfig::default();
            cfg.roster.mode = PlayMode::Versus;
            cfg.roster.p1 = p1;
            cfg.roster.p2 = p2;
            let mut m = PowerOnMachine::new(a.maincpu.clone())
                .with_match_config(cfg)
                .with_data_bank(bank.clone());
            drive_to_country_select(&mut m, Players::Two, 4_000).expect("country select");
            pick_country(&mut m, &a.maincpu, 0, 600).expect("stage start");
            assert_eq!(flow_position(&m), STAGE_START);
            let mut f = 0u32;
            while flow_position(&m) != FIGHT_LOOP {
                m.step();
                f += 1;
                assert!(m.unrecovered().is_empty(), "{p1:?} v {p2:?} pre-fight: {:05x?}", m.unrecovered());
                assert!(f < 1_500, "{p1:?} v {p2:?}: no fight loop");
            }
            assert_eq!(rl(&m, P1_STRUCT + 0x12), expected_table(p1), "{p1:?} bound on P1");
            assert_eq!(rl(&m, P2_STRUCT + 0x12), expected_table(p2), "{p2:?} bound on P2");
            // Play until the first round leaves the fight loop.
            let mut f = 0u32;
            while flow_position(&m) == FIGHT_LOOP && f < 8_000 {
                let (d1, b1, d2, b2) = round_script(f);
                m.set_p1_direction(d1[0], d1[1], d1[2], d1[3]);
                m.set_p1_buttons(b1);
                m.set_p2_direction(d2[0], d2[1], d2[2], d2[3]);
                m.set_p2_buttons(b2);
                m.step();
                f += 1;
                assert!(
                    m.unrecovered().is_empty(),
                    "{p1:?} v {p2:?} round 1 frame {}: {:05x?}",
                    m.state.frame,
                    m.unrecovered()
                );
            }
            assert_ne!(flow_position(&m), FIGHT_LOOP, "{p1:?} v {p2:?}: round 1 ended");
            assert_eq!(rl(&m, P1_STRUCT + 0x12), expected_table(p1));
            assert_eq!(rl(&m, P2_STRUCT + 0x12), expected_table(p2));
        }
    }
}

// ---- the headless session ---------------------------------------------------

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
const DOWN: usize = 2;

fn stick(dir: usize) -> FrameInputs {
    with(|i| i.p1.directions[dir] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn p2_stick(dir: usize) -> FrameInputs {
    with(|i| i.p2.directions[dir] = true)
}

fn p2_lp() -> FrameInputs {
    with(|i| i.p2.buttons[0] = true)
}

/// Power-on, 200 attract frames, Start, Multi Player, Local.
fn to_local_select(a: Assets) -> GameSession {
    let loaded_art = art(&a);
    let mut s = GameSession::new(a, MatchConfig::default());
    if let Some(art) = &loaded_art {
        s = s.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
    }
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, stick(DOWN));
    assert_eq!(tap(&mut s, confirm()).screen, "multi player");
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Local)));
    assert_eq!(out.screen, "select");
    s
}

/// Step until the screen is `name` (or `limit` frames pass), feeding
/// `input(frame)`; returns the frames stepped.
fn until(s: &mut GameSession, name: &str, limit: u32, input: impl Fn(u32) -> FrameInputs) -> u32 {
    for f in 0..limit {
        if s.step(input(f)).screen == name {
            return f + 1;
        }
    }
    panic!("never reached {name} in {limit} frames (at {})", s.screen());
}

/// P1 walks in, then stands and cycles the six buttons (a bound boss's
/// attacks do not fire while a direction is held); P2 stands.
fn p1_attacks(f: u32) -> FrameInputs {
    with(|i| {
        let phase = f % 120;
        i.p1.directions[RIGHT] = phase < 40;
        if phase >= 40 && phase % 10 < 3 {
            i.p1.buttons[((f / 10) % 6) as usize] = true;
        }
    })
}

#[test]
fn local_walk_sagat_against_gen_rematch_and_back_to_the_attract() {
    let Some(a) = assets() else { return };
    let mut s = to_local_select(a);
    let attract = s.attract_machine().save_snapshot();
    // Local's grid is two rows of six: Sagat is the last tile, Gen the
    // eighth (under Ken, where P2's cursor starts).
    for d in [DOWN, RIGHT, RIGHT, RIGHT, RIGHT, RIGHT] {
        assert_eq!(tap(&mut s, stick(d)).transition, Some(Transition::CursorMoved));
    }
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Picked(TournamentFighter::Sagat)));
    // P2 picks with P2's own stick and LP: from Ken down to Gen.
    tap(&mut s, p2_stick(DOWN));
    let out = s.step(p2_lp());
    assert_eq!(
        out.transition,
        Some(Transition::Fighters(TournamentFighter::Sagat, TournamentFighter::Gen))
    );
    assert_eq!(out.screen, "drive", "the hidden drive shows black");
    assert!(out.frame.argb.iter().all(|&p| p == 0xff00_0000));
    s.step(idle());
    until(&mut s, "country select", 30, |_| idle());
    let m = s.fight_machine().expect("the fight's machine");
    assert_eq!(rb(m, GAME_MODE), 1, "a two-player versus game");
    assert_eq!(m.match_config().preset, MatchConfig::default().preset);
    assert!(m.match_config().roster.full_cast);
    // P1 takes the flag with LP: the default, Japan.
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 30, |_| idle());
    let frames = until(&mut s, "after match", 20_000, p1_attacks);
    println!("first match over after {frames} frames");
    let m = s.fight_machine().unwrap();
    assert!(m.match_over(), "stopped before the winner carries on");
    assert_eq!(rl(m, P1_STRUCT + 0x12), expected_table(TournamentFighter::Sagat));
    assert_eq!(rl(m, P2_STRUCT + 0x12), expected_table(TournamentFighter::Gen));
    let country = chosen_country(m);
    // The box holds the fight: nothing steps while it is up.
    let frame = m.state.frame;
    for _ in 0..30 {
        assert_eq!(s.step(idle()).screen, "after match");
    }
    assert_eq!(s.fight_machine().unwrap().state.frame, frame);

    // REMATCH: same fighters, the flag replayed hidden, into the pre-fight.
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Rematch)));
    until(&mut s, "fight", 30, |_| idle());
    let m = s.fight_machine().unwrap();
    assert_eq!(flow_position(m), STAGE_START, "handed over at the stage start");
    assert_eq!(chosen_country(m), country, "the same flag");
    until(&mut s, "after match", 20_000, p1_attacks);
    let m = s.fight_machine().unwrap();
    assert_eq!(rl(m, P1_STRUCT + 0x12), expected_table(TournamentFighter::Sagat));
    assert_eq!(rl(m, P2_STRUCT + 0x12), expected_table(TournamentFighter::Gen));
    assert!(m.unrecovered().is_empty());

    // MENU: the Main Menu, the attract exactly as it was paused.
    tap(&mut s, stick(DOWN));
    tap(&mut s, stick(DOWN));
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Menu)));
    assert_eq!(out.screen, "main menu");
    assert!(s.fight_machine().is_none());
    assert_eq!(s.attract_machine().save_snapshot(), attract, "attract restored");
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "attract");
    for _ in 0..100 {
        s.step(idle());
    }
    assert!(s.attract_machine().unrecovered().is_empty());
}

#[test]
fn reselect_returns_to_the_select_and_the_attract_stays_as_it_was() {
    let Some(a) = assets() else { return };
    let mut s = to_local_select(a);
    let attract = s.attract_machine().save_snapshot();
    tap(&mut s, confirm()); // P1 Ryu
    tap(&mut s, p2_lp()); // P2 Ken, where P2's cursor starts
    until(&mut s, "country select", 40, |_| idle());
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "after match", 20_000, p1_attacks);
    tap(&mut s, stick(DOWN));
    let out = tap(&mut s, confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Reselect)));
    assert_eq!(out.screen, "select");
    assert!(s.fight_machine().is_none());
    assert_eq!(s.attract_machine().save_snapshot(), attract);
    // Back from the select (nothing locked) leaves for Multi Player.
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)).screen, "multi player");
}

#[test]
fn local_fights_use_the_configured_difficulty_and_round_time() {
    let Some(a) = assets() else { return };
    let mut s = to_local_select(a);
    s.set_fight_dips(FightDips { difficulty: 4, round_time: 150 });
    tap(&mut s, confirm());
    tap(&mut s, p2_lp());
    until(&mut s, "country select", 40, |_| idle());
    let m = s.fight_machine().unwrap();
    // Boot decodes DSW2 (0x3c132..0x3c152): difficulty 4 (Very Difficult)
    // to 3, round time 150 to 1.
    assert_eq!(rb(m, DIFFICULTY), 3);
    assert_eq!(rb(m, TIME_SETTING), 1);
    // The attract keeps the factory words: Normal and 100 both decode to 0.
    assert_eq!(rb(s.attract_machine(), DIFFICULTY), 0);
    assert_eq!(rb(s.attract_machine(), TIME_SETTING), 0);
}

#[test]
fn an_untranslated_branch_shows_the_notice_returns_to_the_menu_and_logs() {
    let Some(mut a) = assets() else { return };
    // Copy Joe's broken ROM move-5 header (0x1743c..0x1746c, whose stream
    // reaches 0x44506) onto Ryu's ROM move-5 header (0x1614c..0x1617c).
    let broken = a.maincpu[0x1743c..0x1746c].to_vec();
    a.maincpu[0x1614c..0x1617c].copy_from_slice(&broken);
    let mut s = to_local_select(a);
    tap(&mut s, confirm()); // P1 Ryu
    tap(&mut s, p2_lp()); // P2 Ken
    until(&mut s, "country select", 40, |_| idle());
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 30, |_| idle());
    let mut log = Vec::new();
    until(&mut s, "main menu", 3_000, |_| with(|i| i.p1.directions[DOWN] = true));
    log.extend(s.take_log());
    assert!(s.fight_machine().is_none());
    assert!(
        log.iter().any(|l| l.contains("0x44506") && l.contains("frame")),
        "logged with the frame and addresses: {log:?}"
    );
    let notice = s.notice().expect("a one-line notice").to_string();
    assert!(notice.contains("44506"), "{notice}");
}
