#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REFERENCE_ROOT=${CAMPFIRE_REFERENCE:-$(dirname "$ROOT")}
docker run --rm --name ws10-mail-review-regressions --entrypoint sh \
  --env-file "$ROOT/parity/.env.reference" -e RAILS_ENV=test -e PARALLEL_WORKERS=1 \
  -v "$REFERENCE_ROOT/app:/rails/app:ro" -v "$REFERENCE_ROOT/config:/rails/config:ro" \
  -v "$REFERENCE_ROOT/db:/rails/db:ro" -v "$REFERENCE_ROOT/test:/rails/test:ro" \
  -v "$ROOT/reference-tools/mail:/tools:ro" \
  campfire-reference:latest -ec 'redis-server --daemonize yes; bin/rails db:prepare; bin/rails test /tools/review_test.rb'
