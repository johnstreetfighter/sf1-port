//! The game (spec #150, ADR-0005): the faithful attract, the front end's
//! menus, and the 1987 machine behind them, in one window with the
//! keyboard, the pads and sound. Everything the player sees comes from
//! `sf_engine::front::GameSession`; this binary only finds the ROM set,
//! loads and saves the settings files, keeps the log, opens the window,
//! the audio device and the pads, and steps the game at 60 Hz. The window
//! is the GPU window (ticket #205, ADR-0010): it presents on vsync and
//! steps one game frame every *n* refreshes on a multiple of 60 Hz, and
//! draws every screen through the player's look (ticket #207), loaded from
//! the `shaders` folder beside the executable.
//!
//!   cargo run -p sf-game --release [-- --roms <folder>] [--stage <name>]
//!   cargo run -p sf-game --release -- --console  # live diagnostic log
//!   cargo run -p sf-game --release -- --version
//!   cargo run -p sf-game --release -- smoke [--roms <folder>]   # the host smoke check
//!
//! The ROM set (`roms\sf`) is looked up beside the executable, then in
//! `%APPDATA%\sf1-port\roms\sf`, then in the settings file's folder;
//! `--roms` names another folder or archive first. `--stage`
//! names a stage of the background bank that Local and Training fights
//! under an overhaul preset are fought on, in place of the stage select
//! (a development aid: `--stage test-stage`). With none, the
//! ROM-missing screen offers `RETRY`, browsing on Windows, and `EXIT`. The three settings files
//! and `sf-game.log` live in `%APPDATA%\sf1-port`. Esc is back on every
//! screen; the Exit item and the window's close button quit.

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::time::Duration;

use sf_engine::config::{EvilRyuPlaytest, MatchConfig, RulesetPreset};
use sf_engine::front::rom_missing::RomMissing;
use sf_engine::front::settings::{Aspect, DisplayInfo, Settings, FRAME_DELAY_CAP_MS, LOOK_OFF, MAX_SCALE};
use sf_engine::world_tour::native::WorldTourProfile;
use sf_engine::front::{GameSession, MenuItem, Transition, VideoState};
use sf_host::audio::{Audio, Gains, SoundStart};
use sf_host::bindings::BindingsTable;
use sf_host::files::{self, Files};
use sf_host::gpu_window::{self, Driver, VideoMode, WindowState};
use sf_host::input::frame_inputs;
use sf_host::jukebox::{Action, Deck};
use sf_host::keys::Keyboard;
use sf_host::log::Log;
use sf_host::net::HostNetDriver;
use sf_host::look::{first_line, LookEvent, LookRequest};
use sf_host::pacing::Pacing;
use sf_host::pads::PadReader;
use sf_host::picture::picture_rect;
use sf_host::roms::{self, Candidate, RomSet};
use sf_host::shaders::{self, Catalogue};
use sf_host::themes::{self, Themes};
use sf_video::render::{Frame, HEIGHT, WIDTH};

mod evil_ryu_art;
mod evil_ryu_vs;
mod evil_ryu_vs_name;

fn evil_ryu_title(playtest: EvilRyuPlaytest) -> &'static str {
    match playtest {
        EvilRyuPlaytest::UltraSf2 => "Ultra SFII Playtest - Evil Ryu vs. Sagat",
        EvilRyuPlaytest::Alpha2 => "Alpha 2 Playtest - Evil Ryu vs. Sagat",
        EvilRyuPlaytest::Alpha3A => "Alpha 3 A-ISM Playtest - Evil Ryu vs. Sagat",
        EvilRyuPlaytest::Alpha3V => "Alpha 3 V-ISM Playtest - Evil Ryu vs. Sagat",
        EvilRyuPlaytest::Alpha3X => "Alpha 3 X-ISM Playtest - Evil Ryu vs. Sagat",
        EvilRyuPlaytest::Ae2012 => "Arcade Edition 2012 Playtest - Evil Ryu vs. Sagat (Ultra I)",
        EvilRyuPlaytest::Ae2012Ultra2 => "Arcade Edition 2012 Playtest - Evil Ryu vs. Sagat (Ultra II)",
        EvilRyuPlaytest::Usf4 => "Ultra SFIV Playtest - Evil Ryu vs. Sagat (Ultra Combo Double)",
        EvilRyuPlaytest::Kage => "SFV Champion Edition Playtest - Kage vs. Sagat (V-Skill I / V-Trigger I)",
        EvilRyuPlaytest::KageII => "SFV Champion Edition Playtest - Kage vs. Sagat (V-Skill II / V-Trigger II)",
        EvilRyuPlaytest::Sf6 => "SF6 What-If Playtest - Evil Ryu vs. Sagat",
    }
}

/// The `--roms` directory or archive, when given.
fn roms_arg(args: &[String]) -> Option<PathBuf> {
    let i = args.iter().position(|a| a == "--roms")?;
    args.get(i + 1).map(PathBuf::from)
}

/// The `--stage` name, when given.
fn stage_arg(args: &[String]) -> Option<&str> {
    let i = args.iter().position(|a| a == "--stage")?;
    args.get(i + 1).map(String::as_str)
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf))
}

fn gains(settings: &Settings) -> Gains {
    Gains::from_volumes(settings.master, settings.fm, settings.samples, settings.announcer)
}

fn video(settings: &Settings) -> VideoMode {
    VideoMode {
        scale: u32::from(settings.scale),
        fullscreen: settings.fullscreen,
        aspect: settings.aspect,
        vsync: settings.vsync,
        frame_delay_ms: u32::from(settings.frame_delay()),
    }
}

/// The look `id` names in `catalogue`: `None` for `OFF` (or an id the
/// catalogue does not hold, which the settings never keep).
fn look_request(catalogue: &Catalogue, id: &str) -> Option<LookRequest> {
    let entry = catalogue.find(id)?;
    entry.preset.as_ref().map(|preset| LookRequest {
        id: entry.id.clone(),
        // A saved look on a built-in base is pointed at the shaders folder
        // as it is now; a failure leaves the file, whose load says why.
        preset: shaders::loadable(preset, &shaders::shaders_dir()).unwrap_or_else(|_| preset.clone()),
    })
}

/// The looks: the built-ins beside the executable, then the drop-ins in
/// the app-data folder, with a log line naming the drop-ins when they
/// differ from `before`'s.
fn read_looks(app_dir: &Path, before: Option<&Catalogue>, log: &mut Log) -> Catalogue {
    let dir = shaders::drop_in_dir(app_dir);
    let catalogue = Catalogue::discover(&shaders::shaders_dir(), &dir);
    let drop_ins = catalogue.drop_ins();
    if before.map(Catalogue::drop_ins).as_ref() != Some(&drop_ins) {
        let line = if drop_ins.is_empty() {
            format!("no drop-in looks in {}", dir.display())
        } else {
            format!("drop-in looks in {}: {}", dir.display(), drop_ins.join(", "))
        };
        log.line("looks", &line);
    }
    catalogue
}

#[cfg(windows)]
fn open_diagnostic_console(create: bool) {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn AllocConsole() -> i32;
    }

    // SAFETY: These Win32 calls take no pointers. u32::MAX denotes the
    // launching process's console. Windows preserves redirected stdio handles.
    unsafe {
        if AttachConsole(u32::MAX) == 0 && create {
            AllocConsole();
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str);
    #[cfg(windows)]
    {
        let live_log = args.iter().any(|arg| arg == "--console");
        if live_log || matches!(command, Some("--version" | "-V" | "smoke")) {
            open_diagnostic_console(live_log);
        }
    }
    if matches!(command, Some("--version" | "-V")) {
        println!("{}", sf_engine::version::full_banner());
        return;
    }
    if command == Some("smoke") {
        std::process::exit(smoke(&args));
    }
    let normal_app_dir = files::app_dir();
    let evil_ryu_playtest = args.iter().find_map(|arg| match arg.as_str() {
        "--evil-ryu-usf2" => Some(EvilRyuPlaytest::UltraSf2),
        "--evil-ryu-alpha2" => Some(EvilRyuPlaytest::Alpha2),
        "--evil-ryu-alpha3-a" => Some(EvilRyuPlaytest::Alpha3A),
        "--evil-ryu-alpha3-v" => Some(EvilRyuPlaytest::Alpha3V),
        "--evil-ryu-alpha3-x" => Some(EvilRyuPlaytest::Alpha3X),
        "--evil-ryu-ae2012" => Some(EvilRyuPlaytest::Ae2012),
        "--evil-ryu-ae2012-ultra2" => Some(EvilRyuPlaytest::Ae2012Ultra2),
        "--evil-ryu-usf4" => Some(EvilRyuPlaytest::Usf4),
        "--kage-sfv" => Some(EvilRyuPlaytest::Kage),
        "--kage-sfv-ii" => Some(EvilRyuPlaytest::KageII),
        "--evil-ryu-sf6" => Some(EvilRyuPlaytest::Sf6),
        _ => None,
    });
    let app_dir = match evil_ryu_playtest {
        Some(EvilRyuPlaytest::UltraSf2) => normal_app_dir.join("evil-ryu-spike"),
        Some(EvilRyuPlaytest::Alpha2) => normal_app_dir.join("evil-ryu-alpha2"),
        Some(EvilRyuPlaytest::Alpha3A) => normal_app_dir.join("evil-ryu-alpha3-a"),
        Some(EvilRyuPlaytest::Alpha3V) => normal_app_dir.join("evil-ryu-alpha3-v"),
        Some(EvilRyuPlaytest::Alpha3X) => normal_app_dir.join("evil-ryu-alpha3-x"),
        Some(EvilRyuPlaytest::Ae2012) => normal_app_dir.join("evil-ryu-ae2012"),
        Some(EvilRyuPlaytest::Ae2012Ultra2) => normal_app_dir.join("evil-ryu-ae2012-ultra2"),
        Some(EvilRyuPlaytest::Usf4) => normal_app_dir.join("evil-ryu-usf4"),
        Some(EvilRyuPlaytest::Kage) => normal_app_dir.join("kage-sfv"),
        Some(EvilRyuPlaytest::KageII) => normal_app_dir.join("kage-sfv-ii"),
        Some(EvilRyuPlaytest::Sf6) => normal_app_dir.join("evil-ryu-sf6"),
        None => normal_app_dir.clone(),
    };
    let mut log = Log::create(&files::log_path(&app_dir));
    log.line("version", &sf_engine::version::full_banner());
    let Files { mut settings, config, bindings } = files::load(&app_dir, &mut log);
    let catalogue = read_looks(&app_dir, None, &mut log);
    // Saved looks' resolved presets a stopped game left in the temp folder.
    let swept = shaders::sweep_temp(&std::env::temp_dir(), std::time::Duration::from_secs(3600));
    if swept > 0 {
        log.line("looks", &format!("removed {swept} old temporary look folders"));
    }
    let (look, fallback) = catalogue.known_or_default(&settings.crt_look);
    if let Some(line) = fallback {
        log.line("settings", &line);
    }
    settings.crt_look = look;
    log.line("settings", &files::describe(&settings, &config));
    let mut pads = PadReader::open();
    for note in pads.take_notes() {
        log.line("pads", &note);
    }
    let candidates = roms::candidates(
        roms_arg(&args).as_deref(),
        exe_dir().as_deref(),
        &normal_app_dir,
        settings.rom_folder.as_deref(),
    );
    let mut game = Game {
        args,
        evil_ryu_playtest,
        app_dir,
        log,
        bindings,
        pads,
        candidates,
        selected_rom: None,
        video: video(&settings),
        look: look_request(&catalogue, &settings.crt_look),
        reload_look: false,
        look_failed: None,
        catalogue,
        settings,
        config: Some(config),
        phase: Phase::Starting,
        frame: Frame { argb: vec![0xff00_0000; WIDTH * HEIGHT] },
        panel: None,
    };
    match roms::find(&game.candidates) {
        Ok(set) => game.start(set),
        Err(looked) => game.missing(looked),
    }
    let title = if let Some(playtest) = evil_ryu_playtest {
        evil_ryu_title(playtest).to_owned()
    } else {
        format!("{} (v{})", sf_engine::version::EDITION_NAME, sf_engine::version::VERSION)
    };
    if let Err(e) = gpu_window::run(&title, game.video, &mut game, None) {
        game.log.line("window", &e);
        eprintln!("sf-game: {e}");
    }
    game.finish();
}

/// The game behind the window: the ROM-missing screen until a set is
/// found, then the session.
struct Game {
    args: Vec<String>,
    evil_ryu_playtest: Option<EvilRyuPlaytest>,
    app_dir: PathBuf,
    log: Log,
    bindings: BindingsTable,
    pads: PadReader,
    candidates: Vec<Candidate>,
    /// The latest manually chosen source, including a failed choice to retry.
    selected_rom: Option<PathBuf>,
    /// The settings as loaded, until the session holds them.
    settings: Settings,
    config: Option<MatchConfig>,
    video: VideoMode,
    /// The looks, and the one to draw through (`None` is `OFF`).
    catalogue: Catalogue,
    look: Option<LookRequest>,
    /// The look on screen was replaced by a save under its own name: the
    /// window loads it afresh.
    reload_look: bool,
    /// A look that failed before the session started: its id and error.
    look_failed: Option<(String, String)>,
    phase: Phase,
    frame: Frame,
    /// The tuning screen's panel, drawn sharp over the look.
    panel: Option<sf_engine::front::Panel>,
}

enum Phase {
    Starting,
    /// The ROM-missing screen, until `RETRY` finds a set or `EXIT` quits.
    Missing(RomMissing),
    Playing(Box<Play>),
}

struct Play {
    session: GameSession,
    net: HostNetDriver,
    sound: Option<Audio>,
    /// What the settings file holds; the menus' positions change without
    /// a save and are written when the game ends.
    saved: Settings,
    saved_world_tour: WorldTourProfile,
    /// The screen the last frame showed.
    screen: &'static str,
    /// The extended themes (ticket #199), looked for once the preset is an
    /// overhaul one: `None` until then, `Some(None)` when there are none.
    themes: Option<Option<Themes>>,
    /// Where the themes come from: the music CPU's ROM and the folders the
    /// ROMs were looked for in (the music pack is beside one of them).
    audiocpu: Vec<u8>,
    rom_folders: Vec<PathBuf>,
    /// The music CPU runs with the tune table now.
    tune_table: bool,
    /// The Jukebox's tracks, from the `jukebox` folder beside the ROM set
    /// (ticket #200).
    jukebox: Deck,
}

impl Game {
    fn report(&mut self, looked: &[sf_engine::front::rom_missing::Looked]) {
        for l in looked {
            self.log.line("roms", &format!("{}: {}", l.folder, l.problem));
        }
    }

    fn missing(&mut self, looked: Vec<sf_engine::front::rom_missing::Looked>) {
        self.report(&looked);
        let (set, description) = roms::set_name();
        let screen = RomMissing::open(&set.to_uppercase(), &description.to_uppercase(), looked)
            .with_browse(cfg!(windows));
        self.frame = screen.render();
        self.phase = Phase::Missing(screen);
    }

    fn start_after_search(&mut self, set: RomSet) {
        if self.selected_rom.as_ref() == Some(&set.folder) {
            if let Some(path) = set.folder.to_str() {
                self.settings.rom_folder = Some(path.to_owned());
                if let Err(error) = self.settings.save(&files::settings_path(&self.app_dir)) {
                    self.log.line("settings", &error);
                }
            }
        }
        self.start(set);
    }

    /// Load the art, stages and voice beside `set`, build the session and
    /// open the sound.
    fn start(&mut self, set: RomSet) {
        let log = &mut self.log;
        log.line(
            "roms",
            &format!("{}: all {} files match the manifest", set.folder.display(), set.files),
        );
        let RomSet { folder, mut assets, audio2, audiocpu, .. } = set;
        // Local packs beside the ROMs take precedence over bundled assets.
        let asset_roots: Vec<_> = [exe_dir(), std::env::current_dir().ok()]
            .into_iter()
            .flatten()
            .map(|root| root.join("assets"))
            .collect();
        let mut rom_folders = vec![folder.as_path()];
        rom_folders.extend(self.candidates.iter().filter_map(|c| c.path.as_deref()));
        let art = sf_host::art::load_with_fallbacks(&assets, &rom_folders, &asset_roots, log);
        // The background bank, from the committed stage records and the ROM,
        // with the two-player VS screens built from the ROM.
        let mut stages = sf_host::stages::load(&assets, log);
        if let Some(playtest) = self.evil_ryu_playtest {
            let bank = stages.get_or_insert_with(Default::default);
            if let Err(error) = evil_ryu_vs::install(&assets, bank, playtest) {
                log.line("playtest", &format!("could not load the {} VS screen: {error}", playtest.display_name()));
            }
        }
        // The announcer's voice pack, looked for as the art pack is.
        let voice = sf_host::voice::load_with_fallbacks(&rom_folders, &asset_roots, log);

        let mut config = self.config.take().unwrap_or_default();
        if let Some(playtest) = self.evil_ryu_playtest {
            config = MatchConfig::definitive();
            config.evil_ryu_playtest = Some(playtest);
            evil_ryu_art::apply(&mut assets, playtest);
        }
        let saved_world_tour = files::load_world_tour(&self.app_dir, log);
        let mut session = GameSession::new(assets, config)
            .with_settings(self.settings.clone())
            .with_world_tour_profile(saved_world_tour.clone())
            .with_looks(self.catalogue.names())
            .with_binds(self.bindings.binds(), BindingsTable::default().binds());
        if let Some((id, error)) = self.look_failed.take() {
            session.look_failed(&id, &error);
        }
        if let Some(art) = &art {
            session = session.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
        } else {
            session = session.with_player_tables(&[]);
        }
        if let Some(bank) = stages {
            session = session.with_background_bank(bank);
        }
        if let Some(name) = stage_arg(&self.args) {
            match session.background_bank().and_then(|b| b.stage_index(name)) {
                Some(index) => {
                    session.set_stage(Some(sf_engine::roster::StagePick::Bank(index)));
                    log.line("stages", &format!("Local and Training fights are on {name}"));
                }
                None => log.line("stages", &format!("--stage {name}: the background bank holds no such stage")),
            }
        }
        if let Some(playtest) = self.evil_ryu_playtest {
            session.start_evil_ryu_playtest();
            log.line("playtest", evil_ryu_title(playtest));
        }
        // Tracks set for a stage or fighter the game does not have never play.
        let stage_names: Vec<&str> = session.background_bank().map(|b| b.stage_names().collect()).unwrap_or_default();
        let fighters: Vec<&str> = sf_engine::roster::TournamentFighter::ALL
            .iter()
            .map(|f| f.name())
            .chain(sf_engine::bonus::Challenger::ALL.iter().map(|c| c.name()))
            .collect();
        for line in self.settings.jukebox.unmatched(stage_names, fighters) {
            log.line("jukebox", &line);
        }
        let jukebox = Deck::new(sf_host::jukebox::folders(&rom_folders));
        let themes_from = (audiocpu.clone(), rom_folders.iter().map(|f| f.to_path_buf()).collect());
        let sound = Audio::open(&SoundStart {
            audio2,
            audiocpu,
            frame: 0,
            preroll: sf_audio::LatchTimeline::default(),
        });
        match &sound {
            Some(s) => {
                s.set_gains(gains(&self.settings));
                s.set_voice(voice);
            }
            None => log.line("settings", "audio: no usable output device; running silent"),
        }
        let saved = self.settings.clone();
        let (audiocpu, rom_folders) = themes_from;
        let play = Play { session, net: HostNetDriver::new(), sound, saved, saved_world_tour, screen: "", themes: None, audiocpu, rom_folders, tune_table: false, jukebox };
        self.phase = Phase::Playing(Box::new(play));
    }

    /// Show the frames as `video` says: the window's mode, and the look
    /// (loaded by the window when it changes).
    fn apply(&mut self, video: &VideoState) {
        self.video = VideoMode {
            scale: u32::from(video.scale),
            fullscreen: video.fullscreen,
            aspect: video.aspect,
            vsync: video.vsync,
            frame_delay_ms: u32::from(video.frame_delay_ms),
        };
        if self.look.as_ref().map(|l| l.id.as_str()).unwrap_or(LOOK_OFF) != video.look {
            self.look = look_request(&self.catalogue, &video.look);
        }
    }

    /// Save the settings if the session changed them without a save.
    fn finish(&mut self) {
        if let Phase::Playing(play) = &self.phase {
            if play.session.settings() != &play.saved {
                if let Err(e) = play.session.settings().save(&files::settings_path(&self.app_dir)) {
                    self.log.line("settings", &e);
                }
            }
            if play.session.world_tour_profile() != &play.saved_world_tour {
                if let Err(error) = play.session.world_tour_profile().save(&files::world_tour_path(&self.app_dir)) {
                    self.log.line("world tour", &error);
                }
            }
        }
    }
}

impl Driver for Game {
    fn step(&mut self, keyboard: &mut Keyboard, window: &WindowState) -> bool {
        let pad_frame = self.pads.poll();
        let typed = keyboard.take_typed();
        for note in self.pads.take_notes() {
            self.log.line("pads", &note);
        }
        match &mut self.phase {
            Phase::Starting => {}
            Phase::Missing(screen) => {
                let inputs = frame_inputs(keyboard, &self.bindings, self.settings.devices, &pad_frame);
                match screen.step(inputs) {
                    Some(Transition::Choose(MenuItem::Retry)) => {
                        let mut candidates = self.candidates.clone();
                        if let Some(path) = &self.selected_rom {
                            candidates.insert(0, Candidate {
                                label: path.display().to_string(),
                                path: Some(path.clone()),
                            });
                        }
                        match roms::find(&candidates) {
                            Ok(set) => {
                                self.start_after_search(set);
                                return true;
                            }
                            Err(mut looked) => {
                                for source in &looked {
                                    self.log.line("roms", &format!("{}: {}", source.folder, source.problem));
                                }
                                let chosen = self.selected_rom.as_ref().map(|_| looked.remove(0));
                                screen.update(looked);
                                if let Some(chosen) = chosen {
                                    screen.failed_choice(chosen);
                                }
                            }
                        }
                    }
                    Some(Transition::Choose(item @ (MenuItem::BrowseRomFolder | MenuItem::BrowseRomArchive))) => {
                        let kind = if item == MenuItem::BrowseRomFolder {
                            roms::BrowseKind::Folder
                        } else {
                            roms::BrowseKind::Archive
                        };
                        match roms::browse(window, kind) {
                            Ok(Some(path)) => {
                                let result = roms::load(&path);
                                self.selected_rom = Some(path.clone());
                                match result {
                                    Ok(set) => {
                                        self.start_after_search(set);
                                        return true;
                                    }
                                    Err(problem) => {
                                        let folder = path.display().to_string();
                                        self.log.line("roms", &format!("{folder}: {problem}"));
                                        screen.failed_choice(sf_engine::front::rom_missing::Looked { folder, problem });
                                    }
                                }
                            }
                            Ok(None) => {}
                            Err(problem) => {
                                self.log.line("roms", &problem);
                                screen.failed_choice(sf_engine::front::rom_missing::Looked {
                                    folder: "ROM PICKER".to_owned(),
                                    problem,
                                });
                            }
                        }
                    }
                    Some(Transition::Choose(MenuItem::Exit)) => return false,
                    _ => {}
                }
                self.frame = screen.render();
            }
            Phase::Playing(play) => {
                let (session, log) = (&mut play.session, &mut self.log);
                // The Video tab's notice line and scale range follow the display.
                session.set_display(DisplayInfo {
                    refresh_mhz: window.refresh_hz.map(|hz| (hz * 1000.0).round() as u32),
                    max_scale: window.max_scale.min(u32::from(MAX_SCALE)) as u8,
                });
                // The extended themes and the music pack are looked for only
                // once the preset is an overhaul one, so Faithful 1987 reads
                // nothing from the music pack; a preset changed in Settings
                // is picked up before any fight can start.
                if play.themes.is_none() && session.match_config().preset != RulesetPreset::Faithful1987 {
                    let folders: Vec<&Path> = play.rom_folders.iter().map(PathBuf::as_path).collect();
                    let themes = themes::load(session.match_config().preset, &play.audiocpu, &folders, log);
                    themes::apply(themes.as_ref(), session, log);
                    play.themes = Some(themes);
                }
                let inputs = frame_inputs(keyboard, &self.bindings, session.settings().devices, &pad_frame);
                #[cfg(windows)]
                {
                    use sf_host::keys::RoomCodeShortcut::{Copy, Paste};
                    match keyboard.take_room_code_shortcut() {
                        Some(Copy) => {
                            if let Some(code) = session.room_code_for_copy() {
                                let copied = window.win32_hwnd()
                                    .is_some_and(|owner| sf_host::clipboard::write_text(owner, code));
                                session.room_code_copy_result(copied);
                            }
                        }
                        Some(Paste) if session.accepts_room_code_paste() => {
                            let code = window.win32_hwnd().and_then(sf_host::clipboard::read_text);
                            session.paste_room_code(code.as_deref());
                        }
                        _ => {}
                    }
                }
                session.type_keys(typed);
                play.net.set_lobby_url(session.settings().lobby_url.clone());
                session.set_net_time_ms(play.net.now_ms());
                let out = session.step(inputs);
                #[cfg(windows)]
                keyboard.set_room_code_shortcuts(
                    session.room_code_for_copy().is_some(),
                    session.accepts_room_code_paste(),
                );
                let commands = session.drain_net_commands();
                session.push_net_events(&play.net.poll(&commands));
                for note in play.net.drain_notes() {
                    log.line("netplay", &note);
                }
                for line in session.take_log() {
                    let (cat, body) = line.split_once(": ").unwrap_or(("untranslated", &line));
                    log.line(cat, body);
                }
                if let Some(s) = &play.sound {
                    // The table is the music CPU's for an overhaul fight only,
                    // handed over before the frame that may name a theme.
                    if out.tune_table != play.tune_table {
                        play.tune_table = out.tune_table;
                        let table = play.themes.iter().flatten().find(|_| out.tune_table);
                        s.set_tune_table(table.map(|t| t.table.clone()));
                    }
                    // The Jukebox's track replaces the theme the fight latches:
                    // the theme's command is left out while the track plays,
                    // and passed on when the file could not be played.
                    let action = out.track.as_ref().map_or(Action::None, |cue| play.jukebox.cue(cue, log));
                    let latch = play.jukebox.latch(&action, out.sound_latch);
                    s.push_frame(out.sound_frame, latch, out.latch_cycles);
                    match action {
                        Action::Start(track) => s.start_track(out.sound_frame, track),
                        Action::Stop => s.stop_track(out.sound_frame),
                        Action::None => {}
                    }
                    s.announce(out.sound_frame, &out.announcements);
                }
                if out.quit || (self.evil_ryu_playtest.is_some() && session.fight().is_none()) {
                    return false;
                }
                if session.take_world_tour_changed() {
                    let profile = session.world_tour_profile();
                    match profile.save(&files::world_tour_path(&self.app_dir)) {
                        Ok(()) => play.saved_world_tour = profile.clone(),
                        Err(error) => log.line("world tour", &error),
                    }
                }
                if out.bindings_changed {
                    match session.binds().map(|b| self.bindings.with_binds(b)) {
                        Some(Ok(table)) => {
                            self.bindings = table;
                            match self.bindings.save(&files::bindings_path(&self.app_dir)) {
                                Ok(()) => log.line("settings", "saved the bindings"),
                                Err(e) => log.line("settings", &e),
                            }
                        }
                        Some(Err(e)) => log.line("settings", &format!("the bindings were not changed: {e}")),
                        None => {}
                    }
                }
                if out.settings_changed {
                    let (config, settings) = (session.local_match_config(), session.settings());
                    for result in [
                        config.save(&files::match_config_path(&self.app_dir)),
                        settings.save(&files::settings_path(&self.app_dir)),
                    ] {
                        if let Err(e) = result {
                            log.line("settings", &e);
                        }
                    }
                    log.line("settings", &format!("saved: {}", files::describe(settings, config)));
                    play.saved = settings.clone();
                    if let Some(s) = &play.sound {
                        s.set_gains(gains(settings));
                    }
                }
                // SAVE AS on the tuning screen: write the look, list it and
                // make it the player's look.
                if let Some(request) = session.take_save_look() {
                    match shaders::save_look(&shaders::drop_in_dir(&self.app_dir), &request) {
                        Ok(id) => {
                            log.line("looks", &format!("saved {id} on {}: {} parameters set", request.look, request.overrides.len()));
                            self.catalogue = read_looks(&self.app_dir, Some(&self.catalogue), log);
                            session.set_looks(self.catalogue.names());
                            session.look_saved(Ok(id.clone()));
                            if self.look.as_ref().is_some_and(|l| l.id == id) {
                                self.look = look_request(&self.catalogue, &id);
                                self.reload_look = true;
                            }
                        }
                        Err(e) => {
                            log.line("looks", &format!("could not save \"{}\": {e}", request.name));
                            session.look_saved(Err("COULD NOT SAVE THE LOOK".to_owned()));
                        }
                    }
                }
                // The look list is re-read as Settings opens, so drop-ins
                // added or removed since show in CRT LOOK.
                if out.screen == "settings" && play.screen != "settings" {
                    self.catalogue = read_looks(&self.app_dir, Some(&self.catalogue), log);
                    session.set_looks(self.catalogue.names());
                }
                // The `jukebox` folder is listed as Settings or the Jukebox
                // screen opens, so files dropped in since show; the engine
                // never reads the folder.
                if matches!(out.screen, "settings" | "jukebox") && play.screen != out.screen {
                    session.set_track_formats(sf_engine::jukebox::TrackFormats::new(sf_host::track::supported_extensions()));
                    session.set_tracks(play.jukebox.tracks());
                }
                play.screen = out.screen;
                // The window switches in place, and the aspect and the
                // look change, as the rows change.
                self.frame = out.frame;
                self.panel = out.panel;
                self.apply(&out.video);
            }
        }
        true
    }

    fn frame(&self) -> &Frame {
        &self.frame
    }

    fn video(&self) -> VideoMode {
        self.video
    }

    fn note(&mut self, line: &str) {
        self.log.line("window", line);
    }

    fn look(&self) -> Option<&LookRequest> {
        self.look.as_ref()
    }

    fn take_look_reload(&mut self) -> bool {
        std::mem::take(&mut self.reload_look)
    }

    /// The tuning screen lists a loaded look's parameters.
    fn look_parameters(&mut self, id: &str, params: Vec<sf_engine::front::LookParam>) {
        if let Phase::Playing(play) = &mut self.phase {
            play.session.set_look_params(id, params);
        }
    }

    fn look_tuning(&self) -> Option<&sf_engine::front::LookTuning> {
        match &self.phase {
            Phase::Playing(play) => play.session.look_tuning(),
            _ => None,
        }
    }

    fn panel(&self) -> Option<&sf_engine::front::Panel> {
        self.panel.as_ref()
    }

    /// A look that failed falls back to `OFF`, with its first error on
    /// the notice line (the window logged the whole of it).
    fn look_event(&mut self, event: LookEvent) {
        if let LookEvent::Failed { id, error } = event {
            match &mut self.phase {
                Phase::Playing(play) => play.session.look_failed(&id, first_line(&error)),
                // Before the session: it hears of it as it starts. The
                // settings keep the look either way (never saved as OFF).
                _ => self.look_failed = Some((id.clone(), first_line(&error).to_owned())),
            }
            if self.look.as_ref().is_some_and(|l| l.id == id) {
                self.look = None;
            }
        }
    }
}

/// The host smoke check: the pads connected; the GPU window at 2x, every
/// built-in look loaded (its time or its error), switched in place to
/// borderless fullscreen and back, with its pacing over five seconds of
/// fullscreen sharp and five through each built-in look at true 4:3
/// (presents and game frames a second, late refreshes) against the ship
/// bar; the settings and match-config
/// files round-tripped through a scratch app-data folder, a first launch
/// on a clean one, and the ROM lookup in each of the three folders (with
/// `--roms <folder>` naming a ROM set to copy;
/// `tools/reference/local/roms/sf` by default).
fn smoke(args: &[String]) -> i32 {
    let mut failures = 0;
    let mut report = |ok: bool, what: &str| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        failures += usize::from(!ok);
    };
    let pads = PadReader::open();
    let names = pads.names();
    println!(
        "pads: {}",
        if names.is_empty() { "none connected".to_owned() } else { names.join(", ") }
    );
    let source = roms_arg(args).unwrap_or_else(|| PathBuf::from("tools/reference/local/roms/sf"));
    let sweep = args.iter().any(|a| a == "--frame-delay-sweep");
    // `--sharp` runs the sweep with `CRT LOOK` off (the look is what costs
    // the headroom). Without the sweep, the delay is checked at the only
    // cap above 0, the sharp picture's (with a look the cap is 0).
    smoke_window(&source, sweep, !sweep || args.iter().any(|a| a == "--sharp"), &mut report);

    let scratch = std::env::temp_dir().join(format!("sf1-port-smoke-{}", std::process::id()));
    let app = scratch.join("app").join("sf1-port");
    let settings = Settings {
        scale: 9,
        fullscreen: false,
        vsync: false,
        frame_delay_ms: FRAME_DELAY_CAP_MS,
        master: 7,
        fm: 4,
        samples: 9,
        rom_folder: Some("D:\\roms\\sf".into()),
        ..Settings::default()
    };
    let mut config = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive);
    config.difficulty = 3;
    config.round_time = 200;
    let saved = settings.save(&files::settings_path(&app)).is_ok()
        && config.save(&files::match_config_path(&app)).is_ok();
    let (back, diag) = Settings::load_or_default(&files::settings_path(&app));
    let (config_back, config_diag) = MatchConfig::load_or_default(&files::match_config_path(&app));
    report(
        saved && diag.is_none() && config_diag.is_none() && back == settings && config_back == config,
        &format!("settings and match config round-trip in {}", app.display()),
    );
    let _ = std::fs::write(files::settings_path(&app), "scale = eleven\n");
    let mut log = Log::create(&files::log_path(&app));
    let loaded = files::load(&app, &mut log);
    let text = std::fs::read_to_string(files::log_path(&app)).unwrap_or_default();
    report(
        loaded.settings == Settings::default() && text.contains("parse: "),
        "a corrupted settings file is logged and falls back to the defaults",
    );

    // A first launch on a clean app-data folder.
    let clean = scratch.join("clean").join("sf1-port");
    let mut log = Log::create(&files::log_path(&clean));
    let first = files::load(&clean, &mut log);
    let written = [files::settings_path(&clean), files::match_config_path(&clean), files::bindings_path(&clean)]
        .iter()
        .all(|p| p.is_file());
    report(
        written
            && first.settings.scale == 3
            && first.settings.fullscreen
            && first.settings.vsync
            && first.settings.frame_delay_ms == 0
            && first.settings.aspect == Aspect::True4x3
            && first.settings.crt_look == sf_engine::front::settings::DEFAULT_LOOK
            && [first.settings.master, first.settings.announcer, first.settings.fm, first.settings.samples] == [10; 4]
            && first.config == MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul)
            && first.bindings == BindingsTable::default(),
        "a first launch on a clean folder writes the three files with the spec's defaults (fullscreen, ARCADE CABINET at true 4:3)",
    );

    // The ROM lookup: the set copied into each folder in turn.
    if roms::find(&[Candidate { label: String::new(), path: Some(source.clone()) }]).is_err() {
        println!("skip ROM lookup: no ROM set at {} (pass --roms <folder>)", source.display());
    } else {
        let (exe, app, named) = (scratch.join("exe"), scratch.join("app2"), scratch.join("named"));
        let candidates = roms::candidates(None, Some(&exe), &app, Some(&named.display().to_string()));
        let none = roms::find(&candidates);
        report(
            matches!(&none, Err(looked) if looked.len() == 3),
            "no set: each of the three folders is reported",
        );
        for dir in [exe.join("roms").join("sf"), app.join("roms").join("sf"), named.clone()] {
            let copied = copy_set(&source, &dir);
            let found = roms::find(&candidates).map(|s| s.folder);
            report(
                copied && found.as_ref().ok() == Some(&dir),
                &format!("the set is found in {}", dir.display()),
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
    println!("{}", if failures == 0 { "smoke check passed" } else { "smoke check FAILED" });
    i32::from(failures != 0)
}

/// How long the smoke check's window presents in fullscreen for each
/// pacing report.
const PACING_RUN: Duration = Duration::from_secs(5);
/// How long each frame delay is run for, and (sweeping) the largest tried.
const SWEEP_RUN: Duration = Duration::from_secs(4);
const SWEEP_TO_MS: u32 = 8;
/// How long a look may take to load before the smoke check gives up on it.
const LOOK_LOAD_LIMIT: Duration = Duration::from_secs(30);

/// The smoke check's window: windowed at 2x, loading every built-in look
/// in turn (its load time or error), switched to borderless fullscreen in
/// place, presenting for [`PACING_RUN`] sharp, then with vsync off from the
/// moment it is switched (60 game frames a second still), then through each
/// built-in look at true 4:3 with the attract running (or a moving test
/// picture without a ROM set), then sharp at true 4:3 at the largest
/// `FRAME DELAY` (with a look the cap is 0, so there is nothing to check
/// through one), then switched back to windowed in place. Each
/// check's outcome is pushed to `checks`; the pacing notes include the work
/// a present takes, which the frame delay cap is measured from.
struct SmokeWindow {
    session: Option<GameSession>,
    frame: Frame,
    frames: u64,
    stage: SmokeStage,
    since: Duration,
    start: Option<sf_host::pacing::PresentStats>,
    checks: Vec<(bool, String)>,
    notes: Vec<String>,
    catalogue: Catalogue,
    /// The look the window draws through, and what became of it.
    look: Option<LookRequest>,
    event: Option<LookEvent>,
    /// The frame delay the window runs at in [`SmokeStage::DelayPacing`]
    /// and, when sweeping (`--frame-delay-sweep`), the last one to try.
    delay_ms: u32,
    sweep: bool,
    /// The delay stage runs with the sharp picture.
    sharp: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SmokeStage {
    Windowed,
    /// Loading the built-in look at this index.
    Looks(usize),
    Fullscreen,
    Pacing,
    /// Sharp with vsync off, counted from the switch.
    VsyncOff,
    /// Loading the built-in look at this index in fullscreen at true 4:3,
    /// then its pacing.
    LookLoad(usize),
    LookPacing(usize),
    /// Sharp at the largest frame delay, or each delay in turn when sweeping
    /// (through `ARCADE CABINET` unless `--sharp`).
    DelayPacing,
    Back,
}

impl SmokeWindow {
    fn next(&mut self, stage: SmokeStage, window: &WindowState) {
        self.stage = stage;
        self.since = window.elapsed;
    }

    /// Ask the window for the built-in look at `index` (`None`: sharp).
    fn want(&mut self, index: Option<usize>) {
        self.event = None;
        self.look = index.and_then(|i| {
            let b = &shaders::BUILT_IN[i];
            look_request(&self.catalogue, b.id)
        });
    }

    /// After the look at `index` in fullscreen: the next one's pacing, or
    /// back to windowed after the last.
    fn next_look(&mut self, index: usize, window: &WindowState) {
        if index + 1 < shaders::BUILT_IN.len() {
            self.want(Some(index + 1));
            self.next(SmokeStage::LookLoad(index + 1), window);
        } else {
            // Then sharp at the largest frame delay, or each delay in turn
            // when sweeping (through `ARCADE CABINET` unless `--sharp`).
            let arcade = shaders::BUILT_IN.iter().position(|b| b.id == sf_engine::front::settings::DEFAULT_LOOK);
            self.want(if self.sharp { None } else { arcade });
            self.start = None;
            self.delay_ms = if self.sweep { 0 } else { u32::from(FRAME_DELAY_CAP_MS) };
            self.next(SmokeStage::DelayPacing, window);
        }
    }

    /// The pacing since the stage began, as a note, and whether it meets
    /// the ship bar (spec #203): at 120 Hz about 117-120 presents and
    /// 58.5-60 game frames a second, a handful of late refreshes per 5 s.
    /// Scaled to the refresh found.
    fn pacing(&mut self, what: &str, window: &WindowState) -> bool {
        let start = self.start.unwrap_or_default();
        let seconds = window.elapsed.saturating_sub(self.since).as_secs_f64();
        let presents = (window.stats.presents - start.presents) as f64 / seconds;
        let fps = (window.stats.frames - start.frames) as f64 / seconds;
        let late = window.stats.late - start.late;
        // A check on the late count: the refreshes in the run less the presents.
        let missed = window.refresh_hz.map_or(0.0, |hz| (seconds * hz).round() - (presents * seconds).round());
        let refresh = window.refresh_hz.map_or("unknown".to_owned(), |hz| format!("{hz:.2} Hz"));
        self.notes.push(format!(
            "pacing {what} over {seconds:.1} s at {refresh} ({:?}), {}x{}: {presents:.1} presents/s, {fps:.2} game fps, {late} late refreshes ({missed} refreshes less presents); work per present {:.2} ms mean, {:.2} ms longest so far",
            window.pacing,
            window.surface.0,
            window.surface.1,
            window.stats.mean_work_ms_since(&start),
            window.stats.work_max.as_secs_f64() * 1000.0
        ));
        match (window.pacing, window.refresh_hz) {
            (Pacing::EveryNth(_), Some(hz)) => presents >= hz * 0.975 && (58.5..=60.5).contains(&fps),
            _ => (58.5..=60.5).contains(&fps),
        }
    }
}

impl Driver for SmokeWindow {
    fn step(&mut self, _keyboard: &mut Keyboard, window: &WindowState) -> bool {
        self.frames += 1;
        match &mut self.session {
            Some(session) => self.frame = session.step(Default::default()).frame,
            None => {
                // A bar sweeping across, one pixel a frame.
                let x = (self.frames as usize) % WIDTH;
                for (i, px) in self.frame.argb.iter_mut().enumerate() {
                    *px = if i % WIDTH == x { 0xffff_ffff } else { 0xff20_2040 };
                }
            }
        }
        // Each stage settles for a second before it is checked: a mode
        // switch reaches the surface a few frames later.
        let waited = window.elapsed.saturating_sub(self.since);
        let settled = waited >= Duration::from_secs(1);
        match self.stage {
            SmokeStage::Windowed | SmokeStage::Back if settled => {
                let ok = window.surface == (768, 448) && window.picture.scale == 2 && !window.fullscreen;
                let how = if self.stage == SmokeStage::Back { "switched back to windowed in place" } else { "window opened" };
                self.checks.push((ok, format!("{how} at 2x ({}x{} surface)", window.surface.0, window.surface.1)));
                if self.stage == SmokeStage::Back {
                    return false;
                }
                self.want(Some(0));
                self.next(SmokeStage::Looks(0), window);
            }
            SmokeStage::Looks(i) if self.event.is_some() || waited >= LOOK_LOAD_LIMIT => {
                let name = shaders::BUILT_IN[i].name;
                self.checks.push(match self.event.take() {
                    Some(LookEvent::Loaded { took, .. }) => (true, format!("look {name} loads ({:.2} s)", took.as_secs_f64())),
                    Some(LookEvent::Failed { error, .. }) => (false, format!("look {name} loads: {error}")),
                    None => (false, format!("look {name} loads within {} s", LOOK_LOAD_LIMIT.as_secs())),
                });
                if i + 1 < shaders::BUILT_IN.len() {
                    self.want(Some(i + 1));
                    self.next(SmokeStage::Looks(i + 1), window);
                } else {
                    self.want(None);
                    self.next(SmokeStage::Fullscreen, window);
                }
            }
            SmokeStage::Fullscreen if settled => {
                let expected = window.monitor.map(|m| picture_rect(m, Aspect::Square));
                let ok = window.fullscreen && Some(window.surface) == window.monitor && expected.map(|e| e.scale) == Some(window.picture.scale);
                self.checks.push((
                    ok,
                    format!(
                        "switched in place to borderless fullscreen at {}x{}, integer scale {}",
                        window.surface.0, window.surface.1, window.picture.scale
                    ),
                ));
                self.start = Some(window.stats);
                self.next(SmokeStage::Pacing, window);
            }
            SmokeStage::Pacing if waited >= PACING_RUN => {
                let ok = self.pacing("sharp", window);
                self.checks.push((ok, "sharp pacing meets the ship bar (presents within 2.5% of the refresh, 58.5-60 game fps)".to_owned()));
                // Counted from the moment vsync goes off: no settling.
                self.start = Some(window.stats);
                self.next(SmokeStage::VsyncOff, window);
            }
            SmokeStage::VsyncOff if waited >= PACING_RUN => {
                let ok = self.pacing("sharp, vsync off", window);
                self.checks.push((ok, "with vsync off, switched from on, the game holds 60 game fps".to_owned()));
                self.want(Some(0));
                self.next(SmokeStage::LookLoad(0), window);
            }
            // Loaded, then a second to settle before the count starts.
            SmokeStage::LookLoad(i) if matches!(self.event, Some(LookEvent::Loaded { .. })) && settled => {
                self.start = Some(window.stats);
                self.next(SmokeStage::LookPacing(i), window);
            }
            SmokeStage::LookLoad(i) if matches!(self.event, Some(LookEvent::Failed { .. })) || waited >= LOOK_LOAD_LIMIT => {
                let name = shaders::BUILT_IN[i].name;
                self.checks.push((false, format!("{name} loads in fullscreen for its pacing")));
                self.next_look(i, window);
            }
            SmokeStage::LookPacing(i) if waited >= PACING_RUN => {
                let name = shaders::BUILT_IN[i].name;
                // A draw error during the run shows as a failure event.
                let ok = self.pacing(&format!("through {name} at true 4:3"), window) && !matches!(self.event, Some(LookEvent::Failed { .. }));
                self.checks.push((
                    ok,
                    format!("{name} pacing meets the ship bar (presents within 2.5% of the refresh, 58.5-60 game fps)"),
                ));
                self.next_look(i, window);
            }
            // Each delay gets a second to settle (the swapchain is made
            // again) and its look time to load, then the run.
            SmokeStage::DelayPacing if self.start.is_none() => {
                if settled && (self.look.is_none() || self.event.is_some() || waited >= LOOK_LOAD_LIMIT) {
                    self.start = Some(window.stats);
                    self.since = window.elapsed;
                }
            }
            SmokeStage::DelayPacing if waited >= SWEEP_RUN => {
                let delay = self.delay_ms;
                let through = if self.sharp { "sharp" } else { "through ARCADE CABINET at true 4:3" };
                let ok = self.pacing(&format!("{through} at a {delay} ms frame delay"), window)
                    && !matches!(self.event, Some(LookEvent::Failed { .. }));
                let picture = if self.sharp { "the sharp picture" } else { "ARCADE CABINET" };
                self.checks.push((ok, format!("{picture} at a {delay} ms FRAME DELAY meets the ship bar")));
                if self.sweep && delay < SWEEP_TO_MS {
                    self.delay_ms += 1;
                    self.start = None;
                    self.next(SmokeStage::DelayPacing, window);
                } else {
                    self.want(None);
                    self.next(SmokeStage::Back, window);
                }
            }
            _ => {}
        }
        true
    }

    fn frame(&self) -> &Frame {
        &self.frame
    }

    fn video(&self) -> VideoMode {
        VideoMode {
            scale: 2,
            fullscreen: matches!(
                self.stage,
                SmokeStage::Fullscreen
                    | SmokeStage::Pacing
                    | SmokeStage::VsyncOff
                    | SmokeStage::LookLoad(_)
                    | SmokeStage::LookPacing(_)
                    | SmokeStage::DelayPacing
            ),
            aspect: match self.stage {
                SmokeStage::LookLoad(_) | SmokeStage::LookPacing(_) | SmokeStage::DelayPacing => Aspect::True4x3,
                _ => Aspect::Square,
            },
            vsync: self.stage != SmokeStage::VsyncOff,
            frame_delay_ms: if self.stage == SmokeStage::DelayPacing { self.delay_ms } else { 0 },
        }
    }

    fn note(&mut self, line: &str) {
        self.notes.push(line.to_owned());
    }

    fn look(&self) -> Option<&LookRequest> {
        self.look.as_ref()
    }

    fn look_event(&mut self, event: LookEvent) {
        self.event = Some(event);
    }
}

/// Open the smoke check's window and run it; the attract runs behind it
/// when the ROM set at `source` loads.
fn smoke_window(source: &Path, sweep: bool, sharp: bool, report: &mut dyn FnMut(bool, &str)) {
    let session = roms::find(&[Candidate { label: String::new(), path: Some(source.to_path_buf()) }])
        .ok()
        .map(|set| GameSession::new(set.assets, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul)));
    println!(
        "window: {}",
        if session.is_some() { "the attract runs behind the checks" } else { "no ROM set: a test picture runs behind the checks" }
    );
    let catalogue = Catalogue::built_in(&shaders::shaders_dir());
    println!("looks: {}", shaders::shaders_dir().display());
    let mut smoke = SmokeWindow {
        session,
        frame: Frame { argb: vec![0xff00_0000; WIDTH * HEIGHT] },
        frames: 0,
        stage: SmokeStage::Windowed,
        since: Duration::ZERO,
        start: None,
        checks: Vec::new(),
        notes: Vec::new(),
        catalogue,
        look: None,
        event: None,
        delay_ms: u32::from(FRAME_DELAY_CAP_MS),
        sweep,
        sharp,
    };
    let video = smoke.video();
    let delays = if sweep { SWEEP_TO_MS + 1 } else { 1 };
    let looks = shaders::BUILT_IN.len() as u32;
    let limit = (PACING_RUN + Duration::from_secs(1)) * (2 + looks)
        + (SWEEP_RUN + Duration::from_secs(2)) * delays
        + LOOK_LOAD_LIMIT * (2 * looks + 1)
        + Duration::from_secs(20);
    let result = gpu_window::run("Street Fighter: smoke check", video, &mut smoke, Some(limit));
    for note in &smoke.notes {
        println!("window: {note}");
    }
    if let Err(e) = result {
        report(false, &format!("the GPU window: {e}"));
        return;
    }
    let finished = smoke.stage == SmokeStage::Back && smoke.checks.len() >= 6 + 2 * shaders::BUILT_IN.len();
    for (ok, what) in smoke.checks {
        report(ok, &what);
    }
    report(finished, "the window ran every check before its time limit");
}

/// Copy the files the manifest lists from `from` to `to`.
fn copy_set(from: &Path, to: &Path) -> bool {
    std::fs::create_dir_all(to).is_ok()
        && roms::file_names().iter().all(|name| std::fs::copy(from.join(name), to.join(name)).is_ok())
}
