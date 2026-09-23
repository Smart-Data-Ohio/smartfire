# Smartfire nightly backups

Rolling daily backups to Cloud Storage in a separate GCP project. The full
runbook is [docs/backups.md](../../docs/backups.md); this file is the
on-disk index.

| File | Purpose |
| --- | --- |
| `campfire-backup.sh` | the nightly backup run (systemd timer, on the VM host) |
| `campfire-backup.service` / `.timer` | systemd unit and schedule (09:00 host time daily) |
| `backup.env.example` | VM config template, installed as `/etc/campfire-backups/backup.env` |
| `install-backup.sh` | idempotent VM installer (`sudo deploy/backups/install-backup.sh`) |
| `lifecycle.json` | bucket lifecycle: 7 daily, 4 weekly, 12 monthly, 30-day versions |
| `setup-backup-project.sh` | one-time infra setup; the lead runs it |
| `snapshot-schedule.sh` | nightly boot-disk snapshot schedule, 14-day retention |
| `restore-check.sh` | decrypt-and-verify; shared by the monthly workflow and operators |

The monthly check is [.github/workflows/backup-restore-check.yml](../../.github/workflows/backup-restore-check.yml).
The backup script's contract is pinned by `test/backups/campfire_backup_test.rb`,
which runs it against a temp database with a stubbed `gcloud`.
