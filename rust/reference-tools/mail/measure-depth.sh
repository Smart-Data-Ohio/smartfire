#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REFERENCE_ROOT=${CAMPFIRE_REFERENCE:-$(dirname "$ROOT")}
CACHE=${WS10_MIME_DEPTH_CACHE:-/home/riels/.cache/rust-port/ws10/mime-depth}
SEED=${WS10_MIME_DEPTH_SEED:-$ROOT/parity/.seed/default}
IMAGE=${WS10_MIME_DEPTH_IMAGE:-campfire-reference:latest}
IMAGE=$(docker image inspect "$IMAGE" --format '{{.Id}}')
python3 - "$ROOT" "$REFERENCE_ROOT" <<'PY'
from pathlib import Path
import subprocess
import sys
for name in ('app/mailboxes/application_mailbox.rb', 'app/mailboxes/room_mailbox.rb',
             'app/mailboxes/bounce_mailbox.rb', 'config/environments/production.rb', 'Gemfile.lock'):
    pinned = subprocess.check_output(['git', '-C', sys.argv[1], 'show', f'fec615be:{name}'])
    assert (Path(sys.argv[2]) / name).read_bytes() == pinned, f'reference differs from fec615be: {name}'
PY
mkdir -p "$CACHE/db" "$CACHE/storage" "$CACHE/out"
rm -f "$CACHE/db/production.sqlite3-wal" "$CACHE/db/production.sqlite3-shm"
cp "$SEED/db/production.sqlite3" "$CACHE/db/production.sqlite3"
cp -a "$SEED/storage/." "$CACHE/storage/"
docker run --rm --name ws10-mail-mime-depth --entrypoint sh \
  --env-file "$ROOT/parity/.env.reference" -e RAILS_ENV=production -e PARITY_REDIS=1 -e WS10_MIME_DEPTH_IMAGE="$IMAGE" \
  -v "$REFERENCE_ROOT/app:/rails/app:ro" -v "$REFERENCE_ROOT/config:/rails/config:ro" \
  -v "$REFERENCE_ROOT/db:/rails/db:ro" -v "$REFERENCE_ROOT/lib:/rails/lib:ro" \
  -v "$ROOT/reference-tools/mail:/tools:ro" \
  -v "$CACHE/db:/rails/storage/db" -v "$CACHE/storage:/rails/storage/files" -v "$CACHE/out:/out" \
  "$IMAGE" -ec 'redis-server --daemonize yes; bin/rails runner /tools/mime_depth.rb "$@"' -- "$@"
