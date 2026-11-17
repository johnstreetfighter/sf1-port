# Running a relay

Internet rooms go through a relay: both players connect out to it over QUIC
on UDP, and it forwards their packets inside one room. Anyone can run a relay
for their own community. Players select it under **Settings → Netplay →
RELAY** in the official game; no modified client is needed.

A room code belongs to the relay that issued it. The host and every guest
must select the same relay. The game looks a code up only on the selected
relay and never falls back to another one. A community relay keeps working
when the project's default relay is down.

This guide deploys the relay on a Linux server with systemd. The same binary
and steps serve the project's default relay.

## What you need

- A Linux x86_64 or arm64 server with a public IPv4 address.
- A DNS name you control, for example `relay.example.net`.
- Inbound **UDP 7443** open on the server and any provider firewall. The
  relay uses no TCP port. The certificate steps below also need inbound
  TCP 80 while certbot issues the certificate and at every renewal.
- A certificate for that name from an authority the public web trusts, such
  as Let's Encrypt. The game checks the certificate against the public web
  authorities and the name the player typed. A self-signed certificate, an
  expired one, or one for another name is refused.

## Build from a tagged release

Build from the release tag your players run, so the relay and the game speak
the same relay protocol. The relay first ships in the release after v1.5.9.

```sh
git clone https://github.com/johnstreetfighter/sf1-port.git
cd sf1-port
git checkout <tag>
cargo build --release --locked -p sf-relay --features net
sudo install -m 0755 target/release/sf-relay /usr/local/bin/sf-relay
```

`--locked` builds with the dependency versions recorded in the tag's
`Cargo.lock`. The build needs a stable Rust toolchain and a C compiler. The
repository pins no toolchain; the relay was developed with Rust 1.96. It
needs no ROM, art or game data.

## DNS

Create an `A` record for the relay's name that points at the server's public
IPv4 address. Wait until it resolves before asking for a certificate:

```sh
getent hosts relay.example.net
```

## Certificate

Create the service account and install the deploy hook, then get a
certificate with certbot. The example uses certbot's standalone mode on
TCP 80.

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin sf-relay
sudo install -m 0755 tools/relay/renew-hook.sh /usr/local/sbin/sf-relay-renew-hook
sudo certbot certonly --standalone -d relay.example.net --deploy-hook /usr/local/sbin/sf-relay-renew-hook
```

`--deploy-hook` runs the hook,
[`tools/relay/renew-hook.sh`](../tools/relay/renew-hook.sh), after this first
issue, and certbot saves it for every later renewal of this certificate only.
The hook copies the certificate chain to `/etc/sf-relay`, converts the key to
PKCS #8 (the only key format the relay reads), and restarts the service if it
is running. Check that both files arrived:

```sh
sudo ls -l /etc/sf-relay
```

If `fullchain.pem` or `privkey.pem` is missing, or you already had a
certificate before installing the hook, run the hook once by hand:

```sh
sudo RENEWED_LINEAGE=/etc/letsencrypt/live/relay.example.net /usr/local/sbin/sf-relay-renew-hook
```

Certbot's own timer renews the certificate. Standalone renewal needs TCP 80
reachable each time. The relay reads its certificate only at start, so the
hook restarts it, and a restart closes the rooms open at that moment. Players
can open a new room straight away. Check that the authority can still reach
the server:

```sh
sudo certbot renew --dry-run
```

A dry run does not run deploy hooks. The by-hand command above is the test of
the hook itself.

## Service

Install the example unit,
[`tools/relay/sf-relay.service`](../tools/relay/sf-relay.service), and start
it:

```sh
sudo install -m 0644 tools/relay/sf-relay.service /etc/systemd/system/sf-relay.service
sudo systemctl daemon-reload
sudo systemctl enable --now sf-relay.service
```

The unit runs:

```sh
sf-relay --listen 0.0.0.0:7443 --cert /etc/sf-relay/fullchain.pem --key /etc/sf-relay/privkey.pem
```

To use another port, change `--listen` and tell players to enter
`relay.example.net:PORT`. Open the firewall for the port you chose, for
example with ufw:

```sh
sudo ufw allow 7443/udp
sudo ufw allow 80/tcp
```

The second rule is for certbot's standalone issue and renewal.

## Health checks

Check the relay from any other machine that has the binary:

```sh
sf-relay --check relay.example.net
```

The check connects the way the game does. It exits 0 only if the name
resolves, UDP reaches the relay, the certificate is trusted for that name,
and the relay speaks the relay protocol. It opens no room and does not count
against the join limits. Run it from outside the server, so it also tests DNS
and the firewall, and run it on a schedule if you want an alert when the
relay is down.

On the server, `systemctl status sf-relay` shows whether the process is
running, and the journal carries one line a minute:

```sh
journalctl -u sf-relay -f
```

```text
rooms 3 connections 7
```

The relay logs its listening address and these counts. It never logs room
codes, player addresses or gameplay.

## Limits

The relay bounds its own state. These limits are built in:

| Limit | Value |
| --- | --- |
| Open rooms | 4,096 |
| Seats in a room | the host and 15 guests |
| Idle room | closes after 15 minutes without traffic |
| Room age | closes after 12 hours |
| Rooms created | 10 a minute per address |
| Joins, including failed ones | 30 a minute per address |
| Traffic per connection | 600 datagrams and 256 KiB a second |
| Addresses tracked for throttling | 16,384 |

An address is one IPv4 address or one IPv6 /64. The example unit adds
operating-system ceilings: 512 MB of memory, 64 tasks, one CPU and 4,096 open
files. These ceilings are starting points that have not been measured under
load. Watch `systemctl status sf-relay` for memory and raise them only if
your own traffic needs it.

## Rollback

Keep the previous binary when you upgrade:

```sh
sudo cp /usr/local/bin/sf-relay /usr/local/bin/sf-relay.previous
sudo install -m 0755 target/release/sf-relay /usr/local/bin/sf-relay
sudo systemctl restart sf-relay.service
sf-relay --check relay.example.net
```

If the check fails or players report trouble, put the old binary back:

```sh
sudo install -m 0755 /usr/local/bin/sf-relay.previous /usr/local/bin/sf-relay
sudo systemctl restart sf-relay.service
```

Each restart closes the open rooms. The relay keeps nothing on disk, so there
is no data to migrate or restore.

## Telling your players

Give players the relay's name, and the port if it is not 7443. Each player:

1. Opens **Settings → Netplay** and moves to **RELAY**.
2. Presses confirm, types the name (`RELAY.EXAMPLE.NET` or
   `RELAY.EXAMPLE.NET:9000`) and confirms again.
3. Hosts or joins by code as usual under **Multi Player → Online**.

To go back to the project's relay, edit **RELAY**, delete the text and
confirm the empty entry, or use **RESET TAB**. When a code is not found, the
message names the relay the game asked, so players can see which one they
have selected.
