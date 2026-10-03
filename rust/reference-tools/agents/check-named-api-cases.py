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
    receipt_files = {}
    statuses = {} if log is None else dict(re.findall(r"^test \S*::(\w+) \.\.\. (ok|FAILED|ignored)$", log, re.M))
    seen = set()
    passed = 0
    for case in ledger["cases"]:
        identity = (case["rails_file"], case["rails_test"])
        assert identity not in seen, identity
        seen.add(identity)
        pinned = subprocess.check_output(["git", "show", ledger["reference"] + ":" + case["rails_file"]], cwd=ROOT, text=True)
        assert (ROOT / case["rails_file"]).read_text() == pinned, case["rails_file"]
        assert case["rails_test"] in re.findall(r'^\s*test "(.*?)" do', pinned, re.M), identity
        assert case["status"] in {"passed", "pending"}, identity
        if case["status"] == "pending":
            continue
        assert case["rust_test"] in ports.rust_ports((ROOT / case["rust_file"]).read_text()), identity
        vector = json.loads((ROOT / case["vector"]).read_text())
        rows = [r for r in vector["cases"] if r["key"] == case["key"]]
        assert len(rows) == 1 and rows[0]["rails_test"] == case["rails_test"], identity
        assert rows[0]["observations"] and rows[0]["state"], identity
        receipt_file = case["receipt"]
        if receipt_file not in receipt_files:
            controls = json.loads((ROOT / receipt_file).read_text())
            receipt_files[receipt_file] = {r["key"]: r for r in controls["named_api"]}
        receipt = receipt_files[receipt_file][case["key"]]
        assert receipt["exit"] and receipt["hits"] and receipt["intended_assertion_rejected"], identity
        assert "0 passed; 1 failed; 0 ignored" in receipt["summaries"][0], identity
        if log is not None:
            assert statuses.get(case["rust_test"]) == "ok", identity
        passed += 1
    print(f"WS11 broader named API assertions: {passed} passed; {len(seen)-passed} pending; owner WS11-API (unblocked)")
    next6_checker = Path(__file__).with_name('check-next6-assertions.py')
    if next6_checker.is_file():
        subprocess.run(['python3', str(next6_checker)], cwd=ROOT, check=True)


if __name__ == "__main__":
    check(Path(sys.argv[1]).read_text() if len(sys.argv) > 1 else None)
