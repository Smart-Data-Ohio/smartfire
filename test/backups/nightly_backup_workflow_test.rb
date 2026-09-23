require "test_helper"

# Static contract for .github/workflows/nightly-backup.yml: schedule and
# dispatch trigger, SHA-pinned actions (matching the deploy workflow), the
# IAP-driven script run with checksum verification, and always-run VM
# cleanup. The workflow itself only ever runs against real GCP, so these
# assertions pin the shape that the runbook documents.
class NightlyBackupWorkflowTest < ActiveSupport::TestCase
  WORKFLOW = Rails.root.join(".github/workflows/nightly-backup.yml").freeze

  test "runs nightly and on manual dispatch" do
    text = File.read(WORKFLOW)
    assert_includes text, "cron: '0 9 * * *'"
    assert_includes text, "workflow_dispatch:"
  end

  test "pins every action by SHA" do
    text = File.read(WORKFLOW)
    uses = text.scan(/^\s*uses:\s*(\S+)/).flatten
    assert uses.many?, "no actions found in the workflow"
    uses.each do |ref|
      assert_match(/\A[^@\s]+@[0-9a-f]{40}(?:\s+#\s*\S+)?\z/, ref,
        "unpinned action reference: #{ref}")
    end
  end

  test "drives the backup script over IAP and verifies the checksum" do
    text = File.read(WORKFLOW)
    assert_includes text, "campfire-backup.sh"
    assert_includes text, "--tunnel-through-iap"
    assert_includes text, "BACKUP_SHA256"
    assert_includes text, "sha256sum"
    assert_includes text, "checksum mismatch"
    assert_includes text, "daily/"
    assert_includes text, "weekly/"
    assert_includes text, "monthly/"
  end

  test "cleans the VM even when the run fails" do
    text = File.read(WORKFLOW)
    assert_includes text, "if: always()"
    assert_includes text, "rm -rf"
  end

  test "fails loudly when a release holds the lock" do
    text = File.read(WORKFLOW)
    assert_includes text, "exit 75"
    assert_includes text, "release holds"
  end

  test "requests only read and identity permissions" do
    text = File.read(WORKFLOW)
    assert_includes text, "permissions: {}"
    assert_includes text, "contents: read"
    assert_includes text, "id-token: write"
    refute_includes text, "contents: write"
  end
end
