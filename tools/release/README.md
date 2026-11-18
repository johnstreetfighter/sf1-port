# Release packages

Windows comes first; the [Linux archive](#linux-archive) has its own section at the end.

## Publish with GitHub Actions

Push an annotated `vMAJOR.MINOR.PATCH` tag for the tested commit. The tag must
match the workspace version. GitHub Actions builds the tagged source on Windows,
verifies the ZIP and checksum, publishes a stable GitHub Release, then downloads
and verifies both assets. The run retains the verified files as an Actions
artifact. Its build job uses the runner's read-only token for public dependency
notices. The publishing job uses the runner's repository contents permission.

Run **Windows release** manually from the Actions tab to test the build,
packaging, and extracted executable on a branch. The run creates its tag inside
the checkout and uploads the verified files as an Actions artifact. Release
publication runs on tag pushes. The hosted check runs the extracted
`sf-game.exe --version` and inspects PE imports. Record desktop launch checks in
the release notes when performed on a Windows desktop.

## Build and package locally

When a release changes the edition name, update the first heading in the
[root README](../../README.md) and the Main Menu logo in the release
commit before tagging it. Use the exact edition name for the new version from
[ADR 0022](../../docs/adr/0022-workspace-semver-and-capcom-edition-lineage.md)
and check the heading and the logo shown in the tagged build against
`sf-game --version`.

Build a clean checkout of the exact release tag, with the normal tracker feature
enabled. Use a separate real target directory when another checkout is building;
shared target junctions can reuse an executable from the wrong revision.

```powershell
rtk proxy python tools/release/build-windows.py C:/path/to/isolated-target
rtk proxy powershell -NoProfile -ExecutionPolicy Bypass -File tools/release/package-windows.ps1 -Tag v1.2.1 -BuildDirectory C:/path/to/isolated-target/release -OutputDirectory C:/path/to/release-packages -SourceRoot C:/path/to/tagged-checkout
```

The packager reads locked Cargo metadata and copies `sf-game.exe`, the complete
shader tree, the playable art, voice, and character music packs, the source README/notices,
dependency license texts, `Cargo.lock`, launch instructions and source/build
identity. It produces a Windows x86_64 ZIP
and its SHA-256 sidecar. It rejects a tracked dirty source tree or a checkout that
does not equal the requested tag. For v1.1.0 onward it also runs the executable's
version flag and requires both the release version and exact Git revision.
v1.0.0 predates that flag and workspace versioning; `BUILD.txt` records this.
The execution-policy option applies only to that PowerShell process; it leaves
the user's policy unchanged.

Keep `rtk`, Cargo and an authenticated `gh` CLI available. Some registry archives
omit their workspace's license files. The packager downloads those texts from
the upstream GitHub or GitLab revision in the crate's `.cargo_vcs_info.json`,
records their provenance and fails if it cannot obtain them. Native dependency
license files are retained recursively. Network access is needed for these
pinned notice downloads.
One audited upstream omission, `vec_extract_if_polyfill` 0.1.0, has only an MIT
declaration in its manifest. Its original manifests and full source are retained
with a transparent provenance note; the packager does not invent a copyright
notice or grant. An unknown omission remains an error for review.

Before upload, extract the ZIP into a fresh directory. Compare the executable,
shaders and packs with the tagged build and source, verify ZIP CRC and SHA-256,
and inspect the Windows x64 GUI executable's runtime imports. Run the available version and
Windows launch checks from the extracted directory. Scan the executable and ZIP
for embedded workstation paths. Record the verification actually performed in
the release notes. Keep packages outside temporary worktrees.

Push the annotated tag for the tested commit and publish the GitHub Release with
the Windows ZIP and its SHA-256 sidecar. Mark stable releases as latest. Download
both published assets, compare them with the verified local files, and check the
ZIP against the downloaded sidecar before reporting the release complete.

The ZIP includes the playable packs under `assets/sf1-port-art`,
`assets/sf1-port-voice`, and `assets/sf1-port-music` beside the executable.
The packager uses each tagged manifest as its file list and includes the pack
README. The music manifest lists the 30 approved character scores. The verifier
checks manifest sizes and CRCs and compares every included pack file with the
tagged source. ROMs, the local title arrangement, source captures, listening
renders, raw art candidates, voice archives, settings, and logs stay outside
the ZIP. The quick start links Microsoft's x64 v14 runtime and explains where
users supply their own ROM set.

## Linux archive

The Linux x86_64 archive is for desktop Linux and Steam Deck. It is built and
verified by the **Linux release** workflow, which keeps the archive and its
SHA-256 sidecar as an Actions artifact. A version tag does not publish it yet:
that waits for the Steam Deck checks.

The runtime baseline is Ubuntu 22.04 (glibc 2.35). `build-linux.sh` builds the
locked executable with the default tracker feature inside a fresh
`ubuntu:22.04` container, which is what fixes the oldest system the package
runs on:

```bash
docker run --rm -e RUSTUP_TOOLCHAIN=1.96.0 -v "$PWD":/source -v /path/to/isolated-target:/build ubuntu:22.04 sh /source/tools/release/build-linux.sh
```

```bash
python3 tools/release/package-linux.py --tag v1.5.27 --build-directory /path/to/isolated-target/release --output-directory /path/to/release-packages
```

```bash
python3 tools/release/verify-linux-package.py /path/to/release-packages/sf1-port-v1.5.27-linux-x86_64.tar.gz /path/to/isolated-target/release/sf-game . v1.5.27 <full commit>
```

The packager applies the Windows packager's rules to the same tagged source:
`sf-game`, the complete shader tree, the three playable packs from their
manifests, the README and notices, dependency license texts for the Linux
dependency set, `Cargo.lock`, `QUICKSTART.txt` and `BUILD.txt`. The archive is
`sf1-port-vX.Y.Z-linux-x86_64.tar.gz`, with root ownership, fixed modes and
sorted members, and `sf-game` marked executable. Its `QUICKSTART.txt` carries
the Steam Deck steps: adding `sf-game` as a non-Steam game in Desktop Mode,
choosing the ROM set once with the game's ROM browser, and entering a room
code with the built-in controls. The dependency notice rules
live in `release_notices.py` for Linux and in `package-windows.ps1` for
Windows; change both together.

The verifier checks the sidecar, every member against the tag, and the
executable against the build. It reads the ELF file: x86-64, the standard
loader, no library search path, no linked library beyond glibc's own,
`libgcc_s`, `libstdc++`, `libasound` and `libudev`, and no glibc symbol newer
than 2.35. It then extracts the whole archive and runs `sf-game --version` from
another working folder.

`check-linux-launch.sh` runs in a second fresh `ubuntu:22.04` container with
runtime packages only, as an ordinary user. It launches the extracted game on a
virtual display, checks that settings and the log land in
`~/.local/share/sf1-port`, replaces the package folder and checks that a
changed setting and the player's ROM folder are still there, and runs
`sf-game smoke` from another folder to see the looks load from beside the
executable. The display is Mesa's software Vulkan driver: real graphics drivers,
sound, controllers and the art and voice packs (which need a ROM set) are
checked by hand on a desktop or a Steam Deck and recorded in the release notes.
