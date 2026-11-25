# Project storage cleanup — 24 November 2026

The maintainer requested removal of unnecessary project files during SC18.
Removed **57,228,742,103 logical bytes (53.30 GiB)** of inactive Cargo output:

| Location | GiB removed |
| --- | ---: |
| Main `target/suite/incremental` | 21.23 |
| Main `target/debug` | 4.13 |
| Main `target/release` | 16.60 |
| `target-check/debug` | 0.17 |
| Two retired agent worktrees' `target/debug` profiles | 3.74 |
| Five historical Cargo profiles on D: | 7.43 |

The historical snapshots retain their sources, standalone binaries, shaders
and comparison outputs. Only `deps`, `build`, `incremental`, `.fingerprint`
and `examples` were removed from the selected historical Cargo profiles.
Worktree sources and Git registrations remain in place.

Retained the main optimized suite dependencies, build scripts and shaders;
SC17 measurement evidence and suite logs; SC18's verified baseline executable
and provenance; playtest outputs; ROMs, art, audio, captures, releases, trailers,
hash manifests and the live pinned MAME installation. Paths were resolved and
checked against the project and its storage directories before removal. No
sf1 Cargo, MAME or measuring job ran during deletion. An independent scope
review checked the retained inputs and evidence.

`profile.suite.incremental = false` prevents another large optimized
incremental cache accumulating across version and lockfile changes. The
development profile keeps its existing configuration. Removed build outputs
are recreated when their profiles are next needed.

After deletion, C: had approximately **32 GiB free** and D: **178 GiB free**.
Other projects were active, so these are observations, not a claim that all
changes in drive free space came from this cleanup. No I/O speed improvement
has been measured. Cleanup manifests and results are under `target/sc18/`.
SC18 uses focused validation; the maintainer explicitly waived the full suite.
