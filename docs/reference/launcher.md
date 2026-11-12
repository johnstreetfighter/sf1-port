# The development launcher

This document records the boundary established by
issue #38: the fixed
fight played with the keyboard in the development launcher
(`crates/sf-dev`), both players in the versus fixture and P1 against the
CPU in the one-player fixture, with the translated samples CPU's sound
(and, since ticket #37, both sound programs: both-chips.md). It supersedes the per-ticket launcher notes in movement.md, crouch.md,
jump.md, punch.md and cpu-decision.md (those described earlier, narrower
builds).

## Running it

From the crate directory (`crates/sf-dev`; the paths to the local-only
ROM set and reference captures are relative to it):

```
cargo run -p sf-dev --release            # versus: Ryu (P1) vs Ken (P2)
cargo run -p sf-dev --release -- cpu     # one-player: Ryu (P1) vs Retsu
cargo run -p sf-dev --release -- power-on # power-on: frame 0 through warning, attract, coin/start, country select, and fight with no fixture (tickets #55, #89)
cargo run -p sf-dev --release -- era-hud [p1-era] [p2-era] [art] # preview the HUD in a native fight
cargo run -p sf-dev --release -- smoke   # the host smoke check
cargo run -p sf-dev --release -- kit     # fixed-fight listening kit (ticket #49)
cargo run -p sf-dev --release -- music   # the round music phrase (ticket #11)
cargo run -p sf-dev --release -- record <cpu|versus> <session.tsv>
cargo run -p sf-dev --release -- record-synthetic <session.tsv>
cargo run -p sf-dev --release -- convert <session.tsv> <scenario.json> [scenario-id]
cargo run -p sf-dev --release -- verify <session.tsv> <scenario.json> <run-a> <run-b>
cargo run -p sf-dev --release -- rebind <input> <Key> [bindings.toml]
```

Debug builds render below 60 fps; the state machine is exact in either
profile.

## Ryu's SF1+ combo sweep

Run these commands from the repository root with the local SF1 ROM set
installed. The sweep prints its report to the terminal and saves one route
replay to the path you supply. The directory in this example is ignored by Git.

```text
cargo run -p sf-dev --release -- combo-sweep tools/reference/local/ryu-clk-hadoken.json
cargo run -p sf-dev --release -- combo-replay tools/reference/local/ryu-clk-hadoken.json
cargo run -p sf-dev --release -- combo-verify tools/reference/local/ryu-clk-hadoken.json
```

The sweep enters a Local match under Definitive Overhaul and walks Ryu
into close range against standing Ken in Japan. It moves a single
light-punch input through the fixed quarter-circle motion, beginning on
the frame after the kick input and ending on the last frame of the opener's
hitstun. Reported windows are inclusive and relative to the crouching
light kick's contact. The Hadoken's projectile must hit during that stun.

Both replay commands use the saved inputs from power-on and verify the
recorded fight setup, Hadoken startup, cancel and both hit frames. They require
the same build revision and ROM assets as the sweep. `combo-replay` shows
the route in the developer window. Press Esc to exit playback early.

## SF1+ fighter coverage

Give the sweep a fighter, an output directory and one or more signature
moves. Names are case-insensitive. Quote names with spaces or use hyphens
between the words.

```text
cargo run -p sf-dev --release -- combo-sweep Ryu tools/reference/local/combo-ryu Hadoken Shoryuken
cargo run -p sf-dev --release -- combo-sweep Retsu tools/reference/local/combo-retsu "Kiai Shot"
```

Ryu and Ken use their ROM player tables. Tournament fighters with extended
tables require the current `sf1-port-art` pack beside the SF1 ROM folder.
The sweep verifies the active player table before measuring any inputs.

Each sweep tries the six buttons while standing, crouching and jumping,
including the directional jump attacks and distinct diagonal crouch punches.
For jumps it selects a repeatable input on descent. Ken stands or holds
Down throughout each route. The built-in signature motion stays fixed as
the sweep moves its button pulse, including the release edge, one frame
at a time. Results apply to the recorded spacing, opener and command.

The output directory contains `coverage.json` and a verified replay for
each successful pair and stance. In the report, `no_success` means the
move could be tested in this setup. `unsupported` records a move or
strength the tool could not execute. `fixed_inputs` lists consecutive
runs of identical pad inputs. Each pad uses bits 0–9 for Right, Left,
Down, Up, LP, MP, HP, LK, MK and HK. Successful offsets are relative to
the opener's first clean hit. Each success records whether the signature
started with a cancel or linked after recovery.

Pass any replay filename from the report to `combo-replay` or
`combo-verify`. Both use the exact recorded inputs and verify the build,
assets, setup, player table, opener hits and the signature's combo hit.

## Remappable keyboard bindings (ticket #76, F36)

`crates/sf-host/src/bindings.rs` maps all 25 logical inputs of the cabinet's
`Io` (`in0`, `in1`, `system`) to `minifb::Key` values and persists the table in
TOML at `%APPDATA%\sf1-port\bindings.toml` on Windows (`default_bindings_path`):

| Group | Logical inputs | Default keys |
| --- | --- | --- |
| P1 (`p1_right`, `p1_left`, `p1_down`, `p1_up`, `p1_button1`..`p1_button6`) | Right, Left, Down, Up, LP, MP, HP, LK, MK, HK | `Right`, `Left`, `Down`, `Up`, `A`, `S`, `D`, `Z`, `X`, `C` |
| P2 (`p2_right`, `p2_left`, `p2_down`, `p2_up`, `p2_button1`..`p2_button6`) | Right, Left, Down, Up, LP, MP, HP, LK, MK, HK | `H`, `F`, `G`, `T`, `U`, `I`, `O`, `J`, `K`, `L` |
| Cabinet (`coin1`, `coin2`, `start1`, `start2`, `service1`) | Coin 1, Coin 2, P1 Start, P2 Start, Service 1 | `Key5`, `Key6`, `Key1`, `Key2`, `Key9` |

- `Esc` (`Key::Escape`) stays fixed to quit and is refused as a binding.
- Duplicate keys (binding the same `Key` to two logical inputs) and partial
  tables (missing any of the 25 inputs) are refused.
- When `%APPDATA%\sf1-port\bindings.toml` is missing, corrupt, partial, or
  invalid, `BindingsTable::load_or_default` falls back to `DEFAULT_BINDINGS`
  and reports why.
- `cargo run -p sf-dev --release -- rebind <input> <Key> [bindings.toml]`
  updates a single logical input, validates the 25-key table, and saves the
  updated `bindings.toml`.
- Every one of the 25 logical inputs reaches its exact active-low bit of `Io`
  (`in0`, `in1`, `system`), and only that bit, for both `DEFAULT_BINDINGS` and
  a non-default remapped table (`every_key_reaches_its_documented_input`), and
  feeding `sf-fixed-fight-audit-01` (`2390..=5014`, `2,625` frames) and
  `sf-blocks-01` (`2451..=4320`, `1,870` frames) through a completely remapped
  25-key `BindingsTable` reproduces the MAME reference work RAM and object RAM
  on every frame (`remapped_p1_bindings_reproduce_fixed_fight_audit_01` and
  `remapped_p1_and_p2_bindings_reproduce_blocks_01`).

Esc quits. A key counts as held only while the OS reports it down and the
window is focused, so an alt-tab or input reset releases everything and no
direction can stick. The keys feed the same logical inputs the capture
runner replays from the scenario files (`set_p1_direction`,
`set_p1_buttons`, and P2's), sampled at the next frame's vblank:

- an attack fires when its key is released, as on the reference
  six-button cabinet (the button latch turns the release into the attack
  strength of its group);
- the special-move commands are the scenarios' (specials.md, hadoken.md):
  Shoryuken F, D, DF + punch; Tatsumaki D, DB, B + kick; Hadoken D, DF,
  F + punch with forward held through the walk's gated frames;
- "forward" and "back" follow the fighter's facing, as the input handler
  reads them.

## The fixtures and how far they run

- **Versus** starts from the checkpoint-2450 state of sf-move-01: both
  fighters in their stances, round 1, TIME 99, the stage at the fight's
  start (movement.md). Both players are live. The round can end by KO, by
  the time-over judged on health, by a double KO or a time-out draw; the
  round result, its bonus tally, the next rounds and the three-rounds
  rule follow (round-result.md, round-transition.md, round-outcomes.md).
- **One-player** starts from the checkpoint-2389 state of sf-cpu-01:
  FIGHT! just erased, Retsu about to make his first decision
  (cpu-decision.md). P1 is live; Retsu plays from his AI tables
  (cpu-follow-up.md, retsu-reactions.md). A won match goes on into Geki's
  stage intro and ROUND 1 (match-win.md).
- **Power-on (`power-on`)** starts `PowerOnMachine` at frame 0 with no
  checkpoint fixture and an empty `LatchTimeline` (`GameSound::power_on`),
  running through the reset, boot set-up, warning screen, boot-to-attract
  join at frame 794, attract title and ranking screen, coin/start entry,
  country select, pre-fight intros, and the fight itself with no fixture
  (power-on.md).

Each runs as far as the translation goes. The first untranslated original
branch stops it: the launcher prints the frame and the branch's address
and reloads the fixture; no frame is guessed. Today those are Geki's hit
functions, the continue countdown's expiry or credit continuation, the
versus winner's player setup and the versus game over.

## Sound

Both translated sound programs play on the default output device, in
stereo, mixed as the reference mixes them (`sf_audio::mix::GameSound`,
both-chips.md): the samples CPU's ADPCM voices and impacts on the two
MSM5205s and the music CPU's tunes and FM effects on the YM2151. The sound
runs from power-on, pre-rolled to the fixture frame on the reference's
latch writes before it, from a reference audio run whose history matches
the fixture's: hit-sound-run-a for versus, fight-audio-a for one-player.
The round music and a voice still playing at the fixture are heard. From
the fixture on, the engine's per-frame sound latch drives both programs,
at the interrupt latency band's middle (latch-timing.md). The game loop
pushes each frame's latch; the output plays only as far as the pushed
frames' writes reach, so sound never runs ahead of the game. Without the
reference audio run the pre-roll has no writes and the sound starts silent;
without a usable output device the launcher runs silent and says so.

## The host smoke check

`cargo run -p sf-dev --release -- smoke` opens the window and the audio
output, checks that the sound was pre-rolled (the reference audio run was
read and has latch writes before the fixture frame), loads and validates the
active binding table from `%APPDATA%\sf1-port\bindings.toml` (falling back to
`DEFAULT_BINDINGS` with a note when absent) and reports all 25 bound keys, and
drives both fixtures through a scripted sequence per player — a 30-frame walk
forward, a light punch, and a Hadoken command, as the scenarios script them —
for 240 frames, presenting every fourth frame and pushing every latch to the
audio output. It passes only if both runs stay on translated paths, sound
commands reached the latch, and for each fixture the music CPU stayed on
translated paths and the output played more than 300 ms at a peak above 0.01
(both-chips.md). The keys themselves cannot be pressed from a program; the
check feeds the bindings' inputs through the active `BindingsTable` exactly as
the key reader produces them.

Run on the recording host (Windows 10, the default output device):

```
ok   window opened
ok   audio output opened (default device)
ok   sound pre-rolled to frame 2450 on the reference's 2450 latch writes through it (0.99 s)
note: cannot read %APPDATA%/sf1-port/bindings.toml: The system cannot find the path specified. (os error 3); falling back to default keyboard bindings
ok   25 distinct non-Escape keys bound for P1, P2, and cabinet (P1 dir=Right/Left/Down/Up btn=A/S/D/Z/X/C, P2 dir=H/F/G/T btn=U/I/O/J/K/L, cabinet coin1=Key5 coin2=Key6 start1=Key1 start2=Key2 service1=Key9)
ok   versus fixture: 240 frames of scripted P1 and P2 play on translated paths
ok   versus fixture: sound commands reached the latch (16 frames)
ok   versus fixture: the music CPU stayed on translated paths
ok   versus fixture: the output played 1552 ms of the run, peak level 0.695
ok   one-player fixture: 240 frames of scripted P1 play against Retsu on translated paths
ok   one-player fixture: the music CPU stayed on translated paths
ok   one-player fixture: the output played 1510 ms of the run, peak level 0.426
ok   power-on mode: warning screen typed (249 non-blank cells at frame 554, erased and joined to 23 HUD/INSERT COIN cells at frame 794)
ok   power-on mode: title screen shown (397 non-blank cells at frame 1500, bg_scroll=0x0480, latches [(01, 27), (02, 3f), (03, 80), (3d0, 1c)])
ok   power-on mode: 1647 frames from power-on on translated paths
ok   power-on mode: the music CPU stayed on translated paths
ok   power-on mode: the output played 1400 ms from power-on with no pre-roll, title sound 0x1c peak level 0.317
ok   power-on mode: scripted coin+start played into the fight at frame 2500 with no fixture (mode=2, scene_mode=0x80, latches=11)
smoke check passed
```

## The fixed-fight listening kit (ticket #49, F9)

`cargo run -p sf-dev --release -- kit` runs the one-player fixture (`CpuMachine`
from checkpoint `2389`) through the fixed fight's scripted `P1_KEYS` inputs
(`sf-fixed-fight-01`, `2389..=5000`), captures the launcher's live output at
latency `92` via `audio::LiveCapture`, renders the native batch mixes at
latencies `51` and `92`, slices MAME's `fight-audio-a/audio.wav` aligned to the
same windows (`3830..=4999` for Round 2 and `2389..=4999` for the whole fight),
writes all eight local WAV files into `tools/reference/local/renders/`, and
verifies that the live capture equals the batch mix at latency `92` sample for
sample (`0` differing out of `936,000` on Round 2 and `2,088,800` on the full
fight). See `docs/reference/both-chips.md` for the recorded measures, protocol,
and verdict table.

## The era HUD preview (ticket #217, EH1)

`cargo run -p sf-dev --release -- era-hud [p1-era] [p2-era] [art]` starts a
native local fight with Ryu on P1 and Ken on P2. Use the regular keyboard or
pad controls to earn and spend meter and see call-outs from the fight. The
eras are `sf1` (no gauge), `st`, `alpha`, `sf3`, `sf4`, `sf5` and `sf6`.
The default is `sf6` against `alpha` in `ruled` gauge art. Use `blocks` or
`plate` as the third argument to start with that gauge art.

F5 and F6 cycle the style on P1 and P2 respectively and restart the fight.
Tab cycles the gauge art. F7 and F8 cycle each side through the developer
call-out fixtures, including Alpha Counter gauge flash and Super stage dim.
Esc closes the preview.

## Session recording, scenario conversion, and reference verification (ticket #50, F10)

`crates/sf-dev/src/session.rs` records a keyboard session from either fixture
(`2389` for `cpu`, `2450` for `versus`), converts it to a MAME scenario JSON,
and verifies the native replay against two MAME reference runs:

- **Record (`record <cpu|versus> <session.tsv>` and `record-synthetic <session.tsv>`):**
  Logs one row per stepped frame `f` (`fixture_frame .. last_frame`) with:
  - `frame`: the scenario input frame `f` active before `machine.step()`;
  - `focused` and `focus_changed`: window focus state and transitions (keys
    sampled through `Bindings::inputs` via `session::read_with_focus`, dropping
    held keys when unfocused);
  - `frame_ms`: wall-clock duration of the frame in milliseconds;
  - `p1_mask` and `p2_mask`: 10-bit masks of P1 and P2 `Inputs`
    (`directions[0..4]` in bits `0..4`, `buttons[0..6]` in bits `4..10`;
    `CpuMachine` also wires P2's inputs into `IN1` and `IN0` so `0xff868e`
    `IN1_COPY` matches MAME when P2 keys are pressed during a 1P session).
  Reports total frames, focus changes, and **mean and maximum frame times**.
- **Convert (`convert <session.tsv> <scenario.json> [scenario-id]`):**
  Writes a scenario JSON with the fixture's pre-fixture boot inputs (`sf-cpu-01`
  for `2389`, `sf-move-01` for `2450`), every contiguous key-hold span from
  `fixture_frame` on, `duration_frames = last_frame + 1`,
  `observations.window = [fixture_frame, last_frame]`, and
  `observations.checkpoints = [fixture_frame..=last_frame]`.
- **Verify (`verify <session.tsv> <scenario.json> <run-a> <run-b>`):**
  Reports the session's frame count, focus changes, and mean and maximum frame
  times; checks that the two MAME captures (`run-a` and `run-b`) are byte-for-byte
  identical across all `.tsv`, `.bin`, and `.rgba` observation files; and
  replays the scenario natively from the fixture to `last_frame`, running the
  full comparison on every frame (`screen_hash`, `latch`, `latch_changed`,
  checkpoint `pixels`, `video`, `palette`, and owned/classified work RAM) or
  reporting the first untranslated branch reached.

### Synthetic 1,011-frame session (`sf-session-synth-01`, `2389..=3400`)

`record-synthetic` runs a `1,011`-frame synthetic session (`2389..=3400`) from
`sf-cpu-01`'s checkpoint `2389` that sends every one of the 20 bound keys
(`P1_KEYS` and `P2_KEYS`) through `Bindings::inputs`, plus a simulated focus loss
and regain (`3050..3060`) while `Key::Right` is held:

```
session: cpu fixture (2389..=3400), 1011 frames, 2 focus changes, mean frame time 2.320 ms, max frame time 5.550 ms
verify PASS: reference run-a == run-b over 5063 observation files; native replay matches 1011 frames and 1011 full checkpoints + work RAM through 3400
```

Negative control (`C-NEG`): shifting `p1_right` at frame `2400` by 1 frame to
`2401` is caught at frame `2401` (`checkpoint screen pixels: expected crc32
6b669161, actual crc32 3d3448bf (first differing pixel at (134, 121): expected
argb ff000000, actual ffaa6633)`).

### Human-recorded keyboard sessions of both fixtures (ticket #71, F31)

Ticket #71 (F31, evidence class
`PERSON`) records a person's interactive keyboard sessions of both fixtures
(`tools/reference/local/sessions/human-cpu-01.tsv` and
`tools/reference/local/sessions/human-versus-01.tsv`), converts each to a MAME
scenario (`sf_human_cpu_01.json` / `sf-human-cpu-01` and
`sf_human_versus_01.json` / `sf-human-versus-01`), records two reference runs per
session on the pinned MAME 0.289 (`human-cpu-01-run-a/b` and
`human-versus-01-run-a/b`), and verifies the native replay against both (`cargo
run -p sf-dev --release -- verify` and
`tests::human_recorded_cpu_and_versus_sessions_verify_against_mame`):

| Session (`tools/reference/local/sessions/`) | Fixture & frame span | Frames | Focus changes | Mean / max frame time (`60 fps` target = `16.667 ms`) | Reference capture pair (`run-a == run-b`) | Native replay verification |
| --- | --- | --- | --- | --- | --- | --- |
| `human-cpu-01.tsv` (`sf-human-cpu-01`) | `cpu` (`2389..=3979`, `26.5 s`) | `1,590` | `0` | `17.111 ms` / `131.803 ms` | `7,958` observation files byte-identical | **PASS**: `1,590` frames and `1,590` full checkpoints (`pixels`, `video`, `palette`) + owned/classified work RAM equal through `3979` (`0` untranslated branches) |
| `human-versus-01.tsv` (`sf-human-versus-01`) | `versus` (`2450..=3785`, `22.3 s`) | `1,335` | `0` | `17.112 ms` / `106.519 ms` | `6,683` observation files byte-identical | **PASS**: `1,335` frames and `1,335` full checkpoints (`pixels`, `video`, `palette`) + owned/classified work RAM equal through `3785` (`0` untranslated branches) |

Recorded `verify` output:

```
session: cpu fixture (2389..=3979), 1590 frames, 0 focus changes, mean frame time 17.111 ms, max frame time 131.803 ms
verify PASS: reference run-a == run-b over 7958 observation files; native replay matches 1590 frames and 1590 full checkpoints + work RAM through 3979
session: versus fixture (2450..=3785), 1335 frames, 0 focus changes, mean frame time 17.112 ms, max frame time 106.519 ms
verify PASS: reference run-a == run-b over 6683 observation files; native replay matches 1335 frames and 1335 full checkpoints + work RAM through 3785
```

**Person's notes on controls, pacing, and findings (ticket #71 / F31):**
- **Controls and pacing:** *"Seems to play very well (to the source's standard)."* Mean frame time is `17.11 ms` (`~58.4 fps` under `minifb`'s `set_target_fps(60)` sleep granularity), with all P1/P2 directional, normal-attack, and special-move inputs matching the arcade reference frame for frame across both sessions.
- **Finding (`bindings.toml` first-launch terminal warning):** *"Side note: it seems the bindings aren't being loaded, saw errors in the terminal."* On a machine where `%APPDATA%\sf1-port\bindings.toml` had not yet been created by `rebind`, `load_active_bindings()` printed `bindings: cannot read ...\bindings.toml: The system cannot find the path specified. (os error 3); falling back to default keyboard bindings`. Fixed in `load_active_bindings()` and `smoke()` (`crates/sf-dev/src/main.rs`) by initializing `%APPDATA%\sf1-port\bindings.toml` with `DEFAULT_BINDINGS` when the file does not exist yet, while keeping `BindingsTable::load_or_default`'s diagnostic fallback for malformed or invalid user edits.

## Not covered

- **Joystick / gamepad input**: later work (keyboard remapping is covered in
  `crates/sf-host/src/bindings.rs`, ticket #76).
- **The fixtures' local data**: the launcher needs the local ROM set and
  the two reference captures (tools/reference/local, not in the
  repository).
