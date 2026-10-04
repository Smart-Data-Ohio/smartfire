#!/usr/bin/env python3
"""Regenerate only system inventory source hashes from parity/reference.sha."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def refresh(inventory, pin, read_source):
    inventory['reference'] = pin
    for row in inventory['files']:
        source = read_source(row['file'])
        names = re.findall(rb'^\s*test\s+"([^"]+)"', source, re.M)
        assert [name.decode() for name in names] == [case['name'] for case in row['cases']], row['file']
        row['source_sha256'] = hashlib.sha256(source).hexdigest()
    return inventory


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    path = root / 'rust/plans/ws8bm-system-cases.json'
    pin = (root / 'rust/parity/reference.sha').read_text().strip()
    original = json.loads(path.read_text())
    actual = refresh(json.loads(path.read_text()), pin,
                     lambda file: subprocess.check_output(['git', 'show', f'{pin}:{file}'], cwd=root))
    if args.write:
        path.write_text(json.dumps(actual, indent=2) + '\n')
    else:
        assert actual == original, 'run system-inventory-hashes.py --write'
    print(f"WS8bm system source hashes: {len(actual['files'])} pinned files verified at {pin}")
