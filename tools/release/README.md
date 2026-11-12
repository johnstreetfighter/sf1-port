# Windows release packages

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
shader tree, the source README/notices, dependency license texts, `Cargo.lock`,
launch instructions and source/build identity. It produces a Windows x86_64 ZIP
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

Before upload, extract the ZIP into a fresh directory. Compare the executable and
shaders with the tagged build and source, verify ZIP CRC and SHA-256, and inspect
the Windows x64 GUI executable's runtime imports. Run the available version and
Windows launch checks from the extracted directory. Scan the executable and ZIP
for embedded workstation paths. Record the verification actually performed in
the release notes. Keep packages outside temporary worktrees.

Push the annotated tag for the tested commit and publish the GitHub Release with
the Windows ZIP and its SHA-256 sidecar. Mark stable releases as latest. Download
both published assets, compare them with the verified local files, and check the
ZIP against the downloaded sidecar before reporting the release complete.

The ZIP contains no ROMs, art or voice packs, user settings or logs. Package the
release with the script above so dependency and shader notices remain beside the
executable. The quick start links Microsoft's x64 v14 runtime and explains where
users supply their own ROM set.
