---
status: accepted
---

# The game draws through wgpu and runs RetroArch shader presets with librashader

The user wanted the look of an arcade monitor, on the level of RetroArch's CRT shaders, and smooth motion on a 120 Hz display. Today's minifb window sleeps for a frame instead of waiting for the display's refresh, so pans step unevenly. We decided that the shipped game draws through a winit and wgpu window presented on vsync, and that every look, built in, saved or dropped in, is a RetroArch `.slangp` preset run by librashader's wgpu runtime from shader files vendored at a pinned `libretro/slang-shaders` commit. We chose this because the best CRT shaders already exist as RetroArch presets, and a throwaway test on the user's Radeon 780M at 3200×2000 ran full Royale and Guest at 120/60 at true 4:3. The same choice also gives vsync pacing and drop-in support with no second shader path.

True 4:3 is the one exception to scaling pixel art by whole numbers only. The 1987 cabinet's 4:3 tube showed each game pixel 7/9 as wide as it was tall, so true 4:3 serves the faithful goal. The vertical scale stays a whole number so scanlines are evenly spaced.

## Considered options

- Our own CRT shader on the same window, with librashader only for drop-ins: fully ours and free of the GPL question for built-in looks, but unlikely to match years of work on Guest and Royale, and two shader paths to maintain.
- A software filter in the minifb window: no new dependencies, but it can't produce the look at 3200×2000, can't run RetroArch files, and doesn't fix the pacing.

## Consequences

The host owns a second window path. The development launcher stays on minifb until a later project moves it. Keys from winit are translated into the existing binding names, so `bindings.toml` is unchanged. Most of the vendored shaders are GPL v2-or-later, kept as separate data files with their own headers in the private repository, and they need a licensing check when distribution is decided. librashader's wgpu backend is newer than its Direct3D ones: `crt-geom` ran at 3 fps in the test, so every built-in look is checked by running it. The design is [`docs/design/2026-11-03-video-options-design.md`](../design/2026-11-03-video-options-design.md).
