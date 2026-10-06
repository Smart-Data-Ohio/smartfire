#!/usr/bin/env python3
"""Keep the workspace summary's counts/outcome across nextest and libtest doctests.

With --expect LIST JUNIT..., the JUnit receipts must hold exactly the tests `cargo nextest list
--message-format json` selected for that filter set: each matching test passed exactly once
across the receipts, and every #[ignore] test reported as ignored. Every doctest log must account
for each doctest libtest said it was running, with none failed. Any mismatch fails the check.
"""

import argparse
import json
import os
from pathlib import Path
import re
import xml.etree.ElementTree as ET


def junit_results(path):
    passed = failed = ignored = 0
    failures = []
    for suite in ET.parse(path).getroot().iter("testsuite"):
        for case in suite.findall("testcase"):
            if case.find("skipped") is not None:
                ignored += 1
            elif case.find("failure") is not None or case.find("error") is not None:
                failed += 1
                failures.append(f"{suite.attrib['name']}: {case.attrib['name']}")
            else:
                passed += 1
    return (passed, failed, ignored), failures


def skipped_cases(path):
    """Ignored tests by (binary, name): nextest partitions (CI shards) each report all of them."""
    return {(suite.attrib["name"], case.attrib["name"])
            for suite in ET.parse(path).getroot().iter("testsuite")
            for case in suite.findall("testcase") if case.find("skipped") is not None}


def listed_tests(path):
    """(tests the filter selects, #[ignore] tests nextest reports as skipped) from a list."""
    selected, ignored = set(), set()
    for binary_id, suite in json.loads(Path(path).read_text())["rust-suites"].items():
        for name, case in suite.get("testcases", {}).items():
            match = case["filter-match"]
            if match["status"] == "matches":
                selected.add((binary_id, name))
            elif match.get("reason") == "ignored":
                ignored.add((binary_id, name))
    return selected, ignored


def junit_cases(path):
    passed, skipped = [], set()
    for suite in ET.parse(path).getroot().iter("testsuite"):
        for case in suite.findall("testcase"):
            key = (suite.attrib["name"], case.attrib["name"])
            if case.find("skipped") is not None:
                skipped.add(key)
            elif case.find("failure") is None and case.find("error") is None:
                passed.append(key)
    return passed, skipped


def expectation_errors(list_path, junit_paths):
    """Differences between the tests a list selected and the receipts that ran them."""
    selected, ignored = listed_tests(list_path)
    if not selected:
        return [f"{Path(list_path).name}: the filter selected no tests"]
    passed, skipped = [], set()
    for path in junit_paths:
        cases, skips = junit_cases(path)
        passed.extend(cases)
        skipped |= skips
    errors = []
    name = Path(list_path).name
    missing, extra = selected - set(passed), set(passed) - selected
    duplicated = len(passed) - len(set(passed))
    if missing:
        errors.append(f"{name}: {len(missing)} selected tests did not pass, e.g. {sorted(missing)[:3]}")
    if extra:
        errors.append(f"{name}: {len(extra)} passed tests were not selected, e.g. {sorted(extra)[:3]}")
    if duplicated:
        errors.append(f"{name}: {duplicated} tests passed in more than one receipt")
    if skipped != ignored:
        errors.append(f"{name}: {len(ignored)} ignored tests listed, {len(skipped)} reported "
                      f"(missing {sorted(ignored - skipped)[:3]}, unexpected {sorted(skipped - ignored)[:3]})")
    print(f"{name}: {len(selected)} selected and {len(ignored)} ignored tests listed; "
          f"{len(set(passed) & selected)} passed and {len(skipped & ignored)} ignored in "
          f"{len(junit_paths)} receipt(s)")
    return errors


def doc_errors(path):
    """libtest's own count ("running N tests") against its result lines, with no failures."""
    text = re.sub(r"\x1b\[[0-9;]*m", "", Path(path).read_text())
    running = sum(int(n) for n in re.findall(r"^running (\d+) tests?$", text, re.M))
    results = re.findall(
        r"^test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured;", text, re.M
    )
    passed, failed, ignored, measured = (sum(int(r[i]) for r in results) for i in range(4))
    name = Path(path).name
    if not results:
        return [f"{name}: no doctest result line"]
    errors = []
    if failed:
        errors.append(f"{name}: {failed} doctests failed")
    if passed + failed + ignored + measured != running:
        errors.append(f"{name}: libtest ran {running} doctests but reported "
                      f"{passed + failed + ignored + measured}")
    return errors


def doc_results(path):
    text = re.sub(r"\x1b\[[0-9;]*m", "", Path(path).read_text())
    counts = [0, 0, 0]
    for result in re.finditer(
        r"^test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M
    ):
        counts = [a + int(b) for a, b in zip(counts, result.groups())]
    # Keep libtest's final failing-target diagnostic for doctests, as the old summary did.
    failures = re.findall(r"^error: \d+ targets? failed[^\n]*(?:\n[^\n]*){0,20}", text, re.M)
    return counts, failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--junit", action="append", default=[])
    parser.add_argument("--doc-log", action="append", default=[])
    parser.add_argument("--expect", nargs="+", action="append", default=[], metavar=("LIST", "JUNIT"),
                        help="a nextest list (JSON) and the JUnit receipts that must match it")
    args = parser.parse_args()
    errors = []
    for list_path, *junit_paths in args.expect:
        errors.extend(expectation_errors(list_path, junit_paths))
    if args.expect:
        for path in args.doc_log:
            errors.extend(doc_errors(path))
    counts = [0, 0, 0]
    failures = []
    rows = []
    for parse, paths in ((junit_results, args.junit), (doc_results, args.doc_log)):
        for path in paths:
            result, details = parse(path)
            counts = [a + b for a, b in zip(counts, result)]
            failures.extend(details)
            rows.append(f"{Path(path).name}: {result[0]} passed, {result[1]} failed, {result[2]} ignored")
    # Count each ignored nextest test once, however many shards' receipts list it.
    skipped = set().union(*map(skipped_cases, args.junit))
    junit_ignored = sum(junit_results(path)[0][2] for path in args.junit)
    counts[2] -= junit_ignored - len(skipped)
    passed, failed, ignored = counts
    print("\n".join(rows))
    print(f"Total: {passed} passed, {failed} failed, {ignored} ignored (distinct)")
    outcome = "success" if all(
        os.environ[key] == "success" for key in ("TEST_OUTCOME", "DOC_OUTCOME")
    ) else "failure"
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
        summary.write("### Rust tests (non-blocking)\n\n")
        summary.write(
            f"Outcome: **{outcome}**. {passed} passed, {failed} failed, {ignored} ignored.\n\n"
        )
        summary.write("```\n" + ("\n".join(failures) or "no failing targets") + "\n```\n")
    if outcome != "success":
        print(
            f"::warning title=Rust tests (non-blocking)::{failed} failed, {passed} passed; "
            "see the Tests and Doctests steps' logs"
        )
    for error in errors:
        print(f"::error title=Rust test count::{error}")
    if errors:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
