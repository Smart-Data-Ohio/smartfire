require "test_helper"

# Static contract for .github/workflows/backup-restore-check.yml: monthly
# schedule and dispatch trigger, SHA-pinned actions (matching the deploy
# workflow), and the Rust image, built from the checkout without pushing,
# that restore-check.sh hands a disposable copy of the restored database to.
# The workflow itself only ever runs against real GCP, so these assertions pin
# the shape that the runbook documents.
class RestoreCheckWorkflowTest < ActiveSupport::TestCase
  WORKFLOW = Rails.root.join(".github/workflows/backup-restore-check.yml").freeze

  test "runs monthly and on manual dispatch" do
    text = File.read(WORKFLOW)
    assert_includes text, "cron: '0 9 1 * *'"
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

  test "builds the Rust image from the checkout without pushing before the check" do
    text = File.read(WORKFLOW)
    assert_includes text, "file: rust/Dockerfile"
    assert_includes text, "build-contexts: reference=."
    assert_includes text, "push: false"
    assert_includes text, "load: true"
    refute_includes text, "cache-to:"
    refute_includes text, "setup-ruby"
    assert_operator text.index("Build the Rust image"), :<, text.index("bash deploy/backups/restore-check.sh"),
      "the image must be built before restore-check.sh uses it"
  end

  test "checks the restored database with the Rust image and minimums" do
    text = File.read(WORKFLOW)
    assert_includes text, "restore-check.sh"
    assert_includes text, '--image "$CHECK_IMAGE"'
    assert_includes text, "--min-users 1 --min-messages 1"
    refute_includes text, "--rails-root"
  end

  test "requests only read and identity permissions" do
    text = File.read(WORKFLOW)
    assert_includes text, "permissions: {}"
    assert_includes text, "contents: read"
    assert_includes text, "id-token: write"
    refute_includes text, "contents: write"
  end
end
