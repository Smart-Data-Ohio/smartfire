"""Regression controls for unrelated failures after a healthy baseline."""
from contextlib import redirect_stdout
import io
import subprocess
import unittest
from unittest.mock import patch

from discrimination import require_baseline, require_rejected


TEST = "intended_test"
SOURCE = "crates/campfire/src/controllers/example.rs"
MESSAGE = "intended writer assertion"


def failure(message=MESSAGE, source=SOURCE, test=TEST, code=101):
    stdout = f"failures:\n    example::{test}\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored;\n"
    stderr = f"thread 'example::{test}' (1) panicked at {source}:42:9:\n{message}\n"
    return subprocess.CompletedProcess([], code, stdout, stderr)


class DiscriminationTests(unittest.TestCase):
    def test_rejects_only_the_intended_assertion(self):
        with redirect_stdout(io.StringIO()):
            require_rejected(failure(), {TEST: (SOURCE, MESSAGE)})

    def test_unrelated_single_failures_are_invalid(self):
        for result in [
            failure("CI requires parity/.seed/default"),
            failure("network connection refused"),
            failure("called Option::unwrap() on a None value"),
            failure(source="crates/campfire/src/app.rs"),
            failure(test="unrelated_test"),
            failure(code=1),
        ]:
            with self.subTest(result=result), self.assertRaises(RuntimeError):
                require_rejected(result, {TEST: (SOURCE, MESSAGE)})

    def test_extra_background_panic_is_invalid(self):
        result = failure()
        result.stderr += f"thread 'db' (2) panicked at {SOURCE}:1:1:\nnetwork failure\n"
        with self.assertRaises(RuntimeError):
            require_rejected(result, {TEST: (SOURCE, MESSAGE)})

    def test_baseline_must_pass_the_expected_test_count(self):
        for result in [
            failure(),
            subprocess.CompletedProcess([], 0, "test result: ok. 0 passed; 0 failed; 0 ignored;\n", ""),
            subprocess.CompletedProcess([], 0, "test result: ok. 1 passed; 0 failed; 1 ignored;\n", ""),
        ]:
            with patch("discrimination.run_tests", return_value=result), self.assertRaises(RuntimeError):
                require_baseline(TEST)


if __name__ == "__main__":
    unittest.main()
