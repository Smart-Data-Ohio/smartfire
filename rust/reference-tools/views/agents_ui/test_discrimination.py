"""Regression controls for unrelated failures after a healthy baseline."""
from contextlib import redirect_stdout
import io
import subprocess
import unittest
from unittest.mock import patch

from discrimination import require_baseline, require_rejected
from check_runtime import assertions as runtime_assertions


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

    def test_presence_control_rejects_heartbeat_and_transport_failures(self):
        filename, expected = runtime_assertions["presence-broadcast"]
        source = "crates/campfire/src/controllers/presenters/accounts/tests/" + filename
        for detail in [
            "working presence incorrectly broadcast a status callback",
            "working presence silence is invalid: cable silence transport ended",
            'working presence silence invalid unexpected application frame: {"type":"ping"}',
        ]:
            with self.subTest(detail=detail), self.assertRaises(RuntimeError):
                require_rejected(failure(detail, source=source), {TEST: (source, expected)})
        with redirect_stdout(io.StringIO()):
            require_rejected(failure(expected, source=source), {TEST: (source, expected)})

    def test_extra_background_panic_is_invalid(self):
        result = failure()
        result.stderr += f"thread 'db' (2) panicked at {SOURCE}:1:1:\nnetwork failure\n"
        with self.assertRaises(RuntimeError):
            require_rejected(result, {TEST: (SOURCE, MESSAGE)})

    def test_compiler_warning_gutters_are_not_failed_test_names(self):
        result = failure()
        result.stderr = "warning: function is never used\n    |\n    | fn unused() {}\n" + result.stderr
        with redirect_stdout(io.StringIO()):
            require_rejected(result, {TEST: (SOURCE, MESSAGE)})

    def test_each_assertion_must_belong_to_its_own_test(self):
        result = failure(message="second assertion")
        result.stderr += f"thread 'example::second_test' (2) panicked at {SOURCE}:1:1:\n{MESSAGE}\n"
        result.stdout = f"failures:\n    example::{TEST}\n    example::second_test\n\ntest result: FAILED. 0 passed; 2 failed; 0 ignored;\n"
        with self.assertRaises(RuntimeError):
            require_rejected(result, {
                TEST: (SOURCE, MESSAGE), "second_test": (SOURCE, "second assertion"),
            })

    def test_unrelated_status_failure_at_the_same_assertion_is_invalid(self):
        for status in ["500", "406"]:
            result = failure()
            result.stderr += f"  left: {status}\n right: 303\n"
            assertions = {TEST: (SOURCE, MESSAGE, ("406", "303"))}
            if status == "500":
                with self.assertRaises(RuntimeError):
                    require_rejected(result, assertions)
            else:
                with redirect_stdout(io.StringIO()):
                    require_rejected(result, assertions)

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
