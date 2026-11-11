//! The development launcher's window (minifb): open it at an integer
//! upscale of the 384×224 game frame, present a frame, and read a key's
//! press edge. The game draws through the GPU window instead
//! ([`crate::gpu_window`], ADR-0010).

use minifb::{Key, Window, WindowOptions};
use sf_video::render::{Frame, HEIGHT, WIDTH};

/// Integer upscale for display; the acceptance comparison happens at game
/// resolution before this scaling.
pub const SCALE: usize = 3;

/// Open a window at [`SCALE`] times the game resolution, paced to 60 fps.
pub fn open_window(title: &str) -> Window {
    let mut window = Window::new(
        title,
        WIDTH * SCALE,
        HEIGHT * SCALE,
        WindowOptions::default(),
    )
    .expect("open window");
    window.set_target_fps(60);
    window
}

/// A zeroed presentation buffer for a window opened by [`open_window`].
pub fn frame_buffer() -> Vec<u32> {
    vec![0u32; WIDTH * SCALE * HEIGHT * SCALE]
}

/// Upscale `frame` into `buf` and show it.
pub fn present(window: &mut Window, buf: &mut [u32], frame: &Frame) {
    for y in 0..HEIGHT * SCALE {
        for x in 0..WIDTH * SCALE {
            buf[y * WIDTH * SCALE + x] = frame.argb[(y / SCALE) * WIDTH + (x / SCALE)];
        }
    }
    window
        .update_with_buffer(buf, WIDTH * SCALE, HEIGHT * SCALE)
        .unwrap();
}

/// True on the frame `key` goes down while the window is focused; `prev`
/// carries the key's state between calls.
pub fn poll_key_edge(window: &mut Window, key: Key, prev: &mut bool) -> bool {
    let down = window.is_active() && window.is_key_down(key);
    let edge = down && !*prev;
    *prev = down;
    edge
}
