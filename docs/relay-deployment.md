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
and steps serve the project's default relay, `relay.almamoto.com`, which was
set up with the script below. On Ubuntu 24.04 one script does
the whole setup; see [Set up Ubuntu 24.04 with one command](#set-up-ubuntu-2404-with-one-command).
The sections after it describe each step, for other distributions and for
understanding what the script did.

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

## Set up Ubuntu 24.04 with one command

Get a relay binary first: build it on the server ([Build from a tagged
release](#build-from-a-tagged-release)) or download the one GitHub Actions
built ([Download the CI build](#download-the-ci-build)). Create the DNS record
([DNS](#dns)) and wait until it resolves. Then, from a checkout of the
repository on the server:

```sh
sudo tools/relay/setup-ubuntu.sh relay.example.net --binary ./sf-relay-linux-x86_64
```

Add `--email you@example.net` to give the Let's Encrypt account a contact
address. Without it the script registers with no email address. Let's Encrypt
no longer emails expiry notices either way, so rely on the health check. Running the script
accepts the Let's Encrypt subscriber agreement on your behalf.

The script,
[`tools/relay/setup-ubuntu.sh`](../tools/relay/setup-ubuntu.sh), does this:

- Creates the unprivileged `sf-relay` account.
- Turns on the `ufw` firewall: incoming traffic is refused except SSH (every
  port `sshd` listens on), TCP 80 and UDP 7443.
- Installs the distribution's `certbot` package, the deploy hook and the
  certificate, checks that the hook delivered the certificate to
  `/etc/sf-relay`, and makes a renewal dry run.
- Moves certbot's timer to 09:30 UTC with up to 30 minutes of random delay,
  which is before dawn in US East. A renewal restarts the relay and closes
  the rooms open at that moment.
- Turns on unattended security updates, with automatic reboots off. Reboot
  the server yourself at a quiet hour when a kernel update asks for it.
- Caps the systemd journal at 100 MB.
- Installs the binary and the service unit and starts the relay.
- Turns off SSH root login and password login, but only when the user who ran
  `sudo` has a key in `~/.ssh/authorized_keys`. Otherwise it leaves SSH alone
  and says why. Keep your session open and confirm a new login works before
  you close it.

You can run it again at any time. It changes only what differs, and it
restarts the relay only when the binary or the unit changed. To upgrade, run
it again with the new `--binary`; it keeps the old one as
`/usr/local/bin/sf-relay.previous` ([Rollback](#rollback)).

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

## Download the CI build

The **Relay** workflow in GitHub Actions builds the same locked binary for
Linux x86_64 on Ubuntu 24.04, on every release tag and on a manual run. The
binary is a workflow artifact, not a release download, so you need a GitHub
account to fetch it, and GitHub deletes artifacts after 90 days. Open the run
for your tag under **Actions → Relay**, download the
`sf-relay-linux-x86_64-<commit>-<attempt>` artifact, and check it before use:

```sh
unzip sf-relay-linux-x86_64-*.zip
sha256sum -c sf-relay-linux-x86_64.sha256
```

With the GitHub CLI, `gh run download <run-id> --repo
johnstreetfighter/sf1-port` fetches the same files. The server then needs no
Rust toolchain. Building from source stays available as a fallback.

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

Add `--min-days` to also fail when the certificate expires soon, which is how
a stuck renewal shows up while there is still time to fix it:

```sh
sf-relay --check --min-days 14 relay.example.net
```

The project's **Relay** workflow runs this check against the default relay,
`relay.almamoto.com`, every six hours, trying three times ten seconds apart
before it fails. GitHub emails a failed scheduled run to the person who last
changed the workflow's schedule. GitHub also pauses scheduled workflows after
60 days without activity in the repository and sends a warning first; push a
commit or re-enable the workflow under **Actions** to keep the checks
running. A community relay can copy the workflow and change the host name.

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

The setup script keeps the previous binary as
`/usr/local/bin/sf-relay.previous` whenever `--binary` replaces it. By hand,
keep it yourself when you upgrade:

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

## A temporary second relay on the same server

One server can run a second relay beside the first, for example to test the
community-relay setting against the default relay. It needs its own host
name, certificate, UDP port and service. This example uses
`relay2.example.net` on UDP 17443 and the name `sf-relay2`.

Create an `A` record for `relay2.example.net` that points at the same server,
and wait until it resolves. Then:

```sh
sudo ufw allow 17443/udp
sudo certbot certonly --standalone --non-interactive --agree-tos --register-unsafely-without-email \
    --cert-name relay2.example.net -d relay2.example.net \
    --deploy-hook "env SF_RELAY_NAME=sf-relay2 /usr/local/sbin/sf-relay-renew-hook"
sudo ls -l /etc/sf-relay2
sed -e 's|0.0.0.0:7443|0.0.0.0:17443|' -e 's|/etc/sf-relay/|/etc/sf-relay2/|g' tools/relay/sf-relay.service \
    | sudo tee /etc/systemd/system/sf-relay2.service > /dev/null
sudo systemctl daemon-reload
sudo systemctl enable --now sf-relay2.service
```

If `/etc/sf-relay2` is missing its two files, run the hook by hand:

```sh
sudo SF_RELAY_NAME=sf-relay2 RENEWED_LINEAGE=/etc/letsencrypt/live/relay2.example.net /usr/local/sbin/sf-relay-renew-hook
```

Check it from another machine with `sf-relay --check relay2.example.net:17443`.
Players enter `RELAY2.EXAMPLE.NET:17443`. The two relays share nothing: a code
from one is not found on the other, and stopping one leaves the other running.

Remove it when the test is over:

```sh
sudo systemctl disable --now sf-relay2.service
sudo rm /etc/systemd/system/sf-relay2.service
sudo systemctl daemon-reload
sudo certbot delete --cert-name relay2.example.net
sudo rm -r /etc/sf-relay2
sudo ufw delete allow 17443/udp
```

Then delete the `relay2` DNS record.

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
