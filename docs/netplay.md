# Netplay

Open **Multi Player → Online** to browse rooms, host a room, or enter a room
code or IPv4 address. A room code contains the host's IPv4 address and UDP port;
it is an address, rather than an account or invitation stored on a server.
The default game port is **7000**. Codes for other ports include a final
four-digit hexadecimal port word.

On Windows, press **Ctrl+C** to copy the code shown in **Host Match** or the
selected code in **Browse Rooms**. In **Direct Join**, press **Ctrl+V** to fill
the entry from the clipboard, then confirm **Join** when ready.

Windows adapter detection uses its built-in networking cmdlets. On Linux,
adapter detection requires **iproute2** (the `ip` command). Hosting selects a
local adapter, including on a LAN with no default route; joining selects the
route to the peer. Both queries run away from the game thread.

Online supports 1v1 Versus and co-op Dramatic Battle. Up to four spectators
can watch either mode.

In Versus rooms with Style Cards enabled, each player locks their own era,
revision, and loadout before the stage is picked. Both players use the same
pair of locked styles for the match.

For co-op, open **Host Match**, set **Room Kind** to **Dramatic 2v1**, and open
the room. The guest can select the room in Browse or choose **Dramatic Ally 2**
under **Join As** in Direct Join. The host chooses a rivalry or sets up a custom
team, then sets the lifebar and friendly fire rules. With Era Styles enabled,
the host chooses one era for a rivalry or chooses the Ally 1 and boss styles
in Custom 2v1. The guest confirms the preset ally and its style or picks Ally 2
and its style for a custom team. Both peers run the same CPU boss.

Shared Lifebar gives the allies one Team Era Gauge. Mixed-era allies share
three unified stocks. Independent Lifebar gives each ally their own gauge,
with Ally 2's compact gauge above Ally 1's. The boss keeps a separate gauge.

Choose **JOIN AS → SPECTATOR** in Browse or Direct Join to watch a room before
the match or join by a private room code. **PLAYER** requests the challenger
slot. Selecting a room already in a match enters as a spectator automatically.

Public rooms answer LAN discovery on UDP **7001**. Private rooms are joined
through their code or address. The host reports the actual routed adapter as
Ethernet or Wi-Fi; **WIRED ONLY** rooms reject Wi-Fi peers. Peers must use the
same engine protocol and effective game data and artwork. The host supplies
the match rules and seed. Display, sound, bindings and Jukebox choices remain
local.

Use the **Player 1 controls on each computer**: the host controls the match's
Player 1 fighter and the guest controls its Player 2 fighter. In Dramatic Battle
these controls drive Ally 1 and Ally 2 respectively. A local pause,
Moves list or disconnected-controller notice keeps that player's inputs neutral
while the match continues. After a match, both players vote for Rematch or
Reselect; Menu leaves the room. A spectator can inspect the local menus or
leave observation, while the combatants control match decisions.

**Settings → Netplay** holds the player tag, input delay, maximum rollback,
wired preference, HUD mode and game port. AUTO input delay uses the measured
round-trip time: one frame per 34 milliseconds, capped at four frames.

## Optional community directory

With no `lobby_url` in `settings.toml`, browsing uses LAN discovery. Setting
`lobby_url` to an HTTP(S) endpoint enables the optional directory worker.
Requests run away from the game thread. Directory failures are written to
the game log, while direct connections and LAN discovery remain available.

The endpoint uses this version 1 contract:

- `POST` publishes a public room as JSON with `version: 1`, its numeric UDP
  `port`, and a `beacon` string containing 112 hexadecimal digits. The beacon
  encodes the same 56-byte room announcement used for LAN discovery. The
  server associates it with the host's reachable IPv4 address and expires
  entries when publications stop.
- `POST` with `version: 1`, `port`, and `beacon: null` withdraws that room.
- `GET` returns `{"version":1,"rooms":[{"address":"127.0.0.1:7000","beacon":"…"}]}`.
  Each address's port must match the corresponding beacon. A successful POST
  can return an empty body.

The worker accepts at most 128 rows and 64 KiB per response, disables HTTP
redirects, and limits each request's duration. It publishes public rooms only.
Use an address reachable by the connecting players for a directory entry;
local room codes continue to describe the host's local interface.
