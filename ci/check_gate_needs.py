#!/usr/bin/env python3
"""Fail unless every job in rust.yml is gated: each job other than the gates themselves and the
change detection they read must appear in the `needs` of `Rust port` (job `rust`) or
`Rust correctness` (job `correctness-gate`), so a job added later can't pass unnoticed.

Reads the workflow's block structure directly (job ids are the keys one level under `jobs:`,
with flow-list, scalar or block-list `needs`), so it runs on a bare runner without PyYAML.

Usage: check_gate_needs.py [WORKFLOW]   (default: .github/workflows/rust.yml)
"""
import re
import sys
from pathlib import Path

GATES = ("rust", "correctness-gate")
UNGATED = {"changes", *GATES}
KEY = re.compile(r"^( *)([A-Za-z0-9_-]+):(.*)$")


def name(value):
    return value.split("#")[0].strip().strip("'\"")


def jobs(text):
    """{job id: list of its needs} from the top-level `jobs:` mapping."""
    lines = text.splitlines()
    start = next(i for i, line in enumerate(lines) if re.match(r"^jobs:\s*(#.*)?$", line))
    indent, found, current, in_needs = None, {}, None, None
    for line in lines[start + 1:]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        depth = len(line) - len(line.lstrip(" "))
        if depth == 0:
            break
        indent = indent or depth
        key = KEY.match(line)
        if depth == indent:
            if not key or key.group(3).strip():
                raise ValueError(f"unexpected line in jobs: {line!r}")
            current, in_needs = key.group(2), None
            found[current] = []
            continue
        if in_needs is not None and depth > in_needs and line.lstrip().startswith("- "):
            found[current].append(name(line.lstrip()[2:]))
            continue
        in_needs = None
        if key and depth == 2 * indent and key.group(2) == "needs":
            value = key.group(3).split("#")[0].strip()
            if value.startswith("["):
                found[current] += [name(n) for n in value.strip("[]").split(",") if name(n)]
            elif value:
                found[current].append(name(value))
            else:
                in_needs = depth
    return found


def problems(text):
    found = jobs(text)
    missing = [gate for gate in GATES if gate not in found]
    if missing:
        return [f"gate job {gate!r} not found" for gate in missing]
    gated = set().union(*(found[gate] for gate in GATES))
    return [f"job {job!r} is not in the needs of {' or '.join(GATES)}"
            for job in found if job not in UNGATED and job not in gated]


def main(argv):
    path = Path(argv[0] if argv else ".github/workflows/rust.yml")
    errors = problems(path.read_text())
    for error in errors:
        print(f"::error title=Ungated Rust job::{error}")
    if errors:
        sys.exit(1)
    print(f"Every job in {path} is gated by {' or '.join(GATES)}.")


if __name__ == "__main__":
    main(sys.argv[1:])
