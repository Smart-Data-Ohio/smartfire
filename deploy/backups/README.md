# Smartfire nightly backups

Rolling daily backups to Cloud Storage in a separate GCP project, driven by
the `Nightly backup` GitHub Actions workflow. The full runbook is
[docs/backups.md](../../docs/backups.md); this file is the on-disk index.

| File | Purpose |
| --- | --- |
| `campfire-backup.sh` | produces one encrypted archive on the VM (run by the workflow over IAP SSH, or by hand with `--output-dir`) |
| `backup.env.example` | config template for manual runs (the workflow needs none of this) |
| `lifecycle.json` | bucket lifecycle: 7 daily, 4 weekly, 12 monthly, 30-day versions |
| `setup-backup-project.sh` | one-time infra setup; the lead runs it |
| `snapshot-schedule.sh` | nightly boot-disk snapshot schedule, 14-day retention (skips when the disk already has one) |
| `restore-check.sh` | decrypt-and-verify; shared by the monthly workflow and operators |

The workflows are
[.github/workflows/nightly-backup.yml](../../.github/workflows/nightly-backup.yml)
and
[.github/workflows/backup-restore-check.yml](../../.github/workflows/backup-restore-check.yml).
The backup script's contract is pinned by `tests/test_campfire_backup.py`,
which runs it against a temp database with a stubbed `df`. The other scripts,
both workflows and the runbook have contract tests beside it in `tests/`
(stdlib only; the age cases skip when `age` is not on PATH). Run them from the
repo root:

```sh
python3 -m unittest discover -s deploy/backups/tests -p 'test_*.py' -v
```
