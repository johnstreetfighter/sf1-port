# Vendored RetroArch shaders

These are the shader files behind the game's built-in looks (ticket #204, spec #203, [ADR-0010](../docs/adr/0010-gpu-window-and-retroarch-looks.md)). They are separate data files, compiled at runtime by librashader. No code in the Rust crates links against them.

- **Source:** https://github.com/libretro/slang-shaders
- **Pinned commit:** `53deb2efcb79f4fc3e94e9602d5974b4b64e8f94` (2026-11-02, "Precompute one_x in Maister NTSC vertex stage (#979)")
- **Layout:** the upstream paths are kept as they are (`crt/`, `blurs/`, `include/`), so every `#reference`, `shaderN`, texture and `#include` resolves exactly as it does upstream. Nothing is renamed or edited. Every file keeps its own header.
- **Scope:** only the files the presets below reach, followed through `#reference`, `shaderN`, textures and LUTs, and `#include` (102 files, about 1.6 MB, plus the three Royale text files below).

## Presets

| Preset | Reaches | Files |
|---|---|---|
| `crt/crt-guest-advanced.slangp` | Guest advanced | 16 |
| `crt/crt-guest-advanced-fast.slangp` | Guest advanced, fast variant | 13 |
| `crt/crt-guest-advanced-fastest.slangp` | Guest advanced, fastest variant | 10 |
| `crt/crt-guest-advanced-ntsc.slangp` | Guest advanced with the NTSC passes | 22 |
| `crt/crt-royale.slangp` | CRT-Royale, full (with `blurs/shaders/royale`) | 38 |
| `crt/crt-royale-fast.slangp` | CRT-Royale, fast (with `blurs/shaders/royale`) | 19 |
| `crt/crt-geom.slangp` | crt-geom | 2 |

The counts are each preset's own closure; the presets share files (`include/`, the Guest stock pass, the Royale blurs), so the folder holds fewer than their sum.

## The game's own looks

`looks/` holds the built-in looks' presets, written for this game rather than copied from upstream (tickets #207, #208). Each sets its parameters over a vendored preset; the same resolve test covers them.

| Preset | Look | Built on |
|---|---|---|
| `looks/arcade-cabinet.slangp` | `ARCADE CABINET` | `#reference`s Guest advanced, tuned toward a 15 kHz arcade monitor |
| `looks/consumer-tv.slangp` | `CONSUMER TV` | `#reference`s Guest advanced NTSC: composite colour, a shadow mask, a gentle curve |
| `looks/pro-monitor.slangp` | `PRO MONITOR` | `#reference`s full CRT-Royale: an aperture grille, tight beams, a flat tube |
| `looks/curved-tube.slangp` | `CURVED TUBE` | one pass, `looks/curved-tube/crt-geom-lean.slang` |

`looks/curved-tube/crt-geom-lean.slang` is the one modified shader here: a copy of `crt/shaders/crt-geom.slang` (GPL v2-or-later, header kept) with the mask patterns that build a lookup table per pixel cut out, because through librashader's wgpu runtime they made the whole shader run at about 3 fps. Its header says what changed and when. The vendored original is untouched.

## Licenses

Read from the files' own headers on 2026-11-04.

| Files | License |
|---|---|
| Guest advanced and fast (`crt/shaders/guest/`) | GPL v2-or-later (header in each pass) |
| CRT-Royale (`crt/shaders/crt-royale/`) | GPL v2-or-later; upstream's `LICENSE.TXT` (GPL version 2) is kept beside it, with its `README.TXT` and `THANKS.TXT` |
| crt-geom (`crt/shaders/crt-geom.slang`) | GPL v2-or-later (header: "version 2 of the License, or (at your option) any later version") |
| Shared `include/*.h` (blur-functions, gamma-management, quad-pixel-communication, special-functions) | MIT (header in each) |
| Royale blur passes (`blurs/shaders/royale/`) and Royale's `src-fast/special-functions.h` | MIT (header in each) |

Files with no license header of their own, so nothing in the file states their terms. They are small helpers that belong to the GPL shaders above; they are listed here rather than assumed:

- `crt/shaders/guest/advanced/stock.slang` and `crt/shaders/guest/fast/stock.slang` (pass-through passes)
- `crt/shaders/guest/advanced/ntsc/ntsc-pass1.slang`, `ntsc-pass2.slang`, `ntsc-pass3.slang` ("based on Themaister's NTSC shader" in the header; upstream does not say more)
- `crt/shaders/crt-royale/user-settings.h`, `src/user-cgp-constants.h`, `src-fast/user-settings.h`, `src-fast/user-cgp-constants.h`, `src-fast/derived-settings-and-constants.h`
- `crt/shaders/crt-royale/src/crt-royale-bloom-approx.slang`, `crt-royale-geometry-aa-last-pass.slang`, `crt-royale-scanlines-horizontal-apply-mask.slang` (they include Royale's headers, which carry the GPL notice)
- `include/compat_macros.inc` (a list of macros)

The Guest, Royale and Geom shaders being GPL v2-or-later and the shared includes MIT matches the spec. The repository is private and use is personal and local. The licensing check waits for the distribution decision (ADR-0010).

## The release build

`crates/sf-game/build.rs` copies this folder to `<target profile folder>/shaders`, so a release build puts it beside `sf-game.exe` (`target\release\shaders`). It runs on every build of `sf-game` and again when anything under `shaders/` changes. To ship the game elsewhere, copy `shaders\` next to the executable.

## Windows long paths

The Royale mask textures have long names, for example `TileableLinearSlotMaskTall15Wide9And4d5Horizontal9d14VerticalSpacingResizeTo64BGRshifted.png`. The longest vendored path is 124 characters from the repository root (`shaders\crt\shaders\crt-royale\...`), so a checkout stays under Windows' 260-character limit when the repository folder's own path is under about 130 characters. Nothing was renamed. If a checkout lives deeper than that, run `git config core.longpaths true` before cloning or checking out.

`.gitattributes` marks this folder `-text` so line endings are never rewritten on checkout and the files stay byte-identical to upstream.

## Updating

Fetch the new commit into a folder outside the repository, copy the files the presets reach, change the commit above, and run `cargo test -p sf-game --test vendored_shaders`. The test parses every preset here and confirms each file it names exists.
