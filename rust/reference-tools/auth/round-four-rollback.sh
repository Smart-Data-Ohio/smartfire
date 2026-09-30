#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
DIR="$ROOT/../.scratch/auth/round-four-rollback"
export TMPDIR="$ROOT/../.scratch/tmp" CI=1
export WS9_ZONE_ROLLBACK_DB="$DIR/zone.sqlite3"
mkdir -p "$DIR" "$TMPDIR"
rm -f "$WS9_ZONE_ROLLBACK_DB" "$WS9_ZONE_ROLLBACK_DB-wal" "$WS9_ZONE_ROLLBACK_DB-shm"
cd "$ROOT"
cargo test --locked -j 4 -p campfire app::round_four_security_tests::profile_zone_case_and_alias_validation_matches_rails_without_partial_writes -- --exact
docker run --rm --name ws9-zone-rollback --network none --entrypoint '' \
  --user "$(id -u):$(id -g)" --env-file "$ROOT/parity/.env.reference" \
  -e RAILS_LOG_LEVEL=fatal -e DATABASE_URL=sqlite3:/work/review/zone.sqlite3 \
  -v "$DIR:/work/review" -v "$ROOT/reference-tools/auth/round_four_rollback.rb:/work/round_four_rollback.rb:ro" \
  "${WS9_REFERENCE_IMAGE:-ws9-reference:d7c7de92}" bin/rails runner /work/round_four_rollback.rb
