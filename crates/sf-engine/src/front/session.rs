//! The headless game session (spec #150, user story 74): the whole game
//! without its window, audio device or pads. Each [`GameSession::step`]
//! takes one frame of logical inputs and yields the presented frame, the
//! sound latch, the current screen's name and any transition. The game
//! binary is a window loop over it; the acceptance tests drive it directly.

use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};

use sf_video::busts::{decode_select_busts, Bust};
use sf_video::compositor::RomFont;
use sf_video::render::{render_frame, Frame, SpriteArt};
use sf_video::romset::Assets;
use sf_video::background_bank::{shown_stages, BackgroundBank, MissingTheme, RELEASED_WAVES};
use sf_video::stage_preview::StagePreviews;
use sf_video::sprite_bank::SpriteBank;

use super::after_match::AfterMatchBox;
use super::attract::{FreePlay, TitleLogo};
use super::cabinet::Players;
use super::dramatic_select::{boot_flag, DramaticGuest, DramaticPicks, DramaticSetupScreen};
use super::faithful::{Cabinet, CabinetEvent, DipScreen};
use super::fight::{first_opponent_flag, Fight, FightDips, FightEvent, FightKind, FightSetup, SatsuiArt};
use super::menu::{MenuItem, MenuScreen};
use super::online::{OnlineAction, OnlineScreen};
use super::moves::MovesPanel;
use super::pause::{next_live_preset, PauseBox};
use super::rebind::{Binds, Press, RebindScreen, Walk};
use super::select::SelectScreen;
use super::jukebox::{entries as jukebox_entries, listed_tracks, JukeboxScreen};
use crate::jukebox::TrackFormats;
use super::stage_select::{random_pick, StageScreen};
use super::settings::{default_looks, frame_delay_cap, look_in_use, Aspect, DisplayInfo, LookName, Settings, SettingsScreen};
use super::tune::{DemoScene, LookParam, LookTuning, Panel, SaveLook, TuneScreen, TypedKey};
use super::world_tour::WorldTourScreen;
use super::{sound, Transition};
use crate::config::{EvilRyuPlaytest, MatchConfig, RulesetPreset};
use crate::data_bank::DataBank;
use crate::dramatic::DramaticRules;
use crate::era::{Costume, EraFamily, GaugeMode, Loadout, Revision, StylePick};
use crate::jukebox::{Cues, Resolved, TrackCue};
use crate::fighter::{P1_STRUCT, P2_STRUCT};
use crate::moves::{facing_right, InputNotation};
use crate::roster::{MenuInput, PlayMode, StagePick, TournamentFighter};
use crate::run::PowerOnMachine;
use crate::netplay::{Link, NetTransportCommand, NetTransportEvent, RoomAnnouncement, RoomKind, SyncedMatchRules};
use crate::netplay::protocol::tag_from_str;
use crate::netplay::connection::{Connection, ConnectionEvent};
use crate::netplay::RejectReason;
use crate::canonical::{Canonical, CanonicalHasher};
use crate::world_tour::native::WorldTourProfile;

mod netplay;
mod spectator;
mod world_tour;

/// One player's logical inputs: directions (right, left, down, up) and the
/// six attack buttons (LP, MP, HP, LK, MK, HK), as the machines take them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerInputs {
    pub directions: [bool; 4],
    pub buttons: [bool; 6],
}

/// The front end's own inputs: Enter / Cross confirm, Backspace / Esc /
/// Circle back, the keyboard's or a pad's Start, and the tab pair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuKeys {
    pub confirm: bool,
    pub back: bool,
    pub start: bool,
    pub tab_left: bool,
    pub tab_right: bool,
}

/// Everything held on one frame: both players, the menu keys, and the
/// cabinet switches (`coin1`, `coin2`, `start1`, `start2`, `service1`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameInputs {
    pub p1: PlayerInputs,
    pub p2: PlayerInputs,
    pub menu: MenuKeys,
    pub cabinet: [bool; 5],
    /// Each side's pad menu buttons, when a pad drives that side (Cross,
    /// Circle, Options, L1 and R1 by default). A pad side's LP and LK do
    /// not stand in for confirm and back as the keyboard's do: on the
    /// Classic pad Cross is both confirm and LK.
    pub pads: [Option<MenuKeys>; 2],
    /// The sides whose pad was lost on this frame.
    pub pad_lost: [bool; 2],
    /// The key or pad button first pressed on this frame (the rebind walk).
    pub press: Option<Press>,
}

/// One presented frame.
pub struct FrameOutput {
    pub frame: Frame,
    /// The sound clock: frames presented since power-on (1 on the first
    /// step). The host pushes `sound_latch` at this frame.
    pub sound_frame: u32,
    pub sound_latch: u8,
    /// The latch write's cycles into the frame (the machine's vblank
    /// handler's on attract frames, 0 on the front end's own).
    pub latch_cycles: u32,
    /// The announcer's lines asked for on this frame (an overhaul fight's;
    /// ADR 0011): the host asks his chip for them at `sound_frame`.
    pub announcements: Vec<sf_video::announcer::AnnouncerLine>,
    /// A fight under an overhaul preset is running (its boxes included): the
    /// host runs the music CPU with the extended tune table while this holds
    /// and takes it away after (ticket #199). Never in the attract, the
    /// faithful cabinet or a Faithful 1987 fight.
    pub tune_table: bool,
    /// What the host does with the Jukebox's track on this frame (ticket
    /// #200): start it where the fight latches its stage theme, stop it
    /// where the driver would stop its music or the win jingle replaces the
    /// theme, or when the fight is left. The host leaves the theme's latch
    /// out of the music CPU's input while the track plays, and passes it on
    /// when it could not play the file. Never in the attract, the faithful
    /// cabinet or a Faithful 1987 fight, and never with no track set.
    pub track: Option<TrackCue>,
    /// The screen now showing.
    pub screen: &'static str,
    /// What the input did this frame, if anything.
    pub transition: Option<Transition>,
    /// Exit was chosen: the window closes.
    pub quit: bool,
    /// The Settings screen was left: save the match-config and settings
    /// files and apply the video and sound settings.
    pub settings_changed: bool,
    /// The bindings changed (a rebind walk ended, or the Controls tab was
    /// reset): save the bindings file from [`GameSession::binds`].
    pub bindings_changed: bool,
    /// The video settings as they stand this frame, the Settings screen's
    /// unsaved rows included, for the host to apply as they change.
    pub video: VideoState,
    /// The tuning screen's panel while it shows: a layer the host draws
    /// sharp over the look, after the filter chain, with the picture moved
    /// to the left edge (the one exception to one frame per screen).
    pub panel: Option<Panel>,
}

/// What the host shows the frames through (spec #203): the window's mode
/// and the look. It follows the Settings screen's rows as they change, so
/// a change shows at once; the files are still saved on leaving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoState {
    pub scale: u8,
    pub fullscreen: bool,
    pub aspect: Aspect,
    /// [`LOOK_OFF`](super::settings::LOOK_OFF) or a look's id from the host's catalogue.
    pub look: String,
    pub vsync: bool,
    /// Milliseconds to wait after each vblank before stepping the game:
    /// the saved delay, clamped to the cap for the look ([`Settings::frame_delay`]).
    pub frame_delay_ms: u8,
}

/// One screen on the stack over the attract.
#[derive(Clone)]
enum Screen {
    Menu(MenuScreen),
    Select(SelectScreen),
    Stage(StageScreen),
    DramaticSetup(DramaticSetupScreen),
    DramaticPick(Box<DramaticPicks>),
    DramaticGuest(Box<DramaticGuest>),
    WorldTour(Box<WorldTourScreen>),
    Settings(Box<SettingsScreen>),
    Dip(DipScreen),
    Rebind(Box<RebindScreen>),
    Tune(Box<TuneScreen>),
    Jukebox(Box<JukeboxScreen>),
    Online(Box<OnlineScreen>),
}

impl Screen {
    fn name(&self) -> &'static str {
        match self {
            Screen::Menu(m) => m.name(),
            Screen::Select(s) => s.name(),
            Screen::Stage(s) => s.name(),
            Screen::DramaticSetup(s) => s.name(),
            Screen::DramaticPick(_) => "dramatic select",
            Screen::DramaticGuest(_) => "dramatic guest",
            Screen::WorldTour(screen) => screen.name(),
            Screen::Settings(s) => s.name(),
            Screen::Dip(d) => d.name(),
            Screen::Rebind(r) => r.name(),
            Screen::Tune(t) => t.name(),
            Screen::Jukebox(j) => j.name(),
            Screen::Online(o) => o.name(),
        }
    }

    fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        match self {
            Screen::Menu(m) => m.apply(input),
            Screen::Select(s) => s.apply(input),
            Screen::Stage(s) => s.apply(input),
            Screen::DramaticSetup(s) => s.apply(input),
            Screen::DramaticPick(s) => s.apply(input),
            Screen::DramaticGuest(s) => s.apply(input),
            Screen::Settings(s) => s.apply(input),
            Screen::Dip(d) => d.apply(input),
            Screen::Tune(t) => t.apply(input),
            Screen::Jukebox(j) => j.apply(input),
            Screen::Online(o) => o.apply(input),
            // The walk takes presses, not menu inputs.
            Screen::Rebind(_) => None,
            // World Tour also takes the shared campaign profile.
            Screen::WorldTour(_) => None,
        }
    }
}

/// A fight in progress, its pause box while paused, and its after-match
/// box once the match is over.
#[derive(Clone)]
struct Play {
    fight: Fight,
    world_tour: Option<world_tour::WorldTourRun>,
    netplay: Option<crate::netplay::RollbackEngine>,
    pause: Option<PauseBox>,
    /// The move list over the pause box, while it shows.
    moves: Option<MovesPanel>,
    /// The side (0 Player 1, 1 Player 2) whose pad opened the pause box;
    /// Player 1 when it was a key or a lost pad (ticket #229, ML3).
    paused_by: usize,
    /// The side whose move list shows, while one does.
    moves_side: usize,
    after: Option<AfterMatchBox>,
    /// The fight's last frame, under a box.
    last: Frame,
    /// The pause box closed this frame: the fight is shown, not stepped,
    /// so a restarted round starts exactly at its snapshot.
    held: bool,
}

#[derive(Clone)]
struct NetplaySetup {
    local_side: Option<usize>,
    delay: u8,
    max_rollback: u8,
    flow: netplay::Flow,
}

/// The game from power-on: the attract machine, the stack of screens over
/// it (empty while the attract shows), and the fight when one is running.
/// A clone is a snapshot of the whole game (the ROM regions are shared).
#[derive(Clone)]
pub struct GameSession {
    assets: Arc<Assets>,
    /// The graphics with the extended sprite bank (ADR-0007) and the
    /// extended background bank (ADR-0009), for fights under an overhaul
    /// preset; `None` when no bank was given. The attract, the faithful
    /// cabinet and Faithful 1987 fights always draw with `assets`, the
    /// ROM's own.
    bank_assets: Option<Arc<Assets>>,
    /// Alternate sprite art for the active Arcade awakening.
    satsui_art: Option<Arc<SatsuiArt>>,
    /// The extended background bank, given to the machine of every fight
    /// under an overhaul preset (never the attract's, the cabinet's or a
    /// Faithful 1987 fight's).
    backgrounds: Option<Arc<BackgroundBank>>,
    /// The extended data bank (ADR-0014): the bosses' 1987 player tables,
    /// given to the machine of every fight under an overhaul preset (never
    /// the attract's, the cabinet's or a Faithful 1987 fight's).
    data_bank: Option<Arc<DataBank>>,
    /// The listed stages' backdrops at preview size, drawn the first time a
    /// stage screen opens (spec #192, ticket #197).
    previews: OnceLock<Arc<StagePreviews>>,
    /// The stage Local and Training fights are fought on whatever the stage
    /// select says (spec #192): a development and test hook, which also
    /// keeps the stage select from showing.
    stage: Option<StagePick>,
    /// The stage the stage select picked for the Local or Training fights
    /// of this session, the stage just played once `RANDOM` is resolved:
    /// where a rematch's cursor starts.
    chosen: Option<StagePick>,
    /// The side (0 Player 1, 1 Player 2) at the controls of the next stage
    /// screen: Player 1 for the first match, then the loser of the last,
    /// and after a draw whoever picked last. Starts over at the menu.
    picker: usize,
    /// Each side's last locked era, reused when a new Style card opens.
    preferred_eras: [EraFamily; 2],
    /// The Jukebox's cue machine.
    track_cues: Cues,
    attract: PowerOnMachine,
    free_play: Option<FreePlay>,
    /// The title logo the Main Menu draws, read from the ROM.
    title_logo: Option<TitleLogo>,
    /// The select's busts, decoded from the ROM at startup.
    busts: Arc<[Option<Bust>; 12]>,
    stack: Vec<Screen>,
    config: MatchConfig,
    settings: Settings,
    /// The Settings screen was left this frame: the files are to be saved.
    settings_changed: bool,
    world_tour_profile: WorldTourProfile,
    world_tour_changed: bool,
    play: Option<Play>,
    /// The faithful cabinet, when the 1987 item has powered it on.
    cabinet: Option<Cabinet>,
    /// Sound commands for the attract's next frames, which hold it still
    /// (the faithful cabinet's sound stopping as the attract returns).
    attract_hold: VecDeque<u8>,
    prev: FrameInputs,
    frames: u32,
    /// Sound commands waiting for the latch, one per front-end frame.
    sounds: VecDeque<u8>,
    /// The bindings by name, and their defaults, when the host gave them
    /// (the rebind walk needs them).
    binds: Option<(Binds, Binds)>,
    bindings_changed: bool,
    /// A side whose pad was lost while the fight could not pause (the
    /// hidden drive, Arcade's continue): the box opens when it can.
    lost_pad: Option<usize>,
    /// The one-line notice on the Main Menu after an untranslated branch
    /// or a look that failed to load.
    notice: Option<String>,
    /// `CRT LOOK`'s list, from the host's catalogue.
    looks: Vec<LookName>,
    /// The files of the `jukebox` folder, as the host lists them (the
    /// Jukebox screen's choices).
    tracks: Vec<String>,
    /// The formats the host plays (ticket #201): what `tracks` is filtered by.
    track_formats: TrackFormats,
    /// The chosen look, when it failed to load this run: drawn as `OFF`,
    /// kept in the settings.
    failed_look: Option<String>,
    /// The parameters of the look the host loaded last (its id and list).
    look_params: Option<(String, Vec<LookParam>)>,
    /// The tuning screen's changes, kept for the session (never saved).
    tuning: Option<LookTuning>,
    /// The scene behind the tuning screen, made when it first opens.
    demo: Option<Box<DemoScene>>,
    /// The display, as the host last reported it.
    display: DisplayInfo,
    /// The look the tuning screen asked to save, until the host takes it.
    save_look: Option<SaveLook>,
    /// The keys typed since the last step, for a text entry.
    typed: Vec<TypedKey>,
    net_events: VecDeque<NetTransportEvent>,
    net_commands: Vec<NetTransportCommand>,
    net_connection: Option<Connection>,
    net_setup: Option<NetplaySetup>,
    net_epoch_cursor: netplay::EpochCursor,
    net_saved_config: Option<MatchConfig>,
    net_request: u64,
    net_time_ms: Option<u32>,
    net_spectators: spectator::State,
    log: Vec<String>,
}

impl GameSession {
    /// A session at power-on. `config` is the overhaul configuration the
    /// player has chosen; the attract
    /// itself always runs the faithful machine with every toggle off.
    pub fn new(assets: Assets, config: MatchConfig) -> Self {
        let attract = PowerOnMachine::new(assets.maincpu.clone());
        let free_play = FreePlay::from_rom(&assets.maincpu);
        let title_logo = TitleLogo::from_rom(&assets.maincpu);
        let busts = Arc::new(decode_select_busts(&assets));
        GameSession {
            assets: Arc::new(assets),
            bank_assets: None,
            satsui_art: None,
            backgrounds: None,
            data_bank: None,
            previews: OnceLock::new(),
            stage: None,
            chosen: None,
            picker: 0,
            preferred_eras: [EraFamily::Sf1Plus; 2],
            track_cues: Cues::default(),
            attract,
            free_play,
            title_logo,
            busts,
            stack: Vec::new(),
            config,
            settings: Settings::default(),
            settings_changed: false,
            world_tour_profile: WorldTourProfile::default(),
            world_tour_changed: false,
            binds: None,
            bindings_changed: false,
            lost_pad: None,
            play: None,
            cabinet: None,
            attract_hold: VecDeque::new(),
            prev: FrameInputs::default(),
            frames: 0,
            sounds: VecDeque::new(),
            notice: None,
            looks: default_looks(),
            tracks: Vec::new(),
            track_formats: TrackFormats::default(),
            failed_look: None,
            look_params: None,
            tuning: None,
            demo: None,
            display: DisplayInfo::default(),
            save_look: None,
            typed: Vec::new(),
            net_events: VecDeque::new(),
            net_commands: Vec::new(),
            net_connection: None,
            net_setup: None,
            net_epoch_cursor: netplay::EpochCursor::default(),
            net_saved_config: None,
            net_request: 0,
            net_time_ms: None,
            net_spectators: spectator::State::default(),
            log: Vec::new(),
        }
    }

    /// Install alternate art for the awakened fighter and its red fireball.
    pub fn with_satsui_art(mut self, body: SpriteArt, fire: SpriteArt) -> Self {
        self.satsui_art = Some(Arc::new(SatsuiArt { body, fire }));
        self
    }

    /// The host loaded the look `look`, whose parameters are `params` (in
    /// the shader's own order, headings included): the tuning screen lists
    /// them, an open one at once.
    pub fn set_look_params(&mut self, look: &str, params: Vec<LookParam>) {
        let tuning = self.tuning.as_ref().filter(|t| t.look == look).map_or(&[][..], |t| &t.values[..]);
        for screen in &mut self.stack {
            if let Screen::Tune(t) = screen {
                if t.look().0 == look {
                    t.set_params(params.clone(), tuning);
                }
            }
        }
        self.look_params = Some((look.to_owned(), params));
    }

    /// The tuning to set on the current look's chain: the session's changes
    /// from the tuning screen when they are for the current look. The host
    /// sets every other parameter to the look's own value.
    pub fn look_tuning(&self) -> Option<&LookTuning> {
        let look = self.video().look;
        self.tuning.as_ref().filter(|t| t.look == look)
    }

    /// The display's refresh rate and the largest window scale that fits
    /// it, as the host reads them: the Video tab's notice line and the range
    /// of `WINDOW SCALE`.
    pub fn set_display(&mut self, display: DisplayInfo) {
        self.display = display;
        for screen in &mut self.stack {
            if let Screen::Settings(s) = screen {
                s.set_display(display);
            }
        }
    }

    /// Builder: the look list the Settings screen's `CRT LOOK` steps
    /// through, `OFF` first (the host's catalogue). An empty list keeps the
    /// default `OFF` and `ARCADE CABINET`.
    pub fn with_looks(mut self, looks: Vec<LookName>) -> Self {
        if !looks.is_empty() {
            self.looks = looks;
        }
        self
    }

    /// The look list changed (the host re-reads its catalogue, drop-ins
    /// included, as Settings opens): `CRT LOOK` steps through `looks` from
    /// now on, on an open Settings screen too. An empty list is ignored.
    pub fn set_looks(&mut self, looks: Vec<LookName>) {
        if looks.is_empty() {
            return;
        }
        for screen in &mut self.stack {
            if let Screen::Settings(s) = screen {
                **s = (**s).clone().with_looks(looks.clone());
            }
        }
        self.looks = looks;
    }

    /// The files in the `jukebox` folder beside the ROM set, as the host
    /// lists them (ticket #202): the Jukebox screen's choices, an open one
    /// at once. Only names of the formats the Jukebox plays are kept, each
    /// once ([`listed_tracks`]).
    pub fn set_tracks(&mut self, tracks: Vec<String>) {
        self.tracks = listed_tracks(tracks, &self.track_formats);
        for screen in &mut self.stack {
            if let Screen::Jukebox(j) = screen {
                j.set_tracks(self.tracks.clone());
            }
        }
    }

    /// The formats the host plays (ticket #201): the audio formats, and the
    /// tracker modules when the host was built with their player. Set before
    /// [`Session::set_tracks`]; a build that never sets it offers the audio
    /// formats only.
    pub fn set_track_formats(&mut self, formats: TrackFormats) {
        self.track_formats = formats;
    }

    /// The files the Jukebox screen offers.
    pub fn tracks(&self) -> &[String] {
        &self.tracks
    }

    /// The video settings in force this frame: the open Settings screen's
    /// copy while it shows, else the session's.
    pub fn video(&self) -> VideoState {
        let (settings, look) = match self.stack.iter().rev().find_map(|s| match s {
            Screen::Settings(s) => Some(s),
            _ => None,
        }) {
            Some(s) => (s.settings(), s.look()),
            None => (&self.settings, self.look()),
        };
        VideoState {
            scale: settings.scale,
            fullscreen: settings.fullscreen,
            aspect: settings.aspect,
            look: look.to_owned(),
            vsync: settings.vsync,
            frame_delay_ms: settings.frame_delay_ms.min(frame_delay_cap(look)),
        }
    }

    /// The session's look drawn this run: the settings' `crt_look`, or
    /// `OFF` while it is the look that failed to load.
    fn look(&self) -> &str {
        look_in_use(&self.settings.crt_look, self.failed_look.as_deref())
    }

    /// The host could not load the look `id` (`error` is its first line):
    /// the look drawn is `OFF` for this run, on an open Settings screen too,
    /// and the notice line names the error. The settings keep `id`, so the
    /// fallback is never saved and the next launch tries the look again.
    pub fn look_failed(&mut self, id: &str, error: &str) {
        // Upper-cased, in what the ROM font draws (`_` as a space, `\` as
        // `/`, anything else it lacks as `?`), cut by characters: an error
        // can name an accented file or user folder.
        let notice: String = format!("LOOK FAILED: {error}")
            .to_uppercase()
            .chars()
            .map(|c| match c {
                '_' => ' ',
                '\\' => '/',
                c if sf_video::compositor::drawable(c) => c,
                _ => '?',
            })
            .take(sf_video::compositor::TEXT_COLS - 2)
            .collect();
        if self.settings.crt_look == id {
            self.failed_look = Some(id.to_owned());
        }
        for screen in &mut self.stack {
            if let Screen::Settings(s) = screen {
                s.look_failed(id, &notice);
            }
        }
        self.notice = Some(notice);
    }

    /// Builder: the settings file's contents.
    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    /// Builder: the World Tour profile loaded by the host.
    pub fn with_world_tour_profile(mut self, profile: WorldTourProfile) -> Self {
        self.world_tour_profile = profile;
        self
    }

    pub fn world_tour_profile(&self) -> &WorldTourProfile {
        &self.world_tour_profile
    }

    pub fn take_world_tour_changed(&mut self) -> bool {
        std::mem::take(&mut self.world_tour_changed)
    }

    /// The settings (video, sound, ROM folder, the cabinet's DIP words).
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The picture's aspect as the player has it now: the open Settings
    /// screen's choice, which applies the moment it changes, else the
    /// saved one. The file is written on leaving the screen.
    pub fn live_aspect(&self) -> Aspect {
        self.stack
            .iter()
            .rev()
            .find_map(|s| match s {
                Screen::Settings(s) => Some(s.settings().aspect),
                _ => None,
            })
            .unwrap_or(self.settings.aspect)
    }

    /// Builder: the bindings by name and their defaults (the Controls tab's
    /// rebind walks and `RESET TAB` change them).
    pub fn with_binds(mut self, binds: Binds, defaults: Binds) -> Self {
        self.binds = Some((binds, defaults));
        self
    }

    /// `screen` opened where it was last left.
    fn menu(&self, screen: MenuScreen) -> Screen {
        let row = self.settings.menu_positions[screen.slot()];
        let overhaul = self.config.preset != RulesetPreset::Faithful1987;
        Screen::Menu(screen.with_overhaul_modes(overhaul).at(row))
    }

    /// The item under the cursor, when a menu is showing.
    pub fn menu_item(&self) -> Option<MenuItem> {
        match (&self.play, self.stack.last()) {
            (None, Some(Screen::Menu(m))) if self.cabinet.is_none() => m.cursor().map(|(_, item)| item),
            _ => None,
        }
    }

    /// Builder: the extended sprite bank, drawn in fights under an
    /// overhaul preset.
    pub fn with_sprite_bank(mut self, bank: &SpriteBank) -> Self {
        let base = self.bank_assets.as_deref().unwrap_or(&self.assets);
        self.bank_assets = Some(Arc::new(base.with_sprite_bank(bank)));
        self
    }

    /// Builder: the extended background bank (ADR-0009): its tiles and
    /// pages drawn, and its stages and palettes read by the stage gates, in
    /// fights under an overhaul preset.
    pub fn with_background_bank(mut self, mut bank: BackgroundBank) -> Self {
        if let Some(prev) = self.backgrounds.as_deref() {
            let playable = prev.playable_themes();
            if !playable.is_empty() {
                bank.set_playable_themes(&playable);
            }
        }
        let base = self.bank_assets.as_deref().unwrap_or(&self.assets);
        self.bank_assets = Some(Arc::new(base.with_background_bank(&bank)));
        self.backgrounds = Some(Arc::new(bank));
        self.previews = OnceLock::new();
        self
    }

    /// Builder: the extended data bank (ADR-0014), built from the ROM and
    /// the fighters' pose catalogues (the ones the extended sprite bank
    /// given by [`Self::with_sprite_bank`] holds): the bosses whose poses
    /// loaded play on 1987 player tables in fights under an overhaul preset.
    /// Its notes (who has a table, and why anyone does not) go to the log.
    pub fn with_player_tables(mut self, catalogues: &[sf_video::art::Catalogue]) -> Self {
        let bank = DataBank::build(&self.assets.maincpu, catalogues);
        for note in bank.notes() {
            self.log.push(format!("fighters: {note}"));
        }
        self.data_bank = (!bank.is_empty()).then(|| Arc::new(bank));
        self
    }

    /// The extended data bank, when one was built.
    pub fn data_bank(&self) -> Option<&DataBank> {
        self.data_bank.as_deref()
    }

    /// The bosses the extended data bank holds 1987 player tables for.
    fn player_tables(&self) -> &[TournamentFighter] {
        self.data_bank.as_deref().map_or(&[], DataBank::fighters)
    }

    /// Say which extended tune commands the music CPU can play in an
    /// overhaul fight (ticket #199). A stage naming an extended theme not
    /// among them plays a 1987 stage theme (`FALLBACK_THEME`) and the fight
    /// starts as it would. Returns the stages that fall back, for the host's
    /// log; fights already running keep the themes they started with.
    pub fn set_playable_themes(&mut self, commands: &[u8]) -> Vec<MissingTheme> {
        let bank = self.backgrounds.get_or_insert_with(|| Arc::new(BackgroundBank::new()));
        let bank = Arc::make_mut(bank);
        bank.set_playable_themes(commands);
        bank.missing_themes()
    }

    /// The extended background bank, when one was given.
    pub fn background_bank(&self) -> Option<&BackgroundBank> {
        self.backgrounds.as_deref().filter(|b| !b.is_empty())
    }

    /// Pick the stage the next Local and Training fights under an overhaul
    /// preset are fought on (`None`: the stage select's pick, or the 1987
    /// flow's own). A stage set here is fought on without the stage select
    /// showing; tests and `sf-game --stage` use this.
    pub fn set_stage(&mut self, stage: Option<StagePick>) {
        self.stage = stage;
    }

    /// The graphics for the running fight, including the banks that its
    /// preset uses. Use these when stepping or rendering a cloned fight.
    pub fn fight_assets(&self) -> Option<&Assets> {
        let preset = self.play.as_ref()?.fight.setup().config.preset;
        Some(fight_assets(&self.assets, &self.bank_assets, preset))
    }

    /// The map pages the running fight draws from: the ROM tilemap's, plus
    /// the background bank's under an overhaul preset. `None` when no fight
    /// is running.
    pub fn fight_background_pages(&self) -> Option<usize> {
        self.fight_assets().map(Assets::background_pages)
    }

    /// The sprite tiles the running fight draws with: the ROM's 14,336, plus
    /// the bank's under an overhaul preset. `None` when no fight is running.
    pub fn fight_sprite_tiles(&self) -> Option<usize> {
        self.fight_assets().map(Assets::sprite_tiles)
    }

    /// The bindings by name, when the host gave them.
    pub fn binds(&self) -> Option<&Binds> {
        self.binds.as_ref().map(|(b, _)| b)
    }

    /// The attract's machine, read-only (for the tests and the log).
    pub fn attract_machine(&self) -> &PowerOnMachine {
        &self.attract
    }

    /// The running fight's machine, read-only.
    pub fn fight_machine(&self) -> Option<&PowerOnMachine> {
        self.play.as_ref().map(|p| p.fight.machine())
    }

    /// The faithful cabinet's machine, while it runs.
    pub fn cabinet_machine(&self) -> Option<&PowerOnMachine> {
        self.cabinet.as_ref().map(Cabinet::machine)
    }

    /// The running fight, read-only.
    pub fn fight(&self) -> Option<&Fight> {
        self.play.as_ref().map(|p| &p.fight)
    }

    /// The move list over the pause box, while it shows (the host and the
    /// tests read it; the panel layer is in [`FrameOutput::panel`]).
    pub fn move_list(&self) -> Option<&MovesPanel> {
        self.play.as_ref().and_then(|p| p.moves.as_ref())
    }

    /// The coach, while a Try of a move runs in the fight (Training; the
    /// host and the tests read it).
    pub fn coach(&self) -> Option<&crate::front::coach::Coach> {
        self.play.as_ref().and_then(|p| p.fight.coach())
    }

    /// Call-out events for the running fight's next frame (see
    /// [`Fight::push_callouts`]); none without a fight.
    pub fn push_callouts(&mut self, events: &[sf_video::callouts::CalloutEvent]) {
        if let Some(play) = &mut self.play {
            play.fight.push_callouts(events);
        }
    }

    /// A finished combo for the running fight's announcer (see
    /// [`Fight::finish_combo`]); nothing without a fight.
    pub fn finish_combo(&mut self, hits: u16) {
        if let Some(play) = &mut self.play {
            play.fight.finish_combo(hits);
        }
    }

    /// Fight reads for the running fight's next stepped frames (see
    /// [`Fight::feed_reads`]); none without a fight.
    pub fn feed_fight_reads(&mut self, reads: &[crate::fight_reader::FightRead]) {
        if let Some(play) = &mut self.play {
            play.fight.feed_reads(reads);
        }
    }

    /// The era gauges for the running fight (see [`Fight::set_era_hud`]);
    /// none without a fight.
    pub fn set_era_hud(&mut self, hud: sf_video::era_hud::EraHud) {
        if let Some(play) = &mut self.play {
            play.fight.set_era_hud(hud);
        }
    }

    /// The unmodified ROM assets. Use [`Self::fight_assets`] to render a fight.
    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn match_config(&self) -> &MatchConfig {
        &self.config
    }

    /// The player's offline preferences, including while a room supplies
    /// temporary host rules for the active match.
    pub fn local_match_config(&self) -> &MatchConfig {
        self.net_saved_config.as_ref().unwrap_or(&self.config)
    }

    /// The difficulty and round time every fight powers on with (the
    /// configuration's Gameplay rows).
    pub fn fight_dips(&self) -> FightDips {
        FightDips {
            difficulty: self.config.difficulty,
            round_time: self.config.round_time,
        }
    }

    pub fn set_fight_dips(&mut self, dips: FightDips) {
        self.config.difficulty = dips.difficulty;
        self.config.round_time = dips.round_time;
    }

    /// The Main Menu's one-line notice, if one is up.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Lines for the log since the last call (untranslated branches with
    /// their frame and addresses).
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log)
    }

    /// The screen now showing.
    pub fn screen(&self) -> &'static str {
        if self.cabinet.is_some() {
            return "cabinet";
        }
        match &self.play {
            Some(p) if p.after.is_some() => "after match",
            Some(p) if p.pause.is_some() => "pause",
            Some(p) => p.fight.name(),
            None => self.stack.last().map_or("attract", Screen::name),
        }
    }

    /// The active configuration with the play mode and the chosen fighters
    /// stamped on, as every drive starts from it.
    fn stamped(&self, mode: PlayMode, p1: TournamentFighter, p2: TournamentFighter) -> MatchConfig {
        let mut config = self.config;
        config.dramatic = None;
        config.world_tour = None;
        config.roster.mode = mode;
        config.roster.p1 = p1;
        config.roster.p2 = p2;
        // `FILL METERS` is Training's: it means nothing in any other fight.
        config.fill_meters = mode == PlayMode::Training && self.settings.fill_meters;
        // The picked stage reaches Local and Training under an overhaul
        // preset; Arcade and Faithful 1987 keep the 1987 tables.
        let picks = matches!(mode, PlayMode::Versus | PlayMode::Training) && config.preset != RulesetPreset::Faithful1987;
        let stage = self.stage.filter(|_| self.net_setup.is_none()).or(self.chosen);
        config.roster.stage = if picks { stage } else { None };
        config
    }

    /// Power on and drive a fight of `kind` for `p1` and `p2` (the
    /// opponent in Training; unused in Arcade). `rematch` is the flag of the
    /// fight being replayed. Training against a CPU under Faithful 1987
    /// picks the flag whose first opponent is `p2`, and Capcom's CPU plays
    /// him; under an overhaul preset it is a two-player fight whose Player 2
    /// pad the fair CPU plays (SP21), on the rematch's flag or Japan's,
    /// picked hidden as before. The others show the real country select.
    fn start_fight(&mut self, kind: FightKind, p1: TournamentFighter, p2: TournamentFighter, rematch: Option<u8>) {
        let fair_cpu = matches!(kind, FightKind::TrainingCpu | FightKind::Arcade) && self.config.preset != RulesetPreset::Faithful1987;
        let (players, mode) = match kind {
            FightKind::Local => (Players::Two, PlayMode::Versus),
            FightKind::Arcade => (Players::One, PlayMode::Arcade),
            FightKind::TrainingCpu if fair_cpu => (Players::Two, PlayMode::Training),
            FightKind::TrainingCpu => (Players::One, PlayMode::Training),
            FightKind::TrainingDummy => (Players::Two, PlayMode::Training),
            FightKind::Dramatic { .. } | FightKind::WorldTour => return,
        };
        let flag = match kind {
            FightKind::TrainingCpu if fair_cpu => Some(rematch.unwrap_or(HIDDEN_FLAG)),
            FightKind::TrainingCpu => first_opponent_flag(p2),
            _ => rematch,
        };
        let mut config = self.stamped(mode, p1, p2);
        config.p2_fair_cpu = fair_cpu;
        let setup = FightSetup {
            kind,
            players,
            config,
            dsw1: FightDips::DSW1,
            dsw2: self.fight_dips().dsw2(),
        };
        let styles = match self.stack.last() {
            Some(Screen::Select(screen)) => [screen.locked_style(0), screen.locked_style(1)],
            _ => [None; 2],
        };
        self.launch_fight(setup, flag, styles);
    }

    /// Open the configured private playtest at the Thailand final.
    pub fn start_evil_ryu_playtest(&mut self) {
        let Some(playtest) = self.config.evil_ryu_playtest else { return };
        let mut config = self.stamped(PlayMode::Arcade, TournamentFighter::Ryu, TournamentFighter::Sagat);
        config.p2_fair_cpu = true;
        config.gauge_mode = GaugeMode::EraDefault;
        if playtest == EvilRyuPlaytest::Sf6 {
            config.roster.what_if_styles = true;
        }
        let setup = FightSetup {
            kind: FightKind::Arcade,
            players: Players::One,
            config,
            dsw1: FightDips::DSW1,
            dsw2: self.fight_dips().dsw2(),
        };
        let style = match playtest {
            EvilRyuPlaytest::UltraSf2 => StylePick::sf2(Revision::St, Loadout::Standard, Costume::Classic1987, 0),
            EvilRyuPlaytest::Alpha2 => StylePick::for_revision(Revision::Sfz2, Loadout::Standard),
            EvilRyuPlaytest::Alpha3A => StylePick::for_revision(Revision::Sfz3, Loadout::AIsm),
            EvilRyuPlaytest::Alpha3V => StylePick::for_revision(Revision::Sfz3, Loadout::VIsm),
            EvilRyuPlaytest::Alpha3X => StylePick::for_revision(Revision::Sfz3, Loadout::XIsm),
            EvilRyuPlaytest::Ae2012 => StylePick::for_revision(Revision::Ae2012, Loadout::Ultra1),
            EvilRyuPlaytest::Ae2012Ultra2 => StylePick::for_revision(Revision::Ae2012, Loadout::Ultra2),
            EvilRyuPlaytest::Usf4 => StylePick::for_revision(Revision::Usf4, Loadout::UltraDouble),
            EvilRyuPlaytest::Kage => StylePick::for_revision(Revision::Sf5Ce, Loadout::Skill1Trigger1),
            EvilRyuPlaytest::KageII => StylePick::for_revision(Revision::Sf5Ce, Loadout::Skill2Trigger2),
            EvilRyuPlaytest::Sf6 => StylePick::for_revision(Revision::Sf6, Loadout::Standard),
        };
        self.launch_fight(setup, Some(HIDDEN_FLAG), [Some(style), None]);
    }

    pub(super) fn stamp_dramatic(
        &self,
        ally1: TournamentFighter,
        boss: TournamentFighter,
        rules: DramaticRules,
        stage_pick: Option<StagePick>,
    ) -> MatchConfig {
        let mut config = self.stamped(PlayMode::Versus, ally1, boss);
        config.roster.stage = stage_pick;
        config.p2_fair_cpu = false;
        config.dramatic = Some(rules);
        config
    }

    pub(super) fn start_dramatic(&mut self, config: MatchConfig) -> bool {
        let Some(rules) = config.dramatic else { return false };
        if config.preset == RulesetPreset::Faithful1987 || !rules.styles_available(&config) {
            self.notice = Some("DRAMATIC BATTLE STYLE IS UNAVAILABLE".into());
            return false;
        }
        let picks = [(0, config.roster.p1), (1, rules.ally2), (1, config.roster.p2)];
        if let Some((_, fighter)) = picks.into_iter().find(|&(side, fighter)| {
            !SelectScreen::fighter_available(&config, self.player_tables(), side, fighter)
        }) {
            let name = fighter.name().to_ascii_uppercase();
            self.notice = Some(if !fighter.is_shoto() && !self.player_tables().contains(&fighter) {
                format!("{name} NEEDS THE SF1+ ART PACK")
            } else {
                format!("{name} IS NOT AVAILABLE")
            });
            return false;
        }
        if self.net_dramatic_editing() {
            return self.net_dramatic_offer(config);
        }
        let dips = self.net_connection.as_ref().filter(|_| self.net_setup.is_some())
            .map_or_else(|| self.fight_dips(), |connection| connection.rules().dips);
        let setup = FightSetup {
            kind: FightKind::Dramatic { co_op: rules.co_op },
            players: Players::One,
            config,
            dsw1: FightDips::DSW1,
            dsw2: dips.dsw2(),
        };
        self.launch_fight(setup, Some(boot_flag(rules.stage)), [None; 2]);
        self.play.is_some()
    }

    fn launch_fight(&mut self, setup: FightSetup, flag: Option<u8>, styles: [Option<StylePick>; 2]) {
        let overhaul = setup.config.preset != RulesetPreset::Faithful1987;
        let backgrounds = self.backgrounds.clone().filter(|_| overhaul);
        let data = overhaul.then(|| {
            self.data_bank
                .clone()
                .unwrap_or_else(|| Arc::new(DataBank::build(&self.assets.maincpu, &[])))
        });
        match Fight::start_banked(&self.assets.maincpu, setup, flag, backgrounds, data) {
            Ok(mut fight) => {
                fight.set_satsui_art(self.satsui_art.clone());
                fight.set_selected_styles(styles);
                if setup.config.dramatic.is_some() && self.net_setup.is_some() {
                    if let Some(connection) = &self.net_connection {
                        fight.seed_dramatic_cpu(connection.rules().seed);
                    }
                }
                fight.set_gauge_art(self.settings.gauge_art);
                fight.set_hold_combo(self.settings.hold_combo);
                self.play = Some(Play {
                    fight,
                    world_tour: None,
                    netplay: self.net_setup.as_ref().and_then(|setup| setup.local_side
                        .map(|side| crate::netplay::RollbackEngine::new(side, setup.delay, setup.max_rollback))),
                    pause: None,
                    paused_by: 0,
                    moves_side: 0,
                    moves: None,
                    after: None,
                    last: Frame { argb: Vec::new() },
                    held: false,
                });
                self.net_fight_started();
            }
            Err(event) => self.abandon(event, "the hidden drive"),
        }
    }

    fn start_dramatic_setup(&mut self) {
        let Some(Screen::DramaticSetup(setup)) = self.stack.last() else { return };
        if let Some(preset) = setup.preset() {
            let stage = self.dramatic_stage_pick(preset.rules.stage);
            let config = self.stamp_dramatic(preset.ally1, preset.boss, preset.rules, stage);
            self.start_dramatic(config);
        } else {
            let remote_ally = if self.net_dramatic_editing() {
                let previous = self.config.dramatic.map_or(TournamentFighter::Ken, |rules| rules.ally2);
                let available = std::iter::once(previous).chain(TournamentFighter::ALL)
                    .find(|&fighter| SelectScreen::fighter_available(&self.config, self.player_tables(), 1, fighter));
                let Some(ally) = available else {
                    self.notice = Some("NO ALLY 2 FIGHTERS ARE AVAILABLE".into());
                    return;
                };
                Some(ally)
            } else {
                None
            };
            let picks = DramaticPicks::open(setup.clone(), &self.config, self.player_tables(), remote_ally);
            self.stack.push(Screen::DramaticPick(Box::new(picks)));
        }
    }

    fn dramatic_stage_pick(&self, stage: TournamentFighter) -> Option<StagePick> {
        let owner = stage.name();
        let listing = self.backgrounds.as_ref()?.listed_stages().iter()
            .find(|stage| stage.owner.eq_ignore_ascii_case(owner))?;
        Some(StagePick::Bank(listing.index))
    }

    /// The tuning screen for the look `look`: its parameters if the host has
    /// loaded them (else they arrive through [`Self::set_look_params`]) and
    /// the session's earlier tuning of it.
    fn tune_screen(&self, look: &str) -> TuneScreen {
        let name = self.looks.iter().find(|l| l.id == look).map_or_else(|| look.to_ascii_uppercase(), |l| l.name.clone());
        let params = self.look_params.as_ref().filter(|(id, _)| id == look).map(|(_, p)| p.clone()).unwrap_or_default();
        let tuning = self.tuning.as_ref().filter(|t| t.look == look).map_or(&[][..], |t| &t.values[..]);
        TuneScreen::open(look, &name, params, tuning).with_looks(&self.looks)
    }

    /// The Jukebox screen over the Settings screen `settings`: its tracks,
    /// every stage the stage select shows and every fighter (challengers of
    /// a released wave included, as the Roster tab's switch has it), and the
    /// files the host listed.
    fn jukebox_screen(&self, settings: &SettingsScreen) -> JukeboxScreen {
        let (stages, fighters) = jukebox_entries(self.backgrounds.as_deref(), settings.config().roster.bonus_challengers, RELEASED_WAVES);
        JukeboxScreen::open_with(settings.settings().jukebox.clone(), stages, fighters, self.tracks.clone(), self.track_formats.clone())
    }

    /// The look the tuning screen asked to save (SAVE AS), once: the host
    /// writes it, re-reads its catalogue ([`Self::set_looks`]) and answers
    /// with [`Self::look_saved`].
    pub fn take_save_look(&mut self) -> Option<SaveLook> {
        self.save_look.take()
    }

    /// Keys typed on the keyboard since the last step (the window's text,
    /// key repeats included). The next step gives them to an open text
    /// entry, and drops them otherwise.
    pub fn type_keys(&mut self, keys: impl IntoIterator<Item = TypedKey>) {
        self.typed.extend(keys);
    }

    /// The room code shown on the current host screen or selected in Browse.
    pub fn room_code_for_copy(&self) -> Option<&str> {
        match (&self.cabinet, &self.play, self.stack.last()) {
            (None, None, Some(Screen::Online(screen))) => screen.room_code_for_copy(),
            _ => None,
        }
    }

    /// Direct Join can receive a room code from the host clipboard.
    pub fn accepts_room_code_paste(&self) -> bool {
        matches!(
            (&self.cabinet, &self.play, self.stack.last()),
            (None, None, Some(Screen::Online(screen))) if screen.accepts_room_code_paste()
        )
    }

    /// Display the result of a copy on the current lobby screen.
    pub fn room_code_copy_result(&mut self, copied: bool) {
        if let (None, None, Some(Screen::Online(screen))) =
            (&self.cabinet, &self.play, self.stack.last_mut())
        {
            screen.set_notice(if copied { "ROOM CODE COPIED" } else { "CLIPBOARD UNAVAILABLE" });
        }
    }

    /// Validate and replace the Direct Join entry. Joining still needs Confirm.
    pub fn paste_room_code(&mut self, text: Option<&str>) {
        if let (None, None, Some(Screen::Online(screen))) =
            (&self.cabinet, &self.play, self.stack.last_mut())
        {
            screen.paste_room_code(text);
        }
    }

    /// The host supplies network results; the engine never opens a socket.
    pub fn push_net_events(&mut self, events: &[NetTransportEvent]) {
        let room = 256usize.saturating_sub(self.net_events.len());
        self.net_events.extend(events.iter().take(room).cloned());
    }

    /// Caller-owned monotonic time for RTT only; never changes fight timing.
    /// Hosts supply their clock and headless tests supply the virtual bus clock.
    pub fn set_net_time_ms(&mut self, milliseconds: u32) { self.net_time_ms = Some(milliseconds); }

    /// Outbound actions are consumed once by the host or VirtualNetBus.
    pub fn drain_net_commands(&mut self) -> Vec<NetTransportCommand> {
        std::mem::take(&mut self.net_commands)
    }

    fn receive_net_events(&mut self) {
        if let (Some(connection), Some(now)) = (self.net_connection.as_mut(), self.net_time_ms) {
            connection.set_time_ms(now);
        }
        while let Some(event) = self.net_events.pop_front() {
            if let Some(connection) = self.net_connection.as_mut() { connection.receive(&event, self.frames); }
            match event {
                NetTransportEvent::RoomsDiscovered { rooms } => {
                    if let Some(Screen::Online(screen)) = self.stack.last_mut() { screen.set_rooms(rooms); }
                }
                NetTransportEvent::Bound { room_code, .. }
                    if self.net_connection.as_ref().is_some_and(Connection::is_host) => {
                    if let Some(Screen::Online(screen)) = self.stack.last_mut() { screen.set_bound(room_code); }
                }
                NetTransportEvent::TransportError { message } => {
                    if let Some(Screen::Online(screen)) = self.stack.last_mut() { screen.set_notice(&message); }
                    self.notice = Some(message);
                }
                _ => {}
            }
        }
        if let Some(connection) = self.net_connection.as_mut() { connection.tick(self.frames); }
        self.connection_output();
    }

    fn connection_output(&mut self) {
        let Some(connection) = self.net_connection.as_mut() else { return; };
        self.net_commands.extend(connection.drain_commands());
        let events = connection.drain_events();
        for event in events {
            match event {
                ConnectionEvent::Ready { rules, local_side, delay, max_rollback, generation, frame_base, phase, picker } =>
                    self.net_ready(rules, local_side, delay, max_rollback, generation, frame_base, phase, picker),
                ConnectionEvent::Rejected { reason } => {
                    let notice = match reason {
                        RejectReason::WiredOnlyRequired => "WIRED ONLY REQUIRES ETHERNET",
                        RejectReason::PackHashMismatch => "DATA OR ART PACK MISMATCH",
                        RejectReason::RoomFull => "ROOM IS FULL",
                        RejectReason::VersionMismatch => "VERSION MISMATCH",
                        RejectReason::UnsupportedMode => "ROOM MODE IS UNAVAILABLE",
                        RejectReason::None => "CONNECTION REJECTED",
                    };
                    self.notice = Some(notice.into());
                    if let Some(Screen::Online(screen)) = self.stack.last_mut() { screen.set_notice(notice); }
                }
                ConnectionEvent::PeerLeft { peer, kind } => {
                    self.log.push(format!("netplay: peer {peer} left or timed out"));
                    if kind.is_combatant() { self.net_return_online("PEER LEFT ROOM"); }
                    // Spectator streams are pruned against Connection membership
                    // by net_spectator_tick; losing an observer never stops play.
                }
                ConnectionEvent::Packet { from, packet } => {
                    if !self.net_spectator_packet(from, &packet) { self.net_packet(from, packet); }
                }
                ConnectionEvent::SpectatorJoined { .. } => {}
            }
        }
    }

    fn online_rules(&mut self, kind: RoomKind) -> SyncedMatchRules {
        let data = self.data_bank.get_or_insert_with(|| Arc::new(DataBank::build(&self.assets.maincpu, &[])));
        let mut bank = CanonicalHasher::new();
        bank.tag("sf1-port/online-bank/v1");
        self.assets.maincpu.encode(&mut bank);
        data.encode_compatibility(&mut bank);
        self.backgrounds.as_ref().map(|backgrounds| backgrounds.compatibility_bytes()).encode(&mut bank);
        let mut art = CanonicalHasher::new();
        art.tag("sf1-port/online-art/v1");
        let assets = self.bank_assets.as_deref().unwrap_or(&self.assets);
        let Assets { maincpu: _, gfx1, gfx2, gfx3, gfx4, tilerom } = assets;
        for bytes in [gfx1, gfx2, gfx3, gfx4, tilerom] { bytes.encode(&mut art); }
        let stage = self.chosen.or_else(|| {
            self.backgrounds.as_ref()?.listed_stages().first().map(|stage| StagePick::Bank(stage.index))
        }).unwrap_or(StagePick::Bank(0));
        let (config, stage) = if kind == RoomKind::Dramatic {
            let initial = DramaticSetupScreen::for_config(true, &self.config).preset().expect("the default rivalry");
            let stage = self.dramatic_stage_pick(initial.rules.stage).unwrap_or(stage);
            (self.stamp_dramatic(initial.ally1, initial.boss, initial.rules, Some(stage)), stage)
        } else {
            let mut config = self.config;
            config.roster.mode = PlayMode::Versus;
            config.dramatic = None;
            config.p2_fair_cpu = false;
            config.fill_meters = false;
            (config, stage)
        };
        SyncedMatchRules::new(config, [config.roster.p1, config.roster.p2], self.fight_dips(),
            stage, u64::from(self.frames), bank.finish(), art.finish())
    }

    fn online_actions(&mut self) {
        let actions = match self.stack.last_mut() {
            Some(Screen::Online(screen)) => screen.drain_actions(),
            _ => return,
        };
        for action in actions {
            match action {
                OnlineAction::Refresh => self.net_commands.push(NetTransportCommand::RefreshRooms),
                OnlineAction::Host { port, public, wired_only, kind } => {
                    if kind == RoomKind::Dramatic && self.config.preset == RulesetPreset::Faithful1987 {
                        if let Some(Screen::Online(screen)) = self.stack.last_mut() {
                            screen.set_notice("DRAMATIC BATTLE REQUIRES SF1+");
                        }
                        continue;
                    }
                    let rules = self.online_rules(kind);
                    let room = RoomAnnouncement {
                        host: tag_from_str(&self.settings.player_tag).unwrap_or(*b"PLAYER\0\0\0\0\0\0"),
                        kind, link: Link::Wifi, wired_only, public,
                        preset: rules.config.preset, in_match: false,
                        max_rollback: self.settings.netplay_max_rollback.clamp(1, 8), spectators: 0,
                        config_hash: rules.config_hash(), bank_hash: rules.bank_hash,
                        art_pack_hash: rules.art_pack_hash, protocol_hash: rules.protocol_hash,
                    };
                    if self.net_host(rules, room.clone(), port) {
                        self.net_commands.push(NetTransportCommand::BindHost { port, room });
                    }
                }
                OnlineAction::Join { addr, kind } => {
                    let rules = self.online_rules(RoomKind::Versus);
                    self.net_request = self.net_request.wrapping_add(1).max(1);
                    let request = (u64::from(self.frames) << 32) ^ self.net_request;
                    let tag = tag_from_str(&self.settings.player_tag).unwrap_or(*b"PLAYER\0\0\0\0\0\0");
                    self.net_connection = Some(Connection::join(rules, tag, kind, request));
                    self.net_commands.push(NetTransportCommand::ConnectPeer { addr });
                }
            }
        }
    }

    /// The answer to a save: the new look's id, or why it failed. A saved
    /// look becomes `CRT LOOK`, the tuning screen goes on to it (its
    /// parameters arrive when the host has loaded it), and the changes are
    /// no longer the old look's: they are the new look's own values.
    pub fn look_saved(&mut self, saved: Result<String, String>) {
        let Some(Screen::Tune(t)) = self.stack.last_mut() else { return };
        let id = match saved {
            Ok(id) => id,
            Err(why) => {
                t.save_failed(&why);
                return;
            }
        };
        let old = t.look().0.to_owned();
        if self.tuning.as_ref().is_some_and(|tuning| tuning.look == old) {
            self.tuning = None;
        }
        for screen in &mut self.stack {
            if let Screen::Settings(s) = screen {
                s.set_look(&id);
            }
        }
        let name = self.looks.iter().find(|l| l.id == id).map_or_else(|| id.to_ascii_uppercase(), |l| l.name.clone());
        let tune = self.tune_screen(&id).with_message(&format!("SAVED: {name}"));
        if let Some(top) = self.stack.last_mut() {
            *top = Screen::Tune(Box::new(tune));
        }
        self.sounds.push_back(sound::CONFIRM);
    }

    /// The stage screen the locked fighters lead to: under an overhaul
    /// preset, for Local and Training, when the background bank lists
    /// stages and no stage was set for the session. Otherwise the fight
    /// starts at once (Faithful 1987 shows the real flag screen). The
    /// picker is the session's, except in Training, where the one player
    /// is Player 1; `played` is a rematch's stage, where the cursor starts
    /// instead of the opponent's home stage.
    fn stage_screen(
        &self,
        kind: FightKind,
        p1: TournamentFighter,
        p2: TournamentFighter,
        played: Option<StagePick>,
    ) -> Option<StageScreen> {
        if kind == FightKind::Arcade || self.config.preset == RulesetPreset::Faithful1987
            || (self.stage.is_some() && self.net_setup.is_none() && !matches!(kind, FightKind::Dramatic { .. })) {
            return None;
        }
        let picker = if kind == FightKind::Local { self.picker } else { 0 };
        let bank = self.backgrounds.as_deref()?;
        let shown = shown_stages(bank.listed_stages(), self.config.roster.bonus_challengers, RELEASED_WAVES);
        let screen = StageScreen::open(&shown, kind, p1, p2, picker, played)?;
        let previews = self.previews.get_or_init(|| {
            let assets = self.bank_assets.as_deref().unwrap_or(&self.assets);
            Arc::new(bank.listed_previews(assets))
        });
        Some(screen.with_previews(Arc::clone(previews)))
    }

    /// A stage among those the screen lists, for its `RANDOM` tile. Chosen
    /// as the fight starts, from the frame count, so it is not known to the
    /// players or the screen before.
    fn random_stage(&self, screen: &StageScreen) -> StagePick {
        random_pick(screen.stages(), self.frames)
    }

    /// The select a fight of `kind` was chosen on.
    fn select_for(&mut self, kind: FightKind) -> SelectScreen {
        if self.config.preset != RulesetPreset::Faithful1987 {
            let tables = self.player_tables();
            let missing: Vec<&str> = TournamentFighter::ALL
                .iter()
                .filter(|f| !f.is_shoto() && !tables.contains(f))
                .map(|f| f.name())
                .collect();
            if !missing.is_empty() {
                self.log.push(format!(
                    "select: {} greyed out: their 1987 player tables need the art pack's poses",
                    missing.join(", "),
                ));
            }
        }
        let select = match kind {
            FightKind::Local => SelectScreen::local(&self.config, self.player_tables()),
            FightKind::Arcade => SelectScreen::arcade(&self.config, self.player_tables()),
            FightKind::Dramatic { .. } | FightKind::WorldTour => SelectScreen::single(&self.config, self.player_tables(), 0),
            FightKind::TrainingCpu => SelectScreen::training(&self.config, true, self.player_tables()),
            FightKind::TrainingDummy => SelectScreen::training(&self.config, false, self.player_tables()),
        };
        select.with_preferred_eras(self.preferred_eras)
    }

    /// Arcade's game over returned the ROM to its attract: that machine
    /// becomes the attract (its ranking table, with any new entry, lives
    /// on for the session, as on a cabinet left switched on), every toggle
    /// off, and the Main Menu shows.
    fn arcade_over(&mut self) {
        if let Some(play) = self.play.take() {
            let mut machine = play.fight.into_machine();
            machine.set_match_config(MatchConfig::faithful());
            self.attract = machine;
        }
        self.stack.truncate(1);
    }

    /// A fight whose stage plays a theme from the extended tune table is
    /// left for a menu: stop the music, as nothing else would, so the theme
    /// does not play into the menus (and the music CPU's table, which it
    /// holds on to while a theme plays, can be taken away).
    fn stop_extended_theme(&mut self, config: &MatchConfig) {
        let stage_extended = match config.roster.stage {
            Some(StagePick::Bank(index)) => self
                .backgrounds
                .as_ref()
                .is_some_and(|b| b.plays_extended_theme(index)),
            None => false,
        };
        let ryu_extended = self.play.as_ref().is_some_and(|p| {
            let m = p.fight.machine();
            m.arcade_opponent() == Some(TournamentFighter::Ryu)
                && self
                    .backgrounds
                    .as_ref()
                    .is_some_and(|b| b.can_play(sf_video::background_bank::RYU_THEME))
        });
        if stage_extended || ryu_extended {
            self.sounds.push_back(sound::STOP_ALL);
        }
    }

    /// The Jukebox's cue for a frame whose latch is `latch` (ticket #200).
    ///
    /// Inside a fight under an overhaul preset, a new command that is the
    /// fight's stage theme starts the track the Jukebox gives the fight, and
    /// one that ends the driver's music (the stops, the win jingle) stops
    /// it; leaving the fight stops it too ([`Cues::step`]).
    fn track_cue(&mut self, latch: u8) -> Option<TrackCue> {
        let fighting = self.play.as_ref().is_some_and(|p| p.fight.setup().config.preset != RulesetPreset::Faithful1987);
        let mut cues = std::mem::take(&mut self.track_cues);
        let cue = cues.step(latch, fighting, |command| self.fight_track(command));
        self.track_cues = cues;
        cue
    }

    /// The track for the running fight if `latch` is its stage theme (with
    /// no latch, the track it has now, for [`TrackCue::Prepare`]).
    ///
    /// The stage is the picked one (Local and Training) or, in Arcade, the
    /// one the ladder is at: the start country and the stage and fight
    /// counters in work RAM name the opponent (`rom_stages::ladder_opponent`;
    /// read only, as ADR-0004 asks), known well before his theme. The theme
    /// must be that stage's (or Ryu's theme when Ryu is the CPU opponent in
    /// Arcade, SP15 `#267`). The fighters are Player 1's and Player 2's,
    /// with the stage picker's first (ST2's rule: Player 1 on the first
    /// match, the loser after; Arcade has Player 1's, and Ryu when Ryu is
    /// the CPU opponent).
    fn fight_track(&self, latch: Option<u8>) -> Option<Resolved> {
        if self.settings.jukebox.is_empty() {
            return None;
        }
        let play = self.play.as_ref()?;
        let setup = play.fight.setup();
        let arcade_ryu = play.fight.machine().arcade_opponent() == Some(TournamentFighter::Ryu);
        let stage_info = self.fight_stage_info(&setup);
        if let Some(command) = latch {
            let (_, theme) = stage_info.as_ref()?;
            let is_theme = *theme == u16::from(command)
                || (arcade_ryu && command == sf_video::background_bank::RYU_THEME);
            is_theme.then_some(())?;
        }
        let stage = stage_info.as_ref().map(|(name, _)| name.as_str());
        let (p1, p2) = (setup.config.roster.p1.name(), setup.config.roster.p2.name());
        let (picker, other) = match setup.kind {
            FightKind::Arcade => (p1, arcade_ryu.then_some("Ryu")),
            FightKind::Local if self.picker == 1 => (p2, Some(p1)),
            _ => (p1, Some(p2)),
        };
        self.settings.jukebox.resolve(stage, Some(picker), other)
    }

    /// The stage name and its 1987/bank theme command for the running fight.
    fn fight_stage_info(&self, setup: &FightSetup) -> Option<(String, u16)> {
        match setup.config.roster.stage {
            Some(StagePick::Bank(index)) => {
                let bank = self.backgrounds.as_deref()?;
                let name = bank.stage_names().nth(usize::from(index))?.to_owned();
                let theme = bank.stage(index)?.theme;
                Some((name, theme))
            }
            None if setup.kind == FightKind::Arcade => {
                let machine = self.play.as_ref()?.fight.machine();
                let ram = machine.work_ram();
                let country = usize::from(ram[(super::cabinet::COUNTRY - 0xff8000) as usize]);
                let step = machine.arcade_ladder_step();
                let name = sf_video::rom_stages::ladder_stage_name(&self.assets.maincpu, country, step).ok()?;
                if let Some(bank) = self.backgrounds.as_deref() {
                    if let Some(index) = bank.stage_index(&name) {
                        if let Some(stage) = bank.stage(index) {
                            return Some((name, stage.theme));
                        }
                    }
                }
                let rom = &self.assets.maincpu;
                let ptr_at = 0x42358usize.checked_add(country.checked_mul(4)?)?;
                let table = u32::from_be_bytes(rom.get(ptr_at..ptr_at + 4)?.try_into().ok()?) as usize;
                let cmd_at = table.checked_add(step.checked_mul(2)?)?;
                let theme = u16::from_be_bytes(rom.get(cmd_at..cmd_at + 2)?.try_into().ok()?);
                Some((name, theme))
            }
            None => None,
        }
    }

    /// An untranslated branch (or a drive that never arrived): log it,
    /// leave the fight and show the notice on the Main Menu.
    fn abandon(&mut self, event: FightEvent, during: &str) {
        let FightEvent::Untranslated { frame, addresses } = event else {
            return;
        };
        let list: Vec<String> = addresses.iter().map(|a| format!("0x{a:05x}")).collect();
        let on_tour = matches!(self.stack.last(), Some(Screen::WorldTour(_)));
        let destination = if on_tour { "World Tour" } else { "the Main Menu" };
        self.log.push(format!(
            "frame {frame}: untranslated original branches [{}] during {during}, back to {destination}",
            list.join(", ")
        ));
        self.notice = Some(format!(
            "UNTRANSLATED BRANCH {:05X}",
            addresses.first().copied().unwrap_or(0)
        ));
        if let Some(play) = &self.play {
            let config = play.fight.setup().config;
            self.stop_extended_theme(&config);
        }
        self.play = None;
        if !on_tour {
            self.stack.truncate(1);
        }
    }

    /// Inputs while a fight is up: the after-match box's, the pause box's,
    /// or the keys that open the pause box (keyboard Start or Esc, either
    /// player's Start; not during the hidden drive, nor on Arcade's
    /// continue countdown, where Start continues).
    fn play_input(&mut self, prev: &FrameInputs, now: &FrameInputs, events: &[MenuInput]) -> Option<Transition> {
        self.play.as_ref()?;
        let online = self.net_setup.is_some();
        let local_side = self.net_setup.as_ref().and_then(|setup| setup.local_side).unwrap_or(0);
        let before = if online { netplay::local_inputs(*prev) } else { *prev };
        let input = if online { netplay::local_inputs(*now) } else { *now };
        let online_events = online.then(|| menu_events(&before, &input));
        let events = online_events.as_deref().unwrap_or(events);
        let (prev, now) = (&before, &input);
        let play = self.play.as_mut()?;
        if play.world_tour.is_some_and(|run| run.ending) {
            return None;
        }
        let training = matches!(play.fight.setup().kind, FightKind::TrainingCpu | FightKind::TrainingDummy);
        let can_pause = !play.fight.driving() && !play.fight.in_continue();
        // A lost pad pauses the fight with a notice, at once or as soon as
        // the fight can pause; a box already up takes the notice.
        if let Some(side) = (0..2).find(|&s| now.pad_lost[s]).or(self.lost_pad) {
            let notice = if online { "CONTROLLER DISCONNECTED".into() }
                else { format!("P{} PAD DISCONNECTED", side + 1) };
            self.lost_pad = None;
            if let Some(pause) = play.pause.as_mut() {
                pause.set_notice(notice);
                // The notice is on the pause box: the list gives way to it.
                play.moves = None;
            } else if play.after.is_none() {
                if !can_pause {
                    self.lost_pad = Some(side);
                } else {
                    play.paused_by = if online { local_side } else { 0 };
                    play.fight.stop_try();
                    play.pause = Some(if online { PauseBox::with_online_notice(notice) }
                        else { PauseBox::with_notice(training, notice) });
                    return Some(Transition::Paused);
                }
            }
        }
        if play.after.is_some() {
            return self.after_match_input(events);
        }
        if play.pause.is_some() {
            return self.pause_input(events);
        }
        // A pad's back (Circle) is MK in a fight: only its Start opens the
        // box, and on the keyboard Start, Esc and Backspace.
        let p1_start = edge(prev.menu.start, now.menu.start)
            || pad_edge(prev, now, 0, |m| m.start)
            || edge(prev.cabinet[2], now.cabinet[2]);
        let other_pause = edge(prev.menu.back, now.menu.back)
            || pad_edge(prev, now, 1, |m| m.start)
            || edge(prev.cabinet[3], now.cabinet[3]);
        if can_pause && (p1_start || other_pause) {
            if !other_pause && play.fight.try_alpha2_feint(now.p1) {
                return None;
            }
            // The side whose pad's Start did it; a key is P1's.
            play.paused_by = if online { local_side }
                else { (0..2).find(|&s| pad_edge(prev, now, s, |m| m.start)).unwrap_or(0) };
            play.fight.stop_try();
            play.pause = Some(if online { PauseBox::open_online() } else { PauseBox::open(training) });
            return Some(Transition::Paused);
        }
        None
    }

    /// The pause box's inputs.
    fn pause_input(&mut self, events: &[MenuInput]) -> Option<Transition> {
        for &event in events {
            let play = self.play.as_mut()?;
            // The move list over the box takes the inputs; back returns to it.
            if let Some(moves) = play.moves.as_mut() {
                // With two human pads, either player's list is available.
                if matches!(event, MenuInput::TabLeft | MenuInput::TabRight) {
                    let setup = play.fight.setup();
                    if two_human_move_lists(setup.kind) {
                        play.moves_side = 1 - play.moves_side;
                        *moves = side_moves(&play.fight, play.moves_side, self.settings.move_inputs);
                        self.sounds.push_back(sound::CURSOR);
                        return Some(Transition::CursorMoved);
                    }
                    continue;
                }
                match moves.apply(event) {
                    Some(Transition::CursorMoved) => self.sounds.push_back(sound::CURSOR),
                    Some(Transition::Back) => {
                        play.moves = None;
                        self.sounds.push_back(sound::CONFIRM);
                        return Some(Transition::Back);
                    }
                    // Start resumes the fight straight from the list, and so
                    // does Show, which has the fighter perform the move.
                    Some(Transition::Choose(MenuItem::Resume)) => {
                        // Only the player's own side is shown (never a Local P2's list).
                        let (shown, tried) = (moves.take_show(), moves.take_try());
                        // The coach's fixes are written the way the list was.
                        play.fight.set_move_inputs(self.settings.move_inputs);
                        if let Some(entry) = shown.filter(|_| play.moves_side == 0) {
                            play.fight.show(entry);
                        }
                        if let Some(entry) = tried.filter(|_| play.moves_side == 0) {
                            play.fight.try_move(entry);
                        }
                        play.moves = None;
                        play.pause = None;
                        play.held = self.net_setup.is_none();
                        self.sounds.push_back(sound::CONFIRM);
                        return Some(Transition::Choose(MenuItem::Resume));
                    }
                    _ => {}
                }
                continue;
            }
            let Some(t) = play.pause.as_mut()?.apply(event) else {
                continue;
            };
            match t {
                Transition::CursorMoved => self.sounds.push_back(sound::CURSOR),
                // Back out of the Training settings box, to the pause box.
                Transition::Back => {
                    self.sounds.push_back(sound::CONFIRM);
                    return Some(t);
                }
                Transition::Choose(item) => {
                    self.sounds.push_back(sound::CONFIRM);
                    match item {
                        MenuItem::Resume => {
                            play.pause = None;
                            play.held = self.net_setup.is_none();
                        }
                        MenuItem::Overlays => {
                            let next = play.fight.overlay().next();
                            play.fight.set_overlay(next);
                            let assets = fight_assets(&self.assets, &self.bank_assets, play.fight.setup().config.preset);
                            play.last = play.fight.render(assets);
                        }
                        MenuItem::Look => {
                            // The player's look: the Settings value changes with
                            // it and is saved. The look is the host's to draw, so
                            // the paused frame itself does not change.
                            let n = self.looks.len() as i32;
                            let look = self.look().to_owned();
                            let at = self.looks.iter().position(|l| l.id == look).map_or(0, |i| i as i32);
                            let delta = if event == MenuInput::Left { -1 } else { 1 };
                            self.settings.crt_look = self.looks[(at + delta).rem_euclid(n) as usize].id.clone();
                            self.failed_look = None;
                            self.settings_changed = true;
                        }
                        MenuItem::GaugeArt => {
                            // The fight and the Settings value change together.
                            let art = match event {
                                MenuInput::Left => play.fight.gauge_art().prev(),
                                _ => play.fight.gauge_art().next(),
                            };
                            play.fight.set_gauge_art(art);
                            self.settings.gauge_art = art;
                            self.settings_changed = true;
                            // The paused fight behind the box shows the new art.
                            let assets = fight_assets(&self.assets, &self.bank_assets, play.fight.setup().config.preset);
                            play.last = play.fight.render(assets);
                        }
                        MenuItem::RestartRound | MenuItem::ResetPositions => {
                            // Training's reset keeps the settings the fight has now.
                            let done = if item == MenuItem::ResetPositions {
                                play.fight.reset_positions()
                            } else {
                                play.fight.restart_round()
                            };
                            if done {
                                play.pause = None;
                                play.held = true;
                                let assets = fight_assets(&self.assets, &self.bank_assets, play.fight.setup().config.preset);
                                play.last = play.fight.render(assets);
                            }
                        }
                        // The pause box opened the settings box itself.
                        MenuItem::TrainingSettings => {}
                        MenuItem::LivePreset => {
                            let next = next_live_preset(play.fight.setup().config.preset);
                            play.fight.set_preset(next);
                        }
                        MenuItem::Moves => {
                            // Local opens for the side whose pad paused; every
                            // other fight has one player, P1. Its inputs are
                            // written in the chosen notation, the arrows for the
                            // way that fighter faces now (the fight is frozen
                            // behind the list).
                            let setup = play.fight.setup();
                            play.moves_side = if two_human_move_lists(setup.kind) { play.paused_by } else { 0 };
                            play.moves = Some(side_moves(&play.fight, play.moves_side, self.settings.move_inputs));
                        }
                        MenuItem::HoldCombo => {
                            // Saved with the settings, like the menus' rows.
                            self.settings.hold_combo = !self.settings.hold_combo;
                            play.fight.set_hold_combo(self.settings.hold_combo);
                            self.settings_changed = true;
                        }
                        MenuItem::FillMeters => {
                            // Saved with the settings; the machine carries it too.
                            self.settings.fill_meters = !self.settings.fill_meters;
                            play.fight.set_fill_meters(self.settings.fill_meters);
                            self.settings_changed = true;
                        }
                        MenuItem::QuitToMenu if self.net_setup.is_some() => {
                            self.net_quit();
                        }
                        MenuItem::QuitToMenu if play.world_tour.is_some() => {
                            self.leave_world_tour_fight();
                        }
                        _ => {
                            let config = play.fight.setup().config;
                            self.stop_extended_theme(&config);
                            self.play = None;
                            self.stack.truncate(1);
                        }
                    }
                    return Some(t);
                }
                _ => {}
            }
        }
        None
    }

    /// The after-match box's inputs.
    fn after_match_input(&mut self, events: &[MenuInput]) -> Option<Transition> {
        if self.net_setup.is_some() {
            return self.net_after_match_input(events);
        }
        for &event in events {
            let play = self.play.as_mut()?;
            let Some(t) = play.after.as_mut()?.apply(event) else {
                continue;
            };
            match t {
                Transition::CursorMoved => self.sounds.push_back(sound::CURSOR),
                Transition::Choose(item) => {
                    self.sounds.push_back(sound::CONFIRM);
                    let setup = *play.fight.setup();
                    let country = play.fight.country();
                    if item != MenuItem::Rematch {
                        self.stop_extended_theme(&setup.config);
                    }
                    self.play = None;
                    match item {
                        MenuItem::Rematch if setup.config.dramatic.is_some() => {
                            self.start_dramatic(setup.config);
                        }
                        MenuItem::Rematch if setup.config.evil_ryu_playtest.is_some() => self.start_evil_ryu_playtest(),
                        MenuItem::Rematch => {
                            // The stage screen opens with the loser in
                            // control and the cursor on the stage just
                            // played, so one press keeps it.
                            let (p1, p2) = (setup.config.roster.p1, setup.config.roster.p2);
                            match self.stage_screen(setup.kind, p1, p2, self.chosen) {
                                Some(screen) => self.stack.push(Screen::Stage(screen)),
                                None => self.start_fight(setup.kind, p1, p2, Some(country.unwrap_or(0))),
                            }
                        }
                        MenuItem::Reselect => {
                            if let Some(rules) = setup.config.dramatic {
                                if let Some(at) = self.stack.iter().rposition(|screen| matches!(screen, Screen::DramaticSetup(_))) {
                                    self.stack.truncate(at);
                                }
                                self.stack.push(Screen::DramaticSetup(DramaticSetupScreen::from_rules(rules, &setup.config)));
                            } else {
                                let select = self.select_for(setup.kind);
                                if let Some(top) = self.stack.last_mut() {
                                    *top = Screen::Select(select);
                                }
                            }
                        }
                        _ => self.stack.truncate(1),
                    }
                    return Some(t);
                }
                _ => {}
            }
        }
        None
    }

    /// The menus' and the select's inputs. The Local select takes each
    /// side's own inputs; everything else takes the merged cursor's.
    fn screen_input(&mut self, prev: &FrameInputs, now: &FrameInputs, typed: Vec<TypedKey>, quit: &mut bool) -> Option<Transition> {
        let dramatic_host = self.net_dramatic_editing();
        if self.net_dramatic_waiting() || matches!(self.stack.last(), Some(Screen::DramaticGuest(_)))
            || (self.net_setup.is_some() && !dramatic_host
                && matches!(self.stack.last(), Some(Screen::Select(_) | Screen::Stage(_)))) {
            return self.net_screen_input(prev, now);
        }
        if let Some(Screen::Rebind(walk)) = self.stack.last_mut() {
            let t = walk.press(now.press?);
            match t {
                Some(Transition::Back) => {
                    let binds = *walk.binds();
                    self.stack.pop();
                    if let Some((current, _)) = self.binds.as_mut() {
                        *current = binds;
                    }
                    self.bindings_changed = true;
                    self.sounds.push_back(sound::CONFIRM);
                }
                Some(_) => self.sounds.push_back(sound::CONFIRM),
                None => {}
            }
            return t;
        }
        let mut transition = None;
        // An open name entry takes the keyboard as typed; the keyboard's
        // menu keys and fighter keys stand down, and a pad still steps it.
        let typing = match self.stack.last() {
            Some(Screen::Tune(screen)) => screen.typing(),
            Some(Screen::Settings(screen)) => screen.typing(),
            Some(Screen::Online(screen)) => screen.typing(),
            _ => false,
        };
        let sided: Vec<(usize, Event)> = if typing {
            let pads = pad_menu_events(prev, now).into_iter().map(Event::Menu);
            pads.chain(typed.into_iter().map(Event::Typed)).map(|e| (0, e)).collect()
        } else if dramatic_host {
            let before = netplay::local_inputs(*prev);
            let input = netplay::local_inputs(*now);
            menu_events(&before, &input).into_iter().map(|event| (0, Event::Menu(event))).collect()
        } else { match self.stack.last() {
            Some(Screen::Select(s)) if s.two_players() => {
                side_events(prev, now).into_iter().map(|(side, e)| (side, Event::Menu(e))).collect()
            }
            Some(Screen::Stage(s)) if s.two_players() => {
                side_events(prev, now).into_iter().map(|(side, e)| (side, Event::Menu(e))).collect()
            }
            Some(Screen::DramaticPick(s)) if s.co_op() => {
                side_events(prev, now).into_iter().map(|(side, e)| (side, Event::Menu(e))).collect()
            }
            _ => menu_events(prev, now).into_iter().map(|e| (0, Event::Menu(e))).collect(),
        }};
        for (side, event) in sided {
            let Some(top) = self.stack.last_mut() else {
                break;
            };
            let applied = match (top, event) {
                (Screen::Select(s), Event::Menu(event)) if s.two_players() => s.apply_side(side, event),
                (Screen::Stage(s), Event::Menu(event)) if s.two_players() => s.apply_side(side, event),
                (Screen::DramaticPick(s), Event::Menu(event)) if s.co_op() => {
                    (s.picker() == side || event == MenuInput::Back).then(|| s.apply(event)).flatten()
                }
                (Screen::Tune(t), Event::Typed(key)) => t.type_key(key),
                (Screen::Settings(screen), Event::Typed(key)) => screen.type_key(key),
                (Screen::Online(screen), Event::Typed(key)) => screen.type_key(key),
                (Screen::WorldTour(screen), Event::Menu(event)) => {
                    let (transition, changed) = screen.apply(event, &mut self.world_tour_profile);
                    self.world_tour_changed |= changed;
                    transition
                }
                (_, Event::Typed(_)) => None,
                (other, Event::Menu(event)) => other.apply(event),
            };
            let Some(t) = applied else {
                continue;
            };
            self.notice = None;
            transition = Some(t);
            // A menu remembers its cursor (saved with the settings).
            if let Some(Screen::Menu(m)) = self.stack.last() {
                if let Some((row, _)) = m.cursor() {
                    self.settings.menu_positions[m.slot()] = row;
                }
            }
            let kind = match self.stack.last() {
                Some(Screen::Select(s)) => {
                    self.preferred_eras = s.preferred_eras();
                    s.kind()
                }
                _ => FightKind::Local,
            };
            match t {
                Transition::CursorMoved => self.sounds.push_back(sound::CURSOR),
                Transition::StyleOpened | Transition::Picked(_) => self.sounds.push_back(sound::CONFIRM),
                Transition::Unpicked => {}
                Transition::Fighters(p1, p2) => {
                    self.sounds.push_back(sound::CONFIRM);
                    match self.stage_screen(kind, p1, p2, None) {
                        Some(screen) => self.stack.push(Screen::Stage(screen)),
                        None => self.start_fight(kind, p1, p2, None),
                    }
                    break;
                }
                Transition::Stage(_) | Transition::RandomStage => {
                    self.sounds.push_back(sound::CONFIRM);
                    if let Some(Screen::Stage(screen)) = self.stack.pop() {
                        let (kind, p1, p2) = screen.fight();
                        let stage = match t {
                            Transition::Stage(pick) => pick,
                            _ => self.random_stage(&screen),
                        };
                        if matches!(kind, FightKind::Dramatic { .. }) {
                            let owner = screen.owner(stage).unwrap_or(p2);
                            if let Some(Screen::DramaticPick(picks)) = self.stack.last() {
                                if let Some(matchup) = picks.match_on(owner) {
                                    let config = self.stamp_dramatic(matchup.ally1, matchup.boss, matchup.rules, Some(stage));
                                    self.start_dramatic(config);
                                }
                            }
                        } else {
                            self.chosen = Some(stage);
                            // The cabinet driver still picks a flag, hidden, so
                            // the 1987 flow runs as verified (ADR-0004).
                            self.start_fight(kind, p1, p2, Some(HIDDEN_FLAG));
                        }
                    }
                    break;
                }
                Transition::Fighter(p1) => {
                    self.sounds.push_back(sound::CONFIRM);
                    if let Some(Screen::DramaticPick(picks)) = self.stack.last_mut() {
                        if let Some(matchup) = picks.pick(p1) {
                            let kind = FightKind::Dramatic { co_op: matchup.rules.co_op };
                            match self.stage_screen(kind, matchup.ally1, matchup.boss, None) {
                                Some(screen) => self.stack.push(Screen::Stage(screen)),
                                None => self.notice = Some("STAGE DATA IS UNAVAILABLE".into()),
                            }
                        }
                    } else {
                        self.start_fight(FightKind::Arcade, p1, self.config.roster.p2, None);
                    }
                    break;
                }
                Transition::StartDramatic => {
                    self.sounds.push_back(sound::CONFIRM);
                    self.start_dramatic_setup();
                    break;
                }
                Transition::StartWorldTour => {
                    self.sounds.push_back(sound::CONFIRM);
                    self.start_world_tour_node();
                    break;
                }
                Transition::WorldTourSelect => {
                    let config = self.world_tour_config();
                    let player_tables = self.data_bank.as_deref().map_or(&[][..], DataBank::fighters);
                    if let Some(Screen::WorldTour(screen)) = self.stack.last_mut() {
                        screen.open_select(&config, player_tables, &self.world_tour_profile);
                        self.sounds.push_back(sound::CONFIRM);
                    }
                    break;
                }
                Transition::Choose(item) => {
                    self.sounds.push_back(sound::CONFIRM);
                    let next = match item {
                        MenuItem::SinglePlayer => Some(self.menu(MenuScreen::single_player())),
                        MenuItem::MultiPlayer => Some(self.menu(MenuScreen::multi_player())),
                        MenuItem::Training => Some(self.menu(MenuScreen::training())),
                        MenuItem::WorldTour => Some(Screen::WorldTour(Box::new(WorldTourScreen::open(&self.assets)))),
                        MenuItem::DramaticBattle | MenuItem::LocalCoopDramatic => {
                            let co_op = item == MenuItem::LocalCoopDramatic;
                            Some(Screen::DramaticSetup(DramaticSetupScreen::for_config(co_op, &self.config)))
                        }
                        MenuItem::Settings => Some(Screen::Settings(Box::new(
                            {
                                let mut screen = SettingsScreen::open(self.config, self.settings.clone())
                                    .with_looks(self.looks.clone())
                                    .with_failed_look(self.failed_look.clone());
                                screen.set_display(self.display);
                                screen
                            },
                        ))),
                        MenuItem::RebindP1 | MenuItem::RebindP2 | MenuItem::RebindMenu => {
                            let walk = match item {
                                MenuItem::RebindP1 => Walk::P1,
                                MenuItem::RebindP2 => Walk::P2,
                                _ => Walk::Menu,
                            };
                            self.binds
                                .map(|(binds, _)| Screen::Rebind(Box::new(RebindScreen::open(walk, binds))))
                        }
                        MenuItem::Cabinet1987 => Some(Screen::Dip(DipScreen::open(
                            self.settings.cabinet_dsw1,
                            self.settings.cabinet_dsw2,
                        ))),
                        MenuItem::PowerOn => {
                            if let Some(Screen::Dip(d)) = self.stack.last() {
                                let (dsw1, dsw2) = d.dsw();
                                (self.settings.cabinet_dsw1, self.settings.cabinet_dsw2) = (dsw1, dsw2);
                                self.settings_changed = true;
                                self.sounds.clear();
                                self.cabinet = Some(Cabinet::power_on(&self.assets.maincpu, dsw1, dsw2));
                            }
                            None
                        }
                        MenuItem::TuneLook => self.stack.last().and_then(|top| match top {
                            Screen::Settings(s) => Some(Screen::Tune(Box::new(self.tune_screen(&s.settings().crt_look)))),
                            _ => None,
                        }),
                        MenuItem::Jukebox => self.stack.last().and_then(|top| match top {
                            Screen::Settings(s) => Some(Screen::Jukebox(Box::new(self.jukebox_screen(s)))),
                            _ => None,
                        }),
                        MenuItem::Local => Some(Screen::Select(self.select_for(FightKind::Local))),
                        MenuItem::Online => {
                            self.net_commands.push(NetTransportCommand::RefreshRooms);
                            Some(Screen::Online(Box::new(OnlineScreen::open(self.settings.wired_only, self.settings.netplay_port))))
                        }
                        MenuItem::Arcade => Some(Screen::Select(self.select_for(FightKind::Arcade))),
                        MenuItem::TrainingCpu => Some(Screen::Select(self.select_for(FightKind::TrainingCpu))),
                        MenuItem::TrainingDummy => Some(Screen::Select(self.select_for(FightKind::TrainingDummy))),
                        MenuItem::Exit => {
                            *quit = true;
                            None
                        }
                        _ => None,
                    };
                    self.stack.extend(next);
                    break;
                }
                Transition::Back => {
                    if self.net_dramatic_editing() && matches!(self.stack.last(), Some(Screen::DramaticSetup(_))) {
                        self.net_return_online("ROOM CLOSED");
                        break;
                    }
                    if let Some(Screen::DramaticPick(picks)) = self.stack.last_mut() {
                        if picks.back() {
                            self.sounds.push_back(sound::CONFIRM);
                            break;
                        }
                    }
                    // Leaving Settings keeps what it changed: the session
                    // takes its copies, and the files are saved.
                    match self.stack.pop() {
                        Some(Screen::Online(_)) => {
                            self.net_commands.push(NetTransportCommand::Close);
                            self.net_connection = None;
                            self.net_setup = None;
                            if let Some(config) = self.net_saved_config.take() { self.config = config; }
                        }
                        Some(Screen::Settings(s)) => {
                            self.config = *s.config();
                            self.settings = s.settings().clone();
                            self.failed_look = s.failed_look().map(str::to_owned);
                            self.settings_changed = true;
                        }
                        Some(Screen::Dip(d)) => {
                            (self.settings.cabinet_dsw1, self.settings.cabinet_dsw2) = d.dsw();
                            self.settings_changed = true;
                        }
                        // The tracks go back to the Settings screen under it,
                        // which saves them when it is left.
                        Some(Screen::Jukebox(j)) => {
                            if let Some(Screen::Settings(s)) = self.stack.last_mut() {
                                s.set_jukebox(j.jukebox().clone());
                            }
                        }
                        _ => {}
                    }
                    break;
                }
                Transition::ValueChanged => {
                    self.sounds.push_back(sound::CURSOR);
                    if let Some(Screen::Online(screen)) = self.stack.last() {
                        if self.settings.wired_only != screen.wired_only() {
                            self.settings.wired_only = screen.wired_only();
                            self.settings_changed = true;
                        }
                    }
                    // A knob on the tuning screen: the session keeps it.
                    if let Some(Screen::Tune(t)) = self.stack.last() {
                        self.tuning = Some(t.tuning());
                    }
                    // `RESET TAB` on the Controls tab: the default bindings.
                    if let Some(Screen::Settings(s)) = self.stack.last_mut() {
                        if s.take_reset_bindings() {
                            if let Some((current, defaults)) = self.binds.as_mut() {
                                *current = *defaults;
                                self.bindings_changed = true;
                            }
                        }
                    }
                }
                Transition::SaveLook => {
                    if let Some(Screen::Tune(t)) = self.stack.last_mut() {
                        self.save_look = t.take_save();
                    }
                }
                Transition::OpenMainMenu | Transition::MatchOver | Transition::Paused => {}
            }
        }
        transition
    }

    /// Advance one presented frame with `input` held.
    pub fn step(&mut self, input: FrameInputs) -> FrameOutput {
        let prev = std::mem::replace(&mut self.prev, input);
        self.frames += 1;
        let connection_was_open = self.net_setup.is_some();
        self.receive_net_events();
        self.net_rollback_events();
        let network_closed = connection_was_open && self.net_setup.is_none();
        let events = menu_events(&prev, &input);
        let typed = std::mem::take(&mut self.typed);
        let mut quit = false;

        // The faithful cabinet takes every input itself; only a one-second
        // Esc hold leaves it.
        if let Some(cabinet) = self.cabinet.as_mut() {
            match cabinet.step(&input) {
                Some(CabinetEvent::Leave) => {
                    self.cabinet = None;
                    self.stack.clear();
                    self.attract_hold.extend([sound::STOP_ALL, sound::STOP_SAMPLES]);
                }
                Some(CabinetEvent::Untranslated { frame, addresses }) => {
                    self.cabinet = None;
                    self.abandon(FightEvent::Untranslated { frame, addresses }, "the faithful cabinet");
                }
                None => {}
            }
            if let Some(cabinet) = &self.cabinet {
                let font = RomFont::new(&self.assets.gfx4, &self.assets.maincpu);
                let m = cabinet.machine();
                return FrameOutput {
                    frame: cabinet.render(&self.assets, &font),
                    sound_frame: self.frames,
                    sound_latch: m.state.sound_latch,
                    latch_cycles: m.state.latch_cycles,
                    announcements: Vec::new(),
                    tune_table: false,
                    track: None,
                    screen: self.screen(),
                    transition: None,
                    quit: false,
                    settings_changed: std::mem::take(&mut self.settings_changed),
                    bindings_changed: std::mem::take(&mut self.bindings_changed),
                    video: self.video(),
                    panel: None,
                };
            }
        }

        let online_play = self.net_setup.is_some();
        let online_menu_was_open = self.play.as_ref().is_some_and(|play|
            online_play && (play.pause.is_some() || play.after.is_some()));
        let mut transition = if network_closed {
            None
        } else if self.cabinet.is_some() {
            None
        } else if self.play.is_some() {
            self.play_input(&prev, &input, &events)
        } else if self.stack.is_empty() {
            // Start is honoured from the first frame after power-on's RAM
            // clear, which ends in frame 1 (`0x3b04a`).
            (self.attract.state.frame >= 1 && attract_start(&prev, &input)).then(|| {
                let main = self.menu(MenuScreen::main());
                self.stack.push(main);
                self.sounds.extend([sound::STOP_ALL, sound::STOP_SAMPLES]);
                Transition::OpenMainMenu
            })
        } else {
            self.screen_input(&prev, &input, typed, &mut quit)
        };

        self.online_actions();
        self.net_apply_inputs();

        // The fight steps first, unless a box holds it; an untranslated
        // branch ends it before anything is drawn.
        let mut fight_frame = None;
        let mut abandoned = None;
        let mut returned = false;
        let mut winner = None;
        let mut world_tour_result = None;
        let start = events.contains(&MenuInput::Start);
        if let Some(play) = self.play.as_mut().filter(|p| p.held && !online_play) {
            play.held = false;
        } else if let Some(play) = self.play.as_mut().filter(|p| online_play || (p.after.is_none() && p.pause.is_none())) {
            let assets = fight_assets(&self.assets, &self.bank_assets, play.fight.setup().config.preset);
            let ff = if self.net_setup.as_ref().is_some_and(|setup| setup.local_side.is_none()) {
                Some(self.net_spectators.step(&mut play.fight, assets))
            } else if let Some(rollback) = play.netplay.as_mut() {
                let local = if online_menu_was_open || play.pause.is_some() { PlayerInputs::default() } else { input.p1 };
                rollback.step(&mut play.fight, assets, local)
            } else {
                Some(play.fight.step(assets, input.p1, input.p2, start))
            };
            if let Some(fallback) = play.fight.take_missing_ryu_theme() {
                self.log.push(format!(
                    "themes: Ryu's theme ({:#04x}) is not in the music pack: playing the stage's 1987 theme {fallback:#04x}",
                    sf_video::background_bank::RYU_THEME
                ));
            }
            if let Some(ff) = ff { match ff.event {
                Some(FightEvent::ReturnedToAttract) => returned = true,
                Some(FightEvent::MatchOver) => {
                    if play.world_tour.is_some() {
                        world_tour_result = Some(play.fight.match_winner());
                        transition = Some(Transition::MatchOver);
                    } else if play.netplay.as_ref().is_none_or(|rollback| rollback.confirmed_match_over()) {
                        play.after = Some(if self.net_setup.is_some() { AfterMatchBox::open_online() }
                            else { AfterMatchBox::default() });
                        winner = Some(play.fight.match_winner());
                        transition = Some(Transition::MatchOver);
                    }
                    play.last = ff.frame.clone();
                    fight_frame = Some(ff);
                }
                Some(ref event) => abandoned = Some(event.clone()),
                None => {
                    play.last = ff.frame.clone();
                    fight_frame = Some(ff);
                }
            } }
        }
        // The loser picks the next stage; after a draw the picker stays.
        if let Some(Some(won)) = winner {
            self.picker = 1 - won;
        }
        if let Some(event) = abandoned {
            self.abandon(event, "a fight");
        }
        if let Some(result) = world_tour_result {
            self.finish_world_tour_match(result);
        }
        if returned {
            if self.play.as_ref().is_some_and(|play| play.world_tour.is_some()) {
                self.leave_world_tour_fight();
            } else {
                self.arcade_over();
            }
        }
        self.net_rollback_events();
        self.net_send();
        self.net_spectator_tick();
        // The stage-pick rule starts over when the players are back at the
        // menu or the attract.
        if self.play.is_none() && !matches!(self.stack.last(), Some(Screen::Select(_) | Screen::Stage(_))) {
            self.picker = 0;
            self.chosen = None;
        }

        // While a box holds the fight, the latch carries the boxes' own
        // sounds.
        let box_sound = (self.play.is_some() && fight_frame.is_none())
            .then(|| self.sounds.pop_front().unwrap_or(sound::IDLE));
        let font = RomFont::new(&self.assets.gfx4, &self.assets.maincpu);
        let look_name = match self.looks.iter().find(|l| l.id == self.look()) {
            Some(look) => super::pause::short_look_name(&look.name),
            None => super::pause::short_look_name(self.look()),
        };
        let presented = self.play.as_ref().map(|play| {
            let boxed = |frame: Frame| match (&play.after, &play.pause) {
                (Some(after), _) => after.render(&font, &frame),
                // The move list is a layer beside the fight: the fight shows as it was.
                (None, Some(_)) if play.moves.is_some() => frame,
                (None, Some(pause)) => pause.render(
                    &font,
                    &frame,
                    play.fight.overlay().label(),
                    &look_name,
                    play.fight.gauge_art(),
                    play.fight.setup().config.preset,
                    play.fight.hold_combo(),
                    play.fight.setup().config.fill_meters,
                ),
                (None, None) => frame,
            };
            match fight_frame {
                Some(ff) => (boxed(ff.frame), ff.sound_latch, ff.latch_cycles, ff.announcements),
                None => (boxed(play.last.clone()), box_sound.unwrap_or(sound::IDLE), 0, Vec::new()),
            }
        });
        let (mut frame, sound_latch, latch_cycles, announcements) = match presented {
            Some(p) => p,
            None if self.stack.is_empty() => {
                self.sounds.clear();
                // Held for the returning stop commands, else stepped.
                let held = self.attract_hold.pop_front();
                if held.is_none() {
                    self.attract.step();
                }
                let frame = match &self.free_play {
                    Some(fp) => fp.present(&self.assets, &self.attract.state),
                    None => render_frame(&self.assets, &self.attract.state.scene()),
                };
                match held {
                    Some(cmd) => (frame, cmd, 0, Vec::new()),
                    None => (frame, self.attract.state.sound_latch, self.attract.state.latch_cycles, Vec::new()),
                }
            }
            None => {
                let frame = match self.stack.last().expect("a screen") {
                    Screen::Menu(m) => m.render(&font, self.title_logo.as_ref(), self.notice.as_deref()),
                    Screen::Select(s) => s.render(&font, &self.busts),
                    Screen::Stage(s) => s.render(&font),
                    Screen::WorldTour(screen) => screen.render(&font, &self.busts, &self.world_tour_profile, self.frames, self.notice.as_deref()),
                    Screen::DramaticSetup(s) => s.render(&font, self.notice.as_deref()),
                    Screen::DramaticPick(s) => s.render(&font, &self.busts, self.notice.as_deref()),
                    Screen::DramaticGuest(s) => {
                        let stages = self.backgrounds.as_ref().map_or(&[][..], |bank| bank.listed_stages());
                        s.render(&font, &self.busts, stages, self.notice.as_deref())
                    }
                    Screen::Settings(s) => s.render(&font),
                    Screen::Dip(d) => d.render(&font),
                    Screen::Rebind(r) => r.render(&font),
                    Screen::Jukebox(j) => j.render(&font),
                    Screen::Online(screen) => screen.render(&font),
                    // The demo fight loops behind the panel, silent but for
                    // the screen's own sounds; no input reaches it.
                    Screen::Tune(_) => {
                        let demo = self.demo.get_or_insert_with(|| Box::new(DemoScene::new(&self.assets.maincpu)));
                        demo.step();
                        match &self.free_play {
                            Some(fp) => fp.present(&self.assets, &demo.machine().state),
                            None => render_frame(&self.assets, &demo.machine().state.scene()),
                        }
                    }
                };
                (frame, self.sounds.pop_front().unwrap_or(sound::IDLE), 0, Vec::new())
            }
        };
        self.net_hud(&mut frame);
        let panel = match (&self.play, self.stack.last()) {
            (Some(play), _) => play.moves.as_ref().map(|m| m.panel(&font)),
            (None, Some(Screen::Tune(t))) => t.panel(&font),
            _ => None,
        };
        let track = self.track_cue(sound_latch);
        FrameOutput {
            frame,
            sound_frame: self.frames,
            sound_latch,
            latch_cycles,
            announcements,
            tune_table: self.play.as_ref().is_some_and(|p| p.fight.setup().config.preset != RulesetPreset::Faithful1987),
            track,
            screen: self.screen(),
            transition,
            quit,
            settings_changed: std::mem::take(&mut self.settings_changed),
            bindings_changed: std::mem::take(&mut self.bindings_changed),
            video: self.video(),
            panel,
        }
    }
}

/// The flag the cabinet driver picks, hidden, for a Local or Training
/// dummy fight whose stage was chosen on the stage screen, and for
/// Training against the fair CPU: Japan's. Training against Capcom's CPU
/// (Faithful 1987) picks the flag whose first opponent is the chosen one
/// instead (`first_opponent_flag`). Either way the picked stage decides the
/// scenery and tune, and the country intro is skipped, so the flag shows
/// nowhere.
const HIDDEN_FLAG: u8 = 0;

/// The graphics a fight under `preset` draws with: the bank's under an
/// overhaul preset, the ROM's own under Faithful 1987 or with no bank.
fn fight_assets<'a>(rom: &'a Arc<Assets>, bank: &'a Option<Arc<Assets>>, preset: RulesetPreset) -> &'a Assets {
    match bank {
        Some(bank) if preset != RulesetPreset::Faithful1987 => bank,
        _ => rom,
    }
}

/// The move list for `side` (0 Player 1, 1 Player 2) of `fight`: that
/// fighter's moves, written in `notation` for the way he faces now.
fn side_moves(fight: &Fight, side: usize, notation: InputNotation) -> MovesPanel {
    let setup = fight.setup();
    let machine = fight.machine();
    let game_mode = machine.work_ram()[(crate::fighter::GAME_MODE - crate::fighter::WORK_BASE) as usize];
    let player = if side == 0 && game_mode != 4 { P1_STRUCT } else { P2_STRUCT };
    let facing_right = facing_right(machine.work_ram(), &machine.state.objectram, player);
    let fighter = fight.player_fighter(side);
    let player_table = machine.data_bank().is_some_and(|b| b.table(fighter).is_some());
    let state = fight.pad_era_state(side);
    let entries = crate::moves::live_move_list(fighter, setup.config.preset, player_table, state);
    let panel = MovesPanel::from_entries(fighter, entries);
    let panel = if let Some(form) = machine.satsui_awakening().filter(|form| form.side.fighter_struct() == player) {
        use crate::era::{EraFamily, Loadout, Revision};

        let base_kit = state.style.revision != Revision::St || state.style.loadout != Loadout::Standard;
        if form.profile == crate::config::EvilRyuPlaytest::UltraSf2
            && !matches!(state.style.family, EraFamily::Sf1Plus | EraFamily::Sf2)
        {
            panel.with_evil_ryu_name()
        } else {
            panel.with_evil_ryu_moves(form.profile, form.profile == crate::config::EvilRyuPlaytest::UltraSf2 && base_kit)
        }
    } else if let (0, Some(playtest)) = (side, setup.config.evil_ryu_playtest) {
        panel.with_evil_ryu_moves(playtest, false)
    } else {
        panel
    };
    panel
        .with_inputs(notation, facing_right)
        .with_show(fight.training())
}

/// Both local human pads have their own move lists.
fn two_human_move_lists(kind: FightKind) -> bool {
    matches!(kind, FightKind::Local | FightKind::Dramatic { co_op: true })
}

fn edge(prev: bool, now: bool) -> bool {
    now && !prev
}

/// The attract opens the menu on any Start: the menu Start, the menu
/// confirm, or either cabinet start. Attack buttons do nothing, as on the
/// cabinet.
fn attract_start(prev: &FrameInputs, now: &FrameInputs) -> bool {
    any_menu_edge(prev, now, |m| m.start)
        || any_menu_edge(prev, now, |m| m.confirm)
        || edge(prev.cabinet[2], now.cabinet[2])
        || edge(prev.cabinet[3], now.cabinet[3])
}

/// A side's pad menu button's edge (`key` picks the button).
fn pad_edge(prev: &FrameInputs, now: &FrameInputs, side: usize, key: fn(&MenuKeys) -> bool) -> bool {
    let held = |i: &FrameInputs| i.pads[side].as_ref().is_some_and(key);
    edge(held(prev), held(now))
}

/// A menu key's edge on the keyboard or on either side's pad.
fn any_menu_edge(prev: &FrameInputs, now: &FrameInputs, key: fn(&MenuKeys) -> bool) -> bool {
    edge(key(&prev.menu), key(&now.menu)) || (0..2).any(|side| pad_edge(prev, now, side, key))
}

/// The Local select's input edges, side by side: each side's stick, its
/// pad's menu buttons or (on the keyboard) its LP and LK, and its cabinet
/// start; the keyboard's menu keys go with P1, or with P2 when only P1 is
/// on a pad.
fn side_events(prev: &FrameInputs, now: &FrameInputs) -> Vec<(usize, MenuInput)> {
    let mut events = Vec::new();
    let keys_side = usize::from(now.pads[0].is_some() && now.pads[1].is_none());
    let sides = [(prev.p1, now.p1), (prev.p2, now.p2)];
    for (side, (p, n)) in sides.iter().enumerate() {
        for (i, ev) in [
            (3, MenuInput::Up),
            (2, MenuInput::Down),
            (1, MenuInput::Left),
            (0, MenuInput::Right),
        ] {
            if edge(p.directions[i], n.directions[i]) {
                events.push((side, ev));
            }
        }
        let keys = side == keys_side;
        let keyboard = now.pads[side].is_none();
        if (keyboard && edge(p.buttons[0], n.buttons[0]))
            || (keys && edge(prev.menu.confirm, now.menu.confirm))
            || pad_edge(prev, now, side, |m| m.confirm)
        {
            events.push((side, MenuInput::Confirm));
        }
        if edge(prev.cabinet[2 + side], now.cabinet[2 + side])
            || (keys && edge(prev.menu.start, now.menu.start))
            || pad_edge(prev, now, side, |m| m.start)
        {
            events.push((side, MenuInput::Start));
        }
        if (keyboard && edge(p.buttons[3], n.buttons[3]))
            || (keys && edge(prev.menu.back, now.menu.back))
            || pad_edge(prev, now, side, |m| m.back)
        {
            events.push((side, MenuInput::Back));
        }
    }
    events
}

/// One input to a front-end screen: a menu input, or a key typed into an
/// open text entry.
#[derive(Clone, Copy, Debug)]
enum Event {
    Menu(MenuInput),
    Typed(TypedKey),
}

/// The pads' menu input edges alone: a pad side's stick and its pad's
/// confirm, back and Start. The keyboard types into an open text entry
/// meanwhile.
fn pad_menu_events(prev: &FrameInputs, now: &FrameInputs) -> Vec<MenuInput> {
    let mut events = Vec::new();
    let players = [(prev.p1, now.p1), (prev.p2, now.p2)];
    for (i, ev) in [
        (3, MenuInput::Up),
        (2, MenuInput::Down),
        (1, MenuInput::Left),
        (0, MenuInput::Right),
    ] {
        if players
            .iter()
            .enumerate()
            .any(|(side, (p, n))| now.pads[side].is_some() && edge(p.directions[i], n.directions[i]))
        {
            events.push(ev);
        }
    }
    for (key, ev) in [
        ((|m: &MenuKeys| m.confirm) as fn(&MenuKeys) -> bool, MenuInput::Confirm),
        (|m: &MenuKeys| m.start, MenuInput::Start),
        (|m: &MenuKeys| m.back, MenuInput::Back),
    ] {
        if (0..2).any(|side| pad_edge(prev, now, side, key)) {
            events.push(ev);
        }
    }
    events
}

/// Menu input edges from every source at once (one cursor for both
/// players): either stick, LP to confirm and LK to go back on either side,
/// the menu keys, and either cabinet start as Start.
pub(crate) fn menu_events(prev: &FrameInputs, now: &FrameInputs) -> Vec<MenuInput> {
    let mut events = Vec::new();
    let players = [(prev.p1, now.p1), (prev.p2, now.p2)];
    let dir = |i: usize| players.iter().any(|(p, n)| edge(p.directions[i], n.directions[i]));
    // A pad side's buttons are not confirm and back: its pad has its own.
    let button = |i: usize| {
        players
            .iter()
            .enumerate()
            .any(|(side, (p, n))| now.pads[side].is_none() && edge(p.buttons[i], n.buttons[i]))
    };
    for (i, ev) in [
        (3, MenuInput::Up),
        (2, MenuInput::Down),
        (1, MenuInput::Left),
        (0, MenuInput::Right),
    ] {
        if dir(i) {
            events.push(ev);
        }
    }
    if any_menu_edge(prev, now, |m| m.confirm) || button(0) {
        events.push(MenuInput::Confirm);
    }
    if any_menu_edge(prev, now, |m| m.start)
        || edge(prev.cabinet[2], now.cabinet[2])
        || edge(prev.cabinet[3], now.cabinet[3])
    {
        events.push(MenuInput::Start);
    }
    if any_menu_edge(prev, now, |m| m.back) || button(3) {
        events.push(MenuInput::Back);
    }
    if any_menu_edge(prev, now, |m| m.tab_left) {
        events.push(MenuInput::TabLeft);
    }
    if any_menu_edge(prev, now, |m| m.tab_right) {
        events.push(MenuInput::TabRight);
    }
    events
}
