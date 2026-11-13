//! The overhaul front end (spec #150; ADR-0004, ADR-0005): the screens a
//! player sees around the 1987 machine, and the headless game session the
//! game binary is a window loop over.
//!
//! Every screen is a pure state machine: it takes [`MenuInput`] edges and
//! answers with a [`Transition`], and draws itself into a 384×224 frame
//! with the ROM's own text tiles. Nothing here opens a window, reads a
//! file or writes the 1987 machine's state; the attract runs the faithful
//! power-on machine untouched and the front end only composites over it.
//!
//! [`MenuInput`]: crate::roster::MenuInput

pub mod after_match;
pub mod attract;
pub mod cabinet;
pub mod coach;
pub mod dramatic_select;
mod draw;
pub mod faithful;
pub mod fight;
pub mod jukebox;
pub mod menu;
pub mod moves;
pub mod online;
pub mod pause;
pub mod rebind;
mod replays;
pub mod rom_missing;
pub mod select;
pub mod session;
pub mod show;
mod style_card;
pub mod settings;
pub mod stage_select;
pub mod tune;
pub mod world_tour;

pub use fight::FightDips;
pub use menu::{MenuItem, MenuScreen};
pub use session::{FrameInputs, FrameOutput, GameSession, MenuKeys, PlayerInputs, VideoState};
pub use tune::{LookParam, LookTuning, Panel, PanelSide, SaveLook, TypedKey};

use crate::roster::{StagePick, TournamentFighter};

/// What a screen asks the session to do after an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    /// The cursor moved to another row.
    CursorMoved,
    /// An item was confirmed.
    Choose(MenuItem),
    /// Back: leave this screen for the one under it.
    Back,
    /// The attract's Start: open the Main Menu over the paused attract.
    OpenMainMenu,
    /// The select locked P1's fighter.
    Picked(TournamentFighter),
    /// A fighter tile opened its Style card.
    StyleOpened,
    /// The select unlocked P1's fighter.
    Unpicked,
    /// The select is done: P1's and P2's fighters.
    Fighters(TournamentFighter, TournamentFighter),
    /// The select is done for a one-player mode: P1's fighter.
    Fighter(TournamentFighter),
    /// Dramatic Battle setup is ready to start or choose a custom team.
    StartDramatic,
    /// Launch the selected World Tour mission.
    StartWorldTour,
    /// Open the shared fighter and style picker for World Tour.
    WorldTourSelect,
    /// The stage screen picked a stage: the fight starts on it.
    Stage(StagePick),
    /// The stage screen picked its `RANDOM` tile: the session chooses a
    /// listed stage as the fight starts.
    RandomStage,
    /// The fight ended its match (before the winner carries on).
    MatchOver,
    /// The pause box opened over the fight.
    Paused,
    /// A setting's value changed.
    ValueChanged,
    /// The tuning screen asked to save its look: the host writes the file.
    SaveLook,
    WatchReplay(usize),
}

/// The ROM's FM effects the menus play (both verified by the service sound
/// test, `docs/reference/service-sound-test.md`): `0x09` is the
/// country-select cursor's sound (`0x3e110`), `0x0f` the credit chime
/// (`0x3b802` from the coin and service-credit paths).
pub mod sound {
    pub const CURSOR: u8 = 0x09;
    pub const CONFIRM: u8 = 0x0f;
    /// The original tune used by the two- and four-country screens.
    pub const COUNTRY_SELECT: u8 = 0x34;
    /// The title arrangement supplied by the local music pack.
    pub const TITLE_THEME: u8 = 0x7e;
    /// Stop the current and saved music while letting effects finish.
    pub const STOP_MUSIC: u8 = 0x3f;
    /// Music CPU special command 0: stop everything, the music and both
    /// effect slots (`0x02b5`, music-stops.md).
    pub const STOP_ALL: u8 = 0x00;
    /// Samples CPU: stop the channel (the poll at `0x0046`, hit-sound.md);
    /// boot sends it at frame 3 to silence both MSM5205s. With
    /// [`STOP_ALL`] on the frame before, it stops the attract's sound when
    /// a menu opens over it.
    pub const STOP_SAMPLES: u8 = 0x80;
    /// The latch's idle value, between commands.
    pub const IDLE: u8 = 0xff;
}
