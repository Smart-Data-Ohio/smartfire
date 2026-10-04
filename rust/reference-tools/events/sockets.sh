#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
source "$ROOT/reference-tools/reference_image_env.sh"
STORE=${WS14E_STORE:-$ROOT/../.scratch/sockets-reference}
mkdir -p "$STORE/db" "$STORE/storage"
# This directory is exclusively this generated oracle; fixtures must start clean.
rm -f "$STORE/db/production.sqlite3" "$STORE/db/production.sqlite3-wal" "$STORE/db/production.sqlite3-shm"
docker run --rm -e PARITY_REFERENCE_SHA --name "ws14e-sockets-$$" --cpus 2 --user "$(id -u):$(id -g)" \
  --env-file "$ROOT/parity/.env.reference" -e PARITY_REDIS=1 \
  -v "$(realpath "$STORE/db"):/rails/storage/db" \
  -v "$(realpath "$STORE/storage"):/rails/storage/files" \
  -v "$ROOT:/work:ro" "${PARITY_IMAGE:-campfire-reference}" \
  bash -c 'bin/rails db:prepare >/dev/null && bin/rails runner /work/reference-tools/events/sockets.rb'
cp "$STORE/db/event-sockets.json" "$ROOT/crates/campfire/src/channels/tests/golden/event-sockets.json"
