# Default US East relay VPS and hands-off operations: design

- **Date:** 2026-11-16
- **Issue:** [#130](https://github.com/johnstreetfighter/sf1-port/issues/130) ("Operate the default relay in US East", parent [#123](https://github.com/johnstreetfighter/sf1-port/issues/123))
- **Size:** Project (spans US East VPS and Hostinger DNS provisioning, hands-off TLS/systemd/OS setup, CI prebuilt relay artifact and scheduled health-check alerting, flipping `DEFAULT_RELAY` across engine online tests and docs, and live two-network and outage verification).

## Intent

The QUIC relay crate (`crates/sf-relay`) and community relay selection (`#128`) are already implemented on `main`, but `sf_relay::endpoint::DEFAULT_RELAY` in `crates/sf-relay/src/endpoint.rs` is still `None`. Because fresh installs inherit `None`, **Multi Player → Online → Host Match** hides **VISIBILITY → INTERNET CODE** until a player manually types a community relay under **Settings → Netplay → RELAY**.

Operating a stable, low-cost US East default relay at `relay.almamoto.com` and shipping it as `DEFAULT_RELAY` lets players on Windows, Linux, Steam Deck, and the macOS preview host and join Internet rooms by six-character code out of the box with zero router port forwarding, VPNs, or settings changes, while remaining set-and-forget for a solo maintainer.

## Goals

1. **Run the project's default QUIC relay in US East at `relay.almamoto.com` (UDP `7443`)** using the maintainer's existing `almamoto.com` domain on Hostinger DNS for $0 extra domain cost (user).
2. **Keep day-to-day server operation as hands-off as possible at ~$5/month** with predictable, month-to-month billing and zero bandwidth-overage risk (user).
3. **Automate TLS issuance/renewal, service restarts, OS security patching, release binary builds, and external down-alerts** without requiring third-party monitoring SaaS accounts (agreed in brainstorming).
4. **Enable `DEFAULT_RELAY = Some("relay.almamoto.com")` in official builds** once the live endpoint passes `sf-relay --check`, and verify real two-network match play, capacity headroom, and clean relay-outage fallback to LAN/direct play per `#130`.

## Non-goals

- **Direct peer-to-peer NAT hole-punching / ICE fallback** (out of scope per `#123` and [ADR-0024](../adr/0024-internet-rooms-through-a-quic-relay.md)).
- **Multi-region or Anycast relay fleet** (start with one US East endpoint per `#123`; communities can self-host additional regional relays via `#128`).
- **Public Internet room browsing or matchmaking on the relay.**
- **Moving `almamoto.com` registration or DNS off Hostinger.**
- **External paid observability stacks** (Prometheus, Grafana, or PagerDuty SaaS).

## Anti-goals

- **Surprise traffic or flood bills:** Usage-metered plans where a traffic spike or UDP packet flood against a publicly hardcoded `7443/udp` endpoint could generate runaway egress charges.
- **Idle-reclaimed or renewal-trap hosting:** Free tiers that reclaim low-CPU VMs or promotional VPS plans (such as Hostinger KVM 1) that jump to $10–$12/month on renewal.
- **Sensitive data in routine logs:** Logging room codes, player IP addresses, or gameplay payloads anywhere on the server (`sf-relay` logs only its bind address and once-per-minute `rooms <N> connections <M>` counts).
- **Shipping a dead default endpoint:** Setting `DEFAULT_RELAY = Some("relay.almamoto.com")` in a tagged release before `sf-relay --check relay.almamoto.com` succeeds from outside the server, or allowing a relay outage to break LAN discovery and direct IPv4 play.

## Constraints

- **Dedicated public IPv4 + inbound `7443/udp` and `80/tcp`:** Players on IPv4 home and carrier NATs need a public IPv4 `A` record (`relay.almamoto.com`) and raw inbound UDP `7443` (no HTTP/CDN proxy in front of DNS). Certbot's standalone HTTP-01 challenge requires inbound TCP `80` reachable during initial issuance and 60-day renewals.
- **Trusted WebPKI TLS 1.3 + PKCS #8 key:** `sf-host` validates the relay certificate against `webpki-roots` and the hostname `relay.almamoto.com`; `sf-relay` requires a PKCS #8 private key (`BEGIN PRIVATE KEY`), handled by `tools/relay/renew-hook.sh`.
- **Protocol alignment with tagged builds:** The deployed `sf-relay` binary must be built `--locked` from the same release tag/protocol revision that players run (`docs/relay-deployment.md`).
- **Engine `Host Match` default shift when `DEFAULT_RELAY` becomes `Some`:** Setting `DEFAULT_RELAY = Some("relay.almamoto.com")` makes `Settings::default().relay_endpoint()` return `Some("relay.almamoto.com")` (`crates/sf-engine/src/front/settings.rs`), which causes **Multi Player → Online → Host Match** to default `VISIBILITY` to `INTERNET CODE` (fulfilling `#123` User Story 9) rather than `PUBLIC`. Existing `GameSession` LAN tests (`np3_sessions.rs`, `np4_online.rs`, `np5_*.rs`, `np6_sessions.rs`) that open **Host Match** with `Settings::default()` and expect `NetTransportCommand::BindHost` must be updated accordingly.
- **Human operator boundary (`ready-for-human`):** Account creation, payment, OVHcloud VPS provisioning, Hostinger DNS `A` record creation, and physical two-network / Steam Deck play checks require the maintainer; repository automation prepares exact copy-paste scripts/checklists, CI workflows, code, and tests.

## Approaches

We evaluated US East Linux VPS providers offering a dedicated public IPv4 address and raw inbound UDP, dropping options that undermine low predictable cost or hands-off reliability:
- *Dropped:* Hostinger KVM 1 (promotional pricing renews at ~$10–$12/month), IPv6-only $2.50–$3.50 cloud tiers (Amazon Lightsail $3.50, Vultr $2.50), Hetzner Ashburn (entry `CPX11` sits around ~$21/month in the US), and Oracle Cloud Always Free (signup capacity hurdles and idle-instance reclamation risk).

Three viable approaches remained:

1. **OVHcloud US `VPS-1` in Vint Hill, Virginia (~$4.54–$5.35/month month-to-month) — Chosen.**
   - *Specs:* 2 vCores, 4 GB RAM, 40 GB NVMe SSD, 1 dedicated public IPv4, **unmetered 500 Mbps bandwidth**, automated 24-hour daily backup, and **built-in edge L4/UDP anti-DDoS protection**, with no long-term commitment or renewal jump.
   - *Tradeoffs:* Costs ~$35/year more than an annual budget slice, and bills monthly rather than sub-cent hourly if spinning up a temporary second relay for `#128`.
2. **DigitalOcean Basic Droplet in New York (`NYC1`/`NYC3`, $4.00/month billed hourly).**
   - *Specs:* 1 vCPU, 512 MiB RAM, 10 GiB SSD, 1 dedicated IPv4, 500 GiB/month transfer (~3,000+ hours of matches).
   - *Tradeoffs:* Cleanest cloud panel and sub-cent hourly billing for temporary test nodes, but 512 MiB RAM cannot compile Rust on-box without swap, egress above 500 GiB incurs $0.01/GiB overages, and it lacks OVH's edge UDP game DDoS scrubbing.
3. **BuyVM `Slice` (Piscataway, NJ) or RackNerd Annual KVM (NY/NJ/Atlanta, ~$1.65–$3.50/month; $20–$24/year).**
   - *Specs:* 1 vCore, 512 MB–1 GB RAM, SSD/NVMe, 1 dedicated IPv4, unmetered (BuyVM $24/year or $3.50/month) or hard-capped 2–3 TB/month (RackNerd ~$20/year) with locked renewal prices.
   - *Tradeoffs:* Lowest annual outlay with zero overage risk, but smaller independent control panels, occasional stock waits on BuyVM's $24/year slice, and annual upfront billing on the ~$2/month tiers.

**Selection:** The maintainer chose **Approach 1 (OVHcloud US `VPS-1` in Vint Hill, Virginia)**. For ~$5/month month-to-month with no contract, unmetered bandwidth + built-in edge UDP DDoS protection + 4 GB RAM in Northern Virginia eliminates surprise egress or flood bills and keeps operations hands-off. Because `DEFAULT_RELAY` stores a DNS hostname (`relay.almamoto.com`) rather than a hardcoded IP, migrating hosts later requires only changing one Hostinger DNS `A` record.

## What was decided

### 1. Operator, hosting plan, DNS, bandwidth, and cost

- **Operator:** Maintainer (`johnstreetfighter`), recorded on `#130` before setting `DEFAULT_RELAY` in stable release builds.
- **Plan & region:** OVHcloud US (`us.ovhcloud.com`) `VPS-1` in Vint Hill, Virginia (`US-EAST-VA`), month-to-month (~$4.54–$5.35 USD/month + applicable tax; $0 bandwidth overage; $0 extra domain cost).
- **Included capacity:** 2 vCores, 4 GB RAM, 40 GB NVMe SSD, 1 public IPv4 address, unmetered 500 Mbps bandwidth, built-in edge L4/UDP DDoS mitigation, and 24-hour daily backup. At ~150 MB of relayed traffic per match-hour, 500 Mbps unmetered bandwidth accommodates thousands of concurrent matches well beyond `Limits::default()` (4,096 open rooms, 16 seats/room, 600 datagrams/s and 256 KiB/s per connection).
- *My call:* **OS image — Ubuntu 24.04 LTS (`x86_64`)**, matching GitHub Actions' `ubuntu-24.04` runner glibc baseline so CI-built `sf-relay` binaries run verbatim on the VPS with zero library mismatch.
- **DNS & domain renewal (settled in grilling, 2026-11-16):** Create an **IPv4 `A` record only (no `AAAA` record)** for `relay` → `<VPS IPv4>` under `almamoto.com` in Hostinger DNS (`relay.almamoto.com`, default UDP port `7443`), matching `0.0.0.0:7443` in `tools/relay/sf-relay.service` across IPv4, dual-stack, and 464XLAT mobile carrier networks, with any HTTP/CDN proxy disabled so raw UDP `7443` reaches the server directly. Verify Hostinger auto-renewal is enabled on `almamoto.com` during bring-up so shipped binaries never point at a lapsed domain.
- *My call:* Use `TTL = 300` seconds during initial bring-up and cutover, raising to `3600` seconds once verified.

### 2. Hands-off server configuration, TLS renewal, and logging

- **Idempotent setup script (settled in grilling, 2026-11-16):** Ship `tools/relay/setup-ubuntu.sh <domain> [--binary <path>] [--email <addr>]` for Ubuntu 24.04 so provisioning `relay.almamoto.com` (or a temporary community relay) configures everything in one repeatable pass: the `sf-relay` system account, `ufw` (`22/tcp`, `80/tcp`, `7443/udp`), Ubuntu `apt` `certbot` (avoiding `snapd` overhead) + `/usr/local/sbin/sf-relay-renew-hook` + `--dry-run`, `journald` retention, `unattended-upgrades`, and `tools/relay/sf-relay.service` (installing `--binary <path>` when provided).
- **SSH & firewall (with lockout guard, settled in grilling, 2026-11-16):** Enable `ufw` default-deny incoming with only `22/tcp`, `80/tcp`, and `7443/udp` allowed (mirrored in the OVHcloud edge firewall if enabled). `setup-ubuntu.sh` disables root SSH login and `PasswordAuthentication` in `sshd_config` **only** after verifying a non-empty `authorized_keys` file exists for the login user, refusing to disable password login otherwise.
- **TLS issuance & off-peak auto-renewal (settled in grilling, 2026-11-16):**
  - When `--email` is omitted, `setup-ubuntu.sh` invokes `certbot certonly --standalone -d <domain> --deploy-hook /usr/local/sbin/sf-relay-renew-hook --register-unsafely-without-email --agree-tos --non-interactive` so no personal email address is required or stored in repository scripts.
  - Keep `sf-relay` unchanged (no in-process certificate hot-reload code). Instead, install a systemd drop-in override on `certbot.timer` pinning renewal checks to `09:30 UTC` (`RandomizedDelaySec=30m`, ~04:30–05:30 AM US East) so 60-day `systemctl try-restart sf-relay.service` hooks never drop evening matches. Verify both the hook manually (`sudo RENEWED_LINEAGE=/etc/letsencrypt/live/relay.almamoto.com /usr/local/sbin/sf-relay-renew-hook`) and automated renewal (`sudo certbot renew --dry-run`), and confirm `certbot.timer` is enabled and active.
- **Systemd service & OS patching:** Install `tools/relay/sf-relay.service` (`MemoryMax=512M`, `TasksMax=64`, `LimitNOFILE=4096`, `CPUQuota=100%`, `Restart=on-failure`, enabled on boot). Enable `unattended-upgrades` with `Unattended-Upgrade::Automatic-Reboot "false"` (settled in grilling, 2026-11-16) so automatic security patches never trigger surprise mid-match reboots.
- *My call:* **Journal retention & privacy:** Add `/etc/systemd/journald.conf.d/sf-relay.conf` with `SystemMaxUse=100M` so years of once-per-minute `rooms <N> connections <M>` lines never exhaust disk; verify `journalctl -u sf-relay` contains only the listening address and counts (never room codes, client IPs, or gameplay payloads).

### 3. Binary deployment, CI artifact, and rollback

- *My call:* **CI workflow artifact (`workflow_dispatch` & `v*` tags):** Keep public GitHub Releases strictly limited to the audited player game packages (`ADR-0022` / `#123`), rather than attaching an un-audited server tarball to the Releases page. Have GitHub Actions build `sf-relay` (`cargo build --release --locked -p sf-relay --features net` on `ubuntu-24.04`, `x86_64-unknown-linux-gnu`) and upload `sf-relay` + `.sha256` as a **GitHub Actions workflow artifact** for operator download via `gh run download`. This leaves `crates/sf-relay` CLI flags (`--version` / `build.rs`) and dependency license-notice bundling untouched while still eliminating Rust toolchain maintenance on the VPS.
- **Initial pre-tag bring-up, source path, and rollback runbook:** Because OVHcloud `VPS-1` has 4 GB RAM, `docs/relay-deployment.md`'s tagged-source build remains the community deployment path and an on-box fallback (~60s compile time). During `#130`, bring up the VPS with `sf-relay` built from the candidate commit (via the Actions workflow artifact or on-box source build); once `#131` cuts the coordinated release tag, swap the VPS binary using the `docs/relay-deployment.md` rollback pattern (`cp /usr/local/bin/sf-relay /usr/local/bin/sf-relay.previous`, install and verify the new binary, `systemctl restart sf-relay.service`, and confirm externally with `sf-relay --check relay.almamoto.com`).

### 4. Zero-cost external health check and down-alerts

- **Early TLS expiry check (settled in grilling, 2026-11-16):** Extend `sf-relay --check` with an optional `--min-days <N>` flag (`sf-relay --check --min-days 14 relay.almamoto.com`), checking the presented server certificate chain with `rustls-webpki` at `now + N days` so failed renewals alert two weeks ahead of expiration, while keeping plain `sf-relay --check <host>` unchanged.
- **Scheduled CI health check & retries (settled in grilling, 2026-11-16):** Add `.github/workflows/relay-health.yml` running on `ubuntu-24.04` on a 6-hour `cron` schedule plus `workflow_dispatch`, with Cargo caching, retrying `sf-relay --check --min-days 14 relay.almamoto.com` up to **3 times spaced 10 seconds apart** before failing. This prevents single-packet UDP blips from triggering false alarms while still triggering GitHub's native workflow-failure email notification on real DNS, firewall, daemon, or upcoming-expiry failures at $0 cost.
- **60-day GitHub schedule pause notice:** Document GitHub Actions' 60-day repository-inactivity schedule pause (and day-53 warning email / one-click re-enable) in the operator runbook so scheduled alerts never lapse silently during quiet periods.

### 5. Two-stage rollout, setting `DEFAULT_RELAY`, and `#131` tag boundary

- **Release-tag boundary (`#130` vs `#131`):** `#130` blocks `#131` ("Verify and publish the coordinated cross-platform release") and `#132` while `#125`, `#126`, `#127`, and `#129` remain open, and `.github/workflows/windows-release.yml` publishes a `--latest` GitHub Release on every `v*` tag push. Therefore **`#130` does not bump `Cargo.toml` or push a `v*` release tag**—`#130` verifies using release-candidate builds from the same commit SHA, and `#131` owns cutting the coordinated annotated release tag (after which `relay.almamoto.com` swaps to that tag's binary).
- *My call:* **Two-stage repo rollout around human provisioning:**
  1. **Stage 1 (pre-provisioning, `DEFAULT_RELAY` stays `None`):** Land the VPS provisioning helpers/docs and `.github/workflows/relay-health.yml` with `workflow_dispatch` only (with the `cron` schedule disabled so scheduled checks do not fail before the server exists).
  2. **Human bring-up:** Maintainer provisions the OVHcloud `VPS-1` and Hostinger `A` record for `relay.almamoto.com`, installs `sf-relay` from the candidate commit, and confirms external `sf-relay --check relay.almamoto.com` exits `0`.
  3. **Stage 2 (post-check):** Flip `DEFAULT_RELAY = Some("relay.almamoto.com")` in `crates/sf-relay/src/endpoint.rs`, enable the 6-hour `cron` trigger in `.github/workflows/relay-health.yml`, update the engine test suite and documentation below, and run `#130`'s live acceptance checks.
- **Test updates (Stage 2):**
  - Keep `Settings::default().relay_endpoint()` using `DEFAULT_RELAY` so fresh installs default **Host Match → VISIBILITY** to `INTERNET CODE` (`#123` User Story 9).
  - Update `crates/sf-engine/tests/relay_setting.rs` and `crates/sf-engine/tests/internet_rooms.rs` to verify `DEFAULT_RELAY == Some("relay.almamoto.com")` and default `INTERNET CODE` visibility.
  - Update existing `GameSession` LAN host tests (`np3_sessions.rs`, `np4_online.rs`, `np5_*.rs`, `np6_sessions.rs`) that assume `Host Match` defaults to `VISIBILITY = PUBLIC` (`NetTransportCommand::BindHost`) so they explicitly switch `VISIBILITY` to `PUBLIC` (or clear the relay on the screen).
- **Documentation updates (Stage 2):** Update `docs/netplay.md` (remove the "no default relay runs yet" caveat and document `relay.almamoto.com`), `docs/relay-deployment.md` (mentioning the CI workflow artifact option alongside building from source), `docs/adr/0024-internet-rooms-through-a-quic-relay.md`, and `docs/session-handoff.md`.

### 6. Live acceptance verification (`#130` Criteria 2–4 & `#128` coordination)

- **Metrics & headroom:** Record idle and active RSS (`systemctl status sf-relay`), CPU usage, and `journalctl -u sf-relay` `rooms`/`connections` lines during live matches, verifying wide headroom beneath the 512 MB / 1 CPU systemd ceilings.
- **Two-network release-candidate match (settled in grilling, 2026-11-16):** Using two release-candidate builds built from the exact same Stage-2 commit SHA (via `workflow_dispatch` on `.github/workflows/windows-release.yml` or local `--release --locked` builds so `protocol_version_hash()` matches) between the maintainer's Windows PC on home broadband and a Steam Deck (or second machine) tethered to a phone cellular hotspot (carrier NAT, no port forwarding), host on `relay.almamoto.com` with shipped defaults, join by 6-character code, complete a Versus match and Rematch, and record observed peer RTT, jitter, input delay, and rollback behavior.
- **Outage & recovery drill:** Stop `sf-relay.service` (`sudo systemctl stop sf-relay`), confirm external `sf-relay --check relay.almamoto.com` fails, and verify **new** Host Match and Direct Join attempts while the relay is down report the `relay.almamoto.com` failure on `OnlineScreen` and leave the menu usable while LAN discovery and direct IPv4/LAN-code hosting/joining keep working; then `sudo systemctl start sf-relay` and confirm immediate recovery. (Mid-room/mid-match relay disconnect cleanup and pending-operation cancellation remain scoped to sibling ticket `#127`, so `#130` makes no runtime disconnect changes to `sf-host` or `sf-engine`.)
- **Closing `#128`'s third criterion alongside `#130` (settled in grilling, 2026-11-16):** Because `#130` is blocked by `#128` in GitHub Issues, run the temporary second relay on the **same** OVHcloud `VPS-1` (`relay2.almamoto.com:17443`, separate systemd unit `sf-relay-2.service`, separate Let's Encrypt certificate, `ufw allow 17443/udp`) so no second cloud account is required. Select `RELAY2.ALMAMOTO.COM:17443` in **Settings → Netplay → RELAY**, complete a match, verify failed-join isolation and continued operation while `sf-relay.service` (`7443/udp`) is stopped, restore `relay.almamoto.com` via `RESET TAB`, and tear down `relay2` (`sf-relay-2.service`, `17443/udp` rule, certificate, and Hostinger `A` record) once `#128` closes.

## What's left to the builder

- **Shell implementation of `tools/relay/setup-ubuntu.sh`:** Exact bash structure, CLI flag parsing (`<domain> [--binary <path>] [--email <addr>]`), and `certbot.timer` / `unattended-upgrades` drop-in file formatting.
- **Early expiry check in `crates/sf-relay`:** Exact `rustls-webpki` future-timestamp validation wiring for `--min-days <N>` in `sf-relay --check`.
- **CI workflow artifact placement:** Whether the `sf-relay` Linux workflow-artifact build step shares `.github/workflows/relay-health.yml` (uploading `sf-relay` + `.sha256` on `workflow_dispatch` / `v*` tag runs) or lives in a small separate workflow.
- **Health-check workflow implementation:** Exact Cargo cache keys and triggers in `.github/workflows/relay-health.yml`.
- **LAN test helper refactoring:** Exact helper signature in `crates/sf-engine/tests/` to switch **Host Match → VISIBILITY** to `PUBLIC` for existing `BindHost` integration tests.
