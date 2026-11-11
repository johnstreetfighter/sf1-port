# Difficulty, Round-Time and Attract-Music DIPs in the Fixed Fight (Tickets #59 / F19, #141 / F69)

- **Label:** `DF, CF variants`, `PO / CF`
- **Tickets:** `#59` (`F19`), `#141` (`F69`)
- **Size:** 0; adds reference scenarios and verification from both fixture `2389` (`F19`) and power-on (`F69`).

## Verified Boundary

The four scenarios preserve all six input events from `sf-fixed-fight-01`. Each reference run starts at power-on on the pinned MAME 0.289 baseline, applies only the named DIP changes, and has full checkpoints over `0..=830` and after every frame from `2389` through its capture end. Each capture's frame-2389 checkpoint decodes as a `CpuMachine` fight fixture (`F19`), and `PowerOnMachine` matches each capture from frame `0` through its match decision (`F69`, `crates/sf-engine/tests/dip_switches.rs`), including owned work RAM at frame `2389` and `C-NEG` catching a one-step-off `Round Time Count` (`100` vs `150` in `sf-fixed-fight-dsw-01`) at the first `TIME` digit (`frame 2390` on `checkpoint screen pixels` when `0x3e79c` reloads `0xff8e18..=0xff8e1a` and draws the HUD `TIME` digits).

| Scenario | DIP settings | DSW1 / DSW2 | Capture frames | Match decision (`0xffc009 != 0`) | Native comparison |
| --- | --- | --- | --- | --- | --- |
| `sf-fixed-fight-dsw-01` | Difficulty Easy, Round Time Count 150 | `0xdfff` / `0xffd7` | `0..=5101` | **5100** | `2390..=5100` (2,711 frames) |
| `sf-fixed-fight-dsw-02` | Difficulty Difficult, Round Time Count 200 | `0xdfff` / `0xffaf` | `0..=4800` | **4768** | `2390..=4768` (2,379 frames) |
| `sf-fixed-fight-dsw-03` | Difficulty Very Difficult, Round Time Count 250 | `0xdfff` / `0xff87` | `0..=4700` | **4647** | `2390..=4647` (2,258 frames) |
| `sf-fixed-fight-dsw-04` | Attract Music Off | `0xddff` / `0xffff` | `0..=5100` | **5015** | `2390..=5015` (2,626 frames) |

The decision is the first frame-end at which the match-winner byte `0xffc009` becomes nonzero; the preceding frame is zero in each capture. Every capture continues past that decision. These are controlled DIP variants from development fixtures, not claims of complete gameplay from power-on.

### C-REF: deterministic reference captures

The local capture pairs are `fixed-fight-dsw-01-run-{a,b}` through `fixed-fight-dsw-04-run-{a,b}` under `tools/reference/local/captures/`.

- `sf-fixed-fight-dsw-01`: 3,544 checkpoints (`0..=830` and `2389..=5101`); 17,723 identical observation files.
- `sf-fixed-fight-dsw-02`: 3,243 checkpoints (`0..=830` and `2389..=4800`); 16,218 identical observation files.
- `sf-fixed-fight-dsw-03`: 3,143 checkpoints (`0..=830` and `2389..=4700`); 15,718 identical observation files.
- `sf-fixed-fight-dsw-04`: 3,543 checkpoints (`0..=830` and `2389..=5100`); 17,718 identical observation files.

Each count is five observation files per checkpoint plus `frames.tsv`, `checkpoints.tsv` and `meta.tsv`. The regression test checks the exact file sets and every byte rather than relying on these totals alone. `meta.tsv` agrees with the requested DSW words at both ends of each run.

### C-NAT: native fixed fights through the decisions

`crates/sf-engine/tests/fixed_fight_dips.rs` runs `run_cpu_fight_capture_with_inputs` from each frame-2389 fixture through its recorded decision. On every frame it compares screen hash, sound latch and latch-changed; on every checkpoint it also compares full 384 × 224 pixels, video RAM and palette RAM. The comparison counts equal the native ranges in the table.

A separate public `observe_cpu_fight` pass compares all owned work/object RAM and every other work/object RAM byte after every frame. Only the task slots' saved SR/PC/USP words and stack memory `0xff8106..=0xff8631` may differ. No run reaches an untranslated branch through its decision (`C-STOP`).

### TIME reloads and Retsu's ROM tables

At frame 2390, both reference and native work RAM contain these three TIME digit bytes at `0xff8e18..=0xff8e1a`:

- Easy / 150: `01 05 00`.
- Difficult / 200: `02 00 00`.
- Very Difficult / 250: `02 05 00`.

The fixtures' decoded difficulty bytes at `0xffc007` are Easy `1`, Difficult `2`, Very Difficult `3` and Normal `0`. After wiping the destination ranges, public `cpu_opponent_setup` reproduces each capture's Retsu AI tables at `0xffc058..=0xffc663` and move-table pointer bytes at `0xffc676..=0xffc679` exactly from the selected difficulty's ROM rows. Easy's table image differs from Normal's.

### Attract Music Off latch filtering

Across the complete reference captures `0..=5100`, neither run A nor run B contains a latch command in `0x28..=0x3e`, and the two reference latch streams are identical. The native stream over `2390..=5015` is also identical to the reference stream and contains no command in that range. This verifies the live DSW1 filter at the latch boundary; it is not an audio-waveform comparison.

## Not Covered

- Difficulty settings other than the three named Retsu fights, other opponents, or independent difficulty/round-time combinations.
- Native comparison after the recorded match decisions, including the post-decision frames retained in the reference captures.
- Attract-mode music/demo behavior, audible output, Speed outside [fixed-fight-slow](fixed-fight-slow.md) (`F20`, `#60`), Freeze outside [freeze](freeze.md) (`F21`, `#61`), and the later power-on fight-DIP integration ticket (`F37`, `F69`).
