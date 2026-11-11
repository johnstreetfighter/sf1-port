# The announcer's voice pipeline (EH9, #225)

Spec #216 (stories 64 to 69). Design: `docs/design/2026-11-04-era-hud-and-callouts-design.md` (The announcer). The pack it writes and how the game plays it: [`announcer.md`](announcer.md) (EH8, ADR 0011). The pipeline uses an approach tested previously: an original ElevenLabs voice designed from a text description and never cloned, every take archived with its prompt and settings, a `status` that costs nothing, and a `run` that voices only new or changed lines.

```bash
cargo build --release -p sf-audio                              # the codec the pipeline calls
python tools/voice/pipeline.py status                          # what a run would do: no network, no key, no cost
python tools/voice/pipeline.py design --max-credits 200        # one Voice Design round: three previews (paid)
python tools/voice/pipeline.py design --save 2 --round 1       # keep a preview as the voice (free)
python tools/voice/pipeline.py run --max-credits 1000          # three takes of every new or changed line (paid)
python tools/voice/pipeline.py listen                          # the listening sheet at http://127.0.0.1:8765/sheet.html
python tools/voice/pipeline.py build-pack                      # write sf1-port-voice from the picks
python -m unittest discover -s tools/voice/tests               # the pipeline's tests
```

It needs Python 3.11, ffmpeg on the PATH and the release `sf-audio` binary (`SF_AUDIO_BIN` overrides its path). `ELEVENLABS_API_KEY` is read from the Windows user environment by `tools/voice/el.py` alone (falling back to the process environment). It is never printed, logged or written to a file.

## What is where

| In the repository (`tools/voice/`) | Local, never committed |
|---|---|
| `lines.json`: the 22 lines (stem, spoken text, prompt), the takes per line (3) and the combo cap (15) | `tools/reference/local/roms/sf1-port-voice/`: the voice pack the game loads (`manifest.json`, `<stem>.vox`) |
| `announcer.json`: the voice's description, preview text, models, settings, seed and, once saved, its voice id | `sf1-port-voice/archive/`: every take and design preview, their processed files, the ROM's FIGHT!, the listening sheet |
| `picks.json`: the maintainer's pick of each line's take | |
| `ledger.json`: every paid batch, with the vendor's own counter before and after | |
| the code and its tests | |

The archive lives inside the pack's folder because the game ignores everything there but the manifest and the files it names (`announcer.md`), so one private backup holds both. Update the private backup after each build-pack run and keep the archived files byte-exact.

## The lines

`tools/voice/lines.json` lists one line per voice-pack stem (`sf_video::announcer::AnnouncerLine::file_stem`): Super!, Perfect!, Perfect parry!, Counter!, Punish counter!, Crush counter!, Alpha counter!, Reversal!, First attack!, Burnout!, Custom combo!, and "<N> hit combo!" spelled out ("Five hit combo!") for N = 5 to 15. The pipeline refuses a list that isn't exactly the word lines plus `combo_5` up to the cap.

**The combo cap is 15.** A finished combo of more than 15 hits has no line, so he says nothing rather than a wrong number (`announcer.md`: a count above the pack's highest `combo_N` is not called). Raising it is one entry per count in `lines.json` and one run.

**The prompt** is what the voice is sent: the text in capitals, for emphasis without a delivery tag. A prior voice test found that ElevenLabs' `[shouting]` tag lifts a voice into a high yell and costs clarity, while capitals alone kept 22 of 25 callouts clear (`audio-voice-test.md`, Findings 1). Whether capitals or a tag suits the 1987 announcer is for the maintainer's ear. Changing a prompt re-voices that line alone.

## The voice

`tools/voice/announcer.json` holds the description agreed in the design doc, tuned toward the 1987 voice that says ROUND 1 and FIGHT!:

> A ringside announcer at a 1980s world fighting tournament. Adult man, strong mid-range voice, a little gravel, loud and bright. He barks single words and short phrases with hard consonants, as if he's calling the action over a crowd through a cheap PA. No breathiness, no long vowels, no commentary.

`design` sends it to ElevenLabs Voice Design (`eleven_ttv_v3`, guidance 5, loudness 0.5) with the preview text, which has him say ROUND ONE! and FIGHT! and most of his lines (184 characters; Voice Design needs at least 100). No audio is sent, so nothing is cloned. Each round is a new seed. Its three previews are archived with the request in `archive/design/` and encoded like takes, so they are judged after the ROM's FIGHT! too. `design --save P [--round R]` keeps a preview as the voice, which is free, and writes its voice id into `announcer.json`. A new voice changes every take's key, so the next `run` voices every line again.

If the previews come out strained, describe the instrument in terms of its voice ("shouting" and "booming" gave high yells there).

## What a run does

1. **Takes.** Every line gets three takes. Take *t* is sent with the voice's seed + *t* (`eleven_v4`, the voice's settings, 24 kHz mono 16-bit PCM). Each take's key is a hash of everything sent: the prompt, voice id, model, settings, seed and output format. A take whose key is archived costs nothing. So only a new line, a changed prompt, a new voice or new settings costs credits, and going back to an earlier prompt is free.
2. **Archive.** Each take is written to `archive/takes/<stem>/<key>.wav` as it arrives, with a `.json` sidecar holding the text, the prompt, the full request (model, settings, seed), the voice id and the key. Takes are never deleted. A run that stops partway keeps every take it paid for.
3. **Processing (free).** From the archived original: the silence is trimmed (anything 40 dB under the take's peak, keeping 20 ms either side). Then ffmpeg downsamples it to 4 kHz mono 16-bit, with its resampler's anti-alias low-pass, and the peak is normalised to -2 dBFS, where the ROM's FIGHT! peaks. Then `sf-audio voice-encode` encodes it with the asset pack's encoder (`sf_video::asset_pack::encode_adpcm`, 16 bits shifted to the chip's 12) and writes what the board's MSM5205 model plays from it as `<key>.heard.wav`. `audio.PROCESS` holds every value. Changing one re-processes every take, and nothing is paid again.
4. **The listening sheet**, `archive/sheet.html`. Every button plays the ROM's own FIGHT! and then the take. FIGHT! is samples-CPU command `0x94` (1,968 bytes on channel 0), played by the translated samples CPU from power-on and decoded by the same chip model (`sf-audio voice-rom-sample`). Only encoded takes are on the sheet.

A paid command needs `--max-credits N`. Before spending, it reads the vendor's counter and plans at the list price of 1 credit a character. It stops short of the budget, of `--stop-at` (14,000 by default, under the key's 15,000 cap) and of the plan's limit. Takes it doesn't reach are reported as deferred and wait for the next run. Every paid batch is a ledger entry with the vendor's own counter before and after.

## Picking and the pack

`listen` serves the archive on 127.0.0.1 only. The sheet's radio buttons choose each line's take, and **Save picks** writes `tools/voice/picks.json`, checked first: every stem must be a line, and every take an archived, encoded take. Opened as a file instead, the sheet downloads `picks.json` for moving into place. `pick <stem> <take>` records a pick by hand. A pick names its take by key, so it always means the take the maintainer heard.

`build-pack` writes the picked takes' `.vox` files as `sf1-port-voice` with `sf-audio voice-pack`, which calls `VoiceStore::write` and then opens the result with `VoiceStore::open`, as the game does, checking every line nibble for nibble. A line with no pick, or whose pick is no longer one of its current takes (its prompt changed), is left out and reported. The game doesn't call a line the pack lacks.

## The Rust side

`crates/sf-audio/src/voice_pipeline.rs` and three `sf-audio` subcommands, so Python never reimplements the codec:

| Subcommand | Does |
|---|---|
| `voice-encode --wav T.4k.wav --vox T.vox --heard T.heard.wav` | refuses anything but mono 16-bit 4 kHz; encodes; writes the `.vox` (high nibble first) and the chip's output as a 48 kHz WAV (each 4 kHz value held 12 samples); prints `{nibbles, crc32, seconds}` |
| `voice-rom-sample --command 0x94 --out F.wav` | the ROM sample's nibbles as the samples CPU writes them (after the `0x40` release, up to the reset), as a 48 kHz WAV |
| `voice-pack --plan P.json --out DIR` | `{"lines": {stem: {vox, nibbles}}}` to a pack via `VoiceStore::write`, opened again and compared |

## Cost of a first full run

`status` prints this estimate with no network (`first_full_run`):

| Part | Characters | Credits |
|---|---|---|
| One Voice Design round (three previews; charged on the preview text, once a round) | 184 | 184 |
| 3 takes × 22 lines (302 prompt characters a set) | 906 | 906 at the list price; about 363 at the measured 0.40 |
| Saving the voice | | 0 |
| **Total** | | **1,090 planned at the list price; about 547 at the measured rate** |

The rates were measured on a Starter plan on 2026-10-20. Voice Design charged 1 credit a preview character per round, and saving a voice was free. Text to speech charged 0.39 to 0.40 credits a prompt character, tags included, against the list price of 1. At the $6 a month for 40,000 credits recorded there, that's $0.16 at the list price or $0.08 at the measured rate, as a share of the month's plan. Each extra design round adds 184. The ledger records what was actually spent.

## Tests

- `tools/voice/tests/test_pipeline.py` (Python, no network). The fake vendor, and the real `el.py` against a mocked `urlopen` with a made-up key, check that:
  - `status` touches neither the network nor the key;
  - nothing is voiced before the voice is saved, and a design round archives its request and three encoded previews;
  - a first run voices three seeded takes of every line, and a rerun does nothing;
  - every take is archived with its prompt, settings and seed;
  - a changed prompt re-voices that line alone and keeps its old takes, and going back is free;
  - a new voice re-voices every line, and a processing change re-encodes without spending;
  - a paid run needs a budget and stops at it, at the cap and at the plan's limit, and a stopped run keeps what it paid for;
  - the sheet plays FIGHT! and then only encoded takes;
  - picks are checked, and `build-pack` leaves out unpicked and stale lines;
  - the key reaches no output, file or error, and only `el.py` reads it;
  - the line list matches the Rust stems and spells the counts out, and the description is the design doc's;
  - no audio is in `tools/voice/`.

  An end-to-end test runs real ffmpeg and the real binary from takes to a pack, and skips without them.
- `crates/sf-audio/tests/voice_pipeline.rs`. `voice-encode` round-trips through `validate_adpcm` and prints the file's CRC-32. A take that isn't 4 kHz mono 16-bit is refused. `VoiceStore::open` loads what `voice-pack` writes, and a bad stem or count is refused. With the ROM set, `0x94` is FIGHT!'s 1,968 bytes as 3,936 nibbles, and `0x8c` is channel 1's 2,128 bytes.
