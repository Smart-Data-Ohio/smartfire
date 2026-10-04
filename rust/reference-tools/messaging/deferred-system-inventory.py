#!/usr/bin/env python3
"""Verify named behaviour attribution against the pin; never claim a pixel pass."""
from pathlib import Path
import re
import subprocess
import json
import hashlib
ROOT = Path(__file__).resolve().parents[3]
inventory = json.loads((ROOT / 'rust/plans/ws8bm-system-cases.json').read_text())
totals = {'passed': 0, 'deferred': 0, 'blocked_ws12': 0}
for row in inventory['files']:
    path = row['file']
    source = subprocess.check_output(['git', 'show', f'd7c7de92:{path}'], cwd=ROOT, text=True)
    assert hashlib.sha256(source.encode()).hexdigest() == row['source_sha256']
    assert re.findall(r'^\s*test\s+"([^"]+)"', source, re.M) == [case['name'] for case in row['cases']]
    counts = dict.fromkeys(totals, 0)
    for case in row['cases']:
        assert case['status'] in totals
        assert (case['status'] == 'passed') == bool(case.get('evidence')), \
            f"{case['name']}: closure evidence requires passed status, and passes require closure evidence"
        if case['status'] != 'passed':
            reason = case.get('remaining_reason')
            assert isinstance(reason, str) and reason.strip(), \
                f"{case['name']}: unresolved declarations require a nonblank remaining_reason"
        counts[case['status']] += 1
        totals[case['status']] += 1
    print(f"{path}: {len(row['cases'])} named declarations; {counts['passed']} mapped behaviour passes; {counts['deferred']} deferred; {counts['blocked_ws12']} WS12 blocked", flush=True)
assert sum(totals.values()) == 135
print(f"WS8bm system inventory: 135 named declarations; {totals['passed']} mapped behaviour passes; {totals['deferred']} deferred; {totals['blocked_ws12']} WS12 blocked; no pixel checks", flush=True)
