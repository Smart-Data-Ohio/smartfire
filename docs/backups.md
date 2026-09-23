# Smartfire backups

Rolling daily backups of the app VM, stored encrypted in Cloud Storage in a
**separate GCP project**, plus a nightly boot-disk snapshot schedule. The
whole chain is: a systemd timer on the VM takes a consistent backup every
night without stopping the app, lifecycle rules age out old prefixes, and a
monthly GitHub Actions workflow proves the latest backup restores.

## Contents

- [Architecture](#architecture)
- [Identities and least privilege](#identities-and-least-privilege)
- [Retention](#retention)
- [Setup (lead only, once)](#setup-lead-only-once)
- [On the VM](#on-the-vm)
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

Every night at 09:00 host time (UTC on GCE) the `campfire-backup.timer`
systemd timer runs `campfire-backup.sh` on the VM **host**, not inside the
app container. One run produces one object:

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

On Sundays the same file is additionally uploaded under `weekly/`, and on
the 1st of the month under `monthly/`. Each prefix is a separate upload of
the same local bytes, not a server-side copy: the uploader credential is
create-only and cannot read anything back to copy it.

The scripts, units and lifecycle config live in [deploy/backups/](../deploy/backups/):

| File | Purpose |
| --- | --- |
| `campfire-backup.sh` | the nightly backup run |
| `campfire-backup.service` / `.timer` | systemd unit and schedule |
| `backup.env.example` | VM config template (`/etc/campfire-backups/backup.env`) |
| `install-backup.sh` | idempotent VM installer |
| `lifecycle.json` | bucket lifecycle rules |
| `setup-backup-project.sh` | one-time infra setup (the lead runs it) |
| `snapshot-schedule.sh` | nightly boot-disk snapshot schedule |
| `restore-check.sh` | decrypt-and-verify, shared by CI and operators |

## Identities and least privilege

Backups go to Cloud Storage in a **separate GCP project**
(`smart-data-campfire-backups`) so that a compromised app project, or an
account that administers it, cannot delete the backups.

| Account | Granted on the backup bucket | Used by |
| --- | --- | --- |
| the app VM's own service account | `roles/storage.objectCreator` only (cross-project, bucket-level) | the VM's nightly upload |
| `smartfire-backup-reader@smart-data-campfire-backups` | `roles/storage.objectViewer` only | the monthly restore-check workflow via Workload Identity Federation |

Neither grant can delete or overwrite objects: the uploader can only create
new objects, and the reader can only read them. The bucket additionally has
Object Versioning and 30-day soft delete, so even a backup-project admin's
delete is recoverable: a deleted live object survives as a noncurrent
version for 30 days, and soft-deleted objects are recoverable for 30 days.
This is why the lifecycle keeps a `daysSinceNoncurrentTime` rule rather
than letting versions accumulate forever.

No new identity is attached to the VM and the VM is never stopped for
backups: the uploader grant is a cross-project IAM binding on the backup
bucket itself. `setup-backup-project.sh` reads the VM's service account and
access scopes first and proceeds only when the VM can already upload. When
the VM has no service account, or its scopes cannot write to Cloud Storage,
the script instead stages a dedicated `smartfire-backup-writer` service
account in the backup project, grants *that*, prints the one-time attach
steps (one brief stop), and exits 1 until it is attached. Attaching a
service account or widening scopes is never done silently. No keys are ever
copied onto the VM either way.

The monthly workflow impersonates the reader through Workload Identity
Federation with no long-lived keys. The binding pins the exact ID-qualified
subject GitHub mints for runs of `Smart-Data-Ohio/smartfire` on
`refs/heads/main` (the same form as the deployer's bindings), so dispatch
the workflow by hand from main.

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
backup project and view the app project, and this repository checked out.
The backup project `smart-data-campfire-backups` already exists with
billing linked and the storage and IAM APIs enabled.

1. Pick the bucket name and run the setup script. The project, app VM and
   repository default to the real values; only the bucket is required. It
   creates the bucket, the uploader grant, the reader identity with its
   Workload Identity pool, and the snapshot schedule, and prints every
   value to configure next. Project creation and billing linking stay
   manual: if the project does not exist the script prints the commands
   and stops.

   ```sh
   BACKUP_BUCKET=smartfire-backups-xxx \
     bash deploy/backups/setup-backup-project.sh
   ```

   If the script exits 1 with attach steps, the app VM cannot upload as it
   stands: perform the one-time attach it prints, then re-run it. It
   converges to exit 0.

2. Follow the manual steps it prints: generate the age keypair off the VM,
   install the timer on the VM, set the four GitHub repository variables
   and the private-key secret, and run the restore-check workflow once by
   hand from main.

## On the VM

Install or update (idempotent; never overwrites the config; installs age
itself when the configured encryption needs it):

```sh
sudo deploy/backups/install-backup.sh
sudoedit /etc/campfire-backups/backup.env
sudo systemctl start campfire-backup.service   # one backup now
sudo journalctl -u campfire-backup.service --since '10 min ago'
sudo systemctl list-timers campfire-backup.timer
```

Each run also writes a non-secret summary to
`/var/backups/campfire-nightly-last.json` (stamp, objects, SHA-256, sizes).
Alert if that file goes stale: it is the cheapest proof the timer is alive
(see [Troubleshooting](#troubleshooting)).

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
   if they matter (take a fresh backup with
   `sudo systemctl start campfire-backup.service` and download it).
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
   sudo install -m 0644 production.sqlite3 "$MOUNT/db/production.sqlite3"
   sudo chown --reference="/var/backups/pre-restore-$STAMP.sqlite3" "$MOUNT/db/production.sqlite3"
   sudo rm -f "$MOUNT/db/production.sqlite3-wal" "$MOUNT/db/production.sqlite3-shm"
   sudo mv "$MOUNT/files" "/var/backups/pre-restore-files-$STAMP"
   sudo cp -a files "$MOUNT/files"
   sudo chown -R --reference="/var/backups/pre-restore-files-$STAMP" "$MOUNT/files"
   ```

   Two details matter here. First, remove any `-wal`/`-shm` sidecars: the
   snapshot is self-contained, and a stale WAL from the previous database
   would corrupt the restored one. Second, the app container runs as UID
   1000, not root, so every restored file must keep the live ownership
   (the `chown --reference` lines): root-owned files would leave the app
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
   discovery, ownership preserved, WAL sidecars removed).
5. Start the app and validate like a release cutover: `/up` returns 200,
   recent messages and uploads are present, and the huddle reconciler is
   running.
6. Make the new VM the nightly uploader: install the backup timer on it
   ([On the VM](#on-the-vm)) and, when the VM's service account differs
   from the old one, re-run `setup-backup-project.sh` so the uploader
   grant covers the new identity. Only then point traffic at the new VM.

## Roll back

Rolling back a *restore* (the restore itself was bad, or the wrong backup
was picked): the pre-restore copies from step 4 above are the way back.

1. If the restored app accepted writes worth keeping, preserve first: take
   a fresh backup with `sudo systemctl start campfire-backup.service` and
   download it. Otherwise those writes are dropped by the rollback.
2. Stop the app and confirm nothing is running (step 3 of
   [Restore onto the VM](#restore-onto-the-vm)).
3. Swap the pre-restore files back over the volume. `cp -a` keeps their
   ownership, which is already the live one:

   ```sh
   STAMP=<the pre-restore stamp, from ls /var/backups>
   # (resolve $MOUNT as in step 4 of Restore onto the VM)
   sudo cp -a "/var/backups/pre-restore-$STAMP.sqlite3" "$MOUNT/db/production.sqlite3"
   sudo rm -f "$MOUNT/db/production.sqlite3-wal" "$MOUNT/db/production.sqlite3-shm"
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
| Taken | once per release, while writes are frozen | nightly at 08:00 UTC, crash-consistent |
| Kept | named per release (`campfire-before-<label>-r<run>`); cleaned with the release | 14 days, dropped automatically by the resource policy |
| Covers | the boot disk (the ONCE volume lives under `/var/lib/docker` on it) | the boot disk only; warns when non-boot disks exist |
| Taken by | the deploy workflow's snapshot phase | `snapshot-schedule.sh` (via `setup-backup-project.sh`) |
| Lives in | the app project | the app project |
| Recovers from | a bad deploy (roll back image + frozen DB) | a dead disk, a dead VM, a dead zone |

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

- The public recipient (the `age1...` line) goes in
  `/etc/campfire-backups/backup.env` on the VM as `BACKUP_AGE_RECIPIENT`.
- The private key goes in the GitHub secret `BACKUP_RESTORE_AGE_IDENTITY`
  and in the sealed ops store. It is never copied to the VM, never printed
  by any script (logs carry only counts and checksums), and never committed.

Rotation: generate a new keypair, put the new recipient in `backup.env`
and the new private key in the secret. New backups use the new key; old
backups still need the old private key until they age out — keep it sealed
for 366 days (the monthly retention) after rotating.

The gpg alternative works the same way: the VM holds only the recipient's
public key (`BACKUP_GPG_HOME`), while the private key lives in the
`BACKUP_RESTORE_GPG_KEY` secret for the restore check.

## Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| `campfire-nightly-last.json` older than ~26h | the timer is not completing | `journalctl -u campfire-backup.service`; work through the rows below |
| `an ONCE operation is in flight` | a release (backup/restore/update) overlaps the run | wait for the release, then `sudo systemctl start campfire-backup.service` |
| `no running ONCE application container found` | the app is stopped (release freeze or outage) | the refusal is deliberate: a stopped app means a half-state. Rerun when healthy |
| `still has the example placeholder` | `backup.env` was never edited | `sudoedit /etc/campfire-backups/backup.env` (the guard fails before snapshotting) |
| `another backup holds ...lock` | a previous run is still going, or died holding the lock | wait; if no backup process runs, remove `/var/lock/campfire-backup.lock` and rerun |
| upload denied (403) | the VM identity lost its grant, or a new VM was swapped in | re-run `setup-backup-project.sh`: the VM lookup diagnoses SA and scopes |
| `gcloud: command not found` on the VM | no upload tool on the host | install the Cloud SDK; the installer refuses to proceed without it |
| restore-check `integrity_check` fails | a corrupt or partial backup | stop: verify the previous daily (or newest weekly) instead |
| restore-check below `--min-users`/`--min-messages` | an empty or wrong database | same: do not restore it; investigate which object was picked |
| `could not obtain an access token` in the workflow | the WIF binding does not match the run | dispatch from `main`: only `...:ref:refs/heads/main` is trusted |
| disk pressure on the VM | a run stages ~3x (database + uploads) under `/var/backups` transiently | keep free space above that; sizes are in `campfire-nightly-last.json` |
