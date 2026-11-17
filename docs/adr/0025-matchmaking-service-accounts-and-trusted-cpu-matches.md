---
status: accepted
---

# Matchmaking is a separate service with accounts, and CPU matches are trusted up to a ceiling

On 2026-11-17 the maintainer asked for Ranked and Unranked matchmaking in which a CPU opponent fills in when no person is waiting, Ranked included, so a small community always has a match ([design](../design/2026-11-17-matchmaking-design.md)). This reverses two earlier positions. The rollback design ([ADR-0018](0018-rollback-netplay-and-virtual-net-bus-on-the-1987-engine.md)'s source document) ruled out ranked leaderboards and account sign-ins, and the cross-platform design left skill-based matchmaking out. Both still describe rooms: Internet rooms, LAN play, direct joins and community relays need no account and never depend on matchmaking.

Matchmaking lives in its own server program beside the relay, with its own database of accounts, ratings and match rows. We chose that over building it into `sf-relay` because the relay's promise in [ADR-0024](0024-internet-rooms-through-a-quic-relay.md) is that it reads no payloads and stores nothing, every community relay would otherwise carry account code, and a matchmaking fault could take rooms down. Two matched people meet in an ordinary Internet room on the relay, so the fight, rollback and handshake are unchanged.

A matched fight against a CPU runs on the player's machine with the Fair CPU, and the game reports the result with its recorded inputs. We chose that over having the server play the CPU, which would have made CPU results unfakeable but added online delay to every CPU fight, cost server processing for each fight's whole length, and put a ROM on a public server. The cost is that a tampered game can fake a CPU result. Three things bound it: a win over a CPU never raises a player past the top of Gold, so the upper ranks are earned against people only; every match is stored as a row so ratings can be recomputed without a cheater's matches; and a recording checker on the maintainer's PC re-runs the Fair CPU against the recorded inputs to prove whether a recording is genuine. Nothing is verified on the server.

An account is a name and a password with no email, plus a recovery code shown once. The server therefore holds no personal details, and a player who loses both password and code loses the account. Fresh accounts are free, so a strong player can start again at the bottom; that is accepted at this community's size.

Each tournament fighter at each Fair CPU difficulty level is a CPU entry with its own private rating, kept per CPU version. The version is a number bumped by hand and guarded by a test, not the build, so ratings survive releases that leave the CPU's play alone.
