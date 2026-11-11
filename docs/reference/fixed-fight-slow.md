# The Speed DIP's Slow Game (Tickets #60 / F20, #141 / F69)

- **Label:** `DF / CF`, `PO / CF`
- **Tickets:** `#60` (`F20`), `#141` (`F69`)
- **Size:** 0; the existing translation already implements the one-frame/six-frame fight-loop sleep and the independent round-timer task. Verified from both fixture `2975` (`F20`) and power-on `0..=5652` (`F69`).

## Verified Boundary

`sf-fixed-fight-slow-01` runs on the pinned MAME 0.289 baseline with `Speed = Slow` (`DSW1 = 0xcfff`, `DSW2 = 0xffff`). It captures frames `0..=5660` and records full checkpoints over `0..=830` and after every frame `2975..=5660` (`3,517` checkpoints, `17,588` identical observation files). `PowerOnMachine` matches `0..=5652` from power-on (`#141`, `F69`, `crates/sf-engine/tests/dip_switches.rs`), including owned work RAM at fixture `2975`.

Calibration placed coin at 1450, 1P start at 1520, and country confirmation at 1900. The fixed fight's right, light-punch and light-kick inputs moved to 2986, 3176 and 3216, preserving their offsets from the round-1 fixture. The fight checkpoint at **2975** decodes through `CpuMachine::from_fight_checkpoint`; it has scene mode `0x80` and TIME `100`.

This is a controlled fight fixture, not a claim of complete gameplay from power-on.

| Reference event | Frame |
| --- | ---: |
| FIGHT! voice latch `0x94` | 2796 |
| FIGHT! hold ends / timer reload flag | 2969 |
| TIME reloads to 100 | 2970 |
| Decodable round-1 fight fixture, scene `0x80`, TIME 100 | **2975** |
| First TIME decrement to 99 | 2989 |
| TIME reaches 000 | **4870** |
| Time-over path clears scene mode | **4877** |
| Time-over voice latch `0x96` | 4878 |
| Round-result scene `0x20`, event 7 | **5651** |
| Result jingle latch `0xa7` | **5652** |

TIME still decrements every 19 frames while the slow fight loop sleeps six frames per pass. The timer expiry leads to a time-over round result rather than a match winner.

### C-REF: deterministic reference captures

The local pair `fixed-fight-slow-01-run-{a,b}` under `tools/reference/local/captures/` contains 2,686 full checkpoints. All **13,433 observation files** are identical byte-for-byte: five files per checkpoint plus `frames.tsv`, `checkpoints.tsv` and `meta.tsv`. The regression test checks the exact file sets and every byte, the requested DIP readback, the calibrated inputs, and the event boundaries in the table.

### C-NAT: native comparison through the result

`crates/sf-engine/tests/fixed_fight_slow.rs` runs `run_cpu_fight_capture_with_inputs` from fixture 2975 through result 5652. It produces 2,678 frames including the fixture. Comparison over **2976..=5652** matches all **2,677 frames and checkpoints**: screen hash, sound latch and latch-changed every frame, plus full 384 × 224 pixels, video RAM and palette RAM at every checkpoint.

A separate public `observe_cpu_fight` pass compares all owned work/object RAM and every other work/object RAM byte after every frame. Only the task slots' saved SR/PC/USP words and stack memory `0xff8106..=0xff8631` may differ. No untranslated branch is reached through frame 5652 (`C-STOP`).

### C-NEG: Speed Normal

Starting from the same 2975 fixture with the same inputs, a temporary scenario changes only `Speed` to `Normal` (`DSW1 = 0xdfff`). The native Normal-speed run first differs from the slow reference at **frame 2982**, the first differing fight-loop wake. The negative scenario is generated in a scratch directory and is not a pinned coverage scenario.

## Not Covered

- Speed Normal is used only as the negative control; this is not a separate complete Normal-speed fixture comparison.
- Native comparison after the result jingle at 5652, including the reference-only frames through 5660.
- Audible output, other opponents or rounds, Speed effects outside this fixed fight, Freeze outside [freeze](freeze.md) (`F21`, `#61`), and power-on fight-DIP integration (`F69`).
