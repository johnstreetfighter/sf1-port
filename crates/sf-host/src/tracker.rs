//! Tracker modules as Jukebox tracks (ADR-0009, design section 6a.5, ticket
//! #201 `ST9`): libopenmpt (BSD-3-Clause, built from the vendored source in
//! `sf-openmpt`) renders the module, once, to the same decoded stereo `f32`
//! a file of another format becomes, with the loop its own restart position
//! gives.
//!
//! - **Which files.** libopenmpt reads a module by its content, not its
//!   name, and plays every format on its list: MOD, XM, S3M, IT, MPTM, 669,
//!   MTM and some forty more ([`sf_openmpt::supported_extensions`]). A file
//!   whose extension is on that list (WAV, OGG and FLAC aside) goes to it
//!   first, and symphonia gets the file if libopenmpt cannot play it; any
//!   other file that symphonia does not recognise is offered to libopenmpt
//!   second. So a wrong or missing extension still plays.
//! - **The loop is the module's own** (*my call* on the method). libopenmpt
//!   plays the song once (repeat count 0, `play.at_end` `stop`, so it ends
//!   where it would jump back): that is the track, `end` frames. The
//!   restart position it reports for the module (`restart_order` and
//!   `restart_row`, which include a jump back that makes the song loop, such
//!   as a last-pattern Bxx) is looked up by seeking a fresh module there,
//!   and the time it comes back is the loop's start. The track loops from
//!   that frame to `end`; no restart position is a loop from frame 0. The
//!   intro before it plays once, as in every other track. libopenmpt's
//!   tempo and speed carry over its own jump back, so a module that
//!   changes them later in the song would play the loop the second time
//!   with the tempo it ended on; the track plays the loop as it sounds the
//!   first time through.
//! - **Rate and length.** Rendered at 48 kHz; refused past
//!   [`MAX_SECONDS`](crate::track::MAX_SECONDS) (a module that never ends
//!   hits this too), like any other track.

use std::sync::atomic::{AtomicBool, Ordering};

use sf_openmpt::Module;

use crate::track::{LoopPoints, Track, MAX_SECONDS};

/// The rate a module is rendered at (Hz).
pub const RATE: u32 = 48_000;

/// Frames rendered per call into libopenmpt.
const CHUNK: usize = 4096;

/// Whether a file with this (lower case) extension is a tracker module for
/// libopenmpt to read first. WAV, OGG and FLAC are never: symphonia has
/// those.
pub fn is_tracker_extension(extension: &str) -> bool {
    !matches!(extension, "wav" | "ogg" | "flac") && sf_openmpt::supported_extensions().iter().any(|e| e == extension)
}

/// Quieter than this, the whole way through, is no sound: a module whose
/// samples did not load (a truncated file) plays on and renders silence.
const SILENT: f32 = 1e-4;

/// Render the module in `bytes`, played once, to its end. It is an error to
/// pass [`MAX_SECONDS`], to be `cancel`led, or for libopenmpt to stop on an
/// error.
fn render(module: &mut Module, cancel: &AtomicBool) -> Result<Vec<[f32; 2]>, String> {
    module.set_repeat_count(0);
    // The song ends at its end: no fade-out tail after the loop's last frame.
    module.set_control("play.at_end", "stop");
    let limit = MAX_SECONDS * u64::from(RATE);
    let (mut frames, mut buffer) = (Vec::new(), vec![0f32; CHUNK * 2]);
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(crate::track::CANCELLED.to_owned());
        }
        let n = module.read_stereo(RATE, &mut buffer).map_err(|e| format!("libopenmpt stopped playing it ({e})"))?;
        if n == 0 {
            return Ok(frames);
        }
        if frames.len() as u64 + n as u64 > limit {
            return Err(format!("longer than {} minutes", MAX_SECONDS / 60));
        }
        frames.extend(buffer[..n * 2].chunks_exact(2).map(|c| [c[0], c[1]]));
    }
}

/// The frame the song goes back to once it has played to its end: the
/// restart position libopenmpt reports for the module (its own, or the
/// target of a jump back that loops it), as the time it is reached from the
/// start. 0 if it reports none, or one the first play-through does not reach
/// (`end` frames long).
fn restart_frame(bytes: &[u8], end: u64) -> u64 {
    let Ok(mut module) = Module::from_memory(bytes) else { return 0 };
    let Some((order, row)) = module.restart_position() else { return 0 };
    let seconds = module.seek_order_row(order, row);
    let frame = (seconds * f64::from(RATE)).round();
    if frame.is_finite() && frame > 0.0 && (frame as u64) < end {
        frame as u64
    } else {
        0
    }
}

/// Decode the module in `bytes`.
pub fn decode(bytes: &[u8], cancel: &AtomicBool) -> Result<Track, String> {
    let mut module = Module::from_memory(bytes).map_err(|e| format!("not a playable tracker module ({e})"))?;
    let frames = render(&mut module, cancel)?;
    if !frames.iter().any(|f| f[0].abs().max(f[1].abs()) >= SILENT) {
        return Err("no sound in the module (its samples did not load, or it plays none)".to_owned());
    }
    let end = frames.len() as u64;
    Ok(Track { rate: RATE, frames, looped: Some(LoopPoints { start: restart_frame(bytes, end), end }) })
}
