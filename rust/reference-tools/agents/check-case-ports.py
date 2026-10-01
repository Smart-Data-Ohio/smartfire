#!/usr/bin/env python3
"""Check named Rust case ports against the pin; actual passes come from cargo logs."""
from pathlib import Path
import json
import re
import subprocess
ROOT = Path(__file__).resolve().parents[3]
mapping = json.loads((Path(__file__).parent / 'case-ports.json').read_text())
counts = {}
for group in mapping['files']:
    source = subprocess.check_output(['git', 'show', mapping['reference_pin'] + ':' + group['rails_file']], cwd=ROOT, text=True)
    assert (ROOT / group['rails_file']).read_text() == source, group['rails_file']
    names = re.findall(r'^\s*test "(.*?)" do', source, re.M)
    mapped = [case['rails'] for case in group['cases']]
    assert len(set(mapped)) == len(mapped), group['rails_file']
    assert mapped == [name for name in names if name in mapped], group['rails_file']
    rust = (ROOT / group['rust_file']).read_text()
    ports = re.findall(r'^\s*(?:async )?fn (ws11_\w+)\(', rust, re.M)
    expected_ports = [case['rust'] for case in group['cases']]
    assert all(port in ports for port in expected_ports), group['rust_file']
    total, prior = counts.get(group['rails_file'], (len(names), set()))
    assert not prior.intersection(mapped), group['rails_file']
    counts[group['rails_file']] = (total, prior.union(mapped))
for file, (total, mapped) in counts.items():
    print(f'WS11 named ports: {file}: {len(mapped)} mapped cases; {total-len(mapped)} unmapped case names')
print(f'WS11 source case files: {len(counts)} pinned Git files matched; 0 checkout mismatches')
