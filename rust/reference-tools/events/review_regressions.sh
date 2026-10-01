#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
STORE=${WS14E_STORE:-$ROOT/../.scratch/pr174-reference}
mkdir -p "$STORE/db" "$STORE/storage"
rm -f "$STORE/db/production.sqlite3" "$STORE/db/production.sqlite3-wal" "$STORE/db/production.sqlite3-shm"
docker run --rm --name "ws14e-pr174-reference-$$" --cpus 2 --user "$(id -u):$(id -g)" \
  --env-file "$ROOT/parity/.env.reference" -e PARITY_REDIS=1 \
  -v "$(realpath "$STORE/db"):/rails/storage/db" -v "$(realpath "$STORE/storage"):/rails/storage/files" \
  -v "$ROOT:/work:ro" "${PARITY_IMAGE:-ws14e-reference:d7c7de92}" \
  bash -c 'bin/rails db:prepare >/dev/null && bin/rails runner /work/reference-tools/events/review_regressions.rb'
cp "$STORE/db/event-review-regressions.json" "$ROOT/crates/campfire/src/controllers/rooms/events/event-review-regressions.json"
python3 - "$ROOT" <<'PYCODE'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
data = json.loads((root / 'crates/campfire/src/controllers/rooms/events/event-review-regressions.json').read_text())
(root / 'crates/db/src/tests/calendar_event_test/invitation-boundaries.json').write_text(json.dumps(data['invitations'], indent=2) + '\n')
PYCODE
