#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REFERENCE_ROOT=${CAMPFIRE_REFERENCE:-$(dirname "$ROOT")}
source "$ROOT/reference-tools/rails_link_mounts.sh"
mapfile -t LINK_MOUNTS < <(rails_link_mounts "$REFERENCE_ROOT")
mkdir -p "$ROOT/vectors/mail"
docker run --rm --name ws10-mail-goldens --entrypoint sh \
  --env-file "$ROOT/parity/.env.reference" -e RAILS_ENV=test \
  -v "$REFERENCE_ROOT/test:/rails/test:ro" "${LINK_MOUNTS[@]}" \
  -v "$REFERENCE_ROOT/app/mailboxes:/rails/app/mailboxes:ro" \
  -v "$REFERENCE_ROOT/app/mailers:/rails/app/mailers:ro" \
  -v "$REFERENCE_ROOT/app/views/security_mailer:/rails/app/views/security_mailer:ro" \
  -v "$REFERENCE_ROOT/app/views/two_factor_mailer:/rails/app/views/two_factor_mailer:ro" \
  -v "$REFERENCE_ROOT/app/views/layouts/mailer.html.erb:/rails/app/views/layouts/mailer.html.erb:ro" \
  -v "$REFERENCE_ROOT/app/views/layouts/mailer.text.erb:/rails/app/views/layouts/mailer.text.erb:ro" \
  -v "$ROOT/reference-tools/mail:/tools:ro" -v "$ROOT/vectors/mail:/out" \
  campfire-reference:latest -ec 'bin/rails runner /tools/generate.rb'
