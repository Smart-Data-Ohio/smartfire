#!/usr/bin/env python3
"""Validate broader named API credits; optionally require actual cargo passes."""
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("ports", Path(__file__).with_name("check-case-ports.py"))
ports = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ports)


def check(log=None):
    ledger = json.loads((ROOT / "rust/plans/ws11api-named-api-cases.json").read_text())
    audit = json.loads((ROOT / ledger["audit_source"]).read_text())
    audited = [(c["file"], c["test"]) for c in audit["cases"]
               if c["owner"].startswith("WS11-API")]
    assert audited and len(set(audited)) == len(audited), "duplicate or empty audited inventory"
    declared = {(c["rails_file"], c["rails_test"]) for c in ledger["cases"]}
    assert declared == set(audited), (
        "audited inventory mismatch", "missing", set(audited) - declared,
        "extra", declared - set(audited))
    statuses = [] if log is None else re.findall(
        r"^test (\S+) \.\.\. (ok|FAILED|ignored)$", log, re.M)
    seen = set()
    seen_tests = set()
    seen_vectors = set()
    seen_keys = set()
    pinned_files = {}
    passed = 0
    for case in ledger["cases"]:
        identity = (case["rails_file"], case["rails_test"])
        assert identity not in seen, identity
        seen.add(identity)
        if case["rails_file"] not in pinned_files:
            pinned_files[case["rails_file"]] = subprocess.check_output(
                ["git", "show", ledger["reference"] + ":" + case["rails_file"]], cwd=ROOT, text=True)
        pinned = pinned_files[case["rails_file"]]
        assert (ROOT / case["rails_file"]).read_text() == pinned, case["rails_file"]
        assert case["rails_test"] in re.findall(r'^\s*test "(.*?)" do', pinned, re.M), identity
        assert case["status"] in {"passed", "pending"}, identity
        if case["status"] == "pending":
            assert isinstance(case.get("reason"), str) and case["reason"].strip(), identity
            continue
        assert case["rust_test"] not in seen_tests, (identity, "reused Rust test")
        seen_tests.add(case["rust_test"])
        vector_reference = ((ROOT / case["vector"]).resolve(), case["key"])
        assert vector_reference not in seen_vectors and case["key"] not in seen_keys, (identity, "reused vector key")
        seen_vectors.add(vector_reference)
        seen_keys.add(case["key"])
        rust = (ROOT / case["rust_file"]).read_text()
        assert case["rust_test"] in ports.rust_ports(rust), identity
        # These cases! invocations emit one test calling run(the vector key).
        mappings = [pair for invocation in re.findall(r'\bcases!\s*\{([^}]+)\}', rust, re.S)
                    for pair in re.findall(r'(\w+)\s*=>\s*"([^"]+)"', invocation)]
        assert [key for test, key in mappings if test == case["rust_test"]] == [case["key"]], (identity, "test/vector mismatch")
        assert [test for test, key in mappings if key == case["key"]] == [case["rust_test"]], (identity, "vector/test mismatch")
        vector = json.loads((ROOT / case["vector"]).read_text())
        rows = [r for r in vector["cases"] if r["key"] == case["key"]]
        assert len(rows) == 1, identity
        assert (rows[0]["rails_file"], rows[0]["rails_test"]) == identity, identity
        assert rows[0]["observations"] and rows[0]["state"], identity
        producer = json.loads((ROOT / case["producer_control"]).read_text())
        declarations = [c for c in producer["declarations"] if c["key"] == case["key"]]
        controls = json.loads((ROOT / case["receipt"]).read_text())
        receipts = [r for r in controls["named_api"] if r["key"] == case["key"]]
        assert len(declarations) == len(receipts) == 1, identity
        declaration, receipt = declarations[0], receipts[0]
        for evidence in (declaration, receipt):
            assert (evidence["rails_file"], evidence["rails_test"]) == identity, identity
            assert evidence["rust_test"] == case["rust_test"], (identity, "control/test mismatch")
            assert evidence["mutation"] == rows[0]["mutation"], identity
        assert receipt["intended_assertion"] == declaration["intended_assertion"], identity
        assert receipt["exit"] and receipt["hits"] and receipt["intended_assertion_rejected"], identity
        assert len(receipt["summaries"]) == 1 and "0 passed; 1 failed; 0 ignored" in receipt["summaries"][0], identity
        if log is not None:
            matches = [status for test, status in statuses if test.rsplit("::", 1)[-1] == case["rust_test"]]
            assert matches == ["ok"], (identity, "missing or ambiguous test pass", matches)
        passed += 1
    print(f"WS11 broader named API assertions: {passed} passed; {len(seen)-passed} pending; owner WS11-API (unblocked)")
    next6_checker = Path(__file__).with_name('check-next6-assertions.py')
    if next6_checker.is_file():
        subprocess.run(['python3', str(next6_checker)], cwd=ROOT, check=True)


if __name__ == "__main__":
    check(Path(sys.argv[1]).read_text() if len(sys.argv) > 1 else None)
