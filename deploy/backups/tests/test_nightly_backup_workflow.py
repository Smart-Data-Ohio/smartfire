"""Static contract for .github/workflows/nightly-backup.yml.

Schedule and dispatch trigger, SHA-pinned actions (matching the deploy
workflow), the IAP-driven script run with checksum verification, and
always-run VM cleanup. The workflow itself only ever runs against real GCP, so
these assertions pin the shape that the runbook documents.
"""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / ".github/workflows/nightly-backup.yml"


class NightlyBackupWorkflowTest(unittest.TestCase):
    def test_runs_nightly_and_on_manual_dispatch(self):
        text = WORKFLOW.read_text()
        self.assertIn("cron: '0 9 * * *'", text)
        self.assertIn("workflow_dispatch:", text)

    def test_pins_every_action_by_sha(self):
        text = WORKFLOW.read_text()
        uses = re.findall(r"^\s*uses:\s*(\S+)", text, re.M)
        self.assertGreater(len(uses), 1, "no actions found in the workflow")
        for ref in uses:
            self.assertRegex(ref, r"\A[^@\s]+@[0-9a-f]{40}(?:\s+#\s*\S+)?\Z",
                f"unpinned action reference: {ref}")

    def test_drives_the_backup_script_over_iap_and_verifies_the_checksum(self):
        text = WORKFLOW.read_text()
        self.assertIn("campfire-backup.sh", text)
        self.assertIn("--tunnel-through-iap", text)
        self.assertIn("BACKUP_SHA256", text)
        self.assertIn("sha256sum", text)
        self.assertIn("checksum mismatch", text)
        self.assertIn("daily/", text)
        self.assertIn("weekly/", text)
        self.assertIn("monthly/", text)

    def test_cleans_the_vm_even_when_the_run_fails(self):
        text = WORKFLOW.read_text()
        self.assertIn("if: always()", text)
        self.assertIn("rm -rf", text)

    def test_fails_loudly_when_a_release_holds_the_lock(self):
        text = WORKFLOW.read_text()
        self.assertIn("exit 75", text)
        self.assertIn("release holds", text)

    def test_requests_only_read_and_identity_permissions(self):
        text = WORKFLOW.read_text()
        self.assertIn("permissions: {}", text)
        self.assertIn("contents: read", text)
        self.assertIn("id-token: write", text)
        self.assertNotIn("contents: write", text)


if __name__ == "__main__":
    unittest.main()
