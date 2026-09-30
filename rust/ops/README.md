# Rust image operations

Rails remains the schema owner until cutover. `campfire server` never invokes the
migration runner on an existing database. The explicit commands need no app secrets:

```sh
campfire db-check /rails/storage/db/production.sqlite3
campfire verify-additive-sqlite-migration before.sqlite3 after.sqlite3
campfire db-migrate /rails/storage/db/production.sqlite3 /rails/migrations
```

`db-check` opens an existing database read-only, checks integrity and the exact
compiled Rails migration set, and prints counts without row contents. Unknown
versions fail. A missing database is never created by these commands.

`db-migrate` is for **post-cutover use only**. Supply a complete directory of SQL
files named `VERSION_description.sql`, with canonical decimal Rails versions.
Versions run in numerical order. Every migration and its version insertion share
one immediate transaction. A failure rolls back that migration; earlier successful
migrations stay committed. Repeating the command skips recorded versions. The
runner refuses unknown versions outside the compiled Rails baseline and supplied
catalog, and refuses missing baseline versions. SQL cannot control transactions or
write the migration ledger itself. Regenerate the embedded schema and migration
version set with the existing Rails reference tools when building an app that
accepts the resulting database; merely running a migration does not make an older
binary accept its version.

Before any post-cutover migration, snapshot the database, rehearse on a separate
copy, and run the preservation verifier against before/after snapshots. It follows
our Ruby verifier: preserve preexisting columns, indexes, foreign keys, triggers,
and typed row values; allow additional tables and columns; ignore Rails internal
metadata tables. A mismatch exits 1, usage or unreadable input exits 2. The runner
is deliberately absent from every release and startup path before cutover.

Local command-trace checks use fake Docker/ONCE/cloud boundaries:

```sh
python3 -m unittest discover -s rust/ops/tests -p 'test_release.py'
WS18_BINARY="$PWD/rust/target/debug/campfire" python3 -m unittest discover -s rust/ops/tests -p 'test_additive_reference.py'
```

The committed Rails baseline comes from the recorded `origin/main` SHA. The
differential uses `ws6-reference-d7c7de92:latest` unless
`WS18_REFERENCE_IMAGE` selects another **pinned local reference image**.
