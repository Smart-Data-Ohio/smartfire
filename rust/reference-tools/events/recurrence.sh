#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
STORE=${WS14E_STORE:-$ROOT/../.scratch/recurrence-reference}
mkdir -p "$STORE/db" "$STORE/storage"
# This generated oracle owns its disposable database.
rm -f "$STORE/db/production.sqlite3" "$STORE/db/production.sqlite3-wal" "$STORE/db/production.sqlite3-shm"
docker run --rm --name "ws14e-recurrence-$$" --cpus 2 --user "$(id -u):$(id -g)" \
  --env-file "$ROOT/parity/.env.reference" -e PARITY_REDIS=1 \
  -v "$(realpath "$STORE/db"):/rails/storage/db" \
  -v "$(realpath "$STORE/storage"):/rails/storage/files" \
  -v "$ROOT:/work:ro" "${PARITY_IMAGE:-ws14e-reference:d7c7de92}" \
  bash -c 'bin/rails db:prepare >/dev/null && bin/rails runner /work/reference-tools/events/recurrence.rb && bin/rails runner /work/reference-tools/events/callbacks.rb'
cp "$STORE/db/event-recurrence.json" "$ROOT/crates/db/src/models/calendar_event/recurrence.json"
cp "$STORE/db/event-reminder-window.json" "$ROOT/crates/db/src/models/calendar_event/reminder_window.json"
cp "$STORE/db/event-create-callbacks.json" "$ROOT/crates/db/src/models/calendar_event/create_callbacks.json"
