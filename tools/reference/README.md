# Reference baseline tooling

Tooling for the pinned MAME comparison baseline. See
[`../../../docs/reference/mame-baseline.md`](../../../docs/reference/mame-baseline.md)
for the full baseline record (build identity, machine configuration, fixed
fight, determinism evidence).

- `scenario.py` — scenario model: reference identity, machine set,
  frame-indexed logical inputs, duration, observation settings.
- `lua_gen.py` + `driver_template.lua` — generate the standalone Lua driver
  MAME runs; the driver applies inputs via ioport fields and captures
  screen hashes, pixel dumps, RAM CRCs/dumps, and the sound latch stream.
- `run_scenario.py` — one documented invocation runs a scenario and writes
  a capture directory under `local/captures/`.
- `compare_captures.py` — byte-compares two captures (observations only);
  exit 0 = identical, 1 = first difference reported.
- `run_audio_probe.py` — a scenario with sound-chip write taps, device
  output streams, a WAV, and optional instruction-level traces of the
  samples CPU (`--code-trace`) and of the 68000's vblank handler
  (`--m68k-trace`).
- `vblank_paths.py` — the vblank handler's per-frame paths, latencies and
  costs from a `--m68k-trace` capture (docs/reference/latch-timing.md).
- `search_win.py` — winning-script search tool (issue #46 / F6): extends a
  pinned scenario's input prefix fight by fight with a parametrized strategy
  family (`StrategySpec`: `shoryuken_hp`, `hadoken_hp`, `tatsumaki_hk`,
  normal/crouch/jump attacks, blocks, or idle; configurable start/end frames,
  cycle period, walk-in frames, and move offset), runs candidates in the
  pinned MAME build with a read-only Lua probe that never writes emulated
  memory, and reads the fight outcome (`0xffc009`, `0xffc003` / `0xffc004`,
  round wins, health, and slot-1 resume addresses). Supports evaluating a
  single candidate or searching over the strategy family (`--search`).
- `coverage_ledger.py` — translation coverage probe and ledger generator
  (issue #51 / F11, `docs/reference/coverage-ledger.md`): runs two MAME PC
  coverage probes (`:maincpu`, `:audiocpu`, `:audio2`) per pinned scenario,
  verifies `run-a == run-b`, maps every executed original address to the
  Rust routine(s) that cite it or to its documented gap, maps every
  translated routine to the scenarios that execute it, classifies every
  `note()` / `note_unrecovered()` site as reached or unreached, and runs the
  negative control (`C-NEG`).
- `scenarios/sf_fixed_fight_01.json` — the fixed fight (issue #2).
- `scenarios/sf_fixed_fight_audit_01.json` — the fixed fight's inputs run
  on to frame 5040 with a full checkpoint after every frame from 2389,
  for the milestone audit (issue #39, docs/reference/milestone-audit.md);
  record two runs (`fight-audit-run-a`, `-b`) with `run_scenario.py`.
- `scenarios/sf_win_02.json` — `sf-win-01` extended through Geki, Bonus 1
  (`0x40d30`) and the flight into stage 1's first fight (`0x1734c`, issue #46;
  docs/reference/match-win.md); record two runs (`win-02-run-a`, `-b`) with
  `run_scenario.py`.
- `scenarios/sf_win_03.json` — `sf-win-02` extended through Joe's defeat and
  Mike's setup to Mike's first untranslated hit function (`0x4699c`, issue
  #68; docs/reference/joe.md); record two runs (`win-03-run-a`, `-b`) with
  `run_scenario.py`.
- `scenarios/sf_win_04.json` — `sf-win-03` extended through Mike's defeat,
  the stage-1 bonus and stage 2's arrival; native replay reaches the
  untranslated bonus entry `0x41740` (issue #84; docs/reference/mike.md).
  Record two runs (`win-04-run-a`, `-b`) with `run_scenario.py`.
  The other scenarios are listed by the reference documents that use them.
- `tests/` — `python -m unittest discover -s tests -t .` from this directory.

`local/` holds the MAME distribution, ROMs, and all captures. It is
excluded from Git; keep original game data and data-bearing captures out
of commits.
