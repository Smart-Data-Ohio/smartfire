#!/usr/bin/env bash
# Record both cable goldens from a private, freshly initialized Rails instance.
# Requires the pinned parity image (tag it as ws7-reference:fec615be first).
set -euo pipefail

RUST_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
SCRATCH=${WS7_SCRATCH:-/home/riels/.cache/rust-port/ws7}
IMAGE=${WS7_REFERENCE_IMAGE:-ws7-reference:fec615be}
PORT=${WS7_REFERENCE_PORT:-47040}
NAME=ws7-reference-$PORT
[[ "$IMAGE" == ws7-* ]] || { echo "image must start with ws7-" >&2; exit 1; }
(( PORT >= 47000 && PORT <= 47049 )) || { echo "port must be in the worker's range" >&2; exit 1; }
if docker container inspect "$NAME" >/dev/null 2>&1; then
  echo "$NAME already exists; refusing to replace it" >&2
  exit 1
fi
mkdir -p "$SCRATCH/tmp" "$SCRATCH/evidence"
STORAGE=$(mktemp -d "$SCRATCH/reference.XXXXXX")
mkdir -p "$STORAGE/db" "$STORAGE/files"
trap 'docker rm -f "$NAME" >/dev/null 2>&1 || true' EXIT
docker run -d --name "$NAME" --cpus 2 --user "$(id -u):$(id -g)" \
  --env-file "$RUST_ROOT/parity/.env.reference" -p "127.0.0.1:$PORT:80" \
  -v "$STORAGE/db:/rails/storage/db" -v "$STORAGE/files:/rails/storage/files" \
  -v "$RUST_ROOT:/work:ro" "$IMAGE" >/dev/null
for attempt in $(seq 1 120); do
  if curl -fsS -o /dev/null "http://127.0.0.1:$PORT/up" 2>/dev/null; then break; fi
  if (( attempt == 120 )); then docker logs --tail 30 "$NAME"; exit 1; fi
  sleep 0.5
done
# Prove the baked reference uses exactly the channel and lease source we're porting.
python3 - "$RUST_ROOT/.." "$NAME" <<'PY'
import hashlib
from pathlib import Path
import subprocess
import sys
root, container = Path(sys.argv[1]), sys.argv[2]
files = sorted(str(path.relative_to(root)) for path in (root / "app/channels").rglob("*.rb"))
files += ["app/models/workspace_presence_lease.rb", "config/initializers/turbo_streams_authorization.rb",
          "app/controllers/messages_controller.rb", "app/helpers/messages_helper.rb",
          "app/views/messages/boosts/_boost.html.erb", "app/views/messages/boosts/_reaction.html.erb",
          "app/views/polls/_poll.html.erb"]
actual = subprocess.check_output(["docker", "exec", container, "sha256sum", *["/rails/" + file for file in files]], text=True).splitlines()
for file, line in zip(files, actual, strict=True):
    assert hashlib.sha256((root / file).read_bytes()).hexdigest() == line.split()[0], file
print(f"reference source hashes: {len(files)} matched; 0 mismatched")
PY
cd "$RUST_ROOT"
docker exec -e RAILS_LOG_LEVEL=warn "$NAME" bin/rails runner /work/crates/campfire/src/channels/tests/golden/fixtures.rb > "$SCRATCH/evidence/channels-fixtures.json"
docker exec -e RAILS_LOG_LEVEL=warn "$NAME" bin/rails runner /work/crates/cable/tests/golden/fixtures.rb > "$SCRATCH/evidence/cable-fixtures.json"
export TMPDIR="$SCRATCH/tmp" CARGO_TARGET_DIR="$RUST_ROOT/target" CABLE_TEST_PORT_RANGE=47000-47039
CHANNELS_REFERENCE_PORT="$PORT" CHANNELS_REFERENCE_CONTAINER="$NAME" CHANNELS_REFERENCE_FIXTURES="$SCRATCH/evidence/channels-fixtures.json" \
  cargo test -j 4 -p campfire channels::tests::golden::record_reference -- --ignored
CABLE_REFERENCE_URL="ws://127.0.0.1:$PORT/cable" CABLE_REFERENCE_FIXTURES="$SCRATCH/evidence/cable-fixtures.json" \
  cargo test -j 4 -p campfire_cable --test golden record_reference -- --ignored
echo "reference recording: 70 channel steps and the cable protocol golden regenerated"
