#!/usr/bin/env python3
"""Check the explicit ledger against source hashes and real passed workspace test receipts."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--test-log", type=Path, required=True)
args = parser.parse_args()
ledger = json.loads((ROOT / "rust/plans/ws12-assertion-reconciliation.json").read_text())
rows = ledger["cases"]
assert len(rows) == 231, "retain all 231 formerly deferred declarations"
assert len({(r["file"], r["test"]) for r in rows}) == 231
passed = set(re.findall(r"^test ([\w:]+) \.\.\. ok$", args.test_log.read_text(), re.M))
assert passed, "no executed test receipts"
for row in rows:
    source = ROOT / row["file"]
    assert hashlib.sha256(source.read_bytes()).hexdigest() == row["rails_sha256"], f"Rails source drift: {source}"
    assert f'test "{row["test"]}" do' in source.read_text(), f"missing declaration: {row}"
    if row["reconciliation"] == "flagged":
        assert not row["tests"] and row["owner"] and row["evidence"]
        continue
    assert row["reconciliation"] == "mapped" and row["tests"]
    for test in row["tests"]:
        path = ROOT / test["rust_file"]
        name = test["rust_test"]
        assert path.is_file() and re.search(r"\b" + re.escape(name) + r"\b", path.read_text()), f"missing Rust assertion: {test}"
        assert any(p.rsplit("::", 1)[-1] == name for p in passed), f"Rust assertion did not pass in receipt: {test}"
counts = Counter(r["reconciliation"] for r in rows)
print(f'WS12 assertion reconciliation: {len(rows)} declarations; {counts["mapped"]} executed mappings; {counts["flagged"]} explicitly flagged; 0 missing or non-running credited tests')
