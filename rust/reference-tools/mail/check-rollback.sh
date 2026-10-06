#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REFERENCE_ROOT=${CAMPFIRE_REFERENCE:-$(dirname "$ROOT")}
mkdir -p "$ROOT/target"
EXPORT_DIR=$(mktemp -d "$ROOT/target/ws10-rollback.XXXXXX")
CAMPFIRE_MAIL_EXPORT_DIR="$EXPORT_DIR" \
  cargo test --manifest-path "$ROOT/Cargo.toml" -p campfire_mail --test inbound \
  -j 4 export_for_rails -- --ignored --exact --nocapture
docker run --rm --name ws10-mail-rollback --entrypoint sh \
  --env-file "$ROOT/parity/.env.reference" -e RAILS_ENV=test \
  -v "$REFERENCE_ROOT/app:/rails/app:ro" \
  -v "$REFERENCE_ROOT/config:/rails/config:ro" \
  -v "$REFERENCE_ROOT/test:/rails/test:ro" \
  -v "$ROOT/reference-tools/mail:/tools:ro" \
  -v "$EXPORT_DIR:/rails/storage" \
  campfire-reference:latest -ec 'bin/rails runner /tools/rollback.rb'
