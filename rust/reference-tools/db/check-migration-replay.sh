#!/usr/bin/env bash
# Checks that crates/db/src/schema.sql, which regenerate-schema.sh takes from `db:prepare` on an
# empty database (schema.rb), describes the same schema as a database built the way production
# ones were: every migration in db/migrate replayed from empty (`db:migrate`).
#
#   reference-tools/db/check-migration-replay.sh   # exits 1 and prints the differences, if any
#
# Byte equality isn't expected (a replayed table keeps its columns in migration order, schema.rb
# sorts them), so both sides are reduced to what the app depends on: per table its columns (type,
# NOT NULL, default, primary key, collation), foreign keys and CHECK constraints; indexes (table,
# columns in order, uniqueness, partial WHERE); triggers; virtual tables; and the version set.
# Runs in the campfire-reference image with a copy of the reference's db/ (db:migrate rewrites
# schema.rb). CONTAINER_PREFIX, when set, names the container.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REFERENCE_ROOT=$(cd "${CAMPFIRE_REFERENCE:-$ROOT/..}" && pwd)
IMAGE=${PARITY_IMAGE:-campfire-reference}

OUT=$(mktemp -d)
trap 'rm -rf "$OUT"' EXIT
cp -r "$REFERENCE_ROOT/db" "$OUT/db"
chmod -R a+rwX "$OUT"

docker run --rm --entrypoint "" ${CONTAINER_PREFIX:+--name "$CONTAINER_PREFIX-migration-replay-$$"} \
  --user "$(id -u):$(id -g)" \
  --env-file "$ROOT/parity/.env.reference" \
  -e RAILS_ENV=test -e RAILS_LOG_LEVEL=warn \
  -v "$OUT/db:/rails/db" -v "$OUT:/out" \
  "$IMAGE" sh -ec '
    rm -f storage/db/test.sqlite3*
    bin/rails db:migrate >/dev/null
    sqlite3 storage/db/test.sqlite3 "PRAGMA wal_checkpoint(TRUNCATE)" >/dev/null
    cp storage/db/test.sqlite3 /out/migrated.sqlite3' \
  2> >(grep -v -e VIPS -e '^$' >&2)

python3 - "$ROOT/crates/db/src/schema.sql" "$ROOT/crates/db/src/schema_migrations.txt" "$OUT/migrated.sqlite3" <<'PY'
import re, sqlite3, sys

schema_sql, versions_txt, migrated_path = sys.argv[1:]

def describe(conn):
    facts = set()
    master = conn.execute("SELECT type, name, tbl_name, sql FROM sqlite_master").fetchall()
    for type_, name, table, sql in master:
        if name.startswith("sqlite_") or name.startswith("message_search_index_"):
            continue
        if type_ == "table" and sql.startswith("CREATE VIRTUAL"):
            facts.add(f"virtual {name}: {sql}")
        elif type_ == "table":
            for cid, col, ctype, notnull, default, pk, hidden in conn.execute(f'PRAGMA table_xinfo("{name}")'):
                facts.add(f"column {name}.{col} {ctype} notnull={notnull} default={default} pk={pk} hidden={hidden}")
                collation = re.search(rf'"{re.escape(col)}" [^,]*COLLATE (\w+)', sql)
                if collation:
                    facts.add(f"collate {name}.{col} {collation.group(1)}")
            for row in conn.execute(f'PRAGMA foreign_key_list("{name}")'):
                _, _, ref, from_, to, on_update, on_delete, match = row
                facts.add(f"foreign key {name}.{from_} -> {ref}.{to} on_update={on_update} on_delete={on_delete}")
            for check in re.findall(r'CHECK \((.*?)\)(?:,|\)$)', sql):
                facts.add(f"check {name}: {check}")
            if "AUTOINCREMENT" in sql:
                facts.add(f"autoincrement {name}")
            for _, index, unique, origin, partial in conn.execute(f'PRAGMA index_list("{name}")'):
                columns = [r[2] for r in conn.execute(f'PRAGMA index_xinfo("{index}")') if r[5]]
                if origin == "c":
                    index_sql = conn.execute("SELECT sql FROM sqlite_master WHERE name = ?", (index,)).fetchone()[0]
                    where = index_sql.split(" WHERE ", 1)[1] if " WHERE " in index_sql else None
                    facts.add(f"index {index} on {name}({', '.join(map(str, columns))}) unique={unique} where={where}")
                else:
                    facts.add(f"constraint index on {name}({', '.join(map(str, columns))}) unique={unique} origin={origin}")
        elif type_ in ("trigger", "view"):
            facts.add(f"{type_} {name}: {sql}")
    return facts

loaded = sqlite3.connect(":memory:")
loaded.executescript(open(schema_sql).read())
migrated = sqlite3.connect(migrated_path)

status = 0
only_prepare = sorted(describe(loaded) - describe(migrated))
only_migrate = sorted(describe(migrated) - describe(loaded))
for line in only_prepare:
    print(f"schema.sql only: {line}")
for line in only_migrate:
    print(f"migrations only: {line}")
if only_prepare or only_migrate:
    status = 1

versions = {v for v in open(versions_txt).read().split() if v}
replayed = {r[0] for r in migrated.execute("SELECT version FROM schema_migrations")}
if versions != replayed:
    print(f"versions: schema_migrations.txt only {sorted(versions - replayed)}, replay only {sorted(replayed - versions)}")
    status = 1

tables = sum(1 for f in describe(loaded) if f.startswith("column ") and f.split()[1].endswith(".id"))
if status == 0:
    print(f"migration replay matches schema.sql: {len(describe(loaded))} facts, {tables} tables with an id, {len(versions)} versions")
sys.exit(status)
PY
