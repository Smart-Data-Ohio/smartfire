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
  VM_SA = "campfire-vm@smart-data-campfire.iam.gserviceaccount.com"
  SUBJECT = "repo:Smart-Data-Ohio@262436228/smartfire@1370426325:ref:refs/heads/main"
  PROJECT = "smart-data-campfire-backups"

  GCLOUD_STUB = <<~'SH'.freeze
    #!/usr/bin/env bash
    # Emulates just enough gcloud for setup-backup-project.sh. Scenario state
    # comes from STUB_* variables; every invocation is appended to $STUB_LOG.
    echo "gcloud $*" >> "$STUB_LOG"
    case "$1" in
      projects)
        if [ "${STUB_PROJECT_EXISTS:-1}" != "1" ]; then
          echo "ERROR: project not found" >&2
          exit 1
        fi
        case "$*" in
          *projectNumber*) echo "${STUB_PROJECT_NUMBER:-922766272284}" ;;
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
                  smartfire-backup-writer@*)
                    [ "${STUB_WRITER_EXISTS:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
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
            case "$3" in
              describe)
                [ "${STUB_POOL_EXISTS:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
                ;;
              create)
                exit 0
                ;;
              providers)
                case "$4" in
                  describe)
                    [ "${STUB_PROVIDER_EXISTS:-0}" = "1" ] && exit 0 || { echo "not found" >&2; exit 1; }
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
            # One JSON body serves both the VM identity lookup and the
            # snapshot schedule's boot-disk discovery.
            disk='{"boot": true, "source": "https://www.googleapis.com/compute/v1/projects/x/zones/y/disks/campfire"}'
            if [ -n "${STUB_VM_SA_EMAIL:-}" ]; then
              scopes=""
              for scope in ${STUB_VM_SCOPES:-}; do
                scopes="${scopes}${scopes:+, }\"https://www.googleapis.com/auth/$scope\""
              done
              printf '{"serviceAccounts": [{"email": "%s", "scopes": [%s]}], "disks": [%s]}\n' \
                "$STUB_VM_SA_EMAIL" "$scopes" "$disk"
            else
              printf '{"serviceAccounts": [], "disks": [%s]}\n' "$disk"
            fi
            exit 0
            ;;
          resource-policies)
            case "$3" in
              describe)
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

  test "grants the VM's existing service account with no VM change" do
    with_setup_env(suitable_vm) do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("add-iam-policy-binding") && line.include?("roles/storage.objectCreator") && line.include?("serviceAccount:#{VM_SA}") },
        "VM SA was not granted objectCreator:\n#{log.join("\n")}"
      refute log.any? { |line| line.include?("service-accounts create") && line.include?("smartfire-backup-writer") },
        "no writer service account must be created when the VM SA is usable:\n#{log.join("\n")}"
      refute_includes out, "instances stop", "no VM stop may be prescribed:\n#{out}"
      assert_includes out, "no VM change is needed"
    end
  end

  test "falls back to a writer SA with attach steps when the VM has no service account" do
    with_setup_env({}) do |env, dirs|
      out, status = run_setup(env)
      refute status.success?, "setup must stop until the VM can upload"
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("service-accounts create") && line.include?("smartfire-backup-writer") },
        "fallback writer SA was not created:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("roles/storage.objectCreator") && line.include?("smartfire-backup-writer@") },
        "fallback writer SA was not granted:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("create-oidc") },
        "independent resources (WIF) must still be set up:\n#{log.join("\n")}"
      assert_includes out, "gcloud compute instances stop"
      assert_includes out, "set-service-account"
      assert_includes out, "re-run this script"
    end
  end

  test "falls back when the VM scopes cannot write to Cloud Storage" do
    with_setup_env("STUB_VM_SA_EMAIL" => VM_SA, "STUB_VM_SCOPES" => "devstorage.read_only") do |env, dirs|
      out, status = run_setup(env)
      refute status.success?, "setup must stop until the VM can upload"
      assert_includes out, VM_SA
      assert_includes out, "gcloud compute instances stop"
      assert_includes out, "re-run this script"
    end
  end

  test "binds the reader with the ID-qualified subject" do
    with_setup_env(suitable_vm) do |env, dirs|
      out, status = run_setup(env)
      assert status.success?, "setup failed: #{out}"
      log = gcloud_log(dirs)
      principal = "principal://iam.googleapis.com/projects/922766272284/locations/global" \
        "/workloadIdentityPools/smartfire-backup-pool/subject/#{SUBJECT}"
      assert log.any? { |line| line.include?("roles/iam.workloadIdentityUser") && line.include?(principal) },
        "reader was not bound to the ID-qualified subject:\n#{log.join("\n")}"
      refute log.any? { |line| line.include?("principalSet") },
        "must not use attribute-based bindings:\n#{log.join("\n")}"
    end
  end

  test "defaults to the real backup project, app VM and repository" do
    with_setup_env(suitable_vm) do |env, _dirs|
      env = env.except("BACKUP_PROJECT_ID")
      out, status = run_setup(env)
      assert status.success?, "setup failed without BACKUP_PROJECT_ID: #{out}"
    end

    with_setup_env(suitable_vm) do |env, dirs|
      env = env.except("BACKUP_PROJECT_ID")
      _out, _status = run_setup(env)
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("projects describe #{PROJECT}") },
        "did not use the real backup project by default:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("instances describe campfire") && line.include?("--project=smart-data-campfire") && line.include?("--zone=us-central1-a") },
        "did not look up the real app VM by default:\n#{log.join("\n")}"
    end
  end

  test "prints manual project steps when the project does not exist" do
    with_setup_env(suitable_vm.merge("STUB_PROJECT_EXISTS" => "0")) do |env, _dirs|
      out, status = run_setup(env)
      refute status.success?
      assert_includes out, "gcloud projects create"
      assert_includes out, "billing"
    end
  end

  test "re-running binds nothing when every grant already exists" do
    reader = "smartfire-backup-reader@#{PROJECT}.iam.gserviceaccount.com"
    principal = "principal://iam.googleapis.com/projects/922766272284/locations/global" \
      "/workloadIdentityPools/smartfire-backup-pool/subject/#{SUBJECT}"
    policy = {
      "bindings" => [
        { "role" => "roles/storage.objectCreator", "members" => [ "serviceAccount:#{VM_SA}" ] },
        { "role" => "roles/storage.objectViewer", "members" => [ "serviceAccount:#{reader}" ] }
      ]
    }
    sa_policy = {
      "bindings" => [
        { "role" => "roles/iam.workloadIdentityUser", "members" => [ principal ] }
      ]
    }
    disk = { "resourcePolicies" => [
      "https://www.googleapis.com/compute/v1/projects/smart-data-campfire/regions/us-central1/resourcePolicies/smartfire-nightly-boot-disk"
    ] }
    scenario = suitable_vm.merge(
      "STUB_BUCKET_EXISTS" => "1",
      "STUB_BUCKET_POLICY_JSON" => JSON.generate(policy),
      "STUB_SA_POLICY_JSON" => JSON.generate(sa_policy),
      "STUB_READER_EXISTS" => "1",
      "STUB_POOL_EXISTS" => "1",
      "STUB_PROVIDER_EXISTS" => "1",
      "STUB_POLICY_EXISTS" => "1",
      "STUB_DISK_JSON" => JSON.generate(disk)
    )
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

    def suitable_vm
      { "STUB_VM_SA_EMAIL" => VM_SA, "STUB_VM_SCOPES" => "cloud-platform" }
    end

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
          "BACKUP_PROJECT_ID" => PROJECT
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
