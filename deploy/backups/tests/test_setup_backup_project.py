"""Contract tests for deploy/backups/setup-backup-project.sh.

Each test runs the real script with a stubbed `gcloud` on PATH that emulates
the GCP boundary (project/bucket/IAM/WIF/snapshot state via STUB_* variables)
and records every invocation. Only the cloud boundary is stubbed; jq and the
delegated snapshot-schedule.sh are real.
"""
from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "deploy/backups/setup-backup-project.sh"
RUNNER = "smartfire-backup-runner@smart-data-campfire.iam.gserviceaccount.com"
READER = "smartfire-backup-reader@smart-data-campfire-backups.iam.gserviceaccount.com"
SUBJECT = "repo:Smart-Data-Ohio@262436228/smartfire@1370426325:ref:refs/heads/main"
PROJECT = "smart-data-campfire-backups"
APP_PROJECT = "smart-data-campfire"
UPLOADER_ROLE = "projects/smart-data-campfire-backups/roles/backupUploader"
UPLOADER_PERMISSIONS = "storage.objects.create,storage.objects.get,storage.objects.list"
APP_POOL_PRINCIPAL = ("principal://iam.googleapis.com/projects/112233445566/locations/global"
    f"/workloadIdentityPools/github/subject/{SUBJECT}")

GCLOUD_STUB = r'''#!/usr/bin/env bash
# Emulates just enough gcloud for setup-backup-project.sh. Scenario state
# comes from STUB_* variables; every invocation is appended to $STUB_LOG.
echo "gcloud $*" >> "$STUB_LOG"
: "${BACKUP_PROJECT_ID:=smart-data-campfire-backups}"
case "$1" in
  projects)
    if [[ "$*" == *"$BACKUP_PROJECT_ID"* ]] && [ "${STUB_PROJECT_EXISTS:-1}" != "1" ]; then
      echo "ERROR: project not found" >&2
      exit 1
    fi
    case "$*" in
      *projectNumber*)
        if [[ "$*" == *"$BACKUP_PROJECT_ID"* ]]; then
          echo "${STUB_PROJECT_NUMBER:-922766272284}"
        else
          echo "${STUB_APP_PROJECT_NUMBER:-112233445566}"
        fi
        ;;
      *get-iam-policy*)
        if [ -n "${STUB_PROJECT_POLICY_JSON:-}" ]; then
          printf '%s\n' "$STUB_PROJECT_POLICY_JSON"
        else
          printf '%s\n' '{"bindings":[]}'
        fi
        ;;
    esac
    exit 0
    ;;
  services)
    exit 0
    ;;
  storage)
    case "$3" in
      describe)
        if [ "${STUB_BUCKET_EXISTS:-0}" = "1" ]; then exit 0; else echo "not found" >&2; exit 1; fi
        ;;
      create|update)
        exit 0
        ;;
      get-iam-policy)
        # NB: no :-default with braces here: a } inside ${var:-...}
        # ends the expansion early and leaks a literal }.
        if [ -n "${STUB_BUCKET_POLICY_JSON:-}" ]; then
          printf '%s\n' "$STUB_BUCKET_POLICY_JSON"
        else
          printf '%s\n' '{"bindings":[]}'
        fi
        exit 0
        ;;
      add-iam-policy-binding)
        if [ "${STUB_BIND_HARD_FAIL:-0}" = "1" ]; then
          echo "ERROR: (gcloud.storage.buckets.add-iam-policy-binding) PERMISSION_DENIED: permission denied" >&2
          exit 1
        fi
        if [ "${STUB_BIND_FAIL_ALWAYS:-0}" = "1" ]; then
          echo "ERROR: (gcloud.storage.buckets.add-iam-policy-binding) FAILED_PRECONDITION: role does not exist in the resource's hierarchy." >&2
          exit 1
        fi
        if [ "${STUB_BIND_FAIL_ONCE:-0}" = "1" ] && [ ! -f "${STUB_LOG}.bind_failed" ]; then
          touch "${STUB_LOG}.bind_failed"
          echo "ERROR: (gcloud.storage.buckets.add-iam-policy-binding) FAILED_PRECONDITION: role does not exist in the resource's hierarchy." >&2
          exit 1
        fi
        exit 0
        ;;
      *)
        echo "stub: unhandled storage $3" >&2; exit 9
        ;;
    esac
    ;;
  iam)
    case "$2" in
      service-accounts)
        case "$3" in
          describe)
            case "$4" in
              smartfire-backup-runner@*)
                [ "${STUB_RUNNER_EXISTS:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
                ;;
              *)
                [ "${STUB_READER_EXISTS:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
                ;;
            esac
            ;;
          create)
            exit 0
            ;;
          get-iam-policy)
            if [ -n "${STUB_SA_POLICY_JSON:-}" ]; then
              printf '%s\n' "$STUB_SA_POLICY_JSON"
            else
              printf '%s\n' '{"bindings":[]}'
            fi
            exit 0
            ;;
          add-iam-policy-binding)
            exit 0
            ;;
        esac
        ;;
      roles)
        case "$3" in
          describe)
            [ "${STUB_ROLE_EXISTS:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
            ;;
          create|update)
            exit 0
            ;;
        esac
        ;;
      workload-identity-pools)
        # Pool/provider existence is per project. Both identities bind
        # subjects from the app project's pool; the backup-project
        # branch only exists to catch a regressed second pool.
        pool_var="STUB_POOL_EXISTS"
        provider_var="STUB_PROVIDER_EXISTS"
        if [[ "$*" != *"$BACKUP_PROJECT_ID"* ]]; then
          pool_var="STUB_APP_POOL_EXISTS"
          provider_var="STUB_APP_PROVIDER_EXISTS"
        fi
        case "$3" in
          describe)
            [ "${!pool_var:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
            ;;
          create)
            exit 0
            ;;
          providers)
            case "$4" in
              describe)
                [ "${!provider_var:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
                ;;
              create-oidc)
                exit 0
                ;;
            esac
            ;;
        esac
        ;;
    esac
    ;;
  compute)
    case "$2" in
      instances)
        case "$3" in
          describe)
            # One JSON body serves the snapshot schedule's boot-disk
            # discovery. The setup itself grants nothing on the VM:
            # IAP tunnel access is not granted through instance IAM,
            # so the runner's roles are project-level bindings. Any
            # instances IAM call below falls through to "unhandled"
            # and fails the run loudly.
            disk='{"boot": true, "source": "https://www.googleapis.com/compute/v1/projects/x/zones/y/disks/campfire"}'
            printf '{"serviceAccounts": [], "disks": [%s]}\n' "$disk"
            exit 0
            ;;
          *)
            echo "stub: unhandled instances $3" >&2; exit 9
            ;;
        esac
        ;;
      resource-policies)
        case "$3" in
          describe)
            # Describing any other schedule (the skip path reports its
            # settings) answers with the canned policy body.
            if [ "$4" != "smartfire-nightly-boot-disk" ] && [ -n "${STUB_POLICY_DESCRIBE_JSON:-}" ]; then
              printf '%s\n' "$STUB_POLICY_DESCRIBE_JSON"
              exit 0
            fi
            [ "${STUB_POLICY_EXISTS:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
            ;;
          create)
            exit 0
            ;;
        esac
        ;;
      disks)
        case "$3" in
          describe)
            if [ -n "${STUB_DISK_JSON:-}" ]; then
              printf '%s\n' "$STUB_DISK_JSON"
            else
              printf '%s\n' '{"resourcePolicies":[]}'
            fi
            exit 0
            ;;
          add-resource-policies)
            exit 0
            ;;
        esac
        ;;
    esac
    ;;
esac
echo "stub: unhandled: gcloud $*" >&2
exit 9
'''


def to_json(value):
    """Ruby's JSON.generate: compact separators."""
    return json.dumps(value, separators=(",", ":"))


class SetupBackupProjectTest(unittest.TestCase):
    def test_creates_the_backup_runner_in_the_app_project_with_the_custom_uploader_grant(self):
        with self.setup_env() as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertTrue(any("service-accounts create smartfire-backup-runner" in line and f"--project={APP_PROJECT}" in line for line in log),
                f"runner SA was not created in the app project:\n{joined}")
            self.assertTrue(any("roles create backupUploader" in line and f"--project={PROJECT}" in line and f"--permissions={UPLOADER_PERMISSIONS}" in line for line in log),
                f"custom uploader role was not created with exactly create+get+list:\n{joined}")
            self.assertTrue(any("add-iam-policy-binding" in line and UPLOADER_ROLE in line and f"serviceAccount:{RUNNER}" in line for line in log),
                f"runner was not granted the custom uploader role:\n{joined}")
            self.assertFalse(any("roles/storage.objectCreator" in line for line in log),
                f"objectCreator alone fails the upload with a 403 and must not be granted:\n{joined}")
            self.assertFalse(any("service-accounts create" in line and "smartfire-backup-writer" in line for line in log),
                f"no writer service account must be created:\n{joined}")
            self.assertNotIn("instances stop", out, f"no VM stop may be prescribed:\n{out}")
            self.assertIn("No VM change was needed", out)

    def test_converges_the_custom_uploader_role_when_it_already_exists(self):
        with self.setup_env({"STUB_ROLE_EXISTS": "1"}) as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertTrue(any("roles update backupUploader" in line and f"--project={PROJECT}" in line and f"--permissions={UPLOADER_PERMISSIONS}" in line for line in log),
                f"existing uploader role was not converged to exactly create+get+list:\n{joined}")
            self.assertFalse(any("roles create backupUploader" in line for line in log),
                f"an existing role must be updated, not recreated:\n{joined}")

    def test_retries_the_bucket_binding_while_the_custom_role_propagates(self):
        scenario = {"STUB_BIND_FAIL_ONCE": "1", "BACKUP_BIND_RETRY_SLEEP": "0"}
        with self.setup_env(scenario) as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            self.assertIn("not yet propagated", out)
            log = self.gcloud_log(dirs)
            binds = [line for line in log if "buckets add-iam-policy-binding" in line and UPLOADER_ROLE in line]
            self.assertEqual(2, len(binds),
                "the binding was not retried after the propagation failure:\n" + "\n".join(log))

    def test_gives_up_the_bucket_binding_once_the_retry_budget_is_spent(self):
        scenario = {"STUB_BIND_FAIL_ALWAYS": "1", "BACKUP_BIND_RETRY_SECONDS": "0", "BACKUP_BIND_RETRY_SLEEP": "0"}
        with self.setup_env(scenario) as (env, _dirs):
            out, status = self.run_setup(env)
            self.assertNotEqual(0, status.returncode, f"setup should have failed: {out}")
            self.assertIn(f"could not bind {UPLOADER_ROLE}", out)

    def test_aborts_the_bucket_binding_on_any_other_failure(self):
        scenario = {"STUB_BIND_HARD_FAIL": "1", "BACKUP_BIND_RETRY_SECONDS": "60"}
        with self.setup_env(scenario) as (env, dirs):
            out, status = self.run_setup(env)
            self.assertNotEqual(0, status.returncode, f"setup should have failed: {out}")
            self.assertIn(f"could not bind {UPLOADER_ROLE}", out)
            log = self.gcloud_log(dirs)
            binds = [line for line in log if "buckets add-iam-policy-binding" in line and UPLOADER_ROLE in line]
            self.assertEqual(1, len(binds), "a hard failure must not be retried:\n" + "\n".join(log))

    def test_creates_the_bucket_in_the_us_multi_region(self):
        with self.setup_env() as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            self.assertTrue(any("storage buckets create" in line and "--location=US" in line for line in log),
                "bucket was not created in US:\n" + "\n".join(log))

    def test_grants_the_runner_the_deployers_project_level_roles_in_the_app_project(self):
        with self.setup_env() as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            for role in [
                "roles/iap.tunnelResourceAccessor",
                "roles/compute.osAdminLogin",
                "roles/compute.viewer",
            ]:
                self.assertTrue(any(
                    f"projects add-iam-policy-binding {APP_PROJECT}" in line
                    and f"--role={role}" in line and f"serviceAccount:{RUNNER}" in line
                    for line in log), f"runner was not granted {role} on the app project:\n{joined}")
            self.assertFalse(any("compute instances add-iam-policy-binding" in line for line in log),
                f"IAP tunnel access is not granted through instance IAM:\n{joined}")
            self.assertFalse(any("set-service-account" in line for line in log),
                f"nothing may be attached to the VM:\n{joined}")
            self.assertFalse(any("roles/compute.instanceAdmin" in line or "roles/compute.storageAdmin" in line for line in log),
                f"the runner must hold no deploy rights:\n{joined}")

    def test_binds_the_runner_with_the_id_qualified_subject_from_the_existing_pool(self):
        with self.setup_env() as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            principal = APP_POOL_PRINCIPAL
            self.assertTrue(any(
                f"service-accounts add-iam-policy-binding {RUNNER}" in line
                and "roles/iam.workloadIdentityUser" in line and principal in line
                for line in log), f"runner was not bound to the ID-qualified subject:\n{joined}")
            self.assertTrue(any("workload-identity-pools providers create-oidc github-oidc" in line and f"--project={APP_PROJECT}" in line for line in log),
                f"no WIF provider was created in the app project:\n{joined}")
            self.assertFalse(any("principalSet" in line for line in log),
                f"must not use attribute-based bindings:\n{joined}")

    def test_creates_no_pool_in_the_backup_project_by_default(self):
        with self.setup_env() as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertTrue(any("workload-identity-pools create github" in line and f"--project={APP_PROJECT}" in line for line in log),
                f"missing pool was not created in the app project:\n{joined}")
            self.assertFalse(any("workload-identity-pools" in line and f"--project={PROJECT}" in line for line in log),
                f"no second pool may be created in the backup project:\n{joined}")

    def test_keeps_the_reader_identity_for_the_restore_check(self):
        with self.setup_env() as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertTrue(any("service-accounts create smartfire-backup-reader" in line and f"--project={PROJECT}" in line for line in log),
                f"reader SA was not created in the backup project:\n{joined}")
            self.assertTrue(any("roles/storage.objectViewer" in line and f"serviceAccount:{READER}" in line for line in log),
                f"reader was not granted objectViewer:\n{joined}")
            # The reader SA lives in the backup project, but its principal comes
            # from the app project's pool (the app project number).
            principal = APP_POOL_PRINCIPAL
            self.assertTrue(any(
                f"service-accounts add-iam-policy-binding {READER}" in line
                and f"--project={PROJECT}" in line
                and "roles/iam.workloadIdentityUser" in line and principal in line
                for line in log), f"reader was not bound to the ID-qualified subject:\n{joined}")

    def test_does_not_enable_the_compute_api_in_the_backup_project(self):
        with self.setup_env() as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            backup_enable = [line for line in log if "services enable" in line and f"--project={PROJECT}" in line]
            self.assertTrue(backup_enable, "no API enablement ran for the backup project")
            for line in backup_enable:
                self.assertNotIn("compute.googleapis.com", line,
                    f"compute must not be enabled in the backup project:\n{line}")
            self.assertTrue(any("services enable" in line and f"--project={APP_PROJECT}" in line for line in log),
                "APIs were not enabled in the app project:\n" + "\n".join(log))

    def test_delegates_the_snapshot_schedule(self):
        with self.setup_env() as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertTrue(any("instances describe campfire" in line and f"--project={APP_PROJECT}" in line for line in log),
                f"the snapshot schedule was not delegated:\n{joined}")
            self.assertTrue(any("resource-policies create snapshot-schedule smartfire-nightly-boot-disk" in line for line in log),
                f"the schedule was not created on an unscheduled disk:\n{joined}")

    def test_skips_the_snapshot_schedule_when_the_disk_already_has_one(self):
        disk = {"resourcePolicies": [
            f"https://www.googleapis.com/compute/v1/projects/{APP_PROJECT}/regions/us-central1/resourcePolicies/default-schedule-1"
        ]}
        policy = {
            "snapshotSchedulePolicy": {
                "schedule": {"dailySchedule": {"daysInCycle": 1, "startTime": "14:00"}},
                "retentionPolicy": {"maxRetentionDays": 14, "onSourceDiskDelete": "KEEP_AUTO_SNAPSHOTS"},
            }
        }
        scenario = {
            "STUB_DISK_JSON": to_json(disk),
            "STUB_POLICY_DESCRIBE_JSON": to_json(policy),
        }
        with self.setup_env(scenario) as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            self.assertIn("default-schedule-1", out)
            self.assertIn("14:00", out)
            self.assertIn("14 days", out)
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertFalse(any("resource-policies create" in line for line in log),
                f"no new schedule may be created:\n{joined}")
            self.assertFalse(any("add-resource-policies" in line for line in log),
                f"nothing may be attached to an already-scheduled disk:\n{joined}")

    def test_defaults_to_the_real_backup_project_app_project_and_vm(self):
        with self.setup_env() as (env, _dirs):
            env = {k: v for k, v in env.items() if k not in ("BACKUP_PROJECT_ID", "APP_PROJECT_ID")}
            out, status = self.run_setup(env, unset=("BACKUP_PROJECT_ID", "APP_PROJECT_ID"))
            self.assertEqual(0, status.returncode, f"setup failed without project ids: {out}")

        with self.setup_env() as (env, dirs):
            env = {k: v for k, v in env.items() if k not in ("BACKUP_PROJECT_ID", "APP_PROJECT_ID")}
            _out, _status = self.run_setup(env, unset=("BACKUP_PROJECT_ID", "APP_PROJECT_ID"))
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertTrue(any(f"projects describe {PROJECT}" in line for line in log),
                f"did not use the real backup project by default:\n{joined}")
            self.assertTrue(any(f"projects add-iam-policy-binding {APP_PROJECT}" in line for line in log),
                f"did not bind the real app project by default:\n{joined}")
            self.assertTrue(any("instances describe campfire" in line and f"--project={APP_PROJECT}" in line and "--zone=us-central1-a" in line for line in log),
                f"did not use the real app VM by default:\n{joined}")

    def test_prints_manual_project_steps_when_the_project_does_not_exist(self):
        with self.setup_env({"STUB_PROJECT_EXISTS": "0"}) as (env, _dirs):
            out, status = self.run_setup(env)
            self.assertNotEqual(0, status.returncode)
            self.assertIn("gcloud projects create", out)
            self.assertIn("billing", out)
            self.assertNotIn("compute.googleapis.com", out, "the manual steps must not enable compute either")

    def test_prints_the_github_variables_to_set(self):
        with self.setup_env() as (env, _dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"setup failed: {out}")
            for name in [
                "BACKUP_GCP_BUCKET", "BACKUP_APP_PROJECT", "BACKUP_APP_ZONE", "BACKUP_APP_INSTANCE",
                "BACKUP_RUNNER_WIF_PROVIDER", "BACKUP_RUNNER_SA", "BACKUP_AGE_RECIPIENT",
                "BACKUP_GCP_PROJECT_ID", "BACKUP_GCP_WIF_PROVIDER", "BACKUP_GCP_READER_SA",
                "BACKUP_RESTORE_AGE_IDENTITY",
            ]:
                self.assertIn(name, out, f"missing from the printed variables: {name}")
            self.assertIn(RUNNER, out)
            # Both identities share the app project's existing provider.
            provider = "projects/112233445566/locations/global/workloadIdentityPools/github/providers/github-oidc"
            lines = out.splitlines(keepends=True)
            runner_line = next((line for line in lines if "BACKUP_RUNNER_WIF_PROVIDER" in line), None)
            reader_line = next((line for line in lines if "BACKUP_GCP_WIF_PROVIDER" in line), None)
            self.assertIsNotNone(runner_line, f"runner provider is not the app project's:\n{out}")
            self.assertIn(provider, runner_line, f"runner provider is not the app project's:\n{out}")
            self.assertIsNotNone(reader_line, f"reader provider is not the app project's:\n{out}")
            self.assertIn(provider, reader_line, f"reader provider is not the app project's:\n{out}")

    def test_re_running_binds_nothing_when_every_grant_already_exists(self):
        # Both identities bind the same subject from the app project's pool.
        principal = APP_POOL_PRINCIPAL
        bucket_policy = {
            "bindings": [
                {"role": UPLOADER_ROLE, "members": [f"serviceAccount:{RUNNER}"]},
                {"role": "roles/storage.objectViewer", "members": [f"serviceAccount:{READER}"]},
            ]
        }
        sa_policy = {
            "bindings": [
                {"role": "roles/iam.workloadIdentityUser", "members": [principal]},
            ]
        }
        project_policy = {
            "bindings": [
                {"role": "roles/iap.tunnelResourceAccessor", "members": [f"serviceAccount:{RUNNER}"]},
                {"role": "roles/compute.osAdminLogin", "members": [f"serviceAccount:{RUNNER}"]},
                {"role": "roles/compute.viewer", "members": [f"serviceAccount:{RUNNER}"]},
            ]
        }
        disk = {"resourcePolicies": [
            f"https://www.googleapis.com/compute/v1/projects/{APP_PROJECT}/regions/us-central1/resourcePolicies/smartfire-nightly-boot-disk"
        ]}
        scenario = {
            "STUB_BUCKET_EXISTS": "1",
            "STUB_BUCKET_POLICY_JSON": to_json(bucket_policy),
            "STUB_SA_POLICY_JSON": to_json(sa_policy),
            "STUB_PROJECT_POLICY_JSON": to_json(project_policy),
            "STUB_RUNNER_EXISTS": "1",
            "STUB_READER_EXISTS": "1",
            "STUB_ROLE_EXISTS": "1",
            "STUB_APP_POOL_EXISTS": "1",
            "STUB_APP_PROVIDER_EXISTS": "1",
            "STUB_POLICY_EXISTS": "1",
            "STUB_DISK_JSON": to_json(disk),
        }
        with self.setup_env(scenario) as (env, dirs):
            out, status = self.run_setup(env)
            self.assertEqual(0, status.returncode, f"re-run failed: {out}")
            log = self.gcloud_log(dirs)
            joined = "\n".join(log)
            self.assertFalse(any("add-iam-policy-binding" in line for line in log),
                f"re-run must not duplicate grants:\n{joined}")
            self.assertTrue(any("roles update backupUploader" in line and f"--permissions={UPLOADER_PERMISSIONS}" in line for line in log),
                f"re-run must converge the custom role:\n{joined}")
            self.assertFalse(any("add-resource-policies" in line for line in log),
                f"re-run must not re-attach the schedule:\n{joined}")

    # --- helpers -----------------------------------------------------------

    @contextmanager
    def setup_env(self, scenario=None):
        with tempfile.TemporaryDirectory(prefix="smartfire-setup-test") as tmp:
            root = Path(tmp)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            stub = bin_dir / "gcloud"
            stub.write_text(GCLOUD_STUB)
            stub.chmod(0o755)
            env = {
                "PATH": f"{bin_dir}:{os.environ.get('PATH', '')}",
                "STUB_LOG": str(root / "calls.log"),
                "BACKUP_BUCKET": "test-backup-bucket",
                "BACKUP_PROJECT_ID": PROJECT,
                "APP_PROJECT_ID": APP_PROJECT,
                **(scenario or {}),
            }
            yield env, root

    def run_setup(self, env, unset=()):
        full = {**os.environ, **env}
        for key in unset:
            full.pop(key, None)
        result = subprocess.run([str(SCRIPT)], env=full, stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
        return result.stdout, result

    def gcloud_log(self, dirs):
        return [line.strip() for line in (dirs / "calls.log").read_text().splitlines(keepends=True)]


if __name__ == "__main__":
    unittest.main()
