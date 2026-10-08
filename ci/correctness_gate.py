#!/usr/bin/env python3
"""Check the receipts of every correctness job, including sharded ones, as one gate.

Usage: correctness_gate.py RECEIPTS_DIR [--scope full|messaging-and-browsers] [--head SHA]

Each job uploads target/ci-receipts (correctness.sh): TAG.json with its exit code, the
nextest JUnit for its ignored selection, and for messaging behaviour shards the named cases
it ran. This requires that every job exited 0 on the expected commit, that for each suite
with ignored tests the shards' JUnit receipts together contain exactly the registry's
selection (each test once, all passed), and that the behaviour shards covered every named
case exactly once.
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path

from ignored_tests import ROOT, verify_junit

SCOPES = {
    "full": ["acme", "drive", "browsers", "livekit", "messaging", "agents-ui", "pwa"],
    "messaging-and-browsers": ["browsers", "drive", "messaging"],
}


def suite_files(receipts, suite, suffix):
    """Unsharded TAG is the suite; sharded and part tags are suite-<...>."""
    return sorted(path for path in receipts.rglob(f"*{suffix}")
                  if path.name == f"{suite}{suffix}" or path.name.startswith(f"{suite}-"))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("receipts", type=Path)
    parser.add_argument("--scope", choices=sorted(SCOPES), default="full")
    parser.add_argument("--head")
    args = parser.parse_args(argv)
    sys.stdout.reconfigure(line_buffering=True)
    manifest = json.loads((ROOT / "ci/ignored-tests.json").read_text())
    suites = SCOPES[args.scope]

    for suite in suites:
        jobs = [path for path in suite_files(args.receipts, suite, ".json")
                if not path.name.endswith("-cases.json")]
        if not jobs:
            raise SystemExit(f"correctness gate: no {suite} job receipt")
        for path in jobs:
            receipt = json.loads(path.read_text())
            if receipt["suite"] != suite or receipt["exit_code"] != 0:
                raise SystemExit(f"correctness gate: {path.name} reports exit code {receipt['exit_code']}")
            if args.head and receipt["head"] != args.head:
                raise SystemExit(f"correctness gate: {path.name} tested {receipt['head']}, expected {args.head}")
        print(f"{suite}: {len(jobs)} job receipt(s), all exit 0")

        if suite in manifest:
            junits = suite_files(args.receipts, suite, "-junit.xml")
            if not junits:
                raise SystemExit(f"correctness gate: no {suite} JUnit receipt")
            print(f"{suite}: {len(junits)} JUnit receipt(s)", end=": ")
            verify_junit(manifest[suite], junits)

    if "messaging" in suites:
        cases = suite_files(args.receipts, "messaging", "-cases.json")
        if not cases:
            raise SystemExit("correctness gate: no messaging behaviour receipts")
        verified = subprocess.run([sys.executable, str(ROOT / "reference-tools/messaging/behavior-check.py"),
                                   "--verify-receipts", *map(str, cases)])
        if verified.returncode:
            raise SystemExit("correctness gate: the behaviour shards' receipts do not cover one passing run")


if __name__ == "__main__":
    main()
