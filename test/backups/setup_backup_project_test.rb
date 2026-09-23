require "test_helper"
require "tmpdir"
require "open3"
require "fileutils"
require "json"

# Contract tests for deploy/backups/setup-backup-project.sh. Each test runs
# the real script with a stubbed `gcloud` on PATH that emulates the GCP
# boundary (project/bucket/IAM/WIF/snapshot state via STUB_* variables) and
# records every invocation. Only the cloud boundary is stubbed; jq and the
# delegated snapshot-schedule.sh are real.
class SetupBackupProjectTest < ActiveSupport::TestCase
  SCRIPT = Rails.root.join("deploy/backups/setup-backup-project.sh").freeze
  RUNNER = "smartfire-backup-runner@smart-data-campfire.iam.gserviceaccount.com"
  READER = "smartfire-backup-reader@smart-data-campfire-backups.iam.gserviceaccount.com"
  SUBJECT = "repo:Smart-Data-Ohio@262436228/smartfire@1370426325:ref:refs/heads/main"
  PROJECT = "smart-data-campfire-backups"
  APP_PROJECT = "smart-data-campfire"

  GCLOUD_STUB = <<~'SH'.freeze
    #!/usr/bin/env bash
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
          workload-identity-pools)
            # Pool/provider existence is per project: the runner's pool
            # lives in the app project, the reader's in the backup project.
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
                # discovery. The setup itself never inspects the VM: it
                # holds no service account and needs none.
                disk='{"boot": true, "source": "https://www.googleapis.com/compute/v1/projects/x/zones/y/disks/campfire"}'
                printf '{"serviceAccounts": [], "disks": [%s]}\n' "$disk"
                exit 0
                ;;
              get-iam-policy)
                if [ -n "${STUB_INSTANCE_POLICY_JSON:-}" ]; then
                  printf '%s\n' "$STUB_INSTANCE_POLICY_JSON"
                else
                  printf '%s\n' '{"bindings":[]}'
                fi
                exit 0
                ;;
              add-iam-policy-binding)
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
  SH

  test "creates the backup-runner in the app project with a create-only bucket grant" do
    with_setup_env do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("service-accounts create smartfire-backup-runner") && line.include?("--project=#{APP_PROJECT}") },
        "runner SA was not created in the app project:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("add-iam-policy-binding") && line.include?("roles/storage.objectCreator") && line.include?("serviceAccount:#{RUNNER}") },
        "runner was not granted objectCreator:\n#{log.join("\n")}"
      refute log.any? { |line| line.include?("service-accounts create") && line.include?("smartfire-backup-writer") },
        "no writer service account must be created:\n#{log.join("\n")}"
      refute_includes out, "instances stop", "no VM stop may be prescribed:\n#{out}"
      assert_includes out, "No VM change was needed"
    end
  end

  test "creates the bucket in the US multi-region" do
    with_setup_env do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("storage buckets create") && line.include?("--location=US") },
        "bucket was not created in US:\n#{log.join("\n")}"
    end
  end

  test "grants the runner IAP and SSH roles on the VM only" do
    with_setup_env do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      %w[
        roles/iap.tunnelResourceAccessor
        roles/compute.osAdminLogin
        roles/compute.viewer
      ].each do |role|
        assert log.any? { |line|
          line.include?("compute instances add-iam-policy-binding campfire") &&
            line.include?("--role=#{role}") && line.include?("serviceAccount:#{RUNNER}")
        }, "runner was not granted #{role} on the VM:\n#{log.join("\n")}"
      end
      refute log.any? { |line| line.include?("set-service-account") },
        "nothing may be attached to the VM:\n#{log.join("\n")}"
      refute log.any? { |line| line.include?("roles/compute.instanceAdmin") || line.include?("roles/compute.storageAdmin") },
        "the runner must hold no deploy rights:\n#{log.join("\n")}"
    end
  end

  test "binds the runner with the ID-qualified subject" do
    with_setup_env do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      principal = "principal://iam.googleapis.com/projects/112233445566/locations/global" \
        "/workloadIdentityPools/smartfire-backup-pool/subject/#{SUBJECT}"
      assert log.any? { |line|
               line.include?("service-accounts add-iam-policy-binding #{RUNNER}") &&
                 line.include?("roles/iam.workloadIdentityUser") && line.include?(principal)
             },
        "runner was not bound to the ID-qualified subject:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("workload-identity-pools providers create-oidc") && line.include?("--project=#{APP_PROJECT}") },
        "no WIF provider was created in the app project:\n#{log.join("\n")}"
      refute log.any? { |line| line.include?("principalSet") },
        "must not use attribute-based bindings:\n#{log.join("\n")}"
    end
  end

  test "keeps the reader identity for the restore check" do
    with_setup_env do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("service-accounts create smartfire-backup-reader") && line.include?("--project=#{PROJECT}") },
        "reader SA was not created in the backup project:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("roles/storage.objectViewer") && line.include?("serviceAccount:#{READER}") },
        "reader was not granted objectViewer:\n#{log.join("\n")}"
      principal = "principal://iam.googleapis.com/projects/922766272284/locations/global" \
        "/workloadIdentityPools/smartfire-backup-pool/subject/#{SUBJECT}"
      assert log.any? { |line|
               line.include?("service-accounts add-iam-policy-binding #{READER}") &&
                 line.include?("roles/iam.workloadIdentityUser") && line.include?(principal)
             },
        "reader was not bound to the ID-qualified subject:\n#{log.join("\n")}"
    end
  end

  test "does not enable the compute API in the backup project" do
    with_setup_env do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      backup_enable = log.select { |line| line.include?("services enable") && line.include?("--project=#{PROJECT}") }
      assert backup_enable.any?, "no API enablement ran for the backup project"
      backup_enable.each do |line|
        refute_includes line, "compute.googleapis.com",
          "compute must not be enabled in the backup project:\n#{line}"
      end
      assert log.any? { |line| line.include?("services enable") && line.include?("--project=#{APP_PROJECT}") },
        "APIs were not enabled in the app project:\n#{log.join("\n")}"
    end
  end

  test "delegates the snapshot schedule" do
    with_setup_env do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("instances describe campfire") && line.include?("--project=#{APP_PROJECT}") },
        "the snapshot schedule was not delegated:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("resource-policies create snapshot-schedule smartfire-nightly-boot-disk") },
        "the schedule was not created on an unscheduled disk:\n#{log.join("\n")}"
    end
  end

  test "skips the snapshot schedule when the disk already has one" do
    disk = { "resourcePolicies" => [
      "https://www.googleapis.com/compute/v1/projects/#{APP_PROJECT}/regions/us-central1/resourcePolicies/default-schedule-1"
    ] }
    policy = {
      "snapshotSchedulePolicy" => {
        "schedule" => { "dailySchedule" => { "daysInCycle" => 1, "startTime" => "14:00" } },
        "retentionPolicy" => { "maxRetentionDays" => 14, "onSourceDiskDelete" => "KEEP_AUTO_SNAPSHOTS" }
      }
    }
    scenario = {
      "STUB_DISK_JSON" => JSON.generate(disk),
      "STUB_POLICY_DESCRIBE_JSON" => JSON.generate(policy)
    }
    with_setup_env(scenario) do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      assert_includes out, "default-schedule-1"
      assert_includes out, "14:00"
      assert_includes out, "14 days"
      log = gcloud_log(dirs)
      refute log.any? { |line| line.include?("resource-policies create") },
        "no new schedule may be created:\n#{log.join("\n")}"
      refute log.any? { |line| line.include?("add-resource-policies") },
        "nothing may be attached to an already-scheduled disk:\n#{log.join("\n")}"
    end
  end

  test "defaults to the real backup project, app project and VM" do
    with_setup_env do |env, _dirs|
      env = env.except("BACKUP_PROJECT_ID", "APP_PROJECT_ID")
      out, status = run_setup(env)
      assert status.success?, "setup failed without project ids: #{out}"
    end

    with_setup_env do |env, dirs|
      env = env.except("BACKUP_PROJECT_ID", "APP_PROJECT_ID")
      _out, _status = run_setup(env)
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("projects describe #{PROJECT}") },
        "did not use the real backup project by default:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("instances add-iam-policy-binding campfire") && line.include?("--project=#{APP_PROJECT}") && line.include?("--zone=us-central1-a") },
        "did not bind the real app VM by default:\n#{log.join("\n")}"
    end
  end

  test "prints manual project steps when the project does not exist" do
    with_setup_env("STUB_PROJECT_EXISTS" => "0") do |env, _dirs|
      out, status = run_setup(env)
      refute status.success?
      assert_includes out, "gcloud projects create"
      assert_includes out, "billing"
      refute_includes out, "compute.googleapis.com", "the manual steps must not enable compute either"
    end
  end

  test "prints the GitHub variables to set" do
    with_setup_env do |env, _dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      %w[
        BACKUP_GCP_BUCKET BACKUP_APP_PROJECT BACKUP_APP_ZONE BACKUP_APP_INSTANCE
        BACKUP_RUNNER_WIF_PROVIDER BACKUP_RUNNER_SA BACKUP_AGE_RECIPIENT
        BACKUP_GCP_PROJECT_ID BACKUP_GCP_WIF_PROVIDER BACKUP_GCP_READER_SA
        BACKUP_RESTORE_AGE_IDENTITY
      ].each do |name|
        assert_includes out, name, "missing from the printed variables: #{name}"
      end
      assert_includes out, RUNNER
      assert_includes out, "projects/112233445566/locations/global/workloadIdentityPools/smartfire-backup-pool/providers/github-actions"
    end
  end

  test "re-running binds nothing when every grant already exists" do
    runner_principal = "principal://iam.googleapis.com/projects/112233445566/locations/global" \
      "/workloadIdentityPools/smartfire-backup-pool/subject/#{SUBJECT}"
    reader_principal = "principal://iam.googleapis.com/projects/922766272284/locations/global" \
      "/workloadIdentityPools/smartfire-backup-pool/subject/#{SUBJECT}"
    bucket_policy = {
      "bindings" => [
        { "role" => "roles/storage.objectCreator", "members" => [ "serviceAccount:#{RUNNER}" ] },
        { "role" => "roles/storage.objectViewer", "members" => [ "serviceAccount:#{READER}" ] }
      ]
    }
    sa_policy = {
      "bindings" => [
        { "role" => "roles/iam.workloadIdentityUser", "members" => [ runner_principal, reader_principal ] }
      ]
    }
    instance_policy = {
      "bindings" => [
        { "role" => "roles/iap.tunnelResourceAccessor", "members" => [ "serviceAccount:#{RUNNER}" ] },
        { "role" => "roles/compute.osAdminLogin", "members" => [ "serviceAccount:#{RUNNER}" ] },
        { "role" => "roles/compute.viewer", "members" => [ "serviceAccount:#{RUNNER}" ] }
      ]
    }
    disk = { "resourcePolicies" => [
      "https://www.googleapis.com/compute/v1/projects/#{APP_PROJECT}/regions/us-central1/resourcePolicies/smartfire-nightly-boot-disk"
    ] }
    scenario = {
      "STUB_BUCKET_EXISTS" => "1",
      "STUB_BUCKET_POLICY_JSON" => JSON.generate(bucket_policy),
      "STUB_SA_POLICY_JSON" => JSON.generate(sa_policy),
      "STUB_INSTANCE_POLICY_JSON" => JSON.generate(instance_policy),
      "STUB_RUNNER_EXISTS" => "1",
      "STUB_READER_EXISTS" => "1",
      "STUB_POOL_EXISTS" => "1",
      "STUB_PROVIDER_EXISTS" => "1",
      "STUB_APP_POOL_EXISTS" => "1",
      "STUB_APP_PROVIDER_EXISTS" => "1",
      "STUB_POLICY_EXISTS" => "1",
      "STUB_DISK_JSON" => JSON.generate(disk)
    }
    with_setup_env(scenario) do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "re-run failed: #{out}"
      log = gcloud_log(dirs)
      refute log.any? { |line| line.include?("add-iam-policy-binding") },
        "re-run must not duplicate grants:\n#{log.join("\n")}"
      refute log.any? { |line| line.include?("add-resource-policies") },
        "re-run must not re-attach the schedule:\n#{log.join("\n")}"
    end
  end

  private

    def with_setup_env(scenario = {})
      Dir.mktmpdir("smartfire-setup-test") do |dir|
        root = Pathname.new(dir)
        bin = root.join("bin")
        bin.mkpath
        stub = bin.join("gcloud")
        stub.write(GCLOUD_STUB)
        FileUtils.chmod 0o755, stub
        env = {
          "PATH" => "#{bin}:#{ENV['PATH']}",
          "STUB_LOG" => root.join("calls.log").to_s,
          "BACKUP_BUCKET" => "test-backup-bucket",
          "BACKUP_PROJECT_ID" => PROJECT,
          "APP_PROJECT_ID" => APP_PROJECT
        }.merge(scenario)
        yield env, root
      end
    end

    def run_setup(env)
      Open3.capture2e(env, SCRIPT.to_s)
    end

    def gcloud_log(dirs)
      File.read(dirs.join("calls.log")).lines.map(&:strip)
    end
end
