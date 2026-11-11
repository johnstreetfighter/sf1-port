//! Fixture validation at checkpoint 790 (warning screen): text layer
//! only — exercises char-tile decode, video-RAM layout, palette, and the
//! visible-area origin with real glyph data.
//!
//! Checkpoint 790 sits inside the statically displayed warning screen
//! (screen hash constant over frames 780-794 in the capture's
//! frames.tsv), so its pixel snapshot and RAM dumps describe the same
//! machine state. Checkpoint 794 by contrast is a transition frame: its
//! pixel snapshot is black while its video-RAM dump already carries the
//! next screen's text, so it cannot validate a renderer and is not used.
//!
//! Consumes local-only data; skips on a clean checkout.

mod common;

#[test]
fn frame_790_matches_the_reference_capture_exactly() {
    // gfxctrl low byte 0x08 (text layer only) is written at frame 8 of the
    // reference run and first changes at frame 795 (register tap evidence).
    let Some((scene, reference_pixels)) = common::checkpoint_scene(790, 0, 0, 0x08) else {
        eprintln!("skipping: local reference capture not present");
        return;
    };
    common::assert_frame_matches_reference(&scene, &reference_pixels);
}
