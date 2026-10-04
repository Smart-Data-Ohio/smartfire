#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
export WS9_ROLLBACK_DIR="$ROOT/../.scratch/auth/rollback"
export TMPDIR="$ROOT/../.scratch/tmp"
mkdir -p "$WS9_ROLLBACK_DIR" "$TMPDIR"
rm -f "$WS9_ROLLBACK_DIR/rollback.sqlite3" "$WS9_ROLLBACK_DIR/rollback.sqlite3-wal" "$WS9_ROLLBACK_DIR/rollback.sqlite3-shm" "$WS9_ROLLBACK_DIR/manifest.json" "$WS9_ROLLBACK_DIR/rails-readback.json"
cd "$ROOT"
cargo test --locked -j 4 -p campfire_db write_rails_rollback_fixture
docker run --rm --name ws9-two-factor-rollback --network none --entrypoint "" \
  --user "$(id -u):$(id -g)" --env-file "$ROOT/parity/.env.reference" \
  -e RAILS_LOG_LEVEL=fatal -e DATABASE_URL=sqlite3:/work/rollback/rollback.sqlite3 -e WS9_ROLLBACK_DIR=/work/rollback \
  -v "$ROOT/reference-tools:/work/reference-tools:ro" -v "$WS9_ROLLBACK_DIR:/work/rollback" \
  "${WS9_REFERENCE_IMAGE:-${PARITY_IMAGE:-campfire-reference}}" bin/rails runner /work/reference-tools/auth/rollback.rb
cargo test --locked -j 4 -p campfire_db read_rails_rollback_changes -- --ignored
