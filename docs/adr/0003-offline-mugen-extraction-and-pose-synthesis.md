# Extract M.U.G.E.N frame data and pose keyframes offline into native 1987 overhaul packs

Rather than embedding a runtime M.U.G.E.N `.cns`/`.zss` interpreter or loading third-party `.sff`/`.snd` archives at runtime, the project will use an offline extraction tool (`tools/mugen-extract`) to parse reference M.U.G.E.N character packages into deterministic `sf-overhaul` move tables and pose-conditioning keyframes for `tools/asset-gen`. `tools/asset-gen` then synthesizes authentic `384×224`, 15-color-plus-transparent `xRGB_444` `16×16` 4bpp sprite tiles and 8 kHz MSM5205 ADPCM + YM2151 FM audio packs for the `SF1+` tournament fighters, optional what-if styles, and bonus challengers.

This boundary preserves 60 Hz integer state-snapshot determinism for rollback netplay and Replay Takeover, prevents visual clash from unquantized CPS1/CPS2/CvS sprite rips, and keeps copyrighted third-party `.sff` and `.snd` binaries out of version control and distributed builds.

**Superseded in part by [ADR-0006](0006-rom-only-art-sources.md) (2026-11-02):** fan sprite sets are no longer used as pose references or sources of art. Fan-made move data may still be referenced.
