# The Freeze DIP

- **Label:** `DF / CF`, `PO / CF`
- **Tickets:** `#61` (`F21`), `#141` (`F69`)
- **Scenario:** `sf-freeze-01` (`tools/reference/scenarios/sf_freeze_01.json`)
- **Code:** `sf_engine::tasks::{vblank_prefix, vblank_suffix}`, `sf_engine::cpu::CpuMachine`, `sf_engine::run::{PowerOnMachine, P1Inputs}`
- **Verification:** `crates/sf-engine/tests/freeze.rs`, `crates/sf-engine/tests/dip_switches.rs`, `tools/reference/tests/test_scenario.py`, `tools/reference/tests/test_lua_gen.py`, `crates/sf-compare/tests/scenario.rs`

## Derived behavior and reference boundary

At `0x3b100–0x3b108`, the vblank handler repeatedly reads DSW1 bit 14 while Freeze is On. It does not reach the IN1 copy, sound-latch write, frame-word and sleeping-slot updates, `movem`/`RTE`, or the scheduler. When the bit reads 1, the same interrupted handler resumes at `0x3b10a`; later vblanks do not run the handler prefix repeatedly.

The cabinet's vblank is `irq1_line_hold`, and the handler masks level-1 interrupts. The vblanks during the spin therefore remain one held request, accepted after the interrupted handler's RTE. That held handler runs completely before game tasks resume. This control flow is derived from the original code and MAME's interrupt model; the capture verifies it rather than selecting it.

`sf-freeze-01` preserves the fixed fight's six input events. It starts with DSW1/DSW2 `0xdfff/0xffff`, changes Freeze On at frame 2500 and Off at frame 2560, and runs through frame 5180. Checkpoints cover `0..=830` and every frame `2389..=5180` (`3,623` checkpoints, `18,119` identical observation files). The reference runs are local-only `freeze-01-run-a` and `freeze-01-run-b`. Verified both from checkpoint `2389` (`F21`) and from power-on `0..=5175` on `PowerOnMachine` (`F69`, including owned RAM at `2389` and the 2,360-cycle double latch write at `2561`).

| Boundary | Reference result |
| --- | --- |
| Freeze On write / port readback | frame 2500 / frame 2501, `0x9fff/0xffff` |
| Freeze Off write / port readback | frame 2560 / frame 2561, `0xdfff/0xffff` |
| Entry prefix | 10 work-RAM bytes change from frame 2500 to 2501 |
| Frozen frames | frames `2502..=2560` have work, video and palette RAM identical to frame 2501 |
| Release | 67 work-RAM bytes change from frame 2560 to 2561 |
| Match decision | work-RAM offset `0x4009` (`0xffc009`) is zero through frame 5174 and first nonzero at frame 5175 |

The two runs are byte-identical across all 13,964 observation files: five files per checkpoint plus `frames.tsv`, `checkpoints.tsv`, `dip_readbacks.tsv` and `meta.tsv`. The captures and checkpoints extend five frames past the decision; native acceptance ends at the decision, frame 5175.

## Frame-scheduled DIP changes

Scenario `dip_changes` are cumulative and have strictly increasing frames. Each change resolves named MAME fields or explicit port words against the preceding DIP state. The reference driver writes changed fields through `field.user_value` at frame-done N and logs the actual port words at N+1 in `dip_readbacks.tsv`; final metadata is checked against the final scheduled state. The native one-player input schedule applies the same resolved words before producing N+1. Static DIP behavior is unchanged when no schedule exists.

The coverage probe uses the same field-write ordering, so the pinned Freeze scenario measures the spin and release rather than a run with Freeze permanently Off.

## Release latch timing

The release frame has two sound-latch writes, both `0xffff`, in the audio tap:

| Write | Time (seconds) |
| --- | --- |
| Interrupted handler's suffix | `42.683355375` |
| Complete held vblank | `42.683650375` |

Their separation is exactly 2,360 68000 cycles at 8 MHz. The next write is at `42.700150375` on frame 2562. The interval is accounted from the original instruction path, not fitted to the tap:

| Component | Cycles |
| --- | --- |
| Remainder of the latch-write instruction | 2 |
| Post-write instructions before the slot loop | 94 |
| Sleeping-slot loop | 970 |
| `movem`, RTE and held-exception entry | 196 |
| Held handler, first fetch through its latch write | 1,098 |
| **Total** | **2,360** |

For this release, the slot loop sees eleven non-sleeping slots, four slots waking and one slot remaining asleep, including the final DBRA cost. The native accounting derives this total from the actual slot states. `CpuMachine::latch_writes()` exposes no writes on frozen frames, one on ordinary frames and both writes on release; the second write's cycle count is cumulative so subtracting the first gives 2,360. Existing capture state still records the last write.

## Native and negative verification

- Entry, frozen and focused release windows match reference RAM.
- From fixture 2389 through decision 5175, native execution produces 2,787 checkpoints including the fixture. All 2,786 frames `2390..=5175` match `screen_hash`, `latch` and `latch_changed`; all 2,786 comparison checkpoints match pixels, video RAM and palette RAM.
- Every owned address and every non-slot/non-stack work/object-RAM byte matches over the same native window. No unrecovered branch is reached. Frozen frames have no latch writes, release has two and every other frame has one.
- **C-NEG:** changing only Freeze Off from frame 2560 to 2559 causes the first named native/reference RAM mismatch at frame 2560, while the reference is still frozen.

## Not Covered

- Freeze on the power-on or versus machines, or outside this fixed fight; their bounded paths still report the untranslated spin.
- Input edges during the spin, other freeze durations, repeated toggles, or Freeze set from power-on. This scenario holds an existing direction across the freeze but schedules attacks only after release.
- Native comparison after the match decision: reference frames `5176..=5180` are captured but outside the C-NAT endpoint.
- Audible output and a cycle-exact value for the initial interrupt-acceptance latency. The release's write-to-write interval is exact; the general latency band remains governed by [latch-timing](latch-timing.md).
- Frame overruns and cycle-by-cycle execution of arbitrary game tasks, which remain outside ADR-0001's translation boundary.
