require "test_helper"
require "tmpdir"
require "open3"
require "fileutils"

# Contract tests for deploy/backups/snapshot-schedule.sh, which the lead may
# run directly (not only through setup-backup-project.sh). A stubbed `gcloud`
# emulates the GCP boundary; jq is real except in the missing-jq test, where
# PATH is scrubbed to prove the prerequisite error.
class SnapshotScheduleTest < ActiveSupport::TestCase
  SCRIPT = Rails.root.join("deploy/backups/snapshot-schedule.sh").freeze

  test "creates the schedule and attaches it to the boot disk" do
    with_snapshot_env do |env, dirs|
      out, status = Open3.capture2e(env, SCRIPT.to_s)
      assert status.success?, "snapshot setup failed: #{out}"
      log = gcloud_log(dirs)
      assert log.any? { |line| line.include?("resource-policies create snapshot-schedule smartfire-nightly-boot-disk") },
        "schedule was not created:\n#{log.join("\n")}"
      assert log.any? { |line| line.include?("add-resource-policies") && line.include?("campfire") },
        "schedule was not attached to the boot disk:\n#{log.join("\n")}"
    end
  end

  test "reports a missing jq instead of failing mid-run" do
    with_snapshot_env(scrub_jq: true) do |env, _dirs|
      out, status = Open3.capture2e(env, SCRIPT.to_s)
      refute status.success?
      assert_includes out, "jq is not installed"
    end
  end

  private

    GCLOUD_STUB = <<~'SH'.freeze
      #!/usr/bin/env bash
      echo "gcloud $*" >> "$STUB_LOG"
      case "$2" in
        resource-policies)
          case "$3" in
            describe) echo "not found" >&2; exit 1 ;;
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
              printf '%s' '{"resourcePolicies":[]}'
              exit 0
              ;;
            add-resource-policies) exit 0 ;;
          esac
          ;;
      esac
      echo "stub: unhandled: gcloud $*" >&2
      exit 9
    SH

    def with_snapshot_env(scrub_jq: false)
      Dir.mktmpdir("smartfire-snapshot-test") do |dir|
        root = Pathname.new(dir)
        bin = root.join("bin")
        bin.mkpath
        stub = bin.join("gcloud")
        stub.write(GCLOUD_STUB)
        FileUtils.chmod 0o755, stub
        path = "#{bin}:/usr/bin:/bin"
        if scrub_jq
          # A PATH with a shell but no jq: #!/usr/bin/env bash still resolves,
          # while command -v jq must fail.
          clean = root.join("cleanbin")
          clean.mkpath
          File.symlink("/bin/bash", clean.join("bash"))
          File.symlink("/bin/bash", clean.join("sh"))
          path = "#{bin}:#{clean}"
        end
        env = {
          "PATH" => path,
          "STUB_LOG" => root.join("calls.log").to_s,
          "APP_PROJECT_ID" => "test-app-project"
        }
        yield env, root
      end
    end

    def gcloud_log(dirs)
      File.read(dirs.join("calls.log")).lines.map(&:strip)
    end
end
