#!/usr/bin/env python3
"""Demand pinned Rails rejects rounded timestamps, without rewriting committed fixtures."""
import argparse
import os
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--scratch', type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
scratch = args.scratch.resolve() if args.scratch else root.parent / '.scratch/ws14g/calendar-completion/boundary-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE='ws14g', PARITY_OWNER='ws14g', PARITY_IMAGE='ws9-reference:d7c7de92')
cases = [
    ('fetched-offset', 'meeting-refresh-cases.rb',
     'Rational(attrs.delete(:fetched_offset_us), 1_000_000)',
     'attrs.delete(:fetched_offset_us) / 1_000_000.0', 'fetched_after_boundary'),
    ('pending-offset', 'meeting-refresh-cases.rb',
     'Rational(attrs.delete(:pending_offset_us), 1_000_000)',
     'attrs.delete(:pending_offset_us) / 1_000_000.0', 'pending_after_boundary'),
    ('renewal-offset', 'push-channel-cases.rb',
     'Rational(spec[:expiry_offset_us], 1_000_000)',
     'spec[:expiry_offset_us] / 1_000_000.0', 'renew_after_boundary'),
]
for name, script, before, after, boundary in cases:
    path = root / 'reference-tools/google' / script
    source = path.read_text()
    assert source.count(before) == 1, name
    try:
        path.write_text(source.replace(before, after))
        result = subprocess.run([str(root / 'parity/bin/reference'), 'runner', '--seed', 'default',
                                 '--time', '2026-09-23T10:30:00Z', '--freeze', str(path)],
                                cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / f'{name}.log').write_text(result.stdout)
        lines = [l for l in result.stdout.splitlines() if boundary + ': persisted' in l]
        assert result.returncode != 0 and lines and 'microseconds' in lines[0], result.stdout
        print(name + ': ' + lines[0], flush=True)
    finally:
        path.write_text(source)
print(f'Calendar boundary fixture discrimination: {len(cases)} rounded-offset mutations rejected', flush=True)
