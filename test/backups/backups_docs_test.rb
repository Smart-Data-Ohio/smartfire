require "test_helper"

# Guards the backup runbook against truncation and drift: every
# table-of-contents entry must resolve to a real section, the runbook must
# cover the restore paths the task requires, and known-dangerous instructions
# must stay out.
class BackupsDocsTest < ActiveSupport::TestCase
  DOC = Rails.root.join("docs/backups.md").freeze

  test "every contents entry resolves to a section" do
    text = File.read(DOC)
    anchors = text.scan(/^- \[.*?\]\(#(.*?)\)/).flatten
    assert anchors.many?, "no table-of-contents entries found"
    missing = anchors - headings(text)
    assert missing.empty?, "contents entries without a section: #{missing.join(', ')}"
  end

  test "runbook covers the required restore paths" do
    text = File.read(DOC)
    %w[
      restore-onto-a-fresh-vm roll-back snapshot-schedule-vs-release-snapshots
      monthly-automated-restore-check key-management troubleshooting
    ].each do |anchor|
      assert_includes headings(text), anchor, "missing section: #{anchor}"
    end
    assert_includes text, "PRAGMA integrity_check"
    assert_includes text, "restore-check.sh"
    assert_includes text, "setup-backup-project.sh"
  end

  test "runbook is complete, not truncated" do
    text = File.read(DOC)
    refute_includes text, "truncated"
    tail = text.split(/^## Troubleshooting\s*$/).last.to_s
    assert tail.length > 500, "troubleshooting section looks cut off"
  end

  test "volume discovery never guesses the first volume" do
    text = File.read(DOC)
    refute_includes text, "docker volume ls --format '{{.Name}}' | grep . | head -n 1"
    assert_includes text, "docker ps -a --filter 'label=once'"
  end

  test "restores preserve the live file ownership" do
    text = File.read(DOC)
    refute_includes text, "install -m 0644 -o root -g root production.sqlite3"
    assert_includes text, "chown --reference"
  end

  test "identity section matches the Actions-driven, no-VM-stop design" do
    text = File.read(DOC)
    identity = text[/^## Identities and least privilege.*?(?=^## )/m]
    assert identity, "identities section missing"
    normalized = identity.gsub(/\s+/, " ")
    assert_includes normalized, "smartfire-backup-runner"
    assert_includes normalized, "cross-project"
    assert_includes normalized, "never stopped"
    assert_includes normalized, "no service account and needs none"
    refute_includes text, "smartfire-backup-writer"
    refute_includes text, "campfire-backup.timer"
    assert_includes text, "nightly-backup.yml"
  end

  test "restores move the WAL sidecars aside instead of deleting them" do
    text = File.read(DOC)
    assert_includes text, "pre-restore-$STAMP.sqlite3-wal"
    assert_includes text, "pre-restore-$STAMP.sqlite3-shm"
    refute_includes text, 'rm -f "$MOUNT/db/production.sqlite3-wal"'
    refute_includes text, 'rm -f "$MOUNT/db/production.sqlite3-shm"'
  end

  test "documents the existing snapshot schedule" do
    text = File.read(DOC)
    assert_includes text, "default-schedule-1"
    assert_includes text, "14:00 UTC"
  end

  private

    def headings(text)
      text.scan(/^## (.+)$/).flatten.map { |heading| slug(heading) }
    end

    # GitHub's heading anchor algorithm, close enough for runbook slugs.
    def slug(heading)
      heading.downcase.strip.gsub(/[^\p{Alnum}\s-]/, "").gsub(/\s+/, "-")
    end
end
