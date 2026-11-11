//! Shared helpers for the byte-exact frame-fixture tests.
//!
//! These fixtures consume the local-only reference capture and ROM set, so
//! each test skips cleanly on a checkout without them.
#![allow(dead_code)]

use sf_video::render::{render_frame, HEIGHT, WIDTH};
use sf_video::romset::load_assets;
use sf_video::scene::SceneState;

pub const CAPTURE: &str = "../../tools/reference/local/captures/fight-run-a";
pub const ROMS: &str = "../../tools/reference/local/roms/sf";
pub const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// Load the checkpoint dumps for `frame` as a SceneState, applying the
/// write-only register values recovered from the reference run's register
/// taps (docs/reference/first-frame.md). Returns None when the local
/// capture is absent.
pub fn checkpoint_scene(
    frame: u32,
    bg_scroll: u16,
    fg_scroll: u16,
    active: u8,
) -> Option<(SceneState, Vec<u8>)> {
    let read = |suffix: &str| std::fs::read(format!("{CAPTURE}/ram_{frame:06}_{suffix}.bin")).ok();
    let video = read("video")?;
    let palette = read("palette")?;
    let work = read("work")?;
    let reference_pixels = std::fs::read(format!("{CAPTURE}/pixels_{frame:06}.rgba")).ok()?;
    let scene = SceneState {
        videoram: video,
        objectram: work[0x6000..0x8000].to_vec(), // object RAM at 0xffe000
        paletteram: palette,
        bg_scroll,
        fg_scroll,
        active,
        flip_screen: false,
        bg_page: 0,
        fg_page: 0,
    };
    Some((scene, reference_pixels))
}

/// Render `scene` and assert byte-equality with the reference checkpoint
/// pixels (B,G,R,A bytes — MAME snapshot ARGB u32 LE) at game resolution.
pub fn assert_frame_matches_reference(scene: &SceneState, reference_pixels: &[u8]) {
    let assets = load_assets(ROMS, MANIFEST).expect("ROM regions assemble");
    let frame = render_frame(&assets, scene);
    assert_eq!(frame.argb.len(), WIDTH * HEIGHT);
    let mut diffs = 0usize;
    let mut first = None;
    for i in 0..WIDTH * HEIGHT {
        let (b, g, r, a) = (
            reference_pixels[4 * i],
            reference_pixels[4 * i + 1],
            reference_pixels[4 * i + 2],
            reference_pixels[4 * i + 3],
        );
        let expected = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        if frame.argb[i] != expected {
            diffs += 1;
            if first.is_none() {
                first = Some((i % WIDTH, i / WIDTH, expected, frame.argb[i]));
            }
        }
    }
    if let Some((x, y, e, a)) = first {
        panic!("{diffs} differing pixels; first at ({x},{y}): expected {e:08x}, actual {a:08x}");
    }
}
