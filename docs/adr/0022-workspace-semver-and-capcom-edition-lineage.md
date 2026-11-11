# 22. Workspace SemVer Schedule and Capcom Edition Lineage (`v1.0.0` – `v5.0.0`)

Date: 2026-11-09

## Status

Accepted (`Spec #345: Project Versioning & Capcom Edition Lineage`)

## Context

All 10 crates in the repository previously hardcoded `version = "0.1.0"` independently, no git tags marked completed project milestones, `sf-game` had no `--version` flag or startup version log line, and the Capcom **Edition lineage** in `CONTEXT.md` (*Street Fighter: The World Tournament*, *Street Fighter'*, *Anniversary Edition*, *Super Street Fighter: Rising*, *Hyper Street Fighter: Bloodlines*, *Ultra Street Fighter: Final Round*) was not linked to a concrete SemVer rule or protocol compatibility hash for Rollback Netplay (`ADR-0018`) and `.sf1replay` files (`ADR-0019`).

In addition, because code-only projects (**Project 7: Rollback Netplay**, **Project 5: Endings, Dramatic Battle & World Tour**, and **Project 8: Training Drills, Combo Trials & Replay Takeover**) do not wait on the November 27 PixelLab art renewal—whereas **Project 4's** final two sprite tickets (`ES12`–`ES13`) and **Projects 6, 9, and 10** (`Wave 1`–`Wave 3` art gates `BC8`–`BC10`, `BW8`–`BW11`, `UW9`–`UW13`) do—non-wave projects can finish before Wave 1 (and before Project 4's art tickets).

## Decision

1. **Single Workspace Version in Root `Cargo.toml` (`[workspace.package]`):**
   - Root `Cargo.toml` defines `[workspace.package] version` and `edition = "2021"`, and all 10 workspace crates inherit `version.workspace = true` and `edition.workspace = true`.
2. **Order-Independent Edition-Lineage SemVer Rule (`v1.0.0` – `v5.0.0`):**
   - **Major versions (`1.0`, `2.0`, `3.0`, `4.0`, `5.0`)** map strictly to the **Capcom Edition Lineage**:
     - `1.0.0` (`v1.0.0`): ***Street Fighter: The World Tournament*** (Projects 1 & 2 baseline at `#245`: Faithful 1987 PC port, Overhaul Front End, CRT Looks, Stage Select, Jukebox & Tracker Modules).
     - `1.x.0` (`>= 1.1.0`): ***Street Fighter': The World Tournament*** (Project 3 `SF1+` at `SP20` `#268` / `VR1` `#346` = `1.1.0`, plus any non-wave projects that complete while `1.x` is active).
     - `2.x.0` (`>= 2.0.0`): ***Street Fighter: The World Tournament — Anniversary Edition*** (Project 4 `Spec #269`: Multi-Era Styles & Gauges `SF1`–`SF6` bumps major version to `2.0.0`).
     - `3.x.0` (`>= 3.0.0`): ***Super Street Fighter: Rising*** (Project 6 `Spec #292`: Bonus Challengers — Wave 1 `RELEASED_WAVES = 1` at `BC10` `#302` bumps major version to `3.0.0`).
     - `4.x.0` (`>= 4.0.0`): ***Hyper Street Fighter: Bloodlines*** (Project 9 `Spec #319`: Bonus Challengers — Wave 2 `RELEASED_WAVES = 2` at `BW11` `#330` bumps major version to `4.0.0`).
     - `5.x.0` (`>= 5.0.0`): ***Ultra Street Fighter: Final Round*** (Project 10 `Spec #331`: Bonus Challengers — Wave 3 `RELEASED_WAVES = 3` at `UW13` `#344` bumps major version to `5.0.0`).
   - **Minor versions (`+0.1.0`)** increment (`X.Y.0` → `X.(Y+1).0`) whenever a non-wave feature project (**Project 7** `NP8 #311`, **Project 5** `ED8 #291`, or **Project 8** `TRD6 #318`) finishes, on top of whichever major edition `X` is currently active on `main`.
3. **`sf_engine::version` & Surfacing Rules:**
   - `crates/sf-engine/build.rs` embeds `SF1_GIT_SHA`, and `crates/sf-engine/src/version.rs` exposes `VERSION`, `GIT_SHA`, `EDITION_NAME`, `short_stamp()`, `full_banner()`, and `protocol_version_hash()`.
   - Surfaced in `sf-game --version` / `-V`, line 1 of `sf-game.log`, the OS window title, the top-right corner of the `SETTINGS` screen header (`short_stamp()`), and the Rollback Netplay (`NP2`) / `.sf1replay` (`TRD5`) compatibility headers.
4. **GitHub Releases (November 10, 2026 amendment):**
   - Every published version must have both an annotated Git tag and a published GitHub Release for that same tag, including a Windows executable package with its required runtime files, notices, launch instructions and SHA-256 checksum. Build each package from its tagged source; keep local ROMs and private asset packs separate. Release notes describe the shipped scope, actual validation and material verification limits. Verify the remote tag, published release and downloadable binary assets before calling a version released. This also applies to the historical `v1.0.0` and `v1.1.0` tags.
5. **Fix deliveries (November 10, 2026 amendment):**
   - Every bug-fix or patch delivery increments the SemVer PATCH component. Include the workspace version and lockfile updates with the fix, and verify the executable's reported version before completing or pushing it. The edition and feature-milestone rules above still determine major and minor increments.
6. **Edition branding (November 11, 2026 amendment):**
   - At each edition-name milestone, update the README title and Main Menu logo to match `EDITION_NAME`. The 1987 attract screen uses the original arcade logo.

## Consequences

- Every Edition Lineage release has a deterministic major version (`1.0`–`5.0`), and non-wave projects (`Netplay`, `Modes`, `Training/Replays`) can land in any order (including before Project 4's or Wave 1's Nov 27 PixelLab art tickets) with a clean `+0.1.0` minor bump and git tag.
