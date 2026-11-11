# Video options: an arcade-monitor look, smooth motion, and every option worth having

- **Date:** 2026-11-03
- **Status:** approved by the user, 2026-11-03
- **Project:** the video half of the front end's Settings, grown from two rows into a full display system
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`2026-11-02-overhaul-front-end-design.md`](2026-11-02-overhaul-front-end-design.md), [`front-end-spec.md`](../planning/front-end-spec.md), [ADR-0005](../adr/0005-game-binary-pure-screens-shared-host.md), [ADR-0010](../adr/0010-gpu-window-and-retroarch-looks.md)

## 1. Intent

The user wants two things from the Video tab: **the look of an arcade monitor** and **smoother motion**, and beyond those, "all the options I could ever want" (user, 2026-11-03).

- **Motion.** Play feels choppy and stiff today, worst when the stage pans left and right. The user's display runs at 120 Hz and can switch to 60.
- **The look.** CRT shader options on the level of RetroArch's, something more substantial than basic scanlines. Named looks for players who just want to pick one (the user is one of them), fine-tuning for players who want it, and RetroArch preset files dropped in by anyone who has them.
- **4:3.** The 1987 cabinet showed the game on a 4:3 monitor, so true 4:3 is part of the faithful goal. The user made it an exception to the rule that pixel art is only scaled by whole numbers (user, 2026-11-03).

The one test of success the user named: **it has to look great.** Input lag from the CRT look is not a failure, because the look is an option and never forced (user, 2026-11-03).

## 2. Goals, non-goals, anti-goals

### Goals

1. **Even frame pacing**: every game frame is shown for the same number of display refreshes (two at 120 Hz, one at 60 Hz), so pans glide (section 6).
2. **Three aspect choices**: true 4:3, 4:3 on whole-number scales, and square pixels (section 5).
3. **Looks built on RetroArch's own shaders**, run by librashader, with named built-in looks, live tuning, saved custom looks, and drop-in `.slangp` files (section 7).
4. **A live tuning screen**: a demo fight with a stage pan plays behind a side panel of knobs (section 8).
5. **Every video change applies at once**, with no window reopening on leaving Settings (section 9).
6. **One look covers every screen**: the attract, menus, Settings, the select screens and fights (user, 2026-11-03, grilling). The tuning panel's sharp overlay is the only exception (section 8).
7. **The 1987 cabinet shares the player's look and aspect**, as it shares video today. It has no look of its own (user, 2026-11-03, grilling).

### Non-goals

- **Moving the development launcher (`sf-dev`) off minifb.** It keeps its window; see section 11. *My call*, to keep this project to the shipped game.
- **Bezels, artwork overlays, or reflections around the picture.** RetroArch presets that draw bezels may still work as drop-ins, but no built-in look ships one. *My call*; the user asked for the monitor look, not cabinet art.
- **HDR output**, **variable refresh (FreeSync/G-Sync) support**, and **other operating systems**. Windows stays the first target (project brief).
- **Run-ahead or rollback-style latency reduction.** Frame delay (section 6) is the only latency tool here.

### Anti-goals (failure even if it works)

- **A CRT look that doesn't look great** on the user's 3200×2000 screen. The user judges this by eye; numbers do not settle it.
- **A built-in look that cannot hold full speed** on the user's machine (a Radeon 780M at 3200×2000, 120 Hz) and is offered as if it could. Full speed means the ship bar in section 12.
- **Any change to the game's own pixels or timing.** Visual fidelity is measured on the 384×224 frame before display scaling (`CONTEXT.md`); shaders and pacing only change what reaches the screen. The comparison tests must not move.
- **A broken bindings file.** Existing `bindings.toml` files keep loading and meaning the same keys.
- **A drop-in look that crashes the game.** A file that fails to load falls back to `OFF` with a notice.

## 3. What exists today (found while designing)

- **The window** is minifb (`crates/sf-host/src/window.rs`). `GameWindow` upscales the frame on the CPU by a whole number into a buffer and calls `update_with_buffer`. Pacing is `set_target_fps(60)`, a sleep, not a wait for the display's refresh.
- **Pacing was measured** at a mean frame time of 17.11 ms, about 58.4 fps, under that sleep (`docs/reference/launcher.md`). At 120 Hz that shows frames for an uneven mix of two and three refreshes, with one held over about once a second. That is the choppiness the user sees on pans.
- **The Video tab** has two rows, `SCALE` (1× to 4×) and `FULLSCREEN` (borderless at the monitor's size, largest whole-number scale that fits, centred on black) (`crates/sf-engine/src/front/settings.rs`). They are stored in `settings.toml` as `scale` and `fullscreen` and applied by reopening the window when Settings is left (`crates/sf-game/src/main.rs`).
- **The front-end design ruled out** a CRT or scanline filter and kept `VSYNC` only "if the window library exposes it" (`2026-11-02-overhaul-front-end-design.md`). This design reverses the first and delivers the second.
- **The game runs at 60 frames a second, as the reference does.** The MAME reference captures put frame 2389 at 39.800 s and frame 3830 at 63.840 s (`docs/reference/both-chips.md`), which is 60 Hz. So no separate "arcade frame rate" option is needed. *My call*; I proposed one earlier and the reference makes it pointless.
- **Keys are minifb's `Key` type** throughout the host: `bindings.rs` stores key names (`"A"`, `"Key1"`…) and `input.rs` reads them. `sf-dev` and `sf-game/tests/pad_play.rs` use the same type.
- **Distribution is a deferred decision**; initial use is personal and local (`docs/project-brief.md`). The engine and audio crates are MIT.

## 4. Approaches

Three ways were considered (2026-11-03):

1. **A graphics-card window plus RetroArch's shader engine** (chosen). Replace minifb in the game with winit and wgpu, and run shaders through librashader, which runs RetroArch's `.slangp` presets. The built-in looks would be tuned versions of RetroArch's best-regarded CRT shaders, and drop-ins would run through the same engine, so anything a player drops in works. Vsync and even pacing come with the new window. It's the biggest change to the host, the shaders are mostly GPL, and librashader's wgpu backend is newer than its Direct3D one.
2. **The same window plus our own CRT shader**, with librashader only for drop-ins. That would be fully ours with no licensing question for the built-ins, but realistically it wouldn't match years of work on the Guest or Royale shaders, which misses the user's one test. It also means two shader paths.
3. **Keep minifb and do it in software.** This was dropped. At 3200×2000 it can't produce the look, it can't run RetroArch files, and it doesn't fix the pacing.

The user took the recommendation, approach 1, and agreed to a throwaway test first.

### What the throwaway test showed (2026-11-03)

The test was a scratch crate outside the workspace. It played the real game through `GameSession` in a winit + wgpu 30 window with librashader 0.12.1, borderless fullscreen at 3200×2000, 120 Hz, true 4:3 (2389×1792), on the Radeon 780M using Vulkan. The shaders came from `libretro/slang-shaders` at commit `53deb2ef` (2026-11-02).

| Preset | Load | Steady pacing (5 s windows with no load or capture) | Verdict |
|---|---|---|---|
| sharp pass-through (no CRT) | 0.3 s | 120 presents/s, 60 game fps, ~1 late refresh | baseline |
| `crt-guest-advanced` | 0.6–0.8 s | 117–120/s, 58.5–60 fps, 1–5 late (6–11 in a few busy stretches) | user liked it |
| `crt-guest-advanced-ntsc` | 0.4–0.7 s | ~95/s, ~47 fps | user liked it; **too heavy as is** |
| `crt-royale-fast` | 0.3–0.5 s | 119/s, 59.6 fps, 1–2 late | holds |
| `crt-royale` (full) | 1.3–3.7 s | 120/s, 60 fps, 1 late | holds |
| `crt-geom` | 0.1–0.6 s | ~3/s in every aspect mode | **broken here**; cause unknown |

- **The user played for about 12 minutes**: "Pans felt better", and "I did like guest-advanced but also guest-ntsc. They were all pleasing in their own ways" (user, 2026-11-03).
- **A few refreshes still arrive late**, about one every 5 seconds at best and up to ten in busy stretches. The cause wasn't found. It could be the desktop compositor in borderless fullscreen, game-step time variance, or the test's per-frame texture allocation.
- **Loading a look stalls the picture** for up to 1.4 s (full Royale) on the render thread.
- **The test had a bug with vsync off**: its game clock wasn't reset when vsync was switched, so the game ran fast. Nothing about vsync off was learned.
- **The test also strayed into the Settings screen** during an automatic sweep with no input sent. The cause wasn't checked; it's worth a look while building the tuning screen's demo.

## 5. The picture: aspect and scale

The game frame is 384×224. On the 1987 cabinet's 4:3 tube each pixel was 7/9 as wide as it was tall: (4/3) ÷ (384/224) = 0.778.

| `ASPECT` | Vertical scale | Horizontal | At 3200×2000 | Notes |
|---|---|---|---|---|
| `TRUE 4:3` (default) | largest whole number that fits | width = height × 4/3, not a whole multiple | 8× tall, 2389×1792 | The user's exception (section 1). The shader resamples horizontally, as a tube had no horizontal pixels. |
| `4:3 INTEGER` | largest whole number that fits | nearest whole number to 7/9 of the vertical | 8× tall, 6× wide, 2304×1792 | about 4% short of 4:3; every pixel crisp |
| `SQUARE PIXELS` | largest whole number that fits | same as vertical | 8×, 3072×1792 | today's look |

- **The vertical scale is always a whole number**, so scanlines are evenly spaced.
- **The picture is centred on black** in every mode.
- **`WINDOW SCALE`** sets the vertical scale when windowed, from 1× up to the largest that fits the monitor's work area. Today the range is 1× to 4×. The window's width follows `ASPECT`. *My call*: 4× on a 3200×2000 screen is less than half its height.
- **With `CRT LOOK` off**, `TRUE 4:3` is drawn with sharp, nearest-neighbour vertical scaling and the smoothest even horizontal resampling the builder finds. Slight unevenness is accepted there; the user chose this exception.

## 6. Motion

- **Present on vsync** (wgpu `Fifo`). One game frame advances every *n* refreshes, where *n* = refresh ÷ 60, when the display's refresh is within 1% of a multiple of 60. At 120 Hz each game frame is shown for exactly two refreshes. That is the rule the test used.
- **Displays that aren't a multiple of 60** (144 Hz, 165 Hz) can't show 60 fps evenly. The game advances on a 60 Hz clock and the Video tab shows a notice, for example `144HZ: SET YOUR DISPLAY TO 120 OR 60 FOR SMOOTH MOTION`. *My call*; variable refresh is a non-goal.
- **`VSYNC OFF`** presents as fast as possible (wgpu `Immediate`), with the game on a 60 Hz clock that is reset whenever pacing changes, and tearing allowed. It's for players who want the lowest latency.
- **`FRAME DELAY`** waits a set number of milliseconds after each vblank before stepping the game and rendering, as RetroArch's option does, to cut input lag. Its range is from 0 up to what the measured headroom allows, and the default is 0. *My call* on the default; the builder sets the cap from measurement on the 780M.
- **The game's speed and logic do not change.** Only when frames reach the screen changes.
- **Smoothness has to hold up**: the leftover late refreshes the test saw are to be investigated and reduced (section 12). Possible fixes include reusing render targets and keeping the render thread free of loads.
- **Exclusive fullscreen** may be tried as part of that investigation. It ships only as a third `MODE` value, `EXCLUSIVE`, and only if measurement shows it reduces the lateness. `FULLSCREEN` stays borderless and stays the fullscreen default, because exclusive mode's slow, flickering switches and alt-tab behaviour break the "applies at once" rule. *My call*, after the builder check.

### As built (VO11, #214): the late refreshes

Measured on 2026-11-05 with `sf-game smoke` on the user's machine (Radeon 780M, 3200×2000, 120 Hz, borderless fullscreen, Vulkan), quiet: no builds or other agents, CPU 4.5-14.6% busy in the seconds before each run (not sampled before the second baseline run). Each figure is presents a second / game frames a second / late refreshes, over 5 s.

**Two causes were found.**

1. **The late count overcounted.** It rounded each gap between presents to whole refreshes. The times are taken when a frame is acquired, and with frames queued that wobbles: a gap of one and a half refreshes is often followed by one of half a refresh, and nothing was missed. The sharp picture held 120.0 presents a second, a present for every refresh, and still counted 40-52 late. The count now adds the gaps up in refreshes against the presents and counts a refresh late once the sum is a whole refresh behind; early presents make up for at most three (`PresentStats`, `crates/sf-host/src/pacing.rs`). The smoke run also prints the refreshes in the run less the presents as a check, and the two agree to within one.
2. **A look's present missed its refresh with two frames queued.** The chain runs once per game frame, so at 120 Hz one present carries the chain and the next only the copy. Through `ARCADE CABINET` the window held about 94 presents a second (about 47 game fps), with 131 refreshes in 5 s going by without a present. The GPU time fits easily: offscreen on the quiet machine, a game frame through `ARCADE CABINET` took 4.36 ms of GPU time (median; 9.04 ms p95, 12.21 ms longest) and a plain present 1.00 ms, against 16.7 ms for the pair. Running the chain on every present instead, twice the GPU work, held 115.8 presents a second. So the GPU most likely clocks down between the bursts, and a chain started on a slowed GPU misses its refresh. With three frames queued the GPU has work waiting more often and each frame has a refresh longer before it's due, and every look holds the ship bar. **The fix:** the swapchain queues three frames while a look draws and two with the sharp picture (`frame_latency`, `crates/sf-host/src/gpu_window.rs`), changed in place when a look loads or goes. The cost is up to one more refresh of input lag (8.3 ms at 120 Hz) with a look on, which section 1 accepts for the look.

**Before** (as VO10 left it; the late count as it was, with the refreshes less presents in brackets where measured):

| Run | Sharp | Vsync off | `ARCADE CABINET` | `CONSUMER TV` | `PRO MONITOR` | `CURVED TUBE` | `ARCADE CABINET`, 2 ms delay (4 s) |
|---|---|---|---|---|---|---|---|
| 1 | 120.0 / 60.00 / 40 | 736.8 / 59.24 | 94.6 / 47.32 / 186 | 101.0 / 50.51 / 203 | 113.8 / 56.91 / 100 | 111.6 / 55.78 / 102 | 62.5 / 31.25 / 230 |
| 2 | 119.5 / 59.76 / 52 | 751.6 / 59.30 | 95.2 / 47.61 / 197 | 93.4 / 46.69 / 248 | 115.4 / 57.72 / 96 | 112.6 / 56.32 / 96 | 62.0 / 31.00 / 233 |
| 3 | 120.0 / 60.00 / 42 (0) | 736.6 / 59.25 | 93.9 / 46.96 / 208 (131) | 94.4 / 47.20 / 244 (128) | 116.0 / 58.01 / 97 (20) | 114.3 / 57.14 / 98 (29) | 62.2 / 31.11 / 230 (232) |

**After** (three queued with a look, two sharp, the new late count; the delay run at 1 ms in runs 1 and 2, and at the final cap of 0 ms in run 3):

| Run | Sharp | Vsync off | `ARCADE CABINET` | `CONSUMER TV` | `PRO MONITOR` | `CURVED TUBE` | `ARCADE CABINET` with the delay (4 s) |
|---|---|---|---|---|---|---|---|
| 1 | 120.2 / 60.10 / 1 | 737.8 / 59.27 | 117.9 / 58.94 / 11 | 117.3 / 58.63 / 15 | 119.7 / 59.84 / 1 | 118.2 / 59.10 / 9 | 1 ms: 117.0 / 58.48 / 12 |
| 2 | 120.0 / 60.00 / 0 | 728.7 / 59.31 | 117.6 / 58.80 / 12 | 117.4 / 58.70 / 13 | 118.6 / 59.30 / 7 | 118.2 / 59.10 / 9 | 1 ms: 116.8 / 58.40 / 13 |
| 3 | 120.0 / 60.00 / 0 | 756.4 / 59.23 | 116.3 / 58.16 / 18 | 117.5 / 58.76 / 12 | 118.9 / 59.43 / 7 | 118.3 / 59.16 / 8 | 0 ms: 116.8 / 58.42 / 13 |

At true 4:3 through `ARCADE CABINET`, refreshes going by without a present fell from 131 to 11-18 in 5 s, and presents rose from about 94 to 116-118 a second. That sits at the ship bar's line rather than clear of it: of the seven 4-5 s runs through `ARCADE CABINET` with three queued and no delay (these three, run 3's 0 ms delay run, the three-queued experiment below, and the sweeps' 0 ms rows in section 9), five reached 58.5 game fps and two didn't (58.16, 58.42); the range is 116.3-117.9 presents a second. **The user accepts `ARCADE CABINET` as it is (2026-11-05)**: it stays the default with its tuning, sitting at the ship bar (58.2-58.9 game fps, 58.5 or better in five runs of seven), and the smoke check may now and then fail on it. Neither the look nor the bar changes. Cutting the lateness further stays open work.

**What else was tried** (one smoke run each, quiet machine, presents a second):

| Experiment | Sharp | `ARCADE CABINET` | `CONSUMER TV` | `PRO MONITOR` | `CURVED TUBE` | Finding |
|---|---|---|---|---|---|---|
| The chain on every present, two queued | 119.8 | 115.8 | 81.0 | 115.1 | 118.4 | More GPU work held better, except `CONSUMER TV`, too heavy to run twice per game frame. Not kept: it doubles the GPU's work. |
| Three queued for everything | 120.0 | 117.0 | 117.5 | 118.6 | 118.7 | The fix, kept for looks. |
| One queued for everything | 109.5 | 65.4 | 50.2 | 64.6 | 81.3 | Even the sharp picture misses refreshes; see `FRAME DELAY` in section 9. |
| No validation error scope around each look present | 120.0 | 95.2 | 96.1 | 116.7 | 113.0 | No change; the scope stays. |
| Direct3D 12 instead of Vulkan | 120.0, 0 late (old count) | fails to load | fails to load | fails to load | fails to load | The sharp picture paces perfectly, but every look fails to load through librashader's wgpu runtime on Direct3D 12 ("wgpu error: Validation Error"). Not pursued; worth a look if Vulkan ever misbehaves. |
| Exclusive fullscreen | — | — | — | — | — | Not measurable: winit 0.30.13 panics switching to the monitor's own 3200×2000, 120 Hz, 32-bit mode (`ChangeDisplaySettingsExW` returns -1, `DISP_CHANGE_FAILED`). With no measured improvement, `EXCLUSIVE` is **not** added; `FULLSCREEN` stays borderless and the default. |

- **Render targets are already reused.** The picture texture is made once per rectangle size, and librashader keeps its own pass targets between frames, so no per-frame allocation was found to remove.
- **Loads are already off the render thread** (VO4's loader thread). What's left on it at a switch is making the look's composite pipeline and the chain's first frame, most likely where the smoke run's longest present (43-91 ms, first seen after a look switch) comes from; not checked further. It would be a hitch when switching looks, not steady pacing.
- **Vsync off** holds 59.24-59.31 game fps over the 5 s counted from the switch, inside the bar. The shortfall from 60 wasn't looked into.
- **Known issue on other platforms (review, 2026-11-05; not fixed).** A redraw holds the frame's surface texture while it steps the game (`gpu_window.rs`, `redraw` → `step` → `Gpu::apply`). If a mode or scale change there makes winit's `request_inner_size` return the new size at once, `resize` reconfigures the surface while that texture is held, which could panic. On Windows winit returns `None` and the size arrives later through `Resized`, so it can't happen there; on other platforms it is untested.

## 7. The CRT look

### Built-in looks

`CRT LOOK` lists, in order: `OFF`, the built-in looks, the player's saved looks, then drop-in files.

| Name (proposed) | Built on | Status |
|---|---|---|
| `OFF` | sharp pass-through | holds |
| `ARCADE CABINET` (default) | `crt-guest-advanced`, tuned toward a 15 kHz arcade monitor | holds; the user liked the base |
| `CONSUMER TV` | `crt-guest-advanced-ntsc` (composite colour and blur) | the user liked it; **must be brought to full speed** (below) |
| `PRO MONITOR` | `crt-royale` (aperture grille, phosphor glow) | holds |
| `CURVED TUBE` | `crt-geom` | **only if fixed**; it ran at 3 fps here |

- **Final names, tuning and order are the builder's to propose and the user's to approve by eye.** Each named look is a `.slangp` that references the vendored shaders, with its parameter values set.
- **`CONSUMER TV` at full speed.** It ran at ~47 fps at true 4:3 on the 780M. The builder tries the Guest "fast" NTSC variants, lowering the internal resolution of the NTSC passes, or trimming passes. If none holds 60, it ships tagged `HEAVY` in the list with the reason in the notice line, not as a normal look. *My call*, from the anti-goal.
- **`CURVED TUBE`**: find out why `crt-geom` runs at 3 fps through librashader's wgpu backend. Possible causes are the naga translation, a mipmap or feedback setting, or a bug worth reporting upstream. Ship it if it's fixed and looks good; otherwise leave it out and record why.
- **Every built-in look is checked by running it**, not assumed to work (section 12).
- **What the build found (VO5, #208, 2026-11-04).** Measured offscreen on the 780M at true 4:3 (a 2389×1792 picture in a 3200×2000 target), GPU time per game frame (chain and composite) and per plain present, median on a quiet GPU: `ARCADE CABINET` 7.3 ms, `CONSUMER TV` 8.9 ms, `PRO MONITOR` (full Royale) 6.9 ms, `CURVED TUBE` 4.4 ms, a plain present 1.2 ms. At 120 Hz a game frame has 16.7 ms for its chain and two presents, so all four fit, `CONSUMER TV` with the least room. The machine was shared with other builds, and timings swung by up to 2× between runs. The test's ~47 fps for Guest NTSC was most likely its renderer running the chain on every present, where the game's window runs it once per game frame. The real-window check is `sf-game smoke`, which paces every built-in look.
- **In the real window (VO11, #214, 2026-11-05).** At true 4:3 in borderless fullscreen on the user's machine, with three frames queued (section 6), over 5 s in three smoke runs: `CONSUMER TV` 117.3, 117.4 and 117.5 presents a second (58.63, 58.70, 58.76 game fps), `PRO MONITOR` 119.7, 118.6 and 118.9 (59.84, 59.30, 59.43), `CURVED TUBE` 118.2, 118.2 and 118.3 (59.10, 59.10, 59.16). Those three hold the bar in every run. `ARCADE CABINET` gave 117.9, 117.6 and 116.3 (58.94, 58.80, 58.16): it sits at the line, reaching 58.5 in five of seven runs (section 6), and the user accepts it so (2026-11-05). None is tagged `HEAVY`: none is clearly below the bar. Before the fix, with two queued, none held it (93.4-116.0 presents a second).
- **`crt-geom`'s 3 fps (VO5).** The cause is its mask function. It declares a lookup table per mask pattern (`vec3 slotmask[3][4]`, up to `[6][14]`) as a function-local array and indexes it by pixel position. Through librashader's wgpu runtime (SPIR-V through naga), every pixel pays for every table, in proportion to its size, whichever pattern is chosen. The whole shader took about 340 ms a frame, and one 3×4 table alone added about 6 ms. A copy with the table patterns cut (`shaders/looks/curved-tube/crt-geom-lean.slang`) takes about 4 ms, so `CURVED TUBE` ships on it. It isn't known whether naga or the driver is at fault, so nothing has been reported upstream yet; a native Vulkan run of the original would settle that.

### Drop-ins and saved looks

- **Drop-ins**: `.slangp` files (with their shader folders) placed in `%APPDATA%\sf1-port\shaders\` appear in the list under their file name, after the saved looks. The list is re-read when the Video tab opens. *My call* on the folder, beside `settings.toml`.
- **How drop-ins are found (VO7, #210, the builder's call).** Upstream presets reach outside their own folder (Royale's `../blurs/`, every pass's `../../include/`), so they only work with upstream's layout kept. The folder is therefore searched down to six levels deep, skipping `saved\`, and `crt\crt-royale.slangp` beside `include\` and `blurs\` works as dropped. The id stored in `crt_look` is the path relative to the folder, with `/`. The name is the file name without `.slangp`, upper-cased with `_` as a space, keeping only what the ROM font draws. If two drop-ins share a name, or one takes a built-in look's name, they show their relative paths instead. A drop-in at the folder's top has no folder to show, so a name still taken gets ` (DROP-IN)`, then ` (DROP-IN 2)` and so on, in path order (review fix, 2026-11-05). The list is sorted by path, ignoring case. It's re-read each time the Settings screen opens. Switching tabs inside Settings doesn't re-read it.
- **Saved looks**: `SAVE AS` on the tuning screen writes a `.slangp` in RetroArch's format, holding the base preset's id and the overridden parameters. It goes in `%APPDATA%\sf1-port\shaders\saved\` and is named with the menu's text entry or an auto-name like `ARCADE CABINET 2`. The base is referenced by the game's own built-in id or a path relative to the shaders folder, never an absolute path, so a saved look survives the game being moved or reinstalled. It won't open directly in RetroArch; nobody asked for that. Built-ins never change. *My call*, after the builder check.
  - *As built (the maintainer's check, 2026-11-06)*: the keyboard types the name. While name entry is open, each key's text goes in at the place (letters upper-cased, characters a file name can't hold left out). Backspace and Delete erase, Home and End jump, the arrows move and step, Enter saves and Esc cancels. The keyboard's menu keys and fighter keys stand down meanwhile, so Backspace no longer cancels and LP no longer confirms. A pad still steps the characters as the arcade's initials do.
- **A look that fails to load** (missing file, shader error) falls back to `OFF`, shows the first error in the notice line, and logs it.
  - *As built (review fix, 2026-11-05)*: the fallback is for the run only and is never saved. `settings.toml` keeps the look the player chose, with Settings open or not, so the next launch tries it again and shows the notice again if it still fails. The Video tab and the pause box show `OFF` meanwhile; stepping `CRT LOOK` (or the pause box's `LOOK`) tries the look stepped to afresh. A failure before the session starts (on the ROM-missing screen) reaches the session as it starts. The notice is put in the ROM font's characters (`_` as a space, `\` as `/`, anything else it lacks as `?`) and cut to the screen's width.
- **Loading runs off the render thread**, so the picture never freezes. The old look stays until the new one is ready. The test saw stalls of up to 1.4 s on the render thread.

### Shipping the shaders

- **Vendor the chosen files** from `libretro/slang-shaders` at a pinned commit (the test used `53deb2ef`) into the repo, under a `shaders/` folder beside the workspace. Add a README naming the source, the commit, and each shader's license. Each file keeps its own header. The release build copies `shaders/` beside `sf-game.exe`. The repository is private, and the user chose vendoring over a local pack or a fetch at build time (user, 2026-11-03, grilling). It keeps the build offline and the exact versions tested. Recorded in [ADR-0010](../adr/0010-gpu-window-and-retroarch-looks.md).
- **Licenses**: the Guest, Royale and Geom shaders are GPL v2-or-later; the shared `include/` files are MIT (as found 2026-11-03). They are separate data files compiled at runtime, not linked into the MIT crates. Initial use is personal and local, so this doesn't block. **When distribution is decided**, this needs a proper check then (project brief).

## 8. The tuning screen

The user chose a live scene behind a side panel (option A, 2026-11-03).

- **Opened by `TUNE CRT LOOK >`** on the Video tab. Disabled when `CRT LOOK` is `OFF`.
- **Behind the panel**: real gameplay that loops and includes a stage pan, drawn through the current look at the current aspect, full screen. The source is the builder's call. Options include the attract's demo fight or a short recorded Local fight played back. *Left to the builder*, with the requirement that a pan is on screen within a few seconds of opening.
- **The panel is drawn sharp, on top of the shaded picture, not through the shader.** The engine returns the scene frame and the panel as a separate layer, and the host composites the panel after the filter chain at an integer scale of the ROM font. That way the text stays legible and the panel never stands in for what is being judged. *My call*, after the builder check: drawn inside the 384×224 frame, the panel would get scanlines, cover half the picture, and leave about 24 columns for names. This is a deliberate exception to "one frame per screen" (ADR-0005), and it applies to this screen only.
- **The panel takes the right-hand side.** While it is open, the shaded picture moves to the left edge of the screen, so at true 4:3 on 3200×2000 the panel fills the black margin plus a strip of the picture, not half of it. *My call*.
- **The panel's contents**: framed in the menus' style, it shows the look's name, then one row per shader parameter (`NAME ..... VALUE`), stepped with left and right by the parameter's own step, between its own minimum and maximum. Parameters come from the preset (librashader reads them, including a drop-in's), so a drop-in gets its own knobs.
- **Long parameter lists** (Guest has well over a hundred) scroll, grouped under the shader's own section headings where it declares them. Names are upper-cased in the ROM font and shortened to fit. *Left to the builder* to keep them legible.
- **Every change shows at once** on the scene behind.
- **Rows at the foot**: `SAVE AS >`, `RESET LOOK` (back to the look's values), and Esc to leave. Leaving keeps the changes as the current look for this session. On leaving, an unsaved change asks whether to save it as a new look. *My call*.
- **The panel can hide** with one key, to judge the look across the whole screen. *My call*.
- **Built as a pure state machine** in `sf-engine`, like every other screen (ADR-0005), with the host doing the drawing through the shader.

## 9. The Video tab

| Row | Values | Default |
|---|---|---|
| `MODE` | `FULLSCREEN` / `WINDOWED` | `FULLSCREEN` (borderless) |
| `WINDOW SCALE` | 1× to the largest that fits | 3×, as today |
| `ASPECT` | `TRUE 4:3` / `4:3 INTEGER` / `SQUARE PIXELS` | `TRUE 4:3` with `CRT LOOK` on |
| `CRT LOOK` | `OFF`, built-ins, saved, drop-ins | `ARCADE CABINET` |
| `TUNE CRT LOOK` | `>` | — |
| `VSYNC` | `ON` / `OFF` | `ON` |
| `FRAME DELAY` | `0MS` up to the cap | `0MS` |
| `RESET TAB` | — | — |

- **The rows are grouped** under `DISPLAY`, `MONITOR` and `MOTION` headings, as in the 2026-11-03 mockup.
- **A notice line** at the foot names the display's refresh rate and what it means, for example `120HZ: EVERY FRAME SHOWN TWICE`, or the warning in section 6.
- **First launch** opens borderless fullscreen with the look on (`ARCADE CABINET`, `TRUE 4:3`) (user, 2026-11-03, grilling). A 3× window is 896 pixels tall, too few lines for masks and scanlines to look right. This replaces front-end spec story 5 ("3× scale, windowed"), which the spec for this project must amend. These are video settings, so the "every toggle off" faithful defaults for gameplay are untouched.
- **Every row applies the moment it changes.** Fullscreen and windowed switch in place without reopening the window. Settings are saved on leaving the screen, as today.
- **`RESET TAB`** restores these defaults.

### As built (VO6, #209)

- **Order and groups.** `DISPLAY`: `MODE`, `WINDOW SCALE`, `GAUGE ART` (the era HUD's row, kept on this tab). `MONITOR`: `ASPECT`, `CRT LOOK`, `TUNE CRT LOOK >` (dim until its screen lands). `MOTION`: `VSYNC`, `FRAME DELAY`. Then `RESET TAB`. *Builder's call* on which group takes `ASPECT` and `GAUGE ART`.
- **`WINDOW SCALE`** runs from 1× to the largest vertical scale whose picture fits the monitor with 120 pixels left for the title bar and taskbar (the window library gives the monitor's size, not its work area). The host reports it each frame; `settings.toml` takes `scale` up to 20. A scale kept from a bigger monitor is clamped when the window is sized, and steps down from the new largest.
- **Notice line.** Two lines at most, ending on the row above the preset line: `120HZ: EVERY FRAME SHOWN TWICE`, `60HZ: EVERY FRAME SHOWN ONCE`, `240HZ: EVERY FRAME SHOWN 4 TIMES`, for a rate that is not a multiple of 60 `144HZ: SET YOUR DISPLAY TO 120 OR 60` / `FOR SMOOTH MOTION`, and with vsync off `120HZ, VSYNC OFF: FRAMES SHOWN AS SOON` / `AS READY, TEARING POSSIBLE`. A look that failed to load takes the line over, as before.
- **Vsync** switches the surface between `Fifo` and `Immediate` (else `Mailbox`) in place, before the next frame is acquired, and chooses pacing afresh in the same step, so the 60 Hz clock starts at the switch. The smoke check counts 60 game frames a second from that moment.
- **`FRAME DELAY`** spins for the set milliseconds after the vblank (the swapchain acquire), then reads the keys and steps the game; it applies only with vsync on and only to presents that step a game frame. As VO6 built it, a nonzero delay also made the swapchain queue one frame instead of two; VO11 (#214) dropped that, because one queued frame misses refreshes on the 780M even with the sharp picture and no delay (109.5 presents a second). The delay now keeps the queue it finds: two sharp, three with a look (section 6).
- **The cap depends on the look (user, 2026-11-05, from the VO11 measurements below).**
  - **`CRT LOOK` `OFF`: up to 4 ms** (`FRAME_DELAY_CAP_MS`), the largest delay the sharp picture held in both sweeps (118.5 / 59.25 and 118.0 / 59.02 at 4 ms).
  - **Any look on: 0 ms** (`FRAME_DELAY_CAP_LOOK_MS`). The row shows `0MS`, drawn dim, and left and right change nothing (no `ValueChanged`).
  - **The saved delay is kept.** `frame_delay_ms` in `settings.toml` is the delay the player set; the delay used (`Settings::frame_delay`, and the session's video state) is that, clamped to the cap for the current look. Switching from `OFF` to a look makes the delay 0 and keeps the saved value; switching back to `OFF` uses it again. Stepping the row starts from the delay in use.
  - **A saved delay above 4 ms loads clamped to 4**, with the rest of the file. It is no longer refused, so a cap that moves never resets `settings.toml`. A value that isn't a number is still refused.
  - **The smoke check's delay stage** now runs the sharp picture at 4 ms, since with a look there is no delay to check; `--frame-delay-sweep` is unchanged.
- **What was measured (VO11, #214).** The first rule tried was one cap, the largest delay at which `ARCADE CABINET` at true 4:3 keeps the ship bar. On 2026-11-05, on the quiet machine (CPU 4.5-10.9% busy before each run), presents a second / game fps:

  | Delay | `ARCADE CABINET`, one queued (as VO6 built it) | `ARCADE CABINET`, three queued | same, again | Sharp, one queued (as built) | Sharp, two queued | same, again |
  |---|---|---|---|---|---|---|
  | 0 ms | 94.8 / 47.40 | 117.5 / 58.74 | 117.8 / 58.88 | 119.8 / 59.88 | 120.0 / 60.00 | 119.3 / 59.65 |
  | 1 ms | 63.3 / 31.64 | 117.0 / 58.50 | 116.2 / 58.08 | 100.1 / 50.05 | 119.0 / 59.50 | 119.8 / 59.89 |
  | 2 ms | 63.2 / 31.62 | 106.6 / 53.28 | 113.1 / 56.56 | 96.8 / 48.40 | 119.5 / 59.75 | 119.5 / 59.75 |
  | 3 ms | 58.7 / 29.35 | 101.3 / 50.64 | 98.7 / 49.34 | 90.8 / 45.41 | 119.2 / 59.62 | 118.8 / 59.38 |
  | 4 ms | 55.2 / 27.60 | 88.0 / 43.99 | 90.2 / 45.11 | 83.3 / 41.66 | 118.5 / 59.25 | 118.0 / 59.02 |
  | 5 ms | 53.9 / 26.94 | 80.9 / 40.47 | 81.0 / 40.52 | 75.8 / 37.91 | 116.8 / 58.38 | 117.3 / 58.63 |
  | 6 ms | 50.5 / 25.25 | 77.8 / 38.90 | 78.9 / 39.47 | 74.6 / 37.32 | 113.1 / 56.53 | 114.5 / 57.25 |
  | 7 ms | 49.3 / 24.66 | 78.4 / 39.21 | 78.1 / 39.07 | 73.8 / 36.92 | 114.0 / 56.99 | 115.3 / 57.63 |
  | 8 ms | 48.5 / 24.25 | 77.4 / 38.70 | 76.0 / 37.98 | 68.4 / 34.18 | 110.0 / 55.02 | 109.7 / 54.87 |

  The "0 ms" rows of the as-built columns ran with two queued (the delay only dropped the queue when nonzero). Through `ARCADE CABINET` at 1 ms, the two sweeps and the two full smoke runs gave 58.50, 58.08, 58.48 and 58.40 game fps: held once in four, at the line; with no delay it reached 58.5 in five runs of seven (section 6), so 1 ms costs a little on average (58.37 against 58.64 game fps) where there is none to spare. No delay above 0 holds through a look. The sharp picture alone holds through 4 ms in both sweeps, and 5 ms in one of two. So the user chose the cap by look, above. The earlier runs, on a busy machine, follow.
- **The cap was provisional (VO6).** `sf-game smoke --frame-delay-sweep` (add `--sharp` for no look) runs 0 to 8 ms for four seconds each and reports presents and game frames a second, late refreshes and the work a present took. On 2026-11-04 on the 780M at 3200×2000, 120 Hz, the machine shared with other builds (CPU 35-70% busy, 3D engine 23% before the run), six runs through `ARCADE CABINET` gave 73-95 presents a second at 0 ms (the ship bar is about 117) and 44-62 at 1-8 ms; one run with the sharp picture gave 116 at 0 ms and 94-99 at 1-3 ms, 70-76 from 4 ms. No delay held the bar on a loaded machine, and the look alone (0 ms) was already under it, so those runs say little about headroom. The cap was 2 ms until a run on a quiet machine set it (above).

### The pause box

- **A `LOOK` row** joins the pause box (`crates/sf-engine/src/front/pause.rs`), beside `OVERLAYS`. Left and right step through the same list as `CRT LOOK`, and the fight behind the dimmed box shows each look at once (user, 2026-11-03, grilling).
- **A look chosen there is the player's look**, saved as a Settings change would be. *My call*: two places setting one value shouldn't disagree.
- **Long look names are shortened** to fit the box's 28 cells. *Left to the builder*.
- **The 1987 cabinet has no pause box**, so it has no `LOOK` row; its look is set in Settings.
- **As built (VO8, #211).** `LOOK` sits directly above `GAUGE ART` (the third row, under `OVERLAYS`) in the Local, Arcade and Training boxes. Left and right step the session's look list with wrapping, `OFF` first. Each step is in the very frame that takes it (`video.look`), sets the Settings value and flags the save. Names are upper-cased and cut with `..` to 18 cells, so the label, a gap and the value stay inside the box's 28. The paused frame is not redrawn: the look is drawn by the host, so the fight behind the box is unchanged and the host shows the new look at once.

### Overlays

- **Training's overlays** (hitboxes, frame meter, input display) are drawn in the game frame and pass through the look like everything else (user, 2026-11-03, grilling). A player studying frames can switch the look to `OFF` from the pause box.

## 10. Persistence

`settings.toml` gains these keys, and keeps `scale` and `fullscreen` with their meanings:

| Key | Values |
|---|---|
| `aspect` | `true_4_3`, `integer_4_3`, `square` |
| `crt_look` | `off`, a built-in id, or a path relative to the shaders folder |
| `vsync` | `true` / `false` |
| `frame_delay_ms` | 0 to the cap with `CRT LOOK` off (4); more loads as 4; used clamped to the cap for the look (section 9) |

- **The session's unsaved tuning** isn't persisted; only saved looks are.
- **An old `settings.toml`** without the new keys loads with the new defaults. An unknown `crt_look` (a deleted drop-in) falls back to `ARCADE CABINET` with a log line.
- **`WINDOW SCALE` above 4** is now valid, so the `scale` parse range widens.

## 11. Where the code changes

| Area | Change |
|---|---|
| `crates/sf-host/src/window.rs` | A new GPU window (winit + wgpu, borderless or windowed, vsync and pacing, picture rectangle by aspect, a librashader filter chain, async preset loading, refresh-rate query). The minifb `open_window`/`present` path stays for `sf-dev`. |
| `crates/sf-host/src/input.rs`, `bindings.rs` | Keyboard input from winit events, translated to the existing key vocabulary, so `bindings.toml` and the key names are unchanged. `frame_inputs_from` already takes a `down` closure, which the test used. Window focus also comes from winit. *My call*: keep `minifb::Key` or move to a host-owned key enum with the same names; the builder picks the less disruptive. |
| `crates/sf-host/src/shaders.rs` (new) | Look discovery (built-in, saved, drop-in), `.slangp` writing for `SAVE AS`, parameter listing. |
| `crates/sf-engine/src/front/settings.rs` | The new rows, groups, notice line, defaults and `settings.toml` keys. |
| `crates/sf-engine/src/front/` (new screen) | The tuning screen state machine. |
| `crates/sf-game/src/main.rs` | The loop moves onto the new window's event loop, with the same session, audio and log. The reopen-on-leave code goes. |
| `shaders/` (new) | The vendored shaders and their README. |
| `Cargo.toml`s | `winit`, `wgpu`, `librashader` (wgpu runtime only, as the test configured it) for `sf-host` and `sf-game`. |

The test used winit 0.30, wgpu 30 and librashader 0.12.1. Pin whatever is current and checked when building.

## 12. Testing and acceptance

- **Unit tests** for the picture rectangle in each aspect at several screen sizes (3200×2000, 1920×1080, 2560×1440, small windows), the `settings.toml` round trip and old-file defaults, `.slangp` writing for saved looks, and preset discovery.
- **Screen tests** for the Video tab rows and the tuning screen state machine, without a window (ADR-0005).
- **The existing suites don't move**, the pixel comparisons above all.
- **A smoke check** (`sf-game smoke`, extended) loads every built-in look on the real GPU and reports load time and any error.
- **A pacing check** in the smoke run presents for a set time with each built-in look at true 4:3 fullscreen, and reports presents per second, game frames per second and late refreshes. **The ship bar is the level the user played** on 2026-11-03 with `crt-guest-advanced`: about 117–120 presents and 58.5–60 game frames per second at 120 Hz, with a handful of late refreshes per 5 s. The user ruled that level fine for shipping (user, 2026-11-03). A built-in look clearly below it, like `crt-guest-advanced-ntsc`'s ~47 fps, counts as not holding full speed (section 7). Cutting the leftover lateness further is ongoing work (section 6), not a gate.
- **The user's eye is the acceptance** for the look: each built-in look, on their screen, during a stage pan.

## 13. Left to the builder

- Final look names, tuning and order, for the user to approve by eye (section 7).
- The source of the tuning screen's looping scene (section 8).
- The `FRAME DELAY` cap, from measurement (section 6). *Measured in VO11 (#214)*: 4 ms with `CRT LOOK` off, 0 ms with a look (section 9).
- The key-type choice in the host (section 11). *Chosen in VO2 (#205)*: the host keeps `minifb::Key` as its key vocabulary, and `sf_host::keys` translates winit's physical key codes into it. minifb reads keys by scan code on Windows, and winit's codes are scan-code positions, so every name means the same key on any layout. `bindings.rs`, `sf-dev` and the pad tests are unchanged.
- Fixes for `CONSUMER TV` speed and `crt-geom`, or a recorded reason to leave them out (section 7).
- The cause of the leftover late refreshes (section 6). *Found in VO11 (#214)*: an overcounting late count, and a look's present missing its refresh with two frames queued; see section 6's as-built notes.