//! Host glue shared by the game binary and the development launcher
//! (ADR-0005): the audio output, the input bindings table with its TOML
//! persistence, the pad reader, the game's per-frame inputs from the
//! keyboard and the pads, the game's GPU window with its pacing, keys, looks and the tuning panel,
//! the launcher's window open and present helpers, and the
//! game's files, log, ROM lookup, art, stages, themes, the Jukebox's tracks and announcer's voice. Nothing here
//! knows about screens or fixtures; both binaries drive it from their own
//! loops.

pub mod art;
pub mod audio;
pub mod bindings;
#[cfg(windows)]
pub mod clipboard;
pub mod files;
pub mod gpu_window;
pub mod input;
pub mod jukebox;
pub mod keys;
pub mod log;
pub mod net;
pub mod look;
pub mod pacing;
pub mod pads;
pub mod panel;
pub mod picture;
pub mod roms;
pub mod shaders;
pub mod stages;
pub mod themes;
pub mod track;
#[cfg(feature = "tracker")]
pub mod tracker;
pub mod voice;
pub mod window;
