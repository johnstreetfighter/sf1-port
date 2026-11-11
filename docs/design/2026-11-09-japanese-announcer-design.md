# Alternate Japanese-accented announcer: design

- **Date:** 2026-11-09
- **Issue:** `#348` ("Alternate announcer with a Japanese accent (Capcom homage)")
- **Size:** Project (spans the voice pipeline in `tools/voice`, line/voice types in `sf-video`, Settings in `sf-engine`, pack loading in `sf-host`, and latch routing in `sf-game`).

## Intent

Capcom's late-1980s and early-1990s arcade games had two unmistakable ringside archetypes: the gravel-voiced American tournament barker and the Japanese martial-arts tournament announcer calling English phrases ("Round 1! Fight!", "Japan!", "Time up!") with rolled R's and crisp syllable timing through a cabinet speaker.

The overhaul already has the first archetype (`RINGSIDE`), designed to sit beside the 1987 ROM's own "ROUND 1" and "FIGHT!" samples while calling the 22 overhaul moments on a third emulated MSM5205 (ADR 0011). If an optional Japanese-accented announcer only voiced the 22 new call-outs while leaving the 1987 ROM's American voice on "ROUND 1", "FIGHT!", "JAPAN" and "TIME UP!", every round would alternate between two different speakers.

This project adds an optional `JAPANESE` announcer voice as a **complete 35-line package**: in overhaul fights, when `JAPANESE` is selected, one voice calls both the 13 classic 1987 tournament announcements and the 22 overhaul call-outs and combo counts through the third 4 kHz MSM5205 chip.

## Goals

1. **Offer an optional Japanese-accented announcer (`JAPANESE`) alongside the existing announcer (`RINGSIDE`, the default)** on the `SOUND` tab of Settings (user).
2. **A complete 35-line voice package for `JAPANESE`** (user):
   - the **13 classic 1987 ROM announcer lines** (`JAPAN`, `U.S.A.`, `CHINA`, `ENGLAND`, `THAILAND`, `ROUND 1` through `ROUND 5`, `FIGHT!`, `TIME UP!`, `GAME OVER`, sample commands `0x8f`–`0x9b`);
   - the **22 overhaul call-out and combo lines** (`SUPER!`, `PERFECT!`, `PERFECT PARRY!`, `COUNTER!`, `PUNISH COUNTER!`, `CRUSH COUNTER!`, `ALPHA COUNTER!`, `REVERSAL!`, `FIRST ATTACK!`, `BURNOUT!`, `CUSTOM COMBO!`, `5`–`15 HIT COMBO!`).
3. **Keep `RINGSIDE` unchanged** (user): when `ANNOUNCER VOICE` is `RINGSIDE`, the 13 classic announcements continue to play from the 1987 ROM's `audio2` sample ROM on the board's first MSM5205, and the 22 overhaul call-outs play from the root `sf1-port-voice/` pack on the third MSM5205.
4. **Original voice design, never cloned** (user): the `JAPANESE` voice is designed from a text description with ElevenLabs Voice Design (`eleven_ttv_v3`), auditioned through the 4 kHz 4-bit MSM5205 ADPCM codec (`sf-audio`) before takes are picked, and stored in the local `sf1-port-voice` pack beside the ROM set.

## Non-goals

- **Re-voicing fighter combat samples or post-match quotes.** Fighter grunts, hits, K.O. cries, `HADOUKEN` / `SHORYUKEN` / `TATSUMAKI SENPUKYAKU` (`0x81`–`0x8e`, `0x9f`–`0xb9`, `0xbb`–`0xbf`), and the four post-match fighter quote samples (`0x9c`, `0x9d`, `0x9e`, `0xba`) belong to the fighters, not the announcer, and stay on the 1987 ROM's sample chips.
- **Per-side or per-stage announcer selection.** One setting chooses the announcer for the local machine.
- **Adding new call-out events or changing when call-outs trigger.**

## Anti-goals

- **Any change to the faithful cabinet, the attract loop, or fights under the `Faithful 1987` preset.** All three always play the 1987 ROM's own sample commands on the board's two MSM5205s.
- **Any mutation of 68000 work RAM or sound-queue state.** In line with ADR 0004, the 1987 tasks queue and pop sound commands in work RAM (`0xff865e..0xff8681`) identically regardless of which announcer voice is selected.
- **A voice cloned from any real person or ripped from another game.**
- **Silence when the `japanese` voice sub-pack is absent on disk.** If a player selects `JAPANESE` without `sf1-port-voice/japanese/` installed, the 13 classic ROM calls must fall back to the 1987 ROM samples rather than muting the round and fight calls.

## Constraints

- **Hardware voice path (ADR 0011).** The announcer speaks on the third emulated MSM5205 (`crates/sf-audio/src/announcer.rs`) at 4 kHz (`TICK_HZ / 2`), 4-bit OKI ADPCM nibbles, high nibble first, with the same `LPF_ALPHA` (`0.33`) and `ANNOUNCER_ROUTE_GAIN` (`0.35 * 1.15`) as `RINGSIDE`, scaled by the `ANNOUNCER` volume setting (`0..=10`).
- **Crate boundary (ADR 0011).** Only `sf-host` touches voice-pack files on disk (`sf1-port-voice/`). `sf-video` and `sf-engine` never read `.vox` files or manifests; `sf-audio` takes `Nibbles` slices in memory.
- **Local presentation only.** `ANNOUNCER VOICE` is a local setting stored in `settings.toml` (never in `match_config.toml`), so netplay never syncs it and replays/tests of the engine's state machine are unaffected.
- **No secret leakage.** `ELEVENLABS_API_KEY` is read from the process or Windows user environment (`HKCU\Environment`) by `tools/voice/pipeline.py` and never printed, logged or committed.

## Approaches

1. **Subfolder in `sf1-port-voice/` + output-latch interception at the audio boundary (chosen).**
   - `sf1-port-voice/` keeps its existing 22-line `manifest.json` and `.vox` files at the root for `RINGSIDE`, untouched.
   - `sf1-port-voice/japanese/` holds its own `manifest.json` and 35 `.vox` files (the 22 overhaul call-outs plus the 13 classic ROM announcer calls).
   - `AnnouncerLine` (`crates/sf-video/src/announcer.rs`) gains the 13 classic variants (`from_rom_latch(0x8f..=0x9b)`).
   - In overhaul fights (`out.tune_table`), when `ANNOUNCER VOICE` is `JAPANESE` and the loaded `japanese` `VoiceStore` has the clip for `out.sound_latch` in `0x8f..=0x9b`, the host masks the frame's latch to `0xff` (`sound::IDLE`) before `SamplesSound` sees it (the same seam `Deck::latch` uses for Jukebox tracks) and hands the `AnnouncerLine` to `Announcer` on the third MSM5205.
   - *Trade-off:* `AnnouncerLine::ALL` grows from 22 to 35 variants (13 of which are only present in `sf1-port-voice/japanese/`), which `VoiceStore` already supports because missing stems in a pack are simply left `None` and logged only if missing from what that pack expects.

2. **Synthesize a replacement `audio2` ADPCM ROM bank in memory for `0x8f`–`0x9b`.**
   - Patch the 13 sample pointer/length entries in a copy of the 256 KiB `audio2` ROM when `JAPANESE` is active so `SamplesCpu` plays the new clips on MSM5205 #0 while the 22 overhaul lines play on the third MSM5205.
   - *Trade-off:* Splits one announcer across two chips and two volume sliders (`SAMPLES` for the 13 classic calls, `ANNOUNCER` for the 22 overhaul call-outs), allows combat samples on channel 0 (`0x9f` cheer, `0xa0` clap) to cut off the announcer, and is constrained by the 32 KiB bank-0 window of `audio2`.

3. **Gate `bus.queue_sound` inside `crates/sf-engine/src/tasks.rs` for `0x8f`–`0x9b`.**
   - Suppress queuing `0x8f`–`0x9b` in the 68000 sound queue when `JAPANESE` is active and emit `AnnouncerLine` from `tasks.rs`.
   - *Trade-off:* Mutates the 68000 sound-queue indices (`0xff865e`, `0xff8660`) and buffer (`0xff8662..0xff8681`) in work RAM, violating ADR 0004 and changing `vblank_suffix` cycle counts (`held_entry_cycles`), and cannot fall back to the 1987 ROM voice if the voice pack is missing on disk because `sf-engine` does not know whether the pack loaded.

The user chose Approach 1 (our recommendation): it keeps all 35 lines on the third MSM5205 under the `ANNOUNCER` volume knob, leaves work RAM and `tasks.rs` untouched, leaves `RINGSIDE`'s root voice pack untouched, and falls back cleanly to the 1987 ROM voice if `sf1-port-voice/japanese/` is absent.

## What was decided

### 1. The 35 announcer lines

`AnnouncerLine` (`crates/sf-video/src/announcer.rs`) covers all 35 lines. The 22 existing overhaul call-out and combo lines keep their stems, texts, prompts, and priorities (`1..=6`). The 13 classic 1987 ROM announcer lines (`0x8f`–`0x9b`) are added:

| ROM latch | Variant | Stem | Text | Prompt (for `tools/voice/japanese.json`) | Priority |
|---|---|---|---|---|---|
| `0x8f` | `CountryJapan` | `country_japan` | `Japan!` | `[shouting, ringside announcer, Japanese accent, proud, crisp] JAPAN!` | `6` |
| `0x90` | `CountryUsa` | `country_usa` | `U.S.A.!` | `[shouting, ringside announcer, Japanese accent, punchy, crisp] U. S. A.!` | `6` |
| `0x91` | `CountryEngland` | `country_england` | `England!` | `[shouting, ringside announcer, Japanese accent, punchy, crisp] ENGLAND!` | `6` |
| `0x92` | `CountryChina` | `country_china` | `China!` | `[shouting, ringside announcer, Japanese accent, punchy, crisp] CHINA!` | `6` |
| `0x93` | `CountryThailand` | `country_thailand` | `Thailand!` | `[shouting, ringside announcer, Japanese accent, punchy, crisp] THAILAND!` | `6` |
| `0x94` | `Fight` | `fight` | `Fight!` | `[shouting, ringside announcer, Japanese accent, explosive, commanding] FIGHT!` | `6` |
| `0x95` | `GameOver` | `game_over` | `Game over!` | `[shouting, ringside announcer, Japanese accent, final, commanding] GAME OVER!` | `6` |
| `0x96` | `TimeUp` | `time_up` | `Time up!` | `[shouting, ringside announcer, Japanese accent, urgent, sharp] TIME UP!` | `6` |
| `0x97` | `Round1` | `round_1` | `Round 1!` | `[shouting, ringside announcer, Japanese accent, commanding, crisp] ROUND ONE!` | `6` |
| `0x98` | `Round2` | `round_2` | `Round 2!` | `[shouting, ringside announcer, Japanese accent, commanding, crisp] ROUND TWO!` | `6` |
| `0x99` | `Round3` | `round_3` | `Round 3!` | `[shouting, ringside announcer, Japanese accent, commanding, crisp] ROUND THREE!` | `6` |
| `0x9a` | `Round4` | `round_4` | `Round 4!` | `[shouting, ringside announcer, Japanese accent, commanding, crisp] ROUND FOUR!` | `6` |
| `0x9b` | `Round5` | `round_5` | `Round 5!` | `[shouting, ringside announcer, Japanese accent, commanding, crisp] ROUND FIVE!` | `6` |

- **Why `Time up!` for `0x96`:** In the 1987 ROM, while the on-screen text block at `0x14f74` reads `TIME OVER`, the spoken sample at `0x96` (`docs/reference/adpcm-samples.md`) says "Time up!". The announcer speaks the 1987 call ("Time up!").
- **Why priority `6`:** The 13 classic calls sit at the top of `AnnouncerLine::priority` alongside `Super` and `Perfect` (`6`). A lingering mid-round call-out (`Counter`, `FirstAttack`, or a combo count) is cut off immediately when `TIME UP!` (`0x96`) or `ROUND 1` (`0x97`) arrives, and within the same tier an already-playing clip is not cut off by another tier-6 clip (in practice `ROUND n`, `FIGHT!`, `PERFECT!`, `TIME UP!`, and `GAME OVER` are separated by 60–180 frames of round flow).
- **The 22 overhaul call-out prompts in `tools/voice/japanese.json`:** Same stems (`super`, `perfect`, `perfect_parry`, `counter`, `punish_counter`, `crush_counter`, `alpha_counter`, `reversal`, `first_attack`, `burnout`, `custom_combo`, `combo_5`..`combo_15`) and spoken `text` as `tools/voice/lines.json`, with `Japanese accent` in the delivery bracket prompt so v3 keeps the accent consistent on short one- and two-word lines.

### 2. The voice description

Stored in `tools/voice/japanese.json` (the counterpart to `tools/voice/announcer.json`):

- **Description** (agreed with user):
  > A Japanese martial-arts tournament announcer in the late 1980s speaking English with a natural Japanese accent. Adult Japanese man, resonant mid-baritone voice, crisp and punchy, rolled R's and clean syllable timing. He barks short English tournament phrases with martial authority through an arcade cabinet speaker. No breathiness, no long vowels, no commentary.
- **Preview sample text** (100+ characters for ElevenLabs Voice Design `create-previews`, covering both classic calls and overhaul call-outs):
  > Round one! Fight! Japan! U.S.A.! First attack! Counter! Punish counter! Perfect parry! Super! Ten hit combo! Time up! Perfect! Game over!

### 3. Settings and UI

- **`AnnouncerVoice` enum** (`Ringside` default, `Japanese`):
  - Serializes in `settings.toml` as `"ringside"` / `"japanese"` (with `#[serde(default)]` on `Settings::announcer_voice` so existing `settings.toml` files without the key load `Ringside`).
  - Labels on screen: `"RINGSIDE"` and `"JAPANESE"`.
- **`Tab::Sound` on `SettingsScreen`** (`crates/sf-engine/src/front/settings.rs`):
  - Gains a new row `ANNOUNCER VOICE` immediately below `ANNOUNCER` (`Row::AnnouncerVoice`, `MenuItem::AnnouncerVoice`):
    1. `MASTER` (`0..=10`)
    2. `ANNOUNCER` (`0..=10`)
    3. `ANNOUNCER VOICE` (`RINGSIDE` / `JAPANESE`)
    4. `MUSIC (FM)` (`0..=10`)
    5. `EFFECTS & VOICES` (`0..=10`)
    6. `JUKEBOX`
    7. `RESET TAB`
  - Left and Right cycle between `RINGSIDE` and `JAPANESE` (`Transition::ValueChanged`, playing the cursor chirp `0x0c` **and** auditioning `AnnouncerLine::Fight` on the third MSM5205 at the current `ANNOUNCER` volume; see below).
  - `RESET TAB` on the `SOUND` tab resets `announcer_voice` back to `AnnouncerVoice::Ringside` along with the four volume sliders (`10`).
  - Like the volume sliders (`live_sound` in `crates/sf-engine/src/front/session.rs`), `ANNOUNCER VOICE` takes effect immediately while the Settings screen is open and is saved to `settings.toml` when leaving Settings.
  - **Auditioning on `ANNOUNCER VOICE` (settled in grilling, 2026-11-09):** Cycling `ANNOUNCER VOICE` Left or Right on the `SOUND` tab emits `AnnouncerLine::Fight` in `FrameOutput::announcements` alongside the normal `sound::CURSOR` (`0x0c`) latch, so the player hears `"Fight!"` in the newly selected voice at the `ANNOUNCER` slider's current volume. For `JAPANESE`, that plays `japanese/fight.vox` from the Japanese `VoiceStore`; for `RINGSIDE` (whose root pack has only the 22 overhaul stems), `RINGSIDE`'s `VoiceStore` holds the 1987 ROM's `0x94` (`"FIGHT!"`) nibbles extracted from `assets.audio2` (`voice_pipeline::rom_sample`), so `RINGSIDE` always has its `"FIGHT!"` clip ready on the third MSM5205 for the Settings audition.

### 4. Voice pack layout and host loading

- **On-disk layout:**
  - `sf1-port-voice/manifest.json` + `<stem>.vox` — the 22 overhaul lines for `RINGSIDE` (unchanged).
  - `sf1-port-voice/japanese/manifest.json` + `<stem>.vox` — the 35 lines (13 classic + 22 overhaul) for `JAPANESE`.
- **Lookup:**
  - `sf_host::voice::find` continues to find `sf1-port-voice` beside the ROM candidate folders.
  - `RINGSIDE` loads `VoiceStore::load(&pack_dir)` (expecting the 22 overhaul lines) and populates `AnnouncerLine::Fight` from the 1987 ROM's `0x94` sample in `assets.audio2` for the `ANNOUNCER VOICE` Settings audition.
  - `JAPANESE` loads `VoiceStore::load(&pack_dir.join("japanese"))` (expecting all 35 lines). If the `japanese/` subdirectory is not present, `sf-game` logs one line to `sf1-port.log` (as it already does when `sf1-port-voice` is absent) and holds an empty `VoiceStore` for `JAPANESE`.
  - *My call:* `AnnouncerLine` distinguishes the 22 overhaul lines (`AnnouncerLine::OVERHAUL`) from the full 35-line set (`AnnouncerLine::ALL`) so `VoiceStore::load` for the root `RINGSIDE` pack still reports all 22 lines loaded without falsely warning that the 13 classic ROM stems are "missing" from the root pack.

### 5. Output-latch interception in overhaul fights

- **When interception is active:**
  - Only when `out.tune_table` is `true` (a fight is running under an overhaul preset — never in the attract loop, never in the faithful cabinet, and never in a `Faithful 1987` fight) **and** the live `announcer_voice` is `AnnouncerVoice::Japanese`.
- **What happens on each frame (per-line fallback, settled in grilling, 2026-11-09):**
  1. If `out.tune_table` is `true`, `announcer_voice == AnnouncerVoice::Japanese`, `AnnouncerLine::from_rom_latch(out.sound_latch)` is `Some(line)`, **and** the Japanese `VoiceStore` has a loaded clip for `line` (`voice.clip(line).is_some()`):
     - The latch passed to `AudioOut` (`SamplesSound` / `Ym2151Sound`) for that frame is replaced with `sound::IDLE` (`0xff`), so the 1987 `SamplesCpu` does not start the 1987 ROM voice on MSM5205 #0.
     - `line` is announced on the third MSM5205 ahead of any `out.announcements` on that frame.
  2. For all `out.announcements` (the 22 overhaul call-out and combo lines, plus the Settings `AnnouncerLine::Fight` audition):
     - The active voice's `VoiceStore` (`RINGSIDE` or `JAPANESE`) resolves each `AnnouncerLine` to its `Nibbles` slice and passes it to `Announcer` on the third MSM5205.
  3. If `announcer_voice == AnnouncerVoice::Japanese` but the Japanese `VoiceStore` lacks `line` (for example, `sf1-port-voice/japanese/` is not installed on disk, or only a subset of the 13 classic stems has been picked so far), `out.sound_latch` is **not** masked for that stem, so the 1987 ROM sample still plays on MSM5205 #0.
- **Stopping a playing classic call on `STOP_SAMPLES` (`0x80`):**
  - *My call:* When `out.sound_latch == sound::STOP_SAMPLES` (`0x80`, sent when leaving a fight for the Main Menu or restarting a drive) or when `out.tune_table` becomes `false` (leaving a fight), `Announcer` resets its current clip (`Announcer::stop`), matching how `0x80` stops the 1987 sample channels so a `GAME OVER` or `ROUND 1` clip never bleeds into the menus.

### 6. `tools/voice/` pipeline support

- `tools/voice/pipeline.py` gains `--voice {ringside,japanese}` (default `ringside` so every existing command in `docs/reference/voice-pipeline.md` works unchanged).
- Voice configuration files:
  - `ringside` (default): `tools/voice/announcer.json`, `tools/voice/lines.json` (22 lines), `tools/voice/picks.json`, archive at `tools/reference/local/voice/`, output pack at `tools/reference/local/roms/sf1-port-voice/`.
  - `japanese`: `tools/voice/japanese.json`, the 35-line set (`tools/voice/lines.json`'s 22 lines + the 13 classic lines in `tools/voice/japanese.json` or a shared loader), `tools/voice/picks_japanese.json`, archive at `tools/reference/local/voice/japanese/`, output pack at `tools/reference/local/roms/sf1-port-voice/japanese/`.
- **Listening sheet benchmark (settled in grilling, 2026-11-09):** `sheet.html` for `--voice japanese` still includes the 1987 ROM's `FIGHT!` (`0x94`) player at the top of the page as the 4 kHz hardware loudness, cadence, and grit benchmark.
- `sf-audio` (`crates/sf-audio/src/main.rs`) `pack` command accepts the 35 stems when building `sf1-port-voice/japanese/`.

## Left to the builder

- **Exact factoring between `VoiceStore` and `AudioOut` (`crates/sf-host/src/voice.rs`, `crates/sf-host/src/audio.rs`, `crates/sf-game/src/main.rs`):** whether `AudioOut` holds both `VoiceStore`s and switches by `AnnouncerVoice` inside `AudioOut::step_frame`, or `Playing` in `sf-game` holds a pair of `VoiceStore`s and selects the active one before calling `audio`. Keeping the selection and latch-masking helper in `sf-host` (so unit tests in `sf-host` can test the latch interception, fallback when a clip is absent, and `0x80` stop directly without opening a GPU window) is recommended.
- **How `tools/voice/pipeline.py` merges or loads the 35 lines for `--voice japanese`:** whether the 13 classic lines live in a `"classic_lines"` array inside `tools/voice/japanese.json` (concatenated with `tools/voice/lines.json`'s 22 lines so the 22 overhaul stems have a single source of truth) or in their own JSON file.
- **Tests:**
  - `sf-video`: `AnnouncerLine::from_rom_latch` maps `0x8f..=0x9b` and returns `None` for fighter quotes (`0x9c..=0x9e`, `0xba`) and combat samples; priority of the 13 classic lines is `6`; stem round-trip for all 35 lines.
  - `sf-engine`: `Settings` TOML round-trip with `announcer_voice` (`ringside` / `japanese`, and default when absent); `Tab::Sound` cursor, Left/Right cycling on `ANNOUNCER VOICE`, `RESET TAB` restoring `RINGSIDE`, and `live_sound` reporting the open Settings screen's `announcer_voice`.
  - `sf-host` / `sf-audio`: root `VoiceStore` loads the 22 overhaul lines without warning on the 13 classic stems; `japanese` `VoiceStore` loads up to 35 lines; latch interception masks `0x8f..=0x9b` to `0xff` and queues the `AnnouncerLine` only when `tune_table` is true, `announcer_voice == Japanese`, and the clip is loaded; falls back to the raw latch when the clip is missing, when `announcer_voice == Ringside`, or when `tune_table` is false; `0x80` (`STOP_SAMPLES`) stops the announcer.
