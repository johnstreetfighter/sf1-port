# Matchmaking, ranked play and CPU opponents in the queue: design

- **Date:** 2026-11-17
- **Size:** Project (a new server program with accounts and a database, a queue and rating rules, new front-end screens, CPU opponents in matched play, an operator report, and deployment beside the relay).
- **Marking:** "(user)" is the maintainer's decision in brainstorming. "My call" is the designer's choice, which the maintainer has not been asked about one by one.

## Intent

A player picks Ranked or Unranked, presses one button, and gets a fight. The community is small, so an empty queue would kill the mode. When no person is waiting, the queue offers a labelled CPU opponent, in Ranked too, so a match is always available (user).

Matchmaking is an addition. Rooms, room codes, LAN play, direct joins and community relays keep working with no account, and never depend on the matchmaking service (user).

This reverses two earlier non-goals: the rollback design ruled out "ELO/ranked leaderboards, or account sign-ins", and the cross-platform design left skill-based matchmaking out of its first release. It needs a new ADR. The default-relay design's non-goal, "matchmaking on the relay", still holds: the relay is not changed.

## Goals

1. **Ranked and Unranked queues** that pair two players and start a fight with no host, no code and no room setup (user).
2. **A CPU opponent whenever no person is found**, in both queues, always shown as a CPU (user).
3. **One rank per player** (user), shown as a standard tier (user) and a rating number.
4. **A CPU ceiling:** CPU wins stop raising a player's rank at the top of Gold; higher ranks are earned against people only (user chose the ceiling; its position is my call).
5. **A private rating for every CPU fighter at every difficulty level**, reviewed from time to time to improve the Fair CPU (user).
6. **A simple account for matchmaking only:** name and password, no email, with a recovery code (user).

## Non-goals

- CPU entries on the player ladder (user).
- Seasons or ladder resets.
- Email, password reset by email, or any personal details on the server.
- Stopping players from making fresh accounts.
- Servers outside US East, or region selection.
- Spectating matched fights.
- A preset choice in either queue. Other presets are played in rooms.
- Bonus challengers, Dramatic Battle or era-style choices beyond what Definitive Overhaul already allows in an online Versus room today.
- The server running fights or holding a ROM.
- Automatic replay verification on the server.
- More Fair CPU difficulty levels than the four that exist.

## Anti-goals

- A CPU passed off as a person.
- A rank that can be faked to the top.
- Matchmaking getting in the way of account-free rooms, or a matchmaking outage breaking them.
- Personal details, IP addresses or passwords in logs or in the database in readable form.

## Constraints

- **The engine stays socket-free** (ADR-0005, ADR-0018). Matchmaking traffic crosses the engine/host boundary as commands and events, as `NetTransportCommand` and `NetTransportEvent` do in `crates/sf-engine/src/netplay/transport.rs`. Screens are pure and testable without a network.
- **The relay is not changed** (ADR-0024). It keeps reading no payloads and storing nothing.
- **One server.** The service runs on the existing OVHcloud `VPS-1` (2 vCores, 4 GB RAM) beside `sf-relay`, at no extra monthly cost, and must be as hands-off as the relay (see `docs/design/2026-11-16-default-relay-vps-design.md`).
- **The room handshake already refuses mismatched games.** `HandshakePolicy::welcome` in `crates/sf-engine/src/netplay/protocol.rs` refuses on `protocol_hash`, `bank_hash` and `art_pack_hash`. It does not compare `config_hash`: the guest takes the host's match rules from `Welcome`. Two players the queue pairs must pass it.
- **A recording loads only on the build that made it.** `replay_compatibility_hash()` in `crates/sf-engine/src/version.rs` is built from the version and commit.
- **Playing back a Fair CPU fight replays the CPU's recorded pad.** It does not run the CPU again, so watching a recording does not show whether the CPU played at its true strength.
- **Both games already notice a broken fight.** The rollback engine raises `RollbackEvent::PeerDisconnected` after 300 stalled ticks.
- **The Fair CPU has four levels** (`fair_cpu::Level`: Easy, Normal, Hard, VeryHard), so there are 12 × 4 = 48 CPU entries.
- **Fights are recorded as inputs** (`.sf1replay`, Match replay in `CONTEXT.md`), which is what makes a CPU result checkable later.
- **The sign-up screen must work from a pad** on a Steam Deck in Gaming Mode.

## Approaches

1. **A separate matchmaking service beside the relay; CPU fights run on the player's machine. Chosen (user).** A new server program holds accounts, ratings, the queue and CPU records. When it pairs two people it has them meet in an Internet room on the relay, so the fight runs exactly as a room fight does today. A CPU fight runs locally with the Fair CPU and has no network delay. Trade-off: a CPU result is taken on trust. The ceiling limits what a faked one can gain, and a suspicious recording is checked on the maintainer's PC.
2. **Matchmaking built into the relay.** One program and one deploy. Rejected: the relay would stop being a program that reads and stores nothing, every community relay would carry account code, and a matchmaking fault could take rooms down.
3. **A separate service, with the server playing the CPU over the network.** No CPU result could be faked. Rejected: CPU fights would get online delay, each would cost server processing for its whole length, and the server would need a ROM.

A rented third-party game backend was dropped early: monthly cost, an outside company holding accounts, and less control.

## What was decided

### 1. Accounts

- A matchmaking account is a **name and a password, typed in the game, with no email** (user). Both queues need one (my reading of "an account system for matchmaking"; the user was told and did not object).
- **A recovery code is shown once at sign-up** (user). Entering the name and the code lets the player set a new password. *My call:* using it issues a fresh code and retires the old one.
- The game remembers the sign-in on that machine, so the password is typed once per machine. *My call:* it stores a session token, never the password, in the player's settings folder.
- *My call:* names are unique without regard to case and fit the existing twelve-character player tag. The account name is the name opponents see in matched play.
- *My call:* passwords are stored only as a salted slow hash (Argon2id); recovery codes likewise. Sign-in, sign-up and recovery attempts are throttled per address in memory, as the relay throttles joins, and addresses are never written to disk or logs.
- *My call:* a player can delete their account from the game. The operator can rename or ban an account from the command line, for offensive names and confirmed cheating.

### 2. The two queues

- **Both queues play Definitive Overhaul**, with no preset choice (user). Fighter and stage are picked after the pairing, on the existing select screens (user).
- **Pairing** prefers a close rating and a good connection, and widens both the longer a player waits (user accepted). *My call:* each client measures its round trip to the relay, and the service estimates a pair's connection as the sum of the two, since matched traffic goes through the relay.
- **Only compatible games are paired.** *My call:* the client sends the three values the room handshake refuses on (`protocol_hash`, `bank_hash`, `art_pack_hash`), and the service pairs only equal ones. It does not send its local `config_hash`, or players with different local settings would never meet. A client the service is too old for is told to update.
- **Matched fights use rules the service names, not either player's local settings.** *My call:* the hosting client builds the room from a fixed Definitive Overhaul configuration (round time, toggles, Modern penalty and roster options included), since the handshake would otherwise hand the guest whatever the host had set locally.
- **Unranked shows no rank** and pairs by a separate hidden rating (user accepted).
- **Ranked follows Street Fighter 6's format** (user): each game is best of three rounds; rating moves after every game; a rematch is offered after each game and needs both players; the pairing ends when one player has two wins; then both return to the queue. This is from the designer's memory and forum posts, not an official Capcom page; see Sources.
- **Unranked allows open-ended rematch** (user accepted).

### 3. CPU opponents in the queue

- The queue looks for a person first. *My call:* after about 30 seconds it offers a CPU. The player may accept or keep waiting (user accepted).
- **The CPU is always labelled as a CPU** on the offer, the VS screen and the result (user).
- **The queue picks the CPU entry whose rating is nearest the player's**, whatever its level number (user accepted). *My call:* it chooses at random among the nearest few so a player does not meet the same entry every time. The CPU's fighter is fixed before the player picks theirs.
- **A CPU fight runs on the player's machine** with the Fair CPU at the entry's level (approach 1). *My call:* it uses the match rules the service names, not the player's local difficulty or preset, and behaves as an online fight does: pausing does not stop it and there is no restart.
- **Same format as a human pairing**, with the CPU always accepting the rematch (user accepted).
- **A CPU fight is never interrupted** by a person arriving; the player meets a person on their next search (user accepted).
- The game reports each CPU game's result with its recorded inputs.

### 4. Rank and rating

- **One rank per player** (user).
- **Tiers are Bronze, Silver, Gold, Platinum, Diamond and Master** (user asked for the standard scheme; the exact six, and a few steps inside each, are my call).
- **The CPU ceiling is the top of Gold** (my call, on the user's decision to have a ceiling). A CPU win never raises a player's rating past it. A CPU loss costs rating as any loss does.
- *My call:* a player above the ceiling who accepts a CPU in Ranked plays a game marked "no rank change", win or lose. Otherwise nobody above Gold would ever accept one.
- **Five placement games** before a rank shows; the rating moves faster during them (user accepted). *My call:* CPU games count as placement games, and the ceiling applies.
- **After a ranked game** the player sees their rating, how far it moved, their tier and step, the distance to the next step, and the opponent's rating (user accepted).
- **The ladder screen** is drawn in the style of the 1987 ranking table and shows the top players and the player's own position with the names around it. A player with no ranked game in 30 days is hidden from it and keeps their rating (user accepted).
- **No seasons, no resets** (user accepted).
- *My call:* ratings use Glicko-2, which carries an uncertainty per player. That gives fast placement and sensible handling of a player returning after months, without special rules.

### 5. Results between two people

- **Both games report the result; if they agree, ratings move** (user).
- **A player whose connection drops takes the loss, and the other takes the win**, rage quit or not (user). A player who drops often waits before queueing again (user). *My call:* the wait starts at a few minutes and grows with repeats.
- **If the two reports disagree, no rating changes**, and the recorded fight is kept for the maintainer (user).
- *My call, and a known gap:* each client keeps its connection to the matchmaking service open through the pairing, and the service treats the side whose connection goes silent as the one who dropped. If both stay connected to the service but the fight between them breaks, the service cannot tell who caused it: no rating changes, and the break counts toward both accounts' drop history. The brainstorm described the server "seeing whose connection dropped" through the relay; under approach 1 the relay tells the service nothing, so this is the nearest equivalent.

### 6. CPU entries and the review

- **Each of the 48 fighter-and-level pairs is its own entry with its own rating** (user). They are private to the maintainer (user).
- **Ranked games move a CPU entry's rating. Unranked games do not, but are counted in its records** (user accepted).
- **Each entry carries the CPU's version.** When the Fair CPU's play changes, the new version starts a fresh rating and the old one is kept for comparison (user accepted).
- **The CPU version is a number bumped by hand** (user, on the designer's recommendation), so ratings last across releases that leave the CPU alone. A test guards it: it records the CPU's decisions on a set of fixed fights and fails when they change without a bump. The fixed fights run under Definitive Overhaul, so a balance or move-table change that alters the CPU's play trips the test too. A release was the alternative and was turned down, because restarting all 48 ratings every release might never let them settle.
- **A recording checker is part of this project** (user, on the designer's recommendation). It is a tool run on the maintainer's PC, never on the server. It re-runs the Fair CPU at the entry's level against the player's recorded inputs and reports the first frame where the recorded CPU pad differs, or that none does. Each stored recording carries the build it was made on, and the checker says which build is needed when it cannot load one.
- **Each entry keeps its record against each opposing fighter and a sample of its recorded games**, so a weak entry can be watched (user accepted).
- *My call:* a new entry starts at a rating seeded from its level, with high uncertainty.
- *My call:* every game is stored as a row, and ratings can be recomputed from the rows with chosen accounts left out. That is the remedy when faked results have dragged an entry's rating.
- **The maintainer checks the entries periodically** (user). *My call:* an operator command prints each entry's rating, games played and record by opposing fighter, lists the weakest and strongest, and exports recordings for chosen entries. How often to run it is the maintainer's choice.

### 7. The service

- A new server program in its own crate, separate from `sf-relay` (approach 1).
- *My call:* it speaks QUIC through `quinn` with a WebPKI certificate, as the relay does, on its own port and hostname under `almamoto.com`; it keeps one SQLite file, included in the server's daily backup; it runs as its own systemd unit with its own resource limits so it cannot starve the relay.
- *My call:* its address is a setting with a shipped default, like the relay's, so a community could run its own ladder. A community service keeps its own accounts and ratings.
- *My call:* it logs counts only (accounts online, queue sizes, games per hour), never names, addresses or codes.
- **A pairing of two people is carried out as an Internet room on the default relay.** *My call:* one client is told to host and report the code to the service, which passes it to the other; the relay sees an ordinary room.
- **If the service is down**, the matchmaking screens say so and every other online feature works as before.

## Trust, stated plainly

- A human-versus-human result needs both games to agree, so one tampered game cannot award itself a win.
- Two players can still agree to trade wins. The two-win pairing cap limits it; nothing prevents it.
- A CPU result can be faked by a tampered game. The ceiling limits the gain to Gold. The recording is the evidence: the recording checker proves whether the CPU in it played as the real one would. It runs when the maintainer chooses, so a fake stands until it is checked.
- Fresh accounts are free, so a strong player can restart at the bottom. Accepted at this community's size.

## What's left to the builder

- Crate and binary names, the wire format, and the database schema.
- The exact commands and events at the engine/host boundary, and the host-side client.
- The front-end screens and their layout: sign-up, sign-in, recovery, the queue, the CPU offer, the ranked result, the ladder. They follow the existing front-end conventions and should be mocked up for the maintainer before they are built.
- Where the entry sits under Multi Player → Online.
- Glicko-2 settings, tier boundaries, the number of steps per tier, and the seeded CPU ratings.
- Queue widening rates, the exact CPU-offer wait, and the drop-wait schedule.
- How many recordings are kept per CPU entry and for how long.
- Stage choice in a CPU pairing.
- Password length rules, token lifetime, and the recovery code's form.
- The setup script, systemd unit, firewall rule, certificate and health check for the service, following the relay's.
- Checking the Street Fighter 6 format against an official source if one can be found.

## Sources

Street Fighter 6's ranked format could not be confirmed from an official page. These forum threads agree with the format above (a best-of-three pairing with rematch inside it and none after):

- [Ranked Question (Steam discussions)](https://steamcommunity.com/app/1364780/discussions/0/4340986888820219180/)
- [Rematch options (Steam discussions)](https://steamcommunity.com/app/1364780/discussions/0/3809531491906820610/)
- [Rejecting a rematch (Steam discussions)](https://steamcommunity.com/app/1364780/discussions/0/3809533595059093945/)
