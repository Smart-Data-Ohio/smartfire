"""Static contract for .github/workflows/backup-restore-check.yml.

Monthly schedule and dispatch trigger, SHA-pinned actions (matching the deploy
workflow), and the Rust image, built from the checkout without pushing, that
restore-check.sh hands a disposable copy of the restored database to. The
workflow itself only ever runs against real GCP, so these assertions pin the
shape that the runbook documents.
"""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / ".github/workflows/backup-restore-check.yml"


class RestoreCheckWorkflowTest(unittest.TestCase):
    def test_runs_monthly_and_on_manual_dispatch(self):
        text = WORKFLOW.read_text()
        self.assertIn("cron: '0 9 1 * *'", text)
        self.assertIn("workflow_dispatch:", text)

    def test_pins_every_action_by_sha(self):
        text = WORKFLOW.read_text()
        uses = re.findall(r"^\s*uses:\s*(\S+)", text, re.M)
        self.assertGreater(len(uses), 1, "no actions found in the workflow")
        for ref in uses:
            self.assertRegex(ref, r"\A[^@\s]+@[0-9a-f]{40}(?:\s+#\s*\S+)?\Z",
                f"unpinned action reference: {ref}")

    def test_builds_the_rust_image_from_the_checkout_without_pushing_before_the_check(self):
        text = WORKFLOW.read_text()
        self.assertIn("file: rust/Dockerfile", text)
        self.assertIn("context: rust", text)
        self.assertNotIn("build-contexts:", text)
        self.assertIn("push: false", text)
        self.assertIn("load: true", text)
        self.assertNotIn("cache-to:", text)
        self.assertNotIn("setup-ruby", text)
        self.assertLess(text.index("Build the Rust image"), text.index("bash deploy/backups/restore-check.sh"),
            "the image must be built before restore-check.sh uses it")

    def test_checks_the_restored_database_with_the_rust_image_and_minimums(self):
        text = WORKFLOW.read_text()
        self.assertIn("restore-check.sh", text)
        self.assertIn('--image "$CHECK_IMAGE"', text)
        self.assertIn("--min-users 1 --min-messages 1", text)
        self.assertNotIn("--rails-root", text)

    def test_requests_only_read_and_identity_permissions(self):
        text = WORKFLOW.read_text()
        self.assertIn("permissions: {}", text)
        self.assertIn("contents: read", text)
        self.assertIn("id-token: write", text)
        self.assertNotIn("contents: write", text)


if __name__ == "__main__":
    unittest.main()
