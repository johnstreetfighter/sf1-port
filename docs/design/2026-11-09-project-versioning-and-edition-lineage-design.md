# Project Versioning & Capcom Edition Lineage

- **Date:** 2026-11-09
- **Status:** Approved
- **Project:** Cross-Cutting Versioning Infrastructure (`VR1`) & Order-Independent Edition Lineage SemVer Rules
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`Cargo.toml`](../../Cargo.toml), [`2026-11-02-overhaul-front-end-design.md`](2026-11-02-overhaul-front-end-design.md), [`2026-11-03-bonus-roster-by-lore-design.md`](2026-11-03-bonus-roster-by-lore-design.md), [`2026-11-09-rollback-netplay-design.md`](2026-11-09-rollback-netplay-design.md), [`2026-11-09-training-drills-replay-takeover-and-control-schemes-design.md`](2026-11-09-training-drills-replay-takeover-and-control-schemes-design.md), [`ADR-0018`](../adr/0018-rollback-netplay-and-virtual-net-bus-on-the-1987-engine.md), [`ADR-0019`](../adr/0019-utility-ai-control-schemes-and-replay-takeover-on-the-1987-engine.md), [`ADR-0022`](../adr/0022-workspace-semver-and-capcom-edition-lineage.md)

---

## 1. Intent

Today, all 10 crates in the Cargo workspace hardcode `version = "0.1.0"` independently, there are no git tags, `sf-game` has no `--version` flag or startup version log line, and the Capcom **Edition lineage** defined in [`CONTEXT.md`](../../CONTEXT.md) (*Street Fighter: The World Tournament* → *Street Fighter'* → *Anniversary Edition* → *Super Street Fighter: Rising* → *Hyper Street Fighter: Bloodlines* → *Ultra Street Fighter: Final Round*) is not connected to a formal SemVer rule.

Furthermore, because code-only projects (**Project 7: Rollback Netplay**, **Project 5: Endings, Dramatic Battle & World Tour**, and **Project 8: Training Drills, Combo Trials & Replay Takeover**) do not wait on the November 27 PixelLab art renewal—whereas **Project 4's** final two sprite tickets (`ES12`–`ES13`) and **Projects 6, 9, and 10** (`Wave 1`–`Wave 3` art gates `BC8`–`BC10`, `BW8`–`BW11`, `UW9`–`UW13`) do—non-wave projects will finish before Wave 1 (and potentially before Project 4's art tickets).

This design establishes:
1. A single workspace-wide versioning source of truth in root `Cargo.toml` (`[workspace.package]`).
2. An **Order-Independent Edition-Lineage SemVer Rule**:
   - **Major versions (`1.x`, `2.0`, `3.0`, `4.0`, `5.0`)** are tied strictly to the **Capcom Edition Lineage**.
   - **Minor versions (`+0.1.0`)** increment whenever a non-wave feature project (`SF1+`, `Rollback Netplay`, `Endings/Dramatic/World Tour`, `Training/Replays`) completes, on top of **whichever major edition is currently active on `main` when that project finishes**.
3. Clean version surfacing across CLI (`--version`), `sf-game.log`, the OS window title, and the `SETTINGS` frame header, plus `protocol_version_hash()` for Rollback Netplay (`NP2` `#305`) and `.sf1replay` (`TRD5` `#317`).

---

## 2. Order-Independent SemVer & Capcom Edition Lineage Rules

### 2.1 Major Versions = Capcom Edition Lineage Milestones (`1.0` – `5.0`)
Each major version corresponds to a named Capcom Edition in [`CONTEXT.md`](../../CONTEXT.md) (*Edition lineage*):

| Major Series | Edition Lineage Name (`EDITION_NAME`) | Milestone Trigger |
|---|---|---|
| **`1.0.0`** | ***Street Fighter: The World Tournament*** | **Projects 1 & 2 (Completed at `#245`):** Faithful 1987 PC port (137/137 MAME parity scenarios), Overhaul Front End, GPU CRT Looks, Stage Select, Jukebox & Tracker Modules |
| **`1.x.0`** (`>= 1.1.0`) | ***Street Fighter': The World Tournament*** | **Project 3 (`Spec #246`, closes at `SP20` `#268` / `VR1` `#346`):** `SF1+` on the real engine (`1.1.0`), plus any non-wave projects (`Netplay`, `Modes`, `Training`) that finish while `1.x` is the active major edition |
| **`2.x.0`** (`>= 2.0.0`) | ***Street Fighter: The World Tournament — Anniversary Edition*** | **Project 4 (`Spec #269`, Era Styles & Gauges `SF1`–`SF6`):** Bumps major version to **`2.0.0`** (plus `+0.1.0` for any non-wave projects that finish while `2.x` is active) |
| **`3.x.0`** (`>= 3.0.0`) | ***Super Street Fighter: Rising*** | **Project 6 (`Spec #292`, closes at `BC10` `#302`):** Bonus Challengers — Wave 1 (`RELEASED_WAVES = 1`) bumps major version to **`3.0.0`** |
| **`4.x.0`** (`>= 4.0.0`) | ***Hyper Street Fighter: Bloodlines*** | **Project 9 (`Spec #319`, closes at `BW11` `#330`):** Bonus Challengers — Wave 2 (`RELEASED_WAVES = 2`) bumps major version to **`4.0.0`** |
| **`5.x.0`** (`>= 5.0.0`) | ***Ultra Street Fighter: Final Round*** | **Project 10 (`Spec #331`, closes at `UW13` `#344`):** Bonus Challengers — Wave 3 (`RELEASED_WAVES = 3`, Complete 40-Fighter Roster) bumps major version to **`5.0.0`** |

### 2.2 Minor Versions (`+0.1.0`) = Non-Wave Feature Projects (In Completion Order)
Whenever a non-wave project completes its closing ticket, it bumps the **minor** version (`X.Y.0` → `X.(Y+1).0`) of whichever major series `X` is currently on `main` and creates the corresponding annotated git tag `vX.(Y+1).0`:
- **Project 3 (`SF1+`, `SP20` `#268` / `VR1` `#346`):** Bumps `1.0.0` → **`1.1.0`** (`v1.1.0`, *Street Fighter': The World Tournament*).
- **Project 7 (`Rollback Netplay`, `NP8` `#311`):** Bumps current `X.Y.0` → **`X.(Y+1).0`** (e.g., `1.2.0` if completed before Project 4's final art tickets `ES12`/`ES13`, or `2.1.0` if completed after Project 4).
- **Project 5 (`Endings, Dramatic Battle, & World Tour`, `ED8` `#291`):** Bumps current `X.Y.0` → **`X.(Y+1).0`**.
- **Project 8 (`Training Drills, Combo Trials, Replay Takeover, Utility AI & Controls`, `TRD6` `#318`):** Bumps current `X.Y.0` → **`X.(Y+1).0`**.

---

## 3. Architecture & Implementation (`VR1`)

### 3.1 Single Source of Truth in Root `Cargo.toml` (`[workspace.package]`)
- Define `[workspace.package]` in root [`Cargo.toml`](../../Cargo.toml):
  ```toml
  [workspace.package]
  version = "1.1.0"
  edition = "2021"
  ```
- Update all 10 workspace crates (`crates/sf-compare`, `crates/sf-video`, `crates/sf-dev`, `crates/sf-host`, `crates/sf-game`, `crates/sf-engine`, `crates/sf-openmpt`, `crates/sf-audio`, `crates/sf-art`, `tools/mugen-extract`) to inherit:
  ```toml
  version.workspace = true
  edition.workspace = true
  ```

### 3.2 Compile-Time Git SHA & `sf_engine::version` Module
- Add `crates/sf-engine/build.rs` that invokes `git rev-parse --short=8 HEAD` (falling back to `"nogit"` if built outside a git checkout) and emits:
  - `cargo:rustc-env=SF1_GIT_SHA=<sha>`
  - `cargo:rerun-if-changed=../../.git/HEAD`
  - `cargo:rerun-if-changed=../../.git/refs`
- Add `crates/sf-engine/src/version.rs` exposing:
  - `pub const VERSION: &str = env!("CARGO_PKG_VERSION");`
  - `pub const GIT_SHA: &str = env!("SF1_GIT_SHA");`
  - `pub const fn edition_name(version: &str) -> &'static str`: maps the major/minor prefix of `VERSION` (`"1.0"` → *Street Fighter: The World Tournament*, `"1."` → *Street Fighter': The World Tournament*, `"2."` → *Street Fighter: The World Tournament — Anniversary Edition*, `"3."` → *Super Street Fighter: Rising*, `"4."` → *Hyper Street Fighter: Bloodlines*, `"5."` → *Ultra Street Fighter: Final Round*).
  - `pub const EDITION_NAME: &str = edition_name(VERSION);`
  - `pub const fn short_stamp() -> &'static str`: uppercase `"V" + env!("CARGO_PKG_VERSION")` (e.g. `"V1.1.0"`) for the 1987 ROM font on the Settings screen.
  - `pub fn full_banner() -> String`: returns `format!("{EDITION_NAME} v{VERSION} ({GIT_SHA})")`.
  - `pub const fn protocol_version_hash() -> u32`: deterministic 32-bit FNV-1a hash over `VERSION` and `GIT_SHA` used by Rollback Netplay (`NP2` `#305` `NetMessage::Hello.engine_version_hash`) and `.sf1replay` (`TRD5` `#317` `MatchReplay.engine_version`).

### 3.3 Where the Version Is Surfaced
1. **CLI (`sf-game --version` / `-V` and `sf-dev --version` / `-V`):**
   - Passing `--version` or `-V` prints `sf_engine::version::full_banner()` (e.g. `Street Fighter': The World Tournament v1.1.0 (a1b2c3d4)`) to stdout and exits with code `0`.
2. **Startup Log (`sf-game.log`):**
   - Immediately after `Log::create` in `crates/sf-game/src/main.rs`, log `log.line("version", &sf_engine::version::full_banner())`.
3. **OS Window Title (`crates/sf-host/src/gpu_window.rs`):**
   - Set the window title to `format!("{} (v{})", sf_engine::version::EDITION_NAME, sf_engine::version::VERSION)`.
4. **Settings Screen Header Stamp (`crates/sf-engine/src/front/settings.rs`):**
   - Render `sf_engine::version::short_stamp()` (`"V1.1.0"`) in dim `3×5` font inside the top-right corner of the `SETTINGS` frame header (`y = 24`, right-aligned to column `44`), leaving the 1987 attract loop, Main Menu, Character Select, Stage Select, and fight screens completely untouched.
5. **Replay (`TRD5`) & Rollback Netplay (`NP2`) Compatibility:**
   - Both `NetMessage::Hello` (`NP2` `#305`) and `MatchReplay` (`TRD5` `#317`) read `sf_engine::version::VERSION` and `sf_engine::version::protocol_version_hash()` from `sf_engine::version`.
