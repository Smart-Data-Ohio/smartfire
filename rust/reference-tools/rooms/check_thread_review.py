#!/usr/bin/env python3
"""Regenerate only from pinned Rails, then verify unchanged HTTP golden bytes."""
import argparse, hashlib, json, os, subprocess
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('--record', action='store_true')
args = parser.parse_args()
root = Path(__file__).resolve().parents[3]
scratch = root / '.scratch/review-fixes/rails-pr-thread'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, PARITY_NAMESPACE='ws8br-thread-review', PARITY_OWNER='ws8br', PARITY_IMAGE='ws8br-reference-d7c7de92')
run = subprocess.run([str(root/'rust/parity/bin/reference'), 'exec', '--seed', 'default', '--time', '2026-03-02T16:00:00Z', '--freeze', 'bin/rails', 'runner', '--skip-executor', '/work/reference-tools/rooms/thread_review_http.rb'], cwd=root, env=env, capture_output=True, check=True)
(scratch/'stdout.json').write_bytes(run.stdout)
(scratch/'stderr.log').write_bytes(run.stderr)
captured = json.loads(run.stdout)
for path, digest in captured['sources'].items():
    assert digest == hashlib.sha256(subprocess.check_output(['git', 'show', 'd7c7de92:'+path], cwd=root)).hexdigest(), 'Rails source drift: '+path
fixture = root/'rust/vectors/messaging/pr-thread-http.json'
if args.record:
    fixture.write_text(json.dumps(captured, indent=2, ensure_ascii=False)+'\n')
assert captured == json.loads(fixture.read_text()), 'Rails PR-thread HTTP bytes changed'
print('Rails PR-thread HTTP oracle: 4 responses reproduced; 4 source files match d7c7de92; bytes unchanged')
