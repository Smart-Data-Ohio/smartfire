#!/usr/bin/env python3
"""Keep the workspace summary's counts/outcome across nextest and libtest doctests."""

import argparse
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
    args = parser.parse_args()
    counts = [0, 0, 0]
    failures = []
    for parse, paths in ((junit_results, args.junit), (doc_results, args.doc_log)):
        for path in paths:
            result, details = parse(path)
            counts = [a + b for a, b in zip(counts, result)]
            failures.extend(details)
    passed, failed, ignored = counts
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


if __name__ == "__main__":
    main()
