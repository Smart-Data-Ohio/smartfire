#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
DIR="$ROOT/../.scratch/auth/profile-rollback"
export TMPDIR="$ROOT/../.scratch/tmp"
export WS9_PROFILE_ROLLBACK_DB="$DIR/profile.sqlite3"
export CI=1
mkdir -p "$DIR" "$TMPDIR"
rm -f "$WS9_PROFILE_ROLLBACK_DB" "$WS9_PROFILE_ROLLBACK_DB-wal" "$WS9_PROFILE_ROLLBACK_DB-shm"
cd "$ROOT"
cargo test --locked -j 4 -p campfire app::profile_security_tests::self_changed_email_persists_marker_and_audit_for_rails_rollback -- --exact
docker run --rm --name ws9-profile-rollback --network none --entrypoint '' \
  --user "$(id -u):$(id -g)" --env-file "$ROOT/parity/.env.reference" \
  -e RAILS_LOG_LEVEL=fatal -e DATABASE_URL=sqlite3:/work/review/profile.sqlite3 \
  -v "$DIR:/work/review" -v "$ROOT/reference-tools/auth/profile_rollback.rb:/work/profile_rollback.rb:ro" \
  "${WS9_REFERENCE_IMAGE:-ws9-reference:d7c7de92}" bin/rails runner /work/profile_rollback.rb
