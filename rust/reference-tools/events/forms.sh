#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
STORE=${WS14E_STORE:-$ROOT/../.scratch/forms-reference}
mkdir -p "$STORE/db" "$STORE/storage"
# This directory is exclusively this generated oracle; fixtures must start clean.
rm -f "$STORE/db/production.sqlite3" "$STORE/db/production.sqlite3-wal" "$STORE/db/production.sqlite3-shm"
docker run --rm --name "ws14e-forms-$$" --cpus 2 --user "$(id -u):$(id -g)" \
  --env-file "$ROOT/parity/.env.reference" -e PARITY_REDIS=1 \
  -v "$(realpath "$STORE/db"):/rails/storage/db" \
  -v "$(realpath "$STORE/storage"):/rails/storage/files" \
  -v "$ROOT:/work:ro" "${PARITY_IMAGE:-ws14e-reference:pages-2e20b24c}" \
  bash -c 'bin/rails db:prepare >/dev/null && bin/rails runner /work/reference-tools/events/forms.rb'
cp "$STORE/db/event-forms.json" "$ROOT/crates/views/tests/golden/event-forms.json"

cp "$STORE/db/event-input.json" "$ROOT/crates/campfire/src/controllers/rooms/events/input.json"
