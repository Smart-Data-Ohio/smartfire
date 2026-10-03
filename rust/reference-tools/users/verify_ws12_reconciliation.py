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
parser.add_argument("--ledger", type=Path, default=ROOT / "rust/plans/ws12-assertion-reconciliation.json")
args = parser.parse_args()
ledger = json.loads(args.ledger.read_text())
catalog = json.loads((ROOT / "rust/plans/ws12-assertion-mutations.json").read_text())
recipes = {m["key"]: m for m in catalog["mutations"]}
controls = {r["id"]: r for r in catalog["declarations"]}
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
    control = row.get("mutation_control")
    assert control, f"no producer-mutation receipt: {row['test']}"
    planned = controls[control["id"]]
    assert (planned["file"], planned["test"]) == (row["file"], row["test"])
    assert planned["tests"] == row["tests"], f"different mutation and credited tests: {row['test']}"
    assert control["recipe"] == recipes[planned["mutation"]], "producer recipe drift"
    assert control["outcome"] == "rejected_at_declared_assertion", f"unsupported mutation result: {row['test']}"
    assert control["hits"] > 0 and control["intended_assertion"], "producer not exercised or assertion not reviewed"
    assert control["baseline"] and all(group["exit"] == 0 for group in control["baseline"]), "baseline must pass"
    assert any(group["exit"] != 0 and group["assertion_site"] and group["hits"] > 0 for group in control["mutant"]), "no activated assertion failure"
    for edit in control["recipe"]["edits"]:
        assert (ROOT / edit["file"]).read_text().count(edit["before"]) == edit["occurrences"], f"producer anchor drift: {edit['file']}"
    for test in row["tests"]:
        path = ROOT / test["rust_file"]
        name = test["rust_test"]
        assert path.is_file() and re.search(r"\b" + re.escape(name) + r"\b", path.read_text()), f"missing Rust assertion: {test}"
        assert any(p.rsplit("::", 1)[-1] == name for p in passed), f"Rust assertion did not pass in receipt: {test}"
counts = Counter(r["reconciliation"] for r in rows)
print(f'WS12 assertion reconciliation: {len(rows)} declarations; {counts["mapped"]} executed mutation-backed mappings; {counts["flagged"]} explicitly flagged; 0 missing or non-running credited tests')
