# Smartfire backups

Rolling daily backups of the app VM, stored encrypted in Cloud Storage in a
**separate GCP project**, plus a nightly boot-disk snapshot schedule. The
whole chain is: the `Nightly backup` GitHub Actions workflow produces a
consistent backup on the VM every night without stopping the app,
downloads the encrypted archive and uploads it, lifecycle rules age out old
prefixes, and a monthly GitHub Actions workflow proves the latest backup
restores.

## Contents

- [Architecture](#architecture)
- [Identities and least privilege](#identities-and-least-privilege)
- [Retention](#retention)
- [Setup (lead only, once)](#setup-lead-only-once)
- [The nightly workflow](#the-nightly-workflow)
- [Manual backups](#manual-backups)
- [List the backups](#list-the-backups)
- [Download and decrypt one](#download-and-decrypt-one)
- [Verify its integrity](#verify-its-integrity)
- [Restore onto the VM](#restore-onto-the-vm)
- [Restore onto a fresh VM](#restore-onto-a-fresh-vm)
- [Roll back](#roll-back)
- [Snapshot schedule vs release snapshots](#snapshot-schedule-vs-release-snapshots)
- [Monthly automated restore check](#monthly-automated-restore-check)
- [Key management](#key-management)
- [Troubleshooting](#troubleshooting)

## Architecture

Every night at 09:00 UTC the `Nightly backup` GitHub Actions workflow
(`.github/workflows/nightly-backup.yml`) copies `campfire-backup.sh` to the
app VM and runs it over an IAP SSH tunnel on the VM **host**, not inside
the app container. Nothing is installed on the VM: each run carries the
script with it, so the backup always runs the code on `main`. One run
produces one object:

```
smartfire-backup-20260201-090000.tar.gz.age
```

uploaded to `gs://<backup-bucket>/daily/`. Inside the tarball:

```
smartfire-backup-20260201-090000/
  production.sqlite3   a consistent snapshot via SQLite's online backup API
  files/               a copy of the Active Storage uploads tree
  manifest.json        non-secret facts: stamp, hosts, image ref,
                       SHA-256 of the database, file counts (never secrets,
                       never anything from the app environment)
  SHA256SUMS           checksums of everything above
```

The tarball is compressed with gzip and encrypted client-side with `age`
(default) or `gpg` to a public recipient. The private key is never on the VM.

The database snapshot never stops the app or freezes writes. In container
mode (production) the script runs `script/admin/prepare-backup` inside the
running app container, which uses SQLite's backup API and holds only brief
page locks. The uploads copy follows the database snapshot, so a file
uploaded in between may be absent from that night's backup; the next night
picks it up. If the app container is not running (a release freeze or an
outage), the script fails loudly rather than backing up a half-state.

Encryption happens on the VM, to the age public recipient, so only the
encrypted archive ever crosses the tunnel back to the runner. The runner
verifies its SHA-256 against what the script reported, then uploads it to
`daily/`. On Sundays the same file is additionally uploaded under `weekly/`,
and on the 1st of the month under `monthly/`. Each prefix is a separate
upload of the same local bytes, not a server-side copy: the uploader
credential is create-only and cannot read anything back to copy it. The
workflow removes the staging from the VM afterwards, whether the run
succeeded or failed.

The backup refuses to run on top of a release: before snapshotting it takes
`flock -n` on `/var/lock/campfire-release.lock`, the same lock
`campfire-release.sh` uses. When a release holds the lock the script exits
75 so the workflow fails loudly (rerun it after the release), and
`storage/backups/production.sqlite3` is never written concurrently with a
release. It also checks free space up front (about 3x the database plus
uploads), prunes work directories older than a day, and removes all
plaintext staging on exit.

The scripts and lifecycle config live in [deploy/backups/](../deploy/backups/):

| File | Purpose |
| --- | --- |
| `campfire-backup.sh` | produces one encrypted archive on the VM (run by the workflow, or by hand with `--output-dir`) |
| `backup.env.example` | config template for manual runs |
| `lifecycle.json` | bucket lifecycle rules |
| `setup-backup-project.sh` | one-time infra setup (the lead runs it) |
| `snapshot-schedule.sh` | nightly boot-disk snapshot schedule |
| `restore-check.sh` | decrypt-and-verify, shared by CI and operators |

## Identities and least privilege

Backups go to Cloud Storage in a **separate GCP project**
(`smart-data-campfire-backups`) so that a compromised app project, or an
account that administers it, cannot delete the backups. The bucket lives in
the US multi-region, so it survives the loss of the app region too.

| Account | Granted | Used by |
| --- | --- | --- |
| `smartfire-backup-runner@smart-data-campfire` (in the app project) | `roles/storage.objectCreator` only on the backup bucket (cross-project, bucket-level), plus IAP-tunnel and SSH roles **on the campfire VM only** (`roles/iap.tunnelResourceAccessor`, `roles/compute.osAdminLogin`, `roles/compute.viewer`) | the nightly backup workflow via Workload Identity Federation |
| `smartfire-backup-reader@smart-data-campfire-backups` | `roles/storage.objectViewer` only | the monthly restore-check workflow via Workload Identity Federation |

Neither bucket grant can delete or overwrite objects: the runner can only
create new objects, and the reader can only read them. The bucket
additionally has Object Versioning and 30-day soft delete, so even a
backup-project admin's delete is recoverable: a deleted live object
survives as a noncurrent version for 30 days, and soft-deleted objects are
recoverable for 30 days. This is why the lifecycle keeps a
`daysSinceNoncurrentTime` rule rather than letting versions accumulate
forever. The runner deliberately holds no snapshot, deploy or Artifact
Registry rights: it can reach the VM and create backup objects, nothing
else.

The VM itself has no service account and needs none, and it is never
stopped for backups: the VM only produces the encrypted archive, while the
workflow's identity does the downloading and uploading. No keys are ever
copied onto the VM.

Both workflows impersonate their identity through Workload Identity
Federation with no long-lived keys. Each binding pins the exact
ID-qualified subject GitHub mints for runs of `Smart-Data-Ohio/smartfire`
on `refs/heads/main` (the same form as the deployer's bindings), so
dispatch the workflows by hand from main.

## Retention

| Prefix | Rule | Keeps |
| --- | --- | --- |
| `daily/` | deleted at age 8 days | 7 daily backups |
| `weekly/` | deleted at age 29 days | 4 weekly backups |
| `monthly/` | deleted at age 366 days | 12 monthly backups |
| noncurrent versions | deleted 30 days after becoming noncurrent | delete recovery window |
| incomplete multipart uploads | aborted after 1 day | no orphaned parts |

Ages are one day past the multiple because Cloud Storage counts age from
the midnight after creation; the +1 guarantees at least the stated count
survives. Boot-disk snapshots from the schedule below are kept 14 days.

## Setup (lead only, once)

Prerequisites: `gcloud` authenticated as someone who can administer the
backup project and manage IAM in the app project, and this repository
checked out. The backup project `smart-data-campfire-backups` already
exists with billing linked and the storage and IAM APIs enabled. The
script is idempotent: re-running it converges without duplicating
anything.

1. Pick the bucket name and run the setup script. The projects, app VM and
   repository default to the real values; only the bucket is required. It
   creates the bucket, the backup-runner identity in the app project with
   its Workload Identity pool, the reader identity with its pool, and the
   snapshot schedule, and prints every value to configure next. Project
   creation and billing linking stay manual: if the project does not
   exist the script prints the commands and stops.

   ```sh
   BACKUP_BUCKET=smartfire-backups-xxx \
     bash deploy/backups/setup-backup-project.sh
   ```

2. Follow the manual steps it prints: generate the age keypair off the VM,
   set the GitHub repository variables and the private-key secret, then
   run the `Nightly backup` workflow once by hand from main, followed by
   the restore-check workflow. Nothing is installed on the VM.

## The nightly workflow

`Nightly backup` (`.github/workflows/nightly-backup.yml`) runs at 09:00
UTC every day, and on manual dispatch from `main`. Each run:

1. authenticates to the app project through Workload Identity Federation
   as the `smartfire-backup-runner` (no long-lived keys);
2. makes sure `age` is installed on the VM (installed with apt when
   missing), copies `campfire-backup.sh` to the VM, and runs it over IAP
   SSH with the public recipient from the `BACKUP_AGE_RECIPIENT`
   variable;
3. downloads the encrypted archive the script produced, verifies its
   SHA-256 against what the script reported, and fails loudly on any
   mismatch;
4. uploads it to `daily/` (plus `weekly/` on Sundays and `monthly/` on
   the 1st) with the workflow's own create-only credential;
5. removes the staging from the VM, whether the run succeeded or failed,
   and writes a summary naming the uploaded objects and the checksum.

One run at a time: the workflow serializes itself with a concurrency
group. If a release holds the VM's release lock the script exits 75 and
the run fails with a rerun-after-the-release message; the next night's run
retries naturally.

Each run also writes a non-secret summary on the VM to
`/var/backups/campfire-nightly-last.json` (stamp, file, SHA-256, sizes).
Alert if that file goes stale: it is the cheapest proof the workflow is
reaching the VM (see [Troubleshooting](#troubleshooting)).

## Manual backups

For a one-off backup by hand (for example, preserving the live state
before a restore), copy the script to the VM and run it with the public
recipient. The private key must never be on the VM.

```sh
# From your machine (zone and project default to the app VM):
gcloud compute scp deploy/backups/campfire-backup.sh campfire:~/manual-backup/ \
  --tunnel-through-iap --zone=us-central1-a --project=smart-data-campfire
# On the VM:
sudo env BACKUP_AGE_RECIPIENT='age1...' \
  bash ~/manual-backup/campfire-backup.sh --output-dir /var/backups/manual
```

`deploy/backups/backup.env.example` documents every setting when the
one-liner above is not enough. Fetch the encrypted archive the same way
the workflow does (`gcloud compute scp` through IAP after a `chown`, or
any other copy), verify its SHA-256 against the `BACKUP_SHA256=` line the
script printed, and remove the staging from the VM afterwards.

## List the backups

From any machine authenticated as the backup reader (or a backup admin):

```sh
BUCKET=smartfire-backups-xxx   # the name chosen at setup
gcloud storage ls "gs://${BUCKET}/daily/" | sort
gcloud storage ls "gs://${BUCKET}/weekly/" | sort
gcloud storage ls "gs://${BUCKET}/monthly/" | sort
# The newest daily:
gcloud storage ls "gs://${BUCKET}/daily/" | sort | tail -n 1
```

## Download and decrypt one

You need the age private key (or the GPG private key) from the sealed ops
store. It must never be left on the VM afterwards.

```sh
BUCKET=smartfire-backups-xxx
OBJECT=daily/smartfire-backup-20260201-090000.tar.gz.age
gcloud storage cp "gs://${BUCKET}/${OBJECT}" ./restore/
cd restore
age --decrypt --identity /path/to/smartfire-backup-key.txt \
  --output backup.tar.gz "$(basename "$OBJECT")"
# or, for gpg backups:
# gpg --decrypt --output backup.tar.gz "$(basename "$OBJECT")"
tar -tzf backup.tar.gz   # proves the gzip layer and the tar index are intact
mkdir extracted && tar -xzf backup.tar.gz -C extracted
```

`restore-check.sh` does the decrypt, extract and all of the next section in
one command; prefer it (it is also what CI runs):

```sh
bash deploy/backups/restore-check.sh --backup restore/"$(basename "$OBJECT")" \
  --work-dir restore/checked --age-identity /path/to/smartfire-backup-key.txt
```

## Verify its integrity

Inside the extracted `smartfire-backup-*/` directory:

```sh
cd extracted/smartfire-backup-20260201-090000
sha256sum -c SHA256SUMS          # every file must report OK
sqlite3 production.sqlite3 'PRAGMA integrity_check;'   # must print exactly: ok
sqlite3 production.sqlite3 'SELECT count(*) FROM users; SELECT count(*) FROM messages;'
```

Compare the database SHA-256 with `manifest.json`, and the manifest's file
count with what you see. `restore-check.sh` additionally boots the Rails
app against a disposable copy of the database with `PRAGMA query_only` and
counts rows through the models (`--rails-root`), then re-verifies the
extracted files are byte-identical. If any check fails, stop: pick the
previous daily (or the newest weekly) and verify that one instead.

## Restore onto the VM

This replaces the live database and uploads. It needs a write freeze like
a release does, because two writers must never share one SQLite database.

1. Announce the maintenance window. There is no way to restore without
   dropping the writes accepted after the backup was taken; preserve first
   if they matter (take a fresh backup as in
   [Manual backups](#manual-backups) and download it).
2. Verify the chosen backup on another machine as above. Never restore an
   unverified backup.
3. On the VM, stop the app and confirm nothing is running:

   ```sh
   sudo once stop chat.smartdata.net
   sudo docker ps --filter 'label=once' --format '{{.Names}}'
   # must print nothing
   ```

4. Find the storage volume through the stopped container (never by guessing
   from the volume list) and copy the verified files over it. The live
   files move timestamped aside, in case the restore itself is bad:

   ```sh
   STAMP="$(date -u +%Y%m%d-%H%M%S)"
   CONTAINER="$(sudo docker ps -a --filter 'label=once' --format '{{.Names}}' | grep '^once-app-' | head -n 1)"
   test -n "$CONTAINER" || { echo "no ONCE app container found"; exit 1; }
   VOLUME="$(sudo docker inspect --format '{{range .Mounts}}{{if eq .Destination "/rails/storage"}}{{.Name}}{{end}}{{end}}' "$CONTAINER")"
   test -n "$VOLUME" || { echo "no /rails/storage mount on $CONTAINER"; exit 1; }
   MOUNT="$(sudo docker volume inspect "$VOLUME" --format '{{.Mountpoint}}')"
   sudo cp -a "$MOUNT/db/production.sqlite3" "/var/backups/pre-restore-$STAMP.sqlite3"
   [ -f "$MOUNT/db/production.sqlite3-wal" ] && sudo mv "$MOUNT/db/production.sqlite3-wal" "/var/backups/pre-restore-$STAMP.sqlite3-wal"
   [ -f "$MOUNT/db/production.sqlite3-shm" ] && sudo mv "$MOUNT/db/production.sqlite3-shm" "/var/backups/pre-restore-$STAMP.sqlite3-shm"
   sudo install -m 0644 production.sqlite3 "$MOUNT/db/production.sqlite3"
   sudo chown --reference="/var/backups/pre-restore-$STAMP.sqlite3" "$MOUNT/db/production.sqlite3"
   sudo mv "$MOUNT/files" "/var/backups/pre-restore-files-$STAMP"
   sudo cp -a files "$MOUNT/files"
   sudo chown -R --reference="/var/backups/pre-restore-files-$STAMP" "$MOUNT/files"
   ```

   Two details matter here. First, move any `-wal`/`-shm` sidecars aside
   with the database instead of deleting them: the snapshot is
   self-contained, and a stale WAL from the previous database would
   corrupt the restored one, so the sidecars must not stay next to the
   restored file — but deleting database bytes is never the restore's
   job, and a bad restore rolls back exactly only when every pre-restore
   byte was kept. Second, the app container runs as UID 1000, not root,
   so every restored file must keep the live ownership (the
   `chown --reference` lines): root-owned files would leave the app
   unable to write. `cp -a` alone is not enough, because it preserves the
   *extracting* machine's owner ids.

5. Start the app and validate like a release cutover: `/up` returns 200,
   recent messages and uploads are present, the signing keys are unchanged
   (the restore does not touch ONCE settings), and the huddle reconciler
   process is running. See [deploy/README.md](../deploy/README.md#cutover-and-rollback)
   for the checklist. If the restore drops messages the Open Roles feed
   already announced, reconcile its delivery state as in a rollback.
6. Resume normal timers. Record what was restored, from which object, and
   which writes were dropped.

## Restore onto a fresh VM

For a new VM (disaster recovery, or rehearsing one): the nightly backup
carries the database and uploads, but deliberately no secrets and no ONCE
settings. Those come from a release-time `once backup` archive (which
holds settings and keys) or from a manual reconfiguration.

1. Provision the VM (same zone `us-central1-a` keeps the snapshot schedule
   and any zone-local assumptions valid) with Docker and ONCE.
2. Recreate the ONCE application with the image digest the backup's
   manifest records as `app_image`, and the same environment: restore the
   settings from the newest release-time `once backup` archive, or
   re-enter the environment (the five LiveKit values, web-worker count)
   from the sealed ops store. Let ONCE initialize the volume, then stop
   the app:

   ```sh
   sudo once stop chat.smartdata.net
   ```

3. On another machine, verify the chosen backup as above, then copy the
   decrypted `production.sqlite3` and `files/` to the new VM (scp).
4. On the new VM, lay the verified files over the volume exactly as in
   step 4 of [Restore onto the VM](#restore-onto-the-vm) (stopped-container
   discovery, ownership preserved, WAL sidecars moved aside).
5. Start the app and validate like a release cutover: `/up` returns 200,
   recent messages and uploads are present, and the huddle reconciler is
   running.
6. Make the new VM the nightly backup source: update the
   `BACKUP_APP_PROJECT`, `BACKUP_APP_ZONE` and `BACKUP_APP_INSTANCE`
   repository variables to the new VM, re-run `setup-backup-project.sh`
   so the runner's VM-scoped bindings cover the new instance, and
   dispatch the `Nightly backup` workflow once by hand. Nothing is
   installed on the VM. Only then point traffic at the new VM.

## Roll back

Rolling back a *restore* (the restore itself was bad, or the wrong backup
was picked): the pre-restore copies from step 4 above are the way back.

1. If the restored app accepted writes worth keeping, preserve first: take
   a fresh backup as in [Manual backups](#manual-backups) and download it.
   Otherwise those writes are dropped by the rollback.
2. Stop the app and confirm nothing is running (step 3 of
   [Restore onto the VM](#restore-onto-the-vm)).
3. Swap the pre-restore files back over the volume. `cp -a` keeps their
   ownership, which is already the live one. As in the restore, sidecars
   move aside rather than being deleted:

   ```sh
   STAMP=<the pre-restore stamp, from ls /var/backups>
   # (resolve $MOUNT as in step 4 of Restore onto the VM)
   ROLLBACK_STAMP="$(date -u +%Y%m%d-%H%M%S)"
   sudo cp -a "$MOUNT/db/production.sqlite3" "/var/backups/pre-rollback-$ROLLBACK_STAMP.sqlite3"
   [ -f "$MOUNT/db/production.sqlite3-wal" ] && sudo mv "$MOUNT/db/production.sqlite3-wal" "/var/backups/pre-rollback-$ROLLBACK_STAMP.sqlite3-wal"
   [ -f "$MOUNT/db/production.sqlite3-shm" ] && sudo mv "$MOUNT/db/production.sqlite3-shm" "/var/backups/pre-rollback-$ROLLBACK_STAMP.sqlite3-shm"
   sudo cp -a "/var/backups/pre-restore-$STAMP.sqlite3" "$MOUNT/db/production.sqlite3"
   [ -f "/var/backups/pre-restore-$STAMP.sqlite3-wal" ] && sudo cp -a "/var/backups/pre-restore-$STAMP.sqlite3-wal" "$MOUNT/db/production.sqlite3-wal"
   [ -f "/var/backups/pre-restore-$STAMP.sqlite3-shm" ] && sudo cp -a "/var/backups/pre-restore-$STAMP.sqlite3-shm" "$MOUNT/db/production.sqlite3-shm"
   sudo rm -rf "$MOUNT/files"
   sudo cp -a "/var/backups/pre-restore-files-$STAMP" "$MOUNT/files"
   ```

4. Start the app and validate as in step 5 above.
5. Keep the pre-restore copies until the rollback is proven good, then
   delete them: they hold user data.

For rolling back a *release*, see
[deploy/README.md](../deploy/README.md#cutover-and-rollback) instead: that
never restores the database once writes have been accepted.

## Snapshot schedule vs release snapshots

Two different snapshot mechanisms cover two different failures:

| | Release-time snapshot | Nightly schedule |
| --- | --- | --- |
| Taken | once per release, while writes are frozen | nightly at 14:00 UTC, crash-consistent (`default-schedule-1`) |
| Kept | named per release (`campfire-before-<label>-r<run>`); cleaned with the release | 14 days, dropped automatically by the resource policy |
| Covers | the boot disk (the ONCE volume lives under `/var/lib/docker` on it) | the boot disk only; warns when non-boot disks exist |
| Taken by | the deploy workflow's snapshot phase | `default-schedule-1`, already attached to the boot disk |
| Lives in | the app project | the app project |
| Recovers from | a bad deploy (roll back image + frozen DB) | a dead disk, a dead VM, a dead zone |

The boot disk already carries `default-schedule-1` (daily at 14:00 UTC,
14-day retention, snapshots kept when the disk is deleted), and a disk
holds only one schedule. `snapshot-schedule.sh` (run directly or via
`setup-backup-project.sh`) detects that, reports the existing schedule's
settings, and stops without changing anything. Only on a disk with no
schedule at all does it create and attach `smartfire-nightly-boot-disk`
(daily 08:00 UTC, 14-day retention).

Neither replaces the encrypted logical backup in Cloud Storage: snapshots
are disk images in the app project, while the logical backup is portable
(plain files: database, uploads, manifest), integrity-checked, and held in
the separate backup project, so it survives an app-project compromise that
snapshots would not.

## Monthly automated restore check

The `Monthly backup restore check` workflow
(`.github/workflows/backup-restore-check.yml`) proves the latest backup
restores. On the 1st of every month at 09:00 UTC (and on manual dispatch
from main) it:

1. authenticates to the backup project through Workload Identity Federation
   as the read-only `smartfire-backup-reader` (no long-lived keys);
2. downloads the newest object under `daily/` into a scratch runner;
3. decrypts it with the CI-held private key (`BACKUP_RESTORE_AGE_IDENTITY`,
   or `BACKUP_RESTORE_GPG_KEY` for gpg backups), which lives only in a
   GitHub secret and a 0600 file for the duration of the step;
4. runs `deploy/backups/restore-check.sh`: tarball listing, SHA256SUMS,
   `PRAGMA integrity_check`, row counts, and a Rails boot against a
   disposable copy of the database with `PRAGMA query_only`, counting rows
   through the models and then re-verifying the extracted backup is
   byte-identical;
5. gates on `--min-users 1 --min-messages 1`: an empty file passes
   `integrity_check`, so the minimums are what catch an empty or wrong
   database. Tune them if the workspace could legitimately shrink.

Any failure fails the workflow loudly (red run, `::error::` annotation, no
summary page). A green run writes a summary naming the verified object.
Run it by hand once after setup to prove the whole chain, and after any
change to the backup scripts, the lifecycle, or the keys.

## Key management

The age keypair is generated once, off the VM and off any shared machine:

```sh
age-keygen -o smartfire-backup-key.txt   # keep this file sealed
```

- The public recipient (the `age1...` line) goes in the GitHub variable
  `BACKUP_AGE_RECIPIENT`, which the workflow passes to the VM on every
  run. For manual runs it goes in the sourced config instead (see
  [Manual backups](#manual-backups)).
- The private key goes in the GitHub secret `BACKUP_RESTORE_AGE_IDENTITY`
  and in the sealed ops store. It is never copied to the VM, never printed
  by any script (logs carry only counts and checksums), and never committed.

Rotation: generate a new keypair, put the new recipient in the variable
and the new private key in the secret. New backups use the new key; old
backups still need the old private key until they age out — keep it sealed
for 366 days (the monthly retention) after rotating.

The gpg alternative works the same way: the VM holds only the recipient's
public key (`BACKUP_GPG_HOME`), while the private key lives in the
`BACKUP_RESTORE_GPG_KEY` secret for the restore check.

## Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| `campfire-nightly-last.json` older than ~26h | the workflow is not completing | read the failed run's log; work through the rows below |
| `a release holds ...campfire-release.lock` (exit 75) | a release overlaps the run | rerun the workflow after the release finishes |
| `an ONCE operation is in flight` | a release (backup/restore/update) overlaps the run | wait for the release, then dispatch the workflow again |
| `no running ONCE application container found` | the app is stopped (release freeze or outage) | the refusal is deliberate: a stopped app means a half-state. Rerun when healthy |
| `only ... bytes free` | the VM is too full to stage the backup | free space (a run needs ~3x database + uploads; sizes are in `campfire-nightly-last.json`) and rerun |
| `still has the example placeholder` | `BACKUP_AGE_RECIPIENT` was never set | set the variable to the age1... recipient |
| `another backup holds ...lock` | a previous run is still going | wait for it; the workflow serializes runs, so this means a manual run overlaps |
| upload denied (403) | the runner lost its bucket grant | re-run `setup-backup-project.sh`: it re-converges every grant |
| ssh/scp denied | the runner lost its VM-scoped roles, or the VM was replaced | re-run `setup-backup-project.sh` (and update `BACKUP_APP_*` when the VM changed) |
| checksum mismatch after download | a corrupt transfer (or a compromised path) | do not upload it: rerun; investigate when it repeats |
| restore-check `integrity_check` fails | a corrupt or partial backup | stop: verify the previous daily (or newest weekly) instead |
| restore-check below `--min-users`/`--min-messages` | an empty or wrong database | same: do not restore it; investigate which object was picked |
| `could not obtain an access token` in a workflow | the WIF binding does not match the run | dispatch from `main`: only `...:ref:refs/heads/main` is trusted |
| disk pressure on the VM | a run stages ~3x (database + uploads) under `/var/backups` transiently | keep free space above that; stale work dirs older than a day are pruned at start |
