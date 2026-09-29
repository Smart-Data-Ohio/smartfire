#!/usr/bin/env python3
"""Audit the broadcast source index against our Rails, without an allowlist."""
import argparse
import os
from pathlib import Path
import re
import subprocess

PRIMITIVE = re.compile(
    r"\bbroadcast_(?:append|prepend|replace|update|remove|before|after|refresh|action|render)(?:_later)?_to\b"
    r"|\bActionCable\.server\.broadcast\b|\b\w*Channel\.broadcast_to\b"
    r"|(?<![\w.])broadcast_to\b"
)
STATES = {"ported", "API ready", "waiting on its domain", "dead"}


def source_sites(root):
    files = subprocess.check_output(["git", "ls-files", "app", "lib", "config"], cwd=root, text=True).splitlines()
    sites = []
    for file in files:
        if not file.endswith(".rb"):
            continue
        for number, line in enumerate((root / file).read_text().splitlines(), 1):
            if line.lstrip().startswith("#"):
                continue
            for match in PRIMITIVE.finditer(line):
                if line[:match.start()].rstrip().endswith("def"):
                    continue
                sites.append(f"{file}:{number}")
    return sorted(sites)


def index_rows(document):
    index = document.split("## Source index\n", 1)[1]
    rows = []
    for line in index.splitlines():
        match = re.fullmatch(r"\| `([^`]+)` \| (.+) \| (.+) \|", line)
        if match:
            rows.append(match.groups())
    return rows


def check(sites, rows):
    errors = []
    indexed = [row[0] for row in rows]
    if sorted(sites) != sorted(indexed):
        errors.append(f"missing={sorted(set(sites) - set(indexed))}, extra={sorted(set(indexed) - set(sites))}")
    if len(set(indexed)) != len(indexed):
        errors.append("duplicate source locations")
    for site, owner, state in rows:
        if state not in STATES or not re.fullmatch(r"WS\d+(?:/WS\d+)*", owner):
            errors.append(f"invalid owner/state at {site}: {owner}/{state}")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(os.environ.get("CAMPFIRE_REFERENCE", Path(__file__).resolve().parents[3])))
    parser.add_argument("--check", action="store_true", required=True)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    contract = Path(__file__).resolve().parents[2] / "crates/cable/BROADCASTS.md"
    sites, rows = source_sites(args.root), index_rows(contract.read_text())
    errors = check(sites, rows)
    if errors:
        raise SystemExit("broadcast contract FAILED: " + "; ".join(errors))
    print(f"broadcast contract: {len(sites)} source calls, {len(rows)} indexed, 0 missing, 0 extra")
    if args.self_test:
        assert check(sites, rows[1:]), "an omitted call must fail"
        assert check(sites, rows + [rows[0]]), "a duplicate call must fail"
        assert check(sites + ["app/new.rb:1"], rows), "a new source call must fail"
        assert check(sites, [(rows[0][0], "unknown", "ported")] + rows[1:]), "an unknown owner must fail"
        print("broadcast contract injection tests: 4 passed; 0 failed")


if __name__ == "__main__":
    main()
