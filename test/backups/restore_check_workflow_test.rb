require "test_helper"

# Static contract for .github/workflows/backup-restore-check.yml: monthly
# schedule and dispatch trigger, SHA-pinned actions (matching the deploy
# workflow), and the CI system packages (notably libvips, which the Rails
# boot in config/initializers/vips.rb needs) installed before
# restore-check.sh boots the app against the restored database. The
# workflow itself only ever runs against real GCP, so these assertions pin
# the shape that the runbook documents.
class RestoreCheckWorkflowTest < ActiveSupport::TestCase
  WORKFLOW = Rails.root.join(".github/workflows/backup-restore-check.yml").freeze
  CI = Rails.root.join(".github/workflows/ci.yml").freeze
  PACKAGES = "libsqlite3-0 libvips curl ffmpeg"

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

  test "installs the CI system packages before booting Rails" do
    text = File.read(WORKFLOW)
    assert_includes File.read(CI), PACKAGES, "CI no longer installs the expected packages"
    assert_includes text, PACKAGES, "the restore workflow does not install the CI system packages"
    assert_includes text, "libvips"
    assert_operator text.index(PACKAGES), :<, text.index("--rails-root"),
      "the packages must be installed before restore-check.sh boots Rails"
  end

  test "boots Rails read-only against the restored database with minimums" do
    text = File.read(WORKFLOW)
    assert_includes text, "restore-check.sh"
    assert_includes text, "--rails-root"
    assert_includes text, "--min-users 1 --min-messages 1"
  end

  test "requests only read and identity permissions" do
    text = File.read(WORKFLOW)
    assert_includes text, "permissions: {}"
    assert_includes text, "contents: read"
    assert_includes text, "id-token: write"
    refute_includes text, "contents: write"
  end
end
