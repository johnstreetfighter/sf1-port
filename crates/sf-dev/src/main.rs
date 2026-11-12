//! Development launcher: the native engine's fixtures and power-on mode in a
//! window, played with the keyboard, with both translated sound programs'
//! sound (tickets #6-#12, #37, #38, #49, #50, #55).
//!
//! Run from the crate directory (relative paths assume it):
//!
//!   cargo run -p sf-dev --release            # versus: Ryu (P1) vs Ken (P2)
//!   cargo run -p sf-dev --release -- --version
//!   cargo run -p sf-dev --release -- cpu     # one-player: Ryu (P1) vs Retsu
//!   cargo run -p sf-dev --release -- power-on # power-on: warning screen & title (#55)
//!   cargo run -p sf-dev --release -- select   # portrait character select (overhaul)
//!   cargo run -p sf-dev --release -- era-hud [p1-era] [p2-era] [art] # preview the HUD in a native fight
//!   cargo run -p sf-dev --release -- smoke   # the host smoke check
//!   cargo run -p sf-dev --release -- kit     # fixed-fight listening kit (#49)
//!   cargo run -p sf-dev --release -- music   # the round music phrase (#11)
//!   cargo run -p sf-dev --release -- record <cpu|versus> <session.tsv>
//!   cargo run -p sf-dev --release -- record-synthetic <session.tsv>
//!   cargo run -p sf-dev --release -- convert <session.tsv> <scenario.json> [scenario-id]
//!   cargo run -p sf-dev --release -- verify <session.tsv> <scenario.json> <run-a> <run-b>
//!   cargo run -p sf-dev --release -- replay <session.tsv> [takeover-frame]
//!   cargo run -p sf-dev --release -- combo-sweep <replay.json>
//!   cargo run -p sf-dev --release -- combo-replay <replay.json>
//!   cargo run -p sf-dev --release -- combo-verify <replay.json>
//!   cargo run -p sf-dev --release -- preset [name | set <key> <value>]
//!   cargo run -p sf-dev --release -- rebind <input> <Key> [bindings.toml]
//!
//! **Remappable keyboard bindings** (ticket #76, F36; persisted at
//! `%APPDATA%\sf1-port\bindings.toml` on Windows; the key names are minifb's,
//! so they follow the physical US layout):
//!
//! | | directions (up, left, down, right) | LP, MP, HP | LK, MK, HK |
//! | --- | --- | --- | --- |
//! | P1 | arrow keys | A, S, D | Z, X, C |
//! | P2 | T, F, G, H | U, I, O | J, K, L |
//! | Cabinet | `coin1=Key5`, `coin2=Key6`, `start1=Key1`, `start2=Key2`, `service1=Key9` | | |
//!
//! Esc quits (and cannot be bound). A key counts as held only while the OS reports it down AND
//! the window is focused, so an alt-tab or input reset releases everything;
//! an attack fires when its key is released, as on the reference six-button
//! cabinet, and the special-move commands work as in the scenarios
//! (docs/reference/specials.md, hadoken.md).
//!
//! **The fixtures** (local-only reference captures, docs/reference): versus
//! starts from the checkpoint-2450 state of sf-move-01 (both fighters in
//! their stances, round 1, TIME 99), the one-player mode from the
//! checkpoint-2389 state of sf-cpu-01 (FIGHT! just erased, Retsu about to
//! decide). Both run as far as the translation goes — through KOs, the
//! round results, the rounds that follow, the match end — and stop at the
//! first untranslated original branch, which is printed before the fixture
//! reloads; nothing is guessed.
//!
//! **Sound**: both translated sound programs — the samples CPU's ADPCM
//! voices and impacts on the two MSM5205s, the music CPU's tunes and FM
//! effects on the YM2151 — mixed as the reference mixes them, in stereo on
//! the default output device. The sound runs from power-on, pre-rolled to
//! the fixture frame on the reference's latch writes (a reference audio run
//! with the fixture's history), and from the fixture on follows the
//! engine's per-frame sound latch at the latency band's middle
//! (docs/reference/both-chips.md: the start and the streaming are verified;
//! at that latency the fixed fight's samples CPU differs from the
//! reference from frame 2619 on). Without a usable output device the
//! launcher runs silent and says so.
//!
//! (Debug builds render below 60 fps; the state machine itself is exact in
//! either profile.)

mod combo_sweep;
mod era_hud;
mod session;

use minifb::{Key, Window};
use sf_engine::cpu::CpuMachine;
use sf_engine::run::{PowerOnMachine, WARNING_TITLE_LAST};
use sf_engine::versus::VersusMachine;
use sf_host::audio;
use sf_host::bindings::{self, Bindings, Inputs, P1_KEYS, P2_KEYS};
use sf_host::input::LauncherPads;
use sf_host::window::{open_window, poll_key_edge, present, SCALE};
use sf_video::render::{HEIGHT, WIDTH};
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const VERSUS_FIXTURE: &str = "../../tools/reference/local/captures/move-run-a";
const CPU_FIXTURE: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const CHALLENGER_FIXTURE: &str = "../../tools/reference/local/captures/challenger-01-run-a";
/// The reference audio runs whose histories up to the fixtures are the
/// fixture captures' (sf-punch-01 = sf-move-01 through frame 2450, the
/// fixed fight = sf-cpu-01 through 2389): their latch writes before the
/// fixture frame pre-roll the sound.
const VERSUS_SOUND: &str = "../../tools/reference/local/captures/hit-sound-run-a/audio_taps.tsv";
const CPU_SOUND: &str = "../../tools/reference/local/captures/fight-audio-a/audio_taps.tsv";

fn load_active_bindings() -> bindings::BindingsTable {
    let path = bindings::default_bindings_path();
    if !path.exists() {
        let _ = bindings::DEFAULT_BINDINGS.save(&path);
    }
    let (table, diag) = bindings::BindingsTable::load_or_default(&path);
    if let Some(msg) = diag {
        eprintln!("bindings: {msg}");
    }
    table
}

fn load_active_match_config() -> sf_engine::config::MatchConfig {
    let path = sf_engine::config::default_match_config_path();
    if !path.exists() {
        let _ = sf_engine::config::MatchConfig::default().save(&path);
    }
    let (cfg, diag) = sf_engine::config::MatchConfig::load_or_default(&path);
    if let Some(msg) = diag {
        eprintln!("match_config: {msg}");
    }
    cfg
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("--version" | "-V") => println!("{}", sf_engine::version::full_banner()),
        Some("cpu") => run_cpu(),
        Some("power-on") => run_power_on(),
        Some("select") => run_select(),
        Some("era-hud") => std::process::exit(era_hud::run(&args[2..])),
        Some("music") => play_music_phrase(),
        Some("smoke") => std::process::exit(smoke()),
        Some("kit") => std::process::exit(listening_kit()),
        Some("kit-sound-test") => std::process::exit(sound_test_listening_kit()),
        Some("record") => std::process::exit(cmd_record(&args[2..])),
        Some("record-synthetic") => std::process::exit(cmd_record_synthetic(&args[2..])),
        Some("convert") => std::process::exit(cmd_convert(&args[2..])),
        Some("verify") => std::process::exit(cmd_verify(&args[2..])),
        Some("rebind") => std::process::exit(cmd_rebind(&args[2..])),
        Some("preset") => std::process::exit(cmd_preset(&args[2..])),
        Some("replay") => std::process::exit(cmd_replay(&args[2..])),
        Some(command @ ("combo-sweep" | "combo-replay" | "combo-verify")) => {
            std::process::exit(combo_sweep::run(command, &args[2..]))
        }
        _ => run_versus(),
    }
}

fn poll_overlay_toggle(window: &mut Window, prev_f1: &mut bool) -> bool {
    poll_key_edge(window, Key::F1, prev_f1)
}

/// Overhaul portrait select. Faithful power-on is unchanged.
fn run_select() {
    use sf_engine::config::RulesetPreset;
    use sf_engine::roster::{CharacterSelectScreen, MenuInput, RosterSettings};

    let mut screen = CharacterSelectScreen::open(
        RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul),
        true,
    );
    let mut window = open_window("sf-dev select");
    let mut buf = vec![0u32; WIDTH * SCALE * HEIGHT * SCALE];
    let mut prev = [false; 6];
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let keys = [Key::Left, Key::Right, Key::Up, Key::Down, Key::Enter, Key::Backspace];
        let inputs = [
            MenuInput::Left,
            MenuInput::Right,
            MenuInput::Up,
            MenuInput::Down,
            MenuInput::Confirm,
            MenuInput::Back,
        ];
        for (i, key) in keys.iter().enumerate() {
            if poll_key_edge(&mut window, *key, &mut prev[i]) {
                screen.apply(inputs[i]);
            }
        }
        let frame = screen.portrait_frame();
        present(&mut window, &mut buf, &frame);
    }
}

/// The sound's start for a fixture frame: both sound ROMs and the
/// reference's latch writes through that frame (none, with a message, when
/// the reference run is missing). The fixture frame's vblank ran in the
/// reference, so its write is the reference's; the engine's writes start
/// with the first stepped frame.
fn sound_start(taps: &str, frame: u32) -> audio::SoundStart {
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let taps_path = resolve_local(taps);
    let audio2 = sf_video::romset::load_region(&roms, &manifest, "audio2")
        .expect("audio2 region (local ROM set)");
    let audiocpu = sf_video::romset::load_region(&roms, &manifest, "audiocpu")
        .expect("audiocpu region (local ROM set)");
    let cut = (frame + 1) as f64 / sf_audio::FRAME_HZ as f64;
    let preroll = match sf_audio::check::read_taps(&taps_path) {
        Ok(log) => {
            sf_audio::LatchTimeline::new(log.latch.into_iter().filter(|e| e.0 < cut).collect())
        }
        Err(e) => {
            eprintln!("audio: reference audio run {} not read ({e}); the sound starts silent at the fixture", taps_path.display());
            sf_audio::LatchTimeline::default()
        }
    };
    audio::SoundStart {
        audio2,
        audiocpu,
        frame,
        preroll,
    }
}

/// The sound's start from power-on (frame 0, no pre-roll): both sound ROMs
/// start at `t = 0` and take the native engine's per-frame latch writes from
/// frame 1 on.
fn sound_start_power_on() -> audio::SoundStart {
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let audio2 = sf_video::romset::load_region(&roms, &manifest, "audio2")
        .expect("audio2 region (local ROM set)");
    let audiocpu = sf_video::romset::load_region(&roms, &manifest, "audiocpu")
        .expect("audiocpu region (local ROM set)");
    audio::SoundStart {
        audio2,
        audiocpu,
        frame: 0,
        preroll: sf_audio::LatchTimeline::default(),
    }
}

/// Power-on mode (tickets #55, #89; F15, F49): starts at frame 0 with no
/// fixture and no sound pre-roll, types the warning screen, joins into the
/// attract wall and title screen, accepts coins (`5` / `6`, `9` service),
/// start (`1` / `2`), and both players' directions and attack buttons when
/// focused, playing through country select and pre-fight screens into the
/// fight, and reloads to power-on on any untranslated branch.
fn run_power_on() {
    let active_bindings = load_active_bindings();
    let active_config = load_active_match_config();
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let assets = load_assets(&roms, &manifest).expect("local ROM set assembles");
    let fresh = |cfg: sf_engine::config::MatchConfig| {
        PowerOnMachine::new(assets.maincpu.clone()).with_match_config(cfg)
    };
    let mut machine = fresh(active_config);
    let start = sound_start_power_on();
    let sound = audio::Audio::open(&start);
    if sound.is_none() {
        eprintln!("audio: no usable output device; running silent");
    }
    let mut window = open_window("sf1-port dev: power-on (5/6 coin, 1/2 start, 9 service, F1 overlay, Space pause, F2 save, F4 rewind, F5 preset, Esc quits)");
    let mut buf = vec![0u32; WIDTH * SCALE * HEIGHT * SCALE];
    let mut prev_f1 = false;
    let mut prev_space = false;
    let mut prev_f2 = false;
    let mut prev_f4 = false;
    let mut prev_f5 = false;
    let mut paused = false;
    let mut bookmark: Option<sf_engine::run::PowerOnSnapshot> = None;
    let mut pads = LauncherPads::open();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        pads.poll(window.is_active());
        if poll_overlay_toggle(&mut window, &mut prev_f1) {
            machine.toggle_overlay();
        }
        if poll_key_edge(&mut window, Key::Space, &mut prev_space) {
            paused = !paused;
        }
        if poll_key_edge(&mut window, Key::F2, &mut prev_f2) {
            bookmark = Some(machine.save_snapshot());
        }
        if poll_key_edge(&mut window, Key::F4, &mut prev_f4) {
            if let Some(snap) = &bookmark {
                let keep_overlay = machine.overlay_enabled();
                machine.restore_snapshot(snap);
                machine.set_overlay_enabled(keep_overlay);
            }
        }
        if poll_key_edge(&mut window, Key::F5, &mut prev_f5) {
            machine.cycle_preset();
            let _ = machine
                .match_config()
                .save(&sf_engine::config::default_match_config_path());
            println!("ruleset: {}", machine.match_config().summary());
        }
        if !paused {
            let (p1, p2) = (
                pads.player(0, active_bindings.p1.read(&mut window), &active_bindings.p1_pad),
                pads.player(1, active_bindings.p2.read(&mut window), &active_bindings.p2_pad),
            );
            apply_power_on(&mut machine, p1, p2);
            let [coin1, coin2, start1, start2, service1] =
                pads.cabinet(active_bindings.read_cabinet(&mut window));
            machine.set_cabinet_inputs(coin1, coin2, start1, start2, service1);
            machine.step();
            if let Some(s) = &sound {
                s.push_frame(
                    machine.state.frame,
                    machine.state.sound_latch,
                    machine.state.latch_cycles,
                );
            }
            let unrecovered = machine.unrecovered();
            if !unrecovered.is_empty() {
                let keep_overlay = machine.overlay_enabled();
                let keep_config = *machine.match_config();
                eprintln!(
                    "frame {}: reached untranslated original branches {unrecovered:05x?}; restarting from power-on",
                    machine.state.frame
                );
                machine = fresh(keep_config);
                machine.set_overlay_enabled(keep_overlay);
                if let Some(s) = &sound {
                    s.reset(&start);
                }
            }
        }
        let frame = machine.render_with_overlay(&assets);
        present(&mut window, &mut buf, &frame);
    }
}

fn apply_power_on(machine: &mut PowerOnMachine, p1: Inputs, p2: Inputs) {
    let [r, l, d, u] = p1.directions;
    machine.set_p1_direction(r, l, d, u);
    machine.set_p1_buttons(p1.buttons);
    let [r, l, d, u] = p2.directions;
    machine.set_p2_direction(r, l, d, u);
    machine.set_p2_buttons(p2.buttons);
}

/// The versus fixture, both players on the keyboard.
fn run_versus() {
    let active_bindings = load_active_bindings();
    let active_config = load_active_match_config();
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let versus_fixture = resolve_local(VERSUS_FIXTURE);
    let assets = load_assets(&roms, &manifest).expect("local ROM set assembles");
    let fixture = |cfg: sf_engine::config::MatchConfig| {
        VersusMachine::from_checkpoint_2450_with_program(&versus_fixture, assets.maincpu.clone())
            .expect("checkpoint-2450 fixture dumps (local reference capture)")
            .with_match_config(cfg)
    };
    let mut machine = fixture(active_config);
    let start = sound_start(VERSUS_SOUND, machine.state.frame);
    let sound = audio::Audio::open(&start);
    if sound.is_none() {
        eprintln!("audio: no usable output device; running silent");
    }
    let mut window = open_window("sf1-port dev: versus (P1 arrows + A S D / Z X C, P2 T F G H + U I O / J K L, F1 overlay, Space pause, F2 save, F4 rewind, F5 preset, Esc quits)");
    let mut buf = vec![0u32; WIDTH * SCALE * HEIGHT * SCALE];
    let mut prev_f1 = false;
    let mut prev_space = false;
    let mut prev_f2 = false;
    let mut prev_f4 = false;
    let mut prev_f5 = false;
    let mut paused = false;
    let mut bookmark: Option<sf_engine::fight::FightSnapshot> = None;
    let mut pads = LauncherPads::open();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        pads.poll(window.is_active());
        if poll_overlay_toggle(&mut window, &mut prev_f1) {
            machine.toggle_overlay();
        }
        if poll_key_edge(&mut window, Key::Space, &mut prev_space) {
            paused = !paused;
        }
        if poll_key_edge(&mut window, Key::F2, &mut prev_f2) {
            bookmark = Some(machine.save_snapshot());
        }
        if poll_key_edge(&mut window, Key::F4, &mut prev_f4) {
            if let Some(snap) = &bookmark {
                let keep_overlay = machine.overlay_enabled();
                machine.restore_snapshot(snap);
                machine.set_overlay_enabled(keep_overlay);
            }
        }
        if poll_key_edge(&mut window, Key::F5, &mut prev_f5) {
            machine.cycle_preset();
            let _ = machine
                .match_config()
                .save(&sf_engine::config::default_match_config_path());
            println!("ruleset: {}", machine.match_config().summary());
        }
        if !paused {
            let (p1, p2) = (
                pads.player(0, active_bindings.p1.read(&mut window), &active_bindings.p1_pad),
                pads.player(1, active_bindings.p2.read(&mut window), &active_bindings.p2_pad),
            );
            apply_versus(&mut machine, p1, p2);
            let [coin1, coin2, start1, start2, service1] =
                pads.cabinet(active_bindings.read_cabinet(&mut window));
            machine.set_cabinet_inputs(coin1, coin2, start1, start2, service1);
            machine.step();
            if let Some(s) = &sound {
                s.push_frame(
                    machine.state.frame,
                    machine.state.sound_latch,
                    machine.state.latch_cycles,
                );
            }
            // Stop on any untranslated original branch rather than show
            // guessed frames: report it and reload the fixture.
            let unrecovered = machine.unrecovered();
            if !unrecovered.is_empty() {
                let keep_overlay = machine.overlay_enabled();
                let keep_config = *machine.match_config();
                eprintln!("frame {}: reached untranslated original branches {unrecovered:05x?}; reloading the fixture", machine.state.frame);
                machine = fixture(keep_config);
                machine.set_overlay_enabled(keep_overlay);
                if let Some(s) = &sound {
                    s.reset(&start);
                }
            }
        }
        let frame = machine.render_with_overlay(&assets);
        present(&mut window, &mut buf, &frame);
    }
}

fn apply_versus(machine: &mut VersusMachine, p1: Inputs, p2: Inputs) {
    let [r, l, d, u] = p1.directions;
    machine.set_p1_direction(r, l, d, u);
    machine.set_p1_buttons(p1.buttons);
    let [r, l, d, u] = p2.directions;
    machine.set_p2_direction(r, l, d, u);
    machine.set_p2_buttons(p2.buttons);
}

fn apply_cpu(machine: &mut CpuMachine, p1: Inputs) {
    let [r, l, d, u] = p1.directions;
    machine.set_p1_direction(r, l, d, u);
    machine.set_p1_buttons(p1.buttons);
}

/// The one-player fixture: P1 on the keyboard against the CPU's Retsu.
fn run_cpu() {
    let active_bindings = load_active_bindings();
    let active_config = load_active_match_config();
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let cpu_fixture = resolve_local(CPU_FIXTURE);
    let assets = load_assets(&roms, &manifest).expect("local ROM set assembles");
    let fixture = |cfg: sf_engine::config::MatchConfig| {
        CpuMachine::from_checkpoint_2389(&cpu_fixture, assets.maincpu.clone())
            .expect("checkpoint-2389 fixture dumps (local reference capture)")
            .with_match_config(cfg)
    };
    let mut machine = fixture(active_config);
    let start = sound_start(CPU_SOUND, machine.state.frame);
    let sound = audio::Audio::open(&start);
    if sound.is_none() {
        eprintln!("audio: no usable output device; running silent");
    }
    let mut window = open_window("sf1-port dev: Ryu vs Retsu (arrows + A S D / Z X C, F1 overlay, Space pause, F2 save, F4 rewind, F5 preset, Esc quits)");
    let mut buf = vec![0u32; WIDTH * SCALE * HEIGHT * SCALE];
    let mut prev_f1 = false;
    let mut prev_space = false;
    let mut prev_f2 = false;
    let mut prev_f4 = false;
    let mut prev_f5 = false;
    let mut paused = false;
    let mut bookmark: Option<sf_engine::fight::FightSnapshot> = None;
    let mut pads = LauncherPads::open();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        pads.poll(window.is_active());
        if poll_overlay_toggle(&mut window, &mut prev_f1) {
            machine.toggle_overlay();
        }
        if poll_key_edge(&mut window, Key::Space, &mut prev_space) {
            paused = !paused;
        }
        if poll_key_edge(&mut window, Key::F2, &mut prev_f2) {
            bookmark = Some(machine.save_snapshot());
        }
        if poll_key_edge(&mut window, Key::F4, &mut prev_f4) {
            if let Some(snap) = &bookmark {
                let keep_overlay = machine.overlay_enabled();
                machine.restore_snapshot(snap);
                machine.set_overlay_enabled(keep_overlay);
            }
        }
        if poll_key_edge(&mut window, Key::F5, &mut prev_f5) {
            machine.cycle_preset();
            let _ = machine
                .match_config()
                .save(&sf_engine::config::default_match_config_path());
            println!("ruleset: {}", machine.match_config().summary());
        }
        if !paused {
            let (p1, p2) = (
                pads.player(0, active_bindings.p1.read(&mut window), &active_bindings.p1_pad),
                pads.player(1, active_bindings.p2.read(&mut window), &active_bindings.p2_pad),
            );
            session::apply_cpu_both(&mut machine, p1, p2);
            let [coin1, coin2, start1, start2, service1] =
                pads.cabinet(active_bindings.read_cabinet(&mut window));
            machine.set_cabinet_inputs(coin1, coin2, start1, start2, service1);
            machine.step();
            if let Some(s) = &sound {
                s.push_frame(
                    machine.state.frame,
                    machine.state.sound_latch,
                    machine.state.latch_cycles,
                );
            }
            let unrecovered = machine.unrecovered();
            if !unrecovered.is_empty() {
                let keep_overlay = machine.overlay_enabled();
                let keep_config = *machine.match_config();
                eprintln!("frame {}: reached untranslated original branches {unrecovered:05x?}; reloading the fixture", machine.state.frame);
                machine = fixture(keep_config);
                machine.set_overlay_enabled(keep_overlay);
                if let Some(s) = &sound {
                    s.reset(&start);
                }
            }
        }
        let frame = machine.render_with_overlay(&assets);
        present(&mut window, &mut buf, &frame);
    }
}

/// Interactive scenario replay with deterministic Replay Takeover (`#93`, `O2`):
/// `cargo run -p sf-dev --release -- replay <cpu|versus> <scenario.json>`
/// Hotkeys:
/// - `F1`: toggle collision / command-tracker / 60-cell frame meter overlay
/// Interactive Replay Takeover launcher (`cargo run -p sf-dev --release -- replay <cpu|versus> <scenario.json> [window_frames]`, `#93` `O2`).
///
/// Hotkeys:
/// - `F1`: toggle diagnostic + 60-cell frame meter overlay
/// - `Space`: pause / resume replay playback
/// - `F2`: bookmark frame and take over P1 (default 10s = 600 frames)
/// - `F3`: bookmark frame and take over P2 (default 10s = 600 frames)
/// - `F4`: rewind to takeover bookmark
fn cmd_replay(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("usage: sf-dev replay <cpu|versus> <scenario.json> [window_frames]");
        return 2;
    }
    let fixture_kind = match session::FixtureKind::parse(&args[0]) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let scenario_path = std::path::Path::new(&args[1]);
    let window_frames = if let Some(raw) = args.get(2) {
        match raw.parse::<u32>() {
            Ok(n) if n > 0 => Some(n),
            _ => {
                eprintln!("window_frames must be a positive integer (got '{raw}')");
                return 2;
            }
        }
    } else {
        None
    };
    let active_bindings = load_active_bindings();
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let assets = load_assets(&roms, &manifest).expect("local ROM set assembles");
    let machine = match fixture_kind {
        session::FixtureKind::Cpu => {
            let cpu_fixture = resolve_local(CPU_FIXTURE);
            CpuMachine::from_checkpoint_2389(&cpu_fixture, assets.maincpu.clone())
                .expect("checkpoint-2389 fixture dumps")
        }
        session::FixtureKind::Versus => {
            let versus_fixture = resolve_local(VERSUS_FIXTURE);
            VersusMachine::from_checkpoint_2450_with_program(
                &versus_fixture,
                assets.maincpu.clone(),
            )
            .expect("checkpoint-2450 fixture dumps")
        }
    };
    let mut session =
        match sf_engine::replay::ReplayTakeoverSession::from_scenario(machine, scenario_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("replay scenario error: {e}");
                return 1;
            }
        };
    if let Some(frames) = window_frames {
        session.set_takeover_window_frames(frames);
    }
    let mut window = open_window(
        "sf1-port replay takeover (F1 overlay, Space pause/play, F2 takeover P1, F3 takeover P2, F4 rewind, Esc quits)",
    );
    let mut buf = vec![0u32; WIDTH * SCALE * HEIGHT * SCALE];
    let mut prev_f1 = false;
    let mut prev_space = false;
    let mut prev_f2 = false;
    let mut prev_f3 = false;
    let mut prev_f4 = false;
    let mut pads = LauncherPads::open();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        pads.poll(window.is_active());
        if poll_overlay_toggle(&mut window, &mut prev_f1) {
            session.machine_mut().toggle_overlay();
        }
        if poll_key_edge(&mut window, Key::Space, &mut prev_space) {
            session.toggle_pause();
        }
        if poll_key_edge(&mut window, Key::F2, &mut prev_f2) {
            session.take_over(sf_engine::replay::TakeoverPlayer::P1);
        }
        if poll_key_edge(&mut window, Key::F3, &mut prev_f3) {
            session.take_over(sf_engine::replay::TakeoverPlayer::P2);
        }
        if poll_key_edge(&mut window, Key::F4, &mut prev_f4) {
            session.rewind_to_takeover();
        }
        match session.mode() {
            sf_engine::replay::ReplayMode::Paused => {}
            sf_engine::replay::ReplayMode::Playback => {
                session.step_replay();
            }
            sf_engine::replay::ReplayMode::Takeover { player, .. } => {
                let raw = match player {
                    sf_engine::replay::TakeoverPlayer::P1 => {
                        pads.player(0, active_bindings.p1.read(&mut window), &active_bindings.p1_pad)
                    }
                    sf_engine::replay::TakeoverPlayer::P2 => {
                        pads.player(1, active_bindings.p2.read(&mut window), &active_bindings.p2_pad)
                    }
                };
                session.step_with_live_input(sf_engine::replay::PlayerInputs {
                    directions: raw.directions,
                    buttons: raw.buttons,
                });
            }
        }
        let frame = session.machine().render_with_overlay(&assets);
        present(&mut window, &mut buf, &frame);
    }
    0
}

/// Interactive keyboard session recorder (`cargo run -p sf-dev --release -- record <cpu|versus> <session.tsv>`).
fn cmd_record(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("usage: sf-dev record <cpu|versus> <session.tsv>");
        return 2;
    }
    let fixture_kind = match session::FixtureKind::parse(&args[0]) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let out_path = std::path::Path::new(&args[1]);
    let active_bindings = load_active_bindings();
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let assets = load_assets(&roms, &manifest).expect("local ROM set assembles");
    let mut log = session::SessionLog::new(fixture_kind);
    let mut prev_focused = true;
    let mut buf = vec![0u32; WIDTH * SCALE * HEIGHT * SCALE];

    match fixture_kind {
        session::FixtureKind::Cpu => {
            let cpu_fixture = resolve_local(CPU_FIXTURE);
            let mut machine =
                CpuMachine::from_checkpoint_2389(&cpu_fixture, assets.maincpu.clone())
                    .expect("checkpoint-2389 fixture dumps (local reference capture)");
            let start = sound_start(CPU_SOUND, machine.state.frame);
            let sound = audio::Audio::open(&start);
            let mut window =
                open_window("sf1-port dev [RECORDING cpu]: Ryu vs Retsu (Esc finishes)");
            let mut pads = LauncherPads::open();
            while window.is_open() && !window.is_key_down(Key::Escape) {
                let tick = std::time::Instant::now();
                let f = machine.state.frame;
                let focused = window.is_active();
                pads.poll(focused);
                let focus_changed = focused != prev_focused;
                prev_focused = focused;
                let p1 = session::read_with_focus(&active_bindings.p1, focused, |k| {
                    window.is_key_down(k)
                });
                let p1 = pads.player(0, p1, &active_bindings.p1_pad);
                let p2 = session::read_with_focus(&active_bindings.p2, focused, |k| {
                    window.is_key_down(k)
                });
                let p2 = pads.player(1, p2, &active_bindings.p2_pad);
                session::apply_cpu_both(&mut machine, p1, p2);
                machine.step();
                if let Some(s) = &sound {
                    s.push_frame(
                        machine.state.frame,
                        machine.state.sound_latch,
                        machine.state.latch_cycles,
                    );
                }
                let shown = machine.render_with_overlay(&assets);
                present(&mut window, &mut buf, &shown);
                let frame_ms = tick.elapsed().as_secs_f64() * 1e3;
                log.rows.push(session::SessionFrame {
                    frame: f,
                    focused,
                    focus_changed,
                    frame_ms,
                    p1,
                    p2,
                });
                let unrecovered = machine.unrecovered();
                if !unrecovered.is_empty() {
                    eprintln!("frame {}: reached untranslated original branches {unrecovered:05x?}; ending recording", machine.state.frame);
                    break;
                }
            }
        }
        session::FixtureKind::Versus => {
            let versus_fixture = resolve_local(VERSUS_FIXTURE);
            let mut machine = VersusMachine::from_checkpoint_2450_with_program(
                &versus_fixture,
                assets.maincpu.clone(),
            )
            .expect("checkpoint-2450 fixture dumps (local reference capture)");
            let start = sound_start(VERSUS_SOUND, machine.state.frame);
            let sound = audio::Audio::open(&start);
            let mut window =
                open_window("sf1-port dev [RECORDING versus]: Ryu vs Ken (Esc finishes)");
            let mut pads = LauncherPads::open();
            while window.is_open() && !window.is_key_down(Key::Escape) {
                let tick = std::time::Instant::now();
                let f = machine.state.frame;
                let focused = window.is_active();
                pads.poll(focused);
                let focus_changed = focused != prev_focused;
                prev_focused = focused;
                let p1 = session::read_with_focus(&active_bindings.p1, focused, |k| {
                    window.is_key_down(k)
                });
                let p1 = pads.player(0, p1, &active_bindings.p1_pad);
                let p2 = session::read_with_focus(&active_bindings.p2, focused, |k| {
                    window.is_key_down(k)
                });
                let p2 = pads.player(1, p2, &active_bindings.p2_pad);
                apply_versus(&mut machine, p1, p2);
                machine.step();
                if let Some(s) = &sound {
                    s.push_frame(
                        machine.state.frame,
                        machine.state.sound_latch,
                        machine.state.latch_cycles,
                    );
                }
                let shown = machine.render_with_overlay(&assets);
                present(&mut window, &mut buf, &shown);
                let frame_ms = tick.elapsed().as_secs_f64() * 1e3;
                log.rows.push(session::SessionFrame {
                    frame: f,
                    focused,
                    focus_changed,
                    frame_ms,
                    p1,
                    p2,
                });
                let unrecovered = machine.unrecovered();
                if !unrecovered.is_empty() {
                    eprintln!("frame {}: reached untranslated original branches {unrecovered:05x?}; ending recording", machine.state.frame);
                    break;
                }
            }
        }
    }

    if let Err(e) = log.write(out_path) {
        eprintln!("failed to write session {}: {e}", out_path.display());
        return 1;
    }
    let stats = log.stats();
    println!(
        "recorded {} session to {}: {} frames ({}..={}), {} focus changes, mean frame time {:.3} ms, max frame time {:.3} ms",
        stats.fixture.as_str(),
        out_path.display(),
        stats.frames,
        stats.fixture_frame,
        stats.last_frame,
        stats.focus_changes,
        stats.mean_frame_ms,
        stats.max_frame_ms
    );
    0
}

/// Rebind one logical input and save to `bindings.toml`
/// (`cargo run -p sf-dev --release -- rebind <input> <Key> [bindings.toml]`).
fn cmd_rebind(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("usage: sf-dev rebind <input> <Key> [bindings.toml]");
        return 2;
    }
    let input = match bindings::LogicalInput::from_name(&args[0]) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("rebind failed: {e}");
            return 2;
        }
    };
    let key = match bindings::name_to_key(&args[1]) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("rebind failed: {e}");
            return 2;
        }
    };
    let path = args
        .get(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(bindings::default_bindings_path);
    let (mut table, diag) = bindings::BindingsTable::load_or_default(&path);
    if let Some(msg) = diag {
        eprintln!("bindings: {msg}");
    }
    if let Err(e) = table.rebind_and_save(&path, input, key) {
        eprintln!("rebind failed: {e}");
        return 1;
    }
    println!(
        "rebound {} = {} in {} ({})",
        input.name(),
        bindings::key_to_name(key).unwrap_or("?"),
        path.display(),
        table.summary()
    );
    0
}

/// Inspect, switch, or override the ruleset preset and Tier 1 core toggles (`#94`, `O3`):
/// - `sf-dev preset` -> print active `match_config.toml` summary
/// - `sf-dev preset <preset_name> [match_config.toml]` -> select and save one of the 5 presets
/// - `sf-dev preset set <toggle_key> <value> [match_config.toml]` -> override a toggle and save
fn cmd_preset(args: &[String]) -> i32 {
    if args.is_empty() {
        let cfg = load_active_match_config();
        println!("{}", cfg.summary());
        return 0;
    }
    if args[0] == "set" {
        if args.len() < 3 {
            eprintln!("usage: sf-dev preset set <toggle_key> <value> [match_config.toml]");
            return 2;
        }
        let path = args
            .get(3)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(sf_engine::config::default_match_config_path);
        let (mut cfg, diag) = sf_engine::config::MatchConfig::load_or_default(&path);
        if let Some(msg) = diag {
            eprintln!("match_config: {msg}");
        }
        if let Err(e) = cfg.set_field_by_name(&args[1], &args[2]) {
            eprintln!("preset set failed: {e}");
            return 2;
        }
        if let Err(e) = cfg.save(&path) {
            eprintln!("preset save failed: {e}");
            return 1;
        }
        println!("saved {} ({})", path.display(), cfg.summary());
        return 0;
    }
    let preset = match sf_engine::config::RulesetPreset::from_str_name(&args[0]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("preset failed: {e}");
            return 2;
        }
    };
    let path = args
        .get(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(sf_engine::config::default_match_config_path);
    let mut cfg = sf_engine::config::MatchConfig::load_or_default(&path).0;
    cfg.apply_preset(preset);
    if let Err(e) = cfg.save(&path) {
        eprintln!("preset save failed: {e}");
        return 1;
    }
    println!("saved {} ({})", path.display(), cfg.summary());
    0
}

/// Scripted synthetic 1,011-frame session recorder (`cargo run -p sf-dev --release -- record-synthetic <session.tsv>`).
fn cmd_record_synthetic(args: &[String]) -> i32 {
    let Some(out) = args.first() else {
        eprintln!("usage: sf-dev record-synthetic <session.tsv>");
        return 2;
    };
    let out_path = std::path::Path::new(out);
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let cpu_fixture = resolve_local(CPU_FIXTURE);
    let assets = match load_assets(&roms, &manifest) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ROM set: {e}");
            return 1;
        }
    };
    match session::record_synthetic_session(&cpu_fixture, &assets, out_path) {
        Ok(log) => {
            let stats = log.stats();
            println!(
                "recorded synthetic {} session to {}: {} frames ({}..={}), {} focus changes, mean frame time {:.3} ms, max frame time {:.3} ms",
                stats.fixture.as_str(),
                out_path.display(),
                stats.frames,
                stats.fixture_frame,
                stats.last_frame,
                stats.focus_changes,
                stats.mean_frame_ms,
                stats.max_frame_ms
            );
            0
        }
        Err(e) => {
            eprintln!("record-synthetic failed: {e}");
            1
        }
    }
}

/// Session-to-scenario JSON converter (`cargo run -p sf-dev --release -- convert <session.tsv> <scenario.json> [scenario-id]`).
fn cmd_convert(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("usage: sf-dev convert <session.tsv> <scenario.json> [scenario-id]");
        return 2;
    }
    let session_path = std::path::Path::new(&args[0]);
    let scenario_path = std::path::Path::new(&args[1]);
    let id = args.get(2).map(|s| s.as_str());
    match session::convert_session_file(session_path, scenario_path, id) {
        Ok(stats) => {
            println!(
                "converted {} session ({} frames, window {}..={}) to {}",
                stats.fixture.as_str(),
                stats.frames,
                stats.fixture_frame,
                stats.last_frame,
                scenario_path.display()
            );
            0
        }
        Err(e) => {
            eprintln!("convert failed: {e}");
            1
        }
    }
}

/// Verify a recorded session and scenario against two MAME reference captures
/// (`cargo run -p sf-dev --release -- verify <session.tsv> <scenario.json> <run-a> <run-b>`).
fn cmd_verify(args: &[String]) -> i32 {
    if args.len() < 4 {
        eprintln!("usage: sf-dev verify <session.tsv> <scenario.json> <run-a> <run-b>");
        return 2;
    }
    let session_path = std::path::Path::new(&args[0]);
    let scenario_path = std::path::Path::new(&args[1]);
    let run_a = std::path::Path::new(&args[2]);
    let run_b = std::path::Path::new(&args[3]);
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let assets = match load_assets(&roms, &manifest) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ROM set: {e}");
            return 1;
        }
    };
    let log = match session::SessionLog::load(session_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("session file: {e}");
            return 1;
        }
    };
    let fixture_capture = match log.fixture {
        session::FixtureKind::Cpu => resolve_local(CPU_FIXTURE),
        session::FixtureKind::Versus => resolve_local(VERSUS_FIXTURE),
    };
    match session::verify_session(
        session_path,
        scenario_path,
        run_a,
        run_b,
        &fixture_capture,
        &assets,
    ) {
        Ok(session::VerifyOutcome::Match {
            stats,
            reference_files_compared,
            frames_compared,
            checkpoints_compared,
        }) => {
            println!(
                "session: {} fixture ({}..={}), {} frames, {} focus changes, mean frame time {:.3} ms, max frame time {:.3} ms",
                stats.fixture.as_str(),
                stats.fixture_frame,
                stats.last_frame,
                stats.frames,
                stats.focus_changes,
                stats.mean_frame_ms,
                stats.max_frame_ms
            );
            println!(
                "verify PASS: reference run-a == run-b over {reference_files_compared} observation files; native replay matches {frames_compared} frames and {checkpoints_compared} full checkpoints + work RAM through {}",
                stats.last_frame
            );
            0
        }
        Ok(session::VerifyOutcome::Stopped {
            stats,
            frames_matched,
            frame,
            unrecovered,
        }) => {
            println!(
                "session: {} fixture ({}..={}), {} frames, {} focus changes, mean frame time {:.3} ms, max frame time {:.3} ms",
                stats.fixture.as_str(),
                stats.fixture_frame,
                stats.last_frame,
                stats.frames,
                stats.focus_changes,
                stats.mean_frame_ms,
                stats.max_frame_ms
            );
            println!(
                "verify STOPPED: native replay matched {frames_matched} frames, then reached untranslated branches {unrecovered:05x?} at frame {frame}"
            );
            1
        }
        Ok(session::VerifyOutcome::Mismatch {
            stats,
            frame,
            reason,
        }) => {
            println!(
                "session: {} fixture ({}..={}), {} frames, {} focus changes, mean frame time {:.3} ms, max frame time {:.3} ms",
                stats.fixture.as_str(),
                stats.fixture_frame,
                stats.last_frame,
                stats.frames,
                stats.focus_changes,
                stats.mean_frame_ms,
                stats.max_frame_ms
            );
            println!("verify FAIL at frame {frame}: {reason}");
            1
        }
        Err(e) => {
            eprintln!("verify error: {e}");
            1
        }
    }
}

/// The host smoke check (ticket #38): the window opens and shows frames,
/// the audio output opens and takes the engine's latches, and the key
/// bindings drive both fixtures through a scripted sequence (a walk, a jab
/// and a Hadoken command for each player, as the scenarios do) on
/// translated paths only. Prints a report; the exit code is 0 on success.
/// The keys themselves cannot be pressed from here — the bindings are
/// exercised by feeding their [`Inputs`] exactly as the key reader would.
fn smoke() -> i32 {
    let mut failures = 0;
    let mut report = |ok: bool, what: &str| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            failures += 1;
        }
    };
    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let versus_fixture = resolve_local(VERSUS_FIXTURE);
    let cpu_fixture = resolve_local(CPU_FIXTURE);
    let assets = match load_assets(&roms, &manifest) {
        Ok(a) => a,
        Err(e) => {
            report(false, &format!("ROM set: {e}"));
            return 1;
        }
    };
    let mut window = open_window("sf1-port dev: smoke check");
    report(window.is_open(), "window opened");
    let versus_start = match VersusMachine::from_checkpoint_2450_with_program(
        &versus_fixture,
        assets.maincpu.clone(),
    ) {
        Ok(m) => sound_start(VERSUS_SOUND, m.state.frame),
        Err(e) => {
            report(false, &format!("versus fixture: {e}"));
            return 1;
        }
    };
    let pre = versus_start.preroll.clone().into_events().len();
    let clock = std::time::Instant::now();
    let sound = audio::Audio::open(&versus_start);
    let took = clock.elapsed().as_secs_f64();
    report(sound.is_some(), "audio output opened (default device)");
    report(pre > 0, &format!("sound pre-rolled to frame {} on the reference's {pre} latch writes through it ({took:.2} s)", versus_start.frame));

    // The active bindings (ticket #76, F36): all 25 logical inputs distinct and
    // non-Escape, reported in the smoke output.
    let bindings_path = bindings::default_bindings_path();
    if !bindings_path.exists() {
        let _ = bindings::DEFAULT_BINDINGS.save(&bindings_path);
    }
    let (active_bindings, bindings_ok, bindings_line, bindings_diag) =
        smoke_bindings_check(&bindings_path);
    if let Some(diag) = &bindings_diag {
        println!("note: {diag}");
    }
    report(bindings_ok, &bindings_line);

    let (controls_ok, controls_line) = sf_engine::session_io::host_smoke_line();
    report(controls_ok, &controls_line);

    // A scripted input sequence per player (frame offsets from the start):
    // walk forward 30 frames, a light punch (4 frames held), then a
    // Hadoken command (down 3, forward 20, punch 4 from frame 6).
    let script = |f: u32, fwd: usize| -> Inputs {
        let mut i = Inputs::default();
        if (1..=30).contains(&f) {
            i.directions[fwd] = true;
        }
        if (40..=43).contains(&f) {
            i.buttons[0] = true;
        }
        if (80..=82).contains(&f) {
            i.directions[2] = true;
        }
        if (82..=101).contains(&f) {
            i.directions[fwd] = true;
        }
        if (86..=89).contains(&f) {
            i.buttons[0] = true;
        }
        i
    };
    let p1_through_bindings = |i: Inputs| -> Inputs {
        active_bindings.p1.inputs(|k| {
            active_bindings
                .p1
                .directions
                .iter()
                .zip(i.directions)
                .any(|(&bk, h)| bk == k && h)
                || active_bindings
                    .p1
                    .buttons
                    .iter()
                    .zip(i.buttons)
                    .any(|(&bk, h)| bk == k && h)
        })
    };
    let p2_through_bindings = |i: Inputs| -> Inputs {
        active_bindings.p2.inputs(|k| {
            active_bindings
                .p2
                .directions
                .iter()
                .zip(i.directions)
                .any(|(&bk, h)| bk == k && h)
                || active_bindings
                    .p2
                    .buttons
                    .iter()
                    .zip(i.buttons)
                    .any(|(&bk, h)| bk == k && h)
        })
    };
    let frames = 240;
    let mut buf = vec![0u32; WIDTH * SCALE * HEIGHT * SCALE];

    match VersusMachine::from_checkpoint_2450_with_program(&versus_fixture, assets.maincpu.clone())
    {
        Ok(mut m) => {
            let start = m.state.frame;
            let mut latches = 0;
            for f in 1..=frames {
                apply_versus(
                    &mut m,
                    p1_through_bindings(script(f, 0)),
                    p2_through_bindings(script(f, 1)),
                );
                m.step();
                if let Some(s) = &sound {
                    s.push_frame(m.state.frame, m.state.sound_latch, m.state.latch_cycles);
                }
                if m.state.sound_latch != 0xff {
                    latches += 1;
                }
                if f % 4 == 0 && window.is_open() {
                    let shown = m.render_with_overlay(&assets);
                    present(&mut window, &mut buf, &shown);
                }
            }
            report(m.unrecovered().is_empty() && m.state.frame == start + frames, &format!("versus fixture: {frames} frames of scripted P1 and P2 play on translated paths"));
            report(
                latches > 0,
                &format!("versus fixture: sound commands reached the latch ({latches} frames)"),
            );
            // Let the audio callback play what was pushed.
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Some(s) = &sound {
                let u = s.unrecovered();
                let note = if u.is_empty() {
                    String::new()
                } else {
                    format!(" (reached {u:04x?})")
                };
                report(
                    u.is_empty(),
                    &format!("versus fixture: the music CPU stayed on translated paths{note}"),
                );
                let (played, peak) = s.played();
                report(played > 0.3 && peak > 0.01, &format!("versus fixture: the output played {:.0} ms of the run, peak level {peak:.3}", played * 1e3));
            }
        }
        Err(e) => report(false, &format!("versus fixture: {e}")),
    }
    match CpuMachine::from_checkpoint_2389(&cpu_fixture, assets.maincpu.clone()) {
        Ok(mut m) => {
            let start = m.state.frame;
            if let Some(s) = &sound {
                s.reset(&sound_start(CPU_SOUND, m.state.frame));
            }
            for f in 1..=frames {
                apply_cpu(&mut m, p1_through_bindings(script(f, 0)));
                m.step();
                if let Some(s) = &sound {
                    s.push_frame(m.state.frame, m.state.sound_latch, m.state.latch_cycles);
                }
                if f % 4 == 0 && window.is_open() {
                    let shown = m.render_with_overlay(&assets);
                    present(&mut window, &mut buf, &shown);
                }
            }
            report(m.unrecovered().is_empty() && m.state.frame == start + frames, &format!("one-player fixture: {frames} frames of scripted P1 play against Retsu on translated paths"));
        }
        Err(e) => report(false, &format!("one-player fixture: {e}")),
    }
    // Let the audio callback drain what was pushed.
    std::thread::sleep(std::time::Duration::from_millis(500));
    if let Some(s) = &sound {
        let u = s.unrecovered();
        let note = if u.is_empty() {
            String::new()
        } else {
            format!(" (reached {u:04x?})")
        };
        report(
            u.is_empty(),
            &format!("one-player fixture: the music CPU stayed on translated paths{note}"),
        );
        let (played, peak) = s.played();
        report(
            played > 0.3 && peak > 0.01,
            &format!(
                "one-player fixture: the output played {:.0} ms of the run, peak level {peak:.3}",
                played * 1e3
            ),
        );
    }

    // Ticket #81 (F41) Acceptance 5: pressing the bound P2 start key during
    // the two-credit 1P fight reaches the challenger screen (0x423b8).
    let challenger_fixture = resolve_local(CHALLENGER_FIXTURE);
    match CpuMachine::from_checkpoint_2389(&challenger_fixture, assets.maincpu.clone()) {
        Ok(mut m) => {
            let mut saw_fanfare = false;
            for f in 1..=240u32 {
                let mut held = [false; sf_engine::run::INPUTS];
                if (2..14).contains(&f) {
                    held[23] = true; // start2 through active_bindings.cabinet.start2
                }
                let (p1, p2, [c1, c2, s1, s2, svc]) =
                    active_bindings.round_trip_logical_frame(&held);
                session::apply_cpu_both(&mut m, p1, p2);
                m.set_cabinet_inputs(c1, c2, s1, s2, svc);
                m.step();
                if m.state.sound_latch == 0x3e {
                    saw_fanfare = true;
                }
                if f % 8 == 0 && window.is_open() {
                    let shown = m.render_with_overlay(&assets);
                    present(&mut window, &mut buf, &shown);
                }
            }
            let w = m.work_ram();
            let watch = u32::from_be_bytes([w[0xf24], w[0xf25], w[0xf26], w[0xf27]]);
            // 0x15ba2 writes its frame at video offset 0x052c (0x90c4); 0x15c30 writes 'H' at 0x0636 (0x9011).
            let banner = u16::from_be_bytes([m.state.videoram[0x052c], m.state.videoram[0x052d]]);
            let subtitle = u16::from_be_bytes([m.state.videoram[0x0636], m.state.videoram[0x0637]]);
            let ok = m.unrecovered().is_empty()
                && w[0x4008] == 0x10
                && w[0x65d] == 0
                && watch == 0x3f4c6
                && saw_fanfare
                && banner == 0x90c4
                && subtitle == 0x9011;
            report(
                ok,
                &format!(
                    "challenger join (#81): bound start2 ({}) during 2-credit 1P fight reached challenger screen (scene=0x{:02x}, credits={}, watch=0x{watch:05x}, banner=0x{banner:04x}, subtitle=0x{subtitle:04x})",
                    bindings::key_to_name(active_bindings.cabinet.start2).unwrap_or("?"),
                    w[0x4008],
                    w[0x65d],
                ),
            );
        }
        Err(e) => report(false, &format!("challenger fixture: {e}")),
    }

    // Power-on mode (ticket #55, F15): from frame 0 with no fixture and no
    // pre-roll, types the warning screen (frames 8..=553, held over 554..=793),
    // joins at 794, and shows the title screen through frame 1647.
    {
        let mut m = PowerOnMachine::new(assets.maincpu.clone());
        let po_start = sound_start_power_on();
        if let Some(s) = &sound {
            s.reset(&po_start);
        }
        let mut live_title = audio::LiveCapture::new(&po_start, 975, 1050);
        let non_blank_cells = |vram: &[u8]| -> usize {
            vram.chunks_exact(2)
                .filter(|c| u16::from_be_bytes([c[0], c[1]]) != 0x103c)
                .count()
        };
        let mut warning_cells_at_554 = 0usize;
        let mut warning_cells_at_794 = usize::MAX;
        let mut title_cells_at_1500 = 0usize;
        let mut latches = Vec::new();
        for f in 1..=WARNING_TITLE_LAST {
            m.step();
            if let Some(s) = &sound {
                s.push_frame(m.state.frame, m.state.sound_latch, m.state.latch_cycles);
            }
            if f <= 1050 {
                live_title.push_frame(m.state.frame, m.state.sound_latch, m.state.latch_cycles);
            }
            if m.state.sound_latch != 0xff && m.state.sound_latch != 0x00 {
                latches.push((f, m.state.sound_latch));
            }
            if f == 554 {
                warning_cells_at_554 = non_blank_cells(&m.state.videoram);
                if window.is_open() {
                    let shown = m.render_with_overlay(&assets);
                    present(&mut window, &mut buf, &shown);
                }
            } else if f == 794 {
                warning_cells_at_794 = non_blank_cells(&m.state.videoram);
            } else if f == 1500 {
                title_cells_at_1500 = non_blank_cells(&m.state.videoram);
                if window.is_open() {
                    let shown = m.render_with_overlay(&assets);
                    present(&mut window, &mut buf, &shown);
                }
            } else if f % 32 == 0 && window.is_open() {
                let shown = m.render_with_overlay(&assets);
                present(&mut window, &mut buf, &shown);
            }
        }
        report(
            warning_cells_at_554 == 249 && warning_cells_at_794 == 23,
            &format!(
                "power-on mode: warning screen typed ({warning_cells_at_554} non-blank cells at frame 554, erased and joined to {warning_cells_at_794} HUD/INSERT COIN cells at frame 794)"
            ),
        );
        report(
            title_cells_at_1500 > 50 && m.state.bg_scroll == 0x0480 && latches == vec![(1, 0x27), (2, 0x3f), (3, 0x80), (976, 0x1c)],
            &format!(
                "power-on mode: title screen shown ({title_cells_at_1500} non-blank cells at frame 1500, bg_scroll=0x{:04x}, latches {latches:02x?})",
                m.state.bg_scroll
            ),
        );
        report(
            m.unrecovered().is_empty() && m.state.frame == WARNING_TITLE_LAST,
            &format!(
                "power-on mode: {WARNING_TITLE_LAST} frames from power-on on translated paths"
            ),
        );
        let title_u = live_title.unrecovered();
        let title_samples = live_title.into_samples();
        let title_peak = title_samples.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(s) = &sound {
            let u = s.unrecovered();
            let note = if u.is_empty() && title_u.is_empty() {
                String::new()
            } else {
                format!(" (reached {u:04x?} / {title_u:04x?})")
            };
            report(
                u.is_empty() && title_u.is_empty(),
                &format!("power-on mode: the music CPU stayed on translated paths{note}"),
            );
            let (played, _) = s.played();
            report(
                played > 0.3 && title_peak > 0.01,
                &format!(
                    "power-on mode: the output played {:.0} ms from power-on with no pre-roll, title sound 0x1c peak level {title_peak:.3}",
                    played * 1e3
                ),
            );
        }

        // Ticket #89 (F49) Acceptance 5: power-on launcher plays into the
        // fight with a scripted coin and start (no fixture).
        let mut m_fight = PowerOnMachine::new(assets.maincpu.clone());
        let mut fight_latches = 0u32;
        for f in 1..=2500u32 {
            let mut held = [false; sf_engine::run::INPUTS];
            if (1450..1462).contains(&f) {
                held[20] = true; // coin1
            }
            if (1520..1532).contains(&f) {
                held[22] = true; // start1
            }
            if f >= 2390 {
                let p1 = script(f - 2389, 0);
                held[..4].copy_from_slice(&p1.directions);
                held[4..10].copy_from_slice(&p1.buttons);
            }
            let (p1, p2, [c1, c2, s1, s2, svc]) = active_bindings.round_trip_logical_frame(&held);
            apply_power_on(&mut m_fight, p1, p2);
            m_fight.set_cabinet_inputs(c1, c2, s1, s2, svc);
            m_fight.step();
            if let Some(s) = &sound {
                if f >= 2260 {
                    s.push_frame(
                        m_fight.state.frame,
                        m_fight.state.sound_latch,
                        m_fight.state.latch_cycles,
                    );
                }
            }
            if f >= 2260 && m_fight.state.sound_latch != 0xff {
                fight_latches += 1;
            }
            if f >= 2260 && f % 8 == 0 && window.is_open() {
                let shown = m_fight.render_with_overlay(&assets);
                present(&mut window, &mut buf, &shown);
            }
        }
        let w = m_fight.work_ram();
        let in_fight = m_fight.unrecovered().is_empty()
            && m_fight.state.frame == 2500
            && w[0x4002] == 2
            && w[0x4008] == 0x80
            && m_fight.state.objectram[0x0408] != 0
            && m_fight.state.objectram[0x0a08] != 0
            && fight_latches > 0;
        report(
            in_fight,
            &format!(
                "power-on mode: scripted coin+start played into the fight at frame 2500 with no fixture (mode={}, scene_mode=0x{:02x}, latches={fight_latches})",
                w[0x4002], w[0x4008]
            ),
        );
    }

    drop(sound);
    println!(
        "{}",
        if failures == 0 {
            "smoke check passed"
        } else {
            "smoke check FAILED"
        }
    );
    i32::from(failures != 0)
}

/// Load `path` (or fall back to `DEFAULT_BINDINGS` with a diagnostic note),
/// check that all 25 logical inputs are bound to distinct non-`Escape` keys,
/// and format the smoke check's binding-table report line.
pub fn smoke_bindings_check(
    path: &std::path::Path,
) -> (bindings::BindingsTable, bool, String, Option<String>) {
    let (table, diag) = bindings::BindingsTable::load_or_default(path);
    let mut keys: Vec<Key> = table.entries().into_iter().map(|(_, k)| k).collect();
    let n = keys.len();
    keys.sort_by_key(|k| *k as u32);
    keys.dedup();
    let ok = table.validate().is_ok() && keys.len() == n && n == 25;
    let line = format!(
        "25 distinct non-Escape keys bound for P1, P2, and cabinet ({})",
        table.summary()
    );
    (table, ok, line, diag)
}

/// The round music phrase (ticket #11), rendered by the translated music CPU
/// and the YM2151 model from the sf-music-01 fixture, played in a loop until
/// Enter is pressed.
fn play_music_phrase() {
    use sf_audio::check::{latch_timeline, read_taps, run_music, MUSIC_FIXTURE_IDLE_PHASE};
    let local = "../../tools/reference/local";
    let log = read_taps(&resolve_local(&format!(
        "{local}/captures/music-audio-a/audio_taps.tsv"
    )))
    .expect("reference audio run (tools/reference/run_audio_probe.py)");
    let ram = std::fs::read(resolve_local(&format!(
        "{local}/captures/music-01-run-a/ram_002279_music.bin"
    )))
    .expect("music RAM at the fixture (reference capture of sf-music-01)");
    let rom = sf_video::romset::load_region(
        resolve_local(&format!("{local}/roms/sf")),
        resolve_local(MANIFEST),
        "audiocpu",
    )
    .expect("audiocpu region (local ROM set)");
    let rate = sf_audio::opm::OPM_RATE as f64;
    let (t_fix, t_end) = (2280.0 / 60.0, 2360.0 / 60.0);
    let (s_fix, s_end) = (
        (t_fix * rate).ceil() as usize,
        (t_end * rate).ceil() as usize,
    );
    let run = run_music(
        rom,
        ram,
        &log,
        t_fix,
        latch_timeline(&log, None),
        MUSIC_FIXTURE_IDLE_PHASE,
        s_end as u64,
    );
    if !run.unrecovered.is_empty() {
        eprintln!("untranslated branches reached: {:04x?}", run.unrecovered);
    }
    let pcm: Vec<f32> = run.mix[s_fix..s_end].iter().map(|v| v * 0.6).collect();
    let Some(_player) = audio::Player::looping(pcm, sf_audio::opm::OPM_RATE) else {
        eprintln!("no usable audio output device");
        return;
    };
    println!("playing the round music phrase (frames 2280..=2359) in a loop; press Enter to stop");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

/// Resolve a path that starts with `../../tools/` whether invoked from
/// `crates/sf-dev` or from the repository root.
fn resolve_local(rel_from_crate: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(rel_from_crate);
    if p.exists() {
        return p.to_path_buf();
    }
    if let Some(stripped) = rel_from_crate.strip_prefix("../../") {
        let root_p = std::path::Path::new(stripped);
        if root_p.exists() || root_p.parent().is_some_and(|d| d.exists()) {
            return root_p.to_path_buf();
        }
    }
    p.to_path_buf()
}

/// Least-correlated active one-second slice (at `WAV_RATE`, 2 ms lag search;
/// silent inter-round seconds with RMS < 1e-4 are skipped) and its machine
/// time (seconds from power-on).
fn least_correlated_second(a: &[f32], b: &[f32], f0: u32) -> (f64, f64) {
    let sec = sf_audio::check::WAV_RATE as usize;
    let max_lag = (0.002 * sf_audio::check::WAV_RATE as f64) as isize;
    let mut worst = (1.0f64, 0usize);
    for (j, (ca, cb)) in a.chunks(sec).zip(b.chunks(sec)).enumerate() {
        let rms_a =
            (ca.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / ca.len().max(1) as f64).sqrt();
        let rms_b =
            (cb.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / cb.len().max(1) as f64).sqrt();
        if rms_a < 1e-4 && rms_b < 1e-4 {
            continue;
        }
        let (c, _) = sf_audio::check::best_correlation(ca, cb, max_lag);
        if c < worst.0 {
            worst = (c, j);
        }
    }
    (worst.0, f0 as f64 / 60.0 + worst.1 as f64)
}

/// Build and verify the fixed-fight listening kit (ticket #49, F9):
/// - local renders of the native mix at latencies 51 and 92 (over round 2,
///   frames 3830..=4999, and over the whole fixed fight, frames 2389..=4999);
/// - MAME's `fight-audio-a/audio.wav` aligned and sliced to those windows;
/// - the launcher's live output captured during a scripted run of the fixed
///   fight (`sf-fixed-fight-01`) through `CpuMachine` and `audio::LiveCapture`
///   at `IRQ_LATENCY_MID` (92), verified sample-for-sample identical to the
///   batch mix at latency 92.
fn listening_kit() -> i32 {
    use sf_audio::check::{
        best_correlation, latch_timeline, measure, read_taps, read_wav_mono, render_mix,
        NativeLatch, WAV_RATE,
    };
    use sf_audio::{write_wav, IRQ_LATENCY_MID};

    const NATIVE_FROM: u32 = 2389;
    const NATIVE_LAST: u32 = 5000;
    const SPLICE: u32 = NATIVE_FROM + 1;
    const ROUND2_FIRST: u32 = 3830;
    const ROUND2_LAST: u32 = 4999;
    const FITTED_LATENCY: u32 = 51;

    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let cpu_fixture = resolve_local(CPU_FIXTURE);
    let cpu_sound = resolve_local(CPU_SOUND);
    let wav_ref = resolve_local("../../tools/reference/local/captures/fight-audio-a/audio.wav");
    let renders = resolve_local("../../tools/reference/local/renders");
    std::fs::create_dir_all(&renders).expect("create local renders dir");

    let assets = load_assets(&roms, &manifest).expect("local ROM set");
    let audio2 = sf_video::romset::load_region(&roms, &manifest, "audio2").expect("audio2 region");
    let audiocpu =
        sf_video::romset::load_region(&roms, &manifest, "audiocpu").expect("audiocpu region");
    let log = read_taps(&cpu_sound).expect("reference audio_taps.tsv");
    let cut = (NATIVE_FROM + 1) as f64 / sf_audio::FRAME_HZ as f64;
    let preroll =
        sf_audio::LatchTimeline::new(log.latch.iter().copied().filter(|e| e.0 < cut).collect());
    let start = audio::SoundStart {
        audio2: audio2.clone(),
        audiocpu: audiocpu.clone(),
        frame: NATIVE_FROM,
        preroll,
    };

    // Scripted launcher run of sf-fixed-fight-01 from checkpoint 2389 through
    // frame 5000, feeding P1's inputs via P1_KEYS.inputs(...) and capturing
    // the live output over both Round 2 (3830..=4999) and the full fight
    // (2389..=4999).
    let mut machine = CpuMachine::from_checkpoint_2389(&cpu_fixture, assets.maincpu.clone())
        .expect("checkpoint-2389 fixture");
    let mut live_round2 = audio::LiveCapture::new(&start, ROUND2_FIRST, ROUND2_LAST);
    let mut live_fight = audio::LiveCapture::new(&start, NATIVE_FROM, ROUND2_LAST);
    let mut frames = Vec::with_capacity((NATIVE_LAST - NATIVE_FROM + 1) as usize);
    frames.push((machine.state.frame, machine.state.sound_latch, 0));
    while machine.state.frame < NATIVE_LAST {
        let f = machine.state.frame;
        let p1 = P1_KEYS.inputs(|k| match k {
            Key::Right => (2400..2580).contains(&f),
            Key::A => (2590..2600).contains(&f),
            Key::Z => (2630..2640).contains(&f),
            _ => false,
        });
        apply_cpu(&mut machine, p1);
        machine.step();
        let (frame, latch, cycles) = (
            machine.state.frame,
            machine.state.sound_latch,
            machine.state.latch_cycles,
        );
        frames.push((frame, latch, cycles));
        live_round2.push_frame(frame, latch, cycles);
        live_fight.push_frame(frame, latch, cycles);
    }
    assert!(
        machine.unrecovered().is_empty(),
        "68000 unrecovered: {:05x?}",
        machine.unrecovered()
    );
    assert!(
        live_round2.unrecovered().is_empty(),
        "music CPU unrecovered (round2): {:04x?}",
        live_round2.unrecovered()
    );
    assert!(
        live_fight.unrecovered().is_empty(),
        "music CPU unrecovered (fight): {:04x?}",
        live_fight.unrecovered()
    );
    let live_round2_pcm = live_round2.into_samples();
    let live_fight_pcm = live_fight.into_samples();

    // Batch renders at latencies 51 and 92.
    let tl_51 = latch_timeline(
        &log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE,
            latency: FITTED_LATENCY,
        }),
    );
    let tl_92 = latch_timeline(
        &log,
        Some(NativeLatch {
            frames: &frames,
            from: SPLICE,
            latency: IRQ_LATENCY_MID,
        }),
    );
    let round2_51 = render_mix(
        audio2.clone(),
        audiocpu.clone(),
        tl_51.clone(),
        ROUND2_FIRST,
        ROUND2_LAST,
    );
    let round2_92 = render_mix(
        audio2.clone(),
        audiocpu.clone(),
        tl_92.clone(),
        ROUND2_FIRST,
        ROUND2_LAST,
    );
    let fight_51 = render_mix(
        audio2.clone(),
        audiocpu.clone(),
        tl_51,
        NATIVE_FROM,
        ROUND2_LAST,
    );
    let fight_92 = render_mix(audio2, audiocpu, tl_92, NATIVE_FROM, ROUND2_LAST);

    // Reference audio.wav aligned and sliced to Round 2 and to the full fight.
    let (ref_all, wav_rate) = read_wav_mono(&wav_ref).expect("reference audio.wav");
    assert_eq!(wav_rate, WAV_RATE);
    let r2_start = (ROUND2_FIRST as f64 / 60.0 * WAV_RATE as f64).round() as usize;
    let fight_start = (NATIVE_FROM as f64 / 60.0 * WAV_RATE as f64).round() as usize;
    let round2_mame = &ref_all[r2_start..r2_start + round2_51.len()];
    let fight_mame = &ref_all[fight_start..fight_start + fight_51.len()];

    // Write all local WAV files into tools/reference/local/renders/.
    let files: [(&str, &[f32]); 8] = [
        ("round2-native-mix-51.wav", &round2_51),
        ("round2-native-mix-92.wav", &round2_92),
        ("round2-mame-reference.wav", round2_mame),
        ("round2-launcher-live-92.wav", &live_round2_pcm),
        ("fight-native-mix-51.wav", &fight_51),
        ("fight-native-mix-92.wav", &fight_92),
        ("fight-mame-reference.wav", fight_mame),
        ("fight-launcher-live-92.wav", &live_fight_pcm),
    ];
    for (name, pcm) in files {
        let path = renders.join(name);
        write_wav(&path, pcm, WAV_RATE, 1).expect("write local render WAV");
        println!(
            "wrote {} ({} samples, {:.3} s)",
            path.display(),
            pcm.len(),
            pcm.len() as f64 / WAV_RATE as f64
        );
    }

    // Verify live capture == batch mix sample for sample (and WAV byte for byte).
    let r2_live_diff = live_round2_pcm
        .iter()
        .zip(&round2_92)
        .filter(|(a, b)| a != b)
        .count();
    let fight_live_diff = live_fight_pcm
        .iter()
        .zip(&fight_92)
        .filter(|(a, b)| a != b)
        .count();
    println!(
        "live vs batch (round 2, lat 92): {r2_live_diff} of {} samples differ",
        round2_92.len()
    );
    println!(
        "live vs batch (full fight, lat 92): {fight_live_diff} of {} samples differ",
        fight_92.len()
    );
    assert_eq!(
        r2_live_diff, 0,
        "round 2 live capture must equal batch mix sample for sample"
    );
    assert_eq!(
        fight_live_diff, 0,
        "full fight live capture must equal batch mix sample for sample"
    );

    // Print measures for Round 2 (3830..=4999) and Full Fight (2389..=4999).
    let rate = WAV_RATE as f64;
    let max_lag = (0.020 * rate) as isize;
    for (window_label, f0, m51_pcm, m92_pcm, ref_pcm) in [
        (
            "round2 (3830..=4999)",
            ROUND2_FIRST,
            &round2_51[..],
            &round2_92[..],
            round2_mame,
        ),
        (
            "full fight (2389..=4999)",
            NATIVE_FROM,
            &fight_51[..],
            &fight_92[..],
            fight_mame,
        ),
    ] {
        let m51 = measure(m51_pcm, rate);
        let m92 = measure(m92_pcm, rate);
        let mr = measure(ref_pcm, rate);
        let (c51, lag51) = best_correlation(m51_pcm, ref_pcm, max_lag);
        let (c92, lag92) = best_correlation(m92_pcm, ref_pcm, max_lag);
        let (c92_51, lag92_51) = best_correlation(m92_pcm, m51_pcm, max_lag);
        let (w51, wt51) = least_correlated_second(m51_pcm, ref_pcm, f0);
        let (w92, wt92) = least_correlated_second(m92_pcm, ref_pcm, f0);
        let (w92_51, wt92_51) = least_correlated_second(m92_pcm, m51_pcm, f0);
        println!("=== {window_label} ===");
        for (tag, m) in [("native-51", &m51), ("native-92", &m92), ("mame-ref", &mr)] {
            println!(
                "  {tag}: onset {:.0} ms ({:.2} ms), active {:.0} ms ({:.1} ms), rms {:.4}, peak {:.4}, zcr {:.0}/s, centroid {:.0} Hz",
                m.onset * 1e3, m.onset * 1e3, m.active * 1e3, m.active * 1e3, m.rms, m.peak, m.zcr, m.centroid
            );
        }
        println!(
            "  native-51 vs mame: rms_ratio {:.4}, corr {c51:.4} at lag {lag51} samples ({:.2} ms), least-corr 1s {w51:.4} at {wt51:.2} s",
            m51.rms / mr.rms, lag51 as f64 / rate * 1e3
        );
        println!(
            "  native-92 vs mame: rms_ratio {:.4}, corr {c92:.4} at lag {lag92} samples ({:.2} ms), least-corr 1s {w92:.4} at {wt92:.2} s",
            m92.rms / mr.rms, lag92 as f64 / rate * 1e3
        );
        println!(
            "  native-92 vs native-51: rms_ratio {:.4}, corr {c92_51:.4} at lag {lag92_51} samples ({:.2} ms), least-corr 1s {w92_51:.4} at {wt92_51:.2} s",
            m92.rms / m51.rms, lag92_51 as f64 / rate * 1e3
        );
        if f0 == ROUND2_FIRST {
            assert_eq!(
                (m51.onset * 1e3).round() as u32,
                180,
                "51 render round-2 onset must be 180 ms"
            );
            assert!(
                (m51.rms / mr.rms - 1.0).abs() < 0.02,
                "51 render round-2 RMS ratio within 2%"
            );
            assert!(
                c51 >= 0.98 && lag51.abs() <= 8,
                "51 render round-2 correlation >= 0.98 within 8 samples of lag"
            );
        }
    }
    0
}

fn onset_and_rms(x: &[f32], rate: f64) -> (f64, f64) {
    let win = (0.005 * rate) as usize;
    let env: Vec<f64> = x
        .chunks(win.max(1))
        .map(|c| (c.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / c.len() as f64).sqrt())
        .collect();
    let peak_env = env.iter().cloned().fold(0.0, f64::max);
    let on = env.iter().position(|e| *e > 0.1 * peak_env).unwrap_or(0);
    let rms = (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt();
    (on as f64 * win as f64 / rate, rms)
}

/// Build and measure the sound-test listening kit (ticket #126, F54):
/// - renders of F22 (`sf-sound-test-01`), F23 (`sf-sound-test-02`),
///   F38 (`sf-sound-test-03`) and F24 (`sf-sound-test-04`) against MAME's
///   `audio.wav` in `tools/reference/local/renders/`;
/// - per-command `onset`, `RMS` (`native`, `MAME`, `ratio`) and `correlation`
///   written to `tools/reference/local/renders/sound-test-measures.tsv` and
///   printed to stdout.
fn sound_test_listening_kit() -> i32 {
    use sf_audio::check::{
        best_correlation, latch_timeline, read_taps, read_wav_mono, render_mix, WAV_RATE,
    };
    use sf_audio::write_wav;

    let roms = resolve_local(ROMS);
    let manifest = resolve_local(MANIFEST);
    let renders = resolve_local("../../tools/reference/local/renders");
    std::fs::create_dir_all(&renders).expect("create local renders dir");

    let audio2 = sf_video::romset::load_region(&roms, &manifest, "audio2").expect("audio2 region");
    let audiocpu =
        sf_video::romset::load_region(&roms, &manifest, "audiocpu").expect("audiocpu region");
    let rate = WAV_RATE as f64;
    let max_lag = (0.005 * rate) as isize;

    let suites: [(&str, &str, u32, u32); 4] = [
        (
            "sound-test-01",
            "F22 (#62) FM effects & specials",
            107,
            8699,
        ),
        ("sound-test-02", "F23 (#63) all 21 tunes", 507, 39774),
        (
            "sound-test-03",
            "F38 (#78) tune transitions & stops",
            507,
            6249,
        ),
        (
            "sound-test-04",
            "F24 (#64) all 63 ADPCM samples & stops",
            1377,
            12759,
        ),
    ];

    let mut tsv = String::from(
        "suite\tindex\tcmd\tframe_start\tframe_end\tduration_s\tnative_onset_ms\tmame_onset_ms\tnative_rms\tmame_rms\trms_ratio\tcorr\tlag_samples\n",
    );

    for (name, label, first, last) in suites {
        let cap_dir = resolve_local(&format!(
            "../../tools/reference/local/captures/{name}-audio-a"
        ));
        let log = read_taps(&cap_dir.join("audio_taps.tsv")).expect("read audio_taps.tsv");
        let (ref_all, wav_rate) =
            read_wav_mono(&cap_dir.join("audio.wav")).expect("read audio.wav");
        assert_eq!(wav_rate, WAV_RATE);

        let tl = latch_timeline(&log, None);
        let native_pcm = render_mix(audio2.clone(), audiocpu.clone(), tl, first, last);
        let ref_start = (first as f64 / 60.0 * rate).round() as usize;
        let ref_pcm = &ref_all[ref_start..(ref_start + native_pcm.len()).min(ref_all.len())];
        let n_common = native_pcm.len().min(ref_pcm.len());
        let native_pcm = &native_pcm[..n_common];
        let ref_pcm = &ref_pcm[..n_common];

        let nat_wav = renders.join(format!("{name}-native-mix.wav"));
        let ref_wav = renders.join(format!("{name}-mame-reference.wav"));
        write_wav(&nat_wav, native_pcm, WAV_RATE, 1).expect("write native wav");
        write_wav(&ref_wav, ref_pcm, WAV_RATE, 1).expect("write mame wav");

        let (_, nat_rms) = onset_and_rms(native_pcm, rate);
        let (_, ref_rms) = onset_and_rms(ref_pcm, rate);
        let (suite_corr, suite_lag) = best_correlation(native_pcm, ref_pcm, max_lag);
        println!(
            "=== {name}: {label} (frames {first}..={last}, {:.2} s) ===",
            n_common as f64 / rate
        );
        println!("  wrote {} and {}", nat_wav.display(), ref_wav.display());
        println!(
            "  whole window: native_rms={nat_rms:.5}, mame_rms={ref_rms:.5}, ratio={:.4}, corr={suite_corr:.4} at lag {suite_lag} samples",
            nat_rms / ref_rms.max(1e-12)
        );

        let cmds: Vec<(u32, u8)> = log
            .latch
            .iter()
            .filter(|&&(t, v)| t > 20.0 / 60.0 && v != 0xff)
            .filter(|&&(_, v)| name == "sound-test-04" || v < 0x40)
            .map(|&(t, v)| ((t * 60.0).floor() as u32, v))
            .collect();

        for (idx, &(f_cmd, cmd)) in cmds.iter().enumerate() {
            let f_end = cmds
                .get(idx + 1)
                .map(|&(next_f, _)| next_f.saturating_sub(1).max(f_cmd))
                .unwrap_or(last);
            let s0 = ((f_cmd.saturating_sub(first) as f64 / 60.0) * rate).round() as usize;
            let s1 = (((f_end + 1 - first) as f64 / 60.0) * rate)
                .round()
                .min(n_common as f64) as usize;
            if s0 >= s1 {
                continue;
            }
            let n_slice = &native_pcm[s0..s1];
            let r_slice = &ref_pcm[s0..s1];
            let (n_on, n_rms) = onset_and_rms(n_slice, rate);
            let (r_on, r_rms) = onset_and_rms(r_slice, rate);
            let (corr, lag) = if n_rms < 1e-6 && r_rms < 1e-6 {
                (1.0, 0)
            } else {
                best_correlation(n_slice, r_slice, max_lag)
            };
            let ratio = if r_rms < 1e-6 && n_rms < 1e-6 {
                1.0
            } else {
                n_rms / r_rms.max(1e-12)
            };
            let dur = (s1 - s0) as f64 / rate;
            tsv.push_str(&format!(
                "{name}\t{idx}\t0x{cmd:02x}\t{f_cmd}\t{f_end}\t{dur:.3}\t{:.1}\t{:.1}\t{n_rms:.5}\t{r_rms:.5}\t{ratio:.4}\t{corr:.4}\t{lag}\n",
                n_on * 1e3,
                r_on * 1e3
            ));
        }
    }

    let tsv_path = renders.join("sound-test-measures.tsv");
    std::fs::write(&tsv_path, &tsv).expect("write sound-test-measures.tsv");
    println!("wrote {}", tsv_path.display());
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Non-default 25-key binding table used across the ticket #76 (F36) tests:
    /// every one of the 25 logical inputs is bound to a different key than in
    /// `DEFAULT_BINDINGS`.
    const REMAPPED_BINDINGS: bindings::BindingsTable = bindings::BindingsTable {
        p1: Bindings {
            directions: [Key::NumPad6, Key::NumPad4, Key::NumPad2, Key::NumPad8],
            buttons: [Key::Q, Key::W, Key::E, Key::R, Key::V, Key::B],
        },
        p2: Bindings {
            directions: [Key::F4, Key::F2, Key::F3, Key::F1],
            buttons: [Key::F5, Key::F6, Key::F7, Key::F8, Key::F9, Key::F10],
        },
        cabinet: bindings::CabinetBindings {
            coin1: Key::F11,
            coin2: Key::F12,
            start1: Key::Enter,
            start2: Key::Space,
            service1: Key::Backspace,
        },
        ..bindings::DEFAULT_BINDINGS
    };

    /// Each bound key reaches exactly its documented logical input
    /// (launcher.md's table; [`Inputs`] order: right, left, down, up, then
    /// LP, MP, HP, LK, MK, HK — the order the machines' setters and the
    /// scenario inputs use), and for all 25 logical inputs (`p1_right`..
    /// `p1_button6`, `p2_right`..`p2_button6`, `coin1`, `coin2`, `start1`,
    /// `start2`, `service1`) reaches its exact active-low bit of `Io` (`in0`,
    /// `in1`, `system`), and only that bit, for both `DEFAULT_BINDINGS` and
    /// `REMAPPED_BINDINGS` (ticket #76, F36 Acceptance 2).
    #[test]
    fn every_key_reaches_its_documented_input() {
        let tables: [(&Bindings, [Key; 10]); 2] = [
            (
                &P1_KEYS,
                [
                    Key::Right,
                    Key::Left,
                    Key::Down,
                    Key::Up,
                    Key::A,
                    Key::S,
                    Key::D,
                    Key::Z,
                    Key::X,
                    Key::C,
                ],
            ),
            (
                &P2_KEYS,
                [
                    Key::H,
                    Key::F,
                    Key::G,
                    Key::T,
                    Key::U,
                    Key::I,
                    Key::O,
                    Key::J,
                    Key::K,
                    Key::L,
                ],
            ),
        ];
        for (bindings, keys) in tables {
            for (slot, key) in keys.into_iter().enumerate() {
                let i = bindings.inputs(|k| k == key);
                let held: Vec<bool> = i.directions.into_iter().chain(i.buttons).collect();
                let expected: Vec<bool> = (0..10).map(|s| s == slot).collect();
                assert_eq!(held, expected, "{key:?} drives input {slot} only");
            }
            let none = bindings.inputs(|_| false);
            assert!(
                none.directions.iter().chain(&none.buttons).all(|h| !h),
                "no key, no input"
            );
        }

        // Exact active-low bit position in (in0, in1, system) for each of the
        // 25 LogicalInput variants in canonical order (sf-mame0289.cpp).
        let expected_io_masks: [(u16, u16, u16); 25] = [
            (0xffff, !(1 << 0), 0xffff),  // p1_right
            (0xffff, !(1 << 1), 0xffff),  // p1_left
            (0xffff, !(1 << 2), 0xffff),  // p1_down
            (0xffff, !(1 << 3), 0xffff),  // p1_up
            (0xffff, !(1 << 4), 0xffff),  // p1_button1 (LP)
            (0xffff, !(1 << 5), 0xffff),  // p1_button2 (MP)
            (!(1 << 9), 0xffff, 0xffff),  // p1_button3 (HP)
            (0xffff, !(1 << 6), 0xffff),  // p1_button4 (LK)
            (0xffff, !(1 << 7), 0xffff),  // p1_button5 (MK)
            (!(1 << 2), 0xffff, 0xffff),  // p1_button6 (HK)
            (0xffff, !(1 << 8), 0xffff),  // p2_right
            (0xffff, !(1 << 9), 0xffff),  // p2_left
            (0xffff, !(1 << 10), 0xffff), // p2_down
            (0xffff, !(1 << 11), 0xffff), // p2_up
            (0xffff, !(1 << 12), 0xffff), // p2_button1 (LP)
            (0xffff, !(1 << 13), 0xffff), // p2_button2 (MP)
            (!(1 << 10), 0xffff, 0xffff), // p2_button3 (HP)
            (0xffff, !(1 << 14), 0xffff), // p2_button4 (LK)
            (0xffff, !(1 << 15), 0xffff), // p2_button5 (MK)
            (!(1 << 8), 0xffff, 0xffff),  // p2_button6 (HK)
            (!(1 << 0), 0xffff, 0xffff),  // coin1
            (!(1 << 1), 0xffff, 0xffff),  // coin2
            (0xffff, 0xffff, !(1 << 0)),  // start1
            (0xffff, 0xffff, !(1 << 1)),  // start2
            (0xffff, 0xffff, !(1 << 2)),  // service1
        ];

        for (label, table) in [
            ("default", bindings::DEFAULT_BINDINGS),
            ("remapped", REMAPPED_BINDINGS),
        ] {
            table.validate().expect("table must be valid");
            for (idx, (input, key)) in table.entries().into_iter().enumerate() {
                if label == "remapped" {
                    assert_ne!(
                        key,
                        bindings::DEFAULT_BINDINGS.key(input),
                        "remapped table must differ from default on {}",
                        input.name()
                    );
                }
                let io = table.io(0xdfff, 0xffff, |k| k == key);
                assert_eq!(
                    (io.in0, io.in1, io.system),
                    expected_io_masks[idx],
                    "{label}: {} ({key:?}) must drive only its exact Io bit",
                    input.name()
                );
            }
            let idle = table.io(0xdfff, 0xffff, |_| false);
            assert_eq!(
                (idle.in0, idle.in1, idle.system),
                (0xffff, 0xffff, 0xffff),
                "{label}: no key held must leave all active-low bits high"
            );
        }
    }

    /// Ticket #76 (F36) Acceptance 1 & 5:
    /// - Round-trip save/load of `DEFAULT_BINDINGS` and `REMAPPED_BINDINGS`
    ///   through `bindings.toml` equals the original;
    /// - Missing, corrupt, or partial files fall back to `DEFAULT_BINDINGS`
    ///   and report why;
    /// - Duplicate keys and `Key::Escape` are refused on both `from_toml` and
    ///   `rebind`;
    /// - `rebind_and_save` (and `cmd_rebind`) updates and persists the table;
    /// - `smoke_bindings_check` reports the active binding table.
    #[test]
    fn bindings_toml_roundtrip_fallback_validation_and_rebind() {
        let scratch = sf_engine::run::ScratchDir::new("sf-dev-f36-bindings");
        std::fs::create_dir_all(&*scratch).unwrap();

        // 1. Missing file falls back to defaults and reports why.
        let missing_path = scratch.join("nonexistent_bindings.toml");
        let (fallback, diag) = bindings::BindingsTable::load_or_default(&missing_path);
        assert_eq!(fallback, bindings::DEFAULT_BINDINGS);
        let msg = diag.expect("missing file must produce fallback diagnostic");
        assert!(
            msg.contains("falling back to default keyboard bindings"),
            "unexpected diagnostic: {msg}"
        );

        // 2. Save then load round-trips both DEFAULT_BINDINGS and REMAPPED_BINDINGS.
        let toml_path = scratch.join("nested").join("bindings.toml");
        for table in [bindings::DEFAULT_BINDINGS, REMAPPED_BINDINGS] {
            table.save(&toml_path).expect("save bindings.toml");
            let loaded = bindings::BindingsTable::load(&toml_path).expect("load bindings.toml");
            assert_eq!(loaded, table, "save/load round-trip must equal original");
            let (loaded2, diag2) = bindings::BindingsTable::load_or_default(&toml_path);
            assert_eq!(loaded2, table);
            assert!(
                diag2.is_none(),
                "valid file must not emit fallback diagnostic"
            );
        }

        // 3. Corrupt syntax falls back to defaults and reports why.
        let corrupt_path = scratch.join("corrupt.toml");
        std::fs::write(&corrupt_path, "this is not valid toml\n").unwrap();
        assert!(bindings::BindingsTable::load(&corrupt_path).is_err());
        let (corrupt_fb, corrupt_diag) = bindings::BindingsTable::load_or_default(&corrupt_path);
        assert_eq!(corrupt_fb, bindings::DEFAULT_BINDINGS);
        assert!(corrupt_diag.unwrap().contains("expected 'input = \"Key\"'"));

        // 4. Partial file (missing keys) falls back to defaults and reports why.
        let partial_path = scratch.join("partial.toml");
        std::fs::write(&partial_path, "p1_right = \"Right\"\np1_left = \"Left\"\n").unwrap();
        let (partial_fb, partial_diag) = bindings::BindingsTable::load_or_default(&partial_path);
        assert_eq!(partial_fb, bindings::DEFAULT_BINDINGS);
        let pmsg = partial_diag.unwrap();
        assert!(
            pmsg.contains("missing bindings for") && pmsg.contains("service1"),
            "unexpected partial diagnostic: {pmsg}"
        );

        // 5. Duplicate keys and Escape are refused in TOML and in `rebind`.
        let mut esc_toml = bindings::DEFAULT_BINDINGS.to_toml();
        esc_toml = esc_toml.replace("p1_button1 = \"A\"", "p1_button1 = \"Escape\"");
        let esc_err = bindings::BindingsTable::from_toml(&esc_toml).unwrap_err();
        assert!(
            esc_err.contains("cannot bind Escape"),
            "unexpected error: {esc_err}"
        );

        let mut dup_toml = bindings::DEFAULT_BINDINGS.to_toml();
        dup_toml = dup_toml.replace("p2_button1 = \"U\"", "p2_button1 = \"A\"");
        let dup_err = bindings::BindingsTable::from_toml(&dup_toml).unwrap_err();
        assert!(
            dup_err.contains("duplicate key 'A'"),
            "unexpected error: {dup_err}"
        );

        let mut table = bindings::DEFAULT_BINDINGS;
        assert!(table
            .rebind(bindings::LogicalInput::P1Button1, Key::Escape)
            .is_err());
        assert_eq!(
            table,
            bindings::DEFAULT_BINDINGS,
            "failed rebind must not mutate table"
        );
        assert!(table
            .rebind(bindings::LogicalInput::P2Button1, Key::A)
            .is_err());
        assert_eq!(
            table,
            bindings::DEFAULT_BINDINGS,
            "duplicate rebind must not mutate table"
        );

        // 6. Rebinding a key updates and saves the table, and smoke_bindings_check
        //    reports the active binding table.
        bindings::DEFAULT_BINDINGS.save(&toml_path).unwrap();
        let rc = cmd_rebind(&[
            "p1_button1".to_string(),
            "Q".to_string(),
            toml_path.to_string_lossy().into_owned(),
        ]);
        assert_eq!(rc, 0);
        let rebound = bindings::BindingsTable::load(&toml_path).unwrap();
        assert_eq!(rebound.key(bindings::LogicalInput::P1Button1), Key::Q);
        let (smoke_table, smoke_ok, smoke_line, smoke_diag) = smoke_bindings_check(&toml_path);
        assert!(smoke_ok);
        assert!(smoke_diag.is_none());
        assert_eq!(smoke_table, rebound);
        assert!(
            smoke_line.contains("P1 dir=Right/Left/Down/Up btn=Q/S/D/Z/X/C"),
            "smoke line must report active rebound table: {smoke_line}"
        );
    }

    /// Ticket #76 (F36) Acceptance 3:
    /// `sf-fixed-fight-audit-01`'s P1 inputs (`2390..=5014`), converted from
    /// scenario inputs to the bound physical keys of `REMAPPED_BINDINGS` and
    /// decoded back through `Bindings::inputs` on every frame, reproduce
    /// `fixed-fight-audit-01-run-a` over `2390..=5014` (`2,625` frames, owned
    /// work RAM + object RAM + scheduler slot state/sleep bytes).
    #[test]
    fn remapped_p1_bindings_reproduce_fixed_fight_audit_01() {
        const AUDIT_REF: &str = "../../tools/reference/local/captures/fight-audit-run-a";
        const AUDIT_SCEN: &str = "../../tools/reference/scenarios/sf_fixed_fight_audit_01.json";
        if ![ROMS, AUDIT_REF, AUDIT_SCEN]
            .iter()
            .all(|p| resolve_local(p).exists())
        {
            eprintln!("skipping: local reference captures/ROMs not present");
            return;
        }
        let roms = resolve_local(ROMS);
        let manifest = resolve_local(MANIFEST);
        let reference = resolve_local(AUDIT_REF);
        let scenario = resolve_local(AUDIT_SCEN);
        let assets = load_assets(&roms, &manifest).expect("local ROM set");

        let mut m = CpuMachine::from_checkpoint_2389(&reference, assets.maincpu.clone())
            .expect("checkpoint 2389");
        let (dsw1, dsw2) = sf_engine::run::scenario_dsw(&scenario).expect("scenario dsw");
        m.set_dsw(dsw1, dsw2);
        let events = sf_engine::run::scenario_input_events(&scenario).expect("scenario events");
        let mut next = 0usize;
        let mut held = [false; sf_engine::run::INPUTS];
        let mut compared = 0u32;

        while m.state.frame < 5014 {
            let frame = m.state.frame + 1;
            while next < events.len() && events[next].0 < frame {
                held[events[next].1] = events[next].2;
                next += 1;
            }
            let (p1, p2, [c1, c2, s1, s2, svc]) = REMAPPED_BINDINGS.round_trip_logical_frame(&held);
            session::apply_cpu_both(&mut m, p1, p2);
            m.set_cabinet_inputs(c1, c2, s1, s2, svc);
            m.step();
            assert!(
                m.unrecovered().is_empty(),
                "frame {frame}: unrecovered branches {:05x?}",
                m.unrecovered()
            );
            let work = std::fs::read(reference.join(format!("ram_{frame:06}_work.bin")))
                .expect("reference work RAM");
            let native_work = m.work_ram();
            let slots = (0..16u32)
                .flat_map(|k| [0xff8006 + 0x10 * k, 0xff8007 + 0x10 * k])
                .map(|a| ("task slot state / sleep", a, a + 1));
            for (name, a, b) in sf_engine::run::CPU_RAM_REGIONS.iter().copied().chain(slots) {
                for addr in a..b {
                    let i = (addr - 0xff8000) as usize;
                    let native = if addr >= 0xffe000 {
                        m.state.objectram[i - 0x6000]
                    } else {
                        native_work[i]
                    };
                    assert_eq!(
                        work[i], native,
                        "frame {frame}: {name} differs at 0x{addr:06x}: ref {:02x}, native {native:02x}",
                        work[i]
                    );
                }
            }
            compared += 1;
        }
        assert_eq!(compared, 2625);
    }

    /// Ticket #76 (F36) Acceptance 4:
    /// `sf-blocks-01`'s P1 and P2 inputs (`2451..=4320`), converted from
    /// scenario inputs to the bound physical keys of `REMAPPED_BINDINGS` for
    /// both players and decoded back through `Bindings::inputs` on every
    /// frame, reproduce `blocks-01-run-a` over `2450..=4320` (`1,870` frames).
    #[test]
    fn remapped_p1_and_p2_bindings_reproduce_blocks_01() {
        const BLOCKS_REF: &str = "../../tools/reference/local/captures/blocks-01-run-a";
        const BLOCKS_SCEN: &str = "../../tools/reference/scenarios/sf_blocks_01.json";
        if ![ROMS, BLOCKS_REF, BLOCKS_SCEN]
            .iter()
            .all(|p| resolve_local(p).exists())
        {
            eprintln!("skipping: local reference captures/ROMs not present");
            return;
        }
        let roms = resolve_local(ROMS);
        let manifest = resolve_local(MANIFEST);
        let reference = resolve_local(BLOCKS_REF);
        let scenario = resolve_local(BLOCKS_SCEN);
        let assets = load_assets(&roms, &manifest).expect("local ROM set");

        let mut m = VersusMachine::from_checkpoint_2450_with_program(&reference, assets.maincpu)
            .expect("checkpoint 2450");
        let (dsw1, dsw2) = sf_engine::run::scenario_dsw(&scenario).expect("scenario dsw");
        m.set_dsw(dsw1, dsw2);
        let events = sf_engine::run::scenario_input_events(&scenario).expect("scenario events");
        let mut next = 0usize;
        let mut held = [false; sf_engine::run::INPUTS];
        let mut compared = 0u32;

        while m.state.frame < 4320 {
            let frame = m.state.frame + 1;
            while next < events.len() && events[next].0 < frame {
                held[events[next].1] = events[next].2;
                next += 1;
            }
            let (p1, p2, [c1, c2, s1, s2, svc]) = REMAPPED_BINDINGS.round_trip_logical_frame(&held);
            apply_versus(&mut m, p1, p2);
            m.set_cabinet_inputs(c1, c2, s1, s2, svc);
            m.step();
            assert!(
                m.unrecovered().is_empty(),
                "frame {frame}: unrecovered branches {:05x?}",
                m.unrecovered()
            );
            let work = std::fs::read(reference.join(format!("ram_{frame:06}_work.bin")))
                .expect("reference work RAM");
            let native_work = m.work_ram();
            let slots = (0..16u32)
                .flat_map(|k| [0xff8006 + 0x10 * k, 0xff8007 + 0x10 * k])
                .map(|a| ("task slot state / sleep", a, a + 1));
            for (name, a, b) in sf_engine::run::VERSUS_RAM_REGIONS
                .iter()
                .copied()
                .chain(slots)
            {
                for addr in a..b {
                    let i = (addr - 0xff8000) as usize;
                    let native = if addr >= 0xffe000 {
                        m.state.objectram[i - 0x6000]
                    } else {
                        native_work[i]
                    };
                    assert_eq!(
                        work[i], native,
                        "frame {frame}: {name} differs at 0x{addr:06x}: ref {:02x}, native {native:02x}",
                        work[i]
                    );
                }
            }
            compared += 1;
        }
        assert_eq!(compared, 1870);
    }

    /// Ticket #49 (F9): the fixed-fight listening kit builds the local WAV
    /// renders at latencies 51 and 92, aligns MAME's `audio.wav` to them,
    /// captures the launcher's live output during a scripted run at latency 92,
    /// verifies the 51 render's round-2 measures against `both-chips.md`, and
    /// asserts sample-for-sample (and WAV byte-for-byte) equality between the
    /// live capture and the batch mix at latency 92.
    #[test]
    fn scripted_fixed_fight_live_capture_equals_batch_mix_and_writes_local_kit() {
        let required = [
            ROMS,
            CPU_FIXTURE,
            CPU_SOUND,
            "../../tools/reference/local/captures/fight-audio-a/audio.wav",
        ];
        if !required.iter().all(|p| resolve_local(p).exists()) {
            eprintln!("skipping: local reference captures/ROMs not present");
            return;
        }
        assert_eq!(listening_kit(), 0);
        let renders = resolve_local("../../tools/reference/local/renders");
        for (live_wav, batch_wav) in [
            ("round2-launcher-live-92.wav", "round2-native-mix-92.wav"),
            ("fight-launcher-live-92.wav", "fight-native-mix-92.wav"),
        ] {
            let a = std::fs::read(renders.join(live_wav)).expect("read live wav");
            let b = std::fs::read(renders.join(batch_wav)).expect("read batch wav");
            assert_eq!(a, b, "{live_wav} and {batch_wav} must be byte-identical");
        }
    }

    /// Ticket #50 (F10) Acceptance 1: For every session it converts, the
    /// converter writes `observations.window = [fixture_frame, last_frame]`
    /// and `observations.checkpoints = [fixture_frame..=last_frame]` for both
    /// the one-player fixture (`2389`) and the versus fixture (`2450`).
    #[test]
    fn converter_writes_fixture_to_end_window_and_every_frame_checkpoint_for_both_fixtures() {
        for (kind, expected_first) in [
            (session::FixtureKind::Cpu, 2389u32),
            (session::FixtureKind::Versus, 2450u32),
        ] {
            let mut log = session::SessionLog::new(kind);
            for offset in 0..25u32 {
                log.rows.push(session::SessionFrame {
                    frame: expected_first + offset,
                    focused: true,
                    focus_changed: false,
                    frame_ms: 1.5,
                    p1: session::unpack_inputs(if offset < 5 { 1 } else { 0 }),
                    p2: session::unpack_inputs(if (10..15).contains(&offset) {
                        1 << 4
                    } else {
                        0
                    }),
                });
            }
            let expected_last = expected_first + 25;
            let json = session::session_to_scenario_json(&log, "sf-test-session", "test");
            assert_eq!(json["duration_frames"], expected_last + 1);
            assert_eq!(
                json["observations"]["window"],
                serde_json::json!([expected_first, expected_last])
            );
            let checkpoints: Vec<u32> = json["observations"]["checkpoints"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect();
            let expected_cp: Vec<u32> = (expected_first..=expected_last).collect();
            assert_eq!(checkpoints, expected_cp);
        }
    }

    /// Ticket #50 (F10) Acceptance 1, 2, 3, 4:
    /// - A scripted synthetic session of at least 1,000 frames (`1,011` frames,
    ///   `2389..=3400`) with every one of the 20 bound keys going through
    ///   [`Bindings::inputs`] starts at `sf-cpu-01`'s `2389`.
    /// - Converts to a scenario whose two MAME runs (`session-synth-01-run-a`
    ///   and `session-synth-01-run-b`) are identical, with checkpoints every
    ///   frame from `2389` to `3400`.
    /// - The native replay matches to the session's end (`3400`), with the
    ///   full comparison on every frame (`screen_hash`, `latch`,
    ///   `latch_changed`, checkpoint `pixels`, `video`, `palette`, and work
    ///   RAM).
    /// - Reports the mean and maximum frame times.
    /// - C-NEG: moving one input by 1 frame (`p1_right` from `2400` to `2401`)
    ///   is caught at frame `2401`.
    /// - Also checks that an untranslated branch reached during a session
    ///   replay is reported with its frame and branch address.
    #[test]
    fn synthetic_session_verifies_against_mame_and_catches_one_frame_shifted_input() {
        const SYNTH_A: &str = "../../tools/reference/local/captures/session-synth-01-run-a";
        const SYNTH_B: &str = "../../tools/reference/local/captures/session-synth-01-run-b";
        let required = [ROMS, CPU_FIXTURE, SYNTH_A, SYNTH_B];
        if !required.iter().all(|p| resolve_local(p).exists()) {
            eprintln!("skipping: local reference captures/ROMs not present");
            return;
        }
        let roms = resolve_local(ROMS);
        let manifest = resolve_local(MANIFEST);
        let cpu_fixture = resolve_local(CPU_FIXTURE);
        let run_a = resolve_local(SYNTH_A);
        let run_b = resolve_local(SYNTH_B);
        let assets = load_assets(&roms, &manifest).expect("local ROM set");

        let scratch = sf_engine::run::ScratchDir::new("sf-dev-f10-test");
        std::fs::create_dir_all(&*scratch).unwrap();
        let session_path = scratch.join("session_synth_01.tsv");
        let scenario_path = scratch.join("sf_session_synth_01.json");

        // 1. Record the synthetic session (2389..=3400, 1,011 frames) and
        //    confirm every one of the 20 bound keys went through Bindings::inputs.
        let log = session::record_synthetic_session(&cpu_fixture, &assets, &session_path)
            .expect("record synthetic session");
        let stats = log.stats();
        assert!(
            stats.frames >= 1000,
            "expected >= 1000 frames, got {}",
            stats.frames
        );
        assert_eq!(
            (stats.fixture_frame, stats.last_frame, stats.frames),
            (2389, 3400, 1011)
        );
        assert_eq!(
            stats.focus_changes, 2,
            "focus lost at 3050 and regained at 3060"
        );
        assert!(stats.mean_frame_ms > 0.0 && stats.max_frame_ms >= stats.mean_frame_ms);
        println!(
            "synthetic session timing: {} frames, {} focus changes, mean {:.3} ms, max {:.3} ms",
            stats.frames, stats.focus_changes, stats.mean_frame_ms, stats.max_frame_ms
        );

        let p1_union = log
            .rows
            .iter()
            .fold(0u16, |acc, r| acc | session::pack_inputs(r.p1));
        let p2_union = log
            .rows
            .iter()
            .fold(0u16, |acc, r| acc | session::pack_inputs(r.p2));
        assert_eq!(
            p1_union, 0x3ff,
            "all 10 P1 keys must be exercised through Bindings::inputs"
        );
        assert_eq!(
            p2_union, 0x3ff,
            "all 10 P2 keys must be exercised through Bindings::inputs"
        );

        // 2. Convert to scenario JSON and check window [2389, 3400] and
        //    checkpoints 2389..=3400, plus all 20 press names present.
        session::convert_session_file(&session_path, &scenario_path, Some("sf-session-synth-01"))
            .expect("convert session to scenario");
        let scenario_val: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&scenario_path).unwrap()).unwrap();
        assert_eq!(
            scenario_val["observations"]["window"],
            serde_json::json!([2389, 3400])
        );
        assert_eq!(
            scenario_val["observations"]["checkpoints"]
                .as_array()
                .unwrap()
                .len(),
            1012
        );
        for name in session::P1_PRESS_NAMES
            .into_iter()
            .chain(session::P2_PRESS_NAMES)
        {
            assert!(
                scenario_val["inputs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["press"] == name),
                "scenario must contain converted press '{name}'"
            );
        }

        // 3. Verify against the two MAME runs (Acceptance 1, 2, 3).
        let outcome = session::verify_session(
            &session_path,
            &scenario_path,
            &run_a,
            &run_b,
            &cpu_fixture,
            &assets,
        )
        .expect("verify session");
        let session::VerifyOutcome::Match {
            stats: vstats,
            reference_files_compared,
            frames_compared,
            checkpoints_compared,
        } = outcome
        else {
            panic!("synthetic session must match MAME reference, got {outcome:?}");
        };
        assert_eq!((frames_compared, checkpoints_compared), (1011, 1011));
        assert_eq!(reference_files_compared, 5063);
        assert!(vstats.mean_frame_ms > 0.0 && vstats.max_frame_ms >= vstats.mean_frame_ms);

        // 4. C-NEG: move one input (`p1_right` at frame 2400) by 1 frame to 2401.
        let mut shifted_log = log.clone();
        // Frame 2400 is index 11 (2389 + 11 = 2400); frame 2440 is index 51.
        assert_eq!(shifted_log.rows[11].frame, 2400);
        assert!(shifted_log.rows[11].p1.directions[0]);
        shifted_log.rows[11].p1.directions[0] = false;
        shifted_log.rows[51].p1.directions[0] = true;
        let shifted_session = scratch.join("session_shifted.tsv");
        let shifted_scenario = scratch.join("sf_session_shifted.json");
        shifted_log.write(&shifted_session).unwrap();
        session::convert_session_file(
            &shifted_session,
            &shifted_scenario,
            Some("sf-session-synth-01"),
        )
        .unwrap();
        let neg = session::verify_session(
            &shifted_session,
            &shifted_scenario,
            &run_a,
            &run_b,
            &cpu_fixture,
            &assets,
        )
        .expect("verify shifted session");
        let session::VerifyOutcome::Mismatch { frame, reason, .. } = neg else {
            panic!("1-frame shifted input must fail verification, got {neg:?}");
        };
        println!("C-NEG: 1-frame shifted p1_right caught at frame {frame} ({reason})");
        assert_eq!(
            frame, 2401,
            "1-frame shift at input frame 2400 first differs at end-of-frame 2401"
        );

        // 5. A session through sf-doubleko-01's former 0x3f2d0 stop at 5662
        //    now continues through the translated versus game-over screen.
        let doubleko_a = resolve_local("../../tools/reference/local/captures/doubleko-01-run-a");
        let doubleko_b = resolve_local("../../tools/reference/local/captures/doubleko-01-run-b");
        let doubleko_scen = resolve_local("../../tools/reference/scenarios/sf_doubleko_01.json");
        let versus_fixture = resolve_local(VERSUS_FIXTURE);
        if doubleko_a.is_dir()
            && doubleko_b.is_dir()
            && doubleko_scen.is_file()
            && versus_fixture.is_dir()
        {
            let mut dko_log = session::SessionLog::new(session::FixtureKind::Versus);
            for f in 2450..5665u32 {
                dko_log.rows.push(session::SessionFrame {
                    frame: f,
                    focused: true,
                    focus_changed: false,
                    frame_ms: 1.0,
                    p1: Inputs::default(),
                    p2: Inputs::default(),
                });
            }
            let dko_session = scratch.join("session_dko.tsv");
            dko_log.write(&dko_session).unwrap();
            let matched = session::verify_session(
                &dko_session,
                &doubleko_scen,
                &doubleko_a,
                &doubleko_b,
                &versus_fixture,
                &assets,
            )
            .expect("verify doubleko session");
            assert_eq!(
                matched,
                session::VerifyOutcome::Match {
                    stats: dko_log.stats(),
                    reference_files_compared: 16758,
                    frames_compared: 3215,
                    checkpoints_compared: 3215,
                }
            );
        }
    }

    /// Ticket #71 (F31): the human keyboard sessions recorded from both
    /// fixtures (`human-cpu-01.tsv` over 2389..=3979 and `human-versus-01.tsv`
    /// over 2450..=3785) replay natively without reaching an untranslated
    /// branch and match their two MAME reference captures on every frame and
    /// full checkpoint.
    #[test]
    fn human_recorded_cpu_and_versus_sessions_verify_against_mame() {
        let roms = resolve_local(ROMS);
        let manifest = resolve_local(MANIFEST);
        let cpu_fixture = resolve_local(CPU_FIXTURE);
        let versus_fixture = resolve_local(VERSUS_FIXTURE);
        let cpu_tsv = resolve_local("../../tools/reference/local/sessions/human-cpu-01.tsv");
        let cpu_scen = resolve_local("../../tools/reference/local/sessions/sf_human_cpu_01.json");
        let cpu_a = resolve_local("../../tools/reference/local/captures/human-cpu-01-run-a");
        let cpu_b = resolve_local("../../tools/reference/local/captures/human-cpu-01-run-b");
        let vs_tsv = resolve_local("../../tools/reference/local/sessions/human-versus-01.tsv");
        let vs_scen = resolve_local("../../tools/reference/local/sessions/sf_human_versus_01.json");
        let vs_a = resolve_local("../../tools/reference/local/captures/human-versus-01-run-a");
        let vs_b = resolve_local("../../tools/reference/local/captures/human-versus-01-run-b");
        if !roms.is_dir()
            || !cpu_fixture.is_dir()
            || !versus_fixture.is_dir()
            || !cpu_tsv.is_file()
            || !cpu_scen.is_file()
            || !cpu_a.is_dir()
            || !cpu_b.is_dir()
            || !vs_tsv.is_file()
            || !vs_scen.is_file()
            || !vs_a.is_dir()
            || !vs_b.is_dir()
        {
            eprintln!("skipping: local human session files/captures not present");
            return;
        }
        let assets = load_assets(&roms, &manifest).expect("ROM set");

        let cpu_outcome =
            session::verify_session(&cpu_tsv, &cpu_scen, &cpu_a, &cpu_b, &cpu_fixture, &assets)
                .expect("verify human-cpu-01");
        let session::VerifyOutcome::Match {
            stats: cpu_stats,
            reference_files_compared: cpu_files,
            frames_compared: cpu_frames,
            checkpoints_compared: cpu_ckpts,
        } = cpu_outcome
        else {
            panic!("expected Match on human-cpu-01, got {cpu_outcome:?}");
        };
        assert_eq!(
            (
                cpu_stats.fixture,
                cpu_stats.fixture_frame,
                cpu_stats.last_frame,
                cpu_stats.frames
            ),
            (session::FixtureKind::Cpu, 2389, 3979, 1590)
        );
        assert_eq!((cpu_files, cpu_frames, cpu_ckpts), (7958, 1590, 1590));

        let vs_outcome =
            session::verify_session(&vs_tsv, &vs_scen, &vs_a, &vs_b, &versus_fixture, &assets)
                .expect("verify human-versus-01");
        let session::VerifyOutcome::Match {
            stats: vs_stats,
            reference_files_compared: vs_files,
            frames_compared: vs_frames,
            checkpoints_compared: vs_ckpts,
        } = vs_outcome
        else {
            panic!("expected Match on human-versus-01, got {vs_outcome:?}");
        };
        assert_eq!(
            (
                vs_stats.fixture,
                vs_stats.fixture_frame,
                vs_stats.last_frame,
                vs_stats.frames
            ),
            (session::FixtureKind::Versus, 2450, 3785, 1335)
        );
        assert_eq!((vs_files, vs_frames, vs_ckpts), (6683, 1335, 1335));
    }

    /// Ticket #126 (F54): `sound_test_listening_kit()` writes the native and
    /// MAME WAVs for `sf-sound-test-01..04` and records per-command `onset`,
    /// `RMS`, and `correlation` across all 236 planned sound-test commands in
    /// `tools/reference/local/renders/sound-test-measures.tsv`.
    #[test]
    fn sound_test_listening_kit_renders_wavs_and_per_command_measures() {
        let captures = [
            "../../tools/reference/local/captures/sound-test-01-audio-a/audio.wav",
            "../../tools/reference/local/captures/sound-test-02-audio-a/audio.wav",
            "../../tools/reference/local/captures/sound-test-03-audio-a/audio.wav",
            "../../tools/reference/local/captures/sound-test-04-audio-a/audio.wav",
        ];
        if !resolve_local(ROMS).is_dir() || !captures.iter().all(|p| resolve_local(p).is_file()) {
            eprintln!("skipping: local sound-test captures/ROMs not present");
            return;
        }
        let renders = resolve_local("../../tools/reference/local/renders");
        let tsv_path = renders.join("sound-test-measures.tsv");
        if !tsv_path.is_file() {
            assert_eq!(sound_test_listening_kit(), 0);
        }
        for name in [
            "sound-test-01",
            "sound-test-02",
            "sound-test-03",
            "sound-test-04",
        ] {
            assert!(renders.join(format!("{name}-native-mix.wav")).is_file());
            assert!(renders.join(format!("{name}-mame-reference.wav")).is_file());
        }
        let text = std::fs::read_to_string(&tsv_path).expect("read sound-test-measures.tsv");
        let rows: Vec<&str> = text.lines().skip(1).collect();
        assert_eq!(
            rows.len(),
            236,
            "108 + 42 + 14 + 72 = 236 sound-test commands"
        );
        for row in rows {
            let cols: Vec<&str> = row.split('\t').collect();
            assert_eq!(cols.len(), 13, "row must have 13 columns: {row}");
            let n_on: f64 = cols[6].parse().expect("native_onset_ms");
            let r_on: f64 = cols[7].parse().expect("mame_onset_ms");
            let ratio: f64 = cols[10].parse().expect("rms_ratio");
            let corr: f64 = cols[11].parse().expect("corr");
            assert!(
                (n_on - r_on).abs() <= 5.001,
                "onset within one 5 ms window: {row}"
            );
            assert!(
                ratio.is_finite() && ratio > 0.8 && ratio < 1.25,
                "finite RMS ratio: {row}"
            );
            assert!(corr >= 0.75, "correlation >= 0.75 on every command: {row}");
        }
    }
}
