//! Ticket #200 (`ST8`): the Jukebox in real fights, at the session seam.
//!
//! The session says, frame by frame, what the host does with the
//! Jukebox's track (`FrameOutput::track`): `Prepare` as a fight starts,
//! `Play` where the fight latches its stage theme, `Stop` where the driver
//! would stop its music. These tests drive Local fights through the front
//! end and read those cues.
//!
//! - Character, then stage, then global; when both fighters have a track,
//!   the stage picker's (Player 1 on the first match, the loser after).
//! - The track stops for the win jingle and the round end, and when the
//!   fight is left, as native music would be.
//! - With no track set the session gives no cue, and Faithful 1987 never
//!   uses the Jukebox.
//!
//! Local ROMs only; each test skips without them.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, MenuItem};
use sf_engine::jukebox::{ends_music, Jukebox, Resolved, TrackCue, TrackSource, WIN_JINGLE};
use sf_video::background_bank::BackgroundBank;
use sf_video::romset::{load_assets, Assets};
use sf_video::stage::pack_records;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const TEST_STAGE: &str = include_str!("../../../art/stages/test-stage.json");
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;

fn have_roms() -> bool {
    let there = Path::new(ROMS).is_dir();
    if !there {
        eprintln!("skipping: local ROM set not present");
    }
    there
}

fn rom() -> Assets {
    load_assets(ROMS, MANIFEST).expect("local ROM set assembles")
}

fn bank(a: &Assets) -> BackgroundBank {
    let mut bank = pack_records(a, &[TEST_STAGE]).expect("the test stage packs");
    bank.add_rom_stages(a).expect("the 1987 stages read");
    bank.add_vs_screens(a).expect("the VS screens build");
    bank
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn down() -> FrameInputs {
    with(|i| i.p1.directions[DOWN] = true)
}

fn p2_confirm() -> FrameInputs {
    with(|i| i.p2.buttons[0] = true)
}

/// A session and everything it said.
#[derive(Clone)]
struct Rec {
    s: GameSession,
    /// (frame, cue) for every cue.
    cues: Vec<(u32, TrackCue)>,
    /// (frame, latch) for every non-idle latch.
    latches: Vec<(u32, u8)>,
}

impl Rec {
    /// A session in `preset` at the Main Menu, with `jukebox` set and the
    /// bank loaded. Draws are left as in 1987 (a drawn match has no
    /// winner), as the stage-pick tests have it.
    fn new(preset: RulesetPreset, jukebox: Jukebox) -> Rec {
        let config = MatchConfig::from_preset(preset).with_real_second_timer_and_draws(false);
        let a = rom();
        let b = bank(&a);
        let s = GameSession::new(a, config).with_settings(Settings { jukebox, ..Settings::default() }).with_background_bank(b);
        let mut r = Rec { s, cues: Vec::new(), latches: Vec::new() };
        for _ in 0..200 {
            r.step(idle());
        }
        r
    }

    fn step(&mut self, input: FrameInputs) -> &'static str {
        let out = self.s.step(input);
        if let Some(cue) = out.track {
            self.cues.push((out.sound_frame, cue));
        }
        if out.sound_latch != 0xff {
            self.latches.push((out.sound_frame, out.sound_latch));
        }
        out.screen
    }

    fn tap(&mut self, input: FrameInputs) -> &'static str {
        let screen = self.step(input);
        self.step(idle());
        screen
    }

    fn confirm_fighter(&mut self, input: FrameInputs) {
        if self.s.match_config().era_select_enabled() {
            self.tap(input);
        }
        self.tap(input);
    }

    /// The Main Menu to Local's select, with Ryu and Ken locked.
    fn to_locked_fighters(&mut self) {
        self.tap(with(|i| i.menu.start = true));
        self.tap(down());
        self.tap(confirm());
        assert_eq!(self.tap(confirm()), "select");
        self.confirm_fighter(confirm());
        self.confirm_fighter(p2_confirm());
    }

    /// The Main Menu to Training's select, against a dummy or the CPU.
    fn to_training_select(&mut self, dummy: bool) {
        self.tap(with(|i| i.menu.start = true));
        self.tap(confirm()); // Single Player
        self.tap(down());
        assert_eq!(self.tap(confirm()), "training");
        if dummy {
            self.tap(down());
        }
        assert_eq!(self.tap(confirm()), "select");
    }

    /// Training's select to its stage screen with Ryu and the opponent locked.
    fn lock_training(&mut self) {
        self.confirm_fighter(confirm());
        self.confirm_fighter(confirm());
        assert_eq!(self.s.screen(), "stage select");
    }

    /// The Main Menu to Arcade's first fight loop as Ryu from Japan.
    fn to_arcade_fight(&mut self) {
        self.tap(with(|i| i.menu.start = true));
        assert_eq!(self.tap(confirm()), "single player");
        assert_eq!(self.tap(confirm()), "select");
        self.confirm_fighter(confirm()); // Ryu
        for _ in 0..40 {
            if self.step(idle()) == "country select" {
                break;
            }
        }
        self.tap(with(|i| i.p1.buttons[0] = true)); // Japan, the default flag
        self.to_fight_loop(0);
    }

    /// The Main Menu to Local's stage screen with Ryu and Ken locked.
    fn to_stage_screen(&mut self) {
        self.to_locked_fighters();
        assert_eq!(self.s.screen(), "stage select");
    }

    /// Row `k` of the stage screen from row `from`, confirmed by the picker.
    fn pick(&mut self, from: usize, k: usize) {
        for _ in 0..(k + 11 - from) % 11 {
            self.tap(down());
        }
        self.step(confirm());
        self.step(idle());
    }

    /// The Main Menu (the attract's Start) to Settings, Sound, and the
    /// `JUKEBOX` row's screen, the way a player walks it.
    fn to_jukebox_screen(&mut self) {
        self.tap(with(|i| i.menu.start = true));
        self.tap(down());
        self.tap(down());
        self.tap(down());
        assert_eq!(self.tap(confirm()), "settings");
        for _ in 0..3 {
            self.tap(with(|i| i.menu.tab_right = true));
        }
        // MASTER, ANNOUNCER, FM, SAMPLES, JUKEBOX.
        for _ in 0..4 {
            self.tap(down());
        }
        assert_eq!(self.tap(confirm()), "jukebox");
    }

    /// Down `n` live rows on the Jukebox screen.
    fn jukebox_down(&mut self, n: usize) {
        for _ in 0..n {
            self.tap(down());
        }
    }

    /// Right `n` choices on the Jukebox screen's row.
    fn jukebox_right(&mut self, n: usize) {
        for _ in 0..n {
            self.tap(with(|i| i.p1.directions[RIGHT] = true));
        }
    }

    /// Back out of the Jukebox screen and Settings, which saves.
    fn leave_settings(&mut self) {
        assert_eq!(self.tap(with(|i| i.menu.back = true)), "settings");
        let out = self.s.step(with(|i| i.menu.back = true));
        assert_eq!((out.screen, out.settings_changed), ("main menu", true), "leaving Settings saves");
        self.step(idle());
    }

    /// The Main Menu, its cursor on SETTINGS, to Local's stage screen with
    /// Ryu and Ken locked.
    fn from_settings_to_stage_screen(&mut self) {
        for _ in 0..2 {
            self.tap(with(|i| i.p1.directions[3] = true));
        }
        assert_eq!(self.s.menu_item(), Some(MenuItem::MultiPlayer));
        self.tap(confirm());
        assert_eq!(self.tap(confirm()), "select");
        self.confirm_fighter(confirm());
        self.confirm_fighter(p2_confirm());
        assert_eq!(self.s.screen(), "stage select");
    }

    /// Step to the fight loop and `extra` frames past it.
    fn to_fight_loop(&mut self, extra: u32) {
        for n in 0..4_000 {
            self.step(idle());
            if self.s.fight_machine().is_some_and(|m| flow_position(m) == FIGHT_LOOP) {
                break;
            }
            assert!(n < 3_999, "the fight loop (at {})", self.s.screen());
        }
        for _ in 0..extra {
            self.step(idle());
        }
    }

    /// Play on until the after-match box: `side` attacks and wins.
    fn until_after_match(&mut self, side: usize) {
        for f in 0..80_000u32 {
            let input = with(|i| {
                let (pad, toward) = if side == 0 { (&mut i.p1, RIGHT) } else { (&mut i.p2, LEFT) };
                let phase = f % 120;
                pad.directions[toward] = phase < 40;
                if phase >= 40 && phase % 10 < 3 {
                    pad.buttons[((f / 10) % 6) as usize] = true;
                }
            });
            if self.step(input) == "after match" {
                return;
            }
        }
        panic!("the match end (at {})", self.s.screen());
    }

    /// The after-match or pause box's row `n`, confirmed.
    fn choose(&mut self, n: usize) -> &'static str {
        for _ in 0..n {
            self.tap(down());
        }
        self.tap(confirm())
    }

    fn plays(&self) -> Vec<(u32, &Resolved)> {
        self.cues.iter().filter_map(|(f, c)| if let TrackCue::Play(r) = c { Some((*f, r)) } else { None }).collect()
    }

    fn stops(&self) -> Vec<u32> {
        self.cues.iter().filter_map(|(f, c)| (*c == TrackCue::Stop).then_some(*f)).collect()
    }

    fn latch_at(&self, frame: u32) -> Option<u8> {
        self.latches.iter().find(|(f, _)| *f == frame).map(|(_, l)| *l)
    }
}

/// A Local fight, Ryu against Ken, on `stage` (a row of the stage screen),
/// up to the fight loop and a little past.
fn local_fight(preset: RulesetPreset, jukebox: Jukebox, stage: usize) -> Rec {
    let mut r = Rec::new(preset, jukebox);
    r.to_stage_screen();
    r.pick(2, stage);
    r.to_fight_loop(60);
    r
}

fn tracks(f: impl Fn(&mut Jukebox)) -> Jukebox {
    let mut j = Jukebox::default();
    f(&mut j);
    j
}

/// The file of the fight's first `Play`, and where it came from.
fn first_play(r: &Rec) -> Option<(String, TrackSource)> {
    r.plays().first().map(|(_, t)| (t.file.clone(), t.source.clone()))
}

/// Ryu's hadoken cycle (the Arcade tests'): it beats the first CPU opponent
/// from Japan 2-0 at factory difficulty.
fn hadoken(f: u32) -> FrameInputs {
    let c = f % 50;
    with(|i| {
        i.p1.directions[DOWN] = c < 4;
        i.p1.directions[RIGHT] = (2..8).contains(&c);
        i.p1.buttons[0] = (6..10).contains(&c);
    })
}

fn file(name: &str, source: TrackSource) -> Option<(String, TrackSource)> {
    Some((name.to_owned(), source))
}

#[test]
fn a_fight_resolves_character_then_stage_then_global() {
    if !have_roms() {
        return;
    }
    let overhaul = RulesetPreset::DefinitiveOverhaul;
    let gen = 5; // the stage the fight is on
    // Only the global track: every fight.
    let r = local_fight(overhaul, tracks(|j| j.set_global("all.ogg")), gen);
    assert_eq!(first_play(&r), file("all.ogg", TrackSource::Global));
    // A track for another stage does not apply.
    let r = local_fight(
        overhaul,
        tracks(|j| {
            j.set_global("all.ogg");
            j.set_stage("mike", "mike.ogg");
        }),
        gen,
    );
    assert_eq!(first_play(&r), file("all.ogg", TrackSource::Global), "Mike's stage is not Gen's");
    // This stage's beats the global.
    let r = local_fight(
        overhaul,
        tracks(|j| {
            j.set_global("all.ogg");
            j.set_stage("gen", "gen.flac");
        }),
        gen,
    );
    assert_eq!(first_play(&r), file("gen.flac", TrackSource::Stage("gen".into())));
    // A fighter's track beats the stage's: Ken's, who is Player 2 here.
    let r = local_fight(
        overhaul,
        tracks(|j| {
            j.set_global("all.ogg");
            j.set_stage("gen", "gen.flac");
            j.set_fighter("Ken", "ken.wav");
        }),
        gen,
    );
    assert_eq!(first_play(&r), file("ken.wav", TrackSource::Character("Ken".into())));
    // A fighter nobody is playing does not apply.
    let r = local_fight(
        overhaul,
        tracks(|j| {
            j.set_stage("gen", "gen.flac");
            j.set_fighter("Sagat", "sagat.wav");
        }),
        gen,
    );
    assert_eq!(first_play(&r), file("gen.flac", TrackSource::Stage("gen".into())));
}

#[test]
fn the_track_is_announced_when_the_fight_starts_and_starts_with_the_stage_theme() {
    if !have_roms() {
        return;
    }
    let r = local_fight(RulesetPreset::DefinitiveOverhaul, tracks(|j| j.set_fighter("Ryu", "ryu.ogg")), 2);
    let (first, cue) = &r.cues[0];
    let TrackCue::Prepare(prepared) = cue else { panic!("the first cue is the preparation: {cue:?}") };
    assert_eq!(prepared.file, "ryu.ogg");
    let plays = r.plays();
    assert_eq!(plays.len(), 1, "one round started: {:?}", r.cues);
    let (at, played) = plays[0];
    assert_eq!(played, prepared);
    assert!(at > *first, "ahead of the theme");
    // The cue is on the frame of Joe's stage theme and no other.
    let theme = r.latch_at(at).expect("a latch on the cue's frame");
    assert!(!ends_music(theme), "{theme:#04x}");
    assert_eq!(r.latches.iter().filter(|(_, l)| *l == theme).count(), 1);
}

#[test]
fn the_stage_pickers_character_track_wins_when_both_fighters_have_one() {
    if !have_roms() {
        return;
    }
    let both = tracks(|j| {
        j.set_stage("gen", "gen.ogg");
        j.set_fighter("Ryu", "ryu.ogg");
        j.set_fighter("Ken", "ken.ogg");
    });
    // Player 1 picks the first match: Ryu's.
    let mut r = Rec::new(RulesetPreset::DefinitiveOverhaul, both);
    r.to_stage_screen();
    r.pick(2, 5);
    r.to_fight_loop(60);
    assert_eq!(first_play(&r), file("ryu.ogg", TrackSource::Character("Ryu".into())), "the first match");
    // Player 1 wins, so Player 2 (Ken) picks the rematch: Ken's.
    r.until_after_match(0);
    assert_eq!(r.choose(0), "stage select", "REMATCH");
    r.cues.clear();
    assert_eq!(r.tap(p2_confirm()), "drive");
    r.to_fight_loop(60);
    assert_eq!(first_play(&r), file("ken.ogg", TrackSource::Character("Ken".into())), "the loser picks");
    // Player 2 wins that one, so Player 1 picks the next: Ryu's again.
    r.until_after_match(1);
    assert_eq!(r.choose(0), "stage select");
    r.cues.clear();
    assert_eq!(r.tap(confirm()), "drive");
    r.to_fight_loop(60);
    assert_eq!(first_play(&r), file("ryu.ogg", TrackSource::Character("Ryu".into())), "Player 2 won, so Player 1 picks");
}

#[test]
fn the_track_stops_for_the_round_end_and_the_win_jingle_as_native_music_does() {
    if !have_roms() {
        return;
    }
    let mut r = Rec::new(RulesetPreset::DefinitiveOverhaul, tracks(|j| j.set_global("all.ogg")));
    r.to_stage_screen();
    r.pick(2, 2);
    r.to_fight_loop(0);
    r.until_after_match(0);
    let plays = r.plays();
    assert!(plays.len() >= 2, "a theme per round: {:?}", r.cues);
    // Every stop is on a frame whose latch ends the driver's music.
    let stops = r.stops();
    assert!(!stops.is_empty());
    for frame in &stops {
        let latch = r.latch_at(*frame).expect("a latch");
        assert!(ends_music(latch), "{latch:#04x} at {frame}");
    }
    // Between one round's theme and the next, the round end stopped it.
    for pair in plays.windows(2) {
        assert!(stops.iter().any(|s| *s > pair[0].0 && *s < pair[1].0), "a stop between the rounds at {} and {}", pair[0].0, pair[1].0);
    }
    // The match ended on the win jingle; by then the round end had stopped
    // the track, so the jingle plays over nothing, as natively (the jingle
    // itself ending the track is the cue machine's unit test).
    let last_play = plays.last().unwrap().0;
    let jingle = r.latches.iter().find(|(f, l)| *f > last_play && *l == WIN_JINGLE).expect("the win jingle");
    assert!(stops.iter().any(|s| *s > last_play && *s <= jingle.0), "stopped by the jingle frame {}: {stops:?}", jingle.0);
    // And no track is left playing at the end: the last cue is a stop.
    assert_eq!(r.cues.last().map(|(_, c)| c), Some(&TrackCue::Stop));
}

#[test]
fn leaving_a_fight_stops_the_track() {
    if !have_roms() {
        return;
    }
    let mut r = local_fight(RulesetPreset::DefinitiveOverhaul, tracks(|j| j.set_global("all.ogg")), 2);
    assert_eq!(r.plays().len(), 1);
    assert!(r.stops().is_empty());
    // The pause box keeps the music going, as it does on the native side.
    assert_eq!(r.tap(with(|i| i.menu.start = true)), "pause");
    for _ in 0..30 {
        r.step(idle());
    }
    assert!(r.stops().is_empty(), "paused: {:?}", r.cues);
    assert_eq!(r.choose(5), "main menu", "QUIT TO MENU");
    assert_eq!(r.stops().len(), 1, "{:?}", r.cues);
}

#[test]
fn with_no_track_for_the_fight_the_session_gives_no_cue() {
    if !have_roms() {
        return;
    }
    // A track for another stage does not count: native music, untouched.
    let mut r = Rec::new(RulesetPreset::DefinitiveOverhaul, tracks(|j| j.set_stage("sagat", "s.ogg")));
    r.to_stage_screen();
    r.pick(2, 2);
    r.to_fight_loop(0);
    r.until_after_match(0);
    assert!(r.cues.is_empty(), "{:?}", r.cues);
    assert!(r.latches.iter().any(|(_, l)| *l == WIN_JINGLE), "the native music ran");
}

#[test]
fn faithful_1987_never_uses_the_jukebox() {
    if !have_roms() {
        return;
    }
    let all = tracks(|j| {
        j.set_global("all.ogg");
        j.set_fighter("Ryu", "ryu.ogg");
        j.set_fighter("Ken", "ken.ogg");
    });
    let mut r = Rec::new(RulesetPreset::Faithful1987, all);
    r.to_locked_fighters();
    // The flag screen of the 1987 flow, then the fight.
    for _ in 0..40 {
        r.step(idle());
    }
    r.step(with(|i| i.p1.buttons[0] = true));
    r.to_fight_loop(120);
    assert!(r.s.fight_machine().is_some(), "a fight is running");
    assert!(r.latches.iter().any(|(_, l)| !ends_music(*l)), "the native music ran");
    assert!(r.cues.is_empty(), "{:?}", r.cues);
}

/// The opponent counter of Arcade's ladder (`0xffc004`).
const OPPONENT_NO: u32 = 0xffc004;

fn opponent_no(r: &Rec) -> u8 {
    r.s.fight_machine().map_or(0xff, |m| m.work_ram()[(OPPONENT_NO - 0xff8000) as usize])
}

#[test]
fn arcade_plays_the_ladders_stage_track_announced_ahead_with_nothing_decoded_at_the_theme() {
    if !have_roms() {
        return;
    }
    // The test stage plays the same fallback tune as Geki's stage (no tune
    // table is given here), so the theme alone cannot say which is which;
    // the ladder does.
    let set = tracks(|j| {
        j.set_global("all.ogg");
        j.set_stage("retsu", "retsu.ogg");
        j.set_stage("geki", "geki.ogg");
        j.set_stage("test-stage", "test.ogg");
        j.set_fighter("Sagat", "sagat.ogg");
    });
    let mut r = Rec::new(RulesetPreset::DefinitiveOverhaul, set);
    r.to_arcade_fight();
    r.to_fight_loop(60);
    // Retsu: announced, then played, the same track, the stage's.
    let retsu = Resolved { file: "retsu.ogg".into(), source: TrackSource::Stage("retsu".into()) };
    let prepares: Vec<_> = r.cues.iter().filter_map(|(f, c)| if let TrackCue::Prepare(t) = c { Some((*f, t.clone())) } else { None }).collect();
    assert_eq!(prepares.iter().map(|(_, t)| t).collect::<Vec<_>>(), [&retsu], "the global track is never announced for Retsu: {:?}", r.cues);
    assert_eq!(r.plays().iter().map(|(_, t)| (*t).clone()).collect::<Vec<_>>(), [retsu.clone()]);
    assert!(prepares[0].0 < r.plays()[0].0, "announced ahead of the theme");

    // Beat Retsu (the cycle offset that wins follows the machine's random
    // state, so try them from a snapshot), then Geki is next on the ladder.
    let start = r.clone();
    let won = (0..40u32).find_map(|offset| {
        let mut t = start.clone();
        let (mut f, mut c) = (0u32, offset);
        loop {
            if opponent_no(&t) == 1 {
                return Some(t);
            }
            let in_loop = t.s.fight_machine().is_some_and(|m| flow_position(m) == FIGHT_LOOP);
            if f > 12_000 || t.s.fight_machine().is_none() {
                return None;
            }
            c += u32::from(in_loop);
            t.step(if in_loop { hadoken(c) } else { idle() });
            f += 1;
        }
    });
    let mut r = won.expect("one offset of the hadoken cycle beats Retsu");
    for _ in 0..3_000 {
        r.step(idle());
        if r.plays().iter().any(|(_, t)| t.file == "geki.ogg") {
            break;
        }
    }
    let geki = Resolved { file: "geki.ogg".into(), source: TrackSource::Stage("geki".into()) };
    let plays: Vec<_> = r.plays().iter().map(|(_, t)| (*t).clone()).collect();
    assert!(plays.contains(&geki), "Geki's stage track plays, not the test stage's: {plays:?}");
    assert!(!plays.iter().any(|t| t.file == "test.ogg" || t.file == "all.ogg"), "{plays:?}");
    let announced = r.cues.iter().position(|(_, c)| *c == TrackCue::Prepare(geki.clone())).expect("Geki's track is announced");
    let played = r.cues.iter().position(|(_, c)| *c == TrackCue::Play(geki.clone())).unwrap();
    assert!(announced < played, "announced ahead of his theme");
}

#[test]
fn training_plays_the_picker_s_character_then_the_stage_track() {
    if !have_roms() {
        return;
    }
    let overhaul = RulesetPreset::DefinitiveOverhaul;
    // Against a dummy, Player 1 is at the controls: Ryu's track wins over the
    // dummy's, and with only the dummy's, the dummy's applies.
    let mut r = Rec::new(
        overhaul,
        tracks(|j| {
            j.set_fighter("Ryu", "ryu.ogg");
            j.set_fighter("Ken", "ken.ogg");
            j.set_stage("gen", "gen.ogg");
        }),
    );
    r.to_training_select(true);
    r.lock_training();
    r.pick(2, 5);
    r.to_fight_loop(60);
    assert_eq!(first_play(&r), file("ryu.ogg", TrackSource::Character("Ryu".into())));
    let mut r = Rec::new(overhaul, tracks(|j| j.set_fighter("Ken", "ken.ogg")));
    r.to_training_select(true);
    r.lock_training();
    r.pick(2, 5);
    r.to_fight_loop(60);
    assert_eq!(first_play(&r), file("ken.ogg", TrackSource::Character("Ken".into())));
    // Against the CPU, with no fighter track, the stage's: the cursor starts
    // on the opponent's own stage, and one press keeps it. The fair CPU's
    // first live opponent is Ken (SP21), whose stage is Joe's
    // (`home_owner`).
    let mut r = Rec::new(
        overhaul,
        tracks(|j| {
            j.set_stage("retsu", "retsu.ogg");
            j.set_stage("joe", "joe.ogg");
        }),
    );
    r.to_training_select(false);
    r.lock_training();
    assert_eq!(r.tap(confirm()), "drive");
    r.to_fight_loop(60);
    assert_eq!(first_play(&r), file("joe.ogg", TrackSource::Stage("joe".into())));
    assert!(r.stops().is_empty());
}

/// The rows of the Jukebox screen as the session builds them: the stage
/// the fight is on and the fighter, by their place among the live rows
/// (the global track is row 0, then the stages, then the fighters).
fn jukebox_places(stage: &str, fighter: &str) -> (usize, usize) {
    let a = rom();
    let b = bank(&a);
    let (stages, fighters) = sf_engine::front::jukebox::entries(Some(&b), false, 0);
    let stage = stages.iter().position(|e| e.key == stage).expect("a listed stage");
    let fighter = fighters.iter().position(|e| e.key == fighter).expect("a fighter");
    (1 + stage, 1 + stages.len() + fighter)
}

/// A session at the Main Menu under the Definitive Overhaul preset with the
/// host's `jukebox` folder listed as `files`.
fn with_folder(files: &[&str]) -> Rec {
    let mut r = Rec::new(RulesetPreset::DefinitiveOverhaul, Jukebox::default());
    r.s.set_tracks(files.iter().map(|f| f.to_string()).collect());
    r
}

#[test]
fn a_stage_set_on_the_jukebox_screen_is_the_next_fights_track_and_is_saved() {
    if !have_roms() {
        return;
    }
    let gen = 5;
    let (stage_row, _) = jukebox_places("gen", "Ken");
    assert_eq!(stage_row, 1 + gen, "Gen is the sixth stage, as the stage screen has it");
    // The folder holds two tracks (and files the Jukebox does not play).
    let mut r = with_folder(&["ken.wav", "gen.ogg", "readme.txt", "song.mp3"]);
    r.to_jukebox_screen();
    r.jukebox_down(stage_row);
    // NATIVE, then gen.ogg, then ken.wav.
    r.jukebox_right(1);
    r.leave_settings();
    let saved = r.s.settings().clone();
    assert_eq!(saved.jukebox.stage("gen"), Some("gen.ogg"));
    assert_eq!(saved.jukebox.stage("retsu"), None, "no other row changed");
    // The settings file keeps it: a restart (a session started on what the file says) has it.
    assert_eq!(Settings::from_toml(&saved.to_toml()).as_ref(), Ok(&saved));
    // The next fight, on that stage, plays that file as the stage's track.
    r.from_settings_to_stage_screen();
    r.pick(2, gen);
    r.to_fight_loop(60);
    assert_eq!(first_play(&r), file("gen.ogg", TrackSource::Stage("gen".into())));
}

#[test]
fn a_fighter_set_on_the_jukebox_screen_is_the_next_fights_track() {
    if !have_roms() {
        return;
    }
    let gen = 5;
    let (stage_row, ken_row) = jukebox_places("gen", "Ken");
    let mut r = with_folder(&["gen.ogg", "ken.wav"]);
    r.to_jukebox_screen();
    // Gen's stage to gen.ogg, then Ken (Player 2 here) to ken.wav: the
    // fighter's wins over the stage's.
    r.jukebox_down(stage_row);
    r.jukebox_right(1);
    r.jukebox_down(ken_row - stage_row);
    r.jukebox_right(2);
    r.leave_settings();
    assert_eq!(r.s.settings().jukebox.fighter("Ken"), Some("ken.wav"));
    r.from_settings_to_stage_screen();
    r.pick(2, gen);
    r.to_fight_loop(60);
    assert_eq!(first_play(&r), file("ken.wav", TrackSource::Character("Ken".into())));
}

#[test]
fn native_set_on_the_jukebox_screen_overrides_the_global_track_in_the_next_fight() {
    if !have_roms() {
        return;
    }
    let gen = 5;
    let (_, ken_row) = jukebox_places("gen", "Ken");
    // The global track alone plays in every fight.
    let mut r = with_folder(&["all.ogg", "ken.wav"]);
    r.to_jukebox_screen();
    r.jukebox_right(1);
    assert_eq!(r.s.screen(), "jukebox");
    let mut control = r.clone();
    control.leave_settings();
    control.from_settings_to_stage_screen();
    control.pick(2, gen);
    control.to_fight_loop(60);
    assert_eq!(first_play(&control), file("all.ogg", TrackSource::Global));
    // Ken on NATIVE (GLOBAL, then NATIVE): the native theme plays instead,
    // with no cue for the host at all.
    r.jukebox_down(ken_row);
    r.jukebox_right(1);
    r.leave_settings();
    assert_eq!(r.s.settings().jukebox.fighter_pick("Ken"), Some(&sf_engine::jukebox::Pick::Native));
    assert_eq!(r.s.settings().jukebox.global(), Some("all.ogg"));
    r.from_settings_to_stage_screen();
    r.pick(2, gen);
    r.to_fight_loop(60);
    assert!(r.cues.is_empty(), "the native theme plays: {:?}", r.cues);
    assert!(r.latches.iter().any(|(_, l)| *l == 0x29 || *l == 0x00), "the fight still sends its sound commands: {:?}", r.latches);
}
