#!/usr/bin/env bash
# Runs campfire_db's differential tests against the campfire-reference image (built by
# `parity/bin/reference build`):
#
#   1. schema identity: the sqlite_master of a fresh reference `db:prepare`, of crates/db/src/schema.sql
#      and of a database the Rust crate created must be the same
#   2. fixtures_match_ruby_row_for_row against the reference's `db:fixtures:load`
#   3. scenario_matches_ruby against the reference after crates/db/ruby/scenario.rb
#   4. message_save_touches.json (which timestamps each Message save advances) is what
#      crates/db/ruby/save_touches.rb gets from the reference now
#   5. rollback: the reference boots on a database the Rust crate wrote (export_database_for_rails)
#      and reads, edits, searches and deletes through it (crates/db/ruby/rollback.rb)
#
# Databases land in OUT (default target/db-differential). CONTAINER_PREFIX, when set, names the
# reference containers (for a shared Docker host).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${OUT:-$ROOT/target/db-differential}
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target/db-differential/cargo}
rm -f "$OUT"/*.sqlite3 "$OUT"/*.sql "$OUT"/*.bot_key "$OUT"/*.json; mkdir -p "$OUT"

# The instant both fixture loaders are frozen at, with microseconds.
export CAMPFIRE_FIXTURES_NOW=${CAMPFIRE_FIXTURES_NOW:-$(date -u +"%Y-%m-%d %H:%M:%S.%6N")}

# sqlite_master minus what SQLite derives on its own (see crates/db/src/schema.rs).
SCHEMA_QUERY="SELECT sql || ';' FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'message_search_index_%' ORDER BY rowid"

reference() {
  docker run --rm --entrypoint "" ${CONTAINER_PREFIX:+--name "$CONTAINER_PREFIX-db-differential-$$"} \
    --cpus "${PARITY_CPUS:-2}" \
    --user "$(id -u):$(id -g)" \
    --env-file "$ROOT/parity/.env.reference" \
    -e RAILS_ENV=test -e RAILS_LOG_LEVEL=warn -e SCHEMA_QUERY="$SCHEMA_QUERY" -e CAMPFIRE_FIXTURES_NOW \
    -v "${CAMPFIRE_REFERENCE:-$ROOT/..}/test:/rails/test:ro" \
    -v "$ROOT/crates/db/ruby:/tools:ro" -v "$OUT:/out" \
    "${PARITY_IMAGE:-${REFERENCE_IMAGE:-campfire-reference:latest}}" sh -ec "$1" 2> >(grep -v -e VIPS -e '^$' >&2)
}

echo "== reference: db:prepare, db:fixtures:load, scenario.rb, save_touches.rb"
reference '
  db=storage/db/test.sqlite3
  bin/rails db:prepare >/dev/null
  sqlite3 $db "$SCHEMA_QUERY" > /out/schema_ruby.sql
  bin/rails runner /tools/load_fixtures.rb
  sqlite3 $db "PRAGMA wal_checkpoint(TRUNCATE)" >/dev/null && cp $db /out/fixtures_ruby.sqlite3
  bin/rails runner /tools/scenario.rb
  sqlite3 $db "PRAGMA wal_checkpoint(TRUNCATE)" >/dev/null && cp $db /out/scenario_ruby.sqlite3
  bin/rails runner /tools/save_touches.rb /out/save_touches_ruby.json'

echo "== campfire_db differential tests"
export CAMPFIRE_RUBY_FIXTURES_DB=$OUT/fixtures_ruby.sqlite3
export CAMPFIRE_RUBY_SCENARIO_DB=$OUT/scenario_ruby.sqlite3
export CAMPFIRE_EXPORT_DB=$OUT/rust_export.sqlite3
# Only these three tests belong to this fixture workflow. Other ignored tests (for example
# WS9's encrypted-credential round trip) have their own reference inputs and runners.
test_status=0
for test in tests::fixtures_test::fixtures_match_ruby_row_for_row \
    tests::differential_test::scenario_matches_ruby tests::fixtures_test::export_database_for_rails; do
  cargo test --locked -j "${CARGO_BUILD_JOBS:-4}" -p campfire_db "$test" -- --exact --ignored || test_status=1
done
[ "$test_status" -eq 0 ] || exit "$test_status"


echo "== message save touches"
diff -u "$ROOT/crates/db/src/tests/message_save_touches.json" "$OUT/save_touches_ruby.json"
echo "message_save_touches.json matches the reference"

echo "== schema identity"
sqlite3 "$OUT/rust_export.sqlite3" "$SCHEMA_QUERY" > "$OUT/schema_rust.sql"
diff -u "$ROOT/crates/db/src/schema.sql" "$OUT/schema_ruby.sql"
diff -u "$OUT/schema_ruby.sql" "$OUT/schema_rust.sql"
echo "schema.sql, reference db:prepare and Rust prepare agree"

echo "== reference on the Rust-written database"
reference '
  sqlite3 /out/rust_export.sqlite3 "PRAGMA wal_checkpoint(TRUNCATE)" >/dev/null
  cp /out/rust_export.sqlite3 storage/db/test.sqlite3
  bin/rails runner /tools/rollback.rb'
