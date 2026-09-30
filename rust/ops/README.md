# Rust image operations

Rails remains the schema owner until cutover. `campfire server` never invokes the
migration runner on an existing database. The explicit commands need no app secrets:

```sh
campfire db-check /rails/storage/db/production.sqlite3
campfire db-check --immutable /rails/storage/frozen.sqlite3
campfire verify-additive-sqlite-migration before.sqlite3 after.sqlite3
campfire db-migrate /rails/storage/db/production.sqlite3 /rails/migrations
```

`db-check` opens an existing database read-only, checks integrity and the exact
compiled Rails migration set, and prints counts without row contents. Unknown
versions fail. A missing database is never created by these commands.
Use `--immutable` only for a standalone frozen snapshot on a read-only mount. It
refuses a nonempty WAL sidecar, because ignoring a live WAL would lose committed
data. This also permits reading a backup that retains SQLite's WAL journal-mode
header without creating sidecars on the read-only filesystem.

`db-migrate` is for **post-cutover use only**. Supply a complete directory of SQL
files named `VERSION_description.sql`, with canonical decimal Rails versions.
Versions run in numerical order. Every migration and its version insertion share
one immediate transaction. A failure rolls back that migration; earlier successful
migrations stay committed. Repeating the command skips recorded versions. The
runner refuses unknown versions outside the compiled Rails baseline and supplied
catalog, and refuses missing versions without migration definitions. A future
build's manifest can already include its pending versions; the explicit runner
applies their supplied definitions before that build's server can boot.
SQL cannot control transactions or
write the migration ledger itself. Regenerate the embedded schema and migration
version set when building an app that accepts the resulting database. Until
cutover those generated artifacts still come from the existing Rails reference
tools; a post-cutover release must carry the new schema and version manifest as
well as its migration definitions. Merely running a migration does not make an
older binary accept its version.

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

## Image and release contract

Build from the repository root with the reference as a named context:

```sh
docker build -f rust/Dockerfile --build-context reference=. -t smartfire-rust rust
```

The image has uid/gid 1000, `/rails`, `/rails/storage/{db,files,backups}`, ports
80/443, `bin/boot`, and both ONCE hooks. Like the Rails image it has no
`HEALTHCHECK`. Assets are digested and embedded from the Rails checkout at build
time. `CARGO_BUILD_JOBS` defaults to 4; `CARGO_PROFILE` defaults to release. A
developer can select dev; measurements intended to represent deployment use
release. `CARGO_CACHE_SCOPE` separates a worker's Docker target/registry caches.

`CAMPFIRE_STORAGE_PATH` takes precedence over `CAMPFIRE_STORAGE`, then defaults to
`/rails/storage` in the image. `CAMPFIRE_DATABASE_PATH`, `CAMPFIRE_FILES_PATH`, and
`CAMPFIRE_BACKUPS_PATH` independently override those locations. The backup shim
and restore hook use the same precedence and basename of the selected database.
The shim uses SQLite's online backup API and atomically replaces the completed
snapshot. It takes no outer flock: the release/backup scripts already hold the
release lock around it, and taking that lock again inside the container would
deadlock. Busy database pages are retried; failed snapshots preserve the last
complete backup and return nonzero.

`net.smartdata.campfire.runtime=rust` identifies a Rust image. The release
dispatcher performs a read-only label lookup before freeze/cutover. Unlabelled
images retain the original Rails rehearsal and process checks; unknown labels
fail. The committed command baseline covers those decision points exactly; the
dispatcher label inspection is additive. A Rust candidate must read the exact
deployed migration set on a frozen copy mounted read-only, then the previous
Rails image must boot and serve `/up` on another copy. Both containers have no
network and the same 768 MB cap. Rust never runs a migration in this path.
The existing release lock, 17:05–17:30 UTC refusal, snapshot, failure recovery,
and live-data preservation rules remain in force. The pre-cutover rehearsal
requires a previous Rails image; releasing Rust over an already-deployed Rust
image needs a later ops decision.

The nightly backup script already uses the shared admin path, so it needs no
runtime branch. `restore-check.sh --image REF` additionally checks a disposable
copy with either local runtime image; its existing `--rails-root` path remains.

`publish-rust-image.yml` builds PRs without publishing or cloud credentials. Main
publishes `rust-git-<full SHA>` to `GCP_IMAGE`, independently of immutable Rails
`git-<full SHA>` tags. Deploy's `runtime` defaults to `rails`; `rust` selects the
Rust tag and requires the Rust checks for production. Runtime selection inside
the host script still comes from image metadata. Publication and VM/validation
dry runs are performed by the lead, not by this workstream.

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
