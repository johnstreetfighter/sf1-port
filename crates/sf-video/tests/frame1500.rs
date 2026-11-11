//! Fixture validation at checkpoint 1500 (title screen): the logo and all
//! text are drawn by the tx (character) layer — exercises char-tile
//! decode, video-RAM layout, and palette with real glyph data.
//!
//! Consumes local-only data; skips on a clean checkout.

mod common;

#[test]
fn frame_1500_matches_the_reference_capture_exactly() {
    // gfxctrl low byte 0x08 (text layer only) is written at frame 1467 of
    // the reference run; the next write (0x48, enabling fg) lands at frame
    // 1521 (register tap evidence), so frame 1500 is text-only with
    // scrolls irrelevant.
    let Some((scene, reference_pixels)) = common::checkpoint_scene(1500, 0, 0, 0x08) else {
        eprintln!("skipping: local reference capture not present");
        return;
    };
    common::assert_frame_matches_reference(&scene, &reference_pixels);
}
