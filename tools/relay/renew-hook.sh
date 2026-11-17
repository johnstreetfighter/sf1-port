#!/bin/sh
# Certbot deploy hook for the Internet room relay: copy the renewed
# certificate where the service can read it and restart the service.
# Install as /usr/local/sbin/sf-relay-renew-hook and pass it to certbot with
# --deploy-hook; see docs/relay-deployment.md. Certbot sets RENEWED_LINEAGE.
set -eu
install -d -m 0750 -o root -g sf-relay /etc/sf-relay
install -m 0640 -o root -g sf-relay "$RENEWED_LINEAGE/fullchain.pem" /etc/sf-relay/fullchain.pem
# The relay reads PKCS #8 keys only; this converts any key certbot wrote.
umask 027
openssl pkcs8 -topk8 -nocrypt -in "$RENEWED_LINEAGE/privkey.pem" -out /etc/sf-relay/privkey.pem.new
chgrp sf-relay /etc/sf-relay/privkey.pem.new
mv /etc/sf-relay/privkey.pem.new /etc/sf-relay/privkey.pem
# The relay reads its certificate at start. A restart closes the open rooms.
# try-restart leaves a relay that is not running yet alone.
systemctl try-restart sf-relay.service
