"""Regression checks for the CI summary's failure and ignored-test accounting."""

import importlib.util
from pathlib import Path
import tempfile
import unittest


spec = importlib.util.spec_from_file_location("summary", Path(__file__).with_name("summarize-tests.py"))
summary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(summary)


class SummaryTests(unittest.TestCase):
    def parse(self, text, parser):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report"
            path.write_text(text)
            return parser(path)

    def test_junit_counts_failures_errors_and_ignored_once(self):
        counts, failures = self.parse(
            '<testsuites><testsuite name="db">'
            '<testcase name="passes"><system-out>test result: FAILED. 99 failed</system-out></testcase>'
            '<testcase name="fails"><failure/><rerunFailure/></testcase>'
            '<testcase name="aborts"><error/></testcase>'
            '<testcase name="ignored"><skipped/></testcase>'
            '</testsuite></testsuites>', summary.junit_results
        )
        self.assertEqual(counts, (1, 2, 1))
        self.assertEqual(failures, ["db: fails", "db: aborts"])

    def test_doctests_aggregate_all_libtest_results(self):
        counts, failures = self.parse(
            '\x1b[32mtest result: ok. 3 passed; 0 failed; 2 ignored; 0 measured;\x1b[0m\n'
            'test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured;\n'
            'error: 2 targets failed:\n    --doc a\n    --doc b\n', summary.doc_results
        )
        self.assertEqual(counts, [4, 2, 2])
        self.assertIn("--doc b", failures[0])

    def test_missing_report_is_not_reported_as_success(self):
        with self.assertRaises(FileNotFoundError):
            summary.junit_results("does-not-exist.xml")

    def test_malformed_report_is_not_reported_as_success(self):
        with self.assertRaises(summary.ET.ParseError):
            self.parse('<testsuites>', summary.junit_results)


if __name__ == "__main__":
    unittest.main()
