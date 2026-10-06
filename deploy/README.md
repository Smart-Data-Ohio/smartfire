# Smart Data Campfire release procedure

The existing app remains an ONCE deployment on `campfire` in GCP project `smart-data-campfire`, zone `us-central1-a`. Its hostname is `chat.smartdata.net`. Update that deployment in place so its storage volume, account, users, messages, uploaded files, session signing key, and web-push keys remain attached.

The separate [Huddles media package](huddles/README.md) runs on `campfire-huddles` in the same zone. It is independent of the app's database. This separates resource usage and maintenance; it does not provide zone redundancy. The anticipated workspace size is about 135 users. Concurrent call participants and screen-share traffic need separate capacity testing; the initial two-CPU/eight-GiB media host is a pilot size, not a demonstrated 135-participant capacity.

## Automated path

The steps below are now automated by two GitHub Actions workflows that authenticate to GCP through Workload Identity Federation, with no long-lived key on either VM:

- **Publish image** (`.github/workflows/publish-image.yml`) builds the Rust port (`Dockerfile`) on every push to `main`, pushes its `linux/amd64` manifest to Artifact Registry as `rust-git-<full source sha>`, and attests its provenance. GHCR gets that same amd64 manifest, plus a native `linux/arm64` build, as a multi-arch image for self-hosters; nothing on the Artifact Registry side waits for it. Production has run the Rust port since 2026-10-05; the Rails `git-<sha>` images are no longer deployed.
- **Deploy to GCP** (`.github/workflows/deploy-gcp.yml`) is run by hand for a chosen source revision and environment. It resolves that tag to a digest and runs [`deploy/gcp/campfire-release.sh`](gcp/README.md) on the app VM over an IAP SSH tunnel, in phases: preflight, write freeze and backup, the isolated migration rehearsal of steps 4 and 5 below, boot-disk snapshot, the live `campfire db-migrate`, cutover, and finish. Its job summary contains a paste-ready release record for `docs/releases/`.

The automated path keeps the same ordering rule as the manual one: the migration is rehearsed against a *copy* of the frozen database, in a throwaway container with no network, **before** the live application is touched. Schema changes are SQL migrations compiled into the Rust binary; the server never migrates on boot and refuses a database whose migration versions differ from its own. The cutover therefore runs `campfire db-migrate` with the candidate image on the stopped live database first, and requires it to apply exactly what the rehearsal applied before `once update` starts the new container. The checks that run after the cutover are read-only, because by then the new image is serving and may have accepted writes. If the release does not complete, the recovery phase returns to the previous image. Because the previous image refuses a migrated database, recovery puts the frozen database back when every row of the live one still matches what the migration produced (the new image's boot always touches its job queue, so bytes alone can't tell); once anything else has written, it never restores over those writes and flags the run for an operator instead (keeping the previous image if it accepts the database, otherwise the candidate). A cutover that goes healthy and then fails a read-only check is left running untouched. The feed timer stays paused in every one of these cases.

Deploying to `production` requires a run dispatched on `main`, the revision to be an ancestor of `origin/main`, a successful `rust.yml` push or scheduled run for that exact sha with a successful `Rust port` job, a `rust-git-<sha>` image whose `GIT_REVISION` is that sha (checked in preflight), and an environment reviewer's approval. Start with `dry_run=true`, which stops after preflight and prints the plan.

Read [deploy/gcp/README.md](gcp/README.md) for the phase-by-phase contract and the release record layout. **The manual procedure below remains the authority on why each step exists, and is the procedure to follow by hand whenever the pipeline is unavailable.** Keep the two in step: a change to the cutover here is a change to the script.

## Candidate image

Build the committed source with Docker BuildKit. The app Dockerfile uses `COPY --chmod`, so Docker's legacy builder cannot complete it. Store verified images in the private Artifact Registry repository:

```text
us-central1-docker.pkg.dev/smart-data-campfire/campfire/app
```

Repository tags are immutable. Use a unique tag containing the full source revision, and deploy its resolved `sha256` digest. Record the source revision, image digest, and validation results together. The GCP registry is the delivery route; GHCR carries the same amd64 image for self-hosters but is never deployed from. Because tags cannot be moved, re-publishing an already-released revision resolves the existing digest instead of rebuilding over it.

The VMs have no runtime GCP service account. An authorized operator supplies a short-lived registry credential for a pull or push through protected standard input. Do not copy a long-lived service-account key onto either VM. Docker can restart an already-pulled container without renewing that credential. Future releases require fresh registry authentication.

## Backup and rehearsal

1. Run `once backup chat.smartdata.net <protected-backup-path>` on the app VM. ONCE stores application settings and keys alongside storage. Its Campfire pre-backup hook uses SQLite's backup API to write the consistent database snapshot at `data/backups/production.sqlite3` in the archive.
2. Keep a verified off-machine copy. Treat the entire archive as secret because it contains user data and application keys.
3. Separately preserve `/opt/campfire-open-roles`, `/etc/campfire-open-roles`, its systemd service and timer, and a SQLite backup of `/var/lib/campfire-open-roles/state.sqlite3`. The ONCE application archive does not include these host paths.
4. Extract an isolated copy and retain an untouched `before.sqlite3` snapshot. With network access disabled, run the candidate image's `campfire db-migrate` against only the copy, then `campfire db-check` (the server's strict boot schema check).
5. Run `campfire verify-additive-sqlite-migration BEFORE.sqlite3 AFTER.sqlite3`. Require exit zero and every preexisting table's schema and row data to match. This checks all old typed values, rather than counts alone. Separately compare every uploaded file's contents.
6. If you boot the candidate against the copy, do it with no external network. Never execute copied production background jobs during the rehearsal.

Migrations live in `crates/db/migrations/`; see [ops/README.md](../ops/README.md) for how to write one. SQLite may renumber foreign-key IDs when adding a constraint; the verifier compares complete constraint definitions while preserving their column order and referential actions.

Separately from these per-release checkpoints, the nightly-backup GitHub Actions workflow takes a rolling nightly backup (consistent database snapshot, uploads, config manifest) to Cloud Storage in a separate GCP project, with 7 daily, 4 weekly and 12 monthly retained. See [the backup runbook](../docs/backups.md).

## Public media acceptance

Wait for both Huddles DNS records and valid certificates. Check public WSS, direct media, forced TURN/TLS relay, and revocation with isolated test accounts. The optional browser test topology is documented in [the Huddles guide](../docs/huddles.md). Do not point the fixture test runner at the live app database.

Before cutover, replace all rehearsal media credentials with the separately generated production values, point the gateway callback to `https://chat.smartdata.net`, and close the temporary SSH forwards. The app receives the same production API/gateway secrets and uses the private API URL `http://10.128.0.3:7880`.

## Cutover and rollback

1. Authenticate Docker for the private registry and pull the exact candidate digest before interrupting chat.
2. Pause the Open Roles timer, wait for any current feed run to finish, and stop the existing ONCE app for a short write freeze. Take a fresh coherent backup of app storage, settings/keys, and feed delivery state. Retain the old image locally and take a fresh boot-disk snapshot while writes are stopped.
3. With the app still stopped, run the candidate's `campfire db-migrate /rails/storage/db/production.sqlite3` against the live volume in a one-off container with no network, and confirm it applied the same versions as the rehearsal. Then update the **existing** `chat.smartdata.net` ONCE application with the pinned image and `--auto-update=false`. For an image-only upgrade, omit `--env` to preserve its configured environment, including the five production LiveKit values and web-worker limit. ONCE 0.3.2 replaces the entire environment map when `--env` is supplied; any intentional environment change must therefore include all existing settings. Keep the same application identity and volume. Do not deploy a fresh application or restore over the live volume as an upgrade method.
4. Check schema migration success, preserved business records and uploaded files, signing/web-push key equality, health, assets, gateway authorization, and the running huddle reconciler. Set and test an explicit web-worker count suited to the chat VM's memory; do not assume a build VM's capacity is available on the app VM.
5. Resume the Open Roles timer only after validation. Record the exact image digest and backup paths.

The app VM carries a 1 GB swap file with `vm.swappiness=10`, applied idempotently by the pipeline's `prepare-host` phase (and on demand by the **Configure GCP host** workflow), so a memory spike during the overlap costs latency on a 2 GB host instead of an OOM kill.

ONCE starts a replacement container before retiring the prior one. The explicit write freeze prevents two app versions from writing the same SQLite database during migrations. Disable automatic upstream image updates because this is a maintained fork.

If rollback is needed before accepting new writes, stop the app and restore the coherent checkpoint with its original image and keys (the previous image refuses a migrated database, so the database must go back too). After new writes have been accepted, first preserve them: restoring an older checkpoint by itself would discard those messages and could make the feed repost alerts. Keep the feed's delivery state consistent with the restored message history.

## Periodic tasks

The `campfire` server runs the recurring tasks on loops inside its one process ([`crates/channels/src/jobs/periodic.rs`](../crates/channels/src/jobs/periodic.rs)), so the box needs no extra long-running process. Each tick runs each task whose interval has elapsed; a failing task is logged without stopping the others. Delayed jobs and retries with backoff need no task of their own: the durable job queue in SQLite claims each job when its `run_at` comes. The tasks include:

- **Event reminders** (every `EVENT_REMINDERS_INTERVAL` seconds, default 30): dispatches due event reminders.
- **Stuck rooms** (every 5 minutes): re-enqueues `Room::DestroyJob` for rooms marked deleted over 10 minutes ago that are still present, covering a destroy whose job never ran.
- **Data retention** (every `RETENTION_PRUNE_INTERVAL` seconds, default daily): enqueues `Retention::PruneJob`, which deletes, in batches:
  - `agent_events` older than 90 days,
  - `activity_items` the user has seen (read or handled) and untouched for 180 days — unread items are never pruned,
  - `github_webhook_deliveries` older than 14 days (the redelivery dedupe window is 7 days, still enforced at claim time),
  - completed `huddle_cleanups` older than 7 days (pending cleanups are never pruned),
  - revoked `huddle_grants` older than 30 days, along with their inbox items.

The windows live in [`crates/db/src/models/retention.rs`](../crates/db/src/models/retention.rs) so they are easy to change. The same run re-enqueues `Room::DestroyJob` for any room that has been marked deleted for over an hour but is still present, covering a destroy whose job never ran (queue outage, lost job).
