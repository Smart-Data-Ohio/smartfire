#!/usr/bin/env bash
# Checks that crates/db/src/schema.sql, which regenerate-schema.sh takes from `db:prepare` on an
# empty database (schema.rb), describes the same schema as a database built the way production
# ones were: every migration in db/migrate replayed from empty (`db:migrate`).
#
#   reference-tools/db/check-migration-replay.sh              # exits 1 and prints the differences
#   reference-tools/db/check-migration-replay.sh --self-test  # only the mutation self-test
#
# Byte equality isn't expected (a replayed table keeps its columns in migration order, schema.rb
# sorts them), so both sides are reduced to what the app depends on: per table its columns (type,
# NOT NULL, default, primary key, collation), foreign keys and CHECK constraints; indexes (table,
# uniqueness, each key's column or expression in order with its sort direction and collation,
# partial WHERE); triggers; views; virtual tables; and the version set.
#
# Every run first proves the comparison discriminates: it mutates schema.sql in memory (an index
# expression, key order, sort direction, collation, partial WHERE, a default) and requires each
# mutation to show up as a difference. Then it replays the migrations in the campfire-reference
# image with a copy of the reference's db/ (db:migrate rewrites schema.rb). CONTAINER_PREFIX,
# when set, names the container.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REFERENCE_ROOT=$(cd "${CAMPFIRE_REFERENCE:-$ROOT/..}" && pwd)
IMAGE=${PARITY_IMAGE:-campfire-reference}

compare() { python3 - "$ROOT/crates/db/src/schema.sql" "$ROOT/crates/db/src/schema_migrations.txt" "$@" <<'PY'
import re, sqlite3, sys

schema_sql, versions_txt, mode = sys.argv[1:4]
SCHEMA = open(schema_sql).read()

def split_top_level(text):
    """The comma-separated terms of an index's key list, ignoring commas inside parentheses."""
    terms, depth, current = [], 0, ""
    for char in text:
        if char == "," and depth == 0:
            terms.append(current)
            current = ""
            continue
        depth += {"(": 1, ")": -1}.get(char, 0)
        current += char
    return [" ".join(t.split()) for t in terms + [current]]

def index_definition(sql):
    """The key terms (column or expression, with any COLLATE/ASC/DESC as written) and the WHERE."""
    start = re.search(r'\bON\s+"?\w+"?\s*\(', sql).end()
    depth, end = 1, start
    while depth:
        depth += {"(": 1, ")": -1}.get(sql[end], 0)
        end += 1
    rest = sql[end:].strip()
    where = " ".join(rest[len("WHERE "):].split()) if rest.upper().startswith("WHERE ") else None
    return split_top_level(sql[start:end - 1]), where

def index_keys(conn, index, terms=None):
    """Per key: its column name (or the expression's text), sort direction and collation."""
    keys = [r for r in conn.execute(f'PRAGMA index_xinfo("{index}")') if r[5]]
    described = []
    for position, (_, cid, column, desc, collation, _) in enumerate(keys):
        term = terms[position] if terms else None
        target = f"expr {term}" if cid == -2 else f"column {column}" + (f" as {term}" if term else "")
        described.append(f"{target} desc={desc} coll={collation}")
    return "; ".join(described)

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
                if origin == "c":
                    index_sql = conn.execute("SELECT sql FROM sqlite_master WHERE name = ?", (index,)).fetchone()[0]
                    terms, where = index_definition(index_sql)
                    facts.add(f"index {index} on {name} unique={unique} partial={partial} keys=[{index_keys(conn, index, terms)}] where={where}")
                else:
                    facts.add(f"constraint index on {name} unique={unique} origin={origin} keys=[{index_keys(conn, index)}]")
        elif type_ in ("trigger", "view"):
            facts.add(f"{type_} {name}: {sql}")
    return facts

def load(sql):
    conn = sqlite3.connect(":memory:")
    conn.executescript(sql)
    return conn

def differences(left, right):
    a, b = describe(left), describe(right)
    return sorted(a - b), sorted(b - a)

if mode == "--self-test":
    # Each mutation of schema.sql must change the facts, or the comparison can't see that kind of
    # drift. The first is the review's: an index expression swapped for another.
    mutations = [
        ("index expression", "(LOWER(github_login))", "(LENGTH(github_login))"),
        ("key order", '("message_id", "message_checksum")', '("message_checksum", "message_id")'),
        ("sort direction", '("agent_id", "external_id") WHERE', '("agent_id" DESC, "external_id") WHERE'),
        ("key collation", '("agent_id", "external_id") WHERE', '("agent_id" COLLATE NOCASE, "external_id") WHERE'),
        ("partial WHERE", "WHERE github_login IS NOT NULL", "WHERE github_login IS NULL"),
        ("column default", '"kind" varchar DEFAULT \'personal\' NOT NULL', '"kind" varchar DEFAULT \'shared\' NOT NULL'),
    ]
    original = describe(load(SCHEMA))
    for label, before, after in mutations:
        if SCHEMA.count(before) != 1:
            sys.exit(f"self-test: {label}: expected one {before!r} in schema.sql, found {SCHEMA.count(before)}")
        mutated = describe(load(SCHEMA.replace(before, after)))
        if mutated == original:
            sys.exit(f"self-test: {label} ({before} -> {after}) went unnoticed")
        print(f"self-test: {label} caught: -{sorted(original - mutated)[0]}")
    print(f"self-test: all {len(mutations)} mutations caught")
    sys.exit(0)

loaded = load(SCHEMA)
migrated = sqlite3.connect(mode)

status = 0
only_prepare, only_migrate = differences(loaded, migrated)
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

facts = describe(loaded)
tables = sum(1 for f in facts if f.startswith("column ") and f.split()[1].endswith(".id"))
indexes = sum(1 for f in facts if f.startswith("index "))
if status == 0:
    print(f"migration replay matches schema.sql: {len(facts)} facts, {tables} tables with an id, {indexes} indexes, {len(versions)} versions")
sys.exit(status)
PY
}

compare --self-test
[ "${1:-}" = --self-test ] && exit 0

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

compare "$OUT/migrated.sqlite3"
