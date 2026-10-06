"""Regression checks for the CI summary's failure and ignored-test accounting."""

import importlib.util
import json
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

    def write(self, directory, name, text):
        path = Path(directory) / name
        path.write_text(text)
        return path

    LIST = json.dumps({"rust-suites": {"db": {"testcases": {
        "a": {"ignored": False, "filter-match": {"status": "matches"}},
        "b": {"ignored": False, "filter-match": {"status": "matches"}},
        "slow": {"ignored": True, "filter-match": {"status": "mismatch", "reason": "ignored"}},
        "panics": {"ignored": False, "filter-match": {"status": "mismatch", "reason": "expression"}},
    }}}})

    def junit(self, *cases):
        return ('<testsuites><testsuite name="db">' + "".join(
            f'<testcase name="{name}">{"<skipped/>" if name == "slow" else ""}</testcase>' for name in cases
        ) + '</testsuite></testsuites>')

    def errors(self, *receipts):
        with tempfile.TemporaryDirectory() as directory:
            listed = self.write(directory, "list.json", self.LIST)
            paths = [self.write(directory, f"junit-{i}.xml", text) for i, text in enumerate(receipts)]
            return summary.expectation_errors(listed, paths)

    def test_shards_that_together_ran_the_listed_tests_match(self):
        self.assertEqual(self.errors(self.junit("a", "slow"), self.junit("b", "slow")), [])

    def test_a_listed_test_no_shard_ran_fails(self):
        self.assertIn("did not pass", self.errors(self.junit("a", "slow"))[0])

    def test_a_test_two_shards_ran_fails(self):
        self.assertIn("more than one receipt", self.errors(self.junit("a", "b", "slow"), self.junit("a"))[0])

    def test_an_unselected_test_that_ran_fails(self):
        self.assertIn("not selected", self.errors(self.junit("a", "b", "panics", "slow"))[0])

    def test_a_missing_ignored_test_fails(self):
        self.assertIn("ignored tests listed", self.errors(self.junit("a", "b"))[0])

    def test_doctest_logs_must_account_for_every_doctest(self):
        with tempfile.TemporaryDirectory() as directory:
            good = self.write(directory, "good.log", "running 3 tests\ntest result: ok. 1 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out\n")
            short = self.write(directory, "short.log", "running 3 tests\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured;\n")
            empty = self.write(directory, "empty.log", "error: could not compile\n")
            self.assertEqual(summary.doc_errors(good), [])
            self.assertIn("ran 3 doctests but reported 2", summary.doc_errors(short)[0])
            self.assertIn("no doctest result line", summary.doc_errors(empty)[0])

    def test_missing_report_is_not_reported_as_success(self):
        with self.assertRaises(FileNotFoundError):
            summary.junit_results("does-not-exist.xml")

    def test_malformed_report_is_not_reported_as_success(self):
        with self.assertRaises(summary.ET.ParseError):
            self.parse('<testsuites>', summary.junit_results)


if __name__ == "__main__":
    unittest.main()
