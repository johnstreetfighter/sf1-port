//! Acceptance test for ticket #4: render the fixed fight's initial frame
//! (checkpoint 2280, round-1 fight start) from decoded original graphics
//! data and documented scene state, and compare byte-exactly against the
//! pinned reference capture's checkpoint pixels at game resolution, before
//! display scaling. All four layers (bg, fg, sprites, tx) are active.
//!
//! Consumes local-only data; skips on a clean checkout.

mod common;

#[test]
fn frame_2280_matches_the_reference_capture_exactly() {
    // Write-only register state recovered from register taps of the
    // reference run: gfxctrl 0xe8 written at frame 2274, fg_scroll 0x1d40
    // at frame 2110, bg_scroll 0x1800 at frame 2110 (its increments resume
    // at frame 2283). See docs/reference/first-frame.md.
    let Some((scene, reference_pixels)) = common::checkpoint_scene(2280, 0x1800, 0x1d40, 0xe8)
    else {
        eprintln!("skipping: local reference capture not present");
        return;
    };
    common::assert_frame_matches_reference(&scene, &reference_pixels);
}
