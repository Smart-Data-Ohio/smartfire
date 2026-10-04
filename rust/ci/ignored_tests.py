#!/usr/bin/env python3
"""Fail closed on ignored correctness tests without an exact CI owner/selector."""
import argparse
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent


def inventory(root):
    found = {}
    # Test attributes in source, including tools-only host copies. Never scan generated targets.
    for directory in (root / "crates", root / "reference-tools"):
        for path in directory.rglob("*.rs"):
            source = re.sub(r"/\*.*?\*/", "", path.read_text(), flags=re.S)
            source = re.sub(r"(?m)^\s*//.*$", "", source)
            for match in re.finditer(r'^\s*#\[ignore(?:\s*=\s*"([^"\n]*)")?\]', source, re.M):
                function = re.match(r'\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)', source[match.end():])
                if not function:
                    raise ValueError(f"cannot resolve ignored function in {path}")
                key = (path.relative_to(root).as_posix(), function[1])
                if key in found:
                    raise ValueError(f"duplicate ignored function: {key}")
                found[key] = match[1] or ""
    return found


def check(root, manifest, workflow):
    found = inventory(root)
    owners = {}
    suites = re.search(r"suite: \[([^\]]+)\]", workflow)
    if not suites or 'bash rust/ci/correctness.sh "$SUITE"' not in workflow:
        raise ValueError("correctness runner is not invoked by the workflow")
    jobs = {name.strip() for name in suites[1].split(",")}
    for suite, records in manifest.items():
        if suite not in jobs:
            raise ValueError(f"missing CI job for ignored suite: {suite}")
        for record in records:
            key = (record["path"], record["test"].split("::")[-1])
            if key in owners:
                raise ValueError(f"multiple CI owners: {key}")
            if key not in found or found[key].startswith("utility:"):
                raise ValueError(f"stale correctness selector: {key}")
            owners[key] = suite
    for key, reason in found.items():
        if not reason.startswith("utility:") and key not in owners:
            raise ValueError(f"ignored correctness test has no CI job: {key}")
        if reason.startswith("utility:") and not reason.removeprefix("utility:").strip():
            raise ValueError(f"utility needs a reason: {key}")
    return len(owners), len(found) - len(owners)


def expression(records):
    return " or ".join(f'(package(={r["package"]}) and binary(={r["binary"]}) and test(={r["test"]}))' for r in records)


def verify_junit(records, path):
    cases = ET.parse(path).getroot().findall(".//testcase")
    actual = [case.attrib["name"] for case in cases if case.find("skipped") is None]
    expected = sorted(record["test"] for record in records)
    if sorted(actual) != expected or any(case.find("failure") is not None or case.find("error") is not None for case in cases):
        raise ValueError(f"expected exactly {expected}; executed {actual}; see {path}")
    print(f"Ignored correctness receipt: {len(actual)} passed, 0 failed, 0 selected tests skipped")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--filter", choices=["database", "acme", "browsers", "livekit"])
    parser.add_argument("--junit", type=Path)
    args = parser.parse_args()
    manifest = json.loads((ROOT / "ci/ignored-tests.json").read_text())
    correctness, utilities = check(ROOT, manifest, (ROOT.parent / ".github/workflows/rust.yml").read_text())
    if args.junit:
        if not args.filter:
            parser.error("--junit needs --filter")
        verify_junit(manifest[args.filter], args.junit)
    elif args.filter:
        print(expression(manifest[args.filter]))
    else:
        print(f"Ignored-test guard: {correctness} CI correctness tests, {utilities} utilities; 0 unowned")
