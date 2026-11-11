//! Ticket #199 (`ST7`): a stage's theme plays in its fights from the extended
//! tune table, arrangements load from a local music pack, a stage whose theme
//! is missing plays a 1987 theme and says so, and Faithful 1987 reads nothing
//! from the music pack (ADR-0009, docs/reference/extended-tunes.md).
//!
//! The fights are headless Local fights on a stage in the bank, driven
//! through the front end as the launcher drives them: the session's frames
//! go to the music CPU as `sf-game` pushes them, with the tune table given
//! for a fight under an overhaul preset only. The music packs are temporary
//! ones in a scratch folder; no Capcom melody is read or written here.
//!
//! Local ROMs only; each test skips without them.

use std::path::{Path, PathBuf};

use sf_audio::check::{run_music_from_power_on, OpmWrite};
use sf_audio::mix::GameSound;
use sf_audio::opm::OPM_RATE;
use sf_audio::{LatchTimeline, FRAME_HZ, IRQ_LATENCY_MID};
use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::roster::StagePick;
use sf_host::log::Log;
use sf_host::themes;
use sf_video::romset::{load_assets, load_region, Assets};
use sf_video::stage::pack_records;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const TEST_STAGE: &str = include_str!("../../../art/stages/test-stage.json");
/// The fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;
/// The test theme's command, and the pack score's below.
const TEST_THEME: u8 = 0x40;
const PACK_THEME: u8 = 0x41;
/// The 1987 theme a stage with no theme of its own plays.
const FALLBACK_THEME: u8 = 0x29;
/// The key-code table the driver turns a note into a key code with.
const KEY_TABLE: usize = 0x0a3a;

/// An original two-note score for the temporary pack: C4 E4 for ever on the
/// lead sound of the 1987 stage themes.
const PACK_SCORE: &str = "title Pack test\ncommand 0x41\nunit 10\nchannel 0\n  patch 24\n  forever { C4:4 E4:4 }\n";

/// Original control fixture, with no borrowed melody. One voice holds a
/// fractional pitch across a pitch change, then releases and re-attacks;
/// the other keeps its stock bass parameters. Durations are YM-clock units.
const RAW_VOICE_SCORE: &str = "title Native control test\ncommand 0x41\npriority 8\nunit 10\n\
channel 0\n  voice 16 4 12 5\n  forever {\n\
    raw 0x4a 0x40 8000 0\n    wait 2000\n    pitch 0x4e 0x80\n\
    wait 2000\n    off\n    wait 1000\n\
    raw 0x52 0xc0 8000 2000\n    wait 1000\n  }\n\
channel 1\n  voice 1 0 0 native\n  forever { raw 0x22 0x00 22000 8000 }\n";

struct Roms {
    assets: Assets,
    audio2: Vec<u8>,
    audiocpu: Vec<u8>,
}

fn roms() -> Option<Roms> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(Roms {
        assets: load_assets(ROMS, MANIFEST).expect("local ROM set assembles"),
        audio2: load_region(ROMS, MANIFEST, "audio2").unwrap(),
        audiocpu: load_region(ROMS, MANIFEST, "audiocpu").unwrap(),
    })
}

/// A scratch folder holding `roms/sf` (an empty stand-in for the ROM
/// folder, which is only where the pack is looked for beside) and, when
/// `pack` is given, `roms/sf1-port-music` with that score in it.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(name: &str, pack: Option<&str>) -> Scratch {
        let dir = std::env::temp_dir().join(format!("sf1-port-themes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("roms").join("sf")).unwrap();
        if let Some(score) = pack {
            let pack = dir.join("roms").join(sf_audio::music_pack::MUSIC_FOLDER);
            std::fs::create_dir_all(&pack).unwrap();
            if !score.is_empty() {
                std::fs::write(pack.join("pack-test.score"), score).unwrap();
            }
        }
        Scratch { dir }
    }

    fn rom_folder(&self) -> PathBuf {
        self.dir.join("roms").join("sf")
    }

    fn log_path(&self) -> PathBuf {
        self.dir.join("sf-game.log")
    }

    fn log_text(&self) -> String {
        std::fs::read_to_string(self.log_path()).unwrap_or_default()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The test stage's record, and a second one with the same art on the next page whose
/// theme is the pack's `PACK_THEME`.
fn records() -> [String; 2] {
    let pack_stage = TEST_STAGE.replace("\"name\": \"test-stage\"", "\"name\": \"pack-stage\"").replace("\"theme\": \"0x40\"", "\"theme\": \"0x41\"").replace("\"page\": 1", "\"page\": 2");
    assert_ne!(pack_stage, TEST_STAGE);
    [TEST_STAGE.to_owned(), pack_stage]
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

/// What a Local fight on a stage sent to the music CPU, and how it sounded:
/// the session, and the sound driven as `sf-game` drives it.
struct Fight {
    session: GameSession,
    /// Every frame's (frame, latch, cycles), as the launcher pushes them.
    pushed: Vec<(u32, u8, u32)>,
    /// The tune table was the music CPU's on these frames.
    table_on: Vec<bool>,
    sound: GameSound,
    themes: Option<themes::Themes>,
    on: bool,
}

impl Fight {
    /// One frame, as the launcher does it: the table given or taken away
    /// when the session says so, then the frame's latch pushed.
    fn step(&mut self, input: FrameInputs) -> sf_engine::front::FrameOutput {
        let out = self.session.step(input);
        if out.tune_table != self.on {
            self.on = out.tune_table;
            let table = self.themes.as_ref().filter(|_| self.on).map(|t| t.table.clone());
            self.sound.set_tune_table(table);
        }
        self.sound.push_frame(out.sound_frame, out.sound_latch, out.latch_cycles, IRQ_LATENCY_MID);
        self.pushed.push((out.sound_frame, out.sound_latch, out.latch_cycles));
        self.table_on.push(self.on);
        out
    }

    fn latches(&self) -> Vec<u8> {
        self.pushed.iter().map(|p| p.1).collect()
    }

    /// The time the first frame carrying `command` is pushed at.
    fn asked(&self, command: u8) -> Option<f64> {
        self.pushed.iter().find(|p| p.1 == command).map(|p| f64::from(p.0) / f64::from(FRAME_HZ))
    }

    /// How loud the mix is from `from` seconds on, for `seconds`.
    fn loudest(&mut self, from: f64, seconds: f64) -> f32 {
        assert!(self.sound.wait_ready(from + seconds), "the mix is ready");
        let mut loudest = 0.0f32;
        let mut t = from;
        while t < from + seconds {
            let [l, r] = self.sound.sample_at(t);
            loudest = loudest.max(l.abs()).max(r.abs());
            t += 1.0 / 8_000.0;
        }
        loudest
    }

    /// The live mix over `[from, to)` seconds.
    fn live(&mut self, from: f64, to: f64) -> Vec<[f32; 2]> {
        assert!(self.sound.wait_ready(to), "the mix is ready");
        let mut out = Vec::new();
        let mut t = from;
        while t < to {
            out.push(self.sound.sample_at(t));
            t += 1.0 / 8_000.0;
        }
        out
    }

    fn timeline(&self) -> LatchTimeline {
        let mut timeline = LatchTimeline::default();
        for &(frame, latch, cycles) in &self.pushed {
            timeline.push_frame(frame, latch, cycles, IRQ_LATENCY_MID);
        }
        timeline
    }

    /// The mix of a music CPU that had `program` from power-on, on the same
    /// latch writes, over `[from, to)` seconds.
    fn known(&self, r: &Roms, program: &[u8], from: f64, to: f64) -> Vec<[f32; 2]> {
        let mut sound = GameSound::power_on(r.audio2.clone(), program.to_vec(), self.timeline(), f64::INFINITY);
        assert!(sound.wait_ready(to), "the mix is ready");
        let mut out = Vec::new();
        let mut t = from;
        while t < to {
            out.push(sound.sample_at(t));
            t += 1.0 / 8_000.0;
        }
        out
    }

    /// The YM2151 writes of the music CPU run from power-on on the latches
    /// pushed, with the table behind the ROM or not, for `seconds`.
    fn writes(&self, program: &[u8], seconds: f64) -> Vec<OpmWrite> {
        let run = run_music_from_power_on(program.to_vec(), self.timeline(), (seconds * OPM_RATE as f64) as u64);
        assert!(run.unrecovered.is_empty(), "translated paths only: {:04x?}", run.unrecovered);
        run.writes
    }
}

/// Start a Local fight on `stage` (a stage of `bank`) under `preset`, the
/// sound driven as `sf-game` drives it, and play it to its fight loop and
/// `extra` frames past it. `themes` is what the launcher loaded for the preset.
fn fight(r: &Roms, bank: sf_video::background_bank::BackgroundBank, stage: &str, preset: RulesetPreset, themes: Option<&themes::Themes>, apply: impl FnOnce(&mut GameSession), extra: u32) -> Fight {
    let index = bank.stage_index(stage).expect("the stage is in the bank");
    let mut session = GameSession::new(load_assets(ROMS, MANIFEST).unwrap(), MatchConfig::from_preset(preset)).with_background_bank(bank);
    session.set_stage(Some(StagePick::Bank(index)));
    apply(&mut session);
    let sound = GameSound::power_on(r.audio2.clone(), r.audiocpu.clone(), LatchTimeline::default(), 0.0);
    let mut f = Fight { session, pushed: Vec::new(), table_on: Vec::new(), sound, themes: themes.cloned(), on: false };
    for _ in 0..200 {
        f.step(idle());
    }
    for tap in [
        with(|i| i.menu.start = true),
        with(|i| i.p1.directions[2] = true),
        with(|i| i.menu.confirm = true),
        with(|i| i.menu.confirm = true),
        with(|i| i.p1.buttons[0] = true),
        with(|i| i.p2.buttons[0] = true),
    ] {
        let presses = 1 + usize::from(f.session.screen() == "select" && f.session.match_config().era_select_enabled());
        for _ in 0..presses {
            f.step(tap);
            f.step(idle());
        }
    }
    while f.session.screen() != "country select" {
        f.step(idle());
    }
    f.step(with(|i| i.p1.buttons[0] = true));
    f.step(idle());
    let mut n = 0;
    while f.session.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        f.step(idle());
        n += 1;
        assert!(n < 3_000, "the fight loop (at {})", f.session.screen());
    }
    for _ in 0..extra {
        f.step(idle());
    }
    assert_eq!(flow_position(f.session.fight_machine().unwrap()), FIGHT_LOOP, "still fighting");
    f
}

/// The bank the launcher builds for the stage records `records`.
fn bank(r: &Roms, records: &[String]) -> sf_video::background_bank::BackgroundBank {
    let texts: Vec<&str> = records.iter().map(String::as_str).collect();
    let mut bank = pack_records(&r.assets, &texts).expect("the records pack");
    bank.add_rom_stages(&r.assets).expect("the 1987 stages");
    bank.add_vs_screens(&r.assets).expect("the VS screens");
    bank
}

/// Scientific pitch to the key table's index: C4 = 48.
const fn idx(octave: u8, semitone: u8) -> u8 {
    12 * octave + semitone
}

/// The notes keyed on in `channel` at or after `from` seconds, in order.
fn notes(rom: &[u8], writes: &[OpmWrite], channel: u8, from: f64) -> Vec<u8> {
    let mut code = [0u8; 8];
    let mut out = Vec::new();
    for w in writes {
        match w.reg {
            0x28..=0x2f => code[(w.reg - 0x28) as usize] = w.data,
            0x08 if w.data & 0x78 != 0 && w.data & 7 == channel && w.sample as f64 / OPM_RATE as f64 >= from => {
                out.extend((1..=96u8).find(|&i| rom[KEY_TABLE + i as usize] == code[channel as usize]));
            }
            _ => {}
        }
    }
    out
}

fn launcher_themes(r: &Roms, scratch: &Scratch, preset: RulesetPreset) -> (Option<themes::Themes>, Log) {
    let mut log = Log::create(&scratch.log_path());
    let rom_folder = scratch.rom_folder();
    let themes = themes::load(preset, &r.audiocpu, &[rom_folder.as_path()], &mut log);
    (themes, log)
}

#[test]
fn a_headless_fight_on_the_test_stage_latches_the_test_theme_and_the_driver_plays_it() {
    let Some(r) = roms() else { return };
    let scratch = Scratch::new("test-theme", None);
    let (themes, mut log) = launcher_themes(&r, &scratch, RulesetPreset::DefinitiveOverhaul);
    let themes = themes.expect("the overhaul presets have themes");
    assert_eq!(themes.commands, [TEST_THEME], "the committed test theme, with no pack");
    let mut f = fight(&r, bank(&r, &records()), "test-stage", RulesetPreset::DefinitiveOverhaul, Some(&themes), |s| themes::apply(Some(&themes), s, &mut log), 300);
    assert!(f.latches().contains(&TEST_THEME), "the fight latches the test theme's command");
    assert!(!f.latches().contains(&FALLBACK_THEME));
    assert!(!scratch.log_text().contains("the stage test-stage names"), "{}", scratch.log_text());
    // The fight had the table, and the music CPU plays the theme from it: the
    // lead's first notes are the score's (A4 C5 E5 D5 C5), and it is heard.
    assert_eq!(f.table_on.last(), Some(&true));
    let at = f.asked(TEST_THEME).unwrap();
    let writes = f.writes(&themes.program(&r.audiocpu), at + 4.0);
    let (a, c, e, d) = (idx(4, 9), idx(5, 0), idx(5, 4), idx(5, 2));
    assert_eq!(&notes(&r.audiocpu, &writes, 0, at)[..5], &[a, c, e, d, c]);
    // What the launcher's live music CPU played is, sample for sample, what
    // a music CPU that had the table from power-on plays on the same latch
    // writes, and the theme is heard in it.
    let (from, to) = (at - 0.5, at + 3.0);
    let live = f.live(from, to);
    let known = f.known(&r, &themes.program(&r.audiocpu), from, to);
    let bad = live.iter().zip(&known).position(|(a, b)| a != b);
    assert!(bad.is_none(), "the live mix differs from the power-on run with the table at sample {bad:?} of {}: {:?} vs {:?}", live.len(), bad.map(|i| live[i]), bad.map(|i| known[i]));
    let heard = live[(1.0 * 8_000.0) as usize..].iter().fold(0.0f32, |m, s| m.max(s[0].abs()).max(s[1].abs()));
    assert!(heard > 0.05, "the theme is heard ({heard})");
}

#[test]
fn a_stage_in_the_music_pack_plays_its_arrangement() {
    let Some(r) = roms() else { return };
    let scratch = Scratch::new("pack", Some(PACK_SCORE));
    let (themes, mut log) = launcher_themes(&r, &scratch, RulesetPreset::DefinitiveOverhaul);
    let themes = themes.unwrap();
    assert_eq!(themes.commands, [TEST_THEME, PACK_THEME], "the pack's score joins the committed theme");
    let text = scratch.log_text();
    assert!(text.contains("the music pack at") && text.contains("1 score(s)"), "{text}");
    let mut f = fight(&r, bank(&r, &records()), "pack-stage", RulesetPreset::DefinitiveOverhaul, Some(&themes), |s| themes::apply(Some(&themes), s, &mut log), 300);
    assert!(f.latches().contains(&PACK_THEME) && !f.latches().contains(&FALLBACK_THEME));
    let at = f.asked(PACK_THEME).unwrap();
    let writes = f.writes(&themes.program(&r.audiocpu), at + 3.0);
    assert_eq!(&notes(&r.audiocpu, &writes, 0, at)[..4], &[idx(4, 0), idx(4, 4), idx(4, 0), idx(4, 4)]);
    assert!(f.loudest(at + 1.0, 2.5) > 0.05);
}

#[test]
fn raw_pitch_and_per_voice_tone_controls_reach_the_real_fights_music_cpu() {
    let Some(r) = roms() else { return };
    let scratch = Scratch::new("raw-voice", Some(RAW_VOICE_SCORE));
    let (themes, mut log) = launcher_themes(&r, &scratch, RulesetPreset::DefinitiveOverhaul);
    let themes = themes.expect("the raw score converts through the launcher");
    assert_eq!(themes.commands, [TEST_THEME, PACK_THEME]);
    assert!(!scratch.log_text().contains("music pack: pack-test.score:"));
    let mut f = fight(&r, bank(&r, &records()), "pack-stage", RulesetPreset::DefinitiveOverhaul, Some(&themes), |s| themes::apply(Some(&themes), s, &mut log), 300);
    let at = f.asked(PACK_THEME).expect("the real fight asks for its pack theme");
    let program = themes.program(&r.audiocpu);
    let writes = f.writes(&program, at + 3.0);

    // A stock version of exactly the same original phrase provides native
    // patch levels at each actual KC. This catches accumulating attenuation
    // on re-key-on and one channel's tone controls leaking into the other.
    let stock = RAW_VOICE_SCORE.replace("voice 16 4 12 5", "voice 16 0 0 native");
    let stock_pack = Scratch::new("raw-voice-stock", Some(&stock));
    let (stock_themes, _) = launcher_themes(&r, &stock_pack, RulesetPreset::DefinitiveOverhaul);
    let stock_writes = f.writes(&stock_themes.unwrap().program(&r.audiocpu), at + 3.0);
    let snapshots = |events: &[OpmWrite], channel: u8| {
        let mut state = [0u8; 256];
        let mut notes = Vec::new();
        for w in events {
            state[w.reg as usize] = w.data;
            if w.reg == 8 && w.data & 0x78 != 0 && w.data & 7 == channel && w.sample as f64 / OPM_RATE as f64 >= at {
                notes.push(state);
            }
        }
        notes
    };
    let changed = snapshots(&writes, 0);
    let native = snapshots(&stock_writes, 0);
    assert!(changed.len() >= 4, "two passes re-apply the same patch");
    assert_eq!(changed.len(), native.len());
    for (changed, native) in changed.iter().zip(&native) {
        assert_eq!(changed[0x20] & 0xc7, native[0x20] & 0xc7, "algorithm and pan stay native");
        assert_eq!((changed[0x20] >> 3) & 7, 5, "feedback is the voice's selected value");
        for op in 0..4 {
            let reg = 0x60 + 8 * op;
            let steps = if op == 3 { 4 } else { 12 };
            assert_eq!(changed[reg], native[reg].saturating_add(steps).min(127), "operator {op}: attenuation is applied afresh");
        }
        for base in [0x28, 0x30, 0x38, 0x40, 0x80, 0xa0, 0xc0, 0xe0] {
            let operators = if base < 0x40 { 1 } else { 4 };
            for op in 0..operators {
                assert_eq!(changed[base + op * 8], native[base + op * 8], "pitch/ratio/envelope register {:#x}", base + op * 8);
            }
        }
    }
    let bass = snapshots(&writes, 1);
    let native_bass = snapshots(&stock_writes, 1);
    assert_eq!(bass.len(), native_bass.len());
    assert!(bass.len() >= 2);
    for (actual, expected) in bass.iter().zip(&native_bass) {
        for base in [0x20, 0x28, 0x30, 0x38, 0x40, 0x60, 0x80, 0xa0, 0xc0, 0xe0] {
            let operators = if base < 0x40 { 1 } else { 4 };
            for op in 0..operators {
                assert_eq!(actual[base + 1 + op * 8], expected[base + 1 + op * 8], "stock bass keeps its own parameters");
            }
        }
    }

    let pitch: Vec<u8> = writes.iter().filter(|w| w.reg == 0x28 && w.sample as f64 / OPM_RATE as f64 >= at).map(|w| w.data).collect();
    let fractions: Vec<u8> = writes.iter().filter(|w| w.reg == 0x30 && w.sample as f64 / OPM_RATE as f64 >= at).map(|w| w.data).collect();
    assert_eq!(&pitch[..3], &[0x4a, 0x4e, 0x52]);
    assert_eq!(&fractions[..3], &[0x40, 0x80, 0xc0]);
    let first_on = writes.iter().position(|w| w.reg == 8 && w.data & 0x78 != 0 && w.data & 7 == 0 && w.sample as f64 / OPM_RATE as f64 >= at).unwrap();
    let held_pitch = first_on + writes[first_on..].iter().position(|w| w.reg == 0x28 && w.data == 0x4e).unwrap();
    assert!(!writes[first_on + 1..held_pitch].iter().any(|w| w.reg == 8 && w.data & 7 == 0), "pitch changes a held note without a retrigger or premature gate");

    let (from, to) = (at - 0.1, at + 3.0);
    let live = f.live(from, to);
    assert_eq!(live, f.known(&r, &program, from, to), "fight-by-fight table handoff matches power-on playback");
    assert!(live.iter().any(|s| s[0].abs().max(s[1].abs()) > 0.01), "raw controls produce audible native audio");

    // Leaving this raw theme follows the same native stop and table-release
    // path as an ordinary score, rather than leaving a held raw note behind.
    assert_eq!(f.step(with(|i| i.menu.start = true)).screen, "pause");
    f.step(idle());
    for _ in 0..5 {
        f.step(with(|i| i.p1.directions[2] = true));
        f.step(idle());
    }
    let leaving = f.pushed.len();
    f.step(with(|i| i.menu.confirm = true));
    for _ in 0..30 { f.step(idle()); }
    assert_eq!(f.session.screen(), "main menu");
    assert!(f.pushed[leaving..].iter().any(|p| p.1 == 0x00), "stop-all reaches the music CPU");
    let end = f.pushed.last().unwrap().0 as f64 / f64::from(FRAME_HZ) - 0.01;
    assert!(f.sound.wait_ready(end));
    assert!(!f.sound.has_tune_table(), "no raw stream survives the native stop to retain its table");
}

/// A stage whose theme the pack lacks, or with no pack at all, or with a pack
/// whose score will not convert: the fight starts with a 1987 theme and the
/// log names the missing theme.
#[test]
fn a_stage_whose_theme_the_pack_lacks_plays_a_1987_theme_and_the_log_names_it() {
    let Some(r) = roms() else { return };
    let broken = "command 0x41\nchannel 0\n  patch 24\n  C4:4 end\n  nonsense\n";
    let unsafe_raw = RAW_VOICE_SCORE.replace("raw 0x4a", "raw 0xff");
    for (name, pack, expect) in [
        ("absent", None, "no music pack"),
        ("empty", Some(""), "0 score(s)"),
        ("other", Some(&PACK_SCORE.replace("0x41", "0x42")[..]), "1 score(s)"),
        ("broken", Some(broken), "music pack: pack-test.score"),
        ("unsafe-raw", Some(unsafe_raw.as_str()), "music pack: pack-test.score"),
    ] {
        let scratch = Scratch::new(name, pack);
        let (themes, mut log) = launcher_themes(&r, &scratch, RulesetPreset::DefinitiveOverhaul);
        let themes = themes.unwrap();
        assert!(!themes.commands.contains(&PACK_THEME), "{name}");
        let mut f = fight(&r, bank(&r, &records()), "pack-stage", RulesetPreset::DefinitiveOverhaul, Some(&themes), |s| themes::apply(Some(&themes), s, &mut log), 300);
        let text = scratch.log_text();
        assert!(text.contains(expect), "{name}: {text}");
        assert!(
            text.contains("the stage pack-stage names the theme 0x41, which the extended tune table (the committed themes and the music pack) does not hold: it plays the 1987 theme 0x29"),
            "{name}: {text}"
        );
        assert!(!text.contains("the stage test-stage names"), "{name}: the test theme is committed");
        assert!(f.latches().contains(&FALLBACK_THEME) && !f.latches().contains(&PACK_THEME), "{name}: a 1987 theme");
        let at = f.asked(FALLBACK_THEME).unwrap();
        assert!(f.loudest(at + 1.0, 2.5) > 0.05, "{name}: and it plays");
    }
}

#[test]
fn a_fight_still_starts_when_there_are_no_themes_at_all() {
    let Some(r) = roms() else { return };
    // A launcher that found no table at all (the sound ROM would not
    // convert): every extended theme is missing.
    let scratch = Scratch::new("no-table", None);
    let mut log = Log::create(&scratch.log_path());
    let mut f = fight(&r, bank(&r, &records()), "test-stage", RulesetPreset::DefinitiveOverhaul, None, |s| themes::apply(None, s, &mut log), 300);
    assert!(f.latches().contains(&FALLBACK_THEME) && !f.latches().contains(&TEST_THEME));
    assert!(scratch.log_text().contains("the stage test-stage names the theme 0x40, and there is no extended tune table"), "{}", scratch.log_text());
    assert!(f.table_on.iter().any(|&t| t), "the fight is an overhaul one");
    let at = f.asked(FALLBACK_THEME).unwrap();
    assert!(f.loudest(at + 1.0, 2.5) > 0.05);
}

/// Faithful 1987: no table for the music CPU, no stage, nothing from the pack.
#[test]
fn nothing_from_the_music_pack_is_read_in_faithful_1987() {
    let Some(r) = roms() else { return };
    // A pack whose only score would not convert: reading it would log it.
    let scratch = Scratch::new("faithful", Some("not a score"));
    let (themes, _log) = launcher_themes(&r, &scratch, RulesetPreset::Faithful1987);
    assert!(themes.is_none(), "no table under Faithful 1987");
    let text = scratch.log_text();
    assert!(!text.contains("music pack") && !text.contains("pack-test"), "the pack was not looked at: {text}");

    // The fight: the stage pick is ignored, the 1987 stage music plays, and
    // the music CPU never has the table.
    let f = fight(&r, bank(&r, &records()), "test-stage", RulesetPreset::Faithful1987, None, |_| {}, 300);
    assert!(f.table_on.iter().all(|&t| !t));
    assert!(!f.latches().contains(&TEST_THEME));
    let rom_themes: Vec<u8> = (0..10).map(|k| sf_video::rom_stages::rom_stage(&r.assets.maincpu, k).unwrap().theme as u8).collect();
    assert!(f.latches().iter().any(|l| rom_themes.contains(l)), "a 1987 stage theme");
}

/// The theme does not outlive its fight: leaving it for the menu stops the
/// music, the music CPU gives the table back, and nothing plays the theme
/// in the faithful cabinet that is powered on after.
#[test]
fn a_theme_does_not_outlive_its_fight_into_the_menus_or_the_faithful_cabinet() {
    let Some(r) = roms() else { return };
    let scratch = Scratch::new("leave", None);
    let (themes, mut log) = launcher_themes(&r, &scratch, RulesetPreset::DefinitiveOverhaul);
    let themes = themes.unwrap();
    let mut f = fight(&r, bank(&r, &records()), "test-stage", RulesetPreset::DefinitiveOverhaul, Some(&themes), |s| themes::apply(Some(&themes), s, &mut log), 120);
    let at = f.asked(TEST_THEME).unwrap();
    assert!(f.loudest(at + 1.0, 1.0) > 0.05);
    assert!(f.sound.has_tune_table());
    // Pause, then QUIT TO MENU (the pause box's sixth row).
    assert_eq!(f.step(with(|i| i.menu.start = true)).screen, "pause");
    f.step(idle());
    for _ in 0..5 {
        f.step(with(|i| i.p1.directions[2] = true));
        f.step(idle());
    }
    let left = f.pushed.len();
    f.step(with(|i| i.menu.confirm = true));
    for _ in 0..20 {
        f.step(idle());
    }
    assert_eq!(f.session.screen(), "main menu");
    assert!(f.pushed[left..].iter().any(|p| p.1 == 0x00), "the music is stopped on the way out");
    // The 1987 cabinet, then POWER ON on the DIP screen.
    for _ in 0..8 {
        if f.session.menu_item() == Some(sf_engine::front::MenuItem::Cabinet1987) {
            break;
        }
        f.step(with(|i| i.p1.directions[2] = true));
        f.step(idle());
    }
    assert_eq!(f.step(with(|i| i.menu.confirm = true)).screen, "1987 cabinet");
    f.step(idle());
    for _ in 0..11 {
        f.step(with(|i| i.p1.directions[2] = true));
        f.step(idle());
    }
    assert_eq!(f.step(with(|i| i.menu.confirm = true)).screen, "cabinet");
    let powered = f.pushed.len();
    for _ in 0..600 {
        f.step(idle());
    }
    assert!(!f.table_on[powered], "the session no longer asks for the table");
    assert!(!f.pushed[left..].iter().any(|p| p.1 == TEST_THEME), "the theme is not asked for again");
    let end = f.pushed.last().unwrap().0 as f64 / 60.0 - 0.1;
    assert!(f.sound.wait_ready(end));
    assert!(!f.sound.has_tune_table(), "the music CPU gave the table back");
    // And the music CPU, run on the same writes with the table, has no tune
    // current or saved a little after the stop, before the cabinet is up.
    let frame = f.pushed[left + 30].0;
    let (rams, unrecovered) = sf_audio::check::music_ram_at_frame_ends(themes.program(&r.audiocpu), f.timeline(), &[frame, frame + 1, frame + 2, frame + 3]);
    assert!(unrecovered.is_empty());
    let (_, _, ram) = rams.iter().find(|(_, idle, _)| *idle).expect("an idle frame end");
    assert_eq!((ram[0x11], ram[0x12]), (0, 0), "no tune is current or saved");
}
