# Netplay

Open **Multi Player → Online** to browse rooms, host a room, or enter a room
code or IPv4 address. A LAN address code (`SF1-…`) contains the host's IPv4
address and UDP port; it is an address, rather than an account or invitation
stored on a server. [Internet rooms](#internet-rooms) use a shorter code held
by a relay.
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

## Internet rooms

**Host Match** offers **VISIBILITY → INTERNET CODE** first when a relay is
selected. Opening the room connects to the relay, and the room code appears
only once the relay has the room ready. An Internet room code has six letters
and digits, such as `K7Q-MPX`. It never uses 0, O, 1, I or L, so it cannot be
mistaken for a LAN address code. Guests enter or paste it in **Direct Join**
in any case, with or without the dash.

Both players connect out to the relay over QUIC on UDP, so neither needs port
forwarding. The game checks the relay's certificate against the public web
authorities and refuses a relay it cannot verify. The relay forwards the same
rollback packets between the members of one room only. The host keeps the
same room and code through fighter select, the match, Rematch and Reselect.
Leaving closes the room and expires its code.

### Choosing a relay

**Settings → Netplay → RELAY** shows the selected relay. A fresh install uses
the project's default relay. No default relay runs yet (#130), so the row
shows `NONE` and Internet rooms stay hidden until you set a relay.

To use a community relay, confirm on **RELAY**, type its host name with an
optional port (`RELAY.EXAMPLE.NET` or `RELAY.EXAMPLE.NET:9000`) and confirm.
The port defaults to 7443. To restore the default, save an empty entry or use
**RESET TAB**. The setting is saved as `relay = "relay.example.net"` in
`settings.toml`.

A code belongs to the relay that issued it, so both players must select the
same relay. The game looks a code up only on the selected relay and never
tries another one. When a lookup or connection fails, the message names the
relay that was asked. A change of relay applies to the next room.

The relay limits each room to the host and fifteen guests. It closes rooms
after 15 idle minutes or 12 hours, throttles room creation and joins per
address (failed joins count), and budgets each connection's traffic. It logs
room and connection counts, never codes or gameplay. To run your own, follow
[Running a relay](relay-deployment.md).

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
