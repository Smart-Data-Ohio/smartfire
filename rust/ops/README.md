# Rust image operations

The Rust app owns the schema. `campfire server` never migrates an existing database:
boot loads the compiled schema into an empty one and otherwise requires exactly the
compiled set of migration versions. The explicit commands need no app secrets:

```sh
campfire db-migrate /rails/storage/db/production.sqlite3
campfire db-check /rails/storage/db/production.sqlite3
campfire db-check --immutable /rails/storage/frozen.sqlite3
campfire verify-additive-sqlite-migration before.sqlite3 after.sqlite3
```

`db-migrate` applies this build's pending migrations. They are the SQL files in
`crates/db/migrations/<VERSION>_<name>.sql`, compiled into the binary, with
14-digit Rails-style versions after the last Rails migration (20261003180000).
All pending migrations and their `schema_migrations` rows go in one immediate
transaction with foreign keys off, followed by `PRAGMA foreign_key_check`. Any
failure leaves the database exactly as it was. When nothing is pending it writes
nothing. It refuses:

- versions it doesn't know, meaning a newer build already migrated the database
  (the downgrade guard);
- baseline versions that are missing;
- a path that doesn't exist (it never creates a database).

Migration SQL can't control transactions, attach databases, set pragmas or touch
`schema_migrations`.

`db-check` opens an existing database read-only. It checks integrity and the exact
compiled version set, and prints counts without row contents.

Use `--immutable` only for a standalone frozen snapshot on a read-only mount. It
refuses a nonempty WAL sidecar, because ignoring a live WAL would lose committed
data.

The additive verifier compares before and after backups. Preexisting columns,
indexes, foreign keys, triggers and typed row values must survive; extra tables
and columns are allowed. A mismatch exits 1; usage errors or unreadable input exit 2.

### Writing a migration

1. Add `crates/db/migrations/<VERSION>_<name>.sql`. Additive changes (new
   tables, columns, indexes) pass the release verifier. A table rebuild works
   because foreign keys are off during the run, but the verifier rejects it as
   not additive, so such a release needs an explicit decision.
2. Regenerate the schema files the build boots from:
   `CAMPFIRE_SCHEMA_DUMP=write cargo test -p campfire_db --lib schema::tests::schema_files`.
   That test fails while `crates/db/src/schema.sql`, `schema_migrations.txt` or
   `schema_sequences.txt` are stale, replacing `regenerate-schema.sh --check`.
   It loads `crates/db/baseline/` (the frozen Rails-era dump), applies every
   migration, and dumps.
3. `schema_sha1.txt` is not regenerated. It stays in `baseline/` as the Rails
   marker that fresh loads write to `ar_internal_metadata`. Nothing compares it
   after cutover.

Migrations must not insert rows into an empty database. The generator refuses
any such data. Backfills of existing rows are fine.

Local checks use fake Docker/ONCE/cloud boundaries:

```sh
python3 -m unittest discover -s rust/ops/tests -p 'test_release.py'
python3 -m unittest discover -s rust/ops/tests -p 'test_workflows.py'
WS18_BINARY="$PWD/rust/target/debug/campfire" python3 -m unittest discover -s rust/ops/tests -p 'test_additive_reference.py'
```

## Image and release contract

Build from the repository root, with the reference as a named context:

```sh
docker build -f rust/Dockerfile --build-context reference=. -t smartfire-rust rust
```

The image:

- has uid/gid 1000, `/rails`, `/rails/storage/{db,files,backups}`, ports 80/443,
  `bin/boot`, and both ONCE hooks;
- has no `HEALTHCHECK`;
- embeds assets digested from the checkout at build time;
- uses `CARGO_BUILD_JOBS` (default 4) and `CARGO_PROFILE` (default release).

`CAMPFIRE_STORAGE_PATH` takes precedence over `CAMPFIRE_STORAGE`, then defaults to
`/rails/storage` in the image. `CAMPFIRE_DATABASE_PATH`, `CAMPFIRE_FILES_PATH` and
`CAMPFIRE_BACKUPS_PATH` override those locations independently.

The backup shim:

- uses SQLite's online backup API and atomically replaces the completed snapshot;
- takes no outer flock, because the release and backup scripts already hold the
  release lock around it;
- retries busy pages, and on failure keeps the last complete backup and returns
  nonzero.

`net.smartdata.campfire.runtime=rust` identifies a Rust image.
`deploy/gcp/campfire-release.sh` only releases Rust to Rust. Preflight refuses a
candidate or current image without that label, and records both. Later phases
require the recorded target image to equal `IMAGE_REF`. The deploy workflow
resolves `rust-git-<sha>` once to `${GCP_IMAGE}@${digest}` and passes that string
to every phase.

**Freeze.** It stops the app, fingerprints the database and keeps a frozen copy.
It then rehearses on another copy with the candidate (no network, 768 MB):
`db-migrate`, `db-check`, before/after backups and the additive verifier. The
result records the image, the frozen database SHA-256 and the versions applied.

**Cutover.** It requires that rehearsal for this exact image and these exact bytes.
It runs `db-migrate` with the candidate on the stopped live database, and requires
it to apply exactly what the rehearsal applied (`live-migration-result.json`).
Only then does `once update` start the candidate.

**Rollback.** If the database is still the bytes the migration produced, rollback
puts the frozen copy back and returns to the previous image
(`migration-reverted`).

If anything else has written since, rollback keeps those writes:

- If the previous image's `db-check` accepts the database, rollback restarts the
  previous image (`refused-database-changed`).
- Otherwise it leaves the candidate running (`refused-database-incompatible`).

Both outcomes exit 30 for an operator.

The release lock, the 17:05-17:30 UTC refusal, the boot-disk snapshot and the
pre-release ONCE backup still apply.

The nightly backup script uses the shared admin path.
`restore-check.sh --image REF` (the monthly workflow builds REF from the checkout)
migrates a disposable copy and runs `db-check` on it.

`publish-rust-image.yml` publishes `rust-git-<full SHA>` to `GCP_IMAGE` on pushes to
main that touch the image's inputs. `deploy-gcp.yml` deploys only that tag. Its
single gate is a successful `rust.yml` push or scheduled run for the revision.

## Runtime environment at this workstream's base

Supported variables pass directly to the Rust config or front server:

| Variables | Meaning |
| --- | --- |
| `SECRET_KEY_BASE`, `SECRET_KEY_BASE_DUMMY` | Rails-compatible keys; dummy only for tests/offline operations |
| `RAILS_ENV` | database basename |
| `DISABLE_SSL`, `RAILS_LOG_LEVEL` | SSL middleware and log level |
| `APP_VERSION`, `GIT_REVISION` | version headers and UI |
| `RAILS_MAX_THREADS`, `JOB_CONCURRENCY` | reader pool and in-process job concurrency |
| `VAPID_PUBLIC_KEY`, `VAPID_PRIVATE_KEY` | Web Push; Rust also accepts `VAPID_SUBJECT` |
| `ADMIN_SESSION_IDLE_TIMEOUT_DAYS` | administrator session lifetime |
| `LIVEKIT_URL`, `LIVEKIT_INTERNAL_URL`, `LIVEKIT_API_KEY`, `LIVEKIT_API_SECRET`, `LIVEKIT_GATEWAY_SECRET` | CSP public origin and configured-huddle checks for room-removal broadcasts; full huddle integration is owned by WS13 |
| `HTTP_IDLE_TIMEOUT`, `HTTP_READ_TIMEOUT`, `HTTP_WRITE_TIMEOUT` | front-server timeouts |
| `EVENT_REMINDERS_INTERVAL`, `RETENTION_PRUNE_INTERVAL`, `HUDDLE_RECONCILE_INTERVAL` and the other periodic intervals | WS3's in-process scheduler; domain handlers remain with their owners |
| `TLS_DOMAIN`, `HTTP_PORT`, `HTTPS_PORT`, `TARGET_PORT`, `TARGET_BIND`, `ACME_DIRECTORY`, `STORAGE_PATH` and other Thruster settings (including `THRUSTER_` aliases) | built-in HTTP/TLS front server |

The following Rails variables are not implemented by the checked-out port. They
are preserved by ONCE; packaging does not pretend to implement those features:

| Ignored variables | Reason / owner |
| --- | --- |
| `RAILS_MASTER_KEY` | no encrypted Rails credentials loader; supply supported secrets directly as environment variables |
| `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_SIGN_IN_DOMAINS`, `GOOGLE_CALENDAR_WEBHOOK_URL`, `GOOGLE_PICKER_API_KEY`, `GOOGLE_CLOUD_PROJECT_NUMBER` | Google/OIDC/Calendar/Drive: WS14 and WS9 |
| `GITHUB_APP_CLIENT_ID`, `GITHUB_APP_CLIENT_SECRET`, `GITHUB_TOKEN`, `GITHUB_WEBHOOK_SECRET`, `FIZZY_API_BASE_URL` | integrations: WS15 |
| `INBOUND_EMAIL_DOMAIN`, `INBOUND_EMAIL_AUTHSERV_ID`, `RAILS_INBOUND_EMAIL_PASSWORD` | mail: WS10, in flight |
| `SMTP_ADDRESS`, `SMTP_PORT`, `SMTP_DOMAIN`, `SMTP_USER_NAME`, `SMTP_PASSWORD`, `SMTP_AUTHENTICATION`, `SMTP_ENABLE_STARTTLS`, `MAILER_FROM` | mail: WS10, in flight |
| `APP_URL` | absolute integration/mail URL configuration: WS10/WS14; current rendering uses the request origin |
| `LEGAL_OPERATOR_NAME`, `LEGAL_CONTACT_EMAIL`, `LEGAL_EFFECTIVE_DATE` | public-policy domain: WS8 |
| `SENTRY_DSN`, `SKIP_TELEMETRY` | sends no telemetry; Sentry is out of scope and remains an open question |
| `REDIS_URL` | no Redis in the Rust app |
| `PORT`, `PIDFILE`, `RAILS_MIN_THREADS`, `WEB_CONCURRENCY`, `FORK_PER_JOB`, `INTERVAL`, `RUBY_YJIT_ENABLE` | Puma/Resque/Ruby process controls; Rust uses one process and Thruster target settings |

Test-only Rails variables (`TEST_CLOCK_OFFSET_DAYS`, `LIVEKIT_SYSTEM_TEST*`,
`BACKUP_TEST_EXTRA_PATH`) belong to the Rails test harness. Rust uses its existing
`CAMPFIRE_FROZEN_TIME` test clock. `TMPDIR` remains the system temporary directory
contract. Commented-out cloud storage credentials in `storage.yml` are inactive in
both runtime contracts. Revisit this table as the other workstreams merge.

## Explicit Redis removal at cutover (lead only)

This branch neither removes a Redis process/sidecar nor deletes its AOF. The Rust
restore hook deliberately leaves `appendonlydir` and `appendonly.aof*` untouched.
The Rails image and its restore hook retain their current behavior.

After parity, production-copy rollback tests, and a protected checkpoint pass,
the lead can opt into removal during the controlled write freeze:

1. Confirm all Rails producers/workers and the particular Redis service/container
   have stopped. Record its exact name and AOF paths from the running deployment;
   do not infer a container name or remove any huddle sidecar.
2. Retain the Rails rollback image and encrypted checkpoint including its Redis
   persistence. Bring up Rust with the existing application environment/storage.
3. Verify the Rust process, `/up`, jobs, periodic tasks, cable and huddles. Stop and
   remove only the recorded Redis service/container when its rollback procedure
   is ready. Rust does not consume the old queue.
4. Remove only the recorded `appendonlydir`/legacy AOF paths from the live volume
   after explicit approval of the rollback/queue-disposition plan. Keep the
   protected checkpoint. Restoring old queued jobs against newer database writes
   is a separate operator decision.

LiveKit, the Node gateway and Caddy remain as deployed. No removal command is
automatically invoked by an image hook, release phase, or backup.
