#!/bin/bash
# Turn an Ubuntu 24.04 server into an Internet room relay.
#
#   sudo tools/relay/setup-ubuntu.sh <domain> [--binary <path>] [--email <addr>]
#
# Run it from a checkout of the repository, as a sudo user who logs in with
# an SSH key. It is safe to run again: every step checks what is already in
# place. It accepts the Let's Encrypt subscriber agreement for you. See
# docs/relay-deployment.md.
set -euo pipefail

usage() {
    echo "usage: sudo $0 <domain> [--binary <path>] [--email <addr>]" >&2
    exit 2
}

say() { printf '\n== %s\n' "$*"; }
warn() { printf 'WARNING: %s\n' "$*" >&2; }
die() { printf 'setup-ubuntu: %s\n' "$*" >&2; exit 1; }

domain=""
binary=""
email=""
while [ $# -gt 0 ]; do
    case "$1" in
        --binary) [ $# -ge 2 ] || usage; binary="$2"; shift 2 ;;
        --email) [ $# -ge 2 ] || usage; email="$2"; shift 2 ;;
        -*) usage ;;
        *) [ -z "$domain" ] || usage; domain="$1"; shift ;;
    esac
done
[ -n "$domain" ] || usage

[ "$(id -u)" -eq 0 ] || die "run it with sudo"
# shellcheck disable=SC1091
. /etc/os-release
[ "${ID:-}" = ubuntu ] && [ "${VERSION_ID:-}" = 24.04 ] || die "this script supports Ubuntu 24.04 only"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
for file in renew-hook.sh sf-relay.service; do
    [ -f "$here/$file" ] || die "$here/$file is missing; run the script from a repository checkout"
done
if [ -n "$binary" ]; then
    [ -f "$binary" ] || die "$binary is not a file"
elif [ ! -x /usr/local/bin/sf-relay ]; then
    die "no relay is installed yet; pass --binary <path>"
fi

port=7443
hook=/usr/local/sbin/sf-relay-renew-hook
lineage="/etc/letsencrypt/live/$domain"
# The relay restarts only when its binary or unit changed, so a second run
# does not close open rooms.
restart=no

# Install $1 at $2 with mode $3. Returns 1 when $2 already held the same
# bytes, so callers can act only on a change. A failure stops the script
# here, because `set -e` is off inside a function called from `if` or `||`.
install_if_changed() {
    if [ -f "$2" ] && cmp -s "$1" "$2"; then
        return 1
    fi
    install -D -m "$3" "$1" "$2" || die "cannot install $2"
}

# Write stdin to $1 as a 0644 file. Returns 1 when it was already the same.
write_if_changed() {
    local new changed=0
    new="$(mktemp)" || die "cannot make a temporary file"
    cat > "$new"
    install_if_changed "$new" "$1" 0644 || changed=1
    rm -f "$new"
    return "$changed"
}

say "Packages"
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
# The distribution certbot, not the snap.
apt-get install -y -q certbot ufw unattended-upgrades openssl

say "Service account"
if ! id sf-relay > /dev/null 2>&1; then
    useradd --system --no-create-home --shell /usr/sbin/nologin sf-relay
fi

say "Firewall"
# Keep every port sshd listens on open, so enabling the firewall cannot cut
# this session off.
ssh_ports="$(sshd -T 2> /dev/null | awk '$1 == "port" { print $2 }' | sort -u || true)"
[ -n "$ssh_ports" ] || ssh_ports=22
ufw default deny incoming
ufw default allow outgoing
for ssh_port in $ssh_ports; do
    ufw allow "$ssh_port/tcp"
done
# TCP 80 is for certificate issue and renewal.
ufw allow 80/tcp
ufw allow "$port/udp"
ufw --force enable

say "Relay binary"
if [ -n "$binary" ]; then
    if [ -f /usr/local/bin/sf-relay ] && ! cmp -s "$binary" /usr/local/bin/sf-relay; then
        cp -p /usr/local/bin/sf-relay /usr/local/bin/sf-relay.previous
        echo "kept the old binary as /usr/local/bin/sf-relay.previous"
    fi
    if install_if_changed "$binary" /usr/local/bin/sf-relay 0755; then
        restart=yes
    fi
fi

say "Relay service unit"
# The unit goes in before the certificate: the deploy hook restarts it by
# name. It starts at the end, once its certificate is in place.
if install_if_changed "$here/sf-relay.service" /etc/systemd/system/sf-relay.service 0644; then
    systemctl daemon-reload
    restart=yes
fi

say "Certificate"
install_if_changed "$here/renew-hook.sh" "$hook" 0755 || true
if [ -n "$email" ]; then
    account=(--email "$email" --no-eff-email)
else
    account=(--register-unsafely-without-email)
fi
if [ -d "$lineage" ]; then
    if grep -qsF "renew_hook = $hook" "/etc/letsencrypt/renewal/$domain.conf"; then
        echo "the certificate for $domain already uses the deploy hook"
    else
        echo "a certificate for $domain exists; setting its deploy hook"
        certbot reconfigure --non-interactive --cert-name "$domain" --deploy-hook "$hook"
    fi
else
    certbot certonly --standalone --non-interactive --agree-tos "${account[@]}" \
        --cert-name "$domain" -d "$domain" --deploy-hook "$hook"
fi
# Run the hook by hand when /etc/sf-relay lacks the current certificate,
# as it does when the certificate was issued before the hook was set. The
# hook restarts a running relay, so it is skipped when nothing would change.
if ! cmp -s "$lineage/fullchain.pem" /etc/sf-relay/fullchain.pem || [ ! -s /etc/sf-relay/privkey.pem ]; then
    RENEWED_LINEAGE="$lineage" "$hook"
fi
[ -s /etc/sf-relay/fullchain.pem ] && [ -s /etc/sf-relay/privkey.pem ] || die "the deploy hook left no certificate in /etc/sf-relay"
certbot renew --dry-run --cert-name "$domain"

say "Renewal time"
# Renew before dawn in US East, away from evening play. A renewal restarts
# the relay and closes the open rooms.
if write_if_changed /etc/systemd/system/certbot.timer.d/sf-relay.conf << 'EOF'
[Timer]
OnCalendar=
OnCalendar=*-*-* 09:30:00 UTC
RandomizedDelaySec=30min
EOF
then
    systemctl daemon-reload
    systemctl restart certbot.timer
fi
systemctl enable --now certbot.timer

say "Security updates"
write_if_changed /etc/apt/apt.conf.d/20auto-upgrades << 'EOF' || true
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
EOF
# No automatic reboots: a reboot closes every open room.
write_if_changed /etc/apt/apt.conf.d/52sf-relay-no-reboot << 'EOF' || true
Unattended-Upgrade::Automatic-Reboot "false";
EOF

say "Journal size"
if write_if_changed /etc/systemd/journald.conf.d/sf-relay.conf << 'EOF'
[Journal]
SystemMaxUse=100M
EOF
then
    systemctl restart systemd-journald
fi

say "Relay service"
systemctl enable sf-relay.service
if [ "$restart" = yes ]; then
    systemctl restart sf-relay.service
else
    systemctl start sf-relay.service
fi
sleep 2
systemctl is-active --quiet sf-relay.service || die "sf-relay.service did not stay up; see journalctl -u sf-relay"

say "SSH login"
login="${SUDO_USER:-root}"
home="$(getent passwd "$login" | cut -d: -f6)"
keys="$home/.ssh/authorized_keys"
if [ "$login" = root ]; then
    warn "SSH is unchanged: the script was run as root, and closing root login would lock you out."
    warn "Run it again with sudo from a user who logs in with an SSH key."
elif ! grep -qsE '^[[:space:]]*[^#[:space:]]' "$keys"; then
    warn "SSH is unchanged: $keys holds no key, so password login stays on."
    warn "Add your public key for $login and run the script again."
else
    # The first value sshd reads wins, so this file sorts before the others.
    write_if_changed /etc/ssh/sshd_config.d/00-sf-relay.conf << 'EOF' || true
PermitRootLogin no
PasswordAuthentication no
KbdInteractiveAuthentication no
EOF
    sshd -t
    # ssh.service is not running under socket activation with no session open.
    systemctl try-reload-or-restart ssh
    effective="$(sshd -T | grep -E '^(permitrootlogin|passwordauthentication) ' | sort | tr '\n' ' ')"
    [ "$effective" = "passwordauthentication no permitrootlogin no " ] || die "sshd still reports: $effective"
    echo "root and password login are off; $login logs in with a key"
    echo "keep this session open and confirm a new SSH login works before you close it"
fi

say "Done"
echo "The relay for $domain is running on UDP $port."
echo "Check it from another machine:"
echo "    sf-relay --check --min-days 14 $domain"
