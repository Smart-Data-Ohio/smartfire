"""Fail closed unless a healthy control reaches the mutation's exact assertion."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SUMMARY = re.compile(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
PANIC = re.compile(
    r"thread '([^']+)'[^\n]* panicked at ([^\n]+):\d+:\d+:\n([^\n]*)"
    r"(?:\n\s*left: ([^\n]+)\n\s*right: ([^\n]+))?"
)


def run_tests(test, extra_env=None):
    env = {**os.environ, "CI": "1", "CARGO_BUILD_JOBS": "2", "RUST_TEST_THREADS": "8"}
    # A caller's mutation must never contaminate the control.
    env.pop("WS11UI_INBOX_LIFECYCLE_DEFECT", None)
    env.update(extra_env or {})
    return subprocess.run(
        ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked",
         "-p", "campfire", "--bin", "campfire", test,
         "--", "--test-threads=8", "--nocapture"],
        cwd=ROOT, env=env, capture_output=True, text=True,
    )


def output_of(result):
    return result.stdout + result.stderr


def require_baseline(test, count=1):
    result = run_tests(test)
    output = output_of(result)
    if result.returncode != 0 or SUMMARY.findall(output) != [("ok", str(count), "0", "0")]:
        raise RuntimeError(f"Invalid discrimination: unmutated baseline failed for {test}\n{output}")
    print(f"Discrimination baseline: {test}: {count} passed; 0 failed", flush=True)


def require_rejected(result, assertions, passed=0):
    """Map failed tests to (source, message[, (actual, expected) assertion values])."""
    output = output_of(result)
    failure_section = output.split("failures:\n")[-1].split("\ntest result:", 1)[0]
    failures = re.findall(r"^    (\S+)[ \t]*$", failure_section, re.M)
    expected = set(assertions)
    actual = {name.rsplit("::", 1)[-1] for name in failures}
    panics = PANIC.findall(output)
    valid = (
        result.returncode == 101
        and SUMMARY.findall(output) == [("FAILED", str(passed), str(len(assertions)), "0")]
        and actual == expected
        and len(panics) == len(assertions)
    )
    for test, (source, message, *values) in assertions.items():
        valid = valid and any(
            thread.rsplit("::", 1)[-1] == test and path == source and message in detail
            and (not values or (left, right) == values[0])
            for thread, path, detail, left, right in panics
        )
    if not valid:
        raise RuntimeError(f"Invalid discrimination: mutation did not fail at its intended assertion\n{output}")
    print(SUMMARY.search(output).group(0), flush=True)
    return output
