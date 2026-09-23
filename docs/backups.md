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
  manifest.json        non-secret facts: stamp, host, image ref, env key NAMES,
                       SHA-256 of the database, file counts (never secret values)
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

Backups go to Cloud Storage in a **separate GCP project** so that a
compromised app project, or an account that administers it, cannot delete
the backups. Exactly two service accounts exist in the backup project:

| Account | Granted on the backup bucket | Used by |
| --- | --- | --- |
| `smartfire-backup-writer@<backup-project>` | `roles/storage.objectCreator` only | the VM's nightly upload (attached as the VM service account) |
| `smartfire-backup-reader@<backup-project>` | `roles/storage.objectViewer` only | the monthly restore-check workflow via Workload Identity Federation |

Neither can delete or overwrite objects. The bucket additionally has Object
Versioning and 30-day soft delete, so even a backup-project admin's delete
is recoverable: a deleted live object survives as a noncurrent version for
30 days, and soft-deleted objects are recoverable for 30 days. This is why
the lifecycle keeps a `daysSinceNoncurrentTime` rule rather than letting
versions accumulate forever.

The VM was previously service-account-less (see
[the release pipeline](../deploy/gcp/README.md#authentication)). Attaching
the writer SA is a deliberate, minimal exception: the cloud-platform scope
on the VM is only the coarse gate, and the bucket IAM binding is what
limits the VM to creating backup objects. No keys are ever copied onto the
VM.

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

Prerequisites: `gcloud` authenticated as someone who can administer the new
backup project, and this repository checked out.

1. Pick the backup project id and bucket name, then run the setup script.
   It creates the bucket, both service accounts and their bindings, the
   Workload Identity pool for the restore check, and the snapshot schedule,
   and prints every value to configure next. Project creation and billing
   linking stay manual: if the project does not exist the script prints the
   commands and stops.

   ```sh
   BACKUP_PROJECT_ID=smartfire-backups-xxx BACKUP_BUCKET=smartfire-backups-xxx \
     bash deploy/backups/setup-backup-project.sh
   ```

2. Follow the manual steps it prints: generate the age keypair off the VM,
   attach the writer SA to the app VM (brief downtime), install the timer
   on the VM, set the four GitHub repository variables and the private-key
   secret, and run the restore-check workflow once by hand.

## On the VM

Install or update (idempotent; never overwrites the config):

```sh
sudo deploy/backups/install-backup.sh
sudoedit /etc/campfire-backups/backup.env
sudo systemctl start campfire-backup.service   # one backup now
sudo journalctl -u campfire-backup.service --since '10 min ago'
sudo systemctl list-timers campfire-backup.timer
```

Each run also writes a non-secret summary to
`/var/backups/campfire-nightly-last.json` (stamp, objects, SHA-256, sizes).

## List the backups

From any machine authenticated as the backup reader (or a backup admin):

```sh
BUCKET=smartfire-backups-xxx
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

4. Find the storage volume and copy the verified files over it. Keep a
   timestamped copy of what was there, in case the restore itself is bad:

   ```sh
   VOLUME="$(sudo docker volume ls --format '{{.Name}}' | grep . | head -n 1)"
   # Prefer the exact volume: inspect the stopped container for /rails/storage.
   MOUNT="$(sudo docker volume inspect "$VOLUME" --format '{{.Mountpoint}}')"
   sudo cp "$MOUNT/db/production.sqlite3" "/var/backups/pre-restore-$(date -u +%Y%m%d-%H%M%S).sqlite3"
   sudo install -m 0644 -o root -g root production.sqlite3 "$MOUNT/db/production.sqlite3"
   sudo rm -f "$MOUNT/db/production.sqlite3-wal" "$MOUNT/db/production.sqlite3-shm"
   sudo rm -rf "$MOUNT/files"
   sudo cp -a files "$MOUNT/files"
   ```

   Remove any `-wal`/`-shm` sidecars: the snapshot is self-contained, and a
   stale WAL from the previous database would corrupt the restored one.

5. Start the app and validate like a release cutover: `/up` returns 200,
   recent messages and uploads are present, the signing keys are unchanged
   (the restore does not touch ONCE settings), and the huddle reconciler
   process is running. See [deploy/README.md](../deploy/README.md#cutover-and-rollback)
   for the checklist.
6. Resume normal timers. Record what was restored, from which object, and
   which writes were dropped.

## Restore onto a fresh VM

For a new VM (disaster recovery, or rehe
...[truncated 3801 chars]