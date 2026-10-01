#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
STORE=${WS14E_STORE:-$ROOT/../.scratch/http-errors-reference}
mkdir -p "$STORE/db" "$STORE/storage"
# This directory is exclusively this generated oracle; fixtures must start clean.
rm -f "$STORE/db/production.sqlite3" "$STORE/db/production.sqlite3-wal" "$STORE/db/production.sqlite3-shm"
docker run --rm --name "ws14e-http-errors-$$" --cpus 2 --user "$(id -u):$(id -g)" \
  --env-file "$ROOT/parity/.env.reference" -e PARITY_REDIS=1 \
  -v "$(realpath "$STORE/db"):/rails/storage/db" \
  -v "$(realpath "$STORE/storage"):/rails/storage/files" \
  -v "$ROOT:/work:ro" "${PARITY_IMAGE:-ws14e-reference:d7c7de92}" \
  bash -c 'bin/rails db:prepare >/dev/null && bin/rails runner /work/reference-tools/events/http_errors.rb'
cp "$STORE/db/event-http-errors.json" "$ROOT/crates/campfire/src/controllers/rooms/events/http_errors.json"
