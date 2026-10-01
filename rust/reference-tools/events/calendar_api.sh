#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
STORE=${WS14E_STORE:-$ROOT/../.scratch/calendar-api-reference}
mkdir -p "$STORE/db" "$STORE/storage"
rm -f "$STORE/db/production.sqlite3" "$STORE/db/production.sqlite3-wal" "$STORE/db/production.sqlite3-shm"
docker run --rm --name "ws14e-calendar-api-$$" --cpus 2 --user "$(id -u):$(id -g)" \
  --env-file "$ROOT/parity/.env.reference" -e PARITY_REDIS=1 \
  -v "$(realpath "$STORE/db"):/rails/storage/db" \
  -v "$(realpath "$STORE/storage"):/rails/storage/files" \
  -v "$ROOT:/work:ro" "${PARITY_IMAGE:-ws14e-reference:d7c7de92}" \
  bash -c 'bin/rails db:prepare >/dev/null && bin/rails runner /work/reference-tools/events/calendar_api.rb'
cp "$STORE/db/event-calendar-api.json" "$ROOT/crates/db/src/models/calendar_event/calendar_api.json"
