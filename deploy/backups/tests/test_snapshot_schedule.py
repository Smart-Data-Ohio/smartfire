"""Contract tests for deploy/backups/snapshot-schedule.sh.

The lead may run the script directly (not only through
setup-backup-project.sh). A stubbed `gcloud` emulates the GCP boundary; jq is
real except in the missing-jq test, where PATH is scrubbed to prove the
prerequisite error.
"""
from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "deploy/backups/snapshot-schedule.sh"

POLICY_BODY = {
    "snapshotSchedulePolicy": {
        "schedule": {"dailySchedule": {"daysInCycle": 1, "startTime": "14:00"}},
        "retentionPolicy": {"maxRetentionDays": 14, "onSourceDiskDelete": "KEEP_AUTO_SNAPSHOTS"},
    }
}

GCLOUD_STUB = r'''#!/usr/bin/env bash
echo "gcloud $*" >> "$STUB_LOG"
case "$2" in
  resource-policies)
    case "$3" in
      describe)
        # Describing any other schedule (the skip path reports its
        # settings) answers with the canned policy body.
        if [ "$4" != "smartfire-nightly-boot-disk" ] && [ -n "${STUB_POLICY_DESCRIBE_JSON:-}" ]; then
          printf '%s\n' "$STUB_POLICY_DESCRIBE_JSON"
          exit 0
        fi
        if [ "${STUB_POLICY_EXISTS:-0}" = "1" ]; then exit 0; else echo "not found" >&2; exit 1; fi
        ;;
      create) exit 0 ;;
    esac
    ;;
  instances)
    printf '%s' '{"serviceAccounts": [], "disks": [{"boot": true, "source": "https://www.googleapis.com/compute/v1/projects/x/zones/y/disks/campfire"}]}'
    exit 0
    ;;
  disks)
    case "$3" in
      describe)
        if [ -n "${STUB_DISK_JSON:-}" ]; then
          printf '%s\n' "$STUB_DISK_JSON"
        else
          printf '%s' '{"resourcePolicies":[]}'
        fi
        exit 0
        ;;
      add-resource-policies) exit 0 ;;
    esac
    ;;
esac
echo "stub: unhandled: gcloud $*" >&2
exit 9
'''


def to_json(value):
    """Ruby's JSON.generate: compact separators."""
    return json.dumps(value, separators=(",", ":"))


def run_script(env):
    """Open3.capture2e(env, SCRIPT): env merged over the parent environment."""
    result = subprocess.run([str(SCRIPT)], env={**os.environ, **env}, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    return result.stdout, result


class SnapshotScheduleTest(unittest.TestCase):
    def test_creates_the_schedule_and_attaches_it_to_the_boot_disk(self):
        with self.snapshot_env() as (env, dirs):
            out, status = run_script(env)
            self.assertEqual(0, status.returncode, f"snapshot setup failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertTrue(any("resource-policies create snapshot-schedule smartfire-nightly-boot-disk" in line for line in log),
                f"schedule was not created:\n{joined}")
            self.assertTrue(any("add-resource-policies" in line and "campfire" in line for line in log),
                f"schedule was not attached to the boot disk:\n{joined}")

    def test_skips_a_disk_that_already_has_a_schedule_reporting_its_settings(self):
        disk = {"resourcePolicies": [
            "https://www.googleapis.com/compute/v1/projects/test-app-project/regions/us-central1/resourcePolicies/default-schedule-1"
        ]}
        scenario = {
            "STUB_DISK_JSON": to_json(disk),
            "STUB_POLICY_DESCRIBE_JSON": to_json(POLICY_BODY),
        }
        with self.snapshot_env(scenario) as (env, dirs):
            out, status = run_script(env)
            self.assertEqual(0, status.returncode, f"an already-scheduled disk must not fail the script: {out}")
            self.assertIn("default-schedule-1", out)
            self.assertIn("14:00", out)
            self.assertIn("14 days", out)
            self.assertIn("KEEP_AUTO_SNAPSHOTS", out)
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertFalse(any("resource-policies create" in line for line in log),
                f"no new schedule may be created:\n{joined}")
            self.assertFalse(any("add-resource-policies" in line for line in log),
                f"nothing may be attached to an already-scheduled disk:\n{joined}")

    def test_does_nothing_when_our_policy_is_already_attached(self):
        disk = {"resourcePolicies": [
            "https://www.googleapis.com/compute/v1/projects/test-app-project/regions/us-central1/resourcePolicies/smartfire-nightly-boot-disk"
        ]}
        with self.snapshot_env({"STUB_DISK_JSON": to_json(disk)}) as (env, dirs):
            out, status = run_script(env)
            self.assertEqual(0, status.returncode, f"snapshot setup failed: {out}")
            self.assertIn("already attached", out)
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertFalse(any("resource-policies create" in line for line in log),
                f"an attached policy must not be recreated:\n{joined}")
            self.assertFalse(any("add-resource-policies" in line for line in log),
                f"an attached policy must not be re-attached:\n{joined}")

    def test_reports_a_missing_jq_instead_of_failing_mid_run(self):
        with self.snapshot_env({}, scrub_jq=True) as (env, _dirs):
            out, status = run_script(env)
            self.assertNotEqual(0, status.returncode)
            self.assertIn("jq is not installed", out)

    # --- helpers -----------------------------------------------------------

    @contextmanager
    def snapshot_env(self, scenario=None, scrub_jq=False):
        with tempfile.TemporaryDirectory(prefix="smartfire-snapshot-test") as tmp:
            root = Path(tmp)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            stub = bin_dir / "gcloud"
            stub.write_text(GCLOUD_STUB)
            stub.chmod(0o755)
            path = f"{bin_dir}:/usr/bin:/bin"
            if scrub_jq:
                # A PATH with a shell but no jq: #!/usr/bin/env bash still
                # resolves, while command -v jq must fail.
                clean = root / "cleanbin"
                clean.mkdir()
                (clean / "bash").symlink_to("/bin/bash")
                (clean / "sh").symlink_to("/bin/bash")
                path = f"{bin_dir}:{clean}"
            env = {
                "PATH": path,
                "STUB_LOG": str(root / "calls.log"),
                "APP_PROJECT_ID": "test-app-project",
                **(scenario or {}),
            }
            yield env, root

    def gcloud_log(self, dirs):
        return [line.strip() for line in (dirs / "calls.log").read_text().splitlines(keepends=True)]


if __name__ == "__main__":
    unittest.main()
