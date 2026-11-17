#!/bin/sh
# Certbot deploy hook for the Internet room relay: copy the renewed
# certificate where the service can read it and restart the service.
# Install as /usr/local/sbin/sf-relay-renew-hook and pass it to certbot with
# --deploy-hook; see docs/relay-deployment.md. Certbot sets RENEWED_LINEAGE.
#
# SF_RELAY_NAME names the service and its /etc directory. It defaults to
# sf-relay; a second relay on the same server sets another name.
set -eu
name="${SF_RELAY_NAME:-sf-relay}"
install -d -m 0750 -o root -g sf-relay "/etc/$name"
install -m 0640 -o root -g sf-relay "$RENEWED_LINEAGE/fullchain.pem" "/etc/$name/fullchain.pem"
# The relay reads PKCS #8 keys only; this converts any key certbot wrote.
umask 027
openssl pkcs8 -topk8 -nocrypt -in "$RENEWED_LINEAGE/privkey.pem" -out "/etc/$name/privkey.pem.new"
chgrp sf-relay "/etc/$name/privkey.pem.new"
# openssl writes a private key as 0600 whatever the umask; the relay reads
# it through its group.
chmod 0640 "/etc/$name/privkey.pem.new"
mv "/etc/$name/privkey.pem.new" "/etc/$name/privkey.pem"
# The relay reads its certificate at start. A restart closes the open rooms.
# try-restart leaves a relay that is not running yet alone.
systemctl try-restart "$name.service"
