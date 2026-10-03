#!/usr/bin/env python3
"""Verify attribution labels against the pin; this does not execute the Ruby tests."""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
ledger = (ROOT / 'rust/plans/ws8bm-controller-cases.md').read_text()
total = 0
for section in re.split(r'^## ', ledger, flags=re.M)[1:]:
    path = section.splitlines()[0]
    source = subprocess.check_output(['git', 'show', f'd7c7de92:{path}'], cwd=ROOT, text=True)
    named = re.findall(r'^\s*test\s+"([^"]+)"', source, re.M)
    labels = [line[2:].split(' — ')[0] for line in section.splitlines() if line.startswith('- ')]
    assert set(named) == set(labels), (path, set(named) - set(labels), set(labels) - set(named))
    assert len(named) == len(labels), (path, len(named), len(labels))
    assert 'BLOCKED' not in section, path
    total += len(named)
    print(f'{path}: {len(named)} named declarations; {len(labels)} scoped attributions; 0 blocked', flush=True)
assert total == 156, total
print(f'WS8bm controller inventory: {total} named declarations; {total} scoped attributions; 0 owner-blocked', flush=True)
