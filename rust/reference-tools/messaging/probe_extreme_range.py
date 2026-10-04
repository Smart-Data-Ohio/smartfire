#!/usr/bin/env python3
"""Expose the unported shared Timestamp boundary using real native output.

This is an owner-dependency diagnostic, not a passing parity gate. --strict
compares every recorded out-of-range result without exclusions and fails until
the shared representation supports Rails' arbitrary-size relative values.
"""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--strict', action='store_true')
p.add_argument('runner', nargs=argparse.REMAINDER)
a = p.parse_args()
runner = a.runner[1:] if a.runner[:1] == ['--'] else a.runner
assert runner
r = subprocess.run(runner + ['test', '--locked', '-p', 'campfire',
    'report_unrepresented_extreme_shared_timestamp_dependency', '--',
    '--test-threads=4', '--nocapture'], cwd=ROOT, text=True,
    stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
print(r.stdout, flush=True)
assert r.returncode == 0, 'native diagnostic did not finish'
actual = [json.loads(line.removeprefix('WS8BM2_EXTREME_ACTUAL '))
          for line in r.stdout.splitlines() if line.startswith('WS8BM2_EXTREME_ACTUAL ')]
assert len(actual) == 12
expected = {case['id']: case for case in json.loads(
    (ROOT / 'rust/vectors/messaging/extreme_range.json').read_text())['cases']}
differences = []
for case in actual:
    for operation in ['parse', 'leading', 'trailing']:
        if case[operation] != expected[case['id']][operation]:
            differences.append(f"{case['id']} {operation}")
for difference in differences:
    print(f'WS8bm2 unported shared Timestamp: {difference}', flush=True)
print(f'WS8bm2 extreme-range strict comparison: {len(differences)}/36 outcomes differ; '
      'owner WS11-UI/shared WS8 Timestamp; no approved difference or masking', flush=True)
if a.strict:
    assert not differences, 'unported shared Timestamp differs from fresh Rails'
