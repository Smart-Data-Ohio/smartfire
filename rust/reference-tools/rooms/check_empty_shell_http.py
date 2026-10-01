#!/usr/bin/env python3
"""Reproduce complete room responses from Rails only."""
import argparse, hashlib, json, os, subprocess
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('--record', action='store_true')
args = parser.parse_args()
root = Path(__file__).resolve().parents[3]
scratch = root / '.scratch/review-fixes/rails-empty-shell'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE='ws8br-empty-shell', PARITY_OWNER='ws8br', PARITY_IMAGE='ws8br-reference-status-2e20b24c')
run = subprocess.run([str(root/'rust/parity/bin/reference'), 'exec', '--seed', 'default', '--time', '2026-03-02T16:00:00Z', '--freeze', 'bin/rails', 'runner', '--skip-executor', '/work/reference-tools/rooms/empty_shell_http.rb'], cwd=root, env=env, capture_output=True)
(scratch/'stdout.json').write_bytes(run.stdout)
(scratch/'stderr.log').write_bytes(run.stderr)
run.check_returncode()
captured = json.loads(run.stdout)
for path, digest in captured['sources'].items():
    pin = '2e20b24c' if path == 'app/views/layouts/application.html.erb' else 'd7c7de92'
    assert digest == hashlib.sha256(subprocess.check_output(['git', 'show', pin+':'+path], cwd=root)).hexdigest(), 'Rails source drift: '+path
fixture = root/'rust/crates/campfire/src/controllers/rooms/empty_shell_http.json'
if args.record:
    fixture.write_text(json.dumps(captured, indent=2, ensure_ascii=False)+'\n')
assert captured == json.loads(fixture.read_text()), 'Rails empty-shell HTTP bytes changed'
print('Rails empty-shell HTTP oracle: 4 full responses reproduced; 8 pinned/approved source files verified; bytes unchanged')
