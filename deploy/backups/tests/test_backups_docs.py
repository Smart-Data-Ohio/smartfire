"""Guards the backup runbook against truncation and drift.

Every table-of-contents entry must resolve to a real section, the runbook must
cover the restore paths the task requires, and known-dangerous instructions
must stay out.
"""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[3]
DOC = ROOT / "docs/backups.md"


def slug(heading):
    """GitHub's heading anchor algorithm, close enough for runbook slugs."""
    text = heading.lower().strip()
    text = "".join(char for char in text if char.isalnum() or char in " \t\r\n\f\v-")
    return re.sub(r"[ \t\r\n\f\v]+", "-", text)


def headings(text):
    return [slug(heading) for heading in re.findall(r"^## (.+)$", text, re.M)]


class BackupsDocsTest(unittest.TestCase):
    def test_every_contents_entry_resolves_to_a_section(self):
        text = DOC.read_text()
        anchors = re.findall(r"^- \[.*?\]\(#(.*?)\)", text, re.M)
        self.assertGreater(len(anchors), 1, "no table-of-contents entries found")
        sections = headings(text)
        missing = [anchor for anchor in anchors if anchor not in sections]
        self.assertEqual([], missing, f"contents entries without a section: {', '.join(missing)}")

    def test_runbook_covers_the_required_restore_paths(self):
        text = DOC.read_text()
        for anchor in [
            "restore-onto-a-fresh-vm", "roll-back", "snapshot-schedule-vs-release-snapshots",
            "monthly-automated-restore-check", "key-management", "troubleshooting",
        ]:
            self.assertIn(anchor, headings(text), f"missing section: {anchor}")
        self.assertIn("PRAGMA integrity_check", text)
        self.assertIn("restore-check.sh", text)
        self.assertIn("setup-backup-project.sh", text)

    def test_runbook_is_complete_not_truncated(self):
        text = DOC.read_text()
        self.assertNotIn("truncated", text)
        parts = re.split(r"^## Troubleshooting\s*$", text, flags=re.M)
        while len(parts) > 1 and parts[-1] == "":
            parts.pop()  # Ruby's String#split drops trailing empty fields
        tail = parts[-1] if parts else ""
        self.assertGreater(len(tail), 500, "troubleshooting section looks cut off")

    def test_volume_discovery_never_guesses_the_first_volume(self):
        text = DOC.read_text()
        self.assertNotIn("docker volume ls --format '{{.Name}}' | grep . | head -n 1", text)
        self.assertIn("docker ps -a --filter 'label=once'", text)

    def test_restores_preserve_the_live_file_ownership(self):
        text = DOC.read_text()
        self.assertNotIn("install -m 0644 -o root -g root production.sqlite3", text)
        self.assertIn("chown --reference", text)

    def test_identity_section_matches_the_actions_driven_no_vm_stop_design(self):
        text = DOC.read_text()
        match = re.search(r"^## Identities and least privilege.*?(?=^## )", text, re.M | re.S)
        self.assertTrue(match, "identities section missing")
        normalized = re.sub(r"\s+", " ", match.group(0))
        self.assertIn("smartfire-backup-runner", normalized)
        self.assertIn("cross-project", normalized)
        self.assertIn("never stopped", normalized)
        self.assertIn("no service account and needs none", normalized)
        self.assertNotIn("smartfire-backup-writer", text)
        self.assertNotIn("campfire-backup.timer", text)
        self.assertIn("nightly-backup.yml", text)

    def test_restores_move_the_wal_sidecars_aside_instead_of_deleting_them(self):
        text = DOC.read_text()
        self.assertIn("pre-restore-$STAMP.sqlite3-wal", text)
        self.assertIn("pre-restore-$STAMP.sqlite3-shm", text)
        self.assertNotIn('rm -f "$MOUNT/db/production.sqlite3-wal"', text)
        self.assertNotIn('rm -f "$MOUNT/db/production.sqlite3-shm"', text)

    def test_documents_the_existing_snapshot_schedule(self):
        text = DOC.read_text()
        self.assertIn("default-schedule-1", text)
        self.assertIn("14:00 UTC", text)


if __name__ == "__main__":
    unittest.main()
