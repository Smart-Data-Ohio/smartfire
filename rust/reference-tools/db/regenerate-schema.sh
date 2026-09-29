#!/usr/bin/env bash
# Regenerates crates/db/src/schema.sql, schema_migrations.txt and schema_sha1.txt from the reference
# (Rails) app, the way the Rails image creates a database on first boot: `bin/rails db:prepare`
# (bin/start-app) on an empty database, which loads db/schema.rb and records every migration
# version. Never edit these files by hand.
#
#   reference-tools/db/regenerate-schema.sh           # writes the three files
#   reference-tools/db/regenerate-schema.sh --check   # exits 1 if any would change
#
# schema.sql is that database's sqlite_master, minus the objects SQLite derives on its own (FTS5
# shadow tables, sqlite_sequence, autoindexes), in creation order. schema_migrations.txt is its
# schema_migrations table in insertion order (see `assume_migrated_upto_version`), and
# schema_sha1.txt the `schema_sha1` Rails recorded in ar_internal_metadata (the SHA1 of schema.rb).
#
# Runs in the campfire-reference image (`parity/bin/reference build`) with the reference's db/
# mounted over the image's, so a schema change in the Rails app needs no image rebuild. Rails and
# the sqlite3 gem come from the image, so rebuild it when Gemfile.lock changes (checked below).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
# The reference Rails app: CAMPFIRE_REFERENCE, else the repository root that contains rust/.
REFERENCE_ROOT=$(cd "${CAMPFIRE_REFERENCE:-$ROOT/..}" && pwd)
IMAGE=${PARITY_IMAGE:-campfire-reference}
DEST=$ROOT/crates/db/src
CHECK=
[ "${1:-}" = --check ] && CHECK=1

# Keep in sync with reference-tools/db/differential.sh.
SCHEMA_QUERY="SELECT sql || ';' FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'message_search_index_%' ORDER BY rowid"
VERSIONS_QUERY="SELECT version FROM schema_migrations ORDER BY rowid"
SHA1_QUERY="SELECT value FROM ar_internal_metadata WHERE key = 'schema_sha1'"

docker image inspect "$IMAGE" >/dev/null 2>&1 || { echo "no $IMAGE image: run parity/bin/reference build" >&2; exit 1; }
if ! docker run --rm --entrypoint "" "$IMAGE" cat Gemfile.lock | cmp -s - "$REFERENCE_ROOT/Gemfile.lock"; then
  echo "$IMAGE was built from a different Gemfile.lock than $REFERENCE_ROOT: rebuild it (parity/bin/reference build)" >&2
  exit 1
fi

OUT=$(mktemp -d)
trap 'rm -rf "$OUT"' EXIT
chmod 777 "$OUT"

docker run --rm --entrypoint "" \
  --user "$(id -u):$(id -g)" \
  --env-file "$ROOT/parity/.env.reference" \
  -e RAILS_ENV=test -e RAILS_LOG_LEVEL=warn \
  -e SCHEMA_QUERY="$SCHEMA_QUERY" -e VERSIONS_QUERY="$VERSIONS_QUERY" -e SHA1_QUERY="$SHA1_QUERY" \
  -v "$REFERENCE_ROOT/db:/rails/db:ro" -v "$OUT:/out" \
  "$IMAGE" sh -ec '
    rm -f storage/db/test.sqlite3*
    bin/rails db:prepare >/dev/null
    sqlite3 storage/db/test.sqlite3 "$SCHEMA_QUERY" > /out/schema.sql
    sqlite3 storage/db/test.sqlite3 "$VERSIONS_QUERY" > /out/schema_migrations.txt
    sqlite3 storage/db/test.sqlite3 "$SHA1_QUERY" > /out/schema_sha1.txt' \
  2> >(grep -v -e VIPS -e '^$' >&2)

status=0
for file in schema.sql schema_migrations.txt schema_sha1.txt; do
  if [ -n "$CHECK" ]; then
    if ! diff -u "$DEST/$file" "$OUT/$file"; then
      status=1
    fi
  else
    cp "$OUT/$file" "$DEST/$file"
    echo "wrote crates/db/src/$file"
  fi
done
exit $status
