#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REFERENCE_ROOT=${CAMPFIRE_REFERENCE:-$(dirname "$ROOT")}
source "$ROOT/reference-tools/rails_link_mounts.sh"
mapfile -t LINK_MOUNTS < <(rails_link_mounts "$REFERENCE_ROOT")
# Redis is only the Rails oracle's test dependency, never the Rust mail implementation.
docker run --rm --name ws10-mail-rails-tests --entrypoint sh \
  --env-file "$ROOT/parity/.env.reference" -e RAILS_ENV=test -e PARALLEL_WORKERS=1 \
  -v "$REFERENCE_ROOT/app:/rails/app:ro" \
  -v "$REFERENCE_ROOT/config:/rails/config:ro" \
  -v "$REFERENCE_ROOT/db:/rails/db:ro" \
  -v "$REFERENCE_ROOT/test:/rails/test:ro" "${LINK_MOUNTS[@]}" \
  campfire-reference:latest -ec 'redis-server --daemonize yes; bin/rails db:prepare; bin/rails test test/mailboxes/room_mailbox_test.rb test/mailboxes/bounce_mailbox_test.rb test/mailboxes/relay_ingress_test.rb test/mailers/security_mailer_test.rb test/mailers/two_factor_mailer_test.rb test/controllers/rooms/inbound_email_addresses_controller_test.rb'
