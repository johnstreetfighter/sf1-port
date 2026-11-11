# 0018. Rollback Netplay and Virtual Net Bus on the 1987 Engine

- **Status:** Accepted
- **Date:** 2026-11-09
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`docs/design/2026-11-09-rollback-netplay-design.md`](../design/2026-11-09-rollback-netplay-design.md), [`ADR-0004`](0004-reach-fights-by-driving-the-faithful-flow.md), [`ADR-0005`](0005-game-binary-pure-screens-shared-host.md), [`ADR-0015`](0015-era-styles-and-sub-hp-on-the-1987-engine.md), [`ADR-0016`](0016-dramatic-battle-and-composite-endings-on-the-1987-engine.md), [`ADR-0017`](0017-forty-fighter-roster-and-satsui-awakening-on-the-1987-engine.md)

## Context

Prior to [`ADR-0004`](0004-reach-fights-by-driving-the-faithful-flow.md), `crates/sf-engine/src/netplay.rs` contained a standalone prototype (`struct Sim { x: i32, hash: u64 }`) that stepped a 1D integer coordinate, while `MenuItem::Online` (`Multi Player -> Online`) and the 6th Settings tab (`Tab::Netplay`) remained disabled in the front-end.

`PowerOnMachine` (`save_snapshot` / `restore_snapshot`) and `Fight` already step the real translated 1987 engine. SF1+ is complete for the twelve tournament fighters; later-era rules, the expanded roster and three-fighter Dramatic Battle are future projects. We will replace the prototype with real rollback netplay on the current game while preserving ADR-0005's rule that `sf-engine` stays pure and free of OS sockets or wall-clock timers.

Specifically, we must decide:
1. How [`Fight`](../../crates/sf-engine/src/front/fight.rs) snapshots and restores its state across an 8-frame rollback window (`MAX_ROLLBACK_FRAMES = 8`) without unsigned frame-0 underflow, stuck round-over flags (`self.over`), or duplicate sound latches / announcer call-outs during multi-frame resimulation.
2. How `sf-engine` and `sf-host` divide responsibilities between the deterministic rollback protocol (`RollbackEngine`, `NetPacket`, `OnlineScreen`) and the non-blocking UDP socket / LAN broadcast / adapter-detection transport (`crates/sf-host/src/net.rs`), and how multi-peer rollback sessions (including 1v1 Versus, 2P Online Co-Op Dramatic Battle, and Live Spectating) are tested deterministically in-memory.

## Decisions

### 0. Delivery sequence correction (2026-11-10)

The maintainer authorized netplay after SF1+, before era styles, endings/Dramatic/World Tour and bonus challengers. Initial tickets NP1-NP8 (#304-#311) therefore deliver online 1v1 and live spectating using the existing twelve `TournamentFighter` values, current data bank/stages and SF1/SF1+ rules. Their prerequisites are completed SF1+ #246 and VR1 #346. This correction takes precedence over future-feature assumptions in the original detailed design below.

- NP1 does not depend on BC1 #293; NP4 does not depend on BC2 #294. The current select screen and menu inputs suffice.
- NP6 #309 implements spectating. Online Dramatic Battle is retained in NP9 #352 under #303, blocked by NP5 #308, NP6 #309 and local Dramatic setup #287.
- Later era tickets #270-#277 own style/loadout synchronization and rollback coverage of newly active era state. BC1 #293 owns expanded fighter wire IDs/roster compatibility, BC2 #294 owns expanded-select online routing, and BC6 #298 owns Satsui Awakening rollback tests. ED3 #286 owns third-fighter/team snapshot state for the gated co-op integration.
- Extend synchronized rules and protocol compatibility when those features land. Unsupported fighters, styles and room modes must not be advertised as playable. NP8 remains the baseline netplay minor-version milestone.

NP1 must audit the current `Fight` struct rather than treat the original field list below as exhaustive. In addition to those fields, account for mutable setup, continue progress, round-start snapshot, fed reads, hold-combo, Show and Coach state as applicable. Preserve peer-local overlay, gauge art and notation choices on restore. `Fight` and `CalloutBoard` already implement `Clone`.

The gameplay checksum uses explicit stable bytes for authoritative future-determining machine, scheduler, I/O, configuration, toggle and wrapper state. RAM/object RAM/scrolls alone are insufficient. Exclude local presentation, diagnostics and emitted-audio delivery bookkeeping; never hash pointers, padding or Debug formatting. Compare authoritative state and simulation events directly in determinism tests as well as checksums. Physical bindings stay local; gameplay-affecting control-scheme rules must be normalized or synchronized when they reach real fights.

### 1. Complete `FightSnapshot` and Pre-Step 9-Slot Ring Buffer (`ring_buffer[(step_f as usize) % 9]`)
- `Fight::save_netplay_snapshot(&self, net_frame: u32) -> FightSnapshot` and `Fight::restore_netplay_snapshot(&mut self, snap: &FightSnapshot)` capture `PowerOnSnapshot` **plus every mutable wrapper field on `Fight`** (`over`, `last_flow`, `black`, `sounds`, `country`, `reader`, `read`, `combo`, `callouts`, `moments`, `era_hud`, `pending_callouts`, and `pending_combos`). Capturing `over` and the pending HUD/audio queues ensures rolling back across a speculative round-ending KO or round transition restores `Fight` completely.
- Immediately **before** stepping frame `step_f` (including `step_f = 0`), `RollbackEngine` stores the pre-step `FightSnapshot` in **`ring_buffer[(step_f as usize) % 9]`**. When a late `NetPacket::Input` reveals a misprediction at earliest frame `k_dirty` (`confirmed_frame < k_dirty <= current_frame`, with `confirmed_frame: i64` initialized to `-1`), `RollbackEngine` restores `ring_buffer[(k_dirty as usize) % 9]` and re-steps `step_f` in `k_dirty..=current_frame`, re-saving each pre-step snapshot before stepping.
- Each ring-buffer slot records `emitted_latch: u8` and `emitted_announcements: Vec<AnnouncerLine>`. During resimulation (`step_f <= highest_audio_frame`), any latch or announcement matching what that frame already emitted is suppressed, while any newly discovered non-idle latch (`!= 0xff && != slot.emitted_latch`) is queued into `fight.sounds` so corrected hits play their audio once without machine-gunning.

### 2. Redundant-Window `NetPacket::Input` and Three Unified Room Modes (`RoomKind`)
- All UDP packets carry a 6-byte header (`b"SF1N\x01"` + 1-byte variant tag). Each `NetPacket::Input` (`50` bytes total) carries up to `9` consecutive `u16` input masks (`start_frame .. start_frame + count`), healing up to 8 consecutive dropped UDP packets automatically without retransmission, alongside `(checksum_frame, checksum)` for per-frame 64-bit FNV-1a desync detection.
- The `Hello`/`Welcome` handshake synchronizes `config_hash` (preset, 20 core toggles, DIPs, roster/style/bonus settings, and Dramatic Battle rules), `bank_hash`, `art_pack_hash`, `prng_seed`, `StagePick`, and `RosterFighter`/`StylePick`, while keeping Costume, CRT Look, Gauge Art, Jukebox track, volumes, bindings, and overlays strictly local to each peer.
- The same `RollbackEngine` and wire protocol power all three online modes:
  - **`RoomKind::Versus`:** Host = `P1_STRUCT`, Client = `P2_STRUCT`.
  - **`RoomKind::Dramatic`:** Host = `Ally 1` (`P1_STRUCT`), Client = `Ally 2` (`P2_STRUCT` + `0x1000..0x15ff`), both deterministically stepping the shared CPU `Boss` (`CPU_STRUCT` `0xffc664`).
  - **`RoomKind::Spectate`:** Host retains the current match's `confirmed_inputs: Vec<(u16, u16)>` from frame `0` and streams `16`-frame `SpectatorChunk` packets from each spectator's `next_expected_frame`, allowing spectators joining pre-match or mid-match to catch up with a 15-frame jitter buffer and zero impact on the combatants.

### 3. Non-Blocking Online Pause (`PauseBox::open_online()`)
- During an online match, opening `PauseBox` or `MovesPanel` renders the overlay locally while `GameSession::step` continues stepping `RollbackEngine` and `Fight` at 60 Hz with neutral (`0`) local inputs, and closing either overlay leaves `play.held = false` so the simulation never stalls.

### 4. Socket-Free `sf-engine` Command/Event Queues, `VirtualNetBus`, and `sf-host` UDP Driver (`ADR-0005`)
- `sf-engine` remains 100% socket-free. `GameSession` receives network activity via `push_net_events(&mut self, events: &[NetTransportEvent])` and emits outbound actions via `drain_net_commands(&mut self) -> Vec<NetTransportCommand>`.
- An in-memory **`VirtualNetBus`** in `sf-engine` routes packets between two or more `GameSession` / `RollbackEngine` instances in headless tests under scripted latency, jitter, packet drop, duplication, and reordering.
- `crates/sf-host/src/net.rs` owns the non-blocking `std::net::UdpSocket`, LAN broadcast discovery on UDP port `7001` (`255.255.255.255:7001`), optional HTTP room directory queries, and OS network interface link detection (`Link::Ethernet` vs. `Link::Wifi`).

## Consequences

- **Positive:** Every rollback path—including 8-frame mispredictions across round-ending KOs, `SF2`–`SF6` Era style cancels, 2v1 Co-Op Dramatic Boss AI, and mid-match spectator catch-up—is deterministically verifiable in fast, socket-free `cargo test` runs via `VirtualNetBus`.
- **Positive:** Players can connect with zero external servers via Direct P2P (`IP:PORT` or `SF1-XXXX-XXXX` room codes) or browse LAN and community rooms with live `PING`, `RBF`, `JITTER`, `LINK` (`[ETH]` / `[WIFI]`), and the **Wired-Only Auto-Filter**.
- **Trade-off:** Nine complete pre-step snapshots add copying and memory costs, potentially including the wrapper's round-start machine snapshot. Measure size and save/restore timing in the completed implementation; no measurements are available yet.
