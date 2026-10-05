#!/usr/bin/env python3
"""Run original pinned Rails declarations selected by an assertion map.

Run parity/bin/ci-seed prepare first; its reference tests/environment are mounted
read-only in the existing current-schema image. Network is disabled, including
for the original Google cases (which use WebMock). No Rust-generated expected
values are passed to Rails.
"""
import argparse
import json
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parents[3]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('mapping', type=pathlib.Path)
p.add_argument('--image', default='ws11ui-cutover-reference:current-schema')
p.add_argument('--storage', type=pathlib.Path, required=True)
a = p.parse_args()
map_path = a.mapping.resolve()
mapping = json.loads(map_path.read_text())
ledger = {r['id']: r for r in json.loads((root / 'rust/plans/ledger-ws14-ws15.json').read_text())['records']}
selected = [ledger[r['id']] for r in mapping['records']]
files = sorted({r['rails'] for r in selected})
reference = root / 'rust/parity/.ci/reference'
for file in files:
    pinned = subprocess.check_output(['git', 'show', mapping['reference'] + ':' + file], cwd=root)
    assert (reference / file).read_bytes() == pinned, ('Rails reference test differs', file)
storage = a.storage.resolve()
storage.mkdir(parents=True, exist_ok=True)
names = [re.escape('test_' + re.sub(r'\s+', '_', r['title'])) for r in selected]
command = ['docker', 'run', '--rm', '--network', 'none', '--env-file', str(root / 'rust/parity/.env.reference'),
           '-e', 'RAILS_ENV=test', '-e', 'PARALLEL_WORKERS=4',
           '-v', str(storage) + ':/rails/storage',
           '-v', str(reference / 'test') + ':/rails/test:ro',
           '-v', str(reference / 'config/environments/test.rb') + ':/rails/config/environments/test.rb:ro',
           a.image, 'bundle', 'exec', 'rails', 'test', *files, '-n', '/^(' + '|'.join(names) + ')$/']
print(f'Pinned Rails cases: {len(selected)} declarations; {len(files)} files; network disabled', flush=True)
raise SystemExit(subprocess.call(command, cwd=root))
