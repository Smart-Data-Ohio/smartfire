import json
from pathlib import Path
import tempfile
import unittest

from ignored_tests import ROOT, check, inventory, verify_junit


class IgnoredTestCoverage(unittest.TestCase):
    def test_repository_has_no_unowned_ignored_correctness(self):
        check(ROOT, json.loads((ROOT / "ci/ignored-tests.json").read_text()),
              (ROOT.parent / ".github/workflows/rust.yml").read_text())

    def test_new_ignored_test_and_missing_job_fail_closed(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "crates").mkdir()
            source = root / "crates/tests.rs"
            source.write_text('// #[ignore]\n// fn prose() {}\n#[test]\n#[ignore = "requires a server"]\nasync fn correctness() {}\n')
            record = {"path": "crates/tests.rs", "test": "correctness"}
            workflow = 'suite: [server]\nbash rust/ci/correctness.sh "$SUITE"'
            self.assertEqual(len(inventory(root)), 1)
            with self.assertRaisesRegex(ValueError, "no CI job"):
                check(root, {}, workflow)
            with self.assertRaisesRegex(ValueError, "missing CI job"):
                check(root, {"absent": [record]}, workflow)
            self.assertEqual(check(root, {"server": [record]}, workflow), (1, 0))
            source.write_text('#[test]\n#[ignore = "utility: recorder"]\nfn correctness() {}\n')
            self.assertEqual(check(root, {}, workflow), (0, 1))
            with self.assertRaisesRegex(ValueError, "stale"):
                check(root, {"server": [record]}, workflow)

    def test_missing_or_skipped_selected_test_cannot_make_a_green_receipt(self):
        with tempfile.TemporaryDirectory() as scratch:
            path = Path(scratch) / "junit.xml"
            records = [{"test": "selected"}]
            for body in ('', '<testcase name="selected"><skipped/></testcase>',
                         '<testcase name="selected"><failure/></testcase>', '<testcase name="unrelated"/>'):
                path.write_text(f"<testsuites><testsuite>{body}</testsuite></testsuites>")
                with self.assertRaises(ValueError):
                    verify_junit(records, path)
            path.write_text('<testsuites><testsuite><testcase name="selected"/></testsuite></testsuites>')
            verify_junit(records, path)
