# Windows release packages

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
rtk cargo build --release --locked -j 2 --target-dir C:/path/to/isolated-target -p sf-game --bin sf-game
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

Before upload, extract the ZIP into a fresh directory, compare the executable and
shader files with the tagged build/source, verify the SHA-256, inspect PE runtime
imports and run the available version check from the extracted directory. Record
any runtime/interactive checks actually performed in the release notes. Keep
packages outside a temporary worktree before archiving that worktree.

Upload both files to the published GitHub Release for the annotated tag. The ZIP
contains no ROMs, private art or voice packs, user settings or logs. Do not copy a
whole build or local asset folder into the release. Dependency and shader notices
remain beside the executable. The quick start links Microsoft's current x64 v14
runtime and explains where users supply their own ROM set.
