## Problem Statement

Play feels choppy and stiff, worst when the stage pans left and right. The game window sleeps for about a frame instead of waiting for the display's refresh, which measures at about 58.4 fps. On a 120 Hz display, frames are shown for an uneven mix of two and three refreshes, and one is held over about once a second.

The picture is also nothing like the 1987 cabinet. The Video tab offers only a whole-number scale and borderless fullscreen. The game is shown with square pixels, so everything is about 28% too wide next to the cabinet's 4:3 tube, and there is no way to get an arcade monitor's look. Players who know RetroArch's CRT shaders can't use them here.

## Solution

The game draws through a graphics-card window (winit and wgpu), presented on vsync, so each game frame is shown for the same number of refreshes: two at 120 Hz, one at 60 Hz. Pans glide.

Every screen is shown through a **look**: a named way of showing the picture as a monitor would. Looks are RetroArch `.slangp` presets run by librashader. The game ships built-in looks tuned from RetroArch's best CRT shaders. Players can tune any look on a live screen, with a demo fight panning behind the knobs, and save the result as a look of their own. They can also drop RetroArch preset files into a folder. The picture can be shown at **true 4:3**, the shape the cabinet's tube gave it, or at 4:3 on whole-number scales, or with square pixels as today. Every video change applies at once.

First launch opens borderless fullscreen with the `ARCADE CABINET` look at true 4:3. The 1987 cabinet shares the player's look and aspect. Faithful 1987's game behaviour and pixels don't change: the look only changes what reaches the screen.

Design: `docs/design/2026-11-03-video-options-design.md`. Decision: `docs/adr/0010-gpu-window-and-retroarch-looks.md`. Vocabulary: **Look** and **True 4:3** in `CONTEXT.md`.

## User Stories

1. As a player on a 120 Hz display, I want every game frame shown for exactly two refreshes, so that stage pans glide instead of stepping.
2. As a player on a 60 Hz display, I want every game frame shown for exactly one refresh, so that motion is even.
3. As a player on a 144 Hz or 165 Hz display, I want a notice telling me to set my display to 120 or 60, so that I know why motion can't be perfectly even.
4. As a player, I want the Video tab to name my display's refresh rate and what it means, for example `120HZ: EVERY FRAME SHOWN TWICE`, so that I can see the game is pacing itself correctly.
5. As a latency-minded player, I want `VSYNC OFF`, so that frames reach the screen as soon as they're ready, tearing included.
6. As a player with vsync off, I want the game to keep running at exactly 60 game frames a second, so that the game never speeds up or slows down.
7. As a latency-minded player, I want `FRAME DELAY` in milliseconds, so that input is read later in each refresh and lag drops.
8. As a player, I want the game's speed and logic unchanged by any video setting, so that the arcade timing is untouched.
9. As a player, I want the picture at true 4:3, so that the fighters have the proportions the 1987 cabinet's monitor gave them.
10. As a purist, I want `4:3 INTEGER`, so that I get close to 4:3 with every pixel crisp.
11. As a player, I want `SQUARE PIXELS` kept, so that today's look is still available.
12. As a player, I want the vertical scale always a whole number, so that scanlines are evenly spaced.
13. As a player, I want the picture centred on black in every mode, so that it's never stretched to the screen's edges.
14. As a player, I want a list of built-in looks to pick from, so that I can get a great arcade-monitor look without tuning anything.
15. As a player, I want `OFF` first in the look list, so that I can always get a sharp picture.
16. As a player, I want an `ARCADE CABINET` look based on Guest's advanced CRT shader, so that the game looks like a 15 kHz arcade monitor.
17. As a player, I want a `CONSUMER TV` look based on Guest's NTSC shader, so that I can play with the composite colour and blur of a home TV.
18. As a player, I want a `PRO MONITOR` look based on CRT-Royale, so that I can have an aperture grille and phosphor glow.
19. As a player, I want a `CURVED TUBE` look when it can run properly, so that I can have a curved screen.
20. As a player, I want any look that can't hold full speed on my machine tagged `HEAVY` with the reason, so that I'm never misled about stutter.
21. As a player, I want the look to cover every screen (the attract, menus, Settings, select screens and fights), so that switching screens never flips the look on and off.
22. As a player of the 1987 cabinet, I want it to show my look and aspect, so that the cabinet looks like a cabinet too.
23. As a player, I want every video change to apply the moment I make it, so that I can see what I'm choosing.
24. As a player, I want fullscreen and windowed to switch in place, so that the window never closes and reopens.
25. As a player, I want `WINDOW SCALE` to go up to the largest that fits my screen, so that a window on a large display isn't tiny.
26. As a new player, I want the game to open borderless fullscreen with the `ARCADE CABINET` look at true 4:3, so that the first thing I see looks great.
27. As a new player, I want first-launch video defaults to leave the faithful gameplay defaults untouched, so that "every toggle off" still holds.
28. As a player, I want `RESET TAB` on the Video tab to restore these defaults, so that I can undo experiments.
29. As a player, I want the Video tab grouped under `DISPLAY`, `MONITOR` and `MOTION`, so that the rows are easy to find.
30. As a tinkerer, I want `TUNE CRT LOOK` to open a live tuning screen, so that I can adjust the look's knobs.
31. As a tinkerer, I want a demo fight with a stage pan looping behind the tuning panel, so that I judge the look and the motion on real gameplay.
32. As a tinkerer, I want the tuning panel drawn sharp on top, not through the look, so that I can read the knob names.
33. As a tinkerer, I want the picture to slide left while the panel is open, so that the panel covers as little of it as possible.
34. As a tinkerer, I want one key to hide the panel, so that I can judge the look across the whole screen.
35. As a tinkerer, I want one row per shader parameter, stepping by the parameter's own step between its own limits, so that every knob the shader offers is mine.
36. As a tinkerer, I want long parameter lists to scroll, grouped under the shader's own headings, so that a shader with a hundred knobs is still usable.
37. As a tinkerer, I want each change to show at once on the scene behind, so that I can tune by eye.
38. As a tinkerer, I want `SAVE AS`, so that my tuned look becomes a look of my own in the list.
39. As a tinkerer, I want `RESET LOOK`, so that I can return to the look's own values.
40. As a tinkerer, I want leaving with unsaved changes to ask whether to save them, so that I never lose a tuning by accident.
41. As a tinkerer, I want the built-in looks never changed by tuning, so that I can always get back to them.
42. As a tinkerer, I want my saved looks to survive moving or reinstalling the game, so that they don't break.
43. As a RetroArch user, I want to drop `.slangp` files and their shader folders into the game's shaders folder and have them appear in the list under their file names, so that I can use looks I already love.
44. As a RetroArch user, I want a dropped-in look's own parameters on the tuning screen, so that I can tune it like a built-in look.
45. As a player, I want a look that fails to load to fall back to `OFF` with the first error shown, so that a broken file never crashes the game.
46. As a player, I want a deleted drop-in in my settings to fall back to `ARCADE CABINET` with a log line, so that the game still starts.
47. As a player, I want switching looks never to freeze the picture, so that changing looks mid-fight is smooth; the old look stays until the new one is ready.
48. As a player, I want a `LOOK` row in the pause box, so that I can change the look mid-match without leaving the fight.
49. As a player, I want a look chosen in the pause box to become my look in Settings too, so that the two never disagree.
50. As a Training player, I want hitboxes, the frame meter and the input display to pass through the look like everything else, so that the picture is consistent, and to switch the look to `OFF` from the pause box when I study frames.
51. As a player, I want my `bindings.toml` to keep working unchanged, so that my keys still mean the same thing after the window changes.
52. As a player, I want my old `settings.toml` to load with the new video defaults filled in, so that upgrading never resets my other settings.
53. As a player, I want gamepads to work exactly as before, so that the new window changes nothing about input.
54. As a preservation-minded player, I want the game's own 384×224 pixels and timing unchanged, so that the comparison tests still hold.
55. As the maintainer, I want the shader files vendored at a pinned commit with their licenses and a README, so that the build is offline and uses exactly the versions tested.
56. As the maintainer, I want a smoke run that loads every built-in look on the real GPU and reports pacing, so that a broken or slow look is caught before the player sees it.
57. As the maintainer, I want the development launcher left on its current window, so that this project stays inside the shipped game.

## Implementation Decisions

- **Window (host).** A new GPU window in the host crate, built on winit and wgpu, borderless fullscreen or windowed. It presents with `Fifo` when vsync is on and `Immediate` when it's off. It queries the monitor's refresh rate, computes the picture rectangle by aspect, runs a librashader filter chain from the game frame texture into a picture texture, and composites the picture centred on black. The minifb window path stays in the host for the development launcher (ADR-0010).
- **Pacing rule.** When vsync is on and the refresh is within 1% of a whole multiple *n* of 60, one game frame advances every *n* presents. Otherwise the game advances on a 60 Hz clock, which is reset whenever pacing changes. The throwaway test showed that a stale clock makes the game run fast after switching vsync off. The game runs at 60 Hz, matching the MAME reference timeline, so there's no separate arcade frame rate.
- **Frame delay.** It waits a set number of milliseconds after each vblank before stepping the game and rendering. The cap comes from headroom measured on the user's 780M.
- **Picture rectangle.** This is a pure function from aspect and surface size to an origin and size. The vertical scale is the largest whole number that fits. `TRUE 4:3` sets width to height × 4/3. `4:3 INTEGER` uses the nearest whole number to 7/9 of the vertical scale for the horizontal scale, stepping down until it fits. `SQUARE PIXELS` uses the same scale on both axes. The throwaway test's version, trimmed to the decision:

  ```rust
  // from the throwaway test (2026-11-03); gw, gh = 384, 224
  Square    => { let s = (w / gw).min(h / gh).max(1); (gw * s, gh * s) }
  Integer43 => { /* largest sy fitting h; sx = round(sy * 7/9); step sy down until gw*sx <= w */ }
  True43    => { /* largest sy with gh*sy <= h and gh*sy*4/3 <= w */ (gh * sy * 4 / 3, gh * sy) }
  ```

- **Input.** Keyboard events from winit are translated into the host's existing key vocabulary, so binding names and `bindings.toml` don't change. `frame_inputs_from` already takes a key-down closure, and the test drove the real session through it. Window focus comes from winit. Gamepads stay on gilrs. Keeping `minifb::Key` or moving to a host-owned enum with the same names is the builder's call.
- **Look catalogue (host).** A new host module that discovers looks in order: `OFF`, built-in, saved (`%APPDATA%\sf1-port\shaders\saved\`), then drop-in (`%APPDATA%\sf1-port\shaders\`). It's re-read when the Video tab opens. It lists a look's parameters through librashader's parameter metadata and writes saved looks. A saved look is a `.slangp` in RetroArch's format holding its base look's built-in id, or a path relative to the shaders folder (never absolute), plus parameter overrides.
- **Look loading.** Off the render thread, so the previous look keeps drawing until the new chain is ready; the test saw stalls of up to 1.4 s on the render thread. If a load fails, the look falls back to `OFF`, the first error goes on the notice line, and the error is logged.
- **Built-in looks.** Each is a `.slangp` referencing vendored shaders, with parameters set. `ARCADE CABINET` (Guest advanced, the default), `CONSUMER TV` (Guest NTSC), `PRO MONITOR` (CRT-Royale), and `CURVED TUBE` (crt-geom) only if it's fixed. Final names, tuning and order are the builder's to propose and the user's to approve by eye. `CONSUMER TV` ran at about 47 fps on the 780M at true 4:3. It must reach the ship bar through the Guest fast NTSC variants, lower NTSC-pass resolution, or trimmed passes, or else ship tagged `HEAVY`. `crt-geom` ran at about 3 fps through librashader's wgpu backend. It ships if the cause is found and fixed; otherwise it's left out with the reason recorded.
- **Ship bar (user, 2026-11-03).** The level the user played on `crt-guest-advanced`: about 117–120 presents and 58.5–60 game frames per second at 120 Hz, with a handful of late refreshes per 5 s. A built-in look clearly below it doesn't hold full speed. Cutting the leftover late refreshes further is ongoing work, not a gate. Possible fixes include reused render targets and a render thread free of loads. Exclusive fullscreen may be tried, and ships only as a third `MODE`, `EXCLUSIVE`, if it measurably helps.
- **Vendored shaders.** The chosen files from `libretro/slang-shaders`, pinned (the test used `53deb2ef`), go in a `shaders/` folder at the repository root with a README naming the source, the commit and each file's license. Each file keeps its header. The release build copies the folder beside the game executable. The Guest, Royale and Geom shaders are GPL v2-or-later and the shared includes are MIT. They're separate data files compiled at runtime. The repository is private, and the licensing check waits for the distribution decision (ADR-0010).
- **Settings (engine).** The Video tab's rows are `MODE` (`FULLSCREEN` / `WINDOWED`, default `FULLSCREEN`), `WINDOW SCALE` (1× up to the largest that fits, default 3×), `ASPECT` (default `TRUE 4:3`), `CRT LOOK` (default `ARCADE CABINET`), `TUNE CRT LOOK >`, `VSYNC` (default `ON`), `FRAME DELAY` (default `0MS`) and `RESET TAB`. They're grouped under `DISPLAY`, `MONITOR` and `MOTION`, with a notice line for the refresh rate. Every row applies at once, so the window is no longer reopened when Settings is left. Settings are still saved on leaving the screen.
- **First launch** is borderless fullscreen with `ARCADE CABINET` at `TRUE 4:3`. This amends front-end spec story 5 ("3× scale, windowed").
- **`settings.toml`** keeps `scale` (its range widens past 4) and `fullscreen`, and gains `aspect` (`true_4_3` / `integer_4_3` / `square`), `crt_look` (`off`, a built-in id, or a path relative to the shaders folder), `vsync` and `frame_delay_ms`. Missing keys take the new defaults. Unsaved tuning isn't persisted.
- **Session interface.** The session's frame output carries the current video state (aspect, look, vsync, frame delay, mode, window scale) for the host to apply as it changes. The tuning screen's output also carries a separate panel layer, which the host composites sharp after the filter chain. That second layer is this screen's one exception to "one frame per screen" (ADR-0005). The look catalogue's names and parameter metadata reach the engine through the session, so screens stay pure.
- **Tuning screen (engine).** A new pure screen opened from `TUNE CRT LOOK`, and disabled when the look is `OFF`. Behind it is a looping real-gameplay scene with a stage pan on screen within a few seconds; the builder chooses the source, such as the attract's demo fight or a recorded Local fight. The panel sits on the right in the menus' frame style, and the picture slides to the left edge while it's open. It has the look's name, one `NAME ..... VALUE` row per parameter, scrolling under the shader's own headings with names upper-cased and shortened, then `SAVE AS >` (text entry or an auto-name like `ARCADE CABINET 2`), `RESET LOOK`, and Esc to leave, which asks about unsaved changes. One key hides the panel.
- **Pause box (engine).** A `LOOK` row beside `OVERLAYS` that steps through the same list, shown live behind the dimmed box. A choice there becomes the player's look and is saved like a Settings change. Names are shortened to fit the box's 28 cells. The 1987 cabinet has no pause box.
- **Game binary.** The loop moves onto the new window's event loop with the same session, audio and log. The reopen-on-leave code goes. The smoke subcommand grows a GPU check (below).
- **Dependencies.** winit, wgpu and librashader (wgpu runtime only) go in the host and the game. The test used winit 0.30, wgpu 30 and librashader 0.12.1; pin whatever is current and checked when building.

## Testing Decisions

- **Good tests check behaviour seen from outside**: what a screen shows and returns for given inputs, what a file round-trips to, what rectangle a size produces. They don't check how a module gets there.
- **Seam 1, the game session's step** (inputs in; frame, panel layer and video state out), as every front-end test drives it today: Video tab rows and their values, immediate application (video state changes the frame the row changes), defaults and `RESET TAB`, the notice line for given refresh rates, the pause box `LOOK` row, and the tuning screen (parameter rows, stepping and limits, scrolling, `SAVE AS`, `RESET LOOK`, the unsaved-changes prompt, hiding the panel, disabled when `OFF`). Also the `settings.toml` round trip with the new keys, an old file loading with new defaults, and an unknown `crt_look` falling back. Prior art: the existing front-end settings, menus and pause tests in the engine's test folder, and the settings file tests.
- **Seam 2, host-side pure logic with no GPU**: the picture rectangle for every aspect at 3200×2000, 1920×1080, 2560×1440 and small windows. Look discovery order and file names from a temporary folder, saved-look `.slangp` writing and reading back with relative base references, and the pacing rule's choice of *n* (or the clock) for 60, 120, 144 and 59.94 Hz. Key translation keeping every binding name. Prior art: the host's bindings and files tests, and the art-pack loading tests.
- **Seam 3, the GPU smoke run** on the user's machine: the game's smoke subcommand loads every built-in look and reports load time and any error. It then presents for a set time with each built-in look at true 4:3 fullscreen, reporting presents per second, game frames per second and late refreshes against the ship bar. Prior art: the existing `sf-game smoke` host check and the throwaway test's pacing printout.
- **The existing suites must not move**, the pixel comparisons above all.
- **The user's eye accepts the look**: each built-in look on the user's screen during a stage pan.

## Out of Scope

- Moving the development launcher off minifb.
- Bezels, artwork overlays and reflections in built-in looks. A dropped-in RetroArch preset with a bezel may still work.
- HDR output, variable refresh (FreeSync/G-Sync), and other operating systems.
- Run-ahead and rollback-style latency reduction.
- A separate look for the 1987 cabinet.
- Opening saved looks directly in RetroArch.
- The distribution licensing decision for the GPL shaders.

## Further Notes

- **The throwaway test, 2026-11-03**: winit 0.30, wgpu 30 and librashader 0.12.1 on Vulkan, with the Radeon 780M at 3200×2000, 120 Hz, true 4:3 (2389×1792). Steady results: the sharp pass-through held 120/60. `crt-guest-advanced` held 117–120/58.5–60, `crt-royale-fast` 119/59.6, and full `crt-royale` 120/60 after a 1.3–3.7 s load. `crt-guest-advanced-ntsc` reached about 95/47 and `crt-geom` about 3/1.5. After playing for about 12 minutes, the user said: "Pans felt better", and that they liked Guest advanced and Guest NTSC: "They were all pleasing in their own ways."
- **During an automatic sweep with no input**, the test session reached the Settings screen. The cause wasn't checked; it's worth a look while building the tuning screen's looping scene.
- **The front-end spec's story 5** (first launch "3× scale, windowed") is superseded by story 26 here.

