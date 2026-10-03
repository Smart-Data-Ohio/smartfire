#!/usr/bin/env python3
"""Reject forged coverage credit against an actual passed workspace receipt."""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--test-log", type=Path, required=True)
parser.add_argument("--scratch", type=Path, required=True)
args = parser.parse_args()
args.scratch.mkdir(parents=True, exist_ok=True)
ledger = json.loads((ROOT / "rust/plans/ws12-assertion-reconciliation.json").read_text())
index = next(i for i, row in enumerate(ledger["cases"]) if row["reconciliation"] == "mapped")
command = ["python3", str(ROOT / "rust/reference-tools/users/verify_ws12_reconciliation.py"),
           "--test-log", str(args.test_log.resolve())]
baseline = subprocess.run(command, text=True, capture_output=True, check=True)
print(baseline.stdout, end="")
for name, expected in [("missing receipt", "no producer-mutation receipt"),
                       ("surviving control", "unsupported mutation result"),
                       ("unexercised producer", "producer not exercised")]:
    changed = copy.deepcopy(ledger)
    row = changed["cases"][index]
    if name == "missing receipt":
        del row["mutation_control"]
    elif name == "surviving control":
        row["mutation_control"]["outcome"] = "survived"
    else:
        row["mutation_control"]["hits"] = 0
    with tempfile.TemporaryDirectory(dir=args.scratch) as directory:
        path = Path(directory) / "forged-ledger.json"
        path.write_text(json.dumps(changed))
        result = subprocess.run([*command, "--ledger", str(path)], text=True, capture_output=True)
    assert result.returncode != 0 and expected in result.stderr, (name, result.stderr)
    print(f"WS12_LEDGER_NEGATIVE_CONTROL {name}: rejected at {expected}")
print("WS12_LEDGER_NEGATIVE_CONTROLS 3 rejected; 0 false credits")
